//! fetch, pull and push: what the window sends the net thread, what it
//! shows while one runs, and what it says when one ends. One runs at a
//! time; Esc stops it. What would lose work waits on a band for `y`: a pull
//! where a Claude is working, a push over commits the remote has.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use michi_core::claude::State;
use michi_core::net::{self, Done, Lease, Progress, Why};
use michi_core::{git, paths};

use crate::app::{App, Repo, View};

/// What the net thread is to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Job {
    /// Every remote.
    Fetch,
    /// The branch of the worktree at `wt`, up to its upstream.
    Pull { wt: PathBuf },
    /// The branch of the worktree at `wt`; over the remote's commits with
    /// `lease`.
    Push { wt: PathBuf, lease: Option<Lease> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Fetch,
    Pull,
    Push,
}

impl Job {
    pub fn kind(&self) -> Kind {
        match self {
            Job::Fetch => Kind::Fetch,
            Job::Pull { .. } => Kind::Pull,
            Job::Push { .. } => Kind::Push,
        }
    }
}

impl Kind {
    /// git's word for it, the same in every language.
    pub fn word(self) -> &'static str {
        match self {
            Kind::Fetch => "fetch",
            Kind::Pull => "pull",
            Kind::Push => "push",
        }
    }
}

/// A job for the net thread, with the flag that stops it.
pub struct Order {
    pub job: Job,
    pub stop: Arc<AtomicBool>,
}

/// The job that runs.
pub struct Running {
    pub kind: Kind,
    /// git's last line of progress.
    pub progress: Option<Progress>,
    stop: Arc<AtomicBool>,
}

impl Running {
    /// Whether Esc has asked it to stop.
    pub fn stopping(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }
}

/// A job waiting for a yes under the title bar: `y` runs it, Esc or `n`
/// drops it.
pub struct Band {
    pub text: String,
    pub job: Job,
}

/// How long ago `at` was, in a few words (`3 min ago`).
pub fn ago(app: &App, at: SystemTime, now: SystemTime) -> String {
    let secs = now.duration_since(at).unwrap_or(Duration::ZERO).as_secs();
    let (key, n) = match secs {
        0..60 => return app.tr("net.now").to_owned(),
        60..3600 => ("net.min", secs / 60),
        3600..86400 => ("net.hour", secs / 3600),
        _ => ("net.day", secs / 86400),
    };
    app.tr(key).replace("{}", &n.to_string())
}

/// The folder's name, to call a worktree by.
fn name(wt: &Path) -> String {
    wt.file_name().map_or_else(|| wt.display().to_string(), |n| n.to_string_lossy().into_owned())
}

impl App {
    /// `f`, `p`, `P`, in every view.
    pub(crate) fn net_key(&mut self, c: char) -> bool {
        if !matches!(c, 'f' | 'p' | 'P') {
            return false;
        }
        if self.net.is_some() {
            self.busy();
            return true;
        }
        match (c, self.net_tree()) {
            ('f', _) => self.order(Job::Fetch),
            ('p', Some(wt)) => self.pull(wt),
            (_, Some(wt)) => self.order(Job::Push { wt, lease: None }),
            (_, None) => self.say(self.tr("net.no_tree")),
        }
        true
    }

    /// The worktree a pull or push is for: the one the `2` view shows, the
    /// worktree table's selected row, else the window's own.
    fn net_tree(&self) -> Option<PathBuf> {
        let picked = match self.view {
            View::Changes => self.changes.wt.clone(),
            View::Worktree => self.wt_selected().filter(|t| !t.wt.bare && t.wt.prunable.is_none()).map(|t| t.wt.path.clone()),
            View::Graph => None,
        };
        picked.or_else(|| match &self.repo {
            Repo::Ready(s) => Some(s.top.clone()),
            _ => None,
        })
    }

    /// `p`: a pull moves the worktree's HEAD and files, so where a Claude
    /// is working it waits for a yes.
    fn pull(&mut self, wt: PathBuf) {
        if self.claude_working(&wt) {
            let text = self.tr("net.claude_pull").replace("{}", &name(&wt));
            self.band = Some(Band { text, job: Job::Pull { wt } });
            return;
        }
        self.order(Job::Pull { wt });
    }

    /// Whether a Claude is at work in the worktree at `wt`.
    fn claude_working(&self, wt: &Path) -> bool {
        let key = paths::key(wt);
        let sessions = self.claude.iter().flatten().find(|(p, _)| paths::key(p) == key);
        sessions.is_some_and(|(_, s)| s.iter().any(|s| s.is_agent() && s.state == State::Running))
    }

    /// Sends `job` to the net thread, unless one runs.
    fn order(&mut self, job: Job) {
        if self.net.is_some() {
            self.busy();
            return;
        }
        let Some(r) = &self.reading else { return };
        let stop = Arc::new(AtomicBool::new(false));
        self.net = Some(Running { kind: job.kind(), progress: None, stop: stop.clone() });
        r.net(Order { job, stop });
    }

    fn busy(&mut self) {
        let Some(n) = &self.net else { return };
        let text = self.tr("net.busy").replace("{}", n.kind.word());
        self.say(text);
    }

    /// A key while the band is up: `y` runs its job, `n` drops it, the rest
    /// do nothing.
    pub(crate) fn band_key(&mut self, c: char) -> bool {
        match c {
            'y' => {
                if let Some(b) = self.band.take() {
                    self.order(b.job);
                }
            }
            'n' => self.band = None,
            _ => {}
        }
        true
    }

    /// Esc with nothing else to close: stop the job that runs.
    pub(crate) fn net_stop(&mut self) -> bool {
        let Some(n) = &self.net else { return false };
        if n.stopping() {
            return false;
        }
        n.stop.store(true, Ordering::Relaxed);
        let text = self.tr("net.stopping").replace("{}", n.kind.word());
        self.say(text);
        true
    }

    /// git's progress for the job that runs.
    pub(crate) fn net_progress(&mut self, p: Progress) {
        if let Some(n) = self.net.as_mut() {
            n.progress = Some(p);
        }
    }

    /// The job is over: say how it went, or put up the band that asks to
    /// push over the remote's commits.
    pub(crate) fn net_done(&mut self, job: Job, result: Result<Done, git::Error>) {
        self.net = None;
        let kind = job.kind();
        let (text, secs) = match result {
            Ok(Done::Fetched { moved: 0, .. }) => (self.tr("net.fetched_none").to_owned(), 4),
            Ok(Done::Fetched { moved, .. }) => (self.tr("net.fetched").replace("{}", &moved.to_string()), 4),
            Ok(Done::Pulled { upstream, count }) => {
                (self.tr("net.pulled").replace("{n}", &count.to_string()).replace("{up}", &upstream), 4)
            }
            Ok(Done::Pushed { to, saved: Some(saved), .. }) => {
                (self.tr("net.pushed_over").replace("{to}", &to).replace("{saved}", michi_core::backup::short(&saved)), 10)
            }
            Ok(Done::Pushed { to, new: true, .. }) => (self.tr("net.pushed_new").replace("{}", &to), 4),
            Ok(Done::Pushed { to, .. }) => (self.tr("net.pushed").replace("{}", &to), 4),
            Ok(Done::Even { to }) => {
                let key = if kind == Kind::Push { "net.even_push" } else { "net.even_pull" };
                (self.tr(key).replace("{}", &to), 4)
            }
            Ok(Done::Stopped) => (self.tr("net.stopped").replace("{}", kind.word()), 4),
            Ok(Done::Not(why)) => match why {
                Why::NoRemote => (self.tr("net.no_remote").to_owned(), 6),
                Why::Detached => (self.tr("net.detached").to_owned(), 6),
                Why::NoUpstream { branch } => (self.tr("net.no_upstream").replace("{}", &branch), 8),
                Why::Diverged { upstream, ahead, behind } => (
                    self.tr("net.diverged")
                        .replace("{up}", &upstream)
                        .replace("{a}", &ahead.to_string())
                        .replace("{b}", &behind.to_string()),
                    10,
                ),
                Why::Behind { lease, count, ahead: 0 } => {
                    (self.tr("net.behind_only").replace("{to}", &lease.to).replace("{n}", &count.to_string()), 8)
                }
                Why::Behind { lease, count, .. } => {
                    let Job::Push { wt, .. } = job else { return };
                    let text = self.tr("net.lease").replace("{to}", &lease.to).replace("{n}", &count.to_string());
                    self.band = Some(Band { text, job: Job::Push { wt, lease: Some(lease) } });
                    return;
                }
                Why::Rejected { to, reason } => {
                    let mut text = self.tr("net.rejected").replace("{to}", &to).replace("{why}", &reason);
                    // The remote has commits this side has not seen (or not since the band asked).
                    if ["fetch first", "non-fast-forward", "stale info"].iter().any(|w| reason.contains(w)) {
                        text.push_str(self.tr("net.rejected_fetch"));
                    }
                    (text, 10)
                }
            },
            Err(e) => {
                let mut text = self.tr("net.failed").replace("{job}", kind.word()).replace("{e}", &e.to_string());
                if net::is_ssh(&e) {
                    text.push_str(self.tr("net.ssh"));
                }
                (text, 10)
            }
        };
        self.say_for(text, secs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::{app, toast};
    use michi_core::oid::Oid;

    fn lease() -> Lease {
        Lease {
            remote: "origin".into(),
            dest: "refs/heads/main".into(),
            to: "origin/main".into(),
            tip: Oid::parse("1111111111111111111111111111111111111111").unwrap(),
        }
    }

    #[test]
    fn the_time_since_the_last_fetch_is_short() {
        let a = app();
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let at = |s: u64| now - Duration::from_secs(s);
        assert_eq!(ago(&a, at(5), now), "just now");
        assert_eq!(ago(&a, at(180), now), "3 min ago");
        assert_eq!(ago(&a, at(7200), now), "2 h ago");
        assert_eq!(ago(&a, at(3 * 86400), now), "3 d ago");
        // A clock that went back: just now.
        assert_eq!(ago(&a, now + Duration::from_secs(9), now), "just now");
    }

    #[test]
    fn one_job_at_a_time_and_esc_stops_it() {
        let mut a = app();
        // No reading thread: nothing is sent, nothing runs.
        a.order(Job::Fetch);
        assert!(a.net.is_none());
        let stop = Arc::new(AtomicBool::new(false));
        a.net = Some(Running { kind: Kind::Fetch, progress: None, stop: stop.clone() });
        assert!(a.net_key('P'));
        assert_eq!(toast(&a), "fetch is running: Esc stops it");
        a.net_progress(Progress { remote: None, step: "Receiving objects".into(), percent: Some(45) });
        assert_eq!(a.net.as_ref().unwrap().progress.as_ref().unwrap().percent, Some(45));
        assert!(a.escape());
        assert!(stop.load(Ordering::Relaxed));
        assert_eq!(toast(&a), "Stopping fetch…");
        // Asked once: the next Esc goes on to nothing.
        assert!(!a.escape());
        a.net_done(Job::Fetch, Ok(Done::Stopped));
        assert!(a.net.is_none());
        assert_eq!(toast(&a), "fetch stopped");
    }

    #[test]
    fn each_ending_is_said() {
        let mut a = app();
        let wt = PathBuf::from("/r/feat");
        let push = Job::Push { wt: wt.clone(), lease: None };
        let pull = Job::Pull { wt: wt.clone() };
        let mut said = |job: &Job, r: Result<Done, git::Error>| {
            a.net_done(job.clone(), r);
            toast(&a).to_owned()
        };
        assert_eq!(said(&Job::Fetch, Ok(Done::Fetched { remotes: 1, moved: 0 })), "Fetched: nothing new");
        assert_eq!(said(&Job::Fetch, Ok(Done::Fetched { remotes: 2, moved: 3 })), "Fetched: 3 refs moved");
        assert_eq!(said(&pull, Ok(Done::Pulled { upstream: "origin/main".into(), count: 2 })), "Pulled 2 commits from origin/main");
        assert_eq!(said(&pull, Ok(Done::Even { to: "origin/main".into() })), "Already up to date with origin/main");
        assert_eq!(said(&push, Ok(Done::Even { to: "origin/feat".into() })), "origin/feat has it already");
        assert_eq!(
            said(&push, Ok(Done::Pushed { to: "origin/feat".into(), new: true, saved: None })),
            "Pushed to origin/feat, now its upstream"
        );
        assert_eq!(said(&push, Ok(Done::Pushed { to: "origin/feat".into(), new: false, saved: None })), "Pushed to origin/feat");
        assert_eq!(
            said(&push, Ok(Done::Pushed { to: "origin/feat".into(), new: false, saved: Some("refs/michi/backup/20261016-142233".into()) })),
            "Pushed over origin/feat; what it had is kept in michi/backup/20261016-142233"
        );
        assert_eq!(said(&pull, Ok(Done::Not(Why::NoUpstream { branch: "feat".into() }))), "feat has no upstream to pull from: P pushes it");
        assert_eq!(
            said(&pull, Ok(Done::Not(Why::Diverged { upstream: "origin/main".into(), ahead: 1, behind: 2 }))),
            "origin/main went its own way (↑1 ↓2): a pull only goes forward, so nothing was merged"
        );
        assert_eq!(
            said(&push, Ok(Done::Not(Why::Behind { lease: lease(), count: 2, ahead: 0 }))),
            "Nothing to push: origin/main is 2 ahead (p pulls)"
        );
        assert_eq!(
            said(&push, Ok(Done::Not(Why::Rejected { to: "origin/main".into(), reason: "[rejected] (fetch first)".into() }))),
            "origin/main refused the push: [rejected] (fetch first). f fetches its commits; then P asks before pushing over them"
        );
        assert_eq!(
            said(&push, Ok(Done::Not(Why::Rejected { to: "origin/main".into(), reason: "pre-receive hook declined".into() }))),
            "origin/main refused the push: pre-receive hook declined"
        );
        assert_eq!(
            said(&push, Err(git::Error::Git("Could not resolve host: example.invalid".into()))),
            "push failed: Could not resolve host: example.invalid"
        );
        let ssh = git::Error::Told {
            line: "git@github.com: Permission denied (publickey).".into(),
            all: "git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.".into(),
        };
        assert!(said(&Job::Fetch, Err(ssh)).ends_with("start ssh-agent and ssh-add your key"));
        assert!(a.band.is_none());
    }

    #[test]
    fn a_push_over_the_remote_s_commits_waits_for_a_yes() {
        let mut a = app();
        let wt = PathBuf::from("/r/feat");
        a.net_done(Job::Push { wt: wt.clone(), lease: None }, Ok(Done::Not(Why::Behind { lease: lease(), count: 2, ahead: 1 })));
        let band = a.band.as_ref().expect("a band");
        assert_eq!(band.job, Job::Push { wt: wt.clone(), lease: Some(lease()) });
        assert!(band.text.contains("origin/main has 2 commits"), "{}", band.text);
        // Other keys wait; n drops it.
        assert!(a.feed('j'));
        assert!(a.band.is_some());
        assert!(a.feed('n'));
        assert!(a.band.is_none());
        // Esc drops it too, before anything else.
        a.band = Some(Band { text: String::new(), job: Job::Fetch });
        assert!(a.escape());
        assert!(a.band.is_none());
        // y sends it (no reading thread here: nothing runs).
        a.band = Some(Band { text: String::new(), job: Job::Fetch });
        assert!(a.feed('y'));
        assert!(a.band.is_none());
    }

    #[test]
    fn a_pull_where_a_claude_works_asks_first() {
        use michi_core::claude::Session;
        let mut a = app();
        a.repo = Repo::Ready(michi_core::summary::Summary {
            name: "r".into(),
            head: michi_core::summary::Head::Detached("1111111".into()),
            worktrees: 1,
            top: PathBuf::from("/r/main"),
        });
        let s = Session {
            id: 1,
            state: State::Running,
            command: "claude".into(),
            agent: "claude".into(),
            cwd: "/r/main".into(),
            branch: String::new(),
            title: String::new(),
            note: String::new(),
            since_ms: 0,
            read_only: false,
        };
        a.claude = Some(vec![(PathBuf::from("/r/main"), vec![s])]);
        assert!(a.feed('p'));
        let band = a.band.as_ref().expect("a band");
        assert_eq!(band.job, Job::Pull { wt: PathBuf::from("/r/main") });
        assert_eq!(band.text, "A Claude is working in main. Pull there anyway?");
        a.band = None;
        a.claude = None;
        // No Claude: it goes (no reading thread here: nothing runs).
        assert!(a.feed('p'));
        assert!(a.band.is_none());
    }
}
