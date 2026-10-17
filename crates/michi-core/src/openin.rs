//! Opening a worktree's folder in another program: a terminal, an editor or
//! the system's file manager. Each is a command line the user can change
//! (`[open]` of config.toml); `{path}` in it is the folder, which is added
//! at the end when the line has no `{path}`. The program is started and not
//! waited for, and no shell is involved.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::git;

/// Which program a folder is opened in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Terminal,
    Editor,
    Explorer,
}

impl Tool {
    pub const ALL: [Tool; 3] = [Tool::Terminal, Tool::Editor, Tool::Explorer];

    /// The key of `[open]` in config.toml.
    pub fn key(self) -> &'static str {
        match self {
            Tool::Terminal => "terminal",
            Tool::Editor => "editor",
            Tool::Explorer => "explorer",
        }
    }
}

/// The command line used when config.toml says none.
pub fn default_command(tool: Tool) -> &'static str {
    match tool {
        Tool::Terminal => "tsumugi new {path}",
        Tool::Editor => os::EDITOR,
        Tool::Explorer => os::EXPLORER,
    }
}

/// The program and arguments of `template` for `folder`.
pub fn argv(template: &str, folder: &Path) -> Result<Vec<String>, String> {
    let path = folder.to_string_lossy();
    let mut words: Vec<String> = Vec::new();
    let (mut cur, mut quoted, mut any) = (String::new(), false, false);
    for c in template.chars() {
        if c == '"' {
            quoted = !quoted;
            any = true;
        } else if c.is_whitespace() && !quoted {
            if any {
                words.push(std::mem::take(&mut cur));
                any = false;
            }
        } else {
            cur.push(c);
            any = true;
        }
    }
    if quoted {
        return Err(format!("unclosed quote: {template}"));
    }
    if any {
        words.push(cur);
    }
    if words.is_empty() {
        return Err("empty command".to_owned());
    }
    let has_path = words.iter().any(|w| w.contains("{path}"));
    let mut out: Vec<String> = words.iter().map(|w| w.replace("{path}", &path)).collect();
    if !has_path {
        out.push(path.into_owned());
    }
    Ok(out)
}

/// Starts `template` on `folder`; does not wait for it.
pub fn open(template: &str, folder: &Path) -> Result<(), git::Error> {
    let argv = argv(template, folder).map_err(git::Error::Io)?;
    let mut c = Command::new(&argv[0]);
    c.args(&argv[1..]).current_dir(folder).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    git::hide_window(&mut c);
    let mut child = c.spawn().map_err(|e| git::Error::Io(format!("{}: {e}", argv[0])))?;
    // Reaped on a thread of its own, so nothing is left defunct.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(windows)]
mod os {
    pub const EDITOR: &str = "code.cmd {path}";
    pub const EXPLORER: &str = "explorer {path}";
}

#[cfg(target_os = "macos")]
mod os {
    pub const EDITOR: &str = "code {path}";
    pub const EXPLORER: &str = "open {path}";
}

#[cfg(not(any(windows, target_os = "macos")))]
mod os {
    pub const EDITOR: &str = "code {path}";
    pub const EXPLORER: &str = "xdg-open {path}";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_folder_is_put_where_path_is_or_added() {
        let p = Path::new("/w/a b");
        assert_eq!(argv("code {path}", p).unwrap(), ["code", "/w/a b"]);
        assert_eq!(argv("code", p).unwrap(), ["code", "/w/a b"]);
        assert_eq!(argv("term --cwd={path} -x", p).unwrap(), ["term", "--cwd=/w/a b", "-x"]);
    }

    #[test]
    fn quotes_group_words_and_bad_lines_are_refused() {
        let p = Path::new("/w");
        assert_eq!(argv("\"C:\\Program Files\\x.exe\" --new {path}", p).unwrap(), ["C:\\Program Files\\x.exe", "--new", "/w"]);
        assert!(argv("  ", p).is_err());
        assert!(argv("a \"b", p).is_err());
    }

    #[test]
    fn every_default_names_a_program() {
        for t in Tool::ALL {
            assert!(argv(default_command(t), Path::new("/w")).is_ok());
        }
    }

    #[test]
    fn a_missing_program_is_an_error_not_a_panic() {
        assert!(open("michi-no-such-program-xyz {path}", &std::env::temp_dir()).is_err());
    }
}
