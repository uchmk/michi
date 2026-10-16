//! `michi.com`: the console-subsystem front for `michi.exe`, the same as
//! kura's `kura.com` and yagura's `yagura.com`.
//!
//! `michi.exe` is a windowed program, so PowerShell does not wait for it at
//! the end of a pipeline and does not take its output: `$v = & michi.exe
//! --version` came back empty. The release zip ships this binary beside
//! `michi.exe` as `michi.com`. `PATHEXT` lists `.COM` before `.EXE`, so
//! typing `michi` finds this one first. It is the shape of Visual Studio's
//! `devenv.com` and `devenv.exe`.
//!
//! Given an option, `michi.exe` answers in text (`--version`, `--help` or a
//! refusal) and exits: this runs it with the same arguments and the same
//! standard handles, waits, and passes on the exit code. Being a console
//! program, it is what the shell waits for. Given a folder or nothing,
//! `michi.exe` opens the window: this starts it and returns as soon as the
//! window is taking input, much as typing `michi.exe` always did, or with its
//! exit code and its words if it fails before then (not a folder, two
//! folders).
//!
//! The window does not get this console's standard handles. When they are a
//! pipe (`pwsh -File x.ps1 | Tee-Object`), the reader waits until every
//! process holding the pipe has let go, and with the window holding it that
//! was until the window closed (kura's #192). The window writes into pipes of
//! this program's own instead, copied out here until the window is up.
//!
//! It holds no logic of its own beyond that choice: the version and the help
//! come from `michi.exe`, so the two cannot disagree.

/// Whether `michi.exe` will answer this command line in text and exit rather
/// than open a window: an option does, `--version` and `--help` or a refusal
/// of one it does not know, wherever it stands. Kept in step with
/// `parse_args` in `main.rs`, which takes `-` alone for a folder.
fn answers_in_text(args: &[std::ffi::OsString]) -> bool {
    args.iter().any(|a| a.to_str().is_some_and(|s| s.starts_with('-') && s.len() > 1))
}

#[cfg(windows)]
fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let exe = match std::env::current_exe() {
        Ok(me) => me.with_file_name("michi.exe"),
        Err(e) => {
            eprintln!("michi.com: cannot tell where it is: {e}");
            std::process::exit(2);
        }
    };
    if !exe.is_file() {
        eprintln!("michi.com: no michi.exe beside it ({})", exe.display());
        std::process::exit(2);
    }
    let mut cmd = std::process::Command::new(&exe);
    cmd.args(&args);
    if answers_in_text(&args) {
        match cmd.status() {
            Ok(status) => std::process::exit(status.code().unwrap_or(1)),
            Err(e) => {
                eprintln!("michi.com: {}: {e}", exe.display());
                std::process::exit(2);
            }
        }
    }
    keep_own_handles();
    let piped = std::process::Stdio::piped;
    let mut child = match cmd.stdin(std::process::Stdio::null()).stdout(piped()).stderr(piped()).spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("michi.com: {}: {e}", exe.display());
            std::process::exit(2);
        }
    };
    let copied = [copy_out(child.stdout.take(), std::io::stdout), copy_out(child.stderr.take(), std::io::stderr)];
    let code = until_the_window_is_up(&mut child);
    if let Ok(Some(_)) = child.try_wait() {
        // It failed: let its words reach the console before the exit code.
        // Bounded, because a program it started may still hold the pipe.
        for done in copied {
            let _ = done.recv_timeout(std::time::Duration::from_secs(2));
        }
    }
    std::process::exit(code);
}

/// Makes this program's standard handles stay here. `Command` starts
/// `michi.exe` with every inheritable handle, not only the three it is given,
/// so the pipe of a `| Tee-Object` would reach the window all the same.
#[cfg(windows)]
fn keep_own_handles() {
    use windows_sys::Win32::Foundation::{SetHandleInformation, HANDLE_FLAG_INHERIT, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Console::{GetStdHandle, STD_ERROR_HANDLE, STD_INPUT_HANDLE, STD_OUTPUT_HANDLE};

    for which in [STD_INPUT_HANDLE, STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
        // SAFETY: this process's own standard handles; a missing one is null.
        let h = unsafe { GetStdHandle(which) };
        if !h.is_null() && h != INVALID_HANDLE_VALUE {
            unsafe { SetHandleInformation(h, HANDLE_FLAG_INHERIT, 0) };
        }
    }
}

/// Copies what the window writes to this console, on a thread of its own so
/// that a long message cannot fill the pipe and stall `michi.exe`. The
/// receiver hears once the pipe is closed and everything is copied.
#[cfg(windows)]
fn copy_out<R, W>(from: Option<R>, to: fn() -> W) -> std::sync::mpsc::Receiver<()>
where
    R: std::io::Read + Send + 'static,
    W: std::io::Write + 'static,
{
    let (done, finished) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        if let Some(mut from) = from {
            let mut out = to();
            let _ = std::io::copy(&mut from, &mut out);
            let _ = out.flush();
        }
        let _ = done.send(());
    });
    finished
}

/// Waits for the window to start taking input, then lets it be: the prompt
/// comes back with michi still open. If `michi.exe` exits first, it failed,
/// and its exit code is this one's. The cap is for a machine so slow that
/// neither has happened yet.
#[cfg(windows)]
fn until_the_window_is_up(child: &mut std::process::Child) -> i32 {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT};
    use windows_sys::Win32::System::Threading::{WaitForInputIdle, WaitForSingleObject};

    let process = child.as_raw_handle();
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.code().unwrap_or(1),
            Ok(None) => {}
            Err(_) => return 0,
        }
        // SAFETY: the child's process handle, open for as long as `child` is.
        match unsafe { WaitForInputIdle(process, 50) } {
            // Idle: its message loop is running and waiting for input -- or
            // it has exited, which WaitForInputIdle also answers with 0
            // (kura's CI caught that). So look again, giving an exit that is
            // already under way a moment to land.
            0 => {
                if unsafe { WaitForSingleObject(process, 250) } == WAIT_OBJECT_0 {
                    return match child.try_wait() {
                        Ok(Some(status)) => status.code().unwrap_or(1),
                        _ => 1,
                    };
                }
                return 0;
            }
            WAIT_TIMEOUT => {}
            // WAIT_FAILED: no message queue to wait on, which is a debug
            // build (a console program). Give a failure its moment, then go.
            _ => {
                std::thread::sleep(std::time::Duration::from_millis(300));
                return match child.try_wait() {
                    Ok(Some(status)) => status.code().unwrap_or(1),
                    _ => 0,
                };
            }
        }
        if start.elapsed() > std::time::Duration::from_secs(10) {
            return 0;
        }
    }
}

#[cfg(not(windows))]
fn main() {
    let _ = answers_in_text;
    eprintln!("michi-com is the console front for michi.exe, and only does anything on Windows");
    std::process::exit(2);
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
        for waits in [&["--version"][..], &["-V"], &["--help"], &["-h"], &["--bogus"], &["repo", "--help"]] {
            assert!(answers_in_text(&line(waits)), "{waits:?}");
        }
        for opens in [&[][..], &["repo"], &["-"], &["a", "b"]] {
            assert!(!answers_in_text(&line(opens)), "{opens:?}");
        }
    }
}
