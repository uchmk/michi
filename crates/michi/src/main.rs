//! michi: a keyboard-driven Git client for a repository many Claude Code
//! sessions work in at once, the sibling of kura, yagura, tsumugi and
//! minato. The window and the input; the state is in `app.rs`, the drawing
//! in `ui.rs` and `settings.rs`, and what has nothing to do with egui in the
//! `michi-core` crate.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod act;
mod app;
mod auth;
mod auth_ui;
mod avatar;
mod bisect;
mod blame;
mod blame_ui;
mod bugreport;
mod calm;
mod changes;
mod changes_ui;
mod cheat_ui;
mod cmdpalette;
mod conflict_ui;
mod conflicts;
mod ctxmenu;
mod detail;
mod detail_ui;
mod dnd_ui;
mod fonts;
mod fswatch;
mod graph;
mod graph_ui;
mod guide_ui;
mod i18n;
mod icon;
mod images;
mod intro_ui;
mod issues;
mod keymap;
mod land;
mod legend_ui;
mod mcp;
mod motion;
mod newwork;
mod newwork_ui;
mod notify;
mod panes;
mod patch;
mod picker;
mod picker_ui;
mod plain;
mod prefix;
mod prefs;
mod prform;
mod prform_ui;
mod prwatch;
mod ready;
mod ready_ui;
mod reflog;
mod reflog_ui;
mod reorder;
mod reorder_ui;
mod reply;
mod reply_ui;
mod repo;
mod repos;
mod review;
mod review_notes;
mod review_ui;
mod script;
mod select;
mod settings;
mod side_ui;
mod sizes;
mod stats;
mod stats_ui;
mod strip;
mod submodule;
mod terms;
mod tidy;
mod tidy_ui;
mod trial;
mod trial_ui;
mod turns;
mod ui;
mod viewcfg;
mod work;
mod wt_ui;

use std::ffi::OsString;
use std::path::PathBuf;

use app::{App, Mode, Repo, View};
use ito_keyscript::{Do, Runner, Step};
use michi_core::tabs::{self, Op, Tabs};

/// The named keys that move in a view, by the names `App::named` takes.
const MOVES: [(egui::Key, &str); 9] = [
    (egui::Key::Backspace, "Backspace"),
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
       michi mcp

  <path>         a folder in the repository to open (default: the current folder)
  --keys SCRIPT  press these keys once the window is up, as kura's --keys
                 (`jj<Enter>`, `<C-S-p>`, `<Wait:200>`, `<State:name>`, `<Quit>`);
                 MICHI_KEYS_DONE names the file that says how it ended
  mcp            serve MCP on standard input and output, for Claude Code
                 to read the worktrees and conflicts of the running window
";

struct Michi {
    app: App,
    /// Whether the window was in front at the last frame.
    front: bool,
    /// The monospace face the fonts were last built for.
    mono: ito_fonts::MonoFont,
    script: Option<Runner>,
    /// Writes the window's place to window.toml; none under `--keys`.
    place: Option<ito_common::window::Tracker>,
    /// What the script asked for in `raw_input_hook`, done in the frame.
    todo: Vec<Do>,
    /// Tab presses (`true`: with Shift) taken out of the input before egui could move
    /// its focus to the next widget (the graph's filter box); `keys` hands them to the app.
    tab_stash: Vec<bool>,
    /// The keymap.toml / config.toml warnings already said in a toast.
    warned: Vec<String>,
    /// Requests from `michi mcp`; none under `--keys`.
    mcp: Option<std::sync::mpsc::Receiver<mcp::Ask>>,
    /// The window's tabs (#45): one repository each. The one in front is
    /// `app`; the others wait in `parked`, by the same index, as the whole
    /// state of their view, or as `None` until first shown (restored tabs).
    tabs: Tabs,
    parked: Vec<Option<App>>,
    /// The tabs as last kept in tabs.txt; none to keep under `--keys`.
    tabs_kept: Option<String>,
}

impl Michi {
    /// Answers what `michi mcp` asked, from the state in memory.
    fn answer_mcp(&mut self) {
        let Some(rx) = &self.mcp else { return };
        while let Ok(ask) = rx.try_recv() {
            let _ = ask.reply.send(Ok(mcp::answer_json(&self.app, ask.request)));
        }
    }

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

    /// A folder the repository screen picked is read on a thread; when it
    /// is a repository the window starts over on it, else it stays where it
    /// is and says why.
    fn switch(&mut self, ctx: &egui::Context) {
        if let Some(done) = self.app.cloning.as_ref().and_then(|c| c.answer.try_recv().ok()) {
            if let Some(c) = self.app.cloning.take() {
                match done {
                    Ok(()) => {
                        let text = self.app.tr("repos.cloned").replace("{}", &c.dest.display().to_string());
                        self.app.say_for(text, 6);
                        self.app.open_req = Some(c.dest);
                    }
                    Err(e) => {
                        let mut text = self.app.tr("repos.clone_failed").replace("{}", &e.to_string());
                        text.push_str(&self.app.auth_noted(&e, Some(c.url.clone())));
                        self.app.say_for(text, 10);
                    }
                }
            }
        }
        if let Some(rx) = &self.app.practicing {
            match rx.try_recv() {
                Ok(Ok(dir)) => {
                    self.app.practicing = None;
                    let text = self.app.tr("practice.done").replace("{}", &dir.display().to_string());
                    self.app.say_for(text, 8);
                    self.app.open_req = Some(dir);
                }
                Ok(Err(e)) => {
                    self.app.practicing = None;
                    let text = self.app.tr("practice.failed").replace("{}", &e);
                    self.app.say_for(text, 10);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => ctx.request_repaint_after(std::time::Duration::from_millis(100)),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.app.practicing = None,
            }
        }
        if let Some(dir) = self.app.open_req.take() {
            self.app.opening = Some(repos::start_opening(ctx, dir));
        }
        let Some(found) = self.app.opening.as_ref().and_then(|o| o.answer.try_recv().ok()) else { return };
        let Some(o) = self.app.opening.take() else { return };
        match found {
            Ok(s) => {
                // The folder opens in a tab of its own, or its tab comes to the front.
                let old = self.tabs.active();
                match self.tabs.open(&s.top) {
                    Some(made) => {
                        if made {
                            self.parked.insert(self.tabs.active(), None);
                        }
                        if self.tabs.active() != old {
                            self.show_tab(ctx, Some(old));
                        }
                    }
                    None => {
                        let text = self.app.tr("tabs.full").replace("{}", &tabs::MAX.to_string());
                        self.app.say_for(text, 6);
                    }
                }
            }
            Err(e) => {
                let word = self.app.tr(app::failed_word(&e));
                let text = format!("{word} — {}", o.dir.display());
                self.app.say_for(text, 10);
            }
        }
    }

    /// Puts the window on the tab in front of `self.tabs`: `app` (the tab
    /// `parked_at` had, if it still has one) goes to its place in `parked`,
    /// and the front tab's state comes out, or is started on its folder.
    /// The settings and the recent list are the window's, not a tab's, so
    /// they go across.
    fn show_tab(&mut self, ctx: &egui::Context, parked_at: Option<usize>) {
        let new = self.tabs.active();
        let mut cur = std::mem::replace(&mut self.app, App::new(prefs::Prefs::empty(), PathBuf::new()));
        let prefs = std::mem::replace(&mut cur.prefs, prefs::Prefs::empty());
        let recent = std::mem::take(&mut cur.recent);
        let sizes = cur.sizes.clone();
        let front = cur.front;
        // Behind the front, a tab's worktrees are read now and then.
        if let Some(r) = &cur.reading {
            r.front(false);
        }
        cur.repos = None;
        cur.tab_req = None;
        if let Some(i) = parked_at {
            self.parked[i] = Some(cur);
        }
        let mut next = match self.parked[new].take() {
            Some(mut a) => {
                a.prefs = prefs;
                a
            }
            None => {
                let dir = self.tabs.paths()[new].clone();
                let mut a = App::with_window(prefs, dir.clone(), sizes.clone(), recent.clone());
                a.reading = Some(repo::start(dir, ctx.clone()));
                a
            }
        };
        next.recent = recent;
        // The dragged sizes are the window's too; no file is read for a tab.
        next.sizes = sizes;
        next.front = front;
        if let Some(r) = &next.reading {
            r.front(front);
        }
        if let Repo::Ready(s) = &next.repo {
            next.new_title = Some(format!("{} — michi", s.name));
        }
        self.app = next;
        ctx.request_repaint();
    }

    /// What a key or a click asked of the tabs.
    fn tab_step(&mut self, ctx: &egui::Context) {
        let Some(op) = self.app.tab_req.take() else { return };
        let old = self.tabs.active();
        let closing = match op {
            Op::Close => Some(old),
            Op::CloseAt(i) => Some(i),
            _ => None,
        };
        if let Some(i) = closing {
            if !self.tabs.close(i) {
                if self.tabs.len() < 2 {
                    let text = self.app.tr("tabs.last");
                    self.app.say(text);
                }
                return;
            }
            // The tab in front is dropped with its state; another one is
            // dropped from `parked` and the front one stays (at its new index).
            let parked_at = (i != old).then(|| if i < old { old - 1 } else { old });
            drop(self.parked.remove(i));
            // With no place to go to, `app` (the closed one) is dropped.
            self.show_tab(ctx, parked_at);
            return;
        }
        match op {
            Op::Next => self.tabs.next(),
            Op::Prev => self.tabs.prev(),
            Op::Goto(i) => {
                self.tabs.activate(i);
            }
            Op::Close | Op::CloseAt(_) => {}
        }
        if self.tabs.active() != old {
            self.show_tab(ctx, Some(old));
        }
    }

    /// The front tab's folder as the repository's top once that is read, the
    /// names for the bar, and the tabs kept in tabs.txt when they changed.
    fn sync_tabs(&mut self) {
        let at = self.tabs.active();
        if let Repo::Ready(s) = &self.app.repo {
            if !michi_core::paths::same_path(&self.tabs.paths()[at], &s.top) && self.tabs.index_of(&s.top).is_none() {
                self.tabs.set_path(at, &s.top);
            }
        }
        self.app.tab_labels = self.tabs.labels();
        self.app.tab_active = at;
        let Some(kept) = &mut self.tabs_kept else { return };
        // The conflict view is only there while something clashed, so it is not kept.
        let view = if self.app.view == View::Conflict { View::Graph } else { self.app.view };
        let text = self.tabs.text_with_view(view.digit());
        if *kept == text {
            return;
        }
        if let Some(file) = prefs::window_dir(self.app.prefs.base.as_deref()).map(|d| d.join("tabs.txt")) {
            let text = text.clone();
            std::thread::spawn(move || {
                if let Some(dir) = file.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                let _ = ito_common::write_atomic(&file, text);
            });
        }
        *kept = text;
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
        if self.app.prefs.keys_edit.busy() || self.app.cmd.is_some() || self.app.repos.is_some() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        let mut used = Vec::new();
        for (n, e) in events.iter().enumerate() {
            let egui::Event::Key { key, pressed: true, modifiers, .. } = e else { continue };
            let Some(k) = ito_keys::from_egui(*key, modifiers) else { continue };
            // Ctrl+Z in a text field undoes the typing, not a branch (#64).
            if matches!(self.app.prefs.keymap.act(k), Some(crate::keymap::Act::Undo | crate::keymap::Act::Redo))
                && ctx.egui_wants_keyboard_input()
            {
                continue;
            }
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
        // A right-click menu is open: its own Esc and clicks close it.
        if self.app.cmd.is_some() || self.app.repos.is_some() || egui::Popup::is_any_open(ctx) {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        // The branch, tag, stash and remote palette takes its own keys (ito-palette).
        if self.app.picker.is_some() {
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
        // Ctrl+Enter goes ahead with the question in the band, as it does for a commit or a post.
        if self.app.cheat.is_none()
            && self.app.band.as_ref().is_some_and(|b| !b.typing() && b.parents == 0)
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter))
        {
            self.app.band_key('y');
            return;
        }
        if self.app.pr_form.is_some() {
            let (go, esc) = ctx.input_mut(|i| {
                (i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter), i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
            });
            if go {
                self.app.pr_submit();
            } else if esc {
                self.app.escape();
            }
            return;
        }
        if self.app.reply.is_some() {
            let (send, esc) = ctx.input_mut(|i| {
                let enter =
                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter) | i.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter);
                (enter, i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
            });
            if send {
                self.app.reply_send();
            } else if esc {
                self.app.escape();
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
        // Alt+Up and Alt+Down in the box: the messages of earlier commits.
        if typing {
            let (older, newer) = ctx.input_mut(|i| {
                (i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowUp), i.consume_key(egui::Modifiers::ALT, egui::Key::ArrowDown))
            });
            if older || newer {
                self.app.changes.step_history(older);
            }
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
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)) {
                self.app.graph.cycle_deep();
            }
            if (next || back) && !self.app.ask_content() {
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
        // Alt+1.. is a repository tab (the keymap); the digit it types goes nowhere (AltGr is Ctrl+Alt: that types).
        let alt_held = ctx.input(|i| i.modifiers.alt && !i.modifiers.ctrl);
        for shift in std::mem::take(&mut self.tab_stash) {
            self.app.named(if shift { "Shift+Tab" } else { "Tab" });
        }
        let mut events = events.into_iter();
        while let Some(e) = events.next() {
            match e {
                // egui hands Ctrl+C over as this, not as a key.
                egui::Event::Copy => {
                    if let Some(text) = select::text() {
                        ctx.copy_text(text);
                    } else if self.app.view == View::Graph && self.app.mode == Mode::Main {
                        self.app.copy_hash();
                    }
                }
                egui::Event::Paste(text) => {
                    self.app.pasted(&text);
                }
                egui::Event::Key { key: egui::Key::Escape, pressed: true, .. } if !let_go => {
                    self.app.escape();
                }
                // Tab and Shift+Tab change the sheet of the Git flow sheet, or the pane of the graph view
                // (the presses `raw_input_hook` took out of the input are done before this loop).
                egui::Event::Key { key: egui::Key::Tab, pressed: true, modifiers, .. }
                    if (self.app.cheat.is_some() || self.app.band.is_some() || self.app.tab_ready())
                        && !modifiers.command
                        && !modifiers.alt =>
                {
                    self.app.named(if modifiers.shift { "Shift+Tab" } else { "Tab" });
                }
                egui::Event::Key { key: egui::Key::Enter, pressed: true, modifiers, .. } if modifiers.is_none() => {
                    self.app.named("Enter");
                }
                egui::Event::Key { key: key @ (egui::Key::ArrowDown | egui::Key::ArrowUp), pressed: true, modifiers, .. }
                    if modifiers.shift && !modifiers.command && !modifiers.alt =>
                {
                    self.app.named(if key == egui::Key::ArrowDown { "Shift+ArrowDown" } else { "Shift+ArrowUp" });
                }
                // Alt+Left / Alt+Right move between the panes, as in tsumugi.
                egui::Event::Key { key: key @ (egui::Key::ArrowLeft | egui::Key::ArrowRight), pressed: true, modifiers, .. }
                    if modifiers.alt && !modifiers.command && !modifiers.shift =>
                {
                    self.app.named(if key == egui::Key::ArrowLeft { "Alt+ArrowLeft" } else { "Alt+ArrowRight" });
                }
                // Ctrl+D / Ctrl+U, Ctrl+F / Ctrl+B and Alt+J / Alt+K are PageDown / PageUp.
                egui::Event::Key {
                    key: key @ (egui::Key::D | egui::Key::U | egui::Key::F | egui::Key::B | egui::Key::J | egui::Key::K),
                    pressed: true,
                    modifiers,
                    ..
                } if !modifiers.shift
                    && ((modifiers.ctrl && !modifiers.alt && matches!(key, egui::Key::D | egui::Key::U | egui::Key::F | egui::Key::B))
                        || (modifiers.alt && !modifiers.command && matches!(key, egui::Key::J | egui::Key::K))) =>
                {
                    self.app.named(if matches!(key, egui::Key::D | egui::Key::F | egui::Key::J) { "PageDown" } else { "PageUp" });
                }
                egui::Event::Key { key, pressed: true, modifiers, .. } if modifiers.is_none() => {
                    if let Some((_, name)) = MOVES.iter().find(|(k, _)| *k == key) {
                        self.app.named(name);
                    }
                }
                egui::Event::Text(_) if alt_held => {}
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
        // egui turns Tab into "focus the next widget" before the frame runs, which sent the
        // keys to the filter box. Where Tab moves the panes, the app takes it and egui never sees it.
        if (self.app.cheat.is_some() || self.app.band.is_some() || self.app.tab_ready()) && ctx.memory(|m| m.focused().is_none()) {
            let stash = &mut self.tab_stash;
            raw.events.retain(|e| match e {
                egui::Event::Key { key: egui::Key::Tab, pressed, modifiers, .. } if !modifiers.command && !modifiers.alt => {
                    if *pressed {
                        stash.push(modifiers.shift);
                    }
                    false
                }
                _ => true,
            });
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some(t) = &mut self.place {
            t.follow(&ctx);
        }
        self.switch(&ctx);
        self.tab_step(&ctx);
        self.answer_mcp();
        self.keymap_keys(&ctx);
        if self.app.mode != Mode::Settings {
            self.keys(&ctx);
        }
        // Whatever `keys` returned before reaching is dropped, not kept for a later frame.
        self.tab_stash.clear();
        self.script(&ctx);
        notify::set_window(&ctx);
        self.app.prefs.pump();
        self.app.detail.unified = !self.app.prefs.diff_side;
        self.app.changes.unified = !self.app.prefs.diff_side;
        if let Some(r) = self.app.review.as_mut() {
            r.unified = !self.app.prefs.diff_side;
        }
        self.app.set_language();
        // common.toml's theme, or the OS's light or dark under `system`.
        self.app.prefs.apply_theme(&ctx);
        motion::apply(&ctx, self.app.prefs.motion);
        ctx.set_zoom_factor(self.app.prefs.common.scale_or_one());
        let full = (self.app.prefs.dates == michi_core::dates::Dates::Full).then(|| {
            let clock = ito_common::Clock { date: true, weekday: false, ..self.app.prefs.common.clock_or(None) };
            clock.format()
        });
        graph::set_full_dates(full);
        let mono = self.app.prefs.common.mono_font_or_default();
        if mono != self.mono {
            self.mono = mono;
            fonts::load_in_background(ctx.clone(), mono);
        }
        if let Some(e) = self.app.prefs.failed.take() {
            self.app.say(e);
        }
        if let Some(text) = self.app.prefs.notice.take() {
            self.app.say(text);
        }
        if let Some(n) = self.app.prefs.said.take().filter(|n| *n > 0) {
            let text = self.app.tr("toast.keymap_saved").replace("{}", &n.to_string());
            self.app.say(text);
        }
        // What keymap.toml and config.toml could not be read as, said once each time it changes.
        let warn: Vec<String> =
            [&self.app.prefs.keymap_warn, &self.app.prefs.notify_warn, &self.app.prefs.open_warn, &self.app.prefs.view_warn]
                .into_iter()
                .flatten()
                .cloned()
                .collect();
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
            if let Some(w) = &self.app.pr_watch {
                w.front(front);
            }
        }
        self.app.pump();
        // A parked tab's reading goes on behind the front: what it sent is folded in now
        // (cheap when there is nothing), not all at once when the tab comes back.
        for parked in self.parked.iter_mut().flatten() {
            parked.pump();
        }
        if let Some(text) = self.app.clipboard.take() {
            ctx.copy_text(text);
        }
        self.app.auto_fetch_tick(std::time::SystemTime::now());
        if let Some(wake) = self.app.auto_fetch_wake() {
            ctx.request_repaint_after(wake);
        }
        self.sync_tabs();
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
        repos::show(&ctx, &mut self.app);
        newwork_ui::show(&ctx, &mut self.app);
        prform_ui::show(&ctx, &mut self.app);
        reply_ui::show(&ctx, &mut self.app);
        tidy_ui::show(&ctx, &mut self.app);
        reflog_ui::show(&ctx, &mut self.app);
        stats_ui::show(&ctx, &mut self.app);
        legend_ui::show(&ctx, &mut self.app);
        prefix::show(&ctx, &mut self.app);
        cheat_ui::show(&ctx, &mut self.app);
        ready_ui::show(&ctx, &mut self.app);
        auth_ui::show(&ctx, &mut self.app);
        guide_ui::show(&ctx, &mut self.app);
        blame_ui::show(&ctx, &mut self.app);
        trial_ui::show(&ctx, &mut self.app);
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
        if out.calm {
            self.app.calm();
        }
        if out.next {
            self.app.run_suggestion();
        }
        if out.palette {
            self.app.toggle_palette();
        }
        if out.bell {
            self.app.bell_open = !self.app.bell_open;
        } else if self.app.bell_open {
            match ui::bell_list(&ctx, &self.app, out.bell_rect) {
                None => {}
                Some(ito_notices::Action::Close) => self.app.bell_open = false,
                Some(ito_notices::Action::ReadAll) => self.app.notices.iter_mut().for_each(|n| n.read = true),
                Some(ito_notices::Action::Open(go, id)) => {
                    if let Some(n) = self.app.notices.iter_mut().find(|n| n.id == id) {
                        n.read = true;
                    }
                    if let Some(go) = go {
                        self.app.bell_open = false;
                        self.app.go(go);
                    }
                }
            }
        }
        match out.key {
            Some(ui::BarKey::Char(c)) => self.app.run_pick(cmdpalette::Pick::Key(c)),
            Some(ui::BarKey::Esc) => _ = self.app.escape(),
            None => {}
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
        if self.app.tab_req.is_some() {
            ctx.request_repaint();
        }
        if self.app.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if !ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
            ito_titlebar::resize_edges(&ctx);
        }
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
    /// Serve MCP on the standard handles for Claude Code (`michi mcp`).
    Mcp,
    /// Open a window on this folder, or on the current one.
    Open(Option<PathBuf>, Option<Vec<Step>>),
}

fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<Cli, String> {
    let mut path = None;
    let mut script = None;
    let mut args = args.into_iter().peekable();
    if args.peek().and_then(|a| a.to_str()) == Some("mcp") {
        args.next();
        return match args.next() {
            None => Ok(Cli::Mcp),
            Some(a) => Err(format!("`mcp` takes no argument: `{}`", a.to_string_lossy())),
        };
    }
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
        Ok(Cli::Mcp) => mcp::run(),
        Ok(Cli::Open(path, script)) => (path, script),
        Err(e) => {
            say(&format!("michi: {e}\n\n{USAGE}"), true);
            std::process::exit(2);
        }
    };
    let path_given = path.is_some();
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
    let viewport = ito_titlebar::frameless(egui::ViewportBuilder::default())
        .with_title("michi")
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([720.0, 420.0])
        // The frame is drawn by ui.rs and ito-titlebar: the bar, the buttons, the edges.
        // Alt+Tab and the taskbar; build.rs bakes the same drawing into
        // michi.exe for Explorer.
        .with_icon(std::sync::Arc::new(egui::IconData { rgba: icon::pixels(ICON_PX), width: ICON_PX, height: ICON_PX }));
    let script_is_none = script.is_none();
    let restore = !path_given;
    let viewport = saved.viewport(viewport);
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native(
        "michi",
        options,
        Box::new(move |cc| {
            saved.put(cc);
            let mut prefs = prefs::Prefs::load(ito_common::base_dir());
            let prefs_mono = prefs.common.mono_font_or_default();
            fonts::load_in_background(cc.egui_ctx.clone(), prefs_mono);
            prefs.watch(&cc.egui_ctx);
            prefs.apply_theme(&cc.egui_ctx);
            // The scale is common.toml's, so egui's own Ctrl+= / Ctrl+- stay out of it.
            cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
            cc.egui_ctx.set_zoom_factor(prefs.common.scale_or_one());
            // The tabs of the last run come back beside the folder asked for
            // (not when a folder was named, nor under a script); one on a
            // folder that is gone is let go.
            let kept = script_is_none.then(|| prefs::window_dir(prefs.base.as_deref()).map(|d| d.join("tabs.txt"))).flatten();
            let saved_text = kept.as_ref().filter(|_| restore).and_then(|f| std::fs::read_to_string(f).ok());
            let (tabs, dir, view) = match &saved_text {
                // Started with no folder: the tabs, the front one and its view as they were left.
                Some(text) => {
                    let mut tabs = Tabs::resume(text, &dir);
                    if tabs.retain(|p| p.is_dir()) {
                        let front = tabs.paths()[tabs.active()].clone();
                        (tabs, front, Tabs::saved_view(text).and_then(View::from_digit).filter(|v| *v != View::Conflict))
                    } else {
                        (Tabs::new(&dir), dir, None)
                    }
                }
                None => (Tabs::new(&dir), dir, None),
            };
            let parked: Vec<Option<App>> = (0..tabs.len()).map(|_| None).collect();
            let mut app = App::new(prefs, dir.clone());
            if let Some(v) = view {
                app.view = v;
            }
            app.reading = Some(repo::start(dir, cc.egui_ctx.clone()));
            let mcp = script_is_none.then(|| mcp::listen(cc.egui_ctx.clone()));
            let script = script.map(|steps| {
                let mut r = Runner::new(steps, done);
                r.watch(cc.egui_ctx.clone());
                r
            });
            Ok(Box::new(Michi {
                app,
                front: true,
                mono: prefs_mono,
                script,
                place: saved.tracker(),
                todo: Vec::new(),
                tab_stash: Vec::new(),
                warned: Vec::new(),
                mcp,
                tabs,
                parked,
                tabs_kept: kept.map(|_| String::new()),
            }))
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
        assert_eq!(parse(&["mcp"]), Ok(Cli::Mcp));
        assert!(parse(&["mcp", "x"]).is_err());
        assert!(matches!(parse(&["./mcp"]), Ok(Cli::Open(Some(_), None))), "a folder named mcp is still reachable");
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
