//! A branch's changes as a whole, to look over before it goes into the trunk
//! (docs/design.md: ブランチ全体の差分): what changed from where the branch
//! left the trunk (`trunk...branch`), its uncommitted changes in or out, or
//! what changed since the last look.
//!
//! The last look is a ref, `refs/michi/seen/<branch>`, to the end the review
//! last showed (a worktree's WIP commit when its uncommitted changes were
//! in), so gc keeps it. The files checked as looked at are kept in michi's
//! state folder, outside the repository: for each branch, each file by its
//! path and its content (the blob), so a file that changed after it was
//! checked is no longer checked.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::git;
use crate::oid::Oid;
use crate::show::{self, Changes, File};

/// Where `head` left `trunk`: their merge base; `None` when they have no
/// commit in common.
pub fn merge_base(dir: &Path, trunk: &Oid, head: &Oid) -> Result<Option<Oid>, git::Error> {
    let out = git::read_maybe(dir, &["merge-base", &trunk.to_string(), &head.to_string()])?;
    Ok(out.and_then(|o| Oid::parse(String::from_utf8_lossy(&o).trim())))
}

/// The ref the last look at `branch` is kept in.
pub fn seen_ref(branch: &str) -> String {
    format!("refs/michi/seen/{branch}")
}

/// The end of the last look at `branch`; `None` before the first.
pub fn read_seen(dir: &Path, branch: &str) -> Result<Option<Oid>, git::Error> {
    let spec = format!("{}^{{commit}}", seen_ref(branch));
    let out = git::read_maybe(dir, &["rev-parse", "--verify", "-q", &spec])?;
    Ok(out.and_then(|o| Oid::parse(String::from_utf8_lossy(&o).trim())))
}

/// Makes `end` the last look at `branch`.
pub fn write_seen(dir: &Path, branch: &str, end: &Oid) -> Result<(), git::Error> {
    git::read(dir, &["update-ref", "-m", "michi: looked over", &seen_ref(branch), &end.to_string()]).map(drop)
}

/// The files a review lists.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct List {
    /// Where the branch left the trunk; `None` when they have no commit in
    /// common (and nothing is listed).
    pub base: Option<Oid>,
    /// The end of the last look before this one.
    pub last: Option<Oid>,
    pub changes: Changes,
}

/// The files changed on `branch` (its commit `head`) from where it left
/// `trunk`, or from `since` when given, to `end` (`head`, or its worktree's
/// WIP commit); `end` becomes the last look, unless `stop` says to stop.
pub fn list(
    dir: &Path,
    branch: &str,
    trunk: &Oid,
    head: &Oid,
    end: &Oid,
    since: Option<&Oid>,
    stop: &dyn Fn() -> bool,
) -> Result<List, git::Error> {
    let (base, last) = git::both(|| merge_base(dir, trunk, head), || read_seen(dir, branch));
    let (base, last) = (base?, last?);
    let changes = match since.or(base.as_ref()) {
        Some(from) => show::changes(dir, from, end, false, stop)?,
        None => Changes::default(),
    };
    // A read stopped halfway was not looked at.
    if !stop() {
        write_seen(dir, branch, end)?;
    }
    Ok(List { base, last, changes })
}

/// Where a comparison of `a` with `b` reads from: `a` itself when one is
/// the other's ancestor (or they share no history), else where `b` left
/// `a` (their merge base), as a branch is compared with the trunk.
pub fn compare_from(dir: &Path, a: &Oid, b: &Oid) -> Result<Oid, git::Error> {
    Ok(match merge_base(dir, a, b)? {
        Some(base) if base != *a && base != *b => base,
        _ => *a,
    })
}

/// What changed from `a` to `b`, any two commits (the review's list of a
/// comparison): see [`compare_from`]. `base` is where the diff reads from.
/// Nothing is kept as looked at.
pub fn compare(dir: &Path, a: &Oid, b: &Oid, stop: &dyn Fn() -> bool) -> Result<List, git::Error> {
    let from = compare_from(dir, a, b)?;
    let changes = show::changes(dir, &from, b, false, stop)?;
    Ok(List { base: Some(from), last: None, changes })
}

/// A file as the review shows it: its path and its content at the end
/// (`None` when deleted).
pub type Key = (String, Option<Oid>);

pub fn key(f: &File) -> Key {
    (f.path.clone(), f.blob)
}

/// The files checked as looked at, as the state file keeps them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checks {
    #[serde(default)]
    branches: BTreeMap<String, Vec<Check>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Check {
    path: String,
    /// The content's blob; none for a deleted file.
    #[serde(default)]
    blob: Option<String>,
}

impl Check {
    fn key(&self) -> Option<Key> {
        let blob = match &self.blob {
            Some(hex) => Some(Oid::parse(hex)?),
            None => None,
        };
        Some((self.path.clone(), blob))
    }
}

impl Checks {
    /// From the state file's text (TOML; a JSON file of an older build reads too); a file that cannot be read starts afresh.
    pub fn parse(text: &str) -> Checks {
        toml::from_str(text).or_else(|_| serde_json::from_str(text)).unwrap_or_default()
    }

    pub fn to_toml(&self) -> String {
        toml::to_string(self).unwrap_or_default()
    }

    /// The files of `branch` checked.
    pub fn of(&self, branch: &str) -> BTreeSet<Key> {
        self.branches.get(branch).map_or_else(BTreeSet::new, |cs| cs.iter().filter_map(Check::key).collect())
    }

    /// Checks the file `key` of `branch`, or takes its check off; whether
    /// anything changed.
    pub fn set(&mut self, branch: &str, key: &Key, on: bool) -> bool {
        let checks = self.branches.entry(branch.to_owned()).or_default();
        let at = checks.iter().position(|c| c.key().as_ref() == Some(key));
        let changed = match (at, on) {
            (None, true) => {
                checks.push(Check { path: key.0.clone(), blob: key.1.map(|b| b.to_string()) });
                true
            }
            (Some(i), false) => {
                checks.remove(i);
                true
            }
            _ => false,
        };
        if checks.is_empty() {
            self.branches.remove(branch);
        }
        changed
    }

    /// Keeps the checks of `branch` on files as `files` has them (the whole
    /// of the branch): a file changed since or gone loses its check.
    pub fn prune(&mut self, branch: &str, files: &[File]) -> bool {
        let Some(checks) = self.branches.get_mut(branch) else { return false };
        let now: BTreeSet<Key> = files.iter().map(key).collect();
        let before = checks.len();
        checks.retain(|c| c.key().is_some_and(|k| now.contains(&k)));
        let changed = checks.len() != before;
        if checks.is_empty() {
            self.branches.remove(branch);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn id(r: &TempRepo, rev: &str) -> Oid {
        Oid::parse(&r.out(&r.repo, &["rev-parse", rev])).unwrap()
    }

    /// `main` with `a.txt`; `lane` off it changing `a.txt` and adding
    /// `b.txt`; `main` then adding `c.txt`.
    fn branched(name: &str) -> TempRepo {
        let r = TempRepo::new(name);
        r.write(&r.repo, "a.txt", "one\n");
        r.git(&r.repo, &["add", "-A"]);
        r.git(&r.repo, &["commit", "-q", "-m", "a"]);
        r.git(&r.repo, &["checkout", "-q", "-b", "lane"]);
        r.write(&r.repo, "a.txt", "one\ntwo\n");
        r.git(&r.repo, &["commit", "-q", "-am", "two"]);
        r.write(&r.repo, "b.txt", "b\n");
        r.git(&r.repo, &["add", "-A"]);
        r.git(&r.repo, &["commit", "-q", "-m", "b"]);
        r.git(&r.repo, &["checkout", "-q", "main"]);
        r.write(&r.repo, "c.txt", "c\n");
        r.git(&r.repo, &["add", "-A"]);
        r.git(&r.repo, &["commit", "-q", "-m", "c"]);
        r
    }

    fn paths(l: &List) -> Vec<&str> {
        l.changes.files.iter().map(|f| f.path.as_str()).collect()
    }

    #[test]
    fn the_merge_base_is_where_the_branch_left_and_none_without_one() {
        let r = branched("review-base");
        let (main, lane) = (id(&r, "main"), id(&r, "lane"));
        assert_eq!(merge_base(&r.repo, &main, &lane).unwrap(), Some(id(&r, "lane~2")));
        r.git(&r.repo, &["checkout", "-q", "--orphan", "other"]);
        r.git(&r.repo, &["commit", "-q", "--allow-empty", "-m", "alone"]);
        assert_eq!(merge_base(&r.repo, &main, &id(&r, "other")).unwrap(), None);
    }

    #[test]
    fn a_branch_is_listed_from_where_it_left_without_the_trunk_s_changes() {
        let r = branched("review-list");
        let (main, lane) = (id(&r, "main"), id(&r, "lane"));
        let l = list(&r.repo, "lane", &main, &lane, &lane, None, &|| false).unwrap();
        assert_eq!(l.base, Some(id(&r, "lane~2")));
        assert_eq!(paths(&l), ["a.txt", "b.txt"]);
        assert_eq!(l.changes.files[0].counts, Some((1, 0)));
        assert_eq!(l.changes.files[1].blob, Some(Oid::parse(&r.out(&r.repo, &["rev-parse", "lane:b.txt"])).unwrap()));
    }

    #[test]
    fn each_look_is_kept_in_a_ref_and_since_lists_what_came_after() {
        let r = branched("review-seen");
        let (main, lane) = (id(&r, "main"), id(&r, "lane"));
        assert_eq!(read_seen(&r.repo, "lane").unwrap(), None);
        let first = list(&r.repo, "lane", &main, &lane, &lane, None, &|| false).unwrap();
        assert_eq!(first.last, None);
        assert_eq!(read_seen(&r.repo, "lane").unwrap(), Some(lane));

        r.git(&r.repo, &["checkout", "-q", "lane"]);
        r.write(&r.repo, "b.txt", "b\nmore\n");
        r.git(&r.repo, &["commit", "-q", "-am", "more"]);
        let now = id(&r, "lane");
        let again = list(&r.repo, "lane", &main, &now, &now, Some(&lane), &|| false).unwrap();
        assert_eq!(again.last, Some(lane));
        assert_eq!(paths(&again), ["b.txt"]);
        assert_eq!(read_seen(&r.repo, "lane").unwrap(), Some(now));
    }

    #[test]
    fn a_branch_without_a_base_lists_nothing() {
        let r = branched("review-alone");
        r.git(&r.repo, &["checkout", "-q", "--orphan", "other"]);
        r.git(&r.repo, &["commit", "-q", "--allow-empty", "-m", "alone"]);
        let other = id(&r, "other");
        let l = list(&r.repo, "other", &id(&r, "main"), &other, &other, None, &|| false).unwrap();
        assert_eq!(l.base, None);
        assert!(l.changes.files.is_empty());
    }

    #[test]
    fn two_commits_are_compared_from_themselves_or_from_where_they_parted() {
        let r = branched("review-compare");
        let (main, lane) = (id(&r, "main"), id(&r, "lane"));
        // Two branches: from where lane left main, without main's own c.txt.
        let l = compare(&r.repo, &main, &lane, &|| false).unwrap();
        assert_eq!(l.base, Some(id(&r, "lane~2")));
        assert_eq!(paths(&l), ["a.txt", "b.txt"]);
        // The other way round: what main did since.
        let l = compare(&r.repo, &lane, &main, &|| false).unwrap();
        assert_eq!(paths(&l), ["c.txt"]);
        // One the other's ancestor: plain, in either direction.
        let base = id(&r, "lane~2");
        let l = compare(&r.repo, &base, &lane, &|| false).unwrap();
        assert_eq!((l.base, paths(&l)), (Some(base), vec!["a.txt", "b.txt"]));
        let l = compare(&r.repo, &lane, &base, &|| false).unwrap();
        assert_eq!((l.base, paths(&l)), (Some(lane), vec!["a.txt", "b.txt"]));
        assert!(compare(&r.repo, &lane, &lane, &|| false).unwrap().changes.files.is_empty());
        // Nothing of it is kept as a look.
        assert_eq!(read_seen(&r.repo, "lane").unwrap(), None);
    }

    fn file(path: &str, blob: Option<&str>) -> File {
        File {
            status: show::Status::Modified,
            path: path.to_owned(),
            from: None,
            blob: blob.map(|b| Oid::parse(&b.repeat(40)).unwrap()),
            counts: Some((1, 0)),
            hunks: Vec::new(),
            eol_only: false,
            cut: false,
            not_utf8: false,
        }
    }

    #[test]
    fn checks_go_by_path_and_content_and_a_changed_file_loses_its_own() {
        let (a, b, gone) = (file("a.txt", Some("1")), file("b.txt", Some("2")), file("c.txt", None));
        let mut c = Checks::default();
        assert!(c.set("lane", &key(&a), true));
        assert!(!c.set("lane", &key(&a), true));
        assert!(c.set("lane", &key(&gone), true));
        assert!(c.set("lane", &key(&b), true));
        assert!(c.set("lane", &key(&b), false));
        assert_eq!(c.of("lane"), BTreeSet::from([key(&a), key(&gone)]));
        assert!(c.of("other").is_empty());

        let back = Checks::parse(&c.to_toml());
        assert_eq!(back, c);

        let a2 = file("a.txt", Some("3"));
        assert!(!c.prune("lane", &[a.clone(), gone.clone()]));
        assert!(c.prune("lane", &[a2, gone.clone()]));
        assert_eq!(c.of("lane"), BTreeSet::from([key(&gone)]));
        assert!(c.prune("lane", &[]));
        assert_eq!(c, Checks::default());
        assert_eq!(Checks::parse("not json"), Checks::default());
    }
}
