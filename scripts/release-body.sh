#!/usr/bin/env bash
# Write a whole release page for TAG, built at REF: the per-version notes
# (scripts/release-notes.sh), the platforms note, and the link to CHANGELOG.md.
# Prints Markdown on stdout. The SHA-256 table goes on afterwards
# (release-sums.yml).
#
#   scripts/release-body.sh TAG [REF] [REPO]
#
# REF defaults to TAG. release.yml passes HEAD, because on that run the tag
# is created with the release and does not exist yet.
# REPO (owner/name) defaults to $GITHUB_REPOSITORY.
set -euo pipefail
tag="${1:?usage: scripts/release-body.sh TAG [REF] [REPO]}"
ref="${2:-$tag}"
repo="${3:-${GITHUB_REPOSITORY:?set GITHUB_REPOSITORY or pass REPO}}"
here=$(dirname "$0")

prev=$(git describe --tags --abbrev=0 "$ref^" 2>/dev/null || true)
if [ -z "$prev" ]; then
  # The first release: the whole history is not a release note.
  printf '## %s\n\nThe first release of michi, a keyboard-driven Git client for repositories worked in by several Claude Code sessions at once (one window per repository, all worktrees in view).\n' "$tag"
else
  notes=$(bash "$here/release-notes.sh" "$prev..$ref")
  if [ -z "$notes" ]; then
    printf 'No commits between %s and %s.\n' "$prev" "$tag"
  else
    printf '%s\n' "$notes"
  fi
fi

cat <<'NOTE'

### Platforms

**Windows is the platform this is tested on.** The Linux build is tested in CI
only; **the macOS builds are only compiled and linked**. They are here so that
somebody can start them, not because they are known to work.

### Signing

**Nothing here is code-signed**, so Windows shows an unknown publisher and
SmartScreen warns about a file with no download history; macOS Gatekeeper
refuses the binaries until they are allowed through by hand. Every asset's
SHA-256 is in the table at the end of this page; a matching hash tells you the
file is the one CI built from this tag's commit.

The macOS and Linux downloads are `.tar.gz` because a release asset does not
carry the executable bit and an archive does. The Windows downloads are `.zip`:
`michi.exe` with `michi.com`, which makes `michi --version` and `--help` typed
in a terminal behave like any console command, and the licence files.
NOTE

printf '\n---\n\n%s\n' \
  "詳細は [CHANGELOG.md](https://github.com/$repo/blob/$tag/CHANGELOG.md) にあります（日本語）。"
