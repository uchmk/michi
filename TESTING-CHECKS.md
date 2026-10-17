# 実機チェックリスト

`TESTING.md` から `cargo run -p michi --example make-testcheck` で生成している。**正は TESTING.md** で、
このファイルはその表を 1 行ずつ印を付けられる形に並べたもの。食い違ったら TESTING.md を信じること。

**チェック（`[x]` と `[~]`）だけは手で書いてよく、生成し直しても残る。**それ以外を書き換えても次の生成で消える。
`[x]` は実機で確かめた印、`[~]` は見た目の行を画面の画像で判断した印（済みには数えない。持ち主が同じ画像を見て `[x]` にする）。
**件数はこのファイルに書かない**（印を付けた PR が並ぶと必ずぶつかるため）。`-- --stats` で出る。

付ける前に「失敗していたら、画面かディスクの何が違ったはずか」を自問すること。

## 1. Window and title bar

- [ ] **1.1** Start `michi C:\dev\tsumugi` (any repository) → A 1280x800 window with no system title bar; the logo, `michi` (never 道), the tabs `1 Graph` `2 Changes` `3 Worktrees` hanging from the bar's foot, the repository chip, a palette field, the `,` button and three window buttons on the right 〔未訳〕
- [ ] **1.2** Look at the chip → The repository's name in the accent colour, a drawn branch mark (a trunk and a twig with three dots, not a box), then `main · worktree N` with the real count; hovering shows the repository's top folder 〔未訳〕
- [ ] **1.3** Look at the window title (Alt+Tab, the taskbar) → `tsumugi — michi` once the repository is read 〔未訳〕
- [ ] **1.4** Drag the empty part of the title bar → The window moves; double-click maximises and restores 〔未訳〕
- [ ] **1.5** Click minimise, maximise, close → Each does what it says; close shows a red button under the pointer; the maximise glyph turns into two squares while maximised 〔未訳〕
- [ ] **1.6** Drag each edge and corner → The window resizes with the matching pointer; nothing resizes while maximised 〔未訳〕
- [ ] **1.7** Shrink the window to its smallest → It stops at 720x420; the palette field shortens, then the chip drops the branch, then the chip goes; the tabs never overlap the buttons 〔未訳〕
- [ ] **1.8** Look at michi.exe in Explorer, the taskbar and Alt+Tab → The same logo as in the title bar (the forked path on the tile), sharp at each size 〔未訳〕
- [ ] **1.9** Start michi in a folder with a Japanese name, or on a branch with one → The Japanese draws (after a frame or two), not boxes 〔未訳〕

## 2. Views and keys

- [ ] **2.1** Press `1`, `2`, `3` → The tab in force moves and opens into the view; the graph shows the history (section 6), the worktrees show their table (section 10), the changes show the files to stage (section 14) 〔未訳〕
- [ ] **2.2** Click each tab → Same as the digit 〔未訳〕
- [~] **2.3** Press `?` in `1`, `2` and `3`; narrow the window below 460 wide and press `?` → A card with the keys: "Everywhere" first, then the view's, two to a line with the word at the left and the keys at the right, every key there down to `Esc`; the tab stays lit under it; `?` or Esc closes it; the narrow window has one to a line 〔未訳〕
- [ ] **2.4** Press `N` in any view (Enter spreads the diff: 8.5; `f` `p` `P`: 15; `b` `T` `t`: 16; `s`: 17) → A toast at the foot names the key's work and says it comes in a later stage; it goes after 4 seconds 〔未訳〕
- [ ] **2.5** Press Ctrl+Shift+P, or click the palette field → The same toast for the command search; the field lights under the pointer with a text cursor 〔未訳〕
- [ ] **2.6** Press a key with no work (`q`, `4`) → Nothing happens 〔未訳〕
- [ ] **2.7** Press `?` (or `F1`), in a window short enough that the list does not fit; then `Down`, `PageDown`, `End`, `Home`; then bind another key to Quit on the Keys page and press `?` again → The list has the global keys and the keymap's keys (`Ctrl+Q`, `Ctrl+Shift+P`, ...), then the keys of the view in front; the keys move the list and a thin bar shows where it is; the new Quit key shows in the list; `?` or Esc closes it 〔未訳〕

## 3. Keybar and clock

- [ ] **3.1** Look at the keybar in each view → The view's first keys (the first one lit in the accent colour), then `? Keys`; nothing overlaps the clock when the window is narrow 〔未訳〕
- [ ] **3.2** Look at the right of the keybar → The clock as common.toml says (time, date, weekday); it turns over on the minute 〔未訳〕
- [ ] **3.3** Turn the clock off in the settings → It goes from the keybar, in every uchmk app that is open 〔未訳〕

## 4. Settings

- [ ] **4.1** Press `,` → The settings open between the title bar and the keybar; the title bar stays, the `,` button is lit and no tab is lit; the keybar shows `Esc Back` and `Ctrl+Tab Next page` 〔未訳〕
- [ ] **4.2** Press Ctrl+Tab → The page moves from General to Theme, with a small michi preview (the three tabs, two branches) 〔未訳〕
- [ ] **4.3** Change the language to 日本語 → Every word of michi turns Japanese at once (the tabs 樹形図・変更・worktree, the keybar, the empty views), and other uchmk apps follow 〔未訳〕
- [ ] **4.4** Change the theme, and the size → The whole window follows at once; the logo and the chip stay readable in light and dark 〔未訳〕
- [ ] **4.5** Press Esc, click the `,` button, or click a tab → Each leaves the settings; a tab opens its view 〔未訳〕
- [ ] **4.6** Edit common.toml by hand while michi is open → michi follows within a second 〔未訳〕
- [ ] **4.7** In the settings open the Keys page; press `+ Add` on a row, press `Ctrl+Shift+K`, Enter; then `Ctrl+=`, `Ctrl+-`, `Ctrl+0` in the graph; edit keymap.toml (beside common.toml) by hand → The page lists the keys by layer (names in the language chosen); the new key is written to keymap.toml under `prepend_keymap` and a toast says `keymap.toml: 1 changed`; the keys zoom the window and say the size; a hand edit shows in the page within a second; Esc while a key is being pressed ends that, not the settings 〔未訳〕

## 5. git and errors

- [ ] **5.1** Start `michi C:\dev` (not a repository) → The chip shows `✗ Not a Git repository` in red and a toast says so with the folder; the title stays `michi` 〔未訳〕
- [ ] **5.2** Start `michi C:\no\such` from a terminal → The message `michi: not a folder: C:\no\such` in the terminal, exit code 2, no window 〔未訳〕
- [ ] **5.3** Start `michi --version` and `michi --help` from cmd and from PowerShell → The version or the usage prints in that terminal, exit code 0, no window 〔未訳〕
- [ ] **5.4** Start michi with no git on the PATH → The chip shows `✗ git was not found`; the keybar shows `⚠ git` in red 〔未訳〕
- [ ] **5.5** Start michi with a git older than 2.38 on the PATH → A toast for 10 seconds names the version and asks to update; the keybar keeps `⚠ git 2.x.y` in amber 〔未訳〕
- [ ] **5.6** Start `michi` in a worktree other than the main one → The chip names the repository (not the worktree's folder) and its branch; the count is the whole repository's 〔未訳〕

## 6. Graph

- [ ] **6.1** Open the repository → Within a second, the newest commits first: the graph, the subject with its names, the author, the date (the time today, `Yesterday` and the time, `M/D HH:MM` this year, `Y/M/D` before), the short hash; the trunk (main) runs down the leftmost lane; the header names the columns; only the graph has a scrollbar 〔未訳〕
- [ ] **6.2** Look at the names before the subjects → A worktree's branch as a filled pill with a worktree mark (main in the trunk's colour, others in their lane's); a remote branch as a dashed outline (`origin/main`); a tag in amber with a diamond; hovering a worktree pill shows its folder 〔未訳〕
- [ ] **6.3** Look at the merges into main → Each is folded: a ring on the trunk, a chip `▸N` (the commits inside) before the subject in the merged lane's colour; hovering the chip says how many it folds and names the branches still inside; the header's `Fold PRs z` toggle is lit 〔未訳〕
- [ ] **6.4** Press `z`, then `z` again → The merges open (side lanes appear) with the toast "Unfolded all the merges."; again, they fold with a toast giving how many merges and commits; the header toggle follows (lit or grey) 〔未訳〕
- [ ] **6.5** Click the header toggle → The same as `z` 〔未訳〕
- [ ] **6.6** Select a folded merge with `j`, press `l` (or →), then `h` (or ←) → `l` opens it: the chip becomes `▾N` with a dashed outline and its commits show on their own lane; `h` folds it again 〔未訳〕
- [ ] **6.7** Click a chip → Its merge opens or folds; the selection stays where it was 〔未訳〕
- [ ] **6.8** Press `j` `k`, ↓ ↑, PageDown PageUp, Home End → The selected row (lit across) moves and the list scrolls to keep it in sight; End goes to the oldest row read so far 〔未訳〕
- [ ] **6.9** Click a row; scroll with the wheel → The row is selected; the wheel scrolls the list without moving the selection 〔未訳〕
- [ ] **6.10** Press `/` and type part of a subject, a hash or an author's name → The field at the right of the tool band takes the keys, with the count (`3 hits`) at its right; the text sits in the middle of the field; the other rows dim; the hits get marks on the scrollbar 〔未訳〕
- [ ] **6.11** In the field, press Enter, Shift+Enter, ↓, ↑ → Enter selects the next hit (round to the first after the last), Shift+Enter the one before; the arrows move the selection; the field keeps the keys 〔未訳〕
- [ ] **6.12** Press Esc in the field → The filter clears and every row is bright again 〔未訳〕
- [ ] **6.13** Look at the scrollbar → Marks for the worktrees' heads (in their lanes' colours), the tags (amber) and the selected row (the accent); hovering a mark tells what it is 〔未訳〕
- [ ] **6.14** Drag the thumb; click the track; click a mark → The thumb stays under the pointer to the release; a click on the track brings that part into the middle; a click on a mark selects its row 〔未訳〕
- [ ] **6.15** Look at commits by dependabot[bot] and github-actions[bot] → The author reads `⚙ dependabot` and `⚙ actions`; hovering shows the full name 〔未訳〕
- [ ] **6.16** Open a repository with more than 2000 commits → The first screen comes at once; scrolling near the foot reads on by itself; the last row says how many older commits are left and reads them when clicked; the thumb shrinks as the list grows 〔未訳〕
- [ ] **6.17** Look at a second worktree's commits → They run on their own lane in the worktree's colour, from where they left main 〔未訳〕
- [ ] **6.18** Narrow the window to 720 wide; switch to the light theme → The subject is cut with an ellipsis and the names drop before they crowd it; the filter field stays inside the band; every colour stays readable in light 〔未訳〕
- [ ] **6.19** Open a repository with more than 2000 commits and no `.git\objects\info\commit-graph*` (a fresh `git clone` of a long history), wait a few seconds, close michi and open it again → `.git\objects\info\commit-graphs` appears while michi is open, with no toast; the history looks the same, and the second opening shows the first screen sooner. A repository with fewer commits gets none 〔未訳〕
- [ ] **6.20** In a history of 100,000 commits, press `z` to fold and unfold, and type in the filter (`/`) → Each answers at once, with no stall while typing; scrolling far down lays the rows out as they come in sight. After `End` has read the whole history, `z` there may take a moment 〔未訳〕

## 7. Side column

- [ ] **7.1** Look at the column left of the graph → Headings with counts: worktree, Branches, Remotes, Tags and Stash (when there is one); the worktree the window is on is lit; each worktree and branch has a dot in its lane's colour; there is no scrollbar, the wheel scrolls it 〔未訳〕
- [ ] **7.2** Click a worktree, a branch and a tag → The graph selects that commit and scrolls it into sight; the detail follows 〔未訳〕
- [ ] **7.3** Click a group (`test/`, `origin`) → It opens to list its branches further in, with the count at its right; again, it closes; hovering a branch inside shows its full name 〔未訳〕
- [ ] **7.4** Click a branch whose commit is older than what is read → The graph reads on until it gets there and selects it; one that is not in the history gives the toast `Not in the history: <hash>` 〔未訳〕
- [ ] **7.5** Have a branch ahead of its upstream and one behind (commit without pushing; `git fetch` after a push from elsewhere), and a stash (`git stash`) → The branch and its worktree show `↑N`, `↓N` or both in dim at the right; no number when even or with no upstream. A `Stash` section comes last with each stash's message and a hollow square; hovering shows its hash; a click shows it in the detail (17.9) 〔未訳〕

## 8. Detail and diff

- [ ] **8.1** Select a commit → Below the graph within a moment: the subject in bold, the full hash, the author, the local time and the parents (blue links), the body, the hint `Enter spreads the diff side by side`; `Reading the commit…` shows while it is read 〔未訳〕
- [ ] **8.2** Look at the file list → Each file with its status letter in colour (A green, M, D red, R), its path cut from the left with `…` when long, `+a −d` and a bar; a renamed file says where it came from; more than 12 files list 10 and `N more files — click for all` 〔未訳〕
- [ ] **8.3** Click a file row → The detail scrolls to that file's diff 〔未訳〕
- [ ] **8.4** Look at the diff → One column: the `` rows in blue, added rows green and deleted rows red, the changed words marked a shade deeper, the line numbers at the left; only the detail's wheel scrolls it, with no scrollbar 〔未訳〕
- [ ] **8.5** Press Enter, then Esc → Enter narrows the graph and spreads the diff side by side (old left, new right, the empty side hatched); the title's toggle follows; Esc brings the graph back, and a second Esc clears the filter 〔未訳〕
- [ ] **8.6** Click a parent link → The graph selects the parent and the detail shows it 〔未訳〕
- [ ] **8.7** Select a merge into main → `N commits came into main by this merge` with their hashes, subjects and times, and whether they are folded or open in the graph; the note that a merge's diff comes in the next stage; no diff 〔未訳〕
- [ ] **8.8** Select a commit with a binary file, and a big one → The binary file says it has no diff to show; a very long diff ends with `… the rest is left out` 〔未訳〕
- [ ] **8.9** Narrow the window below 900 wide → The side column goes and the detail stacks under the graph, still readable; Enter still spreads the diff 〔未訳〕
- [ ] **8.10** Switch to the light theme → The diff's greens, reds and blues and the hatching stay readable 〔未訳〕
- [ ] **8.11** Look at a commit adding a file that is not UTF-8 (Latin-1 `café`) → Under the file's name: `Not UTF-8: bytes that are not are shown as �`; the bytes show as that mark (a box in this font) in the diff 〔未訳〕
- [ ] **8.12** Look at the diff of a long line, in one column and side by side (`Enter`), in a narrow and a wide window; also a long Japanese line → It wraps at the pane's edge with the next part under it, the line number only on the first line; the row is as tall as its text, with no empty lines under a Japanese one and nothing cut off with `…` 〔未訳〕

## 9. Uncommitted changes

- [ ] **9.1** Look at the top of the graph → A row `Uncommitted: N files (+a −d)` in italics above the newest commit, with a dashed dot on the lane of the worktree's branch, author `—`, the time the change was last written and the hash `WIP`; the main worktree's change is on top when it was written last 〔未訳〕
- [ ] **9.2** Edit a file in the worktree with michi in front → Within about a second the row's counts and time follow without a key; the selection stays on the row it was on 〔未訳〕
- [ ] **9.3** Select the WIP row → The detail says `Uncommitted changes on <branch>`, the worktree's folder in mono, when it was last written, `Read only here. Stage and commit in Changes (2).`, then the files and the diff from the worktree's `HEAD`; the new file is listed as added 〔未訳〕
- [ ] **9.4** With the WIP row selected, have the Claude there run, then stop or ask → A box under the hint: `Claude is writing.` or `Claude has stopped.` with its state (glyph and word), the tab, its title and since when; a question in amber 〔未訳〕
- [ ] **9.5** Put a file over 10 MB (or a folder over 2000 files or 50 MB) in the worktree, not ignored → The WIP row and its counts leave it out; the detail says `Untracked, too big to count: <path>` (three at most, then `and N more`); `git count-objects -vH` in the repository does not grow by its size; no `git.exe` keeps running after michi is closed 〔未訳〕
- [ ] **9.6** Make the worktree clean again → The WIP row goes within about a second; a worktree whose only change is a big untracked folder has no WIP row 〔未訳〕
- [ ] **9.7** Look at `git status` and the index after michi read the change → Nothing was staged; `git status` shows the same as before 〔未訳〕

## 10. Worktree table

- [ ] **10.1** Press `3` → `Reading the worktrees…`, then a title `<repo> · N worktrees` with `Read again` and `New work` (`N`) at the right, and a table: `Branch · folder`, `From main`, `Uncommitted`, `Unseen`, `Conflicts ahead`, and buttons; the main worktree on top, each row with its branch in its lane colour and its folder under it 〔未訳〕
- [ ] **10.2** Run a Claude in a worktree from tsumugi, then quit tsumugi → The title adds ` · Claude N` and a `Claude (tsumugi)` column shows the state chip in bold, its note (two rows at most), `Tab N · <title>` with `o` and since when; rows with none say `No Claude running`; within a few seconds of quitting tsumugi the column goes 〔未訳〕
- [ ] **10.3** Look at the `From main` column → `↑a ↓b` against main, and under it the branch against its upstream: `Same as origin/x`, `N not pushed to origin/x`, `origin/x is N ahead`, `Diverged from origin/x (↑a ↓b)`, `Not pushed yet`; the main row reads against `origin/main` 〔未訳〕
- [ ] **10.4** Look at the `Uncommitted` column; then put a folder over 2000 files in a worktree, not ignored → `N files`, `+a −d` in green and red and `written at <time>`, or `None`; the big folder adds `Too big to count: <path>` and is not counted 〔未訳〕
- [ ] **10.5** Look at the locked worktree with no branch → `No branch, at <hash>` and a `Locked` mark; a worktree whose folder was deleted says `Its folder is gone (git can prune it)` 〔未訳〕
- [ ] **10.6** Press `j` / `k`, the arrows, PageDown / PageUp, Home / End, and click a row → The selection moves (10 rows a page) and the table scrolls to keep it in view; the wheel scrolls the table with no scrollbar 〔未訳〕
- [ ] **10.7** Press Enter on a row with changes, and double-click a clean row → The graph opens with the row's `Uncommitted` row selected, or its branch's commit 〔未訳〕
- [ ] **10.8** Press `d`, `m`, `x`, `o`, `p` and click `PR` → A toast at the foot names the work and says it comes in a later stage; `New work` gives the same toast as `N`; the main row shows `Pull` and `Show in the graph` instead, and has no `Remove` 〔未訳〕
- [ ] **10.9** Click `Read again` → The toast `Reading the worktrees again…`, and the table follows a change made outside (a new commit, a new worktree) 〔未訳〕
- [ ] **10.10** Narrow the window below 960 wide → Each worktree becomes a card: the branch and folder, then the Claude, `From main`, `Uncommitted`, `Unseen` and `Conflicts ahead` lines, then the buttons, which wrap; still readable at the smallest width 〔未訳〕
- [ ] **10.11** Look at the foot, and switch to the light theme → Only when the Claude column shows, a line on how often it reads tsumugi; colours stay readable in both themes 〔未訳〕
- [ ] **10.12** With a Claude running in a worktree, select its row and press `o`; on a row without a Claude; press `o` in the graph → tsumugi goes to that Claude's tab (waiting or done one first) and a toast `Went to the Claude tab N in tsumugi`; without one a toast `No Claude is running in <worktree>` 〔未訳〕
- [ ] **10.13** Select a linked worktree with uncommitted files and a Claude, press `x`; Esc; once more and `y`; on the main worktree and on this window's own → A band `Remove the worktree …? The folder goes; the branch … stays.` naming the Claude tab(s), the uncommitted files and the unpushed commits; Esc removes nothing; yes closes the tabs, keeps the change under `refs/michi/backup/` and removes the folder (the branch stays; a toast names the backup); main and the window's own say they stay 〔未訳〕
- [ ] **10.14** In the graph or the worktree table press `N`; type a branch, pick the base with Up/Down, type an instruction, raise the count to 2; Ctrl+Enter. Once with an empty branch; once with Esc → A form with the branch (default `tsumugi/MMDD-HHMM`), the base chips, the instruction, the count, the folder and the `tsumugi new …` line as it will run; Ctrl+Enter opens a tsumugi tab with a new worktree and Claude per count (`-1`, `-2` on the branch) and a toast `Started N`; an empty branch says so and stays; Esc closes it and starts nothing 〔未訳〕
- [ ] **10.15** In the worktree table select a worktree on a branch pushed to GitHub, press `p`; on a detached one; on a branch whose remote is not GitHub → The browser opens `https://github.com/<owner>/<repo>/pull/new/<branch>` and a toast says so; the detached one says it is not on a branch; the other says the remote is not on GitHub 〔未訳〕
- [ ] **10.16** In the worktree table select a worktree with 3+ commits since main, press `r`; move one with `J`/`K`, fold one with `s` and another with `f`, reword one with `r` (edit, Ctrl+Enter), drop one with `d`; Enter, then yes. Once with nothing changed; once with the first commit folded; once with a clash (two commits touching the same lines, swapped); once with uncommitted changes in the worktree → The commits are listed oldest first with pick/squash/fixup/reword/drop, a dropped one struck through; Enter with nothing changed or the first commit folded says so; the band names the counts and refs/michi/backup/; after yes the log is as the list said and a toast names the backup; a clash stops, the 4 view shows it and a toast says so; uncommitted changes refuse with a toast 〔未訳〕
- [ ] **10.17** In the worktree table select a branch behind main, press `M`, `y`; then `M` on main, on a detached row, during a merge, and on a branch whose rebase clashes → The band names the branch, main and the backup; after `y` the branch sits on main and the toast names the backup; main says there is nothing to rebase onto, the others refuse with their reason; a clash is called off and the old tip is kept 〔未訳〕

## 11. Conflict prediction

- [ ] **11.1** Press `1` and look at the two worktrees' head commits → Each has a red `⚠ Conflict ahead` pill after its names; hovering says `Would conflict with <branch> in N places (<files>)`, one line per worktree it clashes with 〔未訳〕
- [ ] **11.2** Look at the third worktree's `Uncommitted` row → A dashed yellow `⚠ Conflict ahead` pill after the counts; hovering says `Counting the uncommitted changes, would conflict with <branch> in N places (<files>)` 〔未訳〕
- [ ] **11.3** Press `3` and look at the `Conflicts ahead` column → Red `⚠ <branch>: N places` with the files under it, one for each worktree it clashes with; yellow with `(uncommitted)` for the third; `None` for a worktree that clashes with nothing 〔未訳〕
- [ ] **11.4** Look at the box under the table → `Conflicts ahead (uncommitted changes counted)`, a table of the worktrees' short names both ways: `—` against itself, `·` for a clean pair, the number of places in red, or in yellow with a dashed frame for the uncommitted one; the hint under it and the foot about `git merge-tree` 〔未訳〕
- [ ] **11.5** Hover a number, then click it, then click it again → The tip lists each file with its places; the click frames the cell and the note reads `<a> × <b>: <file> N places. ` with what to expect when they go into main (or, for the yellow one, that committing turns the frame solid); the second click puts the hint back 〔未訳〕
- [ ] **11.6** Commit the third worktree's change; then undo the conflicting edit in one of the first two and commit → Within about a second with michi in front, the yellow pill and cell turn red; then the red ones go, and the column says `None` 〔未訳〕
- [ ] **11.7** Narrow the window below 960 wide, then to 1000 wide; switch to the light theme → As cards, each has a `Conflicts ahead` line and the box takes the full width; at 1000 wide the table's columns shrink but nothing overlaps; red and yellow stay readable in light 〔未訳〕
- [ ] **11.8** Open a repository with only one worktree, and look at `git status`, the branches and `git count-objects -vH` after a minute → The box says `Only one worktree: nothing to pair yet.`; nothing was staged, no branch moved, and the objects do not grow by more than the temporary trees; no `git.exe` keeps running after michi is closed 〔未訳〕
- [ ] **11.9** Make a local branch no worktree is on that changes the same lines as main (in the main worktree: `git switch -c topic main~1`, edit and commit, `git switch main`); press `1` → The branch's head has the red `⚠ Conflict ahead` pill; hovering says `Would conflict with main in N places (<files>)`; main's head gets no pill for it, and the branch is not in the table of pairs under `3`. A branch already merged into main has no pill 〔未訳〕

## 12. Watching and unseen commits

- [ ] **12.1** Commit in the worktree on a branch → Within about a second the history is read again without going blank: the new commit shows on its branch with a blue dot before its subject, which is bold; the selection stays on the commit it was on 〔未訳〕
- [ ] **12.2** Press `3` → The worktree's `Unseen` column says `● 1 new` (the blue dot); the main worktree says `—` and the others `None`; beside the conflicts box, `Unseen changes (1)` lists the commit: short hash, blue dot, the subject in bold, `<short branch> · <time>` at the right, and the foot `The commits the worktrees have gained since you last looked. Selecting one in the graph marks it seen.` 〔未訳〕
- [ ] **12.3** Hover the commit in the list, then click it → The line lights and the pointer becomes a hand; the click opens the graph with that commit selected and its dot and bold go; back in `3`, the column says `None` and the box `All seen.` 〔未訳〕
- [ ] **12.4** Commit twice more; in the graph, move onto one of them with `j` / `k`, and click the other → Each loses its dot once selected; the commit selected when the graph first opens, or that the selection follows when rows come in, does not count as seen 〔未訳〕
- [ ] **12.5** Make 13 commits in the worktree and press `3` → The box shows 12, newest first, and `… 1 more`; a long subject is cut with `…` before the branch and time 〔未訳〕
- [ ] **12.6** Quit michi with commits unseen, commit once more, and start michi again → The commits still unseen keep their dot, and the one made while michi was closed has one too; the file above lists the branch with the commits seen 〔未訳〕
- [ ] **12.7** Narrow the window below 960 wide → The `Unseen changes` box goes under the conflicts box at the full width; the cards have an `Unseen` line 〔未訳〕
- [ ] **12.8** Put another window in front, commit in a worktree, wait, then bring michi back → Nothing is read while michi is behind (no `git.exe` in Task Manager); when it comes back the history and the table follow at once 〔未訳〕
- [ ] **12.9** Run `cargo build` (or write many files) in an ignored folder of a worktree, and run `git status` there, with michi in front → michi reads nothing for them: no `git.exe` keeps coming up in Task Manager, and the table does not change; a `git status` does not start a read that starts another 〔未訳〕
- [ ] **12.10** Open a repository on a network share (`\\server\share` or a mapped drive) → Changes there show within about 5 s while michi is in front; nothing is read while it is behind 〔未訳〕
- [ ] **12.11** Switch to the light theme → The blue dots and bold subjects in the graph, the column and the box stay readable 〔未訳〕

## 13. Branch review

- [~] **13.1** Press `3`, select the worktree and press `d` (or click `Diff d` on it) → The review takes the view: the branch in bold, the switches `Uncommitted u` (lit) and `Since last look s`, `Seen 0 / 5` and `Back Esc`; under them, in red, `Would conflict with main in 1 places (lib.rs)`; the files on the left with a box, the letter (R A M in their colours), the path and `+n −n` (`Binary` for the image); the first file's diff on the right; the key bar lists `j k`, `Space`, `u`, `s`, `Enter`, `PageDown PageUp`, `Esc` 〔未訳〕
- [ ] **13.2** Press `j` / `k`; click a file's name; click another file's box → The selection and the diff follow the keys and the click; the click on a box ticks that file without moving the selection 〔未訳〕
- [ ] **13.3** Press Space on a file, then on a ticked one → The first is ticked, the selection goes on to the next file not ticked and `Seen` counts up; Space on a ticked file takes the tick off 〔未訳〕
- [ ] **13.4** Press Esc, change and save a ticked file in the worktree, wait a second and press `d` → The other ticks are kept; the changed file is not ticked any more (a tick is for the content it was given to); the file above lists the branch 〔未訳〕
- [ ] **13.5** Press `u`, then `u` again → The switch goes dark and the file changed only in the worktree leaves the list, `Seen n / m` with it; the second `u` brings it back 〔未訳〕
- [ ] **13.6** On a branch never looked over, press `s`; then Esc, commit once more in the worktree, press `d` and `s` → First the toast `Not looked over before: all of it is new`; then the switch is lit, the title reads `since the last look (<short hash>)` and only the files changed since show (and the uncommitted ones); with nothing new, `No file changed since the last look` 〔未訳〕
- [ ] **13.7** Press Enter, then Esc twice → Enter widens the diff side by side (the switch reads `Back to one column Esc`), the first Esc puts it back, the second closes the review on the worktree table 〔未訳〕
- [ ] **13.8** Select a file with a long diff; roll the wheel over it, press PageDown and PageUp; select another file → The diff scrolls (no scrollbar); the other file starts at its top 〔未訳〕
- [ ] **13.9** Press `d` on a worktree with a detached HEAD; on a branch with nothing from main → The toast `This worktree is on no branch`; the review reads `No file changed from main` 〔未訳〕
- [ ] **13.10** Commit in the worktree while the review is open; switch the worktree to another branch from a terminal → The list follows the new commit within about a second, keeping the ticks; on another branch the list stays as it was and nothing is read 〔未訳〕
- [ ] **13.11** Narrow the window to its smallest; switch to the light theme → The branch name is cut with `…`, the files keep their counts and the key bar folds what does not fit; the colours stay readable in both themes 〔未訳〕
- [ ] **13.12** In the review of a branch behind main press `m`; Esc; once more and `y`; on `main`'s own review → A band `Merge main into <branch> in <worktree>?`; Esc merges nothing; yes merges (a toast; a clash opens the `4` view); on main a toast says it is main itself 〔未訳〕
- [ ] **13.13** In the review of a branch with a Claude in its worktree: `J` `K` over a diff, click a line, `n`, type a note, Ctrl+Enter; on a second line another; Esc and `d` again; `g` → The cursor line is tinted blue and scrolls into sight; a line with a note has an amber bar on its left and the note shows under the diff; the top bar reads `Notes 2`; the notes are still there after closing the review; `g` toasts that 2 notes were sent, the Claude gets one line naming a file in the temp folder (`michi-notes`) and does not commit, and the notes are gone from the review 〔未訳〕
- [ ] **13.14** `n` on a line that has a note, empty it, Ctrl+Enter; `g` with no notes; Esc while writing → The note is taken away; a toast says there are no notes yet; Esc leaves the note as it was 〔未訳〕

## 14. Changes: stage and commit

- [ ] **14.1** Press `2` in the set-up worktree → The top bar: the worktrees as chips (the window's own lit, a green ● on one where a Claude is writing), `[ ]` after them, `Amend A` on the right; the files on the left under `Staged` and `Not staged`, each with its letter (M A D R `?` in their colours), the path and `+n −n`, the new file last with `?`; under the list the message box with `What changed, and why` and `Commit`; the selected file's diff on the right; the `2` tab reads `2 Changes` and the number of files listed (a file both staged and changed counted once); the key bar lists `Space`, `c`, `l h`, `a`, `j k`, `v`, `A`, `[ ]`, `Enter`, `Esc` 〔未訳〕
- [ ] **14.2** Press `j` / `k`; Space on a file not staged, then on a staged one; `a`, then `a` again; click a file, double-click another → Space moves the file to `Staged` (and back out on a staged one) and the selection stays on that file; the first `a` stages everything, the second unstages everything; a click selects, a double-click stages or unstages; `git status` in a terminal agrees each time 〔未訳〕
- [~] **14.3** On a changed file with two hunks press `l`, `j`, `v`, `j`, Space; then `a`; then `h` → `l` puts an accent strip on the first changed line and the title reads `Space in or out · v range · a hunk · s split · d throw away · h back`; `j` walks the changed lines only; `v` picks a range (tinted blue); Space stages just those lines (`git diff --cached` shows them, `git diff` the rest); `a` stages the hunk under the cursor; `h` goes back to the files 〔未訳〕
- [~] **14.4** Press `l` on the new file, on a binary file and on a renamed file → The toast `This file goes in whole: its lines cannot be picked`; the diff says the same; Space stages the whole file 〔未訳〕
- [ ] **14.5** Press `c`, write a two-line message (`j`, `k`, Space type as text), press Ctrl+Enter → The box takes the keys; the commit is made (toast `Committed <hash>`), the box and the staged list empty, the graph (`1`) shows the commit with the message; Esc in the box only leaves it 〔未訳〕
- [ ] **14.6** Press Ctrl+Enter with nothing staged; then stage a file and press Ctrl+Enter with an empty box → The toasts `Nothing staged to commit` and `Write a commit message first (c)`; nothing is written 〔未訳〕
- [ ] **14.7** On a commit not pushed press `A`, change the message, Ctrl+Enter → The switch lights and the box fills with the head's message (the title reads `Message of the commit amended`, the button `Amend`); the head is replaced: one commit, not two; `A` again before writing empties the box 〔未訳〕
- [ ] **14.8** On a pushed commit press `A`, then Ctrl+Enter; Esc; Ctrl+Enter and `y` → A band under the top bar: `The last commit is pushed: amending it rewrites history others may have.` with `Go ahead y` and `Not now Esc`; Esc leaves the commit as it was; `y` amends it 〔未訳〕
- [ ] **14.9** Start a Claude in a worktree from tsumugi; in michi open that worktree in `2` and press Space; press `x`, Esc, Space, `y`, then Space on another file → The band `A Claude is writing in this worktree. Write beside it?`; `x` does nothing while it waits; Esc says no and nothing is staged; `y` stages; the next Space does not ask again until that Claude stops or another worktree is shown 〔未訳〕
- [ ] **14.10** Write a message, press `]` and `[`; click another worktree's chip; open a repository with one worktree and press `]` → Each worktree's files are read anew and the chip lights; the message written stays with its worktree; with one worktree, the toast `There is only one worktree` 〔未訳〕
- [~] **14.11** Press Enter, then Esc; pick lines and press Enter again → The diff spreads side by side, old on the left and new on the right; the picked lines are tinted on their side only; Esc goes back to one column (on the lines, the first Esc leaves the lines) 〔未訳〕
- [ ] **14.12** With `2` open, save a file in the worktree from an editor; let a Claude commit there → The list follows within about a second and keeps the selection on the same file; staged lines stay staged 〔未訳〕
- [ ] **14.13** Leave `2` for `1`; then make the worktree clean → The `2` tab still counts the uncommitted files of the worktree last shown; with none, no number 〔未訳〕
- [ ] **14.14** Add a pre-commit hook that prints a line and exits 1, then commit → The toast `Could not write: …` with the hook's words for 8 seconds; the staged list and the message stay 〔未訳〕
- [ ] **14.15** Narrow the window to its smallest; switch to the light theme → The list keeps at least 240 px, the chips end in `…`, the key bar folds what does not fit; the tints, the band and the letters stay readable in both themes 〔未訳〕
- [ ] **14.16** Add a pre-commit hook that prints five lines (some in colour, `printf '\033[31m…\033[0m'`) to stdout and stderr and exits 1; commit; Esc, Esc; commit again, then `j`; commit again, unstage and stage a file → The toast `Could not write: <the first line>`; in place of the diff the red title `Could not commit: all git and the hooks said` with `Close Esc` at the right and all five lines below in a fixed-width font, without colour codes; the wheel and PageDown scroll a long one (no scrollbar); the first Esc only leaves the message box, the second closes it and the diff is back; `j` (another file) and a write that goes through close it too; a write that fails with one line only shows the toast 〔未訳〕
- [ ] **14.17** On a hunk with two changes a few lines apart press `s`; `j`, `a`; Space; save the file from an editor; `s` again; on a file whose hunks each have one change press `s` → `s` goes into the lines and each hunk is cut into one piece per run of changes, a `` line before each; `a` picks the piece under the cursor and Space stages only it (`git diff --cached` shows that change alone); after the save the file stays cut; `s` again joins the hunks back; on the other file the toast `Nothing to split: each hunk has one run of changes` 〔未訳〕

## 15. Fetch, pull and push

- [ ] **15.1** Press `f` (or click `fetch f` in the tool band of `1`); press `f` again; restart michi → While it runs the key bar's right side reads, in amber, `fetch · <step> n%` and `Esc stops`; then the toast `Fetched: n refs moved`, the graph shows the moved remote branches and the key bar reads `⟳ fetch just now` (later `n min ago`); the second `f` says `Fetched: nothing new`; after the restart the key bar still gives the time of the last fetch 〔未訳〕
- [ ] **15.2** On a branch behind its upstream press `p`; press `p` again; on a branch with no upstream press `p`; on a branch that went its own way press `p` → `Pulled n commits from origin/main` and the graph moves the branch; then `Already up to date with origin/main`; `main has no upstream to pull from: P pushes it`; `origin/main went its own way (↑a ↓b): a pull only goes forward, so nothing was merged` and nothing moves (pull is fast-forward only) 〔未訳〕
- [~] **15.3** Commit and press `P` and `y` on the band (112.1); press `P` again; on a new branch with no upstream press `P` → `Pushed to origin/main`; `origin/main has it already`; `Pushed to origin/feat, now its upstream` and `git status -sb` shows `feat...origin/feat` 〔未訳〕
- [ ] **15.4** Let a second clone push first, commit here and press `P` without fetching → The toast `origin/main refused the push: [rejected] … (fetch first). f fetches its commits; then F pushes over them, asking first`; nothing is rewritten 〔未訳〕
- [~] **15.5** After 15.4 press `f`, then `P`; then `F`; Esc; `F` again and `y` → `P` only says `origin/main has n commits this branch does not: F pushes over them, asking first` and pushes nothing; `F` shows a red band (112.3) whose note reads `origin/main has n commits this branch does not. Push over them? What origin/main has now is kept under refs/michi/backup/ first.` with `Go ahead y` and `Not now Esc`; Esc pushes nothing; `y` pushes with `--force-with-lease` and says `Pushed over origin/main; what it had is kept in michi/backup/<time>`; that ref points at the remote's old tip and `git reflog refs/michi/backup/<time>` reads `michi: push --force-with-lease over origin/main` 〔未訳〕
- [ ] **15.6** On a branch only behind its upstream press `P`, then `F` → Both say `Nothing to push: origin/main is n ahead (p pulls)`; no band 〔未訳〕
- [~] **15.7** Add a remote that does not answer (`git remote add slow https://10.255.255.1/x.git`), press `f`, then `f` again, then Esc → The key bar shows `fetch` and `Esc stops` in amber; the second `f` says `fetch is running: Esc stops it`; Esc says `Stopping fetch…`, then `fetch stopped`; afterwards no `git`, `git-remote-https` process is left (Task Manager); remove the remote after 〔未訳〕
- [ ] **15.8** In `1` press `p` / `P`; in `3` select another worktree's row and press `P`; click `pull p` on a card; in `2` show another worktree and press `p` → `1` works on the worktree michi was opened in; `3` on the selected worktree; `2` on the worktree shown; a band (15.5) shows over `2` and `3` too 〔未訳〕
- [ ] **15.9** Start a Claude in a worktree from tsumugi; press `p` for that worktree; Esc; `p` and `y` → The band `A Claude is working in <worktree>. Pull there anyway?`; Esc pulls nothing; `y` pulls 〔未訳〕
- [ ] **15.10** Fetch from an https remote that needs a sign-in; fetch from an ssh remote whose key has a passphrase and no ssh-agent → The Git Credential Manager window comes up as it does in a terminal and the fetch goes on after it; the ssh one fails at once (no prompt hangs) with `fetch failed: … — start ssh-agent and ssh-add your key` 〔未訳〕
- [ ] **15.11** Narrow the window to its smallest with a band shown and a fetch running; switch to the light theme → The band wraps its text and grows, its keys stay whole; the key bar folds its keys before the fetch progress or age; both stay readable in both themes 〔未訳〕
- [ ] **15.12** In a clone with two remotes and no `origin` (`git remote rename origin up`, `git remote add mine <path>`), press `P` on a branch with unpushed commits; type `mi`, Enter, `y` on the band (112.1); then `P` again on another branch; Esc → The box `Push to: pick a remote` (`pushing <worktree>`) lists `mine` and `up`; the keys read `Enter pushes to it (and sets the upstream) · Esc closes`; `mi` leaves `mine`; Enter closes it and pushes there (`Pushed … to mine`, upstream becomes `mine/<branch>`); Esc pushes nothing. With `origin` or a `pushRemote` set no box shows 〔未訳〕

## 16. Branches, tags and revert

- [ ] **16.1** In `1` select a commit and press `b` (or click `Branch b` in the tool band); type part of a name; ↑ ↓; move the pointer over the rows; Esc; `b` and click outside; press `b` and type a name at once → A palette under the top bar: the box with `Branch: a name switches to it, a new one is made`, under it `<worktree> · at <commit>`, the local branches (with `here`, `in <worktree>`, `↑a ↓b`) then the remote branches no local one has, each with its commit at the right; the foot reads `Enter switches · Shift+Enter makes and switches · Del deletes · Esc closes`; typing narrows them, the typed letters lit and the best first; the arrows and the pointer move the selection (a resting pointer does not); Esc and a click outside close it; the letters typed right after `b` land in the box 〔未訳〕
- [ ] **16.2** Type a branch name with the Japanese IME and confirm it → The composition shows in the box and the confirmed name narrows the rows; nothing is lost 〔未訳〕
- [~] **16.3** Type a new name and Enter; `b` again, another new name and Shift+Enter → The last row reads `<name>  make it at <commit>`; Enter: `Made branch <name> at <commit>`, the graph shows its chip and the worktree stays on its branch; Shift+Enter: `Made branch <name>; <worktree> is on it` and `git branch --show-current` there gives it 〔未訳〕
- [~] **16.4** Pick a branch no worktree has and Enter; type the name of a remote branch with no local one (`origin/x`) and Enter; type it without `origin/` → `<worktree> is on <name> now`; `Made <name> following origin/<name>; <worktree> is on it` and `git status -sb` shows `<name>...origin/<name>`; the name without `origin/` lists `origin/<name>` and no `make it` row 〔未訳〕
- [~] **16.5** Pick the branch this worktree is on; pick one checked out in another worktree; in a worktree in the middle of a merge (`git merge` that clashes) pick a branch and Enter, then type a new name and Enter → `<worktree> is on <name> already`; `<name> is checked out in <worktree>: a branch is in one worktree at a time`; `Branches can be made. Switching is stopped in <worktree>: it is in the middle of a merge.` for 8 seconds; the palette stays open each time; the new name is made; `git merge --abort` after 〔未訳〕
- [ ] **16.6** Select another worktree's uncommitted row in `1` and press `b`, pick a branch and Enter; select a commit and press `b` → With the uncommitted row the palette reads `<that worktree> · at <its HEAD>` and switches that worktree; with a commit, the window's own worktree and that commit 〔未訳〕
- [ ] **16.7** Start a Claude in a worktree from tsumugi; select its uncommitted row, `b`, pick a branch and Enter; Esc; again and `y` → The band `A Claude is working in <worktree>. Switch it to <name> anyway?`; Esc switches nothing; `y` switches 〔未訳〕
- [ ] **16.8** Select a branch no worktree has, whose commits no other branch or tag has, and press Delete; Esc; again and `y`; delete a branch merged into main the same way → The band `Delete branch <name>? When no other branch or tag has its commits, they are kept under refs/michi/backup/ first.`; Esc deletes nothing; `y`: `Deleted branch <name>; its commits are kept in michi/backup/<time>` and `git log refs/michi/backup/<time>` shows them; the merged one: `Deleted branch <name>` and no new backup 〔未訳〕
- [~] **16.9** Press Delete on the branch checked out here, on one checked out in another worktree, on a remote branch → `<name> is checked out in <worktree>: switch it away first`; `Remote branches are not deleted here: git push origin --delete <name> does it` for 8 seconds; nothing is deleted 〔未訳〕
- [~] **16.10** Select a commit and press `T`; type `v0.1 first release` and Enter; `T`, `v0.2` and Enter → The box reads `Tag: a name, then its message if any`, the tags are listed, the last row reads `v0.1  on <commit>`, the foot `Enter makes or shows · Shift+Enter makes and pushes · Del deletes · Esc closes`; `Made tag v0.1 at <commit>` and the graph shows `◆ v0.1`; `git cat-file -t v0.1` gives `tag` and its message is `first release`; `v0.2` is a light tag (`git cat-file -t` gives `commit`) 〔未訳〕
- [ ] **16.11** `T`, pick `v0.1` and Enter; `T`, pick it and Shift+Enter; `T`, type `v0.3` and Shift+Enter; during a fetch (15.7) `T`, `v0.4` and Shift+Enter → Enter selects its commit in the graph; Shift+Enter: `Pushed v0.1 to origin` and `git ls-remote --tags origin` lists it; `v0.3` is made and pushed; during the fetch `fetch is running: Esc stops it` and nothing is made 〔未訳〕
- [~] **16.12** `T`, pick `v0.1` and press Delete; Esc; again and `y` → The band `Delete tag v0.1? What it points at is kept under refs/michi/backup/ first.`; Esc deletes nothing; `y`: `Deleted tag v0.1; kept in michi/backup/<time>` and `git cat-file -t refs/michi/backup/<time>` gives `tag`; the remote keeps its tag, so the next fetch brings it back 〔未訳〕
- [~] **16.13** Select a commit on the current branch and press `t`; Esc; `t` and `y` → The band `Revert <id> “<subject>” with a new commit on <branch> in <worktree>?` (`A Claude is working there.` added when one is); Esc does nothing; `y`: `Reverted <id> on <worktree>: <new>` and the graph shows `Revert "<subject>"` on top 〔未訳〕
- [~] **16.14** Select a merge commit and press `t`; press `1` → The band `<id> “<subject>” is a merge. Revert it with a new commit on <branch> in <worktree>, keeping which parent's side?` with `keep parent 1 1`, `keep parent 2 2` and `Not now Esc`; `1` reverts it keeping the first parent's side: `Reverted <id> on <worktree>: <new>` 〔未訳〕
- [~] **16.15** Revert a commit whose lines a later commit changed; revert a commit already reverted; in a worktree in the middle of a merge press `t` → `Reverting <id> clashed in <files>, so it was called off: git revert <id> in a terminal lets you resolve it` and `git status` is clean; `Nothing to revert: <id> is undone already`; `<worktree> is in the middle of a merge: finish it first` 〔未訳〕
- [ ] **16.16** Look at the tool band of `1`; press `?`; narrow the window to its smallest with the palette open; switch to the light theme → The tool band reads `Branch b` (`ブランチ b` in Japanese); help lists `b Make or switch a branch`, `T Tag the commit`, `t Revert the commit (asks first)`; the palette narrows with the window, long names end in `…` and the commits stay at the right; readable in both themes 〔未訳〕

## 17. Stash

- [~] **17.1** In `1` press `s` (or click `Stash s` in the tool band); Esc; `s` and click outside → A palette under the top bar: the box with `Stash: a message to put the changes away, or pick one`, under it `in <worktree>`, the first row `Put the changes away` in the accent colour and selected, then the stashes newest first, each with `stash@{k}` and its hash at the right; the foot reads `Enter puts away or applies · Shift+Enter pops · Del drops · Esc closes`; Esc and a click outside close it 〔未訳〕
- [~] **17.2** `s`, type `half done` and Enter → The first row reads `Put the changes away as “half done”` and the stashes below narrow to those matching it; Enter: `Put the changes of <worktree> away: <hash>`; the uncommitted row goes from the graph, `git status` is clean (the untracked file gone too) and `git stash list` shows `On <branch>: half done` 〔未訳〕
- [~] **17.3** `s` and ↓ to a stash → The detail shows the stash: its message, hash and the commit it was made on as its parent, its files (the untracked one as `A`) and their diff 〔未訳〕
- [~] **17.4** On a stash press Enter; then `git checkout -- . ; git clean -fd` in a terminal; `s`, the same stash and Shift+Enter → Enter: `Put the stash back on <worktree>; it stays in the list`, the changes are back, the untracked file too, and the stash is still listed; Shift+Enter: `Put the stash back on <worktree> and dropped it` and it is gone from the palette and the side column 〔未訳〕
- [~] **17.5** With the stash's file changed again, Enter on the stash; then `git checkout -- .`, commit a change to the same lines and Shift+Enter on it → The first: `stash apply failed: error: Your local changes to the following files would be overwritten by merge: <file>` and nothing changes; the second: `The stash clashed on <worktree> in <file>: resolve the marks there and git add them; the stash is kept`, `git status` shows `UU <file>` with the marks in it and the stash is still listed 〔未訳〕
- [~] **17.6** On a stash press Delete; Esc; again and `y`; Delete on the first row → The band `Drop stash <hash> “<message>”? It is kept under refs/michi/backup/ first.`; Esc drops nothing; `y`: `Dropped stash <hash>; kept in michi/backup/<time>`, `git stash list` no longer has it and `git rev-parse refs/michi/backup/<time>` gives its hash; Delete on the first row does nothing 〔未訳〕
- [ ] **17.7** In a worktree in the middle of a merge, `s` and Enter on the first row, then on a stash; with a Claude working in the worktree (from tsumugi), `s` and Enter; Esc; again and `y`; the same on a stash → `Stashes are stopped in <worktree>: it is in the middle of a merge.` for 8 seconds and the palette stays; with the Claude the band `A Claude is working in <worktree>. Put its changes away anyway?` (`Put the stash back there anyway?` on a stash); Esc does nothing; `y` does it 〔未訳〕
- [~] **17.8** Select another worktree's uncommitted row in `1` and press `s`, Enter; with no changes anywhere press `s` and Enter → The palette reads `in <that worktree>` and puts that worktree's changes away; with none: `<worktree> has no changes to put away` 〔未訳〕
- [~] **17.9** Click a stash in the side column; Esc; click it again and press `j` → The detail shows the stash as in 17.3 and its row is lit in blue; Esc brings back the selected commit and the row goes dark; `j` does too 〔未訳〕
- [ ] **17.10** Press `?`; narrow the window to its smallest with the palette open; switch to the light theme → Help lists `s Stash`; the palette narrows with the window, long messages end in `…` and the hashes stay at the right; readable in both themes 〔未訳〕

## 18. Discard

- [~] **18.1** In `2` on a changed tracked file in Unstaged press `d`; Esc; `d` again and `y` → The band `<file> の変更を捨てますか？先に refs/michi/backup/ に退避します。` (`Throw away the changes to <file>? They are kept under refs/michi/backup/ first.`) with `進める y` / `やめる Esc`; Esc leaves the file; `y`: the toast `<file> の変更を捨てました。michi/backup/<time> に退避しました` for 10 seconds, the file leaves Unstaged and `git show refs/michi/backup/<time>:<file>` has the change 〔未訳〕
- [~] **18.2** On the file with two changes press `l`, `v`, `j` over the first change and `d`, `y` → The band `<file> の 2 行の変更を捨てますか？…`; `y`: `<file> の 2 行を捨てました。…`, the first change is gone, the second stays and the cursor stays in the lines; the backup has the file with both changes; `git diff --cached` is as it was 〔未訳〕
- [~] **18.3** `h`, on `u/new.txt` press `d`, `y` → The band `u/new.txt を消しますか？Git が追っていないファイルです。…`; `y`: `u/new.txt を消しました。…`, the file and the empty folder `u` are gone and `git show refs/michi/backup/<time>:u/new.txt` has it 〔未訳〕
- [~] **18.4** Stage a file (Space) and press `d` on it in Staged → `ステージした変更は捨てません。先に外してください（Space）` and nothing changes 〔未訳〕
- [ ] **18.5** With a Claude working in the worktree (from tsumugi), `d` on a file → The band carries both the question for the file and `この worktree では Claude が書いている途中です。横から書き込みますか？`; Esc does nothing; `y` throws the changes away 〔未訳〕
- [ ] **18.6** Press `?`; narrow the window to its smallest with the band up; switch to the light theme → Help lists `d 変更を捨てる（先に確かめる）`; the band's text wraps or ends in `…` with `y` and `Esc` still visible; readable in both themes 〔未訳〕

## 19. Clean up

- [~] **19.1** In `3` press `c` → A list over the table: the title `Clean up: branches merged into main or gone from their remote, and worktrees whose folder is gone`, under it `“Gone from its remote” is as of the last fetch: <age>`; rows with a box, none ticked: `done` `merged into main` with its hash, `feat` `merged into main · its worktree r-feat goes too`, `squashed` `gone from its remote (its tip is kept first)`, `r-old` `a worktree whose folder is gone`; `busy` grey without a box and `has uncommitted changes` in yellow; `main`, `old` and `wip` are not listed; the foot `0 picked · Space picks · a picks all · Enter cleans up · Esc closes` 〔未訳〕
- [ ] **19.2** `j` / `k`; Space on a row, on `busy`; `a`, `a` again; click a row; Esc; `c` and click outside → Space ticks the row and the foot counts it; on `busy` nothing happens; the first `a` ticks every row that can go, the second lets them all go; a click ticks the row; Esc and a click outside close the list 〔未訳〕
- [ ] **19.3** `c`, Enter with nothing ticked → The toast `Pick the rows to clean up with Space first`; the list stays 〔未訳〕
- [ ] **19.4** `a`, Enter; Esc → The band `Clean up 4: done, feat, squashed and 1 more? The worktree folders go too, with the files git ignores in them: r-feat. Commits no other branch has are kept under refs/michi/backup/ first.` with `Go ahead y` / `Not now Esc`; Esc goes back to the list and nothing is gone 〔未訳〕
- [~] **19.5** Enter, `y` → The toast `Cleaned up 4: done, feat, squashed and 1 more. Unmerged commits are kept in michi/backup/<time>`; the list closes; the table has `main` and `busy` left and the top bar's chip reads `worktree 2`; `git branch` no longer has `done`, `feat` and `squashed`, `git worktree list` no longer has `r-feat` and `r-old`, the folder `r-feat` is gone, and `git log -1 refs/michi/backup/<time>` is `squashed: one` 〔未訳〕
- [ ] **19.6** Open a tsumugi tab in a merged branch's worktree; press `c` → That worktree's row is grey with `a tsumugi tab is open there`; closing the tab and pressing `c` again lists it with a box 〔未訳〕
- [ ] **19.7** Commit to a ticked branch from a terminal while its band is up, then `y` → It stays: `… Stayed: <branch> (it moved after it was listed)`; the others go 〔未訳〕
- [ ] **19.8** With nothing left to clean up press `c` → The toast `Nothing to clean up: no branch is merged into main or gone from its remote, and no worktree's folder is gone`; no list opens 〔未訳〕
- [ ] **19.9** Press `?` in `3`; narrow the window to its smallest with the list open; switch to the light theme → Help lists `c Clean up`; long rows end in `…` with the boxes and hashes still visible; readable in both themes 〔未訳〕

## 20. The command line and `michi.com` (Windows)

- [ ] **20.1** Put `michi.exe` and `michi.com` (from the Build artifact, or `michi-com.exe` copied to that name) in one folder; in PowerShell there run `$v = .\michi --version; "[$v]"` → `michi.com` answers: `[michi 0.15.0]` (the version), printed by the second statement, and the prompt comes back after it 〔未訳〕
- [ ] **20.2** Run `.\michi --help` in PowerShell, then in `cmd` → The usage lines print before the prompt comes back, in both 〔未訳〕
- [ ] **20.3** Run `.\michi --bogus`, then `.\michi C:\no-such-folder`, each followed by `$LASTEXITCODE` (`echo %ERRORLEVEL%` in `cmd`) → `michi: unknown argument` and the usage, then `2`; `michi: not a folder: C:\no-such-folder`, then `2`; no window opens 〔未訳〕
- [ ] **20.4** Run `.\michi C:\dev\some-repo` (a repository), then `.\michi` with no argument in a repository's folder → The window opens on that folder and the prompt comes back as soon as it is up, with michi still open; closing the window later prints nothing in the console 〔未訳〕
- [ ] **20.5** Run `$v = .\michi.exe --version` (the windowed program itself), wait a few seconds, then run `.\michi.exe --version` → `$v` is empty and no `panicked` message appears later (the line has nowhere to go and is let go); the second, with no `$v =`, prints the version in the console 〔未訳〕
- [ ] **20.6** In a repository's folder run `$v = .\michi` (no argument, its output taken through a pipe); once the prompt is back, run `Get-ChildItem $env:TEMP\michi-com-*`; close michi and run it again → The prompt comes back once the window is up, with michi still open; two files (`.out` and `.err`) while it runs, none after it closes 〔未訳〕
- [ ] **20.7** With `michi.com` and `michi.exe` in one folder run `$env:MICHI_KEYS_DONE="$env:TEMP\mk\done.txt"; .\michi C:\dev\some-repo --keys '2<Wait:200><State:two>3<Quit>'` (create `$env:TEMP\mk` first), then read `$env:TEMP\mk	wo.txt` and `done.txt`; then `.\michi --keys j -V` and `.\michi --keys` → The window opens, goes to the `2` view, then to `3`, and closes by itself; `two.txt` starts with `focus: changes` and has `view: 2`; `done.txt` ends with `keys: done` after `quit: yes`; `--keys j -V` prints the version and opens no window; `--keys` alone prints `michi: --keys needs a script` and the usage, then `2` 〔未訳〕

## 21. Conflicts

- [~] **21.1** Open michi on `r` → The tab `Conflicts` (`4`) carries `● 8` in red (the places left in every worktree); the title bar has `· merging`; `4` opens the view: the bar `Merging feat into main`, the pill `4 / 4 left` and the button `Call the whole merge off` 〔未訳〕
- [~] **21.2** Look at the left column → `Files that clashed 3`: `a.txt`, `b.txt`, `d.txt` with their places left; `Merged by themselves 1`: `c.txt` `both changed it, apart`; `Merge`: `feat <id> into main <id>.`, `The base is <id>, where the two parted.`, `Since <time>.` and `“Ours” is main, where you are; “theirs” is feat, coming in.`; `Other worktrees stopped 1`: `w` with its 4 〔未訳〕
- [~] **21.3** Look at the first place → `a.txt, line 3` with `● to solve`; the columns `Ours` (blue) and `Theirs` (green) with their lines and line numbers, `… N lines the same …` between places; `Result` reads `Pick with a button below, or fix it in an editor and save`; a `Hint` line; the buttons `Take ours`, `Take theirs`, `Both (ours first)`, `Both (theirs first)`, `Back to base`; the current place has a red edge 〔未訳〕
- [~] **21.4** `Show base`; `Hide base` → A `base` column with the base's lines appears between the two and goes again 〔未訳〕
- [~] **21.5** `o` on the first place; `t` on the second; `[`, `o`; on `b.txt` `b` → Each pick puts its tag (`ours`, `theirs`, `both (ours first)`) on the place, hides its buttons, turns its edge green and moves on; the pill and the tab's `●` count down and the file's count in the left column turns to a green `✓` once its places are picked, `git diff --cached --name-only` then having it; `o` on a solved place says `That place is solved already: “Start this file over” puts it back` 〔未訳〕
- [~] **21.6** On `d.txt` → `Deleted on our side, changed on theirs.` with `Delete it (as our side)` and `Take their file`; `o` deletes it (`d.txt` leaves `git status`'s unmerged files, `main` having deleted it already); `b` there says `A file that clashed whole takes one side: o or t` 〔未訳〕
- [~] **21.7** Click `Start this file over` on `a.txt` → The toast `a.txt is back as git left it; what it had is kept in michi/backup/<time>`; its places are open again with `● to solve`, the pill and the tab count up; `git show refs/michi/backup/<time>:a.txt` has the picks 〔未訳〕
- [~] **21.8** Edit a place's lines in an editor and save → Within a few seconds the place shows the tag `by hand` and counts as solved 〔未訳〕
- [~] **21.9** With places open press Ctrl+Enter → The toast `N left to solve first`; nothing is committed 〔未訳〕
- [~] **21.10** Solve the rest → The bar under the places: `All solved.`, the message box with git's message (`Merge branch 'feat'`) and `Commit`; the view scrolls to it once the last place is solved; clearing the message and pressing Ctrl+Enter says `Write the commit's message first` 〔未訳〕
- [~] **21.11** Ctrl+Enter → The toast `Committed the merge on r: <id>`; the title bar loses `· merging`; the view moves to `w`'s cherry-pick (`Cherry-picking <id> onto side`, `Call the whole cherry-pick off`) 〔未訳〕
- [~] **21.12** In `w` solve all four, Ctrl+Enter → `Committed the cherry-pick on w: <id>`; `git log -1 --format='%an %ad'` in `w` has `feat`'s author and date; with nothing left to solve the tab `Conflicts` goes and the view moves to the graph; `4` then says `Nothing clashed: the 4 view opens while a merge or the like stops on one` 〔未訳〕
- [~] **21.13** Set the two up again; click `Call the whole merge off`; `n`; click it again and `y` → A row under the bar: `Stop the merge and put main back as it was before it (git merge --abort). The files are kept in michi/backup first.` with `Keep on` / `Call it off`, wrapping in a narrow window; `n` closes it; `y`: the toast `Called the merge on r off; its files are kept in michi/backup/<time>`, `git status` in `r` is clean and the view moves to `w` 〔未訳〕
- [~] **21.14** Click `w` under `Other worktrees stopped` → The view shows `w`'s cherry-pick; the tab's `●` still counts both 〔未訳〕
- [~] **21.15** With `r` in the merge: `p`; `C` on a commit in `1`; Ctrl+Enter in `2` with a file staged; `3`, `m` on `w` while `w` is in its cherry-pick → `r is in the middle of a merge: pull waits until it is over`; `r is in the middle of a merge: finish it first`; `r is in the middle of a merge: commit it in the 4 view`; `w is in the middle of a cherry-pick: finish it first`; nothing runs; a fetch and other worktrees' pulls go on 〔未訳〕
- [~] **21.16** With nothing in progress, in `x`'s window: `C` on a `main` commit that clashes; `y` → The band `Pick <id> “<subject>” onto feat in x?` with `Go ahead y` / `Not now Esc`; `y`: `Picking <id> onto x clashed in <files>: solve it in the 4 view` and the `4` view opens on it; a commit that picks cleanly: `Picked <id> onto x: <new>`; a merge commit: the band asks against which parent, with `against parent 1 1` / `against parent 2 2` (a revert's says `keep parent 1`) 〔未訳〕
- [~] **21.17** `3`, `m` on `main`'s own row; on `x`; `y` → `r is on main itself: there is nothing to merge`; the band `Merge main into feat in x?`; `y`: `Merging main into x clashed in <files>: solve it in the 4 view` and the `4` view on `x`; when it merges cleanly: `Merged main into x: <new>` 〔未訳〕
- [ ] **21.18** `t` (revert) on a commit whose revert clashes; `y` → `Reverting <id> clashed in <files>: solve it in the 4 view`; the view on it (`Reverting <id> on main`) and Ctrl+Enter commits it 〔未訳〕
- [ ] **21.19** With a Claude working in `r` (from tsumugi), solve all and Ctrl+Enter → The band `A Claude is working in r. Commit the merge there anyway?`; Esc commits nothing; `y` commits 〔未訳〕
- [ ] **21.20** Press `?` in `4`; narrow the window to its smallest; switch to the light theme → Help lists `] [ Next / previous place`, `o Take ours`, `t Take theirs`, `b B Take both (ours first / theirs first)`, `Ctrl+Enter Commit once all is solved`; the bar's pill and button stay visible, long lines end in `…` or wrap; readable in both themes 〔未訳〕
- [ ] **21.21** In a worktree stopped by a merge (the `r` repo of #23), open the graph (`1`) and the worktree table (`3`) → The graph's top has a red-ringed row with dashed edges to both parents, `⚠ 衝突 残り N` and `マージ中: feat → main`, the hash `MERGE`; solving every place in `4` turns it to `✓ 解けた`. No `⚠ 衝突の予測` pill anywhere; the table's clash column is `—`, the pairs show `·` with「予測は衝突の解消（4）の実物に変わりました。」 〔未訳〕
- [ ] **21.22** While `r` is stopped by the merge, look at the graph's tool band and hover `Pull` and `Stash`; press `p` → `Pull` and `Stash` are dimmed, the hover says `Held while a merge is under way`; `p` toasts why it waits; the other buttons are unchanged 〔未訳〕
- [ ] **21.23** In a repository with no `rerere.enabled` (`git config --get rerere.enabled` prints nothing), make a merge clash so `4` opens → A band asks whether to remember solved clashes (`y` / `n` / Esc); `y` toasts that rerere is on and `git config rerere.enabled` says `true`; in a second clash (after `git merge --abort`) no band comes again this session. With `n` the config says `false` and the band never returns 〔未訳〕
- [ ] **21.24** With `rerere.enabled=true` and a recorded resolution (solve a clash, `git rerere`, `git merge --abort`), merge the same branch again so `4` opens → The file rerere solved shows `✓ ↺ 前に解いた形` on its places and the bottom bar says how many are such; `Enter` on it takes the mark away (the count drops); `x` starts the file over with open places and a toast naming the backup under `refs/michi/backup`, and the same clash is no longer solved by rerere on a new merge. `x` on a place rerere did not solve toasts that it did not 〔未訳〕
- [ ] **21.25** Merge two branches that changed the same lines in different commits so `4` opens, and select a place → Under the place's header two rows appear, `自分` and `相手`, each with the short hash, subject, author and date of the commit that changed those lines on that side (a side no commit touched says so); they show a moment after the place is selected, not at once, and the window never stalls; a long subject is clipped, not wrapped 〔未訳〕
- [ ] **21.26** In the `4` view select an open text place and press `e`; edit the lines; Ctrl+Enter. Then `e` on a solved place and on a file that clashed whole; `e` and Esc → The result box turns into a field holding ours then theirs, with a hint (Ctrl+Enter solves, Esc leaves); Ctrl+Enter solves the place with exactly what was written (tag `ここで書いた`, the file shows it after the next read) and moves to the next open place; Esc leaves the place as it was; a solved place says to restart the file, a whole-file clash says to take a side; the field keeps the keys (letters type, they do not pick) 〔未訳〕
- [ ] **21.27** Merge two branches that changed one line in different words (e.g. `let a = 1;` to `let a = 22;` and to `let a = 3;`) so `4` opens → In the ours and theirs columns only the words that differ from the base are on a stronger ground (`22`, `3`); with the base column hidden or unknown they are compared with the other side; a line one side added is marked whole; unchanged lines are plain; long lines are still cut with … 〔未訳〕
- [ ] **21.28** In `4` with places left and a Claude (tsumugi tab) in that worktree, press `g` or click「Claude に渡す」in the bar; then do the same in a worktree with no Claude tab → A toast says the Claude was asked / a new tab started; in the first case the request line (path of the brief + 「Do not commit」) is typed into that Claude's tab; in the second a new tsumugi tab named after the worktree starts `claude` with it; the brief (`%TEMP%\michi-handoff\<worktree>-<time>.md`) lists each open place with ours / base / theirs and both sides' intent; the repository itself gets no new file; when the Claude adds the files the remaining count falls by itself; with nothing left `g` only says so 〔未訳〕
- [ ] **21.29** In `4` with a file that clashed whole (deleted on one side, or binary), click「外部ツールで開く」or press `w`; once with `merge.tool` set in git config (e.g. `vscode`, `meld`), once without → With a merge tool: `git mergetool` opens that tool on the file, and when it is closed a toast says so, the file is added if the tool reported success, and the count falls; without: the file opens in the program the system has for it (Explorer / `open` / `xdg-open`) and the toast says so; a failure shows git's first line 〔未訳〕
- [ ] **21.30** Hand a conflict to a Claude with `g` (21.28); keep michi open while that Claude works until it stops and waits for you → While it runs nothing is said; once it stops (waiting or done) a toast「<worktree> の Claude が止まりました（待ちか済み）: diff を見てください」shows for 10 s, once 〔未訳〕
- [ ] **21.31** As 21.30: after the stop toast → A second toast says how many places still have conflict markers, or that none are left; a Claude handed notes (13.13) gets no second toast 〔未訳〕

## 22. Branch changes from the picker and the graph

- [ ] **22.1** Press `b`, select a local branch not checked out, `F2`, type a new name, Enter; then `F2` and Enter with the same or a taken name → The box shows `Rename <old> to:` with the old name in it and no rows; Enter toasts `Renamed <old> to <new>` and the graph's chip shows the new name after the next read; the same name does nothing, a taken or bad name toasts git's reason; Esc closes 〔未訳〕
- [ ] **22.2** With a remote branch of the same name (`origin/x`), press `b`, select the local `x`, `Ctrl+U`; then on a branch with no such remote → Toast `x now follows origin/x` (`git rev-parse x@{upstream}` says so); without a remote of that name a toast says no remote branch is named so 〔未訳〕
- [ ] **22.3** In a worktree on a branch, press `b`, select another branch (or a remote one), `Ctrl+R` and yes; once more with a clashing branch → A band names the branch, the worktree and the target; yes rebases and toasts the backup of the old tip (`refs/michi/backup/…`); a clash leaves the worktree as it was (no rebase in progress) and toasts git's words; the same branch or a worktree in the middle of something is refused with a toast; a Claude at work adds its warning to the band 〔未訳〕
- [ ] **22.4** In the graph select a commit and press `m`, yes; once on a clashing commit → A band names the commit and the window's branch; yes merges (a toast with the new commit); a clash opens the `4` view; on the uncommitted row or a detached worktree a toast says why not 〔未訳〕

## 23. Notifications, bug report and window

- [ ] **23.1** With the window behind another, let a handed Claude stop; then make a fetch fail (a remote that is not there); open settings, the Notifications page, turn each column on and off, press the test buttons → The system's notification with michi's icon, and a sound if on, come for each event only while the window is not in front; a webhook set on the page gets the event; the test buttons send at once; edits are written to `config.toml`'s `[notify]` and a hand edit is read back 〔未訳〕
- [ ] **23.2** Press `F12` in any view; read the form that opens; then add `{ on = "<F9>", run = "bugreport" }` to `prepend_keymap` in `keymap.toml` and press `F9` → The browser opens michi's issue form with `version` (`michi 0.41.0 (x86_64)`), `os` and `context` filled (Scale, Config named like `uchmk\michi\config.toml`, the last failure if there was one) and no user name in any of it; a toast says nothing is sent until you submit; `F9` does the same 〔未訳〕
- [ ] **23.3** Move and resize the window, wait a second, close it, start michi again; maximise, close, start again; start with `--keys` and move the window → The window opens where and as big as it was (on the same screen, maximized if it was, with the size it had before when you restore it); `window.toml` sits beside `config.toml`; a start with `--keys` neither reads nor writes it 〔未訳〕

## 24. Command palette

- [ ] **24.1** Press `Ctrl+Shift+P` (or click the field in the title bar); type `fet`, `branch`, `wor`; arrows or `Ctrl+N` / `Ctrl+P` move, Enter picks; press Esc; open it over the Settings; open it while the branch box is open → A box at the top lists the keymap's commands, the tabs and the keys of the view in front, each with its key at the right; typing narrows and lights the letters; Enter does what the key does (a fetch starts, the branch box opens, the Worktrees tab shows); Esc or a click outside closes it; over the settings a pick returns to the main window; with the branch box open it does not open 〔未訳〕

## 25. Warnings from keymap.toml / config.toml

- [ ] **25.1** Write a bad line into keymap.toml (an unknown key name), save it while michi is open; then fix it → A toast names the number of warnings and the first one; it is not said again until the warnings change; fixing the file says nothing 〔未訳〕

## 26. Dragging the column widths

- [ ] **26.1** In the graph view, drag the two lines between the side column, the graph and the detail (a line shows when the pointer is near); double-click a line; with a wide diff drag the line after the graph; close and open michi → The pointer turns to a left-right arrow over a line; the columns follow the pointer and no column goes under its least width; a double click halves that split; the widths come back at the next start; a double click on an edge brings the default width back 〔未訳〕
- [ ] **26.2** Drag the left edge of Author, Date and Commit in the graph's header → The column gets wider as the edge goes left, narrower to the right (40–300); the rows follow; a double click on an edge brings the default width back 〔未訳〕
- [ ] **26.3** In the worktree table drag the left edge of each header (Claude, from main, uncommitted, unseen, clash, buttons) and the edge between the name and Claude → Each column follows the pointer; the name and Claude share what is left; the widths come back at the next start; a double click on an edge brings the default width back 〔未訳〕

## 27. The branch / tag / stash / remote palette on ito-palette

- [ ] **27.1** Open `b`, `T`, `s` and the remote choice of a push; type, move with the arrows and `Ctrl+N` / `Ctrl+P`, click a row, press Esc, click outside → The same box as before: the matched letters lit, the make / save rows in the accent colour, the `for <worktree> at <commit>` line above and the keys below; a click on a row is Enter on it; Esc and a click outside close it 〔未訳〕
- [ ] **27.2** In `b`: `Shift+Enter`, `Delete`, `F2` (type a name, Enter; Esc cancels), `Ctrl+U`, `Ctrl+R` on a row → Each does what it did in v0.48.0; during a rename only the box shows 〔未訳〕

## 28. The repository screen

- [ ] **28.1** Press `Ctrl+O` (also: `Ctrl+Shift+P`, type "repo", Enter); Esc; `Ctrl+O` twice → The repositories michi was opened on, newest first, the folder name with its parent beside it and "here" on the one in the window; Esc and the second `Ctrl+O` close it; no other key reaches the views while it is open → | [ ] 〔未訳〕
- [ ] **28.2** Type part of a name, two words, then clear; move with the arrows; press Enter on another repository → The list narrows to the paths holding every word; Enter switches this window: the title, the graph, the worktree table and the Claude column are the other repository's, the old one is no longer read → | [x] 〔未訳〕
- [ ] **28.3** Type `C:\dev\some-repo` (or `~/...`, `./...`), Enter; then a folder that is not a repository → A first row "Open <path>" in the accent colour; a repository opens and joins the list; a folder that is not one leaves the window as it was and says so in a toast → | [x] 〔未訳〕
- [ ] **28.4** `Delete` on a row; close michi and open it again → The row is gone, and stays gone; the repositories opened since are at the top (`recent.txt` in michi's folder) → | [x] 〔未訳〕

## 29. 確かめの窓（ito-confirm）

- [ ] **29.1** In the graph press `t` on a commit (revert) → The window dims and a box in the middle asks `Are you sure?` / `よろしいですか` with the band's words, the buttons `Go ahead` and `Not now`, and `y Go ahead · Esc Not now` under them; nothing behind answers the mouse → | [~] 〔未訳〕
- [ ] **29.2** Press `y`; then ask again and press `Esc`, then `n`; then click each button → `y` and the first button run it; `Esc`, `n` and the second button drop it → | [x] 〔未訳〕
- [ ] **29.3** Revert a merge commit → One button per parent (`keep parent 1`, `keep parent 2`) and `Not now`; `1` / `2` and a click run it with that side; `y` does nothing → | [x] 〔未訳〕
- [ ] **29.4** Shrink the window to its smallest with the box up → The box narrows and its words wrap; every button stays clickable → | [~] 〔未訳〕

## 30. 文字の選択とコピー（ito-select）

- [ ] **30.1** Select a commit; drag the mouse over some diff lines, the subject and the body → The text under the drag is lit in the theme's selection colour; letting go copies it (paste it in a terminal) → | [x] 〔未訳〕
- [ ] **30.2** Double-click a word, then triple-click a line; press Ctrl+C after a drag → A word, then the line is picked up and copied; Ctrl+C copies it again → | [x] 〔未訳〕
- [ ] **30.3** Put a web address in a commit message; double-click it; Ctrl+click it → The whole address is picked up; Ctrl+click shows the pointing hand and opens it in the browser → | [~] 〔未訳〕
- [ ] **30.4** Click a parent hash, a file row and a merged commit; click an empty place → The click still does what it did (go to the commit, scroll to the file); an empty click lets the selection go → | [x] 〔未訳〕
- [ ] **30.5** Select a diff line and move to another commit → The selection is gone → | [x] 〔未訳〕

## 31. michi mcp（Claude Code に worktree の様子を渡す）

- [ ] **31.1** With michi open on a repository with two worktrees, run `claude mcp add michi -- michi.com mcp`, then ask Claude Code "which worktrees are behind main?" → Claude calls `michi_worktrees` and answers with each worktree's branch, ahead / behind and uncommitted files → | [x] 〔未訳〕
- [ ] **31.2** Make two worktrees edit the same file, wait for michi's prediction, ask "which worktrees would conflict?" → `michi_conflicts` names the pair and the file → | [x] 〔未訳〕
- [ ] **31.3** Close michi and ask again → The tool answers "michi is not running; start michi and ask again" → | [x] 〔未訳〕
- [ ] **31.4** Open two michi windows, close the first, ask again after half a minute → The second window answers → | [x] 〔未訳〕

## 32. 変更の画面の diff の選択とコピー（ito-select）

- [ ] **32.1** In the `2` view pick a file; drag the mouse over some diff lines → The text under the drag is lit in the theme's selection colour; letting go copies it (paste it in a terminal) → | [ ] 〔未訳〕
- [ ] **32.2** Double-click a word, then triple-click a line; press Ctrl+C after a drag → A word, then the line is picked up and copied; Ctrl+C copies it again → | [ ] 〔未訳〕
- [ ] **32.3** Click a file row, a worktree chip, the amend switch and Commit → Each still does what it did; an empty click in the diff lets the selection go → | [ ] 〔未訳〕
- [ ] **32.4** Select a diff line and pick another file or switch the worktree → The selection is gone; the commit view's selection is not drawn here → | [ ] 〔未訳〕

## 33. 衝突の画面の文字の選択とコピー（ito-select、`4`）

- [ ] **33.1** Open a conflict (`4`); drag the mouse over some lines of the ours and theirs columns, and the result → The text under the drag is lit in the selection colour; letting go copies it (paste it in a terminal); line numbers are not picked up → | [ ] 〔未訳〕
- [ ] **33.2** Double-click a word, then triple-click a line in a side, in the context lines and in the result; press Ctrl+C after a drag → A word, then the line is picked up and copied; Ctrl+C copies it again → | [ ] 〔未訳〕
- [ ] **33.3** In a rebase, drag over the ours / theirs commit lines of the intent → The subject and author are picked up → | [ ] 〔未訳〕
- [ ] **33.4** Click a place, a side's pick button, `base` and the buttons of the result; click an empty place → Each click still does what it did (select the place, choose a side, show the base); an empty click lets the selection go → | [ ] 〔未訳〕
- [ ] **33.5** Start writing a result by hand; drag and double-click inside the text box → The text box selects its own text as before; nothing outside it is lit → | [ ] 〔未訳〕
- [ ] **33.6** Select some lines, then scroll the places or solve a place → The selection is gone (no wrong text stays lit) → | [ ] 〔未訳〕

## 34. Selecting text in the review (ito-select)

- [ ] **34.1** Press `d` over a worktree; drag the mouse over some diff lines → The text under the drag is lit in the theme's selection colour; letting go copies it (paste it in a terminal); no comment opens → | [ ] 〔未訳〕
- [ ] **34.2** Double-click a word in the diff, triple-click a line, press Ctrl+C after a drag → A word, then the line is picked up and copied; Ctrl+C copies it again → | [ ] 〔未訳〕
- [ ] **34.3** Single-click a diff line → The cursor moves to the line and its note can be written (`n`), as before → | [ ] 〔未訳〕
- [ ] **34.4** Select a diff line, then pick another file or switch the wide view → The selection is gone → | [ ] 〔未訳〕
- [ ] **34.5** Select diff text, close the review (Esc), select a commit's detail text → No stale selection shows in either place → | [ ] 〔未訳〕

## 35. Answering a waiting Claude from the notification (W)

- [ ] **35.1** With tsumugi running, let a Claude in a worktree reach a question (a permission prompt) while michi is open → A toast `The Claude in <wt> is waiting: <question> (W: answer)` shows for about 15 s; with the window behind another, the system notification `michi: a Claude is waiting` shows too (it follows the `done` settings of Notifications) → | [ ] 〔未訳〕
- [ ] **35.2** Press `W` from any view → The window goes to the worktree view with that worktree selected, its branch review (the diff) open, and the question with an answer line over the bottom of the window → | [ ] 〔未訳〕
- [ ] **35.3** Type `y` (or any text) and press Enter → The text is typed into that Claude's tsumugi tab and submitted; the line closes and the toast says `Sent the answer to tab N`; the Claude goes back to running → | [ ] 〔未訳〕
- [ ] **35.4** Type the answer with a Japanese IME and confirm the conversion with Enter → The conversion is confirmed without sending; the next Enter sends → | [ ] 〔未訳〕
- [ ] **35.5** With two Claudes waiting, press `W` twice → The first press goes to the one that has waited longest, the second to the next one, then round → | [ ] 〔未訳〕
- [ ] **35.6** Press Esc in the answer line → The line closes; the review stays open → | [ ] 〔未訳〕
- [ ] **35.7** Answer the Claude in its own tab while the line is open → The line closes by itself once michi sees the Claude is no longer waiting → | [ ] 〔未訳〕
- [ ] **35.8** Press `W` with no Claude waiting → The toast says `No Claude is waiting` → | [ ] 〔未訳〕
- [ ] **35.9** Start michi while a Claude is already waiting → No toast for it (only waits that begin while michi runs are told); `W` still goes to it → | [ ] 〔未訳〕

## 36. Warning about overlapping work in New work (`N`)

- [ ] **36.1** Change a file (say `crates/michi/src/app.rs`) in one worktree and leave it uncommitted; press `N` and type `fix app.rs` in the instruction → A line `⚠ Heads up: <branch> is changing crates/michi/src/app.rs …` shows under the instruction as you type; nothing else in the form is blocked → | [ ] 〔未訳〕
- [ ] **36.2** Type a folder instead (`tidy crates/michi/src/`) → The same worktree is named with the files under that folder (the first three, then `+N`) → | [ ] 〔未訳〕
- [ ] **36.3** Type an instruction with no file or folder name, or names no worktree is changing → No warning shows → | [ ] 〔未訳〕
- [ ] **36.4** With the warning showing, press Ctrl+Enter → The work starts as usual (the warning does not stop it) → | [ ] 〔未訳〕
- [ ] **36.5** Two worktrees change the same file → Both are named in the warning → | [ ] 〔未訳〕

## 37. Open a worktree's folder in a terminal, editor or file manager (#75)

- [ ] **37.1** In the worktree tab select a row; press `t` → A tsumugi tab opens in that worktree's folder; a toast says it was opened → | [ ] 〔未訳〕
- [ ] **37.2** Press `e`, then `E` → VS Code opens on the folder; the file manager shows the folder (Explorer on Windows) → | [ ] 〔未訳〕
- [ ] **37.3** Click the Terminal, Editor and Explorer buttons on a row (also on the main worktree's row) → Same as the keys, for the clicked row → | [ ] 〔未訳〕
- [ ] **37.4** Open the palette (Ctrl+Shift+P) and type "worktree in" → Three entries (terminal, editor, file manager); picking one opens the window's worktree (the selected row when the worktree tab is in front) → | [ ] 〔未訳〕
- [ ] **37.5** Write `[open]` `editor = "notepad {path}"` into config.toml, wait 2 s, press `e` → Notepad is started with the folder as its argument; Settings > General shows the line → | [ ] 〔未訳〕
- [ ] **37.6** Write `editor = "no-such-program"`, press `e` → A toast "open failed: ..." with the program's name; michi keeps running and does not freeze → | [ ] 〔未訳〕
- [ ] **37.7** Write `editor = "a \"b"` (unclosed quote) → A warning about config.toml names the key; `e` still uses the default → | [ ] 〔未訳〕

## 38. Key hints on hover

- [ ] **38.1** Hover the gear at the top right and the search field beside it → The tooltips read `Settings (,)` and `Search commands (Ctrl+Shift+P)` → | [ ] 〔未訳〕
- [ ] **38.2** Bind another key to `palette` in keymap.toml (prepend), restart, hover the search field → The tooltip shows the new key → | [ ] 〔未訳〕
- [ ] **38.3** Hover each view tab (Graph, Changes, Worktrees) → The tooltip names the view and its digit, for example `Graph (1)` → | [ ] 〔未訳〕
- [ ] **38.4** In the graph view hover Fetch, Pull, Push, Branch, Stash and New work → Each tooltip shows its key, for example `Fetch (f)`, `New work (N)` → | [ ] 〔未訳〕
- [ ] **38.5** During a merge hover Pull and Stash → The tooltip still says they are held, not the key → | [ ] 〔未訳〕
- [ ] **38.6** In the worktree view hover New work and the re-read button → `New work (N)`; the re-read button shows just its name → | [ ] 〔未訳〕

## 39. Graph filter with fuzzy matching (ito-match)

- [ ] **39.1** Press `/` over the graph and type the first letters of a long subject, skipping the rest (e.g. `fxflk` for "Fix the flaky test") → The commit is a hit (the others fade) and the hit count shows it → | [ ] 〔未訳〕
- [ ] **39.2** Type two words, one of a subject and one of an author or a branch name → Only commits that hold both words are hits → | [ ] 〔未訳〕
- [ ] **39.3** Type the first 7 characters of a commit hash, then characters from the middle of one → The first selects that commit; the middle of a hash hits nothing by itself → | [ ] 〔未訳〕
- [ ] **39.4** Type a word, then add letters; then take letters off → Adding letters never lights new rows; taking them off lights rows again, with no lag on a long history → | [ ] 〔未訳〕
- [ ] **39.5** Type in capitals (`FIX`) → The same hits as `fix` → | [ ] 〔未訳〕

## 40. What a push sends

- [ ] **40.1** On `feat` (not on `origin` yet) press `P`; Esc; `P` again and `y` → A band under the top bar: `Push 3 commit(s) to origin/feat, which is not there yet? It becomes this branch's upstream.` with one row per commit (short hash, subject, newest first); Esc pushes nothing (`git ls-remote origin feat` is empty); `y` pushes and says `Pushed to origin/feat, now its upstream` 〔未訳〕
- [ ] **40.2** Commit once more on `feat` and press `P` → `Push 1 commit(s) to origin/feat?` with just that commit; make 15 commits and `P`: the newest 12 are listed and a last row reads `… and 3 more` 〔未訳〕
- [ ] **40.3** Let `b` push a commit to `origin/feat`, press `f` here, commit and press `F` → A red band (red outline and note): the commits that go, and the note `origin/feat has 1 commits this branch does not. Push over them? …`; `P` instead only says `F pushes over them, asking first`; `y` pushes with the lease and names the backup 〔未訳〕
- [ ] **40.4** Press `P` on a branch that origin already has, on one only behind, and in a repository with no remote → `origin/feat has it already`, `Nothing to push: origin/feat is n ahead (p pulls)` and `No remote …`; no band in any of them 〔未訳〕
- [ ] **40.5** With two remotes and no `origin`, press `P`, pick one → The list of remotes first, then the band for the picked remote 〔未訳〕
- [ ] **40.6** Switch the language to Japanese and press `P` → The band reads `n 個のコミットを … に push しますか？`; the commit rows do not run off the box 〔未訳〕

## 41. Autostash on pull and switch

- [ ] **41.1** In a worktree with an uncommitted edit to a file the upstream also changed (different lines), press `p` → The pull goes through instead of stopping; the toast says the changes were stashed and put back, with the `michi/backup/...` name; the edit is still there; `git stash list` is empty → | [ ] 〔未訳〕
- [ ] **41.2** Same, but the edit touches the same lines the upstream changed → The pull goes through; the toast says the stash clashed in the file; the file has `<<<<<<<` marks and the stash stays in the list and under `michi/backup/` → | [ ] 〔未訳〕
- [ ] **41.3** Switch a worktree with changes to a branch that would overwrite them (branch picker) → The switch happens, the toast says the changes were stashed and put back (or clashed) → | [ ] 〔未訳〕
- [ ] **41.4** Pull or switch with no changes, or with changes that do not stand in the way → Behaves as before; no stash, no backup, no extra text in the toast → | [ ] 〔未訳〕
- [ ] **41.5** Switch to a branch that does not exist, with changes in the worktree → The error is shown as before and the changes are still in the worktree (nothing left in the stash list) → | [ ] 〔未訳〕

## 42. Reflog screen (`L`, #50)

- [ ] **42.1** In the graph or the worktree table press `L` → A list opens at the top: HEAD's reflog (`HEAD@{n}`, what happened, the commit's subject) and the backups under `refs/michi/backup/` (tinted), newest first; the window does not freeze on a big repository → | [ ] 〔未訳〕
- [ ] **42.2** Reset a branch (`git reset --hard HEAD~1`) in a worktree, then press `R` → The reset is the top `HEAD@{0}` row and the commit it dropped is the row under it → | [ ] 〔未訳〕
- [ ] **42.3** Move with `j k`, the arrow keys, PageUp/PageDown, Home/End and the mouse; click a row → The selection follows; a click on the selected row asks as Enter does; a click outside the list closes it → | [ ] 〔未訳〕
- [ ] **42.4** Select a row and press Enter → The band asks to bring the commit back as a new branch and says nothing moves; `y` makes `recovered/<id>` at that commit (`-2` after it if taken), the list closes and a toast names the branch; HEAD and the worktree are untouched → | [ ] 〔未訳〕
- [ ] **42.5** After a force push or a branch delete (which keep a backup), press `R` → The backup is listed with the reason michi wrote; bringing it back gives a branch at its commit → | [ ] 〔未訳〕
- [ ] **42.6** Press `R` in a repository with no commits and nothing kept → A toast says the reflog and backups are empty; no list opens → | [ ] 〔未訳〕
- [ ] **42.7** Switch the language to Japanese and open the list → The title, hints, band and toast are in Japanese → | [ ] 〔未訳〕

## 43. Reset: soft, mixed or hard (#77)

- [ ] **43.1** In the graph, select an older commit and press `R` → A band asks how far to reset, tells what each of 1 soft / 2 mixed / 3 hard loses, and has the three buttons and No → | [ ] 〔未訳〕
- [ ] **43.2** With a staged change, press `R` on the previous commit, then `1` → The branch moves; the change shows as staged; the toast names the `michi/backup/…` ref of the old tip → | [ ] 〔未訳〕
- [ ] **43.3** Same, with `2` → The branch moves; the change is unstaged but its text is in the file → | [ ] 〔未訳〕
- [ ] **43.4** With an edited tracked file and an untracked file, press `R` and `3` → The edit is gone, the untracked file stays; the toast names two backup refs (the tip, the changes); `git show <changes>:file` has the edit → | [ ] 〔未訳〕
- [ ] **43.5** Press `R`, then Esc (or `n`, or `y`) → Nothing changes (y does nothing: only 1 2 3 choose) → | [ ] 〔未訳〕
- [ ] **43.6** Press `R` in a worktree in the middle of a merge or rebase; with no commit selected → A toast says to finish it first / to pick a commit; no band → | [ ] 〔未訳〕
- [ ] **43.7** Press `R` where a Claude is working → The band ends with the note that a Claude is working there → | [ ] 〔未訳〕

## 44. Ignoring an untracked file (`i`, #49)

- [ ] **44.1** In the `2` view make an untracked file `x.log` in the worktree, select it in the unstaged list and press `i` → No question; a toast says `Added /x.log to .gitignore`; the file leaves the list after the next read and `.gitignore` in the worktree's root has the line (made if there was none) 〔未訳〕
- [ ] **44.2** Press `i` on a tracked (modified) file, on a staged file; then on an untracked file with `[` `]` and `*` in its name → The first two toast that only an untracked file can be ignored and change nothing; the brackets and star are escaped in the line (`/a\[1\]\*.log`) and git ignores only that file 〔未訳〕
- [ ] **44.3** Press `i` on a file whose line is already in `.gitignore` (an exact line, e.g. after a hand edit) → A toast says the line is in `.gitignore` already; the file is not changed; with a CRLF `.gitignore` the new line gets CRLF 〔未訳〕
- [ ] **44.4** Open the help (`?`) in the `2` view → `i` is listed: Add the untracked file to .gitignore (JA: 追っていないファイルを .gitignore に足す) 〔未訳〕

## 45. Changes as a tree of folders

- [ ] **45.1** In `2` over a worktree with changes in nested folders, press `t` (or click Tree in the top bar) → Both lists become folders (a lone chain such as `a/b/c/` is one row) with files under them, folders first; press `t` again for the flat list 〔未訳〕
- [ ] **45.2** Click a folder row → It folds (▸) and its files vanish from the list; click again to open it (▾). Staged and unstaged fold apart 〔未訳〕
- [ ] **45.3** Click the `+` at the right of an unstaged folder; then the `−` of a staged one → Every file below it goes into the index, then every one (a rename's old path too) comes out; the count at the right matches 〔未訳〕
- [ ] **45.4** With a folder folded, press `j` / `k` and the arrow keys → The selection walks the files in the order shown and skips the folded ones; a selection folded away moves to the nearest shown file 〔未訳〕
- [ ] **45.5** Stage a folder while a Claude is writing in the worktree → The same question for a yes as for a file 〔未訳〕

## 46. Comparing two commits or branches

- [ ] **46.1** In the graph select a commit, then Ctrl+click (Cmd+click on macOS) another → The first row keeps a tint with an outline (the mark), the clicked one is selected; Ctrl+click the marked row again takes the mark off 〔未訳〕
- [ ] **46.2** With a mark and a selection press `D` → The review opens over the worktree table with "Comparison" and `a...b`, listing the files that changed from where the two parted (or plain a to b when one is the other's ancestor); the diff on the right is the usual one 〔未訳〕
- [ ] **46.3** Press `D` with no mark; press Esc in the graph with a mark → A toast asks to Ctrl+click a second commit; Esc takes the mark off 〔未訳〕
- [ ] **46.4** In the comparison press `u` `s` Space `n` `m`; click a file's check box → Nothing happens (no uncommitted, last look, checks, notes or merge); no "seen" count or switches are shown 〔未訳〕
- [ ] **46.5** Compare two branch tips, then swap which one you Ctrl+click first → The list is each side's own changes since they parted; `j` `k` `J` `K` `Enter` and PageUp/PageDown work as in the branch review; Esc closes it 〔未訳〕
- [ ] **46.6** Open the comparison, then press `d` on a worktree → The branch review of the worktree replaces it with its own list, checks and notes 〔未訳〕

## 47. A worktree on no branch (the strip under the title bar)

- [ ] **47.1** In the window's worktree run `git checkout --detach` on a commit → Within a moment a strip appears under the title bar: "Not on a branch: the commit … is checked out. Commits made here belong to no branch." with `R Back to <branch>` and `B Branch from here` at the right → | [ ] 〔未訳〕
- [ ] **47.2** Check out a tag (`git checkout v1`) → The strip names the tag; the body below moves down by the strip's height, nothing is cut off → | [ ] 〔未訳〕
- [ ] **47.3** Commit something on the detached HEAD → The strip adds "Commits on no branch yet: 1"; the number follows each new commit → | [ ] 〔未訳〕
- [ ] **47.4** Press `R` (or click `Back to …`) with such a commit → The worktree is on the branch again, the toast names the backup (`michi/backup/…`) that keeps the commit, the strip goes → | [ ] 〔未訳〕
- [ ] **47.5** Press `B`, type a name, press Shift+Enter → The branch is made at HEAD, the worktree is on it, the strip goes; the commit is on the new branch → | [ ] 〔未訳〕
- [ ] **47.6** Detach, then delete the branch it came from, then look at the strip → `R` is not offered for a branch that is gone (an earlier existing one, or only `B`) → | [ ] 〔未訳〕
- [ ] **47.7** Start `git rebase -i` and stop it at an edit → The strip says it is a rebase, with no buttons; it goes when the rebase ends → | [ ] 〔未訳〕
- [ ] **47.8** With a Claude working in the worktree press `R` → A yes is asked first, as for a switch → | [ ] 〔未訳〕
- [ ] **47.9** Open the settings (`,`) or the key list (`?`) while the strip is up; switch EN and JA → The strip stays tidy at a narrow width (the text is cut with an ellipsis, the buttons stay); both languages read well → | [ ] 〔未訳〕

## 74. Commit message help (history and template)

- [ ] **74.1** In the `2` view press `c` and look at the empty box → The hint reads "What changed, and why (Alt+Up / Alt+Down: earlier messages)"; EN and JA both read well and are not cut off → | [ ] 〔未訳〕
- [ ] **74.2** With the box focused press Alt+Up → The box takes the latest commit's message (all lines); again: the one before; at the oldest nothing happens → | [ ] 〔未訳〕
- [ ] **74.3** Type "wip", press Alt+Up twice, then Alt+Down twice → You are back at "wip" (the draft is kept); the same message is never listed twice → | [ ] 〔未訳〕
- [ ] **74.4** Step to an old message, edit a word, press Alt+Up → The edited text is kept as the draft (Alt+Down from the next message comes back to it) → | [ ] 〔未訳〕
- [ ] **74.5** Run `git config commit.template tpl.txt` (a file with `# help` lines and a `feat: ` line), then press `c` on an empty box → The box is filled with `feat:` without the `#` lines; committing it as it is leaves no `#` line in the message → | [ ] 〔未訳〕
- [ ] **74.6** Type a message first, then press `c` → The template does not replace what was typed; with `A` (amend) the head's message fills, not the template → | [ ] 〔未訳〕
- [ ] **74.7** Commit, then press `c` again → The history has the new message at the top; the template fills the empty box again → | [ ] 〔未訳〕
- [ ] **74.8** In a repository with no commits press `c` and Alt+Up → Nothing breaks; Alt+Up does nothing → | [ ] 〔未訳〕
- [ ] **74.9** Click the box with the mouse (not `c`), then press Alt+Up → The history works the same → | [ ] 〔未訳〕

## 86. Row density (compact / standard / cozy)

- [ ] **86.1** Open the settings (`,`), General: look for "Row density" → Three choices, Compact / Standard / Cozy; Standard is lit by default → | [ ] 〔未訳〕
- [ ] **86.2** Pick Compact → The graph rows, the side column's rows and the reflog / tidy / reorder lists shrink at once (about 20% shorter); text stays readable and nothing is clipped; config.toml has `[view]` `density = "compact"` → | [ ] 〔未訳〕
- [ ] **86.3** Pick Cozy → The same rows grow (about 20% taller); the graph dots and lines stay joined across rows; the scrollbar marks still sit at their rows → | [ ] 〔未訳〕
- [ ] **86.4** Click a row, scroll with the wheel and drag the scrollbar in each density → Clicks land on the row under the pointer; the selected row stays in view → | [ ] 〔未訳〕
- [ ] **86.5** Edit `density` in config.toml by hand (also a wrong word) → The rows change within a few seconds without restart; a wrong word gives a toast warning and the standard rows → | [ ] 〔未訳〕
- [ ] **86.6** Switch EN and JA in the settings → The section and its three choices read well in both → | [ ] 〔未訳〕

## 94. Review queue of finished Claude work (Review column)

- [ ] **94.1** Open the worktree table (`3`) with two or more branch worktrees that differ from main → A Review column sits between Unseen and Clash; a worktree never opened in the review (`d`) reads `Not looked at` (dim) with `0 / N files checked`; the main worktree reads `—`; in the narrow card layout it is a `Review` line 〔未訳〕
- [ ] **94.2** Open a branch's review with `d`, then close it (Esc) and look at the table → That row reads `In review` (blue) within a moment of closing, with the count of files checked 〔未訳〕
- [ ] **94.3** In the review check some files (Space) but not all; then check the last one → `In review` with `k / N`; once every file is checked the row reads `Approved` (green) with `N / N` 〔未訳〕
- [ ] **94.4** Write a note (`n`) on a line and hand it with `g` → The row reads `Flagged` (yellow) even though all files are checked; the Claude commits again and the row goes back to `In review` or `Approved` by what is checked 〔未訳〕
- [ ] **94.5** Commit in an approved worktree a change to a file already checked → The row leaves `Approved` (the changed file lost its check) without opening the review 〔未訳〕
- [ ] **94.6** Drag the Review column's left edge in the header; restart michi → The width follows the drag and is kept; the other columns still resize 〔未訳〕
- [ ] **94.7** Switch the language EN and JA, and a theme with a light background → The stage words read `Not looked at` / `In review` / `Flagged` / `Approved` and JA `未確認` / `確認中` / `指摘済み` / `承認`; every colour is readable 〔未訳〕

## 66. Right-click menus for a commit, a branch and a file

- [ ] **66.1** In the graph (`1`) right-click a commit row → The row is selected and a menu lists Branch (b), Tag (T), Merge (m), Cherry-pick (C), Revert (t), Compare (D), Reset (R), each with its key; a right-click on the WIP row opens nothing 〔未訳〕
- [ ] **66.2** Pick Tag from that menu; then Esc, and right-click another row and click outside the menu → The tag palette opens on the clicked commit (not the one selected before); Esc and a click outside close the menu without doing anything 〔未訳〕
- [ ] **66.3** While a menu is open, type `d` or `j` → Nothing happens to the view (the keys do not reach the window until the menu is closed) 〔未訳〕
- [ ] **66.4** In the worktree table (`3`) right-click a worktree row → The row is selected and a menu lists Pull request, Open in the terminal / editor / file manager, Branch diff, Rebase onto trunk, Take main, Reorder, Reflog, Remove, with keys; picking Open in the editor opens that row's folder 〔未訳〕
- [ ] **66.5** Pick Remove for a worktree row → The same question as pressing `x` asks first; nothing is lost without a yes 〔未訳〕
- [ ] **66.6** In the changes view (`2`) right-click an unstaged file (flat list and tree) → The file is selected and a menu lists Stage (Space), Split, Ignore, Discard, Amend; Stage moves the clicked file, Discard asks first 〔未訳〕
- [ ] **66.7** Press `?` and open the palette; switch EN and JA and look at the menus → No operation exists only in a menu (each is a key in the help and the palette); the menu words read well in both languages 〔未訳〕

## 87. Diff minimap and next / previous change (`n` / `N`, #87)

- [ ] **87.1** In `2`, select a file with changes far apart (a long file) → A thin strip at the right edge of the diff shows a tick per run of changes (green added, red deleted, amber changed in side by side) and a lit box over the part in sight; the diff text does not run under it 〔未訳〕
- [ ] **87.2** Scroll with the wheel and PageDown → The lit box follows; it fills the strip (or goes) when the whole diff fits 〔未訳〕
- [ ] **87.3** Click a tick, then drag along the strip → The diff scrolls to that part, centred on the point; it stays inside the file at both ends 〔未訳〕
- [ ] **87.4** With the focus on the files press `n` repeatedly, then `N` → Each `n` brings the next change to just under the top of the diff, the last one stops there (and does not stick on the one before it); `N` goes back the same way 〔未訳〕
- [ ] **87.5** Press `Enter` (side by side) then `n` / `N` → The ticks and the jumps follow the other layout; no change is skipped or repeated 〔未訳〕
- [ ] **87.6** Press `l` (pick lines) then `n` / `N` → The cursor moves to the first line of the next / previous run of changed lines and the diff follows it; `N` in the middle of a run goes to that run's start 〔未訳〕
- [ ] **87.7** Select a file with no diff text (binary, deleted-empty) and press `n` → No strip, nothing happens; no error 〔未訳〕
- [ ] **87.8** Press `?`; switch EN and JA → The key list names `n N` as jumping to the next or previous change in both languages; the strip looks right in Dark and Light themes 〔未訳〕

## 100. Trial merge of several worktrees into the trunk

- [ ] **100.1** In the worktree table press `T` with two or more worktrees on branches other than the trunk → A list opens at the top with those worktrees (branch and folder), each with a `·` mark → | [ ] 〔未訳〕
- [ ] **100.2** Press `T` with fewer than two such worktrees → A toast says a trial merge needs at least two worktrees; no list opens → | [ ] 〔未訳〕
- [ ] **100.3** Press `j` `k` and Space on three rows in the order c, a, b; click a row → The picked rows show 1, 2, 3 in the order picked; Space or a click on a picked row takes it back and later marks shift; `u` clears all → | [ ] 〔未訳〕
- [ ] **100.4** Press Enter with nothing picked → A toast asks to pick with Space first → | [ ] 〔未訳〕
- [ ] **100.5** Pick two branches that touch different files and press Enter → "Merging in memory…" shows briefly, then each step reads "clean" and the summary counts the changed files with +/- lines → | [ ] 〔未訳〕
- [ ] **100.6** Pick two branches that edit the same lines, run, then pick them in the other order and run again → The clashing step is named with its files and tinted; which step clashes depends on the order → | [ ] 〔未訳〕
- [ ] **100.7** Pick a branch that is already merged into the trunk → Its step reads "already in" → | [ ] 〔未訳〕
- [ ] **100.8** After a run, check `git status`, `git branch -v` and the worktree folders in a terminal → Nothing moved: no branch, index, HEAD or file changed → | [ ] 〔未訳〕
- [ ] **100.9** Change the picks after a run; press Esc; try Ctrl+Shift+P over the list → The old result goes when the picks change; Esc closes the list; the palette does not open over it → | [ ] 〔未訳〕
- [ ] **100.10** Switch EN and JA; use a narrow window → Both languages read well; long paths are cut with an ellipsis → | [ ] 〔未訳〕

## 93. Landing a worktree (merge in, check, fast-forward, clean up)

- [ ] **93.1** In the `3` view select a worktree that is ahead of main and press `l` → A band asks "Land <branch> on main?" with four rows (take main in, run the check, move main forward, remove the worktree) and buttons `1` to `4`; `y` does nothing, Esc closes it 〔未訳〕
- [ ] **93.2** Press `l` on the main worktree, on a detached one, on one with uncommitted changes, on one mid-rebase and on one with nothing ahead → Each answers with a toast saying why and no band opens 〔未訳〕
- [ ] **93.3** Press `l`, then `4`, on a clean branch whose main is already in it → The toast reads "Landed <branch> on main: …"; main now points at the branch tip (`git log main`), the worktree folder and the branch are gone, a Claude tab of that worktree is closed 〔未訳〕
- [ ] **93.4** Let main move on (commit on it), press `l`, then `4` → main is taken into the branch first (a merge commit), the check runs, then main moves forward to that merge; no `git push` happens 〔未訳〕
- [ ] **93.5** Press `l`, then `1` → Only main is taken in; the toast says so ("Land it again to go on") and nothing else moved 〔未訳〕
- [ ] **93.6** Press `l`, then `2`, in a repository with `scripts/verify.sh` that fails → The toast says "Stopped at the check: it failed (exit N)" with the last lines of its output; main did not move and the worktree is still there 〔未訳〕
- [ ] **93.7** Press `l`, then `4`, when main clashes with the branch → The `4` view opens on the clash and the toast says "Stopped at taking main in"; after solving and committing there, `l` `4` goes on from there 〔未訳〕
- [ ] **93.8** Press `l`, then `3`, then `l`, then `4` on the same branch → The second run skips what was done ("main was in … already") and only cleans up 〔未訳〕
- [ ] **93.9** Use a repository without a verify script → The toast says "no check found (skipped)" and the landing goes on 〔未訳〕
- [ ] **93.10** Select the window's own worktree and press `l` → Only `1` to `3` are offered and a note says it cannot be removed from here 〔未訳〕
- [ ] **93.11** Land with a Claude working in the worktree → The band notes a Claude is working there and that its tabs close at the last stage 〔未訳〕
- [ ] **93.12** Switch to JA and press `l` → The band, the buttons and every toast read in Japanese; `?` lists `l` among the worktree keys 〔未訳〕

## 78. Sync: fetch, pull and push in one (`S`)

- [ ] **78.1** In the graph view look at the tool band → A `Sync S` button sits after `Push P`, before the line; its icon is a down and an up arrow; hover says `Sync (S)` → | [ ] 〔未訳〕
- [ ] **78.2** Behind origin by 2 commits, ahead by none: press `S` → One job named `sync` runs (the title bar shows its progress, Esc stops it); the toast says `Synced with origin/<b>: pulled 2 commits` → | [ ] 〔未訳〕
- [ ] **78.3** Ahead by 1, not behind: press `S` → The commit is pushed at once (no band); the toast says `Synced with origin/<b>: pushed`; `git log origin/<b>` has it → | [ ] 〔未訳〕
- [ ] **78.4** Behind by 1 and ahead by 1 (diverged): press `S` → Nothing is merged or pushed; the toast says `Sync stopped at the pull, nothing was pushed: origin/<b> went its own way (↑1 ↓1) …` → | [ ] 〔未訳〕
- [ ] **78.5** Behind by 1, ahead by 1 on a pull that works (the other side already pulled), but the remote hook refuses the push → The toast says `Sync stopped at the push (pulled N commits first): … refused the push: …` and the pulled commits stay → | [ ] 〔未訳〕
- [ ] **78.6** A new branch with no upstream: press `S` → It is pushed and gets the upstream; the toast says `Synced: pushed to origin/<b>, now its upstream` → | [ ] 〔未訳〕
- [ ] **78.7** With two remotes and no `origin`: press `S` → The remote chooser opens; choosing one runs the sync to it → | [ ] 〔未訳〕
- [ ] **78.8** In sync already: press `S` → The toast says `Already in sync with origin/<b>` → | [ ] 〔未訳〕
- [ ] **78.9** Press `S` while a Claude works in the worktree, or in the middle of a merge → The Claude case asks `Sync there anyway?` (y/n); the merge case says sync waits (and the button is greyed) → | [ ] 〔未訳〕
- [ ] **78.10** Press `S` in the 2, 3 and 4 views; open `?`; switch EN and JA → `S` works in each; the key list has a `S` line; both languages read well → | [ ] 〔未訳〕

## 68. Syntax colours in diffs

- [ ] **68.1** Open a commit that changes a `.rs` file (detail pane), then the Changes view of a worktree with a `.rs` edit → Keywords, strings, comments, numbers and names are coloured on context, added and deleted lines; the green/red line tint and the marked words (changed words) still show → | [ ] 〔未訳〕
- [ ] **68.2** Look at a `.toml`, `.md`, `.ts`, `.py` and `Dockerfile` change → Each gets colours; a file of an unknown type (`.xyz`) and a binary file stay plain → | [ ] 〔未訳〕
- [ ] **68.3** Switch the theme in settings between a dark and a light one → The colours follow the theme and stay readable on the added and deleted tints → | [ ] 〔未訳〕
- [ ] **68.4** Open the diff side by side and unified (narrow the window) → Both layouts show the same colours; wrapped long lines keep them → | [ ] 〔未訳〕
- [ ] **68.5** Open a big commit (hundreds of files, a huge generated file) → The window does not stall while it loads; a file of over 6000 diff lines, or a line of over 1500 characters, is left plain → | [ ] 〔未訳〕
- [ ] **68.6** Select diff text with the mouse and copy it → The copied text is the plain text (no colour effects) → | [ ] 〔未訳〕

## 80. How hard a question asks (three steps)

- [ ] **80.1** Press `m` on a branch to merge, or rebase: look at the band → A plain band titled `Are you sure?` in the waiting colour; `y` goes on, Esc stops → | [ ] 〔未訳〕
- [ ] **80.2** Delete a branch, drop a stash, remove a worktree or run the tidy (Enter on picked rows), and start a reset → The band is titled `This is hard to undo` and drawn in red; `y` (or the digits for a reset) and Esc work as before → | [ ] 〔未訳〕
- [ ] **80.3** Make the remote go ahead (push from another clone), then press `F` → The band is red, titled `This overwrites the remote`, lists the commits, says what is overwritten and `To go on, type origin/<b> and press Enter.`; there is no Yes button, only No → | [ ] 〔未訳〕
- [ ] **80.4** In that band press `y`, `n` and a digit → They appear after `>` in the band as typed letters; the band stays and nothing is pushed → | [ ] 〔未訳〕
- [ ] **80.5** Press Enter with a wrong or half-typed name; then Backspace over it and type the whole `origin/<b>`, Enter → The first Enter only toasts `Type origin/<b> first, then Enter`; the full name pushes (with the lease) → | [ ] 〔未訳〕
- [ ] **80.6** In the same band press Esc, or click No → The band closes and nothing is pushed → | [ ] 〔未訳〕
- [ ] **80.7** Switch to JA and repeat 80.2 and 80.3 → Titles, the typing note and the toast read in Japanese → | [ ] 〔未訳〕
