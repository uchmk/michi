//! Moving a worktree's branch to another commit: `git reset` in its three
//! kinds. What each one loses:
//!
//! - soft: nothing in the files and the index; only the branch moves (the
//!   commits after the target stay staged).
//! - mixed: the index goes back to the target (what was staged becomes
//!   unstaged); the files are kept.
//! - hard: the index and the tracked files go back to the target, so every
//!   uncommitted change to a tracked file is gone (untracked files stay).
//!
//! Before it moves, the tip it leaves is kept under `refs/michi/backup/`,
//! and for a hard reset so are the uncommitted tracked changes
//! (`git stash create`).

use std::path::Path;

use crate::backup;
use crate::conflict;
use crate::git;
use crate::oid::Oid;

/// How far the reset goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Soft,
    Mixed,
    Hard,
}

impl Mode {
    /// The three, the gentlest first (the order of the buttons).
    pub const ALL: [Mode; 3] = [Mode::Soft, Mode::Mixed, Mode::Hard];

    /// The flag's word, the same in every language.
    pub fn word(self) -> &'static str {
        match self {
            Mode::Soft => "soft",
            Mode::Mixed => "mixed",
            Mode::Hard => "hard",
        }
    }
}

/// What a reset kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reset {
    /// Where the old tip is kept (none: it was the target already).
    pub tip: Option<String>,
    /// Where the uncommitted tracked changes are kept (a hard reset that
    /// had some).
    pub changes: Option<String>,
}

/// Moves the branch (or the detached head) of the worktree at `wt` to
/// `target`, the way `mode` says. A worktree in the middle of something (a
/// merge, a rebase…) is refused.
pub fn reset(wt: &Path, target: &Oid, mode: Mode) -> Result<Reset, git::Error> {
    conflict::idle(wt)?;
    let tip = git::read_maybe(wt, &["rev-parse", "--verify", "-q", "HEAD^{commit}"])?
        .and_then(|out| Oid::parse(String::from_utf8_lossy(&out).trim()))
        .ok_or_else(|| git::Error::Git("no commit to reset from".to_owned()))?;
    let target = target.to_string();
    let why = format!("before reset --{} to {}", mode.word(), &target[..target.len().min(7)]);
    let changes = if mode == Mode::Hard {
        let out = git::read_text(wt, &["stash", "create", &format!("michi: {why}")])?;
        match Oid::parse(out.trim()) {
            Some(id) => Some(backup::save(wt, &id, &format!("uncommitted changes {why}"))?),
            None => None,
        }
    } else {
        None
    };
    let moved = tip.to_string() != target;
    let kept = if moved { Some(backup::save(wt, &tip, &why)?) } else { None };
    git::read(wt, &["reset", "-q", &format!("--{}", mode.word()), &target])?;
    Ok(Reset { tip: kept, changes })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn commit(t: &TempRepo, file: &str, text: &str) -> Oid {
        t.write(&t.repo, file, text);
        t.git(&t.repo, &["add", file]);
        t.git(&t.repo, &["commit", "-q", "-m", &format!("{file}: {}", text.trim())]);
        head(t)
    }

    fn head(t: &TempRepo) -> Oid {
        Oid::parse(&t.out(&t.repo, &["rev-parse", "HEAD"])).unwrap()
    }

    fn repo(name: &str) -> (TempRepo, Oid, Oid) {
        let t = TempRepo::configured(name);
        let one = commit(&t, "a.txt", "1\n");
        let two = commit(&t, "a.txt", "2\n");
        (t, one, two)
    }

    fn status(t: &TempRepo) -> String {
        t.out(&t.repo, &["status", "--porcelain"])
    }

    #[test]
    fn soft_moves_only_the_branch() {
        let (t, one, two) = repo("reset-soft");
        let r = reset(&t.repo, &one, Mode::Soft).unwrap();
        assert_eq!(head(&t), one);
        assert_eq!(status(&t), "M  a.txt", "still staged");
        assert_eq!(std::fs::read_to_string(t.repo.join("a.txt")).unwrap(), "2\n");
        let tip = r.tip.unwrap();
        assert_eq!(t.out(&t.repo, &["rev-parse", &tip]), two.to_string());
        assert_eq!(r.changes, None);
    }

    #[test]
    fn mixed_unstages_and_keeps_the_files() {
        let (t, one, _) = repo("reset-mixed");
        reset(&t.repo, &one, Mode::Mixed).unwrap();
        assert_eq!(head(&t), one);
        assert_eq!(status(&t), " M a.txt", "unstaged");
        assert_eq!(std::fs::read_to_string(t.repo.join("a.txt")).unwrap(), "2\n");
    }

    #[test]
    fn hard_loses_changes_but_keeps_them_in_a_backup() {
        let (t, one, two) = repo("reset-hard");
        t.write(&t.repo, "a.txt", "dirty\n");
        t.write(&t.repo, "new.txt", "untracked\n");
        let r = reset(&t.repo, &one, Mode::Hard).unwrap();
        assert_eq!(head(&t), one);
        assert_eq!(std::fs::read_to_string(t.repo.join("a.txt")).unwrap(), "1\n");
        assert_eq!(status(&t), "?? new.txt", "untracked stays");
        assert_eq!(t.out(&t.repo, &["rev-parse", &r.tip.unwrap()]), two.to_string());
        let changes = r.changes.unwrap();
        assert_eq!(t.out(&t.repo, &["show", &format!("{changes}:a.txt")]), "dirty");
    }

    #[test]
    fn hard_without_changes_or_move_keeps_nothing() {
        let (t, _, two) = repo("reset-same");
        let r = reset(&t.repo, &two, Mode::Hard).unwrap();
        assert_eq!(r, Reset { tip: None, changes: None });
    }

    #[test]
    fn a_worktree_in_the_middle_of_something_is_refused() {
        let (t, one, two) = repo("reset-busy");
        t.git(&t.repo, &["checkout", "-q", "-b", "side", &one.to_string()]);
        commit(&t, "a.txt", "side\n");
        t.git(&t.repo, &["checkout", "-q", "-"]);
        let _ = std::process::Command::new("git").arg("-C").arg(&t.repo).args(["merge", "side"]).output();
        assert!(reset(&t.repo, &two, Mode::Hard).is_err());
    }
}
