#!/usr/bin/env python3
"""Validate an exported DXF with an independent parser.

The Rust tests check the writer against our own understanding of DXF, which
proves only self-consistency. This reads the file back with ezdxf -- a mature,
unrelated implementation -- and checks the geometry that comes out matches the
gear that went in.

It is the same standard used elsewhere in this project: verify against something
that does not share the code under test.

Usage:
    validate_dxf.py <file.dxf> --root R --tip T --base B --pitch P --teeth Z
"""

import argparse
import math
import sys

try:
    import ezdxf
    from ezdxf.path import make_path
except ImportError:
    sys.exit("ezdxf is required: run inside `nix develop`")

TOL = 1e-6  # mm; the exporter writes 12 decimal places


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("path")
    ap.add_argument("--root", type=float, required=True)
    ap.add_argument("--tip", type=float, required=True)
    ap.add_argument("--base", type=float, required=True)
    ap.add_argument("--pitch", type=float, required=True)
    ap.add_argument("--teeth", type=int, required=True)
    a = ap.parse_args()

    doc = ezdxf.readfile(a.path)
    ents = list(doc.modelspace())
    fail = []

    def check(ok, msg):
        print(("  ok   " if ok else "  FAIL ") + msg)
        if not ok:
            fail.append(msg)

    print(f"{a.path}: parsed as {doc.dxfversion}")

    polys = [e for e in ents if e.dxftype() == "LWPOLYLINE"]
    circles = [e for e in ents if e.dxftype() == "CIRCLE"]
    check(len(polys) == 1, f"exactly one profile polyline (got {len(polys)})")
    if not polys:
        return 1
    pl = polys[0]

    check(bool(pl.closed), "profile polyline is closed")
    check(pl.dxf.layer == "GEAR_PROFILE", f"profile on its own layer (got {pl.dxf.layer})")

    # Arcs are carried as bulges: the tip arc plus the root arc in two halves,
    # per tooth.
    bulged = sum(1 for p in pl.get_points("xyb") if abs(p[2]) > 1e-12)
    check(
        bulged == 3 * a.teeth,
        f"{bulged} arc segments, expected 3 per tooth = {3 * a.teeth}",
    )

    # Expand the arcs and check the profile against the analytic radii. This is
    # the part that would catch a wrong bulge: a bad arc leaves the tip or root
    # circle.
    flat = list(make_path(pl).flattening(distance=TOL / 2))
    radii = [math.hypot(v.x, v.y) for v in flat]
    check(
        min(radii) >= a.root - TOL and max(radii) <= a.tip + TOL,
        f"every point within [root, tip]: {min(radii):.6f}..{max(radii):.6f} "
        f"vs [{a.root:.6f}, {a.tip:.6f}]",
    )
    check(
        abs(min(radii) - a.root) < TOL,
        f"root arc reaches the root circle exactly ({min(radii):.6f} vs {a.root:.6f})",
    )
    check(
        abs(max(radii) - a.tip) < TOL,
        f"tip arc reaches the tip circle exactly ({max(radii):.6f} vs {a.tip:.6f})",
    )

    # Orientation and enclosed area.
    area = 0.5 * sum(
        p.x * q.y - q.x * p.y for p, q in zip(flat, list(flat[1:]) + [flat[0]])
    )
    check(area > 0, f"wound counter-clockwise (signed area {area:.3f} mm^2)")
    check(
        math.pi * a.root**2 < area < math.pi * a.tip**2,
        f"area {area:.3f} lies between the root and tip circles "
        f"({math.pi * a.root ** 2:.3f}..{math.pi * a.tip ** 2:.3f})",
    )

    # Reference circles.
    got = sorted(round(c.dxf.radius, 6) for c in circles)
    want = sorted(round(v, 6) for v in (a.root, a.base, a.pitch, a.tip))
    check(got == want, f"reference circles {got} match {want}")
    check(
        all(c.dxf.layer == "GEAR_REFERENCE" for c in circles),
        "reference circles on the construction layer",
    )

    # z-fold symmetry: rotating by one pitch must map the outline onto itself.
    step = 2 * math.pi / a.teeth
    c, s = math.cos(step), math.sin(step)
    pts = [(v.x, v.y) for v in flat]
    worst = 0.0
    for x, y in pts[:: max(1, len(pts) // 200)]:
        rx, ry = x * c - y * s, x * s + y * c
        worst = max(worst, min(math.hypot(rx - px, ry - py) for px, py in pts))
    check(worst < 5e-3, f"outline is periodic in one tooth pitch (worst {worst:.2e} mm)")

    if fail:
        print(f"\n{len(fail)} check(s) failed")
        return 1
    print("\nall checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
