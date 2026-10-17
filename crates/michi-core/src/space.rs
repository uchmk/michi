//! Whether the diffs ignore whitespace (`w`). One switch for the window: every
//! diff that is shown (a commit's, a comparison's, a file's uncommitted
//! change) is then read with `git diff -w`, and a diff read that way is for
//! looking at only, never for choosing lines to stage.

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

static IGNORE: AtomicBool = AtomicBool::new(false);

/// Whether diffs are read ignoring whitespace.
pub fn ignoring() -> bool {
    IGNORE.load(Ordering::Relaxed)
}

/// Flips the switch; what it is now.
pub fn toggle() -> bool {
    !IGNORE.fetch_xor(true, Ordering::Relaxed)
}

/// The option git gets for a diff read: `-w` when ignoring and the patch is
/// asked for (the lists of files stay as they are).
fn option(ignore: bool, patch: bool) -> Option<&'static str> {
    (ignore && patch).then_some("-w")
}

/// Adds the whitespace option to `c`, a `git diff` or `git show`.
pub fn apply(c: &mut Command, patch: bool) {
    if let Some(o) = option(ignoring(), patch) {
        c.arg(o);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    #[test]
    fn w_goes_only_with_the_patch() {
        assert_eq!(option(true, true), Some("-w"));
        assert_eq!(option(true, false), None);
        assert_eq!(option(false, true), None);
    }

    #[test]
    fn a_whitespace_only_change_has_no_hunk_with_w() {
        let t = TempRepo::configured("space-w");
        t.write(&t.repo, "a.txt", "one two\nx\n");
        t.git(&t.repo, &["add", "a.txt"]);
        t.git(&t.repo, &["commit", "-q", "-m", "a"]);
        t.write(&t.repo, "a.txt", "one  two\nx\n");
        let plain = t.out(&t.repo, &["diff", "--patch"]);
        assert!(plain.contains("+one  two"), "{plain}");
        let w = t.out(&t.repo, &["diff", option(true, true).unwrap(), "--patch"]);
        assert!(!w.contains("@@"), "{w}");
        // A real change still shows, and the words of it.
        t.write(&t.repo, "a.txt", "one  three\nx\n");
        let w = t.out(&t.repo, &["diff", "-w", "--patch"]);
        assert!(w.contains("+one  three"), "{w}");
    }
}
