## T16 — Tests and gates that can fail

**Why.** Many checks here pass by construction. Some compare the code with a copy of itself: two of the "independent" kinematics scripts check against Python transcriptions of the crate's formula, and several unit tests call the same function on both sides. Some use a threshold where an exact law is available, or a bound fitted to an artefact: the rack gate's 1e-3 mm limit is set by a phase-sampling residual that halves with every halving of the step. Some assert inside a branch that never runs, or over a grid that never turns the axis where the fault lives. Mutation runs show the effect. Ignoring the torque sign of a load held still passes all 634 tests and the whole golden corpus [lens-tests-train#1]. Four of four mutations to the flow's power rules survive the suite [edit-ops#3]. check_figures accepts two table rows swapped [tools-ci#1]. The one wall-clock gate that is flaky failed 21 of 40 runs under load alone [lens-tests-train#6]. Each task below adds its proof first, sees that proof fail on the current code where there is a defect, and prefers a law or an independent oracle to a threshold.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T16.1 check_figures matches rows, not a bag of numbers | tools-ci#1, tools-ci#2 | medium | M | — |
| T16.2 The by-hand scripts read the crate and assert | tools-ci#0, kinematics-flow#4, kinematics-flow#5, crossed-worm#9 | medium | M | — |
| T16.3 One train test grid | lens-tests-train#11 | low | M | — |
| T16.4 Mirror law for load cases | lens-tests-train#1 | medium | S | T16.3 |
| T16.5 Path laws: backward figures, whole-flow breakaway, signed speeds | lens-tests-train#3, lens-tests-train#2, lens-tests-train#7 | medium | S | T16.3 |
| T16.6 Conditional assertions must fire | lens-tests-train#4 | low | S | — |
| T16.7 Relief law: a given input is honoured | train-mod-a#8 | medium | S | T16.3, T16.6 |
| T16.8 Work counts replace wall-clock asserts | lens-tests-train#6, lens-tests-geometry#12, added2#86 | medium | M | — |
| T16.9 Hold the shipped shift search to one admissibility oracle | ablate-features#1, added#47, added#62, added2#113, mesh-contact#6, added#63, added2#116 | medium | M | — |
| T16.10 One cutter grid, run in parallel slices | added2#100, added2#34 | low | S | — |
| T16.11 Rack gate: converged in the step, limit tied to its error, k axis | added#12, lens-tests-geometry#1 | medium | M | T16.10 |
| T16.12 Ring gate: exact distance, curtate rings in the grid | tooth-form#2, added2#112 | medium | M | T16.11, T05.2 |
| T16.13 Independent finite-z value gate for the bending model | strength#5, lens-tests-geometry#3 | medium | M | — |
| T16.14 Scripts find what cargo built; the corpus is never emptied | tools-ci#11, tools-ci#16 | low | S | — |
| T16.15 CI workflow: least privilege, per-ref concurrency | tools-ci#7, tools-ci#8 | low | S | — |
| T16.16 Site build without a second lock hash | tools-ci#4 | low | S | — |
| T16.17 check_wasm reads exports from the built module | tools-ci#3, wasm-boundary#8 | low | S | — |
| T16.18 check_doc_links resolves every pointer | tools-ci#5, lens-docs-accuracy-1#15 | low | S | — |
| T16.19 check_units sees every angular field | tools-ci#6 | low | S | — |
| T16.20 Placeholders checked per call and per firing | gear-io#15, added2#29 | low | S | — |
| T16.21 Panics are named; degenerate input cannot panic | tools-ci#14, lens-errors-policy#19 | low | M | — |
| T16.22 Exact-zero asserts only on structural zeros | ablate-constants-rating#1 | low | S | — |
| T16.23 Exact laws in place of tautologies in the geometry tests | lens-tests-geometry#4, lens-tests-geometry#5, lens-tests-geometry#7, lens-tests-geometry#10, lens-tests-geometry#11, added2#69, gear-outline#2, added#10 | low | M | — |
| T16.24 Metrology: helical span, JGMA monotonicity, a corpus row | lens-tests-geometry#8, metrology#17 | low | M | T16.14 |
| T16.25 Efficiency and thickness tests say what they check | lens-docs-accuracy-2#13, mesh-contact#8 | low | S | — |
| T16.26 Distance-mode tests state what mode 3 promises | added#46 | low | S | T10.5, T12.6 |
| T16.27 Strength tests that claim more than they assert | strength#13, added2#108, added3#23 (unverified), added3#5 (unverified) | low | M | — |
| T16.28 Flow order and idle rules pinned | lens-tests-train#9, edit-ops#3 | low | S | T16.3 |
| T16.29 Small train-test fixes | lens-tests-train#8, added2#107, edit-ops#11 | low | S | — |
| T16.30 Every gate is a flake check | tools-ci#12 | low | M | T16.2, T16.14, T16.17 |
| T16.31 Kill the surviving mutants | mutation.md | medium | L | T16.6, T16.13 |

### T16.1 check_figures matches rows, not a bag of numbers
**Change.** In `tools/check_figures.py` (:425–441) the outputs of all tagged commands are joined into one string, and a documented number passes if it equals any number in that string at the document's precision. Instead, each table row's strong figures (two or more decimals) must appear as an ordered subsequence of the number stream of *one* tagged command. Keep the bag rule for prose blocks only. Reorder the columns of `reference.md:640-641` to the `gear-cli shifts` order (Σx … ε, η), or allow an explicit per-block `unordered` flag with a stated reason. Delete the dead branch at :399–402. For the 8 `figures-by-test` markers, which today only check that `fn name(` occurs somewhere (a comment would do): require `#[test]` directly above the fn, extract its body, and require every number in the tagged block to appear among that body's literals at the document's precision. Longer term, have a harness command print each such table and gate it as `figures:`, so only one copy of the literal exists (`arrangements.rs:1399` and `reference.md:1924` hold two today).
**Proof.** Add a `--self-test` mode with a fixture `.md` and fake outputs, run in CI. Today these all pass the script, and each must fail: F2 (97.561 and 98.345 swapped between rows), F4 (`σ_F 66.8 / 56.0` → `56.0 / 66.8`, state.md:201) and F3 (reference.md's `37.9 %` → `39.9 %`). F1 and F6 must stay caught. A prototype of the row rule passes 15 of the 17 gated rows; the other two are the rows to reorder.

### T16.2 The by-hand scripts read the crate and assert
**Change.** Follow `breakaway.py`, which reads `tools/golden/graph.txt`.
- `train_kinematics.py`: parse `tools/golden/kinematics.txt` (it prints `total ratio -43/17` and every slot speed) and compare as exact rationals with the rigid-body derivation. Keep `crate_rows()` as a second assertion: it checks the row formula itself.
- `hula_kinematics.py`: `rolling()` adds a θ-independent increment 200 000 times, so it is the instantaneous relation times a constant. Delete it, and delete the claim at `train_kinematics.py:59-62` that it answers the wobble doubt. Compare `closed()` with `hula_18_0.2.txt`'s `ratio 324 / 1`.
- `breakaway.py`: its docstring says "Nothing below is the crate's assignment search", but `efficiency()` is flow.rs's 2^M search with the same filters and `max(found)`. Consistent branches with distinct efficiency occur in 5,476 of 18,000 Wolfrom cases, so the tie-break matters. Fix the docstring and add carrier-frame closed forms per 3K/compound preset (Pennestrì / Del Castillo) as the independent half. A sign(v_rel) simulation would not remove the ambiguity.
- `crossed_path.py`: it has no assert and always exits 0. Assert every residual and exit non-zero on failure. Measure the zone off the surfaces, taking one-sidedness from the helicoid parameter u ≥ 0, not from the crate's rule; today the script uses the two-sided band that was rejected. Add the 9/37, Σ 90°, β₁ 20°, addendum 2 fixture (3.161 two-sided against 1.829 one-sided) and an off-centre case, and compare with `crossed_17_23_90.txt`. Fix the docstring ("exactly one" of four lines; its own output lists eight, two of them the path). Fix `screw.rs:2195-2197` and `state.md:182`, which say the line of action is read off the surfaces.
- `iso_6336_3_stack.py`: call it an analysis in CLAUDE.md's table, not a check. Make `corrections.md:656` credit the corpus alone.

**Proof.** In a worktree, flip a ring's sign in `arrangements::wolfrom` and regenerate the corpus: the new `train_kinematics.py` fails, and today's passes. Reinstate the two-sided band: `crossed_path.py` fails. Make flow.rs take `min` among positive efficiencies: `breakaway.py`'s closed-form half fails.

### T16.3 One train test grid
**Change.** Add `train::testing` (`cfg(test)`) with one `trains()` grid: every `Preset` plus every `every_arrangement()` entry (hula, Ravigneaux, worm-and-pair), crossed with {alone, after a Spur, before a Layshaft}, each with the standard cases. Keep the Layshaft companion, because the groupings laws need an idle branch. Add one `alone(shape) -> Alone` helper to replace `solve_preset`, `solve_set` (identical bodies), `conventionally` ×3 and `solved` ×2. Move the sweeps in `offers.rs`, `groupings.rs`, `edits.rs`, `preview.rs` (pair-only today) and `mod.rs` onto it one file at a time, deleting each local generator as it goes.
**Proof.** At each step the test count is unchanged and every law's checked counter rises. Seven of these laws, extended with the extra arrangements, already pass. A new preset then reaches every law with no edits.

### T16.4 Mirror law for load cases
**Change.** Add `a_mirrored_case_rates_as_the_case` over the T16.3 grid with ultimate, back-driving and intermittent-fatigue cases. Negate every `Load`'s `torque.manual` and `speed.manual`. Assert: each case's solved flag is unchanged; each `CaseBody` torque and speed is negated; each member case's |torque|, contact stress and bending stress are equal to rel 1e-9; each path efficiency is equal. About 50 ms. The fixtures only ever back-drive with a positive torque, so also add a negative-torque `back_driving` fixture.
**Proof.** It passes on HEAD. Under mutation H (the still-load weight at mod.rs:4412–4418 forced to +1) it fails with "case 1 body 0: -6.944… vs 7.010…". Today H passes the whole suite, and the corpus is unchanged. Related: T11.2 fixes a still load's direction taken from a stale manual value [train-mod-b#0]; this law guards it.

### T16.5 Path laws: backward figures, whole-flow breakaway, signed speeds
**Change.**
- Add `a_paths_figures_are_the_case_through_it`. For each train in the grid, write cases a→b and b→a. Assert `efficiency.forward` = |T_to ω_to| / |T_from ω_from| from the `CaseBody` rows (skip where it reads 0), and `circulation.forward` = Σ `power_through`. Also assert that path(a,b)'s backward figures are path(b,a)'s forward ones. That half is structural in `paths_of` and catches wiring slips; the first half is the independent one.
- Replace the fixture of `a_path_holds_at_rest_where_no_mesh_does` (mod.rs:11344). Its hula fixtures already lock under sliding friction, so the static flow is never decisive. Use the Wolfrom preset with every mesh at μ = 0.05: at μ_s = 0.05 the backward efficiency is > 0.1 (0.236); at μ_s = 0.10 it is exactly 0 while each mesh's backward efficiency stays > 0 (0.994 / 0.993). Drop the history aside in its docstring.
- In `the_graph_gives_every_preset_the_kinematics_it_reports`, drop `.abs()` on member speed and on speed against the carrier (mod.rs:6474–6476). This is verified safe on HEAD.

**Proof.** Mutation T (backward circulation := forward) fails nothing today; mutation U (backward efficiency := forward) fails only two recorded hula pins and no worm test. The new law fails under both ("Worm after None: backward 0 vs the reverse path's 0.618…"). Mutation E (drop `.once_moving(&at_rest)`) fails the Wolfrom fixture: "left: 0.2359…, right: 0.0". Mutation V (unsigned member speeds) fails the signed comparison: "member 1 graph -0.395 vs stage 0.395".

### T16.6 Conditional assertions must fire
**Change.** Add a counter helper to `testing.rs`. A test with an assertion inside a branch counts how often the branch ran and asserts ≥ 1 at the end. Apply it to the four laws whose branches never run:
- (a) `every_input_relief_leaves_given_is_honoured_by_the_solve`: the Distance, Helix and FaceWidth arms (mod.rs:7952, 7957, 7958) never run. Collect the freedom variants that do run and compare with the full list; seeded in T16.7.
- (b) `an_automatic_face_width_satisfies_every_enabled_rating_of_both_gears`: enable all four `face_sources`. The contact source is off by default (mod.rs:2504–2507).
- (c) `the_declared_limit_is_the_freedom_the_shape_actually_has`: delete the `if could_spare` branch and its sentence, unless a real group with `order.len() > automatic_at_most + 1` exists.
- (d) `a_larger_root_round_holds_the_shift_down`: loop `k in 0..=10` (ρ up to 0.5; ρ ≥ 0.45 on 9/37 is uncuttable), with the counter.

**Proof.** Today a `panic!` placed in each branch leaves all four tests green. With the counters, they fail if the fixture change is reverted.

### T16.7 Relief law: a given input is honoured
**Change.** Keep the current law as the cheap structural half. Add the stronger law over the grid: pin everything, nudge one freedom f, relieve with `just = f`. Then either f's typed value is the solved figure, or a note names both the asked and the reached value. A note alone is not a failure: some, such as Planocentric's `clearance_negative` where the tips hold the distance open, are physics. The law reports the freedom-kind coverage from T16.6(a).
**Proof.** It fails on 18 of 122 cases at HEAD, while the suite is green. Examples: Layshaft typed helix 1.0 solves to 0.0; Planetary distance gives `distance_not_reached`; Planocentric typed clearance 0.03 solves to 0.02. Those defects are T10.9, T10.10 and T10.4's [train-mod-a#2, train-mod-a#10, added2#11, added2#55]. Land this law with an explicit expected-failure list, and shrink the list to empty as they land.

### T16.8 Work counts replace wall-clock asserts
**Change.** gear-core has three `Instant::now` gates.
- Give `auto::Search` an evaluation counter, and count Tooth/Gear builds per search. Evaluations alone miss the per-trial cost the hula comment is about. Assert both against a derived bound in `every_search_is_quick_enough_to_type_over` (mod.rs:10000).
- In `solving_an_amplitude_is_quick_enough_to_type_over` (gear.rs:1328), count Tooth builds per amplitude solve. Its recorded 102 ms regression was "building gears whose teeth nothing read"; a count catches that on any machine. It has a >10× margin, so this is low priority.
- In `an_unreachable_tolerance_stays_bounded` (outline.rs:716–727), replace the time assert with the vertex-count band, with n and the depth in its message.

Every timing canary that remains moves to `#[ignore]`, run in a serial CI step, so the default suite holds no `Instant::now` assertion. The count version sees [lens-performance#3] (with the search on, a Layshaft takes about 1.7 s native per solve, and the timing gate skips it). The flow's elimination count (T11.12's second bullet) lands with T11.4 in Phase 3.
**Proof.** Today 21 of 40 concurrent runs fail on the pair's 40 ms ceiling (40.8–65.6 ms), and mutants E and G, which do not touch the search, failed only this test. The count test must pass under llvm-cov and under 40× load, and fail under `Search::SHIPPED.starts * 2`. `MAX_SUBDIVISION_DEPTH` = 15 must fail on the count message, not "took 4.3 s".

### T16.9 Hold the shipped shift search to one admissibility oracle
**Change.** Three tests gate only the dead `shifts_for_efficiency`: auto.rs:2747, auto.rs:2833 and `the_search_beats_a_scan_of_the_same_interval` (mod.rs:5231). The shipped search has self-consistency and pinning laws, but none against an independent scan. Its deletion is T12.5 = T17.3 [auto-search#7, added2#87, added3#18 (unverified)].
- Re-aim the scan law at the shipped search. Build a pair through `solve_preset` with the distance pinned and `set_search(true)`, read the chosen shifts, and compare with a grid.
- The grid oracle `eta_at` refuses only undercut, severing, root round and ε < 1.2. Add the refusals the chooser applies (bottom clearance ≥ 0, tips clear, no flank interference), built from `Tooth`/`Mesh` directly so the oracle stays independent of the search. Add an assertion that oracle and chooser agree on admissibility at every grid point, so the two cannot drift apart. Then set `SLACK` from the measured wall error and state in its doc which sweep set it. Today, on 9/37, three inadmissible points beat the chooser by up to 7.45e-5 against `SLACK` = 1e-4. Among admissible points the excess is 0 on 9/37, 17/43 and 13/37.
- Apply the same filter, plus `root_radius_fits`, in contact.rs's `an_external_pair_loses_least_well_above_its_undercut_floor`. Its "least shift" foil (0.05, −1.25) interferes on both flanks. Its 9/37 best (1.45, 2.00) sits on the grid edge and is unbuildable. Its 17/43 best moves to (0.80, 3.25) if the grid widens. Restate the test as: the best buildable pair has Σx > 1 and beats the least-Σx buildable pair. Among buildable pairs the margin is 0.0024, so drop the 0.005 margin and the "shortening the path" claim, which fails on 9/37. Delete the "interior" sentence and the "x₁ at +0.50" sentence, which describes the chooser. Re-quote the doc figures.
- `Search::maximise`'s step floor `(spacing*first_step).max(resolution)` is gated by nothing. Add a 1-D test whose box width is derived from `Search::SHIPPED`'s fields, between 24 and 96 resolutions, e.g. (0, 0.08) with its peak at 0.0233.

**Proof.** A mutation that visibly loses efficiency (halving the Division box width, shape.rs:1829–1832) must fail the re-aimed law. The swapped-division mutant is near-equivalent (97.678 % either way), so it is not the proof. Removing `.max(resolution)`: 0.020000 against 0.023000, which fails (a 0.02 box passes either way). Widening the contact.rs grid to x₂ = 4.0 must leave its answers unchanged. With `SLACK` at its measured value, removing any one added refusal from `eta_at` fails on 9/37.

### T16.10 One cutter grid, run in parallel slices
**Change.** Add `gear_core::verify::cutter_grid() -> Vec<GearParams>`: z {3,5,8,9,11,13,17,23,31,47} × x {−0.5,−0.2,0,0.2,0.5,0.9} × three α × β {0,20} × ρ {0,0.25,0.38}. Use it from `tests/rack_simulation.rs` and `gear-cli verify` (main.rs:2573), which today repeat it by hand. Split `profile_is_bounded_from_both_sides_by_the_cutter` into six slices (helix × root radius) through one helper, plus one cheap test asserting `len ≥ 1000` and that the slices' union equals the grid. Keep the CLI's iteration order, or re-record the `verify 100` prefix ([gear-cli#6] owns what that prefix covers).
**Proof.** The combined worst values are unchanged (6.150734e-4 at z = 31). This test alone sets nextest's wall time (30 s single-threaded); afterwards the wall time is about the next-longest test. Re-measure CLAUDE.md's "~26 s".

### T16.11 Rack gate: converged in the step, limit tied to its error, k axis
**Change.**
- Make the phase step a parameter of `verify::check_cut`, with `MAX_ROTATION_STEP` as the default. Make `phase_resolution_has_converged` halve the step and assert that the change is below a stated fraction. Today it varies only `profile_points`, so the constant's doc ("no better at half of it") is false.
- For a sharp corner (ρ = 0), the distance is V-shaped in phase and the parabolic refinement cannot fit it. Measure against the corner's own trochoid, which `verify.rs` already builds, or use a V-fit.
- Set `DEVIATION_LIMIT` from the refinement's stated error, not from where the sweep stopped. Smooth-round cases are already at 1e-7 to 1e-9.
- Make `MAX_PHASES` a function of the step, or document it as a runaway guard (z = 3, x = 0.5 needs 5,062 phases and is capped at 4,000).
- Add thickness_mod k ∈ {0.7, 1.3} where the fillet cap binds (small z, large ρ).
- Restate the verify.rs and rack_simulation.rs module docs: the profile is the envelope of the rack the Tooth records, not a proof that the rack is the one GearParams asks for. Any `Rack::from_params` oracle must be written from ISO 53, not transcribed from `Rack::wanted_by`.

T03.1 fixes check_cut's cutter coming from the tooth's own placement [tooth-form#1]; this task narrows it.
**Proof.** Today, on z = 31, x = −0.5, 14.5°, ρ = 0, the deviation goes 6.151e-4 → 3.020e-4 → 1.541e-4 as the step halves, so the new convergence test fails on HEAD. After the change it must be step-independent to 10 %. A bd + 1e-3 perturbation must trip the gate by ≥ 100×. M15 (the fillet cap computed from the k = 1 thickness, tooth.rs:912) passes all 645 tests today and must fail.

### T16.12 Ring gate: exact distance, curtate rings in the grid
**Change.**
- `ring_cut_envelope` reports each radius bin's minimum at the bin centre, and `check_ring_cut` measures to the nearest sample of a 3,000-point reference. The reported 3.633e-3 mm is 0.484–0.498 of a bin width and does not move with the phase count. Use the exact pinion-cutter distance √(R² − r_b²) − r_b·φ_T (an involute's normal is tangent to its base circle), or intersect each transformed cutter segment with the query circle, and measure point to segment. The rack and the ring are then checked the same way.
- Every ring in the two gates is prolate. Add curtate cases, selected by corner radius < cutter operating radius: 30/20 at x ≥ 0.8, 24–25/20, hula-like 19/14 at x ≈ 0.6 and ha 0.7, with ha₀ varied. Assert at least one curtate case is present.
- Move `a_cutter_at_the_wrong_centre_distance_is_visible_to_the_gate` to an absolute threshold.

**Proof.** The extended grid fails at HEAD on 30/20 x 0.8 at 0.059 mm, where the gate's own bound is 5e-3. Land it with the fix of [added#30] (0.0032 mm in the probe). worst_distance must be stable to 10 % as radii and phases double. A 1e-3 mm phase error must trip it.

### T16.13 Independent finite-z value gate for the bending model
**Change.** No test runs `RootStressModel::DolanBroghamer`, the model every train rates with, against an outside value, and `tests/bending.rs` uses `Iso6336` throughout. Add to `tests/bending.rs`, sharing no code with `strength.rs`:
1. An independent generator: a pitch-point envelope trochoid, a brute-force inscribed parabola, and the closed-form ρ_f = ρ + b_c²/(r + b_c). Compare the whole DB factor (Y_F − axial)·K_f (AGMA 908 Eq 5.72–5.76) at all three offered pressure angles and on helical virtual gears. It agrees today to 6.0e-10 (Y_F) and 5.2e-9 (factor) over 255 designs.
2. The 30° tangent section at HPSTC against ISO 6336-3 Method B, as the instrument's gate (7.3e-10 over 804 designs).
3. Replace the z = 4000 rack-limit checks (5e-3; parabola 1e-2) with Richardson extrapolation of s, h and Y_F from z = 8000 and 16000 to 1e-5. The Richardson residual is 5–7e-7, so 1e-6 would leave under 2× margin. Write 30° as a literal in the test's `rack_limit`, citing ISO 6336-3, rather than importing `TANGENT_ANGLE_DEG`. Sweep x ∈ {−0.3, 0, 0.5}, h_f ∈ {1.0, 1.4}, h_a ∈ {0.8, 1.0}.

Rings stay on the corpus. [ablate-constants-rating#4] and [ablate-constants-rating#5] (K_f and the ISO constants, held by one corpus output each) are closed by this.
**Proof.** Each must fail in a worktree: flip the fillet corner-centre sign; perturb H by 0.15 % (CLAUDE.md records that H and L perturbations leave the suite silent); use ρ_F in place of ρ_f; perturb the tangent angle by 0.1°.

### T16.14 Scripts find what cargo built; the corpus is never emptied
**Change.**
- `check_golden.sh:65`, `check_figures.py:115-122`, `build_wasm.sh:48` and `check_wasm.sh:78` build with cargo, then read `$root/target/…`, which ignores `CARGO_TARGET_DIR`. Replace the four with one helper that takes the directory from `cargo metadata --format-version 1 --no-deps` (jq is not on the host). Have `check_wasm.sh` honour `BUILD_WASM_MODULE` as `build_wasm.sh` does. Accept `GEAR_CLI=<path>`.
- In `check_golden.sh`, capture `list="$("$bin" --golden-cases)"` so `set -e` sees a failure, and refuse an empty list. Under `--write`, build the scratch set completely and then swap it in; today the script runs `rm -f` first.

This also closes [gear-cli#10], [added2#43] and [added3#9] (unverified).
**Proof.** With a stub gear-cli whose `--golden-cases` exits 1, `--fast` today prints "32 recorded outputs, all unchanged" and exits 0, and `--write` deletes the corpus. Afterwards both fail and the corpus survives. With `CARGO_TARGET_DIR=/elsewhere` and an empty `./target`, all four scripts use the fresh artifact.

### T16.15 CI workflow: least privilege, per-ref concurrency
**Change.** In `.github/workflows/ci.yml`:
- Set the tests job to `permissions: contents: read`. Its `id-token: write` is unused and contradicts the deploy job's own comment.
- Move `concurrency: {group: pages, cancel-in-progress: false}` onto `deploy`. Give the workflow `group: ci-${{ github.ref }}`, with cancel-in-progress on pull requests. Keep `push` unfiltered, since the owner works on long-lived branches.
- As the first step of [tools-ci#12], move `check_doc_links`, `check_strings` and `check_units` (3–5 s each) to the top of the job, and drop the redundant `cargo build --release --bin gear-cli` (ci.yml:85).

**Proof.** CI stays green, and deploy still serialises on main. A dead doc pointer is reported before the 5-minute build.

### T16.16 Site build without a second lock hash
**Change.** In `flake.nix`'s `webFor`, replace `npmDepsHash` with `npmDeps = pkgs.importNpmLock { npmRoot = ./web; }` and `npmConfigHook = pkgs.importNpmLock.npmConfigHook`. The pinned nixpkgs has it, and every lock entry has integrity and resolved fields. Rewrite `docs/state.md:39-45` without the trap, keeping "nix flake check is not all of what CI runs". Drop only the fixed-output-hash clause at `CLAUDE.md:167`.
**Proof.** `nix build .#web` is byte-identical (`diff -r` of the two outputs is empty). Bumping a devDependency without touching flake.nix still builds.

### T16.17 check_wasm reads exports from the built module
**Change.** Replace the `grep -A2 '^#\[wasm_bindgen\]$' | sed 's/^pub fn …'` at `check_wasm.sh:108` with `WebAssembly.Module.exports` of the optimised module, minus `memory` and `__wbindgen_*`. From the d.ts, also drop `initSync` and the default init. Reword "the module people download" to "a module built by the shipped recipe": the bytes differ only in embedded panic paths, so do not assert a sha256 match. A flake check on `self.packages.wasm` is optional.
**Proof.** Append `#[wasm_bindgen(js_name = fooBar)]`, a `pub fn` below two doc lines, and a `pub async fn`. Today the grep lists 23 and finds none of the three; afterwards the gate fails on each.

### T16.18 check_doc_links resolves every pointer
**Change.**
- Fix `corrections.md:608` to `../README.md`, and fix the two dead examples in the docstring (`#the-lewis-parabola`, `#one-hob-one-setting`).
- Match the whole anchor with `#([^)\s`'"]+)`; today `[a-z0-9-]+` truncates at `_` or an uppercase letter.
- Check that the file exists for every relative link, with or without an anchor, resolved against the linking file, including `../`, `history/` and non-`.md` targets.
- Scan `git ls-files` minus `web/src/wire`. Today tools/, flake.nix and .github are not scanned, and web/src is globbed non-recursively; share the source list with `check_strings.py`.
- Skip fenced lines when collecting headings, and give duplicate headings GitHub's `-1`, `-2` suffixes.
- Delete the promised "heading with no referrer" report rather than build it, since many headings are legitimately unreferenced.
- Stop quoting pointer counts in comments: the script prints 83, ci.yml says 389, the docstring says 175.

**Proof.** Mutations b (missing `.md`), c/c2 (truncated anchor), d (history anchor), e/e2 (tools/, flake.nix) exit 0 today and must fail. h2 (a working `-1` link) must pass. The pre-fix tree fails on `corrections.md:608`.

### T16.19 check_units sees every angular field
**Change.** Widen `FIELD` to any visibility and `looks_angular` to `theta|psi|phi|delta`. Leave out `lead`, which is a length throughout the crate. Accept `f32` and containers. Count a `_rad`/`_deg` suffix as a stated unit. Classify a doc comment by its first unit word. Give units to `Tooth::psi_p`, `psi_b`, `theta_a` and `theta0` (tooth.rs:151, 153, 167, 192) and to mesh.rs `alpha_t`/`alpha_n`. There is no fn-parameter pass, since parameters carry no doc. `Deg`/`Rad` newtypes are the durable fix, but that is a large API change against one recorded case.
**Proof.** Checker mutations u2–u7 (`theta_x`, `pub(crate)`, private, `helix`, `f32`, `Vec<f64>`) exit 0 today and must fail. The tree is green once the docs are fixed.

### T16.20 Placeholders checked per call and per firing
**Change.** Generate, with the bindings, a TypeScript type that maps each `ui.*` key to its placeholder names from `strings_en.toml`, so `npm run check` rejects a `t("ui.k", {…})` whose keys differ. A regex scan gave 19 false positives, so it would be fragile. In `strings.rs`, keep a set of per-firing value sets per key (`observed_values` at :587–592 unions them today) and require each firing to cover the message's placeholders. Five keys fire from two sites today.
**Proof.** Renaming a placeholder in `strings_en.toml` fails `npm run check`. Dropping a value from one site of `CLAMP_FILLET_CAPPED` fails the per-firing check and passes today's.

### T16.21 Panics are named; degenerate input cannot panic
**Change.**
- Set `expect_used`, `unreachable` and `panic` to warn on gear-core, gear-io and gear-wasm, with `#[expect(…, reason)]` at the 5 sites (arrangements.rs:217, materials.rs:80, strings.rs:200 and 225). Remove shape.rs:1852's `unreachable!` by iterating the pairs structurally.
- In gear-cli, add one crate-level allow with a reason covering its 16 `.expect()` calls.
- Delete the 58 `cast_precision_loss` and 3 `too_many_lines` allows, which are no-ops under `pedantic = allow`.
- Set `unsafe_code = "forbid"`; it builds for wasm32.
- Leave `float_cmp` (7 deliberate sentinels) and `indexing_slicing` off. The latter would flag 806 sites, 660 of them in train/*.
- Add a property test that feeds degenerate `GearParams` (teeth 0, module ≤ 0, NaN, 90°) through every public entry point and every wasm entry, and asserts no panic. The refusal itself (rule 5) is T01.6.

**Proof.** A bare `.expect()` added to gear-core fails clippy. The property test fails today: `Gear::new` at teeth = 0 or module = 0 followed by `.profile(64)` panics at gear.rs:564.

### T16.22 Exact-zero asserts only on structural zeros
**Change.** Two assertions compare a solved float with 0 bit for bit. Compare `mod.rs:7517` against `1e-12 × |end torque|`, and `crossed.rs:1499` against a fraction of the search's resolution. Review only the other flow-solved zeros (mod.rs:10873, 10933, 11005, 11153, 11159). Keep the structural ones exact (a locked efficiency, zero-load Hertz, held speeds, β = 0 identities); there are 32 in all.
**Proof.** The unrelated perturbations `pair_clear`, `fric_slide` and `def_root_radius` fail these today (residues 2.95e-17 and 1.77e-15) and must not afterwards. Making the pair carry load must still fail.

### T16.23 Exact laws in place of tautologies in the geometry tests
**Change.**
- `extremes.rs:181` asserts `a <= -d + 1e-9 || a > -d`, which is always true. Replace it with addendum.min = max(−h_f, (r_b(1 + TIP_ABOVE_BASE_FRACTION) − r)/m − x) to 1e-12, and add a dedendum-bound case. At lo − ε, assert ra = r_b(1 + f) for base-bound cases (tooth.rs:471 raises the tip without a note, which T02.6 adds).
- `material.rs:447`: assert 1/E*_ab = ½(1/E*_aa + 1/E*_bb) to 1e-15, plus one hand-computed steel-on-polymer value. Correct the comment, which is false: the test's own pair gives E*_ab 3416 > E*_bb 1738.
- `auto.rs:2157` never calls the crate. Assert that `minimum_profile_shift` changes sign between z = 17 and 18 and matches z_min = 2(h − ρ(1 − sin α))/(m sin² α). Otherwise delete it: its numbers live only in `docs/history/`.
- Helical fillet: add a value pin g.rho = c·m/cos β for an uncapped case (z 40, β 25°, c 0.25). This is the documented approximation in state.md, not an independent truth. If added to regression.rs, mark it as not reference output.
- Contact ratio: add a helical pair and assert ε = (approach + recess)/(π m_n cos α_t / cos β), with the base pitch written in the test. In `degenerate_input_is_clamped_and_reported`, assert `CLAMP_PRESSURE_ANGLE_RAISED` by key; no test asserts that key.
- Fillet cap (`geometry_laws.rs:115`): it asserts only θ₀ ≤ half pitch, so it cannot see a cap that is too small. Add a unit test in tooth.rs asserting that the fillet of radius `rho_fit` is tangent to the tool flank and the centreline at once (ac/R = π/z).
- `outline.rs:451`: measure `g.outline(..)`, not `Gear::new(g.params)`. Fold it into the ring deviation law of [gear-outline#1].
- `ring.rs:1108` compares `conjugate_radius` with itself. Delete it, and re-point mesh.rs:568's "[verified]" at `the_conjugate_relation_holds_at_the_pitch_point`.

**Proof.** Each must fail on its mutation, and each passes today except as noted:
- M13 (floor −0.5·h_f) and the mirror mutation −1.5·h_f.
- M4 (1/(2·max c)).
- Dropping the ρ term in `undercut_shift` fails ~30 other tests today, but not this one.
- M1 (ρ = c·m).
- M3 (normal base pitch); a spur pair cannot see it.
- `w_tip/(2 cos α)` (9 other tests fail today, not the guard).
- Ring tolerance × 4 (caught only by the corpus today).

### T16.24 Metrology: helical span, JGMA monotonicity, a corpus row
**Change.**
- Delete the helical skip in `tests/metrology.rs:66-68` and correct its comment: the formula is ISO 21771's helical form.
- Keep the 66-value preferred-number whitelist; a rounded-R40 set would catch fewer digit errors (58 % against 66 %). Add module and diameter monotonicity with the one recorded exception (fine grade 1, d 12–25: m 0.6–1 gives 28, m 1–1.6 gives 26).
- Add a `gear-cli metrology` row to `COMMANDS` that prints k, W, pins, pin range, M2/M3 and JGMA for a spur gear, a helical gear, a shifted gear, a helical ring and an eccentric gear, and record it. [gear-cli#15] is the same gap.

**Proof.**
- With the skip removed, the test passes at 5.5e-16, and `inv α_t → inv α_n` in `span_over_teeth` fails it.
- Monotonicity raises single-digit mutant detection from 80 % to 92.5 %.
- Changing the helical span contact radius in both span functions passes every test today; it must move the corpus.

### T16.25 Efficiency and thickness tests say what they check
**Change.**
- In `efficiency_matches_a_numerical_average…` (contact.rs:1711), the numeric side divides by the same `cos_bb` as the closed form. Compute the sliding velocity (ω_rel·ξ_t) and the normal force T/(r_b cos β_b) from the axes instead.
- Make `reference.md:86, 607` quote the asserted tolerances (1e-9 absolute, 1e-14), or tighten the asserts to what they state.
- `thickness_modification_equals_an_equivalent_profile_shift` compares x + x_s with x + x_s. Compare it with the literal m(π/2·k + 2x tan α_n)/cos β instead.
- Reword `contact.rs:349-357` and `reference.md:608-611`: the tests check the integral's arithmetic under uniform 1/ε_α weighting, and the pair loads sum to F only on the cycle average.

The load-sharing model change and its cycle-level law belong to [added2#72].
**Proof.** Scaling `thickness_shift()` by 1.1 fails the new thickness test and passes today's. The helical factor 1/cos β_b, derived independently in the verification, is then measured rather than assumed.

### T16.26 Distance-mode tests state what mode 3 promises
**Change.** `a_manual_centre_distance_ignores_the_clearance` passes only because its own distance is not reached: `part.distance_not_reached` fires inside the fixture. Rewrite it with a reachable fixture: 17/43 at 30.6 with clearance 0.5 gives nominal = distance − clearance (30.1), backlash 1.0499 and no notes. `a_distance_the_shifts_cannot_reach_is_said_out_loud` calls 9/37 at 23.00 mm unreachable, but a legal pair reaches it (x₂ −0.49 > x_min(37) ≈ −1.16). Choose its distance below the sum of the true floors under whichever policy [added#45] / [auto-search#8] settles, and assert its notes.
**Proof.** The rewritten tests pass with the chooser floored at x_min, and fail if the reaching member's floor is raised to max(x_min, 0).

### T16.27 Strength tests that claim more than they assert
**Change.**
- `only_the_relative_curvature_reaches_the_pressure` (strength.rs:3229) calls no production code. Test `Mesh::relative_curvature(ξ)` against 1/ρ₁ ± 1/ρ₂ from `curvature_radii`, or delete it.
- `a_rating_is_taken_at_a_point_on_the_tooth`: assert that the section is `None` once ε pushes the load point off the flank.
- `the_virtual_contact_ratio_relation_barely_moves_the_answer` hard-codes spreads 0.0004/0.0012/0.0020. Measure the spread in the test over several mates (17/17 gives 0.00053/0.00185/0.00328) and perturb in the measured, negative direction.
- `sharing_is_off_by_default_and_can_only_ever_relieve_the_tooth` is false for z 9–12 below ε_n = 2 (up to ×1.019). Assert the one-sided law in the Dolan–Broghamer measure: shared ≥ unshared, with equality where the HPSTC governs (0 of 366 cases went lower). [added#27] owns the docs.
- Rate a ring with a rim thickness and assert Y_B = `RimSupport::internal(s, m_n).factor()`.
- `the_sharing_sweep_has_converged` ([added2#108]; [added3#23] (unverified) is the same gap): delete its "exact equality" sentence and add z 9/10 at addendum 1.0. Seeding the single-pair zone's interior maximum with a bracketed maximiser (solve.rs) makes the sub-band answer exact.
- The sharing-note test (unverified [added3#5]): replace its magnitude band from four designs with a grid sweep compared with the message's figures (a grid gives +33.3 % and −73.6 % against the message's 24 %/15 %), or take the numbers out of the message. [added2#109] owns the documented range.

**Proof.**
- Removing `LoadPoint`'s bracket check passes all 679 tests today; it must fail.
- `RimSupport::internal(t, 3·module)` passes today; it must fail.
- The z = 9 sharing case fails today's floor (0.921 against 0.97) because the test measures the wrong model; the new law passes.
- z10 ε 1.7 moves 8.2e-4 between 200 and 800 samples, so the extended convergence test fails at HEAD. It passes once the maximiser lands.

### T16.28 Flow order and idle rules pinned
**Change.** Extend `a_flow_says_every_body_and_every_mesh_once`:
- After each Body row, the carrying rows are in non-increasing `power_through` (couplings, whose `None` sorts as ∞, first), and Idle rows come after all carrying rows.
- On a layshaft, the engaged ratio's meshes are Mesh rows and the other ratio's are Idle.
- In a planetary set, the held ring is said inside the junction, not walked on, and the junction carries the max.

Name the idle threshold (`1e-9` absolute today) and make it relative to the case's input power. Ordering needs a body with two carrying branches, which is the power-split train of the edit-ops flow finding.
**Proof.** M6/K (ascending sort), M7 (every mesh idle), M11 (terminals always walked on) and M16 (junction min) each fail. Today all four pass nextest, and M6 and M16 pass `check_wasm` too.

### T16.29 Small train-test fixes
**Change.**
- `thickness_modification_cannot_break_its_own_invariant` (mod.rs:9313): seed `Auto::automatic(1.0)`. Its current seed 0.7 = 2 − 1.3 is already the answer.
- `self_locking_is_said_out_loud`: require `MESH_SELF_LOCKING` only. This is for clarity; the swap is caught elsewhere.
- In `edits.rs`, name the least tooth count 4 (repeated at :417, :429, :491) and the ring margin zp + 2 once, each with its reason (T15.15 owns the values [lens-magic-numbers#13]). Pin each with a one-line assertion, and pin `round()` in `fitted_teeth` with a helical-against-spur mate.

**Proof.** Making `thickness_mods` read `.manual` for automatic members fails the reseeded test; today only the shape.rs law catches it. M9 (floor instead of round), M14 (ring at 3×) and M18 (threshold 2) pass all 604 tests today, and each must fail.

### T16.30 Every gate is a flake check
**Change.** Add flake checks so that `nix flake check` is CI:
- `checks.golden` and `checks.figures`, with `GEAR_CLI=${self.packages.default}/bin/gear-cli` (needs T16.14, and `tools/`, `docs/` in their source).
- One check per Python gate, plus `validate_dxf`.
- `checks.web = self.packages.web`.
- A crane test derivation for `check_bindings`, and a `buildNpmPackage` check for `npm run check`.

Whether the by-hand scripts run in CI is a separate policy decision; once T16.2 makes them fail on a crate defect, they cost about 20 s. Then update CLAUDE.md's "`nix flake check` is **not** all of them" and README ([lens-docs-accuracy-1#6], [added2#56]).
**Proof.** `nix flake check` fails on each mutation named in T16.1, T16.14, T16.17 and T16.18. CI saves the release gear-cli build and the check_bindings compile.

### T16.31 Kill the surviving mutants
**Change.** Add the 18 laws in [`../mutation.md`](../mutation.md). The first is a module-similarity law: every length scales with m and every angle is unchanged. It runs every fixture at m ≠ 1 as well as at 1, and on its own it kills nine survivors across six files. The others add a both-sides sweep of every threshold and note, one fixture per refusal that breaks only its rule, and a fired-counter on every assertion inside a branch. Delete the 12 equivalent survivors that sit in dead code with their removal tasks (T12.5, T17.2, T17.5, T17.7, T05.12). Allowlist the rest, each with its reason.
**Proof.** Re-run cargo-mutants with `--test-workspace` on the same shards. Every mutant classed as a gap is caught.

### Declined
- lens-tests-train#12: the grid it would trim is deliberate (the fourfold-budget `assert_eq` on every set). It is not what sets the wall time; T16.10 is.
