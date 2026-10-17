//! The diff's minimap: where the changes of a file's diff lie, as runs of
//! rows, and the jumps `n` / `N` make from one run to the next. No egui here;
//! the window turns the runs into a strip beside the diff.

/// What a row of a diff holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// Context, or a hunk's `@@` line.
    Same,
    Add,
    Del,
    /// Side by side: a deleted line paired with an added one.
    Both,
}

impl Mark {
    fn merge(self, other: Mark) -> Mark {
        match (self, other) {
            (a, b) if a == b => a,
            (Mark::Same, b) => b,
            (a, Mark::Same) => a,
            _ => Mark::Both,
        }
    }
}

/// Rows `start..end` that change, with nothing unchanged between them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub start: usize,
    pub end: usize,
    pub mark: Mark,
}

/// The runs of changed rows, in order.
pub fn runs(marks: &[Mark]) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    for (k, &m) in marks.iter().enumerate() {
        if m == Mark::Same {
            continue;
        }
        match out.last_mut() {
            Some(r) if r.end == k => {
                r.end = k + 1;
                r.mark = r.mark.merge(m);
            }
            _ => out.push(Run { start: k, end: k + 1, mark: m }),
        }
    }
    out
}

/// A run's place on the strip: `y0..y1` in pixels from its top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tick {
    pub y0: f32,
    pub y1: f32,
    pub mark: Mark,
}

/// The ticks of `runs` on a strip `track` tall for content `total` tall,
/// where row `k` starts `base + tops[k]` down the content. A tick is at
/// least `min` tall so that a one-line change can be seen and hit.
pub fn ticks(runs: &[Run], tops: &[f32], base: f32, total: f32, track: f32, min: f32) -> Vec<Tick> {
    if total <= 0.0 || track <= 0.0 {
        return Vec::new();
    }
    runs.iter()
        .filter(|r| r.end < tops.len())
        .map(|r| {
            let y0 = ((base + tops[r.start]) / total * track).min(track - min).max(0.0);
            let y1 = ((base + tops[r.end]) / total * track).max(y0 + min).min(track.max(min));
            Tick { y0, y1, mark: r.mark }
        })
        .collect()
}

/// The run `dir` (1 next, -1 previous) from the content position `at`:
/// `starts` are the runs' tops in order; `eps` keeps the run already there
/// from being found again. `None` when there is none that way.
pub fn jump(starts: &[f32], at: f32, dir: i32, eps: f32) -> Option<usize> {
    if dir > 0 {
        starts.iter().position(|&s| s > at + eps)
    } else {
        starts.iter().rposition(|&s| s < at - eps)
    }
}

/// The content position a click at `frac` (0 to 1) down the strip means:
/// the view's top, so that the point is in the middle of the view.
pub fn click_to_scroll(frac: f32, total: f32, view: f32) -> f32 {
    (frac.clamp(0.0, 1.0) * total - view / 2.0).clamp(0.0, (total - view).max(0.0))
}

/// The cursor's place among the changed lines (`(hunk, line)` in order)
/// after `n` / `N`: the first line of the next run of consecutive lines, or
/// of the cursor's own run when going back from the middle of it. Stays when
/// there is no such run.
pub fn step_line(changed: &[(usize, usize)], cursor: usize, dir: i32) -> usize {
    let start = |i: usize| {
        i == 0 || {
            let (a, b) = (changed[i - 1], changed[i]);
            !(a.0 == b.0 && a.1 + 1 == b.1)
        }
    };
    let n = changed.len();
    if dir > 0 {
        (cursor + 1..n).find(|&i| start(i)).unwrap_or(cursor)
    } else {
        (0..cursor.min(n)).rev().find(|&i| start(i)).unwrap_or(cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Mark::*;

    #[test]
    fn runs_join_consecutive_changed_rows_and_merge_their_marks() {
        assert!(runs(&[Same, Same]).is_empty());
        let r = runs(&[Same, Del, Add, Add, Same, Add, Same]);
        assert_eq!(r, [Run { start: 1, end: 4, mark: Both }, Run { start: 5, end: 6, mark: Add }]);
        assert_eq!(runs(&[Add, Add]), [Run { start: 0, end: 2, mark: Add }]);
    }

    #[test]
    fn ticks_scale_to_the_strip_and_are_never_thinner_than_min() {
        let tops = [0.0, 10.0, 20.0, 30.0, 40.0];
        let r = runs(&[Same, Add, Same, Del]);
        let t = ticks(&r, &tops, 0.0, 40.0, 20.0, 2.0);
        assert_eq!(t, [Tick { y0: 5.0, y1: 10.0, mark: Add }, Tick { y0: 15.0, y1: 20.0, mark: Del }]);
        let thin = ticks(&r, &tops, 0.0, 4000.0, 20.0, 2.0);
        assert!(thin.iter().all(|t| t.y1 - t.y0 >= 2.0 && t.y1 <= 20.0));
        assert!(ticks(&r, &tops, 0.0, 0.0, 20.0, 2.0).is_empty());
    }

    #[test]
    fn jump_finds_the_next_and_the_previous_run() {
        let s = [10.0, 50.0, 90.0];
        assert_eq!(jump(&s, 0.0, 1, 1.0), Some(0));
        assert_eq!(jump(&s, 10.0, 1, 1.0), Some(1));
        assert_eq!(jump(&s, 90.0, 1, 1.0), None);
        assert_eq!(jump(&s, 90.0, -1, 1.0), Some(1));
        assert_eq!(jump(&s, 10.0, -1, 1.0), None);
        assert_eq!(jump(&s, 60.0, -1, 1.0), Some(1));
        assert_eq!(jump(&[], 0.0, 1, 1.0), None);
    }

    #[test]
    fn a_click_centres_the_view_and_stays_inside_the_content() {
        assert_eq!(click_to_scroll(0.5, 1000.0, 200.0), 400.0);
        assert_eq!(click_to_scroll(0.0, 1000.0, 200.0), 0.0);
        assert_eq!(click_to_scroll(1.0, 1000.0, 200.0), 800.0);
        assert_eq!(click_to_scroll(0.5, 100.0, 200.0), 0.0);
    }

    #[test]
    fn step_line_moves_by_runs_of_consecutive_lines() {
        // Run A: (0,1)(0,2); run B: (1,0)(1,1); run C: (1,3).
        let c = [(0, 1), (0, 2), (1, 0), (1, 1), (1, 3)];
        assert_eq!(step_line(&c, 0, 1), 2);
        assert_eq!(step_line(&c, 1, 1), 2);
        assert_eq!(step_line(&c, 2, 1), 4);
        assert_eq!(step_line(&c, 4, 1), 4);
        assert_eq!(step_line(&c, 4, -1), 2);
        assert_eq!(step_line(&c, 3, -1), 2);
        assert_eq!(step_line(&c, 2, -1), 0);
        assert_eq!(step_line(&c, 0, -1), 0);
        assert_eq!(step_line(&[], 0, 1), 0);
    }
}
