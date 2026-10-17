//! Throwing away what a worktree has not staged (docs/design.md: 変更を捨てる):
//! a file back as the index has it, an untracked one deleted, or chosen
//! lines and hunks taken out by a patch to `git apply -R`. The index stays
//! as it is.
//!
//! What goes is kept under `refs/michi/backup/` first: the tracked changes
//! as `git stash create` makes them, untracked files in a commit of their
//! own on the head, made with an index of its own.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::backup;
use crate::git;
use crate::oid::Oid;

/// Throws away the unstaged changes of the tracked files at `paths` in the
/// worktree at `wt`: they go back as the index has them. The backup's ref.
pub fn files(wt: &Path, paths: &[&str]) -> Result<String, git::Error> {
    let kept = keep_tracked(wt, &format!("discarded {}", paths.join(", ")))?;
    let mut c = git::command();
    c.env("GIT_LITERAL_PATHSPECS", "1").arg("-C").arg(wt).args(["restore", "--worktree", "--"]).args(paths);
    run(c)?;
    Ok(kept)
}

/// Deletes the untracked files at `paths` in the worktree at `wt`, and the
/// folders that are left empty by it. The backup's ref.
pub fn untracked(wt: &Path, paths: &[&str]) -> Result<String, git::Error> {
    let kept = keep_untracked(wt, paths)?;
    for p in paths {
        let file = wt.join(p);
        fs::remove_file(&file).map_err(|e| git::Error::Io(format!("{p}: {e}")))?;
        // Up to the worktree, while each folder is empty.
        let mut dir = file.parent();
        while let Some(d) = dir.filter(|d| d.starts_with(wt) && *d != wt) {
            if fs::remove_dir(d).is_err() {
                break;
            }
            dir = d.parent();
        }
    }
    Ok(kept)
}

/// Takes the lines of `patch` out of the worktree at `wt`: a patch of the
/// worktree against the index, from [`crate::stage::Patch::pick`] with
/// `reverse` (the lines not picked kept as they are in the worktree). The
/// backup's ref.
pub fn lines(wt: &Path, path: &str, patch: &[u8]) -> Result<String, git::Error> {
    let kept = keep_tracked(wt, &format!("discarded lines of {path}"))?;
    let mut c = git::command();
    c.arg("-C").arg(wt).args(["apply", "--reverse", "--recount", "--whitespace=nowarn", "-"]);
    git::feed(c, patch)?;
    Ok(kept)
}

/// The tracked changes of the worktree at `wt` and its index, kept under a
/// backup with `why`.
fn keep_tracked(wt: &Path, why: &str) -> Result<String, git::Error> {
    let out = git::read_text(wt, &["stash", "create", &format!("michi: {why}")])?;
    let id = Oid::parse(out.trim()).ok_or_else(|| git::Error::Git("nothing to discard".into()))?;
    backup::save(wt, &id, why)
}

/// The untracked files at `paths` of the worktree at `wt`, kept in a commit
/// on the head (on nothing before the first commit) under a backup.
fn keep_untracked(wt: &Path, paths: &[&str]) -> Result<String, git::Error> {
    keep_files(wt, paths, &format!("deleted untracked {}", paths.join(", ")))
}

/// The files at `paths` of the worktree at `wt` as they are now, kept in a
/// commit on the head (on nothing before the first commit) under a backup
/// with `why`.
pub(crate) fn keep_files(wt: &Path, paths: &[&str], why: &str) -> Result<String, git::Error> {
    let index = TempIndex::new();
    let head = git::read_maybe(wt, &["rev-parse", "-q", "--verify", "HEAD^{commit}"])?
        .and_then(|out| Oid::parse(String::from_utf8_lossy(&out).trim()));
    let mut c = index.command(wt);
    match &head {
        Some(h) => c.args(["read-tree", &h.to_string()]),
        None => c.args(["read-tree", "--empty"]),
    };
    run(c)?;
    let mut c = index.command(wt);
    c.env("GIT_LITERAL_PATHSPECS", "1").args(["add", "-f", "--"]).args(paths);
    run(c)?;
    let mut c = index.command(wt);
    c.arg("write-tree");
    let tree = String::from_utf8_lossy(&run(c)?).trim().to_owned();
    let mut c = git::command();
    c.arg("-C").arg(wt).args(["commit-tree", &tree, "-m", &format!("michi: {why}")]);
    if let Some(h) = &head {
        c.args(["-p", &h.to_string()]);
    }
    let out = run(c)?;
    let id = Oid::parse(String::from_utf8_lossy(&out).trim()).ok_or_else(|| git::Error::Io("git commit-tree: no commit".into()))?;
    backup::save(wt, &id, why)
}

/// An index file of our own, gone when dropped.
pub(crate) struct TempIndex(PathBuf);

static COUNT: AtomicUsize = AtomicUsize::new(0);

impl TempIndex {
    pub(crate) fn new() -> TempIndex {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        let n = COUNT.fetch_add(1, Ordering::Relaxed);
        TempIndex(std::env::temp_dir().join(format!("michi-index-{}-{nanos}-{n}", std::process::id())))
    }

    /// git in the worktree at `wt` with this index.
    pub(crate) fn command(&self, wt: &Path) -> Command {
        let mut c = git::command();
        c.env("GIT_INDEX_FILE", &self.0).arg("-C").arg(wt);
        c
    }
}

impl Drop for TempIndex {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

/// What git wrote, or all it said when it refused.
pub(crate) fn run(mut c: Command) -> Result<Vec<u8>, git::Error> {
    let out = c.output().map_err(git::started)?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(git::refused_all(&String::from_utf8_lossy(&out.stderr)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::show::Kind;
    use crate::stage::{self, Side};
    use crate::testrepo::TempRepo;

    const TEN: &str = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12\n13\n14\n15\n16\n";

    fn repo(name: &str) -> TempRepo {
        let t = TempRepo::configured(name);
        t.write(&t.repo, "n.txt", TEN);
        t.write(&t.repo, "b.txt", "b\n");
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "files"]);
        t
    }

    fn read(t: &TempRepo, rel: &str) -> String {
        fs::read_to_string(t.repo.join(rel)).unwrap()
    }

    #[test]
    fn a_file_goes_back_to_the_index_and_its_changes_are_kept() {
        let t = repo("discard-file");
        t.write(&t.repo, "n.txt", &TEN.replace("2\n", "two\n"));
        t.git(&t.repo, &["add", "n.txt"]);
        t.write(&t.repo, "n.txt", &TEN.replace("2\n", "two\n").replace("9\n", "nine\n"));
        t.write(&t.repo, "b.txt", "B\n");
        let kept = files(&t.repo, &["n.txt"]).unwrap();
        assert!(kept.starts_with(backup::PREFIX), "{kept}");
        // What was staged stays; the worktree's own change is gone.
        assert_eq!(read(&t, "n.txt"), TEN.replace("2\n", "two\n"));
        assert_eq!(t.out(&t.repo, &["diff", "--cached", "--name-only"]), "n.txt");
        assert_eq!(read(&t, "b.txt"), "B\n", "another file is left");
        // The backup has the worktree as it was.
        assert_eq!(t.out(&t.repo, &["show", &format!("{kept}:n.txt")]) + "\n", TEN.replace("2\n", "two\n").replace("9\n", "nine\n"));
        assert_eq!(t.out(&t.repo, &["reflog", "-1", "--format=%gs", &kept]), "michi: discarded n.txt");
        // Nothing changed anywhere: nothing to keep, nothing done.
        t.git(&t.repo, &["reset", "-q", "--hard"]);
        assert_eq!(files(&t.repo, &["b.txt"]).unwrap_err().to_string(), "nothing to discard");
    }

    #[test]
    fn chosen_lines_go_and_the_rest_stay() {
        let t = repo("discard-lines");
        let changed = TEN.replace("2\n3\n", "two\n").replace("14\n", "fourteen\n") + "x\n";
        t.write(&t.repo, "n.txt", &changed);
        let file = stage::lists(&t.repo).unwrap().unstaged.into_iter().find(|f| f.path == "n.txt").unwrap();
        let d = stage::diff(&t.repo, Side::Unstaged, &file).unwrap().unwrap();
        // `fourteen` out and `3` back, where the diff had it: before `two`.
        // 14's going, 2's, `two` and `x` stay.
        let mut chosen = Vec::new();
        for (h, hunk) in d.file.hunks.iter().enumerate() {
            for (k, l) in hunk.lines.iter().enumerate() {
                if (l.kind == Kind::Add && l.text == "fourteen") || (l.kind == Kind::Del && l.text == "3") {
                    chosen.push((h, k));
                }
            }
        }
        assert_eq!(chosen.len(), 2);
        let patch = d.patch.as_ref().unwrap().pick(|h, k| chosen.contains(&(h, k)), true).unwrap();
        let kept = lines(&t.repo, "n.txt", &patch).unwrap();
        assert_eq!(read(&t, "n.txt"), TEN.replace("2\n3\n", "3\ntwo\n").replace("14\n", "") + "x\n");
        assert_eq!(t.out(&t.repo, &["show", &format!("{kept}:n.txt")]) + "\n", changed);
        assert_eq!(t.out(&t.repo, &["diff", "--cached", "--name-only"]), "", "the index is as it was");
    }

    #[test]
    fn an_untracked_file_is_deleted_and_kept_in_a_commit_on_the_head() {
        let t = repo("discard-untracked");
        t.write(&t.repo, "new/deep/one.txt", "1\n");
        t.write(&t.repo, "new/two.txt", "2\n");
        t.write(&t.repo, "b.txt", "B\n");
        let kept = untracked(&t.repo, &["new/deep/one.txt"]).unwrap();
        assert!(!t.repo.join("new/deep").exists(), "its empty folder goes");
        assert_eq!(read(&t, "new/two.txt"), "2\n", "the other stays");
        assert_eq!(read(&t, "b.txt"), "B\n");
        assert_eq!(t.out(&t.repo, &["show", &format!("{kept}:new/deep/one.txt")]), "1");
        assert_eq!(t.out(&t.repo, &["rev-parse", &format!("{kept}^")]), t.out(&t.repo, &["rev-parse", "HEAD"]));
        // The backup has the head's files and not the worktree's changes.
        assert_eq!(t.out(&t.repo, &["show", &format!("{kept}:b.txt")]), "b");
        assert_eq!(t.out(&t.repo, &["status", "--porcelain", "--untracked-files=all"]), " M b.txt\n?? new/two.txt");
        assert_eq!(t.out(&t.repo, &["diff", "--cached", "--name-only"]), "");
    }

    #[test]
    fn an_untracked_file_before_the_first_commit_is_kept_too() {
        let t = TempRepo::configured("discard-unborn");
        t.git(&t.repo, &["checkout", "-q", "--orphan", "fresh"]);
        t.git(&t.repo, &["rm", "-rq", "--cached", "--ignore-unmatch", "."]);
        t.write(&t.repo, "a.txt", "a\n");
        let kept = untracked(&t.repo, &["a.txt"]).unwrap();
        assert!(!t.repo.join("a.txt").exists());
        assert_eq!(t.out(&t.repo, &["show", &format!("{kept}:a.txt")]), "a");
        assert_eq!(t.out(&t.repo, &["rev-list", "--count", &kept]), "1");
    }
}
