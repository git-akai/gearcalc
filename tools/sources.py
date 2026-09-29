"""The files the text checks read: every tracked file, less what is generated.

`check_doc_links.py` and `check_strings.py` both scan the tree; one list means
a directory added to the repository is scanned by both or by neither, never by
one of them because its glob happened to reach it.
"""

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Generated from the Rust by ts-rs and never edited by hand; its comments are
# copies of doc comments already scanned where they are written.
GENERATED = ("web/src/wire/",)


def tracked(suffixes=None):
    """Tracked text files under the root, sorted, optionally by suffix."""
    names = subprocess.run(
        ["git", "ls-files", "-z"], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout.split("\0")
    out = []
    for name in sorted(filter(None, names)):
        if name.startswith(GENERATED):
            continue
        path = ROOT / name
        if suffixes is not None and path.suffix not in suffixes:
            continue
        if not path.is_file():
            continue
        out.append(path)
    return out
