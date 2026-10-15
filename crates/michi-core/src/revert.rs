//! Reverting a commit: a new commit on a worktree's branch that undoes it.
//! One that does not go in cleanly is called off, leaving the worktree as it
//! was; the files that clashed say why.

use std::path::Path;

use crate::branch::{self, Doing};
use crate::git;
use crate::oid::Oid;

/// How a revert ended, when git did not refuse it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reverted {
    /// The new commit.
    Made(Oid),
    /// It clashed in these files, and was called off.
    Conflict(Vec<String>),
    /// Its changes are undone already: nothing to commit, and nothing done.
    Empty,
}

/// Reverts `id` on the branch of the worktree at `wt`, with git's own
/// message. A merge needs `mainline`, the parent (from 1) whose side stays.
/// A worktree in the middle of something (a merge, another revert) is
/// refused, and so is one whose changes the revert would overwrite.
pub fn revert(wt: &Path, id: &Oid, mainline: Option<u32>) -> Result<Reverted, git::Error> {
    if let Some(d) = branch::doing(wt) {
        return Err(git::Error::Git(format!("this worktree is in the middle of a {}", d.word())));
    }
    let id = id.to_string();
    let m = mainline.map(|n| n.to_string());
    let mut args = vec!["revert", "--no-edit"];
    if let Some(m) = &m {
        args.extend(["-m", m]);
    }
    args.push(&id);
    let out = git::command().arg("-C").arg(wt).args(&args).output().map_err(git::started)?;
    if out.status.success() {
        return Ok(Reverted::Made(Oid::parse(&git::read_text(wt, &["rev-parse", "HEAD"])?).ok_or_else(no_head)?));
    }
    let said = String::from_utf8_lossy(&out.stderr);
    if branch::doing(wt) != Some(Doing::Revert) {
        // Nothing to commit: git says so on its standard output only.
        if said.trim().is_empty() && out.status.code() == Some(1) {
            return Ok(Reverted::Empty);
        }
        return Err(git::refused_all(&said));
    }
    let failed = git::refused_all(&said);
    // Stopped half-way: called off, whatever stopped it.
    let clashed = git::read_text(wt, &["diff", "--name-only", "--diff-filter=U"]);
    let staged = git::check(wt, &["diff", "--cached", "--quiet", "HEAD"]);
    branch::write(wt, &["revert", "--abort"])?;
    let clashed: Vec<String> = clashed?.lines().map(str::to_owned).collect();
    if !clashed.is_empty() {
        Ok(Reverted::Conflict(clashed))
    } else if staged? {
        Ok(Reverted::Empty)
    } else {
        // Went in, and the commit was refused (a hook): what it said.
        Err(failed)
    }
}

fn no_head() -> git::Error {
    git::Error::Io("HEAD: no commit".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn commit(t: &TempRepo, file: &str, text: &str) -> Oid {
        t.write(&t.repo, file, text);
        t.git(&t.repo, &["add", file]);
        t.git(&t.repo, &["commit", "-q", "-m", &format!("{file}: {}", text.trim())]);
        Oid::parse(&t.out(&t.repo, &["rev-parse", "HEAD"])).unwrap()
    }

    #[test]
    fn a_revert_is_a_new_commit_that_undoes() {
        let t = TempRepo::configured("revert-made");
        commit(&t, "a.txt", "1\n");
        let two = commit(&t, "a.txt", "2\n");
        commit(&t, "b.txt", "b\n");
        let Reverted::Made(new) = revert(&t.repo, &two, None).unwrap() else { panic!() };
        assert_eq!(t.out(&t.repo, &["rev-parse", "HEAD"]), new.to_string());
        assert_eq!(std::fs::read_to_string(t.repo.join("a.txt")).unwrap(), "1\n");
        assert!(t.out(&t.repo, &["log", "-1", "--format=%s"]).starts_with("Revert \"a.txt: 2\""));
        assert_eq!(revert(&t.repo, &two, None).unwrap(), Reverted::Empty, "undone already");
        assert_eq!(t.out(&t.repo, &["rev-parse", "HEAD"]), new.to_string());
        assert_eq!(branch::doing(&t.repo), None);
    }

    #[test]
    fn a_clash_is_called_off() {
        let t = TempRepo::configured("revert-clash");
        commit(&t, "a.txt", "1\n");
        let two = commit(&t, "a.txt", "2\n");
        let three = commit(&t, "a.txt", "3\n");
        t.write(&t.repo, "c.txt", "mine\n");
        assert_eq!(revert(&t.repo, &two, None).unwrap(), Reverted::Conflict(vec!["a.txt".into()]));
        assert_eq!(branch::doing(&t.repo), None);
        assert_eq!(t.out(&t.repo, &["rev-parse", "HEAD"]), three.to_string());
        assert_eq!(std::fs::read_to_string(t.repo.join("a.txt")).unwrap(), "3\n");
        assert!(t.repo.join("c.txt").exists(), "untracked files stay");
    }

    #[test]
    fn a_merge_needs_its_side_and_a_busy_worktree_is_refused() {
        let t = TempRepo::configured("revert-merge");
        commit(&t, "a.txt", "1\n");
        t.git(&t.repo, &["switch", "-q", "-c", "side"]);
        commit(&t, "b.txt", "b\n");
        t.git(&t.repo, &["switch", "-q", "main"]);
        commit(&t, "c.txt", "c\n");
        t.git(&t.repo, &["merge", "-q", "--no-edit", "side"]);
        let merge = Oid::parse(&t.out(&t.repo, &["rev-parse", "HEAD"])).unwrap();
        assert!(revert(&t.repo, &merge, None).unwrap_err().to_string().contains("-m"));
        let Reverted::Made(_) = revert(&t.repo, &merge, Some(1)).unwrap() else { panic!() };
        assert!(!t.repo.join("b.txt").exists(), "the side's change undone");
        assert!(t.repo.join("c.txt").exists());

        t.git(&t.repo, &["switch", "-q", "-c", "x"]);
        commit(&t, "a.txt", "x\n");
        t.git(&t.repo, &["switch", "-q", "main"]);
        let head = commit(&t, "a.txt", "y\n");
        let out = git::command().arg("-C").arg(&t.repo).args(["merge", "x"]).output().unwrap();
        assert!(!out.status.success(), "a clash");
        let e = revert(&t.repo, &head, None).unwrap_err();
        assert_eq!(e.to_string(), "this worktree is in the middle of a merge");
        assert_eq!(branch::doing(&t.repo), Some(Doing::Merge), "left as it was");
    }
}
