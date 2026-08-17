# Gear & geartrain design tool — architecture and mathematics

**Status: milestones 0–4 complete and in CI; milestone 5 (strength) partly
built.** This document is the design of record and is current as of the head of
`main`. Where implementation contradicted the design, the design was corrected
and the correction recorded — see §12.

Conventions: angles in **degrees at the UI boundary, radians everywhere
inside**; lengths in mm; `m` is the **normal** module unless subscripted `m_t`;
subscript `n` = normal plane, `t` = transverse.

## What exists

| | Built | Verified by |
|---|---|---|
| Geometry core | involute + trochoid profile, undercut, severed teeth | rack simulation, both bounds, 1080 cases |
| Primitives | safeguarded `inv⁻¹`, Brent, bracketed Newton | textbook special cases |
| Mesh | centre distance, exact backlash, contact path | direct tooth-thickness computation |
| Metrology | span, over-pins, cutter tip width, JGMA tables | independent pin-tangency check |
| Strength | critical section, form factor, bending stress, Hertz, face width, helical | closed-form rack limits; the contact-half-width route; plane-change identities |
| Efficiency | parallel-axis mesh loss from sliding along the path | numerical average of the instantaneous loss |
| Materials | eight-material library, per-value provenance, TOML round-trip | primary datasheets; cross-family consistency laws |
| Export | DXF with exact arcs, chord-tolerance sampling | `ezdxf`, an unrelated parser |
| UI | gear tabs, parameter grid, viewport, DXF download | end-to-end through the real wasm |

133 tests, ~26 s. `nix flake check` covers build, clippy `--deny warnings`, fmt
and tests; CI additionally typechecks the front end and re-reads an exported DXF
with `ezdxf`.

```bash
nix develop                       # or `direnv allow` once
cargo nextest run                 # the suite
cargo run --bin gear-cli -- show 17 0.2      # drive the maths, no browser
cargo run --bin gear-cli -- materials        # the library, with each value's basis
cargo run --bin gear-cli -- strength 17 43 2.0        # a worked mesh, end to end
cargo run --bin gear-cli -- strength 17 43 2.0 '4340 Hardened Steel' 20  # helical
cargo run --release --bin gear-cli -- bending   # the bending verification sheet
cd web && npm run dev             # the application
```

## What is next

Milestone 6 (spur stage) is the open work; §11 has the full list. Nothing is
blocked. The mathematics of a single mesh is now complete — what milestone 6
adds is the *stage*: face width and material as inputs, torque and cycle
propagation, and the UI that surfaces all of it. Two deferred decisions have
written rationales: load sharing (§4.7) and the gear-rating standards (§6.2).

---

## 1. Stack — and the GUI decision

You raised the possibility of a TypeScript/Svelte front end, with `rof-gui` as
the model, and asked whether egui is still worth doing first. I looked at
`rof-gui`: it is **pure Svelte 5 + TypeScript + Vite + Nix**, no Rust at all (the
"WebAssembly" in its README is ffmpeg.wasm, not their code). It builds through a
flake and deploys to GitHub Pages.

**Recommendation: skip egui. Go straight to a Rust core compiled to wasm with a
Svelte/TypeScript UI.**

The reason is that the §3 architecture makes the UI boundary almost nothing.
Because outputs are a pure function of inputs, the entire wasm surface is three
functions:

```rust
#[wasm_bindgen] pub fn solve_gear (input_json: &str) -> String;
#[wasm_bindgen] pub fn solve_train(input_json: &str) -> String;
#[wasm_bindgen] pub fn export_dxf (input_json: &str) -> String;
```

That is the whole API. `serde` is already needed for the library and geartrain
import/export, so the JSON serialisation is free. There is no state to
synchronise across the boundary, no lifecycle, no callbacks.

Given that, the trade-off is not close:

- **egui-then-Svelte means writing the UI twice** and throwing one away.
- The argument for egui was native debugging. That argument
  **does not survive this architecture**: `gear-core` is a plain Rust library, so
  every hard piece — geometry, solvers, stress, the rack-simulation test suite —
  is debuggable natively through `cargo nextest` and a small CLI, whichever UI
  sits on top. The debugging benefit was never really about egui.
- This spec is form-heavy: several hundred labelled numeric fields in deep nested
  groups, tabs, and an accordion. HTML and CSS are simply better at dense forms
  than immediate-mode layout, where nesting this deep means manual sizing work.
- `rof-gui` gives a proven template for the infrastructure — Vite, Svelte 5 runes,
  the flake, the Pages deploy — so that part is lifting, not designing.

**Your "written in Rust" requirement survives essentially intact.** All
mathematics, all geometry, all solvers, the DXF writer and all serialisation stay
in Rust. TypeScript does layout and event handling. I would make that a hard
project rule:

> **No engineering calculation in TypeScript, ever.** If a number appears in the
> UI, Rust computed it. TypeScript may format it for display and nothing else.

That rule is what keeps the Rust test suite meaningful — otherwise logic quietly
migrates into the view layer where nothing tests it.

One simplification versus rof-gui: it runs analysis in a Web Worker because DSP
is slow. A full geartrain solve is microseconds, so ours runs on the main thread,
recomputing on input change via Svelte's `$derived`. No worker, no async, no
loading states.

| Layer | Choice |
|---|---|
| Toolchain / build | `schlarpc/rust-flake`, extended with a wasm target and a `.#web` output |
| Core | Rust, `gear-core`, dependencies: `serde` only |
| Boundary | `wasm-bindgen` + `serde-wasm-bindgen` |
| UI | Svelte 5 + TypeScript + Vite (the rof-gui stack) |
| Viewport | Canvas 2D — the core emits polylines, TS strokes them |
| Data formats | TOML (materials, geartrains), DXF (profiles) |
| Numerics | hand-rolled (~120 lines of root finding) |

Note the flake gotcha, from rust-flake's own docs: flakes only see git-tracked
files, so new files must be `git add`-ed before `nix build` or `nix develop`
sees them.

---

## 2. Repository layout

```
gears/
├── flake.nix, rust-toolchain.toml, Cargo.toml     (workspace root)
├── crates/
│   ├── gear-core/       pure mathematics. No I/O, no wasm, no UI.
│   │   ├── involute.rs      inv, safeguarded inv⁻¹, root finders
│   │   ├── rack.rs          basic rack, thickness modification
│   │   ├── profile.rs       generated 2D profile (port of gear.py)
│   │   ├── ring.rs          internal-gear profile
│   │   ├── screw.rs         crossed-axis screw gearing (worm + crossed helical)
│   │   ├── mesh.rs          centre distance, operating angle, backlash
│   │   ├── metrology.rs     span, over-pins, tip width, JGMA lookup
│   │   ├── contact.rs       path of contact, load sharing, mesh efficiency
│   │   ├── strength.rs      load, bending, Hertz, face width
│   │   ├── material.rs      material model — values, provenance, allowables
│   │   ├── train/           spur.rs, worm.rs, planetary.rs, accumulate.rs
│   │   └── data/            jgma_116_02.csv
│   ├── gear-io/         DXF writer, TOML (de)serialisation
│   │   └── data/            materials_default.toml   (TOML is I/O, so it lives here)
│   ├── gear-wasm/       the three #[wasm_bindgen] entry points. Thin.
│   └── gear-cli/        dev-only harness: solve a file, dump numbers, sweep
├── web/                 Svelte + TS + Vite front end
├── docs/                DESIGN.md, JGMA 116-02 1983.pdf
└── handoff_inbound/     prior work, reference only
```

`gear-cli` is worth calling out: it is how you drive the mathematics during
development without touching the browser. Sweeps, regression dumps and
"why is this number wrong" all happen there.

---

## 3. State model

### 3.1 Inputs are state; everything else is a pure function

```rust
fn solve_gear (input: &GearInput,  lib: &MaterialLibrary) -> GearResults;
fn solve_train(input: &TrainInput, lib: &MaterialLibrary) -> TrainResults;
```

Outputs are **never stored** — they are recomputed whenever an input changes. A
full solve is microseconds. In exchange: no cache invalidation, no dependency
graph, no field-updates-field wiring, and outputs that cannot disagree with
inputs. Every test is `assert!(solve(x) ≈ expected)` with no setup.

This matters because the spec has long derived chains (profile shift → centre
distance → backlash → train backlash) and several bidirectional couplings.
Pushing updates between fields is where this kind of application normally rots.

### 3.2 Shared-within-a-stage fields are stage fields

Where the spec says *"the same input box for all gears in a stage, changing one
updates the other"* (normal module, pressure angle), **store one copy on the
stage** and render it in both gear panels. The sync bug becomes unwritable.

Same for tooth thickness modification, where `k₁ + k₂ = 2` is required: store
only `k₁` and derive `k₂ = 2 − k₁`. Two boxes, one number, invariant by
construction.

### 3.3 The automatic toggles

About a dozen fields are *value + automatic toggle, locked while automatic*. One
generic covers all: `struct Auto<T> { auto: bool, manual: T }`. The solver
returns the effective value; the UI shows it greyed when automatic. Turning
automatic off seeds `manual` with the last solved value so the field never jumps.

### 3.4 Validation

Ranges are declared as data next to the field, not scattered through the UI.
Out-of-range entry shows in red with the reason and the solver keeps the last
valid value, so a half-typed `-` or `1e` never reaches the maths.

---

## 4. The mathematics

Closed-form unless listed in §5. **[verified]** marks formulas checked
numerically against an independent computation before being written down; the
checks are in the appendix.

### 4.0 Primitives

`inv α = tan α − α`.

**Involute inversion** `α = inv⁻¹(v)` — needed by centre distance, over-pins and
backlash. No closed form. Series seed from `tan α − α = α³/3 + 2α⁵/15 + …`
gives `α₀ ≈ (3v)^⅓ − (2/5)v`, then Newton with `f′ = tan²α`.

**Two guards are mandatory, and both were found by testing:**

1. **Divergence.** [verified] Seed-plus-Newton reaches machine precision in 2–4
   steps up to ~60°, but **diverges at 75°**. Since the spec allows reference
   pressure angles to 60° and *operating* angles run higher still, the solver must
   be **safeguarded**: `inv` is strictly increasing on `[0, π/2)`, so bracket, take
   the Newton step, and bisect whenever it leaves the bracket. Guaranteed
   convergence, still quadratic in practice.
2. **Domain.** `inv α ≥ 0` for `α ≥ 0`, so `v < 0` has **no real solution** — it
   means the requested centre distance is below what the base circles allow.
   This is reachable from ordinary-looking planetary inputs (§4.8 hits it
   constantly), so `inv⁻¹` returns an `Option` and callers report
   "geometrically impossible" rather than producing a NaN that propagates
   silently into a stress number.

The `2/5` in the seed is the second Taylor coefficient, not a fitted constant.

**Root finders**: one safeguarded Newton and one Brent, ~120 lines, unit-tested
against analytic cases. Every solve in §5 routes through these two.

### 4.1 Basic rack and tooth thickness modification

```
m_t = m / cos β        tan α_t = tan α_n / cos β
r   = m_t z / 2        r_b     = r cos α_t
```

Thickness modification `k` (spec: `0 < k < 2`, default 1) is defined on the rack:
tooth width `(π m/2)k`, space `(π m/2)(2−k)`, preserving the pitch. So

```
s_n = m ( (π/2) k + 2 x tan α_n )
```

You suspected this could be expressed through existing parameters. It can,
exactly. Define an **equivalent thickness shift**

```
x_s = π (k − 1) / (4 tan α_n)
```

then `s_n = m( π/2 + 2(x + x_s) tan α_n )` identically. [verified to 4e-16 over
α_n ∈ {14.5, 20, 25, 30}°, x ∈ {−0.5, 0, 0.7}, k ∈ {0.6, 1.0, 1.45}]

Implementation rule, one line:

> **Radial** quantities (root radius, tip radius, cutter depth) use `x`.
> **Thickness** quantities use `x + x_s`.

No new geometry code — `s_t` picks up `x_s`, and `a_c` (the cutter tip-round
offset) already derives from `s_t`, so the trochoid follows.

Two consequences: because `k₁ + k₂ = 2` forces `x_s1 + x_s2 = 0`, **thickness
modification provably cannot move the centre distance** (a good assertion); and
if that rule is ever relaxed, the imbalance appears correctly as backlash through
§4.4 with no new code.

**Cutter tip width** (spec output; the sharp rack tip width ignoring the round),
in the normal plane so it is helix-independent as the spec expects:

```
w_tip,n = π m − s_n − 2 m (h_f − x) tan α_n
```

### 4.2 Profile generation — port of the prior work

`handoff_inbound/gear.py` is correct, validated to 5e-4 mm against a full rack
simulation. **Ported as-is**, with:

| Item | Action |
|---|---|
| `numpy` / `scipy` | plain `Vec<f64>`; our own Brent |
| thickness modification | one line, via `x_s` |
| `tooth_sdf` | **reinstated** in the cut check, as the handoff recommends |
| sampling constants | re-derived from a stated chord tolerance |
| internal gears | new — §4.2.1 |

Retained because they are hard-won: the flank continuing **below the base
circle** to its true intersection with the trochoid (clamping there leaves a
0.3 mm step); `L = r sin α_t − b_c/sin α_t − ρ` as the exact undercut indicator;
the fillet cap `ρ_max = w_tip cos α_t / (2(1 − sin α_t))` (the handoff records
that the obvious `w_tip/(2 cos α_t)` is wrong and silently shrank every
profile-shifted fillet); severed-tooth detection; and clamps that record a note
rather than rejecting input.

**On magic numbers.** The Python's hand-chosen sample counts (400 points, 220
round segments) are replaced by a **chord tolerance in mm** — a user-visible
export setting — with counts derived from local curvature: for radius `R` and
tolerance `ε`, the step is `2 acos(1 − ε/R)`. "How many points" stops being a
guess and becomes a consequence of "how accurate do you want the DXF".

#### 4.2.1 Internal (ring) gears — the main new geometry

Needed for the planetary stage; the prior work has none. Differences: addendum
and dedendum swap sense (`r_a = r − m(h_a − x)` is the *smaller* radius); the
tooth is the *space* of an external gear so the involute runs the other way; the
fillet is generated by a **pinion-shaped cutter**, not a rack, giving a different
(still closed-form) trochoid; and there is a new failure mode — tip interference
between pinion and ring tips — with no external-gear analogue.

This is the largest single piece of new geometry and gets its own milestone,
with its own rack-equivalent validation.

### 4.3 Automatic profile shift — corrected

**Revision 1 had this wrong.** I modelled the working depth as a constraint on
the form radius; your answer to Q3 makes clear it is simpler and more physical
than that. The working depth is *the depth at which we ask the undercut
question* — it substitutes for the dedendum in the cutter-depth term. Reusing the
prior work's `L`, with `b_c = m(h_w − x) − ρ`:

```
x_min = h_w − [ ρ + sin α_t ( r sin α_t − ρ ) ] / m
```

Closed form, exactly invertible because `L` is linear in `x`.

This reproduces exactly the effect you described. With `ρ = 0` it reduces to
`x_min = h_w − z sin²α_t / 2`, so `x = 0` gives `z_min = 2 h_w / sin²α_t`:
[verified]

| Working depth | z_min at α = 20°, ρ = 0 |
|---|---|
| 1.00 module (the classical assumption) | **18** — threshold 17.10, hence "17 teeth" |
| 1.10 module | 19 |
| 1.25 module (a full standard dedendum) | **22** — threshold 21.37 |

So the conventional "17 teeth at 20° PA" is the answer to the question *"is the
flank undercut within 1 module of depth?"*, not *"is the flank undercut at all?"*
Answering the latter needs 22 teeth. That is exactly the hidden assumption this
control is meant to expose.

**A second hidden assumption, which the same formula also exposes.** The
classical rule additionally assumes a **sharp-cornered rack**. With a real cutter
tip radius the straight flank ends higher up, so there is less undercut — at
`ρ = 0.38` (the ISO 53 basic rack), `h_w = 1`, α = 20°, `z_min` drops from 18 to
**13**. [verified] Two assumptions, pulling in opposite directions, both buried
in the same piece of conventional wisdom.

Since the honest answer depends on which cutter you are modelling, the automatic
calculation uses the **actual root radius coefficient** the user has entered, and
the UI should show the sharp-rack figure alongside it for reference. That is one
extra line of output and it makes the difference visible instead of arguable.

**Automatic altered addendum** (tip radius from a minimum tip width): the
transverse thickness at radius `r'` is `s(r') = 2r'(ψ_b − inv α_{r'})` with
`cos α_{r'} = r_b/r'`. Setting `s(r_a) = s_min` is transcendental but monotone
decreasing with a clean analytic derivative,

```
ds/dr' = 2 ( ψ_b − inv α_{r'} − tan α_{r'} )
```

so it is a 2–3 iteration safeguarded Newton bracketed between `r_b` and the
pointed-tooth radius (the `s_min = 0` case the prior work already solves).

### 4.4 Centre distance and backlash

```
inv α_w = inv α_t + 2 (x₁ + x₂) tan α_n / (z₁ + z₂)
a_w     = a cos α_t / cos α_w,        a = m_t (z₁ + z₂)/2
```

Internal mesh: `(x₁ + x₂) → (x₂ − x₁)` and `(z₁ + z₂) → (z₂ − z₁)`, 2 the ring.

**Backlash — exact, not linearised.** Given an actual centre distance `a′`
(nominal + clearance ± tolerance), `cos α′ = a cos α_t / a′` and

```
j_t = 2 a′ ( inv α′ − inv α_w )
```

[verified to 3e-16 mm against direct computation of tooth thicknesses at the
operating pitch circles, over three configurations including helical and
thickness-modified]

Worth dwelling on: it is **exact**, not the textbook first-order
`j_t ≈ 2Δa tan α_w` (to which it correctly reduces); it is zero at `a′ = a_w` by
construction; and every backlash source — profile shift, thickness modification,
clearance, tolerance — enters through just `α_w` and `α′`. One formula, not four
cases.

Angular backlash at gear *i*, with `r′_i = a′ z_i/(z₁+z₂)`: `j_θ,i = j_t / r′_i`.
Nominal/max/min from `a′ = a + c`, `a + c + tol₊`, `a + c − tol₋`.

**Worm axial clearance.** One axial pitch of worm travel advances the wheel one
tooth, so an axial clearance `δ` contributes `j_θ,wheel = 2δ / (m_x z_wheel)`
radians, combined with the centre-distance term above.

### 4.5 Contact ratio, efficiency, and crossed axes

**Path of contact** (external), from the pitch point:

```
ξ_recess   = √(r_a1² − r_b1²) − r′₁ sin α_w
ξ_approach = √(r_a2² − r_b2²) − r′₂ sin α_w
ε_α = (ξ_approach + ξ_recess)/p_bt,     p_bt = π m_t cos α_t
ε_β = b sin β / (π m)
```

*(Corrected — an earlier draft wrote `a_w sin α_w` in **both** lines.
Each length is measured from the pitch point, so each subtracts its own gear's
share `r′ sin α_w`. Only the sum uses `a_w`, since `r′₁ + r′₂ = a_w` — which is
why the familiar contact-ratio formula contains `a_w` and these do not. As
written, both lengths came out negative.)*

Internal: `ε_α = [√(r_a1²−r_b1²) − √(r_a2²−r_b2²) + a_w sin α_w]/p_bt`.

##### Overlap ratio and total contact ratio — planned, milestone 6

`ε_β` is written above but **not yet computed**, for a simple reason: it needs a
face width, and face width is not an input until the stage exists. It arrives
with milestone 6, alongside material and torque.

```
ε_β = b sin β / (π m_n)          overlap (axial) contact ratio
ε_γ = ε_α + ε_β                  total contact ratio
```

`ε_β` is the number of base pitches the tooth trace advances across the face, so
it counts *axial* overlap the way `ε_α` counts profile overlap. Spur gears have
`ε_β = 0` identically, so — like everything else in the helical treatment — this
is one formula with the spur case as a value of it, not a branch.

**It is a design check, not an input to any stress.** Efficiency deliberately
does not use `ε_γ` (see the rejection of that substitution below), and neither
bending nor contact needs it. What it tells the designer is whether the mesh has
full axial overlap: at `ε_β ≥ 1` at least one contact line is engaged at all
times, which is the property helical gears are chosen for — smooth load transfer
and no abrupt engagement. Below 1 the gear is helical in form but still transfers
load like a spur gear, which is usually not what was intended and is invisible
without this output.

**UI**: a `Spur Stage` output beside `Ratio` — three read-only fields, transverse
`ε_α`, overlap `ε_β` and total `ε_γ`, with `ε_β < 1` flagged. This is an addition
to the specification, which lists neither; it is cheap once face width is present
and it answers a question the spec's own helix-angle inputs raise.

**Parallel-axis mesh efficiency, derived rather than quoted.** At a contact point
`ξ` from the pitch point the sliding velocity is `ξ(ω₁+ω₂)` while the input power
is `F_n v_b`, `v_b = ω₁r_b1 = ω₂r_b2`. So the instantaneous fractional loss is
`μ|ξ|(1/r_b1 + 1/r_b2)`. Contact traverses the line of action at constant speed,
so the time average is uniform in `ξ`:

```
η = 1 − μ π (1/z₁ ± 1/z₂) (ε₁² + ε₂²) / (ε_α cos β_b)
```

`ε₁ = ξ_approach/p_bt`, `ε₂ = ξ_recess/p_bt`, `+` external, `−` internal. This is
Buckingham's formula recovered from first principles — no fitted constants.
[verified against a direct numerical average of the instantaneous loss over five
meshes at three helix angles each, agreement to 1e-10 relative.]

**One formula for spur and helical.** `cos β_b` is exactly 1 at zero helix, so
the spur case is a *value* of this expression, not a branch beside it. Friction
acts on `F_bn = F_bt/cos β_b` while useful power crosses as `F_bt`, which is the
whole of the helical correction — the load being spread over several inclined
contact lines changes nothing, because the field of action is uniform along the
face, so the line-weighted mean of `|ξ|` is the same average the spur case takes.

**The `ε_γ` substitution is deliberately not used.** Replacing `ε_α` by
`ε_α + ε_β` is common but drives the predicted loss toward zero as overlap grows,
which is unphysical. Mean sliding does not depend on overlap; only the force
does. What *is* missing is sliding along the contact line — a real helical loss
this under-states, and the honest limit of a one-coefficient friction model.

**The `/ε_α` is load-sharing bookkeeping, and dropping it is the easy mistake.**
It is what holds the *total* transmitted force at `F_n`. Average `|ξ|` per
engagement instead — counting one engagement per base pitch, each carrying the
full load — and the mesh implicitly transmits `ε_α F_n`, so the loss comes out
too large by exactly the contact ratio. The first draft of this implementation
made that error; the numerical check above is what caught it.

Two honest notes. **Forward and backward efficiency come out equal** for
parallel-axis meshes: the formula is symmetric in `(ε₁²+ε₂²)`, and physically it
should be, since swapping driver and driven swaps approach and recess but not the
total sliding. They differ only at second order, which we compute consistently as
`η = P_out/P_in`. Expect to see two identical numbers in the UI; that is a
result, not a bug. Second, the helical case is handled by the `cos β_b` above
rather than by an `ε_γ` substitution — see the paragraphs preceding this one for
why that substitution was rejected.

#### 4.5.1 Worm and crossed-helical are the same mathematics

Answering Q2's research question, at least at the first level: **a crossed-axis
helical pair and a worm drive are both crossed-axis screw gearing.** A worm is
simply a screw gear with very few starts, a high lead angle, and (usually) a
throated wheel. The consequences:

- **Efficiency** uses the screw-gear formulas, which *are* direction-dependent:

  ```
  sin γ = z_starts · m_n / d          (exact — see the note below)
  η_driving_member_1 = (cos α_n − μ tan γ₁) / (cos α_n + μ cot γ₁)
  η_driving_member_2 = (cos α_n − μ cot γ₁) / (cos α_n + μ tan γ₁)
  ```

  Self-locking falls out: `η_back ≤ 0` ⟺ `μ ≥ cos α_n tan γ`.
- **Contact is a point, not a line.** This is the part that genuinely breaks the
  parallel-axis model: Hertzian *line* contact (§4.7), the contact-ratio-based
  load sharing and the backlash geometry of §4.4 all assume line contact. Crossed
  axes need **elliptical (point) Hertz** contact, which is closed form but needs
  the principal curvatures of both surfaces at the contact point and a pair of
  elliptic-integral coefficients (tabulated, or computed by a well-conditioned
  series).
- **Bending has no widely accepted analytical model** for crossed-axis gears —
  practice is to derate a parallel-axis calculation, which is a convention rather
  than a derivation.

So the architectural call is: build **one crossed-axis screw-gear model**
(`screw.rs`) and let both the Worm Stage and a non-zero-axis-angle Spur Stage use
it. That collapses most of Q2's cost — crossed helical becomes nearly free once
the worm stage exists, which is why the milestone order puts worm before it.
The honest limitation to surface in the UI is the bending model; I would show
point-contact Hertz and contact ratio, and mark bending as indicative with the
derating convention named.

**The lead-angle note matters.** People commonly write `tan γ = z m_x/d` and then
iterate, because `m_x = m_n/cos γ` depends on `γ`. Substituting once gives
`sin γ = z m_n/d` — exact, no solve.

#### 4.5.2 Planetary efficiency

Must not be computed mesh-by-mesh in the fixed frame, because the meshes slide at
their *relative* speeds. The Pennestrì–Freudenstein method is general and closed
form:

1. Basic (carrier-fixed) ratio `i₀ = −z_ring/z_sun`.
2. Fixed-carrier efficiency `η₀ = η_sun-planet · η_planet-ring`, each from §4.5 at
   relative speeds.
3. With `w = sgn(T_sun (ω_sun − ω_carrier))` the direction of rolling power:
   `T_ring/T_sun = −i₀ η₀^w`, and `T_carrier = −(T_sun + T_ring)`.
4. `η = |T_out ω_out| / |T_in ω_in|`.

All six input/output combinations and both drive directions from one piece of
algebra, no lookup table — which is what makes it extensible to compound or
Ravigneaux stages later.

### 4.6 Metrology

**Span measurement.** Derived from first principles — the span is a chord along
the base tangent, so it is `(k−1)` base pitches plus one base tooth thickness:

```
W_k = cos β_b · r_b [ 2π(k−1)/z + s_t/r + 2 inv α_t ]
```

which reduces to the familiar
`W_k = m cos α_n [π(k−0.5) + z inv α_t] + 2 x m sin α_n` for the standard rack.
Both go in the test suite, the second as a check on the first. The number of
teeth spanned is computed from the exact admissible range of `k` (both contact
points between form radius and tip radius), picking the `k` nearest the pitch
circle — deterministic, and it reports "no valid span" rather than returning an
unmeasurable number.

**Over pins and balls.** The involute angle at the pin centre satisfies

```
inv φ = ψ_b + d_p / (2 r_b cos β_b) − π/z          then  r_M = r_b / cos φ
```

(`cos β_b` because the pin contacts in the normal plane — the helical case you
flagged; the rest is transverse, which is the correct treatment for balls.)

[verified **independently**, not against a table: for each of four gears the
minimum distance from the computed pin centre to the actual involute flank equals
`d_p/2` to 3e-10 mm. The pin is genuinely tangent.]

With Q5 answered, all four combinations are closed form. Writing `r_M` for the
pin-centre radius:

| | **z even** | **z odd** |
|---|---|---|
| **2 pins** — max distance across opposing pins | `2 r_M + d_p` | `2 r_M cos(π/2z) + d_p` |
| **3 pins** — two adjacent pins form the datum, perpendicular to the third | `2 r_M cos(π/z) + d_p` | `r_M (1 + cos(π/z)) + d_p` |

The three-pin geometry: two pins in adjacent spaces have centres `2π/z` apart, so
their common outer tangent is perpendicular to their bisector at
`r_M cos(π/z) + d_p/2` from the axis. For **odd** `z` a space centre lies exactly
opposite that bisector, giving `r_M + d_p/2` on the far side — hence
`r_M(1 + cos(π/z)) + d_p`. For **even** `z` no space is exactly opposite; the
nearest sits `±π/z` away, contributing `r_M cos(π/z) + d_p/2` and giving the
symmetric `2 r_M cos(π/z) + d_p`. This matches the thread-over-wires convention
you described, including that either count is measurable on either parity.

Pin validity, all closed form: `r_form < r_b/cos φ < r_a` (contact on the usable
flank) and `r_M − d_p/2 > r_f` (pin does not bottom out) — exactly the spec's
"contacts both flanks tangentially and clears the minor diameter".

**Per Q4, only nominal values are computed.** Min/max on span and over-pins need
a tooth thickness tolerance from JGMA 1103-01, which is not available. The
result structs carry `Option<Tolerance>` fields that are `None` today, so adding
the data later is a data change, not a redesign.

#### 4.6.1 JGMA 116-02 — a lookup table, and two grade scales rather than one

I extracted the PDF in `docs/` and **verified the transcription against the
rendered page images**, because the precedence question turns on the numbers
being right. They are. What the standard contains:

- Two tables. **Page 1** covers modules `0.2–0.6` and `0.6–1.0` (both marked
  適用, *applicable*) and `1.0–1.6` (選用, *optional/selective*), pitch diameters
  banded from 1.51 mm to 400 mm, for **grades 0–6**. **Page 2** covers modules
  `1–3.5`, `3.5–6.3`, `6.3–10`, diameters to 4000 mm, for **grades 4–12**.
- Each cell gives two values in μm: 一齒嚙合誤差 (tooth-to-tooth composite error)
  and 全齒嚙合誤差 (total composite error) — exactly the two spec outputs.
- Values are **banded steps, not interpolated**, and are R10/R20 preferred
  numbers. The lookup is: find the module band, find the diameter band, read two
  numbers. No formula, no interpolation.

##### The precedence rule cannot be applied as stated, and the data says why

You asked for "the band with the smaller value wins, regardless of page", on the
sound principle that a tighter allowance is automatically compliant with a looser
one. That principle is right. The problem is the premise it rests on
— that these are overlapping bands of *one* grade scale. **They are not.**

For module 1.0–1.6 at a 12 mm pitch diameter, where both tables apply:

| Grade | Page 1 (fine, `1.0~1.6`) | Page 2 (standard, `1~3.5`) |
|---|---|---|
| 0 | 6.3 / 18 | — |
| 1 | 8 / 25 | — |
| 2 | 10 / 36 | — |
| 3 | 14 / 53 | — |
| 4 | 22 / 71 | **7 / 20** |
| 5 | 32 / 100 | 10 / 32 |
| 6 | 45 / 140 | 14 / 50 |
| 7–12 | — | 20/71 … 71/224 |

Page 2's grade 4 is **three times tighter** than page 1's grade 4. Taking the
smaller value at each grade produces the ladder

```
6.3   8   10   14   7   10   14   20   28   36   45   56   71
                    ^ grade 4
```

which **drops between grade 3 and grade 4**. No rule for choosing between
overlapping entries avoids this, because the grade numbers do not denote the
same thing on the two tables.

*(Corrected — an earlier draft also claimed page 2's grade 4 was tighter
than page 1's grade 0. It is not — 7 against 6.3, marginally looser. The
non-monotonicity is real and is what the argument rests on; that one comparison
was wrong and is withdrawn.)*

The cause is that page 1 and page 2 are **two different grade scales**, and the
grade numbers are not comparable between them. Roughly, page 1 grade *N* lands
near page 2 grade *N+3*, but not consistently enough to be a mapping. The
standard's own annotation supports this reading: page 1's `1.0~1.6` column is
marked 選用 (*optional*), while `0.2~0.6` and `0.6~1.0` are 適用 (*applicable*) —
i.e. the fine-pitch table may optionally be extended to module 1.6, but it is not
the primary table there.

**Recommendation: model two named scales, and never compare grade numbers across
them.**

```
JGMA 116-02 fine       grades 0–6,  modules 0.2–1.6
JGMA 116-02 standard   grades 4–12, modules 1–10
```

##### Settled

The tolerance class control is **one dropdown listing both scales**, filtered to
what the current module and pitch diameter actually have data for:

```
Fine 0 … Fine 6            (JGMA 116-02 fine,     modules 0.2–1.6)
Standard 4 … Standard 12   (JGMA 116-02 standard, modules 1–10)
```

- Both scales are exposed; the user picks. Grade numbers are never compared
  across scales, so every ladder stays monotonic.
- **Default precedence: fine scale first, then lowest grade** — so `Fine 0` where
  the fine scale applies, otherwise `Standard 4`. Deliberately decided on scale
  and grade ordering alone, *not* on which entry yields the smaller error value.
  That keeps the default predictable and independent of the table contents, which
  is what makes it survive the addition of other standards later.
- **Grade 0 is included**; the range is 0–12 subject to availability.
- Each band stays complete per the standard — values are never mixed across
  cells. The table above confirms this is never needed, since one page wins both
  values together.

The tables are transcribed to `data/jgma_116_02.toml` — data, not code, so it
stays auditable against the PDF. Two automated transcription checks: every row's
value count must equal its band count, and every value must be an R10/R20
preferred number. That catches column misalignment, which is the realistic
failure mode when transcribing a scanned table (and which the raw text extraction
did in fact exhibit before the images were checked).

### 4.7 Strength

**Bending.** The spec asks for a new Lewis-type formula accounting for undercut,
profile shift and thickness modification. The framing that replaced it: *stop
using a table of form factors and measure the form factor off the profile we
already generate exactly.* Undercut, profile shift and thickness modification are
then handled because they change the profile, and we measured the profile.

**Critical section — the Lewis parabola, by default.** A cantilever whose outline
is a parabola with its vertex at the load carries uniform bending stress, so the
largest such parabola inscribed in the tooth touches where the real tooth is
weakest. This **diverges from ISO 6336 and AGMA 2101**, which specify a fixed 30°
tangent. The reasons, and the caveats, in one place:

- The 30° tangent is *independent of where the load acts* — the one property the
  cantilever model is meant to have. Its tangents cross the centreline 11.8%
  below the load point at z=9 and 0.04% above it at z=60.
- Experimental single-tooth-bending work reports measured critical locations
  *above* the 30° prediction, with that prediction at the edge of the observed
  range — the direction the parabola moves it. The authors attribute part of the
  divergence to large test deformations, so this is support, not proof.
- It is the more conservative construction everywhere: +2.9% to +13.7% on `Y_F`.
- It changes rankings very little (Spearman ρ = 0.993 over 1521 designs), so the
  choice is principled rather than consequential.
- **`Y_S` is calibrated against the 30° construction.** Where the parabola leaves
  the fillet the pairing is *refused* rather than approximated — see below.

`CriticalSection::TangentAngle` is retained, unused by default, for a number
comparable with a published ISO or AGMA rating.

**Stress correction — switchable, and sometimes undefined.** `Y_S` is the ISO
6336 fit, chosen over Dolan–Broghamer because it is written in the geometry we
already measure (`s_Fn`, `h_Fe`, `ρ_F`) rather than indexed by tooth count and
shift. `StressConcentration::None` reports the form factor alone, which is what
separates a geometry error from an over-fitted correction.

The notch parameter `q_s` is **clamped into the fit's stated range and reported
raw**. The direction matters: `Y_S` rises with `q_s`, so clamping a
sharper-than-stated notch under-predicts stress. `q_s` leaves the range in
practice — 10.3 at z=300 with a 0.05-module cutter — so this is a live case, not
a guard.

Where the parabola's tangency lands on the flank there is no notch, so `ρ_F` has
no meaning and the ISO correction returns nothing. Evaluating it anyway produced
a **17% discontinuity** at z=150→151 while `Y_F` moved 0.03%.

**Load point — decided: (b), the highest point of single-pair contact.**

Three cases, measured over ordinary meshes at module 1:

| mesh | ε | (a) tip | (b) HPSTC | (c) shared | b vs a | c vs a |
|---|---|---|---|---|---|---|
| 17 : 17 | 1.515 | 4.2923 | 3.1078 | 3.1074 | −27.6% | −27.6% |
| 17 : 43 | 1.621 | 4.2923 | 2.9416 | 2.9372 | −31.5% | −31.6% |
| 13 : 60 | 1.614 | 4.7441 | 3.2057 | 3.2003 | −32.4% | −32.5% |
| 25 : 25 | 1.612 | 3.9052 | 2.7760 | 2.7749 | −28.9% | −28.9% |
| 12 : 30, x=+0.4 | 1.455 | 4.0337 | 2.9290 | 2.9224 | −27.4% | −27.5% |
| 20 : 20, x=−0.2 | 1.550 | 4.5534 | 3.1814 | 3.1752 | −30.1% | −30.3% |

**(a) is reported alongside (b)** as the theoretical bound: tip load with the
tooth carrying everything is well defined without a mate, so it is also what the
single-gear calculator shows. (b) is the expected figure wherever a mate exists.

### Why load sharing is not included

(c) was the leading candidate. The measurement retired it, and the reason is
structural rather than a matter of tuning:

> **Once sharing is allowed, the governing point *becomes* the HPSTC.** At the
> tip a tooth carries roughly a third of the load, so tip loading stops
> governing — and the worst surviving point of the cycle is exactly where (b)
> already places it. (c) therefore lands within **0.0–0.2%** of (b) across every
> mesh tried.

So the expensive part of (c) — a calibrated mesh-stiffness model — buys almost
nothing for a worst-case number, while dragging in tooth and rim stiffness,
deflection under load, and manufacturing deviation. Those inputs are not
available to a high-level design tool, and a stiffness model that is not
calibrated produces confident numbers that are *worse* than the conservative
bound, because they look authoritative.

Two conditions would change this, and both are worth naming so the decision can
be revisited rather than rediscovered:

- **A duty-cycle or transmission-error calculation**, where the whole mesh cycle
  matters rather than its worst instant. Sharing is essential there and the
  0.2% figure does not apply.
- **High contact ratio (ε ≥ 2)**, where two pairs are *always* engaged and the
  single-pair zone this argument rests on does not exist.

The machinery is in place either way: `ContactPath::load_fraction` takes a
`LoadSharing` model, and `LoadSharing::LinearRamp` exists as an explicitly
uncalibrated 1/3→2/3 ramp. It is labelled a placeholder for a stiffness model,
not a substitute for one; its purpose was to size the effect, and it has.

**Contact.** Exact Hertzian line contact. At a contact point `ξ` from the pitch
point, `ρ₁ = r_b1 tan α_w + ξ` and `ρ₂ = r_b2 tan α_w − ξ` (note
`ρ₁ + ρ₂ = a_w sin α_w`), and

```
1/E* = (1−ν₁²)/E₁ + (1−ν₂²)/E₂
σ_H  = √( (F_n/b) (1/ρ₁ ± 1/ρ₂) E* / π )
```

`+` external, `−` internal — which is why the material library carries `E` and
`ν`. Crossed-axis stages use point contact instead (§4.5.1).

**Which points are checked — corrected.** An earlier draft said "the pitch point
and the inner point of single-pair contact (usually the pinion's worst case)".
That "usually" was hiding a real defect. Since `ρ₁ + ρ₂` is constant along the
path, the relative radius peaks where the two are equal — at
`ξ = (r_b2 − r_b1) tan α_w / 2` — and falls away toward **both** ends. That
balance point is on the recess side when gear 1 is the pinion and on the
approach side when gear 1 is the wheel, so the worse single-pair boundary swaps
with the labelling. Checking only the inner one made the contact stress of one
physical mesh depend on which gear the caller called 1. **Both boundaries are
now evaluated**, and a test asserts that swapping the labels leaves the answer
unchanged.

**The load is stored as torque, not as a force.** Every force in a mesh is a
projection, and a projection means nothing until you say of what, onto which
plane, at which radius. There are four in play and they differ by `cos α_t`,
`cos α_w` and `cos β_b`:

```
F_t  = 2000 T / d       tangential at the reference cylinder
       2000 T / d'      tangential at the operating cylinder
F_bt = T / r_b          along the transverse line of action
F_bn = F_bt / cos β_b   normal to the tooth flank
```

Storing any one of them bakes a choice of plane and radius into a bare number
that no longer records which it made. Torque does not: it is a property of the
shaft, invariant under every redefinition of a radius, and it is what the
specification takes in and reports out. Each projection is therefore spelled out
at its point of use. `F_bt` is the one quantity **both gears share** — action and
reaction along the line of action — which is why contact stress is built on it,
and `Load::across_mesh` re-quotes a load against the mate by `T₂ = T₁ r_b2/r_b1`.

*(Corrected — an earlier revision stored `F_bt` in a field called `normal_force`.
Nothing it computed was wrong, but the name asserted the normal plane while the
value was transverse, which is the exact failure this arrangement forecloses.)*

**Helical: three plane changes, and they nearly cancel.** For contact,

```
ρ_n  = ρ_t / cos β_b        curvature is seen in the normal plane
F_bn = F_bt / cos β_b       the flank force, not its transverse projection
L    = b / cos β_b          one contact line, inclined across the face
```

which collapses to `σ_H = √((F_bt/b)·cos β_b/ρ_t·E*/π)` — a helical mesh comes
out below the same transverse geometry by exactly `√(cos β_b)`, 3 % at β = 20°.
That is pure geometry and owes nothing to load sharing. The *extra* benefit of
several contact lines being engaged at once **is** load sharing and stays
deferred; assuming a single line is the conservative reading and is continuous
with the spur case at β = 0.

##### Helical bending: where each `cos β` comes from

A helical tooth does not bend as its transverse section. Two separate corrections
follow, and they pull in opposite directions — getting one without the other is
worse than getting neither, because the errors stop cancelling.

**The section: `z_n = z / cos³β`.** Cut the pitch cylinder with a plane normal to
the helix. The intersection is an ellipse whose semi-minor axis is `r` — the cut
does not change the radius perpendicular to the axis — and whose semi-major axis
is `r/cos β`, stretched by the obliquity. The tooth sits at the end of the *minor*
axis, where an ellipse of semi-axes `A ≥ B` has radius of curvature `A²/B`:

```
ρ = (r/cos β)² / r = r / cos²β                    ← two powers, from curvature
```

That `ρ` is the pitch radius of the equivalent spur gear. Converting it to a
tooth count at the *normal* module supplies the third power, because the
transverse module is the larger one, `m_t = m_n/cos β`:

```
z_n = 2ρ/m_n = 2r/(m_n cos²β),  r = m_n z/(2 cos β)   ⟹   z_n = z / cos³β
```

**What it accounts for:** the normal section is *flatter* than the transverse
one — larger effective radius, straighter flanks, lower form factor. A helical
tooth is therefore stronger in bending than its actual tooth count suggests,
which is why `σ_F` falls monotonically with helix angle (69.2 → 53.5 MPa from
β = 0 to 30° in the worked mesh).

**The load point: `ε_αn = ε_α / cos²β_b`.** Again two powers, from two places:

- **Base pitch.** Contact lines in the plane of action lie at `β_b` to the axis,
  so their perpendicular spacing is shorter than the transverse spacing:
  `p_bn = p_bt cos β_b`.
- **Path length.** The path of contact measured in the normal section is longer
  than the transverse path by `1/cos β_b`.

```
ε_αn = (g_α / cos β_b) / (p_bt cos β_b) = ε_α / cos²β_b
```

Note it is the **base** helix angle, `sin β_b = sin β cos α_n`, not `β` — the
contact lines live on the base cylinder, not the reference one.

**What it accounts for:** where the load sits. The highest point of single-pair
contact is one base pitch back from the end of the path, so a higher contact
ratio pushes it *closer to the tip* — longer moment arm, higher stress. This
partly offsets the section effect above, and it is why applying only the `z_n`
correction would over-state a helical gear's strength.

**The load point itself needs no mate.** Measured from the tip it depends only on
the gear's own geometry and the contact ratio:

```
u_load = u_tip − (ε_α − 1) p_b / r_b
```

[verified exact against the path-of-contact construction over seven meshes,
including reversed pairs]. That is what lets the same relation be reused on the
virtual gear with `ε_αn`, and what keeps the API a scalar rather than a mesh.

**One honest limit.** `ε_αn = ε_α/cos²β_b` is ISO's *relation*, not an identity.
Building the virtual pair and measuring its contact ratio directly disagrees with
it — exactly at β = 0, by 0.03 % at 10°, 0.11 % at 20° and 0.20 % at 30°. The gap
is inherent to the construction rather than an error: the virtual gear keeps the
addendum in normal modules, so its tip circle is a smaller fraction of its pitch
radius than the real gear's is, and the two tip circles are not in exact
correspondence. [verified] The consequence is bounded and small — at β = 30° that
0.20 % moves `Y_F` by **0.38 %**.

**At `β = 0` all of it reduces exactly**: `cos β_b = 1`, `z_n = z`, `ε_αn = ε_α`,
and the virtual gear is rebuilt bit-for-bit identical to the real one. **There is
no spur branch anywhere in the strength path** — the spur results are values of
the helical formulas, and the CLI's spur output is unchanged to the last digit
across this whole revision.

**`Y_β` is not applied** — ISO's helix factor is an empirical fit, and omitting it
leaves `Y_β = 1`, which over-predicts stress; conservative, and no fitted constant
enters.

**Minimum face width — closed form, no iteration.** Since `σ_F ∝ 1/b` and
`σ_H ∝ 1/√b`:

```
b_min,bending = b σ_F / σ_allow          b_min,contact = b (σ_H / σ_allow)²
```

`b_min` is independent of the `b` it was evaluated at, as it must be — a good
invariant test.

**S-N — withdrawn, replaced by two allowables.** An earlier draft specified a
Basquin law `σ = C N^(−1/k)` fitted to two points, with an optional endurance
knee. The material survey killed it: those two points do not exist for six of
the eight materials, and no amount of modelling recovers data that was never
measured. Each material instead carries a peak allowable and a cyclic allowable,
which pair with the peak and cyclic input torques. The reasoning is in §6.2.

### 4.8 Planetary layout

**Ring tooth count (Q6).** The required planet profile shift for a common centre
distance is **strictly increasing in `z_ring`** [verified], and `z_ring = z_sun +
2 z_planet` gives exactly `x_planet = 0` — a clean sanity check that the whole
construction passes. Monotonicity is what makes the search provably complete: the
candidate `z_ring` values are exactly the integers between the value that yields
`x_min` and the value that yields `x_max`, so sweeping ascending `x_planet` from
the lower bound cannot miss a solution, as Q6 requires.

**Common centre distance solve.** Define
`g(x_p) = a_w,ext(x_s + x_p) − a_w,int(x_r − x_p)`. External increases with `x_p`,
internal decreases, so `g` is strictly increasing — unique root, Newton is safe.
The derivative is analytic via `d(inv α_w)/dΣx = 2 tan α_n/Σz` and
`d(inv α)/dα = tan²α`:

```
da_w/dΣx = [ a cos α_t sin α_w / cos²α_w ] · [ 2 tan α_n / (Σz tan²α_w) ]
```

[verified against central differences to 6+ digits; Newton from `x_p = 0`
converges to 3.6e-15 mm in **4 iterations**]

**The bracket is closed form, and it is needed.** Both meshes require
`inv α_w ≥ 0`, which bounds `x_p` on both sides:

```
x_p ≥ −inv(α_t)(z_s + z_p)/(2 tan α_n) − x_s          (external)
x_p ≤  x_r + inv(α_t)(z_r − z_p)/(2 tan α_n)          (internal)
```

This is not academic — most candidate `z_ring` values fall outside it. For
z_s = 17, z_p = 17, only `z_r ∈ [48, 54]` has any solution at all; everything
else is genuinely impossible, not merely unconverged. Without the domain guard of
§4.0 this produces NaNs; with it, the UI can say *why*.

Worked example [verified], 17/17, three planets, `x_planet ∈ [0, 0.5]`:

| z_ring | equal spacing | x_planet required |
|---|---|---|
| 48 | no | −0.6684 |
| 49 | yes | −0.4807 |
| 51 | no | −0.0000 ← geometric ideal |
| **52** | **yes** | **+0.2480** ← selected |
| 54 | no | +0.6862 |

**Layout checks**, all closed form:

- Equal planet distribution: `(z_s + z_r) mod N = 0`.
- Simultaneous meshing: `N | z_s` **and** `N | z_r`. (Given equal spacing, either
  implies the other.) In the example above this is false for every candidate,
  since `z_s = 17` is not divisible by 3 — correctly reported.
- Planet clearance: `2 a_w sin(π/N) − d_a,planet`, tested against the minimum.

### 4.9 Train-level accumulation

**Ratio and torque.** Per stage `i = z_out/z_in` (worm: `z_wheel/z_starts`;
planetary: from §4.5.2). Total ratio is the product. Torque propagates with
efficiency — forward `T_{k+1} = T_k i_k η_k`, backward `T_{k−1} = T_k η_k / i_k`
— efficiency always *reducing* delivered torque regardless of direction, which is
the sign convention to get right and to test.

Per Q1, **output speed and output torque are outputs**, computed from the input
speed/torque and the total ratio. The spec's listing of them as inputs was a typo.

**Backlash accumulation**, referred to the output shaft:

```
θ_out,total = Σ_k  j_θ,k / Π_{j>k} i_j
```

Each stage's angular backlash divided by the ratio of everything downstream. A
consequence worth surfacing in the UI: the **last** stage dominates, and backlash
in the first stage is nearly free. Min/max propagate identically.

**Tooth cycles.**

- *Intermittent*: the actuation range is at the output, so work backwards —
  revolutions of gear *i* = `(range/360) × Π(ratios between i and output) ×
  actuation count`.
- *Continuous*: `rpm × 60 × hours × operating%/100`.
- Cycles per revolution: 1 for a simple gear, `N_planets` for sun and ring.
- **Planets are a special case.** A planet tooth is loaded on one flank by the sun
  and the *other* by the ring — fully reversed bending, roughly a 0.7× penalty on
  allowable bending stress by ISO convention — and its rotation counts **relative
  to the carrier**. Easy to get silently wrong; called out in both code and UI.

### 4.10 Angularly varying profile shift (planned)

A purely **2D** variation: the profile shift varies with angular position about
the axis, maximum at 0° and minimum at 180°, as a hob moving radially in and out
once per revolution would produce. It is *not* a beveloid — the variation is
angular, not axial.

#### The model

The natural parametrisation of a once-per-revolution radial hob motion, matching
"maximum at 0°, minimum at 180°", is sinusoidal:

```
x(θ) = (x₁ + x₂)/2  +  ((x₁ − x₂)/2) · cos θ            e = m (x₁ − x₂)/2
```

`e` is the eccentricity the feature is really expressing. Two things stay
concentric and unchanged, and this is the part that makes the result "nearly a
shifted circle" rather than an actual eccentric:

- **The pitch and base circles remain centred on the axis.** Profile shift does
  not move them — [verified] changing `x` leaves `r`, `r_b` and `α_t` altered by
  exactly zero, with the angular seat `ψ_b` the only thing that moves.
- **The angular tooth pitch stays 2π/z.** The hob's tangential rolling is
  untouched; only its radial position varies, so tooth centrelines stay evenly
  spaced.

What does move is the tip and root envelope. Tooth *k* has
`r_a = r + m(h_a + x(θ_k))`, so the tip envelope is a limaçon
`const + e·cos θ` — a displaced circle to first order in `e`. [verified] Its
deviation from a true displaced circle has a closed form, `e²/2ρ`:

| `e` | max deviation from a displaced circle |
|---|---|
| 0.10 mm | 0.0005 mm |
| 0.25 mm | 0.0033 mm |
| 0.50 mm | 0.0132 mm |

So your "nearly circular, but shifted away from the original axis" is
quantitatively right, and we can report the exact departure rather than hand-wave
it. Note precisely *which* centre moves: the tip and root envelopes shift by `e`
while the pitch and base circles do not.

#### The governing constraint

The intended function is **eccentric body motion with a genuinely constant
transmission ratio** — explicitly *not* the varying ratio you get by displacing a
conventional gear. That single requirement determines the geometry, because
constant ratio has an exact and restrictive condition:

> Every **driving** flank must be an involute of one base circle **concentric
> with the rotation axis**, and those flanks must sit at **exactly equal angular
> spacing**.

Nothing else is constrained. Tip radius is free — it only decides where the
involute is truncated. Tooth thickness is free — it only decides backlash and
when contact transfers. That freedom is what makes the feature possible at all,
and it is why the tip envelope can be eccentric while the action stays uniform.

#### Varying the addendum alone cannot work

A variant that varies only the radial part of the shift, holding tooth thickness
uniform, is **not a candidate**: addendum modification does not move the flanks
at all. [verified] Both
flank seats come out identical to a standard gear — drive and coast pitch error
exactly 0.000 μm, tooth thickness variation exactly 0.000 mm — for any `e`.

It therefore produces an eccentric *outer surface* on a mesh that is entirely
concentric. It answers the question "how do I get an eccentric OD", which is not
the question. Whatever eccentricity the mesh is meant to express has to live in
the **flanks**, and only the thickness-changing part of profile shift puts it
there. Dropped.

#### The fundamental trade, and why there is no third option

Profile shift thickens a tooth by moving the rack flank tangentially — and it
moves the tooth's **two flanks in opposite directions**. That single fact
produces an exact impossibility:

> Suppose both flank sets were uniformly spaced: `right_k = 2πk/z + c_R` and
> `left_k = 2πk/z + c_L` for every *k*. Then the tooth's angular thickness is
> `right_k − left_k = c_R − c_L`, the same for every tooth. **Uniform spacing on
> both flanks forces uniform tooth thickness.**

So a gear with varying tooth thickness — which is the whole point here — **cannot
be exactly conjugate in both directions.** There is no clever indexing that
escapes this; it is two lines of algebra. The only choice is how to distribute
the unavoidable error.

Parametrise the per-tooth indexing offset as `λ · (ψ_b,ref − ψ_b,k)`, with λ = 0
meaning no compensation and λ = 1 meaning the driving flank is fully corrected.
Then the drive-flank error scales as `|1 − λ|` and the coast-flank error as
`|1 + λ|`. [verified] At z = 17, α = 20°:

| | λ | Forward drive | Reversed drive | Symmetric? |
|---|---|---|---|---|
| **E2** | 0 | 62.6 μm | 62.6 μm | **yes** |
| **E3** | 1 | **0.000 μm** | 125.2 μm | no |
| — | 0.5 | 31.3 μm | 93.9 μm | no |

(base-tangent pitch error at e = 0.25 mm; all figures scale linearly with e.)

**Answering your second question directly: E3 is not symmetric.** Reversing the
drive makes the coast flanks the driving flanks, and those carry *twice* the
uncompensated error — 125 μm against E2's 63. E3 buys exact conjugacy in one
direction by paying double in the other. It is a one-way component.

And the compromise does not exist: minimising the worse of the two errors gives
`min_λ max(|1−λ|, |1+λ|) = 1` at **λ = 0**. So **E2 is the minimax optimum** — any
compensation that improves one direction degrades the other by more than it
gains. E2 is not merely the naive process; it is provably the best possible
symmetric choice.

The real decision is therefore just:

- **Single-direction drive** → E3. Exactly conjugate forward, at 2× the error in
  reverse. Zero transmission error where it matters.
- **Reversing drive** → E2. Equal and minimal error both ways, 63 μm at
  e = 0.25 mm.

Both give a varying tooth thickness — 339 μm of base-thickness spread at
e = 0.25 mm — and both make tip *and* root eccentric by `e`.

One manufacturing note, since you framed this by the cutting process: **E3 is not
producible by radial hob motion alone.** It needs that radial motion synchronised
with a once-per-revolution differential rotation of the workpiece — an ordinary
extra axis relationship on a CNC hobber with an electronic gearbox, unavailable
on a mechanically geared machine. E2, by contrast, is exactly what the plain
radial oscillation gives.

#### The feature is cheap, for a reason worth stating

A per-tooth constant `x` might look like an approximation — the physical hob
sweeps 1.7 tooth pitches while cutting one tooth. But constant ratio *requires*
each driving flank to be a pure involute at a single seat, which is exactly what
the generator produces for a single scalar `x`. The per-tooth model is therefore
not an approximation of the specification; it **is** the specification:

- The profile generator stays **unchanged**, taking one scalar profile shift, and
  is called ⌈z/2⌉+1 times.
- **No new envelope derivation is needed.** The "L3" exact-envelope work
  once considered here is not required.
- The existing rack-simulation test suite applies unchanged, since every tooth is
  an ordinary gear tooth that must pass it.

What does change is assembly: teeth are generated individually and placed at
`2πk/z + offset_k` rather than replicated z times from one half-tooth.

#### Why the naive process fails

This is the finding that determines how much work the feature is, and it is not
obvious. **One flank is generated over a large sweep of gear rotation** — the hob
is cutting a given tooth long before and after that tooth passes the cutter.

[verified] For z = 17 at 20°, one flank is generated from φ = −20.9° to +16.0°,
a **36.9° sweep — 1.7 tooth pitches**. Over that sweep `x(θ)` is itself changing.
The variation of `x` within a single tooth's own generation:

| Tooth at | `x` varies across its flank by |
|---|---|
| 0° | 0.03 × Δx |
| 45° | 0.43 × Δx |
| **90°** | **0.63 × Δx** |
| 180° | 0.03 × Δx |

So near the quadrature positions, **63% of the whole shift range occurs within
one tooth**. At Δx = 0.25 and module 1 that is 0.16 mm of profile-shift error if
the tooth is treated as having a single value of `x`. That is not a small
approximation to wave through — it is comparable to the feature's entire effect.

This is the quantitative reason **E2 cannot be rescued by refining it**. A truly
faithful simulation of the naive hob process would not converge on a constant
ratio — it would converge on flanks that are not involutes at all, and a ratio
that varies. The fix is E1 or E3's indexing discipline, not a better envelope.

It is also why an eccentric-motion gear is a real design rather than a trivial
one: the cutting process has to be *corrected*, not merely offset.

#### What else changes

1. **z-fold symmetry is lost.** The current `profile()` generates one half-tooth
   and replicates it z times; that must become per-tooth generation. Mirror
   symmetry about the 0–180° axis survives, so only ⌈z/2⌉+1 distinct teeth need
   generating — [verified] 9 for z = 17, 21 for z = 40.
2. **Validity varies around the gear.** Some teeth may be undercut and others not;
   the severed-tooth case can occur over part of the revolution only. Clamp notes
   become per-tooth and the UI must say *which* teeth a warning covers.
3. **Every shift-dependent output becomes a range** — span, over-pins, tip width,
   tip and root diameter — reported as min/max around the revolution.
4. **Viewer**: the full z-tooth outline, since it is no longer symmetric, with the
   tip-envelope centre offset annotated.
5. **DXF**: one closed polyline for the whole gear, as now.

#### Scoping

Still its own milestone, after the geometry core and test suite are proven — but
now a small one, since it needs no new mathematics. The work is per-tooth
assembly, the indexing offset, per-tooth clamp reporting, and the range outputs.
Nothing in milestones 0–9 is affected, and no structural accommodation is needed
now beyond what §3 already provides.

#### Operating mode: the centre distance is commanded, not floating

Settled: the eccentricity is managed by external mechanics, so the centre
distance follows the ideal profile as closely as the machine allows. That removes
the "what floats" question and makes the ideal profile a **first-class output** of
the tool. It is closed form and already implemented in §4.4:

```
inv α_w(θ) = inv α_t + 2 ( x(θ) + x_mate ) tan α_n / (z + z_mate)
a_w(θ)     = a cos α_t / cos α_w(θ)
```

Worth noting up front: **`a_w(θ)` is not exactly sinusoidal even though `x(θ)`
is**, because it passes through `inv⁻¹` and a cosine. So a mechanism built as a
simple crank or eccentric will not track it perfectly, and the residual is
computable — the tool should report both the exact `a_w(θ)` profile and the
residual backlash left by a best-fit pure sinusoid. That is the number an
engineer building the mechanism actually needs.

#### An unresolved coupling — and a correction to the λ recommendation

Commanding the centre distance introduces the effect I flagged, and having looked
harder I no longer think it is negligible:

> Moving the centre distance while the drive flank stays loaded **does** shift the
> angular phase of the driven gear.

The rack case settles the principle unambiguously and is already verified
elsewhere in this document: moving a rack radially by δ with the flank in contact
displaces each flank tangentially by `δ tan α` — that *is* the profile-shift
thickness relation of §4.1. A gear pair behaves the same way, and the magnitude is
of the same order as the pitch errors tabulated above, not smaller.

The consequence matters:

> **λ = 1 (E3) is optimal only for a *fixed* centre distance.** With `a(θ)`
> commanded to track the ideal profile, the centre-distance motion contributes its
> own phase term, and the λ that makes the *total* phase uniform is not 1.

So the recommendation changes shape. Rather than fixing λ = 1, the tool should
**solve for λ** given the operating mode: choose λ so that the indexing
contribution and the centre-distance contribution cancel, leaving uniform total
phase. That is a well-posed one-dimensional problem, and exposing λ (as agreed)
remains right — but its *default* should be computed, not set to 1.

**I have not closed the coefficient.** I built a quick involute mesh model to
pin it down and it failed its own validation check against the §4.4 backlash law,
so nothing from it is quoted here and no number for the optimal λ appears in this
document. Deriving it properly belongs in this milestone, and the acceptance test
is already available and trustworthy: **any mesh-phase model must first reproduce
`j_t = 2a′(inv α′ − inv α_w)`**, which is verified to 3e-16. That check is what
caught the bad model, and it should gate the real one.

This does not affect anything else in the document — §4.4 itself is unaffected,
and no other feature depends on the mesh-phase model.

#### What I would still want confirmed

1. **Sinusoidal `x(θ)`**, or another interpolation between the 0° and 180°
   values? Sinusoidal is what a once-per-revolution radial motion gives and is the
   only form yielding a clean displaced-circle envelope.
2. **What the mechanism can actually follow** — if it is a simple eccentric
   (pure sinusoid in `a`), the tool should optimise `x(θ)` against *that*
   constraint rather than reporting the ideal and letting the residual fall where
   it may. Worth knowing before the milestone starts.

---

## 5. Where closed form is genuinely impossible

The complete list. Five scalar solves, each monotone, each bracketed, each with
an analytic derivative. Everything else in this document is algebraic.

| # | Solve | Method | Iterations |
|---|---|---|---|
| 1 | `inv⁻¹` — involute inversion | series seed + **safeguarded** Newton, with a domain guard | 2–4 |
| 2 | Tip radius for a given tip width (incl. pointed tooth) | Newton, analytic `ds/dr` | 3 |
| 3 | Flank/fillet junction when undercut | Brent, bracketed by construction | ~40 |
| 4 | Planet profile shift for common centre distance | Newton, **closed-form bracket** (§4.8) | 4 |
| 5 | 30° tangent critical root section | Brent on the trochoid parameter | ~40 |

None is an optimiser, none can fail to converge, none has a tuning parameter.
That is as close to closed form as involute geometry allows — the involute
function is not algebraically invertible, and that single fact causes #1, #2 and
#4.

Worth stating separately: **guards matter as much as the solvers.**
Testing found that ordinary planetary inputs routinely request a centre distance
outside the involute domain, and the difference between a guarded and unguarded
`inv⁻¹` there is the difference between "this ring tooth count is impossible" and
a NaN silently reaching a stress figure.

---

## 6. Material library

**TOML**, one file, human-readable and editable —
`crates/gear-io/data/materials_default.toml`. The model is
`gear_core::material`; parsing and export are `gear_io::materials`, because
TOML is I/O and `gear-core` stays serde-only.

```toml
[[material]]
name = "PA6 GF30"
class = "polyamide"
grade = "EMS-GRIVORY Grilon BG-30 S"
condition = "dry as moulded / conditioned at 23 °C, 50 % RH"
source = "EMS-GRIVORY Grilon BG-30 S CAMPUS datasheet, ISO 10350. Retrieved 2026-08-15."
density           = { dry = 1350.0, basis = "datasheet" }
elastic_modulus   = { dry = 9500.0, conditioned = 6000.0, basis = "datasheet" }
poissons_ratio    = { dry = 0.36,   basis = "estimated", note = "0.39 less 0.001 per % glass" }
ultimate_allowable = { dry = 185.0, conditioned = 125.0, basis = "datasheet", note = "Stress at break" }
ultimate_measure  = "break"
fatigue_allowable = { dry = 55.5,  conditioned = 37.5, basis = "estimated", note = "0.30 × ultimate" }
```

### 6.1 What the survey found, and the decision it forced

The data was researched before the model was written, and the model is shaped by
what actually exists. Across the library:

| Property | Availability |
|---|---|
| Density, elastic modulus, tensile strength | published for **all** materials, on primary datasheets |
| Poisson's ratio | published for the steels and POM; **no polyamide datasheet publishes it** |
| Fatigue | published for the steels; a printed **graph** for POM; **nothing at all** for the polyamides |

Two structural findings, not merely missing numbers:

1. **Glass-filled grades have no yield point.** They break first, so their
   datasheets report stress at break, not yield. `ultimate_measure` records
   which, so the number stays comparable to its own source.
2. **`1215 Hardened Steel` is not metallurgically coherent.** 1215 is a ~0.09 %C
   resulphurised free-machining steel and cannot be through-hardened, only
   carburised — a hard case over a soft core, which one scalar cannot represent.
   Both 1215 entries were **dropped**; the list is eight materials, not twelve.

**The decision: ship estimates, label them, and let the user edit them.** A
calculator with empty fields cannot produce a ballpark number, and ballpark
numbers before refinement are the point of the tool. This is a deliberate
departure from the no-magic-numbers bar the rest of the document holds to, and
it is confined to material data — no geometry or solver takes an estimated
constant. Three things keep it honest:

- **Every value carries a `basis`**: `datasheet`, `derived` (computed exactly
  from two published values), `chart` (read off a published graph), or
  `estimated`. `Material::weakest_basis` gives the entry's overall confidence,
  because a material is only as good as its worst number.
- **Anything that is not a plain datasheet reading must carry a `note`** saying
  what it is. A test enforces this.
- **Estimates are class-uniform**, so entries stay comparable to each other even
  where the absolute value is a guess: polyamide fatigue is `0.30 × ultimate`
  throughout, and polyamide `ν` is `0.39 − 0.001 per % glass`.

The `ν` estimates are cheap in consequence and it is worth saying why: `ν`
enters Hertz only through `(1−ν²)/E` and `σ_H ∝ √E*`, so the **entire** plausible
polymer range `ν ∈ [0.33, 0.44]` moves contact stress by **±2.5 %** [verified].
The slope of the estimate is anchored on DuPont's own POM measurements, which
fall 0.37 → 0.35 over 20 % glass. Fatigue is the opposite case — the uncertainty
there is order-of-magnitude and it is flagged as the weakest column in the file.

### 6.2 Two allowables, not an S-N curve

Earlier drafts stored a two-point Basquin law with an optional knee. **Withdrawn.**
Fitting Basquin needs two points on a fatigue curve, and those points do not
exist for six of the eight materials. A curve fitted to invented points is worse
than an honest scalar, because it looks like it knows more than it does.

Instead each material carries `ultimate_allowable` and `fatigue_allowable`,
which map onto the **peak** and **cyclic** input torques the geartrain already
takes (§4.9). Two allowables answer the two questions the tool is actually
asked, and close a loop the spec had left open. Tooth cycles remain an output
for the engineer's judgement rather than feeding a life calculation.

The two available standards that would have done better were considered and are
**not** used: ISO 6336-5 (σ_Flim/σ_Hlim for steels by material class) and
VDI 2736-2 (gear-specific Wöhler lines, but only for unfilled POM, PA66, PET and
PE — none of the glass grades). Both are paywalled, and transcribing their
tables into an open repository is a licensing question the JGMA precedent does
not settle. Named here so the decision can be revisited rather than rediscovered.

### 6.3 Conventions

- **Density units — settled.** Stored SI (kg/m³), displayed as g/cm³. This
  follows the general rule now adopted: **SI internally wherever reasonable, unit
  conversion only at the display boundary.** The two places that rule bends are
  where the domain's own conventions are unambiguous and SI would be perverse —
  lengths in mm rather than metres (all gear geometry, and what DXF expects) and
  stresses in MPa rather than Pa.
- **Moisture is stored, not chosen.** Unfilled PA6 loses **two thirds** of its
  stiffness between dry-as-moulded and 50 % RH — far the largest uncertainty in
  the file. Both states are published, so both are stored, and `Value::get`
  returns the conditioned figure wherever one exists: a gear in service has been
  in service. The selection is not exposed in the UI; the stored dry value is
  there for anyone who needs it.
- **Provenance per entry**, in the data, not a README, or it is lost on the
  first edit. Each entry names the specific *grade* measured — "PA6" is not a
  material, some particular grade was tested, and the entry says which.
- **Default material**: "4340 Hardened Steel" (the spec's "Hardened
  Medium-Carbon Steel" is not in the list — confirmed as one of the typos).
- **Every property is editable in the UI**, seeded from the library and shown
  greyed until edited, so a user can tweak a value for their own case without
  authoring a library file. Overrides live in the input state, so §3.1 still
  holds: outputs remain a pure function of inputs.

### 6.4 The library

Eight materials. Grade choices worth recording:

| Entry | Grade | Note |
|---|---|---|
| 4340 Steel | AISI 4340, annealed | |
| 4340 Hardened Steel | AISI 4340, oil quenched 845 °C, tempered 425 °C | 46 HRC, yield 1365 / UTS 1500 MPa. Lower tempering gives more strength (1860 MPa yield at 205 °C) but 53 HRC is past where a through-hardened gear is practical to finish. This temper is also the one the published `R = −1` fatigue work uses. |
| Brass C360 | UNS C36000, H02 half hard | `ν = 0.32` is **derived**, not estimated: CDA publishes both `E = 14 000 ksi` and `G = 5 300 ksi`, and `ν = E/2G − 1`. |
| POM Delrin 100P | DuPont Delrin 100P NC010 | The one polymer with a published `ν` (0.37) and a real fatigue curve. |
| PA6 | EMS-GRIVORY Grilon BS | |
| PA6 GF30 | EMS-GRIVORY Grilon BG-30 S | |
| PA GF50 | EMS-GRIVORY Grilon BG-50 S | |
| PA GF70 | EMS-GRIVORY Grivory GVX-7H | Partially aromatic copolyamide — 70 % glass does not exist on a plain PA base. |

**No glass-filled POM, and that is a finding.** The obvious candidate, Delrin
570, is glass *filled*: fibres added without effective coupling to the matrix,
so load does not transfer. It is **25 % weaker** in tension than unfilled Delrin
(53 MPa against 71) while being 63 % stiffer — for a tooth in bending, the wrong
trade. Glass *coupled* acetals are genuinely stronger (Celcon GC25A: 106 MPa,
8600 MPa modulus), so a glass POM can be added later provided it is a coupled
grade. The distinction is not pedantry; it reverses the sign of the strength
change.

**Sourcing note.** MatWeb refuses automated access, CAMPUS is a JavaScript
application, and the MatWeb mirrors are corrupted — one "PA6" page carried a
different grade's data and two contradictory yield values. Manufacturer PDF
datasheets in CAMPUS/ISO 10350 format are the reliable route and are what every
polymer entry rests on. PA66 was dropped partly for this reason: EMS publishes
no verifiable unfilled PA66 sheet, and a BASF entry beside seven EMS ones adds a
vendor inconsistency for a material barely distinguishable from PA6.

---

## 7. Export and import

**Geartrain and material library — TOML**, the same shape as the input structs,
so `serde` gives both directions. **Inputs only** (outputs are derived), so files
stay small, diffable and immune to going stale. Import creates a new tab; nothing
is written back except by explicit export, per the spec.

**DXF export** — ASCII, hand-written (~150 lines):

- the profile as a dense `LWPOLYLINE`, spacing derived from the **chord
  tolerance** (§4.2) rather than a fixed count;
- tip and root arcs as true `ARC` entities where the geometry is genuinely
  circular, so CAD sees exact curves rather than facets;
- reference circles (pitch, base, root, tip) on a construction layer.

In the browser the file is a Blob download; the same core call writes a file from
`gear-cli`.

---

## 8. UI structure

```
Sidebar (left, scrollable, collapsible)
├── [Import library] [Export library]
├── ▸ Gears        → one row per gear tab
└── ▸ Geartrains   → one row per geartrain tab

Main
├── Gear tab      → parameter grid + canvas viewport + [Export DXF]
└── Geartrain tab → train header (inputs; ratio/speed/torque as outputs)
                    + [Add stage ▾: Spur | Worm | Planetary]
                    + one collapsible section per stage
```

Per Q7 the Gear Calculator is **single-gear**: the "Coprime?" output and the
"same input box for all gears in a stage" notes do not apply there and are
dropped. Those remain in the Geartrain stages, where there is a mating gear.

The viewport draws the profile with the first tooth centred up, with pan/zoom and
toggleable reference circles. Deleting a tab confirms first; deleting the last
tab creates a fresh default one.

### 8.1 Additions to the specification's field list

Two things the spec does not list are added, both read-only outputs, both cheap
once the data they need is present:

| Where | Output | Why |
|---|---|---|
| Stage, beside `Ratio` | **Contact ratios** — transverse `ε_α`, overlap `ε_β`, total `ε_γ` | The spec has helix-angle inputs but no way to see whether they bought full axial overlap. `ε_β < 1` is flagged: the gear is helical in form but still transfers load like a spur gear. §4.5 |
| Stage, per gear | **Provenance marker** on each material property | The library ships estimates as well as measurements and must not present them alike. §6.1 |

Both are outputs only, so §3.1 is untouched — nothing new becomes state.

The material properties themselves are shown as **editable fields seeded from the
library**, greyed until edited and un-greyed once they are, so a user can tweak a
value for their own case without authoring a library file. The overrides live in
the input state, which is what keeps outputs a pure function of inputs.

---

## 9. Testing

The prior work's central idea carries over intact:

> Bound the profile **from both sides**. Penetration alone is insufficient — an
> arbitrarily undersized profile passes it trivially. Only penetration *and*
> deviation together pin the profile down uniquely.

Ported to Rust using the analytic `tooth_sdf` the handoff recommends reinstating
(exact, ~2× faster, removes the polyline chord-error floor). In Rust this should
run thousands of cases in `cargo nextest`, against the 44 that fit the Python
budget.

On top of that:

1. **Analytic cross-checks** — every formula with a textbook special case is
   tested against it: span (§4.6), undercut shift reducing to `h_w − z sin²α/2`,
   backlash reducing to `2Δa tan α_w`, Buckingham efficiency.
2. **Independent-construction checks** — the strongest kind, and the one that
   caught my own error: over-pins is verified by measuring the actual distance
   from the computed pin centre to the generated involute, not by comparing to a
   table.
3. **Invariants** — thickness modification does not move the centre distance;
   `b_min` is independent of the `b` used; backlash is zero at nominal centre
   distance; planetary meshes share one centre distance to 1e-12;
   `z_r = z_s + 2z_p` ⟹ `x_p = 0`.
4. **Transcription checks** on the JGMA table — row lengths and preferred numbers
   (§4.6.1).
5. **Regression fixtures** — pinned numbers for reference gears so refactors fail
   loudly. The handoff's `legacy_clamp=True` stays as the *negative* fixture,
   proving the suite still detects the old bug.
6. **Property tests** — random valid parameters must give a simple closed curve
   with monotone radius. Note: *not* monotone angle — undercut profiles are
   legitimately re-entrant, a misconception that cost the prior work 161 false
   failures.

---

## 10. Specification decisions and what is still open

| Q | Resolution |
|---|---|
| Q1 | Output speed/torque are **outputs**. Only input speed and input torques are inputs. §4.9. |
| Q2 | Crossed helical is not first-class, but is unified with worm gearing as crossed-axis screw gearing (§4.5.1). Point contact and the absence of an accepted bending model are the real limits; revisit after the worm stage exists. |
| Q3 | Working depth = the depth at which the undercut question is asked. **Revision 1 was wrong**; corrected in §4.3, and it now reproduces the classical z=17 result. |
| Q4 | JGMA 116-02 extracted and characterised (§4.6.1) — a banded lookup table. Tooth thickness tolerance deferred; nominal-only outputs, with `Option` fields left in place. |
| Q5 | Two-pin and three-pin, odd and even, all four closed form and independently verified (§4.6). |
| Q6 | Ring search is provably complete because required planet shift is strictly monotone in ring tooth count, with a closed-form bracket (§4.8). |
| Q7 | Gear Calculator is single-gear; mating-gear references dropped (§8). |

**Applied corrections**: profile shift range `|x| ≤ 2`; output torque unit Nm;
worm starts `≥ 1`; minimum tip width compared against `dedendum × module`;
gear-tab automatic toggles are Inputs; default material "4340 Hardened Steel".

**Everything from the original Q1–Q7 review is settled** and folded into the
sections above. What remains open is listed here in full, so a fresh reader does
not have to hunt for it.

| Open item | Where | Blocks |
|---|---|---|
| Mesh-phase coefficient that sets the optimal λ | §4.10, appendix | only the angular-profile-shift milestone |
| Sinusoidal `x(θ)`, or another interpolation? | §4.10 | same |
| What the eccentric mechanism can physically follow | §4.10 | same |
| A coupled glass POM grade, if one is wanted back in the library | §6.4 | nothing |
| Tooth thickness tolerance (JGMA 1103-01, unavailable) | §4.6 | min/max on span and over-pins only |
| Crossed-axis contact model | §4.5.1 | milestone 10 |

**Deliberately deferred, with a written rationale:** load sharing — see the end
of §4.7. It is not an omission; the measurement says it buys 0.0–0.2% for a
worst-case number, and the conditions that would change that are named.

**Pressure angle range.** Keeping 60° as specified. Nothing found so far forces
branching: the only casualty of high pressure angles is that more `(z, x)`
combinations become geometrically impossible, and the existing clamp mechanism
already reports those with a readable note. The one thing high angles *did*
break was the involute inversion, and that is fixed properly by safeguarding
rather than by narrowing the range. I will flag it if the ring-gear or
crossed-axis work changes this.

---

## 11. Milestones

Each ends with something runnable and tested; the mathematics precedes the UI so
it can be validated in isolation.

| # | Milestone | Gate |
|---|---|---|
| 0 | ✅ **Scaffold** — workspace, wasm target, `gear-cli`, Vite/Svelte shell, flake `.#web` | **met** — `nix flake check` green; the wasm binary returns bit-identical numbers to the native build |
| 1 | ✅ **Geometry core** — port `gear.py` + thickness modification + rack-simulation suite | **met** — penetration 2.1e-15 mm, deviation 6.2e-4 mm over 1080 cases |
| 2 | ✅ **Primitives & metrology** — safeguarded `inv⁻¹`, centre distance, backlash, span, pins, JGMA table | **met** — span reproduces the textbook form to 1e-12 mm, backlash matches a direct computation to 1e-16 mm, pin tangency verified to 3e-10 mm against the generated flank |
| 3 | ✅ **Gear Calculator UI** — sidebar, tabs, parameter grid, canvas viewport, DXF export | **met** — the UI's own request path produces a DXF `ezdxf` reads back with the right geometry |
| 4 | ✅ **Materials** — TOML library, import/export, the preloaded materials | **met** — every value carries a cited primary source and a `basis`; the library round-trips through TOML unchanged and satisfies cross-family consistency laws |
| 5 | ✅ **Mesh & strength** — contact path, bending, load-to-stress path, efficiency, Hertz, face width | **met** — both bending constructions converge to their own closed-form rack limits; Hertz agrees with the contact-half-width route to 1e-12; efficiency matches a numerical average of the instantaneous loss to 1e-10; `b_min` is independent of the face width it was evaluated at |
| 6 | ⬜ **Spur stage** — accordion, train accumulation, torque/cycle propagation, face width and material as inputs, overlap ratio `ε_β`/`ε_γ` as outputs (§4.5) | a two-stage train computes end to end; `ε_β = 0` exactly for a spur stage |
| 7 | ⬜ **Worm stage** — screw-gear model, lead angle, self-locking, axial backlash | self-locking threshold matches the closed form |
| 8 | ⬜ **Ring gear geometry** — internal profile, shaper trochoid, interference checks | own rack-equivalent validation |
| 9 | ⬜ **Planetary stage** — ring tooth search, planet shift solve, layout checks, Pennestrì efficiency | common centre distance to 1e-12; all six drive modes |
| 10 | ⬜ **Crossed-axis spur** — reuse `screw.rs`, point-contact Hertz | Q2 revisited with the worm model in hand |
| 11 | ⬜ **Polish** — train import/export, confirmations, error surfacing, docs | — |

Milestones 1–3 de-risk everything else: they prove the geometry is right, the
mathematics is testable in Rust, and the wasm-to-Svelte pipeline works. I would
review again at the end of milestone 3.

### Where milestones 0 and 1 actually landed

Both are complete. Results, all reproducible from the repository:

| Check | Result |
|---|---|
| Port vs. the Python reference, 1188-case grid | worst deviation **7.5e-14 mm**; zero undercut/severed flag mismatches |
| Two-sided rack verification, 1080 cases | penetration **2.1e-15 mm**, deviation **6.2e-4 mm** |
| Fillet is the tip-round envelope | worst **1.5e-6 mm** |
| Analytic cutter SDF vs. independent polyline | worst **5.4e-7 mm** (the polyline's own chord error) |
| Test suite | 31 tests, 27 s |
| `nix flake check` | green (build, clippy `--deny warnings`, fmt, nextest) |
| wasm boundary | returns bit-identical values to the native build |

Two decisions taken during the port, both recorded in the code:

1. **`tooth_sdf` was reinstated**, as the prior handoff recommended. The cutter is
   now an exact signed distance function rather than a discretised outline, which
   removes the ~3e-6 mm polyline chord floor and deletes the point-in-polygon
   containment test — by the handoff's own account the least trustworthy step in
   the suite. The polyline survives purely as an independent cross-check.
2. **The tuning constants were re-derived rather than inherited.** Sampling counts
   come from a stated tolerance, and the phase resolution is justified by a
   convergence test (`phase_resolution_has_converged`) instead of a chosen number.

One item from milestone 0's original scope was **not** attempted and is not
needed yet: the geartrain and material-library types. Those belong to milestones
4–6 and nothing in the scaffold anticipates them, per §3.1 — inputs are the only
state, so adding them later is additive.

---

---

## 12. Corrections made during implementation

Every one of these was a claim in an earlier revision of this document that
turned out to be wrong. They are recorded rather than quietly edited, because
the pattern is more useful than any single entry: **the errors that survived
longest were the ones that looked reasonable and were never checked against
something independent.**

| § | What was wrong | How it surfaced |
|---|---|---|
| 4.3 | Working depth modelled as a constraint on the form radius | Would not reproduce the classical z=17 result; the correct reading substitutes `h_w` for `h_f` in the cutter-depth term |
| 4.5 | Approach and recess lengths each subtracted `a_w sin α_w` | Both came out **negative**; each must subtract its own gear's `r′ sin α_w`, and only their sum uses `a_w` |
| 4.6.1 | "The standard scale's grade 4 is tighter than the fine scale's grade 0" | False — 7 against 6.3. The argument it supported (non-monotonic merged ladder) survives and is now stated from the transcribed data |
| 4.6.1 | JGMA diameter bands stored half-open as printed | Left a **gap**: a gear of exactly 12.00 mm got no tolerance at all |
| 4.7 | 30° tangent treated as the model rather than an approximation | Its tangents miss the load point by 11.8% at z=9; the Lewis parabola is the construction the cantilever model implies |
| 4.7 | Parabola tangency searched on the fillet only | No solution at all above z≈150 — on large teeth it touches the **flank** |
| 4.7 | ISO `Y_S` applied to a flank tangency | **17% discontinuity** at z=150→151 while `Y_F` moved 0.03%; the correction is a notch factor and there is no notch there |
| 4.7 | "Rack-generated fillets keep `q_s` in range" | False at large z — 10.3 at z=300 with a sharp cutter |
| 4.5 | Mesh efficiency without the `/ε_α` | Implicitly let every engaged pair carry the full load, so the mesh transmitted `ε_α F_n` and the loss came out too large by exactly the contact ratio. Caught by a numerical average of the instantaneous loss |
| 4.7 | Hertz checked at "the inner point of single-pair contact (usually the pinion's worst case)" | Label-dependent: the relative-radius peak moves to the other side when gear 1 is the wheel, so one physical mesh gave two answers. Both boundaries are now checked |
| 4.7 | `Load` stored a force in a field named `normal_force` | The value was the **transverse** `F_bt = T/r_b` while the name asserted the normal plane. Numerically right for spur, but the name would have survived a refactor its meaning did not. Now stores torque, with each projection named at its point of use |
| 4.7 | Helical contact used the transverse force, face width and curvature throughout | Three separate `cos β_b` factors were missing. They nearly cancel — the net is `√(cos β_b)` — so the error was small but the model was wrong in three places at once rather than right |
| 4.7 | Helical bending measured `Y_F` on the transverse section and divided by `m_n` | Mixes planes; under-predicts by about `cos β` (6 % at 20°, 13 % at 30°). Now measured on the ISO virtual spur gear `z_n = z/cos³β` |
| 6 | Two-point Basquin S-N law per material | The data does not exist — no polyamide grade publishes any fatigue figure, and POM's is a printed graph. Replaced by peak and cyclic allowables, §6.2 |
| 6 | `yield_strength` as the single strength field | Glass-filled grades have **no yield point**; their datasheets report stress at break. Renamed to an allowable, with `ultimate_measure` recording which quantity it is |
| 6 | "1215 Hardened Steel" assumed a valid entry | 1215 is ~0.09 %C and cannot be through-harden; only carburised, giving a hard case over a soft core that one scalar cannot represent. Both 1215 entries dropped |
| 6 | Delrin 570 assumed a reasonable "POM GF20" | It is glass *filled*, not *reinforced* — **25 % weaker** than unfilled Delrin. Entry dropped; only a glass *coupled* grade would belong |
| 6 | PA6/PA66 stiffness "roughly halves" when conditioned | Understated: unfilled PA6 modulus falls 3000 → 1000 MPa, a factor of **three** |
| 4.10 | Read as an axial taper (beveloid) | It is an *angular* variation; the beveloid treatment was withdrawn entirely |
| 4.10 | "No changes to the generator" (beveloid reading) | Did not survive the correction above |
| — | Involute inversion by series seed + Newton | **Diverges above ~60°**, inside the allowed pressure-angle range; needs safeguarding |

Two process notes worth carrying forward:

- **A green local test run does not imply a green build.** Twice, the working
  tree held something a fresh checkout does not: a data file crane's source
  filter stripped, and generated wasm bindings that are gitignored. Before
  pushing anything that adds a generated or non-`.rs` file, delete the generated
  directory and re-run the step that consumes it.
- **Independent checks caught what assertions did not.** The rack limits, the
  pin-tangency measurement against the generated flank, `ezdxf`, and the
  continuity sweep each found something that self-consistent tests had passed.


## Appendix — verification log

Everything marked **[verified]** was checked numerically before being written
here. Revision 2 additions are marked ★.

| Claim | Check | Result |
|---|---|---|
| `j_t = 2a′(inv α′ − inv α_w)` | vs. direct tooth thicknesses at operating pitch circles, 3 configs | 3e-16 mm |
| `x_s = π(k−1)/(4 tan α_n)` | vs. direct `s_n`, 36 combinations | 4e-16 mm |
| `inv⁻¹` seed + Newton | residual per step, α = 10…75° | machine precision ≤ 60°, **diverges at 75°** → safeguard |
| ★ `inv⁻¹` domain | `v < 0` reachable from ordinary planetary inputs | must return `Option`, not NaN |
| ★ Corrected `x_min(h_w)` | `z_min` at α=20°, ρ=0 | h_w=1 → 18; h_w=1.25 → 22; matches `2h_w/sin²α` |
| ★ Cutter radius effect | same, ρ=0.38 | z_min drops 18 → 13 |
| ★ Over-pins `r_M` | min distance from pin centre to generated involute, 4 gears | equals `d_p/2` to 3e-10 mm |
| Planetary `dg/dx_p` analytic | vs. central differences | 6+ digits |
| Planetary Newton | from `x_p = 0`, z = 17/17/53 | 3.6e-15 mm in 4 iterations |
| ★ `x_p` monotone in `z_ring` | swept z_r = 45…61 | strictly increasing where defined |
| ★ `z_r = z_s + 2z_p` ⟹ `x_p = 0` | 17/17/51 | −0.000000 |
| ★ Planetary domain bracket | swept z_r = 45…61 | only 48–54 admissible; rest genuinely impossible |
| ☆ JGMA transcription | text extraction vs. rendered page images at 200 dpi | both pages confirmed; raw text extraction had column mangling in the lower grades |
| ☆ JGMA scale comparability | page 1 vs page 2, module 1.0–1.6, d = 12 mm | page 2 grade 4 (7/20 μm) is tighter than page 1 grade **0** (6.3/18 → 22/71 at grade 4); scales are not comparable |
| ✦ Profile shift leaves the involute fixed | `x`: −0.3 → +0.6, spur and helical | `Δr_b = Δr = Δα_t = 0` exactly; only `ψ_b` moves |
| ✦ `ψ_b` linear in `x` | `x` = 0 … 1 in steps of 0.25, three pressure angles | successive deltas identical to 1e-9; matches `m tan α_n / r` |
| ✧ Tip envelope vs displaced circle | `e` = 0.1, 0.25, 0.5 mm | limaçon; deviation = `e²/2ρ` exactly (0.0005 / 0.0033 / 0.0132 mm) |
| ✧ Flank generation sweep | z = 17, α = 20° | φ = −20.9° … +16.0°, a **1.7 tooth-pitch** sweep |
| ✧ `x` variation within one tooth | same, θ = 0…180° | up to **0.63 × Δx** at quadrature — per-tooth constant `x` is not viable |
| ✧ Distinct teeth under mirror symmetry | z = 9, 12, 17, 40 | 5, 7, 9, 21 |
| ✪ Driving-flank pitch error, E2 / E3 | z = 17, α = 20°, e = 0.1 / 0.25 / 0.5 mm | E3 forward exactly **0.000 μm**; E2 = 25 / 63 / 125 μm, linear in e |
| ✫ E1 moves no flank | e = 0.25, 0.5 mm | drive and coast pitch error exactly 0.000 μm; thickness variation exactly 0.000 mm — withdrawn |
| ✫ E3 under reversal | same three eccentricities | 50 / 125 / 250 μm — exactly **2×** E2, and unbounded relative to its own forward error |
| ✫ Minimax over indexing λ | λ = 0, 0.5, 1 | worse-direction error minimised at **λ = 0** (E2); λ = 0.5 gives 31/94 μm, λ = 1 gives 0/125 μm |
| ✫ Both flanks uniform ⟹ uniform thickness | algebraic, two lines | exact; base tooth thickness spread is 339 μm at e = 0.25 mm, so the two are incompatible |
| ✰ Poisson's ratio sensitivity of `σ_H` | swept ν = 0.33…0.44, polymer-on-steel and polymer-on-polymer | **±2.5 %** total — the polyamide `ν` estimates are low-consequence |
| ✰ Steel endurance ratio `0.5 × UTS` | vs. the published annealed 4340 fatigue strength | 345 against 330 MPa, within 5 %. **Not** corroborated at the hardened temper — no published figure exists there, and the ratio is known to fall away above ~1400 MPa UTS, so that entry is flagged optimistic |
| ✰ Brass `ν` from published moduli | `ν = E/2G − 1`, CDA `E` = 14 000 ksi, `G` = 5 300 ksi | 0.321 — derived, not estimated |
| ✰ Material library round-trip | default library → TOML → parse, compared field by field | identical |
| ✰ Polyamide family consistency | Grilon BS / BG-30 S / BG-50 S / Grivory GVX-7H, three separate datasheets | stiffness and density strictly increase with glass; moisture gap strictly closes — a column misread would break it |
| ✦ Mesh efficiency closed form | vs. a direct numerical average of `μ\|ξ\|(1/r_b1+1/r_b2)` over the path, five meshes | agreement to 1e-10 relative; caught a missing `/ε_α` |
| ✦ Hertz `σ_H` | vs. the contact-half-width route, `b_h = √(4F'R/πE*)` then `p_max = 2F'/(π b_h)` | 1e-12 relative — an independent path through the same physics |
| ✦ `ρ₁ + ρ₂` along the path | swept 21 points, z = 17/43 | constant at `a_w sin α_w` to 1e-9 |
| ✦ `b_min` independent of `b` | face widths 1, 5, 12.5, 100 mm, bending and contact | identical to 1e-9 — the check that catches a stress not scaling with `b` |
| ✦ `σ_H ∝ √E*` | steel `E*` = 103 723 MPa vs polymer 1 700 MPa | ratio matches `√(E*₁/E*₂)` to 1e-9 |
| ✦ `σ_H` independent of gear labelling | five meshes including 43/17 and 60/13, labels swapped | identical to 1e-12; caught the inner-boundary-only defect above |
| ✧ Force projections mutually consistent | β = 0, 15, 30°: `F_t`, `F_bt`, `F_bn` against each other | exact; `F_bn = F_bt` only at β = 0 |
| ✧ `F_bt` shared across a mesh | `Load::across_mesh`, three ratios | unchanged to 1e-9; torque scales as `z₂/z₁`, round trip exact |
| ✧ Virtual spur gear | β = 10…45° | `z_n = z/cos³β` to 1e-12, `α_t = α_n`, `m_t = m_n`, β = 0; identity for a spur gear |
| ✧ Normal vs transverse section | β = 0, 15, 30° | identical at β = 0, diverging monotonically thereafter — the helical bending error |
| ✧ Helical `σ_H` ratio | β = 10, 20, 30° against the same mesh without the plane change | exactly `√(cos β_b)` to 1e-12 |
| ✧ Spur results unchanged by the refactor | `gear-cli strength 17 43 2.0` | bit-identical: `σ_F` 69.2/63.4, `σ_H` 692.7, ρ 1.723 |
| ✧ Closed-form HPSTC roll | `u_tip − (ε_α−1)p_b/r_b` vs the path construction, seven meshes | exact (≤ 5.6e-17) |
| ✧ Helical efficiency | numerical average including `1/cos β_b`, five meshes × β = 0, 12, 25° | 1e-10 relative |
| ✧ Virtual gear identity at β = 0 | rebuild vs the original gear | bit-identical, by construction not by branch |
| ✧ `ε_αn = ε_α/cos²β_b` vs a measured virtual pair | β = 0, 10, 20, 30°, two meshes | exact at 0; 0.03 / 0.11 / 0.20 % apart — the construction's own limit, not an error |
| ✧ What that gap costs | perturb `ε_αn` by the observed spread, β = 30° | `Y_F` moves **0.38 %** |
| ✦ `σ_F` composition vs. §4.7's load-case table | z = 17/43 at HPSTC, `Y_F · Y_S` | 2.9415 against the recorded 2.9416 |

The marks record which round of review each check came from; they are kept only
so a claim can be traced to the work that produced it.

**Not verified, and deliberately not quoted.** The mesh-phase coefficient of
§4.10 — how far a commanded centre-distance change shifts the drive-flank phase.
A trial model was built and **rejected**: it failed to reproduce
`j_t = 2a′(inv α′ − inv α_w)`, disagreeing by 10–25% and worsening with `Δa`.
The principle is not in doubt (the rack limit gives `δ tan α` per flank, which is
the §4.1 thickness relation), but no figure for it, and no optimal λ, appears in
this document. Reproducing the backlash law is the gate any replacement must pass
first.
The beveloid entries are withdrawn (§4.10 is an angular, not axial, variation);
the two `✦` rows are general facts about profile shift and remain valid. The `✧`
rows remain valid as measurements of the physical hob process — §4.10 explains
why that process must be corrected rather than modelled faithfully.
