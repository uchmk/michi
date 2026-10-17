//! The graph view's state, with nothing of egui in it: the history as it is
//! read, laid out with the folding in force, the commit selected, the filter
//! and what it hits, and when to read more.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Datelike, TimeZone, Timelike};
use ito_graph::{Colors, Dot, Layout, Placed};
use michi_core::git;
use michi_core::history::{merged_name, Fold, History, Row, Setup};
use michi_core::log::Commit;
use michi_core::oid::Oid;
use michi_core::wip::Wip;

/// The trunk's colour, cyan, as in the mock.
pub const TRUNK_COLOR: u8 = 6;
/// Read more when the rows in sight come this close to the end.
const AHEAD: usize = 300;

/// A mark on the scrollbar (the selected row's is the window's).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// The filter hits it.
    Hit,
    /// A tag points at it.
    Tag,
    /// A worktree's head, in its branch's colour.
    Head(u8),
    /// A worktree's uncommitted change.
    Wip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PillKind {
    /// A branch.
    Local,
    /// A branch a worktree is on, or a detached worktree's folder.
    Worktree,
    /// A remote branch with no branch of its name on the same commit.
    Remote,
    Tag,
}

/// A name on a row, before its subject.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pill {
    pub kind: PillKind,
    /// `feat`, `origin/feat`, a worktree's folder, `v1.0`.
    pub text: String,
    /// The branch's colour; a tag's is the theme's, not this.
    pub color: u8,
    /// The remotes with a branch of the same name on the same commit
    /// (` + origin`).
    pub also: Vec<String>,
    /// A worktree's folder, for the tooltip.
    pub path: Option<String>,
}

/// The chip of a trunk merge that folds something away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chip<'a> {
    /// The commits it folds away.
    pub hidden: u32,
    pub open: bool,
    /// The branches and remote branches among them.
    pub names: &'a [String],
    /// The merged branch's colour, when its subject names it.
    pub color: Option<u8>,
}

/// How the graph shows a trunk merge's commits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoldState {
    /// Folded away; `l` opens them.
    Folded,
    /// Opened by hand; `h` folds them again.
    Open,
    /// Shown, folding being off (or nothing of it folded).
    Off,
}

/// What `z` did, for the toast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Folded {
    /// Folded `merges` merges on the trunk, `commits` commits in all.
    On {
        merges: usize,
        commits: u64,
    },
    /// Folding is on, but no merge has anything to fold (yet).
    Nothing,
    Off,
}

pub struct Graph {
    gen: u32,
    started: bool,
    /// A reading started again (the refs moved): its setup, kept until its
    /// first batch comes, so the old history stays on screen till then.
    next: Option<Setup>,
    history: History,
    fold: Fold,
    layout: Layout<Oid>,
    colors: Colors,
    /// The names `Colors::pin` has had (it counts each call).
    pinned: HashSet<String>,
    /// The worktrees with uncommitted changes, the last written first: the
    /// rows at the top.
    wips: Vec<Wip>,
    /// The history's row for each row shown under the WIP rows, in order.
    shown: Vec<u32>,
    /// How many of `shown` the layout should hold at least: the rows down
    /// to the lowest one in sight so far. The rest is laid out when it
    /// comes in sight, so folding or a hit near the top does not lay out
    /// a long history to its end.
    want: usize,
    done: bool,
    total: Option<u64>,
    error: Option<git::Error>,
    selected: Option<Oid>,
    /// The commit marked by Ctrl+click, to compare the selected one with.
    marked: Option<Oid>,
    /// The selection was moved by a key or the mouse (not by the history
    /// coming in), and the window has not taken note yet.
    picked: bool,
    /// A batch was asked for and has not come.
    asking: bool,
    filter: String,
    /// The filter, trimmed and in lower case.
    query: String,
    /// The history's rows the filter hits, in order.
    hits: Vec<u32>,
    /// What the filter looks in, for each of the history's rows read so
    /// far: made as they are read, so the filter's first key is quick.
    hay: Vec<Hay>,
    marks: Vec<(u32, Mark)>,
    /// How many rows are in sight (the window sets it), for Page Up/Down.
    pub page: u32,
    /// The selected row should be scrolled into sight.
    pub reveal: bool,
    /// The filter's field should take the keyboard.
    pub focus_filter: bool,
}

impl Default for Graph {
    fn default() -> Self {
        Graph {
            gen: 0,
            started: false,
            next: None,
            history: History::default(),
            fold: Fold { on: true, open: HashSet::new() },
            layout: Layout::new(),
            colors: Colors::default(),
            pinned: HashSet::new(),
            wips: Vec::new(),
            shown: Vec::new(),
            want: WANT,
            done: false,
            total: None,
            error: None,
            selected: None,
            marked: None,
            picked: false,
            asking: false,
            filter: String::new(),
            query: String::new(),
            hits: Vec::new(),
            hay: Vec::new(),
            marks: Vec::new(),
            page: 20,
            reveal: false,
            focus_filter: false,
        }
    }
}

impl Graph {
    /// A reading of the history starts. The folding, the selection and the
    /// filter stay; an answer of an older reading is dropped. A reading
    /// again keeps the history on screen until its first batch comes.
    pub fn start(&mut self, gen: u32, setup: Setup) {
        if gen < self.gen {
            return;
        }
        self.gen = gen;
        self.asking = false;
        if self.started && self.error.is_none() {
            self.next = Some(setup);
        } else {
            self.begin(setup);
        }
    }

    /// The history is now read against `setup`, from its first commit.
    fn begin(&mut self, setup: Setup) {
        self.started = true;
        self.next = None;
        if let Some(t) = &setup.trunk {
            if self.pinned.insert(t.name.clone()) {
                self.colors.pin(&t.name, TRUNK_COLOR);
            }
        }
        self.colors.reserve(setup.worktrees.iter().filter_map(|w| w.branch.as_deref()));
        self.history = History::new(setup);
        self.done = false;
        self.total = None;
        self.error = None;
        self.asking = false;
        self.hits.clear();
        self.hay.clear();
        self.relayout();
    }

    /// The next batch of commits, older than those in.
    pub fn commits(&mut self, gen: u32, batch: Vec<Commit>, done: bool) {
        if gen != self.gen || !self.started {
            return;
        }
        if let Some(setup) = self.next.take() {
            self.begin(setup);
        }
        let from = self.history.len();
        self.history.extend(batch);
        self.fill_hay();
        self.done = done;
        self.asking = false;
        self.hit_from(from);
        self.relayout();
    }

    /// How many commits the whole history has.
    pub fn total(&mut self, gen: u32, count: u64) {
        if gen == self.gen {
            self.total = Some(count);
        }
    }

    pub fn failed(&mut self, gen: u32, error: git::Error) {
        if gen == self.gen {
            if let Some(setup) = self.next.take() {
                self.begin(setup);
            }
            self.error = Some(error);
            self.asking = false;
        }
    }

    /// Lays the history out again from the top: a row read later can change
    /// the edges of those above it (a parent folded away), so nothing laid
    /// out is kept. Only the rows down to `want` are laid out now;
    /// [`lay_out_to`](Self::lay_out_to) does the rest as they come in sight.
    fn relayout(&mut self) {
        let trunk = self.history.setup().trunk.as_ref().map(|t| t.name.as_str());
        let mut layout = if trunk.is_some() { Layout::new() } else { Layout::without_trunk() };
        self.shown.clear();
        // Each WIP row is a temporary commit on its worktree's head (or on
        // the merge that folds the head away).
        for w in &self.wips {
            let head = self.history.find(&w.head).and_then(|i| self.history.hidden_by(i, &self.fold)).unwrap_or(w.head);
            let name = w.branch.as_deref();
            // A merge under way has its second parent too, and a ring in the error colour.
            let theirs = w.theirs.map(|t| self.history.find(&t).and_then(|i| self.history.hidden_by(i, &self.fold)).unwrap_or(t));
            let parents: Vec<Oid> = std::iter::once(head).chain(theirs).collect();
            let dot = if theirs.is_some() { Dot::Conflict } else { Dot::Wip };
            let row = ito_graph::Row { id: w.commit, parents: &parents, name, trunk: name.is_some() && name == trunk, dot };
            layout.push(row, &mut self.colors);
        }
        self.shown.extend((0..self.history.len() as u32).filter(|&i| self.history.shown(i, &self.fold)));
        self.layout = layout;
        self.extend_layout(self.want);
        // A commit folded away gives the selection to its merge; one gone
        // (read again after a rebase) to the top.
        // A WIP row selected stays as it is.
        let on_wip = self.selected.is_some_and(|id| self.wips.iter().any(|w| w.commit == id));
        if !on_wip {
            if let Some(i) = self.selected.and_then(|id| self.history.find(&id)) {
                if let Some(m) = self.history.hidden_by(i, &self.fold) {
                    self.selected = Some(m);
                }
            } else if self.done || !self.shown.is_empty() {
                self.selected = None;
            }
        }
        if self.selected.is_none() {
            self.selected =
                self.wips.first().map(|w| w.commit).or_else(|| self.shown.first().map(|&i| self.history.rows()[i as usize].commit.id));
        }
        self.mark();
    }

    /// Lays out the shown rows down to the `n`th (or all of them), and
    /// finishes the layout when the whole history is in it.
    fn extend_layout(&mut self, n: usize) {
        let top = self.wips.len();
        let n = n.min(self.shown.len());
        let from = self.layout.len().saturating_sub(top).min(n);
        for &i in &self.shown[from..n] {
            let r = &self.history.rows()[i as usize];
            let parents = self.history.parents_shown(i, &self.fold);
            let dot = if r.commit.parents.len() > 1 { Dot::Merge } else { Dot::Commit };
            let row = ito_graph::Row { id: r.commit.id, parents: &parents, name: r.name.as_deref(), trunk: r.main, dot };
            self.layout.push(row, &mut self.colors);
        }
        if self.done && self.layout.len() == top + self.shown.len() && !self.layout.finished() {
            self.layout.finish();
        }
    }

    /// Row `k` is in sight: lays out down to there and a screen or so
    /// below.
    pub fn lay_out_to(&mut self, k: u32) {
        let n = (k as usize).saturating_sub(self.wips.len()) + WANT;
        if n > self.want {
            self.want = n;
        }
        if self.layout.len() < self.wips.len() + self.want.min(self.shown.len()) {
            self.extend_layout(self.want);
        }
    }

    /// The filter's hits among the history's rows from `from` on.
    fn hit_from(&mut self, from: usize) {
        if self.query.is_empty() {
            return;
        }
        self.fill_hay();
        for (i, h) in self.hay.iter().enumerate().skip(from) {
            if h.hits(&self.query) {
                self.hits.push(i as u32);
            }
        }
    }

    /// Makes what the filter looks in for the rows read since the last time.
    fn fill_hay(&mut self) {
        let rows = &self.history.rows()[self.hay.len()..];
        self.hay.extend(rows.iter().map(|r| Hay::of(&r.commit, &self.history)));
    }

    /// The scrollbar's marks: a hit, else a tag; a worktree's head over both.
    fn mark(&mut self) {
        self.marks.clear();
        let heads: HashMap<Oid, Option<&str>> =
            self.history.setup().worktrees.iter().filter_map(|w| Some((Oid::parse(&w.head)?, w.branch.as_deref()))).collect();
        let top = self.wips.len();
        self.marks.extend((0..top as u32).map(|k| (k, Mark::Wip)));
        for (k, &i) in self.shown.iter().enumerate() {
            let k = k + top;
            let c = &self.history.rows()[i as usize].commit;
            let mut mark = None;
            if self.hits.binary_search(&i).is_ok() {
                mark = Some(Mark::Hit);
            } else if self.history.labels(&c.id).is_some_and(|l| !l.tags.is_empty()) {
                mark = Some(Mark::Tag);
            }
            if let Some(branch) = heads.get(&c.id) {
                let color = match branch {
                    Some(b) => self.colors.of(b),
                    // Not laid out yet: the trunk's colour till it is.
                    None => self.layout.row(k as u32).map_or(TRUNK_COLOR, |p| p.color),
                };
                mark = Some(Mark::Head(color));
            }
            if let Some(m) = mark {
                self.marks.push((k as u32, m));
            }
        }
    }

    // ── what the window draws ──────────────────────────────────

    pub fn layout(&self) -> &Layout<Oid> {
        &self.layout
    }

    /// How many rows are laid out.
    pub fn len(&self) -> u32 {
        (self.wips.len() + self.shown.len()) as u32
    }

    pub fn is_empty(&self) -> bool {
        self.wips.is_empty() && self.shown.is_empty()
    }

    /// The history's row laid out on row `k`, its index there; `None` for a
    /// WIP row.
    fn index(&self, k: u32) -> Option<u32> {
        (k as usize).checked_sub(self.wips.len()).and_then(|k| self.shown.get(k)).copied()
    }

    /// The history's row laid out on row `k`.
    fn at(&self, k: u32) -> Option<&Row> {
        self.index(k).map(|i| &self.history.rows()[i as usize])
    }

    /// The uncommitted change on row `k`, when it is a WIP row.
    pub fn wip(&self, k: u32) -> Option<&Wip> {
        self.wips.get(k as usize)
    }

    /// The uncommitted change selected.
    pub fn selected_wip(&self) -> Option<&Wip> {
        let id = self.selected?;
        self.wips.iter().find(|w| w.commit == id)
    }

    /// Selects the WIP row of the worktree at `path`; false when it has
    /// none.
    pub fn select_wip(&mut self, path: &std::path::Path) -> bool {
        let Some(id) = self.wips.iter().find(|w| w.path == path).map(|w| w.commit) else { return false };
        self.selected = Some(id);
        self.reveal = true;
        true
    }

    /// The worktrees' uncommitted changes are now `wips` (in any order). A
    /// WIP row selected stays selected when its worktree still has a change.
    pub fn set_wips(&mut self, mut wips: Vec<Wip>) {
        wips.sort_by(|a, b| b.written.cmp(&a.written).then_with(|| a.path.cmp(&b.path)));
        // Two worktrees with the same change on the same commit make the
        // same commit: one row, as the layout wants each id once.
        let mut seen = HashSet::new();
        wips.retain(|w| seen.insert(w.commit));
        if wips == self.wips {
            return;
        }
        if let Some(old) = self.selected_wip() {
            let path = old.path.clone();
            self.selected = wips.iter().find(|w| w.path == path).map(|w| w.commit);
        }
        self.wips = wips;
        self.relayout();
    }

    pub fn commit(&self, k: u32) -> Option<&Commit> {
        self.at(k).map(|r| &r.commit)
    }

    pub fn placed(&self, k: u32) -> Option<&Placed> {
        self.layout.row(k)
    }

    /// The row the selected commit is on (laid out or not).
    pub fn selected_row(&self) -> Option<u32> {
        let id = self.selected?;
        if let Some(k) = self.wips.iter().position(|w| w.commit == id) {
            return Some(k as u32);
        }
        let i = self.history.find(&id)?;
        self.shown.binary_search(&i).ok().map(|k| (k + self.wips.len()) as u32)
    }

    /// The row the marked commit is on.
    pub fn marked_row(&self) -> Option<u32> {
        let id = self.marked?;
        if let Some(k) = self.wips.iter().position(|w| w.commit == id) {
            return Some(k as u32);
        }
        let i = self.history.find(&id)?;
        self.shown.binary_search(&i).ok().map(|k| (k + self.wips.len()) as u32)
    }

    #[cfg(test)]
    pub fn marked(&self) -> Option<Oid> {
        self.marked
    }

    /// Ctrl+click on row `k`: the commit selected until now is marked (kept
    /// when there is one already) and `k` is selected, so the two can be
    /// compared. Clicking the marked row again takes the mark off.
    pub fn mark_to(&mut self, k: u32) {
        let Some(id) = self.wip(k).map(|w| w.commit).or_else(|| self.at(k).map(|r| r.commit.id)) else { return };
        if self.marked == Some(id) {
            self.marked = None;
        } else if self.marked.is_none() {
            self.marked = self.selected.filter(|s| *s != id);
        }
        self.select(k);
    }

    /// Takes the mark off; whether there was one.
    pub fn unmark(&mut self) -> bool {
        self.marked.take().is_some()
    }

    /// The two commits to compare: the marked one, then the selected.
    pub fn pair(&self) -> Option<(Oid, Oid)> {
        Some((self.marked?, self.selected?)).filter(|(a, b)| a != b)
    }

    /// A reading has started (the refs are in).
    pub fn started(&self) -> bool {
        self.started
    }

    /// The whole history is in.
    pub fn done(&self) -> bool {
        self.done
    }

    pub fn asking(&self) -> bool {
        self.asking
    }

    pub fn error(&self) -> Option<&git::Error> {
        self.error.as_ref()
    }

    /// How many commits are still to read, when the count is known.
    pub fn remaining(&self) -> Option<u64> {
        self.total.map(|t| t.saturating_sub(self.history.len() as u64))
    }

    pub fn fold_on(&self) -> bool {
        self.fold.on
    }

    /// The trunk's name, for the folding's toast.
    pub fn trunk_name(&self) -> Option<&str> {
        self.history.setup().trunk.as_ref().map(|t| t.name.as_str())
    }

    pub fn filter(&self) -> &str {
        &self.filter
    }

    /// How many commits the filter hits (folded ones too); `None` with no
    /// filter.
    pub fn hit_count(&self) -> Option<usize> {
        (!self.query.is_empty()).then_some(self.hits.len())
    }

    /// Row `k` is drawn faint: a filter is in and does not hit it.
    pub fn dimmed(&self, k: u32) -> bool {
        !self.query.is_empty() && self.index(k).is_some_and(|i| self.hits.binary_search(&i).is_err())
    }

    pub fn marks(&self) -> &[(u32, Mark)] {
        &self.marks
    }

    /// The commit selected, wherever it is.
    pub fn selected(&self) -> Option<Oid> {
        self.selected
    }

    /// The commit `id`, once it is read.
    pub fn find_commit(&self, id: &Oid) -> Option<&Commit> {
        self.history.find(id).map(|i| &self.history.rows()[i as usize].commit)
    }

    /// What the history is read against: the refs, the worktrees, the trunk.
    pub fn setup(&self) -> &Setup {
        self.history.setup()
    }

    /// How many commits are read.
    pub fn read(&self) -> usize {
        self.history.len()
    }

    /// The commits the trunk merge `id` brought into the trunk, newest first,
    /// as far as they are read; empty when `id` is no such merge. It goes
    /// through the whole history: keep what it gives.
    pub fn merged_by(&self, id: &Oid) -> Vec<Oid> {
        let Some(i) = self.history.find(id) else { return Vec::new() };
        let r = &self.history.rows()[i as usize];
        if !r.main || r.commit.parents.len() < 2 {
            return Vec::new();
        }
        self.history.rows().iter().filter(|c| c.into == Some(i)).map(|c| c.commit.id).collect()
    }

    /// How the graph shows the commits the trunk merge `id` brought in.
    pub fn fold_state(&self, id: &Oid) -> FoldState {
        let folds = self.history.find(id).and_then(|i| self.history.group(i)).is_some_and(|g| g.hidden > 0);
        match (self.fold.on && folds, self.fold.open.contains(id)) {
            (false, _) => FoldState::Off,
            (true, true) => FoldState::Open,
            (true, false) => FoldState::Folded,
        }
    }

    /// The colour a branch's rows take.
    pub fn color_of(&self, name: &str) -> u8 {
        self.colors.of(name)
    }

    /// The names on row `k`: branches (a worktree's filled), remote
    /// branches, detached worktrees, then tags.
    pub fn pills(&self, k: u32) -> Vec<Pill> {
        let Some(r) = self.at(k) else { return Vec::new() };
        let id = r.commit.id;
        let setup = self.history.setup();
        let mut out = Vec::new();
        let labels = self.history.labels(&id);
        if let Some(l) = labels {
            for b in &l.locals {
                let worked = setup.worktrees.iter().find(|w| w.branch.as_deref() == Some(b.as_str()));
                let also = l.remotes.iter().filter(|(_, n)| n == b).map(|(r, _)| r.clone()).collect();
                let kind = if worked.is_some() { PillKind::Worktree } else { PillKind::Local };
                let path = worked.map(|w| w.path.to_string_lossy().into_owned());
                out.push(Pill { kind, text: b.clone(), color: self.colors.of(b), also, path });
            }
            for (remote, n) in l.remotes.iter().filter(|(_, n)| !l.locals.contains(n)) {
                let text = format!("{remote}/{n}");
                out.push(Pill { kind: PillKind::Remote, text, color: self.colors.of(n), also: Vec::new(), path: None });
            }
        }
        let color = self.layout.row(k).map_or(TRUNK_COLOR, |p| p.color);
        for w in setup.worktrees.iter().filter(|w| w.detached && Oid::parse(&w.head) == Some(id)) {
            let folder = w.path.file_name().map_or_else(|| w.path.to_string_lossy(), |f| f.to_string_lossy()).into_owned();
            let path = Some(w.path.to_string_lossy().into_owned());
            out.push(Pill { kind: PillKind::Worktree, text: folder, color, also: Vec::new(), path });
        }
        if let Some(l) = labels {
            out.extend(l.tags.iter().map(|t| Pill { kind: PillKind::Tag, text: t.clone(), color, also: Vec::new(), path: None }));
        }
        out
    }

    /// The chip of row `k`, when it is a trunk merge that folds something
    /// away and folding is on.
    pub fn chip(&self, k: u32) -> Option<Chip<'_>> {
        if !self.fold.on {
            return None;
        }
        let i = self.index(k)?;
        let r = &self.history.rows()[i as usize];
        let g = self.history.group(i).filter(|g| r.main && g.hidden > 0)?;
        let color = merged_name(&r.commit.subject, &self.history.setup().remotes).map(|n| self.colors.of(&n));
        Some(Chip { hidden: g.hidden, open: self.fold.open.contains(&r.commit.id), names: &g.names, color })
    }

    // ── what the keys and the mouse do ────────────────────────

    /// Selects row `k`.
    pub fn select(&mut self, k: u32) {
        if let Some(id) = self.wip(k).map(|w| w.commit).or_else(|| self.at(k).map(|r| r.commit.id)) {
            self.selected = Some(id);
            self.picked = true;
            self.reveal = true;
        }
    }

    /// Whether the selection was moved by a key or the mouse since the last
    /// call.
    pub fn take_picked(&mut self) -> bool {
        std::mem::take(&mut self.picked)
    }

    /// Moves the selection `by` rows, down for more than 0, stopping at the
    /// ends.
    pub fn move_by(&mut self, by: i64) {
        if self.is_empty() {
            return;
        }
        let at = self.selected_row().unwrap_or(0) as i64;
        self.select((at + by).clamp(0, self.len() as i64 - 1) as u32);
    }

    /// One page down (`up`: up), keeping a row of the last page in sight.
    pub fn page_by(&mut self, up: bool) {
        let n = self.page.saturating_sub(1).max(1) as i64;
        self.move_by(if up { -n } else { n });
    }

    pub fn home(&mut self) {
        self.select(0);
    }

    pub fn end(&mut self) {
        self.select(self.len().saturating_sub(1));
    }

    /// `z`: folds the trunk's merges, or shows everything. Every merge
    /// opened by hand closes again.
    pub fn toggle_fold(&mut self) -> Folded {
        self.fold.on = !self.fold.on;
        self.fold.open.clear();
        self.relayout();
        self.reveal = true;
        if !self.fold.on {
            return Folded::Off;
        }
        match self.history.folded() {
            (0, _) => Folded::Nothing,
            (merges, commits) => Folded::On { merges, commits },
        }
    }

    /// `l`: opens the selected merge's fold.
    pub fn open_fold(&mut self) -> bool {
        let Some(k) = self.selected_row() else { return false };
        if self.chip(k).is_none_or(|c| c.open) {
            return false;
        }
        self.toggle_group(k);
        true
    }

    /// `h`: folds the selected merge again, or the open merge the selected
    /// commit came in by (selecting the merge).
    pub fn close_fold(&mut self) -> bool {
        let Some(i) = self.selected.and_then(|id| self.history.find(&id)) else { return false };
        let rows = self.history.rows();
        let r = &rows[i as usize];
        let merge = if r.main { Some(r.commit.id) } else { r.into.map(|m| rows[m as usize].commit.id) };
        let Some(m) = merge.filter(|m| self.fold.on && self.fold.open.contains(m)) else { return false };
        self.fold.open.remove(&m);
        self.selected = Some(m);
        self.picked = true;
        self.relayout();
        self.reveal = true;
        true
    }

    /// The chip on row `k` is clicked: its merge opens or folds again.
    pub fn toggle_group(&mut self, k: u32) {
        let Some(open) = self.chip(k).map(|c| c.open) else { return };
        let Some(id) = self.at(k).map(|r| r.commit.id) else { return };
        if open {
            self.fold.open.remove(&id);
        } else {
            self.fold.open.insert(id);
        }
        self.relayout();
    }

    /// The filter's text is now `text`: a commit is hit when its subject,
    /// author or one of its names holds it, or its hash starts with it, in
    /// any case. The rows it does not hit are drawn faint, not taken away.
    pub fn set_filter(&mut self, text: &str) {
        if self.filter == text {
            return;
        }
        self.filter = text.to_owned();
        let query = text.trim().to_lowercase();
        if query == self.query {
            return;
        }
        // A longer query hits only what the shorter one did: typing on
        // looks only among the hits.
        let narrower = !self.query.is_empty() && query.starts_with(&self.query);
        self.query = query;
        if narrower {
            self.fill_hay();
            let (hay, query) = (&self.hay, &self.query);
            self.hits.retain(|&i| hay[i as usize].hits(query));
        } else {
            self.hits.clear();
            self.hit_from(0);
        }
        self.mark();
    }

    /// Selects the next hit below the selection (`back`: above), going round
    /// at the end. A hit folded away opens its merge.
    pub fn next_hit(&mut self, back: bool) -> bool {
        let (Some(&first), Some(&last)) = (self.hits.first(), self.hits.last()) else { return false };
        let at = self.selected.and_then(|id| self.history.find(&id));
        let i = match (at, back) {
            (None, false) => first,
            (None, true) => last,
            (Some(a), false) => self.hits.iter().copied().find(|&h| h > a).unwrap_or(first),
            (Some(a), true) => self.hits.iter().rev().copied().find(|&h| h < a).unwrap_or(last),
        };
        self.select_at(i);
        true
    }

    /// Selects the commit `id` (a branch's, from the side column), opening
    /// the merge that folds it away; false while it is not read.
    pub fn select_id(&mut self, id: &Oid) -> bool {
        let Some(i) = self.history.find(id) else { return false };
        self.select_at(i);
        true
    }

    /// Selects the history's row `i`, opening the merge that folds it away.
    fn select_at(&mut self, i: u32) {
        if let Some(m) = self.history.hidden_by(i, &self.fold) {
            self.fold.open.insert(m);
        }
        self.selected = Some(self.history.rows()[i as usize].commit.id);
        self.picked = true;
        self.relayout();
        self.reveal = true;
    }

    /// The rows in sight now reach `last`: whether to ask for the next
    /// batch (once until it comes).
    pub fn wants_more(&mut self, last: u32) -> bool {
        if (last as usize) + AHEAD < self.len() as usize {
            return false;
        }
        self.ask_more()
    }

    /// Whether to ask for the next batch now (the row at the end is clicked).
    pub fn ask_more(&mut self) -> bool {
        if !self.started || self.next.is_some() || self.done || self.asking || self.error.is_some() {
            return false;
        }
        self.asking = true;
        true
    }
}

/// How many rows below the lowest one in sight are laid out ahead.
const WANT: usize = 300;

/// What the filter looks in for one commit.
struct Hay {
    /// The hash, then the subject, the author and the names in lower case,
    /// each after a NUL (which a query cannot hold).
    text: Box<str>,
    /// Where the hash ends.
    hash: usize,
}

impl Hay {
    fn of(c: &Commit, history: &History) -> Hay {
        let mut text = c.id.to_string();
        let hash = text.len();
        let mut add = |s: &str| {
            text.push('\0');
            text.push_str(&s.to_lowercase());
        };
        add(&c.subject);
        add(&c.author);
        if let Some(l) = history.labels(&c.id) {
            for n in l.locals.iter().chain(&l.tags) {
                add(n);
            }
            for (r, n) in &l.remotes {
                add(&format!("{r}/{n}"));
            }
        }
        Hay { text: text.into_boxed_str(), hash }
    }

    /// Whether the filter's `query` (lower case) hits it: its hash starts
    /// with it, or its subject, author or one of its names holds it.
    fn hits(&self, query: &str) -> bool {
        self.text[..self.hash].starts_with(query) || self.text[self.hash..].contains(query)
    }
}

/// A bot's name as the graph shows it: `github-actions[bot]` is `⚙ actions`.
/// `None` for a person.
pub fn bot(author: &str) -> Option<String> {
    let name = author.strip_suffix("[bot]")?;
    Some(format!("⚙ {}", name.strip_prefix("github-").unwrap_or(name)))
}

/// When a commit was written, as the graph's column shows it: the time
/// today, `yesterday` and the time, the month and day this year, else the
/// date.
pub fn when<Tz: TimeZone>(t: i64, now: &DateTime<Tz>, yesterday: &str) -> String {
    let Some(d) = now.timezone().timestamp_opt(t, 0).single() else { return String::new() };
    let hm = format!("{:02}:{:02}", d.hour(), d.minute());
    let days = (now.date_naive() - d.date_naive()).num_days();
    match days {
        0 => hm,
        1 => format!("{yesterday} {hm}"),
        _ if d.year() == now.year() => format!("{}/{} {hm}", d.month(), d.day()),
        _ => format!("{}/{}/{}", d.year(), d.month(), d.day()),
    }
}

/// A small history, for the tests here and in app.rs.
#[cfg(test)]
pub(crate) mod fixture {
    use super::*;
    use michi_core::refs::{Ref, Refs, RemoteRef, Trunk};
    use michi_core::worktree::Worktree;

    /// `n` at both ends, so a short hash names one commit.
    pub fn id(n: u32) -> Oid {
        Oid::parse(&format!("{n:x}{n:039x}")).unwrap()
    }

    pub fn commit(n: u32, parents: &[u32], subject: &str) -> Commit {
        Commit {
            id: id(n),
            parents: parents.iter().map(|&p| id(p)).collect(),
            author: if n == 4 { "github-actions[bot]".into() } else { "michi".into() },
            time: 1_767_225_600 + n as i64 * 60,
            subject: subject.into(),
        }
    }

    pub const FEAT: &str = "Merge branch 'feat'";

    /// The uncommitted change of the worktree at `path`, on commit `head`,
    /// held in the temporary commit `commit`: one file, last written at
    /// `written`.
    pub fn wip(path: &str, branch: Option<&str>, head: u32, commit: u32, written: Option<i64>) -> Wip {
        let file = michi_core::show::File {
            status: michi_core::show::Status::Modified,
            path: "src/lib.rs".into(),
            from: None,
            blob: None,
            counts: Some((3, 1)),
            hunks: Vec::new(),
            eol_only: false,
            cut: false,
            not_utf8: false,
        };
        Wip {
            path: path.into(),
            branch: branch.map(str::to_owned),
            head: id(head),
            commit: id(commit),
            files: vec![file],
            written,
            left_out: Vec::new(),
            theirs: None,
        }
    }

    /// main: first ← m1 ← merge(feat: f1 ← f2) ← m2; a worktree on `wip`
    /// (w1) off the merge, `origin/main` on m2, a tag on m1.
    pub fn history() -> Vec<Commit> {
        vec![
            commit(6, &[5], "m2"),
            commit(7, &[5], "w1"),
            commit(5, &[4, 3], FEAT),
            commit(4, &[1], "m1"),
            commit(3, &[2], "f2"),
            commit(2, &[1], "f1"),
            commit(1, &[], "first"),
        ]
    }

    pub fn setup() -> Setup {
        let refs = Refs {
            locals: vec![Ref { name: "main".into(), id: id(6) }, Ref { name: "wip".into(), id: id(7) }],
            remotes: vec![
                RemoteRef { remote: "origin".into(), name: "main".into(), id: id(6) },
                RemoteRef { remote: "origin".into(), name: "feat".into(), id: id(3) },
            ],
            tags: vec![Ref { name: "v1".into(), id: id(4) }],
            ..Refs::default()
        };
        let wt = |path: &str, head: u32, branch: Option<&str>| Worktree {
            path: path.into(),
            head: id(head).to_string(),
            branch: branch.map(str::to_owned),
            detached: branch.is_none(),
            ..Default::default()
        };
        let worktrees = vec![wt("C:/r/repo", 6, Some("main")), wt("C:/r/wt", 7, Some("wip")), wt("C:/r/old", 4, None)];
        Setup::new(refs, worktrees, Some(Trunk { name: "main".into(), head: id(6), remote: Some("origin".into()) }))
    }

    /// The setup's worktrees as the worktree table reads them: `wip` a
    /// commit ahead and pushed, the folder at v1 behind and locked.
    pub fn trees() -> michi_core::track::Trees {
        use michi_core::track::{Tree, Trees, Upstream};
        let up = |name: &str, ahead, behind, gone| Some(Upstream { name: name.into(), ahead, behind, gone });
        let wts = setup().worktrees;
        let mut old = wts[2].clone();
        old.locked = Some(String::new());
        Trees {
            trunk: Some("main".into()),
            trunk_head: Some(id(6)),
            trees: vec![
                Tree { wt: wts[0].clone(), ahead_behind: Some((0, 0)), upstream: up("origin/main", 0, 0, false), doing: None },
                Tree { wt: wts[1].clone(), ahead_behind: Some((1, 0)), upstream: up("origin/wip", 2, 1, false), doing: None },
                Tree { wt: old, ahead_behind: Some((0, 3)), upstream: None, doing: None },
            ],
        }
    }

    /// `wip` has its head commit not seen yet; the folder at v1 nothing.
    pub fn unseen() -> michi_core::unseen::Unseen {
        use michi_core::unseen::{New, Track, Unseen};
        let track = |path: &str, name: &str, commits| Track { path: path.into(), name: name.into(), commits, key: name.into() };
        Unseen {
            tracks: vec![
                track("C:/r/wt", "feature/wip", vec![New { id: id(7), time: 1_767_225_600, subject: "w1".into() }]),
                track("C:/r/old", "old", Vec::new()),
            ],
        }
    }

    /// The three worktrees' sides: `main` and `wip` conflict in two files,
    /// `wip`'s uncommitted change with the folder at v1 in one; a branch
    /// `feat` on f2 with main in one.
    pub fn clashes() -> michi_core::clash::Clashes {
        use michi_core::clash::{Branch, Clashes, Outcome, Side};
        use michi_core::predict::{Conflict, ConflictFile};
        let side = |name: &str, path: &str, head, tip| Side { name: name.into(), path: Some(path.into()), head: id(head), tip: id(tip) };
        let file = |path: &str, hunks| ConflictFile { path: path.into(), hunks };
        Clashes {
            sides: vec![side("main", "C:/r/repo", 6, 6), side("wip", "C:/r/wt", 7, 11), side("C:/r/old", "C:/r/old", 4, 4)],
            pairs: vec![
                Outcome::Committed(Conflict { files: vec![file("src/lib.rs", Some(2)), file("assets/logo.png", None)] }),
                Outcome::Clean,
                Outcome::Uncommitted(Conflict { files: vec![file("src/lib.rs", Some(1))] }),
            ],
            branches: vec![Branch {
                name: "feat".into(),
                head: id(3),
                outcome: Outcome::Committed(Conflict { files: vec![file("docs/a.md", Some(1))] }),
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::*;
    use super::*;
    use chrono::FixedOffset;

    fn read() -> Graph {
        let mut g = Graph::default();
        g.start(1, setup());
        g.commits(1, history(), true);
        g
    }

    fn subjects(g: &Graph) -> Vec<&str> {
        (0..g.len()).map(|k| g.commit(k).unwrap().subject.as_str()).collect()
    }

    fn row_of(g: &Graph, subject: &str) -> u32 {
        (0..g.len()).find(|&k| g.commit(k).is_some_and(|c| c.subject == subject)).unwrap_or_else(|| panic!("no {subject}"))
    }

    #[test]
    fn a_reading_again_keeps_the_history_until_its_first_batch() {
        let mut g = read();
        let rows = g.len();
        g.start(2, setup());
        assert_eq!(g.len(), rows, "still the old history");
        assert!(!g.ask_more(), "no older batch of the old reading");
        // An answer of the older reading is dropped.
        g.commits(1, history(), true);
        assert_eq!(g.len(), rows);
        g.commits(2, history()[..2].to_vec(), false);
        assert_eq!(g.len(), 2);
        // A reading that fails shows the failure, not the old history.
        g.start(3, setup());
        g.failed(3, git::Error::NotRepo);
        assert!(g.error().is_some());
    }

    #[test]
    fn ctrl_click_marks_the_selected_commit_and_selects_another_to_compare() {
        let mut g = read();
        g.select(1);
        let first = g.selected().unwrap();
        assert_eq!(g.pair(), None);
        g.mark_to(3);
        assert_eq!((g.marked(), g.marked_row()), (Some(first), Some(1)));
        let second = g.selected().unwrap();
        assert_eq!(g.pair(), Some((first, second)));
        // A further Ctrl+click moves the other end, not the mark.
        g.mark_to(4);
        assert_eq!(g.pair().map(|p| p.0), Some(first));
        assert_ne!(g.pair().map(|p| p.1), Some(second));
        // Back on the marked one: the mark is off (a commit is not compared with itself).
        g.mark_to(1);
        assert_eq!((g.marked(), g.pair()), (None, None));
        g.mark_to(2);
        assert!(g.unmark());
        assert!(!g.unmark());
    }

    #[test]
    fn only_the_keys_and_the_mouse_pick_a_commit() {
        let mut g = read();
        assert!(!g.take_picked(), "the first selection is no one's pick");
        g.select(2);
        assert!(g.take_picked());
        assert!(!g.take_picked(), "told once");
        g.set_wips(vec![wip("C:/r/wt", Some("wip"), 7, 10, Some(100))]);
        assert!(!g.take_picked(), "a selection that follows the rows is not a pick");
        assert!(g.select_id(&id(4)));
        assert!(g.take_picked());
    }

    #[test]
    fn wip_rows_go_on_top_the_last_written_first() {
        let mut g = read();
        assert_eq!(g.selected_row(), Some(0));
        g.set_wips(vec![wip("C:/r/wt", Some("wip"), 7, 10, Some(100)), wip("C:/r/repo", Some("main"), 6, 11, Some(200))]);
        assert_eq!(g.len(), 7);
        let paths: Vec<_> = (0..2).map(|k| g.wip(k).unwrap().path.to_str().unwrap()).collect();
        assert_eq!(paths, ["C:/r/repo", "C:/r/wt"]);
        assert!(g.wip(2).is_none() && g.commit(0).is_none());
        assert_eq!(g.commit(2).unwrap().subject, "m2");
        assert_eq!(g.placed(0).unwrap().dot, Dot::Wip);
        assert_eq!(&g.marks()[..2], [(0, Mark::Wip), (1, Mark::Wip)]);
        // Each hangs from its worktree's head.
        let to = |k: u32| g.layout().edges_of(k).iter().map(|e| e.to).collect::<Vec<_>>();
        assert_eq!(to(0), [Some(row_of(&g, "m2"))]);
        assert_eq!(to(1), [Some(row_of(&g, "w1"))]);
        // The commit selected before stays selected.
        assert_eq!(g.selected(), Some(id(6)));
        assert!(g.selected_wip().is_none());
    }

    #[test]
    fn a_selected_wip_row_follows_its_worktree() {
        let mut g = read();
        g.set_wips(vec![wip("C:/r/repo", Some("main"), 6, 11, Some(200)), wip("C:/r/wt", Some("wip"), 7, 10, Some(100))]);
        g.select(1);
        assert_eq!(g.selected_wip().unwrap().path.to_str(), Some("C:/r/wt"));
        // Written again: a new temporary commit, the same worktree.
        g.set_wips(vec![wip("C:/r/repo", Some("main"), 6, 11, Some(200)), wip("C:/r/wt", Some("wip"), 7, 12, Some(300))]);
        assert_eq!(g.selected(), Some(id(12)));
        assert_eq!(g.selected_row(), Some(0), "now the last written");
        // Committed: the selection goes to the top.
        g.set_wips(vec![wip("C:/r/repo", Some("main"), 6, 11, Some(200))]);
        assert_eq!(g.selected(), Some(id(11)));
        g.set_wips(Vec::new());
        assert_eq!(g.len(), 5);
        assert_eq!(g.selected(), Some(id(6)));
    }

    #[test]
    fn the_same_change_in_two_worktrees_is_one_row() {
        let mut g = read();
        g.set_wips(vec![wip("C:/r/a", None, 6, 11, Some(1)), wip("C:/r/b", None, 6, 11, Some(2))]);
        assert_eq!(g.len(), 6);
        assert_eq!(g.wip(0).unwrap().path.to_str(), Some("C:/r/b"));
    }

    #[test]
    fn a_wip_on_a_folded_commit_hangs_from_its_merge() {
        let mut g = read();
        g.set_wips(vec![wip("C:/r/f", Some("feat"), 3, 11, None)]);
        let to: Vec<_> = g.layout().edges_of(0).iter().map(|e| e.to).collect();
        assert_eq!(to, [Some(row_of(&g, FEAT))]);
    }

    #[test]
    fn merges_are_folded_at_first_and_z_shows_everything() {
        let mut g = read();
        assert_eq!(subjects(&g), ["m2", "w1", FEAT, "m1", "first"]);
        let m = row_of(&g, FEAT);
        let chip = g.chip(m).unwrap();
        assert_eq!((chip.hidden, chip.open, chip.names), (2, false, &["origin/feat".to_owned()][..]));
        assert_eq!(chip.color, Some(g.color_of("feat")));
        assert_eq!(g.layout().edges_of(m).len(), 1, "the folded side is not drawn");
        assert_eq!(g.placed(0).unwrap().color, TRUNK_COLOR);

        assert_eq!(g.toggle_fold(), Folded::Off);
        assert_eq!(g.len(), 7);
        assert!(g.chip(row_of(&g, FEAT)).is_none(), "no chip with folding off");
        assert_eq!(g.toggle_fold(), Folded::On { merges: 1, commits: 2 });
        assert_eq!(g.len(), 5);
    }

    #[test]
    fn a_trunk_merge_knows_what_it_brought_in() {
        let mut g = read();
        assert_eq!(g.merged_by(&id(5)), [id(3), id(2)]);
        assert!(g.merged_by(&id(6)).is_empty(), "not a merge");
        assert!(g.merged_by(&id(9)).is_empty(), "not read");
        assert_eq!(g.fold_state(&id(5)), FoldState::Folded);
        assert_eq!(g.fold_state(&id(4)), FoldState::Off, "nothing folded under it");
        g.select(row_of(&g, FEAT));
        assert!(g.open_fold());
        assert_eq!(g.fold_state(&id(5)), FoldState::Open);
        g.toggle_fold();
        assert_eq!(g.fold_state(&id(5)), FoldState::Off);
    }

    #[test]
    fn selecting_a_folded_commit_opens_its_merge() {
        let mut g = read();
        assert!(!subjects(&g).contains(&"f1"));
        assert!(g.select_id(&id(2)));
        assert_eq!(g.selected(), Some(id(2)));
        assert_eq!(g.commit(g.selected_row().unwrap()).unwrap().subject, "f1");
        assert_eq!(g.fold_state(&id(5)), FoldState::Open);
        assert!(!g.select_id(&id(9)), "not read yet");
        assert_eq!(g.selected(), Some(id(2)));
    }

    #[test]
    fn l_opens_a_merge_and_h_folds_it_from_inside() {
        let mut g = read();
        assert_eq!(g.selected_row(), Some(0), "the top row at first");
        assert!(!g.open_fold(), "m2 folds nothing");
        g.select(row_of(&g, FEAT));
        assert!(g.open_fold());
        assert_eq!(subjects(&g), ["m2", "w1", FEAT, "m1", "f2", "f1", "first"]);
        assert!(g.chip(row_of(&g, FEAT)).unwrap().open);
        assert!(!g.open_fold(), "open already");

        g.select(row_of(&g, "f1"));
        assert!(g.close_fold());
        assert_eq!(g.len(), 5);
        assert_eq!(g.selected_row(), Some(row_of(&g, FEAT)), "the merge takes the selection");
        assert!(!g.close_fold(), "folded already");

        g.toggle_group(row_of(&g, FEAT));
        assert_eq!(g.len(), 7);
        g.select(row_of(&g, "f2"));
        g.toggle_fold();
        g.toggle_fold();
        assert_eq!(g.len(), 5, "z folds the merges opened by hand again");
        assert_eq!(g.selected_row(), Some(row_of(&g, FEAT)));
    }

    #[test]
    fn the_selection_moves_and_stops_at_the_ends() {
        let mut g = read();
        g.move_by(-3);
        assert_eq!(g.selected_row(), Some(0));
        g.move_by(2);
        assert_eq!(g.selected_row(), Some(2));
        g.page = 3;
        g.page_by(false);
        assert_eq!(g.selected_row(), Some(4));
        g.page_by(true);
        assert_eq!(g.selected_row(), Some(2));
        g.end();
        assert_eq!(g.selected_row(), Some(4));
        g.home();
        assert_eq!(g.selected_row(), Some(0));
        assert!(g.reveal);
    }

    #[test]
    fn the_filter_hits_subjects_authors_names_and_hashes() {
        let mut g = read();
        assert_eq!(g.hit_count(), None);
        assert!(!g.dimmed(0));
        g.set_filter("  M2 ");
        assert_eq!(g.hit_count(), Some(1));
        assert!(!g.dimmed(0) && g.dimmed(1));
        assert!(g.marks().contains(&(0, Mark::Head(TRUNK_COLOR))), "a worktree's head marks over a hit");

        g.set_filter("actions");
        assert_eq!(g.hit_count(), Some(1), "the author");
        g.set_filter("origin/fe");
        assert_eq!(g.hit_count(), Some(1), "a remote branch");
        g.set_filter(&id(7).to_string()[..10]);
        assert_eq!(g.hit_count(), Some(1), "a hash");

        // A hit folded away opens its merge when it is reached.
        g.set_filter("f");
        assert_eq!(g.hit_count(), Some(4), "the merge, f2 (with origin/feat once), f1 and first");
        let at = |g: &Graph| g.commit(g.selected_row().unwrap()).unwrap().subject.clone();
        g.home();
        assert!(g.next_hit(false));
        assert_eq!(at(&g), FEAT);
        assert!(g.next_hit(false));
        assert_eq!(at(&g), "f2");
        assert_eq!(g.len(), 7);
        g.next_hit(false);
        assert!(g.next_hit(false));
        assert_eq!(at(&g), "first");
        assert!(g.next_hit(false), "round to the top");
        assert_eq!(at(&g), FEAT);
        assert!(g.next_hit(true), "and back round to the foot");
        assert_eq!(at(&g), "first");

        // Typing on looks among the hits; taking a letter off, afresh.
        g.set_filter("f2");
        assert_eq!(g.hit_count(), Some(1));
        g.set_filter("f");
        assert_eq!(g.hit_count(), Some(4));

        g.set_filter("");
        assert!(!g.next_hit(false));
        assert!(!g.dimmed(1));
    }

    #[test]
    fn a_long_history_is_laid_out_as_it_comes_in_sight() {
        let mut g = Graph::default();
        g.start(1, setup());
        let at = |n: u32| Oid::parse(&format!("{:040x}", n + 0x1000)).unwrap();
        let chain: Vec<Commit> = (1..=1000)
            .rev()
            .map(|n| Commit { id: at(n), parents: (n > 1).then(|| at(n - 1)).into_iter().collect(), ..commit(1, &[], "c") })
            .collect();
        g.commits(1, chain, true);
        assert_eq!(g.len(), 1000);
        assert_eq!(g.layout().len(), WANT, "the first screens only");
        assert!(!g.layout().finished());
        // The selection's row is known before it is laid out.
        g.end();
        assert_eq!(g.selected_row(), Some(999));
        g.lay_out_to(999);
        assert_eq!(g.layout().len(), 1000);
        assert!(g.layout().finished());
        assert!(g.placed(999).is_some());
        // Laid out again (folding), it keeps what was in sight.
        g.toggle_fold();
        assert_eq!(g.layout().len(), 1000);
    }

    #[test]
    fn pills_say_what_points_at_a_commit() {
        let mut g = read();
        let pill = |kind, text: &str, color, also: &[&str]| Pill {
            kind,
            text: text.into(),
            color,
            also: also.iter().map(|s| s.to_string()).collect(),
            path: (kind == PillKind::Worktree).then(|| {
                format!(
                    "C:/r/{}",
                    if text == "main" {
                        "repo"
                    } else if text == "wip" {
                        "wt"
                    } else {
                        text
                    }
                )
            }),
        };
        assert_eq!(g.pills(0), [pill(PillKind::Worktree, "main", TRUNK_COLOR, &["origin"])]);
        assert_eq!(g.pills(1), [pill(PillKind::Worktree, "wip", 3, &[])], "the first worktree colour after main's");
        let m1 = row_of(&g, "m1");
        let c = g.placed(m1).unwrap().color;
        assert_eq!(g.pills(m1), [pill(PillKind::Worktree, "old", c, &[]), pill(PillKind::Tag, "v1", c, &[])]);
        g.toggle_fold();
        assert_eq!(g.pills(row_of(&g, "f2")), [pill(PillKind::Remote, "origin/feat", g.color_of("feat"), &[])]);
        assert!(g.pills(row_of(&g, "f1")).is_empty());
    }

    #[test]
    fn batches_come_in_and_more_is_asked_once() {
        let mut g = Graph::default();
        assert!(!g.ask_more(), "nothing started");
        g.start(1, setup());
        let all = history();
        g.commits(1, all[..3].to_vec(), false);
        g.total(1, 7);
        assert_eq!(g.remaining(), Some(4));
        assert!(g.layout().edges_of(2).iter().any(|e| e.to.is_none()), "the merge's parents are still to come");
        assert!(g.wants_more(0));
        assert!(!g.wants_more(2), "asked already");
        g.commits(2, all[3..].to_vec(), true);
        assert_eq!(g.len(), 3, "another reading's batch is dropped");
        g.commits(1, all[3..].to_vec(), true);
        assert_eq!(subjects(&g), ["m2", "w1", FEAT, "m1", "first"]);
        assert!(g.done() && !g.wants_more(4) && !g.asking());

        // Read again: the folding, the selection and the filter stay.
        g.toggle_fold();
        g.select(row_of(&g, "f2"));
        g.set_filter("f2");
        g.start(2, setup());
        let left = g.remaining();
        g.total(1, 99);
        assert_eq!(g.remaining(), left, "an older reading's count is dropped");
        g.commits(2, history(), true);
        assert_eq!(g.len(), 7);
        assert_eq!(g.commit(g.selected_row().unwrap()).unwrap().subject, "f2");
        assert_eq!(g.hit_count(), Some(1));
        g.failed(2, git::Error::Git("bad".into()));
        assert!(g.error().is_some());
        g.start(1, setup());
        assert!(g.error().is_some(), "an older reading does not start again");
    }

    #[test]
    fn bots_and_times_read_short() {
        assert_eq!(bot("github-actions[bot]").as_deref(), Some("⚙ actions"));
        assert_eq!(bot("dependabot[bot]").as_deref(), Some("⚙ dependabot"));
        assert_eq!(bot("michi"), None);

        let jst = FixedOffset::east_opt(9 * 3600).unwrap();
        // 2026-10-15 10:00 JST.
        let now = jst.with_ymd_and_hms(2026, 10, 15, 10, 0, 0).unwrap();
        let at = |y, mo, d, h, mi| jst.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap().timestamp();
        assert_eq!(when(at(2026, 10, 15, 0, 5), &now, "昨日"), "00:05");
        assert_eq!(when(at(2026, 10, 14, 23, 59), &now, "昨日"), "昨日 23:59");
        assert_eq!(when(at(2026, 1, 2, 3, 4), &now, "昨日"), "1/2 03:04");
        assert_eq!(when(at(2025, 12, 31, 9, 0), &now, "昨日"), "2025/12/31");
    }

    /// How long a big history takes to the first screen (docs/dev-prompt.md:
    /// 100k commits, 0.3 s), and what the window then does on the UI thread.
    /// `scripts/make-history.py` makes such a repository; then
    /// `MICHI_BENCH_REPO=<it> cargo test --release -p michi big_history -- --ignored --nocapture`.
    #[test]
    #[ignore = "measures the repository MICHI_BENCH_REPO names"]
    fn big_history() {
        use michi_core::log::Reader;
        use std::time::Instant;

        let Some(dir) = std::env::var_os("MICHI_BENCH_REPO").map(std::path::PathBuf::from) else {
            eprintln!("MICHI_BENCH_REPO is not set");
            return;
        };
        let t = Instant::now();
        let lap = |what: &str| eprintln!("{:>8.1} ms  {what}", t.elapsed().as_secs_f64() * 1000.0);
        let timed = |what: &str, f: &mut dyn FnMut()| {
            let s = Instant::now();
            f();
            eprintln!("{:>8.1} ms  {what}", s.elapsed().as_secs_f64() * 1000.0);
        };
        let (version, summary, setup) = crate::repo::first(&dir);
        let (_, _, setup) = (version.unwrap(), summary.unwrap(), setup.unwrap());
        let heads = setup.heads();
        lap("git's version, the summary and the setup");
        let mut log = Reader::start(&dir, &heads).unwrap();
        let batch = log.next_batch(crate::repo::FIRST).unwrap();
        lap("the first batch");
        let mut g = Graph::default();
        g.start(1, setup);
        g.commits(1, batch, log.done());
        lap("the first screen laid out");

        let mut worst = 0.0_f64;
        while !log.done() {
            let batch = log.next_batch(crate::repo::NEXT).unwrap();
            let s = Instant::now();
            g.commits(1, batch, log.done());
            worst = worst.max(s.elapsed().as_secs_f64());
        }
        lap(&format!("all {} commits read; the slowest batch laid out in {:.1} ms", g.history.len(), worst * 1000.0));
        timed("z (unfold)", &mut || {
            g.toggle_fold();
        });
        timed("z (fold)", &mut || {
            g.toggle_fold();
        });
        timed("filter `p`", &mut || g.set_filter("p"));
        timed("filter `part 1234` typed on", &mut || g.set_filter("part 1234"));
        timed("filter `1234` afresh", &mut || g.set_filter("1234"));
        timed("End, laid out to the end", &mut || {
            g.end();
            g.lay_out_to(g.len());
        });
        timed("z (unfold) at the end", &mut || {
            g.toggle_fold();
            g.lay_out_to(g.len());
        });
    }
}
