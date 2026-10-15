//! What michi knows about Git, with nothing of egui in it: running git, the refs,
//! the history and what the graph makes of it, the worktrees, the paths. The
//! window is the `michi` crate.

pub mod git;
pub mod history;
pub mod log;
pub mod oid;
pub mod paths;
pub mod refs;
pub mod summary;
pub mod worktree;

#[cfg(test)]
pub(crate) mod testrepo;
