//! Submodules (`u` on the worktree table): where the submodules of the
//! selected worktree stand against the commits it records, and, after a yes,
//! `submodule update` to put them there. michi does not go into a submodule.

use std::path::PathBuf;

use michi_core::submodule::{self, State, Sub};

use crate::act::{self, Task};
use crate::app::App;
use crate::work::{Band, Then};

impl App {
    /// `u`: read the submodules of the selected worktree, off the UI thread.
    pub(crate) fn open_submodules(&mut self) {
        let Some(t) = self.wt_selected() else { return self.say(self.tr("act.no_tree")) };
        let wt = t.wt.path.clone();
        self.act(Task::FindSubmodules { wt });
    }

    /// The states have come: say so when all is well, else ask before the update.
    pub(crate) fn submodules_found(&mut self, wt: PathBuf, subs: Vec<Sub>) {
        let name = act::name(&wt);
        if subs.is_empty() {
            let text = self.tr("sub.none").replace("{wt}", &name);
            return self.say(text);
        }
        let stale: Vec<String> = submodule::stale(&subs).into_iter().map(str::to_owned).collect();
        if stale.is_empty() {
            let clash: Vec<String> = subs.iter().filter(|s| s.state == State::Clash).map(|s| s.path.clone()).collect();
            let text = if clash.is_empty() {
                self.tr("sub.same").replace("{wt}", &name).replace("{n}", &subs.len().to_string())
            } else {
                self.tr("sub.clash").replace("{wt}", &name).replace("{list}", &self.few(&clash))
            };
            return self.say_for(text, 6);
        }
        let text = self.tr("sub.ask").replace("{wt}", &name).replace("{list}", &self.few(&stale));
        self.band = Some(Band::new(text, Then::Act(Task::UpdateSubmodules { wt })));
    }

    /// The update is over: the toast, with what is still off.
    pub(crate) fn submodules_updated(&self, wt: &std::path::Path, subs: &[Sub]) -> (String, u64) {
        let left: Vec<String> = submodule::stale(subs).into_iter().map(str::to_owned).collect();
        if left.is_empty() {
            (self.tr("sub.done").replace("{wt}", &act::name(wt)).replace("{n}", &subs.len().to_string()), 6)
        } else {
            (self.tr("sub.left").replace("{wt}", &act::name(wt)).replace("{list}", &self.few(&left)), 10)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::{app, toast};

    fn sub(path: &str, state: State) -> Sub {
        Sub { path: path.into(), id: "12345678".into(), state }
    }

    #[test]
    fn no_submodules_and_all_in_place_are_said() {
        let mut a = app();
        let wt = PathBuf::from("/x/repo");
        a.submodules_found(wt.clone(), Vec::new());
        assert_eq!(toast(&a), "repo has no submodules");
        a.submodules_found(wt.clone(), vec![sub("a", State::Same), sub("b", State::Same)]);
        assert_eq!(toast(&a), "All 2 submodules of repo are on their recorded commits");
        assert!(a.band.is_none());
        a.submodules_found(wt, vec![sub("a", State::Clash)]);
        assert_eq!(toast(&a), "A merge left a clash in the submodule(s) of repo: a. Resolve it before an update.");
        assert!(a.band.is_none());
    }

    #[test]
    fn a_submodule_off_its_commit_is_asked_about_before_the_update() {
        let mut a = app();
        let wt = PathBuf::from("/x/repo");
        a.submodules_found(wt.clone(), vec![sub("a", State::Same), sub("b", State::Differs), sub("c", State::Missing)]);
        let band = a.band.as_ref().expect("a question");
        assert!(band.text.contains("b, c"), "{}", band.text);
        assert_eq!(band.then, Then::Act(Task::UpdateSubmodules { wt }));
    }

    #[test]
    fn the_update_says_what_is_still_off() {
        let a = app();
        let wt = PathBuf::from("/x/repo");
        assert_eq!(a.submodules_updated(&wt, &[sub("a", State::Same)]).0, "Put the submodules of repo (1) on their recorded commits");
        assert_eq!(
            a.submodules_updated(&wt, &[sub("a", State::Differs)]).0,
            "Updated the submodules of repo, but these are still off their recorded commit: a"
        );
    }
}
