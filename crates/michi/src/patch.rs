//! Patches over the graph: `Y` copies the selected commit (or the selected
//! worktree's uncommitted changes) to the clipboard as a patch, and pasting a
//! patch (Ctrl+V, Cmd+V) asks to apply it to the window's worktree. The work
//! is `michi_core::patch`; this is the keys, the question and the toasts.

use crate::act::{self, Task};
use crate::app::{App, Repo, View};
use crate::work::{Band, Then};

impl App {
    /// `Y`: asks the act thread for the patch of the selected commit, or of
    /// the uncommitted changes when a worktree's WIP row is selected. The
    /// text goes to the clipboard when it is made.
    pub(crate) fn copy_patch(&mut self) {
        let (wt, id) = match self.graph.selected_wip() {
            Some(w) => (w.path.clone(), None),
            None => {
                let Repo::Ready(s) = &self.repo else { return self.say(self.tr("act.no_tree")) };
                let top = s.top.clone();
                match self.graph.selected().and_then(|id| self.graph.find_commit(&id)) {
                    Some(c) => (top, Some(c.id)),
                    None => return self.say(self.tr("act.pick_commit")),
                }
            }
        };
        self.act(Task::ExportPatch { wt, id });
    }

    /// A paste: a patch in it is offered to the window's worktree (a yes
    /// applies it). Other text, and a paste while something else has the
    /// keys, are let be. False when nothing was done with it.
    pub(crate) fn pasted(&mut self, text: &str) -> bool {
        use crate::app::Mode;
        let free = self.mode == Mode::Main
            && self.view == View::Graph
            && self.band.is_none()
            && self.picker.is_none()
            && self.new_work.is_none()
            && self.reply.is_none()
            && self.tidy.is_none()
            && self.reflog.is_none()
            && self.stats.is_none()
            && self.ready.is_none()
            && self.blame.is_none()
            && self.trial.is_none()
            && self.reorder.is_none();
        if !free || !michi_core::patch::looks_like_patch(text) {
            return false;
        }
        let Repo::Ready(s) = &self.repo else {
            self.say(self.tr("act.no_tree"));
            return true;
        };
        let wt = s.top.clone();
        if let Some(d) = self.doing_in(&wt) {
            let said = self.tr("act.revert_doing").replace("{wt}", &act::name(&wt)).replace("{doing}", &self.doing_word(d));
            self.say_for(said, 8);
            return true;
        }
        let lines = text.lines().count();
        let mut said = self.tr("patch.ask").replace("{n}", &lines.to_string()).replace("{wt}", &act::name(&wt));
        if self.claude_working(&wt) {
            said.push_str(self.tr("act.revert_claude"));
        }
        self.band = Some(Band::new(said, Then::Act(Task::ApplyPatch { wt, patch: text.to_owned() })));
        true
    }
}
