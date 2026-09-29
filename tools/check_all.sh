#!/usr/bin/env bash
# Every check CI runs, run here, with one summary line each.
#
#     tools/check_all.sh           # every step of the `tests` job
#     tools/check_all.sh --fast    # ...without the two nix builds, said so
#     tools/check_all.sh --list    # the plan, run nothing
#
# The steps are read from `.github/workflows/ci.yml` and run verbatim, so this
# cannot run a different command from CI. What is kept here is only how each
# step is treated, keyed by the step's `name:` (or its command where it has
# none):
#
#   CHEAP    run first, because they take seconds and fail on things a long
#            build would otherwise hide; the rest follow in CI's order.
#   NIX      the nix builds `--fast` skips. `nix flake check` is what runs
#            clippy, fmt and the suite, so `--fast` runs those three with
#            cargo in its place and says so.
#   EXCLUDED steps that publish rather than check, with the reason.
#
# Before running anything it checks that every CI step has exactly one
# treatment and every key here names a CI step, so a step added to, renamed in
# or removed from ci.yml fails this script until it is classified.
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ci="$root/.github/workflows/ci.yml"

CHEAP=(
  "check_all.sh runs what this job runs"
  "The CI policy fails on planted faults"
  "Documentation pointers resolve"
  "The string catalogue is complete"
  "Angular units are unambiguous"
  "The figure checker fails on planted faults"
  "Independent derivations agree with the recorded crate"
)
NIX=(
  "nix flake check -L"
  "nix build .#web -L"
)
EXCLUDED=(
  "Package the site|runs on main only; packages the site, checks nothing"
)
# Steps that use an action rather than run a command, with what stands in for
# each here. None of them checks anything; one that did would have to become a
# `run:` step to be run here, and an unlisted one fails the consistency law.
USES=(
  "actions/checkout|the working tree is the checkout"
  "DeterminateSystems/nix-installer-action|the local nix"
  "Upload the site|runs on main only; publishes the site, checks nothing"
)
# Jobs of ci.yml that are not the checks, with the reason. Every other job
# but `tests` fails the consistency law.
EXCLUDED_JOBS=(
  "deploy|publishes the site the tests job built; checks nothing"
)

mode=all
case "${1:-}" in
  --fast) mode=fast ;;
  --list) mode=list ;;
  "") ;;
  *) echo "usage: check_all.sh [--fast|--list]" >&2; exit 2 ;;
esac

# ci.yml and flake.nix are read strictly by `tools/ci_steps.py`, the one reader
# of what CI runs (check_doc_links.py holds CLAUDE.md's table to it too).
read_ci() {
  python3 "$root/tools/ci_steps.py" "$1" "$2" "${EXCLUDED_JOBS[@]%%|*}"
}

plan="$(mktemp -d)"
trap 'rm -rf "$plan"' EXIT
if ! read_ci "$plan/steps" "$plan/flake" 2>"$plan/errors"; then
  echo "FAIL  check_all.sh matches ci.yml and flake.nix" >&2
  sed 's/^/        /' "$plan/errors" >&2
  exit 1
fi
keys=() ifs=() cmds=() uses=()
while IFS= read -r -d '' kind && IFS= read -r -d '' k && IFS= read -r -d '' c && IFS= read -r -d '' r; do
  if [[ "$kind" == uses ]]; then
    uses+=("$k")
  else
    keys+=("$k"); ifs+=("$c"); cmds+=("$r")
  fi
done <"$plan/steps"
mapfile -t FLAKE_CHECK_STANDIN <"$plan/flake"

# --- the consistency law ------------------------------------------------------
in_list() { local x="$1"; shift; local e; for e in "$@"; do [[ "$e" == "$x" ]] && return 0; done; return 1; }
excluded_keys=()
for e in "${EXCLUDED[@]}"; do excluded_keys+=("${e%%|*}"); done
drift=()
(( ${#keys[@]} )) || drift+=("no run: step read from $ci")
for k in "${CHEAP[@]}" "${NIX[@]}" "${excluded_keys[@]}"; do
  in_list "$k" "${keys[@]}" || drift+=("classified here but not a run: step in ci.yml: $k")
done
uses_keys=()
for e in "${USES[@]}"; do uses_keys+=("${e%%|*}"); done
for k in "${uses_keys[@]}"; do
  in_list "$k" "${uses[@]}" || drift+=("classified here but not a uses: step in ci.yml: $k")
done
for k in "${uses[@]}"; do
  in_list "$k" "${uses_keys[@]}" || drift+=("uses: step in ci.yml with no stand-in here: $k")
done
for i in "${!keys[@]}"; do
  if [[ -n "${ifs[$i]}" ]] && ! in_list "${keys[$i]}" "${excluded_keys[@]}"; then
    drift+=("conditional in ci.yml (if: ${ifs[$i]}) and not classified: ${keys[$i]}")
  fi
  if in_list "${keys[$i]}" "${excluded_keys[@]}" && [[ -z "${ifs[$i]}" ]]; then
    drift+=("excluded here but CI runs it on every ref: ${keys[$i]}")
  fi
done
if (( ${#drift[@]} )); then
  echo "FAIL  check_all.sh matches ci.yml" >&2
  printf '        %s\n' "${drift[@]}" >&2
  exit 1
fi

# --- the plan -----------------------------------------------------------------
order=()
for i in "${!keys[@]}"; do in_list "${keys[$i]}" "${CHEAP[@]}" && order+=("$i"); done
for i in "${!keys[@]}"; do in_list "${keys[$i]}" "${CHEAP[@]}" || order+=("$i"); done

if [[ "$mode" == list ]]; then
  echo "ok    check_all.sh matches ci.yml (${#keys[@]} run: steps, ${#uses[@]} uses: steps) and flake.nix"
  for e in "${USES[@]}"; do echo "  uses      ${e%%|*}  (${e#*|})"; done
  for i in "${order[@]}"; do
    k="${keys[$i]}"
    if in_list "$k" "${excluded_keys[@]}"; then echo "  excluded  $k"
    elif in_list "$k" "${NIX[@]}"; then echo "  nix       $k"
      [[ "$k" == "nix flake check -L" ]] && printf '              --fast: %s\n' "${FLAKE_CHECK_STANDIN[@]}"
    else echo "  run       $k"; fi
  done
  exit 0
fi

logs="$(mktemp -d)"
summary=() failed=0 n=0
step() { # label, command
  local label="$1" cmd="$2" log start s
  n=$((n + 1))
  log="$logs/$n.log"
  start=$SECONDS
  echo "--- $label" >&2
  (cd "$root" && bash -euo pipefail -c "$cmd") >"$log" 2>&1
  s=$?
  if (( s == 0 )); then
    summary+=("$(printf 'PASS  %-62s %4ss' "$label" $((SECONDS - start)))")
  else
    failed=$((failed + 1))
    summary+=("$(printf 'FAIL  %-62s %4ss  exit %s, log %s' "$label" $((SECONDS - start)) "$s" "$log")")
    tail -n 30 "$log" | sed 's/^/      /' >&2
  fi
}

summary+=("$(printf 'PASS  %-62s' "check_all.sh matches ci.yml (${#keys[@]} run: steps)")")
for e in "${USES[@]}"; do
  summary+=("$(printf 'SKIP  %-62s %s' "uses: ${e%%|*}" "${e#*|}")")
done
for i in "${order[@]}"; do
  k="${keys[$i]}"
  if in_list "$k" "${excluded_keys[@]}"; then
    for e in "${EXCLUDED[@]}"; do [[ "${e%%|*}" == "$k" ]] && why="${e#*|}"; done
    summary+=("$(printf 'SKIP  %-62s %s' "$k" "$why")")
  elif [[ "$mode" == fast ]] && in_list "$k" "${NIX[@]}"; then
    summary+=("$(printf 'SKIP  %-62s --fast' "$k")")
    if [[ "$k" == "nix flake check -L" ]]; then
      # What flake.nix's `checks` build, each as the cargo command it runs.
      for c in "${FLAKE_CHECK_STANDIN[@]}"; do
        step "$c  (--fast, for nix flake check)" "nix develop --command $c"
      done
    fi
  else
    step "$k" "${cmds[$i]}"
  fi
done

echo
printf '%s\n' "${summary[@]}"
if [[ "$mode" == fast ]]; then
  echo "--fast: the nix builds did not run; this is NOT everything CI runs"
fi
if (( failed )); then
  echo "$failed check(s) failed; logs in $logs"
  exit 1
fi
rm -rf "$logs"
echo "every check passed"
