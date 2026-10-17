//! `[view]` of michi's config.toml: `density = "compact" | "standard" | "cozy"`
//! (how tall the graph's and the lists' rows are, issue #86). Anything else
//! is warned of and keeps the standard.

use michi_core::autofetch::Every;
use michi_core::coauthor::CoAuthor;
use michi_core::density::Density;
use michi_core::level::Level;
use michi_core::motion::Motion;
use serde::Deserialize;

#[derive(Deserialize, Default)]
#[serde(default)]
struct View {
    density: Option<String>,
    motion: Option<String>,
    level: Option<String>,
    avatars: Option<toml::Value>,
    easy: Option<toml::Value>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Net {
    auto_fetch: Option<toml::Value>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Commit {
    co_author: Option<toml::Value>,
    protect: Option<toml::Value>,
    large_mb: Option<toml::Value>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Issues {
    github: Option<toml::Value>,
    keys: Option<toml::Value>,
    prefixes: Option<toml::Value>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct File {
    issues: Issues,
    view: View,
    net: Net,
    commit: Commit,
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

/// `[view] level = "basic" | "full"` (full when not written), with what to warn of.
pub fn read_level(text: Option<&str>) -> (Level, Vec<String>) {
    match text.map(toml::from_str::<File>) {
        Some(Ok(f)) => match f.view.level {
            None => (Level::default(), Vec::new()),
            Some(s) => match s.parse() {
                Ok(l) => (l, Vec::new()),
                Err(()) => (Level::default(), vec![format!("config.toml: [view] level `{s}` is not basic or full")]),
            },
        },
        _ => (Level::default(), Vec::new()),
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

/// `[view] easy` (false unless the file says `true`): the plain wording of
/// Git's terms (#104), with what to warn of.
pub fn easy(text: Option<&str>) -> (bool, Vec<String>) {
    match text.map(toml::from_str::<File>) {
        Some(Ok(File { view: View { easy: Some(v), .. }, .. })) => match v.as_bool() {
            Some(b) => (b, Vec::new()),
            None => (false, vec!["config.toml: [view] easy is not true or false".to_owned()]),
        },
        _ => (false, Vec::new()),
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

/// `[commit] co_author = "Name <email>"`: the trailer added to michi's commits
/// (none when not written), with what to warn of.
pub fn co_author(text: Option<&str>) -> (CoAuthor, Vec<String>) {
    match text.map(toml::from_str::<File>) {
        Some(Ok(File { commit: Commit { co_author: Some(v), .. }, .. })) => match v.as_str().map(CoAuthor::parse) {
            Some(Some(c)) => (c, Vec::new()),
            _ => (CoAuthor::default(), vec!["config.toml: [commit] co_author is not \"Name <email>\"".to_owned()]),
        },
        _ => (CoAuthor::default(), Vec::new()),
    }
}

/// `[commit] protect = ["main", "release/*"]` (the branches a commit asks about;
/// `[]` for none) and `large_mb = 50` (0 for never), with what to warn of (#110, #111).
pub fn guard(text: Option<&str>) -> (michi_core::guard::Config, Vec<String>) {
    let mut cfg = michi_core::guard::Config::default();
    let mut warn = Vec::new();
    let Some(Ok(File { commit: c, .. })) = text.map(toml::from_str::<File>) else { return (cfg, warn) };
    if let Some(v) = c.protect {
        let list: Option<Vec<String>> = v.as_array().and_then(|a| a.iter().map(|x| x.as_str().map(str::to_owned)).collect());
        match list {
            Some(l) => cfg.protect = l,
            None => warn.push("config.toml: [commit] protect is not a list of branch names".to_owned()),
        }
    }
    if let Some(v) = c.large_mb {
        match v.as_integer().filter(|n| *n >= 0) {
            Some(n) => cfg.large = (n as u64).saturating_mul(1024 * 1024),
            None => warn.push("config.toml: [commit] large_mb is not a number of MiB (0 or more)".to_owned()),
        }
    }
    (cfg, warn)
}

/// `[issues]`: `github = false` stops `#123` linking to the GitHub remote,
/// `keys = "https://tracker.example/{key}"` says where `ABC-123` goes (Linear, Jira), and
/// `prefixes = ["ABC"]` lists the projects that count. With what to warn of.
pub fn issues(text: Option<&str>) -> (michi_core::issues::Config, Vec<String>) {
    let mut cfg = michi_core::issues::Config::default();
    let mut warn = Vec::new();
    let Some(Ok(File { issues: i, .. })) = text.map(toml::from_str::<File>) else { return (cfg, warn) };
    if let Some(v) = i.github {
        match v.as_bool() {
            Some(b) => cfg.github = b,
            None => warn.push("config.toml: [issues] github is not true or false".to_owned()),
        }
    }
    if let Some(v) = i.keys {
        match v.as_str().filter(|t| michi_core::issues::valid_template(t)) {
            Some(t) => cfg.keys = Some(t.to_owned()),
            None => warn.push("config.toml: [issues] keys is not an http(s) address with {key} in it".to_owned()),
        }
    }
    if let Some(v) = i.prefixes {
        let list: Option<Vec<String>> = v.as_array().and_then(|a| a.iter().map(|x| x.as_str().map(str::to_owned)).collect());
        match list {
            Some(l) => cfg.prefixes = l,
            None => warn.push("config.toml: [issues] prefixes is not a list of words".to_owned()),
        }
    }
    (cfg, warn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issues_default_to_github_and_read_the_template() {
        assert_eq!(issues(None), (michi_core::issues::Config::default(), vec![]));
        let (c, w) = issues(Some(
            "[issues]
github = false
keys = \"https://linear.app/a/issue/{key}\"
prefixes = [\"ENG\"]
",
        ));
        assert!(w.is_empty(), "{w:?}");
        assert!(!c.github);
        assert_eq!(c.keys.as_deref(), Some("https://linear.app/a/issue/{key}"));
        assert_eq!(c.prefixes, ["ENG"]);
        let (c, w) = issues(Some(
            "[issues]
keys = \"nonsense\"
github = 3
prefixes = 1
",
        ));
        assert_eq!(w.len(), 3, "{w:?}");
        assert_eq!(c, michi_core::issues::Config::default());
    }

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
    fn co_author_is_none_unless_written_well() {
        assert_eq!(co_author(None), (CoAuthor::default(), vec![]));
        let (c, w) = co_author(Some(
            "[commit]
co_author = \"Claude <noreply@anthropic.com>\"
",
        ));
        assert_eq!((c.value(), w.len()), (Some("Claude <noreply@anthropic.com>"), 0));
        for bad in ["\"Claude\"", "3", "true"] {
            let (c, w) = co_author(Some(&format!(
                "[commit]
co_author = {bad}
"
            )));
            assert!(!c.is_set() && w.len() == 1, "{bad}");
        }
    }

    #[test]
    fn the_guard_has_defaults_and_reads_protect_and_large_mb() {
        assert_eq!(guard(None), (michi_core::guard::Config::default(), vec![]));
        let (g, w) = guard(Some(
            "[commit]
protect = [\"main\", \"release/*\"]
large_mb = 10
",
        ));
        assert_eq!((g.protect, g.large, w.len()), (vec!["main".to_owned(), "release/*".to_owned()], 10 << 20, 0));
        let (g, _) = guard(Some(
            "[commit]
protect = []
large_mb = 0
",
        ));
        assert_eq!((g.protect.len(), g.large), (0, 0));
        let (g, w) = guard(Some(
            "[commit]
protect = \"main\"
large_mb = -1
",
        ));
        assert_eq!((g, w.len()), (michi_core::guard::Config::default(), 2));
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
    fn easy_is_off_unless_the_file_says_true() {
        assert_eq!(easy(None), (false, vec![]));
        assert_eq!(easy(Some("[view]\ndensity = \"cozy\"\n")), (false, vec![]));
        assert_eq!(easy(Some("[view]\neasy = true\n")), (true, vec![]));
        let (on, warn) = easy(Some("[view]\neasy = \"yes\"\n"));
        assert!(!on);
        assert_eq!(warn.len(), 1);
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
    fn level_is_full_unless_written() {
        assert_eq!(read_level(None), (Level::Full, vec![]));
        assert_eq!(read_level(Some("[view]\nlevel = \"basic\"\n")).0, Level::Basic);
        assert_eq!(read_level(Some("[view]\ndensity = \"cozy\"\n")), (Level::Full, vec![]));
        let (l, w) = read_level(Some("[view]\nlevel = \"expert\"\n"));
        assert_eq!((l, w.len()), (Level::Full, 1));
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
