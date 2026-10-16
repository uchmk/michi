//! New work (`N`): the form's state and what starting it does. The worktrees
//! are made by tsumugi (michi-core's `newwork`); this is the branch, where it
//! starts from, what the Claude is asked and how many.

use std::path::PathBuf;

use michi_core::newwork::{self, Plan};

use crate::act::Task;
use crate::app::{App, Repo};

pub struct NewWork {
    pub repo: PathBuf,
    pub branch: String,
    /// The places it can start from: label to show, and the revision.
    pub froms: Vec<(String, String)>,
    pub from: usize,
    pub text: String,
    pub count: u8,
    /// Opened this frame: the click that opened it is not a click outside.
    pub opening: bool,
}

impl NewWork {
    pub fn plans(&self) -> Vec<Plan> {
        let from = self.froms.get(self.from).map_or("HEAD", |f| f.1.as_str());
        newwork::plans(self.branch.trim(), from, &self.text, self.count)
    }

    pub fn cycle(&mut self, by: isize) {
        let n = self.froms.len() as isize;
        self.from = (self.from as isize + by).rem_euclid(n.max(1)) as usize;
    }
}

impl App {
    /// `N`: opens the form for the window's repository.
    pub(crate) fn open_new_work(&mut self) {
        let Repo::Ready(s) = &self.repo else {
            self.say(self.tr("act.no_tree"));
            return;
        };
        let repo = s.top.clone();
        let mut froms = Vec::new();
        if let Some(trunk) = self.trees.trunk.clone().or_else(|| self.graph.trunk_name().map(str::to_owned)) {
            let refs = &self.graph.setup().refs;
            if let Some(r) = refs.remotes.iter().find(|r| r.name == trunk && r.remote == "origin") {
                let name = format!("{}/{}", r.remote, r.name);
                froms.push((name.clone(), name));
            }
            froms.push((trunk.clone(), trunk));
        }
        if let Some(id) = self.graph.selected() {
            froms.push((self.tr("new.commit").replace("{}", &id.short()), id.to_string()));
        }
        if froms.is_empty() {
            froms.push(("HEAD".to_owned(), "HEAD".to_owned()));
        }
        let branch = newwork::default_branch(&chrono::Local::now());
        self.new_work = Some(NewWork { repo, branch, froms, from: 0, text: String::new(), count: 1, opening: true });
    }

    /// Ctrl+Enter in the form: starts the worktrees.
    pub(crate) fn new_work_start(&mut self) {
        let Some(n) = &self.new_work else { return };
        if n.branch.trim().is_empty() {
            self.say(self.tr("new.no_branch"));
            return;
        }
        let task = Task::NewWork { repo: n.repo.clone(), plans: n.plans() };
        self.new_work = None;
        self.act(task);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::{read, toast};
    use michi_core::summary::{Head, Summary};

    fn ready() -> App {
        let mut a = read();
        a.repo =
            Repo::Ready(Summary { name: "repo".into(), head: Head::Branch("main".into()), worktrees: 1, top: PathBuf::from("C:/r/repo") });
        a
    }

    #[test]
    fn the_form_needs_a_repository() {
        let mut a = crate::app::tests::app();
        a.open_new_work();
        assert!(a.new_work.is_none());
        assert_eq!(toast(&a), "No worktree to work in");
    }

    #[test]
    fn the_form_opens_with_a_default_branch_and_the_selected_commit() {
        let mut a = ready();
        a.open_new_work();
        let n = a.new_work.as_ref().unwrap();
        assert!(n.branch.starts_with("tsumugi/"));
        assert!(!n.froms.is_empty());
        assert_eq!((n.from, n.count, n.opening), (0, 1, true));
        assert!(n.froms.last().is_some_and(|(l, _)| l.contains(&a.graph.selected().unwrap().short())));
    }

    #[test]
    fn start_refuses_an_empty_branch_and_otherwise_closes_the_form() {
        let mut a = ready();
        a.open_new_work();
        a.new_work.as_mut().unwrap().branch = "  ".into();
        a.new_work_start();
        assert!(a.new_work.is_some(), "the form stays so the branch can be typed");
        assert_eq!(toast(&a), a.tr("new.no_branch"));
        a.new_work.as_mut().unwrap().branch = "feat/x".into();
        a.new_work_start();
        assert!(a.new_work.is_none());
    }
}
