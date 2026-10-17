# 用語集 / Glossary

画面・トースト・docs では、下の表の言葉を使う。表の正は `crates/michi-core/src/terms.rs`（`TERMS`）で、
`crates/michi/src/i18n.rs` のテストが、使ってはいけない書き方（右端の列）が i18n の表に無いことと、この表が `TERMS` と合っていることを見る。
Git のコマンド名（push・pull・rebase・stash・worktree）は、打つものなので日本語でもそのまま書く。
言葉を変えるときは `terms.rs` とこの表を一緒に直す。

| What | English | 日本語 | Not used |
| --- | --- | --- | --- |
| a line of work in a repository | branch | ブランチ | 枝 |
| a folder checked out from a repository | worktree | worktree | work tree, work-tree, ワークツリー, ワークトリー |
| the files as they are now, before a commit | working tree | 作業ツリー | 作業フォルダ, 作業ディレクトリ, ワーキングツリー |
| the thing git keeps | repository | リポジトリ | レポジトリ, リポジトリー |
| changes that cannot be joined by themselves | conflict | 衝突 | コンフリクト, 競合 |
| a conflict foreseen before merging | clash | 衝突 | コンフリクト, 競合 |
| the branch the others are merged into | trunk | 幹 | トランク |
| changes put aside by git stash | stash | stash | shelve, shelf, スタッシュ, 退避棚 |
| a run of changes in a diff | hunk | ハンク | hunk |
| what a diff shows | diff | 差分 | diff の, diff を, diff は, diff が, diff と, diff に, diff で |
| remove a file or a ref | delete | 消す | 削除, デリート |
| give up uncommitted changes | throw away | 捨てる | discard, 破棄 |
| a copy kept before something is lost | kept | 退避 | バックアップ |
| send commits to a remote | push | push | プッシュ |
| bring a remote's commits in | pull | pull | フェッチ, プルする, プルし |
| the branch a branch follows | upstream | upstream | 上流 |
| the app's own screen | window | 窓 | ウィンドウ, ウインドウ |
| a place for files | folder | フォルダ | directory, フォルダー, ディレクトリ |
| a name on a commit | tag | タグ | ラベル |
| move a change into the index | stage | ステージ | ステージング |
| a saved snapshot | commit | コミット | — |
| join one branch into another | merge | マージ | — |
| where HEAD or a branch points | tip | 先端 | — |
