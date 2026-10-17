//! A diff for each turn of a Claude (`y` over the worktree table): when a
//! session of tsumugi stops working, the worktree is kept as a snapshot under
//! `refs/michi/turn/<branch>/N` (off the UI thread); `y` opens the newest
//! turn's diff in the compare review, `y` again goes to the turn before it and
//! `Y` to the one after.

use std::collections::HashMap;

use michi_core::claude::{Session, State};
use michi_core::paths;
use michi_core::turn::{self, Turn};

use crate::act::Task;
use crate::app::{App, Mode, View};
use crate::review::Review;

/// The sessions whose turn ended between `before` and `now`: an agent that
/// was running and is not any more. A session not seen before is no news.
pub fn ended_sessions<'a>(before: &HashMap<u64, State>, now: &'a [Session]) -> Vec<&'a Session> {
    now.iter().filter(|s| !s.agent.is_empty() && before.get(&s.id).is_some_and(|was| turn::ended(was, &s.state))).collect()
}

impl App {
    /// A new list of sessions came: keeps a snapshot of the worktree of each
    /// Claude whose turn ended. The first list only sets the baseline.
    pub(crate) fn turns_changed(&mut self) {
        let groups: Vec<(std::path::PathBuf, Vec<Session>)> = self.claude.clone().unwrap_or_default();
        let states: HashMap<u64, State> = groups.iter().flat_map(|(_, v)| v).map(|s| (s.id, s.state.clone())).collect();
        let Some(before) = self.turn_states.replace(states) else { return };
        let mut tasks = Vec::new();
        for (wt, sessions) in &groups {
            if ended_sessions(&before, sessions).is_empty() {
                continue;
            }
            let key = paths::key(wt);
            let Some(t) = self.trees.trees.iter().find(|t| paths::key(&t.wt.path) == key) else { continue };
            let Some(branch) = t.wt.branch.clone() else { continue };
            tasks.push(Task::Turn { wt: t.wt.path.clone(), branch });
        }
        for task in tasks {
            self.act(task);
        }
    }

    /// `y` over the worktree table: reads the turns kept for the selected
    /// worktree's branch, off the UI thread.
    pub(crate) fn open_turns(&mut self) {
        let Some(t) = self.wt_selected() else { return };
        let (wt, branch) = (t.wt.path.clone(), t.wt.branch.clone());
        let Some(branch) = branch else {
            let text = self.tr("review.detached");
            return self.say(text);
        };
        self.act(Task::FindTurns { wt, branch, back: 0 });
    }

    /// `y` (`older`) or `Y` in the turns: the turn before or after the one
    /// shown. Without turns shown, `y` opens them.
    pub(crate) fn turn_step(&mut self, older: bool) {
        let Some(r) = &self.review else { return };
        let Some((turns, i, branch)) = &r.turn else {
            if older {
                self.open_turns();
            }
            return;
        };
        let (len, i, branch, wt) = (turns.len(), *i, branch.clone(), r.path.clone());
        let to = if older { i.checked_sub(1) } else { Some(i + 1).filter(|&j| j < len) };
        match to {
            Some(j) => self.show_turn(wt, branch, None, j),
            None => {
                let text = self.tr(if older { "turn.first" } else { "turn.last" });
                self.say(text);
            }
        }
    }

    /// The turns have come: shows the newest, or says there are none.
    pub(crate) fn turns_found(&mut self, wt: &std::path::Path, branch: &str, back: usize, turns: Vec<Turn>) {
        if turns.is_empty() {
            let text = self.tr("turn.none").replace("{b}", branch);
            return self.say(text);
        }
        let i = turns.len().saturating_sub(1 + back);
        self.show_turn(wt.to_path_buf(), branch.to_owned(), Some(turns), i);
    }

    /// Opens (or moves) the compare review on turn `i`. With `turns` `None`
    /// the review's own list stays.
    fn show_turn(&mut self, wt: std::path::PathBuf, branch: String, turns: Option<Vec<Turn>>, i: usize) {
        let Some(turns) = turns.or_else(|| self.review.as_mut().and_then(|r| r.turn.take()).map(|(t, _, _)| t)) else { return };
        let Some((a, b)) = turn::pair(&turns, i) else { return };
        let title = self
            .tr("turn.title")
            .replace("{n}", &turns[i].n.to_string())
            .replace("{total}", &turns.last().map_or(0, |t| t.n).to_string())
            .replace("{b}", &branch);
        self.close_review();
        let mut r = Review::compare(wt, title, a, b);
        r.turn = Some((turns, i, branch));
        self.review = Some(r);
        self.mode = Mode::Main;
        self.view = View::Worktree;
        self.review_sync();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(id: u64, state: State, agent: &str) -> Session {
        Session {
            id,
            state,
            command: String::new(),
            agent: agent.to_owned(),
            cwd: "C:/x".into(),
            branch: String::new(),
            title: String::new(),
            note: String::new(),
            since_ms: 0,
            read_only: false,
        }
    }

    #[test]
    fn a_running_agent_that_stops_ended_a_turn() {
        let before = HashMap::from([(1, State::Running), (2, State::Running), (3, State::Waiting), (4, State::Running)]);
        let now = [
            session(1, State::Waiting, "claude"),
            session(2, State::Running, "claude"),
            session(3, State::Waiting, "claude"),
            session(4, State::Done, ""),
            session(5, State::Waiting, "claude"),
        ];
        let ids: Vec<u64> = ended_sessions(&before, &now).iter().map(|s| s.id).collect();
        assert_eq!(ids, vec![1]);
    }
}
