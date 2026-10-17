//! Talking to the remotes: fetch, pull (fast-forward only) and push. git's
//! progress is read as it comes, and a flag stops it. What would lose work
//! is not done but told ([`Why`]): a pull of a branch that went its own way,
//! a push over commits the remote has; the push over them happens on a
//! second call with a [`Lease`], after the remote's tip is kept under
//! `refs/michi/backup/`.

use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant, SystemTime};

use crate::autostash::{self, Stashed};
use crate::backup;
use crate::git;
use crate::oid::Oid;

/// A line of git's progress: `Receiving objects:  45% (450/1000)`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    /// The remote, when a fetch goes to more than one.
    pub remote: Option<String>,
    /// What git is doing (`Receiving objects`), in git's language.
    pub step: String,
    pub percent: Option<u8>,
}

impl Progress {
    /// The progress in a line of git's standard error, `remote: ` or not;
    /// `None` for anything else (`From …`, `error: …`, the refs it moved).
    pub fn parse(line: &str) -> Option<Progress> {
        let line = line.trim();
        let line = line.strip_prefix("remote:").map(str::trim_start).unwrap_or(line);
        let (step, rest) = line.split_once(": ")?;
        let step = step.trim();
        if step.is_empty() || ["error", "fatal", "hint", "warning"].contains(&step.to_ascii_lowercase().as_str()) {
            return None;
        }
        let rest = rest.trim_start();
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        let percent = if rest[digits..].starts_with('%') { rest[..digits].parse::<u8>().ok().filter(|p| *p <= 100) } else { None };
        Some(Progress { remote: None, step: step.to_owned(), percent })
    }
}

/// How a fetch, pull or push ended, when git did not refuse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Done {
    /// Every remote fetched; `moved` refs came, went or moved.
    Fetched { remotes: usize, moved: usize },
    /// The branch went forward `count` commits to `upstream` (`origin/main`);
    /// `stashed`, the local changes that were put away for it and back.
    Pulled { upstream: String, count: usize, stashed: Option<Stashed> },
    /// The branch went to `to` (`origin/main`): `new` there, with the
    /// upstream set; `saved`, the backup of what a push over it replaced.
    Pushed { to: String, new: bool, saved: Option<String> },
    /// The tag `tag` went to `remote`.
    PushedTag { remote: String, tag: String },
    /// Nothing to pull from or push to `to`: the same commit.
    Even { to: String },
    /// A push that has not been made: what it would send and where
    /// ([`preview`]).
    Preview(Preview),
    /// Sync went through: the branch is level with `to` (`origin/main`).
    /// `pulled` commits came in (`stashed`: the local changes put away for
    /// it and back); `pushed` when commits went out.
    Synced { to: String, pulled: usize, pushed: bool, new: bool, stashed: Option<Stashed> },
    /// Sync stopped at `at`, for `why`; what it did before that (`pulled`,
    /// `stashed`) stays.
    SyncHalted { at: Step, why: Why, pulled: usize, stashed: Option<Stashed> },
    /// Stopped on the flag; what git had done stays.
    Stopped,
    /// Not done, and why.
    Not(Why),
}

/// The half of a sync that stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Pull,
    Push,
}

/// What a push of the branch would send, shown before it is made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preview {
    /// Where it goes: `origin/feat`.
    pub to: String,
    /// The branch is not there yet (the push creates it and sets the upstream).
    pub new: bool,
    /// The newest commits to send, at most [`PREVIEW_MAX`], newest first.
    pub commits: Vec<Sent>,
    /// How many commits it sends in all.
    pub total: usize,
    /// How many commits the remote's branch has that this one does not (as of
    /// the last fetch): a push goes over them only with `lease`.
    pub behind: usize,
    /// The push over the remote's branch, when it has commits this one does not.
    pub lease: Option<Lease>,
}

/// A commit a push would send.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sent {
    pub id: Oid,
    pub subject: String,
}

/// The most commits a [`Preview`] lists.
pub const PREVIEW_MAX: usize = 12;

/// Why a pull or push was not done.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Why {
    NoRemote,
    /// Several remotes and none of them is the one pushes go to: the caller
    /// asks which, and pushes again with it.
    Choose {
        remotes: Vec<String>,
    },
    /// HEAD is on no branch.
    Detached,
    /// The branch has no upstream to pull from.
    NoUpstream {
        branch: String,
    },
    /// Commits on both sides: a fast-forward cannot take `upstream`.
    Diverged {
        upstream: String,
        ahead: usize,
        behind: usize,
    },
    /// The remote's branch has `count` commits this one does not (as of the
    /// last fetch); `ahead` the other way. A push with the lease replaces them.
    Behind {
        lease: Lease,
        count: usize,
        ahead: usize,
    },
    /// The remote said no (`[rejected] (fetch first)`, a hook).
    Rejected {
        to: String,
        reason: String,
    },
}

/// A push over the remote's branch, as long as it still is at `tip`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lease {
    pub remote: String,
    /// The branch there: `refs/heads/main`.
    pub dest: String,
    /// What to call it: `origin/main`.
    pub to: String,
    pub tip: Oid,
}

/// `git fetch --prune` of each remote of the repository at `dir`.
pub fn fetch(dir: &Path, stop: &AtomicBool, tell: &mut dyn FnMut(Progress)) -> Result<Done, git::Error> {
    let remotes = remotes(dir)?;
    if remotes.is_empty() {
        return Ok(Done::Not(Why::NoRemote));
    }
    let many = remotes.len() > 1;
    let (mut moved, mut failed) = (0, None);
    // One at a time rather than `--all`: stopping kills the fetch that runs.
    for r in &remotes {
        let mut c = git::command();
        c.arg("-C").arg(dir).args(["fetch", "--prune", "--progress", r]);
        let said = run(c, stop, &mut |mut p| {
            if many {
                p.remote = Some(r.clone());
            }
            tell(p)
        })?;
        let Some(said) = said else { return Ok(Done::Stopped) };
        if said.ok {
            // ` * [new branch]  main -> origin/main`, `   1a2b..3c4d  …`, ` - [deleted] …`
            moved += said.lines.iter().filter(|l| l.contains(" -> ")).count();
        } else if failed.is_none() {
            failed = Some(said.refused());
        }
    }
    match failed {
        Some(e) => Err(e),
        None => Ok(Done::Fetched { remotes: remotes.len(), moved }),
    }
}

/// Brings the branch of the worktree at `dir` up to its upstream: fetches
/// that remote, then fast-forwards; a branch with commits of its own is not
/// merged ([`Why::Diverged`]). Uncommitted changes that stand in the way are
/// stashed around the fast-forward ([`autostash`]).
pub fn pull(dir: &Path, stop: &AtomicBool, tell: &mut dyn FnMut(Progress)) -> Result<Done, git::Error> {
    let Some(branch) = branch(dir)? else { return Ok(Done::Not(Why::Detached)) };
    let remote = config(dir, &format!("branch.{branch}.remote"))?;
    let merge = config(dir, &format!("branch.{branch}.merge"))?;
    let (Some(remote), Some(_)) = (remote, merge) else { return Ok(Done::Not(Why::NoUpstream { branch })) };
    if remote != "." {
        let mut c = git::command();
        c.arg("-C").arg(dir).args(["fetch", "--prune", "--progress", &remote]);
        match run(c, stop, tell)? {
            None => return Ok(Done::Stopped),
            Some(said) if !said.ok => return Err(said.refused()),
            Some(_) => {}
        }
    }
    let up = git::read_text(dir, &["rev-parse", "--symbolic-full-name", &format!("{branch}@{{upstream}}")])?;
    let upstream = short_ref(&up).to_owned();
    if stop.load(Ordering::Relaxed) {
        return Ok(Done::Stopped);
    }
    let (ahead, behind) = counts(dir, &format!("HEAD...{up}"))?;
    if behind == 0 {
        return Ok(Done::Even { to: upstream });
    }
    if ahead > 0 {
        return Ok(Done::Not(Why::Diverged { upstream, ahead, behind }));
    }
    // Local changes in the way are put away, kept, and put back after.
    let ((), stashed) = autostash::around(dir, &format!("pull of {upstream}"), || {
        let out = git::command().arg("-C").arg(dir).args(["merge", "--ff-only", "-q", &up]).output().map_err(git::started)?;
        if out.status.success() {
            Ok(())
        } else {
            Err(git::refused_all(&String::from_utf8_lossy(&out.stderr)))
        }
    })?;
    Ok(Done::Pulled { upstream, count: behind, stashed })
}

/// Sync: fetch the branch's remote, fast-forward to its upstream, then push
/// what the branch has over it. Each half is the plain [`pull`] and
/// [`push`], so nothing is done that those would not: a branch that went its
/// own way ([`Why::Diverged`]) or a push the remote refuses stops the sync
/// where it is ([`Done::SyncHalted`]) and the half before it stays. A branch
/// with no upstream skips the pull and is pushed (its upstream set). Never
/// pushes over the remote's commits: that stays `F`'s, with its band.
pub fn sync(dir: &Path, chosen: Option<&str>, stop: &AtomicBool, tell: &mut dyn FnMut(Progress)) -> Result<Done, git::Error> {
    let (mut pulled, mut stashed) = (0, None);
    let mut to = None;
    match pull(dir, stop, tell)? {
        Done::Pulled { upstream, count, stashed: s } => {
            (pulled, stashed, to) = (count, s, Some(upstream));
        }
        Done::Even { to: up } => to = Some(up),
        Done::Stopped => return Ok(Done::Stopped),
        // Nothing to pull from yet: the push makes the upstream.
        Done::Not(Why::NoUpstream { .. }) => {}
        Done::Not(why) => return Ok(Done::SyncHalted { at: Step::Pull, why, pulled, stashed }),
        other => return Err(git::Error::Io(format!("pull said {other:?}"))),
    }
    if stop.load(Ordering::Relaxed) {
        return Ok(Done::Stopped);
    }
    Ok(match push(dir, chosen, None, stop, tell)? {
        Done::Pushed { to, new, .. } => Done::Synced { to, pulled, pushed: true, new, stashed },
        Done::Even { to: up } => Done::Synced { to: to.unwrap_or(up), pulled, pushed: false, new: false, stashed },
        Done::Stopped => Done::Stopped,
        Done::Not(why) => Done::SyncHalted { at: Step::Push, why, pulled, stashed },
        other => return Err(git::Error::Io(format!("push said {other:?}"))),
    })
}

/// Pushes the branch of the worktree at `dir`: to its push remote
/// (`branch.<b>.pushRemote`, `remote.pushDefault`, its upstream's, `origin`,
/// the only one), or `chosen` when given, setting the upstream when it has
/// none. With several remotes and no way to tell which: [`Why::Choose`]. When the remote's
/// branch has commits this one does not, it is not pushed ([`Why::Behind`])
/// unless `lease` says to: then the remote's tip is kept first and the push
/// goes over it with `--force-with-lease`.
pub fn push(
    dir: &Path,
    chosen: Option<&str>,
    lease: Option<&Lease>,
    stop: &AtomicBool,
    tell: &mut dyn FnMut(Progress),
) -> Result<Done, git::Error> {
    let t = match target(dir, chosen)? {
        Ok(t) => t,
        Err(done) => return Ok(done),
    };
    let Target { branch, remote, dest, new, to } = t;
    let name = dest.strip_prefix("refs/heads/").unwrap_or(&dest).to_owned();
    if lease.is_none() {
        let tracking = format!("refs/remotes/{remote}/{name}");
        if let Some(tip) = git::read_maybe(dir, &["rev-parse", "--verify", "-q", &tracking])? {
            let tip = Oid::parse(String::from_utf8_lossy(&tip).trim()).ok_or_else(|| git::Error::Io(format!("{tracking}: no commit")))?;
            let (ahead, count) = counts(dir, &format!("HEAD...{tracking}"))?;
            if count > 0 {
                return Ok(Done::Not(Why::Behind { lease: Lease { remote, dest, to, tip }, count, ahead }));
            }
        }
    }
    let saved = match lease {
        Some(l) => Some(backup::save(dir, &l.tip, &format!("push --force-with-lease over {}", l.to))?),
        None => None,
    };
    let mut c = git::command();
    c.arg("-C").arg(dir).args(["push", "--porcelain", "--progress"]);
    if new {
        c.arg("--set-upstream");
    }
    if let Some(l) = lease {
        c.arg(format!("--force-with-lease={}:{}", l.dest, l.tip));
    }
    c.arg(&remote).arg(format!("refs/heads/{branch}:{dest}"));
    let Some(said) = run(c, stop, tell)? else { return Ok(Done::Stopped) };
    Ok(match pushed(&said, &to)? {
        Some(done) => done,
        None => Done::Pushed { to, new, saved },
    })
}

/// Where a push of the branch of the worktree at `dir` goes.
struct Target {
    branch: String,
    remote: String,
    /// The branch there: `refs/heads/main`.
    dest: String,
    /// The branch has no upstream yet.
    new: bool,
    /// What to call it: `origin/main`.
    to: String,
}

/// The remote and the branch a push goes to, or why it goes nowhere.
fn target(dir: &Path, chosen: Option<&str>) -> Result<Result<Target, Done>, git::Error> {
    let Some(branch) = branch(dir)? else { return Ok(Err(Done::Not(Why::Detached))) };
    let up_remote = config(dir, &format!("branch.{branch}.remote"))?;
    let up_merge = config(dir, &format!("branch.{branch}.merge"))?;
    let remote = match chosen.map(str::to_owned).map_or_else(|| config(dir, &format!("branch.{branch}.pushRemote")), |r| Ok(Some(r)))? {
        Some(r) => Some(r),
        None => match config(dir, "remote.pushDefault")? {
            Some(r) => Some(r),
            None => match up_remote.clone().filter(|r| r != ".") {
                Some(r) => Some(r),
                None => {
                    let all = remotes(dir)?;
                    if all.iter().any(|r| r == "origin") {
                        Some("origin".to_owned())
                    } else if all.len() == 1 {
                        all.into_iter().next()
                    } else {
                        None
                    }
                }
            },
        },
    };
    let Some(remote) = remote else {
        let remotes = remotes(dir)?;
        return Ok(Err(Done::Not(if remotes.is_empty() { Why::NoRemote } else { Why::Choose { remotes } })));
    };
    let dest = match (&up_remote, &up_merge) {
        (Some(r), Some(m)) if *r == remote => m.clone(),
        _ => format!("refs/heads/{branch}"),
    };
    let new = up_remote.is_none() || up_merge.is_none();
    let name = dest.strip_prefix("refs/heads/").unwrap_or(&dest);
    let to = format!("{remote}/{name}");
    Ok(Ok(Target { branch, remote, dest, new, to }))
}

/// What a push of the branch of the worktree at `dir` would send, without
/// pushing: [`Done::Preview`] when there is something to say, else what
/// [`push`] would say (it has it already, the remote is ahead, no remote to
/// choose). Reads only what the last fetch left, so it needs no network.
pub fn preview(dir: &Path, chosen: Option<&str>) -> Result<Done, git::Error> {
    let t = match target(dir, chosen)? {
        Ok(t) => t,
        Err(done) => return Ok(done),
    };
    let Target { remote, dest, new, to, .. } = t;
    let name = dest.strip_prefix("refs/heads/").unwrap_or(&dest);
    let tracking = format!("refs/remotes/{remote}/{name}");
    let tip = git::read_maybe(dir, &["rev-parse", "--verify", "-q", &tracking])?;
    let (range, behind, lease) = match tip {
        Some(tip) => {
            let tip = Oid::parse(String::from_utf8_lossy(&tip).trim()).ok_or_else(|| git::Error::Io(format!("{tracking}: no commit")))?;
            let (ahead, behind) = counts(dir, &format!("HEAD...{tracking}"))?;
            if behind > 0 && ahead == 0 {
                return Ok(Done::Not(Why::Behind { lease: Lease { remote, dest, to, tip }, count: behind, ahead }));
            }
            if ahead == 0 {
                return Ok(Done::Even { to });
            }
            let lease = (behind > 0).then(|| Lease { remote: remote.clone(), dest: dest.clone(), to: to.clone(), tip });
            (vec![format!("{tracking}..HEAD")], behind, lease)
        }
        // Not there yet: what no ref of the remote has.
        None => (vec!["HEAD".to_owned(), "--not".to_owned(), format!("--remotes={remote}")], 0, None),
    };
    let mut log = vec!["log".to_owned(), format!("--max-count={PREVIEW_MAX}"), "--format=%H%x1f%s".to_owned()];
    log.extend(range.iter().cloned());
    let log: Vec<&str> = log.iter().map(String::as_str).collect();
    let commits = git::read_text(dir, &log)?
        .lines()
        .filter_map(|l| {
            let (id, subject) = l.split_once('')?;
            Some(Sent { id: Oid::parse(id)?, subject: subject.trim().to_owned() })
        })
        .collect();
    let mut count = vec!["rev-list", "--count"];
    count.extend(range.iter().map(String::as_str));
    let total = git::read_text(dir, &count)?.trim().parse().unwrap_or(0);
    Ok(Done::Preview(Preview { to, new, commits, total, behind, lease }))
}

/// `git push` of the tag `tag` to the remote pushes go to
/// (`remote.pushDefault`, `origin`, the only one). One the remote has
/// already on another commit is [`Why::Rejected`]: michi does not push
/// over a tag.
pub fn push_tag(dir: &Path, tag: &str, stop: &AtomicBool, tell: &mut dyn FnMut(Progress)) -> Result<Done, git::Error> {
    let remote = match config(dir, "remote.pushDefault")? {
        Some(r) => Some(r),
        None => {
            let all = remotes(dir)?;
            if all.iter().any(|r| r == "origin") {
                Some("origin".to_owned())
            } else if all.len() == 1 {
                all.into_iter().next()
            } else {
                None
            }
        }
    };
    let Some(remote) = remote else { return Ok(Done::Not(Why::NoRemote)) };
    let full = format!("refs/tags/{tag}");
    let mut c = git::command();
    c.arg("-C").arg(dir).args(["push", "--porcelain", "--progress"]).arg(&remote).arg(format!("{full}:{full}"));
    let Some(said) = run(c, stop, tell)? else { return Ok(Done::Stopped) };
    Ok(pushed(&said, &remote)?.unwrap_or(Done::PushedTag { remote, tag: tag.to_owned() }))
}

/// What `push --porcelain` said when it did not push: a ref it rejected,
/// or every ref there already. `None` when it pushed.
fn pushed(said: &Said, to: &str) -> Result<Option<Done>, git::Error> {
    // `<flag>\t<from>:<to>\t<summary>` for each ref, between `To …` and `Done`.
    let refs: Vec<(char, &str)> = said
        .out
        .lines()
        .filter_map(|l| {
            let mut parts = l.splitn(3, '\t');
            let flag = parts.next()?.chars().next()?;
            parts.next()?;
            Some((flag, parts.next().unwrap_or("").trim()))
        })
        .collect();
    if let Some((_, reason)) = refs.iter().find(|(f, _)| *f == '!') {
        return Ok(Some(Done::Not(Why::Rejected { to: to.to_owned(), reason: (*reason).to_owned() })));
    }
    if !said.ok {
        return Err(said.refused());
    }
    if !refs.is_empty() && refs.iter().all(|(f, _)| *f == '=') {
        return Ok(Some(Done::Even { to: to.to_owned() }));
    }
    Ok(None)
}

/// When the repository whose common folder (`.git`) is `common` last
/// fetched: the newest `FETCH_HEAD`, the worktrees' among them.
pub fn last_fetch(common: &Path) -> Option<SystemTime> {
    let at = |p: &Path| fs::metadata(p.join("FETCH_HEAD")).and_then(|m| m.modified()).ok();
    let mut newest = at(common);
    if let Ok(dir) = fs::read_dir(common.join("worktrees")) {
        for e in dir.flatten() {
            newest = newest.max(at(&e.path()));
        }
    }
    newest
}

/// Whether `e` is ssh not getting in (no key loaded, a passphrase it could
/// not ask for): worth saying to start the ssh agent.
pub fn is_ssh(e: &git::Error) -> bool {
    let all = e.all();
    ["Permission denied (publickey", "Host key verification failed", "passphrase", "authentication agent"].iter().any(|s| all.contains(s))
}

/// What a remote's short name for `full` is: `origin/main` for
/// `refs/remotes/origin/main`, `main` for `refs/heads/main`.
fn short_ref(full: &str) -> &str {
    full.strip_prefix("refs/remotes/").or_else(|| full.strip_prefix("refs/heads/")).unwrap_or(full)
}

fn remotes(dir: &Path) -> Result<Vec<String>, git::Error> {
    Ok(git::read_text(dir, &["remote"])?.lines().map(str::trim).filter(|l| !l.is_empty()).map(str::to_owned).collect())
}

/// The branch HEAD is on, `None` when detached.
fn branch(dir: &Path) -> Result<Option<String>, git::Error> {
    Ok(git::read_maybe(dir, &["symbolic-ref", "-q", "--short", "HEAD"])?.map(|b| String::from_utf8_lossy(&b).trim().to_owned()))
}

fn config(dir: &Path, key: &str) -> Result<Option<String>, git::Error> {
    Ok(git::read_maybe(dir, &["config", "--get", key])?.map(|v| String::from_utf8_lossy(&v).trim().to_owned()))
}

/// The commits only on the left of `a...b`, and only on the right.
fn counts(dir: &Path, range: &str) -> Result<(usize, usize), git::Error> {
    let text = git::read_text(dir, &["rev-list", "--left-right", "--count", range])?;
    let mut n = text.split_whitespace().map(|n| n.parse::<usize>().unwrap_or(0));
    Ok((n.next().unwrap_or(0), n.next().unwrap_or(0)))
}

/// What git said.
struct Said {
    ok: bool,
    out: String,
    /// The lines of its standard error that were not progress.
    lines: Vec<String>,
}

impl Said {
    fn refused(&self) -> git::Error {
        git::refused_all(&self.lines.join("\n"))
    }
}

enum Piece {
    Line(String),
    ErrEnd,
    Out(Vec<u8>),
    OutEnd,
}

/// Runs `c`, telling its progress as it comes; `None` when `stop` stopped it.
/// Stopping git stops what it started too: on Windows the `git.exe` on the
/// PATH is mostly Git for Windows' launcher, and the git doing the work,
/// with its `git-remote-https` or ssh, outlives the launcher killed alone.
#[cfg(windows)]
fn kill_tree(child: &mut Child) {
    let mut c = Command::new("taskkill");
    c.args(["/T", "/F", "/PID", &child.id().to_string()]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    git::hide_window(&mut c);
    let _ = c.status();
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(windows)]
fn own_group(_: &mut Command) {}

/// The group [`own_group`] gave git, all of it.
#[cfg(unix)]
fn kill_tree(child: &mut Child) {
    let group = format!("-{}", child.id());
    let _ = Command::new("kill").args(["-TERM", "--", &group]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
    let _ = child.kill();
    let _ = child.wait();
}

/// git and what it starts in a process group of their own, to stop together.
#[cfg(unix)]
fn own_group(c: &mut Command) {
    use std::os::unix::process::CommandExt;
    c.process_group(0);
}

#[cfg(not(any(windows, unix)))]
fn kill_tree(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(not(any(windows, unix)))]
fn own_group(_: &mut Command) {}

fn run(mut c: Command, stop: &AtomicBool, tell: &mut dyn FnMut(Progress)) -> Result<Option<Said>, git::Error> {
    c.stdout(Stdio::piped()).stderr(Stdio::piped());
    own_group(&mut c);
    let mut child = c.spawn().map_err(git::started)?;
    let (tx, rx) = mpsc::channel();
    // The readers outlive a stopped git when a helper it started (ssh, a
    // credential helper) still holds the pipes; they end with it.
    let mut stderr = child.stderr.take().expect("piped");
    let err_tx = tx.clone();
    let err = std::thread::Builder::new().name("git-net-stderr".into()).spawn(move || {
        let (mut buf, mut line) = ([0u8; 4096], Vec::new());
        let send = |line: &mut Vec<u8>| {
            if !line.is_empty() {
                let _ = err_tx.send(Piece::Line(String::from_utf8_lossy(line).into_owned()));
                line.clear();
            }
        };
        // Progress ends its lines with `\r`, to write over them.
        while let Ok(n @ 1..) = stderr.read(&mut buf) {
            for &b in &buf[..n] {
                if b == b'\r' || b == b'\n' {
                    send(&mut line);
                } else {
                    line.push(b);
                }
            }
        }
        send(&mut line);
        let _ = err_tx.send(Piece::ErrEnd);
    });
    let mut stdout = child.stdout.take().expect("piped");
    let out_tx = tx.clone();
    let out = std::thread::Builder::new().name("git-net-stdout".into()).spawn(move || {
        let mut buf = [0u8; 4096];
        while let Ok(n @ 1..) = stdout.read(&mut buf) {
            let _ = out_tx.send(Piece::Out(buf[..n].to_vec()));
        }
        let _ = out_tx.send(Piece::OutEnd);
    });
    if let Err(e) = err.and(out) {
        kill_tree(&mut child);
        return Err(git::Error::Io(format!("git: {e}")));
    }
    let (mut out, mut lines) = (Vec::new(), Vec::new());
    let (mut err_end, mut out_end) = (false, false);
    let mut exited: Option<(ExitStatus, Instant)> = None;
    loop {
        if stop.load(Ordering::Relaxed) {
            kill_tree(&mut child);
            return Ok(None);
        }
        // `tx` is still here, so this waits rather than seeing the readers gone.
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Piece::Line(l)) => match Progress::parse(&l) {
                Some(p) => tell(p),
                None => lines.push(l),
            },
            Ok(Piece::ErrEnd) => err_end = true,
            Ok(Piece::Out(b)) => out.extend(b),
            Ok(Piece::OutEnd) => out_end = true,
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {}
        }
        if exited.is_none() {
            match child.try_wait() {
                Ok(Some(status)) => exited = Some((status, Instant::now())),
                Ok(None) => {}
                Err(e) => return Err(git::Error::Io(format!("git: {e}"))),
            }
        }
        // A helper can hold the pipes after git is gone: a second for them.
        let Some((status, at)) = exited else { continue };
        if (err_end && out_end) || at.elapsed() > Duration::from_secs(1) {
            while let Ok(p) = rx.try_recv() {
                match p {
                    Piece::Line(l) if Progress::parse(&l).is_none() => lines.push(l),
                    Piece::Out(b) => out.extend(b),
                    _ => {}
                }
            }
            return Ok(Some(Said { ok: status.success(), out: String::from_utf8_lossy(&out).into_owned(), lines }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;
    use std::path::PathBuf;

    fn progress(step: &str, percent: Option<u8>) -> Option<Progress> {
        Some(Progress { remote: None, step: step.to_owned(), percent })
    }

    #[test]
    fn progress_is_read_from_git_s_lines() {
        assert_eq!(Progress::parse("Receiving objects:  45% (450/1000), 1.2 MiB | 1.0 MiB/s"), progress("Receiving objects", Some(45)));
        assert_eq!(Progress::parse("remote: Counting objects: 100% (5/5), done."), progress("Counting objects", Some(100)));
        assert_eq!(Progress::parse("remote: Enumerating objects: 5, done."), progress("Enumerating objects", None));
        assert_eq!(Progress::parse("オブジェクトを受信中:  45% (450/1000)"), progress("オブジェクトを受信中", Some(45)));
        for not in [
            "From ../remote",
            " * [new branch]      main       -> origin/main",
            "error: failed to push some refs to '../remote.git'",
            "fatal: 12: odd",
            "hint: Updates were rejected",
            "ssh: connect to host github.com port 22: Connection refused",
            "Total 3 (delta 0), reused 0 (delta 0)",
            "",
        ] {
            assert_eq!(Progress::parse(not), None, "{not}");
        }
        assert_eq!(Progress::parse("Writing objects: 300% (1/1)"), progress("Writing objects", None));
    }

    /// The repository with a bare remote `origin` (its `main` pushed and
    /// tracked) and someone else's clone of it, `other`.
    fn with_remote(name: &str) -> (TempRepo, PathBuf) {
        let t = TempRepo::configured(name);
        let remote = t.root.join("remote.git");
        t.git(&t.root, &["init", "-q", "--bare", "-b", "main", &remote.to_string_lossy()]);
        t.git(&t.repo, &["remote", "add", "origin", &remote.to_string_lossy()]);
        t.git(&t.repo, &["push", "-q", "-u", "origin", "main"]);
        t.git(&t.root, &["clone", "-q", &remote.to_string_lossy(), "other"]);
        let other = t.root.join("other");
        (t, other)
    }

    fn commit(t: &TempRepo, dir: &Path, file: &str) -> String {
        t.write(dir, file, file);
        t.git(dir, &["add", "."]);
        t.git(dir, &["commit", "-q", "-m", file]);
        t.out(dir, &["rev-parse", "HEAD"])
    }

    fn quiet() -> impl FnMut(Progress) {
        |_| {}
    }

    #[test]
    fn fetch_counts_what_moved() {
        let t = TempRepo::configured("net-fetch-none");
        let no = AtomicBool::new(false);
        assert_eq!(fetch(&t.repo, &no, &mut quiet()).unwrap(), Done::Not(Why::NoRemote));
        assert_eq!(last_fetch(&t.repo.join(".git")), None);
        drop(t);

        let (t, other) = with_remote("net-fetch");
        commit(&t, &other, "a.txt");
        t.git(&other, &["push", "-q", "origin", "HEAD:main", "HEAD:side"]);
        assert_eq!(fetch(&t.repo, &no, &mut quiet()).unwrap(), Done::Fetched { remotes: 1, moved: 2 });
        assert_eq!(fetch(&t.repo, &no, &mut quiet()).unwrap(), Done::Fetched { remotes: 1, moved: 0 });
        assert!(last_fetch(&t.repo.join(".git")).is_some());
        t.git(&other, &["push", "-q", "origin", ":side"]);
        assert_eq!(fetch(&t.repo, &no, &mut quiet()).unwrap(), Done::Fetched { remotes: 1, moved: 1 }, "pruned");

        let stopped = AtomicBool::new(true);
        assert_eq!(fetch(&t.repo, &stopped, &mut quiet()).unwrap(), Done::Stopped);

        t.git(&t.repo, &["remote", "add", "gone", &t.root.join("nowhere.git").to_string_lossy()]);
        assert!(fetch(&t.repo, &no, &mut quiet()).is_err());
    }

    #[test]
    fn a_tag_goes_to_the_remote_but_not_over_its_own() {
        let t = TempRepo::configured("net-tag-none");
        let no = AtomicBool::new(false);
        t.git(&t.repo, &["tag", "v1"]);
        assert_eq!(push_tag(&t.repo, "v1", &no, &mut quiet()).unwrap(), Done::Not(Why::NoRemote));
        drop(t);

        let (t, other) = with_remote("net-tag");
        t.git(&t.repo, &["tag", "-a", "-m", "One", "v1"]);
        assert_eq!(push_tag(&t.repo, "v1", &no, &mut quiet()).unwrap(), Done::PushedTag { remote: "origin".into(), tag: "v1".into() });
        assert_eq!(
            t.out(&other, &["ls-remote", "--tags", "origin", "v1"]).split('\t').next(),
            Some(&*t.out(&t.repo, &["rev-parse", "v1"]))
        );
        assert_eq!(push_tag(&t.repo, "v1", &no, &mut quiet()).unwrap(), Done::Even { to: "origin".into() });
        commit(&t, &other, "a.txt");
        t.git(&other, &["tag", "v2"]);
        t.git(&other, &["push", "-q", "origin", "v2"]);
        t.git(&t.repo, &["tag", "v2"]);
        match push_tag(&t.repo, "v2", &no, &mut quiet()).unwrap() {
            Done::Not(Why::Rejected { to, .. }) => assert_eq!(to, "origin"),
            d => panic!("{d:?}"),
        }
        assert!(push_tag(&t.repo, "nope", &no, &mut quiet()).is_err());
    }

    #[test]
    fn a_preview_lists_what_a_push_would_send_and_over_what() {
        let (t, other) = with_remote("net-preview");
        // Nothing new: the remote has it.
        assert_eq!(preview(&t.repo, None).unwrap(), Done::Even { to: "origin/main".into() });
        let a = commit(&t, &t.repo, "a.txt");
        let b = commit(&t, &t.repo, "b.txt");
        let Done::Preview(p) = preview(&t.repo, None).unwrap() else { panic!("a preview") };
        assert_eq!((p.to.as_str(), p.new, p.total, p.behind, p.lease.is_none()), ("origin/main", false, 2, 0, true));
        // Newest first, with their subjects.
        assert_eq!(p.commits.iter().map(|c| (c.id.to_string(), c.subject.as_str())).collect::<Vec<_>>(), [(b, "b.txt"), (a, "a.txt")]);
        // Nothing was pushed.
        assert_eq!(t.out(&t.repo, &["rev-list", "--count", "origin/main..HEAD"]), "2");

        // A branch the remote lacks: what no ref of the remote has, and it is new there.
        t.git(&t.repo, &["checkout", "-q", "-b", "feat"]);
        commit(&t, &t.repo, "c.txt");
        let Done::Preview(p) = preview(&t.repo, None).unwrap() else { panic!("a preview") };
        assert_eq!((p.to.as_str(), p.new, p.total), ("origin/feat", true, 3));
        // Many commits: the newest few are listed, the count is all.
        for i in 0..PREVIEW_MAX {
            commit(&t, &t.repo, &format!("m{i}.txt"));
        }
        let Done::Preview(p) = preview(&t.repo, None).unwrap() else { panic!("a preview") };
        assert_eq!((p.commits.len(), p.total), (PREVIEW_MAX, 3 + PREVIEW_MAX));

        // The remote is ahead only: nothing to send.
        t.git(&t.repo, &["checkout", "-q", "main"]);
        t.git(&t.repo, &["reset", "-q", "--hard", "origin/main"]);
        commit(&t, &other, "d.txt");
        t.git(&other, &["push", "-q"]);
        t.git(&t.repo, &["fetch", "-q"]);
        assert!(matches!(preview(&t.repo, None).unwrap(), Done::Not(Why::Behind { count: 1, ahead: 0, .. })));
        // Both sides have commits: the preview carries the lease.
        commit(&t, &t.repo, "e.txt");
        let Done::Preview(p) = preview(&t.repo, None).unwrap() else { panic!("a preview") };
        assert_eq!((p.total, p.behind), (1, 1));
        let lease = p.lease.expect("a lease");
        assert_eq!((lease.to.as_str(), lease.dest.as_str()), ("origin/main", "refs/heads/main"));
        assert_eq!(lease.tip.to_string(), t.out(&t.repo, &["rev-parse", "origin/main"]));

        // No remote: the same word as a push.
        let lone = TempRepo::configured("net-preview-none");
        assert_eq!(preview(&lone.repo, None).unwrap(), Done::Not(Why::NoRemote));
    }

    #[test]
    fn pull_only_goes_forward() {
        let (t, other) = with_remote("net-pull");
        let no = AtomicBool::new(false);
        assert_eq!(pull(&t.repo, &no, &mut quiet()).unwrap(), Done::Even { to: "origin/main".into() });
        let theirs = commit(&t, &other, "a.txt");
        t.git(&other, &["push", "-q"]);
        assert_eq!(pull(&t.repo, &no, &mut quiet()).unwrap(), Done::Pulled { upstream: "origin/main".into(), count: 1, stashed: None });
        assert_eq!(t.out(&t.repo, &["rev-parse", "HEAD"]), theirs);

        commit(&t, &t.repo, "mine.txt");
        commit(&t, &other, "b.txt");
        t.git(&other, &["push", "-q"]);
        assert_eq!(
            pull(&t.repo, &no, &mut quiet()).unwrap(),
            Done::Not(Why::Diverged { upstream: "origin/main".into(), ahead: 1, behind: 1 })
        );

        t.git(&t.repo, &["switch", "-q", "-c", "lone"]);
        assert_eq!(pull(&t.repo, &no, &mut quiet()).unwrap(), Done::Not(Why::NoUpstream { branch: "lone".into() }));
        t.git(&t.repo, &["switch", "-q", "--detach"]);
        assert_eq!(pull(&t.repo, &no, &mut quiet()).unwrap(), Done::Not(Why::Detached));
    }

    #[test]
    fn sync_pulls_then_pushes_and_stops_where_it_cannot_go() {
        let (t, other) = with_remote("net-sync");
        let no = AtomicBool::new(false);
        let run = |t: &TempRepo| sync(&t.repo, None, &no, &mut quiet()).unwrap();
        let synced = |to: &str, pulled, pushed, new| Done::Synced { to: to.into(), pulled, pushed, new, stashed: None };
        assert_eq!(run(&t), synced("origin/main", 0, false, false));

        // Only theirs: pulled, nothing to push.
        commit(&t, &other, "a.txt");
        t.git(&other, &["push", "-q"]);
        assert_eq!(run(&t), synced("origin/main", 1, false, false));
        // Only mine: pushed.
        commit(&t, &t.repo, "mine.txt");
        assert_eq!(run(&t), synced("origin/main", 0, true, false));
        // Commits on both sides: it stops at the pull, nothing merged or pushed.
        commit(&t, &t.repo, "mine2.txt");
        t.git(&other, &["pull", "-q"]);
        commit(&t, &other, "b.txt");
        t.git(&other, &["push", "-q"]);
        let mine = t.out(&t.repo, &["rev-parse", "HEAD"]);
        let Done::SyncHalted { at: Step::Pull, why: Why::Diverged { ahead: 1, behind: 1, .. }, pulled: 0, .. } = run(&t) else {
            panic!("not halted at the pull")
        };
        assert_eq!(t.out(&t.repo, &["rev-parse", "HEAD"]), mine);

        // A branch with no upstream is pushed, and gets one.
        t.git(&t.repo, &["switch", "-q", "-c", "feat"]);
        assert_eq!(run(&t), synced("origin/feat", 0, true, true));
        assert_eq!(t.out(&t.repo, &["config", "branch.feat.remote"]), "origin");

        let stopped = AtomicBool::new(true);
        assert_eq!(sync(&t.repo, None, &stopped, &mut quiet()).unwrap(), Done::Stopped);

        t.git(&t.repo, &["switch", "-q", "--detach"]);
        assert_eq!(run(&t), Done::SyncHalted { at: Step::Pull, why: Why::Detached, pulled: 0, stashed: None });
    }

    #[test]
    fn sync_with_no_remote_halts_at_the_push() {
        let t = TempRepo::configured("net-sync-none");
        let no = AtomicBool::new(false);
        // No upstream and no remote: the pull has none to ask, the push says why.
        assert_eq!(
            sync(&t.repo, None, &no, &mut quiet()).unwrap(),
            Done::SyncHalted { at: Step::Push, why: Why::NoRemote, pulled: 0, stashed: None }
        );
    }

    #[test]
    fn a_pull_puts_changes_in_the_way_away_and_back() {
        use crate::autostash::Back;
        let (t, other) = with_remote("net-pull-autostash");
        let no = AtomicBool::new(false);
        commit(&t, &t.repo, "a.txt");
        t.git(&t.repo, &["push", "-q"]);
        t.git(&other, &["pull", "-q"]);
        t.write(
            &other, "a.txt", "theirs
",
        );
        t.git(&other, &["commit", "-q", "-am", "theirs"]);
        t.git(&other, &["push", "-q"]);

        // Edits in a file the pull changes: git alone would refuse.
        t.write(
            &t.repo, "a.txt", "mine
",
        );
        t.write(
            &t.repo, "u.txt", "u
",
        );
        let Done::Pulled { count: 1, stashed: Some(s), .. } = pull(&t.repo, &no, &mut quiet()).unwrap() else { panic!("not pulled") };
        assert!(s.saved.starts_with(backup::PREFIX));
        assert_eq!(s.back, Back::Clash(vec!["a.txt".into()]));
        assert!(std::fs::read_to_string(t.repo.join("a.txt")).unwrap().contains("<<<<<<<"));
        assert_eq!(
            std::fs::read_to_string(t.repo.join("u.txt")).unwrap(),
            "u
"
        );
        assert!(!t.out(&t.repo, &["rev-list", &s.saved, "-1", "--format=%s"]).is_empty());

        // Edits to another file go along without any stash.
        t.git(&t.repo, &["reset", "-q", "--hard"]);
        t.git(&t.repo, &["stash", "drop", "-q"]);
        t.write(
            &other, "b.txt", "b
",
        );
        t.git(&other, &["add", "."]);
        t.git(&other, &["commit", "-q", "-m", "b"]);
        t.git(&other, &["push", "-q"]);
        let done = pull(&t.repo, &no, &mut quiet()).unwrap();
        assert!(matches!(done, Done::Pulled { count: 1, stashed: None, .. }), "{done:?}");
        assert_eq!(
            std::fs::read_to_string(t.repo.join("u.txt")).unwrap(),
            "u
"
        );
    }

    #[test]
    fn push_sets_the_upstream_and_keeps_what_it_goes_over() {
        let (t, other) = with_remote("net-push");
        let no = AtomicBool::new(false);
        assert_eq!(push(&t.repo, None, None, &no, &mut quiet()).unwrap(), Done::Even { to: "origin/main".into() });

        t.git(&t.repo, &["switch", "-q", "-c", "feat"]);
        commit(&t, &t.repo, "one.txt");
        let mut told = 0;
        assert_eq!(
            push(&t.repo, None, None, &no, &mut |_| told += 1).unwrap(),
            Done::Pushed { to: "origin/feat".into(), new: true, saved: None }
        );
        assert_eq!(t.out(&t.repo, &["config", "branch.feat.remote"]), "origin");
        assert_eq!(push(&t.repo, None, None, &no, &mut quiet()).unwrap(), Done::Even { to: "origin/feat".into() });

        // Someone else pushes to feat; this side has not pulled.
        t.git(&other, &["fetch", "-q"]);
        t.git(&other, &["switch", "-q", "feat"]);
        let theirs = commit(&t, &other, "theirs.txt");
        t.git(&other, &["push", "-q"]);
        let mine = commit(&t, &t.repo, "two.txt");
        // Not seen before a fetch: the remote says no.
        assert!(matches!(push(&t.repo, None, None, &no, &mut quiet()).unwrap(), Done::Not(Why::Rejected { .. })));
        fetch(&t.repo, &no, &mut quiet()).unwrap();
        let Done::Not(Why::Behind { lease, count: 1, ahead: 1 }) = push(&t.repo, None, None, &no, &mut quiet()).unwrap() else {
            panic!("not behind")
        };
        assert_eq!((lease.to.as_str(), lease.tip.to_string()), ("origin/feat", theirs.clone()));
        let Done::Pushed { saved: Some(saved), new: false, .. } = push(&t.repo, None, Some(&lease), &no, &mut quiet()).unwrap() else {
            panic!("not pushed")
        };
        assert_eq!(t.out(&t.repo, &["rev-parse", &saved]), theirs);
        assert_eq!(t.out(&other, &["ls-remote", "origin", "refs/heads/feat"]).split('\t').next(), Some(mine.as_str()));

        // The lease is the tip it was given: a remote that moved since is not replaced.
        commit(&t, &other, "again.txt");
        t.git(&other, &["push", "-q", "--force", "origin", "HEAD:feat"]);
        assert!(matches!(push(&t.repo, None, Some(&lease), &no, &mut quiet()).unwrap(), Done::Not(Why::Rejected { .. })));

        t.git(&t.repo, &["switch", "-q", "--detach"]);
        assert_eq!(push(&t.repo, None, None, &no, &mut quiet()).unwrap(), Done::Not(Why::Detached));
    }

    #[test]
    fn push_with_several_remotes_and_no_origin_asks_which() {
        let t = TempRepo::configured("net-push-choose");
        let no = AtomicBool::new(false);
        for name in ["up", "mine"] {
            let bare = t.root.join(format!("{name}.git"));
            t.git(&t.root, &["init", "-q", "--bare", "-b", "main", &bare.to_string_lossy()]);
            t.git(&t.repo, &["remote", "add", name, &bare.to_string_lossy()]);
        }
        assert_eq!(
            push(&t.repo, None, None, &no, &mut quiet()).unwrap(),
            Done::Not(Why::Choose { remotes: vec!["mine".into(), "up".into()] })
        );
        let done = push(&t.repo, Some("mine"), None, &no, &mut quiet()).unwrap();
        assert_eq!(done, Done::Pushed { to: "mine/main".into(), new: true, saved: None });
        assert_eq!(t.out(&t.repo, &["config", "branch.main.remote"]), "mine");
        // Its upstream is the one now.
        assert_eq!(push(&t.repo, None, None, &no, &mut quiet()).unwrap(), Done::Even { to: "mine/main".into() });
    }

    #[test]
    fn ssh_trouble_is_told_apart() {
        let told = git::Error::Told {
            line: "git@github.com: Permission denied (publickey).".into(),
            all: "git@github.com: Permission denied (publickey).\nfatal: Could not read from remote repository.".into(),
        };
        assert!(is_ssh(&told));
        assert!(!is_ssh(&git::Error::Git("Could not resolve host: github.com".into())));
    }
}
