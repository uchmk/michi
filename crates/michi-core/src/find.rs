//! The graph's filter: what is typed is split at spaces, and a commit is hit
//! when every word hits it. A word hits when it is the start of the commit's
//! hash, or when its letters come in order (ito-match's fuzzy rule, the one
//! the palettes use) in the subject, the author or one of the names. Case is
//! ignored.

use ito_match::fuzzy;

use crate::history::History;
use crate::log::Commit;

/// What the filter looks in for one commit.
#[derive(Clone, Debug)]
pub struct Hay {
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
        Hay { hash: c.id.to_string().to_lowercase().into(), fields }
    }

    /// Whether the filter `query` (see [`Query::new`]) hits it.
    pub fn hits(&self, query: &Query) -> bool {
        query.words.iter().all(|w| self.hash.starts_with(w.as_str()) || self.fields.iter().any(|f| fuzzy::match_str(w, f, false).is_some()))
    }
}

/// What is typed in the filter.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Query {
    words: Vec<String>,
}

impl Query {
    /// The words of `text`, in lower case. No word, no filter.
    pub fn new(text: &str) -> Query {
        Query { words: text.split_whitespace().map(str::to_lowercase).collect() }
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// Whether `self` can only hit what `shorter` hit: typing on (a word
    /// grows, or a word is added) looks only among the hits.
    pub fn narrows(&self, shorter: &Query) -> bool {
        if shorter.is_empty() || self.words.len() < shorter.words.len() {
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
        Hay { hash: hash.into(), fields: fields.iter().map(|f| f.to_lowercase().into()).collect() }
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
    fn typing_on_narrows() {
        let q = Query::new;
        assert!(q("ab").narrows(&q("a")));
        assert!(q("ab cd").narrows(&q("ab")));
        assert!(q("ab cd").narrows(&q("ab c")));
        assert!(!q("a").narrows(&q("ab")));
        assert!(!q("xy").narrows(&q("a")));
        assert!(!q("a").narrows(&q("")), "from nothing is a fresh search");
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
