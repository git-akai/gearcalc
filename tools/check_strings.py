#!/usr/bin/env python3
"""Every `ui.*` message is used, and every `t("ui.…")` has a message.

The Rust side of the string catalogue checks itself: `gear_io::strings` holds
every key the core can emit against `Note::key::ALL`, in both directions. The
`[ui]` section has no `Note` behind it -- those are the application's own words
-- so nothing in Rust can see whether a label is still on screen. This does that
half, by reading the catalogue and the Svelte sources together.

Run by hand, like the other tools here; it needs no toolchain beyond Python.

    tools/check_strings.py            # exits non-zero and says what is wrong
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CATALOGUE = ROOT / "crates/gear-io/data/strings_en.toml"
SOURCES = sorted((ROOT / "web/src").glob("*.svelte")) + sorted(
    (ROOT / "web/src").glob("*.ts")
)


def catalogue_ui_keys(text):
    """The keys under [ui].

    A one-level TOML reader is enough here and keeps this script dependency
    free; `gear_io::strings` is the real parser and the one that decides whether
    the file is valid."""
    keys, section = set(), None
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("[") and line.endswith("]"):
            section = line[1:-1]
        elif section == "ui" and re.match(r"^[A-Za-z0-9_]+\s*=", line):
            keys.add("ui." + line.split("=", 1)[0].strip())
    return keys


def used_keys(paths):
    """Any `"ui.…"` literal, not only the ones inside a `t(` call.

    Keys travel as arguments too -- `autoNumber("ui.train_addendum", …)` renders
    through `t` one level down -- and scanning only for `t("…")` reported five
    live labels as orphans. A key is a key wherever it is written."""
    used = set()
    for p in paths:
        for m in re.finditer(r'"(ui\.[A-Za-z0-9_]+)"', p.read_text()):
            used.add(m.group(1))
    return used


def main():
    declared = catalogue_ui_keys(CATALOGUE.read_text())
    used = used_keys(SOURCES)

    missing = sorted(used - declared)
    orphans = sorted(declared - used)

    for k in missing:
        print(f"no message for {k} -- it will render as its own key")
    for k in orphans:
        print(f"{k} is in the catalogue and used nowhere")

    if missing or orphans:
        print(f"\n{len(declared)} ui messages, {len(used)} used")
        return 1
    print(f"{len(declared)} ui messages, all used, all present")
    return 0


if __name__ == "__main__":
    sys.exit(main())
