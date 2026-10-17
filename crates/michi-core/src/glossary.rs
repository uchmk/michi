//! The glossary (issues #104 and #105): one table of the Git words a beginner
//! stumbles on, with a plain wording for each ("easy display", #104) and a
//! one-or-two-line explanation for the hover (#105). English and Japanese.
//!
//! `[view] easy = true` in config.toml turns the plain wording on; the hover
//! works either way.

/// One term: `id` is how the screen asks for it, `git` the word Git uses.
pub struct Term {
    pub id: &'static str,
    pub git: &'static str,
    pub plain_en: &'static str,
    pub plain_ja: &'static str,
    pub tip_en: &'static str,
    pub tip_ja: &'static str,
}

pub const TERMS: &[Term] = &[
    Term {
        id: "stage",
        git: "stage",
        plain_en: "add to the next commit",
        plain_ja: "次のコミットに入れる",
        tip_en: "Pick which changes go into the next commit. Nothing is saved to the history until you commit.",
        tip_ja: "次のコミットに入れる変更を選ぶことです。コミットするまで履歴には何も記録されません。",
    },
    Term {
        id: "unstage",
        git: "unstage",
        plain_en: "take out of the next commit",
        plain_ja: "次のコミットから外す",
        tip_en: "Take a change back out of the next commit. The file itself is not touched.",
        tip_ja: "次のコミットに入れた変更を外します。ファイルの中身は変わりません。",
    },
    Term {
        id: "staged",
        git: "Staged",
        plain_en: "In the next commit",
        plain_ja: "次のコミットに入る",
        tip_en: "Changes that will be saved by the next commit.",
        tip_ja: "次のコミットで記録される変更です。",
    },
    Term {
        id: "unstaged",
        git: "Not staged",
        plain_en: "Not in the next commit",
        plain_ja: "次のコミットに入らない",
        tip_en: "Changed files that the next commit will leave out until you add them.",
        tip_ja: "変更はあるが、入れるまで次のコミットには含まれないファイルです。",
    },
    Term {
        id: "commit",
        git: "commit",
        plain_en: "save to the history",
        plain_ja: "履歴に記録する",
        tip_en: "A saved snapshot of the staged changes, with a message. It can be found and undone later.",
        tip_ja: "入れた変更をメッセージ付きで履歴に記録したものです。あとで探したり戻したりできます。",
    },
    Term {
        id: "stash",
        git: "stash",
        plain_en: "set aside for later",
        plain_ja: "あとで使うために置いておく",
        tip_en: "Put the current changes on a shelf and get a clean worktree; bring them back whenever you like.",
        tip_ja: "今の変更をいったん棚に置いて作業フォルダをきれいにします。あとで元に戻せます。",
    },
    Term {
        id: "rebase",
        git: "rebase",
        plain_en: "replay on top of another branch",
        plain_ja: "別のブランチの先頭に付け替える",
        tip_en: "Move your commits so they start from the newest point of another branch. The commits are rewritten.",
        tip_ja: "自分のコミットを、別のブランチの最新の位置から始まるように付け替えます。コミットは作り直されます。",
    },
    Term {
        id: "merge",
        git: "merge",
        plain_en: "combine branches",
        plain_ja: "ブランチを合流させる",
        tip_en: "Bring the work of one branch into another. Conflicts are asked about when both changed the same lines.",
        tip_ja: "あるブランチの作業を別のブランチに取り込みます。同じ行を変えていると衝突として確かめます。",
    },
    Term {
        id: "cherry-pick",
        git: "cherry-pick",
        plain_en: "copy one commit here",
        plain_ja: "コミットを 1 つだけコピーしてくる",
        tip_en: "Copy a single commit from another branch onto this one.",
        tip_ja: "別のブランチのコミットを 1 つだけ、今のブランチにコピーします。",
    },
    Term {
        id: "revert",
        git: "revert",
        plain_en: "undo a commit with a new commit",
        plain_ja: "コミットを打ち消す新しいコミットを作る",
        tip_en: "Make a new commit that cancels an old one. The history is kept, so it is safe on shared branches.",
        tip_ja: "古いコミットを打ち消す新しいコミットを作ります。履歴は残るので、共有のブランチでも安全です。",
    },
    Term {
        id: "reset",
        git: "reset",
        plain_en: "move the branch back",
        plain_ja: "ブランチを過去の位置に戻す",
        tip_en: "Point the branch at an older commit. Files and staged changes may be lost; michi saves a backup first.",
        tip_ja: "ブランチを古いコミットの位置に動かします。モードによってはファイルや入れた変更が失われます。michi は先に退避します。",
    },
    Term {
        id: "amend",
        git: "amend",
        plain_en: "fix the last commit",
        plain_ja: "直前のコミットを直す",
        tip_en: "Replace the last commit with a corrected one (message or contents). Do not do it after pushing.",
        tip_ja: "直前のコミットを、メッセージや中身を直したもので置き換えます。push したあとはしないでください。",
    },
    Term {
        id: "detached",
        git: "detached HEAD",
        plain_en: "not on any branch",
        plain_ja: "どのブランチにもいない状態",
        tip_en: "You are on a commit, not a branch. New commits here are easy to lose; make a branch to keep them.",
        tip_ja: "ブランチではなくコミットそのものを見ている状態です。ここで作ったコミットは失いやすいので、ブランチを作って残します。",
    },
    Term {
        id: "head",
        git: "HEAD",
        plain_en: "where you are now",
        plain_ja: "今いる場所",
        tip_en: "The commit (usually through a branch) that this worktree is on.",
        tip_ja: "この作業フォルダが今いるコミット（ふつうはブランチ経由）です。",
    },
    Term {
        id: "upstream",
        git: "upstream",
        plain_en: "the branch on the server it follows",
        plain_ja: "追いかけているサーバー上のブランチ",
        tip_en: "The remote branch a local branch is paired with; ahead and behind are counted against it.",
        tip_ja: "手元のブランチと対になっているサーバー側のブランチです。進み・遅れはこれと比べて数えます。",
    },
    Term {
        id: "fetch",
        git: "fetch",
        plain_en: "check the server for news",
        plain_ja: "サーバーの更新を取ってくる",
        tip_en: "Download what is new on the server without touching your files or branches.",
        tip_ja: "サーバーの新しい内容を取ってきます。手元のファイルやブランチは変わりません。",
    },
    Term {
        id: "push",
        git: "push",
        plain_en: "send commits to the server",
        plain_ja: "コミットをサーバーに送る",
        tip_en: "Upload your commits so that others can see them.",
        tip_ja: "自分のコミットをサーバーに送って、ほかの人から見えるようにします。",
    },
    Term {
        id: "worktree",
        git: "worktree",
        plain_en: "working folder",
        plain_ja: "作業フォルダ",
        tip_en: "A folder with its own checked-out branch. One repository can have several, one per Claude session.",
        tip_ja: "ブランチを 1 つ取り出した作業用のフォルダです。1 つのリポジトリに複数持て、Claude のセッションごとに使います。",
    },
    Term {
        id: "tag",
        git: "tag",
        plain_en: "name for a commit",
        plain_ja: "コミットに付ける名札",
        tip_en: "A fixed name on one commit, often a release such as v1.0.",
        tip_ja: "1 つのコミットに付ける動かない名前です。v1.0 のようなリリースの印に使います。",
    },
];

/// The term with `id`.
pub fn find(id: &str) -> Option<&'static Term> {
    TERMS.iter().find(|t| t.id == id)
}

fn ja(lang: &str) -> bool {
    lang == "ja"
}

impl Term {
    /// The plain wording in `lang` (`ja`, else English).
    pub fn plain(&self, lang: &str) -> &'static str {
        if ja(lang) {
            self.plain_ja
        } else {
            self.plain_en
        }
    }

    /// The hover text in `lang`.
    pub fn tip(&self, lang: &str) -> &'static str {
        if ja(lang) {
            self.tip_ja
        } else {
            self.tip_en
        }
    }

    /// The easy display's label: the plain wording with Git's own word beside
    /// it, `set aside for later (stash)`.
    pub fn easy(&self, lang: &str) -> String {
        if ja(lang) {
            format!("{}（{}）", self.plain_ja, self.git)
        } else {
            format!("{} ({})", self.plain_en, self.git)
        }
    }
}

/// The label for `id`: the easy wording when `easy`, else `normal` (the
/// screen's usual word). `normal` also stands in for an unknown `id`.
pub fn label(id: &str, lang: &str, easy: bool, normal: &str) -> String {
    match find(id) {
        Some(t) if easy => t.easy(lang),
        _ => normal.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_every_text_is_written() {
        let mut seen = std::collections::HashSet::new();
        for t in TERMS {
            assert!(seen.insert(t.id), "`{}` twice", t.id);
            for s in [t.git, t.plain_en, t.plain_ja, t.tip_en, t.tip_ja] {
                assert!(!s.trim().is_empty(), "`{}` has an empty text", t.id);
            }
            assert_ne!(t.plain_en.to_lowercase(), t.git.to_lowercase(), "`{}` is not reworded", t.id);
        }
    }

    #[test]
    fn the_japanese_is_japanese_and_the_english_is_not() {
        let has_ja = |s: &str| s.chars().any(|c| ('\u{3040}'..='\u{30ff}').contains(&c) || ('\u{4e00}'..='\u{9fff}').contains(&c));
        for t in TERMS {
            assert!(has_ja(t.plain_ja) && has_ja(t.tip_ja), "`{}` ja", t.id);
            assert!(!has_ja(t.plain_en) && !has_ja(t.tip_en), "`{}` en", t.id);
        }
    }

    #[test]
    fn a_hover_text_is_short() {
        for t in TERMS {
            assert!(t.tip_en.chars().count() <= 170, "`{}` en is long", t.id);
            assert!(t.tip_ja.chars().count() <= 90, "`{}` ja is long", t.id);
        }
    }

    #[test]
    fn the_easy_label_keeps_git_s_word_in_brackets() {
        let t = find("stash").unwrap();
        assert_eq!(t.easy("en"), "set aside for later (stash)");
        assert_eq!(t.easy("ja"), "あとで使うために置いておく（stash）");
        assert_eq!(t.easy("fr"), t.easy("en"));
        assert_eq!(t.plain("ja"), "あとで使うために置いておく");
        assert!(t.tip("en").starts_with("Put"));
    }

    #[test]
    fn label_is_the_usual_word_unless_easy_and_known() {
        assert_eq!(label("stash", "en", false, "Stash"), "Stash");
        assert_eq!(label("stash", "en", true, "Stash"), "set aside for later (stash)");
        assert_eq!(label("nope", "en", true, "Whatever"), "Whatever");
        assert!(find("nope").is_none());
    }
}
