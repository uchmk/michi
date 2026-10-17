//! How the lists write a commit's date: `relative` (the time today, `yesterday`
//! and the time, the month and day) or `full` (the default; always the whole date and the
//! time, in the clock's format: `2026/10/05 14:32`). `[view] dates` in config.toml.

use std::str::FromStr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Dates {
    Relative,
    #[default]
    Full,
}

impl Dates {
    pub const ALL: [Dates; 2] = [Dates::Relative, Dates::Full];

    /// The word config.toml uses.
    pub fn key(self) -> &'static str {
        match self {
            Dates::Relative => "relative",
            Dates::Full => "full",
        }
    }
}

impl FromStr for Dates {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        match s.trim().to_ascii_lowercase().as_str() {
            "relative" => Ok(Dates::Relative),
            "full" => Ok(Dates::Full),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_round_trip() {
        for d in Dates::ALL {
            assert_eq!(d.key().parse(), Ok(d));
        }
        assert!("never".parse::<Dates>().is_err());
        assert_eq!(Dates::default(), Dates::Full);
    }
}
