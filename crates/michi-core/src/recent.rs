//! The repositories michi was opened on: the list the repository screen
//! shows. The pinned ones come first, in the order they were pinned; the rest
//! follow, newest first. Kept as TOML (`paths`, `pinned`); no egui, no disk.

use std::path::{Path, PathBuf};

/// The most paths kept.
pub const MAX: usize = 30;

#[derive(serde::Serialize, serde::Deserialize)]
struct Kept {
    #[serde(default)]
    paths: Vec<String>,
    /// Kept on top, in pin order. A file of an older build has none.
    #[serde(default)]
    pinned: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recent {
    /// The others, newest first.
    paths: Vec<PathBuf>,
    /// The pinned ones, in pin order; `MAX` does not cut them.
    pinned: Vec<PathBuf>,
}

impl Recent {
    /// The list as `text` has it: TOML (`paths = [...]`, newest first); a
    /// file of an older build has one path per line. Blank lines let go.
    pub fn parse(text: &str) -> Self {
        let (lines, pins): (Vec<String>, Vec<String>) = match toml::from_str::<Kept>(text) {
            Ok(k) => (k.paths, k.pinned),
            Err(_) => (text.lines().map(str::to_owned).collect(), Vec::new()),
        };
        let mut r = Self::default();
        for line in pins.iter().map(|l| l.trim()).filter(|l| !l.is_empty()) {
            r.pin(Path::new(line));
        }
        for line in lines.iter().rev().map(|l| l.trim()).filter(|l| !l.is_empty()) {
            r.push(Path::new(line));
        }
        r
    }

    /// The list as TOML, newest first.
    pub fn text(&self) -> String {
        let show = |v: &[PathBuf]| v.iter().map(|p| p.display().to_string()).collect();
        toml::to_string(&Kept { paths: show(&self.paths), pinned: show(&self.pinned) }).unwrap_or_default()
    }

    /// `path` to the front of the others; an earlier entry for the same
    /// folder goes. A pinned folder stays where it is pinned.
    pub fn push(&mut self, path: &Path) {
        if self.is_pinned(path) {
            return;
        }
        self.paths.retain(|p| !crate::paths::same_path(p, path));
        self.paths.insert(0, path.to_path_buf());
        self.paths.truncate(MAX);
    }

    /// `path` pinned, at the end of the pinned ones (it leaves the others).
    pub fn pin(&mut self, path: &Path) {
        if self.is_pinned(path) {
            return;
        }
        self.paths.retain(|p| !crate::paths::same_path(p, path));
        self.pinned.push(path.to_path_buf());
    }

    /// `path` let go of its pin: it is the newest of the others. Whether it was pinned.
    pub fn unpin(&mut self, path: &Path) -> bool {
        let Some(at) = self.pinned.iter().position(|p| crate::paths::same_path(p, path)) else { return false };
        let p = self.pinned.remove(at);
        self.push(&p);
        true
    }

    pub fn is_pinned(&self, path: &Path) -> bool {
        self.pinned.iter().any(|p| crate::paths::same_path(p, path))
    }

    /// The pin comes off a pinned folder, or goes on an unpinned one.
    pub fn toggle_pin(&mut self, path: &Path) {
        if !self.unpin(path) {
            self.pin(path);
        }
    }

    /// Whether something was taken out, from the pinned and the others both.
    pub fn forget(&mut self, path: &Path) -> bool {
        let before = self.paths.len() + self.pinned.len();
        self.paths.retain(|p| !crate::paths::same_path(p, path));
        self.pinned.retain(|p| !crate::paths::same_path(p, path));
        self.paths.len() + self.pinned.len() != before
    }

    /// Every folder as the screen lists it: the pinned, then the others.
    pub fn paths(&self) -> Vec<PathBuf> {
        self.pinned.iter().chain(&self.paths).cloned().collect()
    }

    /// The pinned folders, in pin order.
    pub fn pinned(&self) -> &[PathBuf] {
        &self.pinned
    }
}

/// Whether a repository at `path` belongs in the list: not one made to be
/// thrown away, that is, nothing under one of `temps` (the temporary
/// folder) and no practice repository (`michi-practice-N`).
pub fn worth_remembering(path: &Path, temps: &[&Path]) -> bool {
    let practice = path.file_name().is_some_and(|n| n.to_string_lossy().starts_with(crate::practice::FOLDER_PREFIX));
    !practice && !temps.iter().any(|t| crate::paths::contains(t, path))
}

/// Whether what was typed names a place rather than a search: a drive or
/// a slash, `~`, or `.` at the start.
pub fn looks_like_path(s: &str) -> bool {
    let s = s.trim();
    let b = s.as_bytes();
    s.starts_with(['/', '\\', '~', '.']) || (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':')
}

/// `~` at the start of a typed path is the home folder.
pub fn expand(s: &str, home: Option<&Path>) -> PathBuf {
    let s = s.trim();
    match (s.strip_prefix('~'), home) {
        (Some(rest), Some(h)) if rest.is_empty() || rest.starts_with(['/', '\\']) => h.join(rest.trim_start_matches(['/', '\\'])),
        _ => PathBuf::from(s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn the_newest_is_first_and_a_folder_is_there_once() {
        let mut r = Recent::default();
        r.push(&p("/a"));
        r.push(&p("/b"));
        r.push(&p("/a/"));
        assert_eq!(r.paths(), [p("/a/"), p("/b")]);
    }

    #[test]
    fn the_list_has_a_limit() {
        let mut r = Recent::default();
        for n in 0..MAX + 5 {
            r.push(&p(&format!("/r{n}")));
        }
        assert_eq!(r.paths().len(), MAX);
        assert_eq!(r.paths()[0], p(&format!("/r{}", MAX + 4)));
    }

    #[test]
    fn the_text_round_trips() {
        let mut r = Recent::default();
        r.push(&p("/a"));
        r.push(&p("/b"));
        assert_eq!(Recent::parse(&r.text()), r);
        assert_eq!(Recent::parse("\n  \n/x\n\n/y\n").paths(), [p("/x"), p("/y")]);
        // A Windows path keeps its backslashes through TOML.
        let mut w = Recent::default();
        w.push(&p(r"C:\dev\michi"));
        assert!(w.text().starts_with("paths = "));
        assert_eq!(Recent::parse(&w.text()), w);
    }

    #[test]
    fn forgetting_says_whether_it_did() {
        let mut r = Recent::parse("/a\n/b\n");
        assert!(r.forget(&p("/a")));
        assert!(!r.forget(&p("/a")));
        assert_eq!(r.paths(), [p("/b")]);
    }

    #[test]
    fn pinned_ones_come_first_in_pin_order_and_the_limit_does_not_cut_them() {
        let mut r = Recent::default();
        r.push(&p("/a"));
        r.push(&p("/b"));
        r.push(&p("/c"));
        r.pin(&p("/a"));
        r.pin(&p("/c"));
        assert_eq!(r.paths(), [p("/a"), p("/c"), p("/b")]);
        assert_eq!(r.pinned(), [p("/a"), p("/c")]);
        // Opening a pinned folder does not move it or list it twice.
        r.push(&p("/a"));
        assert_eq!(r.paths(), [p("/a"), p("/c"), p("/b")]);
        for n in 0..MAX + 5 {
            r.push(&p(&format!("/r{n}")));
        }
        assert_eq!(r.pinned().len(), 2);
        assert_eq!(r.paths().len(), MAX + 2);
        assert!(!r.paths().contains(&p("/b")), "the others are cut, the pinned are not");
    }

    #[test]
    fn unpinning_puts_it_back_among_the_others_as_the_newest() {
        let mut r = Recent::default();
        r.push(&p("/a"));
        r.push(&p("/b"));
        r.pin(&p("/a"));
        assert!(r.is_pinned(&p("/a")));
        assert!(r.unpin(&p("/a")));
        assert!(!r.unpin(&p("/a")));
        assert_eq!(r.paths(), [p("/a"), p("/b")]);
        r.toggle_pin(&p("/b"));
        assert_eq!(r.pinned(), [p("/b")]);
        r.toggle_pin(&p("/b"));
        assert!(r.pinned().is_empty());
    }

    #[test]
    fn forgetting_a_pinned_one_takes_the_pin_too() {
        let mut r = Recent::default();
        r.push(&p("/a"));
        r.pin(&p("/a"));
        assert!(r.forget(&p("/a")));
        assert!(r.paths().is_empty() && r.pinned().is_empty());
    }

    #[test]
    fn the_pins_round_trip_and_an_old_file_has_none() {
        let mut r = Recent::default();
        r.push(&p("/a"));
        r.push(&p("/b"));
        r.push(&p("/c"));
        r.pin(&p("/b"));
        r.pin(&p("/a"));
        let text = r.text();
        assert!(text.contains("pinned"));
        assert_eq!(Recent::parse(&text), r);
        assert_eq!(Recent::parse(&text).paths(), [p("/b"), p("/a"), p("/c")]);
        // A TOML file of 0.109 has no `pinned`.
        let old = Recent::parse("paths = [\"/x\", \"/y\"]\n");
        assert_eq!(old.paths(), [p("/x"), p("/y")]);
        assert!(old.pinned().is_empty());
    }

    #[test]
    fn throwaway_repositories_are_not_remembered() {
        let tmp = p("/tmp");
        assert!(!worth_remembering(&p("/tmp/m63"), &[&tmp]));
        assert!(!worth_remembering(&p("/tmp/deep/er/x"), &[&tmp]));
        assert!(!worth_remembering(&p("/home/u/michi-practice-3"), &[&tmp]), "a practice repository anywhere");
        assert!(worth_remembering(&p("/home/u/dev/michi"), &[&tmp]));
        assert!(worth_remembering(&p("/tmpx/a"), &[&tmp]), "only what is under it");
        assert!(worth_remembering(&p("/tmp"), &[]), "no temporary folder known");
    }

    #[test]
    fn a_path_is_told_from_a_search() {
        for yes in ["C:\\dev\\x", "c:/x", "/home/x", "~/x", "~", "./x", "..\\x", "\\\\host\\share"] {
            assert!(looks_like_path(yes), "{yes}");
        }
        for no in ["michi", "my repo", "c", "dev/x"] {
            assert!(!looks_like_path(no), "{no}");
        }
    }

    #[test]
    fn the_home_folder_stands_for_a_tilde() {
        let h = p("/home/u");
        assert_eq!(expand("~", Some(&h)), h);
        assert_eq!(expand("~/dev/x", Some(&h)), h.join("dev/x"));
        assert_eq!(expand("~x", Some(&h)), p("~x"));
        assert_eq!(expand(" /a ", Some(&h)), p("/a"));
        assert_eq!(expand("~/a", None), p("~/a"));
    }
}
