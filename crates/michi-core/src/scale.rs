//! A made-up history of any size and the time budget michi holds itself to on
//! a big one (issue #59). The history is built in memory -- no git, no disk --
//! so the cost measured is michi's own: folding the commits into rows and
//! laying the graph out. The `bench-large` example (michi crate) times it in
//! release mode and CI fails it when it goes over the budget.

use crate::history::{History, Setup};
use crate::log::Commit;
use crate::oid::Oid;
use crate::refs::{Refs, Trunk};

/// Commits in the history the bench builds.
pub const BENCH_COMMITS: usize = 300_000;

/// What michi must do within these times on [`BENCH_COMMITS`] commits, in
/// release mode on a CI runner (several times what a laptop needs, so a
/// noisy machine does not fail it but a quadratic step does).
pub mod budget {
    /// The first screen: the first batch folded into rows and laid out.
    pub const FIRST_SCREEN_MS: u128 = 100;
    /// The whole history folded into rows, in batches as `git log` delivers them.
    pub const ALL_ROWS_MS: u128 = 1_500;
    /// The whole shown history laid out as a graph.
    pub const ALL_LAYOUT_MS: u128 = 1_000;
}

/// The id of the `n`th commit made, oldest first.
fn id(n: usize) -> Oid {
    // `Oid::parse` wants 40 hex digits; spread the number so ids do not sort
    // in the order they were made.
    let x = (n as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0x5555;
    Oid::parse(&format!("{x:016x}{n:024x}")).expect("40 hex digits")
}

/// `n` commits, newest first and every child before its parents (what
/// `git log --date-order` gives): a trunk with a side branch of one to six
/// commits merged into it every fifth step, as a busy repository looks.
/// The result may hold a few more than `n` (a branch and its merge are kept
/// whole).
pub fn synth(n: usize) -> Vec<Commit> {
    // Oldest first; reversed at the end.
    let mut made: Vec<Commit> = Vec::with_capacity(n + 8);
    fn push(made: &mut Vec<Commit>, parents: Vec<Oid>, subject: String) -> Oid {
        let k = made.len();
        let oid = id(k);
        made.push(Commit {
            id: oid,
            parents,
            author: format!("author {}", k % 40),
            email: format!("a{}@example.invalid", k % 40),
            time: 1_767_225_600 + k as i64 * 60,
            subject,
        });
        oid
    }
    let mut trunk = push(&mut made, Vec::new(), "root".into());
    let mut step = 0usize;
    while made.len() < n {
        step += 1;
        if step.is_multiple_of(5) {
            let len = 1 + step / 5 % 6;
            let mut tip = trunk;
            for j in 0..len {
                tip = push(&mut made, vec![tip], format!("feature {step} part {j}"));
            }
            trunk = push(&mut made, vec![trunk, tip], format!("Merge branch 'feat-{}'", step / 5 % 50));
        } else {
            trunk = push(&mut made, vec![trunk], format!("change {step}"));
        }
    }
    made.reverse();
    made
}

/// The setup for a history from [`synth`]: only a trunk named `main`.
pub fn setup_for(commits: &[Commit]) -> Setup {
    let trunk = commits.first().map(|c| Trunk { name: "main".into(), head: c.id, remote: None });
    Setup::new(Refs::default(), Vec::new(), trunk)
}

/// Folds `commits` into a history, `batch` at a time (`git log` is read a
/// batch at a time too).
pub fn fold_in(commits: &[Commit], batch: usize) -> History {
    let mut h = History::new(setup_for(commits));
    for b in commits.chunks(batch.max(1)) {
        h.extend(b.iter().cloned());
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history::Fold;
    use std::collections::HashSet;

    #[test]
    fn a_child_always_comes_before_its_parents() {
        let c = synth(2_000);
        assert!(c.len() >= 2_000);
        let pos: std::collections::HashMap<Oid, usize> = c.iter().enumerate().map(|(i, c)| (c.id, i)).collect();
        assert_eq!(pos.len(), c.len(), "ids are unique");
        for (i, k) in c.iter().enumerate() {
            for p in &k.parents {
                assert!(pos[p] > i, "parent after child");
            }
        }
        assert!(c.iter().any(|k| k.parents.len() > 1), "has merges");
        assert_eq!(c.last().unwrap().parents.len(), 0, "ends at the root");
    }

    #[test]
    fn the_same_size_gives_the_same_history() {
        assert_eq!(synth(500), synth(500));
    }

    #[test]
    fn folding_in_batches_finds_the_trunk_and_hides_the_side_branches() {
        let c = synth(5_000);
        let h = fold_in(&c, 500);
        assert_eq!(h.len(), c.len());
        let fold = Fold { on: true, open: HashSet::new() };
        let shown = (0..h.len() as u32).filter(|&i| h.shown(i, &fold)).count();
        assert!(shown < h.len(), "side branches fold away");
        let (merges, hidden) = h.folded();
        assert!(merges > 0 && hidden as usize == h.len() - shown);
    }
}
