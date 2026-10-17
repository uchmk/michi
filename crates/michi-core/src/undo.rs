//! Undo and Redo for the moves of a branch (#64): a reset, a deleted branch,
//! a renamed one. The window keeps a [`Journal`] of what it did in this
//! session; `Ctrl+Z` runs the last entry's way back, `Ctrl+Shift+Z` the way
//! forward again.
//!
//! Nothing here loses work: a way back is refused when the repository is no
//! longer as the move left it (the branch has moved on, the worktree has
//! changes the move would clobber), and the branch tip it leaves is kept
//! under `refs/michi/backup/` first.

use std::path::{Path, PathBuf};

use crate::reset::Mode;
use crate::{backup, branch, conflict, git, oid::Oid};

/// How many moves the journal keeps.
pub const KEEP: usize = 20;

/// What was done, for the words a toast says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Reset(Mode),
    DeleteBranch,
    RenameBranch,
}

/// One step, ready to be run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Way {
    /// The branch of the worktree `wt`, now at `at`, to `to` the way `mode` says.
    Move { wt: PathBuf, at: Oid, to: Oid, mode: Mode },
    /// Makes the branch `name` at `at` (it must not exist).
    Make { name: String, at: Oid },
    /// Deletes the branch `name`, which must still be at `at`.
    Drop { name: String, at: Oid },
    /// Renames the branch `from` to `to`.
    Rename { from: String, to: String },
}

/// A move, and the two ways: back (`undo`) and forward again (`redo`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub kind: Kind,
    /// What the move was about: a branch's name, or the worktree's folder name.
    pub subject: String,
    pub undo: Way,
    pub redo: Way,
}

impl Entry {
    /// A reset of the worktree at `wt` from `from` to `to`.
    pub fn reset(wt: &Path, mode: Mode, from: Oid, to: Oid) -> Entry {
        let subject = wt.file_name().map_or_else(|| wt.display().to_string(), |n| n.to_string_lossy().into_owned());
        Entry {
            kind: Kind::Reset(mode),
            subject,
            undo: Way::Move { wt: wt.to_owned(), at: to, to: from, mode },
            redo: Way::Move { wt: wt.to_owned(), at: from, to, mode },
        }
    }

    /// The branch `name`, which was at `at`, deleted.
    pub fn deleted(name: &str, at: Oid) -> Entry {
        Entry {
            kind: Kind::DeleteBranch,
            subject: name.to_owned(),
            undo: Way::Make { name: name.to_owned(), at },
            redo: Way::Drop { name: name.to_owned(), at },
        }
    }

    /// The branch `old` renamed to `new`.
    pub fn renamed(old: &str, new: &str) -> Entry {
        Entry {
            kind: Kind::RenameBranch,
            subject: new.to_owned(),
            undo: Way::Rename { from: new.to_owned(), to: old.to_owned() },
            redo: Way::Rename { from: old.to_owned(), to: new.to_owned() },
        }
    }
}

/// The moves of this session, newest last, and those taken back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Journal {
    done: Vec<Entry>,
    undone: Vec<Entry>,
}

impl Journal {
    /// A new move: what was taken back can no longer be done again.
    pub fn record(&mut self, entry: Entry) {
        self.undone.clear();
        self.done.push(entry);
        if self.done.len() > KEEP {
            self.done.remove(0);
        }
    }

    /// The move Undo would take back.
    pub fn next_undo(&self) -> Option<&Entry> {
        self.done.last()
    }

    /// The move Redo would do again.
    pub fn next_redo(&self) -> Option<&Entry> {
        self.undone.last()
    }

    /// The last move was taken back.
    pub fn undid(&mut self) {
        if let Some(e) = self.done.pop() {
            self.undone.push(e);
        }
    }

    /// The last move taken back was done again.
    pub fn redid(&mut self) {
        if let Some(e) = self.undone.pop() {
            self.done.push(e);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.done.is_empty() && self.undone.is_empty()
    }
}

/// The commit the worktree at `wt` is on.
pub fn head(wt: &Path) -> Option<Oid> {
    tip(wt, "HEAD")
}

/// The commit the branch `name` is at.
pub fn branch_tip(dir: &Path, name: &str) -> Option<Oid> {
    tip(dir, &format!("refs/heads/{name}"))
}

fn tip(dir: &Path, rev: &str) -> Option<Oid> {
    let out = git::read_maybe(dir, &["rev-parse", "--verify", "-q", &format!("{rev}^{{commit}}")]).ok()??;
    Oid::parse(String::from_utf8_lossy(&out).trim())
}

fn refuse(why: String) -> git::Error {
    git::Error::Git(why)
}

/// Runs `way` in the repository `dir`, or says why it must not: the
/// repository is not as the move left it.
pub fn run(dir: &Path, way: &Way) -> Result<(), git::Error> {
    match way {
        Way::Move { wt, at, to, mode } => {
            conflict::idle(wt)?;
            if head(wt).as_ref() != Some(at) {
                return Err(refuse(format!("{} has moved on since: it is not at {} any more", wt.display(), at.short())));
            }
            let clean = git::read_text(wt, &["status", "--porcelain", "--untracked-files=no"])?.is_empty();
            let index_clean = git::read_maybe(wt, &["diff", "--cached", "--quiet"])?.is_some();
            // A hard reset left the tree clean, so only a clean tree may be
            // taken back: `--keep` then, which stops rather than clobber.
            // A soft or mixed one kept the files, which stay as they are.
            let flag = match mode {
                Mode::Hard => {
                    if !clean {
                        return Err(refuse("there are uncommitted changes: undoing would lose them".to_owned()));
                    }
                    "--keep"
                }
                Mode::Mixed => {
                    if !index_clean {
                        return Err(refuse("something is staged since: undoing would unstage it".to_owned()));
                    }
                    "--mixed"
                }
                Mode::Soft => "--soft",
            };
            backup::save(wt, at, "before undo or redo")?;
            git::read(wt, &["reset", "-q", flag, &to.to_string()])?;
            Ok(())
        }
        Way::Make { name, at } => {
            if branch_tip(dir, name).is_some() {
                return Err(refuse(format!("the branch '{name}' exists again")));
            }
            branch::create(dir, name, at, None)
        }
        Way::Drop { name, at } => {
            match branch_tip(dir, name) {
                Some(now) if now == *at => {}
                Some(_) => return Err(refuse(format!("the branch '{name}' has moved on since"))),
                None => return Err(refuse(format!("there is no branch '{name}'"))),
            }
            branch::delete(dir, name).map(|_| ())
        }
        Way::Rename { from, to } => branch::rename(dir, from, to),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn commit(t: &TempRepo, text: &str) -> Oid {
        t.write(&t.repo, "a.txt", text);
        t.git(&t.repo, &["add", "a.txt"]);
        t.git(&t.repo, &["commit", "-q", "-m", text.trim()]);
        head(&t.repo).unwrap()
    }

    fn repo(name: &str) -> (TempRepo, Oid, Oid) {
        let t = TempRepo::configured(name);
        let one = commit(&t, "1\n");
        let two = commit(&t, "2\n");
        (t, one, two)
    }

    #[test]
    fn the_journal_walks_back_and_forth_and_forgets_the_future_on_a_new_move() {
        let id = |c: char| Oid::parse(&c.to_string().repeat(40)).unwrap();
        let mut j = Journal::default();
        assert!(j.is_empty() && j.next_undo().is_none() && j.next_redo().is_none());
        j.record(Entry::deleted("a", id('a')));
        j.record(Entry::renamed("b", "c"));
        assert_eq!(j.next_undo().unwrap().kind, Kind::RenameBranch);
        j.undid();
        assert_eq!(j.next_undo().unwrap().kind, Kind::DeleteBranch);
        assert_eq!(j.next_redo().unwrap().kind, Kind::RenameBranch);
        j.redid();
        assert!(j.next_redo().is_none());
        j.undid();
        j.record(Entry::deleted("d", id('d')));
        assert!(j.next_redo().is_none(), "a new move ends the redo line");
    }

    #[test]
    fn the_journal_keeps_the_last_twenty() {
        let mut j = Journal::default();
        for i in 0..30 {
            j.record(Entry::renamed(&format!("a{i}"), &format!("b{i}")));
        }
        assert_eq!(j.done.len(), KEEP);
        assert_eq!(j.next_undo().unwrap().subject, "b29");
    }

    #[test]
    fn a_hard_reset_is_taken_back_and_done_again() {
        let (t, one, two) = repo("undo-hard");
        crate::reset::reset(&t.repo, &one, Mode::Hard).unwrap();
        let e = Entry::reset(&t.repo, Mode::Hard, two, one);
        run(&t.repo, &e.undo).unwrap();
        assert_eq!(head(&t.repo), Some(two));
        assert_eq!(std::fs::read_to_string(t.repo.join("a.txt")).unwrap().replace("\r\n", "\n"), "2\n");
        run(&t.repo, &e.redo).unwrap();
        assert_eq!(head(&t.repo), Some(one));
        assert!(!t.out(&t.repo, &["for-each-ref", "refs/michi/backup"]).is_empty(), "the tip it left is kept");
    }

    #[test]
    fn a_mixed_reset_is_taken_back_with_the_files_as_they_are() {
        let (t, one, two) = repo("undo-mixed");
        crate::reset::reset(&t.repo, &one, Mode::Mixed).unwrap();
        let e = Entry::reset(&t.repo, Mode::Mixed, two, one);
        run(&t.repo, &e.undo).unwrap();
        assert_eq!(head(&t.repo), Some(two));
        assert_eq!(t.out(&t.repo, &["status", "--porcelain"]), "");
    }

    #[test]
    fn a_soft_reset_is_taken_back() {
        let (t, one, two) = repo("undo-soft");
        crate::reset::reset(&t.repo, &one, Mode::Soft).unwrap();
        let e = Entry::reset(&t.repo, Mode::Soft, two, one);
        run(&t.repo, &e.undo).unwrap();
        assert_eq!(head(&t.repo), Some(two));
    }

    #[test]
    fn a_reset_is_not_taken_back_over_work_done_since() {
        let (t, one, two) = repo("undo-refused");
        crate::reset::reset(&t.repo, &one, Mode::Hard).unwrap();
        let e = Entry::reset(&t.repo, Mode::Hard, two, one);
        // Uncommitted change: refused, nothing moves.
        t.write(&t.repo, "a.txt", "mine\n");
        let err = run(&t.repo, &e.undo).unwrap_err();
        assert!(err.to_string().contains("uncommitted"), "{err}");
        assert_eq!(head(&t.repo), Some(one));
        // A new commit: the branch is not where the move left it.
        t.git(&t.repo, &["checkout", "-q", "--", "a.txt"]);
        let three = commit(&t, "3\n");
        let err = run(&t.repo, &e.undo).unwrap_err();
        assert!(err.to_string().contains("moved on"), "{err}");
        assert_eq!(head(&t.repo), Some(three));
    }

    #[test]
    fn a_mixed_reset_is_not_taken_back_over_a_staged_change() {
        let (t, one, two) = repo("undo-staged");
        crate::reset::reset(&t.repo, &one, Mode::Mixed).unwrap();
        let e = Entry::reset(&t.repo, Mode::Mixed, two, one);
        t.git(&t.repo, &["add", "a.txt"]);
        let err = run(&t.repo, &e.undo).unwrap_err();
        assert!(err.to_string().contains("staged"), "{err}");
    }

    #[test]
    fn a_deleted_branch_comes_back_and_goes_again() {
        let (t, _, two) = repo("undo-delete");
        t.git(&t.repo, &["branch", "side"]);
        let e = Entry::deleted("side", branch_tip(&t.repo, "side").unwrap());
        branch::delete(&t.repo, "side").unwrap();
        assert_eq!(branch_tip(&t.repo, "side"), None);
        run(&t.repo, &e.undo).unwrap();
        assert_eq!(branch_tip(&t.repo, "side"), Some(two));
        // Back twice is refused: it is there.
        assert!(run(&t.repo, &e.undo).is_err());
        run(&t.repo, &e.redo).unwrap();
        assert_eq!(branch_tip(&t.repo, "side"), None);
    }

    #[test]
    fn a_branch_that_moved_is_not_deleted_by_redo() {
        let (t, _, _) = repo("undo-redo-moved");
        t.git(&t.repo, &["branch", "side"]);
        let e = Entry::deleted("side", branch_tip(&t.repo, "side").unwrap());
        t.git(&t.repo, &["branch", "-f", "side", "HEAD~1"]);
        let err = run(&t.repo, &e.redo).unwrap_err();
        assert!(err.to_string().contains("moved on"), "{err}");
        assert!(branch_tip(&t.repo, "side").is_some());
    }

    #[test]
    fn a_rename_goes_back_and_forth() {
        let (t, _, _) = repo("undo-rename");
        t.git(&t.repo, &["branch", "old"]);
        branch::rename(&t.repo, "old", "new").unwrap();
        let e = Entry::renamed("old", "new");
        run(&t.repo, &e.undo).unwrap();
        assert!(branch_tip(&t.repo, "old").is_some() && branch_tip(&t.repo, "new").is_none());
        run(&t.repo, &e.redo).unwrap();
        assert!(branch_tip(&t.repo, "new").is_some());
    }
}
