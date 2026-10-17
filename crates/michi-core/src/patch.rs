//! Patches in and out: a commit (or a worktree's uncommitted changes) written
//! as a patch for the clipboard, and a patch from the clipboard applied to a
//! worktree. Applying is all or nothing: `git apply` checks the whole patch
//! first and touches no file when any part of it does not fit.

use std::path::Path;

use crate::conflict;
use crate::git;
use crate::oid::Oid;

/// What a patch changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub files: usize,
    pub added: usize,
    pub removed: usize,
}

/// Whether `text` reads like a patch (a git diff, a plain unified diff or an
/// emailed `format-patch`), so a paste of anything else is let be.
pub fn looks_like_patch(text: &str) -> bool {
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        if line.starts_with("diff --git ") {
            return true;
        }
        if line.starts_with("--- ") && lines.peek().is_some_and(|n| n.starts_with("+++ ")) {
            return true;
        }
    }
    false
}

/// Reads the lines of `git apply --numstat`.
pub fn parse_numstat(text: &str) -> Summary {
    let mut s = Summary::default();
    for line in text.lines() {
        let mut cols = line.splitn(3, '\t');
        let (Some(a), Some(r), Some(_)) = (cols.next(), cols.next(), cols.next()) else { continue };
        s.files += 1;
        // A binary file has `-` for both.
        s.added += a.parse::<usize>().unwrap_or(0);
        s.removed += r.parse::<usize>().unwrap_or(0);
    }
    s
}

/// What `patch` would change, when it fits the worktree at `wt` as it is
/// now. Writes nothing.
pub fn check(wt: &Path, patch: &str) -> Result<Summary, git::Error> {
    let mut c = git::command();
    c.arg("-C").arg(wt).args(["apply", "--check", "--whitespace=nowarn", "-"]);
    git::feed(c, patch.as_bytes())?;
    let mut c = git::command();
    c.arg("-C").arg(wt).args(["apply", "--numstat", "-"]);
    let out = git::feed(c, patch.as_bytes())?;
    Ok(parse_numstat(&String::from_utf8_lossy(&out)))
}

/// Applies `patch` to the files of the worktree at `wt`, leaving the index
/// and HEAD alone (the changes show up as uncommitted ones). A worktree in
/// the middle of a merge, rebase or the like is refused, and so is a patch
/// that does not fit: then nothing is changed.
pub fn apply(wt: &Path, patch: &str) -> Result<Summary, git::Error> {
    conflict::idle(wt)?;
    let summary = check(wt, patch)?;
    let mut c = git::command();
    c.arg("-C").arg(wt).args(["apply", "--whitespace=nowarn", "-"]);
    git::feed(c, patch.as_bytes())?;
    Ok(summary)
}

/// The commit `id` as an emailed patch (what `git format-patch` writes), or,
/// with none, the tracked changes of the worktree at `wt` against HEAD.
/// Files not tracked yet are not in it. A merge has no patch.
pub fn export(wt: &Path, id: Option<&Oid>) -> Result<String, git::Error> {
    let (out, empty) = match id {
        Some(id) => {
            let id = id.to_string();
            // `format-patch` skips a merge and writes the commit before it.
            if git::read_text(wt, &["rev-list", "--parents", "-n", "1", &id])?.split_whitespace().count() > 2 {
                return Err(git::Error::Git("a merge commit has no patch of its own".to_owned()));
            }
            (git::read(wt, &["format-patch", "--stdout", "--binary", "--root", "-1", &id])?, "a merge commit has no patch of its own")
        }
        None => (git::read(wt, &["diff", "--binary", "HEAD"])?, "no tracked changes to make a patch of"),
    };
    let text = String::from_utf8_lossy(&out).into_owned();
    if text.trim().is_empty() {
        return Err(git::Error::Git(empty.to_owned()));
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn commit(t: &TempRepo, dir: &Path, file: &str, text: &str) -> Oid {
        t.write(dir, file, text);
        t.git(dir, &["add", file]);
        t.git(dir, &["commit", "-q", "-m", &format!("{file}: {}", text.trim())]);
        Oid::parse(&t.out(dir, &["rev-parse", "HEAD"])).unwrap()
    }

    #[test]
    fn a_patch_is_told_from_other_text() {
        assert!(looks_like_patch("diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\n"));
        assert!(looks_like_patch("From abc Mon Sep 17 00:00:00 2001\nSubject: x\n\n---\n a | 1 +\n\ndiff --git a/a b/a\n"));
        assert!(looks_like_patch("--- a.txt\n+++ b.txt\n@@ -1 +1 @@\n"));
        assert!(!looks_like_patch("feat/x\n"));
        assert!(!looks_like_patch("--- not a patch\nat all\n"));
        assert!(!looks_like_patch(""));
    }

    #[test]
    fn numstat_is_summed() {
        let s = parse_numstat("3\t1\ta.txt\n-\t-\timg.png\n0\t2\tb.txt\nnoise\n");
        assert_eq!(s, Summary { files: 3, added: 3, removed: 3 });
        assert_eq!(parse_numstat(""), Summary::default());
    }

    #[test]
    fn a_commit_goes_out_and_comes_back_in() {
        let t = TempRepo::configured("patch-roundtrip");
        commit(&t, &t.repo, "a.txt", "1\n");
        let two = commit(&t, &t.repo, "a.txt", "1\n2\n");
        let patch = export(&t.repo, Some(&two)).unwrap();
        assert!(patch.starts_with("From "), "{patch}");
        assert!(looks_like_patch(&patch));
        // Back on the older commit, in a worktree of its own.
        let other = t.root.join("other");
        t.git(&t.repo, &["worktree", "add", "-q", "--detach", other.to_str().unwrap(), "HEAD~1"]);
        let s = apply(&other, &patch).unwrap();
        assert_eq!(s, Summary { files: 1, added: 1, removed: 0 });
        assert_eq!(std::fs::read_to_string(other.join("a.txt")).unwrap(), "1\n2\n");
        // Only the files changed: nothing staged, no new commit.
        assert_eq!(t.out(&other, &["diff", "--cached", "--name-only"]), "");
        assert_eq!(t.out(&other, &["status", "--porcelain"]), " M a.txt");
        // The same patch again does not fit, and changes nothing.
        assert!(apply(&other, &patch).is_err());
        assert_eq!(std::fs::read_to_string(other.join("a.txt")).unwrap(), "1\n2\n");
    }

    #[test]
    fn a_patch_that_does_not_fit_touches_nothing() {
        let t = TempRepo::configured("patch-misfit");
        commit(&t, &t.repo, "a.txt", "1\n");
        commit(&t, &t.repo, "b.txt", "b\n");
        let patch = "diff --git a/b.txt b/b.txt\n--- a/b.txt\n+++ b/b.txt\n@@ -1 +1 @@\n-b\n+B\n\
                     diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -1 +1 @@\n-zzz\n+Z\n";
        assert!(apply(&t.repo, patch).is_err());
        assert_eq!(std::fs::read_to_string(t.repo.join("b.txt")).unwrap(), "b\n");
        assert!(check(&t.repo, "not a patch at all\n").is_err());
    }

    #[test]
    fn uncommitted_changes_go_out_as_a_diff() {
        let t = TempRepo::configured("patch-wip");
        commit(&t, &t.repo, "a.txt", "1\n");
        assert!(export(&t.repo, None).is_err(), "nothing to write");
        t.write(&t.repo, "a.txt", "1\n2\n");
        let patch = export(&t.repo, None).unwrap();
        assert!(patch.starts_with("diff --git "));
        assert!(check(&t.repo, &patch).is_err(), "already in the file");
        t.git(&t.repo, &["checkout", "-q", "--", "a.txt"]);
        assert_eq!(apply(&t.repo, &patch).unwrap().added, 1);
    }

    #[test]
    fn a_merge_has_no_patch_and_a_root_has_one() {
        let t = TempRepo::configured("patch-merge");
        let first = commit(&t, &t.repo, "a.txt", "1\n");
        assert!(export(&t.repo, Some(&first)).is_ok());
        t.git(&t.repo, &["switch", "-q", "-c", "side"]);
        commit(&t, &t.repo, "b.txt", "b\n");
        t.git(&t.repo, &["switch", "-q", "main"]);
        commit(&t, &t.repo, "c.txt", "c\n");
        t.git(&t.repo, &["merge", "-q", "--no-ff", "-m", "merge", "side"]);
        let m = Oid::parse(&t.out(&t.repo, &["rev-parse", "HEAD"])).unwrap();
        assert!(export(&t.repo, Some(&m)).is_err());
    }
}
