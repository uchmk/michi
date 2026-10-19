//! Telling of an event while the window is behind another, the ways
//! `[notify]` in michi's config.toml picks (common-spec, "通知"): the
//! system's notification with michi's icon, a sound, and the webhooks. And
//! `[notify]` as the settings' Notifications page
//! (`ito_prefs::notify_page`) shows it.

use ito_notify::webhook::{Level, Place};
use serde::Deserialize;

/// Who the notifications are from.
const APP: ito_notify::toast::App = ito_notify::toast::App { id: "uchmk.michi", name: "michi" };

/// What michi tells of: a Claude it handed a conflict has stopped, and a
/// fetch, pull or push that failed.
pub const EVENTS: [&str; 2] = ["done", "failed"];

/// `[notify]` as written. What is not known is warned of and set back
/// ([`NotifyCfg::check`]).
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct NotifyCfg {
    /// The events told through the system's notification.
    pub system: Vec<String>,
    /// The events told with a sound.
    pub sound: Vec<String>,
    /// The one webhook of before (`webhook`, `webhook_format`,
    /// `webhook_states`): read as the first of `places`.
    pub webhook_states: Vec<String>,
    pub sound_done: String,
    pub sound_failed: String,
    /// Windows: no sound in focus mode or under a full-screen program.
    pub focus_mode: bool,
    pub webhook: String,
    pub webhook_format: String,
    /// `[[notify.webhooks]]`, as written.
    pub webhooks: Vec<HookCfg>,
    /// Where it is told of, in order. Made by [`NotifyCfg::check`].
    #[serde(skip)]
    pub places: Vec<Place>,
}

/// A `[[notify.webhooks]]` table.
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(default)]
pub struct HookCfg {
    pub url: String,
    pub format: String,
    pub states: Vec<String>,
}

impl Default for HookCfg {
    fn default() -> Self {
        Self { url: String::new(), format: "ntfy".into(), states: EVENTS.map(str::to_owned).to_vec() }
    }
}

impl Default for NotifyCfg {
    fn default() -> Self {
        let both = || EVENTS.map(str::to_owned).to_vec();
        Self {
            system: both(),
            sound: Vec::new(),
            webhook_states: both(),
            sound_done: "chime".into(),
            sound_failed: "alert".into(),
            focus_mode: true,
            webhook: String::new(),
            webhook_format: "ntfy".into(),
            webhooks: Vec::new(),
            places: Vec::new(),
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct File {
    notify: NotifyCfg,
}

impl NotifyCfg {
    /// `[notify]` of the config.toml `text` (none: no file), with what to
    /// warn of.
    pub fn read(text: Option<&str>) -> (Self, Vec<String>) {
        let mut warnings = Vec::new();
        let mut n = match text.map(toml::from_str::<File>) {
            Some(Ok(f)) => f.notify,
            Some(Err(e)) => {
                warnings.push(format!("config.toml: {}", e.message()));
                Self::default()
            }
            None => Self::default(),
        };
        n.check(&mut warnings);
        (n, warnings)
    }

    /// Warn of an event, a sound or a format michi does not know, and set it
    /// back to the default; and of a webhook that is not http(s).
    fn check(&mut self, warnings: &mut Vec<String>) {
        let d = Self::default();
        let path = "config.toml";
        for (key, list) in [("system", &mut self.system), ("sound", &mut self.sound), ("webhook_states", &mut self.webhook_states)] {
            list.retain(|e| {
                let known = EVENTS.contains(&e.as_str());
                if !known {
                    warnings.push(format!("{path}: [notify] `{key}`: `{e}` is not {}", EVENTS.join(" or ")));
                }
                known
            });
        }
        for (key, now, was) in
            [("sound_done", &mut self.sound_done, d.sound_done), ("sound_failed", &mut self.sound_failed, d.sound_failed)]
        {
            if !ito_notify::SOUNDS.contains(&now.as_str()) {
                warnings.push(format!("{path}: [notify] `{key}` `{now}` is not {}", ito_notify::SOUNDS.join(", ")));
                *now = was;
            }
        }
        if !ito_notify::FORMATS.contains(&self.webhook_format.as_str()) {
            warnings.push(format!("{path}: [notify] `webhook_format` `{}` is not {}", self.webhook_format, ito_notify::FORMATS.join(", ")));
            self.webhook_format = d.webhook_format;
        }
        if !self.webhook.is_empty() && !ito_notify::is_http(&self.webhook) {
            warnings.push(format!("{path}: [notify] `webhook` `{}` is not an http(s) address", self.webhook));
        }
        let mut list = Vec::new();
        for (i, h) in self.webhooks.iter().enumerate() {
            let mut p = Place { url: h.url.trim().to_owned(), format: h.format.clone(), states: h.states.clone() };
            warnings.extend(p.problems(&EVENTS).into_iter().map(|w| format!("{path}: [[notify.webhooks]] {}: {w}", i + 1)));
            if !ito_notify::FORMATS.contains(&p.format.as_str()) {
                p.format = "ntfy".into();
            }
            p.states.retain(|s| EVENTS.contains(&s.as_str()));
            list.push(p);
        }
        let old = Place { url: self.webhook.trim().to_owned(), format: self.webhook_format.clone(), states: self.webhook_states.clone() };
        self.places = ito_notify::webhook::places(old, list);
    }

    /// As the settings' Notifications page shows it: a column per event of
    /// [`EVENTS`], in that order.
    pub fn page(&self) -> ito_prefs::Notify {
        let col = |list: &[String]| EVENTS.iter().map(|e| list.iter().any(|x| x == e)).collect();
        ito_prefs::Notify {
            system: col(&self.system),
            sound: col(&self.sound),
            sounds: vec![self.sound_done.clone(), self.sound_failed.clone()],
            focus: self.focus_mode,
            hooks: self.places.iter().map(|p| ito_prefs::Hook::of(p, &EVENTS, true)).collect(),
        }
    }

    /// The webhooks to write when the page's `p` changed them: none when
    /// they are the same as now.
    pub fn webhooks_of(&self, p: &ito_prefs::Notify) -> Option<Vec<Place>> {
        let now: Vec<Place> = p.hooks.iter().map(|h| h.place(&EVENTS)).collect();
        (now != self.places).then_some(now)
    }

    /// The `[notify]` keys to write, as TOML, so that the file reads as the
    /// page's `p`: only those that changed.
    pub fn edits(&self, p: &ito_prefs::Notify) -> Vec<(&'static str, String)> {
        let list = |col: &[bool]| {
            let on: Vec<String> = EVENTS.iter().zip(col).filter(|(_, on)| **on).map(|(e, _)| format!("\"{e}\"")).collect();
            format!("[{}]", on.join(", "))
        };
        let was = self.page();
        let mut out = Vec::new();
        for (key, a, b) in [("system", &was.system, &p.system), ("sound", &was.sound, &p.sound)] {
            if a != b {
                out.push((key, list(b)));
            }
        }
        for (i, key) in ["sound_done", "sound_failed"].into_iter().enumerate() {
            if let Some(now) = p.sounds.get(i).filter(|now| Some(*now) != was.sounds.get(i)) {
                out.push((key, ito_common::quote(now)));
            }
        }
        if p.focus != was.focus {
            out.push(("focus_mode", p.focus.to_string()));
        }
        out
    }
}

/// michi's icon at `side` pixels, as the window's.
fn icon(side: u32) -> Vec<u8> {
    crate::icon::pixels(side)
}

/// The ids of the notices whose system notification was clicked, until the frame takes them.
static CLICKED: std::sync::Mutex<Vec<u64>> = std::sync::Mutex::new(Vec::new());
/// The window, to wake and raise when a notification is clicked.
static WINDOW: std::sync::OnceLock<eframe::egui::Context> = std::sync::OnceLock::new();
static NOTIFIER: std::sync::OnceLock<ito_notify::notifier::Notifier> = std::sync::OnceLock::new();

/// Remember the window, so that a click on a notification can reach it.
pub fn set_window(ctx: &eframe::egui::Context) {
    let _ = WINDOW.set(ctx.clone());
}

/// The notices whose system notification was clicked since the last call.
pub fn take_clicks() -> Vec<u64> {
    std::mem::take(&mut *CLICKED.lock().unwrap_or_else(|e| e.into_inner()))
}

#[cfg(test)]
pub fn push_click(id: u64) {
    CLICKED.lock().unwrap_or_else(|e| e.into_inner()).push(id);
}

fn logo() -> Option<std::path::PathBuf> {
    ito_notify::toast::icon_file(APP.id, icon)
}

fn notifier() -> &'static ito_notify::notifier::Notifier {
    NOTIFIER.get_or_init(|| {
        let click: ito_notify::notifier::Click = std::sync::Arc::new(|tag, arg| {
            if arg != ito_notify::notifier::OPEN {
                return;
            }
            if let Ok(id) = tag.parse::<u64>() {
                CLICKED.lock().unwrap_or_else(|e| e.into_inner()).push(id);
                if let Some(ctx) = WINDOW.get() {
                    ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Focus);
                    ctx.request_repaint();
                }
            }
        });
        ito_notify::notifier::Notifier::new(APP, Some(logo), click)
    })
}

/// Tell of `event` (one of [`EVENTS`]) in every way `n` turns on for it.
/// `notice` is the bell's notice the system's notification opens when clicked.
pub fn tell(n: &NotifyCfg, event: &str, title: &str, body: &str, notice: Option<u64>) {
    let on = |list: &[String]| list.iter().any(|e| e == event);
    let sound = if event == "failed" { &n.sound_failed } else { &n.sound_done };
    send(n, on(&n.system), on(&n.sound).then_some(sound.as_str()), title, body, notice);
    let level = if event == "failed" { Level::Error } else { Level::Done };
    ito_notify::webhook::send_all(&n.places, event, level, title, body, "");
}

/// "Send a test", as the page sets things now: every way, or only the
/// webhook `what` names.
pub fn test(p: &ito_prefs::Notify, what: ito_prefs::Test, title: &str, body: &str) {
    let places: Vec<Place> = p.hooks.iter().map(|h| h.place(&EVENTS)).collect();
    match what {
        ito_prefs::Test::All => {
            let n = NotifyCfg { focus_mode: p.focus, ..NotifyCfg::default() };
            send(&n, true, p.sounds.first().map(String::as_str), title, body, None);
            ito_notify::webhook::send_all(&places, "done", Level::Done, title, body, "");
        }
        ito_prefs::Test::Hook(k) => {
            if let Some(h) = places.get(k).filter(|h| ito_notify::is_http(&h.url)) {
                ito_notify::webhook::send(
                    h.url.trim().to_owned(),
                    h.format.clone(),
                    Level::Done,
                    title.to_owned(),
                    body.to_owned(),
                    String::new(),
                );
            }
        }
    }
}

fn send(n: &NotifyCfg, system: bool, sound: Option<&str>, title: &str, body: &str, notice: Option<u64>) {
    if system {
        match notice {
            Some(id) => notifier().show(ito_notify::notifier::Toast::open(id, title, body, "Open")),
            None => ito_notify::toast::show_with_icon(APP, icon, title, body),
        }
    }
    if let Some(name) = sound {
        if !(n.focus_mode && ito_notify::sound::quiet_time()) {
            ito_notify::sound::play(name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The page shows the file's lists as columns and writes back only what
    /// was changed, as the file spells it.
    #[test]
    fn the_page_writes_only_what_changed() {
        let was = NotifyCfg::default();
        let mut p = was.page();
        assert_eq!(p.system, [true, true]);
        assert_eq!(p.sound, [false, false]);
        assert!(was.edits(&p).is_empty());
        p.system[1] = false;
        p.sound[1] = true;
        p.sounds[1] = "low".into();
        p.focus = false;
        assert_eq!(
            was.edits(&p),
            [
                ("system", "[\"done\"]".to_owned()),
                ("sound", "[\"failed\"]".to_owned()),
                ("sound_failed", "\"low\"".to_owned()),
                ("focus_mode", "false".to_owned()),
            ]
        );
        assert_eq!(was.webhooks_of(&p), None, "no webhook, none to write");
        p.hooks.push(ito_prefs::Hook::new(2));
        p.hooks[0].url = "http://127.0.0.1:8765/hook".into();
        let place = Place { url: "http://127.0.0.1:8765/hook".into(), format: "ntfy".into(), states: vec!["done".into(), "failed".into()] };
        assert_eq!(was.webhooks_of(&p), Some(vec![place.clone()]));
        let now = NotifyCfg { places: vec![place], ..was };
        assert_eq!(now.page().hooks, p.hooks, "and back");
        assert_eq!(now.webhooks_of(&p), None);
    }

    #[test]
    fn what_is_not_known_is_warned_of_and_set_back() {
        let (n, warn) = NotifyCfg::read(Some("[notify]\nsystem = [\"done\", \"waiting\"]\nsound_done = \"boing\"\n"));
        assert_eq!(n.system, ["done"]);
        assert_eq!(n.sound_done, "chime");
        assert_eq!(warn.len(), 2, "{warn:?}");
        let (n, warn) = NotifyCfg::read(None);
        assert!(warn.is_empty() && n == NotifyCfg::default());
        let (_, warn) = NotifyCfg::read(Some("[notify\n"));
        assert_eq!(warn.len(), 1);
    }

    #[test]
    fn the_old_single_webhook_is_the_first_place() {
        let (n, _) = NotifyCfg::read(Some("[notify]\nwebhook = \"https://ntfy.sh/x\"\n[[notify.webhooks]]\nurl = \"https://ntfy.sh/y\"\n"));
        assert_eq!(n.places.iter().map(|p| p.url.as_str()).collect::<Vec<_>>(), ["https://ntfy.sh/x", "https://ntfy.sh/y"]);
    }
}
