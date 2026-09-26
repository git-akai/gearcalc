## T06 — Parallel-axis mesh: path of contact, load sharing, efficiency

**Why.** The line-contact figures are built on the wrong path and the wrong load split. The path of contact runs from tip circle to tip circle and is never cut where a flank stops being involute. So an unshifted 9/37 reports ε 1.519 on a path that starts 0.97 mm inside the pinion's base circle, where the usable ε is 1.00. Past x₁ ≈ −0.34 the same pair's shared bending factor drops 34 % in one 0.002 shift step, and the train then refuses it with two false messages [lens-continuity#0]. There are three load-share assumptions and none is stated as a law. Efficiency charges every engaged pair F/ε_α at every instant, which gives 16.5 % more spur loss than the rigid static split (17/43) [mesh-contact#1]. Above ε = 2 the bending ramp's shares add up to more than the load, up to 1.73 at ε = 3.5 [ablate-constants-geometry#3, mesh-contact#5]. The user-facing note quotes "−24 % to +15 %" for that band, where a sweep reaches −33 % and +74 % [gear-io#0]. At the tolerance ends, the verdicts are taken at the running distance only. A point contact's play is floored at zero where a line contact goes negative [added#59] [mesh-contact#12]. The fix is one path (cut at the usable flank) and one share rule (w/Σw, summing to 1 at every instant) that both efficiency and bending read. Each verdict is taken at both band ends.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T06.1 Cut the path at the usable flank; flank-interference note | lens-continuity#0, added2#45, mesh-contact#2 | high | M | — |
| T06.2 Interference judged at both band ends, in report and search | mesh-contact#12 | low | M | T06.1 |
| T06.3 Signed play at the tight end, one convention for both contacts | added#59, added2#47 | medium | S | — |
| T06.4 "Loses contact" on ε_γ; a separate ISO-scope note on ε_α | added#2 | low | S | — |
| T06.5 The load-share ramp conserves load at every ε; the band edge said once | ablate-constants-geometry#3, mesh-contact#5 | medium | S | — |
| T06.6 Sharing sweep as a bracketed maximiser; the one-sided law below the band | strength#12, added#33 | low | M | T06.1, T06.5 |
| T06.7 Out-of-band note carries this mesh's own change; fixed figures removed | gear-io#0, ablate-constants-rating#3, added2#109 | medium | M | T06.5, T06.6 |
| T06.8 Efficiency weighted by load per unit contact-line length (one model, any ε_β) | mesh-contact#1, added2#72 | medium | M | T06.1, T06.5 |
| T06.9 State the first-order-in-μ residual; correct "exact" and "symmetric" | lens-continuity#8, lens-unification#7, added3#11 (unverified) | low | S | T06.8 |
| T06.10 A line contact reports the length its pressure used | added#28 | low | S | — |

### T06.1 Cut the path at the usable flank; flank-interference note

**Change.** `contact.rs` `ContactPath::new` takes each member's `FlankEnds` (the shape already holds them, shape.rs:2395).
- The effective start is `max(−approach, ξ₁)`, where ξ₁ = √(r_j1² − r_b1²) − r_b1 tan α_w is where the mate's contact reaches member 1's junction.
- The effective end is `min(recess, ξ₂)` for member 2, signed by `mesh::conjugate_radius` so that a ring goes through the same expression.
- Keep the tip-limited ends as separate fields; the interference readout uses them.

Base ε_α, the efficiency integral, `load_fraction` and the HPSTC/LPSTC points on the effective ends. This is ISO 21771's d_Nf-limited ε_α.

Raise a core `Note` (new key, 5 catalogues) whenever `flank_interference` is set, carrying the member and the radial depth (junction radius minus contact radius at the mate's tip). Today only the panel shows it (TrainPanel.svelte:1202), so the harness and the corpus carry the misrated figures silently. Where the mate's tip reaches a fillet the cutter did not undercut, the teeth collide. There the note is the primary verdict, and ε is documented as "the contact left on the usable flanks".

**Proof** (all fail today):
- Law: for every fixed-shift pair over z₁ 5..20 and x₁ ∈ [−0.6, 1.0], the path start satisfies ρ₁ ≥ √(r_j1² − r_b1²) and the end satisfies ρ₂ ≥ √(r_j2² − r_b2²). Today ρ₁ = −0.967 mm on unshifted 9/37.
- Independent check: a rack-generation simulation that shares no code with the crate gives usable ε 1.009 at x = 0 and 0.746 at x = −0.3. The capped ε agrees to 0.01 (crate-capped 1.0025 and 0.743).
- Continuity law on 9/37 in steps of 0.002 in x₁: ε, σ_F (shared and unshared), σ_H and η have no jump across the onset. Today the factor goes 7.437 → 4.940 at x₁ −0.336 → −0.338.
- The train never returns `NoContact` or `NoRootSection` for a parallel pair on that grid. Today it returns NoContact at −0.343 and NoRootSection at −0.4.
- Where `flank_interference` is `[false, false]` the effective ends equal the tip ends bitwise. Assert that every preset is in that state rather than assume it, so the corpus stays unmoved.

**Notes.** Fixed-shift interfering pairs fall to ε < 1 (9/37 to 1.00), so the ε < 1 notes (T06.4) start firing on them. The search already refuses interfering candidates. This removes the false `NoContact` refusal of T02.7 [added3#8 (unverified)]. The HPSTC and the Hertz point move, and T08's ratings move with them.

### T06.2 Interference judged at both band ends, in report and search

**Change.** Evaluate `flank_interference` and the tip and bottom-clearance margins at `at_band(k, −1.0)` and `at_band(k, 1.0)`, and fail if either fails. No end is picked by sign reasoning: each margin is monotone in a, so the two ends are enough.
- Apply this in the report (shape.rs:3839-3841), in the search filter `MeshTrial` (shape.rs:2251-2257, auto.rs:1238-1257), in the point-contact verdict (shape.rs:3869) and in `CrossedTrial` (auto.rs:1193-1206).
- The T06.1 note reports the depth at the worse end.

**Proof.** Law: every search optimum has margin ≥ 0 at both band ends. Today 27 of 112 optima (z₁ ≤ 11) foul at running − 0.02 mm, by 0.0015 to 0.0095 mm. Fixture: 9/37 with the search on reports `[false, false]` while its margin at running − 0.02 is −0.00588 mm.

**Notes.** This moves the reference.md 9/37 row (97.6778 → 97.6672 %, Σx 1.4078 → 1.4222) and `tools/golden/shifts_9_37.txt`. Changing only the report would flag the search's own answers. Tip-diameter tolerance stays unmodelled. The path band's same-end assumption is separate [mesh-contact#0] [train-mod-a#0].

### T06.3 Signed play at the tight end, one convention for both contacts

**Change.**
- In `PointBuilt::angular_play` (shape.rs:2638), drop `.max(0.0)`. The separation `2(a − a₀)·|n_x|` is then signed, like the line contact's `Mesh::backlash`.
- In both `angular_play` and `play_of`'s line arm (shape.rs:3524), add the axial-float term only when the gap is ≥ 0. When the gap is negative, return the gap: float cannot clear a jam, by the code's own premise at shape.rs:2636. State the step at gap = 0 in the comment as physical (free float travel against a jam).
- Build the contact normal once, when `PointBuilt` is made, so the `return 0.0` for a missing normal disappears.
- Rewrite the crossed.rs:476-503 test, whose premise "the flanks would simply touch" is wrong.

**Proof.** Law over every preset and shaft angle: the band minimum is negative exactly when tolerance₋ > clearance. Limit law: at Σ = 0.01° the minimum matches Σ = 0 to first order. Today crossed 17/43 at ±0.1 mm gives −0.389° at Σ = 0 and 0.000° at every Σ ≥ 0.01°. The Worm preset's minimum stays at 4.5366° for every tolerance from 0.02 to 5 mm. `check_golden` must not move at defaults (clearance = tol₋ = 0.02, gap exactly 0).

**Notes.** One alternative keeps `gap + slide` continuous and flags the jam separately. It was rejected because a positive "play" on a jammed mesh is the defect itself. This task lands inside T02.8, which unifies the two play arms [lens-unification#4]; the sign convention here comes first.

### T06.4 "Loses contact" on ε_γ; a separate ISO-scope note on ε_α

**Change.** Change train/mod.rs:471 so that `mesh.contact_ratio_below_one` fires on the ratio that governs continuity: ε_γ on a line contact, the zone ratio on a point contact. Add `mesh.transverse_contact_ratio_below_one` ("transverse contact ratio {ratio}: below 1, outside the range ISO 6336 rates") for ε_α < 1 ≤ ε_γ. Add it to note.rs and the 5 catalogues.

**Proof.** A test in train/mod.rs:
- A 30° helical 17/43 with h_a 0.55 and b 60 (ε_α 0.760, ε_γ 10.310) carries the ISO-scope note and not "loses contact". Today it says it loses contact.
- A spur pair with ε_α < 1 says "loses contact".
- `gear_io::strings` fires the new key.

### T06.5 The load-share ramp conserves load at every ε; the band edge said once

**Change.** In contact.rs:869-925:
- Keep `RAMP_MIN` as the only constant, with `RAMP_MAX = 1 − RAMP_MIN` and its reason (equilibrium) stated.
- `load_share` returns w(d)/Σ_k w(d+k) over the pairs in contact. That is k from ⌈−d⌉ to ⌊ε−d⌋ with 0 < d+k < ε, so the sum is closed form.
- Add `has_single_pair_zone(ε) = ε <= 2.0` (the plateau [ε−1, 1] is non-empty). `Bending::new` uses it in place of its literal `eps_n >= 2.0` (mod.rs:767), and the note fires on ε > 2.

**Proof.** Law: for ε ∈ {1.1, 1.5, 1.99, 2.2, 2.5, 3.0, 3.5} and d over one base pitch, Σ shares = 1 within 1e-12. Today the sum reaches 1.3611 at 2.2, 1.4074 at 2.8 and 1.7333 at 3.5. `the_load_share_is_continuous_and_unchanged_below_two` stays, relaxed from bitwise to one ulp: the two weights sum to 1 only to rounding at 9-35 % of instants.

**Notes.** With w ≡ 1 the same w/Σw is the rigid equal split that T06.8 needs. That makes it the one load-share rule for efficiency and bending. A stiffness-derived w [lens-feature-gaps#4] would slot in as a third weight.

### T06.6 Sharing sweep as a bracketed maximiser; the one-sided law below the band

**Change.** In strength.rs `worst_over_cycle` (:1478):
- Replace the 205-point scan with golden section in `solve.rs`, run on each segment between the share's breakpoints plus the flank end d_end. Seed every endpoint. Stop at the method's √ε_mach floor.
- Delete `SHARING_SAMPLES`, `bending_section_shared_with` (no outside caller) and the convergence test. Add the solve to solve.rs's module doc and to rationale.md's table, whose heading "Ten scalar solves" already lists eleven.
- Unimodality per segment is a measured premise and is stated as one: 2 local maxima were seen in 12 of 582 cases, and golden section still matched there.
- `worst_over_cycle` asserts that it skips no candidate on an external tooth, which T06.1 guarantees.
- Correct the prose to the one-sided law. Below ε_n = 2, sharing leaves the figure unchanged where the HPSTC governs and otherwise raises it. The text to correct is at strength.rs:1557-1571 and :1600 ("0.0–0.2 %"), state.md:735 (which contradicts :739-744), rationale.md:766-772, and the test `..._can_only_ever_relieve_the_tooth`, which should measure the Dolan–Broghamer product and not Iso6336.
- Redocument `LoadSharing::None` as "whole load at the HPSTC".

**Proof.**
- Maximiser against a 20 000-sample scan over the 582-case grid (z 9-97, β 0/15/30, h_a 1.0/1.25/1.35, rings z 40-160, ε 1.05-2.6): never below the scan, at most 1e-5 above. The probe measured ≤ 7.1e-6 with ≤ 154 evaluations.
- Today's scan fails its own 1e-4 bound on z 9/10 at h_a 1.0 (3.7e-4 to 8.2e-4).
- Law, asserted exactly with z 7-10 at h_a 1.0 in the grid: shared ≥ unshared below the band. The size goes into state.md: up to +1.92 % at z9 ε 1.7, +3.82 % at z7 x 0.3.

**Notes.** Land the axial-term sign fix first (T08.7 [added#32]). It removes the V-kink inside the plateau and moves the size, which should be re-measured after it. That the HPSTC is not the worst single-pair point is [added#27]. Duplicate observations of the sweep's doc and test are retired by this task: [added2#108] [added2#111] [ablate-constants-rating#15] [added3#23 (unverified)].

### T06.7 Out-of-band note carries this mesh's own change; fixed figures removed

**Change.**
- In `Bending::of` (train/mod.rs:741-757), when `!has_single_pair_zone`, rate a second time with `LoadSharing::None`. Have the note carry `.number("change", 100·(shared/unshared − 1), 1)`.
- Remove the fixed "−24 % / +15 %" wherever it appears: strings_en:137, de:69, pt:70, zh-Hans:65, zh-Hant:65, the `Bending::note` doc (mod.rs:719-720), state.md:730-732, state.md:1019-1021 ("12–24 %") and reference.md:2847-2848 ("up to a quarter").
- State the measured range once in state.md, tagged with a generator: a test or `gear-cli` sweep that derives ε from real mates over z, β, h_a and the mate.
- Document the step at ε = 2 with its size. Today it is exactly 1 − RAMP_MAX = 1/3 (−33.3 % at z20 h_a 1.371), and it should be re-measured after T06.5. Do not force continuity: the single-pair zone really vanishes there.
- In tests/notes.rs, fix the retired key name in the module doc.

**Proof.** Law over a z × h_a sweep: the note fires iff ε_n > 2, and its {change} equals shared/unshared − 1. The ablation of the 2.0 threshold to 2.1 is silent today and fails under this law. Replace notes.rs's four-case magnitude bands with this law. `check_figures` passes on the tagged block. Current buildable range for reference: −34 % (z17 spur, ε ≈ 2.19) to +25 % (z97 β 20°, ε ≈ 2.3); equal spur pairs z 20-150 reach −33 % and +74 %.

**Notes.** The same change retires T18.10's [added2#26] and T16.27's [added3#5] (unverified).

### T06.8 Efficiency weighted by load per unit contact-line length

**Change.** In contact.rs `efficiency` (:414-439):
- Replace the loss factor (ε₁² + ε₂²)/ε_α with its time-and-face average under the rigid split: load F_n/L(t) per unit contact-line length, with the total held at F_n at every instant, over the ε_α × ε_β field of action.
- It is closed form, piecewise in ε₁, ε₂ and ε_β, with ε₁ and ε₂ taken from T06.1's effective ends. It reduces to Ohlendorf's (1 − ε_α + ε₁² + ε₂²) at ε_β = 0, 1 ≤ ε_α ≤ 2, and to today's expression at integer ε_β. No branch on spur against helical.
- Apply the same 1/L(t) split in screw.rs `CrossedPath::efficiency`, so the Σ → 0 limit still meets. Tighten crossed.rs:1573's 5e-3 bound: changing one side alone opens a 2.4e-3 seam that bound would miss.
- Correct contact.rs:352-357 ("holds the total transmitted force at F_n"), :377-385 and reference.md:596-612 ("Buckingham recovered", "field of action does not matter").
- State default μ 0.08 together with the H_V it is calibrated with (T15.2 [lens-magic-numbers#1]).

**Proof.**
- Rewrite `efficiency_matches_a_numerical_average_of_the_instantaneous_loss` (contact.rs:1704-1740) as a discrete contact-line simulation, as in hv.py and helix.py: lines one base pitch apart sharing F_n by length at each instant, ε_β ∈ {0, 0.3, 0.5, 1, 1.5}, five meshes × three helix angles, to 1e-9. It fails today: 17/43 loss/μ 0.20991 against 0.18012.
- Limit laws: equal to Ohlendorf at ε_β = 0; equal to today's value at integer ε_β; continuous in ε_β (fractions of today 0.858, 0.922, 0.965, 1.000 at ε_β 0, 0.3, 0.5, 1).
- Expected moves: the canary `gear-cli strength 17 43 2.0` goes 98.741 → 98.919 % at μ 0.06. From the probe's ×0.85 proxy, Spur 98.345 → ≈98.59 % and Wolfrom 45.36 → ≈49.39 %. The 17/43 optimum shift stays at Σx 1.26.

**Notes.** The existing test shares the assumption it checks [mesh-contact#8] [lens-docs-accuracy-2#13]. If `efficient_split` has not been deleted by then [mesh-contact#10], its residual must be re-derived from the new integrand.

### T06.9 State the first-order-in-μ residual; correct "exact" and "symmetric"

**Change.** Keep the line-contact closed form first order in μ: it carries the closed form that T06.8 needs.
- Add one tagged "Known-approximate" bullet to state.md: the closed form sits 1.1e-4 below the exact force balance at μ 0.06 on 17/43, and 2.2e-3 at μ 0.3. The gap is O(μ²) (gap/μ² ≈ 0.03) and conservative.
- Reword contact.rs:361-365 to say forward and backward are equal at first order only. The exact balance separates them by 2.0e-5 at μ 0.06 (unverified).
- Correct "exact rather than conservative" in the same doc and rationale.md:862's "falls linearly with μ" (T18.10 [added2#46]).

**Proof.** The existing screw.rs `the_friction_balance_meets_the_parallel_model_at_its_limit` already asserts the O(μ²) decay. `check_figures` covers the new block.

**Notes.** For a fixed load share the exact mean has an elementary per-side closed form (a log term, written with `ln_1p`, locking handled). Take it only if a 1e-4 figure matters, and then with T06.8's weighting. The pitch-point fallback for an empty crossed zone fires only when there is no path at all.

### T06.10 A line contact reports the length its pressure used

**Change.** Store `line_length = b/cos β_b` on the contact-stress record (strength.rs:1848) and pass it to `ContactPatch::line` in place of `face_width` (train/mod.rs:430). Fix the `ContactPatch` doc. First check that `mesh_widths[k]` equals the point contact's min(face[i]).

**Proof.** Add patch length to `the_two_contacts_report_one_patch_at_the_limit` (mod.rs:6106) at 1e-3, and add a row to reference.md's limit table. It fails today: at β 20°, Σ = 0 gives 30.0000 and Σ = 0.01° gives 31.6799. The corpus moves only helical patch lengths.
