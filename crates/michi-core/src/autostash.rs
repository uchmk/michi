//! Doing what local changes stand in the way of (a pull's fast-forward, a
//! switch) anyway: the work is put away as a stash, kept under
//! `refs/michi/backup/` too, the thing is done, and the stash is put back. A
//! clash on the way back is left marked in the worktree for the conflict view,
//! with the stash kept.

use std::path::Path;

use crate::backup;
use crate::branch;
use crate::git;
use crate::stash::{self, Applied, Saved};

/// What happened to the changes put away around a job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stashed {
    /// Where the changes are kept (`refs/michi/backup/…`), whatever came after.
    pub saved: String,
    pub back: Back,
}

/// How putting the changes back ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Back {
    /// Put back; the stash is gone.
    Done,
    /// Put back with clashes in these files, left marked in the worktree; the
    /// stash stays.
    Clash(Vec<String>),
    /// Not put back (git said why); the stash stays.
    Failed(String),
}

/// Runs `job` in the worktree at `wt`. When it fails and the worktree has
/// changes, they are put away (untracked files too), backed up, and `job` runs
/// again on the clean worktree; then they are put back. A second failure puts
/// them back and gives that failure. A worktree in the middle of a merge or
/// the like, or without changes, gives the first failure as it is.
pub fn around<T>(wt: &Path, why: &str, mut job: impl FnMut() -> Result<T, git::Error>) -> Result<(T, Option<Stashed>), git::Error> {
    let first = match job() {
        Ok(v) => return Ok((v, None)),
        Err(e) => e,
    };
    if branch::doing(wt).is_some() || git::read_text(wt, &["status", "--porcelain"]).map_or(true, |s| s.trim().is_empty()) {
        return Err(first);
    }
    let Saved::Made(id) = stash::save(wt, &format!("michi autostash: {why}"))? else { return Err(first) };
    let saved = match backup::save(wt, &id, &format!("autostash before {why}")) {
        Ok(saved) => saved,
        Err(e) => {
            let _ = stash::apply(wt, &id, true);
            return Err(e);
        }
    };
    match job() {
        Err(e) => match stash::apply(wt, &id, true) {
            Ok(Applied::Done) => Err(e),
            // The changes did not come back whole: say so, they are in `saved`.
            Ok(Applied::Conflict(files)) => {
                Err(git::Error::Git(format!("{e}; your changes clashed on the way back in {} (kept in {saved})", files.join(", "))))
            }
            Err(a) => Err(git::Error::Git(format!("{e}; your changes did not come back: {a} (kept in {saved})"))),
        },
        Ok(v) => {
            let back = match stash::apply(wt, &id, true) {
                Ok(Applied::Done) => Back::Done,
                Ok(Applied::Conflict(files)) => Back::Clash(files),
                Err(e) => Back::Failed(e.to_string()),
            };
            Ok((v, Some(Stashed { saved, back })))
        }
    }
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

    fn text(t: &TempRepo, file: &str) -> Option<String> {
        std::fs::read_to_string(t.repo.join(file)).ok()
    }

    /// `main` has `a.txt` = 1; `side` has it = 2 and `b.txt`.
    fn with_side(name: &str) -> TempRepo {
        let t = TempRepo::configured(name);
        commit(&t, "a.txt", "1\n");
        t.git(&t.repo, &["switch", "-q", "-c", "side"]);
        commit(&t, "a.txt", "2\n");
        commit(&t, "b.txt", "b\n");
        t.git(&t.repo, &["switch", "-q", "main"]);
        t
    }

    #[test]
    fn what_works_at_once_is_not_touched() {
        let t = with_side("autostash-plain");
        t.write(&t.repo, "mine.txt", "m\n");
        let (v, stashed) = around(&t.repo, "x", || branch::switch(&t.repo, "side").map(|()| 7)).unwrap();
        assert_eq!((v, stashed), (7, None));
        assert_eq!(text(&t, "mine.txt").as_deref(), Some("m\n"));
        assert!(stash::list(&t.repo).unwrap().is_empty());
    }

    #[test]
    fn changes_in_the_way_are_put_away_and_back() {
        let t = with_side("autostash-back");
        // Edits to a file the switch does not change go along on their own;
        // an untracked file the switch would overwrite is in the way.
        t.write(&t.repo, "b.txt", "mine\n");
        assert!(branch::switch(&t.repo, "side").is_err(), "git alone refuses");
        let (_, stashed) = around(&t.repo, "switch to side", || branch::switch(&t.repo, "side")).unwrap();
        let s = stashed.expect("stashed");
        assert!(s.saved.starts_with(backup::PREFIX), "{}", s.saved);
        assert_eq!(t.out(&t.repo, &["symbolic-ref", "--short", "HEAD"]), "side");
        // b.txt is tracked on side now: it cannot go back over it, and is kept.
        assert!(matches!(s.back, Back::Failed(_)), "{:?}", s.back);
        assert_eq!(stash::list(&t.repo).unwrap().len(), 1);
        assert_eq!(t.out(&t.repo, &["show", &format!("{}^3:b.txt", s.saved)]), "mine");
    }

    #[test]
    fn put_back_when_nothing_clashes() {
        let t = with_side("autostash-ok");
        commit(&t, "d.txt", "d\n");
        t.git(&t.repo, &["switch", "-q", "side"]);
        t.git(&t.repo, &["rm", "-q", "b.txt"]);
        t.git(&t.repo, &["commit", "-q", "-m", "rm b"]);
        t.git(&t.repo, &["switch", "-q", "main"]);
        t.write(&t.repo, "d.txt", "edited\n");
        t.write(&t.repo, "u.txt", "u\n");
        // `side` has no d.txt: the switch would lose the edit.
        assert!(branch::switch(&t.repo, "side").is_err());
        let (_, stashed) = around(&t.repo, "switch to side", || branch::switch(&t.repo, "side")).unwrap();
        let s = stashed.unwrap();
        assert_eq!(s.back, Back::Clash(vec!["d.txt".into()]), "deleted there, edited here");
        assert_eq!(text(&t, "u.txt").as_deref(), Some("u\n"));
    }

    #[test]
    fn a_clash_on_the_way_back_stays_marked_and_kept() {
        let t = with_side("autostash-clash");
        t.write(&t.repo, "a.txt", "mine\n");
        assert!(branch::switch(&t.repo, "side").is_err());
        let (_, stashed) = around(&t.repo, "switch to side", || branch::switch(&t.repo, "side")).unwrap();
        let s = stashed.unwrap();
        assert_eq!(s.back, Back::Clash(vec!["a.txt".into()]));
        assert!(text(&t, "a.txt").unwrap().contains("<<<<<<<"));
        assert_eq!(stash::list(&t.repo).unwrap().len(), 1, "the stash stays");
        assert_eq!(t.out(&t.repo, &["rev-parse", &s.saved]), stash::list(&t.repo).unwrap()[0].to_string());
    }

    #[test]
    fn a_job_that_fails_again_gives_the_work_back() {
        let t = with_side("autostash-fails");
        t.write(&t.repo, "a.txt", "mine\n");
        t.write(&t.repo, "new.txt", "n\n");
        let e = around(&t.repo, "x", || branch::switch(&t.repo, "nope")).unwrap_err();
        assert!(e.all().contains("nope"), "{}", e.all());
        assert_eq!(text(&t, "a.txt").as_deref(), Some("mine\n"));
        assert_eq!(text(&t, "new.txt").as_deref(), Some("n\n"));
        assert!(stash::list(&t.repo).unwrap().is_empty());
    }

    #[test]
    fn a_clean_worktree_gives_the_first_failure() {
        let t = with_side("autostash-clean");
        let e = around(&t.repo, "x", || branch::switch(&t.repo, "nope")).unwrap_err();
        assert!(e.all().contains("nope"), "{}", e.all());
        assert!(stash::list(&t.repo).unwrap().is_empty());
    }
}
