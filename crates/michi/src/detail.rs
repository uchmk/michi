//! The detail's state, with nothing of egui in it: the selected commit as it
//! is read in full, its diff cut into the rows the window draws (one column
//! or side by side), and how the detail is shown.

use michi_core::git;
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

/// A file's diff, cut into rows both ways.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Rows {
    pub unified: Vec<Unified>,
    pub paired: Vec<Paired>,
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
        Rows { unified, paired }
    }

    pub fn len(&self, wide: bool) -> usize {
        if wide {
            self.paired.len()
        } else {
            self.unified.len()
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
        Line { kind, old, new, text: text.into(), marks: if kind == Kind::Context { Vec::new() } else { std::iter::once(0..1).collect() } }
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
                counts: if k == 2 { None } else { Some((2, 1)) },
                hunks: if k == 2 { Vec::new() } else { vec![Hunk { header: "@@ -1,3 +1,4 @@ fn main".into(), lines }] },
                eol_only: false,
                cut: k == 3,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use michi_core::show::{Hunk, Line, Status};

    fn line(kind: Kind, text: &str) -> Line {
        Line { kind, old: 0, new: 0, text: text.into(), marks: Vec::new() }
    }

    fn file(lines: &[(Kind, &str)]) -> File {
        let hunk = Hunk { header: "@@ -1 +1 @@".into(), lines: lines.iter().map(|&(k, t)| line(k, t)).collect() };
        File {
            status: Status::Modified,
            path: "a".into(),
            from: None,
            counts: Some((0, 0)),
            hunks: vec![hunk],
            eol_only: false,
            cut: false,
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
        assert_eq!(r.len(true), 7);
        assert_eq!(r.len(false), 8);
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
    fn long_lists_are_cut_short() {
        assert_eq!(listed(12, FILES_SHOWN, FILES_CUT, false), 12);
        assert_eq!(listed(13, FILES_SHOWN, FILES_CUT, false), 10);
        assert_eq!(listed(13, FILES_SHOWN, FILES_CUT, true), 13);
        assert_eq!(listed(9, MERGED_SHOWN, MERGED_CUT, false), 7);
    }
}
