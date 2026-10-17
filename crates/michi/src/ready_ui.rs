//! Draws the pre-merge checklist (ready.rs) at the top of the window: one row
//! per item with its OK / !! / ?? mark and the reason. It steps aside while the
//! band asks; a click outside closes it.

use eframe::egui::{self, pos2, Id, RichText, Stroke};
use michi_core::ready::Status;

use crate::app::App;
use crate::ready::mark;

const WIDTH: f32 = 720.0;

pub fn show(ctx: &egui::Context, app: &mut App) {
    if app.band.is_some() {
        return;
    }
    let Some(r) = app.ready.as_ref() else { return };
    let c = ito_theme::colors();
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 32.0);
    let (slide, _) = crate::motion::overlay(ctx, app.prefs.motion, Id::new("michi-ready"), -1.0);
    let title = app.tr("ready.title").replace("{branch}", &r.branch).replace("{trunk}", &r.trunk);
    let area = egui::Area::new(Id::new("michi-ready"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(screen.center().x - width / 2.0 - 8.0, screen.top() + 40.0 + slide))
        .show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.border)).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(width);
                let small = egui::FontId::proportional(12.0);
                ui.label(RichText::new(title).strong());
                ui.label(RichText::new(app.tr("ready.sub")).font(small.clone()).color(c.dim));
                ui.add_space(4.0);
                for f in &r.facts {
                    let s = f.status();
                    let ink = match s {
                        Status::Ok => c.done,
                        Status::Warn => c.wait,
                        Status::Unknown => c.dim,
                    };
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(mark(s)).monospace().strong().color(ink));
                        ui.label(RichText::new(app.ready_item(f.item())).color(c.dim));
                        ui.label(app.ready_text(f));
                    });
                }
                ui.add_space(4.0);
                ui.label(RichText::new(app.tr("ready.keys")).font(small).color(c.dim));
            });
        });
    let outside =
        ctx.input(|i| i.pointer.any_pressed()) && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|q| !area.response.rect.contains(q));
    if outside {
        app.ready = None;
    }
}
