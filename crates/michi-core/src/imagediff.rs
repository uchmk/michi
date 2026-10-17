//! Image diff (issue #48): which changed files are pictures, where the bytes
//! of each side are read from, the caps that keep a huge file out of memory,
//! how a picture is fitted into the room it gets, and the difference image of
//! two pictures. Decoding is the UI crate's (it has the `image` crate); this
//! is the part that needs no window and no decoder, so it is tested here.

use std::path::{Path, PathBuf};

use crate::git;

/// A file bigger than this (bytes) is not read.
pub const MAX_BYTES: u64 = 16 * 1024 * 1024;
/// A picture with more pixels than this (width times height) is not decoded:
/// 16 million pixels are 64 MB as RGBA.
pub const MAX_PIXELS: u64 = 16_000_000;

/// The extensions of the pictures the decoder reads.
const EXTENSIONS: [&str; 7] = ["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico"];

/// Whether `path` names a picture that can be shown (by its extension).
pub fn is_image(path: &str) -> bool {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.rsplit_once('.').is_some_and(|(stem, ext)| !stem.is_empty() && EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)))
}

/// Where one side of the change is read from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Src {
    /// There is no such side (the file was added, or deleted).
    Absent,
    /// A blob in the repository at `dir`: `spec` as `cat-file` takes it
    /// (`<rev>:<path>`, or `:<path>` for the index).
    Blob { dir: PathBuf, spec: String },
    /// A file of the worktree.
    File(PathBuf),
}

impl Src {
    /// `path` as `rev` has it.
    pub fn rev(dir: &Path, rev: &str, path: &str) -> Src {
        Src::Blob { dir: dir.to_owned(), spec: format!("{rev}:{path}") }
    }

    /// `path` as the index has it.
    pub fn index(dir: &Path, path: &str) -> Src {
        Src::Blob { dir: dir.to_owned(), spec: format!(":{path}") }
    }

    /// `path` in the worktree `dir`.
    pub fn worktree(dir: &Path, path: &str) -> Src {
        Src::File(dir.join(path))
    }
}

/// What reading one side gave.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bytes {
    Absent,
    Data(Vec<u8>),
    /// Bigger than the cap: its size.
    TooBig(u64),
    /// Could not be read: why.
    Failed(String),
}

/// The bytes at `src`, or why not; nothing over `max` bytes is read. Blocks:
/// call it on a worker thread.
pub fn read(src: &Src, max: u64) -> Bytes {
    match src {
        Src::Absent => Bytes::Absent,
        Src::File(path) => match std::fs::metadata(path) {
            Err(e) => Bytes::Failed(e.to_string()),
            Ok(m) if m.len() > max => Bytes::TooBig(m.len()),
            Ok(_) => std::fs::read(path).map_or_else(|e| Bytes::Failed(e.to_string()), Bytes::Data),
        },
        Src::Blob { dir, spec } => {
            let size = git::read_text(dir, &["cat-file", "-s", spec])
                .and_then(|s| s.trim().parse::<u64>().map_err(|e| git::Error::Io(format!("cat-file -s {spec}: {e}"))));
            match size {
                Err(e) => Bytes::Failed(e.to_string()),
                Ok(n) if n > max => Bytes::TooBig(n),
                Ok(_) => git::read(dir, &["cat-file", "blob", spec]).map_or_else(|e| Bytes::Failed(e.to_string()), Bytes::Data),
            }
        }
    }
}

/// The size `w` by `h` scaled down to fit `max_w` by `max_h`, the shape kept;
/// never scaled up (a small picture stays small). At least 1 by 1.
pub fn fit(w: u32, h: u32, max_w: f32, max_h: f32) -> (f32, f32) {
    if w == 0 || h == 0 {
        return (1.0, 1.0);
    }
    let (w, h) = (w as f32, h as f32);
    let scale = (max_w / w).min(max_h / h).min(1.0);
    ((w * scale).max(1.0), (h * scale).max(1.0))
}

/// `n` bytes as people read them: `512 B`, `1.5 KB`, `2.0 MB`.
pub fn human_bytes(n: u64) -> String {
    const KB: f64 = 1024.0;
    let f = n as f64;
    if n < 1024 {
        format!("{n} B")
    } else if f < KB * KB {
        format!("{:.1} KB", f / KB)
    } else {
        format!("{:.1} MB", f / (KB * KB))
    }
}

/// How the two pictures are laid against each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    /// Before and after, next to each other.
    #[default]
    Side,
    /// One over the other, the divider following the pointer.
    Swipe,
    /// One over the other, the mix following the pointer.
    Onion,
    /// Where they differ.
    Difference,
}

impl Mode {
    /// The next one, round.
    pub fn next(self) -> Mode {
        match self {
            Mode::Side => Mode::Swipe,
            Mode::Swipe => Mode::Onion,
            Mode::Onion => Mode::Difference,
            Mode::Difference => Mode::Side,
        }
    }

    /// The name in the language files.
    pub fn key(self) -> &'static str {
        match self {
            Mode::Side => "image.mode_side",
            Mode::Swipe => "image.mode_swipe",
            Mode::Onion => "image.mode_onion",
            Mode::Difference => "image.mode_diff",
        }
    }
}

/// How the pictures sit in the room they get.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    /// The mode in force: the one asked for, or `Side` when there is only one
    /// picture to show.
    pub mode: Mode,
    /// Points per pixel, the same for both pictures so that their sizes
    /// compare.
    pub scale: f32,
    /// The room one picture area takes, in points.
    pub area: (f32, f32),
    /// Two areas next to each other (`Side` with both pictures).
    pub two: bool,
}

/// Lays out the pictures of sizes `old` and `new` (`None`: not there) in
/// `avail_w` points wide at most `max_h` high, `gap` between two areas.
pub fn layout(old: Option<(u32, u32)>, new: Option<(u32, u32)>, mode: Mode, avail_w: f32, max_h: f32, gap: f32) -> Layout {
    let both = old.is_some() && new.is_some();
    let mode = if both { mode } else { Mode::Side };
    let two = both && mode == Mode::Side;
    let (cw, ch) = old.into_iter().chain(new).fold((1, 1), |(w, h), (pw, ph)| (w.max(pw), h.max(ph)));
    let room = if two { ((avail_w - gap) / 2.0).max(1.0) } else { avail_w.max(1.0) };
    let (fw, fh) = fit(cw, ch, room, max_h);
    Layout { mode, scale: fw / cw as f32, area: (fw, fh), two }
}

/// The difference of two RGBA pictures (`a` is `aw` by `ah`, `b` is `bw` by
/// `bh`) on a canvas big enough for both: width, height, RGBA pixels, and how
/// many pixels differ. A pixel that is the same shows dim; one that differs
/// shows red, the redder the more; one only one of the two has shows magenta.
pub fn difference(a: &[u8], aw: u32, ah: u32, b: &[u8], bw: u32, bh: u32) -> (u32, u32, Vec<u8>, u64) {
    let (w, h) = (aw.max(bw), ah.max(bh));
    let mut out = Vec::with_capacity(w as usize * h as usize * 4);
    let mut differ = 0u64;
    let px = |img: &[u8], iw: u32, ih: u32, x: u32, y: u32| -> Option<[u8; 4]> {
        if x >= iw || y >= ih {
            return None;
        }
        let at = (y as usize * iw as usize + x as usize) * 4;
        img.get(at..at + 4).map(|s| [s[0], s[1], s[2], s[3]])
    };
    for y in 0..h {
        for x in 0..w {
            let p = match (px(a, aw, ah, x, y), px(b, bw, bh, x, y)) {
                (Some(p), Some(q)) => {
                    let d = (0..4).map(|i| p[i].abs_diff(q[i])).max().unwrap_or(0);
                    if d == 0 {
                        let luma = ((u32::from(q[0]) * 3 + u32::from(q[1]) * 6 + u32::from(q[2])) / 10) as u8;
                        let dim = luma / 3 + 20;
                        [dim, dim, dim, 255]
                    } else {
                        differ += 1;
                        [128 + d / 2, 0, 0, 255]
                    }
                }
                _ => {
                    differ += 1;
                    [255, 0, 255, 255]
                }
            };
            out.extend_from_slice(&p);
        }
    }
    (w, h, out, differ)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn pictures_are_told_by_the_extension() {
        for p in ["a.png", "dir/B.JPG", "x/y.jpeg", "anim.gif", "p.webp", "w\\i.bmp", "fav.ico"] {
            assert!(is_image(p), "{p}");
        }
        for p in ["a.svg", "png", ".png", "dir.png/readme", "a.txt", "a.png.bak", ""] {
            assert!(!is_image(p), "{p}");
        }
    }

    #[test]
    fn a_picture_is_fitted_down_and_never_up() {
        assert_eq!(fit(100, 50, 400.0, 400.0), (100.0, 50.0));
        assert_eq!(fit(800, 400, 400.0, 400.0), (400.0, 200.0));
        assert_eq!(fit(400, 800, 400.0, 100.0), (50.0, 100.0));
        assert_eq!(fit(0, 5, 10.0, 10.0), (1.0, 1.0));
        let (w, h) = fit(10_000, 1, 10.0, 10.0);
        assert!(w >= 1.0 && h >= 1.0);
    }

    #[test]
    fn the_layout_shares_one_scale_and_falls_back_to_side_by_side() {
        // Both there, side by side: each gets half the room.
        let l = layout(Some((200, 100)), Some((400, 100)), Mode::Side, 410.0, 300.0, 10.0);
        assert!(l.two);
        assert_eq!(l.mode, Mode::Side);
        assert!((l.scale - 0.5).abs() < 1e-6);
        assert_eq!(l.area, (200.0, 50.0));
        // Swipe takes the whole width.
        let l = layout(Some((200, 100)), Some((400, 100)), Mode::Swipe, 410.0, 300.0, 10.0);
        assert!(!l.two);
        assert_eq!(l.mode, Mode::Swipe);
        assert_eq!(l.scale, 1.0);
        // One picture only: whatever the mode, one area, side by side.
        let l = layout(None, Some((50, 50)), Mode::Difference, 410.0, 300.0, 10.0);
        assert_eq!((l.mode, l.two, l.area), (Mode::Side, false, (50.0, 50.0)));
        // The height caps it.
        let l = layout(Some((100, 1000)), None, Mode::Side, 410.0, 200.0, 10.0);
        assert_eq!(l.area.1, 200.0);
    }

    #[test]
    fn sizes_read_as_people_write_them() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(1023), "1023 B");
        assert_eq!(human_bytes(1536), "1.5 KB");
        assert_eq!(human_bytes(2 * 1024 * 1024), "2.0 MB");
    }

    #[test]
    fn the_mode_goes_round_and_every_one_has_a_name() {
        let mut m = Mode::default();
        let mut seen = Vec::new();
        for _ in 0..4 {
            seen.push(m.key());
            m = m.next();
        }
        assert_eq!(m, Mode::Side);
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), 4);
    }

    #[test]
    fn the_difference_marks_changed_and_missing_pixels() {
        // 2x1 against 3x1: the first pixel is the same, the second differs,
        // the third only the second picture has.
        let a = [10, 20, 30, 255, 0, 0, 0, 255];
        let b = [10, 20, 30, 255, 100, 0, 0, 255, 1, 2, 3, 255];
        let (w, h, px, n) = difference(&a, 2, 1, &b, 3, 1);
        assert_eq!((w, h, n), (3, 1, 2));
        assert_eq!(px.len(), 12);
        assert_eq!(px[0], px[1]);
        assert_eq!(&px[4..8], &[128 + 50, 0, 0, 255]);
        assert_eq!(&px[8..12], &[255, 0, 255, 255]);
        let (_, _, _, same) = difference(&b, 3, 1, &b, 3, 1);
        assert_eq!(same, 0);
    }

    #[test]
    fn sides_are_read_from_a_commit_the_index_and_the_worktree() {
        let t = TempRepo::configured("imagediff-read");
        std::fs::write(t.repo.join("a.png"), [1u8, 2, 3, 4, 5]).unwrap();
        t.git(&t.repo, &["add", "a.png"]);
        t.git(&t.repo, &["commit", "-q", "-m", "add"]);
        std::fs::write(t.repo.join("a.png"), [9u8, 9]).unwrap();
        t.git(&t.repo, &["add", "a.png"]);
        std::fs::write(t.repo.join("a.png"), [7u8]).unwrap();

        assert_eq!(read(&Src::rev(&t.repo, "HEAD", "a.png"), MAX_BYTES), Bytes::Data(vec![1, 2, 3, 4, 5]));
        assert_eq!(read(&Src::index(&t.repo, "a.png"), MAX_BYTES), Bytes::Data(vec![9, 9]));
        assert_eq!(read(&Src::worktree(&t.repo, "a.png"), MAX_BYTES), Bytes::Data(vec![7]));
        assert_eq!(read(&Src::Absent, MAX_BYTES), Bytes::Absent);
        assert!(matches!(read(&Src::rev(&t.repo, "HEAD", "none.png"), MAX_BYTES), Bytes::Failed(_)));
        assert!(matches!(read(&Src::worktree(&t.repo, "none.png"), MAX_BYTES), Bytes::Failed(_)));
    }

    #[test]
    fn a_side_over_the_cap_is_not_read() {
        let t = TempRepo::configured("imagediff-cap");
        std::fs::write(t.repo.join("big.png"), [0u8; 100]).unwrap();
        t.git(&t.repo, &["add", "big.png"]);
        t.git(&t.repo, &["commit", "-q", "-m", "big"]);
        assert_eq!(read(&Src::rev(&t.repo, "HEAD", "big.png"), 99), Bytes::TooBig(100));
        assert_eq!(read(&Src::worktree(&t.repo, "big.png"), 99), Bytes::TooBig(100));
        assert!(matches!(read(&Src::worktree(&t.repo, "big.png"), 100), Bytes::Data(d) if d.len() == 100));
    }
}
