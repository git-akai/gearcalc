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
  "Documentation pointers resolve"
  "The string catalogue is complete"
  "Angular units are unambiguous"
)
NIX=(
  "nix flake check -L"
  "nix build .#web -L"
)
EXCLUDED=(
  "Package the site|runs on main only; packages the site, checks nothing"
)
# What `nix flake check` runs (flake.nix `checks`), for `--fast`.
FLAKE_CHECK_STANDIN=(
  "cargo clippy --all-targets -- --deny warnings"
  "cargo fmt --check"
  "cargo nextest run"
)

mode=all
case "${1:-}" in
  --fast) mode=fast ;;
  --list) mode=list ;;
  "") ;;
  *) echo "usage: check_all.sh [--fast|--list]" >&2; exit 2 ;;
esac

# The `tests` job's steps with a `run:`, as `key \0 if \0 command \0` records.
# ci.yml is plain enough that a line reader serves; python has no yaml module
# in the dev shell.
ci_steps() {
  python3 - "$ci" <<'PY'
import re, sys
lines = open(sys.argv[1]).read().split("\n")
steps, cur, in_tests, i = [], None, False, 0
while i < len(lines):
    line = lines[i]
    if re.match(r"^  \S", line):
        in_tests = line.strip() == "tests:"
    if in_tests:
        m = re.match(r"^      - (.*)$", line)
        if m:
            cur = {}
            steps.append(cur)
            line = "        " + m.group(1)
        m = re.match(r"^        (name|if|run): ?(.*)$", line)
        if cur is not None and m:
            key, val = m.groups()
            if key == "run" and val.strip() == "|":
                block = []
                i += 1
                while i < len(lines) and (lines[i].startswith("          ") or not lines[i].strip()):
                    block.append(lines[i][10:])
                    i += 1
                cur["run"] = "\n".join(block).strip() + "\n"
                continue
            cur[key] = val.strip()
    i += 1
for s in steps:
    if "run" in s:
        key = s.get("name", s["run"].strip())
        sys.stdout.write(f"{key}\0{s.get('if', '')}\0{s['run']}\0")
PY
}

keys=() ifs=() cmds=()
while IFS= read -r -d '' k && IFS= read -r -d '' c && IFS= read -r -d '' r; do
  keys+=("$k"); ifs+=("$c"); cmds+=("$r")
done < <(ci_steps)

# --- the consistency law ------------------------------------------------------
in_list() { local x="$1"; shift; local e; for e in "$@"; do [[ "$e" == "$x" ]] && return 0; done; return 1; }
excluded_keys=()
for e in "${EXCLUDED[@]}"; do excluded_keys+=("${e%%|*}"); done
drift=()
(( ${#keys[@]} )) || drift+=("no run: step read from $ci")
for k in "${CHEAP[@]}" "${NIX[@]}" "${excluded_keys[@]}"; do
  in_list "$k" "${keys[@]}" || drift+=("classified here but not a step in ci.yml: $k")
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
  echo "ok    check_all.sh matches ci.yml (${#keys[@]} run: steps)"
  for i in "${order[@]}"; do
    k="${keys[$i]}"
    if in_list "$k" "${excluded_keys[@]}"; then echo "  excluded  $k"
    elif in_list "$k" "${NIX[@]}"; then echo "  nix       $k"
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
for i in "${order[@]}"; do
  k="${keys[$i]}"
  if in_list "$k" "${excluded_keys[@]}"; then
    for e in "${EXCLUDED[@]}"; do [[ "${e%%|*}" == "$k" ]] && why="${e#*|}"; done
    summary+=("$(printf 'SKIP  %-62s %s' "$k" "$why")")
  elif [[ "$mode" == fast ]] && in_list "$k" "${NIX[@]}"; then
    summary+=("$(printf 'SKIP  %-62s --fast' "$k")")
    if [[ "$k" == "nix flake check -L" ]]; then
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
