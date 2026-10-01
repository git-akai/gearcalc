# Restart manifest (soft pause at a session limit, 2026-10-01)

Branch `audit-ablation`, in sync with origin. Merges into `audit-ablation` are allowed; into `main` or `train-as-graph`, never (owner, 2026-10-01).

## Integrated
- **Stage 0 and Stage 1:** complete (see `stage1-exit.md`).
- **Stage 2:**
  - Q1 (graph structure M1–M3), Q2 (input validator), Q4 (wire and format), Q5 (cases).
  - Q9 (gates and tools), Q10 (bending bias recorded), Q11 (validators classified, mirrors retired).
  - The surface notes for Y_RrelT and Z_R.
- **Bending round 3** written up in `bending-mechanics.md` §11 and `bending-options.md` (616530a).
- **Face-width call** recorded in `plan.md` §5 (de1fba0).
- **Q3** (error model): checker PASS, merged as ad7a385. Optional, not blocking: bound duration and sweep in `input.rs` to what a duty can mean, which would let `ratio::Natural` give way to i128.

## In flight — every branch pushed to origin
| Branch | State | To resume |
|---|---|---|
| `work/s2-q6` (Q6, domain edges) | Paused at WIP 203cd6b. **Tests red:** 19 of 908 failed at the last run, a run that predates the newest laws. Items done: 1 (the converter law is pinned), 4 (ties in matrix.txt within `solve::SQRT_EPSILON`), 5 (the move from dc77eb8 is in corrections.md: 29 trains, six walks). Items 2 and 3 (face width unsized where nothing asks; a capped stress landing on its allowable by construction) are written but not green. Worker a780394c859a0e3e7, checker a29c982794e9f01ac | Run `cargo nextest run --workspace --no-fail-fast` in the worktree. Fix the 19: laws still on the old rule, the relief freedoms, the hula corpus, entries in wasm's absent list, the strings sweep. Then run bindings, wasm and golden `--write` and classify every moved line; then `check_all --fast` and npm check/test. Then a checker. Have it question whether √ε is the right derived tolerance for a tie, or just a convention (WORKER rule 2) |
| `work/field` (contact model, Rust port) | P1 and P2 passed; P3 at 1654551. Run wf_7bac2b43-fb1 was stopped right after `check:P3:1` returned **FAIL** (journal: agent a1aad37a7b5427886), as `fix:P3:1` started. That fix may have left partial edits uncommitted in the worktree. The failures: (1) a region maximum missed by about 1.5e-4 on 2–3 of about 100k random strips, where a weak curvature step near the Hertz peak hides an interior maximum between cosine samples; the fix is to bracket every sign change of p′ by construction, and add the two strips and an adversarial family as fixtures; (2) no test checks the creep curve's values on a non-Hertz strip, since a planted Carter formula passes; the code itself is right | Check `git status` in the field worktree and drop or commit any partial edits. Then resume: `scriptPath …/workflows/scripts/contact-field-rust-port-wf_a57d209f-d46.js` with `resumeFromRunId wf_7bac2b43-fb1`. Everything up to `check:P3:1` is cached; `fix:P3:1` reruns |

The workflow scripts live under `~/.claude/projects/-home-user-gearcalc/97717eac-…/workflows/scripts/`.

## Not started
- **Stage 2:** Q7 (UI: the panel's MeshRanges boxes, and solve_gear's refusal contract), Q8 (harness).
- **Stage 2 exit:** every refusal key with a fixture on each side. 5 keys never fire on the preset sweep: family, not_carried, no_distance, no_such, axis.
- **Stages 3–6** (`plan.md` §3).

## Open owner decisions
- **Bending fast-mode default** (round 3: no fast option is free of substantial structured error):
  - whether the per-tooth bisection and per-load Brent count as a "per-geometry solve";
  - whether to build a table of influence functions (S_V and S_N at each fillet station), or accept R4+NF (4.4–4.9 µs per section in wasm, continuous, five substantial structures).
- **Face width:** the orchestrator's call stands unless the owner overrides it. Automatic width is unsized where nothing asks; crossed and worm members keep a given width.

## Environment
- Watchdog: it stops at every reboot. Start it with
  `nohup setsid ~/.cache/gearcalc-work/watchdog.sh >/dev/null 2>&1 </dev/null &`. Verify it by its
  command line, not a stale PID: `ps -eo pid,args | grep '[b]ash .*watchdog.sh'`.
- Builds go through `~/.cache/gearcalc-work/gc <dir> <cmd…>`.
- Conventions: `~/.cache/gearcalc-work/notes/WORKER.md`; a copy is in `work/pause/`.
- Research scratch, outside the repo: `~/.cache/gearcalc-work/{contact-proto,bending-*,notch-*,…}`.
