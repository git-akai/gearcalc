#!/usr/bin/env python3
"""The hula stage's ratio, from the no-slip condition at each pitch point.

The graph (`gear_core::kinematics`) gives a hula stage the exact ratio
`R = z2 z4 / D` with `D = z2 z4 - z1 z3`, which is Willis applied to both
meshes. This reaches the ratio from the other end -- the velocities of the two
material points at each pitch point, which must be equal -- and shares no
expression with it. Involute teeth keep a constant velocity ratio through the
engagement, so the relation at one crank position is the relation at every one.

It checks two things, and says which fails:

  - **the crate**: `tools/golden/hula_18_0.2.txt`, the corpus `check_golden.sh`
    holds to what `gear-cli hula 18 0.2` prints -- its tooth counts, its exact
    ratio and its wobble and output speeds -- against the no-slip derivation;
  - **Willis**, every arrangement at one and two teeth of difference: `closed`
    against the no-slip derivation. This checks the formula the crate states,
    not the crate.

    tools/hula_kinematics.py            # exits non-zero on either disagreement

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

import re
import sys
from fractions import Fraction
from pathlib import Path

RECORD = Path(__file__).resolve().parent / "golden" / "hula_18_0.2.txt"


def no_slip(z):
    """(wobble, output) speeds per unit crank speed, gear 1 held, exact; None
    where the two meshes do not share the crank's offset."""
    r = [Fraction(zi, 2) for zi in z]  # one module; only ratios of radii matter
    s_a = 1 if z[0] > z[1] else -1
    s_b = 1 if z[3] > z[2] else -1
    e_a, e_b = abs(r[0] - r[1]), abs(r[2] - r[3])
    if e_a != e_b:
        # At equal modules the offsets agree only at equal differences; unequal
        # modules are the stage's business, not this check's.
        return None
    e = e_a
    # Mesh A, gear 1 held: 0 = e + w_W s_a r_2.
    wobble = -e / (s_a * r[1])
    # Mesh B: w_4 s_b r_4 = e + w_W s_b r_3.
    output = (e + wobble * s_b * r[2]) / (s_b * r[3])
    return wobble, output


def closed(z):
    """R = z2 z4 / D -- the relation the crate states."""
    d = z[1] * z[3] - z[0] * z[2]
    return None if d == 0 else Fraction(z[1] * z[3], d)


def name(o):
    return "N" if o == 0 else f"N{o:+d}"


def against_the_crate():
    """The recorded stage against the no-slip derivation. Returns failures."""
    text = RECORD.read_text()
    z = [int(v) for v in re.search(r"z (\d+)/(\d+)/(\d+)/(\d+)", text).groups()]
    # `ratio 324 / 1 = +324.0000`: the fraction is printed unsigned, the
    # decimal carries the sign.
    m = re.search(r"ratio (\d+) / (\d+) = ([-+]\d+\.\d+)", text)
    p, q, signed = int(m.group(1)), int(m.group(2)), float(m.group(3))
    p = p if signed > 0 else -p
    crank, wobble, output = (
        float(v)
        for v in re.search(r"speeds\s+crank (\S+)\s+wobble (\S+)\s+output (\S+) rpm", text).groups()
    )
    w, out = no_slip(z)
    fail = []
    if Fraction(p, q) != 1 / out:
        fail.append(f"ratio: the crate records {p}/{q}, no slip gives {1 / out}")
    for what, got, want, decimals in (
        ("wobble", wobble, float(w) * crank, 3),
        ("output", output, float(out) * crank, 4),
    ):
        if f"{got:.{decimals}f}" != f"{want:.{decimals}f}":
            fail.append(f"{what} speed: the crate records {got}, no slip gives {want:.{decimals}f}")
    label = "/".join(str(v) for v in z)
    print(
        f"\nthe crate, as `gear-cli hula 18 0.2` recorded it\n\n  z {label}   ratio {p}/{q}   "
        f"{'ok' if not fail else 'DISAGREE'}"
    )
    for f in fail:
        print(f"      {f}")
    return len(fail)


def main():
    fail = against_the_crate()
    for step in (1, 2):
        offsets = (-step, 0, step)
        pairs = [(a, b) for a in offsets for b in offsets if abs(a - b) == step]
        print(f"\n{step} tooth of difference in each mesh, N = 18: Willis against no slip\n")
        print(f"  {'arrangement':<24}{'D':>5}{'Willis':>14}{'no slip':>14}")
        n = 18
        for (a, b) in pairs:
            for (c, d) in pairs:
                z = [n + a, n + b, n + c, n + d]
                R = closed(z)
                _, out = no_slip(z)
                label = "/".join(name(v - n) for v in z)
                dd = z[1] * z[3] - z[0] * z[2]
                if R is None:
                    ok = out == 0
                    shown, want = "locked", "locked" if ok else str(out)
                else:
                    ok = out != 0 and 1 / out == R
                    shown, want = str(R), str(1 / out) if out else "locked"
                fail += not ok
                print(f"  {label:<24}{dd:>5}{shown:>14}{want:>14}   {'ok' if ok else 'FAIL'}")
    print()
    if fail:
        print(f"{fail} disagreement(s)")
        return 1
    print("the crate's recorded stage and every arrangement agree with no slip")
    return 0


if __name__ == "__main__":
    sys.exit(main())
