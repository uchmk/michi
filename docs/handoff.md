# 引き継ぎ

前の会話から次の会話への引き継ぎ。新しいセッションは `C:\dev\michi` で始め、CLAUDE.md・TODO.md・この文書の順に読む。
設計の正は [design.md](design.md) とモック。大きなファイル（design.md・graph.rs・wt_ui.rs など）は全部読まず、要るところだけ読む。

## いまの版（v0.14.0、2026-10-16）

段 1「見る」は終わり、段 2「変える」は TODO.md の 2. のうち、残りが後回しにしたもの（各 question の issue の「後回し」）だけになった。入っているもの:

- 樹形図（`ito-graph`、main が左端、lane の PR を畳む `z`、`/` の絞り込み、スクロールバーの印）。10 万コミットで最初の画面 0.1 s、
  畳む 3.5 ms、絞る 1 文字 4〜7 ms。commit-graph が無ければ裏で書く（#2）。
- 左の列（worktree・ブランチ・リモート・タグ・stash、`↑N` `↓N`）、コミットの詳細と差分、WIP の行とその worktree の Claude（`tsumugi ls --json`）。
- worktree の表（`3`）と衝突の予測、ファイルの監視と未読の印、worktree の無いブランチと main の衝突の予測。
- ブランチ全体の差分（`3` の `d`、v0.9.0）。ロジックは `michi-core/src/review.rs`、状態は `michi/src/review.rs`、描画は `review_ui.rs`。
- ステージとコミット（`2`、v0.10.0）。ロジックは `michi-core/src/stage.rs`、状態とテストは `michi/src/changes.rs`、描画は `changes_ui.rs`。
  v0.14.0 で hunk の分割（`s`、`Diff::split`）と、2 行以上で断られた書き込みの出力を差分の代わりに出す（`changes.told`、`git::refused_all` が色を落とす）。
- fetch / pull / push（`f` `p` `P`、v0.11.0）: 進み具合とキーのバーの Esc、`--force-with-lease` の前の帯と退避。`michi-core/src/net.rs`・`michi/src/work.rs`。
- ブランチ（`b`）・タグ（`T`）・revert（`t`）のパレット（v0.12.0）、stash（`s`）と変更を捨てる（`2` の `d`）（v0.13.0）。
  消す・捨てる前は `michi-core/src/backup.rs` で `refs/michi/backup/<時刻>` に退避し、トーストに出す。
- 片付け（`3` の `c`、v0.14.0）: 取り込み済み・リモートで消えたブランチとフォルダの無い worktree を選んでまとめて消す。
  ロジックは `michi-core/src/tidy.rs`、状態は `michi/src/tidy.rs`、描画は `tidy_ui.rs`。
- 書く処理は `michi/src/act.rs` の `Task`（ワーカーで実行、Claude がいれば `work.rs` の `Band` で聞く）。読むスレッドは `repo.rs`。

実機の確認は needs-pc の issue（#1, #3〜#9, #11, #13, #14, #17, #19, #21）で PC のセッションに頼んである。
推奨で決めたことの `question` は #10（差分のレビュー）・#12（`2`）・#15（fetch / pull / push）・#16（ブランチ・タグ・revert）・
#18（stash・捨てる）・#20（片付け・分割・hook の出力）。どれもまだ回答は無い。

## 次にやること

1. モックの 2 回目はやらない（持ち主が 2026-10-15 に「実装を進めて」）。決めることは `question` の issue に文字で書き、推奨で進める。
2. 段 2 の後回し（TODO.md の 2. の `[ ]`）: レビューのメモ・マージ・PR（#10）、pull をマージの途中で止める・push 先のリモートを選ぶ（#15）、
   ブランチの名前の変更・upstream・merge / rebase / cherry-pick（#16、段 3 の衝突の画面と一緒に）。
3. 段 3「衝突」（衝突の画面 `4`、マージ中のロック、rerere、予測を gix-merge に）から。段 4〜6 は TODO.md のとおり。
   段 4 は tsumugi の `new --worktree` と `focus` が要る（tsumugi の TODO.md に書いてある）。

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
- 画面の確認は `scripts/shot.ps1`（PowerShell。先に `cargo build -p michi`。`-Keys` の書き方はファイルの頭。ほかの窓が前に来たら止まる）。
  - 例: `& .\scripts\shot.ps1 -Exe R:\cargo-target\michi\debug\michi.exe -Arg <リポ> -Out <scratchpad>\shots\x -Width 1100 -Keys "2;j|l;v|j| "`。
  - `;` で区切った組ごとに 1 枚。最初の組の前が `x-a.png`、組 1〜5 が `x-b`〜`x-f`。キーは SendKeys の名前（`{ENTER}`・`{ESC}`・`^{ENTER}`・`^{END}`、
    Space は ` ` そのもの）、`sleepN` で N 秒待つ、`@x,y` でクリック。1100 幅で撮ると画像は 1100×800 ほど。
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
