//! The app's state and what the keys do to it. Nothing here draws or runs
//! git, so the keys can be tested without a window or a repository.

use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use michi_core::git::{self, Version};
use michi_core::summary::Summary;

use crate::prefs::Prefs;
use crate::repo::Answer;

/// The three views, opened by `1` to `3` (docs/design.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Graph,
    Changes,
    Worktree,
}

impl View {
    pub const ALL: [View; 3] = [View::Graph, View::Changes, View::Worktree];

    /// The digit that opens it.
    pub fn digit(self) -> char {
        match self {
            View::Graph => '1',
            View::Changes => '2',
            View::Worktree => '3',
        }
    }

    pub fn from_digit(c: char) -> Option<View> {
        View::ALL.into_iter().find(|v| v.digit() == c)
    }

    /// The word-table key of its name.
    pub fn label(self) -> &'static str {
        match self {
            View::Graph => "view.graph",
            View::Changes => "view.changes",
            View::Worktree => "view.worktree",
        }
    }

    /// The word-table key of the line saying what will be here.
    pub fn empty(self) -> &'static str {
        match self {
            View::Graph => "empty.graph",
            View::Changes => "empty.changes",
            View::Worktree => "empty.worktree",
        }
    }

    /// The keys that work in this view: `(keys, word-table key)`, the one to
    /// recommend first. The keybar shows the first few; `?` shows them all.
    pub fn keys(self) -> &'static [(&'static str, &'static str)] {
        match self {
            View::Graph => &[
                ("j k", "key.move"),
                ("Enter", "key.side"),
                ("/", "key.filter"),
                ("z", "key.fold"),
                ("o", "key.tab"),
                ("l h", "key.open_fold"),
                ("b", "key.branch"),
                ("s", "key.stash"),
                ("Esc", "key.back"),
            ],
            View::Changes => &[("Space", "key.stage"), ("c", "key.commit")],
            View::Worktree => &[
                ("j k", "key.select"),
                ("o", "key.tab"),
                ("Enter", "key.in_graph"),
                ("m", "key.take_main"),
                ("x", "key.remove"),
                ("d", "key.branch_diff"),
            ],
        }
    }
}

/// The keys that work in every view.
pub const GLOBAL_KEYS: [(&str, &str); 8] = [
    ("1 2 3", "key.views"),
    ("N", "key.new_work"),
    ("f", "key.fetch"),
    ("p", "key.pull"),
    ("P", "key.push"),
    (",", "key.settings"),
    ("Ctrl+Shift+P", "key.palette"),
    ("?", "key.help"),
];

/// What the window shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Main,
    Settings,
    Help,
}

/// The repository the window is on, as far as it has been read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Repo {
    Loading,
    Ready(Summary),
    Failed(git::Error),
}

pub struct App {
    pub view: View,
    pub mode: Mode,
    pub prefs: Prefs,
    /// The language in force (`en`, `ja`), from common.toml or the OS.
    pub lang: String,
    /// The folder the window was opened on.
    pub dir: PathBuf,
    pub repo: Repo,
    /// The git on the PATH; `None` until it has answered.
    pub git: Option<Version>,
    /// What the reading thread sends back.
    pub answers: Option<Receiver<Answer>>,
    /// A new window title, for the frame to send once.
    pub new_title: Option<String>,
    /// One line at the foot, until its instant.
    pub toast: Option<(String, Instant)>,
    pub quit: bool,
}

impl App {
    pub fn new(prefs: Prefs, dir: PathBuf) -> Self {
        let mut app = Self {
            view: View::Graph,
            mode: Mode::Main,
            prefs,
            lang: "en".into(),
            dir,
            repo: Repo::Loading,
            git: None,
            answers: None,
            new_title: None,
            toast: None,
            quit: false,
        };
        app.set_language();
        app
    }

    /// The language common.toml names, else the OS's.
    pub fn set_language(&mut self) {
        let os = ito_common::os_language();
        self.lang = ito_i18n::resolve(None, self.prefs.common.language.as_deref(), os.as_deref(), &crate::i18n::AVAILABLE);
    }

    /// The words for `key` in the language in force.
    pub fn tr(&self, key: &'static str) -> &'static str {
        crate::i18n::tr(&self.lang, key)
    }

    pub fn say(&mut self, text: impl Into<String>) {
        self.say_for(text, 4);
    }

    pub fn say_for(&mut self, text: impl Into<String>, secs: u64) {
        self.toast = Some((text.into(), Instant::now() + Duration::from_secs(secs)));
    }

    /// How long until the toast goes, if there is one.
    pub fn toast_left(&self) -> Option<Duration> {
        self.toast.as_ref().map(|(_, until)| until.saturating_duration_since(Instant::now()))
    }

    /// Fold in what the reading thread has sent since the last frame.
    pub fn pump(&mut self) {
        let got: Vec<Answer> = match &self.answers {
            Some(rx) => rx.try_iter().collect(),
            None => return,
        };
        for a in got {
            self.take(a);
        }
    }

    pub fn take(&mut self, answer: Answer) {
        match answer {
            Answer::Version(Ok(v)) => {
                self.git = Some(v);
                if !v.is_enough() {
                    let text = self.tr("git.old").replace("{v}", &v.to_string()).replace("{min}", &git::MIN.to_string());
                    self.say_for(text, 10);
                }
            }
            // The summary that follows says why (git missing, mostly).
            Answer::Version(Err(_)) => {}
            Answer::Summary(Ok(s)) => {
                self.new_title = Some(format!("{} — michi", s.name));
                self.repo = Repo::Ready(s);
            }
            Answer::Summary(Err(e)) => {
                let word = self.tr(failed_word(&e));
                let path = self.dir.display();
                let text = match &e {
                    git::Error::Git(d) | git::Error::Io(d) => format!("{word}: {d} — {path}"),
                    _ => format!("{word} — {path}"),
                };
                self.say_for(text, 10);
                self.repo = Repo::Failed(e);
            }
        }
    }

    /// The git on the PATH when it is older than michi needs.
    pub fn old_git(&self) -> Option<Version> {
        self.git.filter(|v| !v.is_enough())
    }

    /// A character typed (not a key with Ctrl or Alt: those are the
    /// shortcuts). Returns whether it was used.
    pub fn feed(&mut self, c: char) -> bool {
        match (self.mode, c) {
            (Mode::Help, '?') => self.mode = Mode::Main,
            (Mode::Settings, _) | (Mode::Help, _) => return false,
            (Mode::Main, '?') => self.mode = Mode::Help,
            (Mode::Main, ',') => self.mode = Mode::Settings,
            (Mode::Main, c) => match View::from_digit(c) {
                Some(v) => self.view = v,
                None => return self.later(|token| token.chars().eq([c])),
            },
        }
        true
    }

    /// A key with a name (`Enter`), outside the help and the settings.
    pub fn named(&mut self, name: &str) -> bool {
        self.mode == Mode::Main && self.later(|token| token == name)
    }

    /// A key of the view or of every view whose work comes in a later stage:
    /// say so. `Space` is the space character.
    fn later(&mut self, is: impl Fn(&str) -> bool) -> bool {
        let found = self.view.keys().iter().chain(GLOBAL_KEYS.iter()).find(|(keys, _)| {
            keys.split(' ').any(|token| {
                let token = if token == "Space" { " " } else { token };
                is(token)
            })
        });
        let Some((_, word)) = found else { return false };
        let text = self.tr("later").replace("{}", self.tr(word));
        self.say(text);
        true
    }

    /// Escape: close the help or the settings.
    pub fn escape(&mut self) -> bool {
        let was = self.mode;
        self.mode = Mode::Main;
        was != Mode::Main
    }

    /// Whether the bar's tab for `v` is the one in force: the view's, under
    /// the key list too, which is drawn over the view.
    pub fn is_on(&self, v: View) -> bool {
        self.mode != Mode::Settings && self.view == v
    }
}

/// The word-table key saying why the repository could not be read.
pub fn failed_word(e: &git::Error) -> &'static str {
    match e {
        git::Error::NoGit => "repo.no_git",
        git::Error::NotRepo => "repo.not_repo",
        git::Error::Git(_) | git::Error::Io(_) => "repo.failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use michi_core::summary::Head;

    fn app() -> App {
        let mut a = App::new(Prefs::empty(), PathBuf::from("C:/dev/x"));
        a.lang = "en".into();
        a
    }

    fn toast(a: &App) -> &str {
        a.toast.as_ref().map_or("", |(t, _)| t)
    }

    #[test]
    fn digits_open_the_views() {
        let mut a = app();
        assert_eq!(a.view, View::Graph);
        for (c, v) in [('2', View::Changes), ('3', View::Worktree), ('1', View::Graph)] {
            assert!(a.feed(c));
            assert_eq!(a.view, v);
        }
        assert!(!a.feed('4'));
        assert!(a.toast.is_none());
    }

    #[test]
    fn comma_and_question_mark_open_and_escape_closes() {
        let mut a = app();
        assert!(a.feed(','));
        assert_eq!(a.mode, Mode::Settings);
        // In the settings the screen's own controls take the characters.
        assert!(!a.feed('3'));
        assert_eq!(a.view, View::Graph);
        assert!(a.escape());
        assert_eq!(a.mode, Mode::Main);
        assert!(!a.escape());
        assert!(a.feed('?'));
        assert_eq!(a.mode, Mode::Help);
        assert!(!a.feed('3'), "the help swallows the views' digits");
        assert!(!a.named("Enter"));
        assert!(a.feed('?'));
        assert_eq!(a.mode, Mode::Main);
    }

    #[test]
    fn the_tab_in_force_is_the_view_but_not_in_the_settings() {
        let mut a = app();
        assert!(a.is_on(View::Graph));
        a.feed('?');
        assert!(a.is_on(View::Graph), "the key list is over the view");
        a.escape();
        a.feed(',');
        assert!(!a.is_on(View::Graph));
    }

    #[test]
    fn every_view_has_keys_and_words_for_them() {
        for v in View::ALL {
            assert!(!v.keys().is_empty());
            for (_, w) in v.keys() {
                assert_ne!(crate::i18n::tr("en", w), *w, "no words for {w}");
            }
            assert_ne!(crate::i18n::tr("en", v.label()), v.label());
            assert_ne!(crate::i18n::tr("en", v.empty()), v.empty());
        }
        for (_, w) in GLOBAL_KEYS {
            assert_ne!(crate::i18n::tr("en", w), w, "no words for {w}");
        }
    }

    #[test]
    fn keys_of_later_stages_say_so() {
        let mut a = app();
        assert!(a.feed('N'));
        assert_eq!(toast(&a), "New work (worktree + Claude) — coming in a later stage");
        assert!(a.feed('f'));
        assert!(toast(&a).starts_with("Fetch"));
        assert!(a.feed('z'), "the graph's own key");
        assert!(toast(&a).starts_with("Fold"));
        assert!(a.named("Enter"));
        assert!(toast(&a).starts_with("Side by side"));
        a.feed('2');
        assert!(a.feed(' '));
        assert!(toast(&a).starts_with("Stage"));
        assert!(!a.feed('z'), "not a key of the changes");
        assert!(!a.feed('q'));
        a.lang = "ja".into();
        assert!(a.feed('c'));
        assert_eq!(toast(&a), "コミット — 後の段階で入ります");
    }

    #[test]
    fn an_old_git_is_said_and_kept() {
        let mut a = app();
        a.take(Answer::Version(Ok(Version(2, 30, 1))));
        assert_eq!(a.old_git(), Some(Version(2, 30, 1)));
        assert!(toast(&a).starts_with("git 2.30.1 is older than 2.38.0"), "{}", toast(&a));
        let mut b = app();
        b.take(Answer::Version(Ok(Version(2, 52, 0))));
        assert_eq!(b.old_git(), None);
        assert!(b.toast.is_none());
    }

    #[test]
    fn the_summary_names_the_window_or_says_why_not() {
        let mut a = app();
        let s = Summary { name: "michi".into(), head: Head::Branch("main".into()), worktrees: 2, top: PathBuf::from("C:/dev/x") };
        a.take(Answer::Summary(Ok(s.clone())));
        assert_eq!(a.new_title.as_deref(), Some("michi — michi"));
        assert_eq!(a.repo, Repo::Ready(s));

        let mut b = app();
        b.take(Answer::Summary(Err(git::Error::NotRepo)));
        assert_eq!(b.repo, Repo::Failed(git::Error::NotRepo));
        assert_eq!(toast(&b), "Not a Git repository — C:/dev/x");
        assert!(b.new_title.is_none());
        b.take(Answer::Summary(Err(git::Error::Git("bad object".into()))));
        assert_eq!(toast(&b), "git failed: bad object — C:/dev/x");
    }
}
