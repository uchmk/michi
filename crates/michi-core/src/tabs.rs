//! The tabs of the window (#45): one repository each, in the order they sit
//! in the bar, and which one is in front. Kept as one path per line, the
//! front one marked with `* `. No egui, no disk.

use std::path::{Path, PathBuf};

use crate::paths::same_path;

/// The most tabs kept.
pub const MAX: usize = 12;

/// What the keys and the bar ask of the tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// The tab to the right; the last goes round to the first.
    Next,
    /// The tab to the left; the first goes round to the last.
    Prev,
    /// The tab in front is closed.
    Close,
    /// This tab is closed (the bar's cross, a middle click).
    CloseAt(usize),
    /// This tab comes to the front (a click, Alt+1..9).
    Goto(usize),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tabs {
    paths: Vec<PathBuf>,
    active: usize,
}

impl Tabs {
    /// One tab, in front.
    pub fn new(first: &Path) -> Self {
        Self { paths: vec![first.to_path_buf()], active: 0 }
    }

    /// The tabs `text` has, then `first` in front: its tab if there is one,
    /// else a new one next to the front tab. Lines that are blank or repeat
    /// a folder are let go, and so are those past [`MAX`].
    pub fn restore(text: &str, first: &Path) -> Self {
        let mut t = Self { paths: Vec::new(), active: 0 };
        for line in text.lines() {
            let line = line.trim();
            let line = line.strip_prefix("* ").unwrap_or(line).trim();
            if line.is_empty() || line.starts_with('#') || t.index_of(Path::new(line)).is_some() || t.paths.len() >= MAX - 1 {
                continue;
            }
            t.paths.push(PathBuf::from(line));
        }
        t.open(first);
        t
    }

    /// The tabs of `text` as they were left, the one marked `* ` in front;
    /// `first` alone when `text` has none. Lines are read as in [`Tabs::restore`].
    pub fn resume(text: &str, first: &Path) -> Self {
        let mut t = Self { paths: Vec::new(), active: 0 };
        for line in text.lines() {
            let line = line.trim();
            let (line, marked) = match line.strip_prefix("* ") {
                Some(rest) => (rest.trim(), true),
                None => (line, false),
            };
            if line.is_empty() || line.starts_with('#') || t.paths.len() >= MAX {
                continue;
            }
            match t.index_of(Path::new(line)) {
                Some(i) if marked => t.active = i,
                Some(_) => {}
                None => {
                    t.paths.push(PathBuf::from(line));
                    if marked {
                        t.active = t.paths.len() - 1;
                    }
                }
            }
        }
        if t.paths.is_empty() {
            return Self::new(first);
        }
        t
    }

    /// The tabs as text, for [`Tabs::restore`].
    pub fn text(&self) -> String {
        self.paths.iter().enumerate().map(|(i, p)| format!("{}{}\n", if i == self.active { "* " } else { "" }, p.display())).collect()
    }

    /// These folders as tabs with number `active` in front (the first when
    /// there is no such tab); repeats and those past [`MAX`] are let go.
    /// `first` alone when there is none.
    pub fn from_paths(paths: &[PathBuf], active: usize, first: &Path) -> Self {
        let mut t = Self { paths: Vec::new(), active: 0 };
        let mut front = None;
        for (i, p) in paths.iter().enumerate() {
            if p.as_os_str().is_empty() || t.paths.len() >= MAX {
                continue;
            }
            let at = match t.index_of(p) {
                Some(at) => at,
                None => {
                    t.paths.push(p.clone());
                    t.paths.len() - 1
                }
            };
            if i == active {
                front = Some(at);
            }
        }
        if t.paths.is_empty() {
            return Self::new(first);
        }
        t.active = front.unwrap_or(0);
        t
    }

    /// Drop the tabs whose folder `keep` refuses; the first tab comes to the
    /// front when the front one is dropped. Whether any tab is left.
    pub fn retain(&mut self, keep: impl Fn(&Path) -> bool) -> bool {
        let front = self.paths.get(self.active).cloned();
        self.paths.retain(|p| keep(p));
        self.active = front.and_then(|f| self.index_of(&f)).unwrap_or(0).min(self.paths.len().saturating_sub(1));
        !self.paths.is_empty()
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }

    pub fn len(&self) -> usize {
        self.paths.len()
    }

    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// The tab in front.
    pub fn active(&self) -> usize {
        self.active
    }

    pub fn index_of(&self, path: &Path) -> Option<usize> {
        self.paths.iter().position(|p| same_path(p, path))
    }

    /// `path` to the front: its own tab if it has one, else a new tab just
    /// right of the front one. Whether a tab was made; `None` when the bar
    /// is full and `path` has none.
    pub fn open(&mut self, path: &Path) -> Option<bool> {
        if let Some(i) = self.index_of(path) {
            self.active = i;
            return Some(false);
        }
        if self.paths.len() >= MAX {
            return None;
        }
        let at = if self.paths.is_empty() { 0 } else { self.active + 1 };
        self.paths.insert(at, path.to_path_buf());
        self.active = at;
        Some(true)
    }

    /// Tab `i` to the front. Whether `i` is a tab.
    pub fn activate(&mut self, i: usize) -> bool {
        if i >= self.paths.len() {
            return false;
        }
        self.active = i;
        true
    }

    pub fn next(&mut self) {
        if !self.paths.is_empty() {
            self.active = (self.active + 1) % self.paths.len();
        }
    }

    pub fn prev(&mut self) {
        if !self.paths.is_empty() {
            self.active = (self.active + self.paths.len() - 1) % self.paths.len();
        }
    }

    /// Tab `i` closed. The one to its right takes the front if it was in
    /// front, or the one to its left when it was the last; a tab left of
    /// the front one leaves the same tab in front. The only tab is not
    /// closed (the window is a repository): false then, and for no such tab.
    pub fn close(&mut self, i: usize) -> bool {
        if self.paths.len() < 2 || i >= self.paths.len() {
            return false;
        }
        self.paths.remove(i);
        if i < self.active || self.active == self.paths.len() {
            self.active -= 1;
        }
        true
    }

    /// The folder tab `i` is on, as the repository's top once that is known.
    pub fn set_path(&mut self, i: usize, path: &Path) {
        if let Some(p) = self.paths.get_mut(i) {
            *p = path.to_path_buf();
        }
    }

    /// What to show on each tab: the folder's name, and its parent's name
    /// after it where two tabs would read the same.
    pub fn labels(&self) -> Vec<String> {
        let leaf = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string());
        let parent = |p: &Path| p.parent().and_then(Path::file_name).map(|n| n.to_string_lossy().into_owned());
        self.paths
            .iter()
            .map(|p| {
                let name = leaf(p);
                let twin = self.paths.iter().filter(|q| !same_path(q, p) && leaf(q).eq_ignore_ascii_case(&name)).count() > 0;
                match (twin, parent(p)) {
                    (true, Some(up)) => format!("{name} ({up})"),
                    _ => name,
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn resume_keeps_the_front_tab_and_the_view() {
        let mut t = three();
        t.activate(1);
        assert_eq!(Tabs::resume(&t.text(), &p("/z")), t);
        assert_eq!(Tabs::from_paths(t.paths(), 1, &p("/z")), t);
        // A view line from a build before is no tab.
        assert_eq!(Tabs::restore("/a\n# view 2\n", &p("/a")).paths().len(), 1);
    }

    #[test]
    fn from_paths_lets_go_repeats_and_finds_the_front() {
        let t = Tabs::from_paths(&[p("/a"), p("/a"), p("/b")], 2, &p("/z"));
        assert_eq!((t.paths().to_vec(), t.active()), (vec![p("/a"), p("/b")], 1));
        let t = Tabs::from_paths(&[p("/a"), p("/b")], 9, &p("/z"));
        assert_eq!(t.active(), 0);
        assert_eq!(Tabs::from_paths(&[], 0, &p("/z")), Tabs::new(&p("/z")));
        assert_eq!(Tabs::from_paths(&[p("")], 0, &p("/z")), Tabs::new(&p("/z")));
    }

    #[test]
    fn resume_without_tabs_is_the_folder_asked_for() {
        assert_eq!(Tabs::resume("", &p("/z")), Tabs::new(&p("/z")));
        assert_eq!(Tabs::resume("# view 2\n\n", &p("/z")), Tabs::new(&p("/z")));
        assert_eq!(Tabs::resume("/a\n* /b\n/a\n", &p("/z")).active(), 1);
        // No mark: the first tab is in front.
        assert_eq!(Tabs::resume("/a\n/b\n", &p("/z")).active(), 0);
    }

    #[test]
    fn retain_drops_tabs_and_keeps_the_front_in_range() {
        let mut t = three();
        t.activate(1);
        assert!(t.retain(|q| q != Path::new("/b")));
        assert_eq!(t.paths(), [p("/a"), p("/c")]);
        assert_eq!(t.active(), 0);
        let mut t = three();
        t.activate(2);
        assert!(t.retain(|q| q != Path::new("/a")));
        assert_eq!((t.paths()[t.active()].clone(), t.len()), (p("/c"), 2));
        let mut t = three();
        assert!(t.retain(|q| q == Path::new("/a")));
        assert_eq!((t.len(), t.active()), (1, 0));
        assert!(!t.retain(|_| false));
    }

    fn three() -> Tabs {
        let mut t = Tabs::new(&p("/a"));
        t.open(&p("/b"));
        t.open(&p("/c"));
        t
    }

    #[test]
    fn a_new_tab_sits_right_of_the_front_one_and_takes_the_front() {
        let mut t = Tabs::new(&p("/a"));
        assert_eq!(t.open(&p("/b")), Some(true));
        assert_eq!(t.open(&p("/c")), Some(true));
        assert!(t.activate(0));
        assert_eq!(t.open(&p("/d")), Some(true));
        assert_eq!(t.paths(), &[p("/a"), p("/d"), p("/b"), p("/c")]);
        assert_eq!(t.active(), 1);
    }

    #[test]
    fn a_folder_with_a_tab_is_brought_to_the_front_not_opened_twice() {
        let mut t = three();
        assert_eq!(t.open(&p("/a/")), Some(false));
        assert_eq!(t.active(), 0);
        assert_eq!(t.len(), 3);
    }

    #[test]
    fn a_full_bar_takes_no_more() {
        let mut t = Tabs::new(&p("/0"));
        for n in 1..MAX {
            assert_eq!(t.open(&p(&format!("/{n}"))), Some(true));
        }
        assert_eq!(t.open(&p("/more")), None);
        assert_eq!(t.open(&p("/3")), Some(false), "but a tab it has still comes to the front");
        assert_eq!(t.len(), MAX);
    }

    #[test]
    fn next_and_prev_go_round() {
        let mut t = three();
        assert_eq!(t.active(), 2);
        t.next();
        assert_eq!(t.active(), 0);
        t.prev();
        assert_eq!(t.active(), 2);
        t.prev();
        assert_eq!(t.active(), 1);
        assert!(!t.activate(3));
        assert_eq!(t.active(), 1);
    }

    #[test]
    fn closing_the_front_tab_gives_the_front_to_its_right_then_its_left() {
        let mut t = three();
        t.activate(1);
        assert!(t.close(1));
        assert_eq!(t.paths(), &[p("/a"), p("/c")]);
        assert_eq!(t.active(), 1, "/c moved into the place");
        assert!(t.close(1));
        assert_eq!(t.paths(), &[p("/a")]);
        assert_eq!(t.active(), 0);
    }

    #[test]
    fn closing_another_tab_keeps_the_same_one_in_front() {
        let mut t = three();
        t.activate(1);
        assert!(t.close(0));
        assert_eq!(t.paths()[t.active()], p("/b"));
        assert!(t.close(1));
        assert_eq!(t.paths()[t.active()], p("/b"));
    }

    #[test]
    fn the_only_tab_stays() {
        let mut t = Tabs::new(&p("/a"));
        assert!(!t.close(0));
        assert!(!three().close(5));
        assert_eq!(t.len(), 1);
        t.next();
        t.prev();
        assert_eq!(t.active(), 0);
    }

    #[test]
    fn the_text_round_trips_with_the_front_tab_marked() {
        let mut t = three();
        t.activate(1);
        assert_eq!(t.text(), "/a\n* /b\n/c\n");
        assert_eq!(Tabs::restore(&t.text(), &p("/b")), t);
    }

    #[test]
    fn restoring_brings_the_asked_folder_to_the_front() {
        let t = Tabs::restore("* /a\n/b\n", &p("/b"));
        assert_eq!(t.paths(), &[p("/a"), p("/b")]);
        assert_eq!(t.active(), 1);
        let t = Tabs::restore("/a\n/b\n", &p("/z"));
        assert_eq!(t.paths(), &[p("/a"), p("/z"), p("/b")], "a new folder goes next to the first tab");
    }

    #[test]
    fn restoring_lets_go_of_blank_repeated_and_too_many() {
        let t = Tabs::restore("\n/a\n  \n* /a/\n/b\n", &p("/b"));
        assert_eq!(t.len(), 2);
        let many: String = (0..30).map(|n| format!("/r{n}\n")).collect();
        let t = Tabs::restore(&many, &p("/first"));
        assert_eq!(t.len(), MAX);
        assert_eq!(t.paths()[t.active()], p("/first"));
        assert_eq!(Tabs::restore("", &p("/x")), Tabs::new(&p("/x")));
    }

    #[test]
    fn two_tabs_of_one_name_tell_their_parents() {
        let mut t = Tabs::new(&p("/dev/michi"));
        t.open(&p("/work/Michi"));
        t.open(&p("/dev/kura"));
        assert_eq!(t.labels(), ["michi (dev)", "Michi (work)", "kura"]);
    }

    #[test]
    fn a_tab_learns_the_top_of_its_repository() {
        let mut t = Tabs::new(&p("/a/src"));
        t.set_path(0, &p("/a"));
        t.set_path(9, &p("/nothing"));
        assert_eq!(t.paths(), &[p("/a")]);
    }
}
