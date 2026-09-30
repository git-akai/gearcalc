# Stage 2 — boundaries, the error model, and the structure they stand on

The audit's Phase 2, the items Stage 1 carried here (`stage1-exit.md` §2), and the order changes
the exit review recommended: graph steps M1–M3 first, and T16.13 as a gate. At most two packages
run at once. Each is checked adversarially and closes with a proof table (see WORKER.md rule 6).

| Pkg | Tasks | Notes |
|---|---|---|
| Q1 structure | design-graph.md M1–M3: `structure.rs` with DisjointSets (all five union-finds replaced); one `Incidence` built per solve; `CarrierTree` and `Shape::validate` (carried_by acyclic; one axis per line in a frame; dense bodies; longer carried-axis cycles) | Behaviour-neutral: `check_identity` is bit-identical except where invalid input is newly refused. M0 structure snapshot first |
| Q9 gates and tools | T16.13 (an independent finite-z bending value gate); cargo-mutants into the dev shell; the literal scan (a bare float below 1e-3 outside a named const) and the absence diff gate (WORKER rules 2–3) in check_all | Before anything touches rating (L, contact model) |
| Q2 validator V | T01.2–T01.11, one validator table per kind of input, called wherever input enters (import, every wasm entry, the CLI), refusing under a key that names the field; T14.6 | After Q1. It empties the known-panic lists (Stage 2 exit). The helix-90° and NaN file trap is item 1 |
| Q4 wire and format | T14.1 (generate every request type; deny_unknown_fields), T14.4 (versioned file format; frozen stage-era DTO), T14.12, T14.14, T05.6 (ring's fewest teeth in `ring.rs`), T09.3 (over-pins around, in the core) | |
| Q3 error model | T02.1–T02.9; absence items 2, 4, 5, 6; refusal keys that name the cause (item 7 and pattern G); T01.12 and T06.3 inside T02.8 | After Q2 (T02.6 and T02.7 follow T01.2/6/8). T02.6's tip bullet is superseded by decision 1 |
| Q5 cases | T13.4 (refuse, never move a load: decision 6), T13.5, T13.6, T13.7, T13.8, T13.9, T13.12 | |
| Q6 domain edges | T03.5, T05.5, T05.8, T06.4; items 3 and 8 (a zero automatic face width; MeshedPlanets margin) | T05.4 dropped (decision 4); T07.3 dropped (spike-verify §5) |
| Q7 UI | T19.3, T19.4, T19.7–T19.12 | |
| Q8 harness | T20.4, T20.6, T20.13; item 9 (T01's stale NoRootSection bullet is a doc fix) | |

Order: Q1 ∥ Q9 → Q2 ∥ Q4 → Q3 ∥ Q5 → Q6 ∥ Q7 → Q8.

## Exit
The audit's Phase 2 exit, plus:
- The known-panic lists (T16.21) are empty.
- The absence gate is green over all of gear-core.
- Every refusal key has a fixture on either side of its cause.
- Every value a note quotes is the report's own value (L4, T16.20).

## Added from the notch research (`notch-research.md`)
- **Q10: the default bending rating's bias.** The research's boundary-element (BEM) check says the
  crate's default Dolan–Broghamer rating is net unconservative on the hardened steel: −2 … −11 %
  median, worst −26 %.
  - Most of it comes from the subtracted axial term.
  - Rating against a polished-coupon endurance limit with no surface factor adds roughly 15–35 %.
  - The crate's own ISO/DB ratio (1.25 median) supports it, but the BEM's Peterson-shoulder canary
    was never run.
  - Steps: run the canary (to within 0.5 %); re-run the BEM grid; record the verified bias with its
    size and sign in state.md (rule 6); bring the choice of default (DB or ISO Y_S), and the missing
    surface factor, to the owner.
  - Runs after Q9, whose independent bending gate rebuilds DB.
- **Also from it:** hardness ÷ 3 estimates σ_u, not σ_y (reading it as σ_y is 10–60 % unconservative).
  Check every place the crate or its material notes derive σ_y from hardness.
