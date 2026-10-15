//! Which changes on disk matter: the window watches each worktree's folder
//! and the common git folder, and reads the worktrees again only when a
//! change could show (docs/design.md: "外からの変化はファイルの監視で拾う").
//!
//! In the git folder the refs, `HEAD`, the index and the worktrees' own
//! folders count; the objects, the logs and the lock files do not (michi's
//! own reading writes objects for the conflict prediction). In a worktree a
//! file git ignores does not count, the build's output and the like; what git
//! ignores is asked of `git check-ignore`, and the folders it ignores are
//! remembered until a `.gitignore` changes.

use std::collections::{HashMap, HashSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use crate::git;
use crate::paths;

/// More changed paths than this to ask about: taken as a change.
pub const MOST: usize = 5000;

/// The folders watched, as [`paths::key`]s.
pub struct Places {
    common: String,
    /// Each worktree's folder and its key, the longest key first (so a
    /// worktree inside another is found before it).
    trees: Vec<(PathBuf, String)>,
}

impl Places {
    /// The repository whose common git folder is `common`, with the
    /// worktrees in `trees`.
    pub fn new(common: &Path, trees: &[PathBuf]) -> Places {
        let mut trees: Vec<(PathBuf, String)> = trees.iter().map(|p| (p.clone(), paths::key(p))).collect();
        trees.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.1.cmp(&b.1)));
        trees.dedup_by(|a, b| a.1 == b.1);
        Places { common: paths::key(common), trees }
    }

    /// The folders to watch, each with all below it: every worktree and the
    /// common git folder, but none inside another.
    pub fn roots(&self, common: &Path) -> Vec<PathBuf> {
        let mut all: Vec<(PathBuf, String)> = self.trees.clone();
        all.push((common.to_path_buf(), self.common.clone()));
        // The shortest first: a folder inside one kept is left out.
        all.sort_by(|a, b| a.1.len().cmp(&b.1.len()).then_with(|| a.1.cmp(&b.1)));
        let mut kept: Vec<(PathBuf, String)> = Vec::new();
        for (p, k) in all {
            if !kept.iter().any(|(_, d)| paths::contains_key(d, &k)) {
                kept.push((p, k));
            }
        }
        kept.into_iter().map(|(p, _)| p).collect()
    }
}

/// The folders git ignores, per worktree (by its key), as paths from the
/// worktree's top with a `/` at the end.
#[derive(Default)]
pub struct Ignored(HashMap<String, HashSet<String>>);

/// Whether the change at `p` surely does not count, with no git asked: in
/// the git folder an object, a log or a lock; in a worktree a folder known to
/// be ignored. The watching thread keeps no more of those.
pub fn needless(places: &Places, ignored: &Ignored, p: &Path) -> bool {
    let k = paths::key(p);
    if let Some(rel) = k.strip_prefix(&places.common).filter(|_| paths::contains_key(&places.common, &k)) {
        let rel = rel.trim_start_matches('/');
        return !rel.eq_ignore_ascii_case("info/exclude") && !in_git(rel);
    }
    let Some((tree, rel)) = place(places, p, &k) else { return false };
    ignored.0.get(&places.trees[tree].1).is_some_and(|dirs| within(dirs, &rel))
}

/// Whether any of the changed `paths` could show in the window.
pub fn matters(places: &Places, changed: &[PathBuf], ignored: &mut Ignored) -> bool {
    if changed.len() > MOST {
        return true;
    }
    // The paths to ask git about, per worktree.
    let mut ask: HashMap<usize, Vec<String>> = HashMap::new();
    for p in changed {
        let k = paths::key(p);
        if let Some(rel) = k.strip_prefix(&places.common).filter(|_| paths::contains_key(&places.common, &k)) {
            let rel = rel.trim_start_matches('/');
            if rel.eq_ignore_ascii_case("info/exclude") {
                ignored.0.clear();
                return true;
            }
            if in_git(rel) {
                return true;
            }
            continue;
        }
        let Some((tree, rel)) = place(places, p, &k) else { return true };
        let first = rel.split('/').next().unwrap_or("");
        if rel.is_empty() {
            return true;
        }
        // A linked worktree's `.git` file; a main worktree's git folder is
        // the common one.
        if first.eq_ignore_ascii_case(".git") {
            continue;
        }
        if rel.rsplit('/').next().is_some_and(|n| n.eq_ignore_ascii_case(".gitignore")) {
            ignored.0.remove(&places.trees[tree].1);
            return true;
        }
        if ignored.0.get(&places.trees[tree].1).is_some_and(|dirs| within(dirs, &rel)) {
            continue;
        }
        ask.entry(tree).or_default().push(rel);
    }
    for (tree, rels) in ask {
        let (path, key) = &places.trees[tree];
        // The folders on the way too: the next change in one git ignores
        // needs no asking.
        let mut queries: Vec<String> = Vec::new();
        let mut once = HashSet::new();
        for rel in &rels {
            for (i, _) in rel.match_indices('/') {
                let dir = &rel[..=i];
                if once.insert(dir.to_owned()) {
                    queries.push(dir.to_owned());
                }
            }
            if once.insert(rel.clone()) {
                queries.push(rel.clone());
            }
        }
        let Ok(out) = check_ignore(path, &queries) else { return true };
        let dirs = ignored.0.entry(key.clone()).or_default();
        dirs.extend(out.iter().filter(|q| q.ends_with('/')).cloned());
        if rels.iter().any(|r| !out.contains(r) && !within(dirs, r)) {
            return true;
        }
    }
    false
}

/// The worktree `p` (whose key is `k`) is in, by its index in `places`, and
/// the path from its top, `/` between the parts.
fn place(places: &Places, p: &Path, k: &str) -> Option<(usize, String)> {
    let tree = places.trees.iter().position(|(_, t)| paths::contains_key(t, k))?;
    // The path as it was written when it can be had: git's patterns can
    // count case.
    let rel = match p.strip_prefix(&places.trees[tree].0) {
        Ok(r) => r.to_string_lossy().replace('\\', "/"),
        Err(_) => k[places.trees[tree].1.len()..].to_owned(),
    };
    Some((tree, rel.trim_matches('/').to_owned()))
}

/// Whether `rel` is inside one of the folders `dirs`.
fn within(dirs: &HashSet<String>, rel: &str) -> bool {
    rel.match_indices('/').any(|(i, _)| dirs.contains(&rel[..=i]))
}

/// Whether the change at `rel` in the common git folder counts.
fn in_git(rel: &str) -> bool {
    let mut parts: Vec<&str> = rel.split('/').filter(|s| !s.is_empty()).collect();
    // A linked worktree's own folder: the same as the common one's top.
    if parts.first().is_some_and(|p| p.eq_ignore_ascii_case("worktrees")) {
        if parts.len() <= 2 {
            return true;
        }
        parts.drain(..2);
    }
    let Some(name) = parts.last() else { return true };
    if name.to_ascii_lowercase().ends_with(".lock") {
        return false;
    }
    const NEVER: [&str; 6] = ["objects", "logs", "hooks", "lfs", "rr-cache", "fsmonitor--daemon"];
    if parts.iter().any(|p| NEVER.iter().any(|n| p.eq_ignore_ascii_case(n))) {
        return false;
    }
    const NOT: [&str; 7] = ["FETCH_HEAD", "ORIG_HEAD", "COMMIT_EDITMSG", "gc.log", "gc.pid", "description", "fsmonitor--daemon.ipc"];
    !(parts.len() == 1 && NOT.iter().any(|n| name.eq_ignore_ascii_case(n)))
}

/// Which of `queries` (paths from the top of the worktree `dir`, folders
/// with a `/` at the end) git ignores.
fn check_ignore(dir: &Path, queries: &[String]) -> Result<HashSet<String>, git::Error> {
    let mut child = git::command()
        .arg("-C")
        .arg(dir)
        .args(["check-ignore", "--stdin", "-z"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| git::Error::Io(e.to_string()))?;
    let mut input = Vec::new();
    for q in queries {
        input.extend_from_slice(q.as_bytes());
        input.push(0);
    }
    let stdin = child.stdin.take();
    // Written from another thread: git answers as it reads, and a full pipe
    // the other way would hold both up.
    let out = std::thread::scope(|s| {
        s.spawn(move || {
            if let Some(mut w) = stdin {
                let _ = w.write_all(&input);
            }
        });
        child.wait_with_output()
    })
    .map_err(|e| git::Error::Io(e.to_string()))?;
    // 1: none of them is ignored.
    match out.status.code() {
        Some(0) | Some(1) => {}
        _ => return Err(git::Error::Git(String::from_utf8_lossy(&out.stderr).trim().to_owned())),
    }
    Ok(out.stdout.split(|&b| b == 0).filter(|s| !s.is_empty()).map(|s| String::from_utf8_lossy(s).into_owned()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn the_git_folder_counts_its_refs_and_not_its_objects() {
        for yes in
            ["HEAD", "index", "refs/heads/feat", "packed-refs", "config", "worktrees", "worktrees/wt", "worktrees/wt/HEAD", "MERGE_HEAD"]
        {
            assert!(in_git(yes), "{yes}");
        }
        for no in [
            "objects/ab/cdef",
            "logs/HEAD",
            "index.lock",
            "refs/heads/feat.lock",
            "FETCH_HEAD",
            "fetch_head",
            "worktrees/wt/logs/HEAD",
            "worktrees/wt/index.lock",
            "worktrees/wt/ORIG_HEAD",
            "modules/sub/objects/pack/x",
        ] {
            assert!(!in_git(no), "{no}");
        }
    }

    #[test]
    fn the_roots_leave_out_a_folder_inside_another() {
        let common = Path::new("/r/repo/.git");
        let trees = [PathBuf::from("/r/repo"), PathBuf::from("/r/repo/.wt/feat"), PathBuf::from("/r/other")];
        let places = Places::new(common, &trees);
        assert_eq!(places.roots(common), [PathBuf::from("/r/repo"), PathBuf::from("/r/other")]);
        // A bare repository's folder is watched by itself.
        let common = Path::new("/r/bare.git");
        assert_eq!(Places::new(common, &[PathBuf::from("/r/wt")]).roots(common), [PathBuf::from("/r/wt"), PathBuf::from("/r/bare.git")]);
    }

    #[test]
    fn ignored_files_do_not_count() {
        let r = TempRepo::new("change");
        r.write(&r.repo, ".gitignore", "/target/\n*.log\n");
        r.write(&r.repo, "target/debug/x.o", "");
        r.write(&r.repo, "src/a.rs", "");
        r.git(&r.repo, &["worktree", "add", "-q", "-b", "feat", "../wt"]);
        let wt = r.root.join("wt");
        let common = r.repo.join(".git");
        let places = Places::new(&common, &[r.repo.clone(), wt.clone()]);
        let mut ig = Ignored::default();
        let m = |ig: &mut Ignored, rels: &[&str]| matters(&places, &rels.iter().map(|p| r.repo.join(p)).collect::<Vec<_>>(), ig);
        assert!(!m(&mut ig, &["target/debug/x.o", "a.log"]));
        // `target/` is known now: no git is asked.
        assert!(needless(&places, &ig, &r.repo.join("target/release/y")));
        assert!(needless(&places, &ig, &r.repo.join(".git/objects/ab/cd")));
        assert!(!needless(&places, &ig, &r.repo.join("src/a.rs")));
        assert!(!m(&mut ig, &["target/release/y"]));
        assert!(m(&mut ig, &["target/debug/x.o", "src/a.rs"]));
        assert!(!m(&mut ig, &[".git/objects/ab/cd", ".git/index.lock"]));
        assert!(m(&mut ig, &[".git/refs/heads/feat"]));
        assert!(m(&mut ig, &[".git/worktrees/wt/HEAD"]));
        // The linked worktree's own `.git` file does not count.
        assert!(!matters(&places, &[wt.join(".git")], &mut ig));
        assert!(matters(&places, &[wt.join("src/b.rs")], &mut ig));
        // A `.gitignore` changed: what was known goes.
        assert!(m(&mut ig, &[".gitignore"]));
        assert!(!needless(&places, &ig, &r.repo.join("target/release/y")));
        // Somewhere not watched: taken as a change.
        assert!(matters(&places, &[r.root.join("elsewhere")], &mut ig));
        assert!(matters(&places, &vec![r.repo.join("a.log"); MOST + 1], &mut ig));
    }
}
