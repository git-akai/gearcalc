#!/usr/bin/env bash
# Print the directory cargo builds into for this workspace.
#
# Asked of cargo itself, so `CARGO_TARGET_DIR`, `build.target-dir` in a
# `.cargo/config.toml` and a workspace moved elsewhere are all honoured. Every
# script that runs what cargo built reads the path from here rather than
# assuming `<root>/target`. `jq` is not on every host, so python reads the JSON.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cargo metadata --format-version 1 --no-deps --manifest-path "$root/Cargo.toml" \
  | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])'
