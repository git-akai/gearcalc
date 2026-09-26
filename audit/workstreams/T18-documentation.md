## T18 — Documentation and comments: one home per fact, in the present tense

**Why.** The prose is not wrong because it was carelessly written. It is wrong because each fact is written in several places, in the vocabulary of the design it described, and only one copy moves when the code does. Three documents give three different lists of what CI runs. README says `nix flake check` is "everything CI checks", and CI runs ten more steps [lens-docs-accuracy-1#6]. A dated `Y_S`, a retired "worm stage" and a "cut by the loss" torque each survive in two to five copies after the code changed. 202 of 13,861 production comment lines carry a history marker (about 130 of them genuine) [lens-docs-clarity#3]. "stage" still occurs 978 times in code after stages were retired [lens-docs-clarity#4]. Numbers typed into prose (payload 1,209,487 B against 1,542,209 B shipped; "Two changes so far" above 25 entries) are in no gate. The fix is structural first: one home per fact, a vocabulary, and one script for CI. Then the instances.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T18.1 One home per fact: the map's head, README, state.md | lens-docs-clarity#2, lens-docs-clarity#16, lens-docs-clarity#10, tools-ci#18, lens-docs-accuracy-1#14, added2#71, lens-docs-accuracy-1#13, lens-docs-clarity#0, lens-docs-accuracy-1#17, added2#56, lens-docs-accuracy-1#8 | low | M | — |
| T18.2 One CI list, run by one script | lens-docs-accuracy-1#6, lens-docs-clarity#1, tools-ci#13, added2#57, tools-ci#17, added2#44, added3#10 (unverified) | medium | M | — |
| T18.3 A glossary, and the retired vocabulary out | lens-docs-clarity#4, added#58, added2#53, added2#66, lens-docs-accuracy-2#14, lens-docs-clarity#6, train-mod-a#13, shape-a#16 | medium | M | — |
| T18.4 The train/ map back to three columns | lens-docs-clarity#9 | medium | S | T18.3 |
| T18.5 Say what `GearCase::torque` is | gear-cli#1, lens-standards#7, added2#33, added2#74 | medium | S | T10.15 |
| T18.6 The flow's branch rule, stated once | lens-performance#11, kinematics-flow#6, added2#96, added2#9 | medium | S | T11.3 |
| T18.7 Rating prose names the model that runs | lens-docs-accuracy-1#1, added#23, added2#67, strength#10, added#35, added2#75, lens-docs-accuracy-2#5, ablate-constants-rating#15, added2#111 | medium | S | — |
| T18.8 `bending-check.html`: generated or cut | strength#9 | medium | M | T18.7 |
| T18.9 Clearance, tip-sizing and thickness prose | added2#14, added2#10, added#67, added#9, lens-docs-accuracy-1#4, lens-architecture#17 | medium | S | T10.4, T10.7 |
| T18.10 Contact and crossed-mesh prose | lens-docs-accuracy-1#2, added2#26, mesh-contact#11, added2#46, added2#0, added#52, crossed-worm#14, lens-magic-numbers#16 | medium | S | T06.7, T06.9 |
| T18.11 The file format stated as a schema, not a change log | lens-docs-clarity#5, ablate-features#10, added2#60, lens-docs-accuracy-1#16, gear-io#16 | medium | S | with T14.4 |
| T18.12 Boundary docs, and build figures from the build | wasm-boundary#10, added2#89, ablate-features#12, added2#91, lens-performance#12, added2#95 | low | S | T18.2 |
| T18.13 The corrections log: readable labels, right anchors, complete rows | lens-docs-clarity#7, lens-docs-accuracy-2#15, added2#65, added2#40, added2#50, added2#20, lens-docs-accuracy-2#10 | low | M | — |
| T18.14 History out of living text | lens-docs-clarity#3, lens-docs-clarity#11, lens-docs-clarity#13, graph-ops#15, crossed-worm#15, lens-docs-clarity#14, added2#99 | low | L | T18.3, T18.13 |
| T18.15 No pointer to a document a reader cannot open | added#51, added2#84, added2#61, added#38 | low | S | — |
| T18.16 Delete the root working notes | lens-docs-clarity#12, lens-docs-accuracy-1#7, added2#41 | low | S | — |
| T18.17 Code doc comments link to the derivation, not restate it | lens-docs-clarity#15 | info | M | T18.1 |
| T18.18 The solve inventory and the solver docs | lens-docs-accuracy-2#1, lens-docs-accuracy-1#9, primitives#6, lens-continuity#5, added2#102, added3#22 (unverified), added2#63, primitives#16, added#3 | low | M | T15.5, T15.13 |
| T18.19 Load cases and ports, as the code treats them | train-mod-b#8, added2#6, graph-ops#10, added2#51 | low | S | — |
| T18.20 The canaries and the corpus paragraph | lens-docs-accuracy-1#5, added2#32, lens-docs-accuracy-1#11, added2#105, lens-docs-accuracy-1#3 | low | S | — |
| T18.21 Search and chooser prose | auto-search#9, auto-search#11, added#40, ablate-features#2, added2#88, added#43, ablate-features#8, ablate-features#13, lens-docs-accuracy-1#12, lens-docs-accuracy-2#6 | low | M | T12.5 |
| T18.22 "Looks wrong and is not", each caution with its test | added#16, added2#58, lens-docs-accuracy-1#10, lens-docs-clarity#8, added#0 | low | S | with T03.11 |
| T18.23 Tooth, ring, metrology and outline prose | metrology#8, added2#62, metrology#9, metrology#10, lens-docs-accuracy-2#9, added2#64, lens-docs-accuracy-2#7, added#14, gear-outline#11, added#20, added#21, lens-docs-accuracy-2#2, lens-errors-policy#18, added#17, added2#68, lens-docs-accuracy-2#3 | low | M | T03.3, T04.7, T05.6 |
| T18.24 One term per concept per language | gear-io#11, gear-io#10, added2#48, shape-b#12 | low | M | T18.3, T10.19 |
| T18.25 Test prose and the coverage claims about tests | lens-tests-geometry#9, lens-tests-train#10, lens-docs-accuracy-1#18 | low | S | — |
| T18.26 Remaining stale pointers | lens-docs-accuracy-2#11, added3#16 (unverified), added2#36, graph-ops#9 | low | S | — |
| T18.27 The worm-bending rationale, on its one real reason | crossed-worm#7 | low | S | — |

### T18.1 One home per fact: the map's head, README, state.md

**Change.**
- Replace CLAUDE.md:7-33 (the census paragraph and its dated series) with two sentences. First: "Prose and code are about 1 to 1 (`tools/line_census.py` takes the figure); that is not a target either way." Second, the rule that replaces "not a target to reduce": **one home per fact**. Derivations live in reference.md, decisions in rationale.md, known biases in state.md, faults in corrections.md, history in git; everything else links. Do not gate the census in `check_figures`: it moves on nearly every commit [tools-ci#18 verdict].
- README: delete the Layout section, "Notes for anyone changing the geometry" (its four cautions live in CLAUDE.md, T18.22), and README.md:92-96's claim that main.rs lists every command. Replace that claim with "`gear-cli help` lists every subcommand"; cut main.rs:9-13 to one line. Trim "Verification tooling" to exactly the six by-hand scripts, adding the missing `breakaway.py`, and keep its prose on what `worm_flank_curvature` and `crossed_path` answer. README.md:119 becomes "the material box shows each value's basis initial, estimates dimmed". README.md:106 adds "and add it to `LANGUAGES` in `crates/gear-io/src/strings.rs`".
- state.md: delete the Layout section (nothing links `#layout`). That also removes the self-referential opening at :259-262 and the false "`Gear` … the only place a gear is drawn" [lens-docs-accuracy-1#13]. A ring is drawn by `Ring::profile` and `Ring::outline`. Unifying the two drawers is T03.15/T05.17's.
- Keep the "proximity is not a mechanism" lesson once, in corrections.md.

**Proof.** Add `tools/check_restatement.py`, wired into `check_doc_links.py`'s CI step. It fails if any run of 12 consecutive words appears in two of README.md, CLAUDE.md and docs/*.md (docs/history and corrections.md excluded). It fails today on README.md:51 against state.md:269, which are identical, and on reference.md:1177 against rationale.md:1296. `check_doc_links.py` stays green.

### T18.2 One CI list, run by one script

**Change.**
- Add `tools/check_all.sh`, which runs every CI step in CI's order. It includes the `validate_dxf.py` invocation with its export and five expected radii (ci.yml:111-117), writing to the cache dir rather than /tmp, and uses `nix develop --command` as CI does. ci.yml calls it.
- CLAUDE.md's checks table says "`tools/check_all.sh` runs every row marked yes". README.md:73 says "`nix flake check` — build, clippy, fmt, tests; `tools/check_all.sh` is all of CI".
- In state.md, replace "Running it" (:38-58, "run all four" and a nine-tool list without `check_units`, `check_wasm` or `validate_dxf`) with a pointer to the table.
- Correct the configuration comments:
  - ci.yml:141, "four consistency checks": there are seven;
  - `check_strings.py:11`, "Run by hand": CI runs it;
  - `check_figures._bin`, "CI builds debug": CI builds release;
  - the flake data filter becomes `.*/data/.*`, so its comment is true;
  - drop the dated count in `check_doc_links.py:4`;
  - delete `[term] verbose` and `color = "auto"` from `.cargo/config.toml` (both defaults);
  - say `jobs = -1` is "all cores but one" (unverified, added3#10).

**Proof.** A law in `check_doc_links.py`: every command in a CLAUDE.md table row marked "yes" appears in `check_all.sh`, and ci.yml's test job invokes nothing else. It fails today: the table gives `validate_dxf.py` with no arguments, which is not runnable.

### T18.3 A glossary, and the retired vocabulary out

**Change.**
- Add a Terms block at the top of reference.md, one line each: train, shape (the graph), axis, body (port, slot), member, mesh, distance, coupling, part (the solve's derived unit), preset (a menu entry that lays pieces in), arrangement (the list a preset builds), family, hold, load case, load, path. Define preset and part without "what a stage was" (reference.md:2198, graph.rs:3).
- Narrow rationale.md:159-170's naming rule to what it argues: the hula is "hula", never "eccentric drive". Drop the "worm drive" ban. It is broken in about ten places, including the UI string `train_size_as_worm` [lens-docs-accuracy-2#14 verdict].
- In all five catalogues, reword `error.screw_axes_are_parallel` (e.g. "a crossed mesh needs axes at an angle; parallel axes are a spur or helical mesh") and the "crossed-axis stage" in `low_efficiency`. The Display copy goes with T02.1.
- Rewrite the two false present-tense docs: `MeshReport` (mod.rs:194-197) becomes "every mesh of every part is one of these, by the graph's mesh index", and `ContactPatch::line`. Drop the clause at mod.rs:467 about stages, keeping "asked here so no caller has to remember to".
- Delete shape.rs:1414's reference to `planetary::solve`, which does not exist.

**Proof.** A dead-vocabulary check in `check_strings.py`, also run over `crates/*/src` production code. It fails on `\bstage (type|kind)s?\b`, `worm stage`, `spur stage` and `\bcards?\b` outside tests, `gear-io/src/train.rs`'s converter and corrections.md. It fails today on strings_en.toml:197 and :142 and on mod.rs:1740-1743. `check_doc_links` must pass after any heading rename.

### T18.4 The train/ map back to three columns

**Change.** CLAUDE.md's train/ table becomes the gear-core table's shape: *File | Answers | Does not know*, about 40 words a cell (today 139-343 words, and no "does not know" column). Deliberate absences fill the third column: `wiring.rs` no geometry, `flow.rs` no geometry, `preview.rs` decides nothing, `arrangements.rs` nothing about solving. Method inventories move to each module's `//!` header. Write the rows in T18.3's terms.

**Proof.** Review against the gear-core table. No word-count gate: that would be a threshold, not a law [lens-docs-clarity#9 verdict].

### T18.5 Say what `GearCase::torque` is

**Change.** The doc (mod.rs:1331-1336) and reference.md:2925-2928 say the far member's torque is "cut by the loss … nought for a locked mesh". The code refers the driver's torque by the ratio with no loss. Write what it is: "the driver's torque referred by the tooth ratio, before the mesh's loss — the force the teeth press with; what the body delivers is `on_body`". On a line contact this is exact, since both flanks share one normal force. T10.15 makes a point mesh's driven side the delivered torque (η·i·T₁). This doc then says so for point contacts and corrects shape.rs's "reference cylinder" comment [added2#74]. Label the CLI and panel figure "tooth-load torque" and print `on_body` beside it. Do not add a state.md bias: T₂/r_b2 = T₁/r_b1 is the true normal load [added2#33 verdict].

**Proof.** A law on every line-contact mesh of every preset: driven `torque · z_a/z_b` equals the driver's torque exactly, and `on_body` equals `torque · η` when the body has no other mesh. On 17/43 at 2 N·m today this gives 5.058824 and 4.975099. `gear-cli train mixed`'s worm wheel (199.0040 N·m against a shaft reacting 122.9936) changes under T10.15, not here.

### T18.6 The flow's branch rule, stated once

**Change.** After T11.3 implements "reject a branch in which no body absorbs power beyond `ZERO`", state that rule once in reference.md:1803-1830, next to the closed form's two conditions, with its physical argument. Point flow.rs's comments (:307-309, :341-347) to it. Delete "the first consistent one is the answer" and "a second genuinely different one would be a finding": a second branch occurs in about 30 % of Wolfrom and hula cases [kinematics-flow#6]. Do not document "every derived load absorbs": a Wolfrom with its carrier derived rightly delivers +22.05 W.

**Proof.** T11.3's law. The doc must name the Wolfrom 18/60/61 counter-branch: (B,F), efficiency 0, ring2 +265.84 W, rejected.

### T18.7 Rating prose names the model that runs

**Change.** Every rating calls `bending_factor(RootStressModel::DolanBroghamer)` (strength.rs:1512).
- state.md:296 and :617, and rationale.md:607: "the notch factor, Dolan–Broghamer's K_f, and Y_B are applied; Y_S is kept for the ISO comparison set".
- strength.rs:1365-1366, :1502 ("largest `bending_factor · share`") and :1015.
- The rest of strength#10's list:
  - strength.rs:106-130: show the coherent-set ρ (0.671), or point at state.md, and drop "not a settled model";
  - :650: parabola, not "the 30° tangent";
  - :819-820 against :805;
  - :1130: 3.48887, not 3.4886;
  - rationale.md:757-760 together with strength.rs:1588.
- `LoadPoint`'s struct doc (1434-1436): one flank bracket for both members, only the direction of travel differs. bending.rs:983 gets a real assertion.
- rationale.md:1267-1275 and reference.md:2869: **both** contact ratings are off by default, each with its one reason, as mod.rs:2489-2507 and state.md:534 already say.
- rationale.md:559: parabola/tangent Y_F 1.006 to 1.312, mean 1.055, over 1508 designs, tagged `<!-- figures: gear-cli matrix -->`.
- `SHARING_SAMPLES`: merge the two stacked doc blocks into one that lists the four seeds the code uses, without "provably". Land it with T06.6 if that replaces the sweep.

**Proof.**
- `check_figures.py` holds the tagged matrix block.
- A new assertion in bending.rs: `bending_section` returns `None` when ε puts the HPSTC past the fillet junction. It must fail with the bracket check in `LoadPoint::at` deleted, which today changes no test.
- `grep -n 'Y_S' docs/state.md docs/rationale.md` leaves only ISO-comparison uses.

### T18.8 `bending-check.html`: generated or cut

**Change.** Cut the page to three things: the verbatim `gear-cli bending` block; the rack-limit table, which `gear-cli bending` now prints; and pointers to state.md#one-bending-model and `gear-cli matrix`. Its hand-written prose (1046dce) contradicts the code: "undefined where the section leaves the fillet", "now unrepresentable", "report Y_F and Y_F·Y_S". Fix check_figures.py:296's docstring and README's description of the page.

**Proof.** `check_figures.py` finds every number on the page in a tagged generator output. It fails today on the rack-limit table.

### T18.9 Clearance, tip-sizing and thickness prose

**Change.**
- reference.md:420-426 claims a set's clearance "is always given … pinned back by relief". Describe the rule the test `an_epicyclic_kinds_clearance_is_pinned_back_whatever_was_touched` holds instead: at most one of distance and clearance is automatic, the one just touched is spared, and a hula's clearance is pinned. Add what an automatic clearance means on a two-mesh distance once T10.7 lands.
- mod.rs:7984-7990: the tips hold the running distance open, so on an internal mesh the clearance cannot exceed what they leave. Planocentric: typed 0.03-0.1 all read 0.02. It is a ceiling, not a floor.
- reference.md:2398-2400: the shipped planocentric *is* sized. At 30/33 it opens to 1.734 mm with the ring at +0.364. Tag it `<!-- figures: gear-cli graph -->`.
- auto.rs:1316-1322 (`tips_are_clear`): asked where the mesh runs (`self.mesh.a_w`), which is the tighter distance. Drop the pointer to a state.md bias that does not exist.
- state.md:449-452: delete the hula "closed-form crank" argument. The distance is a bracket walk plus Brent, and a ring is exempt from the tip-width bound on every arrangement.
- rationale.md's "k₂ = 2 − k₁ … unwritable": rewrite after T10.4 makes thickness one freedom per mesh group. Do not bless the current per-mesh rule [lens-architecture#17 verdict].

**Proof.** The planocentric figure is held by `check_figures`. The set-clearance text is checked against the named test.

### T18.10 Contact and crossed-mesh prose

**Change.**
- state.md: a crossed mesh *does* report `contact_ratio` (along its line of action; `gear-cli crossed 17 23 90` gives ε = 1.758113579), `coprime` and a directional efficiency. What it lacks is the operating pressure angle and the transverse/overlap split.
- The out-of-band sharing figure is quoted three different ways (24 %/15 %, "a quarter", 12-24 %), and a sweep gives +33 %/−74 %. T06.7 makes the note carry this mesh's own change. Then delete the fixed figure from the `Bending::note` doc, reference.md:2847, state.md:730/1019 and all five catalogues, and say the measured range once in state.md.
- contact.rs (under T06.9): the efficiency is exact in its sliding but first order in μ. It understates η by about 0.03·μ² (1.1e-4 at μ = 0.06 on 17/43). Forward and backward agree only at first order.
- mesh-contact#11:
  - (a) "at the distance the mesh describes";
  - (b) "a share of the pairs engaged";
  - (c) "every mesh";
  - (d) delete the "milestone 7, step 5" paragraph;
  - (e) move the helper comment above `fn add`;
  - (f) cite Pedrero–Sánchez–Pleguezuelos for the 1/3 end share.
- Point `PATH_SAMPLES`/`SEARCH_SAMPLES` at `screw::tests::the_path_average_has_converged`, and make that test use the shared constant rather than a literal 2048.
- Rewrite `limited_by_face`'s doc to match its body: each face centred on its member's mid-plane; `None` means off at least one face, narrow or offset. Land it with T07.2/T07.4.
- In two comments, a contact point advances r_b cos β_b per radian, not r_b/cos β_b, and p_bn = 2π r_b cos β_b / z.
- The worm-wheel width: 2m√(q+1) is BS 721's, the 0.67 d₁ cap AGMA's, the length ČSN's. Fix crossed.rs:121-130, reference.md:1075-1076 and `train_note_wheel_width` ×5, as part of T07.9.

**Proof.**
- r_b cos β_b · 2π/z = π m_n cos α_n = `normal_base_pitch()` (test).
- The convergence test reads the constant.
- The sharing figure appears in exactly one document (T18.1's check).

### T18.11 The file format stated as a schema, not a change log

**Change.** Done with T14.4. `gear-io/src/train.rs`'s 291-line module doc lists 25 migrations under "Two changes so far"; every one describes a layout that `from_toml` refuses. Replace it with about 30 lines:
- inputs only;
- the current schema as an annotated TOML example that keeps each field's meaning (a load's `role`, a distance's `tip_clearance`);
- the refusal rule, "refused by name, except where absence has exactly one meaning";
- what `gear-cli convert` reads.

Update CLAUDE.md's "A load-case input" row, which points at the change log. gear-io#16's remaining items:
- dxf.rs:93: "to 1e-12 mm", or write `{value:?}`;
- rename `the_sweep_fires_most_of_the_catalogue`;
- make `observed_values` a `BTreeSet`;
- add the supplied-but-unused value check beside the supplied-but-missing one. It fires today only on `clamp.ring_flank_ungenerated: tip`.

**Proof.** The TOML example in the doc is a doctest that `from_toml` parses. The new catalogue check fails on the one unused value until it is shown or listed.

### T18.12 Boundary docs, and build figures from the build

**Change.**
- Move `default_materials`' doc (lib.rs:992-997) off `solve_ring`. The generated `gear_wasm.d.ts:273-285` gives `solve_ring` both texts, and `default_materials` has none; give it an `# Errors` section.
- Move the `languages` doc onto `languages_impl`.
- Move the round-trip doc (lib.rs:1800-1809) onto `a_geartrain_survives_export_and_import_as_the_same_answers`, with "every part's numbers".
- The Cargo description becomes "pure, JSON in and JSON out; `tools/wasm_boundary.json` lists them" (not "three functions"; there are 23). Delete "the third of the three entry points".
- The module doc says "A measurement that cannot be made returns a reason".
- Delete the payload table under `[profile.wasm]` and the figures in `build_wasm.sh:17-19`. `check_wasm.sh` records payload bytes, raw and gzipped, as T11.11 proposes. Never record times.
- CLAUDE.md rule 3's "a full train solve is microseconds" becomes the measured statement: 0.8-5.8 ms per preset in wasm, 51 ms for a six-preset train.

**Proof.** `check_wasm.sh` fails if any exported function in the generated `.d.ts` lacks a doc comment. It fails today on `default_materials`. The payload bytes are in its recorded output.

### T18.13 The corrections log: readable labels, right anchors, complete rows

**Change.**
- Relabel all 95 "Where" cells that carry a superseded section number (`4.x`, `6`, `8.0`, `11.4`, `11.5`) with the heading's name, as the other rows do. Re-anchor each by content while doing it: the bending rows 356-360, 366 and 439-441 to `reference.md#bending`, and row 361 to `#efficiency-parallel-axes`.
- Row 608: `../README.md`.
- Add a row after 648 recording that an orbiting body is a port again, because a case now declares what it reacts (cite reference.md#load-cases). Row 648's "no port now" stops reading as current, and shape.rs:4166-4172's parenthetical becomes a link to the new row.
- Row 559: the ring's space cap coincides with the rack's thickness bound, but the tip reaching the base or pointed circle, small-z space closure and the cutter's tip corner are the ring's own limits (z = 30 is clamp-free only on [0.10, 0.68]).
- "`Note::is(key)`, checked by the compiler" becomes "by key constant, never by text".
- "Phase 3b" at :668 becomes "retiring the lone stage".

**Proof.** `check_doc_links.py` fails on link text that is only a section number, and resolves anchorless `](x.md)` links relative to the linking file. The anchorless half is tools-ci#5's, with lens-docs-accuracy-1#15 as the same instance. Both fail today.

### T18.14 History out of living text

**Change.**
- The rule, in CLAUDE.md: no self-reference ("this paragraph used to say", "an earlier draft") and no retired vocabulary in code comments, reference.md, rationale.md decisions, state.md or CLAUDE.md. A rejected alternative is kept, but in the present tense ("a fixed 1 module would …"). A fault worth remembering gets one corrections.md row (T18.13), which code may link.
- Sweep the roughly 130 genuine hits: train/mod.rs (30), strength.rs (23), auto.rs (21) and the following.
  - reference.md's 14 passages (e.g. :1362, :2507-2513). Check first that corrections.md has a row for the ones at :651 and :801.
  - conditions.rs's "it used to live on the stage", the handoff pointer and `MotionReport`'s "until it was asked"; wiring.rs:16; kinematics.rs:159.
  - screw.rs:6-7, :549-551, :573-575, :622-630 and :1257. Rewrite guards that only history states (screw.rs:2204-2206) as present-tense guards.
  - The "milestone N"/"Phase N" sites: README.md:88, shaper.rs:6, main.rs:1283, crossed.rs:43. Delete the promise at contact.rs:633-635.
- Cut crossed.rs's 69-line header to a link to reference.md#crossed-axes, which already holds the derivation, keeping only the 0.21 % divergence if reference.md lacks it. Move `proportions` to screw.rs, not arrangements.rs (T14.10 retires the file).
- `a_path_holds_at_rest_where_no_mesh_does`: state the law only. Its fixture is lens-tests-train#2's.

**Proof.** A check over the listed files for `used to (say|be|live)|earlier (draft|version) of this|this (paragraph|table|comment) (said|named)|milestone \d|Phase \d` returns nothing. The narrow regex is deliberate: "no longer" and "now" misfire (state.md:20-22). The rest is reviewed by hand, file by file.

### T18.15 No pointer to a document a reader cannot open

**Change.**
- jgma.rs:165, :368 and the test `available_classes_track_the_module` cite "the specification's default of grade 3". Restate: a fixed default would name a class with no data (grade 3 exists only on the fine scale, below module 1.6).
- jgma_116_02.csv:10's "DESIGN.md 4.6.1" points at rationale.md#the-tolerance-table-has-two-grade-scales-not-one.
- auto.rs:2153 reads "the thresholds in this module's doc table". Drop metrology.rs:7's sentence, whose case no document holds.
- crossed.rs:80's "the appendix" points at the rationale.md section holding the 4 mm/40 mm measurement. bending.rs:372's "DESIGN's appendix" points at reference.md's virtual-spur statement.
- "E2" in gear.rs:675 and :1492 (exported to `Variation.ts`) becomes "the λ = 0 gear's". rationale.md:1108 becomes "the plain radial oscillation (λ = 0)". Regenerate the bindings.

**Proof.** `check_doc_links.py` also scans `crates/**/data/*` and fails on `DESIGN(\.md|'s)`, `the appendix`, `the specification`, `the design document` and `\bE[23]\b` outside docs/history and corrections.md. It fails today on all six sites.

### T18.16 Delete the root working notes

**Change.** Delete `geartrain-refactor-plan.md` and `geartrain-refactor-handoff.md` (1,420 lines). By their own line 11 they are "deleted once built". Their baseline table describes a tree that no longer exists, and their residuals already have homes: reference.md:2231/2500, corrections.md:405 and state.md:636/756-763. Before deleting, rewrite `gear-cli/src/kinematics.rs:302-309` in present terms: an unclosed part costs the whole train its answer, and golden kinematics.txt records it. Remove conditions.rs's prose pointer to the handoff. Keep docs/history/design-record.md. Its header argues for keeping it, and deleting it first needs its arguments (the JGMA two scales, the material survey, the crossed path, the λ minimax proof) checked against the four documents.

**Proof.** `git grep -n 'geartrain-refactor'` returns nothing. `check_doc_links.py` passes.

### T18.17 Code doc comments link to the derivation, not restate it

**Change.** The rule is one home per argument, not a length ratio. Where a doc block restates reference.md or rationale.md, it becomes a one-line why plus a link. First targets:
- strength.rs:1708-1740 (`ContactStress::governing`, identical to rationale.md:1281-1305, down to ρ₁ = ρ₂ = 10 against 5.5/55);
- the "ρ₁ + ρ₂ is constant" step at strength.rs:1724 and :1812;
- mesh.rs:364-400 (`loaded_flank_phase`, which restates reference.md:315-318).

Keep "why this is a named function" paragraphs and the wasm entry-point contracts. Trim only the history in those.

**Proof.** T18.1's `check_restatement.py`, extended to `///` and `//!` blocks against docs/*.md. It fails today on strength.rs:1714-1726.

### T18.18 The solve inventory and the solver docs

**Change.**
- Replace rationale.md:412-427 ("Ten scalar solves … none with a tuning parameter … everything else algebraic — except one search") with one row per purpose: what is solved, the method, the bracket's source, why there is no closed form, and the gate. Name the quadratures and searches in the same table. Give no count, and add no call-site-counting gate [primitives#6 verdict].
- Row 2 is Brent in the roll, twice. With T15.13, the pointed roll becomes the closed form u = tan(inv⁻¹ψ_b) and the row shrinks.
- reference.md:47-48, CLAUDE.md:62 and solve.rs:3: "every root closes in `solve.rs`". Once T15.5 routes the hand-grown brackets and the two predicate bisections (metrology.rs:200, gear.rs:1119) through `grow_bracket`/`bisect_predicate`, the stronger claim becomes true and the text says so.
- involute.rs:28-29: there is no 60° cap (the bound is `strictly(0, 90)`), and inv 60° = 0.685. `ALPHA_MAX` only keeps tan off its pole. The test comment at :83 goes too.
- `inv_inverse`'s doc: `None` for v < 0, for v ≥ inv(`ALPHA_MAX`) ≈ 9.998e11 and for non-finite v. Separate the two run-together links, and cite the current callers.
- The divergence sentence: the bare seeded Newton converges to 73.2°, not 60°.
- `pin_diameter_range`: "64 halvings of [0, 2^k r_b]", with the two 64s named. T09.6 may replace the loop.

**Proof.** Extend the involute round trip from 85° to 89°. Assert `inv_inverse(1e12)` is `None`. Each table row's method matches its call site, reviewed.

### T18.19 Load cases and ports, as the code treats them

**Change.** The code reacts only declared `Reacted` ports (mod.rs:4360-4364).
- In `LoadCase::loads` (mod.rs:2819-2826), write "a port the case does not mention is free; a reaction is declared". Coordinate `LoadRole` (2839-2841) with T10.20, which rewrites it.
- Delete reference.md:2747-2753 from "The two ends it does not load are **reacted**" through "says so by loading it".
- `Ports::ports` (conditions.rs:52-60): "every body not replicated, in body order — a hula's crank, grounded gear, output and wobble body".
- shape.rs:4166-4172: the wobble body carries two gears, not four.

**Proof.** A law over every preset: a case with only the headline load leaves every other open port `Free`. It passes today, and it pins the behaviour the docs now state.

### T18.20 The canaries and the corpus paragraph

**Change.**
- CLAUDE.md:186-190: "leaves every law in the suite silent — K_f trips only the hula test that copies two σ_F figures from the corpus — and the corpus catches every one". Keep one current example with its command, and leave counts to docs/history/audit.md.
- state.md: the worm canary moved **five** times. :840 becomes a pointer to #the-canaries, not the superseded 74.3/63.8. "Load cases moved neither canary" is restricted to `wormstage`: `strength_report` never builds a `Train`.
- state.md:791-794: "swept once over 1.1 million combinations (e725dd4); the test keeps a 179,280-point subsample on the closed form, which `the_per_mesh_flow_is_pennestri_on_a_simple_set` holds the flow to".

**Proof.** In a worktree, H 0.331 → 0.3315 fails exactly `train::shape::hula_recorded::the_harness_hula_is_what_the_corpus_recorded` (678/679), and `check_golden.sh` reports a diff.

### T18.21 Search and chooser prose

**Change.** After T12.5 deletes the unshipped search:
- Move a true doc onto `Search::maximise`: a (scan+1)^dof sweep of the caller's box, then a multi-start compass walk over 3^dof − 1 directions, not "coordinate descent".
- reference.md:781 and rationale.md:478 name `auto::Search` and the shape's `Coordinate` `place`, not `auto::Freedoms`. The same goes for `shifts_report`, contact.rs:495 and shape.rs:1790.
- Delete the orphaned `Pinned` paragraph at auto.rs:764-770. `undercut_bound`'s table reads "given (`Cut::Pinned`)", with no `Decided::Given`. Fix auto.rs:1032's link.
- auto.rs:1639-1656: pair worst 3.26e-6 (9/20); sets converge. Point to corrections.md once, and have the gate print its worst.
- `SLACK`: keep 1e-4 with the true ratios (about 1.4 orders below ±10 % μ, about equal to F53's 1.2e-4 gap), or tighten under T12.
- Trim `teeth_clear`/`as_asked` to "the harness's filter". Delete `effort()` and `backlash_by_drive`. Make `index` `pub(crate)` and drop `index_pub`. `distinct_teeth`/`distinct` and `cmp_checked` lose their phantom customers.
- state.md: one objective over every free shift, searched per component. Drop the 0.998/0.996 hula sentence, and move the timing story to corrections.md.
- Delete "every row of every table reports which bound stopped it": 17/43's optimum is interior.

**Proof.** `git grep -n 'Pinned::place\|shifts_for_efficiency\|auto::Freedoms'` returns nothing. `cargo doc` with broken intra-doc links denied (the rustdoc gate, tools-ci#9) passes on auto.rs and train/pair.rs.

### T18.22 "Looks wrong and is not", each caution with its test

**Change.** Land this with T03.11, which corrects the same shaper and fit-cap prose.
- CLAUDE.md:220-232 says "three in `tooth.rs`, one in `shaper.rs`", and names each test:
  - `legacy_clamp_still_shows_the_junction_step_it_was_kept_to_demonstrate`;
  - a test that fails under w_tip/(2 cos α), e.g. `gear::tests::the_root_leaves_the_fillet_without_a_kink`;
  - `radius_is_monotone_and_theta_stays_in_the_half_pitch`.
- Caution 1 is inverted. r_j = r_b·hypot(1, u_j) ≥ r_b, so on an undercut tooth the fillet is carried past the rack's tangent point to its crossing with the involute, *above* the base circle. Reword it and the six other copies. gear.rs:407 and reference.md:1697 argue from the re-entrant fillet.
- Caution 4: `ShaperCut` caps an oversized round to 95 % of the largest that fits, and reports it. Update state.md:783 and strings.rs:1609's `UNFIRED` evidence to that cap.
- shaper.rs:39-43: σ appears in two places, not in the rolling. Fix the dead `profile::Tooth::trochoid_at` link.

**Proof.** Each named test is checked to fail under its caution's mutation. Across 1,975 undercut teeth, r_j − r_b ∈ [5.9e-8, 0.337] mm, which is positive and supports the reworded caution 1.

### T18.23 Tooth, ring, metrology and outline prose

**Change.**
- reference.md:1418-1419: an external gear's space narrows *inward*, a ring's *outward* [metrology#8 = added2#62].
- metrology.rs:430-431 and :797-798: delete the `# Errors` paragraphs and link `MeasurementError`.
- `best_span_around` minimises the worst offset. k = zα/π + 0.5 is exact, not "empirical". The CSV's 14 non-R40 values are said to be rounded.
- The undercut reduction (reference.md:239, auto.rs:58) is x_min = h_w − z sin²α_t/(2 cos β), and the tooth-count rule is z ≥ 2 h_w cos β / sin²α_t. The 18/22 table is for spur.
- The ring generation limit and junction are written with `a_cut sin α_cut` (cos α_cut = a_ref cos α_t / a_cut), with r_b in place of r_bw. Share the edit with T05.16.
- After T03.3: ring.rs:394-398 and :1605-1608 say the ring's closing space and the rack's pointed tooth are one rule. reference.md's input table gains h_f + x_s ≤ π/(4 tan α_n).
- gear-outline#11:
  - move `displacement`'s and `tooth_outline`'s docs onto their items;
  - distinct teeth ⌊z/2⌋ + 1;
  - fix the dead `Tooth::profile`, tooth.rs:324, dxf.rs:35 and gear.rs:255 links;
  - put the 62.6 µm table in reference.md#angularly-varying-profile-shift with a figures tag.
- After T04.7: the floor and depth-cap comments say what bounds what. Delete "45 seconds".
- Throw-search edges: 1.25/1.49/2.11, or drop them.
- params.rs:8: "root_radius is in normal modules and applied at the transverse module; see state.md". No plane switch [lens-errors-policy#18 verdict].
- state.md's transverse-round entry: the undercut shift is under-read by 0.008/0.019/0.041 modules at β = 20/30/45°, on the non-conservative side. The ellipse fix is T03.14.
- After T05.6: the `smallest_tooth_count` doc gives 2(h_a − x)cos β/(1 − cos α_t).
- reference.md:150-151 names the guards inside the displayed range: α ≤ 0.5° is clamped with a note, and the shallow-cut dedendum threshold is deliberate.

**Proof.**
- A helical case in `minimum_shift_matches_the_closed_form_and_the_cutter_helps`, with α_t from atan(tan α_n / cos β). It checks the doc's formula, which fails today as the doc is written.
- A probe: distinct teeth is 12 at z = 23 and 5 at z = 9.

### T18.24 One term per concept per language

**Change.**
- pt: one variant, PT-BR (contato, reto, acionar, exato). "contacto" is valid PT-PT, not pre-reform. The undercut term goes to a native reviewer.
- zh-Hant: 接觸比 throughout, 內齒圈 for ring (`gear_kind_internal` → "內齒輪（內齒圈）"), and one of 負荷/載荷.
- Ground: Gestell / 機架 / estrutura fixa, never Masse / 地 / Terra.
- pt: anular for ring, correção de perfil for shift, one of torque/binário.
- Reword strings_en.toml's `[error]` header: the catalogue is what the browser reads, and gear-core's Display impls stay until T02.1 removes them.
- shape-b#12: `equal_spacing`'s doc covers stepped planets. `planets_share_load_equally` is raised once per carrier, keyed on `carried_by`. The generic wording of the spacing key is T10.19's.

**Proof.** Add a per-language forbidden-variant list to `check_strings.py`, e.g. zh-Hant 重疊比 and 重合度, and pt contacto, rectos, accionar and exacta. It fails today. T19.11, the zh-Hant Simplified-character fixes [gear-io#9, added2#27, added2#28], can use the same list. A law: MeshedPlanets raises the load-share note once. Today it raises it twice.

### T18.25 Test prose and the coverage claims about tests

**Change.**
- regression.rs:6 says "seven cases".
- Move geometry_laws.rs:677-699 onto `only_an_undercut_tooth_can_be_severed`. The scan wording becomes "a dense scan of the interval the solve brackets".
- CLAUDE.md:146: only `every_add_on_every_preset_solves` sweeps the presets. `every_add_undoes` and `a_refused_edit_changes_nothing` are hand lists. Coordinate with T13.1's walk.
- The untested-modules paragraph names `train/pair.rs` and what holds it. Measure this by swapping `undercut_bound`'s arms in a worktree, not by assertion, or drop the line when T14.10 retires the file.

**Proof.** The perturbation run is recorded in the paragraph's evidence, with its date.

### T18.26 Remaining stale pointers

**Change.**
- rationale.md:310-320: the degrees-to-radians site is the `ScrewParams` construction in shape.rs, not `PairStage::screw_at`.
- reference.md:3029: drop "loses its moisture states".
- reference.md:3075: drop the count of entry points that "compute nothing".
- TrainPanel.svelte:1032-1034 (unverified): "the library entry's, for the one state its condition names, or the override".
- `set_duty`'s doc: the 1000 h is its own seed. Add `Duty::continuous()` with a named, argued constant, and use it in the test at mod.rs:10582. This is shared with T13.6's note.
- `graph_of`: a stage that re-joins two earlier bodies in reverse order keeps the graph's order, so its conventional ends swap. Add that fixture to `fixtures()`.

**Proof.** Stages [1,2], planetary [3,4,5], then [5,1]: part 2 reads input 1, output 5. The test states it.

### T18.27 The worm-bending rationale, on its one real reason

**Change.** Rewrite the rationale for reporting no bending on a worm:
- Keep its one defensible reason: a point load on a wide tooth needs an effective-width convention.
- Drop "the tooth measured would not be the tooth loaded … throated". The crate's wheel is the involute helical tooth it generates.
- Correct "no published bending allowable exists". DIN 3996 and ISO/TS 14521 rate wheel root strength by a convention (b₂H, Y_F for a throated wheel), and this project declines it.

The optional ISO/TS 14521-style root-shear figure is a feature for T07.20.

**Proof.** Review against the cited standards' formula, τ_F = F_tm2/(b₂H m_x)·Y_ε Y_F Y_γ Y_K.
