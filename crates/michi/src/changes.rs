//! The `2` view's state, with nothing of egui in it (docs/design.md: 変更):
//! one worktree's staged and unstaged files on the left, the selected one's
//! diff on the right with lines to pick, and the commit message below the
//! files. What it wants read or done goes out as a [`Wish`]; an answer to an
//! older read is dropped, and the writes are done in turn, none dropped.
//!
//! A write waits for a yes while a Claude is writing in the worktree, and
//! an amend of a commit some remote has; a yes for the Claude holds until
//! the worktree changes or the Claude stops. Throwing changes away always
//! waits for one.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use michi_core::git;
use michi_core::guard::{self, Note};
use michi_core::msghelp::{Help, Walk};
use michi_core::oid::Oid;
use michi_core::show::{File, Kind, Status};
use michi_core::stage::{self, Lists, Side};
use michi_core::tree::{self, Row};

use crate::detail::Rows;
use crate::review::next_gen;

/// What the `2` view wants of its reading thread.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wish {
    /// The worktree's staged and unstaged files.
    List { gen: u32, wt: PathBuf },
    /// One file's diff on one side.
    File { gen: u32, wt: PathBuf, side: Side, file: File },
    /// The head's message and whether a remote has the head, to amend it.
    Last { gen: u32, wt: PathBuf },
    /// The past messages and the template, to help write one (#74).
    Help { gen: u32, wt: PathBuf },
    /// A write, done in turn after those before it.
    Do { wt: PathBuf, op: Op },
}

/// A write to a worktree's index or history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// Files into the index as they are in the worktree; all of them when
    /// there are none.
    Stage(Vec<String>),
    /// Files back as the head has them; all of them when there are none.
    Unstage(Vec<String>),
    /// Picked lines in (out when `reverse`).
    Apply {
        patch: Vec<u8>,
        reverse: bool,
    },
    Commit {
        message: String,
        amend: bool,
    },
    /// The unstaged changes of the file at `path` thrown away, kept under a
    /// backup first: the file back as the index has it, deleted when it is
    /// `untracked`, or only the lines of `patch` (`lines` of them) taken out.
    Discard {
        path: String,
        untracked: bool,
        patch: Option<Vec<u8>>,
        lines: usize,
    },
    /// The untracked file at `path` added to the worktree's `.gitignore`.
    Ignore {
        path: String,
    },
}

impl Op {
    fn is_commit(&self) -> bool {
        matches!(self, Op::Commit { .. })
    }
}

/// What a write did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Did {
    /// Staged or unstaged.
    Staged,
    Committed(Oid),
    /// Threw away changes of `path` as [`Op::Discard`] says, kept under the
    /// backup `saved`.
    Discarded {
        path: String,
        untracked: bool,
        lines: usize,
        saved: String,
    },
    /// The untracked `path` is ignored by the `.gitignore` line `line`
    /// (`already` when the line was there).
    Ignored {
        path: String,
        line: String,
        already: bool,
    },
}

/// Where the keys go: the files, or the selected file's lines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Focus {
    #[default]
    Files,
    Lines,
}

/// A write waiting for a yes, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Confirm {
    pub op: Op,
    /// A Claude is writing in the worktree.
    pub claude: bool,
    /// It amends a commit some remote has.
    pub pushed: bool,
    /// The commit is on this branch, one kept clear of direct commits (#110).
    pub branch: Option<String>,
    /// Staged files that are big or look like secrets (#111).
    pub notes: Vec<Note>,
}

/// A file's diff as read, cut into rows, with its added and deleted lines
/// in the order shown (as `(hunk, line)`), which the cursor moves over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shown {
    pub diff: stage::Diff,
    pub rows: Rows,
    pub changed: Vec<(usize, usize)>,
}

impl Shown {
    fn of(diff: stage::Diff) -> Shown {
        let rows = Rows::of(&diff.file);
        let changed = diff
            .file
            .hunks
            .iter()
            .enumerate()
            .flat_map(|(h, hunk)| hunk.lines.iter().enumerate().filter(|(_, l)| l.kind != Kind::Context).map(move |(k, _)| (h, k)))
            .collect();
        Shown { diff, rows, changed }
    }
}

/// What a file's diff is when read: `None` when the file was gone.
pub type Read = Result<Option<Box<Shown>>, git::Error>;

#[derive(Default)]
pub struct Changes {
    /// `[commit] co_author`: the trailer added to the message committed (#54).
    pub co_author: michi_core::coauthor::CoAuthor,
    /// `[commit] protect` and `large_mb`: what a commit asks about first (#110, #111).
    pub guard: guard::Config,
    /// The worktree shown; `None` until the window picks one.
    pub wt: Option<PathBuf>,
    list_gen: u32,
    list_waiting: bool,
    /// The files; `None` until read.
    pub lists: Option<Result<Lists, git::Error>>,
    /// The selected file, counted over the staged ones then the unstaged.
    pub sel: usize,
    /// Where a multiple selection started (Shift+Up / Down): the files from
    /// here to `sel`, on one side, are acted on together.
    mark: Option<usize>,
    /// Files added one by one with Ctrl+click; with `sel` they are acted on
    /// together, on one side.
    picks: Vec<usize>,
    /// The diff last asked for, and the one read: kept for the same file
    /// while it is read again.
    file_asked: Option<(Side, String)>,
    file_gen: u32,
    file_waiting: bool,
    shown: Option<(Side, String, Read)>,
    /// The wish the shown diff was read for: a new number each time it is
    /// read, so a picture is read again when its file may have changed.
    pub shown_gen: u32,
    /// The file whose hunks are cut into their runs of changes (`s`): it is
    /// cut again each time it is read.
    split: Option<(Side, String)>,
    pub focus: Focus,
    /// The line the cursor is on (of [`Shown::changed`]), and where a range
    /// of lines started.
    pub cursor: usize,
    pub anchor: Option<usize>,
    /// The commit message, and the head's as it was filled in for an amend.
    pub message: String,
    pub amend: bool,
    filled: Option<String>,
    last_gen: u32,
    /// Whether a remote has the head; `None` until read.
    pub pushed: Option<bool>,
    /// A yes was given to write while the worktree's Claude writes.
    pub claude_ok: bool,
    pub confirm: Option<Confirm>,
    /// Writes sent and not done yet.
    pending: u32,
    /// The diff side by side (`Enter`).
    pub wide: bool,
    /// One column even when not widened (side by side is the default).
    pub unified: bool,
    /// The files as a tree of folders (`t`), not kept between runs.
    pub tree: bool,
    /// The folders folded in the tree, per side.
    pub folded: HashSet<(Side, String)>,
    /// Pages to scroll the diff by, for the frame to take: down is more.
    pub pages: i32,
    /// Changes to jump over in the diff (`n` is 1, `N` is -1), for the frame
    /// to take: it knows where the changes lie on the screen.
    pub jump: i32,
    /// The diff goes back to its top: another file is selected.
    pub to_top: bool,
    /// The message box has the keys (the frame says so each frame), and the
    /// frame is to give them to it.
    pub typing: bool,
    pub focus_message: bool,
    /// The past messages and the template of the worktree, read when the box
    /// is first given the keys; read again after a commit.
    help: Option<Help>,
    help_gen: u32,
    walk: Walk,
    /// The messages of the other worktrees, kept while another is shown.
    drafts: HashMap<PathBuf, String>,
    /// All git said when a write failed and it said more than a line (a
    /// hook's output, mostly): shown in place of the diff until Esc, a
    /// write that goes through, the lines or another file.
    pub told: Option<Told>,
}

/// What git said, whole, when a write failed.
#[derive(Debug, Clone, PartialEq)]
pub struct Told {
    /// It was a commit: the hooks spoke too.
    pub commit: bool,
    pub all: String,
}

impl Changes {
    /// Whether the diff is drawn side by side: widened, or by default.
    pub fn paired(&self) -> bool {
        self.wide || !self.unified
    }

    /// Shows the worktree at `wt`, its message kept for when it comes back.
    pub fn open(&mut self, wt: PathBuf) -> Option<Wish> {
        if let Some(old) = self.wt.take() {
            let message = std::mem::take(&mut self.message);
            if !message.trim().is_empty() && !self.amend {
                self.drafts.insert(old, message);
            }
        }
        self.message = self.drafts.remove(&wt).unwrap_or_default();
        self.forget_help();
        self.wt = Some(wt);
        self.lists = None;
        self.sel = 0;
        self.file_asked = None;
        self.file_waiting = false;
        self.shown = None;
        self.split = None;
        self.focus = Focus::Files;
        (self.cursor, self.anchor) = (0, None);
        (self.amend, self.filled, self.pushed) = (false, None, None);
        self.claude_ok = false;
        self.confirm = None;
        self.told = None;
        self.to_top = true;
        self.pages = 0;
        self.jump = 0;
        self.refresh()
    }

    /// The files again.
    pub fn refresh(&mut self) -> Option<Wish> {
        let wt = self.wt.clone()?;
        self.list_gen = next_gen();
        self.list_waiting = true;
        Some(Wish::List { gen: self.list_gen, wt })
    }

    /// The files read for wish `gen`: the selection stays on its file, else
    /// at its place on its side; the selected file's diff is read again.
    pub fn take_list(&mut self, gen: u32, result: Result<Lists, git::Error>) -> Vec<Wish> {
        if gen != self.list_gen {
            return Vec::new();
        }
        self.list_waiting = false;
        let old = self.selected().map(|(side, f)| (side, f.path.clone(), self.index_in(side)));
        let head = self.lists().map(|l| l.head);
        self.lists = Some(result);
        let mut out = Vec::new();
        let Some(Ok(lists)) = &self.lists else {
            self.focus = Focus::Files;
            return out;
        };
        let at = |side: Side, path: &str| lists.of(side).iter().position(|f| f.path == path).map(|k| flat(lists, side, k));
        let n = lists.staged.len() + lists.unstaged.len();
        let new_head = lists.head;
        let sel = match &old {
            Some((side, path, k)) => at(*side, path).or_else(|| {
                let other = if *side == Side::Staged { Side::Unstaged } else { Side::Staged };
                let len = lists.of(*side).len();
                if len > 0 {
                    Some(flat(lists, *side, (*k).min(len - 1)))
                } else {
                    at(other, path)
                }
            }),
            None => None,
        };
        let moved = sel.is_none() || old.as_ref().is_some_and(|(s, p, _)| at(*s, p) != sel);
        self.sel = sel.unwrap_or(self.sel).min(n.saturating_sub(1));
        if moved {
            self.leave_lines();
            self.to_top = true;
        }
        if self.amend && head.is_some_and(|h| h != new_head) {
            out.extend(self.ask_last());
        }
        self.file_asked = None;
        out.extend(self.want_file());
        out
    }

    /// The selected file's diff, read again (the diff is to be read another
    /// way): out of the lines, which the new diff may not have.
    pub fn reread_file(&mut self) -> Option<Wish> {
        self.leave_lines();
        self.split = None;
        self.file_asked = None;
        self.want_file()
    }

    /// The selected file's diff, unless it is asked already.
    fn want_file(&mut self) -> Option<Wish> {
        let wt = self.wt.clone()?;
        let (side, f) = self.selected()?;
        let key = (side, f.path.clone());
        if self.file_asked.as_ref() == Some(&key) {
            return None;
        }
        let file = f.clone();
        self.file_asked = Some(key);
        self.file_gen = next_gen();
        self.file_waiting = true;
        Some(Wish::File { gen: self.file_gen, wt, side, file })
    }

    /// The diff read for wish `gen`; the cursor stays on its place.
    pub fn take_file(&mut self, gen: u32, result: Result<Option<stage::Diff>, git::Error>) {
        if gen != self.file_gen {
            return;
        }
        self.file_waiting = false;
        let Some((side, path)) = self.file_asked.clone() else { return };
        let split = self.split.as_ref().is_some_and(|(s, p)| *s == side && *p == path);
        let read = result.map(|d| {
            d.map(|mut d| {
                if split {
                    d.split();
                }
                Box::new(Shown::of(d))
            })
        });
        let lines = match &read {
            Ok(Some(s)) if s.diff.patch.is_some() => s.changed.len(),
            _ => 0,
        };
        if lines == 0 {
            self.leave_lines();
        } else {
            self.cursor = self.cursor.min(lines - 1);
            self.anchor = self.anchor.map(|a| a.min(lines - 1));
        }
        self.shown = Some((side, path, read));
        self.shown_gen = gen;
    }

    /// The head's message and whether a remote has it, read for wish `gen`:
    /// the message goes in when there is none yet.
    pub fn take_last(&mut self, gen: u32, result: Result<(String, bool), git::Error>) {
        if gen != self.last_gen {
            return;
        }
        match result {
            Ok((message, pushed)) => {
                self.pushed = Some(pushed);
                if self.amend && self.message.trim().is_empty() {
                    self.message = message.clone();
                    self.filled = Some(message);
                }
            }
            // Unknown: an amend is then refused rather than risked.
            Err(_) => self.pushed = None,
        }
    }

    /// What was read for help is let go: it is read again, newer, when the
    /// box next gets the keys.
    fn forget_help(&mut self) {
        (self.help, self.help_gen) = (None, 0);
        self.walk.reset();
    }

    /// `c`: the box gets the keys. The help is read if it was not yet.
    pub fn focus_message(&mut self) -> Option<Wish> {
        self.focus_message = true;
        self.ask_help()
    }

    /// The help is read if it was not yet (the box has the keys).
    pub fn ask_help(&mut self) -> Option<Wish> {
        if self.help.is_some() || self.help_gen != 0 {
            return None;
        }
        let wt = self.wt.clone()?;
        self.help_gen = next_gen();
        Some(Wish::Help { gen: self.help_gen, wt })
    }

    /// The help read for wish `gen`. The template fills a box that is empty
    /// (git does the same); an amend has its own message.
    pub fn take_help(&mut self, gen: u32, help: Help) {
        if gen != self.help_gen {
            return;
        }
        if self.message.trim().is_empty() && !self.amend {
            if let Some(t) = &help.template {
                self.message = t.clone();
            }
        }
        self.help = Some(help);
    }

    /// Alt+Up (`older`) or Alt+Down in the box: the message of an earlier
    /// commit in its place, and back to what was being written. Whether the
    /// box changed.
    pub fn step_history(&mut self, older: bool) -> bool {
        let Some(help) = &self.help else { return false };
        match self.walk.step(&help.past, older, &self.message) {
            Some(text) => {
                self.message = text;
                true
            }
            None => false,
        }
    }

    /// A write in `wt` is done (`commit` when it was a commit): a commit's
    /// message goes, and the files are read again.
    pub fn take_done(&mut self, wt: &Path, commit: bool, ok: bool) -> Option<Wish> {
        self.pending = self.pending.saturating_sub(1);
        if commit && ok {
            self.drafts.remove(wt);
        }
        if self.wt.as_deref() != Some(wt) {
            return None;
        }
        if commit && ok {
            self.message.clear();
            (self.amend, self.filled, self.pushed) = (false, None, None);
            self.forget_help();
        }
        if ok {
            self.told = None;
        }
        self.refresh()
    }

    /// A write in `wt` failed and git said `all`, more than a line: it is
    /// shown in place of the diff, from its top.
    pub fn tell(&mut self, wt: &Path, commit: bool, all: String) {
        if self.wt.as_deref() == Some(wt) {
            self.told = Some(Told { commit, all });
            self.to_top = true;
            self.pages = 0;
            self.jump = 0;
        }
    }

    /// The selected file's diff: `None` while it is read.
    pub fn diff(&self) -> Option<&Read> {
        let (side, path, read) = self.shown.as_ref()?;
        let (s, f) = self.selected()?;
        (s == *side && f.path == *path).then_some(read)
    }

    /// The selected file's diff when its lines can be picked.
    fn shown(&self) -> Option<&Shown> {
        match self.diff()? {
            Ok(Some(s)) if s.diff.patch.is_some() && !s.changed.is_empty() => Some(s),
            _ => None,
        }
    }

    pub fn lists(&self) -> Option<&Lists> {
        self.lists.as_ref()?.as_ref().ok()
    }

    /// The files, or the word saying why there are none.
    fn need_lists(&self) -> Result<&Lists, &'static str> {
        match &self.lists {
            None => Err("changes.reading"),
            Some(Err(_)) => Err("changes.unread"),
            Some(Ok(l)) => Ok(l),
        }
    }

    /// The selected file and its side.
    pub fn selected(&self) -> Option<(Side, &File)> {
        self.at(self.sel)
    }

    /// The `k`th file, counted over the staged ones then the unstaged.
    pub fn at(&self, k: usize) -> Option<(Side, &File)> {
        let l = self.lists()?;
        match l.staged.get(k) {
            Some(f) => Some((Side::Staged, f)),
            None => l.unstaged.get(k - l.staged.len()).map(|f| (Side::Unstaged, f)),
        }
    }

    /// How many files there are on both sides.
    pub fn count(&self) -> usize {
        self.lists().map_or(0, |l| l.staged.len() + l.unstaged.len())
    }

    /// The selection's place on its side.
    fn index_in(&self, side: Side) -> usize {
        let staged = self.lists().map_or(0, |l| l.staged.len());
        if side == Side::Staged {
            self.sel
        } else {
            self.sel.saturating_sub(staged)
        }
    }

    /// The files a Space acts on: the marked range, or the selected one.
    pub fn marked(&self) -> Vec<usize> {
        let staged = self.lists().map_or(0, |l| l.staged.len());
        if !self.picks.is_empty() {
            let mut v: Vec<usize> = self.picks.iter().copied().filter(|k| (*k < staged) == (self.sel < staged)).collect();
            v.push(self.sel);
            v.sort_unstable();
            v.dedup();
            return v;
        }
        match self.mark {
            Some(m) if (m < staged) == (self.sel < staged) => (m.min(self.sel)..=m.max(self.sel)).collect(),
            _ => vec![self.sel],
        }
    }

    /// Shift+click: the files from where the selection was to `k` (on the
    /// same side) are marked together.
    pub fn click_range(&mut self, k: usize) -> Option<Wish> {
        let staged = self.lists().map_or(0, |l| l.staged.len());
        if k >= self.count() || (k < staged) != (self.sel < staged) {
            return self.select(k);
        }
        let mark = self.mark.unwrap_or(self.sel);
        self.picks.clear();
        let wish = self.select(k);
        self.mark = Some(mark);
        wish
    }

    /// Ctrl+click: `k` joins the marked files, or leaves them when it was in.
    pub fn click_toggle(&mut self, k: usize) -> Option<Wish> {
        let staged = self.lists().map_or(0, |l| l.staged.len());
        if k >= self.count() || (k < staged) != (self.sel < staged) {
            return self.select(k);
        }
        let mut set = self.marked();
        match set.iter().position(|x| *x == k) {
            Some(i) if set.len() > 1 => {
                set.remove(i);
            }
            Some(_) => {}
            None => set.push(k),
        }
        let sel = if set.contains(&k) { k } else { *set.last().unwrap_or(&k) };
        let wish = self.select(sel);
        self.mark = None;
        self.picks = if set.len() > 1 { set } else { Vec::new() };
        wish
    }

    /// Whether file `k` is in a multiple selection (of more than the one).
    pub fn is_marked(&self, k: usize) -> bool {
        let r = self.marked();
        r.len() > 1 && r.contains(&k)
    }

    /// Shift+Down / Shift+Up: widen or narrow the multiple selection by one
    /// file. It stays on one side (staged or not).
    pub fn extend(&mut self, by: i64) -> Option<Wish> {
        let staged = self.lists().map_or(0, |l| l.staged.len());
        let to = self.sel as i64 + by;
        if to < 0 || to as usize >= self.count() || ((to as usize) < staged) != (self.sel < staged) {
            return None;
        }
        let mark = self.mark.unwrap_or(self.sel);
        self.picks.clear();
        let wish = self.select(to as usize);
        self.mark = Some(mark);
        wish
    }

    /// Esc: the multiple selection goes. Whether there was one.
    pub fn unmark(&mut self) -> bool {
        let had = !self.picks.is_empty();
        self.picks.clear();
        self.mark.take().is_some() || had
    }

    /// Selects the `k`th file (the last when there are fewer).
    pub fn select(&mut self, k: usize) -> Option<Wish> {
        let n = self.count();
        if n == 0 {
            return None;
        }
        let k = k.min(n - 1);
        self.mark = None;
        self.picks.clear();
        if k != self.sel {
            self.sel = k;
            self.to_top = true;
            self.pages = 0;
            self.jump = 0;
            self.told = None;
            self.leave_lines();
        }
        self.want_file()
    }

    pub fn move_by(&mut self, by: i64) -> Option<Wish> {
        if self.tree {
            let order = self.order_of(true);
            let Some(at) = order.iter().position(|&k| k == self.sel) else {
                // The selection is folded away: the nearest shown file in the direction.
                let full = self.order_of(false);
                let p = full.iter().position(|&k| k == self.sel).unwrap_or(0);
                let shown = |k: &&usize| order.contains(k);
                let near = if by > 0 { full[p..].iter().find(shown) } else { full[..p].iter().rev().find(shown) };
                return near.and_then(|&k| self.select(k));
            };
            let to = (at as i64 + by).clamp(0, order.len() as i64 - 1) as usize;
            return self.select(order[to]);
        }
        self.select((self.sel as i64 + by).max(0) as usize)
    }

    /// The tree's rows for `side`.
    pub fn tree_rows(&self, side: Side) -> Vec<Row> {
        self.rows_of(side, true)
    }

    fn rows_of(&self, side: Side, fold: bool) -> Vec<Row> {
        let Some(l) = self.lists() else { return Vec::new() };
        tree::rows(l.of(side), |p| fold && self.folded.contains(&(side, p.to_owned())))
    }

    /// The files shown, counted as [`Changes::at`] does, in the order the
    /// list has them (the tree's when it is on), the folded folders open
    /// when not `fold`.
    fn order_of(&self, fold: bool) -> Vec<usize> {
        let Some(l) = self.lists() else { return Vec::new() };
        if !self.tree {
            return (0..l.staged.len() + l.unstaged.len()).collect();
        }
        let mut out = tree::visible(&self.rows_of(Side::Staged, fold));
        out.extend(tree::visible(&self.rows_of(Side::Unstaged, fold)).into_iter().map(|i| i + l.staged.len()));
        out
    }

    /// `t`: the list as a tree of folders, or as a flat list again.
    pub fn toggle_tree(&mut self) {
        self.tree = !self.tree;
    }

    /// A folder of the tree folded, or opened again.
    pub fn fold(&mut self, side: Side, dir: &str) {
        let key = (side, dir.to_owned());
        if !self.folded.remove(&key) {
            self.folded.insert(key);
        }
    }

    /// A folder staged whole, or taken out of the index whole when `side`
    /// is the staged one.
    pub fn toggle_dir(&mut self, side: Side, dir: &str, claude: bool) -> Option<Wish> {
        let l = self.lists()?;
        let op = match side {
            Side::Unstaged => Op::Stage(tree::under(&l.unstaged, dir, false)),
            Side::Staged => Op::Unstage(tree::under(&l.staged, dir, true)),
        };
        if matches!(&op, Op::Stage(p) | Op::Unstage(p) if p.is_empty()) {
            return None;
        }
        self.act(op, claude)
    }

    /// The lines between the anchor and the cursor, as `(hunk, line)`.
    pub fn picked(&self) -> Vec<(usize, usize)> {
        let Some(s) = self.shown() else { return Vec::new() };
        let a = self.anchor.unwrap_or(self.cursor);
        let (lo, hi) = (a.min(self.cursor), a.max(self.cursor));
        s.changed.get(lo..=hi.min(s.changed.len().saturating_sub(1))).map_or(Vec::new(), <[_]>::to_vec)
    }

    /// The line the cursor is on, as `(hunk, line)`, in the lines' focus.
    pub fn cursor_line(&self) -> Option<(usize, usize)> {
        (self.focus == Focus::Lines).then(|| self.shown()?.changed.get(self.cursor).copied()).flatten()
    }

    /// The line of the file an editor is opened at: the cursor's line (the
    /// next one that exists in the file when it is a deleted line) in the
    /// lines' focus, else the first changed line of the diff.
    pub fn editor_line(&self) -> Option<usize> {
        let Some(Ok(Some(s))) = self.diff() else { return None };
        let hunks = &s.diff.file.hunks;
        let (h, k) = self.cursor_line().unwrap_or_else(|| {
            let h = 0;
            (h, hunks.first().and_then(|x| x.lines.iter().position(|l| l.kind != Kind::Context)).unwrap_or(0))
        });
        let lines = &hunks.get(h)?.lines;
        let after = lines.iter().skip(k).map(|l| l.new);
        let before = lines.iter().take(k).rev().map(|l| l.new);
        after.chain(before).find(|&n| n > 0).map(|n| n as usize)
    }

    /// `l`: the keys go to the selected file's lines. Err with the word
    /// saying why not.
    pub fn enter_lines(&mut self) -> Result<(), &'static str> {
        let n = match self.diff() {
            None => return Err("changes.reading"),
            Some(Ok(Some(s))) if s.diff.patch.is_some() && !s.changed.is_empty() => s.changed.len(),
            Some(Ok(Some(s))) if s.diff.patch.is_some() => return Err("changes.no_lines"),
            _ => return Err("changes.whole"),
        };
        self.focus = Focus::Lines;
        self.cursor = self.cursor.min(n - 1);
        self.told = None;
        Ok(())
    }

    /// `s`: the selected file's hunks cut into their runs of changes, a
    /// piece each to look at and to pick by itself, the keys going to its
    /// lines; or, when they are cut, put back as git wrote them (read
    /// again). Err with the word saying why not.
    pub fn split(&mut self) -> Result<Option<Wish>, &'static str> {
        self.enter_lines()?;
        let key = self.selected().map(|(side, f)| (side, f.path.clone()));
        if key.is_some() && self.split == key {
            self.split = None;
            self.file_asked = None;
            return Ok(self.want_file());
        }
        let Some((_, _, Ok(Some(s)))) = &mut self.shown else { return Err("changes.reading") };
        let mut diff = s.diff.clone();
        if !diff.split() {
            return Err("changes.no_split");
        }
        **s = Shown::of(diff);
        self.split = key;
        Ok(None)
    }

    /// Whether the selected file's diff has lines to pick.
    pub fn can_pick(&self) -> bool {
        self.shown().is_some_and(|s| s.diff.patch.is_some() && !s.changed.is_empty())
    }

    /// Back to the files.
    pub fn leave_lines(&mut self) {
        self.focus = Focus::Files;
        (self.cursor, self.anchor) = (0, None);
    }

    /// Moves the cursor over the changed lines.
    pub fn move_cursor(&mut self, by: i64) {
        let Some(n) = self.shown().map(|s| s.changed.len()) else { return };
        self.cursor = (self.cursor as i64 + by).clamp(0, n as i64 - 1) as usize;
    }

    /// `n` / `N` on the lines: the cursor goes to the first line of the next
    /// (previous) run of changed lines.
    pub fn step_run(&mut self, dir: i32) {
        let Some(s) = self.shown() else { return };
        self.cursor = michi_core::minimap::step_line(&s.changed, self.cursor, dir);
    }

    /// `v`: a range starts at the cursor, or the one started goes.
    pub fn toggle_range(&mut self) {
        self.anchor = if self.anchor.is_some() { None } else { Some(self.cursor) };
    }

    /// `a` on the lines: the cursor's hunk is picked whole, or no longer.
    pub fn pick_hunk(&mut self) {
        let Some(s) = self.shown() else { return };
        let Some(&(h, _)) = s.changed.get(self.cursor) else { return };
        let first = s.changed.iter().position(|&(hh, _)| hh == h).unwrap_or(0);
        let last = s.changed.iter().rposition(|&(hh, _)| hh == h).unwrap_or(first);
        if self.anchor == Some(first) && self.cursor == last {
            self.anchor = None;
        } else {
            (self.anchor, self.cursor) = (Some(first), last);
        }
    }

    /// Nothing is on the way that the view would act on as it is not.
    fn settled(&self) -> bool {
        self.pending == 0 && !self.list_waiting
    }

    /// Space on the files: the selected file in, or out when it is staged.
    pub fn toggle_file(&mut self, claude: bool) -> Result<Option<Wish>, &'static str> {
        let Some((side, _)) = self.selected() else { return Ok(None) };
        let mut paths = Vec::new();
        for k in self.marked() {
            if let Some((_, f)) = self.at(k) {
                paths.push(f.path.clone());
                if side == Side::Staged {
                    paths.extend(f.from.clone());
                }
            }
        }
        self.mark = None;
        self.picks.clear();
        let op = match side {
            Side::Unstaged => Op::Stage(paths),
            Side::Staged => Op::Unstage(paths),
        };
        Ok(self.act(op, claude))
    }

    /// `a` on the files: everything in, or everything out when nothing is
    /// left to stage.
    pub fn toggle_all(&mut self, claude: bool) -> Result<Option<Wish>, &'static str> {
        let l = self.need_lists()?;
        let op = match (l.unstaged.is_empty(), l.staged.is_empty()) {
            (false, _) => Op::Stage(Vec::new()),
            (true, false) => Op::Unstage(Vec::new()),
            (true, true) => return Err("changes.clean"),
        };
        Ok(self.act(op, claude))
    }

    /// Space on the lines: the picked lines in, or out when they are staged.
    /// The cursor goes to the range's start, where the next line comes when
    /// the diff is read again.
    pub fn apply_lines(&mut self, claude: bool) -> Result<Option<Wish>, &'static str> {
        if !self.settled() || self.file_waiting || self.shown().is_none() {
            return Err("changes.busy");
        }
        let side = self.selected().map(|(s, _)| s);
        let picked: HashSet<(usize, usize)> = self.picked().into_iter().collect();
        let reverse = side == Some(Side::Staged);
        let Some(patch) = self.shown().and_then(|s| s.diff.patch.as_ref()?.pick(|h, k| picked.contains(&(h, k)), reverse)) else {
            return Ok(None);
        };
        self.cursor = self.anchor.take().map_or(self.cursor, |a| a.min(self.cursor));
        Ok(self.act(Op::Apply { patch, reverse }, claude))
    }

    /// `d`: the unstaged changes of the selected file thrown away, or on
    /// the lines those of the picked lines; asked first. Err with the word
    /// saying why not.
    pub fn discard(&mut self, claude: bool) -> Result<Option<Wish>, &'static str> {
        if !self.settled() {
            return Err("changes.busy");
        }
        let Some((side, f)) = self.selected() else { return Ok(None) };
        if side == Side::Staged {
            return Err("changes.discard_staged");
        }
        let (path, untracked) = (f.path.clone(), f.status == Status::Untracked);
        let op = if self.focus == Focus::Lines {
            if self.file_waiting || self.shown().is_none() {
                return Err("changes.busy");
            }
            let picked: HashSet<(usize, usize)> = self.picked().into_iter().collect();
            let Some(patch) = self.shown().and_then(|s| s.diff.patch.as_ref()?.pick(|h, k| picked.contains(&(h, k)), true)) else {
                return Ok(None);
            };
            Op::Discard { path, untracked: false, patch: Some(patch), lines: picked.len() }
        } else {
            Op::Discard { path, untracked, patch: None, lines: 0 }
        };
        Ok(self.act(op, claude))
    }

    /// `i`: the selected untracked file added to the worktree's `.gitignore`.
    /// No question: the line is easy to take out. Err with the word saying
    /// why not.
    pub fn ignore(&mut self, claude: bool) -> Result<Option<Wish>, &'static str> {
        if !self.settled() {
            return Err("changes.busy");
        }
        let Some((side, f)) = self.selected() else { return Ok(None) };
        if side == Side::Staged || f.status != Status::Untracked {
            return Err("changes.ignore_untracked");
        }
        let op = Op::Ignore { path: f.path.clone() };
        Ok(self.act(op, claude))
    }

    /// `A`: the next commit takes the head's place, or no longer. Its
    /// message comes to amend; one filled in and not changed goes again.
    pub fn toggle_amend(&mut self) -> Option<Wish> {
        self.amend = !self.amend;
        if self.amend {
            return self.ask_last();
        }
        if self.filled.take().is_some_and(|m| m == self.message) {
            self.message.clear();
        }
        None
    }

    fn ask_last(&mut self) -> Option<Wish> {
        let wt = self.wt.clone()?;
        self.last_gen = next_gen();
        self.pushed = None;
        Some(Wish::Last { gen: self.last_gen, wt })
    }

    /// Ctrl+Enter: commits what is staged with the message. Err with the
    /// word saying why not.
    pub fn commit(&mut self, claude: bool) -> Result<Option<Wish>, &'static str> {
        let l = self.need_lists()?;
        if !self.settled() {
            return Err("changes.busy");
        }
        if self.message.trim().is_empty() {
            return Err("changes.no_message");
        }
        if self.amend && l.head.is_none() {
            return Err("changes.no_head");
        }
        if !self.amend && l.staged.is_empty() {
            return Err("changes.nothing_staged");
        }
        if self.amend && self.pushed.is_none() {
            return Err("changes.reading");
        }
        let op = Op::Commit { message: self.co_author.apply(&self.message), amend: self.amend };
        let (branch, notes) = self.look(l);
        Ok(self.act_with(op, claude, branch, notes))
    }

    /// Before a commit: the branch when it is one to keep clear of commits,
    /// and the staged files worth a word.
    fn look(&self, l: &Lists) -> (Option<String>, Vec<Note>) {
        let branch = l.branch.clone().filter(|b| guard::is_protected(b, &self.guard.protect));
        let notes = l.notes.iter().filter(|n| n.counts(self.guard.large)).cloned().collect();
        (branch, notes)
    }

    /// The write `op`, or a question first: a Claude writes in the worktree
    /// (`claude`) and no yes was given for it, the commit to amend is a
    /// remote's too, or it throws changes away.
    fn act(&mut self, op: Op, claude: bool) -> Option<Wish> {
        self.act_with(op, claude, None, Vec::new())
    }

    /// [`act`](Self::act) for a commit that is on a protected `branch` or
    /// has `notes` to tell of.
    fn act_with(&mut self, op: Op, claude: bool, branch: Option<String>, notes: Vec<Note>) -> Option<Wish> {
        let claude = claude && !self.claude_ok;
        let pushed = op.is_commit() && self.amend && self.pushed == Some(true);
        if claude || pushed || branch.is_some() || !notes.is_empty() || matches!(op, Op::Discard { .. }) {
            self.confirm = Some(Confirm { op, claude, pushed, branch, notes });
            return None;
        }
        self.send(op)
    }

    fn send(&mut self, op: Op) -> Option<Wish> {
        let wt = self.wt.clone()?;
        self.pending += 1;
        Some(Wish::Do { wt, op })
    }

    /// `y` to the question: the write goes, and a yes for the Claude holds.
    pub fn yes(&mut self) -> Option<Wish> {
        let c = self.confirm.take()?;
        if c.claude {
            self.claude_ok = true;
        }
        if matches!(c.op, Op::Discard { patch: Some(_), .. }) {
            // As for lines staged: the cursor to the range's start.
            self.cursor = self.anchor.take().map_or(self.cursor, |a| a.min(self.cursor));
        }
        self.send(c.op)
    }

    /// The Claude in the worktree stopped writing: the next write asks again.
    pub fn claude_stopped(&mut self) {
        self.claude_ok = false;
    }

    /// The reads asked last, as an answer carries them back.
    #[cfg(test)]
    pub fn gens(&self) -> (u32, u32, u32) {
        (self.list_gen, self.file_gen, self.last_gen)
    }
}

/// The place of side `side`'s `k`th file over both sides.
fn flat(l: &Lists, side: Side, k: usize) -> usize {
    match side {
        Side::Staged => k,
        Side::Unstaged => l.staged.len() + k,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use michi_core::oid::Oid;
    use michi_core::show::{Hunk, Line, Status};

    const WT: &str = "C:/r/wt";

    fn oid(n: u8) -> Oid {
        Oid::parse(&format!("{n:02x}").repeat(20)).unwrap()
    }

    pub(crate) fn file(path: &str) -> File {
        File {
            status: Status::Modified,
            path: path.to_owned(),
            from: None,
            blob: None,
            counts: Some((1, 0)),
            hunks: Vec::new(),
            eol_only: false,
            cut: false,
            not_utf8: false,
        }
    }

    pub(crate) fn lists(staged: &[&str], unstaged: &[&str]) -> Lists {
        Lists {
            staged: staged.iter().map(|p| file(p)).collect(),
            unstaged: unstaged.iter().map(|p| file(p)).collect(),
            cut: false,
            head: Some(oid(1)),
            branch: None,
            notes: Vec::new(),
        }
    }

    fn line(kind: Kind, text: &str) -> Line {
        Line { kind, old: 1, new: 1, text: text.into(), marks: Vec::new(), ink: Vec::new() }
    }

    /// `a.txt` with two hunks: a context line and two added lines, then a
    /// deleted line and an added one; its patch as git writes it.
    pub(crate) fn two_hunks(path: &str) -> stage::Diff {
        let mut f = file(path);
        f.hunks = vec![
            Hunk { header: "@@ -1,1 +1,3 @@".into(), lines: vec![line(Kind::Context, "a"), line(Kind::Add, "b"), line(Kind::Add, "c")] },
            Hunk { header: "@@ -9,1 +11,1 @@".into(), lines: vec![line(Kind::Del, "x"), line(Kind::Add, "y")] },
        ];
        let patch =
            format!("diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n@@ -1,1 +1,3 @@\n a\n+b\n+c\n@@ -9,1 +11,1 @@\n-x\n+y\n");
        stage::Diff { file: f, patch: Some(stage::Patch::parse(patch.as_bytes())) }
    }

    /// `b.txt` with one hunk of two runs, `b` and `c` between them.
    fn two_runs(path: &str) -> stage::Diff {
        let mut f = file(path);
        let lines = [(Kind::Del, "a"), (Kind::Add, "A"), (Kind::Context, "b"), (Kind::Context, "c"), (Kind::Del, "d"), (Kind::Add, "D")];
        f.hunks = vec![Hunk { header: "@@ -1,4 +1,4 @@".into(), lines: lines.iter().map(|&(k, t)| line(k, t)).collect() }];
        let patch = format!("diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n@@ -1,4 +1,4 @@\n-a\n+A\n b\n c\n-d\n+D\n");
        stage::Diff { file: f, patch: Some(stage::Patch::parse(patch.as_bytes())) }
    }

    fn list_gen(w: &[Wish]) -> u32 {
        match w {
            [Wish::List { gen, .. }] => *gen,
            _ => panic!("no list wished: {w:?}"),
        }
    }

    fn file_gen(w: &[Wish]) -> u32 {
        w.iter()
            .find_map(|w| match w {
                Wish::File { gen, .. } => Some(*gen),
                _ => None,
            })
            .unwrap_or_else(|| panic!("no diff wished: {w:?}"))
    }

    /// The files read again as `l`.
    fn reread(c: &mut Changes, l: Lists) -> Vec<Wish> {
        let gen = list_gen(&Vec::from_iter(c.refresh()));
        c.take_list(gen, Ok(l))
    }

    /// The view on `C:/r/wt` with `s.txt` staged, `a.txt` and `b.txt` not,
    /// and `a.txt` selected with its diff read.
    fn opened() -> Changes {
        let mut c = Changes::default();
        let w = c.open(PathBuf::from(WT));
        let ws = c.take_list(list_gen(&Vec::from_iter(w)), Ok(lists(&["s.txt"], &["a.txt", "b.txt"])));
        assert_eq!(c.selected().unwrap().1.path, "s.txt");
        c.take_file(file_gen(&ws), Ok(Some(two_hunks("s.txt"))));
        let w = c.move_by(1);
        c.take_file(file_gen(&Vec::from_iter(w)), Ok(Some(two_hunks("a.txt"))));
        c
    }

    #[test]
    fn the_editor_opens_at_the_cursor_line_or_the_first_change() {
        let mut c = Changes::default();
        let w = c.open(PathBuf::from(WT));
        let ws = c.take_list(list_gen(&Vec::from_iter(w)), Ok(lists(&[], &["a.txt"])));
        assert_eq!(c.editor_line(), None);
        let mut d = two_hunks("a.txt");
        let news = [4, 5, 0, 6];
        let kinds = [Kind::Context, Kind::Add, Kind::Del, Kind::Add];
        d.file.hunks = vec![Hunk {
            header: "@@ -1,3 +4,3 @@".into(),
            lines: kinds.iter().zip(news).map(|(&k, n)| Line { new: n, ..line(k, "t") }).collect(),
        }];
        c.take_file(file_gen(&ws), Ok(Some(d)));
        // Not in the lines: the first changed line.
        assert_eq!(c.editor_line(), Some(5));
        c.enter_lines().unwrap();
        c.move_cursor(1);
        // On a deleted line: the next line that exists.
        assert_eq!(c.editor_line(), Some(6));
        c.move_cursor(1);
        assert_eq!(c.editor_line(), Some(6));
    }

    #[test]
    fn older_answers_are_dropped_and_the_diff_is_kept_while_read_again() {
        let mut c = Changes::default();
        let old = list_gen(&Vec::from_iter(c.open(PathBuf::from(WT))));
        let new = list_gen(&Vec::from_iter(c.refresh()));
        assert_eq!(c.take_list(old, Ok(lists(&["old.txt"], &[]))), []);
        assert!(c.lists.is_none());
        let ws = c.take_list(new, Ok(lists(&[], &["a.txt"])));
        let Some(Wish::File { side, file, gen, .. }) = ws.first() else { panic!("{ws:?}") };
        assert_eq!((*side, file.path.as_str()), (Side::Unstaged, "a.txt"));
        c.take_file(gen.wrapping_sub(1), Ok(None));
        assert!(c.diff().is_none());
        c.take_file(*gen, Ok(Some(two_hunks("a.txt"))));
        assert!(matches!(c.diff(), Some(Ok(Some(s))) if s.changed == [(0, 1), (0, 2), (1, 0), (1, 1)]));
        // Read again: the old diff shows until the new one comes.
        let ws = reread(&mut c, lists(&[], &["a.txt"]));
        assert!(c.diff().is_some());
        assert_eq!(c.gens().1, file_gen(&ws));
    }

    #[test]
    fn the_selection_stays_on_its_file_else_at_its_place() {
        let mut c = opened();
        // a.txt staged: it moved; the selection stays at its place, on b.txt.
        let ws = reread(&mut c, lists(&["a.txt", "s.txt"], &["b.txt"]));
        assert_eq!(c.selected().map(|(s, f)| (s, f.path.as_str())), Some((Side::Unstaged, "b.txt")));
        assert!(c.to_top);
        assert!(matches!(&ws[..], [Wish::File { file, .. }] if file.path == "b.txt"));
        // b.txt staged too: nothing left unstaged, so it is followed.
        reread(&mut c, lists(&["a.txt", "b.txt", "s.txt"], &[]));
        assert_eq!(c.selected().map(|(s, f)| (s, f.path.as_str())), Some((Side::Staged, "b.txt")));
        // A file comes before it: still b.txt.
        reread(&mut c, lists(&["0.txt", "a.txt", "b.txt", "s.txt"], &[]));
        assert_eq!(c.sel, 2);
        // All gone.
        reread(&mut c, lists(&[], &[]));
        assert_eq!((c.sel, c.selected()), (0, None));
        assert_eq!(c.select(3), None);
    }

    #[test]
    fn shift_click_marks_a_range_and_ctrl_click_adds_one_by_one() {
        let mut c = opened();
        reread(&mut c, lists(&["s.txt"], &["a.txt", "b.txt", "c.txt", "d.txt"]));
        c.select(1);
        c.click_range(3);
        assert!((1..=3).all(|k| c.is_marked(k)) && !c.is_marked(4));
        c.select(1);
        c.click_toggle(3);
        c.click_toggle(4);
        assert!(c.is_marked(1) && !c.is_marked(2) && c.is_marked(3) && c.is_marked(4));
        c.click_toggle(3);
        assert_eq!(c.marked(), vec![1, 4]);
        // Not across to the staged side.
        c.click_toggle(0);
        assert_eq!(c.marked(), vec![0]);
        assert!(matches!(c.toggle_file(false), Ok(Some(Wish::Do { op: Op::Unstage(p), .. })) if p == ["s.txt"]));
    }

    #[test]
    fn shift_arrows_mark_a_range_that_space_stages_together() {
        let mut c = opened();
        reread(&mut c, lists(&["s.txt"], &["a.txt", "b.txt", "c.txt"]));
        c.select(1);
        // Not across to the staged side.
        assert!(c.extend(-1).is_none() && !c.is_marked(0) && !c.is_marked(1));
        c.extend(1);
        c.extend(1);
        assert!((1..=3).all(|k| c.is_marked(k)) && !c.is_marked(0));
        assert!(c.extend(1).is_none());
        c.extend(-1);
        assert!(c.is_marked(1) && c.is_marked(2) && !c.is_marked(3));
        assert!(matches!(c.toggle_file(false), Ok(Some(Wish::Do { op: Op::Stage(p), .. })) if p == ["a.txt", "b.txt"]));
        assert!(!c.unmark(), "staging ends the selection");
        c.select(1);
        c.extend(1);
        assert!(c.unmark());
        assert!(!c.is_marked(2));
    }

    #[test]
    fn space_stages_or_unstages_the_file_and_a_does_all() {
        let mut c = opened();
        assert_eq!(c.toggle_file(false), Ok(Some(Wish::Do { wt: WT.into(), op: Op::Stage(vec!["a.txt".into()]) })));
        c.select(0);
        assert!(matches!(c.toggle_file(false), Ok(Some(Wish::Do { op: Op::Unstage(p), .. })) if p == ["s.txt"]));
        assert!(matches!(c.toggle_all(false), Ok(Some(Wish::Do { op: Op::Stage(p), .. })) if p.is_empty()));
        reread(&mut c, lists(&["a.txt"], &[]));
        assert!(matches!(c.toggle_all(false), Ok(Some(Wish::Do { op: Op::Unstage(p), .. })) if p.is_empty()));
        reread(&mut c, lists(&[], &[]));
        assert_eq!(c.toggle_all(false), Err("changes.clean"));
    }

    #[test]
    fn n_and_n_move_the_cursor_over_runs_of_changed_lines() {
        let mut c = opened();
        assert_eq!(c.enter_lines(), Ok(()));
        // two_hunks: (0,1)(0,2) then (1,0)(1,1).
        assert_eq!(c.cursor_line(), Some((0, 1)));
        c.step_run(1);
        assert_eq!(c.cursor_line(), Some((1, 0)));
        c.step_run(1);
        assert_eq!(c.cursor_line(), Some((1, 0)), "no run after the last");
        c.move_cursor(1);
        c.step_run(-1);
        assert_eq!(c.cursor_line(), Some((1, 0)), "back to the start of its own run");
        c.step_run(-1);
        assert_eq!(c.cursor_line(), Some((0, 1)));
    }

    #[test]
    fn lines_are_picked_by_range_and_by_hunk_and_staged_by_a_patch() {
        let mut c = opened();
        assert_eq!(c.enter_lines(), Ok(()));
        assert_eq!(c.cursor_line(), Some((0, 1)));
        c.move_cursor(1);
        assert_eq!(c.picked(), [(0, 2)]);
        c.toggle_range();
        c.move_cursor(5);
        assert_eq!(c.picked(), [(0, 2), (1, 0), (1, 1)]);
        c.pick_hunk();
        assert_eq!(c.picked(), [(1, 0), (1, 1)]);
        c.pick_hunk();
        assert_eq!((c.anchor, c.picked()), (None, vec![(1, 1)]));
        c.move_cursor(-3);
        let ws = c.apply_lines(false);
        let Ok(Some(Wish::Do { op: Op::Apply { patch, reverse: false }, .. })) = ws else { panic!("{ws:?}") };
        let patch = String::from_utf8(patch).unwrap();
        assert!(patch.ends_with("@@ -1,1 +1,3 @@\n a\n+b\n"), "{patch}");
        // Until it is done and read again, the lines are not picked from.
        assert_eq!(c.apply_lines(false), Err("changes.busy"));
        let w = c.take_done(Path::new(WT), false, true);
        assert_eq!(c.apply_lines(false), Err("changes.busy"));
        let ws = c.take_list(list_gen(&Vec::from_iter(w)), Ok(lists(&["a.txt", "s.txt"], &["a.txt", "b.txt"])));
        assert_eq!(c.focus, Focus::Lines);
        assert_eq!(c.apply_lines(false), Err("changes.busy"));
        c.take_file(file_gen(&ws), Ok(Some(two_hunks("a.txt"))));
        c.move_cursor(2);
        c.toggle_range();
        c.move_cursor(1);
        assert!(c.apply_lines(false).is_ok());
        assert_eq!((c.cursor, c.anchor), (2, None));
    }

    #[test]
    fn s_splits_the_hunks_keeps_them_split_as_read_again_and_joins_them() {
        let mut c = opened();
        assert_eq!(c.split(), Err("changes.no_split"), "a.txt has one run in each hunk");
        let w = c.move_by(1);
        c.take_file(file_gen(&Vec::from_iter(w)), Ok(Some(two_runs("b.txt"))));
        assert_eq!(c.split(), Ok(None));
        assert_eq!(c.focus, Focus::Lines);
        let hunks = |c: &Changes| match c.diff() {
            Some(Ok(Some(s))) => s.diff.file.hunks.iter().map(|h| h.header.clone()).collect::<Vec<_>>(),
            _ => panic!("no diff"),
        };
        assert_eq!(hunks(&c), ["@@ -1,2 +1,2 @@", "@@ -3,2 +3,2 @@"]);
        // `a` picks a piece; the patch is the hunk git wrote.
        c.move_cursor(2);
        c.pick_hunk();
        assert_eq!(c.picked(), [(1, 1), (1, 2)]);
        let ws = c.apply_lines(false);
        let Ok(Some(Wish::Do { op: Op::Apply { patch, .. }, .. })) = ws else { panic!("{ws:?}") };
        assert!(String::from_utf8(patch).unwrap().ends_with("@@ -1,4 +1,4 @@\n a\n b\n c\n-d\n+D\n"));
        // Read again, it is cut again.
        let w = c.take_done(Path::new(WT), false, true);
        let ws = c.take_list(list_gen(&Vec::from_iter(w)), Ok(lists(&["s.txt"], &["a.txt", "b.txt"])));
        c.take_file(file_gen(&ws), Ok(Some(two_runs("b.txt"))));
        assert_eq!(hunks(&c).len(), 2);
        // `s` again: read again as git writes it.
        let w = c.split();
        let Ok(Some(Wish::File { gen, .. })) = w else { panic!("{w:?}") };
        c.take_file(gen, Ok(Some(two_runs("b.txt"))));
        assert_eq!(hunks(&c), ["@@ -1,4 +1,4 @@"]);
        assert_eq!(c.focus, Focus::Lines);
    }

    #[test]
    fn staged_lines_go_out_by_the_patch_reversed() {
        let mut c = opened();
        let w = c.select(0);
        c.take_file(file_gen(&Vec::from_iter(w)), Ok(Some(two_hunks("s.txt"))));
        c.enter_lines().unwrap();
        let ws = c.apply_lines(false);
        assert!(matches!(ws, Ok(Some(Wish::Do { op: Op::Apply { reverse: true, .. }, .. }))), "{ws:?}");
    }

    #[test]
    fn a_whole_file_has_no_lines_to_pick() {
        let mut c = opened();
        let w = c.move_by(1);
        let mut f = file("b.txt");
        f.status = Status::Untracked;
        c.take_file(file_gen(&Vec::from_iter(w)), Ok(Some(stage::Diff { file: f, patch: None })));
        assert_eq!(c.enter_lines(), Err("changes.whole"));
        assert_eq!(c.focus, Focus::Files);
        // The file went in the meantime: back to the files.
        let w = c.move_by(-1);
        assert_eq!(c.enter_lines(), Err("changes.reading"));
        c.take_file(file_gen(&Vec::from_iter(w)), Ok(Some(two_hunks("a.txt"))));
        c.enter_lines().unwrap();
        let ws = reread(&mut c, lists(&["s.txt"], &["b.txt"]));
        assert_eq!((c.focus, c.selected().unwrap().1.path.as_str()), (Focus::Files, "b.txt"));
        c.take_file(file_gen(&ws), Ok(None));
        assert!(matches!(c.diff(), Some(Ok(None))));
    }

    #[test]
    fn a_claude_writing_asks_first_and_the_yes_holds() {
        let mut c = opened();
        assert_eq!(c.toggle_file(true), Ok(None));
        assert!(matches!(&c.confirm, Some(Confirm { claude: true, pushed: false, .. })));
        assert!(matches!(c.yes(), Some(Wish::Do { op: Op::Stage(_), .. })));
        assert!(c.confirm.is_none() && c.claude_ok);
        assert!(c.toggle_file(true).unwrap().is_some());
        c.claude_stopped();
        assert_eq!(c.toggle_file(true), Ok(None));
        c.confirm = None;
        // Another worktree asks again.
        c.claude_ok = true;
        c.open("C:/r/other".into());
        assert!(!c.claude_ok);
    }

    #[test]
    fn d_throws_away_a_file_or_the_picked_lines_after_a_yes() {
        let mut c = opened();
        // A file: asked first, always.
        assert_eq!(c.discard(false), Ok(None));
        let discard = |path: &str, untracked| Op::Discard { path: path.into(), untracked, patch: None, lines: 0 };
        assert_eq!(c.confirm, Some(Confirm { op: discard("a.txt", false), claude: false, pushed: false, branch: None, notes: vec![] }));
        assert_eq!(c.yes(), Some(Wish::Do { wt: WT.into(), op: discard("a.txt", false) }));
        assert_eq!(c.discard(false), Err("changes.busy"));
        let w = c.take_done(Path::new(WT), false, true);
        let mut l = lists(&["s.txt"], &["a.txt", "b.txt"]);
        l.unstaged[1].status = Status::Untracked;
        c.take_list(list_gen(&Vec::from_iter(w)), Ok(l));
        // An untracked one is deleted.
        c.move_by(1);
        c.discard(false).unwrap();
        assert_eq!(c.confirm.take().map(|c| c.op), Some(discard("b.txt", true)));
        // Lines: the picked ones, by the patch that takes them out; a no
        // keeps the range.
        let w = c.move_by(-1);
        c.take_file(file_gen(&Vec::from_iter(w)), Ok(Some(two_hunks("a.txt"))));
        c.enter_lines().unwrap();
        c.move_cursor(1);
        c.toggle_range();
        c.move_cursor(1);
        assert_eq!(c.discard(true), Ok(None));
        let Some(Confirm { op: Op::Discard { path, untracked: false, patch: Some(patch), lines: 2 }, claude: true, .. }) = &c.confirm
        else {
            panic!("{:?}", c.confirm)
        };
        assert_eq!(path, "a.txt");
        let patch = String::from_utf8(patch.clone()).unwrap();
        assert!(patch.contains("\n b\n+c\n") && patch.contains("\n-x\n y\n"), "{patch}");
        c.confirm = None;
        assert_eq!((c.cursor, c.anchor), (2, Some(1)));
        c.discard(false).unwrap();
        assert!(c.yes().is_some());
        assert_eq!((c.cursor, c.anchor), (1, None));
        // Staged files are not thrown away.
        let w = c.take_done(Path::new(WT), false, true);
        c.take_list(list_gen(&Vec::from_iter(w)), Ok(lists(&["s.txt"], &["a.txt"])));
        c.select(0);
        assert_eq!(c.discard(false), Err("changes.discard_staged"));
    }

    #[test]
    fn i_ignores_an_untracked_file_without_asking() {
        let mut c = opened();
        // a.txt is tracked: refused.
        assert_eq!(c.ignore(false), Err("changes.ignore_untracked"));
        let w = c.take_done(Path::new(WT), false, true);
        let mut l = lists(&["s.txt"], &["a.txt", "b.txt"]);
        l.unstaged[1].status = Status::Untracked;
        c.take_list(list_gen(&Vec::from_iter(w)), Ok(l));
        // A staged file is refused too.
        c.select(0);
        assert_eq!(c.ignore(false), Err("changes.ignore_untracked"));
        c.select(2);
        assert_eq!(c.ignore(false), Ok(Some(Wish::Do { wt: WT.into(), op: Op::Ignore { path: "b.txt".into() } })));
        assert!(c.confirm.is_none());
        assert_eq!(c.ignore(false), Err("changes.busy"));
    }

    #[test]
    fn a_commit_needs_a_message_and_something_staged() {
        let mut c = opened();
        assert_eq!(c.commit(false), Err("changes.no_message"));
        c.message = "Add a\n".into();
        let w = c.commit(false);
        assert_eq!(w, Ok(Some(Wish::Do { wt: WT.into(), op: Op::Commit { message: "Add a\n".into(), amend: false } })));
        assert_eq!(c.commit(false), Err("changes.busy"));
        assert!(c.take_done(Path::new(WT), true, true).is_some());
        assert!(c.message.is_empty());
        reread(&mut c, lists(&[], &["b.txt"]));
        c.message = "x".into();
        assert_eq!(c.commit(false), Err("changes.nothing_staged"));
        // A failed commit keeps its message.
        reread(&mut c, lists(&["b.txt"], &[]));
        c.commit(false).unwrap();
        c.take_done(Path::new(WT), true, false);
        assert_eq!(c.message, "x");
    }

    fn commit_on(branch: Option<&str>, notes: Vec<Note>) -> Changes {
        let mut c = opened();
        let mut l = lists(&["a.txt"], &[]);
        l.branch = branch.map(str::to_owned);
        l.notes = notes;
        reread(&mut c, l);
        c.message = "x".into();
        c
    }

    fn note(path: &str, size: u64, secret: Option<guard::Secret>) -> Note {
        Note { path: path.into(), size, secret }
    }

    #[test]
    fn a_commit_on_a_protected_branch_asks_first() {
        let mut c = commit_on(Some("main"), vec![]);
        assert_eq!(c.commit(false), Ok(None));
        assert_eq!(c.confirm.as_ref().and_then(|c| c.branch.as_deref()), Some("main"));
        let w = c.yes();
        assert!(matches!(w, Some(Wish::Do { op: Op::Commit { .. }, .. })));
        // A feature branch, a detached head, and a branch left out go straight.
        for (branch, protect) in [(Some("feat"), vec!["main"]), (None, vec!["main"]), (Some("main"), vec![])] {
            let mut c = commit_on(branch, vec![]);
            c.guard.protect = protect.into_iter().map(str::to_owned).collect();
            assert!(matches!(c.commit(false), Ok(Some(Wish::Do { .. }))), "{branch:?}");
        }
    }

    #[test]
    fn big_files_and_secrets_ask_first_by_the_limit() {
        let key = Some(guard::Secret::Name(".env file"));
        let mut c = commit_on(Some("feat"), vec![note(".env", 10, key), note("big.bin", 60 << 20, None), note("mid.bin", 2 << 20, None)]);
        assert_eq!(c.commit(false), Ok(None));
        let paths: Vec<_> = c.confirm.as_ref().unwrap().notes.iter().map(|n| n.path.as_str()).collect();
        assert_eq!(paths, [".env", "big.bin"]);
        // Esc: nothing sent, and the commit can be tried again.
        c.confirm = None;
        c.guard.large = 0;
        assert_eq!(c.commit(false), Ok(None));
        assert_eq!(c.confirm.as_ref().unwrap().notes.len(), 1);
        // Only a file of a size and no secret goes straight when sizes do not count.
        let mut c = commit_on(Some("feat"), vec![note("big.bin", 60 << 20, None)]);
        c.guard.large = 0;
        assert!(matches!(c.commit(false), Ok(Some(Wish::Do { .. }))));
    }

    #[test]
    fn the_help_is_read_once_fills_a_template_and_steps_through_the_past() {
        use michi_core::msghelp::Help;
        let mut c = opened();
        assert!(!c.step_history(true), "nothing read yet");
        let Some(Wish::Help { gen, .. }) = c.focus_message() else { panic!() };
        assert!(c.focus_message);
        assert_eq!(c.ask_help(), None, "asked once");
        // An answer to an older ask is dropped.
        c.take_help(gen + 1, Help { past: vec!["old".into()], template: None });
        assert!(!c.step_history(true));
        let help = Help {
            past: vec!["Add b".into(), "Add a".into()],
            template: Some(
                "type:
"
                .into(),
            ),
        };
        c.take_help(gen, help);
        assert_eq!(
            c.message,
            "type:
",
            "the template fills an empty box"
        );
        c.message = "wip".into();
        assert!(c.step_history(true));
        assert_eq!(c.message, "Add b");
        assert!(c.step_history(true));
        assert_eq!(c.message, "Add a");
        assert!(!c.step_history(true));
        assert!(c.step_history(false) && c.step_history(false));
        assert_eq!(c.message, "wip");
        // A commit lets the help go: it is read again for the next one.
        c.commit(false).unwrap();
        c.take_done(Path::new(WT), true, true);
        assert!(matches!(c.ask_help(), Some(Wish::Help { .. })));
    }

    #[test]
    fn an_amend_fills_the_head_s_message_and_asks_when_it_is_pushed() {
        let mut c = opened();
        let Some(Wish::Last { gen, .. }) = c.toggle_amend() else { panic!() };
        assert!(c.amend);
        c.message = String::new();
        assert_eq!(c.commit(false), Err("changes.no_message"));
        c.take_last(gen, Ok(("Add a".into(), true)));
        assert_eq!((c.message.as_str(), c.pushed), ("Add a", Some(true)));
        assert_eq!(c.commit(false), Ok(None));
        let Some(Confirm { op: Op::Commit { amend: true, .. }, pushed: true, claude: false, .. }) = &c.confirm else {
            panic!("{:?}", c.confirm)
        };
        assert!(c.yes().is_some());
        // Off again: the message filled in goes; one typed stays.
        c.take_done(Path::new(WT), true, true);
        let Some(Wish::Last { gen, .. }) = c.toggle_amend() else { panic!() };
        c.take_last(gen, Ok(("Add a".into(), false)));
        assert_eq!(c.toggle_amend(), None);
        assert!(c.message.is_empty());
        c.message = "typed".into();
        let Some(Wish::Last { gen, .. }) = c.toggle_amend() else { panic!() };
        c.take_last(gen, Ok(("Add a".into(), false)));
        c.toggle_amend();
        assert_eq!(c.message, "typed");
    }

    #[test]
    fn an_amend_before_the_first_commit_is_refused() {
        let mut c = Changes::default();
        let w = c.open("C:/r/new".into());
        let mut l = lists(&["a.txt"], &[]);
        l.head = None;
        c.take_list(list_gen(&Vec::from_iter(w)), Ok(l));
        c.toggle_amend();
        c.message = "first".into();
        assert_eq!(c.commit(false), Err("changes.no_head"));
    }

    #[test]
    fn each_worktree_keeps_its_message() {
        let mut c = opened();
        c.message = "half written".into();
        c.open("C:/r/other".into());
        assert!(c.message.is_empty());
        c.message = "other's".into();
        c.open(WT.into());
        assert_eq!(c.message, "half written");
        c.open("C:/r/other".into());
        assert_eq!(c.message, "other's");
    }

    #[test]
    fn a_failed_list_asks_nothing() {
        let mut c = Changes::default();
        let w = c.open(WT.into());
        assert_eq!(c.take_list(list_gen(&Vec::from_iter(w)), Err(git::Error::Git("bad".into()))), []);
        assert_eq!((c.count(), c.selected()), (0, None));
        assert_eq!(c.toggle_file(false), Ok(None));
        assert_eq!(c.toggle_all(false), Err("changes.unread"));
    }

    #[test]
    fn the_tree_walks_in_its_own_order_stages_folders_whole_and_skips_folded_files() {
        let mut c = Changes::default();
        let w = c.open(PathBuf::from(WT));
        c.take_list(list_gen(&Vec::from_iter(w)), Ok(lists(&["src/m.rs"], &["z.txt", "src/b.rs", "src/a/c.rs"])));
        c.toggle_tree();
        // Staged: src/m.rs (0). Unstaged, as the tree has them: src/a/c.rs, src/b.rs, z.txt.
        assert_eq!(c.order_of(true), [0, 3, 2, 1]);
        c.select(0);
        c.move_by(1);
        assert_eq!(c.sel, 3);
        c.move_by(1);
        assert_eq!(c.sel, 2);
        assert_eq!(
            c.toggle_dir(Side::Unstaged, "src", false),
            Some(Wish::Do { wt: WT.into(), op: Op::Stage(vec!["src/b.rs".into(), "src/a/c.rs".into()]) })
        );
        assert!(matches!(c.toggle_dir(Side::Staged, "src", false), Some(Wish::Do { op: Op::Unstage(p), .. }) if p == ["src/m.rs"]));
        assert_eq!(c.toggle_dir(Side::Unstaged, "nothing", false), None);
        // Folded: its files are not walked to, and a selection folded away moves on.
        c.fold(Side::Unstaged, "src");
        assert_eq!(c.order_of(true), [0, 1]);
        c.move_by(-1);
        assert_eq!(c.sel, 0);
        c.select(2);
        c.move_by(1);
        assert_eq!(c.sel, 1);
        c.fold(Side::Unstaged, "src");
        assert_eq!(c.order_of(true), [0, 3, 2, 1]);
        c.toggle_tree();
        assert_eq!(c.order_of(true), [0, 1, 2, 3]);
    }
}
