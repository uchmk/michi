//! Draws the statistics screen (stats.rs) at the top of the window, the way the
//! reflog list is drawn: a row of bars for the commits per week or month, then
//! the contributors with a bar each, the keys at the foot. The bars are plain
//! rectangles from the painter.

use eframe::egui::{self, pos2, vec2, Id, Rect, RichText, Sense, Stroke};
use michi_core::stats::{self, Grain, Range};

use crate::app::App;
use crate::stats::Statistics;

const ROWS: usize = 12;
const WIDTH: f32 = 760.0;
const ROW_H: f32 = 22.0;
const CHART_H: f32 = 84.0;
const BAR_W: f32 = 9.0;
const BAR_GAP: f32 = 2.0;
const NAME_W: f32 = 190.0;
const COUNT_W: f32 = 150.0;

/// The screen while it is open: a click on a contributor selects it, a click
/// outside closes it.
pub fn show(ctx: &egui::Context, app: &mut App) {
    if app.band.is_some() {
        return;
    }
    let Some(mut st) = app.stats.take() else { return };
    let c = ito_theme::colors();
    let mut clicked = None;
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 32.0);
    let (slide, _) = crate::motion::overlay(ctx, app.prefs.motion, Id::new("michi-stats"), -1.0);
    let area = egui::Area::new(Id::new("michi-stats"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(screen.center().x - width / 2.0 - 8.0, screen.top() + 40.0 + slide))
        .show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.border)).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(width);
                let small = egui::FontId::proportional(12.0);
                let grain = app.tr(if st.grain == Grain::Week { "stats.grain_week" } else { "stats.grain_month" });
                ui.label(RichText::new(app.tr("stats.title").replace("{grain}", grain)).strong());
                let range = app.tr(match st.range {
                    Range::All => "stats.range_all",
                    Range::Recent => "stats.range_recent",
                    Range::Branch => "stats.range_branch",
                });
                let total = st.data.as_ref().map_or(0, |d| d.total);
                ui.label(
                    RichText::new(app.tr("stats.sub").replace("{range}", range).replace("{total}", &total.to_string()))
                        .font(small.clone())
                        .color(c.dim),
                );
                ui.add_space(4.0);
                match &st.data {
                    None => _ = ui.label(RichText::new(app.tr("stats.reading")).color(c.dim)),
                    Some(d) if d.total == 0 => _ = ui.label(RichText::new(app.tr("stats.nothing")).color(c.dim)),
                    Some(_) => {
                        chart(ui, app, &st);
                        ui.add_space(6.0);
                        ui.label(RichText::new(app.tr("stats.contributors")).font(small.clone()).color(c.dim));
                        let n = st.len();
                        let first = st.sel.saturating_sub(ROWS - 1).min(n.saturating_sub(ROWS));
                        for k in first..n.min(first + ROWS) {
                            let (rect, resp) =
                                ui.allocate_exact_size(vec2(ui.available_width(), app.prefs.density.px(ROW_H)), Sense::click());
                            row(ui, app, &st, k, rect);
                            if resp.clicked() {
                                clicked = Some(k);
                            }
                            if resp.hovered() && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
                                st.sel = k;
                            }
                        }
                        if n > ROWS {
                            ui.label(RichText::new(format!("{} / {n}", st.sel + 1)).font(small.clone()).color(c.dim));
                        }
                    }
                }
                ui.add_space(4.0);
                ui.label(RichText::new(app.tr("stats.keys")).font(small).color(c.dim));
            });
        });
    let outside = clicked.is_none()
        && ctx.input(|i| i.pointer.any_pressed())
        && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|q| !area.response.rect.contains(q));
    if outside {
        return;
    }
    if let Some(k) = clicked {
        st.sel = k;
    }
    app.stats = Some(st);
}

/// The commits per week or month as bars, oldest at the left; as many of the
/// newest as the width holds. The first and last names sit under the ends.
fn chart(ui: &mut egui::Ui, app: &App, st: &Statistics) {
    let c = ito_theme::colors();
    let Some(d) = &st.data else { return };
    let buckets = d.buckets(st.grain);
    let fit = (((ui.available_width() - 8.0) / (BAR_W + BAR_GAP)) as usize).max(1);
    let shown = &buckets[buckets.len().saturating_sub(fit)..];
    let max = shown.iter().map(|b| b.commits).max().unwrap_or(0).max(1);
    let grain = app.tr(if st.grain == Grain::Week { "stats.grain_week" } else { "stats.grain_month" });
    let small = egui::FontId::proportional(12.0);
    ui.label(
        RichText::new(app.tr("stats.activity").replace("{grain}", grain).replace("{max}", &max.to_string()))
            .font(small.clone())
            .color(c.dim),
    );
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), CHART_H), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.line_segment([pos2(rect.left(), rect.bottom() - 0.5), pos2(rect.right(), rect.bottom() - 0.5)], Stroke::new(1.0, c.border));
    // Right-aligned, so the newest bar is always at the edge.
    let x0 = rect.right() - 4.0 - shown.len() as f32 * (BAR_W + BAR_GAP);
    let mut hovered = None;
    for (i, b) in shown.iter().enumerate() {
        let x = x0 + i as f32 * (BAR_W + BAR_GAP);
        let h = (CHART_H - 2.0) * b.commits as f32 / max as f32;
        let bar = Rect::from_min_max(pos2(x, rect.bottom() - 1.0 - h), pos2(x + BAR_W, rect.bottom() - 1.0));
        if b.commits > 0 {
            painter.rect_filled(bar, 1.0, c.blue);
        }
        if resp.hover_pos().is_some_and(|p| p.x >= x - BAR_GAP / 2.0 && p.x < x + BAR_W + BAR_GAP / 2.0) {
            hovered = Some(*b);
        }
    }
    if let (Some(first), Some(last)) = (shown.first(), shown.last()) {
        let y = rect.top() + 2.0;
        let ink = c.dim;
        let left = painter.layout_no_wrap(stats::label(st.grain, first.index), small.clone(), ink);
        let right = painter.layout_no_wrap(stats::label(st.grain, last.index), small.clone(), ink);
        painter.galley(pos2(rect.left() + 2.0, y), left, ink);
        painter.galley(pos2(rect.right() - 2.0 - right.size().x, y), right, ink);
    }
    if let Some(b) = hovered {
        resp.on_hover_text(format!("{}: {}", stats::label(st.grain, b.index), b.commits));
    }
}

/// Contributor `k`: name, a bar scaled to the most active one, then the count
/// and share, and the days they were active.
fn row(ui: &egui::Ui, app: &App, st: &Statistics, k: usize, rect: Rect) {
    let c = ito_theme::colors();
    let Some(d) = &st.data else { return };
    let a = &d.authors[k];
    let painter = ui.painter_at(rect);
    if k == st.sel {
        painter.rect_filled(rect, 3.0, c.blue.gamma_multiply(0.22));
    }
    let font = egui::TextStyle::Body.resolve(ui.style());
    let mid = rect.center().y;
    let left = rect.left() + 8.0;
    let mut name = egui::text::LayoutJob::default();
    name.append(&a.name, 0.0, egui::TextFormat::simple(font.clone(), c.fg));
    name.wrap = egui::text::TextWrapping::truncate_at_width(NAME_W - 8.0);
    let name = painter.layout_job(name);
    painter.galley(pos2(left, mid - name.size().y / 2.0), name, c.fg);
    let most = d.authors.first().map_or(1, |a| a.commits).max(1);
    let bar_left = left + NAME_W;
    let bar_room = (rect.right() - COUNT_W - bar_left - 8.0).max(20.0);
    let w = (bar_room * a.commits as f32 / most as f32).max(2.0);
    painter.rect_filled(Rect::from_min_size(pos2(bar_left, mid - 5.0), vec2(w, 10.0)), 2.0, c.blue.gamma_multiply(0.8));
    let pct = (a.commits * 100 + d.total / 2) / d.total.max(1);
    let count = app.tr("stats.n_commits").replace("{n}", &a.commits.to_string()).replace("{pct}", &pct.to_string());
    let count = painter.layout_no_wrap(count, font.clone(), c.dim);
    painter.galley(pos2(rect.right() - 6.0 - count.size().x, mid - count.size().y / 2.0), count, c.dim);
    if k == st.sel {
        let active = app.tr("stats.active").replace("{first}", &stats::date(a.first)).replace("{last}", &stats::date(a.last));
        let g = painter.layout_no_wrap(active, egui::FontId::proportional(11.0), c.dim);
        let x = bar_left + w + 6.0;
        if x + g.size().x < rect.right() - COUNT_W {
            painter.galley(pos2(x, mid - g.size().y / 2.0), g, c.dim);
        }
    }
}
