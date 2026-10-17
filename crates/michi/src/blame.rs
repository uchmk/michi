//! The blame screen (`B` over the graph or the changes): who last touched
//! each line of a file, and (`H`, or Tab) the commits that touched it,
//! renames followed. Enter on a row goes to its commit in the graph. The file
//! is the one the detail is on in the graph (as of the selected commit), or
//! the selected file of the changes (as the worktree has it). blame_ui.rs
//! draws it.

use std::path::PathBuf;

use michi_core::blame::Blamed;
use michi_core::oid::Oid;
use michi_core::show::Status;

use crate::act::Task;
use crate::app::{App, Repo, View};

/// What the list shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Blame,
    History,
}

/// The screen while it is open.
pub struct Blame {
    pub path: String,
    /// The commit the file is read at; `None` for the working tree.
    pub rev: Option<Oid>,
    pub tab: Tab,
    /// `None` while it is read.
    pub data: Option<Blamed>,
    /// The selected row of each tab.
    pub sel: [usize; 2],
}

impl Blame {
    pub fn new(path: String, rev: Option<Oid>, tab: Tab) -> Blame {
        Blame { path, rev, tab, data: None, sel: [0, 0] }
    }

    fn slot(&self) -> usize {
        match self.tab {
            Tab::Blame => 0,
            Tab::History => 1,
        }
    }

    /// How many rows the tab in front has.
    pub fn len(&self) -> usize {
        self.data.as_ref().map_or(0, |d| match self.tab {
            Tab::Blame => d.lines.len(),
            Tab::History => d.history.len(),
        })
    }

    pub fn sel(&self) -> usize {
        self.sel[self.slot()]
    }

    /// Moves the selection by `by` rows, stopping at the ends.
    pub fn move_by(&mut self, by: i64) {
        let last = self.len().saturating_sub(1) as i64;
        let k = self.slot();
        self.sel[k] = (self.sel[k] as i64 + by).clamp(0, last) as usize;
    }

    /// The commit of the selected row.
    pub fn commit(&self) -> Option<Oid> {
        let d = self.data.as_ref()?;
        match self.tab {
            Tab::Blame => d.lines.get(self.sel()).map(|l| l.commit),
            Tab::History => d.history.get(self.sel()).map(|c| c.id),
        }
    }
}

impl App {
    /// `B` (blame) or `H` (history): reads the file the view is on, off the UI
    /// thread, and opens the screen at once.
    pub(crate) fn open_blame(&mut self, tab: Tab) {
        if let Some(b) = &mut self.blame {
            b.tab = tab;
            return;
        }
        let Repo::Ready(s) = &self.repo else { return };
        let top = s.top.clone();
        let found: Option<(PathBuf, Option<Oid>, String, bool)> = match self.view {
            View::Graph => {
                let wip = self.graph.selected_wip().map(|w| w.path.clone());
                let at = if wip.is_some() { None } else { self.graph.selected() };
                let dir = wip.unwrap_or(top);
                match self.detail.got() {
                    Some(Ok(d)) => {
                        let i = self.detail.scroll_to.unwrap_or(0).min(d.files.len().saturating_sub(1));
                        d.files.get(i).map(|f| (dir, at, f.path.clone(), f.status == Status::Deleted))
                    }
                    _ => None,
                }
            }
            View::Changes => {
                let dir = self.changes.wt.clone().unwrap_or(top);
                self.changes.selected().map(|(_, f)| (dir, None, f.path.clone(), f.status == Status::Deleted))
            }
            _ => return,
        };
        let Some((dir, rev, path, deleted)) = found else {
            let text = self.tr("blame.no_file").to_owned();
            return self.say(text);
        };
        if deleted {
            let text = self.tr("blame.deleted").replace("{path}", &path);
            return self.say(text);
        }
        self.blame = Some(Blame::new(path.clone(), rev, tab));
        self.act(Task::FindBlame { dir, rev, path });
    }

    /// The file has been read: it is the one the screen waits for, or it is
    /// dropped (the screen was closed or opened on another file meanwhile).
    pub(crate) fn blame_found(&mut self, path: &str, rev: Option<Oid>, data: Blamed) {
        let Some(b) = &mut self.blame else { return };
        if b.path != path || b.rev != rev {
            return;
        }
        if data.lines.is_empty() && data.history.is_empty() {
            self.blame = None;
            let text = self.tr("blame.nothing").replace("{path}", path);
            return self.say(text);
        }
        b.data = Some(data);
    }

    /// A character while the screen is open.
    pub(crate) fn blame_key(&mut self, c: char) -> bool {
        match c {
            'j' => self.blame_move(1),
            'k' => self.blame_move(-1),
            'B' => self.blame_tab(Tab::Blame),
            'H' => self.blame_tab(Tab::History),
            _ => {}
        }
        true
    }

    fn blame_move(&mut self, by: i64) {
        if let Some(b) = &mut self.blame {
            b.move_by(by);
        }
    }

    fn blame_tab(&mut self, tab: Tab) {
        if let Some(b) = &mut self.blame {
            b.tab = tab;
        }
    }

    /// A named key while the screen is open.
    pub(crate) fn blame_named(&mut self, name: &str) -> bool {
        let Some(b) = &mut self.blame else { return false };
        let n = b.len() as i64;
        match name {
            "ArrowDown" => b.move_by(1),
            "ArrowUp" => b.move_by(-1),
            "PageDown" => b.move_by(20),
            "PageUp" => b.move_by(-20),
            "Home" => b.move_by(-n),
            "End" => b.move_by(n),
            "Tab" => b.tab = if b.tab == Tab::Blame { Tab::History } else { Tab::Blame },
            "Enter" => self.blame_enter(),
            _ => {}
        }
        true
    }

    /// Enter: closes the screen and selects the row's commit in the graph.
    pub(crate) fn blame_enter(&mut self) {
        let Some(id) = self.blame.as_ref().and_then(Blame::commit) else { return };
        if id.is_zero() {
            let text = self.tr("blame.uncommitted").to_owned();
            return self.say(text);
        }
        self.blame = None;
        self.view = View::Graph;
        self.go_to(id);
    }
}

#[cfg(test)]
mod tests {
    use michi_core::blame::{Change, Line};

    use super::*;
    use crate::app::tests::{app, toast};

    fn oid(c: char) -> Oid {
        Oid::parse(&c.to_string().repeat(40)).unwrap()
    }

    fn data() -> Blamed {
        let line = |c: char, text: &str| Line {
            commit: oid(c),
            path: "f.txt".into(),
            author: "Ann".into(),
            when: 100,
            summary: "s".into(),
            uncommitted: c == '0',
            boundary: false,
            text: text.into(),
        };
        Blamed {
            lines: vec![line('a', "one"), line('b', "two"), line('0', "three")],
            history: vec![
                Change { id: oid('b'), author: "Ann".into(), when: 200, subject: "second".into() },
                Change { id: oid('a'), author: "Ann".into(), when: 100, subject: "first".into() },
            ],
        }
    }

    fn open(a: &mut App, tab: Tab) {
        let mut b = Blame::new("f.txt".into(), None, tab);
        b.data = Some(data());
        a.blame = Some(b);
    }

    #[test]
    fn each_tab_keeps_its_own_selection_and_stops_at_the_ends() {
        let mut a = app();
        open(&mut a, Tab::Blame);
        assert!(a.feed('j'));
        assert!(a.feed('j'));
        assert!(a.feed('j'));
        assert_eq!(a.blame.as_ref().unwrap().sel(), 2);
        assert!(a.named("Tab"));
        assert_eq!(a.blame.as_ref().unwrap().tab, Tab::History);
        assert_eq!(a.blame.as_ref().unwrap().sel(), 0);
        assert!(a.named("End"));
        assert_eq!(a.blame.as_ref().unwrap().commit(), Some(oid('a')));
        assert!(a.feed('B'));
        assert_eq!(a.blame.as_ref().unwrap().sel(), 2);
        assert!(a.named("Home"));
        assert_eq!(a.blame.as_ref().unwrap().commit(), Some(oid('a')));
    }

    #[test]
    fn an_uncommitted_line_has_no_commit_to_go_to() {
        let mut a = app();
        open(&mut a, Tab::Blame);
        assert!(a.named("End"));
        assert!(a.named("Enter"));
        assert!(a.blame.is_some());
        assert_eq!(toast(&a), "This line is not committed yet");
    }

    #[test]
    fn enter_closes_the_screen_and_goes_to_the_graph() {
        let mut a = app();
        a.view = View::Changes;
        open(&mut a, Tab::History);
        assert!(a.named("Enter"));
        assert!(a.blame.is_none());
        assert_eq!(a.view, View::Graph);
    }

    #[test]
    fn an_answer_for_another_file_is_dropped_and_an_empty_one_closes_the_screen() {
        let mut a = app();
        a.blame = Some(Blame::new("f.txt".into(), None, Tab::Blame));
        a.blame_found("other.txt", None, data());
        assert!(a.blame.as_ref().unwrap().data.is_none());
        a.blame_found("f.txt", None, data());
        assert_eq!(a.blame.as_ref().unwrap().len(), 3);
        a.blame = Some(Blame::new("f.txt".into(), None, Tab::Blame));
        a.blame_found("f.txt", None, Blamed { lines: Vec::new(), history: Vec::new() });
        assert!(a.blame.is_none());
        assert_eq!(toast(&a), "f.txt has nothing to blame");
    }

    #[test]
    fn esc_closes_it() {
        let mut a = app();
        open(&mut a, Tab::Blame);
        assert!(a.escape());
        assert!(a.blame.is_none());
    }
}
