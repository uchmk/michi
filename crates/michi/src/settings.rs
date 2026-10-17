//! The settings screen (`,`): `ito-prefs` draws the nav and the cards, and
//! the rows common to every uchmk app (the language, the size, the clock,
//! the theme) write common.toml. michi has no settings of its own yet.

use egui::{vec2, Align2, FontId, Sense, Stroke, StrokeKind, Ui};
use ito_common::{themes_dir, CommonChange};
use ito_prefs::{section, Words};

use crate::app::App;

const GENERAL: usize = 0;
const THEME: usize = 1;
const KEYS: usize = 2;
const NOTIFY: usize = 3;

/// The rows on each page, for the search.
fn index(w: &'static Words, lang: &str) -> Vec<(usize, &'static str)> {
    let mut v = vec![
        (GENERAL, w.language),
        (GENERAL, w.scale),
        (GENERAL, w.clock),
        (GENERAL, w.show_time),
        (GENERAL, w.time_format),
        (GENERAL, w.show_date),
        (GENERAL, w.date_format),
        (GENERAL, w.weekday),
        (GENERAL, "language"),
        (GENERAL, "scale"),
        (GENERAL, "[clock]"),
        (GENERAL, "[open]"),
        (GENERAL, "density"),
        (GENERAL, "motion"),
        (GENERAL, "[view]"),
        (THEME, w.mode),
        (THEME, w.themes),
        (THEME, w.preview),
        (THEME, "theme"),
        (THEME, "dark_theme"),
        (THEME, "light_theme"),
    ];
    v.extend(ito_prefs::keys_index(ito_prefs::KeyWords::of(lang)).into_iter().map(|t| (KEYS, t)));
    v.extend(ito_prefs::notify_index(w).into_iter().map(|t| (NOTIFY, t)));
    v.extend([(NOTIFY, "[notify]"), (NOTIFY, "focus_mode"), (NOTIFY, "[[notify.webhooks]]")]);
    v
}

pub fn show(ui: &mut Ui, app: &mut App) {
    let w = Words::of(&app.lang);
    // The Keys page takes every key while it is setting one, Esc included.
    let keys = if app.prefs.keys_edit.busy() { ito_prefs::Keys::default() } else { ito_prefs::Keys::standard(ui, false) };
    let pages = [
        (app.tr("settings.general"), app.tr("settings.general_lead")),
        (app.tr("settings.theme"), app.tr("settings.theme_lead")),
        (app.tr("settings.keys"), app.tr("settings.keys_lead")),
        (w.notify, app.tr("settings.notify_lead")),
    ];
    let index = index(w, &app.lang);
    let file = match app.prefs.state.page {
        KEYS => "keymap.toml",
        NOTIFY => "config.toml",
        _ => "common.toml",
    };
    let nav = ito_prefs::Nav { pages: &pages, index: &index, words: w, file };
    let kw = ito_prefs::KeyWords::of(&app.lang);
    let layers = app.prefs.keymap.layers();
    let mut key_edits: Vec<ito_keymap::Edit> = Vec::new();
    let mut asked: Vec<CommonChange> = Vec::new();
    let mut notify_edits: Vec<(&'static str, String)> = Vec::new();
    let mut webhooks = None;
    let mut test = None;
    let lang = app.lang.clone();
    let tr = |k: &'static str| crate::i18n::tr(&lang, k);
    let prefs = &mut app.prefs;
    let keys_edit = &mut prefs.keys_edit;
    let (notify, drafts) = (&prefs.notify, &mut prefs.notify_drafts);
    let open = &prefs.open;
    let density = prefs.density;
    let mut density_new = None;
    let motion = prefs.motion;
    let mut motion_new = None;
    let (common, themes, base) = (&prefs.common, &prefs.themes, prefs.base.as_deref());
    let out = ito_prefs::show(ui, ito_theme::colors(), &mut prefs.state, &nav, keys, |ui, page, l| match page {
        GENERAL => {
            section(ui, l, tr("settings.display"), |ui| {
                let now = common.language.as_deref().unwrap_or("auto");
                asked.extend(ito_prefs::language_row(ui, l, w, now, ""));
                ito_prefs::sep(ui, l);
                asked.extend(ito_prefs::scale_row(ui, l, w, common.scale_or_one(), ""));
            });
            asked.extend(ito_prefs::clock_card(ui, l, w, &common.clock_or(None)));
            section(ui, l, tr("settings.density"), |ui| {
                ui.label(egui::RichText::new(tr("settings.density_lead")).color(l.c.dim));
                ui.horizontal(|ui| {
                    for d in michi_core::density::Density::ALL {
                        if ui
                            .selectable_label(
                                d == density,
                                tr(match d {
                                    michi_core::density::Density::Compact => "density.compact",
                                    michi_core::density::Density::Standard => "density.standard",
                                    michi_core::density::Density::Cozy => "density.cozy",
                                }),
                            )
                            .clicked()
                            && d != density
                        {
                            density_new = Some(d);
                        }
                    }
                });
            });
            section(ui, l, tr("settings.motion"), |ui| {
                ui.label(egui::RichText::new(tr("settings.motion_lead")).color(l.c.dim));
                ui.horizontal(|ui| {
                    for m in michi_core::motion::Motion::ALL {
                        let word = tr(if m.is_on() { "motion.on" } else { "motion.off" });
                        if ui.selectable_label(m == motion, word).clicked() && m != motion {
                            motion_new = Some(m);
                        }
                    }
                });
            });
            section(ui, l, tr("settings.open"), |ui| {
                ui.label(egui::RichText::new(tr("settings.open_lead")).color(l.c.dim));
                for t in michi_core::openin::Tool::ALL {
                    ui.horizontal(|ui| {
                        ui.label(tr(crate::act::open_word(t)));
                        ui.label(egui::RichText::new(open.command(t)).monospace().color(l.c.dim));
                    });
                }
            });
        }
        THEME => {
            let (t, d, li) = common.theme_choice(None);
            let note = match base {
                Some(b) => format!("{} {}", tr("settings.theme_note"), themes_dir(b).display()),
                None => tr("settings.no_folder").into(),
            };
            let tabs = [tr("view.graph"), tr("view.changes"), tr("view.worktree")];
            asked.extend(ito_prefs::theme_page(ui, l, w, themes, (&t, &d, &li), &note, |ui| preview(ui, tr("settings.sample"), tabs)));
        }
        KEYS => {
            let layer_name = |n: &str| crate::i18n::tr_layer(&lang, n);
            let desc = |d: &str| crate::i18n::tr_desc(&lang, d);
            let table = ito_prefs::KeyTable {
                now: &layers,
                built_in: crate::keymap::Keymap::built_in_layers(),
                noop: "noop",
                own: "keymap.toml",
                layer_name: &layer_name,
                tr: &desc,
                extra: &|_, _| Vec::new(),
                unbound: &[],
                sup: false,
            };
            key_edits.extend(ito_prefs::keys_page(ui, l, kw, &table, keys_edit).unwrap_or_default());
        }
        NOTIFY => {
            let mut n = notify.page();
            let ends = [
                ito_prefs::End { name: tr("notify.done"), color: l.c.done, sound: tr("notify.done_sound") },
                ito_prefs::End { name: tr("notify.failed"), color: l.c.err, sound: tr("notify.failed_sound") },
            ];
            let defaults = crate::notify::NotifyCfg::default().page();
            if let Some(what) = ito_prefs::notify_page(ui, l, w, &ends, &mut n, &defaults, drafts) {
                test = Some((what, n.clone()));
            }
            webhooks = notify.webhooks_of(&n);
            notify_edits = notify.edits(&n);
        }
        _ => {}
    });
    if out.close {
        app.escape();
    }
    if out.open_file {
        let no_folder = crate::i18n::tr(&app.lang, "settings.no_folder");
        app.prefs.open_common(no_folder);
    }
    let no_folder = crate::i18n::tr(&app.lang, "settings.no_folder");
    app.prefs.common_change(asked, no_folder);
    if let Some((what, n)) = test {
        crate::notify::test(&n, what, tr("notify.test_title"), tr("notify.test_body"));
    }
    if let Some(d) = density_new {
        app.prefs.density_set(d, no_folder);
    }
    if let Some(m) = motion_new {
        app.prefs.motion_set(m, no_folder);
    }
    app.prefs.notify_edit(notify_edits, webhooks, no_folder);
    if !key_edits.is_empty() {
        app.prefs.keymap_edit(key_edits, no_folder);
    }
}

/// michi in the theme in force: the title bar's tabs and two branches,
/// small.
fn preview(ui: &mut Ui, sample: &str, tabs: [&str; 3]) {
    let c = ito_theme::colors();
    let (r, _) = ui.allocate_exact_size(vec2(ui.available_width().min(420.0), 96.0), Sense::hover());
    let p = ui.painter_at(r);
    p.rect_filled(r, 8.0, c.bg);
    p.rect_stroke(r, 8.0, Stroke::new(1.0, c.border), StrokeKind::Inside);
    let inner = r.shrink(12.0);
    let mono = FontId::monospace(12.0);
    p.text(inner.left_top(), Align2::LEFT_TOP, sample, FontId::proportional(12.0), c.dim);
    let mut x = inner.left();
    for (k, name) in tabs.into_iter().enumerate() {
        let color = if k == 0 { crate::ui::accent() } else { c.fg };
        let t = p.text(egui::pos2(x, inner.top() + 26.0), Align2::LEFT_TOP, format!("{} {name}", k + 1), mono.clone(), color);
        x = t.right() + 14.0;
    }
    let row = egui::pos2(inner.left(), inner.top() + 52.0);
    let main = p.text(row, Align2::LEFT_TOP, "● main", mono.clone(), c.run);
    p.text(egui::pos2(main.right() + 16.0, row.y), Align2::LEFT_TOP, "● feat", mono, c.wait);
    p.circle_filled(egui::pos2(inner.right() - 8.0, inner.top() + 8.0), 4.0, c.done);
}
