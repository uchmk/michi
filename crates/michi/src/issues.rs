//! Issue links (#44): the references in the shown commit's message and in a
//! worktree's branch name, as addresses, and `I` to open one in the browser.
//! The finding and the addresses are `michi_core::issues`; this is what the
//! window does with them. Nothing is fetched: michi links, the browser (or
//! minato) shows the issue.

use michi_core::issues::{Ref, Tracker};

use crate::app::{App, View};

impl App {
    fn tracker(&self) -> Tracker<'_> {
        Tracker { config: &self.prefs.issues, slug: self.issue_slug.as_ref() }
    }

    /// The issues the shown commit's message names that link.
    pub(crate) fn commit_issues(&self) -> Vec<(Ref, String)> {
        match self.detail.got() {
            Some(Ok(d)) if self.graph.selected() == Some(d.id) => self.tracker().in_message(&format!("{}\n{}", d.subject, d.body)),
            _ => Vec::new(),
        }
    }

    /// The issues `branch`'s name names that link.
    pub(crate) fn branch_issues(&self, branch: &str) -> Vec<(Ref, String)> {
        self.tracker().in_branch(branch)
    }

    /// `I`: opens the first issue of the shown commit (in the graph) or of the
    /// selected worktree's branch (in the worktree table) in the browser; the
    /// toast names it, and how many more there are.
    pub(crate) fn open_issue(&mut self) {
        let found = if self.view == View::Worktree {
            let Some(t) = self.wt_selected() else { return self.say(self.tr("act.no_tree")) };
            let Some(branch) = t.wt.branch.clone() else { return self.say(self.tr("act.no_branch")) };
            self.branch_issues(&branch)
        } else {
            self.commit_issues()
        };
        let Some((first, url)) = found.first().cloned() else { return self.say(self.tr("issue.none")) };
        std::thread::spawn(move || {
            let _ = michi_core::pr::open_url(&url);
        });
        let mut text = self.tr("issue.opened").replace("{}", &first.label());
        if found.len() > 1 {
            text.push_str(&self.tr("issue.more").replace("{}", &(found.len() - 1).to_string()));
        }
        self.say(text);
    }
}
