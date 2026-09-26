## T21 — Engineering practice: disclosed departures, and features the model reaches cheaply

**Why.** The tool departs from ISO/AGMA practice in several places. Each departure is defensible, but rule 6 requires a size and a sign for each one, and they are either missing or stated by direction only. Every planet mesh is rated at `mesh_torque / N` (K_γ = 1). AGMA 6123 puts K_γ between 1.0 and 1.61, yet the assumption appears only in a "declined" row that gives no size [lens-standards#3]. The path "Efficiency" counts only tooth friction. On the 17/43, m 2, 50 N·m, 1500 rpm canary the mesh loses 1.672 % (131 W), and load-dependent bearing loss alone adds 12.9 W (0.164 %), which no document mentions [lens-standards#5]. Five of the eight shipped materials are polymers quoted at 23 °C. There is no service-temperature input, and a Delrin allowable read at 10⁷ cycles is applied to a 1.44·10⁸-cycle case without a word [lens-standards#8, lens-standards#4]. Around these gaps sit several features the unified model reaches in a few lines, because it already holds every input they need: specific sliding (closed form from `curvature_radii`), crowning (`lengthwise_curvature`, fixed at 0 today), power lost in watts (the flow already has `(1−η)·P`), thickness deviation (`thickness_shift` already turns thickness into play), and SVG (the bulge outline). Beyond those are larger scopes that need a model or a source before they can be built.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T21.1 Disclose every departure with size and sign; record what is out of scope | lens-standards#3, lens-standards#5, lens-standards#8, lens-standards#9, lens-feature-gaps#8, lens-feature-gaps#9, lens-feature-gaps#11, lens-feature-gaps#12 | medium | M | — |
| T21.2 An explicit planet load-share factor K_γ per replicated axis | lens-standards#3 | medium | M | T21.1 |
| T21.3 Tooth-thickness deviations: one input for the play band and metrology | lens-feature-gaps#3 | medium | M | T09.10, T10.1 |
| T21.4 Power lost per mesh in watts, and the train's total | lens-feature-gaps#8 | low | S | — |
| T21.5 Per-body bearing loss as an explicit input | lens-standards#5 | medium | M | T21.4 |
| T21.6 A material's fatigue life, and a note when a case exceeds it | lens-standards#4 | low | M | — |
| T21.7 Polymers at service temperature, as library entries | lens-standards#8, lens-feature-gaps#8 | medium | M | T21.1 |
| T21.8 A load stated as power | lens-feature-gaps#16 | low | M | — |
| T21.9 Specific sliding, one expression for line and point contact | lens-feature-gaps#6 | low | S | — |
| T21.10 SVG export | lens-feature-gaps#12 | low | S | — |
| T21.11 Lead crowning through `lengthwise_curvature` | lens-feature-gaps#5 | low | S | — |
| T21.12 Blok flash temperature (scuffing) as an explicit check | lens-standards#9 | low | L | T21.9 |
| T21.13 Load sharing from tooth compliance, gated on ISO c′/c_γ | lens-feature-gaps#4 | low | L | — |
| T21.14 Resonance ratio for a lone pair | lens-feature-gaps#9 | info | M | T21.13 |
| T21.15 EHL film and λ readout | lens-feature-gaps#7 | info | M | T21.7, T21.9 |
| T21.16 Tooth-count synthesis for a target ratio | lens-feature-gaps#17 | info | L | — |
| T21.17 Mesh sign from signed axis directions (bevel kinematics) | lens-feature-gaps#11 | info | XL | T21.1 |

### T21.1 Disclose every departure with size and sign; record what is out of scope

**Change.** Documentation, plus one check that stops the same gap from reopening.
- `docs/state.md` Known-approximate. Add these entries:
  - **Equal planet load sharing**: unconservative. AGMA 6123 gives K_γ = 1.0–1.61 by planet count, application level and mounting, and more for N ≥ 4 without a floating member. Bending scales as K_γ and contact as √K_γ. Cite the standard's range. Do not cite the ±0.02 mm clearance calculation, which is illustrative only. Remove the "Decided, not pending" row, and rewrite `rationale.md:1608-1612`. K_γ is not in the refused list, it is not half of a calibrated pair, and leaving it out is unconservative, which is the test `rationale.md:608-612` itself uses to admit a factor. Give the `planets_share_load_equally` note the same size [lens-standards#3].
  - **Efficiency is tooth friction only**: bearings, seals, churning and windage (ISO/TR 14179's P_VZ0, P_VL, P_VD) are left out, so the figure is optimistic. Canary: mesh loss 1.672 %, bearing load-dependent loss alone 0.164 %. In a Wolfrom, planet-bearing loss is amplified like η₀. Relabel the path readout `train_efficiency` to "Tooth efficiency" ×5. It must not be "Mesh efficiency", because `strings_en.toml:476` already uses that label for the per-mesh figure [lens-standards#5].
  - **Polymers are rated at their conditioning state (23 °C / 50 % RH)**: unconservative for any gear that runs warm. The size must come from the CAMPUS multipoint curves the entries already cite, and stays marked as a debt until it does [lens-standards#8].
- `docs/state.md` Not built, each row with its reason:
  - scuffing (ISO/TS 6336-20/21) and micropitting (6336-22) [lens-standards#9];
  - a VDI 2736-2 tooth-temperature model: the source is paywalled, so it is not to be built from secondary sources [lens-feature-gaps#8];
  - dynamics and resonance, beside K_v: blocked on a stiffness model [lens-feature-gaps#9];
  - bevel, hypoid, face and non-circular gears: conical or new generation mathematics, and no constant ratio [lens-feature-gaps#11];
  - STEP and a 3D view [lens-feature-gaps#12].
  K_v stays declined where the rationale puts it.
- DXF: add one sentence to `reference.md#export-and-import` and a field note on the export button saying that the file is the transverse section and a helical gear's lead is not in it [lens-feature-gaps#12].
- Show `Material.condition` beside the material name on the card. [added2#77] owns this.
- A new check, `tools/check_known_approximate.py`, wired into CI and into CLAUDE.md's table. Every bullet under Known-approximate must name a sign (conservative, unconservative, optimistic or unknown). It must also carry a number or the word "debt". This removes the class, including the instances T08.1, T08.3, T08.5 and T18.23 fix [strength#3, added#22, gear-io#14, strength#16, added#17].

**Proof.** Run the check on the current tree before any edit. It must fail on at least one current entry (for example the entries [strength#16] names), and it must pass once the edits land. `tools/check_doc_links.py` and `tools/check_strings.py` stay green.

### T21.2 An explicit planet load-share factor K_γ per replicated axis

**Change.** Add `load_share_factor: f64` to a replicated axis in `train/shape.rs`, default 1, validated ≥ 1 through the shared input validation ([lens-numerical-robustness#5], T01). In `pressing_torque_at_a` (shape.rs:3288), rate the most-loaded instance at `K_γ · |mesh_torques[k]| / paths(k)`. The flow is untouched. Add a line in `inputs`, the field in `TrainPanel.svelte`, and one label ×5. An "estimate from AGMA 6123" fill is allowed only as a labelled population table, and only once the table is transcribed with its source.

**Proof.** A law over every preset with a replicated axis: at K_γ = k, every planet-mesh bending stress scales by k, contact scales by √k, and every flow total and path figure is bit-identical. At K_γ = 1 the golden corpus does not move.

### T21.3 Tooth-thickness deviations: one input for the play band and metrology

**Change.** Add per-member `thickness_deviation {upper, lower}` (mm, normal plane, default 0) on `MemberGear` / `params.rs`. Fold each deviation into `play_of` (shape.rs:3811) as the equivalent shift that `thickness_shift` already defines. Play is linear in thickness, so the band's extremes are exact at the corners: combine the thickness corners with each independent distance group's corners, using the grouping that the path-band fix [mesh-contact#0] / [train-mod-a#0] / [lens-feature-gaps#0] introduces. The same input also supplies min/max span and over-pins. T09.10 removes the unread `limits` fields, and this task brings them back fed by this input (shift ψ_b by A_sn/(m_n z) inside the existing closed forms). Replace the state.md "Not built" row: the designer types the deviation, and JGMA 1103 defaults can come later without changing the model.

**Proof.** Written first, it fails today because no such input exists. On a spur pair with zero distance tolerance, the normal backlash band widens by exactly (Δs_n1 + Δs_n2)·cos α_n. On helical pairs the transverse figure follows through cos α_wt and cos β_b. Span min/max moves by the closed-form Δs_n·cos α_n. At zero deviation the corpus is unchanged.

### T21.4 Power lost per mesh in watts, and the train's total

**Change.** Add `MeshCase::power_lost` (W) = (1−η)·|mesh power|, computed from the case's torque (N·m) and speed (rpm) in Rust (flow.rs:139-147, 330-333), plus a train total per case. Add a readout and one label ×5.

**Proof.** A law over every preset and case, circulating ones included: Σ mesh losses = P_in − P_out over the ports, to rounding. This figure inherits the path-weighting bias of [added2#72] (T11 owns it). The law holds either way.

### T21.5 Per-body bearing loss as an explicit input

**Change.** Add an optional per-body friction torque, stated either directly or as μ_b·F_r·d/2, entered in `flow.rs` as a per-body loss term beside the mesh losses. When it is set, the path efficiency becomes the train's efficiency and the T21.1 label can change back to "Efficiency". It is off by default.

**Proof.** At zero bearing input, every figure is bit-identical to today. The canary reproduces 12.9 W with the four-bearing figures (25 mm bore, μ = 0.0015). The T21.4 loss-balance law still holds with the bearing losses added.

### T21.6 A material's fatigue life, and a note when a case exceeds it

**Change.** Add an optional `fatigue_cycles: Value` (with a basis) to `Material` (material.rs:234) and to `materials_default.toml`: Delrin 10⁷, 4340 annealed "not stated", and so on. When a fatigue case's cycle count exceeds that life, `allowable()` (train/mod.rs:2718) keeps its value and the rating raises a note, for example `rating.cycles_beyond_source_life`, ×5. Do not add a limited-life curve. If one is ever added, it is off by default and restricted to materials that carry σ_FP,stat as ISO 6336-3 defines it, never the tensile yield.

**Proof.** `gear-cli train mixed` case 3 (1.440e8 cycles against Delrin's 10⁷) raises no note today, and must raise it on the Delrin member after the change. Law: the note fires if and only if cycles > `fatigue_cycles`, and every allowable is unchanged. The note depends on correct cycle counts, so a member loaded by two meshes [shape-b#5], and cases at held bodies [graph-ops#5], [edit-ops#5] and [train-mod-b#3], must be fixed by their owners first or the note under-fires.

### T21.7 Polymers at service temperature, as library entries

**Change.** Each condition is already its own library entry (header lines 13-15). Add elevated-temperature entries for the shipped polymers, taken from the CAMPUS multipoint data the entries already cite, for example "POM Delrin 100P, 80 °C", each with `condition` and `source`. The designer then rates at service temperature without a thermal model. Use the same data to put the size into T21.1's polymer entry.

**Proof.** Every new entry passes the library's existing "every non-datasheet value carries a note" test. Law: for each polymer, the modulus and both allowables do not increase from the 23 °C entry to the elevated one. `tools/check_figures.py` passes on the new state.md size.

### T21.8 A load stated as power

**Change.** A `Load` can state its magnitude as power (kW) instead of torque. `solve_train` resolves it after the motion as T = P/ω, where ω may itself be derived, and reports T as derived. It is refused with a note when ω = 0, including a derived zero such as a held port. `relieve_case` turns P for a power-stated load. Record that choice in `gear-io/src/train.rs`'s change log, and follow CLAUDE.md's load-case row (`gear-wasm` defaults, 5 × strings, `TrainPanel.svelte`, `check_wasm.sh --write`).

**Proof.** Law: a power-stated case solves identically to its torque-stated equivalent over every preset. The zero-speed refusal fires on a held port.

### T21.9 Specific sliding, one expression for line and point contact

**Change.** Define ζ_i = |v_slide| / |v_roll,i| through `contact::sliding_velocity`, so one expression serves both contacts. On a line contact it reduces to 1 − ω_jρ_j/(ω_iρ_i), using `Mesh::curvature_radii` with the signed ρ₂ and signed z₂ so that an internal mesh needs no branch. Report it at both ends of the path, on the contact report (`LineContact` / `BuiltContact`) rather than on the shared `MeshReport`, whose `sliding_ratio` is a different quantity. Add one label ×5. Optionally, add a balanced-sliding objective as a user-picked closure beside efficiency where shape.rs builds the `auto::maximise` closure. No `Objective` enum is needed.

**Proof.** On a 17/43, m 2, 20° standard pair: ζ₁(A) = −9.954, ζ₂(A) = 0.909, ζ₁(E) = 0.607, ζ₂(E) = −1.542, and ζ = 0 at the pitch point. Law: (1−ζ₁)(1−ζ₂) = 1 at every ξ, on external and internal meshes.

### T21.10 SVG export

**Change.** Add `gear_io::svg::{gear_to_svg, ring_to_svg}` beside `dxf.rs`. Each `Vertex {x, y, bulge}` becomes an arc with r = chord/(2 sin(θ/2)), θ = 4·atan(bulge), large-arc flag = |θ| > π, and sweep taken from the bulge's sign. SVG's y-axis points down, so flip y (or invert the sweep). Add an `export_svg` wasm entry with its probe in `tools/wasm_probe.mjs` and a button. The SVG reads the same outline as the DXF, so the outline defects [gear-io#1] and [gear-outline#8] (T04) are fixed once for both formats.

**Proof.** Read back with an independent parser (svgpathtools): arc endpoints match the outline to 1e-9 mm, and the area matches ezdxf's reading of the DXF to within the chord tolerance. Include a ring whose bore arcs would bow the wrong way if y were not flipped. `tools/check_wasm.sh --write`.

### T21.11 Lead crowning through `lengthwise_curvature`

**Change.** Add `crown_height` (mm, default 0) on `MemberGear`. Per mesh, pass the sum of the two members' 1/R, with the exact R = (b²/4 + C²)/(2C), in place of `PARALLEL_AXES` at shape.rs:3318 and :4513. No new branch is needed. The note must state two things: crowning only raises the reported contact stress, because the misalignment it tolerates is not modelled; and near the face ends the max(elliptical, line) rule under-reports. Bending's face-load distribution is not modelled. Add one label ×5.

**Proof.** At C = 0 the corpus is bit-identical. Contact stress is monotone in C (the law of `stress_rises_monotonically_with_lengthwise_curvature_and_returns_to_the_line`). It matches a hand Hertz point-contact calculation at a crown large enough that the ellipse sits inside the face.

### T21.12 Blok flash temperature (scuffing) as an explicit check

**Change.** Compute Blok's flash temperature along the existing path of contact from the sliding speed (T21.9), the Hertz half-width and the load share at each point. μ and the thermal contact coefficients are explicit inputs. The check is off unless asked for, and it changes no rating. Move the T21.1 "Not built" row accordingly.

**Proof.** Against a published ISO/TS 6336-20 worked example, evaluated independently in `tools/`. Law: the flash temperature is zero at the pitch point.

### T21.13 Load sharing from tooth compliance, gated on ISO c′/c_γ

**Change.** Add a third `LoadSharing` value, `Compliance`, beside `None` and `LinearRamp` (contact.rs:937-948). It needs:
- the tooth's bending, shear and compression energy integrals over the generated profile, plus a closed-form foundation term (Weber);
- the Hertz term;
- the per-position compatibility solve Σ F_i = F with F_i/k_i equal.

`load_share` widens to take the pair. Transmission error and tip relief are a separate later step. The existing ramp defects are T06.5, T06.6 and T06.7 [ablate-constants-geometry#3, added2#109, gear-io#0, added#33, strength#12]. The payoff for today's ratings is confined to ε ≥ 2, where the ramp's roughly one-third relief is already flagged, so ordinary bending moves by 0.0–0.2 %.

**Proof.** Gate before exposing it: standard steel spur teeth give c′ ≈ 14 and c_γ ≈ 20 N/(mm·µm) (ISO 6336-1), and match Cornell's compliance curves. Law: the shares sum to 1 at every ξ.

### T21.14 Resonance ratio for a lone pair

**Change.** For a lone pair only, report N = n/n_E1 with n_E1 = (30000/(π z₁))·√(c_γ/m_red). c_γ comes from T21.13 or a labelled input. Body inertias are explicit inputs, not density × rim, because the web, hub and shaft are unknown. Add a note stating ISO's main resonance band with its load-dependent lower bound N_S. Add no stress factor.

**Proof.** A hand ISO 6336-1 n_E1 example. Law: N scales as n·z₁·√(m_red/c_γ), checked by doubling each input.

### T21.15 EHL film and λ readout

**Change.** An opt-in mesh lubricant (η₀ at a stated temperature, α) and a per-member Ra. Report h_min at the pitch point and at the single-pair points, and λ = h_min/√(Ra₁² + Ra₂²). Name the fit in the note (Dowson–Higginson for line contact, Hamrock–Dowson for point contact), and choose the regime from the elasticity parameter, or refuse polymers with a note. It changes no rating. The line/point seam is recorded as a known discontinuity between two fits, not held to a continuity law.

**Proof.** Hamrock–Dowson's published worked example, evaluated independently in `tools/`.

### T21.16 Tooth-count synthesis for a target ratio

**Change.** Add a harness command first (`gear-cli synth R tol stages`, one `COMMANDS` row). Enumerate the admissible count sets by Stern–Brocot over `ratio.rs`. For epicyclic paths, filter through `Shape::assembly` and concentric closure. Solve each candidate and return the Pareto set over |ratio error|, efficiency and a size objective named as a user-visible choice. An offer at a path comes later. No existing edit sizes a mesh to a ratio (`AddRatio` copies the first mesh). [lens-feature-gaps#10] (the hidden per-tooth sensitivity) is the related figure.

**Proof.** Classical two-stage exact-ratio tables. Law: every returned train's path ratio is within tol, and no returned set is dominated. `tools/check_golden.sh --write`.

### T21.17 Mesh sign from signed axis directions (bevel kinematics)

**Change.** Give each axis a signed direction and derive a mesh's sign from the two directions and the apex side. `MeshKind` remains the parallel special case, so the sign stays unrepresentable as a free field (wiring.rs:40-47). Do not store a per-mesh sign. This alone gives a bevel differential's motion and flow with a user-given η. Conical geometry rated through ISO 10300's virtual cylindrical gear is a further XL step. [lens-architecture#8] counts the 37 kind dispatches in shape.rs that this change would touch.

**Proof.** A bevel differential satisfies ω₁ + ω₂ = 2ω_c. `tools/train_kinematics.py` is extended with 3D rigid-body velocities. Every existing preset's motion is bit-identical.
