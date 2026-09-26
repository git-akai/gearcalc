# Findings ledger

Every finding the audit raised, with its verification. `audit/ledger.json` holds the full record: claim, evidence, locations, proposal, validation, the verifier's verdict and, for the high-severity ones, the execution check. Severity is the final one (execution check, else verifier, else auditor). Where the verdict is *partly*, the verifier's corrected claim is the one that holds.

| Id | Verdict | Severity | Workstream | Finding |
|---|---|---|---|---|
| `added#30` | confirmed | high | T05 | Ring fillet is built on the wrong side and branch whenever the cutter's corner lies inside its operating pitch circle: no fillet, or a fillet that misses the flank by up to 0.22 mm |
| `added#55` | confirmed | high | T09 | A helical span's contact radius and validity use the transverse projection W/(2 cos beta_b); coaxial anvils touch at W cos beta_b / 2, so the tool picks the wrong k, can accept a span whose anvils rest below the form circle, and can refuse the ideal one |
| `added#56` | confirmed | high | T07 | A crossed mesh whose first member has a negative helix is silently modelled as \|β1\|: the screw and the members built disagree |
| `added2#45` | partly | high | T06 | Unshifted and moderately shifted small pinions report contact ratio, efficiency and ratings on a path that starts inside the base circle, with no note |
| `added2#94` | confirmed | high | T12 | Turning the search on makes the shipped Planocentric preset refuse to solve (OutsideInvoluteDomain) |
| `crossed-worm#0` | confirmed | high | T07 | A contact that slides wholly off a face falls back to the unlimited tip zone: full contact ratio reported, labelled Face |
| `edit-ops#0` | confirmed | high | T13 | chain_on on an emptied train reads a stale body-list index: panic, or case entries on the wrong bodies |
| `graph-ops#2` | confirmed | high | T13 | An offered remove empties a Wolfrom and leaves a case-named body listed; offers(Train) and Insert then panic at conditions.rs:1198 |
| `kinematics-flow#1` | confirmed | high | T11 | Power flow enumerates 2^M direction assignments over the whole train's meshes: seconds at M=14-16, no bound on M, silent wrap at M≥32 |
| `lens-continuity#0` | confirmed | high | T06 | Parallel path of contact is not limited at the usable flank: under interference, ratings jump, understate, then refuse with false messages |
| `lens-feature-gaps#1` | confirmed | high | T08 | No contact-fatigue allowable: Hertz pressure is judged against a tensile/flexural fatigue figure |
| `lens-numerical-robustness#0` | confirmed | high | T08 | Dolan–Broghamer bending factor goes negative/inf/NaN when the load point is low; a train reports negative bending stress and negative min face width, with no note |
| `lens-numerical-robustness#3` | confirmed | high | T03 | Heavily undercut teeth: the flank-to-fillet junction lies above the tip, so the profile and DXF outline extend outside the tip circle (verify shows 0.06-0.73 mm cutter penetration) |
| `lens-performance#0` | confirmed | high | T11 | Power-flow solve is 2^M over the whole train's meshes: seconds at 16-18 meshes, and it runs on every keystroke |
| `mesh-contact#0` | confirmed | high | T10 | Path backlash band puts every distance at the same end of its tolerance, so it understates the band when an internal and an external mesh are on different distances |
| `metrology#0` | confirmed | high | T09 | JGMA band boundaries use [lo, hi) where the standard prints (lo, hi]: m=1.0 and m=0.6 read the wrong row, m=1.6, standard m=10 and d=1.5 get no entry |
| `ring#0` | confirmed | high | T05 | Tip interference reported clear whenever the pinion's tip circle encloses the ring's (one-tooth differences): teeth that foul by up to 0.75 mm pass |
| `ring#1` | confirmed | high | T05 | Tip-window fold uses \|separation\| and so accepts a tooth pair in the wrong passing order; KHK's signed condition and the roll disagree with it |
| `shape-a#0` | partly | high | T10 | Meshed-planet and other triangle closures are never checked: an unbuildable set solves as valid |
| `strength#1` | partly | high | T08 | Contact fatigue is judged against the bending endurance figure; no flank allowable exists, and the default width ignores contact entirely |
| `strength#2` | confirmed | high | T08 | state.md/reference.md 'helical bending is 26–36 % conservative vs ISO 2019' is wrong: the stack script assumes the spur base equals ISO |
| `strength#3` | confirmed | high | T08 | The ISO K factors are said to be 'mostly ≤ 1', so omitting them is conservative; they are all ≥ 1, and the omission's size is recorded nowhere |
| `tooth-form#0` | confirmed | high | T03 | A rack whose tooth comes to a point above the commanded depth is never detected: root too deep, root arc reversed, outline self-intersecting, no note |
| `train-mod-a#0` | confirmed | high | T10 | A path's backlash band is too narrow when it crosses independent distances whose meshes' play moves in opposite directions (MeshedPlanets reports a minimum of 0.046 deg; the true minimum is 0) |
| `train-mod-a#3` | confirmed | high | T10 | Size readings of mesh groups that are not first on their distance are in no group, so a typed helix is silently ignored and a typed pitch diameter makes the solve refuse |
| `train-mod-b#0` | confirmed | high | T11 | A still load's derived torque decides the flow direction from its stale manual value; relief's seeding locks the wrong answer in |
| `web#0` | confirmed | high | T19 | Gear tab box text goes stale: setKind and neighbouring edits change params without updating the box, so the gear drawn and exported is not the one the boxes show |
| `ablate-constants-geometry#0` | confirmed | medium | T03 | Undercut junction above the tip circle: outline and DXF run past ra, no note, penetration confirmed by the cut simulation |
| `ablate-constants-geometry#2` | confirmed | medium | T05 | Ring rim radius r + 2m_t falls inside the ring's own root circle at moderate positive shifts |
| `ablate-constants-geometry#3` | confirmed | medium | T06 | Load-share ramp: conservation below ε=2 depends on RAMP_MAX = 1 − RAMP_MIN, which no law states; above ε=2 the shares sum to up to 1.41 |
| `ablate-constants-geometry#4` | confirmed | medium | T12 | Shipped shift search is not converged on the hula studies, yet reference.md calls its pinned output 'optimised' and 'best' |
| `ablate-constants-rating#0` | confirmed | medium | T08 | Reversed-bending 0.7 is applied to fatigue allowables that are already fully reversed (R = −1); the docs' reasons for it are false for this library |
| `ablate-features#0` | confirmed | medium | T17 | auto.rs carries ~524 lines of stage-era search machinery that production never reaches; Freedoms and Search::effort have no caller at all |
| `ablate-features#1` | partly | medium | T16 | Two auto.rs tests (148 lines) exercise only shifts_for_efficiency, which no shipped path calls, so they cannot catch a search fault |
| `added#12` | confirmed | medium | T16 | phase_resolution_has_converged never varies the phase step; MAX_ROTATION_STEP's stated justification is false and sharp-corner deviation is linear in the step |
| `added#22` | partly | medium | T08 | Against ISO 6336 with all K factors at 1, the default fatigue bending rating of the shipped hardened steel reads up to about 1.6× (20°) and 2× (25°) less utilised than ISO. Recorded nowhere. |
| `added#31` | confirmed | medium | T03 | tip_clearance returns clear for any pointed tip (half-width ≤ 0), and Tooth caps pointed tips at exactly zero width, so rounding decides whether the check runs |
| `added#37` | confirmed | medium | T04 | The CI's independent DXF check cannot see a wrongly bulged arc, although its own comment says it is the part that would |
| `added#4` | confirmed | medium | T05 | The ring tab's 'smallest tooth count' ignores the ring's profile shift, and is computed in the wasm crate |
| `added#45` | confirmed | medium | T10 | A given-distance pair's answer depends on member order: the reaching member is floored at x_min, the free one at max(x_min,0) |
| `added#59` | confirmed | medium | T06 | The tight end of a backlash band reports play where the teeth jam: point contacts clamp the separation at zero, and both models add axial float to a jam |
| `added#6` | confirmed | medium | T03 | A rack whose flanks cross before its tip generates a self-intersecting outline, silently and inside every admitted range |
| `added#60` | confirmed | medium | T10 | The train never reports bottom clearance on a mesh with given shifts, and teeth_clear, documented as 'the whole question', omits it |
| `added#66` | confirmed | medium | T05 | A tip-sized internal distance leaves the tips at exactly zero room at running, so the shipped Planocentric preset's tips foul inside its own default tolerance band |
| `added#8` | partly | medium | T12 | A ring whose shift is given is still judged as a search candidate, so any ring clamp that does not depend on the candidate turns the whole search off |
| `added2#101` | confirmed | medium | T05 | Ring 'smallest tooth count' is computed in gear-wasm, ignores profile shift, and the test checks its own copy of the formula |
| `added2#109` | partly | medium | T06 | The documented range of what load sharing does above ε_n = 2 (−24 % to +15 %) is exceeded on the test's own grid, up to +30 % spur and ×3 helical |
| `added2#11` | partly | medium | T10 | Relief has more than one fixed point for the same input: which inputs stay given is decided by the order freedoms() lists the groups |
| `added2#112` | confirmed | medium | T16 | The ring cut's own gate stops just short of the regime where its fillet is wrong |
| `added2#14` | confirmed | medium | T18 | reference.md says a set's clearance is always given and pinned by relief; the code and the test say otherwise |
| `added2#15` | confirmed | medium | T07 | A crossed pair whose contact has slid off the face reports the unbounded path: ε jumps from 0.18 to 1.64 as Σ falls through ~0.5° |
| `added2#17` | confirmed | medium | T10 | An absorbed sun or ring shift is capped at the admissible range and refused with a false message, while a planet absorber or a reaching shift is not |
| `added2#18` | confirmed | medium | T10 | An automatic thickness coefficient between two given ones depends on the order of the mesh list |
| `added2#19` | confirmed | medium | T05 | A ring whose tip radius lies beyond its root radius is solved, drawn and exported as DXF with no refusal |
| `added2#26` | confirmed | medium | T18 | The out-of-band sharing figure is stated in five more places than the catalogue, with three different numbers |
| `added2#31` | partly | medium | T20 | hulasweep's gap argument has no effect, so reference.md's 'tips foul until the gap reaches a quarter module, verified by hulasweep' cannot be reproduced |
| `added2#33` | confirmed | medium | T18 | A gear's reported torque is not cut by loss, contrary to its doc and docs/reference.md |
| `added2#4` | confirmed | medium | T11 | The still port's flow weight of ±1 is added in rpm beside real rpm, so a case's answer depends on the given speed's magnitude |
| `added2#55` | confirmed | medium | T10 | Thickness relief allows one given k per mesh, not per mesh group, so a planetary set's k can break its invariant with a mesh-order-dependent planet k |
| `added2#72` | partly | medium | T06 | Parallel-mesh efficiency weights each pair at F/ε everywhere on the path, not F/2 in double contact; it overstates tooth loss by about 15-18 % of the loss against ISO/TR 14179-2 / DIN 3990 H_V, and its own verification test shares the assumption |
| `added2#78` | confirmed | medium | T01 | A worm at normal pressure angle 89-90° is solved and rated rather than refused |
| `added2#87` | partly | medium | T17 | A third test of the dead search, the_search_beats_a_scan_of_the_same_interval, sits among the train's laws and checks nothing shipped |
| `added2#98` | confirmed | medium | T12 | Turning search on for the shipped Planocentric preset makes a train that solves refuse (OutsideInvoluteDomain); no test searches every preset |
| `added3#14` | unverified | medium | T01 | A mesh endpoint out of range in a train file panics rather than being refused |
| `added3#19` | unverified | medium | T10 | chosen_at closes the searched shifts at a plan the search never evaluated |
| `added3#2` | unverified | medium | T10 | Thickness-coefficient relief is declared per mesh, but the relation runs along the whole mesh chain: an idler train can keep two given k's that break the mesh rule, with no word |
| `added3#20` | unverified | medium | T12 | chosen_at returns shifts scored under one plan paired with a later, unevaluated plan |
| `auto-search#0` | confirmed | medium | T12 | The search reports 'no solution' when admissible designs exist, because the feasible region is narrower than one grid step |
| `auto-search#1` | partly | medium | T12 | When one of several independent components finds nothing, its members are left at the box midpoint and reported as 'Chose' |
| `auto-search#2` | confirmed | medium | T12 | The opening sweep is exponential in the number of free shifts: 3.6 s for a five-gear line |
| `auto-search#5` | confirmed | medium | T12 | The recommended shifts sit exactly on a hard wall: rounded to the 3-4 decimals a designer would copy, they are inadmissible |
| `auto-search#6` | confirmed | medium | T12 | working_depth moves the search's floor, but the search's buildability test ignores it and reads undercut at the full dedendum |
| `auto-search#7` | confirmed | medium | T12 | A second, unshipped pair search (shifts_for_efficiency/Bounds/Pinned/free maximise) is what three tests gate; Freedoms is unused; the shape re-implements Pinned |
| `auto-search#8` | confirmed | medium | T12 | The search floor max(x_min, 0) is a hidden rule of thumb, justified by a search defect |
| `crossed-worm#1` | confirmed | medium | T07 | face_widths_for mirrors a one-sided interval and ignores faces centred off the contact, so the 'width for continuity' does not deliver eps = 1 |
| `crossed-worm#10` | confirmed | medium | T07 | Parametrise the screw pair by helix angles indexed by member, not by the worm's diameter and lead angle |
| `crossed-worm#11` | confirmed | medium | T07 | No-contact policy differs by contact kind: a line contact refuses; a point contact reports eps 0, a pitch-point efficiency, a pitch-point rating and both flanks fouled |
| `crossed-worm#4` | confirmed | medium | T07 | 'No stress depends on the face width' is false, yet it is the stated licence for shipping worm proportions |
| `crossed-worm#5` | partly | medium | T07 | The worm proportions are misattributed and partly mis-transcribed; two of them are geometry the crate could compute from its own tip radii |
| `crossed-worm#8` | confirmed | medium | T07 | ZA/ZN claims overreach: the fixed-normal property fails for them, and '1-15 %' is a sampled band on a non-conjugate pairing |
| `crossed-worm#9` | confirmed | medium | T16 | crossed_path.py cannot fail, derives the contact ratio from a copy of the crate's construction, and keeps the two-sided zone the crate abandoned in F83 |
| `edit-ops#1` | confirmed | medium | T13 | A gear added at a helical or worm mate copies its given helix: the offered edit always gives Mesh(Incompatible) |
| `edit-ops#2` | confirmed | medium | T11 | The flow drops the sign of power, so a power-split train's flow states a reversed direction |
| `edit-ops#5` | confirmed | medium | T13 | Join, Insert and Hold can leave a case naming a held body; a duty sweep left there counts zero cycles with no note |
| `edit-ops#6` | confirmed | medium | T13 | The edit and offer laws are narrower than their names: no load cases, one edit deep, one candidate per kind |
| `gear-cli#1` | confirmed | medium | T18 | GearCase::torque doc and reference.md say the driven member's torque is cut by the mesh loss; the solve and its own test say it is not |
| `gear-cli#3` | partly | medium | T20 | The internal-mesh interference instrument (roll_pair, Boundary, flatten) lives in the CLI with no test, unlike verify.rs |
| `gear-cli#4` | confirmed | medium | T20 | hulaband is a 12,780-solve grid search in the CLI whose grid-quantised optimum is quoted in reference.md |
| `gear-io#0` | partly | medium | T06 | The load-sharing note quotes '-24% to +15%'; sweeping the band gives +33 % to -74 %, and the rating jumps a third at ε = 2 |
| `gear-io#1` | confirmed | medium | T04 | One root arc per tooth in every rack-cut gear and ring DXF is not the root circle: its bulge ends at the first fillet sample, not the junction |
| `gear-io#2` | partly | medium | T04 | The CI DXF check cannot catch a wrong bulge, contrary to its own comment, and never sees a ring or an eccentric gear |
| `gear-io#3` | confirmed | medium | T04 | An eccentric gear's DXF draws its tip and root reference circles on the axis, although the outline is displaced by e |
| `gear-io#5` | partly | medium | T01 | The material reader accepts E <= 0, ν outside (-1, 0.5) and negative allowables; the solve then blames the teeth |
| `gear-outline#0` | confirmed | medium | T04 | Root arc on the minus side of every tooth (external and ring) is bulged to the wrong end point: not concentric, off by 1.1-3x the tolerance |
| `gear-outline#1` | confirmed | medium | T04 | The outline tolerance test was loosened to fit the root-arc bug, and its arc distance cannot see it |
| `gear-outline#3` | partly | medium | T04 | λ's 'unavoidable error' holds only at a fixed axis distance; at the commanded (zero-backlash) distance λ=0 is exactly conjugate both ways |
| `gear-outline#5` | confirmed | medium | T04 | The smoothstep root displacement folds the eccentric fillet at admissible inputs, and the export then strays 4.6x the tolerance |
| `gear-outline#6` | confirmed | medium | T04 | With λ≠0 the root 'reach' can fall short of a tooth's own fillet, and the outline crosses itself |
| `gear-outline#7` | confirmed | medium | T05 | Rings have their own profile and outline walkers; the 'one assembly, one outline path' claim is false |
| `graph-ops#0` | confirmed | medium | T01 | A carried_by self-loop or cycle makes Shape::parts allocate without bound (abort/OOM) — reachable from a file or a wasm payload |
| `graph-ops#1` | confirmed | medium | T01 | No structural validation of a train read from TOML or JSON: out-of-range indices panic in from_toml and solve_train |
| `graph-ops#3` | confirmed | medium | T13 | 'Add case' can write a case at bodies the train does not have, and even switched off it makes the whole solve fail |
| `graph-ops#4` | confirmed | medium | T13 | Load cases do not survive an edit round trip: removing the stage a case reacts at drops the reaction and grounds the duty |
| `graph-ops#5` | confirmed | medium | T13 | A fatigue duty measured at a held body or at ground silently counts zero cycles |
| `graph-ops#8` | confirmed | medium | T13 | The suite has no train-level invariant law: conditions.rs's case and hold rules are unchecked, and set_duty has no Rust test |
| `kinematics-flow#0` | confirmed | medium | T11 | Exact kinematics overflows i128 on a 9-pair chain whose answer is ~1e14; the case is then misreported as underdetermined |
| `kinematics-flow#2` | partly | medium | T02 | A refused flow or play solve is reported as efficiency 0 and backlash 0: dual-countershaft trains read as self-locking with zero play |
| `kinematics-flow#3` | confirmed | medium | T11 | carrier_driven_efficiency uses \|R\| and is wrong for a 3K set with a negative ratio |
| `kinematics-flow#4` | confirmed | medium | T16 | train_kinematics.py and hula_kinematics.py never read the crate, so no crate defect can make them fail |
| `lens-architecture#0` | confirmed | medium | T14 | Ring tab's 'smallest tooth count' ignores profile shift (computed in gear-wasm, not the core) |
| `lens-architecture#3` | confirmed | medium | T02 | Gear-tab failures reach a translated UI in English |
| `lens-architecture#6` | confirmed | medium | T14 | train/mod.rs and shape.rs are grab-bags: inputs, reports, rating, relief, errors and solve in one file; impls scattered over 6-7 files |
| `lens-architecture#7` | partly | medium | T11 | The flow and exact kinematics run over the whole train's meshes although the train already decomposes into parts |
| `lens-architecture#9` | confirmed | medium | T14 | The TOML document is the internal struct's serde, unversioned; ~20 breaking changes are migrations in prose only |
| `lens-continuity#1` | confirmed | medium | T07 | Crossed pair whose face misses the contact falls back to the full, face-unlimited path (ε, η and σH reported for teeth that never touch) |
| `lens-continuity#3` | confirmed | medium | T12 | When the shift search finds nothing admissible it falls back to an equal split that violates its constraints and beats the constrained answer next to it |
| `lens-continuity#4` | confirmed | medium | T10 | Hula/planocentric tip sizing allows the tip circles' crossing margin to reach exactly zero; the asked clearance governs only the far-side gap |
| `lens-docs-accuracy-1#0` | confirmed | medium | T05 | The ring's smallest tooth count is computed in gear-wasm and ignores profile shift |
| `lens-docs-accuracy-1#1` | confirmed | medium | T18 | state.md says Y_S is applied; no rating applies it |
| `lens-docs-accuracy-1#2` | confirmed | medium | T18 | A crossed pair is said to have no contact ratio; MeshReport gives one |
| `lens-docs-accuracy-1#4` | confirmed | medium | T18 | The hula crank is called a closed-form solve; it is a bracket walk plus Brent |
| `lens-docs-accuracy-1#6` | confirmed | medium | T18 | README says nix flake check is everything CI checks |
| `lens-docs-accuracy-2#0` | confirmed | medium | T09 | Over-pins contact roll omits cos β_b: pin-validity range wrong on helical gears (doc and code agree, both wrong) |
| `lens-docs-clarity#1` | confirmed | medium | T18 | Three contradictory 'what to run before pushing' lists (README, state.md, CLAUDE.md) |
| `lens-docs-clarity#4` | confirmed | medium | T18 | Terminology unsettled after the refactor: 'stage' (978 in code) vs part/preset/arrangement/shape/footprint, and no glossary |
| `lens-docs-clarity#5` | confirmed | medium | T18 | gear-io/src/train.rs module comment is a 250-line changelog of file formats no reader accepts |
| `lens-docs-clarity#9` | confirmed | medium | T18 | CLAUDE.md's train/ file map has lost its structure: 140-343-word cells and no 'does not know' column |
| `lens-errors-policy#0` | confirmed | medium | T01 | No production validation of 'no shape' inputs: the refuse half of rule 5 exists only in the gear tab's TypeScript |
| `lens-errors-policy#1` | confirmed | medium | T05 | The ring cutter's centre distance silently falls back to reference centres, and the root radius jumps up to 1.13 mm inside the admitted range |
| `lens-errors-policy#10` | confirmed | medium | T02 | English reaches the UI: gear-wasm's throw and adopt refusals, and the material/train import reasons |
| `lens-errors-policy#2` | confirmed | medium | T02 | The documented examples of refusal are not refused: a cutter larger than its ring is clamped, and module zero is built |
| `lens-errors-policy#4` | confirmed | medium | T02 | An external member with no root section refuses the whole train, while a ring or a planet's other mesh is only noted |
| `lens-errors-policy#5` | confirmed | medium | T01 | The material library accepts unphysical constants, and a Poisson ratio of 0.7 silently raises contact stress by 34% |
| `lens-errors-policy#6` | confirmed | medium | T01 | A negative friction coefficient is accepted: mesh efficiency 1.004, path efficiency 0, and the case says the load is not reacted |
| `lens-feature-gaps#0` | confirmed | medium | T10 | Path backlash band is not the worst case when external and internal meshes sit on independent distances |
| `lens-feature-gaps#14` | confirmed | medium | T19 | All work is lost on reload or tab close, with no warning, and reload is the only way out of developer mode |
| `lens-feature-gaps#15` | confirmed | medium | T19 | No undo; destructive edits apply on one click; the dry run is hover/focus-only and cannot be seen on touch |
| `lens-feature-gaps#2` | confirmed | medium | T08 | Fully-reversed fatigue data treated as one-directional, then derated again for reversal |
| `lens-feature-gaps#3` | confirmed | medium | T21 | Backlash band ignores tooth-thickness tolerance, the dominant source of minimum backlash |
| `lens-magic-numbers#1` | confirmed | medium | T15 | Preset friction (sliding 0.08, static 0.16) has no source, yet decides the self-locking verdict |
| `lens-numerical-robustness#1` | partly | medium | T08 | NaN bending stress at a pointed tip: Lewis tangency has a trivial root at the vertex, and the sharing sweep and candidate reduce latch onto the first NaN |
| `lens-numerical-robustness#2` | partly | medium | T03 | A rack whose tip width is ≤ 0 still cuts to its full dedendum: root too deep, negative cutter tip width reported, 0-mm pin 'measures', no note, bound admits it |
| `lens-numerical-robustness#4` | partly | medium | T01 | z = 0 panics in Gear::profile (the gear_profile wasm entry), and Mesh::new accepts a 0-tooth member and returns NaN/inf |
| `lens-numerical-robustness#5` | confirmed | medium | T01 | No shared validation of the graph's scalar inputs: planet count 0 rated as one planet, negative friction gives η > 1, friction ≥ 5 gives η = 0 silently, rim 0 gives NaN, module ≤ 0 refused under unrelated keys |
| `lens-numerical-robustness#8` | partly | medium | T11 | Searches on random layshafts take 0.83-0.89 s natively and then refuse; the 200 ms typing budget is asserted on shipped presets only |
| `lens-performance#1` | confirmed | medium | T11 | `1u32 << m` wraps at 32+ meshes: silently wrong efficiency (false self-locking) and false refusals in release |
| `lens-performance#11` | confirmed | medium | T18 | flow.rs says the first consistent assignment is the answer; the code keeps the most efficient one |
| `lens-performance#2` | confirmed | medium | T11 | Exact kinematics overflows at 9 chained spur pairs and is reported as 'case underdetermined' while the train still shows a ratio |
| `lens-performance#3` | partly | medium | T12 | Shift search is a 13^dof grid plus 3^dof−1 directions: a searched Layshaft is 2.2 s native / 3.9 s wasm per keystroke, and its timing gate skips it |
| `lens-performance#4` | confirmed | medium | T11 | The panel solves synchronously on the main thread on every input; the 'microseconds' premise of that decision is false |
| `lens-performance#6` | confirmed | medium | T11 | The wasm ships every string catalogue twice (172 KB, 11% of the payload) because LANGUAGES is a `pub const` |
| `lens-standards#0` | partly | medium | T07 | Worm friction is a constant 0.08/0.16; AGMA 6034 gives 0.04–0.06 at the canary's sliding speeds, so forward efficiency is understated by 8–14 points and no starting efficiency is reported |
| `lens-standards#1` | confirmed | medium | T07 | The 8.2° default worm is reported as 'cannot be back-driven' (0.000 %); practice says a worm at 8–10° back-drives under vibration, and the rationale's 'the answer a handbook gives' is the reverse of the handbooks |
| `lens-standards#2` | partly | medium | T07 | A worm's reported contact stress is a crossed-helical point-contact peak, about 3.7× ISO/TR 14521 / DIN 3996's mean contact stress for the same drive; wear and temperature, which the rationale names as how worms fail, are not computed |
| `lens-standards#3` | confirmed | medium | T21 | Equal planet load sharing (K_gamma = 1) is disclosed by direction only; its size, 1.0–1.61 in AGMA 6123 and about 2 for four rigid planets at the tool's own tolerances, is recorded nowhere, and the argument for declining it does not apply |
| `lens-standards#5` | confirmed | medium | T21 | The reported efficiency is mesh friction alone; ISO/TR 14179's load-independent and bearing losses are left out and not disclosed in state.md |
| `lens-standards#6` | confirmed | medium | T15 | Defaults written in absolute mm (tip land 0.1 mm, clearance 0.02 mm, tolerance ±0.02 mm, planet gap 0.3 mm) do not scale with module, and at the shipped defaults the band's tight end has zero backlash |
| `lens-standards#7` | confirmed | medium | T18 | GearCase.torque is documented as 'cut by the loss' but carries the lossless driver torque read across; a worm wheel shows 199 N·m while its shaft carries 123 N·m |
| `lens-standards#8` | confirmed | medium | T21 | Polymer gears are rated with 23 °C properties and no tooth temperature (VDI 2736), and nothing in the docs says so |
| `lens-tests-geometry#0` | confirmed | medium | T03 | An external gear with k above 2 − 4·h_f·tan α_n/π is drawn with flanks crossing the space centreline, often with no note, and admissible_ranges admits it |
| `lens-tests-geometry#2` | confirmed | medium | T04 | The outline breaks its stated chord tolerance by up to 4.07×, and its test bound of 3.0 records where a sweep stopped |
| `lens-tests-train#0` | confirmed | medium | T10 | Intermittent-duty cycle counts for planets are untested: taking the turns unsigned under-counts a planet 6x and the suite and golden corpus both stay green |
| `lens-tests-train#1` | confirmed | medium | T16 | No mirror-symmetry law: a load held still with a negative torque can take the wrong flow direction and nothing notices |
| `lens-tests-train#2` | confirmed | medium | T16 | a_path_holds_at_rest_where_no_mesh_does cannot fail for the rule it names; removing whole-flow breakaway is missed by every test |
| `lens-tests-train#3` | confirmed | medium | T16 | A path's backward efficiency and circulation are checked only by pins and the corpus, never by a law |
| `lens-tests-train#5` | confirmed | medium | T01 | A train whose body numbers have a gap imports silently and is misreported as 'underdetermined'; the dense-numbering invariant is checked nowhere outside edits |
| `lens-tests-train#6` | confirmed | medium | T16 | Three wall-clock tests; the search one fails from machine load alone, not only under coverage |
| `lens-unification#0` | confirmed | medium | T04 | Eccentric-gear play rewrites the backlash law without the kind's sign: internal pairs report play and interference swapped |
| `lens-unification#1` | partly | medium | T05 | A mesh listed ring-first is silently wired as external: wrong motion ratio and a misleading refusal |
| `lens-unification#2` | partly | medium | T03 | The rack-cut/shaper-cut split: two tooth types, three dispatch layers, a fake Tooth for every ring, and no shaper-cut external gear |
| `lens-unification#3` | confirmed | medium | T05 | RingMesh is a second internal-mesh model; production reads only its tip clearance |
| `lens-unification#5` | confirmed | medium | T02 | English lives in gear-core twice: Display impls restate the catalogue in other words and carry detail the Note drops |
| `lens-unification#6` | confirmed | medium | T11 | The power flow enumerates 2^M direction assignments over the whole train: exponential time, and a u32 shift that wraps at 32 meshes |
| `mesh-contact#1` | confirmed | medium | T06 | Efficiency spreads the load evenly along the path (Buckingham): 12–19 % more spur loss than a static split, not documented as a bias, and blind to the overlap ratio |
| `mesh-contact#2` | confirmed | medium | T06 | Contact ratio, efficiency and load sharing use the whole tip-to-tip path even when a flank is reached past its usable end |
| `metrology#1` | confirmed | medium | T09 | The contact radius for a ball on a helical gear or ring leaves out cos beta_b, so the pin bound and the verdict near the tip are wrong |
| `metrology#2` | partly | medium | T09 | Best k is 'contact nearest the reference circle': a hidden selection rule, different from DIN/KHK practice, that crowds shifted gears' contacts against the form circle |
| `metrology#7` | confirmed | medium | T09 | Eccentric pins: a space is checked against one of its two bounding teeth, and the summary's range silently leaves out refused positions |
| `primitives#0` | confirmed | medium | T01 | No-shape gear inputs panic or produce huge outputs instead of being refused (rule 5) |
| `primitives#11` | confirmed | medium | T02 | Error messages exist in English twice, in gear-core Display and in the catalogue, and have already drifted |
| `primitives#3` | confirmed | medium | T11 | Ratio::checked_add overflows i128 long before the answer does |
| `ring#2` | confirmed | medium | T05 | Ring cut still falls back silently to reference centres when the radial shift leaves the involute domain: root jumps 0.28 mm, no note, search accepts it |
| `ring#3` | partly | medium | T05 | No check for the cutter trimming the ring's tips; the z−1 cutter clamp guarantees it and the verify instrument cannot see it |
| `ring#4` | partly | medium | T05 | A pointed cutter gives no fillet and an involute extended to rf: profile jumps 0.155 m at the threshold and differs from the cut by 0.9 mm |
| `ring#5` | partly | medium | T05 | The fillet can cross mid-space and come back; solve_root_end checks only θ(0), so self-intersecting roots go unreported (8 unclamped cases in the sweep) |
| `ring#6` | partly | medium | T05 | ring_is_cut_as_asked = clamps.is_empty() lets notes that change nothing veto a ring; the default 43-tooth ring is refused by every search |
| `ring#9` | partly | medium | T05 | How far 'a ring is a gear with a negative tooth count' holds: Ring and Tooth are parallel implementations differing by σ in about a dozen places, and rack vs shaper are two fillet constructions in production |
| `shape-a#1` | confirmed | medium | T10 | The plan's role assignment depends on mesh order: a valid given distance is reported unreached with negative clearance |
| `shape-a#12` | confirmed | medium | T10 | absorb runs a bracketed Newton even when the absorber is not in the first mesh, where a closed form exists |
| `shape-a#2` | confirmed | medium | T12 | Searching one mesh changes the shift division of meshes that did not ask to be searched |
| `shape-a#3` | confirmed | medium | T12 | A Settled member is a real degree of freedom the search never moves; search=true reports NotAsked |
| `shape-a#4` | partly | medium | T10 | An automatic clearance's hidden manual value decides the shifts |
| `shape-a#5` | confirmed | medium | T10 | tip_clearance = 0 is a hidden mode switch: going from 0 to 1e-9 moves the distance, turns a refusal into a solve, and contradicts its own doc |
| `shape-a#7` | confirmed | medium | T01 | A malformed document (index out of range) panics in the core instead of being refused |
| `shape-a#9` | confirmed | medium | T10 | Two given distances in one helix group size its helix twice; the solve refuses with an opaque 'Incompatible' and relief does not repair it |
| `shape-b#0` | confirmed | medium | T10 | Meshed-planet (and Ravigneaux) sets whose three distances cannot form a triangle are solved and reported without complaint |
| `shape-b#1` | partly | medium | T10 | The shipped Wolfrom preset cannot be assembled with its 3 planets at any spacing, and the note says 'sun and ring counts' about a set with no sun |
| `shape-b#10` | confirmed | medium | T10 | A worm wheel's reported torque is the ideal read-across, 1/eta above what its teeth carry (1.62x on the worm preset) |
| `shape-b#4` | confirmed | medium | T10 | Reversed bending is decided by 'in more than one mesh', a topological proxy that is wrong for a gear driven, or driving, on one flank in two meshes |
| `shape-b#6` | confirmed | medium | T10 | The reported minimum face width inverts one mesh's stress at another mesh's width, over-stating it up to 3.6x |
| `shape-b#9` | confirmed | medium | T10 | The thickness-coefficient freedom is grouped per mesh rather than per mesh group, so a shipped planetary with the ring's k typed gets an answer that depends on mesh order |
| `strength#0` | partly | medium | T08 | Fatigue allowables are fully-reversed (R=−1) data but are used as the one-directional allowable; the 0.7 reversed fraction then double-counts |
| `strength#10` | partly | medium | T18 | Stale or contradictory doc comments across strength.rs, reference.md and tests/bending.rs from the pre–Dolan–Broghamer era |
| `strength#11` | partly | medium | T08 | The two model enums allow the incoherent mixtures the docs call unrepresentable; the ISO instrument's fields are woven through the production RootSection |
| `strength#5` | confirmed | medium | T16 | No independent finite-z value gate for the production bending model; K_f constants pinned only by the golden corpus |
| `strength#6` | confirmed | medium | T08 | Z_ε is omitted from contact, adding 12–33 % that is not recorded; the helical single-pair points do not exist at ε_β ≥ 1 |
| `strength#8` | confirmed | medium | T08 | hertz::peak_pressure returns a bare pressure, so callers recompute both terms; contact_stress's lengthwise curvature is dead; point contacts size no width |
| `strength#9` | confirmed | medium | T18 | docs/bending-check.html's hand-written prose is stale and outside every gate; F10's closure covers only the generated block |
| `tools-ci#0` | partly | medium | T16 | train_kinematics.py and hula_kinematics.py never read the crate; 5 of 6 'independent verification' scripts cannot fail on a crate defect |
| `tools-ci#1` | confirmed | medium | T16 | check_figures matches a bag of numbers: swapped rows and swapped pinion/wheel figures pass |
| `tooth-form#1` | partly | medium | T03 | check_cut, the 'definitive' rack gate, builds its cutter from the tooth's own derived placement and cannot see a consistent thickness, depth or pointed-tool error |
| `tooth-form#3` | confirmed | medium | T03 | The rack is not the z_c → ∞ limit in code: the rolled-corner construction is hand-written seven times across three modules |
| `train-mod-a#1` | confirmed | medium | T10 | A member's minimum face width inverts its worst stress at its widest mesh's width, not the width that stress was carried at |
| `train-mod-a#2` | partly | medium | T10 | Relief counts each group on its own, so it lets through states that are over-determined as a whole; the realistic path 'pin the sun's shift, type the distance' then misses the distance |
| `train-mod-a#4` | confirmed | medium | T10 | Thickness-coefficient groups are per mesh, but the rule runs along the whole chain of meshes: a sun and a ring can both stay given, and the planet then breaks the ring mesh's rule depending on mesh order |
| `train-mod-a#5` | confirmed | medium | T10 | The pitch-diameter reading loses the helix hand and clamps a diameter below z·m to a straight tooth with no note |
| `train-mod-a#6` | confirmed | medium | T10 | The overlap box shows mesh 0's ratio but the core reads it at the group's narrowest face, so pinning what the box shows moves the helix |
| `train-mod-a#8` | confirmed | medium | T16 | The relief law checks that a given input is read, not that it is honoured, and only relieves with just = None |
| `train-mod-a#9` | partly | medium | T10 | settle's stated order independence is false, and a walk that does not converge returns silently |
| `train-mod-b#1` | confirmed | medium | T10 | Path backlash min/max put every distance at the same end of its tolerance, which is not a worst-case band |
| `train-mod-b#2` | confirmed | medium | T10 | Tooth-cycle ceilings are taken on float products of exact ratios, so whole counts come out one high (or one per actuation high when reversing) |
| `train-mod-b#3` | confirmed | medium | T13 | An intermittent duty's sweep port is never checked: out of range panics, held gives silent zero cycles, still gives a count that depends on rpm |
| `train-mod-b#4` | confirmed | medium | T10 | Reversal rule: 'more than one mesh' flags a pinion driving two gears, and under a reversing duty a planet's or idler's contact count is wrongly halved |
| `train-mod-b#5` | confirmed | medium | T02 | A statically indeterminate train (twin idlers) is refused, but its path is still reported with efficiency 0 (reads as self-locking) and backlash 0 |
| `train-mod-b#6` | confirmed | medium | T13 | Duplicate entries at one body are accepted: two given speeds at the same port are summed (6000 rpm from two 3000s) |
| `train-mod-b#7` | confirmed | medium | T02 | Over-given speeds are reported as 'short by 4294967295 speeds' |
| `wasm-boundary#0` | confirmed | medium | T01 | Graph indices are never validated: any train entry point, and import_train on a hand-edited file, panics, and a carried-by cycle allocates without bound |
| `wasm-boundary#1` | confirmed | medium | T01 | A trap poisons the wasm instance permanently, loses the panic message, and several wrappers swallow it silently |
| `wasm-boundary#2` | confirmed | medium | T05 | The ring tab's 'smallest tooth count' ignores profile shift, and its formula is written twice (served copy in lib.rs, tested copy in a ring.rs test) |
| `wasm-boundary#3` | confirmed | medium | T14 | The over-pins range around an eccentric gear is computed in the boundary, silently skips seats that fail, and depends on whether tooth 0 seats |
| `wasm-boundary#5` | confirmed | medium | T02 | Five error shapes cross the boundary, and English sentences and a Rust Debug dump reach the user |
| `wasm-boundary#6` | confirmed | medium | T14 | The generated-bindings check cannot see request shapes: seven request types and TrainEdit are hand-written in TypeScript, and unknown fields are silently ignored |
| `web#1` | confirmed | medium | T19 | Clearing or half-typing the train's addendum sends NaN, which fails the whole train request at the boundary |
| `web#17` | partly | medium | T19 | TrainPanel.svelte (3,079 lines) should be split, and the panels' four number-input paths should become one component |
| `web#2` | partly | medium | T19 | Material override box: clearing it writes an override of 0 (Number('') = 0), a comma silently restores the library value, and an unsolved box shows a made-up 0 |
| `web#20` | confirmed | medium | T19 | No front-end tests, and nothing enforces Rule 1 in TypeScript |
| `web#21` | confirmed | medium | T01 | Friction can be typed negative, and the core then reports a mesh efficiency above 1 beside a path efficiency of 0 |
| `web#3` | partly | medium | T19 | The internal-gear (ring) tab validates and annotates its inputs against an external gear's ranges |
| `web#4` | partly | medium | T19 | Gear tab boxes bound straight to numbers turn null when cleared: raw English serde errors, and clearing the throw box switches the input mode |
| `web#9` | confirmed | medium | T19 | Display precision is fixed decimals chosen in TS, and turning auto off rounds the solved value to 4 decimals, which loses precision at the default 0.1 N·m scale |
| `ablate-constants-geometry#1` | confirmed | low | T01 | Pressure angle exactly 0: clamp note fires but thickness_shift is 0/0, giving an empty profile; the clamp is written five times |
| `ablate-constants-geometry#10` | confirmed | low | T15 | The 'is x at the floor' question uses 1e-12 in auto.rs and compat::SAME_SHIFT (1e-9) in tooth.rs |
| `ablate-constants-geometry#11` | partly | low | T04 | Outline has two stops for one thing: the depth cap binds first, the relative floor never binds, and the depth canary cannot see a halving |
| `ablate-constants-geometry#12` | confirmed | low | T15 | Tip-clearance 'lean by the solver's own tolerance' is 1e-9 mm, about 1e5× the solver's tolerance; the clear-side bracket end is available |
| `ablate-constants-geometry#13` | partly | low | T20 | Matrix 'gradient sign agreement, ignoring effects below 0.0%' counts signs at root-finder noise |
| `ablate-constants-geometry#5` | partly | low | T04 | The outline does not meet the chord tolerance it states; midpoint-sagitta stopping overshoots up to 2.7× |
| `ablate-constants-geometry#6` | partly | low | T15 | Flank load direction uses a wrong fallback vector at u=0; the ring's bare 1e-9 tip epsilon leaks into ring ratings to avoid it |
| `ablate-constants-geometry#7` | confirmed | low | T15 | The pointed-tooth roll is solved twice by hand (Newton in tooth.rs, Brent in auto.rs) where inv_inverse exists |
| `ablate-constants-geometry#8` | partly | low | T15 | Bracket walks with tuned growth, cap and nudge constants where a closed bracket exists (tooth, ring, shape tip clearance, auto, metrology) |
| `ablate-constants-geometry#9` | partly | low | T15 | Fixed-count bisection (64 halvings) written three times; the comment says sixty |
| `ablate-constants-rating#1` | confirmed | low | T16 | Exact-zero assertions fail on rounding under unrelated perturbations, giving false catches in the ablation |
| `ablate-constants-rating#10` | confirmed | low | T15 | The re-size fixed point compares millimetres with a shift resolution; its 3-round cap is a recorded observation |
| `ablate-constants-rating#12` | confirmed | low | T15 | The tip-sizing lean of 1e-9 mm claims to be 'the solver's own tolerance', which is 1e-15 |
| `ablate-constants-rating#13` | partly | low | T17 | `EllipticalContact::approach` and Carlson R_F exist only for an output nothing reads, and its `?` can drop the elliptical term |
| `ablate-constants-rating#15` | confirmed | low | T18 | SHARING_SAMPLES has two stacked doc comments that disagree (two seeded candidates vs four); the sampled maximum could be a bracketed maximiser |
| `ablate-constants-rating#16` | partly | low | T17 | `planetary::power` and `carrier_driven_efficiency` are public production functions that only tests call; shape.rs cites a `planetary::solve` that no longer exists |
| `ablate-constants-rating#17` | confirmed | low | T20 | The corpus prints digits finer than the screw-path quadrature has converged, so a change to the sample count reads as a regression |
| `ablate-constants-rating#2` | confirmed | low | T17 | The ±5 shift fallback in `absorb` is dead code; three call sites read the same impossible absence three ways |
| `ablate-constants-rating#3` | confirmed | low | T06 | The out-of-band load-sharing note carries a measured '−24 % to +15 %' in all five catalogues, with no generator; its 2.0 threshold is a literal copy of the ramp's band edge |
| `ablate-constants-rating#4` | confirmed | low | T08 | K_f's constants are checked only by a recorded hula figure; no test compares them with Dolan–Broghamer's published points, though the comment quotes that comparison |
| `ablate-constants-rating#5` | confirmed | low | T08 | The ISO instrument constants (Y_S, 30°/60° tangents, notch range) are held by one corpus output alone |
| `ablate-constants-rating#6` | confirmed | low | T15 | The line contact's patch width (line_half_width) reaches no output and no law |
| `ablate-constants-rating#7` | confirmed | low | T07 | The BS 721 wheel-face formula reaches no output at the shipped worm; its test claims to catch transcription errors and does not |
| `ablate-constants-rating#8` | confirmed | low | T15 | The hula preset's 0.3 mm far-side gap is inert at its shipped counts, contrary to its comment; the planocentric test cannot see whether a gap sized anything |
| `ablate-constants-rating#9` | confirmed | low | T15 | The near-self-locking margin 0.8 is an undocumented rule of thumb deciding a user-visible note; the 0.5 low-efficiency boundary is untested |
| `ablate-features#10` | confirmed | low | T18 | gear-io/src/train.rs opens with ~200 lines of format change log for files the parser now refuses anyway |
| `ablate-features#11` | confirmed | low | T14 | serde back-compat shims read shapes no current writer produces, and 'not carried' crosses the boundary as 0 in one type and null in another |
| `ablate-features#13` | confirmed | low | T18 | Other stale or aspirational docs on test-only items |
| `ablate-features#2` | confirmed | low | T18 | docs/reference.md and CLAUDE.md describe auto::Freedoms and auto::maximise as the shared search machinery, but Freedoms has no caller |
| `ablate-features#3` | confirmed | low | T17 | No gate runs rustdoc: 22 intra-doc links do not resolve, 9 of them to items that no longer exist |
| `ablate-features#4` | confirmed | low | T02 | English prose written into Value.note inside gear-core, and never read by anything |
| `ablate-features#5` | confirmed | low | T19 | The material basis badge shows the enum's English initial: datasheet and derived both show 'd', and TypeScript re-derives is_measured |
| `ablate-features#6` | confirmed | low | T17 | Material provenance crosses the boundary but is never shown, and Family's doc claims a use that does not exist |
| `ablate-features#7` | confirmed | low | T17 | The uniform-tooth metrology API is test-only, duplicating the per-gear *_at/_around API the product uses |
| `ablate-features#8` | partly | low | T18 | teeth_clear and as_asked: their docs say several searches depend on them, but the only caller is one gear-cli line |
| `ablate-features#9` | partly | low | T17 | contact.rs keeps test-only closed-form split code, and sliding_at has a stale milestone note |
| `added#0` | confirmed | low | T18 | shaper.rs module doc contradicts its own code on where σ appears, and names a module that does not exist |
| `added#1` | confirmed | low | T17 | 14 broken rustdoc intra-doc links in gear-core, and no check builds the docs |
| `added#10` | partly | low | T16 | the_general_contact_reading_reproduces_the_internal_one compares conjugate_radius with itself; the '[verified ...]' note in mesh.rs relies on it |
| `added#11` | confirmed | low | T03 | Two inversions of the involute for one mirrored guard: Tooth's pointed-tip clamp runs its own Newton in roll while the ring's uses inv_inverse, and the ring comment calls it 'closed form' |
| `added#13` | confirmed | low | T09 | metrology::cutter_tip_width reads the raw dedendum, not the settled tool, and is a third copy of w_tip |
| `added#14` | confirmed | low | T18 | ring.rs says an external gear 'has never been able to' close its space; reference.md's thickness bound claims to exclude racks with no tooth width |
| `added#15` | confirmed | low | T03 | A tip below the form (junction) radius on a tooth that is not undercut is admitted by the published addendum range: the drawn and DXF outline reach r_j above the reported tip, silently |
| `added#16` | confirmed | low | T18 | The documentation says the flank continues below the base circle; by construction the junction is always above it |
| `added#18` | confirmed | low | T03 | The 'buildable' profile-shift range runs deep into severed teeth and publishes no severing threshold |
| `added#19` | confirmed | low | T04 | The export still hangs on large gears: the floor does not bound the work, and below ~2e-9·m the tolerance is silently missed |
| `added#2` | confirmed | low | T06 | mesh.contact_ratio_below_one tells a helical pair with full overlap that it 'loses contact between teeth' |
| `added#20` | confirmed | low | T18 | Comments credit the chord-tolerance floor with a bound that the depth cap actually provides |
| `added#23` | confirmed | low | T18 | state.md and rationale.md still say ISO's Y_S is one of the two factors applied, but no rating applies it |
| `added#24` | confirmed | low | T17 | rustdoc never runs in CI; 15 unresolved intra-doc links, four of them in the outline/gear scope |
| `added#25` | confirmed | low | T04 | No law looks at a drawn eccentric outline at λ≠0, so both a self-crossing export and a disabled root stretch pass unobserved |
| `added#26` | confirmed | low | T20 | gear-cli `sweep` turns angular_shift and index_offset on Tooth::new, which reads neither: its eccentric axes are dead |
| `added#27` | confirmed | low | T08 | Under the production Dolan–Broghamer model the unshared load point (HPSTC) is not the worst single-pair point for undercut pinions; sharing ON raises the rating below the band, contrary to rationale.md and state.md |
| `added#28` | confirmed | low | T06 | A line contact reports patch_length = face width while its pressure is computed over b/cos β_b; the point contact reports b/cos β_b — a 5.6 % seam at β = 20° absent from the limit table |
| `added#32` | confirmed | low | T08 | The default bending model's axial term drops its sign: a tensile along-tooth load is subtracted as compression relief |
| `added#33` | confirmed | low | T06 | Load sharing below eps_n=2 can only raise the figure; docs say it changes nothing or relieves, and the sweep is neither exact nor converged on small teeth |
| `added#38` | confirmed | low | T18 | 'E2' is an undefined label in a public boundary type's doc and in rationale.md |
| `added#39` | confirmed | low | T12 | At zero friction the shift search is degenerate: every candidate scores 1.0 and the answer is a tie-break, reported as 'Chose' |
| `added#40` | confirmed | low | T18 | maximise's doc describes coordinate descent over a span, and Search's doc counts six constants; the code is a full-grid sweep plus an all-directions pattern walk over five fields |
| `added#41` | partly | low | T09 | The JGMA transcription names its quantities in Traditional Chinese, not the standard's Japanese, and does not name its actual source |
| `added#42` | confirmed | low | T12 | Search::budget is documented as 'a ceiling nothing reaches', but at 3 or more search coordinates most walks exhaust it unconverged |
| `added#43` | confirmed | low | T18 | The SLACK comment describes a tolerance 100x smaller than the constant it documents |
| `added#44` | confirmed | low | T12 | On the shipped Planetary preset, turning the optimiser on always gives 'no solution', and the note does not say why |
| `added#46` | confirmed | low | T16 | Two tests enshrine the search floor's refusals as geometry; one passes only because its own fixture's distance is not reached |
| `added#47` | confirmed | low | T16 | The walk's step floor in Search::maximise is gated by nothing, including the test written for it |
| `added#48` | confirmed | low | T02 | An unreachable given distance a module or more below standard ends in Err(NoRootSection) 'too undercut' on teeth that are not undercut |
| `added#49` | confirmed | low | T17 | No check runs rustdoc's link validation; gear-core has 19 unresolved intra-doc links |
| `added#5` | confirmed | low | T03 | Tooth width is not monotone above the base circle: reference.md and auto.rs say it is, and addendum_for_tip_width wrongly refuses some asks |
| `added#50` | confirmed | low | T09 | pin_diameter_range_around solves z identical spaces on an ordinary gear: 9 ms at z=300, 88 ms at z=3000, on every gear-panel input |
| `added#51` | confirmed | low | T18 | jgma.rs cites 'the specification's default of grade 3', a specification that is nowhere in the repository |
| `added#52` | confirmed | low | T18 | limited_by_face's doc comment still says faces are centred on the pitch plane, and that an offset face merely gives 'a smaller zone' |
| `added#53` | confirmed | low | T07 | 'The two widths differ by 2.4x' is a stale, untagged figure: on the shipped worm they differ by 4.9x and 11.4x |
| `added#54` | confirmed | low | T07 | At small shaft angles a crossed pair's stress scales as 1/√b like a parallel pair's, yet the UI says an automatic crossed face 'has nothing to size it' |
| `added#57` | confirmed | low | T17 | contact::sliding_velocity and sliding_at are a second, production-dead implementation of the slip that Contact::slip computes |
| `added#58` | confirmed | low | T18 | The parallel-axes refusal and the low-efficiency note still speak of 'stages', and the English refusal has drifted from its four translations |
| `added#61` | partly | low | T07 | Two comments say the 17/23 crossed pair at 9°/81° cannot be driven forward at µ = 0.06; it drives at 59 % there |
| `added#62` | confirmed | low | T16 | The production search's sweep test admits interfering and bottoming grid points, and one already uses 74 % of the slack |
| `added#63` | confirmed | low | T16 | contact.rs's optimum-shift test claims an interior optimum; on 9/37 its 'best' is at the grid edge and is itself unbuildable |
| `added#64` | confirmed | low | T17 | cargo doc warns 34 times on gear-core, 14 of them unresolved intra-doc links, and no check runs rustdoc |
| `added#65` | confirmed | low | T17 | sliding_velocity / sliding_at / Sliding are public production API that only tests call |
| `added#67` | confirmed | low | T18 | reference.md says 'a shipped preset is never sized, its tips clearing where the shifts leave them', but the shipped Planocentric is sized by its tips |
| `added#9` | confirmed | low | T18 | MeshTrial::tips_are_clear's doc says the tips are asked at the zero-backlash distance, 'the tighter one'; the code asks at the running distance, which is the tighter one |
| `added2#1` | confirmed | low | T07 | A crossed pair's path of contact flickers on and off as the shaft angle falls from 1e-3 deg to 1e-4 deg |
| `added2#10` | confirmed | low | T18 | The relief-law test's comment calls a tip-sized internal distance's clearance a floor; it is a ceiling |
| `added2#100` | confirmed | low | T16 | The suite's wall time is one serial 1,080-case test, rack_simulation::profile_is_bounded_from_both_sides_by_the_cutter (~30 s) |
| `added2#102` | confirmed | low | T18 | reference.md says the tip-width addendum is a bracketed Newton; the code uses brent |
| `added2#103` | confirmed | low | T04 | An unreachable chord tolerance still yields tens of millions of DXF vertices on a large gear; the safety stop bounds work per span, not per outline |
| `added2#105` | confirmed | low | T18 | CLAUDE.md's claim that K_f perturbations leave the whole suite silent is stale |
| `added2#106` | confirmed | low | T15 | The tip-sizing lean of 1e-9 mm is described as 'the solver's own tolerance' but is a million times larger than it |
| `added2#108` | confirmed | low | T16 | The sharing-sweep test says it asserts exact equality below ε_n = 2, but it asserts only a 1e-4 tolerance |
| `added2#110` | partly | low | T17 | contact::sliding_velocity and sliding_at have no production caller; the crossed-axis promise in their docs is stale, and Contact::slip is the second, live model of the same quantity |
| `added2#111` | confirmed | low | T18 | SHARING_SAMPLES carries two stacked doc comments that say the same thing, and one asserts a proof that does not hold |
| `added2#113` | confirmed | low | T16 | The chooser test's grid oracle admits shifts the chooser correctly refuses, so SLACK is partly absorbing a constraint mismatch |
| `added2#115` | partly | low | T09 | Helical span over teeth never checks that the face is wide enough for the anvils to straddle W sin β_b |
| `added2#116` | confirmed | low | T16 | contact.rs test doc attributes the chooser's 9/37 answer to the test, and its 17/43 result depends on the grid bound because root_radius_fits is omitted |
| `added2#117` | partly | low | T10 | An unsized internal distance is also only checked for far-side tip room at running: Planocentric 30/32 reports clear while its far-side gap goes negative inside its own tolerance band |
| `added2#12` | confirmed | low | T10 | backlash_at feeds three precomputed values through Backlash::banded by faking a distance band and decoding its sign |
| `added2#13` | partly | low | T02 | docs/rationale.md says TrainError's Display is kept 'for Debug', but Debug is derived; std::error::Error is what needs Display |
| `added2#16` | partly | low | T07 | A crossed pair with a spur first member is refused (WormTooThin) at exactly β₁ = 0 but solves at β₁ = 1e-6° |
| `added2#2` | partly | low | T10 | Shape::assembly returns None ('rule does not reach') for a replicated axis with a single mesh, where the answer is known |
| `added2#20` | partly | low | T18 | corrections.md and F54 claim admissible_profile_shift already bounds a ring; measurement and state.md say otherwise |
| `added2#21` | confirmed | low | T19 | The adopt list's fallback makes a worm adoptable and loses role names when the train fails at the boundary |
| `added2#22` | confirmed | low | T19 | Viewport pan jumps under two-finger touch: pointer handlers ignore pointerId |
| `added2#23` | confirmed | low | T02 | A gear on a body the train does not list is refused with the wrong reason (overdetermined) rather than as an unknown body |
| `added2#24` | confirmed | low | T15 | A continuous duty's 1000 h seed is a bare literal with no constructor, while the intermittent duty has one |
| `added2#27` | confirmed | low | T19 | zh-Hant catalogue carries mainland vocabulary although its header says Taiwan terminology |
| `added2#28` | partly | low | T19 | The zh-Hant catalogue contains a Simplified character: 属 in error.screw_axes_are_parallel |
| `added2#3` | confirmed | low | T10 | The shipped Wolfrom preset is one whose planets cannot be evenly spaced, so its default warns |
| `added2#30` | confirmed | low | T20 | `gear-cli worm` rates contact and prints wheel torque at a hard-coded mu 0.06 while the same worm through wormstage runs at 0.08 |
| `added2#32` | confirmed | low | T18 | state.md claims load-case changes 'moved neither canary' as a check, but the strength canary never passes through the train |
| `added2#35` | confirmed | low | T01 | A worm arrangement with more starts than its 7 mm diameter allows (starts > 7 at module 1) produces a Shape that cannot be read back from JSON |
| `added2#37` | confirmed | low | T15 | The shipped fresh-case torques and speed are written twice in gear-wasm |
| `added2#38` | partly | low | T13 | Adding a ring on a planet at a Δz=1 carrier radius is accepted but gives a train that cannot solve |
| `added2#39` | partly | low | T12 | The smallest pinion count that solves is not monotone in the mate's count |
| `added2#40` | confirmed | low | T18 | docs/corrections.md links to a README that is not beside it |
| `added2#42` | confirmed | low | T14 | Every gear-wasm request type, TrainEdit among them, is hand-mirrored in TypeScript as an inline literal |
| `added2#43` | confirmed | low | T20 | check_golden.sh --write deletes the whole corpus when the case list is empty |
| `added2#46` | confirmed | low | T18 | The parallel-axis efficiency is documented as 'exact' and its μ² seam as falling 'linearly with μ', but the formula is linear in μ |
| `added2#47` | confirmed | low | T06 | Crossed-axis play floors at zero below nominal while parallel-axis play goes negative: two conventions for one backlash band |
| `added2#49` | confirmed | low | T02 | A member on a body the graph does not list is refused as a wiring fault (NoCommonFrame), not as a missing body |
| `added2#5` | confirmed | low | T02 | A differential given one speed per port reports 'motion undecided: 0 more speed(s) to give' |
| `added2#50` | confirmed | low | T18 | corrections.md row says a hula's wobble body is 'no port now', and the code and reference.md say it is a port; the reversal has no row |
| `added2#52` | confirmed | low | T07 | A point-contact mesh's efficiency is taken at the pre-sizing width, but it is rated at the sized width |
| `added2#54` | confirmed | low | T04 | DXF reference circles of an eccentric gear come from Tooth::new, not the gear's mean tooth; they disagree with the drawn outline's tool and with the panel |
| `added2#56` | confirmed | low | T18 | README says check_strings/check_doc_links/validate_dxf are 'run by hand rather than in CI' |
| `added2#58` | confirmed | low | T18 | Five places still say ShaperCut refuses an oversized tip round; the code now caps it |
| `added2#59` | confirmed | low | T17 | contact::sliding_velocity and sliding_at are test-only, and their docs claim a unification production does not use |
| `added2#6` | confirmed | low | T18 | docs/reference.md contradicts itself on whether an unmentioned chain end is reacted |
| `added2#60` | confirmed | low | T18 | gear-io train.rs change log says 'Two changes so far' above 24 entries |
| `added2#61` | confirmed | low | T18 | Dangling 'appendix' pointers to the removed DESIGN document |
| `added2#62` | confirmed | low | T18 | reference.md's over-pins prose has the direction each space narrows backwards |
| `added2#64` | partly | low | T18 | auto.rs doc comment carries the same spur-only undercut reduction and tooth-count table |
| `added2#65` | partly | low | T18 | The corrections log's 'Where' column labels 85 rows with section numbers that no current document has |
| `added2#66` | partly | low | T18 | The 'drive' rule is contradicted by the English UI string itself |
| `added2#67` | confirmed | low | T18 | A strength.rs comment names Y_S in a product that multiplies K_f |
| `added2#68` | confirmed | low | T18 | The doc comment on the wasm smallest_tooth_count field states an unshifted formula as the design's limit |
| `added2#69` | confirmed | low | T16 | fillet_cap_guarantees_a_nonnegative_root_arc cannot catch the wrong cap it claims to guard against |
| `added2#7` | confirmed | low | T10 | An automatic face width tends to zero with the load, with no floor and no note |
| `added2#70` | confirmed | low | T17 | Stale intra-doc links to retired items; no check runs rustdoc |
| `added2#73` | partly | low | T07 | The 'close to locking' margin 0.8 × threshold is an unexplained literal |
| `added2#74` | confirmed | low | T18 | The code comment justifying GearCase.torque ('the mesh force at its reference cylinder') is false for crossed and worm meshes |
| `added2#75` | confirmed | low | T18 | Rationale and reference say only ultimate contact is off by default; the code turns both contact ratings off |
| `added2#77` | confirmed | low | T19 | A material's condition (the temperature and humidity its allowables are quoted at) crosses to the panel and is never shown |
| `added2#79` | confirmed | low | T05 | The ring solve ignores a ring's own shift bound: the ring tab validates x against the external gear's Ranges |
| `added2#8` | confirmed | low | T02 | train.load_shared's words blame a double hold for what is a redundant mesh loop |
| `added2#80` | confirmed | low | T02 | editTrain reports a boundary defect as a successful edit |
| `added2#81` | partly | low | T04 | Degenerate Gear::outline allocates over a million vertices instead of refusing |
| `added2#82` | confirmed | low | T13 | Edit::Release of a body that is not held, or does not exist, succeeds silently while Edit::Hold refuses NoSuchIndex |
| `added2#83` | confirmed | low | T02 | The tip radius is raised to the base circle silently |
| `added2#84` | partly | low | T18 | Dangling pointers to a DESIGN.md / 'design document' that no longer exists outside docs/history |
| `added2#88` | confirmed | low | T18 | Prose in three files says production drives auto::shifts_for_efficiency / auto::Pinned |
| `added2#89` | confirmed | low | T18 | default_materials' doc comment is spliced onto solve_ring, and default_materials has none |
| `added2#9` | confirmed | low | T18 | docs/reference.md says the train flow applies 'the output has to absorb' (T_out w_out <= 0); flow::solve does not test it |
| `added2#90` | confirmed | low | T01 | The addendum's legacy { auto, manual } arm accepts any unknown keys, silently breaking the document's deny_unknown_fields guarantee |
| `added2#91` | confirmed | low | T18 | The round-trip test's doc comment sits on the null-carrier test; the round-trip test has none |
| `added2#92` | confirmed | low | T11 | Gear::new cuts an ordinary gear's tooth twice (mean and teeth[0] are identical at Δx=0), which contradicts 'costs what it always did' |
| `added2#95` | confirmed | low | T18 | build_wasm.sh quotes payload sizes that are 27% stale |
| `added2#96` | partly | low | T18 | docs/reference.md says two conditions pick the physical power-flow branch; the suite shows they do not |
| `added2#97` | confirmed | low | T02 | External tooth's tip radius silently raised to the base circle, with no note (the ring's equivalent has one) |
| `added3#0` | unverified | low | T10 | Shape::assembly pools meshes from every body on a carried axis, judging independent planet bodies as one rigid stepped planet |
| `added3#1` | unverified | low | T10 | planets_not_evenly_spaced message names a sun and a sum, which is wrong for Wolfrom and stepped sets |
| `added3#11` | unverified | low | T06 | contact::efficiency's 'forward and backward are equal, physically it should be' is true only at first order in μ |
| `added3#12` | unverified | low | T19 | Every gear-wasm request caller in core.ts swallows boundary errors, so request-shape drift fails silently |
| `added3#13` | unverified | low | T02 | Error Display impls in gear-core hold a second, drifted copy of the English error text |
| `added3#15` | unverified | low | T14 | smallest_tooth_count is engineering math living in the wasm boundary crate |
| `added3#16` | unverified | low | T18 | TrainPanel comment says Rust 'has already chosen between the dry and conditioned states', a choice the core no longer makes |
| `added3#17` | unverified | low | T19 | addCaseOfKind selects the wrong case when editTrain fails silently |
| `added3#18` | unverified | low | T17 | auto.rs's own tests of shifts_for_efficiency test production-dead code |
| `added3#21` | unverified | low | T11 | flow.rs's energy condition is 'total loss ≥ 0', weaker than the documented 'output absorbs'; max-efficiency hides it |
| `added3#22` | unverified | low | T18 | rationale.md's solve table also says the tip-width solve is Newton with analytic ds/dr |
| `added3#23` | unverified | low | T16 | The sharing-sweep convergence test omits the small teeth where 200 samples are not converged below the band |
| `added3#24` | unverified | low | T05 | TipRoom reports tips clear on a stated internal distance whose far-side gap is negative at running |
| `added3#3` | unverified | low | T02 | A set with all three shifts given and its distance automatic is refused as 'these tooth counts cannot be assembled' when the shifts merely disagree |
| `added3#4` | unverified | low | T19 | GearPanel shows an external gear's undercut and pointed-tip figures as a ring's shift note |
| `added3#5` | unverified | low | T16 | The sharing-note test pins magnitudes taken from four hand-picked designs |
| `added3#6` | unverified | low | T19 | zh-Hant uses two words for 'load' (負荷 and 載荷) |
| `added3#7` | unverified | low | T07 | worm() seeds its helix from a hard-coded 7 mm even when the caller then sets another worm diameter |
| `added3#8` | unverified | low | T02 | A mesh with pinion flank interference is refused as 'the teeth never contact' |
| `added3#9` | unverified | low | T20 | check_golden.sh --fast passes vacuously when the harness lists no cases |
| `auto-search#10` | confirmed | low | T12 | admissible_profile_shift's doc still calls the cutter depth (1.20) the upper bound; the code's bound is tooth thickness (1.942) |
| `auto-search#11` | confirmed | low | T18 | Orphaned doc paragraph on root_radius_fits, and undercut_bound's table names a Decided::Given that does not exist |
| `auto-search#12` | partly | low | T12 | Search constants and tolerances with no stated origin, and one 'at the floor' tolerance written three ways |
| `auto-search#13` | partly | low | T12 | The pointed-tooth roll is solved three ways; the tip-width ceiling builds a whole tooth per candidate |
| `auto-search#14` | partly | low | T12 | searchable_shift bisects a piecewise-linear bound, re-running the undercut solve on every evaluation |
| `auto-search#15` | confirmed | low | T14 | Union-find written twice in shape.rs; state.md still describes per-stage objectives |
| `auto-search#16` | partly | low | T12 | The search's answer does not depend on friction: the objective is purely geometric |
| `auto-search#3` | partly | low | T12 | The chosen shifts jump by 0.2-0.9 module under continuous input changes, because the search picks between ridges that differ by about 1e-5 of efficiency |
| `auto-search#4` | partly | low | T12 | The convergence gate passes only on its fixtures: off-default and 7/40 surfaces miss the 1e-5 claim, and the search takes the lower ridge (F53 closure partial) |
| `auto-search#9` | confirmed | low | T18 | Search's doc says the set search is not converged (2.4e-4, 0.30 module) and gives the pair figure as 4.1e-7; both are stale |
| `crossed-worm#12` | partly | low | T07 | contact_ratio and locking_friction hold different quantities on the two contact kinds, including a -1.0 sentinel, so both jump at S = 0 |
| `crossed-worm#13` | confirmed | low | T07 | A crossed mesh's efficiency is computed at the early (pre-rating) face widths, but its zone, contact ratio and locking friction at the final ones |
| `crossed-worm#14` | confirmed | low | T18 | Two comments give the contact point's speed along the line of action as r_b/cos(beta_b) per radian; it is r_b cos(beta_b) |
| `crossed-worm#15` | confirmed | low | T18 | Stale, misplaced and self-historical comments in the crossed code and its tools |
| `crossed-worm#16` | confirmed | low | T07 | Note thresholds for near-locking (0.8 x threshold) and low efficiency (eta < 0.5) are unnamed literals, and the second fires forward-only on point contacts |
| `crossed-worm#17` | confirmed | low | T07 | Duplicated computations inside screw.rs and the point rating |
| `crossed-worm#18` | confirmed | low | T15 | Tolerances chosen by feel in the crossed construction, and a fixed quadrature count |
| `crossed-worm#19` | partly | low | T02 | English error text in gear-core uses retired 'stage' words, and the catalogue copy has drifted from it |
| `crossed-worm#2` | confirmed | low | T07 | The locking friction along the path assumes the path locks below the pitch point; false for forward drive, so it silently falls back and mixes models |
| `crossed-worm#20` | confirmed | low | T07 | arr::worm hard-codes a 7 mm diameter and module 1 into the helix, giving a NaN helix for more than 7 starts |
| `crossed-worm#3` | confirmed | low | T07 | As the shaft angle goes to 0 there are three wrong regimes before parallel: refusal, 100 % efficiency with no zone, and a cancellation-prone sliding ratio. Closed forms exist. |
| `crossed-worm#6` | partly | low | T07 | A cylindrical-wheel (point-contact) model is sized with enveloping-wheel conventions while its own zone of action gives the answer |
| `crossed-worm#7` | confirmed | low | T18 | The rationale for reporting no bending on a worm rests on claims that are false for the standards and for the crate's own wheel |
| `edit-ops#10` | confirmed | low | T13 | Four rules for when a body is bare |
| `edit-ops#11` | confirmed | low | T16 | Several edit-sizing constants are unpinned by any test |
| `edit-ops#12` | confirmed | low | T13 | Small inconsistencies in edit refusals |
| `edit-ops#13` | confirmed | low | T13 | An Insert offer sends a whole preset Shape, although the offer already names the preset |
| `edit-ops#3` | partly | low | T16 | The flow's power-dependent rules are untested: 4 of 4 mutations survive |
| `edit-ops#4` | partly | low | T13 | Preview's comparison is untested, and its path identity is (case index, entry count) |
| `edit-ops#7` | partly | low | T13 | No AddGear-on-an-existing-body offer on the ten presets gives a train that solves |
| `edit-ops#8` | confirmed | low | T11 | offers() spends most of its time on Debug-string compares, and the panel runs every selection's offers twice |
| `edit-ops#9` | partly | low | T11 | Every hover's dry run re-solves the train the panel has already solved |
| `gear-cli#0` | confirmed | low | T20 | shifts and crossed headers print 'mu 0.06' while the figures below are at the preset's mu 0.08 |
| `gear-cli#10` | confirmed | low | T20 | check_golden.sh aborts silently when a recorded command fails |
| `gear-cli#11` | partly | low | T20 | Small formulas the CLI computes itself, each a second or untested implementation |
| `gear-cli#12` | partly | low | T20 | The corpus prints roundoff and Debug names, which will make cosmetic diffs |
| `gear-cli#13` | confirmed | low | T20 | Wrong or stale prose in the harness: orphaned doc comments, a stale shifts comment, 'spur' for helical parts, audit-history counts |
| `gear-cli#14` | partly | low | T07 | A crossed pair whose first member is spur (β1 = 0) is refused as 'worm too thin', while β1 = 1e-6 builds |
| `gear-cli#2` | confirmed | low | T20 | loadcase's 'shared' column re-implements worst_over_cycle without its candidate points, so it always under-reads (c) below (b) |
| `gear-cli#5` | partly | low | T20 | The 'regression canary' `strength` hand-assembles a second rating pipeline that is not the product's configuration |
| `gear-cli#6` | partly | low | T20 | `verify 100` records only z = 3 gears, and misses the grid's actual worst case |
| `gear-cli#7` | partly | low | T20 | `dump` is a dead command: its reference (tools/dump_ref.py) no longer exists, and it is recorded as a bare hash |
| `gear-cli#8` | confirmed | low | T20 | `trainfile` duplicates `convert`'s five figures and is weaker than gear-io's own round-trip test; it writes a fixed /tmp path |
| `gear-cli#9` | confirmed | low | T20 | Positional arguments that fail to parse silently become defaults; error paths exit 0; broken pipe panics |
| `gear-io#10` | confirmed | low | T18 | Terminology for one concept varies within a language (contact ratio, ring, load, ground, port, profile shift, torque) |
| `gear-io#11` | partly | low | T18 | The pt catalogue mixes pre- and post-reform spelling although its header says it follows the Acordo Ortográfico |
| `gear-io#12` | partly | low | T15 | Engineering figures typed into message text duplicate code constants and doc figures that no check compares |
| `gear-io#13` | confirmed | low | T19 | A script subtag loses to a region subtag in the language resolver: zh-Hans-HK resolves to Traditional |
| `gear-io#14` | confirmed | low | T08 | The 4340 hardened steel's fatigue estimate keeps the figure its own note calls optimistic, without a size and sign in state.md |
| `gear-io#15` | confirmed | low | T16 | No check compares ui.* placeholders with what the panel passes |
| `gear-io#16` | confirmed | low | T18 | Small inaccuracies in gear-io comments and test names |
| `gear-io#4` | confirmed | low | T01 | The train reader accepts NaN in every float field, and a NaN input is then misreported by the solve |
| `gear-io#6` | partly | low | T02 | gear-io's own error messages (library and train import) reach the panel in English in all five languages |
| `gear-io#7` | confirmed | low | T01 | An unknown key at the train document's root is silently dropped; the module doc says every struct refuses one |
| `gear-io#8` | partly | low | T14 | Defaulted fields read old files through live constants, so changing a default silently re-means every older file; the doc says refusal is the rule |
| `gear-io#9` | confirmed | low | T19 | zh-Hant catalogue contains five Simplified characters |
| `gear-outline#10` | partly | low | T04 | `corrected` identifies the root arc by exact float equality `r == g.rf` |
| `gear-outline#11` | confirmed | low | T18 | Misattached and stale doc comments in gear.rs, outline.rs and rationale.md |
| `gear-outline#12` | confirmed | low | T04 | The tolerance floor is set by feel and silent; the depth cap is what actually binds |
| `gear-outline#14` | partly | low | T04 | The feasibility edge in `amplitude_for_throw` is bisected (64 solves) where a closed form exists |
| `gear-outline#2` | partly | low | T16 | `a_rings_outline_refines_when_asked_to` measures an external gear, never the ring |
| `gear-outline#4` | confirmed | low | T04 | `sinusoid_backlash` leaves out the kind's sign: negated for an internal mate |
| `gear-outline#8` | confirmed | low | T04 | Eccentric export emits z zero-length segments (one per tooth, including across the closure) |
| `gear-outline#9` | partly | low | T04 | The outline does not clamp a pointed tooth's theta_a ≥ 0 as the profile does, leaving round-off bowties |
| `graph-ops#10` | confirmed | low | T18 | Ports' doc contradicts Shape::ports on the hula (order, and whether the wobble body is a port) |
| `graph-ops#11` | confirmed | low | T15 | Default distance clearance, tolerances and mesh friction are literals in arrangements.rs, written three times; pair() and planetary() re-set what push_distance already set |
| `graph-ops#12` | confirmed | low | T07 | Two definitions of 'a worm': worm_and_pair's worm is not worm()'s, and 7 mm is written twice |
| `graph-ops#13` | partly | low | T14 | Preset::family duplicates Shape::family by hand; small inconsistencies in the builders |
| `graph-ops#14` | confirmed | low | T15 | The panel's fresh-case figures are written twice in gear-wasm |
| `graph-ops#15` | confirmed | low | T18 | conditions.rs comments carry refactor history and a pointer to the working handoff notes |
| `graph-ops#6` | confirmed | low | T13 | Join keeps a load at a body it has just made held |
| `graph-ops#7` | confirmed | low | T13 | pub Train::hold / join / split skip the validation Edit::Hold / Edit::Join apply |
| `graph-ops#9` | confirmed | low | T18 | graph_of does not keep a stage's body order when a stage shares two bodies with earlier stages in the other order |
| `kinematics-flow#5` | confirmed | low | T16 | breakaway.py is not independent of the crate: it reimplements the same 2^M search, filters and max-efficiency choice |
| `kinematics-flow#6` | confirmed | low | T18 | flow::solve's branch choice is 'maximum efficiency', while its comment says 'first consistent' and 'a second genuinely different one would be a finding'; multiple branches are common |
| `kinematics-flow#7` | partly | low | T11 | The flow's rank and consistency are decided with float tolerances (1e-12 absolute pivot, 1e-9 relative), unlike the exact kinematics |
| `kinematics-flow#8` | confirmed | low | T17 | MeshFlow::paths is written and never read by the flow |
| `kinematics-flow#9` | confirmed | low | T11 | Condition order in System::solve decides overflow as well as which conflict is named, yet its docs say 'The order changes no answer' |
| `lens-architecture#1` | partly | low | T14 | Engineering defaults live in gear-wasm and are written twice (fresh-case figures, UI seeds) |
| `lens-architecture#10` | partly | low | T14 | Three records of one gear's inputs and three result pipelines (gear tab, ring tab, train member) |
| `lens-architecture#11` | confirmed | low | T14 | Five hand-rolled union-find / connected-component routines |
| `lens-architecture#12` | confirmed | low | T14 | TipRoom (pure ring geometry) lives in train/mod.rs and auto.rs reaches up into train for it |
| `lens-architecture#13` | partly | low | T14 | wasm surface: 23 hand-written entry points, each an _impl + JsError wrapper, mirrored by 23 hand-written try/catch wrappers in core.ts |
| `lens-architecture#15` | partly | low | T17 | About 137 of 384 pub functions in gear-core are used only inside gear-core; odd shims like `index_pub` |
| `lens-architecture#16` | partly | low | T14 | Small API/naming inconsistencies at the crate seams |
| `lens-architecture#17` | confirmed | low | T18 | rationale.md says k₂ = 2 − k₁ is 'derived and the invariant is unwritable', but k is stored per member and both may be given |
| `lens-architecture#2` | partly | low | T02 | Five refusal channels cross the boundary; an edit refusal is detected by string-prefix sniffing |
| `lens-architecture#4` | partly | low | T13 | Dense-index graph with renumbering on every removal; the UI re-derives the core's renumbering |
| `lens-architecture#5` | partly | low | T14 | shape::rate is one 556-line function of ~20 nested closures; it also sizes face widths, so the cut/rate split is not 'geometry vs load' |
| `lens-architecture#8` | partly | low | T14 | The 'one seam' between line/point contact and rack/ring members leaks: 37 kind dispatches in shape.rs; a bevel pair would touch them all |
| `lens-continuity#2` | partly | low | T12 | Given-distance shift search jumps basins and staircases: 2 µm of distance moves the division by 0.17 module for a 1e-6 change in η |
| `lens-continuity#5` | confirmed | low | T18 | The 'where closed form is impossible' inventory is stale: it lists ten solves and one search; the crate has about 25 root finds, two quadratures, two bisections and three searches |
| `lens-continuity#6` | confirmed | low | T17 | `split_residual`, `efficient_split` and `ContactPath::tip_pressure_angle` are production-dead |
| `lens-continuity#8` | confirmed | low | T06 | The parallel-axis efficiency is the first-order linearisation while the crossed one is the exact balance, so the Σ → 0 limit carries an O(μ²) seam |
| `lens-docs-accuracy-1#10` | confirmed | low | T18 | The 'four in tooth.rs' list puts ShaperCut in tooth.rs and names no tests |
| `lens-docs-accuracy-1#11` | confirmed | low | T18 | The corpus paragraph's mutation measurement has drifted |
| `lens-docs-accuracy-1#12` | confirmed | low | T18 | state.md figures and descriptions left over from retired solvers |
| `lens-docs-accuracy-1#13` | confirmed | low | T18 | state.md says gear.rs is the only place a gear is drawn |
| `lens-docs-accuracy-1#14` | confirmed | low | T18 | Parts of the census paragraph are self-referential history in the map |
| `lens-docs-accuracy-1#15` | confirmed | low | T16 | A broken link from corrections.md to README.md, invisible to check_doc_links |
| `lens-docs-accuracy-1#16` | confirmed | low | T18 | The geartrain format's change log says 'Two changes so far' above a dozen |
| `lens-docs-accuracy-1#17` | partly | low | T18 | Small README inaccuracies: basis display, translating, gear-io role |
| `lens-docs-accuracy-1#18` | partly | low | T18 | The untested-modules paragraph leaves train/pair.rs unaccounted for |
| `lens-docs-accuracy-1#19` | confirmed | low | T14 | Three train modules are named for retired concepts and are nearly empty |
| `lens-docs-accuracy-1#20` | confirmed | low | T17 | An empty package-lock.json is committed at the repo root |
| `lens-docs-accuracy-1#3` | partly | low | T18 | The 1.1-million-case claim for the flow is 179,280 cases of the closed form |
| `lens-docs-accuracy-1#5` | partly | low | T18 | The canary section contradicts itself and the bending section |
| `lens-docs-accuracy-1#7` | partly | low | T18 | The root working notes are stale throughout and overdue for deletion |
| `lens-docs-accuracy-1#8` | confirmed | low | T18 | README's by-hand list includes CI checks and leaves out breakaway.py |
| `lens-docs-accuracy-1#9` | partly | low | T18 | The map says every transcendental step goes through solve.rs; several hand-rolled searches do not |
| `lens-docs-accuracy-2#1` | confirmed | low | T18 | The 'ten scalar solves, everything else algebraic' inventory is far from complete, and two bisections bypass solve.rs |
| `lens-docs-accuracy-2#10` | partly | low | T18 | 'Note::is(key), checked by the compiler' — it takes &str, and the crate's own test shows a mistyped key compiling |
| `lens-docs-accuracy-2#11` | confirmed | low | T18 | Stale API pointers and counts in rationale.md, reference.md and corrections.md |
| `lens-docs-accuracy-2#12` | confirmed | low | T15 | The crate still writes out the base pitch and the angular-play law inline, despite the 'one place' claims |
| `lens-docs-accuracy-2#13` | partly | low | T16 | The efficiency's 'independent numerical average' shares the helical cos β_b assumption and a looser tolerance than documented |
| `lens-docs-accuracy-2#14` | confirmed | low | T18 | The naming rule 'never a worm drive' is broken in reference.md and in code comments |
| `lens-docs-accuracy-2#15` | confirmed | low | T18 | The corrections log links bending faults to #contact-stress |
| `lens-docs-accuracy-2#2` | partly | low | T18 | root_radius is documented as normal modules but is multiplied by the transverse module |
| `lens-docs-accuracy-2#3` | partly | low | T18 | The claim that every input bound sits exactly where the generator's guards clamp is false for pressure angle and dedendum |
| `lens-docs-accuracy-2#4` | confirmed | low | T03 | The tip-width function s(r′) is called monotone decreasing, but its stated slope is positive at the base circle |
| `lens-docs-accuracy-2#5` | confirmed | low | T18 | Stale figure: parabola vs 30° tangent Y_F range |
| `lens-docs-accuracy-2#6` | confirmed | low | T18 | 'Every row of every table reports which bound stopped it' — no table or command does, and reference.md itself calls 17/43's optimum interior |
| `lens-docs-accuracy-2#7` | confirmed | low | T18 | Internal-gear generation limit and junction written with α_t where the code uses the shaper's cutting angle |
| `lens-docs-accuracy-2#8` | confirmed | low | T07 | Crossed shift law written with the radial shift x; the code (correctly) uses the thickness shift x + x_s |
| `lens-docs-accuracy-2#9` | confirmed | low | T18 | The spur-only undercut reduction is stated as general |
| `lens-docs-clarity#0` | confirmed | low | T18 | README claims gear-cli's module comment lists every command; that comment says it no longer does |
| `lens-docs-clarity#10` | partly | low | T18 | CLAUDE.md opens with a 20-line parenthetical census history |
| `lens-docs-clarity#11` | confirmed | low | T18 | reference.md, which says it 'states; it does not argue', carries history parentheticals and 'used to' prose |
| `lens-docs-clarity#12` | partly | low | T18 | Two stale standalone documents: the refactor plan/handoff (1,420 lines) and docs/history/design-record.md (4,408 lines, unreferenced) |
| `lens-docs-clarity#13` | confirmed | low | T18 | 'milestone N' and 'Phase N' references to a deleted brief |
| `lens-docs-clarity#14` | partly | low | T18 | Modules whose production code is a thin shell under long derivation prose (train/crossed.rs: 70-line header over ~70 lines of code) |
| `lens-docs-clarity#2` | partly | low | T18 | README's 'Nothing appears in two of them' is false; systematic restatement across README/CLAUDE.md/docs/code |
| `lens-docs-clarity#3` | partly | low | T18 | History narrated inside living text: 202 production comment lines, 14 'this paragraph used to say' passages |
| `lens-docs-clarity#6` | confirmed | low | T18 | Stale comment: MeshReport describes per-stage results the core no longer has |
| `lens-docs-clarity#7` | confirmed | low | T18 | Corrections log links bending faults to #contact-stress under opaque section numbers of a deleted spec |
| `lens-docs-clarity#8` | partly | low | T18 | CLAUDE.md promises 'the tests that hold each' tooth.rs caution and names none |
| `lens-errors-policy#11` | partly | low | T02 | Five different boundary conventions for a refusal |
| `lens-errors-policy#12` | partly | low | T02 | Nine core error enums, plus Option returns and three note channels, where one Refusal-as-Note model would serve |
| `lens-errors-policy#13` | confirmed | low | T02 | Zero face width is rated to silent nulls, while a negative one is refused as 'no contact' |
| `lens-errors-policy#14` | confirmed | low | T02 | Dead fallbacks that would silently read distance 0 or zero play |
| `lens-errors-policy#15` | confirmed | low | T13 | Case edits never refuse, while graph edits do, and a fresh case's figures are duplicated |
| `lens-errors-policy#16` | confirmed | low | T19 | Train panel inputs commit any finite value: a fractional tooth count breaks the boundary, and bounds are typed as HTML attributes |
| `lens-errors-policy#18` | partly | low | T18 | params.rs says root_radius is in normal modules, but the rack multiplies it by the transverse module |
| `lens-errors-policy#19` | partly | low | T16 | Only unwrap is linted; expect, panic and slice indexing are not, which is how gear.rs:564's panic got through |
| `lens-errors-policy#3` | partly | low | T02 | Refusals name the wrong cause: 'the teeth never contact' now covers bad materials, non-positive face width and module zero |
| `lens-errors-policy#7` | partly | low | T02 | Clamps with no note: planet count 0, cutter teeth 0, tip below the base circle, negative root radius |
| `lens-errors-policy#8` | confirmed | low | T03 | The pressure-angle guard is applied to the tooth but not to thickness_shift(), so a clamped tooth is 7.4% too thin |
| `lens-errors-policy#9` | partly | low | T02 | Gear::profile panics on a degenerate tooth, and Gear::new's 'never panics / guarded to one, as elsewhere' is false |
| `lens-feature-gaps#12` | confirmed | low | T21 | Export is DXF only; SVG is a transcription of the bulge outline, STEP and a 3D view are large |
| `lens-feature-gaps#13` | partly | low | T19 | The geartrain has no drawing: a pair in mesh, or a set, is never seen |
| `lens-feature-gaps#16` | confirmed | low | T21 | Loads are stated as torque only; a power input is missing |
| `lens-feature-gaps#4` | partly | low | T21 | One mesh-stiffness model from the exact profile would replace the uncalibrated ramp and make TE and tip relief askable |
| `lens-feature-gaps#5` | confirmed | low | T21 | Crowning is one argument away: lengthwise_curvature is the unifying parameter and is hard-wired to zero |
| `lens-feature-gaps#6` | partly | low | T21 | Specific sliding is closed form from what the mesh already has, and is not reported |
| `lens-feature-gaps#8` | partly | low | T21 | Power lost per mesh (W) and a thermal/tooth-temperature check are absent; for polymers temperature governs the allowable |
| `lens-magic-numbers#0` | partly | low | T15 | Near-self-locking (0.8) and low-efficiency (0.5) warning thresholds are unexposed rules of thumb |
| `lens-magic-numbers#10` | confirmed | low | T03 | The pointed-tooth radius is solved twice, with its own bracket constant, though it is the inverse involute |
| `lens-magic-numbers#11` | partly | low | T15 | Eleven hand-written bracket-growing and bisection loops, five growth factors, six caps |
| `lens-magic-numbers#13` | confirmed | low | T15 | Edits refuse fewer than 4 teeth, require a ring 2 teeth larger, and default a sun to 6 teeth: all unexplained, and the 4 does not match the core's floor of 1 |
| `lens-magic-numbers#14` | confirmed | low | T15 | The lean past the tip-clearance root is 1e-9 mm absolute, not 'the solver's own tolerance' as its comment says |
| `lens-magic-numbers#16` | confirmed | low | T18 | The worm wheel's face-width cap is attributed to the wrong source |
| `lens-magic-numbers#2` | confirmed | low | T19 | Rule 1 broken: the eccentric-gear mate's tooth count (43) is typed in TypeScript |
| `lens-magic-numbers#3` | partly | low | T15 | Load-case defaults written twice in gear-wasm, and kept in the boundary rather than the core |
| `lens-magic-numbers#4` | confirmed | low | T15 | MemberGear::default repeats GearParams::default by hand (17 teeth, addendum 1, dedendum 1.25, root 0.38) |
| `lens-magic-numbers#5` | confirmed | low | T07 | Worm preset: 7 mm written four times, and the helix formula assumes module 1 (NaN above 7 starts) |
| `lens-magic-numbers#6` | confirmed | low | T15 | The shipped 0.02 mm clearance and tolerance are written nine times in arrangements.rs |
| `lens-magic-numbers#7` | partly | low | T15 | Seeded values are rounded to 1e-4 absolute, so small torques lose precision |
| `lens-magic-numbers#8` | partly | low | T15 | Two thresholds for one question, whether a mesh carries power (1e-12 in flow, 1e-9 in groupings) |
| `lens-magic-numbers#9` | confirmed | low | T15 | The shift-floor check in auto.rs uses 1e-12 and skips compat::SAME_SHIFT (1e-9), which Tooth::undercut uses |
| `lens-numerical-robustness#6` | confirmed | low | T02 | admissible_angular_shift returns Some(±inf) for a one-tooth member; Bound has None for 'unbounded', and ±inf crosses the boundary as null |
| `lens-numerical-robustness#7` | partly | low | T03 | Two formulas written out repeatedly: the roll from radius (about 12 copies) and the cutter tip width/round fit (tooth.rs and auto.rs) |
| `lens-numerical-robustness#9` | confirmed | low | T02 | Tooth exposes NaN as a sentinel in public fields (u_j, u_tip) for a severed tooth; Note::number formats non-finite values without a guard |
| `lens-performance#10` | partly | low | T15 | least_squares_exact's absolute pivot floor 1e-12 refuses long trains' flows as 'load shared' |
| `lens-performance#12` | confirmed | low | T18 | The Cargo.toml wasm-profile table and the 'microseconds' claims are stale |
| `lens-performance#5` | confirmed | low | T11 | paths_of recomputes each mesh's play coefficient three times, each a full exact re-reduction of the whole system |
| `lens-performance#7` | partly | low | T14 | Typed TOML deserialisation monomorphs are ~257 KB of the wasm; the JSON deserialisers for the same types are already there |
| `lens-performance#8` | partly | low | T11 | preview re-solves the unedited train on every hover and fully rates both trains to read one path |
| `lens-performance#9` | confirmed | low | T11 | default_library() re-parses the material TOML on every wasm call that sends no library |
| `lens-standards#4` | partly | low | T21 | Fatigue cases count tooth cycles precisely, but no rating reads them: the allowable ignores life, and a material's fatigue figure has no cycle count |
| `lens-standards#9` | partly | low | T21 | No scuffing, micropitting or dynamic-load check, and speed enters no rating |
| `lens-tests-geometry#1` | partly | low | T16 | The rack simulation builds its cutter from the Tooth's own derived rack, so it cannot detect a misplaced or mis-sized rack; its grid never turns module, k, dedendum or addendum |
| `lens-tests-geometry#10` | partly | low | T16 | No fixture pins an uncapped helical fillet radius; ρ = root_radius·m_t is held only indirectly |
| `lens-tests-geometry#11` | partly | low | T16 | Several threshold tests where an exact value is available |
| `lens-tests-geometry#3` | partly | low | T16 | The bending rack-limit tests use tolerances 10⁴× looser than the law they could assert |
| `lens-tests-geometry#4` | confirmed | low | T16 | extremes::addendum_and_dedendum_bound_each_other asserts a tautology, and nothing pins the addendum floor |
| `lens-tests-geometry#5` | confirmed | low | T16 | No test in the suite pins the contact modulus of two different materials; the one that claims to asserts a tautology |
| `lens-tests-geometry#6` | partly | low | T09 | The JGMA transcription checks cannot see a wrong cell that stays a preferred number and stays monotone in grade, and one cell breaks monotonicity across module bands |
| `lens-tests-geometry#7` | confirmed | low | T16 | auto::the_sharp_rack_reproduces_the_classical_tooth_count_rule never calls the crate |
| `lens-tests-geometry#8` | confirmed | low | T16 | The span test skips every helical case on a false premise, so a helical span's absolute value is pinned only code against code |
| `lens-tests-geometry#9` | confirmed | low | T18 | Stale and fused prose in the integration tests |
| `lens-tests-train#10` | partly | low | T18 | CLAUDE.md says three edit laws 'sweep the presets'; two are hand lists, and the one that sweeps asserts little |
| `lens-tests-train#11` | confirmed | low | T16 | Fixtures and solve helpers are written over and over; the train has no shared grid like tests/common |
| `lens-tests-train#4` | confirmed | low | T16 | Assertion branches that never execute: four laws check less than their docstrings claim |
| `lens-tests-train#7` | confirmed | low | T16 | The kinematics 'every preset' law compares the graph with itself, and member speeds only by magnitude |
| `lens-tests-train#8` | partly | low | T16 | thickness_modification_cannot_break_its_own_invariant cannot fail and duplicates a stronger shape.rs test |
| `lens-tests-train#9` | confirmed | low | T16 | The flow listing's 'most power first' order is untested |
| `lens-unification#10` | confirmed | low | T14 | Remaining rule-4 residue: integer and pair rules re-matched on MeshKind |
| `lens-unification#11` | confirmed | low | T17 | Test-only oracles and fixtures shipped in the production API |
| `lens-unification#4` | confirmed | low | T02 | The line-contact play re-derives angular_play by hand, and hides a failed backlash as zero |
| `lens-unification#7` | partly | low | T06 | Two efficiency models for one phenomenon: a first-order-in-μ loss for line contacts, the exact force balance for point contacts |
| `lens-unification#9` | confirmed | low | T12 | The admissible-range bounds hand-copy the tooth's clamp rules |
| `mesh-contact#10` | confirmed | low | T17 | split_residual / efficient_split: public, unused in production, and wrong under the default no_sharp_tip |
| `mesh-contact#11` | confirmed | low | T18 | Stale or self-contradicting prose in contact.rs and mesh.rs |
| `mesh-contact#12` | confirmed | low | T06 | The interference verdict is taken at the running distance, not at the tightest end of the tolerance band |
| `mesh-contact#3` | confirmed | low | T02 | A band end below the base-circle limit is read as zero play, and a negative distance passes the pressure-angle guard |
| `mesh-contact#4` | confirmed | low | T05 | ring::mesh_with recomputes path length, contact ratio, operating angle and rack compatibility beside ContactPath and Mesh |
| `mesh-contact#5` | confirmed | low | T06 | Above ε = 2 the linear ramp's shares sum to more than the load (up to 1.73×) |
| `mesh-contact#6` | confirmed | low | T16 | A test's 'least shift' foil is a design whose flanks interfere, and its doc quotes it as a design |
| `mesh-contact#7` | confirmed | low | T20 | gear-cli shifts and crossed print a hard-coded 'mu 0.06' while solving at the default 0.08 |
| `mesh-contact#8` | confirmed | low | T16 | The efficiency 'verification' tests share the assumption they are meant to check |
| `metrology#10` | confirmed | low | T18 | Other doc comments that disagree with their code |
| `metrology#11` | confirmed | low | T09 | pin_diameter_range uses its own doubling and 64-step bisection where closed forms, or solve.rs, would do |
| `metrology#12` | confirmed | low | T17 | Every measurement has two copies, one on Tooth and one on Gear; the Tooth copies ship only to tests |
| `metrology#13` | confirmed | low | T09 | `limits` fields are always None; tolerance limits could come from a user-entered thickness allowance in closed form |
| `metrology#14` | confirmed | low | T09 | The default pin is a fixed 1.75 mm whatever the module; the 'ideal' ball has a closed form |
| `metrology#15` | partly | low | T09 | A helical span's face-width condition (b > W_k sin beta_b) is neither checked nor stated |
| `metrology#16` | confirmed | low | T09 | UI labels say 'pins' and 'composite error' where the numbers are for balls on helical gears and are double-flank tolerances |
| `metrology#17` | partly | low | T16 | Test gaps: helical textbook span skipped on a false ground, preferred-number check almost cannot fail, no monotonicity in module or diameter, no metrology in the golden corpus |
| `metrology#18` | partly | low | T09 | Small API and robustness issues |
| `metrology#3` | partly | low | T09 | The two JGMA scales are one ladder offset by four grades, not independent scales |
| `metrology#4` | partly | low | T09 | The JGMA table could be replaced or complemented by ISO 1328-2's continuous closed-form formula for the same two quantities |
| `metrology#5` | unverifiable | low | T09 | The standard-scale transcription stops at module 10; JGMA 116-02's main body is reported to cover m_n 1 to 40 |
| `metrology#6` | confirmed | low | T19 | English tolerance-scale names reach all five languages through `Display for Class`, and Svelte hard-codes them |
| `metrology#8` | confirmed | low | T18 | reference.md has the direction in which a space narrows backwards for both gear kinds |
| `metrology#9` | confirmed | low | T18 | Doc comments contradict the pin-diagnosis fix recorded in corrections.md |
| `primitives#1` | confirmed | low | T03 | thickness_shift uses the unguarded pressure angle while the geometry uses the guarded one |
| `primitives#10` | partly | low | T15 | The pressure-angle guard is written five times, twice without its note |
| `primitives#13` | confirmed | low | T15 | Bracket growth and predicate bisection are hand-rolled at least seven times, each with its own factor and cap |
| `primitives#14` | partly | low | T19 | Core-formatted note values cannot be localised, can print '-0.000', and readouts round in TypeScript anyway |
| `primitives#15` | confirmed | low | T15 | The contact-ratio-below-one note is raised at two sites with the threshold and decimals copied |
| `primitives#2` | confirmed | low | T02 | Two guards fire without a note, contradicting the Clamps contract |
| `primitives#4` | confirmed | low | T15 | Tol's 'relative' tolerance is absolute for every \|x\| < 2.25, and the two solvers read x_tol differently |
| `primitives#5` | partly | low | T02 | brent/newton_bracketed return None for 'no root' and for 'failed' alike, and callers turn it into a value |
| `primitives#6` | confirmed | low | T18 | The solve inventory and inverse-involute docs are stale: 26 solver sites not ten, #2 is Brent, the bare scheme diverges at 74° not 60° |
| `primitives#8` | confirmed | low | T15 | The pointed-tooth roll (u − atan u = ψ_b) is an inverse involute solved by hand twice, with two different solvers |
| `primitives#9` | confirmed | low | T15 | plane.rs identities and base_pitch are re-written at call sites; transverse module m/cos β is written eight times |
| `ring#10` | confirmed | low | T05 | Stale or contradictory prose: σ counted as three places vs two vs none in rolling; a sin α_t where the code uses α_w; crossing angles 'away from the mesh' though measured from it; stale profile:: path |
| `ring#11` | confirmed | low | T05 | Junction travel is found by bracket expansion (−m_t, ×1.4, 64 tries) and Brent though it has a closed form; assorted literals |
| `ring#12` | partly | low | T17 | ShaperCut::equivalent_to_rack and the External shaper kind are public production API used only by tests |
| `ring#7` | partly | low | T05 | RingMesh re-derives Mesh + ContactPath + flank interference; in production only the tip margin is read |
| `ring#8` | partly | low | T05 | A train ring is built twice: a shaper-cut Ring and a whole rack-cut external Tooth (as_gear) that only lends its params to Mesh::new |
| `shape-a#10` | partly | low | T10 | A stated pitch diameter below z*m silently becomes a 0 degree helix |
| `shape-a#11` | partly | low | T15 | The tip-sizing walk and the solver brackets are full of unnamed tolerances and step constants |
| `shape-a#13` | partly | low | T14 | The data model says one thing several ways: helix three ways with last-wins, per-group module stored per member, overlap duplicated by the panel, ring position by convention |
| `shape-a#14` | confirmed | low | T10 | The thickness rule 'at most one given per mesh' does not bind a chain; the documented invariant is not what relief keeps |
| `shape-a#15` | confirmed | low | T14 | Three hand-written union-finds in one file, two of them over the same mesh relation |
| `shape-a#16` | confirmed | low | T18 | Comments cite retired code and narrate history |
| `shape-a#17` | confirmed | low | T14 | chosen_at, sized, plan_held and helix_angles are 100-220-line functions mixing several concerns |
| `shape-a#6` | partly | low | T07 | A distance at a tiny non-zero angle breaks the point-contact model (refused at 1e-9 degrees, eps 0 and efficiency 1 at 1e-6) |
| `shape-a#8` | partly | low | T14 | A mesh's frame is decided per member, so one sun gear meshing two carriers' planets (a Simpson) is refused |
| `shape-b#11` | confirmed | low | T07 | Three different rules decide what a worm thread is |
| `shape-b#12` | confirmed | low | T18 | LayoutReport doc and note wording are stale; the load-share note is raised once per axis |
| `shape-b#2` | confirmed | low | T10 | The assembly rule pairs meshes across independent bodies on one planet axis, giving a false 'not evenly spaced' |
| `shape-b#3` | confirmed | low | T10 | No assembly answer for meshed planets or Ravigneaux, though the condition is derivable and was brute-force verified |
| `shape-b#5` | confirmed | low | T10 | Tooth cycles take the max over a member's meshes, so a gear loaded on one flank by two meshes is counted half |
| `shape-b#7` | confirmed | low | T07 | A point mesh's member contact stress is read at the early width while the mesh report uses the final width, so one contact gets two figures |
| `shape-b#8` | confirmed | low | T07 | Point-mesh efficiency is taken at the early (box) width, so marking a width auto or typed at the same value changes the efficiency |
| `strength#12` | confirmed | low | T06 | The load-sharing sweep is a 200-sample scan with a tuning count; rationale says the crate has only one search |
| `strength#13` | confirmed | low | T16 | Tests that cannot fail or claim more than they assert, and a ring rim arm with no coverage |
| `strength#14` | confirmed | low | T17 | Option returns that are never None, a dead parameter and small magic numbers in strength.rs and hertz.rs |
| `strength#15` | confirmed | low | T08 | reversed_bending_allowable reports a derived-from-estimate value as 'Derived', and gear-core writes English notes |
| `strength#16` | confirmed | low | T08 | Library estimate conventions are not class-uniform, one is untested, and a Known-approximate entry lacks size and sign |
| `strength#17` | confirmed | low | T20 | The CLI strength canary labels F_bt as 'F_n'; the matrix prints NaN% and names Savage's J 'Y_F·K_f' |
| `strength#4` | partly | low | T08 | The Dolan–Broghamer K_f is extrapolated beyond its 14.5–20° data and loses its fillet sensitivity; the doc says 25° is 'inside' and that the fit 'needs no band' |
| `strength#7` | partly | low | T08 | The Hertz max(ellipse, line) under-predicts near the crossing, and the size is unrecorded |
| `tools-ci#10` | confirmed | low | T14 | Four boundary types are still hand-mirrored in TypeScript, outside check_bindings |
| `tools-ci#11` | confirmed | low | T16 | Every script that 'builds, never merely finds' the binary ignores CARGO_TARGET_DIR |
| `tools-ci#12` | confirmed | low | T16 | CI compiles the workspace about 7 times; cheap gates run after the 5-minute build |
| `tools-ci#13` | confirmed | low | T18 | docs/state.md's 'Running it' list omits check_units, check_wasm and validate_dxf, and says 'run all four' |
| `tools-ci#14` | partly | low | T16 | Lints: `expect`, `panic`, `unreachable` and float equality are uncounted; many allow attributes are no-ops |
| `tools-ci#15` | partly | low | T04 | validate_dxf covers one external gear; ring DXF export has no independent reader; expected radii are hand-typed in YAML |
| `tools-ci#16` | confirmed | low | T16 | check_golden --fast silently passes if --golden-cases fails |
| `tools-ci#17` | partly | low | T18 | Stale or inaccurate statements in build and CI comments |
| `tools-ci#18` | confirmed | low | T18 | CLAUDE.md's opening census is hand-maintained history that dates with every commit, against its own rule |
| `tools-ci#2` | confirmed | low | T16 | `figures-by-test` blocks can drift from their test: the gate only checks that the name exists |
| `tools-ci#3` | partly | low | T16 | check_wasm does not measure the shipped module, and its entry-point grep misses three common export forms |
| `tools-ci#4` | confirmed | low | T16 | npmDepsHash trap is avoidable: importNpmLock builds a byte-identical site |
| `tools-ci#5` | confirmed | low | T16 | check_doc_links: the promised 'heading with no referrer' report does not exist, and whole classes of dead pointer pass |
| `tools-ci#6` | partly | low | T16 | check_units misses unit-silent pub angles in tooth.rs and the parameter-level ambiguity it was written for |
| `tools-ci#7` | confirmed | low | T16 | CI's tests job holds id-token: write while running npm ci and project code, contradicting its own comment |
| `tools-ci#8` | confirmed | low | T16 | Repo-wide `concurrency: group: pages` serialises every run and lets pushes on one branch cancel pending runs on another |
| `tools-ci#9` | confirmed | low | T17 | Doctests and rustdoc intra-doc links are never checked: 22 broken links |
| `tooth-form#10` | confirmed | low | T03 | The documented transverse-round residual has a closed-form repair: parameterise the fillet by the corner's contact normal |
| `tooth-form#2` | confirmed | low | T16 | check_ring_cut's reported error is its own radius-bin quantisation (3.6 µm against a true 2e-8 mm), and its 5 µm gate is slack by the same amount |
| `tooth-form#4` | confirmed | low | T03 | The documents contradict each other on whether ShaperCut refuses or caps a round, and its two constructors answer the same guard two ways |
| `tooth-form#5` | confirmed | low | T03 | Stale module names and an overstated claim in the fit-cap commentary |
| `tooth-form#6` | confirmed | low | T03 | Tip below the base circle is raised silently |
| `tooth-form#7` | partly | low | T03 | Tooth::cut_by accepts a tool whose round exceeds its depth at this shift and silently takes the wrong trochoid branch |
| `train-mod-a#10` | partly | low | T10 | Relief clamps Clearance on a distance sized by its tips without releasing it, so a typed clearance is silently replaced (Planocentric) |
| `train-mod-a#11` | confirmed | low | T14 | GearResult repeats params: profile_shift, addendum and helix_angle are bit-identical to params.*, and MemberFacts' reason for them is stale |
| `train-mod-a#12` | confirmed | low | T17 | Broken intra-doc links in scope, pointing at renamed or deleted items |
| `train-mod-a#13` | partly | low | T18 | Much of the prose in scope is the history of retired stage types, not a description of the code |
| `train-mod-a#14` | confirmed | low | T02 | TrainError's Display carries a second English wording of every error, beside the string catalogue |
| `train-mod-a#15` | confirmed | low | T15 | relieved_from rounds a seeded figure to 1e-4 in the core, and so changes the design it pins |
| `train-mod-a#7` | partly | low | T10 | The overlap-to-helix relation jumps at its limit: ε_β just below b/(π m) gives an 87° helix, exactly at the limit is a misleading refusal, and above it the helix drops to 0 |
| `train-mod-b#10` | confirmed | low | T15 | Seed rounding to 1e-4 absolute is written three times and loses small figures |
| `train-mod-b#13` | partly | low | T13 | The case's first-load accessors (torque/speed/set_*) read a declared reaction when it is listed first |
| `train-mod-b#14` | confirmed | low | T10 | FaceSources::width_for jumps from a vanishing ask to the given width at exactly zero |
| `train-mod-b#8` | confirmed | low | T18 | Docs say an unmentioned chain end is reacted; the code treats every unmentioned port as free |
| `train-mod-b#9` | partly | low | T10 | A free port declared in so many words counts differently in relief from an unmentioned (default) free port |
| `wasm-boundary#10` | confirmed | low | T18 | Stale or misplaced documentation at the boundary |
| `wasm-boundary#11` | confirmed | low | T04 | The eccentric gear's mate silently inherits this gear's addendum, dedendum, root radius and thickness factor |
| `wasm-boundary#12` | partly | low | T04 | Chord tolerance is clamped and replaced without a note, and has no published bound |
| `wasm-boundary#4` | partly | low | T04 | The DXF's reference circles are the raw tooth's, not the gear the panel quotes |
| `wasm-boundary#7` | confirmed | low | T15 | The figures a fresh load case starts at are two copies of the same literals, and the tab defaults live in the boundary rather than the core |
| `wasm-boundary#8` | confirmed | low | T16 | check_wasm's coverage gate reads exports by grepping source for an exact attribute line |
| `web#10` | confirmed | low | T19 | Numbers are formatted in mixed locales: toLocaleString beside toFixed, inputs follow the browser, html lang stays 'en' |
| `web#11` | partly | low | T19 | English and unit literals outside the catalogue: 'Fine'/'Standard', 'Unnamed', 'mm/s', 'mm', '°', file-name fallbacks; the catalogue has duplicate unit keys |
| `web#12` | confirmed | low | T19 | The material provenance marker shows the basis's first English letter, so 'datasheet' and 'derived' both show 'd' |
| `web#13` | confirmed | low | T19 | editTrain reports a boundary failure as success, and relieveTrain/relieveCase fail silently |
| `web#14` | confirmed | low | T19 | The gear tab's adopt list runs a full train solve per train tab just to read member names |
| `web#15` | partly | low | T19 | Load-case edits are split: add_case and duty go through the core, removeCase and setRole mutate in TypeScript |
| `web#16` | confirmed | low | T14 | The wire uses mixed conventions for 'none' and for numbering |
| `web#18` | confirmed | low | T17 | Dead or stale code: Library.names, the data-theme CSS blocks, about 50 unused type imports, a stale duplicate doc comment |
| `web#19` | confirmed | low | T19 | Accessibility gaps: segmented controls and list selections announce no state, the viewport has no keyboard or touch zoom |
| `web#5` | confirmed | low | T19 | Train integer inputs (teeth, planet count, cutter teeth, actuations) accept fractions and negatives, which fail the whole train at the boundary |
| `web#6` | confirmed | low | T19 | Rule 1: the gear tab's mate is created with 43 teeth written in TypeScript |
| `web#7` | partly | low | T19 | Rule 1: diameters from radii, mm to µm, and fraction to percent are computed in TypeScript |
| `web#8` | partly | low | T19 | Engineering judgements repeated in TypeScript: locking, contact ratio below 1, idle, internal mesh, crossed, a weak basis |
| `ablate-constants-geometry#14` | confirmed | info | T07 | Right-angle closed-form branch in least_distance_lead_angle is continuous with the general solve and removable |
| `ablate-constants-rating#11` | partly | info | T15 | The flow solve uses three unrelated literal tolerances with three different bases |
| `ablate-constants-rating#14` | confirmed | info | T15 | The aspect-ratio solve is the only call site with a non-default Brent tolerance, and that tolerance makes no difference |
| `ablate-features#12` | confirmed | info | T18 | The gear-wasm crate description still says 'three functions'; there are 23 entry points |
| `ablate-features#14` | confirmed | info | T19 | Two note renderers: the Rust fill() that the strings laws test, and a TypeScript regex the user actually sees |
| `ablate-features#15` | partly | info | T04 | Feature ablation: the eccentric gear is the costliest feature per user (about 1,010 lines plus about 3,000 lines of dependent tests) and never reaches a train |
| `added#17` | confirmed | info | T18 | state.md's transverse-round residual calls its sign conservative, but the undercut shift it moves is under-read, and no output size is given |
| `added#21` | confirmed | info | T18 | The documented z=30 edge of the throw search is 1.49, not 1.4 |
| `added#29` | confirmed | info | T17 | strength.rs `finish` carries a dead `vertex` parameter discarded with `let _ = vertex;` |
| `added#3` | confirmed | info | T18 | ALPHA_MAX's doc misstates the tool's pressure-angle limit and inv(60°) |
| `added#34` | confirmed | info | T17 | ToothOutline::flank_curvature has no production caller |
| `added#35` | confirmed | info | T18 | LoadPoint's struct doc says only a ring has a flank limit; the code and its own comment say both do |
| `added#36` | confirmed | info | T08 | Material library header says fatigue allowables are published for 'the steels'; hardened 4340's is an estimate |
| `added#7` | confirmed | info | T17 | Ratio::cmp_checked has no production caller |
| `added2#0` | confirmed | info | T18 | PATH_SAMPLES and SEARCH_SAMPLES point readers to a convergence test in the wrong module |
| `added2#104` | confirmed | info | T15 | One diameter floor offset by two different epsilons within one function (1.000_001 and 1 + 1e-9), plus 1 + 1e-6 for the room floor |
| `added2#107` | partly | info | T16 | crossed.rs self-locking test accepts either the locked or the near-locked note, so it cannot catch the two branches swapped |
| `added2#114` | confirmed | info | T09 | summarise's per-z metrology loops: the over-pins 'around' sweep runs two full over_pins_at per start on a concentric gear whose every start is identical |
| `added2#25` | partly | info | T19 | Commanded-distance readout flags floating-point noise as interference |
| `added2#29` | confirmed | info | T16 | The note placeholder check takes the union of values over all firings, so one site that omits a value is masked by another that supplies it |
| `added2#34` | confirmed | info | T16 | The cutter-check grid is written twice: in gear-cli verify and in rack_simulation.rs |
| `added2#36` | confirmed | info | T18 | set_duty's doc says continuous hours come from 'the same numbers a fresh case starts with', but a fresh case has no hours |
| `added2#41` | confirmed | info | T18 | geartrain-refactor-plan.md points at a rationale anchor that no longer exists |
| `added2#44` | confirmed | info | T18 | More stale counts and comments in the tool configs |
| `added2#48` | confirmed | info | T18 | strings_en.toml's [error] header says the Display impls are gone; they are not |
| `added2#51` | confirmed | info | T18 | Shape::ports' doc says a hula's wobble body has four gears on it; it has two |
| `added2#53` | confirmed | info | T18 | The English catalogue's screw_axes_are_parallel still names the retired 'worm stage' |
| `added2#57` | confirmed | info | T18 | CLAUDE.md's check table gives validate_dxf.py without the arguments it needs |
| `added2#63` | confirmed | info | T18 | pin_diameter_range's comment says sixty halvings of a few-module bracket; the code does 64 halvings of a bracket that starts at r_b and doubles |
| `added2#71` | confirmed | info | T18 | The layout table exists three times, and state.md's copy carries its own history |
| `added2#76` | confirmed | info | T08 | contact_stress's lengthwise_curvature argument is zero in every production call |
| `added2#85` | confirmed | info | T09 | The JGMA transcription has been fully re-verified against the source page: all 618 values match |
| `added2#86` | confirmed | info | T16 | A third wall-clock assert in outline.rs, redundant with the vertex count beside it |
| `added2#93` | confirmed | info | T14 | The placeholder grammar is written three times in gear-io strings.rs |
| `added2#99` | confirmed | info | T18 | a_path_holds_at_rest_where_no_mesh_does keeps change history in its docstring |
| `added3#10` | unverified | info | T18 | .cargo/config.toml: jobs = -1 is documented as 'all available cores' but means all but one |
| `gear-cli#15` | partly | info | T20 | No CLI command prints metrology or JGMA tolerances; they are in no golden record except one wasm probe |
| `gear-outline#13` | confirmed | info | T04 | Polyline flanks cost 1.2-1.5x optimal chords, and arcs (bulges) would cut them 3-10x |
| `kinematics-flow#10` | refuted | info | — | Train::arranged_as silently makes output = input when input == fixed |
| `lens-architecture#14` | partly | info | T14 | Part/slot lookups rebuild every part; a duplicated `_of(parts)` API exists to avoid it; `member()` panics |
| `lens-continuity#7` | partly | info | T09 | The pin-diameter range is bisected on a verdict although each end has a closed form |
| `lens-continuity#9` | partly | info | T15 | Small hard-coded brackets and nudges in the train solves |
| `lens-docs-clarity#15` | partly | info | T18 | Doc comments much longer than their functions: 187 of 776 fn docs exceed the body, 67 by 3x |
| `lens-docs-clarity#16` | partly | info | T18 | Verdict on 'the ratio is not a target to reduce': the argument does not survive; replace with a one-home rule |
| `lens-errors-policy#17` | partly | info | T15 | `z < 4.0` in fitted_teeth is not NaN-safe before an `as u32` cast |
| `lens-feature-gaps#10` | partly | info | T19 | Ratio-per-tooth sensitivity is computed on every solve and never shown |
| `lens-feature-gaps#11` | confirmed | info | T21 | Bevel gears do not fit the screw model and break the 'sign is the mesh kind's' rule; face, hypoid and non-circular gears are XL |
| `lens-feature-gaps#17` | partly | info | T21 | No train synthesis: tooth counts for a target ratio (and a train-level multi-objective search) are absent |
| `lens-feature-gaps#7` | confirmed | info | T21 | Lubrication regime (film thickness, λ) fits the Hertz/contact data but only as an explicit fitted feature |
| `lens-feature-gaps#9` | partly | info | T21 | No dynamic disclosure; a mesh-resonance ratio is derivable without K_v |
| `lens-magic-numbers#12` | partly | info | T15 | The N·m to N·mm factor is written inline as 1000 or 2000 at four sites |
| `lens-magic-numbers#15` | partly | info | T15 | Shape bracket fallbacks: ±5 modules of shift when the admissible range is open, and 89° as the top of the helix bracket |
| `lens-tests-geometry#12` | partly | info | T16 | A second wall-clock test outside train/ |
| `lens-tests-train#12` | partly | info | T16 | The slowest test in the suite, the_search_is_converged_not_budgeted, takes 9.7 s |
| `lens-unification#8` | partly | info | T07 | The rating's line/point seams are a discontinuity at Σ = 0 (peak pressure 3-6 %, pitch pressure 0.5-3 % with friction) |
| `mesh-contact#9` | partly | info | T01 | ContactPath accepts a member whose tip is inside its own base circle |
| `primitives#12` | partly | info | T15 | Brent is used with residuals that jump to a sentinel value, collapsing it to bisection |
| `primitives#16` | confirmed | info | T18 | inv_inverse doc omits the v > ~1e12 refusal and has a malformed link line |
| `primitives#7` | partly | info | T15 | inv and inv_inverse lose relative accuracy below ~0.3 rad through cancellation; a series fixes both and makes the inverse 0–2 Newton steps |
| `shape-b#13` | partly | info | T01 | A mesh with no distance silently borrows distance 0's axial clearance and tolerances |
| `tooth-form#11` | partly | info | T03 | An external gear cut by a pinion cutter is fully built but unreachable |
| `tooth-form#12` | partly | info | T03 | The profile jumps at the severing threshold (physical, but unflagged as a discontinuity) |
| `tooth-form#8` | refuted | info | — | Involute and trochoid are sampled uniformly in their parameters, not by arc length, although the involute's arc length is closed form |
| `tooth-form#9` | partly | info | T03 | verify.rs carries unexplained literals and dead bindings |
| `train-mod-a#16` | refuted | info | — | Backlash::banded's nominal-as-candidate reasoning is sound, but the per-member band's own minimum can sit below the nominal on both sides without saying so |
| `train-mod-b#11` | partly | info | T11 | per_tooth rebuilds the whole kinematic system once per member per path on every solve, for a figure only the harness reads |
| `train-mod-b#12` | partly | info | T13 | Train::arranged sends every non-load entry, Free included, to the output |
| `wasm-boundary#13` | confirmed | info | T15 | centre_profile branches on angular_shift == 0.0 exactly |
| `wasm-boundary#14` | confirmed | info | T01 | points_per_tooth is an unbounded usize from JavaScript |
| `wasm-boundary#9` | partly | info | T14 | The 23 entry points could be about 12, and 23 near-identical wrappers could be one generic helper |
| `web#22` | partly | info | T19 | Minor duplicate or global state: the popup width is written in TS and CSS; import and adopt errors outlive their tab; defaults() is cloned for every preset label |
