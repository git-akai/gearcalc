#!/usr/bin/env python3
"""A stepped flat bar's stress concentration in bending by a solver that has
no boundary integral in it: the second, independent reference that
`tools/fillet_bem.py`'s shoulder canary is held to.

    python3 tools/shoulder_trefftz.py          # solve the four bars, and hold them to RECORDED

# The solver

Plane elasticity by Kolosov–Muskhelishvili potentials `φ(z)`, `ψ(z)`, each a
sum of basis functions holomorphic in the body, so every one satisfies the
equations of equilibrium and compatibility exactly and only the boundary
conditions are fitted — by least squares on points sampled on the exact
boundary, weighted by the arc length each point stands for:

- a polynomial in `z` of degree `DEG`, orthogonalised on the samples by
  Arnoldi (Vandermonde with Arnoldi, Brubeck, Nakatsukasa and Trefethen 2021);
- simple poles outside the body, clustered exponentially toward every point
  where the boundary is not analytic — each corner and each place a straight
  edge meets a round — along the exterior bisector (the "lightning"
  placement of Gopal and Trefethen 2019), with samples clustered to match.

`σ_xx + σ_yy = 4 Re φ′`, `σ_yy − σ_xx + 2iσ_xy = 2(z̄ φ″ + ψ′)`,
`2μ(u + iv) = κφ − z φ′̄ − ψ̄` (plane strain, `κ = 3 − 4ν`). The system is
solved by a QR factorisation of the weighted, column-scaled matrix and a
truncated singular value decomposition of its triangle.

It is a port of the notch research's prototype (`trefftz-proto/tz.py`, the
rack; `t_shoulder*.py`, these bars), written again here with numpy alone.

# The bar

A bar of depth `D` stepping down to `d` through two quarter-round fillets of
radius `r` and square shoulder faces; its narrow end, `4d` from the step,
carries `σ_xx = σ_nom · y/(d/2)` with `σ_nom = 1`; its wide end, `4D` beyond
the step, is clamped. `K_t` is the largest hoop stress on the fillet.
"""

import cmath
import math
import sys

try:
    import numpy as np
except ImportError:  # `RECORDED` is read without numpy; solving needs it
    np = None

# The four bars `(D/d, r/d)` and their `K_t`, as the prototype's final degree
# gave them, which this script reproduces (`main`).
RECORDED = [(1.5, 0.05, 2.38677), (1.5, 0.1, 1.89742), (2.0, 0.1, 1.94162), (2.0, 0.2, 1.57280)]
# Two resolutions: the second is the answer and the first says how far from
# converged it is. Polynomial degree, poles per vertex, uniform samples per
# piece.
LEVELS = [(80, 32, 120), (100, 40, 150)]
NU = 0.3
# The lightning placement: poles out to `L_POLE` from their vertex, spaced
# as `exp(−σ(√N − √n))`; samples clustered the same way, `M_CLUST` a side.
L_POLE, SIGMA, M_CLUST = 0.5, 4.0, 32
# Singular values below this fraction of the largest are dropped: the basis is
# a frame (rigid motions among it are exactly redundant), not a basis.
RCOND = 1e-14


def bar(D, d, r, ln=4.0, lw=4.0):
    """The boundary, counter-clockwise, as pieces `(z(t), z′(t), bc, traction)`
    for `t ∈ [0, 1]`, and the top fillet's centre."""
    ln, lw = ln * d, lw * D
    c = 2.0 / d
    yt, yb = d / 2 + r, -d / 2 - r
    P = []

    def seg(z0, z1, bc="t", tval=None):
        P.append((lambda t: z0 + (z1 - z0) * t, lambda t: (z1 - z0) + 0 * t, bc, tval))

    def arc(cen, th0, th1):
        P.append((lambda t: cen + r * np.exp(1j * (th0 + (th1 - th0) * t)),
                  lambda t: 1j * (th1 - th0) * r * np.exp(1j * (th0 + (th1 - th0) * t)), "t", None))

    face = (D - d) / 2 > r
    seg(complex(-ln, -d / 2), complex(-r, -d / 2))
    arc(complex(-r, yb), math.pi / 2, 0.0)
    if face:
        seg(complex(0, yb), complex(0, -D / 2))
    seg(complex(0, -D / 2), complex(lw, -D / 2))
    seg(complex(lw, -D / 2), complex(lw, D / 2), bc="u")
    seg(complex(lw, D / 2), complex(0, D / 2))
    if face:
        seg(complex(0, D / 2), complex(0, yt))
    arc(complex(-r, yt), 0.0, -math.pi / 2)
    seg(complex(-r, d / 2), complex(-ln, d / 2))
    seg(complex(-ln, d / 2), complex(-ln, -d / 2), tval=lambda z: -c * z.imag + 0j)
    return P, complex(-r, yt)


def vertices(P):
    """Each piece's start: the point, the exterior bisector, and whether the
    boundary turns there."""
    out = []
    for k in range(len(P)):
        w = complex(P[k][0](0.0))
        ta, tb = complex(P[k - 1][1](1.0)), complex(P[k][1](0.0))
        ext = -1j * ta / abs(ta) - 1j * tb / abs(tb)
        out.append((w, ext / abs(ext)))
    return out


def length(piece):
    t = np.linspace(0, 1, 201)
    return float(np.mean(np.abs(piece[1](t))))


def cluster(n):
    """Distances, as a fraction, of `n` points clustered toward an end."""
    return np.exp(-SIGMA * (math.sqrt(n) - np.sqrt(np.arange(1, n + 1))))


def samples(P, verts, npole, m_unif):
    """Boundary points, outward normals, weights and piece indices: uniform,
    clustered toward both ends of every piece, and at a third, one and three
    times each pole's distance from its vertex."""
    extra = {k: [] for k in range(len(P))}
    dd = L_POLE * cluster(npole)
    dd = np.concatenate([dd / 3, dd, 3 * dd])
    for k in range(len(verts)):
        for kp, end in (((k - 1) % len(P), 1), (k, 0)):
            t = dd / length(P[kp])
            t = t[t < 0.5]
            extra[kp].append(1 - t if end else t)
    Z, N, W, I = [], [], [], []
    for k, (z, dz, _, _) in enumerate(P):
        d = 0.5 * cluster(M_CLUST)
        t = np.unique(np.concatenate([d, 1 - d, np.linspace(0, 1, m_unif + 2)[1:-1], *extra[k]]))
        t = t[(t > 0) & (t < 1)]
        sp = np.abs(dz(t))
        tb = np.concatenate([[0.0], 0.5 * (t[1:] + t[:-1]), [1.0]])
        Z.append(z(t))
        N.append(-1j * dz(t) / sp)
        W.append(sp * np.diff(tb))
        I.append(np.full(len(t), k))
    return np.concatenate(Z), np.concatenate(N), np.concatenate(W), np.concatenate(I)


def inside(poly, z):
    """Whether each point of `z` lies inside the closed polygon `poly`
    (crossing number)."""
    x, y = z.real[:, None], z.imag[:, None]
    a, b = poly[None, :-1], poly[None, 1:]
    cross = ((a.imag > y) != (b.imag > y)) & (
        x < (b.real - a.real) * (y - a.imag) / (b.imag - a.imag + 1e-300) + a.real)
    return np.count_nonzero(cross, axis=1) % 2 == 1


class Arnoldi:
    """Polynomials orthonormal on the sample points, and their first two
    derivatives anywhere, by the recurrence the orthogonalisation found."""

    def __init__(self, Z, n):
        self.c = 0.5 * (Z.real.min() + Z.real.max()) + 0.5j * (Z.imag.min() + Z.imag.max())
        self.s = 0.5 * max(np.ptp(Z.real), np.ptp(Z.imag))
        z = (Z - self.c) / self.s
        M = len(z)
        Q = np.zeros((M, n + 1), complex)
        H = np.zeros((n + 1, n), complex)
        Q[:, 0] = 1
        for k in range(n):
            q = z * Q[:, k]
            for _ in range(2):  # orthogonalised twice, for rounding
                h = Q[:, :k + 1].conj().T @ q / M
                H[:k + 1, k] += h
                q = q - Q[:, :k + 1] @ h
            H[k + 1, k] = np.linalg.norm(q) / math.sqrt(M)
            Q[:, k + 1] = q / H[k + 1, k]
        self.H, self.n = H, n

    def __call__(self, Z):
        z = (Z - self.c) / self.s
        H, n = self.H, self.n
        P = np.zeros((len(z), n + 1), complex)
        D1, D2 = np.zeros_like(P), np.zeros_like(P)
        P[:, 0] = 1
        for k in range(n):
            h = H[:k + 1, k]
            P[:, k + 1] = (z * P[:, k] - P[:, :k + 1] @ h) / H[k + 1, k]
            D1[:, k + 1] = (P[:, k] + z * D1[:, k] - D1[:, :k + 1] @ h) / H[k + 1, k]
            D2[:, k + 1] = (2 * D1[:, k] + z * D2[:, k] - D2[:, :k + 1] @ h) / H[k + 1, k]
        return P, D1 / self.s, D2 / self.s ** 2


def basis(Z, poly, poles):
    """`(f, f′, f″)` of every complex basis function at `Z`, as columns."""
    P, D1, D2 = poly(Z)
    d = Z[:, None] - poles[None, :]
    return (np.hstack([P, 1 / d]), np.hstack([D1, -1 / d ** 2]), np.hstack([D2, 2 / d ** 3]))


def fields(Z, f, f1, f2):
    """Stress and `2μ(u + iv)` of each real column: `φ = f`, `φ = if`,
    `ψ = f`, `ψ = if` for every complex `f`."""
    kappa = 3 - 4 * NU
    cols = []
    zb = np.conj(Z)[:, None]
    zz = Z[:, None]
    zero = np.zeros_like(f)
    for c in (1.0, 1j):
        # φ = c f
        S1 = 4 * np.real(c * f1)
        S2 = 2 * zb * c * f2
        cols.append((S1, S2, kappa * c * f - zz * np.conj(c * f1)))
        # ψ = c f
        S2 = 2 * c * f1
        cols.append((np.zeros(f.shape), S2, -np.conj(c * f) + zero))
    return cols


def rows(Z, N, bcu, S1, S2, U):
    """Boundary-condition rows: traction on free and loaded sides, `2μu` on
    the clamped one."""
    sxx = 0.5 * (S1 - np.real(S2))
    syy = 0.5 * (S1 + np.real(S2))
    sxy = 0.5 * np.imag(S2)
    nx, ny = N.real[:, None], N.imag[:, None]
    tx = sxx * nx + sxy * ny
    ty = sxy * nx + syy * ny
    b = bcu[:, None]
    return np.vstack([np.where(b, U.real, tx), np.where(b, U.imag, ty)])


def solve(D, d, r, deg, npole, m_unif):
    P, centre = bar(D, d, r)
    verts = vertices(P)
    poles = np.concatenate([w + L_POLE * e * cluster(npole) for w, e in verts])
    outline = np.concatenate([P[k][0](np.linspace(0, 1, 400)) for k in range(len(P))])
    poles = poles[~inside(np.append(outline, outline[0]), poles)]
    Z, N, W, I = samples(P, verts, npole, m_unif)
    bcu = np.array([P[k][2] == "u" for k in I])
    poly = Arnoldi(Z, deg)
    A = np.hstack([rows(Z, N, bcu, *c) for c in fields(Z, *basis(Z, poly, poles))])
    rhs = np.zeros(len(Z), complex)
    for k, piece in enumerate(P):
        if piece[3] is not None:
            m = I == k
            rhs[m] = piece[3](Z[m])
    b = np.concatenate([rhs.real, rhs.imag])
    w = np.sqrt(np.concatenate([W, W]))
    A *= w[:, None]
    b *= w
    scale = np.linalg.norm(A, axis=0)
    scale[scale == 0] = 1
    A /= scale
    R = np.linalg.qr(np.column_stack([A, b]), mode="r")
    n = A.shape[1]
    U, s, Vt = np.linalg.svd(R[:n, :n])
    keep = s > RCOND * s[0]
    x = Vt[keep].T @ ((U.T @ R[:n, n])[keep] / s[keep]) / scale
    residual = math.sqrt(R[n, n] ** 2 + float(np.sum((U.T @ R[:n, n])[~keep] ** 2)))
    # The hoop stress along the top fillet, whose centre is outside the body.
    th = np.linspace(-math.pi / 2, 0.0, 4001)
    Zf = centre + r * np.exp(1j * th)
    T = 1j * -np.exp(1j * th)
    cols = fields(Zf, *basis(Zf, poly, poles))
    nb = len(poles) + deg + 1
    S1 = sum(c[0] @ x[i * nb:(i + 1) * nb] for i, c in enumerate(cols))
    S2 = sum(c[1] @ x[i * nb:(i + 1) * nb] for i, c in enumerate(cols))
    sxx, syy, sxy = 0.5 * (S1 - S2.real), 0.5 * (S1 + S2.real), 0.5 * S2.imag
    hoop = sxx * T.real ** 2 + 2 * sxy * T.real * T.imag + syy * T.imag ** 2
    return float(hoop.max()), residual / math.sqrt(2 * W.sum())


def main():
    failed = []
    for D, r, recorded in RECORDED:
        got = [solve(D, 1.0, r, *level) for level in LEVELS]
        (k0, _), (k1, rms) = got
        # Reproduced when the recorded figure lies within this solve's own
        # convergence (the last degree's change) and its printed rounding.
        band = abs(k1 - k0) + 0.5e-5
        ok = abs(k1 - recorded) <= band
        print(f"D/d {D:g} r/d {r:g}: K_t {k0:.5f} -> {k1:.5f} (boundary residual {rms:.1e});"
              f" recorded {recorded:.5f}, {'reproduced' if ok else 'NOT reproduced'} within {band:.1e}")
        if not ok:
            failed.append((D, r))
    if failed:
        sys.exit(f"not reproduced: {failed}")


if __name__ == "__main__":
    main()
