//! `g` over the worktree table: the checklist before the selected branch is
//! merged or turned into a pull request (`michi_core::ready`): clean tree, no
//! clash with the trunk, not behind it, verify passed on this commit, GitHub's
//! checks, pushed. It only reads; the act thread gathers the facts (GitHub
//! can be slow). ready_ui.rs draws it.

use michi_core::paths;
use michi_core::ready::{self, Fact, Input, Status};

use crate::act::{self, Task};
use crate::app::App;

/// The list while it is open.
pub struct Ready {
    pub branch: String,
    pub trunk: String,
    pub facts: Vec<Fact>,
}

impl App {
    /// `g`: gather the facts for the selected worktree's branch.
    pub(crate) fn open_ready(&mut self) {
        let Some(t) = self.wt_selected() else { return self.say(self.tr("act.no_tree")) };
        let (wt, branch_name, head) = (t.wt.path.clone(), t.wt.branch.clone(), michi_core::oid::Oid::parse(&t.wt.head));
        let Some(trunk) = self.trees.trunk.clone().or_else(|| self.graph.trunk_name().map(str::to_owned)) else {
            return self.say(self.tr("act.no_trunk"));
        };
        let name = act::name(&wt);
        let fill = |a: &App, key: &'static str| a.tr(key).replace("{wt}", &name).replace("{trunk}", &trunk);
        let Some(branch) = branch_name else { return self.say_for(fill(self, "ready.detached"), 6) };
        if branch == trunk {
            return self.say_for(fill(self, "ready.is_trunk"), 6);
        }
        let Some(head) = head else { return self.say_for(fill(self, "ready.no_commit"), 6) };
        let key = paths::key(&wt);
        let dirty = self.wips.iter().find(|w| paths::key(&w.path) == key).map_or(0, |w| w.files.len() + w.left_out.len());
        let input = Input {
            clash: ready::clash_of(&self.clashes, &wt, &trunk),
            behind: t.ahead_behind.map(|(_, b)| b),
            upstream: t.upstream.as_ref().map(|u| (u.ahead, u.gone)),
            dirty: Some(dirty),
            wt,
            branch,
            head,
        };
        let text = fill(self, "ready.checking");
        self.say(text);
        self.act(Task::Ready { input, trunk });
    }

    /// The facts have come: open the list.
    pub(crate) fn ready_found(&mut self, input: &Input, trunk: &str, facts: Vec<Fact>) {
        self.toast = None;
        self.ready = Some(Ready { branch: input.branch.clone(), trunk: trunk.to_owned(), facts });
    }

    /// A character while the list is open: it takes them all.
    pub(crate) fn ready_key(&mut self, _c: char) -> bool {
        true
    }

    /// The words for a fact, with its numbers filled in.
    pub fn ready_text(&self, f: &Fact) -> String {
        let (key, n, extra): (&str, String, String) = match f {
            Fact::Clean => ("ready.clean", String::new(), String::new()),
            Fact::Dirty(n) => ("ready.dirty", n.to_string(), String::new()),
            Fact::TreeUnknown => ("ready.tree_unknown", String::new(), String::new()),
            Fact::NoClash => ("ready.no_clash", String::new(), String::new()),
            Fact::Clashes(files) => ("ready.clashes", files.len().to_string(), self.few(files)),
            Fact::ClashUnknown => ("ready.clash_unknown", String::new(), String::new()),
            Fact::UpToDate => ("ready.up_to_date", String::new(), String::new()),
            Fact::Behind(n) => ("ready.behind", n.to_string(), String::new()),
            Fact::BehindUnknown => ("ready.behind_unknown", String::new(), String::new()),
            Fact::VerifyPassed => ("ready.verify_ok", String::new(), String::new()),
            Fact::VerifyFailed(code) => ("ready.verify_failed", code.map_or_else(|| "?".to_owned(), |c| c.to_string()), String::new()),
            Fact::VerifyNone => ("ready.verify_none", String::new(), String::new()),
            Fact::CiPassed(n) => ("ready.ci_ok", n.to_string(), String::new()),
            Fact::CiFailed(n) => ("ready.ci_failed", n.to_string(), String::new()),
            Fact::CiPending(n) => ("ready.ci_pending", n.to_string(), String::new()),
            Fact::CiNone => ("ready.ci_none", String::new(), String::new()),
            Fact::CiNotPushed => ("ready.ci_not_pushed", String::new(), String::new()),
            Fact::CiNotGithub => ("ready.ci_not_github", String::new(), String::new()),
            Fact::CiUnknown(why) => ("ready.ci_unknown", String::new(), why.clone()),
            Fact::Pushed => ("ready.pushed", String::new(), String::new()),
            Fact::Unpushed(n) => ("ready.unpushed", n.to_string(), String::new()),
            Fact::NoUpstream => ("ready.no_upstream", String::new(), String::new()),
            Fact::UpstreamGone => ("ready.upstream_gone", String::new(), String::new()),
        };
        let trunk = self.ready.as_ref().map_or("", |r| r.trunk.as_str());
        self.tr(key).replace("{trunk}", trunk).replace("{n}", &n).replace("{files}", &extra).replace("{why}", &extra)
    }

    /// The word for an item.
    pub fn ready_item(&self, item: ready::Item) -> &'static str {
        self.tr(match item {
            ready::Item::Tree => "ready.item_tree",
            ready::Item::Clash => "ready.item_clash",
            ready::Item::Behind => "ready.item_behind",
            ready::Item::Verify => "ready.item_verify",
            ready::Item::Checks => "ready.item_checks",
            ready::Item::Pushed => "ready.item_pushed",
        })
    }
}

/// The mark in front of a row.
pub fn mark(s: Status) -> &'static str {
    match s {
        Status::Ok => "OK",
        Status::Warn => "!!",
        Status::Unknown => "??",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use michi_core::oid::Oid;
    use michi_core::summary::{Head, Summary};

    use super::*;
    use crate::app::tests::{read, toast};
    use crate::app::Repo;
    use crate::graph::fixture;

    fn on_table() -> App {
        let mut a = read();
        let top = PathBuf::from("C:/r/repo");
        a.repo = Repo::Ready(Summary { name: "repo".into(), head: Head::Branch("main".into()), worktrees: 3, top });
        a.take(crate::repo::Answer::Trees(fixture::trees()));
        assert!(a.feed('3'));
        a
    }

    #[test]
    fn g_says_why_the_trunk_and_a_detached_head_have_no_list() {
        let mut a = on_table();
        assert!(a.feed('g'));
        assert!(a.ready.is_none());
        assert_eq!(toast(&a), "repo is on main itself: there is nothing to check before merging it");
        a.feed('j');
        a.feed('j');
        a.feed('g');
        assert!(a.ready.is_none());
        assert_eq!(toast(&a), "old is on no branch: switch it to one before checking it");
    }

    #[test]
    fn g_on_a_branch_asks_and_the_facts_open_the_list() {
        let mut a = on_table();
        a.feed('j');
        assert!(a.feed('g'));
        assert_eq!(toast(&a), "Checking wt against main…");
        let input = Input {
            wt: PathBuf::from("C:/r/wt"),
            branch: "wip".into(),
            head: Oid::parse(&"a".repeat(40)).unwrap(),
            dirty: Some(0),
            clash: None,
            behind: None,
            upstream: None,
        };
        a.ready_found(
            &input,
            "main",
            vec![Fact::Dirty(2), Fact::Clashes(vec!["a.txt".into(), "b.txt".into()]), Fact::CiUnknown("no gh".into())],
        );
        let r = a.ready.as_ref().unwrap();
        assert_eq!(r.branch, "wip");
        assert_eq!(a.ready_text(&r.facts[0]), "2 changed files are not committed");
        assert!(a.ready_text(&r.facts[1]).starts_with("main would conflict in 2 files"), "{}", a.ready_text(&r.facts[1]));
        assert_eq!(a.ready_text(&r.facts[2]), "GitHub's checks could not be read: no gh");
        // The list takes every key; Esc closes it.
        assert!(a.feed('x'));
        assert!(a.ready.is_some());
        assert!(a.escape());
        assert!(a.ready.is_none());
    }
}
