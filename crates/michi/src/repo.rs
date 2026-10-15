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
//! A fourth thread reads for the branch review: a branch's files from where
//! it left the trunk, and one file's diff, each only for the newest wish,
//! and keeps the files checked as looked at.
//!
//! A third thread keeps an eye on the worktrees: each one's uncommitted
//! change, how it stands against the trunk and its upstream, which would
//! conflict with which, the commits not seen yet, and the Claude sessions
//! tsumugi runs in them. The worktrees are read again when a change on disk
//! could show (once the folders are quiet for 300 ms), or every few seconds
//! where the folders cannot be watched; the Claude sessions every few
//! seconds while the window is in front and seldom behind it. When the refs
//! moved, the history is read again.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use michi_core::change::{self, Places};
use michi_core::clash::{self, Clashes};
use michi_core::claude::{self, Session};
use michi_core::git::{self, Version};
use michi_core::history::Setup;
use michi_core::log::{self, Commit, Reader};
use michi_core::oid::Oid;
use michi_core::review::{self, Checks, List};
use michi_core::show::{self, Changes, Detail};
use michi_core::summary::{self, Summary};
use michi_core::track::{self, Trees};
use michi_core::unseen::{self, Marks, Unseen};
use michi_core::wip::{self, Wip};
use michi_core::worktree::{self, Worktree};

use crate::fswatch::{self, Fs};
use crate::review::Wish;

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
    /// Which worktrees would conflict with which.
    Clashes(Clashes),
    /// The commits not seen yet, per worktree.
    Unseen(Unseen),
    /// The Claude sessions in each worktree (by its folder; only those with
    /// some); `None` when tsumugi is not there to ask.
    Claude(Option<Sessions>),
    /// The files of a branch review, as [`Reading::review`] asked, and those
    /// of its branch checked as looked at.
    ReviewList { gen: u32, result: Result<Box<List>, git::Error>, checks: BTreeSet<review::Key> },
    /// One file's diff in a branch review.
    ReviewFile { gen: u32, result: Result<Box<Changes>, git::Error> },
}

/// What the window asks of the reading thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// The next batch.
    More,
    /// Read the history again from the top.
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
    /// What the branch review wants, and the newest list's and diff's
    /// numbers.
    looks: Sender<Wish>,
    look_latest: Arc<[AtomicU32; 2]>,
}

/// What the window, and the folders' watcher, tell the watching thread.
enum Poke {
    Front(bool),
    Now,
    /// The commit was selected: it is seen now.
    Seen(Oid),
    /// A change on disk.
    Fs(fswatch::Event),
    /// The window is gone.
    Stop,
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

    /// The commit `id` was selected in the graph: it is seen now.
    pub fn seen(&self, id: Oid) {
        let _ = self.pokes.send(Poke::Seen(id));
    }
}

impl Reading {
    /// What the branch review wants read; an older wish of the same kind
    /// not yet answered is dropped.
    pub fn review(&self, wish: Wish) {
        match &wish {
            Wish::List { gen, .. } => self.look_latest[0].store(*gen, Ordering::SeqCst),
            Wish::File { gen, .. } => self.look_latest[1].store(*gen, Ordering::SeqCst),
            Wish::Check { .. } => {}
        }
        let _ = self.looks.send(wish);
    }

    /// The branch review is closed: what it asked is not read on.
    pub fn review_closed(&self) {
        for l in self.look_latest.iter() {
            l.store(0, Ordering::SeqCst);
        }
    }
}

impl Drop for Reading {
    fn drop(&mut self) {
        // The folders' watcher holds a sender too: the watching thread would
        // wait on for ever.
        let _ = self.pokes.send(Poke::Stop);
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
    let (watch_tx, watch_dir, watch_ctx, watch_asks, fs_pokes) = (tx.clone(), dir.clone(), ctx.clone(), asks.clone(), pokes.clone());
    // No thread for the worktrees: no WIP rows and no Claude.
    let _ = std::thread::Builder::new().name("watch".into()).spawn(move || {
        let send = |a: Answer| {
            let _ = watch_tx.send(a);
            watch_ctx.request_repaint();
        };
        let fs = Fs::start(move |e| {
            let _ = fs_pokes.send(Poke::Fs(e));
        });
        watch(&watch_dir, &poke_rx, &watch_asks, fs, send);
    });
    let (looks, look_rx) = mpsc::channel();
    let look_latest = Arc::new([AtomicU32::new(0), AtomicU32::new(0)]);
    let (look_tx, look_dir, look_ctx, latests) = (tx.clone(), dir.clone(), ctx.clone(), look_latest.clone());
    // No thread for the review: it stays empty.
    let _ = std::thread::Builder::new().name("review".into()).spawn(move || {
        let send = |a: Answer| {
            let _ = look_tx.send(a);
            look_ctx.request_repaint();
        };
        reviews(&look_dir, &look_rx, &latests, send);
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
    Reading { answers, asks, wants, latest, pokes, looks, look_latest }
}

/// How long the folders must stay quiet after a change before it is read.
const QUIET: Duration = Duration::from_millis(300);
/// … but no longer than this after the first change, while they keep changing.
const BUSY: Duration = Duration::from_secs(2);
/// How often the worktrees are read anyway while the folders are watched,
/// for a change the watcher missed.
const ANYWAY: Duration = Duration::from_secs(30);
/// How often the worktrees are read while the window is in front and the
/// folders are not watched. Behind, they are not read, and they are read at
/// once when it comes back.
const WIP_EVERY: Duration = Duration::from_secs(5);
/// How often tsumugi is asked while the window is in front.
const CLAUDE_FRONT: Duration = Duration::from_secs(2);
/// … and while it is behind.
const CLAUDE_BEHIND: Duration = Duration::from_secs(10);

/// tsumugi's Claude sessions, by the folder each runs in.
pub type Sessions = Vec<(PathBuf, Vec<Session>)>;

/// What the watching thread sent last, to send only what changed.
#[derive(Default)]
struct Last {
    wips: Option<Vec<Wip>>,
    trees: Option<Trees>,
    clashes: Option<Clashes>,
    unseen: Option<Unseen>,
    claude: Option<Option<Sessions>>,
    /// Every ref and every worktree's `HEAD`: when it changes, the history
    /// is read again.
    refs: Option<String>,
}

/// Reads the worktrees' uncommitted changes, how they stand, which would
/// conflict and what is not seen yet, when a change on disk could show (or
/// every few seconds where the folders are not watched), and asks tsumugi
/// for its sessions every few seconds, until the window is gone. An answer
/// is sent only when it differs from the last.
fn watch(dir: &Path, pokes: &Receiver<Poke>, asks: &Sender<Ask>, mut fs: Fs, send: impl Fn(Answer)) {
    let common = unseen::common_dir(dir).ok();
    let marks_file = common.as_deref().and_then(|c| state_path(c, "seen"));
    let mut marks = marks_file.as_deref().and_then(|f| std::fs::read_to_string(f).ok()).map(|t| Marks::parse(&t)).unwrap_or_default();
    let mut last = Last::default();
    let mut caches = (track::Cache::default(), clash::Cache::default(), unseen::Cache::default());
    let mut worktrees: Vec<Worktree> = Vec::new();
    let (mut front, mut force, mut watching) = (true, true, false);
    let (mut read_at, mut claude_at): (Option<Instant>, Option<Instant>) = (None, None);
    // The changes on disk not read yet: where (`lost` when it is not known),
    // and when the first and the last came.
    let mut changed: HashSet<PathBuf> = HashSet::new();
    let mut lost = false;
    let mut pending: Option<(Instant, Instant)> = None;
    let mut places: Option<Places> = None;
    let mut ignored = change::Ignored::default();
    let mut timeouts = 0u32;
    loop {
        let poll = if watching { ANYWAY } else { WIP_EVERY };
        let mut read_now = front && (force || read_at.is_none_or(|t| t.elapsed() >= poll));
        let settled = pending.is_some_and(|(first, last)| last.elapsed() >= QUIET || first.elapsed() >= BUSY);
        if !read_now && front && settled {
            let paths: Vec<PathBuf> = changed.drain().collect();
            read_now = std::mem::take(&mut lost) || places.as_ref().is_none_or(|p| change::matters(p, &paths, &mut ignored));
            pending = None;
        }
        if read_now {
            (force, lost, pending) = (false, false, None);
            changed.clear();
            read_at = Some(Instant::now());
            if let Ok(wts) = worktree::list(dir) {
                worktrees = wts;
                read_trees(dir, &worktrees, &mut marks, marks_file.as_deref(), &mut caches, &mut last, &send);
                let refs = refs_print(dir, &worktrees);
                if refs.is_some() && last.refs.is_some() && refs != last.refs {
                    let _ = asks.send(Ask::Reload);
                }
                last.refs = refs.or(last.refs.take());
                if let Some(common) = &common {
                    let trees: Vec<PathBuf> =
                        worktrees.iter().filter(|w| !w.bare && w.prunable.is_none()).map(|w| w.path.clone()).collect();
                    let p = Places::new(common, &trees);
                    if fs.watch(p.roots(common)) {
                        watching = false;
                    }
                    places = Some(p);
                }
            }
        }
        let every = if front { CLAUDE_FRONT } else { CLAUDE_BEHIND };
        if claude_at.is_none_or(|t| t.elapsed() >= every) {
            claude_at = Some(Instant::now());
            ask_claude(&worktrees, &mut last, &mut timeouts, &send);
        }
        // Wait for the next thing due; a poke, or a change on disk, may cut
        // it short.
        let now = Instant::now();
        let mut due = claude_at.map_or(now, |t| t + every);
        if front {
            due = due.min(read_at.map_or(now, |t| t + poll));
            if let Some((first, last)) = pending {
                due = due.min((last + QUIET).min(first + BUSY));
            }
        }
        match pokes.recv_timeout(due.saturating_duration_since(now)) {
            Ok(Poke::Front(f)) => {
                force |= f && !front;
                front = f;
            }
            Ok(Poke::Now) => force = true,
            Ok(Poke::Seen(id)) => {
                if let Some(u) = last.unseen.as_mut() {
                    if marks.see(u, &id) {
                        save_marks(&marks, marks_file.as_deref());
                    }
                    if u.remove(&id) {
                        send(Answer::Unseen(u.clone()));
                    }
                }
            }
            Ok(Poke::Fs(fswatch::Event::Paths(paths))) if front => {
                let before = changed.len();
                if !lost {
                    changed.extend(paths.into_iter().filter(|p| !places.as_ref().is_some_and(|pl| change::needless(pl, &ignored, p))));
                    if changed.len() > change::MOST {
                        changed.clear();
                        lost = true;
                    }
                }
                if lost || changed.len() > before {
                    pending = Some(bump(pending));
                }
            }
            Ok(Poke::Fs(fswatch::Event::Lost)) if front => {
                changed.clear();
                lost = true;
                pending = Some(bump(pending));
            }
            // Behind, a change is not kept: everything is read when the
            // window comes back.
            Ok(Poke::Fs(fswatch::Event::Paths(_) | fswatch::Event::Lost)) => {}
            Ok(Poke::Fs(fswatch::Event::Watching(ok))) => watching = ok,
            Ok(Poke::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => return,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

/// When the first change not read yet came, and now.
fn bump(pending: Option<(Instant, Instant)>) -> (Instant, Instant) {
    let now = Instant::now();
    (pending.map_or(now, |(first, _)| first), now)
}

/// Reads each worktree's uncommitted change, how it stands, which would
/// conflict and what is not seen yet, sending what changed.
fn read_trees(
    dir: &Path,
    wts: &[Worktree],
    marks: &mut Marks,
    marks_file: Option<&Path>,
    (counts, predictions, news): &mut (track::Cache, clash::Cache, unseen::Cache),
    last: &mut Last,
    send: &impl Fn(Answer),
) {
    // A worktree that cannot be read has no row; the next read tries again.
    let wips: Vec<Wip> = wip::read_all(wts).into_iter().filter_map(|r| r.ok().flatten()).collect();
    if last.wips.as_ref() != Some(&wips) {
        last.wips = Some(wips.clone());
        send(Answer::Wips(wips.clone()));
    }
    let mut trunk = None;
    match track::read(dir, wts, counts) {
        Ok(trees) => {
            trunk = trees.trunk.clone().zip(trees.trunk_head);
            let (unseen, marked) = unseen::read(dir, &trees, marks, news);
            if marked {
                save_marks(marks, marks_file);
            }
            if last.unseen.as_ref() != Some(&unseen) {
                last.unseen = Some(unseen.clone());
                send(Answer::Unseen(unseen));
            }
            if last.trees.as_ref() != Some(&trees) {
                last.trees = Some(trees.clone());
                send(Answer::Trees(trees));
            }
        }
        Err(e) => eprintln!("michi: {e}"),
    }
    let trunk = trunk.as_ref().map(|(n, h)| (n.as_str(), *h));
    let sides = clash::sides(wts, &wips, trunk);
    let branches = match trunk.map(|t| clash::branches(dir, wts, t)) {
        Some(Ok(b)) => b,
        Some(Err(e)) => {
            eprintln!("michi: {e}");
            Vec::new()
        }
        None => Vec::new(),
    };
    let clashes = clash::read(dir, sides, trunk.map(|t| t.1), branches, predictions);
    if last.clashes.as_ref() != Some(&clashes) {
        last.clashes = Some(clashes.clone());
        send(Answer::Clashes(clashes));
    }
}

/// Asks tsumugi for its sessions, sending them by worktree when they changed.
fn ask_claude(wts: &[Worktree], last: &mut Last, timeouts: &mut u32, send: &impl Fn(Answer)) {
    let claude = match claude::fetch() {
        Ok(None) => Some(None),
        Ok(Some(sessions)) => {
            let per = claude::assign(wts.iter().map(|w| &w.path), &sessions);
            Some(Some(wts.iter().zip(per).filter(|(_, s)| !s.is_empty()).map(|(w, s)| (w.path.clone(), s)).collect()))
        }
        // Worth a line, not every 2 s: the first and every 30th.
        Err(e) => {
            if !matches!(e, claude::Error::Timeout) || timeouts.is_multiple_of(30) {
                eprintln!("michi: {e}");
            }
            *timeouts += u32::from(matches!(e, claude::Error::Timeout));
            None
        }
    };
    if let Some(c) = claude {
        if last.claude.as_ref() != Some(&c) {
            last.claude = Some(c.clone());
            send(Answer::Claude(c));
        }
    }
}

/// Every ref and every worktree's `HEAD` and branch, in one text; `None`
/// when git could not say.
fn refs_print(dir: &Path, wts: &[Worktree]) -> Option<String> {
    let mut out = git::read_text(dir, &["for-each-ref", "--format=%(objectname) %(refname)"]).ok()?;
    for w in wts {
        out.push_str(&format!("\n{} {} {:?}", w.head, w.path.display(), w.branch));
    }
    Some(out)
}

/// The state file in michi's `folder` (`seen`, `review`) for the
/// repository whose common git folder is `common`.
fn state_path(common: &Path, folder: &str) -> Option<PathBuf> {
    let own = common.file_name()?.to_string_lossy().into_owned();
    // `repo/.git` goes by `repo`; a bare `repo.git` by `repo`.
    let name = if own.eq_ignore_ascii_case(".git") {
        common.parent().and_then(|p| p.file_name()).map_or(own.clone(), |n| n.to_string_lossy().into_owned())
    } else {
        own.strip_suffix(".git").unwrap_or(&own).to_owned()
    };
    Some(ito_common::state_dir("michi")?.join(folder).join(unseen::file_name(&name, common)))
}

fn save_marks(marks: &Marks, file: Option<&Path>) {
    let Some(file) = file else { return };
    if let Err(e) = ito_common::write_atomic(file, marks.to_json()) {
        eprintln!("michi: {e}");
    }
}

/// Reads what the branch review wants, until the window is gone: of the
/// wishes that came while one was read, the newest list and the newest diff
/// (a read stopped and dropped once a newer one comes), and every check.
fn reviews(dir: &Path, wishes: &Receiver<Wish>, latest: &[AtomicU32; 2], send: impl Fn(Answer)) {
    let file = unseen::common_dir(dir).ok().and_then(|c| state_path(&c, "review"));
    let mut checks = file.as_deref().and_then(|f| std::fs::read_to_string(f).ok()).map(|t| Checks::parse(&t)).unwrap_or_default();
    let save = |checks: &Checks| {
        let Some(file) = &file else { return };
        if let Err(e) = ito_common::write_atomic(file, checks.to_json()) {
            eprintln!("michi: {e}");
        }
    };
    // Takes in a wish that came: a check at once, the newest list and diff
    // for their turn.
    let take = |w: Wish, checks: &mut Checks, list: &mut Option<Wish>, one: &mut Option<Wish>| match w {
        Wish::List { .. } => *list = Some(w),
        Wish::File { .. } => *one = Some(w),
        Wish::Check { branch, key, on } => {
            if checks.set(&branch, &key, on) {
                save(checks);
            }
        }
    };
    let (mut list, mut one) = (None, None);
    loop {
        if list.is_none() && one.is_none() {
            match wishes.recv() {
                Ok(w) => take(w, &mut checks, &mut list, &mut one),
                Err(_) => return,
            }
        }
        while let Ok(w) = wishes.try_recv() {
            take(w, &mut checks, &mut list, &mut one);
        }
        if let Some(Wish::List { gen, branch, trunk, head, end, since, prune }) = list.take() {
            let stale = || latest[0].load(Ordering::SeqCst) != gen;
            if !stale() {
                let result = review::list(dir, &branch, &trunk, &head, &end, since.as_ref(), &stale);
                // The checks that came while it was read go with it.
                while let Ok(w) = wishes.try_recv() {
                    take(w, &mut checks, &mut list, &mut one);
                }
                if let Ok(l) = &result {
                    // Only the whole of the branch tells which checks are stale.
                    if prune && l.base.is_some() && !l.changes.cut && !stale() && checks.prune(&branch, &l.changes.files) {
                        save(&checks);
                    }
                }
                if !stale() {
                    send(Answer::ReviewList { gen, result: result.map(Box::new), checks: checks.of(&branch) });
                }
            }
        }
        if let Some(Wish::File { gen, from, to, paths }) = one.take() {
            let stale = || latest[1].load(Ordering::SeqCst) != gen;
            if !stale() {
                let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
                let result = show::changes_in(dir, &from, &to, &paths, true, &stale);
                if !stale() {
                    send(Answer::ReviewFile { gen, result: result.map(Box::new) });
                }
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
    let mut graph_asked = false;
    loop {
        gen += 1;
        // A second wish to read again, or one for more of the last reading,
        // is met by this reading.
        while asks.try_recv().is_ok() {}
        let setup = first.take().unwrap_or_else(|| Setup::read(dir));
        match read(dir, gen, setup, asks, &mut graph_asked, &send) {
            Next::Reload => {}
            Next::Stop => return,
        }
    }
}

enum Next {
    Reload,
    Stop,
}

fn read(
    dir: &Path,
    gen: u32,
    setup: Result<Setup, git::Error>,
    asks: &Receiver<Ask>,
    graph_asked: &mut bool,
    send: &impl Fn(Answer),
) -> Next {
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
        // A history longer than the first batch: once the first screen is
        // up, write the commit-graph if there is none, so the next opening
        // need not walk it all first.
        if !std::mem::replace(graph_asked, true) {
            let dir = dir.to_path_buf();
            let _ = std::thread::Builder::new().name("commit-graph".into()).spawn(move || {
                if log::lacks_graph(&dir) {
                    if let Err(e) = log::write_graph(&dir) {
                        eprintln!("michi: the commit-graph was not written: {e}");
                    }
                }
            });
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
