//! Clean-up: the branches done with, merged into the trunk or gone from the
//! remote they followed (a pull request merged with squash leaves its branch
//! so), and the worktrees whose folder is gone, listed to be taken away
//! together. A branch a linked worktree has checked out goes with its
//! worktree; one whose commits nothing else has is kept under
//! `refs/michi/backup/` first. What has to stay is listed with why.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::branch;
use crate::git;
use crate::oid::Oid;
use crate::paths;
use crate::refs::{self, Refs};
use crate::worktree::{self, Worktree};

/// Why a row is listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// The trunk has the branch's commits.
    Merged,
    /// The branch it followed is gone from its remote, and the trunk has not
    /// all its commits: its tip is kept before it goes.
    Gone,
    /// The worktree's folder is gone; its branch, if any, stays.
    Lost,
}

/// Why a row cannot go.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stay {
    /// The branch is checked out in the main worktree.
    Main,
    /// The worktree is the one the window is on.
    Here,
    /// The worktree has uncommitted changes, untracked files too.
    Changes,
    /// The worktree is locked (`git worktree lock`).
    Locked,
    /// A tsumugi tab is open in the worktree (the window tells).
    Open,
}

/// A row of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// The branch to delete; `None` for a worktree whose branch stays.
    pub branch: Option<String>,
    /// Its tip as listed: a branch that has moved since is left alone.
    pub tip: Option<Oid>,
    /// The worktree that goes with it, its folder there or gone.
    pub worktree: Option<PathBuf>,
    pub why: Why,
    /// Whether the worktree's folder is gone.
    pub lost: bool,
    pub stay: Option<Stay>,
}

impl Item {
    /// What the list calls it: the branch, else the worktree's folder.
    pub fn name(&self) -> String {
        match (&self.branch, &self.worktree) {
            (Some(b), _) => b.clone(),
            (None, Some(w)) => folder(w),
            (None, None) => String::new(),
        }
    }
}

/// The list, and the trunk it was measured against.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Found {
    /// The trunk's name (`main`); `None` when the repository has none, and
    /// then no branch is merged.
    pub trunk: Option<String>,
    /// Those that can go first, by why and name; then those that stay.
    pub items: Vec<Item>,
}

/// How a clean-up went.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// What went, as the list calls it.
    pub gone: Vec<String>,
    /// The backups made, one for each branch whose commits nothing else had.
    pub saved: Vec<String>,
    /// What stayed, and why in git's words (empty: it moved since it was
    /// listed).
    pub stayed: Vec<(String, String)>,
}

/// What can be cleaned up in the repository `dir` is in, the window being
/// on the worktree at `here`.
pub fn find(dir: &Path, here: &Path) -> Result<Found, git::Error> {
    let (worktrees, refs) = git::both(|| worktree::list(dir), || Refs::read(dir));
    let (worktrees, refs) = (worktrees?, refs?);
    let main_branch = worktrees.first().and_then(|w| w.branch.as_deref());
    let trunk = refs::trunk(dir, &refs, main_branch)?;
    let merged: HashSet<String> = match &trunk {
        Some(t) => git::read_text(dir, &["for-each-ref", "--format=%(refname)", &format!("--merged={}", t.head), "refs/heads"])?
            .lines()
            .filter_map(|l| l.strip_prefix("refs/heads/"))
            .map(str::to_owned)
            .collect(),
        None => HashSet::new(),
    };
    let trunk_name = trunk.as_ref().map(|t| t.name.as_str());
    let mut items = Vec::new();
    let mut listed: HashSet<String> = HashSet::new();
    for r in &refs.locals {
        if Some(r.name.as_str()) == trunk_name {
            continue;
        }
        let live = at_live(&worktrees, &r.name);
        let why = if merged.contains(&r.name) {
            // A branch that is "merged" only because it has no commits of its
            // own yet (another session's brand-new worktree, still at the
            // commit it was cut from) has not been merged: never offer a live
            // worktree whose branch has no history beyond its creation.
            if live && !has_own_history(dir, &r.name)? {
                continue;
            }
            Why::Merged
        } else if refs.gone.contains(&r.name) {
            Why::Gone
        } else {
            continue;
        };
        let at = worktrees.iter().position(|w| w.branch.as_deref() == Some(r.name.as_str()));
        let (worktree, lost, stay) = match at {
            None => (None, false, None),
            Some(0) => (None, false, Some(Stay::Main)),
            Some(k) => {
                let w = &worktrees[k];
                listed.insert(paths::key(&w.path));
                (Some(w.path.clone()), w.prunable.is_some(), stays(w, here)?)
            }
        };
        items.push(Item { branch: Some(r.name.clone()), tip: Some(r.id), worktree, why, lost, stay });
    }
    // The other worktrees whose folder is gone, their branches staying.
    for w in worktrees.iter().skip(1).filter(|w| w.prunable.is_some() && !listed.contains(&paths::key(&w.path))) {
        items.push(Item { branch: None, tip: None, worktree: Some(w.path.clone()), why: Why::Lost, lost: true, stay: stays(w, here)? });
    }
    sort(&mut items);
    Ok(Found { trunk: trunk.map(|t| t.name), items })
}

/// Whether the branch is checked out in a linked worktree whose folder is there.
fn at_live(worktrees: &[Worktree], name: &str) -> bool {
    worktrees.iter().skip(1).any(|w| w.branch.as_deref() == Some(name) && w.prunable.is_none())
}

/// Whether the branch's reflog shows it ever moved after it was created
/// (`commit`, `merge`, `rebase`, `reset`...). A branch whose reflog says only
/// `branch: Created from ...` (or says nothing: expired, or no reflog) has no
/// evidence of work, so being "merged" into the trunk means nothing.
fn has_own_history(dir: &Path, name: &str) -> Result<bool, git::Error> {
    let log = git::read_text(dir, &["reflog", "show", "--format=%gs", &format!("refs/heads/{name}"), "--"])?;
    Ok(log.lines().any(|l| !l.trim().is_empty() && !l.starts_with("branch: Created")))
}

/// Puts those that can go first, by why and name; then those that stay.
pub fn sort(items: &mut [Item]) {
    items.sort_by_key(|i| (i.stay.is_some(), i.why as u8, i.name()));
}

/// Why the worktree `w` has to stay, if it does.
fn stays(w: &Worktree, here: &Path) -> Result<Option<Stay>, git::Error> {
    if w.locked.is_some() {
        return Ok(Some(Stay::Locked));
    }
    if w.prunable.is_some() {
        return Ok(None);
    }
    if paths::same_path(&w.path, here) {
        return Ok(Some(Stay::Here));
    }
    if branch::doing(&w.path).is_some() || !git::read(&w.path, &["status", "--porcelain", "-z", "--untracked-files=normal"])?.is_empty() {
        return Ok(Some(Stay::Changes));
    }
    Ok(None)
}

/// Takes `picks` away: each one's worktree (`git worktree remove`, which
/// refuses one with changes), then its branch, kept under
/// `refs/michi/backup/` first when nothing else has its commits. One that
/// stays does not stop the others.
pub fn remove(dir: &Path, picks: &[Item]) -> Outcome {
    let mut out = Outcome::default();
    for item in picks.iter().filter(|i| i.stay.is_none()) {
        let name = item.name();
        if let (Some(b), Some(tip)) = (&item.branch, &item.tip) {
            if tip_of(dir, b).ok().flatten().as_ref() != Some(tip) {
                out.stayed.push((name, String::new()));
                continue;
            }
        }
        if let Some(wt) = &item.worktree {
            if let Err(e) = branch::write(dir, &["worktree", "remove", &wt.to_string_lossy()]) {
                out.stayed.push((name, e.to_string()));
                continue;
            }
        }
        if let Some(b) = &item.branch {
            match branch::delete(dir, b) {
                Ok(saved) => out.saved.extend(saved),
                Err(e) => {
                    out.stayed.push((name, e.to_string()));
                    continue;
                }
            }
        }
        out.gone.push(name);
    }
    out
}

/// The commit the branch `name` is on; `None` when there is no such branch.
fn tip_of(dir: &Path, name: &str) -> Result<Option<Oid>, git::Error> {
    let out = git::read_maybe(dir, &["rev-parse", "--verify", "-q", &format!("refs/heads/{name}^{{commit}}")])?;
    Ok(out.and_then(|o| Oid::parse(String::from_utf8_lossy(&o).trim())))
}

/// The folder's name, to call a worktree by.
fn folder(wt: &Path) -> String {
    wt.file_name().map_or_else(|| wt.display().to_string(), |n| n.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup;
    use crate::testrepo::TempRepo;

    fn head(t: &TempRepo, dir: &Path) -> Oid {
        Oid::parse(&t.out(dir, &["rev-parse", "HEAD"])).unwrap()
    }

    fn rows(f: &Found) -> Vec<(String, Why, Option<Stay>)> {
        f.items.iter().map(|i| (i.name(), i.why, i.stay)).collect()
    }

    /// `main` with a commit on top of `first`; `merged` at `first`;
    /// `squashed`, following `origin/squashed` which is gone, with a commit
    /// of its own; `open` with a commit of its own and its upstream there;
    /// `wt-clean` and `wt-dirty`, merged, checked out in worktrees; `wt-gone`
    /// in a worktree whose folder was deleted, with a commit of its own and
    /// no upstream.
    fn repo(name: &str) -> TempRepo {
        let t = TempRepo::configured(name);
        let first = head(&t, &t.repo).to_string();
        t.git(&t.repo, &["commit", "-q", "--allow-empty", "-m", "second"]);
        t.git(&t.repo, &["branch", "merged", &first]);
        for (b, remote) in [("squashed", false), ("open", true)] {
            t.git(&t.repo, &["switch", "-q", "-c", b, "main"]);
            t.git(&t.repo, &["commit", "-q", "--allow-empty", "-m", b]);
            t.git(&t.repo, &["update-ref", &format!("refs/remotes/origin/{b}"), "HEAD"]);
            t.git(&t.repo, &["config", &format!("branch.{b}.remote"), "origin"]);
            t.git(&t.repo, &["config", &format!("branch.{b}.merge"), &format!("refs/heads/{b}")]);
            if !remote {
                t.git(&t.repo, &["update-ref", "-d", &format!("refs/remotes/origin/{b}")]);
            }
        }
        t.git(&t.repo, &["switch", "-q", "main"]);
        t.git(&t.repo, &["remote", "add", "origin", &t.root.join("remote.git").to_string_lossy()]);
        for b in ["wt-clean", "wt-dirty", "wt-gone"] {
            t.git(&t.repo, &["worktree", "add", "-q", "-b", b, t.root.join(b).to_str().unwrap(), "main"]);
            if b != "wt-gone" {
                // Work done in the worktree and landed on the trunk, as a fast-forward.
                t.git(&t.root.join(b), &["commit", "-q", "--allow-empty", "-m", b]);
                t.git(&t.repo, &["merge", "-q", "--ff-only", b]);
            }
        }
        t.write(&t.root.join("wt-dirty"), "new.txt", "new\n");
        let gone = t.root.join("wt-gone");
        t.git(&gone, &["commit", "-q", "--allow-empty", "-m", "lost work"]);
        std::fs::remove_dir_all(&gone).unwrap();
        t
    }

    #[test]
    fn merged_gone_and_lost_are_listed_with_what_stays() {
        let t = repo("tidy-find");
        let f = find(&t.repo, &t.repo).unwrap();
        assert_eq!(f.trunk.as_deref(), Some("main"));
        assert_eq!(
            rows(&f),
            [
                ("merged".to_owned(), Why::Merged, None),
                ("wt-clean".to_owned(), Why::Merged, None),
                ("squashed".to_owned(), Why::Gone, None),
                ("wt-gone".to_owned(), Why::Lost, None),
                ("wt-dirty".to_owned(), Why::Merged, Some(Stay::Changes)),
            ]
        );
        let clean = &f.items[1];
        assert!(paths::same_path(clean.worktree.as_deref().unwrap(), &t.root.join("wt-clean")));
        assert!(!clean.lost);
        let lost = &f.items[3];
        assert_eq!(lost.branch, None, "its branch has work of its own and no upstream: it stays");
        assert!(lost.lost);
        // From the clean worktree, that one stays.
        let f = find(&t.repo, &t.root.join("wt-clean")).unwrap();
        assert_eq!(f.items.iter().find(|i| i.name() == "wt-clean").unwrap().stay, Some(Stay::Here));
    }

    #[test]
    fn a_brand_new_worktree_is_not_merged_just_for_having_no_commits() {
        let t = TempRepo::configured("tidy-fresh");
        t.git(&t.repo, &["worktree", "add", "-q", "-b", "fresh", t.root.join("fresh").to_str().unwrap(), "main"]);
        t.git(&t.repo, &["branch", "plain", "main"]);
        let f = find(&t.repo, &t.repo).unwrap();
        assert_eq!(rows(&f), [("plain".to_owned(), Why::Merged, None)], "a branch with no worktree is still listed");
        // Once it has had a commit of its own and the trunk has it, it is.
        t.git(&t.root.join("fresh"), &["commit", "-q", "--allow-empty", "-m", "work"]);
        t.git(&t.repo, &["merge", "-q", "--ff-only", "fresh"]);
        let f = find(&t.repo, &t.repo).unwrap();
        assert_eq!(rows(&f), [("fresh".to_owned(), Why::Merged, None), ("plain".to_owned(), Why::Merged, None)]);
    }

    #[test]
    fn a_branch_of_the_main_worktree_or_a_locked_one_stays() {
        let t = repo("tidy-stay");
        t.git(&t.repo, &["switch", "-q", "merged"]);
        t.git(&t.repo, &["worktree", "lock", t.root.join("wt-clean").to_str().unwrap()]);
        let f = find(&t.repo, &t.repo).unwrap();
        let stay = |n: &str| f.items.iter().find(|i| i.name() == n).map(|i| i.stay);
        assert_eq!(stay("merged"), Some(Some(Stay::Main)));
        assert_eq!(stay("wt-clean"), Some(Some(Stay::Locked)));
        assert_eq!(stay("main"), None, "the trunk is never listed");
    }

    #[test]
    fn what_was_picked_goes_and_unmerged_tips_are_kept() {
        let t = repo("tidy-remove");
        let squashed = t.out(&t.repo, &["rev-parse", "squashed"]);
        let f = find(&t.repo, &t.repo).unwrap();
        let out = remove(&t.repo, &f.items);
        assert_eq!(out.gone, ["merged", "wt-clean", "squashed", "wt-gone"]);
        assert!(out.stayed.is_empty(), "{:?}", out.stayed);
        assert_eq!(out.saved.len(), 1, "only squashed had commits nothing else has");
        assert!(out.saved[0].starts_with(backup::PREFIX));
        assert_eq!(t.out(&t.repo, &["rev-parse", &out.saved[0]]), squashed);
        let branches = t.out(&t.repo, &["branch", "--format=%(refname:short)"]);
        assert_eq!(branches.lines().collect::<Vec<_>>(), ["main", "open", "wt-dirty", "wt-gone"]);
        assert!(!t.root.join("wt-clean").exists());
        let wts = worktree::list(&t.repo).unwrap();
        let names: Vec<String> = wts.iter().map(|w| folder(&w.path)).collect();
        assert_eq!(names, ["repo", "wt-dirty"]);
        assert!(t.root.join("wt-dirty").join("new.txt").exists());
    }

    #[test]
    fn a_branch_that_moved_or_a_worktree_that_changed_stays() {
        let t = repo("tidy-moved");
        let f = find(&t.repo, &t.repo).unwrap();
        let pick = |n: &str| f.items.iter().find(|i| i.name() == n).unwrap().clone();
        t.git(&t.repo, &["branch", "-f", "merged", "main"]);
        t.write(&t.root.join("wt-clean"), "late.txt", "late\n");
        let out = remove(&t.repo, &[pick("merged"), pick("wt-clean"), pick("wt-dirty")]);
        assert!(out.gone.is_empty(), "{:?}", out.gone);
        assert_eq!(out.stayed.len(), 2, "the one that stays is not tried: {:?}", out.stayed);
        assert_eq!(out.stayed[0], ("merged".to_owned(), String::new()));
        assert_eq!(out.stayed[1].0, "wt-clean");
        assert!(out.stayed[1].1.contains("untracked"), "{:?}", out.stayed[1]);
        assert!(t.root.join("wt-clean").join("late.txt").exists());
        assert_eq!(t.out(&t.repo, &["rev-parse", "merged"]), t.out(&t.repo, &["rev-parse", "main"]));
    }
}
