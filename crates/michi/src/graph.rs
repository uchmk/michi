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
    history: History,
    fold: Fold,
    layout: Layout<Oid>,
    colors: Colors,
    /// The names `Colors::pin` has had (it counts each call).
    pinned: HashSet<String>,
    /// The history's row for each row laid out, in order.
    shown: Vec<u32>,
    done: bool,
    total: Option<u64>,
    error: Option<git::Error>,
    selected: Option<Oid>,
    /// A batch was asked for and has not come.
    asking: bool,
    filter: String,
    /// The filter, trimmed and in lower case.
    query: String,
    /// The history's rows the filter hits, in order.
    hits: Vec<u32>,
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
            history: History::default(),
            fold: Fold { on: true, open: HashSet::new() },
            layout: Layout::new(),
            colors: Colors::default(),
            pinned: HashSet::new(),
            shown: Vec::new(),
            done: false,
            total: None,
            error: None,
            selected: None,
            asking: false,
            filter: String::new(),
            query: String::new(),
            hits: Vec::new(),
            marks: Vec::new(),
            page: 20,
            reveal: false,
            focus_filter: false,
        }
    }
}

impl Graph {
    /// A reading of the history starts. The folding, the selection and the
    /// filter stay; an answer of an older reading is dropped.
    pub fn start(&mut self, gen: u32, setup: Setup) {
        if gen < self.gen {
            return;
        }
        self.gen = gen;
        self.started = true;
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
        self.relayout();
    }

    /// The next batch of commits, older than those in.
    pub fn commits(&mut self, gen: u32, batch: Vec<Commit>, done: bool) {
        if gen != self.gen || !self.started {
            return;
        }
        let from = self.history.len();
        self.history.extend(batch);
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
            self.error = Some(error);
            self.asking = false;
        }
    }

    /// Lays the history out again from the top: a row read later can change
    /// the edges of those above it (a parent folded away), so nothing laid
    /// out is kept.
    fn relayout(&mut self) {
        let mut layout = if self.history.setup().trunk.is_some() { Layout::new() } else { Layout::without_trunk() };
        self.shown.clear();
        for (i, r) in self.history.rows().iter().enumerate() {
            let i = i as u32;
            if !self.history.shown(i, &self.fold) {
                continue;
            }
            let parents = self.history.parents_shown(i, &self.fold);
            let dot = if r.commit.parents.len() > 1 { Dot::Merge } else { Dot::Commit };
            let row = ito_graph::Row { id: r.commit.id, parents: &parents, name: r.name.as_deref(), trunk: r.main, dot };
            layout.push(row, &mut self.colors);
            self.shown.push(i);
        }
        if self.done {
            layout.finish();
        }
        self.layout = layout;
        // A commit folded away gives the selection to its merge; one gone
        // (read again after a rebase) to the top.
        if let Some(i) = self.selected.and_then(|id| self.history.find(&id)) {
            if let Some(m) = self.history.hidden_by(i, &self.fold) {
                self.selected = Some(m);
            }
        } else if self.done || !self.shown.is_empty() {
            self.selected = None;
        }
        if self.selected.is_none() {
            self.selected = self.shown.first().map(|&i| self.history.rows()[i as usize].commit.id);
        }
        self.mark();
    }

    /// The filter's hits among the history's rows from `from` on.
    fn hit_from(&mut self, from: usize) {
        if self.query.is_empty() {
            return;
        }
        for (i, r) in self.history.rows().iter().enumerate().skip(from) {
            if hits(&self.query, &r.commit, &self.history) {
                self.hits.push(i as u32);
            }
        }
    }

    /// The scrollbar's marks: a hit, else a tag; a worktree's head over both.
    fn mark(&mut self) {
        self.marks.clear();
        let heads: HashMap<Oid, Option<&str>> =
            self.history.setup().worktrees.iter().filter_map(|w| Some((Oid::parse(&w.head)?, w.branch.as_deref()))).collect();
        for (k, &i) in self.shown.iter().enumerate() {
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
                    None => self.layout.rows()[k].color,
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
        self.shown.len() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.shown.is_empty()
    }

    /// The history's row laid out on row `k`.
    fn at(&self, k: u32) -> Option<&Row> {
        self.shown.get(k as usize).map(|&i| &self.history.rows()[i as usize])
    }

    pub fn commit(&self, k: u32) -> Option<&Commit> {
        self.at(k).map(|r| &r.commit)
    }

    pub fn placed(&self, k: u32) -> Option<&Placed> {
        self.layout.row(k)
    }

    /// The row the selected commit is on.
    pub fn selected_row(&self) -> Option<u32> {
        self.selected.and_then(|id| self.layout.find(&id))
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
        !self.query.is_empty() && self.shown.get(k as usize).is_some_and(|i| self.hits.binary_search(i).is_err())
    }

    pub fn marks(&self) -> &[(u32, Mark)] {
        &self.marks
    }

    /// The colour a branch's rows take.
    #[cfg(test)]
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
        let i = *self.shown.get(k as usize)?;
        let r = &self.history.rows()[i as usize];
        let g = self.history.group(i).filter(|g| r.main && g.hidden > 0)?;
        let color = merged_name(&r.commit.subject, &self.history.setup().remotes).map(|n| self.colors.of(&n));
        Some(Chip { hidden: g.hidden, open: self.fold.open.contains(&r.commit.id), names: &g.names, color })
    }

    // ── what the keys and the mouse do ────────────────────────

    /// Selects row `k`.
    pub fn select(&mut self, k: u32) {
        if let Some(r) = self.at(k) {
            self.selected = Some(r.commit.id);
            self.reveal = true;
        }
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
        if query != self.query {
            self.query = query;
            self.hits.clear();
            self.hit_from(0);
            self.mark();
        }
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
        if let Some(m) = self.history.hidden_by(i, &self.fold) {
            self.fold.open.insert(m);
        }
        self.selected = Some(self.history.rows()[i as usize].commit.id);
        self.relayout();
        self.reveal = true;
        true
    }

    /// The rows in sight now reach `last`: whether to ask for the next
    /// batch (once until it comes).
    pub fn wants_more(&mut self, last: u32) -> bool {
        if (last as usize) + AHEAD < self.shown.len() {
            return false;
        }
        self.ask_more()
    }

    /// Whether to ask for the next batch now (the row at the end is clicked).
    pub fn ask_more(&mut self) -> bool {
        if !self.started || self.done || self.asking || self.error.is_some() {
            return false;
        }
        self.asking = true;
        true
    }
}

/// Whether the filter's `query` (lower case) hits `c`.
fn hits(query: &str, c: &Commit, history: &History) -> bool {
    let has = |s: &str| s.to_lowercase().contains(query);
    if c.id.starts_with(query) || has(&c.subject) || has(&c.author) {
        return true;
    }
    history
        .labels(&c.id)
        .is_some_and(|l| l.locals.iter().chain(&l.tags).any(|n| has(n)) || l.remotes.iter().any(|(r, n)| has(&format!("{r}/{n}"))))
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
            defaults: vec![],
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
        (0..g.len()).find(|&k| g.commit(k).unwrap().subject == subject).unwrap_or_else(|| panic!("no {subject}"))
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

        g.set_filter("");
        assert!(!g.next_hit(false));
        assert!(!g.dimmed(1));
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
        g.total(1, 99);
        assert_eq!(g.remaining(), None, "an older reading's count is dropped");
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
        timed("filter `part 1234`", &mut || g.set_filter("part 1234"));
        timed("End", &mut || g.end());
    }
}
