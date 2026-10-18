//! The window: the title bar, the view, the keybar and the key list. Drawn
//! with the painter on the rects it lays out itself, so nothing here scrolls
//! (docs/design.md: the graph's is the one scrollbar, in `graph_ui.rs`). The
//! look follows docs/mock.

use std::time::{Duration, SystemTime};

use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, ViewportCommand};
use ito_common::Clock;
use ito_layout::ui::Moved;
use ito_layout::{Dir, Node};
use michi_core::branch::Doing;
use michi_core::git;
use michi_core::strength::Strength;
use michi_core::summary::Head;

use crate::app::{failed_word, App, Mode, Repo, View};
use crate::graph_ui::{paint_icon, Icon};
use crate::sizes::Sizes;
use crate::work::{self, Kind};

/// The accent of the tabs, the chip's name and the best key: main's cyan,
/// the colour of the left lane in the graph.
pub fn accent() -> Color32 {
    ito_theme::colors().run
}

const TITLE_H: f32 = 36.0;
const KEYBAR_H: f32 = 24.0;
/// The bar of the tabs under the title bar.
const TABS_H: f32 = 28.0;
/// The widest a tab grows, and the narrowest it is squeezed to.
const TAB_MAX: f32 = 220.0;
const TAB_MIN: f32 = 70.0;
/// The strip under the title bar for a worktree on no branch.
const STRIP_H: f32 = 28.0;
/// The line under the title bar that says where the worktree stands (#107).
const WHERE_H: f32 = 22.0;
/// How many of a view's keys the keybar shows before "? all keys".
const KEYBAR_KEYS: usize = 5;
/// How far in from the window's sides and foot the pointer resizes it.
const EDGE: f32 = 6.0;

/// What the frame asks of the app.
/// What a click on a keybar entry stands for: the key typed, or Esc.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarKey {
    Char(char),
    Esc,
}

/// The key a keybar entry (`keys`, as the tables spell it) stands for when it
/// is clicked; `None` for the ones that are not one key (`j k`, `Ctrl+Tab`).
pub fn bar_key(keys: &str) -> Option<BarKey> {
    match keys {
        "Esc" | "? Esc" => Some(BarKey::Esc),
        "Space" => Some(BarKey::Char(' ')),
        k => {
            let mut cs = k.chars();
            match (cs.next(), cs.next()) {
                (Some(c), None) => Some(BarKey::Char(c)),
                _ => None,
            }
        }
    }
}

pub struct Out {
    pub view: Option<View>,
    pub settings: bool,
    pub palette: bool,
    /// The bell was clicked.
    pub bell: bool,
    /// A key of the keybar that was clicked (#123).
    pub key: Option<BarKey>,
    /// The calm save button was pressed (#113).
    pub calm: bool,
    /// The suggested next step's button was pressed (#119).
    pub next: bool,
    /// Between the title bar and the keybar: where the settings go.
    pub body: Rect,
}

/// The title bar, the keybar and the view between them; under the settings,
/// the body is left to them.
pub fn frame(ui: &mut Ui, app: &mut App) -> Out {
    let c = ito_theme::colors();
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, c.bg);
    let title = Rect::from_min_size(rect.min, vec2(rect.width(), TITLE_H));
    let keybar = Rect::from_min_max(pos2(rect.left(), rect.bottom() - KEYBAR_H), rect.max);
    // Not on a branch: the strip under the title bar (#109).
    // Two tabs or more: the bar of the window's repositories (#45).
    let tabs = (app.tab_labels.len() > 1 && app.mode != Mode::Settings)
        .then(|| Rect::from_min_size(title.left_bottom(), vec2(rect.width(), TABS_H)));
    let under = title.left_bottom() + vec2(0.0, tabs.map_or(0.0, |t| t.height()));
    let strip = (app.mode != Mode::Settings && app.strip_shown()).then(|| Rect::from_min_size(under, vec2(rect.width(), STRIP_H)));
    // Otherwise a plain line on where the worktree stands (#107).
    let here = if app.mode == Mode::Settings || strip.is_some() { None } else { app.where_text() };
    let here_r = here.as_ref().map(|_| Rect::from_min_size(under, vec2(rect.width(), WHERE_H)));
    let top = under + vec2(0.0, strip.map_or(0.0, |s| s.height()) + here_r.map_or(0.0, |r| r.height()));
    let body = Rect::from_min_max(top, keybar.right_top());
    let mut out = Out { view: None, settings: false, palette: false, bell: false, key: None, calm: false, next: false, body };
    if app.mode != Mode::Settings {
        // The key list goes over the view; the mouse is the list's then.
        // The columns stay clear of the window's resize edge.
        let live = app.mode == Mode::Main;
        // A job waiting for a yes is asked over the view.
        band(ui, app, live);
        // A view that has just been switched to fades in and slides up a few px (#88).
        let shown = crate::motion::enter(ui.ctx(), app.prefs.motion, egui::Id::new(("michi-view", app.view as u8, app.review.is_some())));
        let slide = app.prefs.motion.offset(shown);
        let r = Rect::from_min_max(body.left_top() + vec2(0.0, slide), pos2(body.right() - EDGE, body.bottom()));
        ui.set_opacity(shown);
        if app.view == View::Worktree && app.review.is_some() {
            crate::review_ui::show(ui, app, r, live);
        } else if app.view == View::Worktree {
            crate::wt_ui::show(ui, app, r, live);
        } else if app.view == View::Graph {
            if matches!(app.repo, Repo::Failed(_)) {
                crate::graph_ui::show(ui, app, r, live);
            } else {
                graph_view(ui, app, r, live);
            }
        } else if app.view == View::Changes {
            crate::changes_ui::show(ui, app, r, live);
        } else if app.view == View::Conflict {
            crate::conflict_ui::show(ui, app, r, live);
        } else {
            empty_view(ui, app, body);
        }
        ui.set_opacity(1.0);
        if app.mode == Mode::Help {
            help(ui, app, body);
        }
    }
    if let Some(s) = strip {
        strip_bar(ui, app, s, app.mode == Mode::Main && app.band.is_none() && app.picker.is_none());
    }
    if let (Some(r), Some(text)) = (here_r, here) {
        where_bar(ui, r, text);
    }
    if let Some(t) = tabs {
        tab_bar(ui, app, t);
    }
    title_bar(ui, app, title, &mut out);
    key_bar(ui, app, keybar, &mut out);
    out
}

/// A small clickable button in the keybar ending at `right`; where it begins
/// and whether it was clicked. Its tooltip says what it does.
fn pill(ui: &mut Ui, id: &str, text: &str, tip: &str, right: f32, mid: f32, color: Color32) -> (f32, bool) {
    let c = ito_theme::colors();
    let g = ui.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), FontId::proportional(12.0), color));
    let rect = Rect::from_min_max(pos2(right - g.size().x - 16.0, mid - 10.0), pos2(right, mid + 10.0));
    let resp = ui.interact(rect, egui::Id::new(("michi-pill", id)), egui::Sense::click()).on_hover_text(tip);
    let p = ui.painter();
    if resp.hovered() {
        p.rect_filled(rect, CornerRadius::same(10), c.border);
    }
    p.rect_stroke(rect, CornerRadius::same(10), Stroke::new(1.0, c.border_strong()), StrokeKind::Inside);
    p.galley(pos2(rect.left() + 8.0, mid - g.size().y / 2.0), g, color);
    (rect.left() - 10.0, resp.clicked())
}

/// The tabs of the window, one repository each: a click brings one to the
/// front, the cross or a middle click closes it (#45).
fn tab_bar(ui: &mut Ui, app: &mut App, r: Rect) {
    let c = ito_theme::colors();
    let p = ui.painter().clone();
    p.rect_filled(r, 0.0, c.panel);
    p.line_segment([r.left_bottom() - vec2(0.0, 0.5), r.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
    let n = app.tab_labels.len().max(1) as f32;
    let w = ((r.width() - EDGE - 8.0) / n).clamp(TAB_MIN, TAB_MAX);
    let font = FontId::proportional(12.0);
    let live = app.mode == Mode::Main && app.band.is_none() && app.picker.is_none();
    let mut asked = None;
    for (i, label) in app.tab_labels.iter().enumerate() {
        let cell = Rect::from_min_size(pos2(r.left() + 4.0 + w * i as f32, r.top()), vec2(w, r.height()));
        let front = i == app.tab_active;
        let resp = ui.interact(cell, ui.id().with(("tab", i)), Sense::click());
        if front {
            p.rect_filled(cell.shrink2(vec2(1.0, 0.0)), 0.0, c.bg);
            p.line_segment([cell.left_top() + vec2(0.0, 1.0), cell.right_top() + vec2(0.0, 1.0)], Stroke::new(2.0, accent()));
        } else if resp.hovered() {
            p.rect_filled(cell.shrink2(vec2(1.0, 0.0)), 0.0, c.bg.gamma_multiply(0.5));
        }
        // The cross: for the front tab and the one under the pointer.
        let cross = Rect::from_center_size(pos2(cell.right() - 14.0, cell.center().y), vec2(16.0, 16.0));
        let x = (front || resp.hovered()).then(|| ui.interact(cross, ui.id().with(("tab-x", i)), Sense::click()));
        let color = if front { c.fg } else { c.dim };
        let mut job = egui::text::LayoutJob::simple_singleline(label.clone(), font.clone(), color);
        job.wrap =
            egui::text::TextWrapping {
                max_width: (w - 34.0).max(20.0), max_rows: 1, break_anywhere: true, overflow_character: Some('…')
            };
        let g = ui.fonts_mut(|f| f.layout_job(job));
        p.galley(pos2(cell.left() + 10.0, cell.center().y - g.size().y / 2.0), g, color);
        if let Some(x) = &x {
            let k = if x.hovered() { c.fg } else { c.dim };
            let m = cross.center();
            p.line_segment([m + vec2(-3.5, -3.5), m + vec2(3.5, 3.5)], Stroke::new(1.2, k));
            p.line_segment([m + vec2(-3.5, 3.5), m + vec2(3.5, -3.5)], Stroke::new(1.2, k));
            if x.hovered() {
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if live {
            if x.as_ref().is_some_and(|x| x.clicked()) || resp.middle_clicked() {
                asked = Some(michi_core::tabs::Op::CloseAt(i));
            } else if resp.clicked() && !front {
                asked = Some(michi_core::tabs::Op::Goto(i));
            }
        }
    }
    if asked.is_some() {
        app.tab_req = asked;
    }
}

/// The strip under the title bar for a worktree on no branch: why, and a
/// button (and key) for each way out.
fn strip_bar(ui: &mut Ui, app: &mut App, r: Rect, live: bool) {
    let c = ito_theme::colors();
    let p = ui.painter().clone();
    p.rect_filled(r, 0.0, c.panel);
    p.line_segment([r.left_bottom() - vec2(0.0, 0.5), r.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
    let mid = r.center().y;
    let font = FontId::proportional(12.0);
    let mut right = r.right() - 12.0 - EDGE;
    let mut pressed = None;
    for (key, word) in app.strip_actions().into_iter().rev() {
        let cap = kbd(ui, &p, pos2(right, mid), &key.to_string());
        let text = p.text(pos2(cap.left() - 6.0, mid), Align2::RIGHT_CENTER, word, font.clone(), c.fg);
        let hit = text.union(cap).expand(4.0);
        let resp = ui.interact(hit, ui.id().with(("strip", key)), Sense::click());
        if resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if live && resp.clicked() {
            pressed = Some(key);
        }
        right = hit.left() - 10.0;
    }
    if let Some(text) = app.strip_text() {
        let left = r.left() + 12.0;
        let mut job = egui::text::LayoutJob::simple_singleline(text, font, c.wait);
        job.wrap = egui::text::TextWrapping {
            max_width: (right - left).max(40.0),
            max_rows: 1,
            break_anywhere: true,
            overflow_character: Some('…'),
        };
        let g = ui.fonts_mut(|f| f.layout_job(job));
        p.galley(pos2(left, mid - g.size().y / 2.0), g, c.wait);
    }
    if let Some(key) = pressed {
        app.strip_key(key);
    }
}

/// The line of plain words under the title bar (#107): one row cut short
/// with an ellipsis, the whole of it when the pointer rests on it.
fn where_bar(ui: &mut Ui, r: Rect, text: String) {
    let c = ito_theme::colors();
    let p = ui.painter().clone();
    p.rect_filled(r, 0.0, c.bg);
    p.line_segment([r.left_bottom() - vec2(0.0, 0.5), r.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
    let font = FontId::proportional(12.0);
    let left = r.left() + 12.0;
    let mut job = egui::text::LayoutJob::simple_singleline(text.clone(), font, c.dim);
    job.wrap = egui::text::TextWrapping {
        max_width: (r.right() - left - 12.0 - EDGE).max(40.0),
        max_rows: 1,
        break_anywhere: true,
        overflow_character: Some('…'),
    };
    let g = ui.fonts_mut(|f| f.layout_job(job));
    p.galley(pos2(left, r.center().y - g.size().y / 2.0), g, c.dim);
    ui.interact(r, ui.id().with("where"), Sense::hover()).on_hover_text(text);
}

/// The narrowest a column can be dragged to.
const COL_MIN: f32 = 140.0;
/// The graph's least width when the others are dragged wide.
const GRAPH_MIN: f32 = 240.0;
/// Below this width the side column goes and the detail goes under the
/// graph.
const STACK: f32 = 900.0;
/// The detail's height under the graph.
const STACKED_DETAIL_H: f32 = 330.0;

/// Where the graph view's columns go, and the lines between them.
#[derive(Debug, PartialEq)]
struct Columns {
    side: Option<Rect>,
    graph: Option<Rect>,
    detail: Rect,
    /// The 1-point lines between them.
    lines: Vec<Rect>,
    /// The panes side by side as ito-layout splits them, with the ratios kept
    /// to the least widths; the dividers to drag are over it.
    split: Option<Node<Col>>,
}

/// A column of the graph view, the leaf of its split.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Col {
    Side,
    Graph,
    Detail,
}

/// The gap ito-layout leaves between two panes: the 1-point line.
const GAP: f32 = 1.0;
/// How far from a line the pointer takes it.
const GRAB: f32 = 8.0;

fn split(ratio: f32, first: Node<Col>, second: Node<Col>) -> Node<Col> {
    Node::Split { dir: Dir::Right, ratio, first: Box::new(first), second: Box::new(second) }
}

/// A split's ratio kept so the first pane is at least `least` wide and the
/// second at least `least2`, of the `room` the two share (the gap is theirs).
fn keep(ratio: f32, room: f32, least: f32, least2: f32) -> f32 {
    let room = (room - GAP).max(1.0);
    let first = (room * ratio).clamp(least, (room - least2).max(least));
    first / room
}

/// The side column, the graph and the detail in `r`: side by side, the
/// graph narrowed beside a wide diff, or one over the other in a narrow
/// window (docs/mock).
fn columns(r: Rect, wide: bool, s: &Sizes) -> Columns {
    let panes = |node: &Node<Col>| -> Vec<(Col, Rect)> {
        node.layout(ito_layout::ui::from_egui(r), GAP).into_iter().map(|(c, r)| (c, ito_layout::ui::to_egui(r))).collect()
    };
    let at = |panes: &[(Col, Rect)], c: Col| panes.iter().find(|(p, _)| *p == c).map(|(_, r)| *r).unwrap_or(r);
    let line = |a: Rect, b: Rect| Rect::from_min_max(pos2(a.right(), r.top()), pos2(b.left(), r.bottom()));
    match (r.width() < STACK, wide) {
        (false, false) => {
            let side = keep(s.side, r.width(), COL_MIN, GRAPH_MIN + COL_MIN + GAP);
            let rest = (r.width() - GAP) * (1.0 - side);
            // The detail keeps its least: the graph's share is what is left of it.
            let graph = 1.0 - keep(1.0 - s.graph, rest, COL_MIN, GRAPH_MIN);
            let node = split(side, Node::Leaf(Col::Side), split(graph, Node::Leaf(Col::Graph), Node::Leaf(Col::Detail)));
            let p = panes(&node);
            let (side, graph, detail) = (at(&p, Col::Side), at(&p, Col::Graph), at(&p, Col::Detail));
            Columns { side: Some(side), graph: Some(graph), detail, lines: vec![line(side, graph), line(graph, detail)], split: Some(node) }
        }
        (false, true) => {
            let graph = keep(s.wide, r.width(), GRAPH_MIN, COL_MIN);
            let node = split(graph, Node::Leaf(Col::Graph), Node::Leaf(Col::Detail));
            let p = panes(&node);
            let (graph, detail) = (at(&p, Col::Graph), at(&p, Col::Detail));
            Columns { side: None, graph: Some(graph), detail, lines: vec![line(graph, detail)], split: Some(node) }
        }
        (true, false) => {
            let h = STACKED_DETAIL_H.min(r.height() * 0.55);
            let detail = Rect::from_min_max(pos2(r.left(), r.bottom() - h), r.max);
            let graph = Rect::from_min_max(r.min, pos2(r.right(), detail.top() - 1.0));
            let line = Rect::from_min_max(graph.left_bottom(), pos2(r.right(), detail.top()));
            Columns { side: None, graph: Some(graph), detail, lines: vec![line], split: None }
        }
        (true, true) => Columns { side: None, graph: None, detail: r, lines: Vec::new(), split: None },
    }
}

/// The graph view: the side column, the graph and the selected commit.
fn graph_view(ui: &mut Ui, app: &mut App, r: Rect, live: bool) {
    let c = ito_theme::colors();
    let cols = columns(r, app.detail.wide, &app.sizes);
    crate::dnd_ui::begin(ui.ctx());
    if let Some(side) = cols.side {
        crate::side_ui::show(ui, app, side, live);
    }
    if let Some(graph) = cols.graph {
        crate::graph_ui::show(ui, app, graph, live);
    }
    crate::detail_ui::show(ui, app, cols.detail, live);
    // The blue frame is where the keys are.
    let here = match app.pane {
        crate::panes::GraphPane::Tree => cols.graph,
        crate::panes::GraphPane::Side => cols.side,
        crate::panes::GraphPane::Detail => Some(cols.detail),
    };
    if let Some(r) = here {
        ui.painter().rect_stroke(r.shrink(1.0), 0.0, egui::Stroke::new(1.5, c.blue), egui::StrokeKind::Inside);
    }
    crate::dnd_ui::finish(ui, app, live);
    for line in &cols.lines {
        ui.painter().rect_filled(*line, 0.0, c.border);
    }
    // The dividers (ito-layout) drag the columns, the side and the detail
    // keeping the room the graph needs; they come after the columns, so
    // they take the pointer first.
    let Some(node) = cols.split.as_ref().filter(|_| live) else { return };
    let look = ito_layout::ui::Look { gap: GAP, grab: GRAB, line: accent() };
    match ito_layout::ui::dividers(ui, ui.id().with("columns"), node, ito_layout::ui::from_egui(r), look) {
        Some(Moved::Dragging(n)) => app.sizes.set_panes(&n, cols.side.is_some()),
        // A double-click has no release to save it on.
        Some(Moved::Halved(n)) => {
            app.sizes.set_panes(&n, cols.side.is_some());
            app.sizes.save();
        }
        Some(Moved::Released) => app.sizes.save(),
        None => {}
    }
}

fn title_bar(ui: &mut Ui, app: &App, r: Rect, out: &mut Out) {
    let ctx = ui.ctx().clone();
    let c = ito_theme::colors();
    // The whole bar drags the window; what is drawn over it is registered
    // later, so it takes the pointer first.
    let drag = ui.interact(r, ui.id().with("title-drag"), Sense::click_and_drag());
    if drag.drag_started() {
        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    }
    if drag.double_clicked() {
        let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
    }
    let p = ui.painter().clone();
    p.rect_filled(r, 0.0, c.side);
    // The bar's foot; the tab in force covers its part of it.
    p.line_segment([r.left_bottom() - vec2(0.0, 0.5), r.right_bottom() - vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
    let mid = r.center().y;
    let mut x = r.left() + 12.0;
    // The logo, then the name (the word only: never 道).
    let logo = Rect::from_center_size(pos2(x + 9.0, mid), vec2(18.0, 18.0));
    if let Some(tex) = logo_texture(&ctx) {
        p.image(tex.id(), logo, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
    }
    x = logo.right() + 10.0;
    let name = p.text(pos2(x, mid), Align2::LEFT_CENTER, "michi", FontId::proportional(14.0), c.fg);
    x = name.right() + 16.0;
    // The view tabs, each with its digit, hanging from the bar's foot like
    // folder tabs; the one in force opens into the view. Changes counts the
    // files not committed; Conflicts, there only while something clashed,
    // the places still to solve.
    for v in View::ALL {
        let on = app.is_on(v);
        if v == View::Conflict && !on && app.conflicts.all.is_empty() {
            continue;
        }
        let digit = ui.fonts_mut(|f| f.layout_no_wrap(v.digit().to_string(), FontId::monospace(11.0), c.dim));
        let text = ui.fonts_mut(|f| f.layout_no_wrap(app.tr(v.label()).to_owned(), FontId::proportional(13.0), c.fg));
        let n = if v == View::Changes { app.changes_count() } else { 0 };
        let count = if v == View::Conflict {
            let left = app.conflicts.left();
            let text = if left > 0 { format!("● {left}") } else { "● ✓".to_owned() };
            Some(ui.fonts_mut(|f| f.layout_no_wrap(text, FontId::proportional(11.5), c.err)))
        } else {
            (n > 0).then(|| ui.fonts_mut(|f| f.layout_no_wrap(n.to_string(), FontId::monospace(10.5), c.dim)))
        };
        let w = 12.0 + digit.size().x + 6.0 + text.size().x + count.as_ref().map_or(0.0, |g| 6.0 + g.size().x) + 12.0;
        let tab = Rect::from_min_max(pos2(x, r.bottom() - 30.0), pos2(x + w, r.bottom()));
        let resp = ui.interact(tab, ui.id().with(("tab", v.digit())), Sense::CLICK);
        let round = CornerRadius { nw: 6, ne: 6, sw: 0, se: 0 };
        if on {
            p.rect_filled(tab, round, c.bg);
            p.rect_stroke(tab, round, Stroke::new(1.0, c.border), StrokeKind::Inside);
            p.rect_filled(
                Rect::from_min_max(pos2(tab.left() + 1.0, tab.bottom() - 1.5), pos2(tab.right() - 1.0, tab.bottom() + 0.5)),
                0.0,
                c.bg,
            );
        }
        let (dc, tc) = match (on, resp.hovered()) {
            (true, _) => (accent(), c.fg),
            (false, true) => (c.dim, c.fg),
            (false, false) => (c.dim, c.dim),
        };
        let ty = tab.center().y;
        p.galley(pos2(tab.left() + 12.0, ty - digit.size().y / 2.0), digit.clone(), dc);
        let tx = tab.left() + 12.0 + digit.size().x + 6.0;
        let tw = text.size().x;
        p.galley(pos2(tx, ty - text.size().y / 2.0), text, tc);
        if let Some(g) = count {
            let ink = if v == View::Conflict { c.err } else { c.dim };
            p.galley(pos2(tx + tw + 6.0, ty - g.size().y / 2.0), g, ink);
        }
        if resp.clicked() {
            out.view = Some(v);
        }
        resp.clone().on_hover_text(crate::keymap::tip(app.tr(v.label()), Some(&v.digit().to_string())));
        if resp.hovered() {
            ctx.set_cursor_icon(CursorIcon::PointingHand);
        }
        x = tab.right() + 2.0;
    }
    let left_end = x;
    // The window buttons on the right.
    let mut right = r.right();
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    for (k, kind) in [Caption::Close, Caption::Maximize, Caption::Minimize].into_iter().enumerate() {
        let b = Rect::from_min_max(pos2(right - 44.0, r.top()), pos2(right, r.bottom() - 1.0));
        let resp = ui.interact(b, ui.id().with(("caption", k)), Sense::CLICK);
        if resp.hovered() {
            let fill = if kind == Caption::Close { Color32::from_rgb(0xC4, 0x2B, 0x1C) } else { c.panel };
            p.rect_filled(b, 0.0, fill);
        }
        let ink = match (resp.hovered(), kind) {
            (true, Caption::Close) => Color32::WHITE,
            (true, _) => c.fg,
            _ => c.dim,
        };
        caption_glyph(&p, b.center(), kind, ink, maximized);
        if resp.clicked() {
            match kind {
                Caption::Close => ctx.send_viewport_cmd(ViewportCommand::Close),
                Caption::Minimize => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
                Caption::Maximize => ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized)),
            }
        }
        right = b.left();
    }
    right -= 8.0;
    // The palette field and the bell beside it, in the middle of the window.
    // As tsumugi sizes it: up to 420 wide, as wide as fits either side of the
    // middle, then only the magnifier-sized stub.
    const BELL: f32 = 28.0;
    const GAP: f32 = 6.0;
    let middle = r.center().x;
    let half = (middle - left_end - 12.0).min(right - 90.0 - middle);
    let box_w = 420.0f32.min(2.0 * half - BELL - GAP);
    let w = if box_w < 160.0 { 30.0 } else { box_w };
    let group = w + GAP + BELL;
    let group_left = (middle - group / 2.0).max(left_end + 12.0);
    let group_right = group_left + group;
    let mut centre_end = left_end;
    if group_right <= right {
        let field = Rect::from_min_max(pos2(group_left, mid - 13.0), pos2(group_left + w, mid + 13.0));
        let resp = ui.interact(field, ui.id().with("palette"), Sense::CLICK);
        p.rect_filled(field, CornerRadius::same(5), if resp.hovered() { c.panel } else { c.bg });
        p.rect_stroke(field, CornerRadius::same(5), Stroke::new(1.0, c.border), StrokeKind::Inside);
        let k = if w >= 160.0 {
            kbd(ui, &p, field.right_center() - vec2(6.0, 0.0), "Ctrl+Shift+P")
        } else {
            Rect::from_min_max(field.right_center(), field.right_center())
        };
        let hint = ui.fonts_mut(|f| f.layout_no_wrap(app.tr("palette.hint").to_owned(), FontId::proportional(12.0), c.dim));
        if field.left() + 10.0 + hint.size().x < k.left() - 6.0 {
            p.galley(pos2(field.left() + 10.0, mid - hint.size().y / 2.0), hint, c.dim);
        }
        if resp.clicked() {
            out.palette = true;
        }
        let resp =
            resp.on_hover_text(crate::keymap::tip(app.tr("palette.hint"), app.prefs.keymap.hint(crate::keymap::Act::Palette).as_deref()));
        if resp.hovered() {
            ctx.set_cursor_icon(CursorIcon::Text);
        }
        let bell = Rect::from_min_size(pos2(field.right() + 6.0, mid - 12.0), vec2(BELL, 24.0));
        if bell_button(ui, app, bell) {
            out.bell = true;
        }
        centre_end = bell.right();
    }
    // The settings, as a word, and the repository chip at the far right.
    let word = ui.fonts_mut(|f| f.layout_no_wrap(app.tr("view.settings").to_owned(), FontId::proportional(12.5), c.fg));
    let chip_w = chip(ui, app, &p, centre_end + 12.0 + word.size().x + 24.0, right, mid);
    let chip_x = if chip_w > 0.0 { right - chip_w - 8.0 } else { right };
    let gear = Rect::from_min_max(pos2(chip_x - word.size().x - 16.0, mid - 12.0), pos2(chip_x, mid + 12.0));
    let resp = ui.interact(gear, ui.id().with("gear"), Sense::CLICK);
    let on = app.mode == Mode::Settings;
    if resp.hovered() || on {
        p.rect_filled(gear, CornerRadius::same(5), c.panel);
    }
    p.galley(gear.center() - word.size() / 2.0, word, if on { accent() } else { c.fg });
    if resp.clicked() {
        out.settings = true;
    }
    resp.on_hover_text(crate::keymap::tip(app.tr("view.settings"), app.prefs.keymap.hint(crate::keymap::Act::Settings).as_deref()));
}

/// The bell: the count of notices not read, gold. True when it was clicked.
fn bell_button(ui: &mut Ui, app: &App, rect: Rect) -> bool {
    let c = ito_theme::colors();
    let unread = app.notices.iter().filter(|n| !n.1).count();
    let resp = ui.interact(rect, ui.id().with("bell"), Sense::CLICK);
    let p = ui.painter().clone();
    if resp.hovered() || app.bell_open {
        p.rect_filled(rect, CornerRadius::same(6), c.panel);
    }
    let at = rect.center() + vec2(0.0, -1.0);
    let ink = if unread == 0 { c.dim } else { c.fg };
    let stroke = Stroke::new(1.4, ink);
    let cup = [(-5.5, 4.0), (-4.5, -1.0), (-3.0, -4.5), (0.0, -5.5), (3.0, -4.5), (4.5, -1.0), (5.5, 4.0)];
    p.add(egui::Shape::line(cup.iter().map(|(x, y)| at + vec2(*x, *y)).collect(), stroke));
    p.line_segment([at + vec2(-6.5, 4.0), at + vec2(6.5, 4.0)], stroke);
    p.circle_filled(at + vec2(0.0, 6.2), 1.4, ink);
    if unread > 0 {
        let badge = rect.right_top() + vec2(-5.0, 5.0);
        p.circle_filled(badge, 7.0, Color32::from_rgb(0xE0, 0xA8, 0x30));
        let text = if unread > 9 { "9+".to_owned() } else { unread.to_string() };
        p.text(badge, Align2::CENTER_CENTER, text, FontId::proportional(10.0), Color32::BLACK);
    }
    resp.clone().on_hover_text(app.tr("bell.hover").replace("{}", &unread.to_string()));
    resp.clicked()
}

/// The list the bell opens, under it: newest first. True when it should
/// close (Esc, or a click outside it).
pub fn bell_list(ctx: &egui::Context, app: &App) -> bool {
    let c = ito_theme::colors();
    let r = ctx.content_rect();
    let title = Rect::from_min_size(r.min, vec2(r.width(), TITLE_H));
    let at = pos2((title.center().x + 100.0).min(r.right() - 372.0).max(r.left() + 8.0), title.bottom() + 4.0);
    let mut inside = false;
    let resp = egui::Area::new(egui::Id::new("bell-list")).order(egui::Order::Foreground).fixed_pos(at).show(ctx, |ui| {
        egui::Frame::NONE
            .fill(c.raised())
            .stroke(Stroke::new(1.0, c.border_strong()))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(10.0)
            .show(ui, |ui| {
                ui.set_width(340.0);
                if app.notices.is_empty() {
                    ui.label(egui::RichText::new(app.tr("bell.empty")).color(c.dim));
                    return;
                }
                egui::ScrollArea::vertical().max_height(360.0).auto_shrink([false, true]).show(ui, |ui| {
                    for (text, read) in app.notices.iter().rev() {
                        ui.label(egui::RichText::new(text).color(if *read { c.dim } else { c.fg }).size(12.5));
                        ui.separator();
                    }
                });
            });
    });
    inside |= resp.response.contains_pointer();
    let click_out =
        ctx.input(|i| i.pointer.any_pressed()) && !inside && !ctx.input(|i| i.pointer.hover_pos().is_some_and(|p| p.y < title.bottom()));
    click_out || ctx.input(|i| i.key_pressed(egui::Key::Escape))
}

/// The repository chip, right-aligned to `right`: the name, then the branch
/// and the count of worktrees, and what the worktree is in the middle of in
/// red. What does not fit before `left` goes, the branch first.
fn chip(ui: &Ui, app: &App, p: &egui::Painter, left: f32, right: f32, mid: f32) -> f32 {
    let c = ito_theme::colors();
    let font = FontId::proportional(12.0);
    let doing = match &app.repo {
        Repo::Ready(s) => app.doing_in(&s.top).map(|d| match d {
            Doing::Merge => "chip.merging",
            Doing::Rebase => "chip.rebasing",
            Doing::CherryPick => "chip.cherry",
            Doing::Revert => "chip.reverting",
            Doing::Bisect => "chip.bisecting",
        }),
        _ => None,
    };
    let doing_g = doing.map(|key| ui.fonts_mut(|f| f.layout_no_wrap(format!(" · {}", app.tr(key)), font.clone(), c.err)));
    let doing_w = doing_g.as_ref().map_or(0.0, |g| g.size().x);
    // The branch mark is drawn, not typed: no face every system has draws ⎇.
    let (name, mark, rest, rest_color) = match &app.repo {
        Repo::Loading => (String::new(), false, "…".to_owned(), c.dim),
        Repo::Failed(e) => (String::new(), false, format!("✗ {}", app.tr(failed_word(e))), c.err),
        Repo::Ready(s) => {
            let (mark, head) = match &s.head {
                Head::Branch(b) => (true, b.clone()),
                Head::Detached(h) => (true, h.clone()),
                Head::Bare => (false, "bare".to_owned()),
            };
            (s.name.clone(), mark, format!("{head} · worktree {}", s.worktrees), c.dim)
        }
    };
    let name_g = ui.fonts_mut(|f| f.layout_no_wrap(name.clone(), font.clone(), accent()));
    let rest_g = ui.fonts_mut(|f| f.layout_no_wrap(rest, font, rest_color));
    let gap = if name.is_empty() { 0.0 } else { 5.0 };
    let mark_w = if mark { BRANCH_MARK + 4.0 } else { 0.0 };
    let full = name_g.size().x + gap + mark_w + rest_g.size().x + doing_w;
    let (inner, show_rest) = if right - 18.0 - full >= left {
        (full, true)
    } else if !name.is_empty() && right - 18.0 - name_g.size().x >= left {
        (name_g.size().x, false)
    } else {
        return 0.0;
    };
    let rect = Rect::from_min_max(pos2(right - inner - 18.0, mid - 11.0), pos2(right, mid + 11.0));
    p.rect_stroke(rect, CornerRadius::same(11), Stroke::new(1.0, c.border), StrokeKind::Inside);
    let mut x = rect.left() + 9.0;
    if !name.is_empty() {
        p.galley(pos2(x, mid - name_g.size().y / 2.0), name_g.clone(), accent());
        x += name_g.size().x + gap;
    }
    if show_rest {
        if mark {
            branch_mark(p, pos2(x + BRANCH_MARK / 2.0, mid), rest_color);
            x += mark_w;
        }
        let w = rest_g.size().x;
        p.galley(pos2(x, mid - rest_g.size().y / 2.0), rest_g, rest_color);
        if let Some(g) = doing_g {
            p.galley(pos2(x + w, mid - g.size().y / 2.0), g, c.err);
        }
    }
    // The folder it is on, for a closer look.
    let resp = ui.interact(rect, ui.id().with("chip"), Sense::hover());
    let top = match &app.repo {
        Repo::Ready(s) => s.top.display().to_string(),
        _ => app.dir.display().to_string(),
    };
    resp.on_hover_text(top);
    rect.width()
}

/// The branch mark's width.
const BRANCH_MARK: f32 = 9.0;

/// A branch drawn as a trunk and a twig, each ending in a dot, centred on
/// `at`.
fn branch_mark(p: &egui::Painter, at: Pos2, ink: Color32) {
    let s = Stroke::new(1.2, ink);
    let (trunk_top, trunk_foot, twig) = (at + vec2(-2.5, -4.0), at + vec2(-2.5, 4.0), at + vec2(3.0, -2.5));
    p.line_segment([trunk_top, trunk_foot], s);
    p.add(egui::epaint::QuadraticBezierShape::from_points_stroke(
        [twig, twig + vec2(0.0, 4.0), at + vec2(-2.5, 2.5)],
        false,
        Color32::TRANSPARENT,
        s,
    ));
    for dot in [trunk_top, trunk_foot, twig] {
        p.circle_filled(dot, 1.5, ink);
    }
}

/// A key drawn as a key cap, right-aligned to `right_center`; its rect.
pub(crate) fn kbd(ui: &Ui, p: &egui::Painter, right_center: Pos2, key: &str) -> Rect {
    let c = ito_theme::colors();
    let g = ui.fonts_mut(|f| f.layout_no_wrap(ito_keys::human_text(key).into_owned(), FontId::monospace(10.5), c.dim));
    let r = Rect::from_min_max(right_center - vec2(g.size().x + 8.0, 8.0), right_center + vec2(0.0, 8.0));
    p.rect_filled(r, CornerRadius::same(3), c.panel);
    p.rect_stroke(r, CornerRadius::same(3), Stroke::new(1.0, c.border), StrokeKind::Inside);
    p.line_segment([r.left_bottom() + vec2(1.0, -0.5), r.right_bottom() + vec2(-1.0, -0.5)], Stroke::new(1.0, c.border));
    p.galley(pos2(r.left() + 4.0, r.center().y - g.size().y / 2.0), g, c.dim);
    r
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Caption {
    Minimize,
    Maximize,
    Close,
}

fn caption_glyph(p: &egui::Painter, at: Pos2, kind: Caption, ink: Color32, maximized: bool) {
    let s = Stroke::new(1.0, ink);
    match kind {
        Caption::Minimize => {
            p.line_segment([at - vec2(5.0, 0.0), at + vec2(5.0, 0.0)], s);
        }
        Caption::Maximize if maximized => {
            p.rect_stroke(Rect::from_center_size(at + vec2(-1.0, 1.0), vec2(8.0, 8.0)), 0.0, s, StrokeKind::Middle);
            p.line_segment([at + vec2(-2.0, -4.5), at + vec2(4.5, -4.5)], s);
            p.line_segment([at + vec2(4.5, -4.5), at + vec2(4.5, 2.0)], s);
        }
        Caption::Maximize => {
            p.rect_stroke(Rect::from_center_size(at, vec2(9.0, 9.0)), 0.0, s, StrokeKind::Middle);
        }
        Caption::Close => {
            p.line_segment([at - vec2(4.5, 4.5), at + vec2(4.5, 4.5)], s);
            p.line_segment([at + vec2(-4.5, 4.5), at + vec2(4.5, -4.5)], s);
        }
    }
}

/// The logo as a texture, made once from the icon's own drawing.
fn logo_texture(ctx: &egui::Context) -> Option<egui::TextureHandle> {
    let id = egui::Id::new("michi-logo");
    if let Some(t) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Some(t);
    }
    const PX: u32 = 64;
    let image = egui::ColorImage::from_rgba_unmultiplied([PX as usize; 2], &crate::icon::pixels(PX));
    let tex = ctx.load_texture("michi-logo", image, egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    Some(tex)
}

/// A view that has nothing to show yet.
fn empty_view(ui: &mut Ui, app: &App, r: Rect) {
    let c = ito_theme::colors();
    let p = ui.painter();
    let mid = r.center();
    p.text(mid - vec2(0.0, 14.0), Align2::CENTER_CENTER, app.tr(app.view.label()), FontId::proportional(20.0), c.fg);
    p.text(mid + vec2(0.0, 12.0), Align2::CENTER_CENTER, app.tr(app.view.empty()), FontId::proportional(13.0), c.dim);
    p.text(mid + vec2(0.0, 34.0), Align2::CENTER_CENTER, app.tr("empty.soon"), FontId::proportional(12.0), c.dim.gamma_multiply(0.7));
}

/// The question for a job waiting for a yes to what would lose work or get
/// in a Claude's way, drawn by `ito-confirm` over the window: what it would
/// do, `y` and `Esc` (a merge's revert: a digit for each parent, not `y`).
/// The keys are `App::band_key`'s; a click on a button is the same key.
fn band(ui: &mut Ui, app: &mut App, live: bool) {
    let Some(b) = &app.band else { return };
    let c = ito_theme::colors();
    let style = ito_confirm::Style {
        panel: c.panel,
        border: c.border,
        accent: Some(if b.danger() { c.err } else { c.wait }),
        title: c.fg,
        text: c.fg,
        dim: c.dim,
        warn: if b.danger() { c.err } else { c.wait },
        button_bg: c.border,
        button_text: c.fg,
    };
    let mut buttons = Vec::new();
    let mut keys = Vec::new();
    let mut hint = Vec::new();
    // The strongest step has no button to click: the name is typed.
    let mut notes = b.notes.clone();
    if b.typing() {
        notes.push(format!("> {}_", b.typed));
        hint.push(format!("Enter {}", app.tr("changes.yes")));
    } else if b.parents == 0 {
        buttons.push(app.tr("changes.yes").to_owned());
        keys.push('y');
        hint.push(format!("Ctrl+Enter {}", app.tr("changes.yes")));
    }
    for n in 1..=u32::from(b.parents) {
        let d = char::from_digit(n, 10).unwrap_or('1');
        let word = app.tr(b.parent_key(n)).replace("{}", &n.to_string());
        hint.push(format!("{d} {word}"));
        buttons.push(word);
        keys.push(d);
    }
    buttons.push(app.tr("changes.no").to_owned());
    keys.push('n');
    hint.push(format!("Esc {}", app.tr("changes.no")));
    let body: Vec<String> = std::iter::once(b.text.clone()).chain(b.plan.iter().cloned()).collect();
    let q = ito_confirm::Question {
        title: app.tr(match b.strength {
            Strength::Easy => "band.title",
            Strength::Loud => "band.title_loud",
            Strength::Typed => "band.title_typed",
        }),
        rows: &b.rows,
        body: &body,
        notes: &notes,
        buttons: &buttons,
        hint: &hint.join(" · "),
    };
    let clicked = ito_confirm::show(ui.ctx(), &q, &style);
    if let (true, Some(i)) = (live, clicked) {
        if app.band.as_ref().is_some_and(|b| b.typing()) {
            // The only button left is No; `n` would be a letter of the name.
            app.band = None;
        } else {
            app.band_key(keys[i]);
        }
    }
}

/// What the keybar says of the network at its right: the job that runs and
/// how far it has got, else how long ago the repository fetched.
fn net_status(ui: &Ui, app: &App, p: &egui::Painter, right: f32, mid: f32) -> f32 {
    let c = ito_theme::colors();
    let font = FontId::proportional(12.0);
    if let Some(n) = &app.net {
        let hint = p.text(pos2(right, mid), Align2::RIGHT_CENTER, app.tr("net.stop_hint"), font.clone(), c.dim);
        let mut s = n.kind.word().to_owned();
        if let Some(pr) = &n.progress {
            s.push_str(" · ");
            s.push_str(&pr.step);
            if let Some(pc) = pr.percent {
                s.push_str(&format!(" {pc}%"));
            }
        }
        if n.stopping() {
            s.push_str(" …");
        }
        let t = p.text(pos2(hint.left() - 10.0, mid), Align2::RIGHT_CENTER, s, font, c.wait);
        let icon = match n.kind {
            Kind::Fetch => Icon::Fetch,
            Kind::Pull => Icon::Pull,
            Kind::Push => Icon::Push,
            Kind::Sync => Icon::Sync,
        };
        paint_icon(p, pos2(t.left() - 9.0, mid), icon, c.wait);
        return t.left() - 18.0 - 14.0;
    }
    let s = match (app.last_fetch, app.fetch_failed) {
        (Some(at), failed) => {
            let key = if failed { "net.last_failed" } else { "net.last" };
            app.tr(key).replace("{}", &work::ago(app, at, SystemTime::now()))
        }
        (None, true) => app.tr("net.never_failed").to_owned(),
        (None, false) => return right,
    };
    let colour = if app.fetch_failed { c.wait } else { c.dim };
    let t = p.text(pos2(right, mid), Align2::RIGHT_CENTER, s, font, colour);
    paint_icon(p, pos2(t.left() - 9.0, mid), Icon::Fetch, colour);
    ui.ctx().request_repaint_after(Duration::from_secs(30));
    t.left() - 18.0 - 14.0
}

/// The keybar: the keys of the view at the left, the best one lit; git's
/// state, the network's and the clock at the right.
fn key_bar(ui: &mut Ui, app: &App, r: Rect, out: &mut Out) {
    let c = ito_theme::colors();
    let p = ui.painter().clone();
    p.rect_filled(r, 0.0, c.side);
    p.line_segment([r.left_top() + vec2(0.0, 0.5), r.right_top() + vec2(0.0, 0.5)], Stroke::new(1.0, c.border));
    let mid = r.center().y;
    // The right side first, from the right: the clock, then git's state, so
    // the keys know where to stop.
    let mut right = r.right() - 10.0;
    let clock = app.prefs.common.clock_or(None);
    let now = chrono::Local::now().naive_local();
    if clock.show {
        let t = p.text(pos2(right, mid), Align2::RIGHT_CENTER, clock.text(&now, &app.lang), FontId::proportional(12.0), c.dim);
        right = t.left() - 14.0;
        ui.ctx().request_repaint_after(Clock::next_minute(&now));
    }
    let warn = match (&app.repo, app.old_git()) {
        (Repo::Failed(git::Error::NoGit), _) => Some(("⚠ git".to_owned(), c.err)),
        (_, Some(v)) => Some((format!("⚠ git {v}"), c.wait)),
        _ => None,
    };
    if let Some((text, color)) = warn {
        let t = p.text(pos2(right, mid), Align2::RIGHT_CENTER, text, FontId::proportional(12.0), color);
        right = t.left() - 14.0;
    }
    right = net_status(ui, app, &p, right, mid);
    if app.mode == Mode::Main && matches!(app.repo, Repo::Ready(_)) {
        let (left, clicked) = pill(ui, "calm", app.tr("calm.button"), app.tr("calm.tip"), right, mid, c.dim);
        out.calm = clicked;
        right = left;
        if let Some(s) = app.suggestion() {
            let (left, clicked) = pill(ui, "next", app.tr(s.key()), app.tr("suggest.tip"), right, mid, accent());
            out.next = clicked;
            right = left;
        }
    }
    let keys: Vec<(&str, &'static str)> = match app.mode {
        Mode::Settings => vec![("Esc", "key.back"), ("Ctrl+Tab", "key.page")],
        Mode::Help => vec![("? Esc", "key.back")],
        Mode::Main => app.listed(app.keys()).into_iter().take(KEYBAR_KEYS).chain([("F1", "key.help")]).collect(),
    };
    let mut x = r.left() + 10.0;
    let mut clicked = None;
    for (k, (key, word)) in keys.iter().enumerate() {
        let kc = if k == 0 { accent() } else { c.fg };
        let kg = ui.fonts_mut(|f| f.layout_no_wrap(ito_keys::human_text(key).into_owned(), FontId::monospace(11.5), kc));
        let wg = ui.fonts_mut(|f| f.layout_no_wrap(app.tr(word).to_owned(), FontId::proportional(12.0), c.dim));
        if x + kg.size().x + 4.0 + wg.size().x > right {
            break;
        }
        // An entry that is one key is a button: a click does what the key does.
        let width = kg.size().x + 4.0 + wg.size().x;
        let hit = Rect::from_min_size(pos2(x - 4.0, r.top() + 1.0), vec2(width + 8.0, r.height() - 1.0));
        let hot = bar_key(key).map(|b| (b, ui.interact(hit, ui.id().with(("keybar", k)), Sense::CLICK)));
        let hovered = hot.as_ref().is_some_and(|(_, resp)| resp.hovered());
        if hovered {
            p.rect_filled(hit, CornerRadius::same(4), c.panel);
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if let Some((b, resp)) = hot {
            if resp.clicked() {
                clicked = Some(b);
            }
        }
        p.galley(pos2(x, mid - kg.size().y / 2.0), kg.clone(), kc);
        x += kg.size().x + 4.0;
        p.galley(pos2(x, mid - wg.size().y / 2.0), wg.clone(), if hovered { c.fg } else { c.dim });
        x += wg.size().x + 12.0;
    }
    out.key = clicked;
}

/// The list of every key: the global ones, then the view's, two to a line
/// as the mock has them (one when the window is narrow), each its word at
/// the left and its keys at the right.
fn help(ui: &mut Ui, app: &mut App, r: Rect) {
    const LINE: f32 = 20.0;
    // Two columns when the window is wide enough for them, one when it is not.
    const COLUMN: f32 = 400.0;
    let c = ito_theme::colors();
    let p = ui.painter().clone();
    let w = r.width().min(1000.0);
    let cols = ito_help::columns_for(w - 40.0, COLUMN, 2);
    let columns = ito_help::deal(&app.help_groups(), cols);
    let tall = columns.iter().map(Vec::len).max().unwrap_or(0);
    let plan = ito_help::Plan { columns, scale: 1.0, scrolls: false };
    let need = 50.0 + tall as f32 * LINE + 36.0;
    let card = Rect::from_center_size(r.center(), vec2(w, need.min(r.height() - 24.0).max(120.0)));
    p.rect_filled(card, CornerRadius::same(10), c.raised());
    p.rect_stroke(card, CornerRadius::same(10), Stroke::new(1.0, c.border_strong()), StrokeKind::Inside);
    p.text(pos2(card.left() + 20.0, card.top() + 18.0), Align2::LEFT_TOP, app.tr("help.title"), FontId::proportional(16.0), c.fg);
    let body = Rect::from_min_max(pos2(card.left() + 20.0, card.top() + 50.0), pos2(card.right() - 20.0, card.bottom() - 36.0));
    // The wheel scrolls it by lines, as the arrow keys do.
    let wheel = ui.input(|i| i.smooth_scroll_delta.y);
    if wheel != 0.0 {
        if let Some(d) = app.help_drawn.get() {
            app.help_scroll = ito_help::scrolled(d.scroll, (-wheel / LINE).round() as i64, d.max);
        }
    }
    let style = ito_help::Style {
        font: FontId::monospace(12.0),
        row_h: LINE,
        head: accent(),
        keys: c.fg,
        desc: c.dim,
        warn: c.err,
        dim: c.dim,
        bar: c.border_strong(),
    };
    let drawn = ito_help::draw_plan(&p, body, &plan, app.help_scroll, &style);
    app.help_drawn.set(Some(drawn));
    p.text(pos2(card.left() + 20.0, card.bottom() - 22.0), Align2::LEFT_CENTER, app.tr("help.close"), FontId::proportional(11.5), c.dim);
}

/// The message at the top right of `r`, over whatever is drawn there, while
/// it lasts (as in kura).
pub fn toast(ui: &mut Ui, app: &App, r: Rect) {
    let (Some((text, _)), Some(left)) = (&app.toast, app.toast_left()) else { return };
    ui.ctx().request_repaint_after(left);
    let c = ito_theme::colors();
    let p = ui.painter().clone();
    let max_w = (r.width() * 0.5).clamp(240.0, (r.width() - 24.0).max(240.0));
    let galley = ui.fonts_mut(|f| f.layout(text.clone(), FontId::proportional(12.5), c.fg, max_w - 28.0));
    let size = galley.size() + vec2(28.0, 14.0);
    let box_ = Rect::from_min_size(pos2(r.right() - size.x - 12.0, r.top() + 8.0), size);
    p.rect_filled(box_, CornerRadius::same(8), c.raised());
    p.rect_stroke(box_, CornerRadius::same(8), Stroke::new(1.0, c.border_strong()), StrokeKind::Inside);
    p.galley(box_.min + vec2(14.0, 7.0), galley, c.fg);
}

/// A window without the system's frame is resized by its edges: a strip
/// along each, and the corners, set the pointer and start the system's
/// resizing on the press. Called last in a frame, so its pointer is the one
/// shown. Not while maximized.
pub fn resize_edges(ctx: &egui::Context) {
    use egui::{CursorIcon as C, ResizeDirection as D};
    if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
        return;
    }
    let (pos, pressed) = ctx.input(|i| (i.pointer.latest_pos(), i.pointer.primary_pressed()));
    let Some(p) = pos else { return };
    let Some(dir) = edge_at(ctx.content_rect(), p) else { return };
    ctx.set_cursor_icon(match dir {
        D::North | D::South => C::ResizeVertical,
        D::East | D::West => C::ResizeHorizontal,
        D::NorthWest | D::SouthEast => C::ResizeNwSe,
        D::NorthEast | D::SouthWest => C::ResizeNeSw,
    });
    if pressed {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
    }
}

/// Which edge or corner of `r` the point `p` is on, if any. The top edge is
/// thinner: the band under it is the title bar to drag by.
fn edge_at(r: Rect, p: Pos2) -> Option<egui::ResizeDirection> {
    use egui::ResizeDirection as D;
    const CORNER: f32 = 12.0;
    if !r.contains(p) {
        return None;
    }
    let (left, right) = (p.x - r.left() < EDGE, r.right() - p.x < EDGE);
    let (top, bottom) = (p.y - r.top() < EDGE / 2.0, r.bottom() - p.y < EDGE);
    let (near_left, near_right) = (p.x - r.left() < CORNER, r.right() - p.x < CORNER);
    let (near_top, near_bottom) = (p.y - r.top() < CORNER, r.bottom() - p.y < CORNER);
    let dir = match () {
        _ if (top && near_left) || (left && near_top) => D::NorthWest,
        _ if (top && near_right) || (right && near_top) => D::NorthEast,
        _ if (bottom && near_left) || (left && near_bottom) => D::SouthWest,
        _ if (bottom && near_right) || (right && near_bottom) => D::SouthEast,
        _ if top => D::North,
        _ if bottom => D::South,
        _ if left => D::West,
        _ if right => D::East,
        _ => return None,
    };
    Some(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::ResizeDirection as D;

    #[test]
    fn edges_and_corners() {
        let r = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
        assert_eq!(edge_at(r, pos2(400.0, 300.0)), None);
        assert_eq!(edge_at(r, pos2(400.0, 599.0)), Some(D::South));
        assert_eq!(edge_at(r, pos2(1.0, 300.0)), Some(D::West));
        assert_eq!(edge_at(r, pos2(799.0, 599.0)), Some(D::SouthEast));
        assert_eq!(edge_at(r, pos2(1.0, 1.0)), Some(D::NorthWest));
        // Under the thin top edge is the title bar.
        assert_eq!(edge_at(r, pos2(400.0, 5.0)), None);
        assert_eq!(edge_at(r, pos2(900.0, 5.0)), None);
    }

    #[test]
    fn the_columns_follow_the_width_and_the_wide_diff() {
        let r = |w: f32| Rect::from_min_size(pos2(0.0, 36.0), vec2(w, 600.0));
        let s = Sizes::default();
        let three = columns(r(1280.0), false, &s);
        let (side, graph, detail) = (three.side.unwrap(), three.graph.unwrap(), three.detail);
        assert!((side.width() - 0.16 * 1279.0).abs() < 1.0);
        assert!(graph.width() > detail.width() && detail.width() > side.width());
        assert_eq!((side.right() + 1.0, graph.right() + 1.0, detail.right()), (graph.left(), detail.left(), 1280.0));
        assert_eq!(three.lines.len(), 2);
        let wide = columns(r(1280.0), true, &s);
        assert_eq!(wide.side, None);
        assert!((wide.graph.unwrap().width() - 0.28 * 1279.0).abs() < 1.0);
        assert_eq!(wide.detail.left(), wide.graph.unwrap().right() + 1.0);
        // Dragged too wide, the graph keeps its least and the others theirs.
        let mut big = Sizes::default();
        (big.side, big.graph) = (0.9, 0.05);
        let c = columns(r(1280.0), false, &big);
        assert!(c.graph.unwrap().width() >= GRAPH_MIN - 1.0, "{c:?}");
        assert!(c.detail.width() >= COL_MIN - 1.0 && c.side.unwrap().width() >= COL_MIN - 1.0, "{c:?}");
        big.side = 0.05;
        assert!(columns(r(1280.0), false, &big).side.unwrap().width() >= COL_MIN - 1.0);
        let stacked = columns(r(800.0), false, &s);
        assert_eq!(stacked.side, None);
        assert_eq!(stacked.detail.height(), STACKED_DETAIL_H);
        assert_eq!(stacked.graph.unwrap().bottom() + 1.0, stacked.detail.top());
        let alone = columns(r(800.0), true, &s);
        assert_eq!((alone.graph, alone.detail), (None, r(800.0)));
    }

    #[test]
    fn a_keybar_entry_is_a_button_only_when_it_is_one_key() {
        assert_eq!(bar_key("c"), Some(BarKey::Char('c')));
        assert_eq!(bar_key("?"), Some(BarKey::Char('?')));
        assert_eq!(bar_key("F1"), None);
        assert_eq!(bar_key("Space"), Some(BarKey::Char(' ')));
        assert_eq!(bar_key("Esc"), Some(BarKey::Esc));
        assert_eq!(bar_key("j k"), None);
        assert_eq!(bar_key("Ctrl+Tab"), None);
    }
}
