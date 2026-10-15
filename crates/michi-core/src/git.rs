//! Running git. Every call here only reads, and says so: `GIT_OPTIONAL_LOCKS=0`
//! so a read never takes the index lock a Claude Code session in the same
//! worktree may be about to want, and `GIT_TERMINAL_PROMPT=0` so nothing ever
//! waits for a password nobody will type. The calls block; run them off the
//! UI thread.

use std::fmt;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

/// The oldest git michi works with: `git merge-tree --write-tree`, which the
/// conflict prediction runs, came in 2.38.
pub const MIN: Version = Version(2, 38, 0);

/// A git version, `major.minor.patch`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u32, pub u32, pub u32);

impl Version {
    /// From what `git --version` prints: `git version 2.47.1.windows.1`,
    /// `git version 2.39.3 (Apple Git-146)`, `git version 2.38`.
    pub fn parse(s: &str) -> Option<Version> {
        let word = s.trim().strip_prefix("git version ")?.split_whitespace().next()?;
        let mut parts = word.split('.').map(|p| p.chars().take_while(char::is_ascii_digit).collect::<String>().parse::<u32>().ok());
        let major = parts.next()??;
        let minor = parts.next()??;
        let patch = parts.next().flatten().unwrap_or(0);
        Some(Version(major, minor, patch))
    }

    /// Whether michi works with it.
    pub fn is_enough(self) -> bool {
        self >= MIN
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// Why git gave nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// No `git` on the PATH.
    NoGit,
    /// The folder is in no repository.
    NotRepo,
    /// git ran and refused: the first line it wrote on standard error.
    Git(String),
    /// git could not be started, or wrote what could not be read.
    Io(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoGit => f.write_str("git was not found on the PATH"),
            Error::NotRepo => f.write_str("not a git repository"),
            Error::Git(e) | Error::Io(e) => f.write_str(e),
        }
    }
}

impl std::error::Error for Error {}

/// A git command with michi's environment, not yet run.
pub fn command() -> Command {
    let mut c = Command::new("git");
    c.env("GIT_OPTIONAL_LOCKS", "0").env("GIT_TERMINAL_PROMPT", "0").stdin(Stdio::null());
    hide_window(&mut c);
    c
}

/// No console window flashing up for each call from the windowed exe.
#[cfg(windows)]
fn hide_window(c: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    c.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide_window(_: &mut Command) {}

/// `git -C dir args…`: its standard output, or why there is none.
pub fn read(dir: &Path, args: &[&str]) -> Result<Vec<u8>, Error> {
    let out = command().arg("-C").arg(dir).args(args).output().map_err(started)?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(refused(&String::from_utf8_lossy(&out.stderr)))
    }
}

/// [`read`] as text, the last line break taken off.
pub fn read_text(dir: &Path, args: &[&str]) -> Result<String, Error> {
    let out = read(dir, args)?;
    let text = String::from_utf8(out).map_err(|e| Error::Io(format!("git {}: {e}", args.join(" "))))?;
    Ok(text.trim_end_matches(['\n', '\r']).to_owned())
}

/// The version of the git on the PATH.
pub fn version() -> Result<Version, Error> {
    let out = command().arg("--version").output().map_err(started)?;
    let text = String::from_utf8_lossy(&out.stdout);
    Version::parse(&text).ok_or_else(|| Error::Io(format!("unexpected `git --version`: {}", text.trim())))
}

fn started(e: io::Error) -> Error {
    match e.kind() {
        io::ErrorKind::NotFound => Error::NoGit,
        _ => Error::Io(format!("git: {e}")),
    }
}

/// What git's standard error says, as an [`Error`].
fn refused(stderr: &str) -> Error {
    if stderr.contains("not a git repository") {
        return Error::NotRepo;
    }
    let line = stderr.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("git failed");
    Error::Git(line.strip_prefix("fatal: ").unwrap_or(line).to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_are_read_from_every_platform_s_wording() {
        assert_eq!(Version::parse("git version 2.47.1.windows.1\n"), Some(Version(2, 47, 1)));
        assert_eq!(Version::parse("git version 2.39.3 (Apple Git-146)"), Some(Version(2, 39, 3)));
        assert_eq!(Version::parse("git version 2.38"), Some(Version(2, 38, 0)));
        assert_eq!(Version::parse("git version 2.40.0.rc1"), Some(Version(2, 40, 0)));
        assert_eq!(Version::parse("hg version 6"), None);
        assert_eq!(Version::parse("git version x"), None);
    }

    #[test]
    fn versions_order_and_print() {
        assert!(Version(2, 38, 0).is_enough());
        assert!(Version(3, 0, 0).is_enough());
        assert!(!Version(2, 37, 9).is_enough());
        assert!(Version(2, 9, 0) < Version(2, 10, 0));
        assert_eq!(Version(2, 47, 1).to_string(), "2.47.1");
    }

    #[test]
    fn git_s_refusals_are_sorted() {
        let not_repo = "fatal: not a git repository (or any of the parent directories): .git\n";
        assert_eq!(refused(not_repo), Error::NotRepo);
        assert_eq!(refused("\nfatal: bad revision 'x'\n"), Error::Git("bad revision 'x'".into()));
        assert_eq!(refused(""), Error::Git("git failed".into()));
    }

    #[test]
    fn the_git_here_has_a_version() {
        let v = version().expect("git on the PATH");
        assert!(v.0 >= 2, "{v}");
    }
}
