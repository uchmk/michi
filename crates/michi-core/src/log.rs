//! The commits, newest first, from `git log`: a batch at a time from a git
//! left running between batches, so the first screen comes from the first
//! batch and a long history is only read as far as it is looked at.
//!
//! The order is `--date-order`: never a parent before all of its children,
//! which the graph and the folding need.

use std::io::BufRead;
use std::path::Path;

use crate::git;
use crate::oid::Oid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    pub id: Oid,
    pub parents: Vec<Oid>,
    pub author: String,
    /// The author's e-mail address (what names their GitHub avatar).
    pub email: String,
    /// When it was written (the author's date), in seconds since 1970 UTC.
    pub time: i64,
    /// The first line of the message.
    pub subject: String,
}

const FORMAT: &str = "--format=%H%n%P%n%an%n%ae%n%at%n%s";

/// Where the history starts: every branch, remote branch and tag, and the
/// commits `tips` (the worktrees' heads, detached ones too).
fn args(tips: &[Oid], scope: &Scope) -> Vec<String> {
    if let Some(solo) = &scope.solo {
        return vec![solo.clone()];
    }
    let mut a: Vec<String> = Vec::new();
    for (pseudo, prefix) in [("--branches", "refs/heads/"), ("--remotes", "refs/remotes/"), ("--tags", "refs/tags/")] {
        // `--exclude` holds for the next pseudo-ref option only, so it is said again before each,
        // and it takes the name without that option's prefix.
        a.extend(scope.hidden.iter().filter_map(|h| h.strip_prefix(prefix)).map(|h| format!("--exclude={h}")));
        a.push(pseudo.to_owned());
    }
    a.extend(tips.iter().filter(|t| !t.is_zero()).map(Oid::to_string));
    a
}

/// Which branches the history is read from (issue #71): the whole of it by
/// default, without the `hidden` refs, or only the `solo` ref and what it
/// reaches. Refs are full names (`refs/heads/x`, `refs/remotes/origin/x`,
/// `refs/tags/v1`). A commit that a shown ref reaches stays shown.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scope {
    pub hidden: std::collections::BTreeSet<String>,
    pub solo: Option<String>,
}

impl Scope {
    /// Whether everything is shown.
    pub fn is_all(&self) -> bool {
        self.hidden.is_empty() && self.solo.is_none()
    }

    /// Whether `full` is not read from (hidden, or not the solo one).
    pub fn hides(&self, full: &str) -> bool {
        match &self.solo {
            Some(s) => s != full,
            None => self.hidden.contains(full),
        }
    }

    /// Hide `full` if it is shown, show it if hidden. Leaves solo.
    pub fn toggle_hidden(&mut self, full: &str) {
        self.solo = None;
        if !self.hidden.remove(full) {
            self.hidden.insert(full.to_owned());
        }
    }

    /// Show only `full`; again on the one that is solo, show all.
    pub fn toggle_solo(&mut self, full: &str) {
        self.solo = if self.solo.as_deref() == Some(full) { None } else { Some(full.to_owned()) };
    }

    /// Show everything again.
    pub fn clear(&mut self) {
        self.hidden.clear();
        self.solo = None;
    }

    /// Drop refs that no longer exist (`exists` says whether one does).
    pub fn retain(&mut self, exists: impl Fn(&str) -> bool) {
        self.hidden.retain(|h| exists(h));
        if self.solo.as_deref().is_some_and(|s| !exists(s)) {
            self.solo = None;
        }
    }
}

/// A `git log` read a batch at a time. Dropping it stops git.
pub struct Reader {
    run: git::Running,
    done: bool,
}

impl Reader {
    pub fn start(dir: &Path, tips: &[Oid]) -> Result<Reader, git::Error> {
        Self::start_in(dir, tips, &Scope::default())
    }

    /// Like [`start`](Self::start), read within `scope`.
    pub fn start_in(dir: &Path, tips: &[Oid], scope: &Scope) -> Result<Reader, git::Error> {
        let mut c = git::command();
        c.arg("-C").arg(dir);
        c.args(["log", "-z", "--date-order", "--no-show-signature", "--encoding=UTF-8", FORMAT]).args(args(tips, scope));
        Ok(Reader { run: git::Running::start(c, "log")?, done: false })
    }

    /// Up to `n` more commits; fewer, and [`done`](Self::done) then, at the
    /// end of the history.
    pub fn next_batch(&mut self, n: usize) -> Result<Vec<Commit>, git::Error> {
        let mut batch = Vec::new();
        let mut buf = Vec::new();
        while batch.len() < n && !self.done {
            buf.clear();
            let got = self.run.out.read_until(0, &mut buf).map_err(|e| self.run.io(e))?;
            if got == 0 {
                self.done = true;
                self.run.finish()?;
                break;
            }
            if buf.last() == Some(&0) {
                buf.pop();
            }
            if let Some(c) = parse(&buf) {
                batch.push(c);
            }
        }
        Ok(batch)
    }

    pub fn done(&self) -> bool {
        self.done
    }
}

/// One record: the hash, the parents, the author, the time, the subject,
/// a line each.
fn parse(record: &[u8]) -> Option<Commit> {
    let text = String::from_utf8_lossy(record);
    let text = text.trim_start_matches('\n');
    if text.is_empty() {
        return None;
    }
    let mut lines = text.splitn(6, '\n');
    let id = Oid::parse(lines.next()?)?;
    let parents = lines.next()?.split(' ').filter(|p| !p.is_empty()).filter_map(Oid::parse).collect();
    let author = lines.next()?.to_owned();
    let email = lines.next()?.to_owned();
    let time = lines.next()?.trim().parse().unwrap_or(0);
    let subject = lines.next().unwrap_or("").trim_end_matches('\n').to_owned();
    Some(Commit { id, parents, author, email, time, subject })
}

/// How many commits the whole history has (what [`Reader`] would read).
pub fn count(dir: &Path, tips: &[Oid]) -> Result<u64, git::Error> {
    count_in(dir, tips, &Scope::default())
}

/// Like [`count`], within `scope`.
pub fn count_in(dir: &Path, tips: &[Oid], scope: &Scope) -> Result<u64, git::Error> {
    let mut a = vec!["rev-list".to_owned(), "--count".to_owned()];
    a.extend(args(tips, scope));
    let a: Vec<&str> = a.iter().map(String::as_str).collect();
    let text = git::read_text(dir, &a)?;
    text.trim().parse().map_err(|_| git::Error::Io(format!("unexpected `git rev-list --count`: {text}")))
}

/// Whether the repository lacks the commit-graph (the index of the commits
/// `git gc` writes) and could have one: without it, `--date-order` walks
/// the whole history before the first commit comes. Not when it is shallow
/// or `core.commitGraph` is off, and not when git cannot say.
pub fn lacks_graph(dir: &Path) -> bool {
    let paths = ["objects/info/commit-graph", "objects/info/commit-graphs/commit-graph-chain"];
    let Ok(out) = git::read_text(dir, &["rev-parse", "--is-shallow-repository", "--git-path", paths[0], "--git-path", paths[1]]) else {
        return false;
    };
    let mut lines = out.lines();
    if lines.next() != Some("false") {
        return false;
    }
    if lines.any(|p| dir.join(p).exists()) {
        return false;
    }
    // Unset is on.
    git::read_text(dir, &["config", "--type=bool", "core.commitGraph"]).map_or(true, |v| v != "false")
}

/// Writes the commit-graph for every commit a ref reaches, as a chain git
/// adds to rather than rewrites. The history stays as it is: this is the
/// index `git gc` writes.
pub fn write_graph(dir: &Path) -> Result<(), git::Error> {
    git::read(dir, &["commit-graph", "write", "--reachable", "--split", "--no-progress"]).map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn a_repository_without_a_commit_graph_gets_one() {
        let r = TempRepo::new("log-graph");
        r.git(&r.repo, &["commit", "-q", "--allow-empty", "-m", "second"]);
        r.git(&r.repo, &["worktree", "add", "-q", "-b", "wt", "../wt"]);
        let wt = r.root.join("wt");
        assert!(lacks_graph(&r.repo));
        assert!(lacks_graph(&wt));
        r.git(&r.repo, &["config", "core.commitGraph", "false"]);
        assert!(!lacks_graph(&r.repo));
        r.git(&r.repo, &["config", "--unset", "core.commitGraph"]);

        // From a worktree it goes in the common folder.
        write_graph(&wt).unwrap();
        assert!(!lacks_graph(&r.repo));
        assert!(!lacks_graph(&wt));
        assert!(r.repo.join(".git/objects/info/commit-graphs/commit-graph-chain").exists());
        // The history reads the same.
        let mut log = Reader::start(&r.repo, &[]).unwrap();
        assert_eq!(log.next_batch(10).unwrap().iter().map(|c| c.subject.as_str()).collect::<Vec<_>>(), ["second", "first"]);
        assert!(!lacks_graph(&r.root.join("nowhere")));
    }

    #[test]
    fn records_are_read() {
        let a = "a".repeat(40);
        let b = "b".repeat(40);
        let c = parse(format!("{a}\n{b} {b}\nyuu\\x 名前\nyuu@users.noreply.github.com\n1767225600\nMerge branch 'feat' — ok").as_bytes())
            .unwrap();
        assert_eq!(c.id.to_string(), a);
        assert_eq!(c.parents.len(), 2);
        assert_eq!(c.author, "yuu\\x 名前");
        assert_eq!(c.email, "yuu@users.noreply.github.com");
        assert_eq!(c.time, 1_767_225_600);
        assert_eq!(c.subject, "Merge branch 'feat' — ok");
        let root = parse(format!("\n{a}\n\nx\n\n0\n").as_bytes()).unwrap();
        assert!(root.parents.is_empty());
        assert_eq!(root.subject, "");
        assert_eq!(parse(b""), None);
        assert_eq!(parse(b"\n"), None);
    }

    #[test]
    fn batches_come_newest_first_until_the_end() {
        let r = TempRepo::new("log-batches");
        for i in 0..4 {
            r.git(&r.repo, &["commit", "-q", "--allow-empty", "-m", &format!("c{i}")]);
        }
        r.git(&r.repo, &["worktree", "add", "-q", "--detach", "../wt"]);
        r.git(&r.root.join("wt"), &["commit", "-q", "--allow-empty", "-m", "detached"]);
        let tips: Vec<Oid> = crate::worktree::list(&r.repo).unwrap().iter().filter_map(|w| Oid::parse(&w.head)).collect();
        let mut log = Reader::start(&r.repo, &tips).unwrap();
        let first = log.next_batch(2).unwrap();
        assert_eq!(first.iter().map(|c| c.subject.as_str()).collect::<Vec<_>>(), ["detached", "c3"]);
        assert!(!log.done());
        let rest = log.next_batch(10).unwrap();
        assert_eq!(rest.iter().map(|c| c.subject.as_str()).collect::<Vec<_>>(), ["c2", "c1", "c0", "first"]);
        assert!(log.done());
        assert_eq!(first[0].parents, [first[1].id]);
        assert_eq!(first[1].author, "michi");
        assert!(first[1].time > crate::testrepo::START && first[1].time < first[0].time);
        assert!(log.next_batch(10).unwrap().is_empty());
        assert_eq!(count(&r.repo, &tips).unwrap(), 6);
    }

    fn subjects(r: &TempRepo, scope: &Scope) -> Vec<String> {
        let mut log = Reader::start_in(&r.repo, &[], scope).unwrap();
        log.next_batch(50).unwrap().into_iter().map(|c| c.subject).collect()
    }

    #[test]
    fn hiding_and_soloing_a_branch_narrow_the_history() {
        let r = TempRepo::new("log-scope");
        r.git(&r.repo, &["checkout", "-q", "-b", "feat"]);
        r.git(&r.repo, &["commit", "-q", "--allow-empty", "-m", "on-feat"]);
        r.git(&r.repo, &["checkout", "-q", "main"]);
        r.git(&r.repo, &["commit", "-q", "--allow-empty", "-m", "on-main"]);
        let mut scope = Scope::default();
        assert!(scope.is_all());
        assert_eq!(subjects(&r, &scope).len(), 3);
        assert_eq!(count_in(&r.repo, &[], &scope).unwrap(), 3);

        // Hidden: its own commits go, what main reaches stays.
        scope.toggle_hidden("refs/heads/feat");
        assert!(scope.hides("refs/heads/feat") && !scope.hides("refs/heads/main"));
        assert_eq!(subjects(&r, &scope), ["on-main", "first"]);
        assert_eq!(count_in(&r.repo, &[], &scope).unwrap(), 2);
        scope.toggle_hidden("refs/heads/feat");
        assert!(scope.is_all());

        // Solo: only that branch and its ancestors.
        scope.toggle_solo("refs/heads/feat");
        assert!(scope.hides("refs/heads/main"));
        assert_eq!(subjects(&r, &scope), ["on-feat", "first"]);
        scope.toggle_solo("refs/heads/feat");
        assert!(scope.is_all());

        // A commit a shown branch reaches stays even if the branch at it is hidden.
        r.git(&r.repo, &["branch", "at-first", "main~1"]);
        scope.toggle_hidden("refs/heads/at-first");
        assert_eq!(subjects(&r, &scope).len(), 3);
        scope.retain(|f| f != "refs/heads/at-first");
        assert!(scope.is_all());
    }

    #[test]
    fn hidden_remote_branches_and_tags_go_too() {
        let r = TempRepo::new("log-scope-refs");
        r.git(&r.repo, &["commit", "-q", "--allow-empty", "-m", "tagged"]);
        r.git(&r.repo, &["tag", "v1"]);
        r.git(&r.repo, &["commit", "-q", "--allow-empty", "-m", "head"]);
        r.git(&r.repo, &["update-ref", "refs/remotes/origin/side", "HEAD"]);
        r.git(&r.repo, &["checkout", "-q", "--detach", "HEAD~1"]);
        r.git(&r.repo, &["branch", "-q", "-f", "main", "HEAD~1"]);
        let mut scope = Scope::default();
        assert_eq!(subjects(&r, &scope).len(), 3);
        scope.toggle_hidden("refs/remotes/origin/side");
        assert_eq!(subjects(&r, &scope), ["tagged", "first"]);
        scope.clear();
        scope.toggle_hidden("refs/tags/v1");
        assert_eq!(subjects(&r, &scope).len(), 3, "the other refs still reach it");
    }

    #[test]
    fn a_reader_dropped_halfway_stops_git() {
        let r = TempRepo::new("log-drop");
        for i in 0..3 {
            r.git(&r.repo, &["commit", "-q", "--allow-empty", "-m", &format!("c{i}")]);
        }
        let mut log = Reader::start(&r.repo, &[]).unwrap();
        assert_eq!(log.next_batch(1).unwrap().len(), 1);
        drop(log);
    }

    #[test]
    fn an_empty_repository_has_no_history() {
        let r = TempRepo::new("log-empty");
        r.git(&r.repo, &["update-ref", "-d", "refs/heads/main"]);
        let mut log = Reader::start(&r.repo, &[]).unwrap();
        assert!(log.next_batch(10).unwrap().is_empty());
        assert!(log.done());
        assert_eq!(count(&r.repo, &[]).unwrap(), 0);
    }

    #[test]
    fn a_missing_commit_is_git_s_refusal() {
        let r = TempRepo::new("log-missing");
        let mut log = Reader::start(&r.repo, &[Oid::parse(&"1".repeat(40)).unwrap()]).unwrap();
        assert!(matches!(log.next_batch(10), Err(git::Error::Git(_))));
    }
}
