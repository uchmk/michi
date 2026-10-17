//! Sparse checkout (#31): whether a worktree has it on, and how many tracked
//! files it leaves out of the folder, so a file that is "missing" is not a
//! mystery. Read-only: michi has no switch for it (docs/v1-scope.md); `git
//! sparse-checkout` does the changing.

use std::path::Path;

use crate::git;

/// What is known of a sparse checkout that is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Info {
    /// Cone mode (folders) rather than patterns.
    pub cone: bool,
    /// Tracked files git leaves out of the folder.
    pub left_out: usize,
}

/// Reads the sparse checkout of the worktree at `wt`; `None` when it is off.
pub fn read(wt: &Path) -> Result<Option<Info>, git::Error> {
    let on = git::read_maybe(wt, &["config", "--type=bool", "--get", "core.sparseCheckout"])?;
    if !on.is_some_and(|v| String::from_utf8_lossy(&v).trim() == "true") {
        return Ok(None);
    }
    let cone = git::read_maybe(wt, &["config", "--type=bool", "--get", "core.sparseCheckoutCone"])?
        .is_some_and(|v| String::from_utf8_lossy(&v).trim() == "true");
    let files = git::read_text(wt, &["ls-files", "-v"])?;
    Ok(Some(Info { cone, left_out: count_left_out(&files) }))
}

/// The `ls-files -v` lines tagged `S` (skip-worktree) or `s` (skip-worktree
/// and assumed unchanged).
pub fn count_left_out(ls_files_v: &str) -> usize {
    ls_files_v.lines().filter(|l| l.starts_with(['S', 's'])).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn counts_only_the_skipped_lines() {
        assert_eq!(count_left_out("H a.txt\nS b/c.txt\ns d.txt\nh e.txt\n"), 2);
        assert_eq!(count_left_out(""), 0);
    }

    #[test]
    fn off_by_default() {
        let t = TempRepo::new("sparse-off");
        assert_eq!(read(&t.repo).unwrap(), None);
    }

    #[test]
    fn on_counts_the_files_left_out() {
        let t = TempRepo::new("sparse-on");
        std::fs::create_dir_all(t.repo.join("keep")).unwrap();
        std::fs::create_dir_all(t.repo.join("gone")).unwrap();
        std::fs::write(t.repo.join("keep/a.txt"), "a").unwrap();
        std::fs::write(t.repo.join("gone/b.txt"), "b").unwrap();
        std::fs::write(t.repo.join("gone/c.txt"), "c").unwrap();
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "files"]);
        t.git(&t.repo, &["sparse-checkout", "set", "--cone", "keep"]);
        assert_eq!(read(&t.repo).unwrap(), Some(Info { cone: true, left_out: 2 }));
        t.git(&t.repo, &["sparse-checkout", "disable"]);
        assert_eq!(read(&t.repo).unwrap(), None);
    }
}
