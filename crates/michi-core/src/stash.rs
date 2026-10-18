//! The stashes: a worktree's uncommitted changes put away as a commit under
//! `refs/stash`, untracked files with them, then put back on a worktree or
//! dropped. A dropped one is kept under `refs/michi/backup/` first.

use std::path::Path;

use crate::backup;
use crate::branch::{self, Doing};
use crate::git;
use crate::oid::Oid;
use crate::show::{self, Detail};

/// How a save ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Saved {
    /// The new stash.
    Made(Oid),
    /// The worktree had nothing to put away.
    Nothing,
}

/// How putting a stash back ended, when git did not refuse it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Applied {
    /// Put back (and, popped, gone from the list).
    Done,
    /// Put back with clashes in these files, left marked in the worktree for
    /// the person to resolve; the stash stays.
    Conflict(Vec<String>),
}

/// The stashes, newest first: the k-th is `stash@{k}`.
pub fn list(dir: &Path) -> Result<Vec<Oid>, git::Error> {
    Ok(entries(dir)?.into_iter().map(|(id, _)| id).collect())
}

/// The stashes with what `git stash list` says of each, newest first.
fn entries(dir: &Path) -> Result<Vec<(Oid, String)>, git::Error> {
    let out = git::read_text(dir, &["stash", "list", "--format=%H%x00%gs"])?;
    Ok(out.lines().filter_map(|l| l.split_once('\0')).filter_map(|(id, s)| Some((Oid::parse(id)?, s.to_owned()))).collect())
}

/// Puts the changes of the worktree at `wt` away, untracked files too, with
/// `message` (git's own when it is empty). A worktree in the middle of a
/// merge or the like is refused.
pub fn save(wt: &Path, message: &str) -> Result<Saved, git::Error> {
    if let Some(d) = branch::doing(wt) {
        return Err(busy(d));
    }
    let before = list(wt)?.len();
    let message = message.trim();
    let mut args = vec!["stash", "push", "--include-untracked"];
    if !message.is_empty() {
        args.extend(["-m", message]);
    }
    branch::write(wt, &args)?;
    // Nothing to save is no error to git: the list says.
    let after = list(wt)?;
    match after.first() {
        Some(id) if after.len() > before => Ok(Saved::Made(*id)),
        _ => Ok(Saved::Nothing),
    }
}

/// Puts the stash `id` back on the worktree at `wt`; `pop` drops it from the
/// list once it went in without a clash (no backup: its changes are in the
/// worktree). One that clashes stays. Changes of the worktree it would
/// overwrite make git refuse it, with nothing done.
pub fn apply(wt: &Path, id: &Oid, pop: bool) -> Result<Applied, git::Error> {
    if let Some(d) = branch::doing(wt) {
        return Err(busy(d));
    }
    find(wt, id)?;
    // By its id, not its place: another stash made meanwhile moves places.
    let out = git::command().arg("-C").arg(wt).args(["stash", "apply", &id.to_string()]).output().map_err(git::started)?;
    if !out.status.success() {
        // NUL-separated: without `-z` git quotes a name like `é.txt`.
        let raw = git::read(wt, &["diff", "--name-only", "-z", "--diff-filter=U"])?;
        let clashed: Vec<String> =
            raw.split(|b| *b == 0).filter(|n| !n.is_empty()).map(|n| String::from_utf8_lossy(n).into_owned()).collect();
        if clashed.is_empty() {
            return Err(git::refused_all(&String::from_utf8_lossy(&out.stderr)));
        }
        return Ok(Applied::Conflict(clashed));
    }
    if pop {
        branch::write(wt, &["stash", "drop", "-q", &find(wt, id)?])?;
    }
    Ok(Applied::Done)
}

/// Drops the stash `id` after keeping it under `refs/michi/backup/`: the
/// backup's name.
pub fn drop(dir: &Path, id: &Oid) -> Result<String, git::Error> {
    let subject = entries(dir)?.into_iter().find(|(s, _)| s == id).map(|(_, said)| said).ok_or_else(|| gone(id))?;
    let saved = backup::save(dir, id, &format!("dropped stash: {subject}"))?;
    branch::write(dir, &["stash", "drop", "-q", &find(dir, id)?])?;
    Ok(saved)
}

/// The stash `id` in full, as [`show::read`] gives a commit: what it changed
/// against the commit it was made on, then the untracked files it put away
/// (all added). Its parents are the commit it was made on alone.
pub fn read(dir: &Path, id: &Oid, stop: &dyn Fn() -> bool) -> Result<Detail, git::Error> {
    let mut d = show::read(dir, id, true, stop)?;
    if let Some(untracked) = d.parents.get(2).copied() {
        if !stop() {
            let u = show::read(dir, &untracked, true, stop)?;
            d.files.extend(u.files);
            d.cut |= u.cut;
        }
    }
    d.parents.truncate(1);
    Ok(d)
}

/// `stash@{k}` of the stash `id`.
fn find(dir: &Path, id: &Oid) -> Result<String, git::Error> {
    match list(dir)?.iter().position(|s| s == id) {
        Some(k) => Ok(format!("stash@{{{k}}}")),
        None => Err(gone(id)),
    }
}

fn gone(id: &Oid) -> git::Error {
    git::Error::Git(format!("no stash {}", id.short()))
}

fn busy(d: Doing) -> git::Error {
    git::Error::Git(format!("this worktree is in the middle of a {}", d.word()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn commit(t: &TempRepo, file: &str, text: &str) {
        t.write(&t.repo, file, text);
        t.git(&t.repo, &["add", file]);
        t.git(&t.repo, &["commit", "-q", "-m", file]);
    }

    fn read_file(t: &TempRepo, file: &str) -> Option<String> {
        std::fs::read_to_string(t.repo.join(file)).ok()
    }

    #[test]
    fn a_save_puts_the_changes_away_untracked_too() {
        let t = TempRepo::configured("stash-save");
        commit(&t, "a.txt", "1\n");
        assert_eq!(save(&t.repo, "").unwrap(), Saved::Nothing);
        assert!(list(&t.repo).unwrap().is_empty());
        t.write(&t.repo, "a.txt", "2\n");
        t.write(&t.repo, "new.txt", "new\n");
        let Saved::Made(id) = save(&t.repo, "  half done ").unwrap() else { panic!() };
        assert_eq!(list(&t.repo).unwrap(), [id]);
        assert_eq!(t.out(&t.repo, &["stash", "list", "--format=%gs"]), "On main: half done");
        assert_eq!(read_file(&t, "a.txt").as_deref(), Some("1\n"));
        assert_eq!(read_file(&t, "new.txt"), None);
        assert_eq!(t.out(&t.repo, &["status", "--porcelain"]), "");

        let d = read(&t.repo, &id, &|| false).unwrap();
        let files: Vec<_> = d.files.iter().map(|f| (f.path.as_str(), f.status)).collect();
        assert_eq!(files, [("a.txt", show::Status::Modified), ("new.txt", show::Status::Added)]);
        assert_eq!(d.parents.len(), 1);
        assert_eq!(d.parents[0].to_string(), t.out(&t.repo, &["rev-parse", "HEAD"]));

        t.write(&t.repo, "b.txt", "b\n");
        let Saved::Made(second) = save(&t.repo, "").unwrap() else { panic!() };
        assert_eq!(list(&t.repo).unwrap(), [second, id]);
        assert!(t.out(&t.repo, &["stash", "list", "-1", "--format=%gs"]).starts_with("WIP on main:"));
    }

    #[test]
    fn apply_keeps_it_and_pop_drops_it() {
        let t = TempRepo::configured("stash-apply");
        commit(&t, "a.txt", "1\n");
        t.write(&t.repo, "a.txt", "2\n");
        t.write(&t.repo, "new.txt", "new\n");
        let Saved::Made(id) = save(&t.repo, "x").unwrap() else { panic!() };
        t.write(&t.repo, "other.txt", "o\n");
        let Saved::Made(other) = save(&t.repo, "other").unwrap() else { panic!() };

        assert_eq!(apply(&t.repo, &id, false).unwrap(), Applied::Done);
        assert_eq!(read_file(&t, "a.txt").as_deref(), Some("2\n"));
        assert_eq!(read_file(&t, "new.txt").as_deref(), Some("new\n"));
        assert_eq!(list(&t.repo).unwrap(), [other, id]);

        t.git(&t.repo, &["checkout", "-q", "--", "a.txt"]);
        std::fs::remove_file(t.repo.join("new.txt")).unwrap();
        assert_eq!(apply(&t.repo, &id, true).unwrap(), Applied::Done);
        assert_eq!(read_file(&t, "a.txt").as_deref(), Some("2\n"));
        assert_eq!(list(&t.repo).unwrap(), [other], "the older one popped, the newer one stays");
        assert!(apply(&t.repo, &id, false).unwrap_err().to_string().starts_with("no stash "));
    }

    #[test]
    fn a_clash_stays_and_overwriting_is_refused() {
        let t = TempRepo::configured("stash-clash");
        commit(&t, "a.txt", "1\n");
        t.write(&t.repo, "a.txt", "2\n");
        let Saved::Made(id) = save(&t.repo, "").unwrap() else { panic!() };

        t.write(&t.repo, "a.txt", "mine\n");
        let e = apply(&t.repo, &id, true).unwrap_err();
        assert!(e.all().contains("a.txt"), "{}", e.all());
        assert_eq!(read_file(&t, "a.txt").as_deref(), Some("mine\n"), "nothing done");

        t.git(&t.repo, &["checkout", "-q", "--", "a.txt"]);
        commit(&t, "a.txt", "3\n");
        assert_eq!(apply(&t.repo, &id, true).unwrap(), Applied::Conflict(vec!["a.txt".into()]));
        assert_eq!(list(&t.repo).unwrap(), [id], "a clashing pop keeps it");
        assert!(read_file(&t, "a.txt").unwrap().contains("<<<<<<<"));
    }

    #[test]
    fn a_clash_in_a_file_with_an_unusual_name_is_named_as_it_is() {
        let t = TempRepo::configured("stash-clash-name");
        commit(&t, "é a.txt", "1\n");
        t.write(&t.repo, "é a.txt", "2\n");
        let Saved::Made(id) = save(&t.repo, "").unwrap() else { panic!() };
        commit(&t, "é a.txt", "3\n");
        assert_eq!(apply(&t.repo, &id, true).unwrap(), Applied::Conflict(vec!["é a.txt".into()]));
    }

    #[test]
    fn a_drop_is_kept_and_a_busy_worktree_refused() {
        let t = TempRepo::configured("stash-drop");
        commit(&t, "a.txt", "1\n");
        t.write(&t.repo, "a.txt", "2\n");
        let Saved::Made(id) = save(&t.repo, "keep me").unwrap() else { panic!() };
        let saved = drop(&t.repo, &id).unwrap();
        assert!(saved.starts_with(backup::PREFIX), "{saved}");
        assert!(list(&t.repo).unwrap().is_empty());
        assert_eq!(t.out(&t.repo, &["rev-parse", &saved]), id.to_string());
        assert_eq!(t.out(&t.repo, &["reflog", "-1", "--format=%gs", &saved]), "michi: dropped stash: On main: keep me");
        assert!(drop(&t.repo, &id).is_err(), "gone");

        t.git(&t.repo, &["switch", "-q", "-c", "x"]);
        commit(&t, "a.txt", "x\n");
        t.git(&t.repo, &["switch", "-q", "main"]);
        commit(&t, "a.txt", "y\n");
        let out = git::command().arg("-C").arg(&t.repo).args(["merge", "x"]).output().unwrap();
        assert!(!out.status.success(), "a clash");
        assert_eq!(save(&t.repo, "").unwrap_err().to_string(), "this worktree is in the middle of a merge");
        assert_eq!(apply(&t.repo, &id, false).unwrap_err().to_string(), "this worktree is in the middle of a merge");
    }
}
