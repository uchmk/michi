//! Tags: make one on a commit (light, or with a message), delete one. What a
//! deleted tag pointed at is kept under `refs/michi/backup/` first.

use std::path::Path;

use crate::backup;
use crate::branch;
use crate::git;
use crate::oid::Oid;

/// Whether `name` would do as a new tag's name; why not when not.
pub fn check_name(dir: &Path, name: &str) -> Result<(), git::Error> {
    if name.is_empty() || name.starts_with('-') {
        return Err(git::Error::Git(format!("'{name}' is not a valid tag name")));
    }
    match git::check(dir, &["check-ref-format", &format!("refs/tags/{name}")])? {
        true => Ok(()),
        false => Err(git::Error::Git(format!("'{name}' is not a valid tag name"))),
    }
}

/// Makes the tag `name` on `at`: a light one when `message` is empty, one
/// with the message (and its author and date) when not. A name that is
/// taken is refused.
pub fn create(dir: &Path, name: &str, at: &Oid, message: &str) -> Result<(), git::Error> {
    check_name(dir, name)?;
    let at = at.to_string();
    if message.trim().is_empty() {
        return branch::write(dir, &["tag", name, &at]);
    }
    let mut c = git::command();
    c.arg("-C").arg(dir).args(["tag", "-a", "-F", "-", name, &at]);
    git::feed(c, message.as_bytes()).map(|_| ())
}

/// Deletes the tag `name`, keeping what it pointed at (the tag itself, for
/// one with a message) under `refs/michi/backup/` first: the backup's name.
pub fn delete(dir: &Path, name: &str) -> Result<String, git::Error> {
    let full = format!("refs/tags/{name}");
    let id = git::read_maybe(dir, &["rev-parse", "--verify", "-q", &full])?
        .and_then(|out| Oid::parse(String::from_utf8_lossy(&out).trim()))
        .ok_or_else(|| git::Error::Git(format!("no tag '{name}'")))?;
    let saved = backup::save(dir, &id, &format!("deleted tag {name}"))?;
    branch::write(dir, &["tag", "-d", name])?;
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn head(t: &TempRepo) -> Oid {
        Oid::parse(&t.out(&t.repo, &["rev-parse", "HEAD"])).unwrap()
    }

    #[test]
    fn a_tag_is_light_or_carries_its_message() {
        let t = TempRepo::configured("tag-make");
        let first = head(&t);
        t.git(&t.repo, &["commit", "-q", "--allow-empty", "-m", "second"]);
        create(&t.repo, "v1", &first, "").unwrap();
        assert_eq!(t.out(&t.repo, &["cat-file", "-t", "refs/tags/v1"]), "commit");
        assert_eq!(t.out(&t.repo, &["rev-parse", "v1"]), first.to_string());
        create(&t.repo, "v2", &head(&t), "Second release\n\nWith notes.").unwrap();
        assert_eq!(t.out(&t.repo, &["cat-file", "-t", "refs/tags/v2"]), "tag");
        assert_eq!(t.out(&t.repo, &["tag", "-l", "--format=%(contents:subject)", "v2"]), "Second release");
        let e = create(&t.repo, "v1", &first, "").unwrap_err();
        assert!(e.to_string().contains("already exists"), "{e}");
        for bad in ["", "-x", "a..b", "a b", "x.lock", "a~1"] {
            assert!(create(&t.repo, bad, &first, "").is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_deleted_tag_is_kept() {
        let t = TempRepo::configured("tag-delete");
        create(&t.repo, "v1", &head(&t), "Release").unwrap();
        let object = t.out(&t.repo, &["rev-parse", "refs/tags/v1"]);
        let saved = delete(&t.repo, "v1").unwrap();
        assert!(saved.starts_with(backup::PREFIX), "{saved}");
        assert_eq!(t.out(&t.repo, &["rev-parse", &saved]), object, "the tag itself, message and all");
        // `git reflog` walks commits only: the log itself.
        let log = std::fs::read_to_string(t.repo.join(".git/logs").join(&saved)).unwrap();
        assert!(log.trim_end().ends_with("\tmichi: deleted tag v1"), "{log}");
        assert_eq!(t.out(&t.repo, &["tag", "-l"]), "");
        assert_eq!(delete(&t.repo, "v1").unwrap_err().to_string(), "no tag 'v1'");
    }
}
