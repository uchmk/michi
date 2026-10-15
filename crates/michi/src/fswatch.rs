//! Watching the repository's folders for changes (`notify`): the watching
//! thread reads the worktrees again when one comes, not every few seconds.
//!
//! Starting to watch a folder opens a handle on it (`ReadDirectoryChangesW`
//! on Windows), which can hang on a network drive, so the watcher lives on a
//! thread of its own: the watching thread only hands it the folders it wants.
//! A folder on a network share (`\\server\share`) is not watched, and a folder
//! that cannot be watched says so: the worktrees are then read every few
//! seconds as before.

use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};

use notify::{RecursiveMode, Watcher as _};

/// What the watcher says.
#[derive(Debug)]
pub enum Event {
    /// These paths changed (or were made, or went).
    Paths(Vec<PathBuf>),
    /// Something changed, it is not known where (too many changes at once).
    Lost,
    /// Whether every folder asked for is watched now.
    Watching(bool),
}

pub struct Fs {
    tx: Option<Sender<Vec<PathBuf>>>,
    /// What was asked for last.
    sent: Vec<PathBuf>,
}

impl Fs {
    /// Starts the watcher's thread; what it sees goes to `on`.
    pub fn start(on: impl Fn(Event) + Send + Sync + 'static) -> Fs {
        let (tx, rx) = mpsc::channel::<Vec<PathBuf>>();
        let spawned = std::thread::Builder::new().name("fswatch".into()).spawn(move || run(&rx, on));
        if spawned.is_err() {
            eprintln!("michi: the folders are not watched");
        }
        Fs { tx: spawned.is_ok().then_some(tx), sent: Vec::new() }
    }

    /// Watch exactly `roots`, each with all below it; whether that is new to
    /// the watcher (it says [`Event::Watching`] once it is done).
    pub fn watch(&mut self, roots: Vec<PathBuf>) -> bool {
        let Some(tx) = &self.tx else { return false };
        if roots == self.sent {
            return false;
        }
        if tx.send(roots.clone()).is_err() {
            self.tx = None;
            return false;
        }
        self.sent = roots;
        true
    }
}

/// The watcher's thread, until [`Fs`] is dropped.
fn run(rx: &mpsc::Receiver<Vec<PathBuf>>, on: impl Fn(Event) + Send + Sync + 'static) {
    let on = std::sync::Arc::new(on);
    let tell = on.clone();
    let made = notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
        Ok(ev) if matches!(ev.kind, notify::EventKind::Access(_)) => {}
        Ok(ev) if ev.need_rescan() || ev.paths.is_empty() => tell(Event::Lost),
        Ok(ev) => tell(Event::Paths(ev.paths)),
        Err(_) => tell(Event::Lost),
    });
    let mut inner = match made {
        Ok(w) => w,
        Err(e) => {
            eprintln!("michi: the folders are not watched: {e}");
            on(Event::Watching(false));
            return;
        }
    };
    let mut watched: Vec<PathBuf> = Vec::new();
    while let Ok(mut wanted) = rx.recv() {
        // Only the newest wish counts.
        while let Ok(newer) = rx.try_recv() {
            wanted = newer;
        }
        for gone in watched.iter().filter(|p| !wanted.contains(p)) {
            let _ = inner.unwatch(gone);
        }
        watched.retain(|p| wanted.contains(p));
        let mut all = true;
        for p in &wanted {
            if watched.contains(p) {
                continue;
            }
            if on_share(p) {
                all = false;
                continue;
            }
            match inner.watch(p, RecursiveMode::Recursive) {
                Ok(()) => watched.push(p.clone()),
                Err(e) => {
                    eprintln!("michi: {} is not watched: {e}", p.display());
                    all = false;
                }
            }
        }
        on(Event::Watching(all));
    }
}

/// Whether `p` is on a network share, where a change may never be told.
fn on_share(p: &std::path::Path) -> bool {
    let s = p.to_string_lossy().replace('\\', "/");
    let s = s.strip_prefix("//?/UNC/").map(|r| format!("//{r}")).unwrap_or(s);
    s.starts_with("//") && !s.starts_with("//?/") && !s.starts_with("//./")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn shares_are_told_from_local_folders() {
        assert!(on_share(Path::new(r"\\server\share\repo")));
        assert!(on_share(Path::new("//server/share/repo")));
        assert!(on_share(Path::new(r"\\?\UNC\server\share\repo")));
        assert!(!on_share(Path::new(r"\\?\C:\dev\repo")));
        assert!(!on_share(Path::new("C:/dev/repo")));
        assert!(!on_share(Path::new("/home/u/repo")));
    }
}
