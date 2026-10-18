//! The panes of the graph view (#133): Tab and Shift+Tab move the keys between
//! the graph's tree, the side column and the detail pane. In the side column
//! j/k walk the rows and Enter does what a click does; in the detail pane
//! j/k scroll and Enter widens it. Esc goes back to the tree.

use crate::app::{App, Mode, Repo};
use crate::side_ui::{rows, Kind};

/// Which pane of the graph view has the keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GraphPane {
    #[default]
    Tree,
    Side,
    Detail,
}

impl App {
    /// Whether Tab moves the panes now: the graph view, with nothing laid over it.
    pub(crate) fn panes_ready(&self) -> bool {
        self.view == crate::app::View::Graph && self.overlay_free()
    }

    /// Whether Tab moves the keys between the panes of the screen in front:
    /// the graph's, the changes' files and lines, the conflict's places, the
    /// branch review's files and diff.
    pub(crate) fn tab_ready(&self) -> bool {
        use crate::app::View;
        let has_panes = match self.view {
            View::Graph | View::Changes | View::Conflict => true,
            View::Worktree => self.review.is_some(),
        };
        has_panes && self.overlay_free()
    }

    /// Nothing is laid over the screen.
    fn overlay_free(&self) -> bool {
        self.mode == Mode::Main
            && !matches!(self.repo, Repo::Failed(_))
            && self.picker.is_none()
            && self.new_work.is_none()
            && self.pr_form.is_none()
            && self.reply.is_none()
            && self.repos.is_none()
            && self.band.is_none()
            && self.tidy.is_none()
            && self.trial.is_none()
            && self.ready.is_none()
            && self.auth.is_none()
            && self.reflog.is_none()
            && self.stats.is_none()
            && self.blame.is_none()
            && self.reorder.is_none()
            && self.cheat.is_none()
            && !self.legend
            && !self.guide_open
    }

    /// Tab (`back`: Shift+Tab) on the changes, the conflict or the review:
    /// the keys go between the list and what it shows.
    fn view_tab(&mut self) -> bool {
        use crate::app::View;
        let word = match self.view {
            View::Changes => {
                if self.changes.focus == crate::changes::Focus::Lines {
                    self.changes.leave_lines();
                    "pane.files"
                } else if let Err(why) = self.changes.enter_lines() {
                    let text = self.tr(why);
                    self.say(text);
                    return true;
                } else {
                    "pane.lines"
                }
            }
            View::Conflict => {
                let cf = &mut self.conflicts;
                cf.body_focus = !cf.body_focus;
                if cf.body_focus {
                    "pane.cf_body"
                } else {
                    "pane.cf_places"
                }
            }
            View::Worktree => {
                let Some(r) = self.review.as_mut() else { return false };
                r.body_focus = !r.body_focus;
                if r.body_focus {
                    "pane.diff"
                } else {
                    "pane.files"
                }
            }
            View::Graph => return false,
        };
        let said = self.tr("pane.now").replace("{pane}", self.tr(word));
        self.say(said);
        true
    }

    /// Esc: the keys come back to the list. False when they were there.
    pub(crate) fn view_pane_back(&mut self) -> bool {
        use crate::app::View;
        match self.view {
            View::Conflict => std::mem::take(&mut self.conflicts.body_focus),
            View::Worktree => self.review.as_mut().is_some_and(|r| std::mem::take(&mut r.body_focus)),
            _ => false,
        }
    }

    /// Arrow keys and j/k with the keys on the conflict's or the review's
    /// body: they scroll it by rows.
    pub(crate) fn body_scroll(&mut self, name: &str) -> bool {
        let by = match name {
            "ArrowDown" | "j" => 1,
            "ArrowUp" | "k" => -1,
            _ => return false,
        };
        match self.view {
            crate::app::View::Conflict if self.conflicts.body_focus => self.conflicts.nudge += by,
            crate::app::View::Worktree => match self.review.as_mut() {
                Some(r) if r.body_focus => r.nudge += by,
                _ => return false,
            },
            _ => return false,
        }
        true
    }

    /// The panes Tab goes through: the side column is hidden beside a wide diff.
    fn pane_ring(&self) -> Vec<GraphPane> {
        if self.detail.wide {
            vec![GraphPane::Tree, GraphPane::Detail]
        } else {
            vec![GraphPane::Tree, GraphPane::Detail, GraphPane::Side]
        }
    }

    /// Tab (`back`: Shift+Tab): the next pane.
    pub(crate) fn cycle_pane(&mut self, back: bool) {
        let ring = self.pane_ring();
        let at = ring.iter().position(|p| *p == self.pane).unwrap_or(0);
        let next = if back { (at + ring.len() - 1) % ring.len() } else { (at + 1) % ring.len() };
        self.pane = ring[next];
        if self.pane == GraphPane::Side {
            self.side_cur = self.side_selectable(self.side_cur as i64, 0);
            self.side_follow = true;
        }
        let word = match self.pane {
            GraphPane::Tree => "pane.tree",
            GraphPane::Side => "pane.side",
            GraphPane::Detail => "pane.detail",
        };
        let said = self.tr("pane.now").replace("{pane}", self.tr(word));
        self.say(said);
    }

    /// The nearest row that is not a heading, from `from`, going `dir`
    /// (0: either way, the row itself or the next one down).
    fn side_selectable(&self, from: i64, dir: i64) -> usize {
        let rows = rows(self);
        let ok = |i: i64| i >= 0 && (i as usize) < rows.len() && !matches!(rows[i as usize].kind, Kind::Heading(_));
        let mut i = from + dir;
        let step = if dir == 0 { 1 } else { dir };
        while i >= 0 && (i as usize) < rows.len() {
            if ok(i) {
                return i as usize;
            }
            i += step;
        }
        if dir == 0 {
            // Nothing below: the last row that can be chosen.
            (0..rows.len() as i64).rev().find(|i| ok(*i)).unwrap_or(0) as usize
        } else {
            self.side_cur
        }
    }

    fn side_move(&mut self, dir: i64) {
        self.side_cur = self.side_selectable(self.side_cur as i64, dir);
        self.side_follow = true;
    }

    /// Enter on the side column's row: what a click on it does.
    fn side_activate(&mut self) {
        let rows = rows(self);
        let Some(row) = rows.get(self.side_cur) else { return };
        match (&row.kind, row.id) {
            (Kind::Group { key, .. }, _) => {
                if !self.side_open.remove(key) {
                    self.side_open.insert(key.clone());
                }
            }
            (Kind::Stash, Some(id)) => {
                self.show_stash(id);
                self.pane = GraphPane::Tree;
            }
            (_, Some(id)) => {
                self.go_to(id);
                self.pane = GraphPane::Tree;
            }
            _ => {}
        }
    }

    /// A character with the keys in the side column or the detail pane.
    pub(crate) fn pane_key(&mut self, c: char) -> bool {
        match (self.pane, c) {
            (GraphPane::Side, 'j') => self.side_move(1),
            (GraphPane::Side, 'k') => self.side_move(-1),
            (GraphPane::Side, 'l' | 'h') => {}
            (GraphPane::Detail, 'j') => self.detail.lines += 1,
            (GraphPane::Detail, 'k') => self.detail.lines -= 1,
            (GraphPane::Detail, 'G') => self.detail.to_end = true,
            _ => return false,
        }
        true
    }

    /// A named key with the keys in a pane: Tab, and the side column's own.
    /// The detail pane's scrolling is the wide detail's, in `named`.
    pub(crate) fn pane_named(&mut self, name: &str) -> bool {
        if self.tab_ready() && self.view != crate::app::View::Graph && matches!(name, "Tab" | "Shift+Tab") {
            return self.view_tab();
        }
        if !self.panes_ready() {
            return false;
        }
        match (self.pane, name) {
            (_, "Tab") => self.cycle_pane(false),
            (_, "Shift+Tab") => self.cycle_pane(true),
            (GraphPane::Side, "ArrowDown") => self.side_move(1),
            (GraphPane::Side, "ArrowUp") => self.side_move(-1),
            (GraphPane::Side, "Enter") => self.side_activate(),
            // Sideways does not leave the column (Tab and Esc do).
            (GraphPane::Side, "ArrowRight" | "ArrowLeft") => {}
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::fixture;
    use crate::repo::Answer;

    fn app() -> App {
        let mut a = App::new(crate::prefs::Prefs::empty(), "C:/r/repo".into());
        a.lang = "en".into();
        a.take(Answer::Setup { gen: 1, setup: Box::new(fixture::setup()) });
        a.take(Answer::Commits { gen: 1, batch: fixture::history(), done: true });
        a.view = crate::app::View::Graph;
        a
    }

    #[test]
    fn tab_goes_round_the_panes_and_esc_comes_back() {
        let mut a = crate::app::tests::read();
        a.view = crate::app::View::Graph;
        assert!(a.panes_ready());
        assert!(a.named("Tab"));
        assert_eq!(a.pane, GraphPane::Detail);
        assert!(a.named("Tab"));
        assert_eq!(a.pane, GraphPane::Side);
        assert!(a.named("Tab"));
        assert_eq!(a.pane, GraphPane::Tree);
        assert!(a.named("Shift+Tab"));
        assert_eq!(a.pane, GraphPane::Side);
        assert!(a.escape());
        assert_eq!(a.pane, GraphPane::Tree);
    }

    #[test]
    fn tab_on_the_review_moves_the_keys_to_the_diff_and_esc_brings_them_back() {
        use crate::app::View;
        let mut a = crate::app::tests::read();
        a.view = View::Worktree;
        assert!(!a.tab_ready(), "the table has one pane");
        a.review = Some(crate::review::Review::new("C:/r/wt".into(), "lane".into()));
        assert!(a.tab_ready());
        assert!(a.named("Tab"));
        assert!(a.review.as_ref().unwrap().body_focus);
        assert!(a.named("ArrowDown"));
        assert!(a.feed('j'));
        assert_eq!(a.review.as_ref().unwrap().nudge, 2);
        assert!(a.escape());
        assert!(!a.review.as_ref().unwrap().body_focus);
        assert!(a.review.is_some(), "the first Esc only brings the keys back");
        assert!(a.named("Tab") && a.named("Shift+Tab"));
        assert!(!a.review.as_ref().unwrap().body_focus);
    }

    #[test]
    fn tab_on_the_changes_goes_between_the_files_and_the_lines() {
        use crate::app::View;
        let mut a = crate::app::tests::read();
        a.view = View::Changes;
        assert!(a.tab_ready());
        // Nothing read: the lines cannot take the keys, and the reason is told.
        assert!(a.named("Tab"));
        assert_eq!(a.changes.focus, crate::changes::Focus::Files);
        a.changes.focus = crate::changes::Focus::Lines;
        assert!(a.named("Shift+Tab"));
        assert_eq!(a.changes.focus, crate::changes::Focus::Files);
    }

    #[test]
    fn tab_does_not_move_panes_under_a_screen_laid_over() {
        let mut a = crate::app::tests::read();
        a.view = crate::app::View::Changes;
        a.cheat = None;
        a.legend = true;
        assert!(!a.tab_ready());
    }

    #[test]
    fn the_side_cursor_skips_headings_and_enter_opens_a_group() {
        let mut a = app();
        a.pane = GraphPane::Side;
        a.side_cur = a.side_selectable(0, 0);
        let rows_ = rows(&a);
        assert!(!matches!(rows_[a.side_cur].kind, Kind::Heading(_)));
        let first = a.side_cur;
        a.pane_key('j');
        assert!(a.side_cur > first);
        assert!(!matches!(rows(&a)[a.side_cur].kind, Kind::Heading(_)));
        a.pane_key('k');
        assert_eq!(a.side_cur, first);
        a.pane_key('k');
        assert_eq!(a.side_cur, first, "nothing above");
        let at = rows(&a).iter().position(|r| matches!(r.kind, Kind::Group { .. })).unwrap();
        a.side_cur = at;
        a.pane = GraphPane::Side;
        assert!(a.pane_key('l'), "right is taken and goes nowhere");
        assert!(a.pane == GraphPane::Side);
        assert!(a.pane_named("Enter"));
        assert!(a.side_open.contains("r:origin"));
    }

    #[test]
    fn question_mark_starts_a_deep_search_and_tab_changes_its_mode() {
        let mut a = app();
        assert!(a.feed('?'));
        assert_eq!(a.graph.filter(), "m:");
        assert!(a.graph.focus_filter);
        a.graph.set_filter("fix m:needle");
        a.graph.cycle_deep();
        assert_eq!(a.graph.filter(), "fix p:needle");
        a.graph.cycle_deep();
        a.graph.cycle_deep();
        assert_eq!(a.graph.filter(), "fix f:needle");
        a.graph.cycle_deep();
        assert_eq!(a.graph.filter(), "fix m:needle");
        a.graph.set_filter("g:x");
        a.graph.cycle_deep();
        assert_eq!(a.graph.filter(), "m:x", "another search turns into the first mode");
        a.graph.set_filter("plain");
        a.graph.cycle_deep();
        assert_eq!(a.graph.filter(), "plain", "no search typed: nothing to change");
    }

    #[test]
    fn j_scrolls_the_detail_pane() {
        let mut a = app();
        a.pane = GraphPane::Detail;
        assert!(a.feed('j'));
        assert_eq!(a.detail.lines, 1);
        assert!(a.named("Enter"));
        assert!(a.detail.wide);
    }
}
