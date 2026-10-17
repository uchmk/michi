//! Short animations (issue #88): a view or an overlay that appears fades in
//! and slides a few pixels into its place in about 130 ms. `[view] motion =
//! "on" | "off"` in config.toml turns them off (reduced motion).

use std::str::FromStr;

/// How long a fade-and-slide takes, in seconds (between 100 and 150 ms).
pub const DURATION: f64 = 0.13;
/// How far a view or an overlay slides, in px.
pub const SLIDE_PX: f32 = 8.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Motion {
    #[default]
    On,
    Off,
}

impl Motion {
    pub const ALL: [Motion; 2] = [Motion::On, Motion::Off];

    /// The word config.toml uses.
    pub fn key(self) -> &'static str {
        match self {
            Motion::On => "on",
            Motion::Off => "off",
        }
    }

    pub fn is_on(self) -> bool {
        self == Motion::On
    }

    /// How far along (0.0 to 1.0, eased out) a motion that began `elapsed`
    /// seconds ago is. Off: it has always arrived.
    pub fn progress(self, elapsed: f64) -> f32 {
        if !self.is_on() {
            return 1.0;
        }
        let t = (elapsed / DURATION).clamp(0.0, 1.0) as f32;
        1.0 - (1.0 - t) * (1.0 - t)
    }

    /// How far a thing at `progress` still has to slide: `SLIDE_PX` at the
    /// start, 0 on arrival (and always 0 when off).
    pub fn offset(self, progress: f32) -> f32 {
        if self.is_on() {
            (1.0 - progress.clamp(0.0, 1.0)) * SLIDE_PX
        } else {
            0.0
        }
    }

    /// egui's own animation time (the fade of a window that opens), in seconds.
    pub fn animation_time(self) -> f32 {
        if self.is_on() {
            DURATION as f32
        } else {
            0.0
        }
    }
}

impl FromStr for Motion {
    type Err = ();

    /// Case-insensitive; `true`/`yes` mean on, `false`/`no`/`none`/`reduced` mean off.
    fn from_str(s: &str) -> Result<Self, ()> {
        match s.trim().to_ascii_lowercase().as_str() {
            "on" | "true" | "yes" | "full" => Ok(Motion::On),
            "off" | "false" | "no" | "none" | "reduced" => Ok(Motion::Off),
            _ => Err(()),
        }
    }
}

/// Whether a thing drawn in pass `now` goes on from pass `last` (seen in the
/// last pass or this one) or has just appeared (it was away for a pass).
pub fn continues(last: Option<u64>, now: u64) -> bool {
    matches!(last, Some(l) if l == now || l + 1 == now)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_motion_runs_from_nothing_to_arrived() {
        assert_eq!(Motion::On.progress(0.0), 0.0);
        assert_eq!(Motion::On.progress(DURATION), 1.0);
        assert_eq!(Motion::On.progress(5.0), 1.0);
        assert_eq!(Motion::On.progress(-1.0), 0.0);
        let half = Motion::On.progress(DURATION / 2.0);
        assert!(half > 0.5 && half < 1.0, "eased out: more than half way at half the time");
    }

    #[test]
    fn progress_never_goes_back() {
        let mut last = 0.0;
        for k in 0..=20 {
            let p = Motion::On.progress(DURATION * f64::from(k) / 20.0);
            assert!(p >= last);
            last = p;
        }
    }

    #[test]
    fn off_arrives_at_once_and_does_not_slide() {
        assert_eq!(Motion::Off.progress(0.0), 1.0);
        assert_eq!(Motion::Off.offset(0.0), 0.0);
        assert_eq!(Motion::Off.animation_time(), 0.0);
    }

    #[test]
    fn the_slide_shrinks_to_nothing() {
        assert_eq!(Motion::On.offset(0.0), SLIDE_PX);
        assert_eq!(Motion::On.offset(1.0), 0.0);
        assert!(Motion::On.offset(0.5) < SLIDE_PX);
        assert!(Motion::On.animation_time() > 0.1 && Motion::On.animation_time() < 0.15);
        assert!((0.1..=0.15).contains(&DURATION));
    }

    #[test]
    fn words_round_trip() {
        for m in Motion::ALL {
            assert_eq!(m.key().parse(), Ok(m));
        }
        assert_eq!(" OFF ".parse(), Ok(Motion::Off));
        assert_eq!("reduced".parse(), Ok(Motion::Off));
        assert!("slow".parse::<Motion>().is_err());
        assert_eq!(Motion::default(), Motion::On);
    }

    #[test]
    fn a_thing_that_was_away_starts_again() {
        assert!(!continues(None, 5));
        assert!(continues(Some(5), 5));
        assert!(continues(Some(4), 5));
        assert!(!continues(Some(3), 5));
    }
}
