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

The block runs from the marker to the next blank line -- a Markdown table, a
list, a paragraph -- or to the closing fence of a code block. Every number in it
must appear in what the command prints, **at the document's own precision**: a
document saying 98.74 is satisfied by an output of 98.7412, and one saying
98.741 is not.

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
information a reader wants. The test's name is checked to exist, because a
pointer that no longer resolves is the exact rot this file exists to prevent.

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

    tools/check_figures.py           # exits non-zero and names what drifted
    tools/check_figures.py --list    # what is tagged, what is not, and by what

Dependency-free, like its siblings here.
"""

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def _bin():
    """The harness, **built** -- release if that is what is there, else debug.

    The two print the same bytes, measured rather than assumed, and
    `tools/check_golden.sh` says so at more length. It matters because CI builds
    debug and a documented figure is not worth a second toolchain pass to check.

    It is *built* rather than merely found, which it was not: a command added to
    the harness was invisible here until something else forced a rebuild, so a
    figure tagged with it read as drifted when the document was right and the
    binary was old. That is the stale-binary fault `docs/corrections.md` records,
    met in the second of the two instruments written to catch drift -- the first
    was `check_golden.sh`, and the fix is the same one. Cargo is incremental, so
    an up-to-date tree pays nothing.
    """
    profile = "debug" if (ROOT / "target" / "debug" / "gear-cli").exists() and not (
        ROOT / "target" / "release" / "gear-cli"
    ).exists() else "release"
    argv = ["cargo", "build", "--bin", "gear-cli"]
    if profile == "release":
        argv.append("--release")
    subprocess.run(argv, cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
    return ROOT / "target" / profile / "gear-cli"


BIN = _bin()

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
NUMBER = re.compile(r"-?\d[\d,]*(?:\.\d+)?(?:[eE][-+]?\d+)?")

# Figures that would match almost anything, and say nothing when they do. Tooth
# counts and moduli belong to the *inputs* a table names and are matched anyway
# -- they just do not count toward the strength of the check.
STRONG_DECIMALS = 2


class _Tests:
    """Every `fn name(` in the crates, so a `figures-by-test` pointer resolves."""

    def __init__(self):
        self.text = None

    def search(self, needle):
        if self.text is None:
            self.text = "".join(
                p.read_text() for p in sorted((ROOT / "crates").rglob("*.rs"))
            )
        return needle in self.text


TESTS = _Tests()


def numbers(text):
    """Every number in `text`, as (value, decimals-as-printed)."""
    out = []
    for m in NUMBER.finditer(text):
        raw = m.group(0)
        # A comma inside a number is a thousands separator here; a trailing one
        # is punctuation and is not part of it.
        cleaned = raw.replace(",", "")
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
    if argv[0] == "gear-cli":
        argv = [str(BIN)] + argv[1:]
    else:
        raise SystemExit(f"only `gear-cli ...` commands can be tagged: {command}")
    if not BIN.exists():
        raise SystemExit(
            f"{BIN} is not built. `cargo build --release --bin gear-cli` first."
        )
    result = subprocess.run(argv, capture_output=True, text=True)
    cache[command] = result.stdout + result.stderr
    return cache[command]


def untagged(path, tagged_spans):
    """Tables and code blocks carrying strong figures that nothing generates."""
    lines = path.read_text().splitlines()
    loose = []
    run_start = None
    for n, line in enumerate(lines, 1):
        is_row = line.lstrip().startswith("|")
        if is_row and run_start is None:
            run_start = n
        elif not is_row and run_start is not None:
            text = "\n".join(lines[run_start - 1 : n - 1])
            strong = [x for x in numbers(text) if x[1] >= STRONG_DECIMALS]
            if strong and not any(a <= run_start <= b for a, b in tagged_spans):
                loose.append((run_start, len(strong)))
            run_start = None
    return loose


def main():
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
            if verb == "-by-test":
                name = commands[0]
                if not TESTS.search(f"fn {name}("):
                    failures.append(
                        f"{rel}:{line}: no test named `{name}` -- "
                        "this block claims a gate that does not exist"
                    )
                else:
                    by_test += 1
                if listing:
                    coverage.append(f"  {rel}:{line}  by test  <- {name}")
                continue
            if verb == "-exempt":
                exempt += 1
                if listing:
                    coverage.append(f"  {rel}:{line}  exempt    -- {commands[0]}")
                continue
            tagged_blocks += 1
            output = "\n".join(run(c, cache) for c in commands)
            named = " + ".join(f"`{c}`" for c in commands)
            claimed = "\n".join(BOLD.findall(text)) if verb == "-bold" else text
            if verb == "-bold" and not claimed.strip():
                failures.append(
                    f"{rel}:{line}: {named} is tagged `figures-bold` and the block "
                    "has no bold figures -- nothing would be checked"
                )
                continue

            if verb == "-verbatim":
                if text.strip() and text.strip() in output:
                    # The document quotes the command; the useful direction is
                    # the other one -- everything the command prints is here.
                    pass
                if output.strip() not in path.read_text():
                    failures.append(
                        f"{rel}:{line}: {named} no longer prints what this file contains"
                    )
                else:
                    checked += 1
                if listing:
                    coverage.append(f"  {rel}:{line}  verbatim  <- {' + '.join(commands)}")
                continue

            have = numbers(output)
            missing = []
            for value, decimals, raw in numbers(claimed):
                checked += 1
                if decimals >= STRONG_DECIMALS:
                    tagged_strong += 1
                else:
                    tagged_weak += 1
                want = at(value, decimals)
                if not any(at(v, decimals) == want for v, _, _ in have):
                    missing.append(raw)
            if missing:
                failures.append(
                    f"{rel}:{line}: {named} no longer prints: "
                    + ", ".join(missing[:8])
                    + (f" (+{len(missing) - 8} more)" if len(missing) > 8 else "")
                )
            if listing:
                strong = sum(1 for _, d, _ in numbers(claimed) if d >= STRONG_DECIMALS)
                coverage.append(
                    f"  {rel}:{line}  {strong} strong figures  <- {' + '.join(commands)}"
                )

        if listing:
            for line, strong in untagged(path, spans):
                coverage.append(f"  {rel}:{line}  {strong} strong figures  <- NOTHING")

    if listing:
        print("Tagged blocks, and tables carrying figures that nothing generates:\n")
        print("\n".join(coverage))
        loose = sum(1 for c in coverage if c.endswith("NOTHING"))
        print(
            f"\n{tagged_blocks} gated by a command, {by_test} by a test, "
            f"{exempt} exempt, {loose} still ungated "
            f"(tables with figures at {STRONG_DECIMALS}+ decimals)."
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


if __name__ == "__main__":
    sys.exit(main())
