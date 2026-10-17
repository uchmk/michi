//! What a right-click menu lists. A menu adds no operation of its own: every
//! entry is a key michi already has, so picking one does what typing the key
//! would have done on the same row (the palette does the same). The lists are
//! here, with nothing of egui in them, so a test can hold them to the keys.

/// What was clicked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A commit's row in the graph (a branch name on it is part of the row).
    Commit,
    /// A worktree's row in the table: its branch and its folder.
    Branch,
    /// A file's row in the changes list.
    File,
}

/// One entry: the key it stands for and the word-table key of its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item {
    pub key: char,
    pub word: &'static str,
}

const fn item(key: char, word: &'static str) -> Item {
    Item { key, word }
}

const COMMIT: [Item; 7] = [
    item('b', "key.branch"),
    item('T', "key.tag"),
    item('m', "key.merge_commit"),
    item('C', "key.cherry_pick"),
    item('t', "key.revert"),
    item('D', "key.compare"),
    item('R', "key.reset"),
];

const BRANCH: [Item; 10] = [
    item('O', "key.pr"),
    item('t', "key.open_terminal"),
    item('e', "key.open_editor"),
    item('E', "key.open_explorer"),
    item('d', "key.branch_diff"),
    item('M', "key.rebase_trunk"),
    item('m', "key.take_main"),
    item('r', "key.reorder"),
    item('L', "key.reflog"),
    item('x', "key.remove"),
];

const FILE: [Item; 5] =
    [item(' ', "key.stage"), item('s', "key.split"), item('i', "key.ignore"), item('d', "key.discard"), item('A', "key.amend")];

/// The entries of a menu, in the order shown (the ones that lose something
/// last).
pub fn items(kind: Kind) -> &'static [Item] {
    match kind {
        Kind::Commit => &COMMIT,
        Kind::Branch => &BRANCH,
        Kind::File => &FILE,
    }
}

/// How a key is spelled beside its entry.
pub fn key_label(key: char) -> String {
    match key {
        ' ' => "Space".to_owned(),
        c => c.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Kind; 3] = [Kind::Commit, Kind::Branch, Kind::File];

    #[test]
    fn a_key_is_listed_once_in_a_menu() {
        for kind in ALL {
            let mut keys: Vec<char> = items(kind).iter().map(|i| i.key).collect();
            keys.sort_unstable();
            keys.dedup();
            assert_eq!(keys.len(), items(kind).len(), "{kind:?}");
        }
    }

    #[test]
    fn every_entry_is_named() {
        for kind in ALL {
            assert!(items(kind).iter().all(|i| i.word.starts_with("key.")), "{kind:?}");
        }
    }

    #[test]
    fn the_space_key_is_spelled_out() {
        assert_eq!(key_label(' '), "Space");
        assert_eq!(key_label('d'), "d");
    }
}
