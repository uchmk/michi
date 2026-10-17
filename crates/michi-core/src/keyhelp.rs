//! Keys that mean different things on different screens (`t` reverts a commit
//! in the graph, opens a terminal in the worktree table and takes theirs in a
//! conflict). The lists of keys are the window's; what is worked out from them
//! is here, with nothing of egui in it, so a test can hold the keybar and the
//! key list to what the keys do.

use std::collections::BTreeMap;

/// A screen's keys: `(keys as written, word-table key)`, as the keybar has them.
pub type Keys = [(&'static str, &'static str)];

/// The single keys a written `keys` stands for: `"j k"` is `j` and `k`.
pub fn tokens(keys: &str) -> impl Iterator<Item = &str> {
    keys.split_whitespace()
}

/// What `token` does on each screen that binds it: `(screen, word)`, in the
/// order the screens are given.
pub fn meanings<'a>(screens: &[(&'a str, &Keys)], token: &str) -> Vec<(&'a str, &'static str)> {
    let mut out = Vec::new();
    for (name, keys) in screens {
        for (written, word) in keys.iter() {
            if tokens(written).any(|t| t == token) {
                out.push((*name, *word));
            }
        }
    }
    out
}

/// The keys that mean more than one thing across `screens`, with what each
/// screen makes of them.
pub fn collisions<'a>(screens: &[(&'a str, &Keys)]) -> BTreeMap<String, Vec<(&'a str, &'static str)>> {
    let mut all: BTreeMap<String, Vec<(&'a str, &'static str)>> = BTreeMap::new();
    for (name, keys) in screens {
        for (written, word) in keys.iter() {
            for t in tokens(written) {
                all.entry(t.to_owned()).or_default().push((*name, *word));
            }
        }
    }
    all.retain(|_, v| v.len() > 1 && v.iter().any(|(s, _)| *s != v[0].0));
    all
}

/// The keys a screen lists twice: one key cannot mean two things at once on a
/// screen, so a keybar that says it does is wrong.
pub fn repeated(keys: &Keys) -> Vec<&'static str> {
    let mut seen = Vec::new();
    let mut again = Vec::new();
    for (written, _) in keys.iter() {
        for t in tokens(written) {
            if seen.contains(&t) {
                again.push(t);
            } else {
                seen.push(t);
            }
        }
    }
    again
}

/// The keys that work everywhere, less those a screen takes for itself (its
/// own keys are tried first, so `p` is the pull request in the worktree table,
/// not a pull): `(keys still working, word)`. A row left with no key is gone.
pub fn global_here(global: &Keys, screen: &Keys) -> Vec<(String, &'static str)> {
    let taken: Vec<&str> = screen.iter().flat_map(|(k, _)| tokens(k)).collect();
    global
        .iter()
        .filter_map(|(written, word)| {
            let left: Vec<&str> = tokens(written).filter(|t| !taken.contains(t)).collect();
            (!left.is_empty()).then(|| (left.join(" "), *word))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRAPH: &Keys = &[("j k", "move"), ("t", "revert"), ("T", "tag")];
    const TABLE: &Keys = &[("j k", "select"), ("t e", "open"), ("p", "pr")];
    const GLOBAL: &Keys = &[("1 2", "views"), ("f", "fetch"), ("p", "pull"), ("P F", "push")];

    #[test]
    fn a_key_is_listed_with_what_each_screen_makes_of_it() {
        let screens = [("graph", GRAPH), ("table", TABLE)];
        assert_eq!(meanings(&screens, "t"), vec![("graph", "revert"), ("table", "open")]);
        assert_eq!(meanings(&screens, "T"), vec![("graph", "tag")]);
        assert_eq!(meanings(&screens, "z"), vec![]);
        let c = collisions(&screens);
        assert_eq!(c.keys().map(String::as_str).collect::<Vec<_>>(), ["j", "k", "t"]);
    }

    #[test]
    fn a_screen_that_lists_a_key_twice_is_found() {
        assert!(repeated(GRAPH).is_empty());
        assert_eq!(repeated(&[("j k", "a"), ("k", "b")]), vec!["k"]);
    }

    #[test]
    fn a_global_key_a_screen_takes_is_not_listed_as_working_there() {
        let g = global_here(GLOBAL, TABLE);
        assert_eq!(g, vec![("1 2".to_owned(), "views"), ("f".to_owned(), "fetch"), ("P F".to_owned(), "push")]);
        assert_eq!(global_here(GLOBAL, GRAPH).len(), 4);
        assert_eq!(global_here(&[("p P", "x")], &[("p", "y")]), vec![("P".to_owned(), "x")]);
    }
}
