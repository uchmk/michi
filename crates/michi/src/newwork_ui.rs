//! Draws the new-work form (newwork.rs) at the top of the window: the branch,
//! where it starts from, what the Claude is asked, how many, and the call to
//! tsumugi that starts them.

use eframe::egui::{self, pos2, Id, RichText, Stroke};
use michi_core::newwork;

use crate::app::App;

const WIDTH: f32 = 620.0;

pub fn branch_id() -> Id {
    Id::new("michi-newwork-branch")
}

pub fn show(ctx: &egui::Context, app: &mut App) {
    let Some(mut n) = app.new_work.take() else { return };
    let opening = std::mem::take(&mut n.opening);
    let c = ito_theme::colors();
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 32.0);
    let mut start = false;
    let area = egui::Area::new(Id::new("michi-newwork"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(screen.center().x - width / 2.0 - 8.0, screen.top() + 40.0))
        .show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.border)).corner_radius(6.0).inner_margin(10.0).show(ui, |ui| {
                ui.set_width(width);
                ui.label(RichText::new(app.tr("new.title")).strong());
                ui.add_space(4.0);
                ui.label(RichText::new(app.tr("new.branch")).color(c.dim));
                let edit = ui.add(egui::TextEdit::singleline(&mut n.branch).id(branch_id()).desired_width(f32::INFINITY));
                if opening {
                    edit.request_focus();
                }
                ui.add_space(4.0);
                ui.label(RichText::new(app.tr("new.from")).color(c.dim));
                ui.horizontal_wrapped(|ui| {
                    for (k, (label, _)) in n.froms.iter().enumerate() {
                        if ui.selectable_label(n.from == k, label).clicked() {
                            n.from = k;
                        }
                    }
                });
                ui.add_space(4.0);
                ui.label(RichText::new(app.tr("new.text")).color(c.dim));
                ui.add(egui::TextEdit::multiline(&mut n.text).desired_rows(3).desired_width(f32::INFINITY));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(app.tr("new.count")).color(c.dim));
                    if ui.button("−").clicked() {
                        n.count = n.count.saturating_sub(1).max(1);
                    }
                    ui.label(n.count.to_string());
                    if ui.button("+").clicked() {
                        n.count = (n.count + 1).min(newwork::MAX);
                    }
                });
                ui.add_space(4.0);
                let plans = n.plans();
                if let Some(first) = plans.first() {
                    let folder = newwork::folder(&n.repo, &first.branch);
                    ui.label(RichText::new(format!("{}: {folder}", app.tr("new.folder"))).size(12.0).color(c.dim));
                    ui.label(RichText::new(app.tr("new.run")).size(12.0).color(c.dim));
                    ui.label(RichText::new(newwork::command_line(&n.repo, first)).monospace().size(12.0));
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(app.tr("new.keys")).size(12.0).color(c.dim));
                    if ui.button(app.tr("new.start")).clicked() {
                        start = true;
                    }
                });
            });
        });
    let outside = !opening
        && ctx.input(|i| i.pointer.any_pressed())
        && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|q| !area.response.rect.contains(q));
    if outside {
        return;
    }
    app.new_work = Some(n);
    if start {
        app.new_work_start();
    }
}
