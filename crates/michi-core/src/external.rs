//! Opening a clashed file in an external tool: the tool `merge.tool` names
//! (through `git mergetool`, which adds the file when the tool is done), or
//! else the program the system opens the file with.

use std::path::Path;
use std::process::Command;

use crate::git;

/// How the file was opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opened {
    /// `git mergetool` ran and the tool is closed again.
    Tool,
    /// The system's program for the file was started; it is not waited for.
    File,
}

/// The tool `merge.tool` names, when anyone said.
pub fn tool(wt: &Path) -> Result<Option<String>, git::Error> {
    Ok(git::read_maybe(wt, &["config", "--get", "merge.tool"])?
        .map(|v| String::from_utf8_lossy(&v).trim().to_owned())
        .filter(|t| !t.is_empty()))
}

/// Opens the file at `path` of the worktree at `wt`. With a merge tool set
/// this waits until the tool is closed.
pub fn open(wt: &Path, path: &str) -> Result<Opened, git::Error> {
    if tool(wt)?.is_some() {
        let out = git::command().arg("-C").arg(wt).args(["mergetool", "--no-prompt", "--", path]).output().map_err(git::started)?;
        return if out.status.success() { Ok(Opened::Tool) } else { Err(refused(&String::from_utf8_lossy(&out.stderr))) };
    }
    let file = wt.join(path);
    opener().arg(&file).spawn().map(|_| Opened::File).map_err(|e| git::Error::Io(format!("{}: {e}", file.display())))
}

/// `git mergetool`'s complaint without its warnings (the CRLF notice comes
/// first, and says nothing of why it failed). A deleted-on-one-side file makes
/// it ask which side to keep, and no terminal is there to answer.
fn refused(stderr: &str) -> git::Error {
    if stderr.contains("Use (m)odified or (d)eleted") {
        return git::Error::Git("git mergetool asks which side to keep for a deleted file; use o or t instead".to_owned());
    }
    let rest: Vec<&str> = stderr.lines().filter(|l| !l.starts_with("warning:")).collect();
    git::refused(&rest.join(
        "
",
    ))
}

#[cfg(windows)]
fn opener() -> Command {
    let mut c = Command::new("explorer");
    git::hide_window(&mut c);
    c
}

#[cfg(target_os = "macos")]
fn opener() -> Command {
    Command::new("open")
}

#[cfg(not(any(windows, target_os = "macos")))]
fn opener() -> Command {
    Command::new("xdg-open")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn mergetool_complaints_skip_warnings() {
        let w = "warning: LF will be replaced by CRLF
fatal: no tool
";
        assert_eq!(
            refused(w).to_string(),
            git::refused(
                "fatal: no tool
"
            )
            .to_string()
        );
        let ask = "warning: x
Deleted merge conflict for 'f':
Use (m)odified or (d)eleted file, or (a)bort?";
        assert!(refused(ask).to_string().contains("which side to keep"));
    }

    #[test]
    fn a_merge_tool_is_run_and_its_result_added() {
        let t = TempRepo::configured("external-tool");
        let r = &t.repo;
        t.write(r, "a.txt", "base\n");
        t.git(r, &["add", "."]);
        t.git(r, &["commit", "-q", "-m", "base"]);
        t.git(r, &["switch", "-q", "-c", "feat"]);
        t.write(r, "a.txt", "feat\n");
        t.git(r, &["commit", "-q", "-am", "feat"]);
        t.git(r, &["switch", "-q", "main"]);
        t.write(r, "a.txt", "main\n");
        t.git(r, &["commit", "-q", "-am", "main"]);
        assert_eq!(tool(r).unwrap(), None);
        let merged = std::process::Command::new("git").arg("-C").arg(r).args(["merge", "feat"]).output().unwrap();
        assert!(!merged.status.success());
        for (k, v) in [
            ("merge.tool", "t"),
            ("mergetool.t.cmd", "cp \"$REMOTE\" \"$MERGED\""),
            ("mergetool.t.trustExitCode", "true"),
            ("mergetool.keepBackup", "false"),
        ] {
            t.git(r, &["config", k, v]);
        }
        assert_eq!(tool(r).unwrap().as_deref(), Some("t"));
        assert_eq!(open(r, "a.txt").unwrap(), Opened::Tool);
        assert_eq!(std::fs::read_to_string(r.join("a.txt")).unwrap().trim(), "feat");
        assert!(crate::conflict::unmerged(r).unwrap().is_empty(), "the tool's result was added");
    }
}
