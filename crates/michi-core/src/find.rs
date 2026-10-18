//! The graph's filter: what is typed is split at spaces, and a commit is hit
//! when every word hits it. A word hits when it is the start of the commit's
//! hash, or when its letters come in order (ito-match's fuzzy rule, the one
//! the palettes use) in the subject, the author or one of the names. Case is
//! ignored.
//!
//! A word that starts with `m:` (message), `f:` (all of them), `s:` or `g:` (and what follows it, spaces too)
//! asks for a content search instead: `s:text` is git's pickaxe (`log -S`,
//! the commits that add or remove that text), `g:regex` is `log -G` (the
//! commits whose diff has a line matching the regex). The words before it
//! still filter by the rules above.

use std::collections::HashSet;
use std::path::Path;

use ito_match::fuzzy;

use crate::git;
use crate::history::History;
use crate::log::Commit;
use crate::oid::Oid;

/// How a content search matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// `s:` - `git log -S`: the number of occurrences of the text changes.
    Text,
    /// `g:` - `git log -G`: a changed line matches the regex (any case).
    Regex,
    /// `p:` - `git log -- <path>`: the commit changes a path that contains the text.
    Path,
    /// `d:` - `git log --since --until`: the commit date is in `from..to`.
    Date,
    /// `m:` - `git log --grep`: the message contains the text (any case).
    Message,
    /// `f:` - the message, the diff text or a path: the three above at once.
    Full,
}

/// A content search typed in the filter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Content {
    pub kind: Kind,
    pub text: String,
}

/// Splits `text` into the part that filters by words and the content search
/// after it, if any. An empty search (`s:` alone) is none.
fn split(text: &str) -> (&str, Option<Content>) {
    let mut at = 0;
    for word in text.split_inclusive(char::is_whitespace) {
        let kind = match word.get(..2) {
            Some("s:") => Some(Kind::Text),
            Some("g:") => Some(Kind::Regex),
            Some("p:") => Some(Kind::Path),
            Some("d:") => Some(Kind::Date),
            Some("m:") => Some(Kind::Message),
            Some("f:") => Some(Kind::Full),
            _ => None,
        };
        if let Some(kind) = kind {
            let body = text[at + 2..].trim();
            let content = (!body.is_empty()).then(|| Content { kind, text: body.to_owned() });
            return (&text[..at], content);
        }
        at += word.len();
    }
    (text, None)
}

/// The commits of every branch, remote branch and tag whose changes match
/// `content`, found by git (slow in a big repository: ask off the UI thread).
pub fn search(wt: &Path, content: &Content) -> Result<HashSet<Oid>, git::Error> {
    if content.kind == Kind::Full {
        let mut all = HashSet::new();
        for kind in [Kind::Message, Kind::Path, Kind::Text] {
            all.extend(search(wt, &Content { kind, text: content.text.clone() })?);
        }
        return Ok(all);
    }
    let args = search_args(content);
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let out = git::read(wt, &args)?;
    Ok(String::from_utf8_lossy(&out).lines().filter_map(|l| Oid::parse(l.trim())).collect())
}

/// The arguments of the `git log` that answers `content`.
fn search_args(content: &Content) -> Vec<String> {
    let mut args: Vec<String> = ["log", "--branches", "--remotes", "--tags", "--format=%H", "--no-ext-diff"].map(String::from).into();
    let mut pathspec = None;
    match content.kind {
        Kind::Text => args.push(format!("-S{}", content.text)),
        Kind::Regex => {
            args.push("--regexp-ignore-case".into());
            args.push(format!("-G{}", content.text));
        }
        Kind::Message => {
            args.push("--regexp-ignore-case".into());
            args.push("--fixed-strings".into());
            args.push(format!("--grep={}", content.text));
        }
        // `search` splits it into the three; asked directly it is the message.
        Kind::Full => args.push(format!("--grep={}", content.text)),
        Kind::Path => pathspec = Some(format!(":(icase)*{}*", content.text)),
        Kind::Date => {
            let (from, to) = content.text.split_once("..").unwrap_or((&content.text, ""));
            for (flag, v) in [("--since", from.trim()), ("--until", to.trim())] {
                if !v.is_empty() {
                    args.push(format!("{flag}={v}"));
                }
            }
        }
    }
    args.push("--".into());
    args.extend(pathspec);
    args
}

/// What the filter looks in for one commit.
#[derive(Clone, Debug)]
pub struct Hay {
    /// The commit.
    id: Oid,
    /// The hash, in lower case.
    hash: Box<str>,
    /// The subject, the author and the names, in lower case.
    fields: Vec<Box<str>>,
}

impl Hay {
    pub fn of(c: &Commit, history: &History) -> Hay {
        let mut fields: Vec<Box<str>> = vec![c.subject.to_lowercase().into(), c.author.to_lowercase().into()];
        if let Some(l) = history.labels(&c.id) {
            for n in l.locals.iter().chain(&l.tags) {
                fields.push(n.to_lowercase().into());
            }
            for (r, n) in &l.remotes {
                fields.push(format!("{r}/{n}").to_lowercase().into());
            }
        }
        Hay { id: c.id, hash: c.id.to_string().to_lowercase().into(), fields }
    }

    /// Whether the filter `query` (see [`Query::new`]) hits it.
    pub fn hits(&self, query: &Query) -> bool {
        self.hits_in(query, None)
    }

    /// [`hits`](Self::hits), and with a content search in `query`, the commit
    /// must be among `found` (what [`search`] gave; none yet: no hit).
    pub fn hits_in(&self, query: &Query, found: Option<&HashSet<Oid>>) -> bool {
        if query.content.is_some() && !found.is_some_and(|f| f.contains(&self.id)) {
            return false;
        }
        query.words.iter().all(|w| self.hash.starts_with(w.as_str()) || self.fields.iter().any(|f| fuzzy::match_str(w, f, false).is_some()))
    }
}

/// What is typed in the filter.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Query {
    words: Vec<String>,
    content: Option<Content>,
}

impl Query {
    /// The words of `text`, in lower case, and its content search. Neither,
    /// no filter.
    pub fn new(text: &str) -> Query {
        let (words, content) = split(text);
        Query { words: words.split_whitespace().map(str::to_lowercase).collect(), content }
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty() && self.content.is_none()
    }

    /// The content search typed, if any.
    pub fn content(&self) -> Option<&Content> {
        self.content.as_ref()
    }

    /// Whether `self` can only hit what `shorter` hit: typing on (a word
    /// grows, or a word is added) looks only among the hits.
    pub fn narrows(&self, shorter: &Query) -> bool {
        if shorter.is_empty() || self.words.len() < shorter.words.len() || shorter.content.is_some() && shorter.content != self.content {
            return false;
        }
        // Every word of the shorter query is the start of a word here, in order.
        let mut mine = self.words.iter();
        shorter.words.iter().all(|s| mine.any(|m| m.starts_with(s.as_str())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oid::Oid;

    fn hay(hash: &str, fields: &[&str]) -> Hay {
        Hay {
            id: Oid::parse(&"cd".repeat(20)).unwrap(),
            hash: hash.into(),
            fields: fields.iter().map(|f| f.to_lowercase().into()).collect(),
        }
    }

    #[test]
    fn letters_in_order_hit_a_field() {
        let h = hay("abc123", &["Fix the flaky test", "michi", "origin/feat"]);
        assert!(h.hits(&Query::new("fxtst")), "letters in order");
        assert!(h.hits(&Query::new("FLAKY")), "any case");
        assert!(h.hits(&Query::new("mch")), "the author");
        assert!(h.hits(&Query::new("orgn/ft")), "a name");
        assert!(!h.hits(&Query::new("tsetx")), "out of order");
    }

    #[test]
    fn the_hash_is_matched_by_its_start_only() {
        let h = hay("abc123", &["subject"]);
        assert!(h.hits(&Query::new("abc1")));
        assert!(!h.hits(&Query::new("bc12")), "not from the middle");
        assert!(!h.hits(&Query::new("a1")), "not by letters in order");
    }

    #[test]
    fn every_word_must_hit() {
        let h = hay("abc123", &["fix the flaky test", "michi"]);
        assert!(h.hits(&Query::new("flaky michi")));
        assert!(h.hits(&Query::new("  abc fix ")));
        assert!(!h.hits(&Query::new("flaky nobody")));
    }

    #[test]
    fn an_empty_query_has_no_words() {
        assert!(Query::new("   ").is_empty());
        assert!(!Query::new("a").is_empty());
    }

    #[test]
    fn s_and_g_ask_for_a_content_search() {
        let q = Query::new("fix s:foo bar");
        assert_eq!(q.content(), Some(&Content { kind: Kind::Text, text: "foo bar".into() }));
        assert!(!q.is_empty());
        let q = Query::new("g:^fn +main");
        assert_eq!(q.content(), Some(&Content { kind: Kind::Regex, text: "^fn +main".into() }));
        assert!(Query::new("s:").is_empty(), "an empty search is none");
        assert!(Query::new("s: ").content().is_none());
        assert!(Query::new("abs:x").content().is_none(), "only at the start of a word");
        assert!(!Query::new("abs:x").is_empty());
    }

    #[test]
    fn a_content_search_needs_the_commit_among_the_found() {
        let h = hay("abc123", &["fix the flaky test"]);
        let q = Query::new("fix s:foo");
        let mut found = HashSet::new();
        assert!(!h.hits_in(&q, None), "not searched yet");
        assert!(!h.hits_in(&q, Some(&found)));
        found.insert(Oid::parse(&"cd".repeat(20)).unwrap());
        assert!(h.hits_in(&q, Some(&found)));
        assert!(!h.hits_in(&Query::new("nobody s:foo"), Some(&found)), "the words still count");
    }

    #[test]
    fn content_searches_search_a_real_history() {
        use crate::testrepo::TempRepo;
        let r = TempRepo::new("find-content");
        r.write(&r.repo, "a.txt", "hello\n");
        r.git(&r.repo, &["add", "."]);
        r.git(&r.repo, &["commit", "-q", "-m", "one"]);
        r.write(&r.repo, "a.txt", "hello\nneedle here\n");
        r.git(&r.repo, &["commit", "-q", "-a", "-m", "two"]);
        let two = Oid::parse(&r.out(&r.repo, &["rev-parse", "HEAD"])).unwrap();
        r.write(&r.repo, "b.txt", "other\n");
        r.git(&r.repo, &["add", "."]);
        r.git(&r.repo, &["commit", "-q", "-m", "three"]);
        let text = search(&r.repo, &Content { kind: Kind::Text, text: "needle".into() }).unwrap();
        assert!(text.len() == 1 && text.contains(&two));
        let re = search(&r.repo, &Content { kind: Kind::Regex, text: "NEEDLE.h".into() }).unwrap();
        assert!(re.len() == 1 && re.contains(&two));
        assert!(search(&r.repo, &Content { kind: Kind::Regex, text: "(".into() }).is_err(), "a bad regex is refused");
    }

    #[test]
    fn path_and_date_searches_ask_git_for_them() {
        let q = Query::new("fix p:src/Main.rs");
        assert_eq!(q.content(), Some(&Content { kind: Kind::Path, text: "src/Main.rs".into() }));
        assert_eq!(Query::new("d:2026-10-01..2026-10-15").content().map(|c| c.kind), Some(Kind::Date));
        let a = search_args(&Content { kind: Kind::Path, text: "src".into() });
        assert_eq!(&a[a.len() - 2..], ["--", ":(icase)*src*"]);
        let a = search_args(&Content { kind: Kind::Date, text: "2026-10-01..2026-10-15".into() });
        assert!(a.contains(&"--since=2026-10-01".to_owned()) && a.contains(&"--until=2026-10-15".to_owned()));
        let a = search_args(&Content { kind: Kind::Date, text: "..yesterday".into() });
        assert!(a.contains(&"--until=yesterday".to_owned()) && !a.iter().any(|x| x.starts_with("--since")));
        let a = search_args(&Content { kind: Kind::Date, text: "2.weeks.ago".into() });
        assert!(a.contains(&"--since=2.weeks.ago".to_owned()) && !a.iter().any(|x| x.starts_with("--until")));
    }

    #[test]
    fn path_and_date_searches_search_a_real_history() {
        use crate::testrepo::TempRepo;
        let r = TempRepo::new("find-path-date");
        r.write(
            &r.repo,
            "src/a.txt",
            "a
",
        );
        r.git(&r.repo, &["add", "."]);
        r.git(&r.repo, &["commit", "-q", "-m", "one"]);
        r.write(
            &r.repo,
            "docs/b.txt",
            "b
",
        );
        r.git(&r.repo, &["add", "."]);
        r.git(&r.repo, &["commit", "-q", "-m", "two"]);
        let two = Oid::parse(&r.out(&r.repo, &["rev-parse", "HEAD"])).unwrap();
        let by_path = search(&r.repo, &Content { kind: Kind::Path, text: "DOCS".into() }).unwrap();
        assert!(by_path.len() == 1 && by_path.contains(&two), "a part of the path, any case");
        let all = search(&r.repo, &Content { kind: Kind::Date, text: "2000-01-01..2090-01-01".into() }).unwrap();
        assert!(all.len() >= 2 && all.contains(&two));
        let none = search(&r.repo, &Content { kind: Kind::Date, text: "..2000-01-01".into() }).unwrap();
        assert!(none.is_empty());
    }

    #[test]
    fn m_and_f_ask_for_the_message_and_everything() {
        assert_eq!(Query::new("m:fix it").content(), Some(&Content { kind: Kind::Message, text: "fix it".into() }));
        assert_eq!(Query::new("f:x").content().map(|c| c.kind), Some(Kind::Full));
        let a = search_args(&Content { kind: Kind::Message, text: "a.b".into() });
        assert!(a.contains(&"--grep=a.b".to_owned()) && a.contains(&"--fixed-strings".to_owned()));
    }

    #[test]
    fn deep_searches_search_a_real_history() {
        use crate::testrepo::TempRepo;
        let r = TempRepo::new("find-deep");
        r.write(
            &r.repo, "a.txt", "x
",
        );
        r.git(&r.repo, &["add", "."]);
        r.git(&r.repo, &["commit", "-q", "-m", "Alpha thing"]);
        let one = Oid::parse(&r.out(&r.repo, &["rev-parse", "HEAD"])).unwrap();
        r.write(
            &r.repo, "b.txt", "needle
",
        );
        r.git(&r.repo, &["add", "."]);
        r.git(&r.repo, &["commit", "-q", "-m", "second"]);
        let two = Oid::parse(&r.out(&r.repo, &["rev-parse", "HEAD"])).unwrap();
        let msg = search(&r.repo, &Content { kind: Kind::Message, text: "ALPHA".into() }).unwrap();
        assert!(msg.len() == 1 && msg.contains(&one));
        let full = search(&r.repo, &Content { kind: Kind::Full, text: "needle".into() }).unwrap();
        assert!(full.len() == 1 && full.contains(&two), "the diff text");
        let full = search(&r.repo, &Content { kind: Kind::Full, text: "alpha".into() }).unwrap();
        assert!(full.contains(&one), "the message");
        let full = search(&r.repo, &Content { kind: Kind::Full, text: "b.txt".into() }).unwrap();
        assert!(full.contains(&two), "the path");
    }

    #[test]
    fn typing_on_narrows() {
        let q = Query::new;
        assert!(q("ab").narrows(&q("a")));
        assert!(q("ab cd").narrows(&q("ab")));
        assert!(q("ab cd").narrows(&q("ab c")));
        assert!(!q("a").narrows(&q("ab")));
        assert!(!q("xy").narrows(&q("a")));
        assert!(!q("a").narrows(&q("")), "from nothing is a fresh search");
        assert!(q("a s:x").narrows(&q("a")), "a search added");
        assert!(!q("s:xy").narrows(&q("s:x")), "another search is fresh");
    }

    #[test]
    fn a_commit_is_read_from_its_labels() {
        let c = Commit {
            id: Oid::parse(&"ab".repeat(20)).unwrap(),
            parents: Vec::new(),
            author: "Ann".into(),
            email: String::new(),
            time: 0,
            subject: "Hello".into(),
        };
        let h = Hay::of(&c, &History::default());
        assert!(h.hits(&Query::new("abab")) && h.hits(&Query::new("hlo ann")));
    }
}
