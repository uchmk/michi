# TESTING-KEYS — keys on a real machine

Only keys are here: what to press and what happens. Other checks are in TESTING.md.
The design is issue #133 (F1 is the key list, `g` is the popup of dangerous and rare
operations, `y` copies, `?` searches deeply, Tab moves between the panes).
`[x]` is set only by a real-machine session or the owner; `[ ]` is not checked yet.
A row is added with the change that makes the key, and unticked again when the key moves.

## Global

| # | Key | Expect | Checked |
| --- | --- | --- | --- |
| K1.1 | `F1` | The key list opens for the screen you are on | [ ] |
| K1.2 | `Ctrl+Shift+P` | The command palette opens | [ ] |
| K1.3 | `Ctrl+Shift+G` (provisional) | The Git flow sheet opens; Esc closes it | [ ] |
| K1.4 | `Ctrl+O` or `Ctrl+T` | The repository switcher opens | [ ] |
| K1.5 | `Alt+1` … `Alt+9` | The tab goes to that number | [ ] |
| K1.6 | `Ctrl+Z` / `Ctrl+Shift+Z` | The last reset, branch delete or rename is undone / redone | [ ] |

## Confirm band

| # | Key | Expect | Checked |
| --- | --- | --- | --- |
| K2.1 | `Ctrl+Enter` | The question goes ahead (not for a band that asks for a typed name) | [ ] |
| K2.2 | `y` / `n` / `Esc` | Go ahead / not now / not now | [ ] |

## To be filled as the redesign lands (issue #133)

- `g` popup: `g r` reset, `g d` discard, `g f` force push, `g x` remove worktree, `g a` amend, `g c` tidy, `g l` land, `g m` rebase on main, `g v` move changes, `g b` bisect, `g h` hand to Claude, `g ?` sheet, `g u` unstage, `g g` top.
- `y` copy: `y h` hash, `y p` patch, `y s` file names, `y m` message, `y b` branch, `y f` path, `y d` folder, `y u` URL, `y a` author.
- `Space` marks up to two commits, `D` compares, `Esc` clears.
- `?` deep search (Tab changes the mode); `/` stays the light filter.
- Panes: `Tab` / `Shift+Tab` move between the side column, the tree and the detail pane; `Enter` in the detail pane widens it.
