//! How each worktree stands for the worktree table: its commits against the
//! trunk's, and its branch against the branch it pushes to. The calls block;
//! run them off the UI thread.

use std::collections::HashMap;
use std::path::Path;

use crate::branch::{self, Doing};
use crate::git;
use crate::oid::Oid;
use crate::refs::{self, Refs};
use crate::worktree::Worktree;

/// A branch's upstream (`origin/feat/x`) and how far apart they are.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Upstream {
    pub name: String,
    /// Commits the branch has that the upstream has not (not pushed).
    pub ahead: u32,
    /// … and the other way (not pulled).
    pub behind: u32,
    /// The upstream branch is no more (deleted on the remote after a merge).
    pub gone: bool,
}

/// A worktree with how it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tree {
    pub wt: Worktree,
    /// Commits its `HEAD` has that the trunk has not, and the other way;
    /// `None` without a trunk or a `HEAD` (bare, gone, unborn).
    pub ahead_behind: Option<(u32, u32)>,
    /// Its branch's upstream; `None` when it has none (or no branch).
    pub upstream: Option<Upstream>,
    /// What it is in the middle of (a merge, a rebase).
    pub doing: Option<Doing>,
    /// The branch of another worktree that already holds all of this one's
    /// commits (it was merged in there, not yet on the trunk); `None` when
    /// no worktree does or this one has nothing the trunk has not.
    pub merged_into: Option<String>,
}

/// The worktrees and the trunk they are measured against.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Trees {
    /// The trunk's name (`main`); `None` when the repository has no branch.
    pub trunk: Option<String>,
    /// The trunk's commit: the local branch's, or its remote's when that is
    /// ahead.
    pub trunk_head: Option<Oid>,
    pub trees: Vec<Tree>,
}

/// The counts already made, by `(trunk, head)`: most heads stay where they
/// were from one reading to the next.
#[derive(Default)]
pub struct Cache(HashMap<(Oid, Oid), (u32, u32)>, HashMap<(Oid, Oid), bool>);

/// How each of `worktrees` (of the repository `dir` is in; bare ones left
/// out) stands. A count that fails is left out, not the whole reading.
pub fn read(dir: &Path, worktrees: &[Worktree], cache: &mut Cache) -> Result<Trees, git::Error> {
    let (refs, ups) = git::both(|| Refs::read(dir), || upstreams(dir));
    let (refs, ups) = (refs?, ups?);
    let main_branch = worktrees.first().and_then(|w| w.branch.as_deref());
    let trunk = refs::trunk(dir, &refs, main_branch)?;
    let mut counted = HashMap::new();
    let mut trees = Vec::new();
    for wt in worktrees.iter().filter(|w| !w.bare) {
        let head = Oid::parse(&wt.head).filter(|_| wt.prunable.is_none());
        let ahead_behind = match (&trunk, head) {
            (Some(t), Some(h)) => {
                let key = (t.head, h);
                let got = match cache.0.get(&key) {
                    Some(&n) => Some(n),
                    None => ahead_behind(dir, &t.head, &h).ok(),
                };
                if let Some(n) = got {
                    counted.insert(key, n);
                }
                got
            }
            _ => None,
        };
        let upstream = wt.branch.as_ref().and_then(|b| ups.get(b)).cloned();
        let doing = branch::doing(&wt.path);
        trees.push(Tree { wt: wt.clone(), ahead_behind, upstream, doing, merged_into: None });
    }
    cache.0 = counted;
    let trunk_name = trunk.as_ref().map(|t| t.name.as_str());
    mark_merged(dir, &mut trees, trunk_name, &mut cache.1);
    Ok(Trees { trunk_head: trunk.as_ref().map(|t| t.head), trunk: trunk.map(|t| t.name), trees })
}

/// Sets `merged_into` of the worktrees whose commits another one's branch
/// holds already. Of several, the one with the fewest commits ahead (the
/// nearest). `known` keeps the answers by `(head, other head)`.
fn mark_merged(dir: &Path, trees: &mut [Tree], trunk: Option<&str>, known: &mut HashMap<(Oid, Oid), bool>) {
    // (index, head, branch, ahead) of the worktrees with something the trunk has not.
    let live: Vec<(usize, Oid, String, u32)> = trees
        .iter()
        .enumerate()
        .filter(|(_, t)| t.doing.is_none() && t.wt.prunable.is_none())
        .filter_map(|(i, t)| {
            let branch = t.wt.branch.clone().filter(|b| Some(b.as_str()) != trunk)?;
            let ahead = t.ahead_behind.map(|(a, _)| a).filter(|&a| a > 0)?;
            Some((i, Oid::parse(&t.wt.head)?, branch, ahead))
        })
        .collect();
    let mut kept = HashMap::new();
    for (i, head, _, ahead) in &live {
        let mut best: Option<(u32, &str)> = None;
        for (j, other, name, other_ahead) in &live {
            if i == j || head == other || other_ahead < ahead {
                continue;
            }
            let key = (*head, *other);
            let held = known.get(&key).copied().or_else(|| is_ancestor(dir, head, other).ok());
            if let Some(h) = held {
                kept.insert(key, h);
                if h && best.is_none_or(|(n, _)| *other_ahead < n) {
                    best = Some((*other_ahead, name));
                }
            }
        }
        trees[*i].merged_into = best.map(|(_, n)| n.to_owned());
    }
    *known = kept;
}

/// Whether `old` is `new` or one of its ancestors.
pub fn is_ancestor(dir: &Path, old: &Oid, new: &Oid) -> Result<bool, git::Error> {
    git::check(dir, &["merge-base", "--is-ancestor", &old.to_string(), &new.to_string()])
}

/// Commits `head` has that `base` has not, and the other way.
pub fn ahead_behind(dir: &Path, base: &Oid, head: &Oid) -> Result<(u32, u32), git::Error> {
    if base == head {
        return Ok((0, 0));
    }
    let out = git::read_text(dir, &["rev-list", "--left-right", "--count", &format!("{base}...{head}")])?;
    let mut n = out.split_whitespace().map(str::parse::<u32>);
    match (n.next(), n.next()) {
        (Some(Ok(behind)), Some(Ok(ahead))) => Ok((ahead, behind)),
        _ => Err(git::Error::Io(format!("git rev-list --count: unexpected output: {out}"))),
    }
}

/// Each local branch's upstream, by the branch's name.
pub fn upstreams(dir: &Path) -> Result<HashMap<String, Upstream>, git::Error> {
    let format = "--format=%(refname)%00%(upstream:short)%00%(upstream:track,nobracket)";
    Ok(parse_upstreams(&git::read(dir, &["for-each-ref", format, "refs/heads"])?))
}

/// `for-each-ref`'s lines: the branch, its upstream, and `ahead 1, behind
/// 2` or `gone` (nothing when the two are the same).
pub fn parse_upstreams(out: &[u8]) -> HashMap<String, Upstream> {
    let text = String::from_utf8_lossy(out);
    let mut all = HashMap::new();
    for line in text.lines() {
        let mut f = line.split('\0');
        let (Some(name), Some(up), track) = (f.next(), f.next(), f.next().unwrap_or("")) else { continue };
        let Some(branch) = name.strip_prefix("refs/heads/") else { continue };
        if up.is_empty() {
            continue;
        }
        let mut u = Upstream { name: up.to_owned(), ahead: 0, behind: 0, gone: track.trim() == "gone" };
        for part in track.split(',') {
            let mut w = part.split_whitespace();
            match (w.next(), w.next().and_then(|n| n.parse().ok())) {
                (Some("ahead"), Some(n)) => u.ahead = n,
                (Some("behind"), Some(n)) => u.behind = n,
                _ => {}
            }
        }
        all.insert(branch.to_owned(), u);
    }
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;
    use crate::worktree;

    #[test]
    fn upstream_lines_are_read() {
        let out = b"refs/heads/main\0origin/main\0\nrefs/heads/feat\0origin/feat\0ahead 2, behind 3\n\
refs/heads/old\0origin/old\0gone\nrefs/heads/local\0\0\nrefs/heads/up\0origin/up\0behind 1\n";
        let u = parse_upstreams(out);
        assert_eq!(u.len(), 4);
        assert_eq!(u["main"], Upstream { name: "origin/main".into(), ahead: 0, behind: 0, gone: false });
        assert_eq!((u["feat"].ahead, u["feat"].behind), (2, 3));
        assert!(u["old"].gone);
        assert_eq!((u["up"].ahead, u["up"].behind), (0, 1));
        assert!(!u.contains_key("local"));
    }

    #[test]
    fn a_branch_held_by_another_worktree_is_merged_into_it() {
        let r = TempRepo::new("merged");
        r.git(&r.repo, &["worktree", "add", "-q", "-b", "a", "../wa"]);
        r.git(&r.repo, &["worktree", "add", "-q", "-b", "b", "../wb"]);
        r.git(&r.repo, &["worktree", "add", "-q", "-b", "c", "../wc"]);
        let (wa, wb, wc) = (r.root.join("wa"), r.root.join("wb"), r.root.join("wc"));
        r.git(&wa, &["commit", "-q", "--allow-empty", "-m", "a1"]);
        r.git(&wc, &["commit", "-q", "--allow-empty", "-m", "c1"]);
        r.git(&wb, &["commit", "-q", "--allow-empty", "-m", "b1"]);
        r.git(&wb, &["merge", "-q", "--no-edit", "a"]);
        let all = worktree::list(&r.repo).unwrap();
        let mut cache = Cache::default();
        let t = read(&r.repo, &all, &mut cache).unwrap();
        let by = |n: &str| t.trees.iter().find(|x| x.wt.branch.as_deref() == Some(n)).unwrap();
        assert_eq!(by("a").merged_into.as_deref(), Some("b"));
        assert_eq!(by("b").merged_into, None);
        assert_eq!(by("c").merged_into, None);
        assert_eq!(by("main").merged_into, None);
        // Read again, from the answers kept.
        assert_eq!(read(&r.repo, &all, &mut cache).unwrap(), t);
    }

    #[test]
    fn each_worktree_against_the_trunk_and_its_upstream() {
        let r = TempRepo::new("track");
        // A bare "remote" to push to.
        let far = r.root.join("far.git");
        r.git(&r.root, &["init", "-q", "--bare", "far.git"]);
        r.git(&r.repo, &["remote", "add", "origin", &far.to_string_lossy()]);
        r.git(&r.repo, &["push", "-q", "-u", "origin", "main"]);
        r.git(&r.repo, &["worktree", "add", "-q", "-b", "feat", "../wt"]);
        let wt = r.root.join("wt");
        r.git(&wt, &["commit", "-q", "--allow-empty", "-m", "f1"]);
        r.git(&wt, &["commit", "-q", "--allow-empty", "-m", "f2"]);
        r.git(&wt, &["push", "-q", "-u", "origin", "feat"]);
        r.git(&wt, &["commit", "-q", "--allow-empty", "-m", "f3"]);
        r.git(&r.repo, &["commit", "-q", "--allow-empty", "-m", "m1"]);
        r.git(&r.repo, &["worktree", "add", "-q", "--detach", "../det", "HEAD~1"]);
        let all = worktree::list(&r.repo).unwrap();
        let mut cache = Cache::default();
        let t = read(&r.repo, &all, &mut cache).unwrap();
        assert_eq!(t.trunk.as_deref(), Some("main"));
        assert_eq!(t.trunk_head.map(|h| h.to_string()), Some(r.out(&r.repo, &["rev-parse", "main"])));
        assert_eq!(t.trees.len(), 3);
        let main = &t.trees[0];
        assert_eq!(main.ahead_behind, Some((0, 0)));
        assert_eq!(main.upstream, Some(Upstream { name: "origin/main".into(), ahead: 1, behind: 0, gone: false }));
        let by = |b: Option<&str>| t.trees.iter().find(|x| x.wt.branch.as_deref() == b).unwrap();
        let feat = by(Some("feat"));
        assert_eq!(feat.ahead_behind, Some((3, 1)));
        assert_eq!(feat.upstream.as_ref().map(|u| (u.ahead, u.behind)), Some((1, 0)));
        let det = by(None);
        assert_eq!(det.ahead_behind, Some((0, 1)));
        assert_eq!(det.upstream, None);
        // Read again, from the counts made.
        assert_eq!(cache.0.len(), 3);
        assert_eq!(read(&r.repo, &all, &mut cache).unwrap(), t);
    }
}
