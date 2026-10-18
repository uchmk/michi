//! Looks before a commit (issues #110, #111): is the branch one to keep clear of
//! direct commits (`main`, `trunk`), and do the staged files look wrong to
//! commit (a huge file, a key or `.env`, a token inside)? The check is small,
//! table-driven and conservative: a name or a pattern that is nearly always a
//! secret, so a warning is rare enough to be read. It asks, never forbids.

use std::path::Path;

use crate::git;
use crate::oid::Oid;
use crate::show::{File, Status};

/// Branches no commit is made on without asking, when `[commit] protect`
/// does not say.
pub const PROTECTED: [&str; 3] = ["main", "master", "trunk"];

/// A staged file this big (bytes) or bigger is warned of (50 MiB, where
/// GitHub starts to warn too).
pub const LARGE: u64 = 50 * 1024 * 1024;

/// A staged file smaller than this is noted for its size only if it is
/// suspected of something else: keeps the notes few, so the limit can be
/// changed in the settings without a new read.
const NOTE_FLOOR: u64 = 1024 * 1024;

/// The most a blob may be to be read for tokens in it.
const SCAN_MAX: u64 = 256 * 1024;

/// The most blobs read for tokens in one look. Past it the rest is counted,
/// not read: see [`scan_counted`].
const SCAN_FILES: usize = 1000;

/// `[commit] protect` and `large_mb` of config.toml.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// Branch names (with `*`) a commit asks about; none: never.
    pub protect: Vec<String>,
    /// Bytes from which a staged file is warned of; 0: never.
    pub large: u64,
}

impl Default for Config {
    fn default() -> Config {
        Config { protect: PROTECTED.iter().map(|s| (*s).to_owned()).collect(), large: LARGE }
    }
}

/// Whether `branch` is one of `protect` (a name; `*` stands for any run of
/// characters in it, as in `release/*`).
pub fn is_protected(branch: &str, protect: &[String]) -> bool {
    protect.iter().any(|p| glob(p, branch))
}

/// `pat` against `text`, with `*` for any run of characters.
fn glob(pat: &str, text: &str) -> bool {
    let parts: Vec<&str> = pat.split('*').collect();
    if parts.len() == 1 {
        return pat == text;
    }
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if text.len() < first.len() + last.len() || !text.starts_with(first) || !text.ends_with(last) {
        return false;
    }
    let mut rest = &text[first.len()..text.len() - last.len()];
    for mid in &parts[1..parts.len() - 1] {
        match rest.find(mid) {
            Some(k) => rest = &rest[k + mid.len()..],
            None => return false,
        }
    }
    true
}

/// Why a staged file looks like a secret.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Secret {
    /// Its name: `.env`, a private key's file.
    Name(&'static str),
    /// Something in it: the header of a private key, a token of a service.
    Content(&'static str),
}

/// A staged file worth a word before it is committed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub path: String,
    /// Bytes.
    pub size: u64,
    pub secret: Option<Secret>,
}

impl Note {
    /// Whether it is warned of when files of `limit` bytes are too big
    /// (0: size does not matter).
    pub fn counts(&self, limit: u64) -> bool {
        self.secret.is_some() || self.is_large(limit)
    }

    pub fn is_large(&self, limit: u64) -> bool {
        limit > 0 && self.size >= limit
    }
}

/// Names that are nearly always a secret, as (what, matcher on the file's
/// lower-case name).
type NameRule = (&'static str, fn(&str) -> bool);
const NAMES: [NameRule; 5] = [
    (".env file", |n| {
        n == ".env"
            || n.strip_prefix(".env.").is_some_and(|s| !["example", "sample", "template", "dist", "defaults", "schema"].contains(&s))
    }),
    ("private key file", |n| {
        [".pem", ".key", ".p12", ".pfx", ".ppk", ".jks", ".keystore"].iter().any(|e| n.ends_with(e))
            || (n.ends_with(".asc") && n.contains("private"))
    }),
    ("SSH private key", |n| ["id_rsa", "id_dsa", "id_ecdsa", "id_ed25519"].contains(&n)),
    ("credentials file", |n| n == ".netrc" || n == "_netrc" || n == "credentials.json" || n == "service-account.json"),
    ("AWS credentials", |n| n == "credentials" || n == ".git-credentials"),
];

/// What the file named `path` is suspected of by its name.
pub fn secret_name(path: &str) -> Option<Secret> {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    NAMES.iter().find(|(_, is)| is(&name)).map(|(what, _)| Secret::Name(what))
}

/// Tokens that start with a fixed word, as (what, prefix, characters after,
/// how many after).
type TokenRule = (&'static str, &'static str, fn(u8) -> bool, usize);
const TOKENS: [TokenRule; 6] = [
    ("AWS access key id", "AKIA", |b| b.is_ascii_uppercase() || b.is_ascii_digit(), 16),
    ("GitHub token", "ghp_", |b| b.is_ascii_alphanumeric(), 36),
    ("GitHub token", "github_pat_", |b| b.is_ascii_alphanumeric() || b == b'_', 40),
    ("Slack token", "xoxb-", |b| b.is_ascii_alphanumeric() || b == b'-', 24),
    ("Anthropic API key", "sk-ant-api", |b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_', 30),
    ("Google API key", "AIza", |b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_', 35),
];

/// What `bytes` is suspected of by what is in it.
pub fn secret_content(bytes: &[u8]) -> Option<Secret> {
    if contains(bytes, b"-----BEGIN ") {
        for kind in ["RSA ", "EC ", "DSA ", "OPENSSH ", "ENCRYPTED ", "PGP ", ""] {
            if contains(bytes, format!("-----BEGIN {kind}PRIVATE KEY").as_bytes()) {
                return Some(Secret::Content("private key"));
            }
        }
    }
    for (what, prefix, class, n) in TOKENS {
        let p = prefix.as_bytes();
        let mut from = 0;
        while let Some(k) = find(&bytes[from..], p) {
            let at = from + k + p.len();
            // Not the tail of a longer word, and `n` token characters after.
            let before_ok = from + k == 0 || !bytes[from + k - 1].is_ascii_alphanumeric();
            if before_ok && bytes.len() >= at + n && bytes[at..at + n].iter().all(|&b| class(b)) {
                return Some(Secret::Content(what));
            }
            from = from + k + 1;
        }
    }
    None
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    find(hay, needle).is_some()
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// The staged `files` of the worktree at `wt` that are big or suspected:
/// by name, and by the content of those small enough to read. Reads blobs
/// with two `cat-file` calls at most.
pub fn scan(wt: &Path, files: &[File]) -> Result<Vec<Note>, git::Error> {
    scan_counted(wt, files, SCAN_FILES).map(|(notes, _)| notes)
}

/// [`scan`] reading the content of `max_read` files at most, and how many
/// small text files it left unread past that (their names and sizes were
/// still looked at), so the caller can say the look was partial.
pub fn scan_counted(wt: &Path, files: &[File], max_read: usize) -> Result<(Vec<Note>, usize), git::Error> {
    let mut unread = 0;
    let blobs: Vec<(&File, Oid)> =
        files.iter().filter(|f| !matches!(f.status, Status::Deleted | Status::Unmerged)).filter_map(|f| f.blob.map(|b| (f, b))).collect();
    if blobs.is_empty() {
        return Ok((Vec::new(), 0));
    }
    let sizes = sizes(wt, &blobs.iter().map(|(_, b)| *b).collect::<Vec<_>>())?;
    let mut notes: Vec<Note> = Vec::new();
    let mut read: Vec<(usize, Oid)> = Vec::new();
    for ((f, blob), size) in blobs.iter().zip(&sizes) {
        // A commit (a submodule) or anything else that is not a blob.
        let Some(size) = *size else { continue };
        let secret = secret_name(&f.path);
        if secret.is_some() || size >= NOTE_FLOOR {
            notes.push(Note { path: f.path.clone(), size, secret });
        } else if size <= SCAN_MAX && !f.is_binary() {
            if read.len() >= max_read {
                unread += 1;
                continue;
            }
            read.push((notes.len(), *blob));
            // A place kept for it, dropped again if nothing is found.
            notes.push(Note { path: f.path.clone(), size, secret: None });
        }
    }
    if !read.is_empty() {
        let ids: Vec<Oid> = read.iter().map(|(_, b)| *b).collect();
        let contents = crate::conflict::blobs(wt, &ids)?;
        for (at, blob) in &read {
            if let Some(s) = contents.get(blob).and_then(|c| secret_content(c)) {
                notes[*at].secret = Some(s);
            }
        }
    }
    notes.retain(|n| n.secret.is_some() || n.size >= NOTE_FLOOR);
    Ok((notes, unread))
}

/// The sizes of the blobs `ids` (`cat-file --batch-check`); `None` for one
/// that is not a blob.
pub(crate) fn sizes(wt: &Path, ids: &[Oid]) -> Result<Vec<Option<u64>>, git::Error> {
    let mut c = git::command();
    c.arg("-C").arg(wt).args(["cat-file", "--batch-check"]);
    let input: String = ids.iter().map(|i| format!("{i}\n")).collect();
    let out = git::feed(c, input.as_bytes())?;
    let text = String::from_utf8_lossy(&out);
    let mut lines = text.lines();
    Ok(ids
        .iter()
        .map(|_| {
            // `<oid> blob <size>`, or `<oid> missing`.
            let mut w = lines.next()?.split(' ');
            (w.nth(1)? == "blob").then(|| w.next().and_then(|s| s.parse().ok())).flatten()
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn protected_branches_match_by_name_or_star() {
        let p = names(&["main", "release/*", "*-prod"]);
        assert!(is_protected("main", &p));
        assert!(is_protected("release/1.2", &p));
        assert!(is_protected("eu-prod", &p));
        assert!(!is_protected("feature/main", &p));
        assert!(!is_protected("mainline", &p));
        assert!(!is_protected("main", &[]));
    }

    #[test]
    fn names_that_hold_secrets_are_found_and_samples_are_not() {
        for p in [".env", "app/.env.local", "keys/server.pem", "a/id_rsa", "x.PFX", ".netrc", ".aws/credentials"] {
            assert!(secret_name(p).is_some(), "{p}");
        }
        for p in [".env.example", "id_rsa.pub", "src/environment.rs", "credentials.rs", "monkey.txt", "keyboard.rs"] {
            assert_eq!(secret_name(p), None, "{p}");
        }
    }

    #[test]
    fn tokens_and_keys_are_found_in_content_and_near_misses_are_not() {
        assert_eq!(secret_content(b"x\n-----BEGIN OPENSSH PRIVATE KEY-----\n"), Some(Secret::Content("private key")));
        assert_eq!(secret_content(b"-----BEGIN PRIVATE KEY-----"), Some(Secret::Content("private key")));
        assert_eq!(secret_content(b"id = AKIAIOSFODNN7EXAMPLE\n"), Some(Secret::Content("AWS access key id")));
        let gh = format!("t={}", "ghp_".to_owned() + &"a1".repeat(18));
        assert_eq!(secret_content(gh.as_bytes()), Some(Secret::Content("GitHub token")));
        // A public key or certificate, a short word, a longer word it ends.
        assert_eq!(secret_content(b"-----BEGIN PUBLIC KEY-----"), None);
        assert_eq!(secret_content(b"-----BEGIN CERTIFICATE-----"), None);
        assert_eq!(secret_content(b"AKIA is a prefix"), None);
        assert_eq!(secret_content(b"XAKIAIOSFODNN7EXAMPLE"), None);
        assert_eq!(secret_content(b"fn main() {}"), None);
    }

    fn staged(t: &TempRepo, files: &[(&str, Vec<u8>)]) -> Vec<File> {
        for (name, body) in files {
            let p = t.repo.join(name);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
        }
        t.git(&t.repo, &["add", "-A"]);
        crate::stage::lists(&t.repo).unwrap().staged
    }

    #[test]
    fn a_scan_of_the_index_finds_names_content_and_size() {
        let t = TempRepo::configured("guard-scan");
        let files = staged(
            &t,
            &[
                ("ok.txt", b"hello\n".to_vec()),
                ("prod.env.txt", b"x".to_vec()),
                (".env", b"A=1\n".to_vec()),
                ("notes.txt", b"key: AKIAIOSFODNN7EXAMPLE\n".to_vec()),
                ("big.bin", vec![0u8; 2 * 1024 * 1024]),
                ("deep/id_ed25519", b"x".to_vec()),
            ],
        );
        let notes = scan(&t.repo, &files).unwrap();
        let by = |p: &str| notes.iter().find(|n| n.path == p).unwrap_or_else(|| panic!("{p}: {notes:?}"));
        assert_eq!(by(".env").secret, Some(Secret::Name(".env file")));
        assert_eq!(by("deep/id_ed25519").secret, Some(Secret::Name("SSH private key")));
        assert_eq!(by("notes.txt").secret, Some(Secret::Content("AWS access key id")));
        assert_eq!((by("big.bin").secret, by("big.bin").size), (None, 2 * 1024 * 1024));
        assert!(!by("big.bin").counts(LARGE) && by("big.bin").counts(1024 * 1024));
        assert_eq!(notes.len(), 4, "{notes:?}");
    }

    #[test]
    fn files_past_the_read_cap_are_counted_not_dropped_silently() {
        let t = TempRepo::configured("guard-cap");
        let files =
            staged(&t, &[("a.txt", b"a\n".to_vec()), ("b.txt", b"key AKIAIOSFODNN7EXAMPLE\n".to_vec()), ("c.txt", b"c\n".to_vec())]);
        let (notes, unread) = scan_counted(&t.repo, &files, 1).unwrap();
        assert_eq!(unread, 2);
        assert!(notes.is_empty(), "{notes:?}");
        let (notes, unread) = scan_counted(&t.repo, &files, 10).unwrap();
        assert_eq!((unread, notes.len()), (0, 1));
    }

    #[test]
    fn nothing_staged_or_only_deletions_is_nothing() {
        let t = TempRepo::configured("guard-none");
        assert_eq!(scan(&t.repo, &[]).unwrap(), vec![]);
        let files = staged(&t, &[("a.txt", b"a\n".to_vec())]);
        t.git(&t.repo, &["commit", "-q", "-m", "a"]);
        t.git(&t.repo, &["rm", "-q", "a.txt"]);
        let del = crate::stage::lists(&t.repo).unwrap().staged;
        assert_eq!(files.len(), 1);
        assert_eq!(scan(&t.repo, &del).unwrap(), vec![]);
    }
}
