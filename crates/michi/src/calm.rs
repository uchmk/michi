//! The two buttons beside the keybar's status: the calm save (#113) and the
//! next thing to do (#119). The window only reads the facts of its worktree
//! and does what `michi_core::suggest` names, through the keys a person
//! would have pressed.

use michi_core::paths;
use michi_core::suggest::{self, Facts, Suggestion};

use crate::act::Task;
use crate::app::{App, Repo};

impl App {
    /// Keep the whole state of this window's worktree under `refs/michi/backup`.
    /// It changes nothing, so no yes is asked; the toast says the way back.
    pub fn calm(&mut self) {
        let Some(wt) = self.repo_top() else { return };
        self.act(Task::Calm { wt });
    }

    /// How this window's worktree stands, for the suggestion.
    pub fn suggest_facts(&self) -> Option<Facts> {
        let Repo::Ready(s) = &self.repo else { return None };
        let key = paths::key(&s.top);
        let tree = self.trees.trees.iter().find(|t| paths::key(&t.wt.path) == key)?;
        let trunk = self.trees.trunk.as_deref().or_else(|| self.graph.trunk_name());
        let on_branch = tree.wt.branch.is_some();
        Some(Facts {
            doing: tree.doing,
            conflicts: self.conflicts.left() > 0,
            changes: self.wip_in(&s.top).map_or(0, |w| w.files.len()),
            on_branch,
            is_trunk: on_branch && tree.wt.branch.as_deref() == trunk,
            upstream: tree.upstream.as_ref().filter(|u| !u.gone).map(|u| (u.ahead, u.behind)),
            ahead_of_trunk: tree.ahead_behind.map_or(0, |(ahead, _)| ahead),
        })
    }

    /// The next thing to do here, if there is one.
    pub fn suggestion(&self) -> Option<Suggestion> {
        suggest::suggest(&self.suggest_facts()?)
    }

    /// Do the suggested step as its key would.
    pub fn run_suggestion(&mut self) {
        let Some(s) = self.suggestion() else { return };
        match s {
            Suggestion::Resolve | Suggestion::Finish => {
                if let Some(top) = self.repo_top() {
                    self.show_conflicts(&top);
                }
            }
            Suggestion::Commit => {
                self.mode = crate::app::Mode::Main;
                self.feed('2');
            }
            Suggestion::Pull => {
                self.mode = crate::app::Mode::Main;
                self.feed('p');
            }
            Suggestion::Push | Suggestion::Publish => {
                self.mode = crate::app::Mode::Main;
                self.feed('P');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_a_repository_there_is_nothing_to_suggest_or_keep() {
        let mut a = App::new(crate::prefs::Prefs::empty(), std::env::temp_dir());
        assert!(a.suggest_facts().is_none());
        assert!(a.suggestion().is_none());
        a.run_suggestion();
        a.calm();
    }
}
