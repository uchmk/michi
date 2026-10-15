# michi: real-machine checks

Things a headless machine cannot see: the window, the title bar, the icon, the keys, the clock and the look.
`TESTING-CHECKS.md` is generated from the tables below (`cargo run -p michi --example make-testcheck`); this file is the source of truth.
`[x]` is set only by the owner or a real-machine session; `[~]` means judged from a screenshot.

Rows whose behaviour changes lose their mark before the list is regenerated.

What text can check (reading the repository, the git version, the command line) is in the unit tests, not here.

## 1. Window and title bar

| # | Do | Expect |
| --- | --- | --- |
| 1.1 | Start `michi C:\dev\tsumugi` (any repository) | A 1280x800 window with no system title bar; the logo, `michi` (never 道), the tabs `1 Graph` `2 Changes` `3 Worktrees` hanging from the bar's foot, the repository chip, a palette field, the `,` button and three window buttons on the right |
| 1.2 | Look at the chip | The repository's name in the accent colour, a drawn branch mark (a trunk and a twig with three dots, not a box), then `main · worktree N` with the real count; hovering shows the repository's top folder |
| 1.3 | Look at the window title (Alt+Tab, the taskbar) | `tsumugi — michi` once the repository is read |
| 1.4 | Drag the empty part of the title bar | The window moves; double-click maximises and restores |
| 1.5 | Click minimise, maximise, close | Each does what it says; close shows a red button under the pointer; the maximise glyph turns into two squares while maximised |
| 1.6 | Drag each edge and corner | The window resizes with the matching pointer; nothing resizes while maximised |
| 1.7 | Shrink the window to its smallest | It stops at 720x420; the palette field shortens, then the chip drops the branch, then the chip goes; the tabs never overlap the buttons |
| 1.8 | Look at michi.exe in Explorer, the taskbar and Alt+Tab | The same logo as in the title bar (the forked path on the tile), sharp at each size |
| 1.9 | Start michi in a folder with a Japanese name, or on a branch with one | The Japanese draws (after a frame or two), not boxes |

## 2. Views and keys

| # | Do | Expect |
| --- | --- | --- |
| 2.1 | Press `1`, `2`, `3` | The tab in force moves and opens into the view; each view says what will be there and "Coming in a later stage" |
| 2.2 | Click each tab | Same as the digit |
| 2.3 | Press `?` | A card with the keys: "Everywhere" first, then the view's; the tab stays lit under it; `?` or Esc closes it |
| 2.4 | Press `N`, `f`, `p`, `P` in any view, `z` and Enter in the graph, Space and `c` in the changes | A toast at the foot names the key's work and says it comes in a later stage; it goes after 4 seconds |
| 2.5 | Press Ctrl+Shift+P, or click the palette field | The same toast for the command search; the field lights under the pointer with a text cursor |
| 2.6 | Press a key with no work (`q`, `4`) | Nothing happens |

## 3. Keybar and clock

| # | Do | Expect |
| --- | --- | --- |
| 3.1 | Look at the keybar in each view | The view's first keys (the first one lit in the accent colour), then `? Keys`; nothing overlaps the clock when the window is narrow |
| 3.2 | Look at the right of the keybar | The clock as common.toml says (time, date, weekday); it turns over on the minute |
| 3.3 | Turn the clock off in the settings | It goes from the keybar, in every uchmk app that is open |

## 4. Settings

| # | Do | Expect |
| --- | --- | --- |
| 4.1 | Press `,` | The settings open between the title bar and the keybar; the title bar stays, the `,` button is lit and no tab is lit; the keybar shows `Esc Back` and `Ctrl+Tab Next page` |
| 4.2 | Press Ctrl+Tab | The page moves from General to Theme, with a small michi preview (the three tabs, two branches) |
| 4.3 | Change the language to 日本語 | Every word of michi turns Japanese at once (the tabs 樹形図・変更・worktree, the keybar, the empty views), and other uchmk apps follow |
| 4.4 | Change the theme, and the size | The whole window follows at once; the logo and the chip stay readable in light and dark |
| 4.5 | Press Esc, click the `,` button, or click a tab | Each leaves the settings; a tab opens its view |
| 4.6 | Edit common.toml by hand while michi is open | michi follows within a second |

## 5. git and errors

| # | Do | Expect |
| --- | --- | --- |
| 5.1 | Start `michi C:\dev` (not a repository) | The chip shows `✗ Not a Git repository` in red and a toast says so with the folder; the title stays `michi` |
| 5.2 | Start `michi C:\no\such` from a terminal | The message `michi: not a folder: C:\no\such` in the terminal, exit code 2, no window |
| 5.3 | Start `michi --version` and `michi --help` from cmd and from PowerShell | The version or the usage prints in that terminal, exit code 0, no window |
| 5.4 | Start michi with no git on the PATH | The chip shows `✗ git was not found`; the keybar shows `⚠ git` in red |
| 5.5 | Start michi with a git older than 2.38 on the PATH | A toast for 10 seconds names the version and asks to update; the keybar keeps `⚠ git 2.x.y` in amber |
| 5.6 | Start `michi` in a worktree other than the main one | The chip names the repository (not the worktree's folder) and its branch; the count is the whole repository's |
