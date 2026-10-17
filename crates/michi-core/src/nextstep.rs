//! Empty-state guidance (issue #81): when the history is empty, there is only
//! one worktree or tsumugi is missing, the screen says the one thing to do
//! next. There is no tour; this decides which single line is due.

/// What is known about the repository and its surroundings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// `Some(false)`: the history was read and has no commit. `None`: not
    /// known yet (still reading), so nothing is said about it.
    pub has_history: Option<bool>,
    /// How many worktrees are listed (0 while they are not read).
    pub worktrees: usize,
    /// tsumugi answered `ls --json` (so the Claude column shows).
    pub tsumugi: bool,
    /// tsumugi was asked at least once, so its absence is a fact.
    pub tsumugi_asked: bool,
}

/// The one next step, most urgent first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// No commit yet: stage the files and make the first one (`2`).
    FirstCommit,
    /// Only one worktree: start new work in its own (`N`).
    NewWork,
    /// tsumugi is not running: start it to see the Claude of each worktree.
    StartTsumugi,
}

impl Step {
    /// The i18n key of the line for this step.
    pub fn key(self) -> &'static str {
        match self {
            Step::FirstCommit => "next.first_commit",
            Step::NewWork => "next.new_work",
            Step::StartTsumugi => "next.tsumugi",
        }
    }
}

/// The step due for `f`, if any. Only one is ever returned.
pub fn next_step(f: &Facts) -> Option<Step> {
    if f.has_history == Some(false) {
        return Some(Step::FirstCommit);
    }
    if f.worktrees == 1 {
        return Some(Step::NewWork);
    }
    if f.worktrees > 1 && f.tsumugi_asked && !f.tsumugi {
        return Some(Step::StartTsumugi);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(has_history: Option<bool>, worktrees: usize, tsumugi: bool, asked: bool) -> Facts {
        Facts { has_history, worktrees, tsumugi, tsumugi_asked: asked }
    }

    #[test]
    fn an_empty_history_comes_first() {
        assert_eq!(next_step(&facts(Some(false), 1, false, true)), Some(Step::FirstCommit));
        assert_eq!(next_step(&facts(Some(false), 0, false, false)), Some(Step::FirstCommit));
    }

    #[test]
    fn nothing_is_said_while_reading() {
        assert_eq!(next_step(&facts(None, 0, false, false)), None);
        assert_eq!(next_step(&facts(None, 3, true, true)), None);
    }

    #[test]
    fn one_worktree_asks_for_new_work() {
        assert_eq!(next_step(&facts(Some(true), 1, false, true)), Some(Step::NewWork));
        assert_eq!(next_step(&facts(None, 1, true, true)), Some(Step::NewWork));
    }

    #[test]
    fn a_missing_tsumugi_is_told_once_there_are_worktrees_and_it_was_asked() {
        assert_eq!(next_step(&facts(Some(true), 2, false, true)), Some(Step::StartTsumugi));
        assert_eq!(next_step(&facts(Some(true), 2, false, false)), None, "not asked yet");
        assert_eq!(next_step(&facts(Some(true), 2, true, true)), None);
    }

    #[test]
    fn a_full_setup_says_nothing() {
        assert_eq!(next_step(&facts(Some(true), 4, true, true)), None);
        assert_eq!(next_step(&Facts::default()), None);
    }

    #[test]
    fn every_step_has_a_distinct_key() {
        let keys = [Step::FirstCommit.key(), Step::NewWork.key(), Step::StartTsumugi.key()];
        assert!(keys.iter().all(|k| k.starts_with("next.")));
        assert!(keys[0] != keys[1] && keys[1] != keys[2] && keys[0] != keys[2]);
    }
}
