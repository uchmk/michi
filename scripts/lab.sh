#!/usr/bin/env bash
# Adds worktrees with overlapping work to a practice repository, so that the
# conflict prediction (the graph's warning, the worktree table's pairs) and the
# unseen-changes count have something to show:
#   scripts/lab.sh R:/lab/michi-practice-1
# Open the repository in michi first, then run it again to see new commits count as unseen.
set -euo pipefail
repo=${1:?usage: lab.sh <practice repository>}
cd "$repo"
export GIT_OPTIONAL_LOCKS=0 GIT_TERMINAL_PROMPT=0
wt() { # name branch
  [ -d "../$1" ] || git worktree add -q "../$1" -b "$2" topic-a
}
put() { # dir file text message
  printf '%b' "$3" > "$1/$2"
  git -C "$1" add -- "$2"
  git -C "$1" commit -q -m "$4"
}
wt "$(basename "$repo")-wt-a" lab-a
wt "$(basename "$repo")-wt-b" lab-b
wt "$(basename "$repo")-wt-c" lab-c
a="../$(basename "$repo")-wt-a" b="../$(basename "$repo")-wt-b" c="../$(basename "$repo")-wt-c"
# lab-a and lab-b change the same colors.txt line differently: a conflict between them (and with topic-b).
put "$a" colors.txt 'red\nolive\nblue\n' "Make lime olive"
put "$a" shared.txt 'from a\n' "Add shared.txt in a"
put "$b" colors.txt 'red\nmint\nblue\n' "Make lime mint"
put "$b" notes.txt 'Things to remember.\nBuy stamps.\n' "Note the stamps"
# lab-c commits cleanly; its uncommitted edit of notes.txt meets lab-b's commit only as a waiting change.
put "$c" extra.txt 'extra\n' "Add extra.txt"
printf 'Things to remember.\nCall the bank.\n' > "$c/notes.txt"
# lab-a also adds shared.txt differently, uncommitted in lab-b: the same file as a new file on both sides.
printf 'from b\n' > "$b/shared.txt"
git worktree list
