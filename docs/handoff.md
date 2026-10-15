# 引き継ぎ

前の会話から次の会話への引き継ぎ。新しいセッションは `C:\dev\michi` で始め、CLAUDE.md・TODO.md・この文書の順に読む。
設計の正は [design.md](design.md) とモック。大きなファイル（design.md・graph.rs・wt_ui.rs など）は全部読まず、要るところだけ読む。

## いまの版（v0.10.1、2026-10-16）

段 1「見る」は終わり、段 2「変える」のうち、ブランチ全体の差分（v0.9.0）とステージとコミット（v0.10.0）まで。入っているもの:

- 樹形図（`ito-graph`、main が左端、lane の PR を畳む `z`、`/` の絞り込み、スクロールバーの印）。10 万コミットで最初の画面 0.1 s、
  畳む 3.5 ms、絞る 1 文字 4〜7 ms。commit-graph が無ければ裏で書く（#2）。
- 左の列（worktree・ブランチ・リモート・タグ・stash、`↑N` `↓N`）、コミットの詳細と差分（1 列と横並び、長い行は折り返す、UTF-8 でない注記）。
- WIP の行（worktree ごとの未コミットの変更）と、その worktree の Claude（`tsumugi ls --json`）。
- worktree の表（`3`、960 幅より狭いとカード）と衝突の予測（worktree どうしと trunk の組、前にいるあいだ 5 秒ごと）。
- ファイルの監視（`notify`）と未読の印（`%LOCALAPPDATA%\uchmk\michi\seen\`）。
- worktree の無いローカルのブランチと main の衝突の予測（main が取り込んでいない新しい 30 本、樹形図の頭に ⚠）。
- ブランチ全体の差分（worktree の表の `d`）: `main...<ブランチ>`、`u` 未コミット、`Space` 見た（パスと blob、
  `%LOCALAPPDATA%\uchmk\michi\review\`）、`s` 前回から（`refs/michi/seen/<ブランチ>`）、`Enter` 横並び。
  ロジックは `michi-core/src/review.rs`、状態は `michi/src/review.rs`、描画は `review_ui.rs`。
- ステージとコミット（`2`）: 上に worktree のチップ（`[` `]`、Claude が書いている途中なら ●）と amend のスイッチ（`A`）、
  左にステージ済み・未ステージ（未追跡は `?` で最後、1000 まで）、その下にメッセージの欄、右に差分。
  `Space` 入れる・出す、`a` 全部、`l` 行（`j` `k` は変わった行だけ、`v` 範囲、`a` ハンク、`h` 戻る）、`c` 欄へ、`Ctrl+Enter` コミット、`Enter` 横並び。
  Esc は確かめの帯 → 行 → 横並びの順に戻る。パッチを作れないファイル（未追跡・バイナリ・名前の変更・モード）は丸ごと。
  ロジックは `michi-core/src/stage.rs`（`lists`・`diff`・`Patch::pick`・`apply --cached --recount`・`commit -F -`・`is_pushed`）、
  状態とテストは `michi/src/changes.rs`、描画は `changes_ui.rs`。
- 読むスレッドは `repo.rs` に detail・watch・review・changes・repo（と commit-graph）。changes の書く頼み（ステージ・外す・apply・コミット）は
  捨てずに順に実行し、終わったら一覧を読み直す。worktree を読み直すたびに `Answer::Reread` を送り、`2` を開いていれば一覧を読み直す。

実機の確認は needs-pc の issue #1, #3〜#9, #11, #13 で PC のセッションに頼んである。この会話で付けた `[~]` は 13.1・14.3・14.4・14.11。
推奨で決めたことの `question` は、差分のレビューが #10、`2` の画面（キー・配置・Claude の確かめ・amend・行を選べないファイル・タブの数・後回し）が #12。
どちらもまだ回答は無い。

## 次にやること

1. モックの 2 回目はやらない（持ち主が 2026-10-15 に「実装を進めて」）。段 2 で決めることは `question` の issue に文字で書き、推奨で進める。
2. 段 2 の続き（TODO.md の 2.）:
   - ブランチ（作る・切り替える・消す。消す前に `refs/michi/backup/<時刻>` へ退避）と、fetch / pull / push の進み具合（`--progress` を読む）。
   - 日々の操作（design.md の「日々の操作」）: stash、変更を捨てる、revert、タグ、片付け。壊す操作の前に `refs/michi/backup/<時刻>` へ退避し、
     確かめの窓に何が失われるかを書く。作業中の Claude の worktree では `2` と同じ帯で確かめる。
   - `2` の残り（#12）: ハンクの分割（`git add -p` の `s`）、コミットの hooks の出力を全部見せる（今はトーストに 1 行）。
   - 差分のレビューのメモ・マージ・PR は後の段（#10）。
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
- 画面の確認は `scripts/shot.ps1`（PowerShell。先に `cargo build -p michi`。`-Keys` の書き方はファイルの頭）。
  - 例: `& .\scripts\shot.ps1 -Exe R:\cargo-target\michi\debug\michi.exe -Arg <リポ> -Out <scratchpad>\shots\x -Width 1100 -Keys "2;j|l;v|j| "`。
  - `;` で区切った組ごとに 1 枚。最初の組の前が `x-a.png`、組 1〜5 が `x-b`〜`x-f`。キーは SendKeys の名前（`{ENTER}`・`{ESC}`・`^{ENTER}`・`^{END}`、
    Space は ` ` そのもの）、`sleepN` で待つ、`@x,y` でクリック。1100 幅で撮ると画像は 1100×800 ほど。
  - 書く操作を試すときは、使い捨てのリポ（scratchpad で `git init`、push を試すなら `git init --bare` をリモートに）で。
  - 撮ったあと `michi.exe`・`git.exe` が残っていないか見る（閉じる途中の数秒は残って見える。ほかのセッションの git は触らない）。
- 大きな履歴は `scripts/make-history.py` で作り、`MICHI_BENCH_REPO=<そのフォルダ> cargo test --release -p michi big_history -- --ignored --nocapture`。
- 単体テストは本物のリポジトリに触らない（`michi-core/src/testrepo.rs` の `TempRepo`）。
- ito の rev は a54872d2309c5a82df69c851cecf20e17e20a28c（ito v0.36.0）。
- egui 0.36 の `TextEdit` は、`.frame()` を自分で付けると `margin` が効かない。余白は Frame の `inner_margin` に書く。
- `scripts/verify.sh` は手元の clippy に加えて、もう一方の CI のターゲット（Windows では Linux）の clippy も回す（acd02a9 から）。
- tsumugi・yagura・kura・lane の worktree のファイルは直さない。tsumugi で michi を開いて試さない（git の掃除が走って重くなった）。

## 決まっていること（会話でだけ決めたもの）

- 1 つの窓が 1 つのリポ（全部の worktree）。樹形図の左端は main。ライセンスは MIT / Apache-2.0。ロゴは A 案。ロゴの横に「道」を出さない。
- lane の PR（`test/win-*` など）は畳む。
- stash の行のクリックは stash の段まで何もしない（stash のコミットは樹形図に無い）。
- U+FFFD は今の字体に無く □ で出る。注記と差分で同じなので、そのままにした。
