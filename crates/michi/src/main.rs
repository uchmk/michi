//! michi: a keyboard-driven Git client for a repository many Claude Code
//! sessions work in at once, the sibling of kura, yagura, tsumugi and
//! minato. The window and the input; the state is in `app.rs`, the drawing
//! in `ui.rs` and `settings.rs`, and what has nothing to do with egui in the
//! `michi-core` crate.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod act;
mod app;
mod bugreport;
mod changes;
mod changes_ui;
mod cmdpalette;
mod conflict_ui;
mod conflicts;
mod detail;
mod detail_ui;
mod fonts;
mod fswatch;
mod graph;
mod graph_ui;
mod i18n;
mod icon;
mod keymap;
mod newwork;
mod newwork_ui;
mod notify;
mod picker;
mod picker_ui;
mod prefs;
mod reorder;
mod reorder_ui;
mod repo;
mod review;
mod review_notes;
mod review_ui;
mod script;
mod settings;
mod side_ui;
mod sizes;
mod tidy;
mod tidy_ui;
mod ui;
mod work;
mod wt_ui;

use std::ffi::OsString;
use std::path::PathBuf;

use app::{App, Mode, View};
use ito_keyscript::{Do, Runner, Step};

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

usage: michi [<path>] [--keys SCRIPT] [--version | --help]

  <path>         a folder in the repository to open (default: the current folder)
  --keys SCRIPT  press these keys once the window is up, as kura's --keys
                 (`jj<Enter>`, `<C-S-p>`, `<Wait:200>`, `<State:name>`, `<Quit>`);
                 MICHI_KEYS_DONE names the file that says how it ended
";

struct Michi {
    app: App,
    /// Whether the window was in front at the last frame.
    front: bool,
    script: Option<Runner>,
    /// Writes the window's place to window.toml; none under `--keys`.
    place: Option<ito_common::window::Tracker>,
    /// What the script asked for in `raw_input_hook`, done in the frame.
    todo: Vec<Do>,
    /// The keymap.toml / config.toml warnings already said in a toast.
    warned: Vec<String>,
}

impl Michi {
    fn write_state(&self, path: PathBuf, tail: &str) {
        let text = format!("{}{tail}", self.app.report());
        if let Err(e) = std::fs::write(&path, text) {
            ito_console::err(&format!(
                "michi --keys: {}: {e}
",
                path.display()
            ));
        }
    }

    fn script(&mut self, ctx: &egui::Context) {
        let Some(script) = &mut self.script else { return };
        if let Some(err) = script.shot(ctx) {
            self.app.say(err);
        }
        for d in std::mem::take(&mut self.todo) {
            let Some(script) = &self.script else { return };
            match d {
                Do::State(name) => self.write_state(script.dir.join(format!("{name}.txt")), ""),
                Do::Quit => self.app.quit = true,
                // michi defines no readings of its own.
                Do::Read { .. } => {}
                Do::Done => {
                    if let Some(done) = script.done.clone() {
                        self.write_state(
                            done,
                            "keys: done
",
                        );
                    }
                }
            }
        }
        if let Some(due) = self.script.as_ref().and_then(Runner::due) {
            ctx.request_repaint_after(due);
        }
    }

    /// The keys the keymap binds (the settings, the key list, the palette,
    /// quit, the scale), taken out of the frame's events before the views
    /// see them. Not while the Keys page is taking keys.
    fn keymap_keys(&mut self, ctx: &egui::Context) {
        if self.app.prefs.keys_edit.busy() || self.app.cmd.is_some() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        let mut used = Vec::new();
        for (n, e) in events.iter().enumerate() {
            let egui::Event::Key { key, pressed: true, modifiers, .. } = e else { continue };
            let Some(k) = ito_keys::from_egui(*key, modifiers) else { continue };
            if self.app.key(k) {
                used.push(n);
            }
        }
        if !used.is_empty() {
            ctx.input_mut(|i| {
                let mut n = 0;
                i.events.retain(|_| {
                    n += 1;
                    !used.contains(&(n - 1))
                });
            });
        }
    }

    /// The characters typed and the named keys. While the graph's filter has
    /// the keys, only Enter (the next hit) and the arrows (the selection)
    /// are taken from it; Escape has already let go of it. While the commit
    /// message has them, only Ctrl+Enter (commit) is taken; the Escape that
    /// let go of it goes no further.
    fn keys(&mut self, ctx: &egui::Context) {
        if self.app.cmd.is_some() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        if self.app.picker.is_some() {
            self.picker_keys(ctx);
            return;
        }
        if self.app.new_work.is_some() {
            let (start, esc, up, down) = ctx.input_mut(|i| {
                (
                    i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp),
                    i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown),
                )
            });
            if start {
                self.app.new_work_start();
            } else if esc {
                self.app.escape();
            } else if let Some(n) = &mut self.app.new_work {
                if up {
                    n.cycle(-1);
                }
                if down {
                    n.cycle(1);
                }
            }
            return;
        }
        // A message written for a commit of the rebase screen.
        if self.app.band.is_none() && self.app.reorder.as_ref().is_some_and(|r| r.editing.is_some()) {
            let (keep, leave) = ctx.input_mut(|i| {
                (i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter), i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
            });
            if let Some(r) = &mut self.app.reorder {
                if keep {
                    r.edited();
                } else if leave {
                    r.editing = None;
                }
            }
            return;
        }
        // A note written for a line of the branch review.
        if self.app.band.is_none() && self.app.review.as_ref().is_some_and(|r| r.editing.is_some()) {
            let (keep, leave) = ctx.input_mut(|i| {
                (i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter), i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
            });
            if let Some(r) = &mut self.app.review {
                if keep {
                    r.noted();
                } else if leave {
                    r.editing = None;
                }
            }
            return;
        }
        // A result written by hand in the 4 view takes the keys until it is
        // solved with (Ctrl+Enter) or left (Esc).
        if self.app.conflicts.edit.is_some() {
            let (apply, leave) = ctx.input_mut(|i| {
                (i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter), i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
            });
            if apply {
                self.app.conflict_edit_apply();
            } else if leave {
                self.app.conflict_edit_cancel();
            }
            return;
        }
        let typing = ctx.memory(|m| m.has_focus(changes_ui::message_id()));
        let commit =
            (typing || self.app.view == View::Changes) && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter));
        if commit {
            self.app.changes_commit();
        }
        // The same in the 4 view, for what stopped on a clash.
        let concluding = ctx.memory(|m| m.has_focus(conflict_ui::message_id()));
        let conclude =
            (concluding || self.app.view == View::Conflict) && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter));
        if conclude {
            self.app.conflict_commit();
        }
        if typing || concluding {
            return;
        }
        let let_go = std::mem::take(&mut self.app.changes.typing) | std::mem::take(&mut self.app.conflicts.typing);
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
        let (enter, shift_enter, up, down, delete, esc, rename, upstream, rebase) = ctx.input_mut(|i| {
            let shift_enter = i.consume_key(egui::Modifiers::SHIFT, egui::Key::Enter);
            let mut plain = |k| i.consume_key(egui::Modifiers::NONE, k);
            (
                plain(egui::Key::Enter),
                shift_enter,
                plain(egui::Key::ArrowUp),
                plain(egui::Key::ArrowDown),
                plain(egui::Key::Delete),
                plain(egui::Key::Escape),
                plain(egui::Key::F2),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::U),
                i.consume_key(egui::Modifiers::COMMAND, egui::Key::R),
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
        } else if rename {
            self.app.picker_rename();
        } else if upstream {
            self.app.picker_upstream();
        } else if rebase {
            self.app.picker_rebase();
        }
    }
}

impl eframe::App for Michi {
    /// A window that closes while a script runs says so in the done file
    /// (`<Quit>` and a last key that quits both come here).
    fn on_exit(&mut self) {
        if let Some(t) = &mut self.place {
            t.close(None);
        }
        let Some(script) = &mut self.script else { return };
        if !script.running() {
            return;
        }
        let left = script.left();
        script.finish();
        let Some(done) = script.done.clone() else { return };
        let tail = if self.app.quit { ito_keyscript::quit_report(left) } else { ito_keyscript::closed_report(left) };
        self.write_state(done, &tail);
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        if let Some(script) = &mut self.script {
            if let Some(d) = script.input(ctx, raw) {
                self.todo.push(d);
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some(t) = &mut self.place {
            t.follow(&ctx);
        }
        self.keymap_keys(&ctx);
        if self.app.mode != Mode::Settings {
            self.keys(&ctx);
        }
        self.script(&ctx);
        self.app.prefs.pump();
        self.app.set_language();
        // common.toml's theme, or the OS's light or dark under `system`.
        self.app.prefs.apply_theme(&ctx);
        ctx.set_zoom_factor(self.app.prefs.common.scale_or_one());
        if let Some(e) = self.app.prefs.failed.take() {
            self.app.say(e);
        }
        if let Some(n) = self.app.prefs.said.take().filter(|n| *n > 0) {
            let text = self.app.tr("toast.keymap_saved").replace("{}", &n.to_string());
            self.app.say(text);
        }
        // What keymap.toml and config.toml could not be read as, said once each time it changes.
        let warn: Vec<String> = [&self.app.prefs.keymap_warn, &self.app.prefs.notify_warn].into_iter().flatten().cloned().collect();
        if warn != self.warned {
            if let Some(first) = warn.first().filter(|_| !warn.iter().all(|w| self.warned.contains(w))) {
                let text = self.app.tr("toast.config_warn").replace("{}", &warn.len().to_string()).replace("{1}", first);
                self.app.say(text);
            }
            self.warned = warn;
        }
        // In front, the worktrees are watched closely; behind, now and then.
        let front = ctx.input(|i| i.viewport().focused).unwrap_or(true);
        self.app.front = front;
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
        newwork_ui::show(&ctx, &mut self.app);
        tidy_ui::show(&ctx, &mut self.app);
        reorder_ui::show(&ctx, &mut self.app);
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
            self.app.toggle_palette();
        }
        if let Some(p) = cmdpalette::show(&ctx, &mut self.app) {
            self.app.run_pick(p);
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
    Open(Option<PathBuf>, Option<Vec<Step>>),
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Cli, String> {
    let mut path = None;
    let mut script = None;
    let mut args = args.into_iter();
    while let Some(a) = args.next() {
        match a.to_str() {
            Some("--keys") => {
                let s = args.next().ok_or("--keys needs a script")?;
                let s = s.to_str().ok_or("--keys: the script is not text")?;
                script = Some(ito_keyscript::parse(s).map_err(|e| format!("--keys: {e}"))?);
            }
            Some("--version" | "-V") => return Ok(Cli::Version),
            Some("--help" | "-h") => return Ok(Cli::Help),
            Some(s) if s.starts_with('-') && s.len() > 1 => return Err(format!("unknown argument `{s}`")),
            _ if path.is_some() => return Err(format!("one folder at a time: `{}`", a.to_string_lossy())),
            _ => path = Some(PathBuf::from(a)),
        }
    }
    Ok(Cli::Open(path, script))
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
/// when `err` (`ito_console::out` and `err`, as kura's `say`).
///
/// A release build on Windows is a GUI-subsystem program and has no console
/// of its own. When the standard handle is a file or a pipe (`michi.com`
/// hands on its own, `cmd`'s `>`), the line goes there; otherwise it goes to
/// the console michi was started from. A write that fails is let go:
/// PowerShell gives a bare `$v = & michi.exe --version` a pipe and closes it
/// without waiting, and `println!` panicked on that ("failed printing to
/// stdout", os error 232). `michi.com` is the way to read it there.
fn say(text: &str, err: bool) {
    let line = format!(
        "{text}
"
    );
    match err {
        true => ito_console::err(&line),
        false => ito_console::out(&line),
    }
}

fn main() -> eframe::Result {
    let (path, script) = match parse_args(std::env::args_os().skip(1)) {
        Ok(Cli::Version) => {
            say(&format!("michi {}", env!("CARGO_PKG_VERSION")), false);
            return Ok(());
        }
        Ok(Cli::Help) => {
            say(USAGE, false);
            return Ok(());
        }
        Ok(Cli::Open(path, script)) => (path, script),
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
    let done = std::env::var_os("MICHI_KEYS_DONE").filter(|v| !v.is_empty()).map(PathBuf::from);
    if let (Some(_), Some(done)) = (&script, &done) {
        ito_keyscript::install_panic_report(done.clone());
    }
    // A script plays on a window that keeps no place, and leaves none.
    let saved =
        ito_common::window::Saved::load(script.is_none().then(|| crate::prefs::window_dir(ito_common::base_dir().as_deref())).flatten());
    let viewport = egui::ViewportBuilder::default()
        .with_title("michi")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([720.0, 420.0])
        // The frame is drawn by ui.rs: the title bar, the buttons, the edges.
        .with_decorations(false)
        // Alt+Tab and the taskbar; build.rs bakes the same drawing into
        // michi.exe for Explorer.
        .with_icon(std::sync::Arc::new(egui::IconData { rgba: icon::pixels(ICON_PX), width: ICON_PX, height: ICON_PX }));
    let viewport = saved.viewport(viewport);
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(
        "michi",
        options,
        Box::new(move |cc| {
            saved.put(cc);
            fonts::load_in_background(cc.egui_ctx.clone());
            let mut prefs = prefs::Prefs::load(ito_common::base_dir());
            prefs.watch(&cc.egui_ctx);
            prefs.apply_theme(&cc.egui_ctx);
            // The scale is common.toml's, so egui's own Ctrl+= / Ctrl+- stay out of it.
            cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
            cc.egui_ctx.set_zoom_factor(prefs.common.scale_or_one());
            let mut app = App::new(prefs, dir.clone());
            app.reading = Some(repo::start(dir, cc.egui_ctx.clone()));
            let script = script.map(|steps| {
                let mut r = Runner::new(steps, done);
                r.watch(cc.egui_ctx.clone());
                r
            });
            Ok(Box::new(Michi { app, front: true, script, place: saved.tracker(), todo: Vec::new(), warned: Vec::new() }))
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
        assert_eq!(parse(&[]), Ok(Cli::Open(None, None)));
        assert_eq!(parse(&["C:/dev/x"]), Ok(Cli::Open(Some(PathBuf::from("C:/dev/x")), None)));
        assert_eq!(parse(&["--version"]), Ok(Cli::Version));
        assert_eq!(parse(&["-V"]), Ok(Cli::Version));
        assert_eq!(parse(&["x", "-h"]), Ok(Cli::Help));
        assert_eq!(parse(&["-"]), Ok(Cli::Open(Some(PathBuf::from("-")), None)));
        assert!(parse(&["--nope"]).unwrap_err().contains("--nope"));
        assert!(parse(&["a", "b"]).unwrap_err().contains("`b`"));
    }

    #[test]
    fn a_script_is_a_value_of_keys() {
        let Ok(Cli::Open(path, Some(steps))) = parse(&["repo", "--keys", "j<Enter>"]) else { panic!("no script") };
        assert_eq!(path, Some(PathBuf::from("repo")));
        assert_eq!(steps.len(), 2);
        // `-x` after `--keys` is the script's text, not an option.
        assert!(matches!(parse(&["--keys", "-x"]), Ok(Cli::Open(None, Some(_)))));
        assert!(parse(&["--keys"]).unwrap_err().contains("needs a script"));
        assert!(parse(&["--keys", "<Nope>"]).unwrap_err().starts_with("--keys:"));
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
