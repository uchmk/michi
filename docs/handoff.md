# 引き継ぎ

前の会話から次の会話への引き継ぎ。新しいセッションは `C:\dev\michi` で始め、CLAUDE.md・TODO.md・この文書の順に読む。
設計の正は [design.md](design.md) とモック。大きなファイル（design.md・graph_ui.rs・wt_ui.rs など）は全部読まず、要るところだけ読む。

## いまの状態（v0.109.0、2026-11-22）

- 段はすべて済み。TODO.md の項目は全部 `[x]`。版ごとの中身は CHANGELOG.md と `git log`。
- 最初のリリースは v0.108.2、最新は v0.108.4。
- 実機の確認は #137 に集めた（TESTING.md の `[ ]` の行を全部回す。以前の版ごとの needs-pc の issue はこれに替えた）。
- 持ち主への質問（回答待ち。推奨で進めてよい）: #101 文字の範囲選択でコピー（ito-select）・#103 PR / CI の状態を minato から受ける・
  #123 マウスだけで全部の操作に届くか・#132 Git flow の画面（`Ctrl+Shift+G`）とキー・#134 G のフォームを PR の更新にも使う。
  方針の issue: #133（キーの作り直し 段 2）。

## どこに何があるか

- `crates/michi-core`: egui に依らない git のロジック（単体テスト付き）。
  履歴の読み `history.rs`・`log.rs`、ステージ `stage.rs`、fetch / pull / push `net.rs`、衝突 `conflict.rs`、rebase `rebase.rs`、
  退避 `backup.rs`、片付け `tidy.rs`、ブランチの差分 `review.rs`、衝突の予測 `predict.rs`、worktree `worktree.rs`、テスト用のリポ `testrepo.rs`。
- `crates/michi`: 状態と描画。
  書く処理は `act.rs` の `Task`（ワーカースレッド、Claude がいれば `work.rs` の `Band` で聞く）、読むスレッドは `repo.rs`、
  描画は `*_ui.rs`（`graph_ui.rs`・`wt_ui.rs`・`changes_ui.rs`・`conflict_ui.rs`・`review_ui.rs` など）、
  選ぶ窓は `picker.rs`、コマンドのパレットは `cmdpalette.rs`、入口は `main.rs`。
- `scripts/`: `verify.sh`（push 前）・`shot.ps1`（画面の撮影）・`release-check.sh`・`make-history.py`。

## 次にやること

1. `question` の issue は回答が来るまで推奨で進める（持ち主のコメント `回答: …` がいつも優先）。
2. 実機の確認は PC のセッションが #137 で回す。
3. ito に頼んだもの（`crate-request`、TODO.md の記述を参照）が ito に出たら、rev を上げて置き換える。

## 進め方

- 実装は dev-orchestra の流れ（Sonnet の subagent が実装、`audit`・`verify`、コミットと push は main のセッションだけ）。
- 速さを優先する。2〜3 の機能を 1 つの版にまとめる。作業中は `cargo test -p michi -p michi-core` で、`scripts/verify.sh` は push の前に 1 回。
- 確かめ終えたものは聞かずにコミットして main に push してよい。短い日本語の途中経過をこまめに書く。
- 会話が長くなって要約が増えたら、ここを書き直して新しいセッションに移る。

## 道具のメモ

- ログはファイルに出して末尾だけ読む: `cargo test -p michi -p michi-core > "$TEMP/claude/t.log" 2>&1` のあと
  `test result|FAILED|panicked|^error` を探す。verify も `bash scripts/verify.sh > "$TEMP/claude/v.log" 2>&1; echo rc=$?; tail -2 …`
  （`| tail` の後ろに push を繋げない。版を上げたら `cargo build` で Cargo.lock も合わせる）。
- この PC の Bash は rtk のフックが通る。生の出力が要るときは `rtk proxy cargo …`。heredoc や sed の `\` は崩れることがあるので、
  込み入った書き換えは Edit か、`newline=''` の Python のスクリプトで。
- 画面の確認は `scripts/shot.ps1`（PowerShell。先に `cargo build -p michi`。`-Keys` の書き方はファイルの頭。ほかの窓が前に来たら止まる）。
  - 例: `& .\scripts\shot.ps1 -Exe R:\cargo-target\michi\debug\michi.exe -Arg <リポ> -Out <scratchpad>\shots\x -Width 1100 -Keys "2;j|l;v|j| "`。
  - `;` で区切った組ごとに 1 枚。最初の組の前が `x-a.png`、組 1〜5 が `x-b`〜`x-f`。キーは SendKeys の名前（`{ENTER}`・`{ESC}`・`^{ENTER}`・`^{END}`、
    Space は ` ` そのもの）、`sleepN` で N 秒待つ、`@x,y` でクリック。1100 幅で撮ると画像は 1100×800 ほど。
  - 書く操作を試すときは、使い捨てのリポ（scratchpad で `git init`、push を試すなら `git init --bare` をリモートに）で。
  - shot.ps1 は窓の真ん中・上から 200 をクリックして前に出すので、樹形図では 2 行目が選ばれた状態から始まる（michi の誤りではない）。
  - 衝突の試しのリポは #23 のスクリプトで作れる（`r` がマージ中、`w` が cherry-pick 中、`x` がきれいな `feat`）。Bash の道具から回す
    （PowerShell の `bash` は別物で、何もせずに終わった）。
  - 撮ったあと `michi.exe`・`git.exe` が残っていないか見る（閉じる途中の数秒は残って見える。ほかのセッションの git は触らない）。
- **リリース**: `v*` のタグを push すると `release.yml` が走る。**`release.yml` の成功で終わりにしない。** `bash scripts/release-check.sh vX.Y.Z` が `RELEASE OK`（成果物 6 つ・SHA-256 の表に 6 行・ノート）になるまでが完了。表を見ていないまま「完了」と報告しない。見られなければ「未確認」と理由を書く。
- 大きな履歴は `scripts/make-history.py` で作り、`MICHI_BENCH_REPO=<そのフォルダ> cargo test --release -p michi big_history -- --ignored --nocapture`。
- 単体テストは本物のリポジトリに触らない（`michi-core/src/testrepo.rs` の `TempRepo`）。
- ito は全部のクレートを同じ rev にそろえる。rev は `Cargo.toml` を見る。
- egui 0.36 の `TextEdit` は、`.frame()` を自分で付けると `margin` が効かない。余白は Frame の `inner_margin` に書く。
- `scripts/verify.sh` は手元の clippy に加えて、もう一方の CI のターゲット（Windows では Linux）の clippy も回す（acd02a9 から）。
- tsumugi・yagura・kura・lane の worktree のファイルは直さない。tsumugi で michi を開いて試さない（git の掃除が走って重くなった）。
- 動いている `michi.exe` があると `cargo build`（`verify.sh` の中も）が失敗する。clippy・test は通る。

## 決まっていること（会話でだけ決めたもの）

- 1 つの窓が 1 つのリポ（全部の worktree）。樹形図の左端は main。ライセンスは MIT / Apache-2.0。ロゴは A 案。ロゴの横に「道」を出さない。
- lane の PR（`test/win-*` など）は畳む。
- stash の行のクリックは stash の段まで何もしない（stash のコミットは樹形図に無い）。
- U+FFFD は今の字体に無く □ で出る。注記と差分で同じなので、そのままにした。
- 予測を gix-merge にするのはやらない（2026-10-16 に測った。`git merge-tree` より 4〜15 倍遅く、hunk の数も git と食い違う）。git のコマンドのままにする。
