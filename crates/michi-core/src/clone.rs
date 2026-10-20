//! Cloning a repository from the repository screen: what was typed (an
//! address, and perhaps a folder), where the clone goes, and `git clone`
//! itself. Run it on a thread.

use std::path::{Path, PathBuf};

use crate::git;

/// What the repository screen's query asks to clone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub url: String,
    /// The folder typed after the address, as typed; none for the default place.
    pub dest: Option<String>,
}

/// Whether `s` is a repository address rather than a search or a path:
/// `https://`, `http://`, `ssh://`, `git://`, `file://`, or the scp form
/// `user@host:path`.
pub fn looks_like_url(s: &str) -> bool {
    let s = s.trim();
    if ["https://", "http://", "ssh://", "git://", "file://"].iter().any(|p| s.len() > p.len() && s.starts_with(p)) {
        return true;
    }
    match s.split_once('@') {
        Some((user, rest)) => {
            !user.is_empty()
                && !user.contains(['/', '\\', ':'])
                && rest.split_once(':').is_some_and(|(host, path)| !host.is_empty() && !host.contains(['/', '\\']) && !path.is_empty())
        }
        None => false,
    }
}

/// The request in `query`: an address, then optionally a folder. None when
/// the first word is not an address.
pub fn parse(query: &str) -> Option<Request> {
    let query = query.trim();
    let (url, rest) = match query.split_once(char::is_whitespace) {
        Some((u, r)) => (u, r.trim()),
        None => (query, ""),
    };
    if !looks_like_url(url) {
        return None;
    }
    Some(Request { url: url.to_owned(), dest: (!rest.is_empty()).then(|| rest.to_owned()) })
}

/// The folder name git would make for `url`: the last part of the path
/// without `.git` and trailing slashes. None when there is none.
pub fn repo_name(url: &str) -> Option<String> {
    let url = url.trim().trim_end_matches(['/', '\\']);
    let last = url.rsplit(['/', ':', '\\']).next()?;
    let name = last.strip_suffix(".git").unwrap_or(last);
    (!name.is_empty() && name != "." && name != "..").then(|| name.to_owned())
}

/// Where the clone goes: the folder typed (`~` is `home`; a relative one is
/// under `parent`), else `parent/<name>`. None when no name can be made.
pub fn target(req: &Request, parent: &Path, home: Option<&Path>) -> Option<PathBuf> {
    match &req.dest {
        Some(d) => {
            let p = crate::recent::expand(d, home);
            Some(if p.is_absolute() { p } else { parent.join(p) })
        }
        None => Some(parent.join(repo_name(&req.url)?)),
    }
}

/// `git clone -- url dest`, waiting for it. The folder must not exist or be
/// empty (git says so itself). A half-made folder is taken away on failure
/// only when this call made it.
pub fn run(url: &str, dest: &Path) -> Result<(), git::Error> {
    let existed = dest.exists();
    let out = git::command().args(["clone", "--quiet", "--"]).arg(url).arg(dest).output().map_err(git::started)?;
    if out.status.success() {
        return Ok(());
    }
    if !existed {
        let _ = std::fs::remove_dir_all(dest);
    }
    Err(git::refused_all(&String::from_utf8_lossy(&out.stderr)))
}

/// Like [`run`], telling git's progress as it comes and stopping when `stop`
/// is set: `Ok(false)` then, with the half-made folder taken away when this
/// call made it.
pub fn run_stoppable(
    url: &str,
    dest: &Path,
    stop: &std::sync::atomic::AtomicBool,
    tell: &mut dyn FnMut(crate::net::Progress),
) -> Result<bool, git::Error> {
    let existed = dest.exists();
    // git clones into an existing folder only when it is empty; such a folder stays, emptied.
    let empty = existed && std::fs::read_dir(dest).is_ok_and(|mut d| d.next().is_none());
    let mut c = git::command();
    c.args(["clone", "--progress", "--"]).arg(url).arg(dest);
    let said = crate::net::run(c, stop, tell)?;
    if matches!(&said, Some(s) if s.ok) {
        return Ok(true);
    }
    if !existed {
        let _ = std::fs::remove_dir_all(dest);
    } else if empty {
        for e in std::fs::read_dir(dest).into_iter().flatten().flatten() {
            let p = e.path();
            let _ = if p.is_dir() { std::fs::remove_dir_all(&p) } else { std::fs::remove_file(&p) };
        }
    }
    match said {
        Some(s) => Err(s.refused()),
        None => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn a_stoppable_clone_clones_and_a_stopped_one_leaves_no_folder() {
        use std::sync::atomic::AtomicBool;
        let t = TempRepo::new("clone-stoppable");
        let url = t.repo.to_string_lossy().into_owned();
        let parent = t.root.join("dests");
        std::fs::create_dir_all(&parent).unwrap();
        let (a, b) = (parent.join("a"), parent.join("b"));
        assert!(run_stoppable(&url, &a, &AtomicBool::new(false), &mut |_| {}).unwrap());
        assert!(a.join(".git").exists());
        assert!(!run_stoppable(&url, &b, &AtomicBool::new(true), &mut |_| {}).unwrap());
        assert!(!b.exists(), "the half-made folder is taken away");
        // An empty folder that was there stays, emptied; a full one is never touched.
        let (e, f) = (parent.join("e"), parent.join("f"));
        std::fs::create_dir_all(&e).unwrap();
        std::fs::create_dir_all(&f).unwrap();
        std::fs::write(f.join("mine.txt"), "x").unwrap();
        assert!(!run_stoppable(&url, &e, &AtomicBool::new(true), &mut |_| {}).unwrap());
        assert!(e.exists() && std::fs::read_dir(&e).unwrap().next().is_none());
        assert!(run_stoppable(&url, &f, &AtomicBool::new(false), &mut |_| {}).is_err());
        assert!(f.join("mine.txt").exists());
    }

    #[test]
    fn addresses_are_told_from_paths_and_searches() {
        for u in ["https://github.com/uchmk/michi.git", "git@github.com:uchmk/michi.git", "ssh://git@host/a/b", "file:///tmp/x"] {
            assert!(looks_like_url(u), "{u}");
        }
        for u in ["", "michi", "C:\\dev\\michi", "/dev/michi", "https://", "a@b", "./x@y:z", "~/x"] {
            assert!(!looks_like_url(u), "{u}");
        }
    }

    #[test]
    fn the_query_is_an_address_and_maybe_a_folder() {
        assert_eq!(parse("nope"), None);
        assert_eq!(parse(" https://h/a/b.git "), Some(Request { url: "https://h/a/b.git".into(), dest: None }));
        assert_eq!(parse("git@h:a/b.git  my dir/x"), Some(Request { url: "git@h:a/b.git".into(), dest: Some("my dir/x".into()) }));
    }

    #[test]
    fn the_name_is_the_last_part_without_git() {
        assert_eq!(repo_name("https://h/a/b.git").as_deref(), Some("b"));
        assert_eq!(repo_name("https://h/a/b/").as_deref(), Some("b"));
        assert_eq!(repo_name("git@h:b.git").as_deref(), Some("b"));
        assert_eq!(repo_name("git@h:b").as_deref(), Some("b"));
        assert_eq!(repo_name("https://h/.git"), None);
    }

    #[test]
    fn the_target_is_beside_or_typed() {
        let parent = Path::new("/dev");
        let home = Path::new("/home/u");
        let r = parse("https://h/a/b.git").unwrap();
        assert_eq!(target(&r, parent, Some(home)), Some(PathBuf::from("/dev/b")));
        let r = parse("https://h/a/b.git ~/c").unwrap();
        assert_eq!(target(&r, parent, Some(home)), Some(PathBuf::from("/home/u/c")));
        let r = parse("https://h/a/b.git c").unwrap();
        assert_eq!(target(&r, parent, Some(home)), Some(PathBuf::from("/dev/c")));
    }

    #[test]
    fn a_clone_is_made_and_a_failed_one_leaves_nothing() {
        let t = TempRepo::new("clone");
        let url = t.repo.display().to_string();
        let dest = t.root.join("copy");
        run(&url, &dest).unwrap();
        assert!(dest.join(".git").exists());
        assert!(run(&url, &dest).is_err(), "a folder with a repository in it is refused");
        assert!(dest.join(".git").exists(), "and kept");
        let none = t.root.join("none");
        assert!(run(&t.root.join("missing").display().to_string(), &none).is_err());
        assert!(!none.exists());
    }
}
