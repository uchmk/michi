//! Issue references in commit messages and branch names (#44): `#123`,
//! `GH-123`, `owner/repo#123`, and tracker keys such as `ABC-123` (Linear,
//! Jira). Nothing here touches the network; only [`slug_of`] reads the
//! remotes. A reference becomes a web address through [`Tracker`]: GitHub's
//! issue page from the remote's address, and a template from `[issues]` of
//! config.toml for keys. michi only links; the issue itself is read and
//! written in the browser (or in minato).

use std::path::Path;

use crate::{git, pr, remote};

/// One reference found in a text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ref {
    /// `#123` or `GH-123`: an issue of the repository's own GitHub remote.
    Number(u64),
    /// `owner/repo#123`.
    Repo { owner: String, repo: String, number: u64 },
    /// `ABC-123`, upper-cased: a key of Linear, Jira or the like.
    Key(String),
}

impl Ref {
    /// How it is written on screen.
    pub fn label(&self) -> String {
        match self {
            Ref::Number(n) => format!("#{n}"),
            Ref::Repo { owner, repo, number } => format!("{owner}/{repo}#{number}"),
            Ref::Key(k) => k.clone(),
        }
    }
}

/// `[issues]` of config.toml.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// Whether `#123` and the like link to the GitHub remote's issues.
    pub github: bool,
    /// Where a key goes: an address with `{key}` in it
    /// (`https://linear.app/acme/issue/{key}`). Keys are not linked without it.
    pub keys: Option<String>,
    /// The key prefixes that count (`ABC`); none listed: every `AB-123` does.
    pub prefixes: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self { github: true, keys: None, prefixes: Vec::new() }
    }
}

/// Whether `t` can be a key template: a web address with `{key}` in it.
pub fn valid_template(t: &str) -> bool {
    (t.starts_with("https://") || t.starts_with("http://")) && t.contains("{key}")
}

/// What turns a reference into an address: the config and the GitHub
/// repository the remote names.
pub struct Tracker<'a> {
    pub config: &'a Config,
    pub slug: Option<&'a (String, String)>,
}

impl Tracker<'_> {
    /// The page of `r`; `None` when it cannot be linked (no GitHub remote,
    /// no template for keys, a prefix that is not listed).
    pub fn url(&self, r: &Ref) -> Option<String> {
        match r {
            Ref::Number(n) if self.config.github => self.slug.map(|(o, p)| format!("https://github.com/{o}/{p}/issues/{n}")),
            Ref::Repo { owner, repo, number } if self.config.github => Some(format!("https://github.com/{owner}/{repo}/issues/{number}")),
            Ref::Number(_) | Ref::Repo { .. } => None,
            Ref::Key(k) => {
                let prefix = k.split('-').next().unwrap_or("");
                if !self.config.prefixes.is_empty() && !self.config.prefixes.iter().any(|p| p.eq_ignore_ascii_case(prefix)) {
                    return None;
                }
                self.config.keys.as_ref().map(|t| t.replace("{key}", k))
            }
        }
    }

    /// The references of a commit message that link, each with its address.
    pub fn in_message(&self, text: &str) -> Vec<(Ref, String)> {
        self.linked(find(text))
    }

    /// The references of a branch name that link, each with its address.
    pub fn in_branch(&self, branch: &str) -> Vec<(Ref, String)> {
        self.linked(find_branch(branch))
    }

    fn linked(&self, refs: Vec<Ref>) -> Vec<(Ref, String)> {
        refs.into_iter().filter_map(|r| self.url(&r).map(|u| (r, u))).collect()
    }
}

/// Words that look like a key and are not.
const NOT_KEYS: &[&str] = &[
    "UTF", "SHA", "ISO", "CVE", "AES", "RFC", "HTTP", "TLS", "SSL", "IPV", "ARM", "PEP", "BIP", "WIN", "CRLF", "RSA", "ISSUE", "ISSUES",
    "RELEASE", "VERSION", "HOTFIX", "BUGFIX", "FEATURE", "FEAT", "FIX", "BUG",
];

/// The number at the start of `s` (1 to 9 digits, not 0) and what follows.
fn digits(s: &str) -> (Option<u64>, &str) {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    if end == 0 || end > 9 {
        return (None, s);
    }
    (s[..end].parse().ok().filter(|n| *n > 0), &s[end..])
}

fn push(out: &mut Vec<Ref>, r: Ref) {
    if !out.contains(&r) {
        out.push(r);
    }
}

/// Every reference in a commit message, in order, once each.
pub fn find(text: &str) -> Vec<Ref> {
    let mut out = Vec::new();
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let (at, c) = chars[i];
        let prev = if i == 0 { None } else { Some(chars[i - 1].1) };
        let rest = &text[at..];
        if c == '#' && prev.is_none_or(|p| !(p.is_ascii_alphanumeric() || matches!(p, '_' | '&' | '/' | '#'))) {
            if let (Some(n), _) = digits(&rest[1..]) {
                push(&mut out, Ref::Number(n));
            }
        } else if c.is_ascii_alphabetic() && prev.is_none_or(|p| !(p.is_ascii_alphanumeric() || matches!(p, '_' | '-' | '.' | '/'))) {
            if let Some(r) = owner_repo(rest).or_else(|| key_at(rest)) {
                push(&mut out, r);
            }
        }
        i += 1;
        // The rest of a word already looked at is not a start.
        if c.is_ascii_alphanumeric() {
            while i < chars.len() && chars[i].1.is_ascii_alphanumeric() {
                i += 1;
            }
        }
    }
    out
}

/// `owner/repo#123` at the start of `s`.
fn owner_repo(s: &str) -> Option<Ref> {
    let owner_end = s.find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))?;
    let owner = &s[..owner_end];
    let rest = s[owner_end..].strip_prefix('/')?;
    let repo_end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')))?;
    let repo = &rest[..repo_end];
    let rest = rest[repo_end..].strip_prefix('#')?;
    let (n, tail) = digits(rest);
    if owner.is_empty() || repo.is_empty() || tail.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(Ref::Repo { owner: owner.to_owned(), repo: repo.to_owned(), number: n? })
}

/// `GH-123` or `ABC-123` at the start of `s` (`ABC` in capitals).
fn key_at(s: &str) -> Option<Ref> {
    let letters = s.find(|c: char| !c.is_ascii_alphanumeric()).unwrap_or(s.len());
    let prefix = &s[..letters];
    let rest = s[letters..].strip_prefix('-')?;
    let (n, tail) = digits(rest);
    let n = n?;
    // `CVE-2024-1234`, `ABC-12a`: the number must end the key.
    let more = tail.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_')
        || (tail.starts_with('-') && tail[1..].starts_with(|c: char| c.is_ascii_digit()));
    if more {
        return None;
    }
    if prefix.eq_ignore_ascii_case("gh") {
        return Some(Ref::Number(n));
    }
    let shape = prefix.len() >= 2
        && prefix.len() <= 10
        && prefix.chars().take(2).all(|c| c.is_ascii_uppercase())
        && prefix.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    if !shape || NOT_KEYS.contains(&prefix) {
        return None;
    }
    Some(Ref::Key(format!("{prefix}-{n}")))
}

/// The references a branch name carries: `issue-123`, `gh-123`, `123-fix-x`,
/// `ABC-123-fix-x` (any case), in any folder part: `feature/ABC-123-x`. A
/// date like `tsumugi/1016-1200` is not one.
pub fn find_branch(branch: &str) -> Vec<Ref> {
    let mut out = Vec::new();
    for seg in branch.split('/') {
        let lower = seg.to_ascii_lowercase();
        let named = ["issues-", "issue-", "gh-", "issues_", "issue_", "gh_"].iter().find_map(|p| lower.strip_prefix(p));
        if let Some(rest) = named {
            if let (Some(n), tail) = digits(rest) {
                if !tail.starts_with(|c: char| c.is_ascii_alphanumeric()) {
                    push(&mut out, Ref::Number(n));
                }
            }
            continue;
        }
        if let (Some(n), tail) = digits(&lower) {
            // `123-fix-x`, not `1016-1200` or `2024`.
            if tail.starts_with(['-', '_']) && tail[1..].starts_with(|c: char| c.is_ascii_alphabetic()) {
                push(&mut out, Ref::Number(n));
            }
            continue;
        }
        if let Some(Ref::Key(k)) = key_at(&seg.to_ascii_uppercase()) {
            push(&mut out, Ref::Key(k));
        }
    }
    out
}

/// `(owner, repo)` of the GitHub remote of the repository `dir` is in: `origin`
/// when that is on GitHub, else the first remote that is.
pub fn slug_of(dir: &Path) -> Result<Option<(String, String)>, git::Error> {
    let all = remote::list(dir)?;
    let on_github = |r: &&remote::Remote| pr::github_slug(&r.url).is_some();
    let pick = all.iter().find(|r| r.name == "origin").filter(on_github).or_else(|| all.iter().find(on_github));
    Ok(pick.and_then(|r| pr::github_slug(&r.url)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(text: &str) -> Vec<String> {
        find(text).iter().map(Ref::label).collect()
    }

    #[test]
    fn numbers_and_gh_keys_are_found_once_in_order() {
        assert_eq!(keys("Fix crash (#12), see GH-7 and #12 again"), ["#12", "#7"]);
        assert_eq!(keys("Merge pull request #7 from a/b"), ["#7"]);
        assert_eq!(keys("fixes #1, closes #2.\n\nRefs #30"), ["#1", "#2", "#30"]);
        assert_eq!(keys("#5 at the start"), ["#5"]);
    }

    #[test]
    fn what_is_not_a_reference_is_left_alone() {
        for t in ["C# is fine", "color #fff", "&#123;", "abc#12", "https://x.y/z#12", "issue #", "#0", "#abc", "version #1234567890123"] {
            let got = keys(t);
            assert!(got.is_empty(), "{t}: {got:?}");
        }
    }

    #[test]
    fn a_reference_in_another_repository_names_it() {
        assert_eq!(keys("Same as uchmk/ito#9 and uchmk/ito#9"), ["uchmk/ito#9"]);
        assert_eq!(find("see a-b/c_d.e#3"), vec![Ref::Repo { owner: "a-b".into(), repo: "c_d.e".into(), number: 3 }]);
        assert!(keys("src/main.rs#3x").is_empty());
    }

    #[test]
    fn tracker_keys_are_found_in_capitals_only() {
        assert_eq!(keys("ABC-123: fix; ENG-9 too"), ["ABC-123", "ENG-9"]);
        assert_eq!(keys("(PROJ2-45)"), ["PROJ2-45"]);
        for t in ["abc-123", "UTF-8 text", "SHA-256 sum", "CVE-2024-1234", "A-1", "x ABC-12a", "ABC_DEF-12", "pre-ABC-12"] {
            let got = keys(t);
            assert!(got.is_empty(), "{t}: {got:?}");
        }
    }

    #[test]
    fn a_branch_name_carries_its_issue() {
        let b = |s: &str| find_branch(s).iter().map(Ref::label).collect::<Vec<_>>();
        assert_eq!(b("issue-123"), ["#123"]);
        assert_eq!(b("fix/gh-45-crash"), ["#45"]);
        assert_eq!(b("123-fix-crash"), ["#123"]);
        assert_eq!(b("feature/ABC-123-add-x"), ["ABC-123"]);
        assert_eq!(b("feature/abc-123-add-x"), ["ABC-123"]);
        assert_eq!(b("alice/eng-9"), ["ENG-9"]);
        for n in ["main", "tsumugi/1016-1200", "release-2024", "2024-01-02-x", "fix-the-thing", "v1-2"] {
            assert!(b(n).is_empty(), "{n}: {:?}", b(n));
        }
    }

    #[test]
    fn addresses_follow_the_remote_and_the_template() {
        let cfg = Config { keys: Some("https://linear.app/acme/issue/{key}".into()), ..Config::default() };
        let slug = ("uchmk".to_owned(), "michi".to_owned());
        let t = Tracker { config: &cfg, slug: Some(&slug) };
        assert_eq!(t.url(&Ref::Number(5)).as_deref(), Some("https://github.com/uchmk/michi/issues/5"));
        let other = Ref::Repo { owner: "a".into(), repo: "b".into(), number: 1 };
        assert_eq!(t.url(&other).as_deref(), Some("https://github.com/a/b/issues/1"));
        assert_eq!(t.url(&Ref::Key("ENG-1".into())).as_deref(), Some("https://linear.app/acme/issue/ENG-1"));
        let linked = t.in_message("ENG-1 and #5");
        assert_eq!(linked.len(), 2);
        assert_eq!(linked[0].1, "https://linear.app/acme/issue/ENG-1");
        assert_eq!(t.in_branch("feature/eng-2-x")[0].1, "https://linear.app/acme/issue/ENG-2");
    }

    #[test]
    fn what_cannot_link_is_not_offered() {
        let none = Config::default();
        let t = Tracker { config: &none, slug: None };
        assert!(t.in_message("#5 ENG-1").is_empty(), "no GitHub remote and no template");
        let off = Config { github: false, ..Config::default() };
        let slug = ("a".to_owned(), "b".to_owned());
        assert!(Tracker { config: &off, slug: Some(&slug) }.in_message("#5 a/b#1").is_empty());
        let only = Config { keys: Some("https://j.example/browse/{key}".into()), prefixes: vec!["abc".into()], ..Config::default() };
        let t = Tracker { config: &only, slug: None };
        assert_eq!(t.in_message("ABC-1 XYZ-2").len(), 1);
    }

    #[test]
    fn a_template_needs_an_address_and_the_key() {
        assert!(valid_template("https://x.atlassian.net/browse/{key}"));
        assert!(!valid_template("https://x.atlassian.net/browse/"));
        assert!(!valid_template("javascript:{key}"));
        assert!(!valid_template("ftp://x/{key}"));
    }

    #[test]
    fn the_github_remote_is_origin_or_the_first() {
        let r = crate::testrepo::TempRepo::new("issues-slug");
        assert_eq!(slug_of(&r.repo).unwrap(), None);
        r.git(&r.repo, &["remote", "add", "up", "git@github.com:other/thing.git"]);
        assert_eq!(slug_of(&r.repo).unwrap(), Some(("other".into(), "thing".into())));
        r.git(&r.repo, &["remote", "add", "origin", "https://github.com/uchmk/michi.git"]);
        assert_eq!(slug_of(&r.repo).unwrap(), Some(("uchmk".into(), "michi".into())));
    }
}
