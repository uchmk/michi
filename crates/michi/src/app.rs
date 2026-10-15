//! The app's state and what the keys do to it. Nothing here draws or runs
//! git, so the keys can be tested without a window or a repository.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use michi_core::claude::Session;
use michi_core::git::{self, Version};
use michi_core::oid::Oid;
use michi_core::summary::Summary;

use crate::detail::Pane;
use crate::graph::{Folded, Graph};
use crate::prefs::Prefs;
use crate::repo::{Answer, Ask, Reading};

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
    /// The two ends of the reading thread.
    pub reading: Option<Reading>,
    pub graph: Graph,
    /// The selected commit in full, on the right.
    pub detail: Pane,
    /// The Claude sessions in each worktree that has some; `None` when
    /// tsumugi is not there (or not asked yet).
    pub claude: Option<Vec<(PathBuf, Vec<Session>)>>,
    /// The side column's groups that are open (`b:feat`, `r:origin`).
    pub side_open: HashSet<String>,
    /// A commit clicked in the side column that is not read yet: read on
    /// until it is, then select it.
    seeking: Option<Oid>,
    /// The worktree whose uncommitted change the detail shows.
    detail_wip: Option<PathBuf>,
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
            reading: None,
            graph: Graph::default(),
            detail: Pane::default(),
            claude: None,
            side_open: HashSet::new(),
            seeking: None,
            detail_wip: None,
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
        let got: Vec<Answer> = match &self.reading {
            Some(r) => r.answers.try_iter().collect(),
            None => return,
        };
        for a in got {
            self.take(a);
        }
        self.seek();
    }

    /// Asks for the selected commit in full when the selection has changed,
    /// and counts a trunk merge's commits; whether it had changed.
    pub fn sync_detail(&mut self) -> bool {
        let id = self.graph.selected();
        // The same worktree's change read anew: the detail keeps its place.
        let wip = self.graph.selected_wip().map(|w| w.path.clone());
        let changed = if wip.is_some() && wip == self.detail_wip { self.detail.refresh(id) } else { self.detail.select(id) };
        self.detail_wip = wip;
        if changed {
            if let (Some(id), Some(r)) = (id, &self.reading) {
                // A merge's diff (the whole branch's) comes in stage 2.
                let patch = self.graph.find_commit(&id).is_none_or(|c| c.parents.len() < 2);
                r.detail(id, patch);
            }
        }
        if let Some(id) = id {
            let read = self.graph.read();
            if self.detail.merged(id, read).is_none() {
                let commits = self.graph.merged_by(&id);
                self.detail.set_merged(id, read, commits);
            }
        }
        changed
    }

    /// A row of the side column is clicked: select its commit, reading on
    /// to it when it is further down than read.
    pub fn go_to(&mut self, id: Oid) {
        self.seeking = None;
        if !self.graph.select_id(&id) {
            self.seeking = Some(id);
            self.seek();
        }
    }

    /// Reads on towards the commit the side column wants.
    fn seek(&mut self) {
        let Some(id) = self.seeking else { return };
        if self.graph.select_id(&id) {
            self.seeking = None;
        } else if self.graph.done() || self.graph.error().is_some() {
            self.seeking = None;
            let text = self.tr("side.missing").replace("{}", &id.short());
            self.say(text);
        } else if self.graph.ask_more() {
            self.ask(Ask::More);
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
            Answer::Setup { gen, setup } => self.graph.start(gen, *setup),
            Answer::Commits { gen, batch, done } => self.graph.commits(gen, batch, done),
            Answer::Total { gen, count } => self.graph.total(gen, count),
            Answer::Failed { gen, error } => {
                let text = self.tr("history.failed").replace("{}", &error.to_string());
                self.say_for(text, 10);
                self.graph.failed(gen, error);
            }
            Answer::Detail { id, result } => self.detail.take(id, result),
            Answer::Wips(wips) => self.graph.set_wips(wips),
            Answer::Claude(c) => self.claude = c,
        }
    }

    /// The Claude sessions in the worktree at `path`, the one to show first
    /// first.
    pub fn claude_in(&self, path: &Path) -> &[Session] {
        self.claude.iter().flatten().find(|(p, _)| p == path).map_or(&[], |(_, s)| s.as_slice())
    }

    /// The graph's rows in sight reach row `last`: read on before they run
    /// out.
    pub fn reach(&mut self, last: u32) {
        if self.graph.wants_more(last) {
            self.ask(Ask::More);
        }
    }

    /// The row at the graph's foot is clicked: read the next batch.
    pub fn more(&mut self) {
        if self.graph.ask_more() {
            self.ask(Ask::More);
        }
    }

    fn ask(&self, a: Ask) {
        if let Some(r) = &self.reading {
            let _ = r.asks.send(a);
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
                None if self.view == View::Graph && self.graph_key(c) => {}
                None => return self.later(|token| token.chars().eq([c])),
            },
        }
        true
    }

    /// The graph's own keys that work already.
    fn graph_key(&mut self, c: char) -> bool {
        let g = &mut self.graph;
        match c {
            'j' => g.move_by(1),
            'k' => g.move_by(-1),
            'l' => _ = g.open_fold(),
            'h' => _ = g.close_fold(),
            '/' => g.focus_filter = true,
            'z' => self.fold(),
            _ => return false,
        }
        true
    }

    /// `z`, or the header's toggle: fold the trunk's merges or show them all.
    pub fn fold(&mut self) {
        let text = match self.graph.toggle_fold() {
            Folded::On { merges, commits } => self
                .tr("fold.on")
                .replace("{trunk}", self.graph.trunk_name().unwrap_or("main"))
                .replace("{n}", &merges.to_string())
                .replace("{m}", &commits.to_string()),
            Folded::Nothing => self.tr("fold.nothing").to_owned(),
            Folded::Off => self.tr("fold.off").to_owned(),
        };
        self.say(text);
    }

    /// A key with a name (`Enter`, `ArrowDown`), outside the help and the
    /// settings.
    pub fn named(&mut self, name: &str) -> bool {
        if self.mode != Mode::Main {
            return false;
        }
        if self.view == View::Graph {
            let g = &mut self.graph;
            match name {
                "ArrowDown" => g.move_by(1),
                "ArrowUp" => g.move_by(-1),
                "PageDown" => g.page_by(false),
                "PageUp" => g.page_by(true),
                "Home" => g.home(),
                "End" => g.end(),
                "ArrowRight" => _ = g.open_fold(),
                "ArrowLeft" => _ = g.close_fold(),
                "Enter" => self.detail.wide = !self.detail.wide,
                _ => return self.later(|token| token == name),
            }
            return true;
        }
        self.later(|token| token == name)
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

    /// Escape: close the help or the settings, else bring the graph back
    /// from beside a wide diff, else clear the graph's filter.
    pub fn escape(&mut self) -> bool {
        if self.mode != Mode::Main {
            self.mode = Mode::Main;
            return true;
        }
        if self.view == View::Graph && self.detail.wide {
            self.detail.wide = false;
            return true;
        }
        if self.view == View::Graph && !self.graph.filter().is_empty() {
            self.graph.set_filter("");
            return true;
        }
        false
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
    use crate::graph::fixture::{self, FEAT};
    use michi_core::claude::State;
    use michi_core::summary::Head;

    fn app() -> App {
        let mut a = App::new(Prefs::empty(), PathBuf::from("C:/dev/x"));
        a.lang = "en".into();
        a
    }

    fn toast(a: &App) -> &str {
        a.toast.as_ref().map_or("", |(t, _)| t)
    }

    /// The app with graph.rs's small history read in, as the reading
    /// thread sends it.
    fn read() -> App {
        let mut a = app();
        a.take(Answer::Setup { gen: 1, setup: Box::new(fixture::setup()) });
        a.take(Answer::Commits { gen: 1, batch: fixture::history(), done: true });
        a
    }

    fn selected(a: &App) -> &str {
        let g = &a.graph;
        g.selected_row().and_then(|k| g.commit(k)).map_or("", |c| c.subject.as_str())
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
        assert!(a.feed('b'), "the graph's own key");
        assert!(toast(&a).starts_with("Branch"));
        assert!(a.named("Enter"), "the diff side by side, no longer a later stage");
        assert!(a.detail.wide);
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
    fn the_graph_s_keys_move_the_selection() {
        let mut a = read();
        assert_eq!(selected(&a), "m2");
        assert!(a.feed('j'));
        assert_eq!(selected(&a), "w1");
        assert!(a.named("ArrowDown"));
        assert_eq!(selected(&a), FEAT);
        assert!(a.feed('k'));
        assert_eq!(selected(&a), "w1");
        assert!(a.named("End"));
        assert_eq!(selected(&a), "first");
        assert!(a.named("PageUp"));
        assert_eq!(selected(&a), "m2");
        assert!(a.named("PageDown"));
        assert_eq!(selected(&a), "first");
        assert!(a.named("Home"));
        assert_eq!(selected(&a), "m2");
        assert!(a.toast.is_none(), "nothing to say");
    }

    #[test]
    fn l_h_and_the_arrows_open_and_fold_a_merge() {
        let mut a = read();
        a.feed('j');
        a.feed('j');
        assert_eq!(selected(&a), FEAT);
        assert!(a.named("ArrowRight"));
        assert_eq!(a.graph.len(), 7);
        a.feed('j');
        a.feed('j');
        assert_eq!(selected(&a), "f2");
        assert!(a.named("ArrowLeft"));
        assert_eq!((a.graph.len(), selected(&a)), (5, FEAT), "the merge takes the selection");
        assert!(a.feed('l'));
        assert_eq!(a.graph.len(), 7);
        assert!(a.feed('h'));
        assert_eq!(a.graph.len(), 5);
    }

    #[test]
    fn z_folds_and_says_so() {
        let mut a = read();
        assert!(a.feed('z'));
        assert_eq!(toast(&a), "Unfolded all the merges.");
        assert_eq!(a.graph.len(), 7);
        assert!(a.feed('z'));
        assert_eq!(toast(&a), "Folded the merges into main: 1 (2 commits).");
        assert_eq!(a.graph.len(), 5);
        a.lang = "ja".into();
        a.feed('z');
        a.feed('z');
        assert!(toast(&a).contains("main"), "{}", toast(&a));

        let mut b = app();
        b.feed('z');
        b.feed('z');
        assert_eq!(toast(&b), "No merges to fold yet.");
    }

    #[test]
    fn slash_asks_for_the_filter_and_escape_clears_it() {
        let mut a = read();
        assert!(a.feed('/'));
        assert!(a.graph.focus_filter);
        a.graph.set_filter("f2");
        assert_eq!(a.graph.hit_count(), Some(1));
        assert!(a.escape());
        assert!(a.graph.filter().is_empty());
        assert!(!a.escape(), "nothing left to clear");
        a.graph.set_filter("f2");
        a.feed('2');
        assert!(!a.escape(), "the filter is the graph's");
    }

    #[test]
    fn answers_of_an_older_reading_are_dropped() {
        let mut a = read();
        a.take(Answer::Setup { gen: 2, setup: Box::new(fixture::setup()) });
        a.take(Answer::Commits { gen: 1, batch: fixture::history(), done: true });
        assert_eq!(a.graph.len(), 0, "the first reading's commits come too late");
        a.take(Answer::Failed { gen: 2, error: git::Error::Git("bad".into()) });
        assert_eq!(toast(&a), "Could not read the history: bad");
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

    #[test]
    fn the_detail_follows_the_selection() {
        use crate::detail::fixture::detail;
        let mut a = read();
        assert!(a.sync_detail(), "the first selection is asked for");
        let m2 = a.graph.selected().unwrap();
        assert_eq!(a.detail.asked(), Some(m2));
        assert!(!a.sync_detail());
        assert!(a.detail.got().is_none());
        // An answer for a commit no longer selected is dropped.
        a.take(Answer::Detail { id: fixture::id(1), result: Ok(Box::new(detail(fixture::id(1), vec![], 1))) });
        assert!(a.detail.got().is_none());
        a.take(Answer::Detail { id: m2, result: Ok(Box::new(detail(m2, vec![fixture::id(5)], 2))) });
        assert!(a.detail.rows(1).is_some());
        a.named("ArrowDown");
        assert!(a.sync_detail());
        assert!(a.detail.got().is_none() && a.detail.to_top);
    }

    #[test]
    fn a_trunk_merge_counts_its_commits() {
        let mut a = read();
        assert!(a.graph.select_id(&fixture::id(5)));
        a.sync_detail();
        let read = a.graph.read();
        assert_eq!(a.detail.merged(fixture::id(5), read), Some(&[fixture::id(3), fixture::id(2)][..]));
        assert_eq!(selected(&a), FEAT);
    }

    #[test]
    fn enter_spreads_the_diff_and_escape_takes_it_back_first() {
        let mut a = read();
        a.graph.set_filter("f");
        assert!(a.named("Enter"));
        assert!(a.detail.wide);
        assert!(a.escape());
        assert!(!a.detail.wide);
        assert_eq!(a.graph.filter(), "f", "the filter goes on the next Escape");
        assert!(a.escape());
        assert_eq!(a.graph.filter(), "");
        assert!(!a.escape());
    }

    #[test]
    fn the_side_column_goes_to_a_commit_or_says_it_is_not_there() {
        let mut a = read();
        a.go_to(fixture::id(3));
        assert_eq!(selected(&a), "f2");
        assert!(a.seeking.is_none());
        a.go_to(fixture::id(9));
        assert!(a.seeking.is_none(), "all is read: nothing more to read on to");
        assert!(toast(&a).starts_with("Not in the history: "), "{}", toast(&a));
        assert_eq!(selected(&a), "f2");
    }

    #[test]
    fn a_wip_read_anew_keeps_the_detail_where_it_was() {
        let mut a = read();
        a.take(Answer::Wips(vec![fixture::wip("C:/r/repo", Some("main"), 6, 11, Some(1))]));
        a.graph.select(0);
        assert!(a.sync_detail());
        assert_eq!(a.detail.asked(), Some(fixture::id(11)));
        let shown = crate::detail::fixture::detail(fixture::id(11), vec![fixture::id(6)], 2);
        a.detail.take(fixture::id(11), Ok(Box::new(shown)));
        a.detail.to_top = false;
        a.detail.scroll_to = Some(1);
        // Written again: the new change is asked for, the old one stays shown.
        a.take(Answer::Wips(vec![fixture::wip("C:/r/repo", Some("main"), 6, 12, Some(2))]));
        assert!(a.sync_detail());
        assert_eq!(a.detail.asked(), Some(fixture::id(12)));
        assert!(a.detail.got().is_some());
        assert_eq!((a.detail.to_top, a.detail.scroll_to), (false, Some(1)));
        assert!(!a.sync_detail());
        // Another row: from the top.
        a.graph.select(1);
        assert!(a.sync_detail());
        assert!(a.detail.got().is_none() && a.detail.to_top);
    }

    #[test]
    fn claude_sessions_are_looked_up_by_worktree() {
        let mut a = app();
        let s = Session {
            id: 3,
            state: State::Waiting,
            command: "claude".into(),
            agent: "claude".into(),
            cwd: "C:/r/wt/src".into(),
            branch: "wip".into(),
            title: "fix".into(),
            note: "Allow?".into(),
            since_ms: 0,
            read_only: false,
        };
        a.take(Answer::Claude(Some(vec![(PathBuf::from("C:/r/wt"), vec![s.clone()])])));
        assert_eq!(a.claude_in(Path::new("C:/r/wt")), [s]);
        assert!(a.claude_in(Path::new("C:/r/repo")).is_empty());
        a.take(Answer::Claude(None));
        assert!(a.claude_in(Path::new("C:/r/wt")).is_empty());
    }
}
