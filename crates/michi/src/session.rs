//! What michi keeps between runs: `state.toml` in the app's state folder
//! ([`ito_common::state_dir`], where tsumugi, yagura and minato keep theirs).
//! For now that is the tabs, the one in front and its view; started with no
//! folder, michi comes back as it was closed.

use crate::app::View;
use michi_core::tabs::Tabs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Session {
    /// The folders of the tabs, left to right.
    #[serde(default)]
    pub tabs: Vec<String>,
    /// The number of the tab in front.
    #[serde(default)]
    pub active: usize,
    /// The digit of the front tab's view (`1` to `3`).
    #[serde(default)]
    pub view: String,
}

impl Session {
    /// The tabs and the view the front one is on. The conflict view is only
    /// there while something clashed, so it is kept as the graph.
    pub fn of(tabs: &Tabs, view: View) -> Self {
        let view = if view == View::Conflict { View::Graph } else { view };
        Self {
            tabs: tabs.paths().iter().map(|p| p.to_string_lossy().into_owned()).collect(),
            active: tabs.active(),
            view: view.digit().to_string(),
        }
    }

    pub fn text(&self) -> String {
        toml::to_string(self).unwrap_or_default()
    }

    pub fn parse(text: &str) -> Option<Self> {
        toml::from_str(text).ok()
    }

    /// The tabs, `first` alone when none was kept.
    pub fn to_tabs(&self, first: &Path) -> Tabs {
        let paths: Vec<PathBuf> = self.tabs.iter().map(PathBuf::from).collect();
        Tabs::from_paths(&paths, self.active, first)
    }

    /// The front tab's view, none for one that is not kept.
    pub fn view(&self) -> Option<View> {
        let mut cs = self.view.chars();
        let v = View::from_digit(cs.next()?)?;
        (cs.next().is_none() && v != View::Conflict).then_some(v)
    }
}

/// Where the session is kept.
pub fn path() -> Option<PathBuf> {
    ito_common::state_dir("michi").map(|d| d.join("state.toml"))
}

/// The session of the last run. A build before kept the tabs in `tabs.txt`
/// beside the settings; that is read when there is no `state.toml` yet.
pub fn load(base: Option<&Path>) -> Option<Session> {
    if let Some(s) = path().and_then(|f| std::fs::read_to_string(f).ok()) {
        return Session::parse(&s);
    }
    let old = crate::prefs::window_dir(base).and_then(|d| std::fs::read_to_string(d.join("tabs.txt")).ok())?;
    let tabs = Tabs::resume(&old, Path::new(""));
    let paths = tabs.paths();
    let has = paths.first().is_some_and(|p| !p.as_os_str().is_empty());
    has.then(|| Session::of(&tabs, View::Graph))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tabs() -> Tabs {
        let mut t = Tabs::new(Path::new("/a"));
        t.open(Path::new("/b"));
        t.open(Path::new("/c"));
        t.activate(1);
        t
    }

    #[test]
    fn the_session_round_trips_through_toml() {
        let s = Session::of(&tabs(), View::Changes);
        let back = Session::parse(&s.text()).unwrap();
        assert_eq!(back, s);
        assert_eq!(back.to_tabs(Path::new("/z")), tabs());
        assert_eq!(back.view(), Some(View::Changes));
    }

    #[test]
    fn windows_paths_survive() {
        let t = Tabs::new(Path::new(r"C:\dev\michi"));
        let back = Session::parse(&Session::of(&t, View::Graph).text()).unwrap();
        assert_eq!(back.to_tabs(Path::new("/z")), t);
    }

    #[test]
    fn the_conflict_view_is_kept_as_the_graph() {
        assert_eq!(Session::of(&tabs(), View::Conflict).view(), Some(View::Graph));
        assert_eq!(Session { view: "4".into(), ..Default::default() }.view(), None);
        assert_eq!(Session { view: "12".into(), ..Default::default() }.view(), None);
        assert_eq!(Session::default().view(), None);
    }

    #[test]
    fn a_broken_or_empty_file_is_no_session() {
        assert_eq!(Session::parse("tabs = [").map(|_| ()), None);
        let s = Session::parse("").unwrap();
        assert_eq!(s.to_tabs(Path::new("/z")), Tabs::new(Path::new("/z")));
    }
}
