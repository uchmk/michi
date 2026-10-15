//! A throwaway repository for the tests: `<temp>/michi-test-<pid>-<name>/repo`
//! with one empty commit on `main`, worktrees made beside it. git runs with no
//! user or system configuration and fixed names and dates, so the tests do
//! not depend on the machine.

use std::fs;
use std::path::{Path, PathBuf};

use crate::git;

pub struct TempRepo {
    /// The folder holding `repo` and the worktrees.
    pub root: PathBuf,
    /// The main worktree.
    pub repo: PathBuf,
    config: PathBuf,
}

impl TempRepo {
    pub fn new(name: &str) -> TempRepo {
        let root = std::env::temp_dir().join(format!("michi-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let repo = root.join("repo");
        fs::create_dir_all(&repo).unwrap();
        let config = root.join("gitconfig");
        fs::write(&config, "").unwrap();
        let r = TempRepo { root, repo, config };
        r.git(&r.repo, &["init", "-q", "-b", "main"]);
        r.git(&r.repo, &["commit", "-q", "--allow-empty", "-m", "first"]);
        r
    }

    /// Runs git in `dir` and panics when it fails.
    pub fn git(&self, dir: &Path, args: &[&str]) {
        let out = git::command()
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_CONFIG_GLOBAL", &self.config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "michi")
            .env("GIT_AUTHOR_EMAIL", "michi@example.invalid")
            .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z")
            .env("GIT_COMMITTER_NAME", "michi")
            .env("GIT_COMMITTER_EMAIL", "michi@example.invalid")
            .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z")
            .output()
            .expect("git on the PATH");
        assert!(out.status.success(), "git {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr));
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
