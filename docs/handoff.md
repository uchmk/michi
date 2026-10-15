# 引き継ぎ

前の会話から次の会話への引き継ぎ。新しいセッションは `C:\dev\michi` で始め、CLAUDE.md・TODO.md・この文書の順に読む。
設計の正は [design.md](design.md) とモック。大きなファイル（design.md・graph.rs・wt_ui.rs など）は全部読まず、要るところだけ読む。

## いまの版（v0.8.0、2026-10-15）

段 1「見る」はほぼ終わり。入っているもの:

- 樹形図（`ito-graph`、main が左端、lane の PR を畳む `z`、`/` の絞り込み、スクロールバーの印）。10 万コミットで最初の画面 0.1 s、
  畳む 3.5 ms、絞る 1 文字 4〜7 ms。commit-graph が無ければ裏で書く（#2）。
- 左の列（worktree・ブランチ・リモート・タグ・stash、`↑N` `↓N`）、コミットの詳細と差分（1 列と横並び、長い行は折り返す、UTF-8 でない注記）。
- WIP の行（worktree ごとの未コミットの変更）と、その worktree の Claude（`tsumugi ls --json`）。
- worktree の表（`3`、960 幅より狭いとカード）と衝突の予測（worktree どうしと trunk の組、前にいるあいだ 5 秒ごと）。
- ファイルの監視（`notify`）と未読の印（`%LOCALAPPDATA%\uchmk\michi\seen\`）。

実機の確認は needs-pc の issue #1, #3〜#9 で PC のセッションに頼んである（まだ印は無い）。

## 次にやること

1. 段 1 の残り: 衝突の予測を、worktree に出ていないローカルのブランチと main の組にも広げる（`michi-core/src/predict.rs`・`clash.rs`）。
   マージ中の文言は段 3。
2. モックの 2 回目はやらない（持ち主が 2026-10-15 に「実装を進めて」）。段 2 で決めることは `question` の issue に文字で書き、推奨で進める。
3. 段 2: まずブランチ全体の差分（`d`。見た印・前回から・メモ）、次にステージとコミット、ブランチ、fetch / pull / push、日々の操作
   （design.md の「日々の操作」）、`refs/michi/backup/<時刻>` への退避。
4. 段 3〜6 は TODO.md のとおり。段 4 は tsumugi の `new --worktree` と `focus` が要る（tsumugi の TODO.md に書いてある）。

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
