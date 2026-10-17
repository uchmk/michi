//! The beginners' explanations (#120, #121, #122) drawn the way the views draw
//! the rest: with the painter on rects they lay out themselves. Which topics
//! are due and what is kept is `michi_core::guide`; the words are in
//! `i18n.rs`. Each draws a dismiss cross; the views draw [`link`] in its
//! place once it is dismissed, which brings it back.

use std::sync::Arc;

use egui::epaint::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{pos2, vec2, Align2, CornerRadius, CursorIcon, FontId, Galley, Id, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, Ui};
use michi_core::guide::{Stage, CONFLICT_CARDS};

use crate::app::App;
use crate::ui::accent;

const PAD: f32 = 14.0;
const PILL_H: f32 = 24.0;
const ARROW_W: f32 = 26.0;

/// What a click on an explanation asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Dismiss,
    Prev,
    Next,
}

/// The words of the three conflict cards: title and body.
const CONFLICT: [(&str, &str); CONFLICT_CARDS] = [
    ("guide.conflict.1.title", "guide.conflict.1.body"),
    ("guide.conflict.2.title", "guide.conflict.2.body"),
    ("guide.conflict.3.title", "guide.conflict.3.body"),
];

/// The words of the three steps of the stage diagram and the line for each.
const STAGE: [(&str, &str); 3] = [
    ("guide.stage.changed", "guide.stage.now.changed"),
    ("guide.stage.staged", "guide.stage.now.staged"),
    ("guide.stage.ready", "guide.stage.now.ready"),
];

/// Whether `hit` was clicked; a hand over it.
fn clicked(ui: &Ui, p: &Painter, hit: Rect, id: impl std::hash::Hash + std::fmt::Debug, live: bool, pointer: Option<Pos2>) -> bool {
    let resp = ui.interact(hit.intersect(p.clip_rect()), Id::new(id), Sense::CLICK);
    if live && pointer.is_some_and(|at| hit.contains(at)) {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    live && resp.clicked()
}

fn wrapped(ui: &Ui, text: &str, font: FontId, color: egui::Color32, width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(text.to_owned(), TextFormat { font_id: font, color, ..Default::default() });
    job.wrap = TextWrapping::wrap_at_width(width.max(40.0));
    ui.fonts_mut(|f| f.layout_job(job))
}

/// The dismiss cross with its top right corner at `at`; whether it was clicked.
fn cross(ui: &Ui, p: &Painter, at: Pos2, id: &str, live: bool, pointer: Option<Pos2>) -> bool {
    let c = ito_theme::colors();
    let hit = Rect::from_min_size(at - vec2(22.0, 0.0), vec2(22.0, 22.0));
    let hot = pointer.is_some_and(|m| hit.contains(m));
    p.text(hit.center(), Align2::CENTER_CENTER, "×", FontId::proportional(16.0), if hot { c.fg } else { c.dim });
    clicked(ui, p, hit, ("guide-x", id), live, pointer)
}

/// A small link, its right edge at `right`: "? what is a conflict". Its left
/// edge, and whether it was clicked.
#[expect(clippy::too_many_arguments, reason = "the painter, the place and the look are the frame's")]
pub fn link(ui: &Ui, p: &Painter, right: f32, mid: f32, label: &str, id: &str, live: bool, pointer: Option<Pos2>) -> (f32, bool) {
    let c = ito_theme::colors();
    let g = ui.fonts_mut(|f| f.layout_no_wrap(format!("? {label}"), FontId::proportional(12.0), c.dim));
    let r = Rect::from_min_size(pos2(right - g.size().x, mid - g.size().y / 2.0), g.size());
    let hit = r.expand2(vec2(4.0, 4.0));
    let hot = pointer.is_some_and(|m| hit.contains(m));
    let ink = if hot { accent() } else { c.dim };
    p.galley(r.min, g, ink);
    if hot {
        p.line_segment([r.left_bottom(), r.right_bottom()], Stroke::new(1.0, ink));
    }
    (r.left(), clicked(ui, p, hit, ("guide-link", id), live, pointer))
}

/// The diagram of the Changes view (#120): changed → put in the next commit
/// → commit, with the step the files are at lit. Drawn at `top_left`, `width`
/// wide; what was clicked, and its height.
#[expect(clippy::too_many_arguments, reason = "the painter, the place and the look are the frame's")]
pub fn stage_band(
    ui: &Ui,
    app: &App,
    p: &Painter,
    top_left: Pos2,
    width: f32,
    now: Stage,
    live: bool,
    pointer: Option<Pos2>,
) -> (Option<Act>, f32) {
    let c = ito_theme::colors();
    let step = Stage::ALL.iter().position(|s| *s == now).unwrap_or(0);
    let pills: Vec<Arc<Galley>> = STAGE
        .iter()
        .map(|(word, _)| ui.fonts_mut(|f| f.layout_no_wrap(app.tr(word).to_owned(), FontId::proportional(12.5), c.fg)))
        .collect();
    let text = format!("{}\n{}", app.tr("guide.stage.body"), app.tr(STAGE[step].1));
    let body = wrapped(ui, &text, FontId::proportional(12.0), c.dim, width - 2.0 * PAD - 24.0);
    let h = 8.0 + PILL_H + 6.0 + body.size().y + 10.0;
    let band = Rect::from_min_size(top_left, vec2(width, h));
    p.rect_filled(band, 0.0, c.panel);
    p.line_segment([band.left_bottom() - vec2(0.0, 0.5), band.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
    let mid = band.top() + 8.0 + PILL_H / 2.0;
    let mut x = band.left() + PAD;
    for (k, g) in pills.into_iter().enumerate() {
        let r = Rect::from_min_size(pos2(x, mid - PILL_H / 2.0), vec2(g.size().x + 20.0, PILL_H));
        if k == step {
            p.rect_filled(r, CornerRadius::same(12), accent().gamma_multiply(0.2));
            p.rect_stroke(r, CornerRadius::same(12), Stroke::new(1.5, accent()), StrokeKind::Inside);
        } else {
            p.rect_stroke(r, CornerRadius::same(12), Stroke::new(1.0, c.border), StrokeKind::Inside);
        }
        let ink = if k == step { c.fg } else { c.dim };
        p.galley(pos2(r.left() + 10.0, mid - g.size().y / 2.0), g, ink);
        x = r.right();
        if k < 2 {
            p.text(pos2(x + ARROW_W / 2.0, mid), Align2::CENTER_CENTER, "→", FontId::proportional(14.0), c.dim);
            x += ARROW_W;
        }
    }
    p.galley(pos2(band.left() + PAD, mid + PILL_H / 2.0 + 6.0), body, c.dim);
    let act = cross(ui, p, pos2(band.right() - 6.0, band.top() + 6.0), "stage", live, pointer).then_some(Act::Dismiss);
    (act, h)
}

/// The title and the body of a card laid out, and the card's height.
fn measure(ui: &Ui, title: &str, body: &str, width: f32) -> (Arc<Galley>, Arc<Galley>, f32) {
    let c = ito_theme::colors();
    let tg = wrapped(ui, title, FontId::proportional(13.0), c.fg, width - 2.0 * PAD - 110.0);
    let bg = wrapped(ui, body, FontId::proportional(12.0), c.dim, width - 2.0 * PAD);
    let h = 10.0 + tg.size().y.max(20.0) + 4.0 + bg.size().y + 10.0;
    (tg, bg, h)
}

/// The height the worktree card takes in `width` (the view lays out before it draws).
pub fn worktree_h(ui: &Ui, app: &App, width: f32) -> f32 {
    measure(ui, app.tr("guide.worktree.title"), app.tr("guide.worktree.body"), width).2
}

/// A card of words with a dismiss cross, and for the conflict's a pager: the
/// `card`th of the conflict cards (`None` for the worktree's single card).
/// Drawn at `top_left`, `width` wide; what was clicked, and its height.
#[expect(clippy::too_many_arguments, reason = "the painter, the place and the look are the frame's")]
pub fn card(
    ui: &Ui,
    p: &Painter,
    top_left: Pos2,
    width: f32,
    title: &str,
    body: &str,
    pager: Option<usize>,
    id: &str,
    live: bool,
    pointer: Option<Pos2>,
) -> (Option<Act>, f32) {
    let c = ito_theme::colors();
    let (tg, bg, h) = measure(ui, title, body, width);
    let r = Rect::from_min_size(top_left, vec2(width, h));
    p.rect_filled(r, 0.0, c.panel);
    p.line_segment([r.left_bottom() - vec2(0.0, 0.5), r.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
    p.galley(pos2(r.left() + PAD, r.top() + 10.0), tg, c.fg);
    let mut act = cross(ui, p, pos2(r.right() - 6.0, r.top() + 8.0), id, live, pointer).then_some(Act::Dismiss);
    if let Some(at) = pager {
        let y = r.top() + 8.0 + 11.0;
        let x = r.right() - 6.0 - 22.0 - 8.0;
        let step = |label: &str, right: f32, on: bool, key: &str| {
            let hit = Rect::from_min_size(pos2(right - 20.0, y - 11.0), vec2(20.0, 22.0));
            let ink = if on { c.fg } else { c.border };
            p.text(hit.center(), Align2::CENTER_CENTER, label, FontId::proportional(15.0), ink);
            on && clicked(ui, p, hit, ("guide-page", id, key), live, pointer)
        };
        if step("›", x, at + 1 < CONFLICT_CARDS, "next") {
            act = Some(Act::Next);
        }
        let n =
            p.text(pos2(x - 22.0, y), Align2::RIGHT_CENTER, format!("{}/{}", at + 1, CONFLICT_CARDS), FontId::proportional(11.5), c.dim);
        if step("‹", n.left() - 4.0, at > 0, "prev") {
            act = Some(Act::Prev);
        }
    }
    p.galley(pos2(r.left() + PAD, r.bottom() - 10.0 - bg.size().y), bg, c.dim);
    (act, h)
}

/// The `at`th card of "what is a conflict" (#121).
#[expect(clippy::too_many_arguments, reason = "the painter, the place and the look are the frame's")]
pub fn conflict_card(
    ui: &Ui,
    app: &App,
    p: &Painter,
    top_left: Pos2,
    width: f32,
    at: usize,
    live: bool,
    pointer: Option<Pos2>,
) -> (Option<Act>, f32) {
    let (title, body) = CONFLICT[at.min(CONFLICT_CARDS - 1)];
    card(ui, p, top_left, width, app.tr(title), app.tr(body), Some(at), "conflict", live, pointer)
}

/// "What a worktree is" (#122), four lines.
pub fn worktree_card(ui: &Ui, app: &App, p: &Painter, top_left: Pos2, width: f32, live: bool, pointer: Option<Pos2>) -> (Option<Act>, f32) {
    card(ui, p, top_left, width, app.tr("guide.worktree.title"), app.tr("guide.worktree.body"), None, "worktree", live, pointer)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every word the explanations use is in both tables.
    #[test]
    fn the_words_are_in_both_languages() {
        let mut keys = vec!["guide.stage.body", "guide.worktree.title", "guide.worktree.body", "guide.again", "guide.link.stage"];
        keys.extend(["guide.link.conflict", "guide.link.worktree", "guide.restored"]);
        for (a, b) in CONFLICT.iter().chain(STAGE.iter()) {
            keys.extend([*a, *b]);
        }
        for lang in ["en", "ja"] {
            for k in &keys {
                assert_ne!(crate::i18n::tr(lang, k), *k, "{lang} lacks `{k}`");
            }
        }
    }

    /// The worktree card is four lines (#122).
    #[test]
    fn the_worktree_card_is_four_lines() {
        for lang in ["en", "ja"] {
            assert_eq!(crate::i18n::tr(lang, "guide.worktree.body").lines().count(), 4, "{lang}");
        }
    }
}
