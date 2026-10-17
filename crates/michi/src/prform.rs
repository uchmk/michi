//! The pull request form (`G` in the worktree table): title, description,
//! base branch and draft for the selected row's branch, made with `gh`. A
//! branch with no upstream is pushed first, and only after the form asks a
//! second time (a push goes outside the machine). The commands and the
//! starting text are `michi_core::prcreate`; this is the state, the keys and
//! the words.

use std::path::PathBuf;

use michi_core::git;
use michi_core::prcreate::{self, Draft, Failure, Request};

use crate::act::Task;
use crate::app::App;

pub struct PrForm {
    pub wt: PathBuf,
    pub branch: String,
    pub base: String,
    pub title: String,
    pub body: String,
    pub draft: bool,
    /// The remote the branch is pushed to first (none: it has an upstream).
    pub push_to: Option<String>,
    /// The push was shown and waits for the second Ctrl+Enter.
    pub confirm: bool,
    /// `gh` is at work.
    pub busy: bool,
    /// The address of the pull request that was made.
    pub done: Option<String>,
    /// Opened this frame: the click that opened it is not a click outside.
    pub opening: bool,
}

impl PrForm {
    /// The request the form makes.
    pub fn request(&self) -> Request {
        Request {
            branch: self.branch.clone(),
            base: self.base.trim().to_owned(),
            title: self.title.trim().to_owned(),
            body: self.body.trim().to_owned(),
            draft: self.draft,
            push_to: self.push_to.clone(),
        }
    }
}

impl App {
    /// `G`: reads the starting text for the selected row's branch; the form
    /// opens when it is read.
    pub(crate) fn open_pr_form(&mut self) {
        let Some(t) = self.wt_selected() else { return self.say(self.tr("act.no_tree")) };
        let (wt, branch) = (t.wt.path.clone(), t.wt.branch.clone());
        let Some(branch) = branch else { return self.say(self.tr("act.no_branch")) };
        let base = self.trees.trunk.clone().or_else(|| self.graph.trunk_name().map(str::to_owned)).unwrap_or_else(|| "main".to_owned());
        self.act(Task::PrDraft { wt, branch, base });
    }

    pub(crate) fn pr_draft_found(&mut self, wt: &std::path::Path, branch: &str, base: &str, d: Draft) {
        self.pr_form = Some(PrForm {
            wt: wt.to_path_buf(),
            branch: branch.to_owned(),
            base: base.to_owned(),
            title: d.title,
            body: d.body,
            draft: false,
            push_to: d.push_to,
            confirm: false,
            busy: false,
            done: None,
            opening: true,
        });
    }

    /// Ctrl+Enter in the form: creates, after the push was agreed to.
    pub(crate) fn pr_submit(&mut self) {
        let Some(f) = &mut self.pr_form else { return };
        if f.busy || f.done.is_some() {
            return;
        }
        if f.title.trim().is_empty() {
            return self.say(self.tr("pr.no_title"));
        }
        if f.base.trim().is_empty() {
            return self.say(self.tr("pr.no_base"));
        }
        if f.push_to.is_some() && !f.confirm {
            f.confirm = true;
            return;
        }
        f.busy = true;
        let task = Task::CreatePr { wt: f.wt.clone(), req: f.request() };
        self.act(task);
    }

    /// The words for a creation that failed: `gh` missing and not signed in
    /// say what to do.
    pub(crate) fn pr_failed(&mut self, e: &git::Error) -> (String, u64) {
        if let Some(f) = &mut self.pr_form {
            f.busy = false;
            f.confirm = false;
        }
        match prcreate::failure_of(e) {
            Failure::NoGh => (self.tr("pr.no_gh").to_owned(), 12),
            Failure::NotLoggedIn => (self.tr("pr.no_auth").to_owned(), 12),
            Failure::Other => {
                let text = self.tr("pr.failed").replace("{e}", &e.to_string());
                self.last_error = Some(text.clone());
                (text, 12)
            }
        }
    }

    /// A pull request was made: its address goes to the clipboard and the
    /// form turns to Open and Copy.
    pub(crate) fn pr_created(&mut self, url: String) -> (String, u64) {
        let said = self.tr("pr.done").replace("{url}", &url);
        self.clipboard = Some(url.clone());
        if let Some(f) = &mut self.pr_form {
            f.busy = false;
            f.done = Some(url);
        }
        (said, 12)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::{app, toast};

    fn form() -> PrForm {
        PrForm {
            wt: PathBuf::from("C:/r/feat"),
            branch: "feat".into(),
            base: "main".into(),
            title: " Add x ".into(),
            body: "- a\n".into(),
            draft: true,
            push_to: Some("origin".into()),
            confirm: false,
            busy: false,
            done: None,
            opening: false,
        }
    }

    #[test]
    fn the_request_is_trimmed_and_keeps_the_push() {
        let r = form().request();
        assert_eq!((r.title.as_str(), r.body.as_str(), r.draft, r.push_to.as_deref()), ("Add x", "- a", true, Some("origin")));
    }

    #[test]
    fn a_push_waits_for_a_second_ctrl_enter() {
        let mut a = app();
        a.pr_form = Some(form());
        a.pr_submit();
        let f = a.pr_form.as_ref().unwrap();
        assert!((f.confirm, f.busy) == (true, false), "the first asks");
        a.pr_submit();
        assert!(a.pr_form.as_ref().unwrap().busy, "the second goes");
    }

    #[test]
    fn no_upstream_to_push_goes_at_once_and_a_title_is_needed() {
        let mut a = app();
        let mut f = form();
        f.push_to = None;
        f.title = "  ".into();
        a.pr_form = Some(f);
        a.pr_submit();
        assert_eq!(toast(&a), a.tr("pr.no_title"));
        a.pr_form.as_mut().unwrap().title = "x".into();
        a.pr_submit();
        assert!(a.pr_form.as_ref().unwrap().busy);
    }

    #[test]
    fn failures_say_what_to_do_and_free_the_form() {
        let mut a = app();
        let mut f = form();
        f.busy = true;
        a.pr_form = Some(f);
        let (t, _) = a.pr_failed(&git::Error::Git(prcreate::NOT_LOGGED_IN.into()));
        assert!(t.contains("gh auth login"), "{t}");
        assert!(!a.pr_form.as_ref().unwrap().busy);
        let (t, _) = a.pr_failed(&git::Error::Git(prcreate::NO_GH.into()));
        assert!(t.contains("cli.github.com"), "{t}");
        let (t, _) = a.pr_failed(&git::Error::Git("boom".into()));
        assert!(t.contains("boom"));
    }

    #[test]
    fn a_made_pull_request_copies_its_address() {
        let mut a = app();
        a.pr_form = Some(form());
        let (t, _) = a.pr_created("https://github.com/o/r/pull/1".into());
        assert!(t.contains("pull/1"));
        assert_eq!(a.clipboard.as_deref(), Some("https://github.com/o/r/pull/1"));
        assert_eq!(a.pr_form.as_ref().unwrap().done.as_deref(), Some("https://github.com/o/r/pull/1"));
    }
}
