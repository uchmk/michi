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

    /// Whether the strip is up.
    pub(crate) fn strip_shown(&self) -> bool {
        self.detached.as_ref().is_some_and(|d| d.info.is_some())
    }

    /// What the strip says.
    pub(crate) fn strip_text(&self) -> Option<String> {
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
