//! The watch over the pull requests of the worktrees' branches. One thread of
//! its own asks `gh pr list` (one call for all of them) every minute while the
//! window is in front and every five behind, and sends what changed since the
//! last reading; the first reading is only the baseline. The UI thread never
//! waits for it. minato, when it runs, lays its checks over the reading.
//! `gh` missing or not signed in stops the thread, silently.
//! What the changes come to on screen (a notice that jumps to the pull
//! request, the system's notification behind) is `App::pr_changes`.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use ito_ipc::door::{self, Door};
use ito_notices::Kind as Notice;
use michi_core::prcreate::{self, Failure};
use michi_core::prwatch::{self, Change, Kind, Prs};

use crate::app::{App, Target};

/// How often the pull requests are read while the window is in front …
const FRONT: Duration = Duration::from_secs(60);
/// … and while it is behind.
const BEHIND: Duration = Duration::from_secs(300);

/// minato's door (minato#67): it knows the open pull requests it already
/// searched, so its checks are laid over what `gh` read. Same shape as
/// `minato_core::door::Rq`; postcard numbers the variants.
#[derive(serde::Serialize, serde::Deserialize)]
enum MinatoRq {
    PrState { repo: String },
}

const MINATO: Door<MinatoRq> = Door::new("minato", "MINATO_ADDRESS", 1);

/// Asks minato about the open pull requests and lays its checks over `prs`;
/// minato not running (or not knowing the repository) leaves `gh`'s reading.
fn overlay_minato(prs: &mut Prs) {
    let Some(repo) = prs.values().find_map(|p| prwatch::repo_of(&p.url)) else { return };
    if let Ok(json) = door::ask(&MINATO, &MinatoRq::PrState { repo }) {
        prwatch::overlay_checks(prs, &json);
    }
}

/// The watch stops after this many readings in a row that failed.
const GIVE_UP: u32 = 2;

fn every(front: bool) -> Duration {
    if front {
        FRONT
    } else {
        BEHIND
    }
}

/// One reading: all the pull requests found, and what changed since the last.
struct Reading {
    prs: Prs,
    changes: Vec<Change>,
}

enum Cmd {
    Branches(Vec<String>),
    Front(bool),
}

pub struct PrWatch {
    cmds: Sender<Cmd>,
    changes: Receiver<Reading>,
    /// The branches the thread was last told of.
    told: Vec<String>,
}

impl PrWatch {
    /// Starts the thread for the repository at `dir`; `None` when it cannot.
    pub fn start(dir: PathBuf, front: bool, ctx: egui::Context) -> Option<Self> {
        let (cmds, cmd_rx) = channel();
        let (change_tx, changes) = channel();
        let spawned = std::thread::Builder::new().name("prwatch".into()).spawn(move || {
            run(&dir, front, &cmd_rx, |c| {
                let sent = change_tx.send(c).is_ok();
                ctx.request_repaint();
                sent
            })
        });
        spawned.ok().map(|_| PrWatch { cmds, changes, told: Vec::new() })
    }

    /// Tells the thread the branches to look at, when they differ from the last.
    pub fn branches(&mut self, mut branches: Vec<String>) {
        branches.sort();
        branches.dedup();
        if branches != self.told {
            self.told = branches.clone();
            let _ = self.cmds.send(Cmd::Branches(branches));
        }
    }

    pub fn front(&self, front: bool) {
        let _ = self.cmds.send(Cmd::Front(front));
    }

    fn take(&self) -> Vec<Reading> {
        self.changes.try_iter().collect()
    }
}

/// The thread's loop: waits for its time (or a command), reads, compares with
/// the last reading and hands it and the changes to `send` (false: nobody listens).
fn run(dir: &std::path::Path, mut front: bool, cmds: &Receiver<Cmd>, send: impl Fn(Reading) -> bool) {
    let mut branches: Vec<String> = Vec::new();
    let mut last: Option<Prs> = None;
    // Readings that failed one after the other.
    let mut misses = 0;
    let mut due = Instant::now();
    loop {
        match cmds.recv_timeout(due.saturating_duration_since(Instant::now())) {
            Ok(Cmd::Branches(b)) => {
                // The first branches ask for a reading now, the baseline.
                if branches.is_empty() && !b.is_empty() {
                    due = Instant::now();
                }
                branches = b;
                continue;
            }
            Ok(Cmd::Front(f)) => {
                // Coming back to the front reads at once when the minute is over.
                if f && !front {
                    due = due.min(Instant::now() + FRONT);
                }
                front = f;
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => return,
            Err(RecvTimeoutError::Timeout) => {}
        }
        due = Instant::now() + every(front);
        if branches.is_empty() {
            continue;
        }
        match prwatch::fetch(dir) {
            Ok(mut now) => {
                misses = 0;
                overlay_minato(&mut now);
                let changes = last.as_ref().map_or_else(Vec::new, |old| prwatch::diff(old, &now, &branches));
                if !send(Reading { prs: now.clone(), changes }) {
                    return;
                }
                last = Some(now);
            }
            // Not there or not signed in: nothing to watch with.
            Err(e) if prcreate::failure_of(&e) != Failure::Other => return,
            // Offline or the like: the next time, but not for ever (a remote
            // that is not GitHub fails every time).
            Err(_) => {
                misses += 1;
                if misses >= GIVE_UP {
                    return;
                }
                continue;
            }
        }
    }
}

impl App {
    /// Starts the watch once the repository is read, tells it the branches of
    /// the worktrees and the window's state, and folds in what it found.
    pub(crate) fn pr_watch_tick(&mut self) {
        if self.pr_watch.is_none() && !self.pr_watch_off {
            let (Some(dir), Some(ctx)) = (self.repo_top(), crate::notify::window()) else { return };
            self.pr_watch_off = true;
            self.pr_watch = PrWatch::start(dir, self.front, ctx);
        }
        let branches: Vec<String> = self.trees.trees.iter().filter(|t| !t.wt.bare).filter_map(|t| t.wt.branch.clone()).collect();
        let Some(w) = &mut self.pr_watch else { return };
        w.branches(branches);
        for r in w.take() {
            self.pr_state = r.prs;
            self.pr_changes(&r.changes);
        }
    }

    /// A notice for each change, pointing at the pull request; the system
    /// tells of it too when the window is behind.
    pub(crate) fn pr_changes(&mut self, changes: &[Change]) {
        for c in changes {
            let (kind, key, event) = match c.kind {
                Kind::Merged => (Notice::Ok, "pr.watch.merged", "done"),
                Kind::Closed => (Notice::Info, "pr.watch.closed", "done"),
                Kind::ChecksFailed => (Notice::Err, "pr.watch.checks_failed", "failed"),
                Kind::ChecksPassed => (Notice::Ok, "pr.watch.checks_passed", "done"),
                Kind::Approved => (Notice::Ok, "pr.watch.approved", "done"),
                Kind::ChangesRequested => (Notice::Warn, "pr.watch.changes_requested", "done"),
            };
            let text = self.tr(key).replace("{n}", &c.number.to_string()).replace("{branch}", &c.branch);
            self.notify(kind, text.clone(), Some(Target::Pr(c.url.clone())), if kind == Notice::Err { 12 } else { 8 });
            let title = self.tr("pr.watch.title").to_owned();
            self.tell(event, &title, &text, Some(self.next_notice));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::app;

    fn change(kind: Kind) -> Change {
        Change { branch: "feat".into(), number: 7, url: "https://github.com/o/r/pull/7".into(), kind }
    }

    #[test]
    fn it_reads_every_minute_in_front_and_every_five_behind() {
        assert_eq!((every(true), every(false)), (Duration::from_secs(60), Duration::from_secs(300)));
        assert_eq!(GIVE_UP, 2, "two failed readings in a row stop the watch");
    }

    #[test]
    fn each_change_is_a_notice_that_goes_to_the_pull_request() {
        let mut a = app();
        a.front = true;
        let all = [Kind::Merged, Kind::Closed, Kind::ChecksFailed, Kind::ChecksPassed, Kind::Approved, Kind::ChangesRequested];
        a.pr_changes(&all.map(change));
        assert_eq!(a.notices.len(), 6);
        let kinds: Vec<_> = a.notices.iter().map(|n| n.kind).collect();
        assert_eq!(kinds, [Notice::Ok, Notice::Info, Notice::Err, Notice::Ok, Notice::Ok, Notice::Warn]);
        for n in &a.notices {
            assert_eq!(n.go, Some(Target::Pr("https://github.com/o/r/pull/7".into())));
            assert!(n.title.contains("#7") && n.title.contains("feat"), "{}", n.title);
        }
    }
}
