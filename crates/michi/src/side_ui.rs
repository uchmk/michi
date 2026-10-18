//! The side column: the worktrees, the branches, the remote branches,
//! the tags and the stashes, each a click away from its commit in the graph
//! (a stash's is not in it: the detail shows it). It scrolls by the wheel, with no scrollbar
//! (docs/design.md: the graph's is the one).

use egui::{pos2, vec2, Align2, CornerRadius, CursorIcon, FontId, Id, Rect, Sense, Shape, Stroke, StrokeKind, Ui};
use michi_core::density::Density;
use michi_core::oid::Oid;
use michi_core::paths;

use crate::app::{App, Repo};
use crate::graph_ui::{caret, clipped};

const TOP_PAD: f32 = 6.0;
const ROW_H: f32 = 23.0;
const HEAD_H: f32 = 24.0;
const LEFT: f32 = 14.0;
/// How far in a group's rows start.
const SUB: f32 = 30.0;
const RIGHT: f32 = 12.0;

fn scroll_id() -> Id {
    Id::new("side-scroll")
}

/// A row of the side column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub kind: Kind,
    pub text: String,
    /// In a group: further in.
    pub sub: bool,
    /// The commit a click selects.
    pub id: Option<Oid>,
    pub tip: Option<String>,
    /// On the right: how far a branch is ahead of its upstream and behind
    /// it (`↑2 ↓1`).
    pub note: String,
    /// A heading that is a glossary term: its hover explains it.
    pub term: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A section's heading and how many it holds.
    Heading(usize),
    /// A group to open or close (its key in `App::side_open`), and how many
    /// it holds.
    Group {
        key: String,
        open: bool,
        count: usize,
    },
    /// A worktree, `cur` for the one the window is on.
    Worktree {
        color: u8,
        cur: bool,
    },
    Branch {
        color: u8,
    },
    Tag,
    Stash,
}

impl Row {
    fn height(&self, d: Density) -> f32 {
        d.px(match self.kind {
            Kind::Heading(_) => HEAD_H,
            _ => ROW_H,
        })
    }
}

/// The rows, from the history's setup.
pub fn rows(app: &App) -> Vec<Row> {
    let setup = app.graph.setup();
    let g = &app.graph;
    let mut out = Vec::new();
    let note = |branch: Option<&str>| match branch.and_then(|b| setup.refs.track.get(b)) {
        Some(&(ahead, behind)) => {
            let mut s = String::new();
            if ahead > 0 {
                s = format!("↑{ahead}");
            }
            if behind > 0 {
                s = format!("{s} ↓{behind}").trim_start().to_owned();
            }
            s
        }
        None => String::new(),
    };
    let item = |kind, text: String, sub, id, tip| Row { kind, text, sub, id, tip, note: String::new(), term: None };
    let heading = |key: &'static str, n: usize| {
        // The headings that are Git's words are glossary terms (#104, #105).
        let term = match key {
            "side.worktrees" => Some("worktree"),
            "side.tags" => Some("tag"),
            "side.stash" => Some("stash"),
            _ => None,
        };
        let text = match term {
            Some(t) => app.term(key, t),
            None => app.tr(key).to_owned(),
        };
        Row { kind: Kind::Heading(n), text, sub: false, id: None, tip: None, note: String::new(), term }
    };

    // The worktrees; the one the window is on lit.
    let top = match &app.repo {
        Repo::Ready(s) => Some(paths::key(&s.top)),
        _ => None,
    };
    let trees: Vec<_> = setup.worktrees.iter().filter(|w| !w.bare).collect();
    if !trees.is_empty() {
        out.push(heading("side.worktrees", trees.len()));
        for w in trees {
            let path = w.path.to_string_lossy();
            let folder = path.trim_end_matches('/').rsplit('/').next().unwrap_or(&path).to_owned();
            let text = w.branch.clone().unwrap_or(folder);
            let cur = top.as_deref() == Some(paths::key(&w.path).as_str());
            let note = note(w.branch.as_deref());
            let row = item(Kind::Worktree { color: g.color_of(&text), cur }, text, false, Oid::parse(&w.head), Some(path.into_owned()));
            out.push(Row { note, ..row });
        }
    }

    // The branches: those without a slash, then the rest by what is before
    // their first slash.
    let locals = &setup.refs.locals;
    if !locals.is_empty() {
        out.push(heading("side.branches", locals.len()));
        for r in locals.iter().filter(|r| !r.name.contains('/')) {
            let row = item(Kind::Branch { color: g.color_of(&r.name) }, r.name.clone(), false, Some(r.id), None);
            out.push(Row { note: note(Some(&r.name)), ..row });
        }
        let mut groups: Vec<&str> = locals.iter().filter_map(|r| r.name.split_once('/').map(|(p, _)| p)).collect();
        groups.sort_unstable();
        groups.dedup();
        for prefix in groups {
            let members: Vec<_> = locals.iter().filter(|r| r.name.split_once('/').is_some_and(|(p, _)| p == prefix)).collect();
            let key = format!("b:{prefix}");
            let open = app.side_open.contains(&key);
            out.push(item(Kind::Group { key, open, count: members.len() }, format!("{prefix}/"), false, None, None));
            if open {
                for r in members {
                    let rest = r.name[prefix.len() + 1..].to_owned();
                    let row = item(Kind::Branch { color: g.color_of(&r.name) }, rest, true, Some(r.id), Some(r.name.clone()));
                    out.push(Row { note: note(Some(&r.name)), ..row });
                }
            }
        }
    }

    // The remote branches, a group per remote.
    let remotes: Vec<_> = setup.refs.remotes.iter().filter(|r| r.name != "HEAD").collect();
    if !remotes.is_empty() {
        let mut names: Vec<&str> = remotes.iter().map(|r| r.remote.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        out.push(heading("side.remotes", names.len()));
        for remote in names {
            let members: Vec<_> = remotes.iter().filter(|r| r.remote == remote).collect();
            let key = format!("r:{remote}");
            let open = app.side_open.contains(&key);
            out.push(item(Kind::Group { key, open, count: members.len() }, remote.to_owned(), false, None, None));
            if open {
                for r in members {
                    let full = format!("{}/{}", r.remote, r.name);
                    out.push(item(Kind::Branch { color: g.color_of(&r.name) }, r.name.clone(), true, Some(r.id), Some(full)));
                }
            }
        }
    }

    let tags = &setup.refs.tags;
    if !tags.is_empty() {
        out.push(heading("side.tags", tags.len()));
        for t in tags {
            out.push(item(Kind::Tag, t.name.clone(), false, Some(t.id), None));
        }
    }

    // The stashes. Their commits are not in the graph: a click shows one
    // in the detail.
    let stashes = &setup.refs.stashes;
    if !stashes.is_empty() {
        out.push(heading("side.stash", stashes.len()));
        for s in stashes {
            let tip = format!("{} {}", s.id.short(), s.subject);
            out.push(item(Kind::Stash, s.subject.clone(), false, Some(s.id), Some(tip)));
        }
    }
    out
}

/// What a click on a row does, once they are drawn.
enum Act {
    Go(Oid),
    Stash(Oid),
    Toggle(String),
}

/// The side column in `r`.
pub fn show(ui: &mut Ui, app: &mut App, r: Rect, live: bool) {
    let c = ito_theme::colors();
    let ansi = c.ansi();
    let ctx = ui.ctx().clone();
    ui.painter().rect_filled(r, 0.0, c.side);
    let rows = rows(app);
    let total: f32 = TOP_PAD + rows.iter().map(|r| r.height(app.prefs.density)).sum::<f32>() + 12.0;
    let mut scroll = ctx.data(|d| d.get_temp::<f32>(scroll_id())).unwrap_or(0.0);
    if live && ui.rect_contains_pointer(r) {
        scroll -= ui.input(|i| i.smooth_scroll_delta.y);
    }
    if std::mem::take(&mut app.side_follow) {
        let top = TOP_PAD + rows.iter().take(app.side_cur).map(|r| r.height(app.prefs.density)).sum::<f32>();
        let h = rows.get(app.side_cur).map_or(ROW_H, |r| r.height(app.prefs.density));
        if top < scroll {
            scroll = top - HEAD_H;
        } else if top + h > scroll + r.height() {
            scroll = top + h - r.height() + 6.0;
        }
    }
    scroll = scroll.clamp(0.0, (total - r.height()).max(0.0));
    ctx.data_mut(|d| d.insert_temp(scroll_id(), scroll));

    let p = ui.painter_at(r);
    let pointer = if live { ui.input(|i| i.pointer.hover_pos()).filter(|p| r.contains(*p)) } else { None };
    let mut act = None;
    let shown = app.stash_shown();
    let mut y = r.top() + TOP_PAD - scroll;
    for (k, row) in rows.iter().enumerate() {
        let h = row.height(app.prefs.density);
        let rr = Rect::from_min_size(pos2(r.left(), y), vec2(r.width(), h));
        y += h;
        if rr.bottom() < r.top() || rr.top() > r.bottom() {
            continue;
        }
        let right = rr.right() - RIGHT;
        if let Kind::Heading(n) = row.kind {
            let mid = rr.top() + 8.0 + 13.0 / 2.0;
            let count = p.text(pos2(right, mid), Align2::RIGHT_CENTER, n.to_string(), FontId::monospace(10.5), c.dim);
            let title = row.text.to_uppercase();
            let title: String = title.chars().flat_map(|ch| [ch, '\u{200A}']).collect();
            let room = count.left() - 6.0 - rr.left() - 12.0;
            clipped(ui, &p, pos2(rr.left() + 12.0, mid), title.trim_end(), FontId::proportional(11.0), c.dim, room);
            if let Some(t) = row.term {
                let g = ui.fonts_mut(|f| f.layout_no_wrap(title.trim_end().to_owned(), FontId::proportional(11.0), c.dim));
                let w = g.size().x.min(room.max(0.0));
                crate::terms::hover(ui, &p, Rect::from_min_size(pos2(rr.left() + 12.0, mid - 6.0), vec2(w, 12.0)), t, &app.lang, "side");
            }
            continue;
        }
        let clickable = row.id.is_some() || matches!(row.kind, Kind::Group { .. });
        let draggable = matches!(row.kind, Kind::Branch { .. }) && row.id.is_some();
        let sense = if draggable {
            Sense::click_and_drag()
        } else if clickable {
            Sense::CLICK
        } else {
            Sense::hover()
        };
        let resp = ui.interact(rr.intersect(r), Id::new(("side-row", k)), sense);
        let hot = live && pointer.is_some_and(|p| rr.contains(p));
        // A branch is dragged onto a worktree (#67); a worktree's row takes the drop.
        if let (true, true, Some(tip)) = (live, draggable && resp.drag_started_by(egui::PointerButton::Primary), row.id) {
            let name = row.tip.clone().unwrap_or_else(|| row.text.clone());
            app.dnd = Some(michi_core::dnd::Source::Branch { name, tip });
        }
        if let (Kind::Worktree { .. }, Some(path)) = (&row.kind, &row.tip) {
            crate::dnd_ui::target(&ctx, rr.intersect(r), std::path::PathBuf::from(path));
            if hot && app.dnd.is_some() {
                p.rect_stroke(rr.shrink(1.0), 3.0, Stroke::new(1.5, c.run), StrokeKind::Inside);
            }
        }
        if let Kind::Worktree { cur: true, .. } = row.kind {
            p.rect_filled(rr, 0.0, c.run.gamma_multiply(0.12));
        } else if row.kind == Kind::Stash && shown.is_some() && row.id == shown {
            p.rect_filled(rr, 0.0, c.blue.gamma_multiply(0.22));
        } else if hot {
            p.rect_filled(rr, 0.0, c.panel);
        }
        if live && app.pane == crate::panes::GraphPane::Side && k == app.side_cur {
            p.rect_filled(rr, 0.0, c.blue.gamma_multiply(0.28));
            p.rect_filled(Rect::from_min_size(rr.min, vec2(3.0, rr.height())), 0.0, c.blue);
        }
        let mid = rr.center().y;
        let mut x = rr.left() + if row.sub { SUB } else { LEFT };
        let mut ink = c.fg;
        match &row.kind {
            Kind::Group { open, count, .. } => {
                caret(&p, pos2(x + 4.0, mid), *open, c.dim);
                x += 14.0;
                let n = p.text(pos2(right, mid), Align2::RIGHT_CENTER, count.to_string(), FontId::monospace(11.0), c.dim);
                clipped(ui, &p, pos2(x, mid), &row.text, FontId::proportional(12.5), c.fg, n.left() - 6.0 - x);
                if hot {
                    ctx.set_cursor_icon(CursorIcon::PointingHand);
                }
                if live && resp.clicked() {
                    if let Kind::Group { key, .. } = &row.kind {
                        act = Some(Act::Toggle(key.clone()));
                    }
                }
                continue;
            }
            Kind::Worktree { color, cur } => {
                swatch(&p, pos2(x + 4.0, mid), ansi[*color as usize & 15]);
                if *cur {
                    ink = c.run;
                }
            }
            Kind::Branch { color } => swatch(&p, pos2(x + 4.0, mid), ansi[*color as usize & 15]),
            Kind::Tag => {
                let d = pos2(x + 4.0, mid);
                let diamond = vec![d + vec2(0.0, -4.0), d + vec2(4.0, 0.0), d + vec2(0.0, 4.0), d + vec2(-4.0, 0.0)];
                p.add(Shape::convex_polygon(diamond, c.wait, Stroke::NONE));
                ink = c.wait;
            }
            Kind::Stash => {
                p.rect_stroke(Rect::from_center_size(pos2(x + 4.0, mid), vec2(8.0, 8.0)), 1.0, Stroke::new(1.0, c.dim), StrokeKind::Inside);
                ink = c.dim;
            }
            Kind::Heading(_) => {}
        }
        x += 14.0;
        let mut end = right;
        if !row.note.is_empty() {
            let n = p.text(pos2(right, mid), Align2::RIGHT_CENTER, &row.note, FontId::monospace(11.0), c.dim);
            end = n.left() - 6.0;
        }
        clipped(ui, &p, pos2(x, mid), &row.text, FontId::proportional(12.5), ink, end - x);
        if hot && row.id.is_some() {
            ctx.set_cursor_icon(CursorIcon::PointingHand);
        }
        if let (true, Some(id)) = (live && resp.clicked(), row.id) {
            act = Some(if row.kind == Kind::Stash { Act::Stash(id) } else { Act::Go(id) });
        }
        if let Some(tip) = &row.tip {
            resp.on_hover_text(tip);
        }
    }
    match act {
        Some(Act::Go(id)) => app.go_to(id),
        Some(Act::Stash(id)) => app.show_stash(id),
        Some(Act::Toggle(key)) => {
            if !app.side_open.remove(&key) {
                app.side_open.insert(key);
            }
        }
        None => return,
    }
    ctx.request_repaint();
}

/// A branch's colour as a small square, centred on `at`.
fn swatch(p: &egui::Painter, at: egui::Pos2, color: egui::Color32) {
    p.rect_filled(Rect::from_center_size(at, vec2(8.0, 8.0)), CornerRadius::same(2), color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::fixture::{self, id};
    use crate::repo::Answer;

    fn app() -> App {
        let mut a = App::new(crate::prefs::Prefs::empty(), "C:/r/repo".into());
        a.lang = "en".into();
        a.take(Answer::Setup { gen: 1, setup: Box::new(fixture::setup()) });
        a.take(Answer::Commits { gen: 1, batch: fixture::history(), done: true });
        a
    }

    fn texts(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|r| r.text.as_str()).collect()
    }

    #[test]
    fn the_sections_list_the_refs_and_groups_open_on_a_click() {
        let mut a = app();
        let rows = rows(&a);
        assert_eq!(texts(&rows), ["worktree", "main", "wip", "old", "Branches", "main", "wip", "Remotes", "origin", "Tags", "v1"]);
        assert_eq!(rows[3].id, Some(id(4)), "a detached worktree goes by its folder");
        assert_eq!(rows[3].tip.as_deref(), Some("C:/r/old"));
        assert_eq!(rows[8].kind, Kind::Group { key: "r:origin".into(), open: false, count: 2 });
        a.side_open.insert("r:origin".into());
        let rows = super::rows(&a);
        assert_eq!(texts(&rows)[8..11], ["origin", "main", "feat"]);
        assert!(rows[10].sub);
        assert_eq!((rows[10].id, rows[10].tip.as_deref()), (Some(id(3)), Some("origin/feat")));
    }

    #[test]
    fn branches_tell_their_upstream_and_stashes_follow() {
        let mut setup = fixture::setup();
        setup.refs.track.insert("wip".into(), (2, 1));
        setup.refs.track.insert("main".into(), (0, 3));
        setup.refs.stashes.push(michi_core::refs::Stash { id: id(5), subject: "On wip: half done".into() });
        let mut a = App::new(crate::prefs::Prefs::empty(), "C:/r/repo".into());
        a.lang = "en".into();
        a.take(Answer::Setup { gen: 1, setup: Box::new(setup) });
        let rows = rows(&a);
        let notes: Vec<_> = rows.iter().map(|r| (r.text.as_str(), r.note.as_str())).filter(|(_, n)| !n.is_empty()).collect();
        assert_eq!(notes, [("main", "↓3"), ("wip", "↑2 ↓1"), ("main", "↓3"), ("wip", "↑2 ↓1")]);
        assert_eq!(texts(&rows)[rows.len() - 2..], ["Stash", "On wip: half done"]);
        assert_eq!(rows.last().unwrap().id, Some(id(5)), "a click shows it in the detail");
    }

    #[test]
    fn the_worktree_the_window_is_on_is_lit() {
        let mut a = app();
        a.take(Answer::Summary(Ok(michi_core::summary::Summary {
            name: "repo".into(),
            head: michi_core::summary::Head::Branch("wip".into()),
            worktrees: 3,
            top: if cfg!(windows) { "C:\\r\\wt\\" } else { "C:/r/wt/" }.into(),
        })));
        let cur: Vec<_> = rows(&a).into_iter().filter(|r| matches!(r.kind, Kind::Worktree { cur: true, .. })).collect();
        assert_eq!(texts(&cur), ["wip"]);
    }

    #[test]
    fn branches_with_a_slash_are_grouped() {
        let mut setup = fixture::setup();
        setup.refs.locals.push(michi_core::refs::Ref { name: "feat/a".into(), id: id(2) });
        setup.refs.locals.push(michi_core::refs::Ref { name: "feat/b".into(), id: id(3) });
        let mut a = App::new(crate::prefs::Prefs::empty(), "C:/r/repo".into());
        a.take(Answer::Setup { gen: 1, setup: Box::new(setup) });
        a.side_open.insert("b:feat".into());
        let rows = rows(&a);
        let at = rows.iter().position(|r| r.text == "feat/").unwrap();
        assert_eq!(rows[at].kind, Kind::Group { key: "b:feat".into(), open: true, count: 2 });
        assert_eq!(texts(&rows)[at + 1..at + 3], ["a", "b"]);
        assert_eq!(rows[at + 2].tip.as_deref(), Some("feat/b"));
    }

    #[test]
    fn the_column_draws() {
        let ctx = egui::Context::default();
        let mut a = app();
        a.side_open.insert("r:origin".into());
        for h in [600.0, 80.0] {
            let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, vec2(220.0, h))), ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| {
                let r = ui.max_rect();
                show(ui, &mut a, r, true);
            });
            out.textures_delta.clear();
        }
    }
}
