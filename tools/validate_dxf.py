#!/usr/bin/env python3
"""Validate an exported DXF with an independent parser.

The Rust tests check the writer against our own understanding of DXF, which
proves only self-consistency. This reads the file back with ezdxf -- a mature,
unrelated implementation -- and checks the geometry that comes out matches the
gear that went in.

It is the same standard used elsewhere in this project: verify against something
that does not share the code under test.

**With one correction, learnt the hard way.** An independent reader that
*repairs* what it reads is not a check. ezdxf builds a document: every structure
a file leaves out, it supplies from its own template and then reports the result
as valid. This export shipped with no BLOCKS section, no BLOCK_RECORD table and
no OBJECTS section -- and ezdxf invented all three, so this tool passed it while
SOLIDWORKS, which repairs nothing, refused the file outright.

So the structure is read from the raw tags, before ezdxf is allowed near them.
The full requirement is gated in `gear_io::dxf`'s own tests -- one list, in one
place; what is checked here is the shape that reading cannot recover.

What the gear should be is derived here, from its inputs, by the textbook
relations -- not read from the file, and not from the crate. Each arc is read
from its chord and bulge the way CAD reads it, so an arc bulged onto the wrong
chord is off the axis and fails, however well its end vertices sit.

Usage:
    validate_dxf.py <file.dxf> --teeth Z [--module M] [--shift X]
        [--pressure-angle A] [--angular-shift DX]      external or eccentric
    validate_dxf.py <file.dxf> --ring --teeth Z [--module M] [--pressure-angle A]
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

# The ISO 53 basic rack the harness exports with, in modules.
ADDENDUM = 1.0
DEDENDUM = 1.25
TIP_ROUND = 0.38


def inv(a):
    return math.tan(a) - a


def vertices(pl):
    """(x, y, bulge) of each vertex, as written."""
    return [(p[0], p[1], p[2]) for p in pl.get_points("xyb")]


def arc_of(a, b, bulge):
    """The arc a bulged span stands for: (centre, radius at each end, sweep).

    bulge = tan(sweep/4); the centre is on the chord's bisector, to the left of
    travel for a counter-clockwise (positive) sweep, c/2 * (1 - b^2)/(2b) from
    the chord's middle.
    """
    sweep = 4.0 * math.atan(bulge)
    dx, dy = b[0] - a[0], b[1] - a[1]
    chord = math.hypot(dx, dy)
    h = chord / 2.0 * (1.0 - bulge * bulge) / (2.0 * bulge)
    c = ((a[0] + b[0]) / 2.0 - dy / chord * h, (a[1] + b[1]) / 2.0 + dx / chord * h)
    return c, math.hypot(a[0] - c[0], a[1] - c[1]), math.hypot(b[0] - c[0], b[1] - c[1]), sweep


class External:
    """An external gear cut by the ISO rack, from its inputs alone: spur, at
    shift x + dx cos(theta) for the tooth at theta."""

    def __init__(self, z, m, x, alpha_deg, dx):
        self.z, self.m, self.x, self.dx = z, m, x, dx
        self.alpha = math.radians(alpha_deg)
        self.r = m * z / 2.0
        self.rb = self.r * math.cos(self.alpha)

    def shift(self, k):
        return self.x + self.dx * math.cos(2.0 * math.pi * k / self.z)

    def tip(self, x):
        return self.r + self.m * (ADDENDUM + x)

    def tip_half_angle(self, x):
        """Half the tip land's angle: psi_b - inv(alpha at the tip)."""
        s = self.m * (math.pi / 2.0 + 2.0 * x * math.tan(self.alpha))
        psi_b = s / (2.0 * self.r) + inv(self.alpha)
        return psi_b - inv(math.acos(self.rb / self.tip(x)))

    def root(self, x):
        return self.r - self.m * (DEDENDUM - x)

    def root_half_angle(self, x):
        """Where the rack's tip round leaves the root circle, from the tooth's
        centreline: its corner's lateral place over the pitch radius."""
        s = self.m * (math.pi / 2.0 + 2.0 * x * math.tan(self.alpha))
        rho = TIP_ROUND * self.m
        b_c = self.m * (DEDENDUM - x) - rho
        a_c = s / 2.0 + b_c * math.tan(self.alpha) + rho / math.cos(self.alpha)
        return a_c / self.r


def raw_tags(path):
    """The file as (group code, value) pairs, which is all a DXF is."""
    with open(path, encoding="utf-8") as f:
        lines = f.read().splitlines()
    return [(lines[i].strip(), lines[i + 1]) for i in range(0, len(lines) - 1, 2)]


def structure(path, check):
    """What the file itself declares -- read before any parser can repair it.

    Two properties, both of which a rebuilding reader hides: the sections a
    reader walks in order, and the ownership graph. In R2000 every record names
    its owner by handle (group code 330) and every entity is owned by the block
    record of the space it is drawn in; a pointer into nothing is exactly what
    separates a file a lenient reader fixes from one a strict reader rejects.
    """
    tags = raw_tags(path)
    sections = [tags[i + 1][1] for i, (c, v) in enumerate(tags) if c == "0" and v == "SECTION"]
    check(
        sections == ["HEADER", "CLASSES", "TABLES", "BLOCKS", "ENTITIES", "OBJECTS"],
        f"the six sections R2000 requires, in order (got {sections})",
    )

    handles = {v for c, v in tags if c in ("5", "105")}
    dangling = sorted(
        {v for c, v in tags if c in ("330", "340", "350") and v != "0" and v not in handles}
    )
    check(not dangling, f"every owner and pointer resolves to a handle in the file {dangling or ''}")

    # The entities must be owned by model space, or they are in the file and in
    # no layout -- present to a reader that rebuilds, absent to one that does not.
    model_space = None
    for i, (c, v) in enumerate(tags):
        if c == "0" and v == "BLOCK_RECORD":
            body = []
            for j in range(i + 1, len(tags)):
                if tags[j][0] == "0":
                    break
                body.append(tags[j])
            if any(c2 == "2" and v2 == "*Model_Space" for c2, v2 in body):
                model_space = next(v2 for c2, v2 in body if c2 == "5")
    check(model_space is not None, "a *Model_Space block record exists")

    owners = [
        tags[i + 2] for i, (c, v) in enumerate(tags) if c == "0" and v in ("LWPOLYLINE", "CIRCLE")
    ]
    check(
        bool(owners) and all(o == ("330", model_space) for o in owners),
        f"every entity is owned by model space (handle {model_space})",
    )


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("path")
    ap.add_argument("--teeth", type=int, required=True)
    ap.add_argument("--module", type=float, default=1.0)
    ap.add_argument("--shift", type=float, default=0.0)
    ap.add_argument("--pressure-angle", type=float, default=20.0)
    ap.add_argument("--angular-shift", type=float, default=0.0)
    ap.add_argument("--ring", action="store_true")
    a = ap.parse_args()

    fail = []

    def check(ok, msg):
        print(("  ok   " if ok else "  FAIL ") + msg)
        if not ok:
            fail.append(msg)

    structure(a.path, check)

    doc = ezdxf.readfile(a.path)
    ents = list(doc.modelspace())

    print(f"{a.path}: parsed as {doc.dxfversion}")

    polys = [e for e in ents if e.dxftype() == "LWPOLYLINE"]
    profiles = [e for e in polys if e.dxf.layer == "GEAR_PROFILE"]
    envelopes = [e for e in polys if e.dxf.layer != "GEAR_PROFILE"]
    circles = [e for e in ents if e.dxftype() == "CIRCLE"]
    check(len(profiles) == 1, f"exactly one profile polyline (got {len(profiles)})")
    if not profiles:
        return 1
    pl = profiles[0]
    check(bool(pl.closed), "profile polyline is closed")

    z, m = a.teeth, a.module
    r = m * z / 2.0
    rb = r * math.cos(math.radians(a.pressure_angle))
    v = vertices(pl)
    n = len(v)

    # No span of zero length: a vertex written twice is a section emitted
    # twice, or an empty one emitted at all.
    shortest = min(math.hypot(v[(i + 1) % n][0] - v[i][0], v[(i + 1) % n][1] - v[i][1]) for i in range(n))
    check(shortest > TOL, f"no zero-length span (shortest {shortest:.3e} mm)")

    flat = list(make_path(pl).flattening(distance=TOL / 2))
    radii = [math.hypot(p.x, p.y) for p in flat]
    inner, outer = min(radii), max(radii)
    area = 0.5 * sum(p.x * q.y - q.x * p.y for p, q in zip(flat, list(flat[1:]) + [flat[0]]))

    # Every arc: read from its chord and bulge, it must be centred on the axis
    # with both ends on one radius, and that radius the tip or the root.
    arcs = []
    for i in range(n):
        if v[i][2] == 0.0:
            continue
        p, q = v[i], v[(i + 1) % n]
        c, ra, rb_end, sweep = arc_of(p, q, v[i][2])
        mid = math.atan2(p[1] + q[1], p[0] + q[0])
        arcs.append((math.hypot(*c), ra, rb_end, sweep, mid))
    off_axis = max((arc[0] for arc in arcs), default=0.0)
    check(off_axis < TOL, f"every arc is centred on the axis (worst {off_axis:.3e} mm off it)")
    uneven = max((abs(arc[1] - arc[2]) for arc in arcs), default=0.0)
    check(uneven < TOL, f"every arc has both ends on one radius (worst {uneven:.3e} mm apart)")

    if a.ring:
        ring(a, check, arcs, circles, inner, outer, area, r, rb)
    else:
        external(a, check, arcs, circles, envelopes, inner, outer, area, r, rb)

    # z-fold symmetry, where the teeth are all alike: rotating by one pitch
    # maps the outline onto itself.
    if a.angular_shift == 0.0:
        step = 2 * math.pi / z
        cs, sn = math.cos(step), math.sin(step)
        pts = [(p.x, p.y) for p in flat]
        worst = 0.0
        for x, y in pts[:: max(1, len(pts) // 200)]:
            rx, ry = x * cs - y * sn, x * sn + y * cs
            worst = max(worst, min(math.hypot(rx - px, ry - py) for px, py in pts))
        check(worst < 5e-3, f"outline is periodic in one tooth pitch (worst {worst:.2e} mm)")

    if fail:
        print(f"\n{len(fail)} check(s) failed")
        return 1
    print("\nall checks passed")
    return 0


def external(a, check, arcs, circles, envelopes, inner, outer, area, r, rb):
    g = External(a.teeth, a.module, a.shift, a.pressure_angle, a.angular_shift)
    z = a.teeth
    tooth = lambda angle: round(angle / (2.0 * math.pi / z)) % z  # noqa: E731
    concentric = a.angular_shift == 0.0

    # Each arc at the tip of the tooth it crosses, or -- on a concentric gear,
    # whose root is a circle -- at the root. An eccentric gear's root follows
    # the tool and is no arc.
    tips = [arc for arc in arcs if abs(arc[1] - g.tip(g.shift(tooth(arc[4])))) < TOL]
    roots = [arc for arc in arcs if concentric and abs(arc[1] - g.root(g.x)) < TOL]
    stray = len(arcs) - len(tips) - len(roots)
    check(stray == 0, f"every arc is at a tip or a root ({stray} are at neither)")
    per_tooth = 3 if concentric else 1
    check(
        len(arcs) == per_tooth * z,
        f"{len(arcs)} arcs, expected {per_tooth} per tooth = {per_tooth * z}",
    )

    # The arcs' angles add up to what the tooth leaves, so a dropped vertex
    # cannot leave concentric arcs that are merely short: the tip land is
    # 2(psi_b - inv alpha_a) per tooth, and the root what the two fillets
    # leave of the pitch, 2(pi/z - a_c/r).
    tip_sum = sum(arc[3] for arc in tips)
    tip_want = sum(2.0 * g.tip_half_angle(g.shift(k)) for k in range(z))
    check(abs(tip_sum - tip_want) < 1e-9, f"tip arcs sweep {tip_sum:.9f} rad, the teeth leave {tip_want:.9f}")
    if concentric:
        root_sum = sum(arc[3] for arc in roots)
        root_want = z * 2.0 * (math.pi / z - g.root_half_angle(g.x))
        check(abs(root_sum - root_want) < 1e-9, f"root arcs sweep {root_sum:.9f} rad, the spaces leave {root_want:.9f}")

    lo = min(g.root(g.shift(k)) for k in range(z)) if concentric else g.root(g.x - abs(g.dx))
    hi = max(g.tip(g.shift(k)) for k in range(z))
    check(
        inner >= lo - TOL and outer <= hi + TOL,
        f"every point within [root, tip]: {inner:.6f}..{outer:.6f} vs [{lo:.6f}, {hi:.6f}]",
    )
    check(abs(outer - hi) < TOL, f"the tip is reached ({outer:.6f} vs {hi:.6f})")
    if concentric:
        check(abs(inner - lo) < TOL, f"the root is reached ({inner:.6f} vs {lo:.6f})")

    check(area > 0, f"wound counter-clockwise (signed area {area:.3f} mm^2)")
    check(
        math.pi * lo**2 < area < math.pi * hi**2,
        f"area {area:.3f} lies between the root and tip ({math.pi * lo ** 2:.3f}..{math.pi * hi ** 2:.3f})",
    )

    # Reference entities, all on the construction layer: pitch and base
    # circles, and the tip and root -- circles on a concentric gear, closed
    # curves reaching the extreme tip and root on an eccentric one.
    got = sorted(round(c.dxf.radius, 6) for c in circles)
    if concentric:
        want = sorted(round(x, 6) for x in (g.root(g.x), rb, r, g.tip(g.x)))
        check(got == want, f"reference circles {got} match {want}")
    else:
        want = sorted(round(x, 6) for x in (rb, r))
        check(got == want, f"reference circles {got} are the pitch and base {want}")
        spans = sorted(
            (min(math.hypot(p[0], p[1]) for p in vertices(e)), max(math.hypot(p[0], p[1]) for p in vertices(e)))
            for e in envelopes
        )
        check(len(spans) == 2, f"a tip and a root envelope ({len(spans)} drawn)")
        if len(spans) == 2:
            check(abs(spans[1][1] - hi) < 1e-3, f"the tip envelope reaches the tip ({spans[1][1]:.6f} vs {hi:.6f})")
            check(abs(spans[0][0] - lo) < 1e-3, f"the root envelope reaches its lowest ({spans[0][0]:.6f} vs {lo:.6f})")
    check(
        all(c.dxf.layer == "GEAR_REFERENCE" for c in circles) and all(e.dxf.layer == "GEAR_REFERENCE" for e in envelopes),
        "reference entities on the construction layer",
    )


def ring(a, check, arcs, circles, inner, outer, area, r, rb):
    z = a.teeth
    # A ring's tooth points inward: its tip is the bore's innermost circle and
    # its root the outermost, and each arc is one of the two.
    tips = [arc for arc in arcs if abs(arc[1] - inner) < TOL]
    roots = [arc for arc in arcs if abs(arc[1] - outer) < TOL]
    stray = len(arcs) - len(tips) - len(roots)
    check(stray == 0, f"every arc is at the tip or the root ({stray} are at neither)")
    check(len(tips) == z, f"{len(tips)} tip arcs, one per tooth = {z}")
    # Two halves of a root arc per space, or none where the fillets meet.
    check(len(roots) in (0, 2 * z), f"{len(roots)} root arcs, two per space or none")
    check(inner < r < outer, f"tip {inner:.6f} inside the pitch circle {r:.6f} inside the root {outer:.6f}")

    # The bore is wound as the external outline is, counter-clockwise, and
    # encloses more than the tip circle and less than the root circle.
    check(area > 0, f"the bore is wound counter-clockwise (signed area {area:.3f} mm^2)")
    check(
        math.pi * inner**2 < area < math.pi * outer**2,
        f"area {area:.3f} lies between the tip and root ({math.pi * inner ** 2:.3f}..{math.pi * outer ** 2:.3f})",
    )

    # Pitch, base, tip and root, and the rim outside them all, as construction.
    got = sorted(c.dxf.radius for c in circles)
    check(len(got) == 5, f"five reference circles (got {len(got)})")
    for name, want in (("pitch", r), ("base", rb), ("tip", inner), ("root", outer)):
        check(any(abs(g - want) < TOL for g in got), f"a {name} circle at {want:.6f}")
    check(bool(got) and got[-1] > outer + TOL, f"the rim ({got[-1] if got else 0:.6f}) lies outside the root")
    check(all(c.dxf.layer == "GEAR_REFERENCE" for c in circles), "reference circles on the construction layer")


if __name__ == "__main__":
    sys.exit(main())
