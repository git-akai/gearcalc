# gearcalc — audit and improvement plan

The core mathematics is right. Involute and trochoid generation, ring geometry, mesh geometry, Hertz contact, the bending constructions, the exact kinematics, the power flow and span/ball metrology each agree with an independent computation that shares no code with the crate, mostly to 1e-8 or better. What goes wrong sits around that core:
- models run past the edge of their domain with no cut and no note;
- a figure that could not be computed comes out as a plausible number;
- rules that hold only over the whole train are decided mesh by mesh;
- nothing checks input that describes no shape;
- some allowables and practice rules stand in for the model, and their bias has no recorded size;
- the flow and the search cost grow exponentially with the train;
- too many gates cannot fail, because they share the assumption they check.

The plan makes the gates able to fail first, then fixes the 27 high-severity wrong answers. After that it gives every input and refusal one owner, makes cost linear, and replaces split or copied models with one signed model per phenomenon. Documentation and features come last. The 21 workstream files in [`workstreams/`](workstreams/) (T01–T21) hold each task's change and its proof; [`findings.md`](findings.md) lists every finding and [`ledger.json`](ledger.json) holds its evidence.

## What was done

- **38 auditors**: 21 subsystem slices (primitives, tooth form, ring, outline, mesh contact, crossed/worm, strength, metrology, shape, train, flow, search, edits, graph, gear-io, wasm boundary, web, CLI, tools/CI and others), 14 cross-cutting lenses (unification, continuity, magic numbers, errors policy, numerical robustness, performance, architecture, standards, feature gaps, docs accuracy ×2, docs clarity, tests ×2) and 3 ablations (dead features, geometry constants, rating constants).
- **Adversarial verification** of every finding by a second agent that tried to refute it. Findings raised during verification were verified in turn, except 25 from the last round, which stay unverified.
- **Execution reproduction** of the 36 high-severity findings: 35 reproduced, 1 in part.
- **Mutation testing**: 1,025 gear-core mutants sampled. 144 of the 974 viable ones survived the suite (14.8 %). The most survivors were in `auto.rs` (26 of 86), `ring.rs` (17/83), `metrology.rs` (14/53), `verify.rs` (14/72), `solve.rs` (13/30) and `train/mod.rs` (13/67).
- **Perturbation of 130 constants** (61 geometry, 69 rating). 38 were caught by the suite, 39 only by the golden corpus, and 53 by neither.
- **Coverage**: 88.1 % of workspace lines. Every gear-core file is at 93 % or more. `gear-cli/src/main.rs` is at 0.9 %.

**Ledger: 809 findings.** 552 confirmed, 228 partly confirmed (the corrected claim is used), 3 refuted (dropped), 1 unverifiable and 25 unverified (marked where cited). The 806 kept, by final severity: **27 high, 213 medium, 498 low, 68 info**. Verification cut the auditors' 2 critical and 84 high down to the 27.

## The patterns

Eleven causes account for most findings. Each one is a class of defect, and each has a structural cure that removes the class instead of only its instances.

**P1. Absence written as a value.** When a figure cannot be computed, a number takes its place. A refused flow reports efficiency 0 (which reads as self-locking) and backlash 0 [kinematics-flow#2, train-mod-b#5]. Over-given speeds report "4294967295 short" [train-mod-b#7]. A band end with no operating angle becomes 0 play [mesh-contact#3]. An empty crossed face zone becomes the unbounded tip path [crossed-worm#0]. A locking threshold uses −1 [crossed-worm#12]. Ranges use ±∞ [lens-numerical-robustness#6], a severed flank uses NaN [lens-numerical-robustness#9], and a face width of 0 solves to nulls [lens-errors-policy#13]. An i128 overflow is reported as "underdetermined" [lens-performance#2]. *Cure:* absence is typed (`Option` or a named refusal), and 0 keeps its physical meaning. Two laws hold it: no result carries a non-finite number, and a 0 appears only where its physical cause holds (T02.2, T02.3, T02.9, T07.2, T07.12, T11.6).

**P2. The refusal side of rule 5 has no owner.** Nothing refuses input that describes no shape. Out-of-range indices panic. A `carried_by` cycle grows wasm memory to 2 GiB [graph-ops#0, graph-ops#1, wasm-boundary#0]. All 29 float fields take NaN [gear-io#4]. Module 0 panics or is refused under four different keys [primitives#0, lens-errors-policy#0]. A Poisson ratio of 0.7 is rated silently [lens-errors-policy#5]. Negative friction gives η > 1 [lens-numerical-robustness#5]. Refusal keys name a symptom rather than the cause [lens-errors-policy#3]. Errors carry a second English text that has drifted from the catalogue [lens-unification#5], and refusals cross the boundary in three shapes [wasm-boundary#5]. A trap poisons the instance [wasm-boundary#1]. *Cure:* one validator per kind of input (structure, floats, gear parameters, train scalars, materials), called wherever input enters, refusing under a key that names the field, and held by one fuzz law over every preset's document. Every refusal a designer can reach crosses as a Note (T01, T02).

**P3. One phenomenon, several copies, each with a one-sided assumption.** The ring repeats the external machinery with the sign flipped, and each copy assumes one side. Its tip check folds with `abs` and calls enclosing tip circles clear [ring#0, ring#1]. Its fillet assumes a prolate path [added#30]. The eccentric play drops σ [lens-unification#0]. A mesh's kind is inferred from member order [lens-unification#1]. The crossed pair loses the sign of β₁ through a cosine [added#56]. Point-contact play is floored at 0 where line contact goes negative [added#59]. `carrier_driven_efficiency` uses |R| [kinematics-flow#3]. The rolled corner is written seven times [tooth-form#3]. A second internal-mesh model is kept for tip clearance alone [lens-unification#3]. The stage-era search survives beside the shipped one, and the train law checks the dead copy [ablate-features#0, added2#87]. *Cure:* rule 4 made structural. One model carries the sign or curvature as a parameter: σ, signed helices, and signed tool curvature κ with the rack at κ = 0. Laws run over both kinds and both signs, and a dead-pub gate stops a second copy from surviving (T03.13, T03.15, T04.6, T05.1, T05.2, T05.17, T07.7, T17.2).

**P4. Models used past the edge of their domain.** The parallel path of contact is never cut at the usable flank [lens-continuity#0, added2#45]. A rack whose tooth comes to a point still cuts to full depth [tooth-form#0, lens-numerical-robustness#2]. The fillet can consume the flank so the junction lies above the tip [lens-numerical-robustness#3]. Dolan–Broghamer goes negative or NaN for a low load point [lens-numerical-robustness#0]. Above ε = 2 the sharing ramp's shares sum to more than the load, up to 1.73 at ε = 3.5 [ablate-constants-geometry#3, mesh-contact#5]. The out-of-band note quotes the wrong range [gear-io#0]. *Cure:* each model states its domain as a predicate. At the edge the code clamps with a note or refuses (rule 5), and a continuity law runs across the edge (T03.3, T03.4, T06.1, T06.5, T08.2).

**P5. Rules decided locally that hold only over a whole.** The path backlash band is built mesh by mesh when it must be built per distance [train-mod-a#0, mesh-contact#0]. The carried-axis triangle is never closed [shape-a#0]. Relief is counted group by group and depends on group and mesh order [train-mod-a#2]. A size reading reaches only the first mesh on a distance [train-mod-a#3]. The thickness freedom is per mesh [added2#55]. Plan roles follow list order [shape-a#1]. The assembly rule is pooled per axis [shape-b#3]. Reversal and cycle counts come from a mesh count [shape-b#4, shape-b#5]. The minimum width is inverted at the widest mesh [train-mod-a#1]. Case entries are patched by a different local rule at each edit site [edit-ops#5, graph-ops#4, graph-ops#5]. *Cure:* state each rule over the scope it governs (distance, carrier, mesh group, case, train). Add laws that permuting what should not matter changes nothing, and one train-level invariant checked by a seeded walk over cased trains (T10, T13.1).

**P6. Gates that share the assumption they check.** The rack gate builds its cutter from the tooth's own derived depth [tooth-form#1]. Two "independent" scripts transcribe the crate's own formula [tools-ci#0]. `check_figures` matches a bag of numbers [tools-ci#1]. The efficiency test shares the load-split assumption [mesh-contact#8]. The outline tolerance test treats every arc as concentric [gear-outline#1], and `validate_dxf` cannot see a wrong bulge [gear-io#2]. Conditional assertions never fire [lens-tests-train#4]. A wall-clock gate fails 21 of 40 runs under load [lens-tests-train#6]. `check_golden.sh` can empty the corpus [added2#43]. The mutations "ignore a held load's torque sign" and "unsigned intermittent cycles" pass everything [lens-tests-train#1, lens-tests-train#0]. *Cure:* every gate is an independent oracle or a law and is seen failing before it is trusted. Costs are gated by counts, not time. Mutation and perturbation runs become the measure of a gate (T16, T03.1, T04.2, T04.5, T17.1).

**P7. Engineering outside the core, where no law sees it.** The ring's fewest teeth is computed in gear-wasm and drops the shift, so it shows 34 where the answer is 17 or 44 [wasm-boundary#2, lens-architecture#0]. Over-pins around an eccentric gear is also computed in gear-wasm [wasm-boundary#3]. The CLI has its own interference instrument, optimiser, sharing sweep and friction labels, and the corpus records their errors [gear-cli#3, gear-cli#4, gear-cli#2, gear-cli#0]. TypeScript holds the mate tooth count and the locking verdicts [lens-magic-numbers#2, web#8], and a second copy of each box's text [web#0]. *Cure:* the core computes every figure; the harness and the panel only read it. A Rule-1 literal scan and generated request types enforce this (T14.1–T14.3, T19.2, T19.9, T20.2–T20.7).

**P8. One fact in several places, and only one copy moves.** Load-case figures are written twice [lens-magic-numbers#3]. The 0.02 mm clearance is written nine times, six of them no-ops [lens-magic-numbers#6], and the 7 mm worm four times [lens-magic-numbers#5]. The guarded pressure angle is written five times [ablate-constants-geometry#1], the pointed roll is solved by hand twice beside a third route through `inv_inverse` [primitives#8], and there are five union-finds [lens-architecture#11]. Seven request types are hand-copied into TypeScript [wasm-boundary#6]. Three lists of what CI runs contradict each other [lens-docs-accuracy-1#6], "stage" appears 978 times [lens-docs-clarity#4], and Y_S survives in the docs [lens-docs-accuracy-1#1]. *Cure:* one home per fact, enforced by a check: generated bindings, a restatement check, a dead-vocabulary check, one `check_all.sh`, and one core accessor per quantity (T15.1, T14.1, T18.1–T18.3).

**P9. Practice shipped as the model, with no bias on record.** The K factors are called conservative and all of them are ≥ 1 [strength#3]. The documented ISO comparison is wrong in size and partly in sign [strength#2]. A reversed-bending (R = −1) endurance serves as the one-directional allowable and as the flank allowable [strength#0, strength#1, lens-feature-gaps#1]. The worm uses a constant μ and a "cannot be back-driven" verdict [lens-standards#0, lens-standards#1]. Planet sharing is K_γ = 1 [lens-standards#3]. An absolute 0.02 ± 0.02 mm band gives zero minimum backlash at every module [lens-standards#6]. The near-locking and low-efficiency thresholds and μ 0.08 have no source [lens-magic-numbers#0, lens-magic-numbers#1]. The best-k rule and the 1.75 mm pin are hidden [metrology#2, metrology#14]. The efficiency split overstates spur loss by 16.5 % [mesh-contact#1]. *Cure:* every departure gets a size and a sign in `state.md`, generated by a check (the Known-approximate lint). Every rule of thumb becomes a named option the user can see and change (T08.1, T08.3, T08.4, T21.1, T15.2, T15.9, T07.13).

**P10. Numerical procedure where a closed form or a bracket exists; tolerances set by feel.** Ill-conditioned crossed solves make the path flicker, and η reads 1.000000 near parallel [crossed-worm#3]. The optimiser treats grid resolution as feasibility [auto-search#0] and takes an argmax over ridges it cannot rank [auto-search#3]. The one-dimensional division is found by search [lens-continuity#2]. The sagitta stop overshoots its tolerance by 4.65× [lens-tests-geometry#2]. `Tol`'s absolute floor dominates [primitives#4]. Moving the tip lean 10× changes no test and no golden output [ablate-constants-geometry#12]. *Cure:* closed forms where they exist, closed brackets otherwise, and one tolerance per question with its basis stated. Every surviving constant gets a law that fails when it moves (T07.6, T12.7, T12.11, T12.14, T04.3, T15.4–T15.7, T15.17).

**P11. Unbounded work and silent wrap.** The flow enumerates 2^M directions over the whole train and wraps at M = 32 [kinematics-flow#1, lens-performance#0]. Exact kinematics overflows i128 on nine chained 17/43 pairs, whose ratio is only about 4e3, and reports the train as underdetermined [kinematics-flow#0]; ratio addition that cancels the gcd first moves the refusal from 9 to 13 pairs [primitives#3]. The search grid is 13^dof, 46k calls on a Layshaft [lens-performance#3]. An unreachable chord tolerance gives 13.1 M vertices [added#19]. Everything runs on the main thread [lens-performance#4]. *Cure:* cost linear in the size of the problem, a named cap that refuses above it, and gates written as operation counts (T11.1, T11.4, T11.5, T12.10, T04.7, T11.13).

## Roadmap

Where two workstreams hold the same change, it lands once, under the first key named:
- T01.5 = T03.6 = T15.10's pressure-angle bullet (T15.10's shift floor stays in Phase 4)
- T03.7 = T12.15 = T15.13
- T16.14 = T20.1
- T05.6 = T14.2
- T09.3 = T14.3
- T12.5 = T17.3
- T12.1 = T10.8
- T16.8 = T11.12's search bullet = T12.10 step 1 (T11.12's flow count lands with T11.4)
- T03.2 = T09.9, reused by T12.14
- T13.6 = T14.13 = T15.1's load-case bullet
- T01.12 and T06.3 inside T02.8
- T05.3 inside T10.10
- T15.9 inside T07.13, after T15.2
- T14.7 inside T12.2
- T01.2 and T13.1 share one `check()`, which absorbs T14.6

Documentation tasks tied to a code task land in the same change as that code (for example T18.5 with T10.15, T18.6 with T11.3, T18.21 with T12.5).

### Phase 1 — Gates that can fail, and the wrong answers a user acts on
**Goal.** Every gate can fail on the fault it names, and the 27 high-severity findings are fixed. The wrong answer of [lens-performance#0], the wrap at M ≥ 32, is fixed by T11.1's cap; its cost half lands with T11.4 in Phase 3.

**Track A: gates.** These run in parallel except where arrows show order: T16.14, T18.2, T16.1, T16.2, T16.8, T16.13, T16.15, T16.16, T16.17, T16.18, T16.19, T16.21, T17.1, T19.1, T03.1, T13.1; T16.3 → T16.4, T16.5; T16.3 and T16.6 → T16.7; T04.1 → T04.2 and T04.1, T04.4 → T04.5, where T04.1 and T04.4 are Track B's.

**Track B: high-severity fixes.** The lines below run in parallel with each other.
- Tooth: T03.2 → T03.3; T03.1 → T03.4.
- Ring: T05.1, T05.2.
- Mesh: T06.1.
- Crossed: T07.1, T07.2.
- Strength: T08.1, T08.2, T08.3; then T08.4 → T08.6.
- Metrology: T09.1, T09.2.
- Train: T10.1, T10.2, T10.3, T11.1, T11.2, T12.1.
- Edits: T13.2 and T13.3, landed together with T13.1's walk.
- Crash: T01.1.
- Export: T04.1, T04.4.
- Web: T19.2, after T19.1.

**Exit.**
- Each of the 27 high findings has a proof law that failed at a2f2234 and now passes.
- `tools/check_all.sh` runs everything CI runs.
- `check_golden` refuses an empty case list.
- The `check_figures` fixture with swapped rows fails.
- rustdoc runs with `-D warnings` and 0 broken links.
- No `Instant::now` assertion is left in the default suite; timing canaries run `#[ignore]`d in a serial CI step.
- The seeded edit walk (200 walks × depth 4) is green.
- The mutants H, G, E and T, and the four flow-rule mutants, are caught.
- The 144 surviving gear-core mutants are re-run and the count is recorded.
- The DXF arc law puts every root-arc centre on the axis to 1e-9 mm.

**Unblocks.** Every later phase can trust its gates. T11.4 (after T11.1–T11.3). The rest of T08 and T13.

### Phase 2 — Boundaries and the error model
**Goal.** One validator per kind of input, one refusal channel, and every guard either notes or refuses.

**Tasks.** The input track runs in parallel with the error-model track.
- Input: T01.2, T01.3, T01.4, T01.5, T01.6, T01.7, T01.8, T01.9, T01.10, T01.11.
- Error model: T02.1, T02.2, T02.3, T02.4, T02.5, T02.6, T02.7, T02.8, T02.9. T02.6 and T02.7 come after T01.2, T01.6 and T01.8.
- Wire and format: T14.1, T14.4, T14.12, T14.14, T05.6, T09.3.
- Cases: T13.4, T13.5, T13.6, T13.7, T13.8, T13.9, T13.12.
- Domain edges: T03.5, T05.4, T05.5, T05.8, T06.4, T07.3.
- UI: T19.3, T19.4, T19.7, T19.8, T19.9, T19.10, T19.11, T19.12.
- Harness: T20.4, T20.6, T20.13.

**Exit.**
- The malformed-document fuzz law passes with 0 panics, and every refusal names its field. It sets every index leaf to len, len+1 and 99, sets every float to NaN and ±inf, and adds cycles and gaps.
- The degenerate-input no-panic property test passes.
- 5,000 traps followed by a good call return the recorded answer.
- A grep gate finds no English in any gear-core `Display`.
- Every designer-reachable refusal comes back as a Note whose key is in the catalogue.
- No preset's `TrainResult` carries a non-finite number.
- The Rule-1 literal scan is green.
- Every request type is generated and refuses unknown fields.
- The file format carries a version.

**Unblocks.** The unification work in Phase 4 can refuse instead of guard. T19.5 and T19.6.

### Phase 3 — Performance
**Goal.** Cost linear in the problem, gated by operation counts; the solve off the main thread.

**Tasks.** Four parallel tracks:
- Flow and kinematics: T11.3 → T11.4 → T11.10, T11.15; T11.5 → T11.6 → T11.7 → T11.8.
- Search: T12.9 → T12.10.
- Outline: T04.3 → T04.7.
- Payload: T11.11, T11.16, T11.17, T14.17.

Then T11.14 and, last, T11.13.

**Exit.**
- The seeded flow equals the 2^M oracle on every preset, every arrangement and every chained pair, in both directions: efficiency to 1e-12, torques to 1e-9, and the same branch.
- A 16-mesh planetary chain solves in at most M+1 eliminations per component.
- Chains of 32 and 40 meshes are correct under `--release`.
- Nine or more chained 17/43 pairs solve exactly.
- Search objective calls stay at or below C·dof·budget for every preset with search on.
- Outline vertex count is bounded by the closed-form curvature bound.
- One copy of the catalogues ships in the wasm, and `check_wasm` records the payload bytes.
- Typing into a 6-preset train stays under 50 ms of input latency.

**Unblocks.** The search redesign (T12.7 onwards) and the UI features.

### Phase 4 — One model per phenomenon
**Goal.** Replace split, copied and local rules with one signed model each. Every surviving constant gets a law, and dead surface goes.

**Tasks.** The tracks run in parallel; inside a track, follow the order given.
- **Tooth and tool:** T03.7 → T03.8; T03.9, T03.10, T03.12, T12.14; T03.13 → T03.14; T03.15 comes after T05.17's first steps.
- **Outline and eccentric:** T04.6; T04.8 decides whether T04.9, T04.10 → T04.11, T04.12 and T04.13 are done or deleted.
- **Internal gears:** T05.7, T05.11, T05.14, T05.15; T05.9 → T05.10; T10.10 (with T05.3) → T05.12 → T05.13; then T05.17.
- **Parallel mesh:** T06.5 → T06.6 → T06.7; T08.7 before T06.6; T06.8 → T06.9; T06.2, T06.10, T07.19.
- **Crossed and worm:** T07.6 → T07.7 → T07.18; T07.4, T07.5, T07.8 → T07.15; T07.9, T07.12 → T07.13 (with T15.9, after T15.2); T07.14, T07.16, T07.17.
- **Rating:** T08.5, T08.9, T08.11 → T08.12; T08.7 → T08.8 → T08.10.
- **Metrology:** T17.6 → T09.4 → T09.6 → T09.7, T09.8; T09.5, T09.10.
- **Train graph:** T10.4, T12.6 → T10.5, T10.6, T10.7 → T10.9; T10.11, T10.12 → T10.17; T10.13 → T10.14; T10.15, T10.16, T10.18 → T10.19; T11.9.
- **Optimiser:** T12.5 → T12.7 → T12.8, T12.11 → T12.12; T12.2 → T12.3; T12.4, T12.6, T12.13, T12.16, T12.18.
- **Edits:** T13.10 → T13.11; T13.13, T13.14.
- **Structure:** T14.5 → T14.10; T14.8 → T14.9; T14.11, T14.15 → T14.16, T14.18, T14.20.
- **Numerics:** T15.1 → T15.2, T15.3, T15.4 → T15.5, T15.6 → T15.7; T15.8, T15.10, T15.11, T15.12, T15.14, T15.15, T15.16, T15.17, T15.18.
- **Dead code:** T17.2, then T17.4, T17.5, T17.7, T17.8, T17.9, T17.10, T17.11.
- **Tests:** T16.9, T16.10 → T16.11 → T16.12 (with T05.2), T16.20, T16.22, T16.23, T16.24, T16.25, T16.26, T16.27, T16.28, T16.29, then T16.30.
- **Harness:** T20.2, T20.3, T20.5, T20.7 → T20.8, T20.10, T20.11, T20.12.
- **UI:** T19.13, T19.14, T19.15.

**Exit.**
- At least 92 % of the gear-core mutation sample is caught (85.2 % today). Every survivor, and every survivor of a sample of `train/*.rs`, is either killed or allowlisted with a reason.
- All 130 perturbed constants are either caught by a law or deleted; none is left that is caught by neither.
- The dead-pub allowlist holds only the reference oracles.
- Permutation laws over meshes, members, distances and relief groups are green.
- The relief law holds on 122/122 pinned-box cases and is order-invariant on 322/322.
- The shaper → rack law at κ → 0 is exact.
- One rolling-corner construction is left.
- `RingMesh` is reduced to the tip room.

**Unblocks.** The features in Phase 6 that need one generator, one share rule or a signed axis.

### Phase 5 — Documentation
**Goal.** One home per fact, in the present tense, with every figure generated or gated.

**Tasks.**
- Structural first: T18.1, T18.3 → T18.4, T18.24; T18.13 → T18.14; T18.15, T18.16, T18.17.
- Then the instances, each after its code: T18.5, T18.6, T18.7 → T18.8, T18.9, T18.10, T18.11, T18.12, T18.18, T18.19, T18.20, T18.21, T18.22, T18.23, T18.25, T18.26, T18.27.
- From the other workstreams: T03.11, T05.16, T07.10, T07.11, T09.11, T10.20, T12.17, T14.19, T20.9, T21.1.

**Exit.**
- `check_restatement.py` finds no 12-word run shared between two living documents.
- The dead-vocabulary check is green over docs, catalogues and production code.
- The Known-approximate lint finds every departure with a size and a sign.
- `check_doc_links` resolves every pointer.

**Unblocks.** Nothing; it closes the audit.

### Phase 6 — Features
**Goal.** Features the unified model reaches cheaply, each an explicit option.

**Tasks.** Parallel except where arrows show order.
- Loss, loads and planets: T21.2, T21.4 → T21.5, T21.6, T21.8.
- Materials and tolerances: T21.7, T21.3, T09.12.
- Contact: T21.9 → T21.12, T21.11, T21.13 → T21.14, T21.15.
- Export: T21.10, T04.14.
- UI: T19.5 → T19.6, T19.16, T19.17.
- Larger scopes: T21.16, T07.20, T21.17.

**Exit.** Each feature has its law or published-example gate (for example c′ ≈ 14 and c_γ ≈ 20 N/(mm·µm) for T21.13), and is off or neutral by default.

## Decisions for the owner

1. **Restated prose against "the ratio is not a target."** Recommended: replace that sentence with *one home per fact*, enforced by `check_restatement.py` (T18.1, T18.14, T18.17). The derivations stay; restatements become links. Trade-off: the prose ratio falls, and a reader follows a link instead of reading the argument in place.
2. **Efficiency model.** Recommended: weight the loss by load per unit contact-line length (T06.8). It is closed form, equals Ohlendorf at ε_β = 0 and today's value at integer ε_β, and has no spur/helical branch. Trade-off: the corpus moves (canary 98.741 → 98.919 %, Wolfrom ≈ 45.4 → 49.4 %). The alternative keeps today's model and records its +16.5 % spur loss bias.
3. **Tolerance standard.** Recommended: keep the JGMA tables as the default, since all 294 cells check, once they are stored as printed (T09.1). Add ISO 1328-2:2020 as a user-selected second standard (T09.12) once its formula has been read from the standard itself. Trade-off: two standards to maintain, and their grades do not map onto each other.
4. **Stable IDs or renumber maps.** Recommended: `Train::edit` returns what it made and how it renumbered (T13.11). No renumbering has been shown wrong, and stable IDs would change the file format. Trade-off: undo (T19.5) must replay renumber maps.
5. **The eccentric gear (T04.8).** It costs about 550 lines of code and 460 of comment, it is reachable only in developer mode, and no train builds it. Recommended: cut it unless it is on the product path. Cutting turns six tasks into one deletion; keeping it costs about two weeks (T04.9–T04.13).
6. **Allowables.** Recommended:
   - each fatigue value states its load ratio and specimen kind, and 0.7 applies only to R = 0 data (T08.4);
   - add a contact allowable estimated from hardness, with a quality grade the user can see (T08.6);
   - keep contact sizing off by default, but raise a note whenever σ_H exceeds the allowable.

   Trade-off: defaulting contact sizing on would triple the canary's minimum width.
7. **Shift floor.** Recommended: floor every member at x_min, and make non-negativity an explicit per-member option (T12.6). Trade-off: some searched designs move, and eight tests that encode the current refusals must be rewritten.
8. **Defaults that are rules of thumb.** Recommended:
   - clearance and tolerance in modules, with ISO/TR 10064-2 backlash as a named option (T15.3), which changes today's zero default minimum backlash;
   - the near-locking margin as an input (T15.9);
   - K_γ as an explicit input (T21.2);
   - the worm's locking reported as figures, not a verdict (T07.13).

   Trade-off: more inputs on the panel.
9. **Relief seeding (T15.8).** Recommended: seed the exact figure and display at one significant-figure count that the core exports. This changes the recorded intent "the digits they saw"; the alternative rounds to significant figures in one helper.
10. **By-hand scripts in CI.** Recommended: yes, once T16.2 makes them read the crate. They cost about 20 s.

## What not to touch

These parts were checked against independent computations and hold. Do not re-audit them; change them only through the tasks above.
- **Root finders**: Brent and bracketed Newton match Numerical Recipes step for step, and a switch to Chandrupatla or TOMS748 gains nothing. **Carlson** R_F/R_D are accurate to ≤ 3.3 ε. The inverse-involute seed's 0.4 is exact. **Ratio** normalises correctly. The **plane.rs** identities are exact.
- **Tooth generation**: the involute, trochoid, junction and severing agree with an independent envelope simulation to 1e-8–4e-8 mm, and the tooth is continuous in every input except physical severing. The fillet fit cap w_tip·cos α/(2(1−sin α)) is correct. The first three "looks wrong and is not" cautions hold.
- **Ring geometry** (tip, root, base half-angle, cutting distance, generation limit, junction) matches DIN 3960/KHK to 6e-14 over 819 rings. The drawn profile matches the simulated cut to 2.5 µm wherever no clamp fires.
- **Mesh geometry**: the operating angle, zero-backlash distance, path endpoints, contact ratio, backlash law and the one interference relation match mpmath to 1e-11 over about 12,000 external and internal pairs. ε_α matches ISO 21771 (1.62110).
- **Hertz** matches the elliptical solution to 1e-16 and ISO 6336-2's σ_H0 (629.92 MPa). The line limit is continuous, and the combined modulus equals Z_E.
- **Bending**: the default Lewis-parabola section and Y_F are right. The transcribed K_f, Y_S and Y_B constants are right. The ISO instrument equals Method B's closed form to 1e-10 over 71 designs. Rating at the single-pair boundary is ISO's Z_B/Z_D.
- **Kinematics and flow**: exact rational linear algebra, one matrix read four ways, and an exact play model. The per-mesh flow reproduces Pennestrì and the Wolfrom in both ring orientations. Power balance holds in all 15,764 fuzzed cases.
- **Screw at the pitch point**: the lead-angle and helix relations and the 90° efficiency and locking thresholds match BS 721. The ZI flank curvature is right.
- **Metrology**: W_k matches ISO 21771/DIN 3960 to 6.5e-16, and M over balls to 7e-15. The JGMA fine table matches an independent reproduction in all 294 cells.
- **Graph**: `Shape::parts` partitions exactly and deterministically. Shape edits are atomic, and offers stay complete over 5,366 random edits. JSON and TOML round-trip bit-exact, and `graph_of` converts the recorded old file exactly.
- **Search**: it is deterministic. `minimum_profile_shift`'s closed form is right. `divide_shift_sum` is continuous. The epicyclic search converges.
- **Boundary and web**: malformed JSON or TOML never traps. Output wire types are generated. The five catalogues hold identical 524-key sets. There are no async races, and state holds only inputs.
- **DXF container**: meets R2000, and ezdxf's auditor reports 0 errors.
- **Robustness**: no hang or runaway memory in about 120,000 fuzzed cases outside the carried-axis cycle. Every length is homogeneous in module to 2e-10.
- **Layering**: the foundation modules have no upward dependencies, and the power flow is written once.
- **Gates that already fail on their target**: check_golden, check_bindings, check_strings, check_units and check_wasm's differential law.
