## T02 — Refusals, absences and clamps that say what they are

**Why.** When a figure cannot be computed, the core often reports a plausible number, a sentinel or the wrong reason instead of saying so. A twin-countershaft train (ratio 5.458204, single-branch efficiency 0.968084) reports efficiency 0.000000 both ways, which reads as self-locking, and backlash 0°. Doubling one mesh prints backlash 0.018050° against the true 0.058901°, 69 % low, with no flag [kinematics-flow#2]. A pair given both end speeds is refused as "4294967295 more speed(s) to give" [train-mod-b#7]. A widening tolerance moves the band's minimum play from −4.14° to exactly 0 [mesh-contact#3]. Behind these sit five patterns. Option or `unwrap_or(0)` is the only failure channel. Refusal keys name a symptom ("the teeth never contact" also covers E ≤ 0, face width ≤ 0 and module 0). Every error enum has a second English text in `gear-core` that has drifted from the catalogue, and the four translations follow the drifted text. Refusals cross the wasm boundary in three shapes, one of them sniffed by a `"ui."` prefix. Guards clamp without a note where rule 5 says clamp and say so, or refuse.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T02.1 Errors have one English text: the catalogue's | train-mod-a#14, lens-unification#5, primitives#11, added3#13 (unverified), crossed-worm#19, added2#13, lens-errors-policy#12, ablate-features#4 | medium | M | — |
| T02.2 A path figure that was not computed is absent, not 0 | kinematics-flow#2, train-mod-b#5, added2#8 | medium | M | — |
| T02.3 Given speeds are judged by rank, not by a count formatted from `usize::MAX` | train-mod-b#7, added2#5 | medium | S | — |
| T02.4 A missing root section costs the rating, not the train | lens-errors-policy#4, added#48 | medium | M | T02.1 |
| T02.5 Every refusal a designer can reach crosses as data, as a Note | lens-architecture#2, lens-errors-policy#11, wasm-boundary#5, lens-errors-policy#10, lens-architecture#3, gear-io#6, added2#80 | medium | M | T02.1 |
| T02.6 Every guard that binds either notes or refuses | lens-errors-policy#2, lens-errors-policy#7, primitives#2, added2#83, added2#97, lens-errors-policy#9 | medium | M | T01.6, T01.8 |
| T02.7 Every refusal key names its cause | lens-errors-policy#3, lens-errors-policy#13, added3#8 (unverified), added3#3 (unverified), added2#23, added2#49 | low | M | T02.1, T01.1, T01.2 |
| T02.8 One play law, and no zero for a band end that has no operating angle | lens-unification#4, mesh-contact#3, lens-errors-policy#14 | low | M | — |
| T02.9 Absence is typed, never a sentinel | lens-numerical-robustness#6, lens-numerical-robustness#9, primitives#5 | low | S | — |

Two groups of tasks in T01 carry part of this one. **The input gate** (T01.6, T01.8, T20.13 [lens-errors-policy#0, primitives#0, gear-cli#9]) refuses no-shape inputs (z = 0, m ≤ 0, non-finite values, planet count 0) at every entry point, and material validation (T01.9 [lens-errors-policy#5, gear-io#5]) does the same for unphysical constants. **The structural validator** (T01.1, T01.2 [wasm-boundary#0, graph-ops#1, shape-a#7]) checks every index a `Shape` holds. T02.6 and T02.7 need both. Once both exist, most of the wrong-cause refusals below can no longer be reached.

### T02.1 Errors have one English text: the catalogue's
**Change.**
- Make every `Display` in `gear-core` language-free: `TrainError` (train/mod.rs:1712), `MeshError` (mesh.rs:766), `MeasurementError` (metrology.rs:748) and `EditRefused` (edits.rs:171). Each prints only `note().key` and its values. `Display` stays because `std::error::Error` needs it, not "for Debug": Debug is derived.
- Before the switch, move every payload the Display text carries today into the note, so the change loses nothing:
  - `TrainError::Wiring(_)` names its member or mesh, with one key per `WiringError` variant (`error.train_wiring_no_teeth`, `_no_common_frame`, `_not_a_mesh`, `_not_a_coupling`) in place of the value-less `error.train_wiring`.
  - `InPart` carries its part index.
  - `MotionError::Empty` gets its own key instead of reading "no such body: body 0".
- `TrainError::note` already delegates the `ScrewError` arms to `ScrewError`'s note. Delete their hand-written English.
- Give `EditRefused` an `Explain` with `note::key` constants. Move its keys from `[ui]` to `[error]` (`error.edit_*`).
- Switch the roughly 20 `{e}` sites in `gear-cli` to `words().render(&e.note())` (`Strings::english()`). Tests that match on text switch to `matches!` and `{e:?}`, e.g. crossed.rs:1094.
- Reword `screw_axes_are_parallel` and `low_efficiency` in all five catalogues from one agreed sentence without "stage". The four translations currently translate the Display text, not the English catalogue.
- Make `reversed_bending_allowable` return `f64`: its only caller (train/mod.rs:2642) takes `.value`, so its `format!` English note is thrown away. Scope `Value::note`'s invariant to library entries, which makes `overridden`'s "supplied by the user" unnecessary [ablate-features#4; strength#15 decides the function's future].
- Correct rationale.md:57-61, note.rs:381-382 and the `[error]` header of strings_en.toml.
- Do not build a single `Refusal` struct over all enums: the typed matches callers use (`TrainError::InPart{..}`) would be lost [lens-errors-policy#12].

**Proof.**
- A `gear_io::strings` test renders every variant of every error enum through the English catalogue. It asserts that no `{placeholder}` is left and that the index appears wherever the variant carries one. On the current code it fails for `Wiring`.
- A grep gate finds no English literal in any `impl Display` under `crates/gear-core/src`. Once `metrology#6` is done, that leaves `Ratio` and `jgma::Class`.
- `tools/check_golden.sh`: the only diffs are error lines moving to catalogue wording, e.g. kinematics.txt:1466/1475 and planetary_17_17_3.txt:14. Review them, then run `--write`.

### T02.2 A path figure that was not computed is absent, not 0
**Change.**
- In train/mod.rs `paths_of`, make `PathReport`'s efficiency, circulation and backlash `Option`. `flowing` (mod.rs:3426) maps every `flow::Refused` to `(0.0, 0.0)`, and `coefficient` (mod.rs:3465) maps a non-unique play to 0. Both return `None` instead, so 0 % keeps its one meaning: the path locks (`Directional::locked`).
- In flow.rs, tell the two causes of `Refused::Undetermined` apart by the null space of the torque system:
  - If the null space moves only internal mesh torques, the case has a redundant mesh loop. Carry it as `Undetermined { ports: false }` and raise a new `train.mesh_loop` note naming the meshes.
  - If it moves reacted or derived port torques, the case is held at both ends. Keep `train.load_shared` for it.
  - A redundant kinematic row is not a reliable signal, because a doubled coupling is also redundant [added2#8].
- Five catalogue strings for the new note. The panel's `lockedWays` shows "—" for `None`.
- Optional follow-up: compute a loop's play as the minimum over its spanning trees, which is exact for rigid bodies. Equal load sharing between identical parallel branches is a rule of thumb, so it may appear only as a user-visible option.

**Proof.** Written first, and failing today:
- The twin-layshaft fixture (probe `dual.rs`, 17/43 and 19/41) gives efficiency `None` or 0.968084, never 0, and backlash `None` or at least 0.058901°.
- The doubled-output-mesh fixture gives backlash at least 0.058901° or `None`. Today it gives 0.018050°.
- Law over every preset: doubling any mesh never lowers the path backlash.
- Law: a path's efficiency is 0 only when its flow solved and a mesh locks.
- The twin fixture raises `train.mesh_loop`, and an existing both-ends-held test still raises `train.load_shared`.

### T02.3 Given speeds are judged by rank, not by a count formatted from `usize::MAX`
**Change.**
- In train/mod.rs:4440-4462, drive every given port at its own speed, as an exact rational, and solve:
  - An inconsistent system gives a new `train.case_overdetermined` note naming the port.
  - A residual gives `case_underdetermined { short = residual.len() }`.
  - A consistent, unique solution is accepted and solved.
- Counting against `case_mobility()` alone is not enough: two speeds on one rigid chain meet the count but are rank-deficient.
- Five strings for the new key.

**Proof.** Unit tests that fail today:
- Planetary(12,30,72,3), ring released, three given speeds: today this gives `short: "0"`.
- A pair given 4300 and −1700 rpm, which agree with the ratio: today `short` is 4294967295. It should solve.
- A pair given inconsistent speeds gives `case_overdetermined`.
- Law: no note ever carries `short` 0 or `u32::MAX`.

### T02.4 A missing root section costs the rating, not the train
**Change.**
- Delete the refusal loop at shape.rs:3229-3237. A member with no bending section in any mesh gets `bending_stress: None` and a note, as a ring or a planet's other mesh already does (`gear.bending_unrated_in_mesh`, or one `gear.bending_unrated`).
- A truly severed tooth (`Tooth::severed`) describes no shape. The input gate refuses it under its own key, not as "no root section".
- Reword `error.train_no_root_section`, or retire it once nothing raises it, so it no longer blames undercut. In the probed cases the teeth are not undercut: the load point leaves the flank at an overlapping running distance.
- For a part carrying `part.clearance_negative`, rate it at its shifts' own zero-backlash distance, with the notes, or give it no ratings. Neither the refusal nor the ratings should depend on how far past overlap the given distance lies [added#48].

**Proof.**
- Turn the ring-notch test (shape.rs:4625) into a law over all presets: forcing any one member's section to `None` leaves every other figure equal to the unforced solve. It fails today on Spur member 0 with dedendum 0.
- Sweep the given distance from the `CentreDistanceTooSmall` limit up to standard, search on and off, for 17/43, 23/61, 31/37, 41/43 and 20/20. Every point returns `Ok` with notes, and none returns `NoRootSection`. Today std−1.5 module refuses.

### T02.5 Every refusal a designer can reach crosses as data, as a Note
**Change.** Apply the rule rationale.md:2105-2110 already states: throw only for a malformed request, and return every refusal a designer can reach in the value.
1. `edit_train` returns `{train, refused: Option<Note>}`, as `preview_edit` already does. Delete the `message.startsWith("ui.")` sniff at core.ts:823. Until then, `editTrain` returns `ui.train_boundary_failed` with the detail instead of `null` on a defect. Today `Offers.svelte` reads that `null` as a successful edit [added2#80; web#13 is the same fault].
2. `amplitude_for_throw`'s error gets a `note()`, so `solve_gear` and `solve_ring` return an unreachable throw as `refused: Note` with key `error.gear_throw_unreachable {throw}`, not the English at lib.rs:660-678. A throw with no mate is a malformed request and can stay thrown.
3. `relieve_case` maps `TrainError` through `note()`, not `format!("{e:?}")` (lib.rs:1501).
4. `MaterialError` gets `note()`: `error.library_empty`, `error.library_duplicate_name {name}`, and `error.document_unreadable {detail, line, column}`, with line and column from `toml::de::Error::span`. The Sidebar wraps it in a translated "library import failed" line, as the train import already is. gear-io's train `Parse` drops its English "geartrain file is not valid:" prefix and passes the parser text as `{detail}`.
5. Adopt's refusals are defects by their own comment and stay thrown.

Migrate one entry point at a time, each with its `wasm_probe.mjs` step and `check_wasm.sh --write`.

**Proof.**
- A gear-wasm test drives each `*_impl` through every refusal a designer can reach and asserts that each comes back as a Note whose key exists in the English catalogue. It fails today for the throw (English text) and for edits (`Err`).
- A probe step sends `edit_train` a malformed edit and asserts that the wrapper does not return `null`.
- `check_strings.py` and the fire-every-note test cover the new keys.

### T02.6 Every guard that binds either notes or refuses
**Change.** Rule 5: clamp an input that can nearly be cut and say so; refuse one that describes no shape. Each case below also shows when it becomes reachable.
- **Tip at or below the base circle** (tooth.rs:471, `ra.max(rb·(1+1e-9))` with no note). The tooth has no involute flank, so refuse it through the gate, with the addendum named. `admissible_ranges`' `addendum.min` (auto.rs:671) is already this bound. Apply the same predicate to the ring and retire `clamp.ring_tip_raised`. At ring.rs:283, use `guard::TIP_ABOVE_BASE_FRACTION` instead of a bare 1e-9. T03.4 owns the related fault, a tip below the form radius rather than the base [tooth-form#6].
- **Root radius** (tooth.rs:925). Refuse a negative or non-finite value. Exactly 0 keeps its silent 1e-9·m floor, a legitimate sharp rack.
- **Planet count 0** (shape.rs:463 `.max(1)`) and **cutter teeth 0** (ring.rs:198). Refuse both as shape invariants. Delete the `.max(1)` guards only after the gate lands, so the train panel gains no panic path.
- **Cutter no smaller than its ring** (ring.rs:322, now clamped to `z−1` with `clamp.cutter_teeth_reduced`). Refuse it as `error.ring_cutter_too_large {cutter, ring}`, matching `MeshError::RingTooSmall`. Drop the clamp key and its five strings.
- **Module 0**. The gate refuses it. Then re-check rationale.md:138 and :1635 so they list only refusals that happen.
- **`Gear::profile` panic** (gear.rs:564, `&r[1..]` on an empty half-profile, for z = 0, m = 0 and NaN m). The gate on `resolved_params`, used by `solve_gear_impl`, `gear_profile_impl` and `export_dxf_impl`, removes it. Also cap `outline`'s vertex count: today it returns 65,538 vertices for z = 0 and 1,114,146 for m = 0. Correct the "guarded to one, as elsewhere" line in `Gear::new`'s doc.
- Fix params.rs:188-197, which still promises a "human-readable note".

**Proof.**
- Law: for every guard in `params::guard` and every `.max(1)` on a count, an input that makes it bind yields a `clamp.*` note or a refusal, never an unchanged note list. Compare against the floor, so a sharp rack is not caught. It fails today for z = 17, addendum −0.52 (`ra` = 7.987387, `clamps` = []), root radius −1, and planet count 0.
- The extremes test checks both sides of `addendum.min`.
- `Gear::new(p).profile(n)` and `outline()` at and just outside every invariant bound, under `catch_unwind`: none panics.

### T02.7 Every refusal key names its cause
**Change.** After the gate, material validation and the structural validator have removed most of these paths, map what remains to its own key:
- `TrainError::NoContact` is raised only where `ContactPath::new` returns `None`. Today shape.rs:2563/2591/3322 map every `None` from hertz and contact_stress to it.
- A non-positive combined modulus or a bad material gets `error.train_material_invalid {name, field}`.
- A given face width ≤ 0 is refused with its member named, which also removes the silent nulls at 0: 16 fields go null with no note today.
- Flank interference (approach > r₁ sin α_w) gets its own case, or is rated on the usable path once lens-continuity#0 caps the path at the form radius [added3#8 (unverified)].
- A set whose three given shifts disagree on the running distance gets its own key, with both distances and their difference. "Cannot be assembled" stays for sets no shift can close [added3#3 (unverified)].
- A member on an unlisted body is `error.train_no_such_body`. Today it is `train_overdetermined` or `NoCommonFrame`. It is refused by the validator at `from_toml` and at the top of `solve_train`.
- `ScrewError` gets a `NonFinite` variant; screw.rs:254 now reports a non-finite shift as "not positive".
- `worm_too_thin` names both causes: a zero helix on the worm member, or a pitch diameter below starts × module.

**Proof.**
- Law over the degenerate-input sweep: every `Err` key is one of a declared per-field set, and `NoContact` appears only where `ContactPath::new` returned `None`. It fails today for E = −1, face width −11 and module 0.
- For every preset, setting each member's body to max_body + 1 refuses with `train_no_such_body` carrying that number, both at `import_train` and at solve.
- `pair([2,5])` solves with an interference note, or fails under an interference key.

### T02.8 One play law, and no zero for a band end that has no operating angle
**Change.**
- Give `LineBuilt` an `angular_play(a, axial, z)` that mirrors `PointBuilt`'s. It computes the normal gap as `backlash(a)? · cos α_t(a) · cos β_b + axial · sin β_b`, then calls `mesh::angular_play(gap, z, plane::base_pitch(m_n, α_n))`. `play_of` (shape.rs:3524) becomes one call on `BuiltContact`, with no match.
- Settle the sign first. The Point arm floors the separation at 0 (shape.rs:2638), while the Line arm reports negative play for interference. Negative in both keeps the band continuous.
- A band end below the base-circle limit `a_ref cos α_t` has no operating angle and describes no assembly. Refuse the tolerance under a key naming the distance and the limit, instead of `unwrap_or(0.0)`.
- `Mesh::pressure_angle_at` (mesh.rs:296) and `ring.rs` `described_at` (ring.rs:1047) require `a > 0`.
- Delete the dead `cos_w > -1.0` term (mesh.rs:172).
- Store the resolved distance on `BuiltMesh` and delete the three unreachable `distance_of(k).unwrap_or(0)` calls and `frame(j).unwrap_or(GROUND)` (shape-b#13 is the same cleanup).

**Proof.**
- The new Line arm equals today's `row_play` to 1e-12 at all three band points on every preset.
- Law: the band minimum is non-increasing in `tolerance_minus`, with no jump. On 17/43 today it goes from −4.1377° at ±1 mm to 0.0000° at ±5 mm.
- `pressure_angle_at(−x)` is `Err` for every x > 0. Today `(−100)` gives `Ok(1.8566 rad)` and a backlash of 1054.97 mm.

### T02.9 Absence is typed, never a sentinel
**Change.**
- `admissible_angular_shift` (auto.rs:649) builds `by_spread` and `by_sink` as `Option<f64>`, and returns `Bound::between(amp.map(|a| -a), amp)`. This removes both `f64::INFINITY` sentinels: 268 of the 280 non-finite values in the train fuzz are this one.
- `Note::number` (note.rs:373) gets `debug_assert!(value.is_finite())`.
- Later (M): replace the severed tooth's NaN `u_j`/`u_tip` with `Option<Flank>`, across 59 read sites.
- The solve.rs module doc says what `None` means: no root in the bracket, or a residual that was not finite. Each `unwrap_or` fallback (shaper.rs:450, ring.rs:568, tooth.rs:601/626/689) gets a one-line comment naming the limit it stands for. A typed `SolveError` earns its place only at strength.rs:725/735, where "no tangency" is the signal [primitives#5].

**Proof.**
- Law: every `Ranges` for z = 1-3000 has only finite `Some` values. It fails today at z = 1.
- A JSON scan of every preset's `TrainResult` finds no non-finite number.
- Fire every note over the gear, mesh and train fuzz seeds: no value string is NaN or inf.
