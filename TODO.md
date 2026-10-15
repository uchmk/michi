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
  - [x] Folding and the filter cost 56 ms and 42 ms at 100k commits, on the UI thread at every key: lay out on the reading thread, or only
    the rows that changed (v0.8.0: rows are laid out as they come in sight, the filter keeps each commit's text; `z` 3.5 ms, a filter key
    4–7 ms. `z` at the end of a fully read history still takes 72 ms)
  - [x] The left column and the commit detail with its diff (v0.3.0)
  - [x] The detail's leftovers: tell a file that is not UTF-8 (now read lossily), wrap long diff lines (now cut at the pane's edge),
    the stash section and the `↑N` / `↓N` counts in the left column (v0.8.0; a click on a stash does nothing until the stash stage)
  - [x] The WIP rows (uncommitted changes of each worktree) and the Claude in that worktree in their detail (v0.4.0); untracked files
    over 10 MB, or past 2000 files or 50 MB in all, are left out and named
  - [x] A worktree whose only change is a big untracked folder has no WIP row: say so somewhere (the worktree table, v0.5.0)
  - [x] The worktree table with the Claude column (`tsumugi ls --json`), its cards below 960 wide (v0.5.0)
  - [x] Conflict prediction in the UI (⚠ on heads, a dashed frame on WIP rows, the column and the pair table in the worktree table,
    v0.6.0): the worktrees and the trunk paired, every 5 s while in front
  - [x] Conflict prediction's leftovers: every local branch against main (not only the worktrees; v0.9.0, the latest 30 that main
    has not taken in, ⚠ on their heads in the graph). The text while a merge is under way is in stage 3
  - [x] File watching (`notify` → `Ask::Reload`) and unseen marks (also the worktree table's unseen column) (v0.7.0)
- [x] ~~Second mock round before stage 2~~: dropped (the owner, 2026-10-15: go on with the implementation). What stage 2 has to
  decide goes into `question` issues in words, and the work goes on with the recommendation
- [ ] 2. Change
  - [x] The diff of a whole branch (`d` in the worktree table, v0.9.0): `main...<branch>` with the uncommitted changes in or out (`u`),
    files checked as seen by path and blob (`Space`), since the last look (`s`, `refs/michi/seen/<branch>`), the prediction against
    main on top, side by side (`Enter`) （要確認: #10）
  - [ ] The branch review's leftovers: notes on lines sent to the branch's Claude (stage 6), merge from the review (with `m`, stages 3–4),
    `Open PR` (with the PR button, stage 4) （要確認: #10）
  - [x] Stage and commit (`2`, v0.10.0): files, lines and hunks (`git apply --cached`), the message per worktree, amend (asked first
    when the head is pushed), the check while Claude is writing, the count of uncommitted files on the `2` tab （要確認: #12）
  - [ ] Stage and commit's leftovers: splitting a hunk (`s` of `git add -p`), the commit hooks' output shown whole (now one line in a
    toast); discard and stash are in the everyday operations below （要確認: #12）
  - [x] Fetch / pull / push (v0.11.0): `f` `p` `P` in every view on the worktree the view points at, pull fast-forward only, push with
    the upstream set for a new branch, progress and Esc (the whole process tree) in the key bar, the last fetch's age, the band before
    `--force-with-lease` with the remote's tip kept under `refs/michi/backup/`, the Claude band before a pull （要確認: #15）
  - [ ] Fetch / pull / push's leftovers: stop a pull during a merge (with the merge state, stage 3), push a tag (with the tags), pick the
    remote to push to （要確認: #15）
  - [ ] Branches: the picker on `b` (filter, switch, create at the selected commit, delete with a backup when not merged, a note when
    another worktree has it checked out)
  - [ ] The everyday operations in design.md (amend, revert, tags, stash, discard, clean-up), backups to `refs/michi/backup/<time>`
- [ ] 3. Conflicts: the conflict screen (`4`), locks during a merge (and「· マージ中」in red on the chip), rerere asked once, cherry-pick; prediction moved to gix-merge and
  checked against `git merge-tree --write-tree`; the prediction's text while a merge is under way (「予測は衝突の解消（4）の実物に変わりました」)
- [ ] 4. Worktrees: new work (`N`), remove (`x`), go to the tab (`o`), take in main (`m`), the PR button (`gh pr create`); needs tsumugi `new --worktree BRANCH --from REF`
  and `focus N|NAME` (in tsumugi's TODO.md)
- [ ] 5. Rebase: reorder, squash, reword, the in-progress band
- [ ] 6. Claude: hand a conflict to Claude through `tsumugi send` and watch the markers, notify on waiting / done, hand over notes from the
  branch diff

## Owner

- 【人】 After the skeleton: run the rows of TESTING-CHECKS.md on the real machine (window, title bar, icon, fonts).
