//! Draws the first-run guide (#116) at the top of the window: what is missing
//! (git, the name and email git signs commits with), and the commands to copy.
//! It takes no keys, so work goes on under it; Esc or a click outside closes
//! it, and the command palette brings it back. Signing in to the remote is the
//! `K` help's job, which the guide only points to.

use eframe::egui::{self, pos2, Id, RichText, Sense, Stroke};
use michi_core::setup::{Os, Problem};

use crate::app::App;

const WIDTH: f32 = 640.0;

/// The guide while it is open: a click on a command copies it.
pub fn show(ctx: &egui::Context, app: &mut App) {
    if !app.guide_open || app.problems.is_empty() || app.band.is_some() {
        return;
    }
    let c = ito_theme::colors();
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 32.0);
    let os = Os::current();
    let mut copy: Option<String> = None;
    let area = egui::Area::new(Id::new("michi-guide"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(screen.center().x - width / 2.0 - 8.0, screen.top() + 40.0))
        .show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.border)).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(width);
                let small = egui::FontId::proportional(12.0);
                ui.label(RichText::new(app.tr("guide.title")).strong());
                ui.label(RichText::new(app.tr("guide.lead")).font(small.clone()).color(c.dim));
                for p in &app.problems {
                    ui.add_space(6.0);
                    let line = match p {
                        Problem::OldGit(v) => {
                            app.tr(p.word()).replace("{v}", &v.to_string()).replace("{min}", &michi_core::git::MIN.to_string())
                        }
                        _ => app.tr(p.word()).to_owned(),
                    };
                    ui.label(RichText::new(line).color(c.wait));
                    for step in p.steps(os) {
                        let resp = ui.add(egui::Label::new(RichText::new(&step.command).monospace()).sense(Sense::click()));
                        ui.label(RichText::new(app.tr(step.why)).font(small.clone()).color(c.dim));
                        if resp.clicked() {
                            copy = Some(step.command);
                        }
                    }
                }
                ui.add_space(6.0);
                ui.label(RichText::new(app.tr("guide.auth")).font(small.clone()).color(c.dim));
                ui.label(RichText::new(app.tr("guide.keys")).font(small).color(c.dim));
            });
        });
    let outside = copy.is_none()
        && ctx.input(|i| i.pointer.any_pressed())
        && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|q| !area.response.rect.contains(q));
    if outside {
        app.guide_open = false;
    }
    if let Some(command) = copy {
        app.guide_copy(command);
    }
}

impl App {
    /// A command of the guide was clicked: put it on the clipboard.
    pub(crate) fn guide_copy(&mut self, command: String) {
        let text = self.tr("auth.copied").replace("{}", &command);
        self.clipboard = Some(command);
        self.say(text);
    }
}

#[cfg(test)]
mod tests {
    use crate::app::tests::{app, toast};
    use crate::repo::Answer;
    use michi_core::setup::Problem;

    #[test]
    fn the_guide_opens_when_something_is_wrong_and_not_otherwise() {
        let mut a = app();
        a.take(Answer::Problems(vec![]));
        assert!(!a.guide_open);
        a.take(Answer::Problems(vec![Problem::NoName, Problem::NoEmail]));
        assert!(a.guide_open);
        assert_eq!(a.problems.len(), 2);
    }

    #[test]
    fn the_guide_takes_no_keys_and_esc_closes_it() {
        let mut a = app();
        a.take(Answer::Problems(vec![Problem::NoGit]));
        let view = a.view;
        a.feed('3');
        assert_ne!(a.view, view, "keys still work under the guide");
        assert!(a.guide_open);
        assert!(a.escape());
        assert!(!a.guide_open);
    }

    #[test]
    fn a_click_on_a_command_copies_it() {
        let mut a = app();
        a.guide_copy("git config --global user.name \"X\"".into());
        assert_eq!(a.clipboard.as_deref(), Some("git config --global user.name \"X\""));
        assert!(toast(&a).contains("user.name"));
    }
}
