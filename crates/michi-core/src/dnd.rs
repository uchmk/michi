//! What dropping a commit or a branch on a worktree offers (#67). A drop adds
//! no operation of its own: every entry is one michi already has (`m`, `C`,
//! `M` through the palette), asked the same way with the same band and backup.
//! The rules are here, with nothing of egui in them, so a test can hold them.

use crate::oid::Oid;

/// What is dragged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// A commit's row in the graph.
    Commit(Oid),
    /// A branch's row in the side column: its full name (`origin/x` for a
    /// remote one) and the commit it points at.
    Branch { name: String, tip: Oid },
}

impl Source {
    /// The commit the source stands for.
    pub fn id(&self) -> Oid {
        match self {
            Source::Commit(id) | Source::Branch { tip: id, .. } => *id,
        }
    }

    /// What names the source in a menu or a band.
    pub fn label(&self) -> String {
        match self {
            Source::Commit(id) => id.short(),
            Source::Branch { name, .. } => name.clone(),
        }
    }

    /// What `git merge` and `git rebase` take for it.
    pub fn rev(&self) -> String {
        match self {
            Source::Commit(id) => id.to_string(),
            Source::Branch { name, .. } => name.clone(),
        }
    }
}

/// An operation a drop can offer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// Merge the source into the target's branch.
    Merge,
    /// Pick the commit onto the target.
    CherryPick,
    /// Rebase the target's branch onto the source.
    Rebase,
}

impl Op {
    /// The word-table key of its name.
    pub fn word(self) -> &'static str {
        match self {
            Op::Merge => "dnd.merge",
            Op::CherryPick => "dnd.pick",
            Op::Rebase => "dnd.rebase",
        }
    }
}

/// What can be done with `source` dropped on a worktree whose branch is
/// `target` (`None` when it is detached), in the order shown. Nothing
/// when the drop is a no-op (a branch on its own worktree).
pub fn ops(source: &Source, target: Option<&str>) -> Vec<Op> {
    match (source, target) {
        (Source::Commit(_), Some(_)) => vec![Op::Merge, Op::CherryPick],
        (Source::Commit(_), None) => vec![Op::CherryPick],
        (Source::Branch { name, .. }, Some(t)) if name == t => Vec::new(),
        (Source::Branch { .. }, Some(_)) => vec![Op::Merge, Op::Rebase],
        (Source::Branch { .. }, None) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(c: char) -> Oid {
        Oid::parse(&c.to_string().repeat(40)).unwrap()
    }

    fn branch(name: &str) -> Source {
        Source::Branch { name: name.into(), tip: id('a') }
    }

    #[test]
    fn a_commit_can_be_merged_or_picked() {
        assert_eq!(ops(&Source::Commit(id('a')), Some("feat")), vec![Op::Merge, Op::CherryPick]);
    }

    #[test]
    fn a_commit_on_a_detached_worktree_can_only_be_picked() {
        assert_eq!(ops(&Source::Commit(id('a')), None), vec![Op::CherryPick]);
    }

    #[test]
    fn a_branch_can_be_merged_or_rebased_onto() {
        assert_eq!(ops(&branch("main"), Some("feat")), vec![Op::Merge, Op::Rebase]);
    }

    #[test]
    fn a_branch_on_its_own_worktree_offers_nothing() {
        assert!(ops(&branch("feat"), Some("feat")).is_empty());
        assert!(ops(&branch("feat"), None).is_empty());
    }

    #[test]
    fn the_source_names_itself() {
        assert_eq!(branch("origin/x").rev(), "origin/x");
        assert_eq!(branch("origin/x").label(), "origin/x");
        assert_eq!(Source::Commit(id('b')).rev(), id('b').to_string());
        assert_eq!(Source::Commit(id('b')).label(), id('b').short());
        assert_eq!(branch("x").id(), id('a'));
    }
}
