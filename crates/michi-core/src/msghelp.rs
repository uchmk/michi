//! Help for writing a commit message (#74): the messages of the latest
//! commits to step back through, and the template `commit.template` names.
//! No egui; git is run in the worktree's folder.

use std::path::Path;

use crate::git;

/// The most past messages kept.
pub const MAX_PAST: usize = 50;

/// What the message box can be helped with.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Help {
    /// The messages of the latest commits, newest first, each once.
    pub past: Vec<String>,
    /// The template with its comment lines taken out; `None` when there is
    /// none or nothing else is in it.
    pub template: Option<String>,
}

/// Reads the help for the worktree at `dir`. A repository with no commits
/// has no past; a template that cannot be read is none.
pub fn read(dir: &Path) -> Help {
    let log = git::read(dir, &["log", "-n", "200", "--no-show-signature", "--format=%B%x00", "HEAD"]).unwrap_or_default();
    let past = parse_log(&String::from_utf8_lossy(&log));
    let template = git::read_maybe(dir, &["config", "--path", "--get", "commit.template"])
        .ok()
        .flatten()
        .map(|b| String::from_utf8_lossy(&b).trim().to_owned())
        .filter(|p| !p.is_empty())
        .and_then(|p| {
            let p = Path::new(&p);
            std::fs::read(if p.is_absolute() { p.to_path_buf() } else { dir.join(p) }).ok()
        })
        .and_then(|b| template_text(&String::from_utf8_lossy(&b)));
    Help { past, template }
}

/// The messages in `log` (each ended with a NUL), newest first, the blank
/// ones and the repeats let go, at most [`MAX_PAST`].
pub fn parse_log(log: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for m in log.split('\0') {
        let m = m.trim().to_owned();
        if !m.is_empty() && !out.contains(&m) {
            out.push(m);
            if out.len() == MAX_PAST {
                break;
            }
        }
    }
    out
}

/// A template as the box takes it: git drops the lines starting with `#`
/// when it commits, but michi commits with `--cleanup=whitespace`, so they
/// are not put in. `None` when nothing is left.
pub fn template_text(raw: &str) -> Option<String> {
    let kept: Vec<&str> = raw.lines().filter(|l| !l.starts_with('#')).map(str::trim_end).collect();
    let text = kept.join("\n");
    let text = text.trim_start_matches('\n');
    if text.trim().is_empty() {
        None
    } else {
        Some(format!("{}\n", text.trim_end()))
    }
}

/// Where stepping through the past is: nowhere (the message being written)
/// or the `n`th past message, 0 the newest.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Walk {
    at: Option<usize>,
    /// What was in the box before the first step, to come back to.
    draft: String,
}

impl Walk {
    /// One step older (`older`) or newer from the box's `current` text.
    /// Returns the text to put in the box, `None` when there is nowhere to
    /// go (no past, already the oldest, or already at the draft).
    pub fn step(&mut self, past: &[String], older: bool, current: &str) -> Option<String> {
        // Text changed by hand since the last step is a new draft.
        if self.at.is_none_or(|n| past.get(n).is_none_or(|m| m != current)) {
            self.at = None;
            self.draft = current.to_owned();
        }
        let next = match (self.at, older) {
            (None, true) if !past.is_empty() => Some(0),
            (Some(n), true) if n + 1 < past.len() => Some(n + 1),
            (Some(0), false) => None,
            (Some(n), false) => Some(n - 1),
            _ => return None,
        };
        self.at = next;
        Some(next.map_or_else(|| self.draft.clone(), |n| past[n].clone()))
    }

    /// Forgets where it was (a commit emptied the box, say).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn past(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("m{i}")).collect()
    }

    #[test]
    fn the_log_gives_each_message_once_newest_first() {
        let log = "Add b\n\nbody\n\0Add a\0\n\0Add b\n\nbody\n\0first\0";
        assert_eq!(parse_log(log), ["Add b\n\nbody", "Add a", "first"]);
        let many: String = (0..80).map(|i| format!("m{i}\0")).collect();
        assert_eq!(parse_log(&many).len(), MAX_PAST);
    }

    #[test]
    fn a_template_loses_its_comments() {
        assert_eq!(template_text("# Subject\n\nfeat: \n\n# body\n").as_deref(), Some("feat:\n"));
        assert_eq!(template_text("# only\n# comments\n"), None);
        assert_eq!(template_text(""), None);
    }

    #[test]
    fn stepping_goes_back_and_returns_to_the_draft() {
        let p = past(2);
        let mut w = Walk::default();
        assert_eq!(w.step(&p, false, "wip"), None);
        assert_eq!(w.step(&p, true, "wip").as_deref(), Some("m0"));
        assert_eq!(w.step(&p, true, "m0").as_deref(), Some("m1"));
        assert_eq!(w.step(&p, true, "m1"), None);
        assert_eq!(w.step(&p, false, "m1").as_deref(), Some("m0"));
        assert_eq!(w.step(&p, false, "m0").as_deref(), Some("wip"));
        assert_eq!(w.step(&p, false, "wip"), None);
        assert_eq!(Walk::default().step(&[], true, ""), None);
    }

    #[test]
    fn typing_after_a_step_makes_a_new_draft() {
        let p = past(3);
        let mut w = Walk::default();
        w.step(&p, true, "");
        assert_eq!(w.step(&p, true, "m0 edited").as_deref(), Some("m0"));
        assert_eq!(w.step(&p, false, "m0").as_deref(), Some("m0 edited"));
    }

    #[test]
    fn the_repository_gives_its_past_and_its_template() {
        let t = TempRepo::configured("msghelp");
        t.write(&t.repo, "a.txt", "a\n");
        t.git(&t.repo, &["add", "a.txt"]);
        t.git(&t.repo, &["commit", "-q", "-m", "Add a"]);
        assert_eq!(read(&t.repo).past, ["Add a", "first"]);
        assert_eq!(read(&t.repo).template, None);
        t.write(&t.repo, "tpl.txt", "type: \n# help\n");
        t.git(&t.repo, &["config", "commit.template", "tpl.txt"]);
        assert_eq!(read(&t.repo).template.as_deref(), Some("type:\n"));
        t.git(&t.repo, &["config", "commit.template", "missing.txt"]);
        assert_eq!(read(&t.repo).template, None);
    }
}
