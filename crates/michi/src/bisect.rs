//! Bisect over the graph (#32): `x` marks the selected commit bad and `g`
//! good; with both marked a yes starts `git bisect` in the window's
//! worktree. While it runs the strip under the title bar shows the commit to
//! test, the steps left and, at the end, the first bad commit; `g` `x` `u`
//! mark the commit it checked out good, bad or skipped, `r` copies a
//! `git bisect run` line for a terminal or a Claude, `X` ends it. The work is
//! `michi_core::bisect`; this is the keys, the questions and the words.

use std::path::Path;

use michi_core::bisect::{Mark, State};
use michi_core::oid::Oid;

use crate::act::{self, Task};
use crate::app::{App, Repo};
use crate::picker::same;
use crate::work::{Band, Then};

/// What stands for the test command in the line `r` copies.
const TEST_COMMAND: &str = "<test-command>";

/// The commits marked in the graph for a bisect not started yet.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Marks {
    pub bad: Option<Oid>,
    pub good: Option<Oid>,
}

impl Marks {
    pub fn is_empty(&self) -> bool {
        self.bad.is_none() && self.good.is_none()
    }

    /// Both ends, once both are marked.
    pub fn both(&self) -> Option<(Oid, Oid)> {
        Some((self.bad?, self.good?))
    }
}

impl App {
    /// The worktrees were read: the window's worktree is asked about its
    /// bisect when it is in the middle of one; marks left over from a bisect
    /// that ended are dropped.
    pub(crate) fn bisect_sync(&mut self) {
        let Repo::Ready(s) = &self.repo else { return };
        let wt = s.top.clone();
        if self.doing_in(&wt) == Some(michi_core::branch::Doing::Bisect) {
            self.act(Task::ReadBisect { wt });
        } else {
            self.bisect = None;
        }
    }

    /// The reading is back (none: it runs none).
    pub(crate) fn bisect_read(&mut self, wt: &Path, state: Option<State>) {
        let Repo::Ready(s) = &self.repo else { return };
        if same(&s.top, wt) {
            self.bisect = state;
        }
    }

    /// A start, a mark or a reset is over: what to say.
    pub(crate) fn bisect_done(&mut self, task: &Task, state: Option<State>, saved: Option<String>) -> (String, u64) {
        let saved = saved.as_deref().map_or("", michi_core::backup::short).to_owned();
        let wt = match task {
            Task::BisectStart { wt, .. } | Task::BisectMark { wt, .. } | Task::BisectReset { wt } => act::name(wt),
            _ => String::new(),
        };
        let ended = matches!(task, Task::BisectReset { .. });
        if matches!(task, Task::BisectStart { .. }) {
            self.bisect_marks = Marks::default();
        }
        let progress = state.as_ref().map(|s| self.bisect_progress(s));
        self.bisect = state;
        let started = || self.tr("bisect.started").replace("{wt}", &wt).replace("{saved}", &saved);
        match (task, progress) {
            _ if ended => (self.tr("bisect.ended").replace("{wt}", &wt).replace("{saved}", &saved), 10),
            (Task::BisectStart { .. }, Some(p)) => (format!("{} {p}", started()), 10),
            (Task::BisectStart { .. }, None) => (started(), 10),
            (_, Some(p)) => (p, 10),
            (_, None) => (self.tr("bisect.ended").replace("{wt}", &wt).replace("{saved}", ""), 8),
        }
    }

    /// Where a bisect is, in a sentence.
    pub(crate) fn bisect_progress(&self, s: &State) -> String {
        if let Some(f) = &s.first_bad {
            let key = if s.possible { "bisect.possible" } else { "bisect.found" };
            return self.tr(key).replace("{id}", &f.short()).replace("{subject}", &s.first_subject);
        }
        let Some(h) = s.head.as_ref().filter(|_| s.testing()) else { return self.tr("bisect.waiting").to_owned() };
        let steps = s.steps.map_or_else(|| "?".to_owned(), |n| n.to_string());
        let left = s.left.map_or_else(|| "?".to_owned(), |n| n.to_string());
        self.tr("bisect.next")
            .replace("{id}", &h.short())
            .replace("{subject}", &s.head_subject)
            .replace("{steps}", &steps)
            .replace("{left}", &left)
    }

    /// What the strip says while a bisect runs or commits are marked.
    pub(crate) fn bisect_text(&self) -> Option<String> {
        if let Some(s) = &self.bisect {
            return Some(self.bisect_progress(s));
        }
        let m = &self.bisect_marks;
        if m.is_empty() {
            return None;
        }
        let name = |o: &Option<Oid>| o.as_ref().map_or_else(|| "-".to_owned(), Oid::short);
        Some(self.tr("strip.bisect_marks").replace("{bad}", &name(&m.bad)).replace("{good}", &name(&m.good)))
    }

    /// The strip's buttons for it.
    pub(crate) fn bisect_actions(&self) -> Vec<(char, String)> {
        let word = |c: char, key: &'static str| (c, self.tr(key).to_owned());
        match &self.bisect {
            Some(s) if s.testing() => vec![
                word('y', "strip.bisect_good"),
                word('x', "strip.bisect_bad"),
                word('u', "strip.bisect_skip"),
                word('r', "strip.bisect_run"),
                word('X', "strip.bisect_reset"),
            ],
            Some(s) if s.first_bad.is_none() => {
                vec![word('y', "strip.bisect_good"), word('x', "strip.bisect_bad"), word('X', "strip.bisect_reset")]
            }
            Some(_) => vec![word('X', "strip.bisect_reset")],
            None if self.bisect_marks.both().is_some() => vec![word('r', "strip.bisect_run"), word('X', "strip.bisect_clear")],
            None if !self.bisect_marks.is_empty() => vec![word('X', "strip.bisect_clear")],
            None => Vec::new(),
        }
    }

    /// `x` `y` `u` `X` `r` in the graph. Always used.
    pub(crate) fn bisect_key(&mut self, c: char) -> bool {
        let Repo::Ready(s) = &self.repo else {
            self.say(self.tr("act.no_tree"));
            return true;
        };
        let wt = s.top.clone();
        match c {
            'x' => self.bisect_mark(&wt, Mark::Bad),
            'y' => self.bisect_mark(&wt, Mark::Good),
            'u' => self.bisect_skip(&wt),
            'X' => self.bisect_end(&wt),
            _ => self.bisect_copy(&wt),
        }
        true
    }

    /// The selected commit; says so when there is none.
    fn bisect_picked(&mut self) -> Option<Oid> {
        let id = match self.graph.selected_wip() {
            Some(_) => None,
            None => self.graph.selected().and_then(|id| self.graph.find_commit(&id)).map(|c| c.id),
        };
        if id.is_none() {
            self.say(self.tr("act.pick_commit"));
        }
        id
    }

    fn bisect_mark(&mut self, wt: &Path, how: Mark) {
        if let Some(s) = self.bisect.clone() {
            if s.first_bad.is_some() {
                return self.say(self.tr("bisect.over"));
            }
            // With a commit checked out to test, the mark is for that one;
            // before, for the one selected.
            let id = if s.testing() {
                None
            } else {
                match self.bisect_picked() {
                    Some(id) => Some(id),
                    None => return,
                }
            };
            return self.act(Task::BisectMark { wt: wt.to_path_buf(), how, id });
        }
        if let Some(d) = self.doing_in(wt) {
            let text = self.tr("act.revert_doing").replace("{wt}", &act::name(wt)).replace("{doing}", &self.doing_word(d));
            return self.say_for(text, 8);
        }
        let Some(id) = self.bisect_picked() else { return };
        let m = &mut self.bisect_marks;
        let other = if how == Mark::Bad { m.good } else { m.bad };
        if other == Some(id) {
            return self.say(self.tr("bisect.same"));
        }
        if how == Mark::Bad {
            m.bad = Some(id);
        } else {
            m.good = Some(id);
        }
        let key = if how == Mark::Bad { "bisect.marked_bad" } else { "bisect.marked_good" };
        let text = self.tr(key).replace("{id}", &id.short());
        match self.bisect_marks.both() {
            Some((bad, good)) => self.ask_bisect(wt, bad, good),
            None => self.say_for(text, 5),
        }
    }

    /// Both ends are marked: asks before the worktree's checkouts begin.
    fn ask_bisect(&mut self, wt: &Path, bad: Oid, good: Oid) {
        let mut text =
            self.tr("bisect.ask").replace("{wt}", &act::name(wt)).replace("{bad}", &bad.short()).replace("{good}", &good.short());
        if self.claude_working(wt) {
            text.push_str(self.tr("act.revert_claude"));
        }
        self.band = Some(Band::new(text, Then::Act(Task::BisectStart { wt: wt.to_path_buf(), bad, good })));
    }

    fn bisect_skip(&mut self, wt: &Path) {
        if self.bisect.as_ref().is_some_and(State::testing) {
            self.act(Task::BisectMark { wt: wt.to_path_buf(), how: Mark::Skip, id: None });
        } else {
            self.say(self.tr("bisect.not_testing"));
        }
    }

    /// `X`: ends the bisect (asked first), or drops the marks.
    fn bisect_end(&mut self, wt: &Path) {
        if self.bisect.is_some() {
            let mut text = self.tr("bisect.ask_reset").replace("{wt}", &act::name(wt));
            if self.claude_working(wt) {
                text.push_str(self.tr("act.revert_claude"));
            }
            self.band = Some(Band::new(text, Then::Act(Task::BisectReset { wt: wt.to_path_buf() })));
        } else if self.bisect_marks.is_empty() {
            self.say(self.tr("bisect.nothing"));
        } else {
            self.bisect_marks = Marks::default();
            self.say(self.tr("bisect.cleared"));
        }
    }

    /// `r`: the `git bisect run` line goes to the clipboard.
    fn bisect_copy(&mut self, wt: &Path) {
        let line = match (&self.bisect, self.bisect_marks.both()) {
            (Some(_), _) => michi_core::bisect::run_line(wt, None, TEST_COMMAND),
            (None, Some((bad, good))) => michi_core::bisect::run_line(wt, Some((&bad, &good)), TEST_COMMAND),
            (None, None) => return self.say_for(self.tr("bisect.no_marks"), 5),
        };
        self.clipboard = Some(line);
        self.say_for(self.tr("bisect.copied").replace("{cmd}", TEST_COMMAND), 8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use michi_core::summary::{Head, Summary};

    use crate::app::tests::{read, toast};
    use crate::graph::fixture;
    use crate::repo::Answer;

    fn oid(c: char) -> Oid {
        Oid::parse(&c.to_string().repeat(40)).unwrap()
    }

    fn ready() -> App {
        let mut a = read();
        let top = PathBuf::from("C:/r/repo");
        a.repo = Repo::Ready(Summary { name: "repo".into(), head: Head::Branch("main".into()), worktrees: 3, top });
        a.take(Answer::Trees(fixture::trees()));
        a
    }

    fn state(testing: bool) -> State {
        State {
            bad: Some(oid('b')),
            good: if testing { vec![oid('a')] } else { Vec::new() },
            skipped: Vec::new(),
            head: Some(oid('c')),
            head_subject: "middle".into(),
            first_bad: None,
            first_subject: String::new(),
            possible: false,
            left: Some(6),
            steps: Some(3),
        }
    }

    fn select_other(a: &mut App) {
        a.graph.move_by(1);
    }

    #[test]
    fn two_marks_ask_before_starting() {
        let mut a = ready();
        let first = a.graph.selected().unwrap();
        assert!(a.feed('x'));
        assert_eq!(a.bisect_marks.bad, Some(first));
        assert!(a.band.is_none());
        assert!(toast(&a).contains("bad"), "{}", toast(&a));
        assert!(a.strip_shown());
        select_other(&mut a);
        let second = a.graph.selected().unwrap();
        assert_ne!(first, second);
        assert!(a.feed('y'));
        let band = a.band.as_ref().expect("asks");
        assert!(band.text.contains("checks out"), "{}", band.text);
        let Then::Act(Task::BisectStart { bad, good, .. }) = &band.then else { panic!("{:?}", band.then) };
        assert_eq!((*bad, *good), (first, second));
    }

    #[test]
    fn the_same_commit_cannot_be_both() {
        let mut a = ready();
        assert!(a.feed('x'));
        assert!(a.feed('y'));
        assert!(a.band.is_none());
        assert_eq!(a.bisect_marks.good, None);
        assert!(toast(&a).contains("same"), "{}", toast(&a));
    }

    #[test]
    fn a_worktree_in_the_middle_of_something_is_not_marked() {
        let mut a = ready();
        let top = PathBuf::from("C:/r/repo");
        if let Some(t) = a.trees.trees.iter_mut().find(|t| crate::picker::same(&t.wt.path, &top)) {
            t.doing = Some(michi_core::branch::Doing::Merge);
        }
        assert!(a.feed('x'));
        assert!(a.bisect_marks.is_empty());
        assert!(toast(&a).contains("merge"), "{}", toast(&a));
    }

    #[test]
    fn x_drops_the_marks_and_the_strip_goes() {
        let mut a = ready();
        a.feed('x');
        assert_eq!(a.bisect_actions().iter().map(|(k, _)| *k).collect::<Vec<_>>(), ['X']);
        assert!(a.strip_text().unwrap().contains("bad"));
        assert!(a.feed('X'));
        assert!(a.bisect_marks.is_empty() && !a.strip_shown());
        assert!(a.feed('X'));
        assert!(toast(&a).contains("No bisect"), "{}", toast(&a));
    }

    #[test]
    fn the_strip_follows_the_bisect() {
        let mut a = ready();
        a.bisect = Some(state(true));
        assert!(a.strip_shown());
        let text = a.strip_text().unwrap();
        assert!(text.contains("middle") && text.contains('3'), "{text}");
        let keys: Vec<char> = a.strip_actions().iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, ['y', 'x', 'u', 'r', 'X']);
        let mut s = state(true);
        s.first_bad = Some(oid('d'));
        s.first_subject = "the culprit".into();
        a.bisect = Some(s);
        assert!(a.strip_text().unwrap().contains("the culprit"));
        assert_eq!(a.strip_actions().iter().map(|(k, _)| *k).collect::<Vec<_>>(), ['X']);
        a.bisect = Some(state(false));
        assert_eq!(a.strip_actions().iter().map(|(k, _)| *k).collect::<Vec<_>>(), ['y', 'x', 'X']);
    }

    #[test]
    fn x_while_bisecting_asks_before_ending() {
        let mut a = ready();
        a.bisect = Some(state(true));
        assert!(a.feed('X'));
        let band = a.band.as_ref().expect("asks");
        assert!(matches!(band.then, Then::Act(Task::BisectReset { .. })));
        assert!(band.text.contains("repo"), "{}", band.text);
    }

    #[test]
    fn the_run_line_is_copied() {
        let mut a = ready();
        assert!(a.feed('r'));
        assert!(a.clipboard.is_none());
        assert!(toast(&a).contains("Mark"), "{}", toast(&a));
        a.bisect = Some(state(true));
        assert!(a.feed('r'));
        let line = a.clipboard.take().unwrap();
        assert!(line.contains("bisect run <test-command>") && !line.contains("bisect start"), "{line}");
        a.bisect = None;
        a.bisect_marks = Marks { bad: Some(oid('b')), good: Some(oid('a')) };
        assert!(a.feed('r'));
        assert!(a.clipboard.take().unwrap().contains(&format!("bisect start {} {}", oid('b'), oid('a'))));
    }

    #[test]
    fn a_finished_task_says_where_it_stands() {
        let mut a = ready();
        a.bisect_marks = Marks { bad: Some(oid('b')), good: Some(oid('a')) };
        let wt = PathBuf::from("C:/r/repo");
        let task = Task::BisectStart { wt: wt.clone(), bad: oid('b'), good: oid('a') };
        let (text, _) = a.bisect_done(&task, Some(state(true)), Some("refs/michi/backup/20261017-120000".into()));
        assert!(text.contains("repo") && text.contains("middle") && text.contains("20261017-120000"), "{text}");
        assert!(a.bisect_marks.is_empty() && a.bisect.is_some());
        let task = Task::BisectReset { wt };
        let (text, _) = a.bisect_done(&task, None, Some("refs/michi/backup/20261017-120100".into()));
        assert!(text.contains("20261017-120100") && a.bisect.is_none(), "{text}");
    }

    #[test]
    fn a_reading_for_another_worktree_is_dropped() {
        let mut a = ready();
        a.bisect_read(Path::new("C:/elsewhere"), Some(state(true)));
        assert!(a.bisect.is_none());
        a.bisect_read(Path::new("C:/r/repo"), Some(state(true)));
        assert!(a.bisect.is_some());
    }

    #[test]
    fn skip_needs_a_commit_to_test() {
        let mut a = ready();
        assert!(a.feed('u'));
        assert!(toast(&a).contains("skip") || toast(&a).contains("test"), "{}", toast(&a));
    }
}
