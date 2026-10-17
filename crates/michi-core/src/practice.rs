//! The practice repository (issue #117): a small repository in a temporary
//! folder, with some history, a merge, branches, a tag, an uncommitted change
//! and two branches that conflict when merged, so the keys can be learned
//! where nothing is lost. It is built by running git (off the UI thread).
//!
//! git runs with no user or system configuration and fixed names and dates,
//! so the repository is the same everywhere; what the repository keeps in its
//! own configuration (a name, no signing, no hooks) is what makes the commits
//! the person makes in it work on any machine.

use std::path::{Path, PathBuf};

use crate::git;

/// The branch checked out at the end: merge [`OTHER`] into it to meet the conflict.
pub const CONFLICT_HERE: &str = "topic-a";
/// The branch that conflicts with [`CONFLICT_HERE`].
pub const OTHER: &str = "topic-b";
/// The file both branches change.
pub const CONFLICT_FILE: &str = "colors.txt";

/// 2026-01-05 09:00 UTC, the first commit; each is a few hours after the one before.
const START: i64 = 1_767_603_600;

/// Makes `michi-practice-N` under `parent` (the first N free) holding the
/// repository, and returns its folder. Nothing outside it is touched; on a
/// failure the half-made folder is taken away.
pub fn create(parent: &Path) -> Result<PathBuf, git::Error> {
    let dir = (1..1000)
        .map(|n| parent.join(format!("michi-practice-{n}")))
        .find(|d| std::fs::create_dir(d).is_ok())
        .ok_or_else(|| git::Error::Io(format!("no free michi-practice-N folder in {}", parent.display())))?;
    match build(&dir) {
        Ok(()) => Ok(dir),
        Err(e) => {
            let _ = std::fs::remove_dir_all(&dir);
            Err(e)
        }
    }
}

/// The folder being made, and how many commits it has so far.
struct Maker<'a> {
    dir: &'a Path,
    n: i64,
}

impl Maker<'_> {
    /// Runs git with no configuration of the machine's, at the time of the
    /// commit being made.
    fn git(&self, args: &[&str]) -> Result<(), git::Error> {
        let date = format!("@{} +0000", START + self.n * 3 * 3600);
        let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
        let out = git::command()
            .arg("-C")
            .arg(self.dir)
            .args(args)
            .env("GIT_CONFIG_GLOBAL", null)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "Practice")
            .env("GIT_AUTHOR_EMAIL", "practice@example.invalid")
            .env("GIT_AUTHOR_DATE", &date)
            .env("GIT_COMMITTER_NAME", "Practice")
            .env("GIT_COMMITTER_EMAIL", "practice@example.invalid")
            .env("GIT_COMMITTER_DATE", &date)
            .output()
            .map_err(git::started)?;
        if out.status.success() {
            Ok(())
        } else {
            Err(git::refused_all(&String::from_utf8_lossy(&out.stderr)))
        }
    }

    fn write(&self, file: &str, text: &str) -> Result<(), git::Error> {
        std::fs::write(self.dir.join(file), text).map_err(|e| git::Error::Io(format!("{file}: {e}")))
    }

    /// Writes `file`, stages it and commits.
    fn commit(&mut self, file: &str, text: &str, message: &str) -> Result<(), git::Error> {
        self.write(file, text)?;
        self.git(&["add", "--", file])?;
        self.n += 1;
        self.git(&["commit", "-q", "-m", message])
    }
}

const README: &str = "# Practice\n\nThis repository is for trying michi out. Nothing here matters:\n\
the folder is temporary, and you can delete it any time.\n\nTry:\n\
- walk the history and the branches in the graph\n\
- switch to another branch\n\
- commit the change that is waiting (the dashed row at the top)\n\
- merge topic-b into topic-a: both changed colors.txt, so it stops at a conflict to solve\n";

fn build(dir: &Path) -> Result<(), git::Error> {
    let mut m = Maker { dir, n: 0 };
    m.git(&["init", "-q", "-b", "main"])?;
    // Kept in the repository itself: the person's own commits here must work
    // on a machine with no name set, no signing key and no hooks.
    for (k, v) in [
        ("user.name", "Practice"),
        ("user.email", "practice@example.invalid"),
        ("commit.gpgsign", "false"),
        ("core.autocrlf", "false"),
        ("core.hooksPath", "no-hooks"),
    ] {
        m.git(&["config", k, v])?;
    }
    m.commit("README.md", README, "Add the README")?;
    m.commit("list.txt", "apples\nbread\nmilk\n", "Add a shopping list")?;
    m.commit(CONFLICT_FILE, "red\ngreen\nblue\n", "Add the colors")?;
    m.git(&["tag", "v1.0"])?;

    // A fix on a branch of its own, merged after main has moved on.
    m.git(&["switch", "-q", "-c", "fix/list"])?;
    m.commit("list.txt", "apples\nbread\nmilk\neggs\n", "Add eggs to the list")?;
    m.git(&["switch", "-q", "main"])?;
    m.commit("notes.txt", "Things to remember.\n", "Add notes")?;
    m.n += 1;
    m.git(&["merge", "-q", "--no-ff", "-m", "Merge fix/list", "fix/list"])?;

    // A branch that is still going: two commits, not merged.
    m.git(&["switch", "-q", "-c", "feature/greeting"])?;
    m.commit("greeting.txt", "Hello\n", "Add a greeting")?;
    m.commit("greeting.txt", "Hello, world\n", "Make the greeting longer")?;

    // Two branches that changed the same line of the same file.
    m.git(&["switch", "-q", "main"])?;
    m.git(&["switch", "-q", "-c", OTHER])?;
    m.commit(CONFLICT_FILE, "red\nteal\nblue\n", "Make green teal")?;
    m.git(&["switch", "-q", "main"])?;
    m.git(&["switch", "-q", "-c", CONFLICT_HERE])?;
    m.commit(CONFLICT_FILE, "red\nlime\nblue\n", "Make green lime")?;

    // Left uncommitted: a change and a new file, the dashed row of the graph.
    m.write("list.txt", "apples\nbread\nmilk\neggs\ncheese\n")?;
    m.write("todo.txt", "buy cheese\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A parent folder of its own, removed at the end.
    struct Parent(PathBuf);

    impl Parent {
        fn new(name: &str) -> Parent {
            let p = std::env::temp_dir().join(format!("michi-test-{}-practice-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&p);
            std::fs::create_dir_all(&p).unwrap();
            Parent(p)
        }
    }

    impl Drop for Parent {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn text(dir: &Path, args: &[&str]) -> String {
        git::read_text(dir, args).unwrap()
    }

    #[test]
    fn it_has_history_with_a_merge_branches_a_tag_and_a_waiting_change() {
        let p = Parent::new("shape");
        let dir = create(&p.0).unwrap();
        assert_eq!(dir.file_name().unwrap(), "michi-practice-1");
        assert_eq!(text(&dir, &["rev-parse", "--abbrev-ref", "HEAD"]), CONFLICT_HERE);
        let branches = text(&dir, &["for-each-ref", "--format=%(refname:short)", "refs/heads"]);
        let mut names: Vec<&str> = branches.lines().collect();
        names.sort_unstable();
        assert_eq!(names, ["feature/greeting", "fix/list", "main", "topic-a", "topic-b"]);
        assert_eq!(text(&dir, &["tag", "--list"]), "v1.0");
        assert_eq!(text(&dir, &["rev-list", "--merges", "--count", "main"]), "1", "main has the merge of fix/list");
        assert_eq!(text(&dir, &["rev-list", "--count", "--all"]), "10");
        assert_eq!(text(&dir, &["status", "--porcelain"]), " M list.txt\n?? todo.txt");
    }

    #[test]
    fn the_commits_are_the_practice_ones_whatever_the_machine_has() {
        let p = Parent::new("author");
        let dir = create(&p.0).unwrap();
        let who = text(&dir, &["log", "--all", "--format=%an <%ae> %cn %aI"]);
        assert!(who.lines().all(|l| l.starts_with("Practice <practice@example.invalid> Practice 2026-01-")), "{who}");
        assert_eq!(text(&dir, &["config", "--local", "commit.gpgsign"]), "false");
    }

    #[test]
    fn merging_the_two_topics_stops_at_a_real_conflict() {
        let p = Parent::new("conflict");
        let dir = create(&p.0).unwrap();
        // Set aside the waiting change, as a person would before a merge.
        text(&dir, &["stash", "push", "-u", "-q"]);
        let out = git::command().arg("-C").arg(&dir).args(["merge", OTHER]).output().unwrap();
        assert!(!out.status.success(), "the merge must stop");
        assert_eq!(text(&dir, &["diff", "--name-only", "--diff-filter=U"]), CONFLICT_FILE);
        let file = std::fs::read_to_string(dir.join(CONFLICT_FILE)).unwrap();
        assert!(file.contains("<<<<<<<") && file.contains("lime") && file.contains("teal"), "{file}");
    }

    #[test]
    fn a_second_one_gets_a_folder_of_its_own_and_a_failure_leaves_nothing() {
        let p = Parent::new("twice");
        let a = create(&p.0).unwrap();
        let b = create(&p.0).unwrap();
        assert_ne!(a, b);
        assert!(a.join(".git").exists() && b.join(".git").exists());
        let missing = p.0.join("nope").join("deeper");
        assert!(create(&missing).is_err());
        assert!(!missing.exists());
    }
}
