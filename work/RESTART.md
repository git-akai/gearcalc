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
  adversarial check was interrupted; its partial result is in `notes/s1-p3-check.md`.
  - Open concern: T03.3's cap uses a bare 0.95 factor.
  - Next: finish the check, fix, then rebase onto audit-ablation (the branch is based on f079797).
  - P3 changes `span_over_teeth` to `Option` and moves `base_helix_angle` to `Tooth`; P7 must build
    on it.
- **work/s1-p6** (T08.1–T08.4, T08.6). The worker was paused mid-package; see
  `notes/s1-p6-status.md`. Next: finish, check, integrate.
- **Contact model prototype** (the owner's ruling: the preferred path for every mesh). Partial
  results are in `work/contact-model.md` (IN PROGRESS). Scripts are in
  `~/.cache/gearcalc-work/contact-proto/`, and the oracle is in `~/.cache/gearcalc-work/spike-verify/`.

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
