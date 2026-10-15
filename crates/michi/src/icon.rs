//! The app's icon (assets/icon.svg, mark A "合流"): a gold branch leaving a
//! hollow commit and joining main, the cyan line on the left lane, on kura's
//! dark tile. Drawn here at any size so no image file is carried.

type Rgb = [u8; 3];

/// The tile's colour and its rim (the SVG's `#16181d` and `#2c3039`).
const TILE: Rgb = [0x16, 0x18, 0x1d];
const RIM: Rgb = [0x2c, 0x30, 0x39];
const CYAN: Rgb = [0x6f, 0xd0, 0xd0];
const GOLD: Rgb = [0xe8, 0xc8, 0x7a];

/// One cubic Bézier, in the tile's 32-unit box.
type Cubic = [[f32; 2]; 4];

/// The SVG's marks go through `translate(2.4 2.4) scale(.85)`.
const fn at(x: f32, y: f32) -> [f32; 2] {
    [2.4 + 0.85 * x, 2.4 + 0.85 * y]
}

/// main: `M10 4v24`, as a cubic that is a straight line.
const MAIN: Cubic = [at(10.0, 4.0), at(10.0, 12.0), at(10.0, 20.0), at(10.0, 28.0)];
/// The branch: `M22 25v-5c0-6-12-5-12-11`, the straight part then the bend.
const BRANCH: [Cubic; 2] =
    [[at(22.0, 25.0), at(22.0, 23.33), at(22.0, 21.67), at(22.0, 20.0)], [at(22.0, 20.0), at(22.0, 14.0), at(10.0, 15.0), at(10.0, 9.0)]];
/// The branch's first commit, hollow, and main's, filled.
const RING: [f32; 2] = at(22.0, 25.0);
const DOT: [f32; 2] = at(10.0, 7.0);
/// Their radius (3.6 in the SVG) and the ring's width (2.4).
const DOT_R: f32 = 3.6 * 0.85;
const RING_W: f32 = 2.4 * 0.85;
/// The lines' width (3 in the SVG).
const LINE_W: f32 = 3.0 * 0.85;

/// The icon as RGBA, `size` × `size`.
pub fn pixels(size: u32) -> Vec<u8> {
    let n = size as usize;
    let unit = size as f32 / 32.0;
    // Thicker at small sizes so the lines survive 16 px.
    let boost = if size <= 16 {
        1.35
    } else if size <= 32 {
        1.15
    } else {
        1.0
    };
    let line = LINE_W * unit * boost;
    let main = curve(&[MAIN], unit);
    let branch = curve(&BRANCH, unit);
    let (ring, dot) = (RING.map(|v| v * unit), DOT.map(|v| v * unit));
    let r = DOT_R * unit;
    let ring_w = RING_W * unit * boost;
    // The SVG's tile is inset half a unit with a 1-unit stroke: its outer
    // edge is the icon's, rounded by 7.3; the rim is the outer unit.
    let radius = 7.3 * unit;
    let mut out = vec![0u8; n * n * 4];
    for y in 0..n {
        for x in 0..n {
            let p = [x as f32 + 0.5, y as f32 + 0.5];
            let tile = rounded(p, size as f32, 0.0, radius);
            if tile <= 0.0 {
                continue;
            }
            let mut rgb = TILE.map(f32::from);
            let inside = rounded(p, size as f32, unit, radius - unit);
            blend(&mut rgb, RIM, 1.0 - inside);
            blend(&mut rgb, CYAN, cover(line / 2.0 - distance(p, &main)));
            blend(&mut rgb, GOLD, cover(line / 2.0 - distance(p, &branch)));
            // The ring is filled with the tile first, so the branch's line
            // does not show through its hollow.
            let d = far(p, ring);
            blend(&mut rgb, TILE, cover(r - d));
            blend(&mut rgb, GOLD, cover(ring_w / 2.0 - (d - r).abs()));
            blend(&mut rgb, CYAN, cover(r - far(p, dot)));
            let i = (y * n + x) * 4;
            out[i..i + 3].copy_from_slice(&rgb.map(|v| v.round() as u8));
            out[i + 3] = (tile * 255.0).round() as u8;
        }
    }
    out
}

/// How much of a pixel a shape covers, from how far inside it the pixel's
/// centre is.
fn cover(inside: f32) -> f32 {
    (inside + 0.5).clamp(0.0, 1.0)
}

fn blend(rgb: &mut [f32; 3], color: Rgb, cover: f32) {
    for k in 0..3 {
        rgb[k] += (f32::from(color[k]) - rgb[k]) * cover;
    }
}

fn far(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// The cubics as a run of points, in pixels.
fn curve(segs: &[Cubic], unit: f32) -> Vec<[f32; 2]> {
    let mut out = Vec::new();
    for seg in segs {
        for k in 0..=24 {
            let t = k as f32 / 24.0;
            let u = 1.0 - t;
            let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            let x: f32 = (0..4).map(|j| w[j] * seg[j][0]).sum();
            let y: f32 = (0..4).map(|j| w[j] * seg[j][1]).sum();
            out.push([x * unit, y * unit]);
        }
    }
    out
}

/// How far `p` is from the run of points (its segments).
fn distance(p: [f32; 2], line: &[[f32; 2]]) -> f32 {
    line.windows(2)
        .map(|s| {
            let (a, b) = (s[0], s[1]);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len = dx * dx + dy * dy;
            let t = if len == 0.0 { 0.0 } else { (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len).clamp(0.0, 1.0) };
            far(p, [a[0] + t * dx, a[1] + t * dy])
        })
        .fold(f32::MAX, f32::min)
}

/// How much of the pixel at `p` is inside the rounded square `inset` in from
/// the icon's edges with corners of radius `r`, 0 to 1.
fn rounded(p: [f32; 2], size: f32, inset: f32, r: f32) -> f32 {
    let q = |v: f32| (v - size / 2.0).abs() - (size / 2.0 - inset - r);
    let (qx, qy) = (q(p[0]), q(p[1]));
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - r;
    (0.5 - outside).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_has_main_the_branch_and_both_commits() {
        let px = pixels(128);
        assert_eq!(px.len(), 128 * 128 * 4);
        // In the tile's 32 units: 4 pixels each.
        let get = |ux: f32, uy: f32| {
            let (x, y) = ((ux * 4.0) as usize, (uy * 4.0) as usize);
            &px[(y * 128 + x) * 4..(y * 128 + x) * 4 + 4]
        };
        assert_eq!(get(0.2, 0.2)[3], 0, "the corner is cut round");
        assert_eq!(get(16.0, 3.0), &[0x16, 0x18, 0x1d, 255], "the tile");
        let dot = get(DOT[0], DOT[1]);
        assert!(dot[1] > 0xb0 && dot[0] < 0x90, "main's commit: {dot:?}");
        let main = get(10.9, 22.0);
        assert!(main[1] > 0xb0 && main[0] < 0x90, "main: {main:?}");
        let ring = get(RING[0] + DOT_R, RING[1]);
        assert!(ring[0] > 0xc0 && ring[2] < 0x90, "the ring: {ring:?}");
        assert_eq!(get(RING[0], RING[1]), &[0x16, 0x18, 0x1d, 255], "the ring is hollow");
        let bend = get(16.0, 15.0);
        assert!(bend[0] > 0xc0 && bend[2] < 0x90, "the branch's bend: {bend:?}");
        assert_eq!(pixels(16).len(), 16 * 16 * 4);
    }
}
