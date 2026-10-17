//! Drag and drop in the graph view (#67). A commit's row in the graph or a
//! branch's row in the side column is dragged onto a worktree's row in the
//! side column; a small menu lists what `michi_core::dnd` says can be done,
//! and a pick asks the same band as the key does (the check and the backup
//! are the key's). Nothing runs on the drop itself.

use std::path::PathBuf;

use egui::{vec2, Button, Context, CursorIcon, FontId, Id, Order, Pos2, Rect, RichText, Stroke, Ui};
use michi_core::dnd::{self, Op, Source};
use michi_core::paths;

use crate::app::App;

/// The menu a drop opened.
#[derive(Clone, Debug)]
pub struct DropMenu {
    pub source: Source,
    pub wt: PathBuf,
    /// The target's branch; `None` when it is detached.
    pub branch: Option<String>,
    pub at: Pos2,
}

/// A worktree's row that takes a drop: where it is drawn and its folder.
pub type Target = (Rect, PathBuf);

fn targets_id() -> Id {
    Id::new("dnd-targets")
}

/// Before the side column is drawn: forget last frame's rows.
pub fn begin(ctx: &Context) {
    ctx.data_mut(|d| d.insert_temp::<Vec<Target>>(targets_id(), Vec::new()));
}

/// The side column tells where a worktree's row is.
pub fn target(ctx: &Context, rect: Rect, wt: PathBuf) {
    ctx.data_mut(|d| d.get_temp_mut_or_default::<Vec<Target>>(targets_id()).push((rect, wt)));
}

/// The name a source wears in the ghost under the pointer.
fn ghost(app: &App, source: &Source) -> String {
    match source {
        Source::Commit(id) => match app.graph.find_commit(id) {
            Some(c) => format!("{}  {}", id.short(), c.subject),
            None => id.short(),
        },
        Source::Branch { name, .. } => name.clone(),
    }
}

/// After the columns: draws what is carried, and on the release opens the
/// menu over a worktree, then draws the menu.
pub fn finish(ui: &mut Ui, app: &mut App, live: bool) {
    let ctx = ui.ctx().clone();
    if let Some(source) = app.dnd.clone() {
        let c = ito_theme::colors();
        ctx.set_cursor_icon(CursorIcon::Grabbing);
        let pos = ctx.pointer_latest_pos();
        if let Some(p) = pos {
            let text = ghost(app, &source);
            let g = ctx.fonts_mut(|f| f.layout_no_wrap(text, FontId::proportional(12.0), c.fg));
            let r = Rect::from_min_size(p + vec2(12.0, 8.0), g.size() + vec2(14.0, 8.0));
            let painter = ctx.layer_painter(egui::LayerId::new(Order::Tooltip, Id::new("dnd-ghost")));
            painter.rect(r, 4.0, c.panel, Stroke::new(1.0, c.border), egui::StrokeKind::Inside);
            painter.galley(r.min + vec2(7.0, 4.0), g, c.fg);
        }
        let held = ctx.input(|i| i.pointer.any_down());
        if !held || !live {
            app.dnd = None;
            let targets: Vec<Target> = ctx.data(|d| d.get_temp(targets_id())).unwrap_or_default();
            let hit = pos.and_then(|p| targets.into_iter().find(|(r, _)| r.contains(p)).map(|(_, wt)| (p, wt)));
            if let (true, Some((at, wt))) = (live, hit) {
                let branch = app.worktrees().into_iter().find(|w| paths::key(&w.path) == paths::key(&wt)).and_then(|w| w.branch);
                if dnd::ops(&source, branch.as_deref()).is_empty() {
                    app.say_for(app.tr("dnd.nothing").to_owned(), 5);
                } else {
                    app.drop_menu = Some(DropMenu { source, wt, branch, at });
                }
            }
        }
        ctx.request_repaint();
    }
    menu(&ctx, app);
}

/// The small menu over the worktree the drop landed on.
fn menu(ctx: &Context, app: &mut App) {
    let Some(m) = app.drop_menu.clone() else { return };
    let c = ito_theme::colors();
    let mut pick: Option<Op> = None;
    let area = egui::Area::new(Id::new("michi-drop-menu")).order(Order::Foreground).fixed_pos(m.at + vec2(4.0, 4.0)).show(ctx, |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.set_min_width(220.0);
            let head = app
                .tr("dnd.head")
                .replace("{src}", &m.source.label())
                .replace("{wt}", m.branch.as_deref().unwrap_or(&crate::act::name(&m.wt)));
            ui.label(RichText::new(head).font(FontId::proportional(12.0)).color(c.dim));
            for op in dnd::ops(&m.source, m.branch.as_deref()) {
                if ui.add(Button::new(app.tr(op.word()))).clicked() {
                    pick = Some(op);
                }
            }
        });
    });
    let outside = ctx.input(|i| i.pointer.any_pressed()) && !area.response.contains_pointer();
    let esc = ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
    if pick.is_some() || outside || esc {
        app.drop_menu = None;
    }
    if let Some(op) = pick {
        app.ask_drop(&m, op);
    }
    ctx.request_repaint();
}

impl App {
    /// The entry picked in the drop menu: the band of the same key.
    pub(crate) fn ask_drop(&mut self, m: &DropMenu, op: Op) {
        let wt = m.wt.clone();
        let id = m.source.id();
        let subject = self.graph.find_commit(&id).map(|c| c.subject.clone()).unwrap_or_default();
        match op {
            Op::Merge => self.ask_merge_into(wt, m.source.rev(), m.source.label(), subject),
            Op::CherryPick => {
                let parents = self.graph.find_commit(&id).map_or(1, |c| c.parents.len());
                self.ask_cherry_onto(wt, id, subject, parents);
            }
            Op::Rebase => self.ask_rebase_onto(wt, m.source.rev()),
        }
    }
}

/// The row of a list of `len` rows, `row_h` high from `top`, at `y`.
pub fn row_at(top: f32, row_h: f32, len: u32, y: f32) -> Option<u32> {
    let k = ((y - top) / row_h).floor();
    (k >= 0.0 && (k as u32) < len).then_some(k as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_is_found_under_the_pointer() {
        assert_eq!(row_at(100.0, 20.0, 5, 99.0), None);
        assert_eq!(row_at(100.0, 20.0, 5, 100.0), Some(0));
        assert_eq!(row_at(100.0, 20.0, 5, 199.0), Some(4));
        assert_eq!(row_at(100.0, 20.0, 5, 200.0), None);
    }

    #[test]
    fn the_drop_menu_shows_in_a_frame() {
        let mut app = App::new(crate::prefs::Prefs::empty(), std::env::temp_dir());
        let id = michi_core::oid::Oid::parse(&"a".repeat(40)).unwrap();
        app.drop_menu = Some(DropMenu {
            source: Source::Commit(id),
            wt: PathBuf::from("/r/feat"),
            branch: Some("feat".into()),
            at: egui::pos2(10.0, 10.0),
        });
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| finish(ui, &mut app, true));
        out.textures_delta.clear();
        assert!(app.drop_menu.is_some(), "nothing closes it without a click");
    }

    #[test]
    fn a_release_over_nothing_drops_the_drag() {
        let mut app = App::new(crate::prefs::Prefs::empty(), std::env::temp_dir());
        let id = michi_core::oid::Oid::parse(&"a".repeat(40)).unwrap();
        app.dnd = Some(Source::Commit(id));
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| finish(ui, &mut app, true));
        out.textures_delta.clear();
        assert!(app.dnd.is_none());
        assert!(app.drop_menu.is_none());
    }
}
