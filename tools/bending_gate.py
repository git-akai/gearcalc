#!/usr/bin/env python3
"""The default bending rating, rebuilt from the tooth's proportions and held to
what the crate recorded.

**A gate, run in CI**, on `tools/golden/bendgrid.txt` (`gear-cli bendgrid`,
which `tools/check_golden.sh` holds to the harness):

    python3 tools/bending_gate.py                 # every recorded row
    python3 tools/bending_gate.py --record FILE   # ...of another recording
    python3 tools/bending_gate.py --self-test     # the planted faults

# What it rebuilds

Each row is a tooth of `z` teeth, normal pressure angle `α`, helix `β`, shift
`x`, addendum, dedendum, rack tip round and thickness factor, rated on its
**virtual spur gear** (`z_n = z / cos³β`, module `m_n`, the same rack: the
count of ISO 6336-3:2006, which is the model's convention) at a stated contact
ratio, as a train rates a member:

- **the tooth**, generated here. The flank is the involute of `r_b = r cos α`
  whose half angle at the pitch circle is `s / 2r`. The fillet is the envelope
  of the rack's tip round as the rack rolls on the pitch circle, each point
  the one where the round's normal passes through the pitch point. They meet
  at the round's flank tangency (closed form) or, on an undercut tooth, where
  they cross (bisection); a tooth undercut past its centreline has none. The
  tip is capped where the flanks meet.
- **the load**, at the highest point of single-pair contact, `ε_αn − 1` base
  pitches back from the tip (`ε_αn = ε_α / cos²β_b`), or from where contact
  ends `short` base pitches below it, along the involute's normal.
- **the Lewis section**, by brute force: the largest parabola `x² = 4p(y_v −
  y)`, vertex where the load line crosses the centreline, that the tooth
  contains is the least of `x² / (y_v − y)` over both curves: sampled, then
  refined by bisection on its derivative, taken by complex step.
- **Dolan and Broghamer as AGMA fits them** (AGMA 908-B89; Mitchiner and
  Mabie, 1982): `(Y_F − axial) · K_f`, `K_f = H + (s_Fn/ρ_f)^L (s_Fn/h_Fe)^M`
  with `H, L, M` linear in `α` in radians, and `ρ_f` the fillet's least radius
  of curvature, `ρ + b_c² / (r + b_c)` at its root (the round's centre, a
  point `b_c` below a line rolling on the pitch circle, turns there on
  `b_c² / (r + b_c)`; the fillet is that path offset by the round). No
  reading where the factor is not a positive number.
- **under the ramp**, the section found once at that point and the load moved
  on it round the cycle; the rating is the greatest `factor · share` over the
  cycle's samples: the four seeded points and 201 even ones, which is how the
  rating defines its maximum.
- **the stress**, `F_t / (b m_n) · factor · share`, `F_t = 2000 T / d`.

Nothing here reads `gear_core`. The harness's own intermediate figures (the
tool, the virtual count) are compared and never used, with one exception:
where the harness says it capped the tip round asked for, the round it cut
with is held below the largest that fits and then taken, as the tool the
tooth was cut by.

# The tolerance

The tool and the virtual count are printed to the bit; every other figure
`{:.12e}`, so it carries half a unit in its last digit. Both sides compute it from the same inputs in floating point, each
operation within `ε/2`, and `OPS = 2⁸` operations bound what either spends on
one figure. One step is not a sum of roundings: the tangency, which both sides
find to the rounding of its condition, so a length read there moves by the
tangency's conditioning `κ` (the condition's terms over its slope, times how
fast the lengths move with the parameter, computed per row). Each figure is
held to its printing plus `OPS·ε` of itself, `(1 + κ)` of that at the
tangency, and what those carry through the load angle (`cot α_Fen`, the
arccosine's slope), `K_f`, `Y_F − axial` and the stress, propagated term by
term. `--self-test` holds the two laws of a tolerance: a record differing from
this model by rounding passes, and a figure moved ten times its tolerance
fails.
"""

import cmath
import dataclasses
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RECORD = ROOT / "tools" / "golden" / "bendgrid.txt"

EPS = sys.float_info.epsilon
# Operations either side spends between the inputs and one figure, as a power
# of two: a tooth, a section and a load point take a few hundred.
OPS = 2.0**8
# The harness prints `{:.12e}`: twelve digits after the point.
DIGITS = 12
TORQUE, FACE, MODULE = 10.0, 10.0, 1.0

# The grid, as `crates/gear-cli/src/bendgrid.rs` states it: each block every
# combination of its lists, in this order, each row under both sharings.
BLOCKS = [
    dict(
        teeth=[9, 12, 17, 25, 40, 70, 150, 400],
        alpha=[14.5, 20.0, 25.0],
        shift=[-0.3, 0.0, 0.5],
        helix=[0.0, 15.0, 30.0],
        proportions=[(1.0, 1.25, 0.38, 1.0)],
        eps_n=[1.3, 1.7, 2.3],
        short=[0.0],
    ),
    dict(
        teeth=[12, 25],
        alpha=[20.0],
        shift=[0.0, 0.3],
        helix=[0.0, 20.0],
        proportions=[(1.25, 1.5, 0.25, 1.0), (1.0, 1.25, 0.2, 0.96)],
        eps_n=[1.5, 2.2],
        short=[0.0, 0.2],
    ),
]
SHARINGS = ["none", "ramp"]

# The ramp: a third of the load at either end of the cycle, two thirds where
# the single-pair zone begins, all of it inside that zone.
RAMP = (1.0 / 3.0, 2.0 / 3.0)


@dataclasses.dataclass(frozen=True)
class Model:
    """The rating's conventions. The defaults are the rating; `--self-test`
    plants a fault by changing one."""

    # Even samples of the sweep under the ramp.
    sweep: int = 200
    # A section searched afresh at every load point, not held.
    afresh: bool = False
    # Scale on AGMA's `H` and `L`.
    h_scale: float = 1.0
    l_scale: float = 1.0
    # `ρ_F`, the fillet's radius at the section, in place of its least `ρ_f`.
    notch_at_section: bool = False
    # ISO 6336-3:2019's virtual count `z / (cos²β_b cos β)` for `z / cos³β`.
    virtual_2019: bool = False


RATING = Model()


def inv(a):
    return math.tan(a) - a


def db_coefficients(alpha, model=RATING):
    """`H, L, M` of AGMA's fit to Dolan and Broghamer, `α` in radians."""
    return (
        (0.331 - 0.436 * alpha) * model.h_scale,
        (0.324 - 0.492 * alpha) * model.l_scale,
        0.261 + 0.545 * alpha,
    )


def bisect(f, lo, hi):
    """A root of `f` in `[lo, hi]`, whose ends `f` takes with opposite signs,
    halved until the bracket cannot shrink (a double has 2¹¹ + 52 halvings of
    any finite bracket in it, so the bound is never what stops it)."""
    flo = f(lo)
    for _ in range(2100):
        mid = 0.5 * (lo + hi)
        if mid in (lo, hi):
            return mid
        fm = f(mid)
        if (fm < 0) == (flo < 0):
            lo, flo = mid, fm
        else:
            hi = mid
    raise AssertionError("a bisection did not close")


def slope(f, p):
    """`df/dp` by complex step: exact to rounding, since no difference of
    nearby values is taken."""
    h = 1e-30
    return f(complex(p, h)).imag / h


class Tooth:
    """A rack-cut tooth, `y` along its centreline, `x` across it."""

    def __init__(self, z, m, alpha, x, h_a, h_f, rho, k):
        self.z, self.m, self.alpha = z, m, alpha
        ta = math.tan(alpha)
        self.r = m * z / 2
        self.rb = self.r * math.cos(alpha)
        x_s = math.pi * (k - 1) / (4 * ta)
        self.s = m * (math.pi / 2 + 2 * (x + x_s) * ta)
        self.psi_b = self.s / (2 * self.r) + inv(alpha)
        # The rack's tip, `b_d` below the rolling line, and its round, whose
        # centre is `b_c` below it and `a_c` across from the tooth's centre.
        self.b_d = m * (h_f - x)
        self.rho = rho
        self.b_c = self.b_d - rho
        self.a_c = self.s / 2 + self.b_c * ta + rho / math.cos(alpha)
        # The largest round the rack's tip holds between its flanks.
        w_tip = math.pi * m - self.s - 2 * self.b_d * ta
        self.rho_fit = max(w_tip, 0.0) * math.cos(alpha) / (2 * (1 - math.sin(alpha)))
        r_a = self.r + m * (h_a + x)
        if r_a > self.rb and self.psi_b - inv(math.acos(self.rb / r_a)) <= 0:
            u = bisect(lambda u: self.psi_b - (u - math.atan(u)), 0.0, 50.0)
            r_a = self.rb * math.hypot(1, u)
        self.r_a = r_a
        self.u_tip = math.sqrt(max((r_a / self.rb) ** 2 - 1, 0.0))
        self.u_tip_scale = self.roll_scale(r_a, self.u_tip)
        self.junction()

    def roll_scale(self, rad, u):
        """What a roll `√((R/r_b)² − 1)` is the rounding of: `(R/r_b)²/u`
        times that of `R`, the square root's slope near the base circle."""
        return (rad / self.rb) ** 2 / max(u, sys.float_info.min) + u

    def fillet(self, sig):
        """The fillet at rack travel `sig` from the root, rising as `sig`
        falls: the pitch point plus `(1 + ρ/|CP|)` of the vector to the
        round's centre, in the rolled rack's frame."""
        phi = (self.a_c - sig) / self.r
        kk = 1 + self.rho / cmath.sqrt(sig * sig + self.b_c * self.b_c)
        sn, cs = cmath.sin(phi), cmath.cos(phi)
        return (
            self.r * sn + kk * (sig * cs - self.b_c * sn),
            self.r * cs - kk * (sig * sn + self.b_c * cs),
        )

    def flank(self, u):
        """The involute at roll `u`."""
        rad = self.rb * cmath.sqrt(1 + u * u)
        th = self.psi_b - (u - cmath.atan(u))
        return rad * cmath.sin(th), rad * cmath.cos(th)

    def polar(self, sig):
        x, y = self.fillet(sig)
        return math.hypot(x.real, y.real), math.atan2(x.real, y.real)

    def junction(self):
        """Where the fillet hands over to the flank (`sig_j`, `u_j`), and
        whether a flank is left to load."""
        ta, sa = math.tan(self.alpha), math.sin(self.alpha)
        # The round's flank tangency is generated `l` along the line of action
        # from the base tangent point; short of it, the tooth is undercut.
        l = self.r * sa - (self.b_c + self.rho * sa) / sa
        undercut = l < 0
        if not undercut:
            self.sig_j, self.u_j = -self.b_c / ta, l / self.rb
            # A difference of the two terms of `l`: the rounding of the larger.
            self.u_j_scale = (self.r * sa + (self.b_c + self.rho * sa) / sa) / self.rb
        else:
            # The radius falls as the travel rises to the root, `|sig| ≤ r`.
            def travel(rad):
                return bisect(lambda s: self.polar(s)[0] - rad, -rad, 0.0)

            def gap(s):
                rad, th = self.polar(s)
                return th - (self.psi_b - inv(math.acos(min(self.rb / rad, 1.0))))

            s_tip = travel(self.r_a)
            if self.r_a <= self.rb or gap(s_tip) <= 0:
                self.sig_j, self.u_j = s_tip, self.u_tip
            else:
                s_base = travel(self.rb)
                self.sig_j = s_base if gap(s_base) >= 0 else bisect(gap, s_tip, s_base)
                rad = self.polar(self.sig_j)[0]
                self.u_j = math.sqrt(max((rad / self.rb) ** 2 - 1, 0.0))
            self.u_j_scale = self.roll_scale(self.rb * math.hypot(1, self.u_j), self.u_j)
        self.usable = self.u_j < self.u_tip
        if self.usable and undercut:
            # Severed: the fillet crosses the centreline below the junction.
            n = 400
            self.usable = min(self.polar(self.sig_j * i / n)[1] for i in range(n + 1)) > 0

    def least_fillet_radius(self):
        return self.rho + self.b_c**2 / (self.r + self.b_c)

    def fillet_radius_at(self, sig):
        """The fillet's radius of curvature at `sig`: the complex step's
        exact first derivatives, differenced once for the second."""
        def d1(q):
            return [slope(lambda t: self.fillet(t)[i], q) for i in (0, 1)]

        h = 1e-6 * max(abs(self.sig_j), 1e-3)
        (xs, ys), (xa, ya), (xb, yb) = d1(sig), d1(sig + h), d1(sig - h)
        xss, yss = (xa - xb) / (2 * h), (ya - yb) / (2 * h)
        return (xs * xs + ys * ys) ** 1.5 / abs(xs * yss - ys * xss)

    def load(self, u):
        """Load point and unit direction at roll `u`: along the involute's
        normal, the line to its base tangent point."""
        x, y = (c.real for c in self.flank(u))
        t = self.psi_b - u
        tx, ty = self.rb * math.sin(t), self.rb * math.cos(t)
        n = math.hypot(x - tx, y - ty)
        return (x, y), ((x - tx) / n, (y - ty) / n)

    def section(self, y_v):
        """The Lewis tangency for a vertex at `y_v`, `(curve name, curve,
        parameter)`: the least of `x²/(y_v − y)` over the fillet and the
        flank, which is the largest parabola the tooth contains."""
        best, ends = None, []
        for name, curve, lo, hi in (
            ("fillet", self.fillet, self.sig_j, 0.0),
            ("flank", self.flank, self.u_j, self.u_tip),
        ):
            def g(p, curve=curve):
                x, y = curve(p)
                return x * x / (y_v - y)

            n = 600
            ps = [lo + (hi - lo) * i / n for i in range(n + 1)]
            vals = []
            for p in ps:
                x, y = curve(p)
                vals.append(g(p).real if y.real < y_v and x.real > 0 else math.inf)
            ends += [v for v in (vals[0], vals[n]) if math.isfinite(v)]
            for i in range(1, n):
                if not math.isfinite(vals[i]) or vals[i] > min(vals[i - 1], vals[i + 1]):
                    continue
                if not slope(g, ps[i - 1]) < 0 < slope(g, ps[i + 1]):
                    continue
                p = bisect(lambda q: slope(g, q), ps[i - 1], ps[i + 1])
                if best is None or g(p).real < best[0]:
                    best = (g(p).real, name, curve, p)
        # A least at a curve's end is a corner, where the crate's search for
        # a stationary point has no reading: a row the gate cannot hold, and
        # it says so rather than compare. (The two curves' shared end at a
        # tangent junction is the least only to rounding.)
        if ends and (best is None or min(ends) < best[0] * (1 - OPS * EPS)):
            raise AssertionError(f"the least is at a curve's end, vertex {y_v}")
        return None if best is None else best[1:]


def conditioning(curve, p, y_v):
    """How far the rounding of the tangency condition `x y′ + 2x′(y_v − y)`
    moves `s_Fn` and `h_Fe`, relatively, per unit of relative rounding."""
    def cond(q):
        x, y = (c.real for c in curve(q))
        xs, ys = (slope(lambda t: curve(t)[i], q) for i in (0, 1))
        return x * ys + 2 * xs * (y_v - y), abs(x * ys) + abs(2 * xs * (y_v - y)), x, y, xs, ys

    _, terms, x, y, xs, ys = cond(p)
    step = 1e-7 * max(abs(p), 1e-3)
    dc = (cond(p + step)[0] - cond(p - step)[0]) / (2 * step)
    return terms / abs(dc) * (abs(xs) / x + abs(ys) / (y_v - y))


def figures(t, section, load_point, direction, y_v, model):
    """Every figure of a section with the load at `load_point`."""
    name, curve, p = section
    xt, yt = (c.real for c in curve(p))
    s, h = 2 * xt, y_v - yt
    af = math.atan2(abs(direction[1]), abs(direction[0]))
    m, a = t.m, t.alpha
    y_f = 6 * (h / m) * math.cos(af) / ((s / m) ** 2 * math.cos(a))
    axial = math.sin(af) / ((s / m) * math.cos(a))
    if model.notch_at_section:
        rho_f = t.fillet_radius_at(p if name == "fillet" else t.sig_j)
    else:
        rho_f = t.least_fillet_radius()
    H, L, M = db_coefficients(a, model)
    k_f = H + (s / rho_f) ** L * (s / h) ** M if h > 0 else None
    factor = (y_f - axial) * k_f if k_f is not None else None
    if factor is not None and not (math.isfinite(factor) and factor > 0):
        factor = None
    return dict(
        on=name, s_Fn=s, h_Fe=h, alpha_Fen=af, load_x=load_point[0], load_y=load_point[1],
        y_t=yt, rho_f=rho_f, Y_F=y_f, axial=axial, K_f=k_f, factor=factor,
    )


def ramp(d, eps):
    """The share at `d` base pitches back from the tip of a cycle `eps` long."""
    if eps - 1 <= d <= 1:
        return 1.0
    t = min(d, eps - d) / max(eps - 1, sys.float_info.min)
    return RAMP[0] + (RAMP[1] - RAMP[0]) * min(max(t, 0.0), 1.0)


def rate(row, model=RATING, rho_tool=None):
    """`(tooth, z_n, figures)` of a row, the figures `None` where the model
    has no reading. `rho_tool` is the round the harness cut with where it
    capped the one asked for."""
    z, m = row["z"], MODULE
    alpha, beta = math.radians(row["alpha"]), math.radians(row["beta"])
    sin_bb = math.sin(beta) * math.cos(alpha)
    cos2_bb = 1 - sin_bb * sin_bb
    z_n = z / (cos2_bb * math.cos(beta)) if model.virtual_2019 else z / math.cos(beta) ** 3
    rho = rho_tool if rho_tool is not None else row["rho"] * m
    t = Tooth(z_n, m, alpha, row["x"], row["h_a"], row["h_f"], rho, row["k"])
    if not t.usable:
        return t, z_n, None
    eps_n = row["eps_a"] / cos2_bb
    pitch = 2 * math.pi / t.z
    u_end = t.u_tip - row["short"] / cos2_bb * pitch

    def loaded(d):
        """Load point, direction and vertex `d` base pitches back, if on the
        flank."""
        u = u_end - d * pitch
        if not t.u_j <= u <= t.u_tip:
            return None
        (lx, ly), (dx, dy) = t.load(u)
        return (lx, ly), (dx, dy), ly - lx * dy / dx

    hp = max(eps_n - 1, 0.0)
    at = loaded(hp)
    if at is None:
        return t, z_n, None
    sec = t.section(at[2])
    if sec is None:
        return t, z_n, None
    f = figures(t, sec, *at, model)
    if f["factor"] is None:
        return t, z_n, None
    share, d_best = 1.0, hp
    if row["sharing"] == "ramp":
        ds = [0.0, eps_n, hp, min(eps_n, 1.0)] + [i / model.sweep * eps_n for i in range(model.sweep + 1)]
        best = None
        for d in ds:
            there = loaded(d)
            if there is None:
                continue
            s2 = t.section(there[2]) if model.afresh else sec
            f2 = figures(t, s2, *there, model) if s2 else None
            if f2 is None or f2["factor"] is None:
                continue
            w = f2["factor"] * ramp(d, eps_n)
            if best is None or w > best[0]:
                best = (w, f2, ramp(d, eps_n), d)
        _, f, share, d_best = best
    force = 1000 * TORQUE / (z * m / math.cos(beta) / 2)
    f = dict(
        f,
        share=share,
        sigma_F=force / (FACE * m) * f["factor"] * share,
        kappa=conditioning(sec[1], sec[2], at[2]),
        alpha=alpha,
        moved=d_best != hp,
    )
    return t, z_n, f


FIGURES = ["s_Fn", "h_Fe", "alpha_Fen", "load_x", "load_y", "y_t", "rho_f", "Y_F", "axial",
           "K_f", "factor", "share", "sigma_F"]


def printed(v):
    """Half a unit in the last digit `{:.12e}` prints of `v`."""
    return 0.5 * 10.0 ** (math.floor(math.log10(abs(v))) - DIGITS) if v else 0.0


def form(t, z_n):
    """The figures of form a row records beside its rating."""
    return dict(z_n=z_n, rho_mm=t.rho, b_d=t.b_d, u_j=t.u_j, u_tip=t.u_tip)


def form_tolerances(t, z_n):
    """What each figure of form may differ by: the tool and the count are
    printed to the bit, the rolls to 13 digits, each roll carrying what its
    own arithmetic amplifies."""
    f = form(t, z_n)
    tol = {k: OPS * EPS * abs(f[k]) for k in ("z_n", "rho_mm", "b_d")}
    tol["u_j"] = printed(t.u_j) + OPS * EPS * t.u_j_scale
    tol["u_tip"] = printed(t.u_tip) + OPS * EPS * t.u_tip_scale
    return tol


def tolerances(f):
    """What each figure may differ by, absolutely, before its printing."""
    e = OPS * EPS
    at = e * (1 + f["kappa"])
    tol = {k: e * abs(f[k]) for k in ("load_x", "load_y", "rho_f", "share")}
    for k in ("s_Fn", "h_Fe", "y_t"):
        tol[k] = at * abs(f[k])
    # The crate reads the load's angle as an arccosine, whose slope is
    # `1/sin`: `cot α_Fen` times the rounding of the cosine.
    af = f["alpha_Fen"]
    tol["alpha_Fen"] = e * (af + 1 / math.tan(af))
    # `Y_F` is the least of the ratio the tangency minimises, so stationary in
    # it; it reads the angle through its cosine, the axial term through its
    # sine and `s_Fn`.
    tol["Y_F"] = (e + math.tan(af) * tol["alpha_Fen"]) * abs(f["Y_F"])
    tol["axial"] = (e + at + tol["alpha_Fen"] / math.tan(af)) * abs(f["axial"])
    H, L, M = db_coefficients(f["alpha"])
    tol["K_f"] = e * abs(f["K_f"]) + abs(f["K_f"] - H) * ((abs(L) + 2 * abs(M)) * at + e)
    diff = f["Y_F"] - f["axial"]
    tol["factor"] = e * abs(f["factor"]) + f["K_f"] * (tol["Y_F"] + tol["axial"]) + abs(diff) * tol["K_f"]
    tol["sigma_F"] = e * abs(f["sigma_F"]) + f["sigma_F"] / f["factor"] * tol["factor"]
    return {k: tol[k] + printed(f[k]) for k in tol}


# ------------------------------------------------------------ the record ----


def parse(text):
    """`(header, rows, count)` of a recorded `gear-cli bendgrid`."""
    lines = text.strip().split("\n")
    head = lines[0].split()
    header = dict(zip(head[0::2], head[1::2]))
    rows = []
    for line in lines[1:-1]:
        given, tool, rated = (part.split() for part in line.split(" | "))
        r = dict(zip(given[0::2], given[1::2]))
        row = {k: (v if k == "sharing" else float(v)) for k, v in r.items()}
        row["z"] = int(r["z"])
        row["tool"] = dict(zip(tool[0::2], tool[1::2]))
        row["rated"] = None if rated == ["none"] else dict(zip(rated[0::2], rated[1::2]))
        rows.append(row)
    tail = lines[-1].split()
    return header, rows, int(tail[1])


def grid():
    """Every row's stated inputs, in the harness's order."""
    out = []
    for b in BLOCKS:
        for z in b["teeth"]:
            for a in b["alpha"]:
                for x in b["shift"]:
                    for be in b["helix"]:
                        for (ha, hf, rho, k) in b["proportions"]:
                            for e in b["eps_n"]:
                                for sh in b["short"]:
                                    for s in SHARINGS:
                                        out.append((z, a, be, x, ha, hf, rho, k, e, sh, s))
    return out


def the_grid(header, rows, count):
    """Faults in the record's shape: not the grid, not every row, a contact
    ratio not the one stated."""
    bad = []
    if header != {"torque": "10", "face": "10", "module": "1"}:
        bad.append(f"the record's header is {header}")
    stated = grid()
    if count != len(rows) or len(rows) != len(stated):
        return bad + [f"the record has {len(rows)} rows (it says {count}); the grid has {len(stated)}"]
    for i, (row, want) in enumerate(zip(rows, stated)):
        have = (row["z"], row["alpha"], row["beta"], row["x"], row["h_a"], row["h_f"], row["rho"], row["k"])
        if have != want[:8] or row["short"] != want[9] or row["sharing"] != want[10]:
            bad.append(f"row {i} is {have}, the grid's {want}")
            continue
        a, b = math.radians(row["alpha"]), math.radians(row["beta"])
        eps_n = row["eps_a"] / (1 - (math.sin(b) * math.cos(a)) ** 2)
        if abs(eps_n - want[8]) > OPS * EPS * want[8]:
            bad.append(f"row {i}: eps_a {row['eps_a']} is eps_n {eps_n}, the grid's {want[8]}")
    return bad


def rebuild(rows, model=RATING):
    """This model's answer for every row: `(tooth, z_n, figures)`, the round
    the harness cut with taken where it says it capped the one asked for."""
    out = []
    for row in rows:
        capped = "clamp.fillet_capped" in row["tool"]["clamps"].split(",")
        out.append(rate(row, model, float(row["tool"]["rho_mm"]) if capped else None))
    return out


def check(rows, built):
    """Each recorded row against its rebuild: `(faults, figures compared,
    rows rated)`."""
    bad, compared, rated = [], 0, 0
    for i, (row, (t, z_n, f)) in enumerate(zip(rows, built)):
        tool = row["tool"]
        capped = "clamp.fillet_capped" in tool["clamps"].split(",")
        # The tool, the virtual count and where the flank starts and ends:
        # compared, never used.
        for key, tol in form_tolerances(t, z_n).items():
            compared += 1
            if abs(float(tool[key]) - form(t, z_n)[key]) > tol:
                bad.append(f"row {i}: {key} {tool[key]}, here {form(t, z_n)[key]!r}")
        asked = row["rho"] * MODULE
        if capped and not (t.rho < asked and t.rho <= t.rho_fit):
            bad.append(f"row {i}: the capped round {t.rho} is not below both {asked} and the fit {t.rho_fit}")
        got = row["rated"]
        if (got is None) != (f is None):
            bad.append(f"row {i}: the crate {'has no reading' if got is None else 'rates'}, here {'none' if f is None else 'a rating'}")
            continue
        if f is None:
            continue
        rated += 1
        tol = tolerances(f)
        if got["on"] != f["on"]:
            bad.append(f"row {i}: the tangency is on the {got['on']}, here on the {f['on']}")
        for key in FIGURES:
            compared += 1
            theirs = None if got[key] == "none" else float(got[key])
            if theirs is None or abs(theirs - f[key]) > tol[key]:
                bad.append(f"row {i}: {key} {got[key]}, here {f[key]!r} (tolerance {tol[key]:.2e})")
    return bad, compared, rated


def gate(header, rows, count, built):
    """Every fault in a record, with what was compared."""
    bad = the_grid(header, rows, count)
    if bad:
        return bad, 0, 0
    return check(rows, built)


def main_check(path):
    header, rows, count = parse(Path(path).read_text())
    built = rebuild(rows)
    bad, compared, rated = gate(header, rows, count, built)
    for b in bad[:40]:
        print(b)
    if len(bad) > 40:
        print(f"... and {len(bad) - 40} more")
    # Every loop above ran as often as the record is long: every row its five
    # figures of form, every rated row its figures.
    expected = 5 * len(rows) + len(FIGURES) * rated
    if not bad and compared != expected:
        bad.append(f"compared {compared} figures, the record has {expected}")
    moved = sum(1 for _, _, f in built if f and f["moved"])
    on = {n: sum(1 for _, _, f in built if f and f["on"] == n) for n in ("fillet", "flank")}
    print(
        f"{len(rows)} rows, {rated} rated ({on['fillet']} on the fillet, {on['flank']} on the flank, "
        f"{moved} rated off the single-pair point under the ramp), {compared} figures compared: "
        + ("all within tolerance" if not bad else f"{len(bad)} fault(s)")
    )
    # The held section parts from one searched afresh only where the ramp's
    # maximum leaves the single-pair point; a grid with none cannot see that.
    if moved == 0:
        bad.append("no row is rated off the single-pair point: the held section is untested")
    # ...and the section's two curves are two searches; a grid touching one
    # tests one.
    for name, n in on.items():
        if n == 0:
            bad.append(f"no section is on the {name}: that search is untested")
    return 1 if bad else 0


# --------------------------------------------------------- the self-test ----


def as_printed(v):
    return "none" if v is None else f"{v:.12e}"


def recorded(rows, built):
    """A record this model would pass by construction: every figure as the
    harness prints it."""
    out = []
    for row, (t, z_n, f) in zip(rows, built):
        r = dict(row, rated=None if f is None else dict(
            {k: as_printed(f[k]) for k in FIGURES}, on=f["on"]))
        out.append(r)
    return out


def self_test():
    """The gate's own plants, each at or near its blind spot."""
    header, rows, count = parse(RECORD.read_text())
    built = rebuild(rows)
    rated = [i for i, (_, _, f) in enumerate(built) if f]
    results = []

    def expect(name, want_fail, planted_rows=rows, planted_count=count):
        bad, _, _ = gate(header, planted_rows, planted_count, built)
        ok = bool(bad) == want_fail
        results.append(ok)
        print(f"{'ok    ' if ok else 'WRONG '} {'fails' if bad else 'passes'}  {name}"
              + (f"  ({len(bad)}: {bad[0]})" if bad else ""))

    # The first law of a tolerance: a record differing from this model by
    # rounding passes. Each figure moved by `OPS/4` roundings of itself,
    # alternately up and down, then printed.
    base = recorded(rows, built)
    jitter = []
    for n, (row, (_, _, f)) in enumerate(zip(rows, built)):
        r = dict(row)
        if f is not None:
            sign = 1 if n % 2 else -1
            r["rated"] = dict(r["rated"], **{
                k: as_printed(f[k] * (1 + sign * OPS / 4 * EPS)) for k in FIGURES})
        jitter.append(r)
    expect("this model's own figures, printed", False, base)
    expect("...each moved by a quarter of the operations' rounding", False, jitter)

    # The second: one figure moved ten times its tolerance, figure by figure,
    # on the rated row where that figure's tolerance is widest relative to it.
    for key in FIGURES:
        i = max(rated, key=lambda i: tolerances(built[i][2])[key] / abs(built[i][2][key]))
        f = built[i][2]
        planted = [dict(r) for r in base]
        planted[i] = dict(planted[i], rated=dict(planted[i]["rated"], **{
            key: as_printed(f[key] + 10 * tolerances(f)[key])}))
        expect(f"{key} of row {i} ten times its tolerance off", True, planted)

    # ...and each figure of form, on the row where its tolerance is widest.
    for key in form(*built[0][:2]):
        i = max(range(len(rows)), key=lambda i: form_tolerances(*built[i][:2])[key] / (abs(form(*built[i][:2])[key]) or 1))
        planted = [dict(r) for r in rows]
        mine = form(*built[i][:2])[key] + 10 * form_tolerances(*built[i][:2])[key]
        planted[i] = dict(planted[i], tool=dict(planted[i]["tool"], **{key: repr(mine)}))
        expect(f"{key} of row {i} ten times its tolerance off", True, planted)

    # Faults in the crate, each what that crate would print. Every one is a
    # near miss of some part of the gate:
    def planted_by(model, only=None):
        """The record a crate with `model`'s conventions would print; `only`
        limits the rows rebuilt (the rest print as the rating does)."""
        other = [rate(row, model, float(row["tool"]["rho_mm"])
                      if "clamp.fillet_capped" in row["tool"]["clamps"] else None)
                 if (only is None or i in only) else built[i]
                 for i, row in enumerate(rows)]
        return recorded(rows, other)

    # A section searched afresh at each load point under the ramp: every
    # unshared row and every row whose maximum stays at the single-pair point
    # is unchanged, so a gate of the unshared rating alone passes it.
    moved = [i for i in rated if built[i][2]["moved"]][:4]
    expect(f"the section searched afresh under the ramp (rows {moved})", True,
           planted_by(Model(afresh=True), set(moved)))
    # `H` and `L` each off by ten times the tolerance of the factor on the
    # row the coefficient moves least.
    for coeff in ("h", "l"):
        def moves(i):
            f = built[i][2]
            H, L, _ = db_coefficients(f["alpha"])
            part = f["K_f"] - H
            dk = H if coeff == "h" else part * abs(L * math.log(f["s_Fn"] / f["rho_f"]))
            return dk * (f["Y_F"] - f["axial"])

        i = min((i for i in rated if moves(i) > 0), key=lambda i: moves(i) / tolerances(built[i][2])["factor"])
        scale = 1 + 10 * tolerances(built[i][2])["factor"] / moves(i)
        model = Model(**{f"{coeff}_scale": scale})
        expect(f"{coeff.upper()} scaled by 1 + {scale - 1:.1e} (ten tolerances on row {i})", True,
               planted_by(model, {i}))
    # The notch radius at the section for the fillet's least: the other
    # model's radius, as feeding `Y_S`'s `ρ_F` to Dolan and Broghamer would.
    expect("the notch radius read at the section", True, planted_by(Model(notch_at_section=True), set(rated[:3])))
    # The sweep's even samples one fewer: only rows rated off the single-pair
    # point under the ramp move.
    expect("the sweep sampled 199 times", True, planted_by(Model(sweep=199), set(moved)))
    # ISO 2019's virtual count: every spur row is unchanged.
    helical = [i for i in rated if rows[i]["beta"]][:3]
    expect("the 2019 virtual count", True, planted_by(Model(virtual_2019=True), set(helical)))
    # The record's shape.
    expect("a row missing", True, rows[:-1], count - 1)
    shifted = [dict(r) for r in rows]
    shifted[5] = dict(shifted[5], eps_a=shifted[5]["eps_a"] * (1 + 1e-9))
    expect("a contact ratio a billionth off the grid's", True, shifted)
    none = [dict(r) for r in base]
    none[rated[0]] = dict(none[rated[0]], rated=None)
    expect("a rated row recorded as having no reading", True, none)

    bad = results.count(False)
    print("every plant behaves" if not bad else f"{bad} plant(s) misbehave")
    return 1 if bad else 0


def main():
    args = sys.argv[1:]
    if args == ["--self-test"]:
        return self_test()
    if args[:1] == ["--record"] and len(args) == 2:
        return main_check(args[1])
    if args:
        sys.exit("usage: bending_gate.py [--record FILE | --self-test]")
    return main_check(RECORD)


if __name__ == "__main__":
    sys.exit(main())
