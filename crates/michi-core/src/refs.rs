//! The branches, the remote branches and the tags, from `git for-each-ref`,
//! the stashes, and the trunk: the branch the graph keeps on its left edge.

use std::collections::HashMap;
use std::path::Path;

use crate::git;
use crate::oid::Oid;

/// A branch or a tag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ref {
    pub name: String,
    pub id: Oid,
}

/// A remote branch: `origin/feat/x` is remote `origin`, name `feat/x`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteRef {
    pub remote: String,
    pub name: String,
    pub id: Oid,
}

/// A stash: its commit and what `git stash list` says of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stash {
    pub id: Oid,
    pub subject: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Refs {
    pub locals: Vec<Ref>,
    /// How far each local branch with an upstream is ahead of it and
    /// behind it: name to `(ahead, behind)`. A branch with no upstream, or one
    /// gone from its remote, is not in the map: that is not "in sync".
    pub track: HashMap<String, (u32, u32)>,
    /// The local branches whose upstream is gone from its remote (a pull
    /// request's branch deleted when it was merged).
    pub gone: Vec<String>,
    pub remotes: Vec<RemoteRef>,
    /// Peeled: an annotated tag's commit, not the tag object.
    pub tags: Vec<Ref>,
    /// Each remote's default branch, from `refs/remotes/<remote>/HEAD`:
    /// `(remote, branch)`.
    pub defaults: Vec<(String, String)>,
    /// Newest first.
    pub stashes: Vec<Stash>,
}

impl Refs {
    /// Whether the ref `full` (`refs/heads/x`, `refs/remotes/o/x`, `refs/tags/x`) exists.
    pub fn has_full(&self, full: &str) -> bool {
        if let Some(n) = full.strip_prefix("refs/heads/") {
            self.locals.iter().any(|r| r.name == n)
        } else if let Some(n) = full.strip_prefix("refs/tags/") {
            self.tags.iter().any(|r| r.name == n)
        } else if let Some(n) = full.strip_prefix("refs/remotes/") {
            self.remotes.iter().any(|r| format!("{}/{}", r.remote, r.name) == n)
        } else {
            false
        }
    }
}

/// What points at one commit, for the pills on its row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Labels {
    pub locals: Vec<String>,
    /// `(remote, name)`.
    pub remotes: Vec<(String, String)>,
    pub tags: Vec<String>,
}

/// The trunk and where its line starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trunk {
    /// The branch's name, no remote in front: `main`.
    pub name: String,
    /// Its commit: the local branch's, or the remote's when the local one is
    /// missing or behind it (not pulled yet).
    pub head: Oid,
    /// The remote it was found on, if any.
    pub remote: Option<String>,
}

const FORMAT: &str = "--format=%(refname)%00%(objectname)%00%(*objectname)%00%(symref)%00%(upstream:track,nobracket)%00%(upstream)";

impl Refs {
    /// The refs of the repository `dir` is in.
    pub fn read(dir: &Path) -> Result<Refs, git::Error> {
        let ((remotes, stashes), out) = git::both(
            || {
                // No stash list (a bare repository) is no stashes.
                git::both(|| git::read_text(dir, &["remote"]), || git::read_text(dir, &["stash", "list", "--format=%H%x00%gs"]))
            },
            || git::read(dir, &["for-each-ref", FORMAT, "refs/heads", "refs/remotes", "refs/tags"]),
        );
        let (remotes, out) = (remotes?, out?);
        let remotes: Vec<&str> = remotes.lines().map(str::trim).filter(|r| !r.is_empty()).collect();
        let mut refs = Refs::parse(&out, &remotes);
        refs.stashes = parse_stashes(&stashes.unwrap_or_default());
        Ok(refs)
    }

    /// `for-each-ref`'s lines, NUL between the fields. A remote branch is
    /// split after the longest of `remotes` it starts with (a remote's name
    /// may hold a `/`), else after its first `/`.
    pub fn parse(out: &[u8], remotes: &[&str]) -> Refs {
        let text = String::from_utf8_lossy(out);
        let mut refs = Refs::default();
        for line in text.lines() {
            let mut f = line.split('\0');
            let (Some(name), Some(id), peeled, symref) = (f.next(), f.next(), f.next(), f.next().unwrap_or("")) else { continue };
            // A branch with no upstream has an empty track too, like one even with it.
            let track = f.next().unwrap_or("");
            let has_upstream = f.next().is_none_or(|u| !u.is_empty());
            if let Some(rest) = name.strip_prefix("refs/remotes/") {
                let (remote, branch) = split_remote(rest, remotes);
                if !symref.is_empty() {
                    if branch == "HEAD" {
                        let target =
                            symref.strip_prefix("refs/remotes/").and_then(|t| t.strip_prefix(remote)).and_then(|t| t.strip_prefix('/'));
                        if let Some(t) = target {
                            refs.defaults.push((remote.to_owned(), t.to_owned()));
                        }
                    }
                    continue;
                }
                let Some(id) = Oid::parse(id) else { continue };
                refs.remotes.push(RemoteRef { remote: remote.to_owned(), name: branch.to_owned(), id });
            } else if let Some(branch) = name.strip_prefix("refs/heads/") {
                let Some(id) = Oid::parse(id) else { continue };
                if symref.is_empty() {
                    if has_upstream {
                        if let Some(t) = parse_track(track) {
                            refs.track.insert(branch.to_owned(), t);
                        } else if track == "gone" {
                            refs.gone.push(branch.to_owned());
                        }
                    }
                    refs.locals.push(Ref { name: branch.to_owned(), id });
                }
            } else if let Some(tag) = name.strip_prefix("refs/tags/") {
                let id = peeled.filter(|p| !p.is_empty()).unwrap_or(id);
                let Some(id) = Oid::parse(id) else { continue };
                refs.tags.push(Ref { name: tag.to_owned(), id });
            }
        }
        refs
    }

    /// The trunk's name: the default branch of `origin`, else of any remote,
    /// else `main`, else `master`, else the branch of the main worktree
    /// (`main_branch`), else the first branch.
    pub fn trunk_name(&self, main_branch: Option<&str>) -> Option<(String, Option<String>)> {
        let default = self.defaults.iter().find(|(r, _)| r == "origin").or(self.defaults.first());
        if let Some((remote, branch)) = default {
            return Some((branch.clone(), Some(remote.clone())));
        }
        let has = |n: &str| self.locals.iter().any(|r| r.name == n) || self.remotes.iter().any(|r| r.name == n);
        let name = ["main", "master"].into_iter().find(|n| has(n)).map(str::to_owned);
        let name = name.or(main_branch.map(str::to_owned)).or_else(|| self.locals.first().map(|r| r.name.clone()))?;
        Some((name, None))
    }

    pub fn local(&self, name: &str) -> Option<Oid> {
        self.locals.iter().find(|r| r.name == name).map(|r| r.id)
    }

    /// `name` on `remote`, or, with no remote given, on `origin` and then on
    /// any remote.
    pub fn remote(&self, remote: Option<&str>, name: &str) -> Option<&RemoteRef> {
        let on = |r: &str| self.remotes.iter().find(move |x| x.remote == r && x.name == name);
        match remote {
            Some(r) => on(r),
            None => on("origin").or_else(|| self.remotes.iter().find(|x| x.name == name)),
        }
    }

    /// What points at each commit.
    pub fn labels(&self) -> HashMap<Oid, Labels> {
        let mut by: HashMap<Oid, Labels> = HashMap::new();
        for r in &self.locals {
            by.entry(r.id).or_default().locals.push(r.name.clone());
        }
        for r in &self.remotes {
            by.entry(r.id).or_default().remotes.push((r.remote.clone(), r.name.clone()));
        }
        for r in &self.tags {
            by.entry(r.id).or_default().tags.push(r.name.clone());
        }
        by
    }

    /// The remote names, longest first, for taking `origin/` off a name.
    pub fn remote_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.remotes.iter().map(|r| r.remote.clone()).collect();
        names.extend(self.defaults.iter().map(|(r, _)| r.clone()));
        names.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
        names.dedup();
        names
    }
}

/// `%(upstream:track,nobracket)`: `ahead 2, behind 1`, `ahead 2`, `behind 1`,
/// empty when even with it, or `gone`. `None` for no upstream or a gone one.
fn parse_track(text: &str) -> Option<(u32, u32)> {
    if text == "gone" {
        return None;
    }
    let mut t = (0, 0);
    for part in text.split(", ").filter(|p| !p.is_empty()) {
        match part.split_once(' ') {
            Some(("ahead", n)) => t.0 = n.parse().ok()?,
            Some(("behind", n)) => t.1 = n.parse().ok()?,
            _ => return None,
        }
    }
    Some(t)
}

/// `git stash list --format=%H%x00%gs`.
fn parse_stashes(text: &str) -> Vec<Stash> {
    text.lines()
        .filter_map(|l| {
            let (id, subject) = l.split_once('\0')?;
            Some(Stash { id: Oid::parse(id)?, subject: subject.to_owned() })
        })
        .collect()
}

fn split_remote<'a>(rest: &'a str, remotes: &[&str]) -> (&'a str, &'a str) {
    let known = remotes
        .iter()
        .filter(|r| rest.len() > r.len() && rest.starts_with(**r) && rest.as_bytes()[r.len()] == b'/')
        .max_by_key(|r| r.len());
    match known {
        Some(r) => (&rest[..r.len()], &rest[r.len() + 1..]),
        None => rest.split_once('/').unwrap_or(("", rest)),
    }
}

/// The trunk of the repository `dir` is in (see [`Refs::trunk_name`]), and
/// its head. `None` when there is no branch at all.
pub fn trunk(dir: &Path, refs: &Refs, main_branch: Option<&str>) -> Result<Option<Trunk>, git::Error> {
    let Some((name, remote)) = refs.trunk_name(main_branch) else { return Ok(None) };
    let local = refs.local(&name);
    let far = refs.remote(remote.as_deref(), &name);
    let (head, remote) = match (local, far) {
        (None, None) => return Ok(None),
        (Some(l), None) => (l, remote),
        (None, Some(r)) => (r.id, Some(r.remote.clone())),
        (Some(l), Some(r)) if l == r.id => (l, Some(r.remote.clone())),
        (Some(l), Some(r)) => {
            let behind = git::check(dir, &["merge-base", "--is-ancestor", &l.to_string(), &r.id.to_string()])?;
            (if behind { r.id } else { l }, Some(r.remote.clone()))
        }
    };
    Ok(Some(Trunk { name, head, remote }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn full_names_are_found_by_kind() {
        let o = Oid::parse(&id('a')).unwrap();
        let refs = Refs {
            locals: vec![Ref { name: "feat/x".into(), id: o }],
            tags: vec![Ref { name: "v1".into(), id: o }],
            remotes: vec![RemoteRef { remote: "origin".into(), name: "side".into(), id: o }],
            ..Refs::default()
        };
        assert!(refs.has_full("refs/heads/feat/x"));
        assert!(refs.has_full("refs/tags/v1"));
        assert!(refs.has_full("refs/remotes/origin/side"));
        assert!(!refs.has_full("refs/heads/side"));
        assert!(!refs.has_full("origin/side"));
    }

    fn id(c: char) -> String {
        c.to_string().repeat(40)
    }

    #[test]
    fn for_each_ref_lines_are_sorted_out() {
        let lines = [
            format!("refs/heads/main\0{}\0\0\0ahead 2, behind 1\0refs/remotes/origin/main", id('1')),
            format!("refs/heads/feat/x\0{}\0\0\0\0refs/remotes/origin/feat/x", id('2')),
            format!("refs/heads/done\0{}\0\0\0gone\0refs/remotes/origin/done", id('2')),
            format!("refs/heads/loose\0{}\0\0\0\0", id('5')),
            format!("refs/remotes/origin/HEAD\0{}\0\0refs/remotes/origin/main", id('1')),
            format!("refs/remotes/origin/main\0{}\0\0", id('3')),
            format!("refs/remotes/my/fork/feat/x\0{}\0\0", id('2')),
            format!("refs/remotes/stray/y\0{}\0\0", id('4')),
            format!("refs/tags/v1\0{}\0{}\0", id('a'), id('1')),
            format!("refs/tags/light\0{}\0\0", id('3')),
        ];
        let r = Refs::parse(lines.join("\n").as_bytes(), &["origin", "my/fork"]);
        assert_eq!(r.locals.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["main", "feat/x", "done", "loose"]);
        assert_eq!(r.track.get("loose"), None, "no upstream is not in sync");
        assert_eq!(r.gone, ["done"]);
        assert_eq!(r.track.get("done"), None);
        assert_eq!(r.track.get("main"), Some(&(2, 1)));
        assert_eq!(r.track.get("feat/x"), Some(&(0, 0)), "an empty track is even with its upstream");
        let remotes: Vec<_> = r.remotes.iter().map(|x| (x.remote.as_str(), x.name.as_str())).collect();
        assert_eq!(remotes, [("origin", "main"), ("my/fork", "feat/x"), ("stray", "y")]);
        assert_eq!(r.defaults, [("origin".to_owned(), "main".to_owned())]);
        assert_eq!(r.tags[0].id.to_string(), id('1'), "an annotated tag is its commit");
        assert_eq!(r.tags[1].id.to_string(), id('3'));
        let labels = r.labels();
        let one = &labels[&Oid::parse(&id('1')).unwrap()];
        assert_eq!(one.locals, ["main"]);
        assert_eq!(one.tags, ["v1"]);
        let two = &labels[&Oid::parse(&id('2')).unwrap()];
        assert_eq!(two.locals, ["feat/x", "done"]);
        assert_eq!(two.remotes, [("my/fork".to_owned(), "feat/x".to_owned())]);
        assert_eq!(r.remote_names(), ["my/fork", "origin", "stray"]);
    }

    #[test]
    fn tracks_are_read() {
        assert_eq!(parse_track("ahead 3"), Some((3, 0)));
        assert_eq!(parse_track("behind 12"), Some((0, 12)));
        assert_eq!(parse_track("gone"), None);
        assert_eq!(parse_track("odd"), None);
    }

    #[test]
    fn upstreams_and_stashes_are_read() {
        let r = TempRepo::new("refs-track");
        std::fs::write(r.repo.join("a.txt"), "one\n").unwrap();
        r.git(&r.repo, &["add", "a.txt"]);
        r.git(&r.repo, &["commit", "-q", "-m", "a"]);
        r.git(&r.repo, &["remote", "add", "origin", "https://example.invalid/r.git"]);
        r.git(&r.repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
        r.git(&r.repo, &["branch", "-q", "-u", "origin/main", "main"]);
        r.git(&r.repo, &["branch", "-q", "lonely"]);
        r.git(&r.repo, &["commit", "-q", "--allow-empty", "-m", "b"]);
        std::fs::write(r.repo.join("a.txt"), "two\n").unwrap();
        r.git(&r.repo, &["stash", "push", "-q", "-m", "half done"]);
        let refs = Refs::read(&r.repo).unwrap();
        assert_eq!(refs.track.get("main"), Some(&(1, 0)));
        assert_eq!(refs.track.get("lonely"), None, "a branch with no upstream has no track");
        let stashes: Vec<_> = refs.stashes.iter().map(|s| s.subject.as_str()).collect();
        assert_eq!(stashes, ["On main: half done"]);
    }

    #[test]
    fn the_trunk_is_the_remote_default_then_main_then_master() {
        let mk = |text: &str| Refs::parse(text.as_bytes(), &["origin", "up"]);
        let a = id('1');
        let r = mk(&format!("refs/remotes/up/HEAD\0{a}\0\0refs/remotes/up/dev\nrefs/heads/main\0{a}\0\0"));
        assert_eq!(r.trunk_name(None), Some(("dev".into(), Some("up".into()))));
        let r = mk(&format!("refs/heads/master\0{a}\0\0\nrefs/heads/main\0{a}\0\0"));
        assert_eq!(r.trunk_name(None), Some(("main".into(), None)));
        let r = mk(&format!("refs/heads/feat\0{a}\0\0\nrefs/remotes/origin/master\0{a}\0\0"));
        assert_eq!(r.trunk_name(Some("feat")), Some(("master".into(), None)));
        let r = mk(&format!("refs/heads/feat\0{a}\0\0\nrefs/heads/trunk\0{a}\0\0"));
        assert_eq!(r.trunk_name(Some("trunk")), Some(("trunk".into(), None)));
        assert_eq!(r.trunk_name(None), Some(("feat".into(), None)));
        assert_eq!(Refs::default().trunk_name(Some("main")), Some(("main".into(), None)));
    }

    #[test]
    fn the_trunk_starts_at_the_remote_when_the_local_branch_is_behind() {
        let r = TempRepo::new("refs-trunk");
        r.git(&r.repo, &["commit", "-q", "--allow-empty", "-m", "second"]);
        r.git(&r.repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
        r.git(&r.repo, &["symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main"]);
        r.git(&r.repo, &["reset", "-q", "--hard", "HEAD~1"]);
        r.git(&r.repo, &["tag", "-a", "-m", "one", "v1"]);
        let refs = Refs::read(&r.repo).unwrap();
        let far = refs.remote(None, "main").unwrap().id;
        let near = refs.local("main").unwrap();
        assert_ne!(far, near);
        assert_eq!(refs.tags[0].id, near);
        let t = trunk(&r.repo, &refs, Some("main")).unwrap().unwrap();
        assert_eq!(t, Trunk { name: "main".into(), head: far, remote: Some("origin".into()) });

        // A commit on the local branch: ahead (diverged), so the local one.
        r.git(&r.repo, &["commit", "-q", "--allow-empty", "-m", "local"]);
        let refs = Refs::read(&r.repo).unwrap();
        let t = trunk(&r.repo, &refs, Some("main")).unwrap().unwrap();
        assert_eq!(t.head, refs.local("main").unwrap());
    }
}
