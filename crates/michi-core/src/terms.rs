//! The one table of michi's terms in English and Japanese (#125): which word
//! each Git idea is called on the screen, in the toasts and in the docs, and
//! the variants that must not be used. `GLOSSARY.md` shows the same table to
//! people; a test in the michi crate keeps the i18n tables to it. No egui.

/// One idea and its two words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Term {
    /// What the idea is.
    pub what: &'static str,
    /// The English word.
    pub en: &'static str,
    /// The Japanese word.
    pub ja: &'static str,
    /// English spellings that must not appear (compared in lower case).
    pub en_not: &'static [&'static str],
    /// Japanese spellings that must not appear.
    pub ja_not: &'static [&'static str],
}

const fn term(
    what: &'static str,
    en: &'static str,
    ja: &'static str,
    en_not: &'static [&'static str],
    ja_not: &'static [&'static str],
) -> Term {
    Term { what, en, ja, en_not, ja_not }
}

/// Every term. Git's own command names (`push`, `pull`, `rebase`, `stash`, `worktree`)
/// stay in Latin letters in Japanese, as the commands are typed.
pub const TERMS: &[Term] = &[
    term("a line of work in a repository", "branch", "ブランチ", &[], &["枝"]),
    term("a folder checked out from a repository", "worktree", "worktree", &["work tree", "work-tree"], &["ワークツリー", "ワークトリー"]),
    term(
        "the files as they are now, before a commit",
        "working tree",
        "作業ツリー",
        &[],
        &["作業フォルダ", "作業ディレクトリ", "ワーキングツリー"],
    ),
    term("the thing git keeps", "repository", "リポジトリ", &[], &["レポジトリ", "リポジトリー"]),
    term("changes that cannot be joined by themselves", "conflict", "衝突", &[], &["コンフリクト", "競合"]),
    term("a conflict foreseen before merging", "clash", "衝突", &[], &["コンフリクト", "競合"]),
    term("the branch the others are merged into", "trunk", "幹", &[], &["トランク"]),
    term("changes put aside by git stash", "stash", "stash", &["shelve", "shelf"], &["スタッシュ", "退避棚"]),
    term("a run of changes in a diff", "hunk", "ハンク", &[], &["hunk"]),
    term("what a diff shows", "diff", "差分", &[], &["diff の", "diff を", "diff は", "diff が", "diff と", "diff に", "diff で"]),
    term("remove a file or a ref", "delete", "消す", &[], &["削除", "デリート"]),
    term("give up uncommitted changes", "throw away", "捨てる", &["discard"], &["破棄"]),
    term("a copy kept before something is lost", "kept", "退避", &[], &["バックアップ"]),
    term("send commits to a remote", "push", "push", &[], &["プッシュ"]),
    term("bring a remote's commits in", "pull", "pull", &[], &["フェッチ", "プルする", "プルし"]),
    term("the branch a branch follows", "upstream", "upstream", &[], &["上流"]),
    term("the app's own screen", "window", "窓", &[], &["ウィンドウ", "ウインドウ"]),
    term("a place for files", "folder", "フォルダ", &["directory"], &["フォルダー", "ディレクトリ"]),
    term("a name on a commit", "tag", "タグ", &[], &["ラベル"]),
    term("move a change into the index", "stage", "ステージ", &[], &["ステージング"]),
    term("a saved snapshot", "commit", "コミット", &[], &[]),
    term("join one branch into another", "merge", "マージ", &[], &[]),
    term("where HEAD or a branch points", "tip", "先端", &[], &[]),
];

/// The Japanese variants that must not be used, with the term each belongs to.
pub fn banned_ja() -> impl Iterator<Item = (&'static str, &'static Term)> {
    TERMS.iter().flat_map(|t| t.ja_not.iter().map(move |w| (*w, t)))
}

/// The English variants that must not be used (lower case), with their terms.
pub fn banned_en() -> impl Iterator<Item = (&'static str, &'static Term)> {
    TERMS.iter().flat_map(|t| t.en_not.iter().map(move |w| (*w, t)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_is_never_both_a_term_and_a_banned_variant() {
        for t in TERMS {
            for (w, o) in banned_ja() {
                assert!(!t.ja.contains(w), "`{}` ({}) holds `{w}`, banned for {}", t.ja, t.what, o.what);
            }
            for (w, o) in banned_en() {
                assert!(!t.en.to_lowercase().contains(w), "`{}` ({}) holds `{w}`, banned for {}", t.en, t.what, o.what);
            }
        }
    }

    #[test]
    fn each_idea_is_listed_once() {
        let mut seen = std::collections::HashSet::new();
        for t in TERMS {
            assert!(seen.insert(t.what), "`{}` is listed twice", t.what);
        }
    }
}
