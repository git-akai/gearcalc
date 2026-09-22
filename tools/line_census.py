#!/usr/bin/env python3
"""**The prose-to-code ratio, counted the one way.**

`CLAUDE.md` opens with a ratio — production code against the comment it
carries, beside the standalone documents — and a figure quoted from a `wc`
over a glob is a figure like any other. This is the `wc`, written down, so
the number can be taken again at any tree and compared with the last one
rather than re-derived by hand each time (`docs/corrections.md`: *a scratch
verification is not a verification once the directory is gone*).

What it counts, and what each choice excludes:

- **`crates/**/*.rs`**, with the **test modules split off**: everything under
  a `tests/` directory, and every item a `#[cfg(test)]` attribute attaches to
  — a module, a function, an impl, a use — to its closing brace or its
  semicolon. A test is not production code and its comments are not the
  prose the ratio is about.
- **Blank lines dropped** on both sides, so indentation style moves nothing.
- A line is **comment** where its first non-space is `//` (any of `//`,
  `///`, `//!`) or it lies inside a `/* … */`; otherwise it is **code**. A
  trailing comment on a line of code counts as code: the line is one, and
  the generous reading would flatter the ratio.
- **`docs/*.md`**, excluding `docs/history/` — the superseded design record
  nothing points at, which counting once made the ratio 1.5 to 1 instead of
  1 to 1.

Run it bare for this tree; give it revisions to compare:

    python3 tools/line_census.py                 # the working tree
    python3 tools/line_census.py HEAD~5 HEAD     # two trees, and the change
"""

from __future__ import annotations

import re
import subprocess
import sys
from dataclasses import dataclass
from fnmatch import fnmatch
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


@dataclass
class Count:
    code: int = 0
    comment: int = 0
    test: int = 0

    def __add__(self, other: "Count") -> "Count":
        return Count(
            self.code + other.code,
            self.comment + other.comment,
            self.test + other.test,
        )


def test_spans(src: str) -> list[tuple[int, int]]:
    """Line ranges (0-based, half-open) the `#[cfg(test)]` items cover."""
    spans: list[tuple[int, int]] = []
    for m in re.finditer(r"#\[cfg\(test\)\]", src):
        rest = src[m.end() :]
        brace = rest.find("{")
        semi = rest.find(";")
        if semi != -1 and (brace == -1 or semi < brace):
            end = m.end() + semi + 1
        elif brace == -1:
            continue
        else:
            depth, i = 0, m.end() + brace
            end = len(src)
            while i < len(src):
                if src[i] == "{":
                    depth += 1
                elif src[i] == "}":
                    depth -= 1
                    if depth == 0:
                        end = i + 1
                        break
                i += 1
        spans.append((src.count("\n", 0, m.start()), src.count("\n", 0, end) + 1))
    return spans


def census(src: str, all_test: bool) -> Count:
    lines = src.split("\n")
    is_test = [all_test] * len(lines)
    if not all_test:
        for start, end in test_spans(src):
            for i in range(start, min(end, len(lines))):
                is_test[i] = True
    out = Count()
    in_block = False
    for line, test in zip(lines, is_test):
        text = line.strip()
        was_block = in_block
        if in_block:
            if "*/" in text:
                in_block = False
        elif text.startswith("/*"):
            in_block = "*/" not in text
        if not text:
            continue
        if test:
            out.test += 1
        elif was_block or in_block or text.startswith("//"):
            out.comment += 1
        else:
            out.code += 1
    return out


def read(rev: str | None, path: str) -> str:
    if rev is None:
        return (ROOT / path).read_text()
    return subprocess.run(
        ["git", "show", f"{rev}:{path}"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout


def files(rev: str | None, glob: str) -> list[str]:
    """Every path matching `glob`, in this tree or in a revision's."""
    if rev is None:
        return sorted(str(p.relative_to(ROOT)) for p in ROOT.glob(glob))
    out = subprocess.run(
        ["git", "ls-tree", "-r", "--name-only", rev],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.split()
    return sorted(f for f in out if fnmatch(f, glob) or fnmatch(f, glob.replace("**/", "")))


def at(rev: str | None) -> tuple[Count, int]:
    total = Count()
    for f in files(rev, "crates/**/*.rs"):
        total = total + census(read(rev, f), all_test="/tests/" in f)
    docs = 0
    for f in files(rev, "docs/*.md"):
        if "/history/" in f:
            continue
        docs += sum(1 for line in read(rev, f).split("\n") if line.strip())
    return total, docs


def show(name: str, c: Count, docs: int) -> None:
    print(f"{name}:")
    print(f"  production code {c.code:>7,}")
    print(f"  comment         {c.comment:>7,}   ({c.comment / max(c.code, 1):.2f} per line of code)")
    print(f"  documents       {docs:>7,}")
    print(f"  prose to code   {(c.comment + docs) / max(c.code, 1):>7.2f} to 1")
    print(f"  (test code, not counted above: {c.test:,})")


def main() -> None:
    revs = sys.argv[1:]
    if not revs:
        c, docs = at(None)
        show("the working tree", c, docs)
        return
    seen = [(rev, *at(rev)) for rev in revs]
    for rev, c, docs in seen:
        show(rev, c, docs)
    if len(seen) == 2:
        (_, a, ad), (_, b, bd) = seen
        print("change:")
        print(f"  production code {b.code - a.code:>+7,}")
        print(f"  comment         {b.comment - a.comment:>+7,}")
        print(f"  documents       {bd - ad:>+7,}")
        print(f"  test code       {b.test - a.test:>+7,}")


if __name__ == "__main__":
    main()
