//! The command palette (`<C-S-p>`): every command michi has, found by what
//! is typed (`ito-palette`). The list is made from the same tables as the
//! key list, so a command is in it when it has a key; picking one does what
//! the key would have done.

use ito_palette::{Entry, Style, View, Words};

use crate::app::{App, GLOBAL_KEYS};
use crate::keymap::Act;
use michi_core::openin::Tool;

/// What picking an entry does.
#[derive(Clone, Debug, PartialEq)]
pub enum Pick {
    /// A command of the keymap (the settings, the key list, quit, the scale).
    Act(Act),
    /// A character typed in the main window.
    Key(char),
    /// Open the worktree's folder in a terminal, an editor or the file manager.
    Open(Tool),
}

/// The entries the palette offers now: the keymap's commands, then those of
/// every view's keys that are one character, the view in front first.
pub fn entries(app: &App) -> Vec<Entry<Pick>> {
    let mut out = Vec::new();
    for l in ito_keymap::help_lines(&app.prefs.keymap.main, "noop") {
        let desc = if l.desc.is_empty() { l.run.to_owned() } else { crate::i18n::tr_desc(&app.lang, l.desc) };
        if let Ok(act) = crate::keymap::parse(l.run) {
            out.push(Entry::new(desc, l.keys, Pick::Act(act)));
        }
    }
    for v in crate::app::View::ALL {
        out.push(Entry::new(format!("{} {}", app.tr("key.tab"), app.tr(v.label())), v.digit().to_string(), Pick::Key(v.digit())));
    }
    let in_table = app.view == crate::app::View::Worktree;
    for (t, key) in Tool::ALL.into_iter().zip(["t", "e", "E"]) {
        out.push(Entry::new(app.tr(crate::act::open_word_cmd(t)), if in_table { key } else { "" }, Pick::Open(t)));
    }
    let mut seen = Vec::new();
    for (keys, word) in app.keys().iter().chain(GLOBAL_KEYS.iter().skip(1)) {
        let mut chars = keys.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else { continue };
        if !seen.contains(&c) {
            seen.push(c);
            out.push(Entry::new(app.tr(word), keys.to_string(), Pick::Key(c)));
        }
    }
    out
}

/// Draw the palette and take its keys. What it picked, if anything; it is
/// closed on a pick, Esc and a click outside.
pub fn show(ctx: &egui::Context, app: &mut App) -> Option<Pick> {
    let mut view: View = app.cmd.take()?;
    let c = ito_theme::colors();
    let style = Style {
        panel: c.panel,
        border: c.border,
        text: c.fg,
        dim: c.dim,
        selected_bg: c.border,
        selected_text: crate::ui::accent(),
        hit: crate::ui::accent(),
        accent: crate::ui::accent(),
    };
    let words = Words { hint: app.tr("palette.hint"), none: app.tr("palette.none"), header: "", footer: "" };
    match ito_palette::show(ctx, &mut view, &entries(app), &style, &words) {
        Some(ito_palette::Answer::Pick(p)) => Some(p),
        Some(ito_palette::Answer::Close | ito_palette::Answer::Key(..)) => None,
        None => {
            app.cmd = Some(view);
            None
        }
    }
}

impl App {
    /// Open the palette, or close it; not over a box that has the keys.
    pub fn toggle_palette(&mut self) {
        if self.cmd.take().is_some() {
            return;
        }
        let busy = self.picker.is_some()
            || self.new_work.is_some()
            || self.band.is_some()
            || self.tidy.is_some()
            || self.reflog.is_some()
            || self.stats.is_some()
            || self.blame.is_some()
            || self.trial.is_some()
            || self.reorder.is_some()
            || self.conflicts.edit.is_some();
        if !busy {
            self.cmd = Some(View::new());
        }
    }

    /// Do what the palette picked, as the key would have.
    pub fn run_pick(&mut self, p: Pick) {
        match p {
            Pick::Act(a) => self.run_key(a),
            Pick::Open(t) => self.open_in(t, self.view == crate::app::View::Worktree),
            Pick::Key(c) => {
                self.mode = crate::app::Mode::Main;
                self.feed(c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_has_the_keymaps_commands_and_the_views_keys() {
        let a = App::new(crate::prefs::Prefs::empty(), std::env::temp_dir());
        let e = entries(&a);
        assert!(e.iter().any(|e| e.pick == Pick::Act(Act::Settings)));
        assert!(e.iter().any(|e| e.pick == Pick::Key('3')), "a tab");
        assert!(e.iter().any(|e| e.pick == Pick::Key('b')), "the graph's branch palette");
        assert!(!e.iter().any(|e| e.pick == Pick::Key('j')), "moving is not a command");
    }

    #[test]
    fn a_pick_does_what_the_key_does() {
        let mut a = App::new(crate::prefs::Prefs::empty(), std::env::temp_dir());
        a.toggle_palette();
        assert!(a.cmd.is_some());
        a.run_pick(Pick::Key('3'));
        assert_eq!(a.view, crate::app::View::Worktree);
        a.toggle_palette();
        assert!(a.cmd.is_none(), "the same key closes it");
    }
}
