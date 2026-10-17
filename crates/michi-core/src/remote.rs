//! Remotes: list them with their addresses, add one, remove one. Nothing
//! here touches the network. Removing one drops its remote-tracking branches
//! (git does that), so the caller asks first and says what goes.

use std::path::Path;

use crate::branch;
use crate::git;

/// A remote and where it points.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Remote {
    pub name: String,
    /// The address fetch reads from.
    pub url: String,
    /// The address push writes to, when it is another one.
    pub push_url: Option<String>,
}

/// The remotes of the repository at `dir`, by name.
pub fn list(dir: &Path) -> Result<Vec<Remote>, git::Error> {
    // A repository with no remote prints nothing.
    Ok(parse(&git::read_text(dir, &["remote", "-v"])?))
}

/// `git remote -v`: `name<TAB>address (fetch)` and `... (push)` lines.
pub fn parse(text: &str) -> Vec<Remote> {
    let mut out: Vec<Remote> = Vec::new();
    for line in text.lines() {
        let Some((name, rest)) = line.split_once(['\t', ' ']) else { continue };
        let rest = rest.trim();
        let (url, push) = if let Some(u) = rest.strip_suffix("(fetch)") {
            (u.trim(), false)
        } else if let Some(u) = rest.strip_suffix("(push)") {
            (u.trim(), true)
        } else {
            continue;
        };
        let at = match out.iter().position(|r| r.name == name) {
            Some(k) => k,
            None => {
                out.push(Remote { name: name.to_owned(), url: String::new(), push_url: None });
                out.len() - 1
            }
        };
        if push {
            out[at].push_url = Some(url.to_owned());
        } else {
            out[at].url = url.to_owned();
        }
    }
    // A push address equal to the fetch one says nothing.
    for r in &mut out {
        if r.push_url.as_deref() == Some(r.url.as_str()) {
            r.push_url = None;
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// What is wrong with `name` and `url` as a new remote, in words; none when
/// they will do. Pure: git has the last word when it adds.
pub fn check(name: &str, url: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("a remote needs a name".into());
    }
    if name.starts_with('-')
        || name.contains(['/', '\\', ':', '~', '^', '?', '*', '[', ' '])
        || name.contains("..")
        || name.ends_with(".lock")
    {
        return Err(format!("'{name}' is not a valid remote name"));
    }
    if url.trim().is_empty() {
        return Err("a remote needs an address".into());
    }
    if url.starts_with('-') {
        return Err(format!("'{url}' is not a valid address"));
    }
    Ok(())
}

/// Adds the remote `name` at `url` (it is not fetched). A name that is
/// taken is refused.
pub fn add(dir: &Path, name: &str, url: &str) -> Result<(), git::Error> {
    check(name, url).map_err(git::Error::Git)?;
    branch::write(dir, &["remote", "add", name, url])
}

/// Removes the remote `name`, with its remote-tracking branches. What it
/// pointed at, as it was.
pub fn remove(dir: &Path, name: &str) -> Result<Remote, git::Error> {
    let was = list(dir)?.into_iter().find(|r| r.name == name).ok_or_else(|| git::Error::Git(format!("no remote '{name}'")))?;
    branch::write(dir, &["remote", "remove", name])?;
    Ok(was)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn verbose_lines_become_remotes() {
        let text = "origin\thttps://h/a.git (fetch)\norigin\thttps://h/a.git (push)\nup\thttps://h/b.git (fetch)\nup\tgit@h:b.git (push)";
        let r = parse(text);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0], Remote { name: "origin".into(), url: "https://h/a.git".into(), push_url: None });
        assert_eq!(r[1].push_url.as_deref(), Some("git@h:b.git"));
        assert!(parse("").is_empty());
    }

    #[test]
    fn names_and_addresses_are_checked() {
        assert!(check("origin", "https://h/a.git").is_ok());
        assert!(check("up-stream", "../other").is_ok());
        assert!(check("", "x").is_err());
        assert!(check("a b", "x").is_err());
        assert!(check("a/b", "x").is_err());
        assert!(check("-x", "x").is_err());
        assert!(check("a..b", "x").is_err());
        assert!(check("origin", "").is_err());
        assert!(check("origin", "--upload-pack=x").is_err());
    }

    #[test]
    fn a_remote_is_added_listed_and_removed() {
        let t = TempRepo::configured("remote-manage");
        assert!(list(&t.repo).unwrap().is_empty());
        add(&t.repo, "origin", "https://example.invalid/a.git").unwrap();
        add(&t.repo, "up", "../elsewhere").unwrap();
        let names: Vec<String> = list(&t.repo).unwrap().into_iter().map(|r| r.name).collect();
        assert_eq!(names, ["origin", "up"]);
        let e = add(&t.repo, "origin", "https://example.invalid/b.git").unwrap_err();
        assert!(e.to_string().contains("already exists"), "{e}");
        let was = remove(&t.repo, "origin").unwrap();
        assert_eq!(was.url, "https://example.invalid/a.git");
        assert_eq!(list(&t.repo).unwrap().len(), 1);
        assert!(remove(&t.repo, "origin").is_err());
    }
}
