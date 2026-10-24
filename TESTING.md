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
| 1.4 | Drag the empty part of the title bar | The window moves; double-click maximises and restores (skipped: window drag needs mouse drag, not in the harness) |
| 1.5 | Click minimise, maximise, close | Each does what it says; close shows a red button under the pointer; the maximise glyph turns into two squares while maximised (skipped: OS window buttons and hover, not decidable from stills) |
| 1.6 | Drag each edge and corner | The window resizes with the matching pointer; nothing resizes while maximised (skipped: edge drag not in the harness) |
| 1.7 | Shrink the window to its smallest | It stops at 720x420; the palette field shortens, then the chip drops the branch, then the chip goes; the tabs never overlap the buttons |
| 1.8 | Look at michi.exe in Explorer, the taskbar and Alt+Tab | The same logo as in the title bar (the forked path on the tile), sharp at each size (skipped: needs Explorer/taskbar/Alt+Tab, OS shell) |
| 1.9 | Start michi in a folder with a Japanese name, or on a branch with one | The Japanese draws (after a frame or two), not boxes |
| 1.10 | Look at the title bar (height, the palette box, the bell with unread notices, the three window buttons); hover each; drag the empty part, double-click it; drag the window edges; click the bell | The same bar as tsumugi (40 high, box corner 6, buttons 46 wide, the badge in the `wait` colour); hovers say Minimize / Maximize or Restore / Close and the key of the palette; drag and double-click move and maximise; the edges resize (not while maximised); the notice list opens right under the bell | [ ] (skipped: hover tooltips cannot be driven by the harness) |
| 1.11 | Commit in the `2` view, then open the bell; let a merge stop at a conflict; make a git task fail; click a row; press Up/Down/Enter/Esc in the list | Each notice has a colour by kind (done, info, warning, failed), the time ("3m ago") and a mark when unread; clicking the commit row selects that commit in the graph, the conflict row opens the `4` view; a click marks only that row read, "Mark all read" marks the rest; the badge turns to the error colour while a failure is unread | [ ] (skipped 2026-10-22: needs a real merge conflict, a failing git task and unread-badge colour over time; not set up in the harness) |
| 1.12 | Hand a conflict to a Claude (or let one ask a question) with the michi window behind another; click the system's notification, or its Open button | The window comes to the front and the worktree table selects that worktree; the notice is read in the bell (Windows; macOS only shows it) (skipped: needs the OS notification centre) | [ ] (skipped: needs a real Claude and a window behind another) |
| 1.13 | Make a pull request with `G` (needs `gh`), close the form, then click its notice in the bell; make one fail (e.g. `gh` signed out) and click that notice | The done notice opens the pull request in the browser; the failed notice (failed colour) selects the branch's worktree in the table (skipped: needs gh and GitHub, real repos are off limits) | [ ] (skipped: needs gh and GitHub) |
| 1.14 | On a branch with an open pull request press `G` in the worktree table; change the title and the draft box and press Update; press `G` on a branch with none | The form says "Update the pull request" with the PR's own title, body, base and draft; Update changes them on GitHub and the bell notice opens the PR; with no PR the form is the creating one (skipped: needs gh and GitHub) | [ ] (skipped: needs gh and GitHub) |
| 1.15 | With `gh` signed in and a worktree on a branch that has an open pull request, merge it (or let its checks finish, or approve it) on GitHub; keep the window in front, then behind another | Within about a minute (five behind) a bell notice appears (merged / checks failed or passed / approved / changes requested; failed in the failed colour) and clicking it opens the pull request; behind, the system also notifies; nothing is told for the pull requests as they stood at start; with `gh` missing nothing happens (skipped: needs gh and GitHub) | [ ] (skipped: needs gh and GitHub) |
| 1.16 | With `gh` signed in, open the graph (`1`) in a repository whose branches have pull requests (one with failing checks, one pending, one approved, one merged) | Within a minute the branch's name on its row is followed by a small outlined badge: `#7 ✓` (green, checks passed), `#7 ✓✓` (also approved), `#7 …` (waiting colour, checks running), `#7 ✕` (red, checks failed), `#7 ±` (red, changes asked for), `#7 ⇄` (dim, merged), `#7 ×` (dim, closed), `#7` alone when nothing has run; the pointer turns to a hand, the tip reads "Pull request #7: click to open it" and a click opens it in the browser; with `gh` missing no badge shows (skipped: needs gh and GitHub) | [ ] (skipped: needs gh and GitHub) |
| 1.17 | As 1.16, with minato (0.70.2 or later) running and having searched the repository, and a pull request whose checks minato shows as failed | The badge follows minato's (`✕` for a failure, `✓✓` / `±` from its decision); a pull request merged or closed on GitHub still gets its notice and its `⇄` / `×` badge with `gh` not called (watch the process list for `gh pr list`; the first minute after minato starts to know the repository may still call it once); close minato and the badges go back to `gh`'s reading; with minato running but not knowing the repository nothing changes (skipped: needs minato and GitHub) | | [ ] (skipped: needs gh, GitHub and minato) |
| 1.18 | Start michi (maximized and restored), look at the top of the window | The logo, `michi`, the view tabs, the search box and the bell and window buttons are all in the bar at the very top, not in the middle of the window | | [ ] (skipped: maximize and restore cannot be driven by the harness) |
| 1.19 | Make a job that asks (for example `m` merge on a branch, or `C` cherry-pick) | The buttons sit at the bottom right of the window, rounded, `Cancel` on the left and `Proceed` on the right in solid green; the window has the buttons `Proceed  Ctrl+Enter` and `Cancel  Esc` (JA: `進める  Ctrl+Enter`, `やめる  Esc`) and no line of keys under them; the keys and a click on a button still answer | | [ ] (looked at 2026-10-22: merge box has rounded `やめる Esc` left and solid green `進める Ctrl+Enter` right, bottom right; the key bar of the window still shows `y はい · n いいえ`) |
| 1.20 | Open the `1` view with a worktree that has changed files, look under the branch lists in the left column, then drag the line between them up and down and restart | A `CHANGES` heading with the count and a tree of the changed files (folders fold on a click, `M`/`A`/`D` letters, a click on a file selects the worktree's change in the graph and scrolls the detail to that file's diff); the line follows the mouse (pointer turns to up-down arrows), neither part gets smaller than a few rows, and the position is kept after the restart | | [ ] (looked at in m1-wt3: the changed-files tree sits under the branch lists with a count and dirty.txt) |
| 1.21 | In the `1` view press `Tab` until the changed-files tree has the blue frame, then `j`/`k` (or arrows), `Enter`, `l`/`h` | A blue bar marks the cursor row and the tree scrolls to keep it in sight; `Enter` on a file selects the worktree's change and scrolls the detail to the file's diff, on a folder folds it; `l` opens a folded folder (or goes into an open one, or opens a file), `h` folds an open folder (or goes to the folder a file is in); `Esc` goes back to the graph | | [ ] (looked at: the third Tab puts the blue frame on the changed-files tree; Enter opened nothing visible, so the rest is undecided) |
| 1.22 | Stage one file of a worktree (`git add`), look at the tree | The staged file has a small green dot left of its letter and its hover tip ends with "staged" (JA: "ステージ済み"); the others have none | | [ ] (skipped: staged dot needs a staged file in the lab; not set up) |
| 1.23 | With two worktrees that have changes, click the other worktree's row in the side column, then select the first worktree's change in the graph | The heading reads `CHANGES · branch` and shows the clicked worktree's files; selecting a change row in the graph shows that worktree's files instead | | [ ] (skipped 2026-10-22: needs two worktrees with changes in one lab and a side column click; lab c1 has only one dirty worktree) |
| 1.24 | Click a file in the tree, then look at the tree | The file the detail shows has a faint blue row; it follows when another file is picked in the detail | | [ ] (skipped: no tracked-file row to compare in the lab) |
| 1.25 | Click the `CHANGES` heading | Every folder folds (the pointer is a hand, the tip tells it); another click opens them all | | [ ] (skipped: pointer and tip need hover) |
| 1.26 | Drag the divider away from its place, then double-click it, restart | The divider goes back to the default place (about 62% of the column) and stays there after the restart (skipped: needs a divider drag) | | [ ] (skipped: drag and double-click on the divider not in the harness) |
| 1.27 | Right-click a file in the tree | A menu with the editor, the diff tool and copy the path: the first two open the file / its diff in the tools of `[open]` (a deleted file says it cannot be opened), the last copies the path and says so in a toast | | [ ] (looked at 2026-10-22: menu `エディタで開く` / `差分ツールで開く` / `パスをコピー`; copy put `f.txt` (relative path) on the clipboard; opening in an editor and the deleted-file case not tried) |
| 1.28 | Open two or three repositories in tabs (kept in `state.toml` of the state folder, `%LOCALAPPDATA%\uchmk\michi` on Windows), go to the second tab and press `2`, close michi, start it again with no folder (the icon, `michi.com` alone) | The same tabs come back, the same tab is in front and it is on the changes view; a tab behind opens on the graph when first shown | | [ ] (skipped: state.toml of the real state folder is not touched) |
| 1.29 | Close michi with a tab on a folder you then rename, start it again; then start with `michi <folder>` | The gone folder's tab is left out without a message (the others and the front tab are kept); with a folder named, that folder alone opens as before | | [ ] (skipped: needs renaming a folder across restarts) |
| 1.30 | Start michi over a settings folder from before 0.104.0 (with `window.toml`, `layout.toml`, `recent.txt`, `guides.txt`), move the window and a divider, open a folder | The window opens where it was; `window.toml`, `layout.toml` and the lists are now in `%LOCALAPPDATA%\uchmk\michi` (`window.toml` moved, the others copied, the old copies stay); later changes are written there only | | [ ] (skipped: needs a pre-0.104.0 settings folder) |
| 1.31 | Start 0.105.0 over a state folder of 0.104.0 (`recent.txt`, `guides.txt`, and `seen\*.json`, `review\*.json`, `flagged\*.json` of repositories you used), then look at the recent list (`Ctrl+O`), the dismissed guides, the unseen marks, the review checks and the flags | All of them still show as before; `recent.toml`, `guides.toml` and `seen\*.toml` / `review\*.toml` / `flagged\*.toml` are written (plain TOML); the old files stay and are no longer written | | [ ] (skipped: needs a 0.104.0 state folder) |

## 2. Views and keys

| # | Do | Expect |
| --- | --- | --- |
| 2.1 | Press `1`, `2`, `3`, `4` | The tab in force moves and opens into the view; the graph shows the history (section 6), the worktrees show their table (section 10), the changes show the files to stage (section 14) |
| 2.2 | Click each tab | Same as the digit |
| 2.3 | Press `?` in `1`, `2` and `3`; narrow the window below 460 wide and press `?` | A card with the keys: "Everywhere" first, then the view's, two to a line with the word at the left and the keys at the right, every key there down to `Esc`; the tab stays lit under it; `?` or Esc closes it; the narrow window has one to a line |
| 2.4 | Press `N` in any view | The New work box opens over the view (branch, base, instruction, count, folder, the `tsumugi new` command); Esc closes it |
| 2.5 | Press Ctrl+Shift+P, or click the palette field | The command palette opens with its field focused and the commands listed with their keys; Esc closes it |
| 2.6 | Press a key with no work (`q`) | Nothing happens |
| 2.7 | Press `?` (or `F1`), in a window short enough that the list does not fit; then `Down`, `PageDown`, `End`, `Home`; then bind another key to Quit on the Keys page and press `?` again | The list has the global keys and the keymap's keys (`Ctrl+Q`, `Ctrl+Shift+P`, ...), then the keys of the view in front; the keys move the list and a thin bar shows where it is; the new Quit key shows in the list; `?` or Esc closes it | [x] |

## 3. Keybar and clock

| # | Do | Expect |
| --- | --- | --- |
| 3.1 | Look at the keybar in each view | The view's first keys (the first one lit in the accent colour), then `? Keys`; nothing overlaps the clock when the window is narrow |
| 3.2 | Look at the right of the keybar | The clock as common.toml says (time, date, weekday); it turns over on the minute |
| 3.3 | Turn the clock off in the settings | It goes from the keybar, in every uchmk app that is open |

## 4. Settings

| # | Do | Expect |
| --- | --- | --- |
| 4.1 | Press `Ctrl+,` | The settings open between the title bar and the keybar; the title bar stays, the settings button is lit and no tab is lit; the keybar shows `Esc Back` and `Ctrl+Tab Next page` |
| 4.2 | Press Ctrl+Tab | The page moves from General to Theme, with a small michi preview (the three tabs, two branches) |
| 4.3 | Change the language to 日本語 | Every word of michi turns Japanese at once (the tabs 樹形図・変更・worktree, the keybar, the empty views), and other uchmk apps follow |
| 4.4 | Change the theme, and the size | The whole window follows at once; the logo and the chip stay readable in light and dark |
| 4.5 | Press Esc, click the settings button, or click a tab | Each leaves the settings; a tab opens its view |
| 4.6 | Edit common.toml by hand while michi is open | michi follows within a second |
| 4.7 | In the settings open the Keys page; press `+ Add` on a row, press `Ctrl+Shift+K`, Enter; then `Ctrl+=`, `Ctrl+-`, `Ctrl+0` in the graph; edit keymap.toml (beside common.toml) by hand | The page lists the keys by layer (names in the language chosen); the new key is written to keymap.toml under `prepend_keymap` and a toast says `keymap.toml: 1 changed`; the keys zoom the window and say the size; a hand edit shows in the page within a second; Esc while a key is being pressed ends that, not the settings | [x] (skipped: keymap hot reload and key capture not drivable here) |

## 5. git and errors

| # | Do | Expect |
| --- | --- | --- |
| 5.1 | Start `michi C:\dev` (not a repository) | The middle of the window reads `Not a Git repository` in red with the folder under it, and a toast says so with the folder; the title stays `michi` |
| 5.2 | Start `michi C:\no\such` from a terminal | The message `michi: not a folder: C:\no\such` in the terminal, exit code 2, no window |
| 5.3 | Start `michi --version` and `michi --help` from cmd and from PowerShell | The version or the usage prints in that terminal, exit code 0, no window |
| 5.4 | Start michi with no git on the PATH | The chip shows `✗ git was not found`; the keybar shows `⚠ git` in red |
| 5.5 | Start michi with a git older than 2.38 on the PATH | A toast for 10 seconds names the version and asks to update; the keybar keeps `⚠ git 2.x.y` in amber (skipped: needs OS or machine features the harness lacks) |
| 5.6 | Start `michi` in a worktree other than the main one | The chip names the repository (not the worktree's folder) and its branch; the count is the whole repository's |

## 6. Graph

Open a repository with merges, tags, a remote and a second worktree (`C:\dev\tsumugi` has them all).

| # | Do | Expect |
| --- | --- | --- |
| 6.1 | Open the repository | Within a second, the newest commits first: the graph, the subject with its names, the author, the date (the time today, `Yesterday` and the time, `M/D HH:MM` this year, `Y/M/D` before), the short hash; the trunk (main) runs down the leftmost lane; the header names the columns; only the graph has a scrollbar |
| 6.2 | Look at the names before the subjects | A worktree's branch as a filled pill with a worktree mark (main in the trunk's colour, others in their lane's); a remote branch as a dashed outline (`origin/main`); a tag in amber with a diamond; hovering a worktree pill shows its folder |
| 6.3 | Look at the merges into main | Each is folded: a ring on the trunk, a chip `▸N` (the commits inside) before the subject in the merged lane's colour; hovering the chip says how many it folds and names the branches still inside; the header's `Fold PRs z` toggle is lit |
| 6.4 | Press `z`, then `z` again | The merges open (side lanes appear) with the toast "Unfolded all the merges."; again, they fold with a toast giving how many merges and commits; the header toggle follows (lit or grey) |
| 6.5 | Click the header toggle | The same as `z` |
| 6.6 | Select a folded merge with `j`, press `l` (or →), then `h` (or ←) | `l` opens it: the chip becomes `▾N` with a dashed outline and its commits show on their own lane; `h` folds it again |
| 6.7 | Click a chip | Its merge opens or folds; the selection stays where it was |
| 6.8 | Press `j` `k`, ↓ ↑, PageDown PageUp, Home End | The selected row (lit across) moves and the list scrolls to keep it in sight; End goes to the oldest row read so far |
| 6.9 | Click a row; scroll with the wheel | The row is selected; the wheel scrolls the list without moving the selection (seen [~] in the harness, 1100x600 EN: wheel -10 scrolled the list by about 14 rows; the selection and the detail stayed on 'wt3 change') |
| 6.10 | Press `/` and type part of a subject, a hash or an author's name | The field at the right of the tool band takes the keys, with the count (`3 hits`) at its right; the text sits in the middle of the field; the other rows dim; the hits get marks on the scrollbar |
| 6.11 | In the field, press Enter, Shift+Enter, ↓, ↑ | Enter selects the next hit (round to the first after the last), Shift+Enter the one before; the arrows move the selection; the field keeps the keys |
| 6.12 | Press Esc in the field | The filter clears and every row is bright again |
| 6.13 | Look at the scrollbar | Marks for the worktrees' heads (in their lanes' colours), the tags (amber) and the selected row (the accent); hovering a mark tells what it is |
| 6.14 | Drag the thumb; click the track; click a mark | The thumb stays under the pointer to the release; a click on the track brings that part into the middle; a click on a mark selects its row (skipped: needs drag or wheel, which the harness lacks) |
| 6.15 | Look at commits by dependabot[bot] and github-actions[bot] | The author reads `⚙ dependabot` and `⚙ actions`; hovering shows the full name |
| 6.16 | Open a repository with more than 2000 commits | The first screen comes at once; scrolling near the foot reads on by itself; the last row says how many older commits are left and reads them when clicked; the thumb shrinks as the list grows (partly seen 2026-10-22 (2300-commit lab): first screen at once; `G` then `j` past the 2000th row reads on by itself; the foot row with the count of older commits and the thumb were not seen) |
| 6.17 | Look at a second worktree's commits | They run on their own lane in the worktree's colour, from where they left main |
| 6.18 | Narrow the window to 720 wide; switch to the light theme | The subject is cut with an ellipsis and the names drop before they crowd it; the filter field stays inside the band; every colour stays readable in light (skipped: needs a narrower window, which the harness lacks) |
| 6.19 | Open a repository with more than 2000 commits and no `.git\objects\info\commit-graph*` (a fresh `git clone` of a long history), wait a few seconds, close michi and open it again | `.git\objects\info\commit-graphs` appears while michi is open, with no toast; the history looks the same, and the second opening shows the first screen sooner. A repository with fewer commits gets none (partly seen 2026-10-22 (2300 commits, no commit-graph): the commit-graphs folder under .git/objects/info appears while open, no toast; the faster second opening was not timed) |
| 6.20 | In a history of 100,000 commits, press `z` to fold and unfold, and type in the filter (`/`) | Each answers at once, with no stall while typing; scrolling far down lays the rows out as they come in sight. After `End` has read the whole history, `z` there may take a moment (skipped: needs OS or machine features the harness lacks) |

## 7. Side column

The same repository as section 6.

| # | Do | Expect |
| --- | --- | --- |
| 7.1 | Look at the column left of the graph | Headings with counts: worktree, Branches, Remotes, Tags and Stash (when there is one); the worktree the window is on is lit; each worktree and branch has a dot in its lane's colour; there is no scrollbar, the wheel scrolls it |
| 7.2 | Click a worktree, a branch and a tag | The graph selects that commit and scrolls it into sight; the detail follows |
| 7.3 | Click a group (`test/`, `origin`) | It opens to list its branches further in, with the count at its right; again, it closes; hovering a branch inside shows its full name |
| 7.4 | Click a branch whose commit is older than what is read | The graph reads on until it gets there and selects it; one that is not in the history gives the toast `Not in the history: <hash>` |
| 7.5 | Have a branch ahead of its upstream and one behind (commit without pushing; `git fetch` after a push from elsewhere), and a stash (`git stash`) | The branch and its worktree show `↑N`, `↓N` or both in dim at the right; no number when even or with no upstream. A `Stash` section comes last with each stash's message and a hollow square; hovering shows its hash; a click shows it in the detail (17.9) |

## 8. Detail and diff

| # | Do | Expect |
| --- | --- | --- |
| 8.1 | Select a commit | Below the graph within a moment: the subject in bold, the full hash, the author, the local time and the parents (blue links), the body, the hint `Enter spreads the diff side by side`; `Reading the commit…` shows while it is read |
| 8.2 | Look at the file list | Each file with its status letter in colour (A green, M, D red, R), its path cut from the left with `…` when long, `+a −d` and a bar; every file is listed (the detail scrolls); a renamed file says where it came from over its diff (`元: old`) |
| 8.3 | Click a file row | The detail scrolls to that file's diff |
| 8.4 | Look at the diff | One column: the `` rows in blue, added rows green and deleted rows red, the changed words marked a shade deeper, the line numbers at the left; only the detail's wheel scrolls it, with no scrollbar |
| 8.5 | Press Enter, then Esc | Enter narrows the graph and spreads the diff side by side (old left, new right, the empty side hatched); the title's toggle follows; Esc brings the graph back, and a second Esc clears the filter |
| 8.6 | Click a parent link | The graph selects the parent and the detail shows it |
| 8.7 | Select a merge into main | `N commits came into main by this merge` with their hashes, subjects and times, and whether they are folded or open in the graph; the note that a merge's diff comes in the next stage; no diff |
| 8.8 | Select a commit with a binary file, and a big one | The binary file says it has no diff to show; a very long diff ends with `… the rest is left out` |
| 8.9 | Narrow the window below 900 wide | The side column goes and the detail stacks under the graph, still readable; Enter still spreads the diff (skipped: needs a narrower window, which the harness lacks) |
| 8.10 | Switch to the light theme | The diff's greens, reds and blues and the hatching stay readable |
| 8.11 | Look at a commit adding a file that is not UTF-8 (Latin-1 `café`) | Under the file's name: `Not UTF-8: bad bytes show as �` (one line, not cut at the pane's edge); the bytes show as that mark (a box in this font) in the diff |
| 8.12 | Look at the diff of a long line, in one column and side by side (`Enter`), in a narrow and a wide window; also a long Japanese line | It wraps at the pane's edge with the next part under it, the line number only on the first line; the row is as tall as its text, with no empty lines under a Japanese one and nothing cut off with `…` (wrapping and numbers seen; the blank space under wrapped rows was fixed in 0.106.7 (#141): a 400-letter line and a 260-character Japanese line, side by side at 1100 px, leave none) |

## 9. Uncommitted changes

Open a repository whose worktree has edits, a deleted file and a new file; run a Claude in it from tsumugi.

| # | Do | Expect |
| --- | --- | --- |
| 9.1 | Look at the top of the graph | A row `Uncommitted: N files (+a −d)` in italics above the newest commit, with a dashed dot on the lane of the worktree's branch, author `—`, the time the change was last written and the hash `WIP`; the main worktree's change is on top when it was written last |
| 9.2 | Edit a file in the worktree with michi in front | Within about a second the row's counts and time follow without a key; the selection stays on the row it was on |
| 9.3 | Select the WIP row | The detail says `Uncommitted changes on <branch>`, the worktree's folder in mono, when it was last written, `Read only here. Stage and commit in Changes (2).`, then the files and the diff from the worktree's `HEAD`; the new file is listed as added |
| 9.4 | With the WIP row selected, have the Claude there run, then stop or ask | A box under the hint: `Claude is writing.` or `Claude has stopped.` with its state (glyph and word), the tab, its title and since when; a question in amber (skipped: needs a real Claude or tsumugi session) |
| 9.5 | Put a file over 10 MB (or a folder over 2000 files or 50 MB) in the worktree, not ignored | The WIP row and its counts leave it out; the detail says `Untracked, too big to count: <path>` (three at most, then `and N more`); `git count-objects -vH` in the repository does not grow by its size; no `git.exe` keeps running after michi is closed |
| 9.6 | Make the worktree clean again | The WIP row goes within about a second; a worktree whose only change is a big untracked folder has no WIP row |
| 9.7 | Look at `git status` and the index after michi read the change | Nothing was staged; `git status` shows the same as before |

## 10. Worktree table

Open a repository with a main worktree, a worktree on a branch that is pushed and has one more commit plus edits, and a locked worktree with no branch (`git worktree add --detach` then `git worktree lock`); give the repository an `origin`. Run a Claude in one of them from tsumugi for 10.2.

| # | Do | Expect |
| --- | --- | --- |
| 10.1 | Press `3` | `Reading the worktrees…`, then a title `<repo> · N worktrees` with the sort (`s`), `Read again` and `New work` (`N`) at the right, a dismissable `What is a worktree` note, and a table: `Branch · folder`, `Claude (tsumugi)`, `From main`, `Uncommitted`, `Unseen`, `Review`, `Conflict forecast`, and buttons; under it the forecast grid and the `Unseen changes` box; the main worktree on top, each row with its branch in its lane colour and its folder under it |
| 10.2 | Run a Claude in a worktree from tsumugi, then quit tsumugi | The title adds ` · Claude N` and a `Claude (tsumugi)` column shows the state chip in bold, its note (two rows at most), `Tab N · <title>` with `o` and since when; rows with none say `No Claude running`; within a few seconds of quitting tsumugi the column goes (skipped: needs a real Claude or tsumugi session) |
| 10.3 | Look at the `From main` column | `↑a ↓b` against main, and under it the branch against its upstream: `Same as origin/x`, `N not pushed to origin/x`, `origin/x is N ahead`, `Diverged from origin/x (↑a ↓b)`, `Not pushed yet`; the main row reads against `origin/main` |
| 10.4 | Look at the `Uncommitted` column; then put a folder over 2000 files in a worktree, not ignored | `N files`, `+a −d` in green and red and `written at <time>`, or `None`; the big folder adds `Too big to count: <path>` and is not counted |
| 10.5 | Look at the locked worktree with no branch | `No branch, at <hash>` and a `Locked` mark; a worktree whose folder was deleted says `Its folder is gone (git can prune it)` |
| 10.6 | Press `j` / `k`, the arrows, PageDown / PageUp, Home / End, and click a row | The selection moves (10 rows a page) and the table scrolls to keep it in view; the wheel scrolls the table with no scrollbar |
| 10.7 | Press Enter on a row with changes, and double-click a clean row | On the main row the graph opens (`Show in the graph`); on a linked worktree row the review view opens (`main…<branch>`, `Esc` back) |
| 10.8 | Select a linked worktree and press `d`, then `Esc`; `m`, then `Esc`; `t`, `e`, `E`; `O`, `Q`, `I` | `d` opens the review view (`main…<branch>`, the files, the diff, `Esc` back); `m` asks to bring main in; `t` `e` `E` open the terminal, the editor and the file manager on its folder; `O` opens its pull request page, `Q` the pull request form, `I` the issue; the main row shows `Pull` and `Show in the graph` instead of the review, merge and `Remove` buttons (skipped: needs gh / GitHub / minato, which tests must not touch) (partly seen 2026-10-22: d opens the main...a review view and Esc returns; m shows the merge box; t e E O Q I not tried (open external apps or a browser)) |
| 10.9 | Click `Read again` | The toast `Reading the worktrees again…`, and the table follows a change made outside (a new commit, a new worktree) |
| 10.10 | Narrow the window below 960 wide | Each worktree becomes a card: the branch and folder, then the Claude, `From main`, `Uncommitted`, `Unseen` and `Conflicts ahead` lines, then the buttons, which wrap; still readable at the smallest width |
| 10.11 | Look at the foot, and switch to the light theme | Only when the Claude column shows, a line on how often it reads tsumugi; colours stay readable in both themes |
| 10.12 | With a Claude running in a worktree, select its row and press `o`; on a row without a Claude; press `o` in the graph | tsumugi goes to that Claude's tab (waiting or done one first) and a toast `Went to the Claude tab N in tsumugi`; without one a toast `No Claude is running in <worktree>` | [ ] (skipped: needs a real Claude or tsumugi session) |
| 10.13 | Select a linked worktree with uncommitted files and a Claude, press `g x` (or click `Remove`); Esc; once more and `y`; on the main worktree and on this window's own | A band `Remove the worktree …? The folder goes; the branch … stays.` naming the Claude tab(s), the uncommitted files and the unpushed commits; Esc removes nothing; yes closes the tabs, keeps the change under `refs/michi/backup/` and removes the folder (the branch stays; a toast names the backup); main and the window's own say they stay | [~] (skipped: needs a real Claude or tsumugi session) |
| 10.14 | In the graph or the worktree table press `N`; type a branch, pick the base with Up/Down, type an instruction, raise the count to 2; Ctrl+Enter. Once with an empty branch; once with Esc | A form with the branch (default `tsumugi/MMDD-HHMM`), the base chips, the instruction, the count, the folder and the `tsumugi new …` line as it will run; Ctrl+Enter opens a tsumugi tab with a new worktree and Claude per count (`-1`, `-2` on the branch) and a toast `Started N`; an empty branch says so and stays; Esc closes it and starts nothing | [~] (skipped 2026-10-22: Ctrl+Enter opens a tsumugi tab; needs real tsumugi) |
| 10.15 | In the worktree table select a worktree on a branch pushed to GitHub, press `O`; on a detached one; on a branch whose remote is not GitHub | The browser opens `https://github.com/<owner>/<repo>/pull/new/<branch>` and a toast says so; the detached one says it is not on a branch; the other says the remote is not on GitHub | [x] (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 10.16 | In the worktree table select a worktree with 3+ commits since main, press `r`; move one with `J`/`K`, fold one with `s` and another with `f`, reword one with `r` (edit, Ctrl+Enter), drop one with `d`; Enter, then yes. Once with nothing changed; once with the first commit folded; once with a clash (two commits touching the same lines, swapped); once with uncommitted changes in the worktree | The commits are listed oldest first with pick/squash/fixup/reword/drop, a dropped one struck through; Enter with nothing changed or the first commit folded says so; the band names the counts and refs/michi/backup/; after yes the log is as the list said and a toast names the backup; a clash stops, the 4 view shows it and a toast says so; uncommitted changes refuse with a toast | [x] |
| 10.17 | In the worktree table select a branch behind main, press `m`, `y`; then `m` on main, on a detached row and during a merge or rebase | The band reads `Merge main into <branch> in <folder>?` (and warns when a Claude is working there); after `y` a toast `Merged main into <folder>: …`; main says there is nothing to merge, the others refuse with their reason |

## 11. Conflict prediction

Set up: two worktrees that change the same lines of a file, each committed; a third whose change to that file is not committed yet.

| # | Do | Expect |
| --- | --- | --- |
| 11.1 | Press `1` and look at the two worktrees' head commits | Each has a red `⚠ Conflict ahead` pill after its names; hovering says `Would conflict with <branch> in N places (<files>)`, one line per worktree it clashes with |
| 11.2 | Look at the third worktree's `Uncommitted` row | A dashed yellow `⚠ Conflict ahead` pill after the counts; hovering says `Counting the uncommitted changes, would conflict with <branch> in N places (<files>)` |
| 11.3 | Press `3` and look at the `Conflicts ahead` column | Red `⚠ <branch>: N places` with the files under it, one for each worktree it clashes with; yellow with `(uncommitted)` for the third; `None` for a worktree that clashes with nothing |
| 11.4 | Look at the box under the table | `Conflicts ahead (uncommitted changes counted)`, a table of the worktrees' short names both ways: `—` against itself, `·` for a clean pair, the number of places in red, or in yellow with a dashed frame for the uncommitted one; the hint under it and the foot about `git merge-tree` |
| 11.5 | Hover a number, then click it, then click it again | The tip lists each file with its places; the click frames the cell and the note reads `<a> × <b>: <file> N places. ` with what to expect when they go into main (or, for the yellow one, that committing turns the frame solid); the second click puts the hint back (cannot judge 2026-10-22: clicking the number in R:/lab/c1 left the screen the same before and after, so a change cannot be told; hover tip not drivable) |
| 11.6 | Commit the third worktree's change; then undo the conflicting edit in one of the first two and commit | Within about a second with michi in front, the yellow pill and cell turn red; then the red ones go, and the column says `None` |
| 11.7 | Narrow the window below 960 wide, then to 1000 wide; switch to the light theme | As cards, each has a `Conflicts ahead` line and the box takes the full width; at 1000 wide the table's columns shrink but nothing overlaps; red and yellow stay readable in light (skipped: needs a narrower window, which the harness lacks) |
| 11.8 | Open a repository with only one worktree, and look at `git status`, the branches and `git count-objects -vH` after a minute | The box still shows its table when the trunk branch is there too (main as a thin row and column, `—` and `·`); `Only one worktree: nothing to pair yet.` shows only when fewer than two sides are left (a repository with no other branch); nothing was staged, no branch moved, and the objects do not grow by more than the temporary trees; no `git.exe` keeps running after michi is closed |
| 11.9 | Make a local branch no worktree is on that changes the same lines as main (in the main worktree: `git switch -c topic main~1`, edit and commit, `git switch main`); press `1` | The branch's head has the red `⚠ Conflict ahead` pill; hovering says `Would conflict with main in N places (<files>)`; main's head gets no pill for it, and the branch is not in the table of pairs under `3`. A branch already merged into main has no pill |

## 12. Watching and unseen commits

Set up: the repository of section 10, with michi in front on the graph; commit from a terminal in the worktree on a branch. Where michi keeps what was seen: `%LOCALAPPDATA%\uchmk\michi\seen\<repo>-<hash>.json`.

| # | Do | Expect |
| --- | --- | --- |
| 12.1 | Commit in the worktree on a branch | Within about a second the history is read again without going blank: the new commit shows on its branch with a blue dot before its subject, which is bold; the selection stays on the commit it was on |
| 12.2 | Press `3` | The worktree's `Unseen` column says `● 1 new` (the blue dot); the main worktree says `—` and the others `None`; beside the conflicts box, `Unseen changes (1)` lists the commit: short hash, blue dot, the subject in bold, `<short branch> · <time>` at the right, and the foot `The commits the worktrees have gained since you last looked. Selecting one in the graph marks it seen.` |
| 12.3 | Hover the commit in the list, then click it | The line lights and the pointer becomes a hand; the click opens the graph with that commit selected and its dot and bold go; back in `3`, the column says `None` and the box `All seen.` |
| 12.4 | Commit twice more; in the graph, move onto one of them with `j` / `k`, and click the other | Each loses its dot once selected; the commit selected when the graph first opens, or that the selection follows when rows come in, does not count as seen |
| 12.5 | Make 13 commits in the worktree and press `3` | The box shows 12, newest first, and `… 1 more`; a long subject is cut with `…` before the branch and time (skipped 2026-10-22: the m1 lab shows 11 unseen commits, not 13, and scrolling the box is not driven; not told) |
| 12.6 | Quit michi with commits unseen, commit once more, and start michi again | The commits still unseen keep their dot, and the one made while michi was closed has one too; the file above lists the branch with the commits seen |
| 12.7 | Narrow the window below 960 wide | The `Unseen changes` box goes under the conflicts box at the full width; the cards have an `Unseen` line (skipped: needs a narrower window, which the harness lacks) |
| 12.8 | Put another window in front, commit in a worktree, wait, then bring michi back | Nothing is read while michi is behind (no `git.exe` in Task Manager); when it comes back the history and the table follow at once (skipped: harness cannot put another window in front and watch git.exe) |
| 12.9 | Run `cargo build` (or write many files) in an ignored folder of a worktree, and run `git status` there, with michi in front | michi reads nothing for them: no `git.exe` keeps coming up in Task Manager, and the table does not change; a `git status` does not start a read that starts another (skipped: needs a process monitor and a long build) |
| 12.10 | Open a repository on a network share (`\\server\share` or a mapped drive) | Changes there show within about 5 s while michi is in front; nothing is read while it is behind (skipped: needs OS or machine features the harness lacks) |
| 12.11 | Switch to the light theme | The blue dots and bold subjects in the graph, the column and the box stay readable |

## 13. Branch review

Set up: a worktree on a branch with two commits from main (a renamed file, a changed file that main changed on the same lines too, an added file, an added image) and an uncommitted change to another file. Where michi keeps the files checked: `%LOCALAPPDATA%\uchmk\michi\review\<repo>-<hash>.json`; the last look is the ref `refs/michi/seen/<branch>`.

| # | Do | Expect |
| --- | --- | --- |
| 13.1 | Press `3`, select the worktree and press `d` (or click `Diff d` on it) | The review takes the view: the branch in bold, the switches `Uncommitted u` (lit) and `Since last look s`, `Seen 0 / 5` and `Back Esc`; under them, in red, `Would conflict with main in 1 places (lib.rs)`; the files on the left with a box, the letter (R A M in their colours), the path and `+n −n` (`Binary` for the image); the first file's diff on the right; the key bar lists `j k`, `Tab`, `J K`, `n`, `g` |
| 13.2 | Press `j` / `k`; click a file's name; click another file's box | The selection and the diff follow the keys and the click; the click on a box ticks that file without moving the selection |
| 13.3 | Press Space on a file, then on a ticked one | The first is ticked, the selection goes on to the next file not ticked and `Seen` counts up; Space on a ticked file takes the tick off |
| 13.4 | Press Esc, change and save a ticked file in the worktree, wait a second and press `d` | The other ticks are kept; the changed file is not ticked any more (a tick is for the content it was given to); the file above lists the branch |
| 13.5 | Press `u`, then `u` again | The switch goes dark and the file changed only in the worktree leaves the list, `Seen n / m` with it; the second `u` brings it back |
| 13.6 | On a branch never looked over, press `s`; then Esc, commit once more in the worktree, press `d` and `s` | First the toast `Not looked over before: all of it is new`; then the switch is lit, the title reads `since the last look (<short hash>)` and only the files changed since show (and the uncommitted ones); with nothing new, `No file changed since the last look` |
| 13.7 | Press Enter, then Esc twice | Enter widens the diff side by side (the switch reads `Back to one column Esc`), the first Esc puts it back, the second closes the review on the worktree table |
| 13.8 | Select a file with a long diff; roll the wheel over it, press PageDown and PageUp; select another file | The diff scrolls (no scrollbar); the other file starts at its top (seen [~] in the harness, 1100x600 EN, 2 view: wheel -5 over the diff scrolled it from lines 1-2 to lines 4-5; PageDown/PageUp and the other-file start not checked, the lab has one file) |
| 13.9 | Press `d` on a worktree with a detached HEAD; on a branch with nothing from main | The toast `This worktree is on no branch`; the review reads `No file changed from main` |
| 13.10 | Commit in the worktree while the review is open; switch the worktree to another branch from a terminal | The list follows the new commit within about a second, keeping the ticks; on another branch the list stays as it was and nothing is read |
| 13.11 | Narrow the window to its smallest; switch to the light theme | The branch name is cut with `…`, the files keep their counts and the key bar folds what does not fit; the colours stay readable in both themes (skipped: needs a narrower window, which the harness lacks) |
| 13.12 | In the review of a branch behind main press `m`; Esc; once more and `y`; on `main`'s own review | A band `Merge main into <branch> in <worktree>?`; Esc merges nothing; yes merges (a toast; a clash opens the `4` view); on main a toast says it is main itself | [x] |
| 13.13 | In the review of a branch with a Claude in its worktree: `J` `K` over a diff, click a line, `n`, type a note, Ctrl+Enter; on a second line another; Esc and `d` again; `g` | The cursor line is tinted blue and scrolls into sight; a line with a note has an amber bar on its left and the note shows under the diff; the top bar reads `Notes 2`; the notes are still there after closing the review; `g` toasts that 2 notes were sent, the Claude gets one line naming a file in the temp folder (`michi-notes`) and does not commit, and the notes are gone from the review | [ ] (skipped: needs a real Claude or tsumugi session) |
| 13.14 | `n` on a line that has a note, empty it, Ctrl+Enter; `g` with no notes; Esc while writing | The note is taken away; a toast says there are no notes yet; Esc leaves the note as it was (skipped 2026-10-22: needs a line note typed and emptied, then the toast; not set up) |

## 14. Changes: stage and commit

Set up: a worktree with a staged file, a changed file with two hunks far apart, a new file, a binary file and a renamed file; a second worktree; a commit pushed and one not.
michi writes only through `git add`, `git reset`, `git apply --cached` and `git commit` in the worktree shown.

| # | Do | Expect |
| --- | --- | --- |
| 14.1 | Press `2` in the set-up worktree | The top bar: the worktrees as chips (the window's own lit, a green ● on one where a Claude is writing), `[ ]` after them, `Tree t` and `amend A` on the right; the files on the left under `Staged` and `Not staged`, each with its letter (M A D R `?` in their colours), the path and `+n −n`, the new file last with `?`; under the list the message box with `What changed, and why` and `Commit`; the selected file's diff on the right; the `2` tab reads `2 Changes` and the number of files listed (a file both staged and changed counted once); the key bar lists `Space`, `c`, `l h`, `Tab` |
| 14.2 | Press `j` / `k`; Space on a file not staged, then on a staged one; `a`, then `a` again; click a file, double-click another | Space moves the file to `Staged` (and back out on a staged one) and the selection stays on that file; the first `a` stages everything, the second unstages everything; a click selects, a double-click stages or unstages; `git status` in a terminal agrees each time |
| 14.3 | On a changed file with two hunks press `l`, `j`, `v`, `j`, Space; then `a`; then `h` | `l` puts an accent strip on the first changed line and the title reads `Space in or out · v range · a hunk · s split · d throw away · h back`; `j` walks the changed lines only; `v` picks a range (tinted blue); Space stages just those lines (`git diff --cached` shows them, `git diff` the rest); `a` stages the hunk under the cursor; `h` goes back to the files |
| 14.4 | Press `l` on the new file, on a binary file and on a renamed file | The toast `This file goes in whole: its lines cannot be picked`; the diff says the same; Space stages the whole file |
| 14.5 | On a branch other than `main`, stage a file, press `c`, write a two-line message (`j`, `k`, Space type as text), press Ctrl+Enter (on `main` itself a band first asks `Go ahead y` / `Not now Esc`) | The box takes the keys; the commit is made (toast `Committed <hash>`), the box and the staged list empty, the graph (`1`) shows the commit with the message; Esc in the box only leaves it |
| 14.6 | Press Ctrl+Enter with nothing staged; then stage a file and press Ctrl+Enter with an empty box | The toasts `Nothing staged to commit` and `Write a commit message first (c)`; nothing is written |
| 14.7 | On a commit not pushed press `A`, change the message, Ctrl+Enter | The switch lights and the box fills with the head's message (the title reads `Message of the commit amended`, the button `Amend`); the head is replaced: one commit, not two; `A` again before writing empties the box (skipped: the harness's `A` / click did not show the amend state in the screenshots; undecidable) |
| 14.8 | On a pushed commit press `A`, then Ctrl+Enter; Esc; Ctrl+Enter and `y` | A band under the top bar: `The last commit is pushed: amending it rewrites history others may have.` with `Go ahead y` and `Not now Esc`; Esc leaves the commit as it was; `y` amends it (skipped 2026-10-22: needs a pushed commit, and the labs have no remote) |
| 14.9 | Start a Claude in a worktree from tsumugi; in michi open that worktree in `2` and press Space; press `x`, Esc, Space, `y`, then Space on another file | The band `A Claude is writing in this worktree. Write beside it?`; `x` does nothing while it waits; Esc says no and nothing is staged; `y` stages; the next Space does not ask again until that Claude stops or another worktree is shown (skipped: needs a real Claude or tsumugi session) |
| 14.10 | Write a message, press `]` and `[`; click another worktree's chip; open a repository with one worktree and press `]` | Each worktree's files are read anew and the chip lights; the message written stays with its worktree; with one worktree, the toast `There is only one worktree` |
| 14.11 | Press Enter, then Esc; pick lines and press Enter again | The diff spreads side by side, old on the left and new on the right; the picked lines are tinted on their side only; Esc goes back to one column (on the lines, the first Esc leaves the lines) |
| 14.12 | With `2` open, save a file in the worktree from an editor; let a Claude commit there | The list follows within about a second and keeps the selection on the same file; staged lines stay staged (skipped: needs a real Claude or tsumugi session) |
| 14.13 | Leave `2` for `1`; then make the worktree clean | The `2` tab still counts the uncommitted files of the worktree last shown; with none, no number |
| 14.14 | Add a pre-commit hook that prints a line and exits 1, then commit | The toast `Could not write: …` with the hook's words for 8 seconds; the staged list and the message stay |
| 14.15 | Narrow the window to its smallest; switch to the light theme | The list keeps at least 240 px, the chips end in `…`, the key bar folds what does not fit; the tints, the band and the letters stay readable in both themes (skipped: needs a narrower window, which the harness lacks) |
| 14.16 | Add a pre-commit hook that prints five lines (some in colour, `printf '\033[31m…\033[0m'`) to stdout and stderr and exits 1; commit; Esc, Esc; commit again, then `j`; commit again, unstage and stage a file | The toast `Could not write: <the first line>`; in place of the diff the red title `Could not commit: all git and the hooks said` with `Close Esc` at the right and all five lines below in a fixed-width font, without colour codes; the wheel and PageDown scroll a long one (no scrollbar); the first Esc only leaves the message box, the second closes it and the diff is back; `j` (another file) and a write that goes through close it too; a write that fails with one line only shows the toast (skipped: needs drag or wheel, which the harness lacks) (seen: toast, red title, five lines without colour codes, Esc closes, `j` moves; wheel/PageDown and the one-line case not checked) |
| 14.17 | On a hunk with two changes a few lines apart press `s`; `j`, `a`; Space; save the file from an editor; `s` again; on a file whose hunks each have one change press `s` | `s` goes into the lines and each hunk is cut into one piece per run of changes, a `` line before each; `a` picks the piece under the cursor and Space stages only it (`git diff --cached` shows that change alone); after the save the file stays cut; `s` again joins the hunks back; on the other file the toast `Nothing to split: each hunk has one run of changes` (skipped 2026-10-22: needs saving the file from an external editor) |

## 15. Fetch, pull and push

Set up: a bare repository as `origin` and two clones of it (Git Bash: `git init --bare r.git; git clone r.git a; git clone r.git b`), a commit pushed from `a`; in `a` a second worktree on a new branch.
michi runs `git fetch --prune --progress`, `git pull --ff-only`, `git push` (and `--force-with-lease` only after `y`) in the worktree the view points at.

| # | Do | Expect |
| --- | --- | --- |
| 15.1 | Press `f` (or click `fetch f` in the tool band of `1`); press `f` again; restart michi | While it runs the key bar's right side reads, in amber, `fetch · <step> n%` and `Esc stops`; then the toast `Fetched: n refs moved`, the graph shows the moved remote branches and the key bar reads `⟳ fetch just now` (later `n min ago`); the second `f` says `Fetched: nothing new`; after the restart the key bar still gives the time of the last fetch |
| 15.2 | On a branch behind its upstream press `p`; press `p` again; on a branch with no upstream press `p`; on a branch that went its own way press `p` | A dialog `Pull <worktree>?` first says what comes in (with uncommitted files: they are set aside and put back; with nothing to take: `Nothing to pull in`), `y` goes on; `Pulled n commits from origin/main` and the graph moves the branch; then `Already up to date with origin/main`; `main has no upstream to pull from: P pushes it`; `origin/main went its own way (↑a ↓b): a pull only goes forward, so nothing was merged` and nothing moves (pull is fast-forward only) |
| 15.3 | Commit and press `P` and `y` on the band (112.1); press `P` again; on a new branch with no upstream press `P` | `Pushed to origin/main`; `origin/main has it already`; `Pushed to origin/feat, now its upstream` and `git status -sb` shows `feat...origin/feat` |
| 15.4 | Let a second clone push first, commit here and press `P` without fetching | The toast `origin/main refused the push: [rejected] … (fetch first). f fetches its commits; then F pushes over them, asking first`, or, when michi's own fetch has already seen the commit, `origin/main has 1 commit this branch lacks. F can push over it (asks first)`; nothing is rewritten |
| 15.5 | After 15.4 press `f`, then `P`; then `F`; Esc; `F` again and `y` | `P` only says `origin/main has n commits this branch does not: F pushes over them, asking first` and pushes nothing; `F` shows a red band (112.3) whose note reads `origin/main has n commits this branch does not. Push over them? What origin/main has now is kept under refs/michi/backup/ first.` with `Go ahead y` and `Not now Esc`; Esc pushes nothing; `y` pushes with `--force-with-lease` and says `Pushed over origin/main; what it had is kept in michi/backup/<time>`; that ref points at the remote's old tip and `git reflog refs/michi/backup/<time>` reads `michi: push --force-with-lease over origin/main` |
| 15.6 | On a branch only behind its upstream press `P`, then `F` | Both say `Nothing to push: origin/main is n ahead (p pulls)`; no band |
| 15.7 | Add a remote that does not answer (`git remote add slow https://10.255.255.1/x.git`), press `f`, then `f` again, then Esc | The key bar shows `fetch` and `Esc stops` in amber; the second `f` says `fetch is running: Esc stops it`; Esc says `Stopping fetch…`, then `fetch stopped`; afterwards no `git`, `git-remote-https` process is left (Task Manager); remove the remote after |
| 15.8 | In `1` press `p` / `P`; in `3` select another worktree's row and press `P`; click `pull p` on a card; in `2` show another worktree and press `p` | `1` works on the worktree michi was opened in; `3` on the selected worktree; `2` on the worktree shown; a band (15.5) shows over `2` and `3` too (skipped 2026-10-22: pull and push need a remote; the labs have none) |
| 15.9 | Start a Claude in a worktree from tsumugi; press `p` for that worktree; Esc; `p` and `y` | The band `A Claude is working in <worktree>. Pull there anyway?`; Esc pulls nothing; `y` pulls (skipped: needs a real Claude or tsumugi session) |
| 15.10 | Fetch from an https remote that needs a sign-in; fetch from an ssh remote whose key has a passphrase and no ssh-agent | The Git Credential Manager window comes up as it does in a terminal and the fetch goes on after it; the ssh one fails at once (no prompt hangs) with `fetch failed: … — start ssh-agent and ssh-add your key` (skipped: needs a real sign-in or passphrase remote) |
| 15.11 | Narrow the window to its smallest with a band shown and a fetch running; switch to the light theme | The band wraps its text and grows, its keys stay whole; the key bar folds its keys before the fetch progress or age; both stay readable in both themes (skipped: needs a narrower window, which the harness lacks) |
| 15.12 | In a clone with two remotes and no `origin` (`git remote rename origin up`, `git remote add mine <path>`), press `P` on a branch with unpushed commits; type `mi`, Enter, `y` on the band (112.1); then `P` again on another branch; Esc | The box `Push to: pick a remote` (`pushing <worktree>`) lists `mine` and `up`; the keys read `Enter pushes to it (and sets the upstream) · Esc closes`; `mi` leaves `mine`; Enter closes it and pushes there (`Pushed … to mine`, upstream becomes `mine/<branch>`); Esc pushes nothing. With `origin` or a `pushRemote` set no box shows | [~] (lab p1512, two remotes `up`/`mine`, no origin, branch feat without upstream: `P` opened `push 先の remote を選ぶ` with `mine` and `up`, the keys line read Enter/Esc in Japanese, `mi` left `mine`, Enter then `y` pushed: `mine` has feat and `feat` now tracks `mine/feat`. Not run: the second `P` + Esc, the `origin`/`pushRemote` case, EN); 手で確かめる: #144) |

## 16. Branches, tags and revert

Set up: as in 15 (a bare `origin`, a clone `a` with a second worktree on `feat`), a few more branches, one pushed and deleted locally (`git push origin x:refs/heads/remote/only`).
michi runs `git branch`, `git switch` (`-c`, `--track`), `git branch -D`, `git tag` (`-a` with a message), `git tag -d`, `git push <remote> refs/tags/<name>:refs/tags/<name>` and `git revert --no-edit` (`-m`, then `--abort` when it clashes),
and `git update-ref refs/michi/backup/<time>` before it deletes.

| # | Do | Expect |
| --- | --- | --- |
| 16.1 | In `1` select a commit and press `b` (or click `Branch b` in the tool band); type part of a name; ↑ ↓; move the pointer over the rows; Esc; `b` and click outside; press `b` and type a name at once | A palette under the top bar: the box with `Branch: a name switches to it, a new one is made`, under it `<worktree> · at <commit>`, the local branches (with `here`, `in <worktree>`, `↑a ↓b`) then the remote branches no local one has, each with its commit at the right; the foot reads `Enter switches · Shift+Enter makes and switches · F2 renames · Ctrl+U upstream · Ctrl+R rebases onto · Del deletes · Esc closes`; typing narrows them, the typed letters lit and the best first; the arrows and the pointer move the selection (a resting pointer does not); Esc and a click outside close it; the letters typed right after `b` land in the box |
| 16.2 | Type a branch name with the Japanese IME and confirm it | The composition shows in the box and the confirmed name narrows the rows; nothing is lost (skipped: needs the Japanese IME on a real keyboard) |
| 16.3 | Type a new name and Enter; `b` again, another new name and Shift+Enter | The last row reads `<name>  make it at <commit>`; Enter: `Made branch <name> at <commit>`, the graph shows its chip and the worktree stays on its branch; Shift+Enter: `Made branch <name>; <worktree> is on it` and `git branch --show-current` there gives it |
| 16.4 | Pick a branch no worktree has and Enter; type the name of a remote branch with no local one (`origin/x`) and Enter; type it without `origin/` | `<worktree> is on <name> now`; `Made <name> following origin/<name>; <worktree> is on it` and `git status -sb` shows `<name>...origin/<name>`; the name without `origin/` lists `origin/<name>` and no `make it` row |
| 16.5 | Pick the branch this worktree is on; pick one checked out in another worktree; in a worktree in the middle of a merge (`git merge` that clashes) pick a branch and Enter, then type a new name and Enter | `<worktree> is on <name> already`; `<name> is checked out in <worktree>: a branch is in one worktree at a time`; `Branches can be made. Switching is stopped in <worktree>: it is in the middle of a merge.` for 8 seconds; the palette stays open each time; the new name is made; `git merge --abort` after |
| 16.6 | Select another worktree's uncommitted row in `1` and press `b`, pick a branch and Enter; select a commit and press `b` | With the uncommitted row the palette reads `<that worktree> · at <its HEAD>` and switches that worktree; with a commit, the window's own worktree and that commit (looked at 2026-10-22 in R:/lab/m1: on wt3 uncommitted row b reads m1-wt3 from 5d5f444 (its own HEAD, branch list marks wt3 as here); on a commit row it reads m1 from 5eba0f4; Enter was not pressed) |
| 16.7 | Start a Claude in a worktree from tsumugi; select its uncommitted row, `b`, pick a branch and Enter; Esc; again and `y` | The band `A Claude is working in <worktree>. Switch it to <name> anyway?`; Esc switches nothing; `y` switches (skipped: needs a real Claude or tsumugi session) |
| 16.8 | Select a branch no worktree has, whose commits no other branch or tag has, and press Delete; Esc; again and `y`; delete a branch merged into main the same way | A dialog (`Go ahead Ctrl+Enter` / `Not now Esc`, `y` and `n` too) `Delete branch <name>? When no other branch or tag has its commits, they are kept under refs/michi/backup/ first.`; Esc deletes nothing; `y`: `Deleted branch <name>; its commits are kept in michi/backup/<time>` and `git log refs/michi/backup/<time>` shows them; the merged one: `Deleted branch <name>` and no new backup |
| 16.9 | Press Delete on the branch checked out here, on one checked out in another worktree, on a remote branch | `<name> is checked out in <worktree>: switch it away first`; `Remote branches are not deleted here: git push origin --delete <name> does it` for 8 seconds; nothing is deleted |
| 16.10 | Select a commit and press `T`; type `v0.1 first release` and Enter; `T`, `v0.2` and Enter | The box reads `Tag: a name, then its message if any`, the tags are listed, the last row reads `v0.1  on <commit>`, the foot `Enter makes or shows · Shift+Enter makes and pushes · Del deletes · Esc closes`; `Made tag v0.1 at <commit>` and the graph shows `◆ v0.1`; `git cat-file -t v0.1` gives `tag` and its message is `first release`; `v0.2` is a light tag (`git cat-file -t` gives `commit`) |
| 16.11 | `T`, pick `v0.1` and Enter; `T`, pick it and Shift+Enter; `T`, type `v0.3` and Shift+Enter; during a fetch (15.7) `T`, `v0.4` and Shift+Enter | Enter selects its commit in the graph; Shift+Enter: `Pushed v0.1 to origin` and `git ls-remote --tags origin` lists it; `v0.3` is made and pushed; during the fetch `fetch is running: Esc stops it` and nothing is made (skipped 2026-10-22: pushing tags needs a remote; the labs have none) |
| 16.12 | `T`, pick `v0.1` and press Delete; Esc; again and `y` | The band `Delete tag v0.1? What it points at is kept under refs/michi/backup/ first.`; Esc deletes nothing; `y`: `Deleted tag v0.1; kept in michi/backup/<time>` and `git cat-file -t refs/michi/backup/<time>` gives `tag`; the remote keeps its tag, so the next fetch brings it back |
| 16.13 | Select a commit on the current branch and press `t`; Esc; `t` and `y` | The band `Revert <id> “<subject>” with a new commit on <branch> in <worktree>?` (`A Claude is working there.` added when one is); Esc does nothing; `y`: `Reverted <id> on <worktree>: <new>` and the graph shows `Revert "<subject>"` on top |
| 16.14 | Select a merge commit and press `t`; press `1` | The band `<id> “<subject>” is a merge. Revert it with a new commit on <branch> in <worktree>, keeping which parent's side?` with `keep parent 1 1`, `keep parent 2 2` and `Not now Esc`; `1` reverts it keeping the first parent's side: `Reverted <id> on <worktree>: <new>` |
| 16.15 | Revert a commit whose lines a later commit changed; revert a commit already reverted; in a worktree in the middle of a merge press `t` | `Reverting <id> clashed in <files>, so it was called off: git revert <id> in a terminal lets you resolve it` and `git status` is clean; `Nothing to revert: <id> is undone already`; `<worktree> is in the middle of a merge: finish it first` |
| 16.16 | Look at the tool band of `1`; press `?`; narrow the window to its smallest with the palette open; switch to the light theme | The tool band reads `Branch b` (`ブランチ b` in Japanese); help lists `b Make or switch a branch`, `T Tag the commit`, `t Revert the commit (asks first)`; the palette narrows with the window, long names end in `…` and the commits stay at the right; readable in both themes (skipped 2026-10-22: needs the narrow window and the light theme, which the harness cannot set) |

## 17. Stash

Set up: as in 16; in `a` change a tracked file and add an untracked one.
michi runs `git stash push --include-untracked` (`-m` with the message typed), `git stash apply <hash>`, `git stash drop stash@{k}` (after a pop, and when dropped)
and `git update-ref refs/michi/backup/<time>` before it drops one.

| # | Do | Expect |
| --- | --- | --- |
| 17.1 | In `1` press `s` (or click `Stash s` in the tool band); Esc; `s` and click outside | A palette under the top bar: the box with `Stash: a message to put the changes away, or pick one`, under it `in <worktree>`, the first row `Put the changes away` in the accent colour and selected, then the stashes newest first, each with `stash@{k}` and its hash at the right; the foot reads `Enter puts away or applies · Shift+Enter pops · Del drops · Esc closes`; Esc and a click outside close it |
| 17.2 | `s`, type `half done` and Enter | The first row reads `Put the changes away as “half done”` and the stashes below narrow to those matching it; Enter: `Put the changes of <worktree> away: <hash>`; the uncommitted row goes from the graph, `git status` is clean (the untracked file gone too) and `git stash list` shows `On <branch>: half done` |
| 17.3 | `s` and ↓ to a stash | The detail shows the stash: its message, hash and the commit it was made on as its parent, its files (the untracked one as `A`) and their diff |
| 17.4 | On a stash press Enter; then `git checkout -- . ; git clean -fd` in a terminal; `s`, the same stash and Shift+Enter | Enter: `Put the stash back on <worktree>; it stays in the list`, the changes are back, the untracked file too, and the stash is still listed; Shift+Enter: `Put the stash back on <worktree> and dropped it` and it is gone from the palette and the side column |
| 17.5 | With the stash's file changed again, Enter on the stash; then `git checkout -- .`, commit a change to the same lines and Shift+Enter on it | The first: `stash apply failed: error: Your local changes to the following files would be overwritten by merge: <file>` and nothing changes; the second: `The stash clashed on <worktree> in <file>: resolve the marks there and git add them; the stash is kept`, `git status` shows `UU <file>` with the marks in it and the stash is still listed |
| 17.6 | On a stash press Delete; Esc; again and `y`; Delete on the first row | The band `Drop stash <hash> “<message>”? It is kept under refs/michi/backup/ first.`; Esc drops nothing; `y`: `Dropped stash <hash>; kept in michi/backup/<time>`, `git stash list` no longer has it and `git rev-parse refs/michi/backup/<time>` gives its hash; Delete on the first row does nothing |
| 17.7 | In a worktree in the middle of a merge, `s` and Enter on the first row, then on a stash; with a Claude working in the worktree (from tsumugi), `s` and Enter; Esc; again and `y`; the same on a stash | `Stashes are stopped in <worktree>: it is in the middle of a merge.` for 8 seconds and the palette stays; with the Claude the band `A Claude is working in <worktree>. Put its changes away anyway?` (`Put the stash back there anyway?` on a stash); Esc does nothing; `y` does it (skipped: needs a real Claude or tsumugi session) |
| 17.8 | Select another worktree's uncommitted row in `1` and press `s`, Enter; with no changes anywhere press `s` and Enter | The palette reads `in <that worktree>` and puts that worktree's changes away; with none: `<worktree> has no changes to put away` |
| 17.9 | Click a stash in the side column; Esc; click it again and press `j` | The detail shows the stash as in 17.3 and its row is lit in blue; Esc brings back the selected commit and the row goes dark; `j` does too |
| 17.10 | Press `?`; narrow the window to its smallest with the palette open; switch to the light theme | Help lists `s Stash`; the palette narrows with the window, long messages end in `…` and the hashes stay at the right; readable in both themes (skipped 2026-10-22: needs the narrow window and the light theme, which the harness cannot set) |

## 18. Discard

Set up: as in 16; in `a` change two tracked files (one in two places far apart) and add an untracked file in a new folder, `u/new.txt`.
michi runs `git stash create` and `git update-ref refs/michi/backup/<time>` first, then `git restore --worktree -- <file>`,
or `git apply --reverse` with the picked lines; an untracked file is kept in a commit of its own on the head (made with an index of its own) and then deleted.

| # | Do | Expect |
| --- | --- | --- |
| 18.1 | In `2` on a changed tracked file in Unstaged press `d`; Esc; `d` again and `y` | The band `<file> の変更を捨てますか？先に refs/michi/backup/ に退避します。` (`Throw away the changes to <file>? They are kept under refs/michi/backup/ first.`) with `進める y` / `やめる Esc`; Esc leaves the file; `y`: the toast `<file> の変更を捨てました。michi/backup/<time> に退避しました` for 10 seconds, the file leaves Unstaged and `git show refs/michi/backup/<time>:<file>` has the change |
| 18.2 | On the file with two changes press `l`, `v`, `j` over the first change and `d`, `y` | The band `<file> の 2 行の変更を捨てますか？…`; `y`: `<file> の 2 行を捨てました。…`, the first change is gone, the second stays and the cursor stays in the lines; the backup has the file with both changes; `git diff --cached` is as it was |
| 18.3 | `h`, on `u/new.txt` press `d`, `y` | The band `u/new.txt を消しますか？Git が追っていないファイルです。…`; `y`: `u/new.txt を消しました。…`, the file and the empty folder `u` are gone and `git show refs/michi/backup/<time>:u/new.txt` has it |
| 18.4 | Stage a file (Space) and press `d` on it in Staged | `ステージした変更は捨てません。先に外してください（Space）` and nothing changes |
| 18.5 | With a Claude working in the worktree (from tsumugi), `d` on a file | The band carries both the question for the file and `この worktree では Claude が書いている途中です。横から書き込みますか？`; Esc does nothing; `y` throws the changes away (skipped: needs a real Claude or tsumugi session) |
| 18.6 | Press `?`; narrow the window to its smallest with the band up; switch to the light theme | Help lists `d 変更を捨てる（先に確かめる）`; the band's text wraps or ends in `…` with `y` and `Esc` still visible; readable in both themes (skipped 2026-10-22: needs the narrow window and the light theme, which the harness cannot set) |

## 19. Clean up

Set up: a repository with a remote (`origin`) and these branches: `done` merged into `main`; `feat` merged and checked out in a worktree `r-feat`;
`squashed` pushed, then deleted on the remote and fetched with `f` (like a pull request merged with squash); `busy` merged and checked out in
a worktree `r-busy` with a change not committed; a worktree `r-old` on an unmerged branch `old`, its folder then deleted; `wip` unmerged and never pushed.
michi lists with `git for-each-ref --merged` and the upstream's `[gone]`; for each row picked it runs `git worktree remove <folder>`, then
`git branch -D <branch>` with `git update-ref refs/michi/backup/<time>` first when no other branch has the branch's commits.

| # | Do | Expect |
| --- | --- | --- |
| 19.1 | In `3` press `g` then `c` | A list over the table: the title `Clean up: branches merged into main or gone from their remote, and worktrees whose folder is gone`, under it `“Gone from its remote” is as of the last fetch: <age>`; rows with a box, none ticked: `done` `merged into main` with its hash, `feat` `merged into main · its worktree r-feat goes too`, `squashed` `gone from its remote (its tip is kept first)`, `r-old` `a worktree whose folder is gone`; `busy` grey without a box and `has uncommitted changes` in yellow; `main`, `old` and `wip` are not listed; the foot `0 picked · Space picks · a picks all · Enter cleans up · Esc closes` |
| 19.2 | `j` / `k`; Space on a row, on `busy`; `a`, `a` again; click a row; Esc; `g c` and click outside | Space ticks the row and the foot counts it; on `busy` nothing happens; the first `a` ticks every row that can go, the second lets them all go; a click ticks the row; Esc and a click outside close the list |
| 19.3 | `g c`, Enter with nothing ticked | The toast `Pick the rows to clean up with Space first`; the list stays |
| 19.4 | `a`, Enter; Esc | A dialog `Clean up 4: done, feat, squashed and 1 more? The worktree folders go too, with the files git ignores in them: r-feat. Commits no other branch has are kept under refs/michi/backup/ first.` with `Go ahead y` / `Not now Esc`; Esc goes back to the list and nothing is gone |
| 19.5 | Enter, `y` | The toast `Cleaned up 4: done, feat, squashed and 1 more. Unmerged commits are kept in michi/backup/<time>`; the list closes; the table has `main` and `busy` left and the top bar's chip reads `worktree 2`; `git branch` no longer has `done`, `feat` and `squashed`, `git worktree list` no longer has `r-feat` and `r-old`, the folder `r-feat` is gone, and `git log -1 refs/michi/backup/<time>` is `squashed: one` |
| 19.6 | Open a tsumugi tab in a merged branch's worktree; press `g` then `c` | That worktree's row is grey with `a tsumugi tab is open there`; closing the tab and pressing `g c` again lists it with a box (skipped: needs a real Claude or tsumugi session) |
| 19.7 | Commit to a ticked branch from a terminal while its dialog is up, then `y` | It stays: `… Stayed: <branch> (it moved after it was listed)`; the others go |
| 19.8 | With nothing left to clean up press `g` then `c` | The toast `Nothing to clean up: no branch is merged into main or gone from its remote, and no worktree's folder is gone`; no list opens |
| 19.9 | Press `?` in `3`; narrow the window to its smallest with the list open; switch to the light theme | Help lists `g c Clean up`; long rows end in `…` with the boxes and hashes still visible; readable in both themes (skipped 2026-10-22: needs the narrow window and the light theme, which the harness cannot set) |

## 20. The command line and `michi.com` (Windows)

| # | Do | Expect |
| --- | --- | --- |
| 20.1 | Put `michi.exe` and `michi.com` (from the Build artifact, or `michi-com.exe` copied to that name) in one folder; in PowerShell there run `$v = .\michi --version; "[$v]"` | `michi.com` answers: `[michi 0.15.0]` (the version), printed by the second statement, and the prompt comes back after it |
| 20.2 | Run `.\michi --help` in PowerShell, then in `cmd` | The usage lines print before the prompt comes back, in both |
| 20.3 | Run `.\michi --bogus`, then `.\michi C:\no-such-folder`, each followed by `$LASTEXITCODE` (`echo %ERRORLEVEL%` in `cmd`) | `michi: unknown argument` and the usage, then `2`; `michi: not a folder: C:\no-such-folder`, then `2`; no window opens |
| 20.4 | Run `.\michi C:\dev\some-repo` (a repository), then `.\michi` with no argument in a repository's folder | The window opens on that folder and the prompt comes back as soon as it is up, with michi still open; closing the window later prints nothing in the console |
| 20.5 | Run `$v = .\michi.exe --version` (the windowed program itself), wait a few seconds, then run `.\michi.exe --version` | `$v` is empty and no `panicked` message appears later (the line has nowhere to go and is let go); the second, with no `$v =`, prints the version in the console (skipped: the debug build is a console program, so `$v` fills; needs the release build) |
| 20.6 | In a repository's folder run `$v = .\michi` (no argument, its output taken through a pipe); once the prompt is back, run `Get-ChildItem $env:TEMP\michi-com-*`; close michi and run it again | The prompt comes back once the window is up, with michi still open; two files (`.out` and `.err`) while it runs, none after it closes (partly seen: prompt back at once, one `.out` file while running and none after close, no `.err` seen) (skipped: needs a real PowerShell console and michi.com) |
| 20.7 | With `michi.com` and `michi.exe` in one folder run `$env:MICHI_KEYS_DONE="$env:TEMP\mk\done.txt"; .\michi C:\dev\some-repo --keys '2<Wait:200><State:two>3<Quit>'` (create `$env:TEMP\mk` first), then read `$env:TEMP\mk	wo.txt` and `done.txt`; then `.\michi --keys j -V` and `.\michi --keys` | The window opens, goes to the `2` view, then to `3`, and closes by itself; `two.txt` starts with `focus: changes` and has `view: 2`; `done.txt` ends with `keys: done` after `quit: yes`; `--keys j -V` prints the version and opens no window; `--keys` alone prints `michi: --keys needs a script` and the usage, then `2` (`<Quit>` now writes `quit: yes` / `keys: done` (0.106.7, #142); the `two.txt` part and `--keys` alone were seen) |

## 21. Conflicts

Set up: a repository `r` on `main` with `a.txt` (30 lines), `b.txt` (`port = 80`), `c.txt` and `d.txt`, committed as `base`; a branch `feat` that changes
line 3 and line 20 of `a.txt`, `port = 8080` in `b.txt`, line 9 of `c.txt` and line 1 of `d.txt`; `main` then changes the same two lines of `a.txt`
(adding a line after line 20), `port = 9000` in `b.txt` and line 1 of `c.txt`, and deletes `d.txt`. A worktree `w` on a branch `side` from `base` with
a commit to `b.txt` runs `git cherry-pick feat` and stops on four places; `r` runs `git merge feat` and stops on `a.txt` (2 places), `b.txt` (1) and
`d.txt` (deleted on `main`), `c.txt` merging by itself. A third worktree `x` on `feat`, clean. michi writes each pick into the file and runs
`git add` when the file has no open place left, `git checkout --ours|--theirs` or `git rm` for a file that clashed whole, `git checkout --conflict=zdiff3`
to start a file over, `git commit` (a rebase: `git rebase --continue`) to commit, and `git merge --abort` (and the like) to call it off, keeping the
files under `refs/michi/backup/<time>` first.

| # | Do | Expect |
| --- | --- | --- |
| 21.1 | Open michi on `r` | The tab `Conflicts` (`4`) carries `● 8` in red (the places left in every worktree); the title bar has `· merging`; `4` opens the view: the bar `Merging feat into main`, the pill `4 / 4 left` and the button `Call the whole merge off` |
| 21.2 | Look at the left column | `Files that clashed 3`: `a.txt`, `b.txt`, `d.txt` with their places left; `Merged by themselves 1`: `c.txt` `both changed it, apart`; `Merge`: `feat <id> into main <id>.`, `The base is <id>, where the two parted.`, `Since <time>.` and `“Ours” is main, where you are; “theirs” is feat, coming in.`; `Other worktrees stopped 1`: `w` with its 4 |
| 21.3 | Look at the first place | `a.txt, line 3` with `● to solve`; the columns `Ours` (blue) and `Theirs` (green) with their lines and line numbers, `… N lines the same …` between places; `Result` reads `Pick with a button below, or fix it in an editor and save`; a `Hint` line; the buttons `Take ours`, `Take theirs`, `Both (ours first)`, `Both (theirs first)`, `Back to base`; the current place has a red edge |
| 21.4 | `Show base`; `Hide base` | A `base` column with the base's lines appears between the two and goes again |
| 21.5 | `o` on the first place; `t` on the second; `[`, `o`; on `b.txt` `b` | Each pick puts its tag (`ours`, `theirs`, `both (ours first)`) on the place, hides its buttons, turns its edge green and moves on; the pill and the tab's `●` count down and the file's count in the left column turns to a green `✓` once its places are picked, `git diff --cached --name-only` then having it; `o` on a solved place says `That place is solved already: “Start this file over” puts it back` |
| 21.6 | On `d.txt` | `Deleted on our side, changed on theirs.` with `Delete it (as our side)` and `Take their file`; `o` deletes it (`d.txt` leaves `git status`'s unmerged files, `main` having deleted it already); `b` there says `A file that clashed whole takes one side: o or t` |
| 21.7 | Click `Start this file over` on `a.txt` | The toast `a.txt is back as git left it; what it had is kept in michi/backup/<time>`; its places are open again with `● to solve`, the pill and the tab count up; `git show refs/michi/backup/<time>:a.txt` has the picks |
| 21.8 | Edit a place's lines in an editor and save | Within a few seconds the place shows the tag `by hand` and counts as solved |
| 21.9 | With places open press Ctrl+Enter | The toast `N left to solve first`; nothing is committed |
| 21.10 | Solve the rest | The bar under the places: `All solved.`, the message box with git's message (`Merge branch 'feat'`) and `Commit`; the view scrolls to it once the last place is solved; clearing the message and pressing Ctrl+Enter says `Write the commit's message first` |
| 21.11 | Ctrl+Enter | The toast `Committed the merge on r: <id>`; the title bar loses `· merging`; the view moves to `w`'s cherry-pick (`Cherry-picking <id> onto side`, `Call the whole cherry-pick off`) |
| 21.12 | In `w` solve all four, Ctrl+Enter | `Committed the cherry-pick on w: <id>`; `git log -1 --format='%an %ad'` in `w` has `feat`'s author and date; with nothing left to solve the tab `Conflicts` goes and the view moves to the graph; `4` then says `Nothing clashed: the 4 view opens while a merge or the like stops on one` |
| 21.13 | Set the two up again; click `Call the whole merge off`; `n`; click it again and `y` | A row under the bar: `Stop the merge and put main back as it was before it (git merge --abort). The files are kept in michi/backup first.` with `Keep on` / `Call it off`, wrapping in a narrow window; `n` closes it; `y`: the toast `Called the merge on r off; its files are kept in michi/backup/<time>`, `git status` in `r` is clean and the view moves to `w` |
| 21.14 | Click `w` under `Other worktrees stopped` | The view shows `w`'s cherry-pick; the tab's `●` still counts both |
| 21.15 | With `r` in the merge: `p`; `C` on a commit in `1`; Ctrl+Enter in `2` with a file staged; `3`, `m` on `w` while `w` is in its cherry-pick | `r is in the middle of a merge: pull waits until it is over`; `r is in the middle of a merge: finish it first`; `r is in the middle of a merge: commit it in the 4 view`; `w is in the middle of a cherry-pick: finish it first`; nothing runs; a fetch and other worktrees' pulls go on |
| 21.16 | With nothing in progress, in `x`'s window: `C` on a `main` commit that clashes; `y` | The band `Pick <id> “<subject>” onto feat in x?` with `Go ahead y` / `Not now Esc`; `y`: `Picking <id> onto x clashed in <files>: solve it in the 4 view` and the `4` view opens on it; a commit that picks cleanly: `Picked <id> onto x: <new>`; a merge commit: the band asks against which parent, with `against parent 1 1` / `against parent 2 2` (a revert's says `keep parent 1`) |
| 21.17 | `3`, `m` on `main`'s own row; on `x`; `y` | `r is on main itself: there is nothing to merge`; the band `Merge main into feat in x?`; `y`: `Merging main into x clashed in <files>: solve it in the 4 view` and the `4` view on `x`; when it merges cleanly: `Merged main into x: <new>` |
| 21.18 | `t` (revert) on a commit whose revert clashes; `y` | `Reverting <id> clashed in <files>: solve it in the 4 view`; the view on it (`Reverting <id> on main`) and Ctrl+Enter commits it |
| 21.19 | With a Claude working in `r` (from tsumugi), solve all and Ctrl+Enter | The band `A Claude is working in r. Commit the merge there anyway?`; Esc commits nothing; `y` commits (skipped: needs a real Claude or tsumugi session) |
| 21.20 | Press `?` in `4`; narrow the window to its smallest; switch to the light theme | Help lists `] [ Next / previous place`, `o Take ours`, `t Take theirs`, `b B Take both (ours first / theirs first)`, `Ctrl+Enter Commit once all is solved`; the bar's pill and button stay visible, long lines end in `…` or wrap; readable in both themes (`<Size>` and `<Theme>` now exist, but the row needs a conflict in the lab and was not run) |
| 21.21 | In a worktree stopped by a merge (the `r` repo of #23), open the graph (`1`) and the worktree table (`3`) | The graph's top has a red-ringed row with dashed edges to both parents, `⚠ 衝突 残り N` and `マージ中: feat → main`, the hash `MERGE`; solving every place in `4` turns it to `✓ 解けた`. No `⚠ 衝突の予測` pill anywhere; the table's clash column is `—`, the pairs show `·` with「予測は衝突の解消（4）の実物に変わりました。」 |
| 21.22 | While `r` is stopped by the merge, look at the graph's tool band and hover `Pull` and `Stash`; press `p` | `Pull` and `Stash` are dimmed, the hover says `Held while a merge is under way`; `p` toasts why it waits; the other buttons are unchanged |
| 21.23 | In a repository with no `rerere.enabled` (`git config --get rerere.enabled` prints nothing), make a merge clash so `4` opens | A dialog asks whether to remember solved clashes (`y` / `n` / Esc); `y` toasts that rerere is on and `git config rerere.enabled` says `true`; in a second clash (after `git merge --abort`) no dialog comes again this session. With `n` the config says `false` and the band never returns |
| 21.24 | With `rerere.enabled=true` and a recorded resolution (solve a clash, `git rerere`, `git merge --abort`), merge the same branch again so `4` opens | The file rerere solved shows `✓ ↺ 前に解いた形` on its places and the bottom bar says how many are such; `Enter` on it takes the mark away (the count drops); `g x` starts the file over with open places and a toast naming the backup under `refs/michi/backup`, and the same clash is no longer solved by rerere on a new merge. `g x` on a place rerere did not solve toasts that it did not |
| 21.25 | Merge two branches that changed the same lines in different commits so `4` opens, and select a place | Under the place's header two rows appear, `自分` and `相手`, each with the short hash, subject, author and date of the commit that changed those lines on that side (a side no commit touched says so); they show a moment after the place is selected, not at once, and the window never stalls; a long subject is clipped, not wrapped |
| 21.26 | In the `4` view select an open text place and press `e`; edit the lines; Ctrl+Enter. Then `e` on a solved place and on a file that clashed whole; `e` and Esc | The result box turns into a field holding ours then theirs, with a hint (Ctrl+Enter solves, Esc leaves); Ctrl+Enter solves the place with exactly what was written (tag `ここで書いた`, the file shows it after the next read) and moves to the next open place; Esc leaves the place as it was; a solved place says to restart the file, a whole-file clash says to take a side; the field keeps the keys (letters type, they do not pick) |
| 21.27 | Merge two branches that changed one line in different words (e.g. `let a = 1;` to `let a = 22;` and to `let a = 3;`) so `4` opens | In the ours and theirs columns only the words that differ from the base are on a stronger ground (`22`, `3`); with the base column hidden or unknown they are compared with the other side; a line one side added is marked whole; unchanged lines are plain; long lines are still cut with … |
| 21.28 | In `4` with places left and a Claude (tsumugi tab) in that worktree, press `g` or click「Claude に渡す」in the bar; then do the same in a worktree with no Claude tab | A toast says the Claude was asked / a new tab started; in the first case the request line (path of the brief + 「Do not commit」) is typed into that Claude's tab; in the second a new tsumugi tab named after the worktree starts `claude` with it; the brief (`%TEMP%\michi-handoff\<worktree>-<time>.md`) lists each open place with ours / base / theirs and both sides' intent; the repository itself gets no new file; when the Claude adds the files the remaining count falls by itself; with nothing left `g` only says so (skipped: needs a real Claude or tsumugi session) |
| 21.29 | In `4` with a file that clashed whole (deleted on one side, or binary), click「外部ツールで開く」or press `w`; once with `merge.tool` set in git config (e.g. `vscode`, `meld`), once without | With a merge tool: `git mergetool` opens that tool on the file, and when it is closed a toast says so, the file is added if the tool reported success, and the count falls; without: the file opens in the program the system has for it (Explorer / `open` / `xdg-open`) and the toast says so; a failure shows git's first line that is not a warning; with a file deleted on one side and a tool set, git wants a terminal to ask which side to keep, and the toast says to use `o` or `t` |
| 21.30 | Hand a conflict to a Claude with `g` (21.28); keep michi open while that Claude works until it stops and waits for you | While it runs nothing is said; once it stops (waiting or done) a toast「<worktree> の Claude が止まりました（待ちか済み）: diff を見てください」shows for 10 s, once | [ ] (skipped: needs a real Claude or tsumugi session) |
| 21.31 | As 21.30: after the stop toast | A second toast says how many places still have conflict markers, or that none are left; a Claude handed notes (13.13) gets no second toast | [ ] (skipped: needs a real Claude or tsumugi session) |
## 22. Branch changes from the picker and the graph

| # | Do | Expect |
| --- | --- | --- |
| 22.1 | Press `b`, select a local branch not checked out, `F2`, type a new name, Enter; then `F2` and Enter with the same or a taken name | The box shows `Rename <old> to:` with the old name in it and no rows; Enter toasts `Renamed <old> to <new>` and the graph's chip shows the new name after the next read; the same name does nothing, a taken or bad name toasts git's reason; Esc closes | [~] (2026-10-25: Enter after F2 and a typed name renames the branch in a real window, `git branch` showed it; same/taken-name cases not tried) |
| 22.2 | With a remote branch of the same name (`origin/x`), press `b`, select the local `x`, `Ctrl+U`; then on a branch with no such remote | Toast `x now follows origin/x` (`git rev-parse x@{upstream}` says so); without a remote of that name a toast says no remote branch is named so | [x] |
| 22.3 | In a worktree on a branch, press `b`, select another branch (or a remote one), `Ctrl+R` and yes; once more with a clashing branch | A band names the branch, the worktree and the target; yes rebases and toasts the backup of the old tip (`refs/michi/backup/…`); a clash leaves the worktree as it was (no rebase in progress) and toasts git's words; the same branch or a worktree in the middle of something is refused with a toast; a Claude at work adds its warning to the band | [x] |
| 22.4 | In the graph select a commit and press `m`, yes; once on a clashing commit | A band names the commit and the window's branch; yes merges (a toast with the new commit); a clash opens the `4` view; on the uncommitted row or a detached worktree a toast says why not | [x] |

## 23. Notifications, bug report and window

| # | Do | Expect |
| --- | --- | --- |
| 23.1 | With the window behind another, let a handed Claude stop; then make a fetch fail (a remote that is not there); open settings, the Notifications page, turn each column on and off, press the test buttons | The system's notification with michi's icon, and a sound if on, come for each event only while the window is not in front; a webhook set on the page gets the event; the test buttons send at once; edits are written to `config.toml`'s `[notify]` and a hand edit is read back | [ ] (skipped: needs OS or machine features the harness lacks) |
| 23.2 | Press `F12` in any view; read the form that opens; then add `{ on = "<F9>", run = "bugreport" }` to `prepend_keymap` in `keymap.toml` and press `F9` | The browser opens michi's issue form with `version` (`michi 0.41.0 (x86_64)`), `os` and `context` filled (Scale, Config named like `uchmk\michi\config.toml`, the last failure if there was one) and no user name in any of it; a toast says nothing is sent until you submit; `F9` does the same | [ ] (skipped: needs editing keymap.toml while open and F9) |
| 23.3 | Move and resize the window, wait a second, close it, start michi again; maximise, close, start again; start with `--keys` and move the window | The window opens where and as big as it was (on the same screen, maximized if it was, with the size it had before when you restore it); `window.toml` sits in the state folder (row 1.30); a start with `--keys` neither reads nor writes it | [x] (skipped: the harness cannot move the window or restart it with the saved place) |

## 24. Command palette

| # | Do | Expect |
| --- | --- | --- |
| 24.1 | Press `Ctrl+Shift+P` (or click the field in the title bar); type `fet`, `branch`, `wor`; arrows or `Ctrl+N` / `Ctrl+P` move, Enter picks; press Esc; open it over the Settings; open it while the branch box is open | A box at the top lists the keymap's commands, the tabs and the keys of the view in front, each with its key at the right; typing narrows and lights the letters; Enter does what the key does (a fetch starts, the branch box opens, the Worktrees tab shows); Esc or a click outside closes it; over the settings a pick returns to the main window; with the branch box open it does not open | [x] |

## 25. Warnings from keymap.toml / config.toml

| # | Do | Expect |
| --- | --- | --- |
| 25.1 | Write a bad line into keymap.toml (an unknown key name), save it while michi is open; then fix it | A toast names the number of warnings and the first one; it is not said again until the warnings change; fixing the file says nothing | [~] (skipped: needs editing keymap.toml while open) |

## 26. Dragging the column widths

| # | Do | Expect |
| --- | --- | --- |
| 26.1 | In the graph view, drag the two lines between the side column, the graph and the detail (a line shows when the pointer is near); double-click a line; with a wide diff drag the line after the graph; close and open michi | The pointer turns to a left-right arrow over a line; the columns follow the pointer and no column goes under its least width; a double click halves that split; the widths come back at the next start; a double click on an edge brings the default width back | [~] (`<Drag>` on the side/graph and graph/detail lines widened the side column; 2026-10-25: a real drag of the graph/detail line wrote `graph_ratio = 0.542` to layout.toml at once and a double click on it wrote `0.500` (halved) at once; the pointer shape and the restart look not tried) |
| 26.2 | Drag the left edge of Author, Date and Commit in the graph's header | The column gets wider as the edge goes left, narrower to the right (40–300); the rows follow; a double click on an edge brings the default width back | [~] (`<Drag>` on the Commit header's left edge: dragging left made the column wider and the rows followed; Author/Date not shown at this width; double click and the 40–300 limits not tried) |
| 26.3 | In the worktree table drag the left edge of each header (Claude, from main, uncommitted, unseen, clash, buttons) and the edge between the name and Claude | Each column follows the pointer; the name and Claude share what is left; the widths come back at the next start; a double click on an edge brings the default width back | [ ] (0.106.11: the squeezed window stored the drawn width every frame, so a drag ratcheted the column narrower; fixed. `<Drag>` on the uncommitted header's left edge from 0.4255 to 0.3734 now widens that column (the edge went left about 19 px, not all 100 because the window is squeezed and the others give); not checked by hand: the Claude edge, the double click, the restart) |

## 27. The branch / tag / stash / remote palette on ito-palette

| # | Do | Expect |
| --- | --- | --- |
| 27.1 | Open `b`, `T`, `s` and the remote choice of a push; type, move with the arrows and `Ctrl+N` / `Ctrl+P`, click a row, press Esc, click outside | The same box as before: the matched letters lit, the make / save rows in the accent colour, the `for <worktree> at <commit>` line above and the keys below; a click on a row is Enter on it; Esc and a click outside close it | [~] (partly seen 2026-10-22: b box with fe typed shows the matched letters lit, feat first, a make row for the commit and the key line below; arrows, Ctrl+N, Ctrl+P and clicks were not told apart) |
| 27.2 | In `b`: `Shift+Enter`, `Delete`, `F2` (type a name, Enter; Esc cancels), `Ctrl+U`, `Ctrl+R` on a row | Each does what it did in v0.48.0; during a rename only the box shows | [ ] (partly seen 2026-10-22: F2 shows only the rename box (`b の新しい名前: zzb`) and Esc cancels; Shift+Enter, Delete, Ctrl+U, Ctrl+R not tried here) |

## 28. The repository screen

| # | Do | Expect |
| --- | --- | --- |
| 28.1 | Press `Ctrl+O` (also: `Ctrl+Shift+P`, type "repo", Enter); Esc; `Ctrl+O` twice | The repositories michi was opened on, newest first, the folder name with its parent beside it and "here" on the one in the window; Esc and the second `Ctrl+O` close it; no other key reaches the views while it is open | | [ ] (seen in c1: Ctrl+O opens the repository list with here marked on the open one, and typed keys go to its filter; closing by Esc or a second Ctrl+O not seen) |
| 28.2 | Type part of a name, two words, then clear; move with the arrows; press Enter on another repository | The list narrows to the paths holding every word; Enter switches this window: the title, the graph, the worktree table and the Claude column are the other repository's, the old one is no longer read | | [x] |
| 28.3 | Type `C:\dev\some-repo` (or `~/...`, `./...`), Enter; then a folder that is not a repository | A first row "Open <path>" in the accent colour; a repository opens and joins the list; a folder that is not one leaves the window as it was and says so in a toast | | [x] |
| 28.4 | `Delete` on a row; close michi and open it again | The row is gone, and stays gone; the repositories opened since are at the top (`recent.txt` in michi's folder) | | [x] (skipped: needs closing and reopening michi) |

## 29. 確かめの窓（ito-confirm）

| # | Do | Expect |
| --- | --- | --- |
| 29.1 | In the graph press `t` on a commit (revert) | The window dims and a box in the middle asks `Are you sure?` / `よろしいですか` with the band's words, the buttons `Go ahead` and `Not now`, and `y Go ahead · Esc Not now` under them; nothing behind answers the mouse (looked at 2026-10-22: the box shows `やめる Esc` and `進める Ctrl+Enter` buttons with `y はい · n いいえ` in the key bar, not `Go ahead` / `Not now`; wording to be reconciled with the row) | | [~] |
| 29.2 | Press `y`; then ask again and press `Esc`, then `n`; then click each button | `y` and the first button run it; `Esc`, `n` and the second button drop it | | [x] |
| 29.3 | Revert a merge commit | One button per parent (`keep parent 1`, `keep parent 2`) and `Not now`; `1` / `2` and a click run it with that side; `y` does nothing | | [x] |
| 29.4 | Shrink the window to its smallest with the box up | The box narrows and its words wrap; every button stays clickable | | [~] (skipped: harness cannot resize the window) |

## 30. 文字の選択とコピー（ito-select）

| # | Do | Expect |
| --- | --- | --- |
| 30.1 | Select a commit; drag the mouse over some diff lines, the subject and the body | The text under the drag is lit in the theme's selection colour; letting go copies it (paste it in a terminal) | | [~] (`<Drag>` over the commit hash lit the text in the selection colour; the copy on release not checked) |
| 30.2 | Double-click a word, then triple-click a line; press Ctrl+C after a drag | A word, then the line is picked up and copied; Ctrl+C copies it again | | [x] (skipped: needs drag or wheel, which the harness lacks) |
| 30.3 | Put a web address in a commit message; double-click it; Ctrl+click it | The whole address is picked up; Ctrl+click shows the pointing hand and opens it in the browser | | [~] (skipped: needs a double-click and the OS browser, which the harness lacks) |
| 30.4 | Click a parent hash, a file row and a merged commit; click an empty place | The click still does what it did (go to the commit, scroll to the file); an empty click lets the selection go | | [x] |
| 30.5 | Select a diff line and move to another commit | The selection is gone | | [x] |

## 31. michi mcp（Claude Code に worktree の様子を渡す）

| # | Do | Expect |
| --- | --- | --- |
| 31.1 | With michi open on a repository with two worktrees, run `claude mcp add michi -- michi.com mcp`, then ask Claude Code "which worktrees are behind main?" | Claude calls `michi_worktrees` and answers with each worktree's branch, ahead / behind and uncommitted files | | [x] (skipped: needs a real Claude or tsumugi session) |
| 31.2 | Make two worktrees edit the same file, wait for michi's prediction, ask "which worktrees would conflict?" | `michi_conflicts` names the pair and the file | | [x] (skipped: needs a real Claude or tsumugi session) |
| 31.3 | Close michi and ask again | The tool answers "michi is not running; start michi and ask again" | | [x] (skipped: needs the external ask tool and closing michi) |
| 31.4 | Open two michi windows, close the first, ask again after half a minute | The second window answers | | [x] (skipped: needs two windows and the external ask tool) |

## 32. 変更の画面の diff の選択とコピー（ito-select）

| # | Do | Expect |
| --- | --- | --- |
| 32.1 | In the `2` view pick a file; drag the mouse over some diff lines | The text under the drag is lit in the theme's selection colour; letting go copies it (paste it in a terminal) | | [~] (`<Drag>` over the added lines in the diff lit the text under it in the selection colour; the copy on release not checked) |
| 32.2 | Double-click a word, then triple-click a line; press Ctrl+C after a drag | A word, then the line is picked up and copied; Ctrl+C copies it again | | [ ] (skipped: undecidable by the harness: it has no double/triple click word and cannot read the clipboard) |
| 32.3 | Click a file row, a worktree chip, the amend switch and Commit | Each still does what it did; an empty click in the diff lets the selection go | | [ ] (skipped 2026-10-22: needs a tracked file with selectable lines or a conflict; the lab only has a new file whose lines cannot be selected) |
| 32.4 | Select a diff line and pick another file or switch the worktree | The selection is gone; the commit view's selection is not drawn here | | [ ] (skipped 2026-10-22: needs a tracked file with selectable lines or a conflict; the lab only has a new file whose lines cannot be selected) |

## 33. 衝突の画面の文字の選択とコピー（ito-select、`4`）

| # | Do | Expect |
| --- | --- | --- |
| 33.1 | Open a conflict (`4`); drag the mouse over some lines of the ours and theirs columns, and the result | The text under the drag is lit in the selection colour; letting go copies it (paste it in a terminal); line numbers are not picked up | | [ ] [~] harness 0.106.11 (1400x900, lab cf1 merge): dragging across the base and ours/theirs columns of the conflict lights the text under the drag ("three" in base) and not the line numbers; the clipboard is not readable, so the copy is not judged |
| 33.2 | Double-click a word, then triple-click a line in a side, in the context lines and in the result; press Ctrl+C after a drag | A word, then the line is picked up and copied; Ctrl+C copies it again | | [ ] (skipped: undecidable by the harness: it has no double/triple click word and cannot read the clipboard) |
| 33.3 | In a rebase, drag over the ours / theirs commit lines of the intent | The subject and author are picked up | | [ ] (skipped: needs drag or wheel, which the harness lacks) |
| 33.4 | Click a place, a side's pick button, `base` and the buttons of the result; click an empty place | Each click still does what it did (select the place, choose a side, show the base); an empty click lets the selection go | | [ ] (skipped 2026-10-22: needs a tracked file with selectable lines or a conflict; the lab only has a new file whose lines cannot be selected) |
| 33.5 | Start writing a result by hand; drag and double-click inside the text box | The text box selects its own text as before; nothing outside it is lit | | [ ] (skipped: undecidable by the harness: it has no double click word and cannot read the clipboard) |
| 33.6 | Select some lines, then scroll the places or solve a place | The selection is gone (no wrong text stays lit) | | [ ] (skipped 2026-10-22: needs a tracked file with selectable lines or a conflict; the lab only has a new file whose lines cannot be selected) |

## 34. Selecting text in the review (ito-select)

| # | Do | Expect |
| --- | --- | --- |
| 34.1 | Press `d` over a worktree; drag the mouse over some diff lines | The text under the drag is lit in the theme's selection colour; letting go copies it (paste it in a terminal); no comment opens | | [~] (`<Drag>` over the diff of wt3 lit the text in the selection colour and no comment opened; the copy on release not checked) |
| 34.2 | Double-click a word in the diff, triple-click a line, press Ctrl+C after a drag | A word, then the line is picked up and copied; Ctrl+C copies it again | | [ ] (skipped: undecidable by the harness: it has no double/triple click word and cannot read the clipboard) |
| 34.3 | Single-click a diff line | The cursor moves to the line and its note can be written (`n`), as before | | [ ] (skipped 2026-10-22: needs a tracked file with selectable lines or a conflict; the lab only has a new file whose lines cannot be selected) |
| 34.4 | Select a diff line, then pick another file or switch the wide view | The selection is gone | | [ ] (skipped: needs a mouse drag to select diff text) |
| 34.5 | Select diff text, close the review (Esc), select a commit's detail text | No stale selection shows in either place | | [ ] (skipped: needs a mouse drag to select diff text) |

## 35. Answering a waiting Claude from the notification (W)

| # | Do | Expect |
| --- | --- | --- |
| 35.1 | With tsumugi running, let a Claude in a worktree reach a question (a permission prompt) while michi is open | A toast `The Claude in <wt> is waiting: <question> (W: answer)` shows for about 15 s; with the window behind another, the system notification `michi: a Claude is waiting` shows too (it follows the `done` settings of Notifications) | | [ ] (skipped: needs OS or machine features the harness lacks) |
| 35.2 | Press `W` from any view | The window goes to the worktree view with that worktree selected, its branch review (the diff) open, and the question with an answer line over the bottom of the window | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 35.3 | Type `y` (or any text) and press Enter | The text is typed into that Claude's tsumugi tab and submitted; the line closes and the toast says `Sent the answer to tab N`; the Claude goes back to running | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 35.4 | Type the answer with a Japanese IME and confirm the conversion with Enter | The conversion is confirmed without sending; the next Enter sends | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 35.5 | With two Claudes waiting, press `W` twice | The first press goes to the one that has waited longest, the second to the next one, then round | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 35.6 | Press Esc in the answer line | The line closes; the review stays open | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 35.7 | Answer the Claude in its own tab while the line is open | The line closes by itself once michi sees the Claude is no longer waiting | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 35.8 | Press `W` with no Claude waiting | The toast says `No Claude is waiting` | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 35.9 | Start michi while a Claude is already waiting | No toast for it (only waits that begin while michi runs are told); `W` still goes to it | | [ ] (skipped: needs a real Claude or tsumugi session) |

## 36. Warning about overlapping work in New work (`N`)

| # | Do | Expect |
| --- | --- | --- |
| 36.1 | Change a file (say `crates/michi/src/app.rs`) in one worktree and leave it uncommitted; press `N` and type `fix app.rs` in the instruction | A line `⚠ Heads up: <branch> is changing crates/michi/src/app.rs …` shows under the instruction as you type; nothing else in the form is blocked | | [ ] (skipped: needs uncommitted edits in a worktree plus the N form over them, not set up) |
| 36.2 | Type a folder instead (`tidy crates/michi/src/`) | The same worktree is named with the files under that folder (the first three, then `+N`) | | [ ] (skipped: same setup as 36.1) |
| 36.3 | Type an instruction with no file or folder name, or names no worktree is changing | No warning shows | | [ ] (skipped: same setup as 36.1) |
| 36.4 | With the warning showing, press Ctrl+Enter | The work starts as usual (the warning does not stop it) | | [ ] (skipped 2026-10-22: needs the warning band of a started task; not set up) |
| 36.5 | Two worktrees change the same file | Both are named in the warning | | [ ] (skipped: needs two worktrees changing one file, not set up) |

## 37. Open a worktree's folder in a terminal, editor or file manager (#75)

| # | Do | Expect |
| --- | --- | --- |
| 37.1 | In the worktree tab select a row; press `t` | A tsumugi tab opens in that worktree's folder; a toast says it was opened | | [ ] (skipped 2026-10-22: needs tsumugi) (looked at on a real window in long1: blame screen titled blame: a.txt, as of the commit, rows with commit, who, ago, line number and text, 300 lines open at once) |
| 37.2 | Press `e`, then `E` | VS Code opens on the folder; the file manager shows the folder (Explorer on Windows) | | [ ] (skipped: needs OS or machine features the harness lacks) (skipped: the harness cannot tell which file the detail was scrolled to) |
| 37.3 | Click the Terminal, Editor and Explorer buttons on a row (also on the main worktree's row) | Same as the keys, for the clicked row | | [ ] (skipped 2026-10-22: needs tsumugi) (looked at on a real window: Home 1 / 300, End 300 / 300, PageDown moved 20 rows, 22 rows shown) |
| 37.4 | Open the palette (Ctrl+Shift+P) and type "worktree in" | Three entries (terminal, editor, file manager); picking one opens the window's worktree (the selected row when the worktree tab is in front) | | [ ] (skipped 2026-10-22: palette entries not told apart in the harness) |
| 37.5 | Write `[open]` `editor = "notepad {path}"` into config.toml, wait 2 s, press `e` | Notepad is started with the folder as its argument; Settings > General shows the line | | [ ] (skipped 2026-10-22: needs a config.toml edit and Notepad, an OS app) (skipped: needs a repository with a past rename, not in the lab) |
| 37.6 | Write `editor = "no-such-program"`, press `e` | A toast "open failed: ..." with the program's name; michi keeps running and does not freeze | | [ ] (skipped 2026-10-22: needs a config edit) (undecidable: the screenshot after Enter lags and was not seen) |
| 37.7 | Write `editor = "a \"b"` (unclosed quote) | A warning about config.toml names the key; `e` still uses the default | | [ ] (skipped 2026-10-22: needs an unclosed-quote config.toml and its warning; not tried) |

## 38. Key hints on hover

| # | Do | Expect |
| --- | --- | --- |
| 38.1 | Hover the gear at the top right and the search field beside it | The gear tooltip reads `Settings (Ctrl+,)`; the search field shows its key (`Ctrl+Shift+P`) inside the field, and its tooltip names the command palette | | [ ] | | | [ ] (skipped: hover tooltips cannot be driven by the harness) |
| 38.2 | Bind another key to `palette` in keymap.toml (prepend), restart, hover the search field | The field shows the new key | | [ ] | | | [ ] (skipped: hover tooltips cannot be driven by the harness) |
| 38.3 | Hover each view tab (Graph, Changes, Worktrees) | The tooltip names the view and its digit, for example `Graph (1)` | | [ ] | | | [ ] (skipped: hover tooltips cannot be driven by the harness) |
| 38.4 | In the graph view hover Fetch, Pull, Push and Sync | Each tooltip shows its key, for example `pull (p)`, `Sync (S)` | | [ ] | | | [ ] (skipped: hover tooltips cannot be driven by the harness) |
| 38.5 | During a merge hover Pull and Stash | The tooltip still says they are held, not the key | | [ ] | | | [ ] (skipped: hover tooltips cannot be driven by the harness) |
| 38.6 | In the worktree view hover New work and the re-read button | `New work (N)`; the re-read button shows just its name (skipped: undecidable in the harness, hover tooltips do not show) | | [ ] | | | [ ] (skipped: hover tooltips cannot be driven by the harness) |

## 39. Graph filter with fuzzy matching (ito-match)

| # | Do | Expect |
| --- | --- | --- |
| 39.1 | Press `/` over the graph and type the first letters of a long subject, skipping the rest (e.g. `fxflk` for "Fix the flaky test") | The commit is a hit (the others fade) and the hit count shows it | | [ ] (seen in m1: / then ahon lights the matching rows and fades the rest, the hit count shows 14) |
| 39.2 | Type two words, one of a subject and one of an author or a branch name | Only commits that hold both words are hits | | [ ] (skipped: the lab has no commit whose author or branch word separates the hit sets (feat main gave 13 hits against 14, rows not told apart)) |
| 39.3 | Type the first 7 characters of a commit hash, then characters from the middle of one | The first selects that commit; the middle of a hash hits nothing by itself | | [ ] (looked at 2026-10-22: selection did not move on a hash hit; a mid-hash hit gives 0; not marked) |
| 39.4 | Type a word, then add letters; then take letters off | Adding letters never lights new rows; taking them off lights rows again, with no lag on a long history | | [ ] (skipped 2026-10-22: needs a long history and timing) |
| 39.5 | Type in capitals (`FIX`) | The same hits as `fix` | | [ ] (seen in m1: FEAT B and feat b both give 14 hits with the same rows lit) |

## 40. What a push sends

Set up: as in 15 (a bare `origin`, a clone `a`, a second clone `b`); in `a` a worktree on `feat` with three commits not pushed.
Before the push michi reads what the last fetch left (no network) and lists the commits and the place; `P` and `F` both ask first. Only `y` runs `git push`.

| # | Do | Expect |
| --- | --- | --- |
| 40.1 | On `feat` (not on `origin` yet) press `P`; Esc; `P` again and `y` | A dialog (Esc cancels, Ctrl+Enter or `y` goes on): `Push 3 commit(s) to origin/feat, which is not there yet? It becomes this branch's upstream.` with one row per commit (short hash, subject, newest first); Esc pushes nothing (`git ls-remote origin feat` is empty); `y` pushes and says `Pushed to origin/feat, now its upstream` | [ ] (skipped 2026-10-22: needs a real push) |
| 40.2 | Commit once more on `feat` and press `P` | `Push 1 commit(s) to origin/feat?` with just that commit; make 15 commits and `P`: the newest 12 are listed and a last row reads `… and 3 more` | [ ] (skipped 2026-10-22: needs a real push) |
| 40.3 | Let `b` push a commit to `origin/feat`, press `f` here, commit and press `F` | A red band (red outline and note): the commits that go, and the note `origin/feat has 1 commits this branch does not. Push over them? …`; `P` instead only says `F pushes over them, asking first`; `y` pushes with the lease and names the backup | [ ] (skipped 2026-10-22: needs a second clone pushing to origin/feat in the same moment; not set up) |
| 40.4 | Press `P` on a branch that origin already has, on one only behind, and in a repository with no remote | `origin/feat has it already`, `Nothing to push: origin/feat is n ahead (p pulls)` and `No remote …`; no band in any of them | [ ] (skipped 2026-10-22: needs pushed and behind branches and a no-remote repo in turn; not set up) |
| 40.5 | With two remotes and no `origin`, press `P`, pick one | The list of remotes first, then the band for the picked remote | [ ] (skipped 2026-10-22: needs a repo with two remotes and no origin; not set up) |
| 40.6 | Switch the language to Japanese and press `P` | The band reads `n 個のコミットを … に push しますか？`; the commit rows do not run off the box | [ ] (skipped 2026-10-22: needs a real push) |

## 41. Autostash on pull and switch

| # | Do | Expect |
| --- | --- | --- |
| 41.1 | In a worktree with an uncommitted edit to a file the upstream also changed (different lines), press `p` | The pull goes through instead of stopping; the toast says the changes were stashed and put back, with the `michi/backup/...` name; the edit is still there; `git stash list` is empty | | [ ] (skipped: needs a diverged upstream plus a dirty edit, not set up) |
| 41.2 | Same, but the edit touches the same lines the upstream changed | The pull goes through; the toast says the stash clashed in the file; the file has `<<<<<<<` marks and the stash stays in the list and under `michi/backup/` | | [ ] (skipped: same setup as 41.1) |
| 41.3 | Switch a worktree with changes to a branch that would overwrite them (branch picker) | The switch happens, the toast says the changes were stashed and put back (or clashed) | | [ ] (飛ばし: 変更を上書きするブランチ切替の用意と toast の確認が要る (toast は 3 秒で消える)) |
| 41.4 | Pull or switch with no changes, or with changes that do not stand in the way | Behaves as before; no stash, no backup, no extra text in the toast | | [ ] fixed in 0.106.12 (#148): the toast reads 'Pulled 1 commit from …'; re-check by hand |
| 41.5 | Switch to a branch that does not exist, with changes in the worktree | The error is shown as before and the changes are still in the worktree (nothing left in the stash list) | | [ ] (skipped: same setup as 41.1) |

## 42. Reflog screen (`L`, #50)

| # | Do | Expect |
| --- | --- | --- |
| 42.1 | In the graph or the worktree table press `L` | A list opens at the top: HEAD's reflog (`HEAD@{n}`, what happened, the commit's subject) and the backups under `refs/michi/backup/` (tinted), newest first; the window does not freeze on a big repository | | [ ] (looked at 2026-10-22: L opens the list with 14 rows, newest first; backup tint not judged) |
| 42.2 | Reset a branch (`git reset --hard HEAD~1`) in a worktree, then press `L` | The reset is the top `HEAD@{0}` row and the commit it dropped is the row under it | | [ ] (seen: after reset --hard HEAD~1 the reflog top row moves the branch to HEAD~1 and the dropped c3 is the row under it) |
| 42.3 | Move with `j k`, the arrow keys, PageUp/PageDown, Home/End and the mouse; click a row | The selection follows; a click on the selected row asks as Enter does; a click outside the list closes it | | [ ] (partly seen: j k move the selection; in the command palette `j` is typed into its text field, so this row is about another list; arrows, paging, Home/End and the mouse not tried) |
| 42.4 | Select a row and press Enter | The band asks to bring the commit back as a new branch and says nothing moves; `y` makes `recovered/<id>` at that commit (`-2` after it if taken), the list closes and a toast names the branch; HEAD and the worktree are untouched | | [ ] (looked at 2026-10-22: Enter shows the band for HEAD@{3}; y not pressed) |
| 42.5 | After a force push or a branch delete (which keep a backup), press `L` | The backup is listed with the reason michi wrote; bringing it back gives a branch at its commit | | [ ] (飛ばし: force push か branch の削除で backup を作る手順が要る) |
| 42.6 | Press `R` in a repository with no commits and nothing kept | A toast says the reflog and backups are empty; no list opens | | [ ] (seen in empty1: R gives a notification that the reflog and the michi backups are empty; no list opens) |
| 42.7 | Switch the language to Japanese and open the list | The title, hints, band and toast are in Japanese | | [ ] (skipped: switching the language is not in the harness) |

## 43. Reset: soft, mixed or hard (#77)

| # | Do | Expect |
| --- | --- | --- |
| 43.1 | In the graph, select an older commit and press `g r` | A dialog asks how far to reset and shows a card for each of 1 soft / 2 mixed / 3 hard (what it loses, how to undo); mixed is green and marked Recommended; only No is a button | | [ ] (looked at 2026-10-22: three cards, mixed green and recommended, Esc button only) |
| 43.1b | With the reset cards showing, look at the key bar at the bottom | It reads `1-9 pick · n No · Tab · Esc`; there is no `y`. Pressing `y` or Ctrl+Enter does nothing: the cards stay and no reset runs | | [ ] [~] harness 0.106.11 (lab m1, 1100x600, EN): the key bar reads '1-9 Pick · n No · Tab Move · Esc Back · F1 This list'; no y; y leaves the three cards and nothing moves (Tab Move and F1 are extra to the row's text) |
| 43.2 | With a staged change, press `g r` on the previous commit, then `1` | The branch moves; the change shows as staged; the toast names the `michi/backup/…` ref of the old tip | | [ ] (skipped: moves a lab branch; the reset modes are unit-tested) |
| 43.3 | Same, with `2` | The branch moves; the change is unstaged but its text is in the file | | [ ] (skipped: moves a lab branch; the reset modes are unit-tested) |
| 43.4 | With an edited tracked file and an untracked file, press `g r` and `3` | The edit is gone, the untracked file stays; the toast names two backup refs (the tip, the changes); `git show <changes>:file` has the edit | | [ ] (skipped: moves a lab branch; the reset modes are unit-tested) |
| 43.5 | Press `g r`, then Esc (or `n`, or `y`) | Nothing changes (y does nothing: only 1 2 3 choose) | | [ ] (skipped: the band was not on screen in the run (frame lag); needs a real-machine look) |
| 43.6 | Press `g r` in a worktree in the middle of a merge or rebase; with no commit selected | A toast says to finish it first / to pick a commit; no band | | [ ] (seen in empty1: g r with no commit selected gives a notification to pick a commit first; the mid-merge case was not driven) |
| 43.7 | Press `g r` where a Claude is working | The band ends with the note that a Claude is working there | | [ ] (skipped: needs a real Claude or tsumugi session) |

## 44. Ignoring an untracked file (`i`, #49)

| # | Do | Expect |
| --- | --- | --- |
| 44.1 | In the `2` view make an untracked file `x.log` in the worktree, select it in the unstaged list and press `g i` | No question; a toast says `Added /x.log to .gitignore`; the file leaves the list after the next read and `.gitignore` in the worktree's root has the line (made if there was none) | [ ] (looked at 2026-10-22: g i on an untracked file asks nothing; toast says it added /sub/f4.txt to .gitignore, the row leaves the list and a new untracked .gitignore shows) |
| 44.2 | Press `g i` on a tracked (modified) file, on a staged file; then on an untracked file with `[` `]` and `*` in its name | The first two toast that only an untracked file can be ignored and change nothing; the brackets and star are escaped in the line (`/a\[1\]\*.log`) and git ignores only that file | [ ] (seen: g i on a tracked file toasts that only untracked files can be added; a[1].log became /a\[1\].log and only it is ignored; a star cannot be in a Windows file name so that part is not tried) |
| 44.3 | Press `g i` on a file whose line is already in `.gitignore` (an exact line, e.g. after a hand edit) | A toast says the line is in `.gitignore` already; the file is not changed; with a CRLF `.gitignore` the new line gets CRLF | [ ] (skipped: a file whose line is already in .gitignore is ignored, so it never shows in the changes list to press g i on) |
| 44.4 | Open the help (`?`) in the `2` view | `g i` is listed: Add the untracked file to .gitignore (JA: 追っていないファイルを .gitignore に足す) | [ ] (skipped: the help screen did not open behind the repository list in the run) |

## 45. Changes as a tree of folders

| # | Do | Expect |
| --- | --- | --- |
| 45.1 | In `2` over a worktree with changes in nested folders, press `t` (or click Tree in the top bar) | Both lists become folders (a lone chain such as `a/b/c/` is one row) with files under them, folders first; press `t` again for the flat list |
| 45.2 | Click a folder row | It folds (▸) and its files vanish from the list; click again to open it (▾). Staged and unstaged fold apart |
| 45.3 | Click the `+` at the right of an unstaged folder; then the `−` of a staged one | Every file below it goes into the index, then every one (a rename's old path too) comes out; the count at the right matches |
| 45.4 | With a folder folded, press `j` / `k` and the arrow keys | The selection walks the files in the order shown and skips the folded ones; a selection folded away moves to the nearest shown file |
| 45.5 | Stage a folder while a Claude is writing in the worktree | The same question for a yes as for a file (skipped: needs a real Claude or tsumugi session) |

## 46. Comparing two commits or branches

| # | Do | Expect |
| --- | --- | --- |
| 46.1 | In the graph select a commit, then Ctrl+click (Cmd+click on macOS) another | The first row keeps a tint with an outline (the mark), the clicked one is selected; Ctrl+click the marked row again takes the mark off |
| 46.2 | With a mark and a selection press `D` | The review opens over the worktree table with "Comparison" and `a...b`, listing the files that changed from where the two parted (or plain a to b when one is the other's ancestor); the diff on the right is the usual one |
| 46.3 | Press `D` with no mark; press Esc in the graph with a mark | A toast asks to Ctrl+click a second commit; Esc takes the mark off |
| 46.4 | In the comparison press `u` `s` Space `n` `m`; click a file's check box | Nothing happens (no uncommitted, last look, checks, notes or merge); no "seen" count or switches are shown |
| 46.5 | Compare two branch tips, then swap which one you Ctrl+click first | The list is each side's own changes since they parted; `j` `k` `J` `K` `Enter` and PageUp/PageDown work as in the branch review; Esc closes it |
| 46.6 | Open the comparison, press Esc, then press `d` on a worktree | `d` does nothing while the comparison is open; after Esc the branch review of the worktree opens with its own list, checks and notes |

## 47. A worktree on no branch (the strip under the title bar)

| # | Do | Expect |
| --- | --- | --- |
| 47.1 | In the window's worktree run `git checkout --detach` on a commit | Within a moment a strip appears under the title bar: "Not on a branch: the commit … is checked out. Commits made here belong to no branch." with `R Back to <branch>` and `B Branch from here` at the right | | [ ] (seen: strip shows the not-on-a-branch text with R and B at the right) |
| 47.2 | Check out a tag (`git checkout v1`) | The strip names the tag; the body below moves down by the strip's height, nothing is cut off | | [ ] (seen: tag checkout shows the detached strip with R and B) |
| 47.3 | Commit something on the detached HEAD | The strip adds "Commits on no branch yet: 1"; the number follows each new commit | | [ ] (seen: strip adds the count of commits on no branch (1 after one commit)) |
| 47.4 | Press `R` (or click `Back to …`) with such a commit | The worktree is on the branch again, the toast names the backup (`michi/backup/…`) that keeps the commit, the strip goes | | [ ] (seen: R returns to master) |
| 47.5 | Press `B`, type a name, press Shift+Enter | The branch is made at HEAD, the worktree is on it, the strip goes; the commit is on the new branch | | [ ] [~] harness 0.106.11 (lab dt1, detached): B, 'fresh', Shift+Enter makes the branch fresh and checks it out (git branch --show-current = fresh, toast 'Made branch fresh; dt1 is on it'). FINDING: the window still shows the 'Not on a branch' strip, the side column lists only main and the graph has no fresh, even 12 s later (not refreshed) |
| 47.6 | Detach, then delete the branch it came from, then look at the strip | `R` is not offered for a branch that is gone (an earlier existing one, or only `B`) | | [ ] (seen: after the source branch is deleted the strip offers an earlier existing branch (master)) |
| 47.7 | Start `git rebase -i` and stop it at an edit | The strip says it is a rebase, with no buttons; it goes when the rebase ends | | [ ] (skipped: needs an interactive rebase stopped at edit, not set up) |
| 47.8 | With a Claude working in the worktree press `R` | A yes is asked first, as for a switch | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 47.9 | Open the settings (`,`) or the key list (`?`) while the strip is up; switch EN and JA | The strip stays tidy at a narrow width (the text is cut with an ellipsis, the buttons stay); both languages read well | | [~] (`<Size:480x600>`: the settings page and the key list read well in JA at 480 wide; the strip itself and EN not seen) |

## 74. Commit message help (history and template)

| # | Do | Expect |
| --- | --- | --- |
| 74.1 | In the `2` view press `c` and look at the empty box | The hint reads "What changed, and why (Alt+Up / Alt+Down: earlier messages)"; EN and JA both read well and are not cut off | | [~] (looked at 2026-10-22 with `<Lang:en>` and `<Lang:ja>`: the hint reads "What changed, and why (Alt+Up / Alt+Down: earlier messages)" in EN and as expected in JA, not cut off) |
| 74.2 | With the box focused press Alt+Up | The box takes the latest commit's message (all lines); again: the one before; at the oldest nothing happens | | [ ] (looked at 2026-10-22: first Alt+Up fills the latest message; the second step was not told apart) |
| 74.3 | Type "wip", press Alt+Up twice, then Alt+Down twice | You are back at "wip" (the draft is kept); the same message is never listed twice | | [ ] (seen in hist1: wip then Alt+Up steps to the older messages and Alt+Down comes back to wip) |
| 74.4 | Step to an old message, edit a word, press Alt+Up | The edited text is kept as the draft (Alt+Down from the next message comes back to it) | | [ ] (skipped: the frames show the edited word but not the Alt+Down return; frame lag made it undecidable) |
| 74.5 | Run `git config commit.template tpl.txt` (a file with `# help` lines and a `feat: ` line), then press `c` on an empty box | The box is filled with `feat:` without the `#` lines; committing it as it is leaves no `#` line in the message | | [ ] (seen in tpl1: c on an empty box fills feat: without the # lines; committing it was not driven) |
| 74.6 | Type a message first, then press `c` | The template does not replace what was typed; with `A` (amend) the head's message fills, not the template | | [ ] (skipped: needs a template repo and an amend drive in one run; not driven this pass) |
| 74.7 | Commit, then press `c` again | The history has the new message at the top; the template fills the empty box again | | [ ] (skipped: needs a real commit in tpl1; not driven this pass) |
| 74.8 | In a repository with no commits press `c` and Alt+Up | Nothing breaks; Alt+Up does nothing | | [ ] (seen in empty1: c then Alt+Up leaves the box empty with the placeholder, nothing breaks) |
| 74.9 | Click the box with the mouse (not `c`), then press Alt+Up | The history works the same | | [ ] (seen in hist1: after a click in the box, Alt+Up recalls first msg as with c) |

## 86. Row density (compact / standard / cozy)

| # | Do | Expect |
| --- | --- | --- |
| 86.1 | Open the settings (`,`), General: look for "Row density" | Three choices, Compact / Standard / Cozy; Standard is lit by default | | [ ] (seen: the settings General page has a Row density section with three choices; the standard one is lit) |
| 86.2 | Pick Compact | The graph rows, the side column's rows and the reflog / tidy / reorder lists shrink at once (about 20% shorter); text stays readable and nothing is clipped; config.toml has `[view]` `density = "compact"` | | [ ] (seen in m1: Compact makes the graph and side rows about 20% shorter, nothing clipped, lines joined, config.toml has density = compact; the reflog, tidy and reorder lists were not looked at) |
| 86.3 | Pick Cozy | The same rows grow (about 20% taller); the graph dots and lines stay joined across rows; the scrollbar marks still sit at their rows | | [ ] (seen in m1: Cozy makes the rows about 20% taller, dots and lines stay joined, config.toml has density = cozy; the scrollbar marks were not compared) |
| 86.4 | Click a row, scroll with the wheel and drag the scrollbar in each density | Clicks land on the row under the pointer; the selected row stays in view (skipped: needs the wheel and a scrollbar drag, which the harness lacks) | | [ ] (skipped: needs wheel and scrollbar drag, which the harness lacks) |
| 86.5 | Edit `density` in config.toml by hand (also a wrong word) | The rows change within a few seconds without restart; a wrong word gives a toast warning and the standard rows | | [ ] (skipped: editing config.toml while the window runs is not in the harness) |
| 86.6 | Switch EN and JA in the settings | The section and its three choices read well in both | | [~] (looked at 2026-10-22 with `<Lang:en>`/`<Lang:ja>`: the Row density section and its three choices (Compact / Standard / Cozy, 詰める / 標準 / ゆったり) read well and are not cut off in both) |

## 88. Short animations (fade and slide, can be turned off)

| # | Do | Expect |
| --- | --- | --- |
| 88.1 | Press `1`, `2`, `3` in turn (and `4` while something clashes) | Each view fades in and slides up a few px in about 130 ms; it feels soft, not slow, and the keys work at once during the move (skipped: a 130 ms fade cannot be judged from still screenshots) | | [ ] (skipped: a 130 ms fade cannot be judged from screenshots) |
| 88.2 | Open the new-work window, the reflog, the tidy list, reorder, the trial merge, and a Claude reply box | Each window fades in and slides a few px (down from above; the reply box up from below); closing them is at once (skipped: a 130 ms fade cannot be judged from still screenshots) | | [ ] (skipped: a 130 ms fade cannot be judged from screenshots) |
| 88.3 | Open a confirmation band, `?` and the settings | They fade in without a jump; no flicker on the first frame (skipped: a 130 ms fade cannot be judged from still screenshots) | | [ ] (飛ばし: fade は静止画で判断できない) |
| 88.4 | Open the settings (`,`), General: look for "Motion" and pick Off | `config.toml` has `[view]` `motion = "off"`; from now on views and windows appear at once, with no fade or slide, and the hover blends are gone (skipped: the config.toml part reads right, but the fade itself cannot be judged from stills) | | [ ] (飛ばし: fade は静止画で判断できない) |
| 88.5 | Pick On again; edit `motion` in config.toml by hand (also a wrong word) | Animations come back within a few seconds without restart; a wrong word gives a toast warning and keeps them on (skipped: the fade itself cannot be judged from stills) | | [ ] (skipped: needs the animation seen live and a hand-edited config over time) |
| 88.6 | Leave the window idle on a view for a while (Task Manager) | CPU stays near 0: nothing keeps repainting after the animation ends (an idle window kept one thread at ~100% of a core (#140): the thread is the AMD GPU driver's own, in Vulkan and DX12 alike; 0.106.8 draws with GL first on Windows (ito-gpu, as kura does) and measured 16 ms per 5 s; read Task Manager once more) | | [ ] (skipped: needs Task Manager CPU reading, see #140) |
| 88.7 | Switch EN and JA in the settings | The Motion section and its two choices read well in both | | [~] (looked at 2026-10-22 with `<Lang:en>`/`<Lang:ja>`: the Motion section and its two choices (On / Off (reduce motion), 入 / 切（動きを減らす）) read well in both; the JA note wraps to two lines) |

## 94. Review queue of finished Claude work (Review column)

| # | Do | Expect |
| --- | --- | --- |
| 94.1 | Open the worktree table (`3`) with two or more branch worktrees that differ from main | A Review column sits between Unseen and Clash; a worktree never opened in the review (`d`) reads `Not looked at` (dim) with `0 / N files checked`; the main worktree reads `—`; in the narrow card layout it is a `Review` line | [ ] (seen: Review column between Unseen and Clash, wt3 0/1 unseen, wt2 in review, main none) |
| 94.2 | Open a branch's review with `d`, then close it (Esc) and look at the table | That row reads `In review` (blue) within a moment of closing, with the count of files checked | [ ] (seen: wt2 reads blue 確認中 with 0 / 18 files checked) |
| 94.3 | In the review check some files (Space) but not all; then check the last one | `In review` with `k / N`; once every file is checked the row reads `Approved` (green) with `N / N` | [ ] (seen: wt3 reads green 承認 with 1 / 1 once all files are checked) |
| 94.4 | Write a note (`n`) on a line and hand it with `g` | The row reads `Flagged` (yellow) even though all files are checked; the Claude commits again and the row goes back to `In review` or `Approved` by what is checked | [ ] (skipped: needs a real Claude to receive the handed note) |
| 94.5 | Commit in an approved worktree a change to a file already checked | The row leaves `Approved` (the changed file lost its check) without opening the review | [ ] (skipped: needs a Claude commit in an approved worktree, not set up) |
| 94.6 | Drag the Review column's left edge in the header; restart michi | The width follows the drag and is kept; the other columns still resize | [ ] (0.106.14: the cell is cut with … instead of wrapping inside a word, #154; earlier [~] harness, 1500x600 EN, 3 view: a drag of the Review header's left edge to the right narrowed the column from about 170 to 80 px, the other columns moved over; after restarting with the same config the width was the same; at that width the cell wraps badly ('checke / d'); the other columns' resizing not driven) |
| 94.7 | Switch the language EN and JA, and a theme with a light background | The stage words read `Not looked at` / `In review` / `Flagged` / `Approved` and JA `未確認` / `確認中` / `指摘済み` / `承認`; every colour is readable | [ ] (partly looked at 2026-10-22 with `<Theme:light>` and `<Lang:en>`/`<Lang:ja>`: `In review` / `確認中` is readable on the light background in the worktree table; `Not looked at`, `Flagged` and `Approved` not shown in the lab, so not seen) |

## 95. From a commit to the Claude session of its worktree (#95)

| # | Do | Expect |
| --- | --- | --- |
| 95.1 | With tsumugi running, select a commit that is only on a worktree's branch (not on main) whose worktree has a Claude tab | The detail header shows `Claude in this worktree  tab N · <title>` under the names; the title is the tab's | [ ] (skipped: needs a real Claude or tsumugi session) |
| 95.2 | Click that `tab N · title` | The tab comes to the front in tsumugi (same as `o`); a toast says so | [ ] (skipped: needs real tsumugi tabs) |
| 95.3 | Select a commit on main that no worktree has checked out | No Claude line; select the one the main worktree has checked out (with a Claude there): the line shows | [ ] (skipped: needs a real Claude line) |
| 95.4 | A worktree with two or more Claudes | The first two are listed and `+N more` follows; narrow the window and the line wraps | [ ] (skipped: needs two real Claudes and a narrow window) |
| 95.5 | Quit tsumugi (or have no Claude in the worktree) and select the same commit | The line goes within a few seconds; nothing else changes; the WIP row has no such line | [ ] (skipped: needs a real Claude or tsumugi session) |
| 95.6 | Switch EN and JA | `Claude in this worktree` / `この worktree の Claude`, `tab` / `タブ` read well | [ ] (skipped: EN/JA switch not drivable here) |

## 66. Right-click menus for a commit, a branch and a file

| # | Do | Expect |
| --- | --- | --- |
| 66.1 | In the graph (`1`) right-click a commit row | The row is selected and a menu lists Branch (b), Tag (T), Merge (m), Cherry-pick (C), Revert (t), Compare (D), Reset (R), each with its key; a right-click on the WIP row opens nothing |
| 66.2 | Pick Tag from that menu; then Esc, and right-click another row and click outside the menu | The tag palette opens on the clicked commit (not the one selected before); Esc and a click outside close the menu without doing anything |
| 66.3 | While a menu is open, type `d` or `j` | Nothing happens to the view (the keys do not reach the window until the menu is closed) (looked: with the row menu open, d changed nothing and the menu stayed) |
| 66.4 | In the worktree table (`3`) right-click a worktree row | The row is selected and a menu lists Pull request, Open in the terminal / editor / file manager, Branch diff, Rebase onto trunk, Take main, Reorder, Reflog, Remove, with keys; picking Open in the editor opens that row's folder (looked at 2026-10-22: right click on a worktree row selects it and opens the menu (PR, terminal, editor, explorer, diff, rebase, merge, reflog, remove)) |
| 66.5 | Pick Remove for a worktree row | The same question as pressing `x` asks first; nothing is lost without a yes (looked: Remove asks the same question as x (backs up to refs/michi/backup first); Esc dropped it) |
| 66.6 | In the changes view (`2`) right-click an unstaged file (flat list and tree) | The file is selected and a menu lists Stage (Space), Split, Ignore, Discard, Amend; Stage moves the clicked file, Discard asks first (looked at 2026-10-22: right click on a flat-list unstaged file opens the menu with Space, s, g i, g d, g a; tree list not seen) |
| 66.7 | Press `?` and open the palette; switch EN and JA and look at the menus | No operation exists only in a menu (each is a key in the help and the palette); the menu words read well in both languages (skipped: EN/JA switch not drivable here) |

## 87. Diff minimap and next / previous change (`n` / `N`, #87)

| # | Do | Expect |
| --- | --- | --- |
| 87.1 | In `2`, select a file with changes far apart (a long file) | A thin strip at the right edge of the diff shows a tick per run of changes (green added, red deleted, amber changed in side by side) and a lit box over the part in sight; the diff text does not run under it (looked: a strip at the right edge shows three yellow ticks (changed runs) for a 300-line file with three edits) |
| 87.2 | Scroll with the wheel and PageDown | The lit box follows; it fills the strip (or goes) when the whole diff fits (seen [~] in the harness, 1100x600 EN: after the wheel and PageDown the lit box in the strip at the right edge of the diff sat lower than at the start; the whole-diff-fits case not driven, the box is small and the stills barely show it) |
| 87.3 | Click a tick, then drag along the strip | The diff scrolls to that part, centred on the point; it stays inside the file at both ends (seen [~] in the harness, 1100x600 EN: a click low on the strip put the diff at its end, last line at the bottom with no empty space; a drag up the strip put it back at its first line; the centring in the middle not judged, the lab diff is too short) |
| 87.4 | With the focus on the files press `n` repeatedly, then `N` | Each `n` brings the next change to just under the top of the diff, the last one stops there (and does not stick on the one before it); `N` goes back the same way (looked: after n presses the diff sits on the last change and stays there; the middle steps were not told apart) |
| 87.5 | Press `Enter` (side by side) then `n` / `N` | The ticks and the jumps follow the other layout; no change is skipped or repeated |
| 87.6 | Press `l` (pick lines) then `n` / `N` | The cursor moves to the first line of the next / previous run of changed lines and the diff follows it; `N` in the middle of a run goes to that run's start (skipped: l then n/N cursor move not told apart in the frames) |
| 87.7 | Select a file with no diff text (binary, deleted-empty) and press `n` | No strip, nothing happens; no error (skipped: no binary or empty-diff file prepared) |
| 87.8 | Press `?`; switch EN and JA | The key list names `n N` as jumping to the next or previous change in both languages; the strip looks right in Dark and Light themes (skipped: EN/JA switch not drivable here) |

## 100. Trial merge of several worktrees into the trunk

| # | Do | Expect |
| --- | --- | --- |
| 100.1 | In the worktree table press `T` with two or more worktrees on branches other than the trunk | A list opens at the top with those worktrees (branch and folder), each with a `·` mark | | [ ] (飛ばし: trial-merge の直後にハーネスが窓の焦点を失い、結果の画面が撮れない) |
| 100.2 | Press `T` with fewer than two such worktrees | A toast says a trial merge needs at least two worktrees; no list opens | | [ ] (seen: T with one worktree toasts that 2 or more are needed) |
| 100.3 | Press `j` `k` and Space on three rows in the order c, a, b; click a row | The picked rows show 1, 2, 3 in the order picked; Space or a click on a picked row takes it back and later marks shift; `u` clears all (skipped: the harness loses window focus right after the trial-merge run, so the result frame is not captured) | | [ ] (skipped: the harness loses window focus after the trial-merge list opens) |
| 100.4 | Press Enter with nothing picked | A toast asks to pick with Space first (skipped: the harness loses window focus right after the trial-merge run, so the result frame is not captured) | | [ ] (飛ばし: 100.x はフォーカスを失う) |
| 100.5 | Pick two branches that touch different files and press Enter | "Merging in memory…" shows briefly, then each step reads "clean" and the summary counts the changed files with +/- lines (skipped: the harness loses window focus right after the trial-merge run, so the result frame is not captured) | | [ ] (skipped: same focus loss as 100.3) |
| 100.6 | Pick two branches that edit the same lines, run, then pick them in the other order and run again | The clashing step is named with its files and tinted; which step clashes depends on the order (skipped: the harness loses window focus right after the trial-merge run, so the result frame is not captured) | | [ ] (skipped: same focus loss as 100.3) |
| 100.7 | Pick a branch that is already merged into the trunk | Its step reads "already in" (skipped: the harness loses window focus right after the trial-merge run, so the result frame is not captured) | | [ ] (飛ばし: 100.x はフォーカスを失う) |
| 100.8 | After a run, check `git status`, `git branch -v` and the worktree folders in a terminal | Nothing moved: no branch, index, HEAD or file changed (skipped: the harness loses window focus right after the trial-merge run, so the result frame is not captured) | | [ ] (skipped: same focus loss as 100.3) |
| 100.9 | Change the picks after a run; press Esc; try Ctrl+Shift+P over the list | The old result goes when the picks change; Esc closes the list; the palette does not open over it (skipped: the harness loses window focus right after the trial-merge run, so the result frame is not captured) | | [ ] (skipped: same focus loss as 100.3) |
| 100.10 | Switch EN and JA; use a narrow window | Both languages read well; long paths are cut with an ellipsis (skipped: the harness loses window focus right after the trial-merge run, so the result frame is not captured) | | [ ] (飛ばし: 100.x はフォーカスを失う) |

## 93. Landing a worktree (merge in, check, fast-forward, clean up)

| # | Do | Expect |
| --- | --- | --- |
| 93.1 | In the `3` view select a worktree that is ahead of main and press `g l` | A band asks "Land <branch> on main?" with four rows (take main in, run the check, move main forward, remove the worktree) and buttons `1` to `4`; `y` does nothing, Esc closes it (looked: now g l (not l); four rows 1-4 and Esc, with an unapproved-review warning, on a worktree ahead of main) |
| 93.2 | Press `g l` on the main worktree, on a detached one, on one with uncommitted changes, on one mid-rebase and on one with nothing ahead | Each answers with a toast saying why and no band opens |
| 93.3 | Press `g l`, then `4`, on a clean branch whose main is already in it | The toast reads "Landed <branch> on main: …"; main now points at the branch tip (`git log main`), the worktree folder and the branch are gone, a Claude tab of that worktree is closed (skipped: runs a real land that moves main and removes the worktree; covered by land.rs unit tests) |
| 93.4 | Let main move on (commit on it), press `g l`, then `4` | main is taken into the branch first (a merge commit), the check runs, then main moves forward to that merge; no `git push` happens (skipped: runs a real land that moves main and removes the worktree; covered by land.rs unit tests) |
| 93.5 | Press `g l`, then `1` | Only main is taken in; the toast says so ("Land it again to go on") and nothing else moved (skipped: runs a real land that moves main and removes the worktree; covered by land.rs unit tests) |
| 93.6 | Press `g l`, then `2`, in a repository with `scripts/verify.sh` that fails | The toast says "Stopped at the check: it failed (exit N)" with the last lines of its output; main did not move and the worktree is still there (skipped: runs a real land that moves main and removes the worktree; covered by land.rs unit tests) |
| 93.7 | Press `g l`, then `4`, when main clashes with the branch | The `4` view opens on the clash and the toast says "Stopped at taking main in"; after solving and committing there, `l` `4` goes on from there (skipped: runs a real land that moves main and removes the worktree; covered by land.rs unit tests) |
| 93.8 | Press `g l`, then `3`, then `l`, then `4` on the same branch | The second run skips what was done ("main was in … already") and only cleans up (skipped: runs a real land that moves main and removes the worktree; covered by land.rs unit tests) |
| 93.9 | Use a repository without a verify script | The toast says "no check found (skipped)" and the landing goes on |
| 93.10 | Select the window's own worktree and press `g l` | Only `1` to `3` are offered and a note says it cannot be removed from here |
| 93.11 | Land with a Claude working in the worktree | The band notes a Claude is working there and that its tabs close at the last stage (skipped: needs a real Claude or tsumugi session) |
| 93.12 | Switch to JA and press `g l` | The band, the buttons and every toast read in Japanese; `?` lists `g l` among the worktree keys (skipped: EN/JA switch not drivable here) |

## 91. A snapshot and diff per Claude turn (`y`)

| # | Do | Expect |
| --- | --- | --- |
| 91.1 | Let a Claude in a worktree edit a file and stop to wait (tsumugi shows it `waiting` or `done`) | Within a few seconds `git for-each-ref refs/michi/turn/` lists `refs/michi/turn/<branch>/1`; the index, HEAD and files are untouched; no toast appears | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 91.2 | In the `3` view select that worktree and press `y` | The compare review opens titled `Turn 1 of 1 on <branch>` listing what the Claude changed in that turn | | [ ] (skipped: needs a real Claude turn kept by tsumugi) |
| 91.3 | Give the Claude a second instruction that changes another file; after it stops press `y` | `Turn 2 of 2`: only the second instruction's change is listed | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 91.4 | In the turn review press `y`, then `Y` | `y` goes to the older turn and `Y` to the newer one; at the ends a toast says "That is the oldest turn kept" / "That is the newest turn" | | [ ] (skipped: needs two kept Claude turns from tsumugi) |
| 91.5 | Let a Claude stop without changing any file | No new turn ref is made | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 91.6 | Press `y` on a worktree whose Claude never stopped under michi | A toast says no turn is kept yet | | [ ] (skipped: needs a real Claude session) |
| 91.7 | Restart michi while a Claude is already waiting, then let it work and stop | Only the turn ended after michi looked is kept (the first list is a baseline) | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 91.8 | Switch to JA; press `?` in the worktree table and in the review | Titles and toasts read in Japanese; `y` and `y Y` are listed | | [~] harness 2026-10-22: JA `?` in the worktree table and in the review lists the branch-wide diff keys (j k, Tab, J K, n, g, Space, u, s, y Y, m, Enter) and the titles read in Japanese; a real Claude turn not tried |

## 78. Sync: fetch, pull and push in one (`S`)

| # | Do | Expect |
| --- | --- | --- |
| 78.1 | In the graph view look at the tool band | A `Sync S` button sits after `Push P`, before the line; its icon is a down and an up arrow; hover says `Sync (S)` | | [ ] (looked at (JA): the Sync S button sits after Push P with a down/up arrow icon, before the divider; hover not drivable) |
| 78.2 | Behind origin by 2 commits, ahead by none: press `S` | One job named `sync` runs (the title bar shows its progress, Esc stops it); the toast says `Synced with origin/<b>: pulled 2 commits` | | [ ] (skipped: needs a remote set up behind or ahead; not driven here) |
| 78.3 | Ahead by 1, not behind: press `S` | The commit is pushed at once (no band); the toast says `Synced with origin/<b>: pushed`; `git log origin/<b>` has it | | [ ] (skipped: needs a remote set up behind or ahead; not driven here) |
| 78.4 | Behind by 1 and ahead by 1 (diverged): press `S` | Nothing is merged or pushed; the toast says `Sync stopped at the pull, nothing was pushed: origin/<b> went its own way (↑1 ↓1) …` | | [ ] (skipped: needs a remote set up behind or ahead; not driven here) |
| 78.5 | Behind by 1, ahead by 1 on a pull that works (the other side already pulled), but the remote hook refuses the push | The toast says `Sync stopped at the push (pulled N commits first): … refused the push: …` and the pulled commits stay | | [ ] (skipped: needs a remote hook that refuses the push; not set up) |
| 78.6 | A new branch with no upstream: press `S` | It is pushed and gets the upstream; the toast says `Synced: pushed to origin/<b>, now its upstream` | | [ ] (seen on a lab repo: S on a branch with no upstream left it as feat + origin and the status line says it matches origin/feat; the toast was gone before the frame) |
| 78.7 | With two remotes and no `origin`: press `S` | The remote chooser opens; choosing one runs the sync to it | | [ ] (skipped: not driven (needs a two-remote lab repo and a chooser click)) |
| 78.8 | In sync already: press `S` | The toast says `Already in sync with origin/<b>` | | [ ] (undecidable: the toast is gone before the frame (3 s); only the in-sync status line is seen) |
| 78.9 | Press `S` while a Claude works in the worktree, or in the middle of a merge | The Claude case asks `Sync there anyway?` (y/n); the merge case says sync waits (and the button is greyed) | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 78.10 | Press `S` in the 2, 3 and 4 views; open `?`; switch EN and JA | `S` works in each; the key list has a `S` line; both languages read well | | [~] harness 2026-10-22: `S` ran a sync from the 2 and 3 views and from the graph (toast "Sync stopped at the pull, nothing was pushed…"); the `?` panel lists `S` in EN and JA; `4` did not open a 4 view in the lab (no such tab), so the 4 view is not seen |

## 67. Drag and drop onto a worktree

| # | Do | Expect |
| --- | --- | --- |
| 67.1 | In the graph (`1`) press on a commit row, drag it a little and watch | [ ] (undecidable: `<Drag>` from a commit row to a worktree row ended with no band or menu in the frame, and the grabbing hand and the following label cannot be seen from stills) |
| 67.2 | Drag that commit over a worktree row in the side column | [ ] (undecidable: the outline shows only while the pointer is held over the row; the harness gives the frame after the drop) |
| 67.3 | Let go over the worktree row | A small menu at the pointer reads `<hash> onto <branch>` and offers Merge into it and Cherry-pick onto it; nothing has run yet |
| 67.4 | Pick Merge into it; then do the same and pick Cherry-pick onto it | The same band as `m` and `C` (plan lines, the backup note, the Claude warning) asks first, for that worktree and not for the one the window is on; y runs it |
| 67.5 | Drag a branch row of the side column onto another worktree's row | The menu offers Merge into it and Rebase it onto the dropped branch; Rebase asks the band of the palette's Ctrl+R for that worktree (skipped: needs drag or wheel, which the harness lacks) |
| 67.6 | Drag a branch onto the worktree that has it checked out; drop a commit outside the side column | The first says nothing can be done and opens no menu; the second does nothing (skipped: needs drag or wheel, which the harness lacks) |
| 67.7 | Open the menu, press Esc; open it again and click outside it | Both close it without doing anything |
| 67.8 | Drag onto a worktree in the middle of a merge, and onto one where a Claude works; switch EN and JA | The band says the worktree is busy / warns about the Claude, as the keys do; both languages read well (skipped: needs drag or wheel, which the harness lacks) |

## 68. Syntax colours in diffs

| # | Do | Expect |
| --- | --- | --- |
| 68.1 | Open a commit that changes a `.rs` file (detail pane), then the Changes view of a worktree with a `.rs` edit | Keywords, strings, comments, numbers and names are coloured on context, added and deleted lines; the green/red line tint and the marked words (changed words) still show | | [ ] (seen on a.rs in a commit: strings and numbers coloured, changed words marked, green/red tint kept; keywords look plain) |
| 68.2 | Look at a `.toml`, `.md`, `.ts`, `.py` and `Dockerfile` change | Each gets colours; a file of an unknown type (`.xyz`) and a binary file stay plain | | [ ] (seen: .toml and Dockerfile get colours (FROM/RUN, strings, numbers); .md, .ts, .py and .xyz were below the fold and not judged) |
| 68.3 | Switch the theme in settings between a dark and a light one | The colours follow the theme and stay readable on the added and deleted tints | | [ ] (skipped: theme switch not drivable here) |
| 68.4 | Open the diff side by side and unified (narrow the window) | Both layouts show the same colours; wrapped long lines keep them | | [~] (2026-10-22 ran at 480x600 in lab nw9: unified (EN) and side by side (JA) both keep the red/green colours and the wrapped long lines stay coloured; stills only) |
| 68.5 | Open a big commit (hundreds of files, a huge generated file) | The window does not stall while it loads; a file of over 6000 diff lines, or a line of over 1500 characters, is left plain | | [ ] (skipped: needs a commit with hundreds of files; not built) |
| 68.6 | Select diff text with the mouse and copy it | The copied text is the plain text (no colour effects) | | [ ] (skipped: needs mouse text selection and the clipboard) |

## 92. Suggested order to land the worktrees

| # | Do | Expect |
| --- | --- | --- |
| 92.1 | With three worktrees ahead of main, two of which edit the same lines and one which touches other files: open the worktree table | The name cell of each shows a note `Land 1 of 3` / `2 of 3` / `3 of 3`; the one that conflicts with nothing is `1 of 3` | | [ ] (skipped: needs three worktrees with two clashing edits; not built) |
| 92.2 | Look at the last one of the two that conflict | Its note is yellow and reads `Land 3 of 3 · N places to resolve against those before`; the earlier ones are green with no places | | [ ] (skipped: needs three worktrees with two clashing edits; not built) |
| 92.3 | Only one worktree is ahead of main, or none | No `Land` note shows | | [ ] (skipped: not driven) |
| 92.4 | Commit in a worktree so the order would change; wait for the table to refresh | The numbers follow the new predictions without a restart | | [ ] (skipped: needs a Claude-driven order change) |
| 92.5 | Start a merge in one worktree | The notes go away while the merge is under way and come back after | | [ ] (skipped: needs a merge under way in a worktree) |
| 92.6 | Switch to JA | The notes read `main へ入れる順 1 / 3`, and the yellow one adds `先に入れた分との衝突 N 箇所` | | [ ] [~] harness 0.106.11 (lab m1, <Lang:ja>): wt2 reads 'main へ入れる順 2 / 2' in green; the yellow clash note was not on screen in this lab, so only the first half is seen |

## 90. Loading skeletons in the graph

| # | Do | Expect |
| --- | --- | --- |
| 90.1 | Open a big repository (tens of thousands of commits) in the graph view | Until the first rows come, thin grey bars with a dot (rows of the graph's height) pulse softly in place of an empty screen; no text yet | | [ ] (skipped: needs a huge repo with a slow first read; not timed by the harness) |
| 90.2 | Make the read slow (a huge repository or a network drive) and keep looking | After 2 seconds a line `Still reading the history… N s` joins the bars and the seconds count up | | [ ] (skipped: needs a read slower than 2 s; not drivable) |
| 90.3 | Wait for the first batch | The bars and the text go at once and the real rows take their place, with no flicker | | [ ] (skipped: flicker is not visible in still frames) |
| 90.4 | Open an empty repository (no commit) | No skeleton stays; the screen says `No commits yet.` | | [ ] (looked at: an empty repo shows No commits yet with no skeleton) |
| 90.5 | Switch the theme (dark and light), density (compact, cozy) and EN/JA while the bars show | The bars follow the theme and the row height; the text reads in the chosen language | | [ ] [~] harness 0.106.11 (lab m1): the worktree table in JA with the light theme reads well (text and chips follow the theme); the bars, density (compact, cozy) were not shown, so those parts stay open |
| 90.6 | Watch the CPU while the bars pulse | The window repaints about 15 times a second only while waiting and stops when the rows come | | [ ] (skipped: needs CPU reading of the pulsing bars) |

## 99. Handing a clash to a Claude and reviewing the result

| # | Do | Expect |
| --- | --- | --- |
| 99.1 | In a worktree with a Claude and a merge conflict, press `4` then `g` | A row appears under the bar: "Asked the Claude in ...; waiting for it to stop..."; no buttons yet; `a` and `r` say the Claude is still working | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 99.2 | Let the Claude solve everything (git add) and stop | The toast and notification say it stopped; the row turns green: "... stopped and solved everything", with "Look at the diff" `d`, "Adopt" `a`, "Send back" `r` | | [ ] (skipped: needs OS or machine features the harness lacks) |
| 99.3 | Press `d` (or click the button) | The Changes view opens on that worktree and shows what the Claude did | | [ ] (skipped: needs a real Claude-finished worktree) |
| 99.4 | Back in `4`, press `a` | The usual commit flow runs (message check, Claude-working check); the merge is committed and the row goes away with the conflict | | [ ] (skipped: needs a real Claude-finished worktree) |
| 99.5 | Hand over again, let the Claude leave one place marked, stop it | The row is red with "N places still have markers"; only "Look at the diff" and "Send back" show; `a` says places are left | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 99.6 | Press `r` | The line "Your solution was not accepted ..." is typed into the Claude; the row goes back to waiting; when it stops, the row updates again | | [ ] (skipped: needs a real Claude session waiting on a merge) |
| 99.7 | Press `r` with no Claude session open in the worktree | A new tsumugi tab with a Claude starts with the same line | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 99.8 | Abort the merge, or finish it by hand | The row goes away; `d`, `a`, `r` do nothing | | [ ] (skipped: needs a real Claude session waiting on a merge) |
| 99.9 | Switch EN and JA; narrow the window | The row reads well in both and wraps without hiding the buttons | | [ ] (skipped: needs a real Claude session; no EN/JA switch) |

## 80. How hard a question asks (three steps)

| # | Do | Expect |
| --- | --- | --- |
| 80.1 | Press `m` on a branch to merge, or rebase: look at the band | A plain band titled `Are you sure?` in the waiting colour; `y` goes on, Esc stops | | [ ] (skipped: the m confirmation band did not show in the run (frame lag)) |
| 80.2 | Delete a branch, drop a stash, remove a worktree or run the tidy (Enter on picked rows), and start a reset | The band is titled `This is hard to undo` and drawn in red; `y` (or the digits for a reset) and Esc work as before | | [ ] (skipped: the band did not show in the run) |
| 80.3 | Make the remote go ahead (push from another clone), then press `F` | The band is red, titled `This overwrites the remote`, lists the commits, says what is overwritten and `To go on, type origin/<b> and press Enter.`; there is no Yes button, only No | | [ ] (skipped: needs a remote ahead of the force push; not set up) |
| 80.4 | In that band press `y`, `n` and a digit | They appear after `>` in the band as typed letters; the band stays and nothing is pushed | | [ ] (skipped: depends on 80.3) |
| 80.5 | Press Enter with a wrong or half-typed name; then Backspace over it and type the whole `origin/<b>`, Enter | The first Enter only toasts `Type origin/<b> first, then Enter`; the full name pushes (with the lease) | | [ ] (skipped: depends on 80.3) |
| 80.6 | In the same band press Esc, or click No | The band closes and nothing is pushed | | [ ] (skipped: depends on 80.3) |
| 80.7 | Switch to JA and repeat 80.2 and 80.3 | Titles, the typing note and the toast read in Japanese | | [ ] (skipped: depends on 80.3) |

## 83. Keys that change meaning with the screen (keybar and `?` always match, #83)

| # | Do | Expect |
| --- | --- | --- |
| 83.1 | In `1` the keybar lists `t` as Revert; go to `3` then to `4` (a conflict) | The keybar's `t` reads Open in the terminal in `3`, Theirs in `4`; each does exactly what the keybar says | | [ ] (skipped: the 4 view needs a conflict; the toast says there is none in the lab) |
| 83.2 | In `3` press `?` | Both `3` and `1` list Fetch, Push and `p` Pull (the pull request is `O` in `3`) | | [ ] (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 83.3 | In `2` press `?` | Everywhere does not list `N` for new work (here `n N` jumps between changes); `N` still opens New work in `1` and `3` | | [ ] (skipped: the help frame lagged and was not read) |
| 83.4 | Press `L` (reflog list), then look at the keybar and press `?` | The keybar shows only `j k` Move, Enter Bring the row back, Esc Back; `?` lists those under "Reflog" and no "Everywhere" keys | | [ ] (looked at: the keybar in the reflog list shows j k, Enter, g, r, Esc only) |
| 83.5 | Repeat 83.4 with `c` in `3` (clean-up), `T` in `3` (trial merge) and `r` in `3` (rebase) | Each screen's own keys are in the keybar (Space picks, `a` picks all, `u` clears, `s f d p r`...), and typing `f` or `p` there does nothing to git | | [ ] (skipped: T keybar looked right (j k, Space, u, Enter, Esc) but c and r screens did not open in the run) |
| 83.6 | Start something that asks for a yes (`x` on a worktree, `F` force push) | The keybar reads `y` Yes, `n` No, Esc Back; closing the question brings the view's keys back | | [ ] (skipped: the yes band did not show in the run) |
| 83.7 | Switch EN and JA and repeat 83.4 and 83.6 | The new words read well in both languages and fit the keybar (the rest is cut, not overlapped) | | [ ] [~] harness 0.106.11 (lab m1, EN and JA): the reflog keybar reads 'j k Move · Enter Bring the row as a new branch · g Show the commit in the graph' / 'j k 動かす · Enter その行を新しいブランチとして戻す · g コミットをグラフで見る'; both fit with nothing overlapped (83.4 / 83.6 only the L keybar was repeated) |

## 84. Author avatars

| # | Do | Expect |
| --- | --- | --- |
| 84.1 | In the graph view (a repository with GitHub commits), look at the author column | Each author has a 16 px round avatar before their name: the GitHub picture for an author whose commit e-mail is a `...@users.noreply.github.com` address (it appears a moment after the rows), else a circle with their initials on a steady colour | | [ ] (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 84.2 | Look at a commit by `github-actions[bot]` or `dependabot[bot]` | The bot shows its GitHub picture and the short `⚙ name` text | | [ ] (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 84.3 | Drag the author column narrower than about 100 px | The avatar goes and the name keeps the whole column; widen it and the avatar comes back | | [~] (`<Drag>` of the Author header's edge to the right made the column about 85 px: the avatars went and the names kept the column, the Subject got wider; widening it again not tried) |
| 84.4 | Open Settings, General, and untick `Fetch pictures from GitHub` | Every picture turns into initials at once, `config.toml` gets `[view]` `avatars = false`, and no `curl` process starts when scrolling (check the task manager) | | [ ] (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 84.5 | Tick it again; or write `avatars = true` in config.toml by hand | Pictures come back without a restart; a non-boolean value shows a warning and counts as on | | [ ] (飛ばし: settings の操作と手編集後の再読込が要る (typing 不可)) |
| 84.6 | Disconnect the network, restart michi and scroll the graph | The window stays responsive (no stall on the UI thread); initials show and the pictures are not retried in a loop | | [ ] (飛ばし: ネットワーク断が要る) |
| 84.7 | Switch the theme dark/light and EN/JA in Settings | The initials stay readable on their colours; the Settings text reads in both languages | | [ ] (飛ばし: テーマと EN/JA の切り替えがハーネスに無い) |
| 84.8 | Scroll a long history fast | Scrolling stays smooth while pictures arrive; the same author's picture is not fetched twice | | [ ] (飛ばし: 速いスクロール (wheel) がハーネスに無い) |

## 81. Empty-state guidance (one next step, no tour)

| # | Do | Expect |
| --- | --- | --- |
| 81.1 | Open a repository with no commit (`git init` only) in the graph view | Under `No commits yet.` one brighter line reads `Next: press 2 to stage your files and make the first commit.`; pressing `2` opens Changes | [ ] (looked at (JA): under the empty-history text the line says press 2 to stage and make the first commit; 2 opens Changes) |
| 81.2 | Make the first commit | The line goes at once and the graph shows the commit | [ ] (skipped: would need a commit in the lab; same path as 81.1 not driven) |
| 81.3 | Open a repository with commits and one worktree, go to the worktree view (`3`) | Under the table a line reads `Next: press N to start new work in a worktree of its own.`; `N` opens New work | [ ] (looked at (JA): the Next line shows under the table; pressing N not driven) |
| 81.4 | Add a second worktree (and run tsumugi) | The line is gone | [ ] (skipped: needs a real Claude or tsumugi session) |
| 81.5 | With two or more worktrees and tsumugi not running, look at the worktree view | The line reads `Next: start tsumugi to see the Claude Code running in each worktree.`; start tsumugi and it goes within a few seconds | [ ] (skipped: needs a real Claude or tsumugi session) |
| 81.6 | While the history is still being read (a big repository) | No guidance line shows (the skeleton only) | [ ] (skipped: needs a slow history read the harness cannot hold) |
| 81.7 | Switch EN and JA; narrow the window | The line reads well in both and wraps without overlapping the table | [~] (looked at 2026-10-22 with `<Lang:en>`/`<Lang:ja>` and `<Size:480x600>`: the empty-history line reads `Next: press 2 to stage your files and make the first commit.` / `次は 2 を押して、…` in both and stays inside the box without overlapping; the top note is cut with `…` at that width) |

## 70. Auto fetch (every 5 minutes, quiet)

| # | Do | Expect |
| --- | --- | --- |
| 70.1 | Open a repository with a remote, leave the window in front for 5+ minutes without pressing `f` | The keybar's `fetch N min` goes back to `fetch now` by itself (a fetch ran), with no toast and no notification, even when nothing moved | | [ ] (skipped: needs OS or machine features the harness lacks) |
| 70.2 | Put the window behind another and wait over 5 minutes, then bring it forward | No fetch runs while behind; within a moment of coming forward the fetch runs and the time resets | | [ ] (飛ばし: 5 分以上の待ちと窓の前後入れ替えが要る) |
| 70.3 | Write `[net]` `auto_fetch = 1` in config.toml (no restart), stay in front | A fetch runs about every minute; `auto_fetch = 0` stops them | | [ ] (飛ばし: 1 分周期の自動 fetch を見るにはリモートと待ちが要る) |
| 70.4 | Write `auto_fetch = "soon"` | A warning toast about config.toml; the 5-minute default stays | | [ ] [~] harness config.toml に [net] auto_fetch = "soon" を書いて開くと、警告の toast「keymap.toml / config.toml: 警告 1 件: config.toml: [net] auto_fetch is not a number of minutes (0 turns it off)」が出る。5 分の既定が残る所は未確認。注: <Lang:en> なのに「警告 1 件」が日本語で出た (見出しが未翻訳の疑い) |
| 70.5 | Cut the network (or point `origin` at a bad address) and wait for a fetch | No toast and no system notification; the keybar's right reads `fetch N min · failed` (or `fetch failed`) in the waiting colour; the gaps between tries grow (up to 8 times the interval) | | [ ] (skipped: needs OS or machine features the harness lacks) |
| 70.6 | Restore the network and press `f` (or wait) | The failed mark goes as soon as a fetch works | | [ ] (飛ばし: ネットワーク断が要る) |
| 70.7 | With the network cut, press `f` yourself | The usual `fetch failed: ...` toast and notification still come, and the keybar shows `failed` | | [ ] [~] harness 0.106.11 (lab ff1, origin = a missing path): pressing f shows the toast "fetch failed: ... does not appear to be a git repository" and the keybar reads "fetch just now · failed"; the system notification is not judged (OS feature) |
| 70.8 | Start a pull or push while the timer is due | The timer waits; it never interrupts a job | | [ ] (飛ばし: pull/push 中にタイマーが来る瞬間を作れない) |
| 70.9 | Switch EN and JA | The keybar's `· failed` / `· 失敗` reads well and fits | | [ ] [~] harness 0.106.11 (lab ff1, 1100x600): EN reads `fetch just now · failed`, JA reads `fetch たった今 · 失敗`; both fit the keybar |

## 71. Git LFS pointers (#30)

| # | Do | Expect |
| --- | --- | --- |
| 71.1 | In a repository with LFS files, select a commit that changed one | Above its diff (the three pointer lines) a note: `A Git LFS pointer: content not fetched (git lfs pull)`; other files have no note | | [ ] [~] harness 0.106.11 (lab lfs1: a committed pointer file, no git-lfs): the selected commit's detail shows the note above the 3 pointer lines. The row now has the real wording (#156); the Changes-view and review cases are not run |
| 71.2 | Change an LFS file in a worktree and open it in the Changes view, and in a review | The same note shows above the pointer diff | | [ ] (飛ばし: git-lfs の実物が要る) |
| 71.3 | Switch EN and JA | The note reads well in both and fits the width | | [ ] [~] harness 0.106.11 (lab lfs1, 1100x600): EN "A Git LFS pointer: content not fetched (git lfs pull)" and JA "Git LFS のポインタ: 中身は未取得 (git lfs pull)" read well and fit the width |

## 72. Signature mark in the detail (#35; numbered 71 because `## 35.` is taken)

| # | Do | Expect |
| --- | --- | --- |
| 72.1 | Select a commit that is signed and whose key git trusts (`git log --show-signature` says Good) | The detail's meta line shows a green `✓ Signed` after the date, before the parents | | [ ] (飛ばし: gpg の署名鍵が要る) |
| 72.2 | Select an unsigned commit | No mark at all; the meta line is as before | | [ ] (飛ばし: 署名の有無で比べる行で、署名鍵が要る) |
| 72.3 | Select a commit signed with a key git cannot find (or with gpg missing from PATH) | A `? Signature not checked` mark in the waiting colour; no toast, no error | | [ ] (飛ばし: gpg の署名鍵が要る) |
| 72.4 | Select a commit whose signature is bad (a commit object edited after signing) or whose key expired | A red `✗ Bad signature` mark | | [ ] (飛ばし: gpg の署名鍵が要る) |
| 72.5 | Click through many commits quickly in a repository where most are signed | The detail still follows the selection; the mark may arrive with the diff but never blocks the UI | | [ ] (飛ばし: gpg の署名鍵が要る) |
| 72.6 | Switch EN and JA | `✓ 署名あり` / `✗ 署名が不正` / `? 署名は未確認` read well and wrap with the meta line | | [ ] (飛ばし: EN/JA の切り替えがハーネスに無い) |

## 31. Sparse checkout notice (#31; the mcp section above uses the same number from #101)

| # | Do | Expect |
| --- | --- | --- |
| 31.5 | In a repository run `git sparse-checkout set --cone <one folder>`, open it in michi | A strip under the title bar says `Sparse checkout: N tracked files are left out of this folder...` with N the files not on disk; there are no buttons | | [ ] (見た: sparse checkout の帯 (追跡しているファイル 1 件がこのフォルダにありません) が 1 画面の上に出た) |
| 31.6 | Run `git sparse-checkout disable` while michi is open | The strip goes away on the next refresh (no restart) | | [ ] (飛ばし: michi を開いたまま sparse-checkout を変えられない) |
| 31.7 | Run `git sparse-checkout set --no-cone '/*' '!/some-folder'` | The strip says `Sparse checkout (patterns): ...` | | [ ] (飛ばし: michi を開いたまま sparse-checkout を変えられない) |
| 31.8 | Sparse checkout on and a detached HEAD together | One strip: the detached HEAD's words first, then the sparse sentence; the `R` / `B` buttons stay | | [ ] (飛ばし: michi を開いたまま sparse-checkout を変えられない) |
| 31.9 | Switch EN and JA; narrow the window | The sentence reads well in both and is cut with an ellipsis rather than overflowing | | [~] (2026-10-22 ran at 480x600 EN and JA: the sentence over the views is cut with an ellipsis, not overflowing; the 3 view's sentence not tried. Seen: at this width the top bar's search box covers the minimize/maximize buttons and the bell overlaps maximize) |

## 38. Content search in the `/` filter (`s:` pickaxe, `g:` regex)

| # | Do | Expect |
| --- | --- | --- |
| 38.1 | Press `/`, type `s:` followed by a string you know a commit added (e.g. a function name) | The right of the field reads `Enter: search the changes`; no row is a hit yet | | [ ]  (seen: s:f2 shows the hint Enter: search the changes with every row faint before Enter) |
| 38.2 | Press Enter | After a moment the count reads `N hits`; the commits that added or removed the string are bright, the rest faint; marks appear on the scrollbar | | [ ]  (seen: after Enter the count reads 1 hit, only first msg is bright; scrollbar marks not visible in a 4-row graph) |
| 38.3 | Press Enter again, and Shift+Enter | The selection moves to the next and previous hit (it does not search again) | | [ ]  (undecidable: hist1 has a single hit so the selection cannot be seen to move) |
| 38.4 | Type `g:` and a regex (e.g. `g:fn +main`), press Enter | Commits whose diff has a changed line matching it (any case) are the hits | | [ ] (判断不能: ハーネスで g: の後ろに空白を打てず (+ は Shift になる) 正規表現を作れない) |
| 38.5 | Type a word before it: `fix s:foo` | Only commits that match both the word and the content search stay bright | | [ ]  (undecidable: the combined word plus s: search was not distinguishable in the frames) |
| 38.6 | Edit the text after `s:` | The hits go and `Enter: search the changes` comes back; the old answer is never shown for the new text | | [ ]  (undecidable: the frames lag and the hint after editing was not caught) |
| 38.7 | Type `g:(` and press Enter | A toast says the search failed, with the reason; the window stays usable | | [ ]  (undecidable: no toast was caught in the frames for g:() |
| 38.8 | Press Esc | The filter clears and every row is bright again | | [ ]  (seen: after clearing the filter every row is bright again and the field shows its placeholder) |
| 38.9 | In a large repository press Enter on a search | The window stays responsive while the search runs; the hits come in when it is done | | [ ]  (skipped: needs a large repository with a slow search; big1 not set up for it) |
| 38.10 | Switch EN and JA | The hint in the empty field and the `Enter: ...` line read well and fit | | [~] (2026-10-22 ran EN and JA in lab m1: with s:f the field reads `Enter: search the changes` / `Enter: 変更の中身を探す` and fits; the empty-field hint is cut with an ellipsis in EN (`Filter (subject, hash, n…`); JA empty hint not looked at) |

## 129. Submodules: state and update (#29)

(`## 29.` was taken by the confirmation window, so this section is 129.)

| # | Do | Expect |
| --- | --- | --- |
| 129.1 | In the worktree table of a repository with no submodule press `u` | A toast says the worktree has no submodules | [ ] (判断不能: u の toast が写らなかった) |
| 129.2 | In a repository whose submodules are all on the recorded commit press `u` | A toast says all N submodules are on their recorded commits; no question | [ ] (判断不能: 最新のビルドで submodule 付きの 3 つの lab (所定の位置・古い・未取得) の worktree 表で u を押しても、toast も確認の帯も出ない。ハーネスの鍵の問題か u の不具合か切り分けが要る) |
| 129.3 | `cd` into a submodule and `git checkout HEAD~1`, then press `u` on its worktree | The band lists the submodule as off its commit and asks about `submodule update --init --recursive`; `n`/Esc leaves it as it is | [ ] (判断不能: 最新のビルドで submodule 付きの 3 つの lab (所定の位置・古い・未取得) の worktree 表で u を押しても、toast も確認の帯も出ない。ハーネスの鍵の問題か u の不具合か切り分けが要る) |
| 129.4 | Answer yes to 129.3 | A toast says the submodules were put on their recorded commits; `git submodule status` shows no `+` | [ ] (判断不能: 最新のビルドで submodule 付きの 3 つの lab (所定の位置・古い・未取得) の worktree 表で u を押しても、toast も確認の帯も出ない。ハーネスの鍵の問題か u の不具合か切り分けが要る) |
| 129.5 | Clone a repository with submodules without `--recurse-submodules`, press `u`, then yes | The missing (`-`) submodules are listed, then cloned and checked out | [ ] (判断不能: 最新のビルドで submodule 付きの 3 つの lab (所定の位置・古い・未取得) の worktree 表で u を押しても、toast も確認の帯も出ない。ハーネスの鍵の問題か u の不具合か切り分けが要る) |
| 129.6 | Leave an uncommitted change in a submodule file the update would overwrite, then `u` and yes | git refuses, michi shows `submodule update failed: ...` and the change is still there | [ ] (判断不能: 最新のビルドで submodule 付きの 3 つの lab (所定の位置・古い・未取得) の worktree 表で u を押しても、toast も確認の帯も出ない。ハーネスの鍵の問題か u の不具合か切り分けが要る) |
| 129.7 | Press `u` on a worktree in the middle of a merge, with a stale submodule, then yes | The update is refused with a message naming the merge | [ ] (判断不能: 最新のビルドで submodule 付きの 3 つの lab (所定の位置・古い・未取得) の worktree 表で u を押しても、toast も確認の帯も出ない。ハーネスの鍵の問題か u の不具合か切り分けが要る) |
| 129.8 | Open `?` in the worktree table; switch EN and JA | `u` reads "Submodules: state and update" / "submodule の状態と更新"; the question and toasts read well and fit | [ ] (判断不能: 最新のビルドで submodule 付きの 3 つの lab (所定の位置・古い・未取得) の worktree 表で u を押しても、toast も確認の帯も出ない。ハーネスの鍵の問題か u の不具合か切り分けが要る) |
| 129.9 | Open the command palette (`Ctrl+Shift+P`) in the worktree table | A "Submodules: state and update" entry with `u` is listed and works | [ ] (判断不能: 最新のビルドで submodule 付きの 3 つの lab (所定の位置・古い・未取得) の worktree 表で u を押しても、toast も確認の帯も出ない。ハーネスの鍵の問題か u の不具合か切り分けが要る) |

## 280. Remotes: list, add, remove (`M`)

| # | Do | Expect |
| --- | --- | --- |
| 280.1 | In the graph press `M` in a repository with a remote | A list titled `Remotes` opens: each remote's name with its address; `M Remotes` is in the graph's keys (`?`) | [ ] (looked at on a real window: M opens the Remotes list) |
| 280.2 | Type `mine https://example.com/x.git` | One row `mine`, lit, reads `add it at https://example.com/x.git`; Enter adds it, a toast says nothing was fetched; `M` lists it | [ ] (looked at on a real window: name and URL typed, the row offers to add it) |
| 280.3 | Type only `mine` and press Enter | A toast asks for the address after the name; the list stays open | [ ] (looked at on a real window: name only, a toast asks for the address) |
| 280.4 | Type `a/b https://x` and press Enter | A toast says `'a/b' is not a valid remote name`; the list stays open and nothing is added | [ ] (looked at on a real window: invalid name toast shows; it is git English text even in the JA UI) |
| 280.5 | Select a remote and press Delete | The red question names the remote and its address and says its remote-tracking branches go; `y` or Ctrl+Enter removes it, the graph drops its `origin/...` pills, the toast says where it was | [ ] (looked at on a real window: Delete asks, y removes) |
| 280.6 | Press `Esc` on that question | Nothing is removed | [ ] (looked at on a real window: Esc removes nothing) |
| 280.7 | Press `M` in a repository with no remote | The list opens empty; typing a name and address still adds one | [ ] (looked at on a real window in one1: the list opens empty with a hint to type a name and URL) |
| 280.8 | Switch EN and JA | The hint, the keys line, the question and the toasts read well and fit | [~] (2026-10-22 ran EN and JA in lab m1: the keys line `Enter adds the typed one · Del removes the selected one · Esc closes` / its JA and the row hint `add it at https:` / `名前の後ろに URL を打つ` fit; the JA toast for a name alone fits; the delete question not tried. Seen: an already-raised toast keeps its language after switching to EN) |

## 52. Restore a file's version from a commit

Set up: `a.txt` has two versions (commit `first` has `one`, the head has `two`) and a later commit adds `later.txt`. The window's own worktree gets the file. michi keeps the file as it is now under `refs/michi/backup/<time>` (unless the head has that very version), then runs `git restore --source=<commit> --worktree`; the index and HEAD are not touched.

| # | Do | Expect |
| --- | --- | --- |
| 52.1 | Select `first` in the graph and look at the title bar of `a.txt` in the detail | A dim `Restore this version` at the right of the title (left of the wide toggle), underlined with a hand cursor on hover; not shown on the uncommitted row | | [ ] (looked at on a real window: the link reads この版に戻す with the g w key, not a dim underlined link) |
| 52.2 | Click `Restore this version`, then press Esc | A band asks whether to bring `a.txt` back as `<hash>` has it and says the current file is kept in refs/michi/backup; Esc leaves the file as it was | | [ ] (undecidable: the click on the restore link showed no band in the screenshots) |
| 52.3 | With `a.txt` as the head has it, click it and press `y` | A 10-second toast says it is as `<hash>` has it now (unstaged); the file reads `one`, Changes lists it as unstaged, `git diff --cached` is empty, no new backup ref | | [ ] (undecidable: restore did not run from the harness, see 52.2) |
| 52.4 | Change `a.txt` to `mine`, restore `first`'s version, press `y` | The toast names the backup; `git show refs/michi/backup/<time>:a.txt` gives `mine` and the file reads `one` | | [ ] (undecidable: restore did not run from the harness, see 52.2) |
| 52.5 | Restore the same version again | The toast says it is as `<hash>` has it already; nothing changes | | [ ] (undecidable: restore did not run from the harness, see 52.2) |
| 52.6 | Stage a change to `a.txt`, then restore an older version | The worktree file is the old version; `git diff --cached` still shows the staged change | | [ ] (undecidable: restore did not run from the harness, see 52.2) |
| 52.7 | Select a commit that deletes a file and click its `Restore this version` | A toast says there is no version to bring back; no band | | [ ] (undecidable: restore did not run from the harness, see 52.2) |
| 52.8 | With a Claude working in the worktree, click it; then with the worktree mid-merge | The band also notes that a Claude is working there; mid-merge a toast says to finish the merge first and no band appears | | [ ] (skipped: needs a real Claude or tsumugi session) |
| 52.9 | Switch EN and JA; narrow the window; switch to the light theme | The link, band and toasts read well in both languages; the file name is cut with `…` before it collides with the link; readable in both themes | | [ ] (skipped: needs EN/JA, a narrow window and the light theme) |

## 54. Co-Authored-By on michi's commits

| # | Do | Should | Notes | OK |
|---|----|--------|-------|----|
| 54.1 | With no `[commit]` in config.toml, commit from the `2` view with Ctrl+Enter, then look at the last commit's message | The message is exactly what you typed; no Co-Authored-By | | [ ] (undecidable: Ctrl+Enter and a click on the commit label did not commit from the harness (message typed, no commit made)) |
| 54.2 | Write `[commit]` `co_author = "Claude <noreply@anthropic.com>"` in config.toml (no restart), commit again | The new commit ends with a blank line and `Co-Authored-By: Claude <noreply@anthropic.com>` | | [ ] (undecidable: needs the commit from 54.1) |
| 54.3 | Amend that commit (`A`, Ctrl+Enter) | The trailer is not doubled | | [ ] (undecidable: needs the commit from 54.1) |
| 54.4 | Write `co_author = "Claude"` (no address) | A warning toast about config.toml; no trailer is added | | [ ] (undecidable: needs the commit from 54.1) |

## 101. Pull request pages on GitLab, Bitbucket and Azure DevOps (#33)

| # | Do | Expect |
| --- | --- | --- |
| 101.1 | On a worktree whose remote is a gitlab.com one (group/sub/proj) and branch `feat/x`, press `p` | The browser opens GitLab's new merge request page with the source branch `feat/x` already chosen | | [ ] (飛ばし: GitLab のリモートとブラウザの起動が要る (本物の GitLab に触らない)) |
| 101.2 | Same with a Bitbucket remote (`https://bitbucket.org/ann/repo.git`) | Bitbucket's new pull request page opens with `feat/x` as the source | | [ ] (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 101.3 | Same with an Azure DevOps remote (`https://dev.azure.com/org/proj/_git/repo`, and the ssh form on `ssh.dev.azure.com` with `v3/org/proj/repo`) | Azure's create pull request page opens with `feat/x` as the source | | [ ] (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 101.4 | Same with a GitHub remote | The GitHub compare page opens as before | | [ ] (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 101.5 | Same with a self-hosted or unknown host (`https://git.example.com/a/b.git`) | An error toast says the remote is not on GitHub, GitLab, Bitbucket or Azure DevOps; nothing opens | | [ ] (skipped: needs gh / GitHub / minato, which tests must not touch) |

## 37. Blame and per-file history (`B`, `H`)

| # | Do | Expect |
| --- | --- | --- |
| 37.1 | In the graph, select a commit and press `B` | A screen opens over the graph: `Blame: <first file of the commit>`, `As of <id>`, one row per line with the commit, who, how long ago, the line number and the line; it reads at once and the window stays responsive on a long file | [ ] (見た: B で「blame: f / 859bc84 時点。各行を最後に変えたコミット」の画面が開く (1 行のファイル)) |
| 37.2 | Click a file in the detail (so it is the one scrolled to), select another commit that has it, press `B` | The screen is of the file the detail was on | [ ] (飛ばし: 詳細で選ぶファイルを変えて比べる手順が、1 ファイルのリポジトリでは作れない) |
| 37.3 | Press `j` `k`, PageDown, PageUp, Home, End | The selection moves and stops at the ends; the screen follows it (22 rows) and shows `n / total` | [ ] (飛ばし: 行が 1 つしか無く、端で止まるかが見えない) |
| 37.4 | Press `Tab` (or `H`, then `B`) | The list switches between blame and the file's history (commit, who, ago, subject, newest first); each tab keeps its own selection (skipped: undecidable in the harness, the screen is the same before and after Tab) | [ ] (判断不能: Tab の前後で画面が同じだった (blame のまま)) |
| 37.5 | Open the history of a file that was renamed in the past | The commits from before the rename are listed too; the blame rows of old lines name the commit that wrote them | [ ] (飛ばし: rename 履歴のあるファイルの用意が要る) |
| 37.6 | Press Enter on a row | The screen closes, the graph is in front and that row's commit is selected (reading on to it when it is far down) | [ ] (判断不能: Enter の前後で画面が同じだった (blame の画面が残った)) |
| 37.7 | Click a row once, then again | The first click selects, the second goes to the commit; a click outside closes the screen; Esc closes it | [ ] (partly seen 2026-10-22: a click on a blame row closes the screen and selects that commit in the graph; one-click-selects and click-outside were not told apart)
| 37.8 | In the `2` view, change a file on disk (add a line, do not commit), select it and press `B` | The new line reads `-------` / `not committed` in the waiting colour; Enter on it says `This line is not committed yet` and stays | [ ] (undecidable: B in the 2 view opened nothing from the harness (focus on the list)) |
| 37.9 | Press `B` on a file deleted by the commit, then with nothing selected | A toast says there is nothing to blame (`... was deleted here`) or that no file is selected; no screen opens | [ ] (skipped: needs a commit that deletes a file, not in the lab) |
| 37.10 | Select a worktree's uncommitted row in the graph and press `B` | The file is blamed as that worktree has it | [ ] [~] harness 0.106.11 (lab m1): Uncommitted row, Tab Tab, B: 'Blame: dirty.txt · As the working tree has it', the one line reads 'not committed'. With no file selected B toasts 'No file is selected to blame' |
| 37.11 | Check the keybar and `?` over the screen; switch EN and JA | The keybar names j k, B H Tab, Enter, PageDown PageUp, Esc; global keys are not listed; text reads well in both languages and long lines end in `…` | [~] harness 2026-10-22: over blame the keybar names j k, B H Tab, e and no global keys; the EN `?` panel lists j k, B H Tab, e, Enter, PageDown PageUp, Esc; the JA keybar and EN text read well and long lines end in … |
| 37.12 | With Claude writing in the worktree, press `B` | Nothing changes in the worktree and no `index.lock` appears (a read only) | [ ] (skipped: needs a real Claude or tsumugi session) |

## 253. Per-worktree author and email (#53)

| # | Do | Expect |
| --- | --- | --- |
| 253.1 | With every worktree committing as the same `user.name` / `user.email`, look at the worktree table | No "Commits as" line under any name | [ ] (見た: 同じ作者の 3 worktree で「Commits as」の行はどこにも出ない) |
| 253.2 | In one worktree run `git config extensions.worktreeConfig true` then `git config --worktree user.name Other` and `user.email other@example.com`; wait for the next reread | Every worktree row gets a `Commits as Name <email>` line (JA `作者: …`) under its folder; the changed one shows `Other <other@example.com>` | [ ] (飛ばし: 作者の違う worktree の設定を作り直す手順で、読み直しの待ちも要る) |
| 253.3 | Set the name back in that worktree (`git config --worktree --unset user.name`) so it matches the others again | The lines disappear again without restarting michi | [ ] (飛ばし: 253.2 が前提) |
| 253.4 | Switch EN and JA | The line reads well in both; long values do not overflow the card | [~] (2026-10-22 ran EN and JA in lab m1 with a worktree-config author in wt3: `Commits as …` / `作者: …` read well, and the long `Other Person <other@example.com>` wraps inside the folder column and does not overflow the card) |

## 239. Clone from the repository screen (#39)

| # | Do | Expect |
| --- | --- | --- |
| 239.1 | Press `Ctrl+O`, paste `https://github.com/<owner>/<small repo>.git` | The first row reads `Clone <address> into <folder beside the open repository>/<repo>`, in the accent colour; the hint and footer read well in EN and JA | [ ] (飛ばし: 実際の GitHub のアドレスの clone で、テストは触れない) |
| 239.2 | Press Enter on that row | A toast `Cloning <address> …` shows and the window stays responsive; when git is done a toast `Cloned <folder>` shows and the window switches to the new repository | [ ] (飛ばし: 実際の clone でネットワークが要る) |
| 239.3 | Type an address, a space and a folder (`git@github.com:<owner>/<repo>.git ~/tmp/x`), Enter | It clones into that folder (`~` is the home folder) and switches to it | [ ] (飛ばし: 空白入りの入力をハーネスで打てない) |
| 239.4 | Clone the same address again into the same folder | A toast `Clone failed: destination path ... already exists and is not an empty directory`; the window stays on the repository it was on | [ ] (飛ばし: 実際の clone でネットワークが要る) |
| 239.5 | Clone an address that does not exist, or with the network off | A toast `Clone failed: ...` with git's first line; no half-made folder is left | [ ] (飛ばし: 実際の clone でネットワークが要る) |
| 239.6 | Start a clone of a large repository and press Enter on another clone row | A toast `A clone is still running`; nothing starts twice | [ ] (飛ばし: 大きな clone を走らせる必要がある) |
| 239.7 | Type a path or a word (not an address) | No clone row appears; the list and the typed-path row work as before | [ ] (見た: Ctrl+O に hello を打つと「合うリポジトリがありません」だけで clone の行は出ない) |

## 259. Speed on a very large repository (#59)

The made-up 300,000-commit history is timed by CI (`bench-large`); these rows are the real repository and the real window.

| # | Do | Expect |
| --- | --- | --- |
| 259.1 | Open a repository with 100,000+ commits that has a commit-graph (e.g. a Linux kernel clone after `git gc`) with `michi <folder>` | The first graph screen is drawn within 1 second of the window appearing; the window never stops answering while the rest of the history is read (skipped: needs OS or machine features the harness lacks) |
| 259.2 | Hold the Down key or drag the scrollbar from the top to the bottom and back | The scrolling follows the pointer with no stutter; the graph lanes and the dots stay in place as more rows are read (skipped: needs drag or wheel, which the harness lacks) |
| 259.3 | Open the same repository after deleting `.git/objects/info/commit-graph*` | A bar offers to write the commit-graph (it is the slow case); after it is written the first screen is within 1 second again |
| 259.4 | Press `/` and type a word while the history is still being read, then clear it | The filter answers as you type; the window does not freeze |
| 259.5 | Run `cargo run --release -p michi --example bench-large` | Three lines `ok` (first screen, all rows, all layout) and exit code 0; over the budget in `scale.rs` prints `OVER` and exits 1 |

## 255. Moving uncommitted changes between worktrees (`v`)

| # | Do | Expect |
| --- | --- | --- |
| 255.1 | In the `3` view with two worktrees, edit a tracked file and add an untracked file in one, select its row, press `v` | A palette opens: `Move the uncommitted changes to`, `from <worktree>`, one row per other worktree with its branch | [ ] (seen in c1: v on the c1-c row opens the palette "move uncommitted changes to / from c1-c" with one row per other worktree and its branch) |
| 255.2 | Pick a worktree with Enter | A question names both worktrees and says the changes are kept under refs/michi/backup/; `n` or Esc leaves everything as it was | [ ] (seen: Enter asks whether to move c1-c changes to c1, says c1-c becomes clean and the changes are kept under refs/michi/backup/; Esc closes) |
| 255.3 | Answer `y` | The source is clean, the target has the edit and the untracked file, a toast names the backup; refs/michi/backup/ has the changes; the stash list is empty | [ ] (skipped: would move real changes between lab worktrees; the move itself is unit-tested) |
| 255.4 | Edit the same file differently in both worktrees and move | The move is refused with the reason, and the changes are back in the source as they were | [ ] (skipped: needs a second divergent edit and a move; not driven this run) |
| 255.5 | Make the target change that file in a commit so the changes clash, move | The target has conflict marks, a toast says where, and the stash stays in the list | [ ] (skipped: needs a commit clash setup; not driven this run) |
| 255.6 | Press `v` on a worktree with no changes, then in a repository with one worktree | A toast says there is nothing to move / no other worktree; no palette opens | [ ] fixed in 0.106.12 (#150): v on a worktree with no changes toasts 'has no changes to put away' (harness shot); one worktree still toasts 'no other worktree' |
| 255.7 | Start a merge in either worktree and move | A toast says it is in the middle of a merge; the palette stays | [ ] (skipped: needs a stopped merge in a worktree) |
| 255.8 | With a Claude working in either worktree, move | The question also says a Claude is working there | [ ] (skipped: needs a real Claude or tsumugi session) |
| 255.9 | Switch EN and JA; check the keybar and `?` in the `3` view | `v` Move changes appears; text reads well in both languages | [~] (2026-10-22 ran EN/JA with F1 in lab m1: the 3 view's key list shows `v` Move changes / 変更を移す in both languages; the keybar not judged) |

## 245. Repository tabs (#45)

| # | Do | Expect |
| --- | --- | --- |
| 245.1 | Open one repository, press Ctrl+O (or Ctrl+T), pick another repository | It opens in a new tab right of the front one and the bar appears under the title bar; the first repository is still there in its tab with its view, selection and scroll as left |
| 245.2 | Open the same repository again from the screen | No second tab; its tab comes to the front |
| 245.3 | Press Ctrl+Tab and Ctrl+Shift+Tab (also Ctrl+PageDown / Ctrl+PageUp) | The front tab moves right / left and goes round at the ends; the window title follows the repository |
| 245.4 | Press Alt+1 .. Alt+3 with three tabs | That tab comes to the front; Alt+9 with three tabs does nothing |
| 245.5 | Click a tab; click the cross on a tab; middle-click a tab | The tab comes to the front; the tab closes; the tab closes |
| 245.6 | Close the front tab with Ctrl+W | The tab to its right takes the front (the one to its left if it was last); closing a tab that is not in front keeps the same tab in front |
| 245.7 | With one tab left press Ctrl+W | A toast says it is the only tab; the window stays |
| 245.8 | Change a file in a repository behind the front tab (from a shell), then come back to its tab | The tab shows the change within a moment (it was read while behind, more slowly) |
| 245.9 | Open two tabs, close the window, start michi with no folder from another folder | The tabs come back, the one for the current folder in front, the others load when first shown; a tab whose folder was deleted is not restored; `michi <folder>` does not restore the others |
| 245.10 | Open tabs for two folders of one name (`a/michi`, `b/michi`) | The tabs read `michi (a)` and `michi (b)` |
| 245.11 | Open 12 tabs, then another repository | A toast says the tabs are full; nothing opens |
| 245.12 | Change the settings (theme, language) in one tab, move to another | Both tabs follow the new theme and language; a toast in one tab is not shown in the other |
| 245.13 | Switch EN and JA; check the repository screen footer, the `?` list and the bar | `Open in a tab`, `Close tab`, `Next tab`, `Previous tab`, `Go to tab` read well in both languages |

## 246. Patches: copy and paste-to-apply (`Y`, Ctrl+V)

| # | Do | Expect |
| --- | --- | --- |
| 246.1 | In the graph, select a commit and press `Y`; paste into a text editor | A toast says a patch of N lines was copied; the clipboard holds an emailed patch (`From <id> ...`, `Subject: [PATCH] ...`, then the diff) (seen: Y on a commit notifies "patch (21 lines) copied to the clipboard"; clipboard content not read) |
| 246.2 | Select a worktree's uncommitted (WIP) row and press `Y` | The clipboard holds a plain `diff --git` of the tracked changes; with no changes a toast says there is nothing to make a patch of (seen: Y on the WIP row notifies a 9-line patch) |
| 246.3 | Select a merge commit and press `Y` | A toast says a merge commit has no patch of its own; the clipboard is unchanged (skipped: the lab repo c1 has no merge commit) |
| 246.4 | Copy a patch from 246.1, switch the window to a worktree where it fits, press Ctrl+V (Cmd+V on macOS) over the graph | A bar asks to apply the pasted patch (N lines) to the files of that worktree; `y` applies it, a toast gives files and +/- counts, and the files show as uncommitted changes (nothing staged, no commit) (skipped: Ctrl+V of a clipboard patch cannot be driven by the harness) |
| 246.5 | Paste the same patch again and answer `y` | A toast says the apply failed with git's reason; no file changed (skipped: needs a clipboard paste) |
| 246.6 | Paste ordinary text (a branch name) over the graph; paste in the filter box and in the commit message box | Nothing is asked over the graph; the text goes into the boxes as usual (skipped: needs a clipboard paste) |
| 246.7 | Press Ctrl+V with the changes view, the blame screen or a bar in front | The paste does not open the question (skipped: needs a clipboard paste) |
| 246.8 | With a rebase or merge stopped in the worktree, paste a patch | A toast says the worktree is in the middle of it and to finish first (skipped: needs a clipboard paste and a stopped rebase) |
| 246.9 | Paste a patch while a Claude is working in the worktree | The question ends with the note that a Claude is working there (skipped: needs a real Claude or tsumugi session) |
| 246.10 | Check the keybar and `?` over the graph; switch EN and JA | `Y` and Ctrl+V are listed with their words in both languages |

## 258. Run a refused commit again (#58)

| # | Do | Expect |
| --- | --- | --- |
| 258.1 | Add a pre-commit hook that prints five lines and exits 1; stage a file, write a message, commit; in the panel of what git said press `r` | The panel shows `Run again` with an `r` key cap left of `Close`; `r` runs the commit again (the hook runs again, a new panel or toast follows) with the same message and staged files | [ ] (undecidable: spaces now type, but Ctrl+Enter on main raised no hook panel in the harness (the commit did not reach the hook; a later key landed in the message box), so the panel with Run again could not be shown) |
| 258.2 | Click `Run again` in the same panel | The same as `r` | [ ] (undecidable: spaces now type, but Ctrl+Enter on main raised no hook panel in the harness (the commit did not reach the hook; a later key landed in the message box), so the panel with Run again could not be shown) |
| 258.3 | Fix the hook so it passes (or remove it), then press `r` over the panel | The commit goes through, the panel closes and the message box is cleared | [ ] (undecidable: spaces now type, but Ctrl+Enter on main raised no hook panel in the harness (the commit did not reach the hook; a later key landed in the message box), so the panel with Run again could not be shown) |
| 258.4 | Make a write other than a commit fail with several lines | The panel has `Close` only, no `Run again`; `r` does nothing | [ ] (飛ばし: 空白を含むコミットメッセージがハーネスで打てず、hook も用意できない) |
| 258.5 | Switch EN and JA | `Run again` and `もう一度回す` | [ ] (undecidable: spaces now type, but Ctrl+Enter on main raised no hook panel in the harness (the commit did not reach the hook; a later key landed in the message box), so the panel with Run again could not be shown) |

## 248. Diffs ignoring whitespace (`w`)

| # | Do | Expect |
| --- | --- | --- |
| 248.1 | Commit a change that only re-indents a file plus one real edit; in the graph select it and press `w` | A toast says diffs now ignore whitespace; the detail's diff is read again and shows only the real edit (the re-indented lines are gone) (seen: w toasts that diffs now ignore whitespace; the detail shows the toggle button) |
| 248.2 | Press `w` again | A toast says whitespace changes show again; the full diff is back (seen: w again toasts that whitespace differences show again) |
| 248.3 | In the `2` view, change only the spacing of a line in a tracked file, then press `w` | The file stays in the list; its diff shows nothing to look at; `w` again brings the change back |
| 248.4 | With `w` on in the `2` view, select a file with a real change and press `l` | The lines cannot be picked (the diff is for looking at); Space still stages the whole file |
| 248.5 | With `w` on, select a worktree's WIP row and a branch comparison (`d` in the `3` view) | Both diffs ignore whitespace too |
| 248.6 | Check the keybar and `?` in the graph and the `2` view; switch EN and JA | `w` is listed with its words in both languages; the toasts read well |
| 248.7 | Look at a changed line in the detail diff | The words that changed within the line are marked (word diff), as before |

## 251. Commit search scope in the `/` filter (#51)

| # | Do | Expect |
| --- | --- | --- |
| 251.1 | Press `/`, type `p:readme` and Enter | The commits that changed a path containing `readme` (any case) stay bright, the rest dim; the scrollbar shows the hits | [ ] (skipped: needs drag or wheel, which the harness lacks) |
| 251.2 | Type `fix p:src/` | Only commits with `fix` in the subject that also touched `src/` hit | [ ] (判断不能: 空白を打てず、fix と p:src/ を並べられない) |
| 251.3 | Type `d:2026-10-01..2026-10-15` and Enter | Only commits made in that range hit | [ ] (飛ばし: d: の範囲に空白が要り、ハーネスで打てない) |
| 251.4 | Try `d:2.weeks.ago`, `d:..yesterday` and `d:2026-10-01..` | Each hits the commits in the range git reads from it (since, until, since) | [ ] (飛ばし: d: の範囲に空白が要り、ハーネスで打てない) |
| 251.5 | Type `ann` (an author), then look at the hint of the empty box in EN and JA | The author still filters; the hint names `p:` and `d:` and reads well in both languages | [ ] (飛ばし: 作者名を打つ検索と EN/JA の切り替えがハーネスで確かめきれない) |

## 236. Conflicts a 3-way view cannot show (#36)

| # | Do | Expect |
| --- | --- | --- |
| 236.1 | Merge a branch that changed a file the target deleted, open the conflict screen (`3`) | The file card says it was deleted on our side and changed on theirs, and a second line says "Ours: deleted it. Theirs: changed it."; the buttons offer delete or take their file |
| 236.2 | Merge branches that renamed the same file to two names | The old name card says both sides moved or removed it; the two new names show as files whose card says only one side has the name; each card says what each side did |
| 236.3 | Merge a branch that changed only a file executable bit while the other changed its content | The second line says one side "made it executable" and the other "changed it" |
| 236.4 | Merge a binary file changed on both sides, and a submodule moved on both | The cards say binary / has a submodule; "Open in a tool" (`w`) still works |
| 236.5 | Switch EN and JA; narrow the window | Both lines read well and are cut at the card edge, not over the buttons |

## 232. Bisect (#32)

| # | Do | Expect |
| --- | --- | --- |
| 232.1 | Select a commit, press `x`, select an older one, press `g` | A question names the worktree, says commits are checked out and the old position is kept under refs/michi/backup/; `n` or Esc leaves everything | [ ] (一部見た: x で bad の印と toast「… を bad にしました。次は good のコミットに印を付けてください (y)」、上の帯に bisect: bad .., good -。行の文言が古い: 印は x(bad)/y(good)で、g は移動の menu (b が bisect…)) |
| 232.2 | Answer `y` | Bisect starts; the strip shows the commit under test and the steps left; a toast names the backup | [ ] [~] harness 0.106.11 (lab bs1, 8 commits, 1100x600): marking a commit bad with x and an older one good with y starts the bisect with no question window in between. The top strip then reads "Bisect: test da8694c step 5, then mark it good, bad or skip. About 1 steps left (2 commits to look at)" and a toast names the backup (michi/backup/20261022-153521). The row's wording is stale (it expects a question answered with y) |
| 232.3 | Start with uncommitted tracked changes | Refused with the reason; nothing changed | [ ] (飛ばし: 232.2 の手順が前提) |
| 232.4 | Start in the middle of a merge or rebase | Refused with the reason; nothing changed | [ ] (飛ばし: 途中の merge/rebase を作る手順が要る) |
| 232.5 | Press `g` / `x` / `u` on each tested commit until the end | Strip updates each step; at the end it names the first bad commit | [ ] (飛ばし: 232.2 の手順が前提) |
| 232.6 | Press `X` while bisecting, answer `y` | The worktree returns to where it was; the backup ref stays; the strip goes | [ ] (飛ばし: 232.2 の手順が前提) |
| 232.7 | Press `r` | A `git bisect run` line is on the clipboard and works when `<test-command>` is replaced | [ ] (飛ばし: クリップボードの中身をハーネスで読めない) |
| 232.8 | Run a bisect with a second worktree present | The other worktrees are untouched | [ ] (飛ばし: 232.2 の手順が前提) |
| 232.9 | With a Claude working in the worktree, start | The question also says a Claude is working there | [ ] (skipped: needs a real Claude or tsumugi session) |
| 232.10 | Switch EN and JA; check the keybar and `?` | The bisect keys appear; text reads well in both languages | [ ] [~] harness 0.106.11 (lab bs1, 1100x600): during a bisect the top strip lists Good y, Bad x, Skip u, Copy bisect run r, End bisect X in EN, and good y, bad x, スキップ u, bisect run をコピー r, bisect を終える X in JA; the JA sentence is cut with … at 1100 px. `?` opens in both languages, but its first page (Everywhere, Graph) does not reach the bisect keys, so those are not judged |
| 232.11 | Narrow the window (about 1100 px, JA) while bisecting, then rest the pointer on the strip text | The strip text ends in `…` and does not run under the buttons; the whole text shows in a tooltip (not when it fits) | [ ] fixed in 0.106.14 (#155); the tooltip cannot be driven by the harness |
| 232.12 | Make the window as narrow as it goes (720 px) | The search box and the bell stay left of the minimize/maximize buttons; tabs that do not fit are left out | [ ] (0.106.15, #145) |

## 242. Gitflow (`G`)

| # | Do | Expect |
| --- | --- | --- |
| 242.1 | In a repository with `main` and `develop`, press `G` over the graph (or over the `3` view with a worktree row selected) | A palette opens: `Gitflow`, `in <worktree>`, rows to start a feature, a release and a hotfix, and `Finish <branch>` for the worktree's branch; typing a name does not hide the rows | [ ] (判断不能: ハーネスで G を送っても palette が開いたように写らない (キーが変わった可能性。g の menu に「Git フロー早見表 ?」がある)) |
| 242.2 | Press Enter on a start row with nothing typed | A toast says to type a name first; the palette stays | [ ] (飛ばし: 242.1 が前提) |
| 242.3 | Type `login`, select the feature row, press Enter (no uncommitted change) | A toast says it started `feature/login`; the worktree is on it, made off `develop` | [ ] (飛ばし: 242.1 が前提) |
| 242.4 | Start a hotfix and a release the same way | The hotfix is off `main`, the release off `develop`; a name already taken is refused with a toast | [ ] (飛ばし: 242.1 が前提) |
| 242.5 | Commit on `feature/login`, press `G`, select the finish row, Enter | A bar asks first and says the tips are kept under `refs/michi/backup/`; `y` merges it into `develop` with a merge commit (`--no-ff`), deletes `feature/login`, leaves the worktree on `develop`; nothing is pushed | [ ] (飛ばし: 242.1 が前提) |
| 242.6 | Finish a `release/1.0` and a `hotfix/1.0.1` | Each is merged into `main`, tagged (annotated), then merged into `develop`, then deleted; the toast names the tag | [ ] (飛ばし: 242.1 が前提) |
| 242.7 | Set `gitflow.branch.master`, `gitflow.branch.develop` and `gitflow.prefix.*` (as `git flow init` writes), start and finish | The configured names and prefixes are used | [ ] (飛ばし: 242.1 が前提) |
| 242.8 | Edit a tracked file or add an untracked file, then start or finish | A toast says the worktree has uncommitted changes; nothing changed | [ ] (飛ばし: 242.1 が前提) |
| 242.9 | Check `develop` out in another worktree, then finish a feature | A toast says `develop` is checked out there; nothing changed, and that worktree is untouched | [ ] (飛ばし: 242.1 が前提) |
| 242.10 | Make `main` and a release branch change the same line, then finish the release | A toast says the merge would clash; no branch moved, no backup was made | [ ] (飛ばし: 242.1 が前提) |
| 242.11 | Finish a release whose tag already exists | A toast says the tag is already there; nothing changed | [ ] (飛ばし: 242.1 が前提) |
| 242.12 | Stop in the middle of a rebase or merge, then use a row | A toast says it is in the middle of it | [ ] (飛ばし: 242.1 が前提) |
| 242.13 | Finish while a Claude is working in the worktree | The question ends with a note that a Claude is working there | [ ] (skipped: needs a real Claude or tsumugi session) |
| 242.14 | Check the keybar and `?` over the graph and the `3` view; switch EN and JA | `G` Gitflow is listed; the palette and toasts read well in both languages | [ ] (2026-10-22 ran EN/JA F1: Gitflow is listed in the 3 view as `U` (Gitflow), not `G` as the row says; `i` is the Git flow sheet everywhere; row text needs a decision; palette and toasts not tried) |

## 243. External editor, diff and merge tools (#43)

michi has no editor of its own; it opens an external one at a line and an external diff / merge tool (`ito-openin`, shared with kura). Templates: `{path} {line} {left} {right} {base} {merged}`.

| # | Do | Expect |
| --- | --- | --- |
| 243.1 | In the changes view (`2`), select a file and press `e` | The file opens in the editor (`[open] editor`); a toast names it |
| 243.2 | Focus the lines (`l`), move to a changed line, press `e` | The editor opens at that line (VS Code: `-g path:line`); on a deleted line it opens at the next line that exists |
| 243.3 | With an editor that has no line argument (see Settings > External tools > line arguments), press `e` on a line | It opens the file and the toast says the editor takes no line |
| 243.4 | Add a line argument for it in Settings (`code` = `-g {path}:{line}`) and repeat 243.2 | It now opens at the line |
| 243.5 | Open blame (`B`), move to a line, press `e` | The working file opens at that line (on the History tab, at the top) |
| 243.6 | Press `x` on a modified file in the changes view with `diff_tool = "winmerge"` (or any found tool) | The tool opens with the committed (or staged) side on the left and the working file on the right; the UI never freezes |
| 243.7 | Press `x` on a deleted file and on a file with no diff | A toast says there is nothing to open |
| 243.8 | Open a conflict (merge in progress), press `w` with a merge tool set or found | The merge tool opens with ours (left), theirs (right), the base and the merged file; saving the merged file is read by michi after the tool closes |
| 243.9 | WinMerge 3-way: the same, with WinMerge as the tool | UNVERIFIED on a real machine: the built-in WinMerge 3-way command line must open the three sides and the output file in the right panes |
| 243.10 | With no merge tool set and none found, press `w` | It falls back to `git mergetool` as before |
| 243.11 | In a conflict, press `E` on a block | The editor opens at that block's line |
| 243.12 | In the worktree table press `t`, `e`, `E` | The folder opens in the terminal, editor, file manager as before |
| 243.13 | Open Settings (`,`) and go to External tools | The page lists the five commands, the diff / merge tool row, the tools found (after a moment: "Looking for installed tools..." changes to "Found: ..."), your own tools and the line arguments; config.toml is named as the file |
| 243.14 | Change a command, pick a tool, add an own tool and a line argument, remove them | config.toml gets `[open]`, `[diff_tools.<name>]`, `[line_args]` edits (comments kept); a bad template shows an error and writes nothing |
| 243.15 | Edit config.toml by hand with an invalid `[open]` value | A warning shows on the page; the other settings keep working |
| 243.16 | Check the keybar, `?` and the palette in the changes view, blame and the conflict view; switch EN and JA | `e`, `x`, `E` are listed with their words in both languages |
| 243.17 | In Settings > External tools, scroll to OTHER APPS (EN and JA) | The card lists the uchmk apps found (kura, tsumugi, ...), each with how it was found (pinned here / registered by the app / found on PATH) and its version; before the scan ends it says it is looking | [ ] |
| 243.18 | Pin a path for an app (and an app that has not registered), then unpin it; type a bad name or a path that is not a file | common.toml gets `[apps]` edits (comments kept); the row says `pinned here` and then goes back; a bad name or path shows a message and writes nothing | [ ] |
| 243.19 | Start michi once, then `ls` the apps folder (`ito_common::apps::dir()`), and press the tsumugi keys (`s`, `f`, `N`) with tsumugi off PATH but registered or pinned | michi is registered there; tsumugi is started from the registered / pinned path | [ ] |
| 243.20 | With kura installed (or pinned), open the palette (`:`) and pick `Open the folder in kura`, on the Worktree table and on another view | kura opens on the selected worktree's folder (the window's own folder off the table); the toast says `Opened <wt> in kura`; with kura missing the toast names the error and nothing hangs | [ ] |

## 244. Issue links (`I`, detail links; #44)

| # | Do | Expect |
| --- | --- | --- |
| 244.1 | In a repo with a GitHub remote, select a commit whose message says `#123` | The detail shows an "Issues" line with `#123` in blue; hovering underlines it with a hand cursor (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 244.2 | Click that link | The browser opens `https://github.com/<owner>/<repo>/issues/123` |
| 244.3 | Press `I` over the graph | The same page opens; a toast names the issue and, if the message has more, how many more |
| 244.4 | Select a commit with no issue reference and press `I` | A toast says there is no issue |
| 244.5 | In the worktree table, look at a row whose branch is `123-fix-x` or `GH-123` | A blue "Issue #123" line under the path, and an "Open the issue" button with `I` |
| 244.6 | Select that row and press `I` (or click the button) | The issue opens in the browser; on the trunk row or a branch without a number nothing is offered |
| 244.7 | Set `[issues] keys = "https://example.atlassian.net/browse/{key}"` in config.toml and select a commit naming `ABC-123` | `ABC-123` links to that address; without the setting it is plain text |
| 244.8 | Add `prefixes = ["ABC"]` and name `XYZ-9` and `ABC-1` in one message | Only `ABC-1` links |
| 244.9 | Set `github = false` | `#123` no longer links |
| 244.10 | Give `keys` a template without `{key}` or a non-http address | A warning toast names the setting; keys are not linked |
| 244.11 | Open a repo with no GitHub remote, with `#123` in a message | No link for `#123`; `owner/repo#123` still links (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 244.12 | Check the keybar and `?` in the graph and the worktree table; switch EN and JA | `I` is listed with its words in both languages |

## 247. Statistics: contributors and activity (`Z`)

| # | Do | Expect |
| --- | --- | --- |
| 247.1 | In the graph of a repository with some history, press `Z` | A panel opens at once ("Reading…" for a moment), then shows a row of bars for commits per week (oldest at the left, newest at the right edge), and the contributors, most commits first, each with a bar, a count and a share |
| 247.2 | Hover a bar | A tooltip gives the week and its commit count; empty weeks between are gaps, not skipped |
| 247.3 | Press `j` `k`, PageDown, PageUp, Home, End; click a contributor row | The selection moves and stops at the ends; the selected row also shows the dates of their first and last commit |
| 247.4 | Press `r` three times | The range goes: all branches and tags / last 90 days / current branch (HEAD) / all again; the title line names it, the numbers are read again and the old ones are not shown meanwhile |
| 247.5 | Press `r` twice in quick succession on a large repository | The screen ends on the numbers of the last range, never the earlier one's |
| 247.6 | Press `w` | The bars switch between weeks and months and the title says which |
| 247.7 | Add a `.mailmap` joining two spellings of a person in a repository with both | They appear as one contributor |
| 247.8 | Open it over the worktree table (`3`) and in a repository with no commit yet | It opens from both; with no commit it says there are no commits in the range |
| 247.9 | Press Esc, or click outside the panel | The panel closes; no git index.lock is taken while it reads (a Claude committing in the same worktree is not blocked) |
| 247.10 | Check the keybar and `?` over the graph and the worktree table, and over the panel; switch EN and JA | `Z` is listed, the panel lists j k, r, w, Esc; text reads well in both languages and the bars stay inside the panel in a narrow window |

## 257. Pre-merge checklist (`g`)

| # | Do | Expect |
| --- | --- | --- |
| 257.1 | In the `3` view select a worktree on a feature branch and press `g` | A toast says it is checking, then a list opens: `Before merging <branch> into <trunk>` with six rows, each marked OK, !! or ?? and a reason | [ ] (判断不能: 3 の wt2 行で g を押しても、一覧も toast も写らなかった (キーが変わったか toast が消えた)) |
| 257.2 | Press `g` on the trunk's own row, and on a detached worktree | No list; a toast says why (the trunk itself, or on no branch) | [ ] (飛ばし: 257.1 が前提) |
| 257.3 | Edit a file in the worktree and press `g` again | The Working tree row is !! and counts the changed files | [ ] (飛ばし: 257.1 が前提) |
| 257.4 | Make the branch clash with the trunk (same lines changed) and press `g` | The clash row is !! and names the files; a clean branch says the trunk merges in without a conflict | [ ] (飛ばし: 257.1 が前提) |
| 257.5 | Press `g` on a branch with an unpushed commit, then on one with no upstream | The Pushed row is !! with the count, then with `no upstream` | [ ] (飛ばし: 257.1 が前提) |
| 257.6 | Land the branch with `l` (verify runs) or run a failing verify, then press `g` on the same commit | The verify row says passed (OK) or failed with the exit code (!!); a new commit goes back to `no result recorded` (??) | [ ] (飛ばし: 257.1 が前提) |
| 257.7 | With a GitHub remote and gh signed in, push the branch and press `g` | The GitHub checks row counts passed, failed or pending; with the network off it says it could not be read (??) and the list still opens | [ ] (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 257.8 | Press any key, then Esc; click outside the list | Other keys do nothing; Esc and an outside click close it | [ ] (飛ばし: 257.1 が前提) |
| 257.9 | Check the keybar, `?` and the palette in both languages | `g` is listed as `Check before merging` / `マージ前の確認` | [ ] (2026-10-22 ran EN/JA F1: Check before merging / マージ前の確認 is listed under `R` in the 3 view; `g` is listed as the first row / 先頭の行へ, so the row's key does not match; palette not tried) |

## 241. Sign-in help (#41)

| # | Do | Expect |
| --- | --- | --- |
| 241.1 | Point `origin` at an ssh address you have no key for (or stop ssh-agent) and press `f` | The failure toast ends with the cause ("the ssh key was refused") and "(K for the fix)" |
| 241.2 | Press `K` after 241.1 | A panel names the cause, shows the remote, scheme, host, credential helper and ssh agent state, and lists numbered commands (`ssh-add`, `ssh -T git@<host>` ...) with a line on what each does |
| 241.3 | Press `1`, then paste into a terminal | The toast says the command was copied and the clipboard holds exactly that command; michi ran nothing and nothing in `~/.ssh` or the agent changed |
| 241.4 | Set `origin` to an https address with a wrong host or no helper (`git config --unset-all credential.helper`) and press `f` | The toast names the https sign-in failure; `K` lists `git config --global credential.helper ...`, `gh auth login`, `gh auth setup-git`; no terminal prompt appears and no password is asked (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 241.5 | Fetch from a host that does not resolve (`https://nohost.invalid/x.git`) and press `K` | The cause is "the host could not be reached"; the panel offers `git ls-remote --heads origin` and the proxy check |
| 241.6 | With no failure since the start, press `K` (and use the palette entry "Sign-in help") | The panel opens with "No sign-in failure yet" and the checks for the remote's scheme |
| 241.7 | Clone from the repository screen with an address that needs a login you lack, then press `K` | The failure toast has the cause and the hint; the panel shows the facts for the clone's address (not an existing repository) |
| 241.8 | Put a token in an https remote URL (`https://user:token@github.com/o/r.git`) and open the panel | Neither the token nor the user name is shown anywhere in the panel or the toast |
| 241.9 | Press `Esc`, or click outside the panel; open it again and press `?` | The panel closes; the keybar and `?` list `1-9` and `Esc` in the panel |
| 241.10 | Switch EN and JA with the panel open | Every line (cause, help text, facts, command notes, keys) is in the chosen language; the commands stay as typed |

## 234. Creating a pull request (`Q`, #34)

| # | Do | Expect |
| --- | --- | --- |
| 234.1 | In the `3` view select a worktree whose branch has commits trunk lacks and press `Q` | A form opens at the top: the title is the last commit subject, the body lists the commit subjects, the base is the trunk, the draft box is off; the title box has focus |
| 234.2 | Select a branch with no upstream | The form says the branch will be pushed to `<remote>` first; Ctrl+Enter shows a second line asking to confirm and the button turns to `Push and create`; `Cancel` goes back; nothing was pushed yet |
| 234.3 | Confirm with a second Ctrl+Enter (or the button) on a throwaway branch of a repository you own | The window stays responsive while it works; the branch is pushed with `-u`, a pull request is made, and the form shows its address with Open, Copy and Close; a toast names the address and it is on the clipboard (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 234.4 | Click Open, then Copy, then Close | The browser opens the pull request; a toast says copied; the form closes (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 234.5 | Tick draft, make a pull request on a branch that already has an upstream | No push line; the pull request on GitHub is a draft (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 234.6 | Empty the title or the base and press Ctrl+Enter | A toast says one is needed; nothing runs |
| 234.7 | Run with `gh` off the PATH | A toast says gh was not found and where to get it; the form stays open and editable (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 234.8 | Run after `gh auth logout` | A toast points to `gh auth login`; the form stays open (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 234.9 | Make a pull request for a branch that already has one | A toast carries gh's first line; the form stays open (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 234.10 | Press Esc, or click outside, while the form is open and idle; press `Q` on a row without a branch | The form closes; the toast says the row has no branch |
| 234.11 | Check the keybar and `?` in the `3` view; switch EN and JA | `Q` Create a pull request is listed; the form reads well in both (skipped: needs gh / GitHub / minato, which tests must not touch) |

## 260. Image diff (#48)

| # | Do | Expect |
| --- | --- | --- |
| 260.1 | Commit a changed `.png` (a different picture, same name); in the graph select the commit | The file shows a "Before" and an "After" picture side by side, with the size in pixels and the bytes of each (`1.5 KB`), not "A binary file" | [ ] (飛ばし: 画像の差分の確認で、png の用意と 2 つの画像の見た目の判断が要る) |
| 260.2 | Press `O` three times with the pointer over the pictures | The mode name at the right of the line changes: swipe (the new picture shows right of a blue divider that follows the pointer), onion skin (the new one fades in as the pointer moves right), difference (red where the pixels differ, with a count of differing pixels, or "Identical") | [ ] (飛ばし: 260.1 が前提) |
| 260.3 | Press `O` once more | Back to side by side; the mode stays when you select another commit | [ ] (飛ばし: 260.1 が前提) |
| 260.4 | Select a commit that adds a picture, then one that deletes a picture | Added: "Before: none" and only the after; deleted: only the before; the modes other than side by side fall back to it | [ ] (飛ばし: 260.1 が前提) |
| 260.5 | Select a commit that renames a picture and changes it | The before is the old name's picture | [ ] (飛ばし: 260.1 が前提) |
| 260.6 | Try `.jpg`, `.gif`, `.webp`, `.bmp` and `.ico` changes | Each shows (a gif shows its first frame); a transparent picture sits on a checkerboard | [ ] (飛ばし: 画像形式ごとの実物が要る) |
| 260.7 | Commit a picture over 16 MB, or one with more than 16 million pixels | That side says it is too big (with its size) or has too many pixels; nothing freezes and memory stays low | [ ] (飛ばし: 16 MB の画像が要る) |
| 260.8 | Commit a `.png` that is not a picture (a text file renamed) | That side says it cannot be decoded; the rest of the detail is fine | [ ] (飛ばし: 260.1 が前提) |
| 260.9 | In the `2` view, replace a tracked picture on disk with another | The unstaged entry shows the index picture against the worktree one; `Space` to stage it and the staged entry shows HEAD against the index | [ ] (飛ばし: 260.1 が前提) |
| 260.10 | In the `2` view, add a new untracked picture, then delete a tracked one | New: only the after; deleted: only the before | [ ] (飛ばし: 260.1 が前提) |
| 260.11 | While in the `2` view with a picture shown, overwrite it again on disk | The picture updates without the view flickering back to "Reading the pictures..." | [ ] (飛ばし: 画像を上書きして更新を見る必要がある) |
| 260.12 | Scroll a commit that changes many pictures | The scroll stays smooth; pictures load as they come into sight, the layout shifts at most a little | [ ] (飛ばし: wheel がハーネスに無い) |
| 260.13 | Open `?` in the graph and the `2` view; switch EN and JA | `O` is listed with its words in both languages; the line over the pictures reads well in both | [ ] (partly seen 2026-10-22: F1 in the changes and 3 views in EN and JA; `O` is listed as Image diff in the changes list; the graph list and the line over the pictures not looked at) |

## 261. Settings backup (Advanced page)

| # | Do | Expect |
| --- | --- | --- |
| 261.1 | Open settings (`,`), go to the Advanced page | A Backup card with a file name (in the home folder), Export and Import; EN and JA both read well |
| 261.2 | Press Export | A toast says how many settings files were written; the file holds common.toml, themes, and michi's config.toml / keymap.toml, not the window's place |
| 261.3 | Change the theme and a key, press Import once, then again | The first press only arms the button; the second reads the file back, a toast says what was read, and the theme and the key are as exported within a couple of seconds |
| 261.4 | Import a file that is not a settings bundle | A toast gives the reason and nothing in the settings folder changes |
| 261.5 | Search "backup" in the settings | The Advanced page is listed |

## 114. Operation history (#114)

| # | Do | Expect |
| --- | --- | --- |
| 114.1 | Commit, switch branch, reset to `HEAD~1`, then press `L` | Rows read in plain words with the time at the left ("Committed: ...", "Switched from a to b", "Moved the branch to HEAD~1"), the commit id at the right; michi's backups read "michi kept this before: ..." | [ ] (seen: L lists plain-word rows (moved the branch to HEAD~1, committed c3) with the time at the left and the id at the right; no michi backup row since none was made in ig1) |
| 114.2 | Press `r` in the list, then `r` again | The rows switch to git's own wording (`HEAD@{n}`, `checkout: moving from ...`) and back | [ ] (seen: r switches to HEAD@{n} and reset: moving to ... wording; r back not tried) |
| 114.3 | Select a row and press `g` | The list closes and the graph shows that commit selected; for a commit not in the graph a toast says so | [ ] (seen: g on c3 (not in the graph) closes the list and toasts that it is not in the history) |
| 114.4 | Select a row and press Enter | The band asks to bring it back as a new branch (the undo path); nothing moves until `y` | [ ] (飛ばし: バックアップを作る操作 (force push や branch の削除) が先に要る) |
| 114.5 | Check the keybar and `?` with the list open; switch EN and JA | `g` and `r` are listed; both languages read well | [~] (2026-10-22 ran EN/JA F1 in the changes view: `g` and the g+d, g+a, g+i chords are listed, `[ ]` is Another worktree; JA in the 3 view lists g and g+m, g+l, g+x, g+c; `r` is listed as rebase in the 3 view; both languages read well; list not scrolled) |

## 108. Git errors as a cause and a next step (#108)

| # | Do | Expect |
| --- | --- | --- |
| 108.1 | Make the remote ahead of a branch (commit from another clone), then press `P` (push) without fetching | The toast says the remote has commits this branch does not have, to fetch and pull or rebase, and ends with `(git said: …)` holding git's own words |
| 108.2 | Edit a file that another branch also changes, then switch to that branch from the picker | The toast says uncommitted changes would be overwritten, to commit or stash first; the detail names the file |
| 108.3 | Create a worktree for a branch that is already open in another worktree | The toast says the branch is open in another worktree and offers switching or a new branch |
| 108.4 | Switch the language to Japanese and repeat 108.1 | The same sentence in Japanese, the detail still in git's English; the toast stays readable at its width |
| 108.5 | Trigger a refusal michi has no sentence for (an odd hook output) | git's words are shown as before, nothing extra |

## 104. Easy display (plain wording of Git's terms)

| # | Do | Expect |
| --- | --- | --- |
| 104.1 | Open settings (`,`), General page, find "Easy display" (also by searching "easy") | A card with a checkbox "Show Git terms in plain words"; off by default; EN and JA both read well | [ ] (undecidable: the settings search box takes focus but typed text does not land, and the page needs a wheel to scroll) |
| 104.2 | Tick it, open the `2` view and the `1` view | The list headings read "In the next commit (Staged)" / "Not in the next commit (Not staged)"; the footer reads "N in the next commit"; the side column's headings read "working folder (worktree)", "name for a commit (tag)", "set aside for later (stash)"; JA reads "次のコミットに入る（Staged）", "あとで使うために置いておく（stash）" | [ ] (seen in the 2 view with easy=true: headings read 次のコミットに入る（Staged） / 次のコミットに入らない（Not staged）, footer 次のコミットに 2 件; side column not compared) |
| 104.3 | Untick it (or delete `easy` from `[view]` in config.toml) | The usual words come back within a couple of seconds, without restarting | [ ] (skipped: no way to untick from the harness; not seen) |
| 104.4 | Write `easy = "yes"` under `[view]` in config.toml | A warning about `[view] easy` shows and the usual words stay | [ ] (undecidable: the bell count rose 1 to 2 after easy = "yes" but the toast was not in any frame) |

## 105. Hover glossary

| # | Do | Expect |
| --- | --- | --- |
| 105.1 | Look at the `2` view's list headings, the file title over the diff and the side column's worktree / tag / stash headings | Each of these terms has a dotted underline | [ ] (seen: dotted underlines under the 2 view headings and the WORKTREE heading) |
| 105.2 | Rest the pointer on one for a moment | A small card shows the Git word, its plain wording and a one or two line explanation; it goes when the pointer leaves; it does not hide the click targets nearby | [ ] (skipped: no hover tooltip in the harness) |
| 105.3 | Repeat with the easy display on, and with EN and JA | The card is the same either way and reads well in both languages; the dots line up under the whole label | [ ] (skipped: no hover tooltip and no EN/JA switch in the harness) |

## 115. Basic and full levels

| # | Do | Expect |
| --- | --- | --- |
| 115.1 | Open settings (`,`), General page: find "How much to show" | Basic and Full buttons, Full selected by default; EN and JA both read well | [ ] (undecidable: needs the settings page scrolled; not reachable) |
| 115.2 | Choose Basic | Within a couple of seconds `[view] level = "basic"` is in config.toml; the key bar loses reset, discard, rebase and force push and keeps the everyday keys | [ ] (skipped: needs a click on the Basic button in the scrolled settings page) |
| 115.3 | In Basic, press `?` | The list leaves out the same keys and ends with a line saying how many more commands are in the command palette | [ ] (undecidable: ? list with level = basic screenshotted, but the frame lagged and showed no list) |
| 115.4 | In Basic, open the command palette (`Ctrl+Shift+P`) and search "reset" or "discard" | The command is found and picking it works as the key does | [ ] (skipped: needs the palette typed text, which does not land reliably) |
| 115.5 | In Basic, press a hidden key directly (e.g. the discard key in the `2` view) | It still works, and still asks before it destroys anything | [ ] (skipped: needs a destructive key against a lab repo; not driven) |
| 115.6 | Choose Full | Key bar and `?` list every key again | [ ] (skipped: needs the settings page) |
| 115.7 | Write `level = "expert"` in `[view]` by hand | A warning says it is not basic or full, and Full is used | [ ] (undecidable: the bell count rose after level = "expert" but the toast was not in any frame) |
| 115.8 | Search "basic" in the settings | The General page is listed | [ ] (undecidable: settings search typing does not land) |

## 116. First-run guide

| # | Do | Expect |
| --- | --- | --- |
| 116.1 | Run michi with an empty home (no `user.name`, no `user.email`; e.g. `GIT_CONFIG_GLOBAL` pointing at an empty file) | A "Getting ready" panel opens at the top naming the missing name and email, with the `git config --global` commands to copy | [ ] [~] harness (lab bs1, GIT_CONFIG_GLOBAL=empty file + GIT_CONFIG_NOSYSTEM=1, 1100x600): a "Getting ready" panel opens at the top, names the missing user.name and user.email, and shows `git config --global user.name "Your Name"` / `user.email "you@example.com"` to copy. (Seen only when the repo has no local name/email: lab m1 has them and shows no panel.) |
| 116.2 | Click a command in the panel | It is copied (a toast says so) and the panel stays | [ ] (飛ばし: クリップボードに触る) |
| 116.3 | With the panel open, move around (`j`, `k`, `3`) | Keys still work under the panel; it does not take them | [ ] [~] harness (lab bs1): with the panel open, `3` switched to the Worktrees tab and the panel stayed; the panel does not take the keys. (`j`/`k` not driven.) |
| 116.4 | Press `Esc`, then open the command palette and pick "Getting ready" | Esc closes the panel; the palette entry brings it back | [ ] [~] harness (lab bs1): `Esc` closes the panel. The palette entry that brings it back was not driven (palette typing). |
| 116.5 | Click outside the panel | It closes | [ ] [~] harness (lab bs1): a click at the bottom of the window (outside the panel) closed the panel. |
| 116.6 | Start michi with git off the PATH | The panel says git is not installed and offers the install command for the OS (winget / brew / apt) and the download page | [ ] (飛ばし: git を PATH から外すと drive.ps1 も動かない) |
| 116.7 | Start michi with git 2.38 or newer and a name and email set | No panel opens and the palette has no "Getting ready" entry | [ ] (見た: 名前とメールのある普通の環境では panel は出ない (palette の項目は未確認)) |
| 116.8 | Read the panel's last lines | It points to `K` for the sign-in check; `K` still opens the sign-in help | [ ] [~] harness (lab bs1): the last lines read "To check the sign-in to the remote (GitHub), press K." then "Esc or a click outside closes this; the command palette brings it back." Pressing `K` was not run (GitHub/network). |
| 116.9 | Switch EN and JA | The panel reads well in both | [ ] [~] harness (lab bs1): EN reads well; JA (はじめの準備…) read well in an earlier run. Switching live in the same run is not driven (the run sets the language at start). |

## 117. Practice repository

| # | Do | Expect |
| --- | --- | --- |
| 117.1 | Open the palette (`<C-S-p>`), type "practice", pick "Make a practice repository" | A toast says it is being made, then another gives the folder (`michi-practice-1` in the temp folder); a new tab opens on it, with the window staying responsive meanwhile | [ ] (飛ばし: 一時フォルダの練習リポジトリを作る操作はパレットに文字入力が要り、ハーネスに無い) |
| 117.2 | Look at the graph of the practice tab | About ten commits, a merge, branches `main`, `fix/list`, `feature/greeting`, `topic-a` (checked out), `topic-b`, the tag `v1.0`, and a dashed WIP row on top | [ ] (飛ばし: 一時フォルダの練習リポジトリを作る操作はパレットに文字入力が要り、ハーネスに無い) |
| 117.3 | Merge `topic-b` into `topic-a` (stash or commit the waiting change first if asked) | It stops at a real conflict in `colors.txt`; the conflict screen opens and can be solved and committed | [ ] (飛ばし: 一時フォルダの練習リポジトリを作る操作はパレットに文字入力が要り、ハーネスに無い) |
| 117.4 | Make a second practice repository | It gets its own folder (`michi-practice-2`); the first is untouched | [ ] (飛ばし: 一時フォルダの練習リポジトリを作る操作はパレットに文字入力が要り、ハーネスに無い) |
| 117.5 | Close the tab and delete the folder | Nothing else is affected; michi keeps running; your own git config and repositories are unchanged | [ ] (飛ばし: 一時フォルダの練習リポジトリを作る操作はパレットに文字入力が要り、ハーネスに無い) |
| 117.6 | Commit in the practice repository on a machine with no `user.name` set | The commit works (the name is kept in the repository's own config) | [ ] (飛ばし: 一時フォルダの練習リポジトリを作る操作はパレットに文字入力が要り、ハーネスに無い) |
| 117.7 | Switch EN and JA and open the palette | The command reads well in both | [ ] (飛ばし: 一時フォルダの練習リポジトリを作る操作はパレットに文字入力が要り、ハーネスに無い) |

## 118. Legend of the graph

| # | Do | Expect |
| --- | --- | --- |
| 118.1 | In the graph (`1`), look at the header | A "Legend" button sits left of "Fold PRs"; it hides on a very narrow window instead of overlapping | [ ] [~] harness 0.106.11 (lab m1): Legend sits left of Fold PRs at 1100 and at 480 px wide; no overlap in either |
| 118.2 | Click it | A panel lists lane colours, commit, merge, uncommitted, conflict, faded or dashed ends, branch, worktree, remote, tag, "+ origin", both warning pills and the new-commit dot | [ ] (見た: クリックで凡例の panel が開き、線の色・コミット・マージ・未コミット・衝突・薄れ・ブランチ・worktree・リモート・タグ・+origin・件名の点が並ぶ) |
| 118.3 | Compare each drawing with the real one in the graph | Each looks the same as in the graph (dot, ring, dashes, pill styles and colours) | [ ] (判断不能: 描き方が本物と同じかは画面の画像だけでは比べきれない) |
| 118.4 | Press any key, Esc, or click outside | It closes | [ ] [~] harness 0.106.11 (lab m1): Esc, any key (j) and a click outside each close the panel |
| 118.5 | Palette: "Show the legend of the graph" | The graph view opens with the legend on top | [ ] (飛ばし: palette への入力がハーネスで届かない) |
| 118.6 | Switch EN and JA, and the themes | Words read well in both; the drawings stay readable on light and dark themes | [ ] [~] harness EN/JA × light/dark の凡例 (「樹形図の見方」) を撮った: 全部の図と文言が読め、どちらのテーマでも読める。閉じる案内は「Any key or a click closes this」 |

## 262. Everything by mouse (#123)

| # | Do | Expect |
| --- | --- | --- |
| 262.1 | In each view, hover an entry of the keybar at the bottom | The entry lights up, the cursor is a hand; a click does what the key does (`c`, `Space`, `?`, `Esc` ...) |
| 262.2 | Click `?` in the keybar, then `Esc` in the keybar | The help opens and closes; the same for the settings (gear in the top bar) |
| 262.3 | Click the entries that are several keys (`j k`) | Nothing happens (they are not buttons) |
| 262.4 | Press `Space` in the palette list | `Space` is listed and runs the view's Space action |
| 262.5 | Go through every view and overlay with the mouse only | Known gaps (moving by row, text selection by drag in the diff) are the ones listed in TODO.md; any other gap is a new finding (skipped: needs drag or wheel, which the harness lacks) |

## 263. Commit message guide (#124)

| # | Do | Expect |
| --- | --- | --- |
| 263.1 | Open Changes with an empty message | A dim line under the box: first line about 50 characters, with an example |
| 263.2 | Type `fix` | The line turns to the warning color: the first line is very short |
| 263.3 | Type a first line of 60 characters | The warning color, with the length; with 50 or fewer it is dim `First line N/50` |
| 263.4 | Switch EN and JA; narrow the window | Both read well; the line is cut at the width and the commit still works |

## 264. Terms (#125)

| # | Do | Expect |
| --- | --- | --- |
| 264.1 | Read GLOSSARY.md, then walk the screens in JA and EN | Each idea uses the word in the table (branch/ブランチ, 衝突, 差分, 作業ツリー, 消す, 捨てる ...) |

## 110. A commit on main asks first (#110)

| # | Do | Should | Notes | OK |
|---|----|--------|-------|----|
| 110.1 | In a worktree on `main`, stage a file, write a message, press Ctrl+Enter in the `2` view | A band asks before committing, saying the commit is on main itself; Esc leaves everything as it was, `y` commits | | [ ] (判断不能: cm1 (master) で Ctrl+Enter を押したが、最後の画面でも確認の帯もコミットも写っていない (画面の遅れか入力が届かなかった)) |
| 110.2 | Do the same on a feature branch | The commit goes straight, with no band | | [ ] (判断不能: cm2 (feat) で Ctrl+Enter を押したが、最後の画面が変わらず直行したか判断できない) |
| 110.3 | Write `[commit]` `protect = ["release/*"]` in config.toml (no restart); commit on `release/1.0`, then on `main` | `release/1.0` asks, `main` does not | | [ ] (飛ばし: config.toml を書き換えて別ブランチでコミットする連続操作が要る) |
| 110.4 | Write `protect = []` and commit on `main` | No band | | [ ] (飛ばし: config.toml を書き換えて main でコミットする連続操作が要る) |
| 110.5 | Commit on `main` while a Claude is writing in the worktree | One band tells both reasons; one `y` covers both | | [ ] (skipped: needs a real Claude or tsumugi session) |
## 111. A commit warns of big files and secrets (#111)

| # | Do | Should | Notes | OK |
|---|----|--------|-------|----|
| 111.1 | Stage a `.env` (or `server.pem`) on a feature branch and press Ctrl+Enter | A band names the file and says it looks like a secret; Esc, unstage it, and the commit then goes straight | | [ ] (判断不能: cm2 に .env をステージして Ctrl+Enter。帯が写っていない) |
| 111.2 | Stage a text file holding `-----BEGIN OPENSSH PRIVATE KEY-----` or an `AKIA...` key id | The band names it with what it looks like (private key, AWS access key id) | | [ ] (飛ばし: 秘密や大きいファイルを用意して帯の文言を見る手順が未整備) |
| 111.3 | Stage a file of 60 MB | The band names it with its size; `y` commits anyway | | [ ] (飛ばし: 秘密や大きいファイルを用意して帯の文言を見る手順が未整備) |
| 111.4 | Stage `.env.example` and a 2 MB file | No band | | [ ] (飛ばし: 秘密や大きいファイルを用意して帯の文言を見る手順が未整備) |
| 111.5 | Write `large_mb = 1` under `[commit]` (no restart) and stage the 2 MB file | The band appears; with `large_mb = 0` it does not | | [ ] (飛ばし: 秘密や大きいファイルを用意して帯の文言を見る手順が未整備) |
| 111.6 | Stage five secret-looking files | The band names three and says "and 2 more"; the text is cut off with an ellipsis if narrow | | [ ] (飛ばし: 秘密や大きいファイルを用意して帯の文言を見る手順が未整備) |
| 111.7 | Stage a few thousand files and watch the `2` view | It stays quick; the check runs off the UI thread | | [ ] (飛ばし: 数千ファイルの用意と体感の速さが要る) |

## 106. A plain forecast before the main operations (#106)

| # | Do | Expect |
| --- | --- | --- |
| 106.1 | On a worktree with uncommitted files and a followed branch, press `p` | A band asks first and says in plain words how many commits come in and that the uncommitted files are put aside and brought back | [ ] [~] harness 0.106.11 (lab pl1a: behind origin/main by 2, 2 uncommitted files; 1100x600): `p` opens "Are you sure?" — "2 commit(s) from origin/main will be brought into main. Nothing of yours is rewritten. The 2 uncommitted file(s) are put aside and brought back afterwards." JA reads the same. Not run to the end. |
| 106.2 | On a clean, up-to-date branch, press `p` | No band: the pull just runs | [ ] [~] harness (lab pl2a: clean, up to date): `p` shows no band; a toast says "Already up to date with origin/main". |
| 106.3 | Where the branch and its remote each have commits, press `p` with uncommitted files | The band says the pull will stop and to merge or rebase instead | [ ] [~] harness (lab pl3a: 1 ahead, 2 behind, 1 uncommitted file): the band says "You and origin/main both have commits the other lacks (yours 1, theirs 2), so a pull will stop. Merge or rebase instead." It still offers Go ahead (Ctrl+Enter); the sentence wraps to a 2nd line but is not cut. |
| 106.4 | In the `3` view, press `m` on a worktree | The band says how many commits of the trunk come in and whether it is expected to be clean or to stop on N files | [ ] [~] harness (lab m1, `3` then `j` then `m`, wt2 and wt3): the band asks "Merge main into wt2 in m1-wt?" and says "wt2 already has everything in main; nothing will change." Only that case was seen: the "N commits come in, clean or stops on N files" lines need a worktree behind the trunk (not made). |
| 106.5 | In the graph, select a commit and press `m`, then `C` | Both bands carry a forecast line; the cherry-pick one says a copy is added and the original stays | [ ] (飛ばし: 106.4 と同じ用意の上に、グラフで m と C を続けて押す確認が要る。未実施) |
| 106.6 | Switch branch from the palette with uncommitted files | A band says the files change from A to B and that the uncommitted files are carried over; `y` switches, Esc leaves things as they are | [ ] (飛ばし: パレットに文字を打つ操作がハーネスに無い) |
| 106.7 | Switch with no uncommitted files and no Claude | No band, as before | [ ] (飛ばし: パレットに文字を打つ操作がハーネスに無い) |
| 106.8 | Do 106.1 to 106.6 in EN and in JA | The sentences read well in both and fit the band without being cut | [ ] [~] harness: EN and JA (`<Lang:ja>`) of the 106.1 and 106.4 bands read well and fit at 1100x600; 106.3 only in EN. |

## 107. Where you are now (#107)

| # | Do | Expect |
| --- | --- | --- |
| 107.1 | Open a repository on a branch | A quiet line under the title bar reads "You are on X." with the uncommitted and remote state, and the key to press next | [ ] (見た: タイトルバーの下に「いまは master にいます。コミットしていない変更はありません。このブランチはまだリモートにありません。P で push できます。」) |
| 107.2 | Edit a file, then watch the line | It gains "N files have uncommitted changes. Press 2 to commit them." within a couple of seconds | [ ] (飛ばし: 作業中に変更を足して数秒後の画面を撮る手順がハーネスに無い) (見た: 変更ありの画面で「コミットしていない変更が 2 ファイルあります。2 を押すとコミットできます。」の行を確認。編集の数秒後に増える動きは撮れていない) |
| 107.3 | Commit, then push | The line moves to "not pushed ... Press P", then to "It matches origin/X" | [ ] (飛ばし: コミットと push の連続操作が要る) |
| 107.4 | Switch branches; start a merge that conflicts | The line follows the branch; during the merge it says so and points to `4` | [ ] [~] harness 状態の行がブランチに付いて変わる (main →「You are on main」、feat →「You are on feat」)。マージの衝突中は「You are in the middle of a merge on main. Press 4 …」と 4 を指す |
| 107.5 | Narrow the window | The line stays on one row and ends with an ellipsis; resting the pointer on it shows the whole sentence | [~] (2026-10-22 ran at 480x600: the line stays on one row and ends with an ellipsis (EN and JA); the hover tooltip with the whole sentence cannot be driven by the harness) |
| 107.6 | Check a detached HEAD | The strip is shown as before and the line is not doubled up under it | [ ] (飛ばし: detached HEAD の用意が要る) |
| 107.7 | Open settings; switch EN and JA | The line is hidden in settings; in both languages it reads well and the colours follow the theme | [~] (2026-10-22 ran light theme, lab nw9: EN line reads well under the title bar in the graph; in JA settings the line is hidden; colours follow the light theme; stills only. Seen: a no-remote toast raised at start stayed Japanese after switching to EN) |

## 113. Calm save (#113)

| # | Do | Expect |
| --- | --- | --- |
| 113.1 | In a worktree with a staged change, an unstaged change and a new file, press "Save state" in the keybar (or "Calm down" in the palette) | A toast names `michi/backup/<time>` and a `git restore --source=… --worktree -- .` line; the status, the index and HEAD are exactly as before | [ ] [~] harness staged・unstaged・新規ファイルのある pl1a で「Save state」を押す。toast「Kept all of pl1a as michi/backup/<time>; nothing was changed. To get the files back: git restore --source=michi/backup/<time> --worktree -- .」が出る。status と HEAD は押す前と同じ (refs/michi/backup/ができた) |
| 113.2 | Run the `git restore` line from the toast after editing a file | The file is back as it was when saved | [ ] [~] harness 113.1 の restore の行を、f に 1 行足した後で手で回すと、f は保存時の中身に戻る (git の autocrlf のため改行だけ CRLF) ・ status は同じ |
| 113.3 | Press it in a clean worktree | A toast says there are no changes and HEAD is kept | [ ] [~] harness 変更の無い worktree (pl2a) で押すと toast「pl2a has no changes; its HEAD is kept as michi/backup/<time>」が出る (status 変化なし) |
| 113.4 | Press it in the middle of a merge with a clash | It saves, and the merge is still under way with the same places to solve | [ ] [~] harness 衝突中のマージ (cf1) で押す。toast で保存され、マージは続いたまま (status は UU のまま・未解決の index の段 3 つは同じ) |
| 113.5 | Hover the button; switch EN and JA | The tooltip says nothing is changed; both languages read well and the button fits the keybar at a narrow window | [ ] (飛ばし: hover の tooltip と EN/JA の切り替えがハーネスに無い) |

## 119. Next thing to do (#119)

| # | Do | Expect |
| --- | --- | --- |
| 119.1 | Edit a file | A "Next: commit" button appears in the keybar; pressing it goes to the `2` view | [ ] [~] harness 0.106.11 (lab nx1): a changed file puts 'Next: commit' in the keybar; a click on it opens the 2 view |
| 119.2 | Commit it, with an upstream | "Next: push" appears; pressing it pushes | [ ] [~] harness 上流のある main で 1 コミット先に進めると「Next: push」が出る。押して push する所は未確認 |
| 119.3 | Make the upstream have a commit you lack | "Next: pull" appears (before push); pressing it pulls | [ ] [~] harness 上流が先の pl4 で「Next: pull」を押すと pull され、main が origin/main と同じになる (push との順は未確認) |
| 119.4 | On a new branch with commits and no upstream | "Next: push the branch" appears; the trunk never shows it | [ ] [~] harness 上流の無い新しいブランチ feat (コミットあり) で「Next: push the branch」が出る。trunk では出ない所は未確認 |
| 119.5 | Start a merge with a clash, then solve all places | "Next: solve the clashes" opens the `4` view; after solving it reads "Next: finish the merge" | [ ] [~] harness 衝突中のマージで「Next: solve the clashes」が出る (cf1)。押して 4 が開く所と、解いた後の「finish the merge」は未確認 |
| 119.6 | Clean and in step | No suggestion button is shown; the palette has no "next step" entry | [ ] [~] harness きれいで同期済みの pl2a では提案のボタンが出ない。パレットに「次の手」の項目が無い所は未確認 (文字入力が要る) |

## 262. Beginners' explanations: stage, conflict, worktree (#120, #121, #122)

| # | Do | Expect |
| --- | --- | --- |
| 262.1 | Open the `2` view with changed files and nothing staged | A band under the top bar shows three steps (changed, put in the next commit, commit) with the first lit, and a line saying where the files are |
| 262.2 | Stage a file with `Space`; then write a commit message | The second step lights up after staging, the third once the message is written |
| 262.3 | Click the cross on the band; restart michi and open the `2` view | The band is gone and a small "? ..." link takes its place; it stays gone after the restart |
| 262.4 | Click the "? ..." link | The band comes back |
| 262.5 | Start a merge that conflicts and open the conflict view | A card explains what a conflict is, with a pager `1/3`; `‹` `›` go to the cards on ours and theirs and on how to go on |
| 262.6 | Dismiss the conflict card, restart, open a conflict again; then click its link | Stays dismissed across the restart; the link brings the cards back |
| 262.7 | Open the `3` view | A four-line card says what a worktree is and how it relates to Claude working in parallel; the table sits below it with nothing hidden |
| 262.8 | Dismiss it, then run "Show the explanations again" from the command palette | A toast says they are back; all three explanations show again |
| 262.9 | Narrow the window; switch EN and JA | The cards wrap without overlapping the cross or pager; both languages read well (skipped: needs a narrower window, which the harness lacks) |

## 89. HiDPI and the Windows 11 window frame (#89)

Run each row at 100, 150 and 200 % display scale (Settings > System > Display), and once on a 4K monitor.

| # | Do | Expect |
| --- | --- | --- |
| 89.1 | Start michi at 100, 150 and 200 % scale; look at the title bar, tabs, keybar and the graph | Text and the logo are sharp at every scale; nothing is cut off, blurred or overlapping; the row height and the graph lanes grow with the scale | | [ ] (飛ばし: OS の DPI・スナップ・テーマの切り替えが要る実機の機能) |
| 89.2 | Move the window between a 100 % monitor and a 150 / 200 % one | The window re-lays out at once without a restart; no blur and no wrong-sized strip | | [ ] (飛ばし: OS の DPI・スナップ・テーマの切り替えが要る実機の機能) |
| 89.3 | Snap the window to the left, right and a corner (Win+arrows, the snap layouts on the maximise button); maximise and restore | The window fills the snapped area exactly; the title bar buttons stay clickable; no gap at the edges; the size is restored | | [ ] (飛ばし: OS の DPI・スナップ・テーマの切り替えが要る実機の機能) |
| 89.4 | Switch Windows between dark and light mode while michi runs | The window edge and the title bar keep michi's theme; no white flash or light border in dark mode | | [ ] (飛ばし: OS の DPI・スナップ・テーマの切り替えが要る実機の機能) |
| 89.5 | Look at the window corners on Windows 11 (maximised and not) | The corners are rounded when not maximised and square when maximised; no dark triangle or stripe at a corner | | [ ] (飛ばし: OS の DPI・スナップ・テーマの切り替えが要る実機の機能) |
| 89.6 | Drag the window by the title bar and resize from each edge and corner | The resize cursors show on the edges; the frame follows the mouse without a lag or a flicker | | [ ] (skipped: needs drag or wheel, which the harness lacks) |
| 89.7 | Open the palette, the branch box, a confirm window and a toast at 200 % | They are placed inside the window, sharp, and not larger than the window | | [ ] (飛ばし: OS の DPI・スナップ・テーマの切り替えが要る実機の機能) |

## 265. Undo and Redo of a branch's move (#64)

| # | Do | Expect |
| --- | --- | --- |
| 265.1 | In the graph reset a worktree to an older commit (any mode) and press Ctrl+Z | The branch is back on the commit it was; a toast says `Undid the reset of ...`; the graph and the table follow |
| 265.2 | Press Ctrl+Shift+Z | The reset is done again; a toast says so; a new reset or delete after an Undo ends the Redo line (Ctrl+Shift+Z says there is nothing to do again) |
| 265.3 | Delete a branch (`d` in the branch palette, yes), press Ctrl+Z, then Ctrl+Shift+Z | The branch is back at the same commit, then gone again |
| 265.4 | Rename a branch, press Ctrl+Z | The old name is back |
| 265.5 | Reset hard, then edit a file, then Ctrl+Z | A toast refuses: uncommitted changes; nothing moves. Commit on the branch and Ctrl+Z again: it refuses, the branch has moved on |
| 265.6 | Click into the commit message box, type, press Ctrl+Z | The typing is undone in the box; no branch moves |
| 265.7 | Press Ctrl+Z with nothing done in this window | A toast: `Nothing to undo` |
| 265.8 | Open `?` and the Keys page of the settings | Undo (Ctrl+Z) and Redo (Ctrl+Shift+Z) are listed; rebinding them works; JA reads `元に戻す` and `やり直す` |

## 266. Cmd on a Mac (#61)

| # | Do | Expect |
| --- | --- | --- |
| 266.1 | On macOS press `?` | The keys read `Cmd+...` (not `Ctrl+...`) in the global list, the keybar and the palette field's key cap; `Ctrl+Tab` stays |
| 266.2 | On macOS press Cmd+Enter in the `2` view with a message and a file staged, then Cmd+Z after a reset | The commit is made; the reset is taken back |
| 266.3 | On Windows or Linux press `?` | The keys still read `Ctrl+...` |

## 267. Other operating systems (#62)

Every row of this file is meant to be pressed on macOS and Linux too, unless it names Windows (the title bar's buttons are michi's own on all three).
`cargo run -p michi --example make-testcheck -- --lane macos` and `-- --lane linux` write `TESTING-MACOS.md` and `TESTING-LINUX.md`, one box per row: `[x]` checked there, `[-]` does not apply there.

| # | Do | Expect |
| --- | --- | --- |
| 267.1 | Start michi on macOS and on Linux in a repository | The window opens with the tabs, the chip and the graph; the title reads `<repo> — michi` |
| 267.2 | Resize, maximise and drag the window by its bar | The same as 1.4 to 1.7 (skipped: needs drag or wheel, which the harness lacks) |
| 267.3 | Run `michi --version` and `michi --help` from a terminal | Text on the terminal, no window |
| 267.4 | Open a folder or a worktree with a Japanese name | The Japanese draws, not boxes |

## 268. Accessibility (#60)

Text size and contrast come from ito: the zoom keys (Ctrl+=, Ctrl+-, Ctrl+0) and the themes of the settings (the contrast of each built-in theme is held by ito-theme's tests). A screen reader and a high-contrast theme are not michi's alone: see the crate request in the issue.

| # | Do | Expect |
| --- | --- | --- |
| 268.1 | Press Ctrl+= five times, then Ctrl+0 | The whole window grows with each press (text, rows, key caps) and a toast names the size; Ctrl+0 is back at 100%; nothing overlaps at the largest size |
| 268.2 | Choose each built-in theme in the settings and look at the graph, the table, the changes and a diff | The text reads on its ground; the selected row, a focus ring and a conflict mark are told apart without colour alone (a symbol or a word comes with the colour) |
| 268.3 | Use only the keyboard from start to a commit (`2`, `j`, Space, `c`, type, Ctrl+Enter) | Every step works without the pointer; the focus is always visible |

## 269. Bundled monospace font (#85)

| # | Do | Expect |
| --- | --- | --- |
| 269.1 | Open the settings, Appearance; look at the monospace font card and switch bundled / system | Hashes and paths redraw in JetBrains Mono for bundled and in the system face for system, without a restart; common.toml gets `mono_font` |
| 269.2 | Open "Licence of the bundled font" under the card | The authors and the SIL Open Font License text are shown |

## 270. Date style of the lists

| # | Do | Expect |
| --- | --- | --- |
| 270.1 | In the settings, General, choose "Full date and time" under Dates | The graph, the worktree table, the detail pane and the conflict list write every date as `YYYY/MM/DD hh:mm` at once; config.toml has `[view] dates = "full"` |
| 270.2 | Change the clock's date format (e.g. DD/MM/YYYY) and 12-hour time, then go back to "Short" | The full form follows the clock's format; Short shows today's time, "yesterday" and so on again |

## 271. Multiple selection in Changes, message box

| # | Do | Expect |
| --- | --- | --- |
| 271.1 | In Changes press Shift+Down twice on an unstaged file, then Space | The three rows are shaded alike; Space stages all three at once; Shift+Up narrows the range; Esc drops it; it does not cross into the staged list |
| 271.2 | Type twenty lines in the commit message box | The text stays inside the box and scrolls there; the guide line and the commit row are not covered |

## 272. Graph: scroll the wide detail, copy the hash

| # | Do | Expect |
| --- | --- | --- |
| 272.1 | In the graph select a commit with a long diff, press Enter, then Down, PageDown, End, Home | The detail scrolls by line, page, to the end and to the top; the graph selection stays; Enter narrows it and the arrows move the graph again |
| 272.2 | Select a commit and press `#`, then paste elsewhere; also Ctrl+C with no text picked | The full 40-character hash is pasted; a toast names its first 10 characters |
| 272.3 | In the Worktrees view press `E` on a worktree | Explorer opens that worktree's folder, not the user folder (skipped: needs OS or machine features the harness lacks) |
| 272.4 | Press F1 in a wide window, then in a narrow one; scroll with the wheel and the arrow keys | Two columns when wide, one when narrow; both scroll when the list is taller than the card (seen [~] in the harness, EN: 1400x600 shows Everywhere and Graph side by side and the wheel scrolled both together; 700x600 shows one column and the wheel scrolled it; the arrow keys not driven) |
| 272.5 | In the graph, the Changes view and the F1 list press Ctrl+D, Ctrl+U, Alt+J, Alt+K | They move or scroll a page down and up like PageDown / PageUp; the key lists show them |

## 273. Keys: gg / G, Alt+digit, toast

| # | Do | Expect |
| --- | --- | --- |
| 273.1 | In the graph and in the Worktrees table press `G`, then `gg` | `G` goes to the last row, `gg` to the first; a `g` followed by another key does nothing |
| 273.2 | Press Alt+2 in the graph | The second repository tab opens; the view does not change |
| 273.3 | In the graph trigger a message (`#`) | The toast sits at the top right (see 274) |
| 273.4 | Press `U` in the graph and in the Worktrees table; `R` in the table; `x` then `y` in the graph | Gitflow list; the pre-merge check; the bisect marks bad then good |
| 273.5 | In the graph select a merge commit, press `]`, then `[` | `]` selects the commit the merge brought in, `[` the first parent; on a plain commit `]` says there is no second parent |
| 273.6 | Select a worktree's uncommitted change and drag over its header in the detail | The text is picked up and copied [~] harness 0.106.11 (1100x600, lab m1): after selecting the WIP row, dragging across the detail header "Uncommitted changes on wt3" highlights the text; the clipboard content is not readable by the harness, so the copy is not judged |

## 274. Toast: top right, gone after a few seconds

| # | Do | Expect |
|---|---|---|
| 274.1 | Trigger a message (`#` in the graph) and wait | It appears at the top right, not at the centre, and is gone after about 4 seconds without touching anything |
| 274.2 | In Changes click a file, then Shift+click another on the same side; then Space | Every file between is shaded alike and staged together; a Shift+click on the other side only selects |
| 274.3 | Click a file, Ctrl+click two others (not neighbours), then Ctrl+click one of them again | The Ctrl+clicked files are shaded; the one clicked twice leaves; Space stages the rest; a plain click or Esc drops the marks |
| 274.4 | In the graph detail pane drag across a line with full-width (Japanese) text | Whole characters are picked, never half of one [~] harness 0.106.11 (1100x600, lab jp1): dragging over the Japanese line in the graph detail lights whole characters (日本); the clipboard is not readable |

## 275. Keys and layout: Alt+Left/Right, O, Ctrl+, , dates, message box

| # | Do | Expect |
|---|---|---|
| 275.1 | In `2` select a file and press Alt+Right, then Alt+Left | The keys go to the diff's lines, then back to the files |
| 275.2 | In `3` press `O` on a worktree whose branch is on GitHub; press `p` | `O` opens the pull request page; `p` pulls (skipped: needs gh / GitHub / minato, which tests must not touch) |
| 275.3 | Press `,` anywhere in the main screen; press Ctrl+, | The comma does nothing; Ctrl+, opens and closes the settings |
| 275.4 | With `[view] dates` unwritten, look at the graph's date column | `YYYY/MM/DD hh:mm`, as the settings' clock format at the bottom right; `dates = "relative"` brings the short form back |
| 275.5 | Open `2` | The commit message box is under the diff on the right, not under the file list; the file list runs to the bottom on the left |
| 275.6 | With two repository tabs open, press Ctrl+, then Ctrl+Tab | The tab bar is gone while the settings are open; Ctrl+Tab steps the settings' pages, not the repositories; Ctrl+, closes the settings and the bar returns |

## 270. Git flow sheet

| # | Do | Expect |
| --- | --- | --- |
| 270.1 | Press Ctrl+Shift+G (or palette: "Git flow sheet") | A sheet opens with seven tabs; the first (Basics) shows a flowchart, its commands and the key column (`—`) |
| 270.2 | Press `2`…`7`, `h`, `l`, Tab / Shift+Tab, or click a tab | The sheet changes; every arrow joins its boxes without crossing a box; nothing is clipped on a narrow window (scroll works) |
| 270.3 | Press `j` / `k`, or click a box | The box is outlined and its commands in the table are bright; the numbers on the boxes match the table |
| 270.4 | Click a command | A toast "Copied: …" shows and the command is on the clipboard |
| 270.5 | Look at `reset --hard`, `push --force`, `clean -fd` | They are red, with a hover warning |
| 270.6 | Esc, `q`, or click outside | It closes; the palette does not open over it |
| 270.7 | Switch EN and JA, light and dark | Words fit in their boxes (Japanese may wrap to three lines); colours stay readable |
| 270.8 | Look at the layout | The flow is on the left and the commands on the right, side by side; the columns (command, what, screen, key) do not squash the text |

## 276. Keys of the confirm band

| # | Do | Expect |
| --- | --- | --- |
| 276.1 | In a worktree, merge another branch in (`m`) so the band asks | The band's last line reads `Ctrl+Enter Go ahead · Esc Not now`; the buttons are unchanged |
| 276.2 | Press Ctrl+Enter | The merge goes ahead, as with a click on the button; Esc instead drops the band and nothing changes |
| 276.3 | Open a band that asks for a typed name (the strongest step) and press Ctrl+Enter | Nothing goes ahead; Enter still needs the name typed first |

## 277. Branches already merged into another worktree

| # | Do | Expect |
| --- | --- | --- |
| 277.1 | In the practice lab, merge lab-a and lab-c into topic-a, then look at the worktree table | The lab-a and lab-c rows say `topic-a に取り込み済み`; their land-order line is gone and the rest of the order counts without them |
| 277.2 | Look at those rows' buttons while they have nothing uncommitted | `消す x` comes first, and the note adds that it can be removed; with an uncommitted change the note has no `消せます` and `消す` stays last |
| 277.3 | Press the reread button after another merge | The note appears without restarting |
| 277.4 | Look at the clash table with lab-a and lab-c merged into topic-a | Their rows, columns and numbers are faint; the hint under the table says so |
| 277.5 | Merge a branch into another worktree from another editor or a terminal while michi is in front | The note and the order update by themselves within the poll time (up to 30 s where the folder cannot be watched); the reread button updates at once |

## 278. Overlay order, repository folder hint, parked tabs, one bad setting

| # | Do | Expect |
| --- | --- | --- |
| 278.1 | Open a band (e.g. `x` on a worktree), then press `?` | The cheat sheet does not open over the band; the band stays |
| 278.2 | Open the cheat sheet first, then trigger a band by mouse and press Enter / Ctrl+Enter | The cheat sheet takes the key; the band is not answered |
| 278.3 | In a Graph view press `y` then `d` | The prefix hint says "Repository folder" (ja: リポジトリのフォルダ); in the Worktree view it still says "Worktree folder" |
| 278.4 | Open two tabs, start a fetch in the first, switch to the second | The first tab's result and toast arrive while it is parked |
| 278.5 | Put `density = 3` (a number) in `[view]` of config.toml and start michi | Theme, language and the other `[view]` keys keep their values; only density falls back to the default |
| 278.6 | In a conflict with text places, look at a file head | An "Open in a tool" button sits left of "Restart"; clicking it opens the merge tool (the same as `w`), WinMerge with three panes when `merge` is set as in the settings screen |
| 278.7 | Set `diff_tool = "winmerge"` in `[open]`, start michi, and press `w` in a conflict without opening the settings screen first | WinMerge opens with three panes (not Explorer) (skipped: needs OS or machine features the harness lacks) |
| 278.8 | Double-click a local branch in the graph's left column (clean tree; then with uncommitted files; then mid-merge) | The worktree switches to it as `b` does; with uncommitted files a confirmation band comes first; mid-merge it is refused with the usual message. A branch checked out elsewhere is only said. A single click still selects its commit |

## 279. Hide / solo branches, stop a clone

| # | Do | Expect |
| --- | --- | --- |
| 279.1 | Right-click a local branch in the graph's left column and choose "Hide from graph" | The row is struck through with a hollow swatch; its commits leave the graph unless reachable from a shown branch; a "◐" row appears at the top |
| 279.2 | Right-click a branch and choose "Show only this" | Only that branch's history stays in the graph; clicking the "◐" row (or `X` in the left column) shows every branch again |
| 279.3 | In the left column, on a branch row press `x`, then `o`, then `X` | `x` hides / shows it, `o` shows only it / all again, `X` clears; on a row with no ref a toast says to pick a branch |
| 279.4 | Start a clone of a big repository from `<C-o>` | The keybar shows "clone · Receiving objects NN%" with "Esc / click stops" |
| 279.5 | Press Esc during the clone (then again with another clone, click the keybar status) | The clone stops, the half-made folder is gone, and a toast says so; the window stays on the old repository |
| 279.6 | Press `F12` (or `p` on a worktree with a pushed branch) | The default browser opens the issue form / PR page with its `?query` intact; no Explorer window opens (#131) | [x] 2026-10-24: ran `rundll32 url.dll,FileProtocolHandler` with a `?template=…&context=a` URL; the browser opened and the Explorer window count stayed 3 |
| 279.7 | Press `Ctrl+O`, then `Ctrl+O` again while the list is open (also `Ctrl+T`) | The second press closes the repository list; other keys (`j`) still go to its search box (#130) | [x] 2026-10-25: debug build 0.108.4 launched with scripts/shot.ps1; the first Ctrl+O showed the repository list, the second Ctrl+O closed it (screenshots repos-b / repos-c); also unit-tested (`only_the_repos_key_reaches_the_keymap_while_the_list_is_open`) |
