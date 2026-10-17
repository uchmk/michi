//! Git's terms on the screen (issues #104 and #105): the label of a term in
//! the easy display or as usual, and a dotted underline with a hover that
//! explains it. The words are `michi_core::glossary`.

use egui::{pos2, vec2, Id, Painter, Rect, Sense, Ui};
use michi_core::glossary;

use crate::app::App;

impl App {
    /// The label for glossary term `id`: the plain wording with Git's word in
    /// brackets when the easy display is on, else the words of `key`.
    pub fn term(&self, key: &'static str, id: &str) -> String {
        glossary::label(id, &self.lang, self.prefs.easy, self.tr(key))
    }
}

/// Underline `rect` (a term just drawn with `painter`) with dots and explain
/// glossary term `id` when the pointer rests on it. `salt` tells apart
/// the same term drawn twice.
pub fn hover(ui: &Ui, painter: &Painter, rect: Rect, id: &'static str, lang: &str, salt: &'static str) {
    let Some(term) = glossary::find(id) else { return };
    let c = ito_theme::colors();
    let y = rect.bottom() + 1.0;
    let mut x = rect.left();
    while x < rect.right() {
        painter.circle_filled(pos2(x + 0.5, y), 0.6, c.dim);
        x += 3.0;
    }
    let resp = ui.interact(rect.expand2(vec2(0.0, 2.0)), Id::new(("term", id, salt)), Sense::hover());
    resp.on_hover_ui(|ui| {
        ui.set_max_width(280.0);
        ui.label(egui::RichText::new(format!("{} — {}", term.git, term.plain(lang))).strong());
        ui.label(term.tip(lang));
    });
}
