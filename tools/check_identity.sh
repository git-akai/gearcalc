#!/usr/bin/env bash
# Is the working tree bit-identical to <base-rev> in everything the core answers?
#
#     tools/check_identity.sh <base-rev> [lines]
#
# For a refactor that should move no number. Builds `gear-cli` at <base-rev> in
# a temporary git worktree and in the working tree, runs `gear-cli identity` in
# both (every preset and arrangement with all its load cases, and a gear and
# mesh grid, every float printed so equal text means equal bits), and compares.
# Prints the first differences (40 lines unless given) and how many lines
# differ; exits 0 only when the two are identical.
#
# The base must have the `identity` subcommand. Its builds are kept in
# `<target>/identity-base`, so a second run against a nearby base is
# incremental.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
base="${1:-}"
show="${2:-40}"
[[ -n "$base" ]] || { echo "usage: check_identity.sh <base-rev> [lines]" >&2; exit 2; }
rev="$(git -C "$root" rev-parse --verify "$base^{commit}")"

target="$("$root/tools/cargo_target_dir.sh")"
scratch="$(mktemp -d)"
tree="$scratch/base"
cleanup() {
  git -C "$root" worktree remove --force "$tree" >/dev/null 2>&1 || true
  rm -rf "$scratch"
}
trap cleanup EXIT

git -C "$root" worktree add --detach --quiet "$tree" "$rev"
CARGO_TARGET_DIR="$target/identity-base" \
  cargo build --release --quiet --manifest-path "$tree/Cargo.toml" --bin gear-cli
cargo build --release --quiet --manifest-path "$root/Cargo.toml" --bin gear-cli

run() { # binary, output
  # Captured whole before it is searched: `grep -q` in a pipe can close it
  # early and fail the producer under `pipefail`.
  local cases
  cases="$("$1" --golden-cases)"
  if [[ "$cases" != *$'\tidentity\t'* ]]; then
    echo "check_identity: $1 has no \`identity\` subcommand" >&2
    exit 2
  fi
  "$1" identity >"$2" 2>&1
}
run "$target/identity-base/release/gear-cli" "$scratch/base.txt"
run "$target/release/gear-cli" "$scratch/head.txt"

label="${base} (${rev:0:10})"
if cmp -s "$scratch/base.txt" "$scratch/head.txt"; then
  echo "identical to $label: $(wc -l <"$scratch/base.txt" | tr -d ' ') lines, $(grep -c '^== ' "$scratch/base.txt") cases"
  exit 0
fi

diff -U3 --label "$label" --label "working tree" "$scratch/base.txt" "$scratch/head.txt" \
  >"$scratch/diff" || true
head -n "$show" "$scratch/diff"
# Which cases moved: each file split at its `== ` headers, compared by name.
python3 - "$scratch/base.txt" "$scratch/head.txt" "$label" <<'PY' >&2
import sys
def cases(path):
    out, name = {}, "(before any case)"
    for line in open(path, errors="replace"):
        if line.startswith("== "):
            name = line[3:].rstrip("\n")
        out.setdefault(name, []).append(line)
    return out
b, h = cases(sys.argv[1]), cases(sys.argv[2])
names = list(dict.fromkeys([*b, *h]))
moved = [n for n in names if b.get(n) != h.get(n)]
lines = sum(sum(x != y for x, y in zip(b.get(n, []), h.get(n, [])))
            + abs(len(b.get(n, [])) - len(h.get(n, []))) for n in moved)
print(f"\nDIFFERENT from {sys.argv[3]}: {lines} lines differ in {len(moved)} of {len(names)} cases")
for n in moved[:10]:
    print(f"  {n}")
if len(moved) > 10:
    print(f"  ... and {len(moved) - 10} more")
PY
exit 1
