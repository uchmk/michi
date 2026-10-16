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
| 2.1 | Press `1`, `2`, `3` | The tab in force moves and opens into the view; the graph shows the history (section 6), the worktrees show their table (section 10), the changes show the files to stage (section 14) |
| 2.2 | Click each tab | Same as the digit |
| 2.3 | Press `?` in `1`, `2` and `3`; narrow the window below 460 wide and press `?` | A card with the keys: "Everywhere" first, then the view's, two to a line with the word at the left and the keys at the right, every key there down to `Esc`; the tab stays lit under it; `?` or Esc closes it; the narrow window has one to a line |
| 2.4 | Press `N` in any view (Enter spreads the diff: 8.5; `f` `p` `P`: 15; `b` `T` `t`: 16; `s`: 17) | A toast at the foot names the key's work and says it comes in a later stage; it goes after 4 seconds |
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
| 6.20 | In a history of 100,000 commits, press `z` to fold and unfold, and type in the filter (`/`) | Each answers at once, with no stall while typing; scrolling far down lays the rows out as they come in sight. After `End` has read the whole history, `z` there may take a moment |

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
| 8.2 | Look at the file list | Each file with its status letter in colour (A green, M, D red, R), its path cut from the left with `…` when long, `+a −d` and a bar; a renamed file says where it came from; more than 12 files list 10 and `N more files — click for all` |
| 8.3 | Click a file row | The detail scrolls to that file's diff |
| 8.4 | Look at the diff | One column: the `@@` rows in blue, added rows green and deleted rows red, the changed words marked a shade deeper, the line numbers at the left; only the detail's wheel scrolls it, with no scrollbar |
| 8.5 | Press Enter, then Esc | Enter narrows the graph and spreads the diff side by side (old left, new right, the empty side hatched); the title's toggle follows; Esc brings the graph back, and a second Esc clears the filter |
| 8.6 | Click a parent link | The graph selects the parent and the detail shows it |
| 8.7 | Select a merge into main | `N commits came into main by this merge` with their hashes, subjects and times, and whether they are folded or open in the graph; the note that a merge's diff comes in the next stage; no diff |
| 8.8 | Select a commit with a binary file, and a big one | The binary file says it has no diff to show; a very long diff ends with `… the rest is left out` |
| 8.9 | Narrow the window below 900 wide | The side column goes and the detail stacks under the graph, still readable; Enter still spreads the diff |
| 8.10 | Switch to the light theme | The diff's greens, reds and blues and the hatching stay readable |
| 8.11 | Look at a commit adding a file that is not UTF-8 (Latin-1 `café`) | Under the file's name: `Not UTF-8: bytes that are not are shown as �`; the bytes show as that mark (a box in this font) in the diff |
| 8.12 | Look at the diff of a long line, in one column and side by side (`Enter`), in a narrow and a wide window; also a long Japanese line | It wraps at the pane's edge with the next part under it, the line number only on the first line; the row is as tall as its text, with no empty lines under a Japanese one and nothing cut off with `…` |

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
| 11.9 | Make a local branch no worktree is on that changes the same lines as main (in the main worktree: `git switch -c topic main~1`, edit and commit, `git switch main`); press `1` | The branch's head has the red `⚠ Conflict ahead` pill; hovering says `Would conflict with main in N places (<files>)`; main's head gets no pill for it, and the branch is not in the table of pairs under `3`. A branch already merged into main has no pill |

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

## 13. Branch review

Set up: a worktree on a branch with two commits from main (a renamed file, a changed file that main changed on the same lines too, an added file, an added image) and an uncommitted change to another file. Where michi keeps the files checked: `%LOCALAPPDATA%\uchmk\michi\review\<repo>-<hash>.json`; the last look is the ref `refs/michi/seen/<branch>`.

| # | Do | Expect |
| --- | --- | --- |
| 13.1 | Press `3`, select the worktree and press `d` (or click `Diff d` on it) | The review takes the view: the branch in bold, the switches `Uncommitted u` (lit) and `Since last look s`, `Seen 0 / 5` and `Back Esc`; under them, in red, `Would conflict with main in 1 places (lib.rs)`; the files on the left with a box, the letter (R A M in their colours), the path and `+n −n` (`Binary` for the image); the first file's diff on the right; the key bar lists `j k`, `Space`, `u`, `s`, `Enter`, `PageDown PageUp`, `Esc` |
| 13.2 | Press `j` / `k`; click a file's name; click another file's box | The selection and the diff follow the keys and the click; the click on a box ticks that file without moving the selection |
| 13.3 | Press Space on a file, then on a ticked one | The first is ticked, the selection goes on to the next file not ticked and `Seen` counts up; Space on a ticked file takes the tick off |
| 13.4 | Press Esc, change and save a ticked file in the worktree, wait a second and press `d` | The other ticks are kept; the changed file is not ticked any more (a tick is for the content it was given to); the file above lists the branch |
| 13.5 | Press `u`, then `u` again | The switch goes dark and the file changed only in the worktree leaves the list, `Seen n / m` with it; the second `u` brings it back |
| 13.6 | On a branch never looked over, press `s`; then Esc, commit once more in the worktree, press `d` and `s` | First the toast `Not looked over before: all of it is new`; then the switch is lit, the title reads `since the last look (<short hash>)` and only the files changed since show (and the uncommitted ones); with nothing new, `No file changed since the last look` |
| 13.7 | Press Enter, then Esc twice | Enter widens the diff side by side (the switch reads `Back to one column Esc`), the first Esc puts it back, the second closes the review on the worktree table |
| 13.8 | Select a file with a long diff; roll the wheel over it, press PageDown and PageUp; select another file | The diff scrolls (no scrollbar); the other file starts at its top |
| 13.9 | Press `d` on a worktree with a detached HEAD; on a branch with nothing from main | The toast `This worktree is on no branch`; the review reads `No file changed from main` |
| 13.10 | Commit in the worktree while the review is open; switch the worktree to another branch from a terminal | The list follows the new commit within about a second, keeping the ticks; on another branch the list stays as it was and nothing is read |
| 13.11 | Narrow the window to its smallest; switch to the light theme | The branch name is cut with `…`, the files keep their counts and the key bar folds what does not fit; the colours stay readable in both themes |
| 13.12 | In the review of a branch behind main press `m`; Esc; once more and `y`; on `main`'s own review | A band `Merge main into <branch> in <worktree>?`; Esc merges nothing; yes merges (a toast; a clash opens the `4` view); on main a toast says it is main itself | [ ] |

## 14. Changes: stage and commit

Set up: a worktree with a staged file, a changed file with two hunks far apart, a new file, a binary file and a renamed file; a second worktree; a commit pushed and one not.
michi writes only through `git add`, `git reset`, `git apply --cached` and `git commit` in the worktree shown.

| # | Do | Expect |
| --- | --- | --- |
| 14.1 | Press `2` in the set-up worktree | The top bar: the worktrees as chips (the window's own lit, a green ● on one where a Claude is writing), `[ ]` after them, `Amend A` on the right; the files on the left under `Staged` and `Not staged`, each with its letter (M A D R `?` in their colours), the path and `+n −n`, the new file last with `?`; under the list the message box with `What changed, and why` and `Commit`; the selected file's diff on the right; the `2` tab reads `2 Changes` and the number of files listed (a file both staged and changed counted once); the key bar lists `Space`, `c`, `l h`, `a`, `j k`, `v`, `A`, `[ ]`, `Enter`, `Esc` |
| 14.2 | Press `j` / `k`; Space on a file not staged, then on a staged one; `a`, then `a` again; click a file, double-click another | Space moves the file to `Staged` (and back out on a staged one) and the selection stays on that file; the first `a` stages everything, the second unstages everything; a click selects, a double-click stages or unstages; `git status` in a terminal agrees each time |
| 14.3 | On a changed file with two hunks press `l`, `j`, `v`, `j`, Space; then `a`; then `h` | `l` puts an accent strip on the first changed line and the title reads `Space in or out · v range · a hunk · s split · d throw away · h back`; `j` walks the changed lines only; `v` picks a range (tinted blue); Space stages just those lines (`git diff --cached` shows them, `git diff` the rest); `a` stages the hunk under the cursor; `h` goes back to the files |
| 14.4 | Press `l` on the new file, on a binary file and on a renamed file | The toast `This file goes in whole: its lines cannot be picked`; the diff says the same; Space stages the whole file |
| 14.5 | Press `c`, write a two-line message (`j`, `k`, Space type as text), press Ctrl+Enter | The box takes the keys; the commit is made (toast `Committed <hash>`), the box and the staged list empty, the graph (`1`) shows the commit with the message; Esc in the box only leaves it |
| 14.6 | Press Ctrl+Enter with nothing staged; then stage a file and press Ctrl+Enter with an empty box | The toasts `Nothing staged to commit` and `Write a commit message first (c)`; nothing is written |
| 14.7 | On a commit not pushed press `A`, change the message, Ctrl+Enter | The switch lights and the box fills with the head's message (the title reads `Message of the commit amended`, the button `Amend`); the head is replaced: one commit, not two; `A` again before writing empties the box |
| 14.8 | On a pushed commit press `A`, then Ctrl+Enter; Esc; Ctrl+Enter and `y` | A band under the top bar: `The last commit is pushed: amending it rewrites history others may have.` with `Go ahead y` and `Not now Esc`; Esc leaves the commit as it was; `y` amends it |
| 14.9 | Start a Claude in a worktree from tsumugi; in michi open that worktree in `2` and press Space; press `x`, Esc, Space, `y`, then Space on another file | The band `A Claude is writing in this worktree. Write beside it?`; `x` does nothing while it waits; Esc says no and nothing is staged; `y` stages; the next Space does not ask again until that Claude stops or another worktree is shown |
| 14.10 | Write a message, press `]` and `[`; click another worktree's chip; open a repository with one worktree and press `]` | Each worktree's files are read anew and the chip lights; the message written stays with its worktree; with one worktree, the toast `There is only one worktree` |
| 14.11 | Press Enter, then Esc; pick lines and press Enter again | The diff spreads side by side, old on the left and new on the right; the picked lines are tinted on their side only; Esc goes back to one column (on the lines, the first Esc leaves the lines) |
| 14.12 | With `2` open, save a file in the worktree from an editor; let a Claude commit there | The list follows within about a second and keeps the selection on the same file; staged lines stay staged |
| 14.13 | Leave `2` for `1`; then make the worktree clean | The `2` tab still counts the uncommitted files of the worktree last shown; with none, no number |
| 14.14 | Add a pre-commit hook that prints a line and exits 1, then commit | The toast `Could not write: …` with the hook's words for 8 seconds; the staged list and the message stay |
| 14.15 | Narrow the window to its smallest; switch to the light theme | The list keeps at least 240 px, the chips end in `…`, the key bar folds what does not fit; the tints, the band and the letters stay readable in both themes |
| 14.16 | Add a pre-commit hook that prints five lines (some in colour, `printf '\033[31m…\033[0m'`) to stdout and stderr and exits 1; commit; Esc, Esc; commit again, then `j`; commit again, unstage and stage a file | The toast `Could not write: <the first line>`; in place of the diff the red title `Could not commit: all git and the hooks said` with `Close Esc` at the right and all five lines below in a fixed-width font, without colour codes; the wheel and PageDown scroll a long one (no scrollbar); the first Esc only leaves the message box, the second closes it and the diff is back; `j` (another file) and a write that goes through close it too; a write that fails with one line only shows the toast |
| 14.17 | On a hunk with two changes a few lines apart press `s`; `j`, `a`; Space; save the file from an editor; `s` again; on a file whose hunks each have one change press `s` | `s` goes into the lines and each hunk is cut into one piece per run of changes, a `@@` line before each; `a` picks the piece under the cursor and Space stages only it (`git diff --cached` shows that change alone); after the save the file stays cut; `s` again joins the hunks back; on the other file the toast `Nothing to split: each hunk has one run of changes` |

## 15. Fetch, pull and push

Set up: a bare repository as `origin` and two clones of it (Git Bash: `git init --bare r.git; git clone r.git a; git clone r.git b`), a commit pushed from `a`; in `a` a second worktree on a new branch.
michi runs `git fetch --prune --progress`, `git pull --ff-only`, `git push` (and `--force-with-lease` only after `y`) in the worktree the view points at.

| # | Do | Expect |
| --- | --- | --- |
| 15.1 | Press `f` (or click `fetch f` in the tool band of `1`); press `f` again; restart michi | While it runs the key bar's right side reads, in amber, `fetch · <step> n%` and `Esc stops`; then the toast `Fetched: n refs moved`, the graph shows the moved remote branches and the key bar reads `⟳ fetch just now` (later `n min ago`); the second `f` says `Fetched: nothing new`; after the restart the key bar still gives the time of the last fetch |
| 15.2 | On a branch behind its upstream press `p`; press `p` again; on a branch with no upstream press `p`; on a branch that went its own way press `p` | `Pulled n commits from origin/main` and the graph moves the branch; then `Already up to date with origin/main`; `main has no upstream to pull from: P pushes it`; `origin/main went its own way (↑a ↓b): a pull only goes forward, so nothing was merged` and nothing moves (pull is fast-forward only) |
| 15.3 | Commit and press `P`; press `P` again; on a new branch with no upstream press `P` | `Pushed to origin/main`; `origin/main has it already`; `Pushed to origin/feat, now its upstream` and `git status -sb` shows `feat...origin/feat` |
| 15.4 | Let a second clone push first, commit here and press `P` without fetching | The toast `origin/main refused the push: [rejected] … (fetch first). f fetches its commits; then F pushes over them, asking first`; nothing is rewritten |
| 15.5 | After 15.4 press `f`, then `P`; then `F`; Esc; `F` again and `y` | `P` only says `origin/main has n commits this branch does not: F pushes over them, asking first` and pushes nothing; `F` shows a band under the top bar: `origin/main has n commits this branch does not. Push over them? What origin/main has now is kept under refs/michi/backup/ first.` with `Go ahead y` and `Not now Esc`; Esc pushes nothing; `y` pushes with `--force-with-lease` and says `Pushed over origin/main; what it had is kept in michi/backup/<time>`; that ref points at the remote's old tip and `git reflog refs/michi/backup/<time>` reads `michi: push --force-with-lease over origin/main` |
| 15.6 | On a branch only behind its upstream press `P`, then `F` | Both say `Nothing to push: origin/main is n ahead (p pulls)`; no band |
| 15.7 | Add a remote that does not answer (`git remote add slow https://10.255.255.1/x.git`), press `f`, then `f` again, then Esc | The key bar shows `fetch` and `Esc stops` in amber; the second `f` says `fetch is running: Esc stops it`; Esc says `Stopping fetch…`, then `fetch stopped`; afterwards no `git`, `git-remote-https` process is left (Task Manager); remove the remote after |
| 15.8 | In `1` press `p` / `P`; in `3` select another worktree's row and press `P`; click `pull p` on a card; in `2` show another worktree and press `p` | `1` works on the worktree michi was opened in; `3` on the selected worktree; `2` on the worktree shown; a band (15.5) shows over `2` and `3` too |
| 15.9 | Start a Claude in a worktree from tsumugi; press `p` for that worktree; Esc; `p` and `y` | The band `A Claude is working in <worktree>. Pull there anyway?`; Esc pulls nothing; `y` pulls |
| 15.10 | Fetch from an https remote that needs a sign-in; fetch from an ssh remote whose key has a passphrase and no ssh-agent | The Git Credential Manager window comes up as it does in a terminal and the fetch goes on after it; the ssh one fails at once (no prompt hangs) with `fetch failed: … — start ssh-agent and ssh-add your key` |
| 15.11 | Narrow the window to its smallest with a band shown and a fetch running; switch to the light theme | The band wraps its text and grows, its keys stay whole; the key bar folds its keys before the fetch progress or age; both stay readable in both themes |
| 15.12 | In a clone with two remotes and no `origin` (`git remote rename origin up`, `git remote add mine <path>`), press `P` on a branch with unpushed commits; type `mi`, Enter; then `P` again on another branch; Esc | The box `Push to: pick a remote` (`pushing <worktree>`) lists `mine` and `up`; the keys read `Enter pushes to it (and sets the upstream) · Esc closes`; `mi` leaves `mine`; Enter closes it and pushes there (`Pushed … to mine`, upstream becomes `mine/<branch>`); Esc pushes nothing. With `origin` or a `pushRemote` set no box shows | [ ] |

## 16. Branches, tags and revert

Set up: as in 15 (a bare `origin`, a clone `a` with a second worktree on `feat`), a few more branches, one pushed and deleted locally (`git push origin x:refs/heads/remote/only`).
michi runs `git branch`, `git switch` (`-c`, `--track`), `git branch -D`, `git tag` (`-a` with a message), `git tag -d`, `git push <remote> refs/tags/<name>:refs/tags/<name>` and `git revert --no-edit` (`-m`, then `--abort` when it clashes),
and `git update-ref refs/michi/backup/<time>` before it deletes.

| # | Do | Expect |
| --- | --- | --- |
| 16.1 | In `1` select a commit and press `b` (or click `Branch b` in the tool band); type part of a name; ↑ ↓; move the pointer over the rows; Esc; `b` and click outside; press `b` and type a name at once | A palette under the top bar: the box with `Branch: a name switches to it, a new one is made`, under it `<worktree> · at <commit>`, the local branches (with `here`, `in <worktree>`, `↑a ↓b`) then the remote branches no local one has, each with its commit at the right; the foot reads `Enter switches · Shift+Enter makes and switches · Del deletes · Esc closes`; typing narrows them, the typed letters lit and the best first; the arrows and the pointer move the selection (a resting pointer does not); Esc and a click outside close it; the letters typed right after `b` land in the box |
| 16.2 | Type a branch name with the Japanese IME and confirm it | The composition shows in the box and the confirmed name narrows the rows; nothing is lost |
| 16.3 | Type a new name and Enter; `b` again, another new name and Shift+Enter | The last row reads `<name>  make it at <commit>`; Enter: `Made branch <name> at <commit>`, the graph shows its chip and the worktree stays on its branch; Shift+Enter: `Made branch <name>; <worktree> is on it` and `git branch --show-current` there gives it |
| 16.4 | Pick a branch no worktree has and Enter; type the name of a remote branch with no local one (`origin/x`) and Enter; type it without `origin/` | `<worktree> is on <name> now`; `Made <name> following origin/<name>; <worktree> is on it` and `git status -sb` shows `<name>...origin/<name>`; the name without `origin/` lists `origin/<name>` and no `make it` row |
| 16.5 | Pick the branch this worktree is on; pick one checked out in another worktree; in a worktree in the middle of a merge (`git merge` that clashes) pick a branch and Enter, then type a new name and Enter | `<worktree> is on <name> already`; `<name> is checked out in <worktree>: a branch is in one worktree at a time`; `Branches can be made. Switching is stopped in <worktree>: it is in the middle of a merge.` for 8 seconds; the palette stays open each time; the new name is made; `git merge --abort` after |
| 16.6 | Select another worktree's uncommitted row in `1` and press `b`, pick a branch and Enter; select a commit and press `b` | With the uncommitted row the palette reads `<that worktree> · at <its HEAD>` and switches that worktree; with a commit, the window's own worktree and that commit |
| 16.7 | Start a Claude in a worktree from tsumugi; select its uncommitted row, `b`, pick a branch and Enter; Esc; again and `y` | The band `A Claude is working in <worktree>. Switch it to <name> anyway?`; Esc switches nothing; `y` switches |
| 16.8 | Select a branch no worktree has, whose commits no other branch or tag has, and press Delete; Esc; again and `y`; delete a branch merged into main the same way | The band `Delete branch <name>? When no other branch or tag has its commits, they are kept under refs/michi/backup/ first.`; Esc deletes nothing; `y`: `Deleted branch <name>; its commits are kept in michi/backup/<time>` and `git log refs/michi/backup/<time>` shows them; the merged one: `Deleted branch <name>` and no new backup |
| 16.9 | Press Delete on the branch checked out here, on one checked out in another worktree, on a remote branch | `<name> is checked out in <worktree>: switch it away first`; `Remote branches are not deleted here: git push origin --delete <name> does it` for 8 seconds; nothing is deleted |
| 16.10 | Select a commit and press `T`; type `v0.1 first release` and Enter; `T`, `v0.2` and Enter | The box reads `Tag: a name, then its message if any`, the tags are listed, the last row reads `v0.1  on <commit>`, the foot `Enter makes or shows · Shift+Enter makes and pushes · Del deletes · Esc closes`; `Made tag v0.1 at <commit>` and the graph shows `◆ v0.1`; `git cat-file -t v0.1` gives `tag` and its message is `first release`; `v0.2` is a light tag (`git cat-file -t` gives `commit`) |
| 16.11 | `T`, pick `v0.1` and Enter; `T`, pick it and Shift+Enter; `T`, type `v0.3` and Shift+Enter; during a fetch (15.7) `T`, `v0.4` and Shift+Enter | Enter selects its commit in the graph; Shift+Enter: `Pushed v0.1 to origin` and `git ls-remote --tags origin` lists it; `v0.3` is made and pushed; during the fetch `fetch is running: Esc stops it` and nothing is made |
| 16.12 | `T`, pick `v0.1` and press Delete; Esc; again and `y` | The band `Delete tag v0.1? What it points at is kept under refs/michi/backup/ first.`; Esc deletes nothing; `y`: `Deleted tag v0.1; kept in michi/backup/<time>` and `git cat-file -t refs/michi/backup/<time>` gives `tag`; the remote keeps its tag, so the next fetch brings it back |
| 16.13 | Select a commit on the current branch and press `t`; Esc; `t` and `y` | The band `Revert <id> “<subject>” with a new commit on <branch> in <worktree>?` (`A Claude is working there.` added when one is); Esc does nothing; `y`: `Reverted <id> on <worktree>: <new>` and the graph shows `Revert "<subject>"` on top |
| 16.14 | Select a merge commit and press `t`; press `1` | The band `<id> “<subject>” is a merge. Revert it with a new commit on <branch> in <worktree>, keeping which parent's side?` with `keep parent 1 1`, `keep parent 2 2` and `Not now Esc`; `1` reverts it keeping the first parent's side: `Reverted <id> on <worktree>: <new>` |
| 16.15 | Revert a commit whose lines a later commit changed; revert a commit already reverted; in a worktree in the middle of a merge press `t` | `Reverting <id> clashed in <files>, so it was called off: git revert <id> in a terminal lets you resolve it` and `git status` is clean; `Nothing to revert: <id> is undone already`; `<worktree> is in the middle of a merge: finish it first` |
| 16.16 | Look at the tool band of `1`; press `?`; narrow the window to its smallest with the palette open; switch to the light theme | The tool band reads `Branch b` (`ブランチ b` in Japanese); help lists `b Make or switch a branch`, `T Tag the commit`, `t Revert the commit (asks first)`; the palette narrows with the window, long names end in `…` and the commits stay at the right; readable in both themes |

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
| 17.7 | In a worktree in the middle of a merge, `s` and Enter on the first row, then on a stash; with a Claude working in the worktree (from tsumugi), `s` and Enter; Esc; again and `y`; the same on a stash | `Stashes are stopped in <worktree>: it is in the middle of a merge.` for 8 seconds and the palette stays; with the Claude the band `A Claude is working in <worktree>. Put its changes away anyway?` (`Put the stash back there anyway?` on a stash); Esc does nothing; `y` does it |
| 17.8 | Select another worktree's uncommitted row in `1` and press `s`, Enter; with no changes anywhere press `s` and Enter | The palette reads `in <that worktree>` and puts that worktree's changes away; with none: `<worktree> has no changes to put away` |
| 17.9 | Click a stash in the side column; Esc; click it again and press `j` | The detail shows the stash as in 17.3 and its row is lit in blue; Esc brings back the selected commit and the row goes dark; `j` does too |
| 17.10 | Press `?`; narrow the window to its smallest with the palette open; switch to the light theme | Help lists `s Stash`; the palette narrows with the window, long messages end in `…` and the hashes stay at the right; readable in both themes |

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
| 18.5 | With a Claude working in the worktree (from tsumugi), `d` on a file | The band carries both the question for the file and `この worktree では Claude が書いている途中です。横から書き込みますか？`; Esc does nothing; `y` throws the changes away |
| 18.6 | Press `?`; narrow the window to its smallest with the band up; switch to the light theme | Help lists `d 変更を捨てる（先に確かめる）`; the band's text wraps or ends in `…` with `y` and `Esc` still visible; readable in both themes |

## 19. Clean up

Set up: a repository with a remote (`origin`) and these branches: `done` merged into `main`; `feat` merged and checked out in a worktree `r-feat`;
`squashed` pushed, then deleted on the remote and fetched with `f` (like a pull request merged with squash); `busy` merged and checked out in
a worktree `r-busy` with a change not committed; a worktree `r-old` on an unmerged branch `old`, its folder then deleted; `wip` unmerged and never pushed.
michi lists with `git for-each-ref --merged` and the upstream's `[gone]`; for each row picked it runs `git worktree remove <folder>`, then
`git branch -D <branch>` with `git update-ref refs/michi/backup/<time>` first when no other branch has the branch's commits.

| # | Do | Expect |
| --- | --- | --- |
| 19.1 | In `3` press `c` | A list over the table: the title `Clean up: branches merged into main or gone from their remote, and worktrees whose folder is gone`, under it `“Gone from its remote” is as of the last fetch: <age>`; rows with a box, none ticked: `done` `merged into main` with its hash, `feat` `merged into main · its worktree r-feat goes too`, `squashed` `gone from its remote (its tip is kept first)`, `r-old` `a worktree whose folder is gone`; `busy` grey without a box and `has uncommitted changes` in yellow; `main`, `old` and `wip` are not listed; the foot `0 picked · Space picks · a picks all · Enter cleans up · Esc closes` |
| 19.2 | `j` / `k`; Space on a row, on `busy`; `a`, `a` again; click a row; Esc; `c` and click outside | Space ticks the row and the foot counts it; on `busy` nothing happens; the first `a` ticks every row that can go, the second lets them all go; a click ticks the row; Esc and a click outside close the list |
| 19.3 | `c`, Enter with nothing ticked | The toast `Pick the rows to clean up with Space first`; the list stays |
| 19.4 | `a`, Enter; Esc | The band `Clean up 4: done, feat, squashed and 1 more? The worktree folders go too, with the files git ignores in them: r-feat. Commits no other branch has are kept under refs/michi/backup/ first.` with `Go ahead y` / `Not now Esc`; Esc goes back to the list and nothing is gone |
| 19.5 | Enter, `y` | The toast `Cleaned up 4: done, feat, squashed and 1 more. Unmerged commits are kept in michi/backup/<time>`; the list closes; the table has `main` and `busy` left and the top bar's chip reads `worktree 2`; `git branch` no longer has `done`, `feat` and `squashed`, `git worktree list` no longer has `r-feat` and `r-old`, the folder `r-feat` is gone, and `git log -1 refs/michi/backup/<time>` is `squashed: one` |
| 19.6 | Open a tsumugi tab in a merged branch's worktree; press `c` | That worktree's row is grey with `a tsumugi tab is open there`; closing the tab and pressing `c` again lists it with a box |
| 19.7 | Commit to a ticked branch from a terminal while its band is up, then `y` | It stays: `… Stayed: <branch> (it moved after it was listed)`; the others go |
| 19.8 | With nothing left to clean up press `c` | The toast `Nothing to clean up: no branch is merged into main or gone from its remote, and no worktree's folder is gone`; no list opens |
| 19.9 | Press `?` in `3`; narrow the window to its smallest with the list open; switch to the light theme | Help lists `c Clean up`; long rows end in `…` with the boxes and hashes still visible; readable in both themes |

## 20. The command line and `michi.com` (Windows)

| # | Do | Expect |
| --- | --- | --- |
| 20.1 | Put `michi.exe` and `michi.com` (from the Build artifact, or `michi-com.exe` copied to that name) in one folder; in PowerShell there run `$v = .\michi --version; "[$v]"` | `michi.com` answers: `[michi 0.15.0]` (the version), printed by the second statement, and the prompt comes back after it |
| 20.2 | Run `.\michi --help` in PowerShell, then in `cmd` | The usage lines print before the prompt comes back, in both |
| 20.3 | Run `.\michi --bogus`, then `.\michi C:\no-such-folder`, each followed by `$LASTEXITCODE` (`echo %ERRORLEVEL%` in `cmd`) | `michi: unknown argument` and the usage, then `2`; `michi: not a folder: C:\no-such-folder`, then `2`; no window opens |
| 20.4 | Run `.\michi C:\dev\some-repo` (a repository), then `.\michi` with no argument in a repository's folder | The window opens on that folder and the prompt comes back as soon as it is up, with michi still open; closing the window later prints nothing in the console |
| 20.5 | Run `$v = .\michi.exe --version` (the windowed program itself), wait a few seconds, then run `.\michi.exe --version` | `$v` is empty and no `panicked` message appears later (the line has nowhere to go and is let go); the second, with no `$v =`, prints the version in the console |
| 20.6 | In a repository's folder run `$v = .\michi` (no argument, its output taken through a pipe); once the prompt is back, run `Get-ChildItem $env:TEMP\michi-com-*`; close michi and run it again | The prompt comes back once the window is up, with michi still open; two files (`.out` and `.err`) while it runs, none after it closes |

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
| 21.19 | With a Claude working in `r` (from tsumugi), solve all and Ctrl+Enter | The band `A Claude is working in r. Commit the merge there anyway?`; Esc commits nothing; `y` commits |
| 21.20 | Press `?` in `4`; narrow the window to its smallest; switch to the light theme | Help lists `] [ Next / previous place`, `o Take ours`, `t Take theirs`, `b B Take both (ours first / theirs first)`, `Ctrl+Enter Commit once all is solved`; the bar's pill and button stay visible, long lines end in `…` or wrap; readable in both themes |
| 21.21 | In a worktree stopped by a merge (the `r` repo of #23), open the graph (`1`) and the worktree table (`3`) | The graph's top has a red-ringed row with dashed edges to both parents, `⚠ 衝突 残り N` and `マージ中: feat → main`, the hash `MERGE`; solving every place in `4` turns it to `✓ 解けた`. No `⚠ 衝突の予測` pill anywhere; the table's clash column is `—`, the pairs show `·` with「予測は衝突の解消（4）の実物に変わりました。」 |
| 21.22 | While `r` is stopped by the merge, look at the graph's tool band and hover `Pull` and `Stash`; press `p` | `Pull` and `Stash` are dimmed, the hover says `Held while a merge is under way`; `p` toasts why it waits; the other buttons are unchanged |
| 21.23 | In a repository with no `rerere.enabled` (`git config --get rerere.enabled` prints nothing), make a merge clash so `4` opens | A band asks whether to remember solved clashes (`y` / `n` / Esc); `y` toasts that rerere is on and `git config rerere.enabled` says `true`; in a second clash (after `git merge --abort`) no band comes again this session. With `n` the config says `false` and the band never returns |
| 21.24 | With `rerere.enabled=true` and a recorded resolution (solve a clash, `git rerere`, `git merge --abort`), merge the same branch again so `4` opens | The file rerere solved shows `✓ ↺ 前に解いた形` on its places and the bottom bar says how many are such; `Enter` on it takes the mark away (the count drops); `x` starts the file over with open places and a toast naming the backup under `refs/michi/backup`, and the same clash is no longer solved by rerere on a new merge. `x` on a place rerere did not solve toasts that it did not |
| 21.25 | Merge two branches that changed the same lines in different commits so `4` opens, and select a place | Under the place's header two rows appear, `自分` and `相手`, each with the short hash, subject, author and date of the commit that changed those lines on that side (a side no commit touched says so); they show a moment after the place is selected, not at once, and the window never stalls; a long subject is clipped, not wrapped |
| 21.26 | In the `4` view select an open text place and press `e`; edit the lines; Ctrl+Enter. Then `e` on a solved place and on a file that clashed whole; `e` and Esc | The result box turns into a field holding ours then theirs, with a hint (Ctrl+Enter solves, Esc leaves); Ctrl+Enter solves the place with exactly what was written (tag `ここで書いた`, the file shows it after the next read) and moves to the next open place; Esc leaves the place as it was; a solved place says to restart the file, a whole-file clash says to take a side; the field keeps the keys (letters type, they do not pick) |
| 21.27 | Merge two branches that changed one line in different words (e.g. `let a = 1;` to `let a = 22;` and to `let a = 3;`) so `4` opens | In the ours and theirs columns only the words that differ from the base are on a stronger ground (`22`, `3`); with the base column hidden or unknown they are compared with the other side; a line one side added is marked whole; unchanged lines are plain; long lines are still cut with … |
| 21.28 | In `4` with places left and a Claude (tsumugi tab) in that worktree, press `g` or click「Claude に渡す」in the bar; then do the same in a worktree with no Claude tab | A toast says the Claude was asked / a new tab started; in the first case the request line (path of the brief + 「Do not commit」) is typed into that Claude's tab; in the second a new tsumugi tab named after the worktree starts `claude` with it; the brief (`%TEMP%\michi-handoff\<worktree>-<time>.md`) lists each open place with ours / base / theirs and both sides' intent; the repository itself gets no new file; when the Claude adds the files the remaining count falls by itself; with nothing left `g` only says so |
| 21.29 | In `4` with a file that clashed whole (deleted on one side, or binary), click「外部ツールで開く」or press `w`; once with `merge.tool` set in git config (e.g. `vscode`, `meld`), once without | With a merge tool: `git mergetool` opens that tool on the file, and when it is closed a toast says so, the file is added if the tool reported success, and the count falls; without: the file opens in the program the system has for it (Explorer / `open` / `xdg-open`) and the toast says so; a failure shows git's first line |
| 21.30 | Hand a conflict to a Claude with `g` (21.28); keep michi open while that Claude works until it stops and waits for you | While it runs nothing is said; once it stops (waiting or done) a toast「<worktree> の Claude が止まりました（待ちか済み）: diff を見てください」shows for 10 s, once | [ ] |
| 22.1 | Press `b`, select a local branch not checked out, `F2`, type a new name, Enter; then `F2` and Enter with the same or a taken name | The box shows `Rename <old> to:` with the old name in it and no rows; Enter toasts `Renamed <old> to <new>` and the graph's chip shows the new name after the next read; the same name does nothing, a taken or bad name toasts git's reason; Esc closes | [ ] |
| 22.2 | With a remote branch of the same name (`origin/x`), press `b`, select the local `x`, `Ctrl+U`; then on a branch with no such remote | Toast `x now follows origin/x` (`git rev-parse x@{upstream}` says so); without a remote of that name a toast says no remote branch is named so | [ ] |
| 22.3 | In a worktree on a branch, press `b`, select another branch (or a remote one), `Ctrl+R` and yes; once more with a clashing branch | A band names the branch, the worktree and the target; yes rebases and toasts the backup of the old tip (`refs/michi/backup/…`); a clash leaves the worktree as it was (no rebase in progress) and toasts git's words; the same branch or a worktree in the middle of something is refused with a toast; a Claude at work adds its warning to the band | [ ] |
| 22.4 | In the graph select a commit and press `m`, yes; once on a clashing commit | A band names the commit and the window's branch; yes merges (a toast with the new commit); a clash opens the `4` view; on the uncommitted row or a detached worktree a toast says why not | [ ] |
