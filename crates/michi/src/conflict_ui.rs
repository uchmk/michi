//! The `4` view (docs/design.md「衝突」): at the left the files that clashed,
//! those merged by themselves and what the worktree is in the middle of; at
//! the right a bar over each file's places, the lines around them, and the
//! commit once all is solved.

use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Galley, Id, Painter, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui,
};
use ito_theme::mix;
use michi_core::branch::Doing;
use michi_core::conflict::{Block, Body, Conflict, Hint, Pick, Side, Whole, Why};
use michi_core::paths;

use crate::act;
use crate::app::App;
use crate::conflicts::{places, Place, Tag};
use crate::detail_ui::left_cut;
use crate::graph_ui::clipped;
use crate::ui::kbd;

const BAR_H: f32 = 40.0;
const ABORT_H: f32 = 34.0;
const SIDE_W: f32 = 240.0;
const SIDE_NARROW: f32 = 200.0;
/// Below this width the side column is narrower.
const NARROW: f32 = 900.0;
/// From this width the places show their base by themselves.
const WIDE: f32 = 960.0;
const PAD_X: f32 = 14.0;
const PAD_TOP: f32 = 12.0;
const PAD_BOTTOM: f32 = 40.0;
/// Between the pieces of a file.
const GAP: f32 = 6.0;
/// Between the files.
const GAP_FILE: f32 = 12.0;
const FILE_H: f32 = 28.0;
const SKIP_H: f32 = 20.0;
const LINE: f32 = 18.0;
/// A row of the field a result is written in.
const EDIT_LINE: f32 = 16.0;
const HEAD: f32 = 34.0;
const COL_HEAD: f32 = 22.0;
const RES_HEAD: f32 = 24.0;
const HINT_H: f32 = 18.0;
const WHY_H: f32 = 30.0;
const BTN_H: f32 = 25.0;
/// The line numbers' column in a place, and around it.
const NUM_W: f32 = 36.0;
const CTX_NUM_W: f32 = 44.0;

fn side_scroll_id() -> Id {
    Id::new("conflict-side-scroll")
}

fn scroll_id() -> Id {
    Id::new("conflict-scroll")
}

/// The commit's message, for Ctrl+Enter.
/// The field a result is written in by hand.
pub fn edit_id() -> Id {
    Id::new("conflict-edit")
}

pub fn message_id() -> Id {
    Id::new("conflict-message")
}

/// What a click does, once everything is drawn.
enum Act {
    /// The place, an index into the places.
    Select(usize),
    /// The file's first place still to solve.
    SelectFile(usize),
    /// Another worktree stopped on a clash.
    Show(PathBuf),
    Pick(usize, Pick),
    Base(Place),
    Restart(usize),
    AskAbort,
    AbortNo,
    AbortYes,
    Commit,
}

/// The `4` view in `r`.
pub fn show(ui: &mut Ui, app: &mut App, r: Rect, live: bool) {
    let c = ito_theme::colors();
    let ctx = ui.ctx().clone();
    let p = ui.painter_at(r);
    p.rect_filled(r, 0.0, c.bg);
    let pages = std::mem::take(&mut app.conflicts.pages);
    let follow = std::mem::take(&mut app.conflicts.follow);
    let Some(cf) = app.conflicts.shown() else {
        app.conflicts.typing = false;
        app.conflicts.focus_message = false;
        let g = wrapped(ui, app.tr("cf.none").to_owned(), FontId::proportional(13.0), c.dim, r.width() - 80.0);
        p.galley(r.center() - g.size() / 2.0, g, c.dim);
        return;
    };
    let pointer = if live { ui.input(|i| i.pointer.hover_pos()).filter(|at| r.contains(*at)) } else { None };
    let side_w = if r.width() < NARROW { SIDE_NARROW } else { SIDE_W }.min(r.width() * 0.35);
    let sr = Rect::from_min_max(r.min, pos2(r.left() + side_w, r.bottom()));
    let mr = Rect::from_min_max(pos2(sr.right(), r.top()), r.max);
    let d = Draw { ui, app, cf, live, pointer, wide: mr.width() >= WIDE, field: Default::default() };
    let mut act = d.side(sr);
    let bar = Rect::from_min_size(mr.min, vec2(mr.width(), BAR_H));
    if let Some(a) = d.bar(bar) {
        act = Some(a);
    }
    let mut top = bar.bottom();
    if app.conflicts.abort {
        let (a, h) = d.abort_row(pos2(mr.left(), top), mr.width());
        if a.is_some() {
            act = a;
        }
        top += h;
    }
    let body = Rect::from_min_max(pos2(mr.left(), top), mr.max);
    let (a, field) = d.body(body, pages, follow);
    let written = d.field.take();
    if a.is_some() {
        act = a;
    }

    // The message, once all is solved; put while out of sight too, so it
    // keeps the keys.
    match field {
        Some((field, clip)) => {
            let hint = app.tr("cf.message_hint");
            let edit = egui::TextEdit::multiline(&mut app.conflicts.message)
                .id(message_id())
                .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(8, 6)))
                .hint_text(hint)
                .font(FontId::monospace(12.0))
                .text_color(c.fg)
                .desired_width(field.width())
                .desired_rows(1)
                .interactive(live);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field));
            child.set_clip_rect(clip);
            let resp = child.put(field, edit);
            app.conflicts.typing = resp.has_focus();
            if std::mem::take(&mut app.conflicts.focus_message) {
                resp.request_focus();
            }
        }
        None => {
            app.conflicts.typing = false;
            app.conflicts.focus_message = false;
        }
    }

    match (written, app.conflicts.edit.as_mut()) {
        (Some((field, clip)), Some(e)) => {
            let edit = egui::TextEdit::multiline(&mut e.text)
                .id(edit_id())
                .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(6, 2)))
                .font(FontId::monospace(12.0))
                .text_color(c.fg)
                .desired_width(field.width())
                .desired_rows(1)
                .interactive(live);
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field));
            child.set_clip_rect(clip);
            let resp = child.put(field, edit);
            if std::mem::take(&mut e.focus) {
                resp.request_focus();
            }
        }
        (None, Some(e)) => e.focus = true,
        _ => {}
    }
    app.conflict_want_intent();
    let Some(a) = act else { return };
    let cfs = &mut app.conflicts;
    match a {
        Act::Select(k) => cfs.at = k,
        Act::SelectFile(i) => {
            if let Some(c) = cfs.shown() {
                let all = places(c);
                let open = all.iter().position(|&p| p.0 == i && crate::conflicts::is_open(c, p));
                if let Some(k) = open.or_else(|| all.iter().position(|&p| p.0 == i)) {
                    cfs.at = k;
                    cfs.follow = true;
                }
            }
        }
        Act::Show(wt) => {
            cfs.show(&wt);
            cfs.follow = true;
        }
        Act::Pick(k, pick) => app.conflict_pick_at(k, pick),
        Act::Base(place) => {
            let wide = mr.width() >= WIDE;
            cfs.toggle_base(place, wide);
        }
        Act::Restart(i) => app.conflict_restart(i),
        Act::AskAbort => app.conflict_ask_abort(),
        Act::AbortNo => cfs.abort = false,
        Act::AbortYes => app.conflict_abort(),
        Act::Commit => app.conflict_commit(),
    }
    ctx.request_repaint();
}

/// What a frame draws with.
struct Draw<'a> {
    ui: &'a Ui,
    app: &'a App,
    cf: &'a Conflict,
    live: bool,
    pointer: Option<Pos2>,
    /// Wide enough for the base column by itself.
    wide: bool,
    /// Where the field for a result written by hand goes, and what of it
    /// is in sight.
    field: std::cell::Cell<Option<(Rect, Rect)>>,
}

/// A row of the side column.
enum Row {
    Head(&'static str, String),
    File(usize),
    Auto(usize),
    About(Arc<Galley>),
    Tree(PathBuf, usize),
    Gap,
}

impl Row {
    fn height(&self) -> f32 {
        match self {
            Row::Head(..) => 28.0,
            Row::File(_) | Row::Tree(..) => 23.0,
            Row::Auto(_) => 38.0,
            Row::About(g) => g.size().y + 6.0,
            Row::Gap => 8.0,
        }
    }
}

/// A piece of the places' column, top to bottom.
#[derive(Clone, Copy)]
enum Kind {
    File(usize),
    /// Lines left out, the same on both sides.
    Skip(usize),
    /// The lines before (or `after`) the block `j` of the file `i`, the
    /// first numbered `n`.
    Lines {
        i: usize,
        j: usize,
        after: bool,
        n: usize,
    },
    /// The place `k`: the block `j` of the file `i`, its first line
    /// numbered `n`.
    Block {
        k: usize,
        i: usize,
        j: usize,
        n: usize,
    },
    /// The place `k`: the file `i`, which clashed whole.
    Whole {
        k: usize,
        i: usize,
    },
    /// All is solved: the message and the commit.
    Done,
}

struct Item {
    kind: Kind,
    y: f32,
    h: f32,
}

/// A column of a place: its word in its colour and the note after it, its
/// lines (none: not known) on their fill, and the word for no lines.
struct Col<'a> {
    word: String,
    ink: Color32,
    note: String,
    lines: Option<&'a [String]>,
    /// The words of each line that the reference does not have.
    marks: Vec<Vec<Range<usize>>>,
    fill: Color32,
    empty: &'static str,
}

/// How a place is drawn.
struct Plan {
    /// Picked here, and not read again yet.
    waits: bool,
    /// Still to solve, and not waited for: its buttons show.
    open: bool,
    base: bool,
    /// The columns' rows.
    rows: usize,
    tag: Option<Tag>,
    /// What it is solved with, or will be.
    result: Option<Vec<String>>,
    buttons: Vec<(&'static str, &'static str, Pick)>,
    /// What each side meant by it, once read: one row for each side.
    intent: Option<michi_core::conflict::Intent>,
    /// The rows of the field its result is being written in.
    writing: Option<usize>,
}

impl Plan {
    /// The height of the result's box.
    fn result_px(&self) -> f32 {
        match self.writing {
            Some(n) => n as f32 * EDIT_LINE + 12.0,
            None => self.result_rows() as f32 * LINE + 12.0,
        }
    }

    fn intent_h(&self) -> f32 {
        if self.intent.is_some() {
            2.0 * LINE + 6.0
        } else {
            0.0
        }
    }

    fn result_rows(&self) -> usize {
        self.result.as_ref().map_or(1, |r| r.len().max(1))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Look {
    Plain,
    Danger,
    Prime,
    Tool,
}

impl Draw<'_> {
    fn tr(&self, key: &'static str) -> &'static str {
        self.app.tr(key)
    }

    /// The left column: the files, those merged by themselves, what the
    /// worktree is in the middle of, and the other worktrees stopped.
    fn side(&self, sr: Rect) -> Option<Act> {
        let (ui, cf) = (self.ui, self.cf);
        let c = ito_theme::colors();
        let p = ui.painter_at(sr);
        p.rect_filled(sr, 0.0, c.side);
        p.line_segment([pos2(sr.right() - 0.5, sr.top()), pos2(sr.right() - 0.5, sr.bottom())], Stroke::new(1.0, c.border));
        let width = sr.width() - 1.0;
        let mut rows = vec![Row::Head(self.tr("cf.files"), cf.files.len().to_string())];
        rows.extend((0..cf.files.len()).map(Row::File));
        if !cf.auto.is_empty() {
            rows.push(Row::Gap);
            rows.push(Row::Head(self.tr("cf.auto"), cf.auto.len().to_string()));
            rows.extend((0..cf.auto.len()).map(Row::Auto));
        }
        rows.push(Row::Gap);
        rows.push(Row::Head(self.tr(head_key(cf.doing)), String::new()));
        for line in self.about() {
            rows.push(Row::About(wrapped(ui, line, FontId::proportional(11.5), c.dim, width - 26.0)));
        }
        let me = paths::key(&cf.wt);
        let others: Vec<&Conflict> = self.app.conflicts.all.iter().filter(|o| paths::key(&o.wt) != me).collect();
        if !others.is_empty() {
            rows.push(Row::Gap);
            rows.push(Row::Head(self.tr("cf.trees"), others.len().to_string()));
            rows.extend(others.iter().map(|o| Row::Tree(o.wt.clone(), o.left())));
        }

        let ctx = ui.ctx();
        let total = 10.0 + rows.iter().map(Row::height).sum::<f32>() + 12.0;
        let mut scroll = ctx.data(|d| d.get_temp::<f32>(side_scroll_id())).unwrap_or(0.0);
        if self.live && ui.rect_contains_pointer(sr) {
            scroll -= ui.input(|i| i.smooth_scroll_delta.y);
        }
        scroll = scroll.clamp(0.0, (total - sr.height()).max(0.0));
        ctx.data_mut(|d| d.insert_temp(side_scroll_id(), scroll));

        let pointer = self.pointer.filter(|at| sr.contains(*at));
        let at_file = places(cf).get(self.app.conflicts.at).map(|p| p.0);
        let mut out = None;
        let mut y = sr.top() + 10.0 - scroll;
        for (k, row) in rows.iter().enumerate() {
            let rr = Rect::from_min_size(pos2(sr.left(), y), vec2(width, row.height()));
            y = rr.bottom();
            if rr.bottom() < sr.top() || rr.top() > sr.bottom() {
                continue;
            }
            let right = rr.right() - 12.0;
            let mid = rr.center().y;
            match row {
                Row::Head(title, n) => {
                    let mid = rr.top() + 8.0 + 13.0 / 2.0;
                    let count = p.text(pos2(right, mid), Align2::RIGHT_CENTER, n, FontId::monospace(10.5), c.dim);
                    let title: String = title.to_uppercase().chars().flat_map(|ch| [ch, '\u{200A}']).collect();
                    let w = if n.is_empty() { right } else { count.left() - 6.0 } - rr.left() - 12.0;
                    clipped(ui, &p, pos2(rr.left() + 12.0, mid), title.trim_end(), FontId::proportional(11.0), c.dim, w);
                }
                Row::File(i) => {
                    let f = &cf.files[*i];
                    let hot = pointer.is_some_and(|at| rr.contains(at));
                    if at_file == Some(*i) {
                        p.rect_filled(rr, 0.0, c.selection());
                    } else if hot {
                        p.rect_filled(rr, 0.0, c.panel);
                    }
                    let left = f.left();
                    let (n, ink) = if left > 0 { (left.to_string(), c.err) } else { ("✓".to_owned(), c.done) };
                    let count = p.text(pos2(right, mid), Align2::RIGHT_CENTER, n, FontId::monospace(11.0), ink);
                    let path = left_cut(ui, &f.path, FontId::proportional(12.5), count.left() - 8.0 - rr.left() - 14.0);
                    p.text(pos2(rr.left() + 14.0, mid), Align2::LEFT_CENTER, path, FontId::proportional(12.5), c.fg);
                    if clicked(ui, &p, rr, ("cf-side-file", k), self.live, pointer) {
                        out = Some(Act::SelectFile(*i));
                    }
                }
                Row::Auto(i) => {
                    let a = &cf.auto[*i];
                    let top = rr.top() + 11.5;
                    let count = p.text(pos2(right, top), Align2::RIGHT_CENTER, "✓", FontId::monospace(11.0), c.done);
                    let path = left_cut(ui, &a.path, FontId::proportional(12.5), count.left() - 8.0 - rr.left() - 14.0);
                    p.text(pos2(rr.left() + 14.0, top), Align2::LEFT_CENTER, path, FontId::proportional(12.5), c.fg);
                    let why = self.tr(if a.both { "cf.auto_both" } else { "cf.auto_theirs" });
                    clipped(
                        ui,
                        &p,
                        pos2(rr.left() + 30.0, rr.top() + 28.0),
                        why,
                        FontId::proportional(11.0),
                        c.dim,
                        right - rr.left() - 30.0,
                    );
                }
                Row::About(g) => {
                    p.galley(pos2(rr.left() + 14.0, rr.top() + 3.0), g.clone(), c.dim);
                }
                Row::Tree(wt, left) => {
                    let hot = pointer.is_some_and(|at| rr.contains(at));
                    if hot {
                        p.rect_filled(rr, 0.0, c.panel);
                    }
                    let (n, ink) = if *left > 0 { (left.to_string(), c.err) } else { ("✓".to_owned(), c.done) };
                    let count = p.text(pos2(right, mid), Align2::RIGHT_CENTER, n, FontId::monospace(11.0), ink);
                    let name = act::name(wt);
                    clipped(
                        ui,
                        &p,
                        pos2(rr.left() + 14.0, mid),
                        &name,
                        FontId::proportional(12.5),
                        c.fg,
                        count.left() - 8.0 - rr.left() - 14.0,
                    );
                    if clicked(ui, &p, rr, ("cf-side-tree", k), self.live, pointer) {
                        out = Some(Act::Show(wt.clone()));
                    }
                }
                Row::Gap => {}
            }
        }
        out
    }

    /// What the worktree is in the middle of, in a few lines.
    fn about(&self) -> Vec<String> {
        let cf = self.cf;
        let (ours, theirs) = (label(&cf.ours), label(&cf.theirs));
        // The id once: a side without a name is called by it.
        let short = |s: &Side| s.id.as_ref().map(|o| o.short()).filter(|id| *id != label(s)).unwrap_or_default();
        let mut out = vec![self
            .tr("cf.from_into")
            .replace("{theirs}", &theirs)
            .replace("{tid}", &short(&cf.theirs))
            .replace("{ours}", &ours)
            .replace("{oid}", &short(&cf.ours))
            .replace("  ", " ")];
        if let (Doing::Merge, Some(base)) = (cf.doing, &cf.base) {
            out.push(self.tr("cf.base_at").replace("{base}", &base.short()));
        }
        if let Some(t) = cf.started.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()) {
            let when = crate::graph::when(t.as_secs() as i64, &chrono::Local::now(), self.tr("time.yesterday"));
            out.push(self.tr("cf.since").replace("{time}", &when));
        }
        out.push(self.tr("cf.who").replace("{ours}", &ours).replace("{theirs}", &theirs));
        out
    }

    /// The bar over the places: what is going on, how much is left, and
    /// the button that calls it all off.
    fn bar(&self, r: Rect) -> Option<Act> {
        let (ui, cf) = (self.ui, self.cf);
        let c = ito_theme::colors();
        let p = ui.painter_at(r);
        p.rect_filled(r, 0.0, c.panel);
        p.line_segment([r.left_bottom() - vec2(0.0, 0.5), r.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
        let mid = r.center().y;
        let label_off = self.tr(match cf.doing {
            Doing::Merge | Doing::Bisect => "cf.abort_merge",
            Doing::CherryPick => "cf.abort_cherry",
            Doing::Revert => "cf.abort_revert",
            Doing::Rebase => "cf.abort_rebase",
        });
        let w = button_w(ui, label_off, "", Look::Danger);
        let b = Rect::from_min_max(pos2(r.right() - PAD_X - w, mid - BTN_H / 2.0), pos2(r.right() - PAD_X, mid + BTN_H / 2.0));
        let mut out = None;
        if button(ui, &p, b, label_off, "", Look::Danger, "cf-abort", self.live, self.pointer) {
            out = Some(Act::AskAbort);
        }

        let (left, total) = (cf.left(), cf.total());
        let (pill, ink) = if left > 0 {
            (self.tr("cf.pill_left").replace("{left}", &left.to_string()).replace("{total}", &total.to_string()), c.err)
        } else {
            (self.tr("cf.pill_done").to_owned(), c.done)
        };
        let pg = galley(ui, pill, FontId::proportional(12.0), ink);
        let pill_w = pg.size().x + 16.0;
        let x = r.left() + PAD_X;
        let room = b.left() - 16.0 - pill_w - 10.0 - x;
        let key = match cf.doing {
            Doing::Merge | Doing::Bisect => "cf.bar_merge",
            Doing::CherryPick => "cf.bar_cherry",
            Doing::Revert => "cf.bar_revert",
            Doing::Rebase => "cf.bar_rebase",
        };
        let (from, into) = (label(&cf.theirs), label(&cf.ours));
        let font = FontId::proportional(13.0);
        let mut job = rich(self.tr(key), &[("{from}", &from, c.done), ("{into}", &into, c.run)], &font, c.fg);
        job.wrap = TextWrapping::truncate_at_width(room.max(0.0));
        let g = ui.fonts_mut(|f| f.layout_job(job));
        let at = pos2(x, mid - g.size().y / 2.0);
        p.galley(at, g.clone(), c.fg);
        p.galley(at + vec2(0.45, 0.0), g.clone(), c.fg);
        let pr = Rect::from_min_size(pos2(x + g.size().x + 10.0, mid - 10.0), vec2(pill_w, 20.0));
        p.rect_stroke(pr, CornerRadius::same(10), Stroke::new(1.0, ink), StrokeKind::Inside);
        p.galley(pos2(pr.left() + 8.0, mid - pg.size().y / 2.0), pg, ink);
        out
    }

    /// The row at `top` that asks before it all is called off, `y`, `n`:
    /// what was chosen and its height (the words wrap).
    fn abort_row(&self, top: Pos2, width: f32) -> (Option<Act>, f32) {
        let (ui, cf) = (self.ui, self.cf);
        let c = ito_theme::colors();
        let font = FontId::proportional(12.5);
        let answers = [("n", "cf.keep_on", Act::AbortNo), ("y", "cf.call_off", Act::AbortYes)];
        let size = |s: &str, font: &FontId| galley(ui, s.to_owned(), font.clone(), c.fg).size().x;
        let keys_w: f32 =
            answers.iter().map(|(key, word, _)| size(self.tr(word), &font) + size(key, &FontId::monospace(10.5)) + 34.0).sum();
        let cmd = match cf.doing {
            Doing::Merge => "merge --abort",
            Doing::Rebase => "rebase --abort",
            Doing::CherryPick => "cherry-pick --abort",
            Doing::Revert => "revert --abort",
            Doing::Bisect => "bisect reset",
        };
        let changed = if cf.doing == Doing::Rebase { &cf.theirs } else { &cf.ours };
        let mut text = self
            .tr("cf.abort_ask")
            .replace("{doing}", &self.app.doing_word(cf.doing))
            .replace("{ours}", &label(changed))
            .replace("{cmd}", cmd);
        let solved = cf.total() - cf.left();
        if solved > 0 {
            text.push_str(&self.tr("cf.abort_lost").replace("{}", &solved.to_string()));
        }
        text.push_str(self.tr("cf.abort_kept"));
        let left = top.x + PAD_X;
        let g = wrapped(ui, text, font.clone(), c.fg, (width - 2.0 * PAD_X - 18.0 - keys_w).max(40.0));
        let r = Rect::from_min_size(top, vec2(width, (g.size().y + 16.0).max(ABORT_H)));
        let p = ui.painter_at(r);
        p.rect_filled(r, 0.0, c.err.gamma_multiply(0.14));
        p.line_segment([r.left_bottom() - vec2(0.0, 0.5), r.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
        let mid = r.center().y;
        let at = pos2(left + 18.0, mid - g.size().y / 2.0);
        let first = ui.fonts_mut(|f| f.row_height(&font)) / 2.0;
        p.text(pos2(left, at.y + first), Align2::LEFT_CENTER, "●", FontId::proportional(12.0), c.err);
        p.galley(at, g, c.fg);
        let mut right = r.right() - PAD_X;
        let mut out = None;
        for (key, word, a) in answers {
            let cap = kbd(ui, &p, pos2(right, mid), key);
            let w = p.text(pos2(cap.left() - 6.0, mid), Align2::RIGHT_CENTER, self.tr(word), font.clone(), c.fg);
            let hit = w.union(cap).expand(3.0);
            if clicked(ui, &p, hit, ("cf-abort-row", key), self.live, self.pointer) {
                out = Some(a);
            }
            right = hit.left() - 14.0;
        }
        (out, r.height())
    }

    /// How the place `k` (the block `b` at `place`) is drawn.
    fn plan(&self, place: Place, b: &Block) -> Plan {
        let cfs = &self.app.conflicts;
        let waits = cfs.waits(place);
        let open = b.is_open() && !waits;
        let base = cfs.base_shown(place, self.wide);
        let base_rows = if base { b.base.as_ref().map_or(1, Vec::len) } else { 0 };
        let rows = b.ours.len().max(b.theirs.len()).max(base_rows).max(1);
        let tag = cfs.tag(place);
        let result = match (&b.result, tag) {
            (Some(r), _) => Some(r.clone()),
            (None, Some(Tag::Picked(pick))) if waits => b.lines(pick),
            _ => None,
        };
        let mut buttons = Vec::new();
        if open {
            buttons.push((self.tr("cf.pick_ours"), "o", Pick::Ours));
            buttons.push((self.tr("cf.pick_theirs"), "t", Pick::Theirs));
            buttons.push((self.tr("cf.pick_both"), "b", Pick::Both));
            buttons.push((self.tr("cf.pick_both2"), "B", Pick::BothTheirsFirst));
            if b.base.as_ref().is_some_and(|l| !l.is_empty()) {
                buttons.push((self.tr("cf.pick_base"), "", Pick::Base));
            }
        }
        let intent = cfs.intent(place).filter(|i| i.ours.is_some() || i.theirs.is_some()).cloned();
        let writing = cfs.edit.as_ref().filter(|_| cfs.editing(place)).map(|e| e.rows());
        Plan { waits, open, base, rows, tag, result, buttons, intent, writing }
    }

    /// The buttons of a file that clashed whole, while it is to be solved.
    fn whole_buttons(&self, w: &Whole) -> Vec<(&'static str, &'static str, Pick)> {
        let ours = if w.ours { "cf.take_ours" } else { "cf.take_ours_gone" };
        let theirs = if w.theirs { "cf.take_theirs" } else { "cf.take_theirs_gone" };
        vec![(self.tr(ours), "o", Pick::Ours), (self.tr(theirs), "t", Pick::Theirs)]
    }

    fn block_h(&self, plan: &Plan, width: f32) -> f32 {
        let mut h = HEAD + plan.intent_h() + COL_HEAD + 8.0 + plan.rows as f32 * LINE + 1.0;
        h += RES_HEAD + plan.result_px() + 8.0;
        if plan.open {
            h += HINT_H + 8.0 + flow_h(self.ui, &plan.buttons, width - 20.0) + 10.0;
        }
        h
    }

    fn whole_h(&self, buttons: &[(&'static str, &'static str, Pick)], width: f32) -> f32 {
        let mut h = HEAD + WHY_H;
        if !buttons.is_empty() {
            h += flow_h(self.ui, buttons, width - 20.0) + 10.0;
        }
        h
    }

    /// The pieces of the places' column, `width` wide, and how tall they
    /// all are.
    fn items(&self, width: f32) -> (Vec<Item>, f32) {
        let cf = self.cf;
        let mut out = Vec::new();
        let mut y = PAD_TOP;
        let mut k = 0;
        for (i, f) in cf.files.iter().enumerate() {
            if i > 0 {
                y += GAP_FILE;
            }
            let mut push = |kind: Kind, h: f32, first: bool| {
                if !first {
                    y += GAP;
                }
                out.push(Item { kind, y, h });
                y += h;
            };
            push(Kind::File(i), FILE_H, true);
            match &f.body {
                Body::Text(blocks) => {
                    // Numbered as the mock does: as if each place still
                    // to solve held our side's lines.
                    let mut cur = 1;
                    for (j, b) in blocks.iter().enumerate() {
                        let n = cur + b.skipped + b.before.len();
                        if b.skipped > 0 {
                            push(Kind::Skip(b.skipped), SKIP_H, false);
                        }
                        if !b.before.is_empty() {
                            let from = n.saturating_sub(b.before.len()).max(1);
                            push(Kind::Lines { i, j, after: false, n: from }, b.before.len() as f32 * LINE, false);
                        }
                        let plan = self.plan((i, Some(j)), b);
                        push(Kind::Block { k, i, j, n }, self.block_h(&plan, width), false);
                        let len = b.result.as_ref().map_or(b.ours.len(), Vec::len);
                        if !b.after.is_empty() {
                            push(Kind::Lines { i, j, after: true, n: n + len }, b.after.len() as f32 * LINE, false);
                        }
                        if j + 1 == blocks.len() && b.skipped_after > 0 {
                            push(Kind::Skip(b.skipped_after), SKIP_H, false);
                        }
                        cur = n + len + b.after.len();
                        k += 1;
                    }
                }
                Body::Whole(w) => {
                    let waits = self.app.conflicts.waits((i, None));
                    let buttons = if f.unmerged && !waits { self.whole_buttons(w) } else { Vec::new() };
                    push(Kind::Whole { k, i }, self.whole_h(&buttons, width), false);
                    k += 1;
                }
            }
        }
        if cf.left() == 0 {
            y += GAP_FILE;
            let h = self.done_h();
            out.push(Item { kind: Kind::Done, y, h });
            y += h;
        }
        (out, y + PAD_BOTTOM)
    }

    /// The message's field, by its lines.
    fn field_h(&self) -> f32 {
        let m = &self.app.conflicts.message;
        let rows = m.lines().count() + usize::from(m.ends_with('\n'));
        rows.clamp(1, 6) as f32 * 16.0 + 12.0
    }

    fn done_h(&self) -> f32 {
        let field = if self.cf.doing == Doing::Rebase { 0.0 } else { self.field_h() };
        field.max(BTN_H) + 20.0
    }

    /// The places' column: what to click, and where the message's field
    /// is (with what of it is in sight).
    fn body(&self, br: Rect, pages: i32, follow: bool) -> (Option<Act>, Option<(Rect, Rect)>) {
        let ui = self.ui;
        let ctx = ui.ctx();
        let width = br.width() - 2.0 * PAD_X;
        let (items, total) = self.items(width);
        let h = br.height();
        let mut scroll = ctx.data(|d| d.get_temp::<f32>(scroll_id())).unwrap_or(0.0);
        let at = self.app.conflicts.at;
        let target = if self.app.conflicts.focus_message || (follow && self.cf.left() == 0) {
            items.iter().find(|it| matches!(it.kind, Kind::Done))
        } else if follow {
            items.iter().find(|it| matches!(it.kind, Kind::Block { k, .. } | Kind::Whole { k, .. } if k == at))
        } else {
            None
        };
        if let Some(it) = target {
            if it.y < scroll || it.y + it.h > scroll + h {
                scroll = (it.y - 70.0).max(0.0);
            }
        }
        scroll += pages as f32 * (h - 40.0).max(40.0);
        if self.live && ui.rect_contains_pointer(br) {
            scroll -= ui.input(|i| i.smooth_scroll_delta.y);
        }
        scroll = scroll.clamp(0.0, (total - h).max(0.0));
        ctx.data_mut(|d| d.insert_temp(scroll_id(), scroll));

        let p = ui.painter_at(br);
        let pointer = self.pointer.filter(|at| br.contains(*at));
        let mut out = None;
        let mut field = None;
        for it in &items {
            let r = Rect::from_min_size(pos2(br.left() + PAD_X, br.top() + it.y - scroll), vec2(width, it.h));
            if let Kind::Done = it.kind {
                // Placed out of sight too: see show().
                let (a, f) = self.done_bar(&p, r, pointer);
                field = f.map(|f| (f, br));
                if a.is_some() {
                    out = a;
                }
                continue;
            }
            if r.bottom() < br.top() || r.top() > br.bottom() {
                continue;
            }
            let a = match it.kind {
                Kind::File(i) => self.file_head(&p, i, r, pointer),
                Kind::Skip(n) => {
                    let text = self.tr("cf.same").replace("{}", &n.to_string());
                    let c = ito_theme::colors();
                    p.text(pos2(r.left() + 52.0, r.center().y), Align2::LEFT_CENTER, text, FontId::proportional(11.5), c.dim);
                    None
                }
                Kind::Lines { i, j, after, n } => {
                    self.context(&p, i, j, after, n, r);
                    None
                }
                Kind::Block { k, i, j, n } => self.block(&p, k, (i, j), n, r, pointer),
                Kind::Whole { k, i } => self.whole(&p, k, i, r, pointer),
                Kind::Done => None,
            };
            if a.is_some() {
                out = a;
            }
        }
        (out, field)
    }

    /// A file's head: its path, how many places, and the button that puts
    /// it back as git left it.
    fn file_head(&self, p: &Painter, i: usize, r: Rect, pointer: Option<Pos2>) -> Option<Act> {
        let ui = self.ui;
        let c = ito_theme::colors();
        let f = &self.cf.files[i];
        let mid = r.center().y;
        let label = self.tr("cf.restart");
        let w = button_w(ui, label, "", Look::Tool);
        let b = Rect::from_min_max(pos2(r.right() - w, mid - 12.0), pos2(r.right(), mid + 12.0));
        let mut out = None;
        if button(ui, p, b, label, "", Look::Tool, ("cf-restart", i), self.live, pointer) {
            out = Some(Act::Restart(i));
        }
        let n = galley(ui, self.tr("cf.places").replace("{}", &f.total().to_string()), FontId::proportional(12.0), c.dim);
        let room = b.left() - 10.0 - n.size().x - 10.0 - r.left();
        let path = left_cut(ui, &f.path, FontId::monospace(12.5), room);
        let pw = p.text(pos2(r.left(), mid), Align2::LEFT_CENTER, path, FontId::monospace(12.5), c.fg);
        p.galley(pos2(pw.right() + 10.0, mid - n.size().y / 2.0), n, c.dim);
        out
    }

    /// The lines before or after a place, dim.
    fn context(&self, p: &Painter, i: usize, j: usize, after: bool, n: usize, r: Rect) {
        let c = ito_theme::colors();
        let Body::Text(blocks) = &self.cf.files[i].body else { return };
        let b = &blocks[j];
        let lines = if after { &b.after } else { &b.before };
        for (m, line) in lines.iter().enumerate() {
            let mid = r.top() + m as f32 * LINE + LINE / 2.0;
            let no = (n + m).to_string();
            p.text(pos2(r.left() + CTX_NUM_W - 8.0, mid), Align2::RIGHT_CENTER, no, FontId::monospace(12.0), c.dim.gamma_multiply(0.6));
            let text = line.replace('\t', "    ");
            clipped(self.ui, p, pos2(r.left() + CTX_NUM_W, mid), &text, FontId::monospace(12.0), c.dim, r.width() - CTX_NUM_W);
        }
    }

    /// The head of the place `k`, its state and `tool` at the right; the
    /// tool's rect.
    fn place_head(&self, p: &Painter, k: usize, r: Rect, what: &str, state: (String, Color32), tool: Option<&str>) -> (Rect, Option<Rect>) {
        let ui = self.ui;
        let c = ito_theme::colors();
        let head = Rect::from_min_size(r.min, vec2(r.width(), HEAD));
        p.rect_filled(head, CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, c.panel);
        p.line_segment([head.left_bottom() - vec2(0.0, 0.5), head.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
        let mid = head.center().y;
        let mut right = head.right() - 6.0;
        let mut tool_r = None;
        if let Some(label) = tool {
            let w = button_w(ui, label, "", Look::Tool);
            let b = Rect::from_min_max(pos2(right - w, mid - 12.0), pos2(right, mid + 12.0));
            tool_r = Some(b);
            right = b.left() - 8.0;
        } else {
            right -= 4.0;
        }
        let (state, ink) = state;
        let s = p.text(pos2(right, mid), Align2::RIGHT_CENTER, state, FontId::proportional(11.5), ink);
        let x = head.left() + 10.0;
        let no = galley(ui, format!("{} / {}", k + 1, self.cf.total()), FontId::monospace(12.0), c.fg);
        let at = pos2(x, mid - no.size().y / 2.0);
        let nw = no.size().x;
        p.galley(at, no.clone(), c.fg);
        p.galley(at + vec2(0.45, 0.0), no, c.fg);
        clipped(ui, p, pos2(x + nw + 10.0, mid), what, FontId::proportional(12.0), c.fg, s.left() - 10.0 - x - nw - 10.0);
        (head, tool_r)
    }

    /// The state at the right of a place's head.
    fn state(&self, open: bool, waits: bool, tag: Option<Tag>) -> (String, Color32) {
        let c = ito_theme::colors();
        if waits {
            (self.tr("cf.writing").to_owned(), c.wait)
        } else if open {
            (self.tr("cf.open").to_owned(), c.err)
        } else {
            (format!("✓ {}", self.tr(tag_key(tag))), c.done)
        }
    }

    /// The place `k`: the block `j` of the file `i`, its first line
    /// numbered `n`, in `r`.
    fn block(&self, p: &Painter, k: usize, (i, j): (usize, usize), n: usize, r: Rect, pointer: Option<Pos2>) -> Option<Act> {
        let (ui, app) = (self.ui, self.app);
        let c = ito_theme::colors();
        let f = &self.cf.files[i];
        let Body::Text(blocks) = &f.body else { return None };
        let b = &blocks[j];
        let place = (i, Some(j));
        let plan = self.plan(place, b);
        let cur = app.conflicts.at == k;
        let mut out = None;
        // First, so the buttons over it take their clicks.
        if clicked(ui, p, r, ("cf-block", k), self.live, None) {
            out = Some(Act::Select(k));
        }
        if cur {
            p.rect_stroke(r.expand(1.0), CornerRadius::same(9), Stroke::new(1.0, c.err.gamma_multiply(0.3)), StrokeKind::Inside);
        }
        p.rect_filled(r, CornerRadius::same(8), c.bg);

        let what = self.tr("cf.at").replace("{file}", leaf(&f.path)).replace("{line}", &n.to_string());
        let tool = self.tr(if plan.base { "cf.base_hide" } else { "cf.base_show" });
        let (head, tool_r) = self.place_head(p, k, r, &what, self.state(b.is_open(), plan.waits, plan.tag), Some(tool));
        if let Some(t) = tool_r {
            if button(ui, p, t, tool, "", Look::Tool, ("cf-base", k), self.live, pointer) {
                out = Some(Act::Base(place));
            }
        }

        // Ours, the base, theirs.
        if let Some(i) = &plan.intent {
            let now = chrono::Local::now();
            for (q, (word, ink, ch)) in
                [(self.tr("cf.ours"), c.run, &i.ours), (self.tr("cf.theirs"), c.done, &i.theirs)].into_iter().enumerate()
            {
                let mid = head.bottom() + 3.0 + q as f32 * LINE + LINE / 2.0;
                let w = p.text(pos2(r.left() + 12.0, mid), Align2::LEFT_CENTER, word, FontId::proportional(11.0), ink);
                let text = match ch {
                    Some(ch) => {
                        let at = crate::graph::when(ch.time, &now, self.tr("time.yesterday"));
                        format!("{}  {} · {} · {at}", ch.id.short(), ch.subject, ch.author)
                    }
                    None => self.tr("cf.intent_none").to_owned(),
                };
                clipped(ui, p, pos2(w.right() + 10.0, mid), &text, FontId::proportional(12.0), c.dim, r.right() - w.right() - 24.0);
            }
        }
        let top = head.bottom() + plan.intent_h();
        let rows_h = COL_HEAD + 8.0 + plan.rows as f32 * LINE;
        let parts: &[f32] = if plan.base { &[1.0, 0.8, 1.0] } else { &[1.0, 1.0] };
        let unit = r.width() / parts.iter().sum::<f32>();
        let mut x = r.left();
        let short = |o: &Option<michi_core::oid::Oid>| o.as_ref().map(|o| o.short()).unwrap_or_default();
        // The words each side changed: against the base, else against the other side.
        let tabs = |l: &[String]| l.iter().map(|x| x.replace('\t', "    ")).collect::<Vec<_>>();
        let (ours, theirs) = (tabs(&b.ours), tabs(&b.theirs));
        let base = b.base.as_deref().map(tabs);
        let ours_marks = michi_core::words::marks(base.as_deref().unwrap_or(&theirs), &ours);
        let theirs_marks = michi_core::words::marks(base.as_deref().unwrap_or(&ours), &theirs);
        let mut cols = vec![Col {
            word: self.tr("cf.ours").to_owned(),
            ink: c.run,
            note: side_note(&self.cf.ours),
            lines: Some(&b.ours),
            marks: ours_marks,
            fill: c.run.gamma_multiply(0.07),
            empty: "cf.none_lines",
        }];
        if plan.base {
            let empty = if b.base.as_ref().is_some_and(Vec::is_empty) { "cf.base_empty" } else { "cf.base_unknown" };
            let (ink, note, lines) = (c.fg, short(&self.cf.base), b.base.as_deref());
            cols.push(Col { word: "base".to_owned(), ink, note, lines, marks: Vec::new(), fill: Color32::TRANSPARENT, empty });
        }
        cols.push(Col {
            word: self.tr("cf.theirs").to_owned(),
            ink: c.done,
            note: side_note(&self.cf.theirs),
            lines: Some(&b.theirs),
            marks: theirs_marks,
            fill: c.done.gamma_multiply(0.07),
            empty: "cf.none_lines",
        });
        for (m, Col { word, ink, note, lines, marks, fill, empty }) in cols.iter().enumerate() {
            let w = parts[m] * unit;
            let col = Rect::from_min_size(pos2(x, top), vec2(w, rows_h));
            x += w;
            if m + 1 < cols.len() {
                p.line_segment([pos2(col.right() - 0.5, col.top()), pos2(col.right() - 0.5, col.bottom())], Stroke::new(1.0, c.border));
            }
            let ch = Rect::from_min_size(col.min, vec2(w, COL_HEAD));
            p.line_segment([ch.left_bottom() - vec2(0.0, 0.5), ch.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
            let font = FontId::proportional(11.0);
            let mut job = LayoutJob::default();
            job.append(word, 0.0, TextFormat { font_id: font.clone(), color: *ink, ..Default::default() });
            if !note.is_empty() {
                job.append(&format!(" {note}"), 0.0, TextFormat { font_id: font, color: c.dim, ..Default::default() });
            }
            job.wrap = TextWrapping::truncate_at_width((w - 16.0).max(0.0));
            let g = ui.fonts_mut(|f| f.layout_job(job));
            p.galley(pos2(ch.left() + 8.0, ch.center().y - g.size().y / 2.0), g, c.dim);
            let y0 = ch.bottom() + 4.0;
            match lines.filter(|l| !l.is_empty()) {
                Some(lines) => {
                    for (q, line) in lines.iter().enumerate() {
                        let lr = Rect::from_min_size(pos2(col.left(), y0 + q as f32 * LINE), vec2(w - 1.0, LINE));
                        if *fill != Color32::TRANSPARENT {
                            p.rect_filled(lr, 0.0, *fill);
                        }
                        let mid = lr.center().y;
                        let no = (n + q).to_string();
                        p.text(
                            pos2(lr.left() + NUM_W - 6.0, mid),
                            Align2::RIGHT_CENTER,
                            no,
                            FontId::monospace(12.0),
                            c.dim.gamma_multiply(0.6),
                        );
                        let text = line.replace('\t', "    ");
                        let at = pos2(lr.left() + NUM_W, mid);
                        match marks.get(q).filter(|m| !m.is_empty()) {
                            Some(m) => marked(ui, p, at, &text, m, ink.gamma_multiply(0.35), w - NUM_W - 6.0),
                            None => clipped(ui, p, at, &text, FontId::monospace(12.0), c.fg, w - NUM_W - 6.0),
                        }
                    }
                }
                None => italic(ui, p, pos2(col.left() + 10.0, y0 + LINE / 2.0), self.tr(empty), c.dim, w - 20.0),
            }
        }
        let bottom = top + rows_h;
        p.line_segment([pos2(r.left(), bottom + 0.5), pos2(r.right(), bottom + 0.5)], Stroke::new(1.0, c.border));

        // The result, and the tag of what it was solved with.
        let rh = Rect::from_min_size(pos2(r.left(), bottom + 1.0), vec2(r.width(), RES_HEAD));
        let mid = rh.center().y;
        let wr = p.text(pos2(rh.left() + 10.0, mid), Align2::LEFT_CENTER, self.tr("cf.result"), FontId::proportional(11.0), c.dim);
        if !b.is_open() || plan.waits {
            let word = galley(ui, self.tr(tag_key(plan.tag)).to_owned(), FontId::proportional(11.0), c.fg);
            let chip = Rect::from_min_size(pos2(wr.right() + 8.0, mid - 8.0), vec2(word.size().x + 12.0, 16.0));
            p.rect_filled(chip, CornerRadius::same(3), c.panel);
            p.rect_stroke(chip, CornerRadius::same(3), Stroke::new(1.0, c.border), StrokeKind::Inside);
            p.galley(pos2(chip.left() + 6.0, mid - word.size().y / 2.0), word, c.fg);
        }
        let rb = Rect::from_min_size(pos2(r.left() + 10.0, rh.bottom()), vec2(r.width() - 20.0, plan.result_px()));
        p.rect_filled(rb, CornerRadius::same(5), c.panel);
        let corners = [rb.left_top(), rb.right_top(), rb.right_bottom(), rb.left_bottom(), rb.left_top()];
        p.extend(Shape::dashed_line(&corners, Stroke::new(1.0, c.border_strong()), 3.0, 2.0));
        if plan.writing.is_some() {
            self.field.set(Some((rb.shrink2(vec2(2.0, 4.0)), p.clip_rect())));
            clipped(
                ui,
                p,
                pos2(rh.right() - 14.0 - 300.0, rh.center().y),
                self.tr("cf.edit_hint"),
                FontId::proportional(11.0),
                c.wait,
                300.0,
            );
        }
        match &plan.result {
            _ if plan.writing.is_some() => {}
            Some(lines) if !lines.is_empty() => {
                for (q, line) in lines.iter().enumerate() {
                    let mid = rb.top() + 6.0 + q as f32 * LINE + LINE / 2.0;
                    let text = line.replace('\t', "    ");
                    clipped(ui, p, pos2(rb.left() + 8.0, mid), &text, FontId::monospace(12.0), c.fg, rb.width() - 16.0);
                }
            }
            Some(_) => {
                italic(ui, p, pos2(rb.left() + 8.0, rb.top() + 6.0 + LINE / 2.0), self.tr("cf.none_lines"), c.dim, rb.width() - 16.0)
            }
            None => italic(ui, p, pos2(rb.left() + 8.0, rb.top() + 6.0 + LINE / 2.0), self.tr("cf.result_open"), c.dim, rb.width() - 16.0),
        }

        // What it looks like, and the buttons.
        if plan.open {
            let mid = rb.bottom() + 8.0 + HINT_H / 2.0;
            let hint = match b.hint() {
                Hint::BothAdded => self.tr("cf.hint_added").to_owned(),
                Hint::OursDeleted => self.tr("cf.hint_ours_gone").to_owned(),
                Hint::TheirsDeleted => self.tr("cf.hint_theirs_gone").to_owned(),
                Hint::BothChanged(_) if b.base.is_none() => self.tr("cf.hint_unknown").to_owned(),
                Hint::BothChanged(n) => self.tr("cf.hint_both").replace("{}", &n.to_string()),
            };
            let word = galley(ui, self.tr("cf.hint").to_owned(), FontId::proportional(12.0), c.wait);
            let ww = word.size().x;
            let at = pos2(r.left() + 10.0, mid - word.size().y / 2.0);
            p.galley(at, word.clone(), c.wait);
            p.galley(at + vec2(0.45, 0.0), word, c.wait);
            clipped(ui, p, pos2(r.left() + 10.0 + ww + 8.0, mid), &hint, FontId::proportional(12.0), c.wait, r.width() - 28.0 - ww);
            let top = rb.bottom() + 8.0 + HINT_H + 8.0;
            let rects = flow(ui, &plan.buttons, r.left() + 10.0, top, r.right() - 10.0);
            for (q, ((label, key, pick), br)) in plan.buttons.iter().zip(rects).enumerate() {
                if button(ui, p, br, label, key, Look::Plain, ("cf-pick", k, q), self.live, pointer) {
                    out = Some(Act::Pick(k, *pick));
                }
            }
        }

        // As the mock's CSS: a solved place keeps its green edge when it is the current one.
        let edge = if !b.is_open() {
            mix(c.border, c.done, 0.5)
        } else if cur {
            mix(c.border, c.err, 0.6)
        } else {
            c.border
        };
        p.rect_stroke(r, CornerRadius::same(8), Stroke::new(1.0, edge), StrokeKind::Inside);
        out
    }

    /// The place `k`: the file `i`, which clashed whole, in `r`.
    fn whole(&self, p: &Painter, k: usize, i: usize, r: Rect, pointer: Option<Pos2>) -> Option<Act> {
        let (ui, app) = (self.ui, self.app);
        let c = ito_theme::colors();
        let f = &self.cf.files[i];
        let Body::Whole(w) = &f.body else { return None };
        let place = (i, None);
        let waits = app.conflicts.waits(place);
        let tag = app.conflicts.tag(place);
        let cur = app.conflicts.at == k;
        let mut out = None;
        if clicked(ui, p, r, ("cf-block", k), self.live, None) {
            out = Some(Act::Select(k));
        }
        if cur {
            p.rect_stroke(r.expand(1.0), CornerRadius::same(9), Stroke::new(1.0, c.err.gamma_multiply(0.3)), StrokeKind::Inside);
        }
        p.rect_filled(r, CornerRadius::same(8), c.bg);
        let (head, _) = self.place_head(p, k, r, leaf(&f.path), self.state(f.unmerged, waits, tag), None);
        let why = match (w.why, w.ours, w.theirs) {
            (Why::Deleted, false, _) => "cf.why_ours_gone",
            (Why::Deleted, _, false) => "cf.why_theirs_gone",
            (Why::Binary, ..) => "cf.why_binary",
            _ => "cf.why_other",
        };
        let mid = head.bottom() + WHY_H / 2.0;
        clipped(ui, p, pos2(r.left() + 10.0, mid), self.tr(why), FontId::proportional(12.0), c.fg, r.width() - 20.0);
        if f.unmerged && !waits {
            let buttons = self.whole_buttons(w);
            let rects = flow(ui, &buttons, r.left() + 10.0, head.bottom() + WHY_H, r.right() - 10.0);
            for (q, ((label, key, pick), br)) in buttons.iter().zip(rects).enumerate() {
                if button(ui, p, br, label, key, Look::Plain, ("cf-take", k, q), self.live, pointer) {
                    out = Some(Act::Pick(k, *pick));
                }
            }
        }
        let edge = if !f.unmerged {
            mix(c.border, c.done, 0.5)
        } else if cur {
            mix(c.border, c.err, 0.6)
        } else {
            c.border
        };
        p.rect_stroke(r, CornerRadius::same(8), Stroke::new(1.0, edge), StrokeKind::Inside);
        out
    }

    /// The bar at the end once all is solved: the message and the commit
    /// (a rebase goes on instead). The message's field, for show().
    fn done_bar(&self, p: &Painter, r: Rect, pointer: Option<Pos2>) -> (Option<Act>, Option<Rect>) {
        let ui = self.ui;
        let c = ito_theme::colors();
        let rebase = self.cf.doing == Doing::Rebase;
        p.rect_filled(r, CornerRadius::same(8), c.done.gamma_multiply(0.09));
        p.rect_stroke(r, CornerRadius::same(8), Stroke::new(1.0, c.done), StrokeKind::Inside);
        let inner = r.shrink2(vec2(12.0, 10.0));
        let first = inner.top() + BTN_H / 2.0;
        let label = self.tr(if rebase { "cf.continue_button" } else { "cf.commit_button" });
        let key = if rebase { "" } else { "Ctrl+Enter" };
        let w = button_w(ui, label, key, Look::Prime);
        let b = Rect::from_min_size(pos2(inner.right() - w, inner.top()), vec2(w, BTN_H));
        let mut out = None;
        if button(ui, p, b, label, key, Look::Prime, "cf-commit", self.live, pointer) {
            out = Some(Act::Commit);
        }
        let reused = self.app.conflicts.reused_left();
        let mut said = self.tr("cf.done").to_owned();
        if reused > 0 {
            said = format!("{said} {}", self.tr("cf.reused_left").replace("{}", &reused.to_string()));
        }
        let done = p.text(pos2(inner.left(), first), Align2::LEFT_CENTER, said, FontId::proportional(12.5), c.fg);
        if rebase {
            return (out, None);
        }
        let field = Rect::from_min_max(pos2(done.right() + 12.0, inner.top()), pos2(b.left() - 10.0, inner.top() + self.field_h()));
        let focused = ui.memory(|m| m.has_focus(message_id()));
        p.rect_filled(field, CornerRadius::same(4), c.bg);
        let border = if focused { c.border_strong() } else { c.border };
        p.rect_stroke(field, CornerRadius::same(4), Stroke::new(1.0, border), StrokeKind::Inside);
        (out, Some(field))
    }
}

/// A side's name, else its short id.
fn label(s: &Side) -> String {
    if !s.name.is_empty() {
        return s.name.clone();
    }
    s.id.as_ref().map_or_else(|| "?".to_owned(), |o| o.short())
}

/// What a column's head says after "ours" or "theirs": the side's name and
/// short id, each once.
fn side_note(s: &Side) -> String {
    let id = s.id.as_ref().map(|o| o.short()).unwrap_or_default();
    if s.name.is_empty() || s.name == id {
        id
    } else if id.is_empty() {
        s.name.clone()
    } else {
        format!("{} {id}", s.name)
    }
}

/// The last part of a path.
fn leaf(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn head_key(d: Doing) -> &'static str {
    match d {
        Doing::Merge | Doing::Bisect => "cf.head_merge",
        Doing::CherryPick => "cf.head_cherry",
        Doing::Revert => "cf.head_revert",
        Doing::Rebase => "cf.head_rebase",
    }
}

fn tag_key(tag: Option<Tag>) -> &'static str {
    match tag {
        Some(Tag::Picked(Pick::Ours)) => "cf.tag_ours",
        Some(Tag::Picked(Pick::Theirs)) => "cf.tag_theirs",
        Some(Tag::Picked(Pick::Both)) => "cf.tag_both",
        Some(Tag::Picked(Pick::BothTheirsFirst)) => "cf.tag_both2",
        Some(Tag::Picked(Pick::Base)) => "cf.tag_base",
        Some(Tag::Took(true)) => "cf.tag_file_ours",
        Some(Tag::Took(false)) => "cf.tag_file_theirs",
        Some(Tag::Reused) => "cf.tag_reused",
        Some(Tag::Edited) => "cf.tag_edited",
        None => "cf.hand",
    }
}

/// `template` with each of `parts`' keys put in its own colour.
fn rich(template: &str, parts: &[(&str, &str, Color32)], font: &FontId, ink: Color32) -> LayoutJob {
    let format = |color| TextFormat { font_id: font.clone(), color, ..Default::default() };
    let mut job = LayoutJob::default();
    let mut rest = template;
    while let Some((at, key, text, color)) =
        parts.iter().filter_map(|&(key, text, color)| rest.find(key).map(|at| (at, key, text, color))).min_by_key(|x| x.0)
    {
        job.append(&rest[..at], 0.0, format(ink));
        job.append(text, 0.0, format(color));
        rest = &rest[at + key.len()..];
    }
    job.append(rest, 0.0, format(ink));
    job
}

fn galley(ui: &Ui, text: String, font: FontId, color: Color32) -> Arc<Galley> {
    ui.fonts_mut(|f| f.layout_no_wrap(text, font, color))
}

fn wrapped(ui: &Ui, text: String, font: FontId, color: Color32, width: f32) -> Arc<Galley> {
    ui.fonts_mut(|f| f.layout(text, font, color, width.max(40.0)))
}

/// `text` from `left_center` with the byte `ranges` on a `back` ground, cut with … to `width`.
fn marked(ui: &Ui, p: &Painter, left_center: Pos2, text: &str, ranges: &[Range<usize>], back: Color32, width: f32) {
    if width < 8.0 {
        return;
    }
    let font = FontId::monospace(12.0);
    let c = ito_theme::colors();
    let mut job = LayoutJob::default();
    let mut at = 0;
    let put = |job: &mut LayoutJob, part: &str, background| {
        if !part.is_empty() {
            job.append(part, 0.0, TextFormat { font_id: font.clone(), color: c.fg, background, ..Default::default() });
        }
    };
    for r in ranges.iter().filter(|r| r.end <= text.len() && text.is_char_boundary(r.start) && text.is_char_boundary(r.end)) {
        if r.start < at {
            continue;
        }
        put(&mut job, &text[at..r.start], Color32::TRANSPARENT);
        put(&mut job, &text[r.clone()], back);
        at = r.end;
    }
    put(&mut job, &text[at..], Color32::TRANSPARENT);
    job.wrap = TextWrapping::truncate_at_width(width);
    let g = ui.fonts_mut(|f| f.layout_job(job));
    p.galley(pos2(left_center.x, left_center.y - g.size().y / 2.0), g, c.fg);
}

/// A note in italics from `left_center`, cut to `width`.
fn italic(ui: &Ui, p: &Painter, left_center: Pos2, text: &str, color: Color32, width: f32) {
    let mut job = LayoutJob::single_section(
        text.to_owned(),
        TextFormat { font_id: FontId::proportional(11.5), color, italics: true, ..Default::default() },
    );
    job.wrap = TextWrapping::truncate_at_width(width.max(0.0));
    let g = ui.fonts_mut(|f| f.layout_job(job));
    p.galley(pos2(left_center.x, left_center.y - g.size().y / 2.0), g, color);
}

/// Whether `hit` was clicked; a hand over it.
fn clicked(ui: &Ui, p: &Painter, hit: Rect, id: impl std::hash::Hash + std::fmt::Debug, live: bool, pointer: Option<Pos2>) -> bool {
    let resp = ui.interact(hit.intersect(p.clip_rect()), Id::new(id), Sense::CLICK);
    if live && pointer.is_some_and(|at| hit.contains(at)) {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    live && resp.clicked()
}

fn pad(look: Look) -> f32 {
    if look == Look::Tool {
        8.0
    } else {
        9.0
    }
}

fn button_w(ui: &Ui, label: &str, key: &str, look: Look) -> f32 {
    let g = galley(ui, label.to_owned(), FontId::proportional(12.0), Color32::WHITE).size().x;
    let k = if key.is_empty() { 0.0 } else { 4.0 + galley(ui, key.to_owned(), FontId::monospace(10.5), Color32::WHITE).size().x };
    pad(look) + g + k + pad(look)
}

/// A button in `b`: its label, its key after it; whether it was clicked.
#[expect(clippy::too_many_arguments, reason = "the painter, the place and the look are the frame's")]
fn button(
    ui: &Ui,
    p: &Painter,
    b: Rect,
    label: &str,
    key: &str,
    look: Look,
    id: impl std::hash::Hash + std::fmt::Debug,
    live: bool,
    pointer: Option<Pos2>,
) -> bool {
    let c = ito_theme::colors();
    let hot = live && pointer.is_some_and(|at| b.contains(at));
    let none = Color32::TRANSPARENT;
    let (fill, edge, ink) = match look {
        Look::Plain => (c.panel, if hot { c.run } else { c.border }, c.fg),
        Look::Danger => (c.panel, if hot { c.err } else { c.border }, c.err),
        Look::Prime => (c.run, c.run, c.on_accent()),
        Look::Tool => (if hot { c.panel } else { none }, if hot { c.border } else { none }, c.fg),
    };
    let radius = CornerRadius::same(if look == Look::Tool { 5 } else { 4 });
    if fill != none {
        p.rect_filled(b, radius, fill);
    }
    if edge != none {
        p.rect_stroke(b, radius, Stroke::new(1.0, edge), StrokeKind::Inside);
    }
    let mid = b.center().y;
    let g = galley(ui, label.to_owned(), FontId::proportional(12.0), ink);
    let at = pos2(b.left() + pad(look), mid - g.size().y / 2.0);
    let w = g.size().x;
    p.galley(at, g.clone(), ink);
    if look == Look::Prime {
        p.galley(at + vec2(0.45, 0.0), g, ink);
    }
    if !key.is_empty() {
        let ink = if look == Look::Prime { ink.gamma_multiply(0.7) } else { c.dim };
        p.text(pos2(at.x + w + 4.0, mid), Align2::LEFT_CENTER, key, FontId::monospace(10.5), ink);
    }
    clicked(ui, p, b, id, live, pointer)
}

/// The buttons laid from `left` at `top`, a row after another when one is
/// full at `right`.
fn flow(ui: &Ui, buttons: &[(&str, &str, Pick)], left: f32, top: f32, right: f32) -> Vec<Rect> {
    let mut out = Vec::new();
    let (mut x, mut y) = (left, top);
    for (label, key, _) in buttons {
        let w = button_w(ui, label, key, Look::Plain);
        if x > left && x + w > right {
            x = left;
            y += BTN_H + 5.0;
        }
        out.push(Rect::from_min_size(pos2(x, y), vec2(w, BTN_H)));
        x += w + 5.0;
    }
    out
}

/// How tall the buttons are, laid in `width`.
fn flow_h(ui: &Ui, buttons: &[(&str, &str, Pick)], width: f32) -> f32 {
    flow(ui, buttons, 0.0, 0.0, width).last().map_or(0.0, |r| r.bottom())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rich_text_colours_each_part() {
        let font = FontId::proportional(12.0);
        let job = rich(
            "{from} を {into} にマージ中",
            &[("{from}", "feat", Color32::RED), ("{into}", "main", Color32::BLUE)],
            &font,
            Color32::WHITE,
        );
        assert_eq!(job.text, "feat を main にマージ中");
        let colours: Vec<Color32> = job.sections.iter().filter(|s| !s.byte_range.is_empty()).map(|s| s.format.color).collect();
        assert_eq!(colours, [Color32::RED, Color32::WHITE, Color32::BLUE, Color32::WHITE]);
    }

    #[test]
    fn a_side_is_named_once() {
        let id = michi_core::oid::Oid::parse(&"a".repeat(40)).unwrap();
        let named = Side { name: "feat".into(), id: Some(id) };
        assert_eq!(label(&named), "feat");
        assert_eq!(side_note(&named), "feat aaaaaaa");
        let bare = Side { name: String::new(), id: Some(id) };
        assert_eq!(label(&bare), "aaaaaaa");
        assert_eq!(side_note(&bare), "aaaaaaa");
        let same = Side { name: "aaaaaaa".into(), id: Some(id) };
        assert_eq!(side_note(&same), "aaaaaaa");
        assert_eq!(leaf("src/a/b.rs"), "b.rs");
    }
}
