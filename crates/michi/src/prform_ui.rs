//! Draws the pull request form (prform.rs) at the top of the window.

use eframe::egui::{self, pos2, Id, RichText, Stroke};

use crate::app::App;

const WIDTH: f32 = 620.0;

pub fn title_id() -> Id {
    Id::new("michi-pr-title")
}

pub fn show(ctx: &egui::Context, app: &mut App) {
    let Some(mut f) = app.pr_form.take() else { return };
    let opening = std::mem::take(&mut f.opening);
    let c = ito_theme::colors();
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 32.0);
    let (mut go, mut cancel, mut close, mut open, mut copy) = (false, false, false, false, false);
    let (slide, _) = crate::motion::overlay(ctx, app.prefs.motion, Id::new("michi-pr"), -1.0);
    let area = egui::Area::new(Id::new("michi-pr"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(screen.center().x - width / 2.0 - 8.0, screen.top() + 40.0 + slide))
        .show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.border)).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                ui.set_width(width);
                ui.label(RichText::new(app.tr(if f.edit.is_some() { "pr.update_title" } else { "pr.title" })).strong());
                ui.label(RichText::new(format!("{}: {}", app.tr("pr.branch"), f.branch)).size(12.0).color(c.dim));
                ui.add_space(4.0);
                if let Some(url) = f.done.clone() {
                    ui.label(RichText::new(&url).monospace());
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        open = ui.button(app.tr("pr.open")).clicked();
                        copy = ui.button(app.tr("pr.copy")).clicked();
                        close = ui.button(app.tr("pr.close")).clicked();
                    });
                    return;
                }
                let edit = !f.busy;
                ui.label(RichText::new(app.tr("pr.name")).color(c.dim));
                let t = ui.add_enabled(edit, egui::TextEdit::singleline(&mut f.title).id(title_id()).desired_width(f32::INFINITY));
                if opening {
                    t.request_focus();
                }
                ui.add_space(4.0);
                ui.label(RichText::new(app.tr("pr.body")).color(c.dim));
                ui.add_enabled(edit, egui::TextEdit::multiline(&mut f.body).desired_rows(6).desired_width(f32::INFINITY));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(app.tr("pr.base")).color(c.dim));
                    ui.add_enabled(edit, egui::TextEdit::singleline(&mut f.base).desired_width(160.0));
                    ui.add_enabled(edit, egui::Checkbox::new(&mut f.draft, app.tr("pr.draft")));
                });
                if let Some(remote) = &f.push_to {
                    ui.add_space(4.0);
                    let key = if f.confirm { "pr.push_ask" } else { "pr.push_note" };
                    let text = app.tr(key).replace("{branch}", &f.branch).replace("{remote}", remote);
                    ui.label(RichText::new(format!("⚠ {text}")).size(12.0).color(c.wait));
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if f.busy {
                        ui.label(RichText::new(app.tr("pr.making")).size(12.0).color(c.dim));
                        return;
                    }
                    ui.label(RichText::new(app.tr("pr.keys")).size(12.0).color(c.dim));
                    let word = match (f.edit.is_some(), f.push_to.is_some()) {
                        (true, _) => "pr.update",
                        (false, true) => "pr.push_create",
                        (false, false) => "pr.create",
                    };
                    go = ui.button(app.tr(word)).clicked();
                    if f.confirm {
                        cancel = ui.button(app.tr("pr.cancel")).clicked();
                    }
                });
            });
        });
    let outside = !opening
        && ctx.input(|i| i.pointer.any_pressed())
        && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|q| !area.response.rect.contains(q))
        && !f.busy;
    if outside || close {
        return;
    }
    if cancel {
        f.confirm = false;
    }
    if open {
        if let Some(u) = &f.done {
            let _ = michi_core::pr::open_url(u);
        }
    }
    if copy {
        app.clipboard = f.done.clone();
        app.say(app.tr("pr.copied"));
    }
    app.pr_form = Some(f);
    if go {
        app.pr_submit();
    }
}
