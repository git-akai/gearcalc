#!/usr/bin/env bash
# **The browser payload, built. The only recipe.**
#
# Three things need this exact artifact and used to each carry their own copy of
# how to make it: `flake.nix` (what ships), `web/package.json` (what a developer
# runs) and `tools/check_wasm.sh` (what is checked). Three copies of a build is
# three answers to "which module did you measure?", and the check is worthless
# the moment it is not the shipped one.
#
#     tools/build_wasm.sh <out-dir> [--target web|nodejs]
#
# The pieces, and why each is here rather than in a profile or a flag:
#
#   * `--profile wasm` — `opt-level = "z"`. The measurement and the reasoning
#     live beside the profile in the workspace `Cargo.toml`; it is not repeated
#     here, so there is one place to change it.
#   * `wasm-opt -Oz` — a size pass over the *finished* module. It composes with
#     the profile rather than competing: "z" alone is 1,291,580 bytes and this
#     takes it to 1,209,487. `tools/check_wasm.sh` asserts it changes no answer.
#   * the two `--enable-` features — what rustc emits for this target and what
#     `wasm-opt` cannot infer, since nothing writes a `target_features` section.
#     Not tuning: without them the pass *fails*, which is the failure mode to
#     want. A toolchain bump that adds a third makes the build say so.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

out="${1:-}"
[[ -n "$out" ]] || { echo "usage: build_wasm.sh <out-dir> [--target web|nodejs]" >&2; exit 2; }
shift

target=web
case "${1:-}" in
  --target) target="${2:-}"; shift 2 ;;
  "")       ;;
  *) echo "usage: build_wasm.sh <out-dir> [--target web|nodejs]" >&2; exit 2 ;;
esac

for tool in wasm-bindgen wasm-opt; do
  command -v "$tool" >/dev/null || { echo "build_wasm: $tool not found — run inside \`nix develop\`" >&2; exit 2; }
done

# Skipped when the caller has already compiled — the Nix build does, because
# crane owns that half and the vendored registry is only reachable from there.
if [[ "${BUILD_WASM_SKIP_CARGO:-}" != 1 ]]; then
  cargo build --profile wasm --target wasm32-unknown-unknown \
    --manifest-path "$root/Cargo.toml" --package gear-wasm >&2
fi

module="${BUILD_WASM_MODULE:-$root/target/wasm32-unknown-unknown/wasm/gear_wasm.wasm}"

mkdir -p "$out"
wasm-bindgen --target "$target" --out-dir "$out" "$module"
wasm-opt -Oz --enable-bulk-memory-opt --enable-nontrapping-float-to-int \
  "$out/gear_wasm_bg.wasm" -o "$out/gear_wasm_bg.wasm"
