#!/usr/bin/env python3
"""Every figure the documents print is a figure the code still prints.

# Why this exists

`docs/corrections.md` records this fault three times, which makes it the most
recurrent one in the project:

  - a documented table whose every row had moved, found by regenerating it;
  - two more of the same section's tables, found by regenerating all of them;
  - a figure diagnosed as a regression from the change in hand, which turned out
    to have been stale for months.

The fix each time was to regenerate by hand and remember to do it again. That is
not a mechanism, and the second and third entries are what it costs. A table's
*provenance* -- which command produces it -- was carried in nobody's head
reliably and in no file at all.

# How a figure is tagged

An HTML comment above the block, naming the command that produces it:

    <!-- figures: gear-cli strength 17 43 2.0 -->
    | | |
    |---|---|
    | `sigma_F` | 66.8 / 56.0 MPa |

A script in `tools/` that holds the harness against a derivation of its own
can be named instead of `gear-cli`, and is handed the same binary through
`GEAR_CLI`:

    <!-- figures: tools/iso_6336_3_stack.py -->

The block runs from the marker to the next blank line -- a Markdown table, a
list, a paragraph -- or to the closing fence of a code block. Numbers match
**at the document's own precision**: a document saying 98.74 is satisfied by an
output of 98.7412, and one saying 98.741 is not.

**A table row is matched as a row.** Its numbers, in order, must be an ordered
subsequence of **one line** a block's command prints, so two rows' figures
swapped, two figures within a row swapped, or a figure taken from another line,
fail. A row's labels are not figures: a command it quotes
(`` `gear-cli strength 17 43 2.0` ``) and a code span holding a letter
(`` `N+1/N/N−1/N` ``). Prose and lists are matched as a bag against everything
the block's commands print, since a sentence orders its figures for the reader.

The second verb is for a document that *is* generated output:

    <!-- figures-verbatim: gear-cli bending -->

which requires the command's entire output to appear in the file, byte for byte.
`docs/bending-check.html` is that case -- generated figures with prose written
around them.

The third says a block is deliberately not gated, and why:

    <!-- figures-exempt: a record of what was once wrong; these must not move -->

**This one is load-bearing.** The four documents do not stand in the same
relation to the code's output. `reference.md` and `state.md` describe what the
tool computes, so their figures should track it. `corrections.md` records what
the tool *used* to compute -- regenerating those would erase the history the
document exists to keep. `rationale.md` carries measurements that settled a
decision, and quotations from ISO that were never ours. A checker that reported
all of those as "ungated" every run would be a checker people learn to skip,
which is the failure mode `.github/workflows/ci.yml` already names about
annotations. So an exemption is a *statement*, carrying its reason, and
`--list`'s remaining "NOTHING" entries are the real backlog.

`figures-bold` is the same as `figures:` but reads **only the bolded figures**.
It exists for one real shape: a table with a `before` column and an `after` one,
where only the second is a claim about the code and the document already sets it
in bold. `state.md`'s study 5 is that table, and it is the one whose drift
prompted this whole file -- so exempting it, which was the first thing tried,
would have been building the gate around the fault it was meant to catch.

The fifth names a test that already gates the block:

    <!-- figures-by-test: the_documented_tables_are_the_ones_this_code_prints -->

which is not an exemption -- it is a *different gate*, and saying so is the
information a reader wants. The name must be a `#[test]` fn. A table row must
read, in order, as one of the test's literal tuples -- a parenthesised group of
numeric literals, the row the test holds -- at the document's precision; a line
of prose must find each of its numbers among the body's literals (comments and
strings aside). A figure the test does not hold as that row is one nothing
gates.

# What this is and is not

**A canary, not an invariant.** The digits are free to move; what they are not
free to do is move quietly. A failure here is a reminder to regenerate, not
evidence of a defect -- exactly the reading `train::hula`'s
`the_documented_tables_are_the_ones_this_code_prints` states for the two tables
it covers.

It is also deliberately *one-directional* on strength. Matching at the
document's precision means a low-precision figure is weakly pinned: `1.2` will
find something. The report separates the two so the coverage claim is honest,
and `--list` names every table carrying figures that nothing generates -- which
is the measurement of how much of the documents this reaches.

    tools/check_figures.py              # exits non-zero and names what drifted
    tools/check_figures.py --list       # what is tagged, what is not, and by what
    tools/check_figures.py --self-test  # the rules, on a fixture and planted faults

Dependency-free, like its siblings here.
"""

import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def _bin():
    """The harness, **built** -- release if that is what is there, else debug.

    The two print the same bytes, measured rather than assumed, and
    `tools/check_golden.sh` says so at more length. It matters because a working
    tree may hold only a debug build, and a documented figure is not worth a second toolchain pass to check.

    It is *built* rather than merely found, which it was not: a command added to
    the harness was invisible here until something else forced a rebuild, so a
    figure tagged with it read as drifted when the document was right and the
    binary was old. That is the stale-binary fault `docs/corrections.md` records,
    met in the second of the two instruments written to catch drift -- the first
    was `check_golden.sh`, and the fix is the same one. Cargo is incremental, so
    an up-to-date tree pays nothing.

    `GEAR_CLI=<path>` runs that binary instead and builds nothing. Otherwise
    the directory is cargo's own (`tools/cargo_target_dir.sh`), so
    `CARGO_TARGET_DIR` is honoured.
    """
    if os.environ.get("GEAR_CLI"):
        return Path(os.environ["GEAR_CLI"])
    target = Path(
        subprocess.run(
            [ROOT / "tools" / "cargo_target_dir.sh"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    )
    profile = "debug" if (target / "debug" / "gear-cli").exists() and not (
        target / "release" / "gear-cli"
    ).exists() else "release"
    argv = ["cargo", "build", "--bin", "gear-cli"]
    if profile == "release":
        argv.append("--release")
    subprocess.run(argv, cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
    return target / profile / "gear-cli"


_BIN = []


def binary():
    """The harness, built once and only when a command is run."""
    if not _BIN:
        _BIN.append(_bin())
    return _BIN[0]

FILES = (
    sorted((ROOT / "docs").glob("*.md"))
    + sorted((ROOT / "docs").glob("*.html"))
    + [ROOT / "README.md"]
)

MARKER = re.compile(
    r"<!--\s*figures(-verbatim|-exempt|-by-test|-bold)?:\s*(.+?)\s*-->"
)

# `**0.970**` -- how `state.md` marks the figure that is current in a table
# whose other column is history.
BOLD = re.compile(r"\*\*(.+?)\*\*", re.S)

# `1.723`, `-0.5`, `1,486`, `5.7e-4`, `2.5e+3`. Thousands separators are stripped
# before comparison; a bare `,` as a decimal point is *not* read as one, because
# where the documents use that form they are quoting ISO rather than this tool.
NUMBER = re.compile(r"[-−]?\d[\d,]*(?:\.\d+)?(?:[eE][-+]?\d+)?")

# Figures that would match almost anything, and say nothing when they do. Tooth
# counts and moduli belong to the *inputs* a table names and are matched anyway
# -- they just do not count toward the strength of the check.
STRONG_DECIMALS = 2


# A Rust numeric literal: `1.777_921_669_562`, `12_u32`, `45.0f64`.
LITERAL = re.compile(
    r"(?<![\w.])(\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][-+]?\d+)?)"
    r"(?:_?(?:f64|f32|u8|u16|u32|u64|usize|i8|i16|i32|i64|isize))?(?!\w)"
)


class _Tests:
    """The crates' Rust, so a `figures-by-test` pointer is read as a test."""

    def __init__(self, text=None):
        self.text = text

    def body(self, name, keep_strings=False):
        """`#[test] fn name`'s body, comments blanked and strings too unless
        kept, or None."""
        if self.text is None:
            self.text = "".join(
                p.read_text() for p in sorted((ROOT / "crates").rglob("*.rs"))
            )
        m = re.search(r"#\[test\]\s*\n\s*fn " + re.escape(name) + r"\(\)[^{]*\{", self.text)
        if not m:
            return None
        i, depth = m.end(), 1
        while depth and i < len(self.text):
            depth += (self.text[i] == "{") - (self.text[i] == "}")
            i += 1
        body = self.text[m.end() : i - 1]
        strings = []

        def hold(m):
            strings.append(m.group(0))
            return f"\x00{len(strings) - 1}\x00"

        body = re.sub(r'"(?:\\.|[^"\\])*"', hold, body, flags=re.S)
        body = re.sub(r"//[^\n]*", " ", body)
        return re.sub(r"\x00(\d+)\x00", lambda m: strings[int(m.group(1))] if keep_strings else " ", body)

    def literals(self, name):
        """The numeric literals in the test's body, in order, signed where a
        `-` stands before one after `(`, `[`, `,`, `=` or the line's start, or
        None."""
        body = self.body(name)
        return None if body is None else signed_literals(body)

    def tuples(self, name):
        """The test's literal tuples: each parenthesised group made of numeric
        literals, `true`/`false`, strings and nested `[...]`/`(...)` alone --
        a row of the table the test holds -- as `(literals in order, signed;
        its strings)`. Or None where there is no such test."""
        body = self.body(name, keep_strings=True)
        if body is None:
            return None
        out = []
        for open_at in (i for i, c in enumerate(body) if c == "("):
            depth, j = 0, open_at
            while j < len(body):
                depth += (body[j] == "(") - (body[j] == ")")
                if depth == 0:
                    break
                j += 1
            inner = body[open_at + 1 : j]
            texts = [t[1:-1] for t in re.findall(r'"(?:\\.|[^"\\])*"', inner)]
            bare = re.sub(r'"(?:\\.|[^"\\])*"', " ", inner)
            rest = re.sub(r"\b(?:true|false)\b", " ", LITERAL.sub(" ", bare))
            if set(rest) <= set(" \t\n,[]()-") and LITERAL.search(bare):
                out.append((signed_literals(bare), texts))
        return out


def signed_literals(code):
    """Numeric literals in Rust source, in order, with the sign a `-` gives
    one where it can only be a sign (after `(`, `[`, `,`, `=` or a line's
    start), so `-0.19_f64` is negative and `a - 1.0` is not."""
    out = []
    for m in LITERAL.finditer(code):
        before = code[: m.start()].rstrip()
        sign = 1.0
        if before.endswith("-") and re.search(r"(?:^|[(\[,=])\s*-$", before[-40:] if len(before) > 40 else before, re.M):
            sign = -1.0
        out.append(sign * float(m.group(1).replace("_", "")))
    return out


TESTS = _Tests()


def numbers(text):
    """Every number in `text`, as (value, decimals-as-printed)."""
    out = []
    for m in NUMBER.finditer(text):
        raw = m.group(0)
        # A comma inside a number is a thousands separator here; a trailing one
        # is punctuation and is not part of it.
        # A typographic minus is a minus: `−0.19` is negative.
        cleaned = raw.replace(",", "").replace("−", "-")
        if not cleaned or cleaned in ("-", "."):
            continue
        try:
            value = float(cleaned)
        except ValueError:
            continue
        frac = cleaned.split(".")[1] if "." in cleaned else ""
        # An exponent carries its own precision; treat it as strong.
        decimals = len(frac.split("e")[0].split("E")[0]) if frac else 0
        if "e" in cleaned or "E" in cleaned:
            decimals = max(decimals, STRONG_DECIMALS)
        out.append((value, decimals, raw))
    return out


def at(value, decimals):
    """The value as the document would have printed it, for comparison."""
    return f"{value:.{decimals}f}"


def blocks(path):
    """(verbatim, commands, first-line-number, text) for each tagged block.

    Markers stack: a table whose figures come from two commands carries two
    markers and is matched against the union of what they print. The canary
    table in `state.md` is exactly that, and splitting it in two to fit a
    one-marker rule would be letting the checker rewrite the document.
    """
    lines = path.read_text().splitlines()
    found = []
    i = 0
    while i < len(lines):
        m = MARKER.search(lines[i])
        if not m:
            i += 1
            continue
        verb = m.group(1) or ""
        commands = [m.group(2)]
        # Any immediately following markers belong to the same block.
        while i + 1 < len(lines) and MARKER.search(lines[i + 1]):
            i += 1
            more = MARKER.search(lines[i])
            if (more.group(1) or "") != verb:
                raise SystemExit(f"{path}:{i + 1}: markers of different kinds cannot stack")
            commands.append(more.group(2))
        start = i + 1
        # Skip blank lines between the marker and what it marks.
        while start < len(lines) and not lines[start].strip():
            start += 1

        # **An exemption is about a section; a gate is about a block.** A gated
        # block is one table, because that is the unit a command regenerates. A
        # document that records history records it in prose and tables together
        # for pages at a stretch, so an exemption runs to the next heading --
        # otherwise `corrections.md`'s log alone would need eleven markers, and
        # a marker per table is a marker per table to keep in step.
        if verb == "-exempt":
            end = start
            while end < len(lines) and not lines[end].startswith("#"):
                end += 1
            found.append((verb, commands, start + 1, "\n".join(lines[start:end])))
            i = end
            continue

        end = start
        fence = None
        while end < len(lines):
            line = lines[end]
            if fence is None and line.strip().startswith("```"):
                fence = line.strip()[:3]
                end += 1
                continue
            if fence is not None:
                if line.strip().startswith(fence):
                    end += 1
                    break
                end += 1
                continue
            if not line.strip():
                break
            end += 1
        found.append((verb, commands, start + 1, "\n".join(lines[start:end])))
        i = end
    return found


def run(command, cache):
    if command in cache:
        return cache[command]
    argv = command.split()
    env = dict(os.environ)
    if argv[0] == "gear-cli":
        argv = [str(binary())] + argv[1:]
    elif re.fullmatch(r"tools/[a-z0-9_]+\.py", argv[0]) and (ROOT / argv[0]).exists():
        # A script that compares the harness with a derivation of its own,
        # handed the same binary.
        argv = [sys.executable, str(ROOT / argv[0])] + argv[1:]
        env["GEAR_CLI"] = str(binary())
    else:
        raise SystemExit(
            f"only `gear-cli ...` or `tools/<script>.py` commands can be tagged: {command}"
        )
    if not binary().exists():
        raise SystemExit(
            f"{binary()} is not built. `cargo build --release --bin gear-cli` first."
        )
    result = subprocess.run(argv, capture_output=True, text=True, env=env)
    cache[command] = result.stdout + result.stderr
    return cache[command]


def untagged(path, tagged_spans):
    """Blocks carrying strong figures that nothing generates, tables and prose.

    **Prose counts.** This scanned runs of table rows only, so a figure quoted in
    a sentence was invisible to it and the coverage line under-reported itself --
    one such block was found by hand, which is what said there would be others.
    There are: about fifty paragraphs across the four documents.

    They are *reported* rather than failed, and separately from the tables,
    because the two are not the same claim. A table of figures is output, and
    something should regenerate it. A figure in a sentence is as often history
    ("the ring came out 6 % low"), or an illustration, or a bound quoted from a
    standard -- and tagging fifty paragraphs to find the few that are live output
    is the shape of sweep this project refuses. What the number is for is that
    nobody reads "5 ungated" as "5 figures in this repository are ungated".

    A generated document is skipped: `docs/bending-check.html` *is* a command's
    output, checked verbatim, so its prose is not a claim anybody wrote.
    """
    lines = path.read_text().splitlines()
    loose = []
    prose_ok = path.suffix != ".html"

    def close(kind, start, end):
        text = "\n".join(lines[start - 1 : end - 1])
        strong = [x for x in numbers(text) if x[1] >= STRONG_DECIMALS]
        if strong and not any(a <= start <= b for a, b in tagged_spans):
            loose.append((start, len(strong), kind))

    run_start = None
    prose_start = None
    fenced = False
    for n, line in enumerate(lines, 1):
        stripped = line.lstrip()
        if stripped.startswith("```"):
            fenced = not fenced
        is_row = not fenced and stripped.startswith("|")
        # A paragraph: text that is neither a table, a heading, nor inside a
        # fence. Blank lines end it, which is what makes it a paragraph.
        is_prose = (
            prose_ok
            and not fenced
            and bool(stripped)
            and not is_row
            and not stripped.startswith("#")
            and not stripped.startswith("<!--")
        )

        if is_row and run_start is None:
            run_start = n
        elif not is_row and run_start is not None:
            close("table", run_start, n)
            run_start = None

        if is_prose and prose_start is None:
            prose_start = n
        elif not is_prose and prose_start is not None:
            close("prose", prose_start, n)
            prose_start = None
    if run_start is not None:
        close("table", run_start, len(lines) + 1)
    if prose_start is not None:
        close("prose", prose_start, len(lines) + 1)
    return loose


# A table row, and the rule under its header.
ROW = re.compile(r"^\s*\|")
RULE = re.compile(r"^\s*\|[\s|:-]+\|\s*$")
# A command a row quotes as its label.
LABEL = re.compile(r"`gear-cli [^`]*`")


def same(value, decimals, other):
    """`other` printed at the document's precision reads as `value` does."""
    return at(other, decimals) == at(value, decimals)


def within(value, decimals, literal):
    """A test's literal reads as the document's figure: the same signed value
    to half a unit of the document's last digit (a test writes `93.25` for a
    figure a document rounds either way)."""
    return abs(literal - value) <= 0.5 * 10**-decimals * (1 + 1e-9)


def in_order(want, have, match=None):
    """`want`'s numbers are an ordered subsequence of `have`'s."""
    match = match or (lambda v, d, h: same(v, d, h))
    j = 0
    for h, _, _ in have:
        if j < len(want) and match(want[j][0], want[j][1], h):
            j += 1
    return j == len(want)


# A code span; one holding a letter is a name (`z 9/37`, `N+1/N/N−1/N`,
# `μ F_n`), where a code span of digits alone is a figure.
SPAN = re.compile(r"`([^`]*)`")


def is_name(span):
    return re.search(r"[^\W\d_]", span) is not None


def row_numbers(line, claimed):
    """A table row's numbers, less its names: a `gear-cli` command it quotes
    and any code span holding a letter."""
    line = SPAN.sub(lambda m: " " if is_name(m.group(1)) else m.group(0), line)
    return numbers(claimed(line))


def row_labels(line):
    """(the commands a row quotes, the names in its first cell): what the row
    is about, which the figures after it must belong to."""
    commands = [c[1:-1] for c in LABEL.findall(line)]
    first = line.strip().strip("|").split("|")[0]
    names = [n for n in SPAN.findall(first) if is_name(n) and not n.startswith("gear-cli ")]
    return commands, names


def check_block(verb, commands, text, output_of, tests=TESTS):
    """(failures, strong, weak) for one tagged block.

    `output_of(command)` is what a command prints; `tests` reads test bodies.
    Both are parameters so `--self-test` can hand in fakes."""
    named = " + ".join(f"`{c}`" for c in commands)
    if verb == "-by-test":
        # A row is one of the test's literal tuples, in order; a line of prose
        # is literals of the test's body, as a bag.
        tuples = tests.tuples(commands[0])
        if tuples is None:
            return [f"no #[test] fn named `{commands[0]}` -- this block claims a gate that does not exist"], 0, 0
        literals = [(l, 0, "") for l in tests.literals(commands[0])]
        failures = []
        for line in text.splitlines():
            if RULE.match(line):
                continue
            if ROW.match(line):
                row = row_numbers(line, lambda f: f)
                _, names = row_labels(line)
                # A row's name is one of its tuple's strings: the row the test
                # holds for that name, not any row with the same figures.
                have = [
                    [(l, 0, "") for l in lits]
                    for lits, texts in tuples
                    if all(n in texts for n in names)
                ]
                if row and not any(in_order(row, t, within) for t in have):
                    failures.append(
                        f"`{commands[0]}` holds no tuple"
                        + (f" named {names}" if names else "")
                        + " reading, in order: "
                        + ", ".join(raw for _, _, raw in row[:10])
                    )
            else:
                missing = [
                    raw for v, d, raw in numbers(line)
                    if not any(within(v, d, l) for l, _, _ in literals)
                ]
                if missing:
                    failures.append(f"`{commands[0]}` holds no literal for: " + ", ".join(missing[:8]))
        return failures, 0, 0

    outputs = [numbers(output_of(c)) for c in commands]
    union = [n for o in outputs for n in o]
    # A row is matched against one printed line: two rows' figures, or a
    # figure from a later line, do not make a row. The line must be printed by
    # the command a row quotes, if it quotes one, and a row's named code span
    # must read, in order, in that command's first line: its heading, which
    # names what the command was asked (`pair z 9/37`).
    printed = {c: output_of(c).splitlines() for c in commands}
    failures, strong, weak = [], 0, 0

    def claimed(fragment):
        return "\n".join(BOLD.findall(fragment)) if verb == "-bold" else fragment

    prose = []
    for line in text.splitlines():
        if not ROW.match(line):
            prose.append(line)
            continue
        if RULE.match(line):
            continue
        row = row_numbers(line, claimed)
        strong += sum(d >= STRONG_DECIMALS for _, d, _ in row)
        weak += sum(d < STRONG_DECIMALS for _, d, _ in row)
        quoted, names = row_labels(line)
        label = numbers(" ".join(names))

        def fits(c):
            lines = printed[c]
            heading = numbers(lines[0]) if lines else []
            return (not quoted or c in quoted) and (not label or in_order(label, heading))

        if row and not any(
            in_order(row, numbers(l)) for c in commands if fits(c) for l in printed[c]
        ):
            failures.append(
                f"{named} prints no row reading, in order: "
                + ", ".join(raw for _, _, raw in row[:10])
            )
    bag = numbers(claimed("\n".join(prose)))
    strong += sum(d >= STRONG_DECIMALS for _, d, _ in bag)
    weak += sum(d < STRONG_DECIMALS for _, d, _ in bag)
    missing = [raw for v, d, raw in bag if not any(same(v, d, h) for h, _, _ in union)]
    if missing:
        failures.append(
            f"{named} no longer prints: "
            + ", ".join(missing[:8])
            + (f" (+{len(missing) - 8} more)" if len(missing) > 8 else "")
        )
    return failures, strong, weak


def main():
    if "--self-test" in sys.argv:
        return self_test()
    listing = "--list" in sys.argv
    cache = {}
    failures = []
    checked = tagged_strong = tagged_weak = exempt = by_test = 0
    tagged_blocks = 0
    coverage = []

    for path in FILES:
        rel = path.relative_to(ROOT)
        spans = []
        for verb, commands, line, text in blocks(path):
            spans.append((line, line + text.count("\n") + 1))
            if verb == "-exempt":
                exempt += 1
                if listing:
                    coverage.append(f"  {rel}:{line}  exempt    -- {commands[0]}")
                continue
            named = " + ".join(f"`{c}`" for c in commands)
            if verb == "-verbatim":
                tagged_blocks += 1
                output = "\n".join(run(c, cache) for c in commands)
                if output.strip() not in path.read_text():
                    failures.append(f"{rel}:{line}: {named} no longer prints what this file contains")
                else:
                    checked += 1
                if listing:
                    coverage.append(f"  {rel}:{line}  verbatim  <- {' + '.join(commands)}")
                continue
            if verb == "-bold" and not BOLD.findall(text):
                failures.append(
                    f"{rel}:{line}: {named} is tagged `figures-bold` and the block "
                    "has no bold figures -- nothing would be checked"
                )
                continue
            if verb == "-by-test":
                by_test += 1
                if listing:
                    coverage.append(f"  {rel}:{line}  by test  <- {commands[0]}")
            else:
                tagged_blocks += 1
            found, strong, weak = check_block(verb, commands, text, lambda c: run(c, cache))
            failures += [f"{rel}:{line}: {f}" for f in found]
            checked += strong + weak
            tagged_strong += strong
            tagged_weak += weak
            if listing and verb != "-by-test":
                coverage.append(f"  {rel}:{line}  {strong} strong figures  <- {' + '.join(commands)}")

        if listing:
            for line, strong, kind in untagged(path, spans):
                coverage.append(f"  {rel}:{line}  {strong} strong figures  <- NOTHING ({kind})")

    if listing:
        print("Tagged blocks, and blocks carrying figures that nothing generates:\n")
        print("\n".join(coverage))
        tables = sum(1 for c in coverage if c.endswith("(table)"))
        prose = sum(1 for c in coverage if c.endswith("(prose)"))
        print(
            f"\n{tagged_blocks} gated by a command, {by_test} by a test, "
            f"{exempt} exempt, {tables} tables and {prose} paragraphs still "
            f"ungated (figures at {STRONG_DECIMALS}+ decimals)."
        )
        return 0

    if failures:
        print("\n".join(failures), file=sys.stderr)
        print(
            f"\n{len(failures)} tagged block(s) no longer match what generates them.",
            file=sys.stderr,
        )
        print(
            "That is a question, not a failure: run the command, read the diff, and "
            "update the document if the move was meant.",
            file=sys.stderr,
        )
        return 1

    print(
        f"{tagged_blocks} tagged blocks: {checked} figures still printed "
        f"({tagged_strong} at {STRONG_DECIMALS}+ decimals, {tagged_weak} weaker); "
        f"{by_test} blocks gated by a named test, {exempt} sections exempt"
    )
    return 0


# --- the rules, on a fixture ----------------------------------------------

_FIXTURE_OUTPUT = {
    "gear-cli shifts 9 37": (
        "pair z 9/37\n"
        "least shift   0.4736  0.0000  0.4736  1.3280  97.561 %\n"
        "least loss    0.6746  0.7332  1.4078  1.2929  97.678 %\n"
    ),
    "gear-cli shifts 17 43": (
        "pair z 17/43\n"
        "least shift   0.0057  0.0000  0.0057  1.5993  98.345 %\n"
        "least loss    0.6100  0.6466  1.2566  1.4626  98.488 %\n"
    ),
    "gear-cli hulaband 18": (
        "  d  z  module  backlash\n"
        "  1  18  1.000  0.37141\n"
        "  9  162  0.111  0.01429\n"
    ),
    "gear-cli strength 17 43 2.0": (
        "sigma_F  66.80  56.00 MPa\nsigma_H 692.70 MPa\nrho 1.7231 mm\neta 98.7412 %\n"
    ),
}

_FIXTURE_TEST = """
    #[test]
    fn the_fixture_table_is_the_one_this_code_prints() {
        for (reduction, meshes, keeps) in [(144.0, 98.85, 37.9), (324.0, 99.18, 27.4), (-323.0, 99.18, 27.2)] {
            for (name, shifts) in [("N+1/N", [-0.19_f64, 0.37]), ("N/N+1", [-0.11, 0.11])] {}
            // 12.34 in a comment is not a literal
            assert!(check(reduction, meshes, keeps), "not 56.78 either");
        }
    }
"""

_FIXTURE_BLOCKS = [
    ("", ["gear-cli shifts 9 37", "gear-cli shifts 17 43"],
     "| | Σx | ε | η |\n|---|---|---|---|\n"
     "| `z 9/37`, least shift | 0.4736 | 1.3280 | 97.561 % |\n"
     "| `z 9/37`, least loss | 1.4078 | 1.2929 | **97.678 %** |\n"
     "| `z 17/43`, least shift | 0.0057 | 1.5993 | 98.345 % |\n"
     "| `z 17/43`, least loss | 1.2566 | 1.4626 | **98.488 %** |"),
    ("", ["gear-cli strength 17 43 2.0"],
     "| | |\n|---|---|\n"
     "| `σ_F` | 66.8 / 56.0 MPa |"),
    ("", ["gear-cli strength 17 43 2.0"], "Contact is 692.7 MPa and the loss 98.741 %."),
    ("-by-test", ["the_fixture_table_is_the_one_this_code_prints"],
     "| reduction | meshes | the stage |\n|---|---|---|\n"
     "| 144 | 98.85 % | 37.9 % |\n| 324 | 99.18 % | 27.4 % |\n| −323 | 99.18 % | 27.2 % |\n"
     "| `N+1/N` | −0.19, +0.37 |\n| `N/N+1` | −0.11, +0.11 |"),
    ("", ["gear-cli hulaband 18"],
     "| d | z | module | backlash out |\n|---|---|---|---|\n"
     "| 1 | 18 | 1.000 | 0.371° |\n| 9 | 162 | 0.111 | 0.014° |"),
]

# Each planted fault: (name, block index, text replaced, replacement).
_FIXTURE_FAULTS = [
    ("F1 a figure that drifted", 1, "66.8 /", "66.9 /"),
    ("F2 a figure swapped between rows", 0, "| 1.3280 | 97.561 % |", "| 1.3280 | 98.345 % |"),
    ("F2 ...and back", 0, "| 1.5993 | 98.345 % |", "| 1.5993 | 97.561 % |"),
    ("F3 a figure the test does not hold", 3, "37.9 %", "39.9 %"),
    ("F4 two figures swapped within a row", 1, "66.8 / 56.0", "56.0 / 66.8"),
    ("F5 a prose figure that drifted", 2, "692.7", "692.9"),
    ("F6 a test that does not exist", 3, None, None),
    ("F7 a literal only in a comment", 3, "27.4 %", "12.34 %"),
    ("F8 a row's figure taken from a later printed line", 4, "| 1.000 | 0.371° |", "| 1.000 | 0.014° |"),
    ("F9 a figure from another row of the test", 3, "| 144 | 98.85 % | 37.9 % |", "| 144 | 98.85 % | 27.4 % |"),
    ("F10 a figure from another tuple of the test", 3, "| 324 | 99.18 % | 27.4 % |", "| 324 | 99.18 % | 27.2 % |"),
    ("F11 a row's figures swapped with another pair's", 0,
     "| `z 9/37`, least loss | 1.4078 | 1.2929 | **97.678 %** |",
     "| `z 9/37`, least loss | 1.2566 | 1.4626 | **98.488 %** |"),
    ("F12 a figure's sign flipped", 3, "| `N+1/N` | −0.19,", "| `N+1/N` | +0.19,"),
    ("F13 two figures' signs swapped", 3, "| −0.11, +0.11 |", "| +0.11, −0.11 |"),
    ("F14 a row under another row's name", 3, "| `N/N+1` | −0.11, +0.11 |", "| `N+1/N` | −0.11, +0.11 |"),
]


def self_test():
    """The fixture passes, and every planted fault fails it."""
    tests = _Tests(_FIXTURE_TEST)
    output_of = _FIXTURE_OUTPUT.__getitem__
    bad = []
    for i, (verb, commands, text) in enumerate(_FIXTURE_BLOCKS):
        found, _, _ = check_block(verb, commands, text, output_of, tests)
        if found:
            bad.append(f"the fixture's block {i} fails: {found}")
    faults = list(_FIXTURE_FAULTS)
    # F2's second half is the same edit completed: both rows swapped.
    swap = next(f for f in faults if f[0] == "F2 ...and back")
    faults.remove(swap)
    for name, i, old, new in faults:
        verb, commands, text = _FIXTURE_BLOCKS[i]
        if name.startswith("F6"):
            commands = ["no_such_test"]
        else:
            assert old in text, (name, old)
            text = text.replace(old, new, 1)
            if name.startswith("F2"):
                text = text.replace(swap[2], swap[3], 1)
        found, _, _ = check_block(verb, commands, text, output_of, tests)
        print(f"{'caught' if found else 'MISSED'}  {name}")
        if not found:
            bad.append(f"{name} passes")
    if bad:
        print("\n".join(bad), file=sys.stderr)
        return 1
    print(f"the fixture passes and all {len(faults)} planted faults fail")
    return 0


if __name__ == "__main__":
    sys.exit(main())
