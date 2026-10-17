//! The graph view drawn: the tool band, the header, the rows with the graph
//! at their left, and the scrollbar with its marks — the one scrollbar in
//! michi (docs/design.md). The state is `graph.rs`'s; this only draws it and
//! turns the mouse into what the keys would do.

use egui::epaint::CubicBezierShape;
use egui::text::{LayoutJob, TextWrapping};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, EventFilter, FontId, Id, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind,
    Ui,
};
use ito_graph::ui::Look;
use ito_graph::Metrics;

use crate::app::{failed_word, App, Repo};
use crate::graph::{bot, when, Mark, PillKind};
use michi_core::clash::Outcome;
use michi_core::claude::{Session, State};
use michi_core::oid::Oid;
use michi_core::predict::Conflict;
use michi_core::wip::Wip;

/// The band of buttons and the filter over the rows.
const TOOL_H: f32 = 38.0;
/// The columns' names.
const HEAD_H: f32 = 24.0;
/// The narrowest an author, date or commit column can be dragged to.
const COL_MIN: f32 = 40.0;
const PAD: f32 = 8.0;
/// The scrollbar's width, its marks' too.
const BAR_W: f32 = 7.0;
/// Below this width the filter is shorter.
const NARROW: f32 = 900.0;
/// Below this width the rows leave out the author and the date.
const COMPACT: f32 = 560.0;

/// The filter's field, for the keys to know when it has them.
pub fn filter_id() -> Id {
    Id::new("graph-filter")
}

fn scroll_id() -> Id {
    Id::new("graph-scroll")
}

/// The graph view in `r`. `live`: the key list is not over it, so the mouse
/// works.
pub fn show(ui: &mut Ui, app: &mut App, r: Rect, live: bool) {
    let c = ito_theme::colors();
    if let Repo::Failed(e) = &app.repo {
        let p = ui.painter();
        p.text(r.center() - vec2(0.0, 10.0), Align2::CENTER_CENTER, app.tr(failed_word(e)), FontId::proportional(16.0), c.err);
        p.text(r.center() + vec2(0.0, 14.0), Align2::CENTER_CENTER, app.dir.display().to_string(), FontId::proportional(12.0), c.dim);
        return;
    }
    let tool = Rect::from_min_size(r.min, vec2(r.width(), TOOL_H));
    let head = Rect::from_min_size(tool.left_bottom(), vec2(r.width(), HEAD_H));
    let list = Rect::from_min_max(head.left_bottom(), r.max);
    tool_band(ui, app, tool, live);
    rows(ui, app, head, list, live);
}

// ── the tool band ─────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Icon {
    Fetch,
    Pull,
    Push,
    Plus,
}

struct Tool {
    icon: Option<Icon>,
    word: &'static str,
    key: char,
    prime: bool,
}

const TOOLS: [Tool; 6] = [
    Tool { icon: Some(Icon::Fetch), word: "key.fetch", key: 'f', prime: false },
    Tool { icon: Some(Icon::Pull), word: "key.pull", key: 'p', prime: false },
    Tool { icon: Some(Icon::Push), word: "key.push", key: 'P', prime: false },
    Tool { icon: None, word: "tool.branch", key: 'b', prime: false },
    Tool { icon: None, word: "key.stash", key: 's', prime: false },
    Tool { icon: Some(Icon::Plus), word: "tool.new_work", key: 'N', prime: true },
];

/// The buttons from the left (a line between the remote's and the rest),
/// the filter at the right.
fn tool_band(ui: &mut Ui, app: &mut App, r: Rect, live: bool) {
    let c = ito_theme::colors();
    let p = ui.painter().clone();
    p.line_segment([r.left_bottom() - vec2(0.0, 0.5), r.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
    let mid = r.center().y;
    // Beside a wide diff the filter takes the band, the buttons gone.
    if app.detail.wide {
        let field = Rect::from_min_max(pos2(r.left() + PAD, mid - 13.0), pos2(r.right() - PAD, mid + 13.0));
        filter(ui, app, field, live);
        return;
    }
    let fw = if r.width() < NARROW { 170.0 } else { 210.0 };
    let mut right = r.right() - PAD;
    if r.width() >= fw + 2.0 * PAD + 100.0 {
        let field = Rect::from_min_max(pos2(right - fw, mid - 13.0), pos2(right, mid + 13.0));
        filter(ui, app, field, live);
        right = field.left() - PAD;
    } else {
        app.graph.focus_filter = false;
    }
    let mut x = r.left() + PAD;
    let mut fed = None;
    for (k, t) in TOOLS.iter().enumerate() {
        if k == 3 {
            // The line between the remote's buttons and the rest.
            if x + 9.0 > right {
                break;
            }
            p.rect_filled(Rect::from_min_size(pos2(x + 4.0, mid - 8.0), vec2(1.0, 16.0)), 0.0, c.border);
            x += 9.0;
        }
        let label = ui.fonts_mut(|f| f.layout_no_wrap(app.tr(t.word).to_owned(), FontId::proportional(12.5), c.fg));
        let key = ui.fonts_mut(|f| f.layout_no_wrap(t.key.to_string(), FontId::monospace(10.5), c.dim));
        let icon_w = if t.icon.is_some() { 10.0 + 5.0 } else { 0.0 };
        let w = 8.0 + icon_w + label.size().x + 5.0 + key.size().x + 8.0;
        if x + w > right {
            break;
        }
        let b = Rect::from_min_max(pos2(x, mid - 13.0), pos2(x + w, mid + 13.0));
        let resp = ui.interact(b, ui.id().with(("tool", k)), Sense::CLICK);
        let hot = live && resp.hovered();
        // What moves a worktree's HEAD or index waits for the merge; the press says so.
        let locked = app.merging() && matches!(t.key, 'p' | 's');
        let ink = if t.prime {
            c.run
        } else if locked {
            c.dim
        } else {
            c.fg
        };
        if locked {
            resp.clone().on_hover_text(app.tr("tool.locked"));
        } else {
            resp.clone().on_hover_text(crate::keymap::tip(app.tr(t.word), Some(&t.key.to_string())));
        }
        if hot {
            p.rect_filled(b, CornerRadius::same(5), c.panel);
        }
        if t.prime {
            p.rect_stroke(b, CornerRadius::same(5), Stroke::new(1.0, c.run.gamma_multiply(0.45)), StrokeKind::Inside);
        } else if hot {
            p.rect_stroke(b, CornerRadius::same(5), Stroke::new(1.0, c.border), StrokeKind::Inside);
        }
        let mut tx = b.left() + 8.0;
        if let Some(icon) = t.icon {
            paint_icon(&p, pos2(tx + 5.0, mid), icon, ink);
            tx += icon_w;
        }
        p.galley(pos2(tx, mid - label.size().y / 2.0), label.clone(), ink);
        tx += label.size().x + 5.0;
        p.galley(pos2(tx, mid - key.size().y / 2.0), key, c.dim);
        if hot {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if live && resp.clicked() {
            fed = Some(t.key);
        }
        x = b.right() + 4.0;
    }
    if let Some(k) = fed {
        app.feed(k);
    }
}

/// A tool's icon, about 10 points across, centred on `at`.
pub(crate) fn paint_icon(p: &egui::Painter, at: Pos2, icon: Icon, ink: Color32) {
    let s = Stroke::new(1.3, ink);
    match icon {
        Icon::Fetch => {
            // Most of a circle, an arrowhead at its open end.
            let (r, from, to) = (4.5_f32, -0.35_f32, 4.6_f32);
            let points: Vec<Pos2> = (0..=20)
                .map(|k| {
                    let a = from + (to - from) * k as f32 / 20.0;
                    at + vec2(r * a.cos(), r * a.sin())
                })
                .collect();
            let tip = at + vec2(r * from.cos(), r * from.sin());
            p.add(Shape::line(points, s));
            p.line_segment([tip, tip + vec2(-0.5, -3.0)], s);
            p.line_segment([tip, tip + vec2(-3.0, 0.5)], s);
        }
        Icon::Pull | Icon::Push => {
            let dir = if icon == Icon::Pull { 1.0 } else { -1.0 };
            let tip = at + vec2(0.0, 5.0 * dir);
            p.line_segment([at - vec2(0.0, 5.0 * dir), tip], s);
            p.line_segment([tip, tip + vec2(-3.5, -3.5 * dir)], s);
            p.line_segment([tip, tip + vec2(3.5, -3.5 * dir)], s);
        }
        Icon::Plus => {
            p.line_segment([at - vec2(4.5, 0.0), at + vec2(4.5, 0.0)], s);
            p.line_segment([at - vec2(0.0, 4.5), at + vec2(0.0, 4.5)], s);
        }
    }
}

/// The filter: `/`, the field, and how many it hits.
fn filter(ui: &mut Ui, app: &mut App, field: Rect, live: bool) {
    let c = ito_theme::colors();
    let p = ui.painter().clone();
    let focused = ui.memory(|m| m.has_focus(filter_id()));
    p.rect_filled(field, CornerRadius::same(5), c.panel);
    let border = if focused { c.border_strong() } else { c.border };
    p.rect_stroke(field, CornerRadius::same(5), Stroke::new(1.0, border), StrokeKind::Inside);
    let mid = field.center().y;
    let slash = p.text(pos2(field.left() + 8.0, mid), Align2::LEFT_CENTER, "/", FontId::monospace(11.0), c.dim);
    let mut right = field.right() - 8.0;
    if let Some(n) = app.graph.hit_count() {
        let text = app.tr("filter.count").replace("{}", &n.to_string());
        let t = p.text(pos2(right, mid), Align2::RIGHT_CENTER, text, FontId::proportional(11.0), c.dim);
        right = t.left() - 6.0;
    }
    // One row high, in the middle: `put` stretches the edit to its rect
    // and the text keeps to the top.
    let font = FontId::proportional(12.5);
    let line = ui.ctx().fonts_mut(|f| f.row_height(&font));
    let edit_rect =
        Rect::from_min_max(pos2(slash.right() + 6.0, mid - line / 2.0), pos2(right.max(slash.right() + 20.0), mid + line / 2.0));
    let mut text = app.graph.filter().to_owned();
    let edit = egui::TextEdit::singleline(&mut text)
        .id(filter_id())
        .frame(egui::Frame::NONE)
        .margin(egui::Margin::ZERO)
        .hint_text(app.tr("filter.hint"))
        .font(font)
        .text_color(c.fg)
        .desired_width(edit_rect.width())
        .interactive(live)
        .return_key(None::<egui::KeyboardShortcut>)
        .event_filter(EventFilter { tab: false, horizontal_arrows: true, vertical_arrows: true, escape: false });
    let resp = ui.put(edit_rect, edit);
    if resp.changed() {
        app.graph.set_filter(&text);
    }
    if app.graph.focus_filter {
        app.graph.focus_filter = false;
        resp.request_focus();
    }
}

// ── the rows ──────────────────────────────────────────────────

/// What the mouse did to the rows, done once they are drawn.
enum Act {
    /// A row clicked: selected where it is.
    Select(u32),
    /// A mark on the scrollbar clicked: selected and brought into sight.
    Mark(u32),
    /// A row clicked with Ctrl (Cmd): marked to compare the next with.
    Pick(u32),
    /// A merge's chip clicked.
    Toggle(u32),
    /// The row at the foot clicked.
    More,
    /// The header's toggle clicked.
    Fold,
}

fn rows(ui: &mut Ui, app: &mut App, head: Rect, list: Rect, live: bool) {
    let c = ito_theme::colors();
    let m = Metrics { row: app.prefs.density.px(Metrics::default().row), ..Metrics::default() };
    let row_h = m.row;
    let ctx = ui.ctx().clone();
    let mut scroll = ctx.data(|d| d.get_temp::<f32>(scroll_id())).unwrap_or(0.0);
    let view = list.height().max(0.0);
    app.graph.page = ((view / row_h).floor() as u32).max(1);

    let g = &app.graph;
    if g.is_empty() {
        let (text, color) = match g.error() {
            Some(e) => (app.tr("history.failed").replace("{}", &e.to_string()), c.err),
            None if g.done() => (app.tr("history.empty").to_owned(), c.dim),
            None => (app.tr("history.reading").to_owned(), c.dim),
        };
        ui.painter().text(list.center() - vec2(0.0, HEAD_H), Align2::CENTER_CENTER, text, FontId::proportional(13.0), color);
        return;
    }
    let tail = u32::from(g.started() && !g.done());
    let len = g.len();
    let total = (len + tail) as f32 * row_h;

    // The rows take the clicks the chips, the toggle and the bar leave:
    // those are registered after them.
    let list_resp = ui.interact(list, Id::new("graph-list"), Sense::CLICK);
    if live && ui.rect_contains_pointer(list) {
        scroll -= ui.input(|i| i.smooth_scroll_delta.y);
    }
    if app.graph.reveal {
        if let Some(k) = app.graph.selected_row() {
            scroll = reveal(scroll, view, row_h, k);
        }
        app.graph.reveal = false;
    }
    scroll = scroll.clamp(0.0, (total - view).max(0.0));

    let first = (scroll / row_h).floor() as u32;
    let last = (((scroll + view) / row_h).ceil() as u32).min(len);
    app.reach(last.saturating_sub(1));
    app.graph.lay_out_to(last);

    // Beside a wide diff, or narrow, the hash is the one column left.
    let compact = app.detail.wide || list.width() < COMPACT;
    let g = &app.graph;
    let lanes = g.layout().width_in(first..last).max(2);
    let gw = m.width(lanes);
    let right = list.right() - BAR_W - 4.0;
    let (author_w, date_w, hash_w) = (app.sizes.author, app.sizes.date, app.sizes.hash);
    let hash_x = right - hash_w;
    let (date_x, author_x) = if compact { (hash_x, hash_x) } else { (hash_x - date_w, hash_x - date_w - author_w) };
    let subj_x = list.left() + gw;
    let top = list.top() - scroll;
    let row_rect = |k: u32| Rect::from_min_size(pos2(list.left(), top + k as f32 * row_h), vec2(list.width(), row_h));
    let pointer = if live { ui.input(|i| i.pointer.hover_pos()).filter(|p| list.contains(*p)) } else { None };
    let hovered = pointer.map(|p| ((p.y - top) / row_h).floor()).filter(|k| *k >= 0.0).map(|k| k as u32);

    let p = ui.painter_at(list);
    let mut acts = Vec::new();
    let selected = g.selected_row();
    let marked = g.marked_row();
    for k in first..last {
        let rr = row_rect(k);
        if Some(k) == selected {
            p.rect_filled(rr, 0.0, c.selection());
        } else if Some(k) == marked {
            // The other end of a comparison.
            p.rect_filled(rr, 0.0, c.selection());
            p.rect_stroke(rr.shrink(0.5), 0.0, Stroke::new(1.0, c.run), egui::StrokeKind::Inside);
        } else if Some(k) == hovered && rr.right() > 0.0 {
            p.rect_filled(rr, 0.0, c.fg.gamma_multiply(0.04));
        }
    }
    // A folded merge's stub: a short line from below and to the right into
    // its dot, where the folded side came from.
    let ansi = c.ansi();
    for k in first..last {
        let (Some(chip), Some(at)) = (g.chip(k), g.placed(k)) else { continue };
        if chip.open {
            continue;
        }
        let dot = pos2(list.left() + m.x(at.lane), top + m.y(k));
        let ink = chip.color.map_or(c.dim, |i| ansi[i as usize & 15]).gamma_multiply(0.8);
        let from = dot + vec2(m.lane * 0.8, row_h * 0.42);
        let curve = [from, pos2(from.x, dot.y + 2.0), dot + vec2(4.0, 0.0), dot];
        p.add(CubicBezierShape::from_points_stroke(curve, false, Color32::TRANSPARENT, Stroke::new(2.0, ink)));
    }
    let look = Look { ansi, bg: c.bg, conflict: c.err, width: 2.0 };
    ito_graph::ui::paint(&p, pos2(list.left(), top), g.layout(), first..last, m, &look);

    let now = chrono::Local::now();
    let yesterday = app.tr("time.yesterday");
    let unseen = app.unseen.ids();
    for k in first..len.min(last) {
        if let Some(w) = g.wip(k) {
            let mid = row_rect(k).center().y;
            let dim = c.dim;
            wip_cell(ui, app, &p, w, pos2(subj_x, mid), author_x - PAD - subj_x);
            if !compact {
                clipped(ui, &p, pos2(author_x, mid), "—", FontId::proportional(12.0), dim, author_w - PAD);
                let date = w.written.map_or("—".to_owned(), |t| when(t, &now, yesterday));
                clipped(ui, &p, pos2(date_x, mid), &date, FontId::proportional(12.0), dim, date_w - PAD);
            }
            clipped(ui, &p, pos2(hash_x, mid), if w.theirs.is_some() { "MERGE" } else { "WIP" }, FontId::monospace(11.5), dim, hash_w);
            continue;
        }
        let Some(commit) = g.commit(k) else { continue };
        let rr = row_rect(k);
        let mid = rr.center().y;
        let fade = if g.dimmed(k) { 0.3 } else { 1.0 };
        let fg = c.fg.gamma_multiply(fade);
        let dim = c.dim.gamma_multiply(fade);

        // The subject cell: the dot of a commit not seen yet, the names, the
        // chip, the subject (in bold when not seen).
        let mut x = subj_x;
        let cell_end = author_x - PAD;
        let new = unseen.contains(&commit.id);
        if new {
            p.circle_filled(pos2(x + 3.0, mid), 3.0, c.run.gamma_multiply(fade));
            x += 11.0;
        }
        for pill in g.pills(k) {
            let Some(w) = pill_width(ui, &pill.text, pill.kind, !pill.also.is_empty()) else { continue };
            if x + w > cell_end - 60.0 {
                break;
            }
            let pr = Rect::from_min_size(pos2(x, mid - 8.5), vec2(w, 17.0));
            paint_pill(ui, &p, pr, &pill, fade);
            if let Some(path) = &pill.path {
                let tip = format!("worktree {path}");
                ui.interact(pr, Id::new(("graph-pill", k, &pill.text)), Sense::hover()).on_hover_text(tip);
            }
            x = pr.right() + 5.0;
        }
        if let Some(tip) = head_clashes(app, commit.id) {
            let text = app.tr("clash.pill");
            let w = warn_width(ui, text);
            if x + w <= cell_end - 60.0 {
                let pr = Rect::from_min_size(pos2(x, mid - 8.5), vec2(w, 17.0));
                paint_warn(ui, &p, pr, text, false, fade);
                ui.interact(pr, Id::new(("graph-clash", k)), Sense::hover()).on_hover_text(tip);
                x = pr.right() + 5.0;
            }
        }
        if let Some(chip) = g.chip(k) {
            let digits = ui.fonts_mut(|f| f.layout_no_wrap(chip.hidden.to_string(), FontId::monospace(10.5), c.dim));
            let w = 6.0 + 8.0 + 3.0 + digits.size().x + 6.0;
            if x + w <= cell_end - 40.0 {
                let cr = Rect::from_min_size(pos2(x, mid - 8.5), vec2(w, 17.0));
                let resp = ui.interact(cr, Id::new(("graph-chip", k)), Sense::CLICK);
                let hot = live && resp.hovered();
                let ink = chip.color.map_or(c.dim, |i| ansi[i as usize & 15]).gamma_multiply(fade);
                if chip.open {
                    let dash = ink.gamma_multiply(0.6);
                    let corners = [cr.left_top(), cr.right_top(), cr.right_bottom(), cr.left_bottom(), cr.left_top()];
                    p.extend(Shape::dashed_line(&corners, Stroke::new(1.0, dash), 3.0, 2.0));
                } else {
                    p.rect_filled(cr, CornerRadius::same(9), ink.gamma_multiply(if hot { 0.26 } else { 0.14 }));
                }
                caret(&p, pos2(cr.left() + 10.0, mid), chip.open, ink);
                p.galley(pos2(cr.left() + 17.0, mid - digits.size().y / 2.0), digits, ink);
                if hot {
                    ctx.set_cursor_icon(CursorIcon::PointingHand);
                }
                if live && resp.clicked() {
                    acts.push(Act::Toggle(k));
                }
                let tip = if chip.open {
                    app.tr("fold.open").to_owned()
                } else {
                    let mut t = app.tr("fold.chip").replace("{}", &chip.hidden.to_string());
                    if !chip.names.is_empty() {
                        t.push('\n');
                        t.push_str(&app.tr("fold.names").replace("{}", &chip.names.join(", ")));
                    }
                    t
                };
                resp.on_hover_text(tip);
                x = cr.right() + 6.0;
            }
        }
        clipped(ui, &p, pos2(x, mid), &commit.subject, FontId::proportional(13.0), fg, cell_end - x);
        if new {
            // No bold face: the subject is drawn twice, a hair apart.
            clipped(ui, &p, pos2(x + 0.45, mid), &commit.subject, FontId::proportional(13.0), fg, cell_end - x);
        }

        // The author (a bot by its short name), the date, the hash.
        if !compact {
            let author = bot(&commit.author);
            let shown = author.as_deref().unwrap_or(&commit.author);
            clipped(ui, &p, pos2(author_x, mid), shown, FontId::proportional(12.0), dim, author_w - PAD);
            if author.is_some() {
                let ar = Rect::from_min_max(pos2(author_x, rr.top()), pos2(date_x - PAD, rr.bottom()));
                ui.interact(ar, Id::new(("graph-bot", k)), Sense::hover()).on_hover_text(&commit.author);
            }
            let date = when(commit.time, &now, yesterday);
            clipped(ui, &p, pos2(date_x, mid), &date, FontId::proportional(12.0), dim, date_w - PAD);
        }
        clipped(ui, &p, pos2(hash_x, mid), &commit.id.short(), FontId::monospace(11.5), dim, hash_w);
    }
    // The row at the foot: older commits to read.
    if tail == 1 && last >= len {
        let rr = row_rect(len);
        let (text, color) = if g.asking() {
            (app.tr("more.reading").to_owned(), c.dim)
        } else if let Some(e) = g.error() {
            (app.tr("history.failed").replace("{}", &e.to_string()), c.err)
        } else if let Some(n) = g.remaining() {
            (app.tr("more.count").replace("{}", &n.to_string()), c.dim)
        } else {
            (app.tr("more.unknown").to_owned(), c.dim)
        };
        let resp = ui.interact(rr.intersect(list), Id::new("graph-more"), Sense::CLICK);
        let hot = live && resp.hovered() && !g.asking();
        p.text(pos2(subj_x, rr.center().y), Align2::LEFT_CENTER, text, FontId::proportional(12.0), if hot { c.run } else { color });
        if hot {
            ctx.set_cursor_icon(CursorIcon::PointingHand);
        }
        if live && resp.clicked() {
            acts.push(Act::More);
        }
    }

    // The header, over the rows.
    let hp = ui.painter();
    hp.rect_filled(head, 0.0, c.bg);
    hp.line_segment([head.left_bottom() - vec2(0.0, 0.5), head.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
    let hm = head.center().y;
    let font = FontId::proportional(11.0);
    hp.text(pos2(subj_x, hm), Align2::LEFT_CENTER, app.tr("col.subject"), font.clone(), c.dim);
    if !compact {
        hp.text(pos2(author_x, hm), Align2::LEFT_CENTER, app.tr("col.author"), font.clone(), c.dim);
        hp.text(pos2(date_x, hm), Align2::LEFT_CENTER, app.tr("col.date"), font.clone(), c.dim);
    }
    hp.text(pos2(hash_x, hm), Align2::LEFT_CENTER, app.tr("col.commit"), font, c.dim);
    if let Some(resp) = fold_toggle(ui, app, pos2(author_x - PAD, hm), subj_x + 80.0, live) {
        if live && resp.clicked() {
            acts.push(Act::Fold);
        }
    }

    // The left edge of the author, date and commit columns drags (ito-layout's
    // grips): the column gets wider as the edge goes left.
    let which: Vec<usize> = if compact { vec![2] } else { vec![0, 1, 2] };
    let xs = [author_x, date_x, hash_x];
    let edges: Vec<f32> = which.iter().map(|n| xs[*n] - 2.0).collect();
    if let Some(g) = ito_layout::ui::grips(ui, Id::new("graph-cols"), &edges, head.top()..head.bottom(), 8.0).filter(|_| live) {
        let n = which[g.index];
        let mut w = [app.sizes.author, app.sizes.date, app.sizes.hash];
        if g.reset {
            let d = crate::sizes::Sizes::default();
            w[n] = [d.author, d.date, d.hash][n];
        } else {
            ito_layout::columns::resize(&mut w, n, g.delta, COL_MIN, 300.0, ito_layout::columns::Edge::Left);
        }
        (app.sizes.author, app.sizes.date, app.sizes.hash) = (w[0], w[1], w[2]);
        if g.released || g.reset {
            app.sizes.save();
        }
    }

    // The scrollbar, when the rows do not all fit.
    if total > view + 1.0 {
        if let Some(act) = scrollbar(ui, app, list, total, view, &mut scroll, live) {
            acts.push(act);
        }
    }

    if live && list_resp.clicked() {
        if let Some(k) = list_resp.interact_pointer_pos().map(|p| ((p.y - top) / row_h).floor()) {
            if k >= 0.0 && (k as u32) < len {
                let pick = ui.input(|i| i.modifiers.command);
                acts.insert(0, if pick { Act::Pick(k as u32) } else { Act::Select(k as u32) });
            }
        }
    }
    ctx.data_mut(|d| d.insert_temp(scroll_id(), scroll));
    let repaint = !acts.is_empty();
    for act in acts {
        match act {
            Act::Select(k) => {
                app.graph.select(k);
                app.graph.reveal = false;
            }
            Act::Mark(k) => app.graph.select(k),
            Act::Pick(k) => {
                app.graph.mark_to(k);
                app.graph.reveal = false;
            }
            Act::Toggle(k) => app.graph.toggle_group(k),
            Act::More => app.more(),
            Act::Fold => app.fold(),
        }
    }
    if repaint {
        ctx.request_repaint();
    }
}

/// The scroll that brings row `k` into sight: centred when it is out of
/// sight, else kept a little off the edges.
pub fn reveal(scroll: f32, view: f32, row: f32, k: u32) -> f32 {
    let top = k as f32 * row;
    let bottom = top + row;
    if bottom <= scroll || top >= scroll + view {
        return top + row / 2.0 - view / 2.0;
    }
    let margin = (2.0 * row).min(((view - row) / 2.0).max(0.0));
    if top < scroll + margin {
        top - margin
    } else if bottom > scroll + view - margin {
        bottom + margin - view
    } else {
        scroll
    }
}

/// The header's "Fold PRs" toggle, right-aligned to `right_center`; `None`
/// when it would run left of `left`.
fn fold_toggle(ui: &mut Ui, app: &App, right_center: Pos2, left: f32, live: bool) -> Option<Response> {
    let c = ito_theme::colors();
    let on = app.graph.fold_on();
    let label = ui.fonts_mut(|f| f.layout_no_wrap(app.tr("fold.label").to_owned(), FontId::proportional(11.0), c.dim));
    let key = ui.fonts_mut(|f| f.layout_no_wrap("z".to_owned(), FontId::monospace(10.0), c.dim));
    let w = 7.0 + label.size().x + 4.0 + key.size().x + 7.0;
    let r = Rect::from_min_max(pos2(right_center.x - w, right_center.y - 9.0), pos2(right_center.x, right_center.y + 9.0));
    if r.left() < left {
        return None;
    }
    let resp = ui.interact(r, Id::new("graph-fold"), Sense::CLICK);
    let hot = live && resp.hovered();
    let p = ui.painter();
    let (ink, border) = if on { (c.run, c.run.gamma_multiply(0.4)) } else { (c.dim, if hot { c.run } else { c.border }) };
    if on {
        p.rect_filled(r, CornerRadius::same(9), c.run.gamma_multiply(0.12));
    }
    p.rect_stroke(r, CornerRadius::same(9), Stroke::new(1.0, if hot { c.run } else { border }), StrokeKind::Inside);
    p.galley(pos2(r.left() + 7.0, right_center.y - label.size().y / 2.0), label.clone(), ink);
    p.galley(pos2(r.left() + 7.0 + label.size().x + 4.0, right_center.y - key.size().y / 2.0), key, ink.gamma_multiply(0.8));
    if hot {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    Some(resp.on_hover_text(app.tr("fold.tip")))
}

/// The scrollbar at the right of `list`: the marks, then the thumb. A click
/// on a mark selects its row; elsewhere it jumps there; a drag scrolls.
fn scrollbar(ui: &mut Ui, app: &App, list: Rect, total: f32, view: f32, scroll: &mut f32, live: bool) -> Option<Act> {
    let c = ito_theme::colors();
    let g = &app.graph;
    let row_h = app.prefs.density.px(Metrics::default().row);
    let bar = Rect::from_min_max(pos2(list.right() - 1.0 - BAR_W, list.top()), pos2(list.right() - 1.0, list.bottom() - 2.0));
    let bh = bar.height().max(1.0);
    let resp = ui.interact(bar, Id::new("graph-bar"), Sense::CLICK | Sense::DRAG);
    let p = ui.painter();
    let ansi = c.ansi();
    let mark_y = |k: u32| bar.top() + (k as f32 + 0.5) * row_h / total * bh;
    let selected = g.selected_row();
    // One mark per pixel row: the first for it wins.
    let mut last_px = f32::NEG_INFINITY;
    for &(k, mark) in g.marks() {
        let y = mark_y(k).round();
        if y == last_px || Some(k) == selected {
            continue;
        }
        last_px = y;
        let ink = match mark {
            Mark::Hit => c.fg,
            Mark::Tag | Mark::Wip => c.wait,
            Mark::Head(i) => ansi[i as usize & 15],
        };
        p.rect_filled(Rect::from_min_size(pos2(bar.left(), y - 1.5), vec2(BAR_W, 3.0)), CornerRadius::same(1), ink);
    }
    if let Some(k) = selected {
        p.rect_filled(Rect::from_min_size(pos2(bar.left(), mark_y(k).round() - 1.5), vec2(BAR_W, 3.0)), CornerRadius::same(1), c.run);
    }
    // The thumb, over the marks.
    let hot = live && (resp.hovered() || resp.dragged());
    let thumb_h = (view / total * bh).max(18.0).min(bh);
    let thumb_top = bar.top() + *scroll / total * bh;
    let (w, x_right, alpha) = if hot { (6.0, list.right(), 0.45) } else { (4.0, list.right() - 1.0, 0.26) };
    let thumb = Rect::from_min_max(pos2(x_right - w, thumb_top), pos2(x_right, (thumb_top + thumb_h).min(bar.bottom())));
    p.rect_filled(thumb, CornerRadius::same(2), c.fg.gamma_multiply(alpha));
    if !live {
        return None;
    }
    // The mark nearest the pointer, within a few pixels.
    let near = |y: f32| -> Option<(u32, Option<Mark>)> {
        let mut best: Option<(f32, u32, Option<Mark>)> = None;
        let mut consider = |k: u32, m: Option<Mark>| {
            let d = (mark_y(k) - y).abs();
            if d <= 4.0 && best.is_none_or(|(bd, _, _)| d < bd) {
                best = Some((d, k, m));
            }
        };
        let marks = g.marks();
        // The rows a few pixels around `y`.
        let lo = (((y - 4.0 - bar.top()) / bh * total / row_h) - 0.5).floor().max(0.0) as u32;
        let from = marks.partition_point(|(k, _)| *k < lo);
        for &(k, m) in &marks[from..] {
            if mark_y(k) > y + 4.0 {
                break;
            }
            consider(k, Some(m));
        }
        if let Some(k) = selected {
            consider(k, None);
        }
        best.map(|(_, k, m)| (k, m))
    };
    if let Some(pos) = resp.hover_pos() {
        if let Some((k, mark)) = near(pos.y) {
            let tip = mark_tip(app, k, mark);
            ui.interact(
                Rect::from_center_size(pos2(bar.center().x, mark_y(k)), vec2(BAR_W + 2.0, 9.0)),
                Id::new("graph-mark"),
                Sense::hover(),
            )
            .on_hover_text(tip);
        }
    }
    if resp.clicked() {
        let pos = resp.interact_pointer_pos()?;
        if let Some((k, _)) = near(pos.y) {
            return Some(Act::Mark(k));
        }
        *scroll = ((pos.y - bar.top()) / bh * total - view / 2.0).clamp(0.0, (total - view).max(0.0));
        ui.ctx().request_repaint();
    } else if resp.dragged() || resp.drag_stopped() {
        // From where the press was, not by the deltas: the first move, before
        // egui calls it a drag, would be lost, and so would the last, which
        // comes with the release.
        let grab = Id::new("graph-bar-grab");
        if resp.drag_started() {
            let origin = ui.input(|i| i.pointer.press_origin()).map_or(thumb.center().y, |o| o.y);
            // Pressed off the thumb: as if the thumb had jumped there first.
            let from = if thumb.y_range().contains(origin) { *scroll } else { (origin - bar.top()) / bh * total - view / 2.0 };
            ui.ctx().data_mut(|d| d.insert_temp(grab, (origin, from)));
        }
        let pressed = ui.ctx().data(|d| d.get_temp::<(f32, f32)>(grab));
        if let (Some((origin, from)), Some(pos)) = (pressed, resp.interact_pointer_pos()) {
            *scroll = (from + (pos.y - origin) * total / bh).clamp(0.0, (total - view).max(0.0));
            ui.ctx().request_repaint();
        }
        if resp.drag_stopped() {
            ui.ctx().data_mut(|d| d.remove_temp::<(f32, f32)>(grab));
        }
    }
    None
}

/// What a mark on the scrollbar is, for its tooltip.
fn mark_tip(app: &App, k: u32, mark: Option<Mark>) -> String {
    let g = &app.graph;
    let subject = g.commit(k).map_or("", |c| c.subject.as_str());
    match mark {
        None => format!("{} — {subject}", app.tr("mark.selected")),
        Some(Mark::Hit) => app.tr("mark.hit").replace("{}", subject),
        Some(Mark::Tag) => {
            let tags: Vec<String> = g.pills(k).into_iter().filter(|p| p.kind == PillKind::Tag).map(|p| p.text).collect();
            app.tr("mark.tag").replace("{}", &tags.join(", "))
        }
        Some(Mark::Head(_)) => {
            let names: Vec<String> = g.pills(k).into_iter().filter(|p| p.kind == PillKind::Worktree).map(|p| p.text).collect();
            format!("worktree {} — {subject}", names.join(", "))
        }
        Some(Mark::Wip) => format!("WIP {}", g.wip(k).map_or(String::new(), wip_name)),
    }
}

/// The name a worktree's uncommitted change goes by: its branch, else the
/// commit it is on.
pub(crate) fn wip_name(w: &Wip) -> String {
    w.branch.clone().unwrap_or_else(|| w.head.short())
}

/// How a Claude session's state is shown: its glyph, its word and its colour;
/// `None` for a word michi does not know.
pub(crate) fn claude_look(app: &App, s: &Session) -> Option<(&'static str, &'static str, Color32)> {
    let c = ito_theme::colors();
    Some(match s.state {
        State::Running => ("●", app.tr("claude.running"), c.run),
        State::Waiting => ("◐", app.tr("claude.waiting"), c.wait),
        State::Done => ("✓", app.tr("claude.done"), c.done),
        State::Failed => ("✕", app.tr("claude.failed"), c.err),
        State::Other(_) => return None,
    })
}

/// What the worktrees and the branches whose head is `id` would conflict
/// with, their commits alone, one line each; `None` when nothing.
fn head_clashes(app: &App, id: Oid) -> Option<String> {
    if app.merging() {
        return None;
    }
    let cl = &app.clashes;
    let trunk = app.trees.trunk.as_deref().unwrap_or("main");
    let mut lines: Vec<String> = (0..cl.sides.len())
        .filter(|&i| cl.sides[i].head == id)
        .flat_map(|i| cl.of(i))
        .filter_map(|(j, o)| match o {
            Outcome::Committed(c) => Some(clash_line(app, "clash.head", &cl.sides[j].name, c)),
            _ => None,
        })
        .collect();
    for b in cl.branches.iter().filter(|b| b.head == id) {
        if let Outcome::Committed(c) = &b.outcome {
            let line = clash_line(app, "clash.head", trunk, c);
            // A worktree on the same commit says it already.
            if !lines.contains(&line) {
                lines.push(line);
            }
        }
    }
    (!lines.is_empty()).then(|| {
        lines.join(
            "
",
        )
    })
}

/// `key`'s words for a conflict with `other`: its places and its files.
pub(crate) fn clash_line(app: &App, key: &'static str, other: &str, c: &Conflict) -> String {
    app.tr(key).replace("{b}", other).replace("{n}", &c.places().to_string()).replace("{files}", &leaves(app, c))
}

/// A conflict's files by their last names.
pub(crate) fn leaves(app: &App, c: &Conflict) -> String {
    let names: Vec<&str> = c.files.iter().map(|f| f.path.rsplit('/').next().unwrap_or(&f.path)).collect();
    names.join(app.tr("clash.files_sep"))
}

/// What the worktree at `path` would conflict with only once its
/// uncommitted change is counted; `None` when nothing.
fn wip_clashes(app: &App, path: &std::path::Path) -> Option<String> {
    if app.merging() {
        return None;
    }
    let items: Vec<String> = app
        .clashes_in(path)
        .into_iter()
        .filter_map(|(side, o)| match o {
            Outcome::Uncommitted(c) => Some(clash_line(app, "clash.wip_one", &side.name, c)),
            _ => None,
        })
        .collect();
    (!items.is_empty()).then(|| app.tr("clash.wip").replace("{}", &items.join(app.tr("clash.wip_sep"))))
}

/// How wide the ⚠ pill with `text` is.
pub(crate) fn warn_width(ui: &Ui, text: &str) -> f32 {
    5.0 + ui.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), FontId::proportional(11.0), Color32::WHITE)).size().x + 5.0
}

/// The ⚠ pill: outlined in the error colour when the commits conflict,
/// dashed in the waiting colour when only the uncommitted changes do.
pub(crate) fn paint_warn(ui: &Ui, p: &egui::Painter, r: Rect, text: &str, wip: bool, fade: f32) {
    let c = ito_theme::colors();
    let ink = if wip { c.wait } else { c.err }.gamma_multiply(fade);
    if wip {
        let dash = ink.gamma_multiply(0.7);
        let corners = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
        p.extend(Shape::dashed_line(&corners, Stroke::new(1.0, dash), 3.0, 2.0));
    } else {
        p.rect_stroke(r, CornerRadius::same(3), Stroke::new(1.0, ink), StrokeKind::Inside);
    }
    let g = ui.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), FontId::proportional(11.0), ink));
    p.galley(pos2(r.left() + 5.0, r.center().y - g.size().y / 2.0), g, ink);
}

/// A WIP row's subject cell: how much is uncommitted, in italics, what
/// the Claude there is doing, and the ⚠ when the change would conflict.
fn wip_cell(ui: &Ui, app: &App, p: &egui::Painter, w: &Wip, left_center: Pos2, width: f32) {
    let c = ito_theme::colors();
    if w.theirs.is_some() {
        if let Some(cf) = app.conflicts.of(&w.path) {
            return merging_cell(ui, app, p, cf, left_center, width);
        }
    }
    let warn = wip_clashes(app, &w.path).map(|tip| (tip, warn_width(ui, app.tr("clash.pill"))));
    let width = width - warn.as_ref().map_or(0.0, |(_, pw)| pw + 10.0);
    let (a, d) = w.counts();
    let text =
        app.tr("wip.subject").replace("{n}", &w.files.len().to_string()).replace("{a}", &a.to_string()).replace("{d}", &d.to_string());
    let mut job = LayoutJob::default();
    job.append(&text, 0.0, egui::TextFormat { font_id: FontId::proportional(13.0), color: c.dim, italics: true, ..Default::default() });
    if let Some((glyph, word, ink)) = app.claude_in(&w.path).first().and_then(|s| claude_look(app, s)) {
        let format = egui::TextFormat { font_id: FontId::proportional(11.5), color: ink, ..Default::default() };
        job.append(&format!("{glyph} Claude {word}"), 10.0, format);
    }
    job.wrap = TextWrapping::truncate_at_width(width.max(8.0));
    let g = ui.fonts_mut(|f| f.layout_job(job));
    let end = left_center.x + g.size().x;
    p.galley(pos2(left_center.x, left_center.y - g.size().y / 2.0), g, c.dim);
    if let Some((tip, pw)) = warn {
        let pr = Rect::from_min_size(pos2(end + 10.0, left_center.y - 8.5), vec2(pw, 17.0));
        paint_warn(ui, p, pr, app.tr("clash.pill"), true, 1.0);
        ui.interact(pr, Id::new(("graph-clash-wip", &w.path)), Sense::hover()).on_hover_text(tip);
    }
}

/// A merge under way: the places left (or "solved") as a pill, and what
/// is merged into what.
fn merging_cell(ui: &Ui, app: &App, p: &egui::Painter, cf: &michi_core::conflict::Conflict, left_center: Pos2, width: f32) {
    let c = ito_theme::colors();
    let left = cf.left();
    let (pill, done) = if left > 0 {
        (app.tr("graph.merge_left").replace("{n}", &left.to_string()), false)
    } else {
        (app.tr("graph.merge_done").to_owned(), true)
    };
    let pw = warn_width(ui, &pill);
    let pr = Rect::from_min_size(pos2(left_center.x, left_center.y - 8.5), vec2(pw, 17.0));
    if done {
        let g = ui.fonts_mut(|f| f.layout_no_wrap(pill, FontId::proportional(11.0), c.done));
        p.galley(pr.left_center() - vec2(0.0, g.size().y / 2.0), g, c.done);
    } else {
        paint_warn(ui, p, pr, &pill, false, 1.0);
    }
    let text = app.tr("graph.merging").replace("{from}", &cf.theirs.name).replace("{into}", &cf.ours.name);
    let g = ui.fonts_mut(|f| f.layout_no_wrap(text, FontId::proportional(13.0), c.dim));
    let x = pr.right() + 10.0;
    if x + 8.0 < left_center.x + width {
        let clip = p.with_clip_rect(Rect::from_min_max(pos2(x, left_center.y - 12.0), pos2(left_center.x + width, left_center.y + 12.0)));
        clip.galley(pos2(x, left_center.y - g.size().y / 2.0), g, c.dim);
    }
}

// ── pieces ────────────────────────────────────────────────────

/// How wide a name's pill is; `None` for an empty name.
pub(crate) fn pill_width(ui: &Ui, text: &str, kind: PillKind, also: bool) -> Option<f32> {
    if text.is_empty() {
        return None;
    }
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), FontId::proportional(11.0), Color32::WHITE));
    let mut w = 5.0 + galley.size().x + 5.0;
    if matches!(kind, PillKind::Worktree | PillKind::Tag) {
        w += 8.0 + 3.0;
    }
    if also {
        w += 3.0 + ui.fonts_mut(|f| f.layout_no_wrap(" + origin".to_owned(), FontId::proportional(11.0), Color32::WHITE)).size().x;
    }
    Some(w)
}

/// A name's pill: a branch outlined, a worktree's branch filled, a remote
/// branch dashed, a tag round with a diamond.
pub(crate) fn paint_pill(ui: &Ui, p: &egui::Painter, r: Rect, pill: &crate::graph::Pill, fade: f32) {
    let c = ito_theme::colors();
    let ansi = c.ansi();
    let color = match pill.kind {
        PillKind::Tag => c.wait,
        _ => ansi[pill.color as usize & 15],
    }
    .gamma_multiply(fade);
    let mid = r.center().y;
    let font = FontId::proportional(11.0);
    let mut x = r.left() + 5.0;
    let text_ink = match pill.kind {
        PillKind::Local => {
            p.rect_stroke(r, CornerRadius::same(3), Stroke::new(1.0, color), StrokeKind::Inside);
            color
        }
        PillKind::Worktree => {
            p.rect_filled(r, CornerRadius::same(3), color);
            let ink = c.on_accent().gamma_multiply(fade);
            worktree_mark(p, pos2(x + 4.0, mid), ink);
            x += 11.0;
            ink
        }
        PillKind::Remote => {
            let ink = color.gamma_multiply(0.8);
            let corners = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
            p.extend(Shape::dashed_line(&corners, Stroke::new(1.0, ink), 3.0, 2.0));
            ink
        }
        PillKind::Tag => {
            p.rect_stroke(r, CornerRadius::same(9), Stroke::new(1.0, color), StrokeKind::Inside);
            let d = pos2(x + 4.0, mid);
            let diamond = vec![d + vec2(0.0, -3.5), d + vec2(3.5, 0.0), d + vec2(0.0, 3.5), d + vec2(-3.5, 0.0)];
            p.add(Shape::convex_polygon(diamond, color, Stroke::NONE));
            x += 11.0;
            color
        }
    };
    let g = ui.fonts_mut(|f| f.layout_no_wrap(pill.text.clone(), font.clone(), text_ink));
    p.galley(pos2(x, mid - g.size().y / 2.0), g.clone(), text_ink);
    if !pill.also.is_empty() {
        let also = format!(" + {}", pill.also.join(" + "));
        p.text(pos2(x + g.size().x + 3.0, mid), Align2::LEFT_CENTER, also, font, text_ink.gamma_multiply(0.75));
    }
}

/// Two overlapping squares (⧉), centred on `at`.
pub(crate) fn worktree_mark(p: &egui::Painter, at: Pos2, ink: Color32) {
    let s = Stroke::new(1.0, ink);
    p.rect_stroke(Rect::from_min_size(at + vec2(-4.0, -4.0), vec2(5.5, 5.5)), 0.0, s, StrokeKind::Middle);
    p.rect_stroke(Rect::from_min_size(at + vec2(-1.5, -1.5), vec2(5.5, 5.5)), 0.0, s, StrokeKind::Middle);
}

/// ▸ (folded) or ▾ (open), centred on `at`.
pub(crate) fn caret(p: &egui::Painter, at: Pos2, open: bool, ink: Color32) {
    let points = if open {
        vec![at + vec2(-3.5, -2.0), at + vec2(3.5, -2.0), at + vec2(0.0, 2.5)]
    } else {
        vec![at + vec2(-2.0, -3.5), at + vec2(2.5, 0.0), at + vec2(-2.0, 3.5)]
    };
    p.add(Shape::convex_polygon(points, ink, Stroke::NONE));
}

/// `text` from `left_center`, cut with … to `width`.
pub(crate) fn clipped(ui: &Ui, p: &egui::Painter, left_center: Pos2, text: &str, font: FontId, color: Color32, width: f32) {
    if width < 8.0 || text.is_empty() {
        return;
    }
    let mut job = LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = TextWrapping::truncate_at_width(width);
    let g = ui.fonts_mut(|f| f.layout_job(job));
    p.galley(pos2(left_center.x, left_center.y - g.size().y / 2.0), g, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_out_of_sight_comes_to_the_middle_and_one_in_sight_stays() {
        // 10 rows of 26 in sight from row 4 on.
        let (row, view) = (26.0, 260.0);
        let s = 4.0 * row;
        assert_eq!(reveal(s, view, row, 8), s, "well inside");
        assert_eq!(reveal(s, view, row, 40), 40.0 * row + 13.0 - 130.0, "far below: centred");
        assert_eq!(reveal(s, view, row, 0), 0.0 + 13.0 - 130.0, "above: centred (clamped later)");
        assert_eq!(reveal(s, view, row, 5), 5.0 * row - 2.0 * row, "near the top: two rows kept above");
        assert_eq!(reveal(s, view, row, 12), 13.0 * row + 2.0 * row - view, "near the foot: two rows kept below");
        // A view of one row keeps no margin.
        assert_eq!(reveal(0.0, row, row, 0), 0.0);
    }

    #[test]
    fn a_head_tells_what_it_would_conflict_with() {
        use crate::graph::fixture::{self, id};
        use crate::repo::Answer;
        let mut app = App::new(crate::prefs::Prefs::empty(), "C:/r/repo".into());
        app.lang = "en".into();
        app.take(Answer::Trees(fixture::trees()));
        app.take(Answer::Clashes(fixture::clashes()));
        let main = head_clashes(&app, id(6)).unwrap();
        assert!(main.starts_with("Would conflict with wip in 3 places"), "{main}");
        // A branch no worktree is on, against the trunk.
        assert_eq!(head_clashes(&app, id(3)).as_deref(), Some("Would conflict with main in 1 places (a.md)"));
        assert_eq!(head_clashes(&app, id(4)), None);
        // While a merge is under way the prediction is the real conflict's to show.
        app.trees.trees[0].doing = Some(michi_core::branch::Doing::Merge);
        assert!(app.merging());
        assert_eq!(head_clashes(&app, id(6)), None);
        assert_eq!(head_clashes(&app, id(3)), None);
    }

    /// The view draws without a window: before the history, with it, with
    /// conflicts ahead, filtered, unfolded, narrow, and when the repository failed.
    #[test]
    fn the_view_draws_in_every_state() {
        use crate::graph::fixture;
        use crate::repo::Answer;
        let ctx = egui::Context::default();
        let mut app = App::new(crate::prefs::Prefs::empty(), "C:/dev/x".into());
        let frame = |app: &mut App, width: f32| {
            let input =
                egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(width, 600.0))), ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| {
                let r = ui.max_rect();
                show(ui, app, r, true);
            });
            out.textures_delta.clear();
        };
        frame(&mut app, 1280.0);
        app.take(Answer::Setup { gen: 1, setup: Box::new(fixture::setup()) });
        app.take(Answer::Commits { gen: 1, batch: fixture::history(), done: true });
        frame(&mut app, 1280.0);
        // Conflicts ahead: on the heads and on a WIP row.
        app.take(Answer::Wips(vec![fixture::wip("C:/r/wt", Some("wip"), 7, 11, Some(1_767_225_600))]));
        app.take(Answer::Clashes(fixture::clashes()));
        frame(&mut app, 1280.0);
        frame(&mut app, 720.0);
        app.graph.set_filter("f");
        frame(&mut app, 1280.0);
        app.graph.toggle_fold();
        frame(&mut app, 1280.0);
        frame(&mut app, 720.0);
        app.detail.wide = true;
        frame(&mut app, 360.0);
        app.detail.wide = false;
        app.repo = Repo::Failed(michi_core::git::Error::NotRepo);
        frame(&mut app, 1280.0);
    }
}
