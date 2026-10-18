//! The worktree view (`3`): a row per worktree with its branch and folder,
//! the Claude working in it, how far it is from the trunk and from where it
//! pushes, what is not committed, what is not seen yet and what it would
//! conflict with; under them, the table of the pairs that would conflict and
//! the list of the commits not seen yet. Drawn with the painter;
//! it scrolls by the wheel with no scrollbar (docs/design.md: the graph's is
//! the one). In a narrow window the rows become cards (docs/mock).

use std::sync::Arc;

use egui::text::{LayoutJob, TextWrapping};
use egui::{
    pos2, vec2, Align2, Color32, CornerRadius, CursorIcon, FontId, Galley, Id, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, TextFormat,
    Ui,
};
use michi_core::clash::Outcome;
use michi_core::claude::{Session, State};
use michi_core::oid::Oid;
use michi_core::paths;
use michi_core::queue::Stage;
use michi_core::track::Tree;

use crate::app::{failed_word, App, Repo};
use crate::graph_ui::{claude_look, paint_icon, Icon};
use crate::intro_ui;
use crate::sizes::Sizes;
use michi_core::guide::Topic;

const PAD_X: f32 = 18.0;
const PAD_TOP: f32 = 14.0;
const TOP_H: f32 = 30.0;
const HEAD_H: f32 = 26.0;
/// A cell's padding.
const CELL_X: f32 = 10.0;
const CELL_Y: f32 = 9.0;
/// The narrowest a column can be dragged to.
const COL_MIN: f32 = 60.0;
/// The narrowest the name and Claude columns get before the others give.
const REST_MIN: f32 = 200.0;
const REST_MIN_CLAUDE: f32 = 340.0;
/// The table of pairs' box: its padding and a cell's height.
const BOX_PAD: f32 = 12.0;
const MX_H: f32 = 26.0;
/// A line of the list of unseen commits, and the most lines it shows.
const NEW_H: f32 = 24.0;
const NEW_MOST: usize = 12;
/// The gap between the two boxes side by side, and the narrowest the list
/// gets beside the table of pairs before it goes under it.
const BOX_GAP: f32 = 18.0;
const NEW_MIN: f32 = 320.0;
/// Below this width the rows are cards.
pub const CARDS: f32 = 960.0;
/// The label column of a card's lines.
const LABEL_W: f32 = 96.0;
const BTN_H: f32 = 23.0;
const BTN_GAP: f32 = 4.0;
/// The swatch and the gap after it, before a row's name and folder.
const INDENT: f32 = 16.0;

fn scroll_id() -> Id {
    Id::new("wt-scroll")
}

/// The selected row when the view last showed it: the view follows a
/// selection moved by the keys.
fn seen_id() -> Id {
    Id::new("wt-seen")
}

/// The pair picked in the table of pairs, by the two sides' names.
fn picked_id() -> Id {
    Id::new("wt-pair")
}

/// What a button does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Act {
    /// Opens the graph on the row.
    Enter,
    /// The row's key (`o`, `d`, `m`, `x`, `p`).
    Key(char),
    /// The second key of `g`.
    Go(char),
    Pr,
}

struct Button {
    label: Arc<Galley>,
    key: Option<Arc<Galley>>,
    act: Act,
    danger: bool,
}

impl Button {
    fn width(&self) -> f32 {
        7.0 + self.label.size().x + self.key.as_ref().map_or(0.0, |k| 4.0 + k.size().x) + 7.0
    }
}

/// Where `buttons` go in `width`, from the top left, wrapping.
fn flow(buttons: &[Button], width: f32) -> Vec<Rect> {
    let (mut x, mut y) = (0.0, 0.0);
    buttons
        .iter()
        .map(|b| {
            let w = b.width();
            if x > 0.0 && x + w > width {
                x = 0.0;
                y += BTN_H + BTN_GAP;
            }
            let r = Rect::from_min_size(pos2(x, y), vec2(w, BTN_H));
            x += w + BTN_GAP;
            r
        })
        .collect()
}

/// One piece of a cell, top down.
enum Piece {
    /// A line of text `gap` points under the piece above, `indent` in.
    Text {
        g: Arc<Galley>,
        gap: f32,
        indent: f32,
        bold: bool,
    },
    /// The row's name in bold, after its branch's colour.
    Name {
        g: Arc<Galley>,
        color: Color32,
    },
    Buttons {
        gap: f32,
        buttons: Vec<Button>,
    },
    /// A line of text after the blue dot of what is not seen yet.
    Dot {
        g: Arc<Galley>,
    },
}

impl Piece {
    fn height(&self, width: f32) -> f32 {
        match self {
            Piece::Text { g, gap, .. } => gap + g.size().y,
            Piece::Name { g, .. } | Piece::Dot { g } => g.size().y,
            Piece::Buttons { gap, buttons } => gap + flow(buttons, width).last().map_or(0.0, |r| r.bottom()),
        }
    }
}

/// A cell: its left edge and width (inside its padding) and what is in it.
struct Col {
    x: f32,
    w: f32,
    pieces: Vec<Piece>,
}

impl Col {
    fn height(&self) -> f32 {
        self.pieces.iter().map(|p| p.height(self.w)).sum()
    }
}

/// Cells side by side; as tall as the tallest.
fn line_h(cols: &[Col]) -> f32 {
    cols.iter().map(Col::height).fold(0.0, f32::max)
}

fn layout(ui: &Ui, text: impl Into<String>, font: FontId, color: Color32, width: f32) -> Arc<Galley> {
    ui.fonts_mut(|f| f.layout(text.into(), font, color, width.max(10.0)))
}

fn fmt(font: FontId, color: Color32) -> TextFormat {
    TextFormat { font_id: font, color, ..Default::default() }
}

fn text(g: Arc<Galley>, gap: f32) -> Piece {
    Piece::Text { g, gap, indent: 0.0, bold: false }
}

/// The name a row goes by: its branch, else its folder's name.
pub fn name_of(t: &Tree) -> String {
    let path = t.wt.path.to_string_lossy();
    let folder = path.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or(&path).to_owned();
    t.wt.branch.clone().unwrap_or(folder)
}

/// Whether the worktree's branch is merged into another's and it holds no
/// uncommitted change: nothing is lost by removing it.
fn removable(app: &App, t: &Tree) -> bool {
    t.merged_into.is_some() && t.doing.is_none() && app.wip_in(&t.wt.path).is_none_or(|w| w.files.is_empty() && w.left_out.is_empty())
}

/// The branch and the folder, and whether git keeps the worktree locked or
/// would prune it.
fn name_cell(ui: &Ui, app: &App, t: &Tree, w: f32) -> Vec<Piece> {
    let c = ito_theme::colors();
    let name = name_of(t);
    let color = c.ansi()[app.graph.color_of(&name) as usize & 15];
    let mut out = vec![Piece::Name { g: layout(ui, name, FontId::proportional(13.0), c.fg, w - INDENT), color }];
    let path = t.wt.path.display().to_string();
    let font = FontId::monospace(11.0);
    let cut = crate::detail_ui::left_cut(ui, &path, font.clone(), w - INDENT);
    out.push(Piece::Text { g: layout(ui, cut, font, c.dim, w - INDENT), gap: 2.0, indent: INDENT, bold: false });
    let mut note = |words: String, ink| {
        out.push(Piece::Text { g: layout(ui, words, FontId::proportional(11.5), ink, w - INDENT), gap: 3.0, indent: INDENT, bold: false });
    };
    if t.wt.branch.is_none() {
        let at = t.wt.head.chars().take(7).collect::<String>();
        note(app.tr("wt.detached").replace("{}", &at), c.dim);
    }
    if let Some(into) = &t.merged_into {
        let words = app.tr(if removable(app, t) { "wt.merged_clean" } else { "wt.merged" }).replacen("{}", into, 1).replacen("{}", into, 1);
        note(words, c.done);
    } else if let Some((n, m, k)) = app.land_order_of(&t.wt.path) {
        let key = if k > 0 { "wt.order_resolve" } else { "wt.order" };
        let words = app.tr(key).replace("{n}", &n.to_string()).replace("{m}", &m.to_string()).replace("{k}", &k.to_string());
        note(words, if k > 0 { c.wait } else { c.done });
    }
    if let Some((_, who)) = app.identities.iter().find(|(p, _)| paths::key(p) == paths::key(&t.wt.path)) {
        note(app.tr("wt.author").replace("{}", who), c.dim);
    }
    if let Some(branch) = t.wt.branch.as_deref() {
        let labels: Vec<String> = app.branch_issues(branch).iter().map(|(r, _)| r.label()).collect();
        if !labels.is_empty() {
            note(app.tr("wt.issue").replace("{}", &labels.join(", ")), c.blue);
        }
    }
    if t.wt.locked.is_some() {
        note(app.tr("wt.locked").to_owned(), c.wait);
    }
    if t.wt.prunable.is_some() {
        note(app.tr("wt.prunable").to_owned(), c.err);
    }
    out
}

/// When a session's state began, as `HH:MM`.
fn since(s: &Session) -> Option<String> {
    chrono::DateTime::from_timestamp_millis(s.since_ms as i64)
        .filter(|_| s.since_ms > 0)
        .map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string())
}

/// The Claude in the worktree: what it does since when, what it last said,
/// and the button to its tab.
fn claude_cell(ui: &Ui, app: &App, t: &Tree, w: f32) -> Vec<Piece> {
    let c = ito_theme::colors();
    let sessions = app.claude_in(&t.wt.path);
    let Some(s) = sessions.first() else {
        return vec![text(layout(ui, app.tr("wt.no_claude"), FontId::proportional(12.5), c.dim, w), 0.0)];
    };
    let mut job = LayoutJob::default();
    job.wrap.max_width = w;
    match claude_look(app, s) {
        Some((glyph, word, ink)) => job.append(&format!("{glyph} {word}"), 0.0, fmt(FontId::proportional(12.5), ink)),
        None => {
            let word = match &s.state {
                State::Other(word) => word.as_str(),
                _ => "",
            };
            job.append(word, 0.0, fmt(FontId::proportional(12.5), c.fg));
        }
    }
    if let Some(at) = since(s) {
        job.append(&app.tr("wt.since").replace("{}", &at), 6.0, fmt(FontId::proportional(11.5), c.dim));
    }
    if sessions.len() > 1 {
        job.append(&app.tr("claude.more").replace("{}", &(sessions.len() - 1).to_string()), 6.0, fmt(FontId::proportional(11.5), c.dim));
    }
    let chip = ui.fonts_mut(|f| f.layout_job(job));
    let mut out = vec![Piece::Text { g: chip, gap: 0.0, indent: 0.0, bold: true }];
    let note = s.note.trim();
    if !note.is_empty() {
        // Two lines at most, about 34 characters wide.
        let wrap = TextWrapping { max_width: w.min(34.0 * 7.0), max_rows: 2, break_anywhere: true, overflow_character: Some('…') };
        let mut job = LayoutJob { wrap, ..Default::default() };
        let ink = if s.state == State::Waiting { c.wait } else { c.dim };
        job.append(&note.replace(['\r', '\n'], " "), 0.0, fmt(FontId::proportional(12.0), ink));
        out.push(text(ui.fonts_mut(|f| f.layout_job(job)), 3.0));
    }
    let title = if s.title.is_empty() { &s.command } else { &s.title };
    let go = app.tr("wt.go").replace("{tab}", &s.id.to_string()).replace("{title}", title);
    let go = cut_end(ui, &go, FontId::proportional(11.5), w - 30.0);
    let button = Button {
        label: layout(ui, go, FontId::proportional(11.5), c.fg, f32::INFINITY),
        key: Some(layout(ui, "o", FontId::monospace(10.5), c.dim, f32::INFINITY)),
        act: Act::Key('o'),
        danger: false,
    };
    out.push(Piece::Buttons { gap: 5.0, buttons: vec![button] });
    out
}

/// `text` cut at the end to fit `width`, with `…` for what is cut.
fn cut_end(ui: &Ui, text: &str, font: FontId, width: f32) -> String {
    let g = ui.fonts_mut(|f| f.layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE));
    if g.size().x <= width {
        return text.to_owned();
    }
    let dots = ui.fonts_mut(|f| f.layout_no_wrap("…".to_owned(), font, Color32::WHITE)).size().x;
    let Some(row) = g.rows.first() else { return String::new() };
    let keep = row.glyphs.iter().take_while(|gl| gl.pos.x + gl.advance_width + dots <= width).count();
    let kept: String = text.chars().take(keep).collect();
    format!("{kept}…")
}

/// What the branch's upstream says, under the counts against the trunk.
fn upstream_note(app: &App, t: &Tree) -> Option<(String, bool)> {
    t.wt.branch.as_ref()?;
    let Some(u) = &t.upstream else { return Some((app.tr("wt.up.none").to_owned(), false)) };
    let n = |v: u32| v.to_string();
    Some(match (u.gone, u.ahead, u.behind) {
        (true, _, _) => (app.tr("wt.up.gone").replace("{}", &u.name), true),
        (false, 0, 0) => (app.tr("wt.up.same").replace("{}", &u.name), false),
        (false, a, 0) => (app.tr("wt.up.ahead").replace("{up}", &u.name).replace("{n}", &n(a)), false),
        (false, 0, b) => (app.tr("wt.up.behind").replace("{up}", &u.name).replace("{n}", &n(b)), true),
        (false, a, b) => (app.tr("wt.up.both").replace("{up}", &u.name).replace("{a}", &n(a)).replace("{b}", &n(b)), true),
    })
}

/// `↑ahead ↓behind` against the trunk, and how the branch stands against its
/// upstream.
fn trunk_cell(ui: &Ui, app: &App, t: &Tree, w: f32) -> Vec<Piece> {
    let c = ito_theme::colors();
    let mono = FontId::monospace(12.0);
    let mut job = LayoutJob::default();
    match t.ahead_behind {
        Some((a, b)) => {
            job.append(&format!("↑{a}"), 0.0, fmt(mono.clone(), c.done));
            job.append(&format!("↓{b}"), 8.0, fmt(mono, c.wait));
        }
        None => job.append("—", 0.0, fmt(mono, c.dim)),
    }
    let mut out = vec![text(ui.fonts_mut(|f| f.layout_job(job)), 0.0)];
    if let Some((note, warn)) = upstream_note(app, t) {
        out.push(text(layout(ui, note, FontId::proportional(11.5), if warn { c.wait } else { c.dim }, w), 3.0));
    }
    out
}

/// How much is not committed and when it was last written; the untracked
/// folders too big to count.
fn wip_cell(ui: &Ui, app: &App, t: &Tree, w: f32) -> Vec<Piece> {
    let c = ito_theme::colors();
    let wip = app.wip_in(&t.wt.path);
    let mut out = Vec::new();
    match wip.filter(|w| !w.files.is_empty()) {
        Some(wip) => {
            let (a, d) = wip.counts();
            let mut job = LayoutJob::default();
            job.wrap.max_width = w;
            job.append(&app.tr("wt.files").replace("{}", &wip.files.len().to_string()), 0.0, fmt(FontId::proportional(12.5), c.fg));
            job.append(&format!("+{a}"), 6.0, fmt(FontId::monospace(11.5), c.done));
            job.append(&format!("−{d}"), 5.0, fmt(FontId::monospace(11.5), c.err));
            out.push(text(ui.fonts_mut(|f| f.layout_job(job)), 0.0));
            if let Some(t) = wip.written {
                let at = crate::graph::when(t, &chrono::Local::now(), app.tr("time.yesterday"));
                out.push(text(layout(ui, app.tr("wt.written").replace("{}", &at), FontId::proportional(11.5), c.dim, w), 3.0));
            }
        }
        None => out.push(text(layout(ui, app.tr("wt.none"), FontId::proportional(12.5), c.dim, w), 0.0)),
    }
    if let Some(wip) = wip.filter(|w| !w.left_out.is_empty()) {
        let shown = wip.left_out.len().min(2);
        let mut paths = wip.left_out[..shown].iter().map(|l| l.path.as_str()).collect::<Vec<_>>().join(", ");
        if shown < wip.left_out.len() {
            paths.push_str(&app.tr("wip.left_more").replace("{n}", &(wip.left_out.len() - shown).to_string()));
        }
        out.push(text(layout(ui, app.tr("wt.left_out").replace("{}", &paths), FontId::proportional(11.5), c.dim, w), 3.0));
    }
    out
}

/// How many commits the worktree has gained that are not seen yet; `—` for
/// one not followed (the trunk's).
fn unseen_cell(ui: &Ui, app: &App, t: &Tree, w: f32) -> Vec<Piece> {
    let c = ito_theme::colors();
    let font = FontId::proportional(12.5);
    match app.unseen.of(&t.wt.path) {
        None => vec![text(layout(ui, "—", font, c.dim, w), 0.0)],
        Some([]) => vec![text(layout(ui, app.tr("wt.none"), font, c.dim, w), 0.0)],
        Some(news) => {
            let words = layout(ui, app.tr("wt.unseen").replace("{}", &news.len().to_string()), font, c.fg, w - 11.0);
            vec![Piece::Dot { g: words }]
        }
    }
}

/// Where the worktree's branch stands in review: the stage in its colour
/// and how many of its files are checked; `—` for one with nothing to review.
fn review_cell(ui: &Ui, app: &App, t: &Tree, w: f32) -> Vec<Piece> {
    let c = ito_theme::colors();
    let font = FontId::proportional(12.5);
    let Some(row) = app.reviews.of(&t.wt.path) else { return vec![text(layout(ui, "—", font, c.dim, w), 0.0)] };
    let (key, color) = match row.stage {
        Stage::Unreviewed => ("wt.review.unreviewed", c.dim),
        Stage::InReview => ("wt.review.in_review", c.blue),
        Stage::Flagged => ("wt.review.flagged", c.wait),
        Stage::Approved => ("wt.review.approved", c.done),
    };
    let head = layout(ui, app.tr(key), font.clone(), color, w);
    let count = app.tr("wt.review.count").replace("{n}", &row.checked.to_string()).replace("{m}", &row.total.to_string());
    vec![text(head, 0.0), text(layout(ui, count, FontId::proportional(11.5), c.dim, w), 1.0)]
}

/// What the worktree would conflict with: each side, in how many places,
/// in which files; yellow where only its uncommitted change does.
fn clash_cell(ui: &Ui, app: &App, t: &Tree, w: f32) -> Vec<Piece> {
    let c = ito_theme::colors();
    if app.merging() || app.clashes.side_of(&t.wt.path).is_none() {
        return vec![text(layout(ui, "—", FontId::proportional(12.5), c.dim, w), 0.0)];
    }
    let mut out = Vec::new();
    for (side, o) in app.clashes_in(&t.wt.path) {
        let Some(cf) = o.conflict() else { continue };
        let wip = matches!(o, Outcome::Uncommitted(_));
        let words = app.tr("wt.clash").replace("{b}", &side.name).replace("{n}", &cf.places().to_string());
        let ink = if wip { c.wait } else { c.err };
        let gap = if out.is_empty() { 0.0 } else { 5.0 };
        out.push(Piece::Text { g: layout(ui, words, FontId::proportional(12.5), ink, w), gap, indent: 0.0, bold: true });
        let mut files = crate::graph_ui::leaves(app, cf);
        if wip {
            files = app.tr("wt.clash_wip").replace("{}", &files);
        }
        out.push(text(layout(ui, files, FontId::proportional(11.5), c.dim, w), 2.0));
    }
    for (a, b) in &app.resolved {
        let name = &t.wt.branch;
        let other = if name.as_deref() == Some(a) {
            b
        } else if name.as_deref() == Some(b) {
            a
        } else {
            continue;
        };
        let words = app.tr("wt.resolved").replace("{b}", other);
        let gap = if out.is_empty() { 0.0 } else { 5.0 };
        out.push(Piece::Text { g: layout(ui, words, FontId::proportional(12.5), c.done, w), gap, indent: 0.0, bold: true });
    }
    if out.is_empty() {
        out.push(text(layout(ui, app.tr("wt.none"), FontId::proportional(12.5), c.dim, w), 0.0));
    }
    out
}

/// The row's buttons: the trunk's pulls and opens the graph; the others
/// show their diff, take in the trunk, open a PR or go.
fn buttons(ui: &Ui, app: &App, t: &Tree, k: usize, trunk: &str) -> Vec<Button> {
    let c = ito_theme::colors();
    let b = |label: String, key: Option<&str>, act, danger| Button {
        label: layout(ui, label, FontId::proportional(11.5), c.fg, f32::INFINITY),
        key: key.map(|k| layout(ui, k, FontId::monospace(10.5), c.dim, f32::INFINITY)),
        act,
        danger,
    };
    let mut out = buttons_of(app, t, k, trunk, &b);
    out.extend([
        b(app.tr("open.terminal").to_owned(), Some("t"), Act::Key('t'), false),
        b(app.tr("open.editor").to_owned(), Some("e"), Act::Key('e'), false),
        b(app.tr("open.explorer").to_owned(), Some("E"), Act::Key('E'), false),
    ]);
    out
}

type MakeButton<'a> = dyn Fn(String, Option<&str>, Act, bool) -> Button + 'a;

fn buttons_of(app: &App, t: &Tree, k: usize, trunk: &str, b: &MakeButton) -> Vec<Button> {
    if t.wt.branch.as_deref() == Some(trunk) {
        return vec![
            b(app.tr("key.pull").to_owned(), Some("p"), Act::Key('p'), false),
            b(app.tr("key.in_graph").to_owned(), Some("Enter"), Act::Enter, false),
        ];
    }
    let mut out = Vec::new();
    // Merged elsewhere and clean: removing it comes first.
    let first = k > 0 && removable(app, t);
    if first {
        out.push(b(app.tr("key.remove").to_owned(), Some("g x"), Act::Go('x'), true));
    }
    out.extend([
        b(app.tr("wt.diff").to_owned(), Some("d"), Act::Key('d'), false),
        b(app.tr("wt.take").replace("{}", trunk), Some("m"), Act::Key('m'), false),
    ]);
    if t.wt.branch.as_deref().is_some_and(|br| !app.branch_issues(br).is_empty()) {
        out.push(b(app.tr("key.issue").to_owned(), Some("I"), Act::Key('I'), false));
    }
    if t.wt.branch.is_some() {
        out.push(b("PR".to_owned(), None, Act::Pr, false));
    }
    // The main worktree is not one git can remove.
    if k > 0 && !first {
        out.push(b(app.tr("key.remove").to_owned(), Some("g x"), Act::Go('x'), true));
    }
    out
}

/// A worktree's place on the screen: a table row or a card, as lines of
/// cells, each line's top from the block's top.
struct Block {
    lines: Vec<(f32, Vec<Col>)>,
    height: f32,
    card: bool,
}

/// The table's columns: `(left, width)` of each, from the content's left,
/// with no Claude column when tsumugi is not running.
fn columns(width: f32, claude: bool, sizes: &Sizes) -> Vec<(f32, f32)> {
    let fixed = sizes.table;
    let sum: f32 = fixed.iter().sum();
    let least = if claude { REST_MIN_CLAUDE } else { REST_MIN };
    // In a window just over the cards' width, the fixed columns give too.
    let scale = ((width - least) / sum).clamp(0.6, 1.0);
    let rest = (width - sum * scale).max(120.0);
    let mut widths: Vec<f32> = if claude { vec![rest * sizes.name_share, rest * (1.0 - sizes.name_share)] } else { vec![rest] };
    widths.extend(fixed.map(|w| w * scale));
    let mut x = 0.0;
    widths
        .into_iter()
        .map(|w| {
            let at = (x, w);
            x += w;
            at
        })
        .collect()
}

fn row_block(ui: &Ui, app: &App, t: &Tree, k: usize, trunk: &str, cols: &[(f32, f32)], claude: bool) -> Block {
    let cell = |n: usize| (cols[n].0 + CELL_X, cols[n].1 - 2.0 * CELL_X);
    let mut out = Vec::new();
    let (x, w) = cell(0);
    out.push(Col { x, w, pieces: name_cell(ui, app, t, w) });
    let mut n = 1;
    if claude {
        let (x, w) = cell(1);
        out.push(Col { x, w, pieces: claude_cell(ui, app, t, w) });
        n = 2;
    }
    let (x, w) = cell(n);
    out.push(Col { x, w, pieces: trunk_cell(ui, app, t, w) });
    let (x, w) = cell(n + 1);
    out.push(Col { x, w, pieces: wip_cell(ui, app, t, w) });
    let (x, w) = cell(n + 2);
    out.push(Col { x, w, pieces: unseen_cell(ui, app, t, w) });
    let (x, w) = cell(n + 3);
    out.push(Col { x, w, pieces: review_cell(ui, app, t, w) });
    let (x, w) = cell(n + 4);
    out.push(Col { x, w, pieces: clash_cell(ui, app, t, w) });
    let (x, w) = cell(n + 5);
    out.push(Col { x, w, pieces: vec![Piece::Buttons { gap: 0.0, buttons: buttons(ui, app, t, k, trunk) }] });
    let height = line_h(&out) + 2.0 * CELL_Y;
    Block { lines: vec![(CELL_Y, out)], height, card: false }
}

fn card_block(ui: &Ui, app: &App, t: &Tree, k: usize, trunk: &str, width: f32, claude: bool) -> Block {
    let c = ito_theme::colors();
    let (pad_x, pad_y, gap) = (12.0, 10.0, 8.0);
    let inner = width - 2.0 * pad_x;
    let mut lines = Vec::new();
    let mut y = pad_y;
    let mut push = |cols: Vec<Col>, y: &mut f32| {
        let h = line_h(&cols);
        lines.push((*y, cols));
        *y += h + gap;
    };
    if claude {
        let left = (inner - 16.0) * 0.55;
        let right = inner - 16.0 - left;
        push(
            vec![
                Col { x: pad_x, w: left, pieces: name_cell(ui, app, t, left) },
                Col { x: pad_x + left + 16.0, w: right, pieces: claude_cell(ui, app, t, right) },
            ],
            &mut y,
        );
    } else {
        push(vec![Col { x: pad_x, w: inner, pieces: name_cell(ui, app, t, inner) }], &mut y);
    }
    let value_w = inner - LABEL_W;
    let label = |word: String| text(layout(ui, word, FontId::proportional(12.0), c.dim, LABEL_W - 8.0), 0.0);
    let kv = |word: String, pieces| {
        vec![Col { x: pad_x, w: LABEL_W - 8.0, pieces: vec![label(word)] }, Col { x: pad_x + LABEL_W, w: value_w, pieces }]
    };
    push(kv(app.tr("wt.col.trunk").replace("{}", trunk), trunk_cell(ui, app, t, value_w)), &mut y);
    push(kv(app.tr("wt.col.wip").to_owned(), wip_cell(ui, app, t, value_w)), &mut y);
    push(kv(app.tr("wt.col.unseen").to_owned(), unseen_cell(ui, app, t, value_w)), &mut y);
    push(kv(app.tr("wt.col.review").to_owned(), review_cell(ui, app, t, value_w)), &mut y);
    push(kv(app.tr("wt.col.clash").to_owned(), clash_cell(ui, app, t, value_w)), &mut y);
    push(vec![Col { x: pad_x, w: inner, pieces: vec![Piece::Buttons { gap: 0.0, buttons: buttons(ui, app, t, k, trunk) }] }], &mut y);
    Block { lines, height: y - gap + pad_y, card: true }
}

/// The cell's pieces from `top`; a click on a button is put in `act`.
fn draw_col(ui: &Ui, p: &Painter, col: &Col, at: Pos2, id: Id, live: bool, act: &mut Option<Act>) {
    let c = ito_theme::colors();
    let mut y = at.y;
    let x = at.x + col.x;
    for (n, piece) in col.pieces.iter().enumerate() {
        match piece {
            Piece::Text { g, gap, indent, bold } => {
                y += gap;
                p.galley(pos2(x + indent, y), g.clone(), c.fg);
                if *bold {
                    p.galley(pos2(x + indent + 0.45, y), g.clone(), c.fg);
                }
                y += g.size().y;
            }
            Piece::Name { g, color } => {
                p.circle_filled(pos2(x + 4.5, y + g.size().y / 2.0), 4.5, *color);
                for dx in [0.0, 0.45] {
                    p.galley(pos2(x + INDENT + dx, y), g.clone(), c.fg);
                }
                y += g.size().y;
            }
            Piece::Dot { g } => {
                p.circle_filled(pos2(x + 3.0, y + g.size().y / 2.0), 3.0, c.run);
                p.galley(pos2(x + 11.0, y), g.clone(), c.fg);
                y += g.size().y;
            }
            Piece::Buttons { gap, buttons } => {
                y += gap;
                let places = flow(buttons, col.w);
                for (i, (b, r)) in buttons.iter().zip(&places).enumerate() {
                    let r = r.translate(vec2(x, y));
                    let resp = ui.interact(r.intersect(p.clip_rect()), id.with((n, i)), Sense::CLICK);
                    let hot = live && resp.hovered();
                    let ink = if hot && b.danger { c.err } else { c.fg };
                    p.rect_filled(r, CornerRadius::same(4), c.panel);
                    let edge = match (hot, b.danger) {
                        (true, true) => c.err,
                        (true, false) => c.run,
                        _ => c.border,
                    };
                    p.rect_stroke(r, CornerRadius::same(4), Stroke::new(1.0, edge), StrokeKind::Inside);
                    let mid = r.center().y;
                    p.galley(pos2(r.left() + 7.0, mid - b.label.size().y / 2.0), b.label.clone(), ink);
                    if let Some(k) = &b.key {
                        p.galley(pos2(r.left() + 7.0 + b.label.size().x + 4.0, mid - k.size().y / 2.0), k.clone(), c.dim);
                    }
                    if hot {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                    if live && resp.clicked() {
                        *act = Some(b.act);
                    }
                }
                y += places.last().map_or(0.0, |r| r.bottom());
            }
        }
    }
}

/// The table of pairs, laid out from its box's top left.
struct Pairs {
    w: f32,
    h: f32,
    title: Arc<Galley>,
    /// The sides' short names across the top and down the left, cut to fit.
    tops: Vec<Arc<Galley>>,
    lefts: Vec<Arc<Galley>>,
    head_w: f32,
    cell_w: f32,
    /// The pair picked, its sides in the table's order.
    picked: Option<(usize, usize)>,
    note: Arc<Galley>,
    foot: Arc<Galley>,
}

impl Pairs {
    /// The table's top, from the box's.
    fn grid_top(&self) -> f32 {
        BOX_PAD + self.title.size().y + 10.0
    }

    fn grid_h(&self) -> f32 {
        if self.tops.len() < 2 {
            0.0
        } else {
            (self.tops.len() + 1) as f32 * MX_H + 10.0
        }
    }
}

/// A side's name without what is before its last `/`.
fn short(name: &str) -> &str {
    name.rsplit('/').next().filter(|s| !s.is_empty()).unwrap_or(name)
}

fn pairs_box(ui: &Ui, app: &App, w: f32) -> Pairs {
    let c = ito_theme::colors();
    let inner = w - 2.0 * BOX_PAD;
    let cl = &app.clashes;
    let n = cl.sides.len();
    let font = FontId::proportional(11.5);
    let title = layout(ui, app.tr("mx.title"), FontId::proportional(12.5), c.fg, inner);
    let wide = |s: &str| ui.fonts_mut(|f| f.layout_no_wrap(s.to_owned(), font.clone(), c.dim)).size().x;
    let widest = cl.sides.iter().map(|s| wide(short(&s.name))).fold(0.0, f32::max);
    let head_w = widest.min(140.0) + 20.0;
    let cell_w = if n == 0 { 0.0 } else { (widest + 20.0).clamp(40.0, 140.0).min(((inner - head_w) / n as f32).max(30.0)) };
    let cut = |s: &str, room: f32| layout(ui, cut_end(ui, short(s), font.clone(), room), font.clone(), c.dim, f32::INFINITY);
    let tops = cl.sides.iter().map(|s| cut(&s.name, cell_w - 12.0)).collect();
    let lefts = cl.sides.iter().map(|s| cut(&s.name, head_w - 20.0)).collect();
    let picked = ui.ctx().data(|d| d.get_temp::<(String, String)>(picked_id())).and_then(|(a, b)| {
        let i = cl.sides.iter().position(|s| s.name == a)?;
        let j = cl.sides.iter().position(|s| s.name == b)?;
        cl.between(i, j)?.conflict()?;
        (!app.merging()).then_some((i, j))
    });
    let note = match picked.and_then(|(i, j)| Some((i, j, cl.between(i, j)?))) {
        _ if n < 2 => app.tr("mx.alone").to_owned(),
        _ if app.merging() => app.tr("mx.merging").to_owned(),
        Some((i, j, o)) => {
            let places: Vec<String> = o
                .conflict()
                .map(|cf| &cf.files[..])
                .unwrap_or_default()
                .iter()
                .map(|f| app.tr("mx.place").replace("{path}", &f.path).replace("{n}", &f.hunks.unwrap_or(1).to_string()))
                .collect();
            let mut t = app
                .tr("mx.pair")
                .replace("{a}", &cl.sides[i].name)
                .replace("{b}", &cl.sides[j].name)
                .replace("{files}", &places.join(app.tr("mx.sep")));
            let trunk = app.trees.trunk.as_deref().unwrap_or("main");
            t.push_str(&match o {
                Outcome::Uncommitted(_) => app.tr("mx.uncommitted").to_owned(),
                _ => app.tr("mx.committed").replace("{}", trunk),
            });
            t
        }
        None => app.tr("mx.hint").to_owned(),
    };
    let note = layout(ui, note, FontId::proportional(11.5), if picked.is_some() { c.fg } else { c.dim }, inner);
    let foot = layout(ui, app.tr("mx.foot"), FontId::proportional(11.0), c.dim, inner);
    let mut b = Pairs { w, h: 0.0, title, tops, lefts, head_w, cell_w, picked, note, foot };
    b.h = b.grid_top() + b.grid_h() + b.note.size().y + 8.0 + b.foot.size().y + BOX_PAD;
    b
}

/// The table of pairs in its box from `at`; the pair clicked, if one was.
fn draw_pairs(ui: &Ui, p: &Painter, app: &App, b: &Pairs, at: Pos2, live: bool) -> Option<(usize, usize)> {
    let c = ito_theme::colors();
    let cl = &app.clashes;
    let frame = Rect::from_min_size(at, vec2(b.w, b.h));
    p.rect_stroke(frame, CornerRadius::same(8), Stroke::new(1.0, c.border), StrokeKind::Inside);
    let x0 = at.x + BOX_PAD;
    for dx in [0.0, 0.45] {
        p.galley(pos2(x0 + dx, at.y + BOX_PAD), b.title.clone(), c.fg);
    }
    let mut clicked = None;
    if b.tops.len() >= 2 {
        // A side already merged into another worktree's branch is drawn faint.
        let merged: Vec<bool> = cl
            .sides
            .iter()
            .map(|s| app.trees.trees.iter().any(|t| t.merged_into.is_some() && t.wt.branch.as_deref() == Some(s.name.as_str())))
            .collect();
        let top = at.y + b.grid_top();
        let edge = Stroke::new(1.0, c.border);
        let cell = |row: usize, col: usize| {
            let x = if col == 0 { x0 } else { x0 + b.head_w + (col - 1) as f32 * b.cell_w };
            let w = if col == 0 { b.head_w } else { b.cell_w };
            Rect::from_min_size(pos2(x, top + row as f32 * MX_H), vec2(w, MX_H))
        };
        p.rect_stroke(cell(0, 0), 0.0, edge, StrokeKind::Middle);
        for (j, g) in b.tops.iter().enumerate() {
            let r = cell(0, j + 1);
            p.rect_stroke(r, 0.0, edge, StrokeKind::Middle);
            p.galley(r.center() - g.size() / 2.0, g.clone(), if merged[j] { c.dim.gamma_multiply(0.5) } else { c.dim });
        }
        for (i, g) in b.lefts.iter().enumerate() {
            let r = cell(i + 1, 0);
            p.rect_stroke(r, 0.0, edge, StrokeKind::Middle);
            p.galley(
                pos2(r.right() - 10.0 - g.size().x, r.center().y - g.size().y / 2.0),
                g.clone(),
                if merged[i] { c.dim.gamma_multiply(0.5) } else { c.dim },
            );
            for j in 0..b.tops.len() {
                let r = cell(i + 1, j + 1);
                let mid = r.center();
                let picked = b.picked.is_some_and(|q| q == (i, j) || q == (j, i));
                let faint = merged[i] || merged[j];
                let fade = |ink: Color32| if faint { ink.gamma_multiply(0.45) } else { ink };
                let dim = |s: &str| {
                    p.text(mid, Align2::CENTER_CENTER, s, FontId::proportional(12.0), fade(c.dim));
                };
                match cl.between(i, j).map(|o| if app.merging() && o.conflict().is_some() { &Outcome::Clean } else { o }) {
                    None => {
                        p.rect_filled(r, 0.0, c.fg.gamma_multiply(0.03));
                        dim("—");
                    }
                    Some(Outcome::Clean) if app.resolved.contains(&michi_core::clash::pair(&cl.sides[i].name, &cl.sides[j].name)) => {
                        p.text(mid, Align2::CENTER_CENTER, "✓", FontId::proportional(12.0), fade(c.done));
                    }
                    Some(Outcome::Clean) => dim("·"),
                    Some(Outcome::Unknown) => dim("?"),
                    Some(o) => {
                        let Some(cf) = o.conflict() else { continue };
                        let wip = matches!(o, Outcome::Uncommitted(_));
                        let ink = fade(if wip { c.wait } else { c.err });
                        p.rect_filled(r, 0.0, ink.gamma_multiply(if wip { 0.08 } else { 0.1 }));
                        if picked {
                            p.rect_stroke(r.shrink(2.0), 0.0, Stroke::new(1.5, ink), StrokeKind::Inside);
                        } else if wip {
                            let q = r.shrink(3.0);
                            let corners = [q.left_top(), q.right_top(), q.right_bottom(), q.left_bottom(), q.left_top()];
                            p.extend(egui::Shape::dashed_line(&corners, Stroke::new(1.0, ink.gamma_multiply(0.7)), 3.0, 2.0));
                        }
                        let n = cf.places().to_string();
                        for dx in [0.0, 0.45] {
                            p.text(mid + vec2(dx, 0.0), Align2::CENTER_CENTER, &n, FontId::proportional(12.0), ink);
                        }
                        let resp = ui.interact(r.intersect(p.clip_rect()), Id::new(("wt-pair", i, j)), Sense::CLICK);
                        if live && resp.hovered() {
                            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                        }
                        if live && resp.clicked() {
                            clicked = Some((i, j));
                        }
                        if live {
                            let mut tip: Vec<String> = if wip { vec![app.tr("mx.tip_wip").to_owned()] } else { Vec::new() };
                            tip.extend(cf.files.iter().map(|f| format!("{} {}", f.path, f.hunks.unwrap_or(1))));
                            resp.on_hover_text(tip.join("\n"));
                        }
                    }
                }
                p.rect_stroke(r, 0.0, edge, StrokeKind::Middle);
            }
        }
    }
    let y = at.y + b.grid_top() + b.grid_h();
    p.galley(pos2(x0, y), b.note.clone(), c.dim);
    p.galley(pos2(x0, y + b.note.size().y + 8.0), b.foot.clone(), c.dim);
    clicked
}

/// A line of the list of unseen commits: the commit, its short hash, its
/// subject and where and when.
type NewLine = (Oid, Arc<Galley>, Arc<Galley>, Arc<Galley>);

/// The list of the commits not seen yet, laid out from its box's top left.
struct News {
    w: f32,
    h: f32,
    title: Arc<Galley>,
    /// Each line: the commit, its short hash, its subject and where and when.
    lines: Vec<NewLine>,
    /// `全部見ました。`, or how many more there are than shown.
    rest: Option<Arc<Galley>>,
    foot: Arc<Galley>,
}

impl News {
    fn lines_top(&self) -> f32 {
        BOX_PAD + self.title.size().y + 8.0
    }
}

fn news_box(ui: &Ui, app: &App, w: f32) -> News {
    let c = ito_theme::colors();
    let inner = w - 2.0 * BOX_PAD;
    let all = app.unseen.all();
    let title = layout(ui, app.tr("wt.unseen_title").replace("{}", &all.len().to_string()), FontId::proportional(12.5), c.fg, inner);
    let now = chrono::Local::now();
    let lines: Vec<_> = all
        .iter()
        .take(NEW_MOST)
        .map(|(name, n)| {
            let hash = layout(ui, n.id.short(), FontId::monospace(11.5), c.dim, f32::INFINITY);
            let at = crate::graph::when(n.time, &now, app.tr("time.yesterday"));
            let meta = format!("{} · {at}", short(name));
            let meta =
                layout(ui, cut_end(ui, &meta, FontId::proportional(11.5), inner * 0.4), FontId::proportional(11.5), c.dim, f32::INFINITY);
            let room = inner - 12.0 - 62.0 - 8.0 - 11.0 - 8.0 - meta.size().x;
            let subject =
                layout(ui, cut_end(ui, &n.subject, FontId::proportional(12.5), room), FontId::proportional(12.5), c.fg, f32::INFINITY);
            (n.id, hash, subject, meta)
        })
        .collect();
    let rest = match all.len() {
        0 => Some(app.tr("new.none").to_owned()),
        n if n > NEW_MOST => Some(app.tr("new.more").replace("{}", &(n - NEW_MOST).to_string())),
        _ => None,
    }
    .map(|words| layout(ui, words, FontId::proportional(12.0), c.dim, inner));
    let foot = layout(ui, app.tr("new.foot"), FontId::proportional(11.0), c.dim, inner);
    let mut b = News { w, h: 0.0, title, lines, rest, foot };
    b.h =
        b.lines_top() + b.lines.len() as f32 * NEW_H + b.rest.as_ref().map_or(0.0, |g| g.size().y + 4.0) + 8.0 + b.foot.size().y + BOX_PAD;
    b
}

/// The list of unseen commits in its box from `at`; the commit clicked, if
/// one was.
fn draw_news(ui: &Ui, p: &Painter, b: &News, at: Pos2, live: bool) -> Option<Oid> {
    let c = ito_theme::colors();
    let frame = Rect::from_min_size(at, vec2(b.w, b.h));
    p.rect_stroke(frame, CornerRadius::same(8), Stroke::new(1.0, c.border), StrokeKind::Inside);
    let x0 = at.x + BOX_PAD;
    for dx in [0.0, 0.45] {
        p.galley(pos2(x0 + dx, at.y + BOX_PAD), b.title.clone(), c.fg);
    }
    let mut clicked = None;
    let mut y = at.y + b.lines_top();
    for (id, hash, subject, meta) in &b.lines {
        let r = Rect::from_min_size(pos2(x0, y), vec2(b.w - 2.0 * BOX_PAD, NEW_H));
        let resp = ui.interact(r.intersect(p.clip_rect()), Id::new(("wt-new", *id)), Sense::CLICK);
        if live && resp.hovered() {
            p.rect_filled(r, CornerRadius::same(4), c.fg.gamma_multiply(0.04));
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        if live && resp.clicked() {
            clicked = Some(*id);
        }
        let mid = r.center().y;
        let mut x = r.left() + 6.0;
        p.galley(pos2(x, mid - hash.size().y / 2.0), hash.clone(), c.dim);
        x += 62.0 + 8.0;
        p.circle_filled(pos2(x + 3.0, mid), 3.0, c.run);
        x += 11.0;
        for dx in [0.0, 0.45] {
            p.galley(pos2(x + dx, mid - subject.size().y / 2.0), subject.clone(), c.fg);
        }
        p.galley(pos2(r.right() - 6.0 - meta.size().x, mid - meta.size().y / 2.0), meta.clone(), c.dim);
        y += NEW_H;
    }
    if let Some(g) = &b.rest {
        y += 4.0;
        p.galley(pos2(x0, y), g.clone(), c.dim);
        y += g.size().y;
    }
    p.galley(pos2(x0, y + 8.0), b.foot.clone(), c.dim);
    clicked
}

/// A button of the top bar, its right edge at `right`; its left edge, and
/// whether it was clicked.
#[expect(clippy::too_many_arguments, reason = "the painter, the place and the look are the frame's")]
fn top_button(ui: &Ui, p: &Painter, right: f32, mid: f32, icon: Icon, label: &str, key: Option<&str>, live: bool) -> (f32, bool) {
    let c = ito_theme::colors();
    let prime = key.is_some();
    let ink = if prime { c.run } else { c.fg };
    let g = layout(ui, label, FontId::proportional(12.5), ink, f32::INFINITY);
    let k = key.map(|k| layout(ui, k, FontId::monospace(10.5), c.dim, f32::INFINITY));
    let w = 8.0 + 15.0 + g.size().x + k.as_ref().map_or(0.0, |k| 5.0 + k.size().x) + 8.0;
    let b = Rect::from_min_max(pos2(right - w, mid - 13.0), pos2(right, mid + 13.0));
    let resp = ui.interact(b.intersect(p.clip_rect()), Id::new(("wt-top", label)), Sense::CLICK);
    let hot = live && resp.hovered();
    if hot {
        p.rect_filled(b, CornerRadius::same(5), c.panel);
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    let edge = if prime { c.run.gamma_multiply(0.45) } else { c.border };
    p.rect_stroke(b, CornerRadius::same(5), Stroke::new(1.0, edge), StrokeKind::Inside);
    paint_icon(p, pos2(b.left() + 13.0, mid), icon, ink);
    let mut x = b.left() + 23.0;
    p.galley(pos2(x, mid - g.size().y / 2.0), g.clone(), ink);
    x += g.size().x + 5.0;
    if let Some(k) = k {
        p.galley(pos2(x, mid - k.size().y / 2.0), k, c.dim);
    }
    (b.left(), live && resp.clicked())
}

/// The worktree view in `r`.
pub fn show(ui: &mut Ui, app: &mut App, r: Rect, live: bool) {
    let c = ito_theme::colors();
    let ctx = ui.ctx().clone();
    let p = ui.painter_at(r);
    p.rect_filled(r, 0.0, c.bg);
    if app.trees.trees.is_empty() {
        let word = match &app.repo {
            Repo::Failed(e) => failed_word(e),
            _ => "wt.reading",
        };
        p.text(r.center(), Align2::CENTER_CENTER, app.tr(word), FontId::proportional(13.0), c.dim);
        return;
    }
    let left = r.left() + PAD_X;
    let width = (r.width() - 2.0 * PAD_X).max(200.0);
    let cards = r.width() < CARDS;
    let claude = app.claude.is_some();
    let trunk = app.trees.trunk.clone().unwrap_or_else(|| "main".to_owned());

    // Everything is laid out from the content's top, then drawn scrolled.
    let cols = columns(width, claude, &app.sizes);
    let blocks: Vec<Block> = app
        .trees
        .trees
        .iter()
        .enumerate()
        .map(
            |(k, t)| if cards { card_block(ui, app, t, k, &trunk, width, claude) } else { row_block(ui, app, t, k, &trunk, &cols, claude) },
        )
        .collect();
    let gap = if cards { 10.0 } else { 0.0 };
    // What a worktree is (#122): a card over the table until it is dismissed.
    let guide = app.prefs.guides.shown(Topic::Worktree);
    let guide_h = if guide { intro_ui::worktree_h(ui, app, width) + 8.0 } else { 0.0 };
    let table_top = PAD_TOP + TOP_H + 14.0 + guide_h;
    let rows_top = table_top + if cards { 0.0 } else { HEAD_H };
    let mut tops = Vec::with_capacity(blocks.len());
    let mut y = rows_top;
    for b in &blocks {
        tops.push(y);
        y += b.height + gap;
    }
    let pairs_top = y + 14.0;
    // The table of pairs on the left, the unseen commits beside it; in a
    // narrow window, one under the other.
    let pairs_w = if cards { width } else { ((width - BOX_GAP) / 2.0).max(420.0).min(width) };
    let pairs = pairs_box(ui, app, pairs_w);
    let beside = !cards && width - pairs_w - BOX_GAP >= NEW_MIN;
    let news = news_box(ui, app, if beside { width - pairs_w - BOX_GAP } else { width });
    let news_at = if beside { (pairs_w + BOX_GAP, pairs_top) } else { (0.0, pairs_top + pairs.h + 14.0) };
    let boxes_bottom = (pairs_top + pairs.h).max(news_at.1 + news.h);
    let foot_w = width.min(900.0);
    let feet: Vec<Arc<Galley>> =
        if claude { vec![layout(ui, app.tr("wt.foot_claude"), FontId::proportional(11.5), c.dim, foot_w)] } else { Vec::new() };
    // One line pointing at the next step: one worktree, no tsumugi... (#81).
    let next: Option<Arc<Galley>> = app.next_step().map(|s| layout(ui, app.tr(s.key()), FontId::proportional(12.5), c.fg, foot_w));
    let next_h = next.as_ref().map_or(0.0, |g| g.size().y + 14.0);
    let foot_top = boxes_bottom + 14.0 + next_h;
    let total = foot_top + feet.iter().map(|g| g.size().y + 4.0).sum::<f32>() + 16.0;

    let mut scroll = ctx.data(|d| d.get_temp::<f32>(scroll_id())).unwrap_or(0.0);
    if live && ui.rect_contains_pointer(r) {
        scroll -= ui.input(|i| i.smooth_scroll_delta.y);
    }
    // A selection moved by the keys comes into sight.
    if ctx.data(|d| d.get_temp::<usize>(seen_id())) != Some(app.wt_sel) {
        if let (Some(&top), Some(b)) = (tops.get(app.wt_sel), blocks.get(app.wt_sel)) {
            let first = if app.wt_sel == 0 { 0.0 } else { top - if cards { 0.0 } else { HEAD_H } };
            if first < scroll {
                scroll = first;
            } else if top + b.height > scroll + r.height() {
                scroll = top + b.height - r.height();
            }
        }
        ctx.data_mut(|d| d.insert_temp(seen_id(), app.wt_sel));
    }
    scroll = scroll.clamp(0.0, (total - r.height()).max(0.0));
    ctx.data_mut(|d| d.insert_temp(scroll_id(), scroll));
    let at = |y: f32| r.top() + y - scroll;

    // The top bar: the title, how many, and the buttons.
    let mid = at(PAD_TOP + TOP_H / 2.0);
    let title = layout(ui, app.tr("view.worktree"), FontId::proportional(15.0), c.fg, f32::INFINITY);
    for dx in [0.0, 0.5] {
        p.galley(pos2(left + dx, mid - title.size().y / 2.0), title.clone(), c.fg);
    }
    let repo = match &app.repo {
        Repo::Ready(s) => s.name.clone(),
        _ => String::new(),
    };
    let mut count = app.tr("wt.count").replace("{repo}", &repo).replace("{n}", &app.trees.trees.len().to_string());
    if claude {
        let n: usize = app.trees.trees.iter().map(|t| app.claude_in(&t.wt.path).len()).sum();
        count.push_str(&app.tr("wt.claudes").replace("{}", &n.to_string()));
    }
    let (new_left, new) = top_button(ui, &p, left + width, mid, Icon::Plus, app.tr("tool.new_work"), Some("N"), live);
    let (again_left, again) = top_button(ui, &p, new_left - 6.0, mid, Icon::Fetch, app.tr("wt.reread"), None, live);
    let pointer_at = if live { ui.input(|i| i.pointer.hover_pos()).filter(|q| r.contains(*q)) } else { None };
    let mut room_right = again_left - 10.0;
    let mut guide_act = None;
    if !guide {
        let (l, hit) = intro_ui::link(ui, &p, room_right, mid, app.tr("guide.link.worktree"), "worktree", live, pointer_at);
        if hit {
            guide_act = Some(intro_ui::Act::Dismiss);
        }
        room_right = l - 10.0;
    } else {
        let (a, _) = intro_ui::worktree_card(ui, app, &p, pos2(left, at(PAD_TOP + TOP_H + 6.0)), width, live, pointer_at);
        guide_act = a;
    }
    let count_x = left + title.size().x + 10.0;
    let count = cut_end(ui, &count, FontId::proportional(12.0), room_right - count_x);
    p.text(pos2(count_x, mid), Align2::LEFT_CENTER, count, FontId::proportional(12.0), c.dim);

    // The table's header.
    if !cards {
        let top = at(table_top);
        let mut words = vec![app.tr("wt.col.name").to_owned()];
        if claude {
            words.push(app.tr("wt.col.claude").to_owned());
        }
        words.push(app.tr("wt.col.trunk").replace("{}", &trunk));
        words.push(app.tr("wt.col.wip").to_owned());
        words.push(app.tr("wt.col.unseen").to_owned());
        words.push(app.tr("wt.col.review").to_owned());
        words.push(app.tr("wt.col.clash").to_owned());
        for (word, (x, w)) in words.into_iter().zip(&cols) {
            let word = cut_end(ui, &word, FontId::proportional(11.0), w - 2.0 * CELL_X);
            p.text(pos2(left + x + CELL_X, top + HEAD_H / 2.0), Align2::LEFT_CENTER, word, FontId::proportional(11.0), c.dim);
        }
        p.line_segment([pos2(left, top + HEAD_H - 0.5), pos2(left + width, top + HEAD_H - 0.5)], Stroke::new(1.0, c.border));
        // The edge between two columns drags (ito-layout's grips): the left
        // edge of a fixed column widens it as it goes left, the edge after the
        // name trades with the Claude column.
        let base = 1 + usize::from(claude);
        let ns: Vec<usize> = (1..=(base + 5)).collect();
        let edges: Vec<f32> = ns.iter().map(|n| left + cols[*n].0).collect();
        if let Some(g) = ito_layout::ui::grips(ui, Id::new("wt-cols"), &edges, top..top + HEAD_H, 8.0).filter(|_| live) {
            let n = ns[g.index];
            if claude && n == 1 {
                let both = cols[0].1 + cols[1].1;
                let mut w = [cols[0].1, cols[1].1];
                if g.reset {
                    app.sizes.name_share = Sizes::default().name_share;
                } else {
                    ito_layout::columns::trade(&mut w, 0, g.delta, both * 0.2);
                    app.sizes.name_share = (w[0] / both).clamp(0.2, 0.8);
                }
            } else {
                let i = n - base;
                let mut w = app.sizes.table;
                if g.reset {
                    w[i] = Sizes::default().table[i];
                } else {
                    // `cols` holds the widths as drawn, which the window may have squeezed.
                    w[i] = cols[n].1;
                    ito_layout::columns::resize(&mut w, i, g.delta, COL_MIN, 600.0, ito_layout::columns::Edge::Left);
                }
                app.sizes.table = w;
            }
            if g.released || g.reset {
                app.sizes.save();
            }
        }
    }

    // The rows.
    let pointer = if live { ui.input(|i| i.pointer.hover_pos()).filter(|q| r.contains(*q)) } else { None };
    let mut act: Option<(usize, Option<Act>)> = None;
    for (k, (b, &top)) in blocks.iter().zip(&tops).enumerate() {
        let rr = Rect::from_min_size(pos2(left, at(top)), vec2(width, b.height));
        if rr.bottom() < r.top() || rr.top() > r.bottom() {
            continue;
        }
        let resp = ui.interact(rr.intersect(r), Id::new(("wt-row", k)), Sense::CLICK);
        let selected = k == app.wt_sel;
        let round = if b.card { CornerRadius::same(8) } else { CornerRadius::ZERO };
        if selected {
            p.rect_filled(rr, round, c.selection());
        } else if pointer.is_some_and(|q| rr.contains(q)) {
            p.rect_filled(rr, round, c.fg.gamma_multiply(0.03));
        }
        if b.card {
            p.rect_stroke(rr, round, Stroke::new(1.0, c.border), StrokeKind::Inside);
        } else {
            p.line_segment([pos2(rr.left(), rr.bottom() - 0.5), pos2(rr.right(), rr.bottom() - 0.5)], Stroke::new(1.0, c.border));
        }
        if live && resp.double_clicked() {
            act = Some((k, Some(Act::Enter)));
        } else if live && (resp.clicked() || resp.secondary_clicked()) {
            act = Some((k, None));
        }
        if live {
            resp.context_menu(|ui| {
                if let Some(key) = crate::ctxmenu::show(ui, app, michi_core::menu::Kind::Branch) {
                    act = Some((k, Some(if key.go { Act::Go(key.key) } else { Act::Key(key.key) })));
                }
            });
        }
        let mut pressed = None;
        for (n, (y, line)) in b.lines.iter().enumerate() {
            for (m, col) in line.iter().enumerate() {
                draw_col(ui, &p, col, pos2(left, rr.top() + y), Id::new(("wt-cell", k, n, m)), live, &mut pressed);
            }
        }
        if pressed.is_some() {
            act = Some((k, pressed));
        }
    }

    let pair = draw_pairs(ui, &p, app, &pairs, pos2(left, at(pairs_top)), live);
    let picked_new = draw_news(ui, &p, &news, pos2(left + news_at.0, at(news_at.1)), live);
    if let Some(g) = next {
        p.galley(pos2(left, at(foot_top - next_h)), g, c.fg);
    }
    let mut y = at(foot_top);
    for g in feet {
        let h = g.size().y;
        p.galley(pos2(left, y), g, c.dim);
        y += h + 4.0;
    }

    if let Some((i, j)) = pair {
        let names = (app.clashes.sides[i].name.clone(), app.clashes.sides[j].name.clone());
        let same = pairs.picked.is_some_and(|q| q == (i, j) || q == (j, i));
        ctx.data_mut(|d| {
            if same {
                d.remove_temp::<(String, String)>(picked_id());
            } else {
                d.insert_temp(picked_id(), names);
            }
        });
        ctx.request_repaint();
    }
    if let Some(id) = picked_new {
        app.view = crate::app::View::Graph;
        app.go_to(id);
        ctx.request_repaint();
    }
    match (guide_act, guide) {
        (Some(intro_ui::Act::Dismiss), true) => app.prefs.dismiss_guide(Topic::Worktree),
        (Some(intro_ui::Act::Dismiss), false) => app.prefs.restore_guides(Some(Topic::Worktree)),
        _ => {}
    }
    if again {
        app.reread();
    }
    if new {
        app.feed('N');
    }
    let Some((k, what)) = act else { return };
    app.wt_sel = k;
    // A click on a row selects it: the view need not follow.
    ctx.data_mut(|d| d.insert_temp(seen_id(), k));
    match what {
        Some(Act::Enter) => app.wt_enter(),
        Some(Act::Key(key)) => {
            app.feed(key);
        }
        Some(Act::Go(key)) => {
            app.feed('g');
            app.feed(key);
        }
        Some(Act::Pr) => app.later_word("wt.pr"),
        None => {}
    }
    ctx.request_repaint();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::fixture;
    use crate::repo::Answer;
    use michi_core::summary::{Head, Summary};

    #[test]
    fn buttons_wrap_to_the_width() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let c = ito_theme::colors();
            let b = |w: &str| Button {
                label: layout(ui, w, FontId::proportional(11.5), c.fg, f32::INFINITY),
                key: None,
                act: Act::Pr,
                danger: false,
            };
            let all = [b("one"), b("two"), b("three")];
            let wide = flow(&all, 1000.0);
            assert!(wide.iter().all(|r| r.top() == 0.0));
            let narrow = flow(&all, all[0].width() + 1.0);
            assert_eq!(narrow.iter().map(|r| r.top()).collect::<Vec<_>>(), [0.0, BTN_H + BTN_GAP, 2.0 * (BTN_H + BTN_GAP)]);
            assert!(narrow.iter().all(|r| r.left() == 0.0));
        });
        out.textures_delta.clear();
    }

    #[test]
    fn a_row_goes_by_its_branch_or_its_folder() {
        let t = fixture::trees();
        assert_eq!(name_of(&t.trees[1]), "wip");
        assert_eq!(name_of(&t.trees[2]), "old");
    }

    #[test]
    fn the_table_draws_in_every_state() {
        let ctx = egui::Context::default();
        let mut app = App::new(crate::prefs::Prefs::empty(), "C:/r/repo".into());
        app.lang = "en".into();
        let frame = |app: &mut App, width: f32| {
            let input = egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(width, 300.0))), ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| {
                let r = ui.max_rect();
                show(ui, app, r, true);
            });
            out.textures_delta.clear();
        };
        // Nothing read yet, then a folder that is no repository.
        frame(&mut app, 1200.0);
        app.take(Answer::Summary(Err(michi_core::git::Error::NotRepo)));
        frame(&mut app, 1200.0);
        let s = Summary { name: "repo".into(), head: Head::Branch("main".into()), worktrees: 3, top: "C:/r/repo".into() };
        app.take(Answer::Summary(Ok(s)));
        app.take(Answer::Setup { gen: 1, setup: Box::new(fixture::setup()) });
        app.take(Answer::Commits { gen: 1, batch: fixture::history(), done: true });
        let mut only = fixture::wip("C:/r/repo", Some("main"), 6, 6, None);
        only.files.clear();
        let left = |path: &str| michi_core::predict::LeftOut { path: path.into(), files: 2001, bytes: 1 << 30 };
        only.left_out = vec![left("target-v/"), left("dist/"), left("a.iso")];
        app.take(Answer::Wips(vec![only, fixture::wip("C:/r/wt", Some("wip"), 7, 11, Some(1_767_225_600))]));
        let mut trees = fixture::trees();
        trees.trees[2].wt.prunable = Some("gone".into());
        trees.trees[1].upstream.as_mut().unwrap().gone = true;
        app.take(Answer::Trees(trees));
        // Without tsumugi, wide and as cards.
        frame(&mut app, 1200.0);
        frame(&mut app, 600.0);
        // With tsumugi: a Claude waiting, a second one, and none.
        let session = |id, state| michi_core::claude::Session {
            id,
            state,
            command: "claude".into(),
            agent: "claude".into(),
            cwd: "C:/r/wt".into(),
            branch: "wip".into(),
            title: "fix the parser in a title long enough to be cut at the end of the button".into(),
            note: "Allow edits to src/lib.rs? ".repeat(8),
            since_ms: 1_767_225_600_000,
            read_only: false,
        };
        let sessions = vec![session(3, michi_core::claude::State::Waiting), session(5, michi_core::claude::State::Running)];
        app.take(Answer::Claude(Some(vec![("C:/r/wt".into(), sessions)])));
        frame(&mut app, 1200.0);
        frame(&mut app, 600.0);
        frame(&mut app, 200.0);
        // The keys take the selection out of sight: the view follows.
        app.wt_move(2);
        frame(&mut app, 600.0);
        assert!(ctx.data(|d| d.get_temp::<f32>(scroll_id())).unwrap() > 0.0, "scrolled to the last card");
        app.wt_move(-2);
        frame(&mut app, 600.0);
        assert_eq!(ctx.data(|d| d.get_temp::<f32>(scroll_id())), Some(0.0));
        // Conflicts ahead: in the column, in the table of pairs, with a pair
        // picked, and with one side only.
        app.take(Answer::Clashes(fixture::clashes()));
        assert_eq!(app.clashes_in(std::path::Path::new("C:/r/wt")).len(), 2);
        for width in [1600.0, 1200.0, 980.0, 600.0, 200.0] {
            frame(&mut app, width);
        }
        ctx.data_mut(|d| d.insert_temp(picked_id(), ("wip".to_owned(), "C:/r/old".to_owned())));
        frame(&mut app, 1200.0);
        ctx.data_mut(|d| d.insert_temp(picked_id(), ("main".to_owned(), "wip".to_owned())));
        frame(&mut app, 600.0);
        let mut one = fixture::clashes();
        one.sides.truncate(1);
        one.pairs.clear();
        app.take(Answer::Clashes(one));
        frame(&mut app, 1200.0);
        // Commits not seen yet: in the column, the cards and the list, beside
        // the table of pairs or under it; then more than the list shows.
        app.take(Answer::Unseen(fixture::unseen()));
        // Where the branches stand in review, one of each stage.
        use michi_core::queue::{Reviews, Row};
        let row = |path: &str, stage, checked| Row { path: path.into(), stage, checked, total: 3 };
        app.take(Answer::Reviews(Reviews {
            rows: vec![row("C:/r/wt", Stage::Flagged, 3), row("C:/r/old", Stage::Approved, 3), row("C:/r", Stage::InReview, 1)],
        }));
        for width in [1600.0, 1200.0, 980.0, 600.0, 200.0] {
            frame(&mut app, width);
        }
        let mut many = fixture::unseen();
        let one = many.tracks[0].commits[0].clone();
        many.tracks[0].commits = (1..16u32)
            .map(|n| michi_core::unseen::New { id: fixture::id(n), subject: "a subject ".repeat(n as usize), ..one.clone() })
            .collect();
        app.take(Answer::Unseen(many));
        frame(&mut app, 1600.0);
        frame(&mut app, 600.0);
    }

    #[test]
    fn the_columns_fill_the_width_and_give_near_the_cards() {
        for (width, claude) in [(1600.0, true), (1200.0, true), (CARDS, true), (CARDS, false)] {
            let cols = columns(width, claude, &Sizes::default());
            let (x, w) = *cols.last().unwrap();
            assert!((x + w - width).abs() < 0.5, "{width} {claude}: {cols:?}");
            assert!(cols[0].1 >= 120.0, "{width} {claude}: {cols:?}");
        }
        assert_eq!(columns(1600.0, false, &Sizes::default())[4].1, Sizes::default().table[3]);
    }

    #[test]
    fn a_side_goes_by_the_last_part_of_its_name() {
        assert_eq!(short("feature/login"), "login");
        assert_eq!(short("main"), "main");
        assert_eq!(short("odd/"), "odd/");
    }
}
