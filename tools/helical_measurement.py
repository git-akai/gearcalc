#!/usr/bin/env python3
"""Ball and span contact on a helical gear, from the flank surface alone.

`gear-core`'s `metrology.rs` places a ball and a pair of span anvils with
closed forms: the ball's contact rolls `D cos(beta_b) / 2` back along the
transverse involute normal from its centre, and the span anvils touch at a
transverse roll of `W cos(beta_b) / 2` either side of the base tangent point.
This script shares no code with that. It builds each flank as a surface -- the
transverse involute turned in proportion to height -- and finds the contacts by
Newton's method on the surface's own geometry:

  ball   the centre on the space's bisector at mid-face whose nearest flank
         point is D/2 away; the contact is that nearest point.
  span   two parallel planes, one tangent to each outer flank of k teeth, and
         the common normal between them placed symmetrically -- about the radial
         line through the middle of the spanned group, which a helical gear is
         symmetric under a half turn about. The planes' tilt is solved for, not
         assumed.

The gear itself is set up from the textbook relations for a standard rack
(alpha_t, r_b, beta_b, the base half-thickness psi_b). What is independent is
where the measuring bodies touch the surface, which is the question.

Checks, each with its residual:

  1. the span planes' tilt from the transverse plane is beta_b
  2. the span W equals r_b cos(beta_b) times the flanks' origin-angle difference
  3. the reported contact radii, for comparison with the crate's tests

Usage:
    helical_measurement.py
"""

import math

TAU = 2.0 * math.pi


def inv(a):
    return math.tan(a) - a


def gear(z, mn, an_deg, beta_deg, x):
    """Base radius, base helix angle and base half-thickness angle of a gear
    cut by a standard rack, from the textbook relations."""
    an = math.radians(an_deg)
    beta = math.radians(beta_deg)
    at = math.atan(math.tan(an) / math.cos(beta))
    r = z * mn / (2.0 * math.cos(beta))
    rb = r * math.cos(at)
    bb = math.atan(math.tan(beta) * math.cos(at))
    st = mn * (math.pi / 2.0 + 2.0 * x * math.tan(an)) / math.cos(beta)
    psi_b = st / (2.0 * r) + inv(at)
    return {"z": z, "rb": rb, "bb": bb, "psi_b": psi_b, "r": r}


def flank(g, theta0, w):
    """The flank whose transverse involute starts at base angle `theta0` and
    unwinds toward `w` (+1 counter-clockwise, -1 clockwise), turned about the
    axis in proportion to height by the base lead. Returns the surface and its
    two partial derivatives, all analytic."""
    rb, k = g["rb"], math.tan(g["bb"]) / g["rb"]

    def S(u, h):
        a = theta0 + w * u + k * h
        return (
            rb * (math.cos(a) + w * u * math.sin(a)),
            rb * (math.sin(a) - w * u * math.cos(a)),
            h,
        )

    def Su(u, h):
        a = theta0 + w * u + k * h
        # d/du of rb(e(a) - w u e_perp(a)), with da/du = w
        return (rb * u * math.cos(a), rb * u * math.sin(a), 0.0)

    def Sh(u, h):
        a = theta0 + w * u + k * h
        # d/dh, with da/dh = k
        return (
            rb * k * (-math.sin(a) + w * u * math.cos(a)),
            rb * k * (math.cos(a) + w * u * math.sin(a)),
            1.0,
        )

    return S, Su, Sh


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def sub(a, b):
    return tuple(x - y for x, y in zip(a, b))


def newton(f, x, tol=1e-15, it=100):
    """Newton's method on a square system, Jacobian by central differences.
    The residual is analytic, so the root is exact to rounding; the difference
    Jacobian only sets how fast it is reached."""
    n = len(x)
    for _ in range(it):
        r = f(x)
        if max(abs(v) for v in r) < tol:
            return x
        J = [[0.0] * n for _ in range(n)]
        for j in range(n):
            h = 1e-7 * max(1.0, abs(x[j]))
            xp = list(x)
            xm = list(x)
            xp[j] += h
            xm[j] -= h
            fp, fm = f(xp), f(xm)
            for i in range(n):
                J[i][j] = (fp[i] - fm[i]) / (2 * h)
        dx = solve(J, [-v for v in r])
        x = [a + b for a, b in zip(x, dx)]
    return x


def solve(A, b):
    n = len(b)
    M = [row[:] + [b[i]] for i, row in enumerate(A)]
    for c in range(n):
        p = max(range(c, n), key=lambda i: abs(M[i][c]))
        M[c], M[p] = M[p], M[c]
        for i in range(c + 1, n):
            f = M[i][c] / M[c][c]
            for j in range(c, n + 1):
                M[i][j] -= f * M[c][j]
    x = [0.0] * n
    for i in reversed(range(n)):
        x[i] = (M[i][n] - sum(M[i][j] * x[j] for j in range(i + 1, n))) / M[i][i]
    return x


def ball(g, d):
    """The contact radius of a ball of diameter `d` seated in the space after
    tooth 0, and its centre radius."""
    z, rb = g["z"], g["rb"]
    # Tooth 0's flank toward the space: its involute starts at +psi_b and
    # unwinds clockwise, so the tooth narrows as it rises.
    S, Su, Sh = flank(g, g["psi_b"], -1)
    bis = math.pi / z

    def nearest(R):
        c = (R * math.cos(bis), R * math.sin(bis), 0.0)

        def grad(v):
            u, h = v
            e = sub(S(u, h), c)
            return [dot(e, Su(u, h)), dot(e, Sh(u, h))]

        # Seeded at the transverse foot of the normal from the centre, which
        # keeps Newton on the sheet the ball touches.
        u0 = math.sqrt(max(R * R - rb * rb, 0.0)) / rb
        u, h = newton(grad, [u0, 0.0])
        p = S(u, h)
        return math.dist(p, c), u

    # Bisect the centre radius for a nearest distance of d/2.
    lo, hi = rb * 1.0000001, rb * 3.0
    for _ in range(200):
        mid = 0.5 * (lo + hi)
        dist, _ = nearest(mid)
        if dist < d / 2:
            lo = mid
        else:
            hi = mid
    R = 0.5 * (lo + hi)
    _, u = nearest(R)
    return rb * math.hypot(1.0, u), R


def span(g, k):
    """The span over `k` teeth, the contact radius on the symmetric common
    normal, and the planes' tilt from the transverse plane."""
    z, rb, psi = g["z"], g["rb"], g["psi_b"]
    # Outer flanks: tooth 0's clockwise side (origin -psi_b, unwinding
    # counter-clockwise) and tooth k-1's counter-clockwise side.
    SA, SAu, SAh = flank(g, -psi, +1)
    mid = math.pi * (k - 1) / z
    er = (math.cos(mid), math.sin(mid), 0.0)
    et = (-math.sin(mid), math.cos(mid), 0.0)
    ez = (0.0, 0.0, 1.0)

    def n_of(t):
        return tuple(math.cos(t) * a + math.sin(t) * b for a, b in zip(et, ez))

    def eqs(v):
        u, h, t = v
        n = n_of(t)
        p = SA(u, h)
        # Tangent: the plane's normal is normal to the surface. The half-turn
        # about e_r carries this flank onto the other, so the common normal is
        # the contact's component off e_r, and it must lie along n.
        b, c = dot(p, et), dot(p, ez)
        return [
            dot(n, SAu(u, h)),
            dot(n, SAh(u, h)),
            b * math.sin(t) - c * math.cos(t),
        ]

    # Seed: the transverse picture, half the span either side.
    w0 = rb * (TAU * (k - 1) / z + 2 * psi)
    u, h, t = newton(eqs, [w0 / (2 * rb), 0.0, g["bb"]])
    p = SA(u, h)
    w = 2.0 * math.hypot(dot(p, et), dot(p, ez))
    return w, rb * math.hypot(1.0, u), t


def main():
    worst = 0.0
    print("ball: z m_n alpha_n beta x D -> contact radius, centre radius")
    for z, mn, an, beta, x, d in [
        (20, 1.0, 20.0, 30.0, 0.0, 1.8),
        (20, 1.0, 20.0, 30.0, 0.0, 3.1487),
        (20, 1.0, 20.0, 0.0, 0.0, 1.8),
        (60, 1.0, 20.0, 35.0, 0.0, 1.7),
        (17, 2.0, 20.0, 15.0, 0.3, 3.5),
    ]:
        g = gear(z, mn, an, beta, x)
        c, R = ball(g, d)
        print(f"  {z} {mn} {an} {beta} {x} {d} -> {c:.9f}  {R:.9f}")
    print("span: z m_n alpha_n beta x k -> W, contact radius, tilt - beta_b")
    for z, mn, an, beta, x, k in [
        (60, 1.0, 20.0, 35.0, 0.0, 8),
        (60, 1.0, 20.0, 35.0, 0.0, 12),
        (20, 1.0, 20.0, 30.0, 0.0, 3),
        (20, 1.0, 20.0, 0.0, 0.0, 3),
        (31, 2.0, 25.0, -25.0, 0.5, 5),
    ]:
        g = gear(z, mn, an, beta, x)
        w, c, t = span(g, k)
        w_closed = g["rb"] * math.cos(g["bb"]) * (TAU * (k - 1) / z + 2 * g["psi_b"])
        worst = max(worst, abs(abs(t) - abs(g["bb"])), abs(w - w_closed) / w_closed)
        print(
            f"  {z} {mn} {an} {beta} {x} {k} -> {w:.9f}  {c:.9f}  "
            f"{abs(t) - abs(g['bb']):.1e}"
        )
    print(f"worst residual of checks 1 and 2: {worst:.1e}")
    assert worst < 1e-9


if __name__ == "__main__":
    main()
