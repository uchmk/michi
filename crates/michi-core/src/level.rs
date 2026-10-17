//! How much of michi is shown (issue #115): `basic` keeps the keybar and the
//! key list to the everyday keys, `full` shows them all. In basic the
//! rarely-needed and the destructive keys (reset, rebase, discard, force push,
//! bisect and the like) are left out of the keybar and `?`, and stay in the
//! command palette; every key works in both. `[view] level` in config.toml.

use std::str::FromStr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Level {
    Basic,
    #[default]
    Full,
}

/// The word-table keys of the keys basic leaves out of the keybar and `?`.
pub const ADVANCED: &[&str] = &[
    // The graph.
    "key.revert",
    "key.reset",
    "key.cherry_pick",
    "key.merge_commit",
    "key.compare",
    "key.copy_patch",
    "key.apply_patch",
    "key.bisect_mark",
    "key.bisect_skip",
    "key.bisect_run",
    "key.bisect_reset",
    "key.image_mode",
    "key.ignore_space",
    // The changes.
    "key.split",
    "key.discard",
    "key.amend",
    "key.ext_diff",
    // The worktrees.
    "key.reorder",
    "key.rebase_trunk",
    "key.remove",
    "key.tidy",
    "key.trial",
    "key.turns",
    "key.submodules",
    "key.move_changes",
    "key.flow",
    "key.reflog",
    "key.stats",
    // The conflicts.
    "key.cf_strip",
    "key.cf_external",
    "key.cf_forget",
    // Every view.
    "key.push_over",
];

impl Level {
    pub const ALL: [Level; 2] = [Level::Basic, Level::Full];

    /// The word config.toml uses.
    pub fn key(self) -> &'static str {
        match self {
            Level::Basic => "basic",
            Level::Full => "full",
        }
    }

    /// Whether the key whose word is `word` is listed in the keybar and `?`.
    pub fn lists(self, word: &str) -> bool {
        self == Level::Full || !ADVANCED.contains(&word)
    }

    /// `keys` as this level lists them.
    pub fn visible<'a>(self, keys: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
        keys.iter().filter(|(_, w)| self.lists(w)).copied().collect()
    }

    /// How many of `keys` this level leaves out.
    pub fn hidden(self, keys: &[(&str, &str)]) -> usize {
        keys.iter().filter(|(_, w)| !self.lists(w)).count()
    }
}

impl FromStr for Level {
    type Err = ();

    /// Case-insensitive; `beginner` and `simple` mean basic, `all` means full.
    fn from_str(s: &str) -> Result<Self, ()> {
        match s.trim().to_ascii_lowercase().as_str() {
            "basic" | "beginner" | "simple" => Ok(Level::Basic),
            "full" | "all" | "advanced" => Ok(Level::Full),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEYS: [(&str, &str); 4] = [("j k", "key.move"), ("R", "key.reset"), ("c", "key.commit"), ("d", "key.discard")];

    #[test]
    fn full_lists_everything_and_basic_leaves_the_risky_ones_out() {
        assert_eq!(Level::Full.visible(&KEYS), KEYS);
        assert_eq!(Level::Basic.visible(&KEYS), [("j k", "key.move"), ("c", "key.commit")]);
        assert_eq!(Level::Full.hidden(&KEYS), 0);
        assert_eq!(Level::Basic.hidden(&KEYS), 2);
    }

    #[test]
    fn the_everyday_keys_are_never_advanced() {
        for w in ["key.move", "key.commit", "key.stage", "key.fetch", "key.pull", "key.push", "key.new_work", "key.back", "key.land"] {
            assert!(Level::Basic.lists(w), "{w}");
        }
        for w in ["key.reset", "key.discard", "key.push_over", "key.reorder"] {
            assert!(!Level::Basic.lists(w), "{w}");
        }
    }

    #[test]
    fn the_list_has_no_duplicates() {
        let mut seen = std::collections::HashSet::new();
        for w in ADVANCED {
            assert!(seen.insert(*w), "{w}");
        }
    }

    #[test]
    fn words_round_trip() {
        for l in Level::ALL {
            assert_eq!(l.key().parse(), Ok(l));
        }
        assert_eq!(" Basic ".parse(), Ok(Level::Basic));
        assert_eq!("all".parse(), Ok(Level::Full));
        assert!("expert".parse::<Level>().is_err());
        assert_eq!(Level::default(), Level::Full);
    }
}
