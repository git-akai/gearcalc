## T11 — Kinematics, power flow and performance

**Why.** The train solve rests on two exact-looking engines that do not scale. Their failures are silent, or they are reported as something else. The power flow tries all 2^M mesh-direction assignments over the *whole* train's meshes. That takes 1.7–2.9 s at 16 meshes. In release/wasm the `1u32 << m` bound wraps at M ≥ 32, and a 0.98^32 = 0.52 chain then reports back-drive efficiency 0 or "load not reacted". The exact kinematics cross-multiplies full denominators, so the elimination overflows i128 on 9 chained 17/43 pairs, whose answer is only ~1e14 (51–67 bits). That overflow is reported as "case underdetermined, short 0" beside a headline ratio that solved. The flow's direction is also seeded from outside the model. A still port's ±1 "impending motion" reads the stale seed of an *automatic* torque, and relief writes that seed back. The result is a stable wrong fixed point: +3.4 % on a pair, and a solved worm turned into a refusal. The same ±1 is added in rpm, so answers change below ~1 rpm. The UI solves synchronously on every keystroke and hover, and the "a full solve is microseconds" premise behind that is false: a lone preset takes 1–6 ms in wasm and a 6-preset mix 51 ms. The fixes follow one rule: make each answer a function of the stated inputs alone. That means independent of seeds, order, speed scale and bit width, with a named refusal where a limit is real, and operation counts rather than wall-clock times to gate the cost.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T11.1 Bound the flow enumeration and refuse by name | lens-performance#1, kinematics-flow#1, lens-unification#6, lens-performance#0 | high | S | — |
| T11.2 A still port's impending motion: from given torques only, as a tie-break | train-mod-b#0, added2#4 | high | M | — |
| T11.3 One stated branch-selection rule in the flow | added3#21 (unverified) | low | S | — |
| T11.4 Seeded fixed-point flow, per component, enumeration as oracle | lens-performance#0, kinematics-flow#1, lens-unification#6, lens-architecture#7 | high | L | T11.1–T11.3 |
| T11.5 gcd-first `Ratio` addition | primitives#3 | medium | S | — |
| T11.6 Report kinematic overflow and conflict by name | lens-performance#2, kinematics-flow#0 | medium | S | T11.5 |
| T11.7 Order-independent exact elimination | kinematics-flow#9 | low | M | T11.5, T11.6 |
| T11.8 Compute each exact figure once: play coefficients, `per_tooth` | lens-performance#5, train-mod-b#11 | low | S (+M) | T11.7 for the cache |
| T11.9 Signed `carrier_driven_efficiency`, or none | kinematics-flow#3 | medium | S | — |
| T11.10 The flow listing oriented by signed power | edit-ops#2 | medium | M | T11.4 |
| T11.11 One copy of the string catalogues in the wasm | lens-performance#6 | medium | S | — |
| T11.12 Operation-count gates in place of wall clock | lens-numerical-robustness#8 | medium | M | T11.4 |
| T11.13 Solve off the main thread; state a measured budget | lens-performance#4 | medium | M | T11.4, T11.14 |
| T11.14 Dry runs and offers without redundant solves | edit-ops#8, edit-ops#9, lens-performance#8 | low | S | — |
| T11.15 Honest flow tolerances | kinematics-flow#7 | low | S | T11.4 |
| T11.16 Parse the default material library once | lens-performance#9 | low | S | — |
| T11.17 Cut an ordinary gear's tooth once | added2#92 | low | S | — |

### T11.1 Bound the flow enumeration and refuse by name
**Change.** `train/flow.rs:223,226`: replace `0..(1u32 << m)` and `1 << k` with a `u64` counter behind a named cap, `const MAX_ENUMERATED: usize`, stated with its cost. Above the cap, return a new `Refused::TooManyMeshes { meshes }`. `solve_parts` carries it as a note with a key (5 × `strings_*.toml`), not as `load_shared` / `load_not_reacted`. This lands before T11.4 and stays as the cap on T11.4's fallback subset. A `u64` alone only moves the cliff. The cap is lens-performance#0's correctness half (the wrap at M ≥ 32); its cost half, 2^M work on every keystroke, is T11.4 in Phase 3.
**Proof.** Written first; it fails today in release:
- A synthetic series chain of m external meshes, η = 0.98 both ways, driven from the far end: efficiency equals 0.98^m at m = 32, 33 and 40. Today it gives `Err(Inconsistent)`.
- 32 chained `[17,19]/[19,17]` pairs: the back-driving path equals the forward one (0.505138). Today it gives 0.
- 16 and 17 chained planetary sets: backward efficiency ≈ η_set^n (0.686, 0.670). Today it gives 0.0 or unsolved.
- Run the test under `--release`: the debug profile panics on the shift instead, so a debug-only test cannot see the wrap.

### T11.2 A still port's impending motion: from given torques only, as a tie-break
**Change.** `train/mod.rs:4410-4419,4437,4485`. Today a load given at 0 rpm weighs `if l.torque.manual < 0.0 {-1} else {1}`, even when the torque is automatic. That weight is added into `unit` in rpm units. Make two changes in one edit:
1. Choose each still port's sign so that the *given* torques do positive work on the impending motion. Read a port's own torque sign only when that torque is given. If no given torque does work, return `NothingDrives` as now.
2. Keep the impending motion out of `unit`. Pass the given-speed motion and the still-port motion separately. Take each mesh's direction from the relative speed under the given speeds, and fall back to the still motion only where that speed is exactly zero (lexicographic, i.e. `speeds + ε·still`, ε → 0).

`relieve_case` (mod.rs:3660-3666) can then keep writing seeds, because nothing reads them.
**Proof.** Write these laws first; each fails today:
- (a) Seed invariance. Every preset and case is unchanged when every automatic figure's seed is set to ±1e-9, ±1 or ±1e3. The `stale2` pair fails it: 4.975099 against 5.143957. The Worm fails it too: solved against `load_shared`.
- (b) Relief idempotence: `solve_train ∘ relieve_case == solve_train` on the answer. It fails on both fixtures above.
- (c) Speed-scale invariance: scaling every given speed by 1e-3, 1 or 1e3 changes no torque or efficiency. The `mixweight` planetary gives −13.675453 against −14.333568 at 0.1 rpm.
- (d) Stall closed form: output = T_in·|i|·η_fwd, i.e. 2·2.5294·0.98345 on the pair and 2·40·0.61805 on the worm.

The mirror law (negate every given torque and speed) belongs with lens-tests-train#1.

### T11.3 One stated branch-selection rule in the flow
**Change.** `flow.rs:297-347` rejects only a branch whose total loss is negative, then keeps the highest efficiency. The documented condition in `docs/reference.md:1805-1830` is that the output absorbs. Implement the rule the model means: reject a branch in which no body absorbs power beyond `ZERO`. If every consistent branch is like that, the flow holds. Keep `max` only as a tie-break among the surviving branches, or drop it if the law below shows they agree. Do **not** require every derived port to absorb: a Wolfrom with the carrier derived correctly delivers +22.05 W (kinematics-flow#6). Rewrite the two comments (flow.rs:307-309, 341-345) and the reference.md paragraph in the same change; those docs are T18.6 [kinematics-flow#6, lens-performance#11, added2#9, added2#96]. T11.4 needs this rule as its fixed target.
**Proof.** The full suite stays green; the finding reports that 604 tests stayed green with the max replaced. Add a debug assertion that all *surviving* branches agree to tolerance. This finding is unverified: confirm the assertion holds over every preset and back-driven case before removing `max`.

### T11.4 Seeded fixed-point flow, per component, enumeration as oracle
**Change.** Replace the 2^M loop in `flow::solve`. The call sites are `solve_parts` (mod.rs:4486), once per case, and `paths_of`/`flowing` (mod.rs:3420, 3497-3505), four times per path.
1. **Split into components** that share no unknown torque. Series parts decouple through a known shaft torque. Leave lossless couplings out of the enumerated set. This alone turns 2^ΣM into Σ2^M_i.
2. **Seed the directions.** Solve the lossless flow (every factor 1), which needs no assignment. Set each mesh's driver from the sign of its driving power; a sign of 0 keeps the previous choice.
3. **Iterate.** Re-solve with loss, flip every mesh whose driving power is negative beyond `ZERO`, and repeat until consistent, with at most M+1 rounds. Then apply T11.3's filters.
4. **Fall back.** Only if the iteration cycles or ends inconsistent, enumerate the subset L of meshes that can lock (η ≤ 0 or non-finite one way) or whose lossless power is within `ZERO·P_in`, holding the rest at their seed. L is capped by T11.1.
5. **Clean up.** Move the full enumeration behind `#[cfg(test)]` as the oracle. Hoist the per-assignment `Vec`s out of the loop. Rewrite the flow.rs header (lines 10-19), which presents 2^M as the method.

A block-cut tree over parts (lens-architecture#7) is warranted only if step 4 proves insufficient on split-power loops. The kinematics half of lens-architecture#7 is T11.6/T11.7 and not part of this task.
**Proof.**
- An oracle law over every preset, every arrangement, every chained pair of presets, both load directions, static friction, the self-locking worms and random η in (0,1]. The new solve must equal the enumeration in efficiency (1e-12), shaft torques (1e-9), directions and the locked verdict. It must reach the enumeration's chosen branch, not merely *a* consistent one.
- The existing Pennestrì η₀^w and hula laws pass unchanged.
- T11.1's m = 32/40 law passes with no fallback.
- An operation-count gate (T11.12): a 16-mesh planetary chain takes at most a small fixed number of eliminations. The prototype took 72 µs against 2.09 s.

**Notes.** Evidence for the method: a mirror of the crate's flow agreed in 147,000/147,000 and 392,000/392,000 swept cases, and inside every `flow::solve` of the gear-core suite (604 tests). Without the fallback, the iteration found nothing in 26 locked or Inconsistent calls, and in 7,113 compound and 2,115 Ravigneaux cases of an earlier sweep. The fallback is required.
`breakaway.py` reimplements this same enumeration and is no oracle for it (kinematics-flow#5).

### T11.5 gcd-first `Ratio` addition
**Change.** `ratio.rs:136`: switch `checked_add` to Henrici's form. Let g = gcd(d₁, d₂). Then t = n₁·(d₂/g) + n₂·(d₁/g), g₂ = gcd(t, g), and the result is (t/g₂)/((d₁/g₂)·(d₂/g)). The result is unique and normalised, so it is bit-identical wherever the old form succeeded.
- Delete `cmp_checked`, which has no caller (added#7).
- Correct ratio.rs:26-28 ("i128 holds any train anyone will build") to state the real limit.
- Rewrite `a_conflict_a_missing_shaft_and_an_overflow_are_each_named` (train/mod.rs:7730-7746). Its 4e9-tooth pairs telescope to a 31-bit answer 2e9/(2e9+3), so it asserts the artefact: it fails with the fix. Use non-telescoping distinct large primes whose product truly exceeds 127 bits, and fix its doc.

**Proof.**
- A property test over random reduced fractions: the new add equals the naive add wherever the naive one succeeds. It also returns `Some(1/(15·2⁵⁹))` for 1/(3·2⁶²) + 1/(5·2⁶²), which is `None` today.
- A chain of 9–12 × 17/43 pairs solves to exactly (43/17)^n; today it overflows at 9.
- `check_golden` shows no diff.

The fix alone moves the refusal from 9 to 13 pairs, and from 51–67-bit answers to 94–120-bit answers.

### T11.6 Report kinematic overflow and conflict by name
**Change.** `train/mod.rs:4431-4455` maps every `Err` from `motion_in` to `determined = false` and emits `TRAIN_CASE_UNDERDETERMINED {short: 0}`. Change it to three arms:
- `Err(Refusal::Overflow)` → the existing too-large refusal (`ERROR_TRAIN_OVERFLOW`, as `Train::motion` raises it).
- `Err(Conflicts(b))` → an over-determined note naming `b`.
- `Ok(non-unique)` → `case_underdetermined` alone.

Remove the `.ok()?` in `motion_from` and `per_tooth` (mod.rs:3401-3406, 3437-3456), which make the path vanish, and raise the same note there. This is the fix that removes the class; T11.5 only moves the threshold.
**Proof.** On the tree with T11.5 applied, a chain of 13 Spur presets gives `case_underdetermined{short:0}` and no path while `motion()` is Ok. After this task it gives either the path or the overflow note, never "underdetermined". The rewritten T11.5 test asserts the overflow note.

### T11.7 Order-independent exact elimination
**Change.** `kinematics.rs:437,567,660`. `System::solve` absorbs the structural rows and then the conditions in `first` order, and the production callers pass the driver first (mod.rs:3404, 3445, 4432; conditions.rs:468, 506). That order overflows at n = 9 where body order solves. Make the answer independent of order: solve with the unit condition rows absorbed first, and replay in the stated order only when a conflict appears, so the conflicting condition is still named (`check_parts` depends on this). Fraction-free Bareiss elimination over i128 is the alternative if replay proves awkward. The naive "conditions first" alone would report a locked train as `NoMotion` with no condition named. Until this lands, amend the doc at kinematics.rs:437 ("The order changes no answer") to say that order can decide overflow.
**Proof.** Law: `motion == motion_in` over every permutation of `first`, for chains of up to 12 pairs z ∈ 12..97 driven from either end. It fails today at n = 9: body order returns `Ok(-118587876497/502592611936843)` and driver first returns `Overflow`. The existing conflict-naming tests pass unchanged.

### T11.8 Compute each exact figure once: play coefficients, `per_tooth`
**Change.**
- (S) In `backlash_at` (mod.rs:3459-3483), compute `c: Vec<_> = (0..M).map(|k| coefficient(k, read, from))` once, before `Backlash::banded`. It is currently recomputed at each of the 3 band points, a full exact re-reduction each time.
- (S) Move `PathReport::per_tooth` (serde- and ts-skipped, mod.rs:3920-3921) out of `paths_of` into `pub fn ratio_per_tooth(train, parts, path)`, called by the harness (`gear-cli` kinematics.rs:453, graph.rs:109) and `Alone`. It costs 5–20 % of a small preset's solve for a figure only the harness reads.
- (M, only if profiling after T11.4 shows it dominant) Keep the structural rows' reduction on `System` once, absorb conditions on a clone, and solve all M play right-hand sides in one augmented elimination. The prototype memo took 8 planetary sets from 58.6 to 18.1 ms.

The faked distance band that feeds `banded` is added2#12's to fix.
**Proof.** The golden corpus shows zero diff, since the harness still prints `per_tooth` and backlash. A law that the cached `System::play` equals the uncached one exactly (`Ratio` equality) on every preset and chain.

### T11.9 Signed `carrier_driven_efficiency`, or none
**Change.** `planetary.rs:305,326`: `1/(|R|(1−η₀)+η₀)` holds only for R > 1. For R < 0 the held member drives in the carrier frame, and the correct form is 1/(1+(1−R)(1/η₀−1)). The preferred fix is to derive the value from Pennestrì's `power()` with the carrier as input, which already chooses w, and delete the standalone formula. It has only test callers (arrangements.rs:1664, 1693, 1712). Correct `docs/reference.md:712,1990`, which offer it for the whole 3K family.
**Proof.** Add the crate's own hula fixture [19,18,18,17] (R = −8.5) to `the_hula_efficiency_is_the_reduction_and_the_meshes`: the flow gives 0.927315 and the |R| form 0.942175, so the test fails today. Add a swapped-ring Wolfrom (rings 69/72, planets 21/24, R = −10.5): the independent value is 0.809917 against 0.840336. Both must agree with the flow to 1e-9.

### T11.10 The flow listing oriented by signed power
**Change.** `groupings.rs:93,187-194,256-341`: `share()` takes `power_through.abs()` and walks depth-first from the load, so a `FlowRow::Mesh`'s direction is the walk's order, not the power's. Where two paths merge, the second is written backwards: 83 % of a planetary split's power is shown flowing 5 → 3 when it flows 3 → 5.
- Keep `power_through` signed on `MeshCase`, or record the driver member from the flow solve's assignment.
- Walk only from driver to driven.
- Add `FlowRow::Merge { mesh, from }` for a body reached by a second carrying mesh, and relax the "every body once" law to allow a body to be the target of more than one row.
- Share one `ZERO` between flow and groupings (lens-magic-numbers#8).

This is display only, not a rating error.
**Proof.** Law over every preset, chain and split train: every `Mesh{mesh,to}` in a solved case has `to` as that mesh's driven side, i.e. positive `on_body·speed`. The Planetary + pairs 1→5 and 3→5 fixture fails today. The ordering tests of T16.28 [edit-ops#3] need this split train too.

### T11.11 One copy of the string catalogues in the wasm
**Change.** `gear-io/src/strings.rs:101,135`: change `pub const LANGUAGES` to `pub static`, and `const EN` to `fn en() -> &'static str { LANGUAGES[0].source }`. Each of the five catalogues is currently duplicated (172,534 B raw, ~53 KB gzip, 11 % of the 1,542,209 B module). Lazy per-language catalogues are a separate design change, because every Note renders through the core's catalogue.
**Proof.** Add a size assertion to `tools/check_wasm.sh` that no catalogue's bytes occur more than once in the module. It fails today with count 2. Answers unchanged. Record payload bytes (raw and gzip) in its output, not times (lens-performance#12).

### T11.12 Operation-count gates in place of wall clock
**Change.** Replace `every_search_is_quick_enough_to_type_over` (mod.rs:9997-10042) with deterministic counts. It times only a pair, `planetary(12,30,72,3)` and a hula, and flakes under load (lens-tests-train#6).
- Search: objective evaluations and tooth/gear builds per search, for **every** `Preset::ALL` with each mesh's search on.
- Flow: eliminations per `flow::solve`, for T11.4's 16-mesh chain.

The search bullet lands in Phase 1 as T16.8; the flow bullet lands with T11.4. Fixing the search's cost is T12.10 [lens-performance#3, auto-search#2]; this task supplies the gate that holds it.
**Proof.** The gate fails today on Layshaft with search on: 1.37–1.78 s native, against 0.57 ms with search off. It also covers Idler, Compound and MeshedPlanets (150–315 ms). Random layshafts refuse `NoCommonDistance` after 200–310 ms against ~50 µs with search off; an evaluation budget with a note is the general fix there.

### T11.13 Solve off the main thread; state a measured budget
**Change.** `web/src/TrainPanel.svelte:102-104` derives `solveTrain` synchronously on every change, and `Offers.svelte:69` runs `previewEdit` on hover. After T11.4 and T11.14, move the wasm module into a Web Worker. Post `{seq, train}`, drop stale replies, and keep the last good result with a "computing" mark. The core stays pure, and the worker is only a transport. Debounce the hover preview. Reword `docs/rationale.md:65,1771` and CLAUDE.md rule 3 to state a measured budget, e.g. a lone preset at 1–6 ms in the browser, instead of "microseconds". The rule "inputs are the only state" does not depend on that word. The stale `[profile.wasm]` table is lens-performance#12.
**Proof.** A node benchmark beside `tools/wasm_probe.mjs` records `solve_train` per preset and for mix4/mix6. It records figures only, and T11.12's counts are the gate. Manual: typing into a 6-preset train stays under 50 ms of input latency.

### T11.14 Dry runs and offers without redundant solves
**Change.**
- `preview.rs:65-67`: `unchanged()` Debug-formats both trains per candidate, which is ~73 % of `offers()`'s 413 ms on Layshaft ×6. Format `self` once per `offers()` call. Do not derive `PartialEq`: NaN ≠ NaN and 0.0 == −0.0 change what "unchanged" means.
- `TrainPanel.svelte:2173,2177`: call `offersAt` once per target and pass the adds and verbs lists to the two `<Offers>`.
- `preview.rs:110-111`: keep a one-entry cache of the before-solve in `gear-wasm`, keyed by the train JSON, or split out a `compare(before_result, after_result)` the panel feeds. The before-solve is ~⅓ of a hover whose edit solves and ~100 % of one that fails fast.
- Do **not** skip `rate` for `after`: `rate()` can return `TrainError`, and `unsolved` would go silent.

**Proof.** `an_offer_is_its_edit`, `every_edit_the_train_makes_is_offered` and preview.rs's laws are unchanged. A test that `preview` with a supplied before-result equals `preview` computing it.

### T11.15 Honest flow tolerances
**Change.** In `flow.rs:171,300,366,393-396`:
- Rename `least_squares_exact`, which is float Gauss–Jordan, to `gauss_jordan`.
- Name the absolute pivot `1e-12` and the relative residual `1e-9` in consts with their derivation (n·ε·cond), or make the pivot relative to the column.
- Drop the `fold(1.0, max)` floor (ablate-constants-rating#11).

No wrong decision is known at realistic tooth counts. A structural rank test, if wanted, must come from each assignment's own sparsity: a holding mesh (η ≤ 0) changes the incidence, so the lossless rows cannot decide it. The pivot floor's refusal of long spur chains is lens-performance#10, whose cure is two-sided scaling.
**Proof.** The suite and golden corpus are unchanged. A law: the flow is invariant to scaling every tooth count by 10.

### T11.16 Parse the default material library once
**Change.** Cache `gear_io::default_library()` (materials.rs:79) in a `static OnceLock<MaterialLibrary>`, with a `&'static` accessor for the three borrowing callers (`gear-wasm/src/lib.rs:835,1391,1621`). It is 50–55 µs native per call, ~15 % of a lone Spur solve in wasm.
**Proof.** The materials tests pass. The wasm probe shows `solve_train(materials=null)` ≈ `solve_train(materials=sent)`.

### T11.17 Cut an ordinary gear's tooth once
**Change.** `gear.rs:207` recuts the mean tooth at Δx = 0, identical to `teeth[0]`, so `Gear::new` takes 2.1× a single cut (8.4× at z = 200). Take `mean` as the clone of the tooth whose shift equals `params.profile_shift` exactly, falling back to `Tooth::cut_by`, and merge the clamps after the clone as now. This also covers an eccentric gear's k = z/4 tooth.
**Proof.**
- `check_golden` shows zero diff: the inputs are identical, so the output is bit-identical.
- Extend `an_ordinary_gear_still_generates_one_tooth` (gear.rs:2615) with a `cfg(test)` cut counter asserting one cut. It counts 2 today.

### Declined
None.
