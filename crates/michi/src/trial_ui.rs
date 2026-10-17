//! Draws the trial-merge screen (trial.rs) at the top of the window: the
//! worktrees to pick with their place in the order, and below them what the
//! last run found (where each step clashes, what the end changes).

use eframe::egui::{self, pos2, text::LayoutJob, text::TextWrapping, vec2, Id, Rect, RichText, Sense, Stroke, TextFormat};

use crate::app::App;
use crate::trial::TrialScreen;

const ROWS: usize = 10;
const WIDTH: f32 = 720.0;
const ROW_H: f32 = 22.0;
const SHOWN: usize = 12;

/// The screen while it is open: a click on a row picks it, a click outside
/// closes it. It steps aside while the band asks.
pub fn show(ctx: &egui::Context, app: &mut App) {
    if app.band.is_some() {
        return;
    }
    let Some(mut t) = app.trial.take() else { return };
    let c = ito_theme::colors();
    let mut clicked = None;
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 32.0);
    let (slide, _) = crate::motion::overlay(ctx, app.prefs.motion, Id::new("michi-trial"), -1.0);
    let area = egui::Area::new(Id::new("michi-trial"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(screen.center().x - width / 2.0 - 8.0, screen.top() + 40.0 + slide))
        .show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.border)).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(width);
                let small = egui::FontId::proportional(12.0);
                ui.label(RichText::new(app.tr("trial.title").replace("{trunk}", &t.trunk)).strong());
                ui.label(RichText::new(app.tr("trial.sub")).font(small.clone()).color(c.dim));
                ui.add_space(4.0);
                let n = t.rows.len();
                let first = t.sel.saturating_sub(ROWS - 1).min(n.saturating_sub(ROWS));
                for k in first..n.min(first + ROWS) {
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click());
                    row(ui, &t, k, rect);
                    if resp.clicked() {
                        clicked = Some(k);
                    }
                }
                ui.add_space(4.0);
                if t.running {
                    ui.label(RichText::new(app.tr("trial.running")).font(small.clone()).color(c.dim));
                } else if let Some(r) = &t.result {
                    let line = app
                        .tr("trial.summary")
                        .replace("{steps}", &r.steps.len().to_string())
                        .replace("{clash}", &r.clashing().to_string())
                        .replace("{places}", &r.places().to_string())
                        .replace("{files}", &r.changed.len().to_string())
                        .replace("{add}", &r.lines().0.to_string())
                        .replace("{del}", &r.lines().1.to_string());
                    let ink = if r.clashing() > 0 { c.wait } else { c.fg };
                    ui.label(RichText::new(line).color(ink));
                    for (i, s) in r.steps.iter().enumerate() {
                        let text = match (&s.conflict, s.already) {
                            (Some(cf), _) => {
                                let files: Vec<&str> = cf.files.iter().map(|f| f.path.as_str()).collect();
                                format!("{}. {}  {}  {}", i + 1, s.name, app.tr("trial.clash"), files.join(", "))
                            }
                            (None, true) => format!("{}. {}  {}", i + 1, s.name, app.tr("trial.already")),
                            (None, false) => format!("{}. {}  {}", i + 1, s.name, app.tr("trial.clean")),
                        };
                        let ink = if s.conflict.is_some() { c.wait } else { c.dim };
                        ui.add(egui::Label::new(RichText::new(text).font(small.clone()).color(ink)).truncate());
                    }
                    for ch in r.changed.iter().take(SHOWN) {
                        let stat = match ch.lines {
                            Some((a, d)) => format!("+{a} -{d}"),
                            None => app.tr("trial.binary").to_owned(),
                        };
                        ui.add(egui::Label::new(RichText::new(format!("{stat}  {}", ch.path)).font(small.clone()).color(c.fg)).truncate());
                    }
                    if r.changed.len() > SHOWN {
                        ui.label(RichText::new(format!("… {}", r.changed.len() - SHOWN)).font(small.clone()).color(c.dim));
                    }
                }
                ui.add_space(4.0);
                ui.label(RichText::new(app.tr("trial.keys")).font(small).color(c.dim));
            });
        });
    let outside = clicked.is_none()
        && ctx.input(|i| i.pointer.any_pressed())
        && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|q| !area.response.rect.contains(q));
    if outside {
        return;
    }
    if let Some(k) = clicked {
        t.sel = k;
        t.toggle(k);
    }
    app.trial = Some(t);
}

/// Row `k`: its place in the order (or blank), the branch and the folder.
fn row(ui: &egui::Ui, t: &TrialScreen, k: usize, rect: Rect) {
    let c = ito_theme::colors();
    let r = &t.rows[k];
    let painter = ui.painter_at(rect);
    if k == t.sel {
        painter.rect_filled(rect, 3.0, c.blue.gamma_multiply(0.22));
    }
    let font = egui::TextStyle::Body.resolve(ui.style());
    let mid = rect.center().y;
    let place = t.place(k);
    let mark = place.map_or_else(|| "·".to_owned(), |p| p.to_string());
    let mut job = LayoutJob::default();
    job.append(&format!("{mark:>2}   "), 0.0, TextFormat::simple(font.clone(), if place.is_some() { c.blue } else { c.dim }));
    job.append(&r.branch, 0.0, TextFormat::simple(font.clone(), c.fg));
    job.append(&format!("   {}", r.folder), 0.0, TextFormat::simple(font, c.dim));
    job.wrap = TextWrapping::truncate_at_width((rect.width() - 16.0).max(rect.width() / 3.0));
    let text = painter.layout_job(job);
    painter.galley(pos2(rect.left() + 8.0, mid - text.size().y / 2.0), text, c.fg);
}
