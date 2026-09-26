## T01 — Validate what comes in, once, where it comes in

**Why.** Nothing checks external input after it has been parsed. The TOML reader, the wasm entry points and the core constructors trust every index, every float and every physical constant. The TypeScript `validate()` on the gear tab is the only gate, and rule 5 requires the core to refuse. Three failure modes follow. (1) Out-of-range indices panic, and a `carried_by` cycle grows memory without bound: a one-character edit to a file makes `import_train` hold 2 GiB and trap after about 3.2 s. (2) Input that describes nothing is solved or refused under the wrong reason. All 29 of 29 float fields accept `nan`. A negative friction gives a mesh efficiency of 1.02 beside a path efficiency of 0. A Poisson ratio of 0.7 raises contact stress by 34 % (154.46 → 206.99 MPa) with no note. Module 0 is refused as `no_contact`, `no_common_distance` or `screw_not_positive`, depending on the preset. (3) A trap loses its message, several `core.ts` wrappers swallow it, and after about 1000 traps every call fails until the page is reloaded. The fix is one validator per kind of input: graph structure, finite floats, gear parameters, train scalars and materials. Each is called at every place input enters, refuses with a catalogue key that names the field, and is held by one fuzz law over every preset's document.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T01.1 Bound the carried-axis closure | graph-ops#0, wasm-boundary#0 (loop) | medium | S | — |
| T01.2 `Shape::validate` / `Train::check` in production, called at every entry | graph-ops#1, shape-a#7, wasm-boundary#0, added3#14 (unverified), lens-tests-train#5 | medium | M | T01.1 |
| T01.3 Refuse unknown keys everywhere; delete the addendum shim | gear-io#7, added2#90 | low | S | — |
| T01.4 Finite floats at the serde boundary; round-trip law over every builder | gear-io#4, added2#35 | low | S | T01.2 (harness) |
| T01.5 One accessor for the guarded pressure angle | ablate-constants-geometry#1 | low | S | — |
| T01.6 `GearParams::check` at every boundary; `Gear::profile` never indexes empty | primitives#0, lens-errors-policy#0, lens-numerical-robustness#4 | medium | M | T01.5 |
| T01.7 Crossed contacts meet the parallel path's flank predicates | added2#78, mesh-contact#9 | medium | S | T01.6 |
| T01.8 `Shape::check_inputs` for the train's scalars (friction, count, rim, face) | lens-numerical-robustness#5, lens-errors-policy#6, web#21, lens-errors-policy#0 (train half) | medium | M | T01.2, T01.6 |
| T01.9 `MaterialLibrary::check` in gear-core, on the resolved material | lens-errors-policy#5, gear-io#5 | medium | S | — |
| T01.10 Bound `points_per_tooth` at the boundary | wasm-boundary#14 | info | S | — |
| T01.11 Panic hook, re-instantiation, no swallowed failures in `core.ts` | wasm-boundary#1 | medium | M | — |
| T01.12 Carry each mesh's distance index in `BuiltMesh` | shape-b#13 | info | S | T01.2 |

Every refusal below crosses the boundary as a `Note` key with values, never as English (rule 2). The single refusal channel is T02.1 and T02.5 [lens-errors-policy#12, wasm-boundary#5, lens-architecture#2]. Until that lands, each new refusal here is a `TrainError`/`MeshError` variant with a `note()`, as the existing ones are.

### T01.1 Bound the carried-axis closure
**Change.** At `train/graph.rs:288-298` ("a carried axis brings its carrier's"), push the axis only when it is new: `if let Some(a) = axis_of_body(carrier) { if !axes.contains(&a) { axes.push(a); } }`. The closure then runs at most `axes.len()` times on any input.
**Proof.** Add a unit test in `graph.rs` on `Preset::Spur.build()` with `axes[0].carried_by = 1` (self-carried) and with `axes[0].carried_by = 2, axes[1].carried_by = 1` (a two-axis cycle). Both must return from `parts()` with axes `[0,1]`. Before the fix the test aborts with `memory allocation of 2147483648 bytes failed`. With the fix it passes with the rest of the suite (680/680) [graph-ops#0]. Add the same two cases as graph files, and a staged file with a self-carried axis for `gear-cli convert`.
**Notes.** After this fix the files load and fail at solve time as `NoCommonFrame`. That is the wrong name, which T01.2 corrects.

### T01.2 One structural validator, in production, at every entry
**Change.** Promote `edits::well_formed` (`edits.rs:833`, `#[cfg(test)]`) to `pub fn Shape::validate(&self) -> Result<(), Malformed>` and `Train::check`. Keep one invariant set: the edit laws and the loader must share it. It checks:
- mesh `a`/`b` < members; every `member.body` listed; `body.axis` < axes; distance axes < axes; one `Distance` per unordered axis pair; `count ≥ 1` in structure; coupling ends distinct and listed;
- `carried_by` is ground (0) or a listed body, never a body on the axis it carries, and acyclic;
- every hold, load and `Intermittent` `duty.at` names a listed body (the `speeds[port]` panic at `mod.rs:4527`);
- body numbers dense 1..=N. `from_toml` normalises a gap with `prune()` and sets `adjusted = true`, as it already does for relief. The other design is to key `Train::nodes` (`conditions.rs:361`, `max_body()+1`) by the distinct bodies listed, which removes phantom nodes by construction [lens-tests-train#5]. Choose one. Prune is the smaller change.

Each `Malformed` variant has its own key (`error.malformed_mesh_end {mesh, member}` and so on). A member on an unlisted body and a distance to a missing axis are refused under their own keys, not as `NoCommonFrame`/`NotAMesh`/`Overdetermined`. Call `validate` in `gear_io::train::from_toml` and `convert` before `relieved()`, and in every gear-wasm entry that deserialises a `Train` (`solve_train`, `edit_train`, `offers`, `preview_edit`, `relieve`, `relieve_case`, `adopt_member`). Shapes the core builds itself need no check on every keystroke. Add a `debug_assert!(validate().is_ok())` inside `Shape::apply`'s `transact`, so the edit laws exercise it.
**Proof.** Write this law first and see it fail. `tests/malformed.rs` builds every `Preset::ALL` document with `to_toml` and edits each index leaf to `len`, `len+1`, `len+5` and 99. It adds a self-carrier and a two-axis cycle, and it removes one body number to leave a gap. Each variant is run through `from_toml` and then `solve_train` under `catch_unwind`. Each must return `Ok` or a `Malformed` refusal that names that field, never panic, and a gapped file must solve identically to its dense twin. The current code fails it: the fuzz table counts 39 panics on `bodies/#/axis`, 19 on `meshes/#/a`, 21 on `meshes/#/b` and 9 on `duty/at` [graph-ops#1]. Also add a `tools/wasm_probe.mjs` step that imports a document whose mesh names a member past the list and one with a self-carried axis, and records the refusal key. T01.4, T01.6, T01.8 and T01.9 add rows to this same harness.
**Notes.** This removes the index panics at `shape.rs:1091`, `graph.rs:236`, `graph.rs:291` and `mod.rs:4527`. Where a duty sits on a held body (T13.3, T13.5 [train-mod-b#3]) and a mesh written ring-first (T05.8 [lens-unification#1]) are neighbouring invariants; once they are decided, they go into this validator. [edit-ops#0] and [graph-ops#2] are panics that edits produce, fixed in T13.2. The `debug_assert` catches them.

### T01.3 Refuse unknown keys everywhere; delete the addendum shim
**Change.** Add `#[serde(deny_unknown_fields)]` to `TrainDocument` (`gear-io/src/train.rs:297`). Delete the untagged `Either::WasAuto { manual }` arm of `coefficient` (`train/mod.rs:1071-1081`) and deserialise `addendum` as a plain `f64`. Add the old `{auto, manual}` form to the refused formats that `train.rs` lists. Decide `ground_if_null` (`shape.rs:121`) in the same change [ablate-features#11], so that the "no compatibility shim" paragraph at `train.rs:249-266` is true.
**Proof.** Rewrite `a_field_the_shape_no_longer_has_is_refused_and_named` so it walks every table of the serialised every-preset document and inserts `zz_unknown = 1` in each. Every insertion must be refused with the key named. Today the root is accepted (`true <root>`), and `addendum = { manual = 1.2, junk = 7 }` gives `Ok(1.2)`. The walk also stops a future struct from missing the attribute.

### T01.4 Finite floats at the serde boundary
**Change.** Add a `Finite` deserialiser (`deserialize_with = "finite"`) on `Auto<f64>`'s manual value and on every plain `f64` in the train document, so that TOML `nan`/`inf` fails at parse time with the path. It crosses as `error.document_value_not_finite {field}`, and the refusal stays in gear-io. On the write side, `arrangements::worm` must never seed a NaN. Seed the helix only where `z₁·m < d₁`, or from `acos(min(1, z₁·m/d₁))`. The named-constant cleanup of the 7 mm diameter belongs to [crossed-worm#20, lens-magic-numbers#5]. The tests at `crossed.rs:357/670` should build `arr::crossed` rather than `worm(17,23)`.
**Proof.** (a) In T01.2's harness, replace each float leaf with `nan`, `inf` and `-inf`. Each must be refused with its path. Today all 29 of 29 fields are accepted, a NaN distance is refused as "below the base-circle limit", and a NaN torque solves as `case_nothing_drives`. Finite values still round-trip bit-exact. (b) A law that every `Preset::ALL` shape and every `arrangements::` builder output round-trips through `serde_json` unchanged. It fails today on `worm(17,23).with_first_helix(45.0)` (`invalid type: null, expected f64`).

### T01.5 One accessor for the guarded pressure angle
**Change.** Add `GearParams::normal_pressure_angle_rad() -> (f64, bool)`: the angle raised to `guard::MIN_PRESSURE_ANGLE_DEG`, and whether it was raised. The guard is `<`, so exactly 0.5° is not reported as raised. `thickness_shift` (`params.rs:104`, which today takes `tan` of the raw angle, so α = 0, k = 1 gives 0/0) is computed from it. Every reader switches in one change, or the mesh's inv α_w relation and the tooth stop agreeing:
- the five copies, `tooth.rs:364`, `tooth.rs:402`, `auto.rs:115`, `auto.rs:304` and `auto.rs:662`, which are then deleted;
- the readers of `thickness_shift`: tooth.rs:833, auto.rs:308/678, mesh.rs:746, gear.rs:922/936, ring.rs:244/1032 and shape.rs:522/1175/1359;
- the readers that skip the guard today: ring.rs:218 (the ring then raises `clamp.pressure_angle_raised` as the tooth does), `BasicRack::new` through `Shape::rack_of`, shape.rs:525 and 3526, and the raw recomputation in `gear-wasm/src/lib.rs:556`.

The policy for the angle is rule 5's: α_n ≤ 0 describes no shape and is refused at every boundary (T01.6); 0 < α_n < 0.5° is clamped with the note. The admissible range stays (0, 90°), and reference.md:149, which says each bound sits where the generator's guards begin to clamp, is corrected to name the clamp below 0.5°.
**Proof.** A law for α ∈ {0.1, 0.3, 0.49, 0.5, 0.6}° and k ∈ {0.8, 0.9, 1, 1.1, 1.2}: `Tooth::new` gives a finite, non-empty profile identical to the tooth at `max(α, 0.5°)`, and s_t equals its value there. Today k = 1.2, α = 0.3 gives ψ_b 0.1232 against 0.1109 at 0.5°, and s_t is −7.4 % at 0.3°, k 0.9, and −44 % at 0.1°. Inside the core the accessor still raises α = 0, so k = 1 gives no NaN (today 0 points). A Ring at 0.3° reports α_n = 0.5° with `clamp.pressure_angle_raised`; today it gives 0.3° and no note. A grep check that `MIN_PRESSURE_ANGLE_DEG` is read only in params.rs. With the law in place, perturbing the constant by ×1.1 must be caught.

### T01.6 `GearParams::check` at every boundary
**Change.** Add `GearParams::check(&self) -> Result<(), Note>`, built from the invariant `Bound`s of `auto::ranges_at_shift`, so that the bounds stay written in one place. It refuses m ≤ 0, `teeth == 0`, |β| ≥ 90°, α_n ∉ (0, 90°), k ∉ (0, 2), and any non-finite shift, addendum, dedendum or root radius, under `error.input_out_of_range {field, bound}`. Call it in gear-wasm `resolved_params` (`lib.rs:656`, which `solve_gear`, `gear_profile`, `export_dxf` and `solve_ring` share), in gear-cli argument parsing and in the shape's member validation. A train with module 0 is then refused under that key rather than as "no contact". Do not change `Tooth::new`/`Gear::new` to return `Result`: that touches hundreds of call sites. Instead make `Gear::profile`/`Ring::profile` return an empty outline when a half-profile is empty (today `&r[1..]` at `gear.rs:564` panics). Also make `Mesh::new` refuse z = 0 with `MeshError::NoTeeth`. Keep the test's frozen reference copy at `gear.rs:1386`.
**Proof.** A lone-gear law sweeps each field to 0, −1, NaN, ±inf and its open bound: every entry point returns the refusal and never panics. Today `profile` panics for module 0, teeth 0, α 0, k NaN, x NaN and module NaN. `export_dxf` returns 15.6 MB at module 0 and 20.6 MB at α = 90°. β = 90° gives r_a = 1.39e17 mm. Add `tools/wasm_probe.mjs` fixtures for `gear_profile` and `export_dxf` with module 0 and teeth 0, and a unit test that `Mesh::new` with z = 0 is `Err`.
**Notes.** The open interval (0, 90°) is not where the tooth stops existing. Above arctan(π/(4h_f)), which is 32.1° at h_f = 1.25, the outline already self-intersects [primitives#0]. That region is a geometry predicate (T01.7), not a bound. The documented refusals in `rationale.md` should be re-checked once this lands [lens-errors-policy#2]. The panic-lint question is [lens-errors-policy#19].

### T01.7 Crossed contacts meet the parallel path's flank predicates
**Change.** In the shape's crossed-contact build (`shape.rs:515-531`, beside `Screw::new` at `screw.rs:216`), refuse `NoRootSection` when a member has no bending section, and `OutsideInvoluteDomain` when r_b ≥ r_a, exactly as the parallel path does. Do not add a new rack-formability bound. State the precondition r_a ≥ r_b in `ContactPath::new`'s doc with a `debug_assert`, and leave its `None` unchanged: returning `None` there would turn today's accurate `NoRootSection` into "no contact".
**Proof.** A law over every preset that sweeps α_n towards 90°: the result is either a refusal naming the flank/root, or a finite rating with r_b < r_a. Today the worm at 89° is rated at 2276.29 MPa contact stress with no bending section, while the spur is refused from 85°.

### T01.8 One scalar gate for the train
**Change.** Add `Shape::check_inputs`, beside `check_parts`, which returns `TrainError::Input { piece, field, key }`. Take each numeric bound from the same source as the gear's `Ranges` (`auto.rs`). The rules:
- friction (sliding and static) ≥ 0;
- `Axis.count ≥ 1` (delete the silent `.max(1)` at `shape.rs:463` once the gate exists);
- rim thickness > 0;
- `min_contact_ratio > 0`;
- a fixed face width > 0;
- cutter teeth ≥ 1;
- teeth ≥ 2 on a parallel line contact;
- each member's `GearParams::check` (T01.6).

Export `MeshRanges` beside the member ranges, so that `TrainPanel.svelte:673-674` shows the core's bound instead of an unbounded `numberField`. Where `contact::efficiency` meets its floor of 0 (μ ≳ 4.8 on the spur), raise a note that the loss model is out of range, instead of returning an exact 0.
**Proof.** A law over every preset: each out-of-range scalar in the list gives `TrainError::Input` naming that field and nothing else changes. A companion law: every mesh efficiency is in [0, 1] for μ ≥ 0, including the worm and crossed presets, where the screw efficiency goes furthest above 1. Today:
- spur μ = −0.02 gives mesh η 1.00414, path 0, `load_not_reacted`;
- worm μ = −0.05 gives 1.630;
- static friction −1.16 gives path 0 with no note;
- count 0 is rated identically to count 1;
- layshaft rim 0 gives `Some(NaN)` bending;
- module 0 gets four different keys across ten presets.

`every_add_on_every_preset_solves` must stay green.
**Notes.** Face width 0 now solves to nulls and needs a decision before it is gated [lens-errors-policy#13]. Planetary at friction 20 has both meshes at η 0 and a path efficiency of 1/7; a mesh at η 0 holds, so T11.4's oracle law must decide whether that figure is right. The panel's integer sanitation is [web#5, lens-errors-policy#16].

### T01.9 Material validation in gear-core
**Change.** Add `MaterialLibrary::check()` / `Material::refusal()` in `gear-core/material.rs`, refusing `Implausible { material, field }`. The bounds are all finite, density > 0, E > 0, −1 < ν < 0.5 and 0 < fatigue ≤ ultimate. Call it from `gear_io::materials::from_toml`, from the gear-wasm entry points that take a library, and on the resolved material after member overrides (`material.rs:353`), which never pass through `from_toml`. `every_material_carries_physically_sane_values` becomes a call to the same predicate. Its 0.2 floor on ν is a sanity band, not a physical limit, and does not belong at runtime.
**Proof.** For each shipped material and each numeric field: 0, −1, NaN, ν = 0.5 and ν = 0.7, both in the library and as a member override, are each refused with the material and field named. Today ν = 0.7 is rated silently at 206.99 MPa, and E = 0 is refused as "the teeth never come into contact".

### T01.10 Bound `points_per_tooth`
**Change.** Take `u16` at `gear_profile`/`ring_profile` (`lib.rs:731`, `1014`), and clamp it in `Gear::profile`/`Ring::profile` beside the existing `.max(8)` floor. The better long-term shape is to drop the argument and draw the screen profile to a chord tolerance, as the export does. That also removes the literal 600 from `GearPanel.svelte:253`.
**Proof.** A probe step with n = −1 and 2³¹ returns a refusal. Today it traps (`RuntimeError: unreachable`).

### T01.11 A trap is reported, not swallowed, and never poisons the instance
**Change.** (a) In `gear-wasm`, a `#[wasm_bindgen(start)]` installs `std::panic::set_hook`, which writes the message and location to a `thread_local`, and a `last_panic()` entry reads it. Hooks run under `panic = abort`. (b) In `core.ts`, one wrapper routes every entry point. On `WebAssembly.RuntimeError` it reads `last_panic()` first, then re-instantiates from the cached `WebAssembly.Module` (`initSync`), then surfaces `ui.train_boundary_failed` with the panic text. Remove the silent swallows in `profile`, `ringProfile` (→ `null`), `relieveTrain`, `relieveCase`, `previewEdit`, `offersAt` (→ `[]`) and `editTrain` (non-`ui.` messages → `null`) (the editTrain success-report is T19.8 [web#13]).
**Proof.** A node test runs 5000 trapping calls (mesh `b = 5` `solve_train`), then a good call. The good call must return the recorded answer, and the surfaced error must contain the Rust panic location. Today, after 1000 traps, a good call throws `memory access out of bounds`, and `profile` does the same after 764.
**Notes.** This is defence in depth. T01.2 and T01.6 make the known panics unreachable.

### T01.12 Carry each mesh's distance index
**Change.** Add `distance: usize` to `BuiltMesh` (`shape.rs:2428`), set from the `d` that `build` already finds at `shape.rs:2838`. Index `shape.distances[..]` at `shape.rs:3521`, `3550` and `3566`, and delete the three `distance_of(k).unwrap_or(0)`. `build` already refuses a mesh with no distance, so this is hygiene and removes three linear searches.
**Proof.** clippy and the existing suite. No golden output moves.
