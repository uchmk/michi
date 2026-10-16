//! What michi knows about Git, with nothing of egui in it: running git, the refs,
//! the history and what the graph makes of it, the worktrees, the paths, the
//! Claude sessions tsumugi lists, the conflict prediction, the uncommitted
//! changes. The window is the `michi` crate.

pub mod backup;
pub mod branch;
pub mod change;
pub mod clash;
pub mod claude;
pub mod discard;
pub mod git;
pub mod history;
pub mod log;
pub mod net;
pub mod oid;
pub mod paths;
pub mod predict;
pub mod refs;
pub mod revert;
pub mod review;
pub mod show;
pub mod stage;
pub mod stash;
pub mod summary;
pub mod tag;
pub mod tidy;
pub mod track;
pub mod unseen;
pub mod wip;
pub mod worktree;

#[cfg(test)]
pub(crate) mod testrepo;
