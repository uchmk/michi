//! `git bisect` in a worktree: start it from a bad and a good commit, mark
//! the commit it checks out good, bad or skipped, read how far it is, and
//! reset. A bisect checks commits out, so starting one is refused while the
//! worktree has uncommitted changes (or is in the middle of something), and
//! the commit the worktree was on is kept under `refs/michi/backup/` before
//! it starts and before it is reset. Only the worktree given is touched.
//!
//! What is read comes from `git bisect log` (the marks and the result) and
//! `git rev-list --bisect-vars` (how many steps are left); both read-only.

use std::path::Path;

use crate::backup;
use crate::branch::{self, Doing};
use crate::git;
use crate::oid::Oid;

/// A mark on a commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Good,
    Bad,
    Skip,
}

impl Mark {
    /// The word of `git bisect <word>`, the same in every language.
    pub fn word(self) -> &'static str {
        match self {
            Mark::Good => "good",
            Mark::Bad => "bad",
            Mark::Skip => "skip",
        }
    }
}

/// What `git bisect log` says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Log {
    pub bad: Option<Oid>,
    pub good: Vec<Oid>,
    pub skipped: Vec<Oid>,
    /// The result, once git has narrowed it down to one commit.
    pub first_bad: Option<Oid>,
    /// Skips left several commits that may be the first bad one.
    pub possible: bool,
}

/// How far a running bisect is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub bad: Option<Oid>,
    pub good: Vec<Oid>,
    pub skipped: Vec<Oid>,
    /// The commit the worktree has checked out (the one to test).
    pub head: Option<Oid>,
    pub head_subject: String,
    /// The first bad commit, once found (`possible`: skips leave doubt).
    pub first_bad: Option<Oid>,
    pub first_subject: String,
    pub possible: bool,
    /// Commits still to look at, and the steps left after the next one.
    pub left: Option<u32>,
    pub steps: Option<u32>,
}

impl State {
    /// Whether a bad and a good commit are both known, so a commit is
    /// checked out to test.
    pub fn testing(&self) -> bool {
        self.bad.is_some() && !self.good.is_empty() && self.first_bad.is_none()
    }
}

/// Reads the text of `git bisect log`: the `# bad: [id] subject` style
/// comments git writes for each mark and for the result.
pub fn parse_log(text: &str) -> Log {
    let mut log = Log::default();
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("# ") else { continue };
        let Some((what, tail)) = rest.split_once(": [") else { continue };
        let Some(id) = tail.split(']').next().and_then(Oid::parse) else { continue };
        match what {
            "bad" => log.bad = Some(id),
            "good" => log.good.push(id),
            "skip" => log.skipped.push(id),
            "first bad commit" => log.first_bad = Some(id),
            "possible first bad commit" => {
                log.first_bad = Some(id);
                log.possible = true;
            }
            _ => {}
        }
    }
    log
}

/// The `bisect_nr` and `bisect_steps` of `git rev-list --bisect-vars`.
pub fn parse_vars(text: &str) -> (Option<u32>, Option<u32>) {
    let mut nr = None;
    let mut steps = None;
    for line in text.lines() {
        match line.split_once('=') {
            Some(("bisect_nr", v)) => nr = v.trim().parse().ok(),
            Some(("bisect_steps", v)) => steps = v.trim().parse().ok(),
            _ => {}
        }
    }
    (nr, steps)
}

/// The `git bisect run` line to hand to a terminal or a Claude: it starts
/// the bisect first when none runs (`pending`: its bad and good commits).
/// `test` is the command that exits 0 for a good commit, 1-127 (not 125)
/// for a bad one and 125 to skip.
pub fn run_line(wt: &Path, pending: Option<(&Oid, &Oid)>, test: &str) -> String {
    let dir = wt.display().to_string();
    let start = match pending {
        Some((bad, good)) => format!("git -C \"{dir}\" bisect start {bad} {good} && "),
        None => String::new(),
    };
    format!("{start}git -C \"{dir}\" bisect run {test}")
}

fn rev(wt: &Path, spec: &str) -> Option<Oid> {
    let out = git::read_maybe(wt, &["rev-parse", "-q", "--verify", &format!("{spec}^{{commit}}")]).ok()??;
    Oid::parse(String::from_utf8_lossy(&out).trim())
}

fn subject(wt: &Path, id: &Oid) -> String {
    git::read_text(wt, &["log", "-1", "--format=%s", &id.to_string()]).unwrap_or_default()
}

/// The bisect running in the worktree at `wt`; `None` when there is none.
pub fn read(wt: &Path) -> Result<Option<State>, git::Error> {
    if branch::doing(wt) != Some(Doing::Bisect) {
        return Ok(None);
    }
    let log = parse_log(&git::read_text(wt, &["bisect", "log"]).unwrap_or_default());
    let head = rev(wt, "HEAD");
    let (mut left, mut steps) = (None, None);
    if let (Some(bad), false, None) = (&log.bad, log.good.is_empty(), &log.first_bad) {
        let mut args = vec!["rev-list".to_owned(), "--bisect-vars".to_owned(), bad.to_string()];
        args.extend(log.good.iter().map(|g| format!("^{g}")));
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        if let Ok(text) = git::read_text(wt, &args) {
            (left, steps) = parse_vars(&text);
        }
    }
    Ok(Some(State {
        head_subject: head.as_ref().map(|h| subject(wt, h)).unwrap_or_default(),
        first_subject: log.first_bad.as_ref().map(|h| subject(wt, h)).unwrap_or_default(),
        head,
        bad: log.bad,
        good: log.good,
        skipped: log.skipped,
        first_bad: log.first_bad,
        possible: log.possible,
        left,
        steps,
    }))
}

/// Starts a bisect in the worktree at `wt` with `bad` and `good`. Refused
/// while the worktree is in the middle of something or has uncommitted
/// changes to tracked files (the checkouts would trample them). Where the
/// worktree was is kept first; the checkout of the commit to test follows.
/// Returns the state and the backup.
pub fn start(wt: &Path, bad: &Oid, good: &Oid) -> Result<(Option<State>, String), git::Error> {
    if let Some(d) = branch::doing(wt) {
        return Err(git::Error::Git(format!("this worktree is in the middle of a {}", d.word())));
    }
    if !git::read_text(wt, &["status", "--porcelain", "--untracked-files=no"])?.trim().is_empty() {
        return Err(git::Error::Git("this worktree has uncommitted changes: commit or stash them first".to_owned()));
    }
    let tip = rev(wt, "HEAD").ok_or_else(|| git::Error::Git("no commit to bisect from".to_owned()))?;
    let saved = backup::save(wt, &tip, "before bisect start")?;
    if let Err(e) = git::read(wt, &["bisect", "start", &bad.to_string(), &good.to_string()]) {
        // A refused start (the good commit is not older than the bad one…)
        // can leave the bisect half made.
        if branch::doing(wt) == Some(Doing::Bisect) {
            let _ = git::read(wt, &["bisect", "reset"]);
        }
        return Err(e);
    }
    Ok((read(wt)?, saved))
}

/// Marks `id` (the commit checked out, with none) in the bisect of `wt`.
pub fn mark(wt: &Path, how: Mark, id: Option<&Oid>) -> Result<Option<State>, git::Error> {
    if branch::doing(wt) != Some(Doing::Bisect) {
        return Err(git::Error::Git("no bisect is running in this worktree".to_owned()));
    }
    let id = id.map(ToString::to_string);
    let mut args = vec!["bisect", how.word()];
    args.extend(id.as_deref());
    git::read(wt, &args)?;
    read(wt)
}

/// Ends the bisect of `wt` and puts the worktree back where it started.
/// The commit it had checked out is kept first. The backup's name.
pub fn reset(wt: &Path) -> Result<String, git::Error> {
    if branch::doing(wt) != Some(Doing::Bisect) {
        return Err(git::Error::Git("no bisect is running in this worktree".to_owned()));
    }
    let tip = rev(wt, "HEAD").ok_or_else(|| git::Error::Git("no commit checked out".to_owned()))?;
    let saved = backup::save(wt, &tip, "before bisect reset")?;
    git::read(wt, &["bisect", "reset"])?;
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn commit(t: &TempRepo, n: u32, text: &str) -> Oid {
        t.write(&t.repo, "a.txt", text);
        t.write(&t.repo, "n.txt", &format!("{n}\n"));
        t.git(&t.repo, &["add", "a.txt", "n.txt"]);
        t.git(&t.repo, &["commit", "-q", "-m", &format!("change {n}")]);
        Oid::parse(&t.out(&t.repo, &["rev-parse", "HEAD"])).unwrap()
    }

    /// Eight commits; from the fifth on a.txt says `bad`.
    fn history(name: &str) -> (TempRepo, Vec<Oid>) {
        let t = TempRepo::configured(name);
        let ids = (1..=8).map(|n| commit(&t, n, if n >= 5 { "bad\n" } else { "ok\n" })).collect();
        (t, ids)
    }

    fn id(c: char) -> String {
        c.to_string().repeat(40)
    }

    #[test]
    fn a_log_is_read_for_its_marks_and_its_result() {
        let text = format!(
            "git bisect start '{b}' '{g}'\n# status: waiting for both good and bad commits\n# bad: [{b}] top\ngit bisect bad {b}\n\
             # good: [{g}] old\ngit bisect good {g}\n# skip: [{s}] skipped one\ngit bisect skip {s}\n\
             # first bad commit: [{f}] the culprit\n",
            b = id('b'),
            g = id('a'),
            s = id('c'),
            f = id('d')
        );
        let log = parse_log(&text);
        assert_eq!(log.bad, Oid::parse(&id('b')));
        assert_eq!(log.good, vec![Oid::parse(&id('a')).unwrap()]);
        assert_eq!(log.skipped, vec![Oid::parse(&id('c')).unwrap()]);
        assert_eq!(log.first_bad, Oid::parse(&id('d')));
        assert!(!log.possible);
        let log = parse_log(&format!("# possible first bad commit: [{}] x\n", id('e')));
        assert!(log.possible && log.first_bad.is_some());
        assert_eq!(parse_log("git bisect start\nnoise\n# status: waiting for both\n"), Log::default());
    }

    #[test]
    fn the_steps_are_read_from_the_vars() {
        let text = "bisect_rev='abc'\nbisect_nr=3\nbisect_good=1\nbisect_bad=1\nbisect_all=7\nbisect_steps=2";
        assert_eq!(parse_vars(text), (Some(3), Some(2)));
        assert_eq!(parse_vars(""), (None, None));
    }

    #[test]
    fn the_run_line_starts_the_bisect_when_none_runs() {
        let wt = Path::new("/r/main");
        let (b, g) = (Oid::parse(&id('b')).unwrap(), Oid::parse(&id('a')).unwrap());
        assert_eq!(run_line(wt, None, "make test"), "git -C \"/r/main\" bisect run make test");
        let line = run_line(wt, Some((&b, &g)), "make test");
        assert!(line.starts_with(&format!("git -C \"/r/main\" bisect start {b} {g} && git -C")), "{line}");
        assert!(line.ends_with("bisect run make test"));
    }

    #[test]
    fn a_bisect_finds_the_first_bad_commit_and_resets() {
        let (t, ids) = history("bisect-find");
        let wt = &t.repo;
        assert_eq!(read(wt).unwrap(), None);
        let (state, saved) = start(wt, &ids[7], &ids[0]).unwrap();
        let s = state.unwrap();
        assert!(s.testing());
        assert_eq!(s.bad, Some(ids[7]));
        assert_eq!(s.good, vec![ids[0]]);
        assert!(s.left.is_some() && s.steps.is_some(), "{s:?}");
        assert_eq!(t.out(wt, &["rev-parse", &saved]), ids[7].to_string(), "the old tip is kept");
        // Test the commit checked out, as `git bisect run` would, until it ends.
        let mut s = s;
        for _ in 0..8 {
            if s.first_bad.is_some() {
                break;
            }
            let text = std::fs::read_to_string(wt.join("a.txt")).unwrap();
            let how = if text == "bad\n" { Mark::Bad } else { Mark::Good };
            s = mark(wt, how, None).unwrap().unwrap();
        }
        assert_eq!(s.first_bad, Some(ids[4]));
        assert_eq!(s.first_subject, "change 5");
        assert!(!s.testing());
        // Reset: back on main, and the checked out commit kept.
        let kept = reset(wt).unwrap();
        assert_eq!(read(wt).unwrap(), None);
        assert_eq!(t.out(wt, &["rev-parse", "--abbrev-ref", "HEAD"]), "main");
        assert!(kept.starts_with(backup::PREFIX));
        assert!(reset(wt).is_err(), "nothing to reset");
    }

    #[test]
    fn a_skip_and_a_named_commit_are_marked() {
        let (t, ids) = history("bisect-skip");
        let wt = &t.repo;
        start(wt, &ids[7], &ids[0]).unwrap();
        let s = mark(wt, Mark::Skip, None).unwrap().unwrap();
        assert_eq!(s.skipped.len(), 1);
        let s = mark(wt, Mark::Good, Some(&ids[1])).unwrap().unwrap();
        assert!(s.good.contains(&ids[1]));
    }

    #[test]
    fn a_start_is_refused_over_uncommitted_changes_or_a_running_bisect() {
        let (t, ids) = history("bisect-refuse");
        let wt = &t.repo;
        t.write(wt, "a.txt", "edited\n");
        let e = start(wt, &ids[7], &ids[0]).unwrap_err().to_string();
        assert!(e.contains("uncommitted"), "{e}");
        assert_eq!(read(wt).unwrap(), None);
        assert!(t.out(wt, &["for-each-ref", "refs/michi/backup/"]).is_empty(), "nothing kept when refused");
        t.git(wt, &["checkout", "-q", "a.txt"]);
        start(wt, &ids[7], &ids[0]).unwrap();
        let e = start(wt, &ids[7], &ids[0]).unwrap_err().to_string();
        assert!(e.contains("middle of a bisect"), "{e}");
    }

    #[test]
    fn a_bad_start_leaves_no_bisect() {
        let (t, ids) = history("bisect-bad-start");
        let wt = &t.repo;
        // The good commit is newer than the bad one.
        assert!(start(wt, &ids[0], &ids[7]).is_err() || read(wt).unwrap().is_some());
        if read(wt).unwrap().is_some() {
            reset(wt).unwrap();
        }
        assert_eq!(read(wt).unwrap(), None);
        assert_eq!(t.out(wt, &["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    }

    #[test]
    fn only_the_given_worktree_is_bisected() {
        let (t, ids) = history("bisect-other");
        let other = t.root.join("other");
        t.git(&t.repo, &["worktree", "add", "-q", "-b", "side", other.to_str().unwrap(), "HEAD"]);
        start(&t.repo, &ids[7], &ids[0]).unwrap();
        assert_eq!(read(&other).unwrap(), None);
        assert_eq!(t.out(&other, &["rev-parse", "HEAD"]), ids[7].to_string());
        assert_eq!(t.out(&other, &["rev-parse", "--abbrev-ref", "HEAD"]), "side");
    }
}
