//! `--keys`: a script of keys and clicks played into the window, and the
//! state it leaves for a test to read (common-spec, "キーの台本"). The
//! parsing and the playing are `ito-keyscript`; what is michi's is the
//! state file's lines.

use crate::app::{App, Mode};
use crate::picker::Kind;

impl App {
    /// The state file: `key: value` lines, `focus:` first, then what a
    /// test of michi asks about. Lines it does not know are skipped by the
    /// readers, so more can be added.
    pub fn report(&self) -> String {
        let mut s = String::new();
        s += &format!("focus: {}\n", self.focus());
        s += &format!("overlay: {}\n", self.overlay());
        s += &format!("scale: {}%\n", (self.prefs.common.scale_or_one() * 100.0).round());
        s += &format!("view: {}\n", self.view.digit());
        s += &format!("mode: {:?}\n", self.mode);
        s += &format!("repo: {}\n", self.dir.display());
        s += &format!("selected: {}\n", self.graph.selected().map_or("none".to_owned(), |o| format!("{o:?}")));
        s += &format!("worktree: {}\n", self.wt_sel);
        s += &format!("band: {}\n", self.band.as_ref().map_or("none", |b| b.text.as_str()).replace('\n', "\\n"));
        s += &format!("toast: {}\n", self.toast.as_ref().map_or("", |(t, _)| t.as_str()).replace('\n', "\\n"));
        s
    }

    /// Where the keys land: the overlay that has them, else the view.
    fn focus(&self) -> &'static str {
        match () {
            _ if self.mode == Mode::Settings => "settings",
            _ if self.mode == Mode::Help => "help",
            _ if self.picker.is_some() => "picker",
            _ if self.new_work.is_some() => "new-work",
            _ if self.tidy.is_some() => "tidy",
            _ if self.ready.is_some() => "ready",
            _ if self.reflog.is_some() => "reflog",
            _ if self.blame.is_some() => "blame",
            _ if self.reorder.is_some() => "reorder",
            _ if self.band.is_some() => "band",
            _ if self.review.is_some() => "review",
            _ => match self.view {
                crate::app::View::Graph => "graph",
                crate::app::View::Changes => "changes",
                crate::app::View::Worktree => "worktree",
                crate::app::View::Conflict => "conflict",
            },
        }
    }

    /// The box open over the view, by name (`none` for no box).
    fn overlay(&self) -> String {
        match () {
            _ if self.mode == Mode::Settings => "settings".to_owned(),
            _ if self.mode == Mode::Help => "help".to_owned(),
            _ if self.picker.is_some() => match self.picker.as_ref().map(|p| p.kind) {
                Some(Kind::Tag) => "tags",
                Some(Kind::Stash) => "stashes",
                Some(Kind::Remote) => "remotes",
                Some(Kind::Remotes) => "manage-remotes",
                Some(Kind::Move) => "move-changes",
                _ => "branches",
            }
            .to_owned(),
            _ if self.new_work.is_some() => "new-work".to_owned(),
            _ if self.tidy.is_some() => "tidy".to_owned(),
            _ if self.ready.is_some() => "ready".to_owned(),
            _ if self.reflog.is_some() => "reflog".to_owned(),
            _ if self.blame.is_some() => "blame".to_owned(),
            _ if self.reorder.is_some() => "reorder".to_owned(),
            _ if self.band.is_some() => "band".to_owned(),
            _ if self.review.is_some() => "review".to_owned(),
            _ => "none".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_starts_with_focus_and_names_the_overlay() {
        let a = App::new(crate::prefs::Prefs::empty(), std::env::temp_dir());
        let r = a.report();
        assert!(r.starts_with("focus: graph\n"), "{r}");
        assert!(r.contains("overlay: none\n") && r.contains("scale: 100%\n"), "{r}");
    }
}
