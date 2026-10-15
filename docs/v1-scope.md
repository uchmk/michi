# v1 の範囲

v1（[dev-prompt.md](dev-prompt.md) の段階 1〜6）で作るものは [design.md](design.md)。ここには、v1 で**やらない**ことを書く（2026-10-15 に決めた）。
やらないものに触れるリポジトリでも、michi は止まらずに読めるだけ読み、その所は「michi では扱いません」と出して端末か外部ツールに渡す。

## やらないこと

- **submodule**: 樹形図と差分では submodule の行を 1 行（コミットの id の変化）で出すだけ。中には入らない。衝突は「どちらを取るか」だけ。
- **Git LFS**: ポインタのファイルとして見せる。取得・push は git に任せる（`git lfs` が入っていれば git が呼ぶ）。
- **sparse checkout**: 設定の画面も切り替えも作らない。
- **bisect**: 作らない。
- **GitHub 以外のホスト**: 「PR を開く」・リモートの札のリンクは GitHub だけ。fetch・push は git がするので、ほかのホストでも動く。
- **署名の設定**: 鍵の作成や `user.signingkey` の設定は作らない。署名付きのコミットは git が設定どおりに作る（commit は git で作るため）。
- **PR の作成**: michi では PR を作らない。minato かブラウザに渡す。
- **3 方向で表せない衝突の画面**（削除と変更・rename どうし・バイナリ・改行だけの差・モードの変化・submodule）: 「どちらのファイルを取るか」と
  「外部ツールで開く」だけ。

## 後で考えるもの

- gix での fetch（v1 は `git fetch`）。
- tsumugi のサーバに ito-ipc でつないで状態の変化を受け取る形（v1 は `tsumugi ls --json` を間を置いて呼ぶ）。
- kura・minato からの「michi で開く」の受け口を ito に移すこと。
