//! Row density (issue #86): how tall the graph's and the lists' rows are.
//! Three steps, `compact`, `standard` (the default, the sizes michi had
//! before) and `cozy`, set by `[view] density` in config.toml.

use std::str::FromStr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Density {
    Compact,
    #[default]
    Standard,
    Cozy,
}

impl Density {
    pub const ALL: [Density; 3] = [Density::Compact, Density::Standard, Density::Cozy];

    /// The word config.toml uses.
    pub fn key(self) -> &'static str {
        match self {
            Density::Compact => "compact",
            Density::Standard => "standard",
            Density::Cozy => "cozy",
        }
    }

    /// A row that is `base` px high at the standard step, at this one: about
    /// 80% for compact and 120% for cozy, in whole pixels and never under 14.
    pub fn px(self, base: f32) -> f32 {
        let f = match self {
            Density::Compact => 0.8,
            Density::Standard => 1.0,
            Density::Cozy => 1.2,
        };
        (base * f).round().max(14.0)
    }
}

impl FromStr for Density {
    type Err = ();

    /// Case-insensitive; `normal` and `default` mean standard.
    fn from_str(s: &str) -> Result<Self, ()> {
        match s.trim().to_ascii_lowercase().as_str() {
            "compact" => Ok(Density::Compact),
            "standard" | "normal" | "default" => Ok(Density::Standard),
            "cozy" => Ok(Density::Cozy),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_keeps_the_old_sizes() {
        for base in [22.0, 23.0, 24.0, 26.0] {
            assert_eq!(Density::Standard.px(base), base);
        }
    }

    #[test]
    fn the_steps_are_ordered() {
        for base in [22.0, 23.0, 26.0] {
            assert!(Density::Compact.px(base) < Density::Standard.px(base));
            assert!(Density::Standard.px(base) < Density::Cozy.px(base));
        }
        assert_eq!(Density::Compact.px(26.0), 21.0);
        assert_eq!(Density::Cozy.px(26.0), 31.0);
        assert_eq!(Density::Compact.px(10.0), 14.0, "never squeezed under a text line");
    }

    #[test]
    fn words_round_trip() {
        for d in Density::ALL {
            assert_eq!(d.key().parse(), Ok(d));
        }
        assert_eq!(" Cozy ".parse(), Ok(Density::Cozy));
        assert_eq!("normal".parse(), Ok(Density::Standard));
        assert!("huge".parse::<Density>().is_err());
        assert_eq!(Density::default(), Density::Standard);
    }
}
