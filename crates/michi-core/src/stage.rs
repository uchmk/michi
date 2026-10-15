//! The `2` view's changes (docs/design.md: 変更): what a worktree has
//! staged and what it has not, one file's diff of either side, and the
//! writes: a file or all of them in or out of the index, chosen lines and
//! hunks by a patch to `git apply --cached`, and the commit.
//!
//! The reads go as every read does, without the index lock. The writes take
//! it: a Claude in the same worktree may find it taken for a moment, and
//! one of ours may fail for its; the error says so and nothing is half done.

use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;

use crate::git;
use crate::oid::Oid;
use crate::show::{self, File, Hunk, Kind, Line, Status, LIMITS};

/// The most untracked files listed.
pub const UNTRACKED_MAX: usize = 1000;

/// A worktree's changes, as the `2` view lists them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Lists {
    /// In the index against the head (against nothing before the first
    /// commit), renames found.
    pub staged: Vec<File>,
    /// In the worktree against the index, then the untracked files.
    pub unstaged: Vec<File>,
    /// There were more untracked files than [`UNTRACKED_MAX`].
    pub cut: bool,
    /// What the worktree has checked out; `None` before the first commit.
    pub head: Option<Oid>,
}

impl Lists {
    /// The files of `side`.
    pub fn of(&self, side: Side) -> &[File] {
        match side {
            Side::Staged => &self.staged,
            Side::Unstaged => &self.unstaged,
        }
    }
}

/// Which list a file is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Staged,
    Unstaged,
}

/// The changes of the worktree at `dir`.
pub fn lists(dir: &Path) -> Result<Lists, git::Error> {
    let ((staged, unstaged), (untracked, head)) = git::both(
        || git::both(|| diff_files(dir, Side::Staged), || diff_files(dir, Side::Unstaged)),
        || git::both(|| untracked(dir), || git::read_maybe(dir, &["rev-parse", "-q", "--verify", "HEAD^{commit}"])),
    );
    let (mut unstaged, (more, cut)) = (unstaged?, untracked?);
    unstaged.extend(more);
    let head = head?.and_then(|out| Oid::parse(String::from_utf8_lossy(&out).trim()));
    Ok(Lists { staged: staged?, unstaged, cut, head })
}

/// `git diff` of one side, with its options; `paths` limit it.
fn diff_command(dir: &Path, side: Side, patch: bool, paths: &[&str]) -> Command {
    let mut c = git::command();
    c.env("GIT_LITERAL_PATHSPECS", "1");
    c.arg("-C").arg(dir).args(["-c", "core.quotePath=false", "-c", "diff.suppressBlankEmpty=false", "diff"]);
    if side == Side::Staged {
        c.args(["--cached", "-M"]);
    } else {
        c.arg("--no-renames");
    }
    c.args(["-z", "--raw", "--no-abbrev", "--numstat", "--no-color", "--no-ext-diff", "--no-textconv", "--no-relative"]);
    c.args(["--src-prefix=a/", "--dst-prefix=b/"]);
    if patch {
        c.arg("--patch");
    }
    if !paths.is_empty() {
        c.arg("--").args(paths);
    }
    c
}

/// The files of one side, without their diffs.
fn diff_files(dir: &Path, side: Side) -> Result<Vec<File>, git::Error> {
    let out = run(diff_command(dir, side, false, &[]))?;
    let (changes, _) = show::parse_changes(&out);
    let mut files = changes.files;
    // An unmerged path comes as `U` and again against our side.
    let unmerged: Vec<String> = files.iter().filter(|f| f.status == Status::Unmerged).map(|f| f.path.clone()).collect();
    files.retain(|f| f.status == Status::Unmerged || !unmerged.contains(&f.path));
    Ok(files)
}

fn run(mut c: Command) -> Result<Vec<u8>, git::Error> {
    let out = c.output().map_err(git::started)?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(git::refused(&String::from_utf8_lossy(&out.stderr)))
    }
}

/// The untracked files that are not ignored, with their lines counted, and
/// whether there were more than [`UNTRACKED_MAX`].
fn untracked(dir: &Path) -> Result<(Vec<File>, bool), git::Error> {
    let out = git::read(dir, &["ls-files", "-z", "--others", "--exclude-standard", "--full-name"])?;
    let mut paths: Vec<&[u8]> = out.split(|&b| b == 0).filter(|p| !p.is_empty()).collect();
    let cut = paths.len() > UNTRACKED_MAX;
    paths.truncate(UNTRACKED_MAX);
    let files = paths
        .into_iter()
        .map(|p| {
            let path = String::from_utf8_lossy(p).into_owned();
            let counts = count_lines(&dir.join(&path)).map(|n| (n, 0));
            File {
                status: Status::Untracked,
                path,
                from: None,
                blob: None,
                counts,
                hunks: Vec::new(),
                eol_only: false,
                cut: false,
                not_utf8: false,
            }
        })
        .collect();
    Ok((files, cut))
}

/// Bytes looked at for a NUL, as git does, to tell a binary file.
const SNIFF: usize = 8000;

/// The lines of the file at `path`; `None` when it is binary or cannot be
/// read. A link is one line, its target, as git keeps it.
fn count_lines(path: &Path) -> Option<u32> {
    if fs::symlink_metadata(path).ok()?.file_type().is_symlink() {
        return Some(1);
    }
    let mut f = fs::File::open(path).ok()?;
    let mut buf = vec![0; 1 << 16];
    let (mut lines, mut seen, mut last) = (0u32, 0usize, b'\n');
    loop {
        let n = f.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        let part = &buf[..n];
        if seen < SNIFF && part[..n.min(SNIFF - seen)].contains(&0) {
            return None;
        }
        seen += n;
        lines = lines.saturating_add(part.iter().filter(|&&b| b == b'\n').count() as u32);
        last = part[n - 1];
    }
    Some(if last == b'\n' { lines } else { lines + 1 })
}

/// One file's diff on one side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diff {
    pub file: File,
    /// What its lines can be staged or unstaged from; `None` when it goes
    /// in or out whole (new, deleted, renamed, binary, cut short, a
    /// submodule).
    pub patch: Option<Patch>,
}

/// The diff of `file` on `side` of the worktree at `dir`; `None` when it is
/// no longer there.
pub fn diff(dir: &Path, side: Side, file: &File) -> Result<Option<Diff>, git::Error> {
    if file.status == Status::Untracked {
        return Ok(Some(Diff { file: untracked_diff(dir, file), patch: None }));
    }
    let mut paths = vec![file.path.as_str()];
    if let Some(from) = &file.from {
        paths.push(from);
    }
    let mut git = git::Running::start(diff_command(dir, side, true, &paths), "diff")?;
    let mut out = Vec::new();
    (&mut git.out).take(LIMITS.bytes as u64 + 1).read_to_end(&mut out).map_err(|e| git.io(e))?;
    // Past the most read, git is stopped as it is dropped.
    let whole = out.len() <= LIMITS.bytes;
    if whole {
        git.finish()?;
    }
    drop(git);
    let (changes, patch) = show::parse_changes(&out);
    let Some(mut f) = changes.files.into_iter().find(|f| f.path == file.path) else { return Ok(None) };
    if !whole {
        f.cut = true;
    }
    let patch = (f.status == Status::Modified && !f.is_binary() && !f.cut).then(|| Patch::parse(patch)).filter(|p| !p.submodule);
    Ok(Some(Diff { file: f, patch }))
}

/// An untracked file as all added, from its content.
fn untracked_diff(dir: &Path, file: &File) -> File {
    let mut f = File { counts: None, ..file.clone() };
    let mut bytes = Vec::new();
    let read = fs::File::open(dir.join(&file.path)).and_then(|h| h.take(LIMITS.bytes as u64 + 1).read_to_end(&mut bytes));
    if read.is_err() || bytes[..bytes.len().min(SNIFF)].contains(&0) {
        return f;
    }
    if bytes.len() > LIMITS.bytes {
        bytes.truncate(LIMITS.bytes);
        f.cut = true;
    }
    let mut lines = Vec::new();
    let mut n = 0u32;
    for raw in bytes.split_inclusive(|&b| b == b'\n') {
        n += 1;
        if lines.len() >= LIMITS.file_lines {
            f.cut = true;
            continue;
        }
        let line = raw.strip_suffix(b"\n").unwrap_or(raw);
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        f.not_utf8 |= !show::is_utf8(line, false);
        lines.push(Line { kind: Kind::Add, old: 0, new: n, text: show::text(line, &LIMITS), marks: Vec::new() });
    }
    f.counts = Some((file.counts.map_or(n, |(a, _)| a), 0));
    if !lines.is_empty() {
        f.hunks.push(Hunk { header: format!("@@ -0,0 +1,{n} @@"), lines });
    }
    f
}

/// A file's patch as git wrote it, kept as bytes, its hunks and lines in
/// the order the diff shows them: what chosen lines are built from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Patch {
    /// `diff --git`, `---` and `+++`, a line each.
    head: Vec<u8>,
    hunks: Vec<RawHunk>,
    /// It is a submodule's: its one line is a commit.
    submodule: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RawHunk {
    header: Vec<u8>,
    lines: Vec<RawLine>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RawLine {
    /// The line with its mark (` `, `-`, `+`), a `\r` kept, the line break
    /// left off.
    bytes: Vec<u8>,
    /// `\ No newline at end of file` follows it.
    no_eol: bool,
}

impl Patch {
    /// From one file's section of a patch, read as show.rs reads it so the
    /// hunks and lines are those of [`File::hunks`]. The modes are left
    /// out: a line taken in does not take a mode change with it.
    fn parse(patch: &[u8]) -> Patch {
        let mut p = Patch::default();
        let (mut left_old, mut left_new) = (0u32, 0u32);
        let mut sections = 0;
        for raw in patch.split_inclusive(|&b| b == b'\n') {
            let full = raw.strip_suffix(b"\n").unwrap_or(raw);
            let line = full.strip_suffix(b"\r").unwrap_or(full);
            if left_old > 0 || left_new > 0 {
                let kind = match line.first() {
                    Some(b' ') | None => Kind::Context,
                    Some(b'-') => Kind::Del,
                    Some(b'+') => Kind::Add,
                    Some(b'\\') => {
                        p.no_eol();
                        continue;
                    }
                    Some(_) => {
                        left_old = 0;
                        left_new = 0;
                        continue;
                    }
                };
                if kind != Kind::Add {
                    left_old = left_old.saturating_sub(1);
                }
                if kind != Kind::Del {
                    left_new = left_new.saturating_sub(1);
                }
                let bytes = if full.is_empty() { b" ".to_vec() } else { full.to_vec() };
                if let Some(h) = p.hunks.last_mut() {
                    h.lines.push(RawLine { bytes, no_eol: false });
                }
            } else if line.starts_with(b"diff --git ") {
                sections += 1;
                if sections > 1 {
                    break;
                }
                p.head.extend_from_slice(raw);
            } else if line.starts_with(b"@@ ") {
                let Some((_, lo, _, ln)) = show::hunk_range(&String::from_utf8_lossy(line)) else { continue };
                (left_old, left_new) = (lo, ln);
                p.hunks.push(RawHunk { header: full.to_vec(), lines: Vec::new() });
            } else if line.starts_with(b"\\") {
                p.no_eol();
            } else if p.hunks.is_empty() && (line.starts_with(b"--- ") || line.starts_with(b"+++ ")) {
                p.head.extend_from_slice(raw);
            } else if p.hunks.is_empty() && line.starts_with(b"index ") && line.ends_with(b" 160000") {
                p.submodule = true;
            }
        }
        p
    }

    /// The last line read has no line break after it.
    fn no_eol(&mut self) {
        if let Some(l) = self.hunks.last_mut().and_then(|h| h.lines.last_mut()) {
            l.no_eol = true;
        }
    }

    /// The patch of the lines `picked(hunk, line)` says, for
    /// [`apply`]: to stage them when not `reverse` (the patch is the
    /// worktree's against the index), to unstage them when `reverse` (the
    /// index's against the head). A context line is never picked; `None`
    /// when no added or deleted line is. The hunks keep where they start
    /// and leave the counting to `--recount`.
    pub fn pick(&self, picked: impl Fn(usize, usize) -> bool, reverse: bool) -> Option<Vec<u8>> {
        let mut out = self.head.clone();
        let mut any = false;
        for (h, hunk) in self.hunks.iter().enumerate() {
            let changed = |l: &RawLine| matches!(l.bytes.first(), Some(b'+' | b'-'));
            if !hunk.lines.iter().enumerate().any(|(k, l)| changed(l) && picked(h, k)) {
                continue;
            }
            any = true;
            out.extend_from_slice(&hunk.header);
            out.push(b'\n');
            for (k, l) in hunk.lines.iter().enumerate() {
                let mark = l.bytes.first().copied().unwrap_or(b' ');
                let keep = mark == b' ' || picked(h, k);
                // The side the patch is put on stays as it is: a line left
                // out of it is kept as context, a line not there is dropped.
                let kept_side = if reverse { b'+' } else { b'-' };
                if keep {
                    out.extend_from_slice(&l.bytes);
                } else if mark == kept_side {
                    out.push(b' ');
                    out.extend_from_slice(&l.bytes[1..]);
                } else {
                    continue;
                }
                out.push(b'\n');
                if l.no_eol {
                    out.extend_from_slice(b"\\ No newline at end of file\n");
                }
            }
        }
        any.then_some(out)
    }

    /// The lines of hunk `h` that are added or deleted.
    pub fn changed_in(&self, h: usize) -> Vec<usize> {
        let Some(hunk) = self.hunks.get(h) else { return Vec::new() };
        hunk.lines.iter().enumerate().filter(|(_, l)| matches!(l.bytes.first(), Some(b'+' | b'-'))).map(|(k, _)| k).collect()
    }
}

/// `git apply --cached` of `patch` (from [`Patch::pick`]) in the worktree
/// at `dir`, the other way when `reverse`.
pub fn apply(dir: &Path, patch: &[u8], reverse: bool) -> Result<(), git::Error> {
    let mut c = git::command();
    c.arg("-C").arg(dir).args(["apply", "--cached", "--recount", "--whitespace=nowarn"]);
    if reverse {
        c.arg("--reverse");
    }
    c.arg("-");
    git::feed(c, patch).map(drop)
}

/// The files at `paths` into the index as they are in the worktree (a
/// deleted one out of it); all of them when there are none.
pub fn stage(dir: &Path, paths: &[&str]) -> Result<(), git::Error> {
    let mut c = git::command();
    c.env("GIT_LITERAL_PATHSPECS", "1").arg("-C").arg(dir).args(["add", "-A"]);
    if !paths.is_empty() {
        c.arg("--").args(paths);
    }
    run(c).map(drop)
}

/// The files at `paths` back in the index as the head has them (out of it
/// before the first commit); all of them when there are none. A rename
/// takes both its paths.
pub fn unstage(dir: &Path, paths: &[&str]) -> Result<(), git::Error> {
    let mut c = git::command();
    c.env("GIT_LITERAL_PATHSPECS", "1").arg("-C").arg(dir).args(["reset", "-q"]);
    if !paths.is_empty() {
        c.arg("--").args(paths);
    }
    run(c).map(drop)
}

/// Commits what is staged in the worktree at `dir` with `message` (its
/// blank lines at the ends and the spaces at the ends of its lines taken
/// off, nothing else), in place of the head when `amend`; the hooks run.
/// The new head.
pub fn commit(dir: &Path, message: &str, amend: bool) -> Result<Oid, git::Error> {
    let mut c = git::command();
    c.arg("-C").arg(dir).args(["commit", "-q", "--cleanup=whitespace"]);
    if amend {
        c.arg("--amend");
    }
    c.args(["-F", "-"]);
    git::feed(c, message.as_bytes())?;
    let id = git::read_text(dir, &["rev-parse", "--verify", "HEAD^{commit}"])?;
    Oid::parse(&id).ok_or_else(|| git::Error::Io(format!("git rev-parse: {id}")))
}

/// The head's message, to amend it.
pub fn last_message(dir: &Path) -> Result<String, git::Error> {
    let out = git::read(dir, &["log", "-1", "--no-show-signature", "--format=%B", "HEAD"])?;
    Ok(String::from_utf8_lossy(&out).trim_end().to_owned())
}

/// Whether a remote's branch has the head: amending it then rewrites what
/// others may have.
pub fn is_pushed(dir: &Path) -> Result<bool, git::Error> {
    let out = git::read(dir, &["for-each-ref", "--count=1", "--format=%(refname)", "--contains", "HEAD", "refs/remotes/"])?;
    Ok(!out.trim_ascii().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    /// A repository whose own settings beat the machine's: the calls here
    /// run git as michi does, with the user's configuration.
    fn repo(name: &str) -> TempRepo {
        let t = TempRepo::new(name);
        for (k, v) in [
            ("user.name", "michi"),
            ("user.email", "michi@example.invalid"),
            ("commit.gpgsign", "false"),
            ("core.autocrlf", "false"),
            ("core.safecrlf", "false"),
            ("core.hooksPath", "no-hooks"),
            ("core.excludesFile", "no-excludes"),
            ("apply.whitespace", "nowarn"),
        ] {
            t.git(&t.repo, &["config", k, v]);
        }
        t
    }

    fn paths(files: &[File]) -> Vec<(char, &str)> {
        files.iter().map(|f| (f.status.letter(), f.path.as_str())).collect()
    }

    /// What the index has of `path`.
    fn staged(t: &TempRepo, path: &str) -> String {
        t.out(&t.repo, &["show", &format!(":{path}")])
    }

    const TEN: &str = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12\n13\n14\n15\n16\n";

    #[test]
    fn the_lists_have_each_side_and_the_untracked_files_last() {
        let t = repo("stage-lists");
        t.write(&t.repo, "a.txt", "a\n");
        t.write(&t.repo, "b.txt", "b\n");
        t.write(&t.repo, "gone.txt", "x\n");
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "files"]);
        t.write(&t.repo, "a.txt", "a\nmore\n");
        t.git(&t.repo, &["add", "a.txt"]);
        t.write(&t.repo, "a.txt", "a\nmore\nand more\n");
        t.write(&t.repo, "b.txt", "B\n");
        fs::remove_file(t.repo.join("gone.txt")).unwrap();
        t.write(&t.repo, "new/one.txt", "1\n2\n3");
        t.write(&t.repo, "bin.dat", "a\0b");
        t.write(&t.repo, ".gitignore", "*.log\n");
        t.write(&t.repo, "skip.log", "x\n");
        let l = lists(&t.repo).unwrap();
        assert_eq!(paths(&l.staged), [('M', "a.txt")]);
        assert_eq!(l.staged[0].counts, Some((1, 0)));
        assert_eq!(
            paths(&l.unstaged),
            [('M', "a.txt"), ('M', "b.txt"), ('D', "gone.txt"), ('?', ".gitignore"), ('?', "bin.dat"), ('?', "new/one.txt")]
        );
        assert_eq!(l.unstaged[1].counts, Some((1, 1)));
        assert_eq!(l.unstaged[4].counts, None);
        assert_eq!(l.unstaged[5].counts, Some((3, 0)));
        assert!(!l.cut);
        assert_eq!(l.head.map(|h| h.to_string()), Some(t.out(&t.repo, &["rev-parse", "HEAD"])));
        assert_eq!(l.of(Side::Staged).len(), 1);
    }

    #[test]
    fn before_the_first_commit_everything_staged_is_added() {
        let t = TempRepo::new("stage-unborn");
        t.git(&t.repo, &["checkout", "-q", "--orphan", "fresh"]);
        t.git(&t.repo, &["rm", "-rq", "--cached", "--ignore-unmatch", "."]);
        t.write(&t.repo, "a.txt", "a\n");
        t.write(&t.repo, "b.txt", "b\n");
        stage(&t.repo, &[]).unwrap();
        let l = lists(&t.repo).unwrap();
        assert_eq!(paths(&l.staged), [('A', "a.txt"), ('A', "b.txt")]);
        assert_eq!(l.head, None);
        unstage(&t.repo, &["b.txt"]).unwrap();
        let l = lists(&t.repo).unwrap();
        assert_eq!(paths(&l.staged), [('A', "a.txt")]);
        assert_eq!(paths(&l.unstaged), [('?', "b.txt")]);
        unstage(&t.repo, &[]).unwrap();
        assert!(lists(&t.repo).unwrap().staged.is_empty());
    }

    #[test]
    fn files_go_in_and_out_whole() {
        let t = repo("stage-whole");
        t.write(&t.repo, "a.txt", "a\n");
        t.write(&t.repo, "old name.txt", "keep me\nas I am\n");
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "files"]);
        t.write(&t.repo, "a.txt", "A\n");
        t.git(&t.repo, &["mv", "old name.txt", "new name.txt"]);
        t.write(&t.repo, "[x].txt", "x\n");
        stage(&t.repo, &["a.txt", "[x].txt"]).unwrap();
        let l = lists(&t.repo).unwrap();
        assert_eq!(paths(&l.staged), [('A', "[x].txt"), ('M', "a.txt"), ('R', "new name.txt")]);
        assert_eq!(l.staged[2].from.as_deref(), Some("old name.txt"));
        unstage(&t.repo, &["new name.txt", "old name.txt"]).unwrap();
        let l = lists(&t.repo).unwrap();
        assert_eq!(paths(&l.staged), [('A', "[x].txt"), ('M', "a.txt")]);
        assert_eq!(paths(&l.unstaged), [('D', "old name.txt"), ('?', "new name.txt")]);
        stage(&t.repo, &[]).unwrap();
        let l = lists(&t.repo).unwrap();
        assert_eq!(paths(&l.staged), [('A', "[x].txt"), ('M', "a.txt"), ('R', "new name.txt")]);
        assert!(l.unstaged.is_empty());
        unstage(&t.repo, &[]).unwrap();
        assert!(lists(&t.repo).unwrap().staged.is_empty());
    }

    /// The diff of `path` on `side`, which must be there.
    fn diff_of(t: &TempRepo, side: Side, path: &str) -> Diff {
        let l = lists(&t.repo).unwrap();
        let f = l.of(side).iter().find(|f| f.path == path).unwrap();
        diff(&t.repo, side, f).unwrap().unwrap()
    }

    /// The (hunk, line) of each line whose text is in `texts`.
    fn lines_of(d: &Diff, kind: Kind, texts: &[&str]) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for (h, hunk) in d.file.hunks.iter().enumerate() {
            for (k, l) in hunk.lines.iter().enumerate() {
                if l.kind == kind && texts.contains(&l.text.as_str()) {
                    out.push((h, k));
                }
            }
        }
        assert_eq!(out.len(), texts.len(), "{texts:?}");
        out
    }

    fn pick(d: &Diff, chosen: &[(usize, usize)], reverse: bool) -> Vec<u8> {
        d.patch.as_ref().unwrap().pick(|h, k| chosen.contains(&(h, k)), reverse).unwrap()
    }

    #[test]
    fn chosen_lines_are_staged_and_the_rest_stay_out() {
        let t = repo("stage-lines");
        t.write(&t.repo, "n.txt", TEN);
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "numbers"]);
        // Two hunks: 2 becomes two, 3 goes; 14 becomes fourteen, x after 16.
        let changed = TEN.replace("2\n3\n", "two\n").replace("14\n", "fourteen\n") + "x\n";
        t.write(&t.repo, "n.txt", &changed);
        let d = diff_of(&t, Side::Unstaged, "n.txt");
        assert_eq!(d.file.hunks.len(), 2);
        // Only `two` in, and `14` out: 2 and 3 stay, before it as the diff
        // has them, and 14 goes.
        let mut chosen = lines_of(&d, Kind::Add, &["two"]);
        chosen.extend(lines_of(&d, Kind::Del, &["14"]));
        apply(&t.repo, &pick(&d, &chosen, false), false).unwrap();
        assert_eq!(staged(&t, "n.txt") + "\n", TEN.replacen("1\n2\n3\n", "1\n2\n3\ntwo\n", 1).replace("\n14\n", "\n"));
        // The rest, hunk by hunk, as the view does it.
        let d = diff_of(&t, Side::Unstaged, "n.txt");
        let p = d.patch.as_ref().unwrap();
        let all: Vec<(usize, usize)> = (0..d.file.hunks.len()).flat_map(|h| p.changed_in(h).into_iter().map(move |k| (h, k))).collect();
        apply(&t.repo, &pick(&d, &all, false), false).unwrap();
        assert_eq!(staged(&t, "n.txt") + "\n", changed);
        assert!(lists(&t.repo).unwrap().unstaged.is_empty());
    }

    #[test]
    fn chosen_lines_are_unstaged_and_the_rest_stay_in() {
        let t = repo("unstage-lines");
        t.write(&t.repo, "n.txt", TEN);
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "numbers"]);
        let changed = TEN.replace("2\n3\n", "two\n").replace("14\n", "fourteen\n");
        t.write(&t.repo, "n.txt", &changed);
        stage(&t.repo, &["n.txt"]).unwrap();
        let d = diff_of(&t, Side::Staged, "n.txt");
        // `fourteen` out of the index again (14 still deleted there), and
        // `3` back in it, before `two` as the diff has them.
        let mut chosen = lines_of(&d, Kind::Add, &["fourteen"]);
        chosen.extend(lines_of(&d, Kind::Del, &["3"]));
        apply(&t.repo, &pick(&d, &chosen, true), true).unwrap();
        assert_eq!(staged(&t, "n.txt") + "\n", TEN.replacen("1\n2\n3\n", "1\n3\ntwo\n", 1).replace("\n14\n", "\n"));
        // What the worktree has did not move.
        assert_eq!(fs::read_to_string(t.repo.join("n.txt")).unwrap(), changed);
    }

    #[test]
    fn line_endings_and_a_last_line_without_a_break_survive() {
        let t = repo("stage-eol");
        t.write(&t.repo, "crlf.txt", "a\r\nb\r\nc\r\n");
        t.write(&t.repo, "end.txt", "a\nb");
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "files"]);
        t.write(&t.repo, "crlf.txt", "a\r\nB\r\nc\r\nd\r\n");
        t.write(&t.repo, "end.txt", "a\nB");
        let d = diff_of(&t, Side::Unstaged, "crlf.txt");
        let mut chosen = lines_of(&d, Kind::Add, &["B"]);
        chosen.extend(lines_of(&d, Kind::Del, &["b"]));
        apply(&t.repo, &pick(&d, &chosen, false), false).unwrap();
        assert_eq!(t.out(&t.repo, &["cat-file", "blob", ":crlf.txt"]).into_bytes(), b"a\r\nB\r\nc".to_vec());
        let d = diff_of(&t, Side::Unstaged, "end.txt");
        let mut chosen = lines_of(&d, Kind::Add, &["B"]);
        chosen.extend(lines_of(&d, Kind::Del, &["b"]));
        apply(&t.repo, &pick(&d, &chosen, false), false).unwrap();
        let blob = t.out(&t.repo, &["rev-parse", ":end.txt"]);
        assert_eq!(blob, t.out(&t.repo, &["hash-object", "end.txt"]));
    }

    #[test]
    fn nothing_picked_is_no_patch_and_new_files_go_whole() {
        let t = repo("stage-none");
        t.write(&t.repo, "a.txt", "a\n");
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "a"]);
        t.write(&t.repo, "a.txt", "b\n");
        t.write(&t.repo, "b.txt", "b\n");
        t.git(&t.repo, &["add", "b.txt"]);
        let d = diff_of(&t, Side::Unstaged, "a.txt");
        let p = d.patch.as_ref().unwrap();
        assert_eq!(p.pick(|_, _| false, false), None);
        // Context lines are never what makes a patch.
        assert_eq!(p.changed_in(0), [0, 1]);
        assert_eq!(p.changed_in(5), Vec::<usize>::new());
        let d = diff_of(&t, Side::Staged, "b.txt");
        assert_eq!(d.file.status, Status::Added);
        assert_eq!(d.patch, None);
        // A file that is no longer changed.
        let gone = File { path: "nope.txt".into(), ..d.file.clone() };
        assert_eq!(diff(&t.repo, Side::Staged, &gone).unwrap(), None);
    }

    #[test]
    fn an_untracked_file_shows_as_all_added() {
        let t = repo("stage-untracked");
        t.write(&t.repo, "u.txt", "one\r\ntwo\nthree");
        t.write(&t.repo, "b.bin", "x\0y");
        let d = diff_of(&t, Side::Unstaged, "u.txt");
        assert_eq!(d.file.counts, Some((3, 0)));
        assert_eq!(d.patch, None);
        let h = &d.file.hunks[0];
        assert_eq!(h.header, "@@ -0,0 +1,3 @@");
        let texts: Vec<(&str, u32)> = h.lines.iter().map(|l| (l.text.as_str(), l.new)).collect();
        assert_eq!(texts, [("one", 1), ("two", 2), ("three", 3)]);
        assert!(h.lines.iter().all(|l| l.kind == Kind::Add));
        let d = diff_of(&t, Side::Unstaged, "b.bin");
        assert!(d.file.is_binary());
        assert!(d.file.hunks.is_empty());
    }

    #[test]
    fn a_commit_and_its_amend() {
        let t = repo("stage-commit");
        t.write(&t.repo, "a.txt", "a\n");
        stage(&t.repo, &[]).unwrap();
        let id = commit(&t.repo, "\n  Add a  \n\n# not a comment\n\n", false).unwrap();
        assert_eq!(id.to_string(), t.out(&t.repo, &["rev-parse", "HEAD"]));
        assert_eq!(last_message(&t.repo).unwrap(), "  Add a\n\n# not a comment");
        assert!(!is_pushed(&t.repo).unwrap());
        t.git(&t.repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
        assert!(is_pushed(&t.repo).unwrap());
        t.write(&t.repo, "b.txt", "b\n");
        stage(&t.repo, &["b.txt"]).unwrap();
        let amended = commit(&t.repo, "Add a and b", true).unwrap();
        assert_ne!(amended, id);
        assert_eq!(last_message(&t.repo).unwrap(), "Add a and b");
        assert_eq!(t.out(&t.repo, &["rev-list", "--count", "HEAD"]), "2");
        assert!(!is_pushed(&t.repo).unwrap());
        // Nothing staged: git says so.
        let e = commit(&t.repo, "Nothing", false).unwrap_err();
        assert!(matches!(e, git::Error::Git(_)), "{e:?}");
    }

    #[test]
    fn lines_are_counted_and_binaries_told() {
        let t = repo("stage-count");
        t.write(&t.repo, "a", "");
        t.write(&t.repo, "b", "x");
        t.write(&t.repo, "c", "x\ny\n");
        t.write(&t.repo, "d", "x\0");
        let n = |p: &str| count_lines(&t.repo.join(p));
        assert_eq!((n("a"), n("b"), n("c"), n("d"), n("nope")), (Some(0), Some(1), Some(2), None, None));
    }
}
