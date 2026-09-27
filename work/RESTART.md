# Restart manifest (soft pause, 2026-09-26)

Branch `audit-ablation`. Everything integrated is pushed. Nothing is merged to `main`.

## Done and integrated
- **Stage 0 (all):**
  - `tools/check_all.sh` (CI parity, read strictly from ci.yml and flake.nix);
  - golden-script fixes (T16.14, T20.1);
  - `gear-cli identity` with `tools/check_identity.sh <rev>` (bit-identity harness);
  - the module-homogeneity law (`train/homogeneity.rs`);
  - `Train::check` and the seeded edit walk (T13.1–T13.3), with the H1/H2 fixes and the `Loaded`
    rule (no load is ever dropped or moved).
- **Stage 1 P8 (train):** T10.1, T10.2, T10.3, T11.1, T11.2, T12.1, T01.1.
- **Plan documents:** `plan.md`, `stage1.md`, `design-graph.md`, `spike-contact.md`, and `review/`
  (including spike-verify).

## In flight at pause
Branches live in worktrees under `~/.cache/gearcalc-work/wt/`. Status notes are in
`~/.cache/gearcalc-work/notes/`.
- **work/s1-p3** (T03.1–T03.4, T04.1, T04.2, T04.4, T04.5). The worker finished all 9 commits. The
  check verdict is **FAIL**; see `pause/s1-p3-check.md`.
  - D1: once the depth cap binds, `minimum_profile_shift` loses its fixed point and silently falls
    back. It should return `Option`.
  - D2: replace the 0.95 factor with an exact cap, setting θ0 = π/z at the cap.
  - Minor: silent `unwrap_or`s in tooth.rs; validate_dxf's ring mode reads tip and root radii from
    the file rather than from the inputs.
  - Integration: identity.rs:334 needs `gear_to_dxf(&g, o)`, and wasm_boundary.json conflicts with
    P8's record.
  - Still unrun: the worker's proofs at base vs head.
  - Next: fix these, rebase onto audit-ablation, re-check.
  - P3 changes `span_over_teeth` to `Option` and moves `base_helix_angle` to `Tooth`; P7 must build
    on it.
- **work/s1-p6** (T08.1–T08.4, T08.6). The worker was paused mid-package; see
  `pause/s1-p6-status.md`. T08.1 (K_A) is done at 80668aa. The T08.2 WIP commit 454b271 holds
  exploration only. Next: finish, check, integrate.
- **Contact model prototype** (the owner's ruling: the preferred path for every mesh). Partial
  results are in `work/contact-model.md` (IN PROGRESS). Scripts are in
  `~/.cache/gearcalc-work/contact-proto/`, and the oracle is in `~/.cache/gearcalc-work/spike-verify/`.

## Ask the owner when contact work resumes
- Worm types as variants: ZN (preferred for low-cost manufacture), ZA, ZK. See the last section of
  `contact-model.md`: the flank as a parameter, and the conjugate-wheel question.

## Next, in order
1. Finish P3's check and P6, then integrate both.
2. P4 (ring) after P3; P5 (mesh and crossed; T07.2–T07.4 dropped) together with P7 (metrology,
   after P3); then P1 (gates, including the items carried in `stage1.md`) together with P9 (web);
   then P2 (test grids).
3. Stage 2: the items carried in `stage1.md`, plus redesign V.
4. Stage 3: the graph work M1–M12 (`design-graph.md`), then F, C, N, G, R, K, X and the contact
   model.

## Environment
- Watchdog, which stops at reboot: `nohup setsid ~/.cache/gearcalc-work/watchdog.sh >/dev/null 2>&1 </dev/null &`.
  It checks every 0.3 s, kills any process over 5 GB, and keeps a 3 GB buffer. Kills are logged to
  `~/.cache/gearcalc-work/watchdog.log`.
- Builds: `~/.cache/gearcalc-work/gc <dir> <cmd…>` (two slots).
- Agent conventions: `~/.cache/gearcalc-work/notes/WORKER.md` (workers) and `BRIEF.md` (reviewers).
- Integrate by fast-forward or cherry-pick after an adversarial check passes, then run
  `tools/check_all.sh --fast` and push.

Copies of the notes, conventions, watchdog and gc scripts are in `work/pause/`, in case
`~/.cache/gearcalc-work` is lost. The paused branches are pushed as `origin/work/s1-p3` and
`origin/work/s1-p6`.
