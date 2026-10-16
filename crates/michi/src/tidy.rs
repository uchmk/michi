//! The clean-up list (`c` over the worktree table): what michi_core::tidy
//! found, the rows picked, and the keys that pick them. Nothing is picked at
//! first; Enter asks with the band before anything goes. tidy_ui.rs draws it.

use std::path::Path;

use michi_core::backup;
use michi_core::paths;
use michi_core::tidy::{self, Found, Item, Outcome, Stay, Why};

use crate::act::{self, Task};
use crate::app::{App, Repo};
use crate::work::{Band, Then};

/// The list while it is open.
pub struct Tidy {
    pub found: Found,
    /// Which rows are picked, one for each of `found.items`.
    pub checked: Vec<bool>,
    pub sel: usize,
}

impl Tidy {
    pub fn new(found: Found) -> Tidy {
        let checked = vec![false; found.items.len()];
        Tidy { found, checked, sel: 0 }
    }

    /// Moves the selection by `by` rows, stopping at the ends.
    pub fn move_by(&mut self, by: i32) {
        let last = self.found.items.len().saturating_sub(1) as i64;
        self.sel = (self.sel as i64 + i64::from(by)).clamp(0, last) as usize;
    }

    /// Picks row `k`, or lets it go; a row that stays cannot be picked.
    pub fn toggle(&mut self, k: usize) {
        if self.found.items.get(k).is_some_and(|i| i.stay.is_none()) {
            self.checked[k] = !self.checked[k];
        }
    }

    /// Picks every row that can go; when they all are, lets them all go.
    pub fn toggle_all(&mut self) {
        let all = self.found.items.iter().zip(&self.checked).filter(|(i, _)| i.stay.is_none()).all(|(_, c)| *c);
        for (i, c) in self.found.items.iter().zip(&mut self.checked) {
            if i.stay.is_none() {
                *c = !all;
            }
        }
    }

    /// The rows picked.
    pub fn picks(&self) -> Vec<Item> {
        self.found.items.iter().zip(&self.checked).filter(|(_, c)| **c).map(|(i, _)| i.clone()).collect()
    }

    /// How many rows are picked.
    pub fn count(&self) -> usize {
        self.checked.iter().filter(|c| **c).count()
    }
}

impl App {
    /// `c` over the worktree table: list what can be cleaned up.
    pub(crate) fn open_tidy(&mut self) {
        let Repo::Ready(s) = &self.repo else { return };
        let here = s.top.clone();
        self.act(Task::FindTidy { here });
    }

    /// The list has come: open it, a worktree where a tsumugi tab is open
    /// marked to stay (its shell is in the folder); or say there is nothing.
    pub(crate) fn tidy_found(&mut self, mut found: Found) {
        for item in &mut found.items {
            if item.stay.is_none() && !item.lost && item.worktree.as_deref().is_some_and(|w| self.tab_in(w)) {
                item.stay = Some(Stay::Open);
            }
        }
        tidy::sort(&mut found.items);
        if found.items.is_empty() {
            let text = self.tr("tidy.nothing").replace("{trunk}", found.trunk.as_deref().unwrap_or("main"));
            self.say(text);
            return;
        }
        self.tidy = Some(Tidy::new(found));
    }

    /// Whether tsumugi has a tab in the worktree at `wt`, a Claude or a
    /// shell.
    fn tab_in(&self, wt: &Path) -> bool {
        let key = paths::key(wt);
        self.claude.iter().flatten().any(|(p, s)| paths::key(p) == key && !s.is_empty())
    }

    /// A character while the list is open: `j` `k` move, Space picks the
    /// row, `a` picks them all; the others do nothing.
    pub(crate) fn tidy_key(&mut self, c: char) -> bool {
        let Some(t) = &mut self.tidy else { return false };
        match c {
            'j' => t.move_by(1),
            'k' => t.move_by(-1),
            ' ' => t.toggle(t.sel),
            'a' => t.toggle_all(),
            _ => {}
        }
        true
    }

    /// A named key while the list is open; under the band, none.
    pub(crate) fn tidy_named(&mut self, name: &str) -> bool {
        if self.band.is_some() {
            return true;
        }
        if name == "Enter" {
            self.tidy_enter();
            return true;
        }
        let Some(t) = &mut self.tidy else { return false };
        let n = t.found.items.len() as i32;
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

    /// Enter: the band asks before the rows picked go, saying how many
    /// folders go and that unmerged commits are kept.
    pub(crate) fn tidy_enter(&mut self) {
        let Some(t) = &self.tidy else { return };
        let picks = t.picks();
        if picks.is_empty() {
            self.say(self.tr("tidy.pick_first"));
            return;
        }
        let names: Vec<String> = picks.iter().map(Item::name).collect();
        let mut text = self.tr("tidy.ask").replace("{n}", &picks.len().to_string()).replace("{list}", &self.few(&names));
        let folders: Vec<String> = picks.iter().filter(|i| !i.lost).filter_map(|i| i.worktree.as_deref()).map(act::name).collect();
        if !folders.is_empty() {
            text.push_str(&self.tr("tidy.ask_folders").replace("{}", &self.few(&folders)));
        }
        if picks.iter().any(|i| i.why == Why::Gone) {
            text.push_str(self.tr("tidy.ask_kept"));
        }
        self.band = Some(Band::new(text, Then::Act(Task::Tidy { picks })));
    }

    /// What the clean-up says when it is over: what went, the backups made,
    /// and what stayed with git's first line on why.
    pub(crate) fn tidied(&self, out: &Outcome) -> (String, u64) {
        let mut text = if out.gone.is_empty() {
            self.tr("tidy.none_gone").to_owned()
        } else {
            self.tr("tidy.done").replace("{n}", &out.gone.len().to_string()).replace("{list}", &self.few(&out.gone))
        };
        if !out.saved.is_empty() {
            let saved: Vec<&str> = out.saved.iter().map(|s| backup::short(s)).collect();
            text.push_str(&self.tr("tidy.kept").replace("{}", &saved.join(", ")));
        }
        if !out.stayed.is_empty() {
            let stayed: Vec<String> = out
                .stayed
                .iter()
                .map(|(name, why)| {
                    let why = why.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or(self.tr("tidy.moved"));
                    format!("{name} ({why})")
                })
                .collect();
            text.push_str(&self.tr("tidy.stayed").replace("{}", &stayed.join(", ")));
        }
        let secs = if out.saved.is_empty() && out.stayed.is_empty() { 6 } else { 14 };
        (text, secs)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use michi_core::claude::{Session, State};
    use michi_core::oid::Oid;

    use super::*;
    use crate::app::tests::{app, toast};
    use crate::app::View;

    fn item(branch: Option<&str>, wt: Option<&str>, why: Why, stay: Option<Stay>) -> Item {
        Item {
            branch: branch.map(str::to_owned),
            tip: branch.map(|_| Oid::parse(&"a".repeat(40)).unwrap()),
            worktree: wt.map(PathBuf::from),
            why,
            lost: why == Why::Lost,
            stay,
        }
    }

    fn found() -> Found {
        Found {
            trunk: Some("main".into()),
            items: vec![
                item(Some("done"), None, Why::Merged, None),
                item(Some("feat"), Some("/r/feat"), Why::Merged, None),
                item(Some("squashed"), None, Why::Gone, None),
                item(None, Some("/r/old"), Why::Lost, None),
                item(Some("busy"), Some("/r/busy"), Why::Merged, Some(Stay::Changes)),
            ],
        }
    }

    fn shell(cwd: &str) -> Session {
        Session {
            id: 1,
            state: State::Done,
            command: "pwsh".into(),
            agent: String::new(),
            cwd: cwd.into(),
            branch: String::new(),
            title: String::new(),
            note: String::new(),
            since_ms: 0,
            read_only: false,
        }
    }

    #[test]
    fn rows_that_stay_cannot_be_picked_and_a_picks_all() {
        let mut t = Tidy::new(found());
        assert_eq!(t.count(), 0);
        t.toggle(4);
        assert_eq!(t.count(), 0);
        t.toggle(1);
        assert_eq!(t.picks().iter().map(Item::name).collect::<Vec<_>>(), ["feat"]);
        t.toggle_all();
        assert_eq!(t.checked, [true, true, true, true, false]);
        t.toggle_all();
        assert_eq!(t.count(), 0);
        t.move_by(10);
        assert_eq!(t.sel, 4);
        t.move_by(-10);
        assert_eq!(t.sel, 0);
    }

    #[test]
    fn a_worktree_with_a_tab_stays_and_nothing_found_is_said() {
        let mut a = app();
        a.claude = Some(vec![(PathBuf::from("/r/feat"), vec![shell("/r/feat")])]);
        a.tidy_found(found());
        let t = a.tidy.as_ref().expect("the list");
        let rows: Vec<_> = t.found.items.iter().map(|i| (i.name(), i.stay)).collect();
        assert_eq!(
            rows,
            [
                ("done".into(), None),
                ("squashed".into(), None),
                ("old".into(), None),
                ("busy".into(), Some(Stay::Changes)),
                ("feat".into(), Some(Stay::Open)),
            ]
        );
        a.tidy = None;
        a.tidy_found(Found { trunk: Some("trunk".into()), items: Vec::new() });
        assert!(a.tidy.is_none());
        assert_eq!(
            toast(&a),
            "Nothing to clean up: no branch is merged into trunk or gone from its remote, and no worktree's folder is gone"
        );
    }

    #[test]
    fn keys_pick_and_enter_asks_before_anything_goes() {
        let mut a = app();
        a.view = View::Worktree;
        a.tidy_found(found());
        // Nothing picked: Enter says so.
        assert!(a.named("Enter"));
        assert!(a.band.is_none());
        assert_eq!(toast(&a), "Pick the rows to clean up with Space first");
        assert!(a.feed('a'));
        assert!(a.feed('j'));
        assert!(a.feed(' '));
        assert_eq!(a.wt_sel, 0, "the table's selection stays");
        assert!(a.named("Enter"));
        let band = a.band.as_ref().expect("the band");
        assert_eq!(
            band.text,
            "Clean up 3: done, squashed, old? \
             Commits no other branch has are kept under refs/michi/backup/ first."
        );
        let Then::Act(Task::Tidy { picks }) = &band.then else { panic!("{:?}", band.then) };
        assert_eq!(picks.len(), 3);
        // Under the band the list's keys do nothing; Esc drops the band and
        // the list is there with its picks.
        assert!(a.named("ArrowDown"));
        assert_eq!(a.tidy.as_ref().unwrap().sel, 1);
        assert!(a.escape());
        assert!(a.band.is_none());
        assert_eq!(a.tidy.as_ref().unwrap().count(), 3);
        assert!(a.feed(' '));
        assert!(a.named("Enter"));
        let band = a.band.as_ref().expect("the band");
        assert_eq!(
            band.text,
            "Clean up 4: done, feat, squashed and 1 more? \
             The worktree folders go too, with the files git ignores in them: feat. \
             Commits no other branch has are kept under refs/michi/backup/ first."
        );
        // y sends it and closes the list (no reading thread here).
        assert!(a.feed('y'));
        assert!(a.band.is_none());
        assert!(a.tidy.is_none());
        a.tidy_found(found());
        assert!(a.escape());
        assert!(a.tidy.is_none());
    }

    #[test]
    fn the_outcome_is_said() {
        let mut a = app();
        let out = Outcome { gone: vec!["done".into(), "old".into()], saved: Vec::new(), stayed: Vec::new() };
        a.act_done(Task::Tidy { picks: Vec::new() }, Ok(crate::act::Acted::Tidied(out)));
        assert_eq!(toast(&a), "Cleaned up 2: done, old");
        let out = Outcome {
            gone: vec!["squashed".into()],
            saved: vec!["refs/michi/backup/20261016-120000".into()],
            stayed: vec![
                ("feat".into(), "fatal: '/r/feat' contains modified or untracked files, use --force to delete it\n".into()),
                ("moved".into(), String::new()),
            ],
        };
        a.act_done(Task::Tidy { picks: Vec::new() }, Ok(crate::act::Acted::Tidied(out)));
        assert_eq!(
            toast(&a),
            "Cleaned up 1: squashed. Unmerged commits are kept in michi/backup/20261016-120000. \
             Stayed: feat (fatal: '/r/feat' contains modified or untracked files, use --force to delete it), \
             moved (it moved after it was listed)"
        );
        let out = Outcome { gone: Vec::new(), saved: Vec::new(), stayed: vec![("moved".into(), String::new())] };
        a.act_done(Task::Tidy { picks: Vec::new() }, Ok(crate::act::Acted::Tidied(out)));
        assert_eq!(toast(&a), "Nothing was cleaned up. Stayed: moved (it moved after it was listed)");
    }
}
