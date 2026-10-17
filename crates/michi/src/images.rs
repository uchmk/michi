//! Image diff (issue #48): a changed picture shows its before and its after
//! (side by side, swiped, mixed, or as the difference) in the commit detail
//! and the Changes view. The bytes are read (`git cat-file`, or the worktree
//! file) and decoded on one thread of their own, the textures made there; the
//! UI thread only asks and draws. What is known without a window (where the
//! bytes come from, the caps, the layout, the difference) is in
//! `michi_core::imagediff`.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

use egui::{pos2, vec2, Align2, Color32, Context, FontId, Painter, Rect, TextureHandle, TextureOptions, Ui};
use michi_core::imagediff::{self, Bytes, Mode, Src};

use crate::app::App;
use crate::detail_ui::{NOTE_H, X};

/// The most a picture area is high, in points.
const MAX_H: f32 = 320.0;
/// Between the two areas.
const GAP: f32 = 14.0;
/// The line over the pictures.
const LINE_H: f32 = 22.0;
/// Under the pictures.
const FOOT: f32 = 14.0;
/// A texture is no bigger than this on a side (the picture is cut down).
const TEXTURE_SIDE: u32 = 4096;
/// Pairs kept for the session, the oldest let go first.
const KEPT: usize = 12;

/// What to show: which two sides of which file.
#[derive(Clone, Debug)]
pub struct Job {
    /// The file's place: a newer read of the same place is shown while it is
    /// read again.
    pub slot: String,
    /// This read: a new one when the content may have changed.
    pub key: String,
    pub old: Src,
    pub new: Src,
}

/// Why a side shows no picture.
#[derive(Clone, Debug)]
pub enum Why {
    TooBig(u64),
    TooMany(u32, u32),
    Undecodable,
    Failed(String),
}

pub struct Pic {
    pub w: u32,
    pub h: u32,
    pub bytes: u64,
    tex: TextureHandle,
}

pub enum Side {
    /// There is no such side: the file is new, or gone.
    Absent,
    Pic(Pic),
    Why(Why),
}

pub struct Pair {
    pub old: Side,
    pub new: Side,
    /// Where they differ, and how many pixels do.
    diff: Option<(TextureHandle, u64)>,
}

enum Slot {
    Loading,
    Ready(Arc<Pair>),
}

struct Done {
    key: String,
    slot: String,
    pair: Pair,
}

#[derive(Default)]
struct Inner {
    slots: HashMap<String, Slot>,
    /// The keys in the order they came, for letting the oldest go.
    order: Vec<String>,
    /// The last pair read for each place.
    last: HashMap<String, Arc<Pair>>,
    ask: Option<Sender<(Job, Context)>>,
    got: Option<Receiver<Done>>,
}

#[derive(Default)]
pub struct Images {
    mode: Cell<Mode>,
    inner: RefCell<Inner>,
}

impl Images {
    pub fn mode(&self) -> Mode {
        self.mode.get()
    }

    /// `O`: on to the next way of laying the pictures against each other.
    pub fn cycle(&self) {
        self.mode.set(self.mode.get().next());
    }

    fn pump(inner: &mut Inner) {
        let Some(rx) = &inner.got else { return };
        while let Ok(d) = rx.try_recv() {
            let pair = Arc::new(d.pair);
            inner.last.insert(d.slot, pair.clone());
            inner.slots.insert(d.key, Slot::Ready(pair));
        }
        while inner.order.len() > KEPT {
            let k = inner.order.remove(0);
            inner.slots.remove(&k);
        }
        if inner.last.len() > 2 * KEPT {
            inner.last.clear();
        }
    }

    /// The pair for `job` if it is read, else the last one read for its
    /// place; asks for it the first time. Never waits.
    pub fn get(&self, ctx: &Context, job: &Job) -> Option<Arc<Pair>> {
        let mut inner = self.inner.borrow_mut();
        Self::pump(&mut inner);
        match inner.slots.get(&job.key) {
            Some(Slot::Ready(p)) => return Some(p.clone()),
            Some(Slot::Loading) => return inner.last.get(&job.slot).cloned(),
            None => {}
        }
        if inner.ask.is_none() {
            let (ask_tx, ask_rx) = channel::<(Job, Context)>();
            let (got_tx, got_rx) = channel();
            let spawned = std::thread::Builder::new().name("images".into()).spawn(move || {
                while let Ok((job, ctx)) = ask_rx.recv() {
                    let pair = load(&ctx, &job);
                    if got_tx.send(Done { key: job.key, slot: job.slot, pair }).is_err() {
                        break;
                    }
                    ctx.request_repaint();
                }
            });
            if spawned.is_err() {
                return None;
            }
            (inner.ask, inner.got) = (Some(ask_tx), Some(got_rx));
        }
        let sent = inner.ask.as_ref().is_some_and(|t| t.send((job.clone(), ctx.clone())).is_ok());
        if sent {
            inner.slots.insert(job.key.clone(), Slot::Loading);
            inner.order.push(job.key.clone());
        }
        inner.last.get(&job.slot).cloned()
    }

    /// The pair for `job` if it (or an older read of its place) is here.
    /// Never asks: for the height of a block that may not be in sight.
    fn peek(&self, job: &Job) -> Option<Arc<Pair>> {
        let mut inner = self.inner.borrow_mut();
        Self::pump(&mut inner);
        match inner.slots.get(&job.key) {
            Some(Slot::Ready(p)) => Some(p.clone()),
            _ => inner.last.get(&job.slot).cloned(),
        }
    }
}

/// Reads and decodes both sides of `job`; the textures are made here.
fn load(ctx: &Context, job: &Job) -> Pair {
    let n = job.key.len();
    let (old, old_px) = side(ctx, &format!("img-{n}-old-{}", job.key), &job.old);
    let (new, new_px) = side(ctx, &format!("img-{n}-new-{}", job.key), &job.new);
    let diff = match (&old_px, &new_px) {
        (Some((ow, oh, o)), Some((nw, nh, p))) => {
            let (w, h, px, differ) = imagediff::difference(o, *ow, *oh, p, *nw, *nh);
            Some((texture(ctx, format!("img-diff-{}", job.key), w, h, px), differ))
        }
        _ => None,
    };
    Pair { old, new, diff }
}

type Decoded = (u32, u32, Vec<u8>);

/// One side: what to show for it, and its pixels when it is a picture.
fn side(ctx: &Context, name: &str, src: &Src) -> (Side, Option<Decoded>) {
    let data = match imagediff::read(src, imagediff::MAX_BYTES) {
        Bytes::Absent => return (Side::Absent, None),
        Bytes::TooBig(n) => return (Side::Why(Why::TooBig(n)), None),
        Bytes::Failed(e) => return (Side::Why(Why::Failed(e)), None),
        Bytes::Data(d) => d,
    };
    let bytes = data.len() as u64;
    let Some((w, h)) = dimensions(&data) else { return (Side::Why(Why::Undecodable), None) };
    if u64::from(w) * u64::from(h) > imagediff::MAX_PIXELS {
        return (Side::Why(Why::TooMany(w, h)), None);
    }
    let Ok(img) = image::load_from_memory(&data) else { return (Side::Why(Why::Undecodable), None) };
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    let px = rgba.into_raw();
    let tex = texture(ctx, name.to_owned(), w, h, px.clone());
    (Side::Pic(Pic { w, h, bytes, tex }), Some((w, h, px)))
}

/// A picture's size from its header, without decoding it.
fn dimensions(data: &[u8]) -> Option<(u32, u32)> {
    image::ImageReader::new(Cursor::new(data)).with_guessed_format().ok()?.into_dimensions().ok()
}

/// The pixels as a texture, cut down when a side is longer than a texture
/// may be.
fn texture(ctx: &Context, name: String, w: u32, h: u32, rgba: Vec<u8>) -> TextureHandle {
    let (w, h, rgba) = if w.max(h) > TEXTURE_SIDE {
        let k = TEXTURE_SIDE as f32 / w.max(h) as f32;
        let (nw, nh) = (((w as f32 * k) as u32).max(1), ((h as f32 * k) as u32).max(1));
        match image::RgbaImage::from_raw(w, h, rgba) {
            Some(img) => (nw, nh, image::imageops::resize(&img, nw, nh, image::imageops::FilterType::Triangle).into_raw()),
            None => (1, 1, vec![0; 4]),
        }
    } else {
        (w, h, rgba)
    };
    ctx.load_texture(name, egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba), TextureOptions::LINEAR)
}

fn size_of(s: &Side) -> Option<(u32, u32)> {
    match s {
        Side::Pic(p) => Some((p.w, p.h)),
        _ => None,
    }
}

/// How tall the block for `job` is, in the width `width`.
pub fn height(app: &App, job: &Job, width: f32) -> f32 {
    match app.images.peek(job) {
        None => NOTE_H,
        Some(pair) => match shown(&pair) {
            Shown::Note => NOTE_H + LINE_H,
            Shown::Pictures => {
                let l = layout(app, &pair, width);
                LINE_H + l.area.1 + FOOT
            }
        },
    }
}

enum Shown {
    /// Nothing to draw but the reasons (both sides failed or are absent).
    Note,
    Pictures,
}

fn shown(pair: &Pair) -> Shown {
    if size_of(&pair.old).is_some() || size_of(&pair.new).is_some() {
        Shown::Pictures
    } else {
        Shown::Note
    }
}

fn layout(app: &App, pair: &Pair, width: f32) -> imagediff::Layout {
    let mode = if pair.diff.is_none() && app.images.mode() == Mode::Difference { Mode::Side } else { app.images.mode() };
    imagediff::layout(size_of(&pair.old), size_of(&pair.new), mode, (width - 2.0 * X).max(40.0), MAX_H, GAP)
}

fn why_text(app: &App, w: &Why) -> String {
    match w {
        Why::TooBig(n) => app.tr("image.too_big").replace("{}", &imagediff::human_bytes(*n)),
        Why::TooMany(w, h) => app.tr("image.too_many").replace("{}", &format!("{w}\u{d7}{h}")),
        Why::Undecodable => app.tr("image.undecodable").to_owned(),
        Why::Failed(e) => app.tr("image.failed").replace("{}", e),
    }
}

/// What the line over a side says: its size, or why it is not here.
fn describe(app: &App, label: &str, s: &Side) -> String {
    match s {
        Side::Absent => format!("{label}: {}", app.tr("image.none")),
        Side::Pic(p) => format!("{label}: {}\u{d7}{} \u{b7} {}", p.w, p.h, imagediff::human_bytes(p.bytes)),
        Side::Why(w) => format!("{label}: {}", why_text(app, w)),
    }
}

/// Draws the block for `job` in `rr`, asking for the pictures if they are not
/// here. `clip` is what is in sight; the pointer is read only when `live`.
pub fn draw(ui: &Ui, p: &Painter, app: &App, job: &Job, rr: Rect, live: bool) {
    let c = ito_theme::colors();
    let font = FontId::proportional(12.0);
    let pair = app.images.get(ui.ctx(), job);
    let Some(pair) = pair else {
        p.text(pos2(rr.left() + X, rr.top() + 10.0), Align2::LEFT_TOP, app.tr("image.loading"), font, c.dim);
        return;
    };
    let top = rr.top() + 4.0;
    let info = format!("{}    {}", describe(app, app.tr("image.before"), &pair.old), describe(app, app.tr("image.after"), &pair.new));
    p.text(pos2(rr.left() + X, top), Align2::LEFT_TOP, info, font.clone(), c.dim);
    if matches!(shown(&pair), Shown::Note) {
        return;
    }
    let l = layout(app, &pair, rr.width());
    let mode_text = format!("{}  (I)", app.tr(l.mode.key()));
    p.text(pos2(rr.right() - X, top), Align2::RIGHT_TOP, mode_text, font.clone(), c.blue);

    let area_top = rr.top() + LINE_H;
    let (aw, ah) = l.area;
    let first = Rect::from_min_size(pos2(rr.left() + X, area_top), vec2(aw, ah));
    let pointer = if live { ui.input(|i| i.pointer.hover_pos()) } else { None };
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    // A picture drawn at its own size inside an area.
    let put = |painter: &Painter, area: Rect, s: &Side, tint: Color32| {
        if let Side::Pic(pic) = s {
            let r = Rect::from_min_size(area.min, vec2(pic.w as f32 * l.scale, pic.h as f32 * l.scale));
            painter.image(pic.tex.id(), r, uv, tint);
        }
    };
    if l.two {
        let second = first.translate(vec2(aw + GAP, 0.0));
        for (area, s) in [(first, &pair.old), (second, &pair.new)] {
            backdrop(p, area_of(s, area, l.scale));
            put(p, area, s, Color32::WHITE);
        }
        return;
    }
    backdrop(p, first);
    // The pointer picks the divider, or the mix.
    let frac = pointer.filter(|at| first.contains(*at)).map_or(0.5, |at| ((at.x - first.left()) / first.width().max(1.0)).clamp(0.0, 1.0));
    match l.mode {
        Mode::Difference => {
            if let Some((tex, differ)) = &pair.diff {
                p.image(tex.id(), first, uv, Color32::WHITE);
                let text = if *differ == 0 {
                    app.tr("image.identical").to_owned()
                } else {
                    app.tr("image.differ").replace("{}", &differ.to_string())
                };
                p.text(
                    pos2(first.left() + 6.0, first.bottom() - 4.0),
                    Align2::LEFT_BOTTOM,
                    text,
                    FontId::proportional(12.0),
                    Color32::WHITE,
                );
            }
        }
        Mode::Swipe => {
            put(p, first, &pair.old, Color32::WHITE);
            let x = first.left() + first.width() * frac;
            let right = p.with_clip_rect(Rect::from_min_max(pos2(x, first.top()), first.max).intersect(p.clip_rect()));
            put(&right, first, &pair.new, Color32::WHITE);
            p.line_segment([pos2(x, first.top()), pos2(x, first.bottom())], egui::Stroke::new(1.5, c.blue));
        }
        Mode::Onion => {
            put(p, first, &pair.old, Color32::WHITE);
            put(p, first, &pair.new, Color32::from_white_alpha((frac * 255.0) as u8));
        }
        Mode::Side => {
            // Only one side is a picture here.
            put(p, first, &pair.old, Color32::WHITE);
            put(p, first, &pair.new, Color32::WHITE);
        }
    }
}

/// The room a picture of `s` takes in `area`.
fn area_of(s: &Side, area: Rect, scale: f32) -> Rect {
    match s {
        Side::Pic(p) => Rect::from_min_size(area.min, vec2(p.w as f32 * scale, p.h as f32 * scale)),
        _ => Rect::from_min_size(area.min, vec2(0.0, 0.0)),
    }
}

/// A checkerboard under a picture, so that what is clear shows.
fn backdrop(p: &Painter, r: Rect) {
    if r.width() <= 0.0 || r.height() <= 0.0 {
        return;
    }
    const S: f32 = 10.0;
    let (a, b) = (Color32::from_gray(52), Color32::from_gray(64));
    let p = p.with_clip_rect(r.intersect(p.clip_rect()));
    p.rect_filled(r, 0.0, a);
    let cols = (r.width() / S).ceil() as i32;
    let rows = (r.height() / S).ceil() as i32;
    for j in 0..rows {
        for i in 0..cols {
            if (i + j) % 2 == 1 {
                let q = Rect::from_min_size(pos2(r.left() + i as f32 * S, r.top() + j as f32 * S), vec2(S, S));
                p.rect_filled(q, 0.0, b);
            }
        }
    }
}

/// The job for the file `path` of the commit `id`, whose first parent is
/// `parent` (`None` for a root commit); `from` is where a renamed file was.
pub fn commit_job(dir: &std::path::Path, id: &str, parent: Option<&str>, f: &michi_core::show::File) -> Job {
    use michi_core::show::Status;
    let old = match (parent, f.status) {
        (Some(p), s) if s != Status::Added => Src::rev(dir, p, f.from.as_deref().unwrap_or(&f.path)),
        _ => Src::Absent,
    };
    let new = if f.status == Status::Deleted { Src::Absent } else { Src::rev(dir, id, &f.path) };
    Job { slot: format!("commit:{}", f.path), key: format!("commit:{id}:{}", f.path), old, new }
}

/// The job for the file `f` on `side` of the Changes view of the worktree
/// `dir`; `read` counts the times the diff was read (a file edited again is
/// shown again).
pub fn changes_job(dir: &std::path::Path, side: michi_core::stage::Side, f: &michi_core::show::File, read: u32) -> Job {
    use michi_core::show::Status;
    use michi_core::stage::Side as S;
    let from = f.from.as_deref().unwrap_or(&f.path);
    let (old, new) = match side {
        S::Staged => (
            if f.status == Status::Added { Src::Absent } else { Src::rev(dir, "HEAD", from) },
            if f.status == Status::Deleted { Src::Absent } else { Src::index(dir, &f.path) },
        ),
        S::Unstaged => (
            if f.status == Status::Untracked { Src::Absent } else { Src::index(dir, from) },
            if f.status == Status::Deleted { Src::Absent } else { Src::worktree(dir, &f.path) },
        ),
    };
    let slot = format!("changes:{}:{side:?}:{}", dir.display(), f.path);
    Job { key: format!("{slot}:{read}"), slot, old, new }
}

#[cfg(test)]
mod tests {
    use super::*;
    use michi_core::show::{File, Status};
    use michi_core::stage::Side as S;
    use std::path::Path;

    fn file(status: Status, path: &str, from: Option<&str>) -> File {
        File {
            status,
            path: path.into(),
            from: from.map(Into::into),
            blob: None,
            counts: None,
            hunks: Vec::new(),
            eol_only: false,
            cut: false,
            not_utf8: false,
        }
    }

    #[test]
    fn a_commits_sides_are_the_parent_and_the_commit() {
        let d = Path::new("/r");
        let j = commit_job(d, "c1", Some("p1"), &file(Status::Modified, "a.png", None));
        assert_eq!(j.old, Src::rev(d, "p1", "a.png"));
        assert_eq!(j.new, Src::rev(d, "c1", "a.png"));
        let j = commit_job(d, "c1", Some("p1"), &file(Status::Added, "a.png", None));
        assert_eq!((j.old, j.new), (Src::Absent, Src::rev(d, "c1", "a.png")));
        let j = commit_job(d, "c1", Some("p1"), &file(Status::Deleted, "a.png", None));
        assert_eq!((j.old, j.new), (Src::rev(d, "p1", "a.png"), Src::Absent));
        let j = commit_job(d, "c1", Some("p1"), &file(Status::Renamed, "b.png", Some("a.png")));
        assert_eq!((j.old, j.new), (Src::rev(d, "p1", "a.png"), Src::rev(d, "c1", "b.png")));
        let j = commit_job(d, "c1", None, &file(Status::Modified, "a.png", None));
        assert_eq!(j.old, Src::Absent);
    }

    #[test]
    fn the_changes_sides_are_head_the_index_and_the_worktree() {
        let d = Path::new("/r");
        let m = file(Status::Modified, "a.png", None);
        let staged = changes_job(d, S::Staged, &m, 1);
        assert_eq!((staged.old, staged.new), (Src::rev(d, "HEAD", "a.png"), Src::index(d, "a.png")));
        let unstaged = changes_job(d, S::Unstaged, &m, 1);
        assert_eq!((unstaged.old, unstaged.new), (Src::index(d, "a.png"), Src::worktree(d, "a.png")));
        let new = changes_job(d, S::Unstaged, &file(Status::Untracked, "n.png", None), 1);
        assert_eq!((new.old, new.new), (Src::Absent, Src::worktree(d, "n.png")));
        let gone = changes_job(d, S::Unstaged, &file(Status::Deleted, "a.png", None), 1);
        assert_eq!((gone.old, gone.new), (Src::index(d, "a.png"), Src::Absent));
        let added = changes_job(d, S::Staged, &file(Status::Added, "a.png", None), 1);
        assert_eq!(added.old, Src::Absent);
        // A new read of the same file is a new job at the same place.
        let again = changes_job(d, S::Staged, &m, 2);
        assert_eq!(again.slot, staged.slot);
        assert_ne!(again.key, staged.key);
    }

    #[test]
    fn dimensions_are_read_from_the_header() {
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(image::RgbaImage::new(3, 2)).write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        assert_eq!(dimensions(&png), Some((3, 2)));
        assert_eq!(dimensions(b"not a picture"), None);
    }
}
