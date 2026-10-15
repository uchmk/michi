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
| 2.1 | Press `1`, `2`, `3` | The tab in force moves and opens into the view; the graph shows the history (section 6), the worktrees show their table (section 10), the changes say what will be there and "Coming in a later stage" |
| 2.2 | Click each tab | Same as the digit |
| 2.3 | Press `?` | A card with the keys: "Everywhere" first, then the view's; the tab stays lit under it; `?` or Esc closes it |
| 2.4 | Press `N`, `f`, `p`, `P` in any view, `b` and `s` in the graph (Enter spreads the diff: 8.5), Space and `c` in the changes | A toast at the foot names the key's work and says it comes in a later stage; it goes after 4 seconds |
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
| 6.9 | Click a row; scroll with the wheel | The row is selected; the wheel scrolls the list without moving the selection |
| 6.10 | Press `/` and type part of a subject, a hash or an author's name | The field at the right of the tool band takes the keys, with the count (`3 hits`) at its right; the text sits in the middle of the field; the other rows dim; the hits get marks on the scrollbar |
| 6.11 | In the field, press Enter, Shift+Enter, ↓, ↑ | Enter selects the next hit (round to the first after the last), Shift+Enter the one before; the arrows move the selection; the field keeps the keys |
| 6.12 | Press Esc in the field | The filter clears and every row is bright again |
| 6.13 | Look at the scrollbar | Marks for the worktrees' heads (in their lanes' colours), the tags (amber) and the selected row (the accent); hovering a mark tells what it is |
| 6.14 | Drag the thumb; click the track; click a mark | The thumb stays under the pointer to the release; a click on the track brings that part into the middle; a click on a mark selects its row |
| 6.15 | Look at commits by dependabot[bot] and github-actions[bot] | The author reads `⚙ dependabot` and `⚙ actions`; hovering shows the full name |
| 6.16 | Open a repository with more than 2000 commits | The first screen comes at once; scrolling near the foot reads on by itself; the last row says how many older commits are left and reads them when clicked; the thumb shrinks as the list grows |
| 6.17 | Look at a second worktree's commits | They run on their own lane in the worktree's colour, from where they left main |
| 6.18 | Narrow the window to 720 wide; switch to the light theme | The subject is cut with an ellipsis and the names drop before they crowd it; the filter field stays inside the band; every colour stays readable in light |
| 6.19 | Open a repository with more than 2000 commits and no `.git\objects\info\commit-graph*` (a fresh `git clone` of a long history), wait a few seconds, close michi and open it again | `.git\objects\info\commit-graphs` appears while michi is open, with no toast; the history looks the same, and the second opening shows the first screen sooner. A repository with fewer commits gets none |

## 7. Side column

The same repository as section 6.

| # | Do | Expect |
| --- | --- | --- |
| 7.1 | Look at the column left of the graph | Headings with counts: worktree, Branches, Remotes, Tags; the worktree the window is on is lit; each worktree and branch has a dot in its lane's colour; there is no scrollbar, the wheel scrolls it |
| 7.2 | Click a worktree, a branch and a tag | The graph selects that commit and scrolls it into sight; the detail follows |
| 7.3 | Click a group (`test/`, `origin`) | It opens to list its branches further in, with the count at its right; again, it closes; hovering a branch inside shows its full name |
| 7.4 | Click a branch whose commit is older than what is read | The graph reads on until it gets there and selects it; one that is not in the history gives the toast `Not in the history: <hash>` |

## 8. Detail and diff

| # | Do | Expect |
| --- | --- | --- |
| 8.1 | Select a commit | Below the graph within a moment: the subject in bold, the full hash, the author, the local time and the parents (blue links), the body, the hint `Enter spreads the diff side by side`; `Reading the commit…` shows while it is read |
| 8.2 | Look at the file list | Each file with its status letter in colour (A green, M, D red, R), its path cut from the left with `…` when long, `+a −d` and a bar; a renamed file says where it came from; more than 12 files list 10 and `N more files — click for all` |
| 8.3 | Click a file row | The detail scrolls to that file's diff |
| 8.4 | Look at the diff | One column: the `@@` rows in blue, added rows green and deleted rows red, the changed words marked a shade deeper, the line numbers at the left; only the detail's wheel scrolls it, with no scrollbar |
| 8.5 | Press Enter, then Esc | Enter narrows the graph and spreads the diff side by side (old left, new right, the empty side hatched); the title's toggle follows; Esc brings the graph back, and a second Esc clears the filter |
| 8.6 | Click a parent link | The graph selects the parent and the detail shows it |
| 8.7 | Select a merge into main | `N commits came into main by this merge` with their hashes, subjects and times, and whether they are folded or open in the graph; the note that a merge's diff comes in the next stage; no diff |
| 8.8 | Select a commit with a binary file, and a big one | The binary file says it has no diff to show; a very long diff ends with `… the rest is left out` |
| 8.9 | Narrow the window below 900 wide | The side column goes and the detail stacks under the graph, still readable; Enter still spreads the diff |
| 8.10 | Switch to the light theme | The diff's greens, reds and blues and the hatching stay readable |

## 9. Uncommitted changes

Open a repository whose worktree has edits, a deleted file and a new file; run a Claude in it from tsumugi.

| # | Do | Expect |
| --- | --- | --- |
| 9.1 | Look at the top of the graph | A row `Uncommitted: N files (+a −d)` in italics above the newest commit, with a dashed dot on the lane of the worktree's branch, author `—`, the time the change was last written and the hash `WIP`; the main worktree's change is on top when it was written last |
| 9.2 | Edit a file in the worktree with michi in front | Within about a second the row's counts and time follow without a key; the selection stays on the row it was on |
| 9.3 | Select the WIP row | The detail says `Uncommitted changes on <branch>`, the worktree's folder in mono, when it was last written, `Read only here. Stage and commit in Changes (2).`, then the files and the diff from the worktree's `HEAD`; the new file is listed as added |
| 9.4 | With the WIP row selected, have the Claude there run, then stop or ask | A box under the hint: `Claude is writing.` or `Claude has stopped.` with its state (glyph and word), the tab, its title and since when; a question in amber |
| 9.5 | Put a file over 10 MB (or a folder over 2000 files or 50 MB) in the worktree, not ignored | The WIP row and its counts leave it out; the detail says `Untracked, too big to count: <path>` (three at most, then `and N more`); `git count-objects -vH` in the repository does not grow by its size; no `git.exe` keeps running after michi is closed |
| 9.6 | Make the worktree clean again | The WIP row goes within about a second; a worktree whose only change is a big untracked folder has no WIP row |
| 9.7 | Look at `git status` and the index after michi read the change | Nothing was staged; `git status` shows the same as before |

## 10. Worktree table

Open a repository with a main worktree, a worktree on a branch that is pushed and has one more commit plus edits, and a locked worktree with no branch (`git worktree add --detach` then `git worktree lock`); give the repository an `origin`. Run a Claude in one of them from tsumugi for 10.2.

| # | Do | Expect |
| --- | --- | --- |
| 10.1 | Press `3` | `Reading the worktrees…`, then a title `<repo> · N worktrees` with `Read again` and `New work` (`N`) at the right, and a table: `Branch · folder`, `From main`, `Uncommitted`, `Unseen`, `Conflicts ahead`, and buttons; the main worktree on top, each row with its branch in its lane colour and its folder under it |
| 10.2 | Run a Claude in a worktree from tsumugi, then quit tsumugi | The title adds ` · Claude N` and a `Claude (tsumugi)` column shows the state chip in bold, its note (two rows at most), `Tab N · <title>` with `o` and since when; rows with none say `No Claude running`; within a few seconds of quitting tsumugi the column goes |
| 10.3 | Look at the `From main` column | `↑a ↓b` against main, and under it the branch against its upstream: `Same as origin/x`, `N not pushed to origin/x`, `origin/x is N ahead`, `Diverged from origin/x (↑a ↓b)`, `Not pushed yet`; the main row reads against `origin/main` |
| 10.4 | Look at the `Uncommitted` column; then put a folder over 2000 files in a worktree, not ignored | `N files`, `+a −d` in green and red and `written at <time>`, or `None`; the big folder adds `Too big to count: <path>` and is not counted |
| 10.5 | Look at the locked worktree with no branch | `No branch, at <hash>` and a `Locked` mark; a worktree whose folder was deleted says `Its folder is gone (git can prune it)` |
| 10.6 | Press `j` / `k`, the arrows, PageDown / PageUp, Home / End, and click a row | The selection moves (10 rows a page) and the table scrolls to keep it in view; the wheel scrolls the table with no scrollbar |
| 10.7 | Press Enter on a row with changes, and double-click a clean row | The graph opens with the row's `Uncommitted` row selected, or its branch's commit |
| 10.8 | Press `d`, `m`, `x`, `o`, `p` and click `PR` | A toast at the foot names the work and says it comes in a later stage; `New work` gives the same toast as `N`; the main row shows `Pull` and `Show in the graph` instead, and has no `Remove` |
| 10.9 | Click `Read again` | The toast `Reading the worktrees again…`, and the table follows a change made outside (a new commit, a new worktree) |
| 10.10 | Narrow the window below 960 wide | Each worktree becomes a card: the branch and folder, then the Claude, `From main`, `Uncommitted`, `Unseen` and `Conflicts ahead` lines, then the buttons, which wrap; still readable at the smallest width |
| 10.11 | Look at the foot, and switch to the light theme | Only when the Claude column shows, a line on how often it reads tsumugi; colours stay readable in both themes |

## 11. Conflict prediction

Set up: two worktrees that change the same lines of a file, each committed; a third whose change to that file is not committed yet.

| # | Do | Expect |
| --- | --- | --- |
| 11.1 | Press `1` and look at the two worktrees' head commits | Each has a red `⚠ Conflict ahead` pill after its names; hovering says `Would conflict with <branch> in N places (<files>)`, one line per worktree it clashes with |
| 11.2 | Look at the third worktree's `Uncommitted` row | A dashed yellow `⚠ Conflict ahead` pill after the counts; hovering says `Counting the uncommitted changes, would conflict with <branch> in N places (<files>)` |
| 11.3 | Press `3` and look at the `Conflicts ahead` column | Red `⚠ <branch>: N places` with the files under it, one for each worktree it clashes with; yellow with `(uncommitted)` for the third; `None` for a worktree that clashes with nothing |
| 11.4 | Look at the box under the table | `Conflicts ahead (uncommitted changes counted)`, a table of the worktrees' short names both ways: `—` against itself, `·` for a clean pair, the number of places in red, or in yellow with a dashed frame for the uncommitted one; the hint under it and the foot about `git merge-tree` |
| 11.5 | Hover a number, then click it, then click it again | The tip lists each file with its places; the click frames the cell and the note reads `<a> × <b>: <file> N places. ` with what to expect when they go into main (or, for the yellow one, that committing turns the frame solid); the second click puts the hint back |
| 11.6 | Commit the third worktree's change; then undo the conflicting edit in one of the first two and commit | Within about a second with michi in front, the yellow pill and cell turn red; then the red ones go, and the column says `None` |
| 11.7 | Narrow the window below 960 wide, then to 1000 wide; switch to the light theme | As cards, each has a `Conflicts ahead` line and the box takes the full width; at 1000 wide the table's columns shrink but nothing overlaps; red and yellow stay readable in light |
| 11.8 | Open a repository with only one worktree, and look at `git status`, the branches and `git count-objects -vH` after a minute | The box says `Only one worktree: nothing to pair yet.`; nothing was staged, no branch moved, and the objects do not grow by more than the temporary trees; no `git.exe` keeps running after michi is closed |

## 12. Watching and unseen commits

Set up: the repository of section 10, with michi in front on the graph; commit from a terminal in the worktree on a branch. Where michi keeps what was seen: `%LOCALAPPDATA%\uchmk\michi\seen\<repo>-<hash>.json`.

| # | Do | Expect |
| --- | --- | --- |
| 12.1 | Commit in the worktree on a branch | Within about a second the history is read again without going blank: the new commit shows on its branch with a blue dot before its subject, which is bold; the selection stays on the commit it was on |
| 12.2 | Press `3` | The worktree's `Unseen` column says `● 1 new` (the blue dot); the main worktree says `—` and the others `None`; beside the conflicts box, `Unseen changes (1)` lists the commit: short hash, blue dot, the subject in bold, `<short branch> · <time>` at the right, and the foot `The commits the worktrees have gained since you last looked. Selecting one in the graph marks it seen.` |
| 12.3 | Hover the commit in the list, then click it | The line lights and the pointer becomes a hand; the click opens the graph with that commit selected and its dot and bold go; back in `3`, the column says `None` and the box `All seen.` |
| 12.4 | Commit twice more; in the graph, move onto one of them with `j` / `k`, and click the other | Each loses its dot once selected; the commit selected when the graph first opens, or that the selection follows when rows come in, does not count as seen |
| 12.5 | Make 13 commits in the worktree and press `3` | The box shows 12, newest first, and `… 1 more`; a long subject is cut with `…` before the branch and time |
| 12.6 | Quit michi with commits unseen, commit once more, and start michi again | The commits still unseen keep their dot, and the one made while michi was closed has one too; the file above lists the branch with the commits seen |
| 12.7 | Narrow the window below 960 wide | The `Unseen changes` box goes under the conflicts box at the full width; the cards have an `Unseen` line |
| 12.8 | Put another window in front, commit in a worktree, wait, then bring michi back | Nothing is read while michi is behind (no `git.exe` in Task Manager); when it comes back the history and the table follow at once |
| 12.9 | Run `cargo build` (or write many files) in an ignored folder of a worktree, and run `git status` there, with michi in front | michi reads nothing for them: no `git.exe` keeps coming up in Task Manager, and the table does not change; a `git status` does not start a read that starts another |
| 12.10 | Open a repository on a network share (`\\server\share` or a mapped drive) | Changes there show within about 5 s while michi is in front; nothing is read while it is behind |
| 12.11 | Switch to the light theme | The blue dots and bold subjects in the graph, the column and the box stay readable |
