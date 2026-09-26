## T08 — Strength ratings and their allowables

**Why.** The stress models are mostly sound. What is wrong is the ground around them: which allowable a stress is compared with, the domain on which a factor is defined, and the size of each bias against the standards. The material library has one fatigue number per material. It is a polished-specimen, fully reversed bending endurance, but it is used as the one-directional root allowable, then derated by 0.7 for reversal, and also used as the flank allowable. Nothing in the rating says what it has left out. Three numbers show the pattern. The default canary train runs at σ_H 2.1–2.6 GPa on 46 HRC 4340, which is 1.8–4× every published contact endurance for that steel, and nothing flags it [strength#1]. A light application factor K_A = 1.25 turns the canary's 8.53 mm minimum width into 10.66 mm, which fails its 10 mm face, while the code says leaving out the K factors "is conservative" [strength#3]. With a long-addendum mate, the default bending model reports −1.30 MPa and a negative minimum width, with no note [lens-numerical-robustness#0]. The documents call helical bending "26–36 % conservative" against ISO. Measured, the ratio is 0.90–1.27× at full overlap and falls to 0.53× at low overlap [strength#2].

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T08.1 Say the stresses are nominal; offer K_A | strength#3 | high | M | — |
| T08.2 Give the bending factor a domain | lens-numerical-robustness#0, lens-numerical-robustness#1 | high | M | — |
| T08.3 Generate the bias record against ISO | strength#2, strength#6, added#22 | high | M | — |
| T08.4 Make a fatigue value state its load ratio | strength#0, lens-feature-gaps#2, ablate-constants-rating#0, strength#15 | medium | M | — |
| T08.5 Correct the library's data and header | gear-io#14, added#36, strength#16, added#22 | medium | S | T08.4 |
| T08.6 Add a contact allowable | strength#1, lens-feature-gaps#1 | high | L | T08.4 |
| T08.7 Sign the axial term; record the HPSTC bias | added#32, added#27 | low | S | T08.2 |
| T08.8 Use one `BendingModel` enum and split out the ISO instrument | strength#11 | medium | M | T08.7 |
| T08.9 Pin K_f's coefficients and give them a band | strength#4, ablate-constants-rating#4 | low | S | — |
| T08.10 Check the ISO instrument against a published example | ablate-constants-rating#5 | low | M | T08.8 |
| T08.11 Make Hertz return one `Patch` with a smooth envelope | strength#8, added2#76, strength#7 | medium | M | — |
| T08.12 Make crossed-pair widths continuous as Σ → 0 | strength#8 | medium | M | T08.11 |

### T08.1 Say the stresses are nominal; offer K_A
**Change.**
- Rewrite strength.rs:856-873. K_A, K_v, K_Fβ and K_Fα are ≥ 1 by definition. Leaving them out makes σ_F nominal, which is unconservative by their product, and σ_H by its square root. Z_ε and Z_β are contact factors, not bending ones. Keep the locality argument for K_f. Point the link at `rationale.md#no-isoagma-correction-factors`: `reference.md#contact-stress` has no exclusion list.
- Add a Known-approximate entry in state.md: "All stresses are nominal. The torque entered must already be the design torque. The allowables are not ISO σ_Flim." The last sentence matters: the entry states the stress's bias, not the whole margin's.
- Add a train-level note under one catalogue key, in all 5 catalogues.
- Add an explicit application factor to each load case (`LoadCase`, default 1). It is a designer's input, not a fitted rule. Follow CLAUDE.md's "A load-case input" row: wasm `defaults`, the gear-io change log, strings ×5, TrainPanel.

**Proof.**
- Law over every preset: σ_F ∝ K_A and σ_H ∝ √K_A exactly. The canary already scales ×2.000 and ×1.4142 when the torque doubles.
- K_A = 1 reproduces the golden corpus bit for bit.
- The canary at K_A = 1.25 reports b_min 10.66 mm.

### T08.2 Give the bending factor a domain
The default bending model, Dolan–Broghamer's (Y_F − axial)·K_f, is used outside the load positions where it means anything, and it returns numbers there anyway. Two instances:
- At ε_αn ≥ 2 the unshared load point d = ε − 1 sits low on the flank. The moment arm → 0, K_f blows up, and Y_F falls below the axial term.
- On a pointed tip, the sharing sweep samples the apex. There Y_F ∝ 1/d, and at d = 0 it is 0/0.

**Change.**
- `Bending::new` (train/mod.rs:767) fires the out-of-band note whenever ε_αn ≥ 2, whatever the sharing model. Today it is gated on sharing being on.
- `bending_factor(DolanBroghamer)` returns `None` when the factor is ≤ 0 or the moment arm is ≤ 0. `stress_correction`'s contract already allows `None`.
- Where the section itself vanishes, report `None` for that gear with a note. Today the whole train returns `NoRootSection`: pair([300,300]) with gear 1 at h_a ≥ 2.1 raises no clamp and is still refused whole.
- On a tooth with `clamp.tip_capped_pointed`, the sharing sweep (strength.rs:1488) starts where the top land is finite. Alternatively, rate that tooth on the fillet tangency alone and say so in a note.
- For hygiene, make `worst_over_cycle` (strength.rs:1516, the `is_none_or` latch) and the Lewis `reduce` (strength.rs:746) skip non-finite values and compare with `total_cmp`.
- Do not adopt `max(Y_F ± a)`. It is a third model that neither source states. Do not bracket the flank below the vertex: that swaps the NaN for a number that grows with the sample count.

**Proof.** Write these first and see them fail.
- Fixture: pair([300,300]), gear 1 at h_a 1.9 and h_f 2.15, sharing none. Today it gives −1.30 MPa and −0.0095 mm.
- Fixture: pair([12,40]), sharp tips, h_a 1.6, 25°, x₀ 0.5, LinearRamp. Today it gives `Some(NaN)`.
- Fixture: pair([20,80]), h_a 1.4, 28°, x₀ 1.0. Today it gives 4.29e15 MPa.
- Law over every preset and arrangement, with the addendum swept 1.0–1.8 on one member: every bending stress is `None` or finite and > 0, and so is every `min_face_width.bending`. The fuzz found 36 NaN, 10 negative and 2 −inf results in 20,000 trains.
- Law on pointed teeth below ε 2: shared/unshared ≤ 1.002, and the result converges as `SHARING_SAMPLES` rises. Today the worst value goes 17.4 → 208.6 from 200 to 3200 samples.

**Notes.** Under LinearRamp, gear 1 of the h_a sweep jumps from 28 to 114 MPa between ε 2.66 and 2.84. This is unverified; check it with this law. The note's quoted figure is T06.7 [gear-io#0, added2#109].

### T08.3 Generate the bias record against ISO
**Change.**
- Extend tools/iso_6336_3_stack.py with ISO 6336-3 Method B for Y_F·Y_S. It covers E, G, H, the θ iteration, s_Fn, ρ_F, d_en, α_Fen and h_Fe.
- Read the tool's (Y_F − axial)·K_f from `gear-cli` and print the whole-model ratio as spur base × helix pair.
- Add ISO 6336-2's σ_H with Z_B, Z_ε and Z_β for the golden pairs.
- Replace the state.md:649 bullet, reference.md:1293-1297, rationale.md:672-700, state.md:861 and the script's printed conclusion with measured bands, each tagged `figures:` for check_figures:
  - spur bending at HPSTC: 0.757–1.039 (14.5°), 0.615–1.011 (20°), 0.477–0.957 (25°), tool/ISO;
  - helical: 0.90–1.27 at ε_β ≥ 1, 0.59–0.98 at ε_β 0.3, 0.53–0.93 at ε_β 0.1;
  - contact: +12.3 % on the canary spur pinion (692.7 against 616.8 MPa, 1/Z_ε), and +32.7 % on the β = 20° full-overlap helical (630.6 against 475.2);
  - the existing `gear-cli matrix` ratio (0.510–1.128, mean 0.827), moved into Known-approximate with its sign.
- Do not make the helical contact point depend on ε_β. That would couple σ_H to b and break `min_face_width_contact`'s closed form.

**Proof.** Before printing any ratio, the script asserts that it reproduces the tool's ISO set on 17/43 (Y_F 1.712382, Y_S 1.763254; agreement 7.3e-10). check_figures then gates the bands. That gate matches a bag of numbers, a weakness T16.1 fixes [tools-ci#1].

### T08.4 Make a fatigue value state its load ratio
The library's fatigue figures are fully reversed endurances (R = −1): POM's D671 figure explicitly, and the steels and brass through the rotating-beam ratio. The polyamides state no ratio. The code, UI strings and rationale call the same figure "the one-directional allowable". rationale.md:1502 says the library "has no such column", and it has one. `REVERSED_BENDING_FRACTION`'s doc (material.rs:158-162) calls 0.7 material-independent and a doubling of the stress range; it is neither. Under Goodman the fraction is (1 + S_e/S_u)/2, which is 0.65–0.75 across the library.

**Change.**
- Give each fatigue value a load ratio (−1 or 0) and a specimen kind (polished specimen or gear root). Put them in `Material` and the TOML reader. A user library that omits them gets a default plus a note, or a schema version, so that no existing file breaks.
- `Reversal::bending_allowable` (train/mod.rs:2638-2645) returns the stored value for R = −1 data, and 0.7 × the value only for R = 0 gear data, where ISO's convention belongs.
- A one-directional root on R = −1 data is rated at the stored value, and the bias goes into state.md.
- Do **not** derive an R = 0 allowable by Goodman from these polished figures. It would put 4340H near 1000 MPa, which is 1.27–1.68× ISO 6336-5's σ_FE band and 2.5–3.3× AGMA 2001's s_at. The general-R Goodman form σ_max = 2S_e/((1−R) + (1+R)S_e/S_u) replaces the binary switch only once a stored figure is a root allowable. It also needs a UTS value: `ultimate_allowable` holds yield for the steels.
- Delete `reversed_bending_allowable` and multiply at the one call site. That function labels an estimate as `Derived` and writes English in gear-core. Set `Material::overridden`'s note to `None`.
- Reword material.rs:150-185, strings_*.toml:126-127 ×5, rationale.md:1472-1505 and state.md:1062-1066.
- Add a Known-approximate entry giving each steel's net bias against ISO 6336-5, with sign. 4340H's 750 MPa sits inside the σ_FE band (513–790). Annealed 4340's 330 MPa is 1.2–1.9× below it.

**Proof.**
- Law over the library: a reversed root's allowable on R = −1 data equals the stored value bit for bit. Today it is 0.7×, so the law fails. Today only `train toggles` in the corpus catches a change to 0.7; the suite is silent.
- Test: no derived allowable reports a basis stronger than its input's.
- Re-record `gear-cli train toggles`.

**Notes.** Which gears reverse is decided by a topological proxy, which T10.13 fixes [shape-b#4]. What life a fatigue figure holds at is T21.6 [lens-standards#4].

### T08.5 Correct the library's data and header
**Change** (crates/gear-io/data/materials_default.toml, docs/state.md):
- Set 4340 Hardened's fatigue allowable to 700 MPa, basis `estimated`. The note should read "Shigley S_e′ = 0.5 S_ut, capped at 700 MPa above S_ut 1400 MPa". The shipped 750 is +7.1 % over the rule its own note cites. The note should also state that the figure is an R = −1 polished-specimen value with no surface, size or reliability reduction, and that 750 is 1.9–2.5× AGMA 2001's through-hardened s_at of 301–394 MPa at 400 HB [added#22].
- Header line 26: only annealed 4340's fatigue figure is published.
- Lines 30-36 list all the conventions: 0.5 × UTS, brass 0.30 × UTS, polyamide 0.30 × ultimate, and the Poisson rule. State that "ultimate" means yield where a yield point exists and break where it does not, so the estimates are comparable only within each group.
- In state.md:745, drop or size the "weakest number" line. The polyamide fatigue figures are weaker.

**Proof.**
- New materials.rs test: ν = 0.39 − 0.001·%glass for every glass-filled grade. Only the fatigue rule is tested today.
- Re-record the corpus. Every 4340H figure moves by 750/700.

### T08.6 Add a contact allowable
`allowable(material, Fatigue)` (train/mod.rs:2718-2723) hands the bending endurance to contact too. So train/mod.rs:2497 and rationale.md:1268 ("derived for flanks") are false for all eight entries. `Overrides` cannot express a flank limit apart from a root limit. Overriding the fatigue figure to 1088 MPa moves the bending width (0.8953 → 0.6172 mm) along with the contact width. Contact really does govern: a published flank figure still asks 2.96–10.99 mm on the canary, against bending's 0.89 mm.

**Change.**
- Add `contact_fatigue_allowable: Option<Value>` to `Material`, the TOML and `Overrides`. Key the lookup by rating and kind: `allowable(material, Rating::{Bending, Contact}, kind)`.
- Steels: estimate from hardness with ISO 6336-5 Table 1, through-hardened alloy steel: ML 1.313HV+188, MQ 1.313HV+373, ME 2.213HV+260. The quality grade is an explicit, user-visible option. The note says that 46 HRC lies past the table's 360/390 HV cap.
- Polymers: `None` unless a VDI 2736 figure can be quoted. The rating is then Unavailable, with a note.
- Ultimate contact: replace Hertz against tensile yield with first subsurface yield, p_Y = C(κ)·σ_y. C is 1.79 for line contact and 1.60 for circular contact (von Mises, ν = 0.3), and C(κ) comes in closed form from the Hertz field using hertz's κ. It applies only where `ultimate_measure = Yield`, which gives that field a reader.
- Keep contact sizing off by default, and emit a note whenever σ_H exceeds the flank allowable at the chosen width. Record whichever default remains in state.md with its size.
- Fix rationale.md:1265-1275 and train/mod.rs:2489-2505. Record in Known-approximate the ~12 % offset between the tool's single-pair σ_H and the ISO σ_H that σ_Hlim is calibrated against (from T08.3).
- Add labels to the 5 catalogues. The default-off wording in reference.md is T18.7 [added2#75].

**Proof.**
- Law: overriding the contact column moves only `min_face_width.contact`, and overriding the bending column moves only `.bending`. Today it fails: the bending width moves 0.8953 → 0.6172.
- Law: with contact sizing on, σ_H ≤ the contact allowable in every case at the automatic width.
- Independent Python check that shares no code: C(0) = 1.79 and C(1) = 1.60 from the Hertz subsurface field.
- The default canary raises the new note.

### T08.7 Sign the axial term; record the HPSTC bias
**Change.**
- In `finish` (strength.rs:794), replace the load-angle line with `let load_angle = (load_dir[1] * load_dir[0].signum()).atan2(load_dir[0].abs());`. `axial_compression` then turns tensile, and so negative, past the flip. Document `RootSection::load_angle` and `axial_compression` as signed.
- Keep the unshared rating at the HPSTC, where AGMA 908 defines J. After signing, re-measure the bias and write it into state.md as unconservative. Before signing it is up to +1.43 % at 20° on z ≤ 12 undercut pinions and +10.45 % on extreme grids. Signing adds +1.17–2.91 % on z 7–9 with sharing on.
- Optionally offer "rate at the worst single-pair point" as a user-visible option.
- Correct rationale.md:765-771, state.md:735, and the `SHARING_SAMPLES` and `bending_section_shared` docs.
- Make the strength.rs:2008 test rate with the model `worst_over_cycle` selects by, and assert the one-sided law (shared ≥ unshared) [added#33].

**Proof.**
- New test that fails today: a z = 9, x = 0 tooth swept across the flip. `axial_compression` changes sign where `load_dir[1]` does, and the DB factor has no local maximum there. Today it shows a V: 0.00996 → 0.00001 → 0.00999.
- The fix leaves all 679 tests and the corpus unchanged, so this test is the only gate.

### T08.8 Use one `BendingModel` enum and split out the ISO instrument
**Change.**
- Replace `CriticalSection × RootStressModel` (6 combinations, 2 coherent) with `BendingModel { Savage, Iso6336, FormFactor(Construction) }`. Savage is the parabola + K_f + axial term. Iso6336 is the 30°/60° tangent + Y_S.
- The section builder takes the model and stores the one radius that model needs. `bending_stress(&RootSection)` stays self-contained.
- `bending_section_shared` takes the model, so the sweep maximises the factor it rates.
- Delete the on-flank junction read (strength.rs:796-805) and the three tests that exist only for the mixture.
- Migrate tests/bending.rs:216/357/459 to Savage; all 15 pass there. Migrate strength.rs:2490 and 2568 as well.
- Drop matrix study 5's parabola q_s rows.

**Proof.**
- No mixed model can be constructed, which the compiler enforces.
- The corpus is unchanged except those matrix rows.
- The continuity and rack-limit laws pass on the coherent sets.

**Notes.** Stale prose about the mixture is T18.7 and T18.8 [strength#9, strength#10].

### T08.9 Pin K_f's coefficients and give them a band
**Change.**
- Factor `dolan_broghamer_coefficients(α_n) -> (H, L, M)` out of `stress_correction` (strength.rs:1051-1058).
- Raise a per-gear note outside [14.5°, 25°], the range AGMA 908 tabulates. The input admits (0°, 90°), and past 30° the fillet sensitivity almost vanishes: K_f falls by 0.03 over ρ 0.05→0.1, against 0.240 over 0.05→0.3 at 20°. L crosses zero at 37.7° and H at 43.5°.
- Correct strength.rs:929-938 ("all three are inside it", "needs no band") and reference.md:1250. Add the band to state.md.

**Proof.**
- Test: the coefficients round to Dolan–Broghamer's published values, 0.22/0.20/0.40 at 14.5° and 0.18/0.15/0.45 at 20°. It fails under a digit swap such as 0.331 → 0.313.
- Sub-digit drift is held by the whole-factor gate at 1e-8, which is T16.13 [strength#5].
- Test: the note fires at 14.4° and 25.1° and not between.

### T08.10 Check the ISO instrument against a published example
**Change.**
- Add a test from ISO/TR 6336-30's worked examples: feed the printed s_Fn, h_Fe and ρ_F to the Iso6336 notch model, and assert Y_S to the printed digits.
- Add a property law: the tangent section has its fillet tangent at 30° or 60°.
- Do not test at a chosen point that recomputes the formula. It restates the constants, and the proposed q_s 2.5 reference gives 2.153, not "≈1.97".

**Proof.** The test fails under the recorded perturbations large enough to move a printed digit: Y_S constants 1.2/0.13/2.3, tangents 30/60, notch range 8. Today only `gear-cli matrix` catches them.

**Notes.** This is blocked until the TR text, or another published rated example, is in hand.

### T08.11 Make Hertz return one `Patch` with a smooth envelope
**Change.**
- Have `hertz` return `Patch { pressure, governs: Line { length, half_width } | Ellipse { semi_major, semi_minor } }` from one function. shape.rs:2569-2571 reads it instead of calling three functions, which runs the κ root-find twice.
- Replace `max(ellipse, line)` with p = p_e / √(1.5c − 0.5c³), c = min(1, L/(2a)): the ellipse, with its out-of-face load put back onto the face. It is C¹ at c = 1 and tends to the line as c → 0. Write it so that zero lengthwise curvature evaluates to the line exactly.
- Drop the dead `lengthwise_curvature` argument from `contact_stress`, which every production call passes as `PARALLEL_AXES`, together with its doc at strength.rs:1766-1771.
- Record in state.md that an unrelieved face's edge pressure is singular in both models.

**Proof.**
- Add tools/finite_line_contact.py: a DC-FFT half-space solver with the domain bounded by the face. Validate it against the untruncated ellipse (0.06 %) and compare interior pressure only.
- Law: the closed form ≥ the solver's interior over kx/kc 0–5. Today's max sits 5.9 % below the interior at the crossing, so it fails. The closed form should sit 0.1–3.8 % above.
- `the_general_form_is_bit_identical_to_line_contact_at_parallel_axes` still passes.

### T08.12 Make crossed-pair widths continuous as Σ → 0
**Change.**
- Rate a crossed pair whose `Patch` is line-governed as the line it is: bending included, and `sizes_face: true`. The objection in shape.rs:3141, a point load on a wide tooth, does not hold once the patch spans the face.
- Invert contact with b·(σ_line/σ_allow)² only while the face does not limit the zone. Otherwise solve for the width.
- If this is rejected, state the jump where the width is shown and in state.md.

**Proof.** Law: as Σ → 0, the crossed pair's bending and contact minimum widths tend to the parallel pair's.
- Today the default widths go 0.7719 → 10.0000 mm (the box) between Σ = 0 and 0.01°.
- With contact sizing on, they go 7.07 → 10.00 mm.

**Notes.** The point mesh's early-width read is T07.14 [shape-b#7], and the worm bending rationale is T18.27 [crossed-worm#7].

### Declined
None. No assigned finding was refuted. Where a proposal was corrected (Goodman uplift, `max(Y_F ± a)`, vertex bracketing, the [14.5°, 20°] band, the ε_β-dependent contact point), the task carries the correction.
