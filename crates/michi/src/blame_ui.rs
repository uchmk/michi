//! Draws the blame screen (blame.rs) at the top of the window, the way the
//! reflog list is drawn: rows that follow the selection, its keys at the
//! foot. A blame row is the commit, who, when, the line's number and the
//! line; a history row is the commit, who, when and its subject.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use eframe::egui::{self, pos2, vec2, FontId, Id, Rect, RichText, Sense, Stroke};

use crate::app::App;
use crate::blame::{Blame, Tab};
use crate::work::ago;

const ROWS: usize = 22;
const WIDTH: f32 = 980.0;
const ROW_H: f32 = 20.0;
const ID_W: f32 = 64.0;
const WHO_W: f32 = 120.0;
const WHEN_W: f32 = 84.0;
const NO_W: f32 = 40.0;

/// The screen while it is open: a click on a row selects it (a second one goes
/// to its commit), a click outside closes it.
pub fn show(ctx: &egui::Context, app: &mut App) {
    if app.band.is_some() {
        return;
    }
    let Some(mut b) = app.blame.take() else { return };
    let c = ito_theme::colors();
    let mut clicked = None;
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 32.0);
    let (slide, _) = crate::motion::overlay(ctx, app.prefs.motion, Id::new("michi-blame"), -1.0);
    let area = egui::Area::new(Id::new("michi-blame"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(screen.center().x - width / 2.0 - 8.0, screen.top() + 40.0 + slide))
        .show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.border)).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(width);
                let small = FontId::proportional(12.0);
                let title = match b.tab {
                    Tab::Blame => "blame.title_blame",
                    Tab::History => "blame.title_history",
                };
                ui.label(RichText::new(app.tr(title).replace("{path}", &b.path)).strong());
                let sub = match (b.tab, b.rev) {
                    (Tab::History, _) => app.tr("blame.sub_history").to_owned(),
                    (Tab::Blame, Some(id)) => app.tr("blame.sub_at").replace("{id}", &id.short()),
                    (Tab::Blame, None) => app.tr("blame.sub_tree").to_owned(),
                };
                ui.label(RichText::new(sub).font(small.clone()).color(c.dim));
                ui.add_space(4.0);
                if b.data.is_none() {
                    ui.label(RichText::new(app.tr("blame.reading")).color(c.dim));
                }
                let n = b.len();
                let sel = b.sel();
                let first = sel.saturating_sub(ROWS - 1).min(n.saturating_sub(ROWS));
                for k in first..n.min(first + ROWS) {
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), app.prefs.density.px(ROW_H)), Sense::click());
                    row(ui, app, &b, k, rect);
                    if resp.clicked() {
                        clicked = Some(k);
                    }
                    if resp.hovered() && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
                        let slot = if b.tab == Tab::Blame { 0 } else { 1 };
                        b.sel[slot] = k;
                    }
                }
                if n > ROWS {
                    ui.label(RichText::new(format!("{} / {n}", sel + 1)).font(small.clone()).color(c.dim));
                }
                ui.add_space(4.0);
                ui.label(RichText::new(app.tr("blame.keys")).font(small).color(c.dim));
            });
        });
    let outside = clicked.is_none()
        && ctx.input(|i| i.pointer.any_pressed())
        && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|q| !area.response.rect.contains(q));
    if outside {
        return;
    }
    let go = clicked.is_some_and(|k| k == b.sel());
    if let Some(k) = clicked {
        let slot = if b.tab == Tab::Blame { 0 } else { 1 };
        b.sel[slot] = k;
    }
    app.blame = Some(b);
    if go {
        app.blame_enter();
    }
}

/// Row `k` of the tab in front. A line not committed yet is tinted, and so
/// is the selection.
fn row(ui: &egui::Ui, app: &App, b: &Blame, k: usize, rect: Rect) {
    let c = ito_theme::colors();
    let Some(d) = &b.data else { return };
    let painter = ui.painter_at(rect);
    if k == b.sel() {
        painter.rect_filled(rect, 3.0, c.blue.gamma_multiply(0.22));
    }
    let mono = egui::TextStyle::Monospace.resolve(ui.style());
    let body = egui::TextStyle::Body.resolve(ui.style());
    let mid = rect.center().y;
    let now = SystemTime::now();
    let x = std::cell::Cell::new(rect.left() + 8.0);
    let put = |text: String, font: &FontId, ink: egui::Color32, width: f32| {
        let mut job = egui::text::LayoutJob::default();
        job.append(&text, 0.0, egui::TextFormat::simple(font.clone(), ink));
        job.wrap = egui::text::TextWrapping::truncate_at_width((width - 6.0).max(1.0));
        let g = painter.layout_job(job);
        painter.galley(pos2(x.get(), mid - g.size().y / 2.0), g, ink);
        x.set(x.get() + width);
    };
    match b.tab {
        Tab::Blame => {
            let l = &d.lines[k];
            let tint = if l.uncommitted { c.wait } else { c.dim };
            let id = if l.uncommitted { "-------".to_owned() } else { l.commit.short() };
            put(id, &mono, tint, ID_W + 8.0);
            let who = if l.uncommitted { app.tr("blame.not_committed").to_owned() } else { l.author.clone() };
            put(who, &body, tint, WHO_W);
            let when = if l.uncommitted { String::new() } else { ago(app, UNIX_EPOCH + Duration::from_secs(l.when.max(0) as u64), now) };
            put(when, &body, c.dim, WHEN_W);
            put((k + 1).to_string(), &mono, c.dim, NO_W);
            put(l.text.replace('\t', "    "), &mono, c.fg, (rect.right() - x.get() - 8.0).max(40.0));
        }
        Tab::History => {
            let h = &d.history[k];
            put(h.id.short(), &mono, c.dim, ID_W + 8.0);
            put(h.author.clone(), &body, c.dim, WHO_W);
            put(ago(app, UNIX_EPOCH + Duration::from_secs(h.when.max(0) as u64), now), &body, c.dim, WHEN_W);
            put(h.subject.clone(), &body, c.fg, (rect.right() - x.get() - 8.0).max(40.0));
        }
    }
}
