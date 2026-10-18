//! The graph's legend (#118): what the colours, dots, lines, names and marks of
//! the graph mean, one small drawing and a line of words each. The drawings use
//! the same pieces as the graph (`graph_ui::paint_pill`, `paint_warn`, the dots
//! of ito-graph with the same radii), so what is shown here is what is drawn there.
//! Any key, Esc or a click outside closes it.

use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Id, Pos2, Rect, Sense, Shape, Stroke};

use crate::app::App;
use crate::graph::{Pill, PillKind};

const WIDTH: f32 = 560.0;
const ROW_H: f32 = 26.0;
const ICON_W: f32 = 120.0;

/// What a row of the legend draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Lanes,
    Commit,
    Merge,
    Wip,
    Conflict,
    Open,
    Branch,
    Worktree,
    Remote,
    Tag,
    Also,
    Warn,
    WarnWip,
    New,
}

/// The rows, top to bottom, with the key of the words of each.
pub const ROWS: &[(Icon, &str)] = &[
    (Icon::Lanes, "legend.lanes"),
    (Icon::Commit, "legend.commit"),
    (Icon::Merge, "legend.merge"),
    (Icon::Wip, "legend.wip"),
    (Icon::Conflict, "legend.conflict"),
    (Icon::Open, "legend.open"),
    (Icon::Branch, "legend.branch"),
    (Icon::Worktree, "legend.worktree"),
    (Icon::Remote, "legend.remote"),
    (Icon::Tag, "legend.tag"),
    (Icon::Also, "legend.also"),
    (Icon::Warn, "legend.warn"),
    (Icon::WarnWip, "legend.warn_wip"),
    (Icon::New, "legend.new"),
];

/// The legend while it is open.
pub fn show(ctx: &egui::Context, app: &mut App) {
    if !app.legend {
        return;
    }
    let c = ito_theme::colors();
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 32.0);
    let (slide, _) = crate::motion::overlay(ctx, app.prefs.motion, Id::new("michi-legend"), -1.0);
    let area = egui::Area::new(Id::new("michi-legend"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(screen.center().x - width / 2.0 - 8.0, screen.top() + 40.0 + slide))
        .show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.border)).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(width);
                ui.label(egui::RichText::new(app.tr("legend.title")).strong());
                ui.add_space(4.0);
                rows(ui, app);
                ui.add_space(4.0);
                ui.label(egui::RichText::new(app.tr("legend.keys")).font(FontId::proportional(12.0)).color(c.dim));
            });
        });
    let outside =
        ctx.input(|i| i.pointer.any_pressed()) && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|q| !area.response.rect.contains(q));
    if outside {
        app.legend = false;
    }
}

/// The rows of the legend, to be drawn in `ui` (also the sheet's last tab).
pub fn rows(ui: &mut egui::Ui, app: &App) {
    let c = ito_theme::colors();
    for (icon, key) in ROWS {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), app.prefs.density.px(ROW_H)), Sense::hover());
        let p = ui.painter_at(rect);
        draw(ui, &p, *icon, pos2(rect.left() + 6.0, rect.center().y), rect);
        p.text(pos2(rect.left() + ICON_W, rect.center().y), Align2::LEFT_CENTER, app.tr(key), FontId::proportional(13.0), c.fg);
    }
}

/// The drawing of `icon`, from `at` (the left, middle of the row).
fn draw(ui: &egui::Ui, p: &egui::Painter, icon: Icon, at: Pos2, row: Rect) {
    let c = ito_theme::colors();
    let ansi = c.ansi();
    let lane = ansi[2];
    let bg = c.panel;
    // The dot sits on a short line, as in the graph.
    let dot = pos2(at.x + 20.0, at.y);
    let line = |from: Pos2, to: Pos2, color: Color32, dashed: bool| {
        let s = Stroke::new(2.0, color);
        if dashed {
            p.extend(Shape::dashed_line(&[from, to], s, 3.0, 3.0));
        } else {
            p.line_segment([from, to], s);
        }
    };
    match icon {
        Icon::Lanes => {
            for (i, k) in [1usize, 2, 3, 4, 5].into_iter().enumerate() {
                let x = at.x + 4.0 + i as f32 * 14.0;
                line(pos2(x, row.top() + 3.0), pos2(x, row.bottom() - 3.0), ansi[k], false);
            }
        }
        Icon::Commit => {
            line(dot - vec2(0.0, 10.0), dot + vec2(0.0, 10.0), lane, false);
            p.circle(dot, 4.0, lane, Stroke::new(1.5, bg));
        }
        Icon::Merge => {
            line(dot - vec2(0.0, 10.0), dot + vec2(0.0, 10.0), lane, false);
            p.circle(dot, 5.0, bg, Stroke::new(2.0, lane));
            p.circle_filled(dot, 2.0, lane);
        }
        Icon::Wip => {
            line(dot, dot + vec2(0.0, 10.0), lane, true);
            p.circle_filled(dot, 4.5, bg);
            p.extend(Shape::dashed_line(&ring(dot, 4.5), Stroke::new(1.6, lane), 2.2, 1.8));
        }
        Icon::Conflict => {
            line(dot, dot + vec2(0.0, 10.0), lane, true);
            p.circle(dot, 5.5, bg, Stroke::new(2.0, c.err));
            p.circle_filled(dot, 2.0, c.err);
        }
        Icon::Open => {
            line(dot - vec2(0.0, 10.0), dot, lane, false);
            p.extend(Shape::dashed_line(&[dot, dot + vec2(0.0, 10.0)], Stroke::new(2.0, lane.gamma_multiply(0.4)), 2.0, 3.0));
        }
        Icon::Branch => pill(ui, p, at, PillKind::Local, "main", None),
        Icon::Worktree => pill(ui, p, at, PillKind::Worktree, "wt", None),
        Icon::Remote => pill(ui, p, at, PillKind::Remote, "origin/x", None),
        Icon::Tag => pill(ui, p, at, PillKind::Tag, "v1", None),
        Icon::Also => pill(ui, p, at, PillKind::Local, "main", Some("origin")),
        Icon::Warn | Icon::WarnWip => {
            let text = "\u{26A0}";
            let w = crate::graph_ui::warn_width(ui, text);
            let r = Rect::from_min_size(pos2(at.x, at.y - 8.5), vec2(w, 17.0));
            crate::graph_ui::paint_warn(ui, p, r, text, icon == Icon::WarnWip, 1.0);
        }
        Icon::New => _ = p.circle_filled(pos2(at.x + 3.0, at.y), 3.0, c.run),
    }
}

/// A name's pill, drawn by the graph's own code.
fn pill(ui: &egui::Ui, p: &egui::Painter, at: Pos2, kind: PillKind, text: &str, also: Option<&str>) {
    let pill = Pill { kind, text: text.to_owned(), color: 2, also: also.map(|a| vec![a.to_owned()]).unwrap_or_default(), path: None };
    let Some(w) = crate::graph_ui::pill_width(ui, text, kind, also.is_some()) else { return };
    crate::graph_ui::paint_pill(ui, p, Rect::from_min_size(pos2(at.x, at.y - 8.5), vec2(w, 17.0)), &pill, 1.0);
}

/// The points of a circle, closed, for a dashed ring.
fn ring(center: Pos2, r: f32) -> Vec<Pos2> {
    (0..=24).map(|i| i as f32 / 24.0 * std::f32::consts::TAU).map(|a| center + vec2(a.cos(), a.sin()) * r).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_has_words_in_both_languages() {
        for (_, key) in ROWS {
            for lang in ["en", "ja"] {
                assert_ne!(crate::i18n::tr(lang, key), *key, "{key} in {lang}");
            }
        }
        for key in ["legend.title", "legend.button", "legend.keys"] {
            assert_ne!(crate::i18n::tr("ja", key), key);
        }
    }

    #[test]
    fn a_row_is_listed_once() {
        let mut seen = Vec::new();
        for (icon, _) in ROWS {
            assert!(!seen.contains(icon));
            seen.push(*icon);
        }
    }
}
