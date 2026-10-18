//! Tidying a branch's commits before a pull request: put them in another
//! order, fold some into the one before (squash keeps both messages, fixup
//! drops the folded one's), change a message (reword) or leave one out
//! (drop). It is `git rebase -i` on the commits since the trunk, with the
//! todo list handed over through `GIT_SEQUENCE_EDITOR` and no editor for the
//! messages (`GIT_EDITOR=true`; a message of the user's own goes in with an
//! `exec git commit --amend -F` line after the commit). The old tip is kept
//! under `refs/michi/backup/` first. A conflict stops the rebase as it does
//! on the command line; the conflict screen takes it from there.

use std::fs;
use std::path::{Path, PathBuf};

use crate::backup;
use crate::git;
use crate::oid::Oid;

/// What a line of the todo list does with its commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Pick,
    Reword,
    /// Folds into the commit above; both messages are kept.
    Squash,
    /// Folds into the commit above; its message goes.
    Fixup,
    Drop,
}

impl Action {
    fn word(self) -> &'static str {
        match self {
            Action::Pick | Action::Reword => "pick",
            Action::Squash => "squash",
            Action::Fixup => "fixup",
            Action::Drop => "drop",
        }
    }
}

/// A commit of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub id: Oid,
    pub subject: String,
    /// The message it has now, whole.
    pub full: String,
    pub action: Action,
    /// The whole message it gets (a reword, or a squash that is to read
    /// otherwise than both messages joined); `None` keeps git's.
    pub message: Option<String>,
}

/// Why the list cannot be run as it stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// Nothing is changed: no commit moved, folded, reworded or dropped.
    Nothing,
    /// The first commit left in the list folds into nothing.
    FirstFolds,
    /// Everything is dropped.
    Empty,
}

/// Where the branch left the trunk: the merge base of `HEAD` and `trunk`.
pub fn base(wt: &Path, trunk: &str) -> Result<Option<Oid>, git::Error> {
    Ok(git::read_maybe(wt, &["merge-base", "HEAD", trunk])?.and_then(|v| Oid::parse(String::from_utf8_lossy(&v).trim())))
}

/// What the screen starts from: the worktree's commits since the trunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub wt: PathBuf,
    pub base: Oid,
    pub trunk: String,
    pub entries: Vec<Entry>,
}

/// Reads the branch of `wt` against `trunk`.
pub fn plan(wt: &Path, trunk: &str) -> Result<Plan, git::Error> {
    let base = base(wt, trunk)?.ok_or_else(|| git::Error::Git(format!("no common ancestor with {trunk}")))?;
    let entries = entries(wt, &base)?;
    if entries.is_empty() {
        return Err(git::Error::Git(format!("no commits since {trunk}")));
    }
    Ok(Plan { wt: wt.to_owned(), base, trunk: trunk.to_owned(), entries })
}

/// The commits since `base`, the oldest first, each to be picked. A merge
/// among them is refused: the list would lose its shape.
pub fn entries(wt: &Path, base: &Oid) -> Result<Vec<Entry>, git::Error> {
    let range = format!("{base}..HEAD");
    let text = git::read_text(wt, &["log", "--reverse", "--format=%H%x1f%P%x1f%B%x1e", &range])?;
    let mut out = Vec::new();
    for rec in text.split('\x1e') {
        let mut f = rec.trim_start().splitn(3, '\x1f');
        let (Some(id), Some(parents), Some(full)) = (f.next().and_then(Oid::parse), f.next(), f.next()) else { continue };
        if parents.split_whitespace().count() > 1 {
            return Err(git::Error::Git(format!("{} is a merge: michi tidies a branch of plain commits", id.short())));
        }
        let full = full.trim_end().to_owned();
        let subject = full.lines().next().unwrap_or("").to_owned();
        out.push(Entry { id, subject, full, action: Action::Pick, message: None });
    }
    Ok(out)
}

/// The whole message of `id`, to start a reword from.
pub fn message(wt: &Path, id: &Oid) -> Result<String, git::Error> {
    Ok(git::read_text(wt, &["log", "-1", "--format=%B", &id.to_string()])?.trim_end().to_owned())
}

/// Whether the list can be run, and if not why.
pub fn check(list: &[Entry], original: &[Entry]) -> Result<(), Problem> {
    let kept: Vec<&Entry> = list.iter().filter(|e| e.action != Action::Drop).collect();
    let Some(first) = kept.first() else { return Err(Problem::Empty) };
    if matches!(first.action, Action::Squash | Action::Fixup) {
        return Err(Problem::FirstFolds);
    }
    let same = list.len() == original.len()
        && list.iter().zip(original).all(|(a, b)| a.id == b.id && matches!(a.action, Action::Pick) && a.message.is_none());
    if same {
        return Err(Problem::Nothing);
    }
    Ok(())
}

/// The todo list for git. `message_file(n)` is where the message of the
/// `n`th entry is kept.
pub fn todo(list: &[Entry], message_file: impl Fn(usize) -> String) -> String {
    let mut out = String::new();
    // The message that goes on the commit the current group ends in.
    let mut pending: Option<usize> = None;
    let flush = |out: &mut String, pending: &mut Option<usize>| {
        if let Some(n) = pending.take() {
            out.push_str(&format!("exec git commit --amend -q -F {}\n", sh_quote(&message_file(n))));
        }
    };
    for (n, e) in list.iter().enumerate() {
        if e.action == Action::Drop {
            out.push_str(&format!("drop {} {}\n", e.id, e.subject));
            continue;
        }
        if !matches!(e.action, Action::Squash | Action::Fixup) {
            flush(&mut out, &mut pending);
        }
        out.push_str(&format!("{} {} {}\n", e.action.word(), e.id, e.subject));
        if e.message.is_some() || e.action == Action::Reword {
            pending = Some(n);
        }
    }
    flush(&mut out, &mut pending);
    out
}

/// How the run ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Done; the old tip is under this backup ref.
    Done { backup: String },
    /// A commit clashed and git stopped in the middle: the old tip is under
    /// the backup ref, and `rebase --continue` or `--abort` goes on.
    Stopped { backup: String },
}

/// Runs the list on the commits since `base` in the worktree `wt`.
pub fn run(wt: &Path, base: &Oid, list: &[Entry]) -> Result<Outcome, git::Error> {
    if !git::read_text(wt, &["status", "--porcelain", "--untracked-files=no"])?.is_empty() {
        return Err(git::Error::Git("the worktree has changes that are not committed: commit or stash them first".into()));
    }
    let head = Oid::parse(git::read_text(wt, &["rev-parse", "HEAD"])?.trim()).ok_or_else(|| git::Error::Io("HEAD".into()))?;
    let dir = scratch();
    fs::create_dir_all(&dir).map_err(|e| git::Error::Io(format!("{}: {e}", dir.display())))?;
    let result = (|| {
        let file = |n: usize| slashes(&dir.join(format!("message-{n}")));
        for (n, e) in list.iter().enumerate() {
            let text = match (&e.message, e.action) {
                (Some(m), _) => m.clone(),
                (None, Action::Reword) => message(wt, &e.id)?,
                _ => continue,
            };
            fs::write(file(n), text.trim_end().to_owned() + "\n").map_err(|e| git::Error::Io(e.to_string()))?;
        }
        let todo_file = dir.join("todo");
        fs::write(&todo_file, todo(list, file)).map_err(|e| git::Error::Io(e.to_string()))?;
        let saved = backup::save(wt, &head, "rebase -i")?;
        let out = git::command()
            .arg("-C")
            .arg(wt)
            .args(["-c", "rebase.autoSquash=false", "rebase", "-i", "--no-autostash", &base.to_string()])
            .env("GIT_SEQUENCE_EDITOR", format!("cp -f {}", sh_quote(&slashes(&todo_file))))
            .env("GIT_EDITOR", "true")
            .output()
            .map_err(git::started)?;
        if out.status.success() {
            return Ok(Outcome::Done { backup: saved });
        }
        if in_progress(wt)? {
            return Ok(Outcome::Stopped { backup: saved });
        }
        Err(git::refused(&format!("{}{}", String::from_utf8_lossy(&out.stderr), String::from_utf8_lossy(&out.stdout))))
    })();
    let _ = fs::remove_dir_all(&dir);
    result
}

/// Whether a rebase is waiting in the worktree.
fn in_progress(wt: &Path) -> Result<bool, git::Error> {
    for name in ["rebase-merge", "rebase-apply"] {
        let p = git::read_text(wt, &["rev-parse", "--git-path", name])?;
        let p = PathBuf::from(p.trim());
        let p = if p.is_absolute() { p } else { wt.join(p) };
        if p.exists() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn scratch() -> PathBuf {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!("michi-rebase-{}-{n}", std::process::id()))
}

/// `s` as one word for a POSIX shell: in single quotes, a `'` in it closed,
/// escaped and opened again.
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// The way git's shell wants a path, on Windows too.
fn slashes(p: &Path) -> String {
    p.display().to_string().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn paths_with_a_quote_are_quoted_for_the_shell() {
        assert_eq!(sh_quote("/tmp/a b"), "'/tmp/a b'");
        assert_eq!(sh_quote("/tmp/o'neil"), "'/tmp/o'\\''neil'");
        let list = [Entry {
            id: Oid::parse(&"a".repeat(40)).unwrap(),
            subject: "s".into(),
            full: "s".into(),
            action: Action::Reword,
            message: None,
        }];
        assert!(todo(&list, |_| "/tmp/o'neil/message-0".into()).contains("-F '/tmp/o'\\''neil/message-0'\n"));
    }

    fn setup(name: &str) -> (TempRepo, Oid) {
        let t = TempRepo::configured(name);
        let r = t.repo.clone();
        let base = Oid::parse(&t.out(&r, &["rev-parse", "HEAD"])).unwrap();
        t.git(&r, &["switch", "-q", "-c", "feat"]);
        for (f, m) in [("a", "add a"), ("b", "add b"), ("c", "add c")] {
            t.write(&r, f, "x\n");
            t.git(&r, &["add", "."]);
            t.git(&r, &["commit", "-q", "-m", m]);
        }
        (t, base)
    }

    fn subjects(t: &TempRepo) -> Vec<String> {
        t.out(&t.repo, &["log", "--reverse", "--format=%s", "main..HEAD"]).lines().map(str::to_owned).collect()
    }

    #[test]
    fn the_branch_is_read_oldest_first_from_where_it_left_the_trunk() {
        let (t, base) = setup("rebase-read");
        assert_eq!(base_of(&t), Some(base));
        let list = entries(&t.repo, &base).unwrap();
        assert_eq!(list.iter().map(|e| e.subject.as_str()).collect::<Vec<_>>(), ["add a", "add b", "add c"]);
        assert_eq!(check(&list, &list), Err(Problem::Nothing));
    }

    fn base_of(t: &TempRepo) -> Option<Oid> {
        base(&t.repo, "main").unwrap()
    }

    #[test]
    fn a_list_that_cannot_run_says_why() {
        let (t, base) = setup("rebase-check");
        let orig = entries(&t.repo, &base).unwrap();
        let mut l = orig.clone();
        l[0].action = Action::Squash;
        assert_eq!(check(&l, &orig), Err(Problem::FirstFolds));
        l[0].action = Action::Drop;
        l[1].action = Action::Fixup;
        assert_eq!(check(&l, &orig), Err(Problem::FirstFolds));
        for e in &mut l {
            e.action = Action::Drop;
        }
        assert_eq!(check(&l, &orig), Err(Problem::Empty));
    }

    #[test]
    fn reorder_squash_reword_and_drop_make_the_branch_the_list_says() {
        let (t, base) = setup("rebase-run");
        let mut l = entries(&t.repo, &base).unwrap();
        l.swap(0, 1); // b, a, c
        l[1].action = Action::Fixup; // a into b
        l[2].action = Action::Reword;
        l[2].message = Some("Add c, better".into());
        let out = run(&t.repo, &base, &l).unwrap();
        let Outcome::Done { backup } = out else { panic!("{out:?}") };
        assert_eq!(subjects(&t), ["add b", "Add c, better"]);
        assert!(backup.starts_with(crate::backup::PREFIX));
        assert_eq!(t.out(&t.repo, &["log", "-1", "--format=%s", &backup]), "add c", "the old tip is kept");

        let mut l = entries(&t.repo, &base).unwrap();
        l[0].action = Action::Drop;
        assert!(matches!(run(&t.repo, &base, &l), Ok(Outcome::Done { .. })));
        assert_eq!(subjects(&t), ["Add c, better"]);
    }

    #[test]
    fn a_squash_with_a_message_of_its_own_reads_that_message() {
        let (t, base) = setup("rebase-squash");
        let mut l = entries(&t.repo, &base).unwrap();
        l[1].action = Action::Squash;
        l[1].message = Some("a and b".into());
        run(&t.repo, &base, &l).unwrap();
        assert_eq!(subjects(&t), ["a and b", "add c"]);
    }

    #[test]
    fn a_clash_stops_the_rebase_for_the_conflict_screen() {
        let (t, base) = setup("rebase-clash");
        // b and c both write "x" to a new file `d` differently.
        let r = t.repo.clone();
        t.write(&r, "d", "one\n");
        t.git(&r, &["add", "."]);
        t.git(&r, &["commit", "-q", "-m", "d one"]);
        t.write(&r, "d", "two\n");
        t.git(&r, &["commit", "-q", "-am", "d two"]);
        let mut l = entries(&t.repo, &base).unwrap();
        let n = l.len();
        l.swap(n - 1, n - 2);
        let out = run(&r, &base, &l).unwrap();
        assert!(matches!(out, Outcome::Stopped { .. }), "{out:?}");
        t.git(&r, &["rebase", "--abort"]);
    }

    #[test]
    fn a_merge_in_the_list_is_refused() {
        let (t, base) = setup("rebase-merge");
        let r = t.repo.clone();
        t.git(&r, &["switch", "-q", "-c", "side", "main"]);
        t.write(&r, "s", "s\n");
        t.git(&r, &["add", "."]);
        t.git(&r, &["commit", "-q", "-m", "side"]);
        t.git(&r, &["switch", "-q", "feat"]);
        t.git(&r, &["merge", "-q", "--no-ff", "-m", "merge side", "side"]);
        assert!(entries(&r, &base).is_err());
    }
}
