//! The detail's state, with nothing of egui in it: the selected commit as it
//! is read in full, its diff cut into the rows the window draws (one column
//! or side by side), and how the detail is shown.

use michi_core::git;
use michi_core::minimap::Mark;
use michi_core::oid::Oid;
use michi_core::show::{self, File, Kind};

/// The files listed before `… N more` when there are many.
pub const FILES_SHOWN: usize = 10;
/// More files than this are cut to [`FILES_SHOWN`].
pub const FILES_CUT: usize = 12;
/// The commits of a trunk merge listed before `N more` when there are many.
pub const MERGED_SHOWN: usize = 7;
/// More commits than this are cut to [`MERGED_SHOWN`].
pub const MERGED_CUT: usize = 8;

/// A line of a file's diff: its hunk and its place in it.
pub type At = (u32, u32);

/// A row of a diff in one column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unified {
    /// The hunk's `@@` line.
    Hunk(u32),
    Line(At),
}

/// A row of a diff side by side: the old file on the left, the new on the
/// right. An unchanged line is on both; a deleted line is paired with the
/// added line in its place, else with nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paired {
    Hunk(u32),
    Pair(Option<At>, Option<At>),
}

/// A line's text's width: narrow columns and wide characters.
pub type Cols = (u32, u32);

/// A file's diff, cut into rows both ways.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rows {
    pub unified: Vec<Unified>,
    pub paired: Vec<Paired>,
    /// The columns each row's text takes (0 for a hunk's line, which is
    /// cut, not wrapped), in one column and side by side (the wider half).
    unified_cols: Vec<Cols>,
    paired_cols: Vec<[Cols; 2]>,
}

impl Rows {
    pub fn of(file: &File) -> Rows {
        let mut unified = Vec::new();
        let mut paired = Vec::new();
        for (h, hunk) in file.hunks.iter().enumerate() {
            let h = h as u32;
            unified.push(Unified::Hunk(h));
            paired.push(Paired::Hunk(h));
            let (mut dels, mut adds): (Vec<At>, Vec<At>) = (Vec::new(), Vec::new());
            let flush = |dels: &mut Vec<At>, adds: &mut Vec<At>, paired: &mut Vec<Paired>| {
                for k in 0..dels.len().max(adds.len()) {
                    paired.push(Paired::Pair(dels.get(k).copied(), adds.get(k).copied()));
                }
                dels.clear();
                adds.clear();
            };
            for (l, line) in hunk.lines.iter().enumerate() {
                let at = (h, l as u32);
                unified.push(Unified::Line(at));
                match line.kind {
                    Kind::Del => {
                        // A deletion after additions starts the next pairing.
                        if !adds.is_empty() {
                            flush(&mut dels, &mut adds, &mut paired);
                        }
                        dels.push(at);
                    }
                    Kind::Add => adds.push(at),
                    Kind::Context => {
                        flush(&mut dels, &mut adds, &mut paired);
                        paired.push(Paired::Pair(Some(at), Some(at)));
                    }
                }
            }
            flush(&mut dels, &mut adds, &mut paired);
        }
        let cols = |at: Option<At>| {
            at.map_or((0, 0), |(h, l)| {
                let (n, w) = file.hunks[h as usize].lines[l as usize].columns();
                (n as u32, w as u32)
            })
        };
        let unified_cols = unified
            .iter()
            .map(|r| match *r {
                Unified::Hunk(_) => (0, 0),
                Unified::Line(at) => cols(Some(at)),
            })
            .collect();
        let paired_cols = paired
            .iter()
            .map(|r| match *r {
                Paired::Hunk(_) => [(0, 0); 2],
                Paired::Pair(old, new) => [cols(old), cols(new)],
            })
            .collect();
        Rows { unified, paired, unified_cols, paired_cols }
    }

    /// How many lines of text each row takes, `lines` telling it for a
    /// line's text (from its `Line::columns`); side by side, a row takes its
    /// wider half's.
    pub fn wraps<'a>(&'a self, wide: bool, lines: impl Fn(Cols) -> u32 + 'a) -> Box<dyn Iterator<Item = u32> + 'a> {
        if wide {
            Box::new(self.paired_cols.iter().map(move |&[a, b]| lines(a).max(lines(b)).max(1)))
        } else {
            Box::new(self.unified_cols.iter().map(move |&c| lines(c).max(1)))
        }
    }

    /// What each row holds, for the minimap: a changed line is `Add` or
    /// `Del`; side by side, a deletion paired with an addition is `Both`.
    pub fn marks(&self, wide: bool, file: &File) -> Vec<Mark> {
        let kind = |(h, l): At| match file.hunks[h as usize].lines[l as usize].kind {
            Kind::Add => Mark::Add,
            Kind::Del => Mark::Del,
            Kind::Context => Mark::Same,
        };
        if wide {
            self.paired
                .iter()
                .map(|r| match *r {
                    Paired::Hunk(_) => Mark::Same,
                    Paired::Pair(Some(a), Some(b)) if a == b => Mark::Same,
                    Paired::Pair(Some(_), Some(_)) => Mark::Both,
                    Paired::Pair(a, b) => a.or(b).map_or(Mark::Same, kind),
                })
                .collect()
        } else {
            self.unified
                .iter()
                .map(|r| match *r {
                    Unified::Hunk(_) => Mark::Same,
                    Unified::Line(at) => kind(at),
                })
                .collect()
        }
    }
}

/// What the detail shows and how.
#[derive(Default)]
pub struct Pane {
    /// The commit asked for: the selection when it was made.
    asked: Option<Oid>,
    /// Its answer, once it came.
    got: Option<Result<Box<show::Detail>, git::Error>>,
    /// Each file's diff in rows.
    rows: Vec<Rows>,
    /// The commits the selected trunk merge brought in, as of how many
    /// commits were read.
    merged: Option<(Oid, usize, Vec<Oid>)>,
    /// The diff side by side, the graph narrowed (`Enter`).
    pub wide: bool,
    /// Every file in the list, not the first [`FILES_SHOWN`].
    pub all_files: bool,
    /// The file whose diff to scroll to.
    pub scroll_to: Option<usize>,
    /// The detail is of another commit now: back to its top.
    pub to_top: bool,
}

impl Pane {
    /// The selection is now `id`: whether that is a change (and so the
    /// commit is to be read).
    pub fn select(&mut self, id: Option<Oid>) -> bool {
        if id == self.asked {
            return false;
        }
        self.asked = id;
        self.got = None;
        self.rows.clear();
        self.all_files = false;
        self.scroll_to = None;
        self.to_top = true;
        true
    }

    /// The selection is now `id`, the same thing read anew (a worktree's
    /// uncommitted change that changed): what is shown stays, in its place,
    /// until the new answer comes.
    pub fn refresh(&mut self, id: Option<Oid>) -> bool {
        if id == self.asked {
            return false;
        }
        self.asked = id;
        true
    }

    /// What is shown stays, in its place, but the commit is read again the
    /// next time it is asked for (the diff is to be read another way).
    pub fn forget(&mut self) {
        self.asked = None;
    }

    pub fn asked(&self) -> Option<Oid> {
        self.asked
    }

    /// The answer for the commit `id`; one for a commit no longer selected
    /// is dropped.
    pub fn take(&mut self, id: Oid, result: Result<Box<show::Detail>, git::Error>) {
        if self.asked != Some(id) {
            return;
        }
        self.rows = result.as_ref().map(|d| d.files.iter().map(Rows::of).collect()).unwrap_or_default();
        if self.scroll_to.is_some_and(|f| f >= self.rows.len()) {
            self.scroll_to = None;
        }
        self.got = Some(result);
    }

    /// The selected commit in full, or why not; `None` while it is read.
    pub fn got(&self) -> Option<&Result<Box<show::Detail>, git::Error>> {
        self.got.as_ref()
    }

    pub fn rows(&self, file: usize) -> Option<&Rows> {
        self.rows.get(file)
    }

    /// The commits the trunk merge `id` brought in, if counted for `read`
    /// commits.
    pub fn merged(&self, id: Oid, read: usize) -> Option<&[Oid]> {
        self.merged.as_ref().filter(|(m, r, _)| *m == id && *r == read).map(|(_, _, c)| c.as_slice())
    }

    pub fn set_merged(&mut self, id: Oid, read: usize, commits: Vec<Oid>) {
        self.merged = Some((id, read, commits));
    }
}

/// How many of `n` to list, the rest summed up after them: all of them, or
/// `shown` when there are more than `cut`.
pub fn listed(n: usize, shown: usize, cut: usize, all: bool) -> usize {
    if all || n <= cut {
        n
    } else {
        shown
    }
}

/// A commit in full for the tests: `files` files, the second renamed, the
/// third binary, the fourth cut short.
#[cfg(test)]
pub mod fixture {
    use michi_core::oid::Oid;
    use michi_core::show::{Detail, File, Hunk, Kind, Line, Status};

    fn line(kind: Kind, old: u32, new: u32, text: &str) -> Line {
        Line {
            kind,
            old,
            new,
            text: text.into(),
            marks: if kind == Kind::Context { Vec::new() } else { std::iter::once(0..1).collect() },
            ink: Vec::new(),
        }
    }

    pub fn detail(id: Oid, parents: Vec<Oid>, files: usize) -> Detail {
        let file = |k: usize| {
            let lines = vec![
                line(Kind::Context, 1, 1, "fn main() {"),
                line(Kind::Del, 2, 0, "    old();"),
                line(Kind::Add, 0, 2, "    new();"),
                line(Kind::Add, 0, 3, "\tmore();"),
                line(Kind::Context, 3, 4, "}"),
            ];
            File {
                status: if k == 1 { Status::Renamed } else { Status::Modified },
                path: format!("crates/some/long/folder/file{k}.rs"),
                from: (k == 1).then(|| "old/name.rs".to_owned()),
                blob: None,
                counts: if k == 2 { None } else { Some((2, 1)) },
                hunks: if k == 2 { Vec::new() } else { vec![Hunk { header: "@@ -1,3 +1,4 @@ fn main".into(), lines }] },
                eol_only: false,
                cut: k == 3,
                not_utf8: k == 0,
            }
        };
        Detail {
            id,
            parents,
            author: "dependabot[bot]".into(),
            email: "x@y".into(),
            time: 1_767_225_600,
            subject: "A subject long enough to wrap in a narrow detail, as some are".into(),
            body: "The body.\n\nIts second paragraph, which goes on for a while so that it wraps.".into(),
            files: (0..files).map(file).collect(),
            cut: false,
            signature: michi_core::signature::Signature::Valid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use michi_core::show::{Hunk, Line, Status};

    fn line(kind: Kind, text: &str) -> Line {
        Line { kind, old: 0, new: 0, text: text.into(), marks: Vec::new(), ink: Vec::new() }
    }

    fn file(lines: &[(Kind, &str)]) -> File {
        let hunk = Hunk { header: "@@ -1 +1 @@".into(), lines: lines.iter().map(|&(k, t)| line(k, t)).collect() };
        File {
            status: Status::Modified,
            path: "a".into(),
            from: None,
            blob: None,
            counts: Some((0, 0)),
            hunks: vec![hunk],
            eol_only: false,
            cut: false,
            not_utf8: false,
        }
    }

    #[test]
    fn deletions_pair_with_the_additions_in_their_place() {
        use Kind::*;
        let f = file(&[(Context, "a"), (Del, "b"), (Del, "c"), (Add, "B"), (Context, "d"), (Add, "e"), (Del, "f")]);
        let r = Rows::of(&f);
        assert_eq!(r.unified.len(), 8);
        assert_eq!(r.unified[0], Unified::Hunk(0));
        assert_eq!(r.unified[7], Unified::Line((0, 6)));
        let p = |a: Option<u32>, b: Option<u32>| Paired::Pair(a.map(|l| (0, l)), b.map(|l| (0, l)));
        assert_eq!(
            r.paired,
            [
                Paired::Hunk(0),
                p(Some(0), Some(0)),
                p(Some(1), Some(3)),
                p(Some(2), None),
                p(Some(4), Some(4)),
                // An addition then a deletion: each on its own row.
                p(None, Some(5)),
                p(Some(6), None),
            ]
        );
        assert_eq!(r.wraps(true, |_| 1).count(), 7);
        assert_eq!(r.wraps(false, |_| 1).count(), 8);
    }

    #[test]
    fn rows_are_marked_for_the_minimap_both_ways() {
        use Kind::*;
        let f = file(&[(Context, "a"), (Del, "b"), (Add, "B"), (Context, "d"), (Add, "e")]);
        let r = Rows::of(&f);
        assert_eq!(r.marks(false, &f), [Mark::Same, Mark::Same, Mark::Del, Mark::Add, Mark::Same, Mark::Add]);
        assert_eq!(r.marks(true, &f), [Mark::Same, Mark::Same, Mark::Both, Mark::Same, Mark::Add]);
    }

    #[test]
    fn long_lines_wrap_at_the_width() {
        use Kind::*;
        let long = "x".repeat(25);
        let f = file(&[(Del, "short"), (Add, &long), (Context, "日本語の行")]);
        let r = Rows::of(&f);
        // A wide character two columns here: 25 columns in 10 is 3 lines.
        let per = |n: u32| move |(c, w): Cols| (c + 2 * w).div_ceil(n);
        assert_eq!(r.wraps(false, per(10)).collect::<Vec<_>>(), [1, 1, 3, 1]);
        assert_eq!(r.wraps(false, per(4)).collect::<Vec<_>>(), [1, 2, 7, 3]);
        // Side by side, a pair takes the wider half's lines.
        assert_eq!(r.wraps(true, per(10)).collect::<Vec<_>>(), [1, 3, 1]);
        assert_eq!(r.wraps(true, per(1)).collect::<Vec<_>>(), [1, 25, 10]);
    }

    #[test]
    fn answers_for_another_commit_are_dropped() {
        let (a, b) = (Oid::parse(&"a".repeat(40)).unwrap(), Oid::parse(&"b".repeat(40)).unwrap());
        let mut d = Pane::default();
        assert!(d.select(Some(a)));
        assert!(!d.select(Some(a)));
        d.take(b, Err(git::Error::NotRepo));
        assert!(d.got().is_none());
        d.take(a, Err(git::Error::NotRepo));
        assert!(d.got().is_some());
        d.all_files = true;
        assert!(d.select(Some(b)));
        assert!(d.got().is_none() && !d.all_files && d.to_top);
    }

    #[test]
    fn a_refresh_keeps_what_is_shown_until_the_answer() {
        let (a, b) = (Oid::parse(&"a".repeat(40)).unwrap(), Oid::parse(&"b".repeat(40)).unwrap());
        let mut d = Pane::default();
        d.select(Some(a));
        d.take(a, Ok(Box::new(fixture::detail(a, Vec::new(), 3))));
        d.to_top = false;
        d.scroll_to = Some(2);
        assert!(d.refresh(Some(b)));
        assert!(!d.refresh(Some(b)));
        assert!(d.got().is_some() && d.rows(2).is_some() && !d.to_top);
        // The new answer has fewer files: the scroll to one gone is dropped.
        d.take(b, Ok(Box::new(fixture::detail(b, Vec::new(), 1))));
        assert_eq!(d.got().unwrap().as_ref().unwrap().id, b);
        assert_eq!(d.scroll_to, None);
    }

    #[test]
    fn long_lists_are_cut_short() {
        assert_eq!(listed(12, FILES_SHOWN, FILES_CUT, false), 12);
        assert_eq!(listed(13, FILES_SHOWN, FILES_CUT, false), 10);
        assert_eq!(listed(13, FILES_SHOWN, FILES_CUT, true), 13);
        assert_eq!(listed(9, MERGED_SHOWN, MERGED_CUT, false), 7);
    }
}
