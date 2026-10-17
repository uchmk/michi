//! Draws the reflog list (reflog.rs) at the top of the window, the way the
//! clean-up list is drawn: up to twelve rows that follow the selection, its
//! keys at the foot. It steps aside while the band asks.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use eframe::egui::{self, pos2, text::LayoutJob, text::TextWrapping, vec2, Id, Rect, RichText, Sense, Stroke, TextFormat};
use michi_core::reflog::Entry;

use crate::app::App;
use crate::reflog::Reflog;
use crate::work::ago;

const ROWS: usize = 12;
const WIDTH: f32 = 720.0;
const ROW_H: f32 = 22.0;

/// The list while it is open: a click on a row selects it (a second one asks),
/// a click outside closes it.
pub fn show(ctx: &egui::Context, app: &mut App) {
    if app.band.is_some() {
        return;
    }
    let Some(mut r) = app.reflog.take() else { return };
    let c = ito_theme::colors();
    let mut clicked = None;
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 32.0);
    let area = egui::Area::new(Id::new("michi-reflog"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(screen.center().x - width / 2.0 - 8.0, screen.top() + 40.0))
        .show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.border)).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(width);
                let small = egui::FontId::proportional(12.0);
                ui.label(RichText::new(app.tr("reflog.title")).strong());
                ui.label(RichText::new(app.tr("reflog.sub")).font(small.clone()).color(c.dim));
                ui.add_space(4.0);
                let n = r.entries.len();
                let first = r.sel.saturating_sub(ROWS - 1).min(n.saturating_sub(ROWS));
                for k in first..n.min(first + ROWS) {
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click());
                    row(ui, app, &r, k, rect);
                    if resp.clicked() {
                        clicked = Some(k);
                    }
                    if resp.hovered() && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
                        r.sel = k;
                    }
                }
                if n > ROWS {
                    ui.label(RichText::new(format!("{} / {n}", r.sel + 1)).font(small.clone()).color(c.dim));
                }
                ui.add_space(4.0);
                ui.label(RichText::new(app.tr("reflog.keys")).font(small).color(c.dim));
            });
        });
    let outside = clicked.is_none()
        && ctx.input(|i| i.pointer.any_pressed())
        && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|q| !area.response.rect.contains(q));
    if outside {
        return;
    }
    let ask = clicked.is_some_and(|k| k == r.sel);
    if let Some(k) = clicked {
        r.sel = k;
    }
    app.reflog = Some(r);
    if ask {
        app.reflog_enter();
    }
}

/// Row `k`: when, its name, what happened, and the commit at the right.
/// Backups are tinted.
fn row(ui: &egui::Ui, app: &App, r: &Reflog, k: usize, rect: Rect) {
    let c = ito_theme::colors();
    let e: &Entry = &r.entries[k];
    let painter = ui.painter_at(rect);
    if k == r.sel {
        painter.rect_filled(rect, 3.0, c.blue.gamma_multiply(0.22));
    }
    let font = egui::TextStyle::Body.resolve(ui.style());
    let mid = rect.center().y;
    let when = UNIX_EPOCH + Duration::from_secs(e.when.max(0) as u64);
    let id = painter.layout_no_wrap(e.id.short(), font.clone(), c.dim);
    let left = rect.left() + 8.0;
    let label_ink = if e.is_backup() { c.wait } else { c.fg };
    let mut job = LayoutJob::default();
    job.append(&format!("{}   ", ago(app, when, SystemTime::now())), 0.0, TextFormat::simple(font.clone(), c.dim));
    job.append(&e.label(), 0.0, TextFormat::simple(font.clone(), label_ink));
    job.append(&format!("   {}", e.what), 0.0, TextFormat::simple(font.clone(), c.fg));
    job.append(&format!("   {}", e.subject), 0.0, TextFormat::simple(font, c.dim));
    // Nothing leaves the row: what does not fit ends in "…".
    job.wrap = TextWrapping::truncate_at_width((rect.right() - left - id.size().x - 20.0).max(rect.width() / 3.0));
    let text = painter.layout_job(job);
    painter.galley(pos2(left, mid - text.size().y / 2.0), text, c.fg);
    painter.galley(pos2(rect.right() - 6.0 - id.size().x, mid - id.size().y / 2.0), id, c.dim);
}
