"""
Involute gear cross-section generator.

Inputs are in degrees; everything internal is radians.
Geometry is generated the way a real gear is cut: an involute flank from the
rack's straight flank, and a trochoid fillet swept by the rack's rounded tip
corner.  The flank and the fillet are joined at their true intersection, which
is solved for rather than assumed -- this is what makes undercut gears come out
right.

Profile of one half-tooth, from the tooth tip centre outwards to mid tooth-space:

    tip arc  ->  involute flank  ->  trochoid fillet  ->  root arc

Angles below are "theta": the magnitude of the angle measured from the tooth
centreline.  theta = 0 at the tooth centre, pi/z at mid tooth-space.
"""
import math
from dataclasses import dataclass, field
import numpy as np
from scipy.optimize import brentq


def inv(a):
    """Involute function."""
    return math.tan(a) - a


@dataclass
class GearParams:
    module: float = 1.0
    pressure_angle: float = 20.0      # normal, degrees
    teeth: int = 17
    profile_shift: float = 0.0        # x, in modules
    helix_angle: float = 0.0          # degrees
    addendum: float = 1.0             # in modules
    dedendum: float = 1.25            # in modules
    root_radius: float = 0.38         # cutter tip radius, in modules


@dataclass
class Clamps:
    """Records any guard that altered the requested geometry."""
    pressure_angle: bool = False
    dedendum: bool = False
    thickness: bool = False
    fillet: bool = False
    pointed: bool = False
    notes: list = field(default_factory=list)

    def any(self):
        return bool(self.notes)


class Gear:
    def __init__(self, p: GearParams, legacy_clamp: bool = False):
        self.p = p
        self.legacy_clamp = legacy_clamp
        c = self.clamps = Clamps()
        m, z, x = p.module, p.teeth, p.profile_shift

        an = math.radians(p.pressure_angle)
        if an <= 1e-4:
            an = math.radians(0.5); c.pressure_angle = True
            c.notes.append("pressure angle raised to 0.5 deg")
        beta = math.radians(p.helix_angle)

        # ---- transverse conversion (normal-module system) ------------------
        self.beta = beta
        self.mt = m / math.cos(beta)
        self.alpha_t = math.atan(math.tan(an) / math.cos(beta))
        self.alpha_n = an
        self.R = self.mt * z / 2.0
        self.rb = self.R * math.cos(self.alpha_t)

        # ---- depths and thicknesses ---------------------------------------
        bd = m * (p.dedendum - x)
        if bd < 0.05 * m:
            bd = 0.05 * m; c.dedendum = True
            c.notes.append("dedendum raised: cutter depth was <= 0")
        if bd > 0.9 * self.R:
            bd = 0.9 * self.R; c.dedendum = True
            c.notes.append("dedendum capped: root radius would be <= 0")
        self.bd = bd
        self.rf = self.R - bd

        st = m * (math.pi / 2 + 2 * x * math.tan(an)) / math.cos(beta)
        st_max = 1.9 * self.R * math.pi / z
        if st <= 0.02 * m:
            st = 0.02 * m; c.thickness = True
            c.notes.append("tooth thickness raised: profile shift too negative")
        if st > st_max:
            st = st_max; c.thickness = True
            c.notes.append("tooth thickness capped: profile shift too positive")
        self.st = st
        self.psi_p = st / (2 * self.R)
        self.psi_b = self.psi_p + inv(self.alpha_t)

        # ---- cutter tip radius, clamped so the rounds fit the tip ----------
        at, ca, sa = self.alpha_t, math.cos(self.alpha_t), math.sin(self.alpha_t)
        w_roll = math.pi * self.mt - st            # rack tooth width at rolling line
        w_tip = w_roll - 2 * bd * math.tan(at)     # ... and at the tip line
        rho_fit = w_tip * ca / (2 * (1 - sa)) if w_tip > 0 else 0.0
        rho = p.root_radius * m / math.cos(beta)
        rho_cap = min(0.95 * bd, 0.95 * rho_fit)
        if rho > rho_cap:
            rho = max(rho_cap, 1e-6 * m); c.fillet = True
            c.notes.append("fillet capped to %.4f (tooth space too tight)" % rho)
        self.rho = max(rho, 1e-9)
        self.bc = bd - self.rho                    # tip-round centre depth
        self.ac = st / 2 + self.bc * math.tan(at) + self.rho / ca

        # ---- tip radius, capped at the pointed-tooth limit -----------------
        ra = self.R + m * (p.addendum + x)
        u_point = brentq(lambda u: self.psi_b - (u - math.atan(u)),
                         1e-12, 50.0, xtol=1e-14, rtol=1e-15)
        ra_point = self.rb * math.hypot(1.0, u_point)
        if ra > ra_point:
            ra = ra_point; c.pointed = True
            c.notes.append("tip capped at pointed-tooth radius %.4f" % ra)
        self.ra = max(ra, self.rb * (1 + 1e-9))
        self.u_tip = math.sqrt(max((self.ra / self.rb) ** 2 - 1.0, 0.0))

        # ---- junction between involute flank and trochoid fillet ----------
        self.L = self.R * sa - self.bc / sa - self.rho
        self.undercut = self.L < 0.0
        self.u_j, self.s_j = self._solve_junction()
        if legacy_clamp:
            # the old behaviour: clamp the flank at the base circle and let a
            # straight bridge span whatever gap is left.  Kept only so the test
            # suite can demonstrate that it detects the fault.
            self.u_j = max(self.L, 0.0) / self.rb
            self.s_j = -self.bc / math.tan(self.alpha_t)
        self.r_j = self.rb * math.hypot(1.0, self.u_j)
        self.theta0 = self.ac / self.R          # where the fillet meets the root circle
        self.theta_a = self.psi_b - inv_u(self.u_tip)   # half width of the tip arc
        self.half_pitch = math.pi / z
        self.severed = False
        self._check_severed()

    # ------------------------------------------------------------------ #
    #  primitive curves, both returning (radius, |angle from tooth centre|)
    # ------------------------------------------------------------------ #
    def involute(self, u):
        u = np.asarray(u, dtype=float)
        return self.rb * np.hypot(1.0, u), self.psi_b - (u - np.arctan(u))

    def trochoid(self, s):
        """s is the rack's travel parameter; s = 0 puts the corner at the root."""
        s = np.asarray(s, dtype=float)
        D = np.hypot(s, self.bc)
        k = 1.0 + self.rho / D
        xf, yf = k * s, self.R - k * self.bc
        return np.hypot(xf, yf), np.arctan2(xf, yf) - (s - self.ac) / self.R

    # ------------------------------------------------------------------ #
    def _solve_junction(self):
        """Return (u, s) where the involute flank meets the trochoid fillet.

        Not undercut: the rack's straight flank runs out exactly where its tip
        round begins, so the two curves meet tangentially at a point available
        in closed form.

        Undercut: the fillet has eaten into the flank and the two curves cross.
        Solve for the crossing instead of clamping to the base circle -- the
        clamp is what used to leave a step in the profile.
        """
        at = self.alpha_t
        s_tan = -self.bc / math.tan(at)
        if not self.undercut:
            return self.L / self.rb, s_tan

        r_of = lambda s: float(self.trochoid(s)[0])
        # walk outwards until the fillet has climbed past the base circle
        s_lo = min(s_tan, -1e-9)
        for _ in range(200):
            if r_of(s_lo) > self.rb:
                break
            s_lo *= 1.6
        else:
            return 0.0, s_tan                      # fillet never reaches rb
        s_b = brentq(lambda s: r_of(s) - self.rb, s_lo, 0.0, xtol=1e-15, rtol=1e-15)

        def gap(s):
            r, th = self.trochoid(s)
            r = float(r)
            u = math.sqrt(max((r / self.rb) ** 2 - 1.0, 0.0))
            return float(th) - (self.psi_b - (u - math.atan(u)))

        # gap < 0 at the base circle (fillet inside the flank); march out for a sign change
        s_hi, s_far = s_b, s_b
        for _ in range(200):
            s_far = s_far * 1.4 - 1e-6
            if gap(s_far) > 0:
                break
        else:
            return 0.0, s_b                        # no crossing found; degrade gracefully
        s_j = brentq(gap, s_far, s_hi, xtol=1e-15, rtol=1e-15)
        r_j = float(self.trochoid(s_j)[0])
        return math.sqrt(max((r_j / self.rb) ** 2 - 1.0, 0.0)), s_j

    def _check_severed(self):
        """Detect a tooth cut away entirely by undercut.

        If the fillet reaches the tooth centreline, the two fillets bounding one
        tooth have overlapped and the cutter has removed the whole tooth; any
        material further out is detached.  The profile is then truncated at the
        centreline so it stays a valid simple closed curve, and the condition is
        reported rather than silently producing a self-intersecting outline.
        """
        s = np.linspace(self.s_j, 0.0, 2000)
        th = self.trochoid(s)[1]
        if th.min() >= 0.0:
            return
        i = int(np.argmin(th))
        if i >= len(s) - 1:
            return
        s_c = brentq(lambda ss: float(self.trochoid(ss)[1]), s[i], 0.0,
                     xtol=1e-15, rtol=1e-15)
        self.severed = True
        self.s_j = s_c
        self.u_j = float("nan")
        self.ra = float(self.trochoid(s_c)[0])
        self.r_j = self.ra
        self.theta_a = 0.0
        self.u_tip = float("nan")
        self.clamps.notes.append(
            "tooth severed by undercut: profile truncated at the centreline")

    # ------------------------------------------------------------------ #
    def sections(self):
        """The four half-profile sections, ordered tip -> mid tooth-space."""
        if self.severed:
            return [("trochoid", self._fillet), ("root", self._root_arc)]
        return [("tip",      self._tip_arc),
                ("involute", self._flank),
                ("trochoid", self._fillet),
                ("root",     self._root_arc)]

    def _tip_arc(self, n):
        th = np.linspace(0.0, max(self.theta_a, 0.0), n)
        return np.full(n, self.ra), th

    def _flank(self, n):
        return self.involute(np.linspace(self.u_tip, self.u_j, n))

    def _fillet(self, n):
        return self.trochoid(np.linspace(self.s_j, 0.0, n))

    def _root_arc(self, n):
        th = np.linspace(self.theta0, self.half_pitch, n)
        return np.full(n, self.rf), th

    def _lengths(self):
        out = []
        for _, fn in self.sections():
            r, th = fn(60)
            out.append(float(np.sum(np.hypot(np.diff(r), (r[:-1] + r[1:]) / 2 * np.diff(th)))))
        return np.array(out)

    def half_profile(self, n=400):
        """(r, theta) from the tooth tip centre to mid tooth-space, arc-length spaced."""
        w = self._lengths()
        share = np.maximum(w, w.sum() * 0.004)
        counts = np.maximum((share / share.sum() * n).astype(int), 3)
        rs, ts = [], []
        for (_, fn), c in zip(self.sections(), counts):
            r, t = fn(c)
            if rs:                                   # drop the duplicated joint
                r, t = r[1:], t[1:]
            rs.append(r); ts.append(t)
        return np.concatenate(rs), np.concatenate(ts)

    def profile(self, per_tooth=400):
        """Closed cross-section as x, y arrays, counter-clockwise."""
        r, th = self.half_profile(max(per_tooth // 2, 8))
        r_full = np.concatenate([r[::-1], r[1:]])
        th_full = np.concatenate([-th[::-1], th[1:]])
        xs, ys = [], []
        for k in range(self.p.teeth):
            a = 2 * math.pi * k / self.p.teeth + th_full
            xs.append(r_full * np.cos(a)); ys.append(r_full * np.sin(a))
        x = np.concatenate(xs); y = np.concatenate(ys)
        return np.append(x, x[0]), np.append(y, y[0])


def inv_u(u):
    """Involute function expressed through u = tan(alpha)."""
    return u - math.atan(u)
