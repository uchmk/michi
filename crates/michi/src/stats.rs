//! The statistics screen (`I` over the graph or the worktree table): commits
//! per contributor and per week or month, over the whole history, the last 90
//! days or the current branch. The numbers are read off the UI thread and
//! counted in michi-core; stats_ui.rs draws them.

use michi_core::stats::{Grain, Range, Stats};

use crate::act::Task;
use crate::app::{App, Repo};

/// The screen while it is open.
pub struct Statistics {
    pub range: Range,
    pub grain: Grain,
    /// `None` while it is read.
    pub data: Option<Stats>,
    /// The selected contributor.
    pub sel: usize,
}

impl Statistics {
    pub fn new() -> Statistics {
        Statistics { range: Range::All, grain: Grain::Week, data: None, sel: 0 }
    }

    pub fn len(&self) -> usize {
        self.data.as_ref().map_or(0, |d| d.authors.len())
    }

    /// Moves the selection by `by` contributors, stopping at the ends.
    pub fn move_by(&mut self, by: i64) {
        let last = self.len().saturating_sub(1) as i64;
        self.sel = (self.sel as i64 + by).clamp(0, last) as usize;
    }
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

impl App {
    /// `I`: opens the screen at once and reads the history off the UI thread.
    pub(crate) fn open_stats(&mut self) {
        if self.stats.is_some() {
            return;
        }
        self.stats = Some(Statistics::new());
        self.read_stats();
    }

    /// Asks for the numbers of the range the screen is on.
    fn read_stats(&mut self) {
        let (Repo::Ready(s), Some(st)) = (&self.repo, &self.stats) else { return };
        let (here, range) = (s.top.clone(), st.range);
        self.act(Task::FindStats { here, range, now: now() });
    }

    /// The numbers have come: they are the range the screen waits for, or
    /// they are dropped (the screen was closed, or moved to another range).
    pub(crate) fn stats_found(&mut self, range: Range, data: Stats) {
        let Some(st) = &mut self.stats else { return };
        if st.range != range {
            return;
        }
        st.sel = st.sel.min(data.authors.len().saturating_sub(1));
        st.data = Some(data);
    }

    /// `r`: the next range, read afresh.
    fn stats_range(&mut self) {
        let Some(st) = &mut self.stats else { return };
        st.range = st.range.next();
        st.data = None;
        st.sel = 0;
        self.read_stats();
    }

    /// A character while the screen is open.
    pub(crate) fn stats_key(&mut self, c: char) -> bool {
        match c {
            'j' => self.stats_move(1),
            'k' => self.stats_move(-1),
            'r' => self.stats_range(),
            'w' => {
                if let Some(st) = &mut self.stats {
                    st.grain = st.grain.other();
                }
            }
            _ => {}
        }
        true
    }

    fn stats_move(&mut self, by: i64) {
        if let Some(st) = &mut self.stats {
            st.move_by(by);
        }
    }

    /// A named key while the screen is open; under the band, none.
    pub(crate) fn stats_named(&mut self, name: &str) -> bool {
        let Some(st) = &mut self.stats else { return false };
        let n = st.len() as i64;
        match name {
            "ArrowDown" => st.move_by(1),
            "ArrowUp" => st.move_by(-1),
            "PageDown" => st.move_by(10),
            "PageUp" => st.move_by(-10),
            "Home" => st.move_by(-n),
            "End" => st.move_by(n),
            _ => {}
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use michi_core::stats::parse;

    use super::*;
    use crate::app::tests::app;

    fn data() -> Stats {
        let text: String = [("Ann", "a@x.test", 300), ("Bob", "b@x.test", 200), ("Ann", "a@x.test", 100)]
            .iter()
            .map(|(n, e, t)| format!("{n}\0{e}\0{t}\n"))
            .collect();
        parse(&text, None)
    }

    #[test]
    fn keys_move_cycle_the_range_and_flip_the_grain() {
        let mut a = app();
        a.stats = Some(Statistics::new());
        a.stats_found(Range::All, data());
        assert_eq!(a.stats.as_ref().unwrap().len(), 2);
        assert!(a.feed('j'));
        assert_eq!(a.stats.as_ref().unwrap().sel, 1);
        assert!(a.named("ArrowDown"));
        assert_eq!(a.stats.as_ref().unwrap().sel, 1, "stops at the end");
        assert!(a.named("Home"));
        assert_eq!(a.stats.as_ref().unwrap().sel, 0);
        assert!(a.feed('w'));
        assert_eq!(a.stats.as_ref().unwrap().grain, Grain::Month);
        // The range moves on and drops the numbers, which are read again.
        assert!(a.feed('r'));
        let st = a.stats.as_ref().unwrap();
        assert_eq!(st.range, Range::Recent);
        assert!(st.data.is_none());
    }

    #[test]
    fn numbers_of_another_range_are_dropped_and_esc_closes() {
        let mut a = app();
        a.stats = Some(Statistics::new());
        a.stats.as_mut().unwrap().range = Range::Branch;
        a.stats_found(Range::All, data());
        assert!(a.stats.as_ref().unwrap().data.is_none());
        a.stats_found(Range::Branch, data());
        assert!(a.stats.as_ref().unwrap().data.is_some());
        assert!(a.escape());
        assert!(a.stats.is_none());
        // Numbers that arrive after the screen was closed change nothing.
        a.stats_found(Range::Branch, data());
        assert!(a.stats.is_none());
    }
}
