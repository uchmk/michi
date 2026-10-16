//! Draws the rebase screen (reorder.rs) at the top of the window, the way the
//! clean-up list is drawn: the commits oldest first with what each does, a
//! box for the message being written, and the keys at the foot. It steps
//! aside while the band asks.

use eframe::egui::{self, pos2, text::LayoutJob, text::TextWrapping, vec2, FontId, Id, Rect, RichText, Sense, Stroke, TextFormat, Ui};
use michi_core::rebase::Action;

use crate::act;
use crate::app::App;
use crate::reorder::Reorder;

const ROWS: usize = 12;
const WIDTH: f32 = 680.0;
const ROW_H: f32 = 22.0;

pub fn edit_id() -> Id {
    Id::new("michi-reorder-edit")
}

fn word(a: Action) -> &'static str {
    match a {
        Action::Pick => "pick",
        Action::Reword => "reword",
        Action::Squash => "squash",
        Action::Fixup => "fixup",
        Action::Drop => "drop",
    }
}

pub fn show(ctx: &egui::Context, app: &mut App) {
    if app.band.is_some() {
        return;
    }
    let Some(mut r) = app.reorder.take() else { return };
    let opening = std::mem::take(&mut r.opening);
    let c = ito_theme::colors();
    let mut clicked = None;
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 32.0);
    let area = egui::Area::new(Id::new("michi-reorder"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(screen.center().x - width / 2.0 - 8.0, screen.top() + 40.0))
        .show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.border)).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(width);
                let small = FontId::proportional(12.0);
                let title = app
                    .tr("reorder.title")
                    .replace("{wt}", &act::name(&r.plan.wt))
                    .replace("{trunk}", &r.plan.trunk)
                    .replace("{n}", &r.list.len().to_string());
                ui.label(RichText::new(title).strong());
                ui.add_space(4.0);
                let n = r.list.len();
                let first = r.sel.saturating_sub(ROWS - 1).min(n.saturating_sub(ROWS));
                for k in first..n.min(first + ROWS) {
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click());
                    row(ui, &r, k, rect);
                    if resp.clicked() {
                        clicked = Some(k);
                    }
                }
                if n > ROWS {
                    ui.label(RichText::new(format!("{} / {n}", r.sel + 1)).font(small.clone()).color(c.dim));
                }
                if let Some(text) = &mut r.editing {
                    ui.add_space(4.0);
                    let edit = ui.add(egui::TextEdit::multiline(text).id(edit_id()).desired_rows(4).desired_width(f32::INFINITY));
                    if opening || !edit.has_focus() && !edit.lost_focus() {
                        edit.request_focus();
                    }
                }
                ui.add_space(4.0);
                let keys = if r.editing.is_some() { app.tr("reorder.edit_keys") } else { app.tr("reorder.keys") };
                ui.label(RichText::new(keys).font(small).color(c.dim));
            });
        });
    let outside = !opening
        && clicked.is_none()
        && ctx.input(|i| i.pointer.any_pressed())
        && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|q| !area.response.rect.contains(q));
    if outside {
        return;
    }
    if let (Some(k), None) = (clicked, &r.editing) {
        r.sel = k;
    }
    app.reorder = Some(r);
}

/// Row `k`: what it does, its commit and its subject; a dropped one is dim,
/// a folded one is set in from the one above.
fn row(ui: &Ui, r: &Reorder, k: usize, rect: Rect) {
    let c = ito_theme::colors();
    let e = &r.list[k];
    let painter = ui.painter_at(rect);
    if k == r.sel {
        painter.rect_filled(rect, 3.0, c.blue.gamma_multiply(0.22));
    }
    let font = egui::TextStyle::Body.resolve(ui.style());
    let mono = FontId::monospace(font.size);
    let mid = rect.center().y;
    let ink = match e.action {
        Action::Drop => c.dim,
        _ => c.fg,
    };
    let tag = match e.action {
        Action::Pick => c.dim,
        Action::Drop => c.wait,
        _ => c.blue,
    };
    let w = painter.layout_no_wrap(word(e.action).to_owned(), mono.clone(), tag);
    painter.galley(pos2(rect.left() + 6.0, mid - w.size().y / 2.0), w, tag);
    let id = painter.layout_no_wrap(e.id.short(), mono, c.dim);
    let left = rect.left() + 6.0 + 64.0;
    let indent = if matches!(e.action, Action::Squash | Action::Fixup) { 14.0 } else { 0.0 };
    painter.galley(pos2(left, mid - id.size().y / 2.0), id.clone(), c.dim);
    let text_left = left + id.size().x + 8.0 + indent;
    let mut job = LayoutJob::default();
    let mut fmt = TextFormat::simple(font, ink);
    if e.action == Action::Drop {
        fmt.strikethrough = Stroke::new(1.0, c.dim);
    }
    job.append(&e.subject, 0.0, fmt);
    job.wrap = TextWrapping::truncate_at_width((rect.right() - text_left - 6.0).max(rect.width() / 3.0));
    let text = painter.layout_job(job);
    painter.galley(pos2(text_left, mid - text.size().y / 2.0), text, ink);
}
