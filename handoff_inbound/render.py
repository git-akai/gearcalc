import numpy as np, math, matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from gear import Gear, GearParams

OUT = "/mnt/user-data/outputs/"


def half_xy(g):
    r, th = g.half_profile(1400)
    return r * np.sin(th), r * np.cos(th)


def legacy_half_xy(g):
    """Old construction: flank clamped at the base circle, straight bridge."""
    ri, ti = g.involute(np.linspace(g.u_tip, g.u_j, 500))
    rt, tt = g.trochoid(np.linspace(g.s_j, 0.0, 500))
    tha = np.linspace(0, max(g.theta_a, 0), 60)
    thr = np.linspace(g.theta0, g.half_pitch, 60)
    r = np.concatenate([np.full(60, g.ra), ri, rt, np.full(60, g.rf)])
    t = np.concatenate([tha, ti, tt, thr])
    return r * np.sin(t), r * np.cos(t)


# --------------------------------------------------------------- before/after
cases = [("z=8, x=0.0", GearParams(teeth=8, profile_shift=0.0)),
         ("z=3, x=0.5", GearParams(teeth=3, profile_shift=0.5))]
fig, axes = plt.subplots(2, 2, figsize=(12.5, 11))
for row, (tag, gp) in enumerate(cases):
    old = Gear(gp, legacy_clamp=True)
    new = Gear(gp)
    for col, (g, lbl, xy) in enumerate(
            [(old, "before  (flank clamped at base circle)", legacy_half_xy),
             (new, "after  (flank meets fillet at their intersection)", half_xy)]):
        ax = axes[row][col]
        x, y = xy(g)
        ax.plot(x, y, lw=2.0, color="C3" if col == 0 else "C0", zorder=4)
        arc = np.linspace(-0.25, 0.55, 200)
        for rr, nm in ((g.rb, "base"), (g.rf, "root"), (g.ra, "tip")):
            ax.plot(rr * np.sin(arc), rr * np.cos(arc), "--", lw=.7, color="0.65")
            ax.annotate(nm, (rr * math.sin(0.53), rr * math.cos(0.53)),
                        fontsize=8, color="0.4")
        if col == 0:
            ri, ti = g.involute(g.u_j)
            rt, tt = g.trochoid(g.s_j)
            ax.plot([float(ri) * math.sin(float(ti)), float(rt) * math.sin(float(tt))],
                    [float(ri) * math.cos(float(ti)), float(rt) * math.cos(float(tt))],
                    color="k", lw=1.0, ls=":", zorder=5)
            ax.annotate("gap %.3f mm" % math.hypot(
                float(ri) * math.sin(float(ti)) - float(rt) * math.sin(float(tt)),
                float(ri) * math.cos(float(ti)) - float(rt) * math.cos(float(tt))),
                (float(ri) * math.sin(float(ti)), float(ri) * math.cos(float(ti))),
                textcoords="offset points", xytext=(12, -4), fontsize=9, color="C3")
        ax.set_aspect("equal"); ax.axis("off")
        ax.set_title("%s\n%s" % (tag, lbl), fontsize=10)
        lo, hi = g.rf - 0.35, g.ra + 0.2
        ax.set_xlim(-0.15 * hi, 0.62 * hi); ax.set_ylim(lo * 0.72, hi * 1.02)
plt.tight_layout(); plt.savefig(OUT + "before_after.png", dpi=110); plt.close()

# ------------------------------------------------------------------- matrix
grid = [GearParams(teeth=17, profile_shift=0.2),
        GearParams(teeth=8,  profile_shift=0.0),
        GearParams(teeth=3,  profile_shift=0.5),
        GearParams(teeth=24, pressure_angle=14.5),
        GearParams(teeth=13, pressure_angle=25, profile_shift=-0.3),
        GearParams(teeth=17, profile_shift=0.2, helix_angle=30),
        GearParams(teeth=40, module=2),
        GearParams(teeth=12, profile_shift=0.6, addendum=1.2, dedendum=1.4),
        GearParams(teeth=100)]
fig, axes = plt.subplots(3, 3, figsize=(15, 15))
for ax, gp in zip(axes.ravel(), grid):
    g = Gear(gp)
    x, y = g.profile(420)
    ax.plot(x, y, lw=0.9)
    th = np.linspace(0, 2 * np.pi, 500)
    ax.plot(g.rb * np.cos(th), g.rb * np.sin(th), ":", lw=.5, color="0.65")
    ax.set_aspect("equal"); ax.axis("off")
    ax.set_title("z=%d  pa=%.1f  x=%.1f  helix=%d  m=%g%s"
                 % (gp.teeth, gp.pressure_angle, gp.profile_shift,
                    gp.helix_angle, gp.module, "  UNDERCUT" if g.undercut else ""),
                 fontsize=9)
plt.tight_layout(); plt.savefig(OUT + "matrix.png", dpi=95); plt.close()

# ------------------------------------------------- undercut flank zoom detail
fig, axes = plt.subplots(1, 3, figsize=(16, 5.6))
for ax, gp in zip(axes, [GearParams(teeth=8, profile_shift=0.0),
                         GearParams(teeth=10, profile_shift=0.0),
                         GearParams(teeth=13, profile_shift=-0.2)]):
    g = Gear(gp)
    x, y = half_xy(g)
    ax.plot(x, y, lw=2.2, color="C0", label="generated profile")
    u = np.linspace(g.u_j, g.u_tip, 300)
    ri, ti = g.involute(u)
    ax.plot(ri * np.sin(ti), ri * np.cos(ti), lw=1.0, color="C2", label="involute flank")
    s = np.linspace(g.s_j * 1.25, 0, 400)
    rt, tt = g.trochoid(s)
    ax.plot(rt * np.sin(tt), rt * np.cos(tt), lw=1.0, ls="--", color="C1",
            label="trochoid (extended past junction)")
    rj, tj = g.involute(g.u_j)
    ax.plot([float(rj) * math.sin(float(tj))], [float(rj) * math.cos(float(tj))],
            "o", ms=7, mfc="none", mec="k", label="junction")
    arc = np.linspace(-0.1, 0.5, 120)
    ax.plot(g.rb * np.sin(arc), g.rb * np.cos(arc), ":", lw=.8, color="0.6")
    ax.annotate("base circle", (g.rb * math.sin(0.48), g.rb * math.cos(0.48)),
                fontsize=8, color="0.45")
    ax.set_aspect("equal"); ax.axis("off")
    ax.set_title("z=%d  x=%.1f   undercut notch" % (gp.teeth, gp.profile_shift), fontsize=10)
    ax.legend(fontsize=8, loc="lower right")
plt.tight_layout(); plt.savefig(OUT + "undercut_detail.png", dpi=110); plt.close()
print("rendered")
