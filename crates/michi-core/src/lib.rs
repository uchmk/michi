//! What michi knows about Git, with nothing of egui in it: running git, the refs,
//! the history and what the graph makes of it, the worktrees, the paths, the
//! Claude sessions tsumugi lists, the conflict prediction, the uncommitted
//! changes. The window is the `michi` crate.

pub mod autostash;
pub mod backup;
pub mod branch;
pub mod change;
pub mod clash;
pub mod claude;
pub mod conflict;
pub mod density;
pub mod detached;
pub mod discard;
pub mod external;
pub mod find;
pub mod git;
pub mod handoff;
pub mod history;
pub mod ignore;
pub mod log;
pub mod menu;
pub mod msghelp;
pub mod net;
pub mod newwork;
pub mod notes;
pub mod oid;
pub mod openin;
pub mod overlap;
pub mod paths;
pub mod pr;
pub mod predict;
pub mod queue;
pub mod rebase;
pub mod recent;
pub mod reflog;
pub mod refs;
pub mod rerere;
pub mod reset;
pub mod revert;
pub mod review;
pub mod show;
pub mod stage;
pub mod stash;
pub mod summary;
pub mod tag;
pub mod tidy;
pub mod track;
pub mod tree;
pub mod unseen;
pub mod wip;
pub mod words;
pub mod worktree;

#[cfg(test)]
pub(crate) mod testrepo;
