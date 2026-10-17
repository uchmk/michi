//! Draws the sign-in help (auth.rs) at the top of the window: the cause, what
//! michi found about the remote, then the numbered commands to copy. It steps
//! aside while the band asks.

use eframe::egui::{self, pos2, Id, RichText, Sense, Stroke};
use michi_core::auth::{Agent, Facts, Scheme};

use crate::app::App;
use crate::auth::{cause_key, help_key};

const WIDTH: f32 = 720.0;

/// The panel while it is open: a click on a command copies it, a click
/// outside closes it.
pub fn show(ctx: &egui::Context, app: &mut App) {
    if app.band.is_some() {
        return;
    }
    let Some(a) = app.auth.take() else { return };
    let c = ito_theme::colors();
    let mut copy = None;
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 32.0);
    let (slide, _) = crate::motion::overlay(ctx, app.prefs.motion, Id::new("michi-auth"), -1.0);
    let area = egui::Area::new(Id::new("michi-auth"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(screen.center().x - width / 2.0 - 8.0, screen.top() + 40.0 + slide))
        .show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.border)).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(width);
                let small = egui::FontId::proportional(12.0);
                ui.label(RichText::new(app.tr("auth.title")).strong());
                let sub = if a.failed { app.tr(cause_key(a.kind)) } else { app.tr("auth.no_failure") };
                ui.label(RichText::new(sub).color(if a.failed { c.wait } else { c.dim }));
                ui.add_space(2.0);
                ui.label(RichText::new(app.tr(help_key(a.kind))).font(small.clone()).color(c.dim));
                ui.add_space(4.0);
                for line in fact_lines(app, &a.facts) {
                    ui.label(RichText::new(line).font(small.clone()).color(c.dim));
                }
                ui.add_space(4.0);
                for (k, fix) in a.fixes.iter().enumerate().take(9) {
                    let resp = ui
                        .horizontal(|ui| {
                            ui.label(RichText::new(format!("{}", k + 1)).color(c.blue));
                            ui.add(egui::Label::new(RichText::new(&fix.command).monospace()).sense(Sense::click()))
                        })
                        .inner;
                    ui.label(RichText::new(app.tr(fix.why)).font(small.clone()).color(c.dim));
                    if resp.clicked() {
                        copy = Some(k);
                    }
                }
                ui.add_space(4.0);
                ui.label(RichText::new(app.tr("auth.keys")).font(small).color(c.dim));
            });
        });
    let outside = copy.is_none()
        && ctx.input(|i| i.pointer.any_pressed())
        && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|q| !area.response.rect.contains(q));
    if outside {
        return;
    }
    app.auth = Some(a);
    if let Some(k) = copy {
        if let Some(d) = char::from_digit(k as u32 + 1, 10) {
            app.auth_key(d);
        }
    }
}

/// What michi found, one line each; nothing secret is among it.
fn fact_lines(app: &App, f: &Facts) -> Vec<String> {
    let mut v = Vec::new();
    let scheme = match f.scheme {
        Scheme::Ssh => "ssh",
        Scheme::Https => "https",
        Scheme::Other => "other",
        Scheme::None => "",
    };
    match &f.remote {
        Some(r) => {
            v.push(app.tr("auth.facts.remote").replace("{remote}", r).replace("{scheme}", scheme).replace("{host}", f.host_or_default()))
        }
        None if f.scheme != Scheme::None => {
            v.push(app.tr("auth.facts.address").replace("{scheme}", scheme).replace("{host}", f.host_or_default()))
        }
        None => v.push(app.tr("auth.facts.no_remote").to_owned()),
    }
    match &f.helper {
        Some(h) => v.push(app.tr("auth.facts.helper").replace("{}", h)),
        None => v.push(app.tr("auth.facts.no_helper").to_owned()),
    }
    if matches!(f.scheme, Scheme::Ssh | Scheme::None) {
        v.push(match f.agent {
            Agent::Keys(n) => app.tr("auth.agent.keys").replace("{}", &n.to_string()),
            Agent::Empty => app.tr("auth.agent.empty").to_owned(),
            Agent::Absent => app.tr("auth.agent.absent").to_owned(),
            Agent::Missing => app.tr("auth.agent.missing").to_owned(),
        });
    }
    v
}
