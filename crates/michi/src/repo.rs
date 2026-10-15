//! Reading the repository off the UI thread: one thread per window asks git
//! and sends the answers back, waking the frame after each.
//!
//! git's version, the summary and the history's setup (refs and worktrees)
//! are asked at once; then come the commits a batch at a time — the first batch at
//! once, the next ones when the window asks for [`Ask::More`]. A
//! [`Ask::Reload`] starts the reading over; its answers carry the next
//! generation, so the window drops whatever was still on the way.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use michi_core::git::{self, Version};
use michi_core::history::Setup;
use michi_core::log::{self, Commit, Reader};
use michi_core::summary::{self, Summary};

/// What the reading thread sends back.
#[derive(Debug)]
pub enum Answer {
    /// The git on the PATH.
    Version(Result<Version, git::Error>),
    /// The repository the window is on.
    Summary(Result<Summary, git::Error>),
    /// A reading of the history starts: what it is read against.
    Setup { gen: u32, setup: Box<Setup> },
    /// The next commits, older than the last batch's; `done` at the end.
    Commits { gen: u32, batch: Vec<Commit>, done: bool },
    /// How many commits the whole history has.
    Total { gen: u32, count: u64 },
    /// The history could not be read.
    Failed { gen: u32, error: git::Error },
}

/// What the window asks of the reading thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// The next batch.
    More,
    /// Read the history again from the top.
    #[expect(dead_code, reason = "sent once the watcher and the commands come in (stage 2 on)")]
    Reload,
}

/// The first batch: enough for the first screen and some way down.
pub const FIRST: usize = 2000;
/// Each batch after it.
pub const NEXT: usize = 5000;

/// The two ends the window holds.
pub struct Reading {
    pub answers: Receiver<Answer>,
    pub asks: Sender<Ask>,
}

/// Start reading `dir`: git's version and the summary, then the history.
pub fn start(dir: PathBuf, ctx: egui::Context) -> Reading {
    let (tx, answers) = mpsc::channel();
    let (asks, ask_rx) = mpsc::channel();
    let thread_tx = tx.clone();
    let spawned = std::thread::Builder::new().name("repo".into()).spawn(move || {
        let send = |a: Answer| {
            let _ = thread_tx.send(a);
            ctx.request_repaint();
        };
        let (version, summary, setup) = first(&dir);
        send(Answer::Version(version));
        let ok = summary.is_ok();
        send(Answer::Summary(summary));
        if ok {
            history(&dir, setup, &ask_rx, send);
        }
    });
    if let Err(e) = spawned {
        // No thread: say so where the summary would have gone.
        let _ = tx.send(Answer::Summary(Err(git::Error::Io(e.to_string()))));
    }
    Reading { answers, asks }
}

/// git's version, the summary and the history's setup. Each git call costs
/// mostly the start of its process, so they are asked at once.
pub(crate) fn first(dir: &Path) -> (Result<Version, git::Error>, Result<Summary, git::Error>, Result<Setup, git::Error>) {
    let ((version, summary), setup) = git::both(|| git::both(git::version, || summary::summary(dir)), || Setup::read(dir));
    (version, summary, setup)
}

/// Reads the history of `dir`, once per generation, until the window is
/// gone; the first reading with the setup read already.
fn history(dir: &Path, first: Result<Setup, git::Error>, asks: &Receiver<Ask>, send: impl Fn(Answer)) {
    let mut first = Some(first);
    let mut gen = 0;
    loop {
        gen += 1;
        let setup = first.take().unwrap_or_else(|| Setup::read(dir));
        match read(dir, gen, setup, asks, &send) {
            Next::Reload => {}
            Next::Stop => return,
        }
    }
}

enum Next {
    Reload,
    Stop,
}

fn read(dir: &Path, gen: u32, setup: Result<Setup, git::Error>, asks: &Receiver<Ask>, send: &impl Fn(Answer)) -> Next {
    let failed = |error| {
        send(Answer::Failed { gen, error });
        wait_reload(asks)
    };
    let setup = match setup {
        Ok(s) => s,
        Err(e) => return failed(e),
    };
    let heads = setup.heads();
    send(Answer::Setup { gen, setup: Box::new(setup) });
    let mut log = match Reader::start(dir, &heads) {
        Ok(l) => l,
        Err(e) => return failed(e),
    };
    let batch = |log: &mut Reader, n| match log.next_batch(n) {
        Ok(b) => {
            send(Answer::Commits { gen, batch: b, done: log.done() });
            true
        }
        Err(e) => {
            send(Answer::Failed { gen, error: e });
            false
        }
    };
    if !batch(&mut log, FIRST) {
        return wait_reload(asks);
    }
    if !log.done() {
        if let Ok(count) = log::count(dir, &heads) {
            send(Answer::Total { gen, count });
        }
    }
    loop {
        match asks.recv() {
            Ok(Ask::More) if !log.done() => {
                if !batch(&mut log, NEXT) {
                    return wait_reload(asks);
                }
            }
            Ok(Ask::More) => {}
            Ok(Ask::Reload) => return Next::Reload,
            Err(_) => return Next::Stop,
        }
    }
}

/// Nothing more to read until the window asks for it again.
fn wait_reload(asks: &Receiver<Ask>) -> Next {
    loop {
        match asks.recv() {
            Ok(Ask::Reload) => return Next::Reload,
            Ok(Ask::More) => {}
            Err(_) => return Next::Stop,
        }
    }
}
