//! Blame and the history of one file: who last touched each line
//! (`git blame --porcelain`) and the commits that touched the file,
//! following renames (`git log --follow`). Both are reads, so they run with
//! the read-only environment of [`git::read`].

use std::collections::HashMap;
use std::path::Path;

use crate::git;
use crate::oid::Oid;

/// Most commits listed in a file's history.
pub const HISTORY_ROWS: usize = 300;

/// A line of the file and the commit that last changed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub commit: Oid,
    /// The file's name in that commit (it differs after a rename).
    pub path: String,
    pub author: String,
    /// When the commit was written, in seconds since the epoch.
    pub when: i64,
    pub summary: String,
    /// Not committed yet: the line is in the working tree only.
    pub uncommitted: bool,
    /// Before the oldest commit blame looked at (a shallow clone's edge).
    pub boundary: bool,
    pub text: String,
}

/// A commit that touched the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub id: Oid,
    pub author: String,
    pub when: i64,
    pub subject: String,
}

/// Both lists for one file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blamed {
    pub lines: Vec<Line>,
    pub history: Vec<Change>,
}

#[derive(Clone, Default)]
struct Meta {
    path: String,
    author: String,
    when: i64,
    summary: String,
    boundary: bool,
}

/// Reads `git blame --porcelain` output. The header of a commit comes only
/// before its first line, so what was learned of each commit is kept.
pub fn parse_porcelain(text: &str) -> Vec<Line> {
    let mut metas: HashMap<Oid, Meta> = HashMap::new();
    let mut lines = Vec::new();
    let mut current: Option<Oid> = None;
    for raw in text.split('\n') {
        if let Some(body) = raw.strip_prefix('\t') {
            if let Some(id) = current.take() {
                let m = metas.get(&id).cloned().unwrap_or_default();
                lines.push(Line {
                    commit: id,
                    path: m.path,
                    author: m.author,
                    when: m.when,
                    summary: m.summary,
                    uncommitted: id.is_zero(),
                    boundary: m.boundary,
                    text: body.trim_end_matches('\r').to_owned(),
                });
            }
            continue;
        }
        let (key, value) = raw.split_once(' ').unwrap_or((raw, ""));
        if let Some(id) = Oid::parse(key) {
            current = Some(id);
            metas.entry(id).or_default();
            continue;
        }
        let Some(id) = current else { continue };
        let m = metas.entry(id).or_default();
        match key {
            "author" => m.author = value.to_owned(),
            "author-time" => m.when = value.parse().unwrap_or(0),
            "summary" => m.summary = value.to_owned(),
            "filename" => m.path = value.to_owned(),
            "boundary" => m.boundary = true,
            _ => {}
        }
    }
    lines
}

/// Reads the commits of `git log --format=%H%x00%an%x00%at%x00%s`.
pub fn parse_history(text: &str) -> Vec<Change> {
    text.lines()
        .filter_map(|line| {
            let mut f = line.splitn(4, '\0');
            let id = Oid::parse(f.next()?)?;
            let author = f.next()?.to_owned();
            let when = f.next()?.parse().ok()?;
            Some(Change { id, author, when, subject: f.next()?.to_owned() })
        })
        .collect()
}

/// Who last changed each line of `path` as of `rev` (the working tree, with
/// its uncommitted lines, when none), and the commits that changed the file
/// up to `rev` (HEAD when none), renames followed. `-M` follows lines moved
/// within the file; `-w` ignores whitespace-only changes.
pub fn read(dir: &Path, rev: Option<&Oid>, path: &str) -> Result<Blamed, git::Error> {
    let rev_s = rev.map(Oid::to_string);
    let mut args = vec!["blame", "--porcelain", "-M", "-w"];
    if let Some(r) = &rev_s {
        args.push(r);
    }
    args.extend(["--", path]);
    let text = match git::read(dir, &args) {
        Ok(t) => t,
        // A file HEAD has never had (new, not committed): every line is uncommitted.
        Err(e) => return new_file(dir, rev, path).ok_or(e),
    };
    let lines = parse_porcelain(&String::from_utf8_lossy(&text));
    let n = format!("-n{HISTORY_ROWS}");
    let mut log = vec!["log", "--follow", "--format=%H%x00%an%x00%at%x00%s", &n];
    log.push(rev_s.as_deref().unwrap_or("HEAD"));
    log.extend(["--", path]);
    let history = parse_history(&String::from_utf8_lossy(&git::read(dir, &log)?));
    Ok(Blamed { lines, history })
}

/// The working-tree file `path` when HEAD has no such path: all its lines are
/// "not committed yet", and it has no history.
fn new_file(dir: &Path, rev: Option<&Oid>, path: &str) -> Option<Blamed> {
    if rev.is_some() || !git::read(dir, &["ls-tree", "HEAD", "--", path]).map_or(true, |o| o.is_empty()) {
        return None;
    }
    let bytes = std::fs::read(dir.join(path)).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let zero = Oid::parse(&"0".repeat(40))?;
    let lines = text
        .lines()
        .map(|l| Line {
            commit: zero,
            path: path.to_owned(),
            author: "Not Committed Yet".to_owned(),
            when: 0,
            summary: String::new(),
            uncommitted: true,
            boundary: false,
            text: l.to_owned(),
        })
        .collect();
    Some(Blamed { lines, history: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn id(c: char) -> String {
        c.to_string().repeat(40)
    }

    #[test]
    fn porcelain_headers_come_once_per_commit() {
        let a = id('a');
        let z = id('0');
        let text = format!(
            "{a} 1 1 2\nauthor Ann\nauthor-time 100\nsummary first\nfilename f.txt\n\tone\n{a} 2 2\n\ttwo\n\
             {z} 3 3 1\nauthor Not Committed Yet\nauthor-time 200\nsummary Version of f.txt\nfilename f.txt\n\tthree\r\n"
        );
        let lines = parse_porcelain(&text);
        assert_eq!(lines.len(), 3);
        assert_eq!((lines[0].author.as_str(), lines[0].when, lines[0].text.as_str()), ("Ann", 100, "one"));
        assert_eq!((lines[1].author.as_str(), lines[1].summary.as_str(), lines[1].text.as_str()), ("Ann", "first", "two"));
        assert!(!lines[1].uncommitted);
        assert!(lines[2].uncommitted);
        assert_eq!(lines[2].text, "three");
        assert_eq!(lines[1].path, "f.txt");
    }

    #[test]
    fn boundary_commits_are_marked() {
        let a = id('b');
        let lines = parse_porcelain(&format!("{a} 1 1 1\nauthor A\nauthor-time 1\nsummary s\nboundary\nfilename f\n\tx\n"));
        assert!(lines[0].boundary);
    }

    #[test]
    fn history_lines_are_read_and_bad_ones_skipped() {
        let a = id('a');
        let text = format!("{a}\0Ann\0100\0fix: a\nnot a line\n");
        let rows = parse_history(&text);
        assert_eq!(rows.len(), 1);
        assert_eq!((rows[0].author.as_str(), rows[0].when, rows[0].subject.as_str()), ("Ann", 100, "fix: a"));
    }

    #[test]
    fn a_new_uncommitted_file_is_all_uncommitted_lines() {
        let t = TempRepo::configured("blame-new");
        t.write(
            &t.repo, "a.txt", "a
",
        );
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "a"]);
        t.write(
            &t.repo,
            "dirty.txt",
            "x
y
",
        );
        let b = read(&t.repo, None, "dirty.txt").unwrap();
        assert_eq!(b.lines.len(), 2);
        assert!(b.lines.iter().all(|l| l.uncommitted));
        assert!(b.history.is_empty());
        assert!(read(&t.repo, None, "missing.txt").is_err());
    }

    #[test]
    fn a_real_file_is_blamed_through_a_rename_and_an_uncommitted_line() {
        let t = TempRepo::configured("blame-read");
        t.write(&t.repo, "old.txt", "one\ntwo\n");
        t.git(&t.repo, &["add", "."]);
        t.git(&t.repo, &["commit", "-q", "-m", "add old"]);
        let first = Oid::parse(&t.out(&t.repo, &["rev-parse", "HEAD"])).unwrap();
        t.git(&t.repo, &["mv", "old.txt", "new.txt"]);
        t.git(&t.repo, &["commit", "-q", "-m", "rename"]);
        t.write(&t.repo, "new.txt", "one\ntwo changed\n");
        t.git(&t.repo, &["commit", "-q", "-a", "-m", "change two"]);
        let third = Oid::parse(&t.out(&t.repo, &["rev-parse", "HEAD"])).unwrap();
        t.write(&t.repo, "new.txt", "one\ntwo changed\nthree\n");

        let b = read(&t.repo, None, "new.txt").unwrap();
        assert_eq!(b.lines.len(), 3);
        assert_eq!(b.lines[0].commit, first);
        assert_eq!(b.lines[0].summary, "add old");
        assert_eq!(b.lines[0].path, "old.txt");
        assert_eq!(b.lines[1].commit, third);
        assert!(b.lines[2].uncommitted);
        let subjects: Vec<&str> = b.history.iter().map(|c| c.subject.as_str()).collect();
        assert_eq!(subjects, ["change two", "rename", "add old"]);

        // As of a commit the working tree's extra line is not there.
        let at = read(&t.repo, Some(&third), "new.txt").unwrap();
        assert_eq!(at.lines.len(), 2);
        assert!(at.lines.iter().all(|l| !l.uncommitted));
    }

    #[test]
    fn a_missing_file_is_an_error() {
        let t = TempRepo::configured("blame-missing");
        assert!(read(&t.repo, None, "nope.txt").is_err());
    }
}
