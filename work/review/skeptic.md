# skeptic — verifying the audit's own conclusions (branch audit-ablation @ 03f9823)

Method: code reading plus Rust probes (examples in a detached worktree under gear-io/examples, since removed),
gear-cli release output, and Python checks (mpmath 1.4.1 taken from the nix store; ezdxf from the dev shell).
Everything below was run, not inferred, unless marked "read".

## Headline corrections to the audit

1. **The plan's "What not to touch" list contradicts the audit's own confirmed findings in three places.**
   - "malformed JSON or TOML never traps" is false. A TOML train with `meshes[0].a = 99` panics inside
     `gear_io::train::from_toml` (reproduced: `index out of bounds: the len is 3 but the index is 99`,
     shape.rs:1091), and `gear-wasm` `import_train_impl` (lib.rs:971) calls it directly, so the wasm traps.
     plan.md P2 and graph-ops#1 (confirmed, high) say the same thing. The register entry has to be scoped to
     *syntactically* malformed input, or dropped.
   - "Output wire types are generated" is false. `Imported`, `TrainDocument` (core.ts:580-591, outputs of
     `import_train`) and `Note` (strings.svelte.ts:80, a duplicate of the generated wire/core/Note.ts) are
     hand-written. The audit itself confirmed this as tools-ci#10.
   - "The epicyclic search converges" does not hold as an unqualified statement. The shipped Planocentric
     preset refuses with the search on (reproduced: `Mesh(OutsideInvoluteDomain)`). This is added2#94
     (confirmed, high). Planocentric is an epicyclic preset.
   The verified register (T18.28) would freeze these three sentences, so they need correcting before it is built.

2. **"Every length is homogeneous in module to 2e-10" holds only for the core, not for trains or presets.**
   Probe: each of the 10 presets alone at m = 0.1 and 10, compared with m = 1.
   - Planocentric refuses at m = 0.1 ("axis distance below the base-circle limit"). Worm refuses at m = 10
     (the 7 mm literal). Wolfrom's efficiency moves 5.7 points at m = 0.1. Shifts move by up to 0.19.
   - When the nine absolute-mm fields are scaled by m as well (pitch_diameter, face_width, min_tip_width,
     tip_clearance, clearance, tolerance ±, axial_clearance, min_planet_clearance, a manual distance),
     **every preset is homogeneous to ≤1.3e-14**.
   So the core is exactly homogeneous, and every non-homogeneity comes from mm-valued defaults.
   lens-standards#6 names the cause. The new part here is that two shipped presets refuse outright at
   another module. Proposed gate (new): *a law that every preset, with its length fields scaled by m, returns
   identical dimensionless outputs*. It passes today once the fields are scaled, and it would pin the
   module-relative defaults that lens-standards#6 proposes.

3. **Of the 25 "unverified" findings, 25 hold, but about 19 duplicate findings already confirmed.** They should
   be merged, not double-counted:
   - #0 = shape-b#2; #1 = shape-b#1/added2#3; #2 = shape-b#9 = train-mod-a#4 = shape-a#14 (a triplicate
     already); #4 = web#3/added2#79; #5 = gear-io#0/added2#109.
   - #6 ⊂ gear-io#10; #7 = crossed-worm#20; #8 ≈ lens-continuity#0; #9 = tools-ci#16 (+ added2#43);
     #10 ⊂ tools-ci#17.
   - #13 = primitives#11 = lens-unification#5; #14 = graph-ops#1; #15 = added2#101.
   - #18 ≈ ablate-features#1/added2#87; #19 and #20 are the same finding (= added2#94's mechanism).
   - #21 ≈ kinematics-flow#6; #22 ⊂ lens-docs-accuracy-2#1/#4; #23 ≈ added2#111; #12 extends added2#42.
   Genuinely new among them: #3, #11, #16, #17, and #24 (related to added#66, but a different mechanism).

## (a) The 25 unverified findings — verdicts

| id | verdict | evidence / notes |
|---|---|---|
| added3#0 | **confirmed** | Probe: Compound, Edit::Move{4,None}, z4 = 18 → assembly(1) = Some((false,false)). Body 5 (18 on 24/60) gives (24·18+60·18)/54 = 28 ✓, and body 6 has a single mesh. Before the edit: (true,false). Dup shape-b#2. |
| added3#1 | **confirmed** | Probe: the Wolfrom preset alone gives layouts (1,3,Some(false),Some(false)) and notes include part.planets_not_evenly_spaced. The text in all 5 catalogues names "sun and ring". Dup shape-b#1/added2#3. |
| added3#2 | **confirmed** | Probe: Idler, k2 = 0.9 given and relieved, then k0 = 1.1 given and relieved → given [T,F,T], ks [1.1, 0.9, 0.9], so mesh (1,2) sums to 1.8. No note. Addition: even one group per mesh component is not enough on an **odd cycle of external meshes** (the parity forces k = 1 everywhere), so the cure should compute each component's admissible k (1 dof, or 0 on a frustrated cycle) rather than assume 1. |
| added3#3 | confirmed (read) | shape.rs:1399-1408: with no absorber, a 1e-9 relative mismatch between the two meshes returns None, which is reported as train_no_common_distance ("cannot be assembled"). |
| added3#4 | confirmed (read) | GearPanel.svelte:91-93 always uses `result` (solveGear, external) for profile_shift, even when `internal`. Dup web#3/added2#79. |
| added3#5 | **confirmed** | Probe grid z 20..150/5 × ha 1.0..1.5/0.025: relief max +33.3 % (z20, ha1.375), increase max 73.6 % (z150, ha1.5). The message says −24 %..+15 %. Numbers identical to the finding. Dup gear-io#0. |
| added3#6 | confirmed (read) | zh-Hant mixes 負荷 (most), 載荷 (53, 76, 111) and 負載 (65, 415, 416, 511, 512). zh-Hans uses 载荷 consistently. ⊂ gear-io#10. |
| added3#7 | confirmed (read) | arrangements.rs:712. Also NaN for starts > 7 and wrong for module ≠ 1 (crossed-worm#20). |
| added3#8 | **confirmed** | Probe: pair 2/5, 3/7, 4/9 → Err(NoContact); 5/12 solves. relative_curvature returns None where ρ1 ≤ 0, and ContactPath::new does not clip at the interference point. |
| added3#9 | confirmed (read) | check_golden.sh:79-92/135-162. Also known: `--write` with an empty list deletes the store before `cp` fails (added2#43). |
| added3#10 | confirmed | Cargo: a negative `jobs` = CPUs + value. Also `[term] verbose = false` carries the comment "Show warnings during compilation", which is unrelated to what the key does. |
| added3#11 | **confirmed** | Reproduced: at μ = 0.06, fwd 0.98751530 vs bwd 0.98749496 (Δ 2.03e-5); at μ = 0.3, Δ 5.58e-4. Caveat: the backward formula keeps the same path, whereas contact.rs's own argument is that approach and recess swap. Either way the exact balance is asymmetric at O(μ²), so "physically it should be [equal]" is wrong. |
| added3#12 | confirmed (read) | core.ts:678-684, 711-716, 818-824, 852-857. editTrain's doc admits "swallowed as before". |
| added3#13 | confirmed (read) | mesh.rs:766-777 vs strings_en.toml:190/198. Dup primitives#11/lens-unification#5. |
| added3#14 | **confirmed** | Probe: from_toml of the Idler with meshes[0].a = 99 → PANIC. Dup graph-ops#1. |
| added3#15 | confirmed (read) | lib.rs:554-561. Dup added2#101. |
| added3#16 | confirmed (read) | TrainPanel.svelte:1032-1034 vs material.rs:30-39. New. |
| added3#17 | confirmed (read), **understated** | TrainPanel.svelte:278-282 ignores editTrain's return, so a *refusal* (a 'ui.' key) also selects the old last case, not only a silent failure. New. |
| added3#18 | confirmed (read) | grep: no production caller. Extra stale doc: gear-cli main.rs:1542 says `shifts` "drives auto::shifts_for_efficiency", but it uses set_search. Dup ablate-features#1. |
| added3#19/#20 | **confirmed; duplicates of each other** | Read shape.rs:1874-1935: `found` is set before `plan` is replaced. Also `sized(..)?` inside the loop discards `found` on error, and the `unwrap_or_else(settled)` result is still labelled `Searched::Chose`. Probe: Planocentric with search on → OutsideInvoluteDomain. |
| added3#21 | confirmed (read) | flow.rs:305-347: the loss ≥ 0 filter, then max-efficiency. Its comment says "the first consistent one is the answer", which contradicts the code (kinematics-flow#6). |
| added3#22 | confirmed (read) | rationale.md:419 vs auto.rs:739-751. Extra: auto.rs's comment "The tooth is at its widest where the involute starts" is false. s(u) rises for small u (ds/du = 0 at u = 0 and the curvature is positive), by about 1+ψ_b²/2. So `width(0) < min_tip_width → None` can refuse a width that is achievable (a narrow sliver). See lens-docs-accuracy-2#4. |
| added3#23 | **confirmed, and at reachable ε** | Probe: z10 ha1 ε1.7 rel 8.20e-4, z10 ε1.9 4.40e-4, z9 ε1.7 3.68e-4, z9 ε1.9 1.39e-4, exactly as claimed. I first suspected ε ≥ 1.66 is unreachable for z9/10 (ε vs rack ≈ 1.66/1.68), but z9 at ε = 1.55/1.62/1.65 also moves 3.9e-4 to 7.9e-4. The doc claim "Below the band it cannot move at any resolution" (strength.rs:1928-1931) is refuted. Also, 200→800 under-reports: z10 ε1.68 moves 0 at 800 but 1.8e-4 at 12800, so the quadrupling test is a weak detector. The cure should be the bracketed maximiser. |
| added3#24 | confirmed (read), **proposal refined** | ring.rs:943 returns +∞ for both nested cases: pinion-inside-ring (clear) and ring-tip-circle-inside-pinion-tip-circle (a total foul). Fix it at the root in `tip_clearance` by returning a negative margin when `r_a,pinion − R_a,ring ≥ a`, not in TipRoom. Unify with added#31: the same function's *other* early return (half-width ≤ 0 → +∞) is a second instance of "∞ means clear where no crossing was computed". One signed-margin rule cures both. |

## (b) "What not to touch" claims, tested independently

| claim | result |
|---|---|
| Tooth continuous in every input except severing | **Holds** in my sweep. 6 base z (5..40) × 7 inputs (x, ha, hf, ρ, α 14–32°, β 0–45°, k 0.7–1.3). Hausdorff between successive profiles never exceeded 3× the median step, and every scalar (r_j, ρ, ra, rf, L) jump scales with h down to 1e-6 (kinks at clamp onsets, no jumps). Caveat: continuous ≠ valid. Above α = atan(π/(4h_f)) = 32.1° the half-profile's θ passes half_pitch (α 33°: θmax 0.1879 > 0.1848) with no clamp; this is known as tooth-form#0/added#6/lens-tests-geometry#0. |
| Mesh geometry matches mpmath to 1e-11 | **Holds.** 22,408 pairs: z1 4..150, Δz 1..200, x −0.8..2.0, β 0..60°, α_n 14.5/20/30, external and internal. Worst: α_w rel 4.0e-14, a_w rel 6.3e-16, approach/recess abs 4.2e-13 mm (z = 350), ε abs 7.4e-14, backlash (independent route: operating pitch minus two thicknesses) abs 5.8e-13. No domain or path-existence mismatches. The stated tolerance is loose. |
| Power balance in 15,764 fuzzed cases | Not re-fuzzed. On all 72 cases of `gear-cli kinematics`: Σtorque = 0 and loss ≥ 0 everywhere. The energy filter is weaker than documented (added3#21). |
| Homogeneous in module to 2e-10 | Core: holds (train-level 1e-14 once mm fields scale). Train and presets as shipped: **fails** (see headline 2). Minor: half_profile(300) returns 296 vs 297 points for z5 x0.8 β30 at m = 1 vs other m (rounding knife-edge in allocate_by_arc_length). Harmless. |
| Catalogues: identical 524-key sets | **Holds.** 524 each, 0 missing or extra, 0 placeholder mismatches, identical numerals in every message, no Latin words in zh. Strings identical to English are legitimate cognates (pt Torque/Material/Nominal, de Planet, "auto"). |
| Output wire types are generated | **Refuted** (headline 1): Imported, TrainDocument, Note. |
| Malformed JSON/TOML never traps | **Refuted** (headline 1). |
| DXF container meets R2000 | **Holds.** z17/0, 5/0.8, 40/−0.5, 9/1.5 give AC1015 with all 6 sections, 9 tables, *Model_Space/*Paper_Space, unique handles, HANDSEED FFFF above the max handle 0x10E, every 330 owner resolved, and subclass markers present. ezdxf recover and audit report 0 fixes and 0 errors. |
| Epicyclic search converges | **Refuted as stated** (added2#94, reproduced). |

## (c) 12 confirmed-medium findings sampled (random.seed(7))

| id | holds? | fix right? |
|---|---|---|
| tools-ci#7 | yes (ci.yml:17-20 vs 134-137) | yes, `contents: read`. Verifier's low severity is fair. |
| primitives#0 | yes (`gear-cli show 0` panics at gear.rs:564) | yes. The verifier's addition that α > atan(π/4h_f) is already "no shape" is confirmed by my α probe: the bound must be that, not 90°. |
| lens-docs-clarity#0 | yes (README.md:93-94 vs main.rs:9-13) | yes |
| added2#112 | yes, and **worse than stated**: 30/20 x0.8 gives 0.0653 mm; 24/20 x0.5 gives **0.335 mm**; 25/20 x0.6 gives 0.0675; prolate controls 0.0025-0.0027 | yes, with the verifier's note: select curtate cases by condition and assert that at least one is present |
| strength#6 | yes: tool 692.7 vs ISO Z_B·Z_ε·σ_H0 = 616.78 (M1 1.099599, Z_ε 0.890487), ratio 1.1231 | proposal 1 yes; the verifier is right that proposal 2 breaks the closed-form width |
| shape-b#5 | yes (read wiring.rs:234-245) | yes; per-flank sum shared with train-mod-b#4 |
| lens-tests-train#5 | yes (read conditions.rs:361) | the verifier's "dense map from bodies" is the better cure (no branch, no check) |
| train-mod-b#4 | yes (read shape.rs:3373, mod.rs:3328-3345) | yes; per-member per-case flank flag |
| lens-unification#5 | yes (drifted texts seen) | yes |
| added#31 | yes (read ring.rs:956-959) | yes; merge with added3#24 into one signed-margin rule |
| strength#9 | yes (bending-check.html:91,138,380,388) | yes |
| ablate-features#4 | yes (material.rs:179, 348; the only caller takes .value) | the verifier's correction is right (Value::note invariant) |

No sampled verification was wrong. Four of the 12 (tools-ci#7, lens-docs-clarity#0, shape-b#5, ablate-features#4) have a verifier severity of *low* under a top-level "medium". A filter on top-level severity over-counts, so prioritise by the verifier's severity.
