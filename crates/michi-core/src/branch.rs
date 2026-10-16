//! Branches: make one, switch a worktree to one, delete one. A branch whose
//! commits no other ref has is kept under `refs/michi/backup/` before it
//! goes. And what a worktree is in the middle of (a merge, a rebase), which
//! stops a switch there.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::backup;
use crate::git;
use crate::oid::Oid;
use crate::worktree;

/// What a worktree is in the middle of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Doing {
    Merge,
    Rebase,
    CherryPick,
    Revert,
    Bisect,
}

impl Doing {
    /// git's word for it, the same in every language.
    pub fn word(self) -> &'static str {
        match self {
            Doing::Merge => "merge",
            Doing::Rebase => "rebase",
            Doing::CherryPick => "cherry-pick",
            Doing::Revert => "revert",
            Doing::Bisect => "bisect",
        }
    }
}

/// What the worktree at `wt` is in the middle of, from the files git leaves
/// in its own folder; `None` when nothing (or when its folder cannot be
/// found). Reads a file or two, no git.
pub fn doing(wt: &Path) -> Option<Doing> {
    let dir = git_dir(wt)?;
    let has = |name: &str| dir.join(name).exists();
    if has("rebase-merge") || has("rebase-apply") {
        Some(Doing::Rebase)
    } else if has("MERGE_HEAD") {
        Some(Doing::Merge)
    } else if has("CHERRY_PICK_HEAD") {
        Some(Doing::CherryPick)
    } else if has("REVERT_HEAD") {
        Some(Doing::Revert)
    } else if has("BISECT_LOG") {
        Some(Doing::Bisect)
    } else {
        None
    }
}

/// The worktree's own git folder: `.git` itself, or where its `.git` file
/// points (`gitdir: …/.git/worktrees/x`).
pub(crate) fn git_dir(wt: &Path) -> Option<PathBuf> {
    let dot = wt.join(".git");
    let meta = fs::metadata(&dot).ok()?;
    if meta.is_dir() {
        return Some(dot);
    }
    let text = fs::read_to_string(&dot).ok()?;
    let to = text.lines().find_map(|l| l.strip_prefix("gitdir:"))?.trim();
    let to = Path::new(to);
    Some(if to.is_absolute() { to.to_owned() } else { wt.join(to) })
}

/// Whether `name` would do as a new branch's name; why not when not, in
/// git's words.
pub fn check_name(dir: &Path, name: &str) -> Result<(), git::Error> {
    if name.is_empty() || name.starts_with('-') {
        return Err(git::Error::Git(format!("'{name}' is not a valid branch name")));
    }
    git::read(dir, &["check-ref-format", "--branch", name]).map(|_| ())
}

/// Makes the branch `name` at `at`; with `switch`, switches that worktree to
/// it too. A name that is taken is refused.
pub fn create(dir: &Path, name: &str, at: &Oid, switch: Option<&Path>) -> Result<(), git::Error> {
    check_name(dir, name)?;
    let at = at.to_string();
    match switch {
        Some(wt) => write(wt, &["switch", "--no-guess", "-c", name, &at]),
        None => write(dir, &["branch", "--no-track", name, &at]),
    }
}

/// Switches the worktree at `wt` to the branch `name`. Uncommitted changes
/// go along when they can; git refuses when they would be lost.
pub fn switch(wt: &Path, name: &str) -> Result<(), git::Error> {
    write(wt, &["switch", "--no-guess", name])
}

/// Makes the branch `name` from the remote branch `remote` (`origin/feat`),
/// following it, and switches the worktree at `wt` to it.
pub fn track(wt: &Path, remote: &str, name: &str) -> Result<(), git::Error> {
    check_name(wt, name)?;
    write(wt, &["switch", "--no-guess", "-c", name, "--track", remote])
}

/// Renames the branch `old` to `new` (a worktree on it follows; the
/// upstream setting goes along). A name that is taken is refused.
pub fn rename(dir: &Path, old: &str, new: &str) -> Result<(), git::Error> {
    check_name(dir, new)?;
    write(dir, &["branch", "-m", old, new])
}

/// Deletes the branch `name`. When no other branch, remote branch or tag
/// has its tip, the tip is kept under `refs/michi/backup/` first: the
/// backup's name then. A branch a worktree has checked out is refused.
pub fn delete(dir: &Path, name: &str) -> Result<Option<String>, git::Error> {
    if let Some(w) = worktree::list(dir)?.into_iter().find(|w| w.branch.as_deref() == Some(name)) {
        return Err(git::Error::Git(format!("'{name}' is checked out at '{}'", w.path.display())));
    }
    let full = format!("refs/heads/{name}");
    let tip = git::read_maybe(dir, &["rev-parse", "--verify", "-q", &format!("{full}^{{commit}}")])?
        .and_then(|out| Oid::parse(String::from_utf8_lossy(&out).trim()))
        .ok_or_else(|| git::Error::Git(format!("no branch '{name}'")))?;
    let saved = if kept(dir, &tip, &full)? { None } else { Some(backup::save(dir, &tip, &format!("deleted branch {name}"))?) };
    write(dir, &["branch", "-D", name])?;
    Ok(saved)
}

/// Whether a ref other than `but` (a branch, a remote branch, a tag) has `tip`.
fn kept(dir: &Path, tip: &Oid, but: &str) -> Result<bool, git::Error> {
    let out = git::read_text(
        dir,
        &["for-each-ref", "--format=%(refname)%00%(symref)", "--contains", &tip.to_string(), "refs/heads", "refs/remotes", "refs/tags"],
    )?;
    Ok(out.lines().filter_map(|l| l.split_once('\0')).any(|(r, sym)| r != but && sym.is_empty()))
}

/// Runs git in `dir` to change something, keeping all it said when it
/// refuses.
pub(crate) fn write(dir: &Path, args: &[&str]) -> Result<(), git::Error> {
    let mut c: Command = git::command();
    let out = c.arg("-C").arg(dir).args(args).output().map_err(git::started)?;
    if out.status.success() {
        Ok(())
    } else {
        Err(git::refused_all(&String::from_utf8_lossy(&out.stderr)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn head(t: &TempRepo, dir: &Path) -> Oid {
        Oid::parse(&t.out(dir, &["rev-parse", "HEAD"])).unwrap()
    }

    #[test]
    fn a_branch_is_renamed_and_a_taken_name_refused() {
        let t = TempRepo::configured("branch-rename");
        let at = head(&t, &t.repo);
        create(&t.repo, "old", &at, None).unwrap();
        create(&t.repo, "other", &at, None).unwrap();
        assert!(rename(&t.repo, "old", "other").is_err());
        assert!(rename(&t.repo, "old", "bad name").is_err());
        rename(&t.repo, "old", "new").unwrap();
        let names = t.out(&t.repo, &["branch", "--format=%(refname:short)"]);
        assert!(names.lines().any(|l| l == "new") && !names.lines().any(|l| l == "old"), "{names}");
    }

    #[test]
    fn a_branch_is_made_where_asked_and_a_taken_name_refused() {
        let t = TempRepo::configured("branch-make");
        let first = head(&t, &t.repo);
        t.git(&t.repo, &["commit", "-q", "--allow-empty", "-m", "second"]);
        create(&t.repo, "feat/x", &first, None).unwrap();
        assert_eq!(t.out(&t.repo, &["rev-parse", "feat/x"]), first.to_string());
        assert_eq!(t.out(&t.repo, &["symbolic-ref", "--short", "HEAD"]), "main", "made, not switched to");
        let e = create(&t.repo, "feat/x", &first, None).unwrap_err();
        assert!(e.to_string().contains("already exists"), "{e}");
        for bad in ["", "-x", "a..b", "a b", "x.lock"] {
            assert!(create(&t.repo, bad, &first, None).is_err(), "{bad:?}");
        }
        create(&t.repo, "next", &first, Some(&t.repo)).unwrap();
        assert_eq!(t.out(&t.repo, &["symbolic-ref", "--short", "HEAD"]), "next");
        assert_eq!(head(&t, &t.repo), first);
    }

    #[test]
    fn a_switch_takes_the_changes_along_or_says_why_not() {
        let t = TempRepo::configured("branch-switch");
        t.write(&t.repo, "a.txt", "1\n");
        t.git(&t.repo, &["add", "a.txt"]);
        t.git(&t.repo, &["commit", "-q", "-m", "a"]);
        t.git(&t.repo, &["branch", "old"]);
        t.write(&t.repo, "a.txt", "2\n");
        t.git(&t.repo, &["commit", "-q", "-am", "a2"]);
        t.write(&t.repo, "b.txt", "new\n");
        switch(&t.repo, "old").unwrap();
        assert_eq!(t.out(&t.repo, &["symbolic-ref", "--short", "HEAD"]), "old");
        assert!(t.repo.join("b.txt").exists(), "an untracked file goes along");
        switch(&t.repo, "main").unwrap();
        t.write(&t.repo, "a.txt", "mine\n");
        let e = switch(&t.repo, "old").unwrap_err();
        assert!(e.all().contains("a.txt"), "{e:?}");
        assert_eq!(t.out(&t.repo, &["symbolic-ref", "--short", "HEAD"]), "main");
        assert!(switch(&t.repo, "nope").is_err());
    }

    #[test]
    fn a_remote_branch_is_followed() {
        let t = TempRepo::configured("branch-track");
        let first = head(&t, &t.repo);
        t.git(&t.repo, &["remote", "add", "origin", &t.root.join("remote.git").to_string_lossy()]);
        t.git(&t.repo, &["update-ref", "refs/remotes/origin/feat", &first.to_string()]);
        t.git(&t.repo, &["commit", "-q", "--allow-empty", "-m", "second"]);
        track(&t.repo, "origin/feat", "feat").unwrap();
        assert_eq!(t.out(&t.repo, &["symbolic-ref", "--short", "HEAD"]), "feat");
        assert_eq!(head(&t, &t.repo), first);
        assert_eq!(t.out(&t.repo, &["rev-parse", "--abbrev-ref", "feat@{upstream}"]), "origin/feat");
    }

    #[test]
    fn a_branch_only_it_has_is_kept_before_it_goes() {
        let t = TempRepo::configured("branch-delete");
        t.git(&t.repo, &["branch", "merged"]);
        t.git(&t.repo, &["switch", "-q", "-c", "own"]);
        t.git(&t.repo, &["commit", "-q", "--allow-empty", "-m", "own work"]);
        let own = head(&t, &t.repo);
        t.git(&t.repo, &["switch", "-q", "main"]);
        assert_eq!(delete(&t.repo, "merged").unwrap(), None, "main has its tip");
        let saved = delete(&t.repo, "own").unwrap().expect("a backup");
        assert!(saved.starts_with(backup::PREFIX), "{saved}");
        assert_eq!(t.out(&t.repo, &["rev-parse", &saved]), own.to_string());
        assert_eq!(t.out(&t.repo, &["reflog", "-1", "--format=%gs", &saved]), "michi: deleted branch own");
        assert_eq!(t.out(&t.repo, &["branch", "--format=%(refname:short)"]), "main");
        let e = delete(&t.repo, "main").unwrap_err();
        assert!(e.to_string().contains("checked out"), "{e}");
        assert_eq!(delete(&t.repo, "nope").unwrap_err().to_string(), "no branch 'nope'");
        let backups = t.out(&t.repo, &["for-each-ref", "--format=%(refname)", backup::PREFIX]);
        assert_eq!(backups.lines().count(), 1, "nothing kept for the refusals");
    }

    #[test]
    fn what_a_worktree_is_in_the_middle_of() {
        let t = TempRepo::configured("branch-doing");
        let wt = t.root.join("wt");
        t.git(&t.repo, &["worktree", "add", "-q", "-b", "side", wt.to_str().unwrap()]);
        assert_eq!(doing(&t.repo), None);
        assert_eq!(doing(&wt), None);
        t.write(&t.repo, "f.txt", "main\n");
        t.git(&t.repo, &["add", "f.txt"]);
        t.git(&t.repo, &["commit", "-q", "-m", "main f"]);
        t.write(&wt, "f.txt", "side\n");
        t.git(&wt, &["add", "f.txt"]);
        t.git(&wt, &["commit", "-q", "-m", "side f"]);
        let out = git::command().arg("-C").arg(&wt).args(["merge", "main"]).output().unwrap();
        assert!(!out.status.success(), "a conflict");
        assert_eq!(doing(&wt), Some(Doing::Merge));
        assert_eq!(doing(&t.repo), None, "the other worktree is not");
        let e = switch(&wt, "main").unwrap_err();
        assert!(!e.to_string().is_empty());
        t.git(&wt, &["merge", "--abort"]);
        assert_eq!(doing(&wt), None);
        assert_eq!(doing(&t.root.join("nowhere")), None);
    }
}
