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

# ci.yml and flake.nix read strictly. Anything whose meaning a plain line
# reader could get wrong is refused rather than guessed: a step key outside
# name/id/if/run/uses/with (`shell:`, `env:`, `working-directory:`,
# `continue-on-error:` all change what a `run:` does), a block scalar other
# than `|`, a quoted `run:`, a workflow or job key that changes how steps run
# (`env:`, `defaults:`, `container:`), and a job that is neither `tests` nor
# excluded above. python has no yaml module in the dev shell.
#
# Writes `key \0 if \0 command \0` per `run:` step of `tests` to $1, and to
# $2 the command `--fast` runs for each of flake.nix's `checks`, one a line.
read_ci() {
  python3 - "$ci" "$root/flake.nix" "$1" "$2" "${EXCLUDED_JOBS[@]%%|*}" <<'PY'
import re, sys

ci, flake, steps_out, flake_out, *excluded_jobs = sys.argv[1:]
errors = []
TOP = {"name", "on", "concurrency", "jobs", "permissions"}
JOB = {"runs-on", "permissions", "steps"}
STEP = {"name", "id", "if", "run", "uses", "with"}

lines = open(ci).read().split("\n")
indent = lambda l: len(l) - len(l.lstrip(" "))
steps, jobs, job, cur, where, i = [], [], None, None, None, 0
while i < len(lines):
    line, n = lines[i], i + 1
    i += 1
    text = line.strip()
    if not text or text.startswith("#"):
        continue
    d = indent(line)
    key = re.match(r"^(?:-\s+)?(?P<k>[A-Za-z_-]+):(?:\s|$)", text)
    if d == 0:
        where = key and key["k"]
        if where not in TOP:
            errors.append(f"ci.yml:{n}: workflow key {text!r} is not one this reads")
        continue
    if where != "jobs":
        continue
    if d == 2:
        job = key and key["k"]
        jobs.append(job)
        if job != "tests" and job not in excluded_jobs:
            errors.append(f"ci.yml:{n}: job {job!r} is neither `tests` nor excluded in check_all.sh")
        continue
    if job != "tests":
        continue
    if d == 4:
        section = key and key["k"]
        if section not in JOB:
            errors.append(f"ci.yml:{n}: tests job key {text!r} is not one this reads")
        continue
    if section != "steps":
        continue
    if d == 6 and text.startswith("- "):
        cur = {}
        steps.append(cur)
        d, text = 8, text[2:].strip()
        key = re.match(r"^(?P<k>[A-Za-z_-]+):(?:\s|$)", text)
    if d == 8:
        k = key and key["k"]
        if cur is None or k not in STEP:
            errors.append(f"ci.yml:{n}: step key {text!r} is not one of {sorted(STEP)}")
            continue
        val = text.split(":", 1)[1].strip()
        if k == "run":
            if val.startswith(("|", ">")) and val != "|":
                errors.append(f"ci.yml:{n}: block scalar {val!r}; only `run: |` is read")
                continue
            if val.startswith(("'", '"')):
                errors.append(f"ci.yml:{n}: quoted run: is not read; write it plain or as `|`")
                continue
            if val == "|":
                block = []
                while i < len(lines) and (indent(lines[i]) >= 10 or not lines[i].strip()):
                    block.append(lines[i][10:])
                    i += 1
                val = "\n".join(block).strip() + "\n"
        cur[k] = val
        continue
    if d >= 10 and cur is not None and "with" in cur:
        continue
    errors.append(f"ci.yml:{n}: line not read: {text!r}")

for e in excluded_jobs:
    if e not in jobs:
        errors.append(f"excluded job {e!r} is not in ci.yml")
if "tests" not in jobs:
    errors.append("ci.yml has no tests job")

# flake.nix `checks`: each entry mapped to the cargo command it runs.
src = open(flake).read()
m = re.search(r"\n      checks = eachSystem.*?\n        \{\n(.*?)\n        \}\);", src, re.S)
standins = []
if not m:
    errors.append("flake.nix: no `checks = eachSystem` block")
else:
    entries = re.split(r"\n(?=          [a-z]\w* = )", "\n" + m.group(1))
    for entry in filter(str.strip, entries):
        name, body = entry.strip().split(" = ", 1)
        inner = set(re.findall(r"^\s+(\w+)\s*=", body, re.M))
        inner |= {"inherit " + w for w in re.findall(r"inherit (\w+);", body)}
        known = {
            "self.packages.${system}.default;": ("cargo build --release --workspace", set()),
            "craneLib.cargoFmt": ("cargo fmt --check", set()),
            "craneLib.cargoClippy": ("cargo clippy {cargoClippyExtraArgs}",
                                     {"inherit cargoArtifacts", "cargoClippyExtraArgs"}),
            "craneLib.cargoNextest": ("cargo nextest run",
                                      {"inherit cargoArtifacts", "partitions", "partitionType"}),
        }
        head = next((k for k in known if body.startswith(k)), None)
        if head is None:
            errors.append(f"flake.nix check {name!r}: {body.splitlines()[0]!r} has no cargo stand-in here")
            continue
        cmd, allowed = known[head]
        extra = inner - allowed
        if extra:
            errors.append(f"flake.nix check {name!r}: arguments {sorted(extra)} have no stand-in here")
            continue
        args = dict(re.findall(r'(\w+) = "([^"]*)";', body))
        standins.append(cmd.format(**args))

if errors:
    print("\n".join(errors), file=sys.stderr)
    sys.exit(1)
with open(steps_out, "w") as f:
    for s in steps:
        if "run" in s:
            f.write(f"{s.get('name', s['run'].strip())}\0{s.get('if', '')}\0{s['run']}\0")
with open(flake_out, "w") as f:
    f.write("".join(c + "\n" for c in standins))
PY
}

plan="$(mktemp -d)"
trap 'rm -rf "$plan"' EXIT
if ! read_ci "$plan/steps" "$plan/flake" 2>"$plan/errors"; then
  echo "FAIL  check_all.sh matches ci.yml and flake.nix" >&2
  sed 's/^/        /' "$plan/errors" >&2
  exit 1
fi
keys=() ifs=() cmds=()
while IFS= read -r -d '' k && IFS= read -r -d '' c && IFS= read -r -d '' r; do
  keys+=("$k"); ifs+=("$c"); cmds+=("$r")
done <"$plan/steps"
mapfile -t FLAKE_CHECK_STANDIN <"$plan/flake"

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
  echo "ok    check_all.sh matches ci.yml (${#keys[@]} run: steps) and flake.nix"
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
