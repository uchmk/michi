//! Adding an untracked file to the worktree's `.gitignore` (docs/design.md:
//! 変更の画面; issue #49). One line is appended: the path from the worktree's
//! root, anchored with a leading `/` and escaped so that git takes it
//! literally. Nothing is kept as a backup: the line is easy to take out again.

use std::fs;
use std::path::Path;

use crate::git;

/// What [`add`] did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Added {
    /// The line was appended to `.gitignore`.
    Added(String),
    /// The line was in `.gitignore` already (the file is untracked for
    /// another reason, such as a nested `.gitignore` re-including it).
    Already(String),
}

/// The `.gitignore` line for `path`, a path relative to the worktree's root:
/// anchored, with `/` as the separator and the characters git reads as a
/// pattern escaped. A folder (`path` ending in `/`) keeps its `/`.
pub fn pattern(path: &str) -> String {
    let path = path.replace('\\', "/");
    let path = path.trim_start_matches('/');
    let trimmed = path.trim_end_matches(' ');
    let mut out = String::from("/");
    // `!` and `#` only mean something at the start of a line, which the
    // `/` in front has taken: nothing to escape there.
    for ch in trimmed.chars() {
        if matches!(ch, '*' | '?' | '[' | ']') {
            out.push('\\');
        }
        out.push(ch);
    }
    // Trailing spaces are dropped by git unless escaped.
    for _ in 0..path.len() - trimmed.len() {
        out.push_str("\\ ");
    }
    out
}

/// Appends the line for the untracked `path` to `.gitignore` in the root of
/// the worktree at `wt`, making the file if there is none. A tracked file is
/// refused: ignoring it would change nothing.
pub fn add(wt: &Path, path: &str) -> Result<Added, git::Error> {
    let spec = format!(":(literal){}", path.trim_end_matches('/'));
    let tracked = git::read_text(wt, &["ls-files", "--cached", "--", &spec])?;
    if !tracked.is_empty() {
        return Err(git::Error::Git(format!("{path} is tracked: ignoring it changes nothing")));
    }
    let line = pattern(path);
    let file = wt.join(".gitignore");
    // Bytes, not text: a `.gitignore` that is not UTF-8 keeps every byte.
    let old = match fs::read(&file) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(git::Error::Io(format!(".gitignore: {e}"))),
    };
    let same = |l: &[u8]| l.strip_suffix(b"\r").unwrap_or(l) == line.as_bytes();
    if old.split(|b| *b == b'\n').any(same) {
        return Ok(Added::Already(line));
    }
    // The file's own line breaks, and a break before the new line if the
    // last one has none.
    let eol: &[u8] = if old.windows(2).any(|w| w == b"\r\n") { b"\r\n" } else { b"\n" };
    let mut new = old;
    if !new.is_empty() && !new.ends_with(b"\n") {
        new.extend_from_slice(eol);
    }
    new.extend_from_slice(line.as_bytes());
    new.extend_from_slice(eol);
    fs::write(&file, new).map_err(|e| git::Error::Io(format!(".gitignore: {e}")))?;
    Ok(Added::Added(line))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn the_pattern_is_anchored_and_escaped() {
        assert_eq!(pattern("a.txt"), "/a.txt");
        assert_eq!(pattern("dir/sub/a b.txt"), "/dir/sub/a b.txt");
        assert_eq!(pattern("dir\\a.txt"), "/dir/a.txt");
        assert_eq!(pattern("a[1]*.txt"), "/a\\[1\\]\\*.txt");
        assert_eq!(pattern("!x"), "/!x");
        assert_eq!(pattern("out/"), "/out/");
        assert_eq!(pattern("a "), "/a\\ ");
    }

    #[test]
    fn a_line_is_appended_and_git_ignores_the_file() {
        let t = TempRepo::new("ignore-add");
        t.write(&t.repo, "build/out.bin", "x");
        t.write(&t.repo, "sub/a[1].log", "x");
        t.write(&t.repo, "sub/a1.log", "x");
        t.write(&t.repo, "keep.txt", "x");
        assert_eq!(add(&t.repo, "build/out.bin"), Ok(Added::Added("/build/out.bin".into())));
        assert_eq!(add(&t.repo, "sub/a[1].log"), Ok(Added::Added("/sub/a\\[1\\].log".into())));
        assert_eq!(fs::read_to_string(t.repo.join(".gitignore")).unwrap(), "/build/out.bin\n/sub/a\\[1\\].log\n");
        let status = t.out(&t.repo, &["status", "--porcelain", "-uall"]);
        assert!(!status.contains("out.bin") && !status.contains("a[1].log"), "{status}");
        // The escaped brackets did not catch the sibling.
        assert!(status.contains("keep.txt") && status.contains("a1.log"), "{status}");
    }

    #[test]
    fn an_existing_file_keeps_its_line_breaks_and_a_line_is_not_doubled() {
        let t = TempRepo::new("ignore-eol");
        t.write(&t.repo, ".gitignore", "target\r\nold.log");
        t.write(&t.repo, "a.txt", "x");
        assert_eq!(add(&t.repo, "a.txt"), Ok(Added::Added("/a.txt".into())));
        assert_eq!(fs::read_to_string(t.repo.join(".gitignore")).unwrap(), "target\r\nold.log\r\n/a.txt\r\n");
        assert_eq!(add(&t.repo, "a.txt"), Ok(Added::Already("/a.txt".into())));
        assert_eq!(fs::read_to_string(t.repo.join(".gitignore")).unwrap(), "target\r\nold.log\r\n/a.txt\r\n");
    }

    #[test]
    fn a_gitignore_that_is_not_utf8_keeps_its_bytes() {
        let t = TempRepo::new("ignore-bytes");
        fs::write(t.repo.join(".gitignore"), b"caf\xe9.log\n").unwrap();
        t.write(&t.repo, "a.txt", "x");
        assert_eq!(add(&t.repo, "a.txt"), Ok(Added::Added("/a.txt".into())));
        assert_eq!(fs::read(t.repo.join(".gitignore")).unwrap(), b"caf\xe9.log\n/a.txt\n");
    }

    #[test]
    fn a_tracked_file_is_refused() {
        let t = TempRepo::configured("ignore-tracked");
        t.write(&t.repo, "a.txt", "x");
        t.git(&t.repo, &["add", "a.txt"]);
        t.git(&t.repo, &["commit", "-q", "-m", "add a"]);
        assert!(add(&t.repo, "a.txt").is_err());
        assert!(!t.repo.join(".gitignore").exists());
    }
}
