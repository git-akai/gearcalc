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

1. **Every angular field states its unit** in its doc comment — the word
   `radians`, or `degrees` / `°`.
2. **No angular name is used in both units.** Where one otherwise would be, the
   radian one takes a `_rad` suffix (and a degree one may take `_deg`), so a
   mismatched call site reads wrong instead of reading fine.

Greek names — `alpha_*`, `beta`, `gamma`, `sigma` — are radians by mathematical
convention and still have to say so, because a reader who does not know that
convention is exactly the reader this is for.

    tools/check_units.py            # exits non-zero on either fault
"""
import re
import sys
from pathlib import Path

FIELD = re.compile(r"^\s*pub ([a-z_0-9]+): (\[?f64[^,]*),")
RADIANS = re.compile(r"\bradians\b")
DEGREES = re.compile(r"degrees|°")

# Names that read as angles and are not. Each is a real quantity in some other
# unit, so demanding an angular unit of it would be demanding a wrong answer.
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
    return "angle" in name or re.match(r"(alpha|beta|gamma|sigma)(_|$)", name) is not None


def doc_above(lines: list[str], i: int) -> str:
    """The doc comment attached to line `i`, read upwards."""
    out, j = [], i - 1
    while j >= 0 and (lines[j].strip().startswith("///") or lines[j].strip().startswith("#[")):
        out.append(lines[j].strip())
        j -= 1
    return " ".join(reversed(out))


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    silent: list[str] = []
    units: dict[str, set[str]] = {}
    where: dict[str, list[str]] = {}
    total = 0

    for path in sorted(root.glob("crates/**/*.rs")):
        lines = path.read_text().split("\n")
        for i, line in enumerate(lines):
            m = FIELD.match(line)
            if not m or not looks_angular(m.group(1)):
                continue
            name = m.group(1)
            total += 1
            blob = doc_above(lines, i)
            at = f"{path.relative_to(root)}:{i + 1}"
            if RADIANS.search(blob):
                unit = "radians"
            elif DEGREES.search(blob):
                unit = "degrees"
            else:
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
