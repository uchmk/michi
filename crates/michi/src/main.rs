//! michi: a keyboard-driven Git client for a repository many Claude Code
//! sessions work in at once, the sibling of kura, yagura, tsumugi and
//! minato. The window and the input; the state is in `app.rs`, the drawing
//! in `ui.rs` and `settings.rs`, and what has nothing to do with egui in the
//! `michi-core` crate.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

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
mod prefs;
mod repo;
mod review;
mod review_ui;
mod settings;
mod side_ui;
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
        for e in events {
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
                    for c in t.chars() {
                        self.app.feed(c);
                    }
                }
                _ => {}
            }
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

/// A release build on Windows has no console of its own: borrow the one it
/// was started from, so `--help` and `--version` can be read.
#[cfg(windows)]
fn attach_console() {
    // SAFETY: no arguments to get wrong; failing (no parent console) is fine.
    unsafe extern "system" {
        fn AttachConsole(process: u32) -> i32;
    }
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    let _ = unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
}

#[cfg(not(windows))]
fn attach_console() {}

fn main() -> eframe::Result {
    let path = match parse_args(std::env::args_os().skip(1)) {
        Ok(Cli::Version) => {
            attach_console();
            println!("michi {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        Ok(Cli::Help) => {
            attach_console();
            println!("{USAGE}");
            return Ok(());
        }
        Ok(Cli::Open(path)) => path,
        Err(e) => {
            attach_console();
            eprintln!("michi: {e}\n\n{USAGE}");
            std::process::exit(2);
        }
    };
    let dir = match folder(path) {
        Ok(d) => d,
        Err(e) => {
            attach_console();
            eprintln!("michi: {e}");
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
