//! Telling of an event while the window is behind another, the ways
//! `[notify]` in michi's config.toml picks (common-spec, "通知"): the
//! system's notification with michi's icon, a sound, and the webhooks. And
//! `[notify]` as the settings' Notifications page
//! (`ito_prefs::notify_page`) shows it.

pub use ito_prefs::NotifyCfg;
use ito_prefs::DONE_FAILED;
use serde::Deserialize;

/// Who the notifications are from.
const APP: ito_notify::toast::App = ito_notify::toast::App { id: "uchmk.michi", name: "michi" };

/// What michi tells of: a Claude it handed a conflict has stopped, and a
/// fetch, pull or push that failed.
pub const EVENTS: [ito_prefs::Event; 2] = DONE_FAILED;

#[derive(Deserialize, Default)]
#[serde(default)]
struct File {
    notify: NotifyCfg,
}

/// `[notify]` of the config.toml `text` (none: no file), with what to warn of.
pub fn read(text: Option<&str>) -> (NotifyCfg, Vec<String>) {
    let mut warnings = Vec::new();
    let mut n = match text.map(toml::from_str::<File>) {
        Some(Ok(f)) => f.notify,
        Some(Err(e)) => {
            warnings.push(format!("config.toml: {}", e.message()));
            NotifyCfg::default()
        }
        None => NotifyCfg::default(),
    };
    n.check(&EVENTS, "config.toml", &mut warnings);
    (n, warnings)
}

/// michi's icon at `side` pixels, as the window's.
fn icon(side: u32) -> Vec<u8> {
    crate::icon::pixels(side)
}

/// The window, to wake it for a webhook-free background job; set on the first frame.
static WINDOW: std::sync::OnceLock<eframe::egui::Context> = std::sync::OnceLock::new();

/// Remember the window, so that a click on a notification can reach it.
pub fn set_window(ctx: &eframe::egui::Context) {
    let _ = WINDOW.set(ctx.clone());
    ito_prefs::notify_window(ctx);
}

/// The window, once a frame has been drawn.
pub fn window() -> Option<eframe::egui::Context> {
    WINDOW.get().cloned()
}

/// The notices whose system notification was clicked since the last call.
pub fn take_clicks() -> Vec<u64> {
    #[cfg(test)]
    let pushed = std::mem::take(&mut *TEST_CLICKS.lock().unwrap_or_else(|e| e.into_inner()));
    #[cfg(not(test))]
    let pushed = Vec::new();
    pushed.into_iter().chain(ito_prefs::take_clicks().iter().filter_map(|t| t.parse::<u64>().ok())).collect()
}

#[cfg(test)]
static TEST_CLICKS: std::sync::Mutex<Vec<u64>> = std::sync::Mutex::new(Vec::new());

#[cfg(test)]
pub fn push_click(id: u64) {
    TEST_CLICKS.lock().unwrap_or_else(|e| e.into_inner()).push(id);
}

/// Tell of `event` (one of [`EVENTS`]) in every way `n` turns on for it.
/// `notice` is the bell's notice the system's notification opens when clicked.
pub fn tell(n: &NotifyCfg, event: &str, title: &str, body: &str, notice: Option<u64>) {
    match notice {
        Some(id) => n.tell_open(&EVENTS, APP, icon, event, title, body, &id.to_string(), "Open"),
        None => n.tell(&EVENTS, APP, icon, event, title, body),
    }
}

/// "Send a test", as the page sets things now: every way, or only the
/// webhook `what` names.
pub fn test(p: &ito_prefs::Notify, what: ito_prefs::Test, title: &str, body: &str) {
    ito_prefs::test_notify(&EVENTS, p, what, APP, icon, title, body);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_not_known_is_warned_of_and_set_back() {
        let (n, warn) = read(Some(
            "[notify]
system = [\"done\", \"waiting\"]
sound_done = \"boing\"
",
        ));
        assert_eq!(n.system, Some(vec!["done".to_owned()]), "{warn:?}");
        assert_eq!(n.sounds[0], "chime");
        assert_eq!(warn.len(), 2, "{warn:?}");
        let (n, warn) = read(None);
        assert!(warn.is_empty() && n == NotifyCfg::new(&EVENTS));
        let (_, warn) = read(Some(
            "[notify
",
        ));
        assert_eq!(warn.len(), 1);
    }

    #[test]
    fn the_old_single_webhook_is_the_first_place() {
        let (n, _) = read(Some(
            "[notify]
webhook = \"https://ntfy.sh/x\"
[[notify.webhooks]]
url = \"https://ntfy.sh/y\"
",
        ));
        assert_eq!(n.places.iter().map(|p| p.url.as_str()).collect::<Vec<_>>(), ["https://ntfy.sh/x", "https://ntfy.sh/y"]);
    }
}
