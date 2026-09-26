## T14 — Architecture: one model, three contracts, and where code lives

**Why.** One Rust type often serves three contracts at once: the core's model, the TOML file format, and the JSON wire to TypeScript. Only some of them are checked. `TrainDocument` is `{ name, train: Train }` with no format version. Its module doc holds 25 migration entries, yet 16 fields default silently, three of them from live gear-core constants. On the wire, seven request types and `TrainEdit` are hand-copied in TypeScript, and no request refuses an unknown field: `{materails: …}` comes back OK. Engineering that belongs in the core sits in `gear-wasm`, where no law or golden output sees it. The ring's smallest tooth count drops the profile shift and shows 34 where the truth is 17 (x = +0.5) or 44 (x = −0.3). Inside the core, `train/mod.rs` is 11,363 lines (4,639 production, 6,724 test). `shape::rate` is about 657 lines of closures that capture each other. Five hand-written union-finds compute two relations, and one of them is computed twice. The contact-model seam that is documented as "answered once" is matched on at about 16 sites outside its own impls.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T14.1 Generate every request type; refuse unknown fields | wasm-boundary#6, added2#42, tools-ci#10 | medium | S | — |
| T14.2 Ring's fewest teeth moves to `ring.rs`, with the shift | lens-architecture#0, added3#15 (unverified) | medium | S | — |
| T14.3 `metrology::over_pins_around`, every seat or refuse | wasm-boundary#3 | medium | S | — |
| T14.4 Versioned file format; frozen stage-era DTO; shims out | lens-architecture#9, gear-io#8, ablate-features#11 | medium | M | — |
| T14.5 Split `train/mod.rs`: tests by topic, `report.rs` breaks the cycle | lens-architecture#6 | medium | L | — |
| T14.6 Graph well-formedness checked on input | shape-a#13 | low | S | — |
| T14.7 One `DisjointSets`; one mesh partition | lens-architecture#11, shape-a#15, auto-search#15 | low | S | — |
| T14.8 Contact and member seams answered by methods; MeshKind residue | lens-architecture#8, lens-unification#10 | low | M | — |
| T14.9 Split `rate`, `chosen_at`, `sized`, `helix_angles` | lens-architecture#5, shape-a#17 | low | L | T14.7, T14.8 |
| T14.10 Retire `pair.rs`, `planetary.rs`, `crossed.rs`; `TipRoom` down to `ring` | lens-docs-accuracy-1#19, lens-architecture#12 | low | M | T14.5 |
| T14.11 `GearResult` stops repeating `params` | train-mod-a#11 | low | M | — |
| T14.12 One encoding per wire fact (ground, part numbering) | web#16, ablate-features#11 | low | S | T14.4 |
| T14.13 Shipped load-case figures stated once | lens-architecture#1 | low | S | — |
| T14.14 Crate-seam API: `gear_to_dxf(&Gear)`, `DocumentError`, `member()` | lens-architecture#16, lens-architecture#14 | low | S | — |
| T14.15 One gear request and summary; the ring is a cutter option | lens-architecture#10 | low | M | T14.1, T14.2 |
| T14.16 One generic JSON helper; static entries folded into `defaults` | wasm-boundary#9, lens-architecture#13 | low | M | T14.1, T14.15 |
| T14.17 TOML parsed untyped and read through the JSON deserialiser | lens-performance#7 | low | M | T14.4 |
| T14.18 `Preset::family` read off the shape | graph-ops#13 | low | S | — |
| T14.19 A sun central to two carriers: say the workaround | shape-a#8 | low | S | — |
| T14.20 One placeholder grammar | added2#93 | info | S | — |

### T14.1 Generate every request type; refuse unknown fields
**Change.** In `crates/gear-wasm/src/lib.rs`, add `#[cfg_attr(feature = "typescript", derive(TS), ts(export, export_to = "wasm/"))]` to `AdoptRequest`, `RelieveRequest`, `RelieveCaseRequest`, `TrainEdit`, `EditRequest`, `PreviewRequest` and `OffersRequest`, and make them `pub`. Replace the `json!` row in `languages_impl` with `#[derive(Serialize, TS)] struct LanguageOption`. Add `#[serde(deny_unknown_fields)]` to every request struct, including `GearRequest`, `RingRequest` and `TrainRequest`. Name the library `materials: Option<MaterialLibrary>` everywhere: `RelieveCaseRequest` uses `library` (required) today, which is why `core.ts:710` re-parses `defaultLibrary()` on every relief. Give gear-io an optional `typescript` feature (ts-rs on `TrainDocument` and `Imported`, `export_to = "io/"`), and add the feature and the `io` directory to `check_bindings.sh`. In `web/src`, import `Note` from `./wire` and delete the copy at `strings.svelte.ts:80`. Delete the hand-written `TrainEdit` union (`core.ts:809`), `LanguageOption`, `TrainDocument` and `Imported`. Type every request literal (`const req: EditRequest = {…}`). Fix `wasm_probe.mjs:131,264,265`, which send `library` to `solve_train` and get it dropped.
**Proof.** Written first and failing today: (a) a probe step that sends `solve_train` a misspelt field must be refused, and today `results.json` records OK. (b) In a worktree, rename `TrainEdit::Duty.intermittent`, run `check_bindings.sh --write` and then `npm run check`, which must fail at `editTrain`. Today it stays green. Before adding `deny_unknown_fields`, confirm that the UI literals carry no extras: `tab.mate` must stay exactly `{teeth, profile_shift, internal}`.
**Notes.** The catch-all error swallowing in `core.ts` is T19.8 [added3#12 (unverified), web#13]. Until it goes, a rename degrades to a silent no-op in the UI. This task makes that rename a type error.

### T14.2 Ring's fewest teeth moves to `ring.rs`, with the shift
**Change.** Lands as T05.6 (`ring::smallest_tooth_count`), which carries this task's deletions and its doc fix.
**Proof.** T05.6's law. It fails today at x = +0.5 (the served figure is 34, the true one 17) and at x = −0.3 (34 against 44).

### T14.3 `metrology::over_pins_around`, every seat or refuse
**Change.** Lands as T09.3, which carries `over_pins_around`, the deleted fold and the pin range published as a core `Bound`.
**Proof.** T09.3's law.

### T14.4 Versioned file format; frozen stage-era DTO; shims out
**Change.** In `gear-io/src/train.rs`:
- Add `format = N` at the document root. The writer writes it, and the reader refuses any other value with a key that names the version and points at `gear-cli convert`.
- At the current format, every field is required. That is what the writer already emits for every preset. Remove the 16 `serde(default)` fill-ins from the graph types.
- Give `convert` its own frozen `Staged` DTO instead of `Vec<Shape>`. Its defaults are literals frozen in gear-io (`min_contact_ratio` 1.2, `min_planet_clearance` 0.3, `pressure_angle` 20), each with a one-line comment, and they no longer read `DEFAULT_MIN_CONTACT_RATIO`, `default_planet_clearance()` or `GearParams::default()`.
- Delete `ground_if_null` (`shape.rs:118`) and its only caller, the wasm test at `lib.rs:1813`. Move that test's round-trip paragraph onto its own test.
- Delete `coefficient`'s legacy `{auto, manual}` arm. It accepts arbitrary unknown keys [added2#90].
- Replace the 233-line change log in the module doc with a statement of the current schema, and move the history to `docs/` as a format → change table. This is also the fix for lens-docs-clarity#5, ablate-features#10 and lens-docs-accuracy-1#16.

**Proof.** Written first: (a) a graph file with every `min_contact_ratio` line deleted is refused, naming the field (today it loads with 1.2). (b) In a worktree, perturb `DEFAULT_MIN_CONTACT_RATIO`. The stage-era fixture's `convert` output stays byte-identical (today it moves), while the current-format corpus shifts as expected. (c) Every preset and every shipped fixture round-trips through `import_train`/`export_train`. `a_field_the_shape_no_longer_has_is_refused_and_named` still holds.
**Notes.** Migrations are chained only from `format = N` onward, not over the 25 historical formats. Root-level `deny_unknown_fields` on `TrainDocument` is T01.3 [gear-io#7] and lands with this change.

### T14.5 Split `train/mod.rs`: tests by topic, `report.rs` breaks the cycle
**Change.** Each step is one behaviour-neutral commit:
1. Move the 6,724 test lines of `mod.rs` into `#[cfg(test)]` submodules under `train/tests/{motion,flow,relief,edits,rating,presets}.rs`. This keeps access to private items, which integration tests would lose.
2. Move `MeshReport`, `GearResult`, `MemberRating`, `Bending`, `ContactRatios`, `Backlash`, `LineContact`, `PointContact`, `ContactPatch` and `MemberFacts` into `train/report.rs`. Both the solve in `shape.rs` and `mod.rs` then depend on it, and `shape.rs`'s `use super::{…14 names}` no longer points back into `mod.rs`.
3. Split the remaining groups into `relief.rs` (Freedom, MemberFreedom, Figure, Reading, FreedomGroup), `case.rs` (LoadCase, Load, Duty, Turns, CaseLoad, Cycles) and `error.rs`, with `Train`, `solve_train` and `paths_of` left in `mod.rs`.

Update CLAUDE.md's per-file table in the same commit as each move.
**Proof.** At every step, `cargo nextest run`, clippy, `check_golden`, `check_bindings` (no diff, since `export_to` is explicit) and `check_doc_links` all pass. `grep 'use super::' shape.rs` names no report type after step 2.
**Notes.** A full `model/ topology/ solve/` directory layout is optional after this. It does not by itself remove the cycle, but step 2 does.

### T14.6 Graph well-formedness checked on input
**Change.** Promote `edits.rs`'s `well_formed`, which is `#[cfg(test)]` today, to a production `Shape::validate` called at every entry that takes a graph: `from_toml`, and the wasm solve, edit, preview and offers entries. Add two invariants to it: the ring is `MeshInput.b` (or normalise with `push_mesh`'s swap on load), and there is one `Distance` per unordered axis pair. Each refusal gets its own key. Define `axial_clearance` on the lower-indexed axis so that it does not depend on the order of `Distance.axes`.
**Proof.** Fixtures written first: a ring-first planetary mesh is refused with its own key. Today it is refused as "no profile shift brings the meshes to one axis distance", and lens-unification#1 shows it wiring as external. A duplicate distance is refused. Today it is shadowed, first entry wins. Swapping `Distance.axes` changes no output.
**Notes.** The index-range and carrier-cycle checks of T01.1 and T01.2 [wasm-boundary#0, shape-a#7, graph-ops#0] belong in this same function. The helix, module and overlap representations stay as they are: relief and `share()` guard them, and overlap is per mesh on purpose.

### T14.7 One `DisjointSets`; one mesh partition
**Change.** Add a crate-private `DisjointSets` (`new`, `union`, `find` with full compression, `components()` in first-member order, documented as load-bearing) in `train/graph.rs` or `train/components.rs`. Use it at all five sites: `graph_of` (graph.rs:64), `Shape::parts` (graph.rs:234), `mesh_groups` (shape.rs:1090, which also loses its O(n·groups) regroup), `asking_members` (1996) and `search_components` (2060). Compute the mesh partition once. `asking` is then "the partition contains a searched mesh", the free members are the Free members in asking partitions, and the axis components are the partitions' axes. Rewrite `docs/state.md:487–489` to say: one product-over-meshes objective, searched per component in sum-and-division coordinates, over line and point contacts alike. It says crossed pairs are not searched, and a probe shows they are.
**Proof.** A law over every preset and the graph harness's every-preset-after-every-other: `parts`, `mesh_groups` and the search components are identical before and after, and each search component is exactly one asking component. `check_golden` shows zero diff.

### T14.8 Contact and member seams answered by methods; MeshKind residue
**Change.** Keep `BuiltContact` and `BuiltMember` as enums; a trait with `dyn` gains nothing for two or three models. Move each outside match into a method:
- The 11 contact sites in `shape.rs` become `BuiltContact`/`BuiltMesh` methods: the search trial (2251, 2259), `recommended_face` (worm width, 3153), `bending_contact_ratio() -> Option` (3214), `contact_stress` (3305), `rate_point` (3337), `overlap_floor` (3445), `play` (3524, 3537) and `report` (3813, 3856).
- The member sites at 2219/2220, 3683, 3716 and 3843 become `BuiltMember` methods.

Reword the `BuiltContact` doc to name where the models differ by decision, for example that a point contact has no bending ratio. Clear the `MeshKind` residue:
- `shape.rs:689` becomes `kind.signed(..)`.
- `shape.rs:1056` becomes `MeshKind::mate_thickness_mod(x) = 1 + σ(1 − x)`, called as `kind_of(k).map_or(2 − x, …)` so the crossed arm stays explicit.
- The involute-domain floor (1465) and the base-circle limit (1617) become `mesh::min_shift_sum` and `mesh::base_circle_limit`, shared with `operating_geometry`. The `(1 + 1e-6)` gives way to asking `pressure_angle_at` for the smallest admissible distance.

**Proof.** `nextest`, `check_golden` and `check_wasm` show zero diff. A grep for `BuiltContact::(Line|Point)` and `BuiltMember::(Ring|Rack)` outside the two impls returns nothing, and `MeshKind::Internal =>` in production appears only in `mesh.rs`.
**Notes.** `BuiltMember` is not replaced by `ToothOutline`: bending already routes through it, and the search's shaper cut and the ring-only reports need the `Ring`. The rack/shaper split is T03.15 [lens-unification#2].

### T14.9 Split `rate`, `chosen_at`, `sized`, `helix_angles`
**Change.** Turn `shape::rate` (3250–3907) into a pipeline of named pure functions that follows its real dependency order, cut → flow → size → rate → report:
- `size_widths(&Cut, &CaseLoad, asks) -> Widths`, which records the requirement that set each width,
- the backlash rows and bands,
- the planet layout,
- `report(…) -> (MeshReport, GearResult, notes)`.

Width sizing stays after the flow, which is correct. Split `chosen_at` (1717–1934) into `SearchSpace::new(plan, free, intervals)`, the components from T14.7, and a rounds loop. Split `sized` (1568–1711) into one sizing per distance over a single `bracket_monotone(room, from, dir)`. Derive its bounds from the involute domain's floor and `reach`, instead of the 0.01·from step, the 1.5 growth, the 40/80 iteration caps, the 1e-12 floor and the 1e-9 lean. `helix_angles` and `size_reaching` also use it. The helix-sizing loop becomes `size_from_distance(k)`.
**Proof.** `nextest`, `check_golden` and `check_wasm` show zero diff. New unit tests for `size_widths` alone: the width a given overlap asks for equals `width_for_overlap`, and the largest requirement wins. `bracket_monotone` brackets every root on a monotone test function without an iteration cap.

### T14.10 Retire `pair.rs`, `planetary.rs`, `crossed.rs`; `TipRoom` down to `ring`
**Change.**
- Move `ShiftAsked`, `Decided` and `undercut_bound` into `auto.rs` next to the functions they wrap, and fix `pair.rs:99`'s link to the nonexistent `Decided::Given`.
- Move `Train::arranged_as` next to `Train::arranged` in `mod.rs`.
- Move `crossed::proportions` into `arrangements::worm` or `screw.rs`. Move the backlash derivation (j_n = j_axial·sin β_b1 + 2Δa·sin α_n) into `docs/reference.md#crossed-axes`, or beside `BuiltContact`, and its 1,829 test lines into `train/tests/crossed.rs`.
- In `auto.rs:1332`, ask `ring::mesh_at(ring, tooth, a_w).is_some_and(|m| !m.tip_interference)`, which removes the only upward edge into `train/`, or move `TipRoom` into `ring.rs`.
- Drop the three map rows in CLAUDE.md and correct its list of untested modules [lens-docs-accuracy-1#18].

**Proof.** The suite and corpus are unchanged. `grep -r 'crate::train' crates/gear-core/src/*.rs` finds no code outside `train/`.

### T14.11 `GearResult` stops repeating `params`
**Change.** Drop `GearResult.profile_shift`, `.addendum` and `.helix_angle`, and `MemberFacts.profile_shift`, together with its stale "for a hula gear" doc. Readers take `params.*` instead. Keep `pitch_diameter`, `lead_angle` and `lead` as Rust outputs (rule 1). Update gear-cli (about 40 reads) and `TrainPanel.svelte:1426,1509`.
**Proof.** Measured over every preset with the search on and off, the difference between each dropped field and `params` is 0. `check_bindings`, `npm run check`, and `check_golden --write` produce a diff that is labels only, with no number moving. Inspect it before writing.

### T14.12 One encoding per wire fact
**Change.** `AxisGroup.carried_by` becomes a `Body` with 0 for ground, like `Axis.carried_by` (`groupings.rs:47,123`). `TrainFailure.part` becomes 0-based: drop the `+ 1` at `lib.rs:864` and the `- 1` at `TrainPanel.svelte:2116`. The panel then tests `!== 0` in one way at L241, L536, L584, L830 and `members.ts:34`.
**Proof.** `check_bindings --write`, `npm run check` and `check_wasm --write`. The only diff in the recorded payloads is `part`. A ground axis shows no carried chip in either view.

### T14.13 Shipped load-case figures stated once
**Change.** Lands as T13.6: the two copies in gear-wasm (`defaults_impl` at `lib.rs:1175` and `apply_edit`'s `AddCase` at 1576) of 0.1 N·m / 30,000 rpm (ultimate) and 0.02 N·m / 30,000 rpm (fatigue) read `LoadCase::fresh(kind)` in the core.
**Proof.** T13.6's `AddCase` test.

### T14.14 Crate-seam API
**Change.**
- `gear_io::gear_to_dxf` takes `&Gear`, or `&GearParams`, and draws its reference circles from `Gear::mean()`. Today it draws the outline from `Gear` but the circles from the `Tooth` passed in, so an eccentric gear's root circle is off by up to 1.525 mm at m = 1 [wasm-boundary#4].
- Rename `gear_io::train::TrainError` to `DocumentError`.
- `Train::member()` returns `Option`.

**Proof.** A test that an eccentric gear's DXF root circle equals the summary's `rf`. It fails today at z = 10, x = 0.5, Δx = 2.225. `validate_dxf.py` stays green.

### T14.15 One gear request and summary; the ring is a cutter option
**Change.** Add `cutter: Option<CutterRef>` to `GearRequest`, the same way the train marks a ring with `Member.ring: Option<Cutter>`. Merge `RingSummary` into `GearSummary`, with the ring-only fields optional (`generation_limit`, `rim_radius`, `between_pins`). Retire `solve_ring`, `ring_profile` and `export_ring_dxf` into the gear entries, which takes 23 entry points to 20. Update the probe, `wasm_boundary.json` and `GearPanel.svelte`. The gear tab does not become a one-member train: that pipeline needs materials and load cases, and it cannot hold the eccentric gear.
**Proof.** `check_wasm` reproduces the ring tab's recorded answers through the merged entry. A new law: for every preset member, the tab's summary of `adopt_member`'s params equals the train's `GearResult` for that member.

### T14.16 One generic JSON helper; static entries folded into `defaults`
**Change.** Write each entry as a single line over `fn json<Rq: DeserializeOwned, Rs: Serialize>(input, f) -> Result<String, JsError>`, which replaces 54 hand-written `map_err` sites. Fold `version`, `languages` and `default_materials` into `Defaults`. `strings(language)` and `resolve_language` stay, since they take arguments. Optionally add `profile_points: Option<usize>` to `GearRequest` so one call returns the summary and the outline. This helper carries T02.5's typed `{ok}|{refused: key}` envelope [lens-architecture#2, wasm-boundary#5]. Calling `offersAt` once per selection is T11.14 [edit-ops#8].
**Proof.** The `check_wasm --write` diff is limited to the removed entries, and `npm run check` is green. The probe stays exhaustive: it fails on an entry with no step.
**Notes.** A single tagged `call(Request)` dispatch is not needed. It moves the match without removing a round trip.

### T14.17 TOML parsed untyped and read through the JSON deserialiser
**Change.** In the wasm build only, parse to `toml::Table` and deserialise with `serde_json::from_str(&to_string(&table))`, wrapped in `serde_path_to_error` so that a type error still names its path. The native CLI keeps toml's typed path and its spans. Add a `TrainError`/`DocumentError` variant for type errors. Refuse non-finite floats by name, since TOML allows `inf`/`nan` and JSON does not.
**Proof.** Measured: 264 KB raw and 76 KB gzip off the module. Written first: `a_field_the_shape_no_longer_has_is_refused_and_named` passes on the wasm path. Without the path wrapper it fails, pointing at "line 1 column 3564" of an intermediate string. Every shipped TOML imports to identical JSON before and after. The new dependency changes the Nix fixed-output hash.

### T14.18 `Preset::family` read off the shape
**Change.** `Preset::family(self) = self.build().family()`. Delete the ten-arm match and only the family `assert_eq` in `every_preset_is_in_its_own_family_and_solves_conventionally`; its solve check stays. In `planetary()`, use `external(planet)` and drop `count.max(1)`, which `Shape::count_of` already applies.
**Proof.** `nextest`. The `defaults` payload recorded by `check_wasm` (menu order and families) is unchanged.

### T14.19 A sun central to two carriers: say the workaround
**Change.** A sun that meshes planets on two carriers (a Simpson) is refused with `NoCommonFrame`/`WrongFamily`, which is a deliberate limit. Give that refusal a key whose message says to put two gears on one body. That form solves, and it is the more faithful rating model. Replace `wiring.rs:33–39`'s "no arrangement in scope has it" with the reason: tooth cycles and speed against the frame are per member.
**Proof.** A new fixture, a Simpson whose sun is central to two carriers, is refused with the new key, and its two-gears-on-one-body form solves with every frame Ok and the textbook ratios, checked against `tools/train_kinematics.py`.

### T14.20 One placeholder grammar
**Change.** In `gear-io/src/strings.rs`, one `fn placeholders(template) -> impl Iterator<Item=&str>` serves `fill` (263) and both tests (327, 533). Delete the closure that shadows it. The catalogue law requires every placeholder name to match `\w+`, the grammar the TypeScript regex in `strings.svelte.ts` already uses.
**Proof.** A catalogue entry with `{a b}` or `{}` fails the law. It passes today, and Rust and TypeScript fill it differently.

### Declined
None. No finding here was refuted. Where a claim was narrowed, the narrower one is what the tasks above implement. That covers the `Partition` redesign dropped from lens-architecture#14, no `format` migration chain for historical files in lens-architecture#9, no MeshGroup rebuild in shape-a#13, no one-member-train gear tab in lens-architecture#10, and the `EditRefused::key` point of lens-architecture#16, which is refuted and left as it is.
