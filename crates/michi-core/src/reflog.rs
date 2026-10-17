//! The reflog screen's list: where HEAD has been (`git reflog`) and what
//! michi kept under `refs/michi/backup/`, newest first in one list, and the
//! way back to a row: a new branch at its commit. Nothing is moved or
//! dropped to get there, so it needs no backup of its own.

use std::path::Path;

use crate::backup;
use crate::branch;
use crate::git;
use crate::oid::Oid;

/// Most rows read from HEAD's reflog.
const HEAD_ROWS: usize = 300;
/// Most backups listed.
const BACKUPS: usize = 100;

/// Where a row comes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// HEAD's reflog: `HEAD@{place}`.
    Head(usize),
    /// A ref under `refs/michi/backup/`, by its full name.
    Backup(String),
}

/// A row: a commit HEAD was once on, or one michi kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: Oid,
    pub source: Source,
    /// What happened (`commit: fix`, `checkout: moving from a to b`) or why
    /// michi kept it (`pushed over origin/main`).
    pub what: String,
    /// Subject of the commit.
    pub subject: String,
    /// When, in seconds since the epoch.
    pub when: i64,
}

impl Entry {
    /// The name to show: `HEAD@{3}`, `michi/backup/20261016-142233`.
    pub fn label(&self) -> String {
        match &self.source {
            Source::Head(n) => format!("HEAD@{{{n}}}"),
            Source::Backup(name) => backup::short(name).to_owned(),
        }
    }

    pub fn is_backup(&self) -> bool {
        matches!(self.source, Source::Backup(_))
    }
}

/// The rows, newest first: HEAD's reflog as the worktree at `dir` has it,
/// and every backup.
pub fn list(dir: &Path) -> Result<Vec<Entry>, git::Error> {
    let mut rows = read(dir, "HEAD", HEAD_ROWS, false)?;
    let names = git::read_text(dir, &["for-each-ref", "--sort=-refname", "--format=%(refname)", backup::PREFIX])?;
    for name in names.lines().take(BACKUPS) {
        rows.extend(read(dir, name, 1, true)?);
    }
    // Stable: rows of one second stay in the order they were read.
    rows.sort_by_key(|e| std::cmp::Reverse(e.when));
    Ok(rows)
}

/// The newest `n` lines of the reflog of `name`; `kept` when it is a backup.
fn read(dir: &Path, name: &str, n: usize, kept: bool) -> Result<Vec<Entry>, git::Error> {
    let limit = format!("-n{n}");
    let args = ["reflog", "show", "--date=unix", "--format=%H%x00%gd%x00%gs%x00%s", &limit, name];
    let out = match git::read_text(dir, &args) {
        Ok(o) => o,
        // A repository with no commit yet has no HEAD to read.
        Err(git::Error::Git(_)) if name == "HEAD" => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut rows = Vec::new();
    for (place, line) in out.lines().enumerate() {
        let mut f = line.splitn(4, '\0');
        let (Some(id), Some(sel), Some(what), Some(subject)) = (f.next(), f.next(), f.next(), f.next()) else { continue };
        let Some(id) = Oid::parse(id) else { continue };
        // `HEAD@{1760000000}`: with `--date=unix` the selector holds the time.
        let when = sel.rsplit_once("@{").and_then(|(_, t)| t.strip_suffix('}')).and_then(|t| t.parse().ok()).unwrap_or(0);
        let (source, what) = if kept {
            (Source::Backup(name.to_owned()), what.strip_prefix("michi: ").unwrap_or(what).to_owned())
        } else {
            (Source::Head(place), what.to_owned())
        };
        rows.push(Entry { id, source, what, subject: subject.to_owned(), when });
    }
    Ok(rows)
}

/// Brings `id` back as a new branch, `recovered/<short id>` (`-2`… after it
/// when that is taken). The branch's name.
pub fn recover(dir: &Path, id: &Oid) -> Result<String, git::Error> {
    for n in 1..100 {
        let name = if n == 1 { format!("recovered/{}", id.short()) } else { format!("recovered/{}-{n}", id.short()) };
        if git::read_maybe(dir, &["rev-parse", "--verify", "-q", &format!("refs/heads/{name}")])?.is_some() {
            continue;
        }
        branch::create(dir, &name, id, None)?;
        return Ok(name);
    }
    Err(git::Error::Io(format!("recovered/{}: too many branches", id.short())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn head(t: &TempRepo) -> Oid {
        Oid::parse(&t.out(&t.repo, &["rev-parse", "HEAD"])).unwrap()
    }

    #[test]
    fn head_moves_and_backups_are_one_list_newest_first() {
        let t = TempRepo::configured("reflog-list");
        t.git(&t.repo, &["commit", "-q", "--allow-empty", "-m", "second"]);
        let second = head(&t);
        t.git(&t.repo, &["reset", "-q", "--hard", "HEAD~1"]);
        let name = backup::save(&t.repo, &second, "reset away").unwrap();
        let rows = list(&t.repo).unwrap();
        assert!(rows.len() >= 4, "{rows:?}");
        assert!(rows.windows(2).all(|w| w[0].when >= w[1].when), "{rows:?}");
        let kept = rows.iter().find(|e| e.is_backup()).expect("the backup");
        assert_eq!((kept.id, kept.what.as_str(), kept.subject.as_str()), (second, "reset away", "second"));
        assert_eq!(kept.source, Source::Backup(name));
        assert!(kept.label().starts_with("michi/backup/"), "{}", kept.label());
        let moves: Vec<_> = rows.iter().filter(|e| !e.is_backup()).collect();
        assert_eq!(moves[0].label(), "HEAD@{0}");
        assert!(moves[0].what.starts_with("reset:"), "{}", moves[0].what);
        assert_eq!(moves[1].id, second);
        assert_eq!(moves[1].subject, "second");
        assert!(moves.last().unwrap().what.starts_with("commit (initial)"));
    }

    #[test]
    fn a_row_comes_back_as_a_new_branch() {
        let t = TempRepo::configured("reflog-recover");
        t.git(&t.repo, &["commit", "-q", "--allow-empty", "-m", "second"]);
        let second = head(&t);
        t.git(&t.repo, &["reset", "-q", "--hard", "HEAD~1"]);
        let a = recover(&t.repo, &second).unwrap();
        assert_eq!(a, format!("recovered/{}", second.short()));
        assert_eq!(t.out(&t.repo, &["rev-parse", &a]), second.to_string());
        let b = recover(&t.repo, &second).unwrap();
        assert_eq!(b, format!("{a}-2"));
        // HEAD did not move.
        assert_ne!(head(&t), second);
    }

    #[test]
    fn a_repository_without_commits_has_no_rows() {
        let t = TempRepo::configured("reflog-empty");
        let fresh = t.repo.parent().unwrap().join("fresh");
        std::fs::create_dir_all(&fresh).unwrap();
        t.git(&fresh, &["init", "-q", "-b", "main"]);
        assert_eq!(list(&fresh).unwrap(), Vec::new());
    }
}
