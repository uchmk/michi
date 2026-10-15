//! Which worktrees would conflict with which, for the graph's ⚠, the
//! worktree table's column and its table of pairs: each two heads put through
//! [`predict`], and, where those merge clean, the two with their uncommitted
//! changes. The calls block; run them off the UI thread.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::oid::Oid;
use crate::predict::{self, Conflict};
use crate::wip::Wip;
use crate::worktree::Worktree;

/// One end of a pair: a worktree, or the trunk when no worktree has it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Side {
    /// The branch, or the folder's name when detached; the trunk's name.
    pub name: String,
    /// The worktree's folder; `None` for the trunk.
    pub path: Option<PathBuf>,
    pub head: Oid,
    /// The commit holding its uncommitted change ([`Wip::commit`]); `head`
    /// when it has none.
    pub tip: Oid,
}

/// How a pair would merge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Clean, uncommitted changes and all.
    Clean,
    /// Their commits conflict.
    Committed(Conflict),
    /// Their commits merge clean; with the uncommitted changes, they do not.
    Uncommitted(Conflict),
    /// git could not tell (unrelated histories).
    Unknown,
}

impl Outcome {
    pub fn conflict(&self) -> Option<&Conflict> {
        match self {
            Outcome::Committed(c) | Outcome::Uncommitted(c) => Some(c),
            Outcome::Clean | Outcome::Unknown => None,
        }
    }
}

/// The sides and how each two of them would merge.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Clashes {
    pub sides: Vec<Side>,
    /// Each pair `(i, j)`, `i < j`, in the order (0, 1), (0, 2), …, (1, 2), ….
    pub pairs: Vec<Outcome>,
}

impl Clashes {
    /// How the sides `a` and `b` would merge; `None` for a side with itself.
    pub fn between(&self, a: usize, b: usize) -> Option<&Outcome> {
        let n = self.sides.len();
        if a == b || a >= n || b >= n {
            return None;
        }
        let (i, j) = (a.min(b), a.max(b));
        self.pairs.get(i * (2 * n - i - 1) / 2 + (j - i - 1))
    }

    /// The sides that `i` conflicts with, and how, in the sides' order.
    pub fn of(&self, i: usize) -> Vec<(usize, &Outcome)> {
        (0..self.sides.len()).filter_map(|j| self.between(i, j).filter(|o| o.conflict().is_some()).map(|o| (j, o))).collect()
    }

    /// The side of the worktree at `path`.
    pub fn side_of(&self, path: &Path) -> Option<usize> {
        self.sides.iter().position(|s| s.path.as_deref() == Some(path))
    }
}

/// The sides to pair: each worktree that has a `HEAD` (bare and gone ones
/// left out), with its uncommitted change from `wips`, and in front of them
/// the trunk `(name, commit)` when no worktree is on it.
pub fn sides(worktrees: &[Worktree], wips: &[Wip], trunk: Option<(&str, Oid)>) -> Vec<Side> {
    let mut sides = Vec::new();
    if let Some((name, head)) = trunk {
        if !worktrees.iter().any(|w| w.branch.as_deref() == Some(name)) {
            sides.push(Side { name: name.to_owned(), path: None, head, tip: head });
        }
    }
    for wt in worktrees.iter().filter(|w| !w.bare && w.prunable.is_none()) {
        let Some(head) = Oid::parse(&wt.head) else { continue };
        // The change's own head: `HEAD` may have moved since the list was read.
        let (head, tip) = wips.iter().find(|w| w.path == wt.path).map_or((head, head), |w| (w.head, w.commit));
        let name = wt.branch.clone().unwrap_or_else(|| wt.path.file_name().map_or_else(String::new, |f| f.to_string_lossy().into_owned()));
        sides.push(Side { name, path: Some(wt.path.clone()), head, tip });
    }
    sides
}

/// The predictions already made, by the two commits: a head or a WIP
/// commit stays the same from one reading to the next until it moves.
#[derive(Default)]
pub struct Cache(HashMap<(Oid, Oid), Option<Conflict>>);

impl Cache {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// How each two of `sides` would merge, in the repository `dir`: their
/// heads, and where those merge clean and either has an uncommitted change,
/// their tips. A prediction git fails at gives [`Outcome::Unknown`] and is
/// asked again next time.
pub fn read(dir: &Path, sides: Vec<Side>, cache: &mut Cache) -> Clashes {
    let n = sides.len();
    let key = |a: Oid, b: Oid| if a <= b { (a, b) } else { (b, a) };
    let pairs: Vec<(usize, usize)> = (0..n).flat_map(|i| (i + 1..n).map(move |j| (i, j))).collect();
    // This reading's predictions; `None` where git failed.
    let mut found: HashMap<(Oid, Oid), Option<Option<Conflict>>> = HashMap::new();
    let heads: Vec<(Oid, Oid)> = pairs.iter().map(|&(i, j)| key(sides[i].head, sides[j].head)).collect();
    look_up(dir, &heads, cache, &mut found);
    let dirty = |i: usize| sides[i].tip != sides[i].head;
    let tips: Vec<(Oid, Oid)> = pairs
        .iter()
        .zip(&heads)
        .filter(|(&(i, j), k)| matches!(found.get(*k), Some(Some(None))) && (dirty(i) || dirty(j)))
        .map(|(&(i, j), _)| key(sides[i].tip, sides[j].tip))
        .collect();
    look_up(dir, &tips, cache, &mut found);
    let outcomes = pairs
        .iter()
        .zip(&heads)
        .map(|(&(i, j), k)| match found.get(k) {
            Some(Some(Some(c))) => Outcome::Committed(c.clone()),
            Some(Some(None)) => match found.get(&key(sides[i].tip, sides[j].tip)) {
                Some(Some(Some(c))) => Outcome::Uncommitted(c.clone()),
                Some(Some(None)) => Outcome::Clean,
                _ => Outcome::Unknown,
            },
            _ => Outcome::Unknown,
        })
        .collect();
    cache.0 = found.into_iter().filter_map(|(k, v)| v.map(|v| (k, v))).collect();
    Clashes { sides, pairs: outcomes }
}

/// Fills `found` for each of `keys`: from `cache`, or from git (a few at a
/// time) for those it does not hold.
fn look_up(dir: &Path, keys: &[(Oid, Oid)], cache: &Cache, found: &mut HashMap<(Oid, Oid), Option<Option<Conflict>>>) {
    let mut ask = Vec::new();
    for &k in keys {
        if found.contains_key(&k) {
            continue;
        }
        let known = if k.0 == k.1 { Some(None) } else { cache.0.get(&k).cloned() };
        match known {
            Some(c) => {
                found.insert(k, Some(c));
            }
            None => {
                // Held, so a pair asked twice is asked once.
                found.insert(k, None);
                ask.push(k);
            }
        }
    }
    let revs: Vec<(String, String)> = ask.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
    for (k, r) in ask.into_iter().zip(predict::predict_many(dir, &revs)) {
        found.insert(k, r.ok());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::predict::ConflictFile;
    use crate::testrepo::TempRepo;
    use crate::{wip, worktree};

    fn id(n: u8) -> Oid {
        Oid::parse(&format!("{n:02x}").repeat(20)).unwrap()
    }

    fn side(name: &str) -> Side {
        Side { name: name.into(), path: None, head: id(1), tip: id(1) }
    }

    fn conflict(path: &str) -> Conflict {
        Conflict { files: vec![ConflictFile { path: path.into(), hunks: Some(1) }] }
    }

    #[test]
    fn pairs_are_found_either_way_round() {
        let c = Clashes {
            sides: ["a", "b", "c", "d"].map(side).to_vec(),
            pairs: vec![
                Outcome::Clean,
                Outcome::Committed(conflict("ac")),
                Outcome::Clean,
                Outcome::Unknown,
                Outcome::Uncommitted(conflict("bd")),
                Outcome::Committed(conflict("cd")),
            ],
        };
        assert_eq!(c.between(0, 2), Some(&Outcome::Committed(conflict("ac"))));
        assert_eq!(c.between(2, 0), c.between(0, 2));
        assert_eq!(c.between(1, 2), Some(&Outcome::Unknown));
        assert_eq!(c.between(3, 1), Some(&Outcome::Uncommitted(conflict("bd"))));
        assert_eq!(c.between(2, 3), Some(&Outcome::Committed(conflict("cd"))));
        assert_eq!((c.between(1, 1), c.between(0, 4)), (None, None));
        let of: Vec<usize> = c.of(2).into_iter().map(|(j, _)| j).collect();
        assert_eq!(of, [0, 3]);
        assert!(c.of(1).len() == 1 && c.of(1)[0].0 == 3);
    }

    #[test]
    fn the_trunk_is_a_side_only_when_no_worktree_is_on_it() {
        let wt = |path: &str, branch: Option<&str>, head: u8| Worktree {
            path: path.into(),
            head: id(head).to_string(),
            branch: branch.map(str::to_owned),
            ..Default::default()
        };
        let gone = Worktree { prunable: Some(String::new()), ..wt("C:/r/gone", Some("g"), 5) };
        let bare = Worktree { bare: true, head: String::new(), ..wt("C:/r/bare", None, 0) };
        let all = [wt("C:/r/repo", Some("main"), 2), wt("C:/r/det", None, 3), gone, bare];
        let names = |s: &[Side]| s.iter().map(|s| s.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(&sides(&all, &[], Some(("main", id(9))))), ["main", "det"]);
        let s = sides(&all[1..], &[], Some(("main", id(9))));
        assert_eq!(names(&s), ["main", "det"]);
        assert_eq!((s[0].path.as_deref(), s[0].head), (None, id(9)));
        // An uncommitted change gives the tip.
        let w = Wip { path: "C:/r/det".into(), branch: None, head: id(3), commit: id(4), files: vec![], written: None, left_out: vec![] };
        let s = sides(&all, &[w], None);
        assert_eq!((s[1].head, s[1].tip), (id(3), id(4)));
        assert_eq!((s[0].head, s[0].tip), (id(2), id(2)));
    }

    /// Ten lines `a`..`j`, with the first and the last replaced.
    fn lines(first: &str, last: &str) -> String {
        format!("{first}\nb\nc\nd\ne\nf\ng\nh\ni\n{last}\n")
    }

    fn commit_in(r: &TempRepo, dir: &Path, content: &str) {
        r.write(dir, "f.txt", content);
        r.git(dir, &["add", "."]);
        r.git(dir, &["commit", "-q", "-m", "f"]);
    }

    #[test]
    fn committed_and_uncommitted_conflicts_are_told_apart() {
        let r = TempRepo::new("clash");
        commit_in(&r, &r.repo, &lines("a", "j"));
        r.git(&r.repo, &["worktree", "add", "-q", "-b", "feat", "../feat"]);
        r.git(&r.repo, &["worktree", "add", "-q", "-b", "side", "../side"]);
        let (feat, side) = (r.root.join("feat"), r.root.join("side"));
        commit_in(&r, &r.repo, &lines("main", "main"));
        commit_in(&r, &feat, &lines("feat", "feat"));
        // Uncommitted only: the last line, which main changed as well.
        r.write(&side, "f.txt", &lines("a", "side"));
        let read_now = |cache: &mut Cache| {
            let wts = worktree::list(&r.repo).unwrap();
            let wips: Vec<Wip> = wip::read_all(&wts).into_iter().filter_map(|w| w.ok().flatten()).collect();
            read(&r.repo, sides(&wts, &wips, None), cache)
        };
        let mut cache = Cache::default();
        let c = read_now(&mut cache);
        let names: Vec<&str> = c.sides.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["main", "feat", "side"]);
        let main_feat = c.between(0, 1).unwrap();
        assert!(matches!(main_feat, Outcome::Committed(x) if x.places() == 2 && x.files[0].path == "f.txt"));
        assert!(matches!(c.between(0, 2), Some(Outcome::Uncommitted(x)) if x.places() == 1));
        assert!(matches!(c.between(1, 2), Some(Outcome::Uncommitted(x)) if x.places() == 1));
        // Three pairs of heads, two of tips.
        assert_eq!(cache.len(), 5);
        // Read again from what was found, and the same.
        assert_eq!(read_now(&mut cache), c);
        // The change undone: clean.
        r.write(&side, "f.txt", &lines("a", "j"));
        let c = read_now(&mut cache);
        assert_eq!((c.between(0, 2), c.between(1, 2)), (Some(&Outcome::Clean), Some(&Outcome::Clean)));
        assert_eq!(c.of(0).len(), 1);
    }
}
