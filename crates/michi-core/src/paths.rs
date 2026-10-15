//! Comparing the paths git prints with each other and with the ones a user or
//! tsumugi gives: the one place that knows Windows paths ignore case and take
//! either slash.

use std::path::Path;

/// Whether `a` and `b` name the same folder. On Windows `C:/dev/x`,
/// `c:\dev\x\` and `\\?\C:\dev\x` are the same; elsewhere only a trailing
/// slash is let go.
pub fn same_path(a: &Path, b: &Path) -> bool {
    key(a) == key(b)
}

/// Whether `p` is `dir` or somewhere below it. `C:\dev\a` holds `C:\dev\a\x`
/// but not `C:\dev\ab`.
pub fn contains(dir: &Path, p: &Path) -> bool {
    contains_key(&key(dir), &key(p))
}

/// [`contains`] for two [`key`]s.
pub fn contains_key(dir: &str, p: &str) -> bool {
    match p.strip_prefix(dir) {
        Some("") => true,
        Some(rest) => dir.ends_with('/') || rest.starts_with('/'),
        None => false,
    }
}

/// The form two paths are compared in.
pub fn key(p: &Path) -> String {
    key_of(&p.to_string_lossy(), cfg!(windows))
}

pub(crate) fn key_of(p: &str, windows: bool) -> String {
    if !windows {
        let t = p.trim_end_matches('/');
        return if t.is_empty() && !p.is_empty() { "/".into() } else { t.into() };
    }
    let p = p.strip_prefix(r"\\?\").unwrap_or(p);
    p.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_paths_ignore_case_and_slashes() {
        let k = |p| key_of(p, true);
        assert_eq!(k("C:/dev/michi"), k(r"c:\dev\Michi\"));
        assert_eq!(k(r"\\?\C:\dev\michi"), k("C:/dev/michi"));
        assert_eq!(k(r"\\server\share\x"), "//server/share/x");
        assert_ne!(k("C:/dev/michi"), k("C:/dev/michi-wt"));
    }

    #[test]
    fn a_folder_holds_what_is_below_it_and_not_a_longer_name() {
        for windows in [true, false] {
            let has = |d, p| contains_key(&key_of(d, windows), &key_of(p, windows));
            assert!(has("/dev/a", "/dev/a"));
            assert!(has("/dev/a/", "/dev/a/x/y"));
            assert!(has("/dev/a", "/dev/a/"));
            assert!(!has("/dev/a", "/dev/ab"));
            assert!(!has("/dev/a", "/dev/ab/x"));
            assert!(!has("/dev/a/x", "/dev/a"));
            assert!(has("/", "/dev"));
        }
        let has = |d, p| contains_key(&key_of(d, true), &key_of(p, true));
        assert!(has(r"C:\dev\a", r"c:/DEV/a/src"));
        assert!(!has(r"C:\dev\a", r"C:\dev\ab"));
        assert!(has("C:/", r"c:\dev"));
        assert!(!has("C:/", r"D:\dev"));
        // Elsewhere case counts.
        assert!(!contains_key(&key_of("/Dev/a", false), &key_of("/dev/a/x", false)));
    }

    #[test]
    fn other_paths_keep_their_case() {
        let k = |p| key_of(p, false);
        assert_eq!(k("/home/u/michi/"), k("/home/u/michi"));
        assert_ne!(k("/home/u/Michi"), k("/home/u/michi"));
        assert_eq!(k("/"), "/");
    }
}
