"""Production Rust as the text checks read it: comments and literals blanked,
test code left out, each position placed in the function it is in.

Shared by `check_literals.py` and `check_absence.py`, so the two agree on
what production code is. It is read, not parsed: a line reader that stops at
a crate's first `#[cfg(test)]` misses the production code after a test module
(`train/mod.rs` has some), and one that does not blank strings and comments
finds `unwrap_or(0.0)` in the sentence that forbids it.
"""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def blank(text):
    """`(code, comments)`: `code` is `text` with every comment and every
    string, byte-string and character literal replaced by spaces (newlines
    kept, so offsets and line numbers are the source's); `comments` maps a
    line number (from 1) to the text of the comments on it."""
    out = list(text)
    comments = {}
    i, n, line = 0, len(text), 1

    def wipe(a, b):
        nonlocal line
        for k in range(a, b):
            if out[k] == "\n":
                line += 1
            else:
                out[k] = " "

    def note(a, b):
        for k, part in enumerate(text[a:b].split("\n")):
            comments[line + k] = comments.get(line + k, "") + part

    while i < n:
        c = text[i]
        if c == "\n":
            line += 1
            i += 1
        elif text.startswith("//", i):
            j = text.find("\n", i)
            j = n if j < 0 else j
            note(i, j)
            wipe(i, j)
            i = j
        elif text.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if text.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif text.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            note(i, j)
            wipe(i, j)
            i = j
        elif re.match(r'b?r#*"', text[i:i + 8]) and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_")):
            m = re.match(r'b?r(#*)"', text[i:])
            end = text.find('"' + m.group(1), i + m.end())
            j = n if end < 0 else end + 1 + len(m.group(1))
            wipe(i, j)
            i = j
        elif c == '"' or (c == "b" and text.startswith('b"', i) and (i == 0 or not text[i - 1].isalnum())):
            j = i + (2 if c == "b" else 1)
            while j < n and text[j] != '"':
                j += 2 if text[j] == "\\" else 1
            wipe(i, j + 1)
            i = j + 1
        elif c == "'":
            # A character literal, or a lifetime or label (`'a`, `'outer:`),
            # which has no closing quote after one character.
            m = re.match(r"'(\\(u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^\\'\n])'", text[i:])
            if m:
                wipe(i, i + m.end())
                i += m.end()
            else:
                i += 1
        else:
            i += 1
    return "".join(out), comments


def item_end(code, start):
    """Where the item starting at `start` ends: after its outermost block, or
    at a `;` outside any bracket, whichever comes first."""
    depth = 0
    for k in range(start, len(code)):
        ch = code[k]
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
            if depth == 0 and ch == "}":
                return k + 1
        elif ch == ";" and depth == 0:
            return k + 1
    return len(code)


CFG_TEST = re.compile(r"#\[cfg\(test\)\]")


def test_spans(code):
    """`(start, end)` of every item `#[cfg(test)]` puts under test, the
    attribute included; and the names of modules it declares test-only
    (`#[cfg(test)] mod name;`), whose files are test code whole."""
    spans, modules = [], []
    for m in CFG_TEST.finditer(code):
        start = m.end()
        # Past any further attributes to the item itself.
        rest = code[start:]
        k = 0
        while True:
            s = re.match(r"\s*#\[", rest[k:])
            if not s:
                break
            k = _attr_end(rest, k + s.end() - 1)
        end = item_end(code, start + k)
        decl = re.match(r"\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;", code[start + k:end])
        if decl:
            modules.append(decl.group(1))
        spans.append((m.start(), end))
    return spans, modules


def _attr_end(text, k):
    """The end of the attribute `#[...]` opening at `text[k]`."""
    depth = 0
    for j in range(k, len(text)):
        if text[j] == "[":
            depth += 1
        elif text[j] == "]":
            depth -= 1
            if depth == 0:
                return j + 1
    return len(text)


FN = re.compile(r"\bfn\s+(\w+)")


def functions(code):
    """`(start, end, name)` of every function body."""
    out = []
    for m in FN.finditer(code):
        # The signature ends at the first `{` or `;` outside its brackets: a
        # `;` inside `[f64; 2]` is not a declaration's end.
        depth = 0
        for k in range(m.end(), len(code)):
            ch = code[k]
            if ch in "([":
                depth += 1
            elif ch in ")]":
                depth -= 1
            elif depth == 0 and ch in "{;":
                break
        if code[k] == "{":
            out.append((m.start(), item_end(code, k), m.group(1)))
    return out


class Source:
    """One production file: its blanked code, comments, test spans and
    functions, and where each offset is."""

    def __init__(self, path, root=ROOT):
        self.path = path
        self.rel = path.relative_to(root).as_posix()
        self.text = path.read_text()
        self.code, self.comments = blank(self.text)
        self.tests, self.test_modules = test_spans(self.code)
        self.fns = functions(self.code)
        self.starts = [0] + [k + 1 for k, ch in enumerate(self.code) if ch == "\n"]

    def in_test(self, offset):
        return any(a <= offset < b for a, b in self.tests)

    def line(self, offset):
        lo, hi = 0, len(self.starts)
        while hi - lo > 1:
            mid = (lo + hi) // 2
            if self.starts[mid] <= offset:
                lo = mid
            else:
                hi = mid
        return lo + 1

    def function(self, offset):
        """The innermost function holding `offset`, else the item level."""
        best = None
        for a, b, name in self.fns:
            if a <= offset < b and (best is None or a > best[0]):
                best = (a, name)
        return best[1] if best else "-"


def production(crates=None, root=ROOT):
    """Every production source file of the workspace's crates (or of those
    named): `src/**/*.rs`, less the files a `#[cfg(test)] mod name;` makes
    test-only."""
    files = []
    for crate in sorted((root / "crates").iterdir()):
        if crates is not None and crate.name not in crates:
            continue
        files += sorted((crate / "src").rglob("*.rs"))
    sources = [Source(p, root) for p in files]
    test_only = set()
    for s in sources:
        for name in s.test_modules:
            here = s.path.parent if s.path.name in ("mod.rs", "lib.rs", "main.rs") else s.path.with_suffix("")
            test_only |= {here / f"{name}.rs", here / name / "mod.rs"}
    return [s for s in sources if s.path not in test_only]
