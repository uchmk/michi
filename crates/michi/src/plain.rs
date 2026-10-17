//! Plain sentences in the window (#106, #107): the line under the title bar
//! that says where the worktree stands, and the forecast a band carries
//! before an operation. The sentences are michi-core's [`plain`] builders;
//! this wires them to what the app knows and words them. Nothing here
//! touches the disk.

use std::path::Path;

use michi_core::clash::Outcome;
use michi_core::plain::{self, Here, Outlook, Phrase, Remote};
use michi_core::summary::Head;

use crate::app::{App, Repo};
use crate::picker::same;

impl App {
    /// The sentences worded in the window's language, one after another.
    pub(crate) fn plain_words(&self, phrases: &[Phrase]) -> Vec<String> {
        phrases.iter().map(|p| p.fill(self.tr(p.key))).collect()
    }

    /// How many files of the worktree at `wt` have uncommitted changes.
    pub(crate) fn changed_in(&self, wt: &Path) -> usize {
        self.wips.iter().find(|w| same(&w.path, wt)).map_or(0, |w| w.files.len())
    }

    /// The branch the worktree at `wt` is on.
    fn branch_in(&self, wt: &Path) -> Option<String> {
        self.trees.trees.iter().find(|t| same(&t.wt.path, wt)).and_then(|t| t.wt.branch.clone())
    }

    /// What is known of the window's worktree, or `None` while the worktrees
    /// are not read (nothing is said then rather than something wrong).
    fn here(&self) -> Option<Here> {
        let Repo::Ready(s) = &self.repo else { return None };
        let tree = self.trees.trees.iter().find(|t| same(&t.wt.path, &s.top))?;
        let (branch, head) = match &s.head {
            Head::Branch(b) => (Some(b.clone()), String::new()),
            Head::Detached(h) => (None, h.clone()),
            Head::Bare => return None,
        };
        let remote = match &tree.upstream {
            None => Remote::None,
            Some(u) if u.gone => Remote::Gone(u.name.clone()),
            Some(u) => Remote::Tracking { name: u.name.clone(), ahead: u.ahead, behind: u.behind },
        };
        Some(Here { branch, head, doing: tree.doing, changed: self.changed_in(&s.top), remote })
    }

    /// "Where you are now" (#107): the line under the title bar.
    pub(crate) fn where_text(&self) -> Option<String> {
        let phrases = plain::where_now(&self.here()?);
        Some(self.plain_words(&phrases).join(" "))
    }

    /// How the merge of `other` (a trunk or a branch) into the worktree at
    /// `wt` is expected to go, from the prediction the worktree table keeps.
    pub(crate) fn outlook_with(&self, wt: &Path, other: &str) -> Outlook {
        let c = &self.clashes;
        let (Some(i), Some(j)) = (c.side_of(wt), c.sides.iter().position(|s| s.name == other)) else { return Outlook::Unknown };
        match c.between(i, j) {
            Some(Outcome::Clean) => Outlook::Clean,
            Some(Outcome::Committed(k) | Outcome::Uncommitted(k)) => Outlook::Conflicts(k.files.len()),
            Some(Outcome::Unknown) | None => Outlook::Unknown,
        }
    }

    /// The forecast of a pull of the worktree at `wt` (#106), when its
    /// branch follows something.
    pub(crate) fn pull_forecast(&self, wt: &Path) -> Option<Vec<String>> {
        let t = self.trees.trees.iter().find(|t| same(&t.wt.path, wt))?;
        let (b, u) = (t.wt.branch.as_deref()?, t.upstream.as_ref().filter(|u| !u.gone)?);
        Some(self.plain_words(&plain::pull_plan(b, &u.name, u.ahead, u.behind, self.changed_in(wt))))
    }

    /// The forecast of taking `rev` into the worktree at `wt` (#106).
    /// `commits`: how many are new to it, when known.
    pub(crate) fn merge_forecast(&self, wt: &Path, rev: &str, commits: Option<u32>, outlook: Outlook) -> Vec<String> {
        let branch = self.branch_in(wt).unwrap_or_default();
        self.plain_words(&plain::merge_plan(rev, &branch, commits, outlook))
    }

    /// The forecast of a switch of the worktree at `wt` to `name` (#106).
    pub(crate) fn switch_forecast(&self, wt: &Path, name: &str) -> Vec<String> {
        let from = self.branch_in(wt);
        self.plain_words(&plain::switch_plan(from.as_deref(), name, self.changed_in(wt)))
    }

    /// The forecast of a pick of `id` onto the worktree at `wt` (#106).
    pub(crate) fn pick_forecast(&self, wt: &Path, id: &str) -> Vec<String> {
        let branch = self.branch_in(wt).unwrap_or_default();
        self.plain_words(&plain::pick_plan(id, &branch, Outlook::Unknown))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::tests::read;
    use crate::graph::fixture;
    use crate::repo::Answer;
    use michi_core::summary::Summary;
    use std::path::PathBuf;

    fn summary(branch: &str, top: &str) -> Repo {
        Repo::Ready(Summary { name: "repo".into(), head: Head::Branch(branch.into()), worktrees: 3, top: PathBuf::from(top) })
    }

    fn app() -> App {
        let mut a = read();
        a.repo = summary("main", "C:/r/repo");
        a.take(Answer::Trees(fixture::trees()));
        a.take(Answer::Clashes(fixture::clashes()));
        a
    }

    #[test]
    fn nothing_is_said_before_the_worktrees_are_read() {
        let mut a = read();
        a.repo = summary("main", "C:/r/repo");
        assert_eq!(a.where_text(), None);
    }

    #[test]
    fn where_you_are_is_a_plain_line_in_each_language() {
        let mut a = app();
        let en = a.where_text().unwrap();
        assert!(en.starts_with("You are on main.") && en.contains("matches origin/main"), "{en}");
        a.lang = "ja".into();
        let ja = a.where_text().unwrap();
        assert!(ja.contains("main") && ja.contains("origin/main"), "{ja}");
        assert_ne!(en, ja);
    }

    #[test]
    fn a_branch_that_differs_from_its_remote_says_how() {
        let mut a = app();
        a.repo = summary("feature/wip", "C:/r/wt");
        let t = a.where_text().unwrap();
        assert!(t.contains("origin/wip") && t.contains("2 yours, 1 theirs"), "{t}");
    }

    #[test]
    fn a_merge_forecast_uses_the_prediction_the_table_keeps() {
        let a = app();
        let w = Path::new("C:/r/wt");
        assert_eq!(a.outlook_with(w, "main"), Outlook::Conflicts(2));
        assert_eq!(a.outlook_with(Path::new("C:/r/old"), "main"), Outlook::Clean);
        assert_eq!(a.outlook_with(w, "nope"), Outlook::Unknown);
        let text = a.merge_forecast(w, "main", Some(3), a.outlook_with(w, "main")).join(" ");
        assert!(text.contains("3 commit") && text.contains("2 file"), "{text}");
    }

    #[test]
    fn a_pull_forecast_needs_a_followed_branch() {
        let a = app();
        assert!(a.pull_forecast(Path::new("C:/r/repo")).unwrap()[0].contains("Nothing"));
        assert!(a.pull_forecast(Path::new("C:/r/old")).is_none());
        assert!(a.pull_forecast(Path::new("C:/r/wt")).unwrap()[0].contains("both"));
    }

    #[test]
    fn every_plain_key_has_words_in_both_languages() {
        for lang in crate::i18n::AVAILABLE {
            for k in plain::KEYS {
                assert_ne!(crate::i18n::tr(lang, k), *k, "{lang} lacks {k}");
            }
        }
    }
}
