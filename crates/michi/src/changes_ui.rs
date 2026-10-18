//! The `2` view (docs/design.md: 変更): the worktrees to switch between on
//! top, with ● where a Claude is writing; the staged and the unstaged files
//! on the left over the commit message; the selected file's diff on the
//! right, in one column or side by side, with the lines picked and the
//! cursor over it while the keys are on its lines. A write that waits for a
//! yes shows what it would do under the top bar. Both sides scroll by the
//! wheel, with no scrollbar; only the rows in sight are drawn.

use egui::text::{LayoutJob, TextFormat, TextWrapping};
use egui::{pos2, vec2, Align2, CornerRadius, CursorIcon, FontId, Id, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, Ui};
use michi_core::msghelp::Guide;
use michi_core::paths;
use michi_core::show::File;
use michi_core::stage::{Lists, Side};
use michi_core::tree::Row;

use crate::app::App;
use crate::changes::{Changes, Focus, Op, Shown, Told};
use crate::detail::{Paired, Unified};
use crate::detail_ui::{diff, left_cut, status_ink, Wrap, DIFF_H, FILE_H, NOTE_H, TITLE_H, X};
use crate::intro_ui;
use crate::ui::{accent, kbd};
use michi_core::guide::Topic;

const PAD_X: f32 = 18.0;
const PAD_TOP: f32 = 14.0;
const TOP_H: f32 = 30.0;
/// The line under the top bar that asks for a yes.
const CONFIRM_H: f32 = 34.0;
/// The left column's width, at least and at most.
const LIST_MIN: f32 = 240.0;
const LIST_MAX: f32 = 380.0;
/// A list's heading.
const HEAD_H: f32 = 26.0;
/// How far each folder level of the tree is indented.
const INDENT: f32 = 14.0;
/// The commit message's box under the files.
const BOX_H: f32 = 168.0;
/// The room the path keeps in the title over the diff before the hint.
const PATH_MIN: f32 = 80.0;

fn list_scroll_id() -> Id {
    Id::new("changes-list-scroll")
}

/// The selection the list last brought into sight.
fn list_seen_id() -> Id {
    Id::new("changes-list-seen")
}

/// The width of the minimap beside the diff.
const MINIMAP_W: f32 = 12.0;
/// How far above a change `n` / `N` puts it.
const JUMP_MARGIN: f32 = 2.0 * DIFF_H;

/// The change the last `n` / `N` went to.
fn run_id() -> Id {
    Id::new("changes-run")
}

fn scroll_id() -> Id {
    Id::new("changes-scroll")
}

/// The cursor the diff last brought into sight.
/// How far what git said is scrolled.
fn told_id() -> Id {
    Id::new("changes-told-scroll")
}

fn cursor_seen_id() -> Id {
    Id::new("changes-cursor-seen")
}

/// The commit message's box: while it has the keys, they are its.
pub fn message_id() -> Id {
    Id::new("changes-message")
}

/// What a click does, once everything is drawn.
enum Act {
    Key(char),
    Open(std::path::PathBuf),
    Select(usize),
    /// Shift+click: the range up to the file.
    Range(usize),
    /// Ctrl+click: the file joins the marked ones, or leaves them.
    Pick(usize),
    /// A file double-clicked: staged or unstaged.
    Toggle(usize),
    /// A pick in a file's right-click menu: the key, typed on that file.
    Menu(usize, char),
    Wide(bool),
    /// A folder of the tree folded, or opened.
    Fold(Side, String),
    /// A folder of the tree staged whole, or taken out whole.
    Dir(Side, String),
    Commit,
    No,
    /// What git said is put away.
    Untell,
    /// The refused commit is run again.
    Again,
    /// The stage diagram was dismissed.
    Guide,
    /// The link that brings the stage diagram back.
    GuideShow,
}

/// The `2` view in `r`.
pub fn show(ui: &mut Ui, app: &mut App, r: Rect, live: bool) {
    let c = ito_theme::colors();
    let ctx = ui.ctx().clone();
    let p = ui.painter_at(r);
    p.rect_filled(r, 0.0, c.bg);
    let ch = &mut app.changes;
    let (to_top, pages, jump) = (std::mem::take(&mut ch.to_top), std::mem::take(&mut ch.pages), std::mem::take(&mut ch.jump));
    if to_top {
        // Also while the new file's diff is read.
        ctx.data_mut(|d| {
            d.insert_temp(scroll_id(), 0.0_f32);
            d.insert_temp(told_id(), 0.0_f32);
            d.insert_temp(run_id(), None::<usize>);
        });
    }
    let pointer = if live { ui.input(|i| i.pointer.hover_pos()).filter(|at| r.contains(*at)) } else { None };
    let mut act = None;

    // The top bar: the worktrees, the amend switch.
    let mid = r.top() + PAD_TOP + TOP_H / 2.0;
    let (l, hit) = switch(ui, &p, r.right() - PAD_X, mid, app.tr("changes.amend"), 'A', app.changes.amend, live);
    if hit {
        act = Some(Act::Key('A'));
    }
    let (l, hit) = switch(ui, &p, l - 8.0, mid, app.tr("changes.tree"), 't', app.changes.tree, live);
    if hit {
        act = Some(Act::Key('t'));
    }
    // What the stage is (#120): a diagram under the bar until it is dismissed,
    // then a link in the bar that brings it back.
    let guide = app.prefs.guides.shown(Topic::Stage);
    let mut l = l;
    if !guide {
        let (left, hit) = intro_ui::link(ui, &p, l - 14.0, mid, app.tr("guide.link.stage"), "stage", live, pointer);
        if hit {
            act = Some(Act::GuideShow);
        }
        l = left - 2.0;
    }
    if let Some(a) = worktrees(ui, app, &p, r.left() + PAD_X, l - 16.0, mid, live, pointer) {
        act = Some(a);
    }
    let mut top = r.top() + PAD_TOP + TOP_H + 8.0;
    if app.changes.confirm.is_some() {
        let band = Rect::from_min_max(pos2(r.left(), top), pos2(r.right(), top + CONFIRM_H));
        if let Some(a) = confirm(ui, app, &p, band, live, pointer) {
            act = Some(a);
        }
        top = band.bottom();
    }
    if let Some(lists) = app.changes.lists().filter(|_| guide) {
        let now = michi_core::guide::stage(lists.staged.len(), app.changes.message.trim().is_empty());
        let (a, h) = intro_ui::stage_band(ui, app, &p, pos2(r.left(), top), r.width(), now, live, pointer);
        if a.is_some() {
            act = Some(Act::Guide);
        }
        top += h;
    }

    // The files on the left, the selected one's diff on the right with the
    // message under it.
    p.line_segment([pos2(r.left(), top), pos2(r.right(), top)], Stroke::new(1.0, c.border));
    let lw = (r.width() * 0.3).clamp(LIST_MIN, LIST_MAX).min(r.width() / 2.0);
    let left = Rect::from_min_max(pos2(r.left(), top + 1.0), pos2(r.left() + lw, r.bottom()));
    let right = Rect::from_min_max(pos2(left.right() + 1.0, top + 1.0), r.max);
    p.line_segment([pos2(left.right() + 0.5, top), pos2(left.right() + 0.5, r.bottom())], Stroke::new(1.0, c.border));
    let box_h = BOX_H.min(right.height() * 0.5);
    let dr = Rect::from_min_max(right.min, pos2(right.right(), right.bottom() - box_h));
    let br = Rect::from_min_max(pos2(right.left(), dr.bottom()), right.max);
    list(ui, app, left, live, pointer, &mut act);
    if let Some(a) = message(ui, app, br, live, pointer) {
        act = Some(a);
    }
    let ch = &app.changes;
    if let Some(told) = &ch.told {
        if let Some(a) = told_of(ui, app, told, dr, live, pointer, pages) {
            act = Some(a);
        }
    } else if let Some((side, f)) = ch.selected() {
        if let Some(a) = title_row(ui, app, side, f, Rect::from_min_size(dr.min, vec2(dr.width(), TITLE_H)), live, pointer) {
            act = Some(a);
        }
        let body = Rect::from_min_max(pos2(dr.left(), dr.top() + TITLE_H), dr.max);
        body_of(ui, app, body, live, pages, jump);
    } else if ch.lists().is_some() {
        note(ui, &ui.painter_at(dr), dr, app.tr("changes.pick"));
    }

    let Some(a) = act else { return };
    match a {
        Act::Key(k) => {
            app.feed(k);
        }
        Act::Open(wt) => app.changes_open(wt),
        Act::Select(k) => {
            if app.changes.focus == Focus::Lines {
                app.changes.leave_lines();
            }
            app.changes_select(k);
        }
        Act::Range(k) | Act::Pick(k) => {
            if app.changes.focus == Focus::Lines {
                app.changes.leave_lines();
            }
            app.changes_click_multi(k, matches!(a, Act::Range(_)));
        }
        Act::Toggle(k) => {
            app.changes_select(k);
            app.feed(' ');
        }
        Act::Menu(k, key) => {
            if app.changes.focus == Focus::Lines {
                app.changes.leave_lines();
            }
            app.changes_select(k);
            // The ones that lose work sit behind g.
            if matches!(key, 'd' | 'i' | 'a') {
                app.feed('g');
            }
            app.feed(key);
        }
        Act::Wide(w) => app.changes.wide = w,
        Act::Fold(side, dir) => app.changes.fold(side, &dir),
        Act::Dir(side, dir) => app.changes_dir(side, &dir),
        Act::Commit => app.changes_commit(),
        Act::No => {
            app.escape();
        }
        Act::Untell => app.changes.told = None,
        Act::Again => app.changes_run_again(),
        Act::Guide => app.prefs.dismiss_guide(Topic::Stage),
        Act::GuideShow => app.prefs.restore_guides(Some(Topic::Stage)),
    }
    ctx.request_repaint();
}

/// All git said when a write failed, in place of the diff: what failed
/// over the words as git wrote them, scrolled as the diff is. Whether
/// "close" or "run again" was clicked.
fn told_of(ui: &Ui, app: &App, told: &Told, dr: Rect, live: bool, pointer: Option<Pos2>, pages: i32) -> Option<Act> {
    let c = ito_theme::colors();
    let ctx = ui.ctx();
    let rr = Rect::from_min_size(dr.min, vec2(dr.width(), TITLE_H));
    let p = ui.painter_at(rr);
    p.rect_filled(rr, 0.0, c.side);
    p.line_segment([rr.left_bottom() - vec2(0.0, 0.5), rr.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
    let mid = rr.center().y;
    let cap = kbd(ui, &p, pos2(rr.right() - X, mid), "Esc");
    let w = p.text(pos2(cap.left() - 6.0, mid), Align2::RIGHT_CENTER, app.tr("changes.told_close"), FontId::proportional(12.0), c.dim);
    let closed = clicked(ui, &p, w.union(cap).expand(3.0), "changes-untell", live, pointer);
    // A refused commit can be run again (the hooks run again too).
    let mut again = false;
    let mut left = w.left().min(cap.left());
    if told.commit {
        let cap2 = kbd(ui, &p, pos2(left - 14.0 - 6.0, mid), "r");
        let w2 =
            p.text(pos2(cap2.left() - 6.0, mid), Align2::RIGHT_CENTER, app.tr("changes.told_again"), FontId::proportional(12.0), c.dim);
        again = clicked(ui, &p, w2.union(cap2).expand(3.0), "changes-again", live, pointer);
        left = w2.left();
    }
    let title = app.tr(if told.commit { "changes.told_commit" } else { "changes.told" });
    let g = ui.fonts_mut(|f| {
        let mut job = LayoutJob::simple_singleline(title.to_owned(), FontId::proportional(12.5), c.err);
        job.wrap = TextWrapping::truncate_at_width((left - 12.0 - rr.left() - X).max(0.0));
        f.layout_job(job)
    });
    p.galley(pos2(rr.left() + X, mid - g.size().y / 2.0), g, c.err);

    let body = Rect::from_min_max(pos2(dr.left(), rr.bottom()), dr.max);
    let p = ui.painter_at(body);
    let g = ui.fonts_mut(|f| f.layout(told.all.clone(), FontId::monospace(12.0), c.fg, (body.width() - 2.0 * X).max(40.0)));
    let total = g.size().y + 24.0;
    let mut scroll = ctx.data(|d| d.get_temp::<f32>(told_id())).unwrap_or(0.0);
    scroll += pages as f32 * (body.height() - 40.0).max(40.0);
    if live && ui.rect_contains_pointer(body) {
        scroll -= ui.input(|i| i.smooth_scroll_delta.y);
    }
    scroll = scroll.clamp(0.0, (total - body.height()).max(0.0));
    ctx.data_mut(|d| d.insert_temp(told_id(), scroll));
    p.galley(pos2(body.left() + X, body.top() + 10.0 - scroll), g, c.fg);
    if again {
        Some(Act::Again)
    } else if closed {
        Some(Act::Untell)
    } else {
        None
    }
}

/// Whether `hit` was clicked; a hand over it.
fn clicked(ui: &Ui, p: &Painter, hit: Rect, id: impl std::hash::Hash + std::fmt::Debug, live: bool, pointer: Option<Pos2>) -> bool {
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
    let ink = if on { c.wait } else { c.dim };
    let g = ui.fonts_mut(|f| f.layout_no_wrap(label.to_owned(), FontId::proportional(12.5), ink));
    let k = ui.fonts_mut(|f| f.layout_no_wrap(key.to_string(), FontId::monospace(10.5), c.dim));
    let w = 9.0 + g.size().x + 6.0 + k.size().x + 9.0;
    let b = Rect::from_min_max(pos2(right - w, mid - 12.0), pos2(right, mid + 12.0));
    let resp = ui.interact(b.intersect(p.clip_rect()), Id::new(("changes-switch", key)), Sense::CLICK);
    if on {
        p.rect_filled(b, CornerRadius::same(5), c.wait.gamma_multiply(0.12));
    } else if live && resp.hovered() {
        p.rect_filled(b, CornerRadius::same(5), c.panel);
    }
    if live && resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let edge = if on { c.wait.gamma_multiply(0.45) } else { c.border };
    p.rect_stroke(b, CornerRadius::same(5), Stroke::new(1.0, edge), StrokeKind::Inside);
    p.galley(pos2(b.left() + 9.0, mid - g.size().y / 2.0), g.clone(), ink);
    p.galley(pos2(b.right() - 9.0 - k.size().x, mid - k.size().y / 2.0), k, c.dim);
    (b.left(), live && resp.clicked())
}

/// The worktrees as chips from `left`, the shown one lit, ● where a Claude
/// is writing; `[ ]` after them while there is room before `right`.
#[expect(clippy::too_many_arguments, reason = "the painter, the place and the look are the frame's")]
fn worktrees(ui: &Ui, app: &App, p: &Painter, left: f32, right: f32, mid: f32, live: bool, pointer: Option<Pos2>) -> Option<Act> {
    let c = ito_theme::colors();
    let shown = app.changes.wt.as_deref().map(paths::key);
    let mut x = left;
    let mut out = None;
    let trees: Vec<_> = app.change_trees().collect();
    for (k, t) in trees.iter().enumerate() {
        let on = shown.as_ref() == Some(&paths::key(&t.wt.path));
        let running = app.claude_in(&t.wt.path).iter().any(|s| s.is_agent() && s.state == michi_core::claude::State::Running);
        let g = ui.fonts_mut(|f| f.layout_no_wrap(crate::wt_ui::name_of(t), FontId::proportional(12.5), c.fg));
        let dot = if running { 12.0 } else { 0.0 };
        let w = 10.0 + dot + g.size().x + 10.0;
        if x + w > right {
            // The rest by `[ ]`.
            p.text(pos2(x, mid), Align2::LEFT_CENTER, "…", FontId::proportional(13.0), c.dim);
            break;
        }
        let b = Rect::from_min_max(pos2(x, mid - 12.0), pos2(x + w, mid + 12.0));
        let hot = live && pointer.is_some_and(|at| b.contains(at));
        if on {
            p.rect_filled(b, CornerRadius::same(5), c.selection());
            p.rect_stroke(b, CornerRadius::same(5), Stroke::new(1.0, accent().gamma_multiply(0.6)), StrokeKind::Inside);
        } else {
            if hot {
                p.rect_filled(b, CornerRadius::same(5), c.panel);
            }
            p.rect_stroke(b, CornerRadius::same(5), Stroke::new(1.0, c.border), StrokeKind::Inside);
        }
        if running {
            p.circle_filled(pos2(b.left() + 10.0 + 3.5, mid), 3.5, c.run);
        }
        p.galley(pos2(b.left() + 10.0 + dot, mid - g.size().y / 2.0), g, if on { c.fg } else { c.dim });
        if clicked(ui, p, b, ("changes-wt", k), live, pointer) && !on {
            out = Some(Act::Open(t.wt.path.clone()));
        }
        x = b.right() + 6.0;
    }
    if trees.len() > 1 && x + 30.0 < right {
        kbd(ui, p, pos2(x + 30.0, mid), "[ ]");
    }
    out
}

/// The band asking for a yes: what the write would do, `y` and `Esc`.
fn confirm(ui: &Ui, app: &App, p: &Painter, band: Rect, live: bool, pointer: Option<Pos2>) -> Option<Act> {
    let c = ito_theme::colors();
    let Some(cf) = &app.changes.confirm else { return None };
    p.rect_filled(band, 0.0, c.wait.gamma_multiply(0.14));
    let mid = band.center().y;
    let mut right = band.right() - PAD_X;
    let mut out = None;
    for (key, word, a) in [("Esc", "changes.no", Act::No), ("y", "changes.yes", Act::Key('y'))] {
        let cap = kbd(ui, p, pos2(right, mid), key);
        let w = p.text(pos2(cap.left() - 6.0, mid), Align2::RIGHT_CENTER, app.tr(word), FontId::proportional(12.5), c.fg);
        let hit = w.union(cap).expand(3.0);
        if clicked(ui, p, hit, ("changes-confirm", key), live, pointer) {
            out = Some(a);
        }
        right = hit.left() - 14.0;
    }
    let mut text = Vec::new();
    if let Op::Discard { path, untracked, lines, .. } = &cf.op {
        let key = match (untracked, lines) {
            (true, _) => "changes.discard_untracked",
            (false, 0) => "changes.discard_file",
            _ => "changes.discard_lines",
        };
        text.push(app.tr(key).replace("{file}", path).replace("{n}", &lines.to_string()));
    }
    if let Some(b) = &cf.branch {
        text.push(app.tr("changes.on_protected").replace("{branch}", b));
    }
    if !cf.notes.is_empty() {
        text.push(notes_text(app, cf));
    }
    if cf.claude {
        text.push(app.tr("changes.claude_writing").to_owned());
    }
    if cf.pushed {
        text.push(app.tr("changes.rewrites").to_owned());
    }
    let left = band.left() + PAD_X;
    p.text(pos2(left, mid), Align2::LEFT_CENTER, "●", FontId::proportional(12.0), if cf.claude { c.run } else { c.wait });
    let mut job =
        LayoutJob::single_section(text.join(" "), TextFormat { font_id: FontId::proportional(12.5), color: c.fg, ..Default::default() });
    job.wrap = TextWrapping::truncate_at_width((right - left - 18.0).max(0.0));
    let g = ui.fonts_mut(|f| f.layout_job(job));
    p.galley(pos2(left + 18.0, mid - g.size().y / 2.0), g, c.fg);
    out
}

/// What the staged files are suspected of, with the first few paths.
fn notes_text(app: &App, cf: &crate::changes::Confirm) -> String {
    use michi_core::guard::Secret;
    let shown = cf
        .notes
        .iter()
        .take(3)
        .map(|n| match n.secret {
            Some(Secret::Name(w) | Secret::Content(w)) => format!("{} ({w})", n.path),
            None => format!("{} ({:.0} MB)", n.path, n.size as f64 / (1024.0 * 1024.0)),
        })
        .collect::<Vec<_>>()
        .join(", ");
    let more = cf.notes.len().saturating_sub(3);
    let key = if cf.notes.iter().any(|n| n.secret.is_some()) { "changes.suspect" } else { "changes.large" };
    let mut t = app.tr(key).replace("{files}", &shown);
    if more > 0 {
        t.push_str(&app.tr("changes.and_more").replace("{n}", &more.to_string()));
    }
    t
}

/// A note at the top of `r`, wrapped to its width.
fn note(ui: &Ui, p: &Painter, r: Rect, text: &str) {
    let c = ito_theme::colors();
    let g = ui.fonts_mut(|f| f.layout(text.to_owned(), FontId::proportional(12.0), c.dim, (r.width() - 2.0 * X).max(40.0)));
    p.galley(pos2(r.left() + X, r.top() + 10.0), g, c.dim);
}

/// A piece of the file list, top to bottom.
#[derive(Clone)]
enum Item {
    Head(Side, usize),
    /// The `k`th file, counted over the staged ones then the unstaged.
    File(usize),
    /// A side with no files.
    Empty(Side),
    Cut,
    /// A folder of the tree.
    Dir {
        side: Side,
        path: String,
        name: String,
        depth: usize,
        files: usize,
        folded: bool,
    },
    /// The `k`th file in the tree, under its folders.
    Leaf {
        k: usize,
        name: String,
        depth: usize,
    },
}

impl Item {
    fn height(&self) -> f32 {
        match self {
            Item::Head(..) => HEAD_H,
            Item::File(_) | Item::Empty(_) | Item::Dir { .. } | Item::Leaf { .. } => FILE_H,
            Item::Cut => NOTE_H,
        }
    }

    /// The file the row is, counted as `Changes::at` does.
    fn file(&self) -> Option<usize> {
        match self {
            Item::File(k) | Item::Leaf { k, .. } => Some(*k),
            _ => None,
        }
    }
}

fn items(ch: &Changes, l: &Lists) -> Vec<Item> {
    let mut out = Vec::new();
    let mut k = 0;
    for side in [Side::Staged, Side::Unstaged] {
        let files = l.of(side);
        out.push(Item::Head(side, files.len()));
        if files.is_empty() {
            out.push(Item::Empty(side));
        }
        if ch.tree {
            for row in ch.tree_rows(side) {
                out.push(match row {
                    Row::Dir { path, name, depth, files, folded } => Item::Dir { side, path, name, depth, files, folded },
                    Row::File { index, name, depth } => Item::Leaf { k: k + index, name, depth },
                });
            }
            k += files.len();
        } else {
            for _ in files {
                out.push(Item::File(k));
                k += 1;
            }
        }
    }
    if l.cut {
        out.push(Item::Cut);
    }
    out
}

/// The staged and the unstaged files in `lr`.
fn list(ui: &Ui, app: &App, lr: Rect, live: bool, pointer: Option<Pos2>, act: &mut Option<Act>) {
    let c = ito_theme::colors();
    let ctx = ui.ctx();
    let p = ui.painter_at(lr);
    let ch = &app.changes;
    let l = match &ch.lists {
        None => return note(ui, &p, lr, app.tr("changes.reading")),
        Some(Err(e)) => return note(ui, &p, lr, &app.tr("changes.failed_list").replace("{}", &e.to_string())),
        Some(Ok(l)) => l,
    };
    let items = items(ch, l);
    let pad = 4.0;
    let tops: Vec<f32> = items
        .iter()
        .scan(pad, |y, it| {
            let top = *y;
            *y += it.height();
            Some(top)
        })
        .collect();
    let total = tops.last().copied().unwrap_or(pad) + items.last().map_or(0.0, |it| it.height()) + 12.0;
    let mut scroll = ctx.data(|d| d.get_temp::<f32>(list_scroll_id())).unwrap_or(0.0);
    if live && pointer.is_some_and(|at| lr.contains(at)) {
        scroll -= ui.input(|i| i.smooth_scroll_delta.y);
    }
    // A selection moved by the keys comes into sight.
    if ctx.data(|d| d.get_temp::<usize>(list_seen_id())) != Some(ch.sel) {
        let at = items.iter().position(|it| it.file() == Some(ch.sel));
        if let Some(top) = at.map(|i| tops[i]) {
            if top < scroll + pad {
                // The heading over the first file comes along.
                scroll = top - pad - if at == Some(1) || at == Some(items.len()) { HEAD_H } else { 0.0 };
            } else if top + FILE_H > scroll + lr.height() - pad {
                scroll = top + FILE_H - lr.height() + pad;
            }
        }
        ctx.data_mut(|d| d.insert_temp(list_seen_id(), ch.sel));
    }
    scroll = scroll.clamp(0.0, (total - lr.height()).max(0.0));
    ctx.data_mut(|d| d.insert_temp(list_scroll_id(), scroll));

    let y0 = lr.top() - scroll;
    for (it, top) in items.iter().zip(&tops) {
        let rr = Rect::from_min_size(pos2(lr.left(), y0 + top), vec2(lr.width(), it.height()));
        if rr.bottom() < lr.top() {
            continue;
        }
        if rr.top() > lr.bottom() {
            break;
        }
        match it.clone() {
            Item::Head(side, n) => {
                let (word, term) = if side == Side::Staged { ("changes.staged", "staged") } else { ("changes.unstaged", "unstaged") };
                let label = app.term(word, term);
                let r = p.text(pos2(rr.left() + X, rr.bottom() - 8.0), Align2::LEFT_BOTTOM, label, FontId::proportional(11.5), c.dim);
                crate::terms::hover(ui, &p, r, term, &app.lang, "head");
                p.text(pos2(r.right(), rr.bottom() - 8.0), Align2::LEFT_BOTTOM, format!(" · {n}"), FontId::proportional(11.5), c.dim);
            }
            Item::Empty(side) => {
                let word = if side == Side::Staged { "changes.none_staged" } else { "changes.none_unstaged" };
                p.text(pos2(rr.left() + X + 14.0, rr.center().y), Align2::LEFT_CENTER, app.tr(word), FontId::proportional(12.0), c.dim);
            }
            Item::Cut => note(ui, &p, rr, app.tr("changes.cut")),
            Item::Dir { side, path, name, depth, files, folded } => {
                let hot = pointer.is_some_and(|at| rr.contains(at));
                if hot {
                    p.rect_filled(rr, 0.0, c.fg.gamma_multiply(0.03));
                }
                let mid = rr.center().y;
                let x = rr.left() + X + 4.0 + depth as f32 * INDENT;
                let caret = if folded { "▸" } else { "▾" };
                p.text(pos2(x, mid), Align2::LEFT_CENTER, caret, FontId::proportional(11.0), c.dim);
                // The button that stages the whole folder, or takes it out.
                let btn = Rect::from_center_size(pos2(rr.right() - X - 8.0, mid), vec2(22.0, rr.height()));
                let word = if side == Side::Staged { "−" } else { "+" };
                let on = live && pointer.is_some_and(|at| btn.contains(at));
                let ink = if on { accent() } else { c.dim };
                p.text(btn.center(), Align2::CENTER_CENTER, word, FontId::monospace(13.0), ink);
                let count = p.text(pos2(btn.left() - 4.0, mid), Align2::RIGHT_CENTER, files.to_string(), FontId::proportional(11.5), c.dim);
                let text = left_cut(ui, &format!("{name}/"), FontId::proportional(12.5), count.left() - 8.0 - (x + 14.0));
                p.text(pos2(x + 14.0, mid), Align2::LEFT_CENTER, text, FontId::proportional(12.5), c.fg);
                let bid = ui.interact(btn.intersect(lr), Id::new(("changes-dir-stage", side == Side::Staged, &path)), Sense::CLICK);
                let rid = ui.interact(rr.intersect(lr), Id::new(("changes-dir", side == Side::Staged, &path)), Sense::CLICK);
                if live && (hot || bid.hovered()) {
                    ctx.set_cursor_icon(CursorIcon::PointingHand);
                }
                if live && bid.clicked() {
                    *act = Some(Act::Dir(side, path));
                } else if live && rid.clicked() {
                    *act = Some(Act::Fold(side, path));
                }
            }
            Item::File(_) | Item::Leaf { .. } => {
                let (Some(k), leaf) =
                    (it.file(), if let Item::Leaf { name, depth, .. } = it { Some((name.as_str(), *depth)) } else { None })
                else {
                    continue;
                };
                let Some((_, f)) = ch.at(k) else { continue };
                let hot = pointer.is_some_and(|at| rr.contains(at));
                if k == ch.sel {
                    p.rect_filled(rr, 0.0, c.selection());
                    if ch.focus == Focus::Files {
                        p.rect_filled(Rect::from_min_size(rr.min, vec2(2.0, rr.height())), 0.0, accent());
                    }
                } else if ch.is_marked(k) {
                    p.rect_filled(rr, 0.0, c.selection());
                } else if hot {
                    p.rect_filled(rr, 0.0, c.fg.gamma_multiply(0.03));
                }
                list_row(ui, &p, app, f, rr, leaf);
                let resp = ui.interact(rr.intersect(lr), Id::new(("changes-row", k)), Sense::CLICK);
                if live && hot {
                    ctx.set_cursor_icon(CursorIcon::PointingHand);
                }
                if live && resp.double_clicked() {
                    *act = Some(Act::Toggle(k));
                } else if live && (resp.clicked() || resp.secondary_clicked()) {
                    let (shift, ctrl) = ctx.input(|i| (i.modifiers.shift, i.modifiers.command));
                    *act = Some(match () {
                        _ if resp.secondary_clicked() => Act::Select(k),
                        _ if shift => Act::Range(k),
                        _ if ctrl => Act::Pick(k),
                        _ => Act::Select(k),
                    });
                }
                if live {
                    resp.context_menu(|ui| {
                        if let Some(key) = crate::ctxmenu::show(ui, app, michi_core::menu::Kind::File) {
                            *act = Some(Act::Menu(k, key));
                        }
                    });
                }
            }
        }
    }
}

/// A row of the file list: the status, the path (cut from the left) and the
/// counts.
fn list_row(ui: &Ui, p: &Painter, app: &App, f: &File, rr: Rect, leaf: Option<(&str, usize)>) {
    let c = ito_theme::colors();
    let mid = rr.center().y;
    let x = rr.left() + X;
    p.text(pos2(x + 4.0, mid), Align2::CENTER_CENTER, f.status.letter(), FontId::monospace(11.0), status_ink(f.status));
    let right = rr.right() - X;
    let counts_left = match f.counts {
        Some((a, d)) => {
            let dr = p.text(pos2(right, mid), Align2::RIGHT_CENTER, format!("−{d}"), FontId::monospace(11.0), c.err);
            p.text(pos2(dr.left() - 5.0, mid), Align2::RIGHT_CENTER, format!("+{a}"), FontId::monospace(11.0), c.done).left()
        }
        None => p.text(pos2(right, mid), Align2::RIGHT_CENTER, app.tr("detail.binary"), FontId::proportional(11.5), c.dim).left(),
    };
    let (label, indent) = match leaf {
        Some((name, depth)) => (name, (depth as f32 + 1.0) * INDENT),
        None => (f.path.as_str(), 0.0),
    };
    let px = x + 16.0 + indent;
    let path = left_cut(ui, label, FontId::proportional(12.5), counts_left - 8.0 - px);
    p.text(pos2(px, mid), Align2::LEFT_CENTER, path, FontId::proportional(12.5), c.fg);
}

/// The commit message in `br`: its box, and under it the amend and the way
/// to commit.
fn message(ui: &mut Ui, app: &mut App, br: Rect, live: bool, pointer: Option<Pos2>) -> Option<Act> {
    let c = ito_theme::colors();
    let p = ui.painter_at(br);
    p.rect_filled(br, 0.0, c.side);
    p.line_segment([br.left_top() + vec2(0.0, 0.5), br.right_top() + vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
    let head = br.top() + 16.0;
    let cap = kbd(ui, &p, pos2(br.right() - X, head), "c");
    let w = p.text(pos2(cap.left() - 6.0, head), Align2::RIGHT_CENTER, app.tr("changes.write"), FontId::proportional(11.5), c.dim);
    let word = if app.changes.amend { "changes.amend_message" } else { "changes.message" };
    let mut job = LayoutJob::simple_singleline(app.tr(word).to_owned(), FontId::proportional(11.5), c.dim);
    job.wrap = TextWrapping::truncate_at_width((w.left() - 8.0 - br.left() - X).max(0.0));
    let g = ui.fonts_mut(|fo| fo.layout_job(job));
    p.galley(pos2(br.left() + X, head - g.size().y / 2.0), g, c.dim);

    let foot = br.bottom() - 18.0;
    let field = Rect::from_min_max(pos2(br.left() + X - 4.0, head + 12.0), pos2(br.right() - X + 4.0, (foot - 32.0).max(head + 40.0)));
    let focused = ui.memory(|m| m.has_focus(message_id()));
    p.rect_filled(field, CornerRadius::same(5), c.bg);
    let border = if focused { c.border_strong() } else { c.border };
    p.rect_stroke(field, CornerRadius::same(5), Stroke::new(1.0, border), StrokeKind::Inside);
    let hint = app.tr("changes.hint");
    let edit = egui::TextEdit::multiline(&mut app.changes.message)
        .id(message_id())
        .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(8, 6)))
        .hint_text(hint)
        .font(FontId::monospace(12.0))
        .text_color(c.fg)
        .desired_width(field.width())
        .desired_rows(1)
        .min_size(field.size())
        .interactive(live);
    // A long message scrolls inside its box instead of growing out of it over the guide and the commit row.
    let resp = ui
        .scope_builder(egui::UiBuilder::new().max_rect(field), |ui| {
            ui.set_clip_rect(field.intersect(ui.clip_rect()));
            egui::ScrollArea::vertical()
                .id_salt("commit-message")
                .max_height(field.height())
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| ui.add(edit))
                .inner
        })
        .inner;
    app.changes.typing = resp.has_focus();
    if resp.has_focus() {
        app.changes_help();
    }
    if std::mem::take(&mut app.changes.focus_message) {
        resp.request_focus();
    }

    // The guide to writing it (#124): the first line's length, or an example.
    let (guide, warn) = match michi_core::msghelp::guide(&app.changes.message) {
        Guide::Empty => (app.tr("changes.guide_empty").to_owned(), false),
        Guide::Long(n) => (app.tr("changes.guide_long").replace("{}", &n.to_string()), true),
        Guide::Terse => (app.tr("changes.guide_terse").to_owned(), true),
        Guide::Fine(n) => (app.tr("changes.guide_fine").replace("{}", &n.to_string()), false),
    };
    let ink = if warn { c.wait } else { c.dim };
    let mut job = LayoutJob::single_section(guide, TextFormat { font_id: FontId::proportional(11.0), color: ink, ..Default::default() });
    job.wrap = TextWrapping::truncate_at_width((br.width() - 2.0 * X).max(0.0));
    let g = ui.fonts_mut(|f| f.layout_job(job));
    p.galley(pos2(br.left() + X, field.bottom() + 9.0 - g.size().y / 2.0), g, ink);

    // Commit: Ctrl+Enter, or a click.
    let cap = kbd(ui, &p, pos2(br.right() - X, foot), "Ctrl+Enter");
    let staged = app.changes.lists().map_or(0, |l| l.staged.len());
    let word = if app.changes.amend { "changes.do_amend" } else { "changes.do_commit" };
    let ready = (staged > 0 || app.changes.amend) && !app.changes.message.trim().is_empty();
    let ink = if ready { accent() } else { c.dim };
    let w = p.text(pos2(cap.left() - 6.0, foot), Align2::RIGHT_CENTER, app.tr(word), FontId::proportional(12.5), ink);
    let hit = w.union(cap).expand(3.0);
    let out = clicked(ui, &p, hit, "changes-commit", live, pointer).then_some(Act::Commit);
    let text = app.tr(if app.prefs.easy { "changes.staged_n_easy" } else { "changes.staged_n" }).replace("{}", &staged.to_string());
    let mut job = LayoutJob::single_section(text, TextFormat { font_id: FontId::proportional(11.5), color: c.dim, ..Default::default() });
    job.wrap = TextWrapping::truncate_at_width((hit.left() - 10.0 - br.left() - X).max(0.0));
    let g = ui.fonts_mut(|f| f.layout_job(job));
    p.galley(pos2(br.left() + X, foot - g.size().y / 2.0), g, c.dim);
    out
}

/// The selected file's title over its diff: its side, and the switch
/// between one column and side by side.
fn title_row(ui: &Ui, app: &App, side: Side, f: &File, rr: Rect, live: bool, pointer: Option<Pos2>) -> Option<Act> {
    let c = ito_theme::colors();
    let ch = &app.changes;
    let p = ui.painter_at(rr);
    p.rect_filled(rr, 0.0, c.side);
    p.line_segment([rr.left_bottom() - vec2(0.0, 0.5), rr.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
    let mid = rr.center().y;
    let (word, key) = if ch.wide { ("detail.narrow", "Esc") } else { ("detail.wide", "Enter") };
    let cap = kbd(ui, &p, pos2(rr.right() - X, mid), key);
    let w = p.text(pos2(cap.left() - 6.0, mid), Align2::RIGHT_CENTER, app.tr(word), FontId::proportional(12.0), c.dim);
    let hit = w.union(cap).expand(3.0);
    let out = clicked(ui, &p, hit, "changes-wide", live, pointer).then_some(Act::Wide(!ch.wide));
    let x = rr.left() + X;
    let (side_word, side_term) = if side == Side::Staged { ("changes.staged", "staged") } else { ("changes.unstaged", "unstaged") };
    let tag = p.text(pos2(x, mid), Align2::LEFT_CENTER, app.term(side_word, side_term), FontId::proportional(11.5), c.dim);
    crate::terms::hover(ui, &p, tag, side_term, &app.lang, "title");
    let letter =
        p.text(pos2(tag.right() + 10.0, mid), Align2::LEFT_CENTER, f.status.letter(), FontId::monospace(12.0), status_ink(f.status));
    let px = letter.right() + 8.0;
    let mut job = LayoutJob::default();
    let font = FontId::monospace(12.0);
    job.append(&f.path, 0.0, TextFormat { font_id: font.clone(), color: c.fg, ..Default::default() });
    if let Some(from) = &f.from {
        let text = format!("  {}", app.tr("detail.from").replace("{}", from));
        job.append(&text, 0.0, TextFormat { font_id: font, color: c.dim, ..Default::default() });
    }
    let path_w = ui.fonts_mut(|fo| fo.layout_job(job.clone())).size().x;
    let mut end = hit.left() - 10.0;
    // How the lines are picked, or that they can be, while the path keeps
    // its room or some of it.
    let hint = match ch.focus {
        Focus::Lines => Some((app.tr("changes.lines_hint"), accent())),
        Focus::Files if ch.can_pick() => Some((app.tr("changes.lines_enter"), c.dim)),
        Focus::Files => None,
    };
    if let Some((text, ink)) = hint {
        let t = ui.fonts_mut(|fo| fo.layout_no_wrap(text.to_owned(), FontId::proportional(12.0), ink));
        if end - t.size().x - 12.0 >= px + path_w.min(PATH_MIN) {
            p.galley(pos2(end - t.size().x, mid - t.size().y / 2.0), t.clone(), ink);
            end -= t.size().x + 12.0;
        }
    }
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
    /// The file is a picture: before and after.
    Image,
}

/// The selected file's diff in `body`, scrolled by the wheel and by the
/// `pages` the keys asked for, or to the `jump`th next (previous) change;
/// the cursor is brought into sight. A strip on the right (the minimap)
/// shows where the changes lie and what is in sight; a click or a drag on it
/// scrolls there.
fn body_of(ui: &Ui, app: &App, body: Rect, live: bool, pages: i32, jump: i32) {
    let c = ito_theme::colors();
    let ctx = ui.ctx();
    let p = ui.painter_at(body);
    let ch = &app.changes;
    let s = match ch.diff() {
        None => return note(ui, &p, body, app.tr("changes.diff_reading")),
        Some(Err(e)) => return note(ui, &p, body, &app.tr("changes.diff_failed").replace("{}", &e.to_string())),
        Some(Ok(None)) => return note(ui, &p, body, app.tr("changes.gone")),
        Some(Ok(Some(s))) => s,
    };
    let (f, rows) = (&s.diff.file, &s.rows);
    let mut pieces = Vec::new();
    if f.not_utf8 {
        pieces.push(Piece::Note("detail.not_utf8"));
    }
    if s.diff.patch.is_none() {
        pieces.push(Piece::Note("changes.whole_note"));
    }
    if michi_core::lfs::of_file(f).is_some() {
        pieces.push(Piece::Note("detail.lfs_note"));
    }
    let image = f.is_binary() && michi_core::imagediff::is_image(&f.path);
    if image {
        pieces.push(Piece::Image);
    } else if f.is_binary() {
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
    // The minimap takes the right edge of the body while there is a diff.
    let (body, strip) = if pieces.iter().any(|piece| matches!(piece, Piece::Diff)) {
        let x = body.right() - MINIMAP_W;
        (Rect::from_min_max(body.min, pos2(x, body.bottom())), Some(Rect::from_min_max(pos2(x, body.top()), body.max)))
    } else {
        (body, None)
    };
    let wrap = Wrap::new(ui, body.width(), ch.wide);
    let tops = wrap.tops(rows);
    let image_job =
        ch.wt.as_deref().zip(ch.selected()).map(|(dir, (side, file))| crate::images::changes_job(dir, side, file, ch.shown_gen));
    let heights: Vec<f32> = pieces
        .iter()
        .map(|piece| match piece {
            Piece::Image => image_job.as_ref().map_or(NOTE_H, |job| crate::images::height(app, job, body.width())),
            Piece::Note(_) => NOTE_H,
            Piece::Diff => tops.last().copied().unwrap_or(0.0) + 6.0,
            Piece::Cut => DIFF_H + 6.0,
        })
        .collect();
    let total: f32 = heights.iter().sum::<f32>() + 24.0;
    let diff_top: f32 = pieces.iter().zip(&heights).take_while(|(piece, _)| !matches!(piece, Piece::Diff)).map(|(_, h)| h).sum();

    let mut scroll = ctx.data(|dt| dt.get_temp::<f32>(scroll_id())).unwrap_or(0.0);
    scroll += pages as f32 * (body.height() - 40.0).max(40.0);
    if live && ui.rect_contains_pointer(body) {
        scroll -= ui.input(|i| i.smooth_scroll_delta.y);
    }
    let runs = michi_core::minimap::runs(&rows.marks(ch.wide, f));
    // `n` / `N`: on to the next (previous) change, counted from the one the
    // last jump went to while it is in sight, else from what is in sight.
    if jump != 0 && ch.focus != Focus::Lines && !runs.is_empty() {
        let starts: Vec<f32> = runs.iter().map(|r| diff_top + tops[r.start]).collect();
        let mut at = ctx
            .data(|dt| dt.get_temp::<Option<usize>>(run_id()))
            .flatten()
            .filter(|&i| starts.get(i).is_some_and(|&y| y >= scroll && y <= scroll + body.height()));
        for _ in 0..jump.abs() {
            at = match at {
                Some(i) if jump > 0 => Some((i + 1).min(starts.len() - 1)),
                Some(i) => Some(i.saturating_sub(1)),
                None => michi_core::minimap::jump(&starts, scroll + JUMP_MARGIN, jump, 1.0),
            };
        }
        if let Some(i) = at {
            scroll = starts[i] - JUMP_MARGIN;
        }
        ctx.data_mut(|dt| dt.insert_temp(run_id(), at));
    }
    // A click or a drag on the minimap.
    let strip_resp = strip.map(|r| ui.interact(r, Id::new("changes-minimap"), Sense::click_and_drag()));
    if let (Some(r), Some(resp)) = (strip, &strip_resp) {
        if live && resp.hovered() {
            ctx.set_cursor_icon(CursorIcon::PointingHand);
        }
        if live && resp.is_pointer_button_down_on() {
            if let Some(at) = resp.interact_pointer_pos() {
                scroll = michi_core::minimap::click_to_scroll((at.y - r.top()) / r.height(), total, body.height());
            }
        }
    }
    // The cursor moved by the keys comes into sight.
    let cursor = ch.cursor_line();
    if ctx.data(|dt| dt.get_temp::<Option<(usize, usize)>>(cursor_seen_id())) != Some(cursor) {
        if let Some(k) = cursor.and_then(|at| row_of(ch, s, at)) {
            let (top, bottom) = (diff_top + tops[k], diff_top + tops[k + 1]);
            let room = body.height() - 3.0 * DIFF_H;
            if top < scroll + DIFF_H {
                scroll = top - DIFF_H;
            } else if bottom > scroll + room {
                scroll = bottom - room;
            }
        }
        ctx.data_mut(|dt| dt.insert_temp(cursor_seen_id(), cursor));
    }
    scroll = scroll.clamp(0.0, (total - body.height()).max(0.0));
    ctx.data_mut(|dt| dt.insert_temp(scroll_id(), scroll));

    // A drag over the text picks it up; nothing else in the body takes a click.
    let grab = ui.interact(body, Id::new("changes-select"), Sense::click_and_drag());
    let side = ch.selected().map_or(String::new(), |(side, _)| format!("{side:?}"));
    crate::select::begin(&format!("changes:{:?}:{side}:{}", ch.wt, f.path));
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
                let dr = Rect::from_min_size(rr.min, vec2(rr.width(), rr.height() - 6.0));
                diff(ui, &p, f, rows, dr, body, &wrap, &crate::detail_ui::Marks::default());
                marks(&p, ch, s, dr, body, &tops);
            }
            Piece::Image => {
                if let Some(job) = &image_job {
                    crate::images::draw(ui, &p, app, job, rr, live);
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
    if live {
        let driven = crate::select::drive(&grab, body);
        if let Some(text) = driven.copy {
            ctx.copy_text(text);
        }
        if let Some(url) = driven.open {
            std::thread::spawn(move || {
                let _ = michi_core::pr::open_url(&url);
            });
        }
        if ui.input(|i| i.modifiers.command) && grab.hover_pos().is_some_and(|at| crate::select::url_at(at, body)) {
            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }
    crate::select::paint();
    if let Some(r) = strip {
        minimap(&p, r, &michi_core::minimap::ticks(&runs, &tops, diff_top, total, r.height(), 2.0), scroll, total, body.height());
    }
}

/// The minimap in `r`: a tick for each run of changes (green added, red
/// deleted, amber both), and what is in sight over them.
fn minimap(p: &Painter, r: Rect, ticks: &[michi_core::minimap::Tick], scroll: f32, total: f32, view: f32) {
    use michi_core::minimap::Mark;
    let c = ito_theme::colors();
    p.rect_filled(r, 0.0, c.side);
    p.line_segment([r.left_top() + vec2(0.5, 0.0), r.left_bottom() + vec2(0.5, 0.0)], Stroke::new(1.0, c.border));
    for t in ticks {
        let ink = match t.mark {
            Mark::Add => c.done,
            Mark::Del => c.err,
            Mark::Both | Mark::Same => c.wait,
        };
        p.rect_filled(Rect::from_min_max(pos2(r.left() + 3.0, r.top() + t.y0), pos2(r.right() - 2.0, r.top() + t.y1)), 0.0, ink);
    }
    if total > view {
        let y0 = scroll / total * r.height();
        let y1 = (y0 + view / total * r.height()).max(y0 + 6.0).min(r.height());
        let thumb = Rect::from_min_max(pos2(r.left() + 1.0, r.top() + y0), pos2(r.right(), r.top() + y1));
        p.rect_filled(thumb, 0.0, c.fg.gamma_multiply(0.10));
        p.rect_stroke(thumb, 0.0, Stroke::new(1.0, c.fg.gamma_multiply(0.35)), StrokeKind::Inside);
    }
}

/// The row that shows the line `at`, in the way the diff is shown.
fn row_of(ch: &Changes, s: &Shown, (h, l): (usize, usize)) -> Option<usize> {
    let at = (h as u32, l as u32);
    if ch.wide {
        s.rows.paired.iter().position(|r| matches!(*r, Paired::Pair(old, new) if old == Some(at) || new == Some(at)))
    } else {
        s.rows.unified.iter().position(|r| *r == Unified::Line(at))
    }
}

/// Over the diff in `dr`, while the keys are on its lines: the lines picked
/// tinted, the cursor's line with a strip on its left.
fn marks(p: &Painter, ch: &Changes, s: &Shown, dr: Rect, view: Rect, tops: &[f32]) {
    if ch.focus != Focus::Lines {
        return;
    }
    let c = ito_theme::colors();
    let picked = ch.picked();
    let cursor = ch.cursor_line();
    let first = tops.partition_point(|&t| dr.top() + t <= view.top()).saturating_sub(1);
    let last = tops.partition_point(|&t| dr.top() + t < view.bottom()).min(tops.len() - 1);
    let half = ((dr.width() - 1.0) / 2.0).floor();
    for k in first..last {
        let rr = Rect::from_min_max(pos2(dr.left(), dr.top() + tops[k]), pos2(dr.right(), dr.top() + tops[k + 1]));
        // The parts of the row, each with its line.
        let parts: Vec<(Rect, (u32, u32))> = if ch.wide {
            match s.rows.paired[k] {
                Paired::Hunk(_) => Vec::new(),
                Paired::Pair(old, new) => {
                    let left = Rect::from_min_max(rr.min, pos2(rr.left() + half, rr.bottom()));
                    let right = Rect::from_min_max(pos2(left.right() + 1.0, rr.top()), rr.max);
                    // A line on both sides is unchanged: nothing to pick.
                    if old == new {
                        Vec::new()
                    } else {
                        old.map(|at| (left, at)).into_iter().chain(new.map(|at| (right, at))).collect()
                    }
                }
            }
        } else {
            match s.rows.unified[k] {
                Unified::Hunk(_) => Vec::new(),
                Unified::Line(at) => vec![(rr, at)],
            }
        };
        for (part, (h, l)) in parts {
            let at = (h as usize, l as usize);
            if picked.contains(&at) {
                p.rect_filled(part, 0.0, c.blue.gamma_multiply(0.16));
            }
            if cursor == Some(at) {
                p.rect_stroke(part, 0.0, Stroke::new(1.0, accent().gamma_multiply(0.7)), StrokeKind::Inside);
                p.rect_filled(Rect::from_min_size(part.min, vec2(3.0, part.height())), 0.0, accent());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::View;
    use crate::changes::tests::{lists, two_hunks};
    use crate::changes::Did;
    use crate::graph::fixture;
    use crate::repo::Answer;
    use michi_core::claude::{Session, State};
    use michi_core::stage;
    use michi_core::summary::{Head, Summary};
    use std::path::PathBuf;

    /// The `2` view on the worktree `wip`, its files not read yet.
    fn opened() -> App {
        let mut app = App::new(crate::prefs::Prefs::empty(), "C:/r/wt".into());
        app.lang = "en".into();
        // The stage diagram would move the rows these tests click at.
        app.prefs.guides.dismiss(Topic::Stage);
        let top = PathBuf::from("C:/r/wt");
        app.take(Answer::Summary(Ok(Summary { name: "r".into(), head: Head::Branch("wip".into()), worktrees: 3, top })));
        app.take(Answer::Trees(fixture::trees()));
        assert!(app.feed('2'));
        app.changes_sync();
        assert_eq!(app.view, View::Changes);
        app
    }

    fn take_list(app: &mut App, l: Lists) {
        let gen = app.changes.gens().0;
        app.take(Answer::ChangesList { gen, result: Ok(Box::new(l)) });
    }

    fn take_file(app: &mut App, d: Option<stage::Diff>) {
        let gen = app.changes.gens().1;
        app.take(Answer::ChangesFile { gen, result: Ok(d.map(Box::new)) });
    }

    /// One frame of the view `width` wide; the texts drawn.
    fn frame(ctx: &egui::Context, app: &mut App, width: f32, events: Vec<egui::Event>) -> Vec<String> {
        let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(width, 560.0))), events, ..Default::default() };
        let mut out = ctx.run_ui(input, |ui| {
            let r = ui.max_rect();
            show(ui, app, r, true);
        });
        out.textures_delta.clear();
        let text = |s: &egui::Shape| match s {
            egui::Shape::Text(t) => Some(t.galley.text().to_owned()),
            _ => None,
        };
        out.shapes.iter().filter_map(|c| text(&c.shape)).collect()
    }

    #[test]
    fn the_tree_draws_its_folders_and_t_switches_it() {
        let ctx = egui::Context::default();
        let mut app = opened();
        take_list(&mut app, lists(&["d/s.txt"], &["d/e/a.txt", "d/e/b.txt", "top.txt"]));
        assert!(app.feed('t'));
        assert!(app.changes.tree);
        let drawn = frame(&ctx, &mut app, 900.0, Vec::new());
        assert!(drawn.iter().any(|t| t == "d/e/"), "the folder row: {drawn:?}");
        assert!(drawn.iter().any(|t| t == "a.txt"), "the file under it: {drawn:?}");
        app.changes.fold(Side::Unstaged, "d/e");
        let drawn = frame(&ctx, &mut app, 900.0, Vec::new());
        assert!(!drawn.iter().any(|t| t == "a.txt"), "folded away: {drawn:?}");
        assert!(app.feed('t'));
        assert!(!app.changes.tree);
    }

    #[test]
    fn the_view_draws_in_every_state() {
        let ctx = egui::Context::default();
        let mut app = opened();
        assert_eq!(app.changes.wt.as_deref(), Some(std::path::Path::new("C:/r/wt")), "the window's own worktree first");
        frame(&ctx, &mut app, 900.0, Vec::new());
        take_list(&mut app, lists(&["s.txt"], &["a.txt", "b.txt"]));
        frame(&ctx, &mut app, 900.0, Vec::new());
        take_file(&mut app, Some(two_hunks("s.txt")));
        for width in [1400.0, 900.0, 420.0] {
            frame(&ctx, &mut app, width, Vec::new());
        }
        // The lines, picked by a range, one column and side by side.
        assert!(app.feed('j'));
        take_file(&mut app, Some(two_hunks("a.txt")));
        assert!(app.feed('l'));
        assert_eq!(app.changes.focus, Focus::Lines);
        assert!(app.feed('v'));
        assert!(app.feed('j'));
        assert!(app.feed('j'));
        assert_eq!(app.changes.picked().len(), 3);
        let drawn = frame(&ctx, &mut app, 900.0, Vec::new());
        assert!(
            drawn.iter().any(|t| t == "Space in or out · v range · a hunk · s split · d throw away · h back"),
            "how lines are picked: {drawn:?}"
        );
        assert!(app.named("Enter"));
        frame(&ctx, &mut app, 1400.0, Vec::new());
        assert!(app.escape(), "Esc leaves the lines first");
        assert_eq!(app.changes.focus, Focus::Files);
        assert!(app.escape(), "then the side by side");
        assert!(!app.changes.wide);
        // A whole file, a file gone, a diff not read, nothing to show.
        let mut whole = two_hunks("a.txt");
        whole.patch = None;
        take_file(&mut app, Some(whole));
        frame(&ctx, &mut app, 900.0, Vec::new());
        let binary = stage::Diff { file: File { counts: None, ..crate::changes::tests::file("blob.bin") }, patch: None };
        take_file(&mut app, Some(binary));
        let drawn = frame(&ctx, &mut app, 900.0, Vec::new());
        for note in ["This file goes in whole: its lines cannot be picked", "A binary file: no diff to show"] {
            assert!(drawn.iter().any(|t| t == note), "a binary file goes in whole: {drawn:?}");
        }
        // A picture is read on the side, not called a binary file.
        let picture = stage::Diff { file: File { counts: None, ..crate::changes::tests::file("logo.png") }, patch: None };
        take_file(&mut app, Some(picture));
        let drawn = frame(&ctx, &mut app, 900.0, Vec::new());
        assert!(drawn.iter().any(|t| t == "Reading the pictures..."), "a picture is asked for: {drawn:?}");
        assert!(!drawn.iter().any(|t| t == "A binary file: no diff to show"), "{drawn:?}");
        take_file(&mut app, None);
        frame(&ctx, &mut app, 900.0, Vec::new());
        let gen = app.changes.gens().1;
        app.take(Answer::ChangesFile { gen, result: Err(michi_core::git::Error::Git("bad".into())) });
        frame(&ctx, &mut app, 900.0, Vec::new());
        let mut l = lists(&[], &[]);
        l.cut = true;
        take_list(&mut app, l);
        frame(&ctx, &mut app, 900.0, Vec::new());
        let gen = app.changes.gens().0;
        app.take(Answer::ChangesList { gen, result: Err(michi_core::git::Error::Git("not a repo".into())) });
        frame(&ctx, &mut app, 900.0, Vec::new());
        // A long list scrolls to the selection.
        let many: Vec<String> = (0..60).map(|k| format!("f{k}.txt")).collect();
        let many: Vec<&str> = many.iter().map(String::as_str).collect();
        take_list(&mut app, lists(&many[..30], &many[30..]));
        assert!(app.named("End"));
        frame(&ctx, &mut app, 900.0, Vec::new());
        assert!(ctx.data(|d| d.get_temp::<f32>(list_scroll_id())).unwrap() > 500.0);
    }

    #[test]
    fn a_claude_writing_asks_for_a_yes_under_the_top_bar() {
        let ctx = egui::Context::default();
        let mut app = opened();
        let s = Session {
            id: 3,
            state: State::Running,
            command: "claude".into(),
            agent: "claude".into(),
            cwd: "C:/r/wt".into(),
            branch: "wip".into(),
            title: "fix".into(),
            note: String::new(),
            since_ms: 0,
            read_only: false,
        };
        app.take(Answer::Claude(Some(vec![(PathBuf::from("C:/r/wt"), vec![s])])));
        take_list(&mut app, lists(&[], &["a.txt"]));
        assert!(app.feed(' '));
        assert!(app.changes.confirm.as_ref().is_some_and(|c| c.claude));
        frame(&ctx, &mut app, 900.0, Vec::new());
        assert!(app.feed('x'), "the other keys wait for the answer");
        assert!(app.changes.confirm.is_some());
        assert!(app.escape());
        assert!(app.changes.confirm.is_none(), "Esc says no");
        assert!(app.feed(' '));
        assert!(app.feed('y'));
        assert!(app.changes.confirm.is_none() && app.changes.claude_ok, "y says yes, and it holds");
        frame(&ctx, &mut app, 900.0, Vec::new());
    }

    #[test]
    fn g_holds_the_losing_keys_and_y_f_copies_the_path() {
        let mut app = opened();
        take_list(&mut app, lists(&[], &["dir/a.txt"]));
        app.feed('d');
        assert!(app.changes.confirm.is_none(), "d alone asks nothing now");
        assert!(app.feed('g'));
        assert_eq!(app.prefix, Some(crate::prefix::Prefix::Go));
        assert!(app.escape());
        assert!(app.feed('y') && app.feed('n'));
        assert_eq!(app.prefix, None);
        assert!(app.feed('g') && app.feed('d'));
        assert!(app.changes.confirm.is_some());
        assert!(app.feed('y'), "y answers the question, no window");
        assert_eq!(app.prefix, None);
    }

    #[test]
    fn d_asks_what_goes_and_says_where_it_is_kept() {
        let ctx = egui::Context::default();
        let mut app = opened();
        take_list(&mut app, lists(&[], &["a.txt"]));
        assert!(app.feed('g') && app.feed('d'));
        let drawn = frame(&ctx, &mut app, 1400.0, Vec::new());
        let ask = "Throw away the changes to a.txt? They are kept under refs/michi/backup/ first.";
        assert!(drawn.iter().any(|t| t == ask), "{drawn:?}");
        assert!(app.escape());
        assert!(app.changes.confirm.is_none(), "Esc says no");
        assert!(app.feed('g') && app.feed('d') && app.feed('y'));
        let saved = "refs/michi/backup/2026-10-16T10-00-00".to_owned();
        let did = Did::Discarded { path: "a.txt".into(), untracked: false, lines: 3, saved };
        app.take(Answer::ChangesDone { wt: "C:/r/wt".into(), commit: false, result: Ok(did) });
        assert_eq!(
            app.toast.as_ref().map(|(t, _)| t.as_str()),
            Some("Threw away 3 lines of a.txt; kept in michi/backup/2026-10-16T10-00-00")
        );
    }

    #[test]
    fn a_refused_commit_is_run_again_with_r_over_what_git_said() {
        let ctx = egui::Context::default();
        let mut app = opened();
        take_list(&mut app, lists(&["s.txt"], &["a.txt"]));
        take_file(&mut app, Some(two_hunks("s.txt")));
        let wt = PathBuf::from("C:/r/wt");
        let told = || michi_core::git::Error::Told { line: "lint".into(), all: "lint\nmore".into() };
        // A write that is not a commit: `r` does nothing.
        app.take(Answer::ChangesDone { wt: wt.clone(), commit: false, result: Err(told()) });
        app.toast = None;
        app.feed('r');
        assert!(app.toast.is_none(), "{:?}", app.toast);
        // A refused commit: `r` runs the commit again (no message, so it says so).
        app.take(Answer::ChangesDone { wt, commit: true, result: Err(told()) });
        take_list(&mut app, lists(&["s.txt"], &["a.txt"]));
        app.toast = None;
        assert!(app.feed('r'));
        assert_eq!(app.toast.as_ref().map(|(t, _)| t.as_str()), Some("Write a commit message first (c)"));
        let drawn = frame(&ctx, &mut app, 900.0, Vec::new());
        assert!(drawn.iter().any(|t| t == "Run again"), "{drawn:?}");
    }

    #[test]
    fn all_a_hook_said_is_shown_in_place_of_the_diff_until_put_away() {
        let ctx = egui::Context::default();
        let mut app = opened();
        take_list(&mut app, lists(&["s.txt"], &["a.txt", "b.txt"]));
        take_file(&mut app, Some(two_hunks("s.txt")));
        let wt = PathBuf::from("C:/r/wt");
        let told = || michi_core::git::Error::Told { line: "lint: 2 problems".into(), all: "lint: 2 problems\n  a.rs:1 unused".into() };
        let fail = |app: &mut App, commit: bool, e: michi_core::git::Error| {
            app.take(Answer::ChangesDone { wt: wt.clone(), commit, result: Err(e) });
        };
        fail(&mut app, true, told());
        assert_eq!(app.toast.as_ref().map(|(t, _)| t.as_str()), Some("Could not write: lint: 2 problems"), "the first line in the toast");
        let drawn = frame(&ctx, &mut app, 900.0, Vec::new());
        for text in ["Could not commit: all git and the hooks said", "lint: 2 problems\n  a.rs:1 unused", "Close"] {
            assert!(drawn.iter().any(|t| t == text), "{text:?} in {drawn:?}");
        }
        assert!(!drawn.iter().any(|t| t.starts_with("@@")), "the diff is not drawn: {drawn:?}");
        assert!(app.escape());
        assert!(app.changes.told.is_none(), "Esc puts it away");
        // Another file puts it away too; so does a write that goes through.
        fail(&mut app, false, told());
        let drawn = frame(&ctx, &mut app, 900.0, Vec::new());
        assert!(drawn.iter().any(|t| t == "Could not write: all git said"), "{drawn:?}");
        assert!(app.feed('j'));
        assert!(app.changes.told.is_none());
        fail(&mut app, true, told());
        app.take(Answer::ChangesDone { wt: wt.clone(), commit: false, result: Ok(Did::Staged) });
        assert!(app.changes.told.is_none());
        // A line of git's goes in the toast alone; another worktree's words
        // are not shown here.
        fail(&mut app, true, michi_core::git::Error::Git("nothing to commit".into()));
        assert!(app.changes.told.is_none());
        app.take(Answer::ChangesDone { wt: "C:/r/repo".into(), commit: true, result: Err(told()) });
        assert!(app.changes.told.is_none());
    }

    #[test]
    fn a_click_selects_a_file_and_a_chip_switches_the_worktree() {
        let ctx = egui::Context::default();
        let mut app = opened();
        take_list(&mut app, lists(&["s.txt"], &["a.txt", "b.txt"]));
        frame(&ctx, &mut app, 900.0, Vec::new());
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
        // Under the top bar: the staged heading and file, the unstaged
        // heading, then a.txt and b.txt.
        let top = PAD_TOP + TOP_H + 8.0 + 1.0 + 4.0;
        let row = top + HEAD_H + FILE_H + HEAD_H + FILE_H + FILE_H / 2.0;
        click(&mut app, pos2(120.0, row));
        assert_eq!(app.changes.sel, 2, "b.txt");
        // The chips: repo, then wip, which is shown.
        click(&mut app, pos2(PAD_X + 5.0, PAD_TOP + TOP_H / 2.0));
        assert_eq!(app.changes.wt.as_deref(), Some(std::path::Path::new("C:/r/repo")), "another worktree");
        assert!(app.changes.lists.is_none(), "its files are read anew");
        assert!(app.feed(']'));
    }

    #[test]
    fn a_drag_over_the_diff_picks_its_text_up() {
        let ctx = egui::Context::default();
        let mut app = opened();
        take_list(&mut app, lists(&["s.txt"], &["a.txt"]));
        take_file(&mut app, Some(two_hunks("s.txt")));
        frame(&ctx, &mut app, 1400.0, Vec::new());
        let ev = |at: Pos2, pressed| egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let (from, to) = (pos2(500.0, 120.0), pos2(1300.0, 400.0));
        frame(&ctx, &mut app, 1400.0, vec![egui::Event::PointerMoved(from), ev(from, true)]);
        frame(&ctx, &mut app, 1400.0, vec![egui::Event::PointerMoved(to)]);
        frame(&ctx, &mut app, 1400.0, vec![ev(to, false)]);
        frame(&ctx, &mut app, 1400.0, Vec::new());
        let got = crate::select::text().expect("the drag picked text up");
        assert!(!got.is_empty());
    }
}
