//! Branches, tags, reverts and stashes: what the window sends the act
//! thread, and what it says when one ends. They run one after another, each
//! quick; a tag to push goes on to the net thread when it is made.

use std::path::{Path, PathBuf};

use michi_core::backup;
use michi_core::git;
use michi_core::oid::Oid;
use michi_core::revert::{self, Reverted};
use michi_core::stash::{self, Applied, Saved};
use michi_core::{branch, tag};

use crate::app::App;
use crate::work::Job;

/// What the act thread is to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Task {
    /// The branch `name` at `at`; the worktree at `switch` goes onto it,
    /// when given.
    Create { name: String, at: Oid, switch: Option<PathBuf> },
    /// The worktree at `wt` onto the branch `name`.
    Switch { wt: PathBuf, name: String },
    /// The branch `name` following the remote branch `remote`
    /// (`origin/feat`), with the worktree at `wt` onto it.
    Track { wt: PathBuf, remote: String, name: String },
    /// Deletes the branch `name`.
    Delete { name: String },
    /// The tag `name` on `at`, with `message` (none: a light one); pushed
    /// when made, with `push`.
    Tag { name: String, at: Oid, message: String, push: bool },
    /// Deletes the tag `name`.
    Untag { name: String },
    /// Reverts `id` on the branch of the worktree at `wt`; a merge with
    /// the parent (from 1) whose side stays.
    Revert { wt: PathBuf, id: Oid, mainline: Option<u32> },
    /// Puts the changes of the worktree at `wt` away, with `message`
    /// (git's own when empty).
    Stash { wt: PathBuf, message: String },
    /// Puts the stash `id` back on the worktree at `wt`; drops it then,
    /// with `pop`.
    Unstash { wt: PathBuf, id: Oid, pop: bool },
    /// Drops the stash `id`.
    DropStash { id: Oid },
}

impl Task {
    /// git's words for it, the same in every language.
    pub fn word(&self) -> &'static str {
        match self {
            Task::Create { switch: None, .. } => "branch",
            Task::Create { .. } | Task::Switch { .. } | Task::Track { .. } => "switch",
            Task::Delete { .. } => "branch -D",
            Task::Tag { .. } => "tag",
            Task::Untag { .. } => "tag -d",
            Task::Revert { .. } => "revert",
            Task::Stash { .. } => "stash",
            Task::Unstash { pop: false, .. } => "stash apply",
            Task::Unstash { pop: true, .. } => "stash pop",
            Task::DropStash { .. } => "stash drop",
        }
    }
}

/// How a task ended, when git did not refuse it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Acted {
    Done,
    /// It deleted something, kept under this backup (none: other refs
    /// have it).
    Kept(Option<String>),
    Reverted(Reverted),
    Stashed(Saved),
    Unstashed(Applied),
}

/// Does `task` in the repository `dir` is in.
pub fn run(dir: &Path, task: &Task) -> Result<Acted, git::Error> {
    match task {
        Task::Create { name, at, switch } => branch::create(dir, name, at, switch.as_deref()).map(|()| Acted::Done),
        Task::Switch { wt, name } => branch::switch(wt, name).map(|()| Acted::Done),
        Task::Track { wt, remote, name } => branch::track(wt, remote, name).map(|()| Acted::Done),
        Task::Delete { name } => branch::delete(dir, name).map(Acted::Kept),
        Task::Tag { name, at, message, .. } => tag::create(dir, name, at, message).map(|()| Acted::Done),
        Task::Untag { name } => tag::delete(dir, name).map(|saved| Acted::Kept(Some(saved))),
        Task::Revert { wt, id, mainline } => revert::revert(wt, id, *mainline).map(Acted::Reverted),
        Task::Stash { wt, message } => stash::save(wt, message).map(Acted::Stashed),
        Task::Unstash { wt, id, pop } => stash::apply(wt, id, *pop).map(Acted::Unstashed),
        Task::DropStash { id } => stash::drop(dir, id).map(|saved| Acted::Kept(Some(saved))),
    }
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

    /// The task is over: say how it went; a tag to push goes on to the net
    /// thread.
    pub(crate) fn act_done(&mut self, task: Task, result: Result<Acted, git::Error>) {
        let failed = |a: &App, e: &git::Error| a.tr("act.failed").replace("{job}", task.word()).replace("{e}", &e.to_string());
        let (text, secs) = match (&task, result) {
            (_, Err(e)) => (failed(self, &e), 10),
            (Task::Create { name, at, switch: None }, Ok(_)) => {
                (self.tr("act.created").replace("{name}", name).replace("{at}", &at.short()), 4)
            }
            (Task::Create { name, switch: Some(wt), .. }, Ok(_)) => {
                (self.tr("act.created_switched").replace("{name}", name).replace("{wt}", &self::name(wt)), 4)
            }
            (Task::Switch { wt, name }, Ok(_)) => (self.tr("act.switched").replace("{name}", name).replace("{wt}", &self::name(wt)), 4),
            (Task::Track { wt, remote, name }, Ok(_)) => {
                (self.tr("act.tracked").replace("{name}", name).replace("{remote}", remote).replace("{wt}", &self::name(wt)), 4)
            }
            (Task::Delete { name }, Ok(Acted::Kept(Some(saved)))) => {
                (self.tr("act.deleted_kept").replace("{name}", name).replace("{saved}", backup::short(&saved)), 10)
            }
            (Task::Delete { name }, Ok(_)) => (self.tr("act.deleted").replace("{}", name), 4),
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
            (Task::Revert { wt, id, .. }, Ok(Acted::Reverted(r))) => match r {
                Reverted::Made(new) => (
                    self.tr("act.reverted").replace("{id}", &id.short()).replace("{wt}", &self::name(wt)).replace("{new}", &new.short()),
                    6,
                ),
                Reverted::Empty => (self.tr("act.revert_empty").replace("{}", &id.short()), 6),
                Reverted::Conflict(files) => {
                    (self.tr("act.revert_clash").replace("{id}", &id.short()).replace("{files}", &self.few(&files)), 14)
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
            // Each task ends one way when it goes.
            (_, Ok(_)) => return,
        };
        self.say_for(text, secs);
    }

    /// Up to three files, then how many more.
    fn few(&self, files: &[String]) -> String {
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
            said(revert.clone(), Ok(Acted::Reverted(Reverted::Conflict(files)))),
            "Reverting ccccccc clashed in a, b, c and 2 more, so it was called off: \
             git revert ccccccc in a terminal lets you resolve it"
        );
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
}
