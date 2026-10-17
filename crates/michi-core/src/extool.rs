//! The files an external diff or merge tool is given. A tool wants real
//! files, and git keeps the old sides in its object store, so each side that
//! is not in the working tree is written into a folder of its own in the
//! system's temporary folder (with the file's own name and extension, so the
//! tool picks the language). A side that does not exist (a new or deleted
//! file) is an empty file. Reads git: call it off the UI thread.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use crate::git;

/// The files of a diff: the old side and the new one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pair {
    pub left: PathBuf,
    pub right: PathBuf,
}

/// The files of a three-way merge of a clashed file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Three {
    /// Ours (the stage 2 side).
    pub left: PathBuf,
    /// Theirs (the stage 3 side).
    pub right: PathBuf,
    /// The common ancestor (stage 1); empty when the file was added on both sides.
    pub base: PathBuf,
    /// The file in the working tree, which the tool writes the result to.
    pub merged: PathBuf,
}

/// The diff of `path` in the worktree `wt`: the index against the working
/// file, or (`staged`) HEAD against the index.
pub fn diff_files(wt: &Path, path: &str, staged: bool) -> Result<Pair, git::Error> {
    let dir = fresh_dir()?;
    if staged {
        let left = put(&dir, path, "HEAD", &blob(wt, &format!("HEAD:{path}"))?)?;
        let right = put(&dir, path, "index", &blob(wt, &format!(":{path}"))?)?;
        Ok(Pair { left, right })
    } else {
        let left = put(&dir, path, "index", &blob(wt, &format!(":{path}"))?)?;
        Ok(Pair { left, right: wt.join(path) })
    }
}

/// The three sides of the clashed `path` in the worktree `wt`.
pub fn merge_files(wt: &Path, path: &str) -> Result<Three, git::Error> {
    let dir = fresh_dir()?;
    let left = put(&dir, path, "ours", &blob(wt, &format!(":2:{path}"))?)?;
    let right = put(&dir, path, "theirs", &blob(wt, &format!(":3:{path}"))?)?;
    let base = put(&dir, path, "base", &blob(wt, &format!(":1:{path}"))?)?;
    Ok(Three { left, right, base, merged: wt.join(path) })
}

/// What `spec` names, or nothing when git has no such file there.
fn blob(wt: &Path, spec: &str) -> Result<Vec<u8>, git::Error> {
    match git::read(wt, &["show", spec]) {
        Ok(b) => Ok(b),
        Err(git::Error::Git(_) | git::Error::Told { .. }) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// `<temp>/michi-extool/<n>`, made new; folders from earlier days are removed.
fn fresh_dir() -> Result<PathBuf, git::Error> {
    static N: AtomicU32 = AtomicU32::new(0);
    let root = std::env::temp_dir().join("michi-extool");
    sweep(&root);
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
    let dir = root.join(format!("{stamp}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    std::fs::create_dir_all(&dir).map_err(|e| git::Error::Io(format!("{}: {e}", dir.display())))?;
    Ok(dir)
}

fn sweep(root: &Path) {
    let day = std::time::Duration::from_secs(24 * 3600);
    let Ok(entries) = std::fs::read_dir(root) else { return };
    for e in entries.flatten() {
        let old = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age > day);
        if old {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

/// `bytes` in `dir` as `<stem>.<label>.<ext>` after the file `path`.
fn put(dir: &Path, path: &str, label: &str, bytes: &[u8]) -> Result<PathBuf, git::Error> {
    let name = Path::new(path).file_name().map_or_else(|| "file".to_owned(), |n| n.to_string_lossy().into_owned());
    let named = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => format!("{stem}.{label}.{ext}"),
        _ => format!("{name}.{label}"),
    };
    let to = dir.join(named);
    std::fs::write(&to, bytes).map_err(|e| git::Error::Io(format!("{}: {e}", to.display())))?;
    Ok(to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn the_old_side_is_a_file_and_the_new_side_is_the_working_file() {
        let t = TempRepo::configured("extool-diff");
        let r = &t.repo;
        t.write(r, "a.txt", "one\n");
        t.git(r, &["add", "."]);
        t.git(r, &["commit", "-q", "-m", "one"]);
        t.write(r, "a.txt", "two\n");
        let p = diff_files(r, "a.txt", false).unwrap();
        assert_eq!(std::fs::read_to_string(&p.left).unwrap(), "one\n");
        assert_eq!(p.right, r.join("a.txt"));
        assert_eq!(p.left.file_name().unwrap().to_string_lossy(), "a.index.txt");
        t.git(r, &["add", "."]);
        let p = diff_files(r, "a.txt", true).unwrap();
        assert_eq!(std::fs::read_to_string(&p.left).unwrap(), "one\n");
        assert_eq!(std::fs::read_to_string(&p.right).unwrap(), "two\n");
    }

    #[test]
    fn a_side_that_is_not_there_is_an_empty_file() {
        let t = TempRepo::configured("extool-new");
        let r = &t.repo;
        t.write(r, "a.txt", "one\n");
        t.git(r, &["add", "."]);
        t.git(r, &["commit", "-q", "-m", "one"]);
        t.write(r, "new", "x\n");
        let p = diff_files(r, "new", false).unwrap();
        assert_eq!(std::fs::read(&p.left).unwrap(), b"");
        assert_eq!(p.left.file_name().unwrap().to_string_lossy(), "new.index");
    }

    #[test]
    fn a_clash_gives_ours_theirs_and_the_base() {
        let t = TempRepo::configured("extool-merge");
        let r = &t.repo;
        t.write(r, "a.txt", "base\n");
        t.git(r, &["add", "."]);
        t.git(r, &["commit", "-q", "-m", "base"]);
        let main = t.out(r, &["branch", "--show-current"]);
        t.git(r, &["switch", "-q", "-c", "feat"]);
        t.write(r, "a.txt", "feat\n");
        t.git(r, &["commit", "-q", "-am", "feat"]);
        t.git(r, &["switch", "-q", &main]);
        t.write(r, "a.txt", "main\n");
        t.git(r, &["commit", "-q", "-am", "main"]);
        let merged = git::command().arg("-C").arg(r).args(["merge", "feat"]).output().unwrap();
        assert!(!merged.status.success());
        let m = merge_files(r, "a.txt").unwrap();
        assert_eq!(std::fs::read_to_string(&m.left).unwrap(), "main\n");
        assert_eq!(std::fs::read_to_string(&m.right).unwrap(), "feat\n");
        assert_eq!(std::fs::read_to_string(&m.base).unwrap(), "base\n");
        assert_eq!(m.merged, r.join("a.txt"));
    }
}
