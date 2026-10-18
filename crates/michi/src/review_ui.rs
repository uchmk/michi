//! The branch review drawn over the worktree table (docs/design.md:
//! ブランチ全体の差分): the branch and what it would conflict with on top,
//! its files on the left, each with a check once looked at, and the selected
//! file's diff on the right, in one column or side by side. Both sides
//! scroll by the wheel, with no scrollbar; only the rows in sight are drawn.

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Id, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, Ui};
use michi_core::clash::Outcome;
use michi_core::show::File;

use crate::app::App;
use crate::detail::{At, Paired, Unified};
use crate::detail_ui::{diff, left_cut, status_ink, Marks, Wrap, DIFF_H, FILE_H, NOTE_H, TITLE_H, X};
use crate::graph_ui::{clash_line, clipped};
use crate::review::Review;
use crate::ui::kbd;

const PAD_X: f32 = 18.0;
const PAD_TOP: f32 = 14.0;
const TOP_H: f32 = 30.0;
/// The line under the top bar: what the branch would conflict with.
const CLASH_H: f32 = 24.0;
/// The file list's width, at least and at most.
const LIST_MIN: f32 = 220.0;
const LIST_MAX: f32 = 360.0;
/// A check box's side.
const BOX: f32 = 13.0;

fn list_scroll_id() -> Id {
    Id::new("review-list-scroll")
}

/// The selection the list last brought into sight.
fn list_seen_id() -> Id {
    Id::new("review-list-seen")
}

fn scroll_id() -> Id {
    Id::new("review-scroll")
}

/// What a click does, once everything is drawn.
enum Act {
    Back,
    Key(char),
    Select(usize),
    Check(usize),
    Wide(bool),
    Line(At),
}

/// The note strip under the diff: the room it takes.
const NOTE_BOX_H: f32 = 118.0;
const NOTE_LINE_H: f32 = 26.0;

fn note_id() -> Id {
    Id::new("michi-review-note")
}

/// How far a row of scrolling by key goes.
const ROW_NUDGE: f32 = 48.0;

/// The review in `r`.
pub fn show(ui: &mut Ui, app: &mut App, r: Rect, live: bool) {
    let c = ito_theme::colors();
    let ctx = ui.ctx().clone();
    let p = ui.painter_at(r);
    p.rect_filled(r, 0.0, c.bg);
    let Some(rv) = app.review.as_mut() else { return };
    let (to_top, pages, follow) = (std::mem::take(&mut rv.to_top), std::mem::take(&mut rv.pages), std::mem::take(&mut rv.follow));
    let nudge = std::mem::take(&mut rv.nudge);
    if to_top {
        // Also while the new file's diff is read.
        ctx.data_mut(|d| d.insert_temp(scroll_id(), 0.0_f32));
    }
    let Some(rv) = app.review.as_ref() else { return };
    let pointer = if live { ui.input(|i| i.pointer.hover_pos()).filter(|at| r.contains(*at)) } else { None };
    let mut act = None;
    let trunk = app.trees.trunk.as_deref().unwrap_or("main");

    // The top bar: the branch and its range; the switches, how many are
    // seen, and the way back.
    let left = r.left() + PAD_X;
    let mid = r.top() + PAD_TOP + TOP_H / 2.0;
    let cap = kbd(ui, &p, pos2(r.right() - PAD_X, mid), "Esc");
    let back = p.text(pos2(cap.left() - 6.0, mid), Align2::RIGHT_CENTER, app.tr("key.back"), FontId::proportional(12.0), c.dim);
    if clicked(ui, &p, back.union(cap).expand(3.0), "review-back", live, pointer) {
        act = Some(Act::Back);
    }
    let mut right = back.left() - 16.0;
    if let (Some(Ok(_)), None) = (&rv.list, rv.cmp) {
        let (n, m) = rv.checked();
        let text = app.tr("review.seen").replace("{n}", &n.to_string()).replace("{m}", &m.to_string());
        let ink = if n == m && m > 0 { c.done } else { c.dim };
        right = p.text(pos2(right, mid), Align2::RIGHT_CENTER, text, FontId::proportional(12.0), ink).left() - 14.0;
    }
    if !rv.notes.is_empty() {
        let text = app.tr("review.notes").replace("{n}", &rv.notes.len().to_string());
        right = p.text(pos2(right, mid), Align2::RIGHT_CENTER, text, FontId::proportional(12.0), c.wait).left() - 14.0;
    }
    let switches: &[(&str, char, bool)] =
        if rv.cmp.is_some() { &[] } else { &[("review.since_last", 's', rv.since_last), ("review.uncommitted", 'u', rv.wip)] };
    for &(word, key, on) in switches {
        let (l, hit) = switch(ui, &p, right, mid, app.tr(word), key, on, live);
        if hit {
            act = Some(Act::Key(key));
        }
        right = l - 6.0;
    }
    let mut job = LayoutJob::default();
    job.append(&rv.branch, 0.0, TextFormat { font_id: FontId::proportional(15.0), color: c.fg, ..Default::default() });
    let range = match (rv.cmp, rv.since()) {
        (Some((a, b)), _) => format!("{}...{}", a.short(), b.short()),
        (None, Some(since)) => app.tr("review.since").replace("{}", &since.short()),
        (None, None) => format!("{trunk}...{}", rv.branch),
    };
    job.append(&range, 12.0, TextFormat { font_id: FontId::monospace(11.5), color: c.dim, ..Default::default() });
    job.wrap = TextWrapping::truncate_at_width((right - 10.0 - left).max(0.0));
    let title = ui.fonts_mut(|f| f.layout_job(job));
    // The branch a touch bold, as the worktree table's title.
    for dx in [0.0, 0.5] {
        p.galley(pos2(left + dx, mid - title.size().y / 2.0), title.clone(), c.fg);
    }
    if let Some((text, ink)) = prediction(app, rv) {
        let y = r.top() + PAD_TOP + TOP_H + CLASH_H / 2.0;
        clipped(ui, &p, pos2(left, y), &text, FontId::proportional(12.5), ink, r.right() - PAD_X - left);
    }

    // The files on the left, the selected one's diff on the right.
    let top = r.top() + PAD_TOP + TOP_H + CLASH_H + 6.0;
    p.line_segment([pos2(r.left(), top), pos2(r.right(), top)], Stroke::new(1.0, c.border));
    let lw = (r.width() * 0.3).clamp(LIST_MIN, LIST_MAX).min(r.width() / 2.0);
    let lr = Rect::from_min_max(pos2(r.left(), top + 1.0), pos2(r.left() + lw, r.bottom()));
    let dr = Rect::from_min_max(pos2(lr.right() + 1.0, top + 1.0), r.max);
    p.line_segment([pos2(lr.right() + 0.5, top), pos2(lr.right() + 0.5, r.bottom())], Stroke::new(1.0, c.border));
    list(ui, app, rv, lr, live, pointer, &mut act);
    if let Some(f) = rv.selected() {
        if let Some(a) = title_row(ui, app, rv, f, Rect::from_min_size(dr.min, vec2(dr.width(), TITLE_H)), live, pointer) {
            act = Some(a);
        }
        let full = Rect::from_min_max(pos2(dr.left(), dr.top() + TITLE_H), dr.max);
        let strip_h = if rv.editing.is_some() {
            NOTE_BOX_H
        } else if rv.note_here().is_some() {
            NOTE_LINE_H
        } else {
            0.0
        };
        let body = Rect::from_min_max(full.min, pos2(full.right(), (full.bottom() - strip_h).max(full.top())));
        if let Some(a) = body_of(ui, app, rv, body, live, (pages, nudge), follow) {
            act = Some(a);
        }
        if strip_h > 0.0 {
            let strip = Rect::from_min_max(pos2(full.left(), body.bottom()), full.max);
            note_strip(ui, app, strip);
        }
    }

    let Some(a) = act else { return };
    match a {
        Act::Back => app.close_review(),
        Act::Key(k) => {
            app.feed(k);
        }
        Act::Select(k) => {
            let wish = app.review.as_mut().and_then(|rv| rv.select(k));
            app.wish(wish);
        }
        Act::Check(k) => {
            let wish = app.review.as_mut().and_then(|rv| rv.check_at(k));
            app.wish(wish);
        }
        Act::Line(at) => {
            if let Some(rv) = app.review.as_mut() {
                rv.point(at);
            }
        }
        Act::Wide(w) => {
            if let Some(rv) = app.review.as_mut() {
                rv.wide = w;
            }
        }
    }
    ctx.request_repaint();
}

/// Whether `hit` was clicked; a hand over it.
fn clicked(ui: &Ui, p: &Painter, hit: Rect, id: &str, live: bool, pointer: Option<Pos2>) -> bool {
    let resp = ui.interact(hit.intersect(p.clip_rect()), Id::new(id), Sense::CLICK);
    if live && pointer.is_some_and(|at| hit.contains(at)) {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    live && resp.clicked()
}

/// A switch of the top bar, its right edge at `right`, lit while `on`; its
/// left edge, and whether it was clicked.
#[expect(clippy::too_many_arguments, reason = "the painter, the place and the look are the frame's")]
fn switch(ui: &Ui, p: &Painter, right: f32, mid: f32, label: &str, key: char, on: bool, live: bool) -> (f32, bool) {
    let c = ito_theme::colors();
    let ink = if on { c.run } else { c.dim };
    let g = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), FontId::proportional(12.5), ink));
    let k = ui.fonts_mut(|f| f.layout_no_wrap(key.to_string(), FontId::monospace(10.5), c.dim));
    let w = 9.0 + g.size().x + 6.0 + k.size().x + 9.0;
    let b = Rect::from_min_max(pos2(right - w, mid - 12.0), pos2(right, mid + 12.0));
    let resp = ui.interact(b.intersect(p.clip_rect()), Id::new(("review-switch", key)), Sense::CLICK);
    if on {
        p.rect_filled(b, CornerRadius::same(5), c.run.gamma_multiply(0.12));
    } else if live && resp.hovered() {
        p.rect_filled(b, CornerRadius::same(5), c.panel);
    }
    if live && resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let edge = if on { c.run.gamma_multiply(0.45) } else { c.border };
    p.rect_stroke(b, CornerRadius::same(5), Stroke::new(1.0, edge), StrokeKind::Inside);
    p.galley(pos2(b.left() + 9.0, mid - g.size().y / 2.0), g.clone(), ink);
    p.galley(pos2(b.right() - 9.0 - k.size().x, mid - k.size().y / 2.0), k, c.dim);
    (b.left(), live && resp.clicked())
}

/// What the branch would do against the trunk, with its colour; `None`
/// when it is not known.
fn prediction(app: &App, rv: &Review) -> Option<(String, Color32)> {
    let c = ito_theme::colors();
    let trunk = app.trees.trunk.as_deref()?;
    let cl = &app.clashes;
    if rv.cmp.is_some() {
        return None;
    }
    let me = cl.side_of(&rv.path)?;
    let them = cl.sides.iter().position(|s| s.name == trunk)?;
    Some(match cl.between(me, them)? {
        Outcome::Clean => (app.tr("review.clean").replace("{}", trunk), c.done),
        Outcome::Committed(x) => (clash_line(app, "clash.head", trunk, x), c.err),
        Outcome::Uncommitted(x) => (app.tr("clash.wip").replace("{}", &clash_line(app, "clash.wip_one", trunk, x)), c.wait),
        Outcome::Unknown => return None,
    })
}

/// A note at the top of `r`, wrapped to its width.
fn note(ui: &Ui, p: &Painter, r: Rect, text: &str) {
    let c = ito_theme::colors();
    let g = ui.fonts_mut(|f| f.layout(text.to_owned(), FontId::proportional(12.0), c.dim, (r.width() - 2.0 * X).max(40.0)));
    p.galley(pos2(r.left() + X, r.top() + 10.0), g, c.dim);
}

/// The file list in `lr`.
fn list(ui: &Ui, app: &App, rv: &Review, lr: Rect, live: bool, pointer: Option<Pos2>, act: &mut Option<Act>) {
    let c = ito_theme::colors();
    let ctx = ui.ctx();
    let p = ui.painter_at(lr);
    let trunk = app.trees.trunk.as_deref().unwrap_or("main");
    let l = match &rv.list {
        None => return note(ui, &p, lr, app.tr("review.reading")),
        Some(Err(e)) => return note(ui, &p, lr, &app.tr("review.failed").replace("{}", &e.to_string())),
        Some(Ok(l)) => l,
    };
    let files = &l.changes.files;
    if l.base.is_none() {
        return note(ui, &p, lr, &app.tr("review.no_base").replace("{}", trunk));
    }
    if files.is_empty() {
        let text =
            if rv.since().is_some() { app.tr("review.nothing_since").to_owned() } else { app.tr("review.nothing").replace("{}", trunk) };
        return note(ui, &p, lr, &text);
    }
    let pad = 6.0;
    let rows_h = files.len() as f32 * FILE_H;
    let total = pad + rows_h + if l.changes.cut { NOTE_H } else { 0.0 } + 12.0;
    let mut scroll = ctx.data(|d| d.get_temp::<f32>(list_scroll_id())).unwrap_or(0.0);
    if live && pointer.is_some_and(|at| lr.contains(at)) {
        scroll -= ui.input(|i| i.smooth_scroll_delta.y);
    }
    // A selection moved by the keys comes into sight.
    if ctx.data(|d| d.get_temp::<usize>(list_seen_id())) != Some(rv.sel) {
        let top = pad + rv.sel as f32 * FILE_H;
        if top < scroll + pad {
            scroll = top - pad;
        } else if top + FILE_H > scroll + lr.height() - pad {
            scroll = top + FILE_H - lr.height() + pad;
        }
        ctx.data_mut(|d| d.insert_temp(list_seen_id(), rv.sel));
    }
    scroll = scroll.clamp(0.0, (total - lr.height()).max(0.0));
    ctx.data_mut(|d| d.insert_temp(list_scroll_id(), scroll));

    let y0 = lr.top() + pad - scroll;
    let first = ((scroll - pad) / FILE_H).floor().max(0.0) as usize;
    for (k, f) in files.iter().enumerate().skip(first) {
        let rr = Rect::from_min_size(pos2(lr.left(), y0 + k as f32 * FILE_H), vec2(lr.width(), FILE_H));
        if rr.top() > lr.bottom() {
            break;
        }
        let hot = pointer.is_some_and(|at| rr.contains(at));
        if k == rv.sel {
            p.rect_filled(rr, 0.0, c.selection());
        } else if hot {
            p.rect_filled(rr, 0.0, c.fg.gamma_multiply(0.03));
        }
        let check = list_row(ui, &p, app, f, rr, rv.is_checked(f));
        let resp = ui.interact(rr.intersect(lr), Id::new(("review-row", k)), Sense::CLICK);
        if live && hot {
            ctx.set_cursor_icon(CursorIcon::PointingHand);
        }
        if live && resp.clicked() {
            let on_box = resp.interact_pointer_pos().is_some_and(|at| check.expand(4.0).contains(at));
            *act = Some(if on_box { Act::Check(k) } else { Act::Select(k) });
        }
    }
    if l.changes.cut {
        let at = Rect::from_min_size(pos2(lr.left(), y0 + rows_h), vec2(lr.width(), NOTE_H));
        note(ui, &p, at, app.tr("review.cut"));
    }
}

/// A row of the file list: the check, the status, the path (cut from the
/// left, dim once checked) and the counts; the check box.
fn list_row(ui: &Ui, p: &Painter, app: &App, f: &File, rr: Rect, checked: bool) -> Rect {
    let c = ito_theme::colors();
    let mid = rr.center().y;
    let check = Rect::from_center_size(pos2(rr.left() + X + BOX / 2.0, mid), vec2(BOX, BOX));
    if checked {
        p.rect_filled(check, CornerRadius::same(3), c.done);
        p.text(check.center(), Align2::CENTER_CENTER, "✓", FontId::proportional(10.5), c.bg);
    } else {
        p.rect_stroke(check, CornerRadius::same(3), Stroke::new(1.0, c.dim.gamma_multiply(0.7)), StrokeKind::Inside);
    }
    let x = check.right() + 8.0;
    p.text(pos2(x + 4.0, mid), Align2::CENTER_CENTER, f.status.letter(), FontId::monospace(11.0), status_ink(f.status));
    let right = rr.right() - X;
    let counts_left = match f.counts {
        Some((a, d)) => {
            let dr = p.text(pos2(right, mid), Align2::RIGHT_CENTER, format!("−{d}"), FontId::monospace(11.0), c.err);
            p.text(pos2(dr.left() - 5.0, mid), Align2::RIGHT_CENTER, format!("+{a}"), FontId::monospace(11.0), c.done).left()
        }
        None => p.text(pos2(right, mid), Align2::RIGHT_CENTER, app.tr("detail.binary"), FontId::proportional(11.5), c.dim).left(),
    };
    let px = x + 16.0;
    let path = left_cut(ui, &f.path, FontId::proportional(12.5), counts_left - 8.0 - px);
    p.text(pos2(px, mid), Align2::LEFT_CENTER, path, FontId::proportional(12.5), if checked { c.dim } else { c.fg });
    check
}

/// The selected file's title over its diff, with the switch between one
/// column and side by side.
fn title_row(ui: &Ui, app: &App, rv: &Review, f: &File, rr: Rect, live: bool, pointer: Option<Pos2>) -> Option<Act> {
    let c = ito_theme::colors();
    let p = ui.painter_at(rr);
    p.rect_filled(rr, 0.0, c.side);
    p.line_segment([rr.left_bottom() - vec2(0.0, 0.5), rr.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
    let mid = rr.center().y;
    let (word, key) = if rv.wide { ("detail.narrow", "Esc") } else { ("detail.wide", "Enter") };
    let cap = kbd(ui, &p, pos2(rr.right() - X, mid), key);
    let w = p.text(pos2(cap.left() - 6.0, mid), Align2::RIGHT_CENTER, app.tr(word), FontId::proportional(12.0), c.dim);
    let hit = w.union(cap).expand(3.0);
    let out = clicked(ui, &p, hit, "review-wide", live, pointer).then_some(Act::Wide(!rv.wide));
    let end = hit.left() - 10.0;
    let x = rr.left() + X;
    let letter = p.text(pos2(x, mid), Align2::LEFT_CENTER, f.status.letter(), FontId::monospace(12.0), status_ink(f.status));
    let mut job = LayoutJob::default();
    let font = FontId::monospace(12.0);
    job.append(&f.path, 0.0, TextFormat { font_id: font.clone(), color: c.fg, ..Default::default() });
    if let Some(from) = &f.from {
        let text = format!("  {}", app.tr("detail.from").replace("{}", from));
        job.append(&text, 0.0, TextFormat { font_id: font, color: c.dim, ..Default::default() });
    }
    let px = letter.right() + 8.0;
    job.wrap = TextWrapping::truncate_at_width((end - px).max(0.0));
    let g = ui.fonts_mut(|fo| fo.layout_job(job));
    p.galley(pos2(px, mid - g.size().y / 2.0), g, c.fg);
    out
}

/// A piece of the diff's side, top to bottom.
#[derive(Clone, Copy)]
enum Piece {
    Note(&'static str),
    Diff,
    /// The diff stops short.
    Cut,
}

/// The selected file's diff in `body`, scrolled by the wheel and by the
/// `pages` and rows (`nudge`) the keys asked for.
fn body_of(ui: &Ui, app: &App, rv: &Review, body: Rect, live: bool, (pages, nudge): (i32, i32), follow: bool) -> Option<Act> {
    let c = ito_theme::colors();
    let ctx = ui.ctx();
    let p = ui.painter_at(body);
    let d = match rv.diff() {
        None => {
            note(ui, &p, body, app.tr("detail.reading"));
            return None;
        }
        Some(Err(e)) => {
            note(ui, &p, body, &app.tr("detail.failed").replace("{}", &e.to_string()));
            return None;
        }
        Some(Ok(None)) => {
            note(ui, &p, body, app.tr("detail.no_diff"));
            return None;
        }
        Some(Ok(Some(d))) => d,
    };
    let (f, rows) = (&d.0, &d.1);
    let noted: Vec<At> = (0..f.hunks.len())
        .flat_map(|h| (0..f.hunks[h].lines.len()).map(move |l| (h as u32, l as u32)))
        .filter(|at| rv.is_noted(*at))
        .collect();
    let marks = Marks { cursor: rv.line, noted: &noted };
    let mut pieces = Vec::new();
    if f.not_utf8 {
        pieces.push(Piece::Note("detail.not_utf8"));
    }
    if michi_core::lfs::of_file(f).is_some() {
        pieces.push(Piece::Note("detail.lfs_note"));
    }
    if f.is_binary() {
        pieces.push(Piece::Note("detail.binary_note"));
    } else if f.eol_only {
        pieces.push(Piece::Note("detail.eol_only"));
    } else if f.hunks.is_empty() && !f.cut {
        pieces.push(Piece::Note("detail.no_diff"));
    } else if !f.hunks.is_empty() {
        pieces.push(Piece::Diff);
    }
    if f.cut {
        pieces.push(Piece::Cut);
    }
    let wrap = Wrap::new(ui, body.width(), rv.wide);
    let heights: Vec<f32> = pieces
        .iter()
        .map(|piece| match piece {
            Piece::Note(_) => NOTE_H,
            Piece::Diff => wrap.tops(rows).last().copied().unwrap_or(0.0) + 6.0,
            Piece::Cut => DIFF_H + 6.0,
        })
        .collect();
    let total: f32 = heights.iter().sum::<f32>() + 24.0;

    let mut scroll = ctx.data(|dt| dt.get_temp::<f32>(scroll_id())).unwrap_or(0.0);
    scroll += pages as f32 * (body.height() - 40.0).max(40.0) + nudge as f32 * ROW_NUDGE;
    let tops = wrap.tops(rows);
    let diff_top: f32 = pieces.iter().zip(&heights).take_while(|(piece, _)| !matches!(piece, Piece::Diff)).map(|(_, h)| h).sum();
    let row_of = |at: At| {
        if rv.wide {
            rows.paired.iter().position(|r| matches!(r, Paired::Pair(a, b) if *a == Some(at) || *b == Some(at)))
        } else {
            rows.unified.iter().position(|r| *r == Unified::Line(at))
        }
    };
    if let (true, Some(k)) = (follow, rv.line.and_then(row_of)) {
        let (top, bottom) = (diff_top + tops[k], diff_top + tops[k + 1]);
        if top < scroll {
            scroll = top - 8.0;
        } else if bottom > scroll + body.height() {
            scroll = bottom - body.height() + 8.0;
        }
    }
    if live && ui.rect_contains_pointer(body) {
        scroll -= ui.input(|i| i.smooth_scroll_delta.y);
    }
    scroll = scroll.clamp(0.0, (total - body.height()).max(0.0));
    ctx.data_mut(|dt| dt.insert_temp(scroll_id(), scroll));

    let mut act = None;
    // The diff's text is picked up with the mouse, as the commit detail's.
    crate::select::begin(&format!("review:{}:{}:{}", rv.branch, f.path, rv.wide));
    let mut y = body.top() - scroll;
    for (piece, h) in pieces.iter().zip(heights) {
        let rr = Rect::from_min_size(pos2(body.left(), y), vec2(body.width(), h));
        y += h;
        if rr.bottom() < body.top() || rr.top() > body.bottom() {
            continue;
        }
        match *piece {
            Piece::Note(key) => {
                p.text(pos2(rr.left() + X, rr.top() + 10.0), Align2::LEFT_TOP, app.tr(key), FontId::proportional(12.0), c.dim);
            }
            Piece::Diff => {
                let at = Rect::from_min_size(rr.min, vec2(rr.width(), rr.height() - 6.0));
                diff(ui, &p, f, rows, at, body, &wrap, &marks);
                // A drag or a double click selects text; a plain click still points at a line.
                let view = at.intersect(body);
                let resp = ui.interact(view, Id::new("review-diff"), Sense::click_and_drag());
                if live {
                    let driven = crate::select::drive(&resp, view);
                    if let Some(text) = driven.copy {
                        ctx.copy_text(text);
                    }
                    if let Some(url) = driven.open {
                        std::thread::spawn(move || {
                            let _ = michi_core::pr::open_url(&url);
                        });
                    }
                    if ui.input(|i| i.modifiers.command) && resp.hover_pos().is_some_and(|q| crate::select::url_at(q, view)) {
                        ctx.set_cursor_icon(CursorIcon::PointingHand);
                    }
                }
                if let (true, true, Some(q)) = (live, resp.clicked(), resp.interact_pointer_pos()) {
                    let y = q.y - at.top();
                    let k = tops.partition_point(|&t| t <= y).saturating_sub(1).min(tops.len() - 2);
                    let hit = if rv.wide {
                        match rows.paired[k] {
                            Paired::Pair(old, new) => new.or(old),
                            Paired::Hunk(_) => None,
                        }
                    } else {
                        match rows.unified[k] {
                            Unified::Line(at) => Some(at),
                            Unified::Hunk(_) => None,
                        }
                    };
                    if let Some(at) = hit {
                        act = Some(Act::Line(at));
                    }
                }
            }
            Piece::Cut => {
                let mut job = LayoutJob::default();
                job.append(
                    app.tr("detail.cut"),
                    0.0,
                    TextFormat { font_id: FontId::proportional(12.0), color: c.dim, italics: true, ..Default::default() },
                );
                let g = ui.fonts_mut(|fo| fo.layout_job(job));
                p.galley(pos2(rr.left() + X, rr.top() + (DIFF_H - g.size().y) / 2.0), g, c.dim);
            }
        }
    }
    crate::select::paint();
    act
}

/// The strip under the diff: the note being written for the cursor's line,
/// or the one it has.
fn note_strip(ui: &mut Ui, app: &mut App, strip: Rect) {
    let c = ito_theme::colors();
    let p = ui.painter_at(strip);
    p.rect_filled(strip, 0.0, c.side);
    p.line_segment([strip.left_top() + vec2(0.0, 0.5), strip.right_top() + vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
    let small = FontId::proportional(12.0);
    let editing = app.review.as_ref().is_some_and(|r| r.editing.is_some());
    let head = app.review.as_ref().and_then(|r| Some((r.selected()?.path.clone(), r.target()?)));
    let Some((path, (_, line, code))) = head else { return };
    let keys = app.tr("review.note_keys").to_owned();
    if editing {
        let label = format!("{path}:{line}   {}", code.trim());
        clipped(ui, &p, pos2(strip.left() + X, strip.top() + 12.0), &label, small.clone(), c.dim, strip.width() - 2.0 * X);
        let edit = Rect::from_min_max(pos2(strip.left() + X, strip.top() + 22.0), pos2(strip.right() - X, strip.bottom() - 22.0));
        if let Some(text) = app.review.as_mut().and_then(|r| r.editing.as_mut()) {
            let resp = ui.put(edit, egui::TextEdit::multiline(text).id(note_id()).desired_width(f32::INFINITY));
            if !resp.has_focus() {
                resp.request_focus();
            }
        }
        p.text(pos2(strip.left() + X, strip.bottom() - 11.0), Align2::LEFT_CENTER, keys, small, c.dim);
    } else if let Some(n) = app.review.as_ref().and_then(|r| r.note_here()) {
        let text = format!("✎ {}", n.text.replace('\n', " "));
        clipped(ui, &p, pos2(strip.left() + X, strip.center().y), &text, small, c.wait, strip.width() - 2.0 * X);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::View;
    use crate::detail::fixture::detail;
    use crate::graph::fixture;
    use crate::repo::Answer;
    use michi_core::oid::Oid;
    use michi_core::review::List;
    use michi_core::show::Changes;
    use std::collections::BTreeSet;

    /// The worktree table read, and the review of `wip` open.
    fn opened() -> App {
        let mut app = App::new(crate::prefs::Prefs::empty(), "C:/dev/x".into());
        app.lang = "en".into();
        app.view = View::Worktree;
        app.take(Answer::Trees(fixture::trees()));
        app.take(Answer::Wips(vec![fixture::wip("C:/r/wt", Some("wip"), 7, 11, Some(1))]));
        app.take(Answer::Clashes(fixture::clashes()));
        app.wt_sel = 1;
        assert!(app.feed('d'));
        app
    }

    fn gens(app: &App) -> (u32, u32) {
        app.review.as_ref().expect("the review is open").gens()
    }

    /// detail.rs's four files: not UTF-8, renamed, binary, and cut.
    fn files() -> Vec<File> {
        detail(fixture::id(7), Vec::new(), 4).files
    }

    fn listed(files: Vec<File>, last: Option<Oid>) -> List {
        List { base: Some(fixture::id(5)), last, changes: Changes { files, cut: false } }
    }

    fn take_list(app: &mut App, result: Result<List, michi_core::git::Error>) {
        let gen = gens(app).0;
        app.take(Answer::ReviewList { gen, result: result.map(Box::new), checks: BTreeSet::new() });
    }

    fn take_file(app: &mut App, result: Result<Changes, michi_core::git::Error>) {
        let gen = gens(app).1;
        app.take(Answer::ReviewFile { gen, result: result.map(Box::new) });
    }

    /// The selected file's diff, as git gives it.
    fn take_selected(app: &mut App) {
        let f = app.review.as_ref().and_then(|r| r.selected()).cloned().expect("a file is selected");
        take_file(app, Ok(Changes { files: vec![f], cut: false }));
    }

    fn frame(ctx: &egui::Context, app: &mut App, width: f32, events: Vec<egui::Event>) {
        let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(width, 500.0))), events, ..Default::default() };
        let mut out = ctx.run_ui(input, |ui| {
            let r = ui.max_rect();
            show(ui, app, r, true);
        });
        out.textures_delta.clear();
    }

    #[test]
    fn a_note_is_written_on_a_line_kept_and_taken_away() {
        let ctx = egui::Context::default();
        let mut app = opened();
        take_list(&mut app, Ok(listed(files(), None)));
        take_selected(&mut app);
        assert!(app.feed('g'));
        assert_eq!(crate::app::tests::toast(&app), app.tr("review.no_notes"));
        assert!(app.feed('J'));
        let line = app.review.as_ref().unwrap().line.expect("the first line");
        assert!(app.feed('n'));
        app.review.as_mut().unwrap().editing = Some("why this?".into());
        frame(&ctx, &mut app, 900.0, Vec::new());
        frame(&ctx, &mut app, 500.0, Vec::new());
        app.review.as_mut().unwrap().noted();
        let rv = app.review.as_ref().unwrap();
        assert_eq!((rv.notes.len(), rv.is_noted(line), rv.note_here().map(|n| n.text.as_str())), (1, true, Some("why this?")));
        frame(&ctx, &mut app, 900.0, Vec::new());
        assert!(app.named("Enter"));
        frame(&ctx, &mut app, 1400.0, Vec::new());
        // The notes stay with the worktree while the review is closed.
        app.close_review();
        assert_eq!(app.kept_notes.len(), 1);
        assert!(app.feed('d'));
        assert_eq!(app.review.as_ref().unwrap().notes.len(), 1);
        assert!(app.kept_notes.is_empty());
        // The same line written empty takes it away.
        take_list(&mut app, Ok(listed(files(), None)));
        take_selected(&mut app);
        app.feed('J');
        assert!(app.feed('n'));
        assert_eq!(app.review.as_ref().unwrap().editing.as_deref(), Some("why this?"), "the note is written again");
        app.review.as_mut().unwrap().editing = Some(String::new());
        app.review.as_mut().unwrap().noted();
        assert!(app.review.as_ref().unwrap().notes.is_empty());
    }

    #[test]
    fn the_review_draws_in_every_state() {
        let ctx = egui::Context::default();
        let mut app = opened();
        frame(&ctx, &mut app, 900.0, Vec::new());
        // Looked over before, up to m2.
        take_list(&mut app, Ok(listed(files(), Some(fixture::id(6)))));
        frame(&ctx, &mut app, 900.0, Vec::new());
        for k in 0..4 {
            assert_eq!(app.review.as_ref().unwrap().sel, k);
            take_selected(&mut app);
            frame(&ctx, &mut app, 900.0, Vec::new());
            frame(&ctx, &mut app, 300.0, Vec::new());
            app.feed('j');
        }
        assert!(app.feed(' '));
        assert_eq!(app.review.as_ref().unwrap().checked(), (1, 4), "the last checked, and nothing after it");
        assert!(app.named("Enter"));
        frame(&ctx, &mut app, 1400.0, Vec::new());
        frame(&ctx, &mut app, 500.0, Vec::new());
        assert!(app.escape());
        assert!(!app.review.as_ref().unwrap().wide, "Esc brings one column back first");

        // A long diff scrolls a page at a time, and a new file starts at its top.
        let mut long = files().swap_remove(1);
        let lines = long.hunks[0].lines.clone();
        long.hunks[0].lines = lines.iter().cycle().take(400).cloned().collect();
        take_list(&mut app, Ok(listed(vec![long, files().swap_remove(2)], None)));
        take_selected(&mut app);
        frame(&ctx, &mut app, 900.0, Vec::new());
        assert!(app.named("PageDown"));
        frame(&ctx, &mut app, 900.0, Vec::new());
        assert!(ctx.data(|d| d.get_temp::<f32>(scroll_id())).unwrap() > 300.0, "a page down");
        app.feed('j');
        frame(&ctx, &mut app, 900.0, Vec::new());
        assert_eq!(ctx.data(|d| d.get_temp::<f32>(scroll_id())), Some(0.0));

        // The diff not read, or empty.
        take_file(&mut app, Err(michi_core::git::Error::Git("bad object".into())));
        frame(&ctx, &mut app, 900.0, Vec::new());
        take_file(&mut app, Ok(Changes::default()));
        frame(&ctx, &mut app, 900.0, Vec::new());
        // The list not read, with no base, empty, and cut short.
        take_list(&mut app, Err(michi_core::git::Error::Git("bad revision".into())));
        frame(&ctx, &mut app, 900.0, Vec::new());
        take_list(&mut app, Ok(List { base: None, ..listed(Vec::new(), None) }));
        frame(&ctx, &mut app, 900.0, Vec::new());
        take_list(&mut app, Ok(listed(Vec::new(), Some(fixture::id(6)))));
        frame(&ctx, &mut app, 900.0, Vec::new());
        let mut l = listed(files(), None);
        l.changes.cut = true;
        take_list(&mut app, Ok(l));
        frame(&ctx, &mut app, 900.0, Vec::new());
        // Since the last look, with nothing new.
        assert!(app.feed('s'));
        assert!(app.review.as_ref().unwrap().since_last);
        take_list(&mut app, Ok(listed(Vec::new(), Some(fixture::id(6)))));
        assert!(app.review.as_ref().unwrap().since().is_some());
        frame(&ctx, &mut app, 900.0, Vec::new());
        // With no prediction against the trunk.
        app.take(Answer::Clashes(Default::default()));
        frame(&ctx, &mut app, 900.0, Vec::new());
    }

    #[test]
    fn a_click_selects_a_file_or_checks_it() {
        let ctx = egui::Context::default();
        let mut app = opened();
        take_list(&mut app, Ok(listed(files(), None)));
        frame(&ctx, &mut app, 900.0, Vec::new());
        let row = |k: usize| PAD_TOP + TOP_H + CLASH_H + 7.0 + 6.0 + (k as f32 + 0.5) * FILE_H;
        let click = |app: &mut App, at: Pos2| {
            let button = |pressed| egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            };
            frame(&ctx, app, 900.0, vec![egui::Event::PointerMoved(at), button(true)]);
            frame(&ctx, app, 900.0, vec![button(false)]);
            frame(&ctx, app, 900.0, Vec::new());
        };
        click(&mut app, pos2(120.0, row(2)));
        let rv = app.review.as_ref().unwrap();
        assert_eq!((rv.sel, rv.checked()), (2, (0, 4)), "the path selects");
        click(&mut app, pos2(X + BOX / 2.0, row(0)));
        let rv = app.review.as_ref().unwrap();
        assert_eq!((rv.sel, rv.checked()), (2, (1, 4)), "the box checks, and the selection stays");
        assert!(rv.is_checked(&rv.files()[0]));
        click(&mut app, pos2(X + BOX / 2.0, row(0)));
        assert_eq!(app.review.as_ref().unwrap().checked(), (0, 4), "and takes the check off");
    }

    #[test]
    fn a_drag_over_the_diff_selects_text_and_a_click_still_points_at_a_line() {
        let ctx = egui::Context::default();
        let mut app = opened();
        take_list(&mut app, Ok(listed(files(), None)));
        take_selected(&mut app);
        frame(&ctx, &mut app, 900.0, Vec::new());
        let top = PAD_TOP + TOP_H + CLASH_H + 7.0 + TITLE_H;
        let button = |at: Pos2, pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let (from, to) = (pos2(420.0, top + 30.0), pos2(800.0, top + 80.0));
        frame(&ctx, &mut app, 900.0, vec![egui::Event::PointerMoved(from), button(from, true)]);
        frame(&ctx, &mut app, 900.0, vec![egui::Event::PointerMoved(to)]);
        frame(&ctx, &mut app, 900.0, vec![button(to, false)]);
        frame(&ctx, &mut app, 900.0, Vec::new());
        assert!(crate::select::text().is_some_and(|t| !t.is_empty()), "the drag picked text up");
        assert_eq!(app.review.as_ref().unwrap().line, None, "and was not a click on a line");
        // Some rows are hunk headers, which a click leaves alone; one of the first few is a line.
        for k in 1..6 {
            let at = pos2(420.0, top + 8.0 + 22.0 * k as f32);
            frame(&ctx, &mut app, 900.0, vec![egui::Event::PointerMoved(at), button(at, true)]);
            frame(&ctx, &mut app, 900.0, vec![button(at, false)]);
            frame(&ctx, &mut app, 900.0, Vec::new());
        }
        assert!(app.review.as_ref().unwrap().line.is_some(), "a plain click points at the line");
    }
}
