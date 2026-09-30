#!/usr/bin/env python3
"""Whether a drive breaks away, from a moment balance on every body.

A path's efficiency (`gear_core::train::PathReport`) is its flow once moving,
and **nought where the same flow against static friction cannot start**
(`Directional::once_moving`). The crate's flow counts only the power that
leaves, so a drive that cannot start -- one whose output would have to be
pushed as well -- reads as nought, and whether nought is "above" nought was
once decided by rounding: a compound set back-driven at rest broke away by
three parts in 10^15, alone and after a spur, and did not after an idler.

This reaches the same figures, **sign included**, by a method that shares no
search with the crate's: the carrier-frame power flow of an epicyclic set
(Pennestrì and Freudenstein, 1993; del Castillo, 2002). In the frame of the
carrier the planet body is a shaft, each central member feeds it or draws from
it through one mesh, and which it does is the sign of its power in the *ideal*
flow; each mesh then passes `eta` of what enters it. With the carrier's moment
balance and the case's given torques that is a linear system with no search in
it -- the `eta_0^w` form, `w` the ideal flow's sign, for every preset below,
the three-ring compound included. The crate's flow instead tries each of the
`2^M` assignments of which side of each mesh drives and keeps the best
consistent one (`gear_core::train::flow`).

# What is read, and what is assumed

Every number is read from `tools/golden/graph.txt` -- the teeth, each mesh's
efficiency once moving, the path's ratio and efficiency -- so nothing here can
go stale without the corpus moving first. What is written here is only each
preset's **topology**: which member rides which body, which is a ring, which
body is the meshes' frame, and which the preset holds, drives and loads.

The static figure is not printed anywhere (it is no efficiency of anything
that moves), so it is **derived**: the crate's mesh loss is first order in
the coefficient (`gear_core::contact::efficiency`), and a preset's static
coefficient is twice its sliding one (`train::arrangements::FRICTION`), so
at rest every mesh loses exactly twice what it loses running.
"""

import pathlib
import re
import sys

import numpy as np

CORPUS = pathlib.Path(__file__).resolve().parent / "golden" / "graph.txt"
STATIC_OVER_SLIDING = 0.16 / 0.08

# Each preset's topology: member -> (body, is a ring), the meshes' frame, and
# the body held, driven and loaded by the preset's convention.
PRESETS = {
    # sun 24 in, ring 60 held, ring 59 out; planet 18/17 on one body
    "compound": {
        "members": {1: ("b1", False), 2: ("b2", True), 3: ("b3", True),
                    4: ("b5", False), 5: ("b5", False)},
        "frame": "b4", "planet": "b5", "held": "b2", "input": "b1", "output": "b3",
    },
    # carrier in, ring 60 held, ring 61 out; one planet gear meshing both
    "wolfrom": {
        "members": {1: ("b4", False), 2: ("b2", True), 3: ("b3", True)},
        "frame": "b1", "planet": "b4", "held": "b2", "input": "b1", "output": "b3",
    },
    # sun in, ring held, carrier out -- the control: it breaks away both ways
    "planetary": {
        "members": {1: ("b1", False), 2: ("b4", False), 3: ("b3", True)},
        "frame": "b2", "planet": "b4", "held": "b3", "input": "b1", "output": "b2",
    },
}


def closed_form(preset, read, scale):
    """Forward and backward efficiency by the carrier-frame power flow, and
    the forward ratio, input over output.

    Each mesh joins one central member X to the planet body. With the carrier
    H and the relative speed `u = w_p - w_H`, X turns at
    `w_X - w_H = s_X (z_pX / z_X) u` -- `s_X` +1 for a ring, -1 otherwise.
    The held body is still and the input turns at one. The torques satisfy
    the whole train's moment balance `sum T = 0` (every body is coaxial but the
    planet, which is carried), the input's `T = 1`, nought on a body that is
    neither held nor an end, and the planet's power balance in the carrier's
    frame, `sum eta_X^(w_X) T_X (w_X - w_H) = 0`, where `w_X` is the sign of
    X's relative power in the lossless flow: +1 where X feeds the planet and
    its mesh passes `eta` of that on, -1 where X draws and the planet must
    supply `1/eta` of it."""
    top = PRESETS[preset]
    planet, frame = top["planet"], top["frame"]
    central = {}
    for a, b, eta in read["meshes"]:
        pm, xm = (a, b) if top["members"][a][0] == planet else (b, a)
        body, ring = top["members"][xm]
        eta = 1 - scale * (1 - eta)
        central[body] = (read["teeth"][pm], read["teeth"][xm], 1 if ring else -1, eta)
    bodies = sorted(central) + [frame]

    def per_u(body):
        """(w_X - w_H) per unit u."""
        zp, zx, sx, _ = central[body]
        return sx * zp / zx

    def run(driven, loaded):
        # Speeds: w_X = w_H + per_u(X) u; the held body 0, the driven one 1.
        def speed_row(body):
            return [1.0, per_u(body) if body != frame else 0.0]

        a = np.array([speed_row(top["held"]), speed_row(driven)])
        w_h, u = np.linalg.solve(a, np.array([0.0, 1.0]))
        w = {b: w_h + (per_u(b) * u if b != frame else 0.0) for b in bodies}
        rel = {b: per_u(b) * u for b in central}

        def torques(weights):
            rows, rhs = [[1.0] * len(bodies)], [0.0]
            rows.append([weights.get(b, 0.0) * rel[b] if b in central else 0.0 for b in bodies])
            rhs.append(0.0)
            for b in bodies:
                if b == driven:
                    rows.append([1.0 if c == b else 0.0 for c in bodies])
                    rhs.append(float(np.sign(w[driven])))
                elif b not in (top["held"], loaded):
                    rows.append([1.0 if c == b else 0.0 for c in bodies])
                    rhs.append(0.0)
            t = np.linalg.solve(np.array(rows), np.array(rhs))
            return dict(zip(bodies, t))

        ideal = torques({b: 1.0 for b in central})
        weights = {}
        for b, (_, _, _, eta) in central.items():
            feeds = ideal[b] * rel[b] > 0
            weights[b] = eta if feeds else 1 / eta
        t = torques(weights)
        return -t[loaded] * w[loaded] / (t[driven] * w[driven]), w[driven] / w[loaded]

    (forward, ratio), (backward, _) = run(top["input"], top["output"]), run(top["output"], top["input"])
    return forward, backward, ratio


def section(name):
    """The preset's own block of the corpus: its path, members and meshes."""
    text = CORPUS.read_text()
    block = text.split(f"== {name} ==\n", 1)[1].split("\n== ", 1)[0]
    path = re.search(
        r"path (b\d+) -> (b\d+)\s+ratio (\S+)\s+efficiency (\S+) / (\S+) %", block)
    teeth = {int(m): int(z) for m, z in re.findall(r"^  member (\d+)\s+z (\d+)", block, re.M)}
    meshes = [
        (int(a), int(b), float(e) / 100)
        for a, b, e in re.findall(
            r"^  mesh \d+\s+members (\d+) (\d+) .*?efficiency (\S+) / \S+ %", block, re.M)
    ]
    return {
        "ends": (path.group(1), path.group(2)),
        "ratio": float(path.group(3)),
        "efficiency": (float(path.group(4)) / 100, float(path.group(5)) / 100),
        "teeth": teeth,
        "meshes": meshes,
    }


def main():
    fail = 0
    print(f"\n{'preset':<12}{'ratio':>12}   {'way':<9}{'running':>13}"
          f"{'at rest':>13}{'expected':>13}{'crate':>13}")
    for preset, top in PRESETS.items():
        read = section(preset)
        forward, backward, ratio = closed_form(preset, read, 1.0)
        rest_f, rest_b, _ = closed_form(preset, read, STATIC_OVER_SLIDING)
        ok = read["ends"] == (top["input"], top["output"]) and abs(
            ratio - read["ratio"]) < 1e-6 * abs(ratio)
        fail += not ok
        for way, run, rest, crate in zip(("forward", "backward"), (forward, backward),
                                         (rest_f, rest_b), read["efficiency"]):
            # Once moving where it breaks away, and nothing where it cannot.
            expected = run if rest > 0 else 0.0
            agree = ok and abs(expected - crate) < 5e-6
            fail += not agree
            print(f"{preset:<12}{ratio:>12.6f}   {way:<9}{run * 100:>12.6f}%"
                  f"{rest * 100:>12.6f}%{expected * 100:>12.6f}%{crate * 100:>12.6f}%"
                  f"   {'ok' if agree else 'FAIL'}")
    print()
    if fail:
        print(f"{fail} figure(s) disagree")
        return 1
    print("every figure breaks away where its flow at rest can start, and only there")
    return 0


if __name__ == "__main__":
    sys.exit(main())
