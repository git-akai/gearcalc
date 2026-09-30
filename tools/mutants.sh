#!/usr/bin/env bash
# A sample of mutants (cargo-mutants, in the dev shell), with settings this
# machine holds: what tests fail when one operator, constant or return value
# in the code is changed. A mutant no test notices is code the suite does not
# pin.
#
#     tools/mutants.sh [--sample N] FILE...           # mutants of these files
#     tools/mutants.sh [--sample N] --in-diff REV     # ...of the lines changed since REV
#     tools/mutants.sh ... -- ARG...                  # more cargo-mutants arguments
#
# e.g. a package's own lines, one mutant in ten:
#
#     tools/mutants.sh --sample 10 --in-diff audit-ablation
#
# `--sample N` runs every Nth mutant (round-robin), not the first 1/N, so a
# sample spreads over every file and function in scope. `-- --list` counts
# them without running any.
#
# Settings, each overridable from the environment:
#   MUTANTS_JOBS=1           mutants built and tested at once. Each is a whole
#                            copy of the tree building the workspace, and two
#                            of them beside an editor exhaust this machine.
#   MUTANTS_TEST_THREADS=1   tests at once within one mutant's run. One, and not
#                            the build slot's inherited NEXTEST_TEST_THREADS: a
#                            mutant that breaks a loop's exit allocates without
#                            bound, and three such tests at once starve this
#                            machine. The watchdog's kill reads as caught.
#   CARGO_BUILD_JOBS=4       rustc processes within one build.
#   MUTANTS_OUT=target       where `mutants.out/` (the caught, missed, unviable
#                            and timeout lists) is written.
# Tests are nextest's, and only those of the package a mutant is in (as the
# audit's baseline was taken); a mutant caught only by another crate's tests
# reads missed, so check a survivor with `-- --test-workspace true`.
#
# gear-cli has no tests of its own: the corpus (`tools/check_golden.sh`) and
# the gates reading it hold it, which cargo-mutants cannot run. So --in-diff
# leaves it out, and says so; name its files to mutate them anyway.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

usage() { sed -n '2,/^set -euo/p' "$0" | sed '$d; s/^# \{0,1\}//'; }

sample=1
rev=""
files=()
while (( $# )); do
  case "$1" in
    --sample)
      [[ "${2:-}" =~ ^[1-9][0-9]*$ ]] || { echo "mutants.sh: --sample wants a count" >&2; exit 2; }
      sample="$2"; shift 2 ;;
    --in-diff)
      [[ -n "${2:-}" && "$2" != -* ]] || { echo "mutants.sh: --in-diff wants a revision" >&2; exit 2; }
      rev="$2"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    --) shift; break ;;
    -*) echo "mutants.sh: unknown option $1 (cargo-mutants' go after --)" >&2; exit 2 ;;
    *) files+=("$1"); shift ;;
  esac
done
named=$(( ${#files[@]} > 0 ))
diffed=$(( ${#rev} > 0 ))
if (( named == diffed )); then
  echo "mutants.sh: name files or --in-diff REV, one of the two" >&2
  exit 2
fi
command -v cargo-mutants >/dev/null || {
  echo "mutants.sh: cargo-mutants is not on the path; run inside \`nix develop\`" >&2
  exit 2
}

export NEXTEST_TEST_THREADS="${MUTANTS_TEST_THREADS:-1}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-4}"
args=(
  mutants
  --jobs "${MUTANTS_JOBS:-1}"
  --test-tool nextest
  --test-workspace false
  --cap-lints true
  --output "${MUTANTS_OUT:-$root/target}"
)
if (( sample > 1 )); then
  args+=(--sharding round-robin --shard "0/$sample")
fi

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
if [[ -n "$rev" ]]; then
  git -C "$root" diff "$rev" -- '*.rs' >"$scratch/diff"
  if [[ ! -s "$scratch/diff" ]]; then
    echo "mutants.sh: no Rust changed since $rev; nothing to mutate"
    exit 0
  fi
  args+=(--in-diff "$scratch/diff" --exclude 'crates/gear-cli/**')
  echo "mutants.sh: the Rust changed since $rev, less gear-cli (no tests of its own; see the header)" >&2
else
  for f in "${files[@]}"; do args+=(--file "$f"); done
fi

cd "$root"
cargo "${args[@]}" "$@"
