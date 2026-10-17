//! The detail drawn: the selected commit's header, the commits a trunk
//! merge brought in, its files, and each file's diff in one column or side
//! by side. It scrolls by the wheel, with no scrollbar (docs/design.md: the
//! graph's is the one); only the rows in sight are drawn.

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Id, Painter, Pos2, Rect, Sense, Stroke, Ui};
use michi_core::claude::State;
use michi_core::oid::Oid;
use michi_core::show::{Detail, File, Kind, Line, Status};
use michi_core::wip::Wip;

use crate::app::App;
use crate::detail::{listed, At, Paired, Unified, FILES_CUT, FILES_SHOWN, MERGED_CUT, MERGED_SHOWN};
use crate::graph::{bot, when, FoldState};
use crate::graph_ui::{claude_look, clipped, paint_pill, pill_width, wip_name};
use crate::ui::kbd;

/// The sides' padding.
pub(crate) const X: f32 = 14.0;
/// A row of the file list.
pub(crate) const FILE_H: f32 = 23.0;
/// A row of a merge's commits.
const MERGED_H: f32 = 22.0;
/// A merge's heading, with its note under it.
const MERGED_HEAD_H: f32 = 44.0;
/// A file's title over its diff.
pub(crate) const TITLE_H: f32 = 32.0;
/// A row of a diff; a long line wraps, each more line of text taking the
/// font's row height.
pub(crate) const DIFF_H: f32 = 18.0;
/// A line of a note.
pub(crate) const NOTE_H: f32 = 34.0;
/// A line number's column.
const LN_W: f32 = 38.0;
/// The `+` / `-` column.
const SG_W: f32 = 12.0;
/// The bars after a file's counts.
const BARS_W: f32 = 64.0;
/// The body's width at most: about 72 characters.
const BODY_W: f32 = 520.0;

fn scroll_id() -> Id {
    Id::new("detail-scroll")
}

/// A piece of the detail, top to bottom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Block {
    Header,
    MergedHead,
    /// The merge's `k`th commit.
    Merged(usize),
    /// How many more there are.
    MergedMore(usize),
    Gap,
    /// The `k`th file in the list.
    File(usize),
    FilesMore(usize),
    /// The `k`th file's title over its diff.
    Title(usize),
    Diff(usize),
    /// The `k`th file's diff stops short.
    Cut,
    Note(&'static str),
}

/// What a click does, once everything is drawn.
enum Act {
    Go(Oid),
    Scroll(usize),
    AllFiles,
    Wide(bool),
    /// Go to the tsumugi tab of this session.
    Claude(u64),
}

/// The blocks for `d`, whose trunk merge brought in `merged`.
fn blocks(d: &Detail, merged: usize, all_files: bool) -> Vec<Block> {
    let mut out = vec![Block::Header];
    if merged > 0 {
        out.push(Block::MergedHead);
        let n = listed(merged, MERGED_SHOWN, MERGED_CUT, false);
        out.extend((0..n).map(Block::Merged));
        if n < merged {
            out.push(Block::MergedMore(merged - n));
        }
    }
    let files = &d.files;
    if !files.is_empty() {
        out.push(Block::Gap);
        let n = listed(files.len(), FILES_SHOWN, FILES_CUT, all_files);
        out.extend((0..n).map(Block::File));
        if n < files.len() {
            out.push(Block::FilesMore(files.len() - n));
        }
        out.push(Block::Gap);
    }
    if d.parents.len() > 1 {
        // A merge's diff is the whole branch's (stage 2).
        out.push(Block::Note("detail.merge_note"));
        return out;
    }
    if files.is_empty() {
        out.push(Block::Note("detail.no_diff"));
    }
    for (k, f) in files.iter().enumerate() {
        out.push(Block::Title(k));
        if f.not_utf8 {
            out.push(Block::Note("detail.not_utf8"));
        }
        if michi_core::lfs::of_file(f).is_some() {
            out.push(Block::Note("detail.lfs_note"));
        }
        if f.is_binary() {
            out.push(Block::Note("detail.binary_note"));
        } else if f.eol_only {
            out.push(Block::Note("detail.eol_only"));
        } else if f.hunks.is_empty() && !f.cut {
            out.push(Block::Note("detail.no_diff"));
        } else if !f.hunks.is_empty() {
            out.push(Block::Diff(k));
        }
        if f.cut {
            out.push(Block::Cut);
        }
    }
    out
}

/// The detail in `r`.
pub fn show(ui: &mut Ui, app: &mut App, r: Rect, live: bool) {
    let c = ito_theme::colors();
    let ctx = ui.ctx().clone();
    ui.painter().rect_filled(r, 0.0, c.bg);
    let p = ui.painter_at(r);
    let to_top = std::mem::take(&mut app.detail.to_top);
    let scroll_to = app.detail.scroll_to.take();
    let note = |key: &str| {
        p.text(pos2(r.left() + X, r.top() + 10.0), Align2::LEFT_TOP, key, FontId::proportional(12.0), c.dim);
    };
    if app.detail.asked().is_none() {
        note(app.tr("detail.none"));
        return;
    }
    let d = match app.detail.got() {
        None => return note(app.tr("detail.reading")),
        Some(Err(e)) => return note(&app.tr("detail.failed").replace("{}", &e.to_string())),
        Some(Ok(d)) => d,
    };
    crate::select::begin(&d.id.to_string());
    let merged = app.detail.merged(d.id, app.graph.read()).unwrap_or(&[]);
    let blocks = blocks(d, merged.len(), app.detail.all_files);
    let wide = app.detail.wide;
    let mut none = None;
    let header_h = header(ui, app, d, pos2(r.left(), 0.0), r.width(), None, &mut none);
    let wrap = Wrap::new(ui, r.width(), wide);
    let height = |b: &Block| -> f32 {
        match *b {
            Block::Header => header_h,
            Block::MergedHead => MERGED_HEAD_H,
            Block::Merged(_) | Block::MergedMore(_) => MERGED_H,
            Block::Gap => 6.0,
            Block::File(_) | Block::FilesMore(_) => FILE_H,
            Block::Title(_) => TITLE_H,
            Block::Diff(k) => app.detail.rows(k).map_or(0.0, |rows| wrap.tops(rows).last().copied().unwrap_or(0.0)) + 6.0,
            Block::Cut => DIFF_H + 6.0,
            Block::Note(_) => NOTE_H,
        }
    };
    let heights: Vec<f32> = blocks.iter().map(height).collect();
    let total: f32 = heights.iter().sum::<f32>() + 24.0;

    let mut scroll = ctx.data(|dt| dt.get_temp::<f32>(scroll_id())).unwrap_or(0.0);
    if to_top {
        scroll = 0.0;
    }
    if let Some(k) = scroll_to {
        if let Some(at) = blocks.iter().position(|b| *b == Block::Title(k)) {
            scroll = heights[..at].iter().sum();
        }
    }
    if live && ui.rect_contains_pointer(r) {
        scroll -= ui.input(|i| i.smooth_scroll_delta.y);
    }
    scroll = scroll.clamp(0.0, (total - r.height()).max(0.0));
    ctx.data_mut(|dt| dt.insert_temp(scroll_id(), scroll));

    // Under the rows' own clicks: a drag over the text picks it up.
    let grab = ui.interact(r, Id::new("detail-select"), Sense::click_and_drag());
    let mut act = None;
    let pointer = if live { ui.input(|i| i.pointer.hover_pos()).filter(|at| r.contains(*at)) } else { None };
    let max_count = d.files.iter().filter_map(|f| f.counts).map(|(a, x)| a + x).max().unwrap_or(0).max(1);
    let mut y = r.top() - scroll;
    for (k, b) in blocks.iter().enumerate() {
        let h = heights[k];
        let rr = Rect::from_min_size(pos2(r.left(), y), vec2(r.width(), h));
        y += h;
        if rr.bottom() < r.top() || rr.top() > r.bottom() {
            continue;
        }
        let hot = pointer.is_some_and(|at| rr.contains(at));
        let click = |tag: &str| live && ui.interact(rr.intersect(r), Id::new(("detail", tag, k)), Sense::CLICK).clicked();
        match *b {
            Block::Header => {
                header(ui, app, d, pos2(r.left(), rr.top()), r.width(), Some((&p, live)), &mut act);
            }
            Block::MergedHead => {
                let trunk = app.graph.trunk_name().unwrap_or("main");
                let text = app.tr("detail.merged").replace("{n}", &merged.len().to_string()).replace("{trunk}", trunk);
                p.line_segment([rr.left_top(), rr.right_top()], Stroke::new(1.0, c.border));
                clipped(ui, &p, pos2(rr.left() + X, rr.top() + 16.0), &text, FontId::proportional(12.5), c.fg, rr.width() - 2.0 * X);
                let state = match app.graph.fold_state(&d.id) {
                    FoldState::Folded => Some("detail.merged_folded"),
                    FoldState::Open => Some("detail.merged_open"),
                    FoldState::Off => None,
                };
                if let Some(key) = state {
                    clipped(
                        ui,
                        &p,
                        pos2(rr.left() + X, rr.top() + 33.0),
                        app.tr(key),
                        FontId::proportional(11.5),
                        c.dim,
                        rr.width() - 2.0 * X,
                    );
                }
            }
            Block::Merged(m) => {
                let id = merged[m];
                if hot {
                    p.rect_filled(rr, 0.0, c.panel);
                    ctx.set_cursor_icon(CursorIcon::PointingHand);
                }
                let mid = rr.center().y;
                p.text(pos2(rr.left() + X, mid), Align2::LEFT_CENTER, id.short(), FontId::monospace(11.0), c.dim);
                if let Some(commit) = app.graph.find_commit(&id) {
                    let date = when(commit.time, &chrono::Local::now(), app.tr("time.yesterday"));
                    let dr = p.text(pos2(rr.right() - X, mid), Align2::RIGHT_CENTER, date, FontId::proportional(11.5), c.dim);
                    let sx = rr.left() + X + 62.0;
                    clipped(ui, &p, pos2(sx, mid), &commit.subject, FontId::proportional(12.5), c.fg, dr.left() - 8.0 - sx);
                }
                if click("merged") {
                    act = Some(Act::Go(id));
                }
            }
            Block::MergedMore(n) => {
                let text = app.tr("detail.merged_more").replace("{}", &n.to_string());
                p.text(pos2(rr.left() + X + 62.0, rr.center().y), Align2::LEFT_CENTER, text, FontId::proportional(11.5), c.dim);
            }
            Block::Gap => {}
            Block::File(f) => {
                if hot {
                    p.rect_filled(rr, 0.0, c.panel);
                    ctx.set_cursor_icon(CursorIcon::PointingHand);
                }
                file_row(ui, &p, app, &d.files[f], rr, max_count);
                if click("file") {
                    act = Some(Act::Scroll(f));
                }
            }
            Block::FilesMore(n) => {
                if hot {
                    p.rect_filled(rr, 0.0, c.panel);
                    ctx.set_cursor_icon(CursorIcon::PointingHand);
                }
                let text = app.tr("detail.files_more").replace("{}", &n.to_string());
                p.text(pos2(rr.left() + X + 16.0, rr.center().y), Align2::LEFT_CENTER, text, FontId::proportional(12.0), c.dim);
                if click("more") {
                    act = Some(Act::AllFiles);
                }
            }
            Block::Title(f) => {
                if let Some(a) = title(ui, &p, app, &d.files[f], rr, f == 0, live, pointer) {
                    act = Some(a);
                }
            }
            Block::Diff(f) => {
                if let Some(rows) = app.detail.rows(f) {
                    let body = Rect::from_min_size(rr.min, vec2(rr.width(), rr.height() - 6.0));
                    diff(ui, &p, &d.files[f], rows, body, r, &wrap, &Marks::default());
                }
            }
            Block::Cut => {
                let mut job = LayoutJob::default();
                job.append(
                    app.tr("detail.cut"),
                    0.0,
                    TextFormat { font_id: FontId::proportional(12.0), color: c.dim, italics: true, ..Default::default() },
                );
                let g = ui.fonts_mut(|f| f.layout_job(job));
                p.galley(pos2(rr.left() + X, rr.top() + (DIFF_H - g.size().y) / 2.0), g, c.dim);
            }
            Block::Note(key) => {
                let text = app.tr(key);
                p.text(pos2(rr.left() + X, rr.top() + 10.0), Align2::LEFT_TOP, text, FontId::proportional(12.0), c.dim);
            }
        }
    }

    if live {
        let driven = crate::select::drive(&grab, r);
        if let Some(text) = driven.copy {
            ctx.copy_text(text);
        }
        if let Some(url) = driven.open {
            std::thread::spawn(move || {
                let _ = michi_core::pr::open_url(&url);
            });
        }
        if ui.input(|i| i.modifiers.command) && grab.hover_pos().is_some_and(|at| crate::select::url_at(at, r)) {
            ctx.set_cursor_icon(CursorIcon::PointingHand);
        }
    }
    crate::select::paint();

    let Some(a) = act else { return };
    match a {
        Act::Go(id) => app.go_to(id),
        Act::Scroll(f) => app.detail.scroll_to = Some(f),
        Act::AllFiles => app.detail.all_files = true,
        Act::Wide(w) => app.detail.wide = w,
        Act::Claude(session) => app.act(crate::act::Task::Focus { session }),
    }
    ctx.request_repaint();
}

/// A run of the header's meta line.
struct Run {
    text: String,
    font: FontId,
    color: Color32,
    /// A parent: a click selects it.
    link: Option<Oid>,
}

/// The mark for a signed commit: its words' key and colour. Nothing for an
/// unsigned one.
fn signature_mark(s: michi_core::signature::Signature, c: &ito_theme::Colors) -> Option<(&'static str, Color32)> {
    use michi_core::signature::Signature;
    match s {
        Signature::None => None,
        Signature::Valid => Some(("detail.sig_valid", c.done)),
        Signature::Invalid => Some(("detail.sig_invalid", c.err)),
        Signature::Unchecked => Some(("detail.sig_unchecked", c.wait)),
    }
}

/// The header: the subject; the hash, the author, the date and the
/// parents; the names on the commit; the message's body; the hint. Its
/// height. Drawn at `top` with the painter, measured without.
fn header(ui: &Ui, app: &App, d: &Detail, at: Pos2, width: f32, paint: Option<(&Painter, bool)>, act: &mut Option<Act>) -> f32 {
    if let Some(w) = app.graph.selected_wip().filter(|_| app.stash_shown().is_none()) {
        return wip_header(ui, app, w, at, width, paint.map(|(p, _)| p));
    }
    let c = ito_theme::colors();
    let (left, top) = (at.x, at.y);
    let inner = (width - 2.0 * X).max(40.0);
    let (x0, right) = (left + X, left + X + inner);
    let mut y = top + 12.0;

    let title = ui.fonts_mut(|f| f.layout(d.subject.clone(), FontId::proportional(14.0), c.fg, inner));
    if let Some((p, _)) = paint {
        // No bold face: the title is drawn twice, a hair apart.
        crate::select::note(p, pos2(x0, y), &title);
        p.galley(pos2(x0, y), title.clone(), c.fg);
        p.galley(pos2(x0 + 0.45, y), title.clone(), c.fg);
    }
    y += title.size().y + 6.0;

    // The meta line, wrapped where it runs out.
    let mono = FontId::monospace(12.0);
    let prop = FontId::proportional(12.0);
    let author = bot(&d.author).unwrap_or_else(|| d.author.clone());
    let time = chrono::DateTime::from_timestamp(d.time, 0).map(|t| t.with_timezone(&chrono::Local).format("%Y-%m-%d %H:%M").to_string());
    let mut runs = vec![Run { text: d.id.to_string(), font: mono.clone(), color: c.fg, link: None }];
    runs.push(Run { text: author, font: prop.clone(), color: c.dim, link: None });
    if let Some(t) = time {
        runs.push(Run { text: t, font: prop.clone(), color: c.dim, link: None });
    }
    if let Some((key, color)) = signature_mark(d.signature, &c) {
        runs.push(Run { text: app.tr(key).to_owned(), font: prop.clone(), color, link: None });
    }
    for (k, parent) in d.parents.iter().enumerate() {
        if k == 0 {
            runs.push(Run { text: app.tr("detail.parent").to_owned(), font: prop.clone(), color: c.dim, link: None });
        }
        runs.push(Run { text: parent.short(), font: mono.clone(), color: c.blue, link: Some(*parent) });
    }
    let line_h = 18.0;
    let mut x = x0;
    for (k, run) in runs.iter().enumerate() {
        let g = ui.fonts_mut(|f| f.layout_no_wrap(run.text.clone(), run.font.clone(), run.color));
        let gap = if k == 0 { 0.0 } else { 10.0 };
        if x > x0 && x + gap + g.size().x > right {
            x = x0;
            y += line_h;
        } else {
            x += gap;
        }
        let at = Rect::from_min_size(pos2(x, y + (line_h - g.size().y) / 2.0), g.size());
        if let Some((p, live)) = paint {
            crate::select::note(p, at.min, &g);
            p.galley(at.min, g.clone(), run.color);
            if let Some(id) = run.link {
                let resp = ui.interact(at.intersect(p.clip_rect()), Id::new(("detail-parent", id.short())), Sense::CLICK);
                if live && resp.hovered() {
                    p.line_segment([at.left_bottom(), at.right_bottom()], Stroke::new(1.0, run.color));
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                }
                if live && resp.clicked() {
                    *act = Some(Act::Go(id));
                }
            }
        }
        x += g.size().x;
    }
    y += line_h + 6.0;

    // The names on it, as the graph's row has them.
    let k = app.graph.selected_row().filter(|_| app.graph.selected() == Some(d.id));
    let pills = k.map(|k| app.graph.pills(k)).unwrap_or_default();
    if !pills.is_empty() {
        let mut x = x0;
        for pill in &pills {
            let Some(w) = pill_width(ui, &pill.text, pill.kind, !pill.also.is_empty()) else { continue };
            if x > x0 && x + w > right {
                x = x0;
                y += 22.0;
            }
            if let Some((p, _)) = paint {
                paint_pill(ui, p, Rect::from_min_size(pos2(x, y + 1.0), vec2(w, 18.0)), pill, 1.0);
            }
            x += w + 4.0;
        }
        y += 22.0 + 4.0;
    }

    // The Claude of the worktree this commit is the work of: its tab and title,
    // a click goes to the tab (`tsumugi focus`). Not for the uncommitted rows.
    let made = app.claude_of(&d.id);
    if !made.is_empty() {
        let shown = made.len().min(2);
        let mut x = x0;
        let mut runs = vec![(app.tr("detail.claude").to_owned(), c.dim, None)];
        for s in &made[..shown] {
            let text = app.tr("detail.claude_tab").replace("{tab}", &s.id.to_string()).replace("{title}", s.title.trim());
            runs.push((text, c.blue, Some(s.id)));
        }
        if shown < made.len() {
            runs.push((app.tr("claude.more").replace("{}", &(made.len() - shown).to_string()), c.dim, None));
        }
        for (k, (text, color, link)) in runs.into_iter().enumerate() {
            let g = ui.fonts_mut(|f| f.layout_no_wrap(text, prop.clone(), color));
            let gap = if k == 0 { 0.0 } else { 10.0 };
            if x > x0 && x + gap + g.size().x > right {
                x = x0;
                y += line_h;
            } else {
                x += gap;
            }
            let at = Rect::from_min_size(pos2(x, y + (line_h - g.size().y) / 2.0), g.size());
            if let Some((p, live)) = paint {
                p.galley(at.min, g.clone(), color);
                if let Some(id) = link {
                    let resp = ui.interact(at.intersect(p.clip_rect()), Id::new(("detail-claude", id)), Sense::CLICK);
                    if live && resp.hovered() {
                        p.line_segment([at.left_bottom(), at.right_bottom()], Stroke::new(1.0, color));
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                    if live && resp.clicked() {
                        *act = Some(Act::Claude(id));
                    }
                }
            }
            x += g.size().x;
        }
        y += line_h + 6.0;
    }

    let body = d.body.trim();
    if !body.is_empty() {
        let g = ui.fonts_mut(|f| f.layout(body.to_owned(), FontId::proportional(12.5), c.fg, inner.min(BODY_W)));
        if let Some((p, _)) = paint {
            crate::select::note(p, pos2(x0, y), &g);
            p.galley(pos2(x0, y), g.clone(), c.fg);
        }
        y += g.size().y + 8.0;
    }

    let hint = app.tr(if app.detail.wide { "detail.hint_back" } else { "detail.hint_wide" });
    if let Some((p, _)) = paint {
        clipped(ui, p, pos2(x0, y + 8.0), hint, FontId::proportional(11.5), c.dim, inner);
    }
    y += 16.0 + 10.0;
    if let Some((p, _)) = paint {
        p.line_segment([pos2(left, y - 0.5), pos2(left + width, y - 0.5)], Stroke::new(1.0, c.border));
    }
    y - top
}

/// The header of a worktree's uncommitted change: whose it is, where and
/// when it was last written, that it is only read here, and what the Claude
/// in that worktree is doing. Its height, as [`header`]'s.
fn wip_header(ui: &Ui, app: &App, w: &Wip, at: Pos2, width: f32, paint: Option<&Painter>) -> f32 {
    let c = ito_theme::colors();
    let (left, top) = (at.x, at.y);
    let inner = (width - 2.0 * X).max(40.0);
    let x0 = left + X;
    let mut y = top + 12.0;
    let draw = |g: std::sync::Arc<egui::Galley>, at: Pos2| {
        if let Some(p) = paint {
            p.galley(at, g, c.fg);
        }
    };

    let text = app.tr("wip.title").replace("{}", &wip_name(w));
    let title = ui.fonts_mut(|f| f.layout(text, FontId::proportional(14.0), c.fg, inner));
    draw(title.clone(), pos2(x0, y));
    draw(title.clone(), pos2(x0 + 0.45, y));
    y += title.size().y + 6.0;

    // worktree <path> · last written at HH:MM
    let mut job = LayoutJob::default();
    job.wrap.max_width = inner;
    let prop = |color| TextFormat { font_id: FontId::proportional(12.0), color, ..Default::default() };
    job.append("worktree ", 0.0, prop(c.dim));
    job.append(&w.path.display().to_string(), 0.0, TextFormat { font_id: FontId::monospace(12.0), color: c.fg, ..Default::default() });
    if let Some(t) = w.written {
        let at = when(t, &chrono::Local::now(), app.tr("time.yesterday"));
        job.append(&app.tr("wip.written").replace("{}", &at), 10.0, prop(c.dim));
    }
    let meta = ui.fonts_mut(|f| f.layout_job(job));
    draw(meta.clone(), pos2(x0, y));
    y += meta.size().y + 8.0;

    let hint = ui.fonts_mut(|f| f.layout(app.tr("wip.hint").to_owned(), FontId::proportional(11.5), c.dim, inner));
    draw(hint.clone(), pos2(x0, y));
    y += hint.size().y + 10.0;

    // Untracked files too big to count, so not in the change.
    if !w.left_out.is_empty() {
        let shown = w.left_out.len().min(3);
        let mut paths = w.left_out[..shown].iter().map(|l| l.path.as_str()).collect::<Vec<_>>().join(", ");
        if shown < w.left_out.len() {
            paths.push_str(&app.tr("wip.left_more").replace("{n}", &(w.left_out.len() - shown).to_string()));
        }
        let text = app.tr("wip.left_out").replace("{}", &paths);
        let left = ui.fonts_mut(|f| f.layout(text, FontId::proportional(11.5), c.dim, inner));
        draw(left.clone(), pos2(x0, y));
        y += left.size().y + 10.0;
    }

    // The Claude there: a box with what it does and what it last said.
    let sessions = app.claude_in(&w.path);
    if let Some(s) = sessions.first() {
        let pad = 10.0;
        let mut job = LayoutJob::default();
        job.wrap.max_width = inner - 2.0 * pad;
        let lead = if s.state == State::Running { "wip.writing" } else { "wip.stopped" };
        job.append(app.tr(lead), 0.0, prop(c.fg));
        if let Some((glyph, word, ink)) = claude_look(app, s) {
            job.append(&format!("{glyph} {word}"), 6.0, prop(ink));
        }
        let since = chrono::DateTime::from_timestamp_millis(s.since_ms as i64)
            .filter(|_| s.since_ms > 0)
            .map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string());
        let at = app
            .tr("claude.at")
            .replace("{tab}", &s.id.to_string())
            .replace("{title}", &s.title)
            .replace("{since}", since.as_deref().unwrap_or("—"));
        job.append(&at, 0.0, prop(c.dim));
        if sessions.len() > 1 {
            job.append(&app.tr("claude.more").replace("{}", &(sessions.len() - 1).to_string()), 6.0, prop(c.dim));
        }
        if !s.note.is_empty() {
            job.append(
                "
",
                0.0,
                prop(c.fg),
            );
            job.append(s.note.trim(), 0.0, prop(if s.state == State::Waiting { c.wait } else { c.fg }));
        }
        let g = ui.fonts_mut(|f| f.layout_job(job));
        let bx = Rect::from_min_size(pos2(x0, y), vec2(inner, g.size().y + 2.0 * pad));
        if let Some(p) = paint {
            p.rect_filled(bx, CornerRadius::same(6), c.panel);
            p.rect_stroke(bx, CornerRadius::same(6), Stroke::new(1.0, c.border), egui::StrokeKind::Inside);
        }
        draw(g, bx.min + vec2(pad, pad));
        y = bx.bottom() + 10.0;
    }
    if let Some(p) = paint {
        p.line_segment([pos2(left, y - 0.5), pos2(left + width, y - 0.5)], Stroke::new(1.0, c.border));
    }
    y - top
}

/// A status's letter and colour.
pub(crate) fn status_ink(s: Status) -> Color32 {
    let c = ito_theme::colors();
    match s {
        Status::Added | Status::Untracked => c.done,
        Status::Deleted | Status::Unmerged => c.err,
        Status::Modified => c.wait,
        Status::Renamed | Status::Copied => c.blue,
        Status::TypeChanged | Status::Other => c.dim,
    }
}

/// A row of the file list: the status, the path (cut from the left), the
/// counts and the bars.
fn file_row(ui: &Ui, p: &Painter, app: &App, f: &File, rr: Rect, max_count: u32) {
    let c = ito_theme::colors();
    let mid = rr.center().y;
    let x = rr.left() + X;
    p.text(pos2(x + 4.0, mid), Align2::CENTER_CENTER, f.status.letter(), FontId::monospace(11.0), status_ink(f.status));
    let mut right = rr.right() - X;
    let counts_left = match f.counts {
        Some((a, dl)) => {
            let bars = Rect::from_min_size(pos2(right - BARS_W, mid - 3.5), vec2(BARS_W, 7.0));
            p.rect_filled(bars, CornerRadius::same(2), c.panel);
            let full = BARS_W * ((a + dl) as f32 / max_count as f32).min(1.0);
            let add_w = if a + dl == 0 { 0.0 } else { full * a as f32 / (a + dl) as f32 };
            if add_w > 0.0 {
                p.rect_filled(Rect::from_min_size(bars.min, vec2(add_w, 7.0)), CornerRadius::same(2), c.done);
            }
            if full - add_w > 0.0 {
                p.rect_filled(Rect::from_min_size(bars.min + vec2(add_w, 0.0), vec2(full - add_w, 7.0)), CornerRadius::same(2), c.err);
            }
            right = bars.left() - 8.0;
            let dr = p.text(pos2(right, mid), Align2::RIGHT_CENTER, format!("−{dl}"), FontId::monospace(11.0), c.err);
            let ar = p.text(pos2(dr.left() - 5.0, mid), Align2::RIGHT_CENTER, format!("+{a}"), FontId::monospace(11.0), c.done);
            ar.left()
        }
        None => p.text(pos2(right, mid), Align2::RIGHT_CENTER, app.tr("detail.binary"), FontId::proportional(11.5), c.dim).left(),
    };
    let px = x + 16.0;
    let path = left_cut(ui, &f.path, FontId::proportional(12.5), counts_left - 10.0 - px);
    p.text(pos2(px, mid), Align2::LEFT_CENTER, path, FontId::proportional(12.5), c.fg);
}

/// `text` cut from the left to fit `width`, with `…` for what is cut.
pub(crate) fn left_cut(ui: &Ui, text: &str, font: FontId, width: f32) -> String {
    let g = ui.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE));
    if g.size().x <= width {
        return text.to_owned();
    }
    let dots = ui.fonts_mut(|f| f.layout_no_wrap("…".to_owned(), font, Color32::WHITE)).size().x;
    let Some(row) = g.rows.first() else { return String::new() };
    let end = g.size().x;
    let from = row.glyphs.iter().position(|gl| end - gl.pos.x + dots <= width).unwrap_or(row.glyphs.len());
    let rest: String = text.chars().skip(from).collect();
    format!("…{rest}")
}

/// A file's title over its diff; on the first, the switch between one
/// column and side by side.
#[expect(clippy::too_many_arguments, reason = "the painter, the row and the pointer are the frame's")]
fn title(ui: &Ui, p: &Painter, app: &App, f: &File, rr: Rect, first: bool, live: bool, pointer: Option<Pos2>) -> Option<Act> {
    let c = ito_theme::colors();
    p.rect_filled(rr, 0.0, c.side);
    p.line_segment([rr.left_top(), rr.right_top()], Stroke::new(1.0, c.border));
    p.line_segment([rr.left_bottom() - vec2(0.0, 0.5), rr.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
    let mid = rr.center().y;
    let mut end = rr.right() - X;
    let mut out = None;
    if first {
        let wide = app.detail.wide;
        let (word, key) = if wide { ("detail.narrow", "Esc") } else { ("detail.wide", "Enter") };
        let cap = kbd(ui, p, pos2(end, mid), key);
        let w = p.text(pos2(cap.left() - 6.0, mid), Align2::RIGHT_CENTER, app.tr(word), FontId::proportional(12.0), c.dim);
        let hit = w.union(cap).expand(3.0);
        let resp = ui.interact(hit.intersect(p.clip_rect()), Id::new("detail-wide"), Sense::CLICK);
        if live && pointer.is_some_and(|at| hit.contains(at)) {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if live && resp.clicked() {
            out = Some(Act::Wide(!wide));
        }
        end = hit.left() - 10.0;
    }
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

/// How long lines wrap at the detail's width.
pub(crate) struct Wrap {
    wide: bool,
    /// The text's width in a row, a narrow character's and a wide one's.
    text_w: f32,
    char_w: f32,
    wide_w: f32,
    /// The height of each more line of a wrapped one.
    more_h: f32,
}

impl Wrap {
    pub(crate) fn new(ui: &Ui, width: f32, wide: bool) -> Wrap {
        let font = FontId::monospace(12.0);
        let (char_w, wide_w, more_h) = ui.fonts_mut(|f| (f.glyph_width(&font, '0'), f.glyph_width(&font, '日'), f.row_height(&font)));
        // The text's width as `unified_row` and `half_row` lay it out.
        let text_w = if wide { ((width - 1.0) / 2.0).floor() - 4.0 - LN_W - SG_W } else { width - 6.0 - 2.0 * LN_W - SG_W };
        Wrap { wide, text_w, char_w: char_w.max(1.0), wide_w: wide_w.max(1.0), more_h }
    }

    /// How many lines a line's text takes. Narrow characters alone fill a
    /// line exactly; with wide ones each line may end short of the width by
    /// less than one, so that much is kept off.
    fn lines(&self, (narrow, wide): crate::detail::Cols) -> u32 {
        if wide == 0 {
            let per = (self.text_w / self.char_w).floor().max(1.0) as u32;
            return narrow.div_ceil(per);
        }
        let text = narrow as f32 * self.char_w + wide as f32 * self.wide_w;
        (text / (self.text_w - self.wide_w).max(self.wide_w)).ceil() as u32
    }

    /// Where each row of `rows` starts, from 0, and where the last ends.
    pub(crate) fn tops(&self, rows: &crate::detail::Rows) -> Vec<f32> {
        let mut y = 0.0;
        let mut out = vec![0.0];
        for n in rows.wraps(self.wide, |c| self.lines(c)) {
            y += DIFF_H + (n - 1) as f32 * self.more_h;
            out.push(y);
        }
        out
    }
}

/// The lines of a diff the branch review marks: the cursor's and the ones
/// with a note. The other diffs pass none.
#[derive(Default)]
pub(crate) struct Marks<'a> {
    pub cursor: Option<At>,
    pub noted: &'a [At],
}

impl Marks<'_> {
    fn paint(&self, p: &Painter, rr: Rect, at: Option<At>) {
        let Some(at) = at else { return };
        let c = ito_theme::colors();
        if self.cursor == Some(at) {
            p.rect_filled(rr, 0.0, c.blue.gamma_multiply(0.22));
            p.rect_stroke(rr, 0.0, Stroke::new(1.0, c.blue), egui::StrokeKind::Inside);
        }
        if self.noted.contains(&at) {
            p.rect_filled(Rect::from_min_size(rr.min, vec2(3.0, rr.height())), 0.0, c.wait);
        }
    }
}

/// A file's diff in `body`, only the rows inside `view`.
#[expect(clippy::too_many_arguments, reason = "the painter, the diff, its place and its marks are the frame's")]
pub(crate) fn diff(ui: &Ui, p: &Painter, f: &File, rows: &crate::detail::Rows, body: Rect, view: Rect, wrap: &Wrap, marks: &Marks) {
    let wide = wrap.wide;
    let tops = wrap.tops(rows);
    let n = tops.len() - 1;
    let first = tops.partition_point(|&t| body.top() + t <= view.top()).saturating_sub(1);
    let last = tops.partition_point(|&t| body.top() + t < view.bottom()).min(n);
    let row = |k: usize| Rect::from_min_max(pos2(body.left(), body.top() + tops[k]), pos2(body.right(), body.top() + tops[k + 1]));
    let line = |(h, l): At| &f.hunks[h as usize].lines[l as usize];
    for k in first..last {
        let rr = row(k);
        if wide {
            match rows.paired[k] {
                Paired::Hunk(h) => hunk_row(p, rr, &f.hunks[h as usize].header),
                Paired::Pair(old, new) => {
                    let half = ((rr.width() - 1.0) / 2.0).floor();
                    let left = Rect::from_min_size(rr.min, vec2(half, DIFF_H));
                    let right = Rect::from_min_max(pos2(left.right() + 1.0, rr.top()), rr.max);
                    p.rect_filled(
                        Rect::from_min_max(left.right_top(), pos2(left.right() + 1.0, rr.bottom())),
                        0.0,
                        ito_theme::colors().border,
                    );
                    half_row(ui, p, left, old.map(line), true);
                    half_row(ui, p, right, new.map(line), false);
                    marks.paint(p, Rect::from_min_size(left.min, vec2(left.width(), DIFF_H)), old);
                    marks.paint(p, Rect::from_min_size(right.min, vec2(right.width(), DIFF_H)), new);
                }
            }
        } else {
            match rows.unified[k] {
                Unified::Hunk(h) => hunk_row(p, rr, &f.hunks[h as usize].header),
                Unified::Line(at) => {
                    unified_row(ui, p, rr, line(at));
                    marks.paint(p, rr, Some(at));
                }
            }
        }
    }
}

fn hunk_row(p: &Painter, rr: Rect, header: &str) {
    let c = ito_theme::colors();
    p.rect_filled(rr, 0.0, c.blue.gamma_multiply(0.09));
    let p = p.with_clip_rect(rr.intersect(p.clip_rect()));
    p.text(pos2(rr.left() + 8.0, rr.top() + DIFF_H / 2.0), Align2::LEFT_CENTER, header, FontId::monospace(12.0), c.blue);
}

/// A line's background, sign and its colour.
fn line_look(l: &Line) -> (Option<Color32>, &'static str, Color32) {
    let c = ito_theme::colors();
    match l.kind {
        Kind::Add => (Some(c.done), "+", c.done),
        Kind::Del => (Some(c.err), "-", c.err),
        Kind::Context => (None, " ", c.dim),
    }
}

fn number(p: &Painter, cell: Rect, n: u32) {
    if n == 0 {
        return;
    }
    let c = ito_theme::colors();
    p.text(
        pos2(cell.right() - 6.0, cell.top() + DIFF_H / 2.0),
        Align2::RIGHT_CENTER,
        n.to_string(),
        FontId::monospace(11.0),
        c.dim.gamma_multiply(0.7),
    );
}

/// The theme's colour for a kind of token.
pub(crate) fn tok_color(tok: michi_core::syntax::Tok) -> Color32 {
    use michi_core::syntax::Tok;
    let c = ito_theme::colors();
    match tok {
        Tok::Keyword => c.magenta,
        Tok::Str => c.done,
        Tok::Comment => c.dim,
        Tok::Number => c.wait,
        Tok::Const => c.run,
        Tok::Type => c.run,
        Tok::Func => c.blue,
    }
}

/// The text of a line, its changed words marked, wrapped at `width` in as
/// many lines as `rows` has room for: `at` is the left of the first line's
/// middle.
fn line_text(ui: &Ui, p: &Painter, at: Pos2, l: &Line, mark: Option<Color32>, width: f32, rows: Rect) {
    if width < 8.0 {
        return;
    }
    let c = ito_theme::colors();
    let font = FontId::monospace(12.0);
    let mut job = LayoutJob::default();
    for (text, marked, tok) in l.styled() {
        let color = tok.map_or(c.fg, tok_color);
        let background = match (marked, mark) {
            (true, Some(m)) => m.gamma_multiply(0.34),
            _ => Color32::TRANSPARENT,
        };
        job.append(&text, 0.0, TextFormat { font_id: font.clone(), color, background, ..Default::default() });
    }
    let first_h = ui.fonts_mut(|f| f.row_height(&font));
    let more = ((rows.bottom() - at.y - first_h / 2.0) / first_h).round().max(0.0) as usize;
    job.wrap = TextWrapping { max_width: width, max_rows: 1 + more, break_anywhere: true, overflow_character: Some('…') };
    let g = ui.fonts_mut(|f| f.layout_job(job));
    let top = pos2(at.x, at.y - first_h / 2.0);
    crate::select::note(p, top, &g);
    p.galley(top, g, c.fg);
}

fn unified_row(ui: &Ui, p: &Painter, rr: Rect, l: &Line) {
    let (bg, sign, ink) = line_look(l);
    if let Some(bg) = bg {
        p.rect_filled(rr, 0.0, bg.gamma_multiply(0.13));
    }
    let old = Rect::from_min_size(rr.min, vec2(LN_W, DIFF_H));
    let new = Rect::from_min_size(old.right_top(), vec2(LN_W, DIFF_H));
    number(p, old, l.old);
    number(p, new, l.new);
    let sx = new.right();
    let mid = rr.top() + DIFF_H / 2.0;
    p.text(pos2(sx + SG_W / 2.0, mid), Align2::CENTER_CENTER, sign, FontId::monospace(12.0), ink);
    let tx = sx + SG_W;
    line_text(ui, p, pos2(tx, mid), l, bg, rr.right() - 6.0 - tx, rr);
}

/// One side of a row side by side; nothing on this side is hatched.
fn half_row(ui: &Ui, p: &Painter, rr: Rect, l: Option<&Line>, old: bool) {
    let c = ito_theme::colors();
    let Some(l) = l else {
        let ink = c.fg.gamma_multiply(0.04);
        p.rect_filled(rr, 0.0, ink);
        let hp = p.with_clip_rect(rr.intersect(p.clip_rect()));
        let mut x = rr.left() - DIFF_H;
        while x < rr.right() {
            hp.line_segment([pos2(x, rr.bottom()), pos2(x + DIFF_H, rr.top())], Stroke::new(1.0, ink));
            x += 6.0;
        }
        return;
    };
    let (bg, sign, ink) = line_look(l);
    if let Some(bg) = bg {
        p.rect_filled(rr, 0.0, bg.gamma_multiply(0.13));
    }
    let ln = Rect::from_min_size(rr.min, vec2(LN_W, DIFF_H));
    number(p, ln, if old { l.old } else { l.new });
    let mid = rr.top() + DIFF_H / 2.0;
    p.text(pos2(ln.right() + SG_W / 2.0, mid), Align2::CENTER_CENTER, sign, FontId::monospace(12.0), ink);
    let tx = ln.right() + SG_W;
    line_text(ui, p, pos2(tx, mid), l, bg, rr.right() - 4.0 - tx, rr);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detail::fixture::detail;

    #[test]
    fn the_blocks_follow_the_commit() {
        let a = Oid::parse(&"a".repeat(40)).unwrap();
        let b = Oid::parse(&"b".repeat(40)).unwrap();
        let d = detail(a, vec![b], 4);
        let got = blocks(&d, 0, false);
        assert_eq!(got[..7], [Block::Header, Block::Gap, Block::File(0), Block::File(1), Block::File(2), Block::File(3), Block::Gap]);
        assert_eq!(
            got[7..],
            [
                Block::Title(0),
                Block::Note("detail.not_utf8"),
                Block::Diff(0),
                Block::Title(1),
                Block::Diff(1),
                Block::Title(2),
                Block::Note("detail.binary_note"),
                Block::Title(3),
                Block::Diff(3),
                Block::Cut,
            ]
        );
        // Many files: the list is cut until it is asked for whole.
        let d = detail(a, vec![b], 13);
        assert!(blocks(&d, 0, false).contains(&Block::FilesMore(3)));
        assert!(!blocks(&d, 0, true).iter().any(|b| matches!(b, Block::FilesMore(_))));
        // A merge: its commits, its files, and a note for the diff.
        let d = detail(a, vec![b, b], 1);
        let got = blocks(&d, 9, false);
        assert_eq!(got[..2], [Block::Header, Block::MergedHead]);
        assert!(got.contains(&Block::Merged(6)) && !got.contains(&Block::Merged(7)));
        assert!(got.contains(&Block::MergedMore(2)));
        assert_eq!(got.last(), Some(&Block::Note("detail.merge_note")));
    }

    #[test]
    fn paths_are_cut_from_the_left() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let font = FontId::proportional(12.5);
            assert_eq!(left_cut(ui, "a/b.rs", font.clone(), 500.0), "a/b.rs");
            let cut = left_cut(ui, "crates/michi/src/very/deep/folder/name.rs", font.clone(), 90.0);
            assert!(cut.starts_with('…') && cut.ends_with("name.rs"), "{cut}");
            let w = ui.fonts_mut(|f| f.layout_no_wrap(cut.clone(), font.clone(), Color32::WHITE)).size().x;
            assert!(w <= 90.5, "{cut} is {w} wide");
        });
        out.textures_delta.clear();
    }

    #[test]
    fn the_detail_draws_in_every_state() {
        use crate::graph::fixture;
        use crate::repo::Answer;
        let ctx = egui::Context::default();
        let mut app = App::new(crate::prefs::Prefs::empty(), "C:/dev/x".into());
        app.lang = "en".into();
        let frame = |app: &mut App, width: f32| {
            let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(width, 500.0))), ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| {
                let r = ui.max_rect();
                show(ui, app, r, true);
            });
            out.textures_delta.clear();
        };
        frame(&mut app, 440.0);
        app.take(Answer::Setup { gen: 1, setup: Box::new(fixture::setup()) });
        app.take(Answer::Commits { gen: 1, batch: fixture::history(), done: true });
        app.sync_detail();
        frame(&mut app, 440.0);
        let m2 = app.graph.selected().unwrap();
        app.take(Answer::Detail { id: m2, result: Ok(Box::new(detail(m2, vec![fixture::id(5)], 13))) });
        frame(&mut app, 440.0);
        app.detail.scroll_to = Some(3);
        frame(&mut app, 440.0);
        assert!(ctx.data(|d| d.get_temp::<f32>(scroll_id())).unwrap() > 0.0, "scrolled to the fourth file");
        app.detail.wide = true;
        frame(&mut app, 900.0);
        frame(&mut app, 200.0);
        // A trunk merge, its commits listed.
        app.go_to(fixture::id(5));
        app.sync_detail();
        app.take(Answer::Detail {
            id: fixture::id(5),
            result: Ok(Box::new(detail(fixture::id(5), vec![fixture::id(4), fixture::id(3)], 2))),
        });
        frame(&mut app, 440.0);
        assert_eq!(ctx.data(|d| d.get_temp::<f32>(scroll_id())), Some(0.0), "a new commit starts at its top");
        app.graph.select(0);
        app.sync_detail();
        app.take(Answer::Detail { id: m2, result: Err(michi_core::git::Error::Git("bad object".into())) });
        frame(&mut app, 440.0);
        // A worktree's uncommitted change, with and without its Claude.
        let mut wip = fixture::wip("C:/r/wt", Some("wip"), 7, 11, Some(1_767_225_600));
        // Untracked folders too big to count, more than are listed.
        let left = |path: &str| michi_core::predict::LeftOut { path: path.into(), files: 2001, bytes: 1 << 30 };
        wip.left_out = vec![left("target-v/"), left("dist/"), left("a.iso"), left("b.iso")];
        app.take(Answer::Wips(vec![wip]));
        app.graph.select(0);
        app.sync_detail();
        frame(&mut app, 440.0);
        let w = fixture::id(11);
        app.take(Answer::Detail { id: w, result: Ok(Box::new(detail(w, vec![fixture::id(7)], 2))) });
        frame(&mut app, 440.0);
        let session = |id, state| michi_core::claude::Session {
            id,
            state,
            command: "claude".into(),
            agent: "claude".into(),
            cwd: "C:/r/wt".into(),
            branch: "wip".into(),
            title: "fix the parser".into(),
            note: "Allow edits to src/lib.rs?".into(),
            since_ms: 1_767_225_600_000,
            read_only: false,
        };
        let sessions = vec![session(3, State::Waiting), session(5, State::Running)];
        app.take(Answer::Claude(Some(vec![("C:/r/wt".into(), sessions)])));
        frame(&mut app, 440.0);
        frame(&mut app, 200.0);
    }
}
