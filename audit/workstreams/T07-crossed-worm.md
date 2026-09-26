## T07 — Crossed-axis and worm gearing

**Why.** The screw model is sound where it has been gated (90°, centred faces, positive helices, the shipped worm) and breaks at every edge nobody sampled, and in each case it breaks silently. Four patterns cause almost everything below. (1) An absence stands in for a value: an empty face zone becomes `None`, and callers put the unbounded tip path back in its place. So narrowing a face from 12 to 10 mm raises ε from 0.86 to 1.64 at Σ = 0.5°, and suppresses the one warning meant to catch it [crossed-worm#0]. (2) The pair is parametrised by the worm's diameter rather than by signed helices. Cosine loses the sign of β₁, so an opposite-hand pair is rated as a different pair whose cut gears overlap by 2.75 mm [added#56]. (3) Ill-conditioned numerical solves stand where closed forms exist: a cancelling `sqrt`, a division by sin Σ and an 8-way branch search. As a result the path flickers on and off below 10⁻³° and efficiency reads 1.000000 [crossed-worm#3]. (4) Worm practice (the enveloping-wheel proportions, a constant μ, a "cannot be back-driven" verdict) is shipped as if it were the model. The automatic worm faces are 13.54 / 4.69 mm against the 2.76 / 0.41 mm the modelled contact needs [crossed-worm#6], and the default 8.2° worm back-drives at 40.5 % once it moves [lens-standards#1].

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T07.1 Refuse a helix the screw cannot represent | added#56 | high | S | — |
| T07.2 The face-bounded zone as a total linear interval | crossed-worm#0, lens-continuity#1, added2#15 | high | M | — |
| T07.3 One no-contact rule: refuse | crossed-worm#11 | medium | S | T07.2 |
| T07.4 Centre a crossed face on the operating contact | added2#15 | medium | M | T07.2 |
| T07.5 Width for continuity from one shared window | crossed-worm#1 | medium | M | T07.2 |
| T07.6 Closed-form contact normal, sliding and line of action | crossed-worm#3, shape-a#6, added2#1, crossed-worm#17 (normal) | low | M | — |
| T07.7 Parametrise the screw by signed member helices | crossed-worm#10, added2#16, gear-cli#14, added#56, crossed-worm#17 (m_n, axial_rate) | high | L | T07.1, T07.6 |
| T07.8 State truthfully how face width enters a crossed rating | crossed-worm#4, added#53, added#54, lens-docs-accuracy-2#8 | medium | S | — |
| T07.9 Worm proportions as attributed, named conventions | crossed-worm#5, ablate-constants-rating#7, crossed-worm#6 | medium | S | — |
| T07.10 Limit the ZA/ZN claims to what holds | crossed-worm#8 | medium | S | — |
| T07.11 Size the worm defaults against practice in state.md | lens-standards#0, lens-standards#2 | medium | S | — |
| T07.12 Locking threshold: a principled bracket, no sentinel | crossed-worm#2, crossed-worm#12 | low | M | T07.2, T07.3 |
| T07.13 Report back-driving, starting efficiency and margins as figures | lens-standards#1, lens-standards#0, crossed-worm#16, added2#73, added#61 | medium | M | T07.12 |
| T07.14 Point meshes read at the final width | crossed-worm#13, shape-b#7, shape-b#8, added2#52 | low | M | — |
| T07.15 Automatic crossed face from the model | crossed-worm#6, added#54 | low | M | T07.2, T07.5, T07.8, T07.9 |
| T07.16 One worm builder, one diameter, a seed from the module | graph-ops#12, lens-magic-numbers#5, crossed-worm#20, added3#7 (unverified) | low | S | — |
| T07.17 One rule for worm and wheel roles | shape-b#11 | low | S | — |
| T07.18 Remove the screw module's duplicate computations | crossed-worm#17, ablate-constants-geometry#14 | low | S | T07.6, T07.7 |
| T07.19 One single-pair point definition for both contacts | lens-unification#8 | info | M | — |
| T07.20 Named worm ratings: AGMA 6034 μ(v), ISO/TR 14521 B | lens-standards#0, lens-standards#2 | medium | XL | T07.7, T07.11 |

### T07.1 Refuse a helix the screw cannot represent
**Change.** `Shape::screw_of` (shape.rs:524-530) passes `z·m/cos(helix[m.a])`, so the pair is modelled at |β₁| while helix propagation cuts member b at Σ − β₁. Until T07.7 lands, refuse β₁ < 0 with a named `ScrewError`, and refuse |Σ − β₁| ≥ 90° on member b beside `FirstMemberIsADisc`. Σ 60 / β₁ −30 cuts member b at exactly 90° with d₂ = 3.756e17 mm. Leave β₁ = 0 alone: it is T07.7's case. Add the error key to 5 × `strings_*.toml`.
**Proof.** Law over Σ ∈ {30, 60, 90, 120} and β₁ ∈ (−90, 90) \ {0}: either the solve refuses, or the screw's β₁, β₂ equal `helix[m.a]`, `helix[m.b]` and the members' (d_a + d_b)/2 equals the screw's reference distance, all to 1e-12. HEAD fails at Σ 30 / β₁ −10: radii sum 23.6433 against a running distance of 20.8892, with ε 1.791, pmax 1750.65 and no note. T07.7 then turns the refusals into solves under the same law.

### T07.2 The face-bounded zone as a total linear interval
**Change.** In `screw.rs`, `CrossedPath::limited_by_face` returns an explicit zone that may be empty, never `None`. Write each face bound in the member's axial coordinate, |z_i0 + s·(n·a_i)| ≤ b_i/2. The bound is linear in s, so intersecting it with the tip zone handles n·a_i = 0 (a spur member) with no epsilon: the whole line is inside or none of it is. This replaces `half_span` and its `rate > f64::EPSILON` skip. Delete the fallbacks to `*path` in `PointBuilt::zone` (shape.rs:2500-2505, `.map_or(*path, ..)`) and in `point_mesh_report` (shape.rs:2719-2722, `unwrap_or((*path, Face))`). An empty zone reports ε 0 with `MESH_CONTACT_RATIO_BELOW_ONE` and goes to T07.3's policy. Rewrite state.md:711-717, whose "reports a contact ratio under one" is false once the zone closes.
**Proof.** An independent intersection already exists in probes/repro-crossed-worm-0 and agrees with the crate to 6 digits wherever the zone is non-empty. Gates, all failing on HEAD:
- 17/43 + 20° helix, Σ 0.5°, 0.02 mm clearance, 10 mm faces: ε must be 0 with the note (HEAD reports 1.644148, Face, no note).
- 17/23 at 20°, x = +1/−1, 8 mm faces: ε 0 (HEAD 1.259780).
- 17/23 at 30° with a spur second member, 0.5 mm clearance, spur face 4.7 mm: ε 0 (HEAD 1.1145, Tips).

Monotonicity law over Σ ∈ {0.5, 5, 20, 45, 90}, shifts −1..1, clearance {0, 0.02, 0.3} and faces 0.1..60 mm: ε never increases as either face narrows, and `limited_by == Tips` exactly when ε equals the tip-limited value.

### T07.3 One no-contact rule: refuse
**Change.** A point contact with no path, or with an empty zone after T07.2, returns `TrainError::NoContact`, as a line contact does (shape.rs:2906) and as corrections.md:532 already claims. This removes the whole borrowed answer: the pitch-point efficiency (shape.rs:2486-2492), which enters the flow, the pitch-point locking and rating, `flank_interference = [true, true]` (shape.rs:3868) and `limited_by = Face` with no zone (shape.rs:2767). Before switching, run the relief sweeps and the corpus: search trials call `build`, and a trial with no zone used to be accepted.
**Proof.** Law: no mesh reports a finite efficiency or pressure while its contact is empty. Fixtures that fail on HEAD:
- 17/23 at 90° with 2 mm clearance reports ε 0, η 84.3 %, 1517 MPa.
- The worm at 3 mm clearance reports 62 %, 3376 MPa and "self-locking".

After the change both must refuse, as the parallel 17/23 at 2 mm already does.

### T07.4 Centre a crossed face on the operating contact
**Change.** Faces are centred on `axial_centre` at the reference distance. When clearance opens the distance, the contact slides about Δa/sin Σ along the shafts: centres ±1535.7 path units at 0.5°, 1176 mm at 0.01°, scaling as 1/sin²Σ. So an aligned pair loses contact at ordinary faces for any Σ ≲ 2°. Centre each member's face on the axial position of the operating contact, where a real pair is assembled. Keep the face offset as an explicit input, default 0, if designers need it. State the rule in reference.md#crossed-axes. Once it is in, T07.2's empty zone is a genuine limit rather than an artefact of the convention.
**Proof.** With zero face offset, sweep Σ over (0, 5]° with clearances {0, 0.02, 0.3}: the zone is never empty while b_i ≥ the tip-zone width, and ε is continuous in Σ. HEAD dips to 0.18 at 0.5623° and jumps to 1.638 at 0.7° [added2#15].

### T07.5 Width for continuity from one shared window
**Change.** `face_widths_for` (screw.rs:974-994) mirrors a one-sided interval and sizes each face about its own mid-plane. Choose one window [w₀, w₀ + want] inside the tip zone, with w₀ the clamp of mean(c_i) into [z₀ + want/2, z₁ − want/2] minus want/2. Set width_i = 2·|n·a_i|·max(|w₀ − c_i|, |w₀ + want − c_i|), reusing T07.2's axial form. Name the w₀ rule in reference.md:1057. Do not size each member on its own: ε is set by the intersection of the two windows. On 20°, x₁ = 1 that gives ε 0.74 [crossed-worm#1 verdict].
**Proof.** Law: `limited_by_face(face_widths_for(t)).contact_ratio == t` to 1e-12, over Σ 0.5..110°, shifts −1..1, clearances 0..0.3 and one-sided zones. HEAD fails at:
- 0.5° / 0.02 mm (ε 0.261);
- 5° / 0.02 mm (1.216);
- 45° x₁ = 1 (0.234);
- 20° x₁ = 0.6 (0.805);
- 20° x₁ = 1 (0.363).

The centred gates keep passing at 1.000.

### T07.6 Closed-form contact normal, sliding and line of action
**Change.** In `screw.rs`, replace the numerical constructions with the closed forms:
- **Sliding.** `sliding_ratio = |sin Σ / cos β₂|` replaces `sqrt(1 − 2k cos Σ + k²)`. The worst error over 10⁴ random pairs is 4.4e-16. `Screw::new` then refuses only Σ = 0 exactly.
- **Normal.** One `normal(flank) = [sin αₙ, ∓cos αₙ cos β₁, ±cos αₙ sin β₁]` feeds both `contact_normal` and `rulings`. That removes the division by sin Σ and the duplicate normal (crossed-worm#17).
- **Line of action.** At the reference distance it is P + s·n with P = [r₁, 0, 0]. The tangency parameters come from `foot()`, whose denominator becomes cos²β_b. At another distance, displace the line by δ, solved from u₁·δ = 0, u₂·δ = (a − a_ref)(u₂·x) and n·δ = 0.

This deletes `mesh_branch`, its `1e-6·a` tolerance, the 8-way loop, `solve3` and its `1e-12` guard. Say in the code that the displaced-line solve stays ill-conditioned as Σ → 0: that is the physical slide Δa/Σ (T07.4), not a numerical defect. Keep the Σ = 0 dispatch to the line model (`is_crossed`). A near-parallel threshold that swaps in the line contact would only move the documented seam, so do not add one [shape-a#6].
**Proof.** Sweep Σ log-spaced over 10⁻⁹..1° at several β splits (β_add 10, 20, 30), at zero clearance: the path exists everywhere, ε is continuous, and |η − η(10⁻²°)| < 1e-4. HEAD refuses at ≤ 5.6e-7°, gives ε 0 / η 1.000000 on [1e-6, 3.2e-4]°, and flickers across 1e-3..1e-4° [added2#1]. The 90° and parallel-limit gates hold to 1e-13. Review any corpus diff for size, since contact ratios are printed to 9 digits. `tools/crossed_path.py` needs the matching change (T16.2 [crossed-worm#9]).

### T07.7 Parametrise the screw by signed member helices
**Change.** Target shape:
- **Params.** `ScrewParams { normal_module, normal_pressure_angle_rad, shaft_angle_rad, teeth: [u32; 2], helix_rad (β₁, signed; β₂ = Σ − β₁), profile_shifts }`.
- **Stored fields.** `Screw` stores `helix: [f64; 2]`, `pitch_diameter: [f64; 2] = z_i mₙ / cos β_i`, mₙ itself, and `lead: [Option<f64>; 2]` (None at β = 0), and provides `axis(i)` and `helix(i)`. Signed β_b,i flows into the normal and into `axial_rate`, which reads |n·a_i|.
- **Errors.** They reduce to `NotPositive` and `HelixAtRightAngle(member)`. `WormTooThin`, `FirstMemberIsADisc` and the member-b check T07.1 added are retired, and 5 catalogues change.
- **Worm diameter.** It is converted once, in `Shape::screw_of`, to β₁ = acos(z₁mₙ/d₁), and refused there when d₁ ≤ z₁mₙ.
- **Readings.** Lead and axial module become `Option`, read only on worm distances as now.
- **Duplicates.** Delete the recomputed β₂ at screw.rs:689, 752, 880, 1025 and 1413, and the six hand-built `axis_2`.

Migration, green at each step:
1. Add the new constructor beside the old one, and make the old one a thin adapter.
2. Move `screw_of` and the CLI `worm`/`crossed` commands.
3. Delete the adapter.
4. Remove T07.1's β₁ < 0 refusal.
5. Replace the test `a_crossed_axis_spur_pair_puts_the_spur_member_second`'s "refused" block with the limit law below.

**Proof.**
- **T07.1's law with no refusals.** It now also holds at β₁ < 0.
- **Label swap.** Swapping member labels (Σ mirrored) exchanges η_fwd with η_bwd and the two thresholds, and leaves ε and pmax unchanged, to 1e-12, spur member first included.
- **Spur limit.** [17,43] at β₁ = 0 has the ε and pmax of [43,17] at β₁ = Σ. HEAD refuses β₁ ≤ ~1e-7° as `WormTooThin`, and the golden `crossed_17_43_5.txt` prints "— no such pair". Rerun that golden output with `--write`.

### T07.8 State truthfully how face width enters a crossed rating
**Change.** Correct "no stress depends on the face width" everywhere it appears:
- crossed.rs:78-83;
- reference.md:1052 and 1082;
- state.md:390-394;
- train/mod.rs:676-685 (the Widths::contact doc);
- shape.rs:3143-3149;
- the strings `gear.face_width_as_entered`, `train_note_wheel_width` and `train_note_worm_length` in 5 catalogues.

Face width enters twice. Through the line term of max(ellipse, line), with L = min b_i/cos β_b,i, it governs below about 5-10° at ordinary faces. Through the single-pair bounds of the clipped zone, it matters on any narrow face. At a worm's 90° with faces ≥ 1 mm neither binds. Drop the stale, untagged "differ by 2.4×" in state.md and mod.rs: on the shipped worm the ratios are 4.9× and 11.4× (added#53). In reference.md:941, write the shift sum as (x₁ + x_s1 + x₂ + x_s2)·mₙ, "over the thickness shift, as on parallel axes" (lens-docs-accuracy-2#8).
**Proof.** Replace the equality test in crossed.rs:549 with a two-regime law. Where the line term governs, doubling both faces divides pmax by √2 to 1e-9: at 2°, 852.5 → 602.8 MPa. Where the ellipse governs and the zone is tip-limited, pmax is unchanged, and it rises once b falls below either bound: shipped wheel 0.5 mm gives 4657 MPa against 3376 MPa. Then run `check_strings.py` and `check_doc_links.py`.

### T07.9 Worm proportions as attributed, named conventions
**Change.**
- Drop the 0.67 d₁ cap. It is AGMA's limit on *effective* width inside a rating the crate does not perform, not BS 721, and it binds on every recorded worm: 4.690 = 0.67 × 7. With it goes the false "two statements from the same source" (crossed.rs:126).
- Keep BS 721's b₂ = 2mₓ√(q+1) as the wheel face, attributed exactly. At an addendum of mₓ it is the chord 2√(r_a1² − r₁²) of the worm tip circle.
- Relabel the worm length (11 + c·z₂)·mₓ as ČSN, not "DIN/ČSN".
- Rewrite crossed.rs:88-97: the zone *is* derived now, and the proportions are a deliberate enveloping-wheel convention sized above the model's floor.
- Fix `per BS 721 convention` / `per DIN/ČSN convention` in 5 catalogues and reference.md:1074-1079.
- Do not ship the proposed DIN b₁ chord with a factor 2 until the source is checked.

**Proof.** Test the chord identity 2√((r₁+mₓ)² − r₁²) = 2mₓ√(q+1): 5.68977 by both routes. Replace the self-referential coefficient test (crossed.rs:578-630), which cannot see 2 → 2.1, with a b₂ tabulated in BS 721 or a textbook worked example, asserted to its printed precision. Add a stout recorded case (`worm 1 40 14 90`, q = 14) so the formula reaches the corpus. Expect the wormstage face row to move from 4.690 to 5.690 or 5.657 depending on the addendum, and move the golden output deliberately. T18.10 [lens-magic-numbers#16] is the same misattribution.

### T07.10 Limit the ZA/ZN claims to what holds
**Change.** In rationale.md:873, state.md:673, reference.md:1087, screw.rs:1384 and `tools/worm_flank_curvature.py`, say that ZI is the only worm type the model is exact for. On ZA and ZN, n·a varies along the flank: the spread is 1.33e-2 (ZN) and 1.04e-2 (ZA), against 5.2e-13 for ZI, a 1.6-2.0° tilt. So the path, the shift law, the play and the curvature would all change, and a conjugate wheel would be needed. Qualify "1-15 %" as "a ZN worm on a non-conjugate involute wheel, at the pitch point, over 5-20° of lead". It currently appears in the string `train_zn_worm_s_contact_stress_1` ×5, in gear-cli main.rs:3174/3228 and in golden worm_1_40_7_90.txt:40. Remove the "DESIGN 5" references from `crossed_path.py:32` and `worm_flank_curvature.py:25`.
**Proof.** In the script, assert that ZI's n·z spread is below 1e-10 and ZN's is above 1e-3. Extend `compare()` to 25° and 30° and assert that the deficit exceeds 15 % there (−21.2 %, −37.9 %). Do not assert monotonicity past 30°: the relative curvature goes negative there.

### T07.11 Size the worm defaults against practice in state.md
**Change.** Add two Known-approximate entries to state.md, each with size and sign (rule 6):
- **Friction.** The speed-independent default μ = 0.08 / 0.16 against AGMA 6034 for a steel worm on a bronze wheel. At the canary's 0.439 m/s, AGMA gives μ 0.0574 and forward η 69.6 % against the tool's 61.805 %. Scope note: the shipped wheel is Brass C360, outside AGMA's curve.
- **Contact stress.** The worm's point-contact peak is correct for the cylindrical wheel modelled, and it is 3.5-3.9× ISO/TR 14521 method B's σ_Hm for an enveloping wheel on the same drive: 3376.1 MPa against 875-967 MPa. It cannot be compared with worm-gear allowables.

Reword rationale.md:911-913, which says the stage reports what a worm is limited by. Wear and heat are not computed. Name the default friction's source or its absence together with T15.2 [lens-magic-numbers#1].
**Proof.** Tag both figures for `check_figures.py`, generated by `gear-cli wormstage` and a `tools/` script of AGMA 6034's μ(v), whose μ 0.06 row reproduces the tool's 68.691 %.

### T07.12 Locking threshold: a principled bracket, no sentinel
**Change.**
- **Bracket.** `CrossedPath::locking_friction` brackets on [0, pitch threshold], which fails in forward drive with a high-lead driver: 195 of 512 cases have no root in the bracket. `PointBuilt::locking_friction` then substitutes the pitch-point figure beside a path-average efficiency. Bracket instead between min and max over the zone of each point's closed-form zero μ_i = −T_out(0)/T_out′(μ). Each point's efficiency is a ratio of moments linear in μ, so no ceiling and no fallback are needed.
- **Type.** Make `MeshReport::locking_friction` a `Directional<Option<f64>>`, the type the path already returns. None means "no friction locks it within the model", and it replaces both a line contact's literal −1.0 (mod.rs:483) and a point contact's negative root. The consumers that read only the sign follow: TrainPanel.svelte:489, shape.rs:2700 and main.rs:3059.
- **Docs.** Delete the false "always inside" and "slightly lower friction" wording (screw.rs:1203-1213, reference.md), and delete the "the path average can only lose" test law: it fails in both directions, worst +0.0072.

The ε split into two named measures is optional. If it is done, the normal-plane count ε_α/cos²β_b can be filled on both contact kinds.
**Proof.** Law over the probe grid (8 tooth pairs × 5 shaft angles × 7 splits), both directions: wherever a threshold is reported, the path's η(threshold) = 0 to 1e-12, positive at 0.99× and negative at 1.01×. HEAD fails at 2/41, 90°, β₁ 4.5: path root 0.07421 against a reported 0.07396, and forward η 0.00143 is reported beside the pitch fallback. A grep finds no `Directional::of(|_| -1.0)`.

### T07.13 Report back-driving, starting efficiency and margins as figures
**Change.**
- **Two new figures.** Report the running back-driven efficiency, which the solve already holds as `self.sliding[k]` before `once_moving` clamps it (contact.rs:307), and the forward starting efficiency at the static coefficient. Both cross as note values (rule 2).
- **Self-locking note.** Reword it in 5 catalogues to "statically self-locking at μ_s = x; back-drives at y % once moving", and fix rationale.md:923-925 ("the answer a handbook gives" is the reverse of the handbooks). For the 8.2° default the running figure is 40.5 % at μ 0.08.
- **Near-locking margin.** Replace the literal `0.8 * threshold` (shape.rs:2700) and its `else if` with a named band on μ, stated in reference.md and exposed beside the two coefficients. Fire "near" in either direction when |μ_s − threshold| < band·threshold, so a lock won by 1 % is flagged. Do not use the "threshold between μ_k and μ_s" rule: it can never fire on an unlocked mesh [added2#73].
- **Low-efficiency note.** Ask the `efficiency < 0.5` note in the directions the enabled cases drive the mesh, never in one already reported locked, and name the 0.5. Drop "which usually limits a crossed-axis stage" from strings_en.toml:142.
- **Comments.** Fix contact.rs:259 and mod.rs:5979-5981, which credit the lock to μ = 0.06 rather than to the static 0.16 against a threshold of 0.1486. Write "at a static coefficient above its locking friction".

**Proof.**
- **Back-driving law.** Wherever tan γ > μ_k/cos αₙ, the running back-driven efficiency is positive and reported. The canary line for `wormstage` shows 40.5 %.
- **Note boundaries.** A unit test fires each note on either side of its named boundary.
- **Mirror law.** The mirror pair (17/23 at 9° against 81°) gets the same notes with directions swapped. HEAD gives forward_locking plus low_efficiency "0.0 %" on one pair and only self_locking on its mirror.
- **Test repair.** T16.29 fixes the test that accepts either note [added2#107]; land it with this task.

### T07.14 Point meshes read at the final width
**Change.** `cut` takes each point mesh's efficiency at `early_width` (shape.rs:3190-3193), and the member rating reads `point_probes` at the same width (shape.rs:3358). The mesh report and the locking threshold are taken at `final_width` (shape.rs:3495, 3512, 3857-3864). Fix:
1. Make `rating()` take the point patches as an argument, pass `rated_point` in the final call, and delete `point_probes`.
2. After widths settle, recompute point-mesh efficiency, zone and threshold at the final faces, and re-run the flow once when any changed.

This crosses the cut/rate split, which is why the effort is M. Do not pin a point member to its early width: the line mesh it shares is carried at min(widths), so it would be rated at a width nobody asked for. Correct the comments at shape.rs:3141 and 3405.
**Proof.**
- **Frozen widths.** For every shape, replacing each automatic width with `Auto::fixed(result)` reproduces every mesh and path efficiency bit for bit. HEAD fails on crossed + AddGear on member 1: 0.8429731 against 0.8429475, and 0.842553 against 0.843034 at 1 N·m.
- **Contact stress.** Each member's contact_stress equals its point mesh's max_pressure bit for bit. HEAD gives 2902.238 against 2892.964 MPa at a 2 mm box.
- **Locking consistency.** efficiency(locking threshold) = 0 at the reported widths.

### T07.15 Automatic crossed face from the model
**Change.** An automatic crossed face becomes b_i = max(2·max|zone end − c_i|·|n·a_i|, 2a_patch·cos β_b,i) over every load case. The first term holds the whole tip zone, the second the Hertz patch. The zone term alone raises pmax by 9 % on the shipped worm and 22 % on 2/41, so it is not enough. Where the line term governs, invert it in closed form: L = P·E*/(π R_y σ_allow²) and b = L cos β_b, taking the larger. Where the ellipse term alone exceeds the allowable, no width helps, and the result is None with a note. The worm conventions of T07.9 stay as an explicit "enveloping-wheel proportions" option. The worm bit then changes no figure (shape.rs:3156 is its only numeric use), and a worm becomes a label. This reverses reference.md:1063's decision for crossed gear pairs, so argue that change there explicitly.
**Proof.**
- **Rating unchanged.** At the automatic faces, `limited_by == Tips` and pmax equals its wide-face value to 1e-12, for every crossed preset over a shift and clearance grid.
- **Worm figure.** The shipped worm's automatic faces are at least 5.24 / 0.77 mm, and pmax stays 3376.09.
- **Worm bit.** Toggling it changes only names.

### T07.16 One worm builder, one diameter, a seed from the module
**Change.**
- **Builders.** Build `worm_and_pair` as `worm(..)` followed by the graph's own `Edit` (AddGear/AddRatio on the wheel's body). It currently keeps 10 mm faces against the recommended 13.539 / 4.690, a 0.38 fillet, zero helix seeds and a steel wheel.
- **Constants.** Name `const WORM_PITCH_DIAMETER` once, next to `worm()`, and use it in `ScrewParams::default` too. Name the preset module rather than writing `Builder::new(1.0)` and `f64::from(teeth)`.
- **Helix seed.** Seed the helix from the shape's own module and diameter, acos(z₁m/d₁), only where z₁m < d₁. Otherwise leave the box unseeded; never store NaN. Today `worm(8, 40)` stores NaN and fails a serde round trip. `with_first_diameter` reseeds the automatic boxes (added3#7, unverified).

T01.4 [added2#35] is the same NaN.
**Proof.**
- **Round trip.** Every arrangement builder's output round-trips through serde_json for its documented arguments. HEAD fails at `worm(8,40)` and `worm(17,23)`.
- **Seed.** `worm(1,40).with_first_diameter(10)` seeds acos(1/10).
- **Fixtures.** `worm_and_pair`'s worm matches `worm()`'s shift, root radius, faces and material. Rerun the corpus and kinematics fixtures.

### T07.17 One rule for worm and wheel roles
**Change.** Add `is_worm_wheel` (member b of any mesh on a worm distance) beside `is_worm_thread`. Use both in `member_names` (shape.rs:731) and in cut's recommended widths (shape.rs:3130). The existing ordinal logic then names "Worm 1 / Worm 2".
**Proof.** Extend `every_preset_and_arrangement_names_its_members_as_a_designer_does` with the worm preset plus AddRatio on distance 0: every name's role agrees with `is_worm_thread`/`is_worm_wheel` and with `recommended.is_some()`. HEAD names member 2 (z 1, recommended 13.539) "Gear".

### T07.18 Remove the screw module's duplicate computations
**Change.**
- Delete `enum Flank` in favour of `Drive`.
- `flank_curvature` calls `plane::transverse_pressure_angle` and stops returning the β_b it discards (`let _ = …`, screw.rs:1416).
- Remove the |Σ − 90°| < 1e-12 branch in `least_distance_lead_angle` (screw.rs:437) and keep atan(∛(z₁/z₂)) as a test law (rule 4). Rewrite the docstring that says the branch value "is returned exactly". Widening the branch to 1e-6 left all 679 tests and 32 golden outputs unchanged.

The double Hertz call in `patch_at` is T08.11's [strength#8]: `hertz::peak_pressure` should return the governing patch.
**Proof.** Add the law `least_distance_lead_angle(z₁, z₂, 90°) = atan(∛(z₁/z₂))` to 1e-12 for six pairs. The suite passes, and any corpus diff is reviewed for size.

### T07.19 One single-pair point definition for both contacts
**Change.** The line contact places the single-pair point one transverse base pitch in along the path, the point contact one normal base pitch along the line of action. Define it once as one contact spacing along the path's own parameter (p_bn·cos β_b projected, against p_bt = p_bn/cos β_b), used by both. Leave the flank load alone. The pitch-pressure seam is Coulomb friction's own discontinuity at zero slip, recorded in state.md:700-710. Importing F(±n̂ + μv̂) into line contacts would leave v̂ undefined at the parallel pitch point.
**Proof.** Invert the half of `the_two_contacts_report_one_patch_at_the_limit` (mod.rs:6141) that asserts a peak-pressure seam greater than 3e-2: at Σ = 0.01° the peak pressure meets the line value to 1e-3. The friction seam stays recorded. Update reference.md:1152's table.

### T07.20 Named worm ratings: AGMA 6034 μ(v) and ISO/TR 14521 B
**Change.** This is optional, and each part is an explicit, user-visible named option, never a silent default:
- **Friction.** AGMA 6034 μ(v_s) = 0.103·exp(−1.185·v^0.45) + 0.012, with a static value of 0.150, for a steel worm on a bronze wheel only. Other pairings fall back to the manual μ. Speed-dependent friction makes efficiency per-case, so it moves from `cut` into `rate` and multiplies the path report.
- **Rating.** ISO/TR 14521 / DIN 3996 method B: σ_Hm, wear intensity and sump temperature. It needs an enveloping-wheel geometry the crate lacks.

**Proof.**
- **Friction.** Reproduce AGMA's efficiency at three sliding speeds against `worm.py`: 69.6 % at 0.439 m/s and 76.0 % at 1.111 m/s.
- **Rating.** Reproduce a published ISO/TR 14521 or DIN 3996 worked example to its printed σ_Hm.
