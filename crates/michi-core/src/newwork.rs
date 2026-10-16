//! New work (`N`): which branches and which tsumugi calls start it. tsumugi
//! makes the worktrees (`tsumugi new <repo> --worktree <branch> --from <rev>
//! -- claude "<text>"`); what is here is the names and the command shown.

use std::path::Path;

/// One worktree to start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub branch: String,
    /// The commit-ish the branch starts from.
    pub from: String,
    /// What the Claude is asked; empty to just start it.
    pub text: String,
}

/// The branch the form starts with: `tsumugi/MMDD-HHMM`.
pub fn default_branch(now: &chrono::DateTime<chrono::Local>) -> String {
    format!("tsumugi/{}", now.format("%m%d-%H%M"))
}

/// The most worktrees one start makes.
pub const MAX: u8 = 8;

/// The worktrees for `count` (1 to [`MAX`]) copies of one task: `branch`,
/// or `branch-1`, `branch-2`… from two.
pub fn plans(branch: &str, from: &str, text: &str, count: u8) -> Vec<Plan> {
    let count = count.clamp(1, MAX);
    (1..=count)
        .map(|n| Plan {
            branch: if count == 1 { branch.to_owned() } else { format!("{branch}-{n}") },
            from: from.to_owned(),
            text: text.to_owned(),
        })
        .collect()
}

/// The folder beside the repository a branch's worktree is expected in:
/// `<repo>-<branch with / as ->`.
pub fn folder(repo: &Path, branch: &str) -> String {
    let name = repo.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned());
    format!("{name}-{}", branch.replace('/', "-"))
}

/// The arguments of `tsumugi` for `plan`.
pub fn args(repo: &Path, plan: &Plan) -> Vec<String> {
    let mut a = vec!["new".to_owned(), repo.to_string_lossy().into_owned(), "--worktree".into(), plan.branch.clone()];
    a.extend(["--from".to_owned(), plan.from.clone(), "--".into(), "claude".into()]);
    if !plan.text.trim().is_empty() {
        a.push(plan.text.clone());
    }
    a
}

/// The call as typed in a terminal, to show.
pub fn command_line(repo: &Path, plan: &Plan) -> String {
    let quote =
        |s: &str| if s.is_empty() || s.contains([' ', '"', '\'']) { format!("\"{}\"", s.replace('"', "\\\"")) } else { s.to_owned() };
    let words: Vec<String> = args(repo, plan).iter().map(|s| quote(s)).collect();
    format!("tsumugi {}", words.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn the_default_branch_is_the_time() {
        let t = chrono::Local.with_ymd_and_hms(2026, 10, 16, 9, 5, 0).unwrap();
        assert_eq!(default_branch(&t), "tsumugi/1016-0905");
    }

    #[test]
    fn many_copies_get_numbers_and_one_does_not() {
        let one = plans("a/b", "main", "", 1);
        assert_eq!(one.iter().map(|p| p.branch.as_str()).collect::<Vec<_>>(), ["a/b"]);
        let three = plans("a/b", "main", "x", 3);
        assert_eq!(three.iter().map(|p| p.branch.as_str()).collect::<Vec<_>>(), ["a/b-1", "a/b-2", "a/b-3"]);
        assert_eq!(plans("a", "main", "", 0).len(), 1);
        assert_eq!(plans("a", "main", "", 99).len(), usize::from(MAX));
    }

    #[test]
    fn the_folder_is_beside_the_repository() {
        assert_eq!(folder(Path::new("C:/dev/michi"), "tsumugi/1016-0905"), "michi-tsumugi-1016-0905");
    }

    #[test]
    fn the_command_is_shown_as_it_is_run() {
        let p = Plan { branch: "b".into(), from: "origin/main".into(), text: "fix the \"x\"".into() };
        assert_eq!(
            command_line(Path::new("C:/dev/michi"), &p),
            "tsumugi new C:/dev/michi --worktree b --from origin/main -- claude \"fix the \\\"x\\\"\""
        );
        let p = Plan { text: String::new(), ..p };
        assert_eq!(command_line(Path::new("r"), &p), "tsumugi new r --worktree b --from origin/main -- claude");
    }
}
