//! `[view]` of michi's config.toml: `density = "compact" | "standard" | "cozy"`
//! (how tall the graph's and the lists' rows are, issue #86). Anything else
//! is warned of and keeps the standard.

use michi_core::autofetch::Every;
use michi_core::density::Density;
use michi_core::motion::Motion;
use serde::Deserialize;

#[derive(Deserialize, Default)]
#[serde(default)]
struct View {
    density: Option<String>,
    motion: Option<String>,
    avatars: Option<toml::Value>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Net {
    auto_fetch: Option<toml::Value>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct File {
    view: View,
    net: Net,
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

/// `[view] avatars` (true unless the file says `false`), with what to warn of.
pub fn avatars(text: Option<&str>) -> (bool, Vec<String>) {
    match text.map(toml::from_str::<File>) {
        Some(Ok(File { view: View { avatars: Some(v), .. }, .. })) => match v.as_bool() {
            Some(b) => (b, Vec::new()),
            None => (true, vec!["config.toml: [view] avatars is not true or false".to_owned()]),
        },
        _ => (true, Vec::new()),
    }
}

/// `[net] auto_fetch` (minutes between automatic fetches, 0 is off; 5 when
/// not written), with what to warn of.
pub fn auto_fetch(text: Option<&str>) -> (Every, Vec<String>) {
    match text.map(toml::from_str::<File>) {
        Some(Ok(File { net: Net { auto_fetch: Some(v) }, .. })) => match v.as_integer().and_then(|n| u64::try_from(n).ok()) {
            Some(n) => (Every::minutes(n), Vec::new()),
            None => (Every::default(), vec!["config.toml: [net] auto_fetch is not a number of minutes (0 turns it off)".to_owned()]),
        },
        _ => (Every::default(), Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_fetch_is_five_minutes_unless_written() {
        assert_eq!(auto_fetch(None), (Every::minutes(5), vec![]));
        assert_eq!(
            auto_fetch(Some(
                "[view]
density = \"cozy\"
"
            ))
            .0,
            Every::minutes(5)
        );
        assert_eq!(
            auto_fetch(Some(
                "[net]
auto_fetch = 10
"
            ))
            .0,
            Every::minutes(10)
        );
        assert_eq!(
            auto_fetch(Some(
                "[net]
auto_fetch = 0
"
            ))
            .0,
            Every(None)
        );
        for bad in ["\"soon\"", "-3", "true"] {
            let (e, warn) = auto_fetch(Some(&format!(
                "[net]
auto_fetch = {bad}
"
            )));
            assert!(e == Every::minutes(5) && warn.len() == 1, "{bad}");
        }
    }

    #[test]
    fn avatars_are_on_unless_turned_off() {
        assert_eq!(avatars(None), (true, vec![]));
        assert_eq!(
            avatars(Some(
                "[view]
density = \"cozy\"
"
            )),
            (true, vec![])
        );
        assert_eq!(
            avatars(Some(
                "[view]
avatars = false
"
            )),
            (false, vec![])
        );
        assert!(
            avatars(Some(
                "[view]
avatars = true
density = \"cozy\"
"
            ))
            .0
        );
        let (on, warn) = avatars(Some(
            "[view]
avatars = \"no\"
",
        ));
        assert!(on && warn.len() == 1);
        // A bad value for avatars does not hide the density.
        assert_eq!(
            read(Some(
                "[view]
avatars = 3
density = \"cozy\"
"
            ))
            .0,
            Density::Cozy
        );
    }

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
