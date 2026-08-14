"""Tiered parameter sweep.  Tier 1 is cheap and broad; tiers 2-3 add the
expensive independent checks on a representative subset."""
import itertools, sys, time
import numpy as np
from gear import Gear, GearParams
from gear_tests import (test_rack_model, check_profile, check_thickness,
                        check_fillet_is_envelope, check_inner_envelope, check_cut)

GRID = dict(teeth=[3, 5, 8, 11, 13, 17, 19, 30, 60, 100],
            pressure_angle=[14.5, 20, 25],
            profile_shift=[-0.5, -0.2, 0.0, 0.3, 0.6, 1.0],
            helix_angle=[0, 15, 30, 45],
            hahf=[(1.0, 1.25), (0.8, 1.0), (1.2, 1.45)],
            root_radius=[0.0, 0.2, 0.38])


def cases():
    for z, pa, x, h, (ha, hf), rr in itertools.product(
            GRID["teeth"], GRID["pressure_angle"], GRID["profile_shift"],
            GRID["helix_angle"], GRID["hahf"], GRID["root_radius"]):
        yield GearParams(1.0, pa, z, x, h, ha, hf, rr)


def tier1(gp):
    g = Gear(gp)
    ok, _ = test_rack_model(g)
    p = check_profile(g)
    f = []
    if not ok: f.append("rack model")
    if p["radius monotonic"] > 1e-9: f.append("radius not monotonic")
    if p["theta in range"] > 1e-12: f.append("theta out of range")
    if p["starts at tip"] > 1e-9: f.append("tip radius")
    if p["ends at root"] > 1e-12: f.append("mid-space")
    if p["closure"] > 1e-9: f.append("not closed")
    if p["nan"]: f.append("NaN")
    if p["junction gap"] > 1e-9: f.append("junction gap %.1e" % p["junction gap"])
    if p["root arc >= 0"] < -1e-12: f.append("root arc negative")
    if check_thickness(g) > 1e-9: f.append("thickness law")
    if check_inner_envelope(g, n=90) > 1e-5: f.append("inner envelope")
    return g, f, p


if __name__ == "__main__":
    which = sys.argv[1] if len(sys.argv) > 1 else "1"
    all_cases = list(cases())
    if which == "1":
        t0 = time.time(); bad = []; ratios = []; undercut = 0
        for gp in all_cases:
            g, f, p = tier1(gp)
            undercut += g.undercut
            ratios.append(p["step ratio"])
            if f: bad.append((gp, f))
        print("TIER 1  %d cases in %.0f s" % (len(all_cases), time.time() - t0))
        print("   undercut: %d   failures: %d" % (undercut, len(bad)))
        print("   sampling evenness (max/mean step): median %.2f  worst %.2f"
              % (np.median(ratios), max(ratios)))
        for gp, f in bad[:15]:
            print("   FAIL z=%d pa=%.1f x=%.2f h=%d ha=%.1f hf=%.2f rr=%.2f -> %s"
                  % (gp.teeth, gp.pressure_angle, gp.profile_shift, gp.helix_angle,
                     gp.addendum, gp.dedendum, gp.root_radius, ", ".join(f)))
    else:
        step = int(sys.argv[2]) if len(sys.argv) > 2 else 97
        sub = all_cases[::step]
        t0 = time.time(); bad = []; worst_e = worst_p = worst_d = 0.0
        for gp in sub:
            g, f, _ = tier1(gp)
            e = check_fillet_is_envelope(g)
            worst_e = max(worst_e, e)
            if e > 1e-6: f.append("fillet envelope %.1e" % e)
            if which == "3":
                pen, dev = check_cut(g)
                worst_p, worst_d = max(worst_p, pen), max(worst_d, dev)
                if pen > 1e-4: f.append("penetration %.1e" % pen)
                if dev > 1e-3: f.append("deviation %.1e" % dev)
            if f: bad.append((gp, f))
        print("TIER %s  %d cases in %.0f s" % (which, len(sub), time.time() - t0))
        print("   worst fillet-envelope error %.2e" % worst_e)
        if which == "3":
            print("   worst cutter penetration   %.2e" % worst_p)
            print("   worst deviation from rack  %.2e" % worst_d)
        print("   failures: %d" % len(bad))
        for gp, f in bad[:15]:
            print("   FAIL z=%d pa=%.1f x=%.2f h=%d ha=%.1f hf=%.2f rr=%.2f -> %s"
                  % (gp.teeth, gp.pressure_angle, gp.profile_shift, gp.helix_angle,
                     gp.addendum, gp.dedendum, gp.root_radius, ", ".join(f)))
