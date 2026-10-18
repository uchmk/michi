//! The keymap, laid out like yazi's, kura's and yagura's: a section per
//! layer, each with `keymap` (replaces the built-in list), `prepend_keymap`
//! and `append_keymap`; the first binding that matches wins. Read by
//! `ito-keymap`. Only the keys every uchmk app has are in it so far (the
//! settings, the keys, the palette, quit and the scale); the views' own keys
//! are still `App::feed`'s.

use ito_common::ScaleTo;
use ito_keys::Key;

pub use ito_keymap::resolve;

pub type Binding = ito_keymap::Binding<Act>;
pub type Match<'a> = ito_keymap::Match<'a, Act>;

const DEFAULTS: &str = include_str!("../defaults/keymap.toml");

/// What a key in the `main` layer does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    /// Open the settings, or close them.
    Settings,
    /// Open the key list, or close it.
    Help,
    Palette,
    /// Open the Git flow sheet, or close it.
    Cheat,
    /// Open the repository screen, or close it.
    Repos,
    /// Open the bug report form in the browser.
    BugReport,
    /// The tab to the right, or to the left.
    NextTab,
    PrevTab,
    /// Close the tab in front.
    CloseTab,
    /// The nth tab (from 1) to the front.
    Tab(usize),
    Quit,
    /// Take back the last move of a branch, or do it again (#64).
    Undo,
    Redo,
    Scale(ScaleTo),
    /// Does nothing: a key taken away from a built-in operation.
    Noop,
}

/// The command a `run` word names, for the keymap's reader.
pub fn parse(s: &str) -> Result<Act, String> {
    let mut words = s.split_whitespace();
    let act = match (words.next(), words.next()) {
        (Some("settings"), None) => Act::Settings,
        (Some("help"), None) => Act::Help,
        (Some("palette"), None) => Act::Palette,
        (Some("cheat"), None) => Act::Cheat,
        (Some("repos"), None) => Act::Repos,
        (Some("bugreport"), None) => Act::BugReport,
        (Some("nexttab"), None) => Act::NextTab,
        (Some("prevtab"), None) => Act::PrevTab,
        (Some("closetab"), None) => Act::CloseTab,
        (Some("tab"), Some(n)) => Act::Tab(n.parse::<usize>().ok().filter(|n| *n >= 1).ok_or_else(|| format!("`{s}` is not a command"))?),
        (Some("quit"), None) => Act::Quit,
        (Some("undo"), None) => Act::Undo,
        (Some("redo"), None) => Act::Redo,
        (Some("noop"), None) => Act::Noop,
        (Some("scale"), Some("in")) => Act::Scale(ScaleTo::In),
        (Some("scale"), Some("out")) => Act::Scale(ScaleTo::Out),
        (Some("scale"), Some("reset")) => Act::Scale(ScaleTo::Reset),
        _ => return Err(format!("`{s}` is not a command")),
    };
    if words.next().is_some() {
        return Err(format!("`{s}` is not a command"));
    }
    Ok(act)
}

/// A button's tooltip: its name and, when it has one, its key in brackets.
pub fn tip(word: &str, key: Option<&str>) -> String {
    match key {
        Some(k) => format!("{word} ({k})"),
        None => word.to_owned(),
    }
}

pub(crate) const SPEC: ito_keymap::Spec = ito_keymap::Spec { layers: &[ito_keymap::Layer::new("main")], ignored: &[], defaults: DEFAULTS };

#[derive(Default, Debug)]
pub struct Keymap {
    pub main: Vec<Binding>,
}

impl Keymap {
    /// The built-in keymap with the user's file (its text, if there is one)
    /// laid over it. Problems come back as warnings; a bad binding is
    /// skipped.
    pub fn load(user: Option<&str>) -> (Self, Vec<String>) {
        let mut command = |s: &str, warn: &mut Vec<String>| match parse(s) {
            Ok(a) => Some(a),
            Err(e) => {
                warn.push(format!("keymap: {e}"));
                None
            }
        };
        let files: Vec<(&str, &str)> = user.map(|t| ("keymap.toml", t)).into_iter().collect();
        let mut km = ito_keymap::load(&SPEC, &files, &mut command);
        (Self { main: km.take("main") }, km.warnings)
    }

    /// The key that runs `act`, spelled for a reader (`Ctrl+Shift+P`): the
    /// first binding in the main layer that is just that command. For a
    /// button's tooltip, so a changed keymap changes the hint.
    pub fn hint(&self, act: Act) -> Option<String> {
        let b = self.main.iter().find(|b| b.run.len() == 1 && b.run[0] == act)?;
        Some(ito_keys::human_seq(&b.on))
    }

    /// Every layer's bindings, by name in [`SPEC`]'s order: what the Keys
    /// page lists.
    pub fn layers(&self) -> Vec<(String, Vec<Binding>)> {
        vec![("main".to_owned(), self.main.clone())]
    }

    /// The built-in defaults alone, layer by layer, read once.
    pub fn built_in_layers() -> &'static [(String, Vec<Binding>)] {
        static BUILT_IN_LAYERS: std::sync::OnceLock<Vec<(String, Vec<Binding>)>> = std::sync::OnceLock::new();
        BUILT_IN_LAYERS.get_or_init(|| Self::load(None).0.layers())
    }

    /// What `key` does in the main layer, if it is bound to one command.
    pub fn act(&self, key: Key) -> Option<Act> {
        match resolve(&self.main, &[key]) {
            Match::Exact(b) => b.run.first().copied(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(s: &str) -> Key {
        Key::parse(s).unwrap()
    }

    #[test]
    fn the_defaults_parse_without_a_warning() {
        let (km, warn) = Keymap::load(None);
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(km.act(key(",")), None);
        assert_eq!(km.act(key("<C-,>")), Some(Act::Settings));
        assert_eq!(km.act(key("?")), None, "? is the deep search; F1 is the key list");
        assert_eq!(km.act(key("<F1>")), Some(Act::Help));
        assert_eq!(km.act(key("<C-S-p>")), Some(Act::Palette));
        assert_eq!(km.act(key("<F12>")), Some(Act::BugReport));
        assert_eq!(km.act(key("<C-q>")), Some(Act::Quit));
        assert_eq!(km.act(key("<C-t>")), Some(Act::Repos));
        assert_eq!(km.act(key("<C-w>")), Some(Act::CloseTab));
        assert_eq!(km.act(key("<C-Tab>")), Some(Act::NextTab));
        assert_eq!(km.act(key("<C-S-Tab>")), Some(Act::PrevTab));
        assert_eq!(km.act(key("<C-PageDown>")), Some(Act::NextTab));
        assert_eq!(km.act(key("<C-PageUp>")), Some(Act::PrevTab));
        assert_eq!(km.act(key("<A-3>")), Some(Act::Tab(3)));
        assert_eq!(parse("tab 0"), Err("`tab 0` is not a command".to_owned()));
        for k in ["<C-=>", "<C-+>", "<C-;>"] {
            assert_eq!(km.act(key(k)), Some(Act::Scale(ScaleTo::In)), "{k}");
        }
        assert_eq!(km.act(key("<C-->")), Some(Act::Scale(ScaleTo::Out)));
        assert_eq!(km.act(key("<C-0>")), Some(Act::Scale(ScaleTo::Reset)));
        assert_eq!(km.act(key("j")), None);
    }

    #[test]
    fn a_hint_follows_the_keymap() {
        let (km, _) = Keymap::load(None);
        assert_eq!(km.hint(Act::Palette).as_deref(), Some("Ctrl+Shift+P"));
        assert_eq!(km.hint(Act::Settings).as_deref(), Some("Ctrl+,"));
        let (km, _) = Keymap::load(Some(
            "[main]
prepend_keymap = [{ on = \"<C-k>\", run = \"palette\" }]
",
        ));
        assert_eq!(km.hint(Act::Palette).as_deref(), Some("Ctrl+K"));
        let (km, _) = Keymap::load(Some(
            "[main]
keymap = []
",
        ));
        assert_eq!(km.hint(Act::Palette), None);
        assert_eq!(tip("Fetch", Some("f")), "Fetch (f)");
        assert_eq!(tip("Fetch", None), "Fetch");
    }

    #[test]
    fn a_prepended_key_wins_and_noop_takes_a_default_away() {
        let user = r#"
            [main]
            prepend_keymap = [
                { on = "j", run = "palette" },
                { on = "<C-q>", run = "noop" },
            ]
        "#;
        let (km, warn) = Keymap::load(Some(user));
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(km.act(key("j")), Some(Act::Palette));
        assert_eq!(km.act(key("<C-q>")), Some(Act::Noop));
    }

    #[test]
    fn a_command_that_is_not_one_is_skipped_with_a_warning() {
        let (km, warn) = Keymap::load(Some("[main]\nprepend_keymap = [{ on = \"x\", run = \"frobnicate\" }]\n"));
        assert_eq!(warn.len(), 1, "{warn:?}");
        assert_eq!(km.act(key("x")), None);
    }

    #[test]
    fn the_keys_egui_sends_are_the_keys_written() {
        use egui::{Key as K, Modifiers as M};
        let ctrl = M { ctrl: true, command: true, ..M::NONE };
        let shift = M { shift: true, ..ctrl };
        let (km, _) = Keymap::load(None);
        let at = |k, m| ito_keys::from_egui(k, &m).and_then(|k| km.act(k));
        assert_eq!(at(K::Comma, ctrl), Some(Act::Settings));
        assert_eq!(at(K::F1, M::NONE), Some(Act::Help));
        assert_eq!(at(K::P, shift), Some(Act::Palette));
        assert_eq!(at(K::F12, M::NONE), Some(Act::BugReport));
        assert_eq!(at(K::Q, ctrl), Some(Act::Quit));
        assert_eq!(at(K::W, ctrl), Some(Act::CloseTab));
        assert_eq!(at(K::Tab, ctrl), Some(Act::NextTab));
        assert_eq!(at(K::Tab, shift), Some(Act::PrevTab));
        assert_eq!(at(K::Num2, M::ALT), Some(Act::Tab(2)));
        assert_eq!(at(K::Minus, ctrl), Some(Act::Scale(ScaleTo::Out)));
        assert_eq!(at(K::Num0, ctrl), Some(Act::Scale(ScaleTo::Reset)));
    }
}
