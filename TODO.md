# TODO

`【人】` is the owner's work; `（要確認: #N）` waits for an answer (go on with the recommendation).

## Stages (docs/dev-prompt.md)

- [ ] 0. Skeleton (v0.1.0): workspace with `crates/michi` and `crates/michi-core`, ito crates on the rev tsumugi uses, window with the
  title bar, view tabs, empty views, keybar and `?`, settings through ito, icon from `assets/icon.svg`, `rustfmt.toml`, `scripts/verify.sh`,
  CI, issue forms and labels, TESTING.md, CHANGELOG.md, git version check (2.38 or newer)
- [ ] 1. See (read only): `ito-graph` in ito, the graph with main on the left lane, folding lane PRs (`z`, on by default), the left column,
  the commit detail and its diff, the worktree table with the Claude column (`tsumugi ls --json`), conflict prediction including
  uncommitted changes (`git merge-tree --write-tree`), file watching, unseen marks; measure the first screen on a 100k-commit repository
  (target 0.3 s)
- [ ] Second mock round before stage 2: changes and staging, the diff of a whole branch (seen marks, since last time, notes), rebase;
  ask the owner with a `question` issue
- [ ] 2. Change: stage and commit (lines and hunks, the check while Claude is writing), branches, fetch / pull / push with progress,
  the everyday operations in design.md (amend, revert, tags, stash, discard, clean-up), backups to `refs/michi/backup/<time>`
- [ ] 3. Conflicts: the conflict screen (`4`), locks during a merge, rerere asked once, cherry-pick; prediction moved to gix-merge and
  checked against `git merge-tree --write-tree`
- [ ] 4. Worktrees: new work (`N`), remove (`x`), go to the tab (`o`), take in main (`m`); needs tsumugi `new --worktree BRANCH --from REF`
  and `focus N|NAME` (in tsumugi's TODO.md)
- [ ] 5. Rebase: reorder, squash, reword, the in-progress band
- [ ] 6. Claude: hand a conflict to Claude through `tsumugi send` and watch the markers, notify on waiting / done, hand over notes from the
  branch diff

## Owner

- 【人】 After the skeleton: run the rows of TESTING-CHECKS.md on the real machine (window, title bar, icon, fonts).
