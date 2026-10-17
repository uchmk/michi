//! Draws the clean-up list (tidy.rs) at the top of the window, the way the
//! branch palette is drawn: what it measures against, up to ten rows with a
//! box to pick each, and its keys at the foot. It steps aside while the band
//! asks.

use std::time::SystemTime;

use eframe::egui::{self, pos2, text::LayoutJob, text::TextWrapping, vec2, FontId, Id, Rect, RichText, Sense, Stroke, TextFormat, Ui};
use michi_core::tidy::{Item, Stay, Why};

use crate::act;
use crate::app::App;
use crate::tidy::Tidy;
use crate::ui::accent;
use crate::work::ago;

const ROWS: usize = 10;
const WIDTH: f32 = 640.0;
const ROW_H: f32 = 22.0;

enum Click {
    Row(usize),
    Outside,
}

/// The list while it is open: a click on a row picks it, a click outside
/// closes it.
pub fn show(ctx: &egui::Context, app: &mut App) {
    if app.band.is_some() {
        return;
    }
    let Some(mut t) = app.tidy.take() else { return };
    let c = ito_theme::colors();
    let mut click = None;
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 32.0);
    let (slide, _) = crate::motion::overlay(ctx, app.prefs.motion, Id::new("michi-tidy"), -1.0);
    let area = egui::Area::new(Id::new("michi-tidy"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(screen.center().x - width / 2.0 - 8.0, screen.top() + 40.0 + slide))
        .show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.border)).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(width);
                let small = FontId::proportional(12.0);
                let trunk = t.found.trunk.as_deref().unwrap_or("main");
                ui.label(RichText::new(app.tr("tidy.title").replace("{trunk}", trunk)).strong());
                let fetched = app.last_fetch.map_or_else(|| app.tr("tidy.never").to_owned(), |at| ago(app, at, SystemTime::now()));
                ui.label(RichText::new(app.tr("tidy.as_of").replace("{}", &fetched)).font(small.clone()).color(c.dim));
                ui.add_space(4.0);
                // A window of ROWS rows that follows the selection.
                let n = t.found.items.len();
                let first = t.sel.saturating_sub(ROWS - 1).min(n.saturating_sub(ROWS));
                for k in first..n.min(first + ROWS) {
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), app.prefs.density.px(ROW_H)), Sense::click());
                    row(ui, app, &t, k, rect);
                    if resp.clicked() {
                        click = Some(Click::Row(k));
                    }
                    if resp.hovered() && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
                        t.sel = k;
                    }
                }
                if n > ROWS {
                    ui.label(RichText::new(format!("{} / {n}", t.sel + 1)).font(small.clone()).color(c.dim));
                }
                ui.add_space(4.0);
                let keys = format!("{} · {}", app.tr("tidy.picked").replace("{}", &t.count().to_string()), app.tr("tidy.keys"));
                ui.label(RichText::new(keys).font(small).color(c.dim));
            });
        });
    if click.is_none() && ctx.input(|i| i.pointer.any_pressed()) {
        let outside = ctx.input(|i| i.pointer.interact_pos()).is_some_and(|q| !area.response.rect.contains(q));
        if outside {
            click = Some(Click::Outside);
        }
    }
    match click {
        Some(Click::Row(k)) => {
            t.sel = k;
            t.toggle(k);
            app.tidy = Some(t);
        }
        Some(Click::Outside) => {}
        None => app.tidy = Some(t),
    }
}

/// Row `k`: its box, its name, why it is listed, and why it stays or its
/// commit at the right. A row that stays is dim and has no box.
fn row(ui: &Ui, app: &App, t: &Tidy, k: usize, rect: Rect) {
    let c = ito_theme::colors();
    let item = &t.found.items[k];
    let painter = ui.painter_at(rect);
    if k == t.sel {
        painter.rect_filled(rect, 3.0, c.blue.gamma_multiply(0.22));
    }
    let font = egui::TextStyle::Body.resolve(ui.style());
    let mid = rect.center().y;
    let stays = item.stay.is_some();
    let side = 12.0;
    let tick = Rect::from_center_size(pos2(rect.left() + 6.0 + side / 2.0, mid), vec2(side, side));
    if !stays {
        if t.checked[k] {
            painter.rect_filled(tick, 2.0, accent());
            let (a, b, d) =
                (pos2(tick.left() + 2.5, mid), pos2(tick.left() + 5.0, tick.bottom() - 3.0), pos2(tick.right() - 2.5, tick.top() + 3.0));
            painter.line_segment([a, b], Stroke::new(2.0, c.panel));
            painter.line_segment([b, d], Stroke::new(2.0, c.panel));
        } else {
            painter.rect_stroke(tick, 2.0, Stroke::new(1.0, c.dim), egui::StrokeKind::Inside);
        }
    }
    let (right, right_ink) = match item.stay {
        Some(s) => (app.tr(stay_word(s)).to_owned(), c.wait),
        None => (item.tip.map_or_else(String::new, |id| id.short()), c.dim),
    };
    let right = (!right.is_empty()).then(|| painter.layout_no_wrap(right, font.clone(), right_ink));
    let right_w = right.as_ref().map_or(0.0, |g| g.size().x + 12.0);
    let left = tick.right() + 8.0;
    let ink = if stays { c.dim } else { c.fg };
    // Nothing leaves the row: what does not fit ends in "…".
    let mut job = LayoutJob::default();
    job.append(&item.name(), 0.0, TextFormat::simple(font.clone(), ink));
    job.append(&format!("   {}", describe(app, t, item)), 0.0, TextFormat::simple(font.clone(), c.dim));
    job.wrap = TextWrapping::truncate_at_width((rect.right() - left - right_w).max(rect.width() / 3.0));
    let text = painter.layout_job(job);
    painter.galley(pos2(left, mid - text.size().y / 2.0), text, ink);
    if let Some(g) = right {
        painter.galley(pos2(rect.right() - 6.0 - g.size().x, mid - g.size().y / 2.0), g, right_ink);
    }
}

/// Why the row is listed, and the worktree that goes with its branch.
fn describe(app: &App, t: &Tidy, item: &Item) -> String {
    let trunk = t.found.trunk.as_deref().unwrap_or("main");
    let mut text = match item.why {
        Why::Merged => app.tr("tidy.merged").replace("{}", trunk),
        Why::Gone => app.tr("tidy.gone").to_owned(),
        Why::Lost => app.tr("tidy.lost").to_owned(),
    };
    if let (Some(_), Some(wt)) = (&item.branch, &item.worktree) {
        let key = if item.lost { "tidy.with_lost" } else { "tidy.with" };
        text.push_str(&app.tr(key).replace("{}", &act::name(wt)));
    }
    text
}

/// The word-table key of why a row stays.
fn stay_word(s: Stay) -> &'static str {
    match s {
        Stay::Main => "tidy.stay_main",
        Stay::Here => "tidy.stay_here",
        Stay::Changes => "tidy.stay_changes",
        Stay::Locked => "tidy.stay_locked",
        Stay::Open => "tidy.stay_open",
    }
}
