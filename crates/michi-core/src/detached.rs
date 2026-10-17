//! A worktree whose HEAD is on no branch: why it is (a rebase, a bisect, a
//! tag or commit checked out), which branch it came from, and how many
//! commits only it holds, so the way back is plain. Going back keeps those
//! commits under `refs/michi/backup/` first.

use std::path::Path;

use crate::backup;
use crate::branch;
use crate::git;
use crate::oid::Oid;

/// Why HEAD is on no branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Why {
    /// In the middle of a rebase.
    Rebase,
    /// In the middle of a bisect.
    Bisect,
    /// A tag was checked out (its name).
    Tag(String),
    /// A commit (or a remote branch) was checked out.
    Checkout,
}

/// What is known of a detached HEAD.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Info {
    pub why: Why,
    /// The local branch this worktree was on before, when it still exists.
    pub from: Option<String>,
    /// Commits that no branch, remote branch or tag has: lost when HEAD
    /// moves away without a branch made here.
    pub only_here: usize,
}

/// Reads the state of the detached HEAD of the worktree at `wt`.
pub fn read(wt: &Path) -> Result<Info, git::Error> {
    let why = match branch::doing(wt) {
        Some(branch::Doing::Rebase) => Why::Rebase,
        Some(branch::Doing::Bisect) => Why::Bisect,
        _ => match git::read_text(wt, &["tag", "--points-at", "HEAD"])?.lines().next() {
            Some(tag) => Why::Tag(tag.trim().to_owned()),
            None => Why::Checkout,
        },
    };
    let from = came_from(wt)?;
    let only_here = only_here(wt)?;
    Ok(Info { why, from, only_here })
}

/// The branch the HEAD reflog says we left last that still exists.
fn came_from(wt: &Path) -> Result<Option<String>, git::Error> {
    let log = match git::read_maybe(wt, &["reflog", "show", "--format=%gs", "-n", "300", "HEAD"])? {
        Some(out) => String::from_utf8_lossy(&out).into_owned(),
        None => return Ok(None),
    };
    for name in left_branches(&log) {
        let full = format!("refs/heads/{name}");
        if git::read_maybe(wt, &["show-ref", "--verify", "-q", &full])?.is_some() {
            return Ok(Some(name.to_owned()));
        }
    }
    Ok(None)
}

/// The `X` of each `checkout: moving from X to Y` line, newest first.
fn left_branches(log: &str) -> impl Iterator<Item = &str> {
    log.lines().filter_map(|l| l.strip_prefix("checkout: moving from ")?.split_once(" to ").map(|(from, _)| from))
}

fn only_here(wt: &Path) -> Result<usize, git::Error> {
    let out = git::read_text(wt, &["rev-list", "--count", "HEAD", "--not", "--branches", "--remotes", "--tags"])?;
    Ok(out.trim().parse().unwrap_or(0))
}

/// Switches the worktree at `wt` back to `name`. Commits only the detached
/// HEAD holds are kept under `refs/michi/backup/` first (the ref's name is
/// returned).
pub fn back(wt: &Path, name: &str) -> Result<Option<String>, git::Error> {
    let saved = if only_here(wt)? > 0 { Some(save(wt, &format!("before leaving a detached HEAD for {name}"))?) } else { None };
    branch::switch(wt, name)?;
    Ok(saved)
}

/// Makes the branch `name` at the detached HEAD of `wt` and switches to it:
/// nothing is left behind, so nothing is kept.
pub fn branch_here(wt: &Path, name: &str) -> Result<(), git::Error> {
    let head = head(wt)?;
    branch::create(wt, name, &head, Some(wt))
}

fn head(wt: &Path) -> Result<Oid, git::Error> {
    let out = git::read_text(wt, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    Oid::parse(out.trim()).ok_or_else(|| git::Error::Git("no commit at HEAD".to_owned()))
}

fn save(wt: &Path, why: &str) -> Result<String, git::Error> {
    backup::save(wt, &head(wt)?, why)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn commit(t: &TempRepo, msg: &str) {
        t.git(&t.repo, &["commit", "-q", "--allow-empty", "-m", msg]);
    }

    #[test]
    fn the_branch_left_is_found_in_the_reflog() {
        let log = "checkout: moving from abc1234 to def5678\ncheckout: moving from feat to abc1234\ncommit: x";
        assert_eq!(left_branches(log).collect::<Vec<_>>(), ["abc1234", "feat"]);
    }

    #[test]
    fn a_checked_out_commit_remembers_its_branch() {
        let t = TempRepo::configured("detached-from");
        t.git(&t.repo, &["switch", "-q", "-c", "feat"]);
        commit(&t, "two");
        t.git(&t.repo, &["checkout", "-q", "--detach", "main"]);
        let i = read(&t.repo).unwrap();
        assert_eq!(i, Info { why: Why::Checkout, from: Some("feat".into()), only_here: 0 });
        // One more hop: the branch is still the one left.
        t.git(&t.repo, &["checkout", "-q", "--detach", "feat"]);
        assert_eq!(read(&t.repo).unwrap().from.as_deref(), Some("feat"));
    }

    #[test]
    fn a_tag_is_the_reason() {
        let t = TempRepo::configured("detached-tag");
        t.git(&t.repo, &["tag", "v1"]);
        t.git(&t.repo, &["checkout", "-q", "v1"]);
        let i = read(&t.repo).unwrap();
        assert_eq!(i.why, Why::Tag("v1".into()));
        assert_eq!(i.from.as_deref(), Some("main"));
    }

    #[test]
    fn a_deleted_branch_is_not_offered() {
        let t = TempRepo::configured("detached-gone");
        t.git(&t.repo, &["switch", "-q", "-c", "gone"]);
        t.git(&t.repo, &["checkout", "-q", "--detach", "main"]);
        t.git(&t.repo, &["branch", "-q", "-D", "gone"]);
        // The one before it, since it is gone.
        assert_eq!(read(&t.repo).unwrap().from.as_deref(), Some("main"));
    }

    #[test]
    fn a_rebase_is_the_reason() {
        let t = TempRepo::configured("detached-rebase");
        t.git(&t.repo, &["checkout", "-q", "--detach"]);
        let dir = branch::git_dir(&t.repo).unwrap();
        std::fs::create_dir_all(dir.join("rebase-merge")).unwrap();
        assert_eq!(read(&t.repo).unwrap().why, Why::Rebase);
    }

    #[test]
    fn going_back_keeps_commits_no_branch_has() {
        let t = TempRepo::configured("detached-back");
        t.git(&t.repo, &["checkout", "-q", "--detach"]);
        commit(&t, "lonely");
        let lonely = t.out(&t.repo, &["rev-parse", "HEAD"]);
        assert_eq!(read(&t.repo).unwrap().only_here, 1);
        let saved = back(&t.repo, "main").unwrap().expect("a backup");
        assert_eq!(t.out(&t.repo, &["rev-parse", &saved]), lonely);
        assert_eq!(t.out(&t.repo, &["symbolic-ref", "--short", "HEAD"]), "main");
    }

    #[test]
    fn going_back_without_news_keeps_nothing() {
        let t = TempRepo::configured("detached-back-plain");
        t.git(&t.repo, &["checkout", "-q", "--detach"]);
        assert_eq!(back(&t.repo, "main").unwrap(), None);
        assert_eq!(t.out(&t.repo, &["symbolic-ref", "--short", "HEAD"]), "main");
    }

    #[test]
    fn a_branch_made_here_holds_the_commits() {
        let t = TempRepo::configured("detached-here");
        t.git(&t.repo, &["checkout", "-q", "--detach"]);
        commit(&t, "work");
        let tip = t.out(&t.repo, &["rev-parse", "HEAD"]);
        branch_here(&t.repo, "rescue").unwrap();
        assert_eq!(t.out(&t.repo, &["symbolic-ref", "--short", "HEAD"]), "rescue");
        assert_eq!(t.out(&t.repo, &["rev-parse", "rescue"]), tip);
    }
}
