//! Where each worktree's branch stands in review (docs/design.md: the
//! worktree table's review column): not looked at, being looked over,
//! flagged (notes handed to its Claude, whose fix is awaited), or approved
//! (every file of its diff against the trunk checked as looked at).
//!
//! It is worked out from what the branch review keeps: the last look
//! (`refs/michi/seen/<branch>`), the files checked (`review::Checks`) and,
//! kept here, the branch head the notes were handed at. Only the committed
//! branch counts, not a worktree's uncommitted changes.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::git;
use crate::oid::Oid;
use crate::review::{self, Checks, Key};
use crate::show;
use crate::track::Trees;

/// Where a branch stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Never opened in the review, nothing checked.
    Unreviewed,
    /// Opened, or some files checked, not all.
    InReview,
    /// Notes were handed to its Claude at the head it has still.
    Flagged,
    /// Every file checked.
    Approved,
}

/// What decides a branch's stage.
pub struct Facts<'a> {
    /// The files of the branch's diff against the trunk.
    pub files: &'a [Key],
    /// The files checked as looked at (some may be of an older content).
    pub checked: &'a BTreeSet<Key>,
    /// The end of the last look; `None` before the first.
    pub seen: Option<Oid>,
    pub head: Oid,
    /// The head the notes were last handed at.
    pub flagged: Option<Oid>,
}

/// A branch's stage and how many of its files are checked, of how many;
/// `None` for a branch with no change to review.
pub fn stage(f: &Facts) -> Option<(Stage, usize, usize)> {
    let total = f.files.len();
    if total == 0 {
        return None;
    }
    let checked = f.files.iter().filter(|k| f.checked.contains(*k)).count();
    let stage = if f.flagged == Some(f.head) {
        Stage::Flagged
    } else if checked == total {
        Stage::Approved
    } else if checked > 0 || f.seen.is_some() {
        Stage::InReview
    } else {
        Stage::Unreviewed
    };
    Some((stage, checked, total))
}

/// The heads the notes were handed at, per branch: michi's state file.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Flags {
    #[serde(default)]
    branches: BTreeMap<String, String>,
}

impl Flags {
    /// From the state file's text; a file that cannot be read starts afresh.
    pub fn parse(text: &str) -> Flags {
        serde_json::from_str(text).unwrap_or_default()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    pub fn of(&self, branch: &str) -> Option<Oid> {
        self.branches.get(branch).and_then(|h| Oid::parse(h))
    }

    /// Notes were handed to `branch`'s Claude at `head`.
    pub fn set(&mut self, branch: &str, head: &Oid) {
        self.branches.insert(branch.to_owned(), head.to_string());
    }
}

/// One worktree's standing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub path: PathBuf,
    pub stage: Stage,
    pub checked: usize,
    pub total: usize,
}

/// Every worktree with a branch to review; one without has no row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reviews {
    pub rows: Vec<Row>,
}

impl Reviews {
    pub fn of(&self, path: &Path) -> Option<&Row> {
        self.rows.iter().find(|r| r.path == path)
    }
}

/// The diffs already listed, by `(trunk, head)`.
#[derive(Default)]
pub struct Cache(HashMap<(Oid, Oid), Vec<Key>>);

/// The last looks, by branch.
fn seen_all(dir: &Path) -> Result<HashMap<String, Oid>, git::Error> {
    let out = git::read_text(dir, &["for-each-ref", "--format=%(objectname) %(refname)", "refs/michi/seen"])?;
    Ok(out
        .lines()
        .filter_map(|l| {
            let (id, name) = l.split_once(' ')?;
            Some((name.strip_prefix("refs/michi/seen/")?.to_owned(), Oid::parse(id)?))
        })
        .collect())
}

/// Works out each worktree's standing against the trunk.
pub fn read(dir: &Path, trees: &Trees, checks: &Checks, flags: &Flags, cache: &mut Cache) -> Result<Reviews, git::Error> {
    let Some(trunk) = trees.trunk_head else { return Ok(Reviews::default()) };
    let seen = seen_all(dir)?;
    if cache.0.len() > 64 {
        cache.0.clear();
    }
    let mut rows = Vec::new();
    for t in &trees.trees {
        let (Some(branch), Some(head)) = (&t.wt.branch, Oid::parse(&t.wt.head)) else { continue };
        if head == trunk {
            continue;
        }
        let files = match cache.0.get(&(trunk, head)) {
            Some(f) => f.clone(),
            None => {
                let f = match review::merge_base(dir, &trunk, &head)? {
                    Some(base) => show::changes(dir, &base, &head, false, &|| false)?.files.iter().map(review::key).collect(),
                    None => Vec::new(),
                };
                cache.0.insert((trunk, head), f.clone());
                f
            }
        };
        let facts = Facts { files: &files, checked: &checks.of(branch), seen: seen.get(branch).copied(), head, flagged: flags.of(branch) };
        if let Some((stage, checked, total)) = stage(&facts) {
            rows.push(Row { path: t.wt.path.clone(), stage, checked, total });
        }
    }
    Ok(Reviews { rows })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(c: &str) -> Oid {
        Oid::parse(&c.repeat(40)).unwrap()
    }

    fn key(p: &str) -> Key {
        (p.to_owned(), Some(oid("1")))
    }

    fn stage_of(files: &[Key], checked: &[Key], seen: Option<Oid>, flagged: Option<Oid>) -> Option<(Stage, usize, usize)> {
        stage(&Facts { files, checked: &checked.iter().cloned().collect(), seen, head: oid("a"), flagged })
    }

    #[test]
    fn a_branch_goes_from_unreviewed_through_review_to_approved() {
        let files = [key("a"), key("b")];
        assert_eq!(stage_of(&files, &[], None, None), Some((Stage::Unreviewed, 0, 2)));
        assert_eq!(stage_of(&files, &[], Some(oid("a")), None), Some((Stage::InReview, 0, 2)));
        assert_eq!(stage_of(&files, &[key("a")], None, None), Some((Stage::InReview, 1, 2)));
        assert_eq!(stage_of(&files, &[key("a"), key("b")], Some(oid("a")), None), Some((Stage::Approved, 2, 2)));
    }

    #[test]
    fn a_check_on_an_older_content_does_not_count() {
        let files = [key("a")];
        let old = [("a".to_owned(), Some(oid("2")))];
        assert_eq!(stage_of(&files, &old, None, None), Some((Stage::Unreviewed, 0, 1)));
    }

    #[test]
    fn notes_handed_flag_the_branch_until_its_head_moves() {
        let files = [key("a")];
        assert_eq!(stage_of(&files, &[key("a")], None, Some(oid("a"))), Some((Stage::Flagged, 1, 1)));
        assert_eq!(stage_of(&files, &[], Some(oid("a")), Some(oid("b"))), Some((Stage::InReview, 0, 1)));
    }

    #[test]
    fn a_branch_with_no_change_has_no_stage() {
        assert_eq!(stage_of(&[], &[], None, None), None);
    }

    #[test]
    fn flags_round_trip() {
        let mut f = Flags::default();
        f.set("lane", &oid("a"));
        assert_eq!(Flags::parse(&f.to_json()).of("lane"), Some(oid("a")));
        assert_eq!(f.of("other"), None);
        assert_eq!(Flags::parse("nope"), Flags::default());
    }

    #[test]
    fn worktrees_are_read_against_the_trunk() {
        use crate::testrepo::TempRepo;
        let r = TempRepo::new("queue-read");
        r.write(&r.repo, "a.txt", "one\n");
        r.git(&r.repo, &["add", "-A"]);
        r.git(&r.repo, &["commit", "-q", "-m", "a"]);
        let wt = r.root.join("lane-wt");
        r.git(&r.repo, &["worktree", "add", "-q", "-b", "lane", wt.to_str().unwrap()]);
        r.write(&wt, "b.txt", "b\n");
        r.git(&wt, &["add", "-A"]);
        r.git(&wt, &["commit", "-q", "-m", "b"]);
        let wts = crate::worktree::list(&r.repo).unwrap();
        let trees = crate::track::read(&r.repo, &wts, &mut crate::track::Cache::default()).unwrap();
        let mut cache = Cache::default();
        let got = read(&r.repo, &trees, &Checks::default(), &Flags::default(), &mut cache).unwrap();
        assert_eq!(got.rows.len(), 1, "the trunk's own worktree has none");
        let row = &got.rows[0];
        assert_eq!((row.stage, row.checked, row.total), (Stage::Unreviewed, 0, 1));
        assert!(got.of(&row.path).is_some());
        // A look makes it in review; a check on its one file, approved.
        let head = Oid::parse(&trees.trees.iter().find(|t| t.wt.branch.as_deref() == Some("lane")).unwrap().wt.head).unwrap();
        review::write_seen(&r.repo, "lane", &head).unwrap();
        let got = read(&r.repo, &trees, &Checks::default(), &Flags::default(), &mut cache).unwrap();
        assert_eq!(got.rows[0].stage, Stage::InReview);
        let blob = Oid::parse(&r.out(&wt, &["rev-parse", "HEAD:b.txt"])).unwrap();
        let mut checks = Checks::default();
        checks.set("lane", &("b.txt".to_owned(), Some(blob)), true);
        let got = read(&r.repo, &trees, &checks, &Flags::default(), &mut cache).unwrap();
        assert_eq!(got.rows[0].stage, Stage::Approved);
    }
}
