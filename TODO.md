# TODO

`【人】` is the owner's work; `（要確認: #N）` waits for an answer (go on with the recommendation).

## Stages (docs/dev-prompt.md)

- [x] 0. Skeleton (v0.1.0): workspace with `crates/michi` and `crates/michi-core`, ito crates on the rev tsumugi uses, window with the
  title bar, view tabs, empty views, keybar and `?`, settings through ito, icon from `assets/icon.svg`, `rustfmt.toml`, `scripts/verify.sh`,
  CI, issue forms and labels, TESTING.md, CHANGELOG.md, git version check (2.38 or newer). The settings open inside the frame (the
  title bar and the keybar stay, unlike minato); the icon is drawn in code (`icon.rs`), no `.ico`
- [ ] 1. See (read only)
  - [x] `ito-graph` in ito (ito v0.36.0)
  - [x] The graph (v0.2.0): main on the left lane, pills, folding lane PRs (`z`, on by default), movement keys, `/` filter, the scrollbar
    with its marks, batches of 2000 / 5000
  - [x] Measure the first screen on a 100k-commit repository (v0.2.1, `scripts/make-history.py` and the ignored test `big_history`):
    0.09–0.11 s with a commit-graph, 0.31 s without (git's `--date-order` walks everything first). Fully read in about 0.6 s
  - [x] Write a commit-graph in the background when the repository has none (#2, v0.7.2): 0.31 s → 0.12 s on the next opening
  - [ ] Folding and the filter cost 56 ms and 42 ms at 100k commits, on the UI thread at every key: lay out on the reading thread, or only
    the rows that changed
  - [x] The left column and the commit detail with its diff (v0.3.0)
  - [ ] The detail's leftovers: tell a file that is not UTF-8 (now read lossily), wrap long diff lines (now cut at the pane's edge),
    the stash section and the `↑N` / `↓N` counts in the left column
  - [x] The WIP rows (uncommitted changes of each worktree) and the Claude in that worktree in their detail (v0.4.0); untracked files
    over 10 MB, or past 2000 files or 50 MB in all, are left out and named
  - [x] A worktree whose only change is a big untracked folder has no WIP row: say so somewhere (the worktree table, v0.5.0)
  - [x] The worktree table with the Claude column (`tsumugi ls --json`), its cards below 960 wide (v0.5.0)
  - [x] Conflict prediction in the UI (⚠ on heads, a dashed frame on WIP rows, the column and the pair table in the worktree table,
    v0.6.0): the worktrees and the trunk paired, every 5 s while in front
  - [ ] Conflict prediction's leftovers: every local branch against main (not only the worktrees), and the text while a merge is
    under way (stage 3)
  - [x] File watching (`notify` → `Ask::Reload`) and unseen marks (also the worktree table's unseen column) (v0.7.0)
- [ ] Second mock round before stage 2: changes and staging, the diff of a whole branch (seen marks, since last time, notes), rebase;
  ask the owner with a `question` issue
- [ ] 2. Change: first the diff of a whole branch (`d`; seen marks, since last time, notes), then stage and commit (lines and hunks,
  the check while Claude is writing, the count of uncommitted files on the `2` tab), branches, fetch / pull / push with progress, the everyday operations in design.md (amend, revert, tags, stash, discard, clean-up), backups to `refs/michi/backup/<time>`
- [ ] 3. Conflicts: the conflict screen (`4`), locks during a merge (and「· マージ中」in red on the chip), rerere asked once, cherry-pick; prediction moved to gix-merge and
  checked against `git merge-tree --write-tree`
- [ ] 4. Worktrees: new work (`N`), remove (`x`), go to the tab (`o`), take in main (`m`), the PR button (`gh pr create`); needs tsumugi `new --worktree BRANCH --from REF`
  and `focus N|NAME` (in tsumugi's TODO.md)
- [ ] 5. Rebase: reorder, squash, reword, the in-progress band
- [ ] 6. Claude: hand a conflict to Claude through `tsumugi send` and watch the markers, notify on waiting / done, hand over notes from the
  branch diff

## Owner

- 【人】 After the skeleton: run the rows of TESTING-CHECKS.md on the real machine (window, title bar, icon, fonts).
