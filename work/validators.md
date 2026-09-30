# Validators, classified by method (Q11)

The owner's rule (plan.md §5, 2026-10-03): an independent check has lasting value only
if it applies a **fundamentally different method** — another formulation, a numerical
solve against a closed form, an exact solution, published data, another parser. A
**same-method mirror** (the crate's formulas transcribed into another language)
confirms only the transcription, carries its own upkeep and passes its flaws on.
Mirrors are retired; what they caught or could catch becomes a Rust law that stands
alone. Evidence from this tree: the ring's cut simulation once derived the cutter's
tooth from the ring's and reported 2.7 µm on a cutter 0.44 mm out of place
(`corrections.md`, internal gears) — a check built from the thing under test.

"Caught" is what the check's history records finding in the crate (commit or
`corrections.md`); "upkeep" is what a model change costs it.

## Scripts in `tools/` that check the model

| Check | Checks | Method against the crate's | Caught | Upkeep | Verdict |
|---|---|---|---|---|---|
| `train_kinematics.py` (CI) | every body's exact speed in `golden/kinematics.txt`, 27 sections | rigid-body velocities along the base circles' common tangent, mesh sense from which tangent exists; crate: signed-count rows `z_a(ω_a−ω_f)+z_b(ω_b−ω_f)=0` — **different** | a frame coefficient swapped in `kinematics.rs`: 14 sections (e1dc809) | a layout per new topology | **keep**; its `crate_rows` half (the row formula restated, held to the layout, never reading the crate) and its virtual-work torques (the oracle checking itself) **removed** |
| `breakaway.py` (CI) | each preset's path efficiency both ways, breakaway included, in `golden/graph.txt` | carrier-frame power flow, `η₀^w` with `w` from the ideal flow, no search; crate: `2^M` assignments, filters, max — **different** | flow.rs taking the minimum (e1dc809); the rounding breakaway (`corrections.md`, friction balance) | a topology per preset | **keep** the closed form; its "flow" half, flow.rs's search written again, **removed** |
| `crossed_path.py` (CI) | the crossed line of action and zone; every row of `golden/crossed_17_23_90.txt` | flanks as parametric surfaces, normals by numerical differentiation, one-sidedness from the helicoid's own parameter; crate: a construction in lines — **different** | the two-sided band (3.161 vs 1.829 on 9/37); "one line" in the docs (eight, two the path) | none per model change | **keep** |
| `validate_dxf.py` (CI) | an exported DXF's structure and every arc | ezdxf and raw tags; crate: its own writer — **different parser** | every arc bulged off its chord, 5.2 mm off the axis (9dd0945, T04.1). It missed the missing BLOCKS/OBJECTS that ezdxf invents on load, which SOLIDWORKS found; the structure is read raw now | none | **keep** |
| `worm_flank_curvature.py` (hand) | a worm flank's principal curvatures; what ZI/ZN/ZA costs | first and second fundamental forms of the surface, numerically; crate: developable-helicoid closed form — **different** | the ZN/ZI contact-stress figures in `reference.md#crossed-axes` | none | **keep** |
| `iso_6336_3_stack.py` (hand; its block gated by `check_figures`) | the ratings against ISO 6336-2/-3, after reproducing the tool's ISO set to 1e-8 | ISO Method B and 6336-2 in closed form, published; crate: DB on the generated tooth — **different (published method)** | the bias record against ISO (state.md) | its grid | **keep** (an analysis) |
| `fillet_bem.py` · `shoulder_trefftz.py` (hand; bias tables gated) | the default and ISO ratings against the exact elastic peak | plane-strain BEM (Kelvin kernel); Kolosov–Muskhelishvili Trefftz with lightning poles for the canary; crate: DB fit — **different** | DB unconservative: −15 % median on external fillets, every ring under (Q10) | re-solve when teeth move (`--run`) | **keep** |
| `hula_kinematics.py` (CI) | `golden/hula_18_0.2.txt`'s ratio and speeds; Willis over 32 arrangements | no slip at the pitch points (different) on one record; the other half `closed()` = the crate's `z₂z₄/D` held to it, never the crate — **half mirror** | a frame swap on its one record (e1dc809), also caught by `train_kinematics.py`'s `hula` section | a record and a formula | **retired** → Rust law 1 |
| `helical_measurement.py` (hand) | a helical ball's and span's contact | Newton on the flank surface — different, **and already ported**: `tests/common`'s `Helicoid` with two laws over the helical grid (3788a97, which failed at its base) | T09.2's contact in the base tangent plane (via the port) | a second copy of the port | **retired**: the prototype of a port |
| `first_yield.py` (hand) | `C` at the line and circle ends against `gear-cli iso` | the line field is McEwen's closed form as `hertz.rs` writes it — **mirror**; the circle's closed form and Johnson's 1.79 / 1.60 are already Rust laws (`the_summed_field_is_the_circles_closed_form`, `…_johnsons_at_both_ends`) | nothing recorded | a second copy of the line field | **retired** → Rust law 2 |
| `bending_gate.py` (CI) | the default DB rating over 1,714 teeth (`gear-cli bendgrid`) to a derived tolerance | the same fit, fillet envelope, section rule, ramp, virtual count and round cap, written again — **mirror**, with a different *search* (brute force, bisection) | the flank's one-bracket search (5.9 % low), the ramp's 204-sample maximum (1.4e-3), the flat arccosine (d3f168e) — each a Rust law since | rewritten with every rating change: 655, 237 and 93 lines changed in its three rating commits | **retired** → see below |

## Other code in `tools/`
Repository gates, not checks of the model, so outside the rule and kept:
`check_all.sh`, `ci_steps.py`, `check_doc_links.py`, `check_strings.py`, `check_units.py`,
`check_literals.py`, `check_absence.py` (with `rust_source.py`, `sources.py`),
`check_figures.py`, `check_golden.sh` (a change detector), `check_identity.sh` (bit
identity), `check_wasm.sh` + `wasm_probe.mjs` (the payload run), `check_bindings.sh`,
`mutants.sh`, `line_census.py`, `build_wasm.sh`, `cargo_target_dir.sh`.
`handoff_inbound/*.py`: reference only, the regression fixtures' source (decision 12).
`work/review/unifier_blt_flow.py`: redesign F's prototype; it goes when F is ported.

## Oracles in the Rust tests (all kept: each a different method)
| Oracle | Method against the crate's |
|---|---|
| `verify.rs` + `tests/rack_simulation.rs` | the cut simulated from the cutter alone, bounded from both sides; crate: closed-form envelope |
| `tests/common` `Helicoid` | ball and span contact by Newton on the flank surface; crate: closed forms |
| `tests/bending.rs` `rack_limit`, `parabola_rack_limit` | the exact z → ∞ tooth in closed form; crate: generated teeth |
| `tests/bending.rs` section and continuity laws | brute force over each curve (800 samples, golden section), bisected switches |
| `strength.rs` `the_swept_maximum_is_the_greatest_over_the_cycle` | a 20,000-point sweep; crate: pieces closed by golden section |
| `hertz.rs` circle and (new) line laws | Boussinesq / Flamant summed over the patch; crate: closed forms |
| `planetary::power`, `carrier_driven_efficiency` | Pennestrì's closed form; crate: the per-mesh `2^M` flow |
| metrology pin laws | distance to the generated flank, measured; crate: closed-form seat |
| `testing.rs` | naive numerical quadrature and differences; crate: closed forms |
| `tests/regression.rs` | pinned values of the prior implementation (same method, itself validated by rack simulation): a regression fixture, not a check of method |

## `gear-cli` instruments that exist to compare
`matrix` (the ISO sets on the generated tooth, a published method: kept), `iso` (feeds
`iso_6336_3_stack.py`; its `yield` line is now a corpus record only), `fillet` (feeds
`fillet_bem.py`), `verify` (sweeps `verify.rs`), `identity` (bit identity), `sharingbias`
(the held section's bias, a size and sign, not a check), `bendgrid` (fed the gate; kept as
the corpus's record of the rating across its domain and the source of the ramp figures
state.md quotes), `dump`, `meshsweep`, `hulasweep`, `hulaband` (recorded analyses).

## New laws, and the faults they fail on
| Law | Replaces | Planted fault | Fails | Passes at head |
|---|---|---|---|---|
| 1 `a_hulas_speeds_are_its_pitch_points_rolling_without_slip` (arrangements.rs): 32 arrangements, exact, through `hula` and `Train::motion` | `hula_kinematics.py` | the wobble ring's negative count dropped in `arrangements::hula` | yes | yes |
| 2 `the_line_field_is_flamants_load_summed_over_the_strip` (hertz.rs): McEwen against Flamant by Gauss–Chebyshev, to `10·400·ε` | `first_yield.py`'s line half | McEwen's `2ζ` → `(2 + 1e-11)ζ` (Johnson's law passes it) | yes | yes |
| 3 `a_helical_members_virtual_spur_is_its_normal_section` (bending.rs): the virtual pitch radius is the normal section's osculating radius, from three points, to 1e-10; external and ring | the gate's virtual count, which held externals only | ISO 2019's count on a ring (the suite: silent); `z_n` 1e-9 off (10× the tolerance) | yes; yes | yes |
| 4 `the_load_angle_is_the_roll_off_the_half_base_angle` (bending.rs): the load angle is `abs(u − ψ_b)` to `2⁸ε·r/(r_b u)`, found and moved sections, near a square load | the gate's square-load rows | the angle read by an arccosine (the suite: silent) | yes (1.9e-11 off at 3e-6) | yes |
| 5 `the_swept_maximum_is_the_greatest_over_the_cycle` (strength.rs), strengthened: the flank's end loaded at its own roll, and asserted reached | the gate's far-end rows | the far end rounded off the flank (the suite: silent — the law read the end through the same rounding) | yes | yes |
| 6 `a_path_short_of_the_tip_loads_where_more_contact_would` (bending.rs): unshared, `(ε, s)` rates as `(ε + s, 0)` to `2⁸ε`, spur and helical | the gate's `short` rows | `short` not scaled to the normal plane (the suite: one incidental failure) | yes | yes |
| 7 `each_curve_offers_its_least_and_the_highest_rated_governs`, strengthened: every section's `ρ_f` is the fillet's brute-force least radius | the gate's notch radius | `ρ_f` read at the section (the suite: two figure canaries only) | yes | yes |
| 8 `form_factor_converges_to_the_rack_limit`, strengthened: the axial term against the rack's closed form, `sin α / ((s_Fn/m) cos α)` | the gate's axial figure | the axial term without `cos α_n` (the suite: one figure canary) | yes (0.1459 against 0.1546 at 20°) | yes |

The trimmed scripts alone, on the crate planted and its corpus file re-recorded: flow.rs
keeping the least consistent assignment → `breakaway.py` exits 1 (the Wolfrom's forward
48.88 % read as 0); `kinematics.rs`'s frame coefficient added, not subtracted →
`train_kinematics.py` exits 1 (16 sections). The removed halves added nothing: `crate_rows`
never read the crate, and the flow half was held to the same recorded figures at the same
tolerance as the closed form, which reaches them without the crate's search.

## bending_gate.py: retired
**Method.** A rebuild by the crate's own method: the same AGMA fit, fillet envelope,
involute flank, section rule, ramp, virtual count and round cap, each rule written a second
time by the same hand. A wrong rule is written twice and agrees with itself, and the one
fault that matters most — the fit's own error, DB 15 % unconservative — is invisible to it;
`fillet_bem.py` found that. Its value was its **search** (brute force and bisection where the
crate brackets), which found three faults (the flank's one-bracket search, 5.9 % low; the
ramp's 204-sample maximum, 1.4e-3; the flat arccosine). Its own search was once the one wrong
(a 600-sample flank search stepped over the least under a vanishing tip land, 64dddcd) and
was fixed by adopting the crate's bracket, a closer mirror.

**What it held, planted at the base (9302ecc) against the whole suite**, and what holds it now:

| Gate's plant | The suite at the base fails | Held after this package by |
|---|---|---|
| the section searched afresh under the ramp | `a_pointed_tooth_is_rated_at_its_root`, `the_rating_is_continuous_across_the_pointed_limit`, `the_swept_maximum_…` | those laws |
| `H` + 0.15 %, `H` + 1e-9, `L` + 0.15 % | `each_curve_offers_…` (AGMA's fit from the standard at 4ε) | that law |
| `ρ_F` for `ρ_f` inside `K_f` | `each_curve_offers_…`, two figure canaries | that law |
| `ρ_f` read at the section | two figure canaries only | law 7 |
| ISO 2019's virtual count (external) | `the_virtual_spur_gear_has_the_iso_tooth_count_…` | that test and law 3 (rings too) |
| the round capped at 0.94 | six tests (hula studies, train searches, the regression fixtures) | those tests and the corpus |
| the fillet's notch factor on the flank | nine, `each_curve_offers_…` among them | those laws |
| the highest `Y_F` governing | seven, the section rule and continuity laws among them | those laws |
| tangencies alone | eight, `each_curve_offers_…` among them | those laws |
| `ε_α` for `ε_αn` at the load point | `the_bending_load_point_uses_the_virtual_contact_ratio` and one more | those laws |
| the ramp's maximum from samples | `the_swept_maximum_…` | that law |
| the arccosine load angle | **none** | law 4 |
| the flank's far end rounded off | **none** | law 5 |
| `short` not scaled | one, incidentally | law 6 |
| the axial term without `cos α_n` | one figure canary | law 8 |

**Upkeep.** 1,058 lines, rewritten in each of the three rating commits since it landed (655,
237 and 93 lines changed), and every rule change written twice.

**Verdict: retired.** Everything it caught or planted is now a Rust law or a test that fails
on it; its search survives as the laws' own brute force (800-sample curves with golden
section, 20,000-point sweeps, bisected switches). `gear-cli bendgrid` stays in the corpus as
the record of the rating over 1,714 teeth and the source of the ramp figures `state.md`
quotes. The rating's independent check is `fillet_bem.py`: a different method, and the one
that found the fit's bias.
