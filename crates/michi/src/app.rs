//! The app's state and what the keys do to it. Nothing here draws or runs
//! git, so the keys can be tested without a window or a repository.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use michi_core::clash::{Clashes, Outcome, Side};
use michi_core::claude::{Session, State};
use michi_core::git::{self, Version};
use michi_core::oid::Oid;
use michi_core::paths;
use michi_core::summary::{self, Summary};
use michi_core::track::{Tree, Trees};
use michi_core::unseen::Unseen;
use michi_core::wip::Wip;

use crate::changes::{Changes, Did, Focus, Wish as StageWish};
use crate::conflicts::Conflicts;
use crate::detail::Pane;
use crate::graph::{Folded, Graph};
use crate::picker::{Item as PickItem, Kind as PickKind, Picker};
use crate::prefs::Prefs;
use crate::repo::{Answer, Ask, Reading};
use crate::review::{Ends, Review, Wish};
use crate::work::{Band, Running};

/// The four views, opened by `1` to `4` (docs/design.md); the `4` view
/// only while something clashed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Graph,
    Changes,
    Worktree,
    Conflict,
}

impl View {
    pub const ALL: [View; 4] = [View::Graph, View::Changes, View::Worktree, View::Conflict];

    /// The digit that opens it.
    pub fn digit(self) -> char {
        match self {
            View::Graph => '1',
            View::Changes => '2',
            View::Worktree => '3',
            View::Conflict => '4',
        }
    }

    pub fn from_digit(c: char) -> Option<View> {
        View::ALL.into_iter().find(|v| v.digit() == c)
    }

    /// The word-table key of its name.
    pub fn label(self) -> &'static str {
        match self {
            View::Graph => "view.graph",
            View::Changes => "view.changes",
            View::Worktree => "view.worktree",
            View::Conflict => "view.conflict",
        }
    }

    /// The word-table key of the line saying what will be here.
    pub fn empty(self) -> &'static str {
        match self {
            View::Graph => "empty.graph",
            View::Changes => "empty.changes",
            View::Worktree => "empty.worktree",
            View::Conflict => "empty.conflict",
        }
    }

    /// The keys that work in this view: `(keys, word-table key)`, the one to
    /// recommend first. The keybar shows the first few; `?` shows them all.
    pub fn keys(self) -> &'static [(&'static str, &'static str)] {
        match self {
            View::Graph => &[
                ("j k", "key.move"),
                ("Enter", "key.side"),
                ("/", "key.filter"),
                ("z", "key.fold"),
                ("o", "key.tab"),
                ("l h", "key.open_fold"),
                ("b", "key.branch"),
                ("t", "key.revert"),
                ("C", "key.cherry_pick"),
                ("m", "key.merge_commit"),
                ("T", "key.tag"),
                ("s", "key.stash"),
                ("Esc", "key.back"),
            ],
            View::Changes => &[
                ("Space", "key.stage"),
                ("c", "key.commit"),
                ("l h", "key.lines"),
                ("a", "key.stage_all"),
                ("j k", "key.select"),
                ("v", "key.range"),
                ("s", "key.split"),
                ("d", "key.discard"),
                ("A", "key.amend"),
                ("[ ]", "key.worktree"),
                ("Enter", "key.side"),
                ("Esc", "key.back"),
            ],
            View::Worktree => &[
                ("j k", "key.select"),
                ("o", "key.tab"),
                ("p", "key.pr"),
                ("r", "key.reorder"),
                ("M", "key.rebase_trunk"),
                ("Enter", "key.in_graph"),
                ("m", "key.take_main"),
                ("x", "key.remove"),
                ("d", "key.branch_diff"),
                ("c", "key.tidy"),
            ],
            View::Conflict => &[
                ("] [", "key.place"),
                ("o", "key.ours"),
                ("t", "key.theirs"),
                ("b B", "key.both"),
                ("Enter", "key.cf_reused"),
                ("e", "key.cf_edit"),
                ("g", "key.cf_handoff"),
                ("w", "key.cf_external"),
                ("x", "key.cf_forget"),
                ("Ctrl+Enter", "key.cf_commit"),
                ("PageDown PageUp", "key.scroll"),
            ],
        }
    }
}

/// The keys that work in every view.
pub const GLOBAL_KEYS: [(&str, &str); 6] =
    [("1 2 3 4", "key.views"), ("N", "key.new_work"), ("f", "key.fetch"), ("p", "key.pull"), ("P", "key.push"), ("F", "key.push_over")];

/// The keys of the branch review, drawn over the worktree table.
pub const REVIEW_KEYS: [(&str, &str); 11] = [
    ("j k", "key.file"),
    ("J K", "key.line"),
    ("n", "key.note"),
    ("g", "key.hand_notes"),
    ("Space", "key.seen"),
    ("u", "key.uncommitted"),
    ("s", "key.since_last"),
    ("m", "key.take_main"),
    ("Enter", "key.side"),
    ("PageDown PageUp", "key.scroll"),
    ("Esc", "key.back"),
];

/// What the window shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Main,
    Settings,
    Help,
}

/// The repository the window is on, as far as it has been read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Repo {
    Loading,
    Ready(Summary),
    Failed(git::Error),
}

/// A worktree whose Claude was handed a conflict: see [`App::handed`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Handed {
    pub wt: PathBuf,
    /// Seen running since it was handed.
    pub ran: bool,
    /// Reads that found it stopped without ever seeing it run.
    pub polls: u32,
    /// It was handed a conflict (not notes): look at the markers when it stops.
    pub conflict: bool,
}

pub struct App {
    pub view: View,
    pub mode: Mode,
    pub prefs: Prefs,
    /// The language in force (`en`, `ja`), from common.toml or the OS.
    pub lang: String,
    /// The folder the window was opened on.
    pub dir: PathBuf,
    pub repo: Repo,
    /// The git on the PATH; `None` until it has answered.
    pub git: Option<Version>,
    /// The two ends of the reading thread.
    pub reading: Option<Reading>,
    pub graph: Graph,
    /// The selected commit in full, on the right.
    pub detail: Pane,
    /// The Claude sessions in each worktree that has some; `None` when
    /// tsumugi is not there (or not asked yet).
    pub claude: Option<crate::repo::Sessions>,
    /// The worktrees and how they stand, for the worktree table.
    pub trees: Trees,
    /// Each worktree's uncommitted change, those with only files too big to
    /// count too (the graph shows the others).
    pub wips: Vec<Wip>,
    /// Which worktrees would conflict with which.
    pub clashes: Clashes,
    /// The commits not seen yet, per worktree.
    pub unseen: Unseen,
    /// The worktrees whose Claude was handed a conflict, until it stops.
    pub handed: Vec<Handed>,
    /// Whether the window is in front (a notification is told only when it is not).
    pub front: bool,
    /// The last failure told in a toast, for the bug report (F12).
    pub last_error: Option<String>,
    /// The worktree table's selected row.
    pub wt_sel: usize,
    /// The branch review (`d`), over the worktree table.
    pub review: Option<Review>,
    /// The notes of closed reviews, by worktree, until they are handed over.
    pub kept_notes: Vec<(PathBuf, Vec<michi_core::notes::Note>)>,
    /// The `2` view: one worktree's files to stage and commit.
    pub changes: Changes,
    /// The `4` view: the worktrees stopped on a clash, and their places.
    pub conflicts: Conflicts,
    /// The view of the frame before: the `2` view reads its files again
    /// when it comes in sight.
    last_view: View,
    /// The side column's groups that are open (`b:feat`, `r:origin`).
    pub side_open: HashSet<String>,
    /// A commit clicked in the side column that is not read yet: read on
    /// until it is, then select it.
    seeking: Option<Oid>,
    /// The worktree whose uncommitted change the detail shows.
    detail_wip: Option<PathBuf>,
    /// A new window title, for the frame to send once.
    pub new_title: Option<String>,
    /// One line at the foot, until its instant.
    pub toast: Option<(String, Instant)>,
    /// The fetch, pull or push that runs.
    pub net: Option<Running>,
    /// When the repository last fetched (`FETCH_HEAD`'s time).
    pub last_fetch: Option<SystemTime>,
    /// A job waiting for a yes, under the title bar.
    pub band: Option<Band>,
    /// rerere was asked about (or read as said) in this session.
    pub rerere_asked: bool,
    /// The branch, tag or stash palette, over the graph.
    pub picker: Option<Picker>,
    /// The new-work form while it is open.
    pub new_work: Option<crate::newwork::NewWork>,
    /// The clean-up list, over the worktree table.
    pub tidy: Option<crate::tidy::Tidy>,
    pub reorder: Option<crate::reorder::Reorder>,
    /// The stash clicked in the side column, and the commit the graph had
    /// selected then: the detail shows the stash until that moves.
    pub side_stash: Option<(Oid, Option<Oid>)>,
    pub quit: bool,
    /// The command palette while it is open.
    pub cmd: Option<ito_palette::View>,
    /// The key list's first line, and how much of it the last frame showed.
    pub help_scroll: usize,
    pub help_drawn: std::cell::Cell<Option<ito_help::Drawn>>,
}

impl App {
    pub fn new(prefs: Prefs, dir: PathBuf) -> Self {
        let mut app = Self {
            view: View::Graph,
            mode: Mode::Main,
            prefs,
            lang: "en".into(),
            dir,
            repo: Repo::Loading,
            git: None,
            reading: None,
            graph: Graph::default(),
            detail: Pane::default(),
            claude: None,
            trees: Trees::default(),
            wips: Vec::new(),
            clashes: Clashes::default(),
            unseen: Unseen::default(),
            handed: Vec::new(),
            front: true,
            last_error: None,
            wt_sel: 0,
            review: None,
            kept_notes: Vec::new(),
            changes: Changes::default(),
            conflicts: Conflicts::default(),
            last_view: View::Graph,
            side_open: HashSet::new(),
            seeking: None,
            detail_wip: None,
            new_title: None,
            toast: None,
            net: None,
            last_fetch: None,
            band: None,
            rerere_asked: false,
            picker: None,
            new_work: None,
            tidy: None,
            reorder: None,
            side_stash: None,
            quit: false,
            cmd: None,
            help_scroll: 0,
            help_drawn: std::cell::Cell::new(None),
        };
        app.set_language();
        app
    }

    /// The language common.toml names, else the OS's.
    pub fn set_language(&mut self) {
        let os = ito_common::os_language();
        self.lang = ito_i18n::resolve(None, self.prefs.common.language.as_deref(), os.as_deref(), &crate::i18n::AVAILABLE);
    }

    /// The words for `key` in the language in force.
    pub fn tr(&self, key: &'static str) -> &'static str {
        crate::i18n::tr(&self.lang, key)
    }

    /// What a worktree in the middle of `d` is doing, for the texts' `{doing}`.
    /// The Japanese texts leave out the spaces around it, so that a Japanese
    /// word sits close (`マージの途中`) and an ASCII one gets them here
    /// (`cherry-pick の途中`).
    pub fn doing_word(&self, d: michi_core::branch::Doing) -> String {
        use michi_core::branch::Doing;
        let word = self.tr(match d {
            Doing::Merge => "doing.merge",
            Doing::Rebase => "doing.rebase",
            Doing::CherryPick => "doing.cherry",
            Doing::Revert => "doing.revert",
            Doing::Bisect => "doing.bisect",
        });
        if self.lang == "ja" && word.is_ascii() {
            format!(" {word} ")
        } else {
            word.to_owned()
        }
    }

    pub fn say(&mut self, text: impl Into<String>) {
        self.say_for(text, 4);
    }

    pub fn say_for(&mut self, text: impl Into<String>, secs: u64) {
        self.toast = Some((text.into(), Instant::now() + Duration::from_secs(secs)));
    }

    /// How long until the toast goes, if there is one.
    pub fn toast_left(&self) -> Option<Duration> {
        self.toast.as_ref().map(|(_, until)| until.saturating_duration_since(Instant::now()))
    }

    /// Fold in what the reading thread has sent since the last frame.
    pub fn pump(&mut self) {
        let got: Vec<Answer> = match &self.reading {
            Some(r) => r.answers.try_iter().collect(),
            None => return,
        };
        for a in got {
            self.take(a);
        }
        self.seek();
        self.changes_sync();
    }

    /// The stash the detail shows in place of the selected commit: the one
    /// the stash palette has selected, else the one clicked in the side
    /// column while the graph's selection stays where it was.
    pub fn stash_shown(&self) -> Option<Oid> {
        if let Some(PickItem::Stash { id, .. }) = self.picker.as_ref().and_then(|p| p.selected()) {
            return Some(*id);
        }
        let (id, at) = self.side_stash?;
        let there = self.graph.setup().refs.stashes.iter().any(|s| s.id == id);
        (there && at == self.graph.selected()).then_some(id)
    }

    /// A stash in the side column is clicked: the detail shows it.
    pub fn show_stash(&mut self, id: Oid) {
        self.side_stash = Some((id, self.graph.selected()));
    }

    /// Asks for the selected commit (or the stash shown) in full when the
    /// selection has changed, and counts a trunk merge's commits; whether
    /// it had changed.
    pub fn sync_detail(&mut self) -> bool {
        let stash = self.stash_shown();
        let id = stash.or(self.graph.selected());
        // The same worktree's change read anew: the detail keeps its place.
        let wip = self.graph.selected_wip().filter(|_| stash.is_none()).map(|w| w.path.clone());
        let changed = if wip.is_some() && wip == self.detail_wip { self.detail.refresh(id) } else { self.detail.select(id) };
        self.detail_wip = wip;
        // A commit picked by a key or the mouse is seen now.
        if let (true, Some(id)) = (self.graph.take_picked(), id) {
            if self.unseen.remove(&id) {
                if let Some(r) = &self.reading {
                    r.seen(id);
                }
            }
        }
        if changed {
            if let (Some(id), Some(r)) = (id, &self.reading) {
                if stash.is_some() {
                    r.stash(id);
                } else {
                    // A merge's diff (the whole branch's) comes in stage 2.
                    let patch = self.graph.find_commit(&id).is_none_or(|c| c.parents.len() < 2);
                    r.detail(id, patch);
                }
            }
        }
        if let Some(id) = id.filter(|_| stash.is_none()) {
            let read = self.graph.read();
            if self.detail.merged(id, read).is_none() {
                let commits = self.graph.merged_by(&id);
                self.detail.set_merged(id, read, commits);
            }
        }
        changed
    }

    /// A row of the side column is clicked: select its commit, reading on
    /// to it when it is further down than read.
    pub fn go_to(&mut self, id: Oid) {
        self.seeking = None;
        if !self.graph.select_id(&id) {
            self.seeking = Some(id);
            self.seek();
        }
    }

    /// Reads on towards the commit the side column wants.
    fn seek(&mut self) {
        let Some(id) = self.seeking else { return };
        if self.graph.select_id(&id) {
            self.seeking = None;
        } else if self.graph.done() || self.graph.error().is_some() {
            self.seeking = None;
            let text = self.tr("side.missing").replace("{}", &id.short());
            self.say(text);
        } else if self.graph.ask_more() {
            self.ask(Ask::More);
        }
    }

    pub fn take(&mut self, answer: Answer) {
        match answer {
            Answer::Version(Ok(v)) => {
                self.git = Some(v);
                if !v.is_enough() {
                    let text = self.tr("git.old").replace("{v}", &v.to_string()).replace("{min}", &git::MIN.to_string());
                    self.say_for(text, 10);
                }
            }
            // The summary that follows says why (git missing, mostly).
            Answer::Version(Err(_)) => {}
            Answer::Summary(Ok(s)) => {
                self.new_title = Some(format!("{} — michi", s.name));
                self.repo = Repo::Ready(s);
            }
            Answer::Summary(Err(e)) => {
                let word = self.tr(failed_word(&e));
                let path = self.dir.display();
                let text = match &e {
                    git::Error::Git(d) | git::Error::Io(d) | git::Error::Told { line: d, .. } => format!("{word}: {d} — {path}"),
                    _ => format!("{word} — {path}"),
                };
                self.say_for(text, 10);
                self.repo = Repo::Failed(e);
            }
            Answer::Setup { gen, setup } => self.graph.start(gen, *setup),
            Answer::Commits { gen, batch, done } => self.graph.commits(gen, batch, done),
            Answer::Total { gen, count } => self.graph.total(gen, count),
            Answer::Failed { gen, error } => {
                let text = self.tr("history.failed").replace("{}", &error.to_string());
                self.say_for(text, 10);
                self.graph.failed(gen, error);
            }
            Answer::Detail { id, result } => self.detail.take(id, result),
            Answer::Wips(wips) => {
                self.graph.set_wips(wips.iter().filter(|w| !w.files.is_empty()).cloned().collect());
                self.wips = wips;
                self.review_sync();
            }
            Answer::Trees(trees) => {
                // The selection stays on its worktree when the list changes.
                let path = self.trees.trees.get(self.wt_sel).map(|t| t.wt.path.clone());
                self.wt_sel = path.and_then(|p| trees.trees.iter().position(|t| t.wt.path == p)).unwrap_or(0);
                // The title follows a switch made here or anywhere else, and
                // worktrees added or taken away.
                if let Repo::Ready(s) = &mut self.repo {
                    s.worktrees = trees.trees.len();
                    let top = paths::key(&s.top);
                    if let Some(t) = trees.trees.iter().find(|t| paths::key(&t.wt.path) == top) {
                        s.head = summary::head_of(&t.wt);
                    }
                }
                self.trees = trees;
                self.review_sync();
            }
            Answer::Claude(c) => {
                self.claude = c;
                self.handed_stopped();
            }
            Answer::Clashes(c) => self.clashes = c,
            Answer::Conflicts(c) => {
                let had = !self.conflicts.all.is_empty();
                self.conflicts.set(c);
                // All committed or called off: nothing to show here.
                if had && self.conflicts.all.is_empty() && self.view == View::Conflict {
                    self.view = View::Graph;
                }
            }
            Answer::Unseen(u) => self.unseen = u,
            Answer::ReviewList { gen, result, checks } => {
                let wish = self.review.as_mut().and_then(|r| r.take_list(gen, result.map(|l| *l), checks));
                self.wish(wish);
            }
            Answer::ReviewFile { gen, result } => {
                if let Some(r) = self.review.as_mut() {
                    r.take_file(gen, result.map(|c| *c));
                }
            }
            Answer::Reread => {
                if self.view == View::Changes {
                    let wish = self.changes.refresh();
                    self.stage_wish(wish);
                }
            }
            Answer::ChangesList { gen, result } => {
                let wishes = self.changes.take_list(gen, result.map(|l| *l));
                self.stage_wish(wishes);
            }
            Answer::ChangesFile { gen, result } => self.changes.take_file(gen, result.map(|d| d.map(|d| *d))),
            Answer::ChangesLast { gen, result } => self.changes.take_last(gen, result),
            Answer::ChangesDone { wt, commit, result } => {
                match &result {
                    Ok(Did::Committed(id)) => {
                        let text = self.tr("changes.committed").replace("{}", &id.short());
                        self.say(text);
                    }
                    Ok(Did::Discarded { path, untracked, lines, saved }) => {
                        let key = match (untracked, lines) {
                            (true, _) => "changes.deleted",
                            (false, 0) => "changes.discarded",
                            _ => "changes.discarded_lines",
                        };
                        let text = self.tr(key).replace("{file}", path).replace("{n}", &lines.to_string());
                        self.say_for(text.replace("{saved}", michi_core::backup::short(saved)), 10);
                    }
                    Ok(Did::Staged) => {}
                    Err(e) => {
                        let text = self.tr("changes.failed").replace("{}", &e.to_string());
                        self.say_for(text, 8);
                        if let git::Error::Told { all, .. } = e {
                            self.changes.tell(&wt, commit, all.clone());
                        }
                    }
                }
                let wish = self.changes.take_done(&wt, commit, result.is_ok());
                self.stage_wish(wish);
            }
            Answer::NetProgress(p) => self.net_progress(p),
            Answer::NetDone { job, result } => self.net_done(job, result),
            Answer::ActDone { task, result } => self.act_done(task, result),
            Answer::Fetched(at) => self.last_fetch = at,
        }
    }

    /// Sends what the `2` view wants to the reading thread.
    fn stage_wish(&self, wishes: impl IntoIterator<Item = StageWish>) {
        if let Some(r) = &self.reading {
            for w in wishes {
                r.changes(w);
            }
        }
    }

    /// The worktrees the `2` view can show: those with a folder.
    pub fn change_trees(&self) -> impl Iterator<Item = &Tree> {
        self.trees.trees.iter().filter(|t| !t.wt.bare && t.wt.prunable.is_none())
    }

    /// Whether a Claude is writing in the worktree the `2` view shows.
    pub fn claude_writing(&self) -> bool {
        let Some(key) = self.changes.wt.as_deref().map(paths::key) else { return false };
        let sessions = self.claude.iter().flatten().find(|(p, _)| paths::key(p) == key);
        sessions.is_some_and(|(_, s)| s.iter().any(|s| s.is_agent() && s.state == State::Running))
    }

    /// Each frame: while the `2` view is in sight it has a worktree (the
    /// window's own, else the first), whose files are read again when the
    /// view comes in sight; a yes to write beside a Claude goes once it
    /// stops.
    pub fn changes_sync(&mut self) {
        let entering = self.view == View::Changes && self.last_view != View::Changes;
        self.last_view = self.view;
        if self.view != View::Changes {
            return;
        }
        let top = match &self.repo {
            Repo::Ready(s) => Some(s.top.clone()),
            _ => None,
        };
        let shown = self.changes.wt.as_deref().map(paths::key);
        let trees: Vec<_> = self.change_trees().map(|t| (paths::key(&t.wt.path), t.wt.path.clone())).collect();
        let gone = match &shown {
            None => true,
            Some(key) => !trees.is_empty() && !trees.iter().any(|(k, _)| k == key),
        };
        if gone {
            let top_key = top.as_deref().map(paths::key);
            let pick = trees.iter().find(|(k, _)| Some(k) == top_key.as_ref()).or(trees.first()).map(|(_, p)| p.clone()).or(top);
            if let Some(wt) = pick {
                let wish = self.changes.open(wt);
                self.stage_wish(wish);
            }
        } else if entering {
            let wish = self.changes.refresh();
            self.stage_wish(wish);
        }
        if self.changes.claude_ok && !self.claude_writing() {
            self.changes.claude_stopped();
        }
    }

    /// `[` `]`: the `2` view shows the worktree before or after.
    fn changes_step(&mut self, by: i64) {
        let trees: Vec<PathBuf> = self.change_trees().map(|t| t.wt.path.clone()).collect();
        if trees.len() < 2 {
            let text = self.tr("changes.one_worktree");
            self.say(text);
            return;
        }
        let key = self.changes.wt.as_deref().map(paths::key);
        let at = trees.iter().position(|p| Some(paths::key(p)) == key).unwrap_or(0) as i64;
        let next = (at + by).rem_euclid(trees.len() as i64) as usize;
        let wish = self.changes.open(trees[next].clone());
        self.stage_wish(wish);
    }

    /// The `2` view shows the worktree at `wt` (a click on its name).
    pub fn changes_open(&mut self, wt: PathBuf) {
        if self.changes.wt.as_deref().map(paths::key) != Some(paths::key(&wt)) {
            let wish = self.changes.open(wt);
            self.stage_wish(wish);
        }
    }

    /// The `2` view selects its `k`th file (a click).
    pub fn changes_select(&mut self, k: usize) {
        let wish = self.changes.select(k);
        self.stage_wish(wish);
    }

    /// The files the `2` tab counts: the shown worktree's as read, else its
    /// uncommitted change's (the window's own worktree's before one is
    /// shown).
    pub fn changes_count(&self) -> usize {
        if let (View::Changes, Some(l)) = (self.view, self.changes.lists()) {
            let paths: HashSet<&str> = l.staged.iter().chain(&l.unstaged).map(|f| f.path.as_str()).collect();
            return paths.len();
        }
        let wt = self.changes.wt.clone().or_else(|| match &self.repo {
            Repo::Ready(s) => Some(s.top.clone()),
            _ => None,
        });
        let Some(key) = wt.as_deref().map(paths::key) else { return 0 };
        self.wips.iter().find(|w| paths::key(&w.path) == key).map_or(0, |w| w.files.len())
    }

    /// Ctrl+Enter: commit what is staged in the `2` view. A worktree in
    /// the middle of a merge or the like commits it in the `4` view.
    pub fn changes_commit(&mut self) {
        let wt = self.changes.wt.clone().or_else(|| match &self.repo {
            Repo::Ready(s) => Some(s.top.clone()),
            _ => None,
        });
        if let Some((wt, d)) = wt.and_then(|wt| self.doing_in(&wt).map(|d| (wt, d))) {
            let text = self.tr("changes.commit_doing").replace("{wt}", &crate::act::name(&wt)).replace("{doing}", &self.doing_word(d));
            self.say_for(text, 6);
            return;
        }
        let claude = self.claude_writing();
        let done = self.changes.commit(claude);
        self.changes_did(done);
    }

    /// Sends what the `2` view wants, or says why it could not.
    fn changes_did(&mut self, done: Result<impl IntoIterator<Item = StageWish>, &'static str>) {
        match done {
            Ok(wishes) => self.stage_wish(wishes),
            Err(word) => {
                let text = self.tr(word);
                self.say(text);
            }
        }
    }

    /// The `2` view's own keys. While a write waits for a yes, `y` gives it
    /// and `n` takes it back; the other keys do nothing.
    fn changes_key(&mut self, c: char) -> bool {
        let claude = self.claude_writing();
        let ch = &mut self.changes;
        if ch.confirm.is_some() {
            match c {
                'y' => {
                    let wish = ch.yes();
                    self.stage_wish(wish);
                }
                'n' => ch.confirm = None,
                _ => {}
            }
            return true;
        }
        let lines = ch.focus == Focus::Lines;
        let none = || Ok(None);
        let done = match c {
            'j' | 'k' if lines => {
                ch.move_cursor(if c == 'j' { 1 } else { -1 });
                none()
            }
            'j' => Ok(ch.move_by(1)),
            'k' => Ok(ch.move_by(-1)),
            ' ' if lines => ch.apply_lines(claude),
            ' ' => ch.toggle_file(claude),
            'a' if lines => {
                ch.pick_hunk();
                none()
            }
            'a' => ch.toggle_all(claude),
            'v' if lines => {
                ch.toggle_range();
                none()
            }
            'v' | 'l' if !lines => ch.enter_lines().map(|()| None),
            'h' if lines => {
                ch.leave_lines();
                none()
            }
            // `v` and `l` in the lines, `h` in the files: nothing to do.
            'v' | 'l' | 'h' => none(),
            'c' => {
                ch.focus_message = true;
                none()
            }
            'A' => Ok(ch.toggle_amend()),
            's' => ch.split(),
            'd' => ch.discard(claude),
            '[' | ']' => {
                self.changes_step(if c == ']' { 1 } else { -1 });
                return true;
            }
            _ => return false,
        };
        self.changes_did(done);
        true
    }

    /// The `2` view's keys with a name.
    fn changes_named(&mut self, name: &str) -> bool {
        let ch = &mut self.changes;
        if ch.confirm.is_some() {
            return true;
        }
        let lines = ch.focus == Focus::Lines;
        let wish = match name {
            "ArrowDown" | "ArrowUp" if lines => {
                ch.move_cursor(if name == "ArrowDown" { 1 } else { -1 });
                None
            }
            "Home" | "End" if lines => {
                ch.move_cursor(if name == "End" { i64::MAX / 2 } else { i64::MIN / 2 });
                None
            }
            "ArrowDown" => ch.move_by(1),
            "ArrowUp" => ch.move_by(-1),
            "Home" => ch.select(0),
            "End" => ch.select(usize::MAX),
            "ArrowRight" => return self.changes_key('l'),
            "ArrowLeft" => return self.changes_key('h'),
            "PageDown" => {
                ch.pages += 1;
                None
            }
            "PageUp" => {
                ch.pages -= 1;
                None
            }
            "Enter" => {
                ch.wide = !ch.wide;
                None
            }
            _ => return false,
        };
        self.stage_wish(wish);
        true
    }

    /// `d`: the selected worktree's branch as a whole, from where it left
    /// the trunk.
    pub fn open_review(&mut self) {
        let Some(t) = self.wt_selected() else { return };
        let (path, branch) = (t.wt.path.clone(), t.wt.branch.clone());
        let word = match branch {
            _ if self.trees.trunk_head.is_none() => "review.no_trunk",
            None => "review.detached",
            Some(branch) => {
                let notes = self.notes_back(&path);
                let mut r = Review::new(path, branch);
                r.notes = notes;
                self.review = Some(r);
                self.review_sync();
                return;
            }
        };
        let text = self.tr(word);
        self.say(text);
    }

    /// Closes the branch review.
    pub fn close_review(&mut self) {
        if let Some(r) = self.review.take() {
            self.notes_away(r.path, r.notes);
        }
        if let Some(r) = &self.reading {
            r.review_closed();
        }
    }

    /// What the branch review reads to: the trunk, its worktree's commit and
    /// uncommitted change as they are now; `None` while its worktree is not
    /// on its branch.
    pub fn review_ends(&self) -> Option<Ends> {
        let r = self.review.as_ref()?;
        let t = self.trees.trees.iter().find(|t| t.wt.path == r.path)?;
        if t.wt.branch.as_deref() != Some(r.branch.as_str()) {
            return None;
        }
        let trunk = self.trees.trunk_head?;
        let head = Oid::parse(&t.wt.head)?;
        let wip = self.wip_in(&r.path).filter(|w| !w.files.is_empty() && w.head == head).map(|w| w.commit);
        Some(Ends { trunk, head, wip })
    }

    /// Asks for the branch review's list again when what it reads to has
    /// moved, while it is in sight.
    pub fn review_sync(&mut self) {
        if self.view != View::Worktree {
            return;
        }
        let Some(ends) = self.review_ends() else { return };
        let wish = self.review.as_mut().and_then(|r| r.ask(ends));
        self.wish(wish);
    }

    /// Sends what the branch review wants to the reading thread.
    pub(crate) fn wish(&self, wishes: impl IntoIterator<Item = Wish>) {
        if let Some(r) = &self.reading {
            for w in wishes {
                r.review(w);
            }
        }
    }

    /// The branch review's own keys.
    fn review_key(&mut self, c: char) -> bool {
        if c == 'm' && self.review.is_some() {
            self.ask_merge();
            return true;
        }
        match c {
            'n' if self.review.is_some() => return self.review_note_key(),
            'g' if self.review.is_some() => return self.review_hand_key(),
            _ => {}
        }
        let Some(r) = self.review.as_mut() else { return false };
        let mut said = None;
        let wishes = match c {
            'J' => {
                r.step(1);
                Vec::new()
            }
            'K' => {
                r.step(-1);
                Vec::new()
            }
            'j' => r.move_by(1).into_iter().collect(),
            'k' => r.move_by(-1).into_iter().collect(),
            ' ' => r.check(),
            'u' => {
                r.wip = !r.wip;
                Vec::new()
            }
            's' => {
                said = r.toggle_since().err();
                Vec::new()
            }
            _ => return false,
        };
        self.wish(wishes);
        match said {
            Some(word) => {
                let text = self.tr(word);
                self.say(text);
            }
            None => self.review_sync(),
        }
        true
    }

    fn review_note_key(&mut self) -> bool {
        self.review_note();
        true
    }

    fn review_hand_key(&mut self) -> bool {
        self.review_hand();
        true
    }

    /// The branch review's keys with a name.
    fn review_named(&mut self, name: &str) -> bool {
        let Some(r) = self.review.as_mut() else { return false };
        let wish = match name {
            "ArrowDown" => r.move_by(1),
            "ArrowUp" => r.move_by(-1),
            "Home" => r.select(0),
            "End" => r.select(usize::MAX),
            "PageDown" => {
                r.pages += 1;
                None
            }
            "PageUp" => {
                r.pages -= 1;
                None
            }
            "Enter" => {
                r.wide = !r.wide;
                None
            }
            _ => return false,
        };
        self.wish(wish);
        true
    }

    /// The keys that work now: the view's, or the branch review's over the
    /// worktree table.
    pub fn keys(&self) -> &'static [(&'static str, &'static str)] {
        if self.view == View::Worktree && self.review.is_some() {
            &REVIEW_KEYS
        } else {
            self.view.keys()
        }
    }

    /// Says so when a Claude handed a conflict has stopped writing: it was
    /// seen running and no agent in its worktree runs now. One that is not
    /// seen running for a while is dropped, as is one whose sessions are gone.
    fn handed_stopped(&mut self) {
        let mut said = Vec::new();
        let mut check = Vec::new();
        let mut watched = std::mem::take(&mut self.handed);
        watched.retain_mut(|h| {
            let agents: Vec<_> = self.claude_in(&h.wt).iter().filter(|s| s.is_agent()).collect();
            if agents.is_empty() {
                return h.polls < 3 && h.ran;
            }
            if agents.iter().any(|s| s.state == State::Running) {
                h.ran = true;
                return true;
            }
            h.polls += 1;
            if h.ran || h.polls >= 3 {
                if h.conflict {
                    check.push(h.wt.clone());
                }
                said.push((self.tr("claude.handed_done").replace("{wt}", &crate::act::name(&h.wt)), crate::act::name(&h.wt)));
                return false;
            }
            true
        });
        self.handed = watched;
        if let Some((t, wt)) = said.last().cloned() {
            self.say_for(t, 10);
            let (title, body) = (self.tr("notify.title_done").to_owned(), self.tr("claude.handed_done").replace("{wt}", &wt));
            self.tell("done", &title, &body);
        }
        for wt in check {
            self.act(crate::act::Task::Markers { wt });
        }
    }

    /// Tell of `event` outside the window, when the window is behind another.
    pub fn tell(&self, event: &str, title: &str, body: &str) {
        if !self.front {
            crate::notify::tell(&self.prefs.notify, event, title, body);
        }
    }

    /// Watches the Claude in the worktree at `wt`, which was handed a conflict.
    pub fn watch_handed(&mut self, wt: PathBuf) {
        self.handed.retain(|h| h.wt != wt);
        self.handed.push(Handed { wt, ran: false, polls: 0, conflict: false });
    }

    /// The Claude sessions in the worktree at `path`, the one to show first
    /// first.
    pub fn claude_in(&self, path: &Path) -> &[Session] {
        self.claude.iter().flatten().find(|(p, _)| p == path).map_or(&[], |(_, s)| s.as_slice())
    }

    /// The uncommitted change of the worktree at `path`, if it has one.
    pub fn wip_in(&self, path: &Path) -> Option<&Wip> {
        self.wips.iter().find(|w| w.path == path)
    }

    /// Whether a merge is under way: the predictions are then replaced by
    /// the real conflicts (`4`) and are not shown.
    pub fn merging(&self) -> bool {
        self.trees.trees.iter().any(|t| t.doing == Some(michi_core::branch::Doing::Merge))
    }

    /// The sides the worktree at `path` would conflict with, and how.
    pub fn clashes_in(&self, path: &Path) -> Vec<(&Side, &Outcome)> {
        let Some(i) = self.clashes.side_of(path) else { return Vec::new() };
        self.clashes.of(i).into_iter().map(|(j, o)| (&self.clashes.sides[j], o)).collect()
    }

    /// The worktree table's selected row.
    pub fn wt_selected(&self) -> Option<&Tree> {
        self.trees.trees.get(self.wt_sel)
    }

    /// Moves the worktree table's selection by `by` rows, stopping at the
    /// ends.
    pub fn wt_move(&mut self, by: i32) {
        let last = self.trees.trees.len().saturating_sub(1) as i64;
        self.wt_sel = (self.wt_sel as i64 + i64::from(by)).clamp(0, last) as usize;
    }

    /// Opens the graph on the selected worktree: its uncommitted change, or
    /// its commit when it has none.
    pub fn wt_in_graph(&mut self) {
        let Some(t) = self.wt_selected() else { return };
        let path = t.wt.path.clone();
        let head = Oid::parse(&t.wt.head);
        self.view = View::Graph;
        self.detail.wide = false;
        if !self.graph.select_wip(&path) {
            if let Some(id) = head {
                self.go_to(id);
            }
        }
    }

    /// The table's button: read the worktrees again now.
    pub fn reread(&mut self) {
        if let Some(r) = &self.reading {
            r.reread();
        }
        let text = self.tr("wt.rereading");
        self.say(text);
    }

    /// The graph's rows in sight reach row `last`: read on before they run
    /// out.
    pub fn reach(&mut self, last: u32) {
        if self.graph.wants_more(last) {
            self.ask(Ask::More);
        }
    }

    /// The row at the graph's foot is clicked: read the next batch.
    pub fn more(&mut self) {
        if self.graph.ask_more() {
            self.ask(Ask::More);
        }
    }

    fn ask(&self, a: Ask) {
        if let Some(r) = &self.reading {
            let _ = r.asks.send(a);
        }
    }

    /// The git on the PATH when it is older than michi needs.
    pub fn old_git(&self) -> Option<Version> {
        self.git.filter(|v| !v.is_enough())
    }

    /// A bare character the keymap binds (`,` and `?` by default): it does
    /// what it is bound to. Whether it was one.
    fn bare_act(&mut self, c: char) -> bool {
        match self.prefs.keymap.act(ito_keys::Key::char(c)) {
            Some(a) => {
                self.run_key(a);
                true
            }
            None => false,
        }
    }

    /// A key with Ctrl or Alt, or a named one, from the keymap. Whether it
    /// was bound.
    pub fn key(&mut self, k: ito_keys::Key) -> bool {
        match self.prefs.keymap.act(k) {
            Some(a) => {
                self.run_key(a);
                true
            }
            None => false,
        }
    }

    pub(crate) fn run_key(&mut self, a: crate::keymap::Act) {
        use crate::keymap::Act;
        match a {
            Act::Settings => self.mode = if self.mode == Mode::Settings { Mode::Main } else { Mode::Settings },
            Act::Help => {
                self.help_scroll = 0;
                self.mode = if self.mode == Mode::Help { Mode::Main } else { Mode::Help };
            }
            Act::Palette => self.toggle_palette(),
            Act::BugReport => self.bug_report(),
            Act::Quit => self.quit = true,
            Act::Scale(to) => self.scale(to),
            Act::Noop => {}
        }
    }

    /// F12: the bug report form in the browser, with the version, the OS and
    /// what michi knew filled in (by name, never by path). The browser is
    /// started off the UI thread.
    fn bug_report(&mut self) {
        let url = crate::bugreport::url(self.prefs.base.as_deref(), self.prefs.common.scale_or_one(), self.last_error.as_deref());
        std::thread::spawn(move || {
            let _ = michi_core::pr::open_url(&url);
        });
        let text = self.tr("bugreport.opened").to_owned();
        self.say_for(text, 6);
    }

    /// Zoom in, out or back to 100%, kept in common.toml.
    pub fn scale(&mut self, to: ito_common::ScaleTo) {
        let next = ito_common::scale_step(self.prefs.common.scale_or_one(), to);
        self.prefs.common.scale = Some(next);
        let no_folder = crate::i18n::tr(&self.lang, "settings.no_folder");
        self.prefs.common_change(vec![ito_common::scale_change(next)], no_folder);
        let text = ito_common::scale_text(next, &self.lang);
        self.say(text);
    }

    /// The key list: every key the keymap binds (as it is now, so a changed
    /// key shows), the fixed keys of every view, then those of the view in front.
    pub fn help_lines(&self) -> Vec<ito_help::Line> {
        use ito_help::Line;
        let mut lines = vec![Line::Head(self.tr("help.global").to_owned())];
        let bind = |k: &str, w: &str| Line::Bind { keys: k.to_owned(), desc: w.to_owned() };
        lines.extend(GLOBAL_KEYS.iter().map(|(k, w)| bind(k, self.tr(w))));
        lines.extend(ito_keymap::help_lines(&self.prefs.keymap.main, "noop").into_iter().map(|l| {
            let desc = if l.desc.is_empty() { l.run.to_owned() } else { crate::i18n::tr_desc(&self.lang, l.desc) };
            Line::Bind { keys: l.keys, desc }
        }));
        let view = if self.keys() == self.view.keys() { self.view.label() } else { "key.branch_diff" };
        lines.push(Line::Blank);
        lines.push(Line::Head(self.tr(view).to_owned()));
        lines.extend(self.keys().iter().map(|(k, w)| bind(k, self.tr(w))));
        lines
    }

    /// The arrows, pages, Home and End scroll the key list.
    fn help_named(&mut self, name: &str) -> bool {
        let Some(d) = self.help_drawn.get() else { return false };
        let page = d.visible.saturating_sub(1).max(1) as i64;
        self.help_scroll = match name {
            "ArrowDown" => ito_help::scrolled(d.scroll, 1, d.max),
            "ArrowUp" => ito_help::scrolled(d.scroll, -1, d.max),
            "PageDown" => ito_help::scrolled(d.scroll, page, d.max),
            "PageUp" => ito_help::scrolled(d.scroll, -page, d.max),
            "Home" => 0,
            "End" => d.max,
            _ => return false,
        };
        true
    }

    /// A character typed (not a key with Ctrl or Alt: those are the
    /// shortcuts). Returns whether it was used.
    pub fn feed(&mut self, c: char) -> bool {
        match (self.mode, c) {
            (Mode::Help, c) if self.bare_act(c) => {}
            (Mode::Settings, _) | (Mode::Help, _) => return false,
            (Mode::Main, _) if self.picker.is_some() || self.new_work.is_some() => return false,
            (Mode::Main, c) if self.band.is_some() => return self.band_key(c),
            (Mode::Main, c) if self.tidy.is_some() => return self.tidy_key(c),
            (Mode::Main, c) if self.reorder.is_some() => return self.reorder_key(c),
            (Mode::Main, c) if self.bare_act(c) => {}
            (Mode::Main, c) => match View::from_digit(c) {
                Some(View::Conflict) => self.open_conflicts(),
                Some(v) => {
                    self.view = v;
                    self.review_sync();
                }
                None if self.view == View::Graph && self.graph_key(c) => {}
                None if self.view == View::Worktree && self.review.is_some() && self.review_key(c) => {}
                None if self.view == View::Worktree && self.review.is_none() && self.wt_key(c) => {}
                None if self.view == View::Changes && self.changes_key(c) => {}
                None if self.view == View::Conflict && self.conflict_key(c) => {}
                None if self.net_key(c) => {}
                None => return self.later(|token| token.chars().eq([c])),
            },
        }
        true
    }

    /// The graph's own keys that work already.
    fn graph_key(&mut self, c: char) -> bool {
        let g = &mut self.graph;
        match c {
            'b' => self.open_picker(PickKind::Branch),
            'T' => self.open_picker(PickKind::Tag),
            's' => self.open_picker(PickKind::Stash),
            't' => self.ask_revert(),
            'C' => self.ask_cherry(),
            'm' => self.ask_merge_commit(),
            'j' => g.move_by(1),
            'k' => g.move_by(-1),
            'l' => _ = g.open_fold(),
            'h' => _ = g.close_fold(),
            '/' => g.focus_filter = true,
            'z' => self.fold(),
            'o' => self.focus_claude(false),
            'N' => self.open_new_work(),
            _ => return false,
        }
        true
    }

    /// The worktree table's own keys that work already.
    fn wt_key(&mut self, c: char) -> bool {
        match c {
            'j' => self.wt_move(1),
            'k' => self.wt_move(-1),
            'd' => self.open_review(),
            'o' => self.focus_claude(true),
            'c' => self.open_tidy(),
            'm' => self.ask_merge(),
            'x' => self.ask_remove(),
            'p' => self.open_pr(),
            'r' => self.open_reorder(),
            'M' => self.ask_rebase_trunk(),
            'N' => self.open_new_work(),
            _ => return false,
        }
        true
    }

    /// `o`: goes to the tab of the Claude in the window's worktree (`table`:
    /// in the selected row's instead), the one waiting or done first.
    pub(crate) fn focus_claude(&mut self, table: bool) {
        let wt = if table { self.wt_selected().map(|t| t.wt.path.clone()) } else { self.repo_top() };
        let Some(wt) = wt else { return self.say(self.tr("act.no_tree")) };
        let key = paths::key(&wt);
        let sessions = self.claude.iter().flatten().find(|(p, _)| paths::key(p) == key).map(|(_, s)| s);
        let agents = || sessions.into_iter().flatten().filter(|s| s.is_agent());
        let session = agents().find(|s| s.state != State::Running).or_else(|| agents().next()).map(|s| s.id);
        match session {
            Some(session) => self.act(crate::act::Task::Focus { session }),
            None => {
                let text = self.tr("act.no_claude").replace("{wt}", &crate::act::name(&wt));
                self.say_for(text, 5);
            }
        }
    }

    /// `p`: the pull request page of the selected worktree's branch, in the
    /// browser (michi does not write the pull request itself).
    pub(crate) fn open_pr(&mut self) {
        let Some(t) = self.wt_selected() else { return self.say(self.tr("act.no_tree")) };
        let (wt, branch) = (t.wt.path.clone(), t.wt.branch.clone());
        match branch {
            Some(branch) => self.act(crate::act::Task::OpenPr { wt, branch }),
            None => self.say(self.tr("act.no_branch")),
        }
    }

    /// The window's own worktree, once the repository is read.
    pub(crate) fn repo_top(&self) -> Option<PathBuf> {
        match &self.repo {
            Repo::Ready(s) => Some(s.top.clone()),
            _ => None,
        }
    }

    /// `z`, or the header's toggle: fold the trunk's merges or show them all.
    pub fn fold(&mut self) {
        let text = match self.graph.toggle_fold() {
            Folded::On { merges, commits } => self
                .tr("fold.on")
                .replace("{trunk}", self.graph.trunk_name().unwrap_or("main"))
                .replace("{n}", &merges.to_string())
                .replace("{m}", &commits.to_string()),
            Folded::Nothing => self.tr("fold.nothing").to_owned(),
            Folded::Off => self.tr("fold.off").to_owned(),
        };
        self.say(text);
    }

    /// A key with a name (`Enter`, `ArrowDown`), outside the help and the
    /// settings.
    pub fn named(&mut self, name: &str) -> bool {
        if self.mode == Mode::Help {
            return self.help_named(name);
        }
        if self.mode != Mode::Main {
            return false;
        }
        if self.tidy.is_some() {
            return self.tidy_named(name);
        }
        if self.reorder.is_some() {
            return self.reorder_named(name);
        }
        if self.view == View::Graph {
            let g = &mut self.graph;
            match name {
                "ArrowDown" => g.move_by(1),
                "ArrowUp" => g.move_by(-1),
                "PageDown" => g.page_by(false),
                "PageUp" => g.page_by(true),
                "Home" => g.home(),
                "End" => g.end(),
                "ArrowRight" => _ = g.open_fold(),
                "ArrowLeft" => _ = g.close_fold(),
                "Enter" => self.detail.wide = !self.detail.wide,
                _ => return self.later(|token| token == name),
            }
            return true;
        }
        if self.view == View::Worktree && self.review.is_some() {
            return self.review_named(name) || self.later(|token| token == name);
        }
        if self.view == View::Changes {
            return self.changes_named(name) || self.later(|token| token == name);
        }
        if self.view == View::Conflict {
            return self.conflict_named(name) || self.later(|token| token == name);
        }
        if self.view == View::Worktree {
            let n = self.trees.trees.len() as i32;
            match name {
                "ArrowDown" => self.wt_move(1),
                "ArrowUp" => self.wt_move(-1),
                "PageDown" => self.wt_move(10),
                "PageUp" => self.wt_move(-10),
                "Home" => self.wt_move(-n),
                "End" => self.wt_move(n),
                "Enter" => self.wt_in_graph(),
                _ => return self.later(|token| token == name),
            }
            return true;
        }
        self.later(|token| token == name)
    }

    /// A button whose work comes in a later stage: say so.
    pub fn later_word(&mut self, word: &'static str) {
        let text = self.tr("later").replace("{}", self.tr(word));
        self.say(text);
    }

    /// A key of the view or of every view whose work comes in a later stage:
    /// say so. `Space` is the space character.
    fn later(&mut self, is: impl Fn(&str) -> bool) -> bool {
        let found = self.keys().iter().chain(GLOBAL_KEYS.iter()).find(|(keys, _)| {
            keys.split(' ').any(|token| {
                let token = if token == "Space" { " " } else { token };
                is(token)
            })
        });
        let Some((_, word)) = found else { return false };
        let text = self.tr("later").replace("{}", self.tr(word));
        self.say(text);
        true
    }

    /// Escape: close the help or the settings, else the branch palette,
    /// else drop the job waiting for a yes, else close the clean-up list,
    /// else the stash the side column
    /// shows, else bring the graph back from beside a wide diff, else
    /// clear the graph's filter; over the worktree table, bring the review's
    /// diff back to one column, else close the review. With nothing else to
    /// close, stop the fetch, pull or push that runs.
    pub fn escape(&mut self) -> bool {
        if self.mode != Mode::Main {
            self.mode = Mode::Main;
            return true;
        }
        if self.picker.take().is_some() || self.new_work.take().is_some() {
            return true;
        }
        if self.band.take().is_some() {
            return true;
        }
        if self.reorder.take().is_some() {
            return true;
        }
        if self.tidy.take().is_some() {
            return true;
        }
        if self.view == View::Graph && self.picker.is_none() && self.stash_shown().is_some() {
            self.side_stash = None;
            return true;
        }
        if self.view == View::Graph && self.detail.wide {
            self.detail.wide = false;
            return true;
        }
        if self.view == View::Graph && !self.graph.filter().is_empty() {
            self.graph.set_filter("");
            return true;
        }
        if self.view == View::Changes {
            let ch = &mut self.changes;
            if ch.confirm.take().is_some() || ch.told.take().is_some() {
                return true;
            }
            if ch.focus == Focus::Lines {
                ch.leave_lines();
                return true;
            }
            if ch.wide {
                ch.wide = false;
                return true;
            }
        }
        if self.view == View::Conflict && self.conflicts.abort {
            self.conflicts.abort = false;
            return true;
        }
        if self.view == View::Worktree {
            if let Some(r) = self.review.as_mut().filter(|r| r.wide) {
                r.wide = false;
                return true;
            }
            if self.review.is_some() {
                self.close_review();
                return true;
            }
        }
        self.net_stop()
    }

    /// Whether the bar's tab for `v` is the one in force: the view's, under
    /// the key list too, which is drawn over the view.
    pub fn is_on(&self, v: View) -> bool {
        self.mode != Mode::Settings && self.view == v
    }
}

/// The word-table key saying why the repository could not be read.
pub fn failed_word(e: &git::Error) -> &'static str {
    match e {
        git::Error::NoGit => "repo.no_git",
        git::Error::NotRepo => "repo.not_repo",
        git::Error::Git(_) | git::Error::Io(_) | git::Error::Told { .. } => "repo.failed",
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::graph::fixture::{self, FEAT};
    use michi_core::claude::State;
    use michi_core::summary::Head;

    pub(crate) fn app() -> App {
        let mut a = App::new(Prefs::empty(), PathBuf::from("C:/dev/x"));
        a.lang = "en".into();
        a
    }

    pub(crate) fn toast(a: &App) -> &str {
        a.toast.as_ref().map_or("", |(t, _)| t)
    }

    /// The app with graph.rs's small history read in, as the reading
    /// thread sends it.
    pub(crate) fn read() -> App {
        let mut a = app();
        a.take(Answer::Setup { gen: 1, setup: Box::new(fixture::setup()) });
        a.take(Answer::Commits { gen: 1, batch: fixture::history(), done: true });
        a
    }

    fn selected(a: &App) -> &str {
        let g = &a.graph;
        g.selected_row().and_then(|k| g.commit(k)).map_or("", |c| c.subject.as_str())
    }

    #[test]
    fn digits_open_the_views() {
        let mut a = app();
        assert_eq!(a.view, View::Graph);
        for (c, v) in [('2', View::Changes), ('3', View::Worktree), ('1', View::Graph)] {
            assert!(a.feed(c));
            assert_eq!(a.view, v);
        }
        assert!(!a.feed('5'));
        assert!(a.toast.is_none());
    }

    #[test]
    fn comma_and_question_mark_open_and_escape_closes() {
        let mut a = app();
        assert!(a.feed(','));
        assert_eq!(a.mode, Mode::Settings);
        // In the settings the screen's own controls take the characters.
        assert!(!a.feed('3'));
        assert_eq!(a.view, View::Graph);
        assert!(a.escape());
        assert_eq!(a.mode, Mode::Main);
        assert!(!a.escape());
        assert!(a.feed('?'));
        assert_eq!(a.mode, Mode::Help);
        assert!(!a.feed('3'), "the help swallows the views' digits");
        assert!(!a.named("Enter"));
        assert!(a.feed('?'));
        assert_eq!(a.mode, Mode::Main);
    }

    #[test]
    fn the_key_list_shows_the_keymap_and_scrolls() {
        use ito_help::Line;
        let mut a = App::new(crate::prefs::Prefs::empty(), std::env::temp_dir());
        let lines = a.help_lines();
        let has =
            |keys: &str, desc: &str| lines.iter().any(|l| matches!(l, Line::Bind { keys: k, desc: d } if k.contains(keys) && d == desc));
        let t = |d: &str| crate::i18n::tr_desc(&a.lang, d);
        assert!(has("<C-q>", &t("Quit")) && has("<C-S-p>", &t("Search commands")), "{lines:?}");
        assert!(has("1 2 3 4", a.tr("key.views")));
        // Nothing has been drawn, so there is nothing to scroll.
        a.mode = Mode::Help;
        assert!(!a.named("ArrowDown"));
        a.help_drawn.set(Some(ito_help::Drawn { visible: 10, max: 30, scroll: 0 }));
        assert!(a.named("PageDown"));
        assert_eq!(a.help_scroll, 9);
        assert!(a.named("End") && a.help_scroll == 30);
        assert!(a.named("Home") && a.help_scroll == 0);
    }

    #[test]
    fn the_tab_in_force_is_the_view_but_not_in_the_settings() {
        let mut a = app();
        assert!(a.is_on(View::Graph));
        a.feed('?');
        assert!(a.is_on(View::Graph), "the key list is over the view");
        a.escape();
        a.feed(',');
        assert!(!a.is_on(View::Graph));
    }

    #[test]
    fn every_view_has_keys_and_words_for_them() {
        for v in View::ALL {
            assert!(!v.keys().is_empty());
            for (_, w) in v.keys() {
                assert_ne!(crate::i18n::tr("en", w), *w, "no words for {w}");
            }
            assert_ne!(crate::i18n::tr("en", v.label()), v.label());
            assert_ne!(crate::i18n::tr("en", v.empty()), v.empty());
        }
        for (_, w) in GLOBAL_KEYS.iter().chain(&REVIEW_KEYS) {
            assert_ne!(crate::i18n::tr("en", w), *w, "no words for {w}");
        }
    }

    #[test]
    fn d_opens_the_branch_review_over_the_worktree_table() {
        use michi_core::review::List;
        use michi_core::show::Changes;
        let mut a = app();
        a.view = View::Worktree;
        let mut trees = fixture::trees();
        trees.trunk_head = None;
        a.take(Answer::Trees(trees));
        a.wt_sel = 1;
        assert!(a.feed('d'));
        assert_eq!(toast(&a), "No trunk to compare with");
        a.take(Answer::Trees(fixture::trees()));
        a.wt_sel = 2;
        assert!(a.feed('d'));
        assert_eq!(toast(&a), "This worktree is on no branch");
        assert!(a.review.is_none());

        a.take(Answer::Wips(vec![fixture::wip("C:/r/wt", Some("wip"), 7, 11, Some(1))]));
        a.wt_sel = 1;
        assert!(a.feed('d'));
        assert_eq!(a.keys(), &REVIEW_KEYS[..]);
        assert!(a.feed('m'), "m works in the review");
        assert!(a.band.is_some() || a.toast.is_some(), "m asks or says why not");
        a.band = None;
        let wip = Ends { trunk: fixture::id(6), head: fixture::id(7), wip: Some(fixture::id(11)) };
        assert_eq!(a.review_ends(), Some(wip), "to the uncommitted changes");
        let gen = |a: &App| a.review.as_ref().unwrap().gens().0;
        let first = gen(&a);
        assert_ne!(first, 0, "the list is asked");
        assert!(a.feed('s'));
        assert_eq!(toast(&a), "Reading the branch…");
        let list = List { base: Some(fixture::id(5)), last: None, changes: Changes::default() };
        a.take(Answer::ReviewList { gen: first, result: Ok(Box::new(list)), checks: Default::default() });
        assert!(a.review.as_ref().unwrap().list.as_ref().is_some_and(|l| l.is_ok()));
        assert!(a.feed('s'));
        assert_eq!(toast(&a), "Not looked over before: all of it is new");
        assert!(!a.review.as_ref().unwrap().since_last);

        // Without the uncommitted changes, and the branch moving on: asked again.
        assert!(a.feed('u'));
        let second = gen(&a);
        assert!(!a.review.as_ref().unwrap().wip && second != first);
        let mut trees = fixture::trees();
        trees.trees[1].wt.head = fixture::id(8).to_string();
        a.take(Answer::Trees(trees.clone()));
        assert_ne!(gen(&a), second);
        // Another branch there: nothing to read to until it comes back.
        let third = gen(&a);
        trees.trees[1].wt.branch = Some("other".into());
        a.take(Answer::Trees(trees));
        assert_eq!((a.review_ends(), gen(&a)), (None, third));

        // Over the worktree table only; the other views keep their keys.
        assert!(a.feed('1'));
        assert_eq!(a.keys(), View::Graph.keys());
        assert!(a.feed('3'));
        assert!(a.review.is_some());
        assert!(!a.feed('q'), "not a key of the review");
        assert!(a.named("Enter"));
        assert!(a.review.as_ref().unwrap().wide);
        assert!(a.escape());
        assert!(a.review.as_ref().is_some_and(|r| !r.wide), "one column first");
        assert!(a.escape());
        assert!(a.review.is_none());
        assert_eq!(a.keys(), View::Worktree.keys());
    }

    #[test]
    fn keys_of_later_stages_say_so() {
        let mut a = app();
        assert!(a.feed('N'));
        assert_eq!(toast(&a), "No worktree to work in", "N needs a repository read");
        a.toast = None;
        assert!(a.feed('f'), "fetch works now");
        assert!(a.toast.is_none() && a.net.is_none(), "no reading thread here: nothing runs");
        assert!(a.feed('s'), "the graph's own key");
        assert_eq!(toast(&a), "No worktree to work in", "the stashes, with nothing read");
        assert!(a.named("Enter"), "the diff side by side, no longer a later stage");
        assert!(a.detail.wide);
        a.feed('2');
        assert!(a.feed('c'));
        assert!(a.changes.focus_message, "c goes to the message");
        assert!(!a.feed('z'), "not a key of the changes");
        assert!(!a.feed('q'));
    }

    #[test]
    fn what_a_worktree_is_doing_sits_in_the_texts() {
        use michi_core::branch::Doing;
        let mut a = app();
        let text = |a: &App, d| a.tr("act.concluded").replace("{doing}", &a.doing_word(d)).replace("{wt}", "r").replace("{new}", "abc1234");
        assert_eq!(text(&a, Doing::Merge), "Committed the merge on r: abc1234");
        a.lang = "ja".into();
        assert_eq!(text(&a, Doing::Merge), "r のマージをコミットしました: abc1234");
        assert_eq!(text(&a, Doing::CherryPick), "r の cherry-pick をコミットしました: abc1234");
    }

    #[test]
    fn the_graph_s_keys_move_the_selection() {
        let mut a = read();
        assert_eq!(selected(&a), "m2");
        assert!(a.feed('j'));
        assert_eq!(selected(&a), "w1");
        assert!(a.named("ArrowDown"));
        assert_eq!(selected(&a), FEAT);
        assert!(a.feed('k'));
        assert_eq!(selected(&a), "w1");
        assert!(a.named("End"));
        assert_eq!(selected(&a), "first");
        assert!(a.named("PageUp"));
        assert_eq!(selected(&a), "m2");
        assert!(a.named("PageDown"));
        assert_eq!(selected(&a), "first");
        assert!(a.named("Home"));
        assert_eq!(selected(&a), "m2");
        assert!(a.toast.is_none(), "nothing to say");
    }

    #[test]
    fn l_h_and_the_arrows_open_and_fold_a_merge() {
        let mut a = read();
        a.feed('j');
        a.feed('j');
        assert_eq!(selected(&a), FEAT);
        assert!(a.named("ArrowRight"));
        assert_eq!(a.graph.len(), 7);
        a.feed('j');
        a.feed('j');
        assert_eq!(selected(&a), "f2");
        assert!(a.named("ArrowLeft"));
        assert_eq!((a.graph.len(), selected(&a)), (5, FEAT), "the merge takes the selection");
        assert!(a.feed('l'));
        assert_eq!(a.graph.len(), 7);
        assert!(a.feed('h'));
        assert_eq!(a.graph.len(), 5);
    }

    #[test]
    fn z_folds_and_says_so() {
        let mut a = read();
        assert!(a.feed('z'));
        assert_eq!(toast(&a), "Unfolded all the merges.");
        assert_eq!(a.graph.len(), 7);
        assert!(a.feed('z'));
        assert_eq!(toast(&a), "Folded the merges into main: 1 (2 commits).");
        assert_eq!(a.graph.len(), 5);
        a.lang = "ja".into();
        a.feed('z');
        a.feed('z');
        assert!(toast(&a).contains("main"), "{}", toast(&a));

        let mut b = app();
        b.feed('z');
        b.feed('z');
        assert_eq!(toast(&b), "No merges to fold yet.");
    }

    #[test]
    fn slash_asks_for_the_filter_and_escape_clears_it() {
        let mut a = read();
        assert!(a.feed('/'));
        assert!(a.graph.focus_filter);
        a.graph.set_filter("f2");
        assert_eq!(a.graph.hit_count(), Some(1));
        assert!(a.escape());
        assert!(a.graph.filter().is_empty());
        assert!(!a.escape(), "nothing left to clear");
        a.graph.set_filter("f2");
        a.feed('2');
        assert!(!a.escape(), "the filter is the graph's");
    }

    #[test]
    fn answers_of_an_older_reading_are_dropped() {
        let mut a = read();
        let rows = a.graph.len();
        a.take(Answer::Setup { gen: 2, setup: Box::new(fixture::setup()) });
        a.take(Answer::Commits { gen: 1, batch: fixture::history()[..1].to_vec(), done: true });
        assert_eq!(a.graph.len(), rows, "the first reading's commits come too late");
        a.take(Answer::Failed { gen: 2, error: git::Error::Git("bad".into()) });
        assert_eq!(toast(&a), "Could not read the history: bad");
    }

    #[test]
    fn an_old_git_is_said_and_kept() {
        let mut a = app();
        a.take(Answer::Version(Ok(Version(2, 30, 1))));
        assert_eq!(a.old_git(), Some(Version(2, 30, 1)));
        assert!(toast(&a).starts_with("git 2.30.1 is older than 2.38.0"), "{}", toast(&a));
        let mut b = app();
        b.take(Answer::Version(Ok(Version(2, 52, 0))));
        assert_eq!(b.old_git(), None);
        assert!(b.toast.is_none());
    }

    #[test]
    fn the_summary_names_the_window_or_says_why_not() {
        let mut a = app();
        let s = Summary { name: "michi".into(), head: Head::Branch("main".into()), worktrees: 2, top: PathBuf::from("C:/dev/x") };
        a.take(Answer::Summary(Ok(s.clone())));
        assert_eq!(a.new_title.as_deref(), Some("michi — michi"));
        assert_eq!(a.repo, Repo::Ready(s));

        let mut b = app();
        b.take(Answer::Summary(Err(git::Error::NotRepo)));
        assert_eq!(b.repo, Repo::Failed(git::Error::NotRepo));
        assert_eq!(toast(&b), "Not a Git repository — C:/dev/x");
        assert!(b.new_title.is_none());
        b.take(Answer::Summary(Err(git::Error::Git("bad object".into()))));
        assert_eq!(toast(&b), "git failed: bad object — C:/dev/x");
    }

    #[test]
    fn the_detail_follows_the_selection() {
        use crate::detail::fixture::detail;
        let mut a = read();
        assert!(a.sync_detail(), "the first selection is asked for");
        let m2 = a.graph.selected().unwrap();
        assert_eq!(a.detail.asked(), Some(m2));
        assert!(!a.sync_detail());
        assert!(a.detail.got().is_none());
        // An answer for a commit no longer selected is dropped.
        a.take(Answer::Detail { id: fixture::id(1), result: Ok(Box::new(detail(fixture::id(1), vec![], 1))) });
        assert!(a.detail.got().is_none());
        a.take(Answer::Detail { id: m2, result: Ok(Box::new(detail(m2, vec![fixture::id(5)], 2))) });
        assert!(a.detail.rows(1).is_some());
        a.named("ArrowDown");
        assert!(a.sync_detail());
        assert!(a.detail.got().is_none() && a.detail.to_top);
    }

    #[test]
    fn a_commit_picked_is_seen() {
        let mut a = read();
        a.take(Answer::Unseen(fixture::unseen()));
        a.sync_detail();
        assert_eq!(a.unseen.ids().len(), 1, "the first selection is no one's pick");
        assert!(a.graph.select_id(&fixture::id(4)));
        a.sync_detail();
        assert_eq!(a.unseen.ids().len(), 1, "another commit");
        a.go_to(fixture::id(7));
        a.sync_detail();
        assert!(a.unseen.ids().is_empty());
        assert_eq!(a.unseen.of(std::path::Path::new("C:/r/wt")), Some(&[][..]), "the worktree is still followed");
    }

    #[test]
    fn a_trunk_merge_counts_its_commits() {
        let mut a = read();
        assert!(a.graph.select_id(&fixture::id(5)));
        a.sync_detail();
        let read = a.graph.read();
        assert_eq!(a.detail.merged(fixture::id(5), read), Some(&[fixture::id(3), fixture::id(2)][..]));
        assert_eq!(selected(&a), FEAT);
    }

    #[test]
    fn enter_spreads_the_diff_and_escape_takes_it_back_first() {
        let mut a = read();
        a.graph.set_filter("f");
        assert!(a.named("Enter"));
        assert!(a.detail.wide);
        assert!(a.escape());
        assert!(!a.detail.wide);
        assert_eq!(a.graph.filter(), "f", "the filter goes on the next Escape");
        assert!(a.escape());
        assert_eq!(a.graph.filter(), "");
        assert!(!a.escape());
    }

    #[test]
    fn the_side_column_goes_to_a_commit_or_says_it_is_not_there() {
        let mut a = read();
        a.go_to(fixture::id(3));
        assert_eq!(selected(&a), "f2");
        assert!(a.seeking.is_none());
        a.go_to(fixture::id(9));
        assert!(a.seeking.is_none(), "all is read: nothing more to read on to");
        assert!(toast(&a).starts_with("Not in the history: "), "{}", toast(&a));
        assert_eq!(selected(&a), "f2");
    }

    #[test]
    fn a_wip_read_anew_keeps_the_detail_where_it_was() {
        let mut a = read();
        a.take(Answer::Wips(vec![fixture::wip("C:/r/repo", Some("main"), 6, 11, Some(1))]));
        a.graph.select(0);
        assert!(a.sync_detail());
        assert_eq!(a.detail.asked(), Some(fixture::id(11)));
        let shown = crate::detail::fixture::detail(fixture::id(11), vec![fixture::id(6)], 2);
        a.detail.take(fixture::id(11), Ok(Box::new(shown)));
        a.detail.to_top = false;
        a.detail.scroll_to = Some(1);
        // Written again: the new change is asked for, the old one stays shown.
        a.take(Answer::Wips(vec![fixture::wip("C:/r/repo", Some("main"), 6, 12, Some(2))]));
        assert!(a.sync_detail());
        assert_eq!(a.detail.asked(), Some(fixture::id(12)));
        assert!(a.detail.got().is_some());
        assert_eq!((a.detail.to_top, a.detail.scroll_to), (false, Some(1)));
        assert!(!a.sync_detail());
        // Another row: from the top.
        a.graph.select(1);
        assert!(a.sync_detail());
        assert!(a.detail.got().is_none() && a.detail.to_top);
    }

    #[test]
    fn a_handed_claude_is_announced_once_it_stops() {
        let mut a = app();
        let wt = PathBuf::from("C:/r/wt");
        let s = |state| Session {
            id: 3,
            state,
            command: "claude".into(),
            agent: "claude".into(),
            cwd: "C:/r/wt".into(),
            branch: "wip".into(),
            title: "fix".into(),
            note: String::new(),
            since_ms: 0,
            read_only: false,
        };
        let give = |a: &mut App, st| a.take(Answer::Claude(Some(vec![(PathBuf::from("C:/r/wt"), vec![s(st)])])));
        a.watch_handed(wt);
        give(&mut a, State::Running);
        assert!(a.toast.is_none());
        give(&mut a, State::Running);
        assert!(a.toast.is_none() && a.handed.len() == 1);
        give(&mut a, State::Waiting);
        assert!(toast(&a).contains("wt"), "{}", toast(&a));
        assert!(a.handed.is_empty());
    }

    #[test]
    fn claude_sessions_are_looked_up_by_worktree() {
        let mut a = app();
        let s = Session {
            id: 3,
            state: State::Waiting,
            command: "claude".into(),
            agent: "claude".into(),
            cwd: "C:/r/wt/src".into(),
            branch: "wip".into(),
            title: "fix".into(),
            note: "Allow?".into(),
            since_ms: 0,
            read_only: false,
        };
        a.take(Answer::Claude(Some(vec![(PathBuf::from("C:/r/wt"), vec![s.clone()])])));
        assert_eq!(a.claude_in(Path::new("C:/r/wt")), [s]);
        assert!(a.claude_in(Path::new("C:/r/repo")).is_empty());
        a.take(Answer::Claude(None));
        assert!(a.claude_in(Path::new("C:/r/wt")).is_empty());
    }

    #[test]
    fn o_without_a_claude_in_the_worktree_says_so() {
        let mut a = app();
        a.view = View::Worktree;
        a.take(Answer::Trees(fixture::trees()));
        a.wt_sel = 1;
        assert!(a.feed('o'));
        assert!(toast(&a).starts_with("No Claude is running in"), "{}", toast(&a));
    }

    fn wt_path(a: &App) -> &str {
        a.wt_selected().and_then(|t| t.wt.path.to_str()).unwrap_or("")
    }

    #[test]
    fn the_worktree_table_s_keys_move_the_selection() {
        let mut a = read();
        a.take(Answer::Trees(fixture::trees()));
        a.feed('3');
        assert_eq!(wt_path(&a), "C:/r/repo");
        assert!(a.feed('j'));
        assert_eq!(wt_path(&a), "C:/r/wt");
        assert!(a.named("ArrowDown"));
        assert!(a.named("ArrowDown"), "the last row stays");
        assert_eq!(wt_path(&a), "C:/r/old");
        assert!(a.feed('k'));
        assert!(a.named("ArrowUp"));
        assert_eq!(wt_path(&a), "C:/r/repo");
        assert!(a.named("End"));
        assert_eq!(wt_path(&a), "C:/r/old");
        assert!(a.named("PageUp"));
        assert_eq!(wt_path(&a), "C:/r/repo");
        assert!(a.named("PageDown"));
        assert!(a.named("Home"));
        assert_eq!(wt_path(&a), "C:/r/repo");
        assert!(a.toast.is_none(), "nothing to say");
        // m takes the trunk in, here with nothing to take; the keys of a
        // later stage say so.
        assert!(a.feed('m'));
        assert_eq!(toast(&a), "repo is on main itself: there is nothing to merge");
        a.later_word("wt.pr");
        assert!(toast(&a).starts_with("Open a PR"), "{}", toast(&a));
    }

    #[test]
    fn the_worktree_selection_stays_on_its_worktree() {
        let mut a = app();
        a.take(Answer::Trees(fixture::trees()));
        a.wt_move(1);
        assert_eq!(wt_path(&a), "C:/r/wt");
        // The first worktree is gone: the selection stays on C:/r/wt.
        let mut t = fixture::trees();
        t.trees.remove(0);
        a.take(Answer::Trees(t.clone()));
        assert_eq!((a.wt_sel, wt_path(&a)), (0, "C:/r/wt"));
        // Its worktree gone, the selection goes to the first.
        a.wt_move(1);
        t.trees.remove(1);
        t.trees.remove(0);
        a.take(Answer::Trees(t));
        assert_eq!(a.wt_sel, 0);
        a.take(Answer::Trees(Trees::default()));
        assert_eq!(a.wt_sel, 0);
        a.wt_move(3);
        a.wt_in_graph();
        assert_eq!(a.view, View::Graph, "nothing to open, nothing done");
    }

    #[test]
    fn a_wip_with_only_folders_left_out_stays_out_of_the_graph() {
        let mut a = read();
        let mut only = fixture::wip("C:/r/repo", Some("main"), 6, 6, None);
        only.files.clear();
        only.left_out = vec![michi_core::predict::LeftOut { path: "target-v/".into(), files: 35584, bytes: 20 << 30 }];
        a.take(Answer::Wips(vec![only, fixture::wip("C:/r/wt", Some("wip"), 7, 11, Some(1))]));
        assert_eq!(a.graph.len(), 6, "one WIP row");
        assert!(a.wip_in(Path::new("C:/r/repo")).is_some_and(|w| w.files.is_empty() && w.left_out.len() == 1));
        assert!(a.wip_in(Path::new("C:/r/wt")).is_some());
        assert!(a.wip_in(Path::new("C:/r/old")).is_none());
    }

    #[test]
    fn enter_opens_the_graph_on_the_worktree() {
        let mut a = read();
        a.take(Answer::Wips(vec![fixture::wip("C:/r/wt", Some("wip"), 7, 11, Some(1))]));
        a.take(Answer::Trees(fixture::trees()));
        a.feed('3');
        a.feed('j');
        a.detail.wide = true;
        assert!(a.named("Enter"));
        assert_eq!(a.view, View::Graph);
        assert!(!a.detail.wide);
        assert_eq!(a.graph.selected(), Some(fixture::id(11)), "its uncommitted change");
        // No change: its commit.
        a.feed('3');
        a.named("Home");
        a.named("Enter");
        assert_eq!(selected(&a), "m2");
        a.feed('3');
        a.named("End");
        a.named("Enter");
        assert_eq!(selected(&a), "m1");
    }
}
