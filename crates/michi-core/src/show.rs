//! One commit in full, for the detail: its message, the files it changed
//! against its first parent with how many lines, and the diff with the
//! changed words marked. From one `git show`, read as it comes and stopped
//! once there is more than anyone would scroll through.

use std::io::{self, BufRead};
use std::ops::Range;
use std::path::Path;
use std::time::Duration;

use similar::{ChangeTag, TextDiff};

use crate::git;
use crate::oid::Oid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Detail {
    pub id: Oid,
    pub parents: Vec<Oid>,
    pub author: String,
    pub email: String,
    /// When it was written (the author's date), in seconds since 1970 UTC.
    pub time: i64,
    /// The first paragraph of the message, on one line.
    pub subject: String,
    /// The rest of the message.
    pub body: String,
    /// What changed against the first parent (against nothing for a root
    /// commit), in git's order.
    pub files: Vec<File>,
    /// The diff stops short: there was more than [`LIMITS`] keeps.
    pub cut: bool,
    /// Whether its signature holds (read apart from `git show`, which would
    /// run gpg while the diff waits).
    pub signature: crate::signature::Signature,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    /// A file became a link or the other way round.
    TypeChanged,
    /// In a merge that stopped on it: not yet resolved.
    Unmerged,
    /// Not known to git (the `2` view's files that are not yet added).
    Untracked,
    Other,
}

impl Status {
    /// From the letter in `git diff --raw` (`R100` for a rename).
    fn parse(s: &str) -> Status {
        match s.as_bytes().first() {
            Some(b'A') => Status::Added,
            Some(b'M') => Status::Modified,
            Some(b'D') => Status::Deleted,
            Some(b'R') => Status::Renamed,
            Some(b'C') => Status::Copied,
            Some(b'T') => Status::TypeChanged,
            Some(b'U') => Status::Unmerged,
            _ => Status::Other,
        }
    }

    /// The letter git gives it.
    pub fn letter(self) -> char {
        match self {
            Status::Added => 'A',
            Status::Modified => 'M',
            Status::Deleted => 'D',
            Status::Renamed => 'R',
            Status::Copied => 'C',
            Status::TypeChanged => 'T',
            Status::Unmerged => 'U',
            Status::Untracked => '?',
            Status::Other => 'X',
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    pub status: Status,
    pub path: String,
    /// Where a renamed or copied file came from.
    pub from: Option<String>,
    /// Its content after the change (the blob's id); `None` when deleted.
    pub blob: Option<Oid>,
    /// Lines added and deleted; `None` for a binary file.
    pub counts: Option<(u32, u32)>,
    pub hunks: Vec<Hunk>,
    /// Only line endings changed: CRLF against LF, or the line break at the
    /// end of the file.
    pub eol_only: bool,
    /// Its diff stops short, or was not reached.
    pub cut: bool,
    /// A line of its diff is not UTF-8 (shown with U+FFFD in its place).
    pub not_utf8: bool,
}

impl File {
    pub fn is_binary(&self) -> bool {
        self.counts.is_none()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hunk {
    /// The `@@ -a,b +c,d @@ …` line.
    pub header: String,
    pub lines: Vec<Line>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Context,
    Add,
    Del,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub kind: Kind,
    /// Its number in the old file; 0 for an added line.
    pub old: u32,
    /// Its number in the new file; 0 for a deleted line.
    pub new: u32,
    /// The text without the line break (nor a `\r` before it), tabs kept.
    pub text: String,
    /// The words changed against the paired line, as byte ranges of
    /// `text`, in order and apart.
    pub marks: Vec<Range<usize>>,
    /// Its syntax colours (kinds of token) as byte ranges of `text`, in order
    /// and apart; empty for a language it does not know.
    pub ink: crate::syntax::Ink,
}

/// A tab goes to the next column of every this many.
pub const TAB: usize = 4;

impl Line {
    /// How wide its text is: `(narrow, wide)`, the columns of its narrow
    /// characters (tabs turned into spaces, a wide character taking two
    /// columns for the tab stops) and how many wide characters (CJK) it has,
    /// which a font may draw at other than two columns.
    pub fn columns(&self) -> (usize, usize) {
        let (mut col, mut wide) = (0, 0);
        for ch in self.text.chars() {
            let w = if ch == '\t' { TAB - col % TAB } else { unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0) };
            if w == 2 && ch != '\t' {
                wide += 1;
            }
            col += w;
        }
        (col - 2 * wide, wide)
    }

    /// The text in pieces, each marked or not, tabs turned into spaces.
    pub fn pieces(&self) -> Vec<(String, bool)> {
        self.styled().into_iter().map(|(s, marked, _)| (s, marked)).collect()
    }

    /// The text in pieces, each marked or not and with its kind of token
    /// (`None` for plain text), tabs turned into spaces. A piece never
    /// straddles a mark or a token.
    pub fn styled(&self) -> Vec<(String, bool, Option<crate::syntax::Tok>)> {
        let len = self.text.len();
        let mut bounds: Vec<usize> = vec![0, len];
        for r in self.marks.iter().chain(self.ink.iter().map(|(r, _)| r)) {
            bounds.extend([r.start.min(len), r.end.min(len)]);
        }
        bounds.sort_unstable();
        bounds.dedup();
        let mut cuts = Vec::new();
        for w in bounds.windows(2) {
            let (a, b) = (w[0], w[1]);
            if !self.text.is_char_boundary(a) || !self.text.is_char_boundary(b) {
                continue;
            }
            let marked = self.marks.iter().any(|m| m.start <= a && b <= m.end);
            let tok = self.ink.iter().find(|(r, _)| r.start <= a && b <= r.end).map(|(_, t)| *t);
            cuts.push((a..b, marked, tok));
        }
        if cuts.is_empty() {
            cuts.push((0..0, false, None));
        }
        let mut col = 0;
        cuts.into_iter()
            .map(|(r, marked, tok)| {
                let mut s = String::with_capacity(r.len());
                for ch in self.text[r].chars() {
                    if ch == '\t' {
                        let n = TAB - col % TAB;
                        s.extend(std::iter::repeat_n(' ', n));
                        col += n;
                    } else {
                        s.push(ch);
                        col += 1;
                    }
                }
                (s, marked, tok)
            })
            .collect()
    }
}

/// How much of a diff is kept.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Lines in all.
    pub lines: usize,
    /// Lines of one file.
    pub file_lines: usize,
    /// Bytes of diff read in all.
    pub bytes: usize,
    /// Characters of one line; the rest becomes `…`.
    pub line_chars: usize,
}

pub const LIMITS: Limits = Limits { lines: 5000, file_lines: 1500, bytes: 8 << 20, line_chars: 2000 };

/// Lines longer than this (in bytes) get no word marks.
const MARK_BYTES: usize = 1000;
/// The most time the words of one pair of lines may take.
const MARK_TIME: Duration = Duration::from_millis(10);

const FORMAT: &str = "--format=%H%n%P%n%an%n%ae%n%at%n%s%n%b";

/// The commit `id` of the repository at `dir`, with its diff when `patch`
/// (the files alone without: what a merge gets for now). `stop` is asked
/// as the diff comes; once it says yes, the reading ends there, cut.
pub fn read(dir: &Path, id: &Oid, patch: bool, stop: &dyn Fn() -> bool) -> Result<Detail, git::Error> {
    read_with(dir, id, patch, &LIMITS, stop)
}

fn read_with(dir: &Path, id: &Oid, patch: bool, limits: &Limits, stop: &dyn Fn() -> bool) -> Result<Detail, git::Error> {
    let mut c = git::command();
    c.arg("-C").arg(dir).args(["-c", "core.quotePath=false", "show", FORMAT, "-z", "--raw", "--no-abbrev", "--numstat", "-M", "--root"]);
    c.args(["--diff-merges=first-parent", "--no-color", "--no-ext-diff", "--no-relative", "--no-show-signature"]);
    c.args(["--encoding=UTF-8", "--src-prefix=a/", "--dst-prefix=b/"]);
    crate::space::apply(&mut c, patch);
    if patch {
        c.arg("--patch");
    }
    c.arg(id.to_string());
    let mut run = git::Running::start(c, "show")?;
    let parsed = parse(&mut run.out, patch, limits, stop).map_err(|e| run.io(e))?;
    match parsed {
        Some(p) => {
            if p.ended {
                run.finish()?;
            }
            Ok(p.detail)
        }
        None => {
            run.finish()?;
            Err(git::Error::Io(format!("git show {id}: no commit came")))
        }
    }
}

/// What changed between two commits.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Changes {
    pub files: Vec<File>,
    /// The diff is cut short (see [`Detail::cut`]).
    pub cut: bool,
}

/// What changed from the commit `from` to `to` in the repository at `dir`
/// (a worktree's uncommitted changes: from its head to its WIP commit), as
/// [`read`] gives a commit's.
pub fn changes(dir: &Path, from: &Oid, to: &Oid, patch: bool, stop: &dyn Fn() -> bool) -> Result<Changes, git::Error> {
    changes_in(dir, from, to, &[], patch, stop)
}

/// [`changes`] of the files at `paths` alone (all of them when there are
/// none): a renamed file is found when both its paths are given.
pub fn changes_in(dir: &Path, from: &Oid, to: &Oid, paths: &[&str], patch: bool, stop: &dyn Fn() -> bool) -> Result<Changes, git::Error> {
    let mut c = git::command();
    c.env("GIT_LITERAL_PATHSPECS", "1");
    c.arg("-C").arg(dir).args(["-c", "core.quotePath=false", "diff", "-z", "--raw", "--no-abbrev", "--numstat", "-M"]);
    c.args(["--no-color", "--no-ext-diff", "--no-relative", "--src-prefix=a/", "--dst-prefix=b/"]);
    crate::space::apply(&mut c, patch);
    if patch {
        c.arg("--patch");
    }
    c.args([from.to_string(), to.to_string()]);
    if !paths.is_empty() {
        c.arg("--").args(paths);
    }
    let mut run = git::Running::start(c, "diff")?;
    let mut changes = Changes::default();
    let ended = files(&mut run.out, &mut changes.files, &mut changes.cut, patch, &LIMITS, stop).map_err(|e| run.io(e))?;
    if ended {
        run.finish()?;
    }
    Ok(changes)
}

/// What `git diff -z --raw --numstat [--patch]` wrote, read whole: the
/// changes, and the patch as it came (empty without one) for those that
/// need its bytes.
pub fn parse_changes(mut out: &[u8]) -> (Changes, &[u8]) {
    let mut changes = Changes::default();
    // Reading a slice fails in no way.
    let more = records(&mut out, &mut changes.files).unwrap_or(false);
    let patch = if more { out } else { &[] };
    if more {
        let _ = diff(&mut out, &mut changes.files, &mut changes.cut, &LIMITS, &|| false);
    }
    for f in &mut changes.files {
        polish(f);
    }
    (changes, patch)
}

struct Parsed {
    detail: Detail,
    /// Everything git wrote was read.
    ended: bool,
}

/// The most kept of the message.
const HEAD_BYTES: usize = 1 << 20;
/// The most kept of a path.
const FIELD_BYTES: usize = 1 << 16;

/// What `git show -z --raw --numstat [--patch]` writes: the message to a
/// NUL; a line break; the `--raw` records (`:modes hashes status`, then the
/// path, or the two paths of a rename), then the `--numstat` ones (`added
/// \t deleted \t path`, or an empty path and the two of a rename), each
/// field ending in a NUL; with the patch, an empty field and the patch.
fn parse(out: &mut impl BufRead, patch: bool, limits: &Limits, stop: &dyn Fn() -> bool) -> io::Result<Option<Parsed>> {
    let mut buf = Vec::new();
    if take(out, 0, HEAD_BYTES, &mut buf)? == 0 {
        return Ok(None);
    }
    let Some(mut detail) = header(&String::from_utf8_lossy(&buf)) else { return Ok(None) };
    let ended = files(out, &mut detail.files, &mut detail.cut, patch, limits, stop)?;
    Ok(Some(Parsed { detail, ended }))
}

/// The records of the files (`--raw`, then `--numstat`) and, when `patch`,
/// the patch after them, into `files`; `cut` when the patch is cut short.
/// Whether everything git wrote was read.
fn files(
    out: &mut impl BufRead,
    files: &mut Vec<File>,
    cut: &mut bool,
    patch: bool,
    limits: &Limits,
    stop: &dyn Fn() -> bool,
) -> io::Result<bool> {
    let more = records(out, files)?;
    let ended = if patch && more { diff(out, files, cut, limits, stop)? } else { !more || !patch };
    for f in files.iter_mut() {
        polish(f);
    }
    Ok(ended)
}

/// The records of the files, `--raw` then `--numstat`, into `files`, up to
/// the empty field before a patch. Whether there is more after them.
fn records(out: &mut impl BufRead, files: &mut Vec<File>) -> io::Result<bool> {
    let mut buf = Vec::new();
    let mut first = true;
    let mut counted = 0;
    let more = loop {
        let n = take(out, 0, FIELD_BYTES, &mut buf)?;
        let mut f: &[u8] = &buf;
        if first {
            f = f.strip_prefix(b"\n").unwrap_or(f);
            first = false;
        }
        if n == 0 || f.is_empty() {
            break n > 0;
        }
        let f = String::from_utf8_lossy(f).into_owned();
        if let Some(meta) = f.strip_prefix(':') {
            let status = Status::parse(meta.rsplit(' ').next().unwrap_or(""));
            // `old-mode new-mode old-id new-id status`.
            let blob = meta.split(' ').nth(3).and_then(Oid::parse).filter(|b| !b.is_zero());
            let one = field(out, &mut buf)?;
            let (path, from) = match status {
                Status::Renamed | Status::Copied => (field(out, &mut buf)?, Some(one)),
                _ => (one, None),
            };
            files.push(File {
                status,
                path,
                from,
                blob,
                counts: Some((0, 0)),
                hunks: Vec::new(),
                eol_only: false,
                cut: false,
                not_utf8: false,
            });
        } else {
            let mut p = f.splitn(3, '\t');
            let (added, deleted) = (p.next().unwrap_or(""), p.next().unwrap_or(""));
            let path = match p.next().unwrap_or("") {
                "" => {
                    field(out, &mut buf)?;
                    field(out, &mut buf)?
                }
                path => path.to_owned(),
            };
            let counts = match (added.parse(), deleted.parse()) {
                (Ok(a), Ok(d)) => Some((a, d)),
                _ => None,
            };
            // In the order of the raw records; a type change may come twice.
            if let Some(k) = files[counted..].iter().position(|f| f.path == path) {
                files[counted + k].counts = counts;
                counted += k + 1;
            }
        }
    };
    Ok(more)
}

/// The message: the hash, the parents, the author's name, mail and time,
/// the subject, a line each, then the body.
fn header(text: &str) -> Option<Detail> {
    let mut l = text.splitn(7, '\n');
    let id = Oid::parse(l.next()?)?;
    let parents = l.next()?.split(' ').filter_map(Oid::parse).collect();
    let author = l.next()?.to_owned();
    let email = l.next()?.to_owned();
    let time = l.next()?.trim().parse().unwrap_or(0);
    let subject = l.next().unwrap_or("").to_owned();
    let body = l.next().unwrap_or("").trim_start_matches('\n').trim_end().to_owned();
    Some(Detail { id, parents, author, email, time, subject, body, files: Vec::new(), cut: false, signature: Default::default() })
}

/// The next NUL-ended field as text.
fn field(out: &mut impl BufRead, buf: &mut Vec<u8>) -> io::Result<String> {
    take(out, 0, FIELD_BYTES, buf)?;
    Ok(String::from_utf8_lossy(buf).into_owned())
}

/// Reads to the next `end` and past it, keeping at most `cap` bytes in
/// `buf`, `end` left out: a minified file's one line costs no more than its
/// start. How many bytes went by; 0 at the end.
fn take(r: &mut impl BufRead, end: u8, cap: usize, buf: &mut Vec<u8>) -> io::Result<usize> {
    buf.clear();
    let mut read = 0;
    loop {
        let avail = match r.fill_buf() {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        if avail.is_empty() {
            return Ok(read);
        }
        let (part, used, found) = match avail.iter().position(|&b| b == end) {
            Some(i) => (&avail[..i], i + 1, true),
            None => (avail, avail.len(), false),
        };
        let room = cap.saturating_sub(buf.len());
        buf.extend_from_slice(&part[..part.len().min(room)]);
        r.consume(used);
        read += used;
        if found {
            return Ok(read);
        }
    }
}

/// The patch, a section (`diff --git …`) per file in the order of the raw
/// records; a type change comes as two sections with the same first line.
/// Whether everything git wrote was read.
fn diff(out: &mut impl BufRead, files: &mut [File], cut: &mut bool, limits: &Limits, stop: &dyn Fn() -> bool) -> io::Result<bool> {
    let mut buf = Vec::new();
    let mut at: Option<usize> = None;
    let mut section = Vec::new();
    // What is left of the hunk, old and new, and the next line numbers.
    let (mut left_old, mut left_new, mut old, mut new) = (0u32, 0u32, 0u32, 0u32);
    let (mut kept, mut file_kept, mut bytes) = (0, 0, 0);
    let ended = loop {
        if stop() {
            break false;
        }
        let n = take(out, b'\n', limits.line_chars * 4 + 8, &mut buf)?;
        if n == 0 {
            break true;
        }
        bytes += n;
        if bytes > limits.bytes {
            break false;
        }
        let line: &[u8] = buf.strip_suffix(b"\r").unwrap_or(&buf);
        if left_old > 0 || left_new > 0 {
            let kind = match line.first() {
                Some(b' ') | None => Kind::Context,
                Some(b'-') => Kind::Del,
                Some(b'+') => Kind::Add,
                Some(b'\\') => continue,
                Some(_) => {
                    left_old = 0;
                    left_new = 0;
                    continue;
                }
            };
            let (o, w) = match kind {
                Kind::Context => (old, new),
                Kind::Del => (old, 0),
                Kind::Add => (0, new),
            };
            if kind != Kind::Add {
                old += 1;
                left_old = left_old.saturating_sub(1);
            }
            if kind != Kind::Del {
                new += 1;
                left_new = left_new.saturating_sub(1);
            }
            let Some(file) = at.and_then(|k| files.get_mut(k)) else { continue };
            if file_kept >= limits.file_lines {
                file.cut = true;
                continue;
            }
            if kept >= limits.lines {
                break false;
            }
            let Some(hunk) = file.hunks.last_mut() else { continue };
            file.not_utf8 |= !is_utf8(line, n > buf.len() + 1);
            hunk.lines.push(Line {
                kind,
                old: o,
                new: w,
                text: text(line.get(1..).unwrap_or(b""), limits),
                marks: Vec::new(),
                ink: Vec::new(),
            });
            kept += 1;
            file_kept += 1;
        } else if line.starts_with(b"diff --git ") {
            if at.is_none() || line != section.as_slice() {
                at = Some(at.map_or(0, |k| k + 1));
                file_kept = 0;
            }
            section = line.to_vec();
        } else if line.starts_with(b"@@ ") {
            let header = String::from_utf8_lossy(line).into_owned();
            let Some((o, lo, w, lw)) = hunk_range(&header) else { continue };
            (old, left_old, new, left_new) = (o, lo, w, lw);
            if let Some(file) = at.and_then(|k| files.get_mut(k)) {
                if file_kept < limits.file_lines {
                    file.hunks.push(Hunk { header, lines: Vec::new() });
                }
            }
        }
        // Anything else is a section's head: modes, index, names, binary.
    };
    if !ended {
        *cut = true;
        let from = at.unwrap_or(0);
        for f in files.iter_mut().skip(from) {
            f.cut = true;
        }
    }
    Ok(ended)
}

/// `@@ -a,b +c,d @@`: where the old and the new lines start and how many
/// there are (one when the count is left out).
pub(crate) fn hunk_range(header: &str) -> Option<(u32, u32, u32, u32)> {
    let rest = header.strip_prefix("@@ -")?;
    let (old, rest) = rest.split_once(" +")?;
    let (new, _) = rest.split_once(" @@")?;
    let range = |s: &str| -> Option<(u32, u32)> {
        match s.split_once(',') {
            Some((a, n)) => Some((a.parse().ok()?, n.parse().ok()?)),
            None => Some((s.parse().ok()?, 1)),
        }
    };
    let (a, b) = range(old)?;
    let (c, d) = range(new)?;
    Some((a, b, c, d))
}

/// Whether `bytes` are UTF-8, but for a character cut short at the end
/// of a long line that was `cut`.
pub(crate) fn is_utf8(bytes: &[u8], cut: bool) -> bool {
    std::str::from_utf8(bytes).map_or_else(|e| cut && e.error_len().is_none(), |_| true)
}

/// A diff line's text, cut at [`Limits::line_chars`].
pub(crate) fn text(bytes: &[u8], limits: &Limits) -> String {
    let s = String::from_utf8_lossy(bytes);
    match s.char_indices().nth(limits.line_chars) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None => s.into_owned(),
    }
}

/// After the reading: the word marks of each deleted line and the added
/// line paired with it (the first deleted with the first added that
/// follows, and so on), and whether only line endings changed.
fn polish(file: &mut File) {
    let mut pairs = 0;
    let mut eol = true;
    for h in &mut file.hunks {
        let lines = &mut h.lines;
        let mut i = 0;
        while i < lines.len() {
            match lines[i].kind {
                Kind::Context => {
                    i += 1;
                    continue;
                }
                Kind::Add => {
                    eol = false;
                    i += 1;
                    continue;
                }
                Kind::Del => {}
            }
            let dels = i;
            while i < lines.len() && lines[i].kind == Kind::Del {
                i += 1;
            }
            let adds = i;
            while i < lines.len() && lines[i].kind == Kind::Add {
                i += 1;
            }
            let (d, a) = (adds - dels, i - adds);
            if d != a {
                eol = false;
            }
            let (left, right) = lines.split_at_mut(adds);
            for k in 0..d.min(a) {
                let (o, n) = (&mut left[dels + k], &mut right[k]);
                if o.text == n.text {
                    pairs += 1;
                } else {
                    eol = false;
                    mark(o, n);
                }
            }
        }
    }
    file.eol_only = eol && pairs > 0 && !file.cut;
    crate::syntax::paint(file);
}

/// Marks the words that differ between `old` and `new`, unless the two have
/// too little in common (under 30% of the shorter's words) for marks to
/// help. Spaces are never marked.
fn mark(old: &mut Line, new: &mut Line) {
    if old.text.len() > MARK_BYTES || new.text.len() > MARK_BYTES {
        return;
    }
    let diff = TextDiff::configure().timeout(MARK_TIME).diff_unicode_words(old.text.as_str(), new.text.as_str());
    let (mut o, mut n) = (0, 0);
    let (mut om, mut nm) = (Vec::new(), Vec::new());
    let (mut same, mut old_words, mut new_words) = (0, 0, 0);
    for c in diff.iter_all_changes() {
        let v = c.value();
        let word = !v.trim().is_empty();
        match c.tag() {
            ChangeTag::Equal => {
                o += v.len();
                n += v.len();
                if word {
                    same += 1;
                    old_words += 1;
                    new_words += 1;
                }
            }
            ChangeTag::Delete => {
                if word {
                    join(&mut om, o..o + v.len());
                    old_words += 1;
                }
                o += v.len();
            }
            ChangeTag::Insert => {
                if word {
                    join(&mut nm, n..n + v.len());
                    new_words += 1;
                }
                n += v.len();
            }
        }
    }
    if same == 0 || same * 10 < old_words.min(new_words) * 3 {
        return;
    }
    old.marks = om;
    new.marks = nm;
}

/// Adds `r` to `marks`, run into the last one when they touch.
fn join(marks: &mut Vec<Range<usize>>, r: Range<usize>) {
    if let Some(last) = marks.last_mut() {
        if last.end == r.start {
            last.end = r.end;
            return;
        }
    }
    marks.push(r);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;
    use std::fs;

    fn never() -> bool {
        false
    }

    fn head(r: &TempRepo) -> Oid {
        Oid::parse(&git::read_text(&r.repo, &["rev-parse", "HEAD"]).unwrap()).unwrap()
    }

    fn line(kind: Kind, text: &str) -> Line {
        Line { kind, old: 0, new: 0, text: text.into(), marks: Vec::new(), ink: Vec::new() }
    }

    fn spans(l: &Line) -> Vec<(usize, usize)> {
        l.marks.iter().map(|m| (m.start, m.end)).collect()
    }

    fn kinds(h: &Hunk) -> String {
        h.lines
            .iter()
            .map(|l| match l.kind {
                Kind::Context => ' ',
                Kind::Add => '+',
                Kind::Del => '-',
            })
            .collect()
    }

    #[test]
    fn a_commit_s_files_and_diff_are_read() {
        let r = TempRepo::new("show-files");
        fs::create_dir_all(r.repo.join("d")).unwrap();
        fs::write(r.repo.join("a.txt"), "one\nthe quick brown fox\nthree\n").unwrap();
        fs::write(r.repo.join("b.bin"), b"\x00\x01\x02binary").unwrap();
        fs::write(r.repo.join("d/sp ace.txt"), "a line that stays the same\n".repeat(8)).unwrap();
        fs::write(r.repo.join("名前.txt"), "古い\n").unwrap();
        r.git(&r.repo, &["add", "-A"]);
        r.git(&r.repo, &["commit", "-q", "-m", "add files"]);
        fs::write(r.repo.join("a.txt"), "one\nthe quick red fox\nthree\nfour\n").unwrap();
        fs::remove_file(r.repo.join("b.bin")).unwrap();
        fs::rename(r.repo.join("d/sp ace.txt"), r.repo.join("d/moved.txt")).unwrap();
        fs::write(r.repo.join("名前.txt"), "新しい\n").unwrap();
        r.git(&r.repo, &["add", "-A"]);
        r.git(&r.repo, &["commit", "-q", "-m", "change files", "-m", "Why:\n\nbecause."]);
        let id = head(&r);
        let d = read(&r.repo, &id, true, &never).unwrap();
        assert_eq!(d.id, id);
        assert_eq!(d.parents.len(), 1);
        assert_eq!((d.author.as_str(), d.email.as_str()), ("michi", "michi@example.invalid"));
        assert!(d.time > crate::testrepo::START);
        assert_eq!(d.subject, "change files");
        assert_eq!(d.body, "Why:\n\nbecause.");
        assert!(!d.cut);
        let files: Vec<_> = d.files.iter().map(|f| (f.status.letter(), f.path.as_str(), f.from.as_deref(), f.counts)).collect();
        assert_eq!(
            files,
            [
                ('M', "a.txt", None, Some((2, 1))),
                ('D', "b.bin", None, None),
                ('R', "d/moved.txt", Some("d/sp ace.txt"), Some((0, 0))),
                ('M', "名前.txt", None, Some((1, 1))),
            ]
        );
        let a = &d.files[0];
        assert_eq!(a.hunks.len(), 1);
        assert_eq!(a.hunks[0].header, "@@ -1,3 +1,4 @@");
        assert_eq!(kinds(&a.hunks[0]), " -+ +");
        let nums: Vec<_> = a.hunks[0].lines.iter().map(|l| (l.old, l.new)).collect();
        assert_eq!(nums, [(1, 1), (2, 0), (0, 2), (3, 3), (0, 4)]);
        assert_eq!(a.hunks[0].lines[1].text, "the quick brown fox");
        assert_eq!(spans(&a.hunks[0].lines[1]), [(10, 15)]);
        assert_eq!(spans(&a.hunks[0].lines[2]), [(10, 13)]);
        assert!(a.hunks[0].lines[4].marks.is_empty());
        assert!(d.files[1].is_binary() && d.files[1].hunks.is_empty());
        assert!(d.files[2].hunks.is_empty());
        assert_eq!(d.files[3].hunks[0].lines[1].text, "新しい");
        assert!(d.files.iter().all(|f| !f.cut && !f.eol_only));
    }

    #[test]
    fn the_changes_between_two_commits_are_read() {
        let r = TempRepo::new("show-changes");
        let from = head(&r);
        fs::write(r.repo.join("n.txt"), "one\ntwo\n").unwrap();
        r.git(&r.repo, &["add", "n.txt"]);
        r.git(&r.repo, &["commit", "-q", "-m", "n"]);
        let to = head(&r);
        let c = changes(&r.repo, &from, &to, true, &never).unwrap();
        assert!(!c.cut);
        assert_eq!(c.files.len(), 1);
        let f = &c.files[0];
        assert_eq!((f.status, f.path.as_str(), f.counts), (Status::Added, "n.txt", Some((2, 0))));
        assert_eq!(kinds(&f.hunks[0]), "++");
        let c = changes(&r.repo, &from, &to, false, &never).unwrap();
        assert!(c.files[0].hunks.is_empty() && c.files[0].counts == Some((2, 0)));
        assert!(changes(&r.repo, &to, &to, true, &never).unwrap().files.is_empty());
    }

    #[test]
    fn one_file_s_changes_are_read_with_its_blob() {
        let r = TempRepo::new("show-one-file");
        r.write(&r.repo, "a.txt", &"a line that stays the same\n".repeat(8));
        r.write(&r.repo, "b.txt", "b\n");
        r.git(&r.repo, &["add", "-A"]);
        r.git(&r.repo, &["commit", "-q", "-m", "files"]);
        let from = head(&r);
        fs::rename(r.repo.join("a.txt"), r.repo.join("moved.txt")).unwrap();
        r.write(&r.repo, "b.txt", "b\nmore\n");
        r.write(&r.repo, "[c].txt", "c\n");
        r.git(&r.repo, &["add", "-A"]);
        r.git(&r.repo, &["commit", "-q", "-m", "change"]);
        let to = head(&r);
        let blob = |path: &str| Oid::parse(&r.out(&r.repo, &["rev-parse", &format!("HEAD:{path}")]));
        let all = changes(&r.repo, &from, &to, false, &never).unwrap().files;
        let got: Vec<_> = all.iter().map(|f| (f.path.as_str(), f.blob)).collect();
        assert_eq!(got, [("[c].txt", blob("[c].txt")), ("b.txt", blob("b.txt")), ("moved.txt", blob("moved.txt"))]);
        let one = changes_in(&r.repo, &from, &to, &["moved.txt", "a.txt"], true, &never).unwrap().files;
        assert_eq!(one.len(), 1);
        assert_eq!((one[0].status, one[0].from.as_deref()), (Status::Renamed, Some("a.txt")));
        // Taken as it is, not as a pattern.
        let one = changes_in(&r.repo, &from, &to, &["[c].txt"], true, &never).unwrap().files;
        assert_eq!(one.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), ["[c].txt"]);
        assert_eq!(kinds(&one[0].hunks[0]), "+");
        r.git(&r.repo, &["rm", "-q", "b.txt"]);
        r.git(&r.repo, &["commit", "-q", "-m", "gone"]);
        let gone = changes(&r.repo, &to, &head(&r), false, &never).unwrap().files;
        assert_eq!((gone[0].status, gone[0].blob), (Status::Deleted, None));
    }

    #[test]
    fn a_merge_lists_its_files_against_the_first_parent() {
        let r = TempRepo::new("show-merge");
        r.git(&r.repo, &["switch", "-q", "-c", "feat"]);
        fs::write(r.repo.join("f.txt"), "f\n").unwrap();
        r.git(&r.repo, &["add", "f.txt"]);
        r.git(&r.repo, &["commit", "-q", "-m", "f"]);
        r.git(&r.repo, &["switch", "-q", "main"]);
        fs::write(r.repo.join("m.txt"), "m\n").unwrap();
        r.git(&r.repo, &["add", "m.txt"]);
        r.git(&r.repo, &["commit", "-q", "-m", "m"]);
        r.git(&r.repo, &["merge", "-q", "--no-ff", "-m", "Merge feat", "feat"]);
        let d = read(&r.repo, &head(&r), false, &never).unwrap();
        assert_eq!(d.parents.len(), 2);
        assert_eq!(d.files.len(), 1);
        assert_eq!((d.files[0].status, d.files[0].path.as_str(), d.files[0].counts), (Status::Added, "f.txt", Some((1, 0))));
        assert!(d.files[0].hunks.is_empty() && !d.cut);
    }

    #[test]
    fn a_root_commit_is_against_nothing_and_an_empty_one_has_no_files() {
        let r = TempRepo::new("show-root");
        let empty = read(&r.repo, &head(&r), true, &never).unwrap();
        assert!(empty.parents.is_empty() && empty.files.is_empty() && !empty.cut);
        assert_eq!((empty.subject.as_str(), empty.body.as_str()), ("first", ""));
        r.git(&r.repo, &["switch", "-q", "--orphan", "other"]);
        fs::write(r.repo.join("x.txt"), "x\n").unwrap();
        r.git(&r.repo, &["add", "x.txt"]);
        r.git(&r.repo, &["commit", "-q", "-m", "root"]);
        let root = read(&r.repo, &head(&r), true, &never).unwrap();
        assert!(root.parents.is_empty());
        assert_eq!(root.files.len(), 1);
        assert_eq!(root.files[0].status, Status::Added);
        assert_eq!(kinds(&root.files[0].hunks[0]), "+");
    }

    #[test]
    fn line_endings_alone_are_told_apart() {
        let r = TempRepo::new("show-eol");
        fs::write(r.repo.join("crlf.txt"), "a\nb\n").unwrap();
        fs::write(r.repo.join("real.txt"), "a\nb\n").unwrap();
        fs::write(r.repo.join("end.txt"), "a\nb").unwrap();
        r.git(&r.repo, &["add", "-A"]);
        r.git(&r.repo, &["commit", "-q", "-m", "lf"]);
        fs::write(r.repo.join("crlf.txt"), "a\r\nb\r\n").unwrap();
        fs::write(r.repo.join("real.txt"), "a\r\nB\r\n").unwrap();
        fs::write(r.repo.join("end.txt"), "a\nb\n").unwrap();
        r.git(&r.repo, &["add", "-A"]);
        r.git(&r.repo, &["commit", "-q", "-m", "crlf"]);
        let d = read(&r.repo, &head(&r), true, &never).unwrap();
        let eol: Vec<_> = d.files.iter().map(|f| (f.path.as_str(), f.eol_only)).collect();
        assert_eq!(eol, [("crlf.txt", true), ("end.txt", true), ("real.txt", false)]);
        assert_eq!(d.files[0].hunks[0].lines[0].text, "a");
    }

    #[test]
    fn a_long_diff_is_cut() {
        let r = TempRepo::new("show-cut");
        for name in ["1.txt", "2.txt", "3.txt"] {
            fs::write(r.repo.join(name), "x\n".repeat(4)).unwrap();
        }
        r.git(&r.repo, &["add", "-A"]);
        r.git(&r.repo, &["commit", "-q", "-m", "three"]);
        let limits = Limits { lines: 5, file_lines: 3, bytes: 1 << 20, line_chars: 100 };
        let d = read_with(&r.repo, &head(&r), true, &limits, &never).unwrap();
        assert!(d.cut);
        let got: Vec<_> = d.files.iter().map(|f| (f.hunks.iter().map(|h| h.lines.len()).sum::<usize>(), f.cut)).collect();
        assert_eq!(got, [(3, true), (2, true), (0, true)]);
        // Asked to stop, it stops at once.
        let d = read_with(&r.repo, &head(&r), true, &LIMITS, &|| true).unwrap();
        assert!(d.cut && d.files.iter().all(|f| f.hunks.is_empty() && f.cut));
        assert_eq!(d.files.len(), 3);
    }

    #[test]
    fn a_missing_commit_is_git_s_refusal() {
        let r = TempRepo::new("show-missing");
        let missing = Oid::parse(&"1".repeat(40)).unwrap();
        assert!(matches!(read(&r.repo, &missing, true, &never), Err(git::Error::Git(_))));
    }

    #[test]
    fn a_type_change_s_two_sections_are_one_file() {
        let a = "a".repeat(40);
        let out = format!(
            "{a}\n\nx\nx@y\n0\nlink to file\n\0\n:120000 100644 1111111 2222222 T\0l\0\
             1\t1\tl\0:100644 100644 3333333 4444444 M\0m\0\
             1\t0\tm\0\0\
             diff --git a/l b/l\ndeleted file mode 120000\n--- a/l\n+++ /dev/null\n@@ -1 +0,0 @@\n-target\n\\ No newline at end of file\n\
             diff --git a/l b/l\nnew file mode 100644\n--- /dev/null\n+++ b/l\n@@ -0,0 +1 @@\n+text\n\
             diff --git a/m b/m\n--- a/m\n+++ b/m\n@@ -1 +1,2 @@\n m\n+-- not a header\n"
        );
        let p = parse(&mut out.as_bytes(), true, &LIMITS, &never).unwrap().unwrap();
        assert!(p.ended);
        let f = &p.detail.files;
        assert_eq!(f.len(), 2);
        assert_eq!((f[0].status, f[0].hunks.len()), (Status::TypeChanged, 2));
        assert_eq!(f[1].hunks[0].lines[1].text, "-- not a header");
        assert!(!f[0].eol_only);
    }

    #[test]
    fn words_are_marked_only_when_the_lines_have_enough_in_common() {
        let (mut o, mut n) = (line(Kind::Del, "let total = sum(a, b);"), line(Kind::Add, "let total = sum(a, c);"));
        mark(&mut o, &mut n);
        assert_eq!((spans(&o), spans(&n)), (vec![(19, 20)], vec![(19, 20)]));
        let (mut o, mut n) = (line(Kind::Del, "alpha beta gamma delta"), line(Kind::Add, "one two three delta"));
        mark(&mut o, &mut n);
        assert!(o.marks.is_empty() && n.marks.is_empty());
        let (mut o, mut n) = (line(Kind::Del, "日本語の文章です"), line(Kind::Add, "日本語の説明です"));
        mark(&mut o, &mut n);
        assert!(!o.marks.is_empty());
        assert!(o.marks.iter().all(|m| o.text.is_char_boundary(m.start) && o.text.is_char_boundary(m.end)));
        let (mut o, mut n) = (line(Kind::Del, "a  b"), line(Kind::Add, "a b"));
        mark(&mut o, &mut n);
        assert!(o.marks.is_empty() && n.marks.is_empty());
    }

    #[test]
    fn tabs_become_spaces_to_the_next_column() {
        let mut l = line(Kind::Context, "a\tbc\td");
        l.marks.push(2..4);
        assert_eq!(l.pieces(), [("a   ".to_owned(), false), ("bc".to_owned(), true), ("  d".to_owned(), false)]);
        assert_eq!(line(Kind::Context, "").pieces(), [(String::new(), false)]);
        // Colours and marks cut the same text apart.
        use crate::syntax::Tok;
        let mut l = line(Kind::Add, "let	x");
        l.marks = std::iter::once(2..5).collect();
        l.ink = std::iter::once((0..3, Tok::Keyword)).collect();
        assert_eq!(
            l.styled(),
            [("le".to_owned(), false, Some(Tok::Keyword)), ("t".to_owned(), true, Some(Tok::Keyword)), (" x".to_owned(), true, None)]
        );
    }

    #[test]
    fn columns_count_tabs_and_wide_characters() {
        assert_eq!(line(Kind::Add, "").columns(), (0, 0));
        assert_eq!(line(Kind::Add, "ab\tc").columns(), (5, 0));
        assert_eq!(line(Kind::Add, "日本\tx").columns(), (5, 2));
        assert_eq!(line(Kind::Add, "café").columns(), (4, 0));
    }

    #[test]
    fn hunk_ranges_are_read() {
        assert_eq!(hunk_range("@@ -1,3 +1,4 @@ fn x"), Some((1, 3, 1, 4)));
        assert_eq!(hunk_range("@@ -1 +0,0 @@"), Some((1, 1, 0, 0)));
        assert_eq!(hunk_range("@@@ -1 +1 @@@"), None);
    }

    #[test]
    fn a_file_that_is_not_utf8_is_told() {
        let r = TempRepo::new("show-latin1");
        fs::write(r.repo.join("latin1.txt"), b"caf\xe9\n").unwrap();
        fs::write(r.repo.join("utf8.txt"), "café\n").unwrap();
        r.git(&r.repo, &["add", "-A"]);
        r.git(&r.repo, &["commit", "-q", "-m", "two encodings"]);
        let d = read(&r.repo, &head(&r), true, &never).unwrap();
        let told: Vec<_> = d.files.iter().map(|f| (f.path.as_str(), f.not_utf8)).collect();
        assert_eq!(told, [("latin1.txt", true), ("utf8.txt", false)]);
        assert_eq!(d.files[0].hunks[0].lines[0].text, "caf\u{fffd}");
        // A character cut short where a long line was cut is not counted.
        assert!(is_utf8(&"é".as_bytes()[..1], true));
        assert!(!is_utf8(&"é".as_bytes()[..1], false));
        assert!(!is_utf8(b"\xe9 ", true));
    }

    #[test]
    fn long_lines_are_cut() {
        let limits = Limits { line_chars: 3, ..LIMITS };
        assert_eq!(text("abcdef".as_bytes(), &limits), "abc…");
        assert_eq!(text("あい".as_bytes(), &limits), "あい");
    }
}
