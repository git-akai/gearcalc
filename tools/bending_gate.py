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

- **the rack**: its tip `b_d = m(h_f − x)` below the rolling line, and its tip
  round, which the tool caps at 0.95 of the largest the rack's tip holds
  between its flanks and of `b_d`, where the one asked for does not fit (the
  convention `params.rs` names `FILLET_FRACTION_OF_MAX`). Rebuilt here and held
  to the harness's to the bit.
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
- **the Lewis section, by the section rule** (2026-10-02): each curve offers
  its least Lewis measure `x² / (y_v − y)` over the whole curve below the
  vertex (vertex where the load line crosses the centreline), at an interior
  least — bracketed by construction, the fillet over its whole length and the
  flank from where its condition turns to where it reaches the vertex's
  height, and closed by bisection on the complex-step condition — or at one
  of its ends, the flank's only end being
  its root (its tip is never below the vertex); the fillet's is rated with
  the fillet's notch factor, the flank's with none; the readable candidate
  rating highest governs, and with none readable the row is unrated as
  compressed, with none at all as having no section.
- **Dolan and Broghamer as AGMA fits them** (AGMA 908-B89; Mitchiner and
  Mabie, 1982): `(Y_F − axial) · K_f`, `K_f = H + (s_Fn/ρ_f)^L (s_Fn/h_Fe)^M`
  with `H, L, M` linear in `α` in radians, and `ρ_f` the fillet's least radius
  of curvature, `ρ + b_c² / (r + b_c)` at its root (the round's centre, a
  point `b_c` below a line rolling on the pitch circle, turns there on
  `b_c² / (r + b_c)`; the fillet is that path offset by the round). No
  reading where the factor is not a positive number.
- **under the ramp**, the section found once at that point and the load moved
  on it round the cycle; the rating is the **greatest** `factor · share` on the
  part of the cycle the flank carries. Found here continuously: the cycle cut
  where the share steps or turns, where the load lies square across the tooth
  (its angle's magnitude turns there, at `u = ψ_b`), and at the flank's end;
  each piece sampled and every peak closed by bisection on the complex-step
  derivative, the cuts themselves among the candidates.
- **the stress**, `F_t / (b m_n) · factor · share`, `F_t = 2000 T / d`.

Nothing here reads `gear_core`: the harness's own intermediate figures (the
tool, the virtual count, the flank's ends) are compared, never used.

# The tolerance

Each side computes every figure from the same inputs in floating point, and
`OPS = 2⁸` operations bound what either spends on one figure. **Every length
is a coordinate, or a difference of two, of a point as far out as the tip**,
`r_a`, so it carries `OPS·ε·r_a` absolutely, whatever its own size: `h_Fe`
and `y_t` are differences of numbers of size `r_a`, and a relative tolerance
on them misses exactly that cancellation (at z 3000 it failed eight figures of
an unplanted crate). Two steps amplify it, each by a conditioning computed per
row: the tangency, which both sides find to the rounding of its condition, so
its lengths move by that rounding over the condition's slope times the
curve's speed; and, under the ramp, the maximum's argument, flat to the
value's rounding within `√(2·tol/|W″|)`, or the golden section's own
`√ε` of the cycle, whichever is wider. What a length carries goes through
`α_Fen` (an arccosine), `Y_F`, the axial term, `K_f`, their difference and the
stress by each one's derivative, and the printing (`{:.12e}`, half a unit in
the last digit) is added last. `--self-test` holds the two laws of a
tolerance: a record differing from this model by rounding passes, and each
figure moved ten times its tolerance fails.
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
        teeth=[9, 12, 17, 25, 40, 70, 150, 400, 3000],
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
    dict(
        teeth=[20, 30, 40],
        alpha=[25.0],
        shift=[0.6, 0.8],
        helix=[0.0, 15.0],
        proportions=[(1.25, 1.25, 0.2, 1.0), (1.1, 1.35, 0.25, 1.0)],
        eps_n=[1.2, 1.5],
        short=[0.0],
    ),
    dict(
        teeth=[9, 12],
        alpha=[25.0],
        shift=[0.45, 0.5, 0.505, 0.508],
        helix=[0.0],
        proportions=[(1.0, 1.25, 0.38, 1.0)],
        eps_n=[1.0, 1.05],
        short=[0.0],
    ),
]
SHARINGS = ["none", "ramp"]

# The ramp: a third of the load at either end of the cycle, two thirds where
# the single-pair zone begins, all of it inside that zone.
RAMP = (1.0 / 3.0, 2.0 / 3.0)
# The tool's round, capped at this fraction of the largest that fits and of
# the rack's depth, and never below a billionth of a module: conventions of
# the tool's (`params.rs`, `guard::FILLET_FRACTION_OF_MAX`,
# `guard::MIN_FILLET_MODULES`), restated here as the rule they are.
FILLET_FRACTION = 0.95
MIN_FILLET = 1e-9
# Samples of each piece of the cycle before its peaks are closed.
PIECE = 64


@dataclasses.dataclass(frozen=True)
class Model:
    """The rating's conventions. The defaults are the rating; `--self-test`
    plants a fault by changing one."""

    # A section searched afresh at every load point, not held.
    afresh: bool = False
    # Scale on AGMA's `H` and `L`.
    h_scale: float = 1.0
    l_scale: float = 1.0
    # `ρ_F`, the fillet's radius at the section, in place of its least `ρ_f`.
    notch_at_section: bool = False
    # ISO 6336-3:2019's virtual count `z / (cos²β_b cos β)` for `z / cos³β`.
    virtual_2019: bool = False
    # The round's cap, a fraction of the largest that fits.
    fillet_fraction: float = FILLET_FRACTION
    # The ramp's maximum over the 204 samples the crate once took.
    sampled: bool = False
    # The fillet's notch factor on the flank's section too.
    flank_notch: bool = False
    # The section of highest `Y_F` governs, readable or not.
    by_form_factor: bool = False
    # Tangencies alone: no curve offers an end.
    tangency_only: bool = False


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


class GuardNotRebuilt(Exception):
    """The tool clamped something this gate does not rebuild."""


class Tooth:
    """A rack-cut tooth, `y` along its centreline, `x` across it."""

    def __init__(self, z, m, alpha, x, h_a, h_f, rho_asked, k, fraction=FILLET_FRACTION):
        self.z, self.m, self.alpha = z, m, alpha
        ta = math.tan(alpha)
        self.r = m * z / 2
        self.rb = self.r * math.cos(alpha)
        x_s = math.pi * (k - 1) / (4 * ta)
        self.s = m * (math.pi / 2 + 2 * (x + x_s) * ta)
        self.psi_b = self.s / (2 * self.r) + inv(alpha)
        # The rack's tip, `b_d` below the rolling line.
        self.b_d = m * (h_f - x)
        # Its round: the one asked, capped where it does not fit. The rack
        # holds a round tangent to both flanks and its tip line of at most
        # `w cos α / (2(1 − sin α))`, `w` the sharp tip's width; the tool
        # reads the angle as its transverse one, `atan(tan α / cos 0)`.
        a_t = math.atan(math.tan(alpha) / math.cos(0.0))
        w_tip = (math.pi * m - self.s) - 2.0 * self.b_d * math.tan(a_t)
        self.rho_fit = max(w_tip, 0.0) * math.cos(a_t) / (2.0 * (1.0 - math.sin(a_t)))
        cap = min(fraction * self.b_d, fraction * self.rho_fit)
        rho = rho_asked * m
        self.capped = rho > cap
        self.rho = max(cap if self.capped else rho, MIN_FILLET * m)
        if self.b_d < 0.05 * m or self.b_d > 0.9 * self.r or self.rho_fit < self.rho:
            raise GuardNotRebuilt("a depth guard")
        self.b_c = self.b_d - self.rho
        self.a_c = self.s / 2 + self.b_c * ta + self.rho / math.cos(alpha)
        r_a = self.r + m * (h_a + x)
        self.pointed = r_a > self.rb and self.psi_b - inv(math.acos(self.rb / r_a)) <= 0
        if self.pointed:
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
        """Load point, unit direction (toward the flank from its base tangent
        point) and the vertex, at roll `u`, real or complex."""
        x, y = self.flank(u)
        t = self.psi_b - u
        bx, by = self.rb * cmath.sin(t), self.rb * cmath.cos(t)
        n = cmath.sqrt((x - bx) ** 2 + (y - by) ** 2)
        dx, dy = (x - bx) / n, (y - by) / n
        return (x, y), (dx, dy), y - x * dy / dx

    def condition(self, curve, y_v):
        """The tangency condition `x y′ + 2x′(y_v − y)` along a curve, its
        derivatives by complex step: its sign is that of the Lewis measure's
        slope along the curve's parameter."""
        def c(q):
            x, y = (v.real for v in curve(q))
            xs, ys = (slope(lambda t: curve(t)[k], q) for k in (0, 1))
            return x * ys + 2 * xs * (y_v - y)

        return c

    def interior_least(self, name, y_v):
        """A curve's one interior least of the Lewis measure, bracketed by
        construction, or `None`.

        - The fillet: read as a half-width `w(y)` it falls and is convex, so
          `w + 2w′(y_v − y)` rises and the condition changes sign once at most
          over the whole fillet: a bisection wherever its ends differ.
        - The flank: an involute, whose condition is `r_b(cos δ + u sin δ +
          u/sin δ) = 2y_v` with `δ = u − ψ_b`; the left side falls, then rises
          from where `sin δ − u cos³δ` turns positive, and the least is where
          it crosses on the rising part. That part ends where the flank
          reaches the vertex's height, where the condition is `x y′ > 0`, so
          the bracket's far end has its sign by construction — including the
          last stretch under the tip, which a sampled search can step over
          (z 9, x 0.505, 25°: 600 samples found the fillet's 2.48 where the
          flank's under the land rates 15.4)."""
        if name == "fillet":
            lo, hi = self.sig_j, 0.0
            c = self.condition(self.fillet, y_v)
            clo, chi = c(lo), c(hi)
            return bisect(c, lo, hi) if clo and chi and (clo < 0) != (chi < 0) else None
        lo, hi = self.u_j, self.u_tip
        height = lambda u: self.flank(u)[1].real - y_v
        top = hi if height(hi) < 0 else bisect(height, lo, hi)
        turn = lambda u: math.sin(u - self.psi_b) - u * math.cos(u - self.psi_b) ** 3
        if turn(top) <= 0:
            return None
        start = lo if turn(lo) >= 0 else bisect(turn, lo, top)
        c = self.condition(self.flank, y_v)
        if c(start) >= 0 or c(top) <= 0:
            return None
        return bisect(c, start, top)

    def off_the_axis(self, x, y, y_v):
        """Whether a point is below the vertex and off the centreline by more
        than its coordinates' rounding, `OPS·ε` of its radius: the apex of a
        pointed tip loaded at its point is on both, to rounding."""
        r = math.hypot(x, y)
        return x > OPS * EPS * r and y_v - y > OPS * EPS * r

    def candidates(self, y_v, model=RATING):
        """**What each curve offers**: its least `x²/(y_v − y)` over the whole
        curve below the vertex and off the centreline — an interior least, or
        one of its ends — as `(curve name, curve, parameter, at an end)`, the
        fillet's first. The flank's only end is its root: its tip is never
        below the vertex (with `δ = u_tip − ψ_b` the load line crosses the
        centreline at most `r_b/cos δ` out and the tip corner stands
        `r_b(cos δ + u sin δ)` out, no less wherever the tip's half angle is
        not negative)."""
        out = []
        for name, curve, lo, hi, ends in (
            ("fillet", self.fillet, self.sig_j, 0.0, (self.sig_j, 0.0)),
            ("flank", self.flank, self.u_j, self.u_tip, (self.u_j,)),
        ):
            offered = []
            interior = self.interior_least(name, y_v)
            if interior is not None:
                x, y = (c.real for c in curve(interior))
                if self.off_the_axis(x, y, y_v):
                    offered.append((x * x / (y_v - y), interior, False))
            if not model.tangency_only:
                for e in ends:
                    x, y = (c.real for c in curve(e))
                    if self.off_the_axis(x, y, y_v):
                        offered.append((x * x / (y_v - y), e, True))
            if offered:
                v, p, at_end = min(offered, key=lambda o: (o[0], o[2]))
                out.append((name, curve, p, at_end))
        return out


def at_load(t, sec, u, model=RATING):
    """Every figure with the load at roll `u` (real or complex) on the
    section `sec`: the held section's, or, with `model.afresh`, one found
    for this load."""
    (lx, ly), (dx, dy), y_v = t.load(u)
    if model.afresh:
        sec = governing(t, y_v.real, u, model)[0]
        if sec is None:
            return None
    name, curve, p, _ = sec
    xt, yt = (c.real for c in curve(p))
    s, h = 2 * xt, y_v - yt
    # The load's angle off the across-tooth direction, whichever way it leans.
    sx = dx if dx.real >= 0 else -dx
    sy = dy if dy.real >= 0 else -dy
    af = cmath.atan(sy / sx)
    m, a = t.m, t.alpha
    y_f = 6 * (h / m) * cmath.cos(af) / ((s / m) ** 2 * math.cos(a))
    axial = cmath.sin(af) / ((s / m) * math.cos(a))
    if model.notch_at_section:
        rho_f = t.fillet_radius_at(p if name == "fillet" else t.sig_j)
    else:
        rho_f = t.least_fillet_radius()
    H, L, M = db_coefficients(a, model)
    # The notch factor belongs to the notch: the fillet's section carries
    # Dolan and Broghamer's, the smooth flank's none.
    if h.real <= 0:
        k_f = None
    elif name == "flank" and not model.flank_notch:
        k_f = 1.0 + 0 * s
    else:
        k_f = H + (s / rho_f) ** L * (s / h) ** M
    factor = (y_f - axial) * k_f if k_f is not None else None
    if factor is not None and not (cmath.isfinite(factor) and factor.real > 0):
        factor = None
    return dict(
        on=name, s_Fn=s, h_Fe=h, alpha_Fen=af, load_x=lx, load_y=ly, y_t=yt, rho_f=rho_f,
        Y_F=y_f, axial=axial, K_f=k_f, factor=factor, y_v=y_v, u=u, section=sec,
    )


def governing(t, y_v, u, model=RATING):
    """**The governing section** for the load at roll `u`: of what each curve
    offers, the one rated highest, a candidate the model cannot read (its
    factor not a positive number) never masking one it can. `(section, why)`,
    the section `None` and `why` the reason where there is none: no candidate
    at all, or every candidate compressed."""
    offered = t.candidates(y_v, model)
    if not offered:
        return None, "no_section"
    rated = []
    for sec in offered:
        f = at_load(t, sec, u, dataclasses.replace(model, afresh=False))
        rated.append((sec, f))
    if model.by_form_factor:
        sec, f = max(rated, key=lambda r: r[1]["Y_F"].real)
        return (sec, None) if f["factor"] is not None else (None, "compressed")
    readable = [(sec, f) for sec, f in rated if f["factor"] is not None]
    if not readable:
        return None, "compressed"
    return max(readable, key=lambda r: r[1]["factor"].real)[0], None


def ramp_piece(d, eps, mid):
    """The share at `d` on the piece of the cycle holding `mid`: the ramp's
    linear formula there, so a complex `d` differentiates it."""
    if eps - 1 <= mid <= 1:
        return 1.0 + 0 * d
    t = (d if mid <= eps - mid else eps - d) / max(eps - 1, sys.float_info.min)
    return RAMP[0] + (RAMP[1] - RAMP[0]) * t


def ramp(d, eps):
    """The share at `d` base pitches back from the tip of a cycle `eps` long."""
    if eps - 1 <= d <= 1:
        return 1.0
    t = min(d, eps - d) / max(eps - 1, sys.float_info.min)
    return RAMP[0] + (RAMP[1] - RAMP[0]) * min(max(t, 0.0), 1.0)


def greatest_on_cycle(weight, cuts):
    """The greatest `weight(d, mid)` over the pieces between `cuts`: each piece
    sampled `PIECE` times, its complex-step derivative taken at each sample,
    every fall of that derivative through zero closed by bisection, the cuts
    among the candidates. `(d, value, inside)`, `inside` true where the peak
    is not a cut."""
    best = None
    for a, b in zip(cuts, cuts[1:]):
        mid = 0.5 * (a + b)
        w = lambda d, mid=mid: weight(d, mid)
        dw = lambda d, w=w: slope(w, d)
        ds = [a + (b - a) * i / PIECE for i in range(PIECE + 1)]
        vals = [w(d) for d in ds]
        cands = [(ds[0], vals[0], False), (ds[-1], vals[-1], False)]
        rates = [dw(d) if v is not None and w(complex(d, 1e-30)) is not None else None
                 for d, v in zip(ds, vals)]
        for i in range(PIECE):
            lo, hi = rates[i], rates[i + 1]
            if lo is not None and hi is not None and lo > 0 > hi:
                d = bisect(dw, ds[i], ds[i + 1])
                cands.append((d, w(d), True))
        for d, v, inside in cands:
            if v is not None and (best is None or v.real > best[1]):
                best = (d, v.real, inside)
    return best


def rate(row, model=RATING):
    """`(tooth, z_n, figures)` of a row, the figures `None` where the model
    has no reading."""
    z, m = row["z"], MODULE
    alpha, beta = math.radians(row["alpha"]), math.radians(row["beta"])
    sin_bb = math.sin(beta) * math.cos(alpha)
    cos2_bb = 1 - sin_bb * sin_bb
    z_n = z / (cos2_bb * math.cos(beta)) if model.virtual_2019 else z / math.cos(beta) ** 3
    t = Tooth(z_n, m, alpha, row["x"], row["h_a"], row["h_f"], row["rho"], row["k"], model.fillet_fraction)
    if not t.usable:
        return t, z_n, "no_section"
    eps_n = row["eps_a"] / cos2_bb
    pitch = 2 * math.pi / t.z
    u_end = t.u_tip - row["short"] / cos2_bb * pitch

    # The flank's far end, in base pitches back from where contact ends; its
    # roll is `u_j` itself rather than `u_end − d·pitch` rounded off it.
    far = (u_end - t.u_j) / pitch

    def roll(d):
        return t.u_j - (d - far) * pitch if getattr(d, "real", d) == far else u_end - d * pitch

    def on_flank(d):
        return t.u_j <= roll(d).real <= t.u_tip

    hp = max(eps_n - 1, 0.0)
    if not on_flank(hp):
        return t, z_n, "no_section"
    _, _, y_v = t.load(roll(hp))
    sec, why = governing(t, y_v.real, roll(hp), model)
    if sec is None:
        return t, z_n, why
    f = at_load(t, sec, roll(hp), model)
    share, d_best, inside, wpp = 1.0, hp, False, 0.0
    if row["sharing"] == "ramp":
        def weight(d, mid):
            if not on_flank(d):
                return None
            g = at_load(t, sec, roll(d), model)
            return None if g is None or g["factor"] is None else g["factor"] * ramp_piece(d, eps_n, mid)

        if model.sampled:
            ds = [0.0, eps_n, hp, min(eps_n, 1.0)] + [i / 200 * eps_n for i in range(201)]
            got = [(d, weight(d, d)) for d in ds]
            d_best = max((p for p in got if p[1] is not None), key=lambda p: p[1].real)[0]
        else:
            end = min(eps_n, far)
            level = (u_end - t.psi_b) / pitch
            cuts = sorted({c for c in (0.0, end, hp, eps_n - 1, 1.0, eps_n / 2, level) if 0 <= c <= end})
            d_best, _, inside = greatest_on_cycle(weight, cuts)
            if inside:
                dw = lambda d: slope(lambda q: weight(q, d_best), d)
                step = 1e-4 * eps_n
                wpp = abs(dw(d_best + step) - dw(d_best - step)) / (2 * step)
        f = at_load(t, sec, roll(d_best), model)
        share = ramp(d_best, eps_n)
    real = {k: (v.real if isinstance(v, complex) else v) for k, v in f.items()}
    force = 1000 * TORQUE / (z * m / math.cos(beta) / 2)
    real.update(
        share=share,
        sigma_F=force / (FACE * m) * real["factor"] * share,
        alpha=alpha,
        moved=d_best != hp,
        inside=inside,
        d=d_best,
        wpp=wpp,
        eps_n=eps_n,
        tangency=conditioning(t, sec, y_v.real),
        at_end=sec[3],
        derivs=derivatives(t, sec, roll(d_best), pitch, model, eps_n, d_best) if inside else {},
    )
    return t, z_n, real


def conditioning(t, sec, y_v):
    """How far the section's point may sit from its true place, as the
    rounding of `x` and `y` there, and what that does to the measure
    relatively: `(δx, δy, δg/g)`.

    At a tangency the condition `x y′ + 2x′(y_v − y)` is computed from
    coordinates as large as the tip, so it carries `OPS·ε·(|x y′| + 2|x′||y_v −
    y| + r_a(|y′| + 2|x′|))`; over its slope that is the parameter's rounding,
    the curve's speed makes it a length, and the measure is stationary there.
    At a curve's end the parameter is the end's own, rounded as the end was
    found (the junction's roll to its scale, a rack travel to the tip's
    radius), and the measure moves with it at its own slope."""
    _, curve, p, at_end = sec

    def cond(q):
        x, y = (c.real for c in curve(q))
        xs, ys = (slope(lambda s: curve(s)[i], q) for i in (0, 1))
        return x * ys + 2 * xs * (y_v - y), (x, y, xs, ys)

    _, (x, y, xs, ys) = cond(p)
    if at_end:
        dp = OPS * EPS * (t.u_j_scale if curve == t.flank else t.r_a)
        g = x * x / (y_v - y)
        dg = abs(slope(lambda q: (lambda c: c[0] * c[0] / (y_v - c[1]))(curve(q)), p))
        return abs(xs) * dp, abs(ys) * dp, dg * dp / g
    terms = abs(x * ys) + 2 * abs(xs) * abs(y_v - y) + t.r_a * (abs(ys) + 2 * abs(xs))
    step = 1e-7 * max(abs(p), 1e-3)
    dc = abs(cond(p + step)[0] - cond(p - step)[0]) / (2 * step)
    dp = OPS * EPS * terms / dc
    return abs(xs) * dp, abs(ys) * dp, 0.0


def derivatives(t, sec, u, pitch, model, eps_n, d):
    """Each figure's rate of change with `d` at the load `u`: where the
    maximum's argument is only as good as the value is flat, a figure that
    moves with the load moves by this times that."""
    out = {}
    for key in ("load_x", "load_y", "alpha_Fen", "h_Fe", "Y_F", "axial", "K_f", "factor"):
        out[key] = abs(slope(lambda q: at_load(t, sec, u - (q - d) * pitch, model)[key], d))
    out["share"] = abs(slope(lambda q: ramp_piece(q, eps_n, d), d))
    return out


FIGURES = ["s_Fn", "h_Fe", "alpha_Fen", "load_x", "load_y", "y_t", "rho_f", "Y_F", "axial",
           "K_f", "factor", "share", "sigma_F"]


def printed(v):
    """Half a unit in the last digit `{:.12e}` prints of `v`."""
    return 0.5 * 10.0 ** (math.floor(math.log10(abs(v))) - DIGITS) if v else 0.0


def tolerances(t, f):
    """What each figure may differ by, absolutely, printing included."""
    e = OPS * EPS
    ea = e * t.r_a
    dx, dy, dg = f["tangency"]
    m, an = t.m, t.alpha
    s, h, af = f["s_Fn"], f["h_Fe"], f["alpha_Fen"]
    tol = {"load_x": ea, "load_y": ea, "rho_f": ea, "share": e}
    tol["y_t"] = ea + dy
    tol["s_Fn"] = 2 * (ea + dx)
    y_v = ea * (1 + math.tan(af))
    tol["h_Fe"] = y_v + tol["y_t"]
    # The direction is a difference of the load point and its base tangent
    # point over their distance `r_b u`; the crate reads its angle as an
    # arccosine, whose slope is `1/sin α`, and is `√(2δ)` at a square load.
    lg = t.rb * abs(f["u"].real)
    rel = ea / lg
    tol["alpha_Fen"] = min(rel / math.sin(af), math.sqrt(2 * rel)) if af > 0 else math.sqrt(2 * rel)
    tol["alpha_Fen"] += e * af
    ca, sa = math.cos(af), math.sin(af)
    # `Y_F` is the least of the ratio the tangency minimises, so the
    # tangency's rounding does not reach it; its coordinates' does.
    c1 = 6 / ((s / m) ** 2 * math.cos(an))
    tol["Y_F"] = e * f["Y_F"] + c1 * (ca * (y_v + ea) / m + (h / m) * sa * tol["alpha_Fen"]) \
        + 2 * f["Y_F"] * (2 * ea) / s + f["Y_F"] * dg
    tol["axial"] = e * f["axial"] + f["axial"] * tol["s_Fn"] / s + ca / ((s / m) * math.cos(an)) * tol["alpha_Fen"]
    H, L, M = db_coefficients(an)
    if f["on"] == "flank":
        tol["K_f"] = 0.0
    else:
        part = f["K_f"] - H
        tol["K_f"] = e * f["K_f"] + part * (abs(L) * (tol["s_Fn"] / s + tol["rho_f"] / f["rho_f"])
                                            + abs(M) * (tol["s_Fn"] / s + tol["h_Fe"] / h))
    diff = f["Y_F"] - f["axial"]
    tol["factor"] = e * f["factor"] + f["K_f"] * (tol["Y_F"] + tol["axial"]) + abs(diff) * tol["K_f"]
    w = f["factor"] * f["share"]
    tol_w = tol["factor"] * f["share"] + f["factor"] * tol["share"]
    tol["sigma_F"] = f["sigma_F"] * (e + tol_w / w)
    if f["inside"]:
        # The maximum's argument: flat to the value's rounding within
        # `√(2 tol/|W″|)`, and the crate's golden section stops at `√ε` of
        # the cycle's scale.
        spread = max(math.sqrt(2 * tol_w / f["wpp"]) if f["wpp"] > 0 else math.inf,
                     2 * math.sqrt(EPS) * f["eps_n"])
        for k, rate_ in f["derivs"].items():
            tol[k] += rate_ * spread
        tol["sigma_F"] += f["sigma_F"] / w * 0.5 * f["wpp"] * spread ** 2
    return {k: tol[k] + printed(f[k]) for k in tol}


def form(t, z_n):
    """The figures of form a row records beside its rating."""
    return dict(z_n=z_n, rho_mm=t.rho, b_d=t.b_d, u_j=t.u_j, u_tip=t.u_tip)


def form_tolerances(t, z_n):
    """What each figure of form may differ by: the tool is rebuilt by the
    tool's own arithmetic and held to the bit; the count carries a power's
    rounding; the rolls, printed to 13 digits, what their own arithmetic
    amplifies."""
    return dict(
        z_n=OPS * EPS * z_n,
        rho_mm=0.0,
        b_d=0.0,
        u_j=printed(t.u_j) + OPS * EPS * t.u_j_scale,
        u_tip=printed(t.u_tip) + OPS * EPS * t.u_tip_scale,
    )


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
        row["rated"] = None if rated[0] == "none" else dict(zip(rated[0::2], rated[1::2]))
        row["why"] = rated[1] if rated[0] == "none" else None
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


def rebuild(rows, model=RATING, only=None):
    """This model's answer for every row (or those in `only`)."""
    return [rate(row, model) if only is None or i in only else None for i, row in enumerate(rows)]


def check(rows, built):
    """Each recorded row against its rebuild: `(faults, figures compared,
    rows rated)`."""
    bad, compared, rated = [], 0, 0
    for i, (row, (t, z_n, f)) in enumerate(zip(rows, built)):
        tool = row["tool"]
        clamps = set(filter(lambda k: k != "-", tool["clamps"].split(",")))
        rebuilt = {"clamp.fillet_capped": t.capped, "clamp.tip_capped_pointed": t.pointed}
        if clamps - set(rebuilt):
            bad.append(f"row {i}: the tool clamped {sorted(clamps - set(rebuilt))}, which this does not rebuild")
        for key, here in rebuilt.items():
            if (key in clamps) != here:
                bad.append(f"row {i}: {key} {'not ' if key not in clamps else ''}said by the tool, "
                           f"{'' if here else 'not '}found here")
        # The tool, the virtual count and where the flank starts and ends:
        # compared, never used.
        mine, tols = form(t, z_n), form_tolerances(t, z_n)
        for key, tol in tols.items():
            compared += 1
            if abs(float(tool[key]) - mine[key]) > tol:
                bad.append(f"row {i}: {key} {tool[key]}, here {mine[key]!r}")
        got = row["rated"]
        why = f if isinstance(f, str) else None
        if (got is None) != (why is not None):
            bad.append(f"row {i}: the crate {'has no reading' if got is None else 'rates'}, "
                       f"here {'none (' + why + ')' if why else 'a rating'}")
            continue
        if why is not None:
            compared += 1
            if row["why"] != why:
                bad.append(f"row {i}: unrated as {row['why']}, here as {why}")
            continue
        rated += 1
        tol = tolerances(t, f)
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
    expected = 5 * len(rows) + len(FIGURES) * rated + sum(1 for _, _, f in built if isinstance(f, str))
    if not bad and compared != expected:
        bad.append(f"compared {compared} figures, the record has {expected}")
    fs = [f for _, _, f in built if isinstance(f, dict)]
    moved = sum(1 for f in fs if f["moved"])
    inside = sum(1 for f in fs if f["inside"])
    on = {n: sum(1 for f in fs if f["on"] == n) for n in ("fillet", "flank")}
    capped = sum(1 for t, _, _ in built if t.capped)
    at_end = sum(1 for f in fs if f["at_end"])
    reasons = {w: sum(1 for _, _, f in built if f == w) for w in ("no_section", "compressed")}
    print(
        f"{len(rows)} rows, {rated} rated ({on['fillet']} on the fillet, {on['flank']} on the flank, "
        f"{moved} governed off the single-pair point under the ramp, {inside} of them at a peak "
        f"inside a piece; {at_end} at a curve's end; {capped} rounds capped; unrated "
        f"{reasons['no_section']} with no section, {reasons['compressed']} compressed), "
        f"{compared} figures compared: "
        + ("all within tolerance" if not bad else f"{len(bad)} fault(s)")
    )
    # A grid that cannot see a fault is not a gate: each of these is a
    # search or a rule some row must reach.
    for what, n in (("rated off the single-pair point", moved), ("rated at a peak inside a piece", inside),
                    ("on the fillet", on["fillet"]), ("on the flank", on["flank"]), ("with a capped round", capped),
                    ("rated at a curve's end", at_end), ("unrated as compressed", reasons["compressed"])):
        if n == 0:
            bad.append(f"no row is {what}: that part of the model is untested")
    return 1 if bad else 0


# --------------------------------------------------------- the self-test ----


def as_printed(v):
    return "none" if v is None else f"{v:.12e}"


def recorded(rows, built):
    """A record this model would print: every figure as the harness prints
    it, the tool to the bit."""
    out = []
    for row, (t, z_n, f) in zip(rows, built):
        tool = dict(row["tool"], rho_mm=repr(t.rho), b_d=repr(t.b_d))
        rated = isinstance(f, dict)
        r = dict(row, tool=tool, rated=dict({k: as_printed(f[k]) for k in FIGURES}, on=f["on"]) if rated else None,
                 why=None if rated else f)
        out.append(r)
    return out


def self_test():
    """The gate's own plants, each at or near its blind spot."""
    header, rows, count = parse(RECORD.read_text())
    built = rebuild(rows)
    rated = [i for i, (_, _, f) in enumerate(built) if isinstance(f, dict)]
    results = []

    def expect(name, want_fail, planted_rows=rows, planted_count=count):
        bad, _, _ = gate(header, planted_rows, planted_count, built)
        ok = bool(bad) == want_fail
        results.append(ok)
        print(f"{'ok    ' if ok else 'WRONG '} {'fails' if bad else 'passes'}  {name}"
              + (f"  ({len(bad)}: {bad[0]})" if bad else ""))

    # The first law of a tolerance: a record differing from this model by
    # rounding passes. Each figure moved by a quarter of its tolerance less
    # its printing, alternately up and down, then printed (which adds at
    # most the printing back).
    base = recorded(rows, built)
    jitter = []
    for n, (row, (t, _, f)) in enumerate(zip(base, built)):
        r = dict(row)
        if isinstance(f, dict):
            sign = 1 if n % 2 else -1
            tol = tolerances(t, f)
            r["rated"] = dict(r["rated"], **{
                k: as_printed(f[k] + sign * (tol[k] - printed(f[k])) / 4) for k in FIGURES})
        jitter.append(r)
    expect("this model's own figures, printed", False, base)
    expect("...each moved by a quarter of its tolerance", False, jitter)

    # The second: one figure moved ten times its tolerance, figure by figure,
    # on the rated row where that figure's tolerance is widest relative to it.
    for key in FIGURES:
        i = max((i for i in rated if built[i][2][key]),
                key=lambda i: tolerances(built[i][0], built[i][2])[key] / abs(built[i][2][key]))
        t, _, f = built[i]
        planted = [dict(r) for r in base]
        planted[i] = dict(planted[i], rated=dict(planted[i]["rated"], **{
            key: as_printed(f[key] + 10 * tolerances(t, f)[key])}))
        expect(f"{key} of row {i} (z {rows[i]['z']}) ten times its tolerance off", True, planted)
    # ...and each figure of form, on the row where its tolerance is widest;
    # the tool, compared to the bit, one unit in its last place off.
    for key in ("z_n", "u_j", "u_tip"):
        i = max(range(len(rows)), key=lambda i: form_tolerances(*built[i][:2])[key] / abs(form(*built[i][:2])[key]))
        planted = [dict(r) for r in base]
        mine = form(*built[i][:2])[key] + 10 * form_tolerances(*built[i][:2])[key]
        planted[i] = dict(planted[i], tool=dict(planted[i]["tool"], **{key: repr(mine)}))
        expect(f"{key} of row {i} ten times its tolerance off", True, planted)
    for key in ("rho_mm", "b_d"):
        i = next(i for i, (t, _, _) in enumerate(built) if t.capped)
        planted = [dict(r) for r in base]
        mine = form(*built[i][:2])[key]
        planted[i] = dict(planted[i], tool=dict(planted[i]["tool"], **{key: repr(math.nextafter(mine, math.inf))}))
        expect(f"{key} of row {i} one unit in its last place off", True, planted)

    # Faults in the crate, each what that crate would print. Every one is a
    # near miss of some part of the gate.
    def planted_by(model, only):
        other = rebuild(rows, model, only)
        return recorded(rows, [o if o is not None else b for o, b in zip(other, built)])

    moved = [i for i in rated if built[i][2]["moved"]]
    inside = [i for i in rated if built[i][2]["inside"]]
    capped = [i for i, (t, _, _) in enumerate(built) if t.capped][:6]
    flank = [i for i in rated if built[i][2]["on"] == "flank"]
    helical = [i for i in rated if rows[i]["beta"]][:3]
    # A section searched afresh under the ramp: every unshared row, and every
    # row whose maximum stays at the single-pair point, is unchanged.
    expect(f"the section searched afresh under the ramp (rows {moved[:4]})", True,
           planted_by(Model(afresh=True), set(moved[:4])))
    # The ramp's maximum over the 204 samples the crate took: short by at
    # most 1.4e-3, on rows whose peak lies inside a piece.
    expect(f"the ramp's maximum over 204 samples (rows {inside[:6]})", True,
           planted_by(Model(sampled=True), set(inside[:6])))
    # `H` and `L` each off by ten times the tolerance of the stress on the
    # row the coefficient moves least.
    for coeff in ("h", "l"):
        def moves(i):
            t, _, f = built[i]
            H, L, _ = db_coefficients(f["alpha"])
            part = f["K_f"] - H
            dk = H if coeff == "h" else part * abs(L * math.log(f["s_Fn"] / f["rho_f"]))
            return dk * (f["Y_F"] - f["axial"]) * f["share"]

        def need(i):
            t, _, f = built[i]
            return tolerances(t, f)["sigma_F"] / f["sigma_F"] * f["factor"] * f["share"] / moves(i)

        i = max((i for i in rated if moves(i) > 0), key=need)
        scale = 1 + 10 * need(i)
        expect(f"{coeff.upper()} scaled by 1 + {scale - 1:.1e} (ten tolerances on row {i})", True,
               planted_by(Model(**{f"{coeff}_scale": scale}), {i}))
    expect("the notch radius read at the section", True, planted_by(Model(notch_at_section=True), set(rated[:3])))
    expect("the 2019 virtual count", True, planted_by(Model(virtual_2019=True), set(helical)))
    # The round capped at 0.94 of what fits: only capped rows move.
    expect(f"the round capped at 0.94 of what fits (rows {capped})", True,
           planted_by(Model(fillet_fraction=0.94), set(capped)))
    # The rule's near misses, each changing only the rows it reaches: the
    # fillet's notch factor on the flank's section (the rating before the
    # rule), the section of highest `Y_F` governing whether it reads or not
    # (Savage's comparison), and tangencies alone (no curve offering an end:
    # a ring would lose its notch factor).
    for name, model, rows_of in (
        ("the fillet's notch factor on the flank", Model(flank_notch=True), flank),
        ("the highest Y_F governing", Model(by_form_factor=True), None),
        ("tangencies alone", Model(tangency_only=True), None),
    ):
        other = rebuild(rows, model, set(rows_of) if rows_of is not None else None)
        differ = [i for i, (o, b) in enumerate(zip(other, built))
                  if o is not None and recorded([rows[i]], [o]) != recorded([rows[i]], [b])]
        if not differ:
            results.append(False)
            print(f"WRONG  {name}: no row of the grid differs, so the gate cannot see it")
            continue
        expect(f"{name} (rows {differ[:6]})", True,
               recorded(rows, [other[i] if i in differ[:8] else b for i, b in enumerate(built)]))
    # The record's shape.
    expect("a row missing", True, rows[:-1], count - 1)
    shifted = [dict(r) for r in rows]
    shifted[5] = dict(shifted[5], eps_a=shifted[5]["eps_a"] * (1 + 1e-9))
    expect("a contact ratio a billionth off the grid's", True, shifted)
    none = [dict(r) for r in base]
    none[rated[0]] = dict(none[rated[0]], rated=None, why="compressed")
    expect("a rated row recorded as having no reading", True, none)
    swapped = [dict(r) for r in base]
    i = next(i for i, (_, _, f) in enumerate(built) if f == "compressed")
    swapped[i] = dict(swapped[i], why="no_section")
    expect(f"an unrated row's reason swapped (row {i})", True, swapped)

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
