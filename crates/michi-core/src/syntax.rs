//! Syntax colours for a diff: which kind of token each stretch of a line is.
//! Only the kind (keyword, string, ...) is decided here; the window maps it to
//! a colour of the theme, so a light theme and a dark one both read well.
//!
//! A hunk is read as two streams, the old side (context and deleted lines) and
//! the new side (context and added lines), each as the code it is. A hunk starts
//! mid-file, so a string or comment opened above it may be coloured wrongly;
//! that is the price of colouring without reading the whole file.

use std::ops::Range;
use std::sync::OnceLock;

use syntect::parsing::{ParseState, ScopeStack, SyntaxReference, SyntaxSet};

use crate::show::{File, Hunk, Kind};

/// The kind of a token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tok {
    Keyword,
    Str,
    Comment,
    Number,
    /// `true`, `null`, constants.
    Const,
    Type,
    Func,
}

/// The colour spans of a line: byte ranges of its text, in order and apart.
pub type Ink = Vec<(Range<usize>, Tok)>;

/// A file with more diff lines than this is left plain.
const MAX_LINES: usize = 6000;
/// A line longer than this (minified code) is left plain and not parsed.
const MAX_LINE: usize = 1500;

/// bat's collection, loaded once per process: loading it is slow enough to be
/// felt if every test or every diff did it.
fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(two_face::syntax::extra_newlines)
}

/// Fills in the colours of every line of `file` from its path's language.
/// Does nothing for a language it does not know. Runs on the reading thread.
pub fn paint(file: &mut File) {
    if file.hunks.is_empty() || file.is_binary() {
        return;
    }
    if file.hunks.iter().map(|h| h.lines.len()).sum::<usize>() > MAX_LINES {
        return;
    }
    let set = syntaxes();
    let Some(syntax) = find(set, &file.path) else { return };
    for h in &mut file.hunks {
        paint_hunk(set, syntax, h);
    }
}

fn find<'a>(set: &'a SyntaxSet, path: &str) -> Option<&'a SyntaxReference> {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    // By the whole name first (`Dockerfile`, `Makefile`), then the extension.
    let found = set.find_syntax_by_extension(name).or_else(|| name.rsplit_once('.').and_then(|(_, e)| set.find_syntax_by_extension(e)));
    found.filter(|s| s.name != "Plain Text")
}

fn paint_hunk(set: &SyntaxSet, syntax: &SyntaxReference, h: &mut Hunk) {
    // The old side gives the deleted lines their colours; the new side then
    // gives the context and the added ones theirs.
    for side in [Kind::Del, Kind::Add] {
        let mut state = ParseState::new(syntax);
        let mut stack = ScopeStack::new();
        for line in &mut h.lines {
            if line.kind != side && line.kind != Kind::Context {
                continue;
            }
            let ink = if line.text.len() > MAX_LINE { Vec::new() } else { tokens(set, &mut state, &mut stack, &line.text) };
            if line.kind == side || (side == Kind::Add && line.kind == Kind::Context) {
                line.ink = ink;
            }
        }
    }
}

/// The tokens of one line, advancing the parser.
fn tokens(set: &SyntaxSet, state: &mut ParseState, stack: &mut ScopeStack, text: &str) -> Ink {
    let mut src = String::with_capacity(text.len() + 1);
    src.push_str(text);
    src.push('\n');
    let Ok(ops) = state.parse_line(&src, set) else { return Vec::new() };
    let mut out: Ink = Vec::new();
    let end = text.len();
    let mut put = |stack: &ScopeStack, from: usize, to: usize| {
        let (from, to) = (from.min(end), to.min(end));
        if from >= to {
            return;
        }
        let Some(tok) = stack.as_slice().iter().rev().find_map(|s| classify(&s.build_string())) else { return };
        match out.last_mut() {
            Some((r, t)) if r.end == from && *t == tok => r.end = to,
            _ => out.push((from..to, tok)),
        }
    };
    let mut at = 0;
    for (pos, op) in ops {
        put(stack, at, pos);
        at = pos;
        // A malformed op only loses the colour.
        if stack.apply(&op).is_err() {
            return Vec::new();
        }
    }
    put(stack, at, end);
    out
}

/// What a scope (`keyword.control.rust`) is.
pub fn classify(scope: &str) -> Option<Tok> {
    let is = |p: &str| scope == p || scope.strip_prefix(p).is_some_and(|r| r.starts_with('.'));
    if is("comment") {
        Some(Tok::Comment)
    } else if is("string") {
        Some(Tok::Str)
    } else if is("constant.numeric") {
        Some(Tok::Number)
    } else if is("constant.language") || is("constant.character") || is("constant.other") || is("support.constant") {
        Some(Tok::Const)
    } else if is("keyword.operator") || is("keyword.other.unit") {
        None
    } else if is("keyword") || is("storage") || is("entity.name.tag") {
        Some(Tok::Keyword)
    } else if is("entity.name.function") || is("support.function") || is("variable.function") || is("meta.function-call") {
        Some(Tok::Func)
    } else if ["entity.name.type", "entity.name.class", "entity.name.struct", "entity.name.enum", "entity.name.trait"]
        .iter()
        .chain(&["entity.name.namespace", "entity.other.inherited-class", "support.type", "support.class"])
        .any(|p| is(p))
    {
        Some(Tok::Type)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::show::{Hunk, Line, Status};

    fn line(kind: Kind, text: &str) -> Line {
        Line { kind, old: 1, new: 1, text: text.into(), marks: Vec::new(), ink: Vec::new() }
    }

    fn file(path: &str, lines: Vec<Line>) -> File {
        File {
            status: Status::Modified,
            path: path.into(),
            from: None,
            blob: None,
            counts: Some((1, 1)),
            hunks: vec![Hunk { header: "@@ -1 +1 @@".into(), lines }],
            eol_only: false,
            cut: false,
            not_utf8: false,
        }
    }

    fn word(l: &Line, tok: Tok) -> Vec<&str> {
        l.ink.iter().filter(|(_, t)| *t == tok).map(|(r, _)| &l.text[r.clone()]).collect()
    }

    #[test]
    fn scopes_become_kinds() {
        assert_eq!(classify("keyword.control.rust"), Some(Tok::Keyword));
        assert_eq!(classify("keyword.operator.arithmetic.rust"), None);
        assert_eq!(classify("string.quoted.double.rust"), Some(Tok::Str));
        assert_eq!(classify("comment.line.double-slash.rust"), Some(Tok::Comment));
        assert_eq!(classify("constant.numeric.integer.decimal.rust"), Some(Tok::Number));
        assert_eq!(classify("constant.language.boolean.rust"), Some(Tok::Const));
        assert_eq!(classify("entity.name.function.rust"), Some(Tok::Func));
        assert_eq!(classify("storage.type.struct.rust"), Some(Tok::Keyword));
        assert_eq!(classify("source.rust"), None);
        // A prefix of a name is not the name.
        assert_eq!(classify("stringent"), None);
    }

    #[test]
    fn rust_lines_are_coloured_on_both_sides() {
        let mut f = file(
            "src/main.rs",
            vec![
                line(Kind::Context, "fn main() {"),
                line(Kind::Del, "    let x = 1; // old"),
                line(Kind::Add, "    let s = \"new\";"),
                line(Kind::Context, "}"),
            ],
        );
        paint(&mut f);
        let l = &f.hunks[0].lines;
        assert_eq!(word(&l[0], Tok::Keyword), ["fn"]);
        assert_eq!(word(&l[1], Tok::Keyword), ["let"]);
        assert_eq!(word(&l[1], Tok::Number), ["1"]);
        assert!(word(&l[1], Tok::Comment).iter().any(|c| c.contains("old")));
        assert_eq!(word(&l[2], Tok::Str).concat(), "\"new\"");
        // Ranges stay inside the text, in order and apart.
        for line in l {
            let mut end = 0;
            for (r, _) in &line.ink {
                assert!(r.start >= end && r.end <= line.text.len() && r.start < r.end);
                end = r.end;
            }
        }
    }

    #[test]
    fn an_unknown_language_binary_or_giant_line_stays_plain() {
        let mut f = file("notes.zzzunknown", vec![line(Kind::Add, "fn main() {}")]);
        paint(&mut f);
        assert!(f.hunks[0].lines[0].ink.is_empty());
        let mut f = file("a.rs", vec![line(Kind::Add, &"let a = 1; ".repeat(300))]);
        paint(&mut f);
        assert!(f.hunks[0].lines[0].ink.is_empty());
        let mut f = file("a.rs", vec![line(Kind::Add, "let a = 1;")]);
        f.counts = None;
        paint(&mut f);
        assert!(f.hunks[0].lines[0].ink.is_empty());
    }

    #[test]
    fn a_block_comment_runs_over_lines_and_toml_is_known() {
        let mut f = file("a.rs", vec![line(Kind::Add, "/* one"), line(Kind::Add, "   two */ let a = 1;")]);
        paint(&mut f);
        let l = &f.hunks[0].lines;
        assert_eq!(word(&l[0], Tok::Comment), ["/* one"]);
        assert_eq!(word(&l[1], Tok::Keyword), ["let"]);
        let mut f = file("Cargo.toml", vec![line(Kind::Add, "name = \"michi\"")]);
        paint(&mut f);
        assert!(!f.hunks[0].lines[0].ink.is_empty());
    }
}
