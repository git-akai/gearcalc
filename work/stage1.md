# Stage 1 — gates that can fail, and the high-severity wrong answers

Packages group the audit's Phase 1 tasks by the files they touch. At most two run at once. Each is
checked adversarially before it is integrated. Adjustments from the working plan are marked ▲.

| Pkg | Tasks | Notes |
|---|---|---|
| P3 tooth + export | T03.1, T03.2, T03.3, T03.4, T04.1, T04.2, T04.4, T04.5 | ▲ T03.4 clamps with a note for both kinds, per decision 1; `TIP_ABOVE_BASE_FRACTION` goes |
| P8 train | T10.1, T10.2, T10.3, T11.1, T11.2, T12.1, T01.1 | ▲ T11.1: the refusal is set by the representation (the 2^M mask), not by a chosen cap. Redesign F replaces it. ▲ T01.1 shares `Train::check` |
| P4 ring | T05.1, T05.2, U1 (interim) | ▲ Interim fixes, each with the law G/R will keep. No sign-valued κ special case (PC-1) |
| P5 mesh + crossed | T06.1, T07.1 | ▲ T07.2–T07.4 are dropped: their premise is refuted (spike-verify §5). crossed-worm#0 gets a note now; its cure is the S-C model |
| P6 strength | T08.1, T08.2, T08.3, T08.4, T08.6 | ▲ T08.2 returns `Option` (decision 2). T08.6's allowables come from cited sources |
| P7 metrology | T09.1, T09.2 | |
| P1 gates | T18.2 (the rest of it), T16.1, T16.2, T16.8, T16.15–T16.19, T17.1 | after s0-tools |
| P9 web | T19.1, T19.2 | |
| P2 test grids | T16.3 → T16.4, T16.5; T16.6 → T16.7; T16.21 | after P8 |

Order: P3 ∥ P8, then P4 ∥ P6, P5 ∥ P7, P1 ∥ P9, then P2.

Carried into P1 from the s0-tools check (minor):
- `identity` edits only presets asked alone; add multi-part trains (join, split, insert).
- `identity` labels edit cases by offer index; label them by the edit instead.
- `check_all.sh` neither runs nor flags a `uses:` step.
- `check_golden`'s `.golden.old.$$` is outside the cleanup trap.
- The homogeneity allowance's κ is a heuristic; its comment should say so.

Carried into Stage 2 from the P8 check:
- Path efficiency shows 0.00 when a flow is refused. It is now reachable through Worm AddGear edits 18 and 23 (0-for-none, P1 pattern).
- Genuine flow ties (zero relative speed, zero tie) are settled by efficiency and then enumeration order. Redesign F should settle these.

Carried from the P3 check:
- Stage 2 (typed absence): `edge_of_undercut` reads a Brent failure as "no edge"; u_tip and u_j are
  NaN on severed teeth.
- P4 (ring): `ring.rs:218` does not guard α.
- P1: validate_dxf models only the unclamped cut. Eccentric z40 x1 Δx0.5 fails, on the safe side,
  and no CI case reaches it.

Carried from the P6 re-check:
- The Lewis section, held at the highest point of single-pair contact under load sharing, reads up to
  21 % low (unconservative) against a fresh search. Revisit when the unified contact model sets the
  load positions: a continuous section search that survives the pointed limit.
