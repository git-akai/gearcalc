#!/usr/bin/env python3
"""Worm flank curvature from the surface itself, as an independent check.

`gear-core`'s `screw.rs` gets a worm flank's curvature analytically: the flank
is an involute helicoid, which is developable, so one principal curvature is
exactly zero along the ruling and the other is `r sin a_t / cos beta_b`. The
ruling direction comes from projecting the axis onto the common tangent plane.
That is a construction in planes and angles, and it shares no code with what
this script does.

This builds the flank as a parametric surface -- a straight line under screw
motion -- and takes its first and second fundamental forms by numerical
differentiation, so the principal curvatures come out of differential geometry
with nothing about gears in the derivation. Agreement between the two is the
check.

It also answers a design question, which is why it is kept: ZA, ZN and ZI worms
are the *same* surface family and differ only in where the generating line sits
-- through the axis, in the normal plane, or tangent to the base cylinder -- so
the script can measure what choosing one over another actually costs. The answer
is in DESIGN.md 4.5.1: nothing at all except contact stress, and ZN comes out 1
to 15 % below ZI as the lead angle rises.

Numerical differentiation is fine here and would not be fine in the crate: a
step size is a tuning parameter, and DESIGN 5 does not ship those.

Usage:
    worm_flank_curvature.py             # the comparison table, and self-checks
"""

import math

# --------------------------------------------------------------- small vectors


def rz(v):
    c, s = math.cos(v), math.sin(v)
    return ((c, -s, 0.0), (s, c, 0.0), (0.0, 0.0, 1.0))


def mv(m, x):
    return tuple(sum(m[i][j] * x[j] for j in range(3)) for i in range(3))


def add(a, b):
    return tuple(x + y for x, y in zip(a, b))


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def scale(a, k):
    return tuple(x * k for x in a)


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def cross(a, b):
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def norm(a):
    return math.sqrt(dot(a, a))


def unit(a):
    return scale(a, 1.0 / norm(a))


# ------------------------------------------------------------ the surface


def surface(p0, d, p):
    """A ruled helicoid: the line p0 + u*d, screwed about z with parameter p."""

    def s(u, v):
        return add(mv(rz(v), add(p0, scale(d, u))), (0.0, 0.0, p * v))

    return s


def curvatures(s, u, v, h=1e-4):
    """Principal curvatures and the first principal direction, at (u, v).

    The step balances truncation against roundoff: second differences lose
    accuracy as `eps/h**2` and gain it as `h**2`, so the best step is around
    `eps**0.25` and the floor it leaves is a few parts in 1e8 per 1/mm. That
    floor is why the checks below compare to 1e-6 absolute rather than to
    machine precision -- and why this construction stays in a script rather
    than in the crate.
    """
    su = scale(sub(s(u + h, v), s(u - h, v)), 1 / (2 * h))
    sv = scale(sub(s(u, v + h), s(u, v - h)), 1 / (2 * h))
    suu = scale(add(sub(s(u + h, v), scale(s(u, v), 2.0)), s(u - h, v)), 1 / h**2)
    svv = scale(add(sub(s(u, v + h), scale(s(u, v), 2.0)), s(u, v - h)), 1 / h**2)
    suv = scale(
        sub(sub(s(u + h, v + h), s(u + h, v - h)), sub(s(u - h, v + h), s(u - h, v - h))),
        1 / (4 * h**2),
    )

    n = unit(cross(su, sv))
    e, f, g = dot(su, su), dot(su, sv), dot(sv, sv)
    ll, mm, nn = dot(suu, n), dot(suv, n), dot(svv, n)
    det = e * g - f * f
    gauss = (ll * nn - mm * mm) / det
    mean = (e * nn - 2 * f * mm + g * ll) / (2 * det)
    root = math.sqrt(max(mean * mean - gauss, 0.0))
    k1, k2 = mean + root, mean - root

    a11, a12 = (g * ll - f * mm) / det, (g * mm - f * nn) / det
    a21, a22 = (e * mm - f * ll) / det, (e * nn - f * mm) / det
    if abs(a12) > abs(a22 - k1):
        w = (-a12, a11 - k1)
    else:
        w = (a22 - k1, -a21)
    direction = unit(add(scale(su, w[0]), scale(sv, w[1])))
    return k1, k2, gauss, n, direction, unit(su)


def worm_flank(kind, r, gamma, alpha_n):
    """The generating line of each type, and where on it the radius is r.

    The three differ only in this choice, which is the whole point.
    """
    p = r * math.tan(gamma)
    if kind == "ZA":  # straight-sided in the axial plane; the line meets the axis
        alpha_x = math.atan(math.tan(alpha_n) / math.cos(gamma))
        d = (math.cos(alpha_x), 0.0, math.sin(alpha_x))
        return surface((0.0, 0.0, 0.0), d, p), r / math.cos(alpha_x)
    if kind == "ZN":  # straight-sided in the normal plane at the pitch cylinder
        w = (0.0, -math.sin(gamma), math.cos(gamma))
        d = add((math.cos(alpha_n), 0.0, 0.0), scale(w, math.sin(alpha_n)))
        return surface((r, 0.0, 0.0), d, p), 0.0
    if kind == "ZI":  # involute helicoid: the line is tangent to the base cylinder
        beta_1 = math.pi / 2 - gamma
        alpha_t = math.atan(math.tan(alpha_n) / math.cos(beta_1))
        beta_b = math.asin(math.sin(beta_1) * math.cos(alpha_n))
        r_b = r * math.cos(alpha_t)
        d = (0.0, math.sin(beta_b), math.cos(beta_b))
        u = math.sqrt(max(r * r - r_b * r_b, 0.0)) / math.sin(beta_b)
        return surface((r_b, 0.0, 0.0), d, p), u
    raise ValueError(kind)


def flank_at_pitch_point(kind, r, gamma, alpha_n):
    """Curvatures in the contact point's own frame: x radial, y tangential, z axis.

    The evaluation point is not on the x axis for every type, and the helicoid
    is invariant under screw motion, so rotating the result back is exact.
    """
    s, u = worm_flank(kind, r, gamma, alpha_n)
    k1, k2, gauss, n, direction, ruling = curvatures(s, u, 0.0)
    point = s(u, 0.0)
    back = rz(-math.atan2(point[1], point[0]))
    return k1, k2, gauss, mv(back, n), mv(back, direction), mv(back, ruling)


# ------------------------------------------------------- Hertz, for comparison


def r_d(x, y, z):
    """Carlson's R_D, so the comparison does not lean on the Rust being right."""
    tol = (2.220446049250313e-16) ** (1 / 6)
    carried, factor = 0.0, 1.0
    for _ in range(200):
        sx, sy, sz = math.sqrt(x), math.sqrt(y), math.sqrt(z)
        lam = sx * (sy + sz) + sy * sz
        carried += factor / (sz * (z + lam))
        factor *= 0.25
        x, y, z = 0.25 * (x + lam), 0.25 * (y + lam), 0.25 * (z + lam)
        mu = (x + y + 3 * z) / 5
        dx, dy, dz = (mu - x) / mu, (mu - y) / mu, (mu - z) / mu
        if max(abs(dx), abs(dy), abs(dz)) <= tol:
            break
    ea, eb = dx * dy, dz * dz
    ec, ed = ea - eb, ea - 6 * eb
    ee = ed + 2 * ec
    series = 1 + ed * (-(3 / 14) + (9 / 88) * ed - (9 / 52) * dz * ee) + dz * (
        ee / 6 + dz * (-(9 / 22) * ec + dz * (3 / 26) * ea)
    )
    return 3 * carried + factor * series / (mu * math.sqrt(mu))


def aspect_ratio(q):
    if q <= 0.0:
        return 0.0
    if q >= 1.0:
        return 1.0

    def g(k):
        return r_d(k * k, 0.0, 1.0) / r_d(1.0, 0.0, k * k)

    lo, hi = -1.0, 0.0
    while math.log(g(math.exp(lo))) - math.log(q) > 0.0:
        lo *= 2
        if lo < -300:
            return 0.0
    for _ in range(200):
        mid = 0.5 * (lo + hi)
        if math.log(g(math.exp(mid))) - math.log(q) > 0:
            hi = mid
        else:
            lo = mid
    return math.exp(0.5 * (lo + hi))


def peak_pressure(flat, sharp, load, e_star):
    kappa = aspect_ratio(flat / sharp)
    if kappa <= 0.0:
        return 0.0, float("inf"), 0.0
    a = (load * r_d(kappa * kappa, 0.0, 1.0) / (math.pi * e_star * flat)) ** (1 / 3)
    return 3 * load / (2 * math.pi * a * kappa * a), a, kappa * a


def relative_curvatures(b1, b2, skew):
    total = b1[0] + b1[1] + b2[0] + b2[1]
    d1, d2 = b1[0] - b1[1], b2[0] - b2[1]
    s = math.sin(skew)
    diff = math.sqrt(max((d1 + d2) ** 2 - 4 * d1 * d2 * s * s, 0.0))
    return 0.5 * (total - diff), 0.5 * (total + diff)


# ------------------------------------------------------------------- the checks


def self_checks():
    """Two things this script must get right before its answers mean anything."""
    r, load, e_star = 10.0, 100.0, 110_000.0
    p0, a, _ = peak_pressure(1.0 / r, 1.0 / r, load, e_star)
    a_ref = (3 * load * r / (4 * e_star)) ** (1 / 3)
    p_ref = 3 * load / (2 * math.pi * a_ref**2)
    assert abs(a - a_ref) < 1e-12 * a_ref, (a, a_ref)
    assert abs(p0 - p_ref) < 1e-12 * p_ref, (p0, p_ref)
    print(f"sphere on flat: a {a:.9f} mm, p0 {p0:.4f} MPa -- matches the closed form")

    # And the involute helicoid really is developable, with the curvature
    # `screw.rs` computes analytically.
    for r1, starts, module in [(3.5, 1, 1.0), (4.5, 2, 1.0), (6.0, 4, 1.0)]:
        gamma = math.asin(starts * module / (2 * r1))
        alpha_n = math.radians(20.0)
        beta_1 = math.pi / 2 - gamma
        alpha_t = math.atan(math.tan(alpha_n) / math.cos(beta_1))
        beta_b = math.asin(math.sin(beta_1) * math.cos(alpha_n))
        analytic = math.cos(beta_b) / (r1 * math.sin(alpha_t))
        k1, k2, gauss, _, direction, ruling = flank_at_pitch_point("ZI", r1, gamma, alpha_n)
        off = math.degrees(math.acos(min(1.0, abs(dot(direction, ruling)))))
        assert abs(k1 - analytic) < 1e-6 * analytic, (k1, analytic)
        assert abs(k2) < 1e-6, k2
        assert abs(off - 90.0) < 1e-2, off
        print(
            f"ZI d={2*r1:g} z1={starts}: 1/rho_n {k1:.6f} vs screw.rs {analytic:.6f} /mm, "
            f"K {gauss:+.2e}, principal direction {off:.2f} deg from the ruling"
        )


def compare(r1, starts, module, wheel_teeth, sigma_deg=90.0, alpha_n_deg=20.0,
            load=1000.0, e_star=70811.0):
    """What choosing a worm type costs, with everything else held fixed."""
    gamma = math.asin(starts * module / (2 * r1))
    alpha_n = math.radians(alpha_n_deg)
    sigma = math.radians(sigma_deg)
    beta_2 = sigma - (math.pi / 2 - gamma)

    # The wheel is an involute helical gear, so its flank is the analytic case.
    r2 = wheel_teeth * module / (2 * math.cos(beta_2))
    alpha_t2 = math.atan(math.tan(alpha_n) / math.cos(beta_2))
    beta_b2 = math.asin(math.sin(beta_2) * math.cos(alpha_n))
    k_wheel = math.cos(beta_b2) / (r2 * math.sin(alpha_t2))

    print(
        f"\nd1={2*r1:g} z1={starts} z2={wheel_teeth} m={module:g} "
        f"gamma={math.degrees(gamma):.2f} deg"
    )
    base = None
    for kind in ("ZI", "ZN", "ZA"):
        k1, k2, _, n, direction, _ = flank_at_pitch_point(kind, r1, gamma, alpha_n)
        axis_2 = (0.0, math.sin(sigma), math.cos(sigma))
        g2 = unit(sub(axis_2, scale(n, dot(axis_2, n))))
        wheel_direction = unit(cross(n, g2))
        skew = math.atan2(
            norm(cross(direction, wheel_direction)), dot(direction, wheel_direction)
        )
        flat, sharp = relative_curvatures((k1, k2), (k_wheel, 0.0), skew)
        p0, a, b = peak_pressure(flat, sharp, load, e_star)
        alpha_eff = 90 - math.degrees(math.acos(min(1.0, abs(dot(n, (1.0, 0.0, 0.0))))))
        tag = "" if base is None else f"  {100*(p0/base - 1):+6.2f} % vs ZI"
        base = p0 if base is None else base
        print(
            f"  {kind}: principal ({k1:+.6f}, {k2:+.6f})  relative ({flat:.6f}, {sharp:.6f})"
            f"  p0 {p0:7.1f} MPa  alpha_eff {alpha_eff:.2f} deg{tag}"
        )


if __name__ == "__main__":
    self_checks()
    for case in [
        (6.0, 1, 1.0, 40),
        (3.5, 1, 1.0, 40),
        (10.0, 2, 2.0, 30),
        (4.5, 2, 1.0, 41),
        (6.0, 4, 1.0, 40),
    ]:
        compare(*case)
