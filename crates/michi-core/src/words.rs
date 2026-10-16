//! Which words of a side's lines differ from a reference: the base, or the
//! other side when there is no base. Byte ranges, so a view can paint them.

use std::ops::Range;

use similar::{capture_diff_slices, Algorithm, ChangeTag, DiffOp, TextDiff};

/// For each line of `side`, the byte ranges of the words that are not in the
/// line of `reference` it replaces. A line with no partner is marked whole
/// (an empty line has no range); lines both have in common are not marked.
pub fn marks(reference: &[String], side: &[String]) -> Vec<Vec<Range<usize>>> {
    let mut out = vec![Vec::new(); side.len()];
    for op in capture_diff_slices(Algorithm::Myers, reference, side) {
        match op {
            DiffOp::Equal { .. } | DiffOp::Delete { .. } => {}
            DiffOp::Insert { new_index, new_len, .. } => {
                for q in new_index..new_index + new_len {
                    out[q] = whole(&side[q]);
                }
            }
            DiffOp::Replace { old_index, old_len, new_index, new_len } => {
                for k in 0..new_len {
                    let line = &side[new_index + k];
                    out[new_index + k] = if k < old_len { words(&reference[old_index + k], line) } else { whole(line) };
                }
            }
        }
    }
    out
}

fn whole(line: &str) -> Vec<Range<usize>> {
    if line.is_empty() {
        Vec::new()
    } else {
        std::iter::once(0..line.len()).collect()
    }
}

/// The words of `new` that `old` has not, neighbours joined.
fn words(old: &str, new: &str) -> Vec<Range<usize>> {
    let diff = TextDiff::configure().algorithm(Algorithm::Myers).diff_unicode_words(old, new);
    let mut out: Vec<Range<usize>> = Vec::new();
    let mut at = 0;
    for change in diff.iter_all_changes() {
        let len = change.value().len();
        match change.tag() {
            ChangeTag::Delete => continue,
            ChangeTag::Insert => match out.last_mut() {
                Some(last) if last.end == at => last.end = at + len,
                _ => out.push(at..at + len),
            },
            ChangeTag::Equal => {}
        }
        at += len;
    }
    // Spaces between two marked words are marked too, a space at an edge is not.
    out.retain(|r| !new[r.clone()].trim().is_empty());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|l| (*l).to_owned()).collect()
    }

    #[test]
    fn only_the_changed_words_of_a_line_are_marked() {
        let m = marks(&s(&["let a = 1;", "same"]), &s(&["let a = 22;", "same"]));
        assert_eq!(&"let a = 22;"[m[0][0].clone()], "22");
        assert_eq!(m[0].len(), 1);
        assert!(m[1].is_empty());
    }

    #[test]
    fn a_line_with_no_partner_is_marked_whole_and_an_empty_one_not_at_all() {
        let m = marks(&s(&["a"]), &s(&["a", "new line", ""]));
        assert!(m[0].is_empty());
        assert_eq!((m[1][0].start, m[1][0].end), (0, 8));
        assert!(m[2].is_empty());
    }

    #[test]
    fn nothing_to_compare_with_marks_every_line() {
        let m = marks(&[], &s(&["x y"]));
        assert_eq!(m.len(), 1);
        assert_eq!((m[0][0].start, m[0][0].end), (0, 3));
    }
}
