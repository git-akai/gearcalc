## T03 — One tooth and the tool that cuts it

**Why.** The tooth is built from its own derived quantities, and it is checked against those same quantities. Four ways a tool can fail to cut what was asked are not detected. (1) The rack's tooth comes to a point before the commanded depth (w_tip < 0). (2) The fillet consumes the whole flank, so the junction lies above the tip. (3) The pressure angle is clamped while the thickness is computed at the unclamped angle. (4) A pointed tip passes the ring's tip check. The cut simulation cannot catch (1) or a depth error made consistently, because `verify::cutter_sdf` places its rack from the tooth's own `a_c`, `r_f` and `ρ`. The test grids never go far enough out: dedendum stays at 1.25, k at 1, α ≤ 25°, x ≥ −0.5. Numbers: 21,005 pointed-rack teeth lie inside every published range, and 19,995 of them carry no note. At z=17, k=1.5 the root is 0.171 mm too deep, and the outline crosses itself 17 times. On the standard z=17, x=−1.5 gear the outline reaches 0.146 mm past the tip circle. On fuzz gear 1302 it reaches 1.06 mm past, with check_cut penetration 0.729 mm. Neither case raises a note [tooth-form#0][lens-numerical-robustness#3]. Behind this sits a structural fault. The rolled-corner construction is hand-written seven times, and the rack is the shaper's z→∞ limit only in convergence tests, not in the code [tooth-form#3].

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T03.1 An independent rack for the cut gate | tooth-form#1, lens-numerical-robustness#2 (verify part) | medium | M | — |
| T03.2 Settle the tool once: tip width, round fit, depth | lens-numerical-robustness#7 | low | S | — |
| T03.3 Cap the depth where the rack comes to a point | tooth-form#0, lens-tests-geometry#0, lens-numerical-robustness#2, added#6 | high | M | T03.1, T03.2 |
| T03.4 End the tooth at the tip when the flank is consumed | lens-numerical-robustness#3, ablate-constants-geometry#0, added#15, tooth-form#6 | high | M | T03.1 |
| T03.5 A pointed tip is a point in the tip-window test | added#31 | medium | S | — |
| T03.6 One guarded pressure angle | lens-errors-policy#8, primitives#1 | low | S | — |
| T03.7 The pointed roll from `inv_inverse`, stored once | added#11, lens-magic-numbers#10 | low | S | — |
| T03.8 The tip width peaks above the base circle | lens-docs-accuracy-2#4, added#5 | low | S | T03.7 |
| T03.9 Publish the severing threshold | added#18, tooth-form#12 | low | M | T03.4 |
| T03.10 Make `cut_by`'s precondition unreachable from outside | tooth-form#7 | low | S | T03.3 |
| T03.11 Correct the shaper and fit-cap prose | tooth-form#4, tooth-form#5, tooth-form#11 (header) | low | S | — |
| T03.12 verify.rs literals | tooth-form#9 | info | S | T03.1 |
| T03.13 One rolling corner, rack at κ = 0 | tooth-form#3 | medium | L | T03.3, T03.4 |
| T03.14 The helical round as an ellipse, in closed form | tooth-form#10 | low | L | T03.13 |
| T03.15 One generator for every member (with a shaper-cut external gear) | lens-unification#2, tooth-form#11 | medium | XL | T03.13, T03.1 |

### T03.1 An independent rack for the cut gate
**Change.** In `verify.rs`, `cutter_sdf` currently reads `g.ac`, `g.rf` and `g.rho`. Replace them with a small `BasicRack` value built only from basic-rack inputs and the settled tool:
- tooth width at the datum, (π m_n/2)(2 − k)/cos β, on the datum line r + x·m;
- tip line at `Rack.depth` and round `Rack.tip_round`;
- transverse angle from α_n and β.

Refuse (`None`) a wedge with x1 ≥ x2. Leave `DEVIATION_LIMIT` (1e-3 mm) unchanged: the gate's own 1080-case grid peaks at 6.15e-4 mm on sharp-cornered (ρ = 0) teeth. That peak is phase-step error, linear in the step. Before the limit can tighten, each point's closest approach must be refined by a bracketed 1-D minimisation over ±1 phase step. The V-shaped distance at a ρ→0 corner defeats the parabolic refinement.
**Proof.** On good teeth the swap changes nothing: a probe version gives the identical 6.151e-4 on the worst grid case. Negative fixtures that must fail, and do not fail today:
- s_t +1e-3 mm moved consistently: penetration 4.7e-4 against 0 today.
- b_d +1e-3 mm: deviation 1.0e-3 against 1.1e-8 today.
- The four pointed-rack cases: 4.06e-2 to 1.76e-1 against 3.75e-4 to 7.9e-4 today, which is inside what good ρ=0 teeth already give.

Run the new gate against HEAD in a worktree and watch it fail. Extend `rack_simulation`'s grid with α 30/35/40°, h_f 2.0, k 1.3, and x −0.9/−1.2 at 14.5° and 20°. Add these as targeted rows rather than a full product: the 1080-case test is already the suite's slowest (T16.10 [added2#100]).
**Notes.** lens-tests-geometry#1 (the same blindness, seen from the tests) and added2#34 (the grid written twice, in `gear-cli verify` and here) belong to the test themes. This task removes their cause.

### T03.2 Settle the tool once: tip width, round fit, depth
**Change.**
- Add one function on `Rack`, taking the clamped values: `Rack::settle(st, bd, α_t, m_t) → (w_tip, ρ_fit, depth cap)`. Three callers use it: `Rack::wanted_by` (tooth.rs:~899–925), `auto::ranges_at_shift` (auto.rs:~676–685, now a copy, which keeps its raw inputs because it evaluates a range) and `metrology::cutter_tip_width` (metrology.rs:727). `cutter_tip_width` reads the raw `p.dedendum − p.profile_shift`, so after `clamp.dedendum_raised` it reports the requested tool: at z=30, x=1.5 it reports 0.661 mm against the 0.443 mm tool that actually cut. Compute it from the tooth's `bd` instead, and fix the metrology.rs:1 module doc, which lists it as a measurement.
- Add `involute::roll_at_radius(r, rb)` (0 below r_b), written as √((r−r_b)(r+r_b))/r_b, and use it at the roughly 13 sites of `((r/rb)²−1).max(0).sqrt()`.
- From T09.9: move `base_helix_angle` onto `impl Tooth`, next to `Ring`'s, and have strength.rs's `ToothOutline` impl call it. `span_over_teeth(g, 0)` reports k = 0 but computes k = 1, and it never refuses a large k: make it return `Option` by `span_over_teeth_at`'s rule, or delete it with metrology#12 (T09.4).
**Proof.** A law: `cutter_tip_width / cos β = (π m_t − s_t) − 2 b_d tan α_t` with the tooth's own `b_d`, on clamped gears too. It fails today at z=30, x=1.5, and at x = 1.3 on the default gear (0.6609 against 0.5881). `cutter_tip_width_is_independent_of_helix_angle` still passes. `span_over_teeth(g, 0)` is `None`. The golden corpus and `check_figures.py` are otherwise unchanged.
**Notes.** This is also T09.9 [added#13, metrology#18(a,c)]. T12.14 reuses `Rack::settle` for the range bounds that copy the clamp rules [lens-unification#9]. The pointed-tool clamp is T03.3 [lens-numerical-robustness#2].

### T03.3 Cap the depth where the rack comes to a point
**Change.** In `Rack::wanted_by`, after the existing depth clamps and the round cap, apply the fit condition ρ(1−sin α_t)/cos α_t + b_d·tan α_t ≤ (π m_t − s_t)/2. The condition is linear in ρ and b_d. If the smallest admitted round still violates it, cap b_d at ((π m_t − s_t)/2 − ρ_min(1−sin α_t)/cos α_t)/tan α_t. Do not apply the 0.95 `FILLET_FRACTION_OF_MAX` margin to the depth: it would drop the depth by 5 % the moment w_tip crosses 0. Raise one key for "the tool's tooth closes before the commanded depth", shared with the ring's `CLAMP_RING_SPACE_CLOSED`, since it is the same fact. Do not reuse `CLAMP_DEDENDUM_CAPPED`, whose sentence ("root would reach the axis") is wrong here. Add strings to all 5 catalogues.

Take the dedendum maximum in `ranges_at_shift` from the same function (T03.2). At ρ = 0 it is h_f ≤ π/(4 tan α_n) − x_s, which does not depend on x, z or β until another depth clamp fires. Equivalently, k_max = 2 − 4 h_f tan α_n/π: 1.4207 at 20°, 1.2578 at 25°. `admissible_ranges` currently reports `thickness_mod` ∈ (0, 2) in every case. Fix the prose where it touches this:
- reference.md:159 (the k row claims to exclude a rack with no tooth width);
- ring.rs:396-397 and ring.rs:1607 (an external gear "has never been able to" close its space; also added#14);
- the "non-tool" sentence in rationale.md:~120.

**Proof.** Laws first; each must fail on HEAD.
1. Add a dedendum axis {0.3, 1.25, 2.0, 3.0} and k {0.7, 1.0, 1.3} permanently to `geometry_laws::grid()`. `fillet_cap_guarantees_a_nonnegative_root_arc` fails today: "root arc is negative … at z=3 x=-0.5".
2. New law: `Gear::outline` is a simple closed curve, with no segment crossings and no negative root bulge. Today: 17 crossings at pa 40, and 2 at pa 20, h_f 2.2, all inputs in range.
3. `tests/extremes.rs::is_constructible` gains the O(n²) crossing check. The pa 70/85 cases assert the new key.
4. The note fires exactly when the asked w_tip < 0. `r_f` is continuous in h_f across the cap.
5. The T03.1 gate and the independent `sim.py` both give r_f = r − b_point. Today at z=20, α=30°, h_f=1.4 they give 8.6397 against the crate's 8.600.
6. `cutter_tip_width` ≥ 0 and `pin_diameter_range`'s lower bound > 0. Today 2,039 of 30,000 fuzz gears start the pin range at 0.
7. `the_geometric_bounds_are_exactly_where_the_generator_starts_clamping` accepts the new key. The golden corpus must not move, since it is all 20–25° at h_f 1.25.

### T03.4 End the tooth at the tip when the flank is consumed
**Change.** In `Tooth::solve_junction` / `build_with_z`, replace the open walks with closed brackets:
- base crossing: `brent(r(s) − r_b)` on [−r_b, 0], valid because r(s) ≥ |s|;
- the fillet at the tip: s(r_a) = `brent(r(s) − r_a)` on [−r_a, s_b];
- flank crossing: `brent(gap)` on [s(r_a), s_b], which reproduces today's junction to 1.6e-11 wherever it lies below r_a.

When there is no crossing below r_a (not severed and r_a < r_j), set s_j = s(r_a) and u_j = u_tip, which leaves an empty involute section. Take θ_a from the trochoid at r_a, and raise one note, `clamp.tip_below_form` (5 catalogues). This one rule covers the undercut band on the shift axis, the addendum-axis case on unundercut teeth, and the tip below the base circle. Replace the silent `ra.max(rb·(1 + TIP_ABOVE_BASE_FRACTION))` at tooth.rs:471 with a floor at the root only: a tooth with fillet flanks only can still be cut. Delete `BASE_CROSS_GROWTH`, `CROSSING_GROWTH`, `CROSSING_NUDGE_MODULES` and `MAX_STEPS`. Check every consumer of [r_j, r_a] against an empty active flank: contact-path start, form circle, the interference tests, strength's critical section, metrology's pin seating, and outline's tip arc. Reuse the severed-tooth short-circuits where they exist.
**Proof.** One law over the `tests/common` grid, extended to the admissible shift floor, α 10–30° and negative addenda:
- max |p| over `half_profile`, `Gear::profile` and `Gear::outline` ≤ r_a(1 + 1e-12);
- radius is monotone;
- u_j ≤ u_tip;
- θ_a equals the trochoid's angle at r_a (0.034800 rad at z17, x−1.5, against 0.022639 today, a tip land 35 % narrow).

The T03.1 gate must show zero penetration at z12 x−1.0 α14.5° (5.14e-2 today) and on gear 1302 (0.729 mm today). The note must fire at z40 x0 h_a −1.0, where the sliver is generated geometry and check_cut sees penetration 0. So only the outline law catches that case (2,073 of 12,807 such teeth, worst +0.653 mm). The overshoot at the band edge x = −1.375 is 8e-4 mm, so a switch placed exactly at u_j = u_tip keeps the outline continuous. Assert continuity across it.
**Notes.** The harness grid is T20.10 [gear-cli#6] (`verify 100` records only z = 3). The NaN sentinels in `u_j`, `u_tip` are T02.9 [lens-numerical-robustness#9].

### T03.5 A pointed tip is a point in the tip-window test
**Change.** ring.rs:955-959 has `if half_p <= 0.0 || half_r <= 0.0 { return f64::INFINITY; }`. Delete the early return and clamp each half-width with `.max(0.0)`. This feeds `tips_clear` in the train's engagement rule.
**Proof.** Ring z24 x1.1, cutter z12, pinion z19 x1.2 at a = 2.3613 must report interference. It gives margin −0.0251 rad; today it gives +∞ because half_p = −1.1e-16. Law: sweep x_p through the `CLAMP_TIP_CAPPED_POINTED` threshold and require that no step in the margin exceeds a small multiple of the x_p step. Repeat for a ring raised to its pointed limit (ψ_b < 0).
**Notes.** T05.1 [ring#1] changes the sign convention in the same function. Land the two together.

### T03.6 One guarded pressure angle
**Change.** Lands as T01.5, which carries this task's readers (every reader of `thickness_shift`, and the ring's missing guard) and its proof.
**Proof.** T01.5's law.

### T03.7 The pointed roll from `inv_inverse`, stored once
**Change.** Store `u_point: Option<f64> = inv_inverse(ψ_b).map(f64::tan)` on `Tooth`. The pointed-tip cap in `Tooth::new` (tooth.rs:455-462) reads it, as do `auto::addendum_for_tip_width` (auto.rs:744, now a second solve on the same bracket) and the ring's tip guard (with −ψ_b). Delete `POINTED_TOOTH_MAX_ROLL`. ring.rs:289-291 calls `inv_inverse` "closed form" and claims the two guards share it: reword it.
**Proof.** Over 175 capped teeth the two routes agree to 3.6e-15 mm. The pointed-tip tests pass, and the golden corpus is unchanged at its printed digits.
**Notes.** This fixes primitives#8, ablate-constants-geometry#7 and auto-search#13 as well.

### T03.8 The tip width peaks above the base circle
**Change.** s(r′) = 2r′(ψ_b − inv α) has slope 2ψ_b > 0 at r_b. It peaks where 2u − atan u = ψ_b. In `addendum_for_tip_width`, solve u_max on [0, ψ_b] and refuse only when width(u_max) < min. Bracket the Brent on [u_max, u_point]. Correct auto.rs:711 ("strictly decreasing"), 721-723 and 738 ("widest where the involute starts"), and reference.md:249-252, which says "monotone decreasing" and "bracketed Newton" where the code uses Brent (also added2#102).
**Proof.** At z=6, x=0.5, an ask of 1.001·width(0) returns `None` today. It must return an addendum whose width equals the ask to 1e-12. The peak exceeds width(0) by 1.8 % there and by 4.5 % at z=4, x=0.8. `check_golden` is unchanged.

### T03.9 Publish the severing threshold
**Change.** Factor `check_severed`'s minimum search into `Tooth::least_fillet_angle()`. Add `severed: Option<f64>` to `ShiftRange` as an advisory threshold, like `pointed`. Find it by Brent on [bound.min, undercut threshold]. The bracket always holds, because severing implies undercut. Compute it only on the panel path, not in `amplitude_for_throw` (gear.rs:1102): it costs about 50 tooth builds. Add the panel clause "severed below {…}" (5 catalogues) and regenerate the bindings. Rewrite the wording in three places:
- `ShiftRange.bound` ("what can be built") and auto.rs:1014-1017 / reference.md:810-812 ("a shape no cutter leaves");
- reference.md:125-126 and tooth.rs:202, 638-640 ("removed the whole tooth"). Replace with: the fillets cross on the centreline, what lies above falls away, and r_a drops about 1.2–1.8 modules to below the base circle.

**Proof.** Unit law over z 3–14, α 14.5/20/25°, ρ 0/0.38 and β 0/30°: the tooth at severed + 1e-6 is not severed, and at severed − 1e-6 it is. `severed` is `None` exactly when the tooth at bound.min is not severed. Today at z=8 the bound runs from −2.1304 while the tooth severs below −0.9551. At the default z=17 it severs below −1.7035. Run `check_bindings.sh`, `check_strings.py`, `check_wasm.sh` and `npm run check`.
**Notes.** The geometry at the threshold is physical and already flagged by `clamp.tooth_severed`. No search returns a severed member (`member_is_buildable`), so no search law is added.

### T03.10 Make `cut_by`'s precondition unreachable from outside
**Change.** Make `Tooth::cut_by` and `Rack`'s fields `pub(crate)`; the only caller is `Gear::new`. Add `debug_assert!(b_c > 0.0)` in `build_with_z`. After T03.3, also assert that the minimum round fits (w_tip ≥ its ρ term); today admissible inputs violate it, so this must wait for T03.3. Do not clamp inside `cut_by`: that brings back per-tooth tool settling. Make `ShaperCut::equivalent_to_rack` `#[cfg(test)]` as well (ring#12).
**Proof.** Law: `trochoid_at(0).0 == r_f` on every grid tooth. A hand-built `Rack` with b_c = −0.13 at z30 x1 today puts the fillet bottom 0.76 mm above r_f, with penetration 0.568 mm. At b_c = 0 the result is NaN. No production path reaches it (0 of 46,240 admissible eccentric teeth).

### T03.11 Correct the shaper and fit-cap prose
**Change.** Three corrections.
- **ShaperCut caps rather than refuses.** It caps an oversized round at 0.95 of what fits. It still refuses three things: a corner inside the cutter's base circle, a tip too narrow for the floor round, and a cut that cannot be placed. The stale "refuses" wording is at shaper.rs:405-409, shaper.rs:647-654, CLAUDE.md:229-230 (which also files the item under "Four in tooth.rs"), state.md:783 and ring.rs:1508-1510. Keep `equivalent_to_rack`'s refusal and document it as deliberate for the fixture.
- **The fit cap.** The plausible w_tip/(2 cos α) would shrink the cap by (1−sin α)/cos²α, which is 0.745 at 20°, on every gear, not only shifted ones. That form would cap the default 0.38 round (0.334). Say this once, in tooth.rs, and point to it from tooth.rs:910-911, auto.rs:538-539, CLAUDE.md:226 and `geometry_laws.rs:115`. The law's doc should read "wherever a round fits (w_tip > 0)". Replace `profile::`/`profile.rs` with `tooth::` at shaper.rs:14, 33 and 716.
- **shaper.rs's header row** "external gear, shaper" becomes "the fillet only, as the rack limit's test instrument".

**Proof.** `check_doc_links.py`, then reading.
**Notes.** added2#58, added#0, lens-docs-accuracy-1#10, lens-docs-clarity#8 and added2#69 are the same prose; they are T18.22 and T16.23 and close with this change.

### T03.12 verify.rs literals
**Change.** Delete the dead `(ca, sa)` binding (verify.rs:289/311) and the inert `+ 1e-12` (135): no point falls in the band it exempts across 315 profiles. Scale the centre-path pad by the module: `(3 s_j − m).min(−m)` and `3|s_j| + m` (245-246). Drop `r_low`'s ×1.000001, since acos(1) is defined. Leave FLANK/ROUND/TIP alone: the ring gate's floor is its radius bins, which is T16.12 [added2#112].
**Proof.** `fillet_envelope_error/m` agrees at m = 0.05 and m = 1. Today it is 14× worse at 0.05 (1.20e-7 against 1.08e-8). `rack_simulation` is unchanged. This change can ride with T03.1.

### T03.13 One rolling corner, rack at κ = 0
**Change.** Add `RollingCorner { r_w, κ_c, b_c, phase, ρ, σ }`. With θ = κs, the corner centre is C = (λ s·sinc θ, (r − b_c) + σλκ s²(1 − cos θ)/θ²). C′ and C″ are exact at κ = 0, so only C needs the sinc and versine helpers. Give it one method each for point, polar form, tangent, curvature and θ-slope. It replaces seven copies: tooth.rs:543/562, strength.rs:299/337 and shaper.rs:239/287/340. Keep `verify::fillet_envelope_error`'s centre path and the ring simulation's cutter outside it: they are the instruments. Migration, green at each step:
1. Add the struct and route `ShaperCut` through it. The ring tests stay bit-close.
2. Route `Tooth` and `strength` through it with κ = 0.
3. Delete the duplicates. `the_trochoid_tangent_tends_to_the_racks` and its siblings, which assert < 1e-4 at z_c = 100,000, become a continuity law in κ through 0 plus one finite-difference check of tangent and curvature.

**Proof.** At κ = 0 the arithmetic reduces to the rack's own (k s, r − k b_c), so the golden corpus and the `tests/common` grid must be bit-identical, or within 1e-12 with every difference explained. The shaper tests pass unchanged.

### T03.14 The helical round as an ellipse, in closed form
**Change.** The transverse section of a normal round ρ_n is an ellipse with semi-axes ρ_n/cos β and ρ_n; the code uses a circle of radius ρ_n/cos β (tooth.rs:918). Parameterise the corner by its normal angle φ. Travel is then linear, s(φ) = −q_x − (b_c − q_y)·n_x/n_y, with no solve for either shape, and the correct branch is taken whatever the sign of b_c. The gear point is Q(φ) translated by s along the rolling line and then rotated by −(s − a_c)/r. The corner shape becomes a `RollingCorner` parameter. The fix can also land on the rack alone, if T03.13 is deferred. Consumers to move: `solve_junction`, `check_severed`, `trochoid_theta_slope`, strength.rs:300-346, gear.rs:413/2513, and verify's `cutter_sdf` (map x_n = x_t cos β, which keeps penetration exact) and `fillet_envelope_error`. Three closed forms must change consistently:
- the run-out in `Rack::undercut_shift`, ρ_n(1 − sin α_n);
- the fit cap in the normal plane, with w_tip,n = w_tip,t cos β;
- a_c's corner term, ρ_n/(cos α_n cos β).

Then delete the state.md residual.
**Proof.** At β = 0 the output is identical, because the ellipse reduces to the circle without a branch. At β ≠ 0 it must match an independent swept 3-D rack sliced transversely (`ellipse.py`: implicit-function minimum never negative, with the negative control penetrating −4e-4 to −2e-2). Sizes it removes, per mm of module:
- 20°: fillet 5.4 µm, junction 2.5 µm, x_min under-read 0.0083;
- 45°: 44 µm, 29.5 µm and 0.041.

Canary: the worm (β = 82°) has its round capped at every shift today, though the normal-plane fit (0.472 ≥ 0.38) says the true round fits.
**Notes.** added#17 (the residual called conservative, when the undercut shift is under-read) is closed by this task.

### T03.15 One generator for every member (with a shaper-cut external gear)
**Change.** Target: one `Tooth` generated by one `Tool { depth, tip_round, κ = σ/r_c }` with a signed workpiece count. The ring is σ = −1, and `MeshKind` is derived from the two signs. This collapses:
- `Tooth` and `Ring`'s parallel machinery (involute_at, trochoid_at, solve_junction, sections, sample_section, profile, flank_ends);
- the three dispatch layers: `BuiltMember` (shape.rs:2365, 8 match sites), `auto::Cut` (auto.rs:1068, 10 arms; keep `Pinned` as a flag) and the two `ToothOutline` impls (strength.rs:508/573);
- `check_cut` and `check_ring_cut`, into one simulation over σ and κ that stays built from the basic tool (T03.1).

It makes "a cutter on any member" an input: a shouldered or cluster gear cut by a shaper. A shaper-cut external tooth still needs a junction, an undercut and severing criterion for a pinion cutter, outline, DXF, the strength seam and metrology. Migration:
1. `Mesh::new` takes rack data and signed counts, not `&Tooth`. That removes `as_gear`'s full `Tooth::new` per ring per trial. This step is T05.14 [ring#8].
2. Ring onto the shared generator behind `Tooth`'s API.
3. The rack path becomes the κ = 0 value, bit-checked against today's `Tooth`.
4. Collapse the dispatch types.
5. Expose the cutter choice.

**Proof.** Steps 2–4: bit-identical profiles, clamps and ratings on the grid and the corpus. The shaper convergence tests become equality at κ = 0. The relief and arrangement sweeps are unchanged. `every_search_is_quick_enough_to_type_over` gets faster.
**Notes.** rationale.md (~1005-1020) documents the split as deliberate until "a second shaper-cut member" exists. Commit steps 2–5 together with that feature, not as a standalone rewrite. ring#9 and lens-architecture#8 describe the same split from the ring and shape side.
