//! The Claude Code sessions tsumugi runs, from `tsumugi ls --json`, and which
//! worktree each one is in. [`fetch`] blocks (it starts a process, with a
//! time limit): call it on a worker thread, every 2 s in front and 10 s behind.
//! Everything else here is plain data in, plain data out.

use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Deserializer};

use crate::{git, paths};

/// How long `tsumugi ls --json` may take before it is given up on.
pub const TIMEOUT: Duration = Duration::from_secs(2);

/// What a session is doing, from the `state` word of `tsumugi ls`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Wants a person: `waiting`, and `waiting?` (output stopped a while ago
    /// while a program still runs), which tsumugi marks more weakly.
    Waiting,
    /// `running`: working, or nothing known.
    Running,
    /// `done`: back at the prompt, or an agent said it finished.
    Done,
    /// `error`: an agent said something went wrong.
    Failed,
    /// A word a newer tsumugi may print; kept as it came.
    Other(String),
}

impl State {
    /// From the `state` word of `tsumugi ls`.
    pub fn from_word(word: &str) -> State {
        match word {
            "waiting" | "waiting?" => State::Waiting,
            "running" => State::Running,
            "done" => State::Done,
            "error" => State::Failed,
            other => State::Other(other.to_owned()),
        }
    }
}

/// One session, as far as michi cares. Fields tsumugi has that michi does not
/// read (tags, ports, charset…) are let go, and so are ones a newer tsumugi adds.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Session {
    /// Its number: what `tsumugi focus N` takes.
    #[serde(deserialize_with = "number")]
    pub id: u64,
    #[serde(default, deserialize_with = "state")]
    pub state: State,
    /// The program that was started (`pwsh`, `claude`).
    #[serde(default)]
    pub command: String,
    /// The AI program running in it (`claude`, `codex`); empty for a plain shell.
    #[serde(default)]
    pub agent: String,
    /// Where the shell started, as tsumugi prints it.
    #[serde(default)]
    pub cwd: PathBuf,
    /// The git branch of `cwd`, or empty.
    #[serde(default)]
    pub branch: String,
    /// What the shell last set the title to: the tab's name.
    #[serde(default)]
    pub title: String,
    /// What the last notification said (what a waiting Claude asks), or empty.
    #[serde(default)]
    pub note: String,
    /// When `state` began, in Unix milliseconds (tsumugi's `Info::since_ms`):
    /// for a waiting session, since when it has waited.
    #[serde(default, deserialize_with = "number")]
    pub since_ms: u64,
    /// Typing into it is not allowed.
    #[serde(default)]
    pub read_only: bool,
}

impl Default for State {
    fn default() -> State {
        State::Other(String::new())
    }
}

impl Session {
    /// An AI program runs in it, not a plain shell.
    pub fn is_agent(&self) -> bool {
        !self.agent.is_empty()
    }
}

/// tsumugi writes its numbers as JSON numbers, which another writer may give a
/// fraction or an exponent: read as a float, a negative one is 0.
fn number<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
    let n = f64::deserialize(d)?;
    Ok(if n.is_finite() && n > 0.0 { n as u64 } else { 0 })
}

fn state<'de, D: Deserializer<'de>>(d: D) -> Result<State, D::Error> {
    Ok(State::from_word(&String::deserialize(d)?))
}

/// Why there is no list although tsumugi is there. Worth a line in the log,
/// not a toast: the next poll tries again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// It did not finish within [`TIMEOUT`] and was stopped.
    Timeout,
    /// It could not be started, or its output could not be read.
    Io(String),
    /// It exited well and wrote something that is not a JSON array.
    Json(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Timeout => f.write_str("tsumugi ls did not answer in time"),
            Error::Io(e) => write!(f, "tsumugi ls: {e}"),
            Error::Json(e) => write!(f, "tsumugi ls --json: {e}"),
        }
    }
}

impl std::error::Error for Error {}

/// Runs `tsumugi ls --json` on the PATH. `Ok(None)` when there is nothing to
/// show and nothing to complain about: tsumugi is not installed, or it exited
/// with a failure (it does when no server is running: "no server"). The caller
/// hides the Claude column. `Err` only for what is worth a line in the log.
pub fn fetch() -> Result<Option<Vec<Session>>, Error> {
    fetch_with("tsumugi", TIMEOUT)
}

/// [`fetch`] with another program and time limit.
pub fn fetch_with(program: &str, timeout: Duration) -> Result<Option<Vec<Session>>, Error> {
    match run(program, &["ls", "--json"], timeout)? {
        Some(out) => parse(&out).map(Some),
        None => Ok(None),
    }
}

/// The JSON array of `tsumugi ls --json`. An element that is not a session
/// (no `id`, a field of the wrong kind) is skipped rather than failing the
/// list; anything but an array is an error.
pub fn parse(out: &[u8]) -> Result<Vec<Session>, Error> {
    let values: Vec<serde_json::Value> = serde_json::from_slice(out).map_err(|e| Error::Json(e.to_string()))?;
    Ok(values.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect())
}

/// Which sessions are in which worktree: element `i` of the result is the
/// sessions of `worktrees[i]`, in the order [`sort`] gives. Only sessions with
/// an AI program in them ([`Session::is_agent`]) are placed; a plain shell is
/// not a Claude. A session belongs to the deepest worktree whose folder holds
/// its `cwd` (a worktree may live inside the main one's folder); one in none
/// is left out. Paths compare as [`paths::key`] does: on Windows case and
/// slashes do not matter, and a trailing separator never does.
pub fn assign<P: AsRef<Path>>(worktrees: impl IntoIterator<Item = P>, sessions: &[Session]) -> Vec<Vec<Session>> {
    assign_as(worktrees, sessions, cfg!(windows))
}

fn assign_as<P: AsRef<Path>>(worktrees: impl IntoIterator<Item = P>, sessions: &[Session], windows: bool) -> Vec<Vec<Session>> {
    let keys: Vec<String> = worktrees.into_iter().map(|w| paths::key_of(&w.as_ref().to_string_lossy(), windows)).collect();
    let mut out: Vec<Vec<Session>> = vec![Vec::new(); keys.len()];
    for s in sessions.iter().filter(|s| s.is_agent() && !s.cwd.as_os_str().is_empty()) {
        let at = paths::key_of(&s.cwd.to_string_lossy(), windows);
        // Of the folders that hold it, the one with the longest name is the
        // innermost: they are all prefixes of the same path. The first wins a tie.
        let mut best: Option<usize> = None;
        for (i, k) in keys.iter().enumerate() {
            if paths::contains_key(k, &at) && best.is_none_or(|b| k.len() > keys[b].len()) {
                best = Some(i);
            }
        }
        if let Some(i) = best {
            out[i].push(s.clone());
        }
    }
    out.iter_mut().for_each(|v| sort(v));
    out
}

/// What the column shows first: waiting (the longest wait first), then
/// running (the longest running first), then failed, done and the rest. The
/// caller shows the first and "+N".
pub fn sort(sessions: &mut [Session]) {
    sessions.sort_by_key(|s| {
        let rank = match s.state {
            State::Waiting => 0,
            State::Running => 1,
            State::Failed => 2,
            State::Done => 3,
            State::Other(_) => 4,
        };
        (rank, s.since_ms, s.id)
    });
}

/// Runs `program args…` for its standard output. `Ok(None)` when it is not
/// there or exits with a failure; `Err` when it could not be run or outlived
/// `timeout` (then it is killed). No console window flashes on Windows.
fn run(program: &str, args: &[&str], timeout: Duration) -> Result<Option<Vec<u8>>, Error> {
    let mut c = Command::new(program);
    c.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    git::hide_window(&mut c);
    let mut child = match c.spawn() {
        Ok(child) => child,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(Error::Io(e.to_string())),
    };
    // Read on the side so a long list never fills the pipe and stalls the child.
    let mut stdout = child.stdout.take().expect("piped");
    let reader = std::thread::Builder::new()
        .name("tsumugi-ls".into())
        .spawn(move || {
            let mut v = Vec::new();
            stdout.read_to_end(&mut v).map(|_| v)
        })
        .map_err(|e| Error::Io(e.to_string()));
    let reader = match reader {
        Ok(r) => r,
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
    };
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                // Its pipe closes with it, which ends the reader on its own.
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::Timeout);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Error::Io(e.to_string()));
            }
        }
    };
    let out = reader.join().map_err(|_| Error::Io("reader thread panicked".into()))?.map_err(|e| Error::Io(e.to_string()))?;
    Ok(status.success().then_some(out))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape `tsumugi ls --json` printed on a real machine (`ls_json` in
    /// tsumugi's cli.rs), a waiting `waiting?` pair and a plain shell.
    const REAL: &str = r#"[
  {
    "id": 2,
    "state": "waiting",
    "command": "pwsh",
    "agent": "claude",
    "cwd": "C:\\dev\\tsumugi",
    "project": "C:\\dev\\tsumugi",
    "branch": "main",
    "title": "✳ plan",
    "note": "Claude is waiting for your input",
    "since_ms": 1792053934655,
    "tags": [
      "tsumugi"
    ],
    "ports": [3000],
    "muted": false,
    "charset": "UTF-8",
    "read_only": false,
    "profile": ""
  },
  {
    "id": 4,
    "state": "waiting?",
    "command": "pwsh",
    "agent": "claude",
    "cwd": "C:\\dev\\yagura",
    "project": "C:\\dev\\yagura",
    "branch": "main",
    "title": "✳ Claude Code",
    "note": "",
    "since_ms": 1792055643580,
    "tags": [],
    "ports": [],
    "muted": true,
    "charset": "UTF-8",
    "read_only": true,
    "profile": ""
  },
  {
    "id": 5,
    "state": "done",
    "command": "pwsh",
    "agent": "",
    "cwd": "C:\\dev\\yagura",
    "project": "C:\\dev\\yagura",
    "branch": "main",
    "title": "pwsh",
    "note": "",
    "since_ms": 1792055000000,
    "tags": [],
    "ports": [],
    "muted": false,
    "charset": "UTF-8",
    "read_only": false,
    "profile": ""
  }
]"#;

    fn s(id: u64, state: State, cwd: &str, since_ms: u64) -> Session {
        Session {
            id,
            state,
            command: "pwsh".into(),
            agent: "claude".into(),
            cwd: PathBuf::from(cwd),
            branch: String::new(),
            title: format!("t{id}"),
            note: String::new(),
            since_ms,
            read_only: false,
        }
    }

    fn ids(v: &[Session]) -> Vec<u64> {
        v.iter().map(|s| s.id).collect()
    }

    #[test]
    fn the_real_shape_is_read() {
        let l = parse(REAL.as_bytes()).unwrap();
        assert_eq!(ids(&l), [2, 4, 5]);
        assert_eq!(l[0].state, State::Waiting);
        assert_eq!(l[0].agent, "claude");
        assert_eq!(l[0].cwd, PathBuf::from(r"C:\dev\tsumugi"));
        assert_eq!(l[0].branch, "main");
        assert_eq!(l[0].title, "✳ plan");
        assert_eq!(l[0].note, "Claude is waiting for your input");
        assert_eq!(l[0].since_ms, 1_792_053_934_655);
        assert!(!l[0].read_only && l[1].read_only);
        assert_eq!(l[1].state, State::Waiting, "waiting? is a wait too");
        assert!(l[0].is_agent() && !l[2].is_agent());
    }

    #[test]
    fn an_empty_list_is_a_list() {
        assert_eq!(parse(b"[]").unwrap(), []);
        assert_eq!(parse(b" [ ]\n").unwrap(), []);
    }

    #[test]
    fn unknown_and_missing_fields_do_not_fail_the_list() {
        let l = parse(br#"[{"id": 7, "state": "running", "future": {"a": [1, 2]}, "cwd": "/w/a"}, {"id": 8}]"#).unwrap();
        assert_eq!(ids(&l), [7, 8]);
        assert_eq!(l[0].state, State::Running);
        assert_eq!(l[0].title, "");
        assert_eq!(l[0].since_ms, 0);
        assert!(!l[0].read_only);
        assert_eq!(l[1].state, State::Other(String::new()));
        assert_eq!(l[1].cwd, PathBuf::new());
    }

    #[test]
    fn an_element_that_is_no_session_is_skipped() {
        let l = parse(br#"[{"state": "running"}, 5, null, {"id": "x"}, {"id": 3, "cwd": 4}, {"id": 9, "state": "done"}]"#).unwrap();
        assert_eq!(ids(&l), [9]);
    }

    #[test]
    fn numbers_may_be_floats() {
        let l = parse(br#"[{"id": 1.0, "since_ms": 1.7920539e12}, {"id": 2, "since_ms": -5}]"#).unwrap();
        assert_eq!((l[0].id, l[0].since_ms), (1, 1_792_053_900_000));
        assert_eq!(l[1].since_ms, 0);
    }

    #[test]
    fn what_is_not_a_json_array_is_an_error() {
        for bad in ["", "no server", "{\"id\": 1}", "[{\"id\": 1}", "null", "\u{feff}"] {
            assert!(matches!(parse(bad.as_bytes()), Err(Error::Json(_))), "{bad:?}");
        }
    }

    #[test]
    fn state_words_are_mapped() {
        assert_eq!(State::from_word("waiting"), State::Waiting);
        assert_eq!(State::from_word("waiting?"), State::Waiting);
        assert_eq!(State::from_word("running"), State::Running);
        assert_eq!(State::from_word("done"), State::Done);
        assert_eq!(State::from_word("error"), State::Failed);
        assert_eq!(State::from_word("paused"), State::Other("paused".into()));
        assert_eq!(State::from_word("Waiting"), State::Other("Waiting".into()));
    }

    #[test]
    fn a_session_goes_to_the_deepest_worktree_that_holds_it() {
        let wt = ["/w/main", "/w/main/wt/fix", "/w/other"];
        let list = [
            s(1, State::Running, "/w/main", 1),
            s(2, State::Running, "/w/main/src/x", 1),
            s(3, State::Running, "/w/main/wt/fix", 1),
            s(4, State::Running, "/w/main/wt/fix/crates/a/", 1),
            s(5, State::Running, "/w/main/wt/fixed", 1),
            s(6, State::Running, "/w/other/", 1),
            s(7, State::Running, "/w/otherwise", 1),
            s(8, State::Running, "/elsewhere", 1),
            s(9, State::Running, "", 1),
        ];
        for windows in [false, true] {
            let a = assign_as(wt, &list, windows);
            assert_eq!(a.len(), 3);
            assert_eq!(ids(&a[0]), [1, 2, 5], "windows: {windows}");
            assert_eq!(ids(&a[1]), [3, 4]);
            assert_eq!(ids(&a[2]), [6]);
        }
    }

    #[test]
    fn a_name_that_only_starts_the_same_is_not_inside() {
        let a = assign_as(["/dev/a"], &[s(1, State::Running, "/dev/ab", 1), s(2, State::Running, "/dev/ab/x", 1)], false);
        assert!(a[0].is_empty());
        let a = assign_as([r"C:\dev\a"], &[s(1, State::Running, r"C:\dev\ab", 1)], true);
        assert!(a[0].is_empty());
    }

    #[test]
    fn windows_ignores_case_and_slashes_and_elsewhere_does_not() {
        let list = [s(1, State::Waiting, r"c:\DEV\Michi-wt\fix\src", 1), s(2, State::Waiting, r"C:\dev\michi\", 1)];
        let a = assign_as(["C:/dev/michi", "C:/dev/michi-wt/fix/"], &list, true);
        assert_eq!(ids(&a[0]), [2]);
        assert_eq!(ids(&a[1]), [1]);
        let a = assign_as(["/dev/Michi"], &[s(1, State::Waiting, "/dev/michi", 1), s(2, State::Waiting, "/dev/Michi/", 1)], false);
        assert_eq!(ids(&a[0]), [2]);
    }

    #[test]
    fn the_real_assign_follows_the_platform() {
        let a = assign([PathBuf::from("/w/a"), PathBuf::from("/w/a/b")], &[s(1, State::Done, "/w/a/b/c", 1)]);
        assert_eq!((a[0].len(), ids(&a[1])), (0, vec![1]));
    }

    #[test]
    fn a_plain_shell_is_not_a_claude() {
        let mut shell = s(1, State::Done, "/w/a", 1);
        shell.agent.clear();
        let a = assign_as(["/w/a"], &[shell], false);
        assert!(a[0].is_empty());
    }

    #[test]
    fn waiting_first_the_longest_wait_first_then_running_then_the_rest() {
        let list = [
            s(1, State::Done, "/w", 10),
            s(2, State::Running, "/w", 30),
            s(3, State::Waiting, "/w", 50),
            s(4, State::Other("x".into()), "/w", 5),
            s(5, State::Waiting, "/w", 20),
            s(6, State::Failed, "/w", 40),
            s(7, State::Running, "/w", 20),
            s(8, State::Waiting, "/w", 20),
        ];
        let a = assign_as(["/w"], &list, false);
        assert_eq!(ids(&a[0]), [5, 8, 3, 7, 2, 6, 1, 4]);
    }

    #[test]
    fn no_worktrees_no_groups() {
        assert!(assign_as(Vec::<PathBuf>::new(), &[s(1, State::Running, "/w", 1)], false).is_empty());
    }

    #[test]
    fn a_program_that_is_not_there_is_no_tsumugi() {
        assert_eq!(fetch_with("michi-no-such-program-for-tests", TIMEOUT), Ok(None));
    }

    #[test]
    fn a_failing_program_is_no_tsumugi_and_a_good_one_is_read_as_json() {
        // git is on the PATH wherever the tests run.
        assert_eq!(run("git", &["no-such-subcommand-for-tests"], TIMEOUT), Ok(None));
        let out = run("git", &["--version"], TIMEOUT).unwrap().unwrap();
        assert!(String::from_utf8_lossy(&out).starts_with("git version"));
    }
}
