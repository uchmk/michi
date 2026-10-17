//! Handing a clash to a Claude: the three sides of every place still to be
//! solved and what each side meant, written to a file, and a line that tells
//! the Claude where it is. The Claude is reached with the `tsumugi` command.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::conflict::{Block, Body, Change, Conflict, Intent};
use crate::git;

/// The text of the file: for each file with places to solve, each place's
/// lines in the worktree's file, the base's, and the other side's, with the
/// intent of both sides. `intents` come in the order of the open places.
pub fn brief(c: &Conflict, intents: &[Intent]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Conflicts to solve in {}\n", c.wt.display());
    let _ = writeln!(out, "Ours: {} ({})  Theirs: {} ({})\n", c.ours.name, short(&c.ours.id), c.theirs.name, short(&c.theirs.id));
    let mut n = 0;
    for f in &c.files {
        match &f.body {
            Body::Text(blocks) => {
                for b in blocks.iter().filter(|b| b.is_open()) {
                    let at = b.line.map(|l| format!(" (line {l})")).unwrap_or_default();
                    let _ = writeln!(out, "## {}{at}\n", f.path);
                    if let Some(i) = intents.get(n) {
                        let _ = writeln!(out, "- Ours meant: {}", change(&i.ours));
                        let _ = writeln!(out, "- Theirs meant: {}\n", change(&i.theirs));
                    }
                    n += 1;
                    section(&mut out, "Ours", Some(&b.ours));
                    section(&mut out, "Base", b.base.as_deref());
                    section(&mut out, "Theirs", Some(&b.theirs));
                }
            }
            Body::Whole(_) if f.unmerged => {
                let _ = writeln!(
                    out,
                    "## {}\n\nThis file clashed as a whole (deleted on one side, binary, renamed...): look at `git diff` and the stages.\n",
                    f.path
                );
            }
            Body::Whole(_) => {}
        }
    }
    out
}

fn short(id: &Option<crate::oid::Oid>) -> String {
    id.map(|o| o.short()).unwrap_or_default()
}

fn change(c: &Option<Change>) -> String {
    match c {
        Some(c) => format!("{} {} ({})", c.id.short(), c.subject, c.author),
        None => "(not known)".to_owned(),
    }
}

fn section(out: &mut String, word: &str, lines: Option<&[String]>) {
    let _ = writeln!(out, "{word}:");
    match lines {
        None => {
            let _ = writeln!(out, "(not known)\n");
        }
        Some(l) => {
            let _ = writeln!(out, "```");
            for line in l {
                let _ = writeln!(out, "{line}");
            }
            let _ = writeln!(out, "```\n");
        }
    }
}

/// The open places of `c`, in the order [`brief`] takes their intents.
pub fn open_places(c: &Conflict) -> Vec<(&str, &Block)> {
    let mut all = Vec::new();
    for f in &c.files {
        if let Body::Text(blocks) = &f.body {
            all.extend(blocks.iter().filter(|b| b.is_open()).map(|b| (f.path.as_str(), b)));
        }
    }
    all
}

/// Writes the brief where the Claude can read it (outside the worktree, so
/// the repository stays clean). The file.
pub fn write(c: &Conflict, intents: &[Intent]) -> std::io::Result<PathBuf> {
    let name = c.wt.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "repo".into());
    let name: String = name.chars().map(|ch| if ch.is_alphanumeric() || ch == '-' || ch == '_' { ch } else { '_' }).collect();
    let dir = std::env::temp_dir().join("michi-handoff");
    std::fs::create_dir_all(&dir)?;
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let path = dir.join(format!("{name}-{secs}.md"));
    std::fs::write(&path, brief(c, intents))?;
    Ok(path)
}

/// What the Claude is asked, on one line.
pub fn request(file: &Path) -> String {
    format!(
        "Read {} and solve the conflicts in this worktree keeping the intent of both sides, then git add the files. Do not commit.",
        file.display()
    )
}

/// Where a hand-over of a clash stands, for the strip of the conflict view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Asked; the Claude has not stopped yet.
    Asked,
    /// The Claude stopped, with `left` places still carrying markers.
    Stopped { left: usize },
}

/// What the person can do from the strip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Offer {
    /// Look at what the Claude did, in the diff.
    Review,
    /// Take it: commit (or continue) what was solved.
    Adopt,
    /// Ask the Claude to do it again.
    SendBack,
}

impl Stage {
    /// The choices at this stage, in the order the strip shows them. While the
    /// Claude works there are none; adopting waits until no place is left.
    pub fn offers(self) -> Vec<Offer> {
        match self {
            Stage::Asked => Vec::new(),
            Stage::Stopped { left: 0 } => vec![Offer::Review, Offer::Adopt, Offer::SendBack],
            Stage::Stopped { .. } => vec![Offer::Review, Offer::SendBack],
        }
    }

    /// The stage once the markers were counted (`None`: nothing is stopped
    /// there any more, so there is nothing to follow).
    pub fn after_count(left: Option<usize>) -> Option<Stage> {
        left.map(|left| Stage::Stopped { left })
    }
}

/// What the Claude is told when its answer is sent back, on one line.
pub fn send_back_request(left: usize) -> String {
    if left > 0 {
        format!(
            "Your solution was not accepted: {left} place(s) in this worktree still have conflict markers. Solve them keeping the intent of both sides, then git add the files. Do not commit."
        )
    } else {
        "Your solution was not accepted. Look at `git diff --cached` in this worktree, redo what is wrong keeping the intent of both sides, then git add the files. Do not commit.".to_owned()
    }
}

/// Types `text` into the tsumugi session `id`.
pub fn send(id: u64, text: &str) -> Result<(), git::Error> {
    tsumugi(Command::new("tsumugi").args(["send", &id.to_string(), text]))
}

/// Brings the tsumugi session `id` (its window and tab) to the front.
pub fn focus(id: u64) -> Result<(), git::Error> {
    tsumugi(Command::new("tsumugi").args(["focus", &id.to_string()]))
}

/// Starts a Claude in a new worktree made by tsumugi for `plan`.
pub fn new_work(repo: &Path, plan: &crate::newwork::Plan) -> Result<(), git::Error> {
    tsumugi(Command::new("tsumugi").args(crate::newwork::args(repo, plan)))
}

/// Closes the tsumugi session `id`.
pub fn close(id: u64) -> Result<(), git::Error> {
    tsumugi(Command::new("tsumugi").args(["close", &id.to_string()]))
}

/// A new tsumugi tab in `folder` named `name` that starts claude with `text`.
pub fn start(folder: &Path, name: &str, text: &str) -> Result<(), git::Error> {
    tsumugi(Command::new("tsumugi").arg("new").arg(folder).args(["--name", name, "--", "claude", text]))
}

fn tsumugi(cmd: &mut Command) -> Result<(), git::Error> {
    match cmd.output() {
        Ok(o) if o.status.success() => Ok(()),
        Ok(o) => Err(git::Error::Git(String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("tsumugi failed").to_owned())),
        Err(e) => Err(git::Error::Git(format!("tsumugi: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conflict::{File, Side};
    use crate::oid::Oid;

    fn block(line: Option<usize>) -> Block {
        Block {
            line,
            ours: vec!["mine".into()],
            base: Some(vec!["old".into()]),
            theirs: vec!["yours".into()],
            result: line.is_none().then(|| vec!["x".into()]),
            before: vec![],
            skipped: 0,
            after: vec![],
            skipped_after: 0,
        }
    }

    fn conflict() -> Conflict {
        Conflict {
            wt: PathBuf::from("/r/w"),
            doing: crate::branch::Doing::Merge,
            ours: Side { name: "feat".into(), id: None },
            theirs: Side { name: "main".into(), id: None },
            base: None,
            started: None,
            message: String::new(),
            files: vec![File { path: "a.txt".into(), unmerged: true, body: Body::Text(vec![block(Some(3)), block(None)]) }],
            auto: vec![],
        }
    }

    #[test]
    fn the_brief_holds_only_the_open_places_with_all_three_sides() {
        let c = conflict();
        assert_eq!(open_places(&c).len(), 1);
        let i = Intent {
            ours: None,
            theirs: Some(Change {
                id: Oid::parse("0123456789abcdef0123456789abcdef01234567").unwrap(),
                subject: "Add y".into(),
                author: "me".into(),
                time: 0,
            }),
        };
        let text = brief(&c, &[i]);
        assert!(text.contains("## a.txt (line 3)"));
        assert!(text.contains("Ours:\n```\nmine\n```"));
        assert!(text.contains("Base:\n```\nold\n```"));
        assert!(text.contains("Theirs:\n```\nyours\n```"));
        assert!(text.contains("Theirs meant: 0123456 Add y (me)"));
        assert!(text.contains("Ours meant: (not known)"));
        assert_eq!(text.matches("## ").count(), 1, "the solved place is left out");
    }

    #[test]
    fn the_request_is_one_line_with_the_file() {
        let r = request(Path::new("/t/x.md"));
        assert!(!r.contains('\n') && r.contains("x.md") && r.contains("Do not commit"));
    }

    #[test]
    fn the_strip_offers_adopting_only_when_nothing_is_left() {
        assert!(Stage::Asked.offers().is_empty());
        assert_eq!(Stage::Stopped { left: 2 }.offers(), vec![Offer::Review, Offer::SendBack]);
        assert_eq!(Stage::Stopped { left: 0 }.offers(), vec![Offer::Review, Offer::Adopt, Offer::SendBack]);
    }

    #[test]
    fn a_count_of_markers_gives_the_stage_and_none_ends_it() {
        assert_eq!(Stage::after_count(Some(3)), Some(Stage::Stopped { left: 3 }));
        assert_eq!(Stage::after_count(None), None);
    }

    #[test]
    fn sending_back_is_one_line_that_says_what_is_wrong() {
        let (a, b) = (send_back_request(2), send_back_request(0));
        assert!(!a.contains('\n') && !b.contains('\n'));
        assert!(a.contains("2 place(s)") && b.contains("git diff --cached"));
        assert!(a.contains("Do not commit") && b.contains("Do not commit"));
    }
}
