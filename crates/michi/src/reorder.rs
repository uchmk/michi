//! The rebase screen (`r` over the worktree table): the worktree's commits
//! since the trunk, oldest first, each to keep, fold into the one above,
//! reword or drop, and to move up or down. Nothing runs until Enter and the
//! band's yes. reorder_ui.rs draws it; michi_core's `rebase` runs it.

use michi_core::rebase::{self, Action, Entry, Plan, Problem};

use crate::act::{self, Task};
use crate::app::App;
use crate::work::{Band, Then};

/// The screen while it is open.
pub struct Reorder {
    pub plan: Plan,
    pub list: Vec<Entry>,
    pub sel: usize,
    /// The message being written for row `sel`.
    pub editing: Option<String>,
    /// Opened this frame: the click that opened it is not a click outside.
    pub opening: bool,
}

impl Reorder {
    pub fn new(plan: Plan) -> Reorder {
        Reorder { list: plan.entries.clone(), plan, sel: 0, editing: None, opening: true }
    }

    pub fn move_by(&mut self, by: i32) {
        let last = self.list.len().saturating_sub(1) as i64;
        self.sel = (self.sel as i64 + i64::from(by)).clamp(0, last) as usize;
    }

    /// Moves the selected commit one row `down` (later) or up, the selection with it.
    pub fn shift(&mut self, down: bool) {
        let to = if down { self.sel + 1 } else { self.sel.wrapping_sub(1) };
        if to < self.list.len() {
            self.list.swap(self.sel, to);
            self.sel = to;
        }
    }

    pub fn set(&mut self, action: Action) {
        if let Some(e) = self.list.get_mut(self.sel) {
            // The same key again takes it back.
            e.action = if e.action == action { Action::Pick } else { action };
            if e.action != Action::Reword && e.action != Action::Squash {
                e.message = None;
            }
        }
    }

    /// Starts writing the selected commit's message.
    pub fn edit(&mut self) {
        if let Some(e) = self.list.get_mut(self.sel) {
            let text = e.message.clone().unwrap_or_else(|| e.full.clone());
            if e.action != Action::Squash {
                e.action = Action::Reword;
            }
            self.editing = Some(text);
        }
    }

    /// Keeps what was written.
    pub fn edited(&mut self) {
        if let (Some(text), Some(e)) = (self.editing.take(), self.list.get_mut(self.sel)) {
            let text = text.trim().to_owned();
            if text.is_empty() || text == e.full {
                e.message = None;
                if e.action == Action::Reword {
                    e.action = Action::Pick;
                }
            } else {
                e.subject = text.lines().next().unwrap_or("").to_owned();
                e.message = Some(text);
            }
        }
    }

    pub fn problem(&self) -> Result<(), Problem> {
        rebase::check(&self.list, &self.plan.entries)
    }
}

impl App {
    /// `r` over the worktree table: reads the selected worktree's commits.
    pub(crate) fn open_reorder(&mut self) {
        let Some(t) = self.wt_selected() else { return self.say(self.tr("act.no_tree")) };
        let (wt, branch, doing) = (t.wt.path.clone(), t.wt.branch.clone(), t.doing);
        let Some(trunk) = self.trees.trunk.clone().or_else(|| self.graph.trunk_name().map(str::to_owned)) else {
            return self.say(self.tr("act.no_trunk"));
        };
        if branch.is_none() {
            return self.say(self.tr("act.no_branch"));
        }
        if branch.as_deref() == Some(trunk.as_str()) {
            return self.say(self.tr("reorder.on_trunk").replace("{trunk}", &trunk));
        }
        if doing.is_some() {
            let text = self
                .tr("act.revert_doing")
                .replace("{wt}", &act::name(&wt))
                .replace("{doing}", &doing.map_or_else(String::new, |d| self.doing_word(d)));
            return self.say_for(text, 6);
        }
        self.act(Task::FindReorder { wt, trunk });
    }

    /// The commits have been read: open the screen.
    pub(crate) fn reorder_found(&mut self, plan: Plan) {
        self.reorder = Some(Reorder::new(plan));
    }

    /// A character while the screen is open (not while a message is written).
    pub(crate) fn reorder_key(&mut self, c: char) -> bool {
        let Some(r) = &mut self.reorder else { return false };
        if r.editing.is_some() {
            return false;
        }
        match c {
            'j' => r.move_by(1),
            'k' => r.move_by(-1),
            'J' => r.shift(true),
            'K' => r.shift(false),
            'p' => {
                if let Some(e) = r.list.get_mut(r.sel) {
                    e.action = Action::Pick;
                    e.message = None;
                }
            }
            's' => r.set(Action::Squash),
            'f' => r.set(Action::Fixup),
            'd' => r.set(Action::Drop),
            'r' => r.edit(),
            _ => {}
        }
        true
    }

    /// A named key while the screen is open; under the band, none.
    pub(crate) fn reorder_named(&mut self, name: &str) -> bool {
        if self.band.is_some() {
            return true;
        }
        if name == "Enter" {
            self.reorder_enter();
            return true;
        }
        let Some(r) = &mut self.reorder else { return false };
        let n = r.list.len() as i32;
        match name {
            "ArrowDown" => r.move_by(1),
            "ArrowUp" => r.move_by(-1),
            "PageDown" => r.move_by(10),
            "PageUp" => r.move_by(-10),
            "Home" => r.move_by(-n),
            "End" => r.move_by(n),
            _ => {}
        }
        true
    }

    /// Enter: the band asks, saying what is rewritten and where the old tip
    /// is kept.
    pub(crate) fn reorder_enter(&mut self) {
        let Some(r) = &self.reorder else { return };
        match r.problem() {
            Err(Problem::Nothing) => return self.say(self.tr("reorder.nothing")),
            Err(Problem::FirstFolds) => return self.say(self.tr("reorder.first_folds")),
            Err(Problem::Empty) => return self.say(self.tr("reorder.empty")),
            Ok(()) => {}
        }
        let (wt, base, list) = (r.plan.wt.clone(), r.plan.base, r.list.clone());
        let count = |a: Action| list.iter().filter(|e| e.action == a).count();
        let folded = count(Action::Squash) + count(Action::Fixup);
        let mut text = self
            .tr("reorder.ask")
            .replace("{n}", &list.len().to_string())
            .replace("{wt}", &act::name(&wt))
            .replace("{folded}", &folded.to_string())
            .replace("{dropped}", &count(Action::Drop).to_string())
            .replace("{reworded}", &list.iter().filter(|e| e.message.is_some()).count().to_string());
        if self.claude_working(&wt) {
            text.push_str(self.tr("act.revert_claude"));
        }
        self.band = Some(Band::new(text, Then::Act(Task::Reorder { wt, base, list })));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use michi_core::oid::Oid;
    use std::path::PathBuf;

    fn entry(c: char, subject: &str) -> Entry {
        Entry {
            id: Oid::parse(&c.to_string().repeat(40)).unwrap(),
            subject: subject.into(),
            full: subject.into(),
            action: Action::Pick,
            message: None,
        }
    }

    fn open() -> Reorder {
        let entries = vec![entry('a', "one"), entry('b', "two"), entry('c', "three")];
        Reorder::new(Plan { wt: PathBuf::from("C:/r/repo"), base: Oid::parse(&"0".repeat(40)).unwrap(), trunk: "main".into(), entries })
    }

    #[test]
    fn the_selected_commit_moves_with_the_selection() {
        let mut r = open();
        r.shift(true);
        assert_eq!((r.sel, r.list[1].subject.as_str(), r.list[0].subject.as_str()), (1, "one", "two"));
        r.shift(true);
        r.shift(true);
        assert_eq!((r.sel, r.list[2].subject.as_str()), (2, "one"), "stops at the end");
        assert_eq!(r.problem(), Ok(()));
    }

    #[test]
    fn a_key_twice_takes_the_action_back() {
        let mut r = open();
        r.set(Action::Drop);
        assert_eq!(r.list[0].action, Action::Drop);
        r.set(Action::Drop);
        assert_eq!(r.list[0].action, Action::Pick);
        assert_eq!(r.problem(), Err(Problem::Nothing));
    }

    #[test]
    fn a_message_is_kept_only_when_it_changed() {
        let mut r = open();
        r.edit();
        assert_eq!(r.list[0].action, Action::Reword);
        r.editing = Some("one, better\n\nwith a body".into());
        r.edited();
        assert_eq!((r.list[0].subject.as_str(), r.list[0].message.as_deref()), ("one, better", Some("one, better\n\nwith a body")));
        r.edit();
        r.editing = Some("one".into());
        r.edited();
        assert_eq!((r.list[0].action, r.list[0].message.clone()), (Action::Pick, None));
    }

    #[test]
    fn enter_says_why_a_list_cannot_run_and_otherwise_asks() {
        let mut a = crate::app::tests::app();
        a.reorder = Some(open());
        a.reorder_enter();
        assert!(a.band.is_none());
        assert_eq!(crate::app::tests::toast(&a), a.tr("reorder.nothing"));
        a.reorder_key('s');
        a.reorder_enter();
        assert_eq!(crate::app::tests::toast(&a), a.tr("reorder.first_folds"));
        a.reorder_key('s');
        a.reorder_key('j');
        a.reorder_key('f');
        a.reorder_enter();
        assert!(a.band.is_some());
    }
}
