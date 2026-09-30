# Stage 1 exit review

`audit-ablation` f079797..53b46e0: 116 commits, 89 of them touching code, tools or CI. Stage 0's
T13.1–T13.3, H1 and H2 (9b7c919..f079797) are counted where Phase 1 asks for them.

Run at 53b46e0, in scratch worktrees that are now removed:
- the full `cargo nextest run`: 785 pass, and the 2 skipped are the ignored timing canaries;
- the rustdoc gate, `check_all.sh --list`, check_doc_links, check_strings, check_units, and the
  self-tests of ci_steps and check_figures;
- check_golden against stub binaries;
- 8 mutants planted by hand, and a 149-mutant cargo-mutants sample.

Not re-run here: the nix builds, check_wasm, check_golden on the real binary, and vitest. The
integrator's `check_all.sh` covers them.

## 1. The audit's Phase 1 exit criteria

| # | Criterion | Status | Evidence |
|---|---|---|---|
| 1 | Each of the 27 high findings has a proof law that failed at a2f2234 and passes | **Met in substance.** 22 fixed; 5 interim or partial (§1a) | Every law in §1a exists and passes at 53b46e0. Each failed at its own package's base, as recorded in the commits (98cef05 re-ran all of P3). None was re-run at a2f2234 itself: the code is unchanged a2f2234 → 9b7c919, but later packages' bases include earlier Stage 1 work |
| 2 | `check_all.sh` runs everything CI runs | Met | 9f4891f, a9fa4b9, 9695ca5, b54d4d8. `--list`: "matches ci.yml (17 run: steps, 3 uses: steps) and flake.nix" |
| 3 | `check_golden` refuses an empty case list | Met | A stub listing nothing exits 1 ("listed no cases") under `--fast` and `--write`, and the corpus stays at 37 files. A stub exiting 1 also gives exit 1 |
| 4 | The swapped-rows fixture fails `check_figures` | Met | `--self-test`: the fixture passes and all 14 plants fail, F2 among them |
| 5 | rustdoc with `-D warnings`, 0 broken links | Met as gated | 427b4af. `--document-private-items -D warnings -A rustdoc::private_intra_doc_links` exits 0. Without private items, 27 public docs link to private items (a Stage 5 item) |
| 6 | No `Instant::now` assert in the default suite; canaries ignored and run serially | Met | 3b19bf9, 7583064. The only sites, gear.rs:1428 and train/mod.rs:10873, are `#[ignore]`d; ci.yml:106 runs them with `--run-ignored only --test-threads 1` |
| 7 | The seeded walk, 200 × 4, is green | Met | 656e026 and 4154814 (Stage 0); offers.rs:578–579. It passes, over the 39-train grid since T16.3 |
| 8 | Mutants H, G, E, T and the four flow-rule mutants are caught | **Partly.** H, G, E and T are caught; **the four flow-rule mutants survive** | Planted again at head (§1b). They are T16.28's: the audit put it in its Phase 4, our plan gives it to N, and no Stage 1 package took it |
| 9 | The survivors (181, not 144) are re-run and counted | **Not met**, and not attempted | `cargo mutants` is not in the dev shell. The audit's 27.1.0 is in the nix store; §1b's sample used it on Stage 1's own lines. That sample is not the survivor re-run, which stays in Stage 4 |
| 10 | The DXF arc law puts every root-arc centre on the axis to 1e-9 mm | Met, restated | outline.rs `every_arc_is_a_tip_or_root_arc_about_the_axis` checks three points per bulged span on one circle about the axis to 1e-9·size, since a sub-micron arc has no placeable centre (T04.1). `a_closed_space_has_no_root_arc` passes. In CI, validate_dxf holds every centre to the axis within 1e-6 mm on 5 exports, the ring and the eccentric gear included |
| — | Track A tasks no package took | **Gap** | **T16.13**, the independent finite-z bending gate, was dropped with no recorded decision. Dolan–Broghamer still has no outside value, and K_f's H and L remain suite-silent (CLAUDE.md) |

### 1a. The 27 high findings

"Fixed" means the law fails at its package's base and passes at 53b46e0.

| Finding(s) | Fix | Proof law(s) | Status |
|---|---|---|---|
| added#30 | T05.2 44e3431 | ring.rs `the_fillet_meets_the_flank_on_a_curtate_path_too`, `a_curtate_ring_is_the_shape_its_cutter_leaves` | fixed |
| added#55 | T09.2 3788a97 | tests/metrology.rs `a_ball_touches_the_helicoid…`, `the_span_anvils_touch…`, `helical_contact_at_the_recorded_cases` | fixed |
| added#56 | T07.1 b53e085 | crossed.rs `a_crossed_pair_is_the_pair_its_members_are_cut_as` | **interim:** refused as `FirstMemberOppositeHand`. T07.7's signed β₁ (X/L) cures it |
| added2#45, lens-continuity#0 | T06.1 ca080dc | contact.rs `the_path_starts_and_ends_on_usable_flank`; mod.rs `a_pair_rates_continuously_through_the_onset_of_interference`, `flank_interference_is_said_by_the_mesh` | fixed |
| added2#94 | T12.1 5495aab | shape.rs `searching_never_loses_a_solve_or_efficiency` | fixed |
| crossed-worm#0 | 9a5eb53 (T07.2–4 dropped) | crossed.rs `a_crossed_pairs_contact_never_exceeds_what_its_faces_carry` | **interim:** ε is bounded by the faces and an off-face contact says so. η and pressure still come from the tips' zone, and the search is not face-aware (X, contact model) |
| edit-ops#0, graph-ops#2 | T13.2/T13.3 656e026 | offers.rs `an_emptied_train_takes_the_next_preset_as_it_would_come`; the walk | fixed |
| kinematics-flow#1, lens-performance#0 | T11.1 3b442db | flow.rs `a_flow_past_its_mask_is_refused_by_name`; mod.rs `…_said_on_the_case` | **wrap fixed** (`TooManyMeshes` above M = 31). **Cost unchanged:** 2^M runs for every M ≤ 31. F |
| lens-feature-gaps#1, strength#1 | T08.4 34f9491, T08.6 6daffce, 4077998 | rating_laws `each_allowable_moves_its_own_width_and_nothing_else`, `the_canary_says_its_flank_is_past_its_allowable`; gear-io `a_reversed_root_on_fully_reversed_data…` | fixed. Steels read the ISO 6336-5 estimate; others are said unjudged. Contact sizing is still off by default |
| lens-numerical-robustness#0 | T08.2 ae2c58e, 360c2f0, c7047e5 | rating_laws `every_bending_figure_is_absent_or_positive`; strength.rs `the_rating_is_continuous_across_the_pointed_limit` | fixed. The held section is ≤ 19 % unconservative (recorded) |
| lens-numerical-robustness#3 | T03.4 0210297 | geometry_laws `the_tooth_ends_at_its_tip`; rack_simulation `…past_the_undercut_band` | fixed |
| mesh-contact#0, train-mod-a#0 | T10.1 66c1fcb | mod.rs `a_path_band_is_each_distance_band_summed`, `widening_a_tolerance_never_narrows_a_band` | fixed |
| metrology#0 | T09.1 43977e7 | jgma.rs `every_printed_edge_reads_the_row_that_prints_it` | fixed. The standard scale's edges are assumed |
| ring#0, ring#1 | T05.1 908bbb3 | ring.rs `a_pair_the_roll_finds_fouled_is_called_fouled`, `a_pair_called_clear_rolls_clear` (an independent roll) | fixed |
| shape-a#0 | T10.2 00ddd36, f392f93 | shape.rs `carried_axes_stand_where_their_distances_put_them` | **partly:** triangles close. Still unchecked: longer carried cycles, and a planet against a central member it does not mesh (M3/M5, U8) |
| strength#2 | T08.3 b522a0e | iso_6336_3_stack.py reproduces the tool's ISO set to 1e-8; check_figures gates the bands | fixed |
| strength#3 | T08.1 cd7377f | rating_laws: σ_F ∝ K_A, σ_H ∝ √K_A; a Known-approximate entry | fixed (the stresses are said to be nominal) |
| tooth-form#0 | T03.3 a91fc7a, a807f21, f76ade3 | geometry_laws `the_depth_stops_where_the_racks_tooth_closes`, `the_outline_is_a_simple_closed_curve` | fixed |
| train-mod-a#3 | T10.3 ed406d2 | shape.rs `relief_leaves_one_size_per_group`, `two_distances_asking_two_sizes_are_refused_by_name` | fixed. 9 overlap readings stay unhonoured (T10.11) |
| train-mod-b#0 | T11.2 5956472 | mod.rs `a_case_does_not_read_its_seeds`, `a_case_is_the_same_at_any_speed` | fixed |
| web#0 | T19.2 90620be, a9c7961 | vitest, CI's "Front-end tests" step | fixed (not re-run here) |

### 1b. Mutants

**Named mutants,** planted at 53b46e0 and run against the gear-core suite (704 tests):
- H, G, E and T are caught, each by exactly one law: `a_mirrored_case_rates_as_the_case`,
  `an_intermittent_sweep_counts_what_running_through_it_counts`,
  `a_path_holds_at_rest_where_no_mesh_does` and `a_paths_figures_are_the_case_through_it`.
- **The four flow-rule mutants in groupings.rs all survive, 704/704:** M6 (least power first), M7
  (every solved mesh idle), M11 (junction terminals always walked on) and M16 (a junction's least
  power). `a_case_that_does_not_solve_calls_no_mesh_idle` asks only that *some* mesh idles, and idle
  is still an absolute `1e-9` (groupings.rs:242).

**The sample.** cargo-mutants 27.1.0, `--in-diff` over Stage 1's changes to 17 production files (2,078
mutants), 1 in 14 round-robin: 149 mutants, run on gear-core's tests, as the audit's were.

**Result: 119 caught, 17 missed, 13 unviable, 0 timeouts.** That is 87.5 % of the 136 viable mutants,
or 91.6 % once the 5 equivalent or dead ones are left out and the run-scope one is counted as caught.
The audit's baseline was 84.4 %; Stage 4's target is ≥ 92 %.

The 17 survivors:
- **Equivalent (4):** hertz.rs:597, ring.rs:323, strength.rs:873 and mod.rs:1377. Each makes a
  comparison inclusive at an equality.
- **Dead (1):** ring.rs:579, `solve_root_end` → 0. The fully filleted ring never fires, so its
  `unwrap_or(0.0)` is dead too.
- **Run scope (1):** gear.rs:331, the tip envelope. Planted here, it fails gear-io's
  `the_drawn_envelopes_are_the_gears`.
- **Real gaps (11):**
  - Three derived tolerances are held only from the loose side, and each can grow by orders of
    magnitude with nothing failing: flow.rs:348 (the tie, `ZERO * tie_power`), shape.rs:941 (Brent's
    agreement, `4ε|β| + x_tol`) and shape.rs:3482 (collinearity, `4ε(r₁+r₂+d)`).
  - Three new branches are reached from one side only: ring.rs:1038 (`tip_clearance`'s `None` when
    the circles do not cross), screw.rs:943 (9a5eb53's spur member off its face) and hertz.rs:394 (a
    point contact's κ).
  - Three rating paths no fixture needs: strength.rs:1593 (the sweep's interior samples), strength.rs:890
    (the moved load's axial term) and shaper.rs:293 (the ring fillet's turning-rate curvature).
  - Two notes quote values that nothing checks: mod.rs:805 and mod.rs:1058 (mutation.md's L4).

## 2. Items carried out of Stage 1

Merged from stage1.md, the commit messages, the P3 check, the P6 status and this review. The source
is in brackets.

**Closed in Stage 1:** the s0-tools five (44eb199, 9695ca5, 65a1f51); the ring's α guard (44e3431);
validate_dxf's eccentric case (6ece87e); the shipped sets' interfering rings (91ee874, 89c4be1).

**Stage 2 — boundaries and errors (V)**
1. `import_train` checks only the graph, so a helix of 90° or a NaN in a file panics at `&r[1..]` in
   `Gear::profile` and `Ring::profile` (T01.6). T16.21's known-panic lists — 84 core and 19 wasm calls
   in debug, 16 and 4 in release — must be empty at Stage 2's exit. [P2]
2. Path efficiency reads 0.00 when a flow is refused (Worm AddGear edits 18 and 23). [P8]
3. An automatic face width of 0 on an idle mesh with contact sizing on gives `NoContact` (Layshaft).
   [P6]
4. `edge_of_undercut` reads a Brent failure as "no edge" after a doubling walk of up to 16 steps
   ("a heuristic"). u_tip and u_j are NaN on severed teeth. [P3]
5. 17 `unwrap_or(0.0)` remain in gear-core's production code, and Stage 1 added more zero fallbacks:
   strength.rs:599 and :602 read an absent fillet as 0. [review]
6. `clamp.flank_unsolved` is never fired and is reachable only from non-finite input. Delete its
   fallback once V refuses that input. [f76ade3]
7. Worm `Distance(0)`'s `NoContact` names the wrong cause. The key is Stage 2's; the relief behind it
   is C's. [P2]
8. The MeshedPlanets preset needs margin: it sits 0.06 mm from where its planets can be placed. [P2]
9. T01's crossed-contact bullet names `NoRootSection`, which is gone. [P6]

**Stage 3 — structure**
- **M3/M5:**
  - H2's `TwoFrames` refusal becomes the per-mesh transfer vertex. [Stage 0]
  - Longer carried-axis cycles, and a planet against a central member it does not mesh (U8). [T10.2]
- **M6:** T10.4's odd thickness cycle, and T10.11's 9 overlap readings, whose note names the ratio
  asked, never the one reached. [P2]
- **F (M8):** the 2^M cost for M ≤ 31 [T11.1]; ties settled by efficiency, then enumeration order
  [P8]; a two-positive-branch fixture for breakaway's tie-break [P1]; flow.rs:348 [review].
- **N:** T16.28's four mutants, the idle `1e-9`, tight-side laws for shape.rs:941 and :3482 [review],
  and the homogeneity κ heuristic [s0].
- **C (M10) and the search:**
  - T10.9 ×6: inputs left given and unread (Ravigneaux Clearance(2); the Worm, WormAndPair and
    Wolfrom shifts), and too much pinned at the worm's distance [P2];
  - Layshaft: 46,251 candidates and 98,966 teeth, about 1.4 s [P1];
  - tip holds re-cut per candidate (a pair's teeth 1,822 → 4,912; a set's rings 961 → 3,983), so
    resolve the holds once per shift plan;
  - the 11/17 and 13/18 budget pins, and the stale "4,120" comment [P4, ring-tip].
- **R:** T10.10's tip room (2 unhonoured inputs); a held tip's far gap, too large by m(h_typed −
  h_held), +0.124 mm on 56/61 [ring-tip]; ring.rs:1038 [review].
- **X:** a signed β₁ (T07.7); off-face crossed pairs, whose η and pressure come from the tips' zone and
  whose search is not face-aware [P5]; screw.rs:943 [review].
- **Contact model / L:** the held Lewis section reads 19–21 % unconservative against a fresh search,
  and it inflates the sharing relief the notes quote [P6]. Also hertz.rs:394 and strength.rs:890 and
  :1593 [review].

**Stage 4**
- T16.13; §4 moves it earlier.
- The survivor re-run, and the 130-constant perturbation.
- An owner's call: contact sizing is off by default, and the canary's flanks stand at 2.5–3.1× their
  allowable. [P6]
- The edges of the JGMA standard scale, which need the standard's text (T09.12). [P7]
- validate_dxf's unmodelled capped round, near-axis cap and closing space. [P1]
- The derivation scripts still run by hand: first_yield, helical_measurement, worm_flank_curvature
  (T16.30).
- Dead code: `gear.bending_unrated_in_mesh`, `central_teeth`'s (false, None) arm, and
  `solve_root_end`'s solve. [P5, s0, review]

**Stage 5:** 27 public docs link to private items. [review]

## 3. Patterns the checkers kept finding

By the commit record, ten of the eleven packages needed a round of fixes after their check; P5's only carried an item. P1,
P6 and ring-tip needed two rounds.

**A. Gates weaker than claimed**, the most frequent class. check_all ran `run: >-` as a command;
check_golden ran a malformed line bare; the homogeneity law took its expected power from the data
under test; validate_dxf read a ring's radii off the file; check_figures matched across lines and
tuples and ignored signs; the count gate's ceilings were "measured + 5 %" with a cut bound 8.6–155×
loose; train_kinematics skipped sections; T16.7's escape never fired; the known-panic lists were keyed
by message prefix; the web sweep swallowed a refusal; the stall law passed at base. The root each
time: the author picks the plant, and it is gross rather than at the gate's blind spot.

**B. Tolerances set by feel:** REL/ZERO/FLOOR; the continuity law's 1e-4 and 20×;
`FILLET_FRACTION_OF_MAX` as a margin (a 3e-12 rad root arc left); the `e·(1 + 1e-9)` lean; a canary
raised from 60 to 250 ms; the idle `1e-9`. Every derived replacement found something (f392f93,
f76ade3, dec492f), yet the sample shows three derived ones held from the loose side only.

**C. Absence as a value:** `unwrap_or(x0)` and `unwrap_or(0.0)` (D1, D3); the CLI's +0.0000° for a
missing record; `past = ∞` in the first tip hold, which drove an addendum to −2.4985; a raw `tan 0`
giving a NaN thickness, found at once by D3's `debug_assert`; efficiency 0.00.

**D. Loops that correct rounding.** The tip hold's doubling up to `MANTISSA_DIGITS`, T05.2's ×1.4
walk, T03.4's nudges and the Brent lean were each replaced by a shorter exact construction.

**E. Behaviour that depends on a gear's kind:** the tip bound on for rings, off for external gears
and set by birth site; U1's early return for rings; the pointed tooth's "fillet alone" branch, which
left a step at the limit.

**F. Proof hygiene:** P3's base proofs unrun until 98cef05, and a base figure misquoted; 7 fixtures
turn the tip hold off; the budget test ran with the ring floor off until 9294bb7; the exception lists
grow (19 unhonoured inputs, 3 unfired notes, the known panics, two budget pins).

**G. Refusals that name a symptom.** "That far apart" said of axes too close; T06.1's false
`NoContact`; the worm's `NoContact`.

**For WORKER.md**
1. **A gate ships its own plants.** Its `--self-test` or law includes at least one near-miss fault,
   one that a naive or the previous version of the gate would pass. The checker adds one plant of its
   own. An assertion in a loop or a branch also asserts how many times it ran.
2. **A tolerance has two laws:** a rounding-level input passes, and a fault at 10× the tolerance
   fails. Its value is an ε-and-operation-count expression with its derivation, a named user input
   (decision 3), or a constant with its basis in N's module. "Measured + x %" only in `#[ignore]`
   canaries. A literal scan beside check_units fails a bare float below 1e-3 outside a named `const`.
3. **Absence.** A check_all diff gate refuses a new `unwrap_or(<literal>)`, `map_or(<literal>, …)`, or
   `f64::INFINITY`/`NAN` used as a value, unless the line says `// absence: <why>`. A solver `Option`
   that "cannot" be `None` gets a `debug_assert!`.
4. **Land on a bound by construction:** take the bracket end with the needed sign, or tag by
   construction. No retry, nudge or grow loop. A loop that is truly needed has a derived trip count,
   and a law that it exits on the first pass.
5. **Kinds.** A new `match kind`, ring branch or per-kind default ships a law over both kinds and a
   continuity law across the switch. A default is a rule per gear, never per birth site.
6. **A proof table closes each package** (98cef05's form: law | base, test file only | head), and the
   checker reproduces two rows. A fixture that turns a shipped default off names it, and the law also
   runs at the defaults. Each exception list names the stage that empties it.
7. **Before the check:** report the work-count diff, and run a small `cargo mutants --in-diff` sample
   on the package's own lines. Put cargo-mutants 27.1.0, already in the store, into the dev shell.

**For the plan**
- Stage 2's exit gains three conditions: the known-panic lists are empty; the absence gate is green
  over all of gear-core; and every refusal key has a fixture on either side of its cause, with every
  value a note quotes being the report's own (L4, T16.20).
- Each Stage 3 redesign opens with M0, the structure snapshot, and its laws land before its code.

## 4. Does Stage 1 change the order?

1. **Move M1–M3 into Stage 2, ahead of V's structure validator.** T01.1 (c4ae11e) already put
   `Shape::validate`/`carriers` at every train entry: V's structure half and M3's first half. T10.2
   left carried cycles unchecked because "no preset builds one", which is M3's `CarrierTree` rule.
   Writing V on today's scans and again on `Incidence` is double work, and design-graph.md §5 says
   M1–M4 need nothing first. M4/M5 can wait: H2's refusal holds on the walk.
2. **Move T16.13 into Stage 2 as a gate**, ahead of L and of the contact model in Rust. P6 moved the
   bending section (360c2f0) with no outside value; the 19 % bias surfaced only through a checker's
   instrument; the sample's rating survivors (strength.rs:890, :1593; shaper.rs:293) are the same
   blind spot.
3. **Move T16.28's laws to the start of F, out of N.** They are cheap, and M9 rebuilds the flow view
   on F while nothing pins the rules it replaces.
4. **Keep F first in Stage 3.** lens-performance#0 is still live: 2^M runs for every M ≤ 31. The audit
   measured seconds at M = 16–18, which extrapolates to minutes near M = 25, and decision 4 forbids a
   chosen cap.
5. **The search is now the main source of cost and of tolerances:** Layshaft's 1.4 s, about 2.7× the
   teeth from tip holds, two budget pins, the 12/29 dip, a search that is not face-aware. C stays
   after F; its first step should resolve each bound once per shift plan, and its exit should turn
   the count snapshot into a bound, calls ≤ C·dof·budget (the audit's Phase 3 criterion).
6. **Nothing argues for moving V later, or the contact model earlier.** Every Stage 2 item is about
   input or absence.
