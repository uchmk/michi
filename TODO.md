# TODO

`【人】` is the owner's work; `（要確認: #N）` waits for an answer (go on with the recommendation).

## Stages (docs/dev-prompt.md)

- [x] 0. Skeleton (v0.1.0): workspace with `crates/michi` and `crates/michi-core`, ito crates on the rev tsumugi uses, window with the
  title bar, view tabs, empty views, keybar and `?`, settings through ito, icon from `assets/icon.svg`, `rustfmt.toml`, `scripts/verify.sh`,
  CI, issue forms and labels, TESTING.md, CHANGELOG.md, git version check (2.38 or newer). The settings open inside the frame (the
  title bar and the keybar stay, unlike minato); the icon is drawn in code (`icon.rs`), no `.ico`
- [x] 1. See (read only)
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
- [x] 2. Change
  - [x] The diff of a whole branch (`d` in the worktree table, v0.9.0): `main...<branch>` with the uncommitted changes in or out (`u`),
    files checked as seen by path and blob (`Space`), since the last look (`s`, `refs/michi/seen/<branch>`), the prediction against
    main on top, side by side (`Enter`) （要確認: #10）
  - [x] The branch review's leftovers: ~~notes on lines sent to the branch's Claude~~ (`J K n g`, v0.34.0, TESTING 13.13), ~~merge from the review~~ (`m`, v0.29.0, TESTING 13.12),
    `Open PR` (with the PR button, stage 4) （要確認: #10）
  - [x] Stage and commit (`2`, v0.10.0): files, lines and hunks (`git apply --cached`), the message per worktree, amend (asked first
    when the head is pushed), the check while Claude is writing, the count of uncommitted files on the `2` tab （要確認: #12）
  - [x] Stage and commit's leftovers (v0.14.0): splitting the hunks into their runs of changes (`s`, kept across re-reads), the
    output git and the hooks gave on a refused write shown whole in place of the diff (colour codes stripped) （要確認: #12, #20）
  - [x] Fetch / pull / push (v0.11.0): `f` `p` `P` in every view on the worktree the view points at, pull fast-forward only, push with
    the upstream set for a new branch, progress and Esc (the whole process tree) in the key bar, the last fetch's age, the band before
    `--force-with-lease` with the remote's tip kept under `refs/michi/backup/`, the Claude band before a pull （要確認: #15）
  - [x] Stop a pull during a merge (v0.15.0): with the toast why （要確認: #15, #22）
  - [x] Fetch / pull / push's leftovers: pick the remote to push to (v0.28.0, TESTING 15.12) （要確認: #15）
  - [x] Branches (v0.12.0): the palette on `b` (filter, switch, follow a remote one, make at the selected commit with Enter or make and
    switch with Shift+Enter, Delete with the commits no other branch has kept under `refs/michi/backup/`, refusals when another worktree
    has it, during a merge, and for remote branches), the Claude band before a switch （要確認: #16）
  - [x] Tags and revert (v0.12.0): the tag palette on `T` (annotated with a message, light without; push; delete always kept under
    `refs/michi/backup/`), revert on `t` after a band (a merge asks for the parent; one that clashes is called off) （要確認: #16）
  - [x] Cherry-pick and merge (v0.15.0): `C` in the graph picks onto the window's worktree (a merge asks for the parent), `m` in the
    worktree table merges the local trunk into the row's branch, both after a band; a clash opens `4`, and so does a clashing revert
    （要確認: #16, #22）
  - [x] Branches' leftovers (v0.27.0): rename (`F2`), set the upstream (`Ctrl+U`) and rebase onto the row (`Ctrl+R`, after a band; a clash is
    called off, the old tip kept under `refs/michi/backup/`) in the branch palette, `m` in the graph merges the selected commit （要確認: #16）
  - [x] Stash and discard (v0.13.0): the stash palette on `s` (the row that puts the changes away first, untracked files too, the typed
    text as the message and the filter, Enter applies and Shift+Enter pops, a clash keeps the stash, Delete drops it after a band with
    a backup, the stash in the detail and on a click in the side column), discard on `d` in `2` (files, untracked files with their
    empty folders, picked lines; always asked, kept under `refs/michi/backup/` first, staged changes refused) （要確認: #18）
  - [x] Clean-up (v0.14.0): `c` over the worktree table lists the branches merged into the trunk or gone from their remote and the
    worktrees whose folder is gone, picked with Space / `a`, asked with the band; a branch's worktree goes with it, unmerged tips kept
    under `refs/michi/backup/`, and what has to stay (main, here, changes, locked, a tsumugi tab) is listed with why （要確認: #20）
- [x] 3. Conflicts
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
- [x] 4. Worktrees: new work (`N` done in v0.31.0; remove `x` and the tab `o` in v0.30.0), take in main (`m` merge in v0.15.0, `M` rebase in v0.36.0, TESTING 10.17), the PR page (`p`, v0.32.0; michi opens the page, the browser writes the PR per v1-scope)
  and `focus N|NAME` (done in tsumugi v0.110.0, used by `o` since v0.30.0)
- [x] 5. Rebase: reorder, squash, reword, the in-progress band (`r` screen done in v0.33.0, TESTING 10.16; the in-progress band is the conflict screen's, v0.17.0)
- [x] 6. Claude: hand a conflict to Claude through `tsumugi send` (v0.24.0) and notify on waiting / done (v0.26.0), hand over notes from the
  branch diff (v0.34.0, TESTING 13.13). Watch the markers after it stops (v0.35.0, TESTING 21.31). Stage 6 is done.

- [x] 7. The shared spec through ito (v0.37.0–v0.44.0, ito v0.56.1): `--keys` script (`ito-keyscript`), keymap.toml and the Keys page
  (`ito-keymap`, `ito-keys`), the key list (`ito-help`), notifications and the Notifications page (`ito-notify`), F12 bug report
  (`ito-bugreport`), window place saved (`ito_common::window`), TESTING-CHECKS.md by `ito-testcheck`, command palette `Ctrl+Shift+P` (`ito-palette`).
  - [x] Repository screen on `Ctrl+O`: recent list, switch in the window, typed path (v0.50.0, TESTING 28, #26)
  - [x] The `Band` confirmation drawn by `ito-confirm` (v0.51.0, TESTING 29; the owner approved in the session, 2026-10-17, #27)
  - [x] The `Picker` drawn and keyed by `ito-palette` (v0.49.0, TESTING 27); its matching and rows stay in `picker.rs`
  - [x] Header / table column widths moved onto ito-layout's `grips` / `columns` (v0.48.0, ito#7)
  - [x] Draggable column widths (v0.46.0, panes on `ito-layout` since v0.47.0, TESTING 26): the graph view's three columns, the graph header's columns, the worktree table's columns; kept in `layout.toml`
  - [x] Show `keymap_warn` / `notify_warn` (the warnings read from keymap.toml and config.toml) to the user as a toast (v0.45.0, TESTING 25.1)

## Gaps against other Git clients (2026-10-17)

All wait for the owner; none is started.

- [x] submodule: status and update (not entering): `u` on the worktree table, TESTING 129（要確認: #29）
- [x] Git LFS: "not fetched" marker（要確認: #30）
- [x] sparse checkout: notice only（要確認: #31）: a strip says it is on and how many files are left out (TESTING 31.5–31.9)
- [ ] bisect（要確認: #32）
- [x] Non-GitHub hosts: PR page links for GitLab.com, Bitbucket and Azure DevOps from the remote URL (TESTING 101)（要確認: #33）
- [ ] Creating PRs in michi（要確認: #34）
- [x] Signature status (Verified) marker（要確認: #35）
- [ ] Dedicated screens for conflicts a 3-way view cannot show（要確認: #36）
- [x] blame and per-file history（要確認: #37）
- [x] Content search (pickaxe `s:`, regex `g:`) in the `/` filter, asked with Enter（要確認: #38）
- [x] clone: paste an address in the repository screen (Ctrl+O) to clone beside the open repository, TESTING 239; init later（要確認: #39）
- [x] Remote management: `M` in the graph lists the remotes, adds one (name then address) and removes one after a yes (TESTING 40)（要確認: #40）
- [ ] Authentication guidance（要確認: #41）
- [ ] Gitflow（要確認: #42）
- [ ] Built-in editor（要確認: #43）
- [ ] Issue tracker integration（要確認: #44）
- [x] Multi-repository tabs: Ctrl+O / Ctrl+T opens a repository in a tab, Ctrl+Tab / Ctrl+W / Alt+N, tabs kept between runs, TESTING 245（要確認: #45）
- [x] Patch export / apply（要確認: #46）
- [ ] Statistics and contributor graphs（要確認: #47）
- [ ] Diff extras (images, word diff, ignore whitespace, external tool)（要確認: #48）
- [ ] .gitignore editing（要確認: #49）
- [ ] reflog screen（要確認: #50）
- [ ] Commit search scope in the `/` filter（要確認: #51）
- [x] Per-file operations (restore a version from the detail; "open" not done)（要確認: #52）
- [x] Per-worktree author / email display (display only, recommended option of #53)（要確認: #53）
- [x] Co-Authored-By on michi commits (recommended option of #54: `[commit] co_author = "Name <email>"` in config.toml, off by default)（要確認: #54）
- [x] Moving uncommitted changes between worktrees（要確認: #55）
- [ ] Conflict-prediction marks on the graph（要確認: #56）
- [ ] Pre-merge checklist (test / verify results)（要確認: #57）
- [ ] Hook output and rerun on failure（要確認: #58）
- [ ] Speed target for very large repositories（要確認: #59）
- [ ] Accessibility（要確認: #60）
- [ ] Key list (`?`) and macOS Cmd check（要確認: #61）
- [ ] Real-machine checks off Windows（要確認: #62）

## Gaps against other Git clients, round 2: features, operability, looks (2026-10-17)

All wait for the owner; none is started.

- [ ] Undo / Redo on one key（要確認: #64）
- [x] Compare any two commits or branches（要確認: #65）
- [ ] Context menus (commit, branch, file)（要確認: #66）
- [ ] Drag and drop (merge / rebase / cherry-pick)（要確認: #67）
- [ ] Syntax highlighting in diffs（要確認: #68）
- [ ] CI / PR state on the graph（要確認: #69）
- [x] Auto fetch (recommended option of #70): while the window is in front, fetch every 5 minutes (`[net] auto_fetch` in config.toml, minutes, 0 is off); a failure shows small on the keybar, no toast（要確認: #70）
- [ ] Hide / solo branches（要確認: #71）
- [ ] Tree view of changed files（要確認: #72）
- [ ] Progress and cancel for long operations（要確認: #73）
- [ ] Commit message help（要確認: #74）
- [ ] Open the worktree in terminal / editor / explorer（要確認: #75）
- [x] autostash on pull / checkout (recommended option of #76): changes in the way are stashed, kept under `refs/michi/backup/`, and put back; a clash stays marked in the files（要確認: #76）
- [ ] reset: choose soft / mixed / hard（要確認: #77）
- [x] reset: choose soft / mixed / hard（`R`、1 soft・2 mixed・3 hard。要確認: #77）
- [ ] One-button sync（要確認: #78）
- [ ] Fuzzy pickers everywhere（要確認: #79）
- [ ] Same strength of confirmation for destructive operations（要確認: #80）
- [x] First-run tour and empty states: no tour; the empty graph, a single worktree and a missing tsumugi each show one next step (recommended option of #81)（要確認: #81）
- [ ] Key hints on hover（要確認: #82）
- [ ] Keys whose meaning changes per screen（要確認: #83）
- [x] Author avatars（要確認: #84）
- [ ] Bundled monospace font（要確認: #85）
- [ ] Row density (compact / cozy)（要確認: #86）
- [ ] Diff minimap and next change（要確認: #87）
- [ ] Short animations（要確認: #88）
- [ ] HiDPI and Windows 11 frame checks（要確認: #89）
- [x] Loading skeletons（要確認: #90）

## What only michi can do (2026-10-17)

Worktrees and parallel Claude work are the base; no other client has them.

- [x] Snapshot and diff per Claude turn: `refs/michi/turn/<branch>/N` kept when a Claude stops, `y` / `Y` step through the diffs (TESTING 91)（要確認: #91）
- [x] Suggested order to land worktrees on main: a "Land n of m" note in the worktree table (TESTING 92)（要確認: #92）
- [ ] Land a worktree (merge in, verify, ff, clean up)（要確認: #93）
- [x] Review queue of finished Claude work: the worktree table's Review column (unreviewed / in review / flagged / approved) (TESTING 94)（要確認: #94）
- [ ] From a commit to the Claude session that made it（要確認: #95）
- [x] Warn before a new task overlaps a running worktree（要確認: #97）
- [ ] Answer a waiting Claude from a notification in one step（要確認: #98）
- [ ] Hand-off of a conflict to Claude through to review, in one band（要確認: #99）
- [ ] Trial merge of several worktrees in a chosen order（要確認: #100）
- [ ] Ideas from the other uchmk apps (2026-10-17): drag-select copy through `ito-select` in the commit detail (v0.52.0; the changes, conflict and review panes too in v0.54.0) （要確認: #101）; `michi mcp` through `ito-mcp` / `ito-ipc` (v0.53.0, read-only: worktrees and conflicts; the shared pattern is asked of ito, ito#11) （要確認: #102）; PR / CI state from minato （要確認: #103）

## Easy for beginners (2026-10-17)

All wait for the owner; none is started. Related: #64 (Undo), #74, #77, #80-#83.

- [ ] Plain-language display for Git terms（要確認: #104）
- [ ] Hover glossary for terms on screen（要確認: #105）
- [ ] Say what an operation will do before it runs（要確認: #106）
- [ ] Always say where you are in plain words（要確認: #107）
- [ ] Translate git errors into cause and next step（要確認: #108）
- [ ] Explain detached HEAD and the way back（要確認: #109）
- [ ] Stop commits on the wrong branch（要確認: #110）
- [ ] Warn about large files and secrets before committing（要確認: #111）
- [ ] Show what a push will send（要確認: #112）
- [ ] A "calm down" button that saves the current state（要確認: #113）
- [ ] Operation history（要確認: #114）
- [ ] Basic and full modes（要確認: #115）
- [ ] First-run setup guidance (git, name/email, auth)（要確認: #116）
- [ ] Practice repository（要確認: #117）
- [ ] Legend for the graph symbols（要確認: #118）
- [ ] Next-step suggestion from the current state（要確認: #119）
- [ ] Diagram of stage and commit in Changes（要確認: #120）
- [ ] Beginner explanation of conflicts（要確認: #121）
- [ ] Explain worktrees（要確認: #122）
- [ ] Check that everything is reachable by mouse alone（要確認: #123）
- [ ] Commit message writing guide（要確認: #124）
- [ ] Unify Japanese and English terms（要確認: #125）

## Owner

- 【人】 After the skeleton: run the rows of TESTING-CHECKS.md on the real machine (window, title bar, icon, fonts).
