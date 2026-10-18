//! A worktree stopped in the middle of a merge, a cherry-pick, a revert or a
//! rebase with files that clashed (docs/design.md: 衝突の解消): the places
//! in each file with the lines of both sides and of the base, what went in
//! without a clash, and the writes that solve a place, take a file whole,
//! start a file over, call it all off or commit it.
//!
//! The places are read from the markers in the worktree's file, so a place
//! solved by hand (an editor, a Claude) counts as solved. The places as git
//! left them come from `git merge-file` of the file's three stages (kept by
//! `ls-files --resolve-undo` once the file is added): they give the base
//! when the file's markers have none, and the places solved since.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::backup;
use crate::branch::{self, Doing};
use crate::discard::{self, TempIndex};
use crate::git;
use crate::oid::Oid;
use crate::stage;

/// The unchanged lines shown before and after a place.
pub const AROUND: usize = 3;

/// A worktree in the middle of something that can clash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    pub wt: PathBuf,
    pub doing: Doing,
    /// The side the worktree is on: its branch (no name when detached, as in
    /// a rebase) and its head.
    pub ours: Side,
    /// The side coming in: the branch merged, the commit picked; for a revert
    /// the reverted commit's parent, named after the commit.
    pub theirs: Side,
    /// Where the two sides parted.
    pub base: Option<Oid>,
    /// When it stopped: the time of git's file for it.
    pub started: Option<SystemTime>,
    /// The message git made for the commit, without its comments; empty in a
    /// rebase, which keeps each commit's own.
    pub message: String,
    /// The files that clashed, solved or not, by path.
    pub files: Vec<File>,
    /// The files that came in without a clash.
    pub auto: Vec<Auto>,
}

impl Conflict {
    /// The places in all the files.
    pub fn total(&self) -> usize {
        self.files.iter().map(File::total).sum()
    }

    /// The places not solved yet.
    pub fn left(&self) -> usize {
        self.files.iter().map(File::left).sum()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Side {
    pub name: String,
    pub id: Option<Oid>,
}

/// A file that came in without a clash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Auto {
    pub path: String,
    /// Changed on both sides (in other places, or the same way).
    pub both: bool,
}

/// A file that clashed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    pub path: String,
    /// Still unmerged in the index (not added since).
    pub unmerged: bool,
    pub body: Body,
}

impl File {
    pub fn total(&self) -> usize {
        match &self.body {
            Body::Text(blocks) => blocks.len(),
            Body::Whole(_) => 1,
        }
    }

    pub fn left(&self) -> usize {
        match &self.body {
            Body::Text(blocks) => blocks.iter().filter(|b| b.is_open()).count(),
            Body::Whole(_) => usize::from(self.unmerged),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Body {
    /// Merged line by line: its places, in order.
    Text(Vec<Block>),
    /// Not merged line by line: one side's file is taken whole.
    Whole(Whole),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Whole {
    pub why: Why,
    /// What our side did to the file since the base.
    pub did_ours: Fact,
    /// What their side did to it.
    pub did_theirs: Fact,
    /// Whether our side has the file (no: deleted there).
    pub ours: bool,
    /// Whether their side has it.
    pub theirs: bool,
}

/// Why a file is not merged line by line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// One side deleted it and the other changed it.
    Deleted,
    /// Binary, or marked `-merge` / `merge=binary`.
    Binary,
    /// A link, a submodule, or a clash git saw that is not in the lines.
    Other,
    /// The name is left over from a rename that clashed (both sides moved
    /// the file, or only one side has this name and it had no base).
    Moved,
}

/// What one side did to a file since the base, from the index alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fact {
    /// The side no longer has the file.
    Deleted,
    /// The side added the file (the base had none).
    Added,
    /// The side changed the content.
    Changed,
    /// The side changed only the file's mode (the executable bit).
    Mode { exec: bool },
    /// The side has a link.
    Link,
    /// The side has a submodule.
    Submodule,
    /// The side did not touch it.
    Same,
}

/// What `side` did to the file that the base had as `base` (mode and blob;
/// `None` when there is none).
pub fn fact(base: Option<&(String, Oid)>, side: Option<&(String, Oid)>) -> Fact {
    let Some((mode, id)) = side else { return Fact::Deleted };
    match mode.as_str() {
        "120000" => return Fact::Link,
        "160000" => return Fact::Submodule,
        _ => {}
    }
    let Some((bmode, bid)) = base else { return Fact::Added };
    match (bid == id, bmode == mode) {
        (true, true) => Fact::Same,
        (true, false) => Fact::Mode { exec: mode == "100755" },
        _ => Fact::Changed,
    }
}

/// A place where the sides clashed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// The line (from 1) of its `<<<<<<<` in the worktree's file; `None`
    /// once solved.
    pub line: Option<usize>,
    pub ours: Vec<String>,
    /// `None` when neither the markers nor the stages tell.
    pub base: Option<Vec<String>>,
    pub theirs: Vec<String>,
    /// What the worktree's file has in its place once solved.
    pub result: Option<Vec<String>>,
    /// Up to [`AROUND`] unchanged lines before it, and how many more lines
    /// lie between those and the place before it (or the top) unshown.
    pub before: Vec<String>,
    pub skipped: usize,
    /// Up to [`AROUND`] unchanged lines after it; for the last place, how
    /// many more lie before the end.
    pub after: Vec<String>,
    pub skipped_after: usize,
}

/// What a place is solved with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pick {
    Ours,
    Theirs,
    /// Both, ours first.
    Both,
    /// Both, theirs first.
    BothTheirsFirst,
    Base,
}

/// What a place looks like, for the line that helps choose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hint {
    /// Nothing there before; each side added lines.
    BothAdded,
    /// Our side took the lines out; theirs changed them.
    OursDeleted,
    /// Their side took the lines out; ours changed them.
    TheirsDeleted,
    /// Both changed the same lines: how many (0 when the base is unknown).
    BothChanged(usize),
}

impl Block {
    pub fn is_open(&self) -> bool {
        self.line.is_some()
    }

    /// The lines `pick` leaves in the place; `None` for the base when it is
    /// not known.
    pub fn lines(&self, pick: Pick) -> Option<Vec<String>> {
        Some(match pick {
            Pick::Ours => self.ours.clone(),
            Pick::Theirs => self.theirs.clone(),
            Pick::Both => [&self.ours[..], &self.theirs[..]].concat(),
            Pick::BothTheirsFirst => [&self.theirs[..], &self.ours[..]].concat(),
            Pick::Base => self.base.clone()?,
        })
    }

    pub fn hint(&self) -> Hint {
        if self.ours.is_empty() {
            Hint::OursDeleted
        } else if self.theirs.is_empty() {
            Hint::TheirsDeleted
        } else if self.base.as_ref().is_some_and(Vec::is_empty) {
            Hint::BothAdded
        } else {
            Hint::BothChanged(self.base.as_ref().map_or(0, Vec::len))
        }
    }
}

/// How a merge or a cherry-pick ended, when git did not refuse it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The new head.
    Made(Oid),
    /// Nothing to take: the head has it all.
    UpToDate,
    /// It clashed in these files, and stopped to be solved.
    Conflict(Vec<String>),
    /// The pick's changes are in already: nothing done.
    Empty,
}

/// What the worktree at `wt` is in the middle of, its places and files;
/// `None` when it is in the middle of nothing that clashes.
pub fn read(wt: &Path) -> Result<Option<Conflict>, git::Error> {
    let doing = match branch::doing(wt) {
        None | Some(Doing::Bisect) => return Ok(None),
        Some(d) => d,
    };
    let dir = branch::git_dir(wt).ok_or(git::Error::NotRepo)?;
    let state = |name: &str| fs::read_to_string(dir.join(name)).ok();
    let id_in = |name: &str| state(name).and_then(|s| s.lines().next().and_then(|l| Oid::parse(l.trim())));
    let short = |id: Option<Oid>| id.map(|i| i.short()).unwrap_or_default();
    let head = rev(wt, "HEAD");
    let (theirs, base, stamp, name) = match doing {
        Doing::Merge => {
            let m = id_in("MERGE_HEAD");
            let base = m.and_then(|m| {
                let out = git::read_maybe(wt, &["merge-base", "HEAD", &m.to_string()]).ok()??;
                Oid::parse(String::from_utf8_lossy(&out).trim())
            });
            let name = state("MERGE_MSG").and_then(|s| merged_name(&s)).unwrap_or_else(|| short(m));
            (m, base, "MERGE_HEAD", name)
        }
        Doing::CherryPick => {
            let p = id_in("CHERRY_PICK_HEAD");
            (p, p.and_then(|p| rev(wt, &format!("{p}^"))), "CHERRY_PICK_HEAD", short(p))
        }
        Doing::Revert => {
            let r = id_in("REVERT_HEAD");
            (r.and_then(|r| rev(wt, &format!("{r}^"))), r, "REVERT_HEAD", short(r))
        }
        Doing::Rebase | Doing::Bisect => {
            let r = id_in("REBASE_HEAD");
            let stamp = if dir.join("rebase-merge").exists() { "rebase-merge" } else { "rebase-apply" };
            let name = state(&format!("{stamp}/head-name"))
                .map(|s| s.trim().trim_start_matches("refs/heads/").to_owned())
                .filter(|s| !s.is_empty() && s != "detached HEAD")
                .unwrap_or_else(|| short(r));
            (r, r.and_then(|r| rev(wt, &format!("{r}^"))), stamp, name)
        }
    };
    let ours_name = state("HEAD").and_then(|s| s.trim().strip_prefix("ref: refs/heads/").map(str::to_owned)).unwrap_or_default();
    let message = if doing == Doing::Rebase { String::new() } else { state("MERGE_MSG").map(|s| prepared(&s)).unwrap_or_default() };
    let started = fs::metadata(dir.join(stamp)).and_then(|m| m.modified()).ok();

    let files = files(wt)?;
    let clashed: HashSet<&str> = files.iter().map(|f| f.path.as_str()).collect();
    let auto = match (base, theirs) {
        (Some(b), Some(t)) => {
            let (came, ours_too) = git::both(|| names(wt, &b, &t.to_string()), || names(wt, &b, "HEAD"));
            let ours_too: HashSet<String> = ours_too?.into_iter().collect();
            came?.into_iter().filter(|p| !clashed.contains(p.as_str())).map(|p| Auto { both: ours_too.contains(&p), path: p }).collect()
        }
        _ => Vec::new(),
    };
    Ok(Some(Conflict {
        wt: wt.to_owned(),
        doing,
        ours: Side { name: ours_name, id: head },
        theirs: Side { name, id: theirs },
        base,
        started,
        message,
        files,
        auto,
    }))
}

/// The commit `spec` names, if any.
fn rev(wt: &Path, spec: &str) -> Option<Oid> {
    let out = git::read_maybe(wt, &["rev-parse", "-q", "--verify", &format!("{spec}^{{commit}}")]).ok()??;
    Oid::parse(String::from_utf8_lossy(&out).trim())
}

/// The paths that differ between `a` and `b`.
fn names(wt: &Path, a: &Oid, b: &str) -> Result<Vec<String>, git::Error> {
    let out = git::read(wt, &["diff", "--name-only", "-z", "--no-renames", &a.to_string(), b])?;
    Ok(out.split(|&b| b == 0).filter(|p| !p.is_empty()).map(|p| String::from_utf8_lossy(p).into_owned()).collect())
}

/// The branch a merge's message names: `Merge branch 'feat' into main`,
/// `Merge remote-tracking branch 'origin/x'`, `Merge tag 'v1'`.
fn merged_name(message: &str) -> Option<String> {
    let rest = message.lines().next()?.strip_prefix("Merge ")?;
    let rest = ["branch ", "remote-tracking branch ", "commit ", "tag "].iter().find_map(|k| rest.strip_prefix(k)).unwrap_or(rest);
    let rest = rest.strip_prefix('\'')?;
    Some(rest[..rest.find('\'')?].to_owned())
}

/// A message git made, without the comments it put at its end
/// (`# Conflicts:` and the files).
fn prepared(message: &str) -> String {
    let mut lines: Vec<&str> = message.lines().collect();
    while lines.last().is_some_and(|l| l.starts_with('#') || l.trim().is_empty()) {
        lines.pop();
    }
    lines.join("\n").trim().to_owned()
}

/// A path's stages 1 to 3 (base, ours, theirs): mode and blob.
type Stages = [Option<(String, Oid)>; 3];

/// `ls-files -u -z` or `--resolve-undo -z`, by path in their order.
fn stages(out: &[u8]) -> Vec<(String, Stages)> {
    let mut list: Vec<(String, Stages)> = Vec::new();
    for rec in out.split(|&b| b == 0).filter(|r| !r.is_empty()) {
        let rec = String::from_utf8_lossy(rec);
        let Some((meta, path)) = rec.split_once('\t') else { continue };
        let mut meta = meta.split(' ');
        let (Some(mode), Some(id), Some(n)) = (meta.next(), meta.next(), meta.next()) else { continue };
        let (Some(id), Ok(n)) = (Oid::parse(id), n.parse::<usize>()) else { continue };
        if !(1..=3).contains(&n) || id.is_zero() || mode.trim_start_matches('0').is_empty() {
            continue;
        }
        if list.last().is_none_or(|(p, _)| p != path) {
            list.push((path.to_owned(), Default::default()));
        }
        if let Some((_, s)) = list.last_mut() {
            s[n - 1] = Some((mode.to_owned(), id));
        }
    }
    list
}

/// The paths still unmerged in the worktree at `wt`.
pub fn unmerged(wt: &Path) -> Result<Vec<String>, git::Error> {
    Ok(stages(&git::read(wt, &["ls-files", "-u", "-z"])?).into_iter().map(|(p, _)| p).collect())
}

/// The files that clashed in the worktree at `wt`: those still unmerged and
/// those added since.
fn files(wt: &Path) -> Result<Vec<File>, git::Error> {
    let (open, done) = git::both(|| git::read(wt, &["ls-files", "-u", "-z"]), || git::read(wt, &["ls-files", "--resolve-undo", "-z"]));
    let open = stages(&open?);
    let seen: HashSet<String> = open.iter().map(|(p, _)| p.clone()).collect();
    // git older than 2.40 has no --resolve-undo: the solved files are not
    // shown then.
    let done: Vec<(String, Stages)> = done.map(|o| stages(&o)).unwrap_or_default().into_iter().filter(|(p, _)| !seen.contains(p)).collect();
    let mut all: Vec<(String, Stages, bool)> =
        open.into_iter().map(|(p, s)| (p, s, true)).chain(done.into_iter().map(|(p, s)| (p, s, false))).collect();
    all.sort_by(|a, b| a.0.cmp(&b.0));

    let paths: Vec<&str> = all.iter().map(|(p, _, _)| p.as_str()).collect();
    let binary = whole_by_attr(wt, &paths);
    let mut wanted: Vec<Oid> = Vec::new();
    for (path, s, _) in &all {
        if !binary.contains(path) && s[1].is_some() && s[2].is_some() {
            wanted.extend(s.iter().flatten().filter(|(m, _)| is_file(m)).map(|(_, id)| *id));
        }
    }
    wanted.sort();
    wanted.dedup();
    let blobs = blobs(wt, &wanted)?;

    let mut files = Vec::with_capacity(all.len());
    for (path, s, unmerged) in all {
        let body = body(wt, &path, &s, binary.contains(&path), &blobs, unmerged);
        files.push(File { path, unmerged, body });
    }
    Ok(files)
}

/// A regular file's mode (not a link's or a submodule's).
fn is_file(mode: &str) -> bool {
    mode == "100644" || mode == "100755"
}

fn body(wt: &Path, path: &str, s: &Stages, binary: bool, blobs: &HashMap<Oid, Vec<u8>>, unmerged: bool) -> Body {
    let whole = |why| {
        Body::Whole(Whole {
            why,
            did_ours: fact(s[0].as_ref(), s[1].as_ref()),
            did_theirs: fact(s[0].as_ref(), s[2].as_ref()),
            ours: s[1].is_some(),
            theirs: s[2].is_some(),
        })
    };
    let (Some(ours), Some(theirs)) = (&s[1], &s[2]) else {
        // A name with no base, or that neither side has, is a rename's.
        return whole(if s[0].is_none() || (s[1].is_none() && s[2].is_none()) { Why::Moved } else { Why::Deleted });
    };
    if s.iter().flatten().any(|(m, _)| !is_file(m)) {
        return whole(Why::Other);
    }
    if binary {
        return whole(Why::Binary);
    }
    let blob = |id: &Oid| blobs.get(id).map(Vec::as_slice);
    let (Some(o), Some(t)) = (blob(&ours.1), blob(&theirs.1)) else { return whole(Why::Other) };
    let b = s[0].as_ref().and_then(|(_, id)| blob(id)).unwrap_or(b"");
    if [o, b, t].iter().any(|x| x[..x.len().min(8000)].contains(&0)) {
        return whole(Why::Binary);
    }
    let work = fs::read(wt.join(path)).unwrap_or_default();
    let labels = labels(&work);
    let original = merge_file(wt, [o, b, t], [&labels[0], &labels[1], &labels[2]]);
    let blocks = blocks(original.as_deref(), &work);
    if blocks.is_empty() && unmerged {
        // git saw a clash that is not in the lines (a rename's).
        return whole(Why::Other);
    }
    Body::Text(blocks)
}

/// The paths of `paths` whose `merge` attribute says not to merge their
/// lines (`-merge`, `merge=binary`).
fn whole_by_attr(wt: &Path, paths: &[&str]) -> HashSet<String> {
    if paths.is_empty() {
        return HashSet::new();
    }
    let mut c = git::command();
    c.arg("-C").arg(wt).args(["check-attr", "--stdin", "-z", "merge"]);
    let input: Vec<u8> = paths.iter().flat_map(|p| p.bytes().chain([0])).collect();
    let Ok(out) = git::feed(c, &input) else { return HashSet::new() };
    let fields: Vec<&[u8]> = out.split(|&b| b == 0).collect();
    fields
        .chunks(3)
        .filter(|f| f.len() == 3 && (f[2] == b"unset" || f[2] == b"binary"))
        .map(|f| String::from_utf8_lossy(f[0]).into_owned())
        .collect()
}

/// The blobs `ids`, by id (`cat-file --batch`).
pub(crate) fn blobs(wt: &Path, ids: &[Oid]) -> Result<HashMap<Oid, Vec<u8>>, git::Error> {
    let mut map = HashMap::new();
    if ids.is_empty() {
        return Ok(map);
    }
    let mut c = git::command();
    c.arg("-C").arg(wt).args(["cat-file", "--batch"]);
    let input: String = ids.iter().map(|i| format!("{i}\n")).collect();
    let out = git::feed(c, input.as_bytes())?;
    let mut rest = &out[..];
    for id in ids {
        let Some(nl) = rest.iter().position(|&b| b == b'\n') else { break };
        let head = String::from_utf8_lossy(&rest[..nl]).into_owned();
        rest = &rest[nl + 1..];
        // `<oid> blob <size>`, or `<oid> missing`.
        let Some(size) = head.split(' ').nth(2).and_then(|s| s.parse::<usize>().ok()) else { continue };
        if rest.len() < size {
            break;
        }
        map.insert(*id, rest[..size].to_vec());
        rest = &rest[(size + 1).min(rest.len())..];
    }
    Ok(map)
}

/// The labels of the first place's markers in `work` (ours, base, theirs),
/// for `merge-file` to write the same.
fn labels(work: &[u8]) -> [String; 3] {
    let lines = split(work);
    let mut out = ["ours".to_owned(), "base".to_owned(), "theirs".to_owned()];
    if let Some(p) = places(&lines).first() {
        let label = |at: usize| {
            let k = key(lines[at]);
            (k.len() > 8).then(|| String::from_utf8_lossy(&k[8..]).into_owned())
        };
        let found = [Some(p.start), p.base.as_ref().map(|b| b.start - 1), Some(p.end - 1)];
        for (slot, at) in out.iter_mut().zip(found) {
            if let Some(l) = at.and_then(label) {
                *slot = l;
            }
        }
    }
    out
}

static TEMP: AtomicUsize = AtomicUsize::new(0);

/// Temporary files, gone when dropped.
struct Temps(Vec<PathBuf>);

impl Drop for Temps {
    fn drop(&mut self) {
        for p in &self.0 {
            let _ = fs::remove_file(p);
        }
    }
}

/// `git merge-file` of ours, the base and theirs with zdiff3's markers:
/// the file as git left it when it stopped.
fn merge_file(wt: &Path, sides: [&[u8]; 3], labels: [&str; 3]) -> Option<Vec<u8>> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos());
    let n = TEMP.fetch_add(1, Ordering::Relaxed);
    let temps = Temps((0..3).map(|k| std::env::temp_dir().join(format!("michi-merge-{}-{nanos}-{n}-{k}", std::process::id()))).collect());
    for (p, bytes) in temps.0.iter().zip(sides) {
        fs::write(p, bytes).ok()?;
    }
    let mut c = git::command();
    c.arg("-C").arg(wt).args(["merge-file", "-p", "--zdiff3"]);
    for l in labels {
        c.args(["-L", l]);
    }
    c.args(&temps.0);
    let out = c.output().ok()?;
    // The number of places, or a failure (negative).
    matches!(out.status.code(), Some(0..=127)).then_some(out.stdout)
}

/// A place's markers in a file's lines: the lines from `<<<<<<<` to past
/// `>>>>>>>`, and the lines of each side.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Raw {
    start: usize,
    end: usize,
    ours: Range<usize>,
    base: Option<Range<usize>>,
    theirs: Range<usize>,
}

/// `bytes` split after each line break, the breaks kept.
fn split(bytes: &[u8]) -> Vec<&[u8]> {
    bytes.split_inclusive(|&b| b == b'\n').collect()
}

/// A line without its break (`\n` or `\r\n`).
fn key(line: &[u8]) -> &[u8] {
    let line = line.strip_suffix(b"\n").unwrap_or(line);
    line.strip_suffix(b"\r").unwrap_or(line)
}

fn texts(lines: &[&[u8]]) -> Vec<String> {
    lines.iter().map(|l| String::from_utf8_lossy(key(l)).into_owned()).collect()
}

/// A marker of seven `ch`, alone or before a space and a label.
fn is_marker(line: &[u8], ch: u8) -> bool {
    let k = key(line);
    k.len() >= 7 && k[..7].iter().all(|&b| b == ch) && (k.len() == 7 || k[7] == b' ')
}

/// Whether a `|||||||` comes before the next `>>>>>>>` (or `<<<<<<<`) from line `from`.
fn base_ahead(lines: &[&[u8]], from: usize) -> bool {
    for l in lines.iter().skip(from) {
        if is_marker(l, b'|') {
            return true;
        }
        if is_marker(l, b'>') || is_marker(l, b'<') {
            return false;
        }
    }
    false
}

/// The places marked in `lines`. A place not closed is no place; a
/// `<<<<<<<` inside one starts over from it.
fn places(lines: &[&[u8]]) -> Vec<Raw> {
    let mut out = Vec::new();
    let mut i = 0;
    'next: while i < lines.len() {
        if !is_marker(lines[i], b'<') {
            i += 1;
            continue;
        }
        let start = i;
        let (mut base, mut mid) = (None, None);
        let mut j = i + 1;
        while j < lines.len() {
            let l = lines[j];
            if is_marker(l, b'<') {
                i = j;
                continue 'next;
            }
            match mid {
                None if base.is_none() && is_marker(l, b'|') => base = Some(j),
                // A `=======` ahead of the `|||||||` of a diff3 place is a line
                // of our side, not the middle.
                None if key(l) == b"=======" && (base.is_some() || !base_ahead(lines, j + 1)) => mid = Some(j),
                Some(m) if is_marker(l, b'>') => {
                    let ours = start + 1..base.unwrap_or(m);
                    out.push(Raw { start, end: j + 1, ours, base: base.map(|b| b + 1..m), theirs: m + 1..j });
                    i = j + 1;
                    continue 'next;
                }
                _ => {}
            }
            j += 1;
        }
        break;
    }
    out
}

/// The same place: the same lines on each side, line breaks aside.
fn same(a_lines: &[&[u8]], a: &Raw, b_lines: &[&[u8]], b: &Raw) -> bool {
    let keys = |lines: &[&[u8]], r: &Range<usize>| lines[r.clone()].iter().map(|l| key(l).to_vec()).collect::<Vec<_>>();
    keys(a_lines, &a.ours) == keys(b_lines, &b.ours) && keys(a_lines, &a.theirs) == keys(b_lines, &b.theirs)
}

/// A place in the worktree's file: open (its markers) or solved (the lines
/// in its stead).
struct Region {
    span: Range<usize>,
    line: Option<usize>,
    ours: Vec<String>,
    base: Option<Vec<String>>,
    theirs: Vec<String>,
}

/// The places of the worktree's file `work`, solved ones too when
/// `original` (the file as git left it) tells them.
fn blocks(original: Option<&[u8]>, work: &[u8]) -> Vec<Block> {
    let wl = split(work);
    let wp = places(&wl);
    let open = |w: &Raw, base: Option<Vec<String>>| Region {
        span: w.start..w.end,
        line: Some(w.start + 1),
        ours: texts(&wl[w.ours.clone()]),
        base: w.base.as_ref().map(|b| texts(&wl[b.clone()])).or(base),
        theirs: texts(&wl[w.theirs.clone()]),
    };
    let ol = original.map(split).unwrap_or_default();
    let op = places(&ol);
    // Each open place against the original ones, in order.
    let mut pairs = vec![None; op.len()];
    let mut from = 0;
    let mut all_found = true;
    for (i, w) in wp.iter().enumerate() {
        match (from..op.len()).find(|&j| same(&ol, &op[j], &wl, w)) {
            Some(j) => {
                pairs[j] = Some(i);
                from = j + 1;
            }
            None => all_found = false,
        }
    }
    let regions: Vec<Region> = if all_found && !op.is_empty() {
        let map = line_map(&ol, &wl);
        let mut regions: Vec<Region> = op
            .iter()
            .zip(&pairs)
            .map(|(o, pair)| {
                let base = o.base.as_ref().map(|b| texts(&ol[b.clone()]));
                match pair {
                    Some(i) => open(&wp[*i], base),
                    None => {
                        let start = map[..o.start].iter().rev().find_map(|m| *m).map_or(0, |m| m + 1);
                        let end = map[o.end..].iter().find_map(|m| *m).unwrap_or(wl.len());
                        Region {
                            span: start..end.max(start),
                            line: None,
                            ours: texts(&ol[o.ours.clone()]),
                            base,
                            theirs: texts(&ol[o.theirs.clone()]),
                        }
                    }
                }
            })
            .collect();
        // A solved place stays between its neighbours.
        for k in 0..regions.len() {
            if regions[k].line.is_some() {
                continue;
            }
            let low = if k == 0 { 0 } else { regions[k - 1].span.end };
            let high = regions[k + 1..].iter().find(|r| r.line.is_some()).map_or(wl.len(), |r| r.span.start);
            let s = &mut regions[k].span;
            s.start = s.start.clamp(low, high.max(low));
            s.end = s.end.clamp(s.start, high.max(s.start));
        }
        regions
    } else {
        wp.iter().map(|w| open(w, None)).collect()
    };
    with_context(regions, &wl)
}

/// For each line of `old`, where it is in `new` when unchanged.
fn line_map(old: &[&[u8]], new: &[&[u8]]) -> Vec<Option<usize>> {
    let a: Vec<&[u8]> = old.iter().map(|l| key(l)).collect();
    let b: Vec<&[u8]> = new.iter().map(|l| key(l)).collect();
    let mut map = vec![None; a.len()];
    let deadline = Instant::now() + Duration::from_millis(500);
    for op in similar::capture_diff_slices_deadline(similar::Algorithm::Myers, &a, &b, Some(deadline)) {
        if let similar::DiffOp::Equal { old_index, new_index, len } = op {
            for d in 0..len {
                map[old_index + d] = Some(new_index + d);
            }
        }
    }
    map
}

/// The places with the unchanged lines around them: between two places,
/// [`AROUND`] after the first, then [`AROUND`] before the second, the rest
/// skipped.
fn with_context(regions: Vec<Region>, lines: &[&[u8]]) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::with_capacity(regions.len());
    let mut prev = 0;
    for r in regions {
        let start = r.span.start.max(prev);
        let mut gap = start - prev;
        if let Some(last) = out.last_mut() {
            let a = gap.min(AROUND);
            last.after = texts(&lines[prev..prev + a]);
            gap -= a;
        }
        let b = gap.min(AROUND);
        let end = r.span.end.max(start);
        out.push(Block {
            line: r.line,
            result: r.line.is_none().then(|| texts(&lines[start..end])),
            ours: r.ours,
            base: r.base,
            theirs: r.theirs,
            before: texts(&lines[start - b..start]),
            skipped: gap - b,
            after: Vec::new(),
            skipped_after: 0,
        });
        prev = end;
    }
    if let Some(last) = out.last_mut() {
        let gap = lines.len().saturating_sub(prev);
        let a = gap.min(AROUND);
        last.after = texts(&lines[prev..prev + a]);
        last.skipped_after = gap - a;
    }
    out
}

/// Solves the place `block` of the file at `path` in the worktree at `wt`
/// with `pick`: its markers and sides replaced by the lines picked, each
/// with its own line break. When no place is left, the file is added.
/// Whether it was. A place that is not in the file as it was read is
/// refused.
pub fn resolve(wt: &Path, path: &str, block: &Block, pick: Pick) -> Result<bool, git::Error> {
    solve(wt, path, block, Some(pick), &[])
}

/// Like [`resolve`], with `lines` the person wrote in the place instead of
/// a pick; none leaves the place empty.
pub fn resolve_with(wt: &Path, path: &str, block: &Block, lines: &[String]) -> Result<bool, git::Error> {
    solve(wt, path, block, None, lines)
}

fn solve(wt: &Path, path: &str, block: &Block, pick: Option<Pick>, written: &[String]) -> Result<bool, git::Error> {
    let file = wt.join(path);
    let bytes = fs::read(&file).map_err(|e| git::Error::Io(format!("{path}: {e}")))?;
    let lines = split(&bytes);
    let found = places(&lines);
    let raw = found
        .iter()
        .filter(|r| texts(&lines[r.ours.clone()]) == block.ours && texts(&lines[r.theirs.clone()]) == block.theirs)
        .min_by_key(|r| block.line.map_or(0, |l| (r.start + 1).abs_diff(l)))
        .ok_or_else(|| git::Error::Git(format!("{path}: the place has changed since it was read")))?;
    let eol: &[u8] = if lines[raw.start].ends_with(b"\r\n") { b"\r\n" } else { b"\n" };
    let section = |r: &Range<usize>| lines[r.clone()].concat();
    let mut picked = match pick {
        None => written.iter().flat_map(|l| [l.as_bytes(), eol].concat()).collect(),
        Some(Pick::Ours) => section(&raw.ours),
        Some(Pick::Theirs) => section(&raw.theirs),
        Some(Pick::Both) => [section(&raw.ours), section(&raw.theirs)].concat(),
        Some(Pick::BothTheirsFirst) => [section(&raw.theirs), section(&raw.ours)].concat(),
        Some(Pick::Base) => match (&raw.base, &block.base) {
            (Some(b), _) => section(b),
            (None, Some(b)) => b.iter().flat_map(|l| [l.as_bytes(), eol].concat()).collect(),
            (None, None) => return Err(git::Error::Git(format!("{path}: the base of this place is not known"))),
        },
    };
    // The file ended in the markers without a line break: so it ends.
    if !lines[raw.end - 1].ends_with(b"\n") && picked.ends_with(b"\n") {
        picked.pop();
        if picked.ends_with(b"\r") {
            picked.pop();
        }
    }
    let new = [lines[..raw.start].concat(), picked, lines[raw.end..].concat()].concat();
    keep(wt, &[path], &format!("solved a place in {path}"))?;
    fs::write(&file, &new).map_err(|e| git::Error::Io(format!("{path}: {e}")))?;
    if places(&split(&new)).is_empty() {
        add(wt, &[path])?;
        return Ok(true);
    }
    Ok(false)
}

/// Solves the file at `path` in the worktree at `wt`, which clashed whole,
/// with our side's file or theirs: deleted when that side deleted it.
pub fn take(wt: &Path, path: &str, ours: bool) -> Result<(), git::Error> {
    let stages = stages(&git::read(wt, &["ls-files", "-u", "-z", "--", path])?);
    let has = stages.iter().any(|(p, s)| p == path && s[if ours { 1 } else { 2 }].is_some());
    // What the file has now (the markers, the hand edits) is kept before it is replaced or deleted.
    keep(wt, &[path], &format!("took {} for {path}", if ours { "ours" } else { "theirs" }))?;
    if has {
        write(wt, &["checkout", if ours { "--ours" } else { "--theirs" }], &[path])?;
        add(wt, &[path])
    } else {
        write(wt, &["rm", "-q"], &[path]).map(drop)
    }
}

/// Starts the file at `path` in the worktree at `wt` over: its places back
/// as git left them, open, in the file and the index. What the file had is
/// kept under a backup first; its ref.
pub fn restart(wt: &Path, path: &str) -> Result<String, git::Error> {
    let kept = keep(wt, &[path], &format!("started {path} over"))?;
    write(wt, &["checkout", "--conflict=zdiff3"], &[path])?;
    Ok(kept)
}

/// The commit that last changed a place's lines on one side.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub id: Oid,
    pub subject: String,
    pub author: String,
    /// Seconds since the epoch.
    pub time: i64,
}

/// What each side meant by a place: the commit since the base that
/// changed its lines (`git log -L`). `None` when the side has no lines
/// there or git could not follow them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Intent {
    pub ours: Option<Change>,
    pub theirs: Option<Change>,
}

/// The intent of both sides at `block` of the file at `path` in the
/// worktree at `wt`, whose conflict is `c`.
pub fn intent(wt: &Path, path: &str, block: &Block, c: &Conflict) -> Intent {
    let base = c.base.as_ref();
    let of = |lines: &[String], tip: Option<&Oid>| tip.and_then(|t| change_of(wt, path, lines, t, base));
    Intent { ours: of(&block.ours, c.ours.id.as_ref()), theirs: of(&block.theirs, c.theirs.id.as_ref()) }
}

/// The newest commit in `base..tip` that touched `lines` of the file at `path`.
fn change_of(wt: &Path, path: &str, lines: &[String], tip: &Oid, base: Option<&Oid>) -> Option<Change> {
    if lines.is_empty() {
        return None;
    }
    let blob = git::read(wt, &["show", &format!("{tip}:{path}")]).ok()?;
    let text = String::from_utf8_lossy(&blob);
    let all: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let at = all.windows(lines.len()).position(|w| w.iter().zip(lines).all(|(a, b)| *a == b.trim_end_matches('\r')))?;
    let range = format!("-L{},{}:{path}", at + 1, at + lines.len());
    let tip = tip.to_string();
    let span = base.map(|b| format!("{b}..{tip}")).unwrap_or(tip);
    let out = git::read(wt, &["log", "-n1", "-s", "--format=%H%x1f%an%x1f%ct%x1f%s", &range, &span]).ok()?;
    let out = String::from_utf8_lossy(&out);
    let mut f = out.lines().next()?.split('\x1f');
    Some(Change {
        id: Oid::parse(f.next()?)?,
        author: f.next()?.to_owned(),
        time: f.next()?.parse().ok()?,
        subject: f.next().unwrap_or_default().to_owned(),
    })
}

/// Makes git's rerere forget the resolution it applied to the file at `path`
/// in the worktree at `wt`, then starts the file over (git leaves the
/// applied text in it). The backup's ref. git answers `error:` with a zero
/// status when it knows no such resolution, so the text is what is looked at.
pub fn forget(wt: &Path, path: &str) -> Result<String, git::Error> {
    let mut cmd = git::command();
    cmd.arg("-C").arg(wt).env("GIT_LITERAL_PATHSPECS", "1").args(["rerere", "forget", "--", path]);
    let out = cmd.output().map_err(git::started)?;
    let err = String::from_utf8_lossy(&out.stderr);
    if !out.status.success() || err.lines().any(|l| l.starts_with("error:") || l.starts_with("fatal:")) {
        return Err(git::refused_all(&err));
    }
    restart(wt, path)
}

/// Commits what the worktree at `wt` stopped in: the files solved by hand
/// are added first, and none may be left. A merge, a cherry-pick or a
/// revert commits with `message` (a cherry-pick keeps its author); a rebase
/// goes on with the rest of its commits, which may stop again. The head.
pub fn commit(wt: &Path, message: &str) -> Result<Oid, git::Error> {
    let Some(c) = read(wt)? else { return Err(git::Error::Git("nothing to commit a merge for".into())) };
    if c.left() > 0 {
        return Err(git::Error::Git(format!("{} places are not solved yet", c.left())));
    }
    let solved: Vec<&str> = c.files.iter().filter(|f| f.unmerged && matches!(f.body, Body::Text(_))).map(|f| f.path.as_str()).collect();
    if !solved.is_empty() {
        add(wt, &solved)?;
    }
    let left = unmerged(wt)?;
    if !left.is_empty() {
        return Err(git::Error::Git(format!("still unmerged: {}", left.join(", "))));
    }
    if c.doing == Doing::Rebase {
        let mut cmd = git::command();
        cmd.env("GIT_EDITOR", "true").arg("-C").arg(wt).args(["rebase", "--continue"]);
        let out = cmd.output().map_err(git::started)?;
        if !out.status.success() && unmerged(wt)?.is_empty() {
            return Err(git::refused_all(&String::from_utf8_lossy(&out.stderr)));
        }
        let head = git::read_text(wt, &["rev-parse", "HEAD"])?;
        return Oid::parse(&head).ok_or_else(|| git::Error::Io(format!("git rev-parse: {head}")));
    }
    stage::commit(wt, message, false)
}

/// Calls off what the worktree at `wt` is in the middle of: its branch and
/// files back as they were before (`git merge --abort` and the like). The
/// files that clashed or came in are kept under a backup first; its ref,
/// `None` when there were none.
pub fn abort(wt: &Path) -> Result<Option<String>, git::Error> {
    let Some(c) = read(wt)? else { return Err(git::Error::Git("nothing to call off".into())) };
    let paths: Vec<&str> = c.files.iter().map(|f| f.path.as_str()).chain(c.auto.iter().map(|a| a.path.as_str())).collect();
    let kept = if paths.is_empty() { None } else { Some(keep(wt, &paths, &format!("called off a {}", c.doing.word()))?) };
    branch::write(wt, &[c.doing.word(), "--abort"])?;
    Ok(kept)
}

/// Merges `rev` into the branch of the worktree at `wt` (forwarding it when
/// it can), with git's message. A clash stops to be solved. A worktree in
/// the middle of something is refused, and so is one whose changes the
/// merge would overwrite.
pub fn merge(wt: &Path, rev: &str) -> Result<Outcome, git::Error> {
    idle(wt)?;
    let before = self::rev(wt, "HEAD");
    let out = git::command()
        .arg("-C")
        .arg(wt)
        .args(["-c", "merge.conflictStyle=zdiff3", "merge", "--no-edit", rev])
        .output()
        .map_err(git::started)?;
    if out.status.success() {
        let after = self::rev(wt, "HEAD").ok_or_else(no_head)?;
        return Ok(if before == Some(after) { Outcome::UpToDate } else { Outcome::Made(after) });
    }
    stopped(wt, Doing::Merge, &out)
}

/// Picks the commit `id` onto the branch of the worktree at `wt`, with its
/// message and author. A merge needs `mainline`, the parent (from 1) its
/// changes are taken against. A clash stops to be solved.
pub fn cherry_pick(wt: &Path, id: &Oid, mainline: Option<u32>) -> Result<Outcome, git::Error> {
    idle(wt)?;
    let id = id.to_string();
    let m = mainline.map(|n| n.to_string());
    let mut args = vec!["-c", "merge.conflictStyle=zdiff3", "cherry-pick"];
    if let Some(m) = &m {
        args.extend(["-m", m]);
    }
    args.push(&id);
    let out = git::command().arg("-C").arg(wt).args(&args).output().map_err(git::started)?;
    if out.status.success() {
        return Ok(Outcome::Made(self::rev(wt, "HEAD").ok_or_else(no_head)?));
    }
    stopped(wt, Doing::CherryPick, &out)
}

/// Refuses a worktree in the middle of something.
pub(crate) fn idle(wt: &Path) -> Result<(), git::Error> {
    match branch::doing(wt) {
        Some(d) => Err(git::Error::Git(format!("this worktree is in the middle of a {}", d.word()))),
        None => Ok(()),
    }
}

/// After git stopped `doing` with a failure: the files that clashed, left
/// to be solved; nothing when there was nothing to commit; else (a hook
/// refused the commit) it is called off, and what git said is the error.
pub(crate) fn stopped(wt: &Path, doing: Doing, out: &Output) -> Result<Outcome, git::Error> {
    let said = String::from_utf8_lossy(&out.stderr);
    if branch::doing(wt) != Some(doing) {
        // Nothing to commit: git says so on its standard output only.
        if said.trim().is_empty() && out.status.code() == Some(1) {
            return Ok(Outcome::Empty);
        }
        return Err(git::refused_all(&said));
    }
    let clashed = unmerged(wt)?;
    if !clashed.is_empty() {
        return Ok(Outcome::Conflict(clashed));
    }
    let staged = !git::check(wt, &["diff", "--cached", "--quiet", "HEAD"])?;
    branch::write(wt, &[doing.word(), "--abort"])?;
    if staged {
        Err(git::refused_all(&said))
    } else {
        Ok(Outcome::Empty)
    }
}

fn no_head() -> git::Error {
    git::Error::Io("HEAD: no commit".to_owned())
}

/// `git args… -- paths` in the worktree at `wt`, the paths as they are.
fn write(wt: &Path, args: &[&str], paths: &[&str]) -> Result<Vec<u8>, git::Error> {
    let mut c = git::command();
    c.env("GIT_LITERAL_PATHSPECS", "1").arg("-C").arg(wt).args(args).arg("--").args(paths);
    discard::run(c)
}

/// Adds `paths` as the worktree has them, deleted ones too.
fn add(wt: &Path, paths: &[&str]) -> Result<(), git::Error> {
    write(wt, &["add", "-A"], paths).map(drop)
}

/// The files at `paths` of the worktree at `wt` as they are now, kept in a
/// commit on the head under a backup with `why`; its ref.
fn keep(wt: &Path, paths: &[&str], why: &str) -> Result<String, git::Error> {
    let head = rev(wt, "HEAD").ok_or_else(no_head)?;
    let index = TempIndex::new();
    let mut c = index.command(wt);
    c.args(["read-tree", &head.to_string()]);
    discard::run(c)?;
    let (here, gone): (Vec<&str>, Vec<&str>) = paths.iter().partition(|p| wt.join(p).exists());
    if !here.is_empty() {
        let mut c = index.command(wt);
        c.env("GIT_LITERAL_PATHSPECS", "1").args(["add", "-f", "--"]).args(&here);
        discard::run(c)?;
    }
    if !gone.is_empty() {
        let mut c = index.command(wt);
        c.env("GIT_LITERAL_PATHSPECS", "1").args(["rm", "--cached", "-q", "--ignore-unmatch", "--"]).args(&gone);
        discard::run(c)?;
    }
    let mut c = index.command(wt);
    c.arg("write-tree");
    let tree = String::from_utf8_lossy(&discard::run(c)?).trim().to_owned();
    let mut c = git::command();
    c.arg("-C").arg(wt).args(["commit-tree", &tree, "-p", &head.to_string(), "-m", &format!("michi: {why}")]);
    let out = discard::run(c)?;
    let id = Oid::parse(String::from_utf8_lossy(&out).trim()).ok_or_else(|| git::Error::Io("git commit-tree: no commit".into()))?;
    backup::save(wt, &id, why)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TempRepo;

    fn s(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|l| (*l).to_owned()).collect()
    }

    fn text(t: &TempRepo, rel: &str) -> String {
        fs::read_to_string(t.repo.join(rel)).unwrap()
    }

    fn blocks_of(c: &Conflict, n: usize) -> &[Block] {
        let Body::Text(blocks) = &c.files[n].body else { panic!("{:?}", c.files[n]) };
        blocks
    }

    /// a.txt clashes in lines 2 and 9, b.txt in its one line; c.txt changed
    /// on `feat` only.
    fn clash(name: &str) -> TempRepo {
        let t = TempRepo::configured(name);
        let r = &t.repo;
        let a = |two: &str, nine: &str| format!("1\n{two}\n3\n4\n5\n6\n7\n8\n{nine}\n10\n");
        t.write(r, "a.txt", &a("2", "9"));
        t.write(r, "b.txt", "x\n");
        t.write(r, "c.txt", "c\n");
        t.git(r, &["add", "."]);
        t.git(r, &["commit", "-q", "-m", "base"]);
        t.git(r, &["switch", "-q", "-c", "feat"]);
        t.write(r, "a.txt", &a("two feat", "nine feat"));
        t.write(r, "b.txt", "y\n");
        t.write(r, "c.txt", "c feat\n");
        t.git(r, &["commit", "-q", "-am", "feat"]);
        t.git(r, &["switch", "-q", "main"]);
        t.write(r, "a.txt", &a("two main", "nine main"));
        t.write(r, "b.txt", "z\n");
        t.git(r, &["commit", "-q", "-am", "main"]);
        t
    }

    #[test]
    fn a_merge_stops_and_its_places_are_solved_one_by_one() {
        let t = clash("conflict-merge");
        let r = &t.repo;
        assert_eq!(merge(r, "feat").unwrap(), Outcome::Conflict(s(&["a.txt", "b.txt"])));
        let c = read(r).unwrap().unwrap();
        assert_eq!(c.doing, Doing::Merge);
        assert_eq!((c.ours.name.as_str(), c.theirs.name.as_str()), ("main", "feat"));
        let feat = t.out(r, &["rev-parse", "feat"]);
        assert_eq!(c.theirs.id.map(|i| i.to_string()), Some(feat.clone()));
        assert_eq!(c.base.map(|i| i.to_string()), Some(t.out(r, &["rev-parse", "feat^"])));
        assert_eq!(c.auto, vec![Auto { path: "c.txt".into(), both: false }]);
        assert_eq!((c.total(), c.left()), (3, 3));
        assert_eq!(c.message, "Merge branch 'feat'");
        assert!(c.started.is_some());

        let blocks = blocks_of(&c, 0);
        let b0 = &blocks[0];
        assert_eq!(b0.line, Some(2));
        assert_eq!((&b0.ours, &b0.base, &b0.theirs), (&s(&["two main"]), &Some(s(&["2"])), &s(&["two feat"])));
        assert_eq!((&b0.before, b0.skipped, &b0.after), (&s(&["1"]), 0, &s(&["3", "4", "5"])));
        assert_eq!(b0.hint(), Hint::BothChanged(1));
        let b1 = &blocks[1];
        assert_eq!((b1.line, &b1.before, b1.skipped), (Some(15), &s(&["6", "7", "8"]), 0));
        assert_eq!((&b1.after, b1.skipped_after), (&s(&["10"]), 0));

        assert!(!resolve(r, "a.txt", b0, Pick::Ours).unwrap());
        let c = read(r).unwrap().unwrap();
        let blocks = blocks_of(&c, 0);
        assert_eq!((blocks[0].line, &blocks[0].result), (None, &Some(s(&["two main"]))));
        assert_eq!(blocks[1].line, Some(9));
        assert_eq!(blocks[1].before, s(&["6", "7", "8"]), "the solved place's lines are its own");
        assert_eq!(c.left(), 2);
        assert!(resolve(r, "a.txt", &blocks[1], Pick::Both).unwrap(), "the last place: added");
        assert_eq!(text(&t, "a.txt"), "1\ntwo main\n3\n4\n5\n6\n7\n8\nnine main\nnine feat\n10\n");

        let c = read(r).unwrap().unwrap();
        assert!(!c.files[0].unmerged);
        let blocks = blocks_of(&c, 0);
        assert_eq!(blocks.len(), 2, "still shown once added");
        assert_eq!(blocks[0].result, Some(s(&["two main"])));
        assert_eq!(blocks[1].result, Some(s(&["nine main", "nine feat"])));
        assert!(resolve(r, "b.txt", &blocks_of(&c, 1)[0], Pick::Base).unwrap());
        assert_eq!(text(&t, "b.txt"), "x\n");
        let c = read(r).unwrap().unwrap();
        assert_eq!(c.left(), 0);

        commit(r, &c.message).unwrap();
        assert_eq!(t.out(r, &["rev-parse", "HEAD^2"]), feat);
        assert_eq!(t.out(r, &["log", "-1", "--format=%s"]), "Merge branch 'feat'");
        assert_eq!(read(r).unwrap(), None);
        assert_eq!(merge(r, "feat").unwrap(), Outcome::UpToDate);
    }

    #[test]
    fn a_place_solved_by_hand_counts_as_solved() {
        let t = clash("conflict-hand");
        let r = &t.repo;
        merge(r, "feat").unwrap();
        let was = text(&t, "a.txt");
        let at = was.rfind("<<<<<<<").unwrap();
        let end = was.rfind(">>>>>>> feat\n").unwrap() + ">>>>>>> feat\n".len();
        fs::write(r.join("a.txt"), format!("{}nine both\n{}", &was[..at], &was[end..])).unwrap();
        let c = read(r).unwrap().unwrap();
        let blocks = blocks_of(&c, 0);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].is_open());
        assert_eq!((&blocks[1].result, &blocks[1].theirs), (&Some(s(&["nine both"])), &s(&["nine feat"])));
        assert_eq!(blocks[1].after, s(&["10"]));
        assert!(c.files[0].unmerged);
        assert_eq!(c.left(), 2);
    }

    #[test]
    fn plain_markers_take_the_base_from_the_stages() {
        let t = clash("conflict-plain");
        let r = &t.repo;
        let out = git::command().arg("-C").arg(r).args(["-c", "merge.conflictStyle=merge", "merge", "--no-edit", "feat"]).output().unwrap();
        assert!(!out.status.success());
        assert!(!text(&t, "a.txt").contains("|||||||"));
        let c = read(r).unwrap().unwrap();
        let blocks = blocks_of(&c, 0);
        assert_eq!((blocks.len(), &blocks[0].base), (2, &Some(s(&["2"]))));
        assert!(!resolve(r, "a.txt", &blocks[0], Pick::Base).unwrap());
        assert!(text(&t, "a.txt").starts_with("1\n2\n3\n"));
    }

    #[test]
    fn crlf_lines_keep_their_breaks() {
        let t = TempRepo::configured("conflict-crlf");
        let r = &t.repo;
        t.write(r, "w.txt", "a\r\nb\r\nc\r\n");
        t.git(r, &["add", "."]);
        t.git(r, &["commit", "-q", "-m", "base"]);
        t.git(r, &["switch", "-q", "-c", "feat"]);
        t.write(r, "w.txt", "a\r\nb feat\r\nc\r\n");
        t.git(r, &["commit", "-q", "-am", "feat"]);
        t.git(r, &["switch", "-q", "main"]);
        t.write(r, "w.txt", "a\r\nb main\r\nc\r\n");
        t.git(r, &["commit", "-q", "-am", "main"]);
        assert_eq!(merge(r, "feat").unwrap(), Outcome::Conflict(s(&["w.txt"])));
        let c = read(r).unwrap().unwrap();
        let b = &blocks_of(&c, 0)[0];
        assert_eq!((&b.ours, &b.base, &b.theirs), (&s(&["b main"]), &Some(s(&["b"])), &s(&["b feat"])));
        assert!(resolve(r, "w.txt", b, Pick::BothTheirsFirst).unwrap());
        assert_eq!(text(&t, "w.txt"), "a\r\nb feat\r\nb main\r\nc\r\n");
        let c = read(r).unwrap().unwrap();
        assert_eq!(blocks_of(&c, 0)[0].result, Some(s(&["b feat", "b main"])));
    }

    #[test]
    fn a_stale_place_is_refused() {
        let t = clash("conflict-stale");
        let r = &t.repo;
        merge(r, "feat").unwrap();
        let c = read(r).unwrap().unwrap();
        let b = blocks_of(&c, 1)[0].clone();
        fs::write(r.join("b.txt"), "mine\n").unwrap();
        let e = resolve(r, "b.txt", &b, Pick::Ours).unwrap_err();
        assert!(e.to_string().contains("the place has changed"), "{e}");
        assert_eq!(text(&t, "b.txt"), "mine\n");
    }

    #[test]
    fn a_file_starts_over_after_a_backup() {
        let t = clash("conflict-restart");
        let r = &t.repo;
        merge(r, "feat").unwrap();
        let c = read(r).unwrap().unwrap();
        assert!(resolve(r, "b.txt", &blocks_of(&c, 1)[0], Pick::Theirs).unwrap());
        assert!(!read(r).unwrap().unwrap().files[1].unmerged);
        let kept = restart(r, "b.txt").unwrap();
        assert!(kept.starts_with(backup::PREFIX), "{kept}");
        assert_eq!(t.out(r, &["show", &format!("{kept}:b.txt")]), "y");
        let c = read(r).unwrap().unwrap();
        assert!(c.files[1].unmerged);
        assert_eq!(c.left(), 3);
        assert!(blocks_of(&c, 1)[0].is_open());
    }

    #[test]
    fn calling_off_keeps_what_was_solved() {
        let t = clash("conflict-abort");
        let r = &t.repo;
        let head = t.out(r, &["rev-parse", "HEAD"]);
        merge(r, "feat").unwrap();
        let c = read(r).unwrap().unwrap();
        resolve(r, "b.txt", &blocks_of(&c, 1)[0], Pick::Theirs).unwrap();
        let kept = abort(r).unwrap().unwrap();
        assert_eq!(t.out(r, &["show", &format!("{kept}:b.txt")]), "y");
        assert_eq!(t.out(r, &["show", &format!("{kept}:c.txt")]), "c feat");
        assert_eq!(t.out(r, &["rev-parse", "HEAD"]), head);
        assert_eq!(branch::doing(r), None);
        assert_eq!(text(&t, "b.txt"), "z\n");
        assert_eq!(read(r).unwrap(), None);
    }

    #[test]
    fn a_deleted_file_is_taken_whole() {
        let t = TempRepo::configured("conflict-deleted");
        let r = &t.repo;
        t.write(r, "d.txt", "d\n");
        t.write(r, "e.txt", "e\n");
        t.git(r, &["add", "."]);
        t.git(r, &["commit", "-q", "-m", "base"]);
        t.git(r, &["switch", "-q", "-c", "feat"]);
        t.write(r, "d.txt", "d feat\n");
        t.write(r, "e.txt", "e feat\n");
        t.git(r, &["commit", "-q", "-am", "feat"]);
        t.git(r, &["switch", "-q", "main"]);
        t.git(r, &["rm", "-q", "d.txt", "e.txt"]);
        t.git(r, &["commit", "-q", "-m", "main"]);
        assert_eq!(merge(r, "feat").unwrap(), Outcome::Conflict(s(&["d.txt", "e.txt"])));
        let c = read(r).unwrap().unwrap();
        assert_eq!(
            c.files[0].body,
            Body::Whole(Whole { why: Why::Deleted, did_ours: Fact::Deleted, did_theirs: Fact::Changed, ours: false, theirs: true })
        );
        assert_eq!((c.total(), c.left()), (2, 2));
        take(r, "d.txt", true).unwrap();
        assert_eq!(t.out(r, &["for-each-ref", "refs/michi/backup"]).lines().count(), 1, "the file is kept before it is taken");
        assert!(!r.join("d.txt").exists());
        take(r, "e.txt", false).unwrap();
        assert_eq!(text(&t, "e.txt"), "e feat\n");
        assert_eq!(read(r).unwrap().unwrap().left(), 0);
        commit(r, "Merge feat").unwrap();
        assert_eq!(t.out(r, &["ls-tree", "--name-only", "HEAD"]), "e.txt");
    }

    #[test]
    fn what_a_side_did_comes_from_the_index() {
        let id = |c: char| Oid::parse(&c.to_string().repeat(40)).unwrap();
        let f = |m: &str, c: char| Some((m.to_owned(), id(c)));
        let base = f("100644", 'a');
        assert_eq!(fact(base.as_ref(), None), Fact::Deleted);
        assert_eq!(fact(None, f("100644", 'b').as_ref()), Fact::Added);
        assert_eq!(fact(base.as_ref(), f("100644", 'b').as_ref()), Fact::Changed);
        assert_eq!(fact(base.as_ref(), f("100644", 'a').as_ref()), Fact::Same);
        assert_eq!(fact(base.as_ref(), f("100755", 'a').as_ref()), Fact::Mode { exec: true });
        assert_eq!(fact(f("100755", 'a').as_ref(), f("100644", 'a').as_ref()), Fact::Mode { exec: false });
        assert_eq!(fact(base.as_ref(), f("120000", 'c').as_ref()), Fact::Link);
        assert_eq!(fact(base.as_ref(), f("160000", 'c').as_ref()), Fact::Submodule);
    }

    #[test]
    fn a_rename_on_both_sides_is_left_as_moved() {
        let t = TempRepo::configured("conflict-moved");
        let r = &t.repo;
        t.write(r, "a.txt", "one\ntwo\nthree\nfour\nfive\nsix\n");
        t.git(r, &["add", "."]);
        t.git(r, &["commit", "-q", "-m", "base"]);
        t.git(r, &["switch", "-q", "-c", "feat"]);
        t.git(r, &["mv", "a.txt", "b.txt"]);
        t.git(r, &["commit", "-q", "-m", "feat"]);
        t.git(r, &["switch", "-q", "main"]);
        t.git(r, &["mv", "a.txt", "c.txt"]);
        t.git(r, &["commit", "-q", "-m", "main"]);
        assert!(matches!(merge(r, "feat").unwrap(), Outcome::Conflict(_)));
        let c = read(r).unwrap().unwrap();
        let whys: Vec<(String, Why)> = c
            .files
            .iter()
            .filter_map(|f| match &f.body {
                Body::Whole(w) => Some((f.path.clone(), w.why)),
                Body::Text(_) => None,
            })
            .collect();
        assert!(!whys.is_empty() && whys.iter().all(|(_, w)| *w == Why::Moved), "{whys:?}");
        assert!(whys.iter().any(|(p, _)| p == "a.txt"), "{whys:?}");
    }

    #[test]
    fn a_binary_file_is_taken_whole() {
        let t = TempRepo::configured("conflict-binary");
        let r = &t.repo;
        t.write(r, "x.bin", "a\0\n");
        t.git(r, &["add", "."]);
        t.git(r, &["commit", "-q", "-m", "base"]);
        t.git(r, &["switch", "-q", "-c", "feat"]);
        t.write(r, "x.bin", "b\0\n");
        t.git(r, &["commit", "-q", "-am", "feat"]);
        t.git(r, &["switch", "-q", "main"]);
        t.write(r, "x.bin", "c\0\n");
        t.git(r, &["commit", "-q", "-am", "main"]);
        assert_eq!(merge(r, "feat").unwrap(), Outcome::Conflict(s(&["x.bin"])));
        let c = read(r).unwrap().unwrap();
        assert_eq!(
            c.files[0].body,
            Body::Whole(Whole { why: Why::Binary, did_ours: Fact::Changed, did_theirs: Fact::Changed, ours: true, theirs: true })
        );
        take(r, "x.bin", false).unwrap();
        assert_eq!(text(&t, "x.bin"), "b\0\n");
        assert_eq!(read(r).unwrap().unwrap().left(), 0);
    }

    #[test]
    fn a_pick_that_clashes_keeps_its_message() {
        let t = clash("conflict-pick");
        let r = &t.repo;
        let pick = Oid::parse(&t.out(r, &["rev-parse", "feat"])).unwrap();
        let head = t.out(r, &["rev-parse", "HEAD"]);
        assert_eq!(cherry_pick(r, &pick, None).unwrap(), Outcome::Conflict(s(&["a.txt", "b.txt"])));
        let c = read(r).unwrap().unwrap();
        assert_eq!(c.doing, Doing::CherryPick);
        assert_eq!((c.theirs.name.clone(), c.theirs.id), (pick.short(), Some(pick)));
        assert_eq!(c.base.map(|i| i.to_string()), Some(t.out(r, &["rev-parse", "feat^"])));
        assert_eq!(c.message, "feat");
        assert_eq!(c.auto, vec![Auto { path: "c.txt".into(), both: false }]);
        solve_all(r, Pick::Theirs);
        let c = read(r).unwrap().unwrap();
        assert_eq!(c.left(), 0);
        commit(r, &c.message).unwrap();
        assert_eq!(t.out(r, &["log", "-1", "--format=%s"]), "feat");
        assert_eq!(t.out(r, &["rev-parse", "HEAD^"]), head);
        assert_eq!(branch::doing(r), None);
    }

    /// Solves every open place with `pick`, one at a time.
    fn solve_all(r: &Path, pick: Pick) {
        loop {
            let c = read(r).unwrap().unwrap();
            let open = c.files.iter().find_map(|f| match &f.body {
                Body::Text(blocks) => blocks.iter().find(|b| b.is_open()).map(|b| (f.path.clone(), b.clone())),
                Body::Whole(_) => None,
            });
            let Some((path, block)) = open else { break };
            resolve(r, &path, &block, pick).unwrap();
        }
    }

    #[test]
    fn a_rebase_goes_on_after_the_clash() {
        let t = clash("conflict-rebase");
        let r = &t.repo;
        t.git(r, &["switch", "-q", "feat"]);
        let out = git::command().arg("-C").arg(r).args(["-c", "merge.conflictStyle=zdiff3", "rebase", "main"]).output().unwrap();
        assert!(!out.status.success());
        let c = read(r).unwrap().unwrap();
        assert_eq!(c.doing, Doing::Rebase);
        assert_eq!((c.ours.name.as_str(), c.theirs.name.as_str(), c.message.as_str()), ("", "feat", ""));
        assert!(c.started.is_some());
        assert_eq!(c.left(), 3);
        solve_all(r, Pick::Theirs);
        commit(r, "").unwrap();
        assert_eq!(branch::doing(r), None);
        assert_eq!(t.out(r, &["branch", "--show-current"]), "feat");
        assert_eq!(t.out(r, &["log", "-1", "--format=%s"]), "feat");
        assert_eq!(t.out(r, &["rev-parse", "HEAD^"]), t.out(r, &["rev-parse", "main"]));
    }

    #[test]
    fn a_place_is_solved_with_the_lines_written_in_it() {
        let t = clash("conflict-written");
        let r = &t.repo;
        merge(r, "feat").unwrap();
        let c = read(r).unwrap().unwrap();
        let Body::Text(blocks) = &c.files[0].body else { panic!() };
        assert!(blocks.iter().filter(|b| b.is_open()).count() >= 2);
        let first = blocks.iter().find(|b| b.is_open()).unwrap().clone();
        let lines = s(&["mine", "and yours"]);
        resolve_with(r, "a.txt", &first, &lines).unwrap();
        let c = read(r).unwrap().unwrap();
        let Body::Text(blocks) = &c.files[0].body else { panic!() };
        assert!(blocks.iter().any(|b| b.result.as_ref() == Some(&lines)));
        let text = std::fs::read_to_string(r.join("a.txt")).unwrap();
        assert!(text.replace("\r\n", "\n").contains("mine\nand yours\n"), "{text}");
    }

    #[test]
    fn each_side_s_intent_is_the_commit_that_changed_the_place() {
        let t = clash("conflict-intent");
        let r = &t.repo;
        merge(r, "feat").unwrap();
        let c = read(r).unwrap().unwrap();
        let Body::Text(blocks) = &c.files[0].body else { panic!() };
        let i = intent(r, "a.txt", &blocks[1], &c);
        let (o, th) = (i.ours.unwrap(), i.theirs.unwrap());
        assert_eq!((o.subject.as_str(), th.subject.as_str()), ("main", "feat"));
        assert_eq!(o.id.to_string(), t.out(r, &["rev-parse", "main"]));
        assert_eq!(th.id.to_string(), t.out(r, &["rev-parse", "feat"]));
        assert!(o.time > 0 && !o.author.is_empty());
    }

    #[test]
    fn a_resolution_rerere_applied_is_forgotten_and_the_file_starts_over() {
        let t = clash("conflict-forget");
        let r = &t.repo;
        t.git(r, &["config", "rerere.enabled", "true"]);
        t.git(r, &["config", "core.autocrlf", "false"]);
        assert!(matches!(merge(r, "feat").unwrap(), Outcome::Conflict(_)));
        solve_all(r, Pick::Theirs);
        t.git(r, &["rerere"]);
        abort(r).unwrap();
        // The same clash again: rerere solves a.txt and b.txt on its own.
        assert!(matches!(merge(r, "feat").unwrap(), Outcome::Conflict(_)));
        let c = read(r).unwrap().unwrap();
        assert_eq!(c.left(), 0);
        assert!(c.files.iter().all(|f| f.unmerged));
        let saved = forget(r, "a.txt").unwrap();
        assert!(saved.starts_with("refs/michi/backup/"), "{saved}");
        let c = read(r).unwrap().unwrap();
        assert_eq!(c.left(), 2);
        assert!(text(&t, "a.txt").contains("<<<<<<<"));
        // Nothing is remembered now: git's refusal comes back as the error.
        assert!(forget(r, "a.txt").is_err());
    }

    #[test]
    fn a_merge_without_a_clash_is_made() {
        let t = clash("conflict-clean");
        let r = &t.repo;
        t.git(r, &["switch", "-q", "-c", "other", "main~1"]);
        t.write(r, "o.txt", "o\n");
        t.git(r, &["add", "o.txt"]);
        t.git(r, &["commit", "-q", "-m", "other"]);
        t.git(r, &["switch", "-q", "main"]);
        let Outcome::Made(id) = merge(r, "other").unwrap() else { panic!() };
        assert_eq!(t.out(r, &["rev-parse", "HEAD"]), id.to_string());
        assert_eq!(merge(r, "main~1").unwrap(), Outcome::UpToDate);
        t.write(r, "a.txt", "dirty\n");
        assert!(merge(r, "feat").is_err(), "changes it would overwrite");
        assert_eq!(branch::doing(r), None);
    }

    #[test]
    fn markers_are_read_with_or_without_a_base() {
        let bytes = b"a\n<<<<<<< x\n<<<<<<< ours\no\n||||||| base\nb\n=======\nt\n>>>>>>> theirs\n<<<<<<<\nu\n=======\nv\n>>>>>>>\n<<<<<<< open\nw\n";
        let lines = split(bytes);
        assert_eq!(
            places(&lines),
            vec![
                Raw { start: 2, end: 9, ours: 3..4, base: Some(5..6), theirs: 7..8 },
                Raw { start: 9, end: 14, ours: 10..11, base: None, theirs: 12..13 },
            ]
        );
        assert_eq!(labels(bytes), ["ours".to_owned(), "base".to_owned(), "theirs".to_owned()]);
        assert!(!is_marker(b"<<<<<<<< no\n", b'<'));
        assert!(is_marker(b">>>>>>>\r\n", b'>'));
    }

    #[test]
    fn a_line_of_equals_is_the_middle_only_where_it_belongs() {
        // `=======` in our side of a diff3 place is content, not the middle.
        let bytes = b"<<<<<<< ours
Title
=======
more
||||||| base
b
=======
t
>>>>>>> theirs
";
        assert_eq!(places(&split(bytes)), vec![Raw { start: 0, end: 9, ours: 1..4, base: Some(5..6), theirs: 7..8 }]);
        // Without a base the first one is the middle; one outside a place is nothing.
        let bytes = b"=======
<<<<<<< a
o
=======
t
>>>>>>> b
=======
";
        assert_eq!(places(&split(bytes)), vec![Raw { start: 1, end: 6, ours: 2..3, base: None, theirs: 4..5 }]);
        // Indented, or with a label, it is not a middle.
        assert!(places(&split(
            b"<<<<<<< a
 o
 =======
>>>>>>> b
"
        ))
        .is_empty());
        assert!(places(&split(
            b"<<<<<<< a
======= x
>>>>>>> b
"
        ))
        .is_empty());
    }

    #[test]
    fn the_lines_around_are_cut_to_three() {
        let mut w = String::new();
        let nums = |w: &mut String, r: std::ops::RangeInclusive<usize>| r.for_each(|i| w.push_str(&format!("{i}\n")));
        nums(&mut w, 1..=5);
        w.push_str("<<<<<<< a\nx\n=======\ny\n>>>>>>> b\n");
        nums(&mut w, 6..=15);
        w.push_str("<<<<<<< a\n=======\ny2\n>>>>>>> b\n");
        nums(&mut w, 16..=20);
        let b = blocks(None, w.as_bytes());
        assert_eq!(b.len(), 2);
        assert_eq!((&b[0].before, b[0].skipped, &b[0].after), (&s(&["3", "4", "5"]), 2, &s(&["6", "7", "8"])));
        assert_eq!((b[1].line, &b[1].before, b[1].skipped), (Some(21), &s(&["13", "14", "15"]), 4));
        assert_eq!((&b[1].after, b[1].skipped_after), (&s(&["16", "17", "18"]), 2));
        assert_eq!((b[0].hint(), b[1].hint()), (Hint::BothChanged(0), Hint::OursDeleted));
        assert_eq!(b[0].lines(Pick::BothTheirsFirst), Some(s(&["y", "x"])));
        assert_eq!(b[0].lines(Pick::Base), None);
    }

    #[test]
    fn messages_are_read() {
        assert_eq!(merged_name("Merge branch 'feat/x' into main\n"), Some("feat/x".into()));
        assert_eq!(merged_name("Merge remote-tracking branch 'origin/a'"), Some("origin/a".into()));
        assert_eq!(merged_name("Merge commit 'abc1234'"), Some("abc1234".into()));
        assert_eq!(merged_name("feat"), None);
        assert_eq!(prepared("Merge branch 'feat'\n\n# Conflicts:\n#\ta.txt\n#\tb.txt\n"), "Merge branch 'feat'");
        assert_eq!(prepared("one\n\ntwo\n# c\n"), "one\n\ntwo");
    }
}
