# 実機チェックリスト

`TESTING.md` から `cargo run -p michi --example make-testcheck` で生成している。**正は TESTING.md** で、
このファイルはその表を 1 行ずつ印を付けられる形に並べたもの。食い違ったら TESTING.md を信じること。

**チェック（`[x]` と `[~]`）だけは手で書いてよく、生成し直しても残る。**それ以外を書き換えても次の生成で消える。
`[x]` は実機で確かめた印、`[~]` は見た目の行を画面の画像で判断した印（済みには数えない。持ち主が同じ画像を見て `[x]` にする）。
**件数はこのファイルに書かない**（印を付けた PR が並ぶと必ずぶつかるため）。`-- --stats` で出る。

付ける前に「失敗していたら、画面かディスクの何が違ったはずか」を自問すること。

## 1. Window and title bar

- [ ] **1.1** Start `michi C:\dev\tsumugi` (any repository) → A 1280x800 window with no system title bar; the logo, `michi` (never 道), the tabs `1 Graph` `2 Changes` `3 Worktrees` hanging from the bar's foot, the repository chip, a palette field, the `,` button and three window buttons on the right
- [ ] **1.2** Look at the chip → The repository's name in the accent colour, a drawn branch mark (a trunk and a twig with three dots, not a box), then `main · worktree N` with the real count; hovering shows the repository's top folder
- [ ] **1.3** Look at the window title (Alt+Tab, the taskbar) → `tsumugi — michi` once the repository is read
- [ ] **1.4** Drag the empty part of the title bar → The window moves; double-click maximises and restores
- [ ] **1.5** Click minimise, maximise, close → Each does what it says; close shows a red button under the pointer; the maximise glyph turns into two squares while maximised
- [ ] **1.6** Drag each edge and corner → The window resizes with the matching pointer; nothing resizes while maximised
- [ ] **1.7** Shrink the window to its smallest → It stops at 720x420; the palette field shortens, then the chip drops the branch, then the chip goes; the tabs never overlap the buttons
- [ ] **1.8** Look at michi.exe in Explorer, the taskbar and Alt+Tab → The same logo as in the title bar (the forked path on the tile), sharp at each size
- [ ] **1.9** Start michi in a folder with a Japanese name, or on a branch with one → The Japanese draws (after a frame or two), not boxes

## 2. Views and keys

- [ ] **2.1** Press `1`, `2`, `3` → The tab in force moves and opens into the view; the graph shows the history (section 6), the other two say what will be there and "Coming in a later stage"
- [ ] **2.2** Click each tab → Same as the digit
- [ ] **2.3** Press `?` → A card with the keys: "Everywhere" first, then the view's; the tab stays lit under it; `?` or Esc closes it
- [ ] **2.4** Press `N`, `f`, `p`, `P` in any view, `b` and `s` in the graph (Enter spreads the diff: 8.5), Space and `c` in the changes → A toast at the foot names the key's work and says it comes in a later stage; it goes after 4 seconds
- [ ] **2.5** Press Ctrl+Shift+P, or click the palette field → The same toast for the command search; the field lights under the pointer with a text cursor
- [ ] **2.6** Press a key with no work (`q`, `4`) → Nothing happens

## 3. Keybar and clock

- [ ] **3.1** Look at the keybar in each view → The view's first keys (the first one lit in the accent colour), then `? Keys`; nothing overlaps the clock when the window is narrow
- [ ] **3.2** Look at the right of the keybar → The clock as common.toml says (time, date, weekday); it turns over on the minute
- [ ] **3.3** Turn the clock off in the settings → It goes from the keybar, in every uchmk app that is open

## 4. Settings

- [ ] **4.1** Press `,` → The settings open between the title bar and the keybar; the title bar stays, the `,` button is lit and no tab is lit; the keybar shows `Esc Back` and `Ctrl+Tab Next page`
- [ ] **4.2** Press Ctrl+Tab → The page moves from General to Theme, with a small michi preview (the three tabs, two branches)
- [ ] **4.3** Change the language to 日本語 → Every word of michi turns Japanese at once (the tabs 樹形図・変更・worktree, the keybar, the empty views), and other uchmk apps follow
- [ ] **4.4** Change the theme, and the size → The whole window follows at once; the logo and the chip stay readable in light and dark
- [ ] **4.5** Press Esc, click the `,` button, or click a tab → Each leaves the settings; a tab opens its view
- [ ] **4.6** Edit common.toml by hand while michi is open → michi follows within a second

## 5. git and errors

- [ ] **5.1** Start `michi C:\dev` (not a repository) → The chip shows `✗ Not a Git repository` in red and a toast says so with the folder; the title stays `michi`
- [ ] **5.2** Start `michi C:\no\such` from a terminal → The message `michi: not a folder: C:\no\such` in the terminal, exit code 2, no window
- [ ] **5.3** Start `michi --version` and `michi --help` from cmd and from PowerShell → The version or the usage prints in that terminal, exit code 0, no window
- [ ] **5.4** Start michi with no git on the PATH → The chip shows `✗ git was not found`; the keybar shows `⚠ git` in red
- [ ] **5.5** Start michi with a git older than 2.38 on the PATH → A toast for 10 seconds names the version and asks to update; the keybar keeps `⚠ git 2.x.y` in amber
- [ ] **5.6** Start `michi` in a worktree other than the main one → The chip names the repository (not the worktree's folder) and its branch; the count is the whole repository's

## 6. Graph

- [ ] **6.1** Open the repository → Within a second, the newest commits first: the graph, the subject with its names, the author, the date (the time today, `Yesterday` and the time, `M/D HH:MM` this year, `Y/M/D` before), the short hash; the trunk (main) runs down the leftmost lane; the header names the columns; only the graph has a scrollbar
- [ ] **6.2** Look at the names before the subjects → A worktree's branch as a filled pill with a worktree mark (main in the trunk's colour, others in their lane's); a remote branch as a dashed outline (`origin/main`); a tag in amber with a diamond; hovering a worktree pill shows its folder
- [ ] **6.3** Look at the merges into main → Each is folded: a ring on the trunk, a chip `▸N` (the commits inside) before the subject in the merged lane's colour; hovering the chip says how many it folds and names the branches still inside; the header's `Fold PRs z` toggle is lit
- [ ] **6.4** Press `z`, then `z` again → The merges open (side lanes appear) with the toast "Unfolded all the merges."; again, they fold with a toast giving how many merges and commits; the header toggle follows (lit or grey)
- [ ] **6.5** Click the header toggle → The same as `z`
- [ ] **6.6** Select a folded merge with `j`, press `l` (or →), then `h` (or ←) → `l` opens it: the chip becomes `▾N` with a dashed outline and its commits show on their own lane; `h` folds it again
- [ ] **6.7** Click a chip → Its merge opens or folds; the selection stays where it was
- [ ] **6.8** Press `j` `k`, ↓ ↑, PageDown PageUp, Home End → The selected row (lit across) moves and the list scrolls to keep it in sight; End goes to the oldest row read so far
- [ ] **6.9** Click a row; scroll with the wheel → The row is selected; the wheel scrolls the list without moving the selection
- [ ] **6.10** Press `/` and type part of a subject, a hash or an author's name → The field at the right of the tool band takes the keys, with the count (`3 hits`) at its right; the text sits in the middle of the field; the other rows dim; the hits get marks on the scrollbar
- [ ] **6.11** In the field, press Enter, Shift+Enter, ↓, ↑ → Enter selects the next hit (round to the first after the last), Shift+Enter the one before; the arrows move the selection; the field keeps the keys
- [ ] **6.12** Press Esc in the field → The filter clears and every row is bright again
- [ ] **6.13** Look at the scrollbar → Marks for the worktrees' heads (in their lanes' colours), the tags (amber) and the selected row (the accent); hovering a mark tells what it is
- [ ] **6.14** Drag the thumb; click the track; click a mark → The thumb stays under the pointer to the release; a click on the track brings that part into the middle; a click on a mark selects its row
- [ ] **6.15** Look at commits by dependabot[bot] and github-actions[bot] → The author reads `⚙ dependabot` and `⚙ actions`; hovering shows the full name
- [ ] **6.16** Open a repository with more than 2000 commits → The first screen comes at once; scrolling near the foot reads on by itself; the last row says how many older commits are left and reads them when clicked; the thumb shrinks as the list grows
- [ ] **6.17** Look at a second worktree's commits → They run on their own lane in the worktree's colour, from where they left main
- [ ] **6.18** Narrow the window to 720 wide; switch to the light theme → The subject is cut with an ellipsis and the names drop before they crowd it; the filter field stays inside the band; every colour stays readable in light

## 7. Side column

- [ ] **7.1** Look at the column left of the graph → Headings with counts: worktree, Branches, Remotes, Tags; the worktree the window is on is lit; each worktree and branch has a dot in its lane's colour; there is no scrollbar, the wheel scrolls it
- [ ] **7.2** Click a worktree, a branch and a tag → The graph selects that commit and scrolls it into sight; the detail follows
- [ ] **7.3** Click a group (`test/`, `origin`) → It opens to list its branches further in, with the count at its right; again, it closes; hovering a branch inside shows its full name
- [ ] **7.4** Click a branch whose commit is older than what is read → The graph reads on until it gets there and selects it; one that is not in the history gives the toast `Not in the history: <hash>`

## 8. Detail and diff

- [ ] **8.1** Select a commit → Below the graph within a moment: the subject in bold, the full hash, the author, the local time and the parents (blue links), the body, the hint `Enter spreads the diff side by side`; `Reading the commit…` shows while it is read
- [ ] **8.2** Look at the file list → Each file with its status letter in colour (A green, M, D red, R), its path cut from the left with `…` when long, `+a −d` and a bar; a renamed file says where it came from; more than 12 files list 10 and `N more files — click for all`
- [ ] **8.3** Click a file row → The detail scrolls to that file's diff
- [ ] **8.4** Look at the diff → One column: the `@@` rows in blue, added rows green and deleted rows red, the changed words marked a shade deeper, the line numbers at the left; only the detail's wheel scrolls it, with no scrollbar
- [ ] **8.5** Press Enter, then Esc → Enter narrows the graph and spreads the diff side by side (old left, new right, the empty side hatched); the title's toggle follows; Esc brings the graph back, and a second Esc clears the filter
- [ ] **8.6** Click a parent link → The graph selects the parent and the detail shows it
- [ ] **8.7** Select a merge into main → `N commits came into main by this merge` with their hashes, subjects and times, and whether they are folded or open in the graph; the note that a merge's diff comes in the next stage; no diff
- [ ] **8.8** Select a commit with a binary file, and a big one → The binary file says it has no diff to show; a very long diff ends with `… the rest is left out`
- [ ] **8.9** Narrow the window below 900 wide → The side column goes and the detail stacks under the graph, still readable; Enter still spreads the diff
- [ ] **8.10** Switch to the light theme → The diff's greens, reds and blues and the hatching stay readable

## 9. Uncommitted changes

- [ ] **9.1** Look at the top of the graph → A row `Uncommitted: N files (+a −d)` in italics above the newest commit, with a dashed dot on the lane of the worktree's branch, author `—`, the time the change was last written and the hash `WIP`; the main worktree's change is on top when it was written last
- [ ] **9.2** Edit a file in the worktree and wait a few seconds with michi in front → The row's counts and time follow without a key; the selection stays on the row it was on
- [ ] **9.3** Select the WIP row → The detail says `Uncommitted changes on <branch>`, the worktree's folder in mono, when it was last written, `Read only here. Stage and commit in Changes (2).`, then the files and the diff from the worktree's `HEAD`; the new file is listed as added
- [ ] **9.4** With the WIP row selected, have the Claude there run, then stop or ask → A box under the hint: `Claude is writing.` or `Claude has stopped.` with its state (glyph and word), the tab, its title and since when; a question in amber
- [ ] **9.5** Put a file over 10 MB (or a folder over 2000 files or 50 MB) in the worktree, not ignored → The WIP row and its counts leave it out; the detail says `Untracked, too big to count: <path>` (three at most, then `and N more`); `git count-objects -vH` in the repository does not grow by its size; no `git.exe` keeps running after michi is closed
- [ ] **9.6** Make the worktree clean again → The WIP row goes within a few seconds; a worktree whose only change is a big untracked folder has no WIP row
- [ ] **9.7** Look at `git status` and the index after michi read the change → Nothing was staged; `git status` shows the same as before
