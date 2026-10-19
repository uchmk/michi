//! Creating a pull request from michi with the GitHub CLI (`gh pr create`),
//! or updating the open one of the branch (`gh pr edit`): the form's
//! defaults, the commands, and running them. The branch is pushed
//! first when it has no upstream (the caller asks before that: a push goes
//! outside the machine). Run [`create`] and [`defaults`] on a thread.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::git;
use crate::pr;

/// The text [`create`] gives when there is no `gh`.
pub const NO_GH: &str = "gh (the GitHub CLI) was not found on the PATH";
/// The text [`create`] gives when `gh` is not signed in.
pub const NOT_LOGGED_IN: &str = "gh is not signed in";

/// Why a creation failed, as far as the screen words it differently.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    NoGh,
    NotLoggedIn,
    Other,
}

/// Tells the error of [`create`] apart.
pub fn failure_of(e: &git::Error) -> Failure {
    match e {
        git::Error::Git(t) if t == NO_GH => Failure::NoGh,
        git::Error::Git(t) if t == NOT_LOGGED_IN => Failure::NotLoggedIn,
        _ => Failure::Other,
    }
}

/// What the form starts with.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Draft {
    pub title: String,
    pub body: String,
    /// The remote the branch has to be pushed to first: none when it has an
    /// upstream already.
    pub push_to: Option<String>,
    /// The open pull request of the branch, if there is one: the form then
    /// updates it.
    pub existing: Option<Existing>,
}

/// An open pull request, as `gh pr view` tells it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Existing {
    pub url: String,
    pub title: String,
    pub body: String,
    pub base: String,
    pub draft: bool,
}

/// What to create.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub branch: String,
    pub base: String,
    pub title: String,
    pub body: String,
    pub draft: bool,
    /// Push the branch to this remote (with `-u`) first.
    pub push_to: Option<String>,
    /// Update this pull request instead of creating one.
    pub edit: Option<Existing>,
}

/// The arguments of `gh pr edit` for `req` on the pull request `old`.
pub fn edit_args(req: &Request, old: &Existing) -> Vec<String> {
    ["pr", "edit", &old.url, "--base", &req.base, "--title", &req.title, "--body", &req.body].iter().map(|s| (*s).to_owned()).collect()
}

/// The arguments of `gh pr ready` that make `old` a draft or ready, when
/// `req` says the other.
pub fn ready_args(req: &Request, old: &Existing) -> Option<Vec<String>> {
    if req.draft == old.draft {
        return None;
    }
    let mut a: Vec<String> = ["pr", "ready", &old.url].iter().map(|s| (*s).to_owned()).collect();
    if req.draft {
        a.push("--undo".to_owned());
    }
    Some(a)
}

/// The open pull request in `gh pr view --json url,title,body,baseRefName,isDraft,state`'s `json`.
pub fn parse_existing(json: &str) -> Option<Existing> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    if v.get("state")?.as_str()? != "OPEN" {
        return None;
    }
    let text = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_owned();
    Some(Existing {
        url: text("url"),
        title: text("title"),
        body: text("body"),
        base: text("baseRefName"),
        draft: v.get("isDraft")?.as_bool()?,
    })
}

/// The arguments of `gh` for `req`.
pub fn gh_args(req: &Request) -> Vec<String> {
    let mut a: Vec<String> = ["pr", "create", "--head", &req.branch, "--base", &req.base, "--title", &req.title, "--body", &req.body]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    if req.draft {
        a.push("--draft".to_owned());
    }
    a
}

/// The arguments of `git` that push `branch` to `remote` and make it follow.
pub fn push_args(remote: &str, branch: &str) -> Vec<String> {
    ["push", "-u", remote, branch].iter().map(|s| (*s).to_owned()).collect()
}

/// The title and body for `branch` from its `commits` (`(subject, body)`,
/// oldest first): the title is the subject of the last one (the branch's name
/// when there is none), the body the subjects as a list (a lone commit gives
/// its own body).
pub fn draft_from(branch: &str, commits: &[(String, String)]) -> (String, String) {
    let Some(last) = commits.last() else { return (branch.to_owned(), String::new()) };
    let title = if last.0.trim().is_empty() { branch.to_owned() } else { last.0.trim().to_owned() };
    let body = match commits {
        [one] => one.1.trim().to_owned(),
        many => many.iter().map(|(s, _)| format!("- {}", s.trim())).collect::<Vec<_>>().join("\n"),
    };
    (title, body)
}

/// The commits of `branch` that `base` lacks, oldest first, as `(subject,
/// body)`. `base` is tried as it is and then on `origin`.
fn commits(wt: &Path, branch: &str, base: &str) -> Vec<(String, String)> {
    for b in [base.to_owned(), format!("origin/{base}")] {
        let range = format!("{b}..{branch}");
        let Ok(out) = git::read(wt, &["log", "--reverse", "--no-merges", "--format=%s%x1f%b%x1e", &range, "--"]) else { continue };
        let text = String::from_utf8_lossy(&out).into_owned();
        return text
            .split('\x1e')
            .filter_map(|r| {
                let r = r.trim_start_matches(['\n', '\r']);
                let (s, body) = r.split_once('\x1f')?;
                Some((s.to_owned(), body.to_owned()))
            })
            .collect();
    }
    Vec::new()
}

/// Whether `branch` follows a remote branch.
fn has_upstream(wt: &Path, branch: &str) -> Result<bool, git::Error> {
    let key = format!("branch.{branch}.merge");
    Ok(git::read_maybe(wt, &["config", "--get", &key])?.is_some_and(|v| !String::from_utf8_lossy(&v).trim().is_empty()))
}

/// The open pull request of `branch`, if `gh` is there, signed in and finds
/// one; anything else is no pull request.
fn existing(wt: &Path, branch: &str) -> Option<Existing> {
    let mut c = Command::new("gh");
    c.args(["pr", "view", branch, "--json", "url,title,body,baseRefName,isDraft,state"])
        .current_dir(wt)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GH_PROMPT_DISABLED", "1")
        .stdin(Stdio::null());
    git::hide_window(&mut c);
    let out = c.output().ok().filter(|o| o.status.success())?;
    parse_existing(&String::from_utf8_lossy(&out.stdout))
}

/// The form's starting text for `branch` of the worktree `wt` against `base`:
/// the open pull request's own, when there is one.
pub fn defaults(wt: &Path, branch: &str, base: &str) -> Result<Draft, git::Error> {
    let (title, body) = draft_from(branch, &commits(wt, branch, base));
    let push_to = if has_upstream(wt, branch)? { None } else { Some(pr::remote_of(wt, branch)?) };
    let existing = existing(wt, branch);
    Ok(Draft { title, body, push_to, existing })
}

/// Runs `gh` with `args` in `wt`; what it printed.
fn gh(wt: &Path, args: &[String]) -> Result<String, git::Error> {
    let mut c = Command::new("gh");
    c.args(args).current_dir(wt).env("GIT_TERMINAL_PROMPT", "0").env("GH_PROMPT_DISABLED", "1").stdin(Stdio::null());
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
        if says_not_logged_in(&err) {
            return Err(git::Error::Git(NOT_LOGGED_IN.to_owned()));
        }
        let line = err.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("gh failed");
        return Err(git::Error::Told { line: line.to_owned(), all: err });
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The address `gh pr create` printed: its last line that is one.
pub fn parse_url(stdout: &str) -> Option<String> {
    stdout.lines().rev().map(str::trim).find(|l| l.starts_with("https://") || l.starts_with("http://")).map(str::to_owned)
}

/// Whether what `gh` wrote on standard error says it is not signed in.
pub fn says_not_logged_in(stderr: &str) -> bool {
    let s = stderr.to_ascii_lowercase();
    s.contains("gh auth login") || s.contains("not logged in") || s.contains("authentication required") || s.contains("gh_token")
}

/// Pushes when asked, then creates the pull request or updates the open one;
/// its address.
pub fn create(wt: &Path, req: &Request) -> Result<String, git::Error> {
    if req.branch.trim().is_empty() || req.title.trim().is_empty() || req.base.trim().is_empty() {
        return Err(git::Error::Git("a title, a branch and a base are needed".to_owned()));
    }
    if let Some(remote) = &req.push_to {
        let args = push_args(remote, &req.branch);
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        git::read(wt, &refs)?;
    }
    if let Some(old) = &req.edit {
        let printed = gh(wt, &edit_args(req, old))?;
        if let Some(a) = ready_args(req, old) {
            gh(wt, &a)?;
        }
        return Ok(parse_url(&printed).unwrap_or_else(|| old.url.clone()));
    }
    let printed = gh(wt, &gh_args(req))?;
    parse_url(&printed).ok_or_else(|| git::Error::Io("gh did not print the address".to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn req() -> Request {
        Request {
            branch: "feat/x".into(),
            base: "main".into(),
            title: "Add x".into(),
            body: "- a\n- b".into(),
            draft: false,
            push_to: None,
            edit: None,
        }
    }

    fn old() -> Existing {
        Existing { url: "https://github.com/o/r/pull/7".into(), title: "Old".into(), body: "b".into(), base: "main".into(), draft: false }
    }

    #[test]
    fn an_update_edits_by_address_and_flips_the_draft_only_when_it_changed() {
        let a = edit_args(&req(), &old());
        assert_eq!(
            a,
            [
                "pr",
                "edit",
                "https://github.com/o/r/pull/7",
                "--base",
                "main",
                "--title",
                "Add x",
                "--body",
                "- a
- b"
            ]
        );
        assert_eq!(ready_args(&req(), &old()), None);
        let to_draft = ready_args(&Request { draft: true, ..req() }, &old()).unwrap();
        assert_eq!(to_draft, ["pr", "ready", "https://github.com/o/r/pull/7", "--undo"]);
        let to_ready = ready_args(&req(), &Existing { draft: true, ..old() }).unwrap();
        assert_eq!(to_ready, ["pr", "ready", "https://github.com/o/r/pull/7"]);
    }

    #[test]
    fn only_an_open_pull_request_is_one_to_update() {
        let j = |state: &str| {
            format!(
                r#"{{"url":"https://github.com/o/r/pull/7","title":"T","body":"B","baseRefName":"main","isDraft":true,"state":"{state}"}}"#
            )
        };
        let e = parse_existing(&j("OPEN")).unwrap();
        assert_eq!((e.title.as_str(), e.body.as_str(), e.base.as_str(), e.draft), ("T", "B", "main", true));
        assert_eq!(parse_existing(&j("MERGED")), None);
        assert_eq!(parse_existing("not json"), None);
    }

    #[test]
    fn gh_gets_the_branch_base_title_and_body() {
        let a = gh_args(&req());
        assert_eq!(a, ["pr", "create", "--head", "feat/x", "--base", "main", "--title", "Add x", "--body", "- a\n- b"]);
        let d = gh_args(&Request { draft: true, ..req() });
        assert_eq!(d.last().map(String::as_str), Some("--draft"));
    }

    #[test]
    fn the_push_sets_the_upstream() {
        assert_eq!(push_args("origin", "feat/x"), ["push", "-u", "origin", "feat/x"]);
    }

    #[test]
    fn the_title_is_the_last_subject_and_the_body_lists_the_rest() {
        let c = |s: &str, b: &str| (s.to_owned(), b.to_owned());
        assert_eq!(draft_from("feat/x", &[]), ("feat/x".to_owned(), String::new()));
        assert_eq!(draft_from("f", &[c("Only", "Why.\n")]), ("Only".to_owned(), "Why.".to_owned()));
        assert_eq!(draft_from("f", &[c("One", "x"), c("Two", "")]), ("Two".to_owned(), "- One\n- Two".to_owned()));
    }

    #[test]
    fn the_address_is_the_last_url_line() {
        assert_eq!(parse_url("Creating pull request\n\nhttps://github.com/o/r/pull/7\n").as_deref(), Some("https://github.com/o/r/pull/7"));
        assert_eq!(parse_url("nothing"), None);
    }

    #[test]
    fn sign_in_trouble_is_told_from_the_rest() {
        assert!(says_not_logged_in("To get started with GitHub CLI, please run:  gh auth login"));
        assert!(says_not_logged_in("You are not logged in to any GitHub hosts"));
        assert!(!says_not_logged_in("a pull request for branch x already exists"));
        assert_eq!(failure_of(&git::Error::Git(NO_GH.into())), Failure::NoGh);
        assert_eq!(failure_of(&git::Error::Git(NOT_LOGGED_IN.into())), Failure::NotLoggedIn);
        assert_eq!(failure_of(&git::Error::Git("x".into())), Failure::Other);
    }

    #[test]
    fn defaults_read_the_commits_of_the_branch_and_see_a_missing_upstream() {
        let t = TempRepo::new("prcreate-defaults");
        t.git(&t.repo, &["checkout", "-q", "-b", "feat/x"]);
        t.git(&t.repo, &["commit", "-q", "--allow-empty", "-m", "Add parser"]);
        t.git(&t.repo, &["commit", "-q", "--allow-empty", "-m", "Fix parser edge"]);
        let d = defaults(&t.repo, "feat/x", "main").unwrap();
        assert_eq!(d.title, "Fix parser edge");
        assert_eq!(d.body, "- Add parser\n- Fix parser edge");
        assert_eq!(d.push_to.as_deref(), Some("origin"));
        t.git(&t.repo, &["remote", "add", "origin", "https://github.com/o/r.git"]);
        t.git(&t.repo, &["config", "branch.feat/x.remote", "origin"]);
        t.git(&t.repo, &["config", "branch.feat/x.merge", "refs/heads/feat/x"]);
        assert_eq!(defaults(&t.repo, "feat/x", "main").unwrap().push_to, None);
        // No commits of its own: the branch's name.
        t.git(&t.repo, &["branch", "empty", "main"]);
        assert_eq!(defaults(&t.repo, "empty", "main").unwrap().title, "empty");
    }
}
