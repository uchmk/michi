//! `[view]` of michi's config.toml: `density = "compact" | "standard" | "cozy"`
//! (how tall the graph's and the lists' rows are, issue #86). Anything else
//! is warned of and keeps the standard.

use michi_core::density::Density;
use michi_core::motion::Motion;
use serde::Deserialize;

#[derive(Deserialize, Default)]
#[serde(default)]
struct View {
    density: Option<String>,
    motion: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct File {
    view: View,
}

/// `[view] density` of the config.toml `text` (none: no file), with what to warn of.
pub fn read(text: Option<&str>) -> (Density, Vec<String>) {
    match text.map(toml::from_str::<File>) {
        Some(Ok(f)) => match f.view.density {
            None => (Density::default(), Vec::new()),
            Some(s) => match s.parse() {
                Ok(d) => (d, Vec::new()),
                Err(()) => (Density::default(), vec![format!("config.toml: [view] density `{s}` is not compact, standard or cozy")]),
            },
        },
        // The same file is read for [open] and [notify], which say what is wrong with it.
        _ => (Density::default(), Vec::new()),
    }
}

/// `[view] motion` of the config.toml `text` (none: no file), with what to warn of.
pub fn read_motion(text: Option<&str>) -> (Motion, Vec<String>) {
    match text.map(toml::from_str::<File>) {
        Some(Ok(f)) => match f.view.motion {
            None => (Motion::default(), Vec::new()),
            Some(s) => match s.parse() {
                Ok(m) => (m, Vec::new()),
                Err(()) => (Motion::default(), vec![format!("config.toml: [view] motion `{s}` is not on or off")]),
            },
        },
        _ => (Motion::default(), Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn density_is_read_or_defaulted() {
        assert_eq!(read(None), (Density::Standard, vec![]));
        assert_eq!(read(Some("[view]\ndensity = \"compact\"\n")).0, Density::Compact);
        assert_eq!(read(Some("[view]\ndensity = \"Cozy\"\n")).0, Density::Cozy);
        assert_eq!(read(Some("[open]\neditor = \"x\"\n")), (Density::Standard, vec![]));
    }

    #[test]
    fn a_bad_word_is_warned_of() {
        let (d, w) = read(Some("[view]\ndensity = \"huge\"\n"));
        assert_eq!(d, Density::Standard);
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn motion_is_read_or_defaulted() {
        assert_eq!(read_motion(None), (Motion::On, vec![]));
        assert_eq!(
            read_motion(Some(
                "[view]
motion = \"off\"
"
            ))
            .0,
            Motion::Off
        );
        assert_eq!(
            read_motion(Some(
                "[view]
density = \"cozy\"
"
            )),
            (Motion::On, vec![])
        );
        let (m, w) = read_motion(Some(
            "[view]
motion = \"slow\"
",
        ));
        assert_eq!((m, w.len()), (Motion::On, 1));
    }
}
