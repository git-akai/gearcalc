# The train graph — design (principles 9–11; redesigns F, C, N)

Status: proposal, 2026-09-26, `audit-ablation` @ 9b7c919. Read-only survey; nothing built.
Line numbers are at that commit. "Neutral" means `gear-cli identity` shows no bit moved.

## 1. What is there today

### 1.1 The stored representation (`train/shape.rs`)

`Shape` (shape.rs:299) is six parallel vectors addressed by dense `usize` indices:

| Vector | Element | Fields that matter to structure |
|---|---|---|
| `axes` | `Axis` (:65) | `carried_by: Body` (a train body, ground 0), `count` (N planets), `min_planet_clearance` |
| `bodies` | `BodyOn` (:136) | `body` (train number), `axis`. Position in the list + 1 = the part's *slot* |
| `members` | `Member` (:154) | `body`, `gear: MemberGear` (mod.rs:1105), `module`, `pressure_angle`, `thickness_mod`, `ring: Option<Cutter>`, `pitch_diameter` |
| `meshes` | `MeshInput` (:195) | `a`, `b` (member indices; `b` is the ring on an internal mesh), friction, `overlap`, `min_contact_ratio`, `load_sharing`, `search` |
| `distances` | `Distance` (:258) | `axes: [usize; 2]` (unordered), `angle`, `worm`, `distance`, `clearance`, `tip_clearance`, tolerances, `axial_clearance` |
| `couplings` | `[usize; 2]` (:320) | two train bodies turning as one |

Incidence is never stored: `distance_of(mesh)` (:560) scans `distances` for the member's axes on every
call, `meshes_on(d)` (:572) scans every mesh, and a member's meshes are found by `m.a == i || m.b == i`
scans (about 20 sites, e.g. :1225, :2022, :2041, :2098, :3137, :3735; wiring.rs:240; edits.rs:261).

### 1.2 The derived layers

- **Parts** (`graph.rs:231` `Shape::parts`): components of members joined by a mesh and by sharing a
  distance, each returned as a *copied* `Shape` in local numbering plus index maps (`Part`, graph.rs:208).
  Axes are closed over `carried_by` by an ad hoc worklist (graph.rs:285-296); bodies attached by a
  five-clause predicate (graph.rs:317-333). `conditions.rs:209` re-exports it for the train.
- **Wiring** (`wiring.rs`): `Mount { spins_with, axis_fixed_in, replicated }` per member (:109),
  `MeshSpec { a, b, kind, paths }` per mesh (:135). `axis_fixed_in` is `Shape::frame_of_member`
  (shape.rs:431-452): a member's carrier, else the carrier of the first mate whose carrier turns about
  this member's axis, else ground. `Wiring::frame(k)` (:201) demands the two members' frames agree,
  else `NoCommonFrame`. `add_to` (:258) writes one `MeshRow` per mesh and one coupling row.
- **Kinematics** (`kinematics.rs`): `System` (:302) of exact-rational rows
  `z_a(ω_a − ω_f) + z_b(ω_b − ω_f) = 0` (`mesh`, :343) and `ω_a = ω_b` (`couple`, :373); motion,
  mobility (null-space), torques (rowspace, test-only :509) and play from one Gaussian elimination
  (`Reduced`, :639).
- **Flow** (`flow.rs:202`): rebuilds the torque rows in f64 (`per_unit`, :246) and enumerates all
  `2^M` direction assignments (:223), least-squares per assignment (:356).
- **Conditions** (`conditions.rs`): `held`, `bodies()` (:826), `open_ports` (:554), `check_parts`
  (:439, one motion solve per part to name the hold that locks it), and graph surgery: `join` (:863),
  `merge` (:925), `keep_orders` (:963), `renumber` (:271), `prune` (:302), `drop_orphans` (:339),
  `drop_bare` (:1118); `Shape::renumber_bodies` (shape.rs:404), `append`/`merge_axes` (graph.rs:389/416).
- **Groupings** (`groupings.rs:155` `Train::flow`): a DFS over bodies with a `seen` list; a part is
  a "junction" if any of its axes is carried (:160-166); idle is `power < 1e-9` absolute (:242).

### 1.3 The ad hoc algorithms (the inventory principle 9 retires)

| # | Where | What it is | Standard name |
|---|---|---|---|
| A1 | graph.rs:64-94 | union–find over stage axes (`graph_of`, converter only) | disjoint sets |
| A2 | graph.rs:234-260 | union–find, members by mesh and shared distance (`parts`) | disjoint sets |
| A3 | shape.rs:1087-1113 | union–find without compression + O(n·g) regroup (`mesh_groups`) | disjoint sets |
| A4 | shape.rs:1993-2046 | union–find over meshes (`asking_members`) | disjoint sets |
| A5 | shape.rs:2051-2120 | union–find over meshes + search coordinates (`search_components`) | disjoint sets |
| A6 | shape.rs:880-900 | fixed-point sweep propagating `β_b = Σ − σβ_a`; list order decides conflicts | spanning-forest propagation + chord check |
| A7 | shape.rs:1047-1077 | same sweep for `k` (`k_a + k_b = 2`, ring equal); odd cycles unseen (T10.4) | same; signed-graph balance |
| A8 | shape.rs:1941-1963 | `share()`: first given module/α in a group wins | same (equality edges) |
| A9 | shape.rs:1222-1336 | `plan_held`: fixed-point "one free member reaches" loop, then an `in_meshes` tie-break | bipartite matching / DM |
| A10 | shape.rs:591-623 | `absorbers`: leverage ranking with list-order ties | matching on exact integer structure |
| A11 | shape.rs:431-452, wiring.rs:201-211 | frame per *member*, required equal across a mesh (H2) | transfer vertex, per mesh |
| A12 | graph.rs:285-296 | worklist closure of axes over `carried_by`; no cycle check anywhere in production | rooted tree (parent pointers) |
| A13 | flow.rs:223-350 | exhaustive `2^M` directions | block-triangular decomposition (F) |
| A14 | mod.rs:2326-2460 | relief: bounded walk `for _ in 0..bound` over ordered groups (`settle`) | maximum matching (T10.9) |
| A15 | groupings.rs:155-343 | DFS with `seen` bodies (drops meshes: H1); junction = "part has a carried axis" | SCC condensation + topological order |
| A16 | conditions.rs:439-475 | one motion solve per part to find a locking hold | mobility from the structure, exact rank per block |
| A17 | edits.rs:834 | `well_formed` — graph invariants, test-only | validation (T14.6) |

Five union–finds with three different `find`s; three fixed-point sweeps whose answer depends on
mesh order (T10.4's fixtures prove it for A7; T10.5's for A9); one per-member frame that cannot express
a mesh-local fact (H2, ~5,900 hits in a depth-2 sweep).

## 2. The accepted theory, and what each concept buys

The kinematic graph of a gear train (Buchsbaum & Freudenstein 1970, *J. Mechanisms* 5; Freudenstein
1971, *ASME J. Eng. Ind.* 93; Freudenstein & Yang 1972, *Mech. Mach. Theory* 7; Tsai, *Mechanism
Design: Enumeration of Kinematic Structures According to Function*, CRC 2001, ch. on epicyclic
trains; Hsieh & Tsai 1996, *ASME J. Mech. Des.* 118, fundamental geared entities). Citations to be
checked against the sources before any is quoted in `docs/`.

| Theory | In this code | What it gives |
|---|---|---|
| **Vertex = link** (a rigid body, ground included; a compound gear is one vertex) | a train body; ground 0 | bodies are vertices, gears are *attachments* to vertices, not vertices |
| **Turning-pair edge** `(link, carrier of its axis)`, labelled by the **axis level** | `BodyOn{body, axis}` + `Axis.carried_by` — one parent per body | the turning pairs form a **spanning tree rooted at ground** (B&F's rule). Acyclic `carried_by` *is* that rule; nothing checks it today (T13.1, T14.6) |
| **Geared edge** between two links | a mesh, lifted from members to their bodies | one geared edge per mesh; parallel geared edges between one link pair are legal (a layshaft's ratios) |
| **Fundamental circuit**: a geared edge plus the tree path between its ends. `L = E − V + 1 = G` | — | exactly **one circuit per mesh**; with couplings, one per coupling too |
| **Transfer vertex** `t`: the vertex on the circuit where the level changes from `a`'s axis to `b`'s | `frame_of_member`, per member | the mesh's frame, **per mesh**. The circuit is admissible iff its path carries exactly two levels; three means the axes move in every frame (no constant centre distance); one means coaxial |
| **Fundamental circuit equation** `z_a(ω_a − ω_t) + z_b(ω_b − ω_t) = 0` | `MeshRow` (kinematics.rs:292) | the mesh rows *are* the circuit equations; kinematics.rs is already this theory's linear algebra |
| **Mobility** `F = (V − 1) − G − C` (Grübler for gear trains) | `System::mobility` (exact nullity) | a structural count to hold the exact rank to: `rank < G + C` names a **redundant circuit** (twin-layshaft indeterminacy, added2#8) |
| **Fundamental geared entity / epicyclic unit**: the circuits sharing one non-ground transfer vertex | "junction" = part with a carried axis (groupings.rs:160) | the junction the flow view needs (H1), and `Shape::family` (arrangements.rs:835) read off transfer vertices rather than carried axes |
| **Statics dual**: torques in the rowspace of `A` | `flow.rs`, `System::torques` | the flow's incidence is the same graph; its block structure (F) is the graph's |

Two consequences worth stating on their own:

- **H2 is a representation defect, not an edit bug.** A sun meshing the carrier's planets and a
  ground-fixed pinion has two circuits with transfer vertices *carrier* and *ground*. The per-member
  field cannot hold both; the per-mesh vertex can. Prototype (a short Python script in the session scratchpad, not in the repo, from the shape's
  own `body→axis→carried_by` data): planetary → both frames = carrier; H2's sun+pinion → carrier,
  carrier, ground (mobility 2 = exact nullity); Simpson sun on two carriers → each mesh its own
  carrier (T14.19's "inexpressible" case solves); planet↔planet across two carriers → refused, three
  levels; coaxial pair → refused. So T14.19's workaround key is unnecessary and wiring.rs:33-39 goes.
- **An offset coupling is a geared edge with `z_b = −z_a`**: `z(ω_a − ω_t) − z(ω_b − ω_t) = z(ω_a − ω_b)`,
  the frame cancelling. Kinematically a coupling needs no second row constructor; it stays its own
  edge *kind* only because it has no geometry, loss or play (rule 4 satisfied at the row).

What the theory does **not** give: the closure of shifts against distances (C) and the choice of
automatic inputs (relief). Those are bipartite structures over the *members and centres*, not the
links, and are handled by §4's matching and DM, not by the kinematic graph.

## 3. Target data model (principle 10)

Three kinds of thing, and the rule: **a gear's cut, options and checks read the gear alone; whatever
needs two gears is the mesh's; whatever needs two axes is the centre's; frames are derived.**

```rust
pub struct Train { links: Vec<Link>, axes: Vec<Axis>, gears: Vec<Gear>, meshes: Vec<Mesh>,
                   centres: Vec<Centre>, couplings: Vec<[LinkId; 2]>, held, load_cases }
pub struct Axis   { carried_by: LinkId, count: u32, min_planet_clearance: f64 }   // tree edge label
pub struct Link   { axis: AxisId }                                                 // today's BodyOn
pub struct Gear   { link: LinkId, teeth, module, pressure_angle, helix, pitch_diameter,
                    profile_shift, allowance /* K */, cutter /* G: σ, κ */, addendum, dedendum,
                    root_radius, tip_rule, undercut_rule, working_depth, face_width,
                    face_sources, rim_thickness, material, material_overrides }
pub struct Mesh   { gears: [GearId; 2], friction, static_friction, overlap, min_contact_ratio,
                    load_sharing, search }
pub struct Centre { axes: [AxisId; 2], angle, worm, running, clearance, tip_clearance,
                    tolerance_plus, tolerance_minus, axial_clearance }
```

Derived, never stored (one `Incidence` built once per solve, replacing the scans of §1.1):
gear→meshes, mesh→centre, centre→meshes, link→gears, the rooted turning-pair tree, each mesh's
kind (σ from the two cutters), **frame (transfer vertex)**, side counts (paths), mesh groups, parts.

### 3.1 Fields that move

| Today | Target | Why |
|---|---|---|
| `Member` + `MemberGear` (two structs) | one `Gear` | a gear's inputs are one entity (T14.15's "ring is a cutter option" with it) |
| `Member.thickness_mod` (k, coupled `k_a + k_b = 2`) | `Gear.allowance` (redesign K) | backlash is the mesh's result of two allowances and a centre, not a gear reading its mate |
| `Member.ring: Option<Cutter>` | `Gear.cutter` (redesign G) | the mesh kind is read off two cutters; no `match` on ring-ness |
| `Mount.axis_fixed_in`, `frame_of_member` | deleted; `Incidence::frame(mesh)` | per mesh (H2) |
| `MeshSpec.paths`, `Wiring::paths_seen` (max over meshes) | per (mesh, side) count | a gear's cycles are the sum of its flanks' (T10.14, shape-b#5) |
| tip room, far gap (`tip_room`, :1507; `sized`, :1568) | mesh clearance record (redesign R) | reads both tips |
| worm proportions (`recommended`, :3147-3170) | the mesh proposes; the gear's automatic width takes it as a resolved input | the gear never looks at the centre |
| `Part { shape: Shape, … }` copy | index view over `Incidence` (late, optional; §5 M10) | one graph, not N copies |
| `Distance` | `Centre` (same fields; `axial_clearance` on the lower-indexed axis, T14.6) | name only |

Wire names stay (`serde(rename)`) until T14.4's versioned format lands; the structural move comes first.

### 3.2 Gear↔mate dependencies to rewrite away (found by grep)

| Site | Dependency | Rewrite |
|---|---|---|
| shape.rs:431-452 `frame_of_member` | a gear's frame from its mates' carriers | transfer vertex per mesh |
| shape.rs:1047-1077 `thickness_mods`; :1024 `base_params` reads it (an O(M) propagation *per gear built*) | `k` from the mate | K; until then, resolved once by §4.2 |
| shape.rs:802-900 `helix_angles` (sizing a mesh, then sweeping) | β from the mate | mesh *constraint* `β_a + σβ_b = Σ`, resolved once by §4.2; the gear receives its own β |
| shape.rs:1941 `share` | module, α from the group's first given | mesh equality constraints, same resolver |
| shape.rs:3730-3743 `member_torque` | refers torque through the mate's tooth count | the mesh reports the torque on each side; the gear takes the largest of its sides |
| shape.rs:372 `is_worm_thread`, :734 `member_names` | a gear's role from its centre | stays a naming layer, read off the mesh |
| edits.rs:503-523 `push_follower`, :385-394 `add_on_new_axis` | a new gear *copies* the mate's `MemberGear`, given helix included (T13.7), and the shape's first ring's cutter | new gear = defaults + teeth + cutter; module, α and β follow by the mesh constraints |
| wiring.rs:234 `paths_seen` | a gear's cycles from its meshes' `paths` max | per side, summed (neutral step keeps the max) |

What stays legitimately cross-gear: the equality of normal module and pressure angle across a mesh
(a physical requirement, now a mesh constraint the resolver checks), the helix-hand relation, and
the undercut/absorber closure (C) — all mesh or centre rows, never a gear method.

## 4. The algorithms, each once, in `train/structure.rs`

Rules: integers and sparsity only — **no tolerance in this module** (numeric zero tests are N's);
every function carries its reference and is tested against a brute-force oracle that shares no code
(exhaustive for ≤ 8 vertices: all matchings, transitive closure, cycle-space dimension `E − V + c`).

| Algorithm | API | Replaces | Reference |
|---|---|---|---|
| Disjoint sets (union by rank, full path compression), `components()` in first-member order | `DisjointSets` | A1–A5 (T14.7) | Tarjan 1975, *J. ACM* 22 |
| Rooted tree: parent/level/depth, `path(a, b)`, lowest common ancestor, cycle detection | `CarrierTree` | A11, A12; the `carried_by` acyclicity check (T13.1, T14.6) | B&F 1970 (tree rule); naive LCA by depth is enough at these sizes |
| BFS spanning forest + fundamental cycles (one per chord) | `fundamental_cycles` | the circuits of §2; the chord checks below | Paton 1969, *CACM* 12 |
| Affine propagation on the forest, `x_b = c_e − σ_e x_a`, each chord checked: consistent / forces a value / conflicts (names the chord) | `propagate` | A6, A7, A8, and the held-distance branch of A9 (T10.5's BFS); the odd external cycle of T10.4 is a chord with `Π(−σ) = −1` | Harary 1953, *Michigan Math. J.* 2 (balance of signed graphs) |
| Tarjan strongly connected components, reverse topological order | `scc` | block order for F and C; the flow view's order (A15) | Tarjan 1972, *SIAM J. Comput.* 1 |
| Hopcroft–Karp maximum bipartite matching | `max_matching` | A9's roles, A10's absorber choice, A14's relief (T10.9 names Kuhn; H–K is the same answer) | Hopcroft & Karp 1973, *SIAM J. Comput.* 2 |
| Dulmage–Mendelsohn: over-, well-, under-determined parts; the square part in block-triangular order (matching + SCC) | `dulmage_mendelsohn` | A13 (F), A9/A10/A16 and the search's free directions (C) | Dulmage & Mendelsohn 1958, *Canad. J. Math.* 10; Pothen & Fan 1990, *ACM TOMS* 16 |

How each is used:

- **F (flow)**: rows = bodies with a torque balance, columns = mesh driver torques + unknown loads.
  DM; each 1×1 block's direction is forced by the sign of its exact coefficient, enumeration only
  inside irreducible blocks (U2: 2,997/2,998 agreement; the miss a tie). U2's caveat stands: a carrier
  row on an internal mesh, `−(z_a + η z_b)`, can change sign at low η — that entry is not structurally
  signed and is enumerated. The over-determined part is a load contradiction (named); the
  under-determined part is the twin-layshaft indeterminacy (added2#8), named as a circuit.
- **C (closure)**: rows = one per mesh (`x_a + σx_b = g_k(a_d)`) plus size readings; columns = shifts
  and running distances. The shift coefficients are exact integers (0, ±1, ±2 — the "leverage" of
  shape.rs:597 and :1426), so the sparsity is exact, not thresholded: a planet between two rings has
  a structural zero, which is why A10 had to rank leverage by hand. The well-determined part in BLT
  order gives the plan's roles independent of list order (T10.5), 1×1 blocks are closed-form
  (T10.6), the under-determined part is the search's free directions and components (T12.2/12.3,
  A4/A5), the over-determined part is the relief's conflict (T10.9).
- **Relief**: the same matching; relief turns automatic the least-precious given input that
  lengthens it. The precedence stays a declared order (a user-visible rule, principle 3), not a walk.
- **Mobility check (A16)**: `(V−1) − G − C` per part against the exact nullity; `check_parts` keeps
  its exact solve only to name the hold, now in the order the tree gives.

### 4.1 petgraph or our own

Recommendation: **our own, ~300 lines plus oracle tests; no runtime dependency.**

- petgraph (0.6–0.8) gives `UnionFind`, `tarjan_scc`/`condensation`, and a general-graph
  `maximum_matching` (Gabow). It has no Hopcroft–Karp, no Dulmage–Mendelsohn, no BLT and no
  fundamental cycle basis, so the three algorithms F and C depend on would be ours regardless.
- It would add a second representation: `Shape`'s vectors copied into a `Graph<N, E>` per call, with
  index maps back. That adapter is about the size of the algorithms it borrows.
- gear-core's `Cargo.toml` states a no-dependency policy for auditability and compile time;
  `nix build` vendors the lockfile, so any new crate is a lock and hash change. Its wasm size is
  unmeasured and not the deciding point.
- A dev-dependency on petgraph as an *independent oracle* is worth it only if brute force proves too
  slow at the sizes we test (it will not: ≤ 12 vertices exhaustively).

### 4.2 What stays numeric

Kinematics stays exact-rational Gaussian elimination; flow and closure stay f64 inside their blocks.
Structure decides *what* is solved together and in what order; it never decides a value. Every
threshold now on a structural question (groupings.rs:242 `1e-9`, shape.rs:1404 `1e-9`, flow ties)
moves to N.

## 5. Migration, in behaviour-neutral steps

Each step: laws written first and seen failing (or, for a neutral step, seen passing on both sides);
`gear-cli identity` bit-identical unless the row says otherwise; `check_all.sh` green. M0 extends the
identity snapshot with the *structure*: parts, mesh groups, search components, every mesh's frame,
plan roles — over every preset and every train in the seeded edit walk (T13.1).

| Step | Change | Neutral? | Subsumes | Guarding laws |
|---|---|---|---|---|
| **M1** | `structure.rs` with `DisjointSets`; swap A1–A5; `mesh_groups` loses its regroup | yes | T14.7, lens-architecture#11 | identity; T14.7's partition law; oracle tests |
| **M2** | `Incidence` built once (gear→meshes, mesh→centre, …); `distance_of`/`meshes_on` read it | yes | the O(M²) scans; part of T11.12's op counts | identity; `Incidence` vs the scans on every walk train |
| **M3** | `CarrierTree` + validation: `carried_by` acyclic, one axis per line in a frame, dense bodies; `well_formed` becomes `Shape::validate` | yes on valid input; refusals where invalid | T14.6, T13.1's graph half, T01.1/T01.2's carrier-cycle checks, shape-a#7, graph-ops#0 | fixtures written first (carrier cycle, duplicate distance, ring-first mesh) |
| **M4** | `transfer_vertex(mesh)` computed beside `Wiring::frame` — no caller | yes | — | law: equal to `frame(k)` wherever that is `Ok`, on presets and the walk (the step that proves M5 neutral) |
| **M5** | wiring switches to the per-mesh frame; `Mount.axis_fixed_in` and `frame_of_member` deleted; a three-level circuit refused in `Shape::apply` under a named key; ring across a crossed centre refused there too | **Err→Ok only** on H2 trains; identity on every train that solved | H2, shape-a#8, T14.19 (Simpson solves; no workaround key), wiring.rs:33-39 doc | H2's frame law on the walk: every unrefused offer wires; a Simpson fixture against `tools/train_kinematics.py`; identity elsewhere |
| **M6** | `fundamental_cycles` + `propagate` replace A6/A7/A8 (and a chord conflict gets a key) | presets yes; order fixtures move by intent | T10.4 (odd cycle as a chord), T10.3's helix half, T14.9's `helix_angles` split | T10.4's permutation law; T10.3's fixtures; identity on presets |
| **M7** | `max_matching`, `scc`, `dulmage_mendelsohn` land with oracles; no callers | yes | — | brute-force oracles |
| **M8** | **F**: flow by BLT; the `2^M` loop becomes a `#[cfg(test)]` oracle; rows built from the `MeshRow`s the kinematics already has | yes where one branch; ties recorded | T11.1, T11.3, T11.4, T11.12 (flow), T11.15, T15.12 (conditioning), kinematics-flow#7, lens-architecture#7, added2#8, U9 | oracle law (BLT = 2^M) on presets, walk and U2's random set; op-count gate `Σ 2^{b_i}` |
| **M9** | flow view from F's result: meshes oriented by signed power, condensed by `scc`, topological order; junction = the circuits sharing one non-ground transfer vertex | no (display, H1 trains) | H1, T11.10 | every body and mesh said once, on the walk, solved and unsolved cases |
| **M10** | **C** (after T14.5 and T10.3/T10.4): DM of shifts and distances × mesh rows gives roles, closed-form 1×1 blocks, free directions, relief | presets yes; T10.5/T10.6 fixtures move by intent | T10.5, T10.6, T10.9, T12.2, T12.3, part of T12.10, shape-a#1, shape-a#12, added2#17 | T10.5's permutation law; T10.9's four relief laws; T10.6's closed form = Newton to 1e-12 |
| **M11** | gears stand alone: `Member`+`MemberGear` → `Gear`; §3.2's rewrites; K and R land here | no (K moves backlash, recorded) | T13.7, T14.15, T10.10 and R's/K's task lists | a law that `Tooth` construction takes only `&Gear` + resolved own inputs (type-enforced); corpus diff explained |
| **M12** | `Part` as an index view instead of a copied `Shape` | yes | lens-architecture (part copies) | identity; *only if* M2's op counts show the copies matter |

Order constraints: M1–M4 need nothing else and can land in Stage 1 (M5 is Stage 1's H2 cure, which
replaces the plan's interim "refuse the edit"); M8 before M10 (plan §3); M11 after G, R and K exist.

## 6. Where this may be wrong

- **Axis identity.** The transfer vertex compares axis *entries*. Two entries for one physical line
  in one frame would give three levels and a false refusal. M3's "one axis per line" invariant must
  land before M5; `merge_axes` already enforces it on joins, not on input.
- **M4 may not be neutral everywhere.** `frame_of_member` picks the *first* qualifying mate; on a
  train where two carriers turn about one axis and a central gear meshes only one, the two agree, but
  this is argued, not measured. M4's law is the measurement.
- **DM is structural.** Structural rank ≥ exact rank. A block the structure calls square can be
  singular at particular tooth counts (redundant circuits); that case must be caught by the exact
  kinematics and named, not assumed away.
- **Sizes are tiny** (≤ ~30 gears). Hopcroft–Karp over Kuhn buys nothing in speed; the case for named
  algorithms is correctness by reference and one tested home, which the owner has asked for.
- **Dense indices stay.** Stable ids (a slot map) would remove `prune`/`renumber`/`drop_bare`'s
  renumbering, but the panel and the wire address pieces by index; that is a T14.4-sized change and is
  not proposed here.
