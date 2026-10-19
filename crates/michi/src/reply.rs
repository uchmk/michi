//! Answering a Claude that waits (`W`): the toast says which worktree waits
//! and what it asks; `W` goes to that worktree's branch review (its diff) and
//! puts the question and a line for the answer over it. The answer is typed
//! into the Claude's tsumugi tab (`tsumugi send`) off the UI thread, like the
//! other hand-offs.

use std::path::PathBuf;

use michi_core::claude::{self, Waiting};
use michi_core::paths;

use crate::act::{self, Task};
use crate::app::{App, Mode, View};

/// The answer being written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    pub wt: PathBuf,
    /// The tsumugi session that waits.
    pub session: u64,
    /// What it asks, on one line; empty if it did not say.
    pub question: String,
    /// The answer so far.
    pub text: String,
    /// The line has not been given the keyboard yet.
    pub opening: bool,
}

impl App {
    /// The waits that tsumugi reports now, the longest first.
    pub fn waits(&self) -> Vec<Waiting> {
        claude::waiting(self.claude.as_deref().unwrap_or(&[]))
    }

    /// A new list of sessions came: tells of the Claudes that began to wait
    /// (the toast, and the system's notification when the window is behind),
    /// and drops an answer for one that waits no more. The first list only
    /// sets the baseline: what waited before michi looked is not news.
    pub(crate) fn waits_changed(&mut self) {
        let now = self.waits();
        if let Some(r) = &self.reply {
            if !now.iter().any(|w| w.id == r.session) {
                self.reply = None;
            }
        }
        let keys: Vec<_> = now.iter().map(Waiting::key).collect();
        let Some(seen) = self.waits_told.replace(keys) else { return };
        let Some(w) = claude::fresh(&seen, &now).into_iter().last() else { return };
        let name = act::name(&w.wt);
        let (toast, body) = if w.question.is_empty() {
            (self.tr("claude.asks_blank").replace("{wt}", &name), name.clone())
        } else {
            (self.tr("claude.asks").replace("{wt}", &name).replace("{q}", &w.question), format!("{name}: {}", w.question))
        };
        self.notify(ito_notices::Kind::Wait, toast, Some(crate::app::Target::Worktree(w.wt.clone())), 15);
        let title = self.tr("notify.title_waiting").to_owned();
        self.tell("done", &title, &body, Some(self.next_notice));
    }

    /// `W`: to the worktree of the Claude that has waited longest (again: the
    /// next one), with its diff in sight and the question over it.
    pub(crate) fn jump_waiting(&mut self) {
        let waits = self.waits();
        let after = self.reply.as_ref().and_then(|r| waits.iter().position(|w| w.id == r.session));
        let Some(w) = after.map_or(waits.first(), |i| waits.get((i + 1) % waits.len())).cloned() else {
            return self.say(self.tr("reply.none"));
        };
        let key = paths::key(&w.wt);
        let Some(row) = self.trees.trees.iter().position(|t| paths::key(&t.wt.path) == key) else {
            return self.say(self.tr("act.no_tree"));
        };
        self.mode = Mode::Main;
        self.picker = None;
        self.new_work = None;
        self.view = View::Worktree;
        self.wt_sel = row;
        if self.review.as_ref().is_none_or(|r| paths::key(&r.path) != key) {
            self.close_review();
            self.open_review();
        }
        self.reply = Some(Reply { wt: w.wt, session: w.id, question: w.question, text: String::new(), opening: true });
    }

    /// Enter in the answer: types it into the Claude's tab. Nothing written,
    /// or a Claude that waits no more, says so and keeps the line.
    pub(crate) fn reply_send(&mut self) {
        let Some(r) = &self.reply else { return };
        let text = r.text.trim().to_owned();
        if !self.waits().iter().any(|w| w.id == r.session) {
            self.reply = None;
            return self.say(self.tr("reply.gone"));
        }
        if text.is_empty() {
            return self.say(self.tr("reply.empty"));
        }
        let session = r.session;
        self.reply = None;
        self.act(Task::Reply { session, text });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::toast;
    use crate::graph::fixture;
    use crate::repo::Answer;
    use michi_core::claude::{Session, State};

    fn waiting(id: u64, state: State, since_ms: u64, note: &str) -> Session {
        Session {
            id,
            state,
            command: "claude".into(),
            agent: "claude".into(),
            cwd: "C:/r/wt".into(),
            branch: "wip".into(),
            title: "fix".into(),
            note: note.into(),
            since_ms,
            read_only: false,
        }
    }

    fn app() -> App {
        let mut a = App::new(crate::prefs::Prefs::empty(), "C:/dev/x".into());
        a.lang = "en".into();
        a.take(Answer::Trees(fixture::trees()));
        a
    }

    fn sessions(a: &mut App, list: Vec<Session>) {
        a.take(Answer::Claude(Some(vec![(PathBuf::from("C:/r/wt"), list)])));
    }

    #[test]
    fn a_wait_is_told_once_and_not_for_what_waited_at_the_start() {
        let mut a = app();
        sessions(&mut a, vec![waiting(3, State::Waiting, 10, "Old question?")]);
        assert!(a.toast.is_none(), "the first list is the baseline");
        sessions(&mut a, vec![waiting(3, State::Running, 11, "")]);
        sessions(&mut a, vec![waiting(3, State::Waiting, 20, "Run the migration?")]);
        assert_eq!(toast(&a), "The Claude in wt is waiting: Run the migration? (W: answer)");
        a.toast = None;
        sessions(&mut a, vec![waiting(3, State::Waiting, 20, "Run the migration?")]);
        assert!(a.toast.is_none(), "the same wait is not told again");
    }

    #[test]
    fn w_goes_to_the_worktree_with_its_diff_and_the_question() {
        let mut a = app();
        a.view = View::Graph;
        sessions(&mut a, vec![waiting(3, State::Running, 1, "")]);
        a.feed('W');
        assert_eq!(toast(&a), "No Claude is waiting");
        a.toast = None;
        sessions(&mut a, vec![waiting(3, State::Waiting, 20, "Run the migration?")]);
        assert!(a.feed('W'));
        assert_eq!(a.view, View::Worktree);
        assert_eq!(a.wt_selected().map(|t| t.wt.path.clone()), Some(PathBuf::from("C:/r/wt")));
        assert!(
            a.review.as_ref().is_some_and(|r| r.path.as_path() == std::path::Path::new("C:/r/wt")),
            "the diff of the worktree's branch"
        );
        let r = a.reply.as_ref().expect("the answer line");
        assert_eq!((r.session, r.question.as_str()), (3, "Run the migration?"));
        assert!(!a.feed('y'), "typing goes to the line, not to the keys");
        assert!(a.escape() && a.reply.is_none());
    }

    #[test]
    fn w_again_goes_to_the_next_waiting_claude() {
        let mut a = app();
        sessions(&mut a, vec![waiting(3, State::Running, 1, "")]);
        sessions(&mut a, vec![waiting(3, State::Waiting, 10, "a?"), waiting(4, State::Waiting, 20, "b?")]);
        a.jump_waiting();
        assert_eq!(a.reply.as_ref().map(|r| r.session), Some(3));
        a.jump_waiting();
        assert_eq!(a.reply.as_ref().map(|r| r.session), Some(4));
        a.jump_waiting();
        assert_eq!(a.reply.as_ref().map(|r| r.session), Some(3));
    }

    #[test]
    fn sending_needs_an_answer_and_a_claude_that_still_waits() {
        let mut a = app();
        sessions(&mut a, vec![waiting(3, State::Running, 1, "")]);
        sessions(&mut a, vec![waiting(3, State::Waiting, 10, "ok?")]);
        a.jump_waiting();
        a.reply_send();
        assert_eq!(toast(&a), "Write the answer first");
        assert!(a.reply.is_some());
        a.reply.as_mut().unwrap().text = "  yes  ".into();
        sessions(&mut a, vec![waiting(3, State::Running, 30, "")]);
        assert!(a.reply.is_none(), "answered elsewhere: the line goes");
        a.jump_waiting();
        assert_eq!(toast(&a), "No Claude is waiting");
    }
}
