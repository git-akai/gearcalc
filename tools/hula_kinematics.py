#!/usr/bin/env python3
"""The hula stage's ratio, integrated from the rolling pitch circles.

`gear_core::hula` reports a ratio as two integer products, `R = z2 z4 / D` with
`D = z2 z4 - z1 z3`, which is Willis applied to both meshes. This reaches the
same number from the other end -- by stepping the crank round and integrating
the no-slip condition at each pitch point -- and shares no expression with it.
It is the same standard the crossed-axis path is held to by `crossed_path.py`.

    tools/hula_kinematics.py            # every arrangement, both differences

# The geometry of one mesh

Gears 1 and 4 sit on the fixed axis, 1 grounded and 4 the output; gears 2 and 3
ride a body carried on a crank of offset e. A pair is therefore always internal
and its ring is whichever member has more teeth. With the fixed-axis member F at
the origin and the wobble member W at C = e*u(theta), the pitch point sits at
s*r_F*u -- where s = +1 when F is the ring and -1 when W is -- and then
P - C = s*r_W*u. The two material points there have the same velocity:

    w_F * s * r_F  =  w_c * e  +  w_W * s * r_W

Mesh A with gear 1 held gives the wobble body's speed; mesh B then gives the
output's. Nothing below divides one tooth count by another.
"""

import math
import sys
from fractions import Fraction

STEPS = 200_000


def rolling(z, steps=STEPS):
    """Output revolutions per crank revolution, by integration."""
    r = [zi / 2 for zi in z]  # one module; only ratios of radii matter here
    s_a = 1.0 if z[0] > z[1] else -1.0
    s_b = 1.0 if z[3] > z[2] else -1.0
    e_a, e_b = abs(r[0] - r[1]), abs(r[2] - r[3])
    if abs(e_a - e_b) > 1e-12:
        # The two meshes must share the crank's offset. At equal modules that
        # means equal tooth differences; unequal modules is the general case and
        # is the stage's business, not this check's.
        return None
    phi_w = phi_out = 0.0
    dth = 2 * math.pi / steps
    for _ in range(steps):
        dphi_w = -dth * e_a / (s_a * r[1])
        phi_out += (dth * e_b + dphi_w * s_b * r[2]) / (s_b * r[3])
        phi_w += dphi_w
    return phi_out / (2 * math.pi)


def closed(z):
    """R = z2 z4 / D -- what the crate reports."""
    d = z[1] * z[3] - z[0] * z[2]
    return None if d == 0 else Fraction(z[1] * z[3], d)


def name(o):
    return "N" if o == 0 else f"N{o:+d}"


def main():
    fail = 0
    for step in (1, 2):
        offsets = (-step, 0, step)
        pairs = [(a, b) for a in offsets for b in offsets if abs(a - b) == step]
        print(f"\n{step} tooth of difference in each mesh, N = 18\n")
        print(f"  {'arrangement':<24}{'D':>5}{'ratio':>14}{'integrated':>14}   ")
        n = 18
        for (a, b) in pairs:
            for (c, d) in pairs:
                z = [n + a, n + b, n + c, n + d]
                R = closed(z)
                rolled = rolling(z)
                label = "/".join(name(v - n) for v in (z))
                dd = z[1] * z[3] - z[0] * z[2]
                if R is None:
                    ok = rolled is None or abs(rolled) < 1e-9
                    print(f"  {label:<24}{dd:>5}{'locked':>14}"
                          f"{'0 rev' if ok else f'{rolled:+.6f}':>14}   {'ok' if ok else 'FAIL'}")
                    fail += not ok
                    continue
                want = float(R)
                got = 1 / rolled if rolled else float("inf")
                ok = abs(got - want) < 1e-4 * max(1.0, abs(want))
                fail += not ok
                print(f"  {label:<24}{dd:>5}{want:>14.4f}{got:>14.4f}   "
                      f"{'ok' if ok else 'FAIL'}")
    print()
    if fail:
        print(f"{fail} arrangement(s) disagree")
        return 1
    print("every arrangement agrees with the integration")
    return 0


if __name__ == "__main__":
    sys.exit(main())
