#!/usr/bin/env python3
"""The path of contact of a crossed pair, derived from the surfaces themselves.

`gear-core`'s `screw.rs` builds a crossed pair's line of action as a *construction
in lines*: the contact normal has a fixed direction, fixed by two properties of
an involute helicoid, and the line of action is the one line with that direction
tangent to both base cylinders. That is docs/reference.md#crossed-axes, and it shares no code
with what this script does.

Here the two flanks are built as parametric surfaces from their own definition --
an involute helicoid, the surface a straight line sweeps under screw motion --
and their normals are taken by numerical differentiation. The line of action
is still *constructed* (the common tangent to both base cylinders in the one
direction the two normals share); what the surfaces decide is whether the
construction is right, and where on the line the teeth actually touch.

Checks, each answering a question the construction depends on, each asserted:

  1. Is the normal's angle to the axis really fixed?  n . a_hat = sin beta_b
  2. Is the normal line really tangent to the base cylinder, at r_b?
  3. Eight lines are tangent to both base cylinders in the two directions the
     normals allow. Exactly two pass through the pitch point: the drive and
     coast flanks' lines of action, mirror images.
  4. At a point of that line, are the two surfaces actually tangent -- do their
     own normals agree with each other and with the constructed direction?
  5. Does conjugate action fall out?  omega2/omega1 = -z1/z2, and the same at
     every point of the line, which is what makes the path a path.
  6. As the shafts straighten, does the contact ratio close on the parallel
     pair's classical value?
  7. **The zone of action, off the surfaces.** A point of the line is on the
     path where each flank reaches it: inside the tip cylinder, and on the
     helicoid's own sheet, `u >= 0` -- the sheet placed through the point has
     its normal along the line. The involute's other branch, `u < 0`, is not
     a flank, so the zone is one-sided about each base tangency: the two-sided
     band, `t -/+ rho` about it, counts involute that does not exist (on 9/37
     with tall teeth, 3.161 against 1.829).
  8. **The crate**: every row of `tools/golden/crossed_17_23_90.txt`, the
     corpus `check_golden.sh` holds to what `gear-cli crossed 17 23 90`
     prints, rebuilt at its running centre and its contact ratio measured as
     in 7.

Numerical differentiation is fine here and would not be fine in the crate: a
step size is a tuning parameter, and DESIGN 5 does not ship those.

Usage:
    crossed_path.py            # every check, with its residual; exits 1 on a failure
"""

import re
import sys
from pathlib import Path

import numpy as np

AXIS_1 = np.array([0.0, 0.0, 1.0])
RECORD = Path(__file__).resolve().parent / "golden" / "crossed_17_23_90.txt"
# The clearance a shipped crossed pair runs open by, mm: a stated input of the
# harness's pairs (every clearance "the shipped 0.02 mm"), not a figure it
# computes. The running centre is the nominal one, from the pitch radii, plus
# this.
CLEARANCE_MM = 0.02
FAILURES = []


def expect(ok, what):
    """Record a failed check; the script exits non-zero if any did."""
    if not ok:
        FAILURES.append(what)
        print(f"     FAIL: {what}")


# ------------------------------------------------------------------ the flank


def helicoid(rb, beta_b, hand=+1, phi0=0.0):
    """An involute helicoid: the transverse involute of the base circle,
    rotated in proportion to height. Returns the surface and its unit normal,
    the normal by numerical differentiation of the surface alone."""
    k = hand * np.tan(beta_b) / rb  # rotation per unit height

    def S(u, z):
        a = phi0 + k * z + u
        return np.array(
            [rb * (np.cos(a) + u * np.sin(a)), rb * (np.sin(a) - u * np.cos(a)), z]
        )

    def N(u, z):
        h = 1e-7
        du = (S(u + h, z) - S(u - h, z)) / (2 * h)
        dz = (S(u, z + h) - S(u, z - h)) / (2 * h)
        n = np.cross(du, dz)
        return n / np.linalg.norm(n)

    return S, N


def geometry(z, beta, mn, alpha_n):
    """Normal-plane inputs to the reference and base cylinders of one member."""
    mt = mn / np.cos(beta)
    at = np.arctan(np.tan(alpha_n) / np.cos(beta))
    r = z * mt / 2
    return dict(
        z=z,
        beta=beta,
        mt=mt,
        at=at,
        r=r,
        rb=r * np.cos(at),
        bb=np.arcsin(np.sin(beta) * np.cos(alpha_n)),
    )


# ---------------------------------------------------------- the constructions


def normal_direction(sigma, bb1, bb2, sign2=-1.0):
    """The one direction satisfying n . a_hat = sin beta_b for both axes."""
    nz = np.sin(bb1)
    ny = (sign2 * np.sin(bb2) - nz * np.cos(sigma)) / np.sin(sigma)
    q = 1.0 - ny * ny - nz * nz
    return None if q < 0 else np.array([np.sqrt(q), ny, nz])


def axis_2(sigma):
    return np.array([0.0, np.sin(sigma), np.cos(sigma)])


def tangent_line(sigma, a, rb1, rb2, n, s1=+1.0, s2=+1.0):
    """The line with direction `n` tangent to both base cylinders. The two signs
    pick which side of each cylinder it touches -- four lines in all."""
    c1 = np.cross(n, AXIS_1)
    c2 = np.cross(n, axis_2(sigma))
    c1, c2 = c1 / np.linalg.norm(c1), c2 / np.linalg.norm(c2)
    A = np.array([c1, c2, n])
    b = np.array([s1 * rb1, s2 * rb2 + np.dot([a, 0.0, 0.0], c2), 0.0])
    return np.linalg.solve(A, b)


def foot(P, n, axis_point, axis_dir):
    """Parameter along P + s*n of the point nearest the axis -- the tangency."""
    w = P - axis_point
    d = np.dot(n, axis_dir)
    return -(np.dot(w, n) - np.dot(w, axis_dir) * d) / (1 - d * d)


def lines_of_action(g1, g2, sigma):
    """Every common tangent line, nearest the pitch point first.

    **Two** of the eight pass through it, not one: they are the pair's two
    flanks, drive and coast, and they are mirror images. Which is which is a
    choice about direction of rotation, not a numerical accident -- an earlier
    version of this script picked whichever branch happened to have the smaller
    floating-point residual and then found the flank normals 83 degrees away
    from it.
    """
    a = g1["r"] + g2["r"]
    pitch = np.array([g1["r"], 0.0, 0.0])
    out = []
    for sign2 in (-1.0, +1.0):
        n = normal_direction(sigma, g1["bb"], g2["bb"], sign2)
        if n is None:
            continue
        for s1 in (+1.0, -1.0):
            for s2 in (+1.0, -1.0):
                P = tangent_line(sigma, a, g1["rb"], g2["rb"], n, s1, s2)
                v = pitch - P
                off = np.linalg.norm(v - np.dot(v, n) * n)
                out.append(dict(off=off, n=n, P=P, sign2=sign2, s1=s1, s2=s2))
    return sorted(out, key=lambda r: r["off"])


def line_of_action(g1, g2, sigma):
    """One flank's line of action -- the two are mirror images, so anything
    measured as a length is the same on either."""
    return lines_of_action(g1, g2, sigma)[0]


# --------------------------------------------------------------- the checks


def check_normal_law():
    """1 and 2: the normal's angle to the axis, and its tangency to r_b."""
    print("1/2. The two properties the whole construction rests on")
    print("     (from the surface's own parameterisation, not from a formula)")
    worst_axial, worst_tangent = 0.0, 0.0
    for beta_deg in (0.0, 15.0, 30.0, 45.0):
        for rb in (8.0, 20.0):
            bb = np.radians(beta_deg)
            S, N = helicoid(rb, bb)
            axial, dist = [], []
            for u in (0.05, 0.2, 0.5, 0.9):
                for z in (-5.0, 0.0, 7.0):
                    p, n = S(u, z), N(u, z)
                    axial.append(n[2])
                    c = np.cross(AXIS_1, n)
                    dist.append(abs(np.dot(p, c)) / np.linalg.norm(c))
            axial, dist = np.array(axial), np.array(dist)
            e_ax = max(abs(axial - np.sin(bb)))
            e_tg = max(abs(dist - rb))
            worst_axial = max(worst_axial, e_ax)
            worst_tangent = max(worst_tangent, e_tg)
            print(
                f"     beta_b {beta_deg:5.1f} deg  r_b {rb:5.1f}: "
                f"n.a_hat - sin beta_b  {e_ax:.1e}   "
                f"axis-to-normal - r_b  {e_tg:.1e} mm"
            )
    print(
        f"     worst: {worst_axial:.1e} and {worst_tangent:.1e} mm "
        "-- the direction is fixed, and the line is tangent\n"
    )
    # The central difference's own error at h = 1e-7 is about 1e-9.
    expect(worst_axial < 1e-7, f"normal angle to the axis off by {worst_axial:.1e}")
    expect(worst_tangent < 1e-6, f"normal line off the base cylinder by {worst_tangent:.1e} mm")


def check_branch(g1, g2, sigma):
    """3: eight common tangent lines, two of them the pair's two flanks."""
    print("3. Eight lines are tangent to both base cylinders. Two are the path.")
    for r in lines_of_action(g1, g2, sigma):
        mark = "  <- a flank's line of action" if r["off"] < 1e-9 else ""
        print(
            f"     sign2 {r['sign2']:+.0f}  s1 {r['s1']:+.0f}  s2 {r['s2']:+.0f}: "
            f"pitch point is {r['off']:9.5f} mm off{mark}"
        )
    through = [r for r in lines_of_action(g1, g2, sigma) if r["off"] < 1e-9]
    others = [r["off"] for r in lines_of_action(g1, g2, sigma) if r["off"] >= 1e-9]
    expect(len(through) == 2, f"{len(through)} lines through the pitch point, not 2")
    expect(min(others) > 1e-3, f"a third line passes within {min(others):.1e} mm")
    print("     -- the two that pass are mirror images: drive flank and coast\n")


def place_on(rb, beta_b, target, hand=+1):
    """Phase and parameters putting this surface through `target`, in its own
    frame. All three are closed form: the involute's polar angle is
    `phi0 + k z + u - atan(u)`, which inverts directly. (Bisecting on
    `atan2` instead is where a first attempt went wrong -- the wrap gives two
    sign changes, and the bisection converges on the wrong one.)"""
    r = np.hypot(target[0], target[1])
    u = np.sqrt((r / rb) ** 2 - 1.0)
    z = target[2]
    k = hand * np.tan(beta_b) / rb
    ang = np.arctan2(target[1], target[0])
    return u, z, ang - k * z - u + np.arctan(u)


def check_tangency(g1, g2, sigma):
    """4: put a tooth of each gear through the pitch point and ask each surface
    for its own normal. If both agree with the line, the surfaces touch there.

    Nothing is assumed about hand or branch: every combination is built and the
    agreement is the measurement."""
    print("4. Are the surfaces actually tangent on that line?")
    a = g1["r"] + g2["r"]
    pitch = np.array([g1["r"], 0.0, 0.0])
    c, s = np.cos(sigma), np.sin(sigma)
    R = np.array([[1, 0, 0], [0, c, -s], [0, s, c]])
    local = R.T @ (pitch - np.array([a, 0.0, 0.0]))

    # A tooth has two flanks and the parameterisation above builds one of them;
    # the other is its mirror image in a plane through the axis. In a mesh the
    # driving flank of one member meets the *facing* flank of the other, so the
    # mirror is not optional -- without it gear 2 comes out exactly 2 alpha_n
    # (40 deg here) away from the constructed normal, which is the angle between
    # a tooth's own two flanks and a useful way to recognise the mistake.
    MIRROR = np.diag([1.0, -1.0, 1.0])

    def flank_normal(g, target, hand, mirror, rotate=None):
        M = MIRROR if mirror else np.eye(3)
        u, z, phi = place_on(g["rb"], g["bb"], M @ target, hand)
        S, N = helicoid(g["rb"], g["bb"], hand, phi)
        reach = np.linalg.norm(M @ S(u, z) - target)
        n = M @ N(u, z)
        return (n if rotate is None else rotate @ n), reach

    def angle(p, q):
        return np.degrees(np.arccos(min(1.0, abs(np.dot(p, q)))))

    sides = [(h, m) for h in (+1, -1) for m in (False, True)]
    flanks = [(1, h, m, *flank_normal(g1, pitch, h, m)) for h, m in sides]
    flanks += [(2, h, m, *flank_normal(g2, local, h, m, R)) for h, m in sides]
    print(f"     every flank reaches the point to {max(f[4] for f in flanks):.1e} mm")

    for r in lines_of_action(g1, g2, sigma):
        if r["off"] > 1e-9:
            continue
        tag = f"branch sign2 {r['sign2']:+.0f} s1 {r['s1']:+.0f} s2 {r['s2']:+.0f}"
        hits = [(g, h, m, angle(r["n"], n)) for g, h, m, n, _ in flanks]
        best1 = min((x for x in hits if x[0] == 1), key=lambda x: x[3])
        best2 = min((x for x in hits if x[0] == 2), key=lambda x: x[3])
        side = lambda x: f"hand {x[1]:+d}, {'far' if x[2] else 'near'} flank"
        print(
            f"     {tag}:\n"
            f"       gear 1 ({side(best1)}) is {best1[3]:.2e} deg off the normal\n"
            f"       gear 2 ({side(best2)}) is {best2[3]:.2e} deg off the normal"
        )
        expect(max(best1[3], best2[3]) < 1e-5, f"{tag}: a flank is off the normal")
    print("     -- both surfaces share the constructed normal, so they touch\n")

    # the roll lengths, measured from the pitch point along the line
    loa = line_of_action(g1, g2, sigma)
    n, P = loa["n"], loa["P"]
    t1 = foot(P, n, np.zeros(3), AXIS_1)
    t2 = foot(P, n, np.array([a, 0.0, 0.0]), axis_2(sigma))
    sp = np.dot(pitch - P, n)
    print("     roll length to the pitch point vs. the classical r sin a_t / cos beta_b:")
    for g, t in ((g1, t1), (g2, t2)):
        rho = abs(sp - t)
        classical = g["r"] * np.sin(g["at"]) / np.cos(g["bb"])
        print(f"       {rho:.9f}  vs  {classical:.9f}   ({abs(rho-classical):.1e})")
        expect(abs(rho - classical) < 1e-9, f"roll length {rho} against {classical}")
    print()


def check_conjugate(g1, g2, sigma):
    """5: the speed ratio from v1.n = v2.n, at points along the line."""
    print("5. Conjugate action, from the contact condition alone")
    loa = line_of_action(g1, g2, sigma)
    n, P = loa["n"], loa["P"]
    a = g1["r"] + g2["r"]
    a2 = axis_2(sigma)
    exact = -g1["z"] / g2["z"]
    worst = 0.0
    for s in (-6.0, -3.0, 0.0, 3.0, 6.0):
        X = P + (np.dot(np.array([g1["r"], 0.0, 0.0]) - P, n) + s) * n
        num = np.dot(np.cross(AXIS_1, X), n)
        den = np.dot(np.cross(a2, X - np.array([a, 0.0, 0.0])), n)
        ratio = -num / den
        worst = max(worst, abs(ratio - exact))
        print(f"     s {s:+5.1f} mm along the line: omega2/omega1 = {ratio:+.12f}")
    print(f"     -z1/z2 = {exact:+.12f}; worst deviation {worst:.1e}")
    expect(worst < 1e-9, f"speed ratio off -z1/z2 by {worst:.1e} along the line")
    print("     -- constant along the line, which is what makes it a path\n")


def contact_ratio(g1, g2, sigma, mn, alpha_n, addendum=1.0):
    """Zone of action over normal base pitch, from the construction."""
    loa = line_of_action(g1, g2, sigma)
    n, P = loa["n"], loa["P"]
    a = g1["r"] + g2["r"]
    t1 = foot(P, n, np.zeros(3), AXIS_1)
    t2 = foot(P, n, np.array([a, 0.0, 0.0]), axis_2(sigma))
    band = []
    for g, t in ((g1, t1), (g2, t2)):
        ra = g["r"] + addendum * mn
        rho = np.sqrt(max(ra**2 - g["rb"] ** 2, 0.0)) / np.cos(g["bb"])
        band.append((t - rho, t + rho))
    lo = max(band[0][0], band[1][0])
    hi = min(band[0][1], band[1][1])
    if hi <= lo:
        return None
    return (hi - lo) / (np.pi * mn * np.cos(alpha_n))


def surface_zone(g1, g2, sigma, mn, alpha_n, addendum=1.0, a=None):
    """The contact ratio off the surfaces: where on the line of action each
    flank's own sheet reaches, inside its tip cylinder.

    Each gear's flank is oriented (hand, and which of a tooth's two faces) by
    the surface whose own normal lies along the line at the operating pitch
    point. A point X of the line then lies on that flank's sheet where its
    roll parameter is `u >= 0`: the helicoid `S(u, z)` unwinds from its base
    point `F` along `-e_theta(F)` (`S - F = r_b u (sin a, -cos a, .)`), so
    `u` at X is the transverse part of `X - F` along that direction, over
    `r_b`, with F the line's tangency to the base cylinder. Past F the sheet
    that reaches X is the involute's other branch, which is not a flank. The
    sign is read rather than a normal compared, because both branches' normals
    close on the line as X nears F and a threshold on the angle there would
    set the end of the zone.

    `a` is the running centre distance, the nominal one if not given."""
    a_nom = g1["r"] + g2["r"]
    a = a_nom if a is None else a
    pitch = np.array([a * g1["r"] / a_nom, 0.0, 0.0])
    lines = []
    for sign2 in (-1.0, +1.0):
        n = normal_direction(sigma, g1["bb"], g2["bb"], sign2)
        if n is None:
            continue
        for s1 in (+1.0, -1.0):
            for s2 in (+1.0, -1.0):
                P = tangent_line(sigma, a, g1["rb"], g2["rb"], n, s1, s2)
                v = pitch - P
                lines.append((np.linalg.norm(v - np.dot(v, n) * n), n, P))
    _, n, P = min(lines, key=lambda x: x[0])
    c, sn = np.cos(sigma), np.sin(sigma)
    R = np.array([[1, 0, 0], [0, c, -sn], [0, sn, c]])
    MIRROR = np.diag([1.0, -1.0, 1.0])
    frames = [
        (g1, lambda X: X, lambda v: v),
        (g2, lambda X: R.T @ (X - np.array([a, 0.0, 0.0])), lambda v: R.T @ v),
    ]

    def misfit(g, hand, mirror, X_local, n_local):
        """Angle in degrees between the flank's normal at X and the line, or
        None where X is below the base cylinder."""
        M = MIRROR if mirror else np.eye(3)
        target = M @ X_local
        if np.hypot(target[0], target[1]) < g["rb"]:
            return None
        u, z, phi = place_on(g["rb"], g["bb"], target, hand)
        _, N = helicoid(g["rb"], g["bb"], hand, phi)
        return np.degrees(np.arccos(min(1.0, abs(np.dot(M @ N(u, z), n_local)))))

    s0 = np.dot(pitch - P, n)
    orient = []
    for g, to_local, dir_local in frames:
        X0 = to_local(P + s0 * n)
        best = min(
            ((h, m) for h in (+1, -1) for m in (False, True)),
            key=lambda hm: misfit(g, hm[0], hm[1], X0, dir_local(n)),
        )
        orient.append(best)

    axes = [(np.zeros(3), AXIS_1), (np.array([a, 0.0, 0.0]), axis_2(sigma))]
    feet = [to_local(P + foot(P, n, *axis) * n) for (_, to_local, _), axis in zip(frames, axes)]

    def roll(g, mirror, XL, FL):
        """The signed roll parameter at X of the flank whose base point is F."""
        M = MIRROR if mirror else np.eye(3)
        x, f = M @ XL, M @ FL
        a_f = np.arctan2(f[1], f[0])
        return np.dot((x - f)[:2], [np.sin(a_f), -np.cos(a_f)]) / g["rb"]

    def on_both(sv):
        X = P + sv * n
        for (g, to_local, _), (_, mirror), FL in zip(frames, orient, feet):
            XL = to_local(X)
            if np.hypot(XL[0], XL[1]) > g["r"] + addendum * mn:
                return False
            if roll(g, mirror, XL, FL) < 0:
                return False
        return True

    # Sample the line about the pitch point, then bisect each end.
    reach = 4 * addendum * mn / np.cos(alpha_n) + max(g1["rb"], g2["rb"])
    grid = np.linspace(s0 - reach, s0 + reach, 4001)
    inside = [on_both(sv) for sv in grid]
    if not any(inside):
        return None
    first = inside.index(True)
    last = len(inside) - 1 - inside[::-1].index(True)
    expect(all(inside[first : last + 1]), "the zone is not one interval")

    def edge(out, into):
        for _ in range(80):
            mid = 0.5 * (out + into)
            out, into = (out, mid) if on_both(mid) else (mid, into)
        return into

    lo = edge(grid[first - 1], grid[first]) if first > 0 else grid[0]
    hi = edge(grid[last + 1], grid[last]) if last + 1 < len(grid) else grid[-1]
    return (hi - lo) / (np.pi * mn * np.cos(alpha_n))


def check_zone(mn, alpha_n):
    """7: the zone off the surfaces, against the two-sided band."""
    print("7. The zone of action, from where each flank's own sheet reaches")
    for label, z1, z2, b1, sig, add in (
        ("17/23, 45/45 deg, shafts 90 deg, addendum 1", 17, 23, 45.0, 90.0, 1.0),
        ("9/37, 20/70 deg, shafts 90 deg, addendum 2", 9, 37, 20.0, 90.0, 2.0),
    ):
        sigma = np.radians(sig)
        g1 = geometry(z1, np.radians(b1), mn, alpha_n)
        g2 = geometry(z2, sigma - np.radians(b1), mn, alpha_n)
        two = contact_ratio(g1, g2, sigma, mn, alpha_n, add)
        one = surface_zone(g1, g2, sigma, mn, alpha_n, add)
        print(f"     {label}: off the surfaces {one:.9f}, two-sided band {two:.9f}")
        if z1 == 17:
            # The figure the crate's construction is held to (screw.rs).
            expect(abs(one - 1.777921669562) < 1e-9, f"17/23 zone {one}, not 1.777921669562")
        else:
            # Tall teeth reach past a base tangency: the band counts the
            # involute's other branch there, and the sheet does not. The
            # figure screw.rs's `the_zone_stops_at_the_base_tangency_as_the_surfaces_say`
            # holds the construction to.
            expect(abs(one - 1.829147466) < 1e-9, f"9/37 zone {one}, not 1.829147466")
            expect(one < two - 1.0, f"9/37: the sheet {one} and the band {two} should differ")
    print()


def check_crate(mn, alpha_n):
    """8: the harness's crossed pair, row by row, at its running centre."""
    print("8. The crate: `gear-cli crossed 17 23 90`, as the corpus recorded it")
    text = RECORD.read_text()
    head = re.search(r"z (\d+)/(\d+)\s+shaft angle (\S+) deg\s+module (\S+)\s+alpha (\S+) deg", text)
    z1, z2 = int(head.group(1)), int(head.group(2))
    sigma = np.radians(float(head.group(3)))
    mn, alpha_n = float(head.group(4)), np.radians(float(head.group(5)))
    rows = re.findall(
        r"^\s+(\d+\.\d)\s+(\d+\.\d)\s+\S+\s+\S+\s+(\S+)\s.*?\s(\d\.\d{9})$", text, re.M
    )
    # The running centre is derived, and the harness's shift table held to it.
    even = float(re.search(r"at the even split \((\S+)/", text).group(1))
    printed = float(re.search(r"least shift that clears undercut\s+\S+\s+\S+\s+(\S+)", text).group(1))
    g_even = [geometry(z, np.radians(b), mn, alpha_n) for z, b in ((z1, even), (z2, np.degrees(sigma) - even))]
    derived = g_even[0]["r"] + g_even[1]["r"] + CLEARANCE_MM
    expect(f"{derived:.4f}" == f"{printed:.4f}", f"running centre {printed}, derived {derived:.4f}")
    clearance = CLEARANCE_MM
    print(f"     running {clearance} mm open: {derived:.4f} mm at the even split, as printed")
    worst = 0.0
    for b1, b2, a_nom, eps in rows:
        g1 = geometry(z1, np.radians(float(b1)), mn, alpha_n)
        g2 = geometry(z2, np.radians(float(b2)), mn, alpha_n)
        # The nominal centre from the pitch radii, held to the printed one;
        # the printed one is only to its fourth decimal.
        expect(f"{g1['r'] + g2['r']:.4f}" == a_nom, f"beta1 {b1}: nominal centre {a_nom} is not this pair's")
        one = surface_zone(g1, g2, sigma, mn, alpha_n, 1.0, g1["r"] + g2["r"] + clearance)
        worst = max(worst, abs(one - float(eps)))
        print(f"     beta1 {b1:>5}: crate {eps}, off the surfaces {one:.9f}  ({abs(one - float(eps)):.1e})")
    expect(len(rows) >= 9, f"read {len(rows)} rows of the record")
    expect(worst < 1e-9, f"the crate's contact ratio is {worst:.1e} off the surfaces'")
    print()


def parallel_transverse_ratio(g1, g2, mn, alpha_n, addendum=1.0):
    """The textbook parallel-axis formula, for the gate to converge on."""
    at = g1["at"]
    a = g1["r"] + g2["r"]
    path = (
        np.sqrt((g1["r"] + addendum * mn) ** 2 - g1["rb"] ** 2)
        + np.sqrt((g2["r"] + addendum * mn) ** 2 - g2["rb"] ** 2)
        - a * np.sin(at)
    )
    return path / (np.pi * g1["mt"] * np.cos(at))


def check_limit(mn, alpha_n, z1, z2, b_add):
    """The gate the crate's own test mirrors: crossing goes to parallel.

    The target is not the bare transverse contact ratio. The zone here is a
    length along a line inclined at beta_b to the transverse plane, so it is
    `L_t / cos beta_b`, and it is divided by the *normal* base pitch, which is
    `p_bt cos beta_b`. Both factors are real and both point the same way, so the
    limit is `epsilon_alpha / cos^2 beta_b`. Writing down the transverse value
    instead is the obvious mistake and misses by 11 %.
    """
    print("6. The limit -- as the shafts straighten, does it become the pair we know?")
    g1 = geometry(z1, b_add, mn, alpha_n)
    g2 = geometry(z2, -b_add, mn, alpha_n)
    eps_t = parallel_transverse_ratio(g1, g2, mn, alpha_n)
    bb = np.arcsin(np.sin(b_add) * np.cos(alpha_n))
    target = eps_t / np.cos(bb) ** 2
    print(
        f"     classical parallel pair at Sigma = 0: epsilon_alpha {eps_t:.9f} "
        f"transverse,\n     which on this line and this pitch is "
        f"{eps_t:.6f} / cos^2 {np.degrees(bb):.4f} deg = {target:.9f}\n"
    )
    print("     Sigma (deg)      crossed epsilon        error       pitch offset")
    for sig_deg in (30.0, 10.0, 2.0, 0.5, 0.1, 0.01, 0.001):
        sigma = np.radians(sig_deg)
        h1 = geometry(z1, sigma / 2 + b_add, mn, alpha_n)
        h2 = geometry(z2, sigma / 2 - b_add, mn, alpha_n)
        eps = contact_ratio(h1, h2, sigma, mn, alpha_n)
        off = line_of_action(h1, h2, sigma)["off"]
        if eps is None:
            print(f"     {sig_deg:9.3f}          (no contact)")
            continue
        print(
            f"     {sig_deg:9.3f}      {eps:.9f}      {abs(eps-target):.2e}      {off:.1e}"
        )
    print("     -- monotone, and closing on the parallel value\n")


if __name__ == "__main__":
    mn, alpha_n = 1.0, np.radians(20.0)

    check_normal_law()

    sigma = np.radians(90.0)
    g1 = geometry(17, np.radians(45.0), mn, alpha_n)
    g2 = geometry(23, np.radians(45.0), mn, alpha_n)
    print(
        f"A 17/23 pair, m_n {mn}, alpha_n 20 deg, both helices 45 deg, shafts at 90 deg:\n"
        f"     r1 {g1['r']:.4f}  r_b1 {g1['rb']:.4f}  beta_b1 {np.degrees(g1['bb']):.4f} deg\n"
        f"     r2 {g2['r']:.4f}  r_b2 {g2['rb']:.4f}  beta_b2 {np.degrees(g2['bb']):.4f} deg\n"
        f"     centre distance {g1['r'] + g2['r']:.4f}\n"
    )
    eps = contact_ratio(g1, g2, sigma, mn, alpha_n)
    print(
        f"     contact ratio over the normal base pitch, tips at r + m_n: {eps:.6f}\n"
        f"     (`gear-cli crossed 17 23 90`, beta1 = 45, prints the crate's own)\n"
    )
    check_branch(g1, g2, sigma)
    check_tangency(g1, g2, sigma)
    check_conjugate(g1, g2, sigma)
    check_limit(mn, alpha_n, 17, 43, np.radians(20.0))
    check_zone(mn, alpha_n)
    check_crate(mn, alpha_n)
    if FAILURES:
        print(f"{len(FAILURES)} check(s) failed")
        sys.exit(1)
    print("every check holds")
