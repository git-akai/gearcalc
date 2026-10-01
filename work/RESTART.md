# Restart manifest (soft pause at a session limit, 2026-10-01)

Branch `audit-ablation` @ de1fba0, in sync with origin. Nothing is merged to `main`.

## Integrated
- **Stage 0 and Stage 1:** complete (see `stage1-exit.md`).
- **Stage 2:**
  - Q1 (graph structure M1–M3), Q2 (input validator), Q4 (wire and format), Q5 (cases).
  - Q9 (gates and tools), Q10 (bending bias recorded), Q11 (validators classified, mirrors retired).
  - The surface notes for Y_RrelT and Z_R.
- **Bending round 3** written up in `bending-mechanics.md` §11 and `bending-options.md` (616530a).
- **Face-width call** recorded in `plan.md` §5 (de1fba0).

## In flight — every branch pushed to origin
| Branch | State | To resume |
|---|---|---|
| `work/s2-q3` (Q3, error model) | 33eaf38 answers the checker's second FAIL: exact cycle counts (`ratio::Natural`), one ranking of candidates for both search and solve, the one-sided band extreme. The final re-check was cut short by the pause; see below for its verdict. Worker a84cd21d387940331, checker a48902005e6b3e1a6 | Finish the re-check. Questions open on it: is `Natural` needed or would a smaller exact route do, its arithmetic at limb boundaries, and no jump at the 2^53 cut-over. Then integrate: `check_all.sh --fast` and push |
| `work/s2-q6` (Q6, domain edges) | Round 3 is in WIP commits (5c477ce and on). It answers the checker's second FAIL and builds the face-width call. Worker a780394c859a0e3e7, checker a29c982794e9f01ac | Read the head commit's message for what is done and left among the 5 items: the converter law pinned; automatic width unsized where nothing asks (GearResult.face_width an Option, the box bounded only where read, format-1 files with box 0 read); a capped stress landing on its allowable by construction; ties in matrix.txt; the move from dc77eb8 recorded (29 trains, walks 36/38/101/103/166/168). Finish, have it checked, integrate |
| `work/field` (contact model, Rust port) | P1 and P2 passed; P3 at 1654551. The workflow is stopped after its step `check:P3:1` | Resume the workflow: `scriptPath …/workflows/scripts/contact-field-rust-port-wf_a57d209f-d46.js`, `resumeFromRunId` = the latest run id (see below). Completed agents are served from the cache |

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
