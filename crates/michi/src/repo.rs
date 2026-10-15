//! Reading the repository off the UI thread: one thread per window asks git
//! and sends the answers back, waking the frame after each.
//!
//! git's version, the summary and the history's setup (refs and worktrees)
//! are asked at once; then come the commits a batch at a time — the first batch at
//! once, the next ones when the window asks for [`Ask::More`]. A
//! [`Ask::Reload`] starts the reading over; its answers carry the next
//! generation, so the window drops whatever was still on the way.
//!
//! The selected commit in full comes from a second thread, so a long diff
//! never holds up the history: only the newest wish is read, and one read
//! halfway stops when another comes.
//!
//! A third thread keeps an eye on the worktrees: each one's uncommitted
//! change, how it stands against the trunk and its upstream, and the Claude
//! sessions tsumugi runs in them, read again every few seconds while the
//! window is in front and seldom behind it.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use michi_core::claude::{self, Session};
use michi_core::git::{self, Version};
use michi_core::history::Setup;
use michi_core::log::{self, Commit, Reader};
use michi_core::oid::Oid;
use michi_core::show::{self, Detail};
use michi_core::summary::{self, Summary};
use michi_core::track::{self, Trees};
use michi_core::wip::{self, Wip};
use michi_core::worktree;

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
    /// The commit `id` in full, as [`Reading::detail`] asked.
    Detail { id: Oid, result: Result<Box<Detail>, git::Error> },
    /// The worktrees with uncommitted changes, each read anew (with those
    /// whose only change is too big to count).
    Wips(Vec<Wip>),
    /// How each worktree stands against the trunk and its upstream.
    Trees(Trees),
    /// The Claude sessions in each worktree (by its folder; only those with
    /// some); `None` when tsumugi is not there to ask.
    Claude(Option<Vec<(PathBuf, Vec<Session>)>>),
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

/// The ends the window holds.
pub struct Reading {
    pub answers: Receiver<Answer>,
    pub asks: Sender<Ask>,
    wants: Sender<Want>,
    /// The newest wish's number: a read for any other is stale.
    latest: Arc<AtomicU32>,
    /// Whether the window is in front, and wishes to read the worktrees
    /// now, for the watching thread.
    pokes: Sender<Poke>,
}

/// What the window tells the watching thread.
enum Poke {
    Front(bool),
    Now,
}

impl Reading {
    /// Read the commit `id` in full, with its diff when `patch`. A wish not
    /// yet answered is dropped.
    pub fn detail(&self, id: Oid, patch: bool) {
        let gen = self.latest.fetch_add(1, Ordering::SeqCst).wrapping_add(1);
        let _ = self.wants.send(Want { gen, id, patch });
    }

    /// The window came in front (`true`) or went behind: the worktrees are
    /// read more or less often.
    pub fn front(&self, front: bool) {
        let _ = self.pokes.send(Poke::Front(front));
    }

    /// Read the worktrees again now, not at the next turn.
    pub fn reread(&self) {
        let _ = self.pokes.send(Poke::Now);
    }
}

/// A commit the window wants in full.
struct Want {
    gen: u32,
    id: Oid,
    patch: bool,
}

/// Start reading `dir`: git's version and the summary, then the history.
pub fn start(dir: PathBuf, ctx: egui::Context) -> Reading {
    let (tx, answers) = mpsc::channel();
    let (asks, ask_rx) = mpsc::channel();
    let (wants, want_rx) = mpsc::channel();
    let latest = Arc::new(AtomicU32::new(0));
    let (detail_tx, detail_dir, detail_ctx, detail_latest) = (tx.clone(), dir.clone(), ctx.clone(), latest.clone());
    // No thread for the details: the detail stays without its files.
    let _ = std::thread::Builder::new().name("detail".into()).spawn(move || {
        let send = |a: Answer| {
            let _ = detail_tx.send(a);
            detail_ctx.request_repaint();
        };
        details(&detail_dir, &want_rx, &detail_latest, send);
    });
    let (pokes, poke_rx) = mpsc::channel();
    let (watch_tx, watch_dir, watch_ctx) = (tx.clone(), dir.clone(), ctx.clone());
    // No thread for the worktrees: no WIP rows and no Claude.
    let _ = std::thread::Builder::new().name("watch".into()).spawn(move || {
        let send = |a: Answer| {
            let _ = watch_tx.send(a);
            watch_ctx.request_repaint();
        };
        watch(&watch_dir, &poke_rx, send);
    });
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
    Reading { answers, asks, wants, latest, pokes }
}

/// How often tsumugi is asked while the window is in front.
const CLAUDE_FRONT: Duration = Duration::from_secs(2);
/// … and while it is behind.
const CLAUDE_BEHIND: Duration = Duration::from_secs(10);
/// How often the uncommitted changes and how the worktrees stand are read
/// while the window is in front; behind, they are not, and they are read at
/// once when it comes back.
const WIP_EVERY: Duration = Duration::from_secs(5);

/// Reads the worktrees' uncommitted changes and how they stand, and asks
/// tsumugi for its sessions, again and again, until the window is gone. An
/// answer is sent only when it differs from the last.
fn watch(dir: &Path, pokes: &Receiver<Poke>, send: impl Fn(Answer)) {
    let mut front = true;
    let mut wips_read: Option<Instant> = None;
    let (mut last_wips, mut last_trees, mut last_claude) = (None, None, None);
    let mut counts = track::Cache::default();
    let mut timeouts = 0u32;
    loop {
        let tick = Instant::now();
        let worktrees = worktree::list(dir).ok();
        if let Some(wts) = worktrees.as_ref().filter(|_| front && wips_read.is_none_or(|t| t.elapsed() >= WIP_EVERY)) {
            wips_read = Some(Instant::now());
            // A worktree that cannot be read has no row; the next read tries again.
            let wips: Vec<Wip> = wip::read_all(wts).into_iter().filter_map(|r| r.ok().flatten()).collect();
            if last_wips.as_ref() != Some(&wips) {
                last_wips = Some(wips.clone());
                send(Answer::Wips(wips));
            }
            match track::read(dir, wts, &mut counts) {
                Ok(trees) if last_trees.as_ref() != Some(&trees) => {
                    last_trees = Some(trees.clone());
                    send(Answer::Trees(trees));
                }
                Ok(_) => {}
                Err(e) => eprintln!("michi: {e}"),
            }
        }
        let claude = match claude::fetch() {
            Ok(None) => Some(None),
            Ok(Some(sessions)) => {
                let wts = worktrees.as_deref().unwrap_or_default();
                let per = claude::assign(wts.iter().map(|w| &w.path), &sessions);
                Some(Some(wts.iter().zip(per).filter(|(_, s)| !s.is_empty()).map(|(w, s)| (w.path.clone(), s)).collect()))
            }
            // Worth a line, not every 2 s: the first and every 30th.
            Err(e) => {
                if !matches!(e, claude::Error::Timeout) || timeouts.is_multiple_of(30) {
                    eprintln!("michi: {e}");
                }
                timeouts += u32::from(matches!(e, claude::Error::Timeout));
                None
            }
        };
        if let Some(c) = claude {
            if last_claude.as_ref() != Some(&c) {
                last_claude = Some(c.clone());
                send(Answer::Claude(c));
            }
        }
        // Wait out the tick; coming in front, or a wish to read now, cuts it
        // short.
        let every = if front { CLAUDE_FRONT } else { CLAUDE_BEHIND };
        loop {
            let left = every.saturating_sub(tick.elapsed());
            match pokes.recv_timeout(left) {
                Ok(Poke::Front(f)) => {
                    let came = f && !front;
                    front = f;
                    if came {
                        wips_read = None;
                        break;
                    }
                }
                Ok(Poke::Now) => {
                    wips_read = None;
                    break;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        }
    }
}

/// Reads each commit the window wants in full, until the window is gone.
/// Wishes that came while one was read are dropped but the newest, and a
/// read is stopped (and dropped) once a newer wish comes.
fn details(dir: &Path, wants: &Receiver<Want>, latest: &AtomicU32, send: impl Fn(Answer)) {
    while let Ok(mut want) = wants.recv() {
        while let Ok(newer) = wants.try_recv() {
            want = newer;
        }
        let stale = || latest.load(Ordering::SeqCst) != want.gen;
        if stale() {
            continue;
        }
        let result = show::read(dir, &want.id, want.patch, &stale).map(Box::new);
        if !stale() {
            send(Answer::Detail { id: want.id, result });
        }
    }
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
