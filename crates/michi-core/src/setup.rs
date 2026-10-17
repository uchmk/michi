//! The first-run guide (issue #116): what stops a beginner before the first
//! commit. git missing or too old, `user.name` / `user.email` empty. Found
//! with read-only git calls (run them off the UI thread); the fixes are text
//! for the user to copy, michi runs none of them. Credentials are explained by
//! the sign-in help (`auth.rs`), which the guide points to.

use std::path::Path;

use crate::git::{self, Version};
use crate::identity::{self, Identity};

/// Something to put right before michi (or git) is of use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Problem {
    /// No `git` on the PATH.
    NoGit,
    /// A git older than [`git::MIN`].
    OldGit(Version),
    /// `user.name` is empty: commits would be refused.
    NoName,
    /// `user.email` is empty.
    NoEmail,
}

/// The OS the install command is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Windows,
    Mac,
    Linux,
}

impl Os {
    pub fn current() -> Os {
        if cfg!(windows) {
            Os::Windows
        } else if cfg!(target_os = "macos") {
            Os::Mac
        } else {
            Os::Linux
        }
    }
}

/// Text to copy, and the word-table key of why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub command: String,
    pub why: &'static str,
}

impl Problem {
    /// The word-table key of the one-line description.
    pub fn word(&self) -> &'static str {
        match self {
            Problem::NoGit => "guide.no_git",
            Problem::OldGit(_) => "guide.old_git",
            Problem::NoName => "guide.no_name",
            Problem::NoEmail => "guide.no_email",
        }
    }

    /// What to copy to put it right on `os`.
    pub fn steps(&self, os: Os) -> Vec<Step> {
        let step = |command: &str, why: &'static str| Step { command: command.to_owned(), why };
        match self {
            Problem::NoGit | Problem::OldGit(_) => {
                let (command, why) = match os {
                    Os::Windows => ("winget install --id Git.Git -e", "guide.why.winget"),
                    Os::Mac => ("brew install git", "guide.why.brew"),
                    Os::Linux => ("sudo apt install git", "guide.why.apt"),
                };
                vec![step(command, why), step("https://git-scm.com/downloads", "guide.why.download")]
            }
            Problem::NoName => vec![step("git config --global user.name \"Your Name\"", "guide.why.name")],
            Problem::NoEmail => vec![step("git config --global user.email \"you@example.com\"", "guide.why.email")],
        }
    }
}

/// What is wrong, from git's version and who git says it commits as (none: it
/// could not be asked). A missing git is the one problem then; an old git does
/// not hide the identity's.
pub fn problems(version: &Result<Version, git::Error>, who: Option<&Identity>) -> Vec<Problem> {
    let mut out = Vec::new();
    match version {
        Err(git::Error::NoGit) => return vec![Problem::NoGit],
        Ok(v) if !v.is_enough() => out.push(Problem::OldGit(*v)),
        _ => {}
    }
    if let Some(who) = who {
        if who.name.is_empty() {
            out.push(Problem::NoName);
        }
        if who.email.is_empty() {
            out.push(Problem::NoEmail);
        }
    }
    out
}

/// [`problems`] for the git on the PATH, asking it who it commits as in `dir`.
/// Runs git: call it off the UI thread.
pub fn check(dir: &Path, version: &Result<Version, git::Error>) -> Vec<Problem> {
    let who = if matches!(version, Err(git::Error::NoGit)) { None } else { identity::read(dir).ok() };
    problems(version, who.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn who(name: &str, email: &str) -> Identity {
        Identity { name: name.to_owned(), email: email.to_owned() }
    }

    #[test]
    fn a_set_up_machine_has_no_problems() {
        assert_eq!(problems(&Ok(Version(2, 47, 1)), Some(&who("A", "a@x.test"))), vec![]);
        assert_eq!(problems(&Ok(Version(2, 47, 1)), None), vec![], "an identity that could not be asked is no problem");
    }

    #[test]
    fn no_git_is_the_only_problem_then() {
        assert_eq!(problems(&Err(git::Error::NoGit), Some(&who("", ""))), vec![Problem::NoGit]);
    }

    #[test]
    fn an_old_git_and_an_empty_identity_are_both_told() {
        assert_eq!(
            problems(&Ok(Version(2, 30, 0)), Some(&who("", ""))),
            vec![Problem::OldGit(Version(2, 30, 0)), Problem::NoName, Problem::NoEmail]
        );
        assert_eq!(problems(&Ok(Version(2, 38, 0)), Some(&who("A", ""))), vec![Problem::NoEmail]);
        assert_eq!(problems(&Ok(Version(2, 38, 0)), Some(&who("", "a@x.test"))), vec![Problem::NoName]);
    }

    #[test]
    fn another_git_error_is_not_a_missing_git() {
        assert_eq!(problems(&Err(git::Error::Io("boom".into())), Some(&who("A", "a@x.test"))), vec![]);
    }

    #[test]
    fn every_problem_has_steps_on_every_os() {
        for p in [Problem::NoGit, Problem::OldGit(Version(2, 1, 0)), Problem::NoName, Problem::NoEmail] {
            for os in [Os::Windows, Os::Mac, Os::Linux] {
                let steps = p.steps(os);
                assert!(!steps.is_empty() && steps.iter().all(|s| !s.command.is_empty() && s.why.starts_with("guide.why.")), "{p:?}");
            }
        }
        assert!(Problem::NoGit.steps(Os::Windows)[0].command.contains("winget"));
        assert!(Problem::NoName.steps(Os::Mac)[0].command.contains("user.name"));
    }
}
