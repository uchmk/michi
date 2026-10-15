# michi — Claude 向けメモ

いくつもの Claude Code のセッションがそれぞれの worktree で同時に作業するリポジトリのための、キーボードで扱う Git のクライアント。Rust + egui、Windows 優先。
1 つの窓が 1 つのリポジトリ（全部の worktree を含む）を見せる。姉妹は [tsumugi](https://github.com/uchmk/tsumugi)（ターミナル）・
[kura](https://github.com/uchmk/kura)（ファイル）・[yagura](https://github.com/uchmk/yagura)（見張り）・[minato](https://github.com/uchmk/minato)（GitHub）。
共有のクレート（`ito-*`）は [ito](https://github.com/uchmk/ito) にある。

- **画面と動きの正は [docs/design.md](docs/design.md) と HTML のモック [docs/mock/michi.html](docs/mock/michi.html)**（ブラウザで開いて `?`）。
  持ち主が 2026-10-15 に確定した。変えるときは先に `question` の issue で聞く。v1 でやらないことは [docs/v1-scope.md](docs/v1-scope.md)。
- 最初に頼まれた作業の手順は [docs/dev-prompt.md](docs/dev-prompt.md)。
- モックは `docs/mock/src` で `python build.py` を回して `docs/mock/michi.html` を作り直す（手で `michi.html` を直さない）。
- **uchmk のアプリの共通仕様**（common.toml・設定の画面・テーマ・時計、`ito-common`・`ito-theme`・`ito-prefs`）は
  ito の [docs/common-spec.md](https://github.com/uchmk/ito/blob/main/docs/common-spec.md)。これに従う。
- Cargo の workspace。クレートは `crates/` の下に置く。版はルートの `Cargo.toml` の `[workspace.package]` で 1 つ。
- **push の前に `scripts/verify.sh` を 1 回回す**（骨組みで作る）。最後の行が `ALL OK: …` なら push してよい
  （fmt の check、test、clippy を Linux と `x86_64-pc-windows-msvc` の両方で `-D warnings`）。

## 作業ルール

- 返答は日本語。コード・コメント・コミットメッセージは英語。
- 頼まれるまでコミットしない。
- 作業のブランチに push したら、頼まれなくても `main` にマージして push する（merge コミット。`main` が進んでいれば先にブランチに取り込み、`scripts/verify.sh` を回し直す）。
- 改行は LF。スクリプトで書き換えるときは改行を変えない（Python なら `newline=''`）。
- やることは TODO.md に書く（`【人】` は持ち主の作業）。
- 所見は `finding` の issue、人への確認事項は `question` の issue（背景・止まっている TODO・選択肢（推奨を 1 つ））。
  TODO.md の項目には `（要確認: #N）` を付ける。回答が来るまでは推奨で進めてよい。持ち主のコメント `回答: …` がいつも優先する。
- 持ち主の Windows の PC でしかできないこと（実機の確認・リリース・PC の環境）は、ラベル `needs-pc` の issue で PC のセッションに頼む。
  やること・終わりの条件・版かコミットを、会話の記録を読まなくても進められるように書く。
- **整形は `cargo fmt --all` で揃える。**設定は ito・yagura・minato と同じ `rustfmt.toml`（`max_width = 140`、`use_small_heuristics = "Max"`）。
  CI と verify.sh が `cargo fmt --all --check` で見る。整形だけのコミットは `.git-blame-ignore-revs` に書く。
- clippy は `--all-targets -- -D warnings` で警告ゼロを保つ。検証は CI と同じ stable で回す。

## 共有のクレート（ito）

- `ito-* = { git = "https://github.com/uchmk/ito", rev = "…" }` で読む。**全部のクレートを同じ `rev` にそろえる。**
  使えそうなもの: `ito-common`・`ito-theme`・`ito-prefs`（設定）、`ito-i18n`（言語）、`ito-keys`・`ito-keymap`（キー）、`ito-palette`（パレット）、
  `ito-match`（あいまい検索）、`ito-notify`（トースト）、`ito-winres`（exe のアイコン）、`ito-testcheck`（TESTING の表）。
  樹形図の描画は michi が ito に `ito-graph` として作る（design.md の「ito-graph」）。
- 足りないものや直しは ito に入れて push し、ここの `rev` を上げる（`cargo build` で `Cargo.lock` も合わせる）。ito のファイルは ito の CLAUDE.md に従う。
- 手元で一緒に直すときは `rev` を書き換えずにローカルの ito を差し込む（クレートごとに 1 つ `--config`）:

  ```sh
  cargo build --config 'patch."https://github.com/uchmk/ito".ito-graph.path="../ito/crates/ito-graph"'
  ```

## 実機のテスト（TESTING.md）

- 画面の無い環境では確かめられないもの（窓・キー・トースト・見た目・ファイルの監視の速さ）は TESTING.md の表に積む。
  **機能を足したら、そこに行を足してから完了にする。**動きを変えた行は印を外して（`[ ]` に戻して）から作り直す。
- `[x]` は実機で確かめた印で、付けてよいのは実機のセッションか持ち主だけ。見た目の行は `[~]`（画面の画像で判断した）まで。
- 文字で確かめられるもの（履歴の読み・樹形図のレーン・PR の畳み・衝突の予測・`tsumugi ls --json` の解釈・worktree への割り当て）は単体テストにして CI に回す。
  テストの Git のリポジトリはテストの中で一時のフォルダに `git init` して作る（決まった作者・日時、持ち主の設定を読まない）。本物のリポジトリや GitHub に触らない。

## 版と変更ログ（コードが入ったら）

tsumugi と同じ。`Cargo.toml` の `version` が正、版の繰り上げと CHANGELOG.md（日本語、Keep a Changelog、日付は JST）はセットでコミットする。
`main` への直接の push を許す。マージは merge コミットで行う（rebase / squash を使わない）。

## 設計の約束事

- **ディスクやプロセスに触る処理は UI スレッドで実行しない**（ワーカースレッドとチャネル、古い依頼は捨てる）。git の呼び出しも同じ。
- **読むための git は `GIT_OPTIONAL_LOCKS=0`・`GIT_TERMINAL_PROMPT=0` で呼ぶ。**作業中の Claude の index と HEAD を、確かめずに動かさない
  （design.md の「作業中の Claude とぶつからない」）。
- **取り返せるようにする。**reset・ブランチを消す・変更を捨てる・rebase の前に `refs/michi/backup/<日時>` に退避し、確かめの窓に何が失われるかを書く。
- **描画とロジックを分ける**: Git の読み・樹形図の行・畳み・予測・tsumugi の JSON の解釈は egui に依らない `michi-core` に置き、単体テストを付ける。
- **マルチ OS**: Windows・macOS（Cmd キー）・Linux の違いは `#[cfg(...)]` のモジュールに閉じ込める。修飾キーは macOS では Cmd、ほかは Ctrl。
  パスの比べ方（Windows の大文字と区切り）も 1 か所にまとめる。
- **マルチアーキテクチャ**: x86_64 と ARM64 の 6 ターゲットへのクロスコンパイルを軽く保つ。Pure Rust のクレートを優先する（gix、libgit2 を使わない）。
- **スクロールバーは樹形図だけ。**ほかは入りきらない行を畳む。
- 色は tsumugi の Dark の系統（ito-theme）。アイコンは `icon.rs` でコードから描き、`build.rs` で exe に焼く（tsumugi・yagura・minato と同じ）。
