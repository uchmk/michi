//! Creating a pull request from michi with the GitHub CLI (`gh pr create`):
//! the form's defaults, the commands, and running them. The branch is pushed
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

/// The form's starting text for `branch` of the worktree `wt` against `base`.
pub fn defaults(wt: &Path, branch: &str, base: &str) -> Result<Draft, git::Error> {
    let (title, body) = draft_from(branch, &commits(wt, branch, base));
    let push_to = if has_upstream(wt, branch)? { None } else { Some(pr::remote_of(wt, branch)?) };
    Ok(Draft { title, body, push_to })
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

/// Pushes when asked, then creates the pull request; its address.
pub fn create(wt: &Path, req: &Request) -> Result<String, git::Error> {
    if req.branch.trim().is_empty() || req.title.trim().is_empty() || req.base.trim().is_empty() {
        return Err(git::Error::Git("a title, a branch and a base are needed".to_owned()));
    }
    if let Some(remote) = &req.push_to {
        let args = push_args(remote, &req.branch);
        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        git::read(wt, &refs)?;
    }
    let mut c = Command::new("gh");
    c.args(gh_args(req)).current_dir(wt).env("GIT_TERMINAL_PROMPT", "0").env("GH_PROMPT_DISABLED", "1").stdin(Stdio::null());
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
        let line = err.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("gh pr create failed");
        return Err(git::Error::Told { line: line.to_owned(), all: err });
    }
    parse_url(&String::from_utf8_lossy(&out.stdout)).ok_or_else(|| git::Error::Io("gh did not print the address".to_owned()))
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
        }
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
