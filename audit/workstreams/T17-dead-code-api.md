## T17 — Dead code and public surface: what nothing reads, and the gates that would have said so

**Why.** Every refactor in this crate has left public scaffolding behind, and nothing checks for it. The two checks that would catch it do not exist. First, no check runs rustdoc: 22 intra-doc links do not resolve (gear-core 19, gear-io 1, gear-wasm 2), and about half name items that are gone. Second, no check asks whether a `pub` item has a caller outside tests. When rustc's dead-code pass is seeded only with what gear-io, gear-wasm and gear-cli name, it lists 28 unreachable items. Among them are 294 code and 230 comment lines of stage-era search in `auto.rs` [ablate-features#0]. A textual scan finds 144 of gear-core's 450 `pub fn`s unused outside the crate [lens-architecture#15]. The cost is concrete. Swapping the shape search's sum/division mapping still passes all 604 gear-core tests, because the train law meant to guard that search drives the dead copy [added2#87]. Two public functions document a unification that production does not use [added2#59]. Of the 251 wire fields, 6 are read by no panel code, and CLAUDE.md lists only 2 of them [ablate-features#6]. The plan adds both gates first. Every deletion after that removes a line from the gates' allowlist and keeps them green.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T17.1 Rustdoc gate; the 22 broken links fixed | ablate-features#3, added2#70, added#24, added#64, added#1, added#49, tools-ci#9, train-mod-a#12, ablate-constants-rating#16 | low | M | — |
| T17.2 Dead-pub gate with a shrinking allowlist; narrowing policy | lens-architecture#15, lens-unification#11 (oracles), ablate-constants-rating#16 (oracles) | low | M | T17.1 |
| T17.3 Delete the stage-era search; aim its laws at the shape's search | ablate-features#0, added2#87, added3#18 (unverified) | medium | M | T17.2 |
| T17.4 One relative-velocity primitive; the sliding docs made true | added2#110, added2#59, added#57, added#65, ablate-features#9 | low | S | T17.2 |
| T17.5 Contact split code and `tip_pressure_angle` out | mesh-contact#10, lens-continuity#6, ablate-features#9 | low | S | T17.3 |
| T17.6 Metrology: one Gear-based API; the Tooth forms become the test oracle | metrology#12, ablate-features#7 | low | M | T17.2 |
| T17.7 Strength and Hertz: Options that are never None, dead parameters and outputs | strength#14, added#29, added#34, ablate-constants-rating#13, lens-unification#11 (bindings) | low | S | T17.2 |
| T17.8 Test fixtures out of the public API | ring#12, lens-unification#11, lens-architecture#15 (`index_pub`), added#7 | low | S | T17.2 |
| T17.9 A field written and never read; an absence that cannot occur | kinematics-flow#8, ablate-constants-rating#2 | low | S | — |
| T17.10 Wire fields the panel never reads: listed by a check, not by hand | ablate-features#6 | low | S | — |
| T17.11 Web and repository leftovers | web#18, lens-docs-accuracy-1#20 | low | S | — |

### T17.1 Rustdoc gate; the 22 broken links fixed
**Change.** Fix each unresolved link. Where the item it names is gone, fix the sentence around it too.
- **Retired items.** Six links name items that are gone:
  - `outline.rs:3` and `tooth.rs:324` link `Tooth::profile`, which is now `Gear::profile`. Reword the tooth.rs sentence, since the method no longer lives on the type it describes.
  - `dxf.rs:35` links `gear_core::Tooth::outline`; the method is now `Gear::outline` or `Ring::outline`.
  - `gear.rs:255` links `Self::span`, which is now `extremes`.
  - `conditions.rs:162` links `Wiring::mounts`, and `wiring.rs:308` links `super::member_inputs`; delete both clauses.
- **train/mod.rs.** Nine sites:
  - 268 `crate::kinematics::System::play`
  - 740 `crate::strength::ToothOutline::rim_support`
  - 912: delete the `Loading::for_cases` sentence
  - 1118 `crate::auto::minimum_profile_shift`
  - 1711 `crate::note::Explain::note`
  - 2073: `stated_helix` becomes `Shape::helix_angles`
  - 2225: delete the `always_given` clause
  - 2804: `Port` becomes `PortBody`, or plain text
- **pair.rs:99.** Delete the `Decided::Given` row of the three-row table, because the enum has only `Chosen` and `Absorbed`. Say where a given shift's bound now lives.
- **Unqualified paths.** Qualify the rest:
  - `auto.rs:1032` becomes `crate::ring::Ring`
  - `wiring.rs:87` becomes `crate::kinematics::GROUND`
  - `strength.rs:331` becomes `rolling_curvature_radius`'s full path
  - `mesh.rs:765` and `metrology.rs:747` become `crate::note::Explain::note`
  - `gear-wasm lib.rs:1419` `Freedom` and `:1473` `Train::relieve_case` get their full paths
- **Smaller warnings.** Fix the ambiguous `super::preview` (a function and a module) and the 2 redundant link targets.
- **Unlinked stale reference.** Fix `shape.rs:1414`: `absorb`'s doc cites a `planetary::solve` that exists nowhere. It is not a link, so rustdoc will never catch it. Describe the bracketed Newton on its own terms [ablate-constants-rating#16].
- **The gate.** In `flake.nix` checks, add `doc = craneLib.cargoDoc (commonArgs // { inherit cargoArtifacts; RUSTDOCFLAGS = "-D warnings"; cargoDocExtraArgs = "--workspace --no-deps --document-private-items"; })`. Also add `doctest = craneLib.cargoDocTest (…)`: nextest skips the 4 doctests, and so does `check_bindings`' filter. Running with `--document-private-items` means the 22 public-to-private links resolve and stay checked instead of being silenced. They are deliberate: the private docs carry the reasoning. Add a row to CLAUDE.md's check table, and change "Thirteen checks" to fourteen.

**Proof.** Before fixing anything, add the check: `nix flake check` must fail at HEAD with 22 `unresolved link` errors. After the fixes it passes. Then, in a worktree, put back ``[`Self::span`]`` at gear.rs:255 and confirm the gate goes red. From here on the gate also proves every later task: moving an item under `#[cfg(test)]` breaks the links to it, which several of the tasks below would otherwise do silently.

### T17.2 Dead-pub gate with a shrinking allowlist
**Change.** Add `tools/check_dead_pub.py`. It copies the workspace to a temp directory and rewrites gear-core's `lib.rs` so every `pub mod` becomes `mod`. It then adds `pub use crate::m::Item;` for each gear-core item that gear-io, gear-wasm or gear-cli names, and builds with `-W dead_code`. Integration tests and `#[cfg(test)]` code are deliberately not roots.

The check compares its output against `tools/dead_pub_allow.txt`, one line per item with a reason, and fails both ways: on a dead item that is not listed, and on a listed item that is no longer dead. At HEAD the file holds today's 28 hits. Each later task deletes its lines. The items meant to stay are the reference oracles:
- `planetary::basic_ratio`, `power` and `carrier_driven_efficiency`. CLAUDE.md's file map names them as the check the flow is held to, so keep them public [ablate-constants-rating#16, over lens-unification#11's proposal to move them].
- The ring-cut functions of `verify.rs`, if they are meant as instruments.

Those lines move into CLAUDE.md's "Things that look wrong and are not", which becomes the allowlist's one prose home. The run also flags `note::ALL` and `planetary::Teeth`. Decide each: allowlist it with a reason, or delete it. Add a row to the check table and to `ci.yml`.

**Policy for the wider surface [lens-architecture#15].** Of the 144 `pub fn`s used only inside gear-core, many are the natural API of a public type, such as `Ratio`'s arithmetic. Narrowing those is not a defect fix. `unreachable_pub` cannot find them, because every module is `pub mod`. This gate is the tool for that job: an item it flags is either used, allowlisted with a reason, or deleted. Do no blanket `pub(crate)` pass.

**Proof.** At HEAD the tool reproduces `dead_cli.txt`: `Freedoms`, `Pinned`, `shifts_for_efficiency`, `split_residual`, `efficient_split`, `sliding_at`, and the four Tooth-based metrology forms, among others. Deleting one allowlist line fails the check. Adding a new unused `pub fn` fails it too.

### T17.3 Delete the stage-era search; aim its laws at the shape's search
**Change.** Lands as T12.5, whose first step ports `Pinned::box_of`'s pinned-sum interval into `Shape::chosen_at`'s box (shape.rs:1765-1781) [auto-search#7]. That is the one idea the dead code has that production lacks, and it is what reaches 9/37 at 24.28-24.29 mm. This section is the deletion's full list.

Then delete from `auto.rs`: `Freedoms`, `Bounds`, `Pinned`, `shifts_for_efficiency`, `sides`, `member`, the free fn `maximise`, and `Search::effort`, whose doc argues from a hula solver that no longer exists. Keep `Search`, `Search::SHIPPED`, `refined` and `maximise`. The Search and maximise docs should name their one caller, the shape's search at shape.rs:1893.

Re-aim `the_search_beats_a_scan_of_the_same_interval` (train/mod.rs:5231). Solve `arr::pair(teeth)` with `set_search(true)` and the distance fixed at the one each sum implies, then read `profile_shift` back. `solve_preset`'s refusal defines admissibility, so the separate pre-check goes. The three auto.rs tests at 2764, 2850 and 2876 move with it or go.

Fix the prose in the same commit:
- `gear-cli/src/main.rs:1541-1543`, which says it is "the only command that drives `auto::shifts_for_efficiency`". That is false.
- `contact.rs:495`
- `shape.rs:1790`, which cites `auto::Pinned::place`
- the Search doc's `[maximise]` link at auto.rs:1622

`docs/reference.md` and `rationale.md:478` describe `Freedoms` and `auto::maximise` as shared machinery. That fix belongs to [ablate-features#2], and it must land with this task.

**Proof.** Write the re-aimed law first and confirm it fails on the two mutations the current law misses:
- `shape.rs:1845-1846` swapped to `out[pa]=(s-d)/2, out[pb]=(s+d)/(2*sign)`. Today all 604 tests pass under this mutation, and `gear-cli shifts 9 37` moves from 0.6746/0.7332 to 0.6677/0.7640 at the same 97.678 %.
- the Division box at shape.rs:1829-1832 halved in width.

On the unmodified tree it passes at the existing 1e-5 tolerance. After the deletion, `check_dead_pub` reports nothing in auto.rs and its allowlist loses ten lines. Golden and corpus are byte-identical.

### T17.4 One relative-velocity primitive; the sliding docs made true
**Change.** The relative velocity ω₁â₁×p − ω₂â₂×(p−c) is written twice:
- `contact::sliding_velocity`, which only tests call;
- `Contact::slip` (contact.rs:731-738), which is live in `moment_per_force`.

Make `sliding_velocity` the one production primitive, taking the axes, the centre, the point and the speed ratio, and have `Contact::slip` call it with its arithmetic order unchanged. Move `sliding_at` and `Sliding` into contact.rs's test module. They are the instrument for the parallel-axis laws (pitch-point sliding is zero; sliding is perpendicular to the contact line at every helix). Reword the links to them at contact.rs:404, 550 and 628.

Keep screw.rs's closed form √(1 − 2k cos Σ + k²) (screw.rs:246-249), which gives the reported pitch-point sliding. Its test at screw.rs:1797 stays the independent check against the vector primitive: a closed form checked against kinematics, so no independence is lost [added#57's concern].

Delete the two false doc sentences: "arrives here as a value of this function rather than as a separate screw-gear formula" (contact.rs:600-603), and the "milestone 7, step 5" paragraph (contact.rs:630-635). The line-contact `sliding_velocity: 0.0` (train/mod.rs:494) is correct and documented, so leave it. Reporting the crossed pitch-point sliding through `Contact::slip` instead of `sliding_ratio` would leave one live route. That is optional here and only worth doing if the two differ.

**Proof.** A test asserting `Contact::slip` bit-equal to the old inline expression on every crossed preset. Golden unchanged. The parallel-axis tests (contact.rs:1409-1624) now exercise the production function. `check_dead_pub` drops `sliding_velocity` and `sliding_at`.

### T17.5 Contact split code and `tip_pressure_angle` out
**Change.** Delete `efficient_split` and the `ContactPath::tip_pressure_angle` field. The field is computed on every path at contact.rs:115, and nothing but `split_residual` reads it. For a ring it holds π − α_a: 167.972° where the true angle is 12.028° on 17/51. Move `split_residual` into contact.rs's test module, taking the two tip angles as arguments, as the record of ε₁ = ε₂. Wiring it in was weighed and not taken, and under the default `no_sharp_tip` its premise dr_a/dx = m is false, about 0.47 m.

Reword:
- auto.rs:1467-1475 and gear-cli main.rs:1084-1085, which link it;
- reference.md's "Gated against the loss …: the division the solver returns beats every other tried";
- rationale.md:500, "derived and checked".

corrections.md:564 is a historical record; leave it.

**Proof.** nextest, clippy and the rustdoc gate stay green. `check_dead_pub` loses its two contact.rs split lines. The ring value above is what makes the field's doc false; with the field gone there is nothing left to assert.

### T17.6 Metrology: one Gear-based API; the Tooth forms become the test oracle
**Change.** Move `span_over_teeth`, `best_span`, `pin_geometry` and `over_pins`, the Tooth twins of `span_over_teeth_at`, `best_span_around` and `over_pins_at`, out of `metrology.rs` into `tests/common` as the Δx = 0 oracle, stated as such. Fire the notes in `gear-io/src/strings.rs:1298/1319` through the Gear API. Keep `cutter_tip_width(&Tooth)` and `base_helix_angle`, which wasm and other modules use.

`pin_geometry` is not `Space::of(g).seat(d)`, because `seat` adds form, tip and root checks. The tests at tests/metrology.rs:175, 535, 629 and 669 expect `PinTooSmall` from the base-circle test alone, so the oracle keeps a raw seat in `tests/common`.

Make `between_pins` (metrology.rs:809) drop its parity branch, which contradicts the module's own principle at :560/:689. Seat at `z.div_ceil(2)` and take the chord as hypot(P_a − P_b). It cannot call `over_pins_at`, because a Ring is not a Gear.

Fix `best_span_around`'s doc. It says "averaged over the revolution", but the code takes the least worst-case offset. T09's rule change for choosing k [T09, metrology#18] lands in this single API afterwards. `span_over_teeth(g, 0)` silently computing k = 1 goes with the Tooth form.

**Proof.** In tests/metrology.rs, assert over the tests/common grid that both forms pick the same k and refuse the same gears, and that the values agree to 1e-15 relative. Not bit for bit: on 12,214 uniform gears the span differs by one ulp in 4,847 cases. On a 7,776-gear probe there are 0 k disagreements, 114 common refusals and a worst difference of 4.3e-16. The four M forms at tests/metrology.rs:674-679 stay the published oracle. Also assert `between_pins` equal to its old value within 1e-15 for odd and even z. `check_wasm` is unchanged. `check_dead_pub` loses four lines.

### T17.7 Strength and Hertz: Options that are never None, dead parameters and outputs
**Change.**
- `stress_correction`, `bending_factor` and `bending_stress` return `f64`. Every arm is `Some`, and stress_correction's own doc says "It is always defined". Delete gear-cli's "notch factor undefined" branch (main.rs:2307-2312), the `continue` in `worst_over_cycle` (strength.rs:1513, never executed), the "None where…" doc paragraphs, and the vacuous `.is_some()` asserts at 2568-2569.
- `finish` loses `vertex`, which it discards with `let _ = vertex;` at 790. The value is `crossing[1]`. Drop it at the three call sites (734/744/756). At 7 parameters the `#[allow(clippy::too_many_arguments)]` goes too.
- Delete `ToothOutline::flank_curvature` and both identical impls (`self.rb * u`). The one test uses `v.rb * sec.s`.
- Delete `EllipticalContact::approach` and `elliptic::r_f`, with r_f's doctest and tests. `approach` is read only by the sphere test. Every production contact pays for an R_F evaluation it never uses, and the `r_f(...)?` path could drop the elliptical term. That path is latent: 0 of 17,388 swept cases reach it.
- At strength.rs:794, `load_dir[0].abs().clamp(-1.0, 1.0).acos()` loses the sign of the along-tooth component. Where that component is tensile (α_Fen < 0), it is still subtracted as compression relief. Use `(load_dir[1] * load_dir[0].signum()).atan2(load_dir[0].abs())`: the cosine is unchanged and the sine carries the sign.
- The `1e-12` guard at 674 becomes `== 0.0`. It protects a division by zero and has never fired.
- hertz.rs:433's `x_tol` of 1e-14 becomes `Tol::default()`.
- Remove the discarded `(beta_b_1, beta_b_2)` at screw.rs:1416. verify.rs:311's binding belongs to [tooth-form#9].

**Proof.** A unit test on the section solve with a load line mirrored so that α_Fen < 0. It asserts that the axial term changes sign. It fails today, because the term is ≥ 0 by construction. With the signed angle, all 679 tests and the fast and matrix corpus were unchanged in a worktree. Everything else is a type change the compiler proves, with clippy `-D warnings` and a byte-identical golden corpus.

### T17.8 Test fixtures out of the public API
**Change.**
- **`ShaperCut::equivalent_to_rack`** (shaper.rs:474-529) becomes a private helper in shaper.rs's test module. It is production shaper.rs's only use of `Tooth`, so move `use crate::tooth::Tooth` with it: otherwise clippy fails on the unused import, which has been measured. Qualify the link at shaper.rs:237. Drop its `Option` return and the test `a_cutter_too_small_to_reach_the_root_is_refused`. That test checks only the fixture's own `z_c = 0` guard, and its name describes a branch it never reaches (line 502 is uncovered). Rewrite the module table's External row as "convergence instrument (tests only)". `MeshKind::External` itself stays; see Declined.
- **`Tooth::with_flank_clamped_at_base`** is a negative fixture whose only caller is tests/regression.rs:293. `#[cfg(test)]` does not reach integration tests, so move that regression test into tooth.rs's tests and gate the constructor `#[cfg(test)]`. `#[doc(hidden)]` is the fallback if the move is refused.
- **`PlanetaryShaft::index_pub`** goes. Make `index` a `pub(crate) const fn`. Its callers are planetary.rs:153-154 and kinematics.rs:1049.
- **`Ratio::cmp_checked`** goes with its test `comparison_is_exact_or_absent`. It has no caller, and its doc names one that does not exist.

**Proof.** `cargo clippy --workspace --all-targets -D warnings` (a `cfg(test)` gate alone fails it, as measured), nextest, the rustdoc gate, and four fewer lines in `check_dead_pub`.

### T17.9 A field written and never read; an absence that cannot occur
**Change.**
- **`MeshFlow::paths`** (flow.rs:65). `flow::solve` never reads it: the linear multiplier already totals the instances, and shape.rs:3284 divides by `wiring.meshes[k].paths`. Remove the field, its setters (train/mod.rs:4271, 4295) and the test builders. Say at flow.rs:43 and :122-124 that the torque is the total, and that shape.rs divides it.
- **`ShiftRange`.** `admissible_profile_shift` (auto.rs:371) is its only constructor and always sets both bounds. So `unwrap_or(-5.0)` and `unwrap_or(5.0)` (shape.rs:1479-1480) never bind, and the `?`s at shape.rs:1773 and auto.rs:827 never refuse: three readings of one impossible absence. Keep the serialized `bound: Bound`, since core.ts:242 reads every range alike. Add `ShiftRange::interval() -> (f64, f64)` and use it at all three sites. Refuse a NaN bound once, inside `admissible_profile_shift`: today `Some(NaN)` passes both the fallback and the `?`.

**Proof.** The ablation `shift_fallback` (−5 → −5.5) is silent in the suite and the corpus. After the change the literal no longer exists. Add a law over the tests/common grid: `interval()` is finite with min ≤ max. Golden and corpus are byte-identical, and `check_bindings` is unchanged.

### T17.10 Wire fields the panel never reads: listed by a check, not by hand
**Change.** Add `tools/check_wire_reads.py`. It takes every field of every type in `web/src/wire` and counts reads in `web/src` outside `wire/`. The check fails on an unread field that CLAUDE.md does not list, and on a listed field that is now read. Today it finds 6 of 251 with no read: `Material.condition`, `.source`, `.ultimate_measure`, `MemberGear.rim_thickness`, `CentreProfile.commanded` and `Figure.freedom`. `Material.class`, `.grade` and `Value.note` also go unread; they escape only because their names collide with other types. So match fields by type, not by name.

Replace CLAUDE.md's "Two inputs the panel never reads" with the check's list. Say which fields gear-cli's `materials` command prints (grade, condition, source, each `Value.note`: main.rs:2419-2429), so none of them is dead. Rewrite `Family`'s doc (material.rs:130-133) as "a library classification; the polyamide-estimate test filters on it". `REVERSED_BENDING_FRACTION` applies to any material and never reads `class`. Showing provenance in the panel is [added2#77]'s job, and Value.note's English is [ablate-features#4]'s (T02.1). Each field either one shows shrinks the list.

**Proof.** The check run at HEAD fails and names those fields. After the CLAUDE.md edit it passes. Reading `condition` in a scratch `.svelte` file then fails it, until the list line is removed.

### T17.11 Web and repository leftovers
**Change.**
- In `state.svelte.ts`, delete `Library.names` (:422), which has no reader, and the first of `Trains.remove`'s two doc comments (:375). That comment says a fresh tab is left; the code does not do that.
- In `app.css`, nothing sets `data-theme`. Delete the `:root[data-theme=…]` blocks (81-112), fold the duplicated dark palette into the media query, and drop the comment at 16-21. Keep `color-scheme: light dark`.
- In `core.ts`, replace the hand-kept 75-name import list and its re-export (14-168) with `export type * from './wire'`, and import only what the file uses (about 50 of those names appear only in the re-export).
- Delete the root `package-lock.json`. It is 6 lines with `"packages": {}`, and nothing reads it: flake.nix hashes `web/package-lock.json`.

**Proof.** `npm run check`, `nix build .#web` and `check_strings.py` pass. The bundle is no larger. `grep data-theme web/src` finds nothing.

### Declined
No finding here is dropped whole. Two halves are declined inside placed findings:
- ring#12: `MeshKind::External` on a `ShaperCut` stays. It is the σ parameter (rule 4) and costs no production line.
- ablate-constants-rating#16: the planetary closed forms stay public as documented oracles, and T17.2 allowlists them.

### Related, in other tasks
- T12.5 [auto-search#7]: port the pinned-sum box; T17.3 lands with it.
- T18.21 [ablate-features#2]: reference.md and rationale.md still name `Freedoms` and `auto::maximise`; lands with T17.3.
- T18.21 [ablate-features#13]: stale docs on `Gear::distinct_teeth` and `distinct`.
- T05.12 [ring#7]: `RingMesh` recomputes what `Mesh` and `ContactPath` give.
- T19.16 [lens-feature-gaps#10]: ratio-per-tooth sensitivity is computed and never shown.
- T20.11 [gear-cli#7]: the `dump` command.
- T13.8 [graph-ops#7]: pub `Train::hold`, `join` and `split` bypass the edit validation.
- T02.8 [lens-errors-policy#14]: dead fallbacks reading distance 0.
- T17.2's gate will surface each of these that is a public item with no caller.
