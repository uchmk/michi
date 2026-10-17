//! Times michi's own work on a very large history (issue #59) and exits 1
//! when it is over the budget in `michi_core::scale::budget`.
//!
//!     cargo run --release -p michi --example bench-large [-- <commits>]
//!
//! The history is made up in memory (no git, no disk), so the numbers are
//! michi's folding and graph layout alone. Always run it with `--release`.

use std::collections::HashSet;
use std::time::Instant;

use ito_graph::{Colors, Dot, Layout, Row};
use michi_core::history::{Fold, History};
use michi_core::oid::Oid;
use michi_core::scale::{self, budget};

/// The first batch `git log` gives.
const FIRST_BATCH: usize = 500;
/// The batches after it.
const BATCH: usize = 2_000;

fn main() {
    let n = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(scale::BENCH_COMMITS);
    let commits = scale::synth(n);
    println!("{} commits", commits.len());
    let fold = Fold { on: true, open: HashSet::new() };
    let mut failed = false;
    let mut check = |what: &str, ms: u128, limit: u128| {
        let ok = ms <= limit;
        println!("{what:<14} {ms:>6} ms (budget {limit} ms) {}", if ok { "ok" } else { "OVER" });
        failed |= !ok;
    };

    // The first screen: the first batch folded and laid out.
    let t = Instant::now();
    let mut h = History::new(scale::setup_for(&commits));
    h.extend(commits[..FIRST_BATCH.min(commits.len())].iter().cloned());
    let mut layout = Layout::new();
    let mut colors = Colors::default();
    lay(&h, &fold, &mut layout, &mut colors, 0);
    check("first screen", t.elapsed().as_millis(), budget::FIRST_SCREEN_MS);

    // All the rows, batch by batch.
    let t = Instant::now();
    for b in commits[FIRST_BATCH.min(commits.len())..].chunks(BATCH) {
        h.extend(b.iter().cloned());
    }
    check("all rows", t.elapsed().as_millis(), budget::ALL_ROWS_MS);

    // The whole shown history laid out (what is laid out already is kept).
    let t = Instant::now();
    let done = layout.len();
    lay(&h, &fold, &mut layout, &mut colors, done);
    layout.finish();
    check("all layout", t.elapsed().as_millis(), budget::ALL_LAYOUT_MS);
    println!("{} rows, {} laid out", h.len(), layout.len());

    if failed {
        eprintln!("over budget");
        std::process::exit(1);
    }
}

/// Lays out the shown rows from the `from`th one on.
fn lay(h: &History, fold: &Fold, layout: &mut Layout<Oid>, colors: &mut Colors, from: usize) {
    let shown = (0..h.len() as u32).filter(|&i| h.shown(i, fold)).skip(from);
    for i in shown {
        let r = &h.rows()[i as usize];
        let parents = h.parents_shown(i, fold);
        let dot = if r.commit.parents.len() > 1 { Dot::Merge } else { Dot::Commit };
        layout.push(Row { id: r.commit.id, parents: &parents, name: r.name.as_deref(), trunk: r.main, dot }, colors);
    }
}
