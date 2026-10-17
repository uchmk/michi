//! The command palette (`<C-S-p>`): every command michi has, found by what
//! is typed (`ito-palette`). The list is made from the same tables as the
//! key list, so a command is in it when it has a key; picking one does what
//! the key would have done.

use ito_palette::{Entry, Style, View, Words};

use crate::app::{App, GLOBAL_KEYS};
use crate::keymap::Act;
use ito_openin::Tool;

/// What picking an entry does.
#[derive(Clone, Debug, PartialEq)]
pub enum Pick {
    /// A command of the keymap (the settings, the key list, quit, the scale).
    Act(Act),
    /// A character typed in the main window.
    Key(char),
    /// Open the worktree's folder in a terminal, an editor or the file manager.
    Open(Tool),
    /// Show the first-run guide again (#116).
    Guide,
    /// Make a practice repository in a temporary folder and open it (#117).
    Practice,
    /// Show the legend of the graph (#118).
    Legend,
    /// Keep the whole state of the worktree safely (#113).
    Calm,
    /// Do the suggested next step (#119).
    Next,
}

/// The entries the palette offers now: the keymap's commands, then those of
/// every view's keys that are one character, the view in front first.
pub fn entries(app: &App) -> Vec<Entry<Pick>> {
    let mut out = Vec::new();
    for l in ito_keymap::help_lines(&app.prefs.keymap.main, "noop") {
        let desc = if l.desc.is_empty() { l.run.to_owned() } else { crate::i18n::tr_desc(&app.lang, l.desc) };
        if let Ok(act) = crate::keymap::parse(l.run) {
            out.push(Entry::new(desc, l.keys, Pick::Act(act)));
        }
    }
    for v in crate::app::View::ALL {
        out.push(Entry::new(format!("{} {}", app.tr("key.tab"), app.tr(v.label())), v.digit().to_string(), Pick::Key(v.digit())));
    }
    let in_table = app.view == crate::app::View::Worktree;
    for (t, key) in [(Tool::Terminal, "t"), (Tool::Editor, "e"), (Tool::Explorer, "E")] {
        out.push(Entry::new(app.tr(crate::act::open_word_cmd(t)), if in_table { key } else { "" }, Pick::Open(t)));
    }
    if !app.problems.is_empty() {
        out.push(Entry::new(app.tr("guide.title"), "", Pick::Guide));
    }
    out.push(Entry::new(app.tr("practice.cmd"), "", Pick::Practice));
    out.push(Entry::new(app.tr("legend.cmd"), "", Pick::Legend));
    out.push(Entry::new(app.tr("calm.cmd"), "", Pick::Calm));
    if let Some(s) = app.suggestion() {
        out.push(Entry::new(format!("{} ({})", app.tr("suggest.cmd"), app.tr(s.key())), "", Pick::Next));
    }
    let mut seen = Vec::new();
    for (keys, word) in app.keys().iter().chain(GLOBAL_KEYS.iter().skip(1)) {
        let mut chars = keys.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            // Space (stage, pick) has no letter but is a key all the same.
            if *keys == "Space" && !seen.contains(&' ') {
                seen.push(' ');
                out.push(Entry::new(app.tr(word), keys.to_string(), Pick::Key(' ')));
            }
            continue;
        };
        if !seen.contains(&c) {
            seen.push(c);
            out.push(Entry::new(app.tr(word), keys.to_string(), Pick::Key(c)));
        }
    }
    out
}

/// Draw the palette and take its keys. What it picked, if anything; it is
/// closed on a pick, Esc and a click outside.
pub fn show(ctx: &egui::Context, app: &mut App) -> Option<Pick> {
    let mut view: View = app.cmd.take()?;
    let c = ito_theme::colors();
    let style = Style {
        panel: c.panel,
        border: c.border,
        text: c.fg,
        dim: c.dim,
        selected_bg: c.border,
        selected_text: crate::ui::accent(),
        hit: crate::ui::accent(),
        accent: crate::ui::accent(),
    };
    let words = Words { hint: app.tr("palette.hint"), none: app.tr("palette.none"), header: "", footer: "" };
    match ito_palette::show(ctx, &mut view, &entries(app), &style, &words) {
        Some(ito_palette::Answer::Pick(p)) => Some(p),
        Some(ito_palette::Answer::Close | ito_palette::Answer::Key(..)) => None,
        None => {
            app.cmd = Some(view);
            None
        }
    }
}

impl App {
    /// Open the palette, or close it; not over a box that has the keys.
    pub fn toggle_palette(&mut self) {
        if self.cmd.take().is_some() {
            return;
        }
        let busy = self.picker.is_some()
            || self.new_work.is_some()
            || self.pr_form.is_some()
            || self.band.is_some()
            || self.tidy.is_some()
            || self.reflog.is_some()
            || self.stats.is_some()
            || self.legend
            || self.ready.is_some()
            || self.auth.is_some()
            || self.blame.is_some()
            || self.trial.is_some()
            || self.reorder.is_some()
            || self.conflicts.edit.is_some();
        if !busy {
            self.cmd = Some(View::new());
        }
    }

    /// Build the practice repository on a thread (git runs off the UI thread);
    /// `main.rs` opens it in a tab when it is ready.
    pub fn start_practice(&mut self) {
        if self.practicing.is_some() {
            return;
        }
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(michi_core::practice::create(&std::env::temp_dir()).map_err(|e| e.to_string()));
        });
        self.practicing = Some(rx);
        let text = self.tr("practice.making").to_owned();
        self.say_for(text, 4);
    }

    /// Do what the palette picked, as the key would have.
    pub fn run_pick(&mut self, p: Pick) {
        match p {
            Pick::Act(a) => self.run_key(a),
            Pick::Open(t) => self.open_in(t, self.view == crate::app::View::Worktree),
            Pick::Guide => self.guide_open = true,
            Pick::Practice => self.start_practice(),
            Pick::Legend => {
                self.mode = crate::app::Mode::Main;
                self.view = crate::app::View::Graph;
                self.legend = true;
            }
            Pick::Calm => self.calm(),
            Pick::Next => self.run_suggestion(),
            Pick::Key(c) => {
                self.mode = crate::app::Mode::Main;
                self.feed(c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_has_the_keymaps_commands_and_the_views_keys() {
        let a = App::new(crate::prefs::Prefs::empty(), std::env::temp_dir());
        let e = entries(&a);
        assert!(e.iter().any(|e| e.pick == Pick::Act(Act::Settings)));
        assert!(e.iter().any(|e| e.pick == Pick::Key('3')), "a tab");
        assert!(e.iter().any(|e| e.pick == Pick::Key('b')), "the graph's branch palette");
        assert!(!e.iter().any(|e| e.pick == Pick::Key('j')), "moving is not a command");
    }

    #[test]
    fn the_basic_level_keeps_every_command_in_the_palette() {
        let mut p = crate::prefs::Prefs::empty();
        p.level = michi_core::level::Level::Basic;
        let a = App::new(p, std::env::temp_dir());
        let e = entries(&a);
        assert!(e.iter().any(|e| e.pick == Pick::Key('b')));
        // Every key of every view is still offered when it is one character, whatever the level.
        let full = entries(&App::new(crate::prefs::Prefs::empty(), std::env::temp_dir()));
        assert_eq!(e.len(), full.len());
    }

    #[test]
    fn the_guide_is_offered_only_when_something_is_wrong() {
        let mut a = App::new(crate::prefs::Prefs::empty(), std::env::temp_dir());
        assert!(!entries(&a).iter().any(|e| e.pick == Pick::Guide));
        a.problems = vec![michi_core::setup::Problem::NoName];
        assert!(entries(&a).iter().any(|e| e.pick == Pick::Guide));
        a.run_pick(Pick::Guide);
        assert!(a.guide_open);
        assert!(a.escape(), "Esc closes it");
        assert!(!a.guide_open);
    }

    #[test]
    fn the_practice_repository_and_the_legend_are_in_the_list() {
        let a = App::new(crate::prefs::Prefs::empty(), std::env::temp_dir());
        let e = entries(&a);
        assert!(e.iter().any(|e| e.pick == Pick::Practice));
        assert!(e.iter().any(|e| e.pick == Pick::Legend));
    }

    #[test]
    fn the_legend_opens_on_the_graph_and_any_key_closes_it() {
        let mut a = App::new(crate::prefs::Prefs::empty(), std::env::temp_dir());
        a.run_pick(Pick::Legend);
        assert!(a.legend && a.view == crate::app::View::Graph);
        assert!(a.feed('x'));
        assert!(!a.legend);
    }

    #[test]
    fn a_pick_does_what_the_key_does() {
        let mut a = App::new(crate::prefs::Prefs::empty(), std::env::temp_dir());
        a.toggle_palette();
        assert!(a.cmd.is_some());
        a.run_pick(Pick::Key('3'));
        assert_eq!(a.view, crate::app::View::Worktree);
        a.toggle_palette();
        assert!(a.cmd.is_none(), "the same key closes it");
    }
}
