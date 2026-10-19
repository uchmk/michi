//! Watching the pull requests of the worktrees' branches: one `gh pr list`
//! reads them all, and two readings differ by what the screen tells (merged,
//! closed, the checks came out, a review came in). No egui here; the timing
//! and the notices are in the window.

use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

use crate::git;
use crate::prcreate::{NOT_LOGGED_IN, NO_GH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Open,
    Merged,
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Checks {
    /// None run (or the repository has none).
    None,
    Pending,
    Passed,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Review {
    /// Not decided (nobody reviewed, or none is asked for).
    Open,
    Approved,
    ChangesRequested,
}

/// A pull request, as far as the watching looks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pr {
    pub number: u64,
    pub branch: String,
    pub url: String,
    pub state: State,
    pub checks: Checks,
    pub review: Review,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Merged,
    Closed,
    ChecksFailed,
    ChecksPassed,
    Approved,
    ChangesRequested,
}

/// What happened to a branch's pull request between two readings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub branch: String,
    pub number: u64,
    pub url: String,
    pub kind: Kind,
}

/// The pull requests by branch (the newest of a branch when it had several).
pub type Prs = HashMap<String, Pr>;

/// How a check run or status ends up: `Some(true)` failed, `Some(false)`
/// still going, `None` fine or of no weight (skipped, cancelled).
fn fold_item(item: &Value) -> Option<bool> {
    let text = |k: &str| item.get(k).and_then(Value::as_str).unwrap_or("").to_ascii_uppercase();
    // A check run has `status` and `conclusion`; a commit status has `state`.
    let state = text("state");
    if !state.is_empty() {
        return match state.as_str() {
            "FAILURE" | "ERROR" => Some(true),
            "PENDING" | "EXPECTED" => Some(false),
            _ => None,
        };
    }
    if text("status") != "COMPLETED" {
        return Some(false);
    }
    match text("conclusion").as_str() {
        "FAILURE" | "TIMED_OUT" | "ACTION_REQUIRED" | "STARTUP_FAILURE" => Some(true),
        _ => None,
    }
}

/// What `statusCheckRollup` comes to: any failure fails, else anything going
/// is pending.
pub fn fold_checks(rollup: &Value) -> Checks {
    let Some(items) = rollup.as_array().filter(|a| !a.is_empty()) else { return Checks::None };
    let folded: Vec<Option<bool>> = items.iter().map(fold_item).collect();
    if folded.contains(&Some(true)) {
        Checks::Failed
    } else if folded.contains(&Some(false)) {
        Checks::Pending
    } else {
        Checks::Passed
    }
}

/// Reads the output of `gh pr list --json
/// number,headRefName,url,state,reviewDecision,statusCheckRollup`.
pub fn parse_list(json: &str) -> Prs {
    let mut out = Prs::new();
    let Ok(Value::Array(items)) = serde_json::from_str::<Value>(json) else { return out };
    for v in &items {
        let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("");
        let (branch, url) = (s("headRefName"), s("url"));
        let Some(number) = v.get("number").and_then(Value::as_u64) else { continue };
        if branch.is_empty() || url.is_empty() {
            continue;
        }
        let state = match s("state") {
            "MERGED" => State::Merged,
            "CLOSED" => State::Closed,
            _ => State::Open,
        };
        let review = match s("reviewDecision") {
            "APPROVED" => Review::Approved,
            "CHANGES_REQUESTED" => Review::ChangesRequested,
            _ => Review::Open,
        };
        let checks = fold_checks(v.get("statusCheckRollup").unwrap_or(&Value::Null));
        let pr = Pr { number, branch: branch.to_owned(), url: url.to_owned(), state, checks, review };
        match out.get(branch) {
            Some(old) if old.number > number => {}
            _ => {
                out.insert(branch.to_owned(), pr);
            }
        }
    }
    out
}

/// Reads the repository's pull requests with `gh`, run in `dir`.
pub fn fetch(dir: &Path) -> Result<Prs, git::Error> {
    let mut c = Command::new("gh");
    c.args(["pr", "list", "--state", "all", "--limit", "100", "--json", "number,headRefName,url,state,reviewDecision,statusCheckRollup"])
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GH_PROMPT_DISABLED", "1")
        .stdin(Stdio::null());
    git::hide_window(&mut c);
    let out = c.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            git::Error::Git(NO_GH.to_owned())
        } else {
            git::Error::Io(format!("gh: {e}"))
        }
    })?;
    let err = String::from_utf8_lossy(&out.stderr).into_owned();
    if !out.status.success() {
        if crate::prcreate::says_not_logged_in(&err) {
            return Err(git::Error::Git(NOT_LOGGED_IN.to_owned()));
        }
        let line = err.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("gh failed");
        return Err(git::Error::Told { line: line.to_owned(), all: err });
    }
    Ok(parse_list(&String::from_utf8_lossy(&out.stdout)))
}

/// What changed from `old` to `new` for the pull requests of `branches`. A
/// pull request that was not in `old` is not a change (it was just made, by
/// this window or another); neither is a closed one's later change.
pub fn diff(old: &Prs, new: &Prs, branches: &[String]) -> Vec<Change> {
    let mut out = Vec::new();
    for b in branches {
        let (Some(was), Some(now)) = (old.get(b), new.get(b)) else { continue };
        if was.number != now.number || was.state != State::Open {
            continue;
        }
        let mut push = |kind| out.push(Change { branch: b.clone(), number: now.number, url: now.url.clone(), kind });
        match now.state {
            State::Merged => push(Kind::Merged),
            State::Closed => push(Kind::Closed),
            State::Open => {
                if now.checks != was.checks {
                    match now.checks {
                        Checks::Failed => push(Kind::ChecksFailed),
                        Checks::Passed => push(Kind::ChecksPassed),
                        _ => {}
                    }
                }
                if now.review != was.review {
                    match now.review {
                        Review::Approved => push(Kind::Approved),
                        Review::ChangesRequested => push(Kind::ChangesRequested),
                        Review::Open => {}
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = r#"[
      {"number":3,"headRefName":"feat","url":"https://github.com/o/r/pull/3","state":"OPEN","reviewDecision":"",
       "statusCheckRollup":[{"status":"COMPLETED","conclusion":"SUCCESS"},{"status":"IN_PROGRESS","conclusion":""}]},
      {"number":1,"headRefName":"feat","url":"https://github.com/o/r/pull/1","state":"CLOSED","reviewDecision":"","statusCheckRollup":[]},
      {"number":2,"headRefName":"fix","url":"https://github.com/o/r/pull/2","state":"MERGED","reviewDecision":"APPROVED","statusCheckRollup":null}
    ]"#;

    fn pr(state: State, checks: Checks, review: Review) -> Pr {
        Pr { number: 3, branch: "feat".into(), url: "u".into(), state, checks, review }
    }
    fn one(p: Pr) -> Prs {
        Prs::from([(p.branch.clone(), p)])
    }
    fn kinds(old: Pr, new: Pr) -> Vec<Kind> {
        diff(&one(old), &one(new), &["feat".to_owned()]).into_iter().map(|c| c.kind).collect()
    }

    #[test]
    fn the_list_is_read_by_branch_and_the_newest_wins() {
        let l = parse_list(LIST);
        assert_eq!(l.len(), 2);
        let feat = &l["feat"];
        assert_eq!((feat.number, feat.state, feat.checks), (3, State::Open, Checks::Pending));
        let fix = &l["fix"];
        assert_eq!((fix.state, fix.review, fix.checks), (State::Merged, Review::Approved, Checks::None));
        assert!(parse_list("not json").is_empty());
    }

    #[test]
    fn checks_fold_failure_over_pending_over_passed() {
        let f = |s: &str| fold_checks(&serde_json::from_str(s).unwrap());
        assert_eq!(f("[]"), Checks::None);
        assert_eq!(f(r#"[{"status":"COMPLETED","conclusion":"SUCCESS"},{"status":"COMPLETED","conclusion":"SKIPPED"}]"#), Checks::Passed);
        assert_eq!(f(r#"[{"status":"QUEUED"},{"status":"COMPLETED","conclusion":"SUCCESS"}]"#), Checks::Pending);
        assert_eq!(f(r#"[{"status":"QUEUED"},{"status":"COMPLETED","conclusion":"FAILURE"}]"#), Checks::Failed);
        assert_eq!(f(r#"[{"state":"SUCCESS"}]"#), Checks::Passed);
        assert_eq!(f(r#"[{"state":"PENDING"}]"#), Checks::Pending);
        assert_eq!(f(r#"[{"state":"ERROR"}]"#), Checks::Failed);
        assert_eq!(f(r#"[{"status":"COMPLETED","conclusion":"CANCELLED"}]"#), Checks::Passed);
    }

    #[test]
    fn what_ends_is_told() {
        use {Checks as C, Review as R, State as S};
        assert_eq!(kinds(pr(S::Open, C::Pending, R::Open), pr(S::Merged, C::Passed, R::Approved)), [Kind::Merged]);
        assert_eq!(kinds(pr(S::Open, C::Pending, R::Open), pr(S::Closed, C::Pending, R::Open)), [Kind::Closed]);
        assert_eq!(kinds(pr(S::Open, C::Pending, R::Open), pr(S::Open, C::Failed, R::Open)), [Kind::ChecksFailed]);
        assert_eq!(kinds(pr(S::Open, C::Pending, R::Open), pr(S::Open, C::Passed, R::Open)), [Kind::ChecksPassed]);
        assert_eq!(kinds(pr(S::Open, C::Passed, R::Open), pr(S::Open, C::Passed, R::Approved)), [Kind::Approved]);
        assert_eq!(kinds(pr(S::Open, C::Passed, R::Approved), pr(S::Open, C::Passed, R::ChangesRequested)), [Kind::ChangesRequested]);
        assert_eq!(
            kinds(pr(S::Open, C::Pending, R::Open), pr(S::Open, C::Failed, R::ChangesRequested)),
            [Kind::ChecksFailed, Kind::ChangesRequested]
        );
    }

    #[test]
    fn the_quiet_stays_quiet() {
        use {Checks as C, Review as R, State as S};
        // Nothing moved; checks going; a review taken back.
        assert!(kinds(pr(S::Open, C::Passed, R::Open), pr(S::Open, C::Passed, R::Open)).is_empty());
        assert!(kinds(pr(S::Open, C::Passed, R::Open), pr(S::Open, C::Pending, R::Open)).is_empty());
        assert!(kinds(pr(S::Open, C::Passed, R::Approved), pr(S::Open, C::Passed, R::Open)).is_empty());
        // A closed one that changes later, a new one, a branch nobody has.
        assert!(kinds(pr(S::Merged, C::Passed, R::Open), pr(S::Closed, C::Passed, R::Open)).is_empty());
        assert!(diff(&Prs::new(), &one(pr(State::Merged, Checks::None, Review::Open)), &["feat".to_owned()]).is_empty());
        let both = one(pr(State::Open, Checks::Pending, Review::Open));
        let done = one(pr(State::Merged, Checks::Pending, Review::Open));
        assert!(diff(&both, &done, &["other".to_owned()]).is_empty());
        // Another pull request on the same branch is not a change of this one.
        let mut next = pr(State::Merged, Checks::None, Review::Open);
        next.number = 4;
        assert!(diff(&both, &one(next), &["feat".to_owned()]).is_empty());
    }
}
