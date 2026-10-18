//! The right-click menus of a commit, a branch (a worktree's row) and a
//! file. egui draws the popup; what it lists is `michi_core::menu`, the same
//! operations as the keys and the palette, and a pick is the key typed on the
//! row that was clicked.

use egui::{Button, Ui};
use michi_core::menu::{self, Kind};

use crate::app::App;

/// The menu's entries. The key of the one picked, if one was; the menu is
/// closed then.
pub fn show(ui: &mut Ui, app: &App, kind: Kind) -> Option<menu::Item> {
    ui.set_min_width(230.0);
    let mut picked = None;
    for it in menu::items(kind) {
        let button = Button::new(app.tr(it.word)).shortcut_text(if it.go {
            format!("g {}", menu::key_label(it.key))
        } else {
            menu::key_label(it.key)
        });
        if ui.add(button).clicked() {
            picked = Some(*it);
            ui.close();
        }
    }
    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_entry_is_named_and_is_a_key_of_its_view() {
        use crate::app::View;
        let table = |kind: Kind| match kind {
            Kind::Commit => View::Graph.keys(),
            Kind::Branch => View::Worktree.keys(),
            Kind::File => View::Changes.keys(),
        };
        for kind in [Kind::Commit, Kind::Branch, Kind::File] {
            for it in menu::items(kind) {
                assert_ne!(crate::i18n::tr("en", it.word), it.word, "{} has no words", it.word);
                let label = menu::key_label(it.key);
                let listed = table(kind).iter().any(|(keys, _)| keys.split([' ', '+']).any(|k| k == label || k == it.key.to_string()));
                // The three open-in keys share one line of the worktree table.
                let open = it.word.starts_with("key.open_");
                assert!(listed || open, "{:?} is not a key of its view", it.key);
            }
        }
    }

    #[test]
    fn a_menu_shows_in_a_frame() {
        let app = App::new(crate::prefs::Prefs::empty(), std::env::temp_dir());
        let ctx = egui::Context::default();
        let mut picked = Some(menu::items(Kind::File)[0]);
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| picked = show(ui, &app, Kind::File));
        out.textures_delta.clear();
        assert_eq!(picked, None, "nothing is picked without a click");
    }
}
