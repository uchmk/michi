# 引き継ぎ

前の会話から次の会話への引き継ぎ。新しいセッションは `C:\dev\michi` で始め、CLAUDE.md・TODO.md・この文書の順に読む。
設計の正は [design.md](design.md) とモック。大きなファイル（design.md・graph.rs・wt_ui.rs など）は全部読まず、要るところだけ読む。

## いまの版（v0.9.0、2026-10-15）

段 1「見る」は終わり、段 2「変える」の最初（ブランチ全体の差分）まで。入っているもの:

- 樹形図（`ito-graph`、main が左端、lane の PR を畳む `z`、`/` の絞り込み、スクロールバーの印）。10 万コミットで最初の画面 0.1 s、
  畳む 3.5 ms、絞る 1 文字 4〜7 ms。commit-graph が無ければ裏で書く（#2）。
- 左の列（worktree・ブランチ・リモート・タグ・stash、`↑N` `↓N`）、コミットの詳細と差分（1 列と横並び、長い行は折り返す、UTF-8 でない注記）。
- WIP の行（worktree ごとの未コミットの変更）と、その worktree の Claude（`tsumugi ls --json`）。
- worktree の表（`3`、960 幅より狭いとカード）と衝突の予測（worktree どうしと trunk の組、前にいるあいだ 5 秒ごと）。
- ファイルの監視（`notify`）と未読の印（`%LOCALAPPDATA%\uchmk\michi\seen\`）。
- worktree の無いローカルのブランチと main の衝突の予測（main が取り込んでいない新しい 30 本、樹形図の頭に ⚠）。
- ブランチ全体の差分（worktree の表の `d`）: `main...<ブランチ>`、`u` 未コミット、`Space` 見た（パスと blob、
  `%LOCALAPPDATA%\uchmk\michi\review\`）、`s` 前回から（`refs/michi/seen/<ブランチ>`）、`Enter` 横並び。
  ロジックは `michi-core/src/review.rs`、状態は `michi/src/review.rs`、描画は `review_ui.rs`、読むのは `repo.rs` の 4 本目のスレッド。

実機の確認は needs-pc の issue #1, #3〜#9, #11 で PC のセッションに頼んである（13.1 だけ `[~]`）。
差分のレビューで推奨で決めたこと（キー・「前回」の動き・印の掃除・後回し）は `question` の #10。

## 作業の途中（ステージとコミット、`2` の画面）

v0.9.0 のあと、ロジックの側だけコミットした（版は上げていない。画面はまだ）。
**このコミットとその次（引き継ぎ書）はまだ push していない。**次のセッションは最初に R: があるのを確かめ、
`scripts/verify.sh` が `ALL OK` になってから main に push する（持ち主が 2026-10-15 に「再起動後に push」を選んだ）。

- `michi-core/src/stage.rs`: `lists`（ステージ済み・未ステージ・未追跡を並べて読む。未追跡は 1000 まで）、`diff`（1 ファイルの差分。
  未追跡は全部が追加の hunk）、`Patch::parse` と `pick`（選んだ行だけのパッチ。順は `git add -p` と同じで、消す行が足す行の前）、
  `apply`（`apply --cached --recount`、`reverse` で戻す）、`stage`（`add -A`）・`unstage`（`reset -q`）、`commit`（`-F -`、`--amend`）、
  `last_message`、`is_pushed`（HEAD を含むリモートのブランチがあるか）。テストは 10。
- `git.rs` に `feed`（標準入力に書く）。`show.rs` の `Status` に `Unmerged`（U）と `Untracked`（?）を足し、`Other` の字を X にした
  （版を上げるときの CHANGELOG に書く）。`parse_changes` を公開。
- 画面の側の予定:
  - `repo.rs` に 5 本目のスレッド "changes"。頼みは `List{gen, wt}`・`File{gen, wt, side, file}`。
    書く操作（`Stage`・`Unstage`・`Apply`・`StageAll`・`UnstageAll`・`Commit`）は捨てずに順に実行し、終わりかエラーのあとに一覧を読み直す。
    `Answer::Wips` のたびに一覧を読み直す。
  - `michi/src/changes.rs`（egui に依らない状態とテスト。`review.rs` の gen の作りをまねる）と `changes_ui.rs`（`review_ui.rs` をまねる）。
  - 行の選択は `detail_ui::diff` に選んだ行の印を足すか、`Wrap::tops` で上に重ねる。
  - コミットの文の入力欄は `main.rs` の `keys()` で絞り込みの欄と同じ扱いにする（Esc で出る、Ctrl+Enter でコミット、字は `feed` に渡さない）。
  - その worktree の Claude が実行中なら、最初のステージかコミットの前に y / Esc で確かめる（worktree を変えるか Claude が止まるまで覚える）。
    push 済みの HEAD の amend も y で確かめる。エラーと index の lock はトースト。
  - 決めたこと（キー・並び・Claude の確かめ）は `question` の issue に書き、TODO.md に（要確認: #N）。TESTING.md に 14 節。版は v0.10.0。

## ビルドの場所（2026-10-15 の夜）

ビルドは R:（RAM ディスク）だけ（`~/.claude/CLAUDE.md`）。C: に `CARGO_TARGET_DIR` を向けない。
22:34 ごろ R: が消えた（システムのログに RamDisk の Ntfs のエラー 137）。持ち主が再起動する。
R: が戻っていなければビルドせずに持ち主に知らせる。

## 次にやること

1. モックの 2 回目はやらない（持ち主が 2026-10-15 に「実装を進めて」）。段 2 で決めることは `question` の issue に文字で書き、推奨で進める。
2. 段 2 の続き: ステージとコミットの画面（上の「作業の途中」。`2` の画面、行と hunk、Claude が書いている最中の確かめ、タブの未コミットの数）、ブランチ、
   fetch / pull / push、日々の操作（design.md の「日々の操作」）、`refs/michi/backup/<時刻>` への退避。
   差分のレビューのメモ・マージ・PR は後の段（#10）。
3. 段 3〜6 は TODO.md のとおり。段 4 は tsumugi の `new --worktree` と `focus` が要る（tsumugi の TODO.md に書いてある）。

## 進め方（持ち主の希望）

- 速さを優先する。2〜3 の機能を 1 つの版にまとめる。作業中は `cargo test -p michi -p michi-core` で、`scripts/verify.sh` は push の前に 1 回。
- 確かめ終えたものは聞かずにコミットして main に push してよい。短い日本語の途中経過をこまめに書く。
- サブエージェントは持ち主が頼んだときだけ（使うなら Sonnet、worktree で）。
- 会話が長くなって要約が増えたら、ここを書き直して新しいセッションに移る。

## 道具のメモ

- ログはファイルに出して末尾だけ読む: `cargo test -p michi -p michi-core > "$TEMP/claude/t.log" 2>&1` のあと
  `test result|FAILED|panicked|^error` を探す。verify も `bash scripts/verify.sh > "$TEMP/claude/v.log" 2>&1; echo rc=$?; tail -2 …`
  （`| tail` の後ろに push を繋げない。版を上げたら `cargo build` で Cargo.lock も合わせる）。
- この PC の Bash は rtk のフックが通る。生の出力が要るときは `rtk proxy cargo …`。heredoc や sed の `\` は崩れることがあるので、
  込み入った書き換えは Edit か、`newline=''` の Python のスクリプトで。
- 画面の確認は `scripts/shot.ps1`（PowerShell。先に `cargo build -p michi`。`-Keys` の書き方はファイルの頭）。撮ったあと
  `tasklist` で `michi.exe`・`git.exe` が残っていないか見る（ほかのセッションの git は触らない）。
- 大きな履歴は `scripts/make-history.py` で作り、`MICHI_BENCH_REPO=<そのフォルダ> cargo test --release -p michi big_history -- --ignored --nocapture`。
- 単体テストは本物のリポジトリに触らない（`michi-core/src/testrepo.rs` の `TempRepo`）。
- ito の rev は a54872d2309c5a82df69c851cecf20e17e20a28c（ito v0.36.0）。
- tsumugi・yagura・kura・lane の worktree のファイルは直さない。tsumugi で michi を開いて試さない（git の掃除が走って重くなった）。

## 決まっていること（会話でだけ決めたもの）

- 1 つの窓が 1 つのリポ（全部の worktree）。樹形図の左端は main。ライセンスは MIT / Apache-2.0。ロゴは A 案。ロゴの横に「道」を出さない。
- lane の PR（`test/win-*` など）は畳む。
- stash の行のクリックは stash の段まで何もしない（stash のコミットは樹形図に無い）。
- U+FFFD は今の字体に無く □ で出る。注記と差分で同じなので、そのままにした。
