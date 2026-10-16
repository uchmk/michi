//! The worktrees of a repository, from `git worktree list --porcelain -z`.

use std::path::{Path, PathBuf};

use crate::git;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Worktree {
    /// Its top folder, as git prints it (forward slashes on Windows too).
    pub path: PathBuf,
    /// The commit checked out; empty for a bare repository.
    pub head: String,
    /// The branch, `refs/heads/` taken off; `None` when detached or bare.
    pub branch: Option<String>,
    pub bare: bool,
    pub detached: bool,
    /// Why it is locked (empty when no reason was given); `None` when not.
    pub locked: Option<String>,
    /// Why git would prune it (its folder is gone); `None` when it would not.
    pub prunable: Option<String>,
}

/// Every worktree of the repository `dir` is in, the main one first.
pub fn list(dir: &Path) -> Result<Vec<Worktree>, git::Error> {
    Ok(parse(&git::read(dir, &["worktree", "list", "--porcelain", "-z"])?))
}

/// Takes the worktree at `wt` away, its branch stays. What is uncommitted
/// (untracked files too) is kept first as a commit under
/// `refs/michi/backup/`; that ref's name, `None` when the worktree was clean.
/// Refuses one with untracked files too big to be kept.
pub fn remove(dir: &Path, wt: &Path) -> Result<Option<String>, git::Error> {
    let snap = crate::predict::snapshot(wt, crate::predict::Limits::DEFAULT)?;
    let mut saved = None;
    if let Some(snap) = snap {
        if !snap.left_out.is_empty() {
            return Err(git::Error::Git("it has untracked files too big to keep".to_owned()));
        }
        if let Some(commit) = snap.commit {
            saved = Some(crate::backup::save(dir, &commit, &format!("worktree {} removed with changes", wt.display()))?);
        }
    }
    crate::branch::write(dir, &["worktree", "remove", "--force", &wt.to_string_lossy()])?;
    Ok(saved)
}

/// The records of `--porcelain -z`: each line ends with a NUL, and an empty
/// line ends a worktree.
pub fn parse(out: &[u8]) -> Vec<Worktree> {
    let text = String::from_utf8_lossy(out);
    let mut all: Vec<Worktree> = Vec::new();
    let mut open = false;
    for line in text.split('\0') {
        if line.is_empty() {
            open = false;
            continue;
        }
        if let Some(path) = line.strip_prefix("worktree ") {
            all.push(Worktree { path: PathBuf::from(path), ..Default::default() });
            open = true;
            continue;
        }
        let Some(w) = all.last_mut().filter(|_| open) else { continue };
        let (word, rest) = line.split_once(' ').unwrap_or((line, ""));
        match word {
            "HEAD" => w.head = rest.to_owned(),
            "branch" => w.branch = Some(rest.strip_prefix("refs/heads/").unwrap_or(rest).to_owned()),
            "bare" => w.bare = true,
            "detached" => w.detached = true,
            "locked" => w.locked = Some(rest.to_owned()),
            "prunable" => w.prunable = Some(rest.to_owned()),
            _ => {}
        }
    }
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn remove_keeps_the_branch_and_the_uncommitted_change() {
        let t = TempRepo::configured("wt-remove");
        let wt = t.root.join("wt-x");
        t.git(&t.repo, &["worktree", "add", "-q", "-b", "x", wt.to_str().unwrap(), "main"]);
        assert_eq!(remove(&t.repo, &wt).unwrap(), None);
        assert!(!wt.exists());
        t.git(&t.repo, &["worktree", "add", "-q", wt.to_str().unwrap(), "x"]);
        t.write(
            &wt, "new.txt", "new
",
        );
        let saved = remove(&t.repo, &wt).unwrap().expect("a backup");
        assert!(saved.starts_with(crate::backup::PREFIX));
        assert!(!wt.exists());
        assert_eq!(t.out(&t.repo, &["show", &format!("{saved}:new.txt")]), "new");
        assert!(!t.out(&t.repo, &["branch", "--list", "x"]).is_empty());
    }

    #[test]
    fn porcelain_records_are_read() {
        let out = b"worktree C:/dev/michi\0HEAD 1111111111111111111111111111111111111111\0branch refs/heads/main\0\0\
worktree C:/dev/michi-wt/fix\0HEAD 2222222222222222222222222222222222222222\0detached\0locked\0\0\
worktree C:/dev/gone\0HEAD 3333333333333333333333333333333333333333\0branch refs/heads/feat/x\0locked moving it\0\
prunable gitdir file points to non-existent location\0\0";
        let w = parse(out);
        assert_eq!(w.len(), 3);
        assert_eq!(w[0].path, PathBuf::from("C:/dev/michi"));
        assert_eq!(w[0].branch.as_deref(), Some("main"));
        assert!(!w[0].detached && w[0].locked.is_none());
        assert!(w[1].detached && w[1].branch.is_none());
        assert_eq!(w[1].locked.as_deref(), Some(""));
        assert_eq!(w[2].branch.as_deref(), Some("feat/x"));
        assert_eq!(w[2].locked.as_deref(), Some("moving it"));
        assert!(w[2].prunable.as_deref().is_some_and(|p| p.starts_with("gitdir")));
    }

    #[test]
    fn a_bare_repository_has_no_head() {
        let w = parse(b"worktree /srv/michi.git\0bare\0\0worktree /srv/wt\0HEAD 4444\0branch refs/heads/main\0\0");
        assert!(w[0].bare && w[0].head.is_empty() && w[0].branch.is_none());
        assert_eq!(w[1].branch.as_deref(), Some("main"));
    }

    #[test]
    fn worktrees_of_a_real_repository() {
        let r = TempRepo::new("worktree-list");
        r.git(&r.repo, &["worktree", "add", "-q", "-b", "feat", "../wt-feat"]);
        let w = list(&r.repo).unwrap();
        assert_eq!(w.len(), 2);
        assert_eq!(w[0].branch.as_deref(), Some("main"));
        assert_eq!(w[1].branch.as_deref(), Some("feat"));
        assert_eq!(w[0].head, w[1].head);
        assert_eq!(w[0].head.len(), 40);
        // Asked from the linked worktree, the list is the same.
        assert_eq!(list(&r.root.join("wt-feat")).unwrap(), w);
    }
}
