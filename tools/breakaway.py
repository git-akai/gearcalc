#!/usr/bin/env python3
"""Whether a drive breaks away, from a moment balance on every body.

A path's efficiency (`gear_core::train::PathReport`) is its flow once moving,
and **nought where the same flow against static friction cannot start**
(`Directional::once_moving`). The crate's flow counts only the power that
leaves, so a drive that cannot start -- one whose output would have to be
pushed as well -- reads as nought, and whether nought is "above" nought was
once decided by rounding: a compound set back-driven at rest broke away by
three parts in 10^15, alone and after a spur, and did not after an idler.

This reaches the same figures from the other end, **sign included**. Speeds
come from the rows of each mesh with the chosen body held; torques from the
moment balance of every body a case leaves free, each mesh's driven side at
its `eta`; and the efficiency is the output's power over the input's, which
is negative where the output must be driven too. Nothing below is the crate's
assignment search or its power filter.

    tools/breakaway.py                  # every preset below, both ways

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

import itertools
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
        "frame": "b4", "held": "b2", "input": "b1", "output": "b3",
    },
    # carrier in, ring 60 held, ring 61 out; one planet gear meshing both
    "wolfrom": {
        "members": {1: ("b4", False), 2: ("b2", True), 3: ("b3", True)},
        "frame": "b1", "held": "b2", "input": "b1", "output": "b3",
    },
    # sun in, ring held, carrier out -- the control: it breaks away both ways
    "planetary": {
        "members": {1: ("b1", False), 2: ("b4", False), 3: ("b3", True)},
        "frame": "b2", "held": "b3", "input": "b1", "output": "b2",
    },
}


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


def solve(preset, read, scale):
    """Speeds, and the signed efficiency each way, with every mesh losing
    `scale` times what it loses running."""
    top = PRESETS[preset]
    bodies = sorted({b for b, _ in top["members"].values()} | {top["frame"]})
    index = {b: i for i, b in enumerate(bodies)}
    # (a's body, a's signed teeth, b's body, b's signed teeth, eta)
    meshes = []
    for a, b, eta in read["meshes"]:
        (ba, ring_a), (bb, ring_b) = top["members"][a], top["members"][b]
        za = -read["teeth"][a] if ring_a else read["teeth"][a]
        zb = -read["teeth"][b] if ring_b else read["teeth"][b]
        meshes.append((ba, za, bb, zb, 1 - scale * (1 - eta)))
    frame = top["frame"]

    # Speeds: each mesh's pitch points move together in its frame,
    # za (wa - wf) + zb (wb - wf) = 0, with the held body still and the
    # input at one.
    rows, rhs = [], []
    for ba, za, bb, zb, _ in meshes:
        row = np.zeros(len(bodies))
        row[index[ba]] += za
        row[index[bb]] += zb
        row[index[frame]] -= za + zb
        rows.append(row)
        rhs.append(0.0)
    for body, value in ((top["held"], 0.0), (top["input"], 1.0)):
        row = np.zeros(len(bodies))
        row[index[body]] = 1.0
        rows.append(row)
        rhs.append(value)
    w = np.linalg.lstsq(np.array(rows), np.array(rhs), rcond=None)[0]

    def efficiency(driven, loaded):
        """The best consistent flow driving `driven` against `loaded`, as
        the loaded body's power over the driven one's -- negative where
        the loaded body must be driven too."""
        free = [b for b in bodies if b not in (top["held"], driven, loaded)]
        found = []
        for drivers in itertools.product((0, 1), repeat=len(meshes)):
            # The torque each mesh puts on each body, per unit of its force:
            # the driver's side whole, the driven side at eta, and the frame
            # the reaction of both.
            def on(k, body):
                ba, za, bb, zb, eta = meshes[k]
                ta, tb = (za, eta * zb) if drivers[k] == 0 else (eta * za, zb)
                t = 0.0
                if ba == body:
                    t += ta
                if bb == body:
                    t += tb
                if frame == body:
                    t -= ta + tb
                return t
            # External torque plus the meshes' is nought on every body; the
            # driven body's external torque is one, working with its motion.
            a = [[on(k, b) for k in range(len(meshes))] for b in free + [driven]]
            r = [0.0] * len(free) + [-np.sign(w[index[driven]])]
            try:
                c = np.linalg.solve(np.array(a), np.array(r))
            except np.linalg.LinAlgError:
                continue
            ext = {b: -sum(c[k] * on(k, b) for k in range(len(meshes))) for b in bodies}
            # Each driver gives power up into its mesh, in the mesh's frame...
            works = all(
                c[k] * on(k, meshes[k][0 if drivers[k] == 0 else 2])
                * (w[index[meshes[k][0 if drivers[k] == 0 else 2]]] - w[index[frame]])
                <= 1e-12
                for k in range(len(meshes))
            )
            # ...and the train loses power rather than making it.
            loss = sum(ext[b] * w[index[b]] for b in bodies)
            if works and loss >= -1e-12:
                p_in = ext[driven] * w[index[driven]]
                found.append(-ext[loaded] * w[index[loaded]] / p_in)
        return max(found) if found else None

    return w, index, (efficiency(top["input"], top["output"]),
                      efficiency(top["output"], top["input"]))


def main():
    fail = 0
    print(f"\n{'preset':<12}{'ratio':>12}   {'way':<9}{'running':>13}{'at rest':>13}"
          f"{'expected':>13}{'crate':>13}")
    for preset, top in PRESETS.items():
        read = section(preset)
        w, index, running = solve(preset, read, 1.0)
        _, _, resting = solve(preset, read, STATIC_OVER_SLIDING)
        ratio = w[index[top["input"]]] / w[index[top["output"]]]
        ok = read["ends"] == (top["input"], top["output"]) and abs(
            ratio - read["ratio"]) < 1e-6 * abs(ratio)
        fail += not ok
        for way, run, rest, crate in zip(("forward", "backward"), running, resting,
                                         read["efficiency"]):
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
