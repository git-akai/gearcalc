# Working on this repository

A **map**, not a summary. The four documents in `docs/` say what the tool
computes, why, what was once wrong and what is built; this says where things are
and what it costs to change them.

It exists because the project is about 22,700 lines of production code carrying
14,900 lines of comment, alongside 6,200 lines of standalone document — **prose
and code are about 1 to 1**. That ratio is the reason the model
decisions here are auditable and it is not a target to reduce. What it does mean
is that finding the right file matters more here than in most codebases, and
until this file existed there was no way to do it but read the crate.

*(The first version of this paragraph said 1.5 to 1, having counted
`docs/history/`, which is the **superseded** design record that nothing points
at. A ratio quoted from a `wc` over a glob is a figure like any other, so the
`wc` is written down: `tools/line_census.py` counts `crates/**/*.rs` with the
test modules split off and blank lines dropped, beside `docs/*.md` without
`history/`, and takes the figure at any tree. It reads 16,100 / 12,900 / 5,000
at the tree the audit closed on, 17,100 / 13,600 / 5,100 where the
train-as-graph branch began, 21,900 / 14,700 / 6,100 once the train stored
one graph, and 22,700 / 14,900 / 6,200 at the branch's head, the cards
retired — so that branch added about 5,600 lines of production code against
1,300 of comment: 0.24 of comment a line, a third of the crate's own 0.66.
The reason is where it went: `shape.rs` gained 2,900 lines of solve for
what five stage types used to do separately, while `planetary.rs`, `hula.rs`, `pair.rs` and
`crossed.rs` gave up 2,800 between them — the comment those carried went
with them, and one solve says once what five said five times — and what
came after, the graph's edits, what a piece offers, the dry run and the
groupings, carries its why in `docs/` rather than beside it.)*

> The audit that built this file is closed. Its record is
> `docs/history/audit.md` — kept for its evidence, cited from code by finding
> number, and governing nothing: its faults are in `docs/corrections.md`, its
> decisions in `docs/rationale.md`, its residuals in `docs/state.md`.

---

## The rules that are not negotiable

Each was arrived at by something going wrong, and each is enforced by something
other than good intentions. `docs/rationale.md#the-standing-rules` argues them;
this is the short form.

1. **No engineering calculation in TypeScript.** If a number appears in the UI,
   Rust computed it. A *default* is one of those numbers, and so is a bound and
   so is an angle in degrees. Typing an engineering number into a `.ts` file is
   the bug.
2. **No English in `gear-core`.** The core emits a `Note` — a stable key and the
   values a sentence needs, with the numbers already formatted. Every word lives
   in `crates/gear-io/data/strings_<code>.toml`.
3. **Inputs are the only state.** Outputs are recomputed, never stored. A full
   train solve is microseconds.
4. **Find the parameter, not the branch.** A ring is a gear with a negative tooth
   count; a spur gear is helical at β = 0; a rack is a shaper at `z → ∞`. Every
   surviving `match kind` is a place two answers can silently disagree.
5. **Clamp rather than refuse, and say so** — where the input describes something
   that can nearly be cut. Refuse input that describes *no shape at all*.
6. **A conservative answer is not a free one.** "Conservative" states an error's
   direction and never justifies it. Every known bias is written down with its
   **size and sign** in `docs/state.md`.

---

## Which file answers which question

The load-bearing column is the last one. Knowing that `hertz.rs` has never heard
of a gear is worth more than knowing what it does.

### `crates/gear-core` — all the mathematics, no I/O, no UI, no wasm

| File | Answers | Does not know |
|---|---|---|
| `solve.rs` | Two bracketed root finders. Every transcendental step routes through them | anything about gears |
| `involute.rs` | `inv α = tan α − α`, and its safeguarded inverse | " |
| `elliptic.rs` | Carlson symmetric elliptic integrals | " |
| `ratio.rs` | An exact rational, `i128`, overflow refused rather than wrapped | " |
| `kinematics.rs` | **Bodies, meshes and what relates them** — one matrix read four ways: speeds, mobility, torques, play. A gear reaches it as a signed tooth count | " — no geometry, no stage, no loss |
| `hertz.rs` | General Hertzian contact; line contact is the degenerate value | " — a concave body is a negative radius, which is why there is no internal case |
| `plane.rs` | The normal and transverse planes, the identities between them, the basic rack | tooth counts |
| `params.rs` | A gear's inputs, and the record of any guard that altered them | how any of them is used |
| `note.rs` | What the solve wants a person to read, as a key plus values | any word of English |
| `tooth.rs` | **One tooth's form** at one shift, cut by one `Rack` | that a gear has more than one tooth |
| `gear.rs` | **The assembly**: teeth seated round an axis, cut by one tool. An ordinary gear is `Δx = 0` | anything about a mate, except through `mesh` |
| `shaper.rs` | Generation by a pinion cutter, of which the rack is the `z → ∞` limit | which of the two it is being used for |
| `ring.rs` | Internal gear geometry: the flank, where the tool sits, the limits | external gears |
| `outline.rs` | The gear outline as a CAD-ready closed path, to a chord tolerance | file formats |
| `mesh.rs` | Two gears in mesh: axis distance, operating angle, backlash | load, material, or strength |
| `contact.rs` | The path of contact and how load is shared along it | stress |
| `screw.rs` | Crossed-axis screw gearing — one model for a worm and a crossed pair | that a worm is special |
| `planetary.rs` | The set's **vocabulary** — sun, carrier, ring; an arrangement — and Pennestrì's closed form, the independent check the shape's flow is held to on every arrangement, and `carrier_driven_efficiency`, the 3K family's closed form the hula laws hold the flow to. No solve: the set's closure is the shape's | tooth form, geometry |
| `strength.rs` | The critical section, both notch models, `Y_F`, `K_f`, Hertz beside it. `ToothOutline` is the seam that makes one model serve a tooth and a ring | which stage is asking |
| `metrology.rs` | Span over teeth, over-pins, and what they take round a revolution | tolerances (that is `jgma.rs`) |
| `jgma.rs` | JGMA 116-02 tolerance tables, transcribed and checked | how a tolerance is used |
| `material.rs` | Elastic constants and stress allowables | where the numbers came from (that is the `basis` field) |
| `auto.rs` | Automatic values: the undercut shift, the tip-width addendum, admissible ranges, and `maximise` | what a stage is |
| `verify.rs` | The cut simulated from the cutter alone — the instrument, not the model | the model it checks |
| `testing.rs` | Numerical helpers the tests check closed forms against | *(test-only)* |

### `crates/gear-core/src/train` — the train's one graph, and the presets laid into it

| File | Answers |
|---|---|
| `mod.rs` | **The train, and what it comes to**: one graph (`Train::shape`), its holds and its load cases — each a list of `Load`s on the train's open ports, given or derived — solved in `solve_train` (`solve_parts`): every part cut once, one motion and one flow across the whole graph, and each part rated under its share of it, a `CaseLoad`. `TrainCase` is what a case comes to body by body; `PathReport` what the train comes to along each path a case asks for (`paths_of`) — its ratio, its efficiency with breaking away asked of the whole flow, its play, the power through its teeth, and what one more tooth on each gear does, by the graph's index; `Train::relieve_case` is the relief that keeps a case to the train's mobility. `TrainResult` answers per piece — every gear, mesh, distance and axis by the graph's index, what is a part's own (`PartReport`), and `part`/`by_part`, a part's view laid back out for the harness — through `MemberRating`, `Bending`, `MeshReport`, `GearResult` and its `GearCase` per load; the engagement rule; `Train::alone`, a preset asked alone as a train of one (its bodies numbered by its slots, cased at its conventional ends, solved part by part as any train — a preset is one part; `arranged` asks it held elsewhere, `solve_alone` reads the part and its first path, as `Alone`); and **relief** — `Freedom`, `Reading`, `FreedomGroup` and the walk over them, which the shape feeds and never writes. Six questions are the whole of what a shape owes (`shape.rs`, once the `Constrained` trait): the fifth is its wiring and the sixth its ports |
| `graph.rs` | **What the graph falls apart into**: `Shape::parts`, the pieces that close, search and rate apart — members joined by meshes, meshes by the distances they share — each a `Part` carrying its shape in its own numbering and where every piece of it is in the graph: what a stage was, and the solve's unit, never the designer's. the graph's own surgery (`append`, `merge_axes`); and **`graph_of`, a file of stages as one shape** — a body two stages shared listed once, the fixed axes it turned about one line, a join that cannot be coaxial an offset coupling — which is how a file written as stages converts (`gear-cli convert`); its laws hold the converted graph to what a chain builds, to the stages' motion body for body, and to their solves part by part |
| `shape.rs` | **The graph, and the solve that reads it**: axes (carried or not, replicated or not), the train's bodies on them (`BodyOn` — a body's place in a part's list is the part's slot for it), members, meshes, distances, couplings — and the solve that reads what to do off them: mesh kind from a cutter, frame from the axes, wiring and ports from the bodies; each shift's role (given, free, reaches, absorbs) and the plan that closes every distance, an automatic one sized by its tips where they would cross or a gap is asked; the search in sum-and-division coordinates, per component where meshes share nothing, with the teeth it cut kept; the helix read once per part and propagated; every mesh built as a line contact or, on a distance at an angle, a point contact (`BuiltContact`, the one seam between the two models); the solve in two halves — `cut`, everything no load moves, each mesh's efficiency with it, and `rate`, each member and mesh pressed with its driver's force in each case; what each member is (`member_names`) and whether its planets assemble (`assembly`). **No figure of its own**: a ratio, an efficiency, a play and what a tooth more does are a path's. The train is one of these, and so is every preset |
| `conditions.rs` | **What the train holds, and its bodies**: `held`, every body the train holds, each stated — a preset's conventional hold written when it is laid in — on a body numbered across the train, ground 0, which every part, case and hold names; `Train::parts` and `part_shapes` (the parts and their shapes, read off the graph), `port`, `slot` and `member` (a part's slot or gear in the graph), and `Train::chained`, the constructor that lays a chain's presets into the graph; each with an `_of(parts)` form for a caller holding the parts it solves by. `Train::motion` (the shaft line, driven at the headline case's first load — `headline`, `headline_load` — every body's exact speed and the headline path's ratio, and no part's: a ratio is a path's; a part its holds lock is refused at the hold that locked it, `check_parts`), `open_ports`, `bodies` (every port body with its ends), `motion_report` (**one list of bodies** — ends, `port`, `held`, speed, terms — which the picker and the case rows filter), `chain_ends` (where a case starts on a train with none: the first part's conventional input and the last part's conventional output, read with the holds); and the train's **edits** — `Train::edit` (the graph's, `edits.rs`'s `Edit`, which the panel asks through `edit_train`; the join, the hold and the insert of them made here, the rest on the shape), `join`, `split` (a part's end of a shared body made its own, for a train built in code), `hold`, `release`, `chain_on` (a preset laid in, `lay`, and joined onward) and `insert` (at a body named), `fresh_case`, `set_duty` — each a rule about what else has to change: a join is one body on one axis, the two axes one line (`merge`), every part keeping its own order of bodies (`keep_orders`), refused across an axis distance and between two bodies geared to each other, an offset coupling where an end orbits; every remove closes the numbers up (`drop_bare`, `drop_orphans`, `prune`). Nothing is driven but by a load |
| `edits.rs` | **The graph's edits** — `Edit`: a gear meshing any gear at a body, a new body or a new axis (`Place`), sized to the distance it crosses; a ratio on the body asked; a step; a coupling; a piece removed with what goes with it (`Piece`: a gear left meshing nothing, a body left bare, a distance with no mesh, an axis with nothing on it); a gear moved — each made on a copy and kept whole or refused whole (`Shape::apply`, `transact`), a planet never left meeting nothing on its carrier's axis, a body it adds numbered after the train's (`next`); a join, a hold and an insert are the train's (`Train::edit`, in `conditions.rs`), where a body nothing names any more leaves the train and the numbers close up. **A gear moved off a body does not take the body with it** — a body is a port the train may hold, share or load, and dropping it because its gear moved is how engaging a layshaft's other ratio lost the output; a body nothing names at all is given up by `Train::drop_bare`, a level up, where what else names it can be seen. A refusal (`EditRefused`) names its invariant, crosses as its catalogue key and changes nothing. No kind flips: a swap is a remove and an add, sized by the core |
| `offers.rs` | **What can be done to a piece**: every edit of the graph's set that applies at a gear, a mesh, a body, an axis, an axis distance, a coupling or the train, read off the graph by the rule each edit states and tried on a copy — the refusal's key where it is refused, and none that would change nothing. Laws: an offer is its edit, and every edit a brute-force sweep over every index finds the train makes is offered at every piece it names |
| `preview.rs` | **What an edit would do, before it is made**: the edited copy against the train, both solved, said as notes — the pieces whose count moves, the headline path kept, lost or found, the refusal, why the edited train would not solve. Compares; decides nothing |
| `groupings.rs` | **The graph grouped three ways**, each derived: centres (each pair of axes that mesh and every mesh at that spacing), axes (each axis, its bodies, their gears), and each case's **flow** — the bodies in the order its power reaches them, the meshes carrying it, an epicyclic part as one junction, an idle mesh a branch, and no share at all for a case that did not solve. Laws: every mesh at one centre, every body on one axis, every body and mesh said once in every flow, and no mesh called idle by a case that did not solve |
| `arrangements.rs` | **Every arrangement, as a list of what sits where** — the shipped presets included, since a pair, a crossed pair, a worm and a set are lists like the rest: `epicyclic` is the one builder for that family (the central members and the carrier in slot order, which is the order the conventions read) and `line` for parallel axes; `pair`, `crossed`, `worm`, `planetary`, `wolfrom`, `stepped`, `planocentric`, `meshed_planets`, `ravigneaux`, `layshaft`, `worm_and_pair` and `hula` (a stepped Wolfrom at one planet, at the proportions its family runs at — no preset, and the harness's `hula` commands build it from the list) are the lists; `Shape::with_first_helix`, `with_first_diameter` and `size_free` state one reading of a shape's size over any of them. `StagePreset` is a variant per menu entry with its family (`StageFamily`, which `Shape::family` reads off any shape), its name's key and its starting shape; `defaults()` crosses `StagePreset::ALL`. Knows nothing about solving |
| `wiring.rs` | **Where a part's slots, meshes and couplings sit** — topology alone, no geometry, feeding `kinematics.rs` (an offset coupling is a row, `ω_a = ω_b`, and no mesh); `add_to` is handed the slot-to-body lookup, which is the part's own list of bodies and the identity for a preset asked alone. The frame is *derived* from the two members' common frame rather than stored, and a mesh's sign is its `MeshKind`'s |
| `flow.rs` | **Power mesh by mesh with loss**: an assumed direction per mesh, `2^M` assignments filtered by consistency, the rowspace solve written from each mesh's driver with the driven side at `η` — so a mesh with none *holds*. `Asked` says which bodies' torques are known and which are to be found, so a load case's derived loads and reacted ends are unknowns of the same solve. Reproduces Pennestrì's `η₀^w` on every arrangement and knows nothing of geometry |
| `pair.rs` | **Who decides a shift and what it must satisfy** — `ShiftAsked`, the search floor and the true minimum told apart, and the undercut bound a search or an absorber is held to. A pair was a stage type here, then a vocabulary; it is `arrangements::pair` now, and this is what was left |
| `planetary.rs` | **The set's words** (`Train::arranged_as`): sun, carrier, ring — the words the harness and the shape's laws ask a lone set in — as the hold and the case a train of one set is asked with. The set itself is `arrangements::planetary`. No solve |
| `crossed.rs` | The worm's conventional proportions, the derivation of how play reaches a crossed mesh, and the tests that hold the shape to the screw model on every crossed pair. **No solve**: a distance at an angle is a point contact the shape builds beside its line contacts (`shape.rs`'s `BuiltContact`) |

### The other crates

| Path | Role |
|---|---|
| `crates/gear-io` | DXF export · the TOML material library and geartrain documents (`train::convert` reads one written as stages, once) · the string catalogues |
| `crates/gear-wasm` | The boundary. 23 entry points, JSON in and JSON out, all pure — `tools/wasm_boundary.json` lists them, and `check_wasm.sh` fails on one it does not |
| `crates/gear-cli` | The development harness. `gear-cli help` prints its subcommands, from the `COMMANDS` table that *is* its dispatch |
| `web/src` | Svelte 5 + TypeScript. Layout and event handling **only** — `members.ts` names a gear, a body and an axis the one way both panels do, from the core's reading of the graph; `Offers.svelte` lists what the core offers at the selection (`offers.ts` names it) — the adds as the one menu, the rest as a strip of verbs, each entry's dry run on hover |
| `web/src/wire` | **Generated** by `ts-rs`. Never edited by hand |
| `tools/` | The checks that live outside the Rust suite |
| `handoff_inbound/` | Prior Python work. **Reference only** — do not build on it |

---

## To change X, touch these

Traced, not guessed. The fan-out is real and most of it is load-bearing — five
string catalogues is what five languages costs.

| Change | Files | Then run |
|---|---|---|
| **A model or formula** | the one module in `gear-core` | `cargo nextest run` · `tools/check_golden.sh` · `tools/check_figures.py` |
| **An input of the graph** — a member's, a mesh's, a distance's, an axis's | `train/shape.rs` — its field on the member, the mesh, the distance or the axis, and its `inputs`/`freedoms` if relief may turn it or it argues with another · `train/mod.rs` if shared · `auto.rs` if a search reads it · 5 × `strings_*.toml` · the workspace in `web/src/TrainPanel.svelte` | the above, plus `tools/check_bindings.sh --write` and `tools/check_strings.py`. The relief laws in `train/mod.rs`'s tests run over every preset, so a freedom the solve does not read fails there |
| **A load-case input** | `LoadCase`/`Load`/`LoadRole` in `train/mod.rs` · `solve_train` if it changes the motion, the flow or what a part is handed (`CaseLoad`) · `Train::relieve_case` if it is a figure relief may turn · `conditions.rs`'s `bodies` if it changes what a case has a row for · `gear-wasm`'s `defaults` · `gear-io/src/train.rs`'s change log · 5 × `strings_*.toml` · `web/src/TrainPanel.svelte` | as above; `gear-cli train` and `--write` the corpus and `tools/check_wasm.sh --write` |
| **A per-gear input** | `params.rs` · the generator that reads it · `auto.rs` (`admissible_ranges`) · 5 × `strings_*.toml` · `web/src/GearPanel.svelte` — and, if it is a train member's toggle, one line in `shape.rs`'s `inputs` and one in `MemberFreedom`, for every member at once | as above |
| **An arrangement or a preset** | none of the core, if the shape already holds it: an arrangement is a list in `arrangements.rs`; a *preset* is that list, a `StagePreset` variant with its family and label, and one string ×5 — `defaults()` crosses the list, the add menu offers it at the train's output and at every body (`Train::offers`), and `Train::insert` lays it in; its members are named by `Shape::member_names`. What the shape cannot yet lay out — a member on two axes, a planet–planet assembly rule — is a change to `shape.rs`, and **nothing in `MemberRating`, `MeshReport`, `Bending` or `GearResult` should move** | `cargo nextest run` — `every_arrangement_of_a_set_solves` and the relief laws sweep every preset and every arrangement · `gear-cli kinematics` and `tools/train_kinematics.py` · the corpus |
| **An edit the panel asks for** | the *graph's*: `train/edits.rs` — the `Edit` variant, its rule on the shape (`Shape::apply`) or the train's (`Train::edit`), its refusal (`EditRefused`, whose key is what crosses) · `edit_train`'s `Graph` arm in `gear-wasm` and a step in `tools/wasm_probe.mjs` · where it is offered, in `train/offers.rs` · its name in `web/src/offers.ts` · a refusal's words, 5 × `strings_*.toml`. A *case's*: the method on `Train` in `train/conditions.rs` — what else has to change is the rule — · a `TrainEdit` variant beside `Graph` · the same probe step and the panel's control | `cargo nextest run` — `every_add_on_every_preset_solves`, `every_add_undoes` and `a_refused_edit_changes_nothing` sweep the presets, and `every_gear_the_graph_admits_is_refused_whole_or_undoes` and `a_removal_takes_what_goes_with_it_and_leaves_nothing_hanging` sweep the graph's, and `every_edit_the_train_makes_is_offered` fails on one the train makes and no piece offers; the train's edits have their laws in `train/mod.rs` and `train/edits.rs` · `tools/check_wasm.sh --write` |
| **A note the solve emits** | `note.rs` (the key) · the site that raises it · 5 × `strings_*.toml` | `cargo nextest run` — `gear_io::strings` checks both directions by *firing every note* |
| **A UI string with no `Note` behind it** | 5 × `strings_*.toml` · the `.svelte` that reads it | `tools/check_strings.py` |
| **A material** | `crates/gear-io/data/materials_default.toml` | `cargo nextest run` — every non-datasheet value must carry a note saying what it is |
| **A CLI subcommand** | one row in `gear-cli/src/main.rs`'s `COMMANDS`, carrying how its output is recorded | `tools/check_golden.sh --write` — the script asks the binary, so there is no second list |
| **A type that crosses the boundary** | the Rust type | `tools/check_bindings.sh --write`, then `cd web && npm run check` |
| **A wasm entry point** | `gear-wasm/src/lib.rs` · a call in `tools/wasm_probe.mjs` | `tools/check_wasm.sh --write` — it fails on an entry point with no probe, so the two cannot drift |
| **A documented figure** | the document | `tools/check_figures.py` — and tag the block with what generates it |
| **A language** | one new `strings_<code>.toml` · the list in `gear-io/src/strings.rs` | `cargo nextest run` — a translation that falls behind English's key set fails |

---

## Which check catches what

Thirteen checks in six different ways. `nix flake check` is **not** all of them.

| Run | Catches | In CI |
|---|---|---|
| `cargo nextest run` | The suite: laws, independent verifications, invariants, canaries, negative fixtures. **No count is quoted here** — a number that dates belongs in `docs/state.md`, and this file had one stale within an hour of being written | via `nix flake check` |
| `cargo clippy --all-targets -- --deny warnings` | `unwrap` in production is a warning, and warnings are denied | " |
| `cargo fmt --check` | | " |
| `nix build .#web` | **the site — `flake check` does not cover it**, and it carries a fixed-output hash over `web/package-lock.json` that nothing else consults | yes |
| `cd web && npm run check` | types, which the bundler strips without checking | yes |
| `tools/check_bindings.sh` | `web/src/wire` still matches the Rust it is generated from | yes |
| `tools/check_doc_links.py` | every pointer into the documents resolves, both from code and between documents | yes |
| `tools/check_strings.py` | every `ui.` message is used and every use has a message | yes |
| `tools/check_golden.sh` | **any number the harness prints that moved.** A change detector, not a correctness gate: a diff is a question | yes |
| `tools/check_figures.py` | every figure the documents print is one the code still prints | yes |
| `tools/check_units.py` | **an angle that does not say its unit, or a name that means both.** The crate is degrees where a designer states a number and radians in the mathematics; a name meaning one in one module and the other in the next is how that becomes a bug, and it did | yes |
| `tools/check_wasm.sh` | **the payload, executed** — everything else checks the boundary's shape or `gear-core`'s values, and nothing ran the `.wasm` the browser downloads. Asserts a law (optimising it changes no answer), records what it answers, and fails if an entry point has no probe | yes |
| `python3 tools/validate_dxf.py` | an export read back by a parser that shares no code with the writer | yes |
| `tools/worm_flank_curvature.py` · `crossed_path.py` · `hula_kinematics.py` · `train_kinematics.py` · `breakaway.py` · `iso_6336_3_stack.py` | the crate against derivations that share no code with it | no — by hand |

**Before pushing, run everything the table marks "yes"** — not a chosen
subset. It was "run five" here, and the five omitted `check_units.py`: a
field named `sigma` for a mesh kind's sign went out green on every one of the
five and red on CI, the second red build this paragraph has cost by naming
fewer checks than CI runs. The one-liner is the table; the cheap ones take
seconds.

The corpus is on that list because of a measurement, not for symmetry.
Perturbing five of the rating model's cited constants — `K_f`'s `H` and `L`,
ISO's `Y_S`, the tangent angle, the reversed-bending fraction — leaves the
**entire test suite silent**, and the corpus catches every one. A `nextest` run
is not evidence that the strength model is the one that was there yesterday.

---

## How to test here

In rough order of what has actually caught things.

1. **Verify against something that shares no code.** The rack simulation, the
   pin-tangency measurement, `ezdxf`, the contact-half-width route, the crossed
   path differentiated off the flanks. Every one caught something self-consistent
   tests had passed.
   — *and ask what the independent tool does with a defect.* `ezdxf` builds a
   document, supplying whatever the file omitted, and then agreed it was sound.
   SOLIDWORKS refused it.
2. **Ask what property the answer must have**, checkable without knowing the
   answer.
3. **Prefer a law to a threshold.** A bound taken from a measurement records
   where the sweep stopped, not a fact about the thing.
4. **Turn every axis, in every context it reaches.** `tests/common/mod.rs` is one
   grid with every input nameable. A control turned on a lone gear and left at
   its default wherever it meets a mate is untested.
5. **Before trusting a new gate, run it against the broken code.**
   `git worktree add` a detached HEAD, copy the test in, watch it fail. A gate
   that cannot fail is not a gate — this project has three recorded that could
   not.

---

## Things that look wrong and are not

Four in `tooth.rs`, and the tests that hold each:

1. **The flank continues below the base circle** to its true intersection with
   the trochoid. Clamping and bridging leaves a 0.3 mm step on undercut gears.
2. **The fillet fit cap is `w_tip·cos α / (2(1 − sin α))`.** The plausible
   `w_tip / (2 cos α)` silently shrinks the fillet on every shifted gear.
3. **`θ` is not monotone** along the profile. Undercut gears are legitimately
   re-entrant; the invariant is monotone **radius**.
4. **A rack's figures do not carry to a pinion cutter.** `ShaperCut` refuses a
   tool it cannot hold rather than clamping it.

And elsewhere:

- **`CriticalSection::TangentAngle` and `RootStressModel::Iso6336` look like dead
  code.** No stage reaches them. They are the *instrument* — `gear-cli matrix`
  runs both coherent sets, and it is the only way this tool can be checked
  against a published standard. They go when that command does.
- **`verify.rs` is in the library rather than in `tests/`** so the CLI can sweep
  it over thousands of cases.
- **Eight modules carry no `#[cfg(test)]` module** — `metrology.rs`,
  `params.rs`, `tooth.rs`, `verify.rs`, `train/pair.rs`, `train/conditions.rs`,
  `train/wiring.rs`, `train/planetary.rs` — and seven of them are covered from
  somewhere else: the integration suite for the first, third and fourth, the
  golden corpus for the guards in `params.rs`, and `train/mod.rs`'s and
  `train/shape.rs`'s laws for the last three (a wiring that miscounts paths, a
  convention that names the wrong output, a preset that holds the wrong body
  each fail there). Measured by perturbing each and seeing what fired, not
  assumed. It is where a law belongs that decides it: a profile law wants the
  whole grid `tests/common` builds, a guard's value wants a recorded output,
  and a stage's law wants every preset.
- **One note nothing can fire** is named in `strings.rs`'s `UNFIRED` with
  its evidence. Live code, a live message, deliberately not deleted on
  suspicion — and the evidence carries the breadth of the search that found
  nothing, because an absence has a date. A note whose *site* goes, goes with
  it: two left that way when a stage lost its own motion.

---

## Getting started

```bash
nix develop              # or `direnv allow` once
cargo nextest run        # ~26 s
cargo run --bin gear-cli -- strength 17 43 2.0    # the regression canary
cd web && npm run dev    # the application
```
