#!/usr/bin/env python3
"""Where this tool's ratings stand against ISO 6336-2 and 6336-3, measured.

Run it after `cargo build --release --bin gear-cli`:

    python3 tools/iso_6336_3_stack.py

# What it does

For a grid of external pairs it asks `gear-cli iso` for the inputs of each
pair (the tips as cut, the rack's dedendum and tip radius, the contact ratios)
and for what the tool rates; it then computes ISO's figures itself and prints
the ratio tool / ISO:

- **bending**: ISO 6336-3 Method B, `Y_F · Y_S` (6.2, 7.2: E, G, H, the θ
  iteration, s_Fn, ρ_F, d_en, α_Fen, h_Fe), with the 2019 helix pair `f_ε`
  (6.2, Formulae 10-14) and `Y_β` (Clause 8) on a helical pair, against the
  tool's default `(Y_F − axial)·K_f`, both at the outer point of single-pair
  contact. A helical ratio is printed as the spur base (the two geometry
  factors, each on its own virtual gear) times the helix pair `1/(f_ε·Y_β)`.
- **contact**: ISO 6336-2's pinion
  `σ_H = Z_B Z_H Z_E Z_ε Z_β √(F_t (u+1)/(d_1 b u))` against the tool's rated
  pinion stress.

Every stress is nominal on both sides: no `K_A`, `K_v`, `K_Fβ`, `K_Fα`.

# Why it can be trusted

Nothing here shares code with the crate. **Before any ratio is printed it
reproduces the tool's own ISO set** — ISO's 30° tangent section and `Y_S`,
which `gear-cli iso` measures off the generated profile — to 1e-8 on every
pair it rates, spur and helical, on the tool's own virtual gear. Method B is
the closed form of that construction for a rack-cut tooth, so the two must
agree; a script that did not would be measuring its own mistakes.

# Why it exists

`docs/rationale.md` declines most of ISO's factors, and the argument for
declining them is that they are balanced only as a set — worthless unless the
set is multiplied out. The 2019 edition revised `Y_F` (an internal `f_ε ≤ 1`)
and `Y_β` (a `1/cos³β` that puts it `≥ 1`) together; neither half means
anything alone.
"""

import math
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
# The harness `tools/check_figures.py` hands over, else the release build.
BIN = os.environ.get("GEAR_CLI") or os.path.join(ROOT, "target", "release", "gear-cli")

# How closely Method B must reproduce the tool's ISO set before a ratio is
# printed: both are exact constructions, so only rounding separates them.
AGREE = 1e-8


def inv(a: float) -> float:
    return math.tan(a) - a


def tool(z1, z2, alpha, beta=0.0, face=10.0):
    """`gear-cli iso` for one pair, parsed to a dict."""
    out = subprocess.run(
        [BIN, "iso", str(z1), str(z2), str(alpha), str(beta), "0", "0", str(face)],
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    r = {"gear": [{}, {}]}
    for line in out.splitlines():
        words = line.split()
        if not words:
            continue
        if words[0] == "gear":
            g = r["gear"][int(words[1]) - 1]
            if words[2] == "d_a":
                prefix, pairs = "", words[2:]
            elif words[3] == "none":
                continue
            else:
                prefix, pairs = words[2] + ".", words[3:]
            for k, v in zip(pairs[0::2], pairs[1::2]):
                g[prefix + k] = None if v == "none" else float(v)
        elif words[0] in ("mesh", "load", "contact"):
            for k, v in zip(words[1::2], words[2::2]):
                r[k] = float(v)
    return r


# ------------------------------------------------ ISO 6336-3, Method B ----


def method_b(z, m_n, alpha_n, beta, x, d_a, h_fP, rho_fP, eps_alpha, z_n):
    """`Y_F`, `Y_S` and `q_s` at the outer point of single-pair contact, on a
    virtual gear of `z_n` teeth. Angles in radians, lengths in mm; a rack with
    no protuberance and no thickness allowance."""
    d = z * m_n / math.cos(beta)
    beta_b = math.asin(math.sin(beta) * math.cos(alpha_n))
    eps_an = eps_alpha / math.cos(beta_b) ** 2
    d_n = m_n * z_n
    d_bn = d_n * math.cos(alpha_n)
    d_an = d_n + d_a - d
    e = (
        math.pi / 4 * m_n
        - h_fP * math.tan(alpha_n)
        - (1 - math.sin(alpha_n)) * rho_fP / math.cos(alpha_n)
    )
    g = rho_fP / m_n - h_fP / m_n + x
    h = 2 / z_n * (math.pi / 2 - e / m_n) - math.pi / 3
    theta = math.pi / 6
    for _ in range(200):
        nxt = 2 * g / z_n * math.tan(theta) - h
        done = abs(nxt - theta) < 1e-16
        theta = nxt
        if done:
            break
    s_fn = m_n * (
        z_n * math.sin(math.pi / 3 - theta) + math.sqrt(3) * (g / math.cos(theta) - rho_fP / m_n)
    )
    rho_f = m_n * (rho_fP / m_n + 2 * g * g / (math.cos(theta) * (z_n * math.cos(theta) ** 2 - 2 * g)))
    # The load at the outer point of single-pair contact.
    p_bn = math.pi * d * math.cos(beta) * math.cos(alpha_n) / z
    d_en = 2 * math.sqrt(
        (math.sqrt((d_an / 2) ** 2 - (d_bn / 2) ** 2) - p_bn * (eps_an - 1)) ** 2 + (d_bn / 2) ** 2
    )
    alpha_en = math.acos(d_bn / d_en)
    gamma_e = (math.pi / 2 + 2 * x * math.tan(alpha_n)) / z_n + inv(alpha_n) - inv(alpha_en)
    alpha_fen = alpha_en - gamma_e
    h_fe = (m_n / 2) * (
        (math.cos(gamma_e) - math.sin(gamma_e) * math.tan(alpha_fen)) * d_en / m_n
        - z_n * math.cos(math.pi / 3 - theta)
        - g / math.cos(theta)
        + rho_fP / m_n
    )
    y_f = 6 * (h_fe / m_n) * math.cos(alpha_fen) / ((s_fn / m_n) ** 2 * math.cos(alpha_n))
    ell = s_fn / h_fe
    q_s = s_fn / (2 * rho_f)
    y_s = (1.2 + 0.13 * ell) * q_s ** (1 / (1.21 + 2.3 / ell))
    return y_f, y_s, q_s, eps_an


def f_eps(eps_beta: float, eps_alpha_n: float) -> float:
    """The load distribution factor inside the 2019 `Y_F`, Formulae (10)-(14)."""
    if eps_beta == 0.0:
        return 1.0 if eps_alpha_n < 2.0 else 0.7
    if eps_beta >= 1.0:
        return eps_alpha_n**-0.5
    if eps_alpha_n < 2.0:
        return math.sqrt(1.0 - eps_beta + eps_beta / eps_alpha_n)
    return math.sqrt((1.0 - eps_beta) / 2.0 + eps_beta / eps_alpha_n)


def y_beta(beta: float, eps_beta: float) -> float:
    """The 2019 helix angle factor, Formula (66): `ε_β` held at 1 and `β` at
    30° above them, as the clause says."""
    b = min(abs(math.degrees(beta)), 30.0)
    return (1.0 - min(eps_beta, 1.0) * b / 120.0) / math.cos(math.radians(b)) ** 3


# ------------------------------------------------------------ one pair ----


def rate(z1, z2, alpha_deg, beta_deg=0.0, face=10.0):
    """One unshifted pair measured both ways, the ISO set reproduced first.

    Returns each member's `(spur base, helix pair)` and the pinion's contact
    `(tool, ISO, Z_ε)`."""
    r = tool(z1, z2, alpha_deg, beta_deg, face)
    m_n = 1.0
    alpha_n, beta = math.radians(alpha_deg), math.radians(beta_deg)
    alpha_t = math.atan(math.tan(alpha_n) / math.cos(beta))
    beta_b = math.asin(math.sin(beta) * math.cos(alpha_n))
    d = [z * m_n / math.cos(beta) for z in (z1, z2)]
    d_b = [di * math.cos(alpha_t) for di in d]
    d_a = [g["d_a"] for g in r["gear"]]
    # Unshifted, so the pair runs at its reference distance and angle.
    a_w, alpha_wt = (d[0] + d[1]) / 2, alpha_t
    eps = (
        sum(math.sqrt((da / 2) ** 2 - (db / 2) ** 2) for da, db in zip(d_a, d_b))
        - a_w * math.sin(alpha_wt)
    ) / (math.pi * m_n / math.cos(beta) * math.cos(alpha_t))
    assert abs(eps - r["eps_alpha"]) < AGREE, (z1, z2, eps, r["eps_alpha"])
    eps_beta = face * abs(math.sin(beta)) / (math.pi * m_n)

    rows = []
    for i, z in enumerate((z1, z2)):
        g = r["gear"][i]
        if "iso.Y_F" not in g or g.get("tool.factor") is None:
            continue
        args = (z, m_n, alpha_n, beta, 0.0, g["d_a"], g["h_fP"], g["rho_fP"], eps)
        # On the tool's own virtual gear, z/cos³β, the construction must agree.
        yf_t, ys_t, _, _ = method_b(*args, z / math.cos(beta) ** 3)
        assert abs(yf_t - g["iso.Y_F"]) < AGREE and abs(ys_t - g["iso.Y_S"]) < AGREE, (
            z1, z2, alpha_deg, beta_deg, i + 1, yf_t, g["iso.Y_F"], ys_t, g["iso.Y_S"]
        )
        # ISO 2019's virtual gear, z/(cos²β_b cos β).
        yf, ys, _, eps_an = method_b(*args, z / (math.cos(beta_b) ** 2 * math.cos(beta)))
        pair = 1.0 / (f_eps(eps_beta, eps_an) * y_beta(beta, eps_beta)) if beta_deg else 1.0
        rows.append((g["tool.factor"] / (yf * ys), pair))

    # ISO 6336-2, the pinion.
    u = z2 / z1
    z_h = math.sqrt(
        2 * math.cos(beta_b) * math.cos(alpha_wt) / (math.cos(alpha_t) ** 2 * math.sin(alpha_wt))
    )
    z_e = math.sqrt(r["E_star"] / math.pi)
    if eps_beta >= 1:
        z_eps = math.sqrt(1 / eps)
    else:
        z_eps = math.sqrt((4 - eps) / 3 * (1 - eps_beta) + eps_beta / eps)
    z_beta = 1 / math.sqrt(math.cos(beta))
    m1 = math.tan(alpha_wt) / math.sqrt(
        (math.sqrt((d_a[0] / d_b[0]) ** 2 - 1) - 2 * math.pi / z1)
        * (math.sqrt((d_a[1] / d_b[1]) ** 2 - 1) - (eps - 1) * 2 * math.pi / z2)
    )
    z_b = 1.0 if eps_beta >= 1 else max(1.0, m1 - eps_beta * (m1 - 1))
    sigma_h = z_b * z_h * z_e * z_eps * z_beta * math.sqrt(r["F_t"] * (u + 1) / (d[0] * face * u))
    return rows, (r["rated_1"], sigma_h, z_eps)


def main() -> None:
    if not os.path.exists(BIN):
        sys.exit(f"{BIN} is not built: `cargo build --release --bin gear-cli` first")
    print(__doc__.strip().split("\n")[0])

    canary = tool(17, 43, 20.0)["gear"][0]
    rate(17, 43, 20.0)
    print(
        f"\nThe tool's ISO set on 17/43 reproduced: Y_F {canary['iso.Y_F']:.6f}, "
        f"Y_S {canary['iso.Y_S']:.6f}; agreement asserted to {AGREE:g} on every pair below."
    )

    pinions = (12, 17, 25, 40, 70)
    wheels = (25, 43, 70, 120)
    print("\n== spur bending at the outer point of single-pair contact, tool / ISO ==")
    print(f"  pairs z1 {pinions} with z2 {wheels} (z2 >= z1), x = 0, both members")
    for alpha in (14.5, 20.0, 25.0):
        ratios = []
        for z1 in pinions:
            for z2 in wheels:
                if z2 >= z1:
                    ratios += [b for b, _ in rate(z1, z2, alpha)[0]]
        print(f"  alpha {alpha:4.1f} deg  {min(ratios):.3f}–{max(ratios):.3f}  ({len(ratios)} members)")

    print("\n== helical bending, tool / ISO = spur base x helix pair 1/(f_ε·Y_β) ==")
    print("  pairs 17/43 and 25/70, x = 0, alpha 20 deg, beta 10/20/30 deg, both members")
    print(f"  {'ε_β':>5} {'whole':>12} {'spur base':>12} {'helix pair':>12}")
    for eps_beta in (0.1, 0.3, 0.6, 1.0, 1.6):
        whole, bases, pairs = [], [], []
        for beta in (10.0, 20.0, 30.0):
            face = eps_beta * math.pi / math.sin(math.radians(beta))
            for z1, z2 in ((17, 43), (25, 70)):
                for b, p in rate(z1, z2, 20.0, beta, face)[0]:
                    whole.append(b * p)
                    bases.append(b)
                    pairs.append(p)
        print(
            f"  {eps_beta:5.1f} {min(whole):.3f}–{max(whole):.3f} {min(bases):.3f}–{max(bases):.3f}"
            f" {min(pairs):.3f}–{max(pairs):.3f}"
        )

    print("\n== contact, the pinion: tool / ISO 6336-2 (Z_B Z_H Z_E Z_ε Z_β) ==")
    for label, args in (
        ("17/43 spur, 10 mm", (17, 43, 20.0, 0.0, 10.0)),
        ("17/43 beta 20 deg, 10 mm, ε_β > 1", (17, 43, 20.0, 20.0, 10.0)),
    ):
        tool_h, iso_h, z_eps = rate(*args)[1]
        print(
            f"  {label:<34} {tool_h:.1f} against {iso_h:.1f} MPa: "
            f"{100 * (tool_h / iso_h - 1):+.1f} % (1/Z_ε {1 / z_eps:.4f})"
        )


if __name__ == "__main__":
    main()
