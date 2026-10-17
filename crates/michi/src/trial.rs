//! The trial-merge screen (`T` over the worktree table): pick worktrees with
//! Space in the order to take them into the trunk, Enter merges them in that
//! order in memory (michi_core::trial) and shows where each step clashes and
//! what the end differs from the trunk by. Nothing moves: no branch, index or
//! working folder is touched. trial_ui.rs draws it.

use michi_core::trial::Trial;

use crate::act::Task;
use crate::app::App;

/// A worktree that can be picked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub branch: String,
    /// The worktree's folder name, for the list.
    pub folder: String,
}

/// The screen while it is open.
pub struct TrialScreen {
    pub trunk: String,
    pub rows: Vec<Row>,
    /// Indexes into `rows`, in the order picked (the order of the merges).
    pub order: Vec<usize>,
    pub sel: usize,
    /// What the last run found, for the branches in `ran`.
    pub result: Option<Trial>,
    /// A run is out and its answer has not come.
    pub running: bool,
}

impl TrialScreen {
    pub fn new(trunk: String, rows: Vec<Row>) -> TrialScreen {
        TrialScreen { trunk, rows, order: Vec::new(), sel: 0, result: None, running: false }
    }

    pub fn move_by(&mut self, by: i32) {
        let last = self.rows.len().saturating_sub(1) as i64;
        self.sel = (self.sel as i64 + i64::from(by)).clamp(0, last) as usize;
    }

    /// Picks row `k` as the next to take in, or takes it back. An old result
    /// no longer fits the picks, so it goes.
    pub fn toggle(&mut self, k: usize) {
        if k >= self.rows.len() {
            return;
        }
        match self.order.iter().position(|&i| i == k) {
            Some(at) => _ = self.order.remove(at),
            None => self.order.push(k),
        }
        self.result = None;
    }

    pub fn clear(&mut self) {
        self.order.clear();
        self.result = None;
    }

    /// The place of row `k` in the order (from 1), if picked.
    pub fn place(&self, k: usize) -> Option<usize> {
        self.order.iter().position(|&i| i == k).map(|p| p + 1)
    }

    /// The branches picked, in order.
    pub fn branches(&self) -> Vec<String> {
        self.order.iter().map(|&i| self.rows[i].branch.clone()).collect()
    }
}

impl App {
    /// `T` over the worktree table: the worktrees on a branch other than the
    /// trunk can be picked.
    pub(crate) fn open_trial(&mut self) {
        let Some(trunk) = self.trees.trunk.clone().or_else(|| self.graph.trunk_name().map(str::to_owned)) else {
            return self.say(self.tr("act.no_trunk"));
        };
        let rows: Vec<Row> = self
            .trees
            .trees
            .iter()
            .filter(|t| !t.wt.bare && t.wt.prunable.is_none())
            .filter_map(|t| {
                let branch = t.wt.branch.clone().filter(|b| *b != trunk)?;
                Some(Row { branch, folder: crate::act::name(&t.wt.path) })
            })
            .collect();
        if rows.len() < 2 {
            return self.say(self.tr("trial.too_few").replace("{trunk}", &trunk));
        }
        self.trial = Some(TrialScreen::new(trunk, rows));
    }

    /// A character while the screen is open: `j` `k` move, Space picks, `u`
    /// takes every pick back.
    pub(crate) fn trial_key(&mut self, c: char) -> bool {
        let Some(t) = &mut self.trial else { return false };
        match c {
            'j' => t.move_by(1),
            'k' => t.move_by(-1),
            ' ' => t.toggle(t.sel),
            'u' => t.clear(),
            _ => {}
        }
        true
    }

    /// A named key while the screen is open.
    pub(crate) fn trial_named(&mut self, name: &str) -> bool {
        if name == "Enter" {
            self.trial_run();
            return true;
        }
        let Some(t) = &mut self.trial else { return false };
        let n = t.rows.len() as i32;
        match name {
            "ArrowDown" => t.move_by(1),
            "ArrowUp" => t.move_by(-1),
            "PageDown" => t.move_by(10),
            "PageUp" => t.move_by(-10),
            "Home" => t.move_by(-n),
            "End" => t.move_by(n),
            _ => {}
        }
        true
    }

    /// Enter: merges the picks in their order, off the UI thread.
    pub(crate) fn trial_run(&mut self) {
        let Some(t) = &mut self.trial else { return };
        if t.order.is_empty() {
            self.say(self.tr("trial.pick_first"));
            return;
        }
        if t.running {
            return;
        }
        t.running = true;
        let task = Task::Trial { trunk: t.trunk.clone(), branches: t.branches() };
        self.act(task);
    }

    /// The answer has come: keep it when it is still for the picks.
    pub(crate) fn trial_done(&mut self, branches: &[String], found: Trial) {
        let Some(t) = &mut self.trial else { return };
        t.running = false;
        if t.branches() == branches {
            t.result = Some(found);
        }
    }

    /// A run failed: the screen can run again.
    pub(crate) fn trial_failed(&mut self) {
        if let Some(t) = &mut self.trial {
            t.running = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use michi_core::oid::Oid;
    use michi_core::predict::{Conflict, ConflictFile};
    use michi_core::track::Tree;
    use michi_core::trial::Step;
    use michi_core::worktree::Worktree;

    use super::*;
    use crate::app::tests::{app, toast};
    use crate::app::View;

    fn tree(path: &str, branch: Option<&str>) -> Tree {
        Tree {
            wt: Worktree {
                path: path.into(),
                head: "a".repeat(40),
                branch: branch.map(str::to_owned),
                bare: false,
                detached: branch.is_none(),
                locked: None,
                prunable: None,
            },
            ahead_behind: None,
            upstream: None,
            doing: None,
        }
    }

    fn with_trees(a: &mut App, list: &[(&str, Option<&str>)]) {
        a.trees.trunk = Some("main".into());
        a.trees.trees = list.iter().map(|(p, b)| tree(p, *b)).collect();
        a.view = View::Worktree;
    }

    fn found(branches: &[&str]) -> Trial {
        Trial {
            trunk: "main".into(),
            steps: branches
                .iter()
                .map(|b| Step {
                    name: (*b).into(),
                    conflict: (*b == "b").then(|| Conflict { files: vec![ConflictFile { path: "f".into(), hunks: Some(2) }] }),
                    already: false,
                })
                .collect(),
            tree: Oid::parse(&"b".repeat(40)).unwrap(),
            changed: Vec::new(),
        }
    }

    #[test]
    fn it_needs_two_worktrees_off_the_trunk() {
        let mut a = app();
        with_trees(&mut a, &[("/r/repo", Some("main")), ("/r/a", Some("a")), ("/r/d", None)]);
        assert!(a.feed('T'));
        assert!(a.trial.is_none());
        assert_eq!(toast(&a), "Trial merge needs at least two worktrees on a branch other than main");
    }

    #[test]
    fn picks_make_the_order_and_a_change_drops_the_result() {
        let mut a = app();
        with_trees(&mut a, &[("/r/repo", Some("main")), ("/r/a", Some("a")), ("/r/b", Some("b")), ("/r/c", Some("c"))]);
        assert!(a.feed('T'));
        let t = a.trial.as_ref().expect("the screen");
        assert_eq!(t.rows.iter().map(|r| r.branch.as_str()).collect::<Vec<_>>(), ["a", "b", "c"]);
        // Enter with nothing picked says so.
        assert!(a.named("Enter"));
        assert_eq!(toast(&a), "Pick the worktrees to merge with Space first, in the order to take them in");
        assert!(a.feed('j'));
        assert!(a.feed('j'));
        assert!(a.feed(' '));
        assert!(a.feed('k'));
        assert!(a.feed('k'));
        assert!(a.feed(' '));
        let t = a.trial.as_ref().unwrap();
        assert_eq!(t.branches(), ["c", "a"]);
        assert_eq!((t.place(2), t.place(0), t.place(1)), (Some(1), Some(2), None));
        assert_eq!(a.wt_sel, 0, "the table's selection stays");
        // An answer for the picks is kept; a later change drops it.
        a.trial_done(&["c".into(), "a".into()], found(&["c", "a"]));
        assert!(a.trial.as_ref().unwrap().result.is_some());
        assert!(a.feed(' '));
        assert!(a.trial.as_ref().unwrap().result.is_none());
        assert_eq!(a.trial.as_ref().unwrap().branches(), ["c"]);
        // An answer for other picks is dropped.
        a.trial_done(&["a".into()], found(&["a"]));
        assert!(a.trial.as_ref().unwrap().result.is_none());
        assert!(a.feed('u'));
        assert!(a.trial.as_ref().unwrap().order.is_empty());
        assert!(a.escape());
        assert!(a.trial.is_none());
    }

    #[test]
    fn enter_runs_once_until_the_answer_comes() {
        let mut a = app();
        with_trees(&mut a, &[("/r/repo", Some("main")), ("/r/a", Some("a")), ("/r/b", Some("b"))]);
        a.open_trial();
        a.trial.as_mut().unwrap().toggle(1);
        a.trial_run();
        assert!(a.trial.as_ref().unwrap().running);
        a.trial_run();
        a.trial_failed();
        assert!(!a.trial.as_ref().unwrap().running);
        a.trial_done(&["b".into()], found(&["b"]));
        let r = a.trial.as_ref().unwrap().result.as_ref().unwrap();
        assert_eq!((r.clashing(), r.places()), (1, 2));
    }
}
