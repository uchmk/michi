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
  - [x] Stage and commit's leftovers (v0.14.0): splitting the hunks into their runs of changes (`s`, kept across re-reads), the
    output git and the hooks gave on a refused write shown whole in place of the diff (colour codes stripped) （要確認: #12, #20）
  - [x] Fetch / pull / push (v0.11.0): `f` `p` `P` in every view on the worktree the view points at, pull fast-forward only, push with
    the upstream set for a new branch, progress and Esc (the whole process tree) in the key bar, the last fetch's age, the band before
    `--force-with-lease` with the remote's tip kept under `refs/michi/backup/`, the Claude band before a pull （要確認: #15）
  - [x] Stop a pull during a merge (v0.15.0): with the toast why （要確認: #15, #22）
  - [ ] Fetch / pull / push's leftovers: pick the remote to push to （要確認: #15）
  - [x] Branches (v0.12.0): the palette on `b` (filter, switch, follow a remote one, make at the selected commit with Enter or make and
    switch with Shift+Enter, Delete with the commits no other branch has kept under `refs/michi/backup/`, refusals when another worktree
    has it, during a merge, and for remote branches), the Claude band before a switch （要確認: #16）
  - [x] Tags and revert (v0.12.0): the tag palette on `T` (annotated with a message, light without; push; delete always kept under
    `refs/michi/backup/`), revert on `t` after a band (a merge asks for the parent; one that clashes is called off) （要確認: #16）
  - [x] Cherry-pick and merge (v0.15.0): `C` in the graph picks onto the window's worktree (a merge asks for the parent), `m` in the
    worktree table merges the local trunk into the row's branch, both after a band; a clash opens `4`, and so does a clashing revert
    （要確認: #16, #22）
  - [ ] Branches' leftovers: rename, set the upstream, rebase onto another branch, `m` in the graph （要確認: #16）
  - [x] Stash and discard (v0.13.0): the stash palette on `s` (the row that puts the changes away first, untracked files too, the typed
    text as the message and the filter, Enter applies and Shift+Enter pops, a clash keeps the stash, Delete drops it after a band with
    a backup, the stash in the detail and on a click in the side column), discard on `d` in `2` (files, untracked files with their
    empty folders, picked lines; always asked, kept under `refs/michi/backup/` first, staged changes refused) （要確認: #18）
  - [x] Clean-up (v0.14.0): `c` over the worktree table lists the branches merged into the trunk or gone from their remote and the
    worktrees whose folder is gone, picked with Space / `a`, asked with the band; a branch's worktree goes with it, unmerged tips kept
    under `refs/michi/backup/`, and what has to stay (main, here, changes, locked, a tsumugi tab) is listed with why （要確認: #20）
- [ ] 3. Conflicts
  - [x] The conflict screen (`4`, v0.15.0): the places with ours / theirs (and the base), picks `o` `t` `b` `B` and back to base, files
    that clashed whole, start a file over (kept under `refs/michi/backup/` first), the done bar with git's message and Ctrl+Enter
    (the Claude band first), call it off after the row (kept first), moving on to the next worktree stopped; locks during a merge
    (pull, `C`, `m`, revert, the `2` commit) with「· マージ中」in red on the chip; cherry-pick （要確認: #22）
  - [x] rerere asked once (v0.19.0, TESTING 21.23): the band on the first `4` of a session when `rerere.enabled` is unset; `y` / `n` write it. Guessing and the ↺ marks (「前に解いた形」, `Enter`, forget) are not done
  - [x] rerere's applied form (v0.20.0, TESTING 21.24): `↺ 前に解いた形` on a file rerere solved, `Enter` accepts, `x` forgets (`git rerere forget` + start over, backed up); the bar counts the ones not looked at
  - [x] intent rows (v0.21.0, TESTING 21.25): under the selected text place, one row each for ours / theirs with the commit that changed those lines (`git log -L`; hash, subject, author, date), read on the action thread once per place
  - [x] `e` edits the result (v0.22.0, TESTING 21.26): the open place's result box becomes a field (ours then theirs), Ctrl+Enter solves with the lines written (`conflict::resolve_with`, tag ここで書いた), Esc leaves.
  - [x] word diff (v0.23.0, TESTING 21.27): the words of ours / theirs that the base (else the other side) lacks are marked (`michi_core::words::marks`).
  - [x] hand it to Claude (v0.24.0, TESTING 21.28): `g` / the bar's button writes a brief (`michi_core::handoff`, in the temp dir) and runs `tsumugi send N` to the Claude in that worktree or `tsumugi new … -- claude` (`Task::Handoff`). The toast when that Claude stops: v0.26.0, TESTING 21.30.
  - [x] open in an external tool (v0.25.0, TESTING 21.29): `w` / the whole-file place's button runs `git mergetool --no-prompt` when `merge.tool` is set, else opens the file with the system's program (`michi_core::external`, `Task::External`).
  - [x] ~~Prediction moved to gix-merge~~: tried and dropped (2026-10-16): gix 0.89 was 4–15× slower than `git merge-tree --write-tree`
    (100–420 ms against 25–40 ms) and counted hunks differently (git joins near ones). Stays on the git command.
    Done (v0.18.0 / v0.18.1, TESTING 21.21–21.22; the graph tool band dims `Pull` and `Stash`): the graph's merging row (a WIP row with `theirs`, `Dot::Conflict`); the prediction pills / column / pairs are hidden while merging (`App::merging`) and the pairs' note says「予測は衝突の解消（4）の実物に変わりました」
    （要確認: #22）
- [ ] 4. Worktrees: new work (`N`), remove (`x`), go to the tab (`o`), take in main (`m`, merge done in v0.15.0; rebase not yet), the PR button (`gh pr create`); needs tsumugi `new --worktree BRANCH --from REF`
  and `focus N|NAME` (in tsumugi's TODO.md)
- [ ] 5. Rebase: reorder, squash, reword, the in-progress band
- [ ] 6. Claude: hand a conflict to Claude through `tsumugi send` and watch the markers, notify on waiting / done, hand over notes from the
  branch diff

## Owner

- 【人】 After the skeleton: run the rows of TESTING-CHECKS.md on the real machine (window, title bar, icon, fonts).
