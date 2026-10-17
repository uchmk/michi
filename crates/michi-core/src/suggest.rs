//! The next thing to do in a worktree (#119): one suggestion from how the
//! worktree stands (a merge in the way, changes not committed, commits not
//! pushed or not pulled). A pure function of the facts, so the window only
//! reads them and shows the answer; it never decides for the person.

use crate::branch::Doing;

/// How the worktree stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// What it is in the middle of (a merge, a rebase).
    pub doing: Option<Doing>,
    /// Places still clashing.
    pub conflicts: bool,
    /// Files with changes not committed (staged, unstaged or untracked).
    pub changes: usize,
    /// On a branch (not a detached `HEAD`).
    pub on_branch: bool,
    /// The branch is the trunk (`main`).
    pub is_trunk: bool,
    /// Its upstream, when it has a live one: commits not pushed, not pulled.
    pub upstream: Option<(u32, u32)>,
    /// Commits the branch has that the trunk has not.
    pub ahead_of_trunk: u32,
}

/// The one thing to do next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Suggestion {
    /// Places clash: solve them.
    Resolve,
    /// A merge or the like is in the way and nothing clashes: finish it.
    Finish,
    /// Changes not committed.
    Commit,
    /// The upstream has commits the branch has not (or both have some).
    Pull,
    /// The branch has commits the upstream has not.
    Push,
    /// The branch has commits and no upstream yet: publish it.
    Publish,
}

impl Suggestion {
    /// The i18n key of the button's words.
    pub fn key(self) -> &'static str {
        match self {
            Suggestion::Resolve => "suggest.resolve",
            Suggestion::Finish => "suggest.finish",
            Suggestion::Commit => "suggest.commit",
            Suggestion::Pull => "suggest.pull",
            Suggestion::Push => "suggest.push",
            Suggestion::Publish => "suggest.publish",
        }
    }
}

/// The suggestion for `f`, most pressing first; none when all is in order.
pub fn suggest(f: &Facts) -> Option<Suggestion> {
    match f.doing {
        // A bisect has its own strip and keys.
        Some(Doing::Bisect) => return None,
        Some(_) => return Some(if f.conflicts { Suggestion::Resolve } else { Suggestion::Finish }),
        None => {}
    }
    if f.changes > 0 {
        return Some(Suggestion::Commit);
    }
    if !f.on_branch {
        return None;
    }
    match f.upstream {
        Some((_, behind)) if behind > 0 => Some(Suggestion::Pull),
        Some((ahead, _)) if ahead > 0 => Some(Suggestion::Push),
        Some(_) => None,
        None if !f.is_trunk && f.ahead_of_trunk > 0 => Some(Suggestion::Publish),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn branch() -> Facts {
        Facts { on_branch: true, ..Facts::default() }
    }

    #[test]
    fn a_clean_synced_branch_has_nothing_to_suggest() {
        assert_eq!(suggest(&branch()), None);
        assert_eq!(suggest(&Facts { upstream: Some((0, 0)), ..branch() }), None);
        assert_eq!(suggest(&Facts::default()), None);
    }

    #[test]
    fn changes_ask_for_a_commit() {
        assert_eq!(suggest(&Facts { changes: 3, ..branch() }), Some(Suggestion::Commit));
        // Even on no branch: the commit is still the first thing.
        assert_eq!(suggest(&Facts { changes: 1, ..Facts::default() }), Some(Suggestion::Commit));
        // Changes come before a push.
        assert_eq!(suggest(&Facts { changes: 1, upstream: Some((2, 0)), ..branch() }), Some(Suggestion::Commit));
    }

    #[test]
    fn a_merge_in_the_way_comes_first() {
        let f = Facts { doing: Some(Doing::Merge), conflicts: true, changes: 4, ..branch() };
        assert_eq!(suggest(&f), Some(Suggestion::Resolve));
        assert_eq!(suggest(&Facts { conflicts: false, ..f.clone() }), Some(Suggestion::Finish));
        assert_eq!(suggest(&Facts { doing: Some(Doing::Rebase), ..f }), Some(Suggestion::Resolve));
    }

    #[test]
    fn a_bisect_is_left_to_its_own_strip() {
        assert_eq!(suggest(&Facts { doing: Some(Doing::Bisect), changes: 2, ..branch() }), None);
    }

    #[test]
    fn unpushed_commits_ask_for_a_push_and_unpulled_for_a_pull() {
        assert_eq!(suggest(&Facts { upstream: Some((2, 0)), ..branch() }), Some(Suggestion::Push));
        assert_eq!(suggest(&Facts { upstream: Some((0, 1)), ..branch() }), Some(Suggestion::Pull));
        // Apart both ways: bring theirs in first.
        assert_eq!(suggest(&Facts { upstream: Some((2, 1)), ..branch() }), Some(Suggestion::Pull));
    }

    #[test]
    fn a_branch_without_an_upstream_is_published_but_the_trunk_is_not() {
        assert_eq!(suggest(&Facts { ahead_of_trunk: 2, ..branch() }), Some(Suggestion::Publish));
        assert_eq!(suggest(&Facts { ahead_of_trunk: 2, is_trunk: true, ..branch() }), None);
        assert_eq!(suggest(&Facts { ahead_of_trunk: 0, ..branch() }), None);
        assert_eq!(suggest(&Facts { ahead_of_trunk: 2, on_branch: false, ..Facts::default() }), None);
    }

    #[test]
    fn every_suggestion_has_its_own_key() {
        let all = [Suggestion::Resolve, Suggestion::Finish, Suggestion::Commit, Suggestion::Pull, Suggestion::Push, Suggestion::Publish];
        let mut keys: Vec<_> = all.iter().map(|s| s.key()).collect();
        assert!(keys.iter().all(|k| k.starts_with("suggest.")));
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), all.len());
    }
}
