# gearcalc — working plan (implementation of the audit, and what it missed)

This plan builds on the audit (`audit/`, which stays unedited). It does not repeat the audit's
tasks. It records what changed after a second review, in what order the work lands, how each change
is checked, and what the owner still has to decide. Task ids `Tnn.m` are the audit's.
`PC-`, `U`, `H` and `S` ids refer to the four review reports in [`review/`](review/).

Baseline: `audit-ablation` @ 03f9823. The code is unchanged since the audit (a2f2234). 679 tests
pass in 36 s.

## 1. What the second review found

Four reviewers ran after the audit: a critic of the plan, a unification review, a bug hunt in the
least-audited files, and a skeptic of the audit's own claims. The main results:

**The audit's plan contradicts itself in places.**
- Some tasks are undone by later ones, e.g. T08.2 → `Option` then T17.7 → `f64`, and T07.2 then
  T07.4.
- The tip-at-or-below-base-circle rule has four incompatible policies (PC-13).
- About 20 edits land twice.
- Ten tasks still offer "A or B" (PC-35).
- The register gate, T18.28, passes if someone just bumps a commit hash (PC-36).
- The plan says 144 surviving mutants; the figure is 181.

**Its "What not to touch" list is partly false (S).**
- Malformed TOML *does* trap (graph-ops#1).
- Some output wire types are hand-written (tools-ci#10).
- The epicyclic search does not converge on Planocentric (added2#94).
- Module homogeneity holds in the core but not in the presets: millimetre defaults break it. The
  Worm preset refuses at m = 10 and Planocentric at m = 0.1.

**New defects:**
- **H2 (medium):** an offered edit builds a gear that meshes in two frames. The whole train then
  fails as a `Wiring` "preset defect", about 5,900 hits in a two-step sweep. This refutes
  shape-a#8's "refused cleanly".
- **H1 (low–medium):** a case's flow grouping drops meshes, and sometimes a whole planetary part, on
  edited trains.
- **U1 (medium):** ring members silently ignore `no_sharp_tip`/`min_tip_width`, get no undercut
  floor, and use a different search bound. This is a live breach of rule 4.
- **U8 (medium):** planet clearance assumes equal spacing and never checks planets on the other axes
  of the same carrier.
- **PC-11/PC-32:** the guard conventions in `params.rs` (0.05, 0.9, 0.02, 0.95, 0.95,
  `MAX_TOOTH_THICKNESS_FRACTION_OF_PITCH`) are hidden rules of thumb that no task owns.
- **Smaller items:**
  - About 15 silent `teeth.max(1)` clamps (PC-30).
  - Zero used as "absent" at hertz.rs:300, strength.rs:597, and screw.rs:935 and :1005 (PC-31).
  - A dead zero branch in flow.rs and an absolute ε that can void a crossed average (U9–U12).
  - An SVG backslash (H3).
  - Stale comments in auto.rs and gear-cli main.rs:1542 (S).

**Where the audit patches instances, one structural change removes the whole class.** The
redesigns below replace roughly 45 audit tasks and delete more code than they add.

| Redesign | What it is | Replaces |
|---|---|---|
| **G — one generator** (PC R1, U) | Two independent parameters: workpiece side σ = ±1 and tool pitch curvature κ (the rack is κ = 0, a continuous parameter rather than a special case). One rolling corner in normal-angle form. One domain rule, κ_c + σκ_w > 0, replaces three separate refusals. | T03.13–15, T05.2/9/14/15/17, T14.8 (member half), T04.1's walker, U1, U7. Deletes about 600 lines of mirrored ring machinery, `BuiltMember` and the `auto::Cut` dispatch. |
| **F — flow by blocks** (U2, PC R2) | Block-triangular decomposition of the torque system (matching plus strongly connected components). A 1×1 block's direction is forced; enumeration survives only inside the irreducible blocks, which are the loops where power circulates. Exact, with no seed and no fallback. Prototype: matches 2^M on 2,998 random trains. | T11.1's global cap (becomes per block), T11.4 steps 2–4, T11.15, T11.8, T15.12 (conditioning). T11.4's two-algorithm design is dropped. |
| **C — closure decomposition** (U3, PC R3) | One structural decomposition of shifts against running distances. It gives: relief admissibility, plan roles independent of list order, closed-form absorbers, and the search's free directions and components. | T10.5, T10.6, T10.9, T12.2, T12.3, T14.7, part of T12.10. The search heuristics T12.7/8/12 are replaced by per-component exact division. |
| **R — one signed clearance record per mesh** (PC R4) | Tip crossing, far gap, bottom clearance and interference depth for each mesh, read at both ends of the band. Sizing, the search filter and the report all read it. | T03.5, T05.1, T05.3, T05.12, T06.2, T10.10, T10.16. Deletes `RingMesh`. |
| **L — one load field** (U4) | Share per pair and contact-line length per instant. It feeds bending share, efficiency and contact stress for line and point contact alike. | T06.5, T06.8, T07.19, added#28. Fixes strength#6 (Z_ε omitted). |
| **K — thickness as a per-member allowance** (PC R6) | Backlash comes from per-member thickness deviations. The absolute 0.02 ± 0.02 mm band stops being the only source of backlash. | T10.4's odd-cycle special case, T15.3, T21.3. Cures lens-standards#6 and the module-homogeneity break (S). |
| **X — crossed face as one interval model** (PC R7) | The face as one interval on the contact's axial coordinate. | T07.2/4/5/15, T08.12; removes the Phase-1/Phase-4 reversal. |
| **V — one validator table** (PC R5) | One validator for every kind of input, each refusal keyed by its field. | T01.2, T13.1, T14.6, T01.6/8/9, T05.8, T13.5, parts of T02.6/7. |
| **N — one train tolerance module** (PC R8) | Relative, power-normalised zero tests with their derivation stated. | T11.10 b4, T15.7, T15.12, T16.28. |

**Reviewed and not taken as proposed** (U; owner's rulings 2026-09-26):
- **One efficiency model — taken after all.** The parallel closed form becomes the exact value of
  the same path integral the crossed mesh uses. A law holds the two to agree, so there is one model
  with a fast closed-form route. This lands inside redesign **L**.
- **One contact geometry — still wanted in principle; investigate alternatives (spike S-C).** A
  general numerical tooth-contact analysis is rejected: parallel gearing is not a uniform Σ → 0 limit
  of crossed (second-order distance law, 1-D vs 2-D contact, a face-bounded boundary layer). The
  spike looks for a model that holds both, need not be a published standard, and is accepted if its
  reasoning is sound and its results agree with both closed forms where they apply. Candidates:
  - a face-sliced contact field, with each slice a signed-curvature Hertz contact;
  - the parallel mesh written as the crossed model with its face bound carried explicitly;
  - a line contact taken as the elliptical limit along the instantaneous contact lines.
  Until a candidate passes, the core `Path` interface unifies the two, and the differences between
  them are recorded with size and sign.

Rejected, as recommended:
- One geometry for line and point contact. The interface is unified instead, through a core `Path`
  trait.
- A general contact-analysis engine.
- A linear or complementarity programme (LP/LCP) for the flow.
- Trait objects for the contact enum.
- Merging the exact-rational numerics with the floating-point ones.

## 2. Decisions this plan makes, by the owner's stated principles

These settle the open choices the audit left (PC-13, PC-35). Each is applied once, everywhere.

1. **Tip at or below the base circle:** clamp with a note, for both kinds (rule 5, continuous).
   `TIP_ABOVE_BASE_FRACTION` goes. T02.6, T05.13, T05.15, T15.11 and T16.23 are rewritten to match.
2. **Absence is typed:** `bending_factor` returns `Option`. T17.7's first bullet is dropped.
3. **Rules of thumb become visible options:**
   - The `params.rs` guard conventions, the best-k rule, the 1.75 mm pin, μ = 0.08 and the
     locking/efficiency thresholds all become named, user-visible settings with their source stated.
   - The model underneath each one stays continuous.
4. **No cap without a derivation:**
   - T05.4's minimum α_w and T11.1's global M cap are dropped.
   - A cap survives only where the cost or the domain derives it, e.g. inside an irreducible flow
     block.
5. **Unpublished formulas are not shipped as the model.** T08.11's interpolation becomes an option
   labelled as an estimate, or it is dropped. T08.6's "closed form C(κ)" claim is corrected.
6. **A load is never moved to a guessed body.** Where T13.4 would do that, the edit is refused under
   a named key.
7. **The register gate names a command.** CI re-runs it when the files it covers change (PC-36).
8. **Priority follows the verifier's severity, not the auditor's.**
9. **The train graph is a known structure, handled by known methods**, even where they are more
   than today's trains need.
   - Representation: the kinematic graph of gear-train theory (Buchsbaum–Freudenstein / Tsai).
     Links are vertices; gear pairs and turning pairs are edges labelled by axis. Fundamental
     circuits and transfer vertices come from this representation.
   - Algorithms: standard graph algorithms, named in the code — union–find, Tarjan's strongly
     connected components, Hopcroft–Karp matching, Dulmage–Mendelsohn decomposition.
   - Each algorithm is written once, in `structure.rs`, with its reference. Nothing is ad hoc.
10. **Gears are stand-alone entities; a mesh is the link between two of them.**
    - A gear's form, options and checks belong to the gear, and it never reads its mate.
    - Whatever needs both gears (distance, play, clearance, contact) lives on the mesh.
    - Where a gear depends on its mate today, a small rewrite is preferred over a small dependency.
      Redesigns **K** (thickness is the member's) and **R** (clearance is the mesh's) follow this.
11. **Units are checked for intent before they change.** Some millimetre figures are deliberate:
    axis distance, for one.
    - No value moves between mm and module without confirming, from its history, docs and use, that
      mm was not chosen on purpose.
    - Candidates: the 0.02 ± 0.02 mm clearance band, the worm's 0.04 mm axial clearance, the
      1.75 mm pin, the 7 mm worm.
    - The module-scaling law scales the mm fields too. It tests the model's homogeneity, not what the
      defaults should be.
12. **`handoff_inbound/` stays**: it is the source of the seven regression fixtures.
   `geartrain-refactor-*.md` are deleted, as they promise; this is T18.

## 3. Order of work

Each stage ends with every check green (§4). A stage is pushed to `origin/audit-ablation` when it
closes. Nothing is merged to `main`.

**Stage 0 — The instrument** (small, first):
- `tools/check_all.sh` runs every check CI runs (T16.14/T20.1).
- A bit-identity harness: every preset × case, solved and hashed, so a refactor that should not move
  numbers can prove it.
- The seeded edit walk (T13.1) with H1's flow law and H2's frame law added.
- A module-scaling law over the presets (S).
- The fixes to the "What not to touch" register.

**Stage 1 — Gates that can fail, and the high-severity wrong answers.** The audit's Phase 1, with
these changes:
- Items that the redesigns replace get only a small interim fix plus a failing law, so the law
  carries over. T11.1 is an example: refuse above M = 31 until F lands.
- H2 is added: refuse the edit under a named key now, then apply the per-mesh frame cure.
- U1 is added, as an interim: the ring honours the tip-width and undercut floors.

**Stage 2 — Boundaries and errors.** The audit's Phase 2, built as redesign **V**, and the typed
absences of PC-31. Also PC-30's silent `teeth.max(1)` clamps, which become refusals.

**Stage 3 — Structure.** The redesigns, in order: **F** → **C** → **N**, then **G**, **R**, **L**,
**K** and **X** as parallel tracks where the files do not overlap.
- F runs first because it also serves as Phase 3's performance work.
- Each redesign lands in behaviour-neutral steps under the identity harness, then with its intended
  number changes recorded in the corpus.
- T14.5's split of `train/mod.rs` runs before C.
- Spike **S-C** (unified contact geometry) runs as a research track beside G/R/L. It uses Python
  prototypes against both closed forms and produces a report before any code lands. If it succeeds,
  it replaces the `Path` interface's two implementations.
- The graph work follows [`design-graph.md`](design-graph.md), steps M1–M12.
  - M1–M4 are behaviour-neutral: disjoint sets, one incidence index, carrier-tree validation, and the
    transfer vertex computed beside today's frame.
  - M5 switches each mesh's frame to its transfer vertex. That is H2's structural cure, replacing
    Stage 1's interim refusal.
  - M8 is redesign F; M9 rebuilds the flow view from F's result.
  - M10 is redesign C.
  - M11 makes gears stand-alone, after G, R and K exist.
  - petgraph is not used: it lacks matching, DM and a cycle basis, and would need a second copy of
    the graph.

**Stage 4 — The audit's remaining Phase 4 tasks** that no redesign replaced: rating, metrology, edits,
numerics, dead code, the CLI harness and the UI. Afterwards: re-run mutation testing (target ≥ 92 %
caught, test workspace included) and the 130-constant perturbation.

**Stage 5 — Documentation**, rewritten for any reader.
- Present tense, one home per fact.
- History stays in `corrections.md` and out of the working text.
- Comments say what the code does and why, without argument or narrative.
- `CLAUDE.md` is kept current at each stage.

**Stage 6 — Features** (the audit's Phase 6), each off or neutral by default.

## 4. How work is done and checked

- **Orchestrator:** this session. It holds the plan, splits work into packages of related tasks,
  and integrates commits onto `audit-ablation`, in a linear history with no merge commits.
- **Workers:** at most two at once, each in its own worktree under `~/.cache/gearcalc-work/wt/`.
  Builds go through the two-slot `gc` helper.
- **Watchdog:** checks every 0.3 s. It kills any process over 5 GB, and the largest process that is
  not critical when the machine has under 3 GB available. Every kill is logged and reported.
- **Worker commits:** each proof law is written first and seen failing at the base commit, then the
  change lands, one commit per task or tight group.
- **Adversarial checker:** one agent per package. It:
  - re-runs the proof at the base and at the head;
  - runs `check_all.sh`;
  - diffs the golden corpus and asks why every moved number moved;
  - challenges the change against the principles in §2.
  A package is integrated only after the checker passes it.
- **Re-audit:** after each stage, a short re-review asks whether any new finding changes this plan.
  This file is updated, and the change is logged in §5.

## 5. Change log

- 2026-09-26 — plan written after the four-way review.
- 2026-09-26 — owner's ruling on contact: the unified contact model (S-C) is the preferred path, for
  every mesh, straight teeth included. There is no split case: if the speed is acceptable for helical
  gears, it is acceptable for spur gears. Today's line-contact and point-contact models, and ISO's
  closed forms, become its validation cases. If it turns out right but too slow for practical use, it
  ships as the validation instrument for the existing models instead.
  - Adopted by default (owner may override): tip-edge contact counts, with a "tips relieved" option;
    friction enters the normal force; ratings are read at the field maximum, with ISO's points
    reported beside them.
  - Next step is a phase-exact prototype:
    - load along a line integrated exactly;
    - the shared approach solved exactly per piece;
    - means and maxima over phase from breakpoints;
    - the Hertz aspect ratio as the one bracketed inverse;
    - a closed-form limit at Σ → 0;
    - tooth stiffness in series, derived from the generated tooth and gated against ISO 6336-1's c′
      and c_γ.
- 2026-10-02 — orchestrator's call (bending section rule): the notch factor belongs to the notch.
  - A section in the fillet carries K_f; a section on the smooth flank carries K_f = 1.
  - The governing section is the one with the highest rated stress, not the highest Y_F.
  - An unreadable candidate never masks a readable one.
  - A continuity law runs across the flank/fillet switch.
  - Refined the same day: each curve contributes its constrained extremum of the Lewis measure — an
    interior tangency, or the curve's end where it has none. Otherwise a ring's fillet, which has no
    interior tangency, would drop out, and rings would lose their notch factor, rated about 60 % low.
- 2026-10-02 — owner's idea, researched at low priority: the tooth geometry is known exactly, so
  exact-geometry solvers may beat mesh methods for notch stresses. Candidates: IGA and IGA-BEM, and
  Trefftz / MFS / particular solutions with Williams corner functions in the basis.
  - The singular coefficient is the notch stress intensity factor.
  - Batched dense linear algebra suits accelerators.
  - Being evaluated as a round in `notch-research.md`, including whether a computed K_t should replace
    the Dolan–Broghamer fit.
- 2026-10-02 — owner: the contact model moves to Rust before its development is finished, because
  Python run time is the bottleneck, and development continues in Rust.
  - The Python prototype becomes the differential oracle: a generated JSON set of inputs and outputs
    that each Rust module must reproduce to a derived tolerance. The verifiers' independent scripts
    stay as a second check.
  - The port runs module by module (port → differential test → adversarial review). It is an
    expensive-mode library with a harness command, not yet wired into the live solve.
- 2026-10-02 — orchestrator's call, from rule 5: a cycle of carried axes is refused only when its loop
  fails to close geometrically (a numeric residual with a derived tolerance, T10.2). A structural
  rule would refuse buildable layouts: a bridge idler between planets, or a planet loop that closes
  exactly.
- 2026-10-01 — owner: research singular stress fields (sharp corners and notches) as a lower-priority
  parallel track. A geometrically sharp corner gives infinite stress as the mesh is refined, while in
  reality local yielding and relief dominate. The same mechanics governs notch stresses in bending.
  - Survey the state of the art, and develop new approaches where it helps, that remove the
    convergence failure.
  - This stays a parallel path unless it proves able to replace the existing models fully. Output:
    `work/notch-research.md`.
  - Owner's focus: the valuable methods need no special material factors. The ideal derives
    everything from common properties — E, ν, σ_y, σ_u, hardness, and at most a tabulated endurance
    limit. Measured factors such as ΔK_th, cyclic constants, notch sensitivities and slip layers are
    rarely available across materials. They can bring more error or false confidence than a method
    that ignores the issue.
  - Owner: the existing measured factors (ISO Y_S and notch sensitivity, K_f data, ΔK_th and
    critical-distance data, FKM, IIW) are the validation set for any general method. Each method is
    scored by how well it reproduces them, with size and sign; they are not its inputs.
- 2026-10-01 — Stage 1 closed (`stage1-exit.md`). Stage 1 met the audit's Phase 1 exit except for two
  items: the four flow-rule mutants (T16.28) and the survivor re-run. A 149-mutant sample of Stage 1's
  own lines caught 91.6 % of the non-equivalent mutants.
  - Order changes adopted:
    - graph steps M1–M3 move into Stage 2, ahead of V;
    - T16.13 becomes a Stage 2 gate;
    - T16.28's laws open redesign F;
    - F stays first in Stage 3;
    - search redesign C resolves bounds once per shift plan, and its exit turns the count snapshot
      into the bound calls ≤ C·dof·budget.
  - Seven worker rules were added (WORKER.md) and three Stage 2 exit conditions (`stage2.md`).
- 2026-10-01 — settled by the owner's rules after the fifth contact verification:
  - The matched (hobbed) worm wheel needs a new capped Newton solve, a seed search and a fallback
    minimiser. It is therefore implemented but not exposed; the involute wheel stays.
  - The expensive mode refuses an unset tip-edge radius r_e, because the r_e → 0 limit diverges
    (about r_e^−0.37) and has no finite value. The fast mode is unchanged. The owner may override.
- 2026-09-30 — owner: the bound that holds a tip off its mate's usable flank (sized by the mesh) is on
  by default for every gear, external and ring alike. One rule: every default design is free of
  interference by construction, and searches shorten external tips where they must.
- 2026-09-30 — owner: the contact model continues. Its role is a separate, on-demand analysis. This
  opens **two modes** of analysis:
  - a fast, cheap, flexible mode for real-time setup (today's closed forms at ISO's points);
  - an expensive mode for a final optimisation run, once the design is constrained and close to its
    intended result (the field model).
  Other models once rejected as too expensive may return in the second mode later. For now the focus
  is on building out and perfecting the contact model.
  - Next round: edge contact — one edge-curvature rule at every β, with the tip-edge radius r_e as
    the gear's input. The worm wheel (hobbed and matched, or involute) is still to be decided; both
    stay in the prototype.
- 2026-09-29 — owner accepted the recommendations after the second contact verification
  (`review/contact-verify2.md`):
  - Ratings are read at ISO's points by default, with the field maximum reported beside them, until a
    lengthwise-coupled field is verified.
  - The search keeps today's closed forms, held by a law to agree with the field model; the field
    model serves the final solve and validation.
  - Next round:
    - couple the slices lengthwise through an influence function (the physical cure for the helical
      jump);
    - remove the two remaining special cases;
    - correct the ISO comparisons: Z_ε at point B, and the ring's M1;
    - record the tooth-stiffness spread (about ±15 %) with size and sign;
    - test the matched worm set, whose meshing equation is linear in phase and may be exposable.
- 2026-09-26 — owner's rulings:
  - Redesigns first.
  - Docs and comments rewritten concisely, overriding `CLAUDE.md`'s prose-ratio note.
  - Rules of thumb exposed as options in the core and the UI.
  - `geartrain-refactor-*.md` deleted; `handoff_inbound/` kept.
  - One efficiency model taken; contact-geometry spike S-C added.
  - Principles 9–11 (graph, gears as entities, units) added.
