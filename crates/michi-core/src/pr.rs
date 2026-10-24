//! Handing a branch to the browser to open a pull request on GitHub, GitLab,
//! Bitbucket or Azure DevOps (told apart by the remote's URL). michi does not
//! create the pull request itself: the host's "compare" page for the branch (the page of the pull request that is already there, when there
//! is one) is where it is written.

use std::path::Path;
use std::process::Command;

use crate::git;

/// `(host, path)` of a remote URL (`https://`, `ssh://`, `git://` and the scp
/// form `git@host:path`); the path has no surrounding `/` or trailing `.git`.
pub(crate) fn host_and_path(url: &str) -> Option<(String, String)> {
    let url = url.trim();
    let rest = ["https://", "http://", "ssh://", "git://"].iter().find_map(|p| url.strip_prefix(p));
    let (host, path) = match rest {
        Some(r) => r.split_once('@').map_or(r, |(_, h)| h).split_once(['/', ':'])?,
        None => url.strip_prefix("git@")?.split_once(':')?,
    };
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    (!host.is_empty() && !path.is_empty()).then(|| (host.to_ascii_lowercase(), path.to_owned()))
}

/// The hosts whose pull request page michi links to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Forge {
    GitHub {
        owner: String,
        repo: String,
    },
    /// GitLab.com; `project` is the whole path (groups and subgroups).
    GitLab {
        project: String,
    },
    Bitbucket {
        owner: String,
        repo: String,
    },
    Azure {
        org: String,
        project: String,
        repo: String,
    },
}

/// The host a remote's URL names; `None` for any other host.
pub fn forge_of(url: &str) -> Option<Forge> {
    let (host, path) = host_and_path(url)?;
    let parts: Vec<&str> = path.split('/').collect();
    if parts.iter().any(|p| p.is_empty()) {
        return None;
    }
    let s = |x: &str| x.to_owned();
    match (host.as_str(), parts.as_slice()) {
        ("github.com", [o, r]) => Some(Forge::GitHub { owner: s(o), repo: s(r) }),
        ("gitlab.com", [_, _, ..]) => Some(Forge::GitLab { project: path.clone() }),
        ("bitbucket.org", [o, r]) => Some(Forge::Bitbucket { owner: s(o), repo: s(r) }),
        ("dev.azure.com", [org, project, "_git", repo]) => Some(Forge::Azure { org: s(org), project: s(project), repo: s(repo) }),
        ("ssh.dev.azure.com", ["v3", org, project, repo]) => Some(Forge::Azure { org: s(org), project: s(project), repo: s(repo) }),
        _ => None,
    }
}

/// `owner/repo` of a GitHub remote's URL; `None` for any other host.
pub fn github_slug(url: &str) -> Option<(String, String)> {
    match forge_of(url)? {
        Forge::GitHub { owner, repo } => Some((owner, repo)),
        _ => None,
    }
}

/// Percent-escapes `s`; `slash` says whether `/` is escaped too (in a query).
fn escape(s: &str, slash: bool) -> String {
    let mut b = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || "-._~".contains(c) || (c == '/' && !slash) {
            b.push(c);
        } else {
            let mut buf = [0; 4];
            for byte in c.encode_utf8(&mut buf).bytes() {
                b.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    b
}

/// The page to open a pull request (merge request) for `branch` of the remote at `url`.
pub fn new_pr_url(url: &str, branch: &str) -> Option<String> {
    Some(match forge_of(url)? {
        Forge::GitHub { owner, repo } => format!("https://github.com/{owner}/{repo}/pull/new/{}", escape(branch, false)),
        Forge::GitLab { project } => {
            format!("https://gitlab.com/{project}/-/merge_requests/new?merge_request%5Bsource_branch%5D={}", escape(branch, true))
        }
        Forge::Bitbucket { owner, repo } => {
            format!("https://bitbucket.org/{owner}/{repo}/pull-requests/new?source={}", escape(branch, true))
        }
        Forge::Azure { org, project, repo } => {
            format!("https://dev.azure.com/{org}/{project}/_git/{repo}/pullrequestcreate?sourceRef={}", escape(branch, true))
        }
    })
}

pub(crate) fn config(wt: &Path, key: &str) -> Result<Option<String>, git::Error> {
    Ok(git::read_maybe(wt, &["config", "--get", key])?.map(|v| String::from_utf8_lossy(&v).trim().to_owned()).filter(|v| !v.is_empty()))
}

/// The remote `branch` is pushed to: its `pushRemote`, `remote.pushDefault`,
/// the remote it tracks, else `origin`.
pub(crate) fn remote_of(wt: &Path, branch: &str) -> Result<String, git::Error> {
    for key in [format!("branch.{branch}.pushRemote"), "remote.pushDefault".to_owned(), format!("branch.{branch}.remote")] {
        if let Some(r) = config(wt, &key)? {
            return Ok(r);
        }
    }
    Ok("origin".to_owned())
}

/// Opens the pull request page of `branch` in the browser; returns the URL.
pub fn open(wt: &Path, branch: &str) -> Result<String, git::Error> {
    let remote = remote_of(wt, branch)?;
    let url = config(wt, &format!("remote.{remote}.url"))?.ok_or_else(|| git::Error::Git(format!("no remote {remote}")))?;
    let page = new_pr_url(&url, branch)
        .ok_or_else(|| git::Error::Git(format!("{remote} ({url}) is not on GitHub, GitLab, Bitbucket or Azure DevOps")))?;
    opener().arg(&page).spawn().map_err(|e| git::Error::Io(format!("{page}: {e}")))?;
    Ok(page)
}

/// Opens `url` in the browser.
pub fn open_url(url: &str) -> Result<(), git::Error> {
    opener().arg(url).spawn().map(|_| ()).map_err(|e| git::Error::Io(format!("{url}: {e}")))
}

#[cfg(windows)]
fn opener() -> Command {
    // Not `explorer <url>`: with a `?query` it opens the Documents folder instead of the browser (#131).
    let mut c = Command::new("rundll32");
    c.arg("url.dll,FileProtocolHandler");
    git::hide_window(&mut c);
    c
}

#[cfg(target_os = "macos")]
fn opener() -> Command {
    Command::new("open")
}

#[cfg(not(any(windows, target_os = "macos")))]
fn opener() -> Command {
    Command::new("xdg-open")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_urls_of_every_form_give_the_owner_and_repo() {
        let want = Some(("uchmk".to_owned(), "michi".to_owned()));
        for u in [
            "https://github.com/uchmk/michi.git",
            "https://github.com/uchmk/michi",
            "https://user@github.com/uchmk/michi.git",
            "git@github.com:uchmk/michi.git",
            "ssh://git@github.com/uchmk/michi.git",
            "https://github.com/uchmk/michi/",
        ] {
            assert_eq!(github_slug(u), want, "{u}");
        }
        for u in ["https://gitlab.com/uchmk/michi.git", "git@github.com.evil.com:a/b", "C:/repos/michi", "https://github.com/uchmk"] {
            assert_eq!(github_slug(u), None, "{u}");
        }
    }

    #[test]
    fn other_hosts_get_their_own_pull_request_page() {
        assert_eq!(
            new_pr_url("git@gitlab.com:grp/sub/proj.git", "feat/x y").as_deref(),
            Some("https://gitlab.com/grp/sub/proj/-/merge_requests/new?merge_request%5Bsource_branch%5D=feat%2Fx%20y")
        );
        assert_eq!(
            new_pr_url("https://user@bitbucket.org/ann/repo.git", "feat/x").as_deref(),
            Some("https://bitbucket.org/ann/repo/pull-requests/new?source=feat%2Fx")
        );
        let az = Some("https://dev.azure.com/org/proj/_git/repo/pullrequestcreate?sourceRef=feat%2Fx".to_owned());
        assert_eq!(new_pr_url("https://org@dev.azure.com/org/proj/_git/repo", "feat/x"), az);
        assert_eq!(new_pr_url("git@ssh.dev.azure.com:v3/org/proj/repo", "feat/x"), az);
        for u in [
            "https://gitlab.com/solo",
            "https://bitbucket.org.evil.com/a/b",
            "https://dev.azure.com/org/proj/repo",
            "https://gitlab.com/x//y",
        ] {
            assert_eq!(new_pr_url(u, "x"), None, "{u}");
        }
    }

    #[test]
    fn the_branch_goes_into_the_page_with_odd_letters_escaped() {
        assert_eq!(
            new_pr_url("git@github.com:uchmk/michi.git", "tsumugi/1016-1200").as_deref(),
            Some("https://github.com/uchmk/michi/pull/new/tsumugi/1016-1200")
        );
        assert_eq!(new_pr_url("https://github.com/a/b", "x y#z").as_deref(), Some("https://github.com/a/b/pull/new/x%20y%23z"));
        assert_eq!(new_pr_url("https://example.com/a/b", "x"), None);
    }
}
