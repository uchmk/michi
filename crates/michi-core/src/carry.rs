//! Moving the uncommitted changes of one worktree to another: the changes
//! (untracked files too) are put away as a stash, kept under
//! `refs/michi/backup/`, and put on the other worktree, which leaves the first
//! one clean. A clash on the other side is left marked there, with the stash
//! kept; a refusal puts the changes back where they were.

use std::path::Path;

use crate::backup;
use crate::branch;
use crate::git;
use crate::stash::{self, Applied, Saved};

/// How a move ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Carried {
    /// The source had nothing to move.
    Nothing,
    /// Moved; the changes are also kept at `saved` (`refs/michi/backup/…`).
    Done { saved: String },
    /// Moved with clashes in these files, left marked in the target; the
    /// stash stays, and `saved` keeps the changes too.
    Clash { saved: String, files: Vec<String> },
}

/// Moves the uncommitted changes of the worktree at `from` to the one at
/// `to`. Both must be free of a merge or the like. When the target refuses the
/// changes (they would overwrite its own), they go back to `from` and the
/// refusal is the error.
pub fn changes(from: &Path, to: &Path) -> Result<Carried, git::Error> {
    if crate::paths::key(from) == crate::paths::key(to) {
        return Err(git::Error::Git("the changes are already there".into()));
    }
    for wt in [from, to] {
        if let Some(d) = branch::doing(wt) {
            return Err(git::Error::Git(format!("{} is in the middle of a {}", wt.display(), d.word())));
        }
    }
    let Saved::Made(id) = stash::save(from, "michi: changes moved to another worktree")? else { return Ok(Carried::Nothing) };
    let saved = match backup::save(from, &id, "changes moved to another worktree") {
        Ok(saved) => saved,
        Err(e) => {
            let _ = stash::apply(from, &id, true);
            return Err(e);
        }
    };
    match stash::apply(to, &id, true) {
        Ok(Applied::Done) => Ok(Carried::Done { saved }),
        Ok(Applied::Conflict(files)) => Ok(Carried::Clash { saved, files }),
        Err(e) => {
            // Nothing was done on the target: they go back.
            let _ = stash::apply(from, &id, true);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;
    use std::path::PathBuf;

    /// `main` in the repo with a.txt = 1; a second worktree `other` on `side`.
    fn two(name: &str) -> (TempRepo, PathBuf) {
        let t = TempRepo::configured(name);
        t.write(&t.repo, "a.txt", "1\n");
        t.git(&t.repo, &["add", "a.txt"]);
        t.git(&t.repo, &["commit", "-q", "-m", "a"]);
        let other = t.root.join("other");
        t.git(&t.repo, &["worktree", "add", "-q", "-b", "side", other.to_str().unwrap()]);
        (t, other)
    }

    fn text(dir: &Path, file: &str) -> Option<String> {
        std::fs::read_to_string(dir.join(file)).ok()
    }

    #[test]
    fn edits_and_new_files_move_over_and_the_source_is_clean() {
        let (t, other) = two("carry-ok");
        t.write(&t.repo, "a.txt", "edited\n");
        t.write(&t.repo, "new.txt", "n\n");
        let Carried::Done { saved } = changes(&t.repo, &other).unwrap() else { panic!("not done") };
        assert!(saved.starts_with(backup::PREFIX), "{saved}");
        assert_eq!(text(&other, "a.txt").as_deref(), Some("edited\n"));
        assert_eq!(text(&other, "new.txt").as_deref(), Some("n\n"));
        assert_eq!(text(&t.repo, "a.txt").as_deref(), Some("1\n"));
        assert!(text(&t.repo, "new.txt").is_none());
        assert!(t.out(&t.repo, &["status", "--porcelain"]).is_empty());
        assert!(stash::list(&t.repo).unwrap().is_empty(), "the stash is gone, the backup stays");
    }

    #[test]
    fn nothing_to_move_is_said() {
        let (t, other) = two("carry-nothing");
        assert_eq!(changes(&t.repo, &other).unwrap(), Carried::Nothing);
    }

    #[test]
    fn the_same_worktree_is_refused() {
        let (t, _) = two("carry-same");
        t.write(&t.repo, "a.txt", "edited\n");
        assert!(changes(&t.repo, &t.repo).is_err());
        assert_eq!(text(&t.repo, "a.txt").as_deref(), Some("edited\n"));
    }

    #[test]
    fn a_refusal_puts_the_changes_back() {
        let (t, other) = two("carry-refused");
        t.write(&t.repo, "a.txt", "mine\n");
        t.write(&other, "a.txt", "theirs\n");
        let e = changes(&t.repo, &other);
        assert!(e.is_err(), "{e:?}");
        assert_eq!(text(&t.repo, "a.txt").as_deref(), Some("mine\n"));
        assert_eq!(text(&other, "a.txt").as_deref(), Some("theirs\n"));
        assert!(stash::list(&t.repo).unwrap().is_empty());
    }

    #[test]
    fn a_clash_is_left_marked_and_the_stash_kept() {
        let (t, other) = two("carry-clash");
        t.write(&other, "a.txt", "2\n");
        t.git(&other, &["commit", "-q", "-am", "two"]);
        t.write(&t.repo, "a.txt", "mine\n");
        let Carried::Clash { files, .. } = changes(&t.repo, &other).unwrap() else { panic!("no clash") };
        assert_eq!(files, vec!["a.txt".to_owned()]);
        assert!(text(&other, "a.txt").unwrap().contains("<<<<<<<"));
        assert_eq!(stash::list(&t.repo).unwrap().len(), 1);
        assert_eq!(text(&t.repo, "a.txt").as_deref(), Some("1\n"));
    }
}
