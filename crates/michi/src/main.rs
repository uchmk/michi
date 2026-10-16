//! michi: a keyboard-driven Git client for a repository many Claude Code
//! sessions work in at once, the sibling of kura, yagura, tsumugi and
//! minato. The window and the input; the state is in `app.rs`, the drawing
//! in `ui.rs` and `settings.rs`, and what has nothing to do with egui in the
//! `michi-core` crate.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod act;
mod app;
mod changes;
mod changes_ui;
mod detail;
mod detail_ui;
mod fonts;
mod fswatch;
mod graph;
mod graph_ui;
mod i18n;
mod icon;
mod picker;
mod picker_ui;
mod prefs;
mod repo;
mod review;
mod review_ui;
mod settings;
mod side_ui;
mod tidy;
mod tidy_ui;
mod ui;
mod work;
mod wt_ui;

use std::ffi::OsString;
use std::path::PathBuf;

use app::{App, Mode, View};

/// The named keys that move in a view, by the names `App::named` takes.
const MOVES: [(egui::Key, &str); 8] = [
    (egui::Key::ArrowDown, "ArrowDown"),
    (egui::Key::ArrowUp, "ArrowUp"),
    (egui::Key::ArrowLeft, "ArrowLeft"),
    (egui::Key::ArrowRight, "ArrowRight"),
    (egui::Key::PageDown, "PageDown"),
    (egui::Key::PageUp, "PageUp"),
    (egui::Key::Home, "Home"),
    (egui::Key::End, "End"),
];

/// The window's icon, in pixels square.
const ICON_PX: u32 = 128;

const USAGE: &str = "\
michi — a keyboard-driven Git client

usage: michi [<path>] [--version | --help]

  <path>  a folder in the repository to open (default: the current folder)
";

struct Michi {
    app: App,
    /// Whether the window was in front at the last frame.
    front: bool,
}

impl Michi {
    /// The characters typed and the named keys. While the graph's filter has
    /// the keys, only Enter (the next hit) and the arrows (the selection)
    /// are taken from it; Escape has already let go of it. While the commit
    /// message has them, only Ctrl+Enter (commit) is taken; the Escape that
    /// let go of it goes no further.
    fn keys(&mut self, ctx: &egui::Context) {
        let (events, palette) = ctx.input_mut(|i| {
            let palette = i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::P);
            (i.events.clone(), palette)
        });
        if palette {
            self.palette();
        }
        if self.app.picker.is_some() {
            self.picker_keys(ctx);
            return;
        }
        let typing = ctx.memory(|m| m.has_focus(changes_ui::message_id()));
        let commit =
            (typing || self.app.view == View::Changes) && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter));
        if commit {
            self.app.changes_commit();
        }
        if typing {
            return;
        }
        let let_go = std::mem::take(&mut self.app.changes.typing);
        if ctx.memory(|m| m.has_focus(graph_ui::filter_id())) {
            let (next, back, down, up) = ctx.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
                    i.consume_key(egui::Modifiers::SHIFT, egui::Key::Enter),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                )
            });
            if next || back {
                self.app.graph.next_hit(back);
            }
            if down {
                self.app.graph.move_by(1);
            }
            if up {
                self.app.graph.move_by(-1);
            }
            return;
        }
        let mut events = events.into_iter();
        while let Some(e) = events.next() {
            match e {
                egui::Event::Key { key: egui::Key::Escape, pressed: true, .. } if !let_go => {
                    self.app.escape();
                }
                egui::Event::Key { key: egui::Key::Enter, pressed: true, modifiers, .. } if modifiers.is_none() => {
                    self.app.named("Enter");
                }
                egui::Event::Key { key, pressed: true, modifiers, .. } if modifiers.is_none() => {
                    if let Some((_, name)) = MOVES.iter().find(|(k, _)| *k == key) {
                        self.app.named(name);
                    }
                }
                egui::Event::Text(t) => {
                    let mut chars = t.chars();
                    while let Some(c) = chars.next() {
                        self.app.feed(c);
                        if let Some(p) = &mut self.app.picker {
                            // Typed fast after `b`: the rest goes in the palette's box.
                            let mut rest: String = chars.by_ref().collect();
                            rest.extend(events.by_ref().filter_map(|e| match e {
                                egui::Event::Text(t) => Some(t),
                                _ => None,
                            }));
                            p.query = rest;
                            p.sync();
                            return;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// The branch or tag palette's keys: the arrows move, Enter (Shift+Enter)
    /// picks, Delete deletes, Escape closes; what is typed goes in its box.
    fn picker_keys(&mut self, ctx: &egui::Context) {
        let (enter, shift_enter, up, down, delete, esc) = ctx.input_mut(|i| {
            let shift_enter = i.consume_key(egui::Modifiers::SHIFT, egui::Key::Enter);
            let mut plain = |k| i.consume_key(egui::Modifiers::NONE, k);
            (
                plain(egui::Key::Enter),
                shift_enter,
                plain(egui::Key::ArrowUp),
                plain(egui::Key::ArrowDown),
                plain(egui::Key::Delete),
                plain(egui::Key::Escape),
            )
        });
        if esc {
            self.app.escape();
            return;
        }
        if let Some(p) = &mut self.app.picker {
            if up {
                p.move_by(-1);
            }
            if down {
                p.move_by(1);
            }
        }
        if enter || shift_enter {
            self.app.picker_enter(shift_enter);
        } else if delete {
            self.app.picker_delete();
        }
    }

    /// The palette comes with the commands (stage 6).
    fn palette(&mut self) {
        let text = self.app.tr("later").replace("{}", self.app.tr("key.palette"));
        self.app.say(text);
    }
}

impl eframe::App for Michi {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.app.mode != Mode::Settings {
            self.keys(&ctx);
        }
        self.app.prefs.pump();
        self.app.set_language();
        // common.toml's theme, or the OS's light or dark under `system`.
        self.app.prefs.apply_theme(&ctx);
        ctx.set_zoom_factor(self.app.prefs.common.scale_or_one());
        if let Some(e) = self.app.prefs.failed.take() {
            self.app.say(e);
        }
        // In front, the worktrees are watched closely; behind, now and then.
        let front = ctx.input(|i| i.viewport().focused).unwrap_or(true);
        if front != self.front {
            self.front = front;
            if let Some(r) = &self.app.reading {
                r.front(front);
            }
        }
        self.app.pump();
        self.app.sync_detail();
        if let Some(title) = self.app.new_title.take() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        }
        let out = ui::frame(ui, &mut self.app);
        if self.app.mode == Mode::Settings {
            let mut body = ui.new_child(egui::UiBuilder::new().max_rect(out.body));
            settings::show(&mut body, &mut self.app);
        }
        picker_ui::show(&ctx, &mut self.app);
        tidy_ui::show(&ctx, &mut self.app);
        // A tab leaves the settings and the key list for its view; the gear
        // opens the settings and closes them.
        if let Some(v) = out.view {
            self.app.view = v;
            self.app.mode = Mode::Main;
        }
        self.app.changes_sync();
        if out.settings {
            self.app.mode = if self.app.mode == Mode::Settings { Mode::Main } else { Mode::Settings };
        }
        if out.palette {
            self.palette();
        }
        // A click in the frame may have moved the selection: ask for it now,
        // not a frame late.
        if self.app.sync_detail() {
            ctx.request_repaint();
        }
        ui::toast(ui, &self.app, out.body);
        if self.app.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ui::resize_edges(&ctx);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        ito_theme::colors().bg.to_normalized_gamma_f32()
    }
}

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
enum Cli {
    Version,
    Help,
    /// Open a window on this folder, or on the current one.
    Open(Option<PathBuf>),
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Cli, String> {
    let mut path = None;
    for a in args {
        match a.to_str() {
            Some("--version" | "-V") => return Ok(Cli::Version),
            Some("--help" | "-h") => return Ok(Cli::Help),
            Some(s) if s.starts_with('-') && s.len() > 1 => return Err(format!("unknown argument `{s}`")),
            _ if path.is_some() => return Err(format!("one folder at a time: `{}`", a.to_string_lossy())),
            _ => path = Some(PathBuf::from(a)),
        }
    }
    Ok(Cli::Open(path))
}

/// The folder to open, made absolute so the window's thread and the title
/// do not depend on the current folder.
fn folder(path: Option<PathBuf>) -> Result<PathBuf, String> {
    let path = match path {
        Some(p) => p,
        None => std::env::current_dir().map_err(|e| format!("no current folder: {e}"))?,
    };
    let abs = std::path::absolute(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !abs.is_dir() {
        return Err(format!("not a folder: {}", abs.display()));
    }
    Ok(abs)
}

/// Puts a line where whoever typed the command is looking, on standard error
/// when `err` (kura's `say`).
///
/// A release build on Windows is a GUI-subsystem program and has no console
/// of its own. When the standard handle is a file or a pipe (`michi.com`
/// hands on its own, `cmd`'s `>`), the line goes there; otherwise michi
/// borrows the console it was started from and writes to that. A write that
/// fails is let go: PowerShell gives a bare `$v = & michi.exe --version` a
/// pipe and closes it without waiting, and `println!` panicked on that
/// ("failed printing to stdout", os error 232). `michi.com` is the way to
/// read it there.
fn say(text: &str, err: bool) {
    use std::io::Write;
    #[cfg(windows)]
    {
        if !redirected(err) && attach_console() {
            if let Ok(mut con) = std::fs::OpenOptions::new().write(true).open("CONOUT$") {
                let _ = writeln!(con, "{text}");
                return;
            }
        }
    }
    let _ = match err {
        true => writeln!(std::io::stderr().lock(), "{text}"),
        false => writeln!(std::io::stdout().lock(), "{text}"),
    };
}

/// Whether standard output (standard error when `err`) is a file or a pipe.
#[cfg(windows)]
fn redirected(err: bool) -> bool {
    use windows_sys::Win32::Storage::FileSystem::{GetFileType, FILE_TYPE_DISK, FILE_TYPE_PIPE};
    use windows_sys::Win32::System::Console::{GetStdHandle, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE};

    // SAFETY: this process's own standard handle; a missing one is null, which
    // GetFileType answers as unknown.
    let kind = unsafe { GetFileType(GetStdHandle(if err { STD_ERROR_HANDLE } else { STD_OUTPUT_HANDLE })) };
    kind == FILE_TYPE_DISK || kind == FILE_TYPE_PIPE
}

/// Borrows the console michi was started from. False when there is none
/// (started from Explorer: nobody asked, so the text goes nowhere) or it has
/// one already (a debug build, a console program).
#[cfg(windows)]
fn attach_console() -> bool {
    use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};

    // SAFETY: no arguments to get wrong.
    unsafe { AttachConsole(ATTACH_PARENT_PROCESS) != 0 }
}

fn main() -> eframe::Result {
    let path = match parse_args(std::env::args_os().skip(1)) {
        Ok(Cli::Version) => {
            say(&format!("michi {}", env!("CARGO_PKG_VERSION")), false);
            return Ok(());
        }
        Ok(Cli::Help) => {
            say(USAGE, false);
            return Ok(());
        }
        Ok(Cli::Open(path)) => path,
        Err(e) => {
            say(&format!("michi: {e}\n\n{USAGE}"), true);
            std::process::exit(2);
        }
    };
    let dir = match folder(path) {
        Ok(d) => d,
        Err(e) => {
            say(&format!("michi: {e}"), true);
            std::process::exit(2);
        }
    };
    let viewport = egui::ViewportBuilder::default()
        .with_title("michi")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([720.0, 420.0])
        // The frame is drawn by ui.rs: the title bar, the buttons, the edges.
        .with_decorations(false)
        // Alt+Tab and the taskbar; build.rs bakes the same drawing into
        // michi.exe for Explorer.
        .with_icon(std::sync::Arc::new(egui::IconData { rgba: icon::pixels(ICON_PX), width: ICON_PX, height: ICON_PX }));
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(
        "michi",
        options,
        Box::new(move |cc| {
            fonts::load_in_background(cc.egui_ctx.clone());
            let mut prefs = prefs::Prefs::load(ito_common::base_dir());
            prefs.watch(&cc.egui_ctx);
            prefs.apply_theme(&cc.egui_ctx);
            // The scale is common.toml's, so egui's own Ctrl+= / Ctrl+- stay out of it.
            cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
            cc.egui_ctx.set_zoom_factor(prefs.common.scale_or_one());
            let mut app = App::new(prefs, dir.clone());
            app.reading = Some(repo::start(dir, cc.egui_ctx.clone()));
            Ok(Box::new(Michi { app, front: true }))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Cli, String> {
        parse_args(args.iter().map(OsString::from))
    }

    #[test]
    fn the_command_line() {
        assert_eq!(parse(&[]), Ok(Cli::Open(None)));
        assert_eq!(parse(&["C:/dev/x"]), Ok(Cli::Open(Some(PathBuf::from("C:/dev/x")))));
        assert_eq!(parse(&["--version"]), Ok(Cli::Version));
        assert_eq!(parse(&["-V"]), Ok(Cli::Version));
        assert_eq!(parse(&["x", "-h"]), Ok(Cli::Help));
        assert_eq!(parse(&["-"]), Ok(Cli::Open(Some(PathBuf::from("-")))));
        assert!(parse(&["--nope"]).unwrap_err().contains("--nope"));
        assert!(parse(&["a", "b"]).unwrap_err().contains("`b`"));
    }

    #[test]
    fn a_folder_must_be_one() {
        let here = folder(None).unwrap();
        assert!(here.is_absolute());
        let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        assert!(folder(Some(file)).unwrap_err().starts_with("not a folder"));
        assert!(folder(Some(PathBuf::from("no/such/folder"))).is_err());
    }
}
