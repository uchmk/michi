//! The submodules of a worktree: whether each is where the commit asks, and
//! `submodule update` to bring them there. michi does not go into one (the
//! scope of v1); it says where each stands and puts them back on the recorded
//! commit.

use std::path::Path;

use crate::branch::{self, Doing};
use crate::git;

/// Where a submodule stands against the commit the worktree records for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Checked out at the recorded commit.
    Same,
    /// Checked out at another commit than the recorded one (git's `+`).
    Differs,
    /// Not cloned or not set up yet (git's `-`).
    Missing,
    /// The merge left it with a clash (git's `U`).
    Clash,
}

impl State {
    /// An update would change it.
    pub fn needs_update(self) -> bool {
        matches!(self, State::Differs | State::Missing)
    }
}

/// One submodule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sub {
    pub path: String,
    /// The commit it is on (the recorded one for a missing one), shortened.
    pub id: String,
    pub state: State,
}

/// Reads the output of `git submodule status` (`<mark><sha> <path>[ (<describe>)]`).
pub fn parse(text: &str) -> Vec<Sub> {
    text.lines()
        .filter_map(|line| {
            let mut chars = line.chars();
            let state = match chars.next()? {
                ' ' => State::Same,
                '+' => State::Differs,
                '-' => State::Missing,
                'U' => State::Clash,
                _ => return None,
            };
            let (id, rest) = chars.as_str().split_once(' ')?;
            if id.is_empty() || !id.chars().all(|c| c.is_ascii_hexdigit()) {
                return None;
            }
            // The path may hold spaces; git adds ` (describe)` after it when it can.
            let path = match rest.rfind(" (") {
                Some(i) if rest.ends_with(')') => &rest[..i],
                _ => rest,
            };
            Some(Sub { path: path.to_owned(), id: id.chars().take(8).collect(), state })
        })
        .collect()
}

/// The submodules of the worktree at `wt` (the first level), in git's order.
pub fn list(wt: &Path) -> Result<Vec<Sub>, git::Error> {
    // `git submodule status` is a shell script on Windows and takes seconds to start; without a `.gitmodules` there is nothing to ask.
    if !wt.join(".gitmodules").exists() {
        return Ok(Vec::new());
    }
    Ok(parse(&git::read_text(wt, &["submodule", "status"])?))
}

/// `submodule update --init --recursive` on the worktree at `wt`: puts each
/// submodule on its recorded commit; one with changes git would overwrite is
/// refused by git and stays. A worktree in the middle of a merge or the like
/// is refused. The list after it.
pub fn update(wt: &Path) -> Result<Vec<Sub>, git::Error> {
    if let Some(d) = branch::doing(wt) {
        return Err(busy(d));
    }
    branch::write(wt, &["submodule", "update", "--init", "--recursive"])?;
    list(wt)
}

/// The paths of those in `subs` an update would change.
pub fn stale(subs: &[Sub]) -> Vec<&str> {
    subs.iter().filter(|s| s.state.needs_update()).map(|s| s.path.as_str()).collect()
}

fn busy(d: Doing) -> git::Error {
    git::Error::Git(format!("this worktree is in the middle of a {}", d.word()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn git_status_lines_are_read() {
        let text = " 1234567890abcdef1234567890abcdef12345678 libs/a (heads/main)\n\
                    +abcdef1234567890abcdef1234567890abcdef12 b c (v1.0-3-gabc)\n\
                    -0000000000000000000000000000000000000001 gone\n\
                    U1111111111111111111111111111111111111111 clash\n\
                    junk\n";
        let subs = parse(text);
        assert_eq!(subs.len(), 4);
        assert_eq!(subs[0], Sub { path: "libs/a".into(), id: "12345678".into(), state: State::Same });
        assert_eq!(subs[1].path, "b c", "a path with a space");
        assert_eq!(subs[1].state, State::Differs);
        assert_eq!(subs[2].state, State::Missing);
        assert_eq!(subs[3].state, State::Clash);
        assert_eq!(stale(&subs), vec!["b c", "gone"]);
        assert!(parse("").is_empty());
    }

    #[test]
    fn a_repository_without_submodules_has_none() {
        let t = TempRepo::configured("sub-none");
        assert!(list(&t.repo).unwrap().is_empty());
    }

    #[test]
    fn an_update_puts_a_moved_submodule_back_on_the_recorded_commit() {
        let t = TempRepo::configured("sub-update");
        let lib = t.root.join("lib");
        std::fs::create_dir_all(&lib).unwrap();
        t.git(&lib, &["init", "-q", "-b", "main"]);
        t.configure(&lib);
        t.git(&lib, &["commit", "-q", "--allow-empty", "-m", "lib one"]);
        t.git(&lib, &["commit", "-q", "--allow-empty", "-m", "lib two"]);
        let url = lib.to_string_lossy().into_owned();
        t.git(&t.repo, &["-c", "protocol.file.allow=always", "submodule", "add", "-q", &url, "vendor"]);
        t.git(&t.repo, &["commit", "-q", "-m", "add vendor"]);
        let subs = list(&t.repo).unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].state, State::Same);
        // Move the submodule off the recorded commit.
        t.git(&t.repo.join("vendor"), &["checkout", "-q", "HEAD~1"]);
        let subs = list(&t.repo).unwrap();
        assert_eq!(subs[0].state, State::Differs);
        assert_eq!(stale(&subs), vec!["vendor"]);
        let after = update(&t.repo).unwrap();
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].state, State::Same);
    }

    #[test]
    fn an_update_is_refused_in_the_middle_of_a_merge() {
        let t = TempRepo::configured("sub-busy");
        std::fs::write(t.repo.join(".git").join("MERGE_HEAD"), "x").unwrap();
        assert!(update(&t.repo).is_err());
    }
}
