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
| K4.12 | `D` | Compare the commit marked with Ctrl+click and the selected one | [ ] |
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

## To be filled as the redesign lands (issue #133)

- `g` popup, still to come: `g d` discard, `g a` amend, `g h` hand to Claude, `g u` unstage (Changes and Conflict screens), and moving the one-key versions into it.
- `y` copy, still to come: `y s` file names, `y f` path, `y u` URL, and `y` in the Changes screen.
- `Space` marks up to two commits, `D` compares, `Esc` clears.
- `?` deep search (Tab changes the mode); `/` stays the light filter.
- Panes: `Tab` / `Shift+Tab` move between the side column, the tree and the detail pane; `Enter` in the detail pane widens it.
