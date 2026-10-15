# michi

A keyboard-driven Git client for repositories where several Claude Code
sessions work at once, each in its own worktree. One window shows one
repository with all of its worktrees: the commit graph with main on the left
lane, what each worktree and its Claude session is doing, and a screen that
walks through a merge conflict block by block. Rust + egui, Windows first,
with macOS and Linux on x86_64 and ARM64. A sibling of
[tsumugi](https://github.com/uchmk/tsumugi), [kura](https://github.com/uchmk/kura),
[yagura](https://github.com/uchmk/yagura) and [minato](https://github.com/uchmk/minato),
sharing their crates in [ito](https://github.com/uchmk/ito).

**Status:** the design was settled on 2026-10-15 and is written down in
[docs/design.md](docs/design.md), with what v1 leaves out in
[docs/v1-scope.md](docs/v1-scope.md). The screens are on an HTML mock: open
[docs/mock/michi.html](docs/mock/michi.html) in a browser, click the window and
press `?` for the keys. It draws the commit graph (lane PRs folded into their
merge on main, the commit detail and its diff), the worktree table with
conflict prediction and the conflict screen, using the real history of
tsumugi with three made-up Claude sessions on top. The page is built from
[docs/mock/src](docs/mock/src) with `python build.py`. Development follows
[docs/dev-prompt.md](docs/dev-prompt.md) in stages.

v0.1.0 is the skeleton: the window with its own title bar (the view tabs, the
repository chip with the branch and the count of worktrees, the palette
field), the three views still empty, the keybar with the clock, `?` for the
keys and the settings shared by every uchmk app. It checks that git is 2.38
or newer. Nothing in it changes a repository yet.

```sh
cargo run -p michi -- C:\dev\some-repo   # or no path: the current folder
```

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option. The icon in [assets](assets) is not part of that grant; see
[assets/README.md](assets/README.md).
