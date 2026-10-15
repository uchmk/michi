//! michi's own words, in English and Japanese. The language is the one in
//! common.toml (`auto`: the OS's), shared by every uchmk app; the settings
//! screen's own words are `ito_prefs::Words`.

const EN: &[(&str, &str)] = &[
    ("view.graph", "Graph"),
    ("view.changes", "Changes"),
    ("view.worktree", "Worktrees"),
    ("view.settings", "Settings"),
    ("palette.hint", "Search commands"),
    ("empty.graph", "The history of every branch and worktree will be drawn here, main on the left."),
    ("empty.changes", "The changes of this worktree will be staged and committed here."),
    ("empty.worktree", "The worktrees will be listed here, with their branches and the Claude Code running in each."),
    ("empty.soon", "Coming in a later stage."),
    ("later", "{} — coming in a later stage"),
    ("help.title", "Keys"),
    ("help.global", "Everywhere"),
    ("help.close", "Press ? or Esc to close"),
    ("key.views", "Switch the view"),
    ("key.new_work", "New work (worktree + Claude)"),
    ("key.fetch", "Fetch"),
    ("key.pull", "Pull"),
    ("key.push", "Push"),
    ("key.settings", "Settings"),
    ("key.palette", "Search commands"),
    ("key.help", "This list"),
    ("key.move", "Move"),
    ("key.side", "Side by side"),
    ("key.filter", "Filter"),
    ("key.fold", "Fold the lane PRs"),
    ("key.tab", "Go to the tab"),
    ("key.open_fold", "Open or fold a merge"),
    ("key.branch", "Branch"),
    ("key.stash", "Stash"),
    ("key.back", "Back"),
    ("key.page", "Next page"),
    ("key.stage", "Stage"),
    ("key.commit", "Commit"),
    ("key.select", "Select"),
    ("key.in_graph", "Show in the graph"),
    ("key.take_main", "Take in main"),
    ("key.remove", "Remove"),
    ("key.branch_diff", "Diff of the whole branch"),
    ("repo.loading", "Reading…"),
    ("repo.no_git", "git was not found"),
    ("repo.not_repo", "Not a Git repository"),
    ("repo.failed", "git failed"),
    ("git.old", "git {v} is older than {min}; some views will not work. Please update git."),
    ("history.reading", "Reading the history…"),
    ("history.empty", "No commits yet."),
    ("history.failed", "Could not read the history: {}"),
    ("fold.on", "Folded the merges into {trunk}: {n} ({m} commits)."),
    ("fold.off", "Unfolded all the merges."),
    ("fold.nothing", "No merges to fold yet."),
    ("fold.tip", "Show only main's first parents and fold the commits merges brought into main"),
    ("fold.chip", "{} commits folded (l opens them)"),
    ("fold.names", "Branches inside: {}"),
    ("fold.open", "h folds them again"),
    ("fold.label", "Fold PRs"),
    ("col.subject", "Subject"),
    ("col.author", "Author"),
    ("col.date", "Date"),
    ("col.commit", "Commit"),
    ("time.yesterday", "Yesterday"),
    ("filter.hint", "Filter (subject, hash, name)"),
    ("filter.count", "{} hits"),
    ("more.count", "… {} older commits — click to read them"),
    ("more.unknown", "… older commits — click to read them"),
    ("more.reading", "… reading older commits"),
    ("mark.hit", "Filtered: {}"),
    ("mark.tag", "Tag {}"),
    ("mark.selected", "Selected"),
    ("tool.new_work", "New work"),
    ("settings.general", "General"),
    ("settings.general_lead", "The language, the size and the clock, shared by every uchmk app."),
    ("settings.theme", "Theme"),
    ("settings.theme_lead", "The colours, shared by every uchmk app."),
    ("settings.display", "Display"),
    ("settings.theme_note", "Your own themes go in"),
    ("settings.no_folder", "This machine has no config folder."),
    ("settings.sample", "A sample in this theme"),
];

const JA: &[(&str, &str)] = &[
    ("view.graph", "樹形図"),
    ("view.changes", "変更"),
    ("view.worktree", "worktree"),
    ("view.settings", "設定"),
    ("palette.hint", "コマンドを探す"),
    ("empty.graph", "全部のブランチと worktree の履歴がここに描かれます（左端は main）。"),
    ("empty.changes", "この worktree の変更をここでステージしてコミットします。"),
    ("empty.worktree", "worktree と、そのブランチ・そこで動く Claude Code がここに並びます。"),
    ("empty.soon", "後の段階で入ります。"),
    ("later", "{} — 後の段階で入ります"),
    ("help.title", "キー"),
    ("help.global", "どこでも"),
    ("help.close", "? か Esc で閉じる"),
    ("key.views", "画面を切り替える"),
    ("key.new_work", "新しい作業（worktree + Claude）"),
    ("key.fetch", "fetch"),
    ("key.pull", "pull"),
    ("key.push", "push"),
    ("key.settings", "設定"),
    ("key.palette", "コマンドを探す"),
    ("key.help", "この一覧"),
    ("key.move", "動かす"),
    ("key.side", "横並び"),
    ("key.filter", "絞る"),
    ("key.fold", "PR を畳む"),
    ("key.tab", "タブへ"),
    ("key.open_fold", "マージを開く・畳む"),
    ("key.branch", "ブランチ"),
    ("key.stash", "stash"),
    ("key.back", "戻る"),
    ("key.page", "次のページ"),
    ("key.stage", "ステージ"),
    ("key.commit", "コミット"),
    ("key.select", "選ぶ"),
    ("key.in_graph", "樹形図で見る"),
    ("key.take_main", "main を取り込む"),
    ("key.remove", "消す"),
    ("key.branch_diff", "ブランチ全体の差分"),
    ("repo.loading", "読んでいます…"),
    ("repo.no_git", "git が見つかりません"),
    ("repo.not_repo", "Git のリポジトリではありません"),
    ("repo.failed", "git が失敗しました"),
    ("git.old", "git {v} は {min} より古いため、一部の画面が動きません。git を新しくしてください。"),
    ("history.reading", "履歴を読んでいます…"),
    ("history.empty", "まだコミットがありません。"),
    ("history.failed", "履歴を読めませんでした: {}"),
    ("fold.on", "{trunk} に入った {n} 本のマージを畳みました（{m} コミット）。"),
    ("fold.off", "マージを全部広げました。"),
    ("fold.nothing", "畳むマージはまだありません。"),
    ("fold.tip", "main の first-parent だけを見せ、マージで main に入ったコミットを畳む"),
    ("fold.chip", "{} コミットを畳んでいます（l で開く）"),
    ("fold.names", "中のブランチ: {}"),
    ("fold.open", "h で畳む"),
    ("fold.label", "PR を畳む"),
    ("col.subject", "説明"),
    ("col.author", "作者"),
    ("col.date", "日時"),
    ("col.commit", "コミット"),
    ("time.yesterday", "昨日"),
    ("filter.hint", "絞る（説明・ハッシュ・名前）"),
    ("filter.count", "{} 件"),
    ("more.count", "… さらに古い {} 件 — 押すと読み込みます"),
    ("more.unknown", "… さらに古いコミット — 押すと読み込みます"),
    ("more.reading", "… さらに古いコミットを読んでいます"),
    ("mark.hit", "絞った結果: {}"),
    ("mark.tag", "タグ {}"),
    ("mark.selected", "選んでいる所"),
    ("tool.new_work", "新しい作業"),
    ("settings.general", "一般"),
    ("settings.general_lead", "言語・大きさ・時計。uchmk のどのアプリでも共通です。"),
    ("settings.theme", "テーマ"),
    ("settings.theme_lead", "色。uchmk のどのアプリでも共通です。"),
    ("settings.display", "表示"),
    ("settings.theme_note", "自分のテーマの置き場所:"),
    ("settings.no_folder", "このマシンには設定のフォルダがありません。"),
    ("settings.sample", "このテーマでの見本"),
];

/// The language codes michi has words for.
pub const AVAILABLE: [&str; 2] = ["en", "ja"];

/// The words for `key` in `lang` (`ja`, else English); the key itself when
/// there are none.
pub fn tr(lang: &str, key: &'static str) -> &'static str {
    let table = if lang == "ja" { JA } else { EN };
    table.iter().chain(EN).find(|(k, _)| *k == key).map_or(key, |(_, v)| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_languages_have_the_same_keys() {
        fn keys(t: &[(&'static str, &'static str)]) -> Vec<&'static str> {
            t.iter().map(|(k, _)| *k).collect()
        }
        assert_eq!(keys(EN), keys(JA));
    }

    #[test]
    fn unknown_keys_come_back_as_they_are() {
        assert_eq!(tr("ja", "nope"), "nope");
        assert_eq!(tr("ja", "view.graph"), "樹形図");
        assert_eq!(tr("fr", "view.graph"), "Graph");
    }
}
