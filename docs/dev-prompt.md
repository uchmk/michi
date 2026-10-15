# 開発のセッションへの指示

michi の開発を始めるセッション（クラウドの Claude Code）に貼る文。持ち主が 2026-10-15 に設計を確定したときに書いた。

---

uchmk/michi で、Git のデスクトップのクライアント michi を作り始めてください。

## まず読むもの

1. `CLAUDE.md`（作業ルール。返答は日本語、コード・コメント・コミットは英語、LF、`cargo fmt --all` で整形、push の前に `scripts/verify.sh`）
2. `docs/design.md`（確定した設計。画面・キー・樹形図の描き方・PR を畳む・worktree の表・衝突の解消・gix と git の分担）と `docs/v1-scope.md`
3. `docs/mock/michi.html`（動く画面イメージ。ブラウザで開いて窓を押し、`?`。上の「場面」で樹形図・worktree・衝突の解消・ロゴを切り替える。
   迷ったらこれと同じ動きにする。作りは `docs/mock/src/app.js`）
4. 姉妹の作りを見本にする: minato（いちばん新しく骨組みから作ったもの。`docs/dev-prompt.md` と最初のコミットの並び）、
   tsumugi（`crates/tsumugi/src/icon.rs`・`build.rs`・`scripts/verify.sh`・`.github/workflows/`・`.github/ISSUE_TEMPLATE/`・TESTING.md の仕組み）、
   kura（アイコンの `.ico` の作り方）、共有のクレートの ito（`docs/common-spec.md` と各 crate の README、ito の CLAUDE.md）。

## 進め方

下の段ごとに、動くところまで作って `scripts/verify.sh` が `ALL OK` になったら、版を上げ CHANGELOG.md を書いてコミットし、main に push してください
（このリポジトリは push を頼まなくてよい）。段の途中でも、区切りがよければ小さく push してかまいません。
画面の要る確認は TESTING.md に行を足し、持ち主の PC での確認が要るときは `needs-pc` の issue を立ててください。
設計にない決めごとが出たら `question` の issue に推奨を書いて、推奨で先に進めてください。やることは TODO.md に積んでください。

テストの Git のリポジトリは、テストの中で一時のフォルダに `git init` して作ってください（決まった作者・日時で、`GIT_CONFIG_GLOBAL` などで
持ち主の設定を読まない）。本物のリポジトリや GitHub にテストから触らないでください。

### 0. 骨組み（v0.1.0）

- Cargo の workspace、`crates/michi`（bin）と `crates/michi-core`（Git の読み・樹形図の行の組み立て・畳み・予測・tsumugi の JSON の解釈。egui に依らない）。
- ito の `ito-common`・`ito-theme`・`ito-prefs`・`ito-i18n`・`ito-keys`・`ito-keymap`・`ito-palette`・`ito-notify`・`ito-winres`・`ito-testcheck` から
  要るものを同じ `rev` で読む（tsumugi が今使っている `rev` に合わせる）。
- egui の窓: タイトルバー（ロゴ・`michi`・ビューのタブ・リポジトリの札・パレットの欄）、空のビュー（`1` `2` `3` `,`）、keybar（時計つき）、
  `?` のキー一覧。設定の画面は ito の共通のもの。窓は 1280×800 で開く。
- 起動の引数 `michi <path>`（無ければ今のフォルダ）でリポジトリを開き、タイトルバーの札に名前・今のブランチ・worktree の数を出す
  （`git worktree list --porcelain` をワーカースレッドで）。git の版を確かめ、2.38 より古ければ知らせる。
- `icon.rs` で `assets/icon.svg` の絵をコードから描き、窓のアイコンにする。`build.rs` で同じ絵を `ito-winres` で exe に焼く（tsumugi と同じ。`.ico` は持たない）。
- `rustfmt.toml`（`max_width = 140`、`use_small_heuristics = "Max"`）、`scripts/verify.sh`（fmt の check・test・clippy `-D warnings` を Linux と
  `x86_64-pc-windows-msvc`）、CI（`ci.yml`・`arm64.yml`・`checklists.yml`、release は後でよい）、`.gitattributes`、
  issue のフォーム（finding・question・needs-pc・retest）とラベル（`scripts/labels.sh`）、TESTING.md と make-testcheck、TODO.md、CHANGELOG.md。
- README の Status を書き直す。

### 1. 見る（読むだけ）

- **ito-graph** を ito に作る（design.md の「ito-graph」）。レーン・色・曲線の経路の計算は egui に依らない層で単体テストを付け、描画は feature の egui 層。
  ito に push して `rev` を上げる。
- 樹形図: 全部の ref と worktree の HEAD から履歴を読み（まずは `git log --format=…` で、後で gix に置き換えてよい）、仮想スクロールで描く。
  左端は main、色の規則、札、`⧉`、WIP の行、見ていない印、スクロールバーの印、`/` の絞り、「… さらに古い N 件」。
- **PR を畳む**（`z`、既定で入、`l` `h`、「▸ N」、詳しくの「このマージで main に入った N コミット」、飛ぶと開く）。畳みの計算は michi-core の関数にし、
  画面イメージと同じ規則（いつも出すもの）を単体テストで確かめる。
- 左の列（worktree・ブランチの束・リモート・タグ・stash）と、詳しく（コミット・ファイル・差分、`Enter` で横並び）。単語単位の強調（similar）。
- worktree の表（`3`）: 列・操作のボタン（段階 1 では押すと「段階 N で入ります」のトーストでよい）・960 のカード。
- Claude の列: `tsumugi ls --json` を 2 秒（後ろなら 10 秒）ごと。JSON の解釈と、`cwd` から worktree への割り当て（Windows の大文字と区切り）は単体テスト。
  tsumugi が無ければ列を消す。
- 衝突の予測: `git merge-tree --write-tree` で 2 本ずつ（後で gix-merge に）。未コミットの変更は一時の index（`GIT_INDEX_FILE`）で作業フォルダの tree を作って数える。
  ブランチも本当の index も動かさない。計算し直す時（fetch のあと・HEAD が動いたとき・作業フォルダの変化から 2 秒）。表と `⚠`・点線の枠。
- 外からの変化はファイルの監視（notify）で拾い、300 ms 静かになったら読み直す。読むための git は全部 `GIT_OPTIONAL_LOCKS=0`。
- 見ていないコミットの印（状態のフォルダに置く）。
- 段階 1 のあいだに速さを測る（10 万コミットのリポジトリで最初の画面まで。目標 0.3 秒）。結果は TODO.md か issue に書く。

### 2 回目の画面イメージ（段階 2 の前）

段階 2 に入る前に、`docs/mock/` に「変更とステージ」「ブランチ全体の差分（見た印・前回から・メモ）」「rebase」を描き足し、
`question` の issue で持ち主に見てもらってください（作り方は `docs/mock/src/build.py`。1 回目と同じ見た目と書き方で）。

### 2. 変える

まずブランチ全体の差分（`d`。2 回目の画面イメージのとおり。範囲・未コミットの変更を含める切り替え・見た印・前回から・行のメモ。読むだけ）。
次にステージ・コミット（`space`・`c`、行と hunk のステージ、Claude が書いている途中の確かめ）・ブランチ（`b`）・fetch/pull/push（進み具合と取り消し）。
design.md の「日々の基本の操作」の表（amend・revert・タグ・stash・変更を捨てる・片付け）。破壊的な操作の前の退避（`refs/michi/backup/<日時>`）。

### 3. 衝突

衝突の解消の画面（`4`）: 3 方向・意図の行・結果の欄・ボタン・`]` `[`・rerere（最初に聞いて `rerere.enabled`）・このファイルを最初から・中止・コミットの欄、
マージ中に止める操作。cherry-pick（衝突したらこの画面）。予測を gix-merge に置き換え、`git merge-tree --write-tree` と突き合わせるテストを付ける。

### 4. worktree

新しい作業（`N`）・消す（`x`）・タブへ（`o`）。tsumugi に `new --worktree BRANCH --from REF` と `focus N|NAME` が要る（tsumugi の TODO.md にある）。
無ければ tsumugi のリポジトリで足すか、`needs-pc` / `question` の issue で頼んでください。main を取り込む（`m`）。

### 5. rebase

2 回目の画面イメージのとおり。並べ替え・まとめる・題を直す・途中状態の帯。

### 6. Claude

衝突を Claude に渡す（ファイルに書き出して `tsumugi send`、監視で残りを減らす、待ち・済みで知らせる）。ブランチ全体の差分に付けたメモを渡す。

## 守ること

- UI スレッドでディスクとプロセスに触らない（ワーカースレッドとチャネル、古い依頼は捨てる）。
- 読むための git は `GIT_OPTIONAL_LOCKS=0`、`GIT_TERMINAL_PROMPT=0`。Claude が作業中の worktree の index と HEAD を、確かめずに動かさない。
- 破壊的な操作の前に退避し、確かめの窓に何が失われるかを書く。
- スクロールバーは樹形図だけ。ほかは畳む。色は ito-theme。ロゴの横に「道」の字を出さない。
- ito に足りないものは ito に入れて push し、`rev` を上げる（ito の CLAUDE.md に従う）。
- 実機の印 `[x]` は付けない（持ち主か実機のセッションだけ）。
