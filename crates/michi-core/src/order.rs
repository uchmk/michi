//! A suggested order to land the worktrees on the trunk: the one that leaves
//! the fewest places to resolve by hand, worked out from the predictions
//! already made ([`Clashes`]). No git is run here.
//!
//! Every two worktrees that conflict cost the same whichever lands first; what
//! the order decides is who pays: the one landing later resolves against the
//! one landed before. So the worktrees that conflict with nothing land first
//! (they never stop), then, one at a time, the one that has the fewest places
//! to resolve against those landed already and leaves the fewest to those
//! still waiting. Of two as cheap, the one that conflicts with the trunk
//! itself goes later, as it needs the trunk taken in first anyway.

use std::path::PathBuf;

use crate::clash::{Clashes, Outcome};
use crate::track::Trees;

/// One worktree in the suggested order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub path: PathBuf,
    /// Places to resolve against the worktrees landed before it; 0 when it
    /// goes in clean.
    pub resolve: u32,
}

/// Places a committed conflict has; an uncommitted-only or no conflict has 0
/// (what is not committed cannot land).
fn places(o: Option<&Outcome>) -> u32 {
    match o {
        Some(Outcome::Committed(c)) => c.places(),
        _ => 0,
    }
}

/// The order to land the worktrees that have commits the trunk has not (the
/// trunk's own worktree left out). Empty with fewer than two: one has no
/// order to speak of.
pub fn suggest(clashes: &Clashes, trees: &Trees) -> Vec<Step> {
    let trunk = trees.trunk.as_deref();
    // (side index, path, commits ahead of the trunk)
    let mut left: Vec<(usize, PathBuf, u32)> = trees
        .trees
        .iter()
        .filter(|t| t.wt.branch.is_some() && t.wt.branch.as_deref() != trunk)
        // Merged into another worktree's branch: it lands with that one.
        .filter(|t| t.merged_into.is_none())
        .filter_map(|t| {
            let ahead = t.ahead_behind.map(|(a, _)| a).filter(|&a| a > 0)?;
            Some((clashes.side_of(&t.wt.path)?, t.wt.path.clone(), ahead))
        })
        .collect();
    if left.len() < 2 {
        return Vec::new();
    }
    // The side the trunk is: its worktree's, or the stand-in in front.
    let trunk_side = clashes.sides.iter().position(|s| Some(s.name.as_str()) == trunk);
    let mut done: Vec<usize> = Vec::new();
    let mut out = Vec::new();
    while !left.is_empty() {
        let against = |i: usize| -> u32 { done.iter().map(|&d| places(clashes.between(i, d))).sum() };
        let best = (0..left.len())
            .min_by_key(|&k| {
                let i = left[k].0;
                let leaves: u32 = left.iter().filter(|r| r.0 != i).map(|r| places(clashes.between(i, r.0))).sum();
                let with_trunk = trunk_side.map_or(0, |t| places(clashes.between(i, t)));
                (against(i), leaves, with_trunk, left[k].2, k)
            })
            .unwrap_or(0);
        let (side, path, _) = left.remove(best);
        let resolve = against(side);
        done.push(side);
        out.push(Step { path, resolve });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clash::Side;
    use crate::oid::Oid;
    use crate::predict::{Conflict, ConflictFile};
    use crate::track::Tree;
    use crate::worktree::Worktree;

    fn oid() -> Oid {
        Oid::parse(&"a".repeat(40)).unwrap()
    }

    fn conflict(n: u32) -> Outcome {
        Outcome::Committed(Conflict { files: vec![ConflictFile { path: "f".into(), hunks: Some(n) }] })
    }

    fn tree(name: &str, ahead: u32) -> Tree {
        let wt = Worktree {
            path: PathBuf::from(format!("/r/{name}")),
            head: "a".repeat(40),
            branch: Some(name.to_owned()),
            bare: false,
            detached: false,
            locked: None,
            prunable: None,
        };
        Tree { wt, ahead_behind: Some((ahead, 0)), upstream: None, doing: None, merged_into: None }
    }

    /// `names` are the worktrees (the first is main), `conflicts` the pairs of
    /// indexes with their places.
    fn setup(names: &[&str], conflicts: &[(usize, usize, u32)]) -> (Clashes, Trees) {
        let sides: Vec<Side> = names
            .iter()
            .map(|n| Side { name: (*n).to_owned(), path: Some(PathBuf::from(format!("/r/{n}"))), head: oid(), tip: oid() })
            .collect();
        let n = sides.len();
        let mut pairs = Vec::new();
        for i in 0..n {
            for j in i + 1..n {
                pairs.push(conflicts.iter().find(|c| c.0 == i && c.1 == j).map_or(Outcome::Clean, |c| conflict(c.2)));
            }
        }
        let trees = Trees {
            trunk: Some("main".into()),
            trunk_head: Some(oid()),
            trees: names.iter().enumerate().map(|(i, n)| tree(n, if i == 0 { 0 } else { 1 })).collect(),
        };
        (Clashes { sides, pairs, branches: Vec::new() }, trees)
    }

    fn names(steps: &[Step]) -> Vec<String> {
        steps.iter().map(|s| s.path.file_name().unwrap().to_string_lossy().into_owned()).collect()
    }

    #[test]
    fn fewer_than_two_gives_no_order() {
        let (c, t) = setup(&["main", "a"], &[]);
        assert!(suggest(&c, &t).is_empty());
    }

    #[test]
    fn independent_ones_go_first() {
        // a and b conflict; c conflicts with nothing.
        let (c, t) = setup(&["main", "a", "b", "c"], &[(1, 2, 4)]);
        let steps = suggest(&c, &t);
        assert_eq!(names(&steps)[0], "c");
        assert_eq!(steps.iter().map(|s| s.resolve).collect::<Vec<_>>(), vec![0, 0, 4]);
    }

    #[test]
    fn the_one_that_hurts_the_fewest_goes_before() {
        // b conflicts with a (2) and c (5); a with c (1).
        let (c, t) = setup(&["main", "a", "b", "c"], &[(1, 2, 2), (2, 3, 5), (1, 3, 1)]);
        let steps = suggest(&c, &t);
        assert_eq!(names(&steps), vec!["a", "c", "b"]);
        assert_eq!(steps.iter().map(|s| s.resolve).collect::<Vec<_>>(), vec![0, 1, 7]);
    }

    #[test]
    fn nothing_ahead_and_the_trunk_are_left_out() {
        let (c, mut t) = setup(&["main", "a", "b", "c"], &[]);
        t.trees[3].ahead_behind = Some((0, 2));
        assert_eq!(names(&suggest(&c, &t)).len(), 2);
    }

    #[test]
    fn one_merged_into_another_lands_with_it() {
        let (c, mut t) = setup(&["main", "a", "b", "c"], &[]);
        t.trees[1].merged_into = Some("b".into());
        assert_eq!(names(&suggest(&c, &t)), vec!["b", "c"]);
    }

    #[test]
    fn uncommitted_conflicts_do_not_count() {
        let (mut c, t) = setup(&["main", "a", "b"], &[]);
        c.pairs[2] = Outcome::Uncommitted(Conflict { files: vec![ConflictFile { path: "f".into(), hunks: Some(3) }] });
        assert!(suggest(&c, &t).iter().all(|s| s.resolve == 0));
    }
}
