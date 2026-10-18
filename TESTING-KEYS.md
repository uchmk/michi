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

## Every screen (as the key list shows them)

| # | Key | Expect (the key bar's words) | Checked |
| --- | --- | --- | --- |
| K3.1 | `1 2 3 4` | Switch the view | [ ] |
| K3.2 | `N` | New work (worktree + Claude) | [ ] |
| K3.3 | `W` | Go to the waiting Claude: its diff and question, and answer it | [ ] |
| K3.4 | `f` | Fetch | [ ] |
| K3.5 | `p` | Pull | [ ] |
| K3.6 | `P` | Push | [ ] |
| K3.7 | `F` | Push over the remote's commits (asks first) | [ ] |
| K3.8 | `S` | Sync: pull (fast-forward only), then push | [ ] |
| K3.9 | `K` | Sign-in help | [ ] |

## Graph screen (today's keys, before the redesign)

| # | Key | Expect (the key bar's words) | Checked |
| --- | --- | --- | --- |
| K4.1 | `j k` | Move | [ ] |
| K4.2 | `Enter` | Side by side | [ ] |
| K4.3 | `/` | Filter | [ ] |
| K4.4 | `z` | Fold the lane PRs | [ ] |
| K4.5 | `o` | Go to the tab | [ ] |
| K4.6 | `l h` | Open or fold a merge | [ ] |
| K4.7 | `b` | Make or switch a branch | [ ] |
| K4.8 | `t` | Revert the commit (asks first) | [ ] |
| K4.9 | `R` | Reset the branch to the commit: soft, mixed or hard (asks first) | [ ] |
| K4.10 | `C` | Cherry-pick the commit (asks first) | [ ] |
| K4.11 | `m` | Merge the commit into this worktree (asks first) | [ ] |
| K4.12 | `Space` then `D` | Compare the commit marked with Space (or Ctrl+click) and the selected one | [ ] |
| K4.13 | `T` | Tag the commit | [ ] |
| K4.14 | `s` | Stash | [ ] |
| K4.15 | `M` | Remotes | [ ] |
| K4.16 | `L` | Reflog | [ ] |
| K4.17 | `Z` | Statistics | [ ] |
| K4.18 | `B` | Blame the file | [ ] |
| K4.19 | `H` | History of the file | [ ] |
| K4.20 | `Y` | Copy as a patch | [ ] |
| K4.21 | `#` | Copy the commit hash | [ ] |
| K4.22 | `[ ]` | Go to the parent / the merged-in commit | [ ] |
| K4.23 | `U` | Gitflow | [ ] |
| K4.24 | `I` | Open the issue | [ ] |
| K4.25 | `Ctrl+V` | Paste a patch to apply it | [ ] |
| K4.26 | `w` | Ignore whitespace in diffs | [ ] |
| K4.27 | `x y` | Bisect: mark the commit bad / good | [ ] |
| K4.28 | `u` | Bisect: skip the commit to test | [ ] |
| K4.29 | `r` | Copy a git bisect run line | [ ] |
| K4.30 | `X` | End the bisect / drop the marks | [ ] |
| K4.31 | `O` | Image diff: side by side, swipe, onion, difference | [ ] |
| K4.32 | `Esc` | Back | [ ] |

## Changes screen (today's keys, before the redesign)

| # | Key | Expect (the key bar's words) | Checked |
| --- | --- | --- | --- |
| K5.1 | `Space` | Stage | [ ] |
| K5.2 | `c` | Commit | [ ] |
| K5.3 | `l h` | Pick lines, back to the files | [ ] |
| K5.4 | `a` | Stage all (on the lines: the hunk) | [ ] |
| K5.5 | `j k` | Select | [ ] |
| K5.6 | `v` | A range of lines | [ ] |
| K5.7 | `s` | Split the hunks into their runs of changes, or join them back | [ ] |
| K5.8 | `n N` | Jump to the next or previous change in the diff | [ ] |
| K5.9 | `d` | Throw the changes away (asks first) | [ ] |
| K5.10 | `i` | Add the untracked file to .gitignore | [ ] |
| K5.11 | `w` | Ignore whitespace in diffs | [ ] |
| K5.12 | `O` | Image diff: side by side, swipe, onion, difference | [ ] |
| K5.13 | `A` | Amend the last commit | [ ] |
| K5.14 | `B` | Blame the file | [ ] |
| K5.15 | `H` | History of the file | [ ] |
| K5.16 | `e` | Open the file in the editor at this line | [ ] |
| K5.17 | `x` | Open the file's diff in the external diff tool | [ ] |
| K5.18 | `[ ]` | Another worktree | [ ] |
| K5.19 | `Enter` | Side by side | [ ] |
| K5.20 | `Esc` | Back | [ ] |

## Worktree screen (today's keys, before the redesign)

| # | Key | Expect (the key bar's words) | Checked |
| --- | --- | --- | --- |
| K6.1 | `j k` | Select | [ ] |
| K6.2 | `o` | Go to the tab | [ ] |
| K6.3 | `O` | Open the PR | [ ] |
| K6.4 | `I` | Open the issue | [ ] |
| K6.5 | `Q` | Create a PR (gh) | [ ] |
| K6.6 | `t e E` | Open the folder in the terminal, editor, file manager | [ ] |
| K6.7 | `r` | Rebase: reorder, fold, reword, drop | [ ] |
| K6.8 | `M` | Rebase the branch onto main (asks first) | [ ] |
| K6.9 | `Enter` | Show in the graph | [ ] |
| K6.10 | `m` | Take in main | [ ] |
| K6.11 | `l` | Land on main: take in, check, fast-forward, clean up | [ ] |
| K6.12 | `x` | Remove | [ ] |
| K6.13 | `d` | Diff of the whole branch | [ ] |
| K6.14 | `c` | Clean up | [ ] |
| K6.15 | `T` | Trial merge | [ ] |
| K6.16 | `L` | Reflog | [ ] |
| K6.17 | `Z` | Statistics | [ ] |
| K6.18 | `y` | Diff per Claude turn | [ ] |
| K6.19 | `u` | Submodules: state and update | [ ] |
| K6.20 | `v` | Move changes | [ ] |
| K6.21 | `U` | Gitflow | [ ] |
| K6.22 | `R` | Check before merging | [ ] |

## Conflict screen (today's keys, before the redesign)

| # | Key | Expect (the key bar's words) | Checked |
| --- | --- | --- | --- |
| K7.1 | `] [` | Next / previous place | [ ] |
| K7.2 | `o` | Take ours | [ ] |
| K7.3 | `t` | Take theirs | [ ] |
| K7.4 | `b B` | Take both (ours first / theirs first) | [ ] |
| K7.5 | `Enter` | Accept the form rerere applied | [ ] |
| K7.6 | `e` | Write the result by hand | [ ] |
| K7.7 | `g` | Hand the clashes to a Claude | [ ] |
| K7.8 | `d a r` | After a hand-over: look at the diff, adopt, send back | [ ] |
| K7.9 | `w` | Open the file in an external tool | [ ] |
| K7.10 | `E` | Open the file in the editor at the block's line | [ ] |
| K7.11 | `x` | Forget the form rerere applied; solve it again | [ ] |
| K7.12 | `Ctrl+Enter` | Commit once all is solved | [ ] |
| K7.13 | `PageDown PageUp Ctrl+D Ctrl+U Alt+J Alt+K` | Scroll the diff | [ ] |

## The `g` and `y` windows (0.77.0)

| # | Key | Expect | Checked |
| --- | --- | --- | --- |
| K8.1 | `g` (graph) | A window lists g first row, r reset (red), b bisect, ? sheet; Esc closes it | [ ] |
| K8.2 | `g g` | The first row is selected | [ ] |
| K8.3 | `g r` | The reset question opens | [ ] |
| K8.4 | `g b`, then `y` / `x` | Bisect window; marks the commit good / bad | [ ] |
| K8.5 | `g ?` | The Git flow sheet opens | [ ] |
| K8.6 | `g` (worktree table) | Lists x remove, l land, c tidy, m rebase, v move, f force push (red) | [ ] |
| K8.7 | `g x`, `g l`, `g c`, `g m`, `g v`, `g f` | The same question or window as the one-key version | [ ] |
| K8.8 | `y` (graph) | A window lists h hash, p patch, m message, a author | [ ] |
| K8.9 | `y h`, `y p`, `y m`, `y a` | The clipboard holds the hash, the patch, the first line, `name <email>` | [ ] |
| K8.10 | `y b`, `y d` (worktree table) | The clipboard holds the branch name, the folder | [ ] |
| K8.11 | `Y` (worktree table) | The Claude turns open (they were on `y`) | [ ] |
| K8.12 | `x`, move, `y` (graph) | `y` marks the commit good while only a bad one is marked | [ ] |
| K8.13 | `g`, then a key not in the window | The window closes and a toast says so | [ ] |

## Panes of the graph view (0.78.0)

| # | Key | Expect | Checked |
| --- | --- | --- | --- |
| K9.1 | `Tab` / `Shift+Tab` (graph) | The keys go graph → side column → detail (backwards with Shift); a toast names the pane | [ ] |
| K9.2 | `j k` in the side column | A blue bar walks the rows, skipping headings, and scrolls into view | [ ] |
| K9.3 | `Enter` or `l` on a branch, tag, worktree | The graph selects its commit and the keys return to the graph | [ ] |
| K9.4 | `Enter` on a group or a stash | The group opens or closes; a stash shows in the detail | [ ] |
| K9.5 | `j k`, `G` in the detail | The detail scrolls; a blue frame shows it has the keys | [ ] |
| K9.6 | `Enter` in the detail | It widens (side by side); `Tab` then only goes graph ↔ detail | [ ] |
| K9.7 | `Esc` in a pane | The keys go back to the graph | [ ] |

## Deep search (0.79.0)

| # | Key | Expect | Checked |
| --- | --- | --- | --- |
| K10.1 | `?` (graph) | The filter field takes the keys with `m:` in it | [ ] |
| K10.2 | `Tab` in the field | The prefix goes `m:` → `p:` → `s:` → `f:` → `m:`, the text kept | [ ] |
| K10.3 | `Enter` after typing | The field says it is asking; then only the matching commits are bright, `Enter` goes to the next | [ ] |
| K10.4 | `m:` / `p:` / `s:` / `f:` with a word you know | Message / path / diff text / any of them hit the right commits | [ ] |
| K10.5 | `F1` | The key list opens; `?` no longer does | [ ] |
| K10.6 | The window stays alive while a search runs in a big repository | You can still move and scroll | [ ] |

## Marking commits (0.80.0)

| # | Key | Expect | Checked |
| --- | --- | --- | --- |
| K11.1 | `Space` (graph) | The selected commit gets the mark | [ ] |
| K11.2 | `j`, then `D` | The two are compared (the review form) | [ ] |
| K11.3 | `Space` on the marked commit; `Esc` | The mark goes | [ ] |
| K11.4 | `D` with no mark | A toast says how to mark | [ ] |

## Detail file list (0.81.0)

| # | Key | Expect | Checked |
| --- | --- | --- | --- |
| K12.1 | Select a commit that changes 13+ files | The detail lists every file; no "N more files" row | [ ] |
| K12.2 | `Tab` to the detail, `j k` | The long list scrolls with the diff | [ ] |
| K12.3 | Select a commit | The detail shows the diff side by side (old left, new right) before `Enter` is pressed | [ ] |
| K12.4 | `Enter` in the detail | The detail widens (the graph narrows); `Esc` narrows it back, still side by side | [ ] |
| K13.1 | Worktree table, `s` | The rows reorder by the landing order (the trunk's first); a toast says so; `s` again restores the listed order and the selection stays on its worktree | [ ] |
| K13.2 | `Enter` or double-click on a row not yet approved | The branch's whole diff opens (the review) | [ ] |
| K13.3 | `Enter` or double-click on an approved row, or the trunk's | The graph opens on it | [ ] |
| K13.4 | `l` on a row not yet approved | The band carries the note that it is not approved; landing still works | [ ] |
| K13.5 | Settings → General → Diff layout | Unchecking it writes `diff_side = false` to config.toml's `[view]`; the detail shows one column until Enter | [ ] |
| K14.1 | Changes, `Tab` on a file with changed lines | The keys go to the lines (as `l`); a toast says so; `Tab` or `Shift+Tab` again returns to the files | [ ] |
| K14.2 | Conflict, `Tab` | The keys go to the places' column; `j` `k` and the arrows scroll it by rows; `Esc` returns to the file list without leaving the screen | [ ] |
| K14.3 | Worktree table → `d` (review), `Tab` | The keys go to the diff; `j` `k` scroll it, not the file selection; `Esc` returns, a second `Esc` closes the review | [ ] |
| K14.4 | F1 on those three screens | `Tab` is listed in each | [ ] |
| K15.1 | Changes, `g` | A window lists `g` first row, `d` discard, `a` amend, `i` ignore; `d` alone does nothing | [ ] |
| K15.2 | Changes, `g d` then `y` | The question for the file is asked; `y` says yes without opening the copy window | [ ] |
| K15.3 | Conflict, `g` | The window lists first row, `h` hand to Claude, `x` forget; `g g` goes to the first place | [ ] |
| K15.4 | Changes or Conflict, `y f` / `y n` | The path / the file name is on the clipboard; a toast says so | [ ] |
| K15.5 | Right-click a file in Changes → discard | Same as `g d` | [ ] |
| K15.6 | Graph, `Tab` three times | The keys go tree → detail → side column → tree; the blue frame follows them | [ ] |
| K15.7 | Graph, side column, `→` or `l` | Nothing moves; `Enter` opens the row and returns to the tree | [ ] |
| K16.1 | Graph, `R` alone | Nothing happens; `g r` asks about reset | [ ] |
| K16.2 | Worktree table, `x` `l` `c` `M` alone | Nothing happens; `g x` `g l` `g c` `g m` do what they did | [ ] |
| K16.3 | `F` alone in any view | Nothing happens; `g f` force-pushes (after its question) | [ ] |
| K16.4 | Right-click a worktree row → remove | Same as `g x` | [ ] |
| K17.1 | Graph, Tab to the detail, `n` `N` | The blue bar moves down and up the file list | [ ] |
| K17.2 | Graph detail, `Enter` | The diff scrolls to the file the bar is on | [ ] |
| K18.1 | Graph tree, `Enter` | The blue frame moves to the detail | [ ] |
| K18.1b | Graph detail, `Enter` then `Esc` | `Enter` flips side by side/one column (width unchanged); `Esc` restores width and puts the keys on the list; `j`/`k` and arrows move the same pane | [ ] |
| K18.4 | Graph detail header | Shows `N files · +A ~C −D lines` for the whole commit | [ ] |
| K18.5 | Changes, several files marked, `y f` / `y n` | Every marked path/name is on the clipboard, one per line | [ ] |
| K18.6 | Graph detail header, and the strip under the Changes list | `N files · M lines in all · +A ~C −D lines`; the lines are those of the changed files after the change | [ ] |
| K18.7 | Worktrees: merge a branch so two worktrees stop conflicting | A toast `X と Y の衝突は解消済みです`; the table shows `✓ Y との衝突は解消済み` and the pair table a green ✓ | [ ] |
| K18.8 | Graph or Changes, `y d` | The repository folder path is on the clipboard | [ ] |
| K18.9 | Worktrees: click the 着地の順 button, the table's name heading, or press `s` | The table sorts by landing order; the button stays lit and the heading says `↓ 着地の順`; again restores the listed order | [ ] |
| K18.2 | Graph detail, `[` `]` | The diff jumps to the previous and next file; the list does not move | [ ] |
| K18.3 | Changes and review screens | The diff is side by side from the start | [ ] |

## To be filled as the redesign lands (issue #133)

- `g` popup, still to come: `g u` unstage.
- `y` copy, still to come: `y u` URL.
- `?` deep search: rows that do not hit are still drawn faint, not taken away (a "only hits" view is still to come).
- Panes: Tab on the worktree table itself (one pane) does nothing, on purpose.

### K19 タイトルバーと早見表 (0.89.0)

- [ ] K19.1 タイトルバー — コマンドパレットが窓の中央、右隣にベル（トーストが出ると未読の数）。押すと一覧、Esc か外のクリックで閉じて既読。右端にリポジトリ、その左に「設定」。
- [ ] K19.2 `i` — Git フロー早見表が開く。最初のタブは樹形図の凡例。別のタブで閉じて開き直すと、そのタブから開く。
