#!/usr/bin/env bash
# Every check a push to main must pass. The first failure stops with its
# output; the last line is `ALL OK: ...` when everything passed. Like
# tsumugi's and minato's scripts/verify.sh.
#
# The Windows clippy is a type check only -- it never runs Windows code, so
# read path strings by eye. michi has no C to build, so it runs on every
# machine (the target is added when it is missing). `cargo fmt --check` only
# looks; run `cargo fmt --all` to fix what it finds.

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
rustup +stable target list --installed 2>/dev/null | grep -qx x86_64-pc-windows-msvc \
    || rustup +stable target add x86_64-pc-windows-msvc >/dev/null 2>&1
run cargo +stable clippy -q --workspace --all-targets --all-features --target x86_64-pc-windows-msvc -- -D warnings
# The checklist follows TESTING.md.
run cargo run -q -p michi --example make-testcheck -- --check
echo "ALL OK: $tests"
