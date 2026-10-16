//! Draws the branch, tag and stash palette (picker.rs) at the top of the window,
//! the way ito-palette draws the command palette: the box to type in, up to
//! ten rows with the typed characters lit, and its keys at the foot.

use eframe::egui::{self, pos2, text::LayoutJob, text::TextWrapping, vec2, FontId, Id, Rect, RichText, Sense, Stroke, TextFormat, Ui};

use crate::act;
use crate::app::App;
use crate::picker::{same, Item, Kind, Picker};
use crate::ui::accent;

const ROWS: usize = 10;
const WIDTH: f32 = 560.0;
const ROW_H: f32 = 22.0;

/// The box typed in.
pub fn query_id() -> Id {
    Id::new("michi-picker-query")
}

enum Click {
    Row(usize),
    Outside,
}

/// The palette while it is open: what is typed narrows it, a click on a
/// row is Enter on it, a click outside closes it.
pub fn show(ctx: &egui::Context, app: &mut App) {
    let Some(mut p) = app.picker.take() else { return };
    let opening = std::mem::take(&mut p.opening);
    let c = ito_theme::colors();
    let mut click = None;
    let screen = ctx.content_rect();
    let width = WIDTH.min(screen.width() - 32.0);
    let area = egui::Area::new(Id::new("michi-picker"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(screen.center().x - width / 2.0 - 8.0, screen.top() + 40.0))
        .show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.border)).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(width);
                let renaming = p.renaming.as_ref().map(|old| app.tr("picker.rename").replace("{}", old));
                let (hint, keys) = match p.kind {
                    Kind::Branch => ("picker.branch", "picker.branch_keys"),
                    Kind::Tag => ("picker.tag", "picker.tag_keys"),
                    Kind::Stash => ("picker.stash", "picker.stash_keys"),
                    Kind::Remote => ("picker.remote", "picker.remote_keys"),
                };
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut p.query)
                        .id(query_id())
                        .hint_text(renaming.as_deref().unwrap_or(app.tr(hint)))
                        .desired_width(f32::INFINITY)
                        .lock_focus(true),
                );
                // Only when it has lost it: asking again every frame ends an
                // IME composition, so Japanese could not be typed here.
                if !edit.has_focus() {
                    edit.request_focus();
                }
                p.sync();
                let small = FontId::proportional(12.0);
                let at = match p.kind {
                    Kind::Stash => app.tr("picker.stash_for").replace("{wt}", &act::name(&p.wt)),
                    Kind::Remote => app.tr("picker.remote_for").replace("{wt}", &act::name(&p.wt)),
                    _ => app.tr("picker.for").replace("{wt}", &act::name(&p.wt)).replace("{at}", &p.at.short()),
                };
                ui.add_space(2.0);
                ui.label(RichText::new(at).font(small.clone()).color(c.dim));
                ui.add_space(2.0);
                if p.rows.is_empty() {
                    ui.colored_label(c.dim, app.tr("picker.none"));
                }
                // A window of ROWS rows that follows the selection.
                let first = p.sel.saturating_sub(ROWS - 1).min(p.rows.len().saturating_sub(ROWS));
                for k in first..p.rows.len().min(first + ROWS) {
                    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_H), Sense::click());
                    row(ui, app, &p, k, rect);
                    if resp.clicked() {
                        click = Some(Click::Row(k));
                    }
                    if !opening && resp.hovered() && ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO) {
                        p.sel = k;
                    }
                }
                ui.add_space(4.0);
                ui.label(RichText::new(app.tr(keys)).font(small).color(c.dim));
            });
        });
    if click.is_none() && !opening && ctx.input(|i| i.pointer.any_pressed()) {
        let outside = ctx.input(|i| i.pointer.interact_pos()).is_some_and(|q| !area.response.rect.contains(q));
        if outside {
            click = Some(Click::Outside);
        }
    }
    app.picker = Some(p);
    match click {
        Some(Click::Row(k)) => {
            if let Some(p) = &mut app.picker {
                p.sel = k;
            }
            app.picker_enter(false);
        }
        Some(Click::Outside) => app.picker = None,
        None => {}
    }
}

/// Row `k`: its title with the typed characters lit, where the branch is
/// and how far from its upstream, and its commit at the right.
fn row(ui: &Ui, app: &App, p: &Picker, k: usize, rect: Rect) {
    let c = ito_theme::colors();
    let r = &p.rows[k];
    let painter = ui.painter_at(rect);
    if k == p.sel {
        painter.rect_filled(rect, 3.0, c.blue.gamma_multiply(0.22));
    }
    let font = egui::TextStyle::Body.resolve(ui.style());
    let mid = rect.center().y;
    let (detail, id) = describe(app, p, &r.item);
    let id = id.map(|s| painter.layout_no_wrap(s, font.clone(), c.dim));
    let detail = (!detail.is_empty()).then(|| painter.layout_no_wrap(detail, font.clone(), c.dim));
    let id_w = id.as_ref().map_or(0.0, |g| g.size().x + 12.0);
    let detail_w = detail.as_ref().map_or(0.0, |g| g.size().x + 10.0);
    let room = (rect.width() - 12.0 - id_w - detail_w).max(rect.width() / 2.0);
    // Nothing leaves the row: a long title ends in "…".
    let mut job = LayoutJob::default();
    let ink = if matches!(r.item, Item::Create { .. } | Item::Save) { accent() } else { c.fg };
    let text = match &r.item {
        Item::Save if p.message().is_empty() => app.tr("picker.save").to_owned(),
        Item::Save => app.tr("picker.save_as").replace("{}", p.message()),
        item => item.title(),
    };
    let mut run = String::new();
    let mut lit = false;
    for (n, ch) in text.chars().enumerate() {
        let hit = r.hits.contains(&n);
        if hit != lit && !run.is_empty() {
            job.append(&std::mem::take(&mut run), 0.0, TextFormat::simple(font.clone(), if lit { accent() } else { ink }));
        }
        lit = hit;
        run.push(ch);
    }
    job.append(&run, 0.0, TextFormat::simple(font.clone(), if lit { accent() } else { ink }));
    job.wrap = TextWrapping::truncate_at_width(room);
    let title = painter.layout_job(job);
    let right = rect.left() + 6.0 + title.size().x;
    painter.galley(pos2(rect.left() + 6.0, mid - title.size().y / 2.0), title, ink);
    if let Some(g) = detail {
        painter.galley(pos2(right + 10.0, mid - g.size().y / 2.0), g, c.dim);
    }
    if let Some(g) = id {
        painter.galley(pos2(rect.right() - 6.0 - g.size().x, mid - g.size().y / 2.0), g, c.dim);
    }
}

/// What a row says after its title, and its commit.
fn describe(app: &App, p: &Picker, item: &Item) -> (String, Option<String>) {
    match item {
        Item::Local { id, at, track, .. } => {
            let mut words = Vec::new();
            match at {
                Some(w) if same(w, &p.wt) => words.push(app.tr("picker.here").to_owned()),
                Some(w) => words.push(app.tr("picker.in").replace("{}", &act::name(w))),
                None => {}
            }
            if let Some((ahead, behind)) = *track {
                if ahead > 0 {
                    words.push(format!("↑{ahead}"));
                }
                if behind > 0 {
                    words.push(format!("↓{behind}"));
                }
            }
            (words.join("  "), Some(id.short()))
        }
        Item::Remote { id, .. } | Item::Tag { id, .. } => (String::new(), Some(id.short())),
        Item::Create { .. } => {
            let key = if p.kind == Kind::Branch { "picker.create" } else { "picker.tag_at" };
            (app.tr(key).replace("{}", &p.at.short()), None)
        }
        Item::Stash { id, place, .. } => (format!("stash@{{{place}}}"), Some(id.short())),
        Item::Save | Item::Target { .. } => (String::new(), None),
    }
}
