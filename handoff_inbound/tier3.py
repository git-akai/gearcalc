import sys, time, json, os
from gear import Gear
from gear_tests import check_cut
from sweep import cases, tier1
from gear_tests import check_fillet_is_envelope

step, part, nparts = int(sys.argv[1]), int(sys.argv[2]), int(sys.argv[3])
sub = list(cases())[::step]
mine = sub[part::nparts]
res = {"n": 0, "pen": 0.0, "dev": 0.0, "env": 0.0, "fails": []}
t0 = time.time()
for gp in mine:
    g, f, _ = tier1(gp)
    e = check_fillet_is_envelope(g)
    pen, dev = check_cut(g)
    res["n"] += 1
    res["pen"] = max(res["pen"], pen); res["dev"] = max(res["dev"], dev)
    res["env"] = max(res["env"], e)
    if e > 1e-6: f.append("fillet envelope %.1e" % e)
    if pen > 1e-4:
        f.append("penetration %.1e" % pen)
        print("   !! z=%d pa=%.1f x=%.2f ha=%.1f rr=%.2f  pen=%.4f" %
              (gp.teeth, gp.pressure_angle, gp.profile_shift, gp.addendum, gp.root_radius, pen))
        print("      R=%.6f rb=%.6f rf=%.6f ra=%.6f rho=%.6f bc=%.6f ac=%.6f" %
              (g.R, g.rb, g.rf, g.ra, g.rho, g.bc, g.ac))
        print("      s_j=%.6f u_j=%.6f theta0=%.6f theta_a=%.6f severed=%s undercut=%s" %
              (g.s_j, g.u_j, g.theta0, g.theta_a, g.severed, g.undercut))
        print("      clamps=%s" % g.clamps.notes)
    if dev > 1e-3: f.append("deviation %.1e" % dev)
    if f:
        res["fails"].append("z=%d pa=%.1f x=%.2f h=%d ha=%.1f hf=%.2f rr=%.2f -> %s"
                            % (gp.teeth, gp.pressure_angle, gp.profile_shift, gp.helix_angle,
                               gp.addendum, gp.dedendum, gp.root_radius, ", ".join(f)))
res["secs"] = round(time.time() - t0)
acc = json.load(open("tier3.json")) if os.path.exists("tier3.json") else []
acc.append(res); json.dump(acc, open("tier3.json", "w"))
print("part %d/%d: %d cases in %d s | worst pen %.2e dev %.2e env %.2e | fails %d"
      % (part, nparts, res["n"], res["secs"], res["pen"], res["dev"], res["env"], len(res["fails"])))
for x in res["fails"][:8]:
    print("   FAIL " + x)
