//! The `g` and `y` prefixes (#133): a first key opens a small window that
//! lists what the second key does (which-key style), so the dangerous and the
//! rare operations take two keys and can be found by looking. `g` is the
//! careful operations (and `gg`, the first row), `y` copies. The choices are
//! per view; each one calls what its own key already calls.

use eframe::egui::{self, pos2, Align2, FontId, Id, Sense, Stroke};

use crate::app::{App, Mode, View};

/// Which prefix is waiting for its second key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prefix {
    /// `g`: the first row, and the operations that can lose work.
    Go,
    /// `y`: copy something to the clipboard.
    Copy,
    /// `g b`: bisect.
    Bisect,
}

/// One line of a prefix's window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    pub key: char,
    /// The i18n key of its words.
    pub word: &'static str,
    /// Drawn in the warning colour: it can lose work.
    pub danger: bool,
}

const fn ok(key: char, word: &'static str) -> Choice {
    Choice { key, word, danger: false }
}

const fn risky(key: char, word: &'static str) -> Choice {
    Choice { key, word, danger: true }
}

impl Prefix {
    /// The i18n key of the window's title.
    fn title(self) -> &'static str {
        match self {
            Prefix::Go => "prefix.go",
            Prefix::Copy => "prefix.copy",
            Prefix::Bisect => "prefix.bisect",
        }
    }
}

impl App {
    /// Whether `g` opens its window here: the graph and the worktree table.
    pub(crate) fn g_prefix_view(&self) -> bool {
        match self.view {
            View::Graph => true,
            View::Worktree => self.review.is_none(),
            _ => false,
        }
    }

    /// Whether `y` opens the copy window here. While a bisect asks for good
    /// or bad, `y` is still "good" (the strip shows it).
    pub(crate) fn y_prefix_view(&self) -> bool {
        match self.view {
            View::Graph => !self.bisect_actions().iter().any(|(k, _)| *k == 'y') && !self.good_is_next(),
            View::Worktree => self.review.is_none(),
            _ => false,
        }
    }

    /// A bad commit is marked and a good one is not: `y` marks it.
    fn good_is_next(&self) -> bool {
        self.bisect.is_none() && self.bisect_marks.bad.is_some() && self.bisect_marks.good.is_none()
    }

    /// What the second key of `p` can be here.
    pub(crate) fn prefix_choices(&self, p: Prefix) -> Vec<Choice> {
        match (p, self.view) {
            (Prefix::Go, View::Graph) => {
                vec![ok('g', "pf.first"), risky('r', "pf.reset"), ok('b', "pf.bisect"), ok('?', "pf.cheat")]
            }
            (Prefix::Go, _) => vec![
                ok('g', "pf.first"),
                risky('x', "pf.remove"),
                risky('l', "pf.land"),
                risky('c', "pf.tidy"),
                risky('m', "pf.rebase"),
                ok('v', "pf.move"),
                risky('f', "pf.force"),
                ok('?', "pf.cheat"),
            ],
            (Prefix::Bisect, _) => vec![ok('y', "pf.good"), ok('x', "pf.bad"), ok('u', "pf.skip"), ok('r', "pf.run"), risky('X', "pf.end")],
            (Prefix::Copy, View::Graph) => {
                vec![ok('h', "pf.hash"), ok('p', "pf.patch"), ok('m', "pf.subject"), ok('a', "pf.author")]
            }
            (Prefix::Copy, _) => vec![ok('b', "pf.branch"), ok('d', "pf.folder")],
        }
    }

    /// The second key of a prefix. Always used: a key that is none of the
    /// choices closes the window and says so.
    pub(crate) fn prefix_key(&mut self, p: Prefix, c: char) -> bool {
        if !self.prefix_choices(p).iter().any(|ch| ch.key == c) {
            let said = self.tr("pf.none").replace("{key}", &c.to_string());
            self.say(said);
            return true;
        }
        match (p, c) {
            (Prefix::Go, 'g') => self.go_first(),
            (Prefix::Go, 'r') => self.ask_reset(),
            (Prefix::Go, 'b') => self.prefix = Some(Prefix::Bisect),
            (Prefix::Go, '?') => self.toggle_cheat(),
            (Prefix::Go, 'x') => self.ask_remove(),
            (Prefix::Go, 'l') => self.ask_land(),
            (Prefix::Go, 'c') => self.open_tidy(),
            (Prefix::Go, 'm') => self.ask_rebase_trunk(),
            (Prefix::Go, 'v') => self.open_move(),
            (Prefix::Go, 'f') => _ = self.net_key('F'),
            (Prefix::Bisect, c) => _ = self.bisect_key(c),
            (Prefix::Copy, 'h') => _ = self.copy_hash(),
            (Prefix::Copy, 'p') => self.copy_patch(),
            (Prefix::Copy, 'm') => self.copy_commit_field(false),
            (Prefix::Copy, 'a') => self.copy_commit_field(true),
            (Prefix::Copy, 'b') => self.copy_tree_field(true),
            (Prefix::Copy, 'd') => self.copy_tree_field(false),
            _ => {}
        }
        true
    }

    /// `y m` (the first line of the message) or `y a` (the author) of the
    /// selected commit.
    fn copy_commit_field(&mut self, author: bool) {
        let Some(c) = self.graph.selected().and_then(|id| self.graph.find_commit(&id)) else {
            return self.say(self.tr("act.pick_commit"));
        };
        let text = if author { format!("{} <{}>", c.author, c.email) } else { c.subject.clone() };
        self.copied(text);
    }

    /// `y b` (the branch) or `y d` (the folder) of the selected worktree.
    fn copy_tree_field(&mut self, branch: bool) {
        let Some(t) = self.wt_selected() else { return self.say(self.tr("act.no_tree")) };
        let text = if branch { t.wt.branch.clone() } else { Some(t.wt.path.display().to_string()) };
        match text {
            Some(text) => self.copied(text),
            None => self.say(self.tr("act.no_branch")),
        }
    }

    /// Puts `text` on the clipboard and says what.
    fn copied(&mut self, text: String) {
        let said = self.tr("copy.done").replace("{text}", &text);
        self.clipboard = Some(text);
        self.say(said);
    }
}

/// The window of the waiting prefix, above the key bar.
pub fn show(ctx: &egui::Context, app: &mut App) {
    let Some(p) = app.prefix else { return };
    if app.mode != Mode::Main {
        return;
    }
    let c = ito_theme::colors();
    let screen = ctx.content_rect();
    let choices = app.prefix_choices(p);
    egui::Area::new(Id::new("michi-prefix"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(screen.left() + 16.0, screen.bottom() - 40.0))
        .pivot(Align2::LEFT_BOTTOM)
        .show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.border)).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_min_width(240.0);
                ui.label(egui::RichText::new(app.tr(p.title())).strong());
                ui.add_space(2.0);
                for ch in &choices {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 22.0), Sense::hover());
                    let ink = if ch.danger { c.err } else { c.fg };
                    let painter = ui.painter_at(rect);
                    painter.text(pos2(rect.left() + 4.0, rect.center().y), Align2::LEFT_CENTER, ch.key, FontId::monospace(13.0), c.wait);
                    painter.text(
                        pos2(rect.left() + 30.0, rect.center().y),
                        Align2::LEFT_CENTER,
                        app.tr(ch.word),
                        FontId::proportional(13.0),
                        ink,
                    );
                }
                ui.label(egui::RichText::new(app.tr("prefix.esc")).font(FontId::proportional(12.0)).color(c.dim));
            });
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_choice_key_is_used_once_per_window() {
        for p in [Prefix::Go, Prefix::Copy, Prefix::Bisect] {
            for view in [View::Graph, View::Worktree] {
                let mut a = crate::app::tests::read();
                a.view = view;
                let keys: Vec<char> = a.prefix_choices(p).iter().map(|c| c.key).collect();
                let mut sorted = keys.clone();
                sorted.sort_unstable();
                sorted.dedup();
                assert_eq!(keys.len(), sorted.len(), "{p:?} in {view:?}");
            }
        }
    }

    #[test]
    fn g_then_g_goes_to_the_first_row_and_other_keys_close_the_window() {
        let mut a = crate::app::tests::read();
        a.view = View::Graph;
        assert!(a.feed('g'));
        assert_eq!(a.prefix, Some(Prefix::Go));
        assert!(a.feed('g'));
        assert_eq!(a.prefix, None);
        assert!(a.feed('g'));
        assert!(a.feed('Q'));
        assert_eq!(a.prefix, None);
    }

    #[test]
    fn y_opens_the_copy_window_and_h_closes_it() {
        let mut a = crate::app::tests::read();
        a.view = View::Graph;
        assert!(a.feed('y'));
        assert_eq!(a.prefix, Some(Prefix::Copy));
        assert!(a.feed('h'));
        assert_eq!(a.prefix, None);
    }

    #[test]
    fn g_b_opens_the_bisect_window() {
        let mut a = crate::app::tests::read();
        a.view = View::Graph;
        a.feed('g');
        a.feed('b');
        assert_eq!(a.prefix, Some(Prefix::Bisect));
    }

    #[test]
    fn esc_closes_the_window() {
        let mut a = crate::app::tests::read();
        a.view = View::Graph;
        a.feed('g');
        a.escape();
        assert_eq!(a.prefix, None);
    }
}
