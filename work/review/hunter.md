# hunter — fresh bug hunt in the thinly-audited files

Branch audit-ablation @ 03f9823. The probes ran in a worktree, now removed. The probe module is kept at
/home/user/.cache/gearcalc-work/hunter/hunter_fuzz.rs: drop it into `crates/gear-core/src/train/` and add
`#[cfg(test)] mod hunter_fuzz;` to `train/mod.rs`. Tests: `hunter_two_steps` (set HUNTER_DEPTH2=1),
`hunter_random_walks`, `hunter_flow_repro2`, `hunter_sun_pinion`, `hunter_wiring_repro`. The Python
checks are planet2.py and regr.py in the same directory; their interpreter path is in `py`.

Method: (1) read every file in the brief; (2) a random-walk and an exhaustive depth-2 sweep over
the graph's own *offered, unrefused* edits (`Train::offers` → `Train::edit`) from every preset,
checking after each edit the laws the modules state (groupings, parts, flows, energy, path
efficiency/backlash/ratio consistency) and catching panics; (3) independent recomputation in
Python (scipy/numpy via nix) where a number is at stake.

Every item was checked against audit/ledger.json and the workstreams/mutation.md by file, symbol
and keyword.

---

## H1 — A case's flow drops meshes (and whole epicyclic parts) that end at a held or already-said body

- **Where**: `crates/gear-core/src/train/groupings.rs:256-340` (`Train::flow`): the step filter
  `!seen.contains(to)` (l.271), the junction branch's `seen.push(*to)` for a held terminal or one
  with nothing beyond (l.326), and the planets `seen.extend` (l.302).
- **Claim**: the module and its tests state "every body and mesh said once in every flow". That
  holds on the chained presets the law sweeps, and fails on trains one or two offered edits away. A
  body is marked `seen` without being expanded (a held junction terminal, orbiting planets), and a
  step is dropped whenever its far end is `seen`, so a mesh or junction whose far ends are all such
  bodies is never emitted. The panel's flow view (`TrainPanel.svelte:128`) then does not list that
  mesh's gears at all.
- **Evidence** (probe `hunter_flow_repro2`, `hunter_two_steps`):
  - Wolfrom → Insert Wolfrom at body 2 (the held ring) → Remove(Mesh 3): both cases **solved**. Flow
    = `[Body 1, Junction{part 0, meshes [0,1]}, Body 3, Body 5, Body 6]`, and **mesh 2 is never
    said**. Part 1 (planet on body 6, ring on held body 5, carrier = held body 2) has both terminals
    held.
  - MeshedPlanets → Remove(Mesh 2) → Join{3,4}: solved. Flow omits mesh 1, and with it the whole of
    part 1's junction (terminals 2 and 3, and 3 is held).
  - Unsolved variant: MeshedPlanets → Remove(Mesh 2) → Hold(2); Wolfrom → Insert Spur at 2 → Hold(5).
  - The exhaustive depth-2 sweep over offered edits from every preset finds 261 solved-case and 134
    unsolved-case flows that omit at least one mesh.
- **Novelty**: edit-ops#2 (direction), edit-ops#3 and lens-tests-train#9 (order and idle untested)
  and mutation #137 (coupling rows) all take the structural law as holding. None reports it failing.
- **Severity**: low–medium. It is display only, but a whole part can vanish from the flow view.
- **Fix**: walk by *pieces said* rather than *bodies seen*. Filter a step when its mesh or junction
  has been emitted, not when its `to` is seen. After the walk, emit any mesh or junction not yet
  said as an Idle row or a Junction, so that "every mesh once" holds by construction. Add the
  groupings law to a sweep over edited trains (the offers tests already build them).
- **Confidence**: high. Reproduced.

## H2 — Offered edits build a gear meshing in two frames; the train then fails as a "wiring" fault (the verdict on shape-a#8 is wrong)

- **Where**: `train/shape.rs:431-452` (`frame_of_member`: one frame per member),
  `train/wiring.rs:218-228` (`Wiring::frame` requires the two per-member frames to agree),
  `train/edits.rs:371-381` (only `add_on_new_axis` checks the frame; `Place::Body` and
  `Place::NewBody` do not), and the `well_formed` law in `offers.rs`, which never builds the wiring.
- **Claim**: shape-a#8's verdict says "The editor refuses it cleanly and gives no wrong answer:
  add_on_new_axis returns WrongFamily". Only `Place::NewAxis` is guarded. A gear added at an
  *existing* body or axis across a distance is accepted when its mate already meshes in another
  frame. The train then fails as a whole with `error.train_wiring` (`NoCommonFrame`). `WiringError`
  documents that refusal as describing two axes turning against each other, or "a preset's defect
  rather than a design's". wiring.rs:33-39 also still says "No arrangement in scope has it".
- **Evidence** (probe `hunter_sun_pinion`): Spur then Planetary, which solves. The offer
  `AddGear{mate: 2 (Sun), on: Body(1) (the input shaft), ring: false}`, i.e. a pinion on the input
  shaft driving the sun directly, is listed with `refused: None`. Made, it gives `solve_train` =
  `Err(InPart{part:0, cause: Wiring(NoCommonFrame(3))})`. The same holds with mate = the ring. The
  sun meshes the planets in the carrier's frame and the new pinion in ground's. Each mesh has a
  well-defined frame, but the member can carry only one. The exhaustive depth-2 sweep hits
  NoCommonFrame about 5,900 times (mesh index 0-6 across Spur, Idler, Worm, Wolfrom, Compound,
  Layshaft, Planetary).
- **Sibling**: `Worm → AddGear{mate 0, on Body(2), ring:true}` is offered and accepted, putting a
  ring on a crossed distance. `kind_of` (shape.rs:470-479) has no kind for it. After
  `Remove(Mesh 0)` the train fails as `Wiring(NotAMesh(0))`, "a preset's defect".
- **Severity**: medium. No number is wrong, but an offered, accepted, plausible edit (a pinion
  driving a sun) turns the whole train into a misattributed refusal, contrary to the audit's verdict
  and to T14's premise that the editor guards it.
- **Fix**: the structural cure is shape-a#8's original proposal. Make the frame a property of the
  mesh: the intersection of the frames each member's axis stands still in (ground, plus the carrier
  of a coaxial carrier body). Sum a member's cycles per mesh; per-flank cycles are already wanted by
  shape-b#5. At minimum, `Shape::apply` should build `wiring()` and check `frame(k)` for every mesh.
  It would then refuse such an edit with a named key, and the offers law `an_offer_is_its_edit`
  would hold it. Refuse `ring: true` across a distance at an angle in the same way.
- **Confidence**: high.

## H3 — (info) diagram.rs's severed-tooth SVG carries a literal backslash

- **Where**: `crates/gear-cli/src/diagram.rs:61-62`: `r#"...class="label" \⏎   text-anchor=..."#`.
- **Claim**: inside a raw string, `\`-newline is not a line continuation, so the emitted tag contains
  `\`, a newline and spaces. That is ill-formed XML: a strict SVG consumer refuses it, while an
  HTML parser shrugs it off as an attribute named `\`. None of the eight fixed `bending` cases is
  severed, so the corpus never reaches the branch.
- **Fix**: use `concat!` or a single line.
- **Confidence**: high, from language semantics (checked with `cat -A`).

---

## Checked and found sound (no report)

- `kinematics.rs`: `Reduced::absorb`/`solution` give a correct incremental RREF. `Solution::rebased`
  keeps the family: I traced the invariants, direction k stays zero at every other `at`. The
  lock-up telescoping claim holds, and overflow is already kinematics-flow#0/#9. `torques()`'
  possibly duplicated residual directions are test-only.
- `elliptic.rs`: the R_F and R_D series coefficients match Carlson's and NR's `rf`/`rd` exactly, and
  so does the domain handling.
- `plane.rs`: the identities and their tests are correct.
- `planetary.rs::power`: the rolling-sign logic matches Pennestrì. `carrier_driven_efficiency`'s |R|
  is kinematics-flow#3.
- `gear-cli planetary 17 17 3`: every x_planet, running distance and α_w for z_r = 48–54 reproduces
  exactly from an independent Python closure (inv α_w on both meshes, brentq). 55 has none, as the
  CLI says.
- `handoff_inbound/gear.py` still reproduces every field of all 7 non-trivial fixtures in
  `tests/regression.rs` bit for bit, run with scipy. The directory is load-bearing as the fixtures'
  provenance, so it is not dead weight. It shares the transverse-circle round approximation with
  `tooth.rs`, being its origin, so it is no independent oracle for that (tooth-form#10 and
  lens-docs-accuracy-2#2 cover the approximation).
- Flow idle threshold: `power_through` is normalised to input power, and an idle mesh reads exactly
  0.0 at torques 1e-12…1e6. There is no scale bug; lens-magic-numbers#8 covers the literal.
- `strings.rs` resolve/fill: nothing beyond gear-io#13 and added2#93. `check_strings.py` has no
  orphans hidden by comment-only or test-only uses (checked).
- Root docs: geartrain-refactor-plan/handoff are T18's deletion (lens-docs-accuracy-1#7). The
  tools/*.py independence questions are kinematics-flow#4/#5, tools-ci#0 and crossed-worm#9, and
  hula_kinematics' θ-independent "integration" is already in kinematics-flow#4.
- Numbers held up under edits. Over the whole depth-2 sweep and 100×12-step walks, four laws never
  fired on any solved case: no energy created (Σ Tω ≥ 0), path efficiency in [0, 1], backlash
  min ≤ nominal ≤ max with nominal ≥ 0, and the path ratio equal to the solved case's speed
  ratio.
- No new panic: the random walks (480 walks × 8 steps) and the depth-2 sweep hit only
  conditions.rs:1198 (graph-ops#2). "Body in no part" after offered removals (Wolfrom
  Remove(Axis 1), Planocentric Remove(Mesh 0), …) is the graph-ops#2 / mutation #136 family.
