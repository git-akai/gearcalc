#!/usr/bin/env bash
# **The payload, executed.** Every other check looks at the boundary without
# running it: `check_bindings.sh` compares its generated *shape* against the
# Rust, `cargo nextest run` exercises `gear-core` natively, and the golden corpus
# records what `gear-cli` prints. The `.wasm` file the browser downloads was
# never run by anything.
#
# That gap has a cost the moment a build step rewrites the module. `wasm-opt`
# does exactly that (see `flake.nix`), and so would a toolchain bump or a linker
# flag. Without this, a payload that computes differently — or does not start —
# ships with every check green.
#
# Two claims, and they are different in kind:
#
#   1. **A law.** Optimising the module changes no answer. Asserted
#      differentially, by running the same probe against the module before and
#      after `wasm-opt` and requiring the two outputs to be identical. It needs
#      no recorded file and cannot go stale.
#   2. **A change detector.** What the boundary answers is recorded, in the
#      corpus idiom — a diff is a question, not a failure.
#
# And a coverage claim that holds itself up: the probe reports which entry
# points it called, and this fails if the crate exports one the probe does not.
# A new entry point cannot arrive untested, the same way a new `gear-cli`
# subcommand cannot arrive unrecorded.
#
#     tools/check_wasm.sh          # both claims; exits non-zero on either
#     tools/check_wasm.sh --write  # ...or accept what the boundary answers now
#
# Requires the dev shell: `wasm-bindgen`, `wasm-opt` and `node`.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# **Not in `tools/golden/`**, though it is the same idiom. That directory is
# `check_golden.sh`'s corpus, asked from the CLI binary and rewritten wholesale
# — its `--write` does `rm -f golden/*.txt` first, so a record kept there that
# the CLI does not produce is one a routine `--write` silently deletes. Found by
# putting it there: `check_golden.sh` reported the extra file as a diff.
record="$root/tools/wasm_boundary.json"

mode=check
case "${1:-}" in
  --write) mode=write ;;
  "")      ;;
  *) echo "usage: check_wasm.sh [--write]" >&2; exit 2 ;;
esac

for tool in wasm-bindgen wasm-opt node; do
  command -v "$tool" >/dev/null || { echo "check_wasm: $tool not found — run inside \`nix develop\`" >&2; exit 2; }
done

scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT

# **The shipped recipe, not a re-creation of it.** `tools/build_wasm.sh` is what
# `flake.nix` and `web/package.json` run, so the module measured here is the
# module people download. A check that assembled its own build would answer a
# question nobody asked.
#
# `--target nodejs` rather than `web` is the one difference, and it is in the
# *wrapper*: the `.wasm` beside it is byte-identical, and that is the artifact
# under test.
#
# Built every time, never merely found — the stale-binary fault
# `check_golden.sh` and `check_figures.py` have each been bitten by.
"$root/tools/build_wasm.sh" "$scratch" --target nodejs >/dev/null

cp "$scratch/gear_wasm_bg.wasm" "$scratch/optimised.wasm"
node "$root/tools/wasm_probe.mjs" "$scratch/gear_wasm.js" > "$scratch/after.json"

# The unoptimised half of the differential: the same build with the size pass
# left off. `build_wasm.sh` always applies it — it is the shipped artifact's
# recipe — so the module before it is recovered by running `wasm-bindgen`
# alone, which is what that script does first.
wasm-bindgen --target nodejs --out-dir "$scratch" \
  "$root/target/wasm32-unknown-unknown/wasm/gear_wasm.wasm"
cp "$scratch/gear_wasm_bg.wasm" "$scratch/unoptimised.wasm"
node "$root/tools/wasm_probe.mjs" "$scratch/gear_wasm.js" > "$scratch/before.json"

cp "$scratch/optimised.wasm" "$scratch/gear_wasm_bg.wasm"

status=0

# --- claim 1: the law -------------------------------------------------------
if diff -q "$scratch/before.json" "$scratch/after.json" >/dev/null; then
  echo "wasm: optimising the payload changes no answer, across $(node -e '
    console.log(JSON.parse(require("fs").readFileSync(process.argv[1],"utf8")).entries.length)
  ' "$scratch/after.json") entry points"
else
  echo "wasm: **the optimised payload answers differently** — the pass is not safe here" >&2
  diff "$scratch/before.json" "$scratch/after.json" | head -40 >&2
  status=1
fi

printf '  %s -> %s bytes (-%s %%)\n' \
  "$(wc -c <"$scratch/unoptimised.wasm" | tr -d ' ')" \
  "$(wc -c <"$scratch/optimised.wasm" | tr -d ' ')" \
  "$(node -e 'const a=+process.argv[1],b=+process.argv[2];console.log((100*(a-b)/a).toFixed(1))' \
      "$(wc -c <"$scratch/unoptimised.wasm")" "$(wc -c <"$scratch/optimised.wasm")")"

# --- the coverage claim -----------------------------------------------------
# The crate's exports, from the crate. `#[wasm_bindgen]` on a `pub fn` is the
# declaration, so this reads the source rather than a second list.
exported="$(grep -A2 '^#\[wasm_bindgen\]$' "$root/crates/gear-wasm/src/lib.rs" \
  | sed -n 's/^pub fn \([a-z_0-9]*\).*/\1/p' | sort -u)"
called="$(node -e '
  console.log(JSON.parse(require("fs").readFileSync(process.argv[1],"utf8")).entries.join("\n"))
' "$scratch/after.json" | sort -u)"
missing="$(comm -23 <(echo "$exported") <(echo "$called") || true)"
if [[ -n "$missing" ]]; then
  echo "wasm: entry points the probe never calls — add them to tools/wasm_probe.mjs:" >&2
  echo "$missing" | sed 's/^/  /' >&2
  status=1
fi

# --- claim 2: the change detector -------------------------------------------
if [[ "$mode" == write ]]; then
  mkdir -p "$(dirname "$record")"
  cp "$scratch/after.json" "$record"
  echo "  recorded $(wc -c <"$record" | tr -d ' ') bytes to tools/wasm_boundary.json"
  exit "$status"
fi

if [[ ! -f "$record" ]]; then
  echo "no boundary record yet — create it with:" >&2
  echo "  tools/check_wasm.sh --write" >&2
  exit 1
fi

if diff -q "$record" "$scratch/after.json" >/dev/null; then
  echo "  the boundary answers what it answered before"
else
  echo "wasm: the boundary's answers moved — a diff here is a question, not a failure" >&2
  diff "$record" "$scratch/after.json" | head -60 >&2
  status=1
fi

exit "$status"
