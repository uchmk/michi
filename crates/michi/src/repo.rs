//! Reading the repository off the UI thread: one thread per window asks git
//! and sends the answers back, waking the frame after each.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};

use michi_core::git::{self, Version};
use michi_core::summary::{self, Summary};

/// What the reading thread sends back.
#[derive(Debug)]
pub enum Answer {
    /// The git on the PATH.
    Version(Result<Version, git::Error>),
    /// The repository the window is on.
    Summary(Result<Summary, git::Error>),
}

/// Start reading `dir`: git's version first, then the summary.
pub fn start(dir: PathBuf, ctx: egui::Context) -> Receiver<Answer> {
    let (tx, rx) = mpsc::channel();
    let send = |tx: &Sender<Answer>, a: Answer, ctx: &egui::Context| {
        let _ = tx.send(a);
        ctx.request_repaint();
    };
    let thread_tx = tx.clone();
    let spawned = std::thread::Builder::new().name("repo".into()).spawn(move || {
        send(&thread_tx, Answer::Version(git::version()), &ctx);
        send(&thread_tx, Answer::Summary(summary::summary(&dir)), &ctx);
    });
    if let Err(e) = spawned {
        // No thread: say so where the summary would have gone.
        let _ = tx.send(Answer::Summary(Err(git::Error::Io(e.to_string()))));
    }
    rx
}
