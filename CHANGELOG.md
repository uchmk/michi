# 変更ログ

形式は [Keep a Changelog](https://keepachangelog.com/ja/1.1.0/) に従う。日付は JST。

## [0.1.0] - 2026-10-15

### 追加

- 骨組み。Cargo の workspace に `crates/michi`（窓）と `crates/michi-core`（git を読む、egui に依らない層）を置いた。ito のクレートは tsumugi と同じ rev で読む。
- 自前のタイトルバー: ロゴ・`michi`・ビューのタブ（`1` 樹形図・`2` 変更・`3` worktree）・リポジトリの札・コマンドの欄・設定の `,`・窓の 3 つのボタン。
  ドラッグで動かし、ダブルクリックで最大化し、縁と角で大きさを変える。
- リポジトリの札: `michi <path>`（無ければ今のフォルダ）のリポジトリの名前・今のブランチ（枝の印は描く）・worktree の数。
  読みはワーカースレッドで `git rev-parse` と `git worktree list --porcelain`。リポジトリでなければ札とトーストで知らせる。窓の題は「<名前> — michi」。
- 3 つのビューは空で、何が入るかを書いた。後の段階のキー（`N`・`f`・`p`・`P`、ビューごとのキー、Ctrl+Shift+P）は「後の段階で入ります」のトーストを出す。
- `?` のキー一覧（どこでも使うキーと、そのビューのキー）。
- keybar: ビューのキー（いちばん勧めるものを強調）と、右に時計（common.toml の `[clock]`）。
- 設定の画面（`,`）は ito の共通のもの（言語・大きさ・時計・テーマ）。タイトルバーと keybar を残したまま、その間に開く。テーマの頁に michi の見本。
- 言語は英語と日本語（common.toml の `language`、`auto` は OS の言語）。日本語の字体と絵文字はシステムのものを後ろで読む。
- git の版を確かめ、2.38 より古ければ 10 秒のトーストと keybar の `⚠ git` で知らせる。git が無ければ札と keybar に出す。
- アイコン: 「合流」の絵を `icon.rs` がコードで描き、窓のアイコンとタイトルバーのロゴにし、`build.rs` が `ito-winres` で exe に焼く。
- コマンドライン: `--version`・`--help`。知らない引数や無いフォルダは理由を出して終了コード 2。
- `scripts/verify.sh`（fmt・test・clippy を Linux と `x86_64-pc-windows-msvc` で）、CI（x64 の Windows と Linux、ARM64、チェック表）、
  issue のフォーム（finding・question・needs-pc・retest）と `scripts/labels.sh`、TESTING.md と `make-testcheck`。
