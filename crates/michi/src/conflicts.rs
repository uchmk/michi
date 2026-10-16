//! The `4` view's state: the worktrees stopped in the middle of a merge, a
//! cherry-pick, a revert or a rebase, the place selected, and what its keys
//! do. A pick goes to the act thread and the places are read again after
//! it; until then the place waits, and once solved it shows what it was
//! solved with.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use michi_core::branch::Doing;
use michi_core::conflict::{Body, Conflict, Intent, Pick};
use michi_core::paths;

use crate::act::{self, Task};
use crate::app::{App, Repo, View};
use crate::work::{Band, Then};

/// A place of a conflict: its file's index, and its block's (`None`: the
/// file clashed whole).
pub type Place = (usize, Option<usize>);

/// A place by the worktree's key, the file's path and the block.
type Key = (String, String, Option<usize>);

/// What a place was solved with here, for its tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tag {
    Picked(Pick),
    /// The file taken whole: our side's (`true`) or theirs.
    Took(bool),
    /// The file git's rerere solved on its own, not looked at yet.
    Reused,
}

/// The places of `c`, in order: each block of a file merged line by line,
/// one for a file that clashed whole.
pub fn places(c: &Conflict) -> Vec<Place> {
    let mut all = Vec::new();
    for (i, f) in c.files.iter().enumerate() {
        match &f.body {
            Body::Text(blocks) => all.extend((0..blocks.len()).map(|j| (i, Some(j)))),
            Body::Whole(_) => all.push((i, None)),
        }
    }
    all
}

/// Whether the place is still to be solved.
pub fn is_open(c: &Conflict, (i, j): Place) -> bool {
    let Some(f) = c.files.get(i) else { return false };
    match (&f.body, j) {
        (Body::Text(blocks), Some(j)) => blocks.get(j).is_some_and(|b| b.is_open()),
        (Body::Whole(_), None) => f.unmerged,
        _ => false,
    }
}

#[derive(Default)]
pub struct Conflicts {
    /// Each worktree stopped in the middle of something, as last read.
    pub all: Vec<Conflict>,
    /// The worktree to show; the first is shown while it is not stopped
    /// (not yet, or not any more).
    pub wt: Option<PathBuf>,
    /// The selected place, an index into [`places`] of the one shown.
    pub at: usize,
    tags: HashMap<Key, Tag>,
    /// The files rerere solved that were looked at (the worktree's key and
    /// the path).
    confirmed: HashSet<(String, String)>,
    /// What each side meant by a place, once read; and the places asked for.
    intents: HashMap<Key, Intent>,
    asked: HashSet<Key>,
    /// Places picked here whose file is not read again yet.
    pending: HashSet<Key>,
    /// The places whose base column is shown or hidden by hand; the rest
    /// show it when the view is wide enough.
    pub bases: HashMap<Key, bool>,
    /// The row that asks before the whole thing is called off is up.
    pub abort: bool,
    /// The commit's message, once all is solved.
    pub message: String,
    /// The worktree the message was filled in for.
    message_for: Option<PathBuf>,
    /// Bring the selected place in sight on the next frame.
    pub follow: bool,
    /// Pages to scroll by on the next frame (PageDown: +1).
    pub pages: i32,
    /// Whether the message had the keys on the last frame.
    pub typing: bool,
    /// Give the message the keys on the next frame.
    pub focus_message: bool,
}

impl Conflicts {
    /// The worktrees as read again: what was picked is not waited for any
    /// more, and the tags of worktrees no longer stopped go. When the last
    /// place of the one shown was solved, the message comes in sight.
    pub fn set(&mut self, all: Vec<Conflict>) {
        self.update(all, |_| true);
    }

    /// The worktree at `wt` read again on its own, after a pick there
    /// (`None`: it is not stopped any more); the others stay as they were.
    pub fn set_one(&mut self, wt: &Path, c: Option<Conflict>) {
        let key = paths::key(wt);
        let mut all = self.all.clone();
        match (all.iter().position(|o| paths::key(&o.wt) == key), c) {
            (Some(i), Some(c)) => all[i] = c,
            (Some(i), None) => {
                all.remove(i);
            }
            (None, Some(c)) => all.push(c),
            (None, None) => {}
        }
        self.update(all, |k| k == key);
    }

    /// The worktrees now `all`, those whose key is `fresh` read since their
    /// picks were sent.
    fn update(&mut self, all: Vec<Conflict>, fresh: impl Fn(&str) -> bool) {
        let was = self.shown().filter(|c| c.left() > 0).map(|c| paths::key(&c.wt));
        let keys: HashSet<String> = all.iter().map(|c| paths::key(&c.wt)).collect();
        self.tags.retain(|(k, ..), _| keys.contains(k));
        self.confirmed.retain(|(k, _)| keys.contains(k));
        self.intents.retain(|(k, ..), _| keys.contains(k));
        self.asked.retain(|(k, ..)| keys.contains(k));
        self.bases.retain(|(k, ..), _| keys.contains(k));
        self.pending.retain(|(k, ..)| !fresh(k));
        self.all = all;
        self.settle();
        if self.shown().is_some_and(|c| c.left() == 0 && was.as_deref() == Some(paths::key(&c.wt).as_str())) {
            self.follow = true;
        }
    }

    /// Keeps the selection in the places, and fills the message in for a
    /// worktree newly shown.
    fn settle(&mut self) {
        let Some(c) = self.shown() else {
            self.at = 0;
            self.abort = false;
            self.message_for = None;
            return;
        };
        let (wt, message) = (c.wt.clone(), c.message.trim_end().to_owned());
        let n = places(c).len();
        let new = self.message_for.as_deref().is_none_or(|m| paths::key(m) != paths::key(&wt));
        if new {
            self.message_for = Some(wt);
            self.message = message;
            self.abort = false;
            self.at = 0;
            if self.shown().is_some_and(|c| !places(c).first().is_some_and(|p| is_open(c, *p))) {
                self.next_open();
            }
            self.follow = true;
        }
        self.at = self.at.min(n.saturating_sub(1));
    }

    /// The conflict the view shows.
    pub fn shown(&self) -> Option<&Conflict> {
        let key = self.wt.as_deref().map(paths::key);
        key.and_then(|k| self.all.iter().find(|c| paths::key(&c.wt) == k)).or_else(|| self.all.first())
    }

    /// Shows the worktree at `wt`, if it is stopped. Whether it is.
    pub fn show(&mut self, wt: &Path) -> bool {
        let key = paths::key(wt);
        if !self.all.iter().any(|c| paths::key(&c.wt) == key) {
            return false;
        }
        self.wt = Some(wt.to_path_buf());
        self.settle();
        true
    }

    /// The conflict of the worktree at `wt`, if it is stopped.
    pub fn of(&self, wt: &Path) -> Option<&Conflict> {
        let key = paths::key(wt);
        self.all.iter().find(|c| paths::key(&c.wt) == key)
    }

    /// The places of the one shown.
    pub fn places(&self) -> Vec<Place> {
        self.shown().map(places).unwrap_or_default()
    }

    /// `]` `[`: the next or the one before, round the end.
    pub fn move_by(&mut self, by: i32) {
        let n = self.places().len() as i64;
        if n == 0 {
            return;
        }
        self.at = (self.at as i64 + i64::from(by)).rem_euclid(n) as usize;
        self.follow = true;
    }

    /// Home, End.
    pub fn move_to(&mut self, last: bool) {
        self.at = if last { self.places().len().saturating_sub(1) } else { 0 };
        self.follow = true;
    }

    /// Selects the next place still to be solved and not waited for, round
    /// the end; stays when there is none.
    pub fn next_open(&mut self) {
        let Some(c) = self.shown() else { return };
        let all = places(c);
        let n = all.len();
        let wt = paths::key(&c.wt);
        let next = (1..=n).map(|k| (self.at + k) % n).find(|&i| {
            let p = all[i];
            is_open(c, p) && !self.pending.contains(&(wt.clone(), c.files[p.0].path.clone(), p.1))
        });
        if let Some(i) = next {
            self.at = i;
        }
        self.follow = true;
    }

    /// The places still to be solved, in every worktree: for the tab.
    pub fn left(&self) -> usize {
        self.all.iter().map(Conflict::left).sum()
    }

    /// The key of the place `p` of the one shown.
    pub fn key(&self, p: Place) -> Option<Key> {
        let c = self.shown()?;
        Some((paths::key(&c.wt), c.files.get(p.0)?.path.clone(), p.1))
    }

    /// What the place was solved with here; `None` when it was not (by hand,
    /// or by someone else).
    pub fn tag(&self, p: Place) -> Option<Tag> {
        let key = self.key(p)?;
        if let Some(t) = self.tags.get(&key) {
            return Some(*t);
        }
        self.reused(p.0).then_some(Tag::Reused)
    }

    /// Whether the file `i` of the one shown looks solved by rerere and was
    /// not looked at: still unmerged, no place open, and nothing picked in it
    /// here. (A file solved by hand in an editor looks the same.)
    pub fn reused(&self, i: usize) -> bool {
        let Some(c) = self.shown() else { return false };
        let Some(f) = c.files.get(i) else { return false };
        let Body::Text(blocks) = &f.body else { return false };
        let wt = paths::key(&c.wt);
        f.unmerged
            && !blocks.is_empty()
            && blocks.iter().all(|b| !b.is_open())
            && !self.confirmed.contains(&(wt.clone(), f.path.clone()))
            && !self.tags.keys().any(|(k, p, _)| *k == wt && *p == f.path)
    }

    /// How many files of the one shown rerere solved that are not looked at.
    pub fn reused_left(&self) -> usize {
        self.shown().map_or(0, |c| (0..c.files.len()).filter(|&i| self.reused(i)).count())
    }

    /// The file `i` of the one shown, looked at. Whether it was one to look at.
    pub fn confirm(&mut self, i: usize) -> bool {
        if !self.reused(i) {
            return false;
        }
        let Some(c) = self.shown() else { return false };
        let key = (paths::key(&c.wt), c.files[i].path.clone());
        self.confirmed.insert(key);
        true
    }

    /// What each side meant by the place, once read.
    pub fn intent(&self, p: Place) -> Option<&Intent> {
        self.intents.get(&self.key(p)?)
    }

    /// The intent of the place `path`/`j` of the worktree at `wt` was read.
    pub fn set_intent(&mut self, wt: &Path, path: &str, j: usize, i: Intent) {
        self.intents.insert((paths::key(wt), path.to_owned(), Some(j)), i);
    }

    /// Whether the place was picked and is not read again yet.
    pub fn waits(&self, p: Place) -> bool {
        self.key(p).is_some_and(|k| self.pending.contains(&k))
    }

    /// Whether the place's base column shows, the view being `wide` enough
    /// for it by itself.
    pub fn base_shown(&self, p: Place, wide: bool) -> bool {
        self.key(p).and_then(|k| self.bases.get(&k).copied()).unwrap_or(wide)
    }

    /// The button that shows or hides the place's base.
    pub fn toggle_base(&mut self, p: Place, wide: bool) {
        let shown = self.base_shown(p, wide);
        if let Some(k) = self.key(p) {
            self.bases.insert(k, !shown);
        }
    }
}

impl App {
    /// `4`: the conflict of the worktree in sight (the table's row, the `2`
    /// view's, the window's own), else the one shown last. With none,
    /// says so.
    pub(crate) fn open_conflicts(&mut self) {
        if self.conflicts.all.is_empty() {
            self.say(self.tr("cf.none"));
            return;
        }
        let near = match self.view {
            View::Worktree => self.wt_selected().map(|t| t.wt.path.clone()),
            View::Changes => self.changes.wt.clone(),
            View::Graph | View::Conflict => None,
        };
        let top = match &self.repo {
            Repo::Ready(s) => Some(s.top.clone()),
            _ => None,
        };
        if self.view != View::Conflict {
            if let Some(wt) = near.into_iter().chain(top).find(|wt| self.conflicts.of(wt).is_some()) {
                self.conflicts.show(&wt);
            }
        }
        self.view = View::Conflict;
        self.conflicts.follow = true;
    }

    /// The `4` view's own keys that work already.
    pub(crate) fn conflict_key(&mut self, c: char) -> bool {
        if self.conflicts.abort {
            match c {
                'y' => self.conflict_abort(),
                'n' => self.conflicts.abort = false,
                _ => {}
            }
            return true;
        }
        match c {
            ']' => self.conflicts.move_by(1),
            '[' => self.conflicts.move_by(-1),
            'o' => self.conflict_pick(Pick::Ours),
            't' => self.conflict_pick(Pick::Theirs),
            'b' => self.conflict_pick(Pick::Both),
            'B' => self.conflict_pick(Pick::BothTheirsFirst),
            'x' => self.conflict_forget(),
            _ => return false,
        }
        true
    }

    /// The `4` view's keys with a name.
    pub(crate) fn conflict_named(&mut self, name: &str) -> bool {
        let cf = &mut self.conflicts;
        match name {
            "ArrowDown" => cf.move_by(1),
            "ArrowUp" => cf.move_by(-1),
            "Home" => cf.move_to(false),
            "End" => cf.move_to(true),
            "Enter" => return self.conflict_confirm(),
            "PageDown" => cf.pages += 1,
            "PageUp" => cf.pages -= 1,
            _ => return false,
        }
        true
    }

    /// Solves the selected place with `pick`, then selects the next one to
    /// solve.
    pub(crate) fn conflict_pick(&mut self, pick: Pick) {
        let at = self.conflicts.at;
        self.conflict_pick_at(at, pick);
    }

    /// Solves the place `at` (an index into the places) with `pick`.
    pub(crate) fn conflict_pick_at(&mut self, at: usize, pick: Pick) {
        let cf = &self.conflicts;
        let Some(c) = cf.shown() else { return };
        let Some(&p) = places(c).get(at) else { return };
        if !is_open(c, p) || cf.waits(p) {
            self.say(self.tr("cf.solved"));
            return;
        }
        let (wt, file) = (c.wt.clone(), &c.files[p.0]);
        let path = file.path.clone();
        let (task, tag) = match (&file.body, p.1) {
            (Body::Text(blocks), Some(j)) => {
                let block = &blocks[j];
                if pick == Pick::Base && block.base.is_none() {
                    self.say(self.tr("cf.no_base"));
                    return;
                }
                (Task::Resolve { wt, path, block: Box::new(block.clone()), pick }, Tag::Picked(pick))
            }
            (Body::Whole(_), _) => match pick {
                Pick::Ours | Pick::Theirs => {
                    let ours = pick == Pick::Ours;
                    (Task::Take { wt, path, ours }, Tag::Took(ours))
                }
                _ => {
                    self.say(self.tr("cf.whole"));
                    return;
                }
            },
            (Body::Text(_), None) => return,
        };
        let Some(key) = self.conflicts.key(p) else { return };
        self.conflicts.pending.insert(key.clone());
        self.conflicts.tags.insert(key, tag);
        self.conflicts.at = at;
        self.conflicts.next_open();
        self.act(task);
    }

    /// The pick of `path` in the worktree at `wt` failed: it is open again.
    pub(crate) fn conflict_unpick(&mut self, wt: &Path, path: &str) {
        let key = paths::key(wt);
        let mine = |(k, p, _): &Key| *k == key && p == path;
        self.conflicts.pending.retain(|k| !mine(k));
        self.conflicts.intents.retain(|k, _| !mine(k));
        self.conflicts.asked.retain(|k| !mine(k));
        self.conflicts.tags.retain(|k, _| !mine(k));
    }

    /// Ctrl+Enter in the `4` view: commit what was solved, when all is.
    pub(crate) fn conflict_commit(&mut self) {
        let Some(c) = self.conflicts.shown() else { return };
        let (wt, doing, left) = (c.wt.clone(), c.doing, c.left());
        let message = self.conflicts.message.trim().to_owned();
        if left > 0 {
            let text = self.tr("cf.left").replace("{}", &left.to_string());
            self.say(text);
            return;
        }
        if message.is_empty() && doing != Doing::Rebase {
            self.say(self.tr("cf.no_message"));
            self.conflicts.focus_message = true;
            return;
        }
        let task = Task::Conclude { wt: wt.clone(), doing, message };
        if self.claude_working(&wt) {
            let text = self.tr("cf.claude_commit").replace("{wt}", &act::name(&wt)).replace("{doing}", &self.doing_word(doing));
            self.band = Some(Band::new(text, Then::Act(task)));
            return;
        }
        self.act(task);
    }

    /// The bar's button: ask before the whole thing is called off.
    pub(crate) fn conflict_ask_abort(&mut self) {
        if self.conflicts.shown().is_some() {
            self.conflicts.abort = true;
        }
    }

    /// The row's yes: call the whole thing off.
    pub(crate) fn conflict_abort(&mut self) {
        self.conflicts.abort = false;
        let Some(c) = self.conflicts.shown() else { return };
        let task = Task::Abort { wt: c.wt.clone(), doing: c.doing };
        self.act(task);
    }

    /// Reads what each side meant by the selected place, once.
    pub(crate) fn conflict_want_intent(&mut self) {
        let cf = &self.conflicts;
        let Some(c) = cf.shown() else { return };
        let Some(&p) = places(c).get(cf.at) else { return };
        let (Body::Text(blocks), Some(j)) = (&c.files[p.0].body, p.1) else { return };
        let Some(key) = cf.key(p) else { return };
        if cf.asked.contains(&key) || (c.ours.id.is_none() && c.theirs.id.is_none()) {
            return;
        }
        let task = Task::Intent {
            wt: c.wt.clone(),
            path: c.files[p.0].path.clone(),
            j,
            block: Box::new(blocks[j].clone()),
            conflict: Box::new(c.clone()),
        };
        self.conflicts.asked.insert(key);
        self.act(task);
    }

    /// Enter: the selected place's file, solved by rerere, looked at and
    /// right. Whether it was one.
    pub(crate) fn conflict_confirm(&mut self) -> bool {
        let at = self.conflicts.at;
        let Some(&(i, _)) = self.conflicts.places().get(at) else { return false };
        if !self.conflicts.confirm(i) {
            return false;
        }
        self.conflicts.next_open();
        true
    }

    /// `x`: the selected place's file, which rerere solved, solved again
    /// from the start and not remembered.
    pub(crate) fn conflict_forget(&mut self) {
        let at = self.conflicts.at;
        let Some(c) = self.conflicts.shown() else { return };
        let Some(&(i, _)) = places(c).get(at) else { return };
        if !self.conflicts.reused(i) {
            self.say(self.tr("cf.not_reused"));
            return;
        }
        let (wt, path) = (c.wt.clone(), c.files[i].path.clone());
        self.act(Task::Forget { wt, path });
    }

    /// The file's button: its places back as git left them.
    pub(crate) fn conflict_restart(&mut self, file: usize) {
        let Some(c) = self.conflicts.shown() else { return };
        let Some(f) = c.files.get(file) else { return };
        let (wt, path) = (c.wt.clone(), f.path.clone());
        self.conflict_unpick(&wt, &path);
        self.act(Task::Restart { wt, path });
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::app::tests::{app, toast};
    use michi_core::conflict::File;
    use michi_core::conflict::{Block, Side, Whole, Why};

    fn block(open: bool) -> Block {
        Block {
            line: open.then_some(3),
            ours: vec!["a".into()],
            base: Some(vec!["o".into()]),
            theirs: vec!["b".into()],
            result: (!open).then(|| vec!["a".into()]),
            before: Vec::new(),
            skipped: 0,
            after: Vec::new(),
            skipped_after: 0,
        }
    }

    /// A merge in `wt` with x.txt (places open, solved, open) and y.bin
    /// clashed whole.
    pub(crate) fn conflict(wt: &str) -> Conflict {
        Conflict {
            wt: PathBuf::from(wt),
            doing: Doing::Merge,
            ours: Side { name: "main".into(), id: None },
            theirs: Side { name: "feat".into(), id: None },
            base: None,
            started: None,
            message: "Merge branch 'feat'\n".into(),
            files: vec![
                File { path: "x.txt".into(), unmerged: true, body: Body::Text(vec![block(true), block(false), block(true)]) },
                File { path: "y.bin".into(), unmerged: true, body: Body::Whole(Whole { why: Why::Binary, ours: true, theirs: true }) },
            ],
            auto: Vec::new(),
        }
    }

    #[test]
    fn an_intent_is_kept_for_its_place_until_the_worktree_is_done() {
        let mut cf = Conflicts::default();
        cf.set(vec![conflict("/r/main")]);
        let change = michi_core::conflict::Change {
            id: michi_core::oid::Oid::parse(&"a".repeat(40)).unwrap(),
            subject: "s".into(),
            author: "me".into(),
            time: 0,
        };
        cf.set_intent(Path::new("/r/main"), "x.txt", 2, Intent { ours: Some(change), theirs: None });
        assert!(cf.intent((0, Some(2))).is_some_and(|i| i.theirs.is_none()));
        assert!(cf.intent((0, Some(0))).is_none(), "another place");
        cf.set_one(Path::new("/r/main"), None);
        assert!(cf.intents.is_empty());
    }

    #[test]
    fn a_worktree_read_on_its_own_leaves_the_others_waiting() {
        let mut cf = Conflicts::default();
        cf.set(vec![conflict("/r/main"), conflict("/r/feat")]);
        let key = |wt: &str| (paths::key(Path::new(wt)), "x.txt".to_owned(), Some(0));
        cf.pending.extend([key("/r/main"), key("/r/feat")]);
        let mut solved = conflict("/r/main");
        solved.files.remove(1);
        cf.set_one(Path::new("/r/main"), Some(solved));
        assert_eq!((cf.all.len(), cf.all[0].files.len()), (2, 1), "put in its place");
        assert!(!cf.pending.contains(&key("/r/main")));
        assert!(cf.pending.contains(&key("/r/feat")), "not read yet");
        cf.set_one(Path::new("/r/main"), None);
        assert_eq!(cf.all.iter().map(|c| c.wt.clone()).collect::<Vec<_>>(), [PathBuf::from("/r/feat")]);
    }

    #[test]
    fn a_file_solved_without_a_pick_here_is_rerere_s_until_looked_at() {
        let mut a = app();
        let mut c = conflict("/r/main");
        c.files[0].body = Body::Text(vec![block(false), block(false)]);
        a.conflicts.set(vec![c]);
        assert_eq!(a.conflicts.tag((0, Some(1))), Some(Tag::Reused));
        assert_eq!((a.conflicts.reused(0), a.conflicts.reused(1), a.conflicts.reused_left()), (true, false, 1));
        // x on a place that is not rerere's says so; Enter does nothing.
        a.conflicts.at = 2;
        assert!(!a.conflict_confirm());
        a.conflict_key('x');
        assert_eq!(toast(&a), a.tr("cf.not_reused"));
        // Enter on rerere's file looks at it.
        a.conflicts.at = 0;
        assert!(a.conflict_confirm());
        assert_eq!((a.conflicts.tag((0, Some(0))), a.conflicts.reused_left()), (None, 0));
        assert!(!a.conflict_confirm(), "once");
    }

    #[test]
    fn places_go_round_and_skip_what_is_solved_or_waited_for() {
        let mut cf = Conflicts::default();
        cf.set(vec![conflict("/r/main")]);
        assert_eq!(cf.places(), [(0, Some(0)), (0, Some(1)), (0, Some(2)), (1, None)]);
        assert_eq!((cf.at, cf.message.as_str()), (0, "Merge branch 'feat'"));
        cf.move_by(-1);
        assert_eq!(cf.at, 3);
        cf.move_by(1);
        assert_eq!(cf.at, 0);
        cf.next_open();
        assert_eq!(cf.at, 2, "the solved one is passed over");
        cf.pending.insert(cf.key((1, None)).unwrap());
        cf.next_open();
        assert_eq!(cf.at, 0, "and so is the one waited for");
        assert_eq!(cf.left(), 3);
    }

    #[test]
    fn a_pick_sends_its_task_tags_the_place_and_moves_on() {
        let mut a = app();
        a.conflicts.set(vec![conflict("/r/main")]);
        a.view = View::Conflict;
        assert!(a.feed('t'));
        assert_eq!(a.conflicts.tag((0, Some(0))), Some(Tag::Picked(Pick::Theirs)));
        assert!(a.conflicts.waits((0, Some(0))));
        assert_eq!(a.conflicts.at, 2);
        a.feed('o');
        assert_eq!(a.conflicts.at, 3);
        a.feed('b');
        assert_eq!(toast(&a), "A file that clashed whole takes one side: o or t");
        a.feed('t');
        assert_eq!(a.conflicts.tag((1, None)), Some(Tag::Took(false)));
        a.conflicts.at = 1;
        a.feed('o');
        assert!(toast(&a).starts_with("That place is solved already"), "{}", toast(&a));
        a.conflict_unpick(Path::new("/r/main"), "x.txt");
        assert_eq!((a.conflicts.tag((0, Some(0))), a.conflicts.waits((0, Some(0)))), (None, false));
        a.conflicts.set(vec![conflict("/r/main")]);
        assert!(!a.conflicts.waits((1, None)), "read again");
        assert_eq!(a.conflicts.tag((1, None)), Some(Tag::Took(false)));
    }

    #[test]
    fn four_opens_only_while_something_clashed() {
        let mut a = app();
        assert!(a.feed('4'));
        assert_eq!((a.view, toast(&a)), (View::Graph, "Nothing clashed: the 4 view opens while a merge or the like stops on one"));
        a.conflicts.set(vec![conflict("/r/main"), conflict("/r/feat")]);
        a.feed('4');
        assert_eq!(a.view, View::Conflict);
        assert_eq!(a.conflicts.shown().unwrap().wt, PathBuf::from("/r/main"));
        assert!(a.conflicts.show(Path::new("/r/feat")));
        assert!(!a.conflicts.show(Path::new("/r/other")));
        a.conflicts.set(vec![conflict("/r/main")]);
        assert_eq!(a.conflicts.shown().unwrap().wt, PathBuf::from("/r/main"), "gone: the first");
    }

    #[test]
    fn a_commit_waits_for_every_place_and_a_message() {
        let mut a = app();
        a.conflicts.set(vec![conflict("/r/main")]);
        a.conflict_commit();
        assert_eq!(toast(&a), "3 left to solve first");
        let mut c = conflict("/r/main");
        c.files.truncate(1);
        let Body::Text(blocks) = &mut c.files[0].body else { panic!() };
        *blocks = vec![block(false)];
        a.conflicts.set(vec![c]);
        a.conflicts.message.clear();
        a.conflict_commit();
        assert_eq!(toast(&a), "Write the commit's message first");
        assert!(a.conflicts.focus_message);
    }

    #[test]
    fn the_abort_row_takes_y_and_n() {
        let mut a = app();
        a.conflicts.set(vec![conflict("/r/main")]);
        a.view = View::Conflict;
        a.conflict_ask_abort();
        a.feed('o');
        assert!(a.conflicts.abort, "other keys wait");
        a.feed('n');
        assert!(!a.conflicts.abort);
        a.conflict_ask_abort();
        assert!(a.escape());
        assert!(!a.conflicts.abort);
    }
}
