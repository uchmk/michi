//! The strip under the title bar when the window's worktree is on no branch
//! (#109): why it is, how many commits only it holds, and the ways out:
//! `R` back to the branch it came from, `B` a branch made from here. The
//! reading (the reflog, the tags) is the act thread's; nothing here touches
//! the disk.

use michi_core::detached::{Info, Why};
use michi_core::summary::Head;

use crate::act::Task;
use crate::app::{App, Repo};

/// What is known of the detached HEAD of the window's worktree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Strip {
    /// The short hash the reading is for: a new HEAD asks again.
    pub head: String,
    /// None until the first reading is back.
    pub info: Option<Info>,
}

impl App {
    /// The worktrees were read: the window's one is on a branch (the strip
    /// goes), or on a new detached HEAD (it is read again, the last reading
    /// staying up meanwhile).
    pub(crate) fn detached_sync(&mut self) {
        let now = match &self.repo {
            Repo::Ready(s) => match &s.head {
                Head::Detached(h) => Some((h.clone(), s.top.clone())),
                _ => None,
            },
            _ => None,
        };
        let Some((head, wt)) = now else {
            self.detached = None;
            return;
        };
        if self.detached.as_ref().is_some_and(|d| d.head == head) {
            return;
        }
        let info = self.detached.take().and_then(|d| d.info);
        self.detached = Some(Strip { head, info });
        self.act(Task::ReadDetached { wt });
    }

    /// The reading is back.
    pub(crate) fn detached_read(&mut self, head_of: &std::path::Path, info: Info) {
        let Repo::Ready(s) = &self.repo else { return };
        if !crate::picker::same(&s.top, head_of) {
            return;
        }
        if let Some(d) = &mut self.detached {
            d.info = Some(info);
        }
    }

    /// The worktrees were read: the window's worktree is asked whether it is
    /// a sparse checkout (#31). The answer replaces the last one.
    pub(crate) fn sparse_sync(&mut self) {
        let Repo::Ready(s) = &self.repo else { return };
        let wt = s.top.clone();
        self.act(Task::ReadSparse { wt });
    }

    /// The reading is back (none: sparse checkout is off).
    pub(crate) fn sparse_read(&mut self, wt: &std::path::Path, info: Option<michi_core::sparse::Info>) {
        let Repo::Ready(s) = &self.repo else { return };
        if crate::picker::same(&s.top, wt) {
            self.sparse = info;
        }
    }

    /// Whether the strip is up.
    pub(crate) fn strip_shown(&self) -> bool {
        self.detached.as_ref().is_some_and(|d| d.info.is_some()) || self.sparse.is_some() || self.bisect_text().is_some()
    }

    /// The sentence about a sparse checkout: it is on, and how many files
    /// are left out of the folder.
    fn sparse_text(&self) -> Option<String> {
        let i = self.sparse.as_ref()?;
        let key = if i.cone { "strip.sparse_cone" } else { "strip.sparse" };
        Some(self.tr(key).replace("{n}", &i.left_out.to_string()))
    }

    /// What the strip says.
    pub(crate) fn strip_text(&self) -> Option<String> {
        // A bisect leaves the worktree on no branch, and says so itself.
        if let Some(bisect) = self.bisect_text() {
            return Some(bisect);
        }
        let sparse = self.sparse_text();
        let Some(detached) = self.detached_text() else { return sparse };
        Some(match sparse {
            Some(s) => format!("{detached} {s}"),
            None => detached,
        })
    }

    /// What the strip says of a detached HEAD.
    fn detached_text(&self) -> Option<String> {
        let d = self.detached.as_ref()?;
        let i = d.info.as_ref()?;
        let mut text = match &i.why {
            Why::Rebase => self.tr("strip.rebase").to_owned(),
            Why::Bisect => self.tr("strip.bisect").to_owned(),
            Why::Tag(t) => self.tr("strip.tag").replace("{tag}", t).replace("{head}", &d.head),
            Why::Checkout => self.tr("strip.commit").replace("{head}", &d.head),
        };
        if i.only_here > 0 && matches!(i.why, Why::Tag(_) | Why::Checkout) {
            text.push(' ');
            text.push_str(&self.tr("strip.only_here").replace("{n}", &i.only_here.to_string()));
        }
        Some(text)
    }

    /// The keys the strip offers, with the words on its buttons, in the
    /// order they are shown. A rebase or a bisect ends by itself: none.
    pub(crate) fn strip_actions(&self) -> Vec<(char, String)> {
        let bisect = self.bisect_actions();
        if !bisect.is_empty() {
            return bisect;
        }
        let Some(i) = self.detached.as_ref().and_then(|d| d.info.as_ref()) else { return Vec::new() };
        if matches!(i.why, Why::Rebase | Why::Bisect) {
            return Vec::new();
        }
        let mut v = Vec::new();
        if let Some(b) = &i.from {
            v.push(('R', self.tr("strip.back").replace("{b}", b)));
        }
        v.push(('B', self.tr("strip.branch").to_owned()));
        v
    }

    /// `R` and `B` while the strip is up. Returns whether it was used.
    pub(crate) fn strip_key(&mut self, c: char) -> bool {
        if !self.strip_actions().iter().any(|(k, _)| *k == c) {
            return false;
        }
        if !self.bisect_actions().is_empty() {
            return self.bisect_key(c);
        }
        match c {
            'R' => self.strip_back(),
            _ => self.open_picker_here(),
        }
        true
    }

    fn strip_back(&mut self) {
        let Some(name) = self.detached.as_ref().and_then(|d| d.info.as_ref()).and_then(|i| i.from.clone()) else { return };
        let Repo::Ready(s) = &self.repo else { return };
        let wt = s.top.clone();
        self.switch_back(Task::Back { wt: wt.clone(), name }, &wt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use michi_core::summary::Summary;

    use crate::app::tests::read;
    use crate::graph::fixture;
    use crate::repo::Answer;

    fn detached_app() -> App {
        let mut a = read();
        let top = PathBuf::from("C:/r/repo");
        a.repo = Repo::Ready(Summary { name: "repo".into(), head: Head::Branch("main".into()), worktrees: 3, top });
        a.take(Answer::Trees(fixture::trees()));
        if let Repo::Ready(s) = &mut a.repo {
            s.head = Head::Detached("1111111".into());
        }
        a
    }

    fn top(a: &App) -> PathBuf {
        match &a.repo {
            Repo::Ready(s) => s.top.clone(),
            _ => unreachable!(),
        }
    }

    fn info(why: Why, from: Option<&str>, only_here: usize) -> Info {
        Info { why, from: from.map(str::to_owned), only_here }
    }

    #[test]
    fn nothing_is_up_before_the_reading_is_back() {
        let mut a = detached_app();
        a.detached_sync();
        assert!(!a.strip_shown());
        assert!(a.strip_actions().is_empty());
    }

    #[test]
    fn the_strip_says_why_and_offers_the_ways_out() {
        let mut a = detached_app();
        a.detached_sync();
        let top = top(&a);
        a.detached_read(&top, info(Why::Tag("v1".into()), Some("main"), 2));
        assert!(a.strip_shown());
        let text = a.strip_text().unwrap();
        assert!(text.contains("v1") && text.contains("1111111") && text.contains("Commits on no branch yet: 2"), "{text}");
        let keys: Vec<char> = a.strip_actions().iter().map(|(k, _)| *k).collect();
        assert_eq!(keys, ['R', 'B']);
        assert!(a.strip_actions()[0].1.contains("main"));
        // Without a branch to go back to, only the new branch.
        a.detached_read(&top, info(Why::Checkout, None, 0));
        assert_eq!(a.strip_actions().iter().map(|(k, _)| *k).collect::<Vec<_>>(), ['B']);
        assert!(!a.strip_key('R'));
    }

    #[test]
    fn a_rebase_or_bisect_offers_nothing() {
        let mut a = detached_app();
        a.detached_sync();
        let top = top(&a);
        a.detached_read(&top, info(Why::Rebase, Some("main"), 3));
        assert!(a.strip_shown());
        assert!(a.strip_actions().is_empty());
        assert!(!a.strip_key('R') && !a.strip_key('B'));
        assert!(!a.strip_text().unwrap().contains("no branch yet"));
    }

    fn sparse(cone: bool, left_out: usize) -> Option<michi_core::sparse::Info> {
        Some(michi_core::sparse::Info { cone, left_out })
    }

    #[test]
    fn a_sparse_checkout_gets_the_strip_with_no_buttons() {
        let mut a = detached_app();
        assert!(!a.strip_shown());
        let top = top(&a);
        a.sparse_read(&top, sparse(true, 12));
        assert!(a.strip_shown());
        let text = a.strip_text().unwrap();
        assert!(text.contains("Sparse checkout") && text.contains("12"), "{text}");
        assert!(a.strip_actions().is_empty());
        a.sparse_read(&top, sparse(false, 3));
        assert!(a.strip_text().unwrap().contains("patterns"));
        // Turned off: the strip goes.
        a.sparse_read(&top, None);
        assert!(!a.strip_shown() && a.strip_text().is_none());
    }

    #[test]
    fn a_reading_for_another_worktree_is_dropped() {
        let mut a = detached_app();
        a.sparse_read(std::path::Path::new("C:/elsewhere"), sparse(true, 1));
        assert!(a.sparse.is_none());
    }

    #[test]
    fn sparse_and_detached_say_both() {
        let mut a = detached_app();
        a.detached_sync();
        let top = top(&a);
        a.detached_read(&top, info(Why::Checkout, None, 0));
        a.sparse_read(&top, sparse(true, 5));
        let text = a.strip_text().unwrap();
        assert!(text.contains("Not on a branch") && text.contains("Sparse checkout"), "{text}");
        // The buttons are the detached HEAD's.
        assert_eq!(a.strip_actions().len(), 1);
    }

    #[test]
    fn a_branch_takes_the_strip_away() {
        let mut a = detached_app();
        a.detached_sync();
        assert!(a.detached.is_some());
        if let Repo::Ready(s) = &mut a.repo {
            s.head = Head::Branch("main".into());
        }
        a.detached_sync();
        assert!(a.detached.is_none());
    }

    #[test]
    fn b_opens_the_branch_palette_here() {
        let mut a = detached_app();
        a.detached_sync();
        let top = top(&a);
        a.detached_read(&top, info(Why::Checkout, Some("main"), 0));
        assert!(a.strip_key('B'));
        assert!(a.picker.is_some());
    }
}
