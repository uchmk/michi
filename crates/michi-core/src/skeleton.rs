//! Loading skeletons (issue #90): while the history is read and no row has
//! come, the graph shows thin placeholder rows; after a while a line of
//! progress text joins them. This is the part that needs no egui: how many
//! rows, how wide each bar is, how the pulse goes and when the text is due.

use std::time::Duration;

/// After this long without a row the progress text is shown too.
pub const TEXT_AFTER: Duration = Duration::from_secs(2);

/// Most placeholder rows drawn.
const MOST_ROWS: usize = 40;

/// How many placeholder rows fill `height` px of rows `row_h` px high.
pub fn rows(height: f32, row_h: f32) -> usize {
    if height.is_nan() || row_h.is_nan() || height <= 0.0 || row_h <= 0.0 {
        return 0;
    }
    ((height / row_h).ceil() as usize).min(MOST_ROWS)
}

/// The width of row `k`'s subject bar as a share (0.25 to 0.85) of the room
/// for it: fixed per row, so the rows do not jump from frame to frame.
pub fn bar(k: usize) -> f32 {
    let h = (k as u32).wrapping_add(7).wrapping_mul(2_654_435_761).rotate_left(13) % 1000;
    0.25 + 0.6 * (h as f32 / 999.0)
}

/// The placeholders' strength (0.35 to 1.0), a slow pulse over `t`; each row
/// is a little behind the one above so a wave runs down.
pub fn pulse(t: Duration, k: usize) -> f32 {
    let phase = t.as_secs_f32() * 1.6 - k as f32 * 0.15;
    0.675 + 0.325 * phase.sin()
}

/// The progress text is due once `t` has gone past [`TEXT_AFTER`].
pub fn text_due(t: Duration) -> bool {
    t > TEXT_AFTER
}

/// The whole seconds to show in the progress text.
pub fn seconds(t: Duration) -> u64 {
    t.as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_fill_the_height_and_are_capped() {
        assert_eq!(rows(100.0, 25.0), 4);
        assert_eq!(rows(101.0, 25.0), 5);
        assert_eq!(rows(100_000.0, 25.0), MOST_ROWS);
        assert_eq!(rows(0.0, 25.0), 0);
        assert_eq!(rows(-3.0, 25.0), 0);
        assert_eq!(rows(100.0, 0.0), 0);
        assert_eq!(rows(f32::NAN, 25.0), 0);
    }

    #[test]
    fn bars_are_stable_and_in_range() {
        for k in 0..200 {
            let b = bar(k);
            assert!((0.25..=0.85).contains(&b), "row {k}: {b}");
            assert_eq!(b, bar(k));
        }
        let distinct = (0..20).map(|k| (bar(k) * 1000.0) as u32).collect::<std::collections::HashSet<_>>();
        assert!(distinct.len() > 5);
    }

    #[test]
    fn pulse_stays_in_range() {
        for ms in (0..10_000).step_by(37) {
            for k in 0..30 {
                let p = pulse(Duration::from_millis(ms), k);
                assert!((0.34..=1.01).contains(&p), "{p}");
            }
        }
    }

    #[test]
    fn text_comes_after_two_seconds() {
        assert!(!text_due(Duration::ZERO));
        assert!(!text_due(Duration::from_secs(2)));
        assert!(text_due(Duration::from_millis(2001)));
        assert_eq!(seconds(Duration::from_millis(3999)), 3);
    }
}
