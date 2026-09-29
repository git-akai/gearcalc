#!/usr/bin/env python3
"""**Every angle says what unit it is in, and no name means both.**

This crate works in two angular units and has to: a designer states a pressure
angle in degrees because that is what a drawing says, and every trigonometric
expression wants radians. Both are right. What is not right is a *name* that
means one thing in one module and the other thing in the next — and this project
had four, plus eight angles that stated no unit at all.

It is not a hypothetical. `Screw::least_distance_lead_angle` was written taking a
shaft angle in radians, the stage's `shaft_angle` holds degrees, and the call site
read correctly to its author and was wrong. The field was *documented*; the
documentation is not what a reader checks. So the name carries it now, and this
keeps that true.

**The rule, in two halves:**

1. **Every angular field states its unit**: a `_rad` or `_deg` suffix, or
   in its doc comment the word `radians`, or `degrees` / `°` — the first of
   them, where a doc names both to relate them.
2. **No angular name is used in both units.** Where one otherwise would be, the
   radian one takes a `_rad` suffix (and a degree one may take `_deg`), so a
   mismatched call site reads wrong instead of reading fine.

Every struct field of any visibility and any float container is read. Greek
names — `alpha_*`, `beta`, `gamma`, `sigma`, `theta`, `psi`, `phi`, `delta` —
are radians by mathematical
convention and still have to say so, because a reader who does not know that
convention is exactly the reader this is for.

    tools/check_units.py            # exits non-zero on either fault
"""
import re
import sys
from pathlib import Path

# A struct field of any visibility, holding a float in any container: a bare
# `f64` or `f32`, an `Auto`/`Option`, a `Vec`, an array or a tuple. The pattern
# once matched `pub` fields of bare `f64` alone, so every automatic angle, every
# `pub(crate)` or private one and every list of angles went unchecked.
FIELD = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?([a-z_0-9]+): ([^,]*\bf(?:64|32)\b[^,]*),\s*(?://.*)?$")
# The opening line of a struct with named fields, where FIELD is read; a
# function's parameters look the same and carry no doc to read.
STRUCT = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?struct \w+[^;(]*\{\s*$")
UNIT = re.compile(r"\bradians\b|degrees|°")

# Names that read as angles and are not. Each is a real quantity in some other
# unit, so demanding an angular unit of it would be demanding a wrong answer.
# `lead` is not a pattern at all: it is a length throughout the crate.
NOT_ANGLES = {
    # An amplitude in modules, despite the name: `x(θ) = shift + a·cos θ`.
    "angular_shift",
    # An arc length along the pitch circle, in the same units as `Tooth::ac`.
    "phase",
    # A sign, not a quantity.
    "rolling_power_sign",
}


def looks_angular(name: str) -> bool:
    if name in NOT_ANGLES:
        return False
    return "angle" in name or re.match(
        r"(alpha|beta|gamma|sigma|theta|psi|phi|delta|helix)(\d|_|$)", name
    ) is not None


def stated_unit(name: str, doc: str):
    """The unit a field states: its suffix, else the first unit word of its doc."""
    if name.endswith("_rad"):
        return "radians"
    if name.endswith("_deg"):
        return "degrees"
    m = UNIT.search(doc)
    if not m:
        return None
    return "radians" if m.group(0) == "radians" else "degrees"


def doc_above(lines: list[str], i: int) -> str:
    """The doc comment attached to line `i`, read upwards."""
    out, j = [], i - 1
    while j >= 0 and (lines[j].strip().startswith("///") or lines[j].strip().startswith("#[")):
        out.append(lines[j].strip())
        j -= 1
    return " ".join(reversed(out))


def fields(lines: list[str]):
    """(line index, name) of every float field inside a struct body."""
    depth = None
    for i, line in enumerate(lines):
        if depth is None:
            if STRUCT.match(line):
                depth = 0
            else:
                continue
        depth += line.count("{") - line.count("}")
        m = FIELD.match(line)
        if m and depth == 1:
            yield i, m.group(1)
        if depth <= 0:
            depth = None


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    silent: list[str] = []
    units: dict[str, set[str]] = {}
    where: dict[str, list[str]] = {}
    total = 0

    for path in sorted(root.glob("crates/**/*.rs")):
        lines = path.read_text().split("\n")
        for i, name in fields(lines):
            if not looks_angular(name):
                continue
            total += 1
            at = f"{path.relative_to(root)}:{i + 1}"
            unit = stated_unit(name, doc_above(lines, i))
            if unit is None:
                silent.append(f"{at}  {name}")
                continue
            units.setdefault(name, set()).add(unit)
            where.setdefault(name, []).append(f"{unit:8} {at}")

    both = {n: u for n, u in units.items() if len(u) > 1}
    status = 0

    if silent:
        status = 1
        print("angles that do not say what unit they are in:", file=sys.stderr)
        for s in silent:
            print(f"  {s}", file=sys.stderr)

    if both:
        status = 1
        print("\nnames used in **both** units — suffix the radian one `_rad`:", file=sys.stderr)
        for n in sorted(both):
            print(f"  {n}", file=sys.stderr)
            for w in where[n]:
                print(f"      {w}", file=sys.stderr)

    if status == 0:
        print(f"{total} angular fields, all stating one unit, no name meaning two")
    return status


if __name__ == "__main__":
    sys.exit(main())
