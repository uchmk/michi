//! Conflict prediction: whether two branches would conflict if merged, found
//! with `git merge-tree --write-tree`, which merges in memory and touches no
//! working folder, branch or index (it only writes objects, which every
//! worktree shares). Uncommitted changes count too: [`tip`] folds a dirty
//! worktree into a temporary commit built from a temporary index file. The
//! calls block; run them off the UI thread. The caller picks the pairs.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::git;
use crate::oid::Oid;

/// What [`predict`] finds: the conflict, `None` for a clean merge, or why git could not tell.
pub type Prediction = Result<Option<Conflict>, git::Error>;

/// A merge that does not go through.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    /// The conflicted paths, in the order git lists them (sorted).
    pub files: Vec<ConflictFile>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictFile {
    /// The path, with `/` between folders.
    pub path: String,
    /// How many conflict hunks (`<<<<<<<` markers) the merged file holds.
    /// `None` when there are none to count: the conflict is not in the
    /// contents (one side deleted or renamed it, a file against a folder), the
    /// file is binary, or counting failed.
    pub hunks: Option<u32>,
}

impl Conflict {
    /// The hunks over all files that have a count.
    pub fn hunks(&self) -> u32 {
        self.files.iter().filter_map(|f| f.hunks).sum()
    }
}

/// Whether merging `b` into `a` (any two revisions) would conflict, run in
/// the repository `repo` (any of its worktrees). `None` when it merges clean.
/// Fails when a revision is unknown or the histories are unrelated.
pub fn predict(repo: &Path, a: &str, b: &str) -> Prediction {
    for rev in [a, b] {
        if rev.starts_with('-') {
            return Err(git::Error::Git(format!("bad revision '{rev}'")));
        }
    }
    let args = ["--no-optional-locks", "merge-tree", "--write-tree", "--name-only", "-z", "--no-messages", a, b];
    let out = git::command().arg("-C").arg(repo).args(args).output().map_err(git::started)?;
    match out.status.code() {
        Some(0) => Ok(None),
        // An unknown revision exits 1 as well, with nothing on standard output.
        Some(1) => match parse(&out.stdout) {
            Some((tree, paths)) => {
                let mut hunks = count_hunks(repo, tree, &paths);
                let files = paths.into_iter().map(|path| ConflictFile { hunks: hunks.remove(&path), path }).collect();
                Ok(Some(Conflict { files }))
            }
            None => Err(git::refused(&String::from_utf8_lossy(&out.stderr))),
        },
        _ => Err(git::refused(&String::from_utf8_lossy(&out.stderr))),
    }
}

/// [`predict`] for many pairs of revisions at once, on a few threads (each
/// call is mostly the cost of starting git). The results come in the order
/// of `pairs`.
pub fn predict_many(repo: &Path, pairs: &[(String, String)]) -> Vec<Prediction> {
    let next = AtomicUsize::new(0);
    let done: Mutex<Vec<Option<Prediction>>> = Mutex::new(vec![None; pairs.len()]);
    let threads = std::thread::available_parallelism().map_or(2, |n| n.get()).clamp(1, 4).min(pairs.len());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some((a, b)) = pairs.get(i) else { break };
                let r = predict(repo, a, b);
                done.lock().unwrap_or_else(|e| e.into_inner())[i] = Some(r);
            });
        }
    });
    let done = done.into_inner().unwrap_or_else(|e| e.into_inner());
    done.into_iter().map(|r| r.unwrap_or_else(|| Err(git::Error::Io("git merge-tree: not run".into())))).collect()
}

/// The revision to predict with for the worktree `worktree`: a temporary
/// commit of its working folder when it has uncommitted changes (see
/// [`wip_commit`]), otherwise its `HEAD`.
pub fn tip(worktree: &Path) -> Result<Oid, git::Error> {
    match wip_commit(worktree)? {
        Some(wip) => Ok(wip),
        None => Ok(head(worktree)?.0),
    }
}

/// A commit holding the working folder of `worktree` as it is now (tracked
/// changes, deleted files and untracked files that are not ignored), with
/// `HEAD` as its parent, so `HEAD` against it is the uncommitted change.
/// `None` when the folder is clean. The index, `HEAD`, the branch and the
/// files stay as they were: the commit is built from a temporary copy of
/// the index (so `git add -A` skips what the stat data says is unchanged),
/// deleted afterwards. The real index is only read, and no call takes its
/// lock. The commit is no ref's target, so `git gc` takes it in time. It
/// has fixed names and dates, so the same state gives the same commit.
pub fn wip_commit(worktree: &Path) -> Result<Option<Oid>, git::Error> {
    // `core.fsmonitor=false`: a read must not start a file system monitor.
    let quiet = ["--no-optional-locks", "-c", "core.fsmonitor=false"];
    let status = git::read(worktree, &[quiet[0], quiet[1], quiet[2], "status", "--porcelain=v2", "-z", "--untracked-files=all"])?;
    if status.is_empty() {
        return Ok(None);
    }
    let (head, head_tree) = head(worktree)?;
    let head_hex = head.to_string();
    let index = TempIndex::new();
    let with_index = |args: &[&str]| -> Result<String, git::Error> {
        let mut c = git::command();
        c.arg("-C").arg(worktree).args(quiet).args(args).env("GIT_INDEX_FILE", &index.0);
        c.env("GIT_AUTHOR_NAME", "michi").env("GIT_AUTHOR_EMAIL", "michi@example.invalid").env("GIT_AUTHOR_DATE", "@0 +0000");
        c.env("GIT_COMMITTER_NAME", "michi").env("GIT_COMMITTER_EMAIL", "michi@example.invalid").env("GIT_COMMITTER_DATE", "@0 +0000");
        text(c, args)
    };
    if !index.seed_from(worktree) {
        with_index(&["read-tree", &head_hex])?;
    }
    with_index(&["add", "-A"])?;
    let tree = with_index(&["write-tree"])?;
    if tree == head_tree {
        return Ok(None);
    }
    let commit = with_index(&["commit-tree", &tree, "-p", &head_hex, "-m", "wip", "--no-gpg-sign"])?;
    parse_oid(&commit, "commit-tree").map(Some)
}

/// `HEAD` of `worktree` and its tree.
fn head(worktree: &Path) -> Result<(Oid, String), git::Error> {
    let text = git::read_text(worktree, &["rev-parse", "HEAD^{commit}", "HEAD^{tree}"])?;
    let mut lines = text.lines();
    let commit = lines.next().unwrap_or_default();
    let tree = lines.next().unwrap_or_default();
    Ok((parse_oid(commit, "rev-parse")?, tree.to_owned()))
}

fn parse_oid(hex: &str, what: &str) -> Result<Oid, git::Error> {
    Oid::parse(hex.trim()).ok_or_else(|| git::Error::Io(format!("git {what}: unexpected output: {}", hex.trim())))
}

/// Runs `c` and returns its standard output, the last line break taken off.
fn text(mut c: Command, args: &[&str]) -> Result<String, git::Error> {
    let out = c.output().map_err(git::started)?;
    if !out.status.success() {
        return Err(git::refused(&String::from_utf8_lossy(&out.stderr)));
    }
    let s = String::from_utf8(out.stdout).map_err(|e| git::Error::Io(format!("git {}: {e}", args.join(" "))))?;
    Ok(s.trim_end_matches(['\n', '\r']).to_owned())
}

/// A temporary index file, deleted (with its lock) when dropped.
struct TempIndex(PathBuf);

impl TempIndex {
    fn new() -> TempIndex {
        static N: AtomicUsize = AtomicUsize::new(0);
        let name = format!("michi-wip-index-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed));
        let t = TempIndex(std::env::temp_dir().join(name));
        t.remove();
        t
    }

    /// Copies the index `worktree` uses into it; whether there was one to
    /// copy. The original is only read.
    fn seed_from(&self, worktree: &Path) -> bool {
        let Ok(path) = git::read_text(worktree, &["rev-parse", "--git-path", "index"]) else { return false };
        // Relative to the folder git ran in.
        let real = worktree.join(path);
        std::fs::copy(real, &self.0).is_ok()
    }

    fn remove(&self) {
        let _ = std::fs::remove_file(&self.0);
        let mut lock = self.0.clone().into_os_string();
        lock.push(".lock");
        let _ = std::fs::remove_file(lock);
    }
}

impl Drop for TempIndex {
    fn drop(&mut self) {
        self.remove();
    }
}

/// The tree oid and the conflicted paths from `merge-tree --name-only -z`:
/// the tree and a NUL, each path and a NUL, then, unless `--no-messages`
/// was given, an empty entry and the informational messages (which are
/// not read). `None` when it does not start with a tree.
fn parse(out: &[u8]) -> Option<(Oid, Vec<String>)> {
    let mut parts = out.split(|&b| b == 0);
    let tree = Oid::parse(std::str::from_utf8(parts.next()?).ok()?)?;
    let paths = parts.take_while(|p| !p.is_empty()).map(|p| String::from_utf8_lossy(p).into_owned()).collect();
    Some((tree, paths))
}

/// The conflict hunks of each of `paths` in the merged `tree`, by
/// `git grep -c` for the marker lines (the merge writes the marked-up file
/// into the tree). Paths with no marker are left out, and a failing git
/// leaves its chunk out too.
fn count_hunks(repo: &Path, tree: Oid, paths: &[String]) -> HashMap<String, u32> {
    let tree = tree.to_string();
    let mut counts = HashMap::new();
    // A few at a time: the command line is not unlimited, on Windows least.
    let mut start = 0;
    while start < paths.len() {
        let mut end = start;
        let mut size = 0;
        while end < paths.len() && end - start < 200 && size < 8000 {
            size += paths[end].len() + 1;
            end += 1;
        }
        let mut c = git::command();
        c.arg("-C").arg(repo).args(["--no-optional-locks", "--literal-pathspecs", "grep", "-c", "-z", "-I", "-E", "-e", "^<<<<<<<( |$)"]);
        c.arg(&tree).arg("--").args(paths[start..end].iter().map(OsStr::new));
        // Exit 1 is no match at all.
        if let Ok(out) = c.output() {
            if matches!(out.status.code(), Some(0 | 1)) {
                read_counts(&out.stdout, &tree, &mut counts);
            }
        }
        start = end;
    }
    counts
}

/// The records `<tree>:<path>` NUL `<count>` LF of `git grep -c -z`.
fn read_counts(out: &[u8], tree: &str, into: &mut HashMap<String, u32>) {
    let prefix = format!("{tree}:");
    let mut rest = out;
    while let Some(nul) = rest.iter().position(|&b| b == 0) {
        let name = String::from_utf8_lossy(&rest[..nul]).into_owned();
        let after = &rest[nul + 1..];
        let end = after.iter().position(|&b| b == b'\n').unwrap_or(after.len());
        let count = std::str::from_utf8(&after[..end]).ok().and_then(|n| n.trim().parse::<u32>().ok());
        if let (Some(path), Some(n)) = (name.strip_prefix(&prefix), count) {
            if n > 0 {
                into.insert(path.to_owned(), n);
            }
        }
        rest = after.get(end + 1..).unwrap_or_default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;
    use std::fs;

    const TREE: &str = "93ce6a0d51212fe02e1f59de6b3f74ebed80e7cb";

    /// Ten lines `a`..`j`, with the first and the last replaced.
    fn lines(first: &str, last: &str) -> String {
        format!("{first}\nb\nc\nd\ne\nf\ng\nh\ni\n{last}\n")
    }

    /// A repository with `f.txt` committed on `main`, and the worktree
    /// `../wt` on the new branch `wt` at that commit.
    fn base(name: &str) -> TempRepo {
        let r = TempRepo::new(name);
        r.write(&r.repo, "f.txt", &lines("a", "j"));
        r.git(&r.repo, &["add", "."]);
        r.git(&r.repo, &["commit", "-q", "-m", "base"]);
        r.git(&r.repo, &["worktree", "add", "-q", "-b", "wt", "../wt"]);
        r
    }

    fn commit_in(r: &TempRepo, dir: &Path, file: &str, content: &str) {
        r.write(dir, file, content);
        r.git(dir, &["add", "."]);
        r.git(dir, &["commit", "-q", "-m", file]);
    }

    fn state(r: &TempRepo, wt: &Path) -> (String, String, Vec<u8>, std::time::SystemTime) {
        let status = r.out(wt, &["status", "--porcelain=v2", "--untracked-files=all"]);
        let head = r.out(wt, &["rev-parse", "HEAD"]);
        let index = PathBuf::from(r.out(wt, &["rev-parse", "--absolute-git-dir"])).join("index");
        (status, head, fs::read(&index).unwrap(), fs::metadata(&index).unwrap().modified().unwrap())
    }

    #[test]
    fn the_output_of_merge_tree_is_read_with_or_without_messages() {
        let names = format!("{TREE}\0f.txt\0dir/g.txt\0");
        let (tree, paths) = parse(names.as_bytes()).unwrap();
        assert_eq!(tree.to_string(), TREE);
        assert_eq!(paths, ["f.txt", "dir/g.txt"]);
        let with = format!("{TREE}\0f.txt\0\0\x31\0f.txt\0Auto-merging\0Auto-merging f.txt\n\0");
        assert_eq!(parse(with.as_bytes()).unwrap().1, ["f.txt"]);
        assert_eq!(parse(format!("{TREE}\0").as_bytes()).unwrap().1, Vec::<String>::new());
        assert!(parse(b"").is_none() && parse(b"merge-tree: nope\0").is_none());
    }

    #[test]
    fn the_counts_of_git_grep_are_read() {
        let out = format!("{TREE}:f.txt\x002\n{TREE}:dir/a b.txt\x001\n{TREE}:z\x000\n");
        let mut m = HashMap::new();
        read_counts(out.as_bytes(), TREE, &mut m);
        assert_eq!(m.len(), 2);
        assert_eq!((m["f.txt"], m["dir/a b.txt"]), (2, 1));
    }

    #[test]
    fn a_clean_merge_predicts_nothing() {
        let r = base("predict-clean");
        commit_in(&r, &r.repo, "g.txt", "main\n");
        commit_in(&r, &r.root.join("wt"), "h.txt", "wt\n");
        assert_eq!(predict(&r.repo, "main", "wt"), Ok(None));
        // Edits to different ends of the same file merge as well.
        commit_in(&r, &r.root.join("wt"), "f.txt", &lines("a", "J"));
        commit_in(&r, &r.repo, "f.txt", &lines("A", "j"));
        assert_eq!(predict(&r.repo, "main", "wt"), Ok(None));
    }

    #[test]
    fn a_conflict_names_its_files_and_counts_its_hunks() {
        let r = base("predict-conflict");
        commit_in(&r, &r.repo, "f.txt", &lines("main", "main"));
        commit_in(&r, &r.root.join("wt"), "f.txt", &lines("wt", "wt"));
        let c = predict(&r.repo, "main", "wt").unwrap().unwrap();
        assert_eq!(c.files, [ConflictFile { path: "f.txt".into(), hunks: Some(2) }]);
        assert_eq!(c.hunks(), 2);
        // The same from the other worktree and the other way round.
        assert_eq!(predict(&r.root.join("wt"), "wt", "main").unwrap(), Some(c));
    }

    #[test]
    fn a_conflict_without_markers_has_no_hunk_count() {
        let r = base("predict-modify-delete");
        r.write(&r.repo, "d/g.txt", "one\n");
        r.git(&r.repo, &["add", "."]);
        r.git(&r.repo, &["commit", "-q", "-m", "g"]);
        let wt = r.root.join("wt");
        r.git(&wt, &["merge", "-q", "main"]);
        r.git(&r.repo, &["rm", "-q", "d/g.txt"]);
        r.git(&r.repo, &["commit", "-q", "-m", "rm"]);
        commit_in(&r, &wt, "d/g.txt", "two\n");
        let c = predict(&r.repo, "main", "wt").unwrap().unwrap();
        assert_eq!(c.files, [ConflictFile { path: "d/g.txt".into(), hunks: None }]);
        assert_eq!(c.hunks(), 0);
    }

    #[test]
    fn many_pairs_come_back_in_order() {
        let r = base("predict-many");
        commit_in(&r, &r.repo, "f.txt", &lines("main", "j"));
        let wt = r.root.join("wt");
        let pairs = [("main".to_owned(), "wt".to_owned()), ("main".to_owned(), "nope".to_owned()), ("wt".to_owned(), "wt".to_owned())];
        assert_eq!(predict_many(&r.repo, &[]), vec![]);
        let all = predict_many(&r.repo, &pairs);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0], Ok(None));
        assert!(matches!(all[1], Err(git::Error::Git(_))));
        assert_eq!(all[2], Ok(None));
        commit_in(&r, &wt, "f.txt", &lines("wt", "j"));
        assert!(predict_many(&r.repo, &pairs[..1])[0].as_ref().is_ok_and(Option::is_some));
    }

    #[test]
    fn an_unknown_revision_is_an_error() {
        let r = base("predict-unknown");
        assert!(matches!(predict(&r.repo, "main", "nope"), Err(git::Error::Git(_))));
        assert!(matches!(predict(&r.repo, "nope", "main"), Err(git::Error::Git(_))));
        assert!(matches!(predict(&r.repo, "--help", "main"), Err(git::Error::Git(_))));
        assert!(predict(&r.root, "main", "wt").is_err());
    }

    #[test]
    fn a_clean_worktree_has_no_wip_commit() {
        let r = base("predict-wip-clean");
        let wt = r.root.join("wt");
        assert_eq!(wip_commit(&wt), Ok(None));
        assert_eq!(wip_commit(&r.repo), Ok(None));
        assert_eq!(tip(&wt).unwrap().to_string(), r.out(&wt, &["rev-parse", "HEAD"]));
    }

    #[test]
    fn uncommitted_changes_conflict_and_leave_the_worktree_alone() {
        let r = base("predict-wip");
        let wt = r.root.join("wt");
        commit_in(&r, &r.repo, "f.txt", &lines("main", "j"));
        // One edit staged, one not, and a file removed.
        r.write(&wt, "f.txt", &lines("wt", "j"));
        r.git(&wt, &["add", "f.txt"]);
        r.write(&wt, "f.txt", &lines("wt", "J"));
        assert_eq!(predict(&r.repo, "main", "wt"), Ok(None));
        let before = state(&r, &wt);
        let files = fs::read_to_string(wt.join("f.txt")).unwrap();

        let wip = wip_commit(&wt).unwrap().expect("a dirty folder");
        assert_eq!(tip(&wt).unwrap(), wip);
        assert_eq!(wip_commit(&wt).unwrap(), Some(wip), "the same state, the same commit");
        assert_eq!(r.out(&wt, &["rev-parse", &format!("{wip}^")]), before.1);
        assert_eq!(r.out(&wt, &["show", &format!("{wip}:f.txt")]), lines("wt", "J").trim_end());
        let c = predict(&r.repo, "main", &wip.to_string()).unwrap().unwrap();
        assert_eq!(c.files, [ConflictFile { path: "f.txt".into(), hunks: Some(1) }]);

        assert_eq!(state(&r, &wt), before, "status, HEAD, index contents and mtime");
        assert_eq!(fs::read_to_string(wt.join("f.txt")).unwrap(), files);
        assert!(!fs::read_dir(r.root.join("repo/.git/worktrees/wt")).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("lock")));
    }

    #[test]
    fn untracked_files_count() {
        let r = base("predict-untracked");
        let wt = r.root.join("wt");
        commit_in(&r, &r.repo, "n.txt", "main\n");
        r.write(&wt, "sub/n.txt", "x\n");
        r.write(&wt, "n.txt", "wt\n");
        let before = state(&r, &wt);
        let wip = wip_commit(&wt).unwrap().expect("untracked files");
        let c = predict(&r.repo, "main", &wip.to_string()).unwrap().unwrap();
        assert_eq!(c.files.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), ["n.txt"]);
        assert_eq!(r.out(&wt, &["show", &format!("{wip}:sub/n.txt")]), "x");
        assert_eq!(state(&r, &wt), before);
        // Only an ignored file is no change.
        fs::remove_file(wt.join("n.txt")).unwrap();
        fs::remove_dir_all(wt.join("sub")).unwrap();
        r.write(&wt, "ignored.log", "x\n");
        let exclude = PathBuf::from(r.out(&wt, &["rev-parse", "--git-common-dir"]));
        fs::create_dir_all(exclude.join("info")).unwrap();
        fs::write(exclude.join("info/exclude"), "*.log\n").unwrap();
        assert_eq!(wip_commit(&wt), Ok(None));
    }
}
