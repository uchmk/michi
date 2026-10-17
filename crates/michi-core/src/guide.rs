//! Short explanations for beginners (issues #120, #121 and #122): what the
//! stage is, what a conflict is and what a worktree is. Each is shown until it
//! is dismissed, and the dismissal is kept per user. This decides which
//! topics are due and what is kept; the words are the `michi` crate's tables
//! and the drawing is its views'.

use std::collections::BTreeSet;

/// A thing that is explained.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Topic {
    /// Changes view: changed, put in the next commit, committed.
    Stage,
    /// Conflict view: what a conflict is, ours and theirs, how to go on.
    Conflict,
    /// Worktree view: what a worktree is and how it relates to Claude.
    Worktree,
}

impl Topic {
    pub const ALL: [Topic; 3] = [Topic::Stage, Topic::Conflict, Topic::Worktree];

    /// The name kept in the file.
    pub fn name(self) -> &'static str {
        match self {
            Topic::Stage => "stage",
            Topic::Conflict => "conflict",
            Topic::Worktree => "worktree",
        }
    }

    fn from_name(s: &str) -> Option<Topic> {
        Topic::ALL.into_iter().find(|t| t.name() == s)
    }
}

/// The topics that were dismissed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Guides {
    dismissed: BTreeSet<Topic>,
}

impl Guides {
    /// Read from the kept text (one topic name per line); unknown lines are
    /// ignored so a newer file does not break an older michi.
    pub fn parse(text: &str) -> Self {
        Self { dismissed: text.lines().filter_map(|l| Topic::from_name(l.trim())).collect() }
    }

    /// The text to keep.
    pub fn text(&self) -> String {
        self.dismissed.iter().map(|t| format!("{}\n", t.name())).collect()
    }

    /// Whether `t` is to be shown in full (it was not dismissed).
    pub fn shown(&self, t: Topic) -> bool {
        !self.dismissed.contains(&t)
    }

    /// Dismiss `t`; whether that changed anything.
    pub fn dismiss(&mut self, t: Topic) -> bool {
        self.dismissed.insert(t)
    }

    /// Show `t` again; whether that changed anything.
    pub fn restore(&mut self, t: Topic) -> bool {
        self.dismissed.remove(&t)
    }

    /// Show all of them again; whether that changed anything.
    pub fn restore_all(&mut self) -> bool {
        let any = !self.dismissed.is_empty();
        self.dismissed.clear();
        any
    }
}

/// Where a change stands on its way into a commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Changed in the worktree, not yet chosen for the next commit.
    Changed,
    /// Chosen for the next commit (staged).
    Staged,
    /// Chosen, and the message is written: the commit is next.
    Ready,
}

impl Stage {
    /// The three steps in order, for drawing the diagram.
    pub const ALL: [Stage; 3] = [Stage::Changed, Stage::Staged, Stage::Ready];
}

/// The step to light up: with nothing staged it is the first; with files
/// staged it is the second until a message is written, then the third.
pub fn stage(staged: usize, message_empty: bool) -> Stage {
    if staged == 0 {
        Stage::Changed
    } else if message_empty {
        Stage::Staged
    } else {
        Stage::Ready
    }
}

/// How many cards the conflict explanation has.
pub const CONFLICT_CARDS: usize = 3;

/// The card after (`delta` 1) or before (-1) `at`, kept inside the cards.
pub fn card(at: usize, delta: i32) -> usize {
    (at as i64 + i64::from(delta)).clamp(0, CONFLICT_CARDS as i64 - 1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_is_shown_until_it_is_dismissed() {
        let mut g = Guides::default();
        assert!(Topic::ALL.iter().all(|t| g.shown(*t)));
        assert!(g.dismiss(Topic::Conflict));
        assert!(!g.dismiss(Topic::Conflict));
        assert!(!g.shown(Topic::Conflict));
        assert!(g.shown(Topic::Stage) && g.shown(Topic::Worktree));
    }

    #[test]
    fn the_kept_text_comes_back_and_unknown_lines_are_let_go() {
        let mut g = Guides::default();
        g.dismiss(Topic::Worktree);
        g.dismiss(Topic::Stage);
        assert_eq!(Guides::parse(&g.text()), g);
        let read = Guides::parse("stage\nfuture-topic\n\n  conflict  \r\n");
        assert!(!read.shown(Topic::Stage) && !read.shown(Topic::Conflict) && read.shown(Topic::Worktree));
        assert_eq!(Guides::parse(""), Guides::default());
    }

    #[test]
    fn restoring_shows_them_again() {
        let mut g = Guides::parse("stage\nworktree\n");
        assert!(g.restore(Topic::Stage));
        assert!(!g.restore(Topic::Stage));
        assert!(g.restore_all());
        assert!(!g.restore_all());
        assert!(Topic::ALL.iter().all(|t| g.shown(*t)));
    }

    #[test]
    fn the_step_follows_what_is_staged_and_written() {
        assert_eq!(stage(0, true), Stage::Changed);
        assert_eq!(stage(0, false), Stage::Changed);
        assert_eq!(stage(2, true), Stage::Staged);
        assert_eq!(stage(1, false), Stage::Ready);
    }

    #[test]
    fn the_cards_stop_at_both_ends() {
        assert_eq!(card(0, -1), 0);
        assert_eq!(card(0, 1), 1);
        assert_eq!(card(CONFLICT_CARDS - 1, 1), CONFLICT_CARDS - 1);
    }
}
