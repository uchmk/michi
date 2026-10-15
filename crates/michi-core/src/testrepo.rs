//! A throwaway repository for the tests: `<temp>/michi-test-<pid>-<name>/repo`
//! with one empty commit on `main`, worktrees made beside it. git runs with no
//! user or system configuration and fixed names, and each call a minute after
//! the one before from 2026-01-01 00:00 UTC, so the tests do not depend on
//! the machine and the commits come in the order they were made.

use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};

use crate::git;

pub struct TempRepo {
    /// The folder holding `repo` and the worktrees.
    pub root: PathBuf,
    /// The main worktree.
    pub repo: PathBuf,
    config: PathBuf,
    /// Minutes past the start for the next call.
    clock: Cell<i64>,
}

/// 2026-01-01 00:00 UTC.
pub const START: i64 = 1_767_225_600;

impl TempRepo {
    pub fn new(name: &str) -> TempRepo {
        let root = std::env::temp_dir().join(format!("michi-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        // git prints long paths; the temp folder can come as a short one
        // (`C:\Users\RUNNER~1\…` on the CI's Windows).
        let root = long_path(&root);
        let repo = root.join("repo");
        fs::create_dir_all(&repo).unwrap();
        let config = root.join("gitconfig");
        fs::write(&config, "").unwrap();
        let r = TempRepo { root, repo, config, clock: Cell::new(0) };
        r.git(&r.repo, &["init", "-q", "-b", "main"]);
        r.git(&r.repo, &["commit", "-q", "--allow-empty", "-m", "first"]);
        r
    }

    /// Runs git in `dir` and panics when it fails.
    pub fn git(&self, dir: &Path, args: &[&str]) {
        self.out(dir, args);
    }

    /// Runs git in `dir` and returns what it wrote, the last line break taken
    /// off; panics when it fails.
    pub fn out(&self, dir: &Path, args: &[&str]) -> String {
        let minute = self.clock.get();
        self.clock.set(minute + 1);
        let date = format!("@{} +0000", START + minute * 60);
        let out = git::command()
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_CONFIG_GLOBAL", &self.config)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "michi")
            .env("GIT_AUTHOR_EMAIL", "michi@example.invalid")
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_NAME", "michi")
            .env("GIT_COMMITTER_EMAIL", "michi@example.invalid")
            .env("GIT_COMMITTER_DATE", &date)
            .output()
            .expect("git on the PATH");
        assert!(out.status.success(), "git {}: {}", args.join(" "), String::from_utf8_lossy(&out.stderr));
        String::from_utf8_lossy(&out.stdout).trim_end_matches(['\n', '\r']).to_owned()
    }

    /// Writes `content` to `rel` under `dir`, making the folders.
    pub fn write(&self, dir: &Path, rel: &str, content: &str) {
        let path = dir.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
}

/// `path` with its short (8.3) names spelled out, without the `\\?\` that
/// `canonicalize` puts in front on Windows.
fn long_path(path: &Path) -> PathBuf {
    let full = fs::canonicalize(path).unwrap();
    match full.to_string_lossy().strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => full,
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
