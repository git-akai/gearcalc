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

# ---------------------------------------------------------------- the corpus
#
# One invocation per subcommand, plus a second where the first would leave a
# whole regime uncovered — `show 9 -0.3` is an undercut tooth, which `show 17
# 0.2` is not.
#
# **This list is expected to match the harness's own dispatch**, and today it is
# kept in step by hand, which is the fault `docs/state.md` records against the
# harness's module comment (audit F8). Phase 1 derives both from one table in
# `main.rs`; until then, a command added without a line here is invisible.
FAST_CASES=(
  "show 17 0.2"
  "show 9 -0.3"
  "sweep"
  "materials"
  "strength 17 43 2.0"
  "train"
  "train mixed"
  "trainfile"
  "worm 1 40 7 90"
  "wormstage 1 40 7 2"
  "crossed 17 23 90"
  "planetary 17 17 3"
  "planetstage 24 18 60 3"
  "hula 18 0.2"
  "loadcase"
  "dump"
  "dxf 17 0.2 0.001"
)

# Recorded as a digest rather than in full.
#
# `dump` is a raw export of every sampled profile point — 10.9 MB, and every
# figure in it is one some other command derives from and prints in ten. A
# checked-in file that size is one nobody reads a diff of, which makes it a
# change detector that detects nothing. The digest moves if any point does, and
# that is the whole job.
DIGEST_CASES=("dump")

# Deliberately **not** here: `bending`.
#
# Its output is the body of `docs/bending-check.html`, verbatim — so a golden
# copy would be a third copy of it, and the useful check is against the document
# rather than against a recording. `tools/check_figures.py` owns it (audit F10).

# Together about a minute. Kept in the default run because the audit's whole
# premise is that a partial check reading as a complete one is the worse failure.
SLOW_CASES=(
  "matrix"
  "verify 100"
  "hulaband 18"
  "meshsweep 60 20 0.8"
  "hulasweep 18 0.25"
)

cases=("${FAST_CASES[@]}")
$fast || cases+=("${SLOW_CASES[@]}")

# **Either profile will do, and that is a measurement rather than a hope**: every
# case here gives byte-identical output from a debug and a release build, which
# was checked when this was written. So a `--fast` run can use whatever the
# working tree already has and CI need not pay for a second build.
#
# The full run insists on release all the same, because two of the slow cases are
# minutes rather than seconds without it.
bin="$root/target/release/gear-cli"
if $fast && [[ ! -x "$bin" && -x "$root/target/debug/gear-cli" ]]; then
  bin="$root/target/debug/gear-cli"
fi
if [[ ! -x "$bin" ]]; then
  cargo build --release --manifest-path "$root/Cargo.toml" --bin gear-cli >/dev/null
  bin="$root/target/release/gear-cli"
fi

slug() { echo "$1" | tr ' /' '__'; }

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

is_digest() {
  local c
  for c in "${DIGEST_CASES[@]}"; do [[ "$c" == "$1" ]] && return 0; done
  return 1
}

for case in "${cases[@]}"; do
  out="$scratch/$(slug "$case").txt"
  # 2>&1 deliberately: a command that starts printing to stderr has changed what
  # it says, and that is exactly what this exists to notice.
  # shellcheck disable=SC2086
  "$bin" $case >"$out" 2>&1
  if is_digest "$case"; then
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
  # Removed rather than overwritten, so a command dropped from the list takes its
  # golden file with it instead of leaving one nothing regenerates.
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
  for case in "${cases[@]}"; do
    f="$(slug "$case").txt"
    if [[ ! -f "$store/$f" ]]; then
      echo "no golden for: gear-cli $case" >&2
      status=1
    elif ! diff -u "$store/$f" "$scratch/$f"; then
      status=1
    fi
  done
  echo
  echo "SKIPPED ${#SLOW_CASES[@]} slow commands (${SLOW_CASES[*]}) — this run is NOT the whole corpus" >&2
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
