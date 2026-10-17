//! The repository screen (`<C-o>`): the repositories michi was opened on,
//! newest first, to switch to without leaving the window, drawn by
//! `ito-palette`. What is typed narrows the list; a typed path (a drive, a
//! slash, `~` or `.` first) adds a row that opens it. The list is kept in
//! `recent.txt` in michi's folder. A folder is read on a thread first and
//! only a repository replaces the one in the window.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use eframe::egui::{self, Key, Modifiers};
use ito_palette::{Answer, Entry, Style, Tone, View, Words};
use michi_core::recent::{self, Recent};
use michi_core::summary::Summary;

use crate::app::App;

/// What a row of the screen stands for.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    /// A repository of the list.
    Recent(PathBuf),
    /// The path that was typed.
    Typed(PathBuf),
}

/// A folder being read before the window switches to it.
pub struct Opening {
    pub dir: PathBuf,
    pub answer: Receiver<Result<Summary, michi_core::git::Error>>,
}

/// The file the list is kept in.
pub fn file(base: Option<&Path>) -> Option<PathBuf> {
    crate::prefs::window_dir(base).map(|d| d.join("recent.txt"))
}

pub fn load(base: Option<&Path>) -> Recent {
    file(base).and_then(|f| std::fs::read_to_string(f).ok()).map(|t| Recent::parse(&t)).unwrap_or_default()
}

fn home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from)
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
    v.keys = vec![(Modifiers::NONE, Key::Delete)];
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
    let entries: Vec<Entry<Row>> = rows(&app.recent, &v.query)
        .into_iter()
        .map(|row| match &row {
            Row::Typed(p) => {
                let mut e = Entry::new(app.tr("repos.open").replace("{}", &p.display().to_string()), "", row.clone());
                e.tone = Tone::Accent;
                e
            }
            Row::Recent(p) => {
                let is_here = here.as_deref().is_some_and(|h| michi_core::paths::same_path(h, p));
                let mut e = Entry::new(leaf(p), if is_here { app.tr("repos.here") } else { "" }, row.clone());
                e.detail = parent(p);
                e
            }
        })
        .collect();
    let words =
        Words { hint: app.tr("repos.hint"), none: app.tr("repos.none"), header: app.tr("repos.title"), footer: app.tr("repos.keys") };
    match ito_palette::show(ctx, &mut v, &entries, &style, &words) {
        None => app.repos = Some(v),
        Some(Answer::Pick(Row::Recent(p) | Row::Typed(p))) => app.open_repo(p),
        Some(Answer::Key(_, _, Row::Recent(p))) => {
            app.forget_repo(&p);
            app.repos = Some(v);
        }
        Some(Answer::Key(..)) => app.repos = Some(v),
        Some(Answer::Close) => {}
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
            || self.band.is_some()
            || self.tidy.is_some()
            || self.reorder.is_some()
            || self.conflicts.edit.is_some();
        if !busy {
            self.repos = Some(view());
        }
    }

    /// Ask to switch the window to `dir`; it is read first (see [`Opening`]).
    pub fn open_repo(&mut self, dir: PathBuf) {
        self.open_req = Some(dir);
    }

    pub fn forget_repo(&mut self, dir: &Path) {
        if self.recent.forget(dir) {
            self.save_recent();
        }
    }

    /// `top` put in front of the list, and the list kept.
    pub fn remember_repo(&mut self, top: &Path) {
        self.recent.push(top);
        self.save_recent();
    }

    fn save_recent(&self) {
        let Some(file) = file(self.prefs.base.as_deref()) else { return };
        let text = self.recent.text();
        std::thread::spawn(move || {
            if let Some(dir) = file.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = ito_common::write_atomic(&file, text);
        });
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
}
