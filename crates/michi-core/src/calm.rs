//! The "calm down" save (#113): when the person no longer knows what they
//! did, everything of a worktree is kept at once under `refs/michi/backup/`,
//! and nothing in the worktree changes. The kept commit has the working
//! folder as its tree (untracked files too, up to [`predict::Limits`]), the
//! `HEAD` as its first parent and, when the index differs from `HEAD`, a
//! commit of the index as its second. It is also safe in the middle of a
//! merge or a rebase: nothing is touched, so the state can be left as it is.

use std::path::Path;

use crate::backup;
use crate::git;
use crate::oid::Oid;
use crate::predict::{self, LeftOut, Limits};

/// What was kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Calmed {
    /// The backup ref, `refs/michi/backup/<time>`.
    pub backup: String,
    /// The worktree's `HEAD` when it was kept.
    pub head: Oid,
    /// Whether anything but `HEAD` was in it (the worktree had changes).
    pub changes: bool,
    /// Whether the index held something of its own (staged changes), kept as
    /// the second parent.
    pub staged: bool,
    /// Untracked files too big to keep, which stay where they are.
    pub left_out: Vec<LeftOut>,
}

/// Keeps the whole state of the worktree at `wt`: `HEAD`, the index and the
/// working folder. The calls block; run them off the UI thread.
pub fn save(wt: &Path) -> Result<Calmed, git::Error> {
    let snap = predict::snapshot(wt, Limits::DEFAULT)?;
    let head = match &snap {
        Some(s) => s.head,
        None => Oid::parse(git::read_text(wt, &["rev-parse", "HEAD^{commit}"])?.trim())
            .ok_or_else(|| git::Error::Io("git rev-parse: unexpected output".into()))?,
    };
    let (work, left_out) = match snap {
        Some(s) => (s.commit, s.left_out),
        None => (None, Vec::new()),
    };
    let Some(work) = work else {
        let backup = backup::save(wt, &head, "calm: nothing but HEAD to keep")?;
        return Ok(Calmed { backup, head, changes: false, staged: false, left_out });
    };
    let hex = head.to_string();
    let tree = git::read_text(wt, &["rev-parse", &format!("{work}^{{tree}}")])?.trim().to_owned();
    // The index as a commit on HEAD: `write-tree` refuses an index with
    // unmerged entries, and then only the working folder is kept.
    let head_tree = git::read_text(wt, &["rev-parse", "HEAD^{tree}"])?.trim().to_owned();
    let index = match git::read_text(wt, &["write-tree"]) {
        Ok(t) if t.trim() != head_tree => {
            Some(git::read_text(wt, &["commit-tree", t.trim(), "-p", &hex, "-m", "michi: calm (index)"])?.trim().to_owned())
        }
        _ => None,
    };
    let mut args = vec!["commit-tree", &tree, "-p", &hex];
    if let Some(i) = &index {
        args.extend(["-p", i]);
    }
    args.extend(["-m", "michi: calm (working folder)"]);
    let kept = Oid::parse(git::read_text(wt, &args)?.trim()).ok_or_else(|| git::Error::Io("git commit-tree: no commit".into()))?;
    let backup = backup::save(wt, &kept, "calm: HEAD, index and working folder")?;
    Ok(Calmed { backup, head, changes: true, staged: index.is_some(), left_out })
}

/// The git command that puts the files back as they were kept (it leaves
/// `HEAD` and the index alone), for the message after the save.
pub fn way_back(c: &Calmed) -> String {
    format!("git restore --source={} --worktree -- .", backup::short(&c.backup))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn rev(t: &TempRepo, spec: &str) -> String {
        t.out(&t.repo, &["rev-parse", spec])
    }

    #[test]
    fn a_clean_worktree_keeps_head_alone() {
        let t = TempRepo::configured("calm-clean");
        let c = save(&t.repo).unwrap();
        assert!(!c.changes && !c.staged && c.left_out.is_empty());
        assert!(c.backup.starts_with(backup::PREFIX));
        assert_eq!(rev(&t, &c.backup), rev(&t, "HEAD"));
        assert_eq!(c.head.to_string(), rev(&t, "HEAD"));
    }

    #[test]
    fn changes_staged_and_untracked_are_kept_and_nothing_moves() {
        let t = TempRepo::configured("calm-dirty");
        t.write(&t.repo, "a.txt", "one\n");
        t.git(&t.repo, &["add", "a.txt"]);
        t.git(&t.repo, &["commit", "-q", "-m", "a"]);
        let head = rev(&t, "HEAD");
        t.write(&t.repo, "a.txt", "two\n");
        t.git(&t.repo, &["add", "a.txt"]);
        t.write(&t.repo, "a.txt", "three\n");
        t.write(&t.repo, "new.txt", "fresh\n");
        let status = t.out(&t.repo, &["status", "--porcelain"]);

        let c = save(&t.repo).unwrap();
        assert!(c.changes && c.staged);
        // The worktree, the index and HEAD are as they were.
        assert_eq!(t.out(&t.repo, &["status", "--porcelain"]), status);
        assert_eq!(rev(&t, "HEAD"), head);
        // The kept commit: working folder as tree, HEAD and the index as parents.
        assert_eq!(t.out(&t.repo, &["show", &format!("{}:a.txt", c.backup)]), "three");
        assert_eq!(t.out(&t.repo, &["show", &format!("{}:new.txt", c.backup)]), "fresh");
        assert_eq!(rev(&t, &format!("{}^1", c.backup)), head);
        assert_eq!(t.out(&t.repo, &["show", &format!("{}^2:a.txt", c.backup)]), "two");
        let n = t.out(&t.repo, &["for-each-ref", "--format=%(refname)", backup::PREFIX]);
        assert_eq!(n, c.backup);
    }

    #[test]
    fn unstaged_changes_alone_make_a_single_parent() {
        let t = TempRepo::configured("calm-unstaged");
        t.write(&t.repo, "b.txt", "x\n");
        let c = save(&t.repo).unwrap();
        assert!(c.changes && !c.staged);
        assert_eq!(t.out(&t.repo, &["rev-list", "--parents", "-n1", &c.backup]).split_whitespace().count(), 2);
    }

    #[test]
    fn it_works_in_the_middle_of_a_merge_with_a_conflict() {
        let t = TempRepo::configured("calm-merge");
        t.write(&t.repo, "f.txt", "base\n");
        t.git(&t.repo, &["add", "f.txt"]);
        t.git(&t.repo, &["commit", "-q", "-m", "base"]);
        t.git(&t.repo, &["checkout", "-q", "-b", "side"]);
        t.write(&t.repo, "f.txt", "side\n");
        t.git(&t.repo, &["commit", "-q", "-am", "side"]);
        t.git(&t.repo, &["checkout", "-q", "main"]);
        t.write(&t.repo, "f.txt", "main\n");
        t.git(&t.repo, &["commit", "-q", "-am", "main"]);
        let merge = git::command().arg("-C").arg(&t.repo).args(["merge", "side"]).output().unwrap();
        assert!(!merge.status.success());
        let c = save(&t.repo).unwrap();
        assert!(c.changes);
        assert!(t.out(&t.repo, &["show", &format!("{}:f.txt", c.backup)]).contains("<<<<<<<"));
        // Still in the merge.
        assert!(crate::branch::doing(&t.repo).is_some());
        assert!(!t.out(&t.repo, &["diff", "--name-only", "--diff-filter=U"]).is_empty());
    }

    #[test]
    fn the_way_back_names_the_ref() {
        let c = Calmed {
            backup: "refs/michi/backup/20261016-142233".into(),
            head: Oid::parse(&"a".repeat(40)).unwrap(),
            changes: true,
            staged: false,
            left_out: Vec::new(),
        };
        assert_eq!(way_back(&c), "git restore --source=michi/backup/20261016-142233 --worktree -- .");
    }
}
