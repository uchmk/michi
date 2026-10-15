//! The window: the title bar, the view, the keybar and the key list. Drawn
//! with the painter on the rects it lays out itself, so nothing here scrolls
//! (docs/design.md: no scrollbar anywhere). The look follows docs/mock.

use egui::{pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, ViewportCommand};
use ito_common::Clock;
use michi_core::git;
use michi_core::summary::Head;

use crate::app::{failed_word, App, Mode, Repo, View, GLOBAL_KEYS};

/// The accent of the tabs, the chip's name and the best key: main's cyan,
/// the colour of the left lane in the graph.
pub fn accent() -> Color32 {
    ito_theme::colors().run
}

const TITLE_H: f32 = 36.0;
const KEYBAR_H: f32 = 24.0;
/// How many of a view's keys the keybar shows before "? all keys".
const KEYBAR_KEYS: usize = 5;
/// Below this width the palette field is shorter.
const NARROW: f32 = 1100.0;

/// What the frame asks of the app.
pub struct Out {
    pub view: Option<View>,
    pub settings: bool,
    pub palette: bool,
    /// Between the title bar and the keybar: where the settings go.
    pub body: Rect,
}

/// The title bar, the keybar and the view between them; under the settings,
/// the body is left to them.
pub fn frame(ui: &mut Ui, app: &App) -> Out {
    let c = ito_theme::colors();
    let rect = ui.max_rect();
    ui.painter().rect_filled(rect, 0.0, c.bg);
    let title = Rect::from_min_size(rect.min, vec2(rect.width(), TITLE_H));
    let keybar = Rect::from_min_max(pos2(rect.left(), rect.bottom() - KEYBAR_H), rect.max);
    let body = Rect::from_min_max(title.left_bottom(), keybar.right_top());
    let mut out = Out { view: None, settings: false, palette: false, body };
    match app.mode {
        Mode::Help => help(ui, app, body),
        Mode::Main => empty_view(ui, app, body),
        Mode::Settings => {}
    }
    title_bar(ui, app, title, &mut out);
    key_bar(ui, app, keybar);
    out
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
    // folder tabs; the one in force opens into the view.
    for v in View::ALL {
        let on = app.is_on(v);
        let digit = ui.fonts_mut(|f| f.layout_no_wrap(v.digit().to_string(), FontId::monospace(11.0), c.dim));
        let text = ui.fonts_mut(|f| f.layout_no_wrap(app.tr(v.label()).to_owned(), FontId::proportional(13.0), c.fg));
        let w = 12.0 + digit.size().x + 6.0 + text.size().x + 12.0;
        let tab = Rect::from_min_max(pos2(x, r.bottom() - 30.0), pos2(x + w, r.bottom()));
        let resp = ui.interact(tab, ui.id().with(("tab", v.digit())), Sense::click());
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
        p.galley(pos2(tab.left() + 12.0 + digit.size().x + 6.0, ty - text.size().y / 2.0), text, tc);
        if resp.clicked() {
            out.view = Some(v);
        }
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
        let resp = ui.interact(b, ui.id().with(("caption", k)), Sense::click());
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
    right -= 6.0;
    // The comma opens the settings.
    let gear = Rect::from_center_size(pos2(right - 12.0, mid), vec2(24.0, 24.0));
    let resp = ui.interact(gear, ui.id().with("gear"), Sense::click());
    if resp.hovered() || app.mode == Mode::Settings {
        p.rect_filled(gear, CornerRadius::same(5), c.panel);
    }
    p.text(gear.center(), Align2::CENTER_CENTER, ",", FontId::monospace(15.0), if app.mode == Mode::Settings { accent() } else { c.fg });
    if resp.clicked() {
        out.settings = true;
    }
    resp.on_hover_text(app.tr("view.settings"));
    right = gear.left() - 8.0;
    // The palette field.
    let w = if r.width() < NARROW { 150.0 } else { 230.0 };
    if right - w > left_end + 12.0 {
        let field = Rect::from_min_max(pos2(right - w, mid - 12.0), pos2(right, mid + 12.0));
        let resp = ui.interact(field, ui.id().with("palette"), Sense::click());
        p.rect_filled(field, CornerRadius::same(5), if resp.hovered() { c.panel } else { c.bg });
        p.rect_stroke(field, CornerRadius::same(5), Stroke::new(1.0, c.border), StrokeKind::Inside);
        let k = kbd(ui, &p, field.right_center() - vec2(6.0, 0.0), "Ctrl+Shift+P");
        let hint = ui.fonts_mut(|f| f.layout_no_wrap(app.tr("palette.hint").to_owned(), FontId::proportional(12.0), c.dim));
        if field.left() + 10.0 + hint.size().x < k.left() - 6.0 {
            p.galley(pos2(field.left() + 10.0, mid - hint.size().y / 2.0), hint, c.dim);
        }
        if resp.clicked() {
            out.palette = true;
        }
        if resp.hovered() {
            ctx.set_cursor_icon(CursorIcon::Text);
        }
        right = field.left() - 10.0;
    }
    chip(ui, app, &p, left_end + 10.0, right, mid);
}

/// The repository chip, right-aligned to `right`: the name, then the branch
/// and the count of worktrees. What does not fit before `left` goes, the
/// branch first.
fn chip(ui: &Ui, app: &App, p: &egui::Painter, left: f32, right: f32, mid: f32) {
    let c = ito_theme::colors();
    let font = FontId::proportional(12.0);
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
    let full = name_g.size().x + gap + mark_w + rest_g.size().x;
    let (inner, show_rest) = if right - 18.0 - full >= left {
        (full, true)
    } else if !name.is_empty() && right - 18.0 - name_g.size().x >= left {
        (name_g.size().x, false)
    } else {
        return;
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
        p.galley(pos2(x, mid - rest_g.size().y / 2.0), rest_g, rest_color);
    }
    // The folder it is on, for a closer look.
    let resp = ui.interact(rect, ui.id().with("chip"), Sense::hover());
    let top = match &app.repo {
        Repo::Ready(s) => s.top.display().to_string(),
        _ => app.dir.display().to_string(),
    };
    resp.on_hover_text(top);
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
fn kbd(ui: &Ui, p: &egui::Painter, right_center: Pos2, key: &str) -> Rect {
    let c = ito_theme::colors();
    let g = ui.fonts_mut(|f| f.layout_no_wrap(key.to_owned(), FontId::monospace(10.5), c.dim));
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

/// The keybar: the keys of the view at the left, the best one lit; git's
/// state and the clock at the right.
fn key_bar(ui: &mut Ui, app: &App, r: Rect) {
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
    let keys: Vec<(&str, &'static str)> = match app.mode {
        Mode::Settings => vec![("Esc", "key.back"), ("Ctrl+Tab", "key.page")],
        Mode::Help => vec![("? Esc", "key.back")],
        Mode::Main => app.view.keys().iter().take(KEYBAR_KEYS).copied().chain([("?", "key.help")]).collect(),
    };
    let mut x = r.left() + 10.0;
    for (k, (key, word)) in keys.iter().enumerate() {
        let kc = if k == 0 { accent() } else { c.fg };
        let kg = ui.fonts_mut(|f| f.layout_no_wrap((*key).to_owned(), FontId::monospace(11.5), kc));
        let wg = ui.fonts_mut(|f| f.layout_no_wrap(app.tr(word).to_owned(), FontId::proportional(12.0), c.dim));
        if x + kg.size().x + 4.0 + wg.size().x > right {
            break;
        }
        p.galley(pos2(x, mid - kg.size().y / 2.0), kg.clone(), kc);
        x += kg.size().x + 4.0;
        p.galley(pos2(x, mid - wg.size().y / 2.0), wg.clone(), c.dim);
        x += wg.size().x + 12.0;
    }
}

/// The list of every key: the global ones, then the view's.
fn help(ui: &mut Ui, app: &App, r: Rect) {
    let c = ito_theme::colors();
    let p = ui.painter().clone();
    let w = r.width().min(640.0);
    let card = Rect::from_center_size(r.center(), vec2(w, (r.height() - 24.0).clamp(80.0, 460.0)));
    p.rect_filled(card, CornerRadius::same(10), c.raised());
    p.rect_stroke(card, CornerRadius::same(10), Stroke::new(1.0, c.border_strong()), StrokeKind::Inside);
    let mut y = card.top() + 18.0;
    p.text(pos2(card.left() + 20.0, y), Align2::LEFT_TOP, app.tr("help.title"), FontId::proportional(16.0), c.fg);
    y += 32.0;
    let section = |title: &str, rows: &[(&str, &'static str)], y: &mut f32| {
        p.text(pos2(card.left() + 20.0, *y), Align2::LEFT_TOP, title, FontId::proportional(11.5), accent());
        *y += 20.0;
        for (key, word) in rows {
            if *y + 18.0 > card.bottom() - 28.0 {
                return;
            }
            p.text(pos2(card.left() + 20.0, *y), Align2::LEFT_TOP, *key, FontId::monospace(12.0), c.fg);
            p.text(pos2(card.left() + 150.0, *y), Align2::LEFT_TOP, app.tr(word), FontId::proportional(12.5), c.dim);
            *y += 20.0;
        }
        *y += 8.0;
    };
    section(app.tr("help.global"), &GLOBAL_KEYS, &mut y);
    section(app.tr(app.view.label()), app.view.keys(), &mut y);
    p.text(pos2(card.left() + 20.0, card.bottom() - 22.0), Align2::LEFT_CENTER, app.tr("help.close"), FontId::proportional(11.5), c.dim);
}

/// The message at the foot of `r`, over whatever is drawn there, while it
/// lasts.
pub fn toast(ui: &mut Ui, app: &App, r: Rect) {
    let (Some((text, _)), Some(left)) = (&app.toast, app.toast_left()) else { return };
    ui.ctx().request_repaint_after(left);
    let c = ito_theme::colors();
    let p = ui.painter().clone();
    let galley = ui.fonts_mut(|f| f.layout_no_wrap(text.clone(), FontId::proportional(12.5), c.fg));
    let box_ = Rect::from_center_size(pos2(r.center().x, r.bottom() - 24.0), galley.size() + vec2(28.0, 14.0));
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
    const EDGE: f32 = 6.0;
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
}
