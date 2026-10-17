//! A snapshot of a worktree for each turn of the Claude working in it, so
//! what one instruction changed can be seen as a diff. A turn ends when
//! tsumugi reports the session going from `running` to `waiting` or `done`
//! ([`ended`]); the working folder is then held in a commit (like the WIP
//! commit: tracked changes, deletions, untracked files within limits), whose
//! parent is the worktree's `HEAD`, under `refs/michi/turn/<branch>/<N>`.
//! Comparing turn N-1 with N is the turn's diff. gc keeps the commits the
//! refs hold. Only the newest [`KEEP`] turns of a branch are kept. The calls
//! block; run them off the UI thread.

use std::path::Path;

use crate::claude::State;
use crate::git;
use crate::oid::Oid;
use crate::predict::{self, Limits};

/// Where the turns go.
pub const PREFIX: &str = "refs/michi/turn/";

/// How many turns of one branch are kept.
pub const KEEP: usize = 100;

/// One turn's snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Turn {
    /// Its number on the branch, from 1.
    pub n: u32,
    /// The snapshot commit.
    pub commit: Oid,
    /// The worktree's `HEAD` when it was taken.
    pub head: Oid,
    /// When it was taken (seconds since 1970).
    pub time: i64,
}

/// The ref of turn `n` of `branch`.
pub fn ref_name(branch: &str, n: u32) -> String {
    format!("{PREFIX}{branch}/{n}")
}

/// Whether a session going from `was` to `now` has ended a turn: an agent
/// that was working stops, to wait for a person or because it is done.
pub fn ended(was: &State, now: &State) -> bool {
    *was == State::Running && matches!(now, State::Waiting | State::Done | State::Failed)
}

/// The turns of `branch`, the oldest first.
pub fn list(dir: &Path, branch: &str) -> Result<Vec<Turn>, git::Error> {
    let prefix = format!("{PREFIX}{branch}/");
    let out = git::read_text(dir, &["for-each-ref", "--format=%(refname)%09%(objectname)%09%(parent)%09%(committerdate:unix)", &prefix])?;
    let mut turns: Vec<Turn> = out.lines().filter_map(|l| parse_line(&prefix, l)).collect();
    turns.sort_by_key(|t| t.n);
    Ok(turns)
}

fn parse_line(prefix: &str, line: &str) -> Option<Turn> {
    let mut f = line.split('\t');
    let n = f.next()?.strip_prefix(prefix)?.parse::<u32>().ok()?;
    let commit = Oid::parse(f.next()?)?;
    let head = Oid::parse(f.next()?.split(' ').next()?)?;
    let time = f.next()?.trim().parse().ok()?;
    Some(Turn { n, commit, head, time })
}

/// What turn `i` of `turns` changed, as the two commits to compare: the turn
/// before it (the first turn: the `HEAD` it was taken on).
pub fn pair(turns: &[Turn], i: usize) -> Option<(Oid, Oid)> {
    let t = turns.get(i)?;
    let before = if i == 0 { t.head } else { turns[i - 1].commit };
    Some((before, t.commit))
}

/// Takes the snapshot of the worktree at `wt`, on `branch`, as a new turn.
/// `None` when the folder is the same as at the last turn (the Claude changed
/// nothing). Nothing but the new ref (and the oldest ones past [`KEEP`]) is
/// written: the index, `HEAD` and the files stay as they were.
pub fn snapshot(wt: &Path, branch: &str) -> Result<Option<Turn>, git::Error> {
    let snap = predict::snapshot(wt, Limits::DEFAULT)?;
    let tree_of = |rev: &str| git::read_text(wt, &["rev-parse", &format!("{rev}^{{tree}}")]);
    let (head, tree) = match &snap {
        Some(s) => (s.head, tree_of(&s.commit.unwrap_or(s.head).to_string())?),
        None => {
            let head = git::read_text(wt, &["rev-parse", "HEAD^{commit}"])?;
            let head = Oid::parse(&head).ok_or_else(|| git::Error::Io(format!("git rev-parse: unexpected output: {head}")))?;
            (head, tree_of(&head.to_string())?)
        }
    };
    let turns = list(wt, branch)?;
    if let Some(last) = turns.last() {
        if tree_of(&last.commit.to_string())? == tree {
            return Ok(None);
        }
    }
    let n = turns.last().map_or(1, |t| t.n + 1);
    let mut c = git::command();
    c.arg("-C").arg(wt).args(["commit-tree", &tree, "-p", &head.to_string(), "-m", &format!("turn {n} of {branch}"), "--no-gpg-sign"]);
    c.env("GIT_AUTHOR_NAME", "michi").env("GIT_AUTHOR_EMAIL", "michi@example.invalid");
    c.env("GIT_COMMITTER_NAME", "michi").env("GIT_COMMITTER_EMAIL", "michi@example.invalid");
    let out = c.output().map_err(git::started)?;
    if !out.status.success() {
        return Err(git::refused(&String::from_utf8_lossy(&out.stderr)));
    }
    let hex = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    let commit = Oid::parse(&hex).ok_or_else(|| git::Error::Io(format!("git commit-tree: unexpected output: {hex}")))?;
    // The empty old value: only while the ref is not there.
    git::read(wt, &["update-ref", "--create-reflog", "-m", "michi: turn", &ref_name(branch, n), &commit.to_string(), ""])?;
    let all = list(wt, branch)?;
    for old in all.iter().take(all.len().saturating_sub(KEEP)) {
        git::read(wt, &["update-ref", "-d", &ref_name(branch, old.n)])?;
    }
    let made = all.into_iter().find(|t| t.n == n);
    made.map(Some).ok_or_else(|| git::Error::Io(format!("{}: not found after update-ref", ref_name(branch, n))))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn a_turn_ends_when_a_working_agent_stops() {
        assert!(ended(&State::Running, &State::Waiting));
        assert!(ended(&State::Running, &State::Done));
        assert!(ended(&State::Running, &State::Failed));
        assert!(!ended(&State::Waiting, &State::Running));
        assert!(!ended(&State::Waiting, &State::Done));
        assert!(!ended(&State::Running, &State::Running));
        assert!(!ended(&State::Running, &State::Other("x".into())));
    }

    #[test]
    fn turns_are_snapshots_of_the_folder_and_their_diff_is_what_one_turn_changed() {
        let r = TempRepo::configured("turn");
        r.write(&r.repo, "a.txt", "one\n");
        r.git(&r.repo, &["add", "."]);
        r.git(&r.repo, &["commit", "-q", "-m", "base"]);
        let head = r.out(&r.repo, &["rev-parse", "HEAD"]);
        assert_eq!(list(&r.repo, "main").unwrap(), []);
        // The first turn is a baseline; a second one with the same folder is not a turn.
        let t1 = snapshot(&r.repo, "main").unwrap().unwrap();
        assert_eq!((t1.n, t1.head.to_string()), (1, head.clone()));
        assert_eq!(snapshot(&r.repo, "main").unwrap(), None);
        // A change and an untracked file: turn 2.
        r.write(&r.repo, "a.txt", "one\ntwo\n");
        r.write(&r.repo, "new.txt", "n\n");
        let t2 = snapshot(&r.repo, "main").unwrap().unwrap();
        assert_eq!(t2.n, 2);
        // The index and the branch were not touched.
        assert_eq!(r.out(&r.repo, &["diff", "--cached", "--name-only"]), "");
        assert_eq!(r.out(&r.repo, &["rev-parse", "HEAD"]), head);
        // A commit made by the Claude is part of the next turn's diff.
        r.git(&r.repo, &["add", "."]);
        r.git(&r.repo, &["commit", "-q", "-m", "claude's"]);
        r.write(&r.repo, "a.txt", "one\ntwo\nthree\n");
        let t3 = snapshot(&r.repo, "main").unwrap().unwrap();
        let turns = list(&r.repo, "main").unwrap();
        assert_eq!(turns.iter().map(|t| t.n).collect::<Vec<_>>(), [1, 2, 3]);
        assert_eq!(turns[2], t3);
        assert_eq!(r.out(&r.repo, &["rev-parse", &ref_name("main", 2)]), t2.commit.to_string());
        let (a, b) = pair(&turns, 1).unwrap();
        assert_eq!((a, b), (t1.commit, t2.commit));
        let names = r.out(&r.repo, &["diff", "--name-only", &a.to_string(), &b.to_string()]);
        assert_eq!(names.lines().collect::<Vec<_>>(), ["a.txt", "new.txt"]);
        let (a, b) = pair(&turns, 2).unwrap();
        assert_eq!(r.out(&r.repo, &["diff", "--name-only", &a.to_string(), &b.to_string()]), "a.txt");
        // The first turn is compared with the HEAD it was taken on.
        assert_eq!(pair(&turns, 0).unwrap().0.to_string(), head);
        assert_eq!(pair(&turns, 3), None);
    }

    #[test]
    fn branches_keep_their_turns_apart_and_a_slash_in_a_name_is_fine() {
        let r = TempRepo::configured("turn-branches");
        r.write(&r.repo, "a.txt", "1\n");
        snapshot(&r.repo, "feat/x").unwrap().unwrap();
        r.write(&r.repo, "a.txt", "2\n");
        snapshot(&r.repo, "feat/x").unwrap().unwrap();
        r.write(&r.repo, "a.txt", "3\n");
        snapshot(&r.repo, "feat").unwrap().unwrap();
        assert_eq!(list(&r.repo, "feat/x").unwrap().len(), 2);
        // `feat`'s prefix also matches `feat/x/…`, which is not a number.
        assert_eq!(list(&r.repo, "feat").unwrap().len(), 1);
        assert_eq!(list(&r.repo, "other").unwrap(), []);
    }

    #[test]
    fn only_the_newest_turns_are_kept() {
        let r = TempRepo::configured("turn-keep");
        for i in 0..KEEP + 3 {
            r.write(&r.repo, "a.txt", &format!("{i}\n"));
            snapshot(&r.repo, "main").unwrap().unwrap();
        }
        let turns = list(&r.repo, "main").unwrap();
        assert_eq!(turns.len(), KEEP);
        assert_eq!(turns[0].n, 4);
        assert_eq!(turns.last().unwrap().n as usize, KEEP + 3);
    }

    #[test]
    fn a_line_of_for_each_ref_is_read() {
        let h = "a".repeat(40);
        let line = format!("refs/michi/turn/b/7\t{h}\t{h}\t1767225600");
        let t = parse_line("refs/michi/turn/b/", &line).unwrap();
        assert_eq!((t.n, t.time), (7, 1_767_225_600));
        assert_eq!(parse_line("refs/michi/turn/b/", &format!("refs/michi/turn/b/x/7\t{h}\t{h}\t1")), None);
    }
}
