//! The checklist before a branch is merged or turned into a pull request:
//! the working tree is clean, the trunk would not clash with it, it is not
//! behind the trunk, `scripts/verify.sh` passed on this very commit, GitHub's
//! checks for the commit are green, and everything is pushed. Each item is
//! ok, a warning, or unknown (with the reason), never a guess.
//!
//! Where the verify result comes from: michi records it (`record_verify`, in
//! `<git-common-dir>/michi/verify/<commit>`) whenever a landing runs the
//! repository's check, and anything else may write the same file (`ok` or
//! `fail <exit code>` on one line). yagura keeps no such record. GitHub's
//! checks are asked of `gh api` with a short timeout and cached for a minute;
//! without `gh`, a network or a GitHub remote the item is unknown. The calls
//! block; run them off the UI thread.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{mpsc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::clash::Clashes;
use crate::git;
use crate::oid::Oid;
use crate::pr;

/// How an item stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok,
    Warn,
    Unknown,
}

/// The six items, in the order they are listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Tree,
    Clash,
    Behind,
    Verify,
    Checks,
    Pushed,
}

/// What was found for an item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Fact {
    Clean,
    Dirty(usize),
    TreeUnknown,
    NoClash,
    Clashes(Vec<String>),
    ClashUnknown,
    UpToDate,
    Behind(u32),
    BehindUnknown,
    VerifyPassed,
    VerifyFailed(Option<i32>),
    VerifyNone,
    CiPassed(u32),
    CiFailed(u32),
    CiPending(u32),
    CiNone,
    CiNotPushed,
    CiNotGithub,
    /// GitHub could not be asked: why, in one line.
    CiUnknown(String),
    Pushed,
    Unpushed(u32),
    NoUpstream,
    UpstreamGone,
}

impl Fact {
    pub fn item(&self) -> Item {
        use Fact::*;
        match self {
            Clean | Dirty(_) | TreeUnknown => Item::Tree,
            NoClash | Clashes(_) | ClashUnknown => Item::Clash,
            UpToDate | Behind(_) | BehindUnknown => Item::Behind,
            VerifyPassed | VerifyFailed(_) | VerifyNone => Item::Verify,
            CiPassed(_) | CiFailed(_) | CiPending(_) | CiNone | CiNotPushed | CiNotGithub | CiUnknown(_) => Item::Checks,
            Pushed | Unpushed(_) | NoUpstream | UpstreamGone => Item::Pushed,
        }
    }

    pub fn status(&self) -> Status {
        use Fact::*;
        match self {
            Clean | NoClash | UpToDate | VerifyPassed | CiPassed(_) | Pushed => Status::Ok,
            Dirty(_) | Clashes(_) | Behind(_) | VerifyFailed(_) | CiFailed(_) | CiPending(_) | Unpushed(_) | NoUpstream | UpstreamGone => {
                Status::Warn
            }
            TreeUnknown | ClashUnknown | BehindUnknown | VerifyNone | CiNone | CiNotPushed | CiNotGithub | CiUnknown(_) => Status::Unknown,
        }
    }
}

/// What the window already knows about the branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    /// The branch's worktree.
    pub wt: PathBuf,
    pub branch: String,
    pub head: Oid,
    /// Changed files in the worktree; `None` when not read yet.
    pub dirty: Option<usize>,
    /// The files the trunk would conflict in (empty when clean); `None` when
    /// the prediction has no word on it yet (see [`clash_of`]).
    pub clash: Option<Vec<String>>,
    /// Commits the trunk has that the branch lacks.
    pub behind: Option<u32>,
    /// `(ahead, gone)` of its upstream; `None` when it has none.
    pub upstream: Option<(u32, bool)>,
}

/// The files `wt`'s head would conflict in with the trunk, from the
/// prediction the window keeps: `Some(files)` (empty when clean) or `None`
/// when it has no word on that pair yet.
pub fn clash_of(clashes: &Clashes, wt: &Path, trunk: &str) -> Option<Vec<String>> {
    let a = clashes.side_of(wt)?;
    let b = clashes.sides.iter().position(|s| s.name == trunk && !s.path.as_deref().is_some_and(|p| crate::paths::same_path(p, wt)))?;
    match clashes.between(a, b)? {
        crate::clash::Outcome::Unknown => None,
        o => Some(o.conflict().map_or_else(Vec::new, |c| c.files.iter().map(|f| f.path.clone()).collect())),
    }
}

/// The facts that need no disk or network.
fn known(i: &Input) -> [Fact; 3] {
    [
        match i.dirty {
            None => Fact::TreeUnknown,
            Some(0) => Fact::Clean,
            Some(n) => Fact::Dirty(n),
        },
        match &i.clash {
            None => Fact::ClashUnknown,
            Some(f) if f.is_empty() => Fact::NoClash,
            Some(f) => Fact::Clashes(f.clone()),
        },
        match i.behind {
            None => Fact::BehindUnknown,
            Some(0) => Fact::UpToDate,
            Some(n) => Fact::Behind(n),
        },
    ]
}

fn pushed(i: &Input) -> Fact {
    match i.upstream {
        None => Fact::NoUpstream,
        Some((_, true)) => Fact::UpstreamGone,
        Some((0, false)) => Fact::Pushed,
        Some((n, false)) => Fact::Unpushed(n),
    }
}

/// The whole list, one fact per item in [`Item`]'s order. Reads the verify
/// record and asks GitHub.
pub fn check(i: &Input) -> Vec<Fact> {
    let [tree, clash, behind] = known(i);
    let verify = match read_verify(&i.wt, &i.head) {
        Some(Mark::Ok) => Fact::VerifyPassed,
        Some(Mark::Failed(code)) => Fact::VerifyFailed(code),
        None => Fact::VerifyNone,
    };
    let pushed = pushed(i);
    let ci = if matches!(pushed, Fact::Pushed) { checks(&i.wt, &i.branch, &i.head) } else { Fact::CiNotPushed };
    vec![tree, clash, behind, verify, ci, pushed]
}

/// A recorded verify result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    Ok,
    Failed(Option<i32>),
}

fn mark_path(wt: &Path, head: &Oid) -> Option<PathBuf> {
    let out = git::read_text(wt, &["rev-parse", "--path-format=absolute", "--git-common-dir"]).ok()?;
    let dir = out.trim();
    (!dir.is_empty()).then(|| PathBuf::from(dir).join("michi").join("verify").join(head.to_string()))
}

/// Records that the check ran on `head` of the worktree `wt`: `None` for
/// passed, `Some(code)` for failed. Best effort.
pub fn record_verify(wt: &Path, head: &Oid, failed: Option<Option<i32>>) {
    let Some(path) = mark_path(wt, head) else { return };
    let text = match failed {
        None => "ok\n".to_owned(),
        Some(Some(c)) => format!("fail {c}\n"),
        Some(None) => "fail\n".to_owned(),
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, text);
}

/// The recorded result for `head`, if any.
pub fn read_verify(wt: &Path, head: &Oid) -> Option<Mark> {
    parse_mark(&std::fs::read_to_string(mark_path(wt, head)?).ok()?)
}

fn parse_mark(text: &str) -> Option<Mark> {
    let line = text.lines().next()?.trim();
    if line == "ok" {
        return Some(Mark::Ok);
    }
    let rest = line.strip_prefix("fail")?;
    Some(Mark::Failed(rest.trim().parse().ok()))
}

/// Counts the conclusions `gh` printed (`success,failure,in_progress`) as one
/// fact.
pub fn parse_checks(text: &str) -> Fact {
    let (mut ok, mut bad, mut wait) = (0u32, 0u32, 0u32);
    for c in text.trim().split(',').map(str::trim).filter(|c| !c.is_empty()) {
        match c {
            "success" | "neutral" | "skipped" => ok += 1,
            "failure" | "cancelled" | "timed_out" | "action_required" | "startup_failure" | "stale" => bad += 1,
            _ => wait += 1,
        }
    }
    match (ok, bad, wait) {
        (0, 0, 0) => Fact::CiNone,
        (_, b, _) if b > 0 => Fact::CiFailed(b),
        (_, _, w) if w > 0 => Fact::CiPending(w),
        (o, _, _) => Fact::CiPassed(o),
    }
}

const TTL: Duration = Duration::from_secs(60);
const GH_WAIT: Duration = Duration::from_secs(10);

type Cached = Mutex<HashMap<Oid, (Instant, Fact)>>;

fn cache() -> &'static Cached {
    static C: OnceLock<Cached> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// GitHub's check runs for `head`, asked through `gh`. Cached for a minute
/// unless it could not be asked.
fn checks(wt: &Path, branch: &str, head: &Oid) -> Fact {
    if let Some((at, f)) = cache().lock().ok().and_then(|c| c.get(head).cloned()) {
        if at.elapsed() < TTL {
            return f;
        }
    }
    let fact = ask_github(wt, branch, head);
    if !matches!(fact, Fact::CiUnknown(_)) {
        if let Ok(mut c) = cache().lock() {
            c.insert(*head, (Instant::now(), fact.clone()));
        }
    }
    fact
}

fn ask_github(wt: &Path, branch: &str, head: &Oid) -> Fact {
    let Ok(Some(url)) = pr::remote_of(wt, branch).and_then(|r| pr::config(wt, &format!("remote.{r}.url"))) else {
        return Fact::CiNotGithub;
    };
    let Some((owner, repo)) = pr::github_slug(&url) else { return Fact::CiNotGithub };
    let path = format!("repos/{owner}/{repo}/commits/{head}/check-runs");
    let mut c = Command::new("gh");
    c.args(["api", &path, "--jq", "[.check_runs[] | (.conclusion // .status)] | join(\",\")"])
        .env("GH_PROMPT_DISABLED", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    git::hide_window(&mut c);
    let mut child = match c.spawn() {
        Ok(ch) => ch,
        Err(e) => return Fact::CiUnknown(format!("gh: {e}")),
    };
    let (tx, rx) = mpsc::channel();
    let (mut out, mut err) = (child.stdout.take(), child.stderr.take());
    std::thread::spawn(move || {
        let (mut o, mut e) = (String::new(), String::new());
        if let Some(s) = out.as_mut() {
            let _ = s.read_to_string(&mut o);
        }
        if let Some(s) = err.as_mut() {
            let _ = s.read_to_string(&mut e);
        }
        let _ = tx.send((o, e));
    });
    match rx.recv_timeout(GH_WAIT) {
        Ok((o, e)) => match child.wait() {
            Ok(st) if st.success() => parse_checks(&o),
            _ => Fact::CiUnknown(e.lines().find(|l| !l.trim().is_empty()).unwrap_or("gh failed").trim().chars().take(120).collect()),
        },
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            Fact::CiUnknown("timed out".to_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clash::{Outcome, Side};
    use crate::predict::{Conflict, ConflictFile};
    use crate::testrepo::TempRepo;

    fn oid(c: char) -> Oid {
        Oid::parse(&c.to_string().repeat(40)).unwrap()
    }

    fn input() -> Input {
        Input {
            wt: PathBuf::from("/x"),
            branch: "feat".into(),
            head: oid('a'),
            dirty: Some(0),
            clash: Some(Vec::new()),
            behind: Some(0),
            upstream: Some((0, false)),
        }
    }

    #[test]
    fn a_ready_branch_is_ok_all_the_way() {
        let i = input();
        let mut all: Vec<Fact> = known(&i).into();
        all.push(pushed(&i));
        assert!(all.iter().all(|f| f.status() == Status::Ok), "{all:?}");
    }

    #[test]
    fn each_trouble_is_a_warning_and_a_missing_word_is_unknown() {
        let mut i = input();
        i.dirty = Some(3);
        i.clash = Some(vec!["a.txt".into()]);
        i.behind = Some(2);
        i.upstream = Some((4, false));
        let f = known(&i);
        assert_eq!(f, [Fact::Dirty(3), Fact::Clashes(vec!["a.txt".into()]), Fact::Behind(2)]);
        assert!(f.iter().all(|f| f.status() == Status::Warn));
        assert_eq!(pushed(&i), Fact::Unpushed(4));
        i.upstream = None;
        assert_eq!(pushed(&i), Fact::NoUpstream);
        i.upstream = Some((0, true));
        assert_eq!(pushed(&i), Fact::UpstreamGone);
        i.dirty = None;
        i.clash = None;
        i.behind = None;
        let f = known(&i);
        assert_eq!(f, [Fact::TreeUnknown, Fact::ClashUnknown, Fact::BehindUnknown]);
        assert!(f.iter().all(|f| f.status() == Status::Unknown));
    }

    #[test]
    fn items_stay_in_their_places() {
        assert_eq!(Fact::CiPassed(2).item(), Item::Checks);
        assert_eq!(Fact::VerifyNone.item(), Item::Verify);
        assert_eq!(Fact::UpstreamGone.item(), Item::Pushed);
    }

    #[test]
    fn marks_parse_and_unknown_text_is_no_mark() {
        assert_eq!(parse_mark("ok\n"), Some(Mark::Ok));
        assert_eq!(parse_mark("fail 101\n"), Some(Mark::Failed(Some(101))));
        assert_eq!(parse_mark("fail\n"), Some(Mark::Failed(None)));
        assert_eq!(parse_mark("maybe"), None);
        assert_eq!(parse_mark(""), None);
    }

    #[test]
    fn check_runs_are_counted() {
        assert_eq!(parse_checks("success,success,skipped\n"), Fact::CiPassed(3));
        assert_eq!(parse_checks("success,failure,in_progress"), Fact::CiFailed(1));
        assert_eq!(parse_checks("success,queued"), Fact::CiPending(1));
        assert_eq!(parse_checks("\n"), Fact::CiNone);
        assert_eq!(parse_checks("cancelled"), Fact::CiFailed(1));
    }

    #[test]
    fn the_verify_record_belongs_to_the_commit_it_was_made_on() {
        let t = TempRepo::new("ready-mark");
        let head = Oid::parse(&t.out(&t.repo, &["rev-parse", "HEAD"])).unwrap();
        assert_eq!(read_verify(&t.repo, &head), None);
        record_verify(&t.repo, &head, None);
        assert_eq!(read_verify(&t.repo, &head), Some(Mark::Ok));
        record_verify(&t.repo, &head, Some(Some(3)));
        assert_eq!(read_verify(&t.repo, &head), Some(Mark::Failed(Some(3))));
        t.git(&t.repo, &["commit", "-q", "--allow-empty", "-m", "second"]);
        let next = Oid::parse(&t.out(&t.repo, &["rev-parse", "HEAD"])).unwrap();
        assert_eq!(read_verify(&t.repo, &next), None, "a new commit has no result yet");
        // The worktrees of one repository share the record.
        let wt = t.root.join("wt");
        t.git(&t.repo, &["worktree", "add", "-q", "-b", "feat", wt.to_str().unwrap()]);
        assert_eq!(read_verify(&wt, &next), None);
        record_verify(&wt, &next, None);
        assert_eq!(read_verify(&t.repo, &next), Some(Mark::Ok));
    }

    #[test]
    fn a_branch_that_is_not_pushed_or_not_on_github_does_not_ask_github() {
        let t = TempRepo::new("ready-ci");
        let head = Oid::parse(&t.out(&t.repo, &["rev-parse", "HEAD"])).unwrap();
        // No remote at all, or one that is not GitHub: no call is made.
        assert_eq!(ask_github(&t.repo, "main", &head), Fact::CiNotGithub);
        t.git(&t.repo, &["remote", "add", "origin", "https://example.com/a/b.git"]);
        assert_eq!(ask_github(&t.repo, "main", &head), Fact::CiNotGithub);
        let mut i = input();
        i.wt = t.repo.clone();
        i.head = head;
        i.branch = "main".into();
        i.upstream = Some((1, false));
        let all = check(&i);
        assert_eq!(all.len(), 6);
        assert_eq!(all[4], Fact::CiNotPushed);
        assert_eq!(all[3], Fact::VerifyNone);
        assert_eq!(all[5], Fact::Unpushed(1));
    }

    #[test]
    fn the_clash_comes_from_the_pair_with_the_trunk() {
        let side =
            |name: &str, path: Option<&str>, h: char| Side { name: name.into(), path: path.map(PathBuf::from), head: oid(h), tip: oid(h) };
        let mut c = Clashes {
            sides: vec![side("main", None, 'a'), side("feat", Some("/wt"), 'b'), side("other", Some("/o"), 'c')],
            ..Clashes::default()
        };
        c.pairs = vec![
            Outcome::Committed(Conflict { files: vec![ConflictFile { path: "a.txt".into(), hunks: Some(1) }] }),
            Outcome::Clean,
            Outcome::Clean,
        ];
        assert_eq!(clash_of(&c, Path::new("/wt"), "main"), Some(vec!["a.txt".to_owned()]));
        assert_eq!(clash_of(&c, Path::new("/o"), "main"), Some(Vec::new()));
        assert_eq!(clash_of(&c, Path::new("/nowhere"), "main"), None);
        assert_eq!(clash_of(&c, Path::new("/wt"), "gone"), None);
        c.pairs[0] = Outcome::Unknown;
        assert_eq!(clash_of(&c, Path::new("/wt"), "main"), None);
    }
}
