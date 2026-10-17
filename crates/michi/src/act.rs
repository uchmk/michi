//! Branches, tags, reverts, stashes, clean-ups, merges and their conflicts:
//! what the window sends the act thread, and what it says when one ends.
//! They run one after another, each quick; a tag to push goes on to the net
//! thread when it is made.

use std::path::{Path, PathBuf};

use michi_core::autostash::{Back as AutoBack, Stashed as Autostashed};
use michi_core::backup;
use michi_core::branch::Doing;
use michi_core::conflict::{self, Block, Conflict, Pick};
use michi_core::external::{self, Opened};
use michi_core::git;
use michi_core::handoff::{self, Stage};
use michi_core::oid::Oid;
use michi_core::rerere;
use michi_core::revert::{self, Reverted};
use michi_core::stash::{self, Applied, Saved};
use michi_core::tidy::{self, Found, Item, Outcome};
use michi_core::{branch, tag, worktree};

use crate::app::{App, View};
use crate::work::{Band, Job, Then};

/// What the act thread is to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Task {
    /// The branch `name` at `at`; the worktree at `switch` goes onto it,
    /// when given.
    Create {
        name: String,
        at: Oid,
        switch: Option<PathBuf>,
    },
    /// The worktree at `wt` onto the branch `name`.
    Switch {
        wt: PathBuf,
        name: String,
    },
    /// The branch `name` following the remote branch `remote`
    /// (`origin/feat`), with the worktree at `wt` onto it.
    Track {
        wt: PathBuf,
        remote: String,
        name: String,
    },
    /// Deletes the branch `name`.
    Delete {
        name: String,
    },
    /// Renames the branch `old` to `new`.
    Rename {
        old: String,
        new: String,
    },
    /// Makes the branch `name` follow the remote branch of its name.
    Upstream {
        name: String,
    },
    /// Rebases the branch of the worktree at `wt` onto `onto`.
    Rebase {
        wt: PathBuf,
        onto: String,
    },
    /// The tag `name` on `at`, with `message` (none: a light one); pushed
    /// when made, with `push`.
    Tag {
        name: String,
        at: Oid,
        message: String,
        push: bool,
    },
    /// Deletes the tag `name`.
    Untag {
        name: String,
    },
    /// Reverts `id` on the branch of the worktree at `wt`; a merge with
    /// the parent (from 1) whose side stays.
    Revert {
        wt: PathBuf,
        id: Oid,
        mainline: Option<u32>,
    },
    /// Moves the branch of the worktree at `wt` to `id`, the way `mode` says.
    Reset {
        wt: PathBuf,
        id: Oid,
        mode: michi_core::reset::Mode,
    },
    /// Starts a worktree and a Claude in it for each of `plans`, through tsumugi.
    NewWork {
        repo: PathBuf,
        plans: Vec<michi_core::newwork::Plan>,
    },
    /// Closes the Claude tabs `sessions`, keeps the uncommitted change of the
    /// worktree at `wt` and takes the worktree away (its branch stays).
    RemoveWorktree {
        wt: PathBuf,
        sessions: Vec<u64>,
    },
    /// Puts the changes of the worktree at `wt` away, with `message`
    /// (git's own when empty).
    Stash {
        wt: PathBuf,
        message: String,
    },
    /// Puts the stash `id` back on the worktree at `wt`; drops it then,
    /// with `pop`.
    Unstash {
        wt: PathBuf,
        id: Oid,
        pop: bool,
    },
    /// Drops the stash `id`.
    DropStash {
        id: Oid,
    },
    /// Lists what can be cleaned up, the window being on the worktree at
    /// `here`.
    FindTidy {
        here: PathBuf,
    },
    /// Lists HEAD's reflog (as the worktree at `here` has it) and the backups.
    FindReflog {
        here: PathBuf,
    },
    /// Merges `branches` into `trunk` in memory, in their order (the trial merge).
    Trial {
        trunk: String,
        branches: Vec<String>,
    },
    /// Brings the commit `id` back as a new branch.
    Recover {
        id: Oid,
    },
    /// Reads the commits of the worktree at `wt` since `trunk`, for the
    /// rebase screen.
    FindReorder {
        wt: PathBuf,
        trunk: String,
    },
    /// Runs the rebase screen's list on the commits since `base`.
    Reorder {
        wt: PathBuf,
        base: Oid,
        list: Vec<michi_core::rebase::Entry>,
    },
    /// Takes the rows picked from that list away.
    Tidy {
        picks: Vec<Item>,
    },
    /// Merges `rev` into the branch of the worktree at `wt`.
    Merge {
        wt: PathBuf,
        rev: String,
    },
    /// Lands the branch of `req` on the trunk as far as `until`; the Claude
    /// tabs `sessions` close just before the worktree goes.
    Land {
        req: michi_core::land::Request,
        until: michi_core::land::Stage,
        sessions: Vec<u64>,
    },
    /// Picks `id` onto the branch of the worktree at `wt`; a merge against
    /// the parent (from 1) given.
    CherryPick {
        wt: PathBuf,
        id: Oid,
        mainline: Option<u32>,
    },
    /// Solves the place `block` of the file at `path` with `pick`.
    Resolve {
        wt: PathBuf,
        path: String,
        block: Box<Block>,
        pick: Pick,
    },
    ResolveWith {
        wt: PathBuf,
        path: String,
        block: Box<Block>,
        lines: Vec<String>,
    },
    /// Solves the file at `path`, which clashed whole, with our side's
    /// (`ours`) or theirs.
    Take {
        wt: PathBuf,
        path: String,
        ours: bool,
    },
    /// The file at `path` back as git left it.
    Restart {
        wt: PathBuf,
        path: String,
    },
    /// The file at `path`, which rerere solved, started over and rerere's
    /// memory of it dropped.
    Forget {
        wt: PathBuf,
        path: String,
    },
    /// What each side meant by the place `block` of the file at `path`.
    Intent {
        wt: PathBuf,
        path: String,
        j: usize,
        block: Box<Block>,
        conflict: Box<Conflict>,
    },
    /// Commits what the worktree at `wt` stopped in, all solved.
    Conclude {
        wt: PathBuf,
        doing: Doing,
        message: String,
    },
    /// Calls off what the worktree at `wt` is in the middle of.
    Abort {
        wt: PathBuf,
        doing: Doing,
    },
    /// Writes what is left to solve in `conflict` to a file and asks a Claude
    /// to solve it: the tsumugi session `session`, or a new tab when none.
    Handoff {
        conflict: Box<Conflict>,
        session: Option<u64>,
    },
    /// Tells the Claude of `wt` its solution of a clash was not accepted: the
    /// tsumugi session `session`, or a new tab when none. `left` places
    /// still have markers.
    SendBack {
        wt: PathBuf,
        session: Option<u64>,
        left: usize,
    },
    /// Writes the notes on the lines of `branch`'s diff to a file and asks a
    /// Claude to act on them: the tsumugi session `session`, or a new tab.
    HandNotes {
        wt: PathBuf,
        branch: String,
        trunk: String,
        notes: Vec<michi_core::notes::Note>,
        session: Option<u64>,
    },
    /// Counts the places of the conflict at `wt` that still have markers,
    /// once the Claude handed it has stopped.
    Markers {
        wt: PathBuf,
    },
    /// Opens the pull request page of `branch` on GitHub in the browser.
    OpenPr {
        wt: PathBuf,
        branch: String,
    },
    /// Opens the folder of the worktree at `wt` with the command line `command`
    /// (a terminal, an editor or the file manager).
    OpenIn {
        wt: PathBuf,
        tool: michi_core::openin::Tool,
        command: String,
    },
    /// Brings the tsumugi session `session` to the front (`tsumugi focus`).
    Focus {
        session: u64,
    },
    /// Types `text` into the tsumugi session `session` (`tsumugi send`): the
    /// answer to a waiting Claude.
    Reply {
        session: u64,
        text: String,
    },
    /// Opens the file at `path` in an external tool (`git mergetool` when
    /// `merge.tool` is set, else the system's program for the file).
    External {
        wt: PathBuf,
        path: String,
    },
    /// Reads whether rerere is on in the repository of the worktree at `wt`.
    AskRerere {
        wt: PathBuf,
    },
    /// Turns rerere on or off in the repository of the worktree at `wt`.
    Rerere {
        wt: PathBuf,
        on: bool,
    },
    /// Reads why the worktree at `wt` is on no branch.
    ReadDetached {
        wt: PathBuf,
    },
    /// Reads whether the worktree at `wt` is a sparse checkout (#31).
    ReadSparse {
        wt: PathBuf,
    },
    /// The worktree at `wt` back onto the branch `name` it left, keeping
    /// what only its detached HEAD held.
    Back {
        wt: PathBuf,
        name: String,
    },
    /// A Claude of the worktree at `wt` (on `branch`) stopped working: its
    /// folder is kept as a new turn.
    Turn {
        wt: PathBuf,
        branch: String,
    },
    /// Reads the turns kept for `branch`; the one `back` from the newest is
    /// to be shown.
    FindTurns {
        wt: PathBuf,
        branch: String,
        back: usize,
    },
}

/// The word-table key of what a tool is called.
pub fn open_word(tool: michi_core::openin::Tool) -> &'static str {
    use michi_core::openin::Tool;
    match tool {
        Tool::Terminal => "open.terminal",
        Tool::Editor => "open.editor",
        Tool::Explorer => "open.explorer",
    }
}

/// The word-table key of the command that opens a worktree in a tool.
pub fn open_word_cmd(tool: michi_core::openin::Tool) -> &'static str {
    use michi_core::openin::Tool;
    match tool {
        Tool::Terminal => "key.open_terminal",
        Tool::Editor => "key.open_editor",
        Tool::Explorer => "key.open_explorer",
    }
}

impl Task {
    /// git's words for it, the same in every language.
    pub fn word(&self) -> &'static str {
        match self {
            Task::Create { switch: None, .. } => "branch",
            Task::Create { .. } | Task::Switch { .. } | Task::Track { .. } => "switch",
            Task::Delete { .. } => "branch -D",
            Task::Rename { .. } => "branch -m",
            Task::Upstream { .. } => "branch --set-upstream-to",
            Task::Rebase { .. } => "rebase",
            Task::Tag { .. } => "tag",
            Task::Untag { .. } => "tag -d",
            Task::Revert { .. } => "revert",
            Task::Reset { mode, .. } => match mode {
                michi_core::reset::Mode::Soft => "reset --soft",
                michi_core::reset::Mode::Mixed => "reset --mixed",
                michi_core::reset::Mode::Hard => "reset --hard",
            },
            Task::Stash { .. } => "stash",
            Task::Unstash { pop: false, .. } => "stash apply",
            Task::Unstash { pop: true, .. } => "stash pop",
            Task::DropStash { .. } => "stash drop",
            Task::FindTidy { .. } => "for-each-ref",
            Task::FindReflog { .. } => "reflog",
            Task::Trial { .. } => "merge-tree",
            Task::Recover { .. } => "branch",
            Task::FindReorder { .. } => "log",
            Task::Reorder { .. } => "rebase -i",
            Task::Tidy { .. } | Task::RemoveWorktree { .. } => "worktree remove",
            Task::Merge { .. } | Task::Land { .. } => "merge",
            Task::CherryPick { .. } => "cherry-pick",
            Task::Resolve { .. } | Task::ResolveWith { .. } => "add",
            Task::Take { .. } => "checkout",
            Task::Restart { .. } => "checkout --conflict",
            Task::Forget { .. } => "rerere forget",
            Task::Intent { .. } => "log -L",
            Task::Conclude { doing: Doing::Rebase, .. } => "rebase --continue",
            Task::Conclude { .. } => "commit",
            Task::Abort { doing: Doing::Merge, .. } => "merge --abort",
            Task::Abort { doing: Doing::Rebase, .. } => "rebase --abort",
            Task::Abort { doing: Doing::CherryPick, .. } => "cherry-pick --abort",
            Task::Abort { doing: Doing::Revert, .. } => "revert --abort",
            Task::Abort { doing: Doing::Bisect, .. } => "bisect reset",
            Task::Markers { .. } => "conflict",
            Task::Handoff { .. }
            | Task::SendBack { .. }
            | Task::HandNotes { .. }
            | Task::Focus { .. }
            | Task::Reply { .. }
            | Task::NewWork { .. } => "tsumugi",
            Task::OpenPr { .. } => "remote",
            Task::OpenIn { .. } => "open",
            Task::External { .. } => "mergetool",
            Task::AskRerere { .. } => "config --get",
            Task::Rerere { .. } => "config",
            Task::ReadDetached { .. } => "rev-list",
            Task::ReadSparse { .. } => "ls-files",
            Task::Back { .. } => "switch",
            Task::Turn { .. } => "commit-tree",
            Task::FindTurns { .. } => "for-each-ref",
        }
    }
}

/// How a task ended, when git did not refuse it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Acted {
    Done,
    Opened(Opened),
    /// It deleted something, kept under this backup (none: other refs
    /// have it).
    Kept(Option<String>),
    /// The branch now follows this remote branch.
    Upstream(String),
    Reverted(Reverted),
    Reset(michi_core::reset::Reset),
    Stashed(Saved),
    /// Switched; with the changes that were stashed around it, if any.
    Switched(Option<Autostashed>),
    Unstashed(Applied),
    Found(Found),
    Reflog(Vec<michi_core::reflog::Entry>),
    Trialed(Box<michi_core::trial::Trial>),
    Recovered(String),
    Planned(michi_core::rebase::Plan),
    Reordered(michi_core::rebase::Outcome),
    Tidied(Outcome),
    Merged(conflict::Outcome),
    Landed(michi_core::land::Outcome),
    /// The head after the commit.
    Committed(Oid),
    Solved(Box<Solved>),
    Intent(conflict::Intent),
    /// Whether rerere is on (none: nobody said).
    Rerere(Option<bool>),
    /// The places left with markers (none: nothing is in the middle of a clash).
    Markers(Option<usize>),
    /// Why a worktree is on no branch.
    Detached(michi_core::detached::Info),
    /// A worktree's sparse checkout (none: it is off).
    Sparse(Option<michi_core::sparse::Info>),
    /// The turns kept for a branch, the oldest first.
    Turns(Vec<michi_core::turn::Turn>),
}

/// A place solved, or a file put back as git left it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Solved {
    /// Where what the file had is kept (put back).
    pub saved: Option<String>,
    /// The worktree read again just after, for the view not to wait for the
    /// next read of everything: its conflict (none: it is not stopped any
    /// more), or nothing when it could not be read.
    pub read: Option<Option<Conflict>>,
}

/// Does `task` in the repository `dir` is in.
pub fn run(dir: &Path, task: &Task) -> Result<Acted, git::Error> {
    match task {
        Task::Create { name, at, switch } => branch::create(dir, name, at, switch.as_deref()).map(|()| Acted::Done),
        Task::Switch { wt, name } => branch::switch_keeping(wt, name).map(Acted::Switched),
        Task::Track { wt, remote, name } => branch::track(wt, remote, name).map(|()| Acted::Done),
        Task::Delete { name } => branch::delete(dir, name).map(Acted::Kept),
        Task::Rename { old, new } => branch::rename(dir, old, new).map(|()| Acted::Done),
        Task::Upstream { name } => match branch::same_name_remote(dir, name)? {
            Some(remote) => branch::set_upstream(dir, name, &remote).map(|()| Acted::Upstream(remote)),
            None => Err(git::Error::Git(format!("no remote branch is named '{name}'"))),
        },
        Task::Rebase { wt, onto } => branch::rebase(wt, onto).map(|saved| Acted::Kept(Some(saved))),
        Task::Tag { name, at, message, .. } => tag::create(dir, name, at, message).map(|()| Acted::Done),
        Task::Untag { name } => tag::delete(dir, name).map(|saved| Acted::Kept(Some(saved))),
        Task::Revert { wt, id, mainline } => revert::revert(wt, id, *mainline).map(Acted::Reverted),
        Task::Reset { wt, id, mode } => michi_core::reset::reset(wt, id, *mode).map(Acted::Reset),
        Task::Stash { wt, message } => stash::save(wt, message).map(Acted::Stashed),
        Task::Unstash { wt, id, pop } => stash::apply(wt, id, *pop).map(Acted::Unstashed),
        Task::DropStash { id } => stash::drop(dir, id).map(|saved| Acted::Kept(Some(saved))),
        Task::FindTidy { here } => tidy::find(dir, here).map(Acted::Found),
        Task::FindReflog { here } => michi_core::reflog::list(here).map(Acted::Reflog),
        Task::Trial { trunk, branches } => michi_core::trial::run(dir, trunk, branches).map(|t| Acted::Trialed(Box::new(t))),
        Task::Recover { id } => michi_core::reflog::recover(dir, id).map(Acted::Recovered),
        Task::FindReorder { wt, trunk } => michi_core::rebase::plan(wt, trunk).map(Acted::Planned),
        Task::Reorder { wt, base, list } => michi_core::rebase::run(wt, base, list).map(Acted::Reordered),
        Task::RemoveWorktree { wt, sessions } => {
            for id in sessions {
                let _ = handoff::close(*id);
            }
            worktree::remove(dir, wt).map(Acted::Kept)
        }
        Task::Tidy { picks } => Ok(Acted::Tidied(tidy::remove(dir, picks))),
        Task::Merge { wt, rev } => conflict::merge(wt, rev).map(Acted::Merged),
        Task::Land { req, until, sessions } => Ok(Acted::Landed(michi_core::land::run(dir, req, *until, || {
            for id in sessions {
                let _ = handoff::close(*id);
            }
        }))),
        Task::CherryPick { wt, id, mainline } => conflict::cherry_pick(wt, id, *mainline).map(Acted::Merged),
        Task::Resolve { wt, path, block, pick } => conflict::resolve(wt, path, block, *pick).map(|_| solved(wt, None)),
        Task::ResolveWith { wt, path, block, lines } => conflict::resolve_with(wt, path, block, lines).map(|_| solved(wt, None)),
        Task::Take { wt, path, ours } => conflict::take(wt, path, *ours).map(|()| solved(wt, None)),
        Task::Restart { wt, path } => conflict::restart(wt, path).map(|saved| solved(wt, Some(saved))),
        Task::Intent { wt, path, block, conflict, .. } => Ok(Acted::Intent(conflict::intent(wt, path, block, conflict))),
        Task::Forget { wt, path } => conflict::forget(wt, path).map(|saved| solved(wt, Some(saved))),
        Task::Conclude { wt, message, .. } => conflict::commit(wt, message).map(Acted::Committed),
        Task::Abort { wt, .. } => conflict::abort(wt).map(Acted::Kept),
        Task::External { wt, path } => external::open(wt, path).map(Acted::Opened),
        Task::NewWork { repo, plans } => plans.iter().try_for_each(|p| handoff::new_work(repo, p)).map(|()| Acted::Done),
        Task::OpenPr { wt, branch } => michi_core::pr::open(wt, branch).map(|_| Acted::Done),
        Task::OpenIn { wt, command, .. } => michi_core::openin::open(command, wt).map(|()| Acted::Done),
        Task::Focus { session } => handoff::focus(*session).map(|()| Acted::Done),
        Task::Reply { session, text } => handoff::send(*session, text).map(|()| Acted::Done),
        Task::Handoff { conflict, session } => hand_off(conflict, *session).map(|()| Acted::Done),
        Task::SendBack { wt, session, left } => send_back(wt, *session, *left).map(|()| Acted::Done),
        Task::HandNotes { wt, branch, trunk, notes, session } => hand_notes(wt, branch, trunk, notes, *session).map(|()| Acted::Done),
        Task::Markers { wt } => conflict::read(wt).map(|c| Acted::Markers(c.map(|c| c.left()))),
        Task::AskRerere { wt } => rerere::enabled(wt).map(Acted::Rerere),
        Task::Rerere { wt, on } => rerere::set(wt, *on).map(|()| Acted::Done),
        Task::ReadDetached { wt } => michi_core::detached::read(wt).map(Acted::Detached),
        Task::ReadSparse { wt } => michi_core::sparse::read(wt).map(Acted::Sparse),
        Task::Back { wt, name } => michi_core::detached::back(wt, name).map(Acted::Kept),
        Task::Turn { wt, branch } => michi_core::turn::snapshot(wt, branch).map(|_| Acted::Done),
        Task::FindTurns { wt, branch, .. } => michi_core::turn::list(wt, branch).map(Acted::Turns),
    }
}

/// A place in the worktree at `wt` was solved or put back: it is read again.
/// The brief written, then the Claude asked: in session `session`, or in a
/// new tab of the worktree.
fn hand_off(c: &Conflict, session: Option<u64>) -> Result<(), git::Error> {
    let intents: Vec<_> = handoff::open_places(c).into_iter().map(|(path, b)| conflict::intent(&c.wt, path, b, c)).collect();
    let file = handoff::write(c, &intents).map_err(|e| git::Error::Io(e.to_string()))?;
    let text = handoff::request(&file);
    match session {
        Some(id) => handoff::send(id, &text),
        None => handoff::start(&c.wt, &name(&c.wt), &text),
    }
}

/// Tells the Claude in `wt` that what it solved is sent back.
fn send_back(wt: &Path, session: Option<u64>, left: usize) -> Result<(), git::Error> {
    let text = handoff::send_back_request(left);
    match session {
        Some(id) => handoff::send(id, &text),
        None => handoff::start(wt, &name(wt), &text),
    }
}

fn hand_notes(wt: &Path, branch: &str, trunk: &str, notes: &[michi_core::notes::Note], session: Option<u64>) -> Result<(), git::Error> {
    let file = michi_core::notes::write(wt, branch, trunk, notes).map_err(|e| git::Error::Io(e.to_string()))?;
    let text = michi_core::notes::request(&file);
    match session {
        Some(id) => handoff::send(id, &text),
        None => handoff::start(wt, &name(wt), &text),
    }?;
    // The branch is flagged until its Claude commits again; failing to
    // remember that does not undo the hand-over.
    if let Err(e) = flag(wt, branch) {
        eprintln!("michi: {e}");
    }
    Ok(())
}

/// Remembers that notes were handed to `branch`'s Claude at the head it has now.
fn flag(wt: &Path, branch: &str) -> Result<(), git::Error> {
    let head = git::read_text(wt, &["rev-parse", "HEAD"])?;
    let head = michi_core::oid::Oid::parse(head.trim()).ok_or_else(|| git::Error::Io("no HEAD".into()))?;
    let common = michi_core::unseen::common_dir(wt)?;
    let Some(file) = crate::repo::state_path(&common, "flagged") else { return Ok(()) };
    let mut flags = std::fs::read_to_string(&file).map(|t| michi_core::queue::Flags::parse(&t)).unwrap_or_default();
    flags.set(branch, &head);
    ito_common::write_atomic(&file, flags.to_json()).map_err(|e| git::Error::Io(e.to_string()))
}

fn solved(wt: &Path, saved: Option<String>) -> Acted {
    Acted::Solved(Box::new(Solved { saved, read: conflict::read(wt).ok() }))
}

/// The folder's name, to call a worktree by.
pub fn name(wt: &Path) -> String {
    wt.file_name().map_or_else(|| wt.display().to_string(), |n| n.to_string_lossy().into_owned())
}

impl App {
    /// Sends `task` to the act thread.
    pub(crate) fn act(&mut self, task: Task) {
        if let Some(r) = &self.reading {
            r.act(task);
        }
    }

    /// What to add to a toast when changes were stashed around a job, and how
    /// long to show it. A clash stays marked in the files (a stash put back
    /// leaves no merge for the `4` view to read), as with a stash applied by hand.
    pub(crate) fn autostash_said(&self, s: &Autostashed) -> (String, u64) {
        let saved = backup::short(&s.saved);
        match &s.back {
            AutoBack::Done => (self.tr("auto.back").replace("{saved}", saved), 8),
            AutoBack::Clash(files) => (self.tr("auto.clash").replace("{files}", &self.few(files)).replace("{saved}", saved), 14),
            AutoBack::Failed(why) => (self.tr("auto.failed").replace("{why}", why).replace("{saved}", saved), 14),
        }
    }

    /// The task is over: say how it went; a tag to push goes on to the net
    /// thread.
    pub(crate) fn act_done(&mut self, task: Task, result: Result<Acted, git::Error>) {
        let failed = |a: &App, e: &git::Error| a.tr("act.failed").replace("{job}", task.word()).replace("{e}", &e.to_string());
        if let (
            Task::Resolve { wt, .. }
            | Task::ResolveWith { wt, .. }
            | Task::Take { wt, .. }
            | Task::Restart { wt, .. }
            | Task::Forget { wt, .. },
            Ok(Acted::Solved(s)),
        ) = (&task, &result)
        {
            if let Some(read) = &s.read {
                self.conflicts.set_one(wt, read.clone());
            }
        }
        if let (Task::Trial { .. }, Err(_)) = (&task, &result) {
            self.trial_failed();
        }
        let (text, secs) = match (&task, result) {
            // Not being able to ask is not worth a toast.
            // A turn kept in the background is not worth a toast either way.
            (
                Task::AskRerere { .. } | Task::Markers { .. } | Task::ReadDetached { .. } | Task::ReadSparse { .. } | Task::Turn { .. },
                Err(_),
            ) => return,
            (Task::Turn { .. }, Ok(_)) => return,
            (Task::FindTurns { wt, branch, back }, Ok(Acted::Turns(turns))) => {
                self.turns_found(wt, branch, *back, turns);
                return;
            }
            (Task::ReadSparse { wt }, Ok(Acted::Sparse(i))) => {
                self.sparse_read(wt, i);
                return;
            }
            (Task::ReadDetached { wt }, Ok(Acted::Detached(i))) => {
                self.detached_read(wt, i);
                return;
            }
            (Task::Markers { wt }, Ok(Acted::Markers(None))) => {
                self.conflicts.set_stage(wt, Stage::after_count(None));
                return;
            }
            (Task::Markers { wt }, Ok(Acted::Markers(Some(n)))) => {
                self.conflicts.set_stage(wt, Stage::after_count(Some(n)));
                let key = if n == 0 { "claude.markers_gone" } else { "claude.markers_left" };
                (self.tr(key).replace("{wt}", &self::name(wt)).replace("{n}", &n.to_string()), 10)
            }
            (Task::Intent { wt, path, j, .. }, Ok(Acted::Intent(i))) => {
                self.conflicts.set_intent(wt, path, *j, i);
                return;
            }
            (Task::Intent { .. }, Err(_)) => return,
            (Task::AskRerere { wt }, Ok(Acted::Rerere(None))) => {
                let text = self.tr("act.rerere_ask").replace("{wt}", &self::name(wt));
                self.band = Some(Band::new(text, Then::Act(Task::Rerere { wt: wt.clone(), on: true })));
                return;
            }
            (Task::AskRerere { .. }, Ok(_)) => return,
            (Task::External { path, .. }, Ok(Acted::Opened(how))) => {
                let key = if how == Opened::Tool { "act.external_tool" } else { "act.external_file" };
                (self.tr(key).replace("{path}", path), 6)
            }
            (Task::OpenPr { branch, .. }, Ok(_)) => (self.tr("act.pr_opened").replace("{b}", branch), 4),
            (Task::Reply { session, .. }, Ok(_)) => (self.tr("reply.sent").replace("{n}", &session.to_string()), 4),
            (Task::OpenIn { wt, tool, .. }, Ok(_)) => {
                (self.tr("act.open_in").replace("{wt}", &self::name(wt)).replace("{tool}", self.tr(open_word(*tool))), 3)
            }
            (Task::Focus { session }, Ok(_)) => (self.tr("act.focused").replace("{n}", &session.to_string()), 3),
            (Task::Handoff { conflict, session }, Ok(_)) => {
                let key = if session.is_some() { "act.handed" } else { "act.handed_new" };
                self.watch_handed(conflict.wt.clone());
                if let Some(h) = self.handed.last_mut() {
                    h.conflict = true;
                }
                self.conflicts.set_stage(&conflict.wt, Some(Stage::Asked));
                (self.tr(key).replace("{wt}", &self::name(&conflict.wt)), 8)
            }
            (Task::SendBack { wt, session, .. }, Ok(_)) => {
                let key = if session.is_some() { "act.sent_back" } else { "act.sent_back_new" };
                self.watch_handed(wt.clone());
                if let Some(h) = self.handed.last_mut() {
                    h.conflict = true;
                }
                self.conflicts.set_stage(wt, Some(Stage::Asked));
                (self.tr(key).replace("{wt}", &self::name(wt)), 8)
            }
            (Task::HandNotes { wt, notes, session, .. }, Ok(_)) => {
                let key = if session.is_some() { "act.noted" } else { "act.noted_new" };
                self.notes_sent(wt);
                self.watch_handed(wt.clone());
                (self.tr(key).replace("{wt}", &self::name(wt)).replace("{n}", &notes.len().to_string()), 8)
            }
            (Task::Rerere { on, .. }, Ok(_)) => (self.tr(if *on { "act.rerere_on" } else { "act.rerere_off" }).to_owned(), 6),
            (Task::Resolve { wt, path, .. } | Task::ResolveWith { wt, path, .. } | Task::Take { wt, path, .. }, Err(e)) => {
                self.conflict_unpick(wt, path);
                (failed(self, &e), 10)
            }
            (_, Err(e)) => {
                let text = failed(self, &e);
                self.last_error = Some(text.clone());
                (text, 10)
            }
            (Task::Create { name, at, switch: None }, Ok(_)) => {
                (self.tr("act.created").replace("{name}", name).replace("{at}", &at.short()), 4)
            }
            (Task::Create { name, switch: Some(wt), .. }, Ok(_)) => {
                (self.tr("act.created_switched").replace("{name}", name).replace("{wt}", &self::name(wt)), 4)
            }
            (Task::Switch { wt, name }, Ok(Acted::Switched(Some(s)))) => {
                let (text, secs) = self.autostash_said(&s);
                (self.tr("act.switched").replace("{name}", name).replace("{wt}", &self::name(wt)) + &text, secs)
            }
            (Task::Switch { wt, name }, Ok(_)) => (self.tr("act.switched").replace("{name}", name).replace("{wt}", &self::name(wt)), 4),
            (Task::Back { wt, name }, Ok(Acted::Kept(Some(saved)))) => (
                self.tr("act.back_kept").replace("{name}", name).replace("{wt}", &self::name(wt)).replace("{saved}", backup::short(&saved)),
                10,
            ),
            (Task::Back { wt, name }, Ok(_)) => (self.tr("act.back").replace("{name}", name).replace("{wt}", &self::name(wt)), 4),
            (Task::Track { wt, remote, name }, Ok(_)) => {
                (self.tr("act.tracked").replace("{name}", name).replace("{remote}", remote).replace("{wt}", &self::name(wt)), 4)
            }
            (Task::Delete { name }, Ok(Acted::Kept(Some(saved)))) => {
                (self.tr("act.deleted_kept").replace("{name}", name).replace("{saved}", backup::short(&saved)), 10)
            }
            (Task::RemoveWorktree { wt, .. }, Ok(Acted::Kept(saved))) => {
                let key = if saved.is_some() { "act.removed_kept" } else { "act.removed" };
                let saved = saved.as_deref().map_or("", backup::short);
                (self.tr(key).replace("{wt}", &self::name(wt)).replace("{saved}", saved), 10)
            }
            (Task::NewWork { plans, .. }, Ok(_)) => (self.tr("act.started").replace("{n}", &plans.len().to_string()), 5),
            (Task::Delete { name }, Ok(_)) => (self.tr("act.deleted").replace("{}", name), 4),
            (Task::Rename { old, new }, Ok(_)) => (self.tr("act.renamed").replace("{old}", old).replace("{new}", new), 4),
            (Task::Upstream { name }, Ok(Acted::Upstream(remote))) => {
                (self.tr("act.upstream").replace("{name}", name).replace("{remote}", &remote), 4)
            }
            (Task::Rebase { wt, onto }, Ok(Acted::Kept(saved))) => {
                let saved = saved.as_deref().map_or("", backup::short);
                (self.tr("act.rebased").replace("{wt}", &self::name(wt)).replace("{onto}", onto).replace("{saved}", saved), 10)
            }
            (Task::Tag { name, at, push: false, .. }, Ok(_)) => {
                (self.tr("act.tagged").replace("{name}", name).replace("{at}", &at.short()), 4)
            }
            (Task::Tag { name, at, push: true, .. }, Ok(_)) => {
                if let Some(n) = &self.net {
                    let text = self.tr("act.tagged_unpushed").replace("{name}", name).replace("{job}", n.kind.word());
                    (text, 8)
                } else {
                    self.order(Job::PushTag { tag: name.clone() });
                    (self.tr("act.tagged").replace("{name}", name).replace("{at}", &at.short()), 4)
                }
            }
            (Task::Untag { name }, Ok(Acted::Kept(saved))) => {
                let saved = saved.as_deref().map_or("", backup::short);
                (self.tr("act.untagged").replace("{name}", name).replace("{saved}", saved), 10)
            }
            (Task::Reset { wt, id, mode }, Ok(Acted::Reset(r))) => {
                let key = match (r.changes.is_some(), r.tip.is_some()) {
                    (true, _) => "act.reset_changes",
                    (false, true) => "act.reset_kept",
                    (false, false) => "act.reset_same",
                };
                let tip = r.tip.as_deref().map_or("", backup::short);
                let changes = r.changes.as_deref().map_or("", backup::short);
                let text = self
                    .tr(key)
                    .replace("{wt}", &self::name(wt))
                    .replace("{id}", &id.short())
                    .replace("{mode}", mode.word())
                    .replace("{tip}", tip)
                    .replace("{changes}", changes);
                (text, 10)
            }
            (Task::Revert { wt, id, .. }, Ok(Acted::Reverted(r))) => match r {
                Reverted::Made(new) => (
                    self.tr("act.reverted").replace("{id}", &id.short()).replace("{wt}", &self::name(wt)).replace("{new}", &new.short()),
                    6,
                ),
                Reverted::Empty => (self.tr("act.revert_empty").replace("{}", &id.short()), 6),
                Reverted::Conflict(files) => {
                    self.show_conflicts(wt);
                    (self.tr("act.revert_clash").replace("{id}", &id.short()).replace("{files}", &self.few(&files)), 8)
                }
            },
            (Task::Stash { wt, .. }, Ok(Acted::Stashed(Saved::Made(id)))) => {
                (self.tr("act.stashed").replace("{wt}", &self::name(wt)).replace("{id}", &id.short()), 4)
            }
            (Task::Stash { wt, .. }, Ok(_)) => (self.tr("act.stash_nothing").replace("{}", &self::name(wt)), 4),
            (Task::Unstash { wt, pop, .. }, Ok(Acted::Unstashed(Applied::Done))) => {
                let key = if *pop { "act.popped" } else { "act.applied" };
                (self.tr(key).replace("{}", &self::name(wt)), 4)
            }
            (Task::Unstash { wt, .. }, Ok(Acted::Unstashed(Applied::Conflict(files)))) => {
                (self.tr("act.unstash_clash").replace("{wt}", &self::name(wt)).replace("{files}", &self.few(&files)), 14)
            }
            (Task::DropStash { id }, Ok(Acted::Kept(saved))) => {
                let saved = saved.as_deref().map_or("", backup::short);
                (self.tr("act.dropped").replace("{id}", &id.short()).replace("{saved}", saved), 10)
            }
            (Task::FindTidy { .. }, Ok(Acted::Found(found))) => return self.tidy_found(found),
            (Task::FindReflog { .. }, Ok(Acted::Reflog(rows))) => return self.reflog_found(rows),
            (Task::Trial { branches, .. }, Ok(Acted::Trialed(found))) => return self.trial_done(branches, *found),
            (Task::Recover { id }, Ok(Acted::Recovered(name))) => {
                (self.tr("reflog.done").replace("{id}", &id.short()).replace("{name}", &name), 6)
            }
            (Task::FindReorder { .. }, Ok(Acted::Planned(plan))) => return self.reorder_found(plan),
            (Task::Reorder { wt, .. }, Ok(Acted::Reordered(out))) => {
                self.reorder = None;
                let tr =
                    |key: &'static str, saved: &str| self.tr(key).replace("{wt}", &self::name(wt)).replace("{saved}", backup::short(saved));
                match out {
                    michi_core::rebase::Outcome::Done { backup } => (tr("reorder.done", &backup), 8),
                    michi_core::rebase::Outcome::Stopped { backup } => {
                        let text = tr("reorder.clash", &backup);
                        self.show_conflicts(wt);
                        (text, 8)
                    }
                }
            }
            (Task::Tidy { .. }, Ok(Acted::Tidied(out))) => self.tidied(&out),
            (Task::Merge { wt, rev }, Ok(Acted::Merged(out))) => {
                let tr = |key: &'static str| self.tr(key).replace("{rev}", rev).replace("{wt}", &self::name(wt));
                match out {
                    conflict::Outcome::Made(new) => (tr("act.merged").replace("{new}", &new.short()), 6),
                    conflict::Outcome::UpToDate | conflict::Outcome::Empty => (tr("act.merge_none"), 6),
                    conflict::Outcome::Conflict(files) => {
                        let text = tr("act.merge_clash").replace("{files}", &self.few(&files));
                        self.show_conflicts(wt);
                        (text, 8)
                    }
                }
            }
            (Task::Land { req, until, .. }, Ok(Acted::Landed(out))) => self.landed(req, *until, &out),
            (Task::CherryPick { wt, id, .. }, Ok(Acted::Merged(out))) => {
                let tr = |key: &'static str| self.tr(key).replace("{id}", &id.short()).replace("{wt}", &self::name(wt));
                match out {
                    conflict::Outcome::Made(new) => (tr("act.picked").replace("{new}", &new.short()), 6),
                    conflict::Outcome::UpToDate | conflict::Outcome::Empty => (tr("act.pick_empty"), 6),
                    conflict::Outcome::Conflict(files) => {
                        let text = tr("act.pick_clash").replace("{files}", &self.few(&files));
                        self.show_conflicts(wt);
                        (text, 8)
                    }
                }
            }
            (Task::Restart { path, .. }, Ok(Acted::Solved(s))) => {
                let saved = s.saved.as_deref().map_or("", backup::short);
                (self.tr("act.restarted").replace("{path}", path).replace("{saved}", saved), 8)
            }
            (Task::Forget { path, .. }, Ok(Acted::Solved(s))) => {
                let saved = s.saved.as_deref().map_or("", backup::short);
                (self.tr("act.forgotten").replace("{path}", path).replace("{saved}", saved), 8)
            }
            (Task::Conclude { wt, doing, .. }, Ok(Acted::Committed(new))) => {
                let text = self.tr("act.concluded").replace("{doing}", &self.doing_word(*doing)).replace("{wt}", &self::name(wt));
                (text.replace("{new}", &new.short()), 6)
            }
            (Task::Abort { wt, doing }, Ok(Acted::Kept(saved))) => {
                let text = self.tr("act.aborted").replace("{doing}", &self.doing_word(*doing)).replace("{wt}", &self::name(wt));
                let kept = saved.as_deref().map(|s| self.tr("act.aborted_kept").replace("{}", backup::short(s))).unwrap_or_default();
                (text + &kept, 10)
            }
            // Each task ends one way when it goes.
            (_, Ok(_)) => return,
        };
        self.say_for(text, secs);
    }

    /// A merge, a pick or a revert stopped on a clash in the worktree at
    /// `wt`: the `4` view shows it, once read.
    pub(crate) fn show_conflicts(&mut self, wt: &Path) {
        self.conflicts.wt = Some(wt.to_path_buf());
        self.conflicts.show(wt);
        self.view = View::Conflict;
        // Once a session, ask whether to reuse what is solved here (rerere).
        if !std::mem::replace(&mut self.rerere_asked, true) {
            self.act(Task::AskRerere { wt: wt.to_path_buf() });
        }
    }

    /// Up to three files, then how many more.
    pub(crate) fn few(&self, files: &[String]) -> String {
        let mut list = files.iter().take(3).map(String::as_str).collect::<Vec<_>>().join(", ");
        if files.len() > 3 {
            list.push_str(&self.tr("act.more").replace("{}", &(files.len() - 3).to_string()));
        }
        list
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::{app, toast};
    use crate::work::{Kind, Running};

    fn oid(c: char) -> Oid {
        Oid::parse(&c.to_string().repeat(40)).unwrap()
    }

    #[test]
    fn each_ending_is_said() {
        let mut a = app();
        let wt = PathBuf::from("/r/feat");
        let mut said = |task: Task, r: Result<Acted, git::Error>| {
            a.act_done(task, r);
            toast(&a).to_owned()
        };
        assert_eq!(said(Task::Create { name: "x".into(), at: oid('a'), switch: None }, Ok(Acted::Done)), "Made branch x at aaaaaaa");
        assert_eq!(
            said(Task::Create { name: "x".into(), at: oid('a'), switch: Some(wt.clone()) }, Ok(Acted::Done)),
            "Made branch x; feat is on it"
        );
        assert_eq!(said(Task::Switch { wt: wt.clone(), name: "main".into() }, Ok(Acted::Done)), "feat is on main now");
        assert_eq!(
            said(Task::Track { wt: wt.clone(), remote: "origin/y".into(), name: "y".into() }, Ok(Acted::Done)),
            "Made y following origin/y; feat is on it"
        );
        assert_eq!(said(Task::Delete { name: "x".into() }, Ok(Acted::Kept(None))), "Deleted branch x");
        assert_eq!(
            said(Task::Delete { name: "x".into() }, Ok(Acted::Kept(Some("refs/michi/backup/20261016-120000".into())))),
            "Deleted branch x; its commits are kept in michi/backup/20261016-120000"
        );
        assert_eq!(
            said(Task::Tag { name: "v1".into(), at: oid('b'), message: String::new(), push: false }, Ok(Acted::Done)),
            "Made tag v1 at bbbbbbb"
        );
        assert_eq!(
            said(Task::Untag { name: "v1".into() }, Ok(Acted::Kept(Some("refs/michi/backup/20261016-120000".into())))),
            "Deleted tag v1; kept in michi/backup/20261016-120000"
        );
        let revert = Task::Revert { wt: wt.clone(), id: oid('c'), mainline: None };
        assert_eq!(said(revert.clone(), Ok(Acted::Reverted(Reverted::Made(oid('d'))))), "Reverted ccccccc on feat: ddddddd");
        assert_eq!(said(revert.clone(), Ok(Acted::Reverted(Reverted::Empty))), "Nothing to revert: ccccccc is undone already");
        let files = ["a", "b", "c", "d", "e"].map(String::from).to_vec();
        assert_eq!(
            said(revert.clone(), Ok(Acted::Reverted(Reverted::Conflict(files.clone())))),
            "Reverting ccccccc clashed in a, b, c and 2 more: solve it in the 4 view"
        );
        let merge = Task::Merge { wt: wt.clone(), rev: "main".into() };
        assert_eq!(said(merge.clone(), Ok(Acted::Merged(conflict::Outcome::Made(oid('d'))))), "Merged main into feat: ddddddd");
        assert_eq!(said(merge.clone(), Ok(Acted::Merged(conflict::Outcome::UpToDate))), "feat has all of main already");
        assert_eq!(
            said(merge, Ok(Acted::Merged(conflict::Outcome::Conflict(files.clone())))),
            "Merging main into feat clashed in a, b, c and 2 more: solve it in the 4 view"
        );
        let pick = Task::CherryPick { wt: wt.clone(), id: oid('c'), mainline: None };
        assert_eq!(said(pick.clone(), Ok(Acted::Merged(conflict::Outcome::Made(oid('d'))))), "Picked ccccccc onto feat: ddddddd");
        assert_eq!(said(pick.clone(), Ok(Acted::Merged(conflict::Outcome::Empty))), "Nothing to pick: feat has the changes of ccccccc");
        assert_eq!(
            said(pick, Ok(Acted::Merged(conflict::Outcome::Conflict(vec!["a".into()])))),
            "Picking ccccccc onto feat clashed in a: solve it in the 4 view"
        );
        assert_eq!(
            said(
                Task::Restart { wt: wt.clone(), path: "a".into() },
                Ok(Acted::Solved(Box::new(Solved { saved: Some("refs/michi/backup/20261016-120000".into()), read: None })))
            ),
            "a is back as git left it; what it had is kept in michi/backup/20261016-120000"
        );
        assert_eq!(
            said(Task::Conclude { wt: wt.clone(), doing: Doing::Merge, message: "m".into() }, Ok(Acted::Committed(oid('f')))),
            "Committed the merge on feat: fffffff"
        );
        assert_eq!(
            said(Task::Abort { wt: wt.clone(), doing: Doing::Merge }, Ok(Acted::Kept(Some("refs/michi/backup/20261016-120000".into())))),
            "Called the merge on feat off; its files are kept in michi/backup/20261016-120000"
        );
        assert_eq!(
            said(Task::Abort { wt: wt.clone(), doing: Doing::CherryPick }, Ok(Acted::Kept(None))),
            "Called the cherry-pick on feat off"
        );
        assert_eq!(
            said(
                Task::Switch { wt: wt.clone(), name: "main".into() },
                Ok(Acted::Switched(Some(Autostashed { saved: "refs/michi/backup/20261016-120000".into(), back: AutoBack::Done })))
            ),
            "feat is on main now; the changes in the way were stashed and put back (kept in michi/backup/20261016-120000)"
        );
        assert!(said(
            Task::Switch { wt: wt.clone(), name: "main".into() },
            Ok(Acted::Switched(Some(Autostashed { saved: "refs/michi/backup/x".into(), back: AutoBack::Clash(vec!["a.txt".into()]) })))
        )
        .contains("clashed in a.txt"));
        assert_eq!(
            said(Task::Stash { wt: wt.clone(), message: String::new() }, Ok(Acted::Stashed(Saved::Made(oid('e'))))),
            "Put the changes of feat away: eeeeeee"
        );
        assert_eq!(
            said(Task::Stash { wt: wt.clone(), message: String::new() }, Ok(Acted::Stashed(Saved::Nothing))),
            "feat has no changes to put away"
        );
        assert_eq!(
            said(Task::Unstash { wt: wt.clone(), id: oid('e'), pop: false }, Ok(Acted::Unstashed(Applied::Done))),
            "Put the stash back on feat; it stays in the list"
        );
        assert_eq!(
            said(Task::Unstash { wt: wt.clone(), id: oid('e'), pop: true }, Ok(Acted::Unstashed(Applied::Done))),
            "Put the stash back on feat and dropped it"
        );
        assert_eq!(
            said(Task::Unstash { wt: wt.clone(), id: oid('e'), pop: true }, Ok(Acted::Unstashed(Applied::Conflict(vec!["a".into()])))),
            "The stash clashed on feat in a: resolve the marks there and git add them; the stash is kept"
        );
        assert_eq!(
            said(Task::DropStash { id: oid('e') }, Ok(Acted::Kept(Some("refs/michi/backup/20261016-120000".into())))),
            "Dropped stash eeeeeee; kept in michi/backup/20261016-120000"
        );
        assert_eq!(
            said(Task::Switch { wt, name: "main".into() }, Err(git::Error::Git("your local changes would be overwritten".into()))),
            "switch failed: your local changes would be overwritten"
        );
    }

    #[test]
    fn a_tag_to_push_waits_for_no_job() {
        let mut a = app();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        a.net = Some(Running::new(Kind::Fetch, stop));
        a.act_done(Task::Tag { name: "v1".into(), at: oid('a'), message: String::new(), push: true }, Ok(Acted::Done));
        assert_eq!(toast(&a), "Made tag v1, not pushed: fetch is running");
    }

    #[test]
    fn rerere_is_asked_once_and_n_writes_it_down() {
        let mut a = app();
        let wt = PathBuf::from("w");
        a.act_done(Task::AskRerere { wt: wt.clone() }, Ok(Acted::Rerere(Some(true))));
        assert!(a.band.is_none());
        a.act_done(Task::AskRerere { wt: wt.clone() }, Err(git::Error::Git("no".into())));
        assert!(a.band.is_none());
        a.act_done(Task::AskRerere { wt: wt.clone() }, Ok(Acted::Rerere(None)));
        assert_eq!(a.band.as_ref().map(|b| b.then.clone()), Some(Then::Act(Task::Rerere { wt: wt.clone(), on: true })));
        a.band_key('n');
        assert!(a.band.is_none());
        a.act_done(Task::Rerere { wt: wt.clone(), on: false }, Ok(Acted::Done));
        assert_eq!(toast(&a), "rerere stays off (git config rerere.enabled false)");
        a.act_done(Task::Rerere { wt, on: true }, Ok(Acted::Done));
        assert!(toast(&a).starts_with("rerere is on"));
    }

    #[test]
    fn a_handoff_says_whether_it_asked_a_running_claude_or_started_one() {
        let mut a = app();
        let c = Box::new(crate::conflicts::tests::conflict("/r/main"));
        a.act_done(Task::Handoff { conflict: c.clone(), session: Some(3) }, Ok(Acted::Done));
        assert!(toast(&a).starts_with("Asked the Claude in main"), "{}", toast(&a));
        a.act_done(Task::Handoff { conflict: c.clone(), session: None }, Ok(Acted::Done));
        assert!(toast(&a).starts_with("Started a Claude in a new tab"), "{}", toast(&a));
        a.act_done(Task::Handoff { conflict: c, session: None }, Err(git::Error::Git("tsumugi: gone".into())));
        assert_eq!(toast(&a), "tsumugi failed: tsumugi: gone");
    }

    #[test]
    fn opening_a_folder_says_where_and_what_failed() {
        use michi_core::openin::Tool;
        let mut a = app();
        let t = || Task::OpenIn { wt: PathBuf::from("/r/main"), tool: Tool::Editor, command: "code {path}".into() };
        a.act_done(t(), Ok(Acted::Done));
        assert_eq!(toast(&a), "Opened main in the Editor");
        a.act_done(t(), Err(git::Error::Io("code: gone".into())));
        assert!(toast(&a).starts_with("open failed"), "{}", toast(&a));
    }

    #[test]
    fn an_external_open_says_which_way_it_went() {
        let mut a = app();
        let t = || Task::External { wt: PathBuf::from("/r/main"), path: "y.bin".into() };
        a.act_done(t(), Ok(Acted::Opened(Opened::Tool)));
        assert_eq!(toast(&a), "Closed the merge tool on y.bin");
        a.act_done(t(), Ok(Acted::Opened(Opened::File)));
        assert!(toast(&a).starts_with("Opened y.bin with"), "{}", toast(&a));
        a.act_done(t(), Err(git::Error::Git("no tool".into())));
        assert_eq!(toast(&a), "mergetool failed: no tool");
    }

    #[test]
    fn the_markers_left_after_a_handoff_are_counted_in_a_toast() {
        let mut a = app();
        let t = || Task::Markers { wt: PathBuf::from("/r/main") };
        a.act_done(t(), Ok(Acted::Markers(Some(2))));
        assert!(toast(&a).contains("2 places"), "{}", toast(&a));
        a.act_done(t(), Ok(Acted::Markers(Some(0))));
        assert!(toast(&a).contains("no conflict markers"), "{}", toast(&a));
        a.toast = None;
        a.act_done(t(), Ok(Acted::Markers(None)));
        assert!(a.toast.is_none());
    }
}
