//! The Git flow sheet: a flowchart per thing to do, with the commands of its
//! boxes under it and a column for the michi key that does the same. The data
//! is `michi_core::cheat`; this draws it. `1`-`7` or `h` `l` change the sheet,
//! `j` `k` walk the boxes, a click on a command copies it, Esc or a click
//! outside closes it.

use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Id, Pos2, Rect, RichText, Sense, Shape, Stroke};
use michi_core::cheat::{Flow, Kind, Topic};

use crate::app::App;

const NODE_W: f32 = 120.0;
const NODE_H: f32 = 54.0;
const GAP_X: f32 = 18.0;
const GAP_Y: f32 = 34.0;
const GUTTER: f32 = 30.0;
const WIDTH: f32 = 1500.0;

/// Which sheet is open and which box is picked.
#[derive(Clone, Copy, Debug, Default)]
pub struct Cheat {
    pub tab: usize,
    pub sel: Option<u8>,
}

impl Cheat {
    pub fn topic(&self) -> Topic {
        Topic::ALL[self.tab.min(Topic::ALL.len() - 1)]
    }

    fn set_tab(&mut self, tab: usize) {
        if tab < Topic::ALL.len() {
            self.tab = tab;
            self.sel = None;
        }
    }

    fn step_tab(&mut self, by: i32) {
        let n = Topic::ALL.len() as i32;
        self.set_tab((self.tab as i32 + by).rem_euclid(n) as usize);
    }

    /// The next (or previous) box in reading order; the first or last when none is picked.
    fn walk(&mut self, by: i32) {
        let walk = self.topic().flow().walk();
        let at = self.sel.and_then(|s| walk.iter().position(|&i| i == s));
        let next = match at {
            Some(p) => (p as i32 + by).clamp(0, walk.len() as i32 - 1) as usize,
            None if by >= 0 => 0,
            None => walk.len() - 1,
        };
        self.sel = walk.get(next).copied();
    }
}

impl App {
    /// Open the sheet, or close it.
    pub fn toggle_cheat(&mut self) {
        self.cheat = if self.cheat.is_some() { None } else { Some(Cheat::default()) };
        if self.cheat.is_some() {
            self.mode = crate::app::Mode::Main;
        }
    }

    /// A character typed while the sheet is open.
    pub(crate) fn cheat_key(&mut self, c: char) -> bool {
        let Some(st) = &mut self.cheat else { return false };
        match c {
            '1'..='9' => st.set_tab(c as usize - '1' as usize),
            'h' => st.step_tab(-1),
            'l' => st.step_tab(1),
            'j' => st.walk(1),
            'k' => st.walk(-1),
            'q' => self.cheat = None,
            _ => {}
        }
        true
    }

    /// A named key while the sheet is open.
    pub(crate) fn cheat_named(&mut self, name: &str) -> bool {
        let Some(st) = &mut self.cheat else { return false };
        match name {
            "ArrowLeft" | "Shift+Tab" => st.step_tab(-1),
            "ArrowRight" | "Tab" => st.step_tab(1),
            "ArrowDown" => st.walk(1),
            "ArrowUp" => st.walk(-1),
            _ => {}
        }
        true
    }

    fn cheat_copy(&mut self, command: String) {
        let text = self.tr("cheat.copied").replace("{}", &command);
        self.clipboard = Some(command);
        self.say(text);
    }
}

/// The rectangle of the cell a box sits in.
fn cell(origin: Pos2, at: (u8, u8)) -> Rect {
    Rect::from_min_size(origin + vec2(at.0 as f32 * (NODE_W + GAP_X), at.1 as f32 * (NODE_H + GAP_Y)), vec2(NODE_W, NODE_H))
}

/// The points of an arrow from one box to another. Down the page it leaves the
/// bottom and enters the top; along a row it goes side to side; back up the
/// page it goes round by the right-hand gutter at `gutter_x`.
fn route(a: Rect, b: Rect, gutter_x: f32) -> Vec<Pos2> {
    if b.top() > a.bottom() - 1.0 {
        let (s, e) = (a.center_bottom(), b.center_top());
        if (s.x - e.x).abs() < 0.5 {
            return vec![s, e];
        }
        let mid = (s.y + e.y) / 2.0;
        return vec![s, pos2(s.x, mid), pos2(e.x, mid), e];
    }
    if (a.center().y - b.center().y).abs() < 1.0 {
        let (s, e) = if b.center().x > a.center().x { (a.right_center(), b.left_center()) } else { (a.left_center(), b.right_center()) };
        return vec![s, e];
    }
    let (s, e) = (a.right_center(), b.right_center());
    vec![s, pos2(gutter_x, s.y), pos2(gutter_x, e.y), e]
}

/// Where the words of an arrow sit: beside a straight down arrow, above the
/// sideways run of a bent one, over the first run of a back arrow.
fn label_at(pts: &[Pos2]) -> (Pos2, Align2) {
    match pts {
        [a, b] if (a.x - b.x).abs() < 0.5 => (pos2(a.x + 4.0, (a.y + b.y) / 2.0 - 7.0), Align2::LEFT_CENTER),
        [a, b] => (pos2((a.x + b.x) / 2.0, a.y - 8.0), Align2::CENTER_CENTER),
        [_, m, n, e] if (m.y - n.y).abs() < 0.5 => (pos2(e.x, m.y - 8.0), Align2::CENTER_CENTER),
        [a, m, _, _] => (pos2((a.x + m.x) / 2.0, a.y - 8.0), Align2::CENTER_CENTER),
        _ => (pts[0], Align2::CENTER_CENTER),
    }
}

fn arrow_head(p: &egui::Painter, from: Pos2, to: Pos2, color: Color32) {
    let d = (to - from).normalized();
    if !d.x.is_finite() {
        return;
    }
    let n = vec2(-d.y, d.x);
    p.add(Shape::convex_polygon(vec![to, to - d * 8.0 + n * 4.0, to - d * 8.0 - n * 4.0], color, Stroke::NONE));
}

/// The sheet while it is open.
pub fn show(ctx: &egui::Context, app: &mut App) {
    let Some(mut st) = app.cheat.take() else { return };
    let c = ito_theme::colors();
    let screen = ctx.content_rect();
    let max_h = (screen.height() - 100.0).max(200.0);
    let lang = app.lang.clone();
    let flow: &'static Flow = st.topic().flow();
    let cols = steps_cols(ctx, flow, &lang);
    // Sized for the largest sheet, so switching tabs does not resize the window.
    let left_w = (3.0 * NODE_W + 2.0 * GAP_X + GUTTER).min(WIDTH * 0.4);
    let right_need = Topic::ALL.iter().map(|t| steps_cols(ctx, t.flow(), &lang).total()).fold(0.0, f32::max);
    let width = (left_w + right_need + 80.0).clamp(640.0, WIDTH).min(screen.width() - 32.0);
    let body_h = Topic::ALL
        .iter()
        .map(|t| {
            let (_, rows) = t.flow().size();
            rows as f32 * NODE_H + (rows as f32 - 1.0) * GAP_Y + 8.0
        })
        .fold(0.0, f32::max)
        .min(max_h);
    let mut copy: Option<String> = None;
    let mut picked_tab: Option<usize> = None;
    let mut picked_node: Option<u8> = None;
    let area = egui::Area::new(Id::new("michi-cheat"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos2(screen.center().x - width / 2.0 - 8.0, screen.top() + 40.0))
        .show(ctx, |ui| {
            egui::Frame::new().fill(c.panel).stroke(Stroke::new(1.0, c.border)).corner_radius(6.0).inner_margin(8.0).show(ui, |ui| {
                ui.set_width(width);
                let small = FontId::proportional(12.0);
                ui.label(RichText::new(app.tr("cheat.title")).strong());
                ui.horizontal_wrapped(|ui| {
                    for (i, tp) in Topic::ALL.iter().enumerate() {
                        let name = format!("{} {}", i + 1, tp.name().get(&lang));
                        if ui.selectable_label(st.tab == i, name).clicked() {
                            picked_tab = Some(i);
                        }
                    }
                });
                ui.separator();
                ui.label(RichText::new(flow.title.get(&lang)).strong().color(c.fg));
                ui.add_space(4.0);
                let right_w = (width - left_w - ui.spacing().item_spacing.x * 2.0 - 12.0).max(300.0);
                let rows = ui.horizontal_top(|ui| {
                    let left = cell_of(ui, left_w, |ui| {
                        egui::ScrollArea::both()
                            .id_salt("cheat-flow")
                            .max_height(body_h)
                            .min_scrolled_height(body_h)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                diagram(ui, flow, &st, &lang, &mut picked_node);
                            });
                    });
                    ui.add_space(6.0);
                    let sep_x = ui.cursor().left();
                    ui.add_space(6.0);
                    let right = cell_of(ui, right_w, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt("cheat-commands")
                            .max_height(body_h)
                            .min_scrolled_height(body_h)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                if let Some(note) = flow.note {
                                    ui.label(RichText::new(note.get(&lang)).font(small.clone()).color(c.dim));
                                    ui.add_space(6.0);
                                }
                                if !flow.table.is_empty() {
                                    table(ui, flow, &lang);
                                    ui.add_space(8.0);
                                }
                                steps(ui, app, flow, &st, &lang, &cols, &mut copy, &mut picked_node);
                            });
                    });
                    (left, right, sep_x)
                });
                // The divider is as tall as the cells, not as the room left.
                let (left, right, sep_x) = rows.inner;
                let (top, bottom) = (left.top().min(right.top()), left.bottom().max(right.bottom()));
                ui.painter().vline(sep_x, top..=bottom, ui.visuals().widgets.noninteractive.bg_stroke);
                ui.add_space(4.0);
                ui.label(RichText::new(app.tr("cheat.keys")).font(small).color(c.dim));
            });
        });
    let outside = copy.is_none()
        && picked_tab.is_none()
        && picked_node.is_none()
        && ctx.input(|i| i.pointer.any_pressed())
        && ctx.input(|i| i.pointer.interact_pos()).is_some_and(|q| !area.response.rect.contains(q));
    if outside {
        return;
    }
    if let Some(i) = picked_tab {
        st.set_tab(i);
    }
    if let Some(n) = picked_node {
        st.sel = Some(n);
    }
    app.cheat = Some(st);
    if let Some(command) = copy {
        app.cheat_copy(command);
    }
}

fn diagram(ui: &mut egui::Ui, flow: &Flow, st: &Cheat, lang: &str, picked: &mut Option<u8>) {
    let c = ito_theme::colors();
    let (cols, rows) = flow.size();
    let grid_w = cols as f32 * NODE_W + (cols as f32 - 1.0) * GAP_X;
    let total_w = grid_w + GUTTER;
    let height = rows as f32 * NODE_H + (rows as f32 - 1.0) * GAP_Y;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width().max(total_w), height + 4.0), Sense::hover());
    let origin = pos2(rect.left() + ((rect.width() - total_w) / 2.0).max(0.0), rect.top() + 2.0);
    let gutter_x = origin.x + grid_w + GUTTER / 2.0;
    let p = ui.painter_at(rect);
    let line = Stroke::new(1.2, c.dim);
    for e in flow.edges {
        let (Some(a), Some(b)) = (flow.node(e.from), flow.node(e.to)) else { continue };
        let pts = route(cell(origin, a.at), cell(origin, b.at), gutter_x);
        p.add(Shape::line(pts.clone(), line));
        if let [.., prev, last] = pts.as_slice() {
            arrow_head(&p, *prev, *last, c.dim);
        }
        if let Some(label) = e.label {
            let (at, align) = label_at(&pts);
            p.text(at, align, label.get(lang), FontId::proportional(11.0), c.dim);
        }
    }
    for n in flow.nodes {
        let r = cell(origin, n.at);
        let resp = ui.interact(r, Id::new(("cheat-node", flow.topic, n.id)), Sense::click());
        if resp.clicked() {
            *picked = Some(n.id);
        }
        let on = st.sel == Some(n.id);
        let edge = match n.kind {
            Kind::Start => c.done,
            Kind::Action => c.blue,
            Kind::Decision => c.wait,
        };
        let fill = if on || resp.hovered() { c.side } else { c.bg };
        let radius = if n.kind == Kind::Decision { 20.0 } else { 6.0 };
        p.rect(r, radius, fill, Stroke::new(if on { 2.5 } else { 1.2 }, edge), egui::StrokeKind::Inside);
        let galley = p.layout(n.label.get(lang).to_owned(), FontId::proportional(12.0), c.fg, r.width() - 16.0);
        p.galley(r.center() - galley.size() / 2.0, galley, c.fg);
        if flow.steps_of(n.id).next().is_some() {
            let badge = pos2(r.left() + 9.0, r.top() + 9.0);
            p.circle_filled(badge, 8.0, edge);
            p.text(badge, Align2::CENTER_CENTER, (n.id + 1).to_string(), FontId::proportional(10.0), c.bg);
        }
    }
}

fn table(ui: &mut egui::Ui, flow: &Flow, lang: &str) {
    let c = ito_theme::colors();
    let cols = flow.table.first().map_or(1, |r| r.len()).max(1);
    let gap = ui.spacing().item_spacing.x;
    let w = ((ui.available_width() - 24.0 - gap * cols as f32) / cols as f32).max(80.0);
    for (i, row) in flow.table.iter().enumerate() {
        ui.horizontal_top(|ui| {
            for (j, cell) in row.iter().enumerate() {
                cell_of(ui, w, |ui| {
                    let text = RichText::new(cell.get(lang));
                    ui.label(if i == 0 {
                        text.strong()
                    } else if j == 0 {
                        text.monospace().color(c.blue)
                    } else {
                        text
                    });
                });
            }
        });
        ui.add_space(2.0);
    }
}

/// A table cell of a fixed width, so a long text wraps in its own column.
/// The height offered is large: a `horizontal_top` row only offers one line,
/// which clipped the text and the scroll areas to the first row.
fn cell_of(ui: &mut egui::Ui, w: f32, add: impl FnOnce(&mut egui::Ui)) -> Rect {
    ui.allocate_ui_with_layout(vec2(w, 100_000.0), egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.set_width(w);
        add(ui);
    })
    .response
    .rect
}

/// The widths of the steps table's columns, measured from its text.
struct Cols {
    num: f32,
    cmd: f32,
    why: f32,
    screen: f32,
    key: f32,
}

impl Cols {
    fn total(&self) -> f32 {
        self.num + self.cmd + self.why + self.screen + self.key + 8.0 * 4.0
    }
}

fn steps_cols(ctx: &egui::Context, flow: &Flow, lang: &str) -> Cols {
    let style = ctx.global_style();
    let (mono, body) = (egui::TextStyle::Monospace.resolve(&style), egui::TextStyle::Body.resolve(&style));
    let wide = |font: &FontId, text: &str| ctx.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE).size().x);
    let cmd = flow.steps.iter().map(|s| wide(&mono, s.command)).fold(0.0, f32::max) + 6.0;
    let why = flow.steps.iter().map(|s| wide(&body, s.why.get(lang))).fold(0.0, f32::max) + 6.0;
    Cols { num: 24.0, cmd, why, screen: 110.0, key: 90.0 }
}

#[allow(clippy::too_many_arguments)]
fn steps(
    ui: &mut egui::Ui,
    app: &App,
    flow: &Flow,
    st: &Cheat,
    lang: &str,
    cols: &Cols,
    copy: &mut Option<String>,
    picked: &mut Option<u8>,
) {
    let c = ito_theme::colors();
    let small = FontId::proportional(12.0);
    let spacing = ui.spacing().item_spacing.x;
    let (w_num, w_cmd, w_why, w_screen, w_key) = (cols.num, cols.cmd, cols.why, cols.screen, cols.key);
    let head = |ui: &mut egui::Ui, w: f32, key: &'static str| {
        cell_of(ui, w, |ui| {
            ui.label(RichText::new(app.tr(key)).font(small.clone()).color(c.dim));
        })
    };
    ui.horizontal_top(|ui| {
        ui.add_space(w_num + spacing);
        head(ui, w_cmd, "cheat.command");
        head(ui, w_why, "cheat.what");
        head(ui, w_screen, "cheat.screen");
        head(ui, w_key, "cheat.key");
    });
    for s in flow.steps {
        let on = st.sel == Some(s.node);
        ui.horizontal_top(|ui| {
            cell_of(ui, w_num, |ui| {
                let num = ui.add(
                    egui::Label::new(RichText::new((s.node + 1).to_string()).color(if on { c.fg } else { c.dim }).strong())
                        .sense(Sense::click()),
                );
                if num.clicked() {
                    *picked = Some(s.node);
                }
            });
            cell_of(ui, w_cmd, |ui| {
                let color = if s.danger {
                    // Selected: a paler red, so the change shows on a red command too.
                    if on {
                        c.err.lerp_to_gamma(c.fg, 0.55)
                    } else {
                        c.err
                    }
                } else if on {
                    c.fg
                } else {
                    c.blue
                };
                let mut cmd = RichText::new(s.command).monospace().color(color);
                if on {
                    cmd = cmd.strong();
                }
                let mut resp = ui.add(egui::Label::new(cmd).sense(Sense::click()));
                if s.danger {
                    resp = resp.on_hover_text(app.tr("cheat.danger"));
                }
                if resp.clicked() {
                    *copy = Some(s.command.to_owned());
                }
            });
            cell_of(ui, w_why, |ui| {
                ui.add(egui::Label::new(RichText::new(s.why.get(lang)).color(if on { c.fg } else { c.dim })).wrap());
            });
            cell_of(ui, w_screen, |ui| {
                ui.label(RichText::new(s.screen.unwrap_or("—")).color(c.dim));
            });
            cell_of(ui, w_key, |ui| {
                ui.label(RichText::new(s.key.unwrap_or("—")).monospace().color(c.dim));
            });
        });
        ui.add_space(2.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(col: u8, row: u8) -> Rect {
        cell(pos2(0.0, 0.0), (col, row))
    }

    #[test]
    fn an_arrow_down_a_column_is_straight_and_a_bent_one_runs_along_the_gap() {
        assert_eq!(route(r(1, 0), r(1, 1), 999.0).len(), 2);
        let bent = route(r(1, 1), r(0, 2), 999.0);
        assert_eq!(bent.len(), 4);
        assert_eq!(bent[1].y, bent[2].y);
        assert!(bent[1].y > r(1, 1).bottom() && bent[1].y < r(0, 2).top());
    }

    #[test]
    fn an_arrow_back_up_goes_round_the_gutter() {
        let back = route(r(1, 7), r(1, 3), 500.0);
        assert_eq!(back.len(), 4);
        assert_eq!(back[1].x, 500.0);
        assert_eq!(back[3], r(1, 3).right_center());
    }

    #[test]
    fn the_keys_change_the_sheet_and_walk_the_boxes() {
        let mut st = Cheat::default();
        st.set_tab(3);
        assert_eq!(st.topic(), Topic::Reset);
        st.step_tab(-4);
        assert_eq!(st.topic(), Topic::Rescue);
        st.walk(1);
        assert_eq!(st.sel, Some(st.topic().flow().walk()[0]));
        st.walk(-1);
        assert_eq!(st.sel, Some(st.topic().flow().walk()[0]));
        st.set_tab(0);
        assert_eq!(st.sel, None);
    }

    #[test]
    fn every_sheet_fits_the_window_without_a_box_over_another() {
        for t in Topic::ALL {
            let f = t.flow();
            let rects: Vec<Rect> = f.nodes.iter().map(|n| cell(pos2(0.0, 0.0), n.at)).collect();
            for (i, a) in rects.iter().enumerate() {
                assert!(rects[i + 1..].iter().all(|b| !a.intersects(*b)));
            }
            let (cols, _) = f.size();
            assert!(cols as f32 * (NODE_W + GAP_X) <= WIDTH);
        }
    }
}
