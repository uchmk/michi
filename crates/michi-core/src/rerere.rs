//! git's rerere (reuse recorded resolution) for the repository: whether it
//! is on, and turning it on or off in the repository's own configuration
//! (so a `git merge` typed in a terminal reuses a resolution too).

use std::path::Path;

use crate::{branch, git};

/// `rerere.enabled` as the repository (or the user's configuration) has it;
/// `None` when nobody said. A `rr-cache` folder also turns git's rerere on
/// when nothing is said, which counts as said.
pub fn enabled(dir: &Path) -> Result<Option<bool>, git::Error> {
    if let Some(v) = git::read_maybe(dir, &["config", "--type=bool", "--get", "rerere.enabled"])? {
        return Ok(Some(String::from_utf8_lossy(&v).trim() == "true"));
    }
    let cache = git::read_text(dir, &["rev-parse", "--git-path", "rr-cache"])?;
    let cache = if Path::new(&cache).is_absolute() { Path::new(&cache).to_path_buf() } else { dir.join(cache) };
    Ok(cache.is_dir().then_some(true))
}

/// Writes `rerere.enabled` into the repository's configuration. Turned on in
/// the middle of a merge, it records the clashes already there, so what is
/// solved now is remembered (git only records at the clash otherwise).
pub fn set(dir: &Path, on: bool) -> Result<(), git::Error> {
    branch::write(dir, &["config", "rerere.enabled", if on { "true" } else { "false" }])?;
    if on {
        // Best effort: it is not an error when nothing is stopped.
        let _ = branch::write(dir, &["rerere"]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn nobody_said_then_written_both_ways() {
        let t = TempRepo::new("rerere-set");
        assert_eq!(enabled(&t.repo).unwrap(), None);
        set(&t.repo, false).unwrap();
        assert_eq!(enabled(&t.repo).unwrap(), Some(false));
        set(&t.repo, true).unwrap();
        assert_eq!(enabled(&t.repo).unwrap(), Some(true));
    }

    #[test]
    fn a_rr_cache_folder_counts_as_said() {
        let t = TempRepo::new("rerere-cache");
        std::fs::create_dir_all(t.repo.join(".git").join("rr-cache")).unwrap();
        assert_eq!(enabled(&t.repo).unwrap(), Some(true));
    }
}
