//! Draws the branch, tag, stash and remote palette (picker.rs) with
//! ito-palette: picker.rs matches and builds the rows, the palette draws them
//! and takes the arrows, Enter, Esc and the clicks.

use eframe::egui::{self, Key};
use ito_palette::{Answer, Entry, Style, Tone, Words};

use crate::act;
use crate::app::App;
use crate::picker::{same, Item, Kind, Picker, Step};

/// The palette while it is open: what is typed narrows it, a click on a
/// row is Enter on it, a click outside closes it.
pub fn show(ctx: &egui::Context, app: &mut App) {
    let Some(mut p) = app.picker.take() else { return };
    p.view.query.clone_from(&p.query);
    p.view.selected = p.sel;
    p.sync();
    let c = ito_theme::colors();
    let style = Style {
        panel: c.panel,
        border: c.border,
        text: c.fg,
        dim: c.dim,
        selected_bg: c.blue.gamma_multiply(0.22),
        selected_text: c.fg,
        hit: crate::ui::accent(),
        accent: crate::ui::accent(),
    };
    let renaming = p.renaming.as_ref().map(|old| app.tr("picker.rename").replace("{}", old));
    let (hint, keys) = match p.kind {
        Kind::Branch => ("picker.branch", "picker.branch_keys"),
        Kind::Tag => ("picker.tag", "picker.tag_keys"),
        Kind::Stash => ("picker.stash", "picker.stash_keys"),
        Kind::Move => ("picker.move", "picker.move_keys"),
        Kind::Remote => ("picker.remote", "picker.remote_keys"),
        Kind::Remotes => ("picker.remotes", "picker.remotes_keys"),
        Kind::Flow => ("picker.flow", "picker.flow_keys"),
    };
    let header = match p.kind {
        Kind::Stash => app.tr("picker.stash_for").replace("{wt}", &act::name(&p.wt)),
        Kind::Move => app.tr("picker.move_for").replace("{wt}", &act::name(&p.wt)),
        Kind::Remote => app.tr("picker.remote_for").replace("{wt}", &act::name(&p.wt)),
        Kind::Remotes => app.tr("picker.remotes_for").to_owned(),
        Kind::Flow => app.tr("picker.flow_for").replace("{wt}", &act::name(&p.wt)),
        _ => app.tr("picker.for").replace("{wt}", &act::name(&p.wt)).replace("{at}", &p.at.short()),
    };
    // While renaming the box is the new name: one row stands for it, so that
    // Enter reaches `picker_enter` (the palette closes on Enter with no row).
    let header = renaming.clone().unwrap_or(header);
    let words = Words { hint: app.tr(hint), none: app.tr("picker.none"), header: &header, footer: app.tr(keys) };
    let entries: Vec<Entry<usize>> = if p.renaming.is_some() {
        let mut e = Entry::new(p.query.trim().to_owned(), "", 0);
        e.tone = Tone::Accent;
        vec![e]
    } else {
        (0..p.rows.len()).map(|k| entry(app, &p, k)).collect()
    };
    let answer = ito_palette::show(ctx, &mut p.view, &entries, &style, &words);
    // The box is what was typed, unless the palette's own keys changed it.
    if p.view.query != p.query {
        p.query.clone_from(&p.view.query);
    }
    p.sel = p.view.selected;
    app.picker = Some(p);
    match answer {
        None => {}
        Some(Answer::Pick(k)) => select(app, k, |a| a.picker_enter(false)),
        Some(Answer::Key(m, key, k)) => select(app, k, |a| match (key, m.shift) {
            (Key::Enter, _) => a.picker_enter(true),
            (Key::Delete, _) => a.picker_delete(),
            (Key::F2, _) => a.picker_rename(),
            (Key::U, _) => a.picker_upstream(),
            (Key::R, _) => a.picker_rebase(),
            _ => {}
        }),
        Some(Answer::Close) => {
            app.escape();
        }
    }
}

/// Put the selection on row `k`, then do `act`.
fn select(app: &mut App, k: usize, act: impl FnOnce(&mut App)) {
    if let Some(p) = &mut app.picker {
        p.sel = k;
    }
    act(app);
}

/// Row `k` as the palette's line: its title with the typed characters lit,
/// where the branch is and how far from its upstream, and its commit.
fn entry(app: &App, p: &Picker, k: usize) -> Entry<usize> {
    let r = &p.rows[k];
    let (detail, id) = describe(app, p, &r.item);
    let title = match &r.item {
        Item::Save if p.message().is_empty() => app.tr("picker.save").to_owned(),
        Item::Save => app.tr("picker.save_as").replace("{}", p.message()),
        Item::Step { step: Step::Start(k), .. } => {
            let key = match k {
                michi_core::flow::Kind::Feature => "flow.start_feature",
                michi_core::flow::Kind::Release => "flow.start_release",
                michi_core::flow::Kind::Hotfix => "flow.start_hotfix",
            };
            app.tr(key).replace("{}", p.name())
        }
        Item::Step { step: Step::Finish, branch } => app.tr("flow.finish_of").replace("{}", branch.as_deref().unwrap_or("")),
        item => item.title(),
    };
    let mut e = Entry::new(title, id.unwrap_or_default(), k);
    e.detail = detail;
    e.hits = r.hits.clone();
    if matches!(r.item, Item::Create { .. } | Item::Save | Item::Step { .. }) {
        e.tone = Tone::Accent;
    }
    e
}

/// What a row says after its title, and its commit.
fn describe(app: &App, p: &Picker, item: &Item) -> (String, Option<String>) {
    match item {
        Item::Local { id, at, track, .. } => {
            let mut words = Vec::new();
            match at {
                Some(w) if same(w, &p.wt) => words.push(app.tr("picker.here").to_owned()),
                Some(w) => words.push(app.tr("picker.in").replace("{}", &act::name(w))),
                None => {}
            }
            if let Some((ahead, behind)) = *track {
                if ahead > 0 {
                    words.push(format!("↑{ahead}"));
                }
                if behind > 0 {
                    words.push(format!("↓{behind}"));
                }
            }
            (words.join("  "), Some(id.short()))
        }
        Item::Remote { id, .. } | Item::Tag { id, .. } => (String::new(), Some(id.short())),
        Item::Create { .. } => {
            if p.kind == Kind::Remotes {
                let text = if p.message().is_empty() {
                    app.tr("remote.type_url").to_owned()
                } else {
                    app.tr("remote.add_at").replace("{}", p.message())
                };
                return (text, None);
            }
            let key = if p.kind == Kind::Branch { "picker.create" } else { "picker.tag_at" };
            (app.tr(key).replace("{}", &p.at.short()), None)
        }
        Item::Stash { id, place, .. } => (format!("stash@{{{place}}}"), Some(id.short())),
        Item::Address { url, .. } => (url.clone(), None),
        Item::Place { branch, .. } => (branch.clone().unwrap_or_else(|| app.tr("picker.detached").to_owned()), None),
        Item::Step { step: Step::Start(_), .. } => (app.tr("flow.start_detail").to_owned(), None),
        Item::Step { step: Step::Finish, .. } => (app.tr("flow.finish_detail").to_owned(), None),
        Item::Save | Item::Target { .. } => (String::new(), None),
    }
}
