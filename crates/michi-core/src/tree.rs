//! The changed files as a tree of folders (docs/design.md: 変更): the rows the
//! `2` view draws when it shows folders instead of a flat list, and the
//! paths a folder stands for when it is staged or unstaged whole.
//!
//! A folder holding only one folder and no file is joined with it (`a/b/c`
//! is one row), as editors show the folders of a source tree. Folders come
//! before files, each in name order.

use std::collections::BTreeMap;

use crate::show::File;

/// A row of the tree, top to bottom.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    Dir {
        /// From the top, with `/` between the names.
        path: String,
        /// What the row shows: the names of the folders joined into it.
        name: String,
        depth: usize,
        /// How many files are below it, however deep.
        files: usize,
        folded: bool,
    },
    File {
        /// Its place in the files given to [`rows`].
        index: usize,
        name: String,
        depth: usize,
    },
}

#[derive(Default)]
struct Node {
    dirs: BTreeMap<String, Node>,
    files: Vec<(String, usize)>,
    count: usize,
}

/// The rows for `files`, without those below a folder `folded` says is
/// folded (given its path).
pub fn rows(files: &[File], folded: impl Fn(&str) -> bool) -> Vec<Row> {
    build(files, folded, false)
}

/// Like [`rows`], but folders and files share one order by name, as the
/// flat list of a commit's files (in path order) has them.
pub fn rows_by_name(files: &[File], folded: impl Fn(&str) -> bool) -> Vec<Row> {
    build(files, folded, true)
}

fn build(files: &[File], folded: impl Fn(&str) -> bool, mixed: bool) -> Vec<Row> {
    let mut root = Node::default();
    for (index, f) in files.iter().enumerate() {
        let mut node = &mut root;
        let mut names = f.path.split('/').filter(|n| !n.is_empty()).peekable();
        while let Some(name) = names.next() {
            node.count += 1;
            if names.peek().is_some() {
                node = node.dirs.entry(name.to_owned()).or_default();
            } else {
                node.files.push((name.to_owned(), index));
            }
        }
    }
    let mut out = Vec::new();
    emit(&root, "", 0, &folded, mixed, &mut out);
    out
}

fn emit(node: &Node, base: &str, depth: usize, folded: &impl Fn(&str) -> bool, mixed: bool, out: &mut Vec<Row>) {
    let mut dirs = Vec::new();
    for (name, child) in &node.dirs {
        let (mut name, mut child) = (name.clone(), child);
        while child.files.is_empty() && child.dirs.len() == 1 {
            let (n, c) = child.dirs.iter().next().expect("one folder");
            name = format!("{name}/{n}");
            child = c;
        }
        dirs.push((name, child));
    }
    let mut files = node.files.clone();
    files.sort();
    let mut dirs = dirs.into_iter().peekable();
    let mut files = files.into_iter().peekable();
    loop {
        let dir_first = match (dirs.peek(), files.peek()) {
            (Some((d, _)), Some((f, _))) => !mixed || d <= f,
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => break,
        };
        if dir_first {
            let (name, child) = dirs.next().expect("a folder");
            let path = if base.is_empty() { name.clone() } else { format!("{base}/{name}") };
            let is_folded = folded(&path);
            out.push(Row::Dir { path: path.clone(), name, depth, files: child.count, folded: is_folded });
            if !is_folded {
                emit(child, &path, depth + 1, folded, mixed, out);
            }
        } else {
            let (name, index) = files.next().expect("a file");
            out.push(Row::File { index, name, depth });
        }
    }
}

/// The files shown, in the order the rows have them.
pub fn visible(rows: &[Row]) -> Vec<usize> {
    rows.iter().filter_map(|r| if let Row::File { index, .. } = r { Some(*index) } else { None }).collect()
}

/// The path of every folder of `files`, however deep (what folding all
/// folds).
pub fn dir_paths(files: &[File]) -> Vec<String> {
    rows(files, |_| false)
        .into_iter()
        .filter_map(|r| match r {
            Row::Dir { path, .. } => Some(path),
            Row::File { .. } => None,
        })
        .collect()
}

/// The row of the folder `rows[i]` is in, if it is in one.
pub fn parent_of(rows: &[Row], i: usize) -> Option<usize> {
    let depth = match rows.get(i)? {
        Row::Dir { depth, .. } | Row::File { depth, .. } => *depth,
    };
    (0..i).rev().find(|j| matches!(&rows[*j], Row::Dir { depth: d, .. } if *d + 1 == depth))
}

/// The paths of the files below the folder `dir`, with those a renamed one
/// came from when `with_from` (taking a rename out of the index takes both).
pub fn under(files: &[File], dir: &str, with_from: bool) -> Vec<String> {
    let prefix = format!("{dir}/");
    let mut out = Vec::new();
    for f in files.iter().filter(|f| f.path.starts_with(&prefix)) {
        out.push(f.path.clone());
        if with_from {
            out.extend(f.from.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::show::Status;

    fn file(path: &str, from: Option<&str>) -> File {
        File {
            status: Status::Modified,
            path: path.into(),
            from: from.map(Into::into),
            blob: None,
            counts: Some((1, 0)),
            hunks: Vec::new(),
            eol_only: false,
            cut: false,
            not_utf8: false,
        }
    }

    #[test]
    fn by_name_mixes_folders_and_files() {
        let files = vec![file("src/app.rs", None), file("colors.txt", None), file("docs/g.md", None), file("names.txt", None)];
        let names: Vec<_> = rows_by_name(&files, |_| true)
            .iter()
            .map(|r| match r {
                Row::Dir { name, .. } | Row::File { name, .. } => name.clone(),
            })
            .collect();
        assert_eq!(names, ["colors.txt", "docs", "names.txt", "src"]);
    }

    fn shape(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|r| match r {
                Row::Dir { name, depth, files, folded, .. } => {
                    format!("{}{name}/ {files}{}", " ".repeat(*depth), if *folded { " (folded)" } else { "" })
                }
                Row::File { name, depth, index } => format!("{}{name} #{index}", " ".repeat(*depth)),
            })
            .collect()
    }

    fn sample() -> Vec<File> {
        vec![
            file("src/a/b/x.rs", None),
            file("README.md", None),
            file("src/a/b/y.rs", None),
            file("src/lib.rs", None),
            file("docs/one/two/z.md", None),
        ]
    }

    #[test]
    fn folders_come_first_and_a_lone_folder_chain_is_one_row() {
        let r = rows(&sample(), |_| false);
        assert_eq!(shape(&r), ["docs/one/two/ 1", " z.md #4", "src/ 3", " a/b/ 2", "  x.rs #0", "  y.rs #2", " lib.rs #3", "README.md #1"]);
        assert_eq!(visible(&r), [4, 0, 2, 3, 1]);
    }

    #[test]
    fn a_folded_folder_hides_what_is_below_but_counts_it() {
        let r = rows(&sample(), |p| p == "src");
        assert_eq!(shape(&r), ["docs/one/two/ 1", " z.md #4", "src/ 3 (folded)", "README.md #1"]);
        let r = rows(&sample(), |p| p == "src/a/b");
        assert!(shape(&r).contains(&" a/b/ 2 (folded)".to_string()));
        assert_eq!(visible(&r), [4, 3, 1]);
    }

    #[test]
    fn a_folder_stands_for_every_file_below_it() {
        let files = vec![file("src/new.rs", Some("old/was.rs")), file("src/a/b.rs", None), file("srcx/c.rs", None)];
        assert_eq!(under(&files, "src", false), ["src/new.rs", "src/a/b.rs"]);
        assert_eq!(under(&files, "src", true), ["src/new.rs", "old/was.rs", "src/a/b.rs"]);
        assert!(under(&files, "sr", true).is_empty());
    }

    #[test]
    fn every_folder_is_named_and_a_row_knows_its_parent() {
        assert_eq!(dir_paths(&sample()), ["docs/one/two", "src", "src/a/b"]);
        let r = rows(&sample(), |_| false);
        assert_eq!(parent_of(&r, 1), Some(0));
        assert_eq!(parent_of(&r, 3), Some(2));
        assert_eq!(parent_of(&r, 4), Some(3));
        assert_eq!(parent_of(&r, 7), None);
        assert_eq!(parent_of(&r, 99), None);
    }

    #[test]
    fn no_files_no_rows() {
        assert!(rows(&[], |_| false).is_empty());
    }
}
