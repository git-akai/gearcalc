# Restart manifest (soft pause, 2026-09-30)

Branch `audit-ablation`. Everything integrated is pushed, unless the P1 line below says otherwise.
Nothing is merged to `main`.

## Integrated
- **Stage 0 (all):**
  - `tools/check_all.sh` (CI parity);
  - the golden-script fixes;
  - `gear-cli identity` with `tools/check_identity.sh <rev>`;
  - the homogeneity law;
  - `Train::check` and the seeded edit walk, with the H1/H2 fixes and the `Loaded` rule.
- **Stage 1:**
  - P8 train;
  - P3 tooth and export;
  - P6 strength;
  - P7 metrology;
  - P4 ring;
  - P9 web;
  - P5 mesh and crossed.
- **P1 gates:** fast-forwarded locally to 815170a. Its full `tools/check_all.sh`, nix builds
  included, was running at the pause.
  - If `git log origin/audit-ablation` lacks 815170a, re-run `tools/check_all.sh` and push if green.
  - Its checker passed every item after two rounds. The last round's four holes were fixed and each
    plant was seen failing.

## In flight at pause
- **work/s1-ringtip** (worktree `~/.cache/gearcalc-work/wt/s1-ringtip`). The preset ring tips are
  sized by the mesh. The owner ruled the tip bound on by default for every gear.
  - The second check failed on four items:
    - the held-tip law must compare radii directly;
    - tag a side only when the built tip lands on its bound;
    - the 12/29 claim should be checked against the 401-point scan;
    - identity needs searched and long-addendum external cases.
  - The worker was fixing these at the pause; see the branch log and any WIP commit. Next: finish,
    re-check, integrate.
- **Contact model, round 4** (Python only). See `work/contact-model.md` ("ROUND 4 IN PROGRESS" if
  paused mid-round) and `review/contact-verify4.md`.
  - Priorities: continuous line seeding (the helical r_e basin flip), a smooth tip form, face-end
    convergence, the tooth-stiffness gap (1.4×), and the matched worm wheel as a surface.
  - Code is in `~/.cache/gearcalc-work/contact-proto/`.
  - After it: an independent verification (round 5), as for every round.

## Owner's rulings (see `plan.md` §5)
- Two modes: fast closed forms at ISO's points for the live answer; the contact field model as the
  expensive, on-demand final analysis.
- The tip bound is on for every gear.
- Worm types are evaluated. A matched set is exposed only if it needs no extra branch or solve.
- Decisions still pending: the worm wheel (involute or matched), and requiring a smooth tip form in
  the expensive mode. Ask after round 4 is verified.

## Next, in order
1. Push P1 if it is not yet pushed. Finish and integrate the ring-tip branch.
2. P2 (test grids): T16.3 → T16.4, T16.5; T16.6 → T16.7; T16.21. That closes Stage 1.
3. Stage 2: the items carried in `stage1.md`, plus redesign V.
4. Stage 3: the graph work M1–M12 (`design-graph.md`), then F, C, N, G, R, K, X and the contact
   model in Rust. The search redesign (C/R3) takes P1's Layshaft cost finding.

## Environment
- Watchdog, which stops at reboot: `nohup setsid ~/.cache/gearcalc-work/watchdog.sh >/dev/null 2>&1 </dev/null &`.
- Builds: `~/.cache/gearcalc-work/gc <dir> <cmd…>` (two slots). Never leave a server running under
  it, because a server keeps holding its slot.
- Conventions: `~/.cache/gearcalc-work/notes/WORKER.md` and `BRIEF.md`. Copies are in `work/pause/`.
- Integrate after an adversarial check passes, then run `tools/check_all.sh --fast` (full when CI
  changes) and push.
