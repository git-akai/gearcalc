#!/usr/bin/env python3
"""First subsurface yield under a Hertz contact, from the closed-form fields.

    python3 tools/first_yield.py      # after `cargo build --release --bin gear-cli`

The ultimate contact rating judges a peak pressure against `p_Y = C·σ_y`, the
pressure at which the material first yields below the surface (von Mises).
`C` has no closed form: it is the maximum over depth of a stress field that
does. This derives it at the two ends where the field on the axis is closed
form, sharing no code with the crate:

- line contact, plane strain (McEwen), with ζ = z/b:
  σ_x = −p₀((1+2ζ²)/√(1+ζ²) − 2ζ),  σ_z = −p₀/√(1+ζ²),  σ_y = ν(σ_x+σ_z);
- circular contact (Johnson 3.45), with ζ = z/a:
  σ_r = σ_θ = −p₀(1+ν)(1 − ζ atan(1/ζ)) + ½p₀/(1+ζ²),  σ_z = −p₀/(1+ζ²).

It checks the published figures (Johnson, *Contact Mechanics*, 4.2: 1.79 and
1.60 at ν = 0.3, to three figures), then that `gear-cli iso` prints the same
`C` at both ends, at ν = 0.3 and at the canary material's.
"""

import math
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.environ.get("GEAR_CLI") or os.path.join(ROOT, "target", "release", "gear-cli")


def von_mises(a, b, c):
    return math.sqrt(((a - b) ** 2 + (b - c) ** 2 + (c - a) ** 2) / 2)


def line(nu, z):
    q = math.sqrt(1 + z * z)
    sx = -((1 + 2 * z * z) / q - 2 * z)
    sz = -1 / q
    return von_mises(sx, nu * (sx + sz), sz)


def circle(nu, z):
    sr = -(1 + nu) * (1 - z * math.atan(1 / z)) + 0.5 / (1 + z * z)
    return von_mises(sr, sr, -1 / (1 + z * z))


def factor(field, nu):
    """1 / the peak of `field` over depth: a fine scan, then a ternary search
    about its best point."""
    zs = [0.01 * k for k in range(1, 400)]
    best = max(zs, key=lambda z: field(nu, z))
    lo, hi = best - 0.01, best + 0.01
    for _ in range(200):
        m1, m2 = lo + (hi - lo) / 3, hi - (hi - lo) / 3
        if field(nu, m1) < field(nu, m2):
            lo = m1
        else:
            hi = m2
    return 1 / field(nu, (lo + hi) / 2)


def main():
    c_line, c_circle = factor(line, 0.3), factor(circle, 0.3)
    print(f"C at ν 0.3: line {c_line:.4f}, circle {c_circle:.4f}")
    assert round(c_line, 2) == 1.79, c_line
    assert abs(c_circle - 1.60) < 0.015, c_circle
    if not os.path.exists(BIN):
        sys.exit(f"{BIN} is not built: `cargo build --release --bin gear-cli` first")
    out = subprocess.run([BIN, "iso"], capture_output=True, text=True, check=True).stdout
    rows = [line_.split() for line_ in out.splitlines() if line_.startswith("yield ")]
    assert rows, "gear-cli iso printed no first-yield line"
    for w in rows:
        nu, tool_line, tool_circle = float(w[2]), float(w[4]), float(w[6])
        mine = (factor(line, nu), factor(circle, nu))
        for got, want in zip((tool_line, tool_circle), mine):
            assert abs(got - want) < 1e-6 * want, (nu, got, want)
        print(f"  ν {nu}: the crate's {tool_line:.6f} / {tool_circle:.6f} agree")


if __name__ == "__main__":
    main()
