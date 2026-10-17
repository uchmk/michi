//! Write `TESTING-CHECKS.md`: the manual half of TESTING.md, with a box to
//! tick per check. Ticks survive regeneration, rows a test names are left out.
//!
//! The generator is ito's `ito-testcheck` (shared with kura and tsumugi); this
//! is michi's configuration of it: the paths and the preamble.
//!
//!     cargo run -p michi --example make-testcheck [-- --lane macos|linux] [--check | --stats]
//!
//! `--check` writes nothing and exits 1 when the file is out of date. The file
//! keeps `[~]` (an appearance row an agent judged from a screenshot, not done
//! until the owner looks).

use ito_testcheck::testcheck::{run, Config, Counts, Lane};

fn main() {
    run(&Config {
        source: "TESTING.md",
        notes: "scripts/testcheck-ja.toml",
        src_dirs: &["crates"],
        dep_crates: &[],
        release_list: None,
        command: "cargo run -p michi --example make-testcheck",
        lanes: &[
            Lane { name: "windows", out: "TESTING-CHECKS.md", marks: &['~'], setup: true, check_release: false, preamble },
            Lane { name: "macos", out: "TESTING-MACOS.md", marks: &['-'], setup: false, check_release: false, preamble: macos },
            Lane { name: "linux", out: "TESTING-LINUX.md", marks: &['-'], setup: false, check_release: false, preamble: linux },
        ],
    });
}

fn preamble(_: &Counts) -> String {
    "# 実機チェックリスト

`TESTING.md` から `cargo run -p michi --example make-testcheck` で生成している。**正は TESTING.md** で、
このファイルはその表を 1 行ずつ印を付けられる形に並べたもの。食い違ったら TESTING.md を信じること。

**チェック（`[x]` と `[~]`）だけは手で書いてよく、生成し直しても残る。**それ以外を書き換えても次の生成で消える。
`[x]` は実機で確かめた印、`[~]` は見た目の行を画面の画像で判断した印（済みには数えない。持ち主が同じ画像を見て `[x]` にする）。
**件数はこのファイルに書かない**（印を付けた PR が並ぶと必ずぶつかるため）。`-- --stats` で出る。

付ける前に「失敗していたら、画面かディスクの何が違ったはずか」を自問すること。
"
    .to_owned()
}

fn macos(_: &Counts) -> String {
    other_os("macOS", "macos", "TESTING-MACOS.md", "Cmd キーの行（Ctrl の代わりに Cmd）は macOS で押す。")
}

fn linux(_: &Counts) -> String {
    other_os("Linux", "linux", "TESTING-LINUX.md", "X11 と Wayland のどちらでも確かめられるなら、どちらかを PR に書く。")
}

fn other_os(name: &str, lane: &str, file: &str, extra: &str) -> String {
    format!(
        "# {name} チェックリスト

`TESTING.md` から `cargo run -p michi --example make-testcheck -- --lane {lane}` で生成している（`{file}`）。
**正は TESTING.md**。このファイルは {name} で押す側の並べ替えで、Windows 実機の `TESTING-CHECKS.md` とは別に数える。

**チェック（`[x]` と `[-]`）だけは手で書いてよく、生成し直しても残る。**

- `[x]`: {name} で操作し、期待値を読めるテキストかファイルの状態で確かめた。
- `[-]`: {name} では対象外（Windows だけの行など）。理由は PR に書く。
- 見た目の行（色、滑らかさ、フォント）は、実機の目で見た人だけが付ける。
- {extra}
"
    )
}
