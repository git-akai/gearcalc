"""
Test suite for the gear generator.

The central test simulates the actual cutting rack and checks the profile from
BOTH sides:

  penetration  no point of the gear may lie inside the cutter at any phase
               (material the tool would have removed is still there)
  deviation    every generated point must be touched by the cutter at some
               phase (the profile sits further from the tool than any cut
               could have left it)

Penetration alone is not sufficient: an arbitrarily undersized profile passes
it trivially.  Both bounds together pin the profile down uniquely.

The rack model used by the test is itself verified first, so a faulty test
rack cannot silently certify a faulty gear.
"""
import math
import numpy as np
from matplotlib.path import Path
from scipy.optimize import brentq
from gear import Gear, GearParams, inv


# --------------------------------------------------------------------------- #
#  rack model
# --------------------------------------------------------------------------- #
def rack_base(g, n_round=220):
    """The cutter tooth outline, built once per gear.

    Every copy and every phase is this same shape translated along x, so it is
    built once and the query points are translated instead.

    The cache lives on the Gear instance.  It was previously a module dict keyed
    on id(g), which is unsound: CPython reuses addresses after collection, so a
    new gear could be handed a previous gear's outline.
    """
    cache = g.__dict__.setdefault("_rack_cache", {})
    if n_round in cache:
        return cache[n_round]
    at = g.alpha_t
    pitch = math.pi * g.mt
    top = g.R + g.p.module * (g.p.addendum + g.p.profile_shift) + 0.25 * g.p.module
    yc = g.R - g.bc
    a1 = np.linspace(math.pi + at, 1.5 * math.pi, n_round)
    a2 = np.linspace(1.5 * math.pi, 2 * math.pi - at, n_round)
    x = np.concatenate([[g.st / 2 - (top - g.R) * math.tan(at)],
                        g.ac + g.rho * np.cos(a1),
                        pitch - g.ac + g.rho * np.cos(a2),
                        [pitch - g.st / 2 + (top - g.R) * math.tan(at)]])
    y = np.concatenate([[top], yc + g.rho * np.sin(a1),
                        yc + g.rho * np.sin(a2), [top]])
    v = np.column_stack([x, y])
    cache[n_round] = v
    return v


def rack_tooth(g, shift=0.0, n_round=220):
    v = rack_base(g, n_round).copy()
    v[:, 0] += shift
    return v


def test_rack_model(g, tol=1e-9):
    """The tip round must be tangent to both the flank and the tip line."""
    at = g.alpha_t
    C = np.array([g.ac, g.R - g.bc])
    P0 = np.array([g.st / 2, g.R])                     # flank crosses the rolling line
    nrm = np.array([math.cos(at), math.sin(at)])       # unit normal of the flank
    d_flank = float(np.dot(C - P0, nrm))
    d_tip = C[1] - g.rf
    v = rack_tooth(g)
    on_circle = np.abs(np.hypot(v[1:-1, 0] - C[0], v[1:-1, 1] - C[1]) - g.rho)
    err = {"flank tangency": abs(d_flank - g.rho),
           "tip tangency": abs(d_tip - g.rho),
           "round discretisation": float(on_circle[:len(on_circle)//2].max())}
    ok = err["flank tangency"] < tol and err["tip tangency"] < tol
    return ok, err


# --------------------------------------------------------------------------- #
#  exact signed distance to the cutter tooth
# --------------------------------------------------------------------------- #
def tooth_sdf(g, px, py, shift):
    """Exact signed distance from points to one cutter tooth; negative = inside.

    The tooth is convex: a wedge of three faces (two flanks, the tip flat) with
    its two bottom corners rounded by rho.  That is exactly the wedge eroded by
    rho, then Minkowski-summed with a disc of radius rho -- and the eroded
    wedge's corners are precisely the tip-round centres.  So the distance is
    dist(p, wedge) - rho, needing three features instead of a discretised arc.
    No chord error, and no polygon rebuild per phase.
    """
    ca, sa = math.cos(g.alpha_t), math.sin(g.alpha_t)
    pitch = math.pi * g.mt
    yb = g.rf + g.rho                      # height of the eroded wedge's base
    x1, x2 = g.ac + shift, pitch - g.ac + shift
    dy = py - yb

    e1 = (px - x1) * ca + dy * sa          # signed distance, left face
    e2 = (x2 - px) * ca + dy * sa          # signed distance, right face
    e3 = dy                                # signed distance, tip flat
    inside = (e1 >= 0) & (e2 >= 0) & (e3 >= 0)

    # left ray from (x1, yb) heading up-left; right ray from (x2, yb) up-right
    t1 = np.maximum((px - x1) * (-sa) + dy * ca, 0.0)
    d1 = np.hypot(px - (x1 - sa * t1), py - (yb + ca * t1))
    t2 = np.maximum((px - x2) * sa + dy * ca, 0.0)
    d2 = np.hypot(px - (x2 + sa * t2), py - (yb + ca * t2))
    tb = np.clip((px - x1) / (x2 - x1), 0.0, 1.0)
    db = np.hypot(px - (x1 + tb * (x2 - x1)), dy)

    out = np.minimum(np.minimum(d1, d2), db) - g.rho
    ins = -(g.rho + np.minimum(np.minimum(e1, e2), e3))
    return np.where(inside, ins, out)


def test_sdf_matches_polyline(g, n=4000, seed=0):
    """Cross-check the analytic distance against the discretised rack outline."""
    rng = np.random.default_rng(seed)
    v = rack_tooth(g, 0.0, n_round=900)
    lo, hi = v.min(0), v.max(0)
    pad = 0.4 * g.p.module
    # only compare where the real check operates: gear points never rise above
    # the tip radius, whereas the drawn outline is truncated at the rack's own
    # root line, so its flank segments end there and the two models must differ
    px = rng.uniform(lo[0] - pad, hi[0] + pad, n)
    py = rng.uniform(g.rf - pad, g.ra, n)
    a = np.abs(tooth_sdf(g, px, py, 0.0))
    b = _seg_dist(px, py, v)
    m = a < 0.5 * g.p.module            # near field, where the comparison is meaningful
    return float(np.abs(a[m] - b[m]).max())


# --------------------------------------------------------------------------- #
#  two-sided cutter verification
# --------------------------------------------------------------------------- #
def _seg_dist(px, py, v):
    a, b = v[:-1], v[1:]
    ab = b - a
    L = (ab ** 2).sum(1)
    L[L == 0] = 1e-30
    t = np.clip(((px[:, None] - a[None, :, 0]) * ab[None, :, 0] +
                 (py[:, None] - a[None, :, 1]) * ab[None, :, 1]) / L[None], 0, 1)
    dx = px[:, None] - (a[None, :, 0] + t * ab[None, :, 0])
    dy = py[:, None] - (a[None, :, 1] + t * ab[None, :, 1])
    return np.sqrt(dx * dx + dy * dy).min(1)


def rack_travel_range(g):
    """Rack displacements xi over which tooth 0 is actually generated.

    The fillet is cut when the rack is roughly a whole pitch away from the
    tooth, so a sweep of one pitch centred on the tooth misses it entirely and
    reports a large false deviation.  Derive the span instead of guessing it.
    """
    at = g.alpha_t
    tau = lambda u: u * g.rb - g.R * math.sin(at)
    xi_flank = ([] if g.severed else
                [tau(u) / math.cos(at) - g.st / 2 for u in (g.u_j, g.u_tip)])
    xi_fillet = [g.s_j - g.ac, -g.ac]
    xi_root = [0.0, g.R * g.half_pitch]
    lo, hi = min(xi_flank + xi_fillet + xi_root), max(xi_flank + xi_fillet + xi_root)
    pad = 0.6 * math.pi * g.mt
    return lo - pad, hi + pad


def check_cut(g, per_pitch=420, npts=150):
    """Two-sided verification against the generating rack.

    penetration: deepest intrusion of the gear into the cutter (must be 0)
    deviation:   furthest any generated point sits from the cutter at its
                 closest approach (must be 0 -- catches an undersized profile,
                 which the penetration bound alone cannot see)
    """
    r, th = g.half_profile(npts)
    px, py = r * np.sin(th), r * np.cos(th)      # +theta side; the other is its mirror
    generated = th > g.theta_a + 1e-12            # tip arc is not rack-cut

    pitch = math.pi * g.mt
    lo, hi = rack_travel_range(g)
    # bound the gear rotation per phase step, not the rack travel: a small gear
    # turns far more per unit of rack travel, and that is what limits accuracy
    nphase = int(np.clip((hi - lo) / g.R / 1e-3, per_pitch * (hi - lo) / pitch, 4000))
    copies = range(int(lo / pitch) - 3, int(hi / pitch) + 4)

    base = rack_base(g)
    path = Path(base)
    bx_lo, bx_hi = base[:, 0].min(), base[:, 0].max()
    D = np.full((nphase + 1, px.size), np.inf)
    worst_pen = 0.0
    for k in range(nphase + 1):
        xi = lo + (hi - lo) * k / nphase
        phi = xi / g.R
        ca, sa = math.cos(-phi), math.sin(-phi)
        fx = px * ca - py * sa
        fy = px * sa + py * ca
        for j in copies:
            sh = xi + j * pitch
            qx = fx - sh                       # shift the points, not the rack
            if bx_lo > qx.max() + 0.55 * pitch or bx_hi < qx.min() - 0.55 * pitch:
                continue
            D[k] = np.minimum(D[k], _seg_dist(qx, fy, base))
            inside = path.contains_points(np.column_stack([qx, fy]), radius=-1e-12)
            if inside.any():
                worst_pen = max(worst_pen, _seg_dist(qx[inside], fy[inside], base).max())

    # distance to the cutter is quadratic in phase near contact, so refine the
    # sampled minimum by parabolic interpolation instead of paying for more phases
    k = D.argmin(0)
    idx = np.arange(px.size)
    kc = np.clip(k, 1, nphase - 1)
    d0, d1, d2 = D[kc - 1, idx], D[kc, idx], D[kc + 1, idx]
    denom = d0 - 2 * d1 + d2
    refined = np.where(denom > 0, d1 - (d2 - d0) ** 2 / (8 * denom), d1)
    best = np.maximum(np.minimum(D.min(0), refined), 0.0)
    return worst_pen, float(best[generated].max())


def check_fillet_is_envelope(g, n=150, ns=20000):
    """Independent of the envelope derivation: every fillet point must lie
    exactly rho from the path traced by the cutter tip-round centre."""
    r, th = g.trochoid(np.linspace(g.s_j, 0.0, n))
    px, py = r * np.sin(th), r * np.cos(th)
    s = np.linspace(min(g.s_j * 3 - 1, -1), abs(g.s_j) * 3 + 1, ns)
    phi = (s - g.ac) / g.R
    cx = s * np.cos(phi) - (g.R - g.bc) * np.sin(phi)
    cy = s * np.sin(phi) + (g.R - g.bc) * np.cos(phi)
    # distance to the polyline, not to its sample points: with a sharp-cornered
    # rack (rho -> 0) the fillet lies ON the centre path, so point-sampling error
    # would dominate the very quantity being measured
    d = _seg_dist(px, py, np.column_stack([cx, cy]))
    return float(np.abs(d - g.rho).max())


def check_inner_envelope(g, n=200):
    """The profile must be the innermost boundary of what the tool can reach.

    A curve only bounds material over the radii where the tool actually
    generates it: the rack's straight flank is truncated where its tip round
    begins, so the involute exists only from that radius up (and never below
    the base circle).  Within each curve's own domain the boundary is the
    smaller angle.  Clamping the flank short of its true meeting point with the
    fillet shows up here as a violation.
    """
    r, th = g.half_profile(14000)
    # the tip and root arcs sit at constant radius, so r is not invertible there;
    # look up only on the strictly monotone flank+fillet span
    span = g.ra - g.rf
    keep = (r > g.rf + 1e-6 * span) & (r < g.ra - 1e-6 * span)
    order = np.argsort(r[keep])
    r_s, th_s = r[keep][order], th[keep][order]
    u_form = max(g.L / g.rb, 0.0)                 # flank runs out here
    r_form = max(g.rb * math.hypot(1.0, u_form), g.rb)
    r_tr_max = float(g.trochoid(g.s_j)[0])
    worst = 0.0
    for rr in np.linspace(g.rf + 1e-3 * span, g.ra - 1e-3 * span, n):
        cands = []
        if (not g.severed) and rr >= r_form - 1e-12:
            u = math.sqrt(max((rr / g.rb) ** 2 - 1.0, 0.0))
            if u <= g.u_tip + 1e-12:
                cands.append(g.psi_b - (u - math.atan(u)))
        if g.rf - 1e-12 <= rr <= r_tr_max + 1e-12:
            try:
                sr = brentq(lambda ss: float(g.trochoid(ss)[0]) - rr,
                            g.s_j - 1e-12, 0.0, xtol=1e-15, rtol=1e-15)
                cands.append(float(g.trochoid(sr)[1]))
            except ValueError:
                pass
        if not cands:
            continue
        worst = max(worst, abs(float(np.interp(rr, r_s, th_s)) - min(cands)))
    return worst


# --------------------------------------------------------------------------- #
#  geometry laws
# --------------------------------------------------------------------------- #
def check_thickness(g, samples=6):
    """Involute tooth-thickness law, checked on the flank."""
    worst = 0.0
    if g.severed:
        return 0.0
    lo, hi = g.r_j * 1.001, g.ra * 0.999
    if hi <= lo:
        return 0.0
    for rr in np.linspace(lo, hi, samples):
        a_r = math.acos(min(g.rb / rr, 1.0))
        want = g.st / g.R + 2 * (inv(g.alpha_t) - inv(a_r))
        u = math.sqrt((rr / g.rb) ** 2 - 1)
        got = 2 * (g.psi_b - (u - math.atan(u)))
        worst = max(worst, abs(got - want))
    return worst


def check_profile(g):
    r, th = g.half_profile(1200)
    x, y = g.profile(500)
    out = {}
    out["radius monotonic"] = float(np.max(np.diff(r)))
    out["theta in range"] = float(max(-th.min(), th.max() - g.half_pitch))
    out["starts at tip"] = abs(r[0] - g.ra)
    out["ends at root"] = abs(th[-1] - g.half_pitch)
    out["closure"] = math.hypot(x[0] - x[-1], y[0] - y[-1])
    out["nan"] = int(np.isnan(x).sum() + np.isnan(y).sum())
    if g.severed:
        out["junction gap"] = 0.0
    else:
        ri, ti = float(g.involute(g.u_j)[0]), float(g.involute(g.u_j)[1])
        rt, tt = float(g.trochoid(g.s_j)[0]), float(g.trochoid(g.s_j)[1])
        out["junction gap"] = math.hypot(ri * math.cos(ti) - rt * math.cos(tt),
                                         ri * math.sin(ti) - rt * math.sin(tt))
    out["root arc >= 0"] = g.half_pitch - g.theta0
    d = np.hypot(np.diff(x), np.diff(y))
    out["step ratio"] = float(d.max() / d.mean())
    return out


def run(params, cut=True, verbose=True):
    g = Gear(params)
    ok_rack, rack_err = test_rack_model(g)
    prof = check_profile(g)
    thick = check_thickness(g)
    env_fillet = check_fillet_is_envelope(g)
    env_inner = check_inner_envelope(g)
    sdf_err = test_sdf_matches_polyline(g, n=1500)
    pen = dev = None
    if cut:
        pen, dev = check_cut(g)

    fails = []
    if not ok_rack:                          fails.append("test rack not tangent")
    if sdf_err > 1e-5:                       fails.append("sdf model disagrees %.1e" % sdf_err)
    if prof["radius monotonic"] > 1e-9:      fails.append("radius not monotonic")
    if prof["theta in range"] > 1e-12:       fails.append("theta outside [0, half pitch]")
    if prof["starts at tip"] > 1e-9:         fails.append("does not start at tip radius")
    if prof["ends at root"] > 1e-12:         fails.append("does not end at mid tooth-space")
    if prof["closure"] > 1e-9:               fails.append("profile not closed")
    if prof["nan"]:                          fails.append("NaN in profile")
    if prof["junction gap"] > 1e-9:          fails.append("flank and fillet do not meet")
    if prof["root arc >= 0"] < -1e-12:       fails.append("root arc negative")
    if thick > 1e-9:                         fails.append("tooth thickness law violated")
    if env_fillet > 1e-6:                    fails.append("fillet is not the tip-round envelope")
    if env_inner > 1e-6:                     fails.append("profile is not the inner envelope")
    if cut and pen > 1e-4:                   fails.append("cutter penetration %.2e" % pen)
    if cut and dev > 3e-3:                   fails.append("deviation from rack %.2e" % dev)

    if verbose:
        p = params
        print("z=%-4d pa=%-5.1f x=%-5.2f helix=%-4.1f ha=%.2f hf=%.2f rho=%.2f  %s"
              % (p.teeth, p.pressure_angle, p.profile_shift, p.helix_angle,
                 p.addendum, p.dedendum, p.root_radius, "UNDERCUT" if g.undercut else ""))
        print("   junction gap %.1e | thickness law %.1e | fillet envelope %.1e | inner envelope %.1e"
              % (prof["junction gap"], thick, env_fillet, env_inner))
        print("   test rack: tangency ok=%s, analytic-vs-outline %.1e" % (ok_rack, sdf_err))
        if cut:
            print("   rack: penetration %.1e   deviation %.1e" % (pen, dev))
        if g.clamps.any():
            print("   clamps: " + "; ".join(g.clamps.notes))
        print("   " + ("PASS" if not fails else "FAIL -> " + ", ".join(fails)))
    return g, fails, dict(prof, thickness=thick, penetration=pen, deviation=dev,
                          fillet_envelope=env_fillet, inner_envelope=env_inner)
