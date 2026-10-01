# Restart manifest (clean shutdown for diagnostics, 2026-10-03)

Branch `audit-ablation` @ 0fff733, in sync with origin. Nothing is merged to `main`. Nothing was
running at shutdown.

## Integrated
- **Stage 0 and Stage 1:** complete (see `stage1-exit.md`).
- **Stage 2:** Q1 (graph structure M1–M3), Q2 (input validator), Q4 (wire and format), Q5 (cases),
  Q9 (gates and tools), Q10 (bending bias recorded), Q11 (validators classified, mirrors retired),
  and the surface notes for Y_RrelT and Z_R.

## In flight — every branch pushed to origin
| Branch | State | To resume |
|---|---|---|
| `work/s2-q3` (Q3, error model) | WIP at df98f7c. Its tests may be red. Agent ref a84cd21d387940331 | Rebase onto audit-ablation, finish the brief in `stage2.md` (Q3), have it checked, integrate |
| `work/s2-q6` (Q6, domain edges) | WIP at f9f3faf. Agent ref a780394c859a0e3e7 | Same |
| `work/field` (contact model, Rust port) | P1 and P2 passed; P3 in progress (ec802ef/4d31e2e) | Resume the workflow: `scriptPath …/workflows/scripts/contact-field-rust-port-wf_a57d209f-d46.js`, `resumeFromRunId wf_7bac2b43-fb1` |
| Bending round 3 (research; no repo branch) | Tracks part-done | Resume: `…/bending-round-3-wf_5f01b8e9-b8c.js`, `resumeFromRunId wf_5f01b8e9-b8c` |

Both workflow scripts live under `~/.claude/projects/-home-user-gearcalc/97717eac-…/workflows/scripts/`.

## Not started
- **Stage 2:** Q7 (UI), Q8 (harness).
- **Stages 3–6** (`plan.md` §3).

## Open owner decisions
- **Bending fast-mode default:** keep DB with its bias recorded, or pick from
  `bending-options.md` once round 3 reports.

## Environment
- Watchdog (not running at shutdown, and it stops at every reboot): start it with
  `nohup setsid ~/.cache/gearcalc-work/watchdog.sh >/dev/null 2>&1 </dev/null &`. Verify it by its
  command line, not a stale PID: `ps -eo pid,args | grep '[b]ash .*watchdog.sh'`.
- Builds go through `~/.cache/gearcalc-work/gc <dir> <cmd…>`.
- Conventions: `~/.cache/gearcalc-work/notes/WORKER.md`; a copy is in `work/pause/`.
- Research scratch, outside the repo: `~/.cache/gearcalc-work/{contact-proto,bending-*,notch-*,…}`.
