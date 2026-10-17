//! One file brought back as an older commit had it (docs/design.md: ファイル
//! 単位の操作). The file in the worktree becomes the commit's version; the
//! index and HEAD stay as they are, so the change shows in the Changes view
//! as an ordinary unstaged one.
//!
//! What the worktree had is kept under `refs/michi/backup/` first, unless it
//! is the same as the commit's version or as the head's (those are in the
//! history already).

use std::path::Path;

use crate::discard;
use crate::git;
use crate::oid::Oid;

/// What bringing a version back did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Restored {
    /// The worktree had that version already.
    Same,
    /// The file is the commit's now. The backup of what it was (none: it
    /// was not there, or the head has it).
    Done(Option<String>),
}

/// The file at `path` of the worktree at `wt` made as the commit `id` has it.
pub fn file(wt: &Path, id: &Oid, path: &str) -> Result<Restored, git::Error> {
    let spec = format!("{id}:{path}");
    let version = git::read_maybe(wt, &["rev-parse", "-q", "--verify", &spec])?
        .map(|out| String::from_utf8_lossy(&out).trim().to_owned())
        .ok_or_else(|| git::Error::Git(format!("{path} is not in {}", id.short())))?;
    let now = if wt.join(path).is_file() { Some(hash(wt, path)?) } else { None };
    if now.as_deref() == Some(version.as_str()) {
        return Ok(Restored::Same);
    }
    // What would be lost: the file as the worktree has it, unless the head
    // has that very version.
    let saved = match now {
        Some(now) => {
            let head = git::read_maybe(wt, &["rev-parse", "-q", "--verify", &format!("HEAD:{path}")])?
                .map(|out| String::from_utf8_lossy(&out).trim().to_owned());
            if head.as_deref() == Some(now.as_str()) {
                None
            } else {
                Some(discard::keep_files(wt, &[path], &format!("replaced {path} with its version of {}", id.short()))?)
            }
        }
        None => None,
    };
    let mut c = git::command();
    c.env("GIT_LITERAL_PATHSPECS", "1").arg("-C").arg(wt).args(["restore", "--source", &id.to_string(), "--worktree", "--", path]);
    discard::run(c)?;
    Ok(Restored::Done(saved))
}

/// The blob id git would give the file at `path` of the worktree.
fn hash(wt: &Path, path: &str) -> Result<String, git::Error> {
    let mut c = git::command();
    c.env("GIT_LITERAL_PATHSPECS", "1").arg("-C").arg(wt).args(["hash-object", "--path", path, "--", path]);
    Ok(String::from_utf8_lossy(&discard::run(c)?).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backup;
    use crate::testrepo::TempRepo;
    use std::fs;

    fn id(t: &TempRepo, rev: &str) -> Oid {
        Oid::parse(&t.out(&t.repo, &["rev-parse", rev])).unwrap()
    }

    fn two_versions(name: &str) -> TempRepo {
        let t = TempRepo::configured(name);
        t.write(&t.repo, "a.txt", "one\n");
        t.write(&t.repo, "keep.txt", "keep\n");
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "first"]);
        t.write(&t.repo, "a.txt", "two\n");
        t.git(&t.repo, &["commit", "-q", "-am", "second"]);
        t
    }

    #[test]
    fn an_old_version_comes_back_unstaged_and_the_head_is_not_kept() {
        let t = two_versions("restore-clean");
        let first = id(&t, "HEAD~1");
        let head = t.out(&t.repo, &["rev-parse", "HEAD"]);
        // The worktree has the head's version: nothing is lost, no backup.
        assert_eq!(file(&t.repo, &first, "a.txt").unwrap(), Restored::Done(None));
        assert_eq!(fs::read_to_string(t.repo.join("a.txt")).unwrap(), "one\n");
        assert_eq!(t.out(&t.repo, &["diff", "--cached", "--name-only"]), "", "the index is as it was");
        assert_eq!(t.out(&t.repo, &["diff", "--name-only"]), "a.txt");
        assert_eq!(t.out(&t.repo, &["rev-parse", "HEAD"]), head);
        assert_eq!(t.out(&t.repo, &["for-each-ref", "refs/michi/backup/"]), "");
        // Again: it is that version already.
        assert_eq!(file(&t.repo, &first, "a.txt").unwrap(), Restored::Same);
    }

    #[test]
    fn what_the_worktree_had_is_kept_first() {
        let t = two_versions("restore-dirty");
        let first = id(&t, "HEAD~1");
        t.write(&t.repo, "a.txt", "mine\n");
        t.write(&t.repo, "keep.txt", "also mine\n");
        let Restored::Done(Some(saved)) = file(&t.repo, &first, "a.txt").unwrap() else { panic!("a backup is made") };
        assert!(saved.starts_with(backup::PREFIX), "{saved}");
        assert_eq!(backup::short(&saved), &saved["refs/".len()..]);
        assert_eq!(t.out(&t.repo, &["show", &format!("{saved}:a.txt")]), "mine");
        assert_eq!(
            t.out(&t.repo, &["reflog", "-1", "--format=%gs", &saved]),
            format!("michi: replaced a.txt with its version of {}", first.short())
        );
        assert_eq!(fs::read_to_string(t.repo.join("a.txt")).unwrap(), "one\n");
        assert_eq!(fs::read_to_string(t.repo.join("keep.txt")).unwrap(), "also mine\n", "another file is left");
    }

    #[test]
    fn a_staged_file_keeps_its_index_and_a_deleted_one_comes_back() {
        let t = two_versions("restore-staged");
        let first = id(&t, "HEAD~1");
        t.write(&t.repo, "a.txt", "staged\n");
        t.git(&t.repo, &["add", "a.txt"]);
        file(&t.repo, &first, "a.txt").unwrap();
        assert_eq!(t.out(&t.repo, &["show", ":a.txt"]), "staged", "the index is left");
        fs::remove_file(t.repo.join("keep.txt")).unwrap();
        assert_eq!(file(&t.repo, &id(&t, "HEAD"), "keep.txt").unwrap(), Restored::Done(None));
        assert_eq!(fs::read_to_string(t.repo.join("keep.txt")).unwrap(), "keep\n");
    }

    #[test]
    fn a_file_the_commit_does_not_have_is_refused() {
        let t = two_versions("restore-missing");
        t.write(&t.repo, "later.txt", "x\n");
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "third"]);
        let err = file(&t.repo, &id(&t, "HEAD~1"), "later.txt").unwrap_err().to_string();
        assert!(err.contains("later.txt is not in"), "{err}");
        assert_eq!(fs::read_to_string(t.repo.join("later.txt")).unwrap(), "x\n");
    }

    #[test]
    fn a_name_with_glob_characters_is_taken_as_it_is() {
        let t = TempRepo::configured("restore-literal");
        t.write(&t.repo, "a[1].txt", "one\n");
        t.write(&t.repo, "a1.txt", "other\n");
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "first"]);
        let first = id(&t, "HEAD");
        t.write(&t.repo, "a[1].txt", "changed\n");
        t.write(&t.repo, "a1.txt", "changed\n");
        file(&t.repo, &first, "a[1].txt").unwrap();
        assert_eq!(fs::read_to_string(t.repo.join("a[1].txt")).unwrap(), "one\n");
        assert_eq!(fs::read_to_string(t.repo.join("a1.txt")).unwrap(), "changed\n");
    }
}
