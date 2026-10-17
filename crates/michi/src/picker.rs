//! The branch, tag and stash palette over the graph. `b` lists the branches
//! to switch a worktree to, and makes a new one from what is typed; `T`
//! lists the tags to show, push or delete, and makes one on the selected
//! commit; `s` puts a worktree's changes away, or a stash back. What is
//! typed is matched the way the other palettes match (ito-match). And `t`,
//! which asks before it reverts the selected commit.

use std::path::{Path, PathBuf};

use ito_match::fuzzy;
use michi_core::branch::Doing;
use michi_core::history::Setup;
use michi_core::oid::Oid;
use michi_core::paths;
use michi_core::worktree::Worktree;

use crate::act::{self, Task};
use crate::app::{App, Repo};
use crate::work::{Band, Job, Then};

/// What the palette lists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Branch,
    Tag,
    Stash,
    /// The remotes a push could go to, for the push waiting in `job`.
    Remote,
}

/// A row of the palette.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    /// A branch; `at` is the worktree that has it checked out, `track` how
    /// far it is ahead of its upstream and behind it.
    Local {
        name: String,
        id: Oid,
        at: Option<PathBuf>,
        track: Option<(u32, u32)>,
    },
    /// A remote branch no local branch has the name of.
    Remote {
        remote: String,
        name: String,
        id: Oid,
    },
    Tag {
        name: String,
        id: Oid,
    },
    /// What is typed, as a new branch or tag.
    Create {
        name: String,
    },
    /// The worktree's changes, to put away as a new stash (what is typed is
    /// its message).
    Save,
    /// A remote to push to.
    Target {
        name: String,
    },
    /// A stash, `stash@{place}`.
    Stash {
        id: Oid,
        subject: String,
        place: usize,
    },
}

impl Item {
    /// What the row says first, and what is matched: `origin/feat` for a
    /// remote branch.
    pub fn title(&self) -> String {
        match self {
            Item::Local { name, .. } | Item::Tag { name, .. } | Item::Create { name } | Item::Target { name } => name.clone(),
            Item::Remote { remote, name, .. } => format!("{remote}/{name}"),
            Item::Stash { subject, .. } => subject.clone(),
            Item::Save => String::new(),
        }
    }
}

/// A row that matches what is typed: the item and its characters that
/// matched (char offsets into the title).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub item: Item,
    pub hits: Vec<usize>,
}

/// The keys besides Enter that act on the selected row.
fn picker_view() -> ito_palette::View {
    use eframe::egui::{Key, Modifiers};
    let mut v = ito_palette::View::new();
    v.filter = false;
    v.keys = vec![
        (Modifiers::SHIFT, Key::Enter),
        (Modifiers::NONE, Key::Delete),
        (Modifiers::NONE, Key::F2),
        (Modifiers::COMMAND, Key::U),
        (Modifiers::COMMAND, Key::R),
    ];
    v
}

pub struct Picker {
    pub kind: Kind,
    /// What is typed: a branch's name; a tag's name, then its message; a
    /// new stash's message, which narrows the stashes too.
    pub query: String,
    /// The query the rows are for.
    matched: Option<String>,
    pub sel: usize,
    /// The worktree a branch switches, a stash is made of or put back on.
    pub wt: PathBuf,
    /// Where a new branch or tag goes.
    pub at: Oid,
    items: Vec<Item>,
    pub rows: Vec<Row>,
    /// The palette's own state (ito-palette): the keys it takes and the
    /// click that opened it.
    pub view: ito_palette::View,
    /// The branch being renamed: the box then holds its new name and the
    /// rows are not shown.
    pub renaming: Option<String>,
    /// The push a remote is being chosen for.
    pub job: Option<Job>,
}

impl Picker {
    /// The branches (or the tags, or the stashes) of `setup`, with
    /// `worktrees` to tell where each branch is checked out.
    pub fn new(kind: Kind, setup: &Setup, worktrees: &[Worktree], wt: PathBuf, at: Oid) -> Picker {
        let refs = &setup.refs;
        let items = match kind {
            Kind::Branch => {
                let locals = refs.locals.iter().map(|r| Item::Local {
                    name: r.name.clone(),
                    id: r.id,
                    at: worktrees.iter().find(|w| w.branch.as_deref() == Some(r.name.as_str())).map(|w| w.path.clone()),
                    track: refs.track.get(&r.name).copied(),
                });
                // A remote branch with a local one of its name is that one's.
                let remotes = refs
                    .remotes
                    .iter()
                    .filter(|r| r.name != "HEAD" && !refs.locals.iter().any(|l| l.name == r.name))
                    .map(|r| Item::Remote { remote: r.remote.clone(), name: r.name.clone(), id: r.id });
                locals.chain(remotes).collect()
            }
            Kind::Tag => refs.tags.iter().map(|r| Item::Tag { name: r.name.clone(), id: r.id }).collect(),
            Kind::Remote => Vec::new(),
            Kind::Stash => {
                refs.stashes.iter().enumerate().map(|(place, s)| Item::Stash { id: s.id, subject: s.subject.clone(), place }).collect()
            }
        };
        let mut p = Picker {
            kind,
            query: String::new(),
            matched: None,
            sel: 0,
            wt,
            at,
            items,
            rows: Vec::new(),
            view: picker_view(),
            renaming: None,
            job: None,
        };
        p.sync();
        p
    }

    /// The palette of the remotes the push `job` could go to.
    pub fn targets(remotes: Vec<String>, job: Job) -> Picker {
        let wt = match &job {
            Job::Push { wt, .. } | Job::Sync { wt, .. } => wt.clone(),
            _ => PathBuf::new(),
        };
        let items = remotes.into_iter().map(|name| Item::Target { name }).collect();
        let mut p = Picker {
            kind: Kind::Remote,
            query: String::new(),
            matched: None,
            sel: 0,
            wt,
            at: Oid::parse(&"0".repeat(40)).expect("a zero id"),
            items,
            rows: Vec::new(),
            view: picker_view(),
            renaming: None,
            job: Some(job),
        };
        p.sync();
        p
    }

    /// The name typed: all of it for a branch (and what narrows the
    /// stashes), the first word for a tag.
    pub fn name(&self) -> &str {
        match self.kind {
            Kind::Branch | Kind::Stash | Kind::Remote => self.query.trim(),
            Kind::Tag => self.query.split_whitespace().next().unwrap_or(""),
        }
    }

    /// A new tag's message: what is typed after its name; a new stash's:
    /// all of it.
    pub fn message(&self) -> &str {
        if self.kind == Kind::Stash {
            return self.query.trim();
        }
        let q = self.query.trim_start();
        q.find(char::is_whitespace).map_or("", |k| q[k..].trim())
    }

    /// Matches the rows again when the query has changed: the best first,
    /// in the list's order when as good; what is typed as a new one last,
    /// unless a branch has that name (a remote one is followed instead).
    /// The stashes have the row that makes one first, and selected.
    pub fn sync(&mut self) {
        if self.matched.as_deref() == Some(self.query.as_str()) {
            return;
        }
        self.matched = Some(self.query.clone());
        let name = self.name().to_owned();
        let case = fuzzy::is_case_sensitive(&name, true, false);
        let mut hits: Vec<(i32, usize, Row)> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(k, item)| {
                let hit = fuzzy::match_str(&name, &item.title(), case)?;
                Some((hit.score, k, Row { item: item.clone(), hits: hit.positions }))
            })
            .collect();
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        self.rows = hits.into_iter().map(|(_, _, r)| r).collect();
        let taken = |i: &Item| i.title() == name || matches!(i, Item::Remote { name: n, .. } if *n == name);
        if self.kind == Kind::Stash {
            self.rows.insert(0, Row { item: Item::Save, hits: Vec::new() });
        } else if self.kind != Kind::Remote && !name.is_empty() && !self.items.iter().any(taken) {
            self.rows.push(Row { item: Item::Create { name }, hits: Vec::new() });
        }
        self.sel = 0;
        if self.renaming.is_some() {
            self.rows.clear();
        }
    }

    pub fn selected(&self) -> Option<&Item> {
        self.rows.get(self.sel).map(|r| &r.item)
    }
}

impl App {
    /// `b`, `T` or `s`: the palette, for the worktree whose uncommitted
    /// change the graph has selected, else for the window's own.
    pub(crate) fn open_picker(&mut self, kind: Kind) {
        let Repo::Ready(s) = &self.repo else {
            self.say(self.tr("act.no_tree"));
            return;
        };
        let worktrees = self.worktrees();
        let (wt, at) = match self.graph.selected_wip() {
            Some(w) => (w.path.clone(), Some(w.head)),
            None => {
                let head = worktrees.iter().find(|w| same(&w.path, &s.top)).and_then(|w| Oid::parse(&w.head));
                // A stash is made of the worktree's head, not the selection.
                let at = if kind == Kind::Stash { head.or(self.graph.selected()) } else { self.graph.selected().or(head) };
                (s.top.clone(), at)
            }
        };
        let Some(at) = at else {
            self.say(self.tr("act.pick_commit"));
            return;
        };
        self.picker = Some(Picker::new(kind, self.graph.setup(), &worktrees, wt, at));
    }

    /// The worktrees as last read: the table's, else the graph's.
    fn worktrees(&self) -> Vec<Worktree> {
        if self.trees.trees.is_empty() {
            self.graph.setup().worktrees.clone()
        } else {
            self.trees.trees.iter().map(|t| t.wt.clone()).collect()
        }
    }

    /// What the worktree at `wt` is in the middle of, as last read.
    pub(crate) fn doing_in(&self, wt: &Path) -> Option<Doing> {
        self.trees.trees.iter().find(|t| same(&t.wt.path, wt)).and_then(|t| t.doing)
    }

    /// Enter on the palette's row (`shift`: Shift+Enter). A branch: switch
    /// to it; a new one: make it (and switch to it). A tag: show it (push
    /// it); a new one: make it (and push it). What cannot be done is said
    /// and the palette stays.
    pub(crate) fn picker_enter(&mut self, shift: bool) {
        let Some(p) = &self.picker else { return };
        if let Some(old) = p.renaming.clone() {
            let new = p.query.trim().to_owned();
            if !new.is_empty() && new != old {
                self.picker = None;
                self.act(Task::Rename { old, new });
            }
            return;
        }
        let Some(item) = p.selected().cloned() else { return };
        let (kind, wt, at, message) = (p.kind, p.wt.clone(), p.at, p.message().to_owned());
        match (kind, item) {
            (Kind::Branch, Item::Local { name, at: Some(there), .. }) => {
                let text = if same(&there, &wt) {
                    self.tr("act.here").replace("{wt}", &act::name(&wt)).replace("{name}", &name)
                } else {
                    self.tr("act.elsewhere").replace("{name}", &name).replace("{wt}", &act::name(&there))
                };
                self.say(text);
            }
            (Kind::Branch, Item::Local { name, at: None, .. }) => self.switch_to(Task::Switch { wt: wt.clone(), name }, &wt),
            (Kind::Branch, Item::Remote { remote, name, .. }) => {
                self.switch_to(Task::Track { wt: wt.clone(), remote: format!("{remote}/{name}"), name }, &wt);
            }
            (Kind::Branch, Item::Create { name }) if shift => self.switch_to(Task::Create { name, at, switch: Some(wt.clone()) }, &wt),
            (Kind::Branch, Item::Create { name }) => {
                self.picker = None;
                self.act(Task::Create { name, at, switch: None });
            }
            (Kind::Tag, Item::Create { .. }) if shift && self.net.is_some() => self.busy(),
            (Kind::Tag, Item::Create { name }) => {
                self.picker = None;
                self.act(Task::Tag { name, at, message, push: shift });
            }
            (Kind::Tag, Item::Tag { name, .. }) if shift => {
                if self.net.is_some() {
                    self.busy();
                } else {
                    self.picker = None;
                    self.order(Job::PushTag { tag: name });
                }
            }
            (Kind::Tag, Item::Tag { id, .. }) => {
                self.picker = None;
                self.go_to(id);
            }
            (Kind::Remote, Item::Target { name }) => {
                let job = self.picker.take().and_then(|p| p.job);
                match job {
                    Some(Job::Push { wt, lease, over, shown, .. }) => self.order(Job::Push { wt, remote: Some(name), lease, over, shown }),
                    Some(Job::Sync { wt, .. }) => self.order(Job::Sync { wt, remote: Some(name) }),
                    _ => {}
                }
            }
            (Kind::Stash, Item::Save) => self.guarded(Task::Stash { wt: wt.clone(), message }, &wt, "act.stash_doing", "act.claude_stash"),
            (Kind::Stash, Item::Stash { id, .. }) => {
                self.guarded(Task::Unstash { wt: wt.clone(), id, pop: shift }, &wt, "act.stash_doing", "act.claude_unstash");
            }
            _ => {}
        }
    }

    /// F2 on a local branch's row: the box then takes its new name.
    pub(crate) fn picker_rename(&mut self) {
        let Some(p) = &mut self.picker else { return };
        if p.kind != Kind::Branch || p.renaming.is_some() {
            return;
        }
        if let Some(Item::Local { name, .. }) = p.selected().cloned() {
            p.query = name.clone();
            p.renaming = Some(name);
            p.matched = None;
            p.sync();
        }
    }

    /// Ctrl+U on a local branch's row: it follows the remote branch of its
    /// name.
    pub(crate) fn picker_upstream(&mut self) {
        let Some(Item::Local { name, .. }) = self.picker.as_ref().filter(|p| p.renaming.is_none()).and_then(|p| p.selected().cloned())
        else {
            return;
        };
        self.picker = None;
        self.act(Task::Upstream { name });
    }

    /// Ctrl+R on a branch's row: asks before it rebases the palette's
    /// worktree's branch onto it.
    pub(crate) fn picker_rebase(&mut self) {
        let Some(p) = self.picker.as_ref().filter(|p| p.kind == Kind::Branch && p.renaming.is_none()) else { return };
        let (wt, onto) = match p.selected() {
            Some(Item::Local { name, .. }) => (p.wt.clone(), name.clone()),
            Some(Item::Remote { remote, name, .. }) => (p.wt.clone(), format!("{remote}/{name}")),
            _ => return,
        };
        let branch = self.worktrees().into_iter().find(|w| same(&w.path, &wt)).and_then(|w| w.branch);
        let say = |a: &mut App, key: &'static str| {
            let doing = a.doing_in(&wt).map_or_else(String::new, |d| a.doing_word(d));
            let text = a.tr(key).replace("{wt}", &act::name(&wt)).replace("{rev}", &onto).replace("{doing}", &doing);
            a.say_for(text, 6);
        };
        let Some(branch) = branch else { return say(self, "act.merge_detached") };
        if branch == onto {
            return say(self, "act.rebase_self");
        }
        if self.doing_in(&wt).is_some() {
            return say(self, "act.revert_doing");
        }
        self.picker = None;
        let mut text = self.tr("act.rebase").replace("{branch}", &branch).replace("{onto}", &onto).replace("{wt}", &act::name(&wt));
        if self.claude_working(&wt) {
            text.push_str(self.tr("act.revert_claude"));
        }
        self.band = Some(Band::new(text, Then::Act(Task::Rebase { wt, onto })));
    }

    /// `m` in the graph: asks before it merges the selected commit into the
    /// window's worktree's branch.
    pub(crate) fn ask_merge_commit(&mut self) {
        let Repo::Ready(s) = &self.repo else {
            self.say(self.tr("act.no_tree"));
            return;
        };
        let wt = s.top.clone();
        let commit = match self.graph.selected_wip() {
            Some(_) => None,
            None => self.graph.selected().and_then(|id| self.graph.find_commit(&id)),
        };
        let Some(c) = commit else {
            self.say(self.tr("act.pick_commit"));
            return;
        };
        let (id, subject) = (c.id, c.subject.clone());
        let rev = id.to_string();
        let branch = self.worktrees().into_iter().find(|w| same(&w.path, &wt)).and_then(|w| w.branch);
        let say = |a: &mut App, key: &'static str| {
            let doing = a.doing_in(&wt).map_or_else(String::new, |d| a.doing_word(d));
            let text = a.tr(key).replace("{wt}", &act::name(&wt)).replace("{rev}", &id.short()).replace("{doing}", &doing);
            a.say_for(text, 6);
        };
        let Some(branch) = branch else { return say(self, "act.merge_detached") };
        if self.doing_in(&wt).is_some() {
            return say(self, "act.revert_doing");
        }
        let mut text = self
            .tr("act.merge_commit")
            .replace("{id}", &id.short())
            .replace("{subject}", &subject)
            .replace("{branch}", &branch)
            .replace("{wt}", &act::name(&wt));
        if self.claude_working(&wt) {
            text.push_str(self.tr("act.revert_claude"));
        }
        self.band = Some(Band::new(text, Then::Act(Task::Merge { wt, rev })));
    }

    /// `B` on the strip: the branch palette for the window's worktree, a new
    /// branch at its HEAD (type a name, Shift+Enter).
    pub(crate) fn open_picker_here(&mut self) {
        let Repo::Ready(s) = &self.repo else { return };
        let top = s.top.clone();
        let worktrees = self.worktrees();
        let Some(at) = worktrees.iter().find(|w| same(&w.path, &top)).and_then(|w| Oid::parse(&w.head)) else {
            self.say(self.tr("act.pick_commit"));
            return;
        };
        self.picker = Some(Picker::new(Kind::Branch, self.graph.setup(), &worktrees, top, at));
        self.say_for(self.tr("strip.branch_hint").to_owned(), 8);
    }

    /// `R` on the strip: the worktree back onto its branch; where a Claude
    /// is working, after a yes.
    pub(crate) fn switch_back(&mut self, task: Task, wt: &Path) {
        self.guarded(task, wt, "act.doing", "act.claude_switch");
    }

    /// Switches the worktree at `wt` (`task`): not in the middle of a merge
    /// or a rebase; where a Claude is working, after a yes.
    fn switch_to(&mut self, task: Task, wt: &Path) {
        self.guarded(task, wt, "act.doing", "act.claude_switch");
    }

    /// Runs `task`, which moves the index of the worktree at `wt`: not in
    /// the middle of a merge or a rebase (`doing` says so); where a Claude
    /// is working, after a yes (`claude` asks).
    fn guarded(&mut self, task: Task, wt: &Path, doing: &'static str, claude: &'static str) {
        if let Some(d) = self.doing_in(wt) {
            let text = self.tr(doing).replace("{wt}", &act::name(wt)).replace("{doing}", &self.doing_word(d));
            self.say_for(text, 8);
            return;
        }
        self.picker = None;
        if self.claude_working(wt) {
            let name = match &task {
                Task::Switch { name, .. } | Task::Track { name, .. } | Task::Create { name, .. } | Task::Back { name, .. } => name.as_str(),
                _ => "",
            };
            let text = self.tr(claude).replace("{wt}", &act::name(wt)).replace("{name}", name);
            self.band = Some(Band::new(text, Then::Act(task)));
        } else {
            self.act(task);
        }
    }

    /// Delete on the palette's row: a branch, a tag or a stash, after a
    /// yes. A branch checked out somewhere, or a remote one, is not.
    pub(crate) fn picker_delete(&mut self) {
        let Some(item) = self.picker.as_ref().and_then(|p| p.selected().cloned()) else { return };
        match item {
            Item::Local { name, at: Some(there), .. } => {
                let text = self.tr("act.delete_here").replace("{name}", &name).replace("{wt}", &act::name(&there));
                self.say(text);
            }
            Item::Local { name, at: None, .. } => {
                self.picker = None;
                let text = self.tr("act.delete").replace("{name}", &name);
                self.band = Some(Band::new(text, Then::Act(Task::Delete { name })));
            }
            Item::Remote { remote, name, .. } => {
                let text = self.tr("act.delete_remote").replace("{remote}", &remote).replace("{name}", &name);
                self.say_for(text, 8);
            }
            Item::Tag { name, .. } => {
                self.picker = None;
                let text = self.tr("act.untag").replace("{}", &name);
                self.band = Some(Band::new(text, Then::Act(Task::Untag { name })));
            }
            Item::Stash { id, subject, .. } => {
                self.picker = None;
                let text = self.tr("act.drop_stash").replace("{id}", &id.short()).replace("{subject}", &subject);
                self.band = Some(Band::new(text, Then::Act(Task::DropStash { id })));
            }
            Item::Create { .. } | Item::Save | Item::Target { .. } => {}
        }
    }

    /// `t`: asks before it reverts the selected commit with a new commit on
    /// the window's worktree; a merge asks which parent's side to keep.
    pub(crate) fn ask_revert(&mut self) {
        let Repo::Ready(s) = &self.repo else {
            self.say(self.tr("act.no_tree"));
            return;
        };
        let wt = s.top.clone();
        let commit = match self.graph.selected_wip() {
            Some(_) => None,
            None => self.graph.selected().and_then(|id| self.graph.find_commit(&id)),
        };
        let Some(c) = commit else {
            self.say(self.tr("act.pick_commit"));
            return;
        };
        let (id, subject, parents) = (c.id, c.subject.clone(), c.parents.len());
        if let Some(d) = self.doing_in(&wt) {
            let text = self.tr("act.revert_doing").replace("{wt}", &act::name(&wt)).replace("{doing}", &self.doing_word(d));
            self.say_for(text, 8);
            return;
        }
        let branch = self.worktrees().into_iter().find(|w| same(&w.path, &wt)).and_then(|w| w.branch);
        let branch = branch.unwrap_or_else(|| self.tr("act.detached").to_owned());
        let key = if parents > 1 { "act.revert_merge" } else { "act.revert" };
        let mut text = self
            .tr(key)
            .replace("{id}", &id.short())
            .replace("{subject}", &subject)
            .replace("{branch}", &branch)
            .replace("{wt}", &act::name(&wt));
        if self.claude_working(&wt) {
            text.push_str(self.tr("act.revert_claude"));
        }
        let parents = if parents > 1 { parents.min(9) as u8 } else { 0 };
        self.band = Some(Band { parents, ..Band::new(text, Then::Act(Task::Revert { wt, id, mainline: None })) });
    }

    /// `R`: asks how far to reset the window's worktree's branch to the
    /// selected commit: `1` soft, `2` mixed, `3` hard, each told what it loses.
    pub(crate) fn ask_reset(&mut self) {
        let Repo::Ready(s) = &self.repo else {
            self.say(self.tr("act.no_tree"));
            return;
        };
        let wt = s.top.clone();
        let commit = match self.graph.selected_wip() {
            Some(_) => None,
            None => self.graph.selected().and_then(|id| self.graph.find_commit(&id)),
        };
        let Some(c) = commit else {
            self.say(self.tr("act.pick_commit"));
            return;
        };
        let (id, subject) = (c.id, c.subject.clone());
        if let Some(d) = self.doing_in(&wt) {
            let text = self.tr("act.revert_doing").replace("{wt}", &act::name(&wt)).replace("{doing}", &self.doing_word(d));
            self.say_for(text, 8);
            return;
        }
        let branch = self.worktrees().into_iter().find(|w| same(&w.path, &wt)).and_then(|w| w.branch);
        let branch = branch.unwrap_or_else(|| self.tr("act.detached").to_owned());
        let mut text = self
            .tr("act.reset")
            .replace("{id}", &id.short())
            .replace("{subject}", &subject)
            .replace("{branch}", &branch)
            .replace("{wt}", &act::name(&wt));
        if self.claude_working(&wt) {
            text.push_str(self.tr("act.revert_claude"));
        }
        let task = Task::Reset { wt, id, mode: michi_core::reset::Mode::Mixed };
        self.band = Some(Band { parents: 3, ..Band::new(text, Then::Act(task)) });
    }

    /// `C`: asks before it picks the selected commit onto the window's
    /// worktree; a merge asks which parent its changes are taken against.
    pub(crate) fn ask_cherry(&mut self) {
        let Repo::Ready(s) = &self.repo else {
            self.say(self.tr("act.no_tree"));
            return;
        };
        let wt = s.top.clone();
        let commit = match self.graph.selected_wip() {
            Some(_) => None,
            None => self.graph.selected().and_then(|id| self.graph.find_commit(&id)),
        };
        let Some(c) = commit else {
            self.say(self.tr("act.pick_commit"));
            return;
        };
        let (id, subject, parents) = (c.id, c.subject.clone(), c.parents.len());
        if let Some(d) = self.doing_in(&wt) {
            let text = self.tr("act.revert_doing").replace("{wt}", &act::name(&wt)).replace("{doing}", &self.doing_word(d));
            self.say_for(text, 8);
            return;
        }
        let branch = self.worktrees().into_iter().find(|w| same(&w.path, &wt)).and_then(|w| w.branch);
        let branch = branch.unwrap_or_else(|| self.tr("act.detached").to_owned());
        let key = if parents > 1 { "act.cherry_merge" } else { "act.cherry" };
        let mut text = self
            .tr(key)
            .replace("{id}", &id.short())
            .replace("{subject}", &subject)
            .replace("{branch}", &branch)
            .replace("{wt}", &act::name(&wt));
        if self.claude_working(&wt) {
            text.push_str(self.tr("act.revert_claude"));
        }
        let parents = if parents > 1 { parents.min(9) as u8 } else { 0 };
        self.band = Some(Band { parents, ..Band::new(text, Then::Act(Task::CherryPick { wt, id, mainline: None })) });
    }

    /// `x` in the worktree table: asks before it takes the selected worktree
    /// away (the branch stays).
    pub(crate) fn ask_remove(&mut self) {
        let Some(t) = self.wt_selected() else {
            self.say(self.tr("act.no_tree"));
            return;
        };
        let (wt, branch, ahead, is_main) = (
            t.wt.path.clone(),
            t.wt.branch.clone(),
            t.upstream.as_ref().map_or(0, |u| u.ahead),
            self.trees.trees.first().is_some_and(|f| same(&f.wt.path, &t.wt.path)),
        );
        let name = act::name(&wt);
        if is_main {
            let text = self.tr("act.remove_main").replace("{wt}", &name);
            return self.say_for(text, 5);
        }
        if self.repo_top().is_some_and(|top| same(&top, &wt)) {
            let text = self.tr("act.remove_here").replace("{wt}", &name);
            return self.say_for(text, 5);
        }
        let key = paths::key(&wt);
        let sessions: Vec<u64> = self
            .claude
            .iter()
            .flatten()
            .filter(|(p, _)| paths::key(p) == key)
            .flat_map(|(_, s)| s.iter())
            .filter(|s| s.is_agent())
            .map(|s| s.id)
            .collect();
        let files = self.wips.iter().find(|w| same(&w.path, &wt)).map_or(0, |w| w.files.len());
        let claude = if sessions.is_empty() { String::new() } else { format!(" {}", self.tr("act.remove_claude")) };
        let changes =
            if files == 0 { String::new() } else { format!(" {}", self.tr("act.remove_changes").replace("{n}", &files.to_string())) };
        let ahead = if ahead == 0 { String::new() } else { format!(" {}", self.tr("act.remove_ahead").replace("{n}", &ahead.to_string())) };
        let text = self
            .tr("act.remove")
            .replace("{wt}", &name)
            .replace("{branch}", branch.as_deref().unwrap_or("-"))
            .replace("{claude}", &claude)
            .replace("{changes}", &changes)
            .replace("{ahead}", &ahead);
        self.band = Some(Band::new(text, Then::Act(Task::RemoveWorktree { wt, sessions })));
    }

    /// `m` in the worktree table: asks before it merges the trunk (the
    /// local branch) into the selected worktree's branch.
    pub(crate) fn ask_merge(&mut self) {
        let Some(t) = self.wt_selected() else {
            self.say(self.tr("act.no_tree"));
            return;
        };
        let (wt, branch, doing) = (t.wt.path.clone(), t.wt.branch.clone(), t.doing);
        let Some(trunk) = self.trees.trunk.clone().or_else(|| self.graph.trunk_name().map(str::to_owned)) else {
            self.say(self.tr("act.no_trunk"));
            return;
        };
        let say = |a: &mut App, key: &'static str| {
            let text = a.tr(key).replace("{wt}", &act::name(&wt)).replace("{rev}", &trunk);
            let text = text.replace("{doing}", &doing.map_or_else(String::new, |d| a.doing_word(d)));
            a.say_for(text, 6);
        };
        let Some(branch) = branch else { return say(self, "act.merge_detached") };
        if branch == trunk {
            return say(self, "act.merge_self");
        }
        if doing.is_some() {
            return say(self, "act.revert_doing");
        }
        let mut text = self.tr("act.merge").replace("{rev}", &trunk).replace("{branch}", &branch).replace("{wt}", &act::name(&wt));
        if self.claude_working(&wt) {
            text.push_str(self.tr("act.revert_claude"));
        }
        self.band = Some(Band::new(text, Then::Act(Task::Merge { wt, rev: trunk })));
    }

    /// `M` over the worktree table: asks before the row's branch is rebased
    /// onto the trunk (the other way to take main in).
    pub(crate) fn ask_rebase_trunk(&mut self) {
        let Some(t) = self.wt_selected() else { return self.say(self.tr("act.no_tree")) };
        let (wt, branch, doing) = (t.wt.path.clone(), t.wt.branch.clone(), t.doing);
        let Some(trunk) = self.trees.trunk.clone().or_else(|| self.graph.trunk_name().map(str::to_owned)) else {
            return self.say(self.tr("act.no_trunk"));
        };
        let say = |a: &mut App, key: &'static str| {
            let text = a.tr(key).replace("{wt}", &act::name(&wt)).replace("{rev}", &trunk);
            let text = text.replace("{doing}", &doing.map_or_else(String::new, |d| a.doing_word(d)));
            a.say_for(text, 6);
        };
        let Some(branch) = branch else { return say(self, "act.merge_detached") };
        if branch == trunk {
            return say(self, "act.rebase_self");
        }
        if doing.is_some() {
            return say(self, "act.revert_doing");
        }
        let mut text = self.tr("act.rebase").replace("{branch}", &branch).replace("{onto}", &trunk).replace("{wt}", &act::name(&wt));
        if self.claude_working(&wt) {
            text.push_str(self.tr("act.revert_claude"));
        }
        self.band = Some(Band::new(text, Then::Act(Task::Rebase { wt, onto: trunk })));
    }
}

pub(crate) fn same(a: &Path, b: &Path) -> bool {
    paths::key(a) == paths::key(b)
}

#[cfg(test)]
mod tests {
    impl Picker {
        /// Moves the selection by `by` rows, stopping at the ends.
        pub fn move_by(&mut self, by: i32) {
            let last = self.rows.len().saturating_sub(1) as i64;
            self.sel = (self.sel as i64 + i64::from(by)).clamp(0, last) as usize;
        }
    }

    use super::*;
    use crate::app::tests::{app, read, toast};
    use crate::graph::fixture::{self, id};

    fn titles(p: &Picker) -> Vec<String> {
        p.rows.iter().map(|r| r.item.title()).collect()
    }

    fn picker(kind: Kind) -> Picker {
        let s = fixture::setup();
        Picker::new(kind, &s, &s.worktrees, PathBuf::from("C:/r/repo"), id(6))
    }

    #[test]
    fn branches_then_the_remote_ones_no_local_has() {
        let p = picker(Kind::Branch);
        assert_eq!(titles(&p), ["main", "wip", "origin/feat"]);
        let Item::Local { at, .. } = &p.rows[1].item else { panic!() };
        assert_eq!(at.as_deref(), Some(Path::new("C:/r/wt")));
        assert_eq!(titles(&picker(Kind::Tag)), ["v1"]);
    }

    #[test]
    fn what_is_typed_filters_and_comes_last_as_a_new_one() {
        let mut p = picker(Kind::Branch);
        p.query = "fe".into();
        p.sync();
        assert_eq!(titles(&p), ["origin/feat", "fe"]);
        assert_eq!(p.rows[0].hits, [7, 8]);
        p.move_by(5);
        assert_eq!(p.sel, 1);
        // The same name as a branch: no new one.
        p.query = "main ".into();
        p.sync();
        assert_eq!((titles(&p), p.sel), (vec!["main".to_owned()], 0));
        // A remote branch's name: it is followed, not made anew.
        p.query = "feat".into();
        p.sync();
        assert_eq!(titles(&p), ["origin/feat"]);
        p.query = "zzz".into();
        p.sync();
        assert_eq!(p.rows.iter().map(|r| &r.item).collect::<Vec<_>>(), [&Item::Create { name: "zzz".into() }]);
        // A tag: the name, then its message.
        let mut t = picker(Kind::Tag);
        t.query = "v2  Second release ".into();
        t.sync();
        assert_eq!((t.name(), t.message()), ("v2", "Second release"));
        assert_eq!(titles(&t), ["v2"]);
    }

    /// The app, read, with the selection on the commit `subject`.
    fn on(subject: &str) -> App {
        use michi_core::summary::{Head, Summary};
        let mut a = read();
        let top = PathBuf::from("C:/r/repo");
        a.repo = Repo::Ready(Summary { name: "repo".into(), head: Head::Branch("main".into()), worktrees: 3, top });
        a.take(crate::repo::Answer::Trees(fixture::trees()));
        let id = fixture::history().iter().find(|c| c.subject == subject).unwrap().id;
        assert!(a.graph.select_id(&id));
        a
    }

    fn typed(a: &mut App, q: &str) {
        let p = a.picker.as_mut().unwrap();
        p.query = q.into();
        p.sync();
    }

    #[test]
    fn b_makes_a_branch_at_the_selected_commit() {
        let mut a = on("m1");
        assert!(a.feed('b'));
        let p = a.picker.as_ref().unwrap();
        assert_eq!((p.kind, p.wt.as_path(), p.at), (Kind::Branch, Path::new("C:/r/repo"), id(4)));
        typed(&mut a, "new");
        a.picker_enter(false);
        assert!(a.picker.is_none(), "no reading thread here: it closes all the same");
        assert!(a.band.is_none());
    }

    #[test]
    fn a_branch_checked_out_is_not_switched_to_nor_deleted() {
        let mut a = on("m2");
        assert!(a.feed('b'));
        typed(&mut a, "main");
        a.picker_enter(false);
        assert_eq!(toast(&a), "repo is on main already");
        typed(&mut a, "wip");
        a.picker_enter(true);
        assert_eq!(toast(&a), "wip is checked out in wt: a branch is in one worktree at a time");
        a.picker_delete();
        assert_eq!(toast(&a), "wip is checked out in wt: switch it away first");
        typed(&mut a, "feat");
        a.picker_delete();
        assert_eq!(toast(&a), "Remote branches are not deleted here: git push origin --delete feat does it");
        assert!(a.picker.is_some() && a.band.is_none(), "the palette stays");
        assert!(a.escape());
        assert!(a.picker.is_none());
    }

    #[test]
    fn a_worktree_in_a_merge_is_not_switched() {
        let mut a = on("m2");
        let mut trees = fixture::trees();
        trees.trees[0].doing = Some(Doing::Merge);
        a.take(crate::repo::Answer::Trees(trees));
        assert!(a.feed('b'));
        typed(&mut a, "feat");
        a.picker_enter(false);
        assert!(toast(&a).starts_with("Branches can be made. Switching is stopped in repo: it is in the middle of a merge."));
        assert!(a.picker.is_some());
        // Made, not switched to: that goes.
        typed(&mut a, "x");
        a.picker_enter(true);
        assert!(a.picker.is_some(), "and switched to: not");
        a.picker_enter(false);
        assert!(a.picker.is_none());
    }

    #[test]
    fn deleting_a_branch_or_a_tag_waits_for_a_yes() {
        let mut a = on("m2");
        let mut s = fixture::setup();
        s.refs.locals.push(michi_core::refs::Ref { name: "old".into(), id: id(3) });
        a.picker = Some(Picker::new(Kind::Branch, &s, &s.worktrees, PathBuf::from("C:/r/repo"), id(6)));
        typed(&mut a, "old");
        a.picker_delete();
        assert!(a.picker.is_none());
        let band = a.band.as_ref().unwrap();
        assert_eq!(band.then, Then::Act(Task::Delete { name: "old".into() }));
        assert!(band.text.starts_with("Delete branch old?"));
        assert!(a.escape());
        assert!(a.feed('T'));
        typed(&mut a, "v1");
        a.picker_delete();
        assert_eq!(a.band.as_ref().unwrap().then, Then::Act(Task::Untag { name: "v1".into() }));
    }

    #[test]
    fn enter_on_a_tag_shows_its_commit() {
        let mut a = on("m2");
        assert!(a.feed('T'));
        typed(&mut a, "v1");
        a.picker_enter(false);
        assert!(a.picker.is_none());
        assert_eq!(a.graph.selected(), Some(id(4)));
    }

    #[test]
    fn a_tag_to_push_waits_for_the_running_job() {
        use std::sync::atomic::AtomicBool;
        use std::sync::Arc;
        let mut a = on("m2");
        a.net = Some(crate::work::Running::new(crate::work::Kind::Fetch, Arc::new(AtomicBool::new(false))));
        assert!(a.feed('T'));
        typed(&mut a, "v2");
        a.picker_enter(true);
        assert!(toast(&a).starts_with("fetch is running"), "{}", toast(&a));
        assert!(a.picker.is_some());
    }

    #[test]
    fn t_asks_before_it_reverts() {
        let mut a = on("m1");
        assert!(a.feed('t'));
        let band = a.band.take().unwrap();
        let wt = PathBuf::from("C:/r/repo");
        assert_eq!((band.then, band.parents), (Then::Act(Task::Revert { wt: wt.clone(), id: id(4), mainline: None }), 0));
        assert_eq!(band.text, format!("Revert {} “m1” with a new commit on main in repo?", id(4).short()));

        // A merge: which parent's side to keep.
        let mut a = on(fixture::FEAT);
        assert!(a.feed('t'));
        let band = a.band.as_ref().unwrap();
        assert_eq!(band.parents, 2);
        assert_eq!(band.parent_key(1), "band.parent", "a revert keeps a parent's side");
        assert!(band.text.contains("is a merge"));
        assert!(a.feed('1'));
        assert!(a.band.is_none());

        // In the middle of a revert: not now.
        let mut a = on("m1");
        let mut trees = fixture::trees();
        trees.trees[0].doing = Some(Doing::Revert);
        a.take(crate::repo::Answer::Trees(trees));
        assert!(a.feed('t'));
        assert_eq!(toast(&a), "repo is in the middle of a revert: finish it first");
        assert!(a.band.is_none());
    }

    #[test]
    fn c_asks_before_it_picks_a_commit() {
        let mut a = on("m1");
        assert!(a.feed('C'));
        let band = a.band.take().unwrap();
        let wt = PathBuf::from("C:/r/repo");
        assert_eq!((band.then, band.parents), (Then::Act(Task::CherryPick { wt, id: id(4), mainline: None }), 0));
        assert_eq!(band.text, format!("Pick {} “m1” onto main in repo?", id(4).short()));

        // A merge: against which parent.
        let mut a = on(fixture::FEAT);
        assert!(a.feed('C'));
        let band = a.band.as_ref().unwrap();
        assert_eq!(band.parents, 2);
        assert_eq!(band.parent_key(1), "band.parent_pick", "a pick takes the changes against a parent");
        assert!(band.text.contains("is a merge"), "{}", band.text);
        assert!(a.feed('y'));
        assert!(a.band.is_some(), "y is not a parent");
        assert!(a.feed('2'));
        assert!(a.band.is_none());

        // In the middle of a merge: not now.
        let mut a = on("m1");
        let mut trees = fixture::trees();
        trees.trees[0].doing = Some(Doing::Merge);
        a.take(crate::repo::Answer::Trees(trees));
        assert!(a.feed('C'));
        assert_eq!(toast(&a), "repo is in the middle of a merge: finish it first");
        assert!(a.band.is_none());
    }

    #[test]
    fn m_in_the_worktree_table_asks_before_it_merges_the_trunk() {
        let mut a = on("m1");
        assert!(a.feed('3'));
        assert!(a.feed('m'));
        assert_eq!(toast(&a), "repo is on main itself: there is nothing to merge");
        assert!(a.band.is_none());
        assert!(a.feed('j'));
        assert!(a.feed('m'));
        let band = a.band.take().unwrap();
        assert_eq!(band.then, Then::Act(Task::Merge { wt: PathBuf::from("C:/r/wt"), rev: "main".into() }));
        assert_eq!(band.text, "Merge main into wip in wt?");
        assert!(a.feed('j'));
        assert!(a.feed('m'));
        assert_eq!(toast(&a), "old is on no branch: switch it to one before merging main");

        // In the middle of a rebase: not now.
        let mut trees = fixture::trees();
        trees.trees[1].doing = Some(Doing::Rebase);
        a.take(crate::repo::Answer::Trees(trees));
        assert!(a.feed('k'));
        assert!(a.feed('m'));
        assert_eq!(toast(&a), "wt is in the middle of a rebase: finish it first");
        assert!(a.band.is_none());
    }

    #[test]
    fn x_in_the_worktree_table_asks_before_it_removes_a_worktree() {
        let mut a = on("m1");
        assert!(a.feed('3'));
        assert!(a.feed('x'));
        assert_eq!(toast(&a), "repo is the main worktree: it stays");
        assert!(a.band.is_none());
        assert!(a.feed('j'));
        assert!(a.feed('x'));
        let band = a.band.take().unwrap();
        assert_eq!(band.then, Then::Act(Task::RemoveWorktree { wt: PathBuf::from("C:/r/wt"), sessions: Vec::new() }));
        assert!(band.text.starts_with("Remove the worktree wt? The folder goes; the branch wip stays."), "{}", band.text);
    }

    #[test]
    fn p_in_the_worktree_table_is_a_key_of_the_table() {
        let mut a = on("m1");
        assert!(a.feed('3'));
        assert!(a.keys().iter().any(|(k, _)| *k == "p"));
        assert!(a.feed('p'), "no reading thread here: nothing runs, nothing is said");
    }

    #[test]
    fn m_in_the_graph_asks_before_it_merges_the_selected_commit() {
        let mut a = on("m1");
        let id = a.graph.selected().unwrap();
        assert!(a.feed('m'));
        let band = a.band.take().unwrap();
        assert_eq!(band.then, Then::Act(Task::Merge { wt: PathBuf::from("C:/r/repo"), rev: id.to_string() }));
        assert!(band.text.starts_with(&format!("Merge {} “m1” into main in repo?", id.short())), "{}", band.text);
    }

    #[test]
    fn the_branch_palette_renames_follows_and_rebases() {
        let mut a = on("m1");
        assert!(a.feed('b'));
        typed(&mut a, "wip");
        a.picker_rename();
        let p = a.picker.as_ref().unwrap();
        assert_eq!((p.renaming.as_deref(), p.query.as_str(), p.rows.len()), (Some("wip"), "wip", 0));
        a.picker_enter(false);
        assert!(a.picker.is_some(), "the same name: nothing to do yet");
        typed(&mut a, "wip2");
        a.picker_enter(false);
        assert!(a.picker.is_none());

        assert!(a.feed('b'));
        typed(&mut a, "wip");
        a.picker_rebase();
        let band = a.band.take().unwrap();
        assert_eq!(band.then, Then::Act(Task::Rebase { wt: PathBuf::from("C:/r/repo"), onto: "wip".into() }));
        assert!(a.picker.is_none());

        assert!(a.feed('b'));
        typed(&mut a, "main");
        a.picker_rebase();
        assert_eq!(toast(&a), "repo is on main itself: there is nothing to rebase onto");
        assert!(a.picker.is_some() && a.band.is_none());
        typed(&mut a, "feat");
        a.picker_rebase();
        assert_eq!(a.band.take().unwrap().then, Then::Act(Task::Rebase { wt: PathBuf::from("C:/r/repo"), onto: "origin/feat".into() }));
    }

    #[test]
    fn a_worktree_in_a_merge_keeps_its_head_from_a_pull_and_a_commit() {
        let mut a = on("m1");
        let mut trees = fixture::trees();
        trees.trees[0].doing = Some(Doing::Merge);
        a.take(crate::repo::Answer::Trees(trees));
        assert!(a.feed('p'));
        assert_eq!(toast(&a), "repo is in the middle of a merge: pull waits until it is over");
        assert!(a.band.is_none());
        a.toast = None;
        a.changes_commit();
        assert_eq!(toast(&a), "repo is in the middle of a merge: commit it in the 4 view");
        // Another worktree's pull goes on.
        assert!(a.feed('3'));
        assert!(a.feed('j'));
        a.toast = None;
        assert!(a.feed('p'));
        assert!(a.toast.is_none() && a.band.is_none(), "no reading thread here: nothing runs");
    }

    /// The app, read, with two stashes (the newer first) and the selection
    /// on a commit.
    fn stashed() -> App {
        use michi_core::refs::Stash;
        use michi_core::summary::{Head, Summary};
        let mut a = app();
        let mut s = fixture::setup();
        s.refs.stashes = vec![
            Stash { id: id(13), subject: "On main: half done".into() },
            Stash { id: id(14), subject: "WIP on wip: 7777777 w1".into() },
        ];
        a.take(crate::repo::Answer::Setup { gen: 1, setup: Box::new(s) });
        a.take(crate::repo::Answer::Commits { gen: 1, batch: fixture::history(), done: true });
        let top = PathBuf::from("C:/r/repo");
        a.repo = Repo::Ready(Summary { name: "repo".into(), head: Head::Branch("main".into()), worktrees: 3, top });
        a.take(crate::repo::Answer::Trees(fixture::trees()));
        assert!(a.graph.select_id(&id(6)));
        a
    }

    #[test]
    fn s_lists_the_stashes_after_a_row_that_puts_the_changes_away() {
        let mut a = stashed();
        assert!(a.feed('s'));
        let p = a.picker.as_ref().unwrap();
        assert_eq!((p.kind, p.wt.as_path()), (Kind::Stash, Path::new("C:/r/repo")));
        assert_eq!(p.rows[0].item, Item::Save);
        assert_eq!(titles(p), ["", "On main: half done", "WIP on wip: 7777777 w1"]);
        assert_eq!(p.sel, 0);
        assert_eq!(a.stash_shown(), None, "the row that makes one");
        a.picker.as_mut().unwrap().move_by(1);
        assert_eq!(a.stash_shown(), Some(id(13)), "the detail shows the one selected");
        // What is typed narrows them, and is the new one's message.
        typed(&mut a, "half ");
        let p = a.picker.as_ref().unwrap();
        assert_eq!((titles(p), p.sel, p.message()), (vec![String::new(), "On main: half done".into()], 0, "half"));
        a.picker_enter(false);
        assert!(a.picker.is_none(), "no reading thread here: it closes all the same");
        assert!(a.band.is_none());
    }

    #[test]
    fn a_stash_goes_back_and_shift_pops_it_where_a_claude_works_after_a_yes() {
        use michi_core::claude::{Session, State};
        let mut a = stashed();
        let wt = PathBuf::from("C:/r/repo");
        let s = Session {
            id: 1,
            state: State::Running,
            command: "claude".into(),
            agent: "claude".into(),
            cwd: "C:/r/repo".into(),
            branch: String::new(),
            title: String::new(),
            note: String::new(),
            since_ms: 0,
            read_only: false,
        };
        a.claude = Some(vec![(wt.clone(), vec![s])]);
        assert!(a.feed('s'));
        a.picker.as_mut().unwrap().move_by(2);
        a.picker_enter(true);
        assert!(a.picker.is_none());
        let band = a.band.as_ref().unwrap();
        assert_eq!(band.then, Then::Act(Task::Unstash { wt: wt.clone(), id: id(14), pop: true }));
        assert_eq!(band.text, "A Claude is working in repo. Put the stash back there anyway?");
        assert!(a.escape());
        assert!(a.feed('s'));
        a.picker.as_mut().unwrap().move_by(1);
        a.picker_enter(false);
        assert_eq!(a.band.as_ref().unwrap().then, Then::Act(Task::Unstash { wt: wt.clone(), id: id(13), pop: false }));
        assert!(a.escape());
        assert!(a.feed('s'));
        typed(&mut a, "later");
        a.picker_enter(false);
        let band = a.band.as_ref().unwrap();
        assert_eq!(band.then, Then::Act(Task::Stash { wt, message: "later".into() }));
        assert_eq!(band.text, "A Claude is working in repo. Put its changes away anyway?");
    }

    #[test]
    fn a_worktree_in_a_merge_keeps_its_stashes() {
        let mut a = stashed();
        let mut trees = fixture::trees();
        trees.trees[0].doing = Some(Doing::Merge);
        a.take(crate::repo::Answer::Trees(trees));
        assert!(a.feed('s'));
        a.picker_enter(false);
        assert_eq!(toast(&a), "Stashes are stopped in repo: it is in the middle of a merge.");
        a.picker.as_mut().unwrap().move_by(1);
        a.picker_enter(false);
        assert!(a.picker.is_some() && a.band.is_none(), "the palette stays");
    }

    #[test]
    fn dropping_a_stash_waits_for_a_yes() {
        let mut a = stashed();
        assert!(a.feed('s'));
        a.picker_delete();
        assert!(a.picker.is_some() && a.band.is_none(), "the row that makes one: nothing");
        a.picker.as_mut().unwrap().move_by(2);
        a.picker_delete();
        assert!(a.picker.is_none());
        let band = a.band.as_ref().unwrap();
        assert_eq!(band.then, Then::Act(Task::DropStash { id: id(14) }));
        let text = format!("Drop stash {} “WIP on wip: 7777777 w1”? It is kept under refs/michi/backup/ first.", id(14).short());
        assert_eq!(band.text, text);
    }

    #[test]
    fn a_stash_clicked_aside_shows_until_the_selection_moves_or_escape() {
        let mut a = stashed();
        a.show_stash(id(13));
        assert_eq!(a.stash_shown(), Some(id(13)));
        assert!(a.escape());
        assert_eq!(a.stash_shown(), None);
        a.show_stash(id(14));
        assert_eq!(a.stash_shown(), Some(id(14)));
        assert!(a.graph.select_id(&id(4)));
        assert_eq!(a.stash_shown(), None, "the graph's selection moved");
        a.show_stash(id(1));
        assert_eq!(a.stash_shown(), None, "no such stash");
    }

    #[test]
    fn nothing_read_nothing_to_pick() {
        let mut a = app();
        assert!(a.feed('b'));
        assert_eq!(toast(&a), "No worktree to work in");
        assert!(a.picker.is_none());
    }
}
