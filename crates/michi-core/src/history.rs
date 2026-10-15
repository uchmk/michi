//! What the graph knows about each commit beyond git's record, worked out as
//! the commits come in newest first: the trunk's line, a branch name for the
//! colour, and the folding — which merge on the trunk brought each commit
//! in, and which commits stay shown all the same.
//!
//! git's `--date-order` puts every child before its parents, so what the
//! children tell a commit is complete when it arrives, and nothing worked
//! out is undone when more of the history is read.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use crate::git;
use crate::log::Commit;
use crate::oid::Oid;
use crate::refs::{self, Labels, Refs, Trunk};
use crate::worktree::{self, Worktree};

/// What the history is read against: the refs and the worktrees as they
/// were when the reading started.
#[derive(Clone, Debug, Default)]
pub struct Setup {
    pub refs: Refs,
    pub worktrees: Vec<Worktree>,
    pub trunk: Option<Trunk>,
    /// What points at each commit.
    pub labels: HashMap<Oid, Labels>,
    /// The remotes' names, longest first, to take off a merged branch's name.
    pub remotes: Vec<String>,
}

impl Setup {
    pub fn new(refs: Refs, worktrees: Vec<Worktree>, trunk: Option<Trunk>) -> Setup {
        let labels = refs.labels();
        let remotes = refs.remote_names();
        Setup { refs, worktrees, trunk, labels, remotes }
    }

    /// The setup of the repository `dir` is in.
    pub fn read(dir: &Path) -> Result<Setup, git::Error> {
        let (refs, worktrees) = git::both(|| Refs::read(dir), || worktree::list(dir));
        let (refs, worktrees) = (refs?, worktrees?);
        let main_branch = worktrees.first().and_then(|w| w.branch.as_deref());
        let trunk = refs::trunk(dir, &refs, main_branch)?;
        Ok(Setup::new(refs, worktrees, trunk))
    }

    /// The worktrees' heads, detached ones too: `git log` starts from them
    /// beside the refs, and they and every commit they need down to the
    /// trunk are never folded.
    pub fn heads(&self) -> Vec<Oid> {
        self.worktrees.iter().filter_map(|w| Oid::parse(&w.head)).filter(|h| !h.is_zero()).collect()
    }
}

#[derive(Clone, Debug)]
pub struct Row {
    pub commit: Commit,
    /// On the trunk's first-parent line: the graph's left edge, never folded.
    pub main: bool,
    /// The branch it is taken to be on, for its colour.
    pub name: Option<Arc<str>>,
    /// The row of the merge on the trunk that brought it in.
    pub into: Option<u32>,
    /// Shown even when its merge is folded.
    pub keep: bool,
}

/// What a merge on the trunk folds away.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Group {
    /// The commits it hides (those kept shown are not counted).
    pub hidden: u32,
    /// The branches and remote branches on them: `feat`, `origin/feat`.
    pub names: Vec<String>,
}

/// Whether merges are folded, and which are open all the same (by the
/// merge's commit, so they stay open when the history is read again).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fold {
    pub on: bool,
    pub open: HashSet<Oid>,
}

#[derive(Debug, Default)]
pub struct History {
    setup: Setup,
    trunk_name: Option<Arc<str>>,
    rows: Vec<Row>,
    index: HashMap<Oid, u32>,
    groups: HashMap<u32, Group>,
    /// The next commit on the trunk's line.
    main_next: Option<Oid>,
    /// Names the children passed down, the first one kept.
    names: HashMap<Oid, Arc<str>>,
    /// The oldest trunk merge (largest row) each commit came in by.
    claims: HashMap<Oid, u32>,
    /// Commits a worktree's head needs.
    far: HashSet<Oid>,
}

impl History {
    pub fn new(setup: Setup) -> Self {
        let trunk_name = setup.trunk.as_ref().map(|t| Arc::from(t.name.as_str()));
        let main_next = setup.trunk.as_ref().map(|t| t.head);
        let far = setup.heads().into_iter().collect();
        History { setup, trunk_name, main_next, far, ..Default::default() }
    }

    pub fn setup(&self) -> &Setup {
        &self.setup
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn find(&self, id: &Oid) -> Option<u32> {
        self.index.get(id).copied()
    }

    pub fn labels(&self, id: &Oid) -> Option<&Labels> {
        self.setup.labels.get(id)
    }

    /// What the trunk merge on row `i` folds away.
    pub fn group(&self, i: u32) -> Option<&Group> {
        self.groups.get(&i)
    }

    /// The next commits, older than those already in.
    pub fn extend(&mut self, commits: impl IntoIterator<Item = Commit>) {
        for c in commits {
            self.push(c);
        }
    }

    fn push(&mut self, c: Commit) {
        let i = self.rows.len() as u32;
        let main = self.main_next == Some(c.id);
        if main {
            self.main_next = c.parents.first().copied();
        }
        let labels = self.setup.labels.get(&c.id);
        let pending = self.names.remove(&c.id);
        let name = if main {
            self.trunk_name.clone()
        } else if let Some(l) = labels.filter(|l| !l.locals.is_empty() || !l.remotes.is_empty()) {
            Some(Arc::from(l.locals.first().unwrap_or_else(|| &l.remotes[0].1).as_str()))
        } else {
            pending
        };
        if c.parents.len() > 1 {
            if let Some(m) = merged_name(&c.subject, &self.setup.remotes) {
                self.names.entry(c.parents[1]).or_insert_with(|| Arc::from(m.as_str()));
            }
        }
        if let (Some(n), Some(&p)) = (&name, c.parents.first()) {
            self.names.entry(p).or_insert_with(|| n.clone());
        }

        let into = self.claims.remove(&c.id).filter(|_| !main);
        if main {
            for &p in c.parents.iter().skip(1) {
                claim(&mut self.claims, p, i);
            }
        } else if let Some(m) = into {
            for &p in &c.parents {
                claim(&mut self.claims, p, m);
            }
        }

        let far = self.far.remove(&c.id);
        let marked = labels.is_some_and(|l| !l.locals.is_empty() || !l.tags.is_empty());
        let keep = !main && (far || marked);
        if far && !main {
            self.far.extend(c.parents.iter().copied());
        }
        if let Some(m) = into {
            let g = self.groups.entry(m).or_default();
            if !keep {
                g.hidden += 1;
                if let Some(l) = labels {
                    let names = l.locals.iter().cloned().chain(l.remotes.iter().map(|(r, n)| format!("{r}/{n}")));
                    for n in names {
                        if !g.names.contains(&n) {
                            g.names.push(n);
                        }
                    }
                }
            }
        }

        self.index.insert(c.id, i);
        self.rows.push(Row { commit: c, main, name, into, keep });
    }

    /// Whether row `i` is shown.
    pub fn shown(&self, i: u32, fold: &Fold) -> bool {
        let r = &self.rows[i as usize];
        match r.into {
            Some(m) if fold.on && !r.keep => fold.open.contains(&self.rows[m as usize].commit.id),
            _ => true,
        }
    }

    /// The trunk merge whose folding hides row `i`, if it is hidden.
    pub fn hidden_by(&self, i: u32, fold: &Fold) -> Option<Oid> {
        if self.shown(i, fold) {
            return None;
        }
        self.rows[i as usize].into.map(|m| self.rows[m as usize].commit.id)
    }

    /// Row `i`'s parents as the graph draws them: a folded merge loses its
    /// hidden side; any other edge into a hidden commit goes down first
    /// parents to the nearest one shown, or to one not read yet.
    pub fn parents_shown(&self, i: u32, fold: &Fold) -> Vec<Oid> {
        let r = &self.rows[i as usize];
        let mut out: Vec<Oid> = Vec::with_capacity(r.commit.parents.len());
        for (k, &p) in r.commit.parents.iter().enumerate() {
            let target = match self.find(&p) {
                None => Some(p),
                Some(j) if self.shown(j, fold) => Some(p),
                Some(_) if k > 0 && r.main => None,
                Some(mut j) => loop {
                    let Some(&q) = self.rows[j as usize].commit.parents.first() else { break None };
                    match self.find(&q) {
                        Some(n) if !self.shown(n, fold) => j = n,
                        _ => break Some(q),
                    }
                },
            };
            if let Some(t) = target.filter(|t| *t != r.commit.id && !out.contains(t)) {
                out.push(t);
            }
        }
        out
    }

    /// How many trunk merges fold something away, and how many commits.
    pub fn folded(&self) -> (usize, u64) {
        let full = self.groups.values().filter(|g| g.hidden > 0);
        full.fold((0, 0), |(n, c), g| (n + 1, c + g.hidden as u64))
    }
}

/// `id` came in by the trunk merge on row `merge`, unless by an older one.
fn claim(claims: &mut HashMap<Oid, u32>, id: Oid, merge: u32) {
    let e = claims.entry(id).or_insert(merge);
    *e = (*e).max(merge);
}

/// The branch a merge brought in, from its subject, as GitHub and git word
/// it; a remote's name in front is taken off (`origin/feat` → `feat`).
pub fn merged_name(subject: &str, remotes: &[String]) -> Option<String> {
    let quoted = |rest: &str| rest.split_once('\'').map(|(n, _)| n.to_owned()).filter(|n| !n.is_empty());
    let name = if let Some(rest) = subject.strip_prefix("Merge pull request #") {
        let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let (owner, branch) = rest[digits..].strip_prefix(" from ")?.split_once('/')?;
        (digits > 0 && !owner.is_empty() && !branch.is_empty()).then(|| branch.to_owned())?
    } else if let Some(rest) = subject.strip_prefix("Merge remote-tracking branch '") {
        quoted(rest)?
    } else if let Some(rest) = subject.strip_prefix("Merge branch '") {
        quoted(rest)?
    } else {
        let (word, rest) = subject.strip_prefix("Merge ")?.split_once(' ')?;
        (!word.is_empty() && rest.starts_with("into")).then(|| word.to_owned())?
    };
    let bare = remotes.iter().find_map(|r| name.strip_prefix(r.as_str()).and_then(|n| n.strip_prefix('/')));
    Some(bare.map(str::to_owned).unwrap_or(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::Reader;
    use crate::testrepo::TempRepo;

    fn read(r: &TempRepo) -> History {
        let setup = Setup::read(&r.repo).unwrap();
        let mut log = Reader::start(&r.repo, &setup.heads()).unwrap();
        let mut h = History::new(setup);
        h.extend(log.next_batch(1000).unwrap());
        assert!(log.done());
        h
    }

    fn row(h: &History, subject: &str) -> u32 {
        h.rows().iter().position(|r| r.commit.subject == subject).unwrap_or_else(|| panic!("no {subject}")) as u32
    }

    fn subjects(h: &History, ids: &[Oid]) -> Vec<String> {
        ids.iter().map(|id| h.rows()[h.find(id).unwrap() as usize].commit.subject.clone()).collect()
    }

    fn commit(r: &TempRepo, dir: &Path, subject: &str) {
        r.git(dir, &["commit", "-q", "--allow-empty", "-m", subject]);
    }

    fn merge(r: &TempRepo, branch: &str, subject: &str) {
        r.git(&r.repo, &["merge", "-q", "--no-ff", "--no-edit", "-m", subject, branch]);
    }

    /// `feat` (f1, f2) merged and deleted; `side` off f1, not merged;
    /// `other` (o1) merged and kept; a worktree on `wip` (w1) off the trunk.
    fn branches(name: &str) -> TempRepo {
        let r = TempRepo::new(name);
        r.git(&r.repo, &["checkout", "-q", "-b", "feat"]);
        commit(&r, &r.repo, "f1");
        commit(&r, &r.repo, "f2");
        r.git(&r.repo, &["checkout", "-q", "main"]);
        commit(&r, &r.repo, "m1");
        merge(&r, "feat", "Merge branch 'feat'");
        r.git(&r.repo, &["checkout", "-q", "-b", "side", "feat~1"]);
        commit(&r, &r.repo, "s1");
        r.git(&r.repo, &["checkout", "-q", "main"]);
        r.git(&r.repo, &["branch", "-q", "-D", "feat"]);
        r.git(&r.repo, &["checkout", "-q", "-b", "other"]);
        commit(&r, &r.repo, "o1");
        r.git(&r.repo, &["checkout", "-q", "main"]);
        merge(&r, "other", PR);
        r.git(&r.repo, &["worktree", "add", "-q", "-b", "wip", "../wt"]);
        commit(&r, &r.root.join("wt"), "w1");
        commit(&r, &r.repo, "m2");
        r
    }

    const PR: &str = "Merge pull request #7 from someone/other";
    const FEAT: &str = "Merge branch 'feat'";

    #[test]
    fn the_trunk_and_the_names_come_from_the_refs_and_the_merges() {
        let r = branches("history-names");
        let h = read(&r);
        let order: Vec<&str> = h.rows().iter().map(|r| r.commit.subject.as_str()).collect();
        assert_eq!(order, ["m2", "w1", PR, "o1", "s1", FEAT, "m1", "f2", "f1", "first"]);
        let main: Vec<&str> = h.rows().iter().filter(|r| r.main).map(|r| r.commit.subject.as_str()).collect();
        assert_eq!(main, ["m2", PR, FEAT, "m1", "first"]);
        let name = |s: &str| h.rows()[row(&h, s) as usize].name.as_deref().map(str::to_owned);
        assert_eq!(name("m2").as_deref(), Some("main"));
        assert_eq!(name("w1").as_deref(), Some("wip"));
        assert_eq!(name("o1").as_deref(), Some("other"));
        assert_eq!(name("f2").as_deref(), Some("feat"), "from the merge's subject");
        assert_eq!(name("f1").as_deref(), Some("side"), "the first child to come in names it");
    }

    #[test]
    fn a_trunk_merge_folds_what_it_brought_in() {
        let r = branches("history-fold");
        let h = read(&r);
        let m = row(&h, FEAT);
        let pr = row(&h, PR);
        assert_eq!(h.rows()[row(&h, "f2") as usize].into, Some(m));
        assert_eq!(h.rows()[row(&h, "f1") as usize].into, Some(m));
        assert_eq!(h.rows()[row(&h, "o1") as usize].into, Some(pr));
        assert_eq!(h.rows()[row(&h, "s1") as usize].into, None, "not on the trunk yet");
        assert_eq!(h.group(m), Some(&Group { hidden: 2, names: vec![] }));
        assert_eq!(h.group(pr), Some(&Group { hidden: 0, names: vec![] }), "o1 has a branch: kept");
        assert_eq!(h.folded(), (1, 2));

        let mut fold = Fold { on: true, ..Default::default() };
        let shown: Vec<&str> =
            (0..h.len() as u32).filter(|&i| h.shown(i, &fold)).map(|i| h.rows()[i as usize].commit.subject.as_str()).collect();
        assert_eq!(shown.len(), 8);
        assert!(!shown.contains(&"f1") && !shown.contains(&"f2"));
        let merge_id = h.rows()[m as usize].commit.id;
        assert_eq!(h.hidden_by(row(&h, "f1"), &fold), Some(merge_id));
        assert_eq!(subjects(&h, &h.parents_shown(m, &fold)), ["m1"], "the folded side is dropped");
        assert_eq!(subjects(&h, &h.parents_shown(row(&h, "s1"), &fold)), ["first"], "down to the nearest shown");

        fold.open.insert(merge_id);
        assert!(h.shown(row(&h, "f1"), &fold));
        assert_eq!(h.hidden_by(row(&h, "f1"), &fold), None);
        assert_eq!(subjects(&h, &h.parents_shown(m, &fold)), ["m1", "f2"]);
        assert_eq!(subjects(&h, &h.parents_shown(row(&h, "s1"), &fold)), ["f1"]);

        let off = Fold::default();
        assert!((0..h.len() as u32).all(|i| h.shown(i, &off)));
    }

    #[test]
    fn a_worktree_s_commits_and_a_branch_s_stay_but_a_remote_s_fold() {
        let r = TempRepo::new("history-keep");
        r.git(&r.repo, &["checkout", "-q", "-b", "feat"]);
        commit(&r, &r.repo, "f1");
        commit(&r, &r.repo, "f2");
        r.git(&r.repo, &["checkout", "-q", "main"]);
        let subject = "Merge remote-tracking branch 'origin/feat'";
        merge(&r, "feat", subject);
        r.git(&r.repo, &["update-ref", "refs/remotes/origin/feat", "feat"]);
        r.git(&r.repo, &["worktree", "add", "-q", "--detach", "../old", "feat"]);
        r.git(&r.repo, &["branch", "-q", "-D", "feat"]);

        let h = read(&r);
        let m = row(&h, subject);
        assert!(h.rows()[row(&h, "f2") as usize].keep && h.rows()[row(&h, "f1") as usize].keep, "the worktree needs both");
        assert_eq!(h.folded(), (0, 0));

        r.git(&r.repo, &["worktree", "remove", "../old"]);
        let h = read(&r);
        assert_eq!(h.group(m), Some(&Group { hidden: 2, names: vec!["origin/feat".into()] }));
        assert_eq!(h.rows()[row(&h, "f2") as usize].name.as_deref(), Some("feat"));

        r.git(&r.repo, &["tag", "v1", "origin/feat~1"]);
        let h = read(&r);
        assert!(h.rows()[row(&h, "f1") as usize].keep, "a tag keeps it");
        assert_eq!(h.group(m).map(|g| g.hidden), Some(1));
    }

    #[test]
    fn merged_names_are_read_from_every_wording() {
        let remotes = ["my/fork".to_owned(), "origin".to_owned()];
        let n = |s: &str| merged_name(s, &remotes);
        assert_eq!(n("Merge pull request #12 from uchmk/feat/x").as_deref(), Some("feat/x"));
        assert_eq!(n("Merge branch 'fix' into main").as_deref(), Some("fix"));
        assert_eq!(n("Merge remote-tracking branch 'origin/fix'").as_deref(), Some("fix"));
        assert_eq!(n("Merge remote-tracking branch 'my/fork/fix'").as_deref(), Some("fix"));
        assert_eq!(n("Merge branch 'origin/fix'").as_deref(), Some("fix"));
        assert_eq!(n("Merge test/win-1 into main").as_deref(), Some("test/win-1"));
        assert_eq!(n("Merge pull request # from x/y"), None);
        assert_eq!(n("Merge pull request #3"), None);
        assert_eq!(n("Merge branch ''"), None);
        assert_eq!(n("Merged the fix"), None);
        assert_eq!(n("Add a merge button"), None);
    }

    #[test]
    fn a_history_read_in_batches_is_the_same() {
        let r = branches("history-batches");
        let whole = read(&r);
        let setup = Setup::read(&r.repo).unwrap();
        let mut log = Reader::start(&r.repo, &setup.heads()).unwrap();
        let mut h = History::new(setup);
        while !log.done() {
            h.extend(log.next_batch(3).unwrap());
        }
        let facts = |h: &History| h.rows().iter().map(|r| (r.commit.id, r.main, r.name.clone(), r.into, r.keep)).collect::<Vec<_>>();
        assert_eq!(facts(&h), facts(&whole));
    }
}
