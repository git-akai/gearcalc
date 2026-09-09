#!/usr/bin/env bash
# Every number the harness can print, captured — so a refactor's effect on any
# reported figure is a **diff** rather than a claim.
#
# The test suite asserts properties: a law holds, an invariant is exact, a canary
# has not moved. What it cannot do is notice that some *other* number moved,
# because no test names it. `docs/corrections.md` has that fault twice over —
# three documented tables found stale in one session, and a figure attributed to
# the change in hand that had been wrong for months. Both were found by running
# the old code and looking at all of its output.
#
# So this runs every `gear-cli` subcommand and compares the whole of what it
# says against what it said before. It is a **change detector, not a gate on
# correctness**: a diff here is not a failure, it is a question — did you mean to
# move that? Answer it, then `--write`.
#
#     tools/check_golden.sh              # every command; exits non-zero on a diff
#     tools/check_golden.sh --fast       # ...skipping the five slow ones, loudly
#     tools/check_golden.sh --write      # ...or accept what the code prints now
#
# One script for both directions, as `check_bindings.sh` is, and for the same
# reason: a check whose fix is a different command is a check people skip.
#
# **The corpus is the harness's own table.** Each subcommand declares how its
# output is kept — in full at named invocations, as a digest, or somewhere else
# entirely — as a field on `Command` in `crates/gear-cli/src/main.rs`, and this
# script asks the binary (`--golden-cases`) rather than carrying a second copy.
# A list here would be a list to keep in step, which is the fault that made this
# whole audit phase necessary.
#
# **Determinism was measured, not assumed.** Every command below gives identical
# output on two consecutive runs, and identical output from a debug and a release
# build. If that ever stops being true the offending command does not belong
# here — a golden file that changes on its own trains you to ignore the diff.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
store="$root/tools/golden"

mode=check
fast=false
case "${1:-}" in
  --write) mode=write ;;
  --fast)  fast=true ;;
  "")      ;;
  *) echo "usage: check_golden.sh [--write|--fast]" >&2; exit 2 ;;
esac


# **Either profile will do, and that is a measurement rather than a hope**: every
# case here gives byte-identical output from a debug and a release build, which
# was checked when this was written. So a `--fast` run can use whatever the
# working tree already has and CI need not pay for a second build.
#
# The full run insists on release all the same, because two of the slow cases are
# minutes rather than seconds without it.
#
# **Built every time, never merely found.** This used to take whatever binary
# was already on disk and only build when there was none, which is how a corpus
# comes to be written from code that no longer exists — the fault
# `docs/corrections.md` records under a stale binary, met a second time in the
# instrument written to catch it. A command added to the table was invisible
# until something else forced a rebuild, and `--write` then *deleted* its golden
# file. Cargo is incremental, so an up-to-date tree pays nothing for this.
bin="$root/target/release/gear-cli"
profile=(--release)
if $fast && [[ ! -x "$bin" && -x "$root/target/debug/gear-cli" ]]; then
  bin="$root/target/debug/gear-cli"
  profile=()
fi
cargo build "${profile[@]}" --manifest-path "$root/Cargo.toml" --bin gear-cli >/dev/null

slug() { echo "$1" | tr ' /' '__'; }

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

# `speed \t keep \t invocation \t why`, from the harness itself.
cases=()
skipped=()
elsewhere=()
while IFS=$'\t' read -r speed keep case why; do
  if [[ "$keep" == none ]]; then
    elsewhere+=("$case — $why")
    continue
  fi
  if $fast && [[ "$speed" == slow ]]; then
    skipped+=("$case")
    continue
  fi
  cases+=("$keep|$case")
done < <("$bin" --golden-cases)

for entry in "${cases[@]}"; do
  keep="${entry%%|*}"
  case="${entry#*|}"
  out="$scratch/$(slug "$case").txt"
  # 2>&1 deliberately: a command that starts printing to stderr has changed what
  # it says, and that is exactly what this exists to notice.
  # shellcheck disable=SC2086
  "$bin" $case >"$out" 2>&1
  if [[ "$keep" == digest ]]; then
    # The shape as well as the hash, so a diff says *how* it moved rather than
    # only that it did — a bare hash tells you nothing you can act on.
    printf 'gear-cli %s\n  sha256 %s\n  %s lines, %s bytes\n' \
      "$case" \
      "$(sha256sum <"$out" | cut -d' ' -f1)" \
      "$(wc -l <"$out" | tr -d ' ')" \
      "$(wc -c <"$out" | tr -d ' ')" >"$out.digest"
    mv "$out.digest" "$out"
  fi
done

if [[ "$mode" == write ]]; then
  if $fast; then
    echo "refusing to --write a partial corpus; drop --fast" >&2
    exit 2
  fi
  mkdir -p "$store"
  # Removed rather than overwritten, so a command dropped from the table takes
  # its golden file with it instead of leaving one nothing regenerates.
  rm -f "$store"/*.txt
  cp "$scratch"/*.txt "$store/"
  echo "tools/golden: $(ls "$store"/*.txt | wc -l) files written"
  exit 0
fi

if [[ ! -d "$store" ]]; then
  echo "no golden corpus yet — create it with:" >&2
  echo "  tools/check_golden.sh --write" >&2
  exit 1
fi

status=0
if $fast; then
  # Compare only what was run. Named individually rather than by a directory
  # diff, so a skipped case cannot read as a passing one.
  for entry in "${cases[@]}"; do
    case="${entry#*|}"
    f="$(slug "$case").txt"
    if [[ ! -f "$store/$f" ]]; then
      echo "no golden for: gear-cli $case" >&2
      status=1
    elif ! diff -u "$store/$f" "$scratch/$f"; then
      status=1
    fi
  done
  echo
  echo "SKIPPED ${#skipped[@]} slow commands (${skipped[*]}) — this run is NOT the whole corpus" >&2
else
  diff -r -u "$store" "$scratch" || status=1
fi

if [[ $status -ne 0 ]]; then
  echo
  echo "the harness prints something different from the recorded corpus." >&2
  echo "that is a question, not a failure. if you meant it:" >&2
  echo "  tools/check_golden.sh --write" >&2
  exit 1
fi

echo "tools/golden: $(ls "$store"/*.txt | wc -l) recorded outputs, all unchanged"
# What is deliberately not here, said out loud. A coverage claim that omits its
# own exceptions is how a partial check comes to read as a complete one.
for e in "${elsewhere[@]}"; do
  echo "  not recorded here: $e"
done
