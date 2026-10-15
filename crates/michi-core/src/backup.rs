//! What michi keeps before it drops something: a ref under
//! `refs/michi/backup/`, named after the local time (`20261016-142233`,
//! `-2`… after it when that is taken), with a reflog line saying what it was
//! kept from. gc keeps the commits it holds; `git log` and
//! `git for-each-ref refs/michi/backup/` find it.

use std::path::Path;

use crate::git;
use crate::oid::Oid;

/// Where the backups go.
pub const PREFIX: &str = "refs/michi/backup/";

/// Keeps `id` under a new backup ref, with `why` (`pushed over
/// origin/main`) in its reflog. The ref's full name.
pub fn save(dir: &Path, id: &Oid, why: &str) -> Result<String, git::Error> {
    save_as(dir, id, why, &chrono::Local::now().format("%Y%m%d-%H%M%S").to_string())
}

fn save_as(dir: &Path, id: &Oid, why: &str, stamp: &str) -> Result<String, git::Error> {
    let message = format!("michi: {why}");
    for n in 1..100 {
        let name = if n == 1 { format!("{PREFIX}{stamp}") } else { format!("{PREFIX}{stamp}-{n}") };
        if git::read_maybe(dir, &["rev-parse", "--verify", "-q", &name])?.is_some() {
            continue;
        }
        // The empty old value: only while the ref is not there.
        git::read(dir, &["update-ref", "--create-reflog", "-m", &message, &name, &id.to_string(), ""])?;
        return Ok(name);
    }
    Err(git::Error::Io(format!("{PREFIX}{stamp}: too many backups in one second")))
}

/// The name to show: `michi/backup/20261016-142233`.
pub fn short(name: &str) -> &str {
    name.strip_prefix("refs/").unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn a_backup_is_a_new_ref_with_its_reason() {
        let t = TempRepo::configured("backup");
        let head = Oid::parse(&t.out(&t.repo, &["rev-parse", "HEAD"])).unwrap();
        let a = save_as(&t.repo, &head, "pushed over origin/main", "20261016-142233").unwrap();
        assert_eq!(a, "refs/michi/backup/20261016-142233");
        assert_eq!(short(&a), "michi/backup/20261016-142233");
        let b = save_as(&t.repo, &head, "again", "20261016-142233").unwrap();
        assert_eq!(b, "refs/michi/backup/20261016-142233-2");
        assert_eq!(t.out(&t.repo, &["rev-parse", &a]), head.to_string());
        assert_eq!(t.out(&t.repo, &["reflog", "-1", "--format=%gs", &a]), "michi: pushed over origin/main");
        let now = save(&t.repo, &head, "now").unwrap();
        assert!(now.starts_with(PREFIX) && now.len() >= PREFIX.len() + 15, "{now}");
    }
}
