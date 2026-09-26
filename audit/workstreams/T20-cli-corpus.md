## T20 — The CLI harness and golden corpus

**Why.** `gear-cli` is the development harness, and `tools/check_golden.sh` records its output as the change detector the rating model relies on. Parts of it have become a second, untested engineering layer. It has its own interference instrument, its own optimiser, its own load-sharing sweep and its own friction literals, and the corpus records their mistakes as truth. `main.rs` has 0.9 % test coverage. The `hulaband` optimum that reference.md quotes moves from 58.849 % to 59.645 % when its x grid is refined from 0.05 to 0.01. Four golden headers print `mu 0.06` above figures computed at 0.08. `worm` and `wormstage` give two answers for the same worm (wheel torque 54.953 against 49.44 N·m). The corpus is also unbalanced. `verify 100` checks only z = 3. `sweep`'s eccentric axes change nothing (every count is divisible by 4). The matrix and the screw path print digits that are solver noise. The script that guards the corpus can delete it, or pass having compared nothing. The fix is one rule: the harness prints what the core computes, labelled from the core's own values, and the corpus records only converged digits over cases that can fail.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T20.1 `check_golden.sh` fails loudly and never empties the store | added2#43, added3#9 (unverified), gear-cli#10 | low | S | — |
| T20.2 Internal-mesh clearance moves to `gear_core::verify`, with a fouling fixture | gear-cli#3, added2#31 | medium | M | T20.1 |
| T20.3 `hulaband`: a continuous optimum in place of the 0.05 grid | gear-cli#4 | medium | M | T20.1 |
| T20.4 Friction read from the mesh, and one answer per worm | gear-cli#0, mesh-contact#7, added2#30, gear-cli#11 (worm) | low | S | T20.1 |
| T20.5 `loadcase` (c) from the core's `worst_over_cycle` candidates | gear-cli#2 | low | S | T20.1 |
| T20.6 A law ties the `strength` canary to `Train::alone` | gear-cli#5 | low | S | — |
| T20.7 Derived figures read from the core, not recomputed | gear-cli#11 (rest), gear-cli#12 (residual) | low | M | T20.1 |
| T20.8 The corpus prints only converged digits | gear-cli#12, ablate-constants-geometry#13, ablate-constants-rating#17 | low | S | T20.7; T15.16 |
| T20.9 Labels and prose say what is computed | strength#17, gear-cli#13 | low | S | T20.1 |
| T20.10 `sweep` and `verify` records reach the whole grid | added#26, gear-cli#6 | low | S | T20.1 |
| T20.11 Readable records; metrology in the corpus; coverage stated | gear-cli#7, gear-cli#15 | low | S | T20.1 |
| T20.12 Fold `trainfile` into `convert` | gear-cli#8 | low | S | T20.1 |
| T20.13 Arguments that do not parse are refused, with exit codes | gear-cli#9 | low | S | T20.1 |

### T20.1 `check_golden.sh` fails loudly and never empties the store
**Change.** In `tools/check_golden.sh`:
- Capture the case list with `list=$("$bin" --golden-cases)`, which `set -e` checks, refuse an empty list, and read the loop from `<<<"$list"`.
- After the speed filter, require `(( ${#cases[@]} ))`, and print the number compared, not the store's file count.
- Replace line 100 with `"$bin" $case >"$out" 2>&1 || { s=$?; echo "exit $s" >>"$out"; echo "gear-cli $case exited $s" >&2; status=1; }` and continue to the diff. The exit status is then recorded in the output, and a change to it is detected like any other change.
- In `--write`, build into a fresh directory, check that it is non-empty, and swap it in. Do not `rm -f "$store"/*.txt` before the copy has succeeded.

**Proof.** Use a stub binary in a fake root. When `--golden-cases` exits 1 or prints nothing, all three modes must exit non-zero and leave `tools/golden` at 32 files. Today `--write` empties it and `--fast` prints "32 recorded outputs, all unchanged". A stub case that panics must be named on stderr, and its panic text must appear in the diff. `tools-ci#16` is the same defect and closes with this task.

### T20.2 Internal-mesh clearance moves to `gear_core::verify`, with a fouling fixture
**Change.**
- Move `flatten` (main.rs:832, the only inverse of `outline.rs`'s bulge = tan(θ/4)) into `gear_core::outline::densify(outline, per_arc)`.
- Move `Boundary` and `roll_pair` (main.rs:879–1000) into `gear_core::verify::internal_mesh_clearance(ring, pinion, a, phases, steps) -> {best_phase, least, at}`. This follows the precedent that `verify.rs` lives in the library so the harness can sweep it.
- Name each tolerance and say why it has its value: 1e-5, 10·TOL, 3600 buckets, ±30-bucket search, the 1 mm band and `PER_ARC = 64`.
- Give `hulasweep` an optional fixed crank offset (or far-side gap) that bypasses the tip sizing. Today every requested gap at or below the tip-sized 0.2779 mm rolls the same pair.
- Record one case below that bound. Align the summary default `(18, 0.2)` with the recorded argument.
- Reword reference.md:1647–1651 so that the "quarter module" is the tip-sized 0.2779 mm and the 134° fouling is backed by the recorded case.

**Proof.** Write these tests before the move:
- `densify` puts its points on the arc to 1e-12.
- The standard 40/20 and 60/20 controls give least ≥ −10·TOL.
- A pair rolled at a − 0.05 mm gives least < −0.01.
- planocentric(28,30) at a fixed 1.0304, which fouls by 0.44 mm [ring#10, T05], reports fouling.

A recorded `hulasweep` case below 0.2779 mm must report negative clearance near 134°. On the current code every gap from 0 to 0.25 prints "touching at 20.2°", so this record fails today. Dropping the crossing test's dedup must turn the controls into fouling. The existing `meshsweep` and `hulasweep_18_0.25` text must stay unchanged.

### T20.3 `hulaband`: a continuous optimum in place of the 0.05 grid
**Change.** In `hula_band` (main.rs:1101), keep the discrete addendum {0.8…0.4} and cutter {10, 14, 20, 28} lists, and state them in reference.md as the designer's catalogue. For each (addendum, cutter), replace the x ∈ [−1.5, 2.0] step-0.05 grid:
1. Bracket the admissible x interval by solving each active constraint (ε = 1, tip margin = 0) with `solve.rs`'s bracketed root finder.
2. Maximise inside the interval with Brent or golden section. For d ≥ 5 the optimum is interior, with ε from 1.11 to 1.42.

Better still, pose it as the core's `auto::maximise` with the ring shift free, if its constraint set already holds these bounds. That would remove the second optimiser. Correct the doc comment at main.rs:1093–1095: its "every row sits at ε ≈ 1.00" is false for d ≥ 5. Regenerate reference.md:2018–2027, which quotes 58.9 / 94.0 / 94.6. At step 0.01 these already read 59.6 / 94.1 / 94.7. `teeth_clear` and `as_asked` have this filter as their only caller; their docs are T18.21 [ablate-features#8].

**Proof.** Refining the tolerance (or, for a grid fallback, the step 0.05 → 0.01 → 0.002) must not move any reported efficiency beyond its printed digits. Today d = 2 moves by 0.80 points. The winner must meet its active constraint to 1e-9, or have a zero bracketed derivative when it is interior. d = 2 must reach at least 59.645 %.

### T20.4 Friction read from the mesh, and one answer per worm
**Change.**
- `shifts_report` (main.rs:1568) and `crossed_report` (main.rs:3535–3537) print `meshes[0].sliding_friction` from the shape they solve. The crossed header also prints static friction, because its "locked" rows depend on it.
- `worm_report` drops `let mu = 0.06` (main.rs:3144) and reads μ from `arr::worm(starts, wheel_teeth)`'s mesh. Label its torque "delivered". Alternatively, delete its contact section in favour of `wormstage`.
- Grep the headers for other literal restatements of core defaults (`module 1`, `alpha 20`) and read each off the shape.

The default friction itself is T07.11 and T07.13 [lens-standards#0]: for steel on bronze at the canary's 0.44 m/s AGMA 6034 gives μ ≈ 0.057, so the shipped 0.08 understates forward efficiency by about 8 points.

**Proof.** A CLI unit test checks that the header's μ equals `arr::pair`'s mesh friction. It fails today on 0.06 ≠ 0.08. `worm 1 40 7 90` must give a wheel torque of 2 × 40 × 0.61805 = 49.44 N·m, down from 54.953. Its contact stress must be compared against `wormstage`'s 3376.1 MPa; the remaining gap is the clearance difference, a = 23.7073 against 23.7273. Golden diffs: the four headers and the worm lines.

### T20.5 `loadcase` (c) from the core's `worst_over_cycle` candidates
**Change.** Replace the 401-point sweep at main.rs:2952–2959 with `strength::bending_section_shared(&g1, eps, LoadSharing::LinearRamp)`, which seeds 0, ε, HPSTC and min(ε, 1). Rewrite the command summary at main.rs:436: it prints three load positions for six meshes, not "two load cases".

**Proof.** c ≥ b on every row, and c = b where the plateau governs, because the share is 1 on the plateau and at most `RAMP_MAX` = 2/3 off it. Today c < b on all six rows, e.g. 12:30 x+0.4, b 2.3537, c 2.3481. The rows also equal what the train's LinearRamp rating reports for the same mesh.

### T20.6 A law ties the `strength` canary to `Train::alone`
**Change.** Keep the canary's isolation from product defaults. Add a test showing that `strength_report`'s hand-assembled figures equal `Train::alone` on `arr::pair` 17/43 with face 10, shifts fixed at 0, `no_undercut` off and clearance 0: σ_F 66.791 / 55.999, σ_H 692.65, ε 1.62110. This means lifting the assembly into a function the test can call. Replace the inline `RimSupport::external(s, g.ra − g.rf)` at main.rs:2302 with `rim_support`. The state.md:243 sentence that cites the canary as evidence about load cases is T18.20 [added2#32].

**Proof.** The law passes on the current code, as the probe matched every printed digit. A perturbation that reaches only one of the two pipelines, for example the train's clearance default, must make it fail. The golden `strength` file stays unchanged.

### T20.7 Derived figures read from the core, not recomputed
**Change.** In `main.rs`:
- η₀ comes from the part's or path's efficiency, not the product of mesh efficiencies at 125, 187 and 1736.
- The `planetary` residual column (3320, |n0 − n1|) prints the `DistanceReport`'s own residual (running − nominal), the one `SetView` (124) prints. Today it is a closure residual only at zero clearance.
- Sliding speed comes from `MeshCase::sliding_velocity` with the case run at 3000 rpm, not v_s·ω·r by hand (1111.0 = 3 × 370.317).
- The worm's d1 is `z1·m_n / sin γ` with m_n read from params; drop the unreachable `.max(1)` (3083).
- The two default shaper-sizing rules (702: ring − 5 floored at 4; 1199: ring − 5 floored at 6) become one stated, printed CLI default. A hidden constant must not go into `gear-core`.

The hula ratio products (145) are deliberate display and stay.

**Proof.** A grep of `main.rs` finds no product of efficiencies and no ω·r arithmetic. Golden diffs are limited to `meshsweep`'s cutter where the floor differs, and `planetary` at non-zero clearance. A `planetary` run at clearance > 0 must print the same residual as `SetView`.

### T20.8 The corpus prints only converged digits
**Change.**
- One harness helper for fixed-point output normalises −0 (`v + 0.0`). Today 15 lines print `-0.00000`.
- Print residuals as `closes` when |r| < 1e-12·a, and the value otherwise. Today 3.6e-15 appears beside 0.0e0 in 6 lines.
- Print roles and presets by their serde snake_case name, not Debug-lowercase, which gives `meshedplanets` for `meshed_planets`. Renaming a single-word variant is already a boundary change, so it needs no more than that.
- In `matrix`, drop the 0.0 % sign table (main.rs:2729), or floor it at about 1e-9 relative, and round to a stated relative precision before ranking for Spearman ρ.
- Until T15.16 replaces the 2048-point trapezium with Gauss–Legendre, print path efficiency and torque to the digits the quadrature guarantees (6 significant). T15.16 then rewrites `the_path_average_has_converged` and its "far below any digit reported" sentence.

**Proof.**
- A build with `Tol::default().x_tol` = 1e-14 must give byte-identical `matrix` output. Today 5 of 18 cells and two ρ values move.
- `PATH_SAMPLES` ± 10 % must leave `graph`, `trainfile` and `convert` byte-identical. Today 4 + 1 + 1 lines move.
- Reordering the residual's terms must produce no diff.

### T20.9 Labels and prose say what is computed
**Change.**
- `strength`'s "F_n … along the line of action" (main.rs:2253) is F_bt. It is wrong once a helix is given: 237.1 against F_bn 250.4 at β 20°. Print both.
- `matrix` prints `— (0 of n)` for its `f64::NAN` sentinel (matrix.rs:342), which appears 6 times in the golden file.
- The DolanBroghamer model is named `Y_F·K_f (Savage J, with axial)`.
- state.md:845 and :929 say that the study's ISO set is tip-loaded (Method-C-like, not HPSTC).
- Move the doc comments stacked on `elevation_drive` (main.rs:1283) onto `train_report` and `train_file_report`. Rewrite the `shifts` comment (1642), which says the clearance is not read although the rows show it is.
- Replace `kind_name`'s spur/worm branch, which labels helical and crossed pairs "spur", with one word, `pair`, plus the printed β or shaft angle (rule 4).
- Delete the recurrence counts ("a third time", "ninth time") and "milestone 5/6". corrections.md holds the history.

**Proof.** The golden `train.txt` no longer reads `part 2 spur … beta 15 deg`, and `matrix.txt` has no `NaN`. `cargo doc` shows each comment on its own function. `check_figures.py` stays green.

### T20.10 `sweep` and `verify` records reach the whole grid
**Change.** `sweep` (main.rs:2491) builds `Gear::new` and counts per distinct tooth (`Gear::distinct`, `per_tooth_clamps`), and its line says that teeth are what it counts. Today `Tooth::new` reads neither `angular_shift` nor `index_offset`. Record the full `verify` (1080 cases, 38 s, already marked slow) in place of `verify 100`, which stops inside z = 3, and print the covered z set. If the grid moves into `gear_core::verify`, share it with `rack_simulation.rs` (T16.10 [added2#34]).

**Proof.** The `sweep` counts stop being all multiples of 4 (today 23792 / 1008 / 18572), and perturbing `Gear::new`'s shared-tool depth rule moves them. The `verify` record's worst deviation becomes 6.150734e-4 at z = 31, where it is 5.710682e-4 at z = 3 today.

### T20.11 Readable records; metrology in the corpus; coverage stated
**Change.**
- Rewrite `dump`'s doc comment (main.rs:2519). It cites `tools/dump_ref.py`, which never existed in the repository. The command remains the corpus's only full-grid profile detector.
- Record `dump`'s per-case `S` scalar rows (1188 short lines) in place of the 10.9 MB sha256, so that a diff names the gear.
- Add a `measure` command that prints span and over-pins for a spur, a helical, a shifted and a helical ring gear, which T09 expects.
- The full-table JGMA guard is T09.5.
- Add one sentence to CLAUDE.md's check table on which corpus covers what: golden against the wasm record, which holds `ranges` and one tolerance.

**Proof.** In a worktree, perturbing the pin-tangency tolerance must give a `check_golden` diff; today it gives none. Perturbing one gear's profile must give a `dump` diff that names that case.

### T20.12 Fold `trainfile` into `convert`
**Change.** Delete `train_file_report` (main.rs:1349) and its golden file. Its five figures are `convert`'s, from copied closures, and a lost field is already caught by gear-io's text-equality round trip (train.rs:646). If writing the drive stays useful, make it `convert --write PATH` with PATH required. Never write a fixed /tmp path.

**Proof.** The corpus loses `trainfile.txt` and no other file changes. `grep /tmp crates/gear-cli` finds nothing.

### T20.13 Arguments that do not parse are refused, with exit codes
**Change.**
- `arg()` (main.rs:351) returns `Result`, with the message "argument N: cannot parse {s:?} as {type}".
- `run` returns `Result`, and `main` exits 2 on a usage error (unknown command or mode, unknown material) and 1 on a failed solve.
- An unlisted mode string (`shifts epicyclicx`, `train bogus`) is refused, not replaced by the default.
- `hulasweep`'s mesh index is bounds-checked (main.rs:1247).
- A broken pipe ends quietly.

The core half, where `Gear::new` refuses z = 0 and non-finite shifts (today `show 0` panics at gear.rs:564), is T01.6.

**Proof.** A CLI unit test runs each bad invocation through the dispatch and expects `Err`: `strength 17 abc`, `shifts epicyclicx`, `train bogus`, `bogus`, an unknown material and `hulasweep 18 0.2 5`. All exit 0 or panic today. `gear-cli dump | head -1` exits without the "Broken pipe" panic.
