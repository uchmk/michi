//! Automatic fetch (issue #70): while the window is in front, fetch every few
//! minutes (`[net] auto_fetch` in config.toml, in minutes, 0 is off). This is
//! only the timing; the fetch itself is the same job as `f`.

use std::time::Duration;

/// Minutes between automatic fetches when config.toml says nothing.
pub const DEFAULT_MINUTES: u64 = 5;
/// A failure doubles the wait, up to this many doublings (8 times the interval).
const MAX_DOUBLINGS: u32 = 3;

/// The wait between fetches: `None` is off.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Every(pub Option<Duration>);

impl Default for Every {
    fn default() -> Self {
        Every::minutes(DEFAULT_MINUTES)
    }
}

impl Every {
    /// `minutes` apart; 0 is off.
    pub fn minutes(minutes: u64) -> Every {
        Every((minutes > 0).then(|| Duration::from_secs(minutes.saturating_mul(60))))
    }

    /// Whether a fetch is due: `since` is the time since the later of the last
    /// fetch and the last try (none: never), `failures` the tries in a row that
    /// failed (the wait grows with them, so a lost network is not hammered),
    /// `front` and `busy` whether the window is in front and a net job runs.
    pub fn due(self, since: Option<Duration>, failures: u32, front: bool, busy: bool) -> bool {
        let Some(every) = self.0 else { return false };
        if !front || busy {
            return false;
        }
        match since {
            None => true,
            Some(s) => s >= every * 2u32.pow(failures.min(MAX_DOUBLINGS)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(n: u64) -> Option<Duration> {
        Some(Duration::from_secs(n))
    }

    #[test]
    fn zero_minutes_is_off() {
        assert_eq!(Every::minutes(0), Every(None));
        assert!(!Every::minutes(0).due(None, 0, true, false));
        assert_eq!(Every::default(), Every::minutes(5));
    }

    #[test]
    fn it_waits_for_the_interval() {
        let e = Every::minutes(5);
        assert!(e.due(None, 0, true, false));
        assert!(!e.due(secs(299), 0, true, false));
        assert!(e.due(secs(300), 0, true, false));
    }

    #[test]
    fn it_waits_behind_and_while_busy() {
        let e = Every::minutes(5);
        assert!(!e.due(secs(9999), 0, false, false));
        assert!(!e.due(secs(9999), 0, true, true));
    }

    #[test]
    fn failures_stretch_the_wait_to_eight_times() {
        let e = Every::minutes(1);
        assert!(!e.due(secs(119), 1, true, false));
        assert!(e.due(secs(120), 1, true, false));
        assert!(!e.due(secs(479), 3, true, false));
        assert!(e.due(secs(480), 3, true, false));
        assert!(!e.due(secs(479), 20, true, false));
        assert!(e.due(secs(480), 20, true, false));
    }
}
