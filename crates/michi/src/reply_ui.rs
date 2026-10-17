//! Draws the answer line (reply.rs) at the bottom of the window, over the
//! branch review, so that the diff stays in sight above it.

use eframe::egui::{self, pos2, Align2, Id, RichText, Stroke};

use crate::app::App;

const WIDTH: f32 = 720.0;

pub fn edit_id() -> Id {
    Id::new("michi-reply")
}

pub fn show(ctx: &egui::Context, app: &mut App) {
    let Some(mut r) = app.reply.take() else { return };
    let opening = std::mem::take(&mut r.opening);
    let c = ito_theme::colors();
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 32.0);
    let title = app.tr("reply.title").replace("{wt}", &crate::act::name(&r.wt)).replace("{n}", &r.session.to_string());
    let question = if r.question.is_empty() { app.tr("reply.blank").to_owned() } else { r.question.clone() };
    let hint = app.tr("reply.hint").to_owned();
    let (slide, _) = crate::motion::overlay(ctx, app.prefs.motion, Id::new("michi-reply-area"), 1.0);
    egui::Area::new(Id::new("michi-reply-area"))
        .order(egui::Order::Foreground)
        .pivot(Align2::CENTER_BOTTOM)
        .fixed_pos(pos2(screen.center().x, screen.bottom() - 40.0 + slide))
        .show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.wait)).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                ui.set_width(width);
                ui.label(RichText::new(title).strong().color(c.wait));
                ui.add_space(2.0);
                ui.label(RichText::new(question).color(c.fg));
                ui.add_space(4.0);
                let edit = ui.add(egui::TextEdit::singleline(&mut r.text).id(edit_id()).desired_width(f32::INFINITY));
                if opening || !edit.has_focus() {
                    edit.request_focus();
                }
                ui.add_space(2.0);
                ui.label(RichText::new(hint).size(12.0).color(c.dim));
            });
        });
    app.reply = Some(r);
}
