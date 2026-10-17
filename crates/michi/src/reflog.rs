//! The reflog screen (`R` over the graph or the worktree table): where HEAD
//! has been and what michi kept under `refs/michi/backup/`, in one list.
//! Enter asks, then brings the row back as a new branch (nothing moves or
//! goes). reflog_ui.rs draws it.

use michi_core::reflog::{Entry, Source};

use crate::act::Task;
use crate::app::{App, Repo};
use crate::work::{Band, Then};

/// The list while it is open.
pub struct Reflog {
    pub entries: Vec<Entry>,
    pub sel: usize,
}

impl Reflog {
    pub fn new(entries: Vec<Entry>) -> Reflog {
        Reflog { entries, sel: 0 }
    }

    /// Moves the selection by `by` rows, stopping at the ends.
    pub fn move_by(&mut self, by: i32) {
        let last = self.entries.len().saturating_sub(1) as i64;
        self.sel = (self.sel as i64 + i64::from(by)).clamp(0, last) as usize;
    }

    pub fn selected(&self) -> Option<&Entry> {
        self.entries.get(self.sel)
    }
}

impl App {
    /// `R`: read HEAD's reflog and the backups, off the UI thread.
    pub(crate) fn open_reflog(&mut self) {
        let Repo::Ready(s) = &self.repo else { return };
        let here = s.top.clone();
        self.act(Task::FindReflog { here });
    }

    /// The rows have come: open the list, or say there are none.
    pub(crate) fn reflog_found(&mut self, entries: Vec<Entry>) {
        if entries.is_empty() {
            let text = self.tr("reflog.nothing").to_owned();
            self.say(text);
            return;
        }
        self.reflog = Some(Reflog::new(entries));
    }

    /// A character while the list is open: `j` `k` move.
    pub(crate) fn reflog_key(&mut self, c: char) -> bool {
        let Some(r) = &mut self.reflog else { return false };
        match c {
            'j' => r.move_by(1),
            'k' => r.move_by(-1),
            _ => {}
        }
        true
    }

    /// A named key while the list is open; under the band, none.
    pub(crate) fn reflog_named(&mut self, name: &str) -> bool {
        if self.band.is_some() {
            return true;
        }
        if name == "Enter" {
            self.reflog_enter();
            return true;
        }
        let Some(r) = &mut self.reflog else { return false };
        let n = r.entries.len() as i32;
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

    /// Enter: the band asks before the selected row comes back as a branch.
    pub(crate) fn reflog_enter(&mut self) {
        let Some(e) = self.reflog.as_ref().and_then(Reflog::selected) else { return };
        let what = match &e.source {
            Source::Head(_) => e.label(),
            Source::Backup(name) => michi_core::backup::short(name).to_owned(),
        };
        let text = self.tr("reflog.ask").replace("{id}", &e.id.short()).replace("{what}", &what).replace("{subject}", &e.subject);
        self.band = Some(Band::new(text, Then::Act(Task::Recover { id: e.id })));
    }
}

#[cfg(test)]
mod tests {
    use michi_core::oid::Oid;

    use super::*;
    use crate::app::tests::{app, toast};
    use crate::app::View;

    fn entry(n: usize, backup: bool) -> Entry {
        let id = Oid::parse(&format!("{n:x}").repeat(40)[..40]).unwrap();
        let source = if backup { Source::Backup(format!("refs/michi/backup/2026101{n}-120000")) } else { Source::Head(n) };
        Entry { id, source, what: "reset: moving to HEAD~1".into(), subject: format!("commit {n}"), when: 1_000 - n as i64 }
    }

    #[test]
    fn nothing_found_is_said_and_rows_open_the_list() {
        let mut a = app();
        a.reflog_found(Vec::new());
        assert!(a.reflog.is_none());
        assert_eq!(toast(&a), "The reflog and michi's backups are empty");
        a.reflog_found(vec![entry(1, false), entry(2, true)]);
        assert_eq!(a.reflog.as_ref().unwrap().entries.len(), 2);
    }

    #[test]
    fn keys_move_and_enter_asks_before_a_branch_is_made() {
        let mut a = app();
        a.view = View::Graph;
        a.reflog_found(vec![entry(1, false), entry(2, true)]);
        assert!(a.feed('j'));
        assert_eq!(a.reflog.as_ref().unwrap().sel, 1);
        assert!(a.named("ArrowUp"));
        assert_eq!(a.reflog.as_ref().unwrap().sel, 0);
        assert!(a.named("Enter"));
        let band = a.band.as_ref().expect("the band");
        assert_eq!(band.text, "Bring 1111111 back as a new branch (HEAD@{1}: commit 1)? Nothing moves and nothing is dropped.");
        let Then::Act(Task::Recover { id }) = &band.then else { panic!("{:?}", band.then) };
        assert_eq!(*id, entry(1, false).id);
        // Under the band the list's keys do nothing; Esc drops the band.
        assert!(a.named("ArrowDown"));
        assert_eq!(a.reflog.as_ref().unwrap().sel, 0);
        assert!(a.escape());
        assert!(a.band.is_none());
        assert!(a.reflog.is_some());
        assert!(a.escape());
        assert!(a.reflog.is_none());
    }

    #[test]
    fn the_branch_made_is_said() {
        let mut a = app();
        a.act_done(Task::Recover { id: entry(1, false).id }, Ok(crate::act::Acted::Recovered("recovered/1111111".into())));
        assert_eq!(toast(&a), "Brought 1111111 back as the branch recovered/1111111");
    }
}
