//! The repositories michi was opened on, newest first: the list the
//! repository screen shows. Kept as one path per line; no egui, no disk.

use std::path::{Path, PathBuf};

/// The most paths kept.
pub const MAX: usize = 30;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Recent {
    paths: Vec<PathBuf>,
}

impl Recent {
    /// The list as `text` has it: one path per line, blank lines let go.
    pub fn parse(text: &str) -> Self {
        let mut r = Self::default();
        for line in text.lines().rev().map(str::trim).filter(|l| !l.is_empty()) {
            r.push(Path::new(line));
        }
        r
    }

    /// The list as text, newest first.
    pub fn text(&self) -> String {
        self.paths.iter().map(|p| format!("{}\n", p.display())).collect()
    }

    /// `path` to the front; an earlier entry for the same folder goes.
    pub fn push(&mut self, path: &Path) {
        self.forget(path);
        self.paths.insert(0, path.to_path_buf());
        self.paths.truncate(MAX);
    }

    /// Whether something was taken out.
    pub fn forget(&mut self, path: &Path) -> bool {
        let before = self.paths.len();
        self.paths.retain(|p| !crate::paths::same_path(p, path));
        self.paths.len() != before
    }

    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
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
        assert_eq!(r.paths(), &[p("/a/"), p("/b")]);
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
        assert_eq!(Recent::parse("\n  \n/x\n\n/y\n").paths(), &[p("/x"), p("/y")]);
    }

    #[test]
    fn forgetting_says_whether_it_did() {
        let mut r = Recent::parse("/a\n/b\n");
        assert!(r.forget(&p("/a")));
        assert!(!r.forget(&p("/a")));
        assert_eq!(r.paths(), &[p("/b")]);
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
