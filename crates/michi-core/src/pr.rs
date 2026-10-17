//! Handing a branch to the browser to open a pull request on GitHub. michi
//! does not create the pull request itself: GitHub's "compare" page for the
//! branch (the page of the pull request that is already there, when there
//! is one) is where it is written.

use std::path::Path;
use std::process::Command;

use crate::git;

/// `owner/repo` of a GitHub remote's URL (`https://`, `git@github.com:` and
/// `ssh://` forms); `None` for any other host.
pub fn github_slug(url: &str) -> Option<(String, String)> {
    let url = url.trim();
    let rest = ["https://", "http://", "ssh://", "git://"]
        .iter()
        .find_map(|p| url.strip_prefix(p))
        .map(|r| r.split_once('@').map_or(r, |(_, h)| h))
        .or_else(|| url.strip_prefix("git@"))?;
    let path = rest.strip_prefix("github.com").and_then(|p| p.strip_prefix(['/', ':']))?;
    let path = path.trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let (owner, repo) = path.split_once('/')?;
    (!owner.is_empty() && !repo.is_empty() && !repo.contains('/')).then(|| (owner.to_owned(), repo.to_owned()))
}

/// The page to open a pull request for `branch` of the remote at `url`.
pub fn new_pr_url(url: &str, branch: &str) -> Option<String> {
    let (owner, repo) = github_slug(url)?;
    let mut b = String::new();
    for c in branch.chars() {
        if c.is_ascii_alphanumeric() || "/-._~".contains(c) {
            b.push(c);
        } else {
            let mut buf = [0; 4];
            for byte in c.encode_utf8(&mut buf).bytes() {
                b.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    Some(format!("https://github.com/{owner}/{repo}/pull/new/{b}"))
}

fn config(wt: &Path, key: &str) -> Result<Option<String>, git::Error> {
    Ok(git::read_maybe(wt, &["config", "--get", key])?.map(|v| String::from_utf8_lossy(&v).trim().to_owned()).filter(|v| !v.is_empty()))
}

/// The remote `branch` is pushed to: its `pushRemote`, `remote.pushDefault`,
/// the remote it tracks, else `origin`.
fn remote_of(wt: &Path, branch: &str) -> Result<String, git::Error> {
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
    let page = new_pr_url(&url, branch).ok_or_else(|| git::Error::Git(format!("{remote} ({url}) is not on GitHub")))?;
    opener().arg(&page).spawn().map_err(|e| git::Error::Io(format!("{page}: {e}")))?;
    Ok(page)
}

/// Opens `url` in the browser.
pub fn open_url(url: &str) -> Result<(), git::Error> {
    opener().arg(url).spawn().map(|_| ()).map_err(|e| git::Error::Io(format!("{url}: {e}")))
}

#[cfg(windows)]
fn opener() -> Command {
    let mut c = Command::new("explorer");
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
    fn the_branch_goes_into_the_page_with_odd_letters_escaped() {
        assert_eq!(
            new_pr_url("git@github.com:uchmk/michi.git", "tsumugi/1016-1200").as_deref(),
            Some("https://github.com/uchmk/michi/pull/new/tsumugi/1016-1200")
        );
        assert_eq!(new_pr_url("https://github.com/a/b", "x y#z").as_deref(), Some("https://github.com/a/b/pull/new/x%20y%23z"));
        assert_eq!(new_pr_url("https://example.com/a/b", "x"), None);
    }
}
