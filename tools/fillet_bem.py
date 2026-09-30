#!/usr/bin/env python3
"""The exact elastic peak at a generated tooth's root, against what the crate
rates there.

    python3 tools/fillet_bem.py              # the bias tables: the record joined with today's ratings
    python3 tools/fillet_bem.py --self-test  # the instrument's canaries and its planted faults
    python3 tools/fillet_bem.py --run [JOBS] # re-solve every tooth and rewrite the record (by hand)
    python3 tools/fillet_bem.py --tooth KIND Z ALPHA X RHO MATE   # one tooth, three meshes

**An analysis, run by hand, whose conclusions are gated.** A tooth takes ten
to thirty seconds to solve (a whole pinion the longest), the record about nine
minutes on twelve cores; the answer, the
peak, depends on the tooth's geometry alone and is kept in
`tools/fillet_bem.txt` beside the figures of the geometry it was solved on.
The default mode reads that record, asks `gear-cli fillet grid` for what the
crate rates on the same teeth today, refuses to print a ratio if any tooth's
geometry has moved since it was solved (re-run `--run` then), and prints the
bias tables `docs/state.md` quotes, so `tools/check_figures.py` sees a change
in the rating as a change in the bias. That mode needs no numpy.

# The instrument

A plane-strain boundary-element method written here and sharing no code with
`gear_core` — the direct formulation with Kelvin's fundamental solution,
quadratic isoparametric elements, collocation at the nodes, the strongly
singular diagonal by the rigid-body identity, weakly singular integrals by a
cubic substitution and nearly singular ones by recursive subdivision; the
boundary stress from the element's own shape derivative and the peak from a
least-squares parabola over the samples round the largest. It is the research
round's instrument (`work/notch-research.md`, prototype P2) with its dense
elimination replaced by LAPACK's and its assembly vectorised.

It reads only the tooth's outline, which `gear-cli fillet ... outline` prints
sampled on the crate's exact curves with their tangents, and interpolates
between samples by cubic Hermite segments. The body is five teeth on a rim
`RIM` modules deep, its far arc and its two cuts held; or, on a pinion small
enough that such a rim would reach a shaft of `BORE` of its root radius (z
12 and 17 here), the whole gear held on that shaft. The load is a smooth `cos²`
traction of half-width 0.05 m along the crate's load direction, centred on
the crate's load point, the highest point of single-pair contact (the tip,
below a contact ratio of one: the patch then straddles the corner), and
scaled so its resultant is `F_n = F_t / cos α_n` with `F_t / (b m) = 1`, so
the peak **is** the factor the crate multiplies `F_t / (b m)` by. Three
meshes, each finer at the last one's peak; the last is the record's.

# The canaries, before the instrument is trusted (`--self-test`)

Each held to `TOL`, the research round's pre-registered 0.5 %:

- **Kirsch**: a circular hole under remote tension, `K_t = 3` exactly.
- **Inglis**: an elliptical hole, `a/b = 3`, `K_t = 7` exactly.
- **Golovin**: a curved bar in pure bending, exact, with a held end — the
  finite, mixed problem the hole canaries are not.
- **Four stepped flat bars** in in-plane bending, against a second solver
  that shares nothing with this one but the elasticity
  (`tools/shoulder_trefftz.py`, whose recorded answers these are).
- **The research round's canary tooth**, as its instrument solved it, to that
  instrument's own resolution.
- **The body**: a shaft half as large under a whole pinion, a rim twice as
  deep or seven teeth in a sector (external and ring), and a sector against
  the whole gear at z 30, just past the switch, each move the peak by less
  than `TOL`.

**Peterson's shoulder**, the canary the research pre-registered — Chart 3.4
of Pilkey and Pilkey, *Peterson's Stress Concentration Factors*, 3rd ed.
(2008), from the photoelastic work of Leven and Hartman (1951) and Wilson and
White (1973), whose fit `K_t = C₁ + C₂(2h/D) + C₃(2h/D)² + C₄(2h/D)³`,
`σ_nom = 6M/(t d²)`, is read from its reproduction in Pilkey, *Formulas for
Stress, Strain, and Structural Matrices*, 2nd ed. (2005) — is printed beside
the two solvers and not gated: it sits 0.5–2 % off both, which agree with
each other to a tenth of that, so it is the chart's figure that is measured.

The planted faults are the instrument with one thing wrong: a held end held
at rest (the hole canaries pass it), plane stress's modulus (it converges, so
a refinement gate passes it), a kernel sign, and every stress scaled by ten
times the tolerance and by a tenth of it (which must pass).
"""

import math
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "tools"))
RECORD = ROOT / "tools" / "fillet_bem.txt"
# The rim under a sector's teeth, modules: deep enough that twice as much
# moves the peak by less than the tolerance (`--self-test` holds it; at five
# modules a 40-tooth ring moved 1.1 %).
RIM = 10.0
# The shaft a small pinion is held on, as a fraction of its root radius: where
# a rim of `RIM` modules would reach it, the whole gear is modelled on it
# (`body_of`). Half as large moves the peak by less than the tolerance
# (`--self-test` holds it).
BORE = 0.25
# The gate every canary is held to: pre-registered by the research round
# (`work/notch-research.md`, next round P3, item 1), and fine enough to resolve the
# biases it exists to measure, which run from a few to tens of percent.
TOL = 0.005
# Planted faults that scale every boundary stress: one a tenth of the
# tolerance (rounding-level, must pass) and one ten times it (must fail).
SCALED = {"tenth": 1 + TOL / 10, "tenfold": 1 + 10 * TOL}
BIN = os.environ.get("GEAR_CLI") or str(ROOT / "target" / "release" / "gear-cli")


# --------------------------------------------------------------------------
# The boundary-element method
# --------------------------------------------------------------------------

def _np():
    import numpy

    return numpy


class BEM:
    """Plane-strain direct BEM. The domain lies to the LEFT of the direction
    of travel (outer loop counter-clockwise, holes clockwise); the outward
    normal is `(t_y, −t_x)`."""

    def __init__(self, E=1.0, nu=0.3, exterior=False, fault=None):
        np = _np()
        self.np = np
        self.E, self.nu, self.exterior = E, nu, exterior
        self.mu = E / (2 * (1 + nu))
        self.C1 = 1 / (8 * math.pi * self.mu * (1 - nu))
        self.C2 = -1 / (4 * math.pi * (1 - nu))
        self.nodes = []
        self.elems = []
        # A planted fault for the self-test, else None.
        self.fault = fault
        self.GL = {n: np.polynomial.legendre.leggauss(n) for n in (4, 6, 8, 12, 16)}

    # ---- the mesh ----------------------------------------------------------
    def node(self, p):
        self.nodes.append((float(p[0]), float(p[1])))
        return len(self.nodes) - 1

    def add_curve(self, f, p0, p1, size, bc="t", tval=None, uval=None, first=None,
                  last=None, nsamp=3000, tag=""):
        """Mesh the curve `f(p)`, `p0 → p1`, with element length `size(x, y)`,
        graded by equal increments of `∫ ds/h`. `first`/`last` reuse existing
        end nodes. `tval`/`uval` give the prescribed traction/displacement at
        `(x, y)`. Returns the end nodes."""
        ps = [p0 + (p1 - p0) * i / nsamp for i in range(nsamp + 1)]
        xs = [f(p) for p in ps]
        w = [0.0]
        for a, b in zip(xs[:-1], xs[1:]):
            mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
            w.append(w[-1] + math.dist(a, b) / size(*mid))
        # Whole elements: a count a rounding over an integer is that integer.
        n = max(1, int(math.ceil(w[-1] - 1e-9)))

        def p_at(target):
            lo, hi = 0, nsamp
            while hi - lo > 1:
                mid = (lo + hi) // 2
                if w[mid] < target:
                    lo = mid
                else:
                    hi = mid
            fr = 0.0 if w[hi] == w[lo] else (target - w[lo]) / (w[hi] - w[lo])
            return ps[lo] + (ps[hi] - ps[lo]) * fr

        pts = [p_at(w[-1] * i / (2 * n)) for i in range(2 * n + 1)]
        pts[0], pts[-1] = p0, p1
        idx = []
        for i, p in enumerate(pts):
            if i == 0 and first is not None:
                idx.append(first)
            elif i == 2 * n and last is not None:
                idx.append(last)
            else:
                idx.append(self.node(f(p)))
        for e in range(n):
            self.elems.append(dict(n=(idx[2 * e], idx[2 * e + 1], idx[2 * e + 2]), bc=bc,
                                   tval=tval, uval=uval, tag=tag))
        return idx[0], idx[-1]

    # ---- geometry of an element at local coordinates xi (array) -------------
    def _geo(self, e, xi):
        np = self.np
        P = self._P[e]
        N = np.stack([0.5 * xi * (xi - 1), 1 - xi * xi, 0.5 * xi * (xi + 1)])
        dN = np.stack([xi - 0.5, -2 * xi, xi + 0.5])
        x = N.T @ P[:, 0]
        y = N.T @ P[:, 1]
        dx = dN.T @ P[:, 0]
        dy = dN.T @ P[:, 1]
        J = np.hypot(dx, dy)
        return N, x, y, J, dy / J, -dx / J

    # ---- kernels, sources (S) by quadrature points (Q) ------------------------
    def _accumulate(self, e, S, xi, wt, H, G, rows):
        """Add ∫T*N_k and ∫U*N_k over the quadrature `(xi, wt)` of element `e`
        for the sources `S` into `H[rows]`, `G[rows]` (each `(·, 3, 4)`)."""
        np = self.np
        nu = self.nu
        N, x, y, J, nx, ny = self._geo(e, xi)
        xs = self._X[S][:, None]
        ys = self._Y[S][:, None]
        rx = x[None, :] - xs
        ry = y[None, :] - ys
        r = np.hypot(rx, ry)
        rx = rx / r
        ry = ry / r
        drn = rx * nx + ry * ny
        lr = -np.log(r)
        C1, C2 = self.C1, self.C2
        a0 = 1 - 2 * nu
        k = C2 / r
        u11 = C1 * ((3 - 4 * nu) * lr + rx * rx)
        u22 = C1 * ((3 - 4 * nu) * lr + ry * ry)
        u12 = C1 * rx * ry
        t11 = k * drn * (a0 + 2 * rx * rx)
        t22 = k * drn * (a0 + 2 * ry * ry)
        t12 = k * (drn * 2 * rx * ry - a0 * (rx * ny - ry * nx))
        t21 = k * (drn * 2 * rx * ry - a0 * (ry * nx - rx * ny))
        if self.fault == "kernel":
            # The rotation term of T* with its sign reversed on one component.
            t12 = k * (drn * 2 * rx * ry + a0 * (rx * ny - ry * nx))
        W = N * (wt * J)[None, :]  # (3, Q)
        for c, t in enumerate((t11, t12, t21, t22)):
            H[rows, :, c] += t @ W.T
        for c, u in enumerate((u11, u12, u12, u22)):
            G[rows, :, c] += u @ W.T

    def _element(self, e, H, G):
        """Every source's integrals over element `e`."""
        np = self.np
        n_e = self.elems[e]["n"]
        Ns = len(self.nodes)
        # Self: the source is a node of the element, singular at xi0 — the
        # cubic substitution towards it on each side.
        xg, wg = self.GL[16]
        t = 0.5 * (xg + 1)
        for li, j in enumerate(n_e):
            xi0 = (-1.0, 0.0, 1.0)[li]
            for end in (-1.0, 1.0):
                if end == xi0:
                    continue
                xi = xi0 + (end - xi0) * t ** 3
                wt = abs(end - xi0) * 3 * t * t * 0.5 * wg
                self._accumulate(e, np.array([j]), xi, wt, H, G, np.array([j]))
        others = np.setdiff1d(np.arange(Ns), np.array(n_e))
        self._near(e, others, -1.0, 1.0, H, G, 0)

    def _near(self, e, S, a, b, H, G, depth):
        """Sources `S` over `[a, b]`: subdivided where a source sits closer
        than the sub-element's chord, else Gauss of an order set by the
        distance ratio. No source but the element's own nodes (taken by
        `_element`) lies on it, so halving ends; thirty halvings, a
        sub-element 1e-9 of the element, is where it is cut regardless."""
        np = self.np
        if len(S) == 0:
            return
        _, xa, ya, *_ = self._geo(e, np.array([a, b, 0.5 * (a + b)]))
        L = math.hypot(xa[1] - xa[0], ya[1] - ya[0])
        d = np.min(np.hypot(self._X[S][:, None] - xa[None, :], self._Y[S][:, None] - ya[None, :]), axis=1)
        ratio = d / L
        near = ratio < 1.0
        if depth >= 30:
            near[:] = False
        if near.any():
            m = 0.5 * (a + b)
            self._near(e, S[near], a, m, H, G, depth + 1)
            self._near(e, S[near], m, b, H, G, depth + 1)
        far = ~near
        order = np.where(ratio > 6, 4, np.where(ratio > 3, 6, np.where(ratio > 1.8, 8, 12)))
        for n in (4, 6, 8, 12):
            sel = far & (order == n)
            if sel.any():
                xg, wg = self.GL[n]
                xi = 0.5 * (a + b) + 0.5 * (b - a) * xg
                wt = 0.5 * (b - a) * wg
                self._accumulate(e, S[sel], xi, wt, H, G, S[sel])

    def solve(self):
        np = self.np
        Nn = len(self.nodes)
        n2 = 2 * Nn
        self._X = np.array([p[0] for p in self.nodes])
        self._Y = np.array([p[1] for p in self.nodes])
        self._P = [np.array([self.nodes[j] for j in e["n"]]) for e in self.elems]
        fixed = np.zeros(Nn, bool)
        uknown = {}
        for e in self.elems:
            if e["bc"] == "u":
                for j in e["n"]:
                    fixed[j] = True
                    f = e["uval"]
                    # The planted `held` fault holds every held node at rest
                    # whatever it was given: a body held at rest, as a tooth's
                    # rim is, and a problem held nowhere, as both hole
                    # canaries are, cannot tell.
                    uknown[j] = f(*self.nodes[j]) if f and self.fault != "held" else (0.0, 0.0)
        Hm = np.zeros((n2, n2))
        A = np.zeros((n2, n2))
        rhs = np.zeros(n2)
        for ei, e in enumerate(self.elems):
            H = np.zeros((Nn, 3, 4))
            G = np.zeros((Nn, 3, 4))
            self._element(ei, H, G)
            for kk, j in enumerate(e["n"]):
                h = H[:, kk, :]
                g = G[:, kk, :]
                Hm[0::2, 2 * j] += h[:, 0]
                Hm[0::2, 2 * j + 1] += h[:, 1]
                Hm[1::2, 2 * j] += h[:, 2]
                Hm[1::2, 2 * j + 1] += h[:, 3]
                if e["bc"] == "t":
                    f = e["tval"]
                    tx, ty = f(*self.nodes[j]) if f else (0.0, 0.0)
                    rhs[0::2] += g[:, 0] * tx + g[:, 1] * ty
                    rhs[1::2] += g[:, 2] * tx + g[:, 3] * ty
                else:
                    A[0::2, 2 * j] -= g[:, 0]
                    A[0::2, 2 * j + 1] -= g[:, 1]
                    A[1::2, 2 * j] -= g[:, 2]
                    A[1::2, 2 * j + 1] -= g[:, 3]
        # The diagonal blocks by the rigid-body identity: a rigid translation
        # carries no traction, so each row of H sums to the free term.
        for i in range(Nn):
            Hm[2 * i:2 * i + 2, 2 * i:2 * i + 2] = 0.0
        sx = Hm[:, 0::2].sum(axis=1)
        sy = Hm[:, 1::2].sum(axis=1)
        ext = 1.0 if self.exterior else 0.0
        for i in range(Nn):
            for c in (0, 1):
                Hm[2 * i + c, 2 * i] = (ext if c == 0 else 0.0) - sx[2 * i + c]
                Hm[2 * i + c, 2 * i + 1] = (ext if c == 1 else 0.0) - sy[2 * i + c]
        free = np.repeat(~fixed, 2)
        A[:, free] += Hm[:, free]
        for j in np.nonzero(fixed)[0]:
            ux, uy = uknown[j]
            rhs -= Hm[:, 2 * j] * ux + Hm[:, 2 * j + 1] * uy
        sol = np.linalg.solve(A, rhs)
        self.u = [None] * Nn
        tfix = [None] * Nn
        for j in range(Nn):
            if fixed[j]:
                self.u[j] = uknown[j]
                tfix[j] = (sol[2 * j], sol[2 * j + 1])
            else:
                self.u[j] = (sol[2 * j], sol[2 * j + 1])
        for e in self.elems:
            f = e["tval"]
            e["tn"] = [((f(*self.nodes[j]) if f else (0.0, 0.0)) if e["bc"] == "t" else tfix[j])
                       for j in e["n"]]
        return self

    # ---- the boundary stress -----------------------------------------------
    def boundary_stress(self, e, xi):
        """`(σ_tt, σ_nn, x, y, t_x, t_y)` at `xi` on element `e`, plane strain:
        `σ_tt = E' ε_tt + ν' σ_nn` from the element's own shape derivative."""
        N = (0.5 * xi * (xi - 1), 1 - xi * xi, 0.5 * xi * (xi + 1))
        dN = (xi - 0.5, -2 * xi, xi + 0.5)
        P = [self.nodes[j] for j in e["n"]]
        U = [self.u[j] for j in e["n"]]
        T = e["tn"]
        dx = sum(dN[k] * P[k][0] for k in range(3))
        dy = sum(dN[k] * P[k][1] for k in range(3))
        J = math.hypot(dx, dy)
        tx, ty = dx / J, dy / J
        nx, ny = ty, -tx
        dux = sum(dN[k] * U[k][0] for k in range(3))
        duy = sum(dN[k] * U[k][1] for k in range(3))
        ett = (dux * tx + duy * ty) / J
        trx = sum(N[k] * T[k][0] for k in range(3))
        try_ = sum(N[k] * T[k][1] for k in range(3))
        snn = trx * nx + try_ * ny
        # Plane strain. The planted `strain` fault takes plane stress's
        # modulus: a solve that converges, to a figure 9 % low.
        Ep = self.E if self.fault == "strain" else self.E / (1 - self.nu ** 2)
        nup = self.nu / (1 - self.nu)
        if self.fault in SCALED:
            Ep *= SCALED[self.fault]
        x = sum(N[k] * P[k][0] for k in range(3))
        y = sum(N[k] * P[k][1] for k in range(3))
        return Ep * ett + nup * snn, snn, x, y, tx, ty


G3 = 1 / math.sqrt(3)


def sample_tt(M, elems, extra=None):
    """`σ_tt` at each element's two Gauss points, in the elements' order, with
    the arc length between consecutive samples: `[(σ, x, y, arc)]`.
    `extra(x, y, t_x, t_y)` adds a remote field's part."""
    out = []
    arc = 0.0
    last = None
    for e in elems:
        for xi in (-G3, G3):
            stt, _, x, y, tx, ty = M.boundary_stress(e, xi)
            if extra:
                stt += extra(x, y, tx, ty)
            if last is not None:
                arc += math.dist(last, (x, y))
            last = (x, y)
            out.append((stt, x, y, arc))
    return out


def peak_ls(samples, half, nmin=9):
    """The peak of a least-squares parabola in arc length through the samples
    within `±half` of the largest (at least `nmin`), which averages out the
    element-to-element staircase of a C⁰ derivative. `(peak, x, y)`."""
    i = max(range(len(samples)), key=lambda k: samples[k][0])
    a0 = samples[i][3]
    order = sorted(range(len(samples)), key=lambda k: abs(samples[k][3] - a0))
    use = [k for k in order if abs(samples[k][3] - a0) <= half]
    if len(use) < nmin:
        use = order[:nmin]
    xs = [samples[k][3] - a0 for k in use]
    ys = [samples[k][0] for k in use]
    S = [[sum(x ** (p + q) for x in xs) for q in range(3)] for p in range(3)]
    b = [sum(y * x ** p for x, y in zip(xs, ys)) for p in range(3)]
    c = _solve3(S, b)
    x_at, y_at = samples[i][1], samples[i][2]
    if c[2] < 0:
        xv = -c[1] / (2 * c[2])
        if min(xs) <= xv <= max(xs):
            j = min(use, key=lambda k: abs(samples[k][3] - a0 - xv))
            return c[0] + c[1] * xv + c[2] * xv * xv, samples[j][1], samples[j][2]
    return max(ys), x_at, y_at


def _solve3(S, b):
    """Cramer's rule on the 3×3 normal equations."""
    def det(m):
        return (m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
                - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
                + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]))
    d = det(S)
    out = []
    for c in range(3):
        m = [row[:] for row in S]
        for r in range(3):
            m[r][c] = b[r]
        out.append(det(m) / d)
    return out


# --------------------------------------------------------------------------
# The canaries' bodies
# --------------------------------------------------------------------------

def hole(a, b, h_tip, h_far, grade, fault=None):
    """An elliptical hole, semi-axes `a` along x and `b` along y, in an
    infinite plane under `σ_yy = 1` far away: the perturbation problem, with
    traction `−σ_∞·n` on the hole. The peak `σ_tt` is at `(±a, 0)`."""
    M = BEM(1.0, 0.3, exterior=True, fault=fault)

    def f(p):
        return (a * math.cos(p), -b * math.sin(p))  # clockwise: domain on the left

    def size(x, y):
        d = min(math.hypot(x - a, y), math.hypot(x + a, y))
        return max(h_tip, min(h_far, h_tip + grade * d))

    def tr(x, y):
        nx, ny = -x / a ** 2, -y / b ** 2
        L = math.hypot(nx, ny)
        return (0.0, -ny / L)

    a0, b0 = M.add_curve(f, 0.0, math.pi, size, tval=tr)
    M.add_curve(f, math.pi, 2 * math.pi, size, tval=tr, first=b0, last=a0)
    M.solve()
    return peak_ls(sample_tt(M, M.elems, extra=lambda x, y, tx, ty: ty * ty), half=0.0)[0]


def peterson_shoulder_kt(D, d, r):
    """Chart 3.4's fit: `K_t` of a stepped flat bar with shoulder fillets in
    in-plane bending, `σ_nom = 6M/(t d²)`, on its branch `2 ≤ h/r ≤ 20`.

    The other branch, `h/r < 2`, is not used: its `C₄` as the reproduction
    prints it, `−0.405 + 0.249√(h/r) − 0.200 h/r`, does not meet this
    branch's at `h/r = 2` (−0.453 against +0.348) where the other three
    coefficients meet to 0.005, so a sign there is in doubt."""
    h = (D - d) / 2
    q = h / r
    s = math.sqrt(q)
    if not 2.0 <= q <= 20.0:
        raise ValueError(f"h/r = {q} is outside the branch the canary reads")
    c = (1.058 + 1.002 * s - 0.038 * q, -3.652 + 1.639 * s - 0.436 * q,
         6.170 - 5.687 * s + 1.175 * q, -2.558 + 3.046 * s - 0.701 * q)
    u = 2 * h / D
    return c[0] + c[1] * u + c[2] * u * u + c[3] * u ** 3


def shoulder(D, d, r, h_tip, grade=0.1, h_far=None, lengths=4.0, fault=None):
    """`K_t` of a stepped flat bar with shoulder fillets (`h ≥ r`: a quarter
    round and a square shoulder face) under in-plane bending, by the BEM:
    the narrow end `lengths·d` from the step carries `σ_xx = σ_nom·y/(d/2)`,
    the wide end `lengths·D` beyond it is held at the exact pure-bending
    displacement of the moment that traction applies, which is consistent with
    it, so neither end disturbs the step. Plane strain; `K_t` is `ν`-free
    (Michell)."""
    E, nu = 1.0, 0.3
    h = (D - d) / 2
    if h < r:
        raise ValueError("the canary's shoulder needs h ≥ r")
    M = BEM(E, nu, fault=fault)
    h_far = h_far or 0.25 * d
    Ln, Lw = lengths * d, lengths * D
    c_d = 2.0 / d  # σ_xx = c_d·y on the narrow part: σ_nom = c_d·d/2 = 1
    c_D = c_d * (d / D) ** 3  # the same moment on the wide part
    A = (1 - nu * nu) * c_D / E
    B = -nu * (1 + nu) * c_D / E

    def u_end(x, y):
        return (A * x * y, -A * x * x / 2 + B * y * y / 2)

    def to_fillet(x, y, yc, lo, hi):
        """Distance from `(x, y)` to the quarter round about `(−r, yc)`
        spanning angles `lo … hi`."""
        th = math.atan2(y - yc, x + r)
        if lo <= th <= hi:
            return abs(math.hypot(x + r, y - yc) - r)
        return min(math.dist((x, y), (-r + r * math.cos(a), yc + r * math.sin(a))) for a in (lo, hi))

    def size(x, y):
        dd = min(to_fillet(x, y, d / 2 + r, -math.pi / 2, 0.0),
                 to_fillet(x, y, -d / 2 - r, 0.0, math.pi / 2))
        return min(h_far, h_tip + grade * dd)

    def line(p, q):
        return lambda s: (p[0] + (q[0] - p[0]) * s, p[1] + (q[1] - p[1]) * s)

    yc_t, yc_b = d / 2 + r, -d / 2 - r
    pieces = [
        (line((-Ln, -d / 2), (-r, -d / 2)), 0.0, 1.0, "t", None, "narrow-"),
        (lambda th: (-r + r * math.cos(th), yc_b + r * math.sin(th)), math.pi / 2, 0.0, "t", None, "fillet-"),
        (line((0.0, yc_b), (0.0, -D / 2)), 0.0, 1.0, "t", None, "face-"),
        (line((0.0, -D / 2), (Lw, -D / 2)), 0.0, 1.0, "t", None, "wide-"),
        (line((Lw, -D / 2), (Lw, D / 2)), 0.0, 1.0, "u", None, "end"),
        (line((Lw, D / 2), (0.0, D / 2)), 0.0, 1.0, "t", None, "wide+"),
        (line((0.0, D / 2), (0.0, yc_t)), 0.0, 1.0, "t", None, "face+"),
        (lambda th: (-r + r * math.cos(th), yc_t + r * math.sin(th)), 0.0, -math.pi / 2, "t", None, "fillet+"),
        (line((-r, d / 2), (-Ln, d / 2)), 0.0, 1.0, "t", None, "narrow+"),
        (line((-Ln, d / 2), (-Ln, -d / 2)), 0.0, 1.0, "t", lambda x, y: (-c_d * y, 0.0), "load"),
    ]
    first = prev = None
    for k, (f, a0, b0, bc, tv, tag) in enumerate(pieces):
        if bc == "t" and h == r and tag.startswith("face"):
            continue
        last = first if k == len(pieces) - 1 else None
        fa, fb = M.add_curve(f, a0, b0, size, bc=bc, tval=tv, uval=u_end, first=prev,
                             last=last, tag=tag)
        first = fa if first is None else first
        prev = fb
    M.solve()
    top = [e for e in M.elems if e["tag"] in ("face+", "fillet+", "narrow+")]
    return peak_ls(sample_tt(M, top), 0.15 * r)[0], len(M.elems)


def golovin_field(a, b, M, E, nu):
    """Golovin's exact curved bar in pure bending (Timoshenko and Goodier,
    *Theory of Elasticity*, article 29), plane strain: `σ_r(r)`, `σ_θ(r)` and the
    displacement `u_r(r)` on the end `θ = 0` (where `u_θ = 0` with the rigid
    motion chosen so), for the moment `M` that stretches the inner face."""
    lba = math.log(b / a)
    N = (b * b - a * a) ** 2 - 4 * a * a * b * b * lba * lba
    A = -4 * M / N * a * a * b * b * lba
    B = -2 * M / N * (b * b - a * a)
    C = M / N * (b * b - a * a + 2 * (b * b * math.log(b) - a * a * math.log(a)))
    Ep, nup = E / (1 - nu * nu), nu / (1 - nu)

    def s_r(r):
        return A / r ** 2 + B * (1 + 2 * math.log(r)) + 2 * C

    def s_t(r):
        return -A / r ** 2 + B * (3 + 2 * math.log(r)) + 2 * C

    def u_r(r):
        return (-(1 + nup) * A / r + 2 * (1 - nup) * B * r * math.log(r) - (1 + nup) * B * r
                + 2 * (1 - nup) * C * r) / Ep

    return s_r, s_t, u_r


def golovin(a, b, h, fault=None):
    """The curved bar by the BEM: a quarter annulus, the end `θ = 0` held at
    the exact displacement, the end `θ = π/2` carrying the exact `σ_θ`. Returns
    the largest relative error of `σ_tt` against the exact `σ_θ(a)` over the
    middle half of the inner face, where the peak is."""
    E, nu = 1.0, 0.3
    M = 1.0
    s_r, s_t, u_r = golovin_field(a, b, M, E, nu)
    B = BEM(E, nu, fault=fault)

    def size(x, y):
        return h

    def radial(th, r0, r1):
        return lambda s: ((r0 + (r1 - r0) * s) * math.cos(th), (r0 + (r1 - r0) * s) * math.sin(th))

    def circle(r):
        return lambda th: (r * math.cos(th), r * math.sin(th))

    q = math.pi / 2
    pieces = [
        (radial(0.0, a, b), 0.0, 1.0, "u", None, "fixed"),
        (circle(b), 0.0, q, "t", None, "outer"),
        (radial(q, b, a), 0.0, 1.0, "t", lambda x, y: (-s_t(math.hypot(x, y)), 0.0), "load"),
        (circle(a), q, 0.0, "t", None, "inner"),
    ]
    first = prev = None
    for k, (f, p0, p1, bc, tv, tag) in enumerate(pieces):
        last = first if k == len(pieces) - 1 else None
        fa, fb = B.add_curve(f, p0, p1, size, bc=bc, tval=tv,
                             uval=lambda x, y: (u_r(x), 0.0), first=prev, last=last, tag=tag)
        first = fa if first is None else first
        prev = fb
    B.solve()
    exact = s_t(a)
    worst = 0.0
    count = 0
    samples = sample_tt(B, [e for e in B.elems if e["tag"] == "inner"])
    for s in samples:
        th = math.atan2(s[2], s[1])
        if q / 4 <= th <= 3 * q / 4:
            worst = max(worst, abs(s[0] / exact - 1))
            count += 1
    # A uniform mesh: the middle half of the face holds half its samples.
    assert abs(count - len(samples) / 2) <= 2, (count, len(samples))
    return worst, count, exact * (b - a) ** 2 / (6 * M)


# --------------------------------------------------------------------------
# The crate's tooth, read from the harness
# --------------------------------------------------------------------------

class Curve:
    """A curve through samples `(x, y, t_x, t_y)`, unit tangents, by cubic
    Hermite segments in chord length: the interpolant matches each sample's
    point and direction, so between samples a few microns apart it departs
    from the curve by rounding."""

    def __init__(self, pts):
        self.p = [(x, y) for x, y, _, _ in pts]
        s = [0.0]
        for a, b in zip(self.p[:-1], self.p[1:]):
            s.append(s[-1] + math.dist(a, b))
        self.s = s
        self.length = s[-1]
        # Each tangent along the direction of travel.
        t = []
        for i, (_, _, tx, ty) in enumerate(pts):
            j0, j1 = (i, i + 1) if i + 1 < len(pts) else (i - 1, i)
            cx, cy = self.p[j1][0] - self.p[j0][0], self.p[j1][1] - self.p[j0][1]
            sgn = 1.0 if cx * tx + cy * ty >= 0 else -1.0
            t.append((sgn * tx, sgn * ty))
        self.t = t

    def __call__(self, q):
        s = self.s
        lo, hi = 0, len(s) - 1
        q = min(max(q, 0.0), self.length)
        while hi - lo > 1:
            mid = (lo + hi) // 2
            if s[mid] <= q:
                lo = mid
            else:
                hi = mid
        h = s[hi] - s[lo]
        u = (q - s[lo]) / h
        h00 = 2 * u ** 3 - 3 * u * u + 1
        h10 = u ** 3 - 2 * u * u + u
        h01 = -2 * u ** 3 + 3 * u * u
        h11 = u ** 3 - u * u
        (x0, y0), (x1, y1) = self.p[lo], self.p[hi]
        (a0, b0), (a1, b1) = self.t[lo], self.t[hi]
        return (h00 * x0 + h10 * h * a0 + h01 * x1 + h11 * h * a1,
                h00 * y0 + h10 * h * b0 + h01 * y1 + h11 * h * b1)


def _numbers(words):
    """`key value` pairs after the first word, as floats where they parse."""
    out = {}
    for k, v in zip(words[1::2], words[2::2]):
        try:
            out[k] = float(v)
        except ValueError:
            out[k] = v
    return out


def parse_cases(text):
    """`gear-cli fillet` output, one dict per case."""
    cases = []
    lines = text.splitlines()
    i = 0
    while i < len(lines):
        w = lines[i].split()
        if not w:
            i += 1
            continue
        if w[0] == "case":
            head, _, tail = lines[i].partition(" | ")
            hw = head.split()
            c = dict(kind=hw[1], z=int(hw[3]), alpha=float(hw[5]), x=float(hw[7]),
                     rho=float(hw[9]), mate=int(hw[11]), pieces=[])
            if tail.strip() == "no_mesh":
                c["no_mesh"] = True
            else:
                c.update(_numbers(["-"] + tail.split()))
            cases.append(c)
        elif w[0] in ("db", "iso"):
            cases[-1][w[0]] = None if w[1] == "none" else _numbers(w)
        elif w[0] == "load":
            cases[-1]["load"] = _numbers(w)
        elif w[0] == "piece":
            n = int(w[2])
            pts = [tuple(map(float, lines[i + 1 + k].split())) for k in range(n)]
            cases[-1]["pieces"].append((w[1], pts))
            i += n
        i += 1
    return cases


def harness(*args):
    """Run `gear-cli` and return its output."""
    return subprocess.run([BIN, *map(str, args)], capture_output=True, text=True, check=True).stdout


def crate_tooth(kind, z, alpha, x, rho, mate):
    """One tooth with its outline."""
    return parse_cases(harness("fillet", kind, z, alpha, x, rho, mate, "outline"))[0]


def rot(p, f):
    """A point turned by `f` about the gear's axis (angle from `+y` toward `+x`)."""
    c, s = math.cos(f), math.sin(f)
    return (p[0] * c + p[1] * s, p[1] * c - p[0] * s)


def tooth_model(c, h_tip, hot, grade=0.1, h_far=0.25, h_rest=0.5, rim=RIM, bore=BORE, c_p=0.05,
                nb=2, E=206000.0, nu=0.3, whole=None, cuts="u", fault=None):
    """The BEM model of the member `c`, the centre tooth loaded on its `+x`
    flank. Returns the solved model and the loaded side's root, fillet and
    flank elements in order along the outline.

    The body (`body_of`): a sector of `2 nb + 1` teeth on a rim `rim`
    modules deep, its far arc and its two cuts held; or, where such a rim
    would reach a shaft of radius `bore · r_f` (a small pinion), the whole
    gear, every tooth, held on that shaft. A ring's rim is outside its teeth
    and never reaches anything."""
    ring = c["kind"] == "ring"
    hp = c["half_pitch"]
    rf = c["r_f"]
    whole = body_of(c, rim, bore) == "whole" if whole is None else whole
    half = [(name, Curve(pts)) for name, pts in c["pieces"]]
    load_point = (c["load"]["x"], c["load"]["y"])
    load_dir = (c["load"]["dx"], c["load"]["dy"])
    scale = [1.0]

    def load_t(x, y):
        s = math.dist((x, y), load_point)
        if s >= c_p:
            return (0.0, 0.0)
        p = scale[0] * math.cos(0.5 * math.pi * s / c_p) ** 2
        return (p * load_dir[0], p * load_dir[1])

    def size_loaded(x, y):
        return min(h_far, h_tip + grade * math.dist((x, y), hot),
                   max(c_p / 3, 0.2 * math.dist((x, y), load_point) + c_p / 3))

    def size_rest(x, y):
        return min(h_rest, h_tip + grade * math.dist((x, y), hot))

    # Tooth by tooth, each from its -x mid-space to its +x one: the mirrored
    # half reversed, then the half as printed. The whole gear has every
    # tooth, and its last tooth ends where its first began.
    teeth = range(-(c["z"] // 2), c["z"] - c["z"] // 2) if whole else range(-nb, nb + 1)
    path = []
    for k in teeth:
        f = 2 * k * hp
        centre = k == 0
        sz = size_loaded if centre else size_rest
        for name, cv in reversed(half):
            path.append((lambda q, cv=cv, f=f: rot((-cv(q)[0], cv(q)[1]), f), cv.length, 0.0,
                         sz, (name + "-") if centre else "", None, "t"))
        for name, cv in half:
            tv = load_t if centre and name in ("flank", "tip") else None
            path.append((lambda q, cv=cv, f=f: rot(cv(q), f), 0.0, cv.length, sz,
                         (name + "+") if centre else "", tv, "t"))
    th = len(teeth) * hp

    def radial(a, r0, r1):
        return lambda s: ((r0 + (r1 - r0) * s) * math.sin(a), (r0 + (r1 - r0) * s) * math.cos(a))

    def circle(r):
        return lambda a: (r * math.sin(a), r * math.cos(a))

    # Each loop keeps the body on its left: the outline of an external gear
    # counter-clockwise, a ring's bore and a gear's shaft clockwise.
    loops = []
    if ring:
        ro = rf + rim
        loops.append(path + [(radial(th, rf, ro), 0.0, 1.0, size_rest, "cutR", None, cuts),
                             (circle(ro), th, -th, size_rest, "far", None, "u"),
                             (radial(-th, ro, rf), 0.0, 1.0, size_rest, "cutL", None, cuts)])
    else:
        outline = [(f, p1, p0, sz, tag, tv, bc) for f, p0, p1, sz, tag, tv, bc in reversed(path)]
        if whole:
            rs = bore * rf
            loops.append(outline)
            loops.append([(circle(rs), 0.0, math.pi, size_rest, "shaft", None, "u"),
                          (circle(rs), math.pi, 2 * math.pi, size_rest, "shaft", None, "u")])
        else:
            ri = rf - rim
            if ri <= 0:
                raise ValueError(f"a rim of {rim} modules crosses the axis at r_f {rf}")
            loops.append(outline + [(radial(-th, rf, ri), 0.0, 1.0, size_rest, "cutL", None, cuts),
                                    (circle(ri), -th, th, size_rest, "far", None, "u"),
                                    (radial(th, ri, rf), 0.0, 1.0, size_rest, "cutR", None, cuts)])
    M = BEM(E, nu, fault=fault)
    for loop in loops:
        first = prev = None
        for k, (f, p0, p1, sz, tag, tv, bc) in enumerate(loop):
            last = first if k == len(loop) - 1 else None
            fa, fb = M.add_curve(f, p0, p1, sz, bc=bc, tval=tv, first=prev, last=last, tag=tag)
            first = fa if first is None else first
            prev = fb
    # The patch's resultant, as the elements carry it interpolated, made F_n.
    np = M.np
    xg, wg = M.GL[16]
    Fx = Fy = 0.0
    for e in M.elems:
        if e["tval"] is None:
            continue
        T = [e["tval"](*M.nodes[j]) for j in e["n"]]
        P = np.array([M.nodes[j] for j in e["n"]])
        N = np.stack([0.5 * xg * (xg - 1), 1 - xg * xg, 0.5 * xg * (xg + 1)])
        dN = np.stack([xg - 0.5, -2 * xg, xg + 0.5])
        J = np.hypot(dN.T @ P[:, 0], dN.T @ P[:, 1])
        Fx += float(np.sum(wg * J * (N.T @ np.array([t[0] for t in T]))))
        Fy += float(np.sum(wg * J * (N.T @ np.array([t[1] for t in T]))))
    Fn = 1.0 / math.cos(math.radians(c["alpha"]))
    scale[0] = Fn / math.hypot(Fx, Fy)
    M.solve()
    side = [e for e in M.elems if e["tag"] in ("root+", "fillet+", "flank+")]
    return M, side


def body_of(c, rim=RIM, bore=BORE):
    """`sector` or `whole`: whether a rim `rim` modules deep under an
    external member's teeth stays clear of a shaft `bore · r_f` in radius."""
    if c["kind"] == "ring" or c["r_f"] - rim >= bore * c["r_f"]:
        return "sector"
    return "whole"


def rho_ref(c):
    """The smallest radius the rated sections read: where the mesh is finest."""
    radii = [c[s]["rho_f"] for s in ("db", "iso") if c.get(s)] + [c[s]["rho_F"] for s in ("db", "iso") if c.get(s)]
    return min(radii)


# The meshes a tooth is solved on: the element size at the peak as a
# fraction of the smallest radius the rated sections read, and the growth
# away from it; each pass is centred on the previous one's peak.
PASSES = ((8, 0.1), (16, 0.07), (32, 0.05))


def solve_tooth(c, passes=PASSES, **kw):
    """The peak on the loaded fillet over successive meshes, each finer at the
    previous one's peak: `[(peak, elements)]`."""
    rr = rho_ref(c)
    db = c.get("db")
    if db and db["on"] == "fillet":
        hot = (db["tangency_x"], db["tangency_y"])
    else:
        _, pts = next(p for p in c["pieces"] if p[0] == "fillet")
        hot = pts[0][:2]
    out = []
    for k, g in passes:
        M, side = tooth_model(c, rr / k, hot, grade=g, **kw)
        pk, px, py = peak_ls(sample_tt(M, side), 0.15 * rr)
        out.append((pk, len(M.elems)))
        hot = (px, py)
    return out


# --------------------------------------------------------------------------
# The record: each tooth's peak and the geometry it was solved on
# --------------------------------------------------------------------------

# What identifies the geometry a peak was solved on. The harness prints 13
# figures, so an unmoved one reads back within 5e-13; the peak follows the
# geometry at order one, so a move below the record's mesh convergence (1e-4
# at the median) cannot show in a ratio. A move past 1e-9, between the two,
# means the tooth is not the one recorded and must be solved again.
FINGERPRINT = [("eps", None), ("r_a", None), ("r_f", None), ("half_pitch", None),
               ("x", "load"), ("y", "load"), ("dx", "load"), ("dy", "load"),
               ("rho_f", "iso"), ("rho_F", "iso"), ("s_Fn", "iso")]
FINGERPRINT_REL = 1e-9


def fingerprint(c):
    out = []
    for key, where in FINGERPRINT:
        src = c.get(where) if where else c
        out.append(None if src is None else src.get(key))
    return out


def key_of(c):
    return (c["kind"], c["z"], c["alpha"], c["x"], c["rho"], c["mate"])


def _solve_one(args):
    """One tooth for `--run`, in a worker process."""
    kind, z, alpha, x, rho, mate = args
    c = crate_tooth(kind, z, alpha, x, rho, mate)
    if c.get("no_mesh") or not c["pieces"] or not c.get("load"):
        return c, None
    return c, solve_tooth(c)


def run(jobs):
    """Solve every tooth `gear-cli fillet grid` lists and write the record."""
    import multiprocessing as mp

    cases = parse_cases(harness("fillet", "grid"))
    with mp.Pool(jobs) as pool:
        solved = pool.map(_solve_one, [key_of(c) for c in cases], chunksize=1)
    lines = [
        "# tools/fillet_bem.py --run: the exact elastic peak at each tooth's loaded fillet,",
        "# sigma / (F_t / (b m)); five teeth on a rim of RIM modules, far arc and cuts held,",
        "# or the whole gear on a shaft of BORE r_f where that rim would reach it;",
        f"# RIM {RIM}; BORE {BORE}; passes {' '.join(f'{k}/{g}' for k, g in PASSES)} (rho_ref / k, growth g).",
    ]
    for c, peaks in solved:
        head = " ".join(str(v) for v in key_of(c))
        if peaks is None:
            lines.append(f"{head} | none")
            continue
        fp = " ".join(f"{v!r}" for v in fingerprint(c))
        lines.append(f"{head} | peak {' '.join(f'{p:.6e}' for p, _ in peaks)}"
                     f" | elements {' '.join(str(n) for _, n in peaks)} | body {body_of(c)}"
                     f" | fingerprint {fp}")
    lines += shoulder_lines()
    RECORD.write_text("\n".join(lines) + "\n")
    print(f"{len(solved)} teeth -> {RECORD.relative_to(ROOT)}")


def shoulder_lines():
    """The canary shoulders as the BEM solves them, for the summary's table."""
    return [f"shoulder {D!r} 1.0 {r!r} | bem {shoulder(D, 1.0, r, r / 16)[0]:.6e}"
            for D, r, _ in SHOULDER_TREFFTZ]


def read_record():
    rec = {}
    for line in RECORD.read_text().splitlines():
        if line.startswith("#") or not line.strip():
            continue
        if line.startswith("shoulder "):
            w = line.split()
            rec[("shoulder", float(w[1]), float(w[3]))] = float(w[-1])
            continue
        parts = [p.split() for p in line.split(" | ")]
        k = parts[0]
        key = (k[0], int(k[1]), float(k[2]), float(k[3]), float(k[4]), int(k[5]))
        if parts[1][0] == "none":
            rec[key] = None
            continue
        rec[key] = dict(peaks=[float(v) for v in parts[1][1:]],
                        elements=[int(v) for v in parts[2][1:]],
                        body=parts[3][1],
                        fingerprint=[None if v == "None" else float(v) for v in parts[4][1:]])
    return rec


# --------------------------------------------------------------------------
# The bias tables
# --------------------------------------------------------------------------

def y_s(l, q):
    """ISO 6336-3's `Y_S` at `L = s_Fn/h_Fe` and notch parameter `q`."""
    return (1.2 + 0.13 * l) * q ** (1 / (1.21 + 2.3 / l))


# ISO 6336-3 Method B's slip-layer thickness `ρ'`, mm, for through-hardened
# steels by yield point (IACS UR M56 Rev.4 M56.3.11, reproducing ISO
# 6336-3:2019): interpolated between the rows, held at the table's ends.
SLIP_LAYER = [(500.0, 0.0281), (600.0, 0.0194), (800.0, 0.0064), (1000.0, 0.0014)]


def slip_layer(yield_point):
    pts = SLIP_LAYER
    if yield_point <= pts[0][0]:
        return pts[0][1]
    if yield_point >= pts[-1][0]:
        return pts[-1][1]
    for (a, ra), (b, rb) in zip(pts[:-1], pts[1:]):
        if a <= yield_point <= b:
            return ra + (rb - ra) * (yield_point - a) / (b - a)
    raise ValueError(yield_point)


# A coupon's own relative stress gradient is its bar's, `2/d` in bending. ISO
# does not state one and the library's sources do not state their bar, so the
# support is taken over a 10 mm bar's and printed beside the R. R. Moore
# bar's, 7.62 mm (0.3 in), the smaller rotating-beam specimen (recalled).
COUPON_BARS = {"a round bar": 10.0, "the R. R. Moore bar": 7.62}
COUPON_BAR = COUPON_BARS["a round bar"]


def notch_support(rho_p, q_s, bar=COUPON_BAR):
    """ISO's support at the root over the coupon's: `1 + √(ρ' χ)` at the
    root's gradient `χ = 0.2 (1 + 2 q_s)` /mm, the form inside `Y_δrelT`
    (M56.3.11), over the same at a round bar's `2/d`. The coupon's endurance
    already holds its own bar's support, so crediting the root's alone
    counts that twice."""
    def support(chi):
        return 1 + math.sqrt(rho_p * chi)

    return support(0.2 * (1 + 2 * q_s)) / support(2 / bar)


# Tensile strength, MPa, from which ISO reads a through-hardened steel's
# surface on its own line rather than a normalised steel's (M56.3.12).
STRONG_STEEL = 800.0
# ISO's `Y_RrelT` lines (M56.3.12): `(below Rz 1, a, b, c)`, the factor
# `a − b (Rz + 1)^c` over `1 ≤ Rz ≤ 40` µm. The first is case-hardened steels'
# and through-hardened ones' of `σ_B ≥ STRONG_STEEL`, the second normalised
# steels' below it, the third nitrided steels', which the library has none of.
SURFACE_LINES = {
    f"the line for sigma_B {STRONG_STEEL:g} and above": (1.120, 1.674, 0.529, 0.1),
    f"the line below sigma_B {STRONG_STEEL:g}": (1.070, 5.306, 4.203, 0.01),
    "the nitrided steels' line": (1.025, 4.299, 3.259, 0.0058),
}
# The top of the range ISO's lines are stated over, µm.
RZ_MAX = 40.0


def surface_relative(rz, line):
    """ISO's `Y_RrelT` at a root roughness `Rz` µm on one of its lines;
    refused past the range the lines are stated over."""
    below, a, b, c = SURFACE_LINES[line]
    if rz > RZ_MAX:
        raise ValueError(f"Rz {rz} is past ISO's range")
    return below if rz < 1 else a - b * (rz + 1) ** c


# The root roughness ISO's reference test gear has (its `Y_RrelT` = 1): a
# hobbed root. A coupon's is read on ISO's line below `Rz` 1 µm, as a polished
# bar's, though no figure in the library states its finish: the steels' are
# rotating-beam figures, annealed 4340's published and 4340 Hardened's
# estimated by the classical 0.5 × UTS ratio. Only the steels are read here.
# Not every endurance is such a coupon: brass's is a 0.30 × UTS estimate,
# POM's a moulded ASTM D671 bar, the polyamides' 0.30 × ultimate with no
# specimen, and ISO has no line for any of them.
RZ_REFERENCE = 10.0
RZ_POLISHED = 0.5
# The roots the surface is printed at: ground (a ground root is typically
# 2–4 µm), ISO's hobbed reference, and the top of ISO's range.
RZ_ROOTS = (("ground", 3.0), ("hobbed", RZ_REFERENCE), ("the top of ISO's range", RZ_MAX))


def surface_line(steel):
    """The ISO line a through-hardened library steel is read on."""
    strong, normalised, _ = SURFACE_LINES
    return strong if steel["uts"] >= STRONG_STEEL else normalised


# Marin's surface factor `a σ_u^b` (Shigley's table, recalled and not
# checked against a copy): ground and machined.
MARIN = {"ground": (1.58, -0.085), "machined": (4.51, -0.265)}
# One Vickers number is this many MPa of indentation pressure.
HV_MPA = 9.80665


def library_steels():
    """The library's steels: name, yield, ultimate and Vickers hardness."""
    import re
    import tomllib

    lib = tomllib.loads((ROOT / "crates" / "gear-io" / "data" / "materials_default.toml").read_text())
    out = []
    for m in lib["material"]:
        if m.get("class") != "steel":
            continue
        uts = re.search(r"UTS is (\d+) MPa", m["ultimate_allowable"].get("note", ""))
        out.append(dict(name=m["name"], yield_=m["ultimate_allowable"]["value"],
                        measure=m["ultimate_measure"], uts=float(uts.group(1)) if uts else None,
                        hv=m["contact_estimate"]["hardness"]["value"]))
    return out


def _stats(v):
    v = sorted(v)
    n = len(v)
    med = v[n // 2] if n % 2 else 0.5 * (v[n // 2 - 1] + v[n // 2])
    return v[0], med, v[-1], n, sum(1 for x in v if x < 0)


def _row(label, model, v):
    """One table row: the set, the model, and the bias over it."""
    if not v:
        return f"{label} | {model} | none"
    lo, med, hi, n, under = _stats(v)
    return (f"{label} | {model} | n {n} | {100 * lo:+.1f} | {100 * med:+.1f} | {100 * hi:+.1f}"
            f" | under {round(100 * under / n)} %")


# The models a row is read under: the default, the default without its
# axial term, ISO's set, and ISO's `Y_S` continued past its `q_s = 8` clamp.
MODELS = [("db", "DB"), ("db_bending", "DB without the axial term"), ("iso", "ISO"),
          ("iso_continued", "ISO, Y_S continued past its clamp")]


def summary():
    rec = read_record()
    now = parse_cases(harness("fillet", "grid"))
    rows = []
    stale = []
    for c in now:
        k = key_of(c)
        r = rec.get(k)
        if k not in rec:
            stale.append(f"{k}: not in the record")
            continue
        if r is None:
            continue
        fp = fingerprint(c)
        moved = [name for (name, _), a, b in zip(FINGERPRINT, fp, r["fingerprint"])
                 if (a is None) != (b is None) or (a is not None and abs(a - b) > FINGERPRINT_REL * max(1.0, abs(b)))]
        if moved:
            stale.append(f"{k}: {', '.join(moved)} moved")
            continue
        peak = r["peaks"][-1]
        db, iso = c.get("db"), c.get("iso")
        row = dict(key=k, kind=c["kind"], peak=peak, conv=abs(r["peaks"][-1] / r["peaks"][-2] - 1),
                   undercut=c.get("undercut") == "true")
        if db and db.get("factor") != "none":
            row["db"] = db["factor"] / peak - 1
            row["db_bending"] = db["Y_F"] * db["correction"] / peak - 1
            row["axial_share"] = db["axial"] / db["Y_F"]
            row["fillet"] = db["rho_f"] / db["s_Fn"]
        if iso and iso.get("factor") != "none":
            row["iso"] = iso["factor"] / peak - 1
            row["q_s"] = iso["q_s"]
            l = iso["s_Fn"] / iso["h_Fe"]
            row["iso_continued"] = iso["Y_F"] * y_s(l, max(iso["q_s"], 1.0)) / peak - 1
        rows.append(row)
    if stale:
        print("the record is not of these teeth; solve them again (--run):")
        for s_ in stale:
            print("  " + s_)
        sys.exit(1)

    def sel(pred, key):
        return [r[key] for r in rows if key in r and pred(r)]

    ext = lambda r: r["kind"] == "external"  # noqa: E731
    ring = lambda r: r["kind"] == "ring"  # noqa: E731
    bands = [("every fillet", lambda r: True),
             ("ordinary fillet", lambda r: r.get("fillet", 0) >= 0.1),
             ("middle fillet", lambda r: 0.02 <= r.get("fillet", 0) < 0.1),
             ("tight fillet", lambda r: r.get("fillet", 1) < 0.02)]
    ordinary = bands[1][1]

    print("The canary: a stepped flat bar in in-plane bending, K_t = sigma_max / (6M / t d^2);")
    print("Chart 3.4's fit (Pilkey, 3rd ed.), the BEM, the Trefftz solver, and each against the Trefftz solver, %.")
    for D, r, kt in SHOULDER_TREFFTZ:
        bem = rec[("shoulder", D, r)]
        chart = peterson_shoulder_kt(D, 1.0, r)
        print(f"canary D/d {D:g} r/d {r:g} | chart {chart:.4f} | BEM {bem:.4f} | Trefftz {kt:.4f}"
              f" | BEM {100 * (bem / kt - 1):+.2f} | chart {100 * (chart / kt - 1):+.2f}")
    print()
    for kind in ("external", "ring"):
        keys = [r["key"] for r in rows if r["kind"] == kind]
        axis = lambda i: " ".join(f"{v:g}" for v in sorted({k[i] for k in keys}))  # noqa: E731
        print(f"{kind}: z {axis(1)}; alpha {axis(2)}; x {axis(3)}; tool round {axis(4)}; mate {axis(5)}")
    whole = sorted({r["key"][1] for r in rows if rec[r["key"]]["body"] == "whole"})
    print(f"the whole gear on its shaft at z {' '.join(map(str, whole))}; a sector elsewhere")
    print("fillets: ordinary rho_f/s_Fn >= 0.1, middle 0.02 to 0.1, tight below 0.02")
    print(f"record {RECORD.relative_to(ROOT)}: {len(rows)} teeth | mesh convergence, the last two passes:"
          f" median {100 * _stats([r['conv'] for r in rows])[1]:.3f} % | worst {100 * max(r['conv'] for r in rows):.3f} %"
          f" | the gate of every canary and body check (--self-test) {100 * TOL:g} %")
    print()
    print("Rated over the exact elastic peak, less one, %: set | model | teeth | min | median | max | under the peak")
    for kind, kp in (("external", ext), ("ring", ring)):
        for band, bp in bands:
            for key, model in MODELS:
                print(_row(f"bias {kind}, {band}", model,
                           sel(lambda r, kp=kp, bp=bp: kp(r) and bp(r), key)))
    for kind, kp in (("external", ext), ("ring", ring)):
        for band, bp in (("q_s below 8", lambda r: r.get("q_s", 0) < 8),
                         ("q_s 8 and above", lambda r: r.get("q_s", 0) >= 8)):
            for key, model in MODELS[2:]:
                print(_row(f"bias {kind}, {band}", model, sel(lambda r, kp=kp, bp=bp: kp(r) and bp(r), key)))
    for a in (14.5, 20.0, 25.0):
        for key, model in MODELS:
            print(_row(f"bias external, ordinary fillet at {a:g} deg", model,
                       sel(lambda r, a=a: ext(r) and ordinary(r) and r["key"][2] == a, key)))
    for z in sorted({r["key"][1] for r in rows if ext(r)}):
        print(_row(f"bias external, ordinary fillet at z {z}", "DB",
                   sel(lambda r, z=z: ext(r) and ordinary(r) and r["key"][1] == z, "db")))
    # Dolan and Broghamer's specimens held no undercut tooth.
    for label, under in (("undercut", True), ("not undercut", False)):
        for key, model in (MODELS[0], MODELS[2]):
            print(_row(f"bias external, ordinary fillet, {label}", model,
                       sel(lambda r, u=under: ext(r) and ordinary(r) and r["undercut"] == u, key)))
    shares = sel(lambda r: ext(r) and ordinary(r), "axial_share")
    print(f"the axial term's share of Y_F, external ordinary fillets: median {100 * _stats(shares)[1]:.1f} %")
    print()

    # By material: what the root feels is the peak over the notch support,
    # against the coupon's endurance times the surface's factor.
    print("By material, external ordinary fillets: rated over felt, less one, %: (1 + bias) n k_s - 1,")
    print(f"n ISO's notch support over the coupon's ({COUPON_BAR:g} mm bar), k_s the root's surface"
          " against the coupon, read as polished.")
    felt = [r for r in rows if ext(r) and ordinary(r) and "q_s" in r]
    for st in library_steels():
        rho_p = slip_layer(st["yield_"])
        line = surface_line(st)
        k_s = surface_relative(RZ_REFERENCE, line) / surface_relative(RZ_POLISHED, line)
        for label, with_ks in (("support", 1.0), (f"support and a hobbed root, Rz {RZ_REFERENCE:g}", k_s)):
            for key, model in MODELS[:3]:
                v = [(1 + r[key]) * notch_support(rho_p, r["q_s"]) * with_ks - 1 for r in felt if key in r]
                print(_row(f"{st['name']}, {label}", model, v))
        n = _stats([notch_support(rho_p, r["q_s"]) - 1 for r in felt])
        marin = " | ".join(f"Marin {fin} {a * st['uts'] ** b:.3f}" for fin, (a, b) in MARIN.items())
        print(f"factors {st['name']} | slip layer {rho_p:.4f} mm | notch support, median {100 * n[1]:.1f} %"
              f" | the coupon over a hobbed root {1 / k_s:.3f} | sigma_u {st['uts']:g} | {marin}")
    print("brass, POM and the polyamides: no support and no surface figure in ISO; the elastic-peak rows are theirs")
    print()

    # The note on the root's endurance (`docs/state.md`): what omitting the
    # surface, and the surface with the support, does to the utilisation.
    print("The coupon's endurance read as the root's, on the steels, by ISO's lines, against the coupon read"
          " as polished: the allowable used over the root's, and the rating's utilisation over the root's,"
          " less one, %.")
    for st in library_steels():
        line = surface_line(st)
        for root, rz in RZ_ROOTS:
            k = surface_relative(rz, line) / surface_relative(RZ_POLISHED, line)
            print(f"surface {st['name']} | sigma_u {st['uts']:g}, {line} | {root}, Rz {rz:g}"
                  f" | allowable {100 * (1 / k - 1):+.1f} % | utilisation {100 * (k - 1):+.1f} %")
    nitrided = list(SURFACE_LINES)[2]
    k = surface_relative(RZ_REFERENCE, nitrided) / surface_relative(RZ_POLISHED, nitrided)
    print(f"surface, {nitrided}, no material in the library | hobbed, Rz {RZ_REFERENCE:g}"
          f" | allowable {100 * (1 / k - 1):+.1f} % | utilisation {100 * (k - 1):+.1f} %")
    qs = sorted(r["q_s"] for r in felt)
    teeth = (("the least", qs[0]), ("the median", _stats(qs)[1]), ("the most", qs[-1]))
    print(f"With ISO's notch support over the coupon's, a hobbed root, Rz {RZ_REFERENCE:g}, at the external"
          " ordinary fillets' q_s:")
    for st in library_steels():
        rho_p = slip_layer(st["yield_"])
        line = surface_line(st)
        k = surface_relative(RZ_REFERENCE, line) / surface_relative(RZ_POLISHED, line)
        for bar, d in COUPON_BARS.items():
            for tooth, q in teeth if d == COUPON_BAR else teeth[1:2]:
                n = notch_support(rho_p, q, d)
                print(f"pair {st['name']} | slip layer {rho_p:.4f} mm | {bar} of {d:g} mm, gradient {2 / d:.3f} /mm"
                      f" | {tooth} q_s {q:.2f} | surface {100 * (k - 1):+.1f} % | support {100 * (n - 1):+.1f} %"
                      f" | utilisation {100 * (k * n - 1):+.1f} %")
    # A figure reduced from load to stress by ISO's rating carries ISO's
    # stress over the default's at the same load.
    both = [r for r in rows if ext(r) and ordinary(r) and "db" in r and "iso" in r]
    iso_med, db_med = _stats([r["iso"] for r in both])[1], _stats([r["db"] for r in both])[1]
    lo, med, hi, _, _ = _stats([(1 + r["iso"]) / (1 + r["db"]) - 1 for r in both])
    print(f"gear root, external ordinary fillets: over the peak, median | ISO {100 * iso_med:+.1f} %"
          f" | the default {100 * db_med:+.1f} %")
    print(f"gear root, external ordinary fillets: ISO over the default, tooth by tooth | least {100 * lo:+.1f} %"
          f" | median {100 * med:+.1f} % | most {100 * hi:+.1f} %")
    print()
    print("Hardness / 3 (Tabor) against the library's own yield and ultimate:")
    for st in library_steels():
        h3 = st["hv"] * HV_MPA / 3
        print(f"hardness {st['name']} | HV {st['hv']:g} | H/3 {h3:.0f} MPa | yield {st['yield_']:g}"
              f" | {100 * (h3 / st['yield_'] - 1):+.0f} % | ultimate {st['uts']:g} | {100 * (h3 / st['uts'] - 1):+.0f} %")


# --------------------------------------------------------------------------
# The self-test
# --------------------------------------------------------------------------

# The stepped bar by a second, independent solver with no boundary integral
# in it (`tools/shoulder_trefftz.py`, which solves them again and holds its
# answer to these): `(D/d, r/d, K_t)`.
SHOULDER_TREFFTZ = __import__("shoulder_trefftz").RECORDED
# The research round's canary pinion (17/43, 20°, rack round 0.38) as its
# instrument solved it — three teeth, a three-module rim, cuts free, two
# passes (`notch-proto/t_fillet_canary.json`, every digit).
RESEARCH_CANARY = (2.9788584744444675, 2.979591738643474)


def canaries(fault=None, quick=False):
    """Every gate's `(name, error, tolerance)`; `quick` skips the slow two."""
    out = []
    out.append(("Kirsch, K_t 3", hole(1, 1, 0.075, 0.075, 0.25, fault) / 3 - 1, TOL))
    out.append(("Inglis, a/b 3, K_t 7", hole(1, 1 / 3, 1 / 9 / 16, 0.1, 0.1, fault) / 7 - 1, TOL))
    out.append(("Golovin's curved bar, b/a 2", golovin(1.0, 2.0, 0.05, fault)[0], TOL))
    for D, r, kt in SHOULDER_TREFFTZ:
        v = shoulder(D, 1.0, r, r / 16, fault=fault)[0]
        out.append((f"shoulder D/d {D:g} r/d {r:g} against the Trefftz solver", v / kt - 1, TOL))
    if quick:
        return out
    c = crate_tooth("external", 17, 20, 0, 0.38, 43)
    old = solve_tooth(c, passes=PASSES[:2], nb=1, rim=3.0, whole=False, cuts="t", fault=fault)
    # A port may differ from its original by no more than the original
    # resolves: the change between its own two meshes.
    resolves = abs(RESEARCH_CANARY[1] / RESEARCH_CANARY[0] - 1)
    for (p, _), ref in zip(old, RESEARCH_CANARY):
        out.append(("the research instrument's canary tooth", p / ref - 1, resolves))
    # The body: each form checked where the record uses it, on a member of
    # each kind, and the two forms against each other across the switch.
    whole = [c, crate_tooth("external", 12, 25, 0.5, 0.1, 43)]
    assert all(body_of(m) == "whole" for m in whole)
    for m in whole:
        base = solve_tooth(m, fault=fault)[-1][0]
        thin = solve_tooth(m, bore=BORE / 2, fault=fault)[-1][0]
        out.append((f"the body: external z {m['z']}, whole, its shaft half as large", thin / base - 1, TOL))
    sectors = [crate_tooth("external", 60, 20, 0, 0.38, 43), crate_tooth("ring", 40, 20, 0, 0.1, 17)]
    assert all(body_of(m) == body_of(m, rim=2 * RIM) == "sector" for m in sectors)
    for m in sectors:
        base = solve_tooth(m, fault=fault)[-1][0]
        deep = solve_tooth(m, rim=2 * RIM, fault=fault)[-1][0]
        wide = solve_tooth(m, nb=3, fault=fault)[-1][0]
        z = f"{m['kind']} z {m['z']}"
        out.append((f"the body: {z}, sector, its rim twice as deep", deep / base - 1, TOL))
        out.append((f"the body: {z}, sector, seven teeth", wide / base - 1, TOL))
    # z 30 is the grid's first sector above the switch.
    near = crate_tooth("external", 30, 20, 0, 0.38, 43)
    assert body_of(near) == "sector"
    sector = solve_tooth(near, fault=fault)[-1][0]
    gear = solve_tooth(near, whole=True, fault=fault)[-1][0]
    out.append(("the body: external z 30, sector against the whole gear", sector / gear - 1, TOL))
    return out


def figures_law():
    """Every figure `docs/state.md` quotes from this tool, moved one unit in
    its last digit, fails `tools/check_figures.py`'s rule for its block: the
    tables are matched row by row against one printed line each, where a list
    or a paragraph is matched as a bag against everything printed. Returns
    (figures moved, those that still passed)."""
    import contextlib
    import io
    import re

    import check_figures as cf

    buf = io.StringIO()
    with contextlib.redirect_stdout(buf):
        summary()
    out = buf.getvalue()
    number = re.compile(r"(?<![\w./])[-−+]?\d+(?:\.\d+)?(?![\w/])")
    moved = passed = 0
    blocks = [b for b in cf.blocks(ROOT / "docs" / "state.md") if b[1] == ["tools/fillet_bem.py"]]
    for verb, commands, _, text in blocks:
        assert not cf.check_block(verb, commands, text, lambda _c: out)[0], "the document has drifted"
        for m in number.finditer(text):
            raw = m.group(0)
            dec = len(raw.split(".")[1]) if "." in raw else 0
            v = float(raw.replace("−", "-"))
            w = v + (1 if v >= 0 else -1) * 10 ** -dec
            new = (f"{w:+.{dec}f}" if raw[0] in "+−-" else f"{w:.{dec}f}").replace("-", "−")
            planted = text[:m.start()] + new + text[m.end():]
            moved += 1
            passed += not cf.check_block(verb, commands, planted, lambda _c: out)[0]
    return len(blocks), moved, passed


def self_test():
    failed = []
    tables, moved, passed = figures_law()
    print(f"  {'ok  ' if moved and not passed else 'FAIL'} state.md's {tables} tables from this tool:"
          f" {moved} figures each moved one unit, {passed} still pass")
    if not moved or passed:
        failed.append("a figure state.md quotes moves unseen")
    gates = canaries()
    for name, err, tol in gates:
        ok = abs(err) <= tol
        print(f"  {'ok  ' if ok else 'FAIL'} {name}: {err:+.2e} (tolerance {tol:g})")
        if not ok:
            failed.append(name)
    assert len(gates) == 3 + len(SHOULDER_TREFFTZ) + 2 + 2 + 4 + 1, len(gates)
    print("Peterson's Chart 3.4, the pre-registered canary, reported: the chart's fit against the two solvers")
    for D, r, kt in SHOULDER_TREFFTZ:
        chart = peterson_shoulder_kt(D, 1.0, r)
        print(f"  D/d {D:g} r/d {r:g} h/r {(D - 1) / 2 / r:g}: chart {chart:.4f}, solvers {kt:.4f}: "
              f"the chart {100 * (chart / kt - 1):+.2f} %")
    # The planted faults: each fails a gate, and the named near-misses pass
    # what a weaker gate would have been.
    plants = 0
    for fault, near_miss in (("held", "the hole canaries"), ("strain", None), ("kernel", None),
                             ("tenfold", None)):
        res = canaries(fault, quick=True)
        caught = [n for n, e, t in res if abs(e) > t]
        if not caught:
            failed.append(f"planted {fault} not caught")
        if near_miss:
            holes_pass = all(abs(e) <= t for n, e, t in res[:2])
            if not holes_pass:
                failed.append(f"planted {fault} was meant to pass {near_miss}")
        print(f"  planted {fault}: caught by {len(caught)} of {len(res)}"
              + (f"; passes {near_miss}, as a gate of those alone would" if near_miss else ""))
        plants += 1
    res = canaries("tenth", quick=True)
    if any(abs(e) > t for n, e, t in res):
        failed.append("a rounding-level fault fails a gate")
    print(f"  planted tenth (a tenth of the tolerance): passes all {len(res)}")
    plants += 1
    # A self-convergence gate passes `strain`: two meshes agree.
    a = shoulder(1.5, 1.0, 0.1, 0.1 / 16, fault="strain")[0]
    b = shoulder(1.5, 1.0, 0.1, 0.1 / 32, fault="strain")[0]
    if abs(a / b - 1) > TOL:
        failed.append("strain was meant to converge")
    print(f"  planted strain converges (two meshes {100 * (a / b - 1):+.3f} %), so only an exact canary catches it")
    assert plants == 5, plants
    if failed:
        print("FAILED: " + "; ".join(failed))
        sys.exit(1)
    print("ok")


if __name__ == "__main__":
    args = sys.argv[1:]
    if args[:1] == ["--run"]:
        run(int(args[1]) if len(args) > 1 else 8)
    elif args[:1] == ["--self-test"]:
        self_test()
    elif args[:1] == ["--tooth"]:
        c = crate_tooth(*args[1:7])
        for (k, g), (p, n) in zip(PASSES, solve_tooth(c)):
            print(f"rho_ref/{k} growth {g}: peak {p:.6f} ({n} elements)")
        for s_ in ("db", "iso"):
            print(s_, c.get(s_))
    else:
        summary()

