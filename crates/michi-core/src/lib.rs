//! What michi knows about Git, with nothing of egui in it: running git, the
//! worktrees, the paths. The window is the `michi` crate.

pub mod git;
pub mod paths;
pub mod summary;
pub mod worktree;

#[cfg(test)]
pub(crate) mod testrepo;
