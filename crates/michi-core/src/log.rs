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
    /// When it was written (the author's date), in seconds since 1970 UTC.
    pub time: i64,
    /// The first line of the message.
    pub subject: String,
}

const FORMAT: &str = "--format=%H%n%P%n%an%n%at%n%s";

/// Where the history starts: every branch, remote branch and tag, and the
/// commits `tips` (the worktrees' heads, detached ones too).
fn args(tips: &[Oid]) -> Vec<String> {
    let mut a: Vec<String> = ["--branches", "--remotes", "--tags"].map(String::from).to_vec();
    a.extend(tips.iter().filter(|t| !t.is_zero()).map(Oid::to_string));
    a
}

/// A `git log` read a batch at a time. Dropping it stops git.
pub struct Reader {
    run: git::Running,
    done: bool,
}

impl Reader {
    pub fn start(dir: &Path, tips: &[Oid]) -> Result<Reader, git::Error> {
        let mut c = git::command();
        c.arg("-C").arg(dir);
        c.args(["log", "-z", "--date-order", "--no-show-signature", "--encoding=UTF-8", FORMAT]).args(args(tips));
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
    let mut lines = text.splitn(5, '\n');
    let id = Oid::parse(lines.next()?)?;
    let parents = lines.next()?.split(' ').filter(|p| !p.is_empty()).filter_map(Oid::parse).collect();
    let author = lines.next()?.to_owned();
    let time = lines.next()?.trim().parse().unwrap_or(0);
    let subject = lines.next().unwrap_or("").trim_end_matches('\n').to_owned();
    Some(Commit { id, parents, author, time, subject })
}

/// How many commits the whole history has (what [`Reader`] would read).
pub fn count(dir: &Path, tips: &[Oid]) -> Result<u64, git::Error> {
    let mut a = vec!["rev-list".to_owned(), "--count".to_owned()];
    a.extend(args(tips));
    let a: Vec<&str> = a.iter().map(String::as_str).collect();
    let text = git::read_text(dir, &a)?;
    text.trim().parse().map_err(|_| git::Error::Io(format!("unexpected `git rev-list --count`: {text}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn records_are_read() {
        let a = "a".repeat(40);
        let b = "b".repeat(40);
        let c = parse(format!("{a}\n{b} {b}\nyuu\\x 名前\n1767225600\nMerge branch 'feat' — ok").as_bytes()).unwrap();
        assert_eq!(c.id.to_string(), a);
        assert_eq!(c.parents.len(), 2);
        assert_eq!(c.author, "yuu\\x 名前");
        assert_eq!(c.time, 1_767_225_600);
        assert_eq!(c.subject, "Merge branch 'feat' — ok");
        let root = parse(format!("\n{a}\n\nx\n0\n").as_bytes()).unwrap();
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
