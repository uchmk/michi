//! Sign-in help (`K`): when a fetch, push or clone is refused for want of a
//! login (or cannot reach the host), a toast says the cause and `K` opens a
//! small panel with the exact fix for that kind. The panel reads the state it
//! depends on (the remote's scheme, the credential helper, the ssh agent) off
//! the UI thread; `1`-`9` copy a command. michi never runs a command that
//! changes a login and never reads a secret. The work is `michi_core::auth`;
//! auth_ui.rs draws the panel.

use std::path::PathBuf;

use michi_core::auth::{self, Facts, Fix, Kind, Scheme};
use michi_core::git;

use crate::act::Task;
use crate::app::{App, Repo};

/// The panel while it is open.
pub struct Auth {
    pub kind: Kind,
    /// Whether a failure is behind it (else it shows the checks for the remote's scheme).
    pub failed: bool,
    pub facts: Facts,
    pub fixes: Vec<Fix>,
}

/// The word-table key of the one-line cause of `kind`.
pub fn cause_key(kind: Kind) -> &'static str {
    match kind {
        Kind::SshKey => "auth.cause.ssh_key",
        Kind::HostKey => "auth.cause.host_key",
        Kind::HttpsAuth => "auth.cause.https",
        Kind::TokenExpired => "auth.cause.expired",
        Kind::Sso => "auth.cause.sso",
        Kind::Network => "auth.cause.network",
    }
}

/// The word-table key of the longer explanation of `kind`.
pub fn help_key(kind: Kind) -> &'static str {
    match kind {
        Kind::SshKey => "auth.help.ssh_key",
        Kind::HostKey => "auth.help.host_key",
        Kind::HttpsAuth => "auth.help.https",
        Kind::TokenExpired => "auth.help.expired",
        Kind::Sso => "auth.help.sso",
        Kind::Network => "auth.help.network",
    }
}

impl App {
    /// A fetch, push or clone failed with `e`: remember the kind of login
    /// trouble it tells of (the clone's `url` for a clone) and return what to
    /// add to the toast, or nothing when it is not about logging in.
    pub(crate) fn auth_noted(&mut self, e: &git::Error, url: Option<String>) -> String {
        let Some(kind) = auth::classify_error(e) else { return String::new() };
        self.auth_failed = Some((kind, url));
        format!(" — {} ({})", self.tr(cause_key(kind)), self.tr("auth.hint"))
    }

    /// `K`: read the facts off the UI thread, then open the panel.
    pub(crate) fn open_auth(&mut self) {
        let here = match &self.repo {
            Repo::Ready(s) => s.top.clone(),
            _ => PathBuf::from("."),
        };
        let url = self.auth_failed.as_ref().and_then(|(_, u)| u.clone());
        self.act(Task::FindAuth { here, url });
    }

    /// The facts have come: open the panel for the last failure, or for the
    /// remote's scheme when there was none.
    pub(crate) fn auth_found(&mut self, facts: Facts) {
        let failed = self.auth_failed.as_ref().map(|(k, _)| *k);
        let kind = failed.unwrap_or(match facts.scheme {
            Scheme::Https => Kind::HttpsAuth,
            Scheme::Ssh => Kind::SshKey,
            Scheme::Other | Scheme::None => Kind::Network,
        });
        let fixes = auth::fixes(kind, &facts);
        self.auth = Some(Auth { kind, failed: failed.is_some(), facts, fixes });
    }

    /// A character while the panel is open: `1`-`9` copy that command.
    pub(crate) fn auth_key(&mut self, c: char) -> bool {
        let Some(n) = c.to_digit(10).filter(|n| *n >= 1) else { return true };
        let Some(command) = self.auth.as_ref().and_then(|a| a.fixes.get(n as usize - 1)).map(|f| f.command.clone()) else { return true };
        let text = self.tr("auth.copied").replace("{}", &command);
        self.clipboard = Some(command);
        self.say(text);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::{app, toast};
    use michi_core::auth::Agent;

    fn facts(scheme: Scheme) -> Facts {
        Facts { remote: Some("origin".into()), scheme, host: Some("github.com".into()), helper: None, agent: Agent::Empty }
    }

    #[test]
    fn a_login_failure_is_remembered_and_named_in_the_toast() {
        let mut a = app();
        let e = git::Error::Told {
            line: "git@github.com: Permission denied (publickey).".into(),
            all: "git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.".into(),
        };
        let note = a.auth_noted(&e, None);
        assert!(note.contains("ssh") && note.contains("K"), "{note}");
        assert_eq!(a.auth_failed.as_ref().map(|f| f.0), Some(Kind::SshKey));
        assert_eq!(a.auth_noted(&git::Error::Git("refusing to merge unrelated histories".into()), None), "");
    }

    #[test]
    fn a_failed_fetch_toast_points_at_the_help() {
        let mut a = app();
        a.net_done(
            crate::work::Job::Fetch,
            Err(git::Error::Git("fatal: could not read Username for 'https://github.com': terminal prompts disabled".into())),
        );
        assert!(toast(&a).contains("K"), "{}", toast(&a));
        assert_eq!(a.auth_failed.as_ref().map(|f| f.0), Some(Kind::HttpsAuth));
    }

    #[test]
    fn the_panel_opens_for_the_last_failure_or_the_scheme() {
        let mut a = app();
        a.auth_found(facts(Scheme::Https));
        let p = a.auth.take().unwrap();
        assert!(!p.failed && p.kind == Kind::HttpsAuth);
        a.auth_failed = Some((Kind::Sso, None));
        a.auth_found(facts(Scheme::Https));
        let p = a.auth.as_ref().unwrap();
        assert!(p.failed && p.kind == Kind::Sso);
    }

    #[test]
    fn a_digit_copies_that_command_and_nothing_runs() {
        let mut a = app();
        a.auth_found(facts(Scheme::Ssh));
        let first = a.auth.as_ref().unwrap().fixes[0].command.clone();
        assert!(a.auth_key('1'));
        assert_eq!(a.clipboard.as_deref(), Some(first.as_str()));
        assert!(toast(&a).contains(&first));
        a.clipboard = None;
        assert!(a.auth_key('9') && a.clipboard.is_none(), "no such row");
        assert!(a.auth_key('x') && a.clipboard.is_none());
    }

    #[test]
    fn escape_closes_it() {
        let mut a = app();
        a.auth_found(facts(Scheme::Ssh));
        assert!(a.escape());
        assert!(a.auth.is_none());
    }
}
