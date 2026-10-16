//! The uncommitted change of each worktree, for the graph's WIP rows: its
//! files with their counts, against the worktree's `HEAD`, and when it was
//! last written. Read from the temporary commit of [`predict::snapshot`], so
//! untracked files count (up to its limits) and the index stays as it was. The calls block;
//! run them off the UI thread.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

use crate::git;
use crate::oid::Oid;
use crate::predict::{self, LeftOut};
use crate::show::{self, File};
use crate::worktree::Worktree;

/// A worktree with uncommitted changes, or with untracked files too big to
/// count and nothing else (no files then).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wip {
    /// The worktree's top folder.
    pub path: PathBuf,
    /// Its branch; `None` when detached.
    pub branch: Option<String>,
    /// The commit the change is on (the worktree's `HEAD`).
    pub head: Oid,
    /// The temporary commit holding the change, `head` its parent; `head`
    /// itself when there are no files.
    pub commit: Oid,
    /// The changed files with their counts, no diff read.
    pub files: Vec<File>,
    /// When a changed file that is still there was last written (seconds
    /// since 1970); `None` when the change only deletes.
    pub written: Option<i64>,
    /// Untracked files and folders too big to count.
    pub left_out: Vec<LeftOut>,
    /// The commit being merged in, when the worktree is in the middle of a
    /// merge (`MERGE_HEAD`).
    pub theirs: Option<Oid>,
}

impl Wip {
    /// The lines added and deleted, over the files that have counts.
    pub fn counts(&self) -> (u32, u32) {
        self.files.iter().filter_map(|f| f.counts).fold((0, 0), |(a, d), (fa, fd)| (a + fa, d + fd))
    }
}

/// What [`read`] gives for a worktree.
pub type Read = Result<Option<Wip>, git::Error>;

/// The uncommitted change of the worktree `wt`; `None` when it is clean, bare
/// or gone. Untracked files too big to count and nothing else give a
/// [`Wip`] with no files.
pub fn read(wt: &Worktree) -> Read {
    if wt.bare || wt.prunable.is_some() {
        return Ok(None);
    }
    let Some(snap) = predict::snapshot(&wt.path, predict::Limits::DEFAULT)? else { return Ok(None) };
    // The snapshot's, not `wt.head`: `HEAD` may have moved since the list was read.
    let head = snap.head;
    let theirs = merge_head(&wt.path);
    let Some(commit) = snap.commit else {
        return Ok(Some(Wip {
            path: wt.path.clone(),
            branch: wt.branch.clone(),
            head,
            commit: head,
            files: Vec::new(),
            written: None,
            left_out: snap.left_out,
            theirs,
        }));
    };
    let files = show::changes(&wt.path, &head, &commit, false, &|| false)?.files;
    let written = files.iter().filter_map(|f| modified(&wt.path.join(&f.path))).max();
    Ok(Some(Wip { path: wt.path.clone(), branch: wt.branch.clone(), head, commit, files, written, left_out: snap.left_out, theirs }))
}

/// The commit a merge under way in `wt` brings in.
fn merge_head(wt: &Path) -> Option<Oid> {
    let dir = crate::branch::git_dir(wt)?;
    let text = std::fs::read_to_string(dir.join("MERGE_HEAD")).ok()?;
    Oid::parse(text.lines().next()?.trim())
}

/// [`read`] for each of `worktrees`, a few at a time; the results come in
/// their order.
pub fn read_all(worktrees: &[Worktree]) -> Vec<Read> {
    let next = AtomicUsize::new(0);
    let done: Mutex<Vec<Option<Read>>> = Mutex::new(vec![None; worktrees.len()]);
    let threads = std::thread::available_parallelism().map_or(2, |n| n.get()).clamp(1, 4).min(worktrees.len());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(wt) = worktrees.get(i) else { break };
                let r = read(wt);
                done.lock().unwrap_or_else(|e| e.into_inner())[i] = Some(r);
            });
        }
    });
    let done = done.into_inner().unwrap_or_else(|e| e.into_inner());
    done.into_iter().map(|r| r.unwrap_or(Ok(None))).collect()
}

fn modified(path: &Path) -> Option<i64> {
    let t = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(t.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::show::Status;
    use crate::testrepo::TempRepo;
    use crate::worktree;

    #[test]
    fn each_worktree_s_uncommitted_change_is_read() {
        let r = TempRepo::new("wip-read");
        r.write(&r.repo, "a.txt", "one\ntwo\n");
        r.write(&r.repo, "b.txt", "b\n");
        r.git(&r.repo, &["add", "."]);
        r.git(&r.repo, &["commit", "-q", "-m", "base"]);
        r.git(&r.repo, &["worktree", "add", "-q", "-b", "wt", "../wt"]);
        // The main worktree: an edit, a deletion and an untracked file.
        r.write(&r.repo, "a.txt", "one\nTWO\nthree\n");
        std::fs::remove_file(r.repo.join("b.txt")).unwrap();
        r.write(&r.repo, "new.txt", "n\n");
        let all = worktree::list(&r.repo).unwrap();
        let got = read_all(&all);
        assert_eq!(got.len(), 2);
        let w = got[0].as_ref().unwrap().as_ref().unwrap();
        assert_eq!(w.branch.as_deref(), Some("main"));
        assert_eq!(w.head.to_string(), r.out(&r.repo, &["rev-parse", "HEAD"]));
        let files: Vec<_> = w.files.iter().map(|f| (f.status, f.path.as_str())).collect();
        assert_eq!(files, [(Status::Modified, "a.txt"), (Status::Deleted, "b.txt"), (Status::Added, "new.txt")]);
        assert_eq!(w.counts(), (3, 2));
        assert!(w.written.is_some());
        // The other is clean.
        assert_eq!(got[1], Ok(None));
        // Nothing was staged.
        assert_eq!(r.out(&r.repo, &["diff", "--cached", "--name-only"]), "");
    }

    #[test]
    fn a_merge_under_way_names_the_commit_coming_in() {
        let r = TempRepo::new("wip-merge");
        r.write(
            &r.repo, "a.txt", "one
",
        );
        r.git(&r.repo, &["add", "."]);
        r.git(&r.repo, &["commit", "-q", "-m", "base"]);
        r.git(&r.repo, &["checkout", "-q", "-b", "feat"]);
        r.write(
            &r.repo, "a.txt", "feat
",
        );
        r.git(&r.repo, &["commit", "-q", "-am", "feat"]);
        r.git(&r.repo, &["checkout", "-q", "main"]);
        r.write(
            &r.repo, "a.txt", "main
",
        );
        r.git(&r.repo, &["commit", "-q", "-am", "main"]);
        let all = worktree::list(&r.repo).unwrap();
        let m = std::process::Command::new("git").arg("-C").arg(&r.repo).args(["merge", "feat"]).output().unwrap();
        let w = read(&all[0]).unwrap().unwrap_or_else(|| {
            panic!("no wip: merge {:?} {} {}", m.status, String::from_utf8_lossy(&m.stdout), String::from_utf8_lossy(&m.stderr))
        });
        assert_eq!(w.theirs.map(|t| t.to_string()), Some(r.out(&r.repo, &["rev-parse", "feat"])));
    }

    #[test]
    fn a_change_that_only_deletes_has_no_write_time() {
        let r = TempRepo::new("wip-delete");
        r.write(&r.repo, "a.txt", "a\n");
        r.git(&r.repo, &["add", "."]);
        r.git(&r.repo, &["commit", "-q", "-m", "base"]);
        std::fs::remove_file(r.repo.join("a.txt")).unwrap();
        let wt = &worktree::list(&r.repo).unwrap()[0];
        let w = read(wt).unwrap().unwrap();
        assert_eq!((w.files.len(), w.written), (1, None));
    }
}
