//! What the settings screen reads and writes (ito's `docs/common-spec.md`):
//! common.toml (the language, the scale, the theme and the clock, shared by
//! every uchmk app) and the user's themes. The files are watched every 2 s
//! and written on threads of their own; what was read comes back as a
//! [`Reload`], which [`Prefs::pump`] folds in. The UI thread never touches
//! the disk.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use ito_common::{common_path, edit_common, themes_dir, write_atomic, Common, CommonChange};
use ito_notify::webhook::Place;
use ito_prefs::State;
use ito_theme::Theme;

use crate::keymap::Keymap;
use crate::notify::NotifyCfg;

/// How often the files are looked at.
const WATCH: Duration = Duration::from_secs(2);

/// A file read again, or a write that failed.
pub enum Reload {
    Common(Result<(Common, Vec<String>), String>),
    Themes(Vec<Theme>, Vec<String>),
    /// keymap.toml's text (none when there is no file), and what the Keys page's
    /// number of changes the Keys page made to it (none when it was only read).
    Keymap(Option<String>, Option<usize>),
    /// config.toml's text (none when there is no file).
    Config(Option<String>),
    Failed(String),
}

pub struct Prefs {
    /// The `uchmk` folder; `None` when the OS has no config directory.
    pub base: Option<PathBuf>,
    pub common: Common,
    /// The built-in themes, then the user's.
    pub themes: Vec<Theme>,
    pub common_warn: Vec<String>,
    pub theme_warn: Vec<String>,
    /// The theme common.toml names but which is not there.
    pub theme_err: Option<String>,
    /// The screen's page, search and focus.
    pub state: State,
    /// The last write that failed, for a toast.
    pub failed: Option<String>,
    /// The keymap in force: the built-in keys with keymap.toml laid over.
    pub keymap: Keymap,
    pub keymap_warn: Vec<String>,
    /// How many changes the last write on the Keys page made, for a toast.
    pub said: Option<usize>,
    /// `[notify]` of config.toml, and what it warns of.
    pub notify: NotifyCfg,
    pub notify_warn: Vec<String>,
    /// `[open]` of config.toml: the commands that open a worktree's folder.
    pub open: ito_openin::Config,
    pub open_warn: Vec<String>,
    /// The diff / merge tools found installed (empty until the settings screen
    /// or the first use has asked for them, on a thread of its own).
    pub found: Vec<ito_openin::diff::Found>,
    /// The answer to a search for them that is on its way.
    pub found_rx: Option<Receiver<Vec<ito_openin::diff::Found>>>,
    /// Whether they have been looked for.
    pub found_known: bool,
    /// The external tools page's drafts and the form that adds a tool.
    pub open_drafts: ito_prefs::Drafts,
    pub open_form: ito_prefs::OpenForm,
    /// `[view] density` of config.toml: the height of the graph's and the lists' rows.
    pub density: michi_core::density::Density,
    /// `[view] motion` of config.toml: the short fades and slides (#88).
    pub motion: michi_core::motion::Motion,
    pub view_warn: Vec<String>,
    /// `[view] avatars` of config.toml: whether authors' pictures are fetched from GitHub.
    pub avatars: bool,
    /// `[net] auto_fetch` of config.toml: how often to fetch by itself (#70).
    pub auto_fetch: michi_core::autofetch::Every,
    /// `[commit] co_author` of config.toml: the trailer added to michi's commits (#54).
    pub co_author: michi_core::coauthor::CoAuthor,
    /// `[issues]` of config.toml: how issue references become links (#44).
    pub issues: michi_core::issues::Config,
    /// The Notifications page's drafts (a webhook's address being typed).
    pub notify_drafts: ito_prefs::Drafts,
    /// The Keys page's search, filter and the keys being pressed.
    pub keys_edit: ito_prefs::KeysEdit,
    tx: Sender<Reload>,
    rx: Receiver<Reload>,
    wake: Option<egui::Context>,
    /// The OS's light or dark, for `system` (egui's lags a switch on Windows).
    os_light: ito_common::OsLight,
}

impl Prefs {
    /// Nothing read: the defaults and the built-in themes (the tests).
    pub fn empty() -> Self {
        let (tx, rx) = channel();
        Self {
            base: None,
            common: Common::default(),
            themes: ito_theme::builtin(),
            common_warn: Vec::new(),
            theme_warn: Vec::new(),
            theme_err: None,
            state: State::default(),
            failed: None,
            keymap: Keymap::load(None).0,
            keymap_warn: Vec::new(),
            said: None,
            notify: NotifyCfg::default(),
            notify_warn: Vec::new(),
            open: Default::default(),
            open_warn: Vec::new(),
            found: Vec::new(),
            found_rx: None,
            found_known: false,
            open_drafts: ito_prefs::Drafts::new(),
            open_form: ito_prefs::OpenForm::default(),
            density: Default::default(),
            motion: Default::default(),
            avatars: true,
            auto_fetch: Default::default(),
            co_author: Default::default(),
            issues: Default::default(),
            view_warn: Vec::new(),
            notify_drafts: ito_prefs::Drafts::new(),
            keys_edit: ito_prefs::KeysEdit::default(),
            tx,
            rx,
            wake: None,
            os_light: ito_common::OsLight::default(),
        }
    }

    /// common.toml and the themes in `base`.
    pub fn load(base: Option<PathBuf>) -> Self {
        let mut p = Self::empty();
        if let Some(b) = &base {
            match Common::read(b) {
                Ok((c, w)) => (p.common, p.common_warn) = (c, w),
                Err(e) => p.common_warn = vec![e],
            }
            let (user, warn) = ito_theme::load_dir(&themes_dir(b));
            p.themes.extend(user);
            p.theme_warn = warn;
            let (km, warn) = Keymap::load(read_keymap(b).as_deref());
            (p.keymap, p.keymap_warn) = (km, warn);
            let config = read_config(b);
            (p.notify, p.notify_warn) = NotifyCfg::read(config.as_deref());
            (p.open, p.open_warn) = ito_openin::Config::read(config.as_deref());
            p.read_view(config.as_deref());
        }
        p.base = base;
        p
    }

    /// Look at the files and the OS's light or dark every 2 s from now on,
    /// and wake `ctx` when one has been read again.
    pub fn watch(&mut self, ctx: &egui::Context) {
        self.wake = Some(ctx.clone());
        let os_ctx = ctx.clone();
        self.os_light = ito_common::OsLight::watch(WATCH, move || os_ctx.request_repaint());
        let Some(base) = self.base.clone() else { return };
        let (common, themes, keymap) = (common_path(&base), themes_dir(&base), keymap_path(&base));
        let config = config_path(&base);
        let mut paths = vec![common.clone(), keymap.clone(), config.clone(), themes.clone()];
        // A theme file edited in place; one added or taken away changes the
        // folder's time.
        if let Ok(entries) = std::fs::read_dir(&themes) {
            paths.extend(entries.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "toml")));
        }
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        ito_common::watch(paths, WATCH, move |changed| {
            let mut sent = true;
            if changed.contains(&common) {
                sent &= tx.send(Reload::Common(Common::read(&base))).is_ok();
            }
            if changed.contains(&keymap) {
                sent &= tx.send(Reload::Keymap(read_keymap(&base), None)).is_ok();
            }
            if changed.contains(&config) {
                sent &= tx.send(Reload::Config(read_config(&base))).is_ok();
            }
            if changed.iter().any(|p| p.starts_with(&themes)) {
                let (user, warn) = ito_theme::load_dir(&themes);
                let all = ito_theme::builtin().into_iter().chain(user).collect();
                sent &= tx.send(Reload::Themes(all, warn)).is_ok();
            }
            ctx.request_repaint();
            sent
        });
    }

    /// Fold in what the threads have read since the last frame.
    pub fn pump(&mut self) {
        if let Some(rx) = &self.found_rx {
            match rx.try_recv() {
                Ok(found) => {
                    (self.found, self.found_known, self.found_rx) = (found, true, None);
                    if let Some(ctx) = &self.wake {
                        ctx.request_repaint();
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    if let Some(ctx) = &self.wake {
                        ctx.request_repaint_after(Duration::from_millis(100));
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => (self.found_known, self.found_rx) = (true, None),
            }
        }
        while let Ok(r) = self.rx.try_recv() {
            match r {
                Reload::Common(Ok((c, w))) => (self.common, self.common_warn) = (c, w),
                Reload::Common(Err(e)) => self.common_warn = vec![e],
                Reload::Themes(t, w) => (self.themes, self.theme_warn) = (t, w),
                Reload::Keymap(text, said) => {
                    (self.keymap, self.keymap_warn) = Keymap::load(text.as_deref());
                    if said.is_some() {
                        self.said = said;
                    }
                }
                Reload::Config(text) => {
                    (self.notify, self.notify_warn) = NotifyCfg::read(text.as_deref());
                    (self.open, self.open_warn) = ito_openin::Config::read(text.as_deref());
                    self.read_view(text.as_deref());
                }
                Reload::Failed(e) => self.failed = Some(e),
            }
        }
    }

    fn send_back(tx: &Sender<Reload>, wake: &Option<egui::Context>, r: Reload) {
        let _ = tx.send(r);
        if let Some(ctx) = wake {
            ctx.request_repaint();
        }
    }

    /// Write `changes` into common.toml on a thread, then read it again.
    pub fn common_change(&self, changes: Vec<CommonChange>, no_folder: &str) {
        if changes.is_empty() {
            return;
        }
        let (tx, wake) = (self.tx.clone(), self.wake.clone());
        let Some(base) = self.base.clone() else {
            Self::send_back(&tx, &wake, Reload::Failed(no_folder.into()));
            return;
        };
        std::thread::spawn(move || {
            let done = edit_common(&base, |text| changes.iter().try_fold(text.to_owned(), |t, c| c.apply(&t)));
            let r = match done {
                Ok(()) => Reload::Common(Common::read(&base)),
                Err(e) => Reload::Failed(e),
            };
            Self::send_back(&tx, &wake, r);
        });
    }

    /// Make the Keys page's `edits` in keymap.toml on a thread (only what
    /// differs from the built-in keys is written), then read it again.
    pub fn keymap_edit(&self, edits: Vec<ito_keymap::Edit>, no_folder: &str) {
        if edits.is_empty() {
            return;
        }
        let (tx, wake) = (self.tx.clone(), self.wake.clone());
        let Some(base) = self.base.clone() else {
            Self::send_back(&tx, &wake, Reload::Failed(no_folder.into()));
            return;
        };
        std::thread::spawn(move || {
            let r = match write_keymap(&keymap_path(&base), &edits) {
                Ok(said) => Reload::Keymap(read_keymap(&base), Some(said)),
                Err(e) => Reload::Failed(e),
            };
            Self::send_back(&tx, &wake, r);
        });
    }

    /// Write the Notifications page's `edits` (and its `webhooks`, when it
    /// changed them) into config.toml on a thread, then read it again.
    pub fn notify_edit(&self, edits: Vec<(&'static str, String)>, webhooks: Option<Vec<Place>>, no_folder: &str) {
        if edits.is_empty() && webhooks.is_none() {
            return;
        }
        let (tx, wake) = (self.tx.clone(), self.wake.clone());
        let Some(base) = self.base.clone() else {
            Self::send_back(&tx, &wake, Reload::Failed(no_folder.into()));
            return;
        };
        std::thread::spawn(move || {
            let r = match write_config(&config_path(&base), &edits, &[], webhooks.as_deref()) {
                Ok(()) => Reload::Config(read_config(&base)),
                Err(e) => Reload::Failed(e),
            };
            Self::send_back(&tx, &wake, r);
        });
    }

    /// Looks for the installed diff / merge tools on a thread, once. The answer
    /// is taken in by [`Prefs::pump`].
    pub fn find_tools(&mut self) {
        if self.found_known || self.found_rx.is_some() {
            return;
        }
        self.found_rx = Some(ito_openin::diff::detect_in_background());
        if let Some(ctx) = &self.wake {
            // The thread cannot wake the window; the pump polls while it waits.
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    /// Make the external tools page's `edits` in config.toml on a thread, then
    /// read it again.
    pub fn open_edit(&self, edits: Vec<ito_openin::Edit>, no_folder: &str) {
        if edits.is_empty() {
            return;
        }
        let (tx, wake) = (self.tx.clone(), self.wake.clone());
        let Some(base) = self.base.clone() else {
            Self::send_back(&tx, &wake, Reload::Failed(no_folder.into()));
            return;
        };
        std::thread::spawn(move || {
            let r = match write_open(&config_path(&base), &edits) {
                Ok(()) => Reload::Config(read_config(&base)),
                Err(e) => Reload::Failed(e),
            };
            Self::send_back(&tx, &wake, r);
        });
    }

    /// `[view]` of the config.toml `text`: the density and the avatars.
    fn read_view(&mut self, text: Option<&str>) {
        (self.density, self.view_warn) = crate::viewcfg::read(text);
        let (motion, warn) = crate::viewcfg::read_motion(text);
        self.motion = motion;
        self.view_warn.extend(warn);
        let (on, warn) = crate::viewcfg::avatars(text);
        self.avatars = on;
        self.view_warn.extend(warn);
        let (every, warn) = crate::viewcfg::auto_fetch(text);
        self.auto_fetch = every;
        self.view_warn.extend(warn);
        let (co, warn) = crate::viewcfg::co_author(text);
        self.co_author = co;
        self.view_warn.extend(warn);
        let (issues, warn) = crate::viewcfg::issues(text);
        self.issues = issues;
        self.view_warn.extend(warn);
    }

    /// Write `[view] avatars` into config.toml on a thread, then read it again.
    pub fn avatars_set(&self, on: bool, no_folder: &str) {
        self.view_set(("avatars", on.to_string()), no_folder);
    }

    /// Write `[view] density` into config.toml on a thread, then read it again.
    pub fn density_set(&self, d: michi_core::density::Density, no_folder: &str) {
        self.view_set(("density", ito_common::quote(d.key())), no_folder);
    }

    /// Write `[view] motion` into config.toml on a thread, then read it again.
    pub fn motion_set(&self, m: michi_core::motion::Motion, no_folder: &str) {
        self.view_set(("motion", ito_common::quote(m.key())), no_folder);
    }

    /// Write one `[view]` key (the value already TOML) on a thread, then read config.toml again.
    fn view_set(&self, edit: (&'static str, String), no_folder: &str) {
        let (tx, wake) = (self.tx.clone(), self.wake.clone());
        let Some(base) = self.base.clone() else {
            Self::send_back(&tx, &wake, Reload::Failed(no_folder.into()));
            return;
        };
        std::thread::spawn(move || {
            let r = match write_config(&config_path(&base), &[], &[edit], None) {
                Ok(()) => Reload::Config(read_config(&base)),
                Err(e) => Reload::Failed(e),
            };
            Self::send_back(&tx, &wake, r);
        });
    }

    /// Open common.toml (made empty first when there is none) with what the
    /// OS opens it with, on a thread.
    pub fn open_common(&self, no_folder: &str) {
        let (tx, wake) = (self.tx.clone(), self.wake.clone());
        let Some(base) = self.base.clone() else {
            Self::send_back(&tx, &wake, Reload::Failed(no_folder.into()));
            return;
        };
        std::thread::spawn(move || {
            let path = common_path(&base);
            let made = if path.exists() { Ok(()) } else { write_atomic(&path, "") };
            if let Err(e) = made.and_then(|()| open_path(&path)) {
                Self::send_back(&tx, &wake, Reload::Failed(e));
            }
        });
    }

    /// The theme common.toml picks, in force from this frame on. Called
    /// every frame (the OS may have gone light or dark).
    pub fn apply_theme(&mut self, ctx: &egui::Context) {
        let os_light = self.os_light.light_or(ctx.system_theme() == Some(egui::Theme::Light));
        let (choice, dark, light) = self.common.theme_choice(None);
        let colors = match ito_theme::pick(&self.themes, &choice, &dark, &light, os_light) {
            Ok(t) => {
                self.theme_err = None;
                t.colors
            }
            Err(e) => {
                self.theme_err = Some(e);
                self.themes[0].colors
            }
        };
        if colors != ito_theme::colors() {
            ito_theme::set(colors);
            ctx.set_visuals(colors.visuals());
        }
    }
}

/// keymap.toml's full path: in michi's own folder (`MICHI_CONFIG_HOME` when
/// set, else `michi` in the `uchmk` folder `base`).
pub fn keymap_path(base: &Path) -> PathBuf {
    ito_common::env_dir("MICHI_CONFIG_HOME").unwrap_or_else(|| base.join("michi")).join("keymap.toml")
}

/// The folder window.toml lives in: michi's own, beside keymap.toml.
pub fn window_dir(base: Option<&Path>) -> Option<PathBuf> {
    base.and_then(|b| keymap_path(b).parent().map(Path::to_path_buf))
}

/// config.toml's full path: in michi's own folder, beside keymap.toml.
pub fn config_path(base: &Path) -> PathBuf {
    keymap_path(base).with_file_name("config.toml")
}

/// config.toml's text; none when there is no file.
fn read_config(base: &Path) -> Option<String> {
    std::fs::read_to_string(config_path(base)).ok()
}

/// Set `edits` (`[notify]` keys, already TOML) and `webhooks` in the
/// config.toml at `path`, written only when that changes it. Call it off the
/// UI thread.
pub fn write_config(path: &Path, edits: &[(&str, String)], view: &[(&str, String)], webhooks: Option<&[Place]>) -> Result<(), String> {
    static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let at = |e: String| format!("{}: {e}", path.display());
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(at(e.to_string())),
    };
    let mut new = text.clone();
    for (key, value) in edits {
        new = ito_common::set_key(&new, Some("notify"), key, value).map_err(at)?;
    }
    for (key, value) in view {
        new = ito_common::set_key(&new, Some("view"), key, value).map_err(at)?;
    }
    if let Some(places) = webhooks {
        new = ito_prefs::write_webhooks(&new, places, true).map_err(at)?;
    }
    if new == text {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| at(e.to_string()))?;
    }
    write_atomic(path, new)
}

/// Make the external tools page's `edits` in the config.toml at `path`,
/// written only when that changes it. Call it off the UI thread.
pub fn write_open(path: &Path, edits: &[ito_openin::Edit]) -> Result<(), String> {
    static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let at = |e: String| format!("{}: {e}", path.display());
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(at(e.to_string())),
    };
    let new = ito_prefs::write_edits(&text, edits).map_err(at)?;
    if new == text {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| at(e.to_string()))?;
    }
    write_atomic(path, new)
}

/// keymap.toml's text; none when there is no file (or it cannot be read, which
/// the keymap then reads as having no file).
fn read_keymap(base: &Path) -> Option<String> {
    std::fs::read_to_string(keymap_path(base)).ok()
}

/// Make `edits` in the keymap.toml at `path` (none yet is an empty one),
/// written only when that changes it; how many changes were made. Call it off the
/// UI thread.
pub fn write_keymap(path: &Path, edits: &[ito_keymap::Edit]) -> Result<usize, String> {
    static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let at = |e: String| format!("{}: {e}", path.display());
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(at(e.to_string())),
    };
    let new = ito_keymap::edit(&text, &crate::keymap::SPEC, edits, "noop").map_err(at)?;
    if new == text {
        return Ok(0);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| at(e.to_string()))?;
    }
    write_atomic(path, new)?;
    Ok(edits.len())
}

#[cfg(windows)]
fn opener() -> std::process::Command {
    std::process::Command::new("explorer")
}

#[cfg(target_os = "macos")]
fn opener() -> std::process::Command {
    std::process::Command::new("open")
}

#[cfg(not(any(windows, target_os = "macos")))]
fn opener() -> std::process::Command {
    std::process::Command::new("xdg-open")
}

fn open_path(path: &Path) -> Result<(), String> {
    // explorer answers 1 even when it opened the file, so only a failure to
    // start it is said.
    opener().arg(path).status().map(drop).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ito_keymap::Edit;

    #[test]
    fn a_key_added_is_written_ahead_of_the_defaults_and_reset_takes_it_back() {
        let dir = std::env::temp_dir().join(format!("michi-keymap-{}", std::process::id()));
        let path = dir.join("keymap.toml");
        let _ = std::fs::remove_dir_all(&dir);
        let on = vec![ito_keys::Key::parse("<C-k>").expect("a key")];
        let add = Edit::Add { layer: "main".into(), on, run: "palette".into(), desc: "Search commands".into() };
        assert_eq!(write_keymap(&path, &[add]), Ok(1));
        let text = std::fs::read_to_string(&path).expect("written");
        assert!(text.contains("prepend_keymap") && text.contains("<C-k>"), "{text}");
        let (km, warn) = Keymap::load(Some(&text));
        assert!(warn.is_empty(), "{warn:?}");
        assert_eq!(km.act(ito_keys::Key::parse("<C-k>").expect("a key")), Some(crate::keymap::Act::Palette));
        let reset = Edit::Reset { layer: "main".into(), run: "palette".into() };
        assert_eq!(write_keymap(&path, &[reset]), Ok(1));
        let (km, _) = Keymap::load(std::fs::read_to_string(&path).ok().as_deref());
        assert_eq!(km.act(ito_keys::Key::parse("<C-k>").expect("a key")), None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
