//! Notes on the lines of a branch's diff, handed to the Claude working on the
//! branch: each says where (a file and a line of its new or old side), the
//! line itself, and what to do about it. They are written to a file with a
//! line that tells the Claude where it is (as `handoff` does for a clash).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Which side of the diff the line number is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    /// The file as the branch has it (added and unchanged lines).
    New,
    /// The file as it was (deleted lines).
    Old,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub path: String,
    pub line: u32,
    pub side: Side,
    /// The line's text, to find it again when the numbers moved.
    pub code: String,
    pub text: String,
}

impl Note {
    fn at(&self) -> (&str, Side, u32) {
        (&self.path, self.side, self.line)
    }
}

/// Puts `note` in `notes`, in place of the one on the same line; a note with
/// no text takes that one away. The notes stay in file and line order.
pub fn put(notes: &mut Vec<Note>, note: Note) {
    notes.retain(|n| n.at() != note.at());
    if !note.text.trim().is_empty() {
        notes.push(Note { text: note.text.trim().to_owned(), ..note });
    }
    notes.sort_by(|a, b| (&a.path, a.side, a.line).cmp(&(&b.path, b.side, b.line)));
}

/// The note on a line, if any.
pub fn find<'a>(notes: &'a [Note], path: &str, side: Side, line: u32) -> Option<&'a Note> {
    notes.iter().find(|n| n.at() == (path, side, line))
}

/// The text of the file.
pub fn brief(branch: &str, trunk: &str, notes: &[Note]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Notes on the diff of {branch} against {trunk}\n");
    let _ = writeln!(out, "Each note is about one line of `git diff {trunk}...{branch}`. Do what it says in this worktree.\n");
    for n in notes {
        let side = match n.side {
            Side::New => "",
            Side::Old => " (a line deleted here; its old number)",
        };
        let _ = writeln!(out, "## {}:{}{side}\n", n.path, n.line);
        let _ = writeln!(out, "```\n{}\n```\n", n.code);
        let _ = writeln!(out, "{}\n", n.text);
    }
    out
}

/// Writes the brief where the Claude can read it (outside the worktree, so
/// the repository stays clean). The file.
pub fn write(wt: &Path, branch: &str, trunk: &str, notes: &[Note]) -> std::io::Result<PathBuf> {
    let name = wt.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "repo".into());
    let name: String = name.chars().map(|ch| if ch.is_alphanumeric() || ch == '-' || ch == '_' { ch } else { '_' }).collect();
    let dir = std::env::temp_dir().join("michi-notes");
    std::fs::create_dir_all(&dir)?;
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let path = dir.join(format!("{name}-{secs}.md"));
    std::fs::write(&path, brief(branch, trunk, notes))?;
    Ok(path)
}

/// What the Claude is asked, on one line.
pub fn request(file: &Path) -> String {
    format!("Read {} and make the changes its notes ask for in this worktree. Do not commit.", file.display())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(path: &str, line: u32, text: &str) -> Note {
        Note { path: path.into(), line, side: Side::New, code: format!("code {line}"), text: text.into() }
    }

    #[test]
    fn a_note_replaces_the_one_on_its_line_and_empty_takes_it_away() {
        let mut v = Vec::new();
        put(&mut v, note("b.rs", 9, "later"));
        put(&mut v, note("a.rs", 12, "one"));
        put(&mut v, note("a.rs", 3, "  two \n"));
        assert_eq!(v.iter().map(|n| (n.path.as_str(), n.line)).collect::<Vec<_>>(), [("a.rs", 3), ("a.rs", 12), ("b.rs", 9)]);
        assert_eq!(v[0].text, "two");
        put(&mut v, note("a.rs", 12, "changed"));
        assert_eq!(find(&v, "a.rs", Side::New, 12).map(|n| n.text.as_str()), Some("changed"));
        put(&mut v, note("a.rs", 12, " "));
        assert!(find(&v, "a.rs", Side::New, 12).is_none());
        assert_eq!(v.len(), 2);
        // The same number on the old side is another line.
        put(&mut v, Note { side: Side::Old, ..note("a.rs", 3, "old") });
        assert_eq!(v.len(), 3);
    }

    #[test]
    fn the_brief_names_each_line_with_its_text_and_the_request_is_one_line() {
        let b = brief("feat", "main", &[note("a.rs", 3, "rename it")]);
        assert!(b.contains("diff of feat against main") && b.contains("## a.rs:3\n") && b.contains("```\ncode 3\n```\n\nrename it"));
        let r = request(Path::new("/t/x.md"));
        assert!(!r.contains('\n') && r.contains("x.md") && r.contains("Do not commit"));
    }
}
