//! Why a fetch, push or clone was refused for want of a login, and what to
//! do about it. The cause is read from git's standard error; the state it
//! depends on (the remote's address, the credential helper, the ssh agent)
//! is read with read-only commands. Nothing here changes a login, runs a
//! command that does, or reads a secret: the fixes are text for the user to
//! copy. Git never prompts (`GIT_TERMINAL_PROMPT=0`), so every failure comes
//! back as text to classify.

use std::path::Path;
use std::process::Stdio;

use crate::git;

/// What stood in the way, as far as the text says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// ssh got no key it accepts (none loaded, wrong key, no agent).
    SshKey,
    /// ssh does not know the host's key, or it changed.
    HostKey,
    /// https asked for a login and none was given or it was wrong.
    HttpsAuth,
    /// The token or password was once good and is not any more.
    TokenExpired,
    /// The organization wants the token or key authorized for SSO.
    Sso,
    /// The host could not be reached at all.
    Network,
}

impl Kind {
    /// The word the i18n tables use for it.
    pub fn word(self) -> &'static str {
        match self {
            Kind::SshKey => "ssh_key",
            Kind::HostKey => "host_key",
            Kind::HttpsAuth => "https",
            Kind::TokenExpired => "expired",
            Kind::Sso => "sso",
            Kind::Network => "network",
        }
    }
}

/// The kind of failure git's standard error `stderr` tells of; none when it
/// is not about logging in or reaching the host.
pub fn classify(stderr: &str) -> Option<Kind> {
    let s = stderr.to_ascii_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| s.contains(w));
    if has(&["saml", "single sign-on"]) {
        return Some(Kind::Sso);
    }
    if has(&["host key verification failed", "host key is known", "remote host identification has changed", "no matching host key"]) {
        return Some(Kind::HostKey);
    }
    if has(&["permission denied (publickey", "agent refused operation", "authentication agent", "enter passphrase"]) {
        return Some(Kind::SshKey);
    }
    if has(&["token expired", "token has expired", "has expired", "bad credentials", "has been revoked"]) {
        return Some(Kind::TokenExpired);
    }
    if has(&[
        "could not read username",
        "could not read password",
        "authentication failed",
        "invalid username or password",
        "invalid username or token",
        "password authentication is not supported",
        "support for password authentication was removed",
        "http basic: access denied",
        "returned error: 401",
        "returned error: 403",
        "terminal prompts disabled",
    ]) {
        return Some(Kind::HttpsAuth);
    }
    if has(&[
        "could not resolve host",
        "connection refused",
        "connection timed out",
        "operation timed out",
        "network is unreachable",
        "failed to connect to",
        "connection reset",
        "ssh: connect to host",
        "no route to host",
    ]) {
        return Some(Kind::Network);
    }
    None
}

/// The kind of failure `e` tells of.
pub fn classify_error(e: &git::Error) -> Option<Kind> {
    classify(&e.all())
}

/// How the remote is reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    Ssh,
    Https,
    /// `git://`, a path, anything else.
    Other,
    /// No remote.
    None,
}

/// The scheme of a remote address; the scp form `user@host:path` is ssh.
pub fn scheme_of(url: &str) -> Scheme {
    let url = url.trim();
    if url.is_empty() {
        Scheme::None
    } else if url.starts_with("https://") || url.starts_with("http://") {
        Scheme::Https
    } else if url.starts_with("ssh://") || (!is_drive_path(url) && !url.contains("://") && url.contains('@') && url.contains(':')) {
        Scheme::Ssh
    } else {
        Scheme::Other
    }
}

/// `C:\x` or `C:/x`: a Windows path, which may hold an `@` and is no scp address.
fn is_drive_path(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/')
}

/// What the ssh agent holds, as `ssh-add -l` tells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Agent {
    /// The agent answers and holds this many keys.
    Keys(usize),
    /// The agent answers and holds none.
    Empty,
    /// No agent to talk to.
    Absent,
    /// `ssh-add` is not installed, or not asked (an https remote).
    Missing,
}

/// The state the fix depends on. Never holds a secret: the address has its
/// user and password taken off, and the helper is only named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Facts {
    pub remote: Option<String>,
    pub scheme: Scheme,
    pub host: Option<String>,
    /// The first `credential.helper` (its name, not its arguments); none when unset.
    pub helper: Option<String>,
    pub agent: Agent,
}

impl Facts {
    /// The host to name in a command: the remote's, else GitHub's.
    ///
    /// The fixes are lines for the user to paste into a shell, so a host
    /// with anything but a host name's characters in it (a remote address
    /// is whatever a repository's config says) is not used.
    pub fn host_or_default(&self) -> &str {
        self.host
            .as_deref()
            .filter(|h| !h.starts_with('-') && h.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')))
            .unwrap_or("github.com")
    }
}

/// Reads the facts for the repository at `dir`, or for `url` when a clone's
/// address is given. Only reads: `git remote`, `git config`, `ssh-add -l`.
pub fn probe(dir: &Path, url: Option<&str>) -> Facts {
    let (remote, address) = match url {
        Some(u) => (None, Some(u.trim().to_owned())),
        None => {
            let name = remote_name(dir);
            let address = name.as_deref().and_then(|n| git::read_text(dir, &["remote", "get-url", n]).ok()).map(|s| s.trim().to_owned());
            (name, address)
        }
    };
    let address = address.filter(|a| !a.is_empty());
    let scheme = address.as_deref().map_or(Scheme::None, scheme_of);
    let host = address.as_deref().and_then(|a| crate::pr::host_and_path(a).map(|(h, _)| h).or_else(|| bare_host(a)));
    let helper = git::read_text(dir, &["config", "--get-all", "credential.helper"])
        .ok()
        .and_then(|t| t.lines().map(helper_name).find(|n| !n.is_empty()));
    let agent = if matches!(scheme, Scheme::Ssh | Scheme::None) { agent() } else { Agent::Missing };
    Facts { remote, scheme, host, helper, agent }
}

/// `origin`, else the first remote.
fn remote_name(dir: &Path) -> Option<String> {
    let all = git::read_text(dir, &["remote"]).ok()?;
    let names: Vec<&str> = all.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    names.iter().find(|n| **n == "origin").or(names.first()).map(|n| (*n).to_owned())
}

/// The host of an address whose path part is empty (`ssh://git@host`).
fn bare_host(url: &str) -> Option<String> {
    let rest = url.trim().split_once("://")?.1;
    let host = rest.split_once('@').map_or(rest, |(_, h)| h);
    let host = host.split(['/', ':']).next()?;
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

/// The name of a helper line: `manager`, not `manager --some-arg`; a shell
/// helper (`!…`) is only called `custom`, as its text could hold anything.
fn helper_name(line: &str) -> String {
    let l = line.trim();
    if l.starts_with('!') {
        return "custom".to_owned();
    }
    l.split_whitespace().next().unwrap_or("").chars().take(40).collect()
}

/// `ssh-add -l`: what the agent holds.
fn agent() -> Agent {
    let mut c = std::process::Command::new("ssh-add");
    c.arg("-l").stdin(Stdio::null());
    git::hide_window(&mut c);
    match c.output() {
        Err(_) => Agent::Missing,
        Ok(o) => agent_from(o.status.code(), &String::from_utf8_lossy(&o.stdout), &String::from_utf8_lossy(&o.stderr)),
    }
}

/// What `ssh-add -l` answering `code` with `out` and `err` means.
pub fn agent_from(code: Option<i32>, out: &str, err: &str) -> Agent {
    let e = err.to_ascii_lowercase();
    if code == Some(0) {
        return Agent::Keys(out.lines().filter(|l| !l.trim().is_empty()).count());
    }
    if code == Some(2) || e.contains("connect") || e.contains("could not open") || e.contains("no such file") {
        return Agent::Absent;
    }
    Agent::Empty
}

/// A line to copy: the command, and the word table key that says what it does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fix {
    pub command: String,
    pub why: &'static str,
}

fn fix(command: impl Into<String>, why: &'static str) -> Fix {
    Fix { command: command.into(), why }
}

/// The commands that cure `kind`, given what is true now. None changes a
/// login by itself: each is for the user to look at and run.
pub fn fixes(kind: Kind, f: &Facts) -> Vec<Fix> {
    let host = f.host_or_default();
    let ssh_t = format!("ssh -T git@{host}");
    match kind {
        Kind::SshKey => {
            let mut v = Vec::new();
            if f.agent == Agent::Absent {
                v.push(if cfg!(windows) {
                    fix("Start-Service ssh-agent", "auth.why.start_agent_win")
                } else {
                    fix("eval \"$(ssh-agent -s)\"", "auth.why.start_agent")
                });
            }
            if !matches!(f.agent, Agent::Keys(_)) {
                v.push(fix("ssh-add", "auth.why.add_key"));
            }
            v.push(fix("ssh-add -l", "auth.why.list_keys"));
            v.push(fix(ssh_t, "auth.why.test_ssh"));
            v
        }
        Kind::HostKey => vec![fix(ssh_t, "auth.why.accept_host"), fix(format!("ssh-keygen -R {host}"), "auth.why.forget_host")],
        Kind::HttpsAuth => {
            let mut v = Vec::new();
            if f.helper.is_none() {
                let helper = if cfg!(windows) {
                    "manager"
                } else if cfg!(target_os = "macos") {
                    "osxkeychain"
                } else {
                    "cache"
                };
                v.push(fix(format!("git config --global credential.helper {helper}"), "auth.why.set_helper"));
            }
            v.push(fix("gh auth login", "auth.why.gh_login"));
            v.push(fix("gh auth setup-git", "auth.why.gh_setup"));
            v
        }
        Kind::TokenExpired => {
            let mut v = vec![fix("gh auth status", "auth.why.gh_status"), fix("gh auth refresh", "auth.why.gh_refresh")];
            if cfg!(windows) {
                v.push(fix(format!("cmdkey /delete:git:https://{host}"), "auth.why.forget_cred"));
            }
            v
        }
        Kind::Sso => vec![fix("gh auth status", "auth.why.gh_status"), fix("gh auth refresh", "auth.why.gh_refresh")],
        Kind::Network => {
            let mut v = Vec::new();
            if let Some(r) = &f.remote {
                v.push(fix(format!("git ls-remote --heads {r}"), "auth.why.ls_remote"));
            }
            v.push(fix("git config --get-regexp \"^https?\\.proxy$\"", "auth.why.proxy"));
            v
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(s: &str) -> Option<Kind> {
        classify(s)
    }

    #[test]
    fn ssh_key_refused() {
        let denied = "git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.\n\nPlease make sure you have the correct access rights\nand the repository exists.";
        assert_eq!(kind(denied), Some(Kind::SshKey));
        let agent = "sign_and_send_pubkey: signing failed for ED25519 \"/home/a/.ssh/id\" from agent: agent refused operation\ngit@github.com: Permission denied (publickey).";
        assert_eq!(kind(agent), Some(Kind::SshKey));
        assert_eq!(
            kind("Enter passphrase for key '/home/a/.ssh/id_ed25519': \nfatal: Could not read from remote repository."),
            Some(Kind::SshKey)
        );
        assert_eq!(kind("Permission denied (publickey,keyboard-interactive)."), Some(Kind::SshKey));
    }

    #[test]
    fn host_key_unknown_or_changed() {
        assert_eq!(kind("Host key verification failed.\nfatal: Could not read from remote repository."), Some(Kind::HostKey));
        assert_eq!(
            kind("@@@@ WARNING: REMOTE HOST IDENTIFICATION HAS CHANGED! @@@@\nOffending ECDSA key in /home/a/.ssh/known_hosts:3"),
            Some(Kind::HostKey)
        );
        assert_eq!(
            kind("No ED25519 host key is known for github.com and you have requested strict checking.\nHost key verification failed."),
            Some(Kind::HostKey)
        );
    }

    #[test]
    fn https_login_failed() {
        assert_eq!(kind("fatal: could not read Username for 'https://github.com': terminal prompts disabled"), Some(Kind::HttpsAuth));
        assert_eq!(
            kind("remote: Invalid username or password.\nfatal: Authentication failed for 'https://github.com/uchmk/michi.git/'"),
            Some(Kind::HttpsAuth)
        );
        assert_eq!(
            kind("remote: Support for password authentication was removed on August 13, 2021.\nfatal: Authentication failed for 'https://github.com/a/b.git/'"),
            Some(Kind::HttpsAuth)
        );
        assert_eq!(
            kind("remote: HTTP Basic: Access denied\nfatal: Authentication failed for 'https://gitlab.com/a/b.git/'"),
            Some(Kind::HttpsAuth)
        );
        assert_eq!(
            kind("fatal: unable to access 'https://github.com/a/b.git/': The requested URL returned error: 403"),
            Some(Kind::HttpsAuth)
        );
    }

    #[test]
    fn token_expired() {
        assert_eq!(
            kind("remote: Bad credentials\nfatal: Authentication failed for 'https://github.com/a/b.git/'"),
            Some(Kind::TokenExpired)
        );
        assert_eq!(kind("remote: The token has expired\nfatal: Authentication failed"), Some(Kind::TokenExpired));
    }

    #[test]
    fn sso_required() {
        let saml = "remote: Resource protected by organization SAML enforcement. You must grant your Personal Access token access to this organization.\nfatal: unable to access 'https://github.com/org/r.git/': The requested URL returned error: 403";
        assert_eq!(kind(saml), Some(Kind::Sso));
        let ssh = "ERROR: The 'org' organization has enabled or enforced SAML SSO. To access this repository, you must use the HTTPS remote with a personal access token or SSH with an SSH key and passphrase that has been authorized for this organization.\nfatal: Could not read from remote repository.";
        assert_eq!(kind(ssh), Some(Kind::Sso));
    }

    #[test]
    fn network_trouble() {
        assert_eq!(kind("fatal: unable to access 'https://github.com/a/b.git/': Could not resolve host: github.com"), Some(Kind::Network));
        assert_eq!(
            kind("ssh: connect to host github.com port 22: Connection refused\nfatal: Could not read from remote repository."),
            Some(Kind::Network)
        );
        assert_eq!(kind("ssh: connect to host github.com port 22: Connection timed out"), Some(Kind::Network));
        assert_eq!(
            kind("fatal: unable to access 'https://h/a.git/': Failed to connect to h port 443 after 21000 ms: Couldn't connect to server"),
            Some(Kind::Network)
        );
    }

    #[test]
    fn other_failures_are_not_auth() {
        assert_eq!(
            kind(
                "error: failed to push some refs to 'origin'\nhint: Updates were rejected because the tip of your current branch is behind"
            ),
            None
        );
        assert_eq!(kind("fatal: refusing to merge unrelated histories"), None);
        assert_eq!(kind(""), None);
    }

    #[test]
    fn schemes_and_hosts() {
        assert_eq!(scheme_of("git@github.com:uchmk/michi.git"), Scheme::Ssh);
        assert_eq!(scheme_of("ssh://git@host/a/b"), Scheme::Ssh);
        assert_eq!(scheme_of("https://github.com/a/b"), Scheme::Https);
        assert_eq!(scheme_of("git://host/a"), Scheme::Other);
        assert_eq!(scheme_of("C:\\dev\\x"), Scheme::Other);
        assert_eq!(scheme_of(""), Scheme::None);
        assert_eq!(scheme_of("C:\\dev\\a@b"), Scheme::Other);
        let facts = |host: &str| Facts { remote: None, scheme: Scheme::Ssh, host: Some(host.into()), helper: None, agent: Agent::Missing };
        assert_eq!(facts("git.example.com").host_or_default(), "git.example.com");
        assert_eq!(facts("h; rm -rf ~").host_or_default(), "github.com");
        assert_eq!(facts("-oProxyCommand=x").host_or_default(), "github.com");
        assert_eq!(bare_host("ssh://git@Example.com"), Some("example.com".into()));
    }

    #[test]
    fn helper_names_hold_no_arguments_or_scripts() {
        assert_eq!(helper_name("manager"), "manager");
        assert_eq!(helper_name("store --file /home/a/.secret"), "store");
        assert_eq!(helper_name("!f() { echo password=hunter2; }; f"), "custom");
        assert_eq!(helper_name("  "), "");
    }

    #[test]
    fn agent_states() {
        assert_eq!(agent_from(Some(0), "256 SHA256:x a (ED25519)\n256 SHA256:y b (RSA)\n", ""), Agent::Keys(2));
        assert_eq!(agent_from(Some(1), "The agent has no identities.\n", ""), Agent::Empty);
        assert_eq!(agent_from(Some(2), "", "Could not open a connection to your authentication agent."), Agent::Absent);
        assert_eq!(agent_from(Some(1), "", "Error connecting to agent: No such file or directory"), Agent::Absent);
    }

    fn facts(scheme: Scheme, helper: Option<&str>, agent: Agent) -> Facts {
        Facts { remote: Some("origin".into()), scheme, host: Some("github.com".into()), helper: helper.map(str::to_owned), agent }
    }

    #[test]
    fn fixes_follow_the_facts_and_change_nothing_themselves() {
        let none = fixes(Kind::SshKey, &facts(Scheme::Ssh, None, Agent::Empty));
        assert!(none.iter().any(|f| f.command == "ssh-add") && none.iter().any(|f| f.command == "ssh -T git@github.com"));
        let held = fixes(Kind::SshKey, &facts(Scheme::Ssh, None, Agent::Keys(1)));
        assert!(!held.iter().any(|f| f.command == "ssh-add"), "a key is loaded: the problem is which key");
        let bare = fixes(Kind::HttpsAuth, &facts(Scheme::Https, None, Agent::Missing));
        assert!(bare.iter().any(|f| f.command.starts_with("git config --global credential.helper ")));
        let has = fixes(Kind::HttpsAuth, &facts(Scheme::Https, Some("manager"), Agent::Missing));
        assert!(!has.iter().any(|f| f.command.contains("credential.helper")));
        assert!(has.iter().any(|f| f.command == "gh auth login"));
        let host = fixes(Kind::HostKey, &facts(Scheme::Ssh, None, Agent::Keys(1)));
        assert!(host.iter().any(|f| f.command == "ssh-keygen -R github.com"));
        for k in [Kind::SshKey, Kind::HostKey, Kind::HttpsAuth, Kind::TokenExpired, Kind::Sso, Kind::Network] {
            assert!(!fixes(k, &facts(Scheme::Ssh, None, Agent::Empty)).is_empty(), "{k:?}");
        }
    }
}
