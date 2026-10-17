//! The trial merge: several branches taken into the trunk one after another,
//! in a chosen order, in memory. Each step is `git merge-tree --write-tree`
//! of what the steps before made and the next branch, so nothing is checked
//! out and no branch, index or working folder moves (only objects are
//! written, which nothing points to). The result is where each step clashed
//! and what the end differs from the trunk by. A step that clashes goes on
//! with the markers git wrote into the tree, as if left unsolved, so the
//! steps after it still show their own clashes. The calls block; run them off
//! the UI thread.

use std::path::Path;

use crate::git;
use crate::oid::Oid;
use crate::predict::{self, Conflict};

/// One branch taken in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// The name asked for (a branch).
    pub name: String,
    /// The clash this step made, `None` when it merged clean.
    pub conflict: Option<Conflict>,
    /// Everything of it was in already: nothing to merge.
    pub already: bool,
}

/// A file the end differs in from the trunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changed {
    pub path: String,
    /// Lines added and removed; `None` for a binary file.
    pub lines: Option<(u64, u64)>,
}

/// What a trial found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trial {
    pub trunk: String,
    pub steps: Vec<Step>,
    /// The tree the last step made.
    pub tree: Oid,
    /// How the end differs from the trunk.
    pub changed: Vec<Changed>,
}

impl Trial {
    /// The places to resolve over all steps (see [`Conflict::places`]).
    pub fn places(&self) -> u32 {
        self.steps.iter().filter_map(|s| s.conflict.as_ref()).map(Conflict::places).sum()
    }

    /// How many steps clashed.
    pub fn clashing(&self) -> usize {
        self.steps.iter().filter(|s| s.conflict.is_some()).count()
    }

    /// Lines added and removed over the files that have a count.
    pub fn lines(&self) -> (u64, u64) {
        self.changed.iter().filter_map(|c| c.lines).fold((0, 0), |(a, d), (x, y)| (a + x, d + y))
    }
}

/// Takes `branches` into `trunk` in their order, in the repository `repo`
/// (any of its worktrees). Fails when a name is unknown, or when the
/// histories are unrelated.
pub fn run(repo: &Path, trunk: &str, branches: &[String]) -> Result<Trial, git::Error> {
    for rev in std::iter::once(trunk).chain(branches.iter().map(String::as_str)) {
        if rev.starts_with('-') {
            return Err(git::Error::Git(format!("bad revision '{rev}'")));
        }
    }
    let start = commit_of(repo, trunk)?;
    let (mut head, mut tree) = (start, start_tree(repo, start)?);
    let mut steps = Vec::new();
    for name in branches {
        let tip = commit_of(repo, name)?;
        if git::check(repo, &["--no-optional-locks", "merge-base", "--is-ancestor", &tip.to_string(), &head.to_string()])? {
            steps.push(Step { name: name.clone(), conflict: None, already: true });
            continue;
        }
        let (made, conflict) = predict::merge(repo, &head.to_string(), &tip.to_string())?;
        head = join(repo, made, head, tip)?;
        tree = made;
        steps.push(Step { name: name.clone(), conflict, already: false });
    }
    let changed = diff(repo, &start.to_string(), &tree.to_string())?;
    Ok(Trial { trunk: trunk.to_owned(), steps, tree, changed })
}

fn commit_of(repo: &Path, rev: &str) -> Result<Oid, git::Error> {
    let text = git::read_text(repo, &["--no-optional-locks", "rev-parse", "--verify", "--end-of-options", &format!("{rev}^{{commit}}")])?;
    Oid::parse(text.trim()).ok_or_else(|| git::Error::Git(format!("unknown revision '{rev}'")))
}

fn start_tree(repo: &Path, commit: Oid) -> Result<Oid, git::Error> {
    let text = git::read_text(repo, &["--no-optional-locks", "rev-parse", &format!("{commit}^{{tree}}")])?;
    Oid::parse(text.trim()).ok_or_else(|| git::Error::Io(format!("git rev-parse: unexpected output: {}", text.trim())))
}

/// A commit of `tree` with parents `a` and `b`, so the next step has a merge
/// base that knows both. Nothing points to it.
fn join(repo: &Path, tree: Oid, a: Oid, b: Oid) -> Result<Oid, git::Error> {
    let mut c = git::command();
    c.arg("-C").arg(repo).args(["--no-optional-locks", "commit-tree", &tree.to_string(), "-p", &a.to_string(), "-p", &b.to_string()]);
    c.args(["-m", "michi trial", "--no-gpg-sign"]);
    c.env("GIT_AUTHOR_NAME", "michi").env("GIT_AUTHOR_EMAIL", "michi@example.invalid").env("GIT_AUTHOR_DATE", "@0 +0000");
    c.env("GIT_COMMITTER_NAME", "michi").env("GIT_COMMITTER_EMAIL", "michi@example.invalid").env("GIT_COMMITTER_DATE", "@0 +0000");
    let out = c.output().map_err(git::started)?;
    if !out.status.success() {
        return Err(git::refused(&String::from_utf8_lossy(&out.stderr)));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Oid::parse(text.trim()).ok_or_else(|| git::Error::Io(format!("git commit-tree: unexpected output: {}", text.trim())))
}

fn diff(repo: &Path, from: &str, to: &str) -> Result<Vec<Changed>, git::Error> {
    let out = git::read(repo, &["--no-optional-locks", "diff", "--numstat", "-z", "--no-renames", "--no-ext-diff", from, to])?;
    Ok(parse_numstat(&out))
}

/// The records `added TAB removed TAB path NUL` of `git diff --numstat -z
/// --no-renames`; a binary file has `-` for both counts.
fn parse_numstat(out: &[u8]) -> Vec<Changed> {
    out.split(|&b| b == 0)
        .filter(|r| !r.is_empty())
        .filter_map(|r| {
            let text = String::from_utf8_lossy(r);
            let mut parts = text.splitn(3, '\t');
            let (a, d, path) = (parts.next()?, parts.next()?, parts.next()?);
            let lines = a.parse().ok().zip(d.parse().ok());
            Some(Changed { path: path.to_owned(), lines })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn lines(first: &str, last: &str) -> String {
        format!("{first}\nb\nc\nd\ne\nf\ng\nh\ni\n{last}\n")
    }

    /// `main` with `f.txt`, and the branches `one`, `two`, `three` off it:
    /// `one` and `two` both change the first line (they clash), `three`
    /// adds a file.
    fn repo(name: &str) -> TempRepo {
        let r = TempRepo::new(name);
        r.write(&r.repo, "f.txt", &lines("a", "j"));
        r.git(&r.repo, &["add", "."]);
        r.git(&r.repo, &["commit", "-q", "-m", "base"]);
        for (branch, file, content) in
            [("one", "f.txt", lines("one", "j")), ("two", "f.txt", lines("two", "j")), ("three", "g.txt", "new\n".to_owned())]
        {
            r.git(&r.repo, &["checkout", "-q", "-b", branch, "main"]);
            r.write(&r.repo, file, &content);
            r.git(&r.repo, &["add", "."]);
            r.git(&r.repo, &["commit", "-q", "-m", branch]);
        }
        r.git(&r.repo, &["checkout", "-q", "main"]);
        r
    }

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn numstat_is_read() {
        let out = b"3\t1\tf.txt\0-\t-\timg.png\0";
        assert_eq!(
            parse_numstat(out),
            [Changed { path: "f.txt".into(), lines: Some((3, 1)) }, Changed { path: "img.png".into(), lines: None }]
        );
        assert!(parse_numstat(b"").is_empty());
    }

    #[test]
    fn clean_branches_merge_in_any_order() {
        let r = repo("trial-clean");
        for order in [["one", "three"], ["three", "one"]] {
            let t = run(&r.repo, "main", &names(&order)).unwrap();
            assert_eq!(t.clashing(), 0);
            assert_eq!(t.places(), 0);
            let files: Vec<_> = t.changed.iter().map(|c| c.path.as_str()).collect();
            assert_eq!(files, ["f.txt", "g.txt"]);
            assert_eq!(t.lines(), (2, 1));
        }
    }

    #[test]
    fn the_second_of_two_clashing_branches_is_where_it_clashes() {
        let r = repo("trial-clash");
        let t = run(&r.repo, "main", &names(&["one", "two", "three"])).unwrap();
        assert_eq!(t.clashing(), 1);
        assert!(t.steps[0].conflict.is_none());
        let c = t.steps[1].conflict.as_ref().expect("two clashes with one");
        assert_eq!(c.files[0].path, "f.txt");
        assert_eq!(t.places(), 1);
        // The step after goes on from the markers and is clean.
        assert!(t.steps[2].conflict.is_none());
        // The other order moves the clash to the other branch.
        let t = run(&r.repo, "main", &names(&["two", "one"])).unwrap();
        assert!(t.steps[0].conflict.is_none() && t.steps[1].conflict.is_some());
    }

    #[test]
    fn a_branch_already_in_is_skipped_and_nothing_moves() {
        let r = repo("trial-already");
        let before = r.out(&r.repo, &["for-each-ref", "--format=%(refname) %(objectname)"]);
        let status = r.out(&r.repo, &["status", "--porcelain=v2"]);
        let t = run(&r.repo, "main", &names(&["main", "one"])).unwrap();
        assert!(t.steps[0].already && !t.steps[1].already);
        assert_eq!(r.out(&r.repo, &["for-each-ref", "--format=%(refname) %(objectname)"]), before);
        assert_eq!(r.out(&r.repo, &["status", "--porcelain=v2"]), status);
        let none = run(&r.repo, "main", &[]).unwrap();
        assert!(none.changed.is_empty());
    }

    #[test]
    fn unknown_names_fail() {
        let r = repo("trial-unknown");
        assert!(run(&r.repo, "main", &names(&["nope"])).is_err());
        assert!(run(&r.repo, "main", &names(&["--help"])).is_err());
        assert!(run(&r.repo, "nope", &names(&["one"])).is_err());
    }
}
