## T04 — The gear outline, eccentric gears and DXF export

**Why.** The outline and its export are built from hand-sequenced walkers and two stopping rules, and every check reads them the same blind way. The missing root-arc vertex is the clearest case. One root arc per tooth is bulged onto the wrong chord, so in the DXF that CI generates, 17 of 51 arcs are centred up to 5.236 mm off the axis. Three things fail to see it: the in-crate tolerance test (it treats every arc as concentric), the bulge test (it checks the start vertex only) and `validate_dxf.py`, which prints "all checks passed". The chord tolerance is also a hope rather than a bound. The midpoint-sagitta stop overshoots it by up to 4.65×. Below about 2e-9·m the depth cap silently takes over. That cap bounds each curve but not the whole outline, so a z200 gear at 1e-10 mm makes 13.1 M vertices and a 510 MB DXF in 13.3 s. The eccentric gear adds its own family of defects, all inside published ranges: a backlash law copied without the kind's sign (internal play and interference swapped), a DXF that draws its tip circle on the axis while the teeth cross it by 0.30 mm, and root and fillet geometry that folds or crosses itself at λ≠0. That feature is developer-only, and no train ever builds it.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T04.1 Every section emits its own end vertex | gear-outline#0, gear-io#1, gear-outline#8, gear-outline#9 | medium | S | — |
| T04.2 The tolerance law measures the true arc | gear-outline#1 | medium | S | T04.1 |
| T04.3 A subdivision stop that bounds the chord error | lens-tests-geometry#2, ablate-constants-geometry#5 | medium | M | T04.2 |
| T04.4 The DXF takes the `Gear`, and draws the envelopes it has | gear-io#3, added2#54, wasm-boundary#4 | medium | S | — |
| T04.5 CI reads back every kind of DXF and checks each arc | gear-io#2, added#37, tools-ci#15 | medium | M | T04.1, T04.4 |
| T04.6 Eccentric backlash through the one backlash law | lens-unification#0, gear-outline#4 | medium | S | — |
| T04.7 One chord-tolerance floor, reported, with its bound sent from Rust | added#19, added2#103, wasm-boundary#12, gear-outline#12, ablate-constants-geometry#11, added2#81 | low | M | T04.3 |
| T04.8 Decide whether to keep the eccentric gear | ablate-features#15 | info | S | — |
| T04.9 State λ's two operating modes | gear-outline#3 | medium | M | T04.8 |
| T04.10 λ: seam and crossing law, a published bound, root identity by section | gear-outline#6, gear-outline#10, added#25 | medium | M | T04.1, T04.8 |
| T04.11 A root displacement that cannot fold the fillet | gear-outline#5 | medium | M | T04.3, T04.10 |
| T04.12 The eccentric mate's thickness is an input | wasm-boundary#11 | low | S | T04.8 |
| T04.13 `amplitude_for_throw`'s edge in closed form | gear-outline#14 | low | S | T04.8 |
| T04.14 Arc-fitted flanks in place of chords | gear-outline#13 | info | L | T04.2, T04.3 |

### T04.1 Every section emits its own end vertex

**Change.** Every emitted section ends on its own end vertex, and an arc is one bulged vertex followed by that end. This removes the whole class of missing and duplicated vertices, in four places:
- `crates/gear-core/src/outline.rs`, `Tooth::emit_root`, concentric branch, side < 0: after the bulged vertex (rf, −half_pitch), push `Vertex::line(pt(rf, −θ0))`. `subdivide` never emits its start, so today the bulge lands on the chord to the first fillet sample [gear-outline#0, gear-io#1].
- In the same function, delete the severed branch's explicit `pt(rf, −θ0)` push and the varying-root branch's `curve(1.0)` push. Each duplicates a vertex that the previous walk already ended on: z zero-length segments on every eccentric gear, plus one per severed tooth [gear-outline#8].
- In `Ring::outline` step 1, push `pt(rf, −theta_root)` after `start` [gear-io#1].
- In `Tooth::build_with_z`, when `CLAMP_TIP_CAPPED_POINTED` fires, set `θ_a = 0.0` and `u_tip = u_point`, as `check_severed` already does. The walker then skips the tip vertex and its bulge when `θ_a == 0`. Today a round-off tip span of about 1e-15 mm is left on every pointed tooth, and on the 40% where θ_a rounds negative the flanks cross [gear-outline#9].

**Proof.** Write these laws first. They replace `only_the_tip_and_root_arcs_carry_a_bulge` and run on external, helical, undercut, severed, pointed, eccentric and ring outlines:
- (a) For every bulged span, the arc centre computed from its chord and bulge lies on the axis within 1e-9, both ends lie on rf or ra within 1e-9, and bulge = tan(Δθ/4).
- (b) No two cyclically consecutive vertices are closer than 1e-12·ra.

On today's code (a) fails on every tooth: centres are 22.4 mm off at z17, 47.6 mm on a z43 ring and 83.8 mm on a z90 ring. (b) fails on z24 Δx0.25 (24 duplicates), on z9 x̄−0.3 Δx1 (12) and on pointed teeth (2,303 spans under 1e-12 mm across 138 outlines). With the three-line fix applied, the suite passes apart from the known timing test, and the test's own deviation metric falls to at most 0.9993×.

**Notes.** One vertex more per tooth, so re-record `dxf_17_0.2_0.001`. Check `meshsweep_60_20_0.8` and `hulasweep_18_0.25` too, because `roll_pair` flattens outlines. The lasting cure is one walker over `sections()` for tooth, gear and ring, which is T05.17 [gear-outline#7]. Law (a) will also catch the outline defects of T03.3, T03.4 and T05.5 [tooth-form#0, added#6, lens-numerical-robustness#3, ablate-constants-geometry#0, added#15, added2#19], so expect it to need their fixes, or their cases excluded by name, before it runs the whole grid.

### T04.2 The tolerance law measures the true arc

**Change.** In `outline.rs`'s `worst_deviation`, measure a bulged span as the true point-to-arc distance, with the centre taken from the chord and bulge and the angular extent clamped. Today it uses `|r_p − r_a|`, which sees neither a non-concentric arc nor an arc's extent. Remove the `r ≤ 3.0` and `ratio[2] ≤ 1.4` thresholds, which sit just above the values the missing vertex produced. Correct the test doc's diagnosis: the 2.5× was the missing vertex, not undercut flanks. Give the ring the same deviation law in place of its vertex-only check, and drop its vacuous `gap < 2·pitch_arc`.

**Proof.** Tightened to 1.1×, the test fails on today's code (z9 x−0.2: 2.51/1.81/1.11) and passes once T04.1 lands. Run it against HEAD in a worktree first, as testing rule 5 asks. The final bound comes from T04.3.

**Notes.** T16.23 [gear-outline#2], where the ring refinement test measures an external gear, folds into this law.

### T04.3 A subdivision stop that bounds the chord error

**Change.** `subdivide` stops on the sagitta at the parameter midpoint, which does not bound a chord's worst deviation. Using the test's own metric, the ratio reaches 4.65× (z5 x0 β25 m5 at 1e-4), and 147 of 288 cases exceed 1.5×. It does not converge as the tolerance tightens: z17 swings between 1.0 and 1.97. Replace the stop with a bound. Split while L²/(8·ρ_min) > tol, where ρ_min is the smallest radius of curvature over the span. Both curves have closed forms: the involute's ρ = r_b·u, and the trochoid's is `rolling_curvature_radius`. ρ is monotone on each section, so the span's endpoint values bound it. Quarter-point sampling was proposed instead and rejected: it is still a heuristic. Correct the docs that promise "within 1 µm" (outline.rs module doc, `subdivide`, reference.md#sampling).

**Proof.** worst_deviation/tol ≤ 1 + (reference error) at 1e-2, 1e-3 and 1e-4. The grid is widened to turn z ∈ {5, 9, 17, 43}, x, β, ρ, m and thickness_mod. It fails today at 4.65×. Record the vertex growth before merging, and re-record the golden DXF.

**Notes.** This also bounds the eccentric export stray of gear-outline#5, whatever shape the displaced fillet takes.

### T04.4 The DXF takes the `Gear`, and draws the envelopes it has

**Change.** Change the signature to `gear_io::gear_to_dxf(gear: &Gear, opts)`. The outline comes from `gear.outline(tol)` and the reference circles from `gear.mean()`, the tooth the panel quotes. `export_dxf_impl` (gear-wasm) and gear-cli's `dxf` then build one `Gear`, which removes the second `Gear::new` inside the writer [added2#54, wasm-boundary#4]. `outline_to_dxf` takes envelopes (centre plus radius, or a closed polyline) rather than bare radii, so the writer knows nothing of gear kinds. Pitch and base stay as circles at the origin. Tip and root become the limaçon r(θ) = ra + e·cos θ and r(θ) = rf + e·cos θ, with e = m·angular_shift, as an LWPOLYLINE to the chord tolerance [gear-io#3]. At e = 0 they collapse to today's circles, byte for byte.

**Proof.** A gear-io law over angular_shift ∈ {0, 0.1, 0.3}: max |v| over the outline equals the drawn tip envelope's maximum within tol, the root likewise, and the circle radii equal solve_gear's. It fails today in two cases:
- z30 m1 Δx0.3: the teeth cross the drawn tip circle by 0.30 mm.
- z6 x0.5 Δx0.8: rf is 2.25 in the DXF against 2.15 in the gear.

Golden `dxf 17 0.2 0.001` is unchanged at e = 0.

### T04.5 CI reads back every kind of DXF and checks each arc

**Change.** In `tools/validate_dxf.py`:
- For every bulged span, compute the arc (θ = 4·atan b, then the centre on the chord's bisector). Require |centre| < 1e-6 mm, both ends on one radius, and that radius equal to the root or the tip.
- Require each tooth's included root and tip angles to sum to the analytic values, so a dropped vertex cannot leave concentric but short arcs.
- Reject zero-length segments.
- Derive the reference radii from `--teeth --module --shift --pressure-angle`, replacing the four radii hand-typed in `ci.yml`.
- Fix the comment that says the radius checks "would catch a wrong bulge".

In `ci.yml`, run the check on:
- 17/0.2;
- 9/−0.3, which passes today and needs no new tool;
- a ring, through a ring mode on gear-cli's `dxf` COMMANDS row, which also gives the ring a golden. The script's ring rules: hole winding, tip inside root, the rim as a construction circle, and a bulge count set by the ring's form;
- an eccentric gear, once T04.4 lands.

**Proof.** Check the gate first. The new script must fail on today's CI DXF (17 off-axis spans, the worst 5.236 mm) and on a copy with the leading root half-arc's bulge flipped, which today passes. It must also fail on a ring with its winding flipped. It must pass on the export after T04.1. Land it in the same change as T04.1.

**Notes.** The docs that misdescribe this tool are T18.1 and T18.2 [added2#56, added2#57, tools-ci#13].

### T04.6 Eccentric backlash through the one backlash law

**Change.** Add a free function `mesh::backlash_at(kind, a_ref, alpha_t, alpha_w, a) = kind.sign()·2a(inv α(a) − inv α_w)`. `Mesh::backlash` calls it, and so does `centre_profile_of` in place of its hand copy (gear.rs:986-1006, which omits σ although it is in scope). The copy's second acos/domain check goes too. Keep `RingTooSmall` and the domain errors. On `MeshSide::Second`, handle the sign itself, not just the argument order.

**Proof.** A law over both kinds, both sides and Δx ∈ {0.05, 0.2, 0.4}: `sinusoid_backlash` equals [min, max] of `Mesh::new(tooth_k, mate, kind).backlash(fit_k)` to 1e-12. The Internal rows fail today: z20 in a z60 ring at Δx0.3 reports [−0.008590, 0.009076] where the law gives [−0.009076, 0.008590]. The existing tests (gear.rs:2022, gear-wasm lib.rs:2140) check only that an external range straddles zero.

**Notes.** The panel's interference warning fires on every range, because a fitted residual always straddles zero. Reporting the crank offset that never interferes (from min j) would give it content. The exact-zero branch in the boundary is T15.18 [wasm-boundary#13].

### T04.7 One chord-tolerance floor, reported, with its bound sent from Rust

**Change.** Replace `MIN_RELATIVE_TOLERANCE` and its two copies with one clamp function in `outline.rs` that `Gear::outline` and `Ring::outline` both call. The copies are gear.rs:613 on `mean.ra` and outline.rs:265 on `rf`, while the doc says "tip". The floor must scale with the tooth, not ra. An absolute 1 nm (1e-6 mm, 5000× below JGMA's finest 5 µm) was measured to keep every tested gear off the depth cap: at z200 m1 it gives 238k vertices, a 9.3 MB DXF, 250 ms and no span ending on depth. Re-measure after T04.3.

The outline returns its vertices with notes:
- `clamp.chord_tolerance_raised` when the floor binds;
- a note when 0, a negative or NaN is replaced by the default;
- a note if any span still ends on depth.

The DXF entry points return a result carrying those notes instead of a bare `String`. A non-finite sagitta ends subdivision as an error rather than recursing to depth. The floor reaches the panel through the defaults or ranges payload, so `GearPanel.svelte:548`'s `min="0"` is replaced by a number Rust computed (rule 1). `MAX_SUBDIVISION_DEPTH` becomes a safety stop that no admitted input reaches.

**Proof.** A law over z ∈ {17, 200, 2000} and the grid's largest module:
- at the floor, no span ends on depth;
- vertices per tooth stay under one constant;
- the note fires at 1e-9, at 0 and at NaN.

Today the tolerance is missed silently, by 19.7× at z17 1e-10 and 98× at z17 m5. z200 gives 13.1 M vertices, and z2000 at 1e-9 gives 49.7 M. Rewrite `an_unreachable_tolerance_stays_bounded` as that law. Today its band [400k, 1.2M) still passes at depth 13 (about 557k), despite its "catches a factor of two".

**Notes.** A degenerate gear (m = 0 or NaN) gives 1,114,146 vertices [added2#81]. With the non-finite stop the outline refuses it. The params gate that refuses it at the boundary is T02.6 and T01.6 [lens-errors-policy#9, lens-numerical-robustness#4]. The comments that credit the floor with the cap's work (T18.23 [added#20]) and the redundant wall-clock assert (T16.8 [added2#86]) land with this task.

### T04.8 Decide whether to keep the eccentric gear

**Change.** Decide whether to keep the eccentric gear before T04.9–T04.13 are funded. It costs about 550 lines of code and 460 of comment across gear.rs, metrology's `*_at`/`_around`, auto.rs and gear-wasm, plus about 30 tests of its own. It is reachable only from the single-gear tab in developer mode, and every train member is built at `angular_shift: 0.0` (shape.rs:1027). Keeping it means about two weeks of the tasks below. Cutting it turns T04.6 and T04.9–T04.13 into one deletion and reduces T04.4 to its signature change.

**Notes.** Do not split the eccentric results into a separate optional result. That would bring back the branch rule 4 removed, and at Δx = 0 `Gear::new` already generates one distinct tooth.

### T04.9 State λ's two operating modes

**Change.** The documents give one trade-off (drive error |1−λ|, coast |1+λ|, "λ=1 exact forward") and never say that it assumes a fixed axis distance. The same panel reports a commanded profile and a crank in which the mate follows a_w(x_k). Rewrite the following to state both modes:
- reference.md#angularly-varying-profile-shift;
- rationale.md:1088-1103;
- gear.rs:32-44 and 666-678;
- `strings_*.toml:312` in all five catalogues.

At a fixed distance, the |1∓λ| trade-off holds. With the mate following, λ=0 is best on both flanks and λ≠0 adds λ(ψ̄−ψ_k)·r_b to both. Whatever λ is, a tooth-frequency residual of about one adjacent seat step remains (about 22 µm peak to peak at z24 Δx0.25 m1). Retitle `Variation`'s drive and coast pitch errors as fixed-distance inspection quantities. Keep λ: it is the only way to be exact forward at a fixed distance.

**Proof.** A per-position law that closes the drawn outlines at `commanded[k]` on each flank. It asserts zero spread across k at λ=0, and λ·max|ψ̄−ψ_k|·r_b at λ≠0. At z24 Δx0.25 against z43, the check's figures are: following, λ=0 0/0 µm and λ=1 171/171 µm; fixed distance, λ=1 0/342 µm. Tag the figures for `check_figures.py`. Do not assert continuous exactness.

### T04.10 λ: seam and crossing law, a published bound, root identity by section

**Change.** Write the law first [added#25]. It runs over z ∈ {9, 12, 17, 23}, Δx up to `admissible_angular_shift` and λ ∈ {±0.5, ±1}, on both `Gear::outline` (bulges flattened to several chords) and `Gear::profile`:
- each tooth's last root vertex equals the next tooth's first within 1e-12·ra;
- there is no proper self-crossing.

Delete the `continue` at gear.rs:2382-2385 that skips the export at λ≠0.

Then fix the geometry:
- Add `Gear::root_point(k, side, along)`, called for every root sample by both `emit_root` and `Gear::profile`. Delete the bitwise `r == g.rf` test in `corrected` [gear-outline#10].
- `root_reach < θ0` is exactly the condition that neighbouring fillets overlap [gear-outline#6]. Publish its closed-form bound in `admissible_ranges`, |λ| ≤ 2(π/z − θ0) / max_k |ψ_b,k − ψ_b,k+1|. `Ranges` gains an `index_offset` bound, which the panel reads. A λ beyond the bound is clamped with a note (rule 5).

**Proof.** The law fails today on z9 Δx0.75 λ±1 (2 crossings in both drawings, 0 at λ=0). Across 1,477 in-bound cases, 70 invert. It also fails under a one-ulp change to the export's root radius, which opens 22 of 23 seams on z23 Δx1 λ1 while today's suite stays green. It passes after the fix.

### T04.11 A root displacement that cannot fold the fillet

**Change.** In gear.rs's `displacement` (404-422), the smoothstep in radius over the fillet alone folds once |Δ| > (2/3)(r_j − r_f). Displace instead along one monotone parameter over the whole run from junction to mid-space: the trochoid parameter joined to the root arc's angle, normalised by arc length. State the no-fold condition max|Δ|·w′/L < 1. Where it would still fail, clamp and raise a note.

**Proof.** Two laws over z, Δx up to the admissible maximum, small ρ and λ:
- the drawn fillet never goes below `root_at(angle)`;
- its radius is monotone from junction to root, apart from the envelope's own slope.

Both fail today. On z23 α25 x0.2 Δx1 ded1 add0.8 λ1 (ρ 0.0475), the slope reaches −2.13 and the notch is 0.0489 mm. On z24 Δx2 ρ0.02, the slope is −3.51 and the notch 0.070 mm. The export of the first case strays 1.44/3.61/4.59× at 1e-2/1e-3/1e-4.

### T04.12 The eccentric mate's thickness is an input

**Change.** `eccentric_mate` (gear-wasm lib.rs:636) copies `..req.params`. Of what it copies, only `thickness_mod` reaches `centre_profile_of`, and it is not shared by definition. Add `thickness_mod` (serde default 1.0) to `MateRef` and build the mate with it. Moving `eccentric_mate` into gear-core beside `centre_profile` is better still. Then run `check_bindings.sh --write` and `check_wasm.sh --write`.

**Proof.** A test on z20 Δx0.3 against z40 at k = 0.95. With the mate at 1.0 against a copied 0.95, the mean commanded distance is 29.884072 against 29.770343 and the throw differs by 3.3%. Assert the shift that the zero-backlash relation predicts.

**Notes.** The mate's tooth count typed in TypeScript is T19.9 [lens-magic-numbers#2]. The resolved-params throw inversion uses the same mate, so this change fixes it too.

### T04.13 `amplitude_for_throw`'s edge in closed form

**Change.** Replace the 64-step bisection (gear.rs:1113-1129) with dx = min(ceiling, v₀ / max(−κ, −κ·c_lo)), where:
- v₀ = inv α_t + 2·x_sum(0)·tan α_n/Σz, with Σz signed;
- κ = 2·sign_e·tan α_n/Σz;
- c_lo = cos(2π⌊z/2⌋/z).

Then step `next_down` while `throw(dx)` fails. That took at most 3 steps in 478 cases. Keep the bisection only as a guard for a failure that is not rounding. Share c_lo with `admissible_angular_shift` (auto.rs:630) through one helper. Keep `solving_an_amplitude_is_quick_enough_to_type_over`.

**Proof.** Across the grid of z, x̄, mate and kind: `throw(dx)` is ok, and `throw(next_up(dx))` fails or dx equals the ceiling. Do not require bit equality with the bisection: they differ by up to 3 ulps.

### T04.14 Arc-fitted flanks in place of chords

**Change.** Dyadic chords cost 1.23–1.50× the equal-sagitta optimum ∫√(κ/8tol)ds, and they kink by √(8·tol·κ) at every vertex. Biarcs fitted with the closed-form tangents (strength.rs:299/357, ring.rs:680/690) are estimated at 2–10 arcs per flank against 6–106 chords. Measure that first. If it holds, replace the chords rather than add an `OutlineStyle` beside them: the file already carries bulges. A cheaper first step, which only saves vertices, places the involute's chords by inverting ∫√(κ/8tol)ds.

**Proof.** The T04.2/T04.3 deviation law at 1.0× with point-to-arc distance, G1 continuity asserted at every joint, the entity count recorded, `validate_dxf.py` green, and one import into SOLIDWORKS by hand.
