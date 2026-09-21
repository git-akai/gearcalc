# Working on this repository

A **map**, not a summary. The four documents in `docs/` say what the tool
computes, why, what was once wrong and what is built; this says where things are
and what it costs to change them.

It exists because the project is about 19,900 lines of production code carrying
14,000 lines of comment, alongside 5,500 lines of standalone document — **prose
and code are about 1 to 1**. That ratio is the reason the model
decisions here are auditable and it is not a target to reduce. What it does mean
is that finding the right file matters more here than in most codebases, and
until this file existed there was no way to do it but read the crate.

*(The first version of this paragraph said 1.5 to 1, having counted
`docs/history/`, which is the **superseded** design record that nothing points
at. A ratio quoted from a `wc` over a glob is a figure like any other; this one
counts `crates/**/*.rs` with the test modules split off, blank lines dropped,
at the tree the geartrain refactor's outliers pass closed on — 16,100 /
12,900 / 5,000 at the tree the audit closed on, and 17,200 / 13,600 / 5,100
where the refactor began. `geartrain-refactor-plan.md#5-what-gets-deleted` says where the 2,600 lines
of code went.)*

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
| `kinematics.rs` | **Shafts, meshes and what relates them** — one matrix read four ways: speeds, mobility, torques, play. A gear reaches it as a signed tooth count | " — no geometry, no stage, no loss |
| `hertz.rs` | General Hertzian contact; line contact is the degenerate value | " — a concave body is a negative radius, which is why there is no internal case |
| `plane.rs` | The normal and transverse planes, the identities between them, the basic rack | tooth counts |
| `params.rs` | A gear's inputs, and the record of any guard that altered them | how any of them is used |
| `note.rs` | What the solve wants a person to read, as a key plus values | any word of English |
| `tooth.rs` | **One tooth's form** at one shift, cut by one `Rack` | that a gear has more than one tooth |
| `gear.rs` | **The assembly**: teeth seated round an axis, cut by one tool. An ordinary gear is `Δx = 0` | anything about a mate, except through `mesh` |
| `shaper.rs` | Generation by a pinion cutter, of which the rack is the `z → ∞` limit | which of the two it is being used for |
| `ring.rs` | Internal gear geometry: the flank, where the tool sits, the limits | external gears |
| `outline.rs` | The gear outline as a CAD-ready closed path, to a chord tolerance | file formats |
| `mesh.rs` | Two gears in mesh: centre distance, operating angle, backlash | load, material, or strength |
| `contact.rs` | The path of contact and how load is shared along it | stress |
| `screw.rs` | Crossed-axis screw gearing — one model for a worm and a crossed pair | that a worm is special |
| `planetary.rs` | The set's **vocabulary** — sun, carrier, ring; an arrangement — and Pennestrì's closed form, the independent check the shape's flow is held to on every arrangement. No solve: the set's closure is the shape's | tooth form, geometry |
| `strength.rs` | The critical section, both notch models, `Y_F`, `K_f`, Hertz beside it. `ToothOutline` is the seam that makes one model serve a tooth and a ring | which stage is asking |
| `metrology.rs` | Span over teeth, over-pins, and what they take round a revolution | tolerances (that is `jgma.rs`) |
| `jgma.rs` | JGMA 116-02 tolerance tables, transcribed and checked | how a tolerance is used |
| `material.rs` | Elastic constants and stress allowables | where the numbers came from (that is the `basis` field) |
| `auto.rs` | Automatic values: the undercut shift, the tip-width addendum, admissible ranges, and `maximise` | what a stage is |
| `verify.rs` | The cut simulated from the cutter alone — the instrument, not the model | the model it checks |
| `testing.rs` | Numerical helpers the tests check closed forms against | *(test-only)* |

### `crates/gear-core/src/train` — the stage, and the presets over it

| File | Answers |
|---|---|
| `mod.rs` | **What every stage shares**: the load cases — each a list of `Load`s on the train's open ports, given or derived, solved in `solve_train` as one motion and one flow across every stage and handed down as a `CaseLoad` per stage; `TrainCase` is what it comes to shaft by shaft, `PathReport` what the train comes to along each path the ends or a case ask for (`paths_of`), and `Train::relieve_case` is the relief that keeps a case to the train's mobility — `StageLoads` (the lone-stage asked form), `MemberRating`, `Bending`, `MeshReport`, `GearResult` and its `GearCase` per load, the engagement rule, `Stage` (one variant, the shape, keeping its tag) and the train that strings stages together — and **relief**: `Freedom`, `Reading`, `FreedomGroup` and the walk over them, which a stage feeds through `Constrained` and never writes. `Constrained` is the whole of what a stage owes: six questions: the fifth is its wiring and the sixth its ports |
| `wiring.rs` | **Where a stage's shafts and meshes sit** — topology alone, no geometry, feeding `kinematics.rs`. The frame is *derived* from the two members' common frame rather than stored, and a mesh's sign is its `MeshKind`'s |
| `conditions.rs` | **What the train holds and ties together**: a `Constraint` on a `ShaftRef` (held or free), a `Coupling` between two stages' shafts — written, never assumed; `Train::chain` is the constructor that writes a chain's — and the `StageBoundary` each stage is handed, its own reporting convention, derived and never stored. `Train::motion` (the shaft line, driven at the train's first end where it has two), `open_ports`, `bodies` (the ports with the coupled shafts gathered), `ends`, and the **edits** the panel asks through `edit_train` — `couple`, `uncouple`, `hold`, `release`, `push_stage`, `remove_stage`, `fresh_case`, `set_duty` — each a rule about what else has to change. Nothing is driven but by a load |
| `shape.rs` | **The one stage**: axes (carried or not, replicated or not), shafts on them, members, meshes, distances — and the solve that reads what to do off that graph: mesh kind from a cutter, frame from the axes, wiring and ports from the shafts; each shift's role (given, free, reaches, absorbs) and the plan that closes every distance, an automatic one sized by its tips where they would cross or a gap is asked; the search in sum-and-division coordinates, per component where meshes share nothing, with the teeth it cut kept; the helix read once and propagated; every mesh built as a line contact or, on a distance at an angle, a point contact (`BuiltContact`, the one seam between the two models) and pressed with its driver's force; the power through each mesh and the ratio a tooth more would give; what each member is (`member_names`) and whether its planets assemble (`assembly`). A spur pair, a crossed pair, a worm, a planetary set and a hula stage are presets over it |
| `arrangements.rs` | **The arrangements the shape reaches with no code of their own** — a layshaft, a Wolfrom, a stepped planet, a planocentric, meshed planets, a Ravigneaux, a worm feeding a spur pair — each a list of what sits where through `Builder`; the harness's fixtures and the suite's textbook ratios name them | anything about solving |
| `flow.rs` | **Power mesh by mesh with loss**: an assumed direction per mesh, `2^M` assignments filtered by consistency, the rowspace solve written from each mesh's driver with the driven side at `η` — so a mesh with none *holds*. `Asked` says which shafts' torques are known and which are to be found, so a load case's derived loads and reacted ends are unknowns of the same solve. Reproduces Pennestrì's `η₀^w` on every arrangement and knows nothing of geometry |
| `pair.rs` | The pair **preset**: what `defaults` builds a spur or a worm from, and the undercut bound a search or an absorber is held to. No solve, no chooser — the tests' door onto the shape is `cfg(test)` |
| `planetary.rs` | The set **preset**: three members, the shafts they sit on, the boundary a convention gives it. No solve |
| `crossed.rs` | The worm's conventional proportions, the derivation of how play reaches a crossed mesh, and the tests that hold the shape to the screw model on every crossed pair. **No solve**: a distance at an angle is a point contact the shape builds beside its line contacts (`shape.rs`'s `BuiltContact`) |
| `hula.rs` | The hula **preset** — four gears on a crank as a designer states them — and `stage_efficiency`, the closed form the per-mesh flow is checked against. No solve: kinematically a compound planet with one planet and both meshes internal, which `Shape::from` lays out |

### The other crates

| Path | Role |
|---|---|
| `crates/gear-io` | DXF export · the TOML material library and geartrain documents · the string catalogues |
| `crates/gear-wasm` | The boundary. 21 entry points, JSON in and JSON out, all pure — `tools/wasm_boundary.json` lists them, and `check_wasm.sh` fails on one it does not |
| `crates/gear-cli` | The development harness. `gear-cli help` prints its subcommands, from the `COMMANDS` table that *is* its dispatch |
| `web/src` | Svelte 5 + TypeScript. Layout and event handling **only** |
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
| **A stage-level input** | `train/shape.rs` — its field on the shape, a member, a mesh or a distance, and its `Constrained` impl if relief may turn it or it argues with another · `train/mod.rs` if shared · `auto.rs` if a search reads it · 5 × `strings_*.toml` · `web/src/TrainPanel.svelte` | the above, plus `tools/check_bindings.sh --write` and `tools/check_strings.py`. The relief laws in `train/mod.rs`'s tests run over every preset, so a freedom the solve does not read fails there |
| **A load-case input** | `LoadCase`/`Load`/`LoadRole` in `train/mod.rs` · `solve_train` if it changes the motion, the flow or what a stage is handed (`CaseLoad`) · `Train::relieve_case` if it is a figure relief may turn · `conditions.rs`'s `bodies` if it changes what a case has a row for · `gear-wasm`'s `defaults` · `gear-io/src/train.rs`'s change log · 5 × `strings_*.toml` · `web/src/TrainPanel.svelte` | as above; `gear-cli train` and `--write` the corpus and `tools/check_wasm.sh --write` |
| **A per-gear input** | `params.rs` · the generator that reads it · `auto.rs` (`admissible_ranges`) · 5 × `strings_*.toml` · `web/src/GearPanel.svelte` — and, if it is a stage member's toggle, one line in `shape.rs`'s `inputs` and one in `MemberFreedom`, for every member at once | as above |
| **A stage arrangement** | none of the core, if the shape already holds it: a preset is a builder in `pair.rs`/`planetary.rs` and a `defaults` entry in `gear-wasm`, a button row in `web/src/core.ts`, pushed onto a train by `Train::push_stage`; its members are named by `Shape::member_names`. What the shape cannot yet lay out — a member on two axes, a planet–planet assembly rule — is a change to `shape.rs`, and **nothing in `MemberRating`, `MeshReport`, `Bending` or `GearResult` should move** | `cargo nextest run` — `every_arrangement_of_a_set_solves` and the relief laws sweep every preset and every arrangement · `gear-cli kinematics` and `tools/train_kinematics.py` · the corpus |
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
| `tools/worm_flank_curvature.py` · `crossed_path.py` · `hula_kinematics.py` · `train_kinematics.py` · `iso_6336_3_stack.py` | the crate against derivations that share no code with it | no — by hand |

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
  convention that names the wrong output, a preset that holds the wrong shaft
  each fail there). Measured by perturbing each and seeing what fired, not
  assumed. It is where a law belongs that decides it: a profile law wants the
  whole grid `tests/common` builds, a guard's value wants a recorded output,
  and a stage's law wants every preset.
- **Two notes nothing can fire** are named in `strings.rs`'s `UNFIRED` with
  their evidence. Live code, a live message, deliberately not deleted on
  suspicion — and the evidence carries the breadth of the search that found
  nothing, because an absence has a date. The newer one also carries a *test*
  that fails if the absence stops being true, which a sentence cannot.

---

## Getting started

```bash
nix develop              # or `direnv allow` once
cargo nextest run        # ~26 s
cargo run --bin gear-cli -- strength 17 43 2.0    # the regression canary
cd web && npm run dev    # the application
```
