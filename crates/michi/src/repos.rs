//! The repository screen (`<C-o>`, or a click on the title bar's repository
//! name): the repositories michi was opened on, the pinned ones first and the
//! rest newest first, to switch to without leaving the window, drawn by
//! `ito-palette`. What is typed narrows the list; a typed path (a drive, a
//! slash, `~` or `.` first) adds a row that opens it; the first row chooses a
//! folder in the native dialog. A row says whether it is here, in a tab or gone. The list is kept in
//! `recent.toml` in michi's state folder. A folder is read on a thread first and
//! only a repository replaces the one in the window.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};

use eframe::egui::{self, Key, Modifiers};
use ito_palette::{Answer, Entry, Style, Tone, View, Words};
use michi_core::clone;
use michi_core::recent::{self, Recent};
use michi_core::summary::Summary;

use crate::app::App;

/// What a row of the screen stands for.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    /// Choose a folder in the native dialog.
    Browse,
    /// A repository of the list.
    Recent(PathBuf),
    /// The path that was typed.
    Typed(PathBuf),
    /// An address that was typed, and where its clone goes.
    Clone(String, PathBuf),
}

/// A `git clone` running on a thread.
pub struct Cloning {
    pub dest: PathBuf,
    /// The address being cloned, for the sign-in help when it fails.
    pub url: String,
    /// `Ok(true)` cloned, `Ok(false)` stopped (the half-made folder is gone).
    pub answer: Receiver<Result<bool, michi_core::git::Error>>,
    /// Set to stop it (Esc, or a click on the keybar's status).
    pub stop: Arc<AtomicBool>,
    /// The last progress git told.
    pub progress: Arc<Mutex<Option<michi_core::net::Progress>>>,
}

impl Cloning {
    pub fn stopping(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    pub fn progress(&self) -> Option<michi_core::net::Progress> {
        self.progress.lock().ok().and_then(|p| p.clone())
    }
}

/// The row that clones what `query` asks for into `parent` (the folder
/// beside the open repository), when it asks for a clone and a folder name
/// can be made of it.
pub fn clone_row(query: &str, parent: &Path) -> Option<Row> {
    let req = clone::parse(query)?;
    let dest = clone::target(&req, parent, home().as_deref())?;
    Some(Row::Clone(req.url, dest))
}

/// Which listed folders are gone. Looking is disk work, so it is done on a
/// thread when the screen opens and after the list changes, never per frame.
#[derive(Default)]
pub struct Seen {
    pub missing: Vec<PathBuf>,
    /// The list changed (or the screen just opened): look again.
    pub stale: bool,
    asked: Option<Receiver<Vec<PathBuf>>>,
}

impl Seen {
    pub fn is_missing(&self, p: &Path) -> bool {
        self.missing.iter().any(|m| michi_core::paths::same_path(m, p))
    }

    /// Start looking at `paths` when the list is stale; take the answer when it is in.
    fn poll(&mut self, ctx: &egui::Context, paths: Vec<PathBuf>) {
        if std::mem::take(&mut self.stale) {
            let (tx, rx) = mpsc::channel();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let gone = paths.into_iter().filter(|p| !p.is_dir()).collect();
                let _ = tx.send(gone);
                ctx.request_repaint();
            });
            self.asked = Some(rx);
        }
        if let Some(gone) = self.asked.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.missing = gone;
            self.asked = None;
        }
    }
}

/// A folder being read before the window switches to it.
pub struct Opening {
    pub dir: PathBuf,
    pub answer: Receiver<Result<Summary, michi_core::git::Error>>,
}

/// The file the list is kept in.
pub fn file(base: Option<&Path>) -> Option<PathBuf> {
    crate::prefs::state_folder(base).map(|d| d.join("recent.toml"))
}

pub fn load(base: Option<&Path>) -> Recent {
    crate::prefs::state_text(base, "recent.toml", "recent.txt").map(|t| Recent::parse(&t)).unwrap_or_default()
}

fn home() -> Option<PathBuf> {
    ito_common::home_dir()
}

/// The rows for `query`: the typed path first when it is one, then the list
/// where every word of the query is in the path (all of it, for a path).
pub fn rows(recent: &Recent, query: &str) -> Vec<Row> {
    let mut out = Vec::new();
    let path = recent::looks_like_path(query);
    if path {
        out.push(Row::Typed(recent::expand(query, home().as_deref())));
    }
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    for p in recent.paths() {
        let key = p.to_string_lossy().to_lowercase();
        if path || words.iter().all(|w| key.contains(w.as_str())) {
            out.push(Row::Recent(p.clone()));
        }
    }
    out
}

fn view() -> View {
    let mut v = View::new();
    v.filter = false;
    // Del forgets the row; Ctrl+D (Cmd+D) pins it, or lets its pin go.
    v.keys = vec![(Modifiers::NONE, Key::Delete), (Modifiers::COMMAND, Key::D)];
    v
}

fn leaf(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}

fn parent(p: &Path) -> String {
    p.parent().map(|d| d.display().to_string()).unwrap_or_default()
}

/// Draw the screen and take its keys.
pub fn show(ctx: &egui::Context, app: &mut App) {
    let Some(mut v) = app.repos.take() else { return };
    let c = ito_theme::colors();
    let style = Style {
        panel: c.panel,
        border: c.border,
        text: c.fg,
        dim: c.dim,
        selected_bg: c.blue.gamma_multiply(0.22),
        selected_text: c.fg,
        hit: crate::ui::accent(),
        accent: crate::ui::accent(),
    };
    let here = match &app.repo {
        crate::app::Repo::Ready(s) => Some(s.top.clone()),
        _ => None,
    };
    let beside = here.as_deref().and_then(Path::parent).map(Path::to_path_buf).or_else(home).unwrap_or_default();
    app.repos_seen.poll(ctx, app.recent.paths());
    let mut found = rows(&app.recent, &v.query);
    let clone = clone_row(&v.query, &beside);
    // The folder dialog is the way in for anyone who has no path or address at hand.
    // First while nothing is typed; after the matches once a name is typed, so Enter still opens the match.
    if clone.is_none() && !recent::looks_like_path(&v.query) {
        if v.query.trim().is_empty() {
            found.insert(0, Row::Browse);
        } else {
            found.push(Row::Browse);
        }
    }
    if let Some(row) = clone {
        found.insert(0, row);
    }
    let entries: Vec<Entry<Row>> = found
        .into_iter()
        .map(|row| match &row {
            Row::Browse => {
                let mut e = Entry::new(app.tr("repos.browse"), "", row.clone());
                e.tone = Tone::Accent;
                e
            }
            Row::Typed(p) => {
                let mut e = Entry::new(app.tr("repos.open").replace("{}", &p.display().to_string()), "", row.clone());
                e.tone = Tone::Accent;
                e
            }
            Row::Clone(url, dest) => {
                let text = app.tr("repos.clone").replacen("{}", url, 1).replacen("{}", &dest.display().to_string(), 1);
                let mut e = Entry::new(text, "", row.clone());
                e.tone = Tone::Accent;
                e
            }
            Row::Recent(p) => {
                let label = state_label(
                    here.as_deref().is_some_and(|h| michi_core::paths::same_path(h, p)),
                    app.tab_paths.iter().any(|t| michi_core::paths::same_path(t, p)),
                    app.repos_seen.is_missing(p),
                );
                let word = label.map_or("", |l| app.tr(l));
                let name = if app.recent.is_pinned(p) { format!("\u{2605} {}", leaf(p)) } else { leaf(p) };
                let mut e = Entry::new(name, word, row.clone());
                e.detail = parent(p);
                e
            }
        })
        .collect();
    let words =
        Words { hint: app.tr("repos.hint"), none: app.tr("repos.none"), header: app.tr("repos.title"), footer: app.tr("repos.keys") };
    match ito_palette::show(ctx, &mut v, &entries, &style, &words) {
        None => app.repos = Some(v),
        Some(Answer::Pick(Row::Browse)) => app.browse_req = true,
        Some(Answer::Pick(Row::Recent(p))) if app.repos_seen.is_missing(&p) => {
            // The screen stays: Del is what the toast offers.
            let text = app.tr("repos.missing_pick").to_owned();
            app.say(text);
            app.repos = Some(v);
        }
        Some(Answer::Pick(Row::Recent(p) | Row::Typed(p))) => app.open_repo(p),
        Some(Answer::Pick(Row::Clone(url, dest))) => app.start_clone(ctx, url, dest),
        Some(Answer::Key(m, k, Row::Recent(p))) => {
            if k == Key::Delete && !m.command {
                app.forget_repo(&p);
            } else {
                app.toggle_pin_repo(&p);
            }
            app.repos = Some(v);
        }
        Some(Answer::Key(..)) => app.repos = Some(v),
        Some(Answer::Close) => {}
    }
}

/// The word (a language key) on the right of a listed folder row: it is the
/// repository of the window, it is open in another tab, or it is gone.
pub fn state_label(here: bool, in_tab: bool, missing: bool) -> Option<&'static str> {
    if here {
        Some("repos.here")
    } else if in_tab {
        Some("repos.tab")
    } else if missing {
        Some("repos.missing")
    } else {
        None
    }
}

impl App {
    /// Open the screen, or close it; not over a box that has the keys.
    pub fn toggle_repos(&mut self) {
        if self.repos.take().is_some() {
            return;
        }
        let busy = self.cmd.is_some()
            || self.picker.is_some()
            || self.new_work.is_some()
            || self.pr_form.is_some()
            || self.band.is_some()
            || self.tidy.is_some()
            || self.reflog.is_some()
            || self.stats.is_some()
            || self.ready.is_some()
            || self.blame.is_some()
            || self.reorder.is_some()
            || self.conflicts.edit.is_some();
        if !busy {
            self.repos = Some(view());
            self.repos_seen.stale = true;
        }
    }

    /// Ask to switch the window to `dir`; it is read first (see [`Opening`]).
    pub fn open_repo(&mut self, dir: PathBuf) {
        self.open_req = Some(dir);
    }

    /// `git clone url dest` on a thread; the window switches to it when it is done.
    pub fn start_clone(&mut self, ctx: &egui::Context, url: String, dest: PathBuf) {
        if self.cloning.is_some() {
            let text = self.tr("repos.clone_busy");
            self.say(text);
            return;
        }
        let text = self.tr("repos.cloning").replace("{}", &url);
        self.say_for(text, 8);
        let (tx, answer) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(Mutex::new(None));
        let (d, ctx, u, s, pr) = (dest.clone(), ctx.clone(), url.clone(), stop.clone(), progress.clone());
        std::thread::spawn(move || {
            let tell_ctx = ctx.clone();
            let mut tell = |p: michi_core::net::Progress| {
                if let Ok(mut g) = pr.lock() {
                    *g = Some(p);
                }
                tell_ctx.request_repaint();
            };
            let _ = tx.send(clone::run_stoppable(&u, &d, &s, &mut tell));
            ctx.request_repaint();
        });
        self.cloning = Some(Cloning { dest, url, answer, stop, progress });
    }

    pub fn forget_repo(&mut self, dir: &Path) {
        if self.recent.forget(dir) {
            self.repos_seen.stale = true;
            self.save_recent();
        }
    }

    /// The pin goes on `dir`, or comes off it.
    pub fn toggle_pin_repo(&mut self, dir: &Path) {
        self.recent.toggle_pin(dir);
        self.save_recent();
    }

    /// `top` put in front of the list, and the list kept.
    /// Not a throwaway one: nothing in the temporary folder, no practice repository.
    pub fn remember_repo(&mut self, top: &Path) {
        // The temporary folder is resolved once per run, not on every open.
        static TEMP: std::sync::OnceLock<(PathBuf, PathBuf)> = std::sync::OnceLock::new();
        let (temp, real) = TEMP.get_or_init(|| {
            let temp = std::env::temp_dir();
            let real = std::fs::canonicalize(&temp).unwrap_or_else(|_| temp.clone());
            (temp, real)
        });
        if !recent::worth_remembering(top, &[temp, real]) {
            return;
        }
        let before = self.recent.text();
        self.recent.push(top);
        // The same list is not written again.
        if self.recent.text() != before {
            self.save_recent();
        }
    }

    fn save_recent(&self) {
        let Some(file) = file(self.prefs.base.as_deref()) else { return };
        // One writer per file: the newest list wins and the UI never waits on the disk.
        static WRITERS: std::sync::Mutex<Vec<ito_common::kept::Writer>> = std::sync::Mutex::new(Vec::new());
        let mut all = WRITERS.lock().unwrap_or_else(|e| e.into_inner());
        let i = match all.iter().position(|w| w.path() == file) {
            Some(i) => i,
            None => {
                all.push(ito_common::kept::Writer::new(file, true));
                all.len() - 1
            }
        };
        all[i].save(self.recent.text());
    }
}

/// Read `dir` on a thread; the frame wakes when it is done.
pub fn start_opening(ctx: &egui::Context, dir: PathBuf) -> Opening {
    let (tx, answer) = mpsc::channel();
    let (d, ctx) = (dir.clone(), ctx.clone());
    std::thread::spawn(move || {
        let _ = tx.send(michi_core::summary::summary(&d));
        ctx.request_repaint();
    });
    Opening { dir, answer }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list() -> Recent {
        Recent::parse("/dev/michi\n/dev/kura\n/work/Michi-fork\n")
    }

    #[test]
    fn what_is_typed_narrows_the_list_by_every_word() {
        let r = list();
        assert_eq!(rows(&r, "").len(), 3);
        assert_eq!(rows(&r, "michi").len(), 2, "case is let go");
        assert_eq!(rows(&r, "michi fork"), vec![Row::Recent(PathBuf::from("/work/Michi-fork"))]);
        assert!(rows(&r, "nope").is_empty());
    }

    #[test]
    fn a_typed_path_is_the_first_row() {
        let r = list();
        let rs = rows(&r, "/dev/new");
        assert_eq!(rs[0], Row::Typed(PathBuf::from("/dev/new")));
        assert_eq!(rs.len(), 4, "the list stays under it");
    }

    #[test]
    fn an_address_makes_a_clone_row_beside_the_open_repository() {
        let parent = Path::new("/dev");
        assert!(clone_row("michi", parent).is_none());
        assert_eq!(clone_row("https://h/a/b.git", parent), Some(Row::Clone("https://h/a/b.git".into(), PathBuf::from("/dev/b"))));
        assert_eq!(clone_row("https://h/.git", parent), None);
        assert_eq!(clone_row("https://h/.git /x/y", parent), Some(Row::Clone("https://h/.git".into(), PathBuf::from("/x/y"))));
    }

    #[test]
    fn the_screen_opens_closes_and_asks_to_switch() {
        let mut a = App::new(crate::prefs::Prefs::empty(), std::env::temp_dir());
        a.toggle_repos();
        assert!(a.repos.is_some());
        a.toggle_repos();
        assert!(a.repos.is_none());
        a.remember_repo(Path::new("/x"));
        a.forget_repo(Path::new("/x"));
        assert!(a.recent.paths().is_empty());
        a.open_repo(PathBuf::from("/y"));
        assert_eq!(a.open_req, Some(PathBuf::from("/y")));
    }

    #[test]
    fn a_row_says_here_then_tab_then_missing() {
        assert_eq!(state_label(true, true, true), Some("repos.here"));
        assert_eq!(state_label(false, true, true), Some("repos.tab"));
        assert_eq!(state_label(false, false, true), Some("repos.missing"));
        assert_eq!(state_label(false, false, false), None);
    }

    #[test]
    fn seen_knows_a_gone_folder_by_its_path() {
        let seen = Seen { missing: vec![PathBuf::from("/gone/x")], ..Default::default() };
        assert!(seen.is_missing(Path::new("/gone/x/")));
        assert!(!seen.is_missing(Path::new("/here")));
    }

    #[test]
    fn throwaway_folders_are_not_remembered_and_pins_toggle() {
        let mut a = App::new(crate::prefs::Prefs::empty(), std::env::temp_dir());
        a.remember_repo(&std::env::temp_dir().join("m63"));
        a.remember_repo(Path::new("/home/u/michi-practice-2"));
        assert!(a.recent.paths().is_empty());
        a.remember_repo(Path::new("/dev/a"));
        a.toggle_pin_repo(Path::new("/dev/a"));
        assert!(a.recent.is_pinned(Path::new("/dev/a")));
        a.toggle_pin_repo(Path::new("/dev/a"));
        assert!(!a.recent.is_pinned(Path::new("/dev/a")));
    }
}
