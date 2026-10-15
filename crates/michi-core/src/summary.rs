//! The few facts the title bar shows about the repository a window is on.

use std::path::{Path, PathBuf};

use crate::git;
use crate::paths::same_path;
use crate::worktree::{self, Worktree};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    /// The repository's name: the main worktree's folder.
    pub name: String,
    /// What the worktree the window was opened in has checked out.
    pub head: Head,
    /// How many worktrees the repository has, the main one included.
    pub worktrees: usize,
    /// The top folder of the worktree the window was opened in.
    pub top: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Head {
    Branch(String),
    /// The first 7 characters of the commit.
    Detached(String),
    Bare,
}

/// The summary of the repository `dir` is in.
pub fn summary(dir: &Path) -> Result<Summary, git::Error> {
    let (all, top) = git::both(|| worktree::list(dir), || git::read_text(dir, &["rev-parse", "--show-toplevel"]));
    let all = all?;
    let first = all.first().ok_or_else(|| git::Error::Git("git worktree list printed nothing".into()))?;
    // A bare repository has no top folder; it is the first entry then.
    let here = match top {
        Ok(top) => all.iter().find(|w| same_path(&w.path, Path::new(&top))).unwrap_or(first),
        Err(_) => first,
    };
    Ok(Summary { name: name_of(&first.path), head: head_of(here), worktrees: all.len(), top: here.path.clone() })
}

fn head_of(w: &Worktree) -> Head {
    match &w.branch {
        Some(b) => Head::Branch(b.clone()),
        None if w.bare => Head::Bare,
        None => Head::Detached(w.head.chars().take(7).collect()),
    }
}

/// `C:/dev/michi` → `michi`, `/srv/michi.git` → `michi`, `/srv/michi/.git` → `michi`.
fn name_of(path: &Path) -> String {
    let file = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let name = file(path);
    let name = if name == ".git" { path.parent().map(file).unwrap_or_default() } else { name };
    let name = name.strip_suffix(".git").map(str::to_owned).unwrap_or(name);
    if name.is_empty() {
        path.to_string_lossy().into_owned()
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn names_come_from_the_main_folder() {
        assert_eq!(name_of(Path::new("C:/dev/michi")), "michi");
        assert_eq!(name_of(Path::new("/srv/michi.git")), "michi");
        assert_eq!(name_of(Path::new("/srv/michi/.git")), "michi");
        assert_eq!(name_of(Path::new("/")), "/");
    }

    #[test]
    fn each_worktree_tells_its_own_head() {
        let r = TempRepo::new("summary");
        r.git(&r.repo, &["worktree", "add", "-q", "-b", "feat", "../wt-feat"]);
        r.git(&r.repo, &["worktree", "add", "-q", "--detach", "../wt-detached"]);

        let main = summary(&r.repo).unwrap();
        assert_eq!(main.name, "repo");
        assert_eq!(main.head, Head::Branch("main".into()));
        assert_eq!(main.worktrees, 3);
        assert!(same_path(&main.top, &r.repo));

        // From a folder inside a linked worktree: its branch, the same name.
        let sub = r.root.join("wt-feat").join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        let feat = summary(&sub).unwrap();
        assert_eq!(feat.name, "repo");
        assert_eq!(feat.head, Head::Branch("feat".into()));
        assert!(same_path(&feat.top, &r.root.join("wt-feat")));

        let detached = summary(&r.root.join("wt-detached")).unwrap();
        let Head::Detached(short) = &detached.head else { panic!("{:?}", detached.head) };
        assert_eq!(short.len(), 7);
    }
}
