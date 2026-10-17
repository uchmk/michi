//! Git LFS pointers. A file kept in LFS is, to git, a three-line text file
//! (`version`, `oid`, `size`); the diff of it shows that pointer, not the
//! content. This reads one out of a file's diff so the window can say so.
//! Nothing here runs git or touches the disk: fetching is left to `git lfs`.

use crate::show::{File, Kind};

/// The first line of every pointer.
const VERSION: &str = "version https://git-lfs.github.com/spec/v1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pointer {
    /// The hash of the content, as written after `sha256:`.
    pub oid: String,
    /// The bytes of the content.
    pub size: u64,
}

/// The pointer the `lines` of a file make up; `None` when they are not one.
pub fn parse<'a>(lines: impl IntoIterator<Item = &'a str>) -> Option<Pointer> {
    let mut lines = lines.into_iter().map(|l| l.trim_end_matches('\r'));
    if lines.next()? != VERSION {
        return None;
    }
    let (mut oid, mut size) = (None, None);
    for line in lines {
        if let Some(h) = line.strip_prefix("oid sha256:") {
            oid = Some(h).filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()));
        } else if let Some(n) = line.strip_prefix("size ") {
            size = n.parse().ok();
        }
    }
    Some(Pointer { oid: oid?.to_owned(), size: size? })
}

/// The pointer a file's diff shows: the new side's, or the old side's for a
/// file that went away. `None` for any other file, and for a diff that does
/// not reach the pointer's first line.
pub fn of_file(f: &File) -> Option<Pointer> {
    let lines = |keep: fn(Kind) -> bool| f.hunks.iter().flat_map(|h| &h.lines).filter(move |l| keep(l.kind)).map(|l| l.text.as_str());
    parse(lines(|k| k != Kind::Del)).or_else(|| parse(lines(|k| k != Kind::Add)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::show::{Hunk, Line};

    const OID: &str = "4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393";

    fn line(kind: Kind, text: &str) -> Line {
        Line { kind, old: 0, new: 0, text: text.to_owned(), marks: vec![], ink: vec![] }
    }

    fn file(lines: Vec<Line>) -> File {
        File {
            status: crate::show::Status::Modified,
            path: "a.psd".into(),
            from: None,
            blob: None,
            counts: Some((1, 1)),
            hunks: vec![Hunk { header: "@@ -1,3 +1,3 @@".into(), lines }],
            eol_only: false,
            cut: false,
            not_utf8: false,
        }
    }

    #[test]
    fn reads_a_pointer() {
        let size = "size 12345".to_owned();
        let p = parse(["version https://git-lfs.github.com/spec/v1", &format!("oid sha256:{OID}"), &size]).unwrap();
        assert_eq!((p.oid.as_str(), p.size), (OID, 12345));
    }

    #[test]
    fn crlf_is_fine() {
        let oid = format!("oid sha256:{OID}\r");
        assert!(parse(["version https://git-lfs.github.com/spec/v1\r", &oid, "size 1\r"]).is_some());
    }

    #[test]
    fn other_text_is_not_a_pointer() {
        assert_eq!(parse(["hello", "world"]), None);
        assert_eq!(parse([]), None);
        // The version line must come first.
        assert_eq!(parse([&format!("oid sha256:{OID}"), "size 1", "version https://git-lfs.github.com/spec/v1"]), None);
        // Missing size, or a short hash.
        assert_eq!(parse(["version https://git-lfs.github.com/spec/v1", &format!("oid sha256:{OID}")]), None);
        assert_eq!(parse(["version https://git-lfs.github.com/spec/v1", "oid sha256:abc", "size 1"]), None);
        assert_eq!(parse(["version https://git-lfs.github.com/spec/v1", &format!("oid sha256:{OID}"), "size x"]), None);
    }

    #[test]
    fn a_changed_pointer_reads_from_the_new_side() {
        let new = "9".repeat(64);
        let f = file(vec![
            line(Kind::Context, "version https://git-lfs.github.com/spec/v1"),
            line(Kind::Del, &format!("oid sha256:{OID}")),
            line(Kind::Del, "size 1"),
            line(Kind::Add, &format!("oid sha256:{new}")),
            line(Kind::Add, "size 2"),
        ]);
        let p = of_file(&f).unwrap();
        assert_eq!((p.oid, p.size), (new, 2));
    }

    #[test]
    fn a_deleted_pointer_reads_from_the_old_side() {
        let f = file(vec![
            line(Kind::Del, "version https://git-lfs.github.com/spec/v1"),
            line(Kind::Del, &format!("oid sha256:{OID}")),
            line(Kind::Del, "size 7"),
        ]);
        assert_eq!(of_file(&f).map(|p| p.size), Some(7));
    }

    #[test]
    fn a_plain_file_has_none() {
        assert_eq!(of_file(&file(vec![line(Kind::Add, "fn main() {}")])), None);
        assert_eq!(of_file(&file(vec![])), None);
    }
}
