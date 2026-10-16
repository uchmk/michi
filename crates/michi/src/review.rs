//! The branch review's state, with nothing of egui in it (docs/design.md:
//! ブランチ全体の差分): a worktree's branch from where it left the trunk,
//! its files on the left, the selected one's diff on the right, and the
//! files checked as looked at. What it wants read goes out as a [`Wish`]
//! with a number, and an answer to an older one is dropped.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use michi_core::git;
use michi_core::oid::Oid;
use michi_core::review::{self, Key, List};
use michi_core::show::{Changes, File};

use crate::detail::{At, Rows, Unified};
use michi_core::notes::{self, Note, Side};

/// What the review reads to: the trunk's commit, the branch's, and the
/// worktree's WIP commit when it has uncommitted changes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ends {
    pub trunk: Oid,
    pub head: Oid,
    pub wip: Option<Oid>,
}

/// What the review wants of the reading thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wish {
    /// The files changed on `branch` (its commit `head`) from where it left
    /// `trunk`, or from `since`, to `end`; with `prune`, the checks kept to
    /// the files as they are now.
    List { gen: u32, branch: String, trunk: Oid, head: Oid, end: Oid, since: Option<Oid>, prune: bool },
    /// One file's diff from `from` to `to`: its path, and where it came
    /// from when renamed.
    File { gen: u32, from: Oid, to: Oid, paths: Vec<String> },
    /// The file `key` of `branch` is checked as looked at, or not.
    Check { branch: String, key: Key, on: bool },
}

/// A number for the next wish to read, never the same twice in the
/// process (0 is none's): an answer to a review closed since is not taken
/// for one to the review open now.
pub(crate) fn next_gen() -> u32 {
    static GEN: AtomicU32 = AtomicU32::new(0);
    GEN.fetch_add(1, Ordering::Relaxed).wrapping_add(1)
}

/// What the list is read for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Range {
    trunk: Oid,
    head: Oid,
    end: Oid,
    since: Option<Oid>,
}

/// A file's diff as read: the file, and its diff cut into rows; `None` when
/// git had nothing for it.
pub type Diff = Option<Box<(File, Rows)>>;

pub struct Review {
    /// The worktree's folder and its branch.
    pub path: PathBuf,
    pub branch: String,
    /// The uncommitted changes are in (`u`).
    pub wip: bool,
    /// Only what changed since the last look (`s`).
    pub since_last: bool,
    /// The range last asked for, and the one the list was read for.
    asked: Option<Range>,
    listed: Option<Range>,
    list_gen: u32,
    /// The list; `None` until read.
    pub list: Option<Result<List, git::Error>>,
    /// The end of the last look before this review opened; `None` until the
    /// first list tells.
    last: Option<Option<Oid>>,
    pub sel: usize,
    /// The diff last asked for (its range's start and end, its path), and
    /// the one read: kept for the same path while another is read.
    file_asked: Option<(Oid, Oid, String)>,
    file_gen: u32,
    shown: Option<(String, Result<Diff, git::Error>)>,
    /// The files checked as looked at.
    pub checks: BTreeSet<Key>,
    /// The diff side by side.
    pub wide: bool,
    /// Pages to scroll the diff by, for the frame to take: down is more.
    pub pages: i32,
    /// The diff goes back to its top: another file is selected.
    pub to_top: bool,
    /// The line of the selected file's diff the notes are about (`J` `K`).
    pub line: Option<At>,
    /// The cursor moved: the diff brings it into sight.
    pub follow: bool,
    /// The notes on lines of the branch, until they are handed to its Claude.
    pub notes: Vec<Note>,
    /// The note being written for `line`.
    pub editing: Option<String>,
}

impl Review {
    pub fn new(path: PathBuf, branch: String) -> Review {
        Review {
            path,
            branch,
            wip: true,
            since_last: false,
            asked: None,
            listed: None,
            list_gen: 0,
            list: None,
            last: None,
            sel: 0,
            file_asked: None,
            file_gen: 0,
            shown: None,
            checks: BTreeSet::new(),
            wide: false,
            pages: 0,
            to_top: false,
            line: None,
            follow: false,
            notes: Vec::new(),
            editing: None,
        }
    }

    /// The list wanted for `ends`, unless it is asked already.
    pub fn ask(&mut self, ends: Ends) -> Option<Wish> {
        let end = if self.wip { ends.wip.unwrap_or(ends.head) } else { ends.head };
        let since = if self.since_last { self.last.flatten() } else { None };
        let range = Range { trunk: ends.trunk, head: ends.head, end, since };
        if self.asked == Some(range) {
            return None;
        }
        self.asked = Some(range);
        self.list_gen = next_gen();
        // Only the whole of the branch as it is tells which checks are stale.
        let prune = since.is_none() && (self.wip || ends.wip.is_none());
        Some(Wish::List { gen: self.list_gen, branch: self.branch.clone(), trunk: ends.trunk, head: ends.head, end, since, prune })
    }

    /// The list read for wish `gen`, with the files checked; the selection
    /// stays on its file. The selected file's diff, if it is wanted.
    pub fn take_list(&mut self, gen: u32, result: Result<List, git::Error>, checks: BTreeSet<Key>) -> Option<Wish> {
        if gen != self.list_gen {
            return None;
        }
        let path = self.selected().map(|f| f.path.clone());
        if let Ok(l) = &result {
            if self.last.is_none() {
                self.last = Some(l.last);
            }
            let files = &l.changes.files;
            let sel = path.as_ref().and_then(|p| files.iter().position(|f| f.path == *p));
            if sel.is_none() {
                self.to_top = true;
                self.line = None;
            }
            self.sel = sel.unwrap_or(0);
        }
        self.listed = self.asked;
        self.checks = checks;
        self.list = Some(result);
        self.want_file()
    }

    /// The selected file's diff, unless it is asked already.
    fn want_file(&mut self) -> Option<Wish> {
        let range = self.listed?;
        let Some(Ok(l)) = &self.list else { return None };
        let from = range.since.or(l.base)?;
        let f = l.changes.files.get(self.sel)?;
        let key = (from, range.end, f.path.clone());
        if self.file_asked.as_ref() == Some(&key) {
            return None;
        }
        let paths = std::iter::once(f.path.clone()).chain(f.from.clone()).collect();
        self.file_asked = Some(key);
        self.file_gen = next_gen();
        Some(Wish::File { gen: self.file_gen, from, to: range.end, paths })
    }

    /// The diff read for wish `gen`.
    pub fn take_file(&mut self, gen: u32, result: Result<Changes, git::Error>) {
        if gen != self.file_gen {
            return;
        }
        let Some((_, _, path)) = self.file_asked.clone() else { return };
        let diff = result.map(|c| {
            let f = c.files.iter().find(|f| f.path == path).or(c.files.first())?;
            let rows = Rows::of(f);
            Some(Box::new((f.clone(), rows)))
        });
        self.shown = Some((path, diff));
    }

    /// The selected file's diff: `None` while it is read.
    pub fn diff(&self) -> Option<&Result<Diff, git::Error>> {
        let (path, diff) = self.shown.as_ref()?;
        (self.selected()?.path == *path).then_some(diff)
    }

    pub fn files(&self) -> &[File] {
        match &self.list {
            Some(Ok(l)) => &l.changes.files,
            _ => &[],
        }
    }

    pub fn selected(&self) -> Option<&File> {
        self.files().get(self.sel)
    }

    /// The lines of the selected file's diff, top to bottom.
    fn diff_lines(&self) -> Vec<At> {
        match self.diff() {
            Some(Ok(Some(d))) => d.1.unified.iter().filter_map(|u| if let Unified::Line(at) = u { Some(*at) } else { None }).collect(),
            _ => Vec::new(),
        }
    }

    /// Moves the line cursor by `by` lines in the selected file's diff; the
    /// first move puts it on the first (or last) line.
    pub fn step(&mut self, by: i64) {
        let lines = self.diff_lines();
        if lines.is_empty() {
            return;
        }
        let now = self.line.and_then(|at| lines.iter().position(|l| *l == at));
        let k = match now {
            Some(k) => (k as i64 + by).clamp(0, lines.len() as i64 - 1) as usize,
            None if by < 0 => lines.len() - 1,
            None => 0,
        };
        self.line = Some(lines[k]);
        self.follow = true;
    }

    /// Puts the cursor on `at`, a line of the selected file's diff.
    pub fn point(&mut self, at: At) {
        if self.diff_lines().contains(&at) {
            self.line = Some(at);
        }
    }

    /// What a note on the cursor's line is about: its side, its number and
    /// its text. `None` without a line or a diff.
    pub fn target(&self) -> Option<(Side, u32, String)> {
        let at = self.line?;
        let Some(Ok(Some(d))) = self.diff() else { return None };
        let l = d.0.hunks.get(at.0 as usize)?.lines.get(at.1 as usize)?;
        Some(if l.new > 0 { (Side::New, l.new, l.text.clone()) } else { (Side::Old, l.old, l.text.clone()) })
    }

    /// The note on the cursor's line.
    pub fn note_here(&self) -> Option<&Note> {
        let (side, line, _) = self.target()?;
        notes::find(&self.notes, &self.selected()?.path, side, line)
    }

    /// Whether the line `at` of the selected file's diff has a note.
    pub fn is_noted(&self, at: At) -> bool {
        let (Some(Ok(Some(d))), Some(f)) = (self.diff(), self.selected()) else { return false };
        let Some(l) = d.0.hunks.get(at.0 as usize).and_then(|h| h.lines.get(at.1 as usize)) else { return false };
        let (side, n) = if l.new > 0 { (Side::New, l.new) } else { (Side::Old, l.old) };
        notes::find(&self.notes, &f.path, side, n).is_some()
    }

    /// `n`: starts writing the note for the cursor's line (the line is the
    /// first one when there is none). False when there is no line.
    pub fn write_note(&mut self) -> bool {
        if self.line.is_none() {
            self.step(1);
        }
        if self.target().is_none() {
            return false;
        }
        self.editing = Some(self.note_here().map(|n| n.text.clone()).unwrap_or_default());
        true
    }

    /// Keeps what was written (nothing written takes the note away).
    pub fn noted(&mut self) {
        let (Some(text), Some((side, line, code)), Some(f)) = (self.editing.take(), self.target(), self.selected()) else { return };
        let path = f.path.clone();
        notes::put(&mut self.notes, Note { path, line, side, code, text });
    }

    pub fn is_checked(&self, f: &File) -> bool {
        self.checks.contains(&review::key(f))
    }

    /// How many of the files are checked, of how many.
    pub fn checked(&self) -> (usize, usize) {
        let files = self.files();
        (files.iter().filter(|f| self.is_checked(f)).count(), files.len())
    }

    /// The end of the last look when `s` is on.
    pub fn since(&self) -> Option<Oid> {
        self.listed.and_then(|r| r.since)
    }

    /// Selects the `k`th file (the last when there are fewer).
    pub fn select(&mut self, k: usize) -> Option<Wish> {
        let n = self.files().len();
        if n == 0 {
            return None;
        }
        let k = k.min(n - 1);
        if k != self.sel {
            self.sel = k;
            self.to_top = true;
            self.pages = 0;
            self.line = None;
        }
        self.want_file()
    }

    pub fn move_by(&mut self, by: i64) -> Option<Wish> {
        self.select((self.sel as i64 + by).max(0) as usize)
    }

    /// Checks the `k`th file as looked at, or takes its check off.
    pub fn check_at(&mut self, k: usize) -> Option<Wish> {
        let key = review::key(self.files().get(k)?);
        let on = !self.checks.remove(&key);
        if on {
            self.checks.insert(key.clone());
        }
        Some(Wish::Check { branch: self.branch.clone(), key, on })
    }

    /// Space: checks the selected file, or takes its check off; a file
    /// checked, the next is selected.
    pub fn check(&mut self) -> Vec<Wish> {
        let Some(w) = self.check_at(self.sel) else { return Vec::new() };
        let on = matches!(w, Wish::Check { on: true, .. });
        let mut out = vec![w];
        if on {
            out.extend(self.move_by(1));
        }
        out
    }

    /// The list's and the diff's wishes asked last, as an answer carries
    /// them back.
    #[cfg(test)]
    pub fn gens(&self) -> (u32, u32) {
        (self.list_gen, self.file_gen)
    }

    /// `s`: only what changed since the last look, or the whole branch
    /// again. Err with the word saying why not: the last look is not known
    /// yet, or there was none.
    pub fn toggle_since(&mut self) -> Result<(), &'static str> {
        if !self.since_last {
            match self.last {
                None => return Err("review.reading"),
                Some(None) => return Err("review.never"),
                Some(Some(_)) => {}
            }
        }
        self.since_last = !self.since_last;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use michi_core::show::Status;

    fn oid(n: u8) -> Oid {
        Oid::parse(&format!("{n:02x}").repeat(20)).unwrap()
    }

    fn file(path: &str, blob: u8) -> File {
        File {
            status: Status::Modified,
            path: path.to_owned(),
            from: None,
            blob: Some(oid(blob)),
            counts: Some((1, 0)),
            hunks: Vec::new(),
            eol_only: false,
            cut: false,
            not_utf8: false,
        }
    }

    fn listed(files: Vec<File>, last: Option<Oid>) -> List {
        List { base: Some(oid(1)), last, changes: Changes { files, cut: false } }
    }

    fn ends(head: u8, wip: Option<u8>) -> Ends {
        Ends { trunk: oid(9), head: oid(head), wip: wip.map(oid) }
    }

    fn gen(w: &Option<Wish>) -> u32 {
        match w {
            Some(Wish::List { gen, .. } | Wish::File { gen, .. }) => *gen,
            _ => panic!("no list or file wished: {w:?}"),
        }
    }

    /// A review with `a.txt` and `b.txt` listed, and the first's diff
    /// asked.
    fn opened() -> (Review, u32) {
        let mut r = Review::new(PathBuf::from("C:/r/wt"), "lane".into());
        let w = r.ask(ends(2, Some(3)));
        let file = r.take_list(gen(&w), Ok(listed(vec![file("a.txt", 4), file("b.txt", 5)], None)), BTreeSet::new());
        let fgen = gen(&file);
        (r, fgen)
    }

    #[test]
    fn the_list_is_asked_once_per_range_with_the_uncommitted_changes_in() {
        let mut r = Review::new(PathBuf::from("C:/r/wt"), "lane".into());
        let w = r.ask(ends(2, Some(3)));
        let Some(Wish::List { end, since, prune, branch, .. }) = &w else { panic!("{w:?}") };
        assert_eq!((*end, *since, *prune, branch.as_str()), (oid(3), None, true, "lane"));
        assert_eq!(r.ask(ends(2, Some(3))), None);
        r.wip = false;
        let Some(Wish::List { end, prune, .. }) = r.ask(ends(2, Some(3))) else { panic!() };
        // Without the uncommitted changes the checks on them are not stale.
        assert_eq!((end, prune), (oid(2), false));
        let Some(Wish::List { end, prune, .. }) = r.ask(ends(3, None)) else { panic!() };
        assert_eq!((end, prune), (oid(3), true));
        assert!(r.ask(ends(4, None)).is_some());
    }

    #[test]
    fn a_list_asks_for_the_first_file_s_diff_and_older_answers_are_dropped() {
        let mut r = Review::new(PathBuf::from("C:/r/wt"), "lane".into());
        let old = gen(&r.ask(ends(2, None)));
        let new = gen(&r.ask(ends(3, None)));
        assert_eq!(r.take_list(old, Ok(listed(vec![file("old.txt", 4)], None)), BTreeSet::new()), None);
        assert!(r.list.is_none());
        let w = r.take_list(new, Ok(listed(vec![file("a.txt", 4)], None)), BTreeSet::new());
        let Some(Wish::File { from, to, paths, gen }) = w else { panic!("{w:?}") };
        assert_eq!((from, to, paths), (oid(1), oid(3), vec!["a.txt".to_owned()]));
        assert!(r.diff().is_none());
        r.take_file(gen.wrapping_sub(1), Ok(Changes::default()));
        assert!(r.diff().is_none());
        r.take_file(gen, Ok(Changes { files: vec![file("a.txt", 4)], cut: false }));
        assert!(matches!(r.diff(), Some(Ok(Some(d))) if d.0.path == "a.txt"));
    }

    #[test]
    fn the_selection_stays_on_its_file_and_its_diff_until_the_new_one_comes() {
        let (mut r, fgen) = opened();
        r.take_file(fgen, Ok(Changes { files: vec![file("a.txt", 4)], cut: false }));
        let w = r.move_by(1);
        assert_eq!(r.selected().unwrap().path, "b.txt");
        assert!(r.to_top);
        let fgen = gen(&w);
        r.take_file(fgen, Ok(Changes { files: vec![file("b.txt", 5)], cut: false }));
        assert_eq!(r.move_by(5), None);
        assert_eq!(r.sel, 1);

        // b.txt changes and a file comes before it: still b.txt, its old diff
        // shown while the new one is read.
        let w = r.ask(ends(2, Some(6)));
        let w = r.take_list(gen(&w), Ok(listed(vec![file("0.txt", 7), file("a.txt", 4), file("b.txt", 8)], None)), BTreeSet::new());
        assert_eq!(r.sel, 2);
        assert!(matches!(w, Some(Wish::File { to, .. }) if to == oid(6)));
        assert!(matches!(r.diff(), Some(Ok(Some(d))) if d.0.blob == Some(oid(5))));
    }

    #[test]
    fn space_checks_and_goes_on_and_a_check_goes_by_content() {
        let (mut r, _) = opened();
        let ws = r.check();
        assert_eq!(ws.len(), 2);
        assert_eq!(ws[0], Wish::Check { branch: "lane".into(), key: ("a.txt".into(), Some(oid(4))), on: true });
        assert!(matches!(ws[1], Wish::File { .. }));
        assert_eq!(r.sel, 1);
        assert_eq!(r.checked(), (1, 2));
        // Taking a check off stays.
        r.select(0);
        let ws = r.check();
        assert_eq!(ws, [Wish::Check { branch: "lane".into(), key: ("a.txt".into(), Some(oid(4))), on: false }]);
        assert_eq!((r.sel, r.checked()), (0, (0, 2)));
        // The checks the list comes with are the thread's: a.txt changed.
        r.check_at(0);
        let w = r.ask(ends(2, Some(6)));
        let kept = BTreeSet::from([("b.txt".to_owned(), Some(oid(5)))]);
        r.take_list(gen(&w), Ok(listed(vec![file("a.txt", 7), file("b.txt", 5)], None)), kept);
        assert_eq!(r.checked(), (1, 2));
        assert!(!r.is_checked(&r.files()[0]));
    }

    #[test]
    fn since_the_last_look_needs_one_and_lists_from_it() {
        let (mut r, _) = opened();
        assert_eq!(r.toggle_since(), Err("review.never"));
        let mut r = Review::new(PathBuf::from("C:/r/wt"), "lane".into());
        assert_eq!(r.toggle_since(), Err("review.reading"));
        let w = r.ask(ends(2, None));
        r.take_list(gen(&w), Ok(listed(vec![file("a.txt", 4)], Some(oid(5)))), BTreeSet::new());
        assert_eq!(r.toggle_since(), Ok(()));
        let w = r.ask(ends(2, None));
        let Some(Wish::List { since, prune, .. }) = &w else { panic!("{w:?}") };
        assert_eq!((*since, *prune), (Some(oid(5)), false));
        // Each list moves the last look; the review keeps the one it opened with.
        let f = r.take_list(gen(&w), Ok(listed(vec![file("a.txt", 4)], Some(oid(2)))), BTreeSet::new());
        assert!(matches!(f, Some(Wish::File { from, .. }) if from == oid(5)));
        assert_eq!(r.since(), Some(oid(5)));
        assert_eq!(r.toggle_since(), Ok(()));
        assert!(matches!(r.ask(ends(2, None)), Some(Wish::List { since: None, .. })));
    }

    #[test]
    fn a_failed_list_asks_nothing() {
        let mut r = Review::new(PathBuf::from("C:/r/wt"), "lane".into());
        let w = r.ask(ends(2, None));
        assert_eq!(r.take_list(gen(&w), Err(git::Error::Git("bad".into())), BTreeSet::new()), None);
        assert!(r.files().is_empty());
        assert_eq!(r.check(), []);
        assert_eq!(r.select(0), None);
    }
}
