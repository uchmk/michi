//! The app's state and what the keys do to it. Nothing here draws or runs
//! git, so the keys can be tested without a window or a repository.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use ito_openin::Tool;
use michi_core::clash::{Clashes, Outcome, Side};
use michi_core::claude::{Session, State};
use michi_core::git::{self, Version};
use michi_core::oid::Oid;
use michi_core::paths;
use michi_core::queue::Reviews;
use michi_core::summary::{self, Summary};
use michi_core::track::{Tree, Trees};
use michi_core::unseen::Unseen;
use michi_core::wip::Wip;

use crate::changes::{Changes, Did, Focus, Wish as StageWish};
use crate::conflicts::Conflicts;
use crate::detail::Pane;
use crate::graph::{Folded, Graph};
use crate::picker::{Item as PickItem, Kind as PickKind, Picker};
use crate::prefs::Prefs;
use crate::repo::{Answer, Ask, Reading};
use crate::review::{Ends, Review, Wish};
use crate::work::{Band, Running};

/// The four views, opened by `1` to `4` (docs/design.md); the `4` view
/// only while something clashed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum View {
    Graph,
    Changes,
    Worktree,
    Conflict,
}

impl View {
    pub const ALL: [View; 4] = [View::Graph, View::Changes, View::Worktree, View::Conflict];

    /// The digit that opens it.
    pub fn digit(self) -> char {
        match self {
            View::Graph => '1',
            View::Changes => '2',
            View::Worktree => '3',
            View::Conflict => '4',
        }
    }

    pub fn from_digit(c: char) -> Option<View> {
        View::ALL.into_iter().find(|v| v.digit() == c)
    }

    /// The word-table key of its name.
    pub fn label(self) -> &'static str {
        match self {
            View::Graph => "view.graph",
            View::Changes => "view.changes",
            View::Worktree => "view.worktree",
            View::Conflict => "view.conflict",
        }
    }

    /// The word-table key of the line saying what will be here.
    pub fn empty(self) -> &'static str {
        match self {
            View::Graph => "empty.graph",
            View::Changes => "empty.changes",
            View::Worktree => "empty.worktree",
            View::Conflict => "empty.conflict",
        }
    }

    /// The keys that work in this view: `(keys, word-table key)`, the one to
    /// recommend first. The keybar shows the first few; `?` shows them all.
    pub fn keys(self) -> &'static [(&'static str, &'static str)] {
        match self {
            View::Graph => &[
                ("j k", "key.move"),
                ("Tab", "key.panes"),
                ("Enter", "key.side"),
                ("/", "key.filter"),
                ("?", "key.deep"),
                ("z", "key.fold"),
                ("o", "key.tab"),
                ("l h", "key.open_fold"),
                ("b", "key.branch"),
                ("t", "key.revert"),
                ("R", "key.reset"),
                ("C", "key.cherry_pick"),
                ("m", "key.merge_commit"),
                ("Space D", "key.compare"),
                ("T", "key.tag"),
                ("s", "key.stash"),
                ("M", "key.remotes"),
                ("L", "key.reflog"),
                ("Z", "key.stats"),
                ("B", "key.blame"),
                ("H", "key.file_history"),
                ("Y", "key.copy_patch"),
                ("#", "key.copy_hash"),
                ("[ ]", "key.go_parent"),
                ("U", "key.flow"),
                ("I", "key.issue"),
                ("Ctrl+V", "key.apply_patch"),
                ("w", "key.ignore_space"),
                ("x", "key.bisect_mark"),
                ("u", "key.bisect_skip"),
                ("r", "key.bisect_run"),
                ("X", "key.bisect_reset"),
                ("O", "key.image_mode"),
                ("g", "key.go"),
                ("y", "key.copy"),
                ("Esc", "key.back"),
            ],
            View::Changes => &[
                ("Space", "key.stage"),
                ("c", "key.commit"),
                ("l h", "key.lines"),
                ("Tab", "key.pane_list"),
                ("a", "key.stage_all"),
                ("j k", "key.select"),
                ("v", "key.range"),
                ("s", "key.split"),
                ("n N", "key.next_change"),
                ("w", "key.ignore_space"),
                ("O", "key.image_mode"),
                ("B", "key.blame"),
                ("H", "key.file_history"),
                ("e", "key.editor_line"),
                ("x", "key.ext_diff"),
                ("[ ]", "key.worktree"),
                ("g", "key.go"),
                ("g+d", "key.discard"),
                ("g+a", "key.amend"),
                ("g+i", "key.ignore"),
                ("y", "key.copy"),
                ("Enter", "key.side"),
                ("Esc", "key.back"),
            ],
            View::Worktree => &[
                ("j k", "key.select"),
                ("o", "key.tab"),
                ("O", "key.pr"),
                ("I", "key.issue"),
                ("Q", "key.pr_create"),
                ("t e E", "key.open_in"),
                ("r", "key.reorder"),
                ("M", "key.rebase_trunk"),
                ("Enter", "key.in_graph"),
                ("m", "key.take_main"),
                ("l", "key.land"),
                ("s", "key.sort_land"),
                ("x", "key.remove"),
                ("d", "key.branch_diff"),
                ("c", "key.tidy"),
                ("T", "key.trial"),
                ("L", "key.reflog"),
                ("Z", "key.stats"),
                ("Y", "key.turns"),
                ("u", "key.submodules"),
                ("v", "key.move_changes"),
                ("U", "key.flow"),
                ("R", "key.ready"),
                ("g", "key.go"),
                ("y", "key.copy"),
            ],
            View::Conflict => &[
                ("] [", "key.place"),
                ("Tab", "key.pane_list"),
                ("o", "key.ours"),
                ("t", "key.theirs"),
                ("b B", "key.both"),
                ("Enter", "key.cf_reused"),
                ("e", "key.cf_edit"),
                ("d a r", "key.cf_strip"),
                ("w", "key.cf_external"),
                ("E", "key.cf_editor"),
                ("g", "key.go"),
                ("g+h", "key.cf_handoff"),
                ("g+x", "key.cf_forget"),
                ("y", "key.copy"),
                ("Ctrl+Enter", "key.cf_commit"),
                ("PageDown PageUp Ctrl+D Ctrl+U Alt+J Alt+K", "key.scroll"),
            ],
        }
    }
}

/// The keys that work in every view.
pub const GLOBAL_KEYS: [(&str, &str); 9] = [
    ("1 2 3 4", "key.views"),
    ("N", "key.new_work"),
    ("W", "key.waiting"),
    ("f", "key.fetch"),
    ("p", "key.pull"),
    ("P", "key.push"),
    ("F", "key.push_over"),
    ("S", "key.sync_help"),
    ("K", "key.auth"),
];

/// The keys of the branch review, drawn over the worktree table.
pub const REVIEW_KEYS: [(&str, &str); 13] = [
    ("j k", "key.file"),
    ("Tab", "key.pane_list"),
    ("J K", "key.line"),
    ("n", "key.note"),
    ("g", "key.hand_notes"),
    ("Space", "key.seen"),
    ("u", "key.uncommitted"),
    ("s", "key.since_last"),
    ("y Y", "key.turns_step"),
    ("m", "key.take_main"),
    ("Enter", "key.side"),
    ("PageDown PageUp Ctrl+D Ctrl+U Alt+J Alt+K", "key.scroll"),
    ("Esc", "key.back"),
];

/// The keys of the question band (`y` / `n` / `Esc`), over any view.
pub const BAND_KEYS: [(&str, &str); 3] = [("y", "key.yes"), ("n", "key.no"), ("Esc", "key.back")];
/// The keys of the clean-up list (`c` over the worktree table).
pub const TIDY_KEYS: [(&str, &str); 5] =
    [("j k", "key.move"), ("Space", "key.pick"), ("a", "key.pick_all"), ("Enter", "key.run"), ("Esc", "key.back")];
/// The keys of the pre-merge checklist (`R` over the worktree table).
pub const READY_KEYS: [(&str, &str); 1] = [("Esc", "key.back")];
/// The keys of the sign-in help (`K`).
pub const AUTH_KEYS: [(&str, &str); 2] = [("1-9", "key.copy_command"), ("Esc", "key.back")];
/// The keys of the reflog list (`L`).
pub const REFLOG_KEYS: [(&str, &str); 5] =
    [("j k", "key.move"), ("Enter", "key.bring_back"), ("g", "key.show_commit"), ("r", "key.raw_wording"), ("Esc", "key.back")];
/// The keys of the statistics screen (`I`).
pub const STATS_KEYS: [(&str, &str); 4] = [("j k", "key.move"), ("r", "key.stats_range"), ("w", "key.stats_grain"), ("Esc", "key.back")];
/// The keys of the blame screen (`B` over the graph or the changes).
pub const BLAME_KEYS: [(&str, &str); 6] = [
    ("j k", "key.move"),
    ("B H Tab", "key.blame_tab"),
    ("e", "key.editor_line"),
    ("Enter", "key.in_graph_commit"),
    ("PageDown PageUp Ctrl+D Ctrl+U Alt+J Alt+K", "key.page_rows"),
    ("Esc", "key.back"),
];
/// The keys of the trial-merge screen (`T`).
pub const TRIAL_KEYS: [(&str, &str); 5] =
    [("j k", "key.move"), ("Space", "key.pick"), ("u", "key.unpick"), ("Enter", "key.run"), ("Esc", "key.back")];
/// The keys of the rebase screen (`r`).
pub const REORDER_KEYS: [(&str, &str); 6] = [
    ("j k", "key.select"),
    ("J K", "key.shift_row"),
    ("p s f d", "key.row_action"),
    ("r", "key.reword"),
    ("Enter", "key.run"),
    ("Esc", "key.back"),
];

/// What the window shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Main,
    Settings,
    Help,
}

/// The repository the window is on, as far as it has been read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Repo {
    Loading,
    Ready(Summary),
    Failed(git::Error),
}

/// A worktree whose Claude was handed a conflict: see [`App::handed`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Handed {
    pub wt: PathBuf,
    /// Seen running since it was handed.
    pub ran: bool,
    /// Reads that found it stopped without ever seeing it run.
    pub polls: u32,
    /// It was handed a conflict (not notes): look at the markers when it stops.
    pub conflict: bool,
}

pub struct App {
    pub view: View,
    pub mode: Mode,
    pub prefs: Prefs,
    /// The column widths the user dragged.
    pub sizes: crate::sizes::Sizes,
    /// The authors' pictures fetched so far.
    pub avatars: crate::avatar::Avatars,
    /// The pictures of the image diff.
    pub images: crate::images::Images,
    /// The language in force (`en`, `ja`), from common.toml or the OS.
    pub lang: String,
    /// The folder the window was opened on.
    pub dir: PathBuf,
    pub repo: Repo,
    /// The git on the PATH; `None` until it has answered.
    pub git: Option<Version>,
    /// The two ends of the reading thread.
    pub reading: Option<Reading>,
    pub graph: Graph,
    /// The selected commit in full, on the right.
    pub detail: Pane,
    /// The Claude sessions in each worktree that has some; `None` when
    /// tsumugi is not there (or not asked yet).
    pub claude: Option<crate::repo::Sessions>,
    /// tsumugi has been asked at least once (so `claude: None` means it is not there).
    pub claude_asked: bool,
    /// `(owner, repo)` of the GitHub remote, for `#123` links (#44).
    pub issue_slug: Option<(String, String)>,
    /// The worktrees and how they stand, for the worktree table.
    pub trees: Trees,
    /// Each worktree's uncommitted change, those with only files too big to
    /// count too (the graph shows the others).
    pub wips: Vec<Wip>,
    /// `Name <email>` of the worktrees that commit as someone other than the
    /// rest (empty when all are the same).
    pub identities: Vec<(PathBuf, String)>,
    /// Which worktrees would conflict with which.
    pub clashes: Clashes,
    /// The commits not seen yet, per worktree.
    pub unseen: Unseen,
    /// Where each worktree's branch stands in review.
    pub reviews: Reviews,
    /// The worktrees whose Claude was handed a conflict, until it stops.
    pub handed: Vec<Handed>,
    /// Whether the window is in front (a notification is told only when it is not).
    pub front: bool,
    /// The last failure told in a toast, for the bug report (F12).
    pub last_error: Option<String>,
    /// The worktree table's selected row.
    pub wt_sel: usize,
    /// The worktree table sorted by the suggested landing order (`s`).
    pub wt_by_land: bool,
    /// The worktrees as git listed them, before `wt_by_land` sorts `trees`.
    trees_raw: Trees,
    /// The branch review (`d`), over the worktree table.
    pub review: Option<Review>,
    /// The notes of closed reviews, by worktree, until they are handed over.
    pub kept_notes: Vec<(PathBuf, Vec<michi_core::notes::Note>)>,
    /// The `2` view: one worktree's files to stage and commit.
    pub changes: Changes,
    /// The `4` view: the worktrees stopped on a clash, and their places.
    pub conflicts: Conflicts,
    /// The view of the frame before: the `2` view reads its files again
    /// when it comes in sight.
    last_view: View,
    /// The side column's groups that are open (`b:feat`, `r:origin`).
    pub side_open: HashSet<String>,
    /// A commit clicked in the side column that is not read yet: read on
    /// until it is, then select it.
    seeking: Option<Oid>,
    /// The worktree whose uncommitted change the detail shows.
    detail_wip: Option<PathBuf>,
    /// A new window title, for the frame to send once.
    pub new_title: Option<String>,
    /// One line at the foot, until its instant.
    pub toast: Option<(String, Instant)>,
    /// The fetch, pull or push that runs.
    pub net: Option<Running>,
    /// When the repository last fetched (`FETCH_HEAD`'s time).
    pub last_fetch: Option<SystemTime>,
    /// The fetch that runs was started by the timer (#70): it says nothing unless it fails.
    pub auto_job: bool,
    /// When the timer last started a fetch.
    pub auto_try: Option<SystemTime>,
    /// Fetches the timer started in a row that failed.
    pub auto_failures: u32,
    /// The last fetch failed: the keybar says so, small.
    pub fetch_failed: bool,
    /// A job waiting for a yes, under the title bar.
    pub band: Option<Band>,
    /// A commit or a branch being dragged toward a worktree (#67).
    pub dnd: Option<michi_core::dnd::Source>,
    /// The small menu a drop opened: what was dropped, on which worktree, where.
    pub drop_menu: Option<crate::dnd_ui::DropMenu>,
    /// The window's worktree is on no branch: what was read of it (#109).
    pub detached: Option<crate::strip::Strip>,
    /// The window's worktree is a sparse checkout (#31); None when it is not
    /// (or not read yet).
    pub sparse: Option<michi_core::sparse::Info>,
    /// A bisect runs in the window's worktree: how far it is; None when it
    /// does not (or is not read yet).
    pub bisect: Option<michi_core::bisect::State>,
    /// The bad and the good commit marked in the graph for a bisect not
    /// started yet.
    pub bisect_marks: crate::bisect::Marks,
    /// rerere was asked about (or read as said) in this session.
    pub rerere_asked: bool,
    /// The branch, tag or stash palette, over the graph.
    pub picker: Option<Picker>,
    /// The new-work form while it is open.
    pub new_work: Option<crate::newwork::NewWork>,
    /// The pull request form (`G`).
    pub pr_form: Option<crate::prform::PrForm>,
    /// The answer being written to a waiting Claude (`W`).
    pub reply: Option<crate::reply::Reply>,
    /// The keys of the waits already told of; `None` until tsumugi was heard once.
    pub waits_told: Option<Vec<(u64, u64)>>,
    /// The state each Claude session was last seen in, to tell a turn that
    /// ended; `None` until tsumugi was heard once.
    pub turn_states: Option<std::collections::HashMap<u64, michi_core::claude::State>>,
    /// The clean-up list, over the worktree table.
    pub tidy: Option<crate::tidy::Tidy>,
    /// The moves of branches done in this window, for Undo and Redo (#64).
    pub journal: michi_core::undo::Journal,
    /// An undo or redo is on its way: another waits for it.
    pub undoing: bool,
    pub reflog: Option<crate::reflog::Reflog>,
    pub stats: Option<crate::stats::Statistics>,
    /// The graph's legend (#118), while open; any key closes it.
    pub legend: bool,
    /// The Git flow sheet, while open.
    pub cheat: Option<crate::cheat_ui::Cheat>,
    /// A `g` or `y` was typed: its window waits for the second key (#133).
    pub prefix: Option<crate::prefix::Prefix>,
    /// Which pane of the graph view has the keys (Tab moves it).
    pub pane: crate::panes::GraphPane,
    /// The side column's cursor: an index into `side_ui::rows`.
    pub side_cur: usize,
    /// The cursor moved: the side column scrolls it into view.
    pub side_follow: bool,
    /// The practice repository being made (#117): its folder, or why not.
    pub practicing: Option<std::sync::mpsc::Receiver<Result<std::path::PathBuf, String>>>,
    pub ready: Option<crate::ready::Ready>,
    /// The sign-in help panel (`K`), while open.
    pub auth: Option<crate::auth::Auth>,
    /// The last login or network failure of a fetch, push or clone, and the
    /// clone's address when it was a clone: what `K` explains.
    pub auth_failed: Option<(michi_core::auth::Kind, Option<String>)>,
    /// What the first-run guide has to tell (#116): found once at startup.
    pub problems: Vec<michi_core::setup::Problem>,
    /// Whether the first-run guide is on screen. It takes no keys: work goes on under it.
    pub guide_open: bool,
    pub blame: Option<crate::blame::Blame>,
    /// Text for the clipboard, taken by the window on its next frame.
    pub clipboard: Option<String>,
    /// The trial-merge screen (`T` over the worktree table), while open.
    pub trial: Option<crate::trial::TrialScreen>,
    pub reorder: Option<crate::reorder::Reorder>,
    /// The stash clicked in the side column, and the commit the graph had
    /// selected then: the detail shows the stash until that moves.
    pub side_stash: Option<(Oid, Option<Oid>)>,
    pub quit: bool,
    /// The command palette while it is open.
    pub cmd: Option<ito_palette::View>,
    /// The repository screen while it is open.
    pub repos: Option<ito_palette::View>,
    /// The repositories michi was opened on, newest first.
    pub recent: michi_core::recent::Recent,
    /// A folder the repository screen picked, for the window to read and switch to.
    pub open_req: Option<PathBuf>,
    /// A folder being read before the switch.
    pub opening: Option<crate::repos::Opening>,
    /// A `git clone` running on a thread, started from the repository screen.
    pub cloning: Option<crate::repos::Cloning>,
    /// The names of the window's tabs and which is in front, set by the window each frame (#45).
    pub tab_labels: Vec<String>,
    pub tab_active: usize,
    /// What a key or a click asked of the tabs, for the window to do.
    pub tab_req: Option<michi_core::tabs::Op>,
    /// The key list's first line, and how much of it the last frame showed.
    pub help_scroll: usize,
    pub help_drawn: std::cell::Cell<Option<ito_help::Drawn>>,
}

impl App {
    pub fn new(prefs: Prefs, dir: PathBuf) -> Self {
        let base = prefs.base.clone();
        let mut app = Self {
            view: View::Graph,
            mode: Mode::Main,
            sizes: crate::sizes::Sizes::load(crate::prefs::window_dir(prefs.base.as_deref()).as_deref()),
            avatars: Default::default(),
            images: Default::default(),
            prefs,
            lang: "en".into(),
            dir,
            repo: Repo::Loading,
            git: None,
            reading: None,
            graph: Graph::default(),
            detail: Pane::default(),
            claude: None,
            claude_asked: false,
            issue_slug: None,
            trees: Trees::default(),
            wips: Vec::new(),
            identities: Vec::new(),
            clashes: Clashes::default(),
            unseen: Unseen::default(),
            reviews: Reviews::default(),
            handed: Vec::new(),
            front: true,
            last_error: None,
            wt_sel: 0,
            wt_by_land: false,
            trees_raw: Trees::default(),
            review: None,
            kept_notes: Vec::new(),
            changes: Changes::default(),
            conflicts: Conflicts::default(),
            last_view: View::Graph,
            side_open: HashSet::new(),
            seeking: None,
            detail_wip: None,
            new_title: None,
            toast: None,
            net: None,
            last_fetch: None,
            auto_job: false,
            auto_try: None,
            auto_failures: 0,
            fetch_failed: false,
            band: None,
            dnd: None,
            drop_menu: None,
            detached: None,
            sparse: None,
            bisect: None,
            bisect_marks: crate::bisect::Marks::default(),
            rerere_asked: false,
            picker: None,
            tab_labels: Vec::new(),
            tab_active: 0,
            tab_req: None,
            new_work: None,
            pr_form: None,
            reply: None,
            waits_told: None,
            turn_states: None,
            tidy: None,
            journal: Default::default(),
            undoing: false,
            reflog: None,
            stats: None,
            legend: false,
            cheat: None,
            prefix: None,
            pane: crate::panes::GraphPane::Tree,
            side_cur: 0,
            side_follow: false,
            practicing: None,
            ready: None,
            auth: None,
            auth_failed: None,
            problems: Vec::new(),
            guide_open: false,
            blame: None,
            clipboard: None,
            trial: None,
            reorder: None,
            side_stash: None,
            quit: false,
            cmd: None,
            repos: None,
            recent: crate::repos::load(base.as_deref()),
            open_req: None,
            opening: None,
            cloning: None,
            help_scroll: 0,
            help_drawn: std::cell::Cell::new(None),
        };
        app.set_language();
        app
    }

    /// The language common.toml names, else the OS's.
    pub fn set_language(&mut self) {
        let os = ito_common::os_language();
        self.lang = ito_i18n::resolve(None, self.prefs.common.language.as_deref(), os.as_deref(), &crate::i18n::AVAILABLE);
    }

    /// The words for `key` in the language in force.
    pub fn tr(&self, key: &'static str) -> &'static str {
        crate::i18n::tr(&self.lang, key)
    }

    /// What git refused with, for a toast (#108): the cause and the next step
    /// when the refusal is a known one, git's own words kept after them as the
    /// detail; otherwise git's words as they are.
    pub fn git_says(&self, e: &git::Error) -> String {
        match michi_core::explain::explain_error(e) {
            Some(r) => {
                self.tr("gerr.fmt").replace("{cause}", self.tr(r.cause)).replace("{next}", self.tr(r.next)).replace("{raw}", &e.to_string())
            }
            None => e.to_string(),
        }
    }

    /// What a worktree in the middle of `d` is doing, for the texts' `{doing}`.
    /// The Japanese texts leave out the spaces around it, so that a Japanese
    /// word sits close (`マージの途中`) and an ASCII one gets them here
    /// (`cherry-pick の途中`).
    pub fn doing_word(&self, d: michi_core::branch::Doing) -> String {
        use michi_core::branch::Doing;
        let word = self.tr(match d {
            Doing::Merge => "doing.merge",
            Doing::Rebase => "doing.rebase",
            Doing::CherryPick => "doing.cherry",
            Doing::Revert => "doing.revert",
            Doing::Bisect => "doing.bisect",
        });
        if self.lang == "ja" && word.is_ascii() {
            format!(" {word} ")
        } else {
            word.to_owned()
        }
    }

    pub fn say(&mut self, text: impl Into<String>) {
        self.say_for(text, 4);
    }

    pub fn say_for(&mut self, text: impl Into<String>, secs: u64) {
        self.toast = Some((text.into(), Instant::now() + Duration::from_secs(secs)));
    }

    /// How long until the toast goes, if there is one.
    pub fn toast_left(&self) -> Option<Duration> {
        self.toast.as_ref().and_then(|(_, until)| until.checked_duration_since(Instant::now()).filter(|d| !d.is_zero()))
    }

    /// Fold in what the reading thread has sent since the last frame.
    pub fn pump(&mut self) {
        let got: Vec<Answer> = match &self.reading {
            Some(r) => r.answers.try_iter().collect(),
            None => return,
        };
        for a in got {
            self.take(a);
        }
        self.seek();
        self.changes_sync();
    }

    /// The stash the detail shows in place of the selected commit: the one
    /// the stash palette has selected, else the one clicked in the side
    /// column while the graph's selection stays where it was.
    pub fn stash_shown(&self) -> Option<Oid> {
        if let Some(PickItem::Stash { id, .. }) = self.picker.as_ref().and_then(|p| p.selected()) {
            return Some(*id);
        }
        let (id, at) = self.side_stash?;
        let there = self.graph.setup().refs.stashes.iter().any(|s| s.id == id);
        (there && at == self.graph.selected()).then_some(id)
    }

    /// A stash in the side column is clicked: the detail shows it.
    pub fn show_stash(&mut self, id: Oid) {
        self.side_stash = Some((id, self.graph.selected()));
    }

    /// `w`: the diffs ignore whitespace, or no longer do. What is shown is
    /// read again; the file in the `2` view loses its lines (a diff that
    /// ignores whitespace is for looking at).
    pub fn toggle_ignore_space(&mut self) {
        let on = michi_core::space::toggle();
        self.detail.forget();
        let wish = self.changes.reread_file();
        self.stage_wish(wish);
        let key = if on { "diff.space_on" } else { "diff.space_off" };
        self.say(self.tr(key));
    }

    /// Asks for the selected commit (or the stash shown) in full when the
    /// selection has changed, and counts a trunk merge's commits; whether
    /// it had changed.
    pub fn sync_detail(&mut self) -> bool {
        let stash = self.stash_shown();
        let id = stash.or(self.graph.selected());
        // The same worktree's change read anew: the detail keeps its place.
        let wip = self.graph.selected_wip().filter(|_| stash.is_none()).map(|w| w.path.clone());
        let changed = if wip.is_some() && wip == self.detail_wip { self.detail.refresh(id) } else { self.detail.select(id) };
        self.detail_wip = wip;
        // A commit picked by a key or the mouse is seen now.
        if let (true, Some(id)) = (self.graph.take_picked(), id) {
            if self.unseen.remove(&id) {
                if let Some(r) = &self.reading {
                    r.seen(id);
                }
            }
        }
        if changed {
            if let (Some(id), Some(r)) = (id, &self.reading) {
                if stash.is_some() {
                    r.stash(id);
                } else {
                    // A merge's diff (the whole branch's) comes in stage 2.
                    let patch = self.graph.find_commit(&id).is_none_or(|c| c.parents.len() < 2);
                    r.detail(id, patch);
                }
            }
        }
        if let Some(id) = id.filter(|_| stash.is_none()) {
            let read = self.graph.read();
            if self.detail.merged(id, read).is_none() {
                let commits = self.graph.merged_by(&id);
                self.detail.set_merged(id, read, commits);
            }
        }
        changed
    }

    /// A row of the side column is clicked: select its commit, reading on
    /// to it when it is further down than read.
    pub fn go_to(&mut self, id: Oid) {
        self.seeking = None;
        if !self.graph.select_id(&id) {
            self.seeking = Some(id);
            self.seek();
        }
    }

    /// Reads on towards the commit the side column wants.
    fn seek(&mut self) {
        let Some(id) = self.seeking else { return };
        if self.graph.select_id(&id) {
            self.seeking = None;
        } else if self.graph.done() || self.graph.error().is_some() {
            self.seeking = None;
            let text = self.tr("side.missing").replace("{}", &id.short());
            self.say(text);
        } else if self.graph.ask_more() {
            self.ask(Ask::More);
        }
    }

    pub fn take(&mut self, answer: Answer) {
        match answer {
            Answer::Version(Ok(v)) => {
                self.git = Some(v);
                if !v.is_enough() {
                    let text = self.tr("git.old").replace("{v}", &v.to_string()).replace("{min}", &git::MIN.to_string());
                    self.say_for(text, 10);
                }
            }
            // The summary that follows says why (git missing, mostly).
            Answer::Version(Err(_)) => {}
            Answer::Problems(p) => {
                // The guide opens by itself once, when something is wrong.
                self.guide_open = !p.is_empty();
                self.problems = p;
            }
            Answer::Summary(Ok(s)) => {
                self.new_title = Some(format!("{} — michi", s.name));
                self.remember_repo(&s.top);
                self.repo = Repo::Ready(s);
            }
            Answer::IssueSlug(slug) => self.issue_slug = slug,
            Answer::Summary(Err(e)) => {
                let word = self.tr(failed_word(&e));
                let path = self.dir.display();
                let text = match &e {
                    git::Error::Git(d) | git::Error::Io(d) | git::Error::Told { line: d, .. } => format!("{word}: {d} — {path}"),
                    _ => format!("{word} — {path}"),
                };
                self.say_for(text, 10);
                self.repo = Repo::Failed(e);
            }
            Answer::Setup { gen, setup } => self.graph.start(gen, *setup),
            Answer::Commits { gen, batch, done } => self.graph.commits(gen, batch, done),
            Answer::Total { gen, count } => self.graph.total(gen, count),
            Answer::Failed { gen, error } => {
                let text = self.tr("history.failed").replace("{}", &error.to_string());
                self.say_for(text, 10);
                self.graph.failed(gen, error);
            }
            Answer::Detail { id, result } => self.detail.take(id, result),
            Answer::Identities(all) => self.identities = all,
            Answer::Wips(wips) => {
                self.graph.set_wips(wips.iter().filter(|w| !w.files.is_empty()).cloned().collect());
                self.wips = wips;
                self.review_sync();
            }
            Answer::Trees(trees) => {
                self.trees_raw = trees.clone();
                let trees = self.sorted(trees);
                // The selection stays on its worktree when the list changes.
                let path = self.trees.trees.get(self.wt_sel).map(|t| t.wt.path.clone());
                self.wt_sel = path.and_then(|p| trees.trees.iter().position(|t| t.wt.path == p)).unwrap_or(0);
                // The title follows a switch made here or anywhere else, and
                // worktrees added or taken away.
                if let Repo::Ready(s) = &mut self.repo {
                    s.worktrees = trees.trees.len();
                    let top = paths::key(&s.top);
                    if let Some(t) = trees.trees.iter().find(|t| paths::key(&t.wt.path) == top) {
                        s.head = summary::head_of(&t.wt);
                    }
                }
                self.trees = trees;
                self.detached_sync();
                self.sparse_sync();
                self.bisect_sync();
                self.review_sync();
            }
            Answer::Claude(c) => {
                self.claude = c;
                self.claude_asked = true;
                self.handed_stopped();
                self.waits_changed();
                self.turns_changed();
            }
            Answer::Clashes(c) => {
                self.clashes = c;
                self.resort();
            }
            Answer::Conflicts(c) => {
                let had = !self.conflicts.all.is_empty();
                self.conflicts.set(c);
                // All committed or called off: nothing to show here.
                if had && self.conflicts.all.is_empty() && self.view == View::Conflict {
                    self.view = View::Graph;
                }
            }
            Answer::Unseen(u) => self.unseen = u,
            Answer::Reviews(r) => self.reviews = r,
            Answer::ReviewList { gen, result, checks } => {
                let wish = self.review.as_mut().and_then(|r| r.take_list(gen, result.map(|l| *l), checks));
                self.wish(wish);
            }
            Answer::ReviewFile { gen, result } => {
                if let Some(r) = self.review.as_mut() {
                    r.take_file(gen, result.map(|c| *c));
                }
            }
            Answer::Reread => {
                if self.view == View::Changes {
                    let wish = self.changes.refresh();
                    self.stage_wish(wish);
                }
            }
            Answer::ChangesList { gen, result } => {
                let wishes = self.changes.take_list(gen, result.map(|l| *l));
                self.stage_wish(wishes);
            }
            Answer::ChangesFile { gen, result } => self.changes.take_file(gen, result.map(|d| d.map(|d| *d))),
            Answer::ChangesLast { gen, result } => self.changes.take_last(gen, result),
            Answer::ChangesHelp { gen, help } => self.changes.take_help(gen, help),
            Answer::ChangesDone { wt, commit, result } => {
                match &result {
                    Ok(Did::Committed(id)) => {
                        let text = self.tr("changes.committed").replace("{}", &id.short());
                        self.say(text);
                    }
                    Ok(Did::Discarded { path, untracked, lines, saved }) => {
                        let key = match (untracked, lines) {
                            (true, _) => "changes.deleted",
                            (false, 0) => "changes.discarded",
                            _ => "changes.discarded_lines",
                        };
                        let text = self.tr(key).replace("{file}", path).replace("{n}", &lines.to_string());
                        self.say_for(text.replace("{saved}", michi_core::backup::short(saved)), 10);
                    }
                    Ok(Did::Ignored { path, line, already }) => {
                        let key = if *already { "changes.ignored_already" } else { "changes.ignored" };
                        let text = self.tr(key).replace("{file}", path).replace("{line}", line);
                        self.say_for(text, 8);
                    }
                    Ok(Did::Staged) => {}
                    Err(e) => {
                        let text = self.tr("changes.failed").replace("{}", &self.git_says(e));
                        self.say_for(text, 8);
                        if let git::Error::Told { all, .. } = e {
                            self.changes.tell(&wt, commit, all.clone());
                        }
                    }
                }
                let wish = self.changes.take_done(&wt, commit, result.is_ok());
                self.stage_wish(wish);
            }
            Answer::NetProgress(p) => self.net_progress(p),
            Answer::NetDone { job, result } => self.net_done(job, result),
            Answer::ActDone { task, result } => self.act_done(task, result),
            Answer::Journaled(entry) => self.journal.record(entry),
            Answer::Fetched(at) => self.last_fetch = at,
        }
    }

    /// Sends what the `2` view wants to the reading thread.
    fn stage_wish(&self, wishes: impl IntoIterator<Item = StageWish>) {
        if let Some(r) = &self.reading {
            for w in wishes {
                r.changes(w);
            }
        }
    }

    /// The worktrees the `2` view can show: those with a folder.
    pub fn change_trees(&self) -> impl Iterator<Item = &Tree> {
        self.trees.trees.iter().filter(|t| !t.wt.bare && t.wt.prunable.is_none())
    }

    /// Whether a Claude is writing in the worktree the `2` view shows.
    pub fn claude_writing(&self) -> bool {
        let Some(key) = self.changes.wt.as_deref().map(paths::key) else { return false };
        let sessions = self.claude.iter().flatten().find(|(p, _)| paths::key(p) == key);
        sessions.is_some_and(|(_, s)| s.iter().any(|s| s.is_agent() && s.state == State::Running))
    }

    /// Each frame: while the `2` view is in sight it has a worktree (the
    /// window's own, else the first), whose files are read again when the
    /// view comes in sight; a yes to write beside a Claude goes once it
    /// stops.
    pub fn changes_sync(&mut self) {
        let entering = self.view == View::Changes && self.last_view != View::Changes;
        self.last_view = self.view;
        if self.view != View::Changes {
            return;
        }
        let top = match &self.repo {
            Repo::Ready(s) => Some(s.top.clone()),
            _ => None,
        };
        let shown = self.changes.wt.as_deref().map(paths::key);
        let trees: Vec<_> = self.change_trees().map(|t| (paths::key(&t.wt.path), t.wt.path.clone())).collect();
        let gone = match &shown {
            None => true,
            Some(key) => !trees.is_empty() && !trees.iter().any(|(k, _)| k == key),
        };
        if gone {
            let top_key = top.as_deref().map(paths::key);
            let pick = trees.iter().find(|(k, _)| Some(k) == top_key.as_ref()).or(trees.first()).map(|(_, p)| p.clone()).or(top);
            if let Some(wt) = pick {
                let wish = self.changes.open(wt);
                self.stage_wish(wish);
            }
        } else if entering {
            let wish = self.changes.refresh();
            self.stage_wish(wish);
        }
        if self.changes.claude_ok && !self.claude_writing() {
            self.changes.claude_stopped();
        }
    }

    /// `[` `]`: the `2` view shows the worktree before or after.
    fn changes_step(&mut self, by: i64) {
        let trees: Vec<PathBuf> = self.change_trees().map(|t| t.wt.path.clone()).collect();
        if trees.len() < 2 {
            let text = self.tr("changes.one_worktree");
            self.say(text);
            return;
        }
        let key = self.changes.wt.as_deref().map(paths::key);
        let at = trees.iter().position(|p| Some(paths::key(p)) == key).unwrap_or(0) as i64;
        let next = (at + by).rem_euclid(trees.len() as i64) as usize;
        let wish = self.changes.open(trees[next].clone());
        self.stage_wish(wish);
    }

    /// The `2` view shows the worktree at `wt` (a click on its name).
    pub fn changes_open(&mut self, wt: PathBuf) {
        if self.changes.wt.as_deref().map(paths::key) != Some(paths::key(&wt)) {
            let wish = self.changes.open(wt);
            self.stage_wish(wish);
        }
    }

    /// The `2` view selects its `k`th file (a click).
    /// A Shift+click (`range`) or Ctrl+click on file `k`.
    pub fn changes_click_multi(&mut self, k: usize, range: bool) {
        let wish = if range { self.changes.click_range(k) } else { self.changes.click_toggle(k) };
        self.stage_wish(wish);
    }

    pub fn changes_select(&mut self, k: usize) {
        let wish = self.changes.select(k);
        self.stage_wish(wish);
    }

    /// The files the `2` tab counts: the shown worktree's as read, else its
    /// uncommitted change's (the window's own worktree's before one is
    /// shown).
    pub fn changes_count(&self) -> usize {
        if let (View::Changes, Some(l)) = (self.view, self.changes.lists()) {
            let paths: HashSet<&str> = l.staged.iter().chain(&l.unstaged).map(|f| f.path.as_str()).collect();
            return paths.len();
        }
        let wt = self.changes.wt.clone().or_else(|| match &self.repo {
            Repo::Ready(s) => Some(s.top.clone()),
            _ => None,
        });
        let Some(key) = wt.as_deref().map(paths::key) else { return 0 };
        self.wips.iter().find(|w| paths::key(&w.path) == key).map_or(0, |w| w.files.len())
    }

    /// Ctrl+Enter: commit what is staged in the `2` view. A worktree in
    /// the middle of a merge or the like commits it in the `4` view.
    pub fn changes_commit(&mut self) {
        let wt = self.changes.wt.clone().or_else(|| match &self.repo {
            Repo::Ready(s) => Some(s.top.clone()),
            _ => None,
        });
        if let Some((wt, d)) = wt.and_then(|wt| self.doing_in(&wt).map(|d| (wt, d))) {
            let text = self.tr("changes.commit_doing").replace("{wt}", &crate::act::name(&wt)).replace("{doing}", &self.doing_word(d));
            self.say_for(text, 6);
            return;
        }
        let claude = self.claude_writing();
        self.changes.co_author = self.prefs.co_author.clone();
        self.changes.guard = self.prefs.guard.clone();
        let done = self.changes.commit(claude);
        self.changes_did(done);
    }

    /// A commit the hooks refused is run again (`r` over what git said, or
    /// its button): the message and the staged files are as they were.
    pub fn changes_run_again(&mut self) {
        if self.changes.told.as_ref().is_some_and(|t| t.commit) {
            self.changes_commit();
        }
    }

    /// The message box has the keys: what helps writing a message is read
    /// once, and Alt+Up / Alt+Down in it step through the past messages.
    pub fn changes_help(&mut self) {
        let wish = self.changes.ask_help();
        self.stage_wish(wish);
    }

    /// A folder of the `2` view's tree staged whole (taken out of the index
    /// whole on the staged side).
    pub fn changes_dir(&mut self, side: michi_core::stage::Side, dir: &str) {
        let claude = self.claude_writing();
        let done = Ok(self.changes.toggle_dir(side, dir, claude));
        self.changes_did(done);
    }

    /// Sends what the `2` view wants, or says why it could not.
    fn changes_did(&mut self, done: Result<impl IntoIterator<Item = StageWish>, &'static str>) {
        match done {
            Ok(wishes) => self.stage_wish(wishes),
            Err(word) => {
                let text = self.tr(word);
                self.say(text);
            }
        }
    }

    /// The `2` view's own keys. While a write waits for a yes, `y` gives it
    /// and `n` takes it back; the other keys do nothing.
    fn changes_key(&mut self, c: char) -> bool {
        let claude = self.claude_writing();
        let ch = &mut self.changes;
        if ch.confirm.is_some() {
            match c {
                'y' => {
                    let wish = ch.yes();
                    self.stage_wish(wish);
                }
                'n' => ch.confirm = None,
                _ => {}
            }
            return true;
        }
        if c == 'r' && ch.told.as_ref().is_some_and(|t| t.commit) {
            self.changes_run_again();
            return true;
        }
        let lines = ch.focus == Focus::Lines;
        let none = || Ok(None);
        let done = match c {
            'j' | 'k' if lines => {
                ch.move_cursor(if c == 'j' { 1 } else { -1 });
                none()
            }
            'j' => Ok(ch.move_by(1)),
            'k' => Ok(ch.move_by(-1)),
            ' ' if lines => ch.apply_lines(claude),
            ' ' => ch.toggle_file(claude),
            'a' if lines => {
                ch.pick_hunk();
                none()
            }
            'a' => ch.toggle_all(claude),
            'v' if lines => {
                ch.toggle_range();
                none()
            }
            'v' | 'l' if !lines => ch.enter_lines().map(|()| None),
            'h' if lines => {
                ch.leave_lines();
                none()
            }
            // `v` and `l` in the lines, `h` in the files: nothing to do.
            'v' | 'l' | 'h' => none(),
            'c' => Ok(ch.focus_message()),
            't' if !lines => {
                ch.toggle_tree();
                none()
            }
            's' => ch.split(),
            'n' | 'N' => {
                let dir = if c == 'n' { 1 } else { -1 };
                if lines {
                    ch.step_run(dir);
                } else {
                    ch.jump += dir;
                }
                none()
            }
            'w' => {
                self.toggle_ignore_space();
                return true;
            }
            'O' => {
                self.images.cycle();
                return true;
            }
            'B' | 'H' => {
                self.open_blame(if c == 'B' { crate::blame::Tab::Blame } else { crate::blame::Tab::History });
                return true;
            }
            'e' | 'x' => {
                self.changes_external(c == 'x');
                return true;
            }
            '[' | ']' => {
                self.changes_step(if c == ']' { 1 } else { -1 });
                return true;
            }
            _ => return false,
        };
        self.changes_did(done);
        true
    }

    /// The second key of `g` in the `2` view: the operations that can lose
    /// work.
    pub(crate) fn changes_go(&mut self, c: char) {
        let claude = self.claude_writing();
        let ch = &mut self.changes;
        let done = match c {
            'd' => ch.discard(claude),
            'i' => ch.ignore(claude),
            'a' => Ok(ch.toggle_amend()),
            'g' => Ok(ch.select(0)),
            _ => return,
        };
        self.changes_did(done);
    }

    /// The `2` view's keys with a name.
    fn changes_named(&mut self, name: &str) -> bool {
        let ch = &mut self.changes;
        if ch.confirm.is_some() {
            return true;
        }
        let lines = ch.focus == Focus::Lines;
        let wish = match name {
            "ArrowDown" | "ArrowUp" if lines => {
                ch.move_cursor(if name == "ArrowDown" { 1 } else { -1 });
                None
            }
            "Home" | "End" if lines => {
                ch.move_cursor(if name == "End" { i64::MAX / 2 } else { i64::MIN / 2 });
                None
            }
            "ArrowDown" => ch.move_by(1),
            "ArrowUp" => ch.move_by(-1),
            "Shift+ArrowDown" => ch.extend(1),
            "Shift+ArrowUp" => ch.extend(-1),
            "Home" => ch.select(0),
            "End" => ch.select(usize::MAX),
            "ArrowRight" | "Alt+ArrowRight" => return self.changes_key('l'),
            "ArrowLeft" | "Alt+ArrowLeft" => return self.changes_key('h'),
            "PageDown" => {
                ch.pages += 1;
                None
            }
            "PageUp" => {
                ch.pages -= 1;
                None
            }
            "Enter" => {
                ch.wide = !ch.wide;
                None
            }
            _ => return false,
        };
        self.stage_wish(wish);
        true
    }

    /// `d`: the selected worktree's branch as a whole, from where it left
    /// the trunk.
    pub fn open_review(&mut self) {
        let Some(t) = self.wt_selected() else { return };
        let (path, branch) = (t.wt.path.clone(), t.wt.branch.clone());
        let word = match branch {
            _ if self.trees.trunk_head.is_none() => "review.no_trunk",
            None => "review.detached",
            Some(branch) => {
                let notes = self.notes_back(&path);
                let mut r = Review::new(path, branch);
                r.notes = notes;
                self.review = Some(r);
                self.review_sync();
                return;
            }
        };
        let text = self.tr(word);
        self.say(text);
    }

    /// `D` in the graph: the commit marked by Ctrl+click and the selected
    /// one compared, in the review's form (from where they parted).
    pub fn open_compare(&mut self) {
        let Some((a, b)) = self.graph.pair() else {
            let text = self.tr("compare.pick");
            return self.say(text);
        };
        let Some(path) = self.repo_top() else { return };
        self.close_review();
        let title = self.tr("compare.title").to_owned();
        self.review = Some(Review::compare(path, title, a, b));
        self.view = View::Worktree;
        self.review_sync();
    }

    /// Closes the branch review.
    pub fn close_review(&mut self) {
        if let Some(r) = self.review.take() {
            self.notes_away(r.path, r.notes);
        }
        if let Some(r) = &self.reading {
            r.review_closed();
        }
    }

    /// What the branch review reads to: the trunk, its worktree's commit and
    /// uncommitted change as they are now; `None` while its worktree is not
    /// on its branch.
    pub fn review_ends(&self) -> Option<Ends> {
        let r = self.review.as_ref()?;
        let t = self.trees.trees.iter().find(|t| t.wt.path == r.path)?;
        if t.wt.branch.as_deref() != Some(r.branch.as_str()) {
            return None;
        }
        let trunk = self.trees.trunk_head?;
        let head = Oid::parse(&t.wt.head)?;
        let wip = self.wip_in(&r.path).filter(|w| !w.files.is_empty() && w.head == head).map(|w| w.commit);
        Some(Ends { trunk, head, wip })
    }

    /// Asks for the branch review's list again when what it reads to has
    /// moved, while it is in sight.
    pub fn review_sync(&mut self) {
        if self.view != View::Worktree {
            return;
        }
        if self.review.as_ref().is_some_and(|r| r.cmp.is_some()) {
            let wish = self.review.as_mut().and_then(|r| r.ask_compare());
            return self.wish(wish);
        }
        let Some(ends) = self.review_ends() else { return };
        let wish = self.review.as_mut().and_then(|r| r.ask(ends));
        self.wish(wish);
    }

    /// Sends what the branch review wants to the reading thread.
    pub(crate) fn wish(&self, wishes: impl IntoIterator<Item = Wish>) {
        if let Some(r) = &self.reading {
            for w in wishes {
                r.review(w);
            }
        }
    }

    /// The branch review's own keys.
    fn review_key(&mut self, c: char) -> bool {
        if matches!(c, 'y' | 'Y') {
            self.turn_step(c == 'y');
            return true;
        }
        // A comparison has no worktree: no uncommitted changes, last look,
        // checks, notes or merge.
        if matches!(c, 'm' | 'n' | 'g' | ' ' | 'u' | 's') && self.review.as_ref().is_some_and(|r| r.cmp.is_some()) {
            return true;
        }
        if c == 'm' && self.review.is_some() {
            self.ask_merge();
            return true;
        }
        match c {
            'n' if self.review.is_some() => return self.review_note_key(),
            'g' if self.review.is_some() => return self.review_hand_key(),
            _ => {}
        }
        let Some(r) = self.review.as_mut() else { return false };
        let mut said = None;
        let wishes = match c {
            'J' => {
                r.step(1);
                Vec::new()
            }
            'K' => {
                r.step(-1);
                Vec::new()
            }
            'j' => r.move_by(1).into_iter().collect(),
            'k' => r.move_by(-1).into_iter().collect(),
            ' ' => r.check(),
            'u' => {
                r.wip = !r.wip;
                Vec::new()
            }
            's' => {
                said = r.toggle_since().err();
                Vec::new()
            }
            _ => return false,
        };
        self.wish(wishes);
        match said {
            Some(word) => {
                let text = self.tr(word);
                self.say(text);
            }
            None => self.review_sync(),
        }
        true
    }

    fn review_note_key(&mut self) -> bool {
        self.review_note();
        true
    }

    fn review_hand_key(&mut self) -> bool {
        self.review_hand();
        true
    }

    /// The branch review's keys with a name.
    fn review_named(&mut self, name: &str) -> bool {
        let Some(r) = self.review.as_mut() else { return false };
        let wish = match name {
            "ArrowDown" => r.move_by(1),
            "ArrowUp" => r.move_by(-1),
            "Home" => r.select(0),
            "End" => r.select(usize::MAX),
            "PageDown" => {
                r.pages += 1;
                None
            }
            "PageUp" => {
                r.pages -= 1;
                None
            }
            "Enter" => {
                r.wide = !r.wide;
                None
            }
            _ => return false,
        };
        self.wish(wish);
        true
    }

    /// The keys that work now: the view's, or the branch review's over the
    /// worktree table.
    pub fn keys(&self) -> &'static [(&'static str, &'static str)] {
        self.keys_of().0
    }

    /// The keys in front now and the word-table key of what they belong to
    /// (the view, or the screen over it). The order is the one `feed` takes
    /// the screens in, so the keybar and `?` name the keys that really work.
    pub fn keys_of(&self) -> (&'static [(&'static str, &'static str)], &'static str) {
        if self.band.is_some() {
            (&BAND_KEYS, "key.asking")
        } else if self.tidy.is_some() {
            (&TIDY_KEYS, "key.tidy")
        } else if self.ready.is_some() {
            (&READY_KEYS, "key.ready")
        } else if self.auth.is_some() {
            (&AUTH_KEYS, "key.auth")
        } else if self.reflog.is_some() {
            (&REFLOG_KEYS, "key.reflog")
        } else if self.stats.is_some() {
            (&STATS_KEYS, "key.stats_screen")
        } else if self.blame.is_some() {
            (&BLAME_KEYS, "key.blame_screen")
        } else if self.trial.is_some() {
            (&TRIAL_KEYS, "key.trial")
        } else if self.reorder.is_some() {
            (&REORDER_KEYS, "key.reorder")
        } else if self.view == View::Worktree && self.review.is_some() {
            (&REVIEW_KEYS, "key.branch_diff")
        } else {
            (self.view.keys(), self.view.label())
        }
    }

    /// Says so when a Claude handed a conflict has stopped writing: it was
    /// seen running and no agent in its worktree runs now. One that is not
    /// seen running for a while is dropped, as is one whose sessions are gone.
    fn handed_stopped(&mut self) {
        let mut said = Vec::new();
        let mut check = Vec::new();
        let mut watched = std::mem::take(&mut self.handed);
        watched.retain_mut(|h| {
            let agents: Vec<_> = self.claude_in(&h.wt).iter().filter(|s| s.is_agent()).collect();
            if agents.is_empty() {
                return h.polls < 3 && h.ran;
            }
            if agents.iter().any(|s| s.state == State::Running) {
                h.ran = true;
                return true;
            }
            h.polls += 1;
            if h.ran || h.polls >= 3 {
                if h.conflict {
                    check.push(h.wt.clone());
                }
                said.push((self.tr("claude.handed_done").replace("{wt}", &crate::act::name(&h.wt)), crate::act::name(&h.wt)));
                return false;
            }
            true
        });
        self.handed = watched;
        if let Some((t, wt)) = said.last().cloned() {
            self.say_for(t, 10);
            let (title, body) = (self.tr("notify.title_done").to_owned(), self.tr("claude.handed_done").replace("{wt}", &wt));
            self.tell("done", &title, &body);
        }
        for wt in check {
            self.act(crate::act::Task::Markers { wt });
        }
    }

    /// Tell of `event` outside the window, when the window is behind another.
    pub fn tell(&self, event: &str, title: &str, body: &str) {
        if !self.front {
            crate::notify::tell(&self.prefs.notify, event, title, body);
        }
    }

    /// Watches the Claude in the worktree at `wt`, which was handed a conflict.
    pub fn watch_handed(&mut self, wt: PathBuf) {
        self.handed.retain(|h| h.wt != wt);
        self.handed.push(Handed { wt, ran: false, polls: 0, conflict: false });
    }

    /// The one next step the empty states point at (issue #81), if any.
    pub fn next_step(&self) -> Option<michi_core::nextstep::Step> {
        let g = &self.graph;
        let has_history = if g.error().is_some() || !g.done() { None } else { Some(!g.is_empty()) };
        michi_core::nextstep::next_step(&michi_core::nextstep::Facts {
            has_history,
            worktrees: self.trees.trees.len(),
            tsumugi: self.claude.is_some(),
            tsumugi_asked: self.claude_asked,
        })
    }

    /// The Claude sessions in the worktree at `path`, the one to show first
    /// first.
    pub fn claude_in(&self, path: &Path) -> &[Session] {
        self.claude.iter().flatten().find(|(p, _)| p == path).map_or(&[], |(_, s)| s.as_slice())
    }

    /// The Claude sessions that may have made the commit `id`: the agents in
    /// the worktrees whose own work it is ([`michi_core::claude::made_in`]),
    /// the one to show first first in each. Tsumugi keeps no record of which
    /// session wrote which commit, so this is the worktree's Claude now.
    pub fn claude_of(&self, id: &Oid) -> Vec<&Session> {
        if self.claude.iter().flatten().all(|(_, s)| !s.iter().any(Session::is_agent)) {
            return Vec::new();
        }
        let tips: Vec<michi_core::claude::Tip> = self
            .trees
            .trees
            .iter()
            .filter_map(|t| Some(michi_core::claude::Tip { wt: t.wt.path.clone(), head: Oid::parse(&t.wt.head)? }))
            .collect();
        michi_core::claude::made_in(id, &tips, |o| self.graph.node(o))
            .iter()
            .flat_map(|wt| self.claude_in(wt).iter().filter(|s| s.is_agent()))
            .collect()
    }

    /// The uncommitted change of the worktree at `path`, if it has one.
    pub fn wip_in(&self, path: &Path) -> Option<&Wip> {
        self.wips.iter().find(|w| w.path == path)
    }

    /// Whether a merge is under way: the predictions are then replaced by
    /// the real conflicts (`4`) and are not shown.
    pub fn merging(&self) -> bool {
        self.trees.trees.iter().any(|t| t.doing == Some(michi_core::branch::Doing::Merge))
    }

    /// The worktree at `path`'s place in the suggested order to land the
    /// worktrees: `(number, of how many, places to resolve)`. `None` for one
    /// not in it, or while a merge is under way (the predictions are not
    /// there then).
    pub fn land_order_of(&self, path: &Path) -> Option<(usize, usize, u32)> {
        if self.merging() {
            return None;
        }
        let steps = michi_core::order::suggest(&self.clashes, &self.trees);
        let n = steps.iter().position(|s| s.path == path)?;
        Some((n + 1, steps.len(), steps[n].resolve))
    }

    /// `trees` in the table's order: as listed, or by the suggested landing
    /// order (the unranked, the trunk's among them, first).
    fn sorted(&self, mut trees: Trees) -> Trees {
        if self.wt_by_land && !self.merging() {
            let order = michi_core::order::suggest(&self.clashes, &trees);
            trees.trees.sort_by_key(|t| order.iter().position(|s| s.path == t.wt.path).map_or(0, |n| n + 1));
        }
        trees
    }

    /// Sorts the table again (the order or the predictions changed), the
    /// selection staying on its worktree.
    fn resort(&mut self) {
        let path = self.trees.trees.get(self.wt_sel).map(|t| t.wt.path.clone());
        self.trees = self.sorted(self.trees_raw.clone());
        self.wt_sel = path.and_then(|p| self.trees.trees.iter().position(|t| t.wt.path == p)).unwrap_or(0);
    }

    /// `s`: the table by the suggested landing order, or as listed.
    pub(crate) fn toggle_land_sort(&mut self) {
        self.wt_by_land = !self.wt_by_land;
        self.resort();
        let key = if self.wt_by_land { "wt.sort_land" } else { "wt.sort_listed" };
        let text = self.tr(key).to_owned();
        self.say(text);
    }

    /// `Enter` and a double click on the table's row: the branch's diff when
    /// it is not approved yet, else the graph on it.
    pub fn wt_enter(&mut self) {
        let unapproved = self.wt_selected().is_some_and(|t| {
            t.wt.branch.is_some()
                && t.wt.branch != self.trees.trunk
                && self.reviews.of(&t.wt.path).is_some_and(|r| r.stage != michi_core::queue::Stage::Approved)
        });
        if unapproved {
            self.open_review();
        } else {
            self.wt_in_graph();
        }
    }

    /// The sides the worktree at `path` would conflict with, and how.
    pub fn clashes_in(&self, path: &Path) -> Vec<(&Side, &Outcome)> {
        let Some(i) = self.clashes.side_of(path) else { return Vec::new() };
        self.clashes.of(i).into_iter().map(|(j, o)| (&self.clashes.sides[j], o)).collect()
    }

    /// The worktree table's selected row.
    pub fn wt_selected(&self) -> Option<&Tree> {
        self.trees.trees.get(self.wt_sel)
    }

    /// Moves the worktree table's selection by `by` rows, stopping at the
    /// ends.
    pub fn wt_move(&mut self, by: i32) {
        let last = self.trees.trees.len().saturating_sub(1) as i64;
        self.wt_sel = (self.wt_sel as i64 + i64::from(by)).clamp(0, last) as usize;
    }

    /// Opens the graph on the selected worktree: its uncommitted change, or
    /// its commit when it has none.
    pub fn wt_in_graph(&mut self) {
        let Some(t) = self.wt_selected() else { return };
        let path = t.wt.path.clone();
        let head = Oid::parse(&t.wt.head);
        self.view = View::Graph;
        self.detail.wide = false;
        if !self.graph.select_wip(&path) {
            if let Some(id) = head {
                self.go_to(id);
            }
        }
    }

    /// The table's button: read the worktrees again now.
    pub fn reread(&mut self) {
        if let Some(r) = &self.reading {
            r.reread();
        }
        let text = self.tr("wt.rereading");
        self.say(text);
    }

    /// The graph's rows in sight reach row `last`: read on before they run
    /// out.
    pub fn reach(&mut self, last: u32) {
        if self.graph.wants_more(last) {
            self.ask(Ask::More);
        }
    }

    /// The row at the graph's foot is clicked: read the next batch.
    pub fn more(&mut self) {
        if self.graph.ask_more() {
            self.ask(Ask::More);
        }
    }

    fn ask(&self, a: Ask) {
        if let Some(r) = &self.reading {
            let _ = r.asks.send(a);
        }
    }

    /// The git on the PATH when it is older than michi needs.
    pub fn old_git(&self) -> Option<Version> {
        self.git.filter(|v| !v.is_enough())
    }

    /// A bare character the keymap binds (`,` and `?` by default): it does
    /// what it is bound to. Whether it was one.
    fn bare_act(&mut self, c: char) -> bool {
        match self.prefs.keymap.act(ito_keys::Key::char(c)) {
            Some(a) => {
                self.run_key(a);
                true
            }
            None => false,
        }
    }

    /// A key with Ctrl or Alt, or a named one, from the keymap. Whether it
    /// was bound.
    pub fn key(&mut self, k: ito_keys::Key) -> bool {
        match self.prefs.keymap.act(k) {
            // The settings page steps with Ctrl+Tab and the like: not the tabs.
            Some(crate::keymap::Act::NextTab | crate::keymap::Act::PrevTab | crate::keymap::Act::CloseTab | crate::keymap::Act::Tab(_))
                if self.mode == Mode::Settings =>
            {
                false
            }
            Some(a) => {
                self.run_key(a);
                true
            }
            None => false,
        }
    }

    pub(crate) fn run_key(&mut self, a: crate::keymap::Act) {
        use crate::keymap::Act;
        match a {
            Act::Settings => self.mode = if self.mode == Mode::Settings { Mode::Main } else { Mode::Settings },
            Act::Help => {
                self.help_scroll = 0;
                self.mode = if self.mode == Mode::Help { Mode::Main } else { Mode::Help };
            }
            Act::Palette => self.toggle_palette(),
            Act::Cheat => self.toggle_cheat(),
            Act::Repos => self.toggle_repos(),
            Act::BugReport => self.bug_report(),
            Act::NextTab => self.tab_req = Some(michi_core::tabs::Op::Next),
            Act::PrevTab => self.tab_req = Some(michi_core::tabs::Op::Prev),
            Act::CloseTab => self.tab_req = Some(michi_core::tabs::Op::Close),
            Act::Tab(n) => self.tab_req = Some(michi_core::tabs::Op::Goto(n - 1)),
            Act::Quit => self.quit = true,
            Act::Undo => self.undo(false),
            Act::Redo => self.undo(true),
            Act::Scale(to) => self.scale(to),
            Act::Noop => {}
        }
    }

    /// F12: the bug report form in the browser, with the version, the OS and
    /// what michi knew filled in (by name, never by path). The browser is
    /// started off the UI thread.
    fn bug_report(&mut self) {
        let url = crate::bugreport::url(self.prefs.base.as_deref(), self.prefs.common.scale_or_one(), self.last_error.as_deref());
        std::thread::spawn(move || {
            let _ = michi_core::pr::open_url(&url);
        });
        let text = self.tr("bugreport.opened").to_owned();
        self.say_for(text, 6);
    }

    /// Zoom in, out or back to 100%, kept in common.toml.
    pub fn scale(&mut self, to: ito_common::ScaleTo) {
        let next = ito_common::scale_step(self.prefs.common.scale_or_one(), to);
        self.prefs.common.scale = Some(next);
        let no_folder = crate::i18n::tr(&self.lang, "settings.no_folder");
        self.prefs.common_change(vec![ito_common::scale_change(next)], no_folder);
        let text = ito_common::scale_text(next, &self.lang);
        self.say(text);
    }

    /// The key list: every key the keymap binds (as it is now, so a changed
    /// key shows), the fixed keys of every view, then those of the view in front.
    pub fn help_lines(&self) -> Vec<ito_help::Line> {
        use ito_help::Line;
        let mut lines = vec![Line::Head(self.tr("help.global").to_owned())];
        let bind = |k: &str, w: &str| Line::Bind { keys: ito_keys::human_text(k).into_owned(), desc: w.to_owned() };
        // The keys of the screen in front are tried first, and a screen over a
        // view takes every key: the list names only the global ones that work.
        let (keys, head) = self.keys_of();
        let over = self.band.is_some()
            || self.tidy.is_some()
            || self.reflog.is_some()
            || self.stats.is_some()
            || self.ready.is_some()
            || self.auth.is_some()
            || self.blame.is_some()
            || self.trial.is_some()
            || self.reorder.is_some();
        if !over {
            let global = michi_core::keyhelp::global_here(&GLOBAL_KEYS, keys);
            lines.extend(global.iter().filter(|(_, w)| self.prefs.level.lists(w)).map(|(k, w)| bind(k, self.tr(w))));
        }
        lines.extend(ito_keymap::help_lines(&self.prefs.keymap.main, "noop").into_iter().map(|l| {
            let desc = if l.desc.is_empty() { l.run.to_owned() } else { crate::i18n::tr_desc(&self.lang, l.desc) };
            Line::Bind { keys: l.keys, desc }
        }));
        lines.push(Line::Blank);
        lines.push(Line::Head(self.tr(head).to_owned()));
        lines.extend(self.listed(keys).iter().map(|(k, w)| bind(k, self.tr(w))));
        let hidden = self.prefs.level.hidden(keys);
        if hidden > 0 {
            lines.push(Line::Blank);
            lines.push(Line::Head(self.tr("level.more").replace("{n}", &hidden.to_string())));
        }
        lines
    }

    /// The key list as titled groups, to be dealt into columns.
    pub fn help_groups(&self) -> Vec<ito_help::Group> {
        use ito_help::Line;
        let mut out: Vec<ito_help::Group> = Vec::new();
        for l in self.help_lines() {
            match l {
                Line::Head(title) => out.push(ito_help::Group { title, rows: Vec::new() }),
                Line::Bind { keys, desc } => {
                    if let Some(g) = out.last_mut() {
                        g.rows.push((keys, desc));
                    }
                }
                _ => {}
            }
        }
        out
    }

    /// `keys` as the level lists them in the keybar and `?`. Every key still
    /// works, and the command palette has all of them (#115).
    pub fn listed(&self, keys: &'static [(&'static str, &'static str)]) -> Vec<(&'static str, &'static str)> {
        self.prefs.level.visible(keys)
    }

    /// The arrows, pages, Home and End scroll the key list.
    fn help_named(&mut self, name: &str) -> bool {
        let Some(d) = self.help_drawn.get() else { return false };
        let page = d.visible.saturating_sub(1).max(1) as i64;
        self.help_scroll = match name {
            "ArrowDown" => ito_help::scrolled(d.scroll, 1, d.max),
            "ArrowUp" => ito_help::scrolled(d.scroll, -1, d.max),
            "PageDown" => ito_help::scrolled(d.scroll, page, d.max),
            "PageUp" => ito_help::scrolled(d.scroll, -page, d.max),
            "Home" => 0,
            "End" => d.max,
            _ => return false,
        };
        true
    }

    /// A character typed (not a key with Ctrl or Alt: those are the
    /// shortcuts). Returns whether it was used.
    pub fn feed(&mut self, c: char) -> bool {
        if let Some(p) = self.prefix.take() {
            if self.mode == Mode::Main {
                return self.prefix_key(p, c);
            }
        }
        match (self.mode, c) {
            (Mode::Help, c) if self.bare_act(c) => {}
            (Mode::Settings, _) | (Mode::Help, _) => return false,
            (Mode::Main, _) if self.picker.is_some() || self.new_work.is_some() || self.pr_form.is_some() || self.reply.is_some() => {
                return false
            }
            (Mode::Main, c) if self.cheat.is_some() => return self.cheat_key(c),
            (Mode::Main, _) if self.legend => {
                self.legend = false;
                return true;
            }
            (Mode::Main, c) if self.band.is_some() => return self.band_key(c),
            (Mode::Main, c) if self.tidy.is_some() => return self.tidy_key(c),
            (Mode::Main, c) if self.ready.is_some() => return self.ready_key(c),
            (Mode::Main, c) if self.auth.is_some() => return self.auth_key(c),
            (Mode::Main, c) if self.reflog.is_some() => return self.reflog_key(c),
            (Mode::Main, c) if self.stats.is_some() => return self.stats_key(c),
            (Mode::Main, c) if self.blame.is_some() => return self.blame_key(c),
            (Mode::Main, c) if self.trial.is_some() => return self.trial_key(c),
            (Mode::Main, c) if self.reorder.is_some() => return self.reorder_key(c),
            (Mode::Main, c) if self.bare_act(c) => {}
            (Mode::Main, 'g') if self.g_prefix_view() => self.prefix = Some(crate::prefix::Prefix::Go),
            (Mode::Main, 'y') if self.y_prefix_view() => self.prefix = Some(crate::prefix::Prefix::Copy),
            (Mode::Main, c) => match View::from_digit(c) {
                Some(View::Conflict) => self.open_conflicts(),
                Some(v) => {
                    self.view = v;
                    self.review_sync();
                }
                None if self.view == View::Graph && self.pane_key(c) => {}
                None if self.view == View::Graph && self.graph_key(c) => {}
                None if self.body_scroll(&c.to_string()) => {}
                None if self.view == View::Worktree && self.review.is_some() && self.review_key(c) => {}
                None if self.view == View::Worktree && self.review.is_none() && self.wt_key(c) => {}
                None if self.view == View::Changes && self.changes_key(c) => {}
                None if self.view == View::Conflict && self.conflict_key(c) => {}
                None if c == 'W' => self.jump_waiting(),
                None if c == 'K' => self.open_auth(),
                None if self.net_key(c) => {}
                None if self.strip_key(c) => {}
                None => return self.later(|token| token.chars().eq([c])),
            },
        }
        true
    }

    /// `[` / `]`: selects the `n`th parent of the selected commit (`]`: the
    /// second one, where a merge came from).
    fn go_parent(&mut self, n: usize) {
        let Some(id) = self.graph.selected() else { return self.say(self.tr("act.pick_commit")) };
        let parent = self.graph.find_commit(&id).and_then(|c| c.parents.get(n).copied());
        match parent {
            Some(p) => self.go_to(p),
            None => self.say(self.tr(if n == 0 { "act.no_parent" } else { "act.no_merge_parent" })),
        }
    }

    /// `g g`: the first row of the graph or the table.
    pub(crate) fn go_first(&mut self) {
        match self.view {
            View::Graph => self.graph.home(),
            View::Worktree => self.wt_sel = 0,
            View::Conflict => self.conflicts.move_to(false),
            _ => {}
        }
    }

    /// The graph's own keys that work already.
    fn graph_key(&mut self, c: char) -> bool {
        let g = &mut self.graph;
        match c {
            'b' => self.open_picker(PickKind::Branch),
            'T' => self.open_picker(PickKind::Tag),
            's' => self.open_picker(PickKind::Stash),
            'M' => self.open_remotes(),
            'L' => self.open_reflog(),
            'Z' => self.open_stats(),
            'B' => self.open_blame(crate::blame::Tab::Blame),
            'H' => self.open_blame(crate::blame::Tab::History),
            'Y' => self.copy_patch(),
            '#' => _ = self.copy_hash(),
            '[' => self.go_parent(0),
            ']' => self.go_parent(1),
            'w' => self.toggle_ignore_space(),
            'x' | 'y' | 'u' | 'X' | 'r' => return self.bisect_key(c),
            'U' => self.open_picker(PickKind::Flow),
            'G' => g.end(),
            'I' => self.open_issue(),
            'O' => self.images.cycle(),
            't' => self.ask_revert(),
            'R' => self.ask_reset(),
            'C' => self.ask_cherry(),
            'D' => self.open_compare(),
            'm' => self.ask_merge_commit(),
            'j' => g.move_by(1),
            'k' => g.move_by(-1),
            'l' => _ = g.open_fold(),
            'h' => _ = g.close_fold(),
            '/' => g.focus_filter = true,
            '?' => g.start_deep(),
            ' ' => g.toggle_mark(),
            'z' => self.fold(),
            'o' => self.focus_claude(false),
            'N' => self.open_new_work(),
            _ => return false,
        }
        true
    }

    /// The worktree table's own keys that work already.
    fn wt_key(&mut self, c: char) -> bool {
        match c {
            'j' => self.wt_move(1),
            'k' => self.wt_move(-1),
            'd' => self.open_review(),
            'o' => self.focus_claude(true),
            'c' => self.open_tidy(),
            'T' => self.open_trial(),
            'L' => self.open_reflog(),
            'Z' => self.open_stats(),
            'Y' => self.open_turns(),
            'u' => self.open_submodules(),
            'v' => self.open_move(),
            'U' => self.open_flow(),
            'R' => self.open_ready(),
            'G' => self.wt_move(i32::MAX / 2),
            'm' => self.ask_merge(),
            'l' => self.ask_land(),
            's' => self.toggle_land_sort(),
            'x' => self.ask_remove(),
            'O' => self.open_pr(),
            'I' => self.open_issue(),
            'Q' => self.open_pr_form(),
            't' => self.open_in(Tool::Terminal, true),
            'e' => self.open_in(Tool::Editor, true),
            'E' => self.open_in(Tool::Explorer, true),
            'r' => self.open_reorder(),
            'M' => self.ask_rebase_trunk(),
            'N' => self.open_new_work(),
            _ => return false,
        }
        true
    }

    /// `o`: goes to the tab of the Claude in the window's worktree (`table`:
    /// in the selected row's instead), the one waiting or done first.
    pub(crate) fn focus_claude(&mut self, table: bool) {
        let wt = if table { self.wt_selected().map(|t| t.wt.path.clone()) } else { self.repo_top() };
        let Some(wt) = wt else { return self.say(self.tr("act.no_tree")) };
        let key = paths::key(&wt);
        let sessions = self.claude.iter().flatten().find(|(p, _)| paths::key(p) == key).map(|(_, s)| s);
        let agents = || sessions.into_iter().flatten().filter(|s| s.is_agent());
        let session = agents().find(|s| s.state != State::Running).or_else(|| agents().next()).map(|s| s.id);
        match session {
            Some(session) => self.act(crate::act::Task::Focus { session }),
            None => {
                let text = self.tr("act.no_claude").replace("{wt}", &crate::act::name(&wt));
                self.say_for(text, 5);
            }
        }
    }

    /// `p`: the pull request page of the selected worktree's branch, in the
    /// browser (michi does not write the pull request itself).
    pub(crate) fn open_pr(&mut self) {
        let Some(t) = self.wt_selected() else { return self.say(self.tr("act.no_tree")) };
        let (wt, branch) = (t.wt.path.clone(), t.wt.branch.clone());
        match branch {
            Some(branch) => self.act(crate::act::Task::OpenPr { wt, branch }),
            None => self.say(self.tr("act.no_branch")),
        }
    }

    /// Opens the folder of the selected worktree (`table`; else the window's
    /// own) in the terminal, the editor or the file manager `[open]` names.
    pub(crate) fn open_in(&mut self, tool: Tool, table: bool) {
        let wt = if table { self.wt_selected().map(|t| t.wt.path.clone()) } else { self.repo_top() };
        let Some(wt) = wt else { return self.say(self.tr("act.no_tree")) };
        let command = self.prefs.open.command(tool, &self.prefs.found);
        self.act(crate::act::Task::OpenIn { wt, tool, command });
    }

    /// Opens the file `path` of the worktree `wt` in the editor `[open]`
    /// names, at `line` when it takes one (#43). Off the UI thread.
    pub(crate) fn open_file_at(&mut self, wt: PathBuf, path: String, line: Option<usize>) {
        let command = self.prefs.open.command(ito_openin::Tool::Editor, &self.prefs.found);
        let line_args = self.prefs.open.line_args.clone();
        self.act(crate::act::Task::OpenFile { wt, path, line, command, line_args });
    }

    /// Opens the diff of the file `path` of `wt` in the external diff tool (#43).
    pub(crate) fn open_diff_tool(&mut self, wt: PathBuf, path: String, staged: bool) {
        let command = self.prefs.open.command(ito_openin::Tool::Diff, &self.prefs.found);
        self.act(crate::act::Task::OpenDiff { wt, path, staged, command });
    }

    /// `e` (the editor at the cursor's line) or `x` (the external diff tool)
    /// on the `2` view's selected file.
    fn changes_external(&mut self, diff: bool) {
        let wt = self.changes.wt.clone().or_else(|| self.repo_top());
        let sel = self.changes.selected().map(|(side, f)| (side, f.path.clone(), f.status == michi_core::show::Status::Deleted));
        let (Some(wt), Some((side, path, deleted))) = (wt, sel) else { return self.say(self.tr("blame.no_file")) };
        if diff {
            return self.open_diff_tool(wt, path, side == michi_core::stage::Side::Staged);
        }
        if deleted {
            let text = self.tr("blame.deleted").replace("{path}", &path);
            return self.say(text);
        }
        let line = self.changes.editor_line();
        self.open_file_at(wt, path, line);
    }

    /// The window's own worktree, once the repository is read.
    pub(crate) fn repo_top(&self) -> Option<PathBuf> {
        match &self.repo {
            Repo::Ready(s) => Some(s.top.clone()),
            _ => None,
        }
    }

    /// `z`, or the header's toggle: fold the trunk's merges or show them all.
    pub fn fold(&mut self) {
        let text = match self.graph.toggle_fold() {
            Folded::On { merges, commits } => self
                .tr("fold.on")
                .replace("{trunk}", self.graph.trunk_name().unwrap_or("main"))
                .replace("{n}", &merges.to_string())
                .replace("{m}", &commits.to_string()),
            Folded::Nothing => self.tr("fold.nothing").to_owned(),
            Folded::Off => self.tr("fold.off").to_owned(),
        };
        self.say(text);
    }

    /// A key with a name (`Enter`, `ArrowDown`), outside the help and the
    /// settings.
    pub fn named(&mut self, name: &str) -> bool {
        self.prefix = None;
        if self.mode == Mode::Help {
            return self.help_named(name);
        }
        if self.mode != Mode::Main {
            return false;
        }
        if self.band_named(name) {
            return true;
        }
        if self.tidy.is_some() {
            return self.tidy_named(name);
        }
        if self.trial.is_some() {
            return self.trial_named(name);
        }
        if self.ready.is_some() || self.auth.is_some() {
            return true;
        }
        if self.reflog.is_some() {
            return self.reflog_named(name);
        }
        if self.cheat.is_some() {
            return self.cheat_named(name);
        }
        if self.legend {
            self.legend = false;
            return true;
        }
        if self.stats.is_some() {
            return self.stats_named(name);
        }
        if self.blame.is_some() {
            return self.blame_named(name);
        }
        if self.reorder.is_some() {
            return self.reorder_named(name);
        }
        if self.pane_named(name) || self.body_scroll(name) {
            return true;
        }
        if self.view == View::Graph {
            let g = &mut self.graph;
            if self.detail.wide || self.pane == crate::panes::GraphPane::Detail {
                // The wide detail has the keys: they scroll it, not the graph.
                let d = &mut self.detail;
                match name {
                    "ArrowDown" => d.lines += 1,
                    "ArrowUp" => d.lines -= 1,
                    "PageDown" => d.pages += 1,
                    "PageUp" => d.pages -= 1,
                    "Home" => d.to_top = true,
                    "End" => d.to_end = true,
                    "Enter" => d.wide = !d.wide,
                    _ => return self.later(|token| token == name),
                }
                return true;
            }
            match name {
                "ArrowDown" => g.move_by(1),
                "ArrowUp" => g.move_by(-1),
                "PageDown" => g.page_by(false),
                "PageUp" => g.page_by(true),
                "Home" => g.home(),
                "End" => g.end(),
                "ArrowRight" => _ = g.open_fold(),
                "ArrowLeft" => _ = g.close_fold(),
                "Enter" => self.detail.wide = !self.detail.wide,
                _ => return self.later(|token| token == name),
            }
            return true;
        }
        if self.view == View::Worktree && self.review.is_some() {
            return self.review_named(name) || self.later(|token| token == name);
        }
        if self.view == View::Changes {
            return self.changes_named(name) || self.later(|token| token == name);
        }
        if self.view == View::Conflict {
            return self.conflict_named(name) || self.later(|token| token == name);
        }
        if self.view == View::Worktree {
            let n = self.trees.trees.len() as i32;
            match name {
                "ArrowDown" => self.wt_move(1),
                "ArrowUp" => self.wt_move(-1),
                "PageDown" => self.wt_move(10),
                "PageUp" => self.wt_move(-10),
                "Home" => self.wt_move(-n),
                "End" => self.wt_move(n),
                "Enter" => self.wt_enter(),
                _ => return self.later(|token| token == name),
            }
            return true;
        }
        self.later(|token| token == name)
    }

    /// A button whose work comes in a later stage: say so.
    pub fn later_word(&mut self, word: &'static str) {
        let text = self.tr("later").replace("{}", self.tr(word));
        self.say(text);
    }

    /// A key of the view or of every view whose work comes in a later stage:
    /// say so. `Space` is the space character.
    fn later(&mut self, is: impl Fn(&str) -> bool) -> bool {
        let found = self.keys().iter().chain(GLOBAL_KEYS.iter()).find(|(keys, _)| {
            keys.split(' ').any(|token| {
                let token = if token == "Space" { " " } else { token };
                is(token)
            })
        });
        let Some((_, word)) = found else { return false };
        let text = self.tr("later").replace("{}", self.tr(word));
        self.say(text);
        true
    }

    /// Escape: close the help or the settings, else the branch palette,
    /// else drop the job waiting for a yes, else close the clean-up list,
    /// else the stash the side column
    /// shows, else bring the graph back from beside a wide diff, else
    /// clear the graph's filter; over the worktree table, bring the review's
    /// diff back to one column, else close the review. With nothing else to
    /// close, stop the fetch, pull or push that runs.
    pub fn escape(&mut self) -> bool {
        if self.prefix.take().is_some() {
            return true;
        }
        if self.mode != Mode::Main {
            self.mode = Mode::Main;
            return true;
        }
        if std::mem::take(&mut self.guide_open) {
            return true;
        }
        if self.repos.take().is_some() {
            return true;
        }
        if self.picker.take().is_some() || self.new_work.take().is_some() || self.reply.take().is_some() {
            return true;
        }
        if self.pr_form.as_ref().is_some_and(|f| !f.busy) {
            self.pr_form = None;
            return true;
        }
        if self.cheat.take().is_some() {
            return true;
        }
        if std::mem::take(&mut self.legend) {
            return true;
        }
        if self.band.take().is_some() {
            return true;
        }
        if self.reorder.take().is_some() {
            return true;
        }
        if self.tidy.take().is_some() {
            return true;
        }
        if self.reflog.take().is_some()
            || self.stats.take().is_some()
            || self.blame.take().is_some()
            || self.ready.take().is_some()
            || self.auth.take().is_some()
        {
            return true;
        }
        if self.trial.take().is_some() {
            return true;
        }
        if self.view == View::Graph && self.picker.is_none() && self.stash_shown().is_some() {
            self.side_stash = None;
            return true;
        }
        if self.view == View::Graph && self.detail.wide {
            self.detail.wide = false;
            return true;
        }
        if self.view == View::Graph && self.pane != crate::panes::GraphPane::Tree {
            self.pane = crate::panes::GraphPane::Tree;
            return true;
        }
        if self.view == View::Graph && !self.graph.filter().is_empty() {
            self.graph.set_filter("");
            return true;
        }
        if self.view == View::Graph && self.graph.unmark() {
            return true;
        }
        if self.view == View::Changes {
            let ch = &mut self.changes;
            if ch.confirm.take().is_some() || ch.told.take().is_some() {
                return true;
            }
            if ch.unmark() {
                return true;
            }
            if ch.focus == Focus::Lines {
                ch.leave_lines();
                return true;
            }
            if ch.wide {
                ch.wide = false;
                return true;
            }
        }
        if self.view == View::Conflict && self.conflicts.abort {
            self.conflicts.abort = false;
            return true;
        }
        if self.view_pane_back() {
            return true;
        }
        if self.view == View::Worktree {
            if let Some(r) = self.review.as_mut().filter(|r| r.wide) {
                r.wide = false;
                return true;
            }
            if self.review.is_some() {
                self.close_review();
                return true;
            }
        }
        self.net_stop()
    }

    /// Whether the bar's tab for `v` is the one in force: the view's, under
    /// the key list too, which is drawn over the view.
    pub fn is_on(&self, v: View) -> bool {
        self.mode != Mode::Settings && self.view == v
    }
}

/// The word-table key saying why the repository could not be read.
pub fn failed_word(e: &git::Error) -> &'static str {
    match e {
        git::Error::NoGit => "repo.no_git",
        git::Error::NotRepo => "repo.not_repo",
        git::Error::Git(_) | git::Error::Io(_) | git::Error::Told { .. } => "repo.failed",
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::graph::fixture::{self, FEAT};
    use michi_core::claude::State;
    use michi_core::summary::Head;

    pub(crate) fn app() -> App {
        let mut a = App::new(Prefs::empty(), PathBuf::from("C:/dev/x"));
        a.lang = "en".into();
        a
    }

    pub(crate) fn toast(a: &App) -> &str {
        a.toast.as_ref().map_or("", |(t, _)| t)
    }

    /// The app with graph.rs's small history read in, as the reading
    /// thread sends it.
    pub(crate) fn read() -> App {
        let mut a = app();
        a.take(Answer::Setup { gen: 1, setup: Box::new(fixture::setup()) });
        a.take(Answer::Commits { gen: 1, batch: fixture::history(), done: true });
        a
    }

    fn selected(a: &App) -> &str {
        let g = &a.graph;
        g.selected_row().and_then(|k| g.commit(k)).map_or("", |c| c.subject.as_str())
    }

    #[test]
    fn brackets_go_to_the_parent_and_the_merged_in_commit() {
        let mut a = read();
        a.graph.select_id(&fixture::id(5));
        assert!(a.feed(']'));
        assert_eq!(selected(&a), "f2");
        a.graph.select_id(&fixture::id(5));
        assert!(a.feed('['));
        assert_eq!(selected(&a), "m1");
    }

    #[test]
    fn a_toast_is_gone_once_its_time_is_up() {
        let mut a = app();
        a.say_for("hello", 60);
        assert!(a.toast_left().is_some());
        a.toast = Some(("hello".into(), Instant::now()));
        assert!(a.toast_left().is_none(), "an expired toast is not drawn again");
    }

    #[test]
    fn digits_open_the_views() {
        let mut a = app();
        assert_eq!(a.view, View::Graph);
        for (c, v) in [('2', View::Changes), ('3', View::Worktree), ('1', View::Graph)] {
            assert!(a.feed(c));
            assert_eq!(a.view, v);
        }
        assert!(!a.feed('5'));
        assert!(a.toast.is_none());
    }

    #[test]
    fn comma_and_question_mark_open_and_escape_closes() {
        let mut a = app();
        assert!(!a.feed(','), "a bare comma is free");
        assert!(a.key(ito_keys::Key::parse("<C-,>").unwrap()));
        assert_eq!(a.mode, Mode::Settings);
        // In the settings the screen's own controls take the characters.
        assert!(!a.feed('3'));
        assert_eq!(a.view, View::Graph);
        assert!(a.escape());
        assert_eq!(a.mode, Mode::Main);
        assert!(!a.escape());
        a.run_key(crate::keymap::Act::Help);
        assert_eq!(a.mode, Mode::Help);
        assert!(!a.feed('3'), "the help swallows the views' digits");
        assert!(!a.named("Enter"));
        a.run_key(crate::keymap::Act::Help);
        assert_eq!(a.mode, Mode::Main);
    }

    #[test]
    fn the_key_list_shows_the_keymap_and_scrolls() {
        use ito_help::Line;
        let mut a = App::new(crate::prefs::Prefs::empty(), std::env::temp_dir());
        let lines = a.help_lines();
        let has =
            |keys: &str, desc: &str| lines.iter().any(|l| matches!(l, Line::Bind { keys: k, desc: d } if k.contains(keys) && d == desc));
        let t = |d: &str| crate::i18n::tr_desc(&a.lang, d);
        assert!(has("<C-q>", &t("Quit")) && has("<C-S-p>", &t("Search commands")), "{lines:?}");
        assert!(has("1 2 3 4", a.tr("key.views")));
        // Nothing has been drawn, so there is nothing to scroll.
        a.mode = Mode::Help;
        assert!(!a.named("ArrowDown"));
        a.help_drawn.set(Some(ito_help::Drawn { visible: 10, max: 30, scroll: 0, above: 0, below: 0 }));
        assert!(a.named("PageDown"));
        assert_eq!(a.help_scroll, 9);
        assert!(a.named("End") && a.help_scroll == 30);
        assert!(a.named("Home") && a.help_scroll == 0);
    }

    #[test]
    fn the_tab_in_force_is_the_view_but_not_in_the_settings() {
        let mut a = app();
        assert!(a.is_on(View::Graph));
        a.run_key(crate::keymap::Act::Help);
        assert!(a.is_on(View::Graph), "the key list is over the view");
        a.escape();
        a.key(ito_keys::Key::parse("<C-,>").unwrap());
        assert!(!a.is_on(View::Graph));
    }

    #[test]
    fn every_view_has_keys_and_words_for_them() {
        for v in View::ALL {
            assert!(!v.keys().is_empty());
            for (_, w) in v.keys() {
                assert_ne!(crate::i18n::tr("en", w), *w, "no words for {w}");
            }
            assert_ne!(crate::i18n::tr("en", v.label()), v.label());
            assert_ne!(crate::i18n::tr("en", v.empty()), v.empty());
        }
        for (_, w) in GLOBAL_KEYS.iter().chain(&REVIEW_KEYS) {
            assert_ne!(crate::i18n::tr("en", w), *w, "no words for {w}");
        }
    }

    /// Every screen that has keys of its own, by the word-table key of its name.
    fn screens() -> Vec<(&'static str, &'static michi_core::keyhelp::Keys)> {
        let mut v: Vec<(&'static str, &'static michi_core::keyhelp::Keys)> = View::ALL.iter().map(|v| (v.label(), v.keys())).collect();
        v.extend([
            ("key.branch_diff", &REVIEW_KEYS[..]),
            ("key.asking", &BAND_KEYS[..]),
            ("key.tidy", &TIDY_KEYS[..]),
            ("key.ready", &READY_KEYS[..]),
            ("key.auth", &AUTH_KEYS[..]),
            ("key.reflog", &REFLOG_KEYS[..]),
            ("key.stats_screen", &STATS_KEYS[..]),
            ("key.blame_screen", &BLAME_KEYS[..]),
            ("key.trial", &TRIAL_KEYS[..]),
            ("key.reorder", &REORDER_KEYS[..]),
        ]);
        v
    }

    #[test]
    fn every_screen_names_each_key_once_and_has_words_in_both_languages() {
        for (name, keys) in screens().into_iter().chain([("help.global", &GLOBAL_KEYS[..])]) {
            assert_eq!(michi_core::keyhelp::repeated(keys), Vec::<&str>::new(), "{name} lists a key twice");
            for lang in ["en", "ja"] {
                assert_ne!(crate::i18n::tr(lang, name), name, "no {lang} words for {name}");
                for (_, w) in keys {
                    assert_ne!(crate::i18n::tr(lang, w), *w, "no {lang} words for {w}");
                }
            }
        }
    }

    #[test]
    fn every_advanced_word_is_a_key_somewhere_and_basic_keeps_the_everyday_ones() {
        let all: Vec<&str> =
            screens().into_iter().flat_map(|(_, k)| k.iter().map(|(_, w)| *w)).chain(GLOBAL_KEYS.iter().map(|(_, w)| *w)).collect();
        for w in michi_core::level::ADVANCED {
            assert!(all.contains(w), "{w} is not a key word of any screen");
        }
        // No view is left with an empty list in basic.
        for (name, keys) in screens() {
            assert!(!michi_core::level::Level::Basic.visible(keys).is_empty(), "{name}");
        }
    }

    #[test]
    fn basic_shortens_the_key_list_and_says_where_the_rest_is() {
        let mut p = crate::prefs::Prefs::empty();
        p.level = michi_core::level::Level::Basic;
        let basic = App::new(p, std::env::temp_dir());
        let full = app();
        let text = |a: &App| a.help_lines().len();
        assert!(text(&basic) > 0 && text(&basic) != text(&full));
        assert!(basic.listed(basic.keys()).len() < full.listed(full.keys()).len());
        assert_eq!(basic.keys().len(), full.keys().len(), "every key still works");
    }

    #[test]
    fn the_keys_that_change_their_meaning_with_the_screen_are_the_ones_known() {
        use michi_core::keyhelp::{collisions, meanings};
        let screens = screens();
        // The ones the owner chose to keep: written down so a new clash is a decision.
        let t = meanings(&screens, "t");
        assert_eq!(t, [("view.graph", "key.revert"), ("view.worktree", "key.open_in"), ("view.conflict", "key.theirs")]);
        let m = meanings(&screens, "m");
        assert!(m.contains(&("view.graph", "key.merge_commit")) && m.contains(&("view.worktree", "key.take_main")));
        // A single-letter key on the views' own lists that is also global is
        // shadowed there (the screen's key is tried first).
        let global: Vec<&str> = GLOBAL_KEYS.iter().flat_map(|(k, _)| michi_core::keyhelp::tokens(k)).collect();
        let mut shadows: Vec<(&str, &str)> = Vec::new();
        for (name, keys) in &screens[..View::ALL.len()] {
            for (k, _) in keys.iter() {
                shadows.extend(michi_core::keyhelp::tokens(k).filter(|t| global.contains(t)).map(|t| (*name, t)));
            }
        }
        assert_eq!(shadows, [("view.changes", "N")], "a view shadows a global key");
        assert!(collisions(&screens).contains_key("t"));
    }

    #[test]
    fn the_key_list_names_only_the_global_keys_that_work_on_the_screen() {
        let text = |a: &App| {
            a.help_lines().iter().map(|l| format!("{l:?}")).collect::<Vec<_>>().join(
                "
",
            )
        };
        let mut a = app();
        a.view = View::Worktree;
        let t = text(&a);
        assert!(t.contains("Pull") && t.contains("Push"), "{t}");
        a.view = View::Graph;
        assert!(text(&a).contains("Pull"));
    }

    #[test]
    fn the_keybar_follows_the_screen_in_front() {
        use michi_core::reflog::{Entry, Source};
        let mut a = app();
        a.view = View::Worktree;
        assert_eq!(a.keys(), View::Worktree.keys());
        a.reflog_found(vec![Entry {
            id: Oid::parse(&"1".repeat(40)).unwrap(),
            source: Source::Head(1),
            what: "reset".into(),
            subject: "s".into(),
            when: 1,
        }]);
        assert_eq!(a.keys(), &REFLOG_KEYS[..]);
        assert_eq!(a.keys_of().1, "key.reflog");
        let lines = a.help_lines();
        assert!(!lines.iter().any(|l| format!("{l:?}").contains("Fetch")), "no global key works over the list");
        a.reflog = None;
        a.band = Some(Band::new("ask".into(), crate::work::Then::Act(crate::act::Task::Rerere { wt: PathBuf::from("C:/r"), on: false })));
        assert_eq!(a.keys(), &BAND_KEYS[..]);
        a.band = None;
        assert_eq!(a.keys(), View::Worktree.keys());
    }

    #[test]
    fn d_opens_the_branch_review_over_the_worktree_table() {
        use michi_core::review::List;
        use michi_core::show::Changes;
        let mut a = app();
        a.view = View::Worktree;
        let mut trees = fixture::trees();
        trees.trunk_head = None;
        a.take(Answer::Trees(trees));
        a.wt_sel = 1;
        assert!(a.feed('d'));
        assert_eq!(toast(&a), "No trunk to compare with");
        a.take(Answer::Trees(fixture::trees()));
        a.wt_sel = 2;
        assert!(a.feed('d'));
        assert_eq!(toast(&a), "This worktree is on no branch");
        assert!(a.review.is_none());

        a.take(Answer::Wips(vec![fixture::wip("C:/r/wt", Some("wip"), 7, 11, Some(1))]));
        a.wt_sel = 1;
        assert!(a.feed('d'));
        assert_eq!(a.keys(), &REVIEW_KEYS[..]);
        assert!(a.feed('m'), "m works in the review");
        assert!(a.band.is_some() || a.toast.is_some(), "m asks or says why not");
        a.band = None;
        let wip = Ends { trunk: fixture::id(6), head: fixture::id(7), wip: Some(fixture::id(11)) };
        assert_eq!(a.review_ends(), Some(wip), "to the uncommitted changes");
        let gen = |a: &App| a.review.as_ref().unwrap().gens().0;
        let first = gen(&a);
        assert_ne!(first, 0, "the list is asked");
        assert!(a.feed('s'));
        assert_eq!(toast(&a), "Reading the branch…");
        let list = List { base: Some(fixture::id(5)), last: None, changes: Changes::default() };
        a.take(Answer::ReviewList { gen: first, result: Ok(Box::new(list)), checks: Default::default() });
        assert!(a.review.as_ref().unwrap().list.as_ref().is_some_and(|l| l.is_ok()));
        assert!(a.feed('s'));
        assert_eq!(toast(&a), "Not looked over before: all of it is new");
        assert!(!a.review.as_ref().unwrap().since_last);

        // Without the uncommitted changes, and the branch moving on: asked again.
        assert!(a.feed('u'));
        let second = gen(&a);
        assert!(!a.review.as_ref().unwrap().wip && second != first);
        let mut trees = fixture::trees();
        trees.trees[1].wt.head = fixture::id(8).to_string();
        a.take(Answer::Trees(trees.clone()));
        assert_ne!(gen(&a), second);
        // Another branch there: nothing to read to until it comes back.
        let third = gen(&a);
        trees.trees[1].wt.branch = Some("other".into());
        a.take(Answer::Trees(trees));
        assert_eq!((a.review_ends(), gen(&a)), (None, third));

        // Over the worktree table only; the other views keep their keys.
        assert!(a.feed('1'));
        assert_eq!(a.keys(), View::Graph.keys());
        assert!(a.feed('3'));
        assert!(a.review.is_some());
        assert!(!a.feed('q'), "not a key of the review");
        assert!(a.named("Enter"));
        assert!(a.review.as_ref().unwrap().wide);
        assert!(a.escape());
        assert!(a.review.as_ref().is_some_and(|r| !r.wide), "one column first");
        assert!(a.escape());
        assert!(a.review.is_none());
        assert_eq!(a.keys(), View::Worktree.keys());
    }

    #[test]
    fn keys_of_later_stages_say_so() {
        let mut a = app();
        assert!(a.feed('N'));
        assert_eq!(toast(&a), "No worktree to work in", "N needs a repository read");
        a.toast = None;
        assert!(a.feed('f'), "fetch works now");
        assert!(a.toast.is_none() && a.net.is_none(), "no reading thread here: nothing runs");
        assert!(a.feed('s'), "the graph's own key");
        assert_eq!(toast(&a), "No worktree to work in", "the stashes, with nothing read");
        assert!(a.named("Enter"), "the diff side by side, no longer a later stage");
        assert!(a.detail.wide);
        a.feed('2');
        assert!(a.feed('c'));
        assert!(a.changes.focus_message, "c goes to the message");
        assert!(!a.feed('z'), "not a key of the changes");
        assert!(!a.feed('q'));
    }

    #[test]
    fn what_a_worktree_is_doing_sits_in_the_texts() {
        use michi_core::branch::Doing;
        let mut a = app();
        let text = |a: &App, d| a.tr("act.concluded").replace("{doing}", &a.doing_word(d)).replace("{wt}", "r").replace("{new}", "abc1234");
        assert_eq!(text(&a, Doing::Merge), "Committed the merge on r: abc1234");
        a.lang = "ja".into();
        assert_eq!(text(&a, Doing::Merge), "r のマージをコミットしました: abc1234");
        assert_eq!(text(&a, Doing::CherryPick), "r の cherry-pick をコミットしました: abc1234");
    }

    #[test]
    fn the_graph_s_keys_move_the_selection() {
        let mut a = read();
        assert_eq!(selected(&a), "m2");
        assert!(a.feed('j'));
        assert_eq!(selected(&a), "w1");
        assert!(a.named("ArrowDown"));
        assert_eq!(selected(&a), FEAT);
        assert!(a.feed('k'));
        assert_eq!(selected(&a), "w1");
        assert!(a.named("End"));
        assert_eq!(selected(&a), "first");
        assert!(a.named("PageUp"));
        assert_eq!(selected(&a), "m2");
        assert!(a.named("PageDown"));
        assert_eq!(selected(&a), "first");
        assert!(a.named("Home"));
        assert_eq!(selected(&a), "m2");
        assert!(a.toast.is_none(), "nothing to say");
    }

    #[test]
    fn l_h_and_the_arrows_open_and_fold_a_merge() {
        let mut a = read();
        a.feed('j');
        a.feed('j');
        assert_eq!(selected(&a), FEAT);
        assert!(a.named("ArrowRight"));
        assert_eq!(a.graph.len(), 7);
        a.feed('j');
        a.feed('j');
        assert_eq!(selected(&a), "f2");
        assert!(a.named("ArrowLeft"));
        assert_eq!((a.graph.len(), selected(&a)), (5, FEAT), "the merge takes the selection");
        assert!(a.feed('l'));
        assert_eq!(a.graph.len(), 7);
        assert!(a.feed('h'));
        assert_eq!(a.graph.len(), 5);
    }

    #[test]
    fn z_folds_and_says_so() {
        let mut a = read();
        assert!(a.feed('z'));
        assert_eq!(toast(&a), "Unfolded all the merges.");
        assert_eq!(a.graph.len(), 7);
        assert!(a.feed('z'));
        assert_eq!(toast(&a), "Folded the merges into main: 1 (2 commits).");
        assert_eq!(a.graph.len(), 5);
        a.lang = "ja".into();
        a.feed('z');
        a.feed('z');
        assert!(toast(&a).contains("main"), "{}", toast(&a));

        let mut b = app();
        b.feed('z');
        b.feed('z');
        assert_eq!(toast(&b), "No merges to fold yet.");
    }

    #[test]
    fn slash_asks_for_the_filter_and_escape_clears_it() {
        let mut a = read();
        assert!(a.feed('/'));
        assert!(a.graph.focus_filter);
        a.graph.set_filter("f2");
        assert_eq!(a.graph.hit_count(), Some(1));
        assert!(a.escape());
        assert!(a.graph.filter().is_empty());
        assert!(!a.escape(), "nothing left to clear");
        a.graph.set_filter("f2");
        a.feed('2');
        assert!(!a.escape(), "the filter is the graph's");
    }

    #[test]
    fn answers_of_an_older_reading_are_dropped() {
        let mut a = read();
        let rows = a.graph.len();
        a.take(Answer::Setup { gen: 2, setup: Box::new(fixture::setup()) });
        a.take(Answer::Commits { gen: 1, batch: fixture::history()[..1].to_vec(), done: true });
        assert_eq!(a.graph.len(), rows, "the first reading's commits come too late");
        a.take(Answer::Failed { gen: 2, error: git::Error::Git("bad".into()) });
        assert_eq!(toast(&a), "Could not read the history: bad");
    }

    #[test]
    fn an_old_git_is_said_and_kept() {
        let mut a = app();
        a.take(Answer::Version(Ok(Version(2, 30, 1))));
        assert_eq!(a.old_git(), Some(Version(2, 30, 1)));
        assert!(toast(&a).starts_with("git 2.30.1 is older than 2.38.0"), "{}", toast(&a));
        let mut b = app();
        b.take(Answer::Version(Ok(Version(2, 52, 0))));
        assert_eq!(b.old_git(), None);
        assert!(b.toast.is_none());
    }

    #[test]
    fn the_tab_keys_ask_the_window_for_a_tab_step() {
        use crate::keymap::Act;
        use michi_core::tabs::Op;
        let mut a = app();
        for (act, op) in [(Act::NextTab, Op::Next), (Act::PrevTab, Op::Prev), (Act::CloseTab, Op::Close), (Act::Tab(3), Op::Goto(2))] {
            a.run_key(act);
            assert_eq!(a.tab_req.take(), Some(op));
        }
    }

    #[test]
    fn the_settings_leave_the_tab_keys_to_their_pages() {
        let mut a = app();
        a.mode = Mode::Settings;
        assert!(!a.key(ito_keys::Key::parse("<C-Tab>").unwrap()));
        assert_eq!(a.tab_req, None);
        a.mode = Mode::Main;
        assert!(a.key(ito_keys::Key::parse("<C-Tab>").unwrap()));
        assert!(a.tab_req.is_some());
    }

    #[test]
    fn the_summary_names_the_window_or_says_why_not() {
        let mut a = app();
        let s = Summary { name: "michi".into(), head: Head::Branch("main".into()), worktrees: 2, top: PathBuf::from("C:/dev/x") };
        a.take(Answer::Summary(Ok(s.clone())));
        assert_eq!(a.new_title.as_deref(), Some("michi — michi"));
        assert_eq!(a.repo, Repo::Ready(s));

        let mut b = app();
        b.take(Answer::Summary(Err(git::Error::NotRepo)));
        assert_eq!(b.repo, Repo::Failed(git::Error::NotRepo));
        assert_eq!(toast(&b), "Not a Git repository — C:/dev/x");
        assert!(b.new_title.is_none());
        b.take(Answer::Summary(Err(git::Error::Git("bad object".into()))));
        assert_eq!(toast(&b), "git failed: bad object — C:/dev/x");
    }

    #[test]
    fn the_detail_follows_the_selection() {
        use crate::detail::fixture::detail;
        let mut a = read();
        assert!(a.sync_detail(), "the first selection is asked for");
        let m2 = a.graph.selected().unwrap();
        assert_eq!(a.detail.asked(), Some(m2));
        assert!(!a.sync_detail());
        assert!(a.detail.got().is_none());
        // An answer for a commit no longer selected is dropped.
        a.take(Answer::Detail { id: fixture::id(1), result: Ok(Box::new(detail(fixture::id(1), vec![], 1))) });
        assert!(a.detail.got().is_none());
        a.take(Answer::Detail { id: m2, result: Ok(Box::new(detail(m2, vec![fixture::id(5)], 2))) });
        assert!(a.detail.rows(1).is_some());
        a.named("ArrowDown");
        assert!(a.sync_detail());
        assert!(a.detail.got().is_none() && a.detail.to_top);
    }

    #[test]
    fn a_commit_picked_is_seen() {
        let mut a = read();
        a.take(Answer::Unseen(fixture::unseen()));
        a.sync_detail();
        assert_eq!(a.unseen.ids().len(), 1, "the first selection is no one's pick");
        assert!(a.graph.select_id(&fixture::id(4)));
        a.sync_detail();
        assert_eq!(a.unseen.ids().len(), 1, "another commit");
        a.go_to(fixture::id(7));
        a.sync_detail();
        assert!(a.unseen.ids().is_empty());
        assert_eq!(a.unseen.of(std::path::Path::new("C:/r/wt")), Some(&[][..]), "the worktree is still followed");
    }

    #[test]
    fn a_trunk_merge_counts_its_commits() {
        let mut a = read();
        assert!(a.graph.select_id(&fixture::id(5)));
        a.sync_detail();
        let read = a.graph.read();
        assert_eq!(a.detail.merged(fixture::id(5), read), Some(&[fixture::id(3), fixture::id(2)][..]));
        assert_eq!(selected(&a), FEAT);
    }

    #[test]
    fn enter_spreads_the_diff_and_escape_takes_it_back_first() {
        let mut a = read();
        a.graph.set_filter("f");
        assert!(a.named("Enter"));
        assert!(a.detail.wide);
        assert!(a.escape());
        assert!(!a.detail.wide);
        assert_eq!(a.graph.filter(), "f", "the filter goes on the next Escape");
        assert!(a.escape());
        assert_eq!(a.graph.filter(), "");
        assert!(!a.escape());
    }

    #[test]
    fn the_side_column_goes_to_a_commit_or_says_it_is_not_there() {
        let mut a = read();
        a.go_to(fixture::id(3));
        assert_eq!(selected(&a), "f2");
        assert!(a.seeking.is_none());
        a.go_to(fixture::id(9));
        assert!(a.seeking.is_none(), "all is read: nothing more to read on to");
        assert!(toast(&a).starts_with("Not in the history: "), "{}", toast(&a));
        assert_eq!(selected(&a), "f2");
    }

    #[test]
    fn a_wip_read_anew_keeps_the_detail_where_it_was() {
        let mut a = read();
        a.take(Answer::Wips(vec![fixture::wip("C:/r/repo", Some("main"), 6, 11, Some(1))]));
        a.graph.select(0);
        assert!(a.sync_detail());
        assert_eq!(a.detail.asked(), Some(fixture::id(11)));
        let shown = crate::detail::fixture::detail(fixture::id(11), vec![fixture::id(6)], 2);
        a.detail.take(fixture::id(11), Ok(Box::new(shown)));
        a.detail.to_top = false;
        a.detail.scroll_to = Some(1);
        // Written again: the new change is asked for, the old one stays shown.
        a.take(Answer::Wips(vec![fixture::wip("C:/r/repo", Some("main"), 6, 12, Some(2))]));
        assert!(a.sync_detail());
        assert_eq!(a.detail.asked(), Some(fixture::id(12)));
        assert!(a.detail.got().is_some());
        assert_eq!((a.detail.to_top, a.detail.scroll_to), (false, Some(1)));
        assert!(!a.sync_detail());
        // Another row: from the top.
        a.graph.select(1);
        assert!(a.sync_detail());
        assert!(a.detail.got().is_none() && a.detail.to_top);
    }

    #[test]
    fn a_handed_claude_is_announced_once_it_stops() {
        let mut a = app();
        let wt = PathBuf::from("C:/r/wt");
        let s = |state| Session {
            id: 3,
            state,
            command: "claude".into(),
            agent: "claude".into(),
            cwd: "C:/r/wt".into(),
            branch: "wip".into(),
            title: "fix".into(),
            note: String::new(),
            since_ms: 0,
            read_only: false,
        };
        let give = |a: &mut App, st| a.take(Answer::Claude(Some(vec![(PathBuf::from("C:/r/wt"), vec![s(st)])])));
        a.watch_handed(wt);
        give(&mut a, State::Running);
        assert!(a.toast.is_none());
        give(&mut a, State::Running);
        assert!(a.toast.is_none() && a.handed.len() == 1);
        give(&mut a, State::Waiting);
        assert!(toast(&a).contains("wt"), "{}", toast(&a));
        assert!(a.handed.is_empty());
    }

    #[test]
    fn claude_sessions_are_looked_up_by_worktree() {
        let mut a = app();
        let s = Session {
            id: 3,
            state: State::Waiting,
            command: "claude".into(),
            agent: "claude".into(),
            cwd: "C:/r/wt/src".into(),
            branch: "wip".into(),
            title: "fix".into(),
            note: "Allow?".into(),
            since_ms: 0,
            read_only: false,
        };
        a.take(Answer::Claude(Some(vec![(PathBuf::from("C:/r/wt"), vec![s.clone()])])));
        assert_eq!(a.claude_in(Path::new("C:/r/wt")), [s]);
        assert!(a.claude_in(Path::new("C:/r/repo")).is_empty());
        a.take(Answer::Claude(None));
        assert!(a.claude_in(Path::new("C:/r/wt")).is_empty());
    }

    #[test]
    fn o_without_a_claude_in_the_worktree_says_so() {
        let mut a = app();
        a.view = View::Worktree;
        a.take(Answer::Trees(fixture::trees()));
        a.wt_sel = 1;
        assert!(a.feed('o'));
        assert!(toast(&a).starts_with("No Claude is running in"), "{}", toast(&a));
    }

    fn wt_path(a: &App) -> &str {
        a.wt_selected().and_then(|t| t.wt.path.to_str()).unwrap_or("")
    }

    #[test]
    fn the_worktree_table_s_keys_move_the_selection() {
        let mut a = read();
        a.take(Answer::Trees(fixture::trees()));
        a.feed('3');
        assert_eq!(wt_path(&a), "C:/r/repo");
        assert!(a.feed('j'));
        assert_eq!(wt_path(&a), "C:/r/wt");
        assert!(a.named("ArrowDown"));
        assert!(a.named("ArrowDown"), "the last row stays");
        assert_eq!(wt_path(&a), "C:/r/old");
        assert!(a.feed('k'));
        assert!(a.named("ArrowUp"));
        assert_eq!(wt_path(&a), "C:/r/repo");
        assert!(a.named("End"));
        assert_eq!(wt_path(&a), "C:/r/old");
        assert!(a.named("PageUp"));
        assert_eq!(wt_path(&a), "C:/r/repo");
        assert!(a.named("PageDown"));
        assert!(a.named("Home"));
        assert_eq!(wt_path(&a), "C:/r/repo");
        assert!(a.toast.is_none(), "nothing to say");
        // m takes the trunk in, here with nothing to take; the keys of a
        // later stage say so.
        assert!(a.feed('m'));
        assert_eq!(toast(&a), "repo is on main itself: there is nothing to merge");
        a.later_word("wt.pr");
        assert!(toast(&a).starts_with("Open a PR"), "{}", toast(&a));
    }

    #[test]
    fn the_worktree_selection_stays_on_its_worktree() {
        let mut a = app();
        a.take(Answer::Trees(fixture::trees()));
        a.wt_move(1);
        assert_eq!(wt_path(&a), "C:/r/wt");
        // The first worktree is gone: the selection stays on C:/r/wt.
        let mut t = fixture::trees();
        t.trees.remove(0);
        a.take(Answer::Trees(t.clone()));
        assert_eq!((a.wt_sel, wt_path(&a)), (0, "C:/r/wt"));
        // Its worktree gone, the selection goes to the first.
        a.wt_move(1);
        t.trees.remove(1);
        t.trees.remove(0);
        a.take(Answer::Trees(t));
        assert_eq!(a.wt_sel, 0);
        a.take(Answer::Trees(Trees::default()));
        assert_eq!(a.wt_sel, 0);
        a.wt_move(3);
        a.wt_in_graph();
        assert_eq!(a.view, View::Graph, "nothing to open, nothing done");
    }

    #[test]
    fn a_wip_with_only_folders_left_out_stays_out_of_the_graph() {
        let mut a = read();
        let mut only = fixture::wip("C:/r/repo", Some("main"), 6, 6, None);
        only.files.clear();
        only.left_out = vec![michi_core::predict::LeftOut { path: "target-v/".into(), files: 35584, bytes: 20 << 30 }];
        a.take(Answer::Wips(vec![only, fixture::wip("C:/r/wt", Some("wip"), 7, 11, Some(1))]));
        assert_eq!(a.graph.len(), 6, "one WIP row");
        assert!(a.wip_in(Path::new("C:/r/repo")).is_some_and(|w| w.files.is_empty() && w.left_out.len() == 1));
        assert!(a.wip_in(Path::new("C:/r/wt")).is_some());
        assert!(a.wip_in(Path::new("C:/r/old")).is_none());
    }

    #[test]
    fn the_wide_detail_takes_the_scroll_keys_from_the_graph() {
        let mut a = read();
        a.view = View::Graph;
        a.detail.wide = true;
        let before = a.graph.selected();
        assert!(a.named("ArrowDown"));
        assert!(a.named("PageDown"));
        assert!(a.named("End"));
        assert_eq!((a.detail.lines, a.detail.pages, a.detail.to_end), (1, 1, true));
        assert_eq!(a.graph.selected(), before, "the graph does not move");
        assert!(a.named("Enter"));
        assert!(!a.detail.wide);
        a.named("ArrowDown");
        assert_eq!(a.detail.lines, 1, "narrow: the graph has the keys");
    }

    #[test]
    fn the_key_list_is_dealt_in_groups_each_with_a_title() {
        let a = read();
        let g = a.help_groups();
        assert!(g.len() >= 2 && g.iter().all(|g| !g.title.is_empty()));
        assert!(g.iter().map(|g| g.rows.len()).sum::<usize>() > 10);
    }

    #[test]
    fn the_hash_key_copies_the_selected_commit() {
        let mut a = read();
        a.view = View::Graph;
        a.feed('#');
        let id = a.graph.selected().map(|i| i.to_string());
        assert_eq!(a.clipboard, id);
    }

    #[test]
    fn enter_opens_the_graph_on_the_worktree() {
        let mut a = read();
        a.take(Answer::Wips(vec![fixture::wip("C:/r/wt", Some("wip"), 7, 11, Some(1))]));
        a.take(Answer::Trees(fixture::trees()));
        a.feed('3');
        a.feed('j');
        a.detail.wide = true;
        assert!(a.named("Enter"));
        assert_eq!(a.view, View::Graph);
        assert!(!a.detail.wide);
        assert_eq!(a.graph.selected(), Some(fixture::id(11)), "its uncommitted change");
        // No change: its commit.
        a.feed('3');
        a.named("Home");
        a.named("Enter");
        assert_eq!(selected(&a), "m2");
        a.feed('3');
        a.named("End");
        a.named("Enter");
        assert_eq!(selected(&a), "m1");
    }
}
