import numpy as np, math
from matplotlib.path import Path
from gearbuild import evaluate

def profile(c, n=600):
    T = np.linspace(0, c["Teeth"], c["Teeth"]*n+1)
    x, y, env = evaluate(c, T)
    return x, y, env

def thickness_at(x, y, env, r_query, tooth_center_angle=0.0):
    """Angular half-thickness of the tooth centred on angle 0, measured at radius r."""
    r = np.hypot(x, y); th = np.arctan2(y, x)
    th = (th + np.pi) % (2*np.pi) - np.pi          # wrap to +-pi
    m = np.abs(th) < (np.pi/ env["_z"])            # points belonging to tooth 0
    r, th = r[m], th[m]
    # find crossings of r = r_query
    s = np.sign(r - r_query)
    idx = np.where(np.diff(s) != 0)[0]
    crossings = []
    for i in idx:
        f = (r_query - r[i]) / (r[i+1] - r[i])
        crossings.append(th[i] + f*(th[i+1] - th[i]))
    crossings = sorted(crossings)
    if len(crossings) != 2:
        return None
    return crossings[1] - crossings[0]

def inv(a): return math.tan(a) - a

def check_thickness(c):
    x, y, env = profile(c)
    env["_z"] = c["Teeth"]
    R, Rb, St = env["_R"], env["_Rb"], env["_St"]
    at_rad = math.acos(env["_CosAt"])
    out = []
    for frac in (0.0, 0.35, 0.7, 1.0):
        r = env["_R2"] + frac*(env["_Ra"] - env["_R2"])   # sample within the involute flank
        r = min(r, env["_Ra"]*0.999); r = max(r, env["_R2"]*1.001)
        got = thickness_at(x, y, env, r)
        if got is None: continue
        a_r = math.acos(Rb/r)
        want = 2*r*(St/(2*R) + inv(math.atan(math.sqrt((R/Rb)**2-1)*0+math.tan(math.acos(Rb/R))) - inv(a_r)))
        # textbook: s(r) = r*( s/R_pitch + 2*(inv(alpha_t) - inv(alpha_r)) ) ; angular = s(r)/r
        want_ang = St/R + 2*(inv(at_rad) - inv(a_r))
        out.append((r, got, want_ang, abs(got-want_ang)))
    return out

def rack_polygon(env, phase_x):
    """One cutter tooth (the part that cuts the space) in the fixed frame, tip rounds included."""
    R, b, rho, St = env["_R"], env["_Bd"], env["_Rho"], env["_St"]
    at = math.acos(env["_CosAt"])
    Bc = env["_Bc"]; Ac = env["_Ac"]
    pts = []
    top = env["_Ra"]*0.99985
    # right flank of the LEFT-adjacent tooth is at +St/2 ... build the tooth centred right of the space
    # left flank line: x = St/2 + (R-y)*tan(at)
    yc = R - Bc                     # centre height of tip round
    cx = Ac                         # centre lateral offset (left tooth-side round)
    ang0 = math.pi/2 + at           # start angle on round (tangency with flank)
    pts.append((St/2 - (top-R)*math.tan(at), top))
    ths = np.linspace(math.pi + at, 1.5*math.pi, 40)   # round from flank tangency to tip line
    for t in ths:
        pts.append((cx + rho*math.cos(t), yc + rho*math.sin(t)))
    # mirror side of the same cutter tooth
    pitch = math.pi*env["_Mt"]
    cx2 = pitch - Ac
    ths = np.linspace(1.5*math.pi, 2*math.pi - at, 40)
    for t in ths:
        pts.append((cx2 + rho*math.cos(t), yc + rho*math.sin(t)))
    pts.append((pitch - St/2 + (top-R)*math.tan(at), top))
    pts = [(px + phase_x, py) for px, py in pts]
    return Path(np.array(pts))

def check_cut(c, nphase=240):
    x, y, env = profile(c, n=200)
    env = dict(env); env["_Mt"] = 2.0*env["_R"]/c["Teeth"]   # inlined in the compact build
    pitch = math.pi*env["_Mt"]
    worst = 0.0
    pts = np.column_stack([x, y])
    for k in range(nphase):
        s = -pitch/2 + pitch*k/nphase          # rack shift
        phi = s/env["_R"]
        # bring gear points into the fixed frame: rotate by -phi
        rot = math.pi/2 - phi          # output frame (tooth on +X) -> rack frame (tooth on +Y)
        ca, sa = math.cos(rot), math.sin(rot)
        fx = pts[:, 0]*ca - pts[:, 1]*sa
        fy = pts[:, 0]*sa + pts[:, 1]*ca
        for shift in (-pitch, 0.0, pitch):
            poly = rack_polygon(env, s + shift)
            inside = poly.contains_points(np.column_stack([fx, fy]), radius=-1e-9)
            if inside.any():
                v = poly.vertices
                a = v[:-1]; bb = v[1:]
                p = np.column_stack([fx[inside], fy[inside]])
                ab = bb - a
                t = np.clip(((p[:, None, :] - a[None]) * ab[None]).sum(-1) /
                            (ab**2).sum(-1)[None], 0, 1)
                proj = a[None] + t[..., None]*ab[None]
                d = np.min(np.hypot(p[:, None, 0]-proj[..., 0], p[:, None, 1]-proj[..., 1]), axis=1)
                worst = max(worst, d.max())
    return worst

def check_smooth(c):
    x, y, env = profile(c, n=800)
    d = np.hypot(np.diff(x), np.diff(y))
    r = np.hypot(x, y)
    return dict(maxstep=d.max(), meanstep=d.mean(), ratio=d.max()/d.mean(),
                rmin=r.min(), rmax=r.max(), Rf=env["_Rf"], Ra=env["_Ra"],
                closure=math.hypot(x[0]-x[-1], y[0]-y[-1]), nan=int(np.isnan(x).sum()))

if __name__ == "__main__":
    base = dict(Module=1, PressureAngle=20, Teeth=17, ProfileShift=0.2, HelixAngle=0,
                Addendum=1, Dedendum=1.25, RootRadius=0.38)
    print("--- involute tooth-thickness law (rad) ---")
    for r, got, want, err in check_thickness(base):
        print("  r=%.4f  measured=%.8f  theory=%.8f  err=%.2e" % (r, got, want, err))
    print("--- rack cutter penetration (mm) ---")
    print("  worst penetration:", check_cut(base))
    print("--- smoothness / closure ---")
    for k, v in check_smooth(base).items():
        print("   %-8s %s" % (k, v))
