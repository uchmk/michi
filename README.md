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

**Status:** design. The screens are being agreed on an HTML mock first: open
[docs/mock/michi.html](docs/mock/michi.html) in a browser, click the window and
press `?` for the keys. The first round draws the commit graph (with the
commit detail and its diff), the worktree table and the conflict screen, using
the real history of tsumugi with three made-up Claude sessions on top. The
changes and staging screen, the diff of a whole branch and rebase come in the
next rounds. The page is built from [docs/mock/src](docs/mock/src) with
`python build.py`. No code yet.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at
your option.
