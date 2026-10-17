//! `michi.com`: the console-subsystem front for `michi.exe`, which the
//! release zip ships beside it. PowerShell does not wait for a windowed
//! program and does not take its output (`$v = & michi.exe --version` came
//! back empty); `PATHEXT` lists `.COM` before `.EXE`, so typing `michi`
//! finds this one first. The front itself is `ito_console::front`, shared
//! with kura, yagura, tsumugi and minato; what is michi's is which command
//! lines answer in text.
//!
//! It holds no logic of its own beyond that choice: the version and the help
//! come from `michi.exe`, so the two cannot disagree.

/// Whether `michi.exe` will answer this command line in text and exit rather
/// than open a window: an option does, `--version` and `--help` or a refusal
/// of one it does not know, wherever it stands. Kept in step with
/// `parse_args` in `main.rs`, which takes `-` alone for a folder.
fn answers_in_text(args: &[std::ffi::OsString]) -> bool {
    // The word after `--keys` is its script, never an option: `--keys -x`
    // is not read here (`parse_args` does the same).
    let mut args = args.iter().map(|a| a.to_str().unwrap_or(""));
    while let Some(a) = args.next() {
        if a == "--keys" {
            // No script: `michi.exe` refuses in text.
            if args.next().is_none() {
                return true;
            }
        } else if a.starts_with('-') && a.len() > 1 {
            return true;
        }
    }
    false
}

fn main() {
    ito_console::front("michi", answers_in_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(args: &[&str]) -> Vec<std::ffi::OsString> {
        args.iter().map(Into::into).collect()
    }

    /// An option is answered in text; a folder or nothing opens the window.
    #[test]
    fn it_waits_only_for_what_answers_in_text() {
        for waits in
            [&["--version"][..], &["-V"], &["--help"], &["-h"], &["--bogus"], &["repo", "--help"], &["--keys", "j", "-V"], &["--keys"]]
        {
            assert!(answers_in_text(&line(waits)), "{waits:?}");
        }
        for opens in [&[][..], &["repo"], &["-"], &["a", "b"], &["--keys", "j"], &["--keys", "-x"], &["repo", "--keys", "<C-q>"]] {
            assert!(!answers_in_text(&line(opens)), "{opens:?}");
        }
    }
}
