# Unified contact model: phase-exact prototype (the expensive mode)

Authors: contact-proto (2026-09-26/29), contact-proto-2, -3, -4 (rounds two to four), contact-proto-5
(2026-09-29, round five, this revision; its matched-wheel part by contact-proto-5-worm). Read-only on the
repo; pure Python, no numpy.
- **Scripts and outputs:** `~/.cache/gearcalc-work/contact-proto/` (README lists them). Round four's text
  is kept there as `contact-model.round4.md`, and rounds two and three likewise.
- **Built on:** `work/plan.md` §5 (2026-10-01: two modes; the matched wheel implemented and not exposed;
  the expensive mode refuses an unset r_e) and `review/contact-verify5.md`.

**Status: DRAFT — being filled as the runs finish.** Every process was capped at 1.8 GB, pools ≤ 4.

**Verdict of this round.**
1. (friction) TBD
2. (report) TBD
3. (face ends) TBD
4. (tooth) TBD
5. (matched wheel) TBD
6. `trace.py`'s docstring now says 2.77e-10 mm (the slip was 2.8e-13).
7. (ISO) TBD

## 1. What changed in the model (`trace.py`, `coupled.py`)

### 1.1 Friction is integrated over each panel
- Round four took Coulomb's direction at each station's centre. A station crossing the line where sliding
  vanishes flipped its whole panel's friction, and the state jumped: D by 0.1–0.2 % about 22 times per
  pitch (contact-verify5 §1).
- The sliding velocity is affine in position (v₁ − v₂ = Ω × X + const, Ω = a₁ − (ω₂/ω₁) a₂). Along a
  panel's chord its tangential part is A + B u, u ∈ [−½, ½], and q is constant on the panel. So the
  panel's friction is −μ q ē with ē the **mean** of the unit direction over the panel, in closed form
  (`panel_slide`). With w = u − u₀ (u₀ the foot of the origin on the line) and h the line's offset:
  - ∫ (B w + h)/R dw = B R/|B|² + h asinh(|B| w/|h|)/|B|, R = √(|B|²w² + |h|²);
  - when A ∥ B (every Σ = 0 mesh) h = 0 and this is the exact integral of sgn(w): the panel is split
    where sliding is zero; when Σ ≠ 0 the direction rotates smoothly and the formula is its exact mean;
  - the differences over the panel are written without cancellation (w₂ − w₁ = 1), so a panel far from
    the zero loses nothing; the mean sliding speed (the loss) has the same form.
- The slices seed (the span's D₀) splits its Gauss rule at the same zero, so the span is continuous too.
- Coulomb's own discontinuity stays: the traction's direction still reverses inside the panel. Only the
  integrated panel force is continuous. `friction='point'` restores round four.
- What does not become continuous: **a spur line** (β = 0, Σ = 0) is parallel to the pitch line, so the
  whole line's friction reverses at one phase. That is the exact Coulomb force of the line, not a
  discretisation (§2.1).

### 1.2 The report is continuous
- `pflank` and `pedge` are now the maxima of the pressure over two **regions**: the flank, and the edge
  (a round and the tip land beyond it, of either member). The regions are fixed by the forms' tangency
  lines, and the pressure is continuous for a C¹ form (round four §2.2). A maximum of a continuous field
  over a closed region whose boundary moves continuously is continuous. The tangency belongs to both.
- Round four classed each station by where its peak sat, so a whole station's peak switched class:
  pflank 427 → 1513 MPa between adjacent phases (c10), and 1510 → 2770 (s0).
- `Fedge` is the load carried on the edge region: each strip's pressure integrated over its edge part
  (closed form for Hertz; Gauss between the section's curvature steps otherwise).
- `L` is the load-weighted length (∫q)²/∫q²: the length itself for a uniform line, and a line entering
  at q → 0 adds nothing. Round four counted loaded panels, and a spur line's rigid overlap is all or
  nothing (L jumped 20 → 10 mm on s0), which reached the seed through the nominal line load F/L.
- `at` (where the maximum sits) is information: a location jumps where two local maxima trade places;
  the value does not.

### 1.3 The foundation reads the gear's own rim (`coupled.rim_of`)
- The derived foundation is referenced at the depth of the gear's own rim, H = s_R, for either kind (the
  bore of an external gear, a ring's outer rim): the crate's `MemberGear::rim_thickness`.
- Unset, s_R is the thickness ISO 6336-3:2019 §9.3 first calls thick: 1.2 h_t for an external gear,
  3.5 m_n for a ring. That is also what the crate rates an unset rim at (Y_B = 1), so one field is read
  one way for bending and for stiffness. An external body is at most solid (s_R ≤ r_f: a worm is a shaft).
- Round four's hidden defaults were r_f/r_int = 1.4 for an external gear and 3.5 m_n for a ring.

### 1.4 An instrument, not a model: `image_scale`
- A factor ψ on each body's mirror image (default 1, the model). Used only to bracket the free end (§2.3).

## 2. This round's items

(TBD: being filled as runs finish.)

## 5. Stage 3: the Rust migration plan

The field model lands as the **expensive mode**, beside today's closed forms, which stay the **fast mode**.
It is additive: no fast-mode number moves, and every step lands under the identity harness (`gear-cli
identity`) with the fast mode bit-identical. It runs as plan §3's research track S-C, after **G** (one
generator: the form circle and root the field clips at) and beside **R**, **L** and **K**.

### 5.1 Two modes, one wiring

| | Fast mode (default) | Expensive mode (on demand) |
|---|---|---|
| What | today's closed forms at ISO's points: `contact.rs` (line path, load share, efficiency), `screw.rs` (crossed path), `hertz.rs`, `strength::contact_stress` | the field: every pair's line, the coupled load, the exact 2-D across contact, friction per panel, maxima over phase |
| When | every solve, the search, the panel's live figures | once per mesh per load case, at the final solve, when asked |
| Cost | microseconds | 5–24 s per mesh per case (estimate, ±2×; §2.7 of round four) |
| Inputs | as today | as today, plus each gear's form (`tip_relief`, `edge_radius`, `end_relief`) and `rim_thickness` |
| Refuses | as today | a gear with no stated `edge_radius` (`field.edge_radius_unset`): the sharp edge has no finite value (r_e^−0.37) |
| Reports | `MeshReport` as today | `MeshReport.field: Option<FieldReport>`: `p_max` and where, `p_flank`, `p_edge` (region maxima), `edge_share`, `efficiency`, `length` (load-weighted), the ISO-point readings, and the forms and rims used |

- One request type, `Rating { Fast, Field }`, on the case the caller asks about; the train's solve is
  unchanged. `Field` runs `rate` first, then the field per mesh on the rated loads (`CaseLoad`), so the
  field sees exactly the torques the fast mode rated.
- The search never calls the field. A law holds the two modes to agree where both apply (§5.4).
- Boundary: one new wasm entry point `field_rate(train, case, mesh)` (JSON in and out, pure), run in a
  web worker; a CLI row `gear-cli field` with its output recorded in the corpus on one small case.

### 5.2 Modules and data types (all in `crates/gear-core/src/field/`)

| Step | Module | Types | What it is | Replaces / absorbs |
|---|---|---|---|---|
| F1 | `form.rs` | `ToothForm { tip_relief: Option<Relief>, edge_radius: Option<f64>, end_relief: Option<Relief> }`, `Relief { depth, length }` | the gear's tip and end form as one C¹ function of one coordinate along the flank normal (relief, the round tangent to flank and land, the land); refuses a form that is not C¹ | new inputs on `MemberGear` (principle 10); the fast mode ignores them |
| F2 | `gap.rs` | `FieldMember` (from `BuiltMember`: signed z, helix, flank closed form, form, circles, faces), `Gap`, `Section { d, dz }`, `Valley { nodes, ends }` | the exact rigid gap of two members with their forms; the family of sections X·d = kΔζ (one d per mesh); the intrinsic section minimum (a fixed point, ≤ 8 rounds, one bracketed root per round); the trace; the ends as roots of the field margin on valley points | nothing in the fast mode; the anchor search becomes a seed only |
| F3 | `across.rs` | `Strip { c, m, pressure }`, `StripReport { p_max, t_max, p_flank, p_edge, edge_load }` | the frictionless 2-D contact of a piecewise-quadratic section in closed form (I₀ = 0, q = (E*/2) c I₁, p by logarithms); Hertz when no step lies inside; the region maxima | `hertz::line_pressure` inside the field (the fast mode keeps it) |
| F4 | `kernel.rs` | `Kernel`, `Panel` | the lengthwise influence G(r) of a strip along a line (closed-form derivative through R_D, `elliptic.rs`), depth-referenced; panel integrals; each body's mirror image at its face crossings; the Bernstein-ellipse width rule | T08.11's proposed DC-FFT instrument (this is its job, exactly) |
| F5 | `compliance.rs` | `ToothCompliance` (Chebyshev in depth below the tip) | the derived tooth: potential-energy beam over the generated profile (from `tooth.rs`) + the half-plane foundation (L* = 18(1−ν²)/π, M* = 2(1−2ν)(1+ν), P*, Q* depth-referenced at H = `rim_thickness`, default the rim ISO 6336-3 first calls thick) | T21.13's tooth model (its `Compliance` load share reads this) |
| F6 | `solve.rs` | `Station`, `Line`, `State { approach, lines, torques, loss }` | the coupled lines: stations on the valley, K = tooth in series + kernel + images, active set, the approach in closed form per iterate, the strip-width fixed point; friction per panel by the exact integral of Coulomb's direction (`panel_slide`); the seed split at the zero of sliding | the fast mode's load share and efficiency inside the field (T06.5, T06.8 keep their fast-mode fixes) |
| F7 | `phase.rs` | `Piece`, `FieldReport` | breakpoints (signature scan, bisection), Gauss means per piece, maxima by golden section; ISO's points as phase roots | — |
| F8 | wiring | `Rating`, `MeshReport.field`, strings ×5, `field_rate`, `gear-cli field` | the two modes (§5.1) | — |
| F9 | `gap.rs` (optional flank) | `Flank::{Involute, Matched(Hob)}` | the matched (hobbed) worm wheel as a second flank kind, a closest-point projection; **implemented, not exposed** (plan §5, 2026-10-01) | — |

Each module is written once and serves every mesh: spur, helical, crossed, worm and ring are parameters
(signed z, Σ), not branches.

### 5.3 Order and gates (each step lands only when its laws fail at the base and pass at the head)

| Step | Laws that gate it (in the Rust suite unless marked) |
|---|---|
| F1 | C¹ at every joint (value and slope within 1e-12); curvature exactly 1/r_e on the round; the foot map exact; a C⁰ form refused |
| F2 | the gap equals `tools/contact_oracle.py` (the independent exact rigid mesh, ISO 21771 play and a₀) to 1e-12 mm; \|∇d\| = 1; seed independence: displacing the seed along and across the valley moves nothing beyond 1e-8 (the verifier's a2 law); the verifier's 2.77e-10 mm case agrees to 8 digits; every loop capped and counted |
| F3 | equals Hertz off a step to 1e-12; equals an independent discretised half-plane BEM (`tools/bem2d.py`, the verifier's) within 1e-5 on the same section and within 0.15 % on the exact C¹ circle (by hand); p finite and continuous for every C¹ form |
| F4 | the line limit; the Hertz ellipse reproduced at O(N⁻²); an aligned-end line with images equals the infinite line; the width rule exact to 1e-10 |
| F5 | c′ against the independent FE table (`tools/fe_tooth.py`, by hand) within the stated band over s_R (§2.4: −9.2 … +7.2 %); against ISO 6336-1 c′_th within 1 % at the default rim; the ring's rack tooth finite |
| F6 | equilibrium to 1e-12 (the torque balance); complementarity (q ≥ 0, gap ≥ 0 at every station); `panel_slide` against a brute-force integral; **continuity: every input (a, x₁, b, b₁, β, Σ, T) moved by ±1e-9 at the phases where the zero of sliding crosses a panel centre and a panel boundary changes D and p by ≤ 1e-6 relative, and by 1e-7 and 1e-5 proportionally** (the verifier's a3 law, §2.1); the verifier's h20 case to 1e-8; no refused span, no non-converged fixed point, no lost valley on the preset grid |
| F7 | **the dense-scan law: 1000 phases per pitch on h20, s0, c1, c10, no isolated jump above 2e-4 in D, p_max, p_flank, F or L** (the verifier's jump detector) — the one stated exception, the spur pitch-point reversal, asserted as a one-sided jump of its known size; analyse's maximum ≥ the dense grid's; means converged in the piece count |
| F8 | the fast mode bit-identical under the identity harness; the field's ISO-point reading within the documented table of ISO 6336-2 (§2.6); the Σ → 0 and β → 0 series smooth (§2.5 of round four as laws); `check_wasm.sh` probes `field_rate`; strings fire both ways |
| F9 | play j = s_h/(cos α_n cos γ) for ZI exactly; the rigid gap on conjugate lines ≤ 1e-12 mm; the projection never capped, never failed on the worm grid |

### 5.4 What it subsumes from the audit

- **As the reference its proofs need** (the task stays in the fast mode; its law reads the field):
  T06.5 (the load share conserves load: the field's coupled share is the reference), T06.8 (efficiency
  weighted by load per unit line length: the field's per-panel loss is the "discrete contact-line
  simulation" the task asks for), T06.9 (first order in μ: the field's balance is exact in μ), T06.10
  (the line length a pressure used: the field's load-weighted length), T08.3's σ_H comparison (§2.6),
  T08.11 (the finite-line-contact instrument: F4 is it), T21.9 (specific sliding from one expression:
  F6's panel sliding).
- **Replaced in the expensive mode**: T07.19 (one single-pair point: the field has none, it reads the
  maximum), T08.12 and T07.2/4/5/15's crossed face (the field's face is a clip on the valley, continuous
  in Σ — plan redesign **X**'s interval model is the fast mode's), strength#6 (Z_ε at B: the field reads
  the load it carries), T21.13 (compliance load sharing: F5 + F6), T21.11 (lead crowning: an end relief
  or a lead form on F1).
- **Spike S-C** (plan §3): closed by this plan — the unified model is the field; `Path`'s two
  implementations stay as the fast mode.
