# The Train as One Graph — Plan

**What this is.** A worked plan for the step `geartrain-refactor-plan.md`
stopped short of: the train stops being a list of stages and becomes one
graph — axes, bodies, gears, meshes, distances — and the *stage* survives
only as what a preset inserts and as groupings read off the graph, never as a
container. It is a working note like that one, and is deleted once built.

The interface half is drawn, not described, on one canvas:
<https://claude.ai/artifact/6NknQ9eiqDAxea31E2LKnb>. Its upper rows are the
select-and-act model this plan starts from; its lower rows — *the page*, *one
list, three ways*, *working a mesh*, *a run of three* — are §3. Figures on the
boards are the application's own, read off the running app for a
three-stage train built from the menu's presets (a spur pair, a layshaft, a
compound set); where one was not printed it is left off rather than made up.

**The claim in one sentence.** The solve already has no stages in it; the
model keeps them only as a partition that can disagree with the graph — two
axes for one shaft, a distance that cannot be stated, a convention that is an
input nobody wrote — and the refactor is to delete the partition and read
every grouping a designer needs (by flow, by centres, by axis, by mesh) off
the one graph instead.

---

## 1. What a stage is today, measured

At `b535ca5`.

| File | Lines | What is stage-shaped in it |
|---|---|---|
| `train/mod.rs` | 10877 | `solve_train` solves every stage twice (bare, then loaded) and hands each a `CaseLoad`; `StageLoads`/`solve_any`, the lone-stage asked form; `paths_of` seeds its rows from the first stage's input and the last stage's output |
| `train/shape.rs` | 5834 | `Shape` *is* a stage: `optimisation`, `load_sharing` and `min_planet_clearance` sit on it; `solve_shape_after` solves a lone-stage motion under a boundary before anything else |
| `train/conditions.rs` | 1451 | `StageBoundary` and the conventions; `boundaries()`, `ports`, `ends`; the edits keyed by stage (`edit_stage`, `move_end`, `split`) |
| `train/edits.rs` | 1087 | `StageEdit`, whose every variant is relative to one stage |
| `gear-wasm/src/lib.rs` | 3066 | `relieve_stage`; `edit_train`'s `Stage` arm; `adopt_member` by stage and member; `solve_train`'s per-stage results and topology |
| `web/src/TrainPanel.svelte` | 3247 | one card per stage, and everything in it |

`.stages[` is written 85 times in the core and 9 times outside it;
`StageBoundary`/`boundaries` 89 times; `solve_any`/`StageLoads` 212 times,
nearly all in tests; `StagePreset` 37.

### 1.1 What the solve already does without stages

- **The motion is one system.** `Train::system()` builds one `System` over
  the train's bodies and adds each stage's wiring into it through the
  slot-to-body lookup; the stage is a fragment of the matrix, not a boundary
  in it.
- **The flow is one solve.** `flow.rs` assigns directions over every mesh of
  the train at once (`2^M`, filtered), and a case's derived loads and reacted
  ends are unknowns of that one solve.
- **Bodies are the train's.** One number across the train, ground 0; an
  axis's `carried_by` is already a train body.
- **The search is per component, not per stage.** `search_components`
  groups the free axes by the meshes they can move — members shared, *or
  meshes tied through one automatic distance by an absorber*. A layshaft's
  three meshes are one component; so is a set's two; so would be two
  "stages" if they shared either.
- **Mesh groups are derived.** `Shape::mesh_groups` — the connected
  components of the mesh graph — decide which members share a module and a
  pressure angle. Nothing stores them.
- **Paths already cross stages.** `PathReport` gives ratio, efficiency,
  circulation and backlash between any two bodies the graph can answer.
- **The independent oracle has never had stages.** `tools/train_kinematics.py`
  builds shafts, gears and meshes directly and agrees with the crate on every
  arrangement.

### 1.2 What a stage still owns

Four things, each with a better home (§2):

1. **Axes and distances.** A distance joins two axes of one shape.
2. **Three inputs**: `optimisation`, `load_sharing`, `min_planet_clearance`.
3. **Its figures**: ratio, efficiency, circulation, backlash, and one more
   tooth — each read under the stage's *own* motion, solved alone under its
   convention.
4. **Being what a preset makes**, and what `push_stage` appends.

### 1.3 The defect: one shaft, two axes

A body a stage lists turns about an axis *of that stage*. A body two stages
share — every coupling there is — therefore turns about two axes, one in each
shape, and nothing in the model says they are the same line, or parallel, or
anything at all.

On a three-stage train built from the menu — a spur pair, a layshaft, a
compound set — that is six axes and two doubled shafts. As one graph it is
**four** — the input's, one long centreline carrying Bodies 2, 3, 5, 6, 7 and
8, the layshaft, and the planets' — and the collapse is not a simplification
but the truth of the machine: a shaft is straight.

What the doubling costs:

- **The train cannot be drawn.** A schematic needs to know how each part
  sits relative to the rest, and the model does not hold it.
- **Coaxiality is unstatable**, though it is true of nearly every gearbox.
- **Two groups on one pair of axes cannot share a distance.** Another ratio
  across existing centres must live in the same stage or lose them.

### 1.4 Conventions are inputs nobody wrote

`StageBoundary` is derived every solve: the first ring listed is held, the
first free port is the input, the next the output. The panel shows the first
as "Fixed", and `Train::release` writes it off "in so many words" — a hold
that is only there until someone looks at it. The train's headline row is
`Train::ends`: the first stage's input to the last stage's output, so it
depends on the order the stages were pushed in, which nobody chose as such.
Both are inputs the file does not contain and the designer never stated.

---

## 2. The model

### 2.1 One graph

```rust
pub struct Train {
    pub axes: Vec<Axis>,             // a body turns about exactly one
    pub bodies: Vec<BodyOn>,         // each body once: its axis
    pub members: Vec<Member>,
    pub meshes: Vec<MeshInput>,
    pub distances: Vec<Distance>,    // one per pair of axes that mesh
    pub constraints: Vec<BodyConstraint>,
    pub load_cases: Vec<LoadCase>,   // and the paths the train reports (§2.4)
    pub reversed_bending: bool,
}
```

Structurally this is today's `Shape` with the train's constraints and cases
beside it, and `Shape` can stay the name of the graph part so that
most of `shape.rs` moves by deletion rather than by rewrite.

**The invariants**, each a refusal that names itself as `EditRefused` does
today, and each checked on load:

1. every body turns about exactly one axis;
2. a carried axis is carried by a body on *another* axis;
3. one distance per pair of axes that mesh, and none between axes that do not;
4. every member is in a mesh, and a mesh's two members are on two axes with a
   distance between them;
5. every body a member, an axis, a hold or a case names exists.

### 2.2 Where every stage input goes

Every input ends on the thing it describes. Where two or more of the same
input are bound by a relation, the established rule holds — **given on at
most one, automatic on the rest, and relief keeps it so** — which is the
helix's rule and the tooth-thickness coefficient's today.

| Today | Moves to | Rule |
|---|---|---|
| `Member::module` (plain; the panel writes it to every member of a mesh group) | `Member::module: Auto<f64>` | equal across the mesh group; given on at most one member; `MemberFreedom::Module` |
| `Member::pressure_angle` (the same) | `Member::pressure_angle: Auto<f64>` | the same; `MemberFreedom::PressureAngle` |
| `Shape::load_sharing` | `MeshInput::load_sharing` | per mesh: the ramp is a model of one contact |
| `Shape::optimisation.enabled` | `MeshInput::search: bool` | per mesh, effective per search component (below) |
| `Shape::min_planet_clearance` | `Axis::min_tip_gap` | per replicated axis, which is where it is reported (`LayoutReport`) |
| `MeshInput::overlap`, `min_contact_ratio`, frictions | stay | already the mesh's |
| `Distance` (angle, distance, clearance, tolerances, tip gap, float, worm) | stay, train-wide | one per pair of the train's axes |
| `Axis::carried_by`, `count` | stay, train-wide | |

**Module and pressure angle join the helix's rule.** Where none of a group is
given, the first member's box stands — the helix's precedent, "where nothing
decides it, the first member's stands at its box". A file that writes plain
numbers is read as given on each group's first member and automatic on the
rest, and is refused, naming the members, where two members of one group
differ (what `Mesh::new` refuses today).

**The search is honest about its unit.** Its unit is the component
`search_components` finds, not a mesh: a planet's shift moves both its meshes,
and an absorber ties every mesh on an automatic distance together. So the
switch is stored on each mesh, a component is searched where **any** of its
meshes asks, and the panel shows the one switch on every mesh of the
component with the others named — "searched with Gear 3 ⇄ Gear 4 and Gear 5 ⇄
Gear 6". Stored per mesh because components are derived and move under edits,
and a stored per-component flag would be an input that can disagree with the
graph. (Decision 1.)

### 2.3 Where every stage output goes

| Today, on `ShapeResult` | Moves to |
|---|---|
| `ratio`, `efficiency`, `circulation`, `backlash` — under the stage's own convention | the **paths** (§2.4): a `PathReport` per case's load→reaction |
| `ratio_per_tooth` — one more tooth, against the stage ratio | per member, **per path**: what one more tooth here does to each |
| `members: Vec<GearResult>` | train-wide, per member |
| `meshes: Vec<MeshReport>` | train-wide, per mesh |
| `distances: Vec<DistanceReport>` | train-wide, per distance |
| `layouts: Vec<LayoutReport>` | per replicated axis (`AxisReport`) |
| `cases: Vec<SlotCase>` | gone: `TrainCase` already says every body's speed and torque |
| `notes` | on the thing they concern — a member, a mesh, a distance, an axis, a case |

### 2.4 A case is the reading

The train reports a path for every case — from each of its loads to each of
its reactions, once — as `paths_of` does now, **minus the conventional
ends**, which go with `Train::ends`. A stage's ratio was always a proxy for
one of these ("Body 2 to Body 3"); a case says which, and can cross what used
to be a stage boundary.

There is no second input for it. A path worth watching is a case worth
running, and a fresh train starts with a default case — its first preset's
conventional input loaded, its output reacted — so a new spur pair still opens
on its ratio, and says under what load. (Considered and dropped: a
`readings` list beside the cases. It would have been two ways of naming the
same path, one of them unloaded.)

### 2.5 Presets insert, and write what their conventions meant

A preset builds a **fragment** — axes, bodies, members, meshes, distances —
with its conventional input, output and holds named in it. Inserting one:

1. numbers its bodies after the train's largest (today's rule);
2. where it is inserted **at** a body, its input body *is* that body — and its
   input axis is that body's axis, which is the coaxiality `push_stage` joined
   without stating;
3. writes its conventional holds as `BodyConstraint`s, so "Fixed" is a stated
   hold from the moment it appears;
4. carries the case entries and holds at the old output on to the new one,
   as `push_stage` carries case entries now; on an empty train, writes the
   default cases at the preset's conventional input and output.

`StagePreset::ALL`, `defaults()` and the menu stay as they are. What goes is
the preset's *afterlife*: once inserted, a planetary set is gears on axes like
everything else, and can be edited into anything the graph admits.

**Old files are converted once, not read for ever.** The format refuses a
shape it no longer has, by name, and says how to convert — its own argued
policy, which a second reader would break. A `gear-cli convert` does the
conversion: concatenates a stage-shaped file into one graph, axes merged
where a body is shared, and writes each stage's convention as explicit
holds; its cases already name the paths it reports. A law holds every
converted fixture's figures to the stage-shaped solve it came from (§6).

### 2.6 One edit set

| Today | Becomes | Note |
|---|---|---|
| `AddCentral { gear, ring }` | `AddGear { mate, on: NewBody(central axis), ring }` | a sun or a ring on a planet gear is a gear meshing it |
| `AddAxis` | `AddGear { mate, on: NewAxis { carried_by: ground }, ring: false }` | an axis meshing **any** gear, not the last — the order dependence goes |
| — | `AddGear { mate, on: Body(b) }` / `NewBody(axis)` / `NewAxis {…}` | everything the four above guessed at |
| `AddMesh { distance }` | `AddRatio { distance, shared: Body }` | a compound, two gears; which body is shared is **asked**, not guessed |
| `AddStep { axis }` | `AddStep { axis }` | a compound: a gear on the planet's body with a ring on it |
| `push_stage` | `Insert { preset, at: Option<Body> }` | §2.5 |
| `RemoveMember`, `RemoveMesh`, `RemoveStep`, `RemoveAxis`, `remove_stage` | `Remove(Member \| Mesh \| Axis \| Body)` | each with its rule of what goes with it: a mate left meshing nothing, a body left bare that nothing names (`drop_bare`), an axis left empty and its distances |
| `MoveBody` | `Move { member, to: Body \| New }` | unchanged rule, including the body it leaves staying |
| `split(stage, body)`, `move_end` | `Move` of the members that leave | a split was always "these gears onto a body of their own" |
| `join(a, b)` | `Join { a, b }` | **merges the axes** where the two turn about different ones — refused where one is carried and the other is not, or where two distances would join one pair of axes and disagree beyond relief |
| `hold`, `release` | `Hold(b)`, `Release(b)` | unchanged |
| `relieve_stage` | `relieve(freedom)` | freedoms indexed train-wide |
| cases, duty | unchanged | |

`AddRatio` and `AddStep` survive as compounds because each is a genuinely
different intent — another ratio, another step — that happens to take two
gears; everything else a card did is `AddGear`, `Remove`, `Move` or `Join`.

### 2.7 The solve

1. **Motion**: `train.system()` over the graph — unchanged in substance.
2. **Geometry, once**: today's `solve_shape_after` over the one graph *with
   no boundary* — closure per distance, search per component, the helix,
   contacts, sizes. The lone-stage motion under a convention is deleted.
3. **Flow, per case**: today's.
4. **Ratings from each case's flow directly.** The two-pass structure — solve
   every stage bare, then again with the `CaseLoad`s the train derived —
   exists because a stage was solved before the train knew its loads. With
   one graph it is one pass: geometry, then flow, then each mesh pressed with
   its own driver's force. `Chosen` survives only where relief needs a prior.
5. **Results**: `TrainResult { paths, cases, members, meshes, distances,
   axes, notes, topology }`, where `topology` carries every grouping the panel
   draws (§3) — computed in Rust, because the *order* of the flow grouping
   reads power direction, which is a result, and one rule should serve the
   harness and the panel as `member_names` does now.

The flow's `2^M` is already over every mesh of the train and does not change.
The geometry is the same work done once instead of twice.

---

## 3. The interface

Drawn on the canvas's lower rows: the page, the same page with the flow
drawn across rather than down, the list grouped three ways, the mesh
workspace, and a run of three gears. What the boards establish:

### 3.1 Four groupings, four questions

The graph has one natural grouping per question a designer asks of it, and
each is *derived* — none is stored, so none can disagree with the gears:

| Grouping | Its unit | Answers | Serves |
|---|---|---|---|
| **Flow** | a step along the power path: bodies joined by the meshes that carry power between them; an epicyclic cluster is one junction with three terminals; an idle ratio is a branch | how will it work; what does each case do to every body | reading, evaluating |
| **Centres** | a pair of axes and every mesh at that spacing — the unit the shifts close over and, with its absorber, the unit the search runs over | what is geometrically coupled | tuning geometry |
| **Axes** | an axis, the bodies on it, the gears on each | what turns with what | building, wiring |
| **Mesh group** (the workspace) | the gears a run of meshes joins, focused on one mesh | what the teeth are doing | tuning, strength |

The first three are one list with a switch; the fourth is the workspace
beside it. **Flow is the default**, since it is the one that says what the
train *does*; the others are one click away, and a selection survives the
switch.

Centres is the discovery of this plan: in every shipped preset but two —
the idler, and meshed planets — one distance is exactly what the stage was,
and where a preset has more than one, the centres grouping says so instead of
pretending they are one thing. It is also the unit the search already works
in, which is why it is worth a grouping of its own.

### 3.2 The mesh workspace

The right pane, whatever the list is grouped by, shows the selection's own
unit. For a gear or a mesh, that is its **mesh group**, laid out as
alternating columns — gear, mesh, gear, mesh, gear — focused on one mesh, with
the neighbours in the run a click away and the next mesh *along the flow*
named at the edge ("Body 2 also carries Gear 3 → Gear 3 ⇄ Gear 4").

- **Gear columns** hold the gear's inputs, grouped as the gear tab groups
  them. The mutex rows — module, pressure angle, helix, thickness — sit on
  both columns, so which member states the value and which follows is visible
  at a glance ("given" beside one, "follows Gear 1" beside the other).
- **The mesh column** holds what is the mesh's: friction, load sharing, the
  search and its floor, the axial contact ratio.
- **The centres block** under it holds the distance's inputs — angle,
  distance, clearance, tolerances, tip gap — named with what else they set
  ("also sets Gear 3 ⇄ Gear 4 and Gear 5 ⇄ Gear 6"), since editing them here
  moves meshes that are not on screen.
- **Outputs** sit below, and **run down, never across**: a label and a
  value per row, so every figure starts at the same edge and no descriptor is
  read as a number. First what the mesh comes to whatever the load —
  efficiency, contact ratios, operating angle, backlash, interference, the
  ratio across it and what one more tooth on each gear does to it — then one
  column per case, with a band of rows per gear in the order the columns run
  (Gear 1, the mesh, Gear 2), so the weaker gear of the pair is read across a
  row and each case's governing figure is marked.

For a body the pane is the body — hold, join, what is on it, its speed and
torque per case. For an axis, the axis — its bodies, its distances, its
planets and their gap. For a case, the case and the path it reports.

### 3.3 Select, act, and see before you click

The select-and-act model on the canvas's upper rows carries over whole: rows carry
no controls; selecting one opens a strip of what can be done to it, with
refusals shown as disabled verbs and their reasons; there is **one** add menu,
whose entries are complete outcomes generated from the graph.

Every entry and every verb gets a **dry run on hover**: the core applies the
edit to a copy, solves it and reports what it would make, what else it would
change and what the paths would come to — or why it would be refused. An
edit is a pure function and a solve is microseconds, so this is cheap; it is
one entry point, `preview_edit(train, edit)`, returning the refusal or the
diff as keys and values, so the words stay in the catalogue and the
comparison stays in Rust.

### 3.4 Cases, and the paths they report

Across the top: the cases as a row of chips with one selected, and under
them the path each reports — the train's headline figures. The selected case is what the flow
list annotates each body with — its speed and torque — so evaluating a case
is reading down the list.

---

## 4. Phases

Each ends green on everything `CLAUDE.md` marks "yes" and is committable on
its own. The corpus is the arbiter throughout, and every phase says what its
diff should be.

**Phase 0 — characterise.** No model change. Extend the corpus: every preset
inserted into an empty train and after every other preset, printed per
member, per mesh, per distance and per body for every case; the
three-stage train the canvas draws, in full. `gear-cli train` gains the train-wide printing. *Diff:
additive only.*

**Phase 1 — the graph beside the stages.** Folded into 3d: a flattening
with no caller proves less than the corpus proves when the solve runs on
it, and its motion law (the flattened graph's motion is `train.system()`'s)
lands with the solve that uses it.

**Phase 2 — the inputs find their homes.** Module and pressure angle become
`Auto` with their freedoms; load sharing and the search move onto the meshes;
the planet gap onto the axis. Change-log entries and conversion for files
that write the old fields; defaults that reproduce today's behaviour exactly.
*Diff: none in any figure; the wasm record's shapes change.*

**Phase 3 — a stage stops being a thing that has figures, then one
solve.** Re-sequenced once Phase 2 was in and `solve_shape_after` had been
read end to end: a stage's own figures — ratio, efficiency, backlash, power
through the teeth, one more tooth — come from a *second*, lone-stage motion
solved under the stage's convention and threaded through the geometry, and
nothing else in the stage reads it: the ratings use the train's flow,
handed down per case. So the lone motion goes first, and the flattening
after it is close to concatenation.

- **3a — paths carry what stage figures carried.** `PathReport` gains the
  power through the teeth and what one more tooth on each gear does to it;
  a mesh's power through it is reported per case, from the train's flow.
  *Diff: additive.*
- **3b — the lone stage retires from its callers.** Every test helper and
  harness command that asked a stage alone (`StageLoads`, `solve_any`)
  asks a train with one preset inserted and a case at its conventional
  ends, and reads its path. *Diff: none in any figure.* **Done**, and
  `StageLoads`, `solve_any` and `StageBoundary::of` deleted with their
  last callers: `Train::alone` (a stage's bodies numbered by its slots,
  which closes the numbers up on a shape edited in place), `arranged`,
  `under` (the lone-stage vocabulary's adapter) and `solve_alone`, whose
  `Alone` carries the figures under the names a stage had. **It moved one
  figure, rightly**: a path never asked its whole flow whether it breaks
  away, and the flow read a drive that cannot start as nought plus
  rounding, so the compound back-driven broke away by 3 × 10⁻¹⁵ — 48.5 %
  where it holds. Now nought, alone and in every chain; `corrections.md`
  has the row, and `tools/breakaway.py` the derivation that keeps the sign.
- **3c — the lone motion goes.** `solve_shape_after` loses its boundary,
  `ShapeResult` its five figures, and `MeshReport` its lone-motion power.
  The panel's stage header and results read the paths instead. *Diff:
  every file that printed stage figures changes form; `graph.txt`, which
  never printed them, not at all.* **Done**: `unit_motion`, `UnitMotion`,
  `engagements` and the two notes only the lone motion raised went with
  it; the stage cards show no figure, and the paths list each path's
  figures down the page, one more tooth named gear by gear across the
  train. `a_stage_is_a_path` became `a_path_is_what_it_crosses` — a stage
  in a chain is the same stage alone — and tests that read a stage's
  figures in a train ask the path across its ends (`tests::across`).
- **3d — one solve over the graph.** `Train::graph()` flattens the stages,
  axes merged wherever a body is shared, and `solve_train` solves it once;
  per-stage results are sliced back out for the result still shaped by
  stage. *Diff: none* — the null diff that proves the solve. Two
  stage-local assumptions found by reading for it, each its own step:
  1. **The frame a member meshes in** was the first carrier on its axis;
     on a graph a gear on a sun's shaft meshes a fixed-axis pinion in
     ground. Asked of the member's meshes now. **Done.**
  2. **The search's unit is the component, and it was the shape**: one
     closure over every distance, one set of sizing rounds and one outcome
     for the whole shape, so on a graph one stage's closure failing, its
     tips resizing or its interval missing would reach every other's.
     Each component is searched on its own constraints.
  3. `Train::graph()`, its motion law, and a law that one solve of the
     graph is every stage's solve, element by element, on every fixture.
     **Found on the way, and decided**: a planocentric's output was its
     orbiting planet, and a chain after it would put the next gear on the
     eccentric. Its output is now a shaft on the centre line turned by an
     **offset coupling** (pins, an Oldham coupling) — an element a stage may
     have and lose, so a planocentric and a hula are one step and a
     coupling apart (decision 8). **Done.** A file written before it, whose
     chain joins a planocentric's planet, is the converter's to rewrite in
     Phase 4: the join becomes the coupled shaft's.
  4. The production switch and the slicing — notes and indices back to
     their stages — weighed once 1–3 are in: it is an adapter Phase 6
     deletes. **Dropped, for a better seam.** The graph falls apart by
     itself into parts that close, search and rate apart (`Shape::parts`:
     members joined by meshes, meshes by distances), and a train of stages
     falls apart into exactly its stages, each part solving as its stage
     does. So Phase 4 solves the stored graph part by part and deals the
     cards from the parts: a part's result *is* a card's result, and no
     slicing is written to be deleted.
  **Done** (steps 1–3): the frame rule, the offset coupling, `Train::graph`
  and `Shape::parts`, held by three laws — the graph's motion is the
  train's, body for body and exactly; the train falls apart into its
  stages, each part's solve its stage's; and a join off an orbiting body
  is a coupling — on every preset alone, every ordered pair and the
  three-stage train, each law run first against the code it guards and
  seen to fail.

**Phase 4, step 1 — holds are stated.** Done: `held` is every hold, a
preset's conventional one written at insertion; `release` takes a hold out;
`Constraint`, `BodyConstraint` and `by_convention` are gone, and the file's
`constraints` is refused by name. No figure moved.

**Phase 4, step 2 — a path is a case's.** Done: the chain's ends row is
gone, the motion is read along the headline case (`headline`,
`headline_load`), and `chain_ends` is only where a case starts on a train
with none.

**Phase 4 — storage flips.** `Train` holds the graph. Conventions are written
as holds at insertion, and a fresh train's default cases at its first
preset's ends; `Train::ends` and its row go. **The reader does not convert**:
the format's own policy is one reader and a loud refusal (`gear-io`'s
change log), and it is argued there — a second reader for an old shape is
carried and tested for ever. A stage-shaped file is refused by name, and a
one-off `gear-cli convert` rewrites it, with a law holding every converted
fixture's figures to its Phase-3 answer. *Diff: the corpus's train fixtures
change form and no number moves.*

**Phase 4, step 3 — the train is one graph.** Done, in two commits. First
the edits a parallel card needed to stand on a graph: an axis at any gear
and any axis removed (`AddAxis { mate }`, `RemoveAxis { axis }`). Then the
flip: `Train { load_cases, reversed_bending, shape, held }`, the cards read
off it (`Train::parts`, dealt in `StagePorts::part`) and solved part by part
(`solve_parts`; a stage asked alone is one card, `Part::whole`). Decided on
the way, each held by a law run first against the code it guards:
- **An edit is the card's**: asked in its numbering, made on the graph
  through the part's maps (`Shape::edit_part`), reading its stage among
  the card's gears — an axis removed from them alone, a gear alone among
  them already on a body of its own. Every add on every preset, asked of
  it as a train's second card, is the same add alone.
- **A join is one body on one axis**, the axes one line, refused across an
  axis distance, and an offset coupling where an end orbits, which a split
  takes away; **every part keeps its own order of bodies** through it
  (`keep_orders`), since a card's slots are what its conventions read.
- **A card lists what it has something on**: a layshaft's output in neutral
  is the next stage's and not on the layshaft's card until a gear is
  engaged on it again (`state.md`).
- **The converter** is `graph_of` behind `gear-cli convert`; its laws hold
  the converted graph to what a chain builds now, to the stages' motion
  and to their solves, and the elevation drive the old tool wrote converts
  to the file the tool writes now, every figure bit for bit.
- The panel's cards stand on the graph's own objects (`web/src/cards.ts`),
  so binding and relief are unchanged, and the sidebar's count is the
  core's parts.
*Diff: as planned.* The graph corpus and every train record byte for byte;
`trainfile` shorter by the bodies no longer listed twice; a worm feeding a
pair in one stage reads as the two parts it is, every figure as it was; the
wasm record's storage form, every solved number identical and the cards
after every probe edit the old stages field for field.

**Phase 5 — one edit set.** §2.6. `every_add_on_every_preset_solves`,
`every_add_undoes` and `a_refused_edit_changes_nothing` sweep every preset
inserted alone and after another; new laws for `Join`'s axis merge and its
refusals, and for `AddGear` over every mate and target the graph admits.
`preview_edit` lands here, since it is `edit` plus `solve` and nothing else.

**Phase 5, step 1 — the graph's edits.** Done: `Edit` — `AddGear` at a
body, a new body or a new axis (`Place`), `AddRatio` on the body asked,
`AddStep`, `Couple`, `Remove` of a member, mesh, axis, body or coupling
(`Piece`) with what goes with it, `Move`, `Join`, `Hold`, `Release` and
`Insert` at a body — made by `Train::edit`, each on a copy and kept whole
or refused whole. The card's `StageEdit` is read into these with its own
refusals, so the panel's behaviour held: the probe's every card step
answers byte for byte. Decided on the way: **a lock by construction is
made and named by the solve**, as a lock by holds is — the edit's to say is
whether the graph hangs together — save the one the card's move already
refused, a gear on its own planet's carrier; **a gear's body goes with it
where nothing else is on it**, as it did, but a shaft an offset coupling
turns stays; `Join` makes an orbiting end's join a coupling rather than
refusing it (decision 8). Laws: every gear the graph admits over every mate
and place, on every preset alone and as a second card, is refused whole or
leaves nothing hanging, sized to the distance it crosses, and undoes;
every removal leaves nothing hanging; a join is one body on one axis or
says why; an insert runs on its shaft; a ratio goes on the body asked —
each rule run against broken code first. Found by them: a gear taken off a
coupled shaft took the shaft, a hold outlived an emptied train, and an
edit's refusal had never crossed the boundary as its key (`corrections.md`).

**Phase 5, step 2 — `preview_edit`.** Done: `train::preview` makes nothing
of its own — the edit is made on a copy by the rule `edit_train` makes it
by (one `apply_edit` behind both entry points), both trains are solved,
and the difference is said as notes (`preview.*`): each kind of piece whose
count moves, the cards, the holds and the case entries, before and after;
the headline path kept, lost or found, ratio and efficiency before and
after; the refusal by its key; why the edited train would not solve. A
path is compared where the headline case keeps its entries, which is when
it is the same path whatever the edit numbered again. The interface's
hover is Phase 7's; the wrapper waits in `core.ts` (`previewEdit`).

**Phase 6 — results and the boundary.** Done: `TrainResult` answers per
piece — `members`, `meshes`, `distances` and `axes` by the graph's index,
`parts` for what is a part's own — laid from the parts' own solves, the
indices a result carries renumbered to the graph's; `TrainResult::part` lays
a card's view back out, and a law holds it to the part's own solve on every
preset alone and after a pair and a set. The figures are the train's, by the
graph's freedoms; `relieve_stage` is `relieve`, over the graph; `adopt_member`
takes the graph's member. **Decided on the way**: a shaft two parts share
carries a torque that is neither part's external load — what the one hands
the other — which `TrainCase` does not say and the cards need, so the
torque a part's meshes put on each of its bodies stays the part's own
(`PartReport::cases`), the junction's terminals in the flow grouping to come;
and the notes stay the part's until the workspace shows pieces. The cards
read their view from the core (`TrainOutcome::cards`) rather than slicing
the result a second time in TypeScript, until Phase 7 retires them. *Diff:*
the corpus and every path, case, topology and motion byte for byte; each
card's view equal to the stage result it replaced, and the figures the same
values.

**Phase 7, step 1 — the groupings, from the core.** Done: `Train::groupings`
(centres and axes, which need no solve) and `Train::flows` (each case's
flow: from the case's first load, down the meshes carrying its power, most
first; an epicyclic part one junction whose planets, held ends and idle
ends are said inside it; an idle mesh a branch walked after the path), with
the graph-level member names, cross the boundary beside the result. On the
mockups' own train — a pair, a layshaft, a compound set — the flow reads as
drawn. Laws: every mesh at one centre, every body on one axis, and every
body and every mesh said once in every case's flow, entered at the case's
load.

**Phase 7, step 2 — the list and the workspace.** Done, beside the cards
until the workspace does all they do: the case strip (a chip per case; the
one chosen is the case the flow and the workspace are shown for) and the
path it walks; the list in its three groupings, the flow drawn down as the
mockups draw it; and the workspace for the selection — a mesh as its two
gears either side of the mesh and its axis distance, with what it comes to
running down under them; a centre, a body (held and released there), an
axis (its planets and their gap), a junction. The selection and the
grouping are the tab's (decision 4). The workspace stands on the whole
graph as one card (`wholePart`), so every field and relief hook it shares
with the cards takes the graph's pieces by the graph's numbers; it lays
itself out by its own width, three columns where there is room.

**Phase 7, step 3 — a case in the workspace.** Done: a case chip selects the
case, and the workspace edits it — its duty, each body's role and its
figures, what it comes to body by body, its switch — by the one editor the
accordion used (`caseEditor`); a case is added from the strip and shown.

**Phase 7, step 4 — select, act, and see before you click.** Done:
`Train::offers` reads what can be done at a piece off the graph by the rule
each edit states — a gear meshes across a distance, so it is offered on the
axes a distance joins to its mate's; a gear moves among its axis's bodies; a
step goes on a carried axis — and tries each on a copy, so an offer carries
its refusal's key and none is offered that would change nothing (a gear
alone on its body moved to one of its own). It crosses as `offers`. The
panel lists the adds as **one** menu under the list — at each piece
selected, then at the train's output — and the rest as a strip of verbs
over the workspace, a join's and a move's destinations listed under the
verb; a refused entry stays in its place, `aria-disabled` rather than
disabled so hovering and focus still reach it, and every entry's dry run is
`preview_edit` drawn beside it. After an add the workspace shows the first
mesh it made; after a join, the body kept; after a removal, nothing. Laws:
an offer is its edit, over every piece of every preset alone and after a
pair; and every edit a brute-force sweep over every index finds the train
makes is offered at **every** piece it names — which, run against broken
code, first let through the body's and the axis's view of an edit its gear
offered, and now does not. **Found on the way**: a coupling within one part
has no row in the flow (a flow's coupling row joins two parts), so nothing
could select it — a coupled shaft now says what it turns with, its
workspace links to the coupling, and the body offers the coupling's
removal.

**Phase 7, step 5a — the cards leave the panel.** Done: the stage cards,
the paths-and-bodies block and the case accordions are gone, and what only
they showed has a place in the list and the workspace — the path's power
through the teeth and its play's tolerance band in the path box; each
part's notes, and why the train has no answer, under it, each naming its
part by its meshes and showing it on a click; the reversed-bending switch
beside them; a case that cannot solve said on its chip; a search's floor,
a worm's lead, and what one more tooth on either gear makes the shown
case's path, on the mesh; the torque each part's meshes put on a body
where the train's figure does not say it — a shaft two parts share, a
carrier — on the body; the planets' layout on the axis. `cards.ts` is
gone: the workspace stands on the graph itself and relief is the train's.
No "Stage N" is said anywhere — a body is its number, a gear its role and
number (`members.ts`, which the gear tab's adopt list reads too), and the
sidebar counts gears. **Found on the way**: `PathReport::per_tooth` was
still numbered part by part — the cards' order — among fields numbered by
the graph; it is by the graph's index now, which a chain built in order
never told apart, so the corpus held.

**Phase 7, step 5b — the card vocabulary retires.** Done: `StageEdit`,
`Shape::edit_part` and `Train::edit_stage` are gone, and with them the
card's own refusals — `LastOfItsKind` among them, a rule about a card's
chain that the graph's cascading removal does not need; `move_end`, the
port select's rule, and `edit_train`'s card-era edits (join, split, hold,
release, move end, push and remove stage, stage), the panel asking the
graph's edits and a case's; `TrainOutcome::cards`, and `topology` for the
parts themselves (`parts`), the panel reading no port, family or card
mesh group any more — `StagePorts` and `PortSpec` went with it. The laws
the card edits carried were ported, each card edit asked as the graph's
edit it was read into: every add on every preset solves, every add
undoes, every refusal names its reason (the at-no-radius refusal on
meshed planets now, a planocentric's ring taking its planet with it), the
hula and the planocentric reached by edits; the laws of the translation
itself went with it. Five mutations against the ported laws, each caught.
The probe's edits asked as the graph's record, step for step, the trains
the card edits recorded, byte for byte. `split`, `push_stage` and
`remove_stage` stay as the train's operations a law or a fixture builds
with. The join refusal says what it is — two bodies **geared to each
other** — and a preview no longer counts parts, which nothing on screen
names; `TrainResult::cards` is `by_part`, the harness's.

**Phase 7, step 5c — the flows, driven.** Done, in the browser against
the running app, one session end to end: the default pair's mesh removed,
taking both gears, and from nothing a spur pair, a layshaft and a set laid
in at the output from the one add menu, the cases carried to each new
output (−41.8961 : 1); the layshaft's other ratio engaged in the two moves
it is on the machine — the idle gear onto the output, previewed as the
lock it is while both are engaged, then the engaged one off to a body of
its own (−96.6423 : 1), the shaft nothing named any more given up; the
set's ring released and its carrier held, previewed as the case entries
it takes, the case reacted at the ring (+82.8363 : 1, the set reversing);
one mesh tuned, a tooth on the first pinion (78.2343 : 1, 17/18 of it);
and a case read from its chip. **Found by them**: a case that did not
solve showed every mesh idle and every body at 0 rpm — the flow read no
power as "idle", and the list printed the unsolved case's zeros as figures
— so a case that did not solve has no shares in the core's flow (a law,
run against the old code first) and no figures in the list, which says why
it does not solve above its rows; and the case editor, laid out by the
window's width in the accordion it came from, is laid out by the
workspace's.

**Phase 6 — results and the boundary.** `TrainResult` per member, mesh,
distance, axis and path, with `topology` carrying the groupings;
`relieve_stage` → `relieve`; `adopt_member` by member; bindings; the probe.
*Diff: the wasm record, whole.*

**Phase 7 — the interface.** The two panes, the three list groupings, the
workspace, select-and-act, the add menu with the dry run, the case strip
and its paths. Driven in the browser against the canvas, flow by flow: build a
spur-to-layshaft-to-set train from nothing, engage the other ratio, hold the
carrier, tune one mesh, read a case.

**Phase 8 — the documents.** `reference.md`'s *The stage* becomes *The graph*;
`rationale.md` gains why a stage is a preset's footprint and not a container;
a `corrections.md` row per model change (the doubled axis first); `state.md`;
the map in `CLAUDE.md`, whose `train/` table is rewritten and whose census
is taken again at the branch's head; this plan deleted.

---

## 5. What gets deleted

- **`StageBoundary`** and the per-solve conventions (they become holds a
  preset writes); `Train::boundaries`, `Train::ends`, `ports`' convention
  half.
- **The lone-stage motion** in `solve_shape_after` and everything it feeds:
  the stage figures, `SlotCase`.
- **The two-pass solve**: `CaseLoad`, the per-stage second pass, most of
  `Chosen`'s plumbing.
- **`StageLoads` and `solve_any`** — the lone-stage asked form — retire:
  every caller becomes "a train with one preset inserted and a case on it",
  which is what each meant. The 212 references are mostly tests.
- **`StageEdit` and `edit_stage`**, into one `Edit`; `split` and `move_end`
  into `Move`.
- **The panel's stage cards**, `bodies_of`, `cardOrder`, the per-stage
  mesh-group and distance blocks — into one list and one workspace.
- **`train/planetary.rs`'s `boundary_for`**, the set's words as a boundary,
  with `StageLoads`.

Measured when built, against `tools/line_census.py` at this plan's tree.

---

## 6. Testing

In the order `CLAUDE.md` ranks what has caught things:

1. **Against something that shares no code.** `tools/train_kinematics.py` has
   no stages and never did: extend it to the canvas's three-stage train and to every
   pair of presets inserted one after the other, and hold the flattened
   graph's speeds to it.
2. **Properties that need no answer.** A body turns about one axis after
   every edit; every converted file satisfies the five invariants;
   `preview_edit` followed by the edit is the edit.
3. **Laws over thresholds.** The flattening law (Phase 1), the null diff
   (Phase 3), the conversion law (Phase 4), the three edit laws re-framed
   (Phase 5).
4. **Every axis, in every context.** Every preset alone, after each other
   preset, and with its input on each of the previous preset's bodies.
5. **Each new gate against the broken code first.** The axis-merge law in
   particular: run it against a `Join` that merges bodies without merging
   axes, and watch it fail.

The browser flows in Phase 7 are scripted as the last rounds of this branch
were, and each board on the canvas is a flow the script walks.

---

## 7. Documentation, strings and checks

- Five catalogues: the stage headings, the per-stage notes and the edits'
  names go; the workspace, the groupings, the paths and the dry run's
  sentences arrive. `tools/check_strings.py` holds both directions.
- `tools/check_bindings.sh --write` at Phases 2, 4 and 6.
- `tools/check_wasm.sh --write` at every phase that moves the boundary, and
  its record read, not just written.
- `tools/check_figures.py`: the documents' figures that are stage figures
  become path figures and are re-tagged.
- The corpus at every phase, with the diff each phase names.

---

## 8. Risks, and what would change the plan

- **A stage-local assumption in the geometry.** Phase 3 exists to find it.
  The likeliest place is the helix — "read once and propagated" per shape —
  and the tip-sized automatic distance, which reads every internal mesh on a
  distance. If one is found, it becomes an explicit rule over the graph, not
  a reason to keep stages.
- **`Join` refusing what people have.** Every coupling today becomes an axis
  merge. A shaft is straight, so a coupling between two stages *is* coaxial,
  and the only honest refusals are a carried axis joined to a ground one or
  two distances that disagree — both describe no machine. The conversion law
  would surface any file that falls foul.
- **Names.** `member_names` numbers rings and planets per stage; train-wide it
  would run on ("Ring 3"). Scope ordinals to an epicyclic cluster — a carrier
  and what meshes its planets — which is derivable and is what a designer
  means.
- **The search switch per mesh** (§2.2) may read as more granular than it is.
  If the "any mesh asks" rule confuses in use, store it per component after
  all and accept that an edit which merges components merges their switches.
- **Size.** The flow's `2^M` is over every mesh already; a train of a dozen
  meshes is unchanged by this plan. A train of thirty would not be, and was
  not before.
- **The panel on a narrow screen.** Two panes stack; the workspace's columns
  scroll within it. The canvas draws the wide case.

---

## 9. Decisions taken

1. **The search switch is per mesh**, a component searched where any of its
   meshes asks, and the panel naming the others it covers.
2. **No stage labels.** A stage's name lives in the preset menu and nowhere
   after insertion.
3. **A fresh train starts with a default case** at its first preset's
   conventional ends, and that case is its headline path. No `readings`.
4. **The selection is view state kept on the tab**, like `open`: leaving a
   tab and coming back finds the same thing selected.
5. **The flow is drawn down**, as a list beside the workspace.
6. **`StageLoads` and `solve_any` retire.**
7. **Outputs run down, never across** (§3.2): labels and figures mixed at
   uneven widths do not read in a line.
8. **An orbiting output is an offset coupling**, a general element rather
   than an Oldham coupling by name — most cycloidal drives use pins — and
   never mandatory: presets transition into one another by edits, and a
   preset the edits cannot reach or leave marks a hole in the method.
