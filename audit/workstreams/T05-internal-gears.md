## T05 — Internal gears and internal meshes

**Why.** `ring.rs` repeats the external gear's machinery with the sign flipped. Each copy carries a one-sided assumption that the external case never needed, and nothing checks it:
- the tip circles of a meshing pair always cross;
- the pinion tooth always leaves the lens through its own space;
- the cutter's corner always lies outside its operating pitch circle (a prolate path);
- the ring's shift always stays inside the cutter's involute domain;
- the far side of the cut is never reached.

Where an assumption fails, the code does one of three things. It returns a silent default (`+∞`, reference centres, a placeholder cutter). It takes the wrong branch. Or it gives a verdict that the independent roll contradicts. Three numbers show the size of it:
- When the pinion's tip circle encloses the ring's, the tips are reported clear in all 1,146 designs scanned, and all 7 of those that were rolled foul, by 0.19–0.66 mm [ring#1].
- 4,182 of 28,800 swept rings have a curtate cutter path. 3,336 of them lose their fillet under a false "cutter does not reach this ring's flank", and bending goes unrated [added#30].
- The ring tab's "smallest tooth count" is wrong in 20 of the 24 designs swept, by up to 5× (34 against 7) [wasm-boundary#2].

The tasks below replace each assumption with the signed, continuous quantity it was standing in for. They then shrink the second internal-mesh model, and finally close the Tooth/Ring split.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T05.1 Signed, unfolded tip margin; the far gap enters the verdict | ring#0, ring#1, added3#24 (unverified) | high | M | — |
| T05.2 Curtate cutter path: signed trochoid offset and a closed-form junction | added#30, ring#5, ring#11 (junction) | high | M | — |
| T05.3 Tip room judged at the tight end of the tolerance band | added#66 | medium | S–M | T05.1 |
| T05.4 No reference-centre fallback: clamp the ring's shift at the cutter's domain edge | ring#2, lens-errors-policy#1 | medium | S | — |
| T05.5 Refuse a ring whose tip reaches its root | added2#19 | medium | S | — |
| T05.6 Smallest tooth count in gear-core, with the shift | wasm-boundary#2, added#4, lens-docs-accuracy-1#0, added2#101 | medium | S | — |
| T05.7 Rim radius from the root | ablate-constants-geometry#2 | medium | S | — |
| T05.8 Mesh order normalised once; wiring refuses rather than defaults | lens-unification#1 | medium | S | — |
| T05.9 A pointed cutter cuts with its point | ring#4 | medium | M | T05.2 |
| T05.10 Cutter trimming: signed check and a no-trim cutter clamp | ring#3, ring#11 (z−1 cap) | medium | M | T05.1, T05.9 |
| T05.11 "Flank ungenerated" is a finding, not a clamp; a tip-form radius | ring#6 | medium | M | — |
| T05.12 Shrink RingMesh to the tip room | lens-unification#3, ring#7, mesh-contact#4 | medium | M | T05.1, T05.3 |
| T05.13 The ring's own shift range | added2#79 | low | M | T05.4, T05.5 |
| T05.14 Place a mesh from each member's realised thickness; drop `as_gear` | ring#8 | low | M | — |
| T05.15 Sign and guard literals | ring#11 (literals) | low | S | — |
| T05.16 Prose that contradicts the code | ring#10 | low | S | T05.1, T05.9 |
| T05.17 One tooth form for both kinds (redesign) | ring#9, gear-outline#7 | medium | XL | T05.2, T05.9, T05.12, T05.14 |

### T05.1 Signed, unfolded tip margin; the far gap enters the verdict
**Change.** Rewrite `ring::tip_clearance` (ring.rs:940-975) around one signed quantity:
- Let D = θ_p − (θ_r − π/z_r)·z_r/z_p and H = half_p + half_r·z_r/z_p. The margin is `min(D − H, 2π/z_p − D − H)`. This drops the `fold(..).abs()`, which passes a tooth that has overtaken a ring tooth inside the lens.
- Return +∞ only when `a <= R_a − r_a` (the ring's circle encloses the pinion's) or `a >= R_a + r_a`.
- When `a <= r_a − R_a` (the pinion's circle encloses the ring's), do not return early. Fall through to the existing clamp at θ = π. The margin there is negative and continuous with tangency.
- Remove the `half_p <= 0 || half_r <= 0` early return.
- In `TipRoom::at`, set `tip_interference = margin < 0 || far_gap < tip_clearance` [added3#24 (unverified)].
- Do not use KHK's E: it is one-sided. On the automatic planocentric 29/30 (roll +0.0005 mm) it gives −0.0012.

**Proof.** Write the oracle first: an analytic roll in ring.rs's tests (involute teeth, zero backlash, every pinion outline point tested against ring material; about 40 lines, sharing no code with `tip_clearance`). It matches the harness roll to 0.01 mm. Then add these, all failing at HEAD:
- 48/46 at x 0.9/0.9, a = 1.000 must report a foul. Today the margin is +0.0391 rad, every flag is false, and the roll is −0.512 mm.
- 24/23 at xr 0.8, xp 0.2, cutter 12 must report a foul (enclosing case, −0.187 mm).
- 43/42 at ha 0.7/0.8, a = 0.5 must report a foul (today `tip false (+inf)`, roll −0.587 mm). This replaces `any_pair_that_meshes_has_tip_circles_that_cross`.
- Stated 30/32 at 0.96 must report a foul (far gap −0.014 mm).
- The law: over tooth differences 1–10, shifts and addenda, "all flags clear" implies roll ≥ −0.02 m.
- Controls: the automatic planocentric 28/30 and 29/30 stay clear.

The unfolded margin alone passed the whole suite (680 tests) and moved the corpus only by a −0 sign on two planet torques. That is why the new laws are the gate.

**Notes.** The train reaches this fault: planocentric(28,30) at a fixed 1.0304 reports `teeth_clear` on a pair that fouls by 0.44 mm. Rewrite the false premise at reference.md:1636-1638 and the `tip_clearance` doc (both angles are measured on the mesh side) [ring#10]. A roll instrument in `verify.rs` is T20.2's [gear-cli#3]. This task needs only the test oracle.

### T05.2 Curtate cutter path: signed trochoid offset and a closed-form junction
**Change.** Store κ = sign(r_g − r′_c) once on `ShaperCut`, beside `corner_radius`.
- Use k = 1 + κρ/d in `trochoid_at`, `trochoid_point_and_tangent` and the curvature form (shaper.rs:249/304/357).
- Replace `solve_junction`'s bracket walk (−m_t, ×1.4, 64 tries, then Brent; ring.rs:528-548) with s_j = r′_c(α_w − atan t_g), where t_g = √((r_g/r_bc)² − 1). This matches the solver to 1.5e-13 mm over 5,399 prolate rings. Its sign picks the branch by itself, and as z_c → ∞ it tends to Tooth's s_tan.
- Handle κ = 0 (the corner passes through the pitch point) as its own case. The fillet there is the round's arc about the stationary pitch point at s = 0. Today it is NaN at 30/20, x = 0.68325895.
- Do not cap the round. A curtate path is a real tool at a real setting.

**Proof.** These gates fail at HEAD:
- The law trochoid(s_j) = involute(u_j) to 1e-9 mm. At HEAD, 841 fillets break it, every one curtate. 194 of those carry no clamp, and the worst is 0.218 mm (90/80, ρ 0, ha0 0.8, k 0.8, x 1).
- `trochoid_at(0) = rf` on every ring. At HEAD it gives rf − 2ρ when the path is curtate.
- rf, r_j and the half-profile are continuous in x through 30/20 at x ≈ 0.683. At HEAD u_j jumps 0.094 mm there.
- check_ring_cut < 5 µm on a grid that pairs large shifts with small z − z_c: 30/20 at x ≥ 0.8; 25/20 at k 0.6, x 1; the hula's 19/14 at x 0.607, ha 0.7, ha0 1.0; ha0 from 0.8 to 1.25. At HEAD, 30/20 at x 0.8 reads 0.0589 mm, against 0.0032 once corrected.

This extends `a_shifted_ring_is_the_shape_its_cutter_leaves`, whose grid never leaves the prolate case (T16.12 [added2#112]).

**Notes.** `gear-cli hula 18 0.5` and tools/golden/hula_18_0.2.txt lose the false fillet-gap note and gain σ_F. The mid-space crossings of [ring#5] (221 of 17,751 fillets, 8 of them unclamped) are this same wrong branch. Its proposed first-root search would not help, because θ(s_j) already exceeds the half pitch.

Afterwards, check whether `CLAMP_RING_FLANK_FILLET_GAP` can still fire. Retire it if it cannot, or reword it (the message is false in the curtate case). `ring_fully_filleted` cannot fire, because the capped round leaves θ(0) < half pitch (largest θ(0) − hp is −5.65e-7). Replace its UNFIRED "likely reason" with that argument, or delete the note.

### T05.3 Tip room judged at the tight end of the tolerance band
**Change.** On an internal mesh, the crossing margin and the far gap both rise with the distance. Evaluate `TipRoom` at `running − tolerance_minus` in these places:
- in `Shape::tip_room` (shape.rs:1515-1560), inside the Brent solve;
- in the report (shape.rs:3844).

Do not add tol₋ to today's root: the ring re-shifts with the distance, so that shortcut is not exact. Add one helper that judges admissibility across the band: flank interference at the plus end and tip room at the minus end, with no sign per check. The report then states "clear across the band".

**Proof.** The shipped Planocentric (30/33, ±0.02) at running − 0.02 must give no tip interference and a margin ≥ 0. At HEAD it is −0.0811°, with foul TRUE. The unsized 30/32 far gap, which goes from +0.0151 to −0.0049 mm, must be caught (T10.10 [added2#117], the same change). A fixture with tolerance_minus = 0 must reproduce today's distances to the bit. The assertion at arrangements.rs:1858-1863 moves to the tight end. graph.txt and kinematics.txt change, and their diffs need review.

**Notes.** The crossing margin also needs its own clearance (T10.10 [lens-continuity#4]). It goes into the same helper.

### T05.4 No reference-centre fallback: clamp the ring's shift at the cutter's domain edge
**Change.** Delete `operating_geometry(..).map_or(r − r_c, ..)` (ring.rs:363-373). Past x_lim = −inv α_t (z − z_c)/(2 tan α_n), clamp x itself, for ra and for the cut alike, to a margin above x_lim. Derive that margin from a minimum α_w named in `guard`. Clamping to exactly α_w = 0 still reads 0.02–0.04 mm off the cut. Push `clamp.ring_shift_raised` with the honoured x, and add the string to all 5 catalogues.

**Proof.** A continuity law: rf(x) is Lipschitz across x_lim on 60/50, 60/20 and 43/20. At HEAD, 60/50 jumps 0.296 mm between x = −0.20425 and −0.20525 with empty clamps. At HEAD, 60/20 jumps 1.138 mm at x = −0.819, a shift the ring tab accepts. The note fires exactly below the bound. check_ring_cut < 5 µm on every ring with empty clamps, on a grid down to x = −1. At HEAD, 60/50 at x = −0.20525 reads 0.0718 mm and `ring_is_cut_as_asked` is true.

### T05.5 Refuse a ring whose tip reaches its root
**Change.** After both clamps in `Ring::cut_by_at_virtual_z`, refuse the ring when ra ≥ rf. There is no tooth left, so under rule 5 this is a refusal, not a clamp. Give it a catalogued key. The crossing shift x = h_a − (r − rf_max)/m is closed form, and T05.13 uses it as the ring's upper bound.

**Proof.** A gear-core law over x ∈ [−3, 6] and z ∈ {20, 30, 60, 100}: every ring built has ra < rf. At HEAD, z = 60 at x = 5 (ra 34.000, rf 33.150) and z = 30 at x = 4 both produce a DXF (29,755 and 9,627 bytes). They must be refused. A minimum tooth height is a separate question for the user and is not part of this refusal.

### T05.6 Smallest tooth count in gear-core, with the shift
**Change.** Add `ring::smallest_tooth_count(params)` next to the tip clamp. It returns floor(T) + 1, with T = 2(h_a − x) cos β/(1 − cos α_t). Use the guarded α_t from `plane::transverse_pressure_angle`, and return 1 when h_a − x ≤ 0. Delete gear-wasm lib.rs:554-561 (the only engineering formula in the boundary crate; α = 0 saturates it to u32::MAX), the copy in the ring.rs test, and the wasm test that pins the x = 0 case. `RingSummary` then calls the core function. Fix the field's doc formula [added2#68].

Say in its doc and in reference.md:1592 that this is the base-circle bound only: the thin-tooth clamp can fire at large z, and `ring_flank_ungenerated` can fire at these counts. Also carry x in the formula at reference.md:1592. Optionally report the band's upper end, where ψ_b < 0.

**Proof.** A law over x ∈ {−0.5, −0.3, 0, 0.3, 0.5, 0.8}, h_a ∈ {0.6, 0.8, 1}, α ∈ {14.5, 20, 25}° and β ∈ {0, 30}°: `Ring::cut_by` at n raises no base-circle tip clamp, and at n − 1 it does. Keep z below the thin-tooth end. It fails at HEAD at x ≠ 0: at x = −0.3 the tab says 34 and 44 is the first clean count; at x = 0.8 it says 34 against 7. Also add a shifted ring to tools/wasm_probe.mjs and run `check_wasm.sh --write`.

**Notes.** This is also T14.2 [lens-architecture#0, added3#15 (unverified)].

### T05.7 Rim radius from the root
**Change.** `Ring::rim_radius` (ring.rs:587) becomes rf + 2m_t, and its doc says "two modules of material beyond the deepest cut". Today there are about 0.75 m of material at x = 0.

**Proof.** A law over x ∈ [−0.5, 1.2], z ∈ {30, 43, 60, 100, 200} and cutter addendum ∈ {1.25, 1.4}: rim − rf ≥ 2m_t − 1e-12. It fails at HEAD on z43 at x 1.0 (−0.0696) and z100 at x 1.0 (−0.1751). The ring DXF construction circle and the panel annulus change, so review the golden diff.

### T05.8 Mesh order normalised once; wiring refuses rather than defaults
**Change.** Add a `Shape` method that orders every internal mesh as (gear, ring). Call it from gear-io's `relieved()` and from the wasm train entry. Fifteen or more consumers assume b is the ring, so normalising once is lower risk than teaching each one both orders. `Shape::wiring` returns a Result: `kind_of(k).ok_or(NotAMesh)` in place of `.unwrap_or(External)` (shape.rs:4150). Give two rings in mesh their own refusal key.

**Proof.** A law over every preset and arrangement: swapping a and b in every mesh, then loading, leaves `Train::motion` and every `PathReport` identical. At HEAD a swapped planetary gives −5/1 against 7/1, and the solve refuses with an unrelated NoCommonDistance. A negative fixture: two rings in mesh are refused with the new key.

### T05.9 A pointed cutter cuts with its point
**Change.** When no round fits the cutter's tip, cap its effective addendum at its pointed radius, as `Tooth` does for a pointed tooth (tooth.rs:455-470), and push a note. The fillet is then the point's trochoid: the ρ = 0 form of T05.2. Delete the placeholder `ShaperCut` (ring.rs:452-464; corner at the cutter pitch radius, tip round 0, phase 0) and the `cutter_no_tip_corner` path. `check_ring_cut` refuses a ring without a real cut instead of simulating a fake one. Fix `corner_angle`'s comment: it returns None past the boundary so that `largest_tip_round` can find it, and the caller caps the round at 95% of that.

**Proof.** r_j, rf and the half-profile are continuous in k across the old threshold on 43/20 and 60/20. At HEAD r_j jumps from 22.5952 to 22.7500 between k 0.775 and 0.78, and the drawn profile moves 0.026 mm against 0.004 for the neighbouring step. check_ring_cut < 5 µm on both sides. The ring stays rateable (`ToothOutline::is_usable`).

**Notes.** Related: T03.11 and T18.22 [tooth-form#4, added2#58], where the docs disagree on whether the round is capped or refused.

### T05.10 Cutter trimming: signed check and a no-trim cutter clamp
**Change.** In `Ring::cut_by`, evaluate T05.1's two-sided margin with the cutter as the pinion, at a_cut, and with the cutter's tip half-width taken including its round. Replace the `(z − 1).max(1)` cutter cap (ring.rs:324) with the largest cutter count that clears; z − 1 is close to the worst choice. Push `clamp.ring_tips_trimmed` when the user's own count trims. Run check_ring_cut with spans ≈ z_c/2 + 1 so the instrument reaches the far side. `ring_cut_envelope_spans` already takes the parameter.

**Proof.** A law: on a grid of (z, z_c, ha, ha0) with no trimming note, the full-travel envelope equals the ±2-pitch one. At HEAD this fails on 43/40 (0.551 mm), 43/38 (0.318), 43/35 (0.037) and the clamped 43/50→42 (0.893). On 26/20 at hula proportions it fails by 0.024 mm. Every one of these today reports 0.0027 mm and carries no note. The shipped presets must stay clean: all have a positive signed margin.

### T05.11 "Flank ungenerated" is a finding, not a clamp; a tip-form radius
**Change.**
- Move `ring_flank_ungenerated` from `Ring::clamps` to the ring's notes, beside the rack's undercut note (shape.rs:3683). `GearResult.clamps` then means what its doc says (train/mod.rs:1425): guards that altered the geometry.
- `ring_is_cut_as_asked` reads only real clamps.
- Add `Ring::tip_form_radius = max(ra, generation_limit)`. Start the conjugate contact path there; ContactPath is fed from shape.rs:2886-2889, not from `flank_ends()`. Keep the physical tip for the mate-side interference check.

**Proof.** A law: the ring-side start of every internal contact path is at or above `generation_limit`. The default 43/20 ring has empty clamps and one note. The shipped Wolfrom rings 60 and 61 (graph.txt:255, 260) move from clamps to notes. Review the golden diff.

**Notes.** What actually turns the search off is a Given ring judged as a candidate, which T12.4 fixes [added#8].

### T05.12 Shrink RingMesh to the tip room
**Change.** Reduce ring.rs:849-1101 (about 250 lines) to `tip_room(ring, pinion, a) -> TipRoom`, fed by the `Mesh` the train already built. Delete `RingMesh`, `mesh_with`, `mesh_at`, `reference_geometry` and `described_at`. Every internal mesh of every trial pays 0.55 µs for them, and production reads only the tip fields.
- Port `gear-cli meshsweep` to `Mesh::new` + `ContactPath` + `flank_interference` + `tip_room`.
- Move the textbook internal contact-ratio formula into ring.rs's tests as the oracle against ContactPath. It is the one independent check left, and ContactPath's doc (contact.rs:71-72) leans on it.
- Rewrite the docs that cite `mesh_with`: mesh.rs:568, contact.rs:72 and auto.rs:1316-1322.

**Proof.** tools/golden/meshsweep_60_20_0.8.txt stays byte-identical, and the hula and meshsweep lines match to their printed precision. The oracle test passes and fails under a perturbed ContactPath.

**Notes.** Deleting the circular self-comparison test is T16.23 [added#10]. Moving TipRoom into ring.rs is T14.10 [lens-architecture#12]; if both land, do it here.

### T05.13 The ring's own shift range
**Change.** Add `Ring::admissible_ranges`:
- Lower end: the larger of T05.4's domain edge and the tip-at-base-circle or pointed bound (the `ra_min` clamp).
- Upper end: the smaller of the space cap and T05.5's ra = rf crossing.

Return it as `RingSummary.ranges`. In internal mode, GearPanel reads `ring.ok.ranges` instead of the external solve's ranges (GearPanel.svelte:124). Feed the same bound to the search and relief for ring members. Before this lands, show no external bound on an internal tab, as the train card does (T19.3 [web#3, added3#4 (unverified)]).

**Proof.** A test that each end equals its closed form for several (z, z_c, β). A ring built just inside either end raises no clamp, and one just outside does. The ring tab refuses x below the lower end. On z = 30 today it admits [−2.13, 0.10), which the cut clamps.

**Notes.** No derivation of x_lim as a *ring's* bound exists [added2#79]. The derivation is T05.4's (inv α_w = 0 at the cutting distance), and it must be written down with the code.

### T05.14 Place a mesh from each member's realised thickness; drop `as_gear`
**Change.** Give `Tooth` and `Ring` one accessor, `realised_x_thick()`:
- Tooth: (s_t cos β/m − π/2)/(2 tan α_n), from the s_t it cut.
- Ring: `x_space`.

`Mesh::new` takes these per member, through a small `MeshMember`, instead of raw params (mesh.rs:250-275, 745). Drop `as_gear: Tooth::new(p)` from `BuiltMember::Ring` (shape.rs:2802-2806). It costs about 10% of each ring build.

**Proof.** A 60-tooth ring with x_r = 2.0 and `ring_space_capped`, against a 20-tooth pinion: play at a_w is 0. Today it jams by −0.0456 mm. External 20/40 with x2 = 2.0 and `tooth_thickness_capped`: play 0, against +0.0445 today. Every preset law and the corpus stay unchanged: searches never reach a capped thickness.

**Notes.** This is step 1 of T03.15 [lens-unification#2].

### T05.15 Sign and guard literals
**Change.**
- ring.rs:282 uses `guard::TIP_ABOVE_BASE_FRACTION` in place of `1e-9`.
- shape.rs:689-692 uses `kind.signed(z)`.
- shape.rs:1054-1057 uses `self.kind_of(k).map_or(1.0, MeshKind::sign)` in the form 1 + σ(1 − x).

The junction bracket goes in T05.2, and the z − 1 cap in T05.10.

**Proof.** nextest and the corpus stay bit-identical.

### T05.16 Prose that contradicts the code
**Change.**
- shaper.rs:37-40: σ appears in two places, the axis distance and the tip side, and never in the rolling. This agrees with reference.md:1562.
- shaper.rs:14, 33: `profile::Tooth` becomes `tooth::Tooth`.
- ring.rs:515/518/598 and reference.md:1580/1583/1593: a sin α_t becomes a sin α_w, which is what the code uses.

T05.1 carries the tip_clearance and reference.md:1638 fixes, and T05.9 the corner_angle one. These lines are shared with T18.22 and T18.23 [added#0, lens-docs-accuracy-2#7]; land them once.

**Proof.** `tools/check_doc_links.py`, and a reread against ring.rs:533 and 611.

### T05.17 One tooth form for both kinds (redesign)
**Target.** One tooth-form type carrying σ = sign(z) and a generator {Rack, Shaper}. Every sign-flipped site becomes one expression: `involute_at`, flank point and tangent, load direction, generated thickness, ra, the mirrored inv⁻¹ guards (added#11), sections, sampling and profile, and outline. `Gear` becomes the assembly over the form, with one outline walker. The claims that are false today become true: "one outline path" (gear.rs:48), and state.md:628's "the core supports" an eccentric ring.

What stays deliberate: metrology's signed `Space`, `plane::base_helix_angle`, and the ToothOutline seam's internal tangent angle and rim model.

**Migration, green at every step:**
1. (M) Extract σ-parameterised free functions for the sites above, used by both structs. The corpus stays bit-identical.
2. (M) Write one outline walker over sections, in which each section emits its own end vertex; that alone fixes gear-outline#0 for both kinds. Prove it bit-identical to `tooth_outline` (apart from the fixed vertex), keeping `allocate_by_arc_length` so `a_concentric_gear_is_drawn_exactly_as_it_always_was` holds. Move Ring onto it and delete `Ring::outline` and `Ring::profile`. The ring DXF golden changes only by that vertex.
3. (L–XL) Write the shaper corner in κ = 1/r′_c form, so κ = 0 is the rack exactly. Start from T05.2's closed-form junction, whose limit is `s_tan`. The rack's undercut crossing and severing solves (tooth.rs:574-640) need shaper counterparts first. shaper.rs's convergence tests become identities at κ = 0, and the rack corpus holds to 1e-12. `equivalent_to_rack` then moves into tests (T17.8 [ring#12]).
4. (L) Merge Ring into the form type. `BuiltMember::{Rack, Ring}` and `auto::Cut::{ByRack, ByShaper}` lose their enums.

Eccentric rings are not free. Each of these needs a ring reading: root displacement, the one-tool rule for a Cutter, `admissible_angular_shift`, and `variation()`'s seats.

**Proof.** Steps 1 and 2 run the bit-identity tests above. Steps 3 and 4 run nextest, the corpus and check_ring_cut over T05.2's grid.

### Declined
None. [ring#5] is placed under T05.2: its symptom is real, but its mechanism and fix are T05.2's, not the ones it names.
