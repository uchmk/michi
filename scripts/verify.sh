#!/usr/bin/env bash
# Every check a push to main must pass. The first failure stops with its
# output; the last line is `ALL OK: ...` when everything passed. Like
# tsumugi's and minato's scripts/verify.sh.
#
# Clippy covers every OS michi ships for -- Linux, Windows and macOS --
# whatever the host: the host first, then the others. Those are type checks
# only -- they never run that code, so read Windows path strings by eye.
# michi has no C to build for any of them, so they run on every machine (the
# target is added when it is missing). Before 2026-10-15 the second clippy
# was always Windows, so a run on the Windows PC never compiled Linux and
# still said ALL OK (yagura's v0.62.0 went red on Linux that way); before
# 2026-10-16 macOS was left to the Mac, which CI here does not have.
# `cargo fmt --check` only looks; run `cargo fmt --all` to fix what it finds.

set -o pipefail
cd "$(dirname "$0")/.." || exit 1

run() {
    local out rc
    out=$("$@" 2>&1)
    rc=$?
    if [ $rc -ne 0 ]; then
        echo "FAILED: $*"
        echo "$out" | tail -30
        exit 1
    fi
    last=$out
}

run cargo fmt --all --check
run cargo build -q --locked --workspace --all-features
run cargo test -q --workspace --all-features
tests=$(echo "$last" | grep 'test result' | awk '{p+=$4; f+=$6} END {print p " passed; " f " failed"}')
run cargo +stable clippy -q --workspace --all-targets --all-features -- -D warnings
host=$(rustc +stable -vV | sed -n 's/^host: //p')
for t in x86_64-unknown-linux-gnu x86_64-pc-windows-msvc aarch64-apple-darwin; do
    [ "$t" = "$host" ] && continue
    rustup +stable target list --installed 2>/dev/null | grep -qx "$t" \
        || rustup +stable target add "$t" >/dev/null 2>&1
    run cargo +stable clippy -q --workspace --all-targets --all-features --target "$t" -- -D warnings
done
# The checklist follows TESTING.md.
run cargo run -q -p michi --example make-testcheck -- --check
run python scripts/ito-first.py
echo "ALL OK: $tests"
