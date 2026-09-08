#!/usr/bin/env python3
"""Where this tool's bending rating stands against ISO 6336-3:2019, factor by factor.

Run it:

    python3 tools/iso_6336_3_stack.py

# Why this exists

`docs/rationale.md` declines most of ISO 6336-3's factors, and the argument for
declining them is that they are **balanced only as a set**. That argument is
worthless as an assertion — it has to be multiplied out, because the direction
of any single factor tells you nothing about the direction of the set it came
from. This project learned that the expensive way: `Y_β` exceeds 1 over most of
its own figure, which reads as "omitting it is unconservative", and it was
adopted on that reading and reverted on this measurement.

The 2019 edition's Foreword lists its first two changes as a modification of
`Y_β` (Clause 8) and a modification of `Y_F` (6.2) — **together**. The revised
`Y_F` carries an internal `f_ε` that is `≤ 1`; the revised `Y_β` gained a
`1/cos³β` that puts it `≥ 1`. Neither half means anything alone.

Nothing here shares code with the crate; that is the point. The crate's own
figures come from `gear-cli strength`, and the two are compared by hand.
"""

import math

ALPHA_N = math.radians(20.0)


def base_helix_angle(beta_deg: float) -> float:
    """`sin β_b = sin β cos α_n` — ISO 6336-3:2019 Formula (15)."""
    return math.asin(math.sin(math.radians(beta_deg)) * math.cos(ALPHA_N))


def y_beta(beta_deg: float, eps_beta: float) -> float:
    """The helix angle factor, Formula (66).

    `ε_β` is held at 1 above it and `β` at 30° above that, both substitutions
    the clause states. The `1/cos³β` is what the 2006 edition did not have.
    """
    beta = min(abs(beta_deg), 30.0)
    eps = min(eps_beta, 1.0)
    return (1.0 - eps * beta / 120.0) / math.cos(math.radians(beta)) ** 3


def f_eps(eps_beta: float, eps_alpha_n: float) -> float:
    """The load distribution factor **inside** the 2019 `Y_F`, Formulae (10)-(14)."""
    if eps_beta == 0.0:
        return 1.0 if eps_alpha_n < 2.0 else 0.7
    if eps_beta >= 1.0:
        return eps_alpha_n**-0.5
    if eps_alpha_n < 2.0:
        return math.sqrt(1.0 - eps_beta + eps_beta / eps_alpha_n)
    return math.sqrt((1.0 - eps_beta) / 2.0 + eps_beta / eps_alpha_n)


def virtual_tooth_count_ratio(beta_deg: float) -> float:
    """This crate's `z_n` over ISO 2019's.

    The crate uses `z/cos³β` (the classical form, and the 2006 one); ISO 2019
    uses `z/(cos²β_b · cos β)`. A third mixing, and the smallest.
    """
    beta = math.radians(beta_deg)
    return (math.cos(base_helix_angle(beta_deg)) ** 2 * math.cos(beta)) / math.cos(beta) ** 3


def main() -> None:
    eps_alpha = 1.65  # an ordinary transverse contact ratio
    print(__doc__.strip().split("\n")[0])
    print(f"\nTransverse contact ratio {eps_alpha}, α_n 20°.")
    print("'this tool' applies neither f_ε nor Y_β; 1.00 would agree with ISO.\n")
    print(f"{'β°':>4} {'ε_β':>5} {'ε_αn':>6} {'f_ε':>7} {'Y_β':>7} {'PRODUCT':>8} {'tool/ISO':>9}")
    print("-" * 52)

    rows = []
    for beta in (5.0, 10.0, 15.0, 20.0, 25.0, 30.0):
        eps_an = eps_alpha / math.cos(base_helix_angle(beta)) ** 2
        for eps_b in (0.1, 0.3, 0.6, 1.0, 1.6):
            fe = f_eps(eps_b, eps_an)
            yb = y_beta(beta, eps_b)
            product = fe * yb
            rows.append((beta, eps_b, product, 1.0 / product))
            print(
                f"{beta:4.0f} {eps_b:5.2f} {eps_an:6.3f} {fe:7.4f} {yb:7.4f} "
                f"{product:8.4f} {1.0 / product:9.3f}"
            )

    def band(name: str, keep) -> None:
        sel = [r for r in rows if keep(r[1])]
        lo, hi = min(r[3] for r in sel), max(r[3] for r in sel)
        print(f"  {name:<34} {lo:.2f}–{hi:.2f}×")

    print("\nWhat this tool reports, over ISO 6336-3:2019:")
    band("full overlap (ε_β ≥ 1)", lambda e: e >= 1.0)
    band("low overlap (ε_β ≤ 0.3)", lambda e: e <= 0.3)
    print("\n  Full overlap is the designed-for case and the tool is conservative there.")
    print("  Low overlap is the one regime it runs BELOW the standard — and a helical")
    print("  stage without full axial overlap already raises `stage.overlap_below_one`.")

    print("\nThe third mixing: this crate's virtual gear follows the 2006 z_n.")
    for beta in (10.0, 20.0, 30.0):
        print(f"  β {beta:4.0f}°: {100 * (virtual_tooth_count_ratio(beta) - 1):+.2f}% the teeth ISO 2019 would use")


if __name__ == "__main__":
    main()
