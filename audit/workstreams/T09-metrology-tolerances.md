## T09 — Metrology and tolerances

**Why.** Two geometric facts and one boundary rule were left to convention, and no check could fail on any of them. First, the JGMA table prints its bands as (lo, hi], with only the first block closed at lo. The code reads every band as [lo, hi) and patches the diameter edges with +0.01, so the table now follows two conventions at once. As a result the shipped default gear (m = 1, z = 9) is given fine 0 at 6.3/18 µm where the printed row says 6.0/18, m = 1.6 drops out of the fine scale, and standard m = 10 gets no entry at all [metrology#0]. Second, on a helical gear the normal of the involute helicoid lies in the base tangent plane at β_b to the transverse plane. Both measurements drop that projection: the ball contact is missing a factor of cos β_b, and the span contact is off by 1/cos² β_b. For a z60 β35 gear the tool picks k = 8 with its contact reported at 36.286 mm, but the anvils actually touch at 34.915 mm, below the form circle at 35.716 mm. It also refuses the correct span, k = 12. Over 720 helical gears, the chosen span touches off the usable flank in 214 of them [added#55]. The largest seating ball is overstated by 2–12 % [metrology#1]. In both cases the suite still passes 678 of 679 with the fix applied, so no test looks at a helical contact. Around these three faults sit the smaller ones: rules the user can neither see nor choose (the best-k rule, a fixed 1.75 mm pin), an eccentric space checked against only one of the two teeth that bound it, fields that nothing reads, and wording that says more than the standard does.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T09.1 JGMA bands stored as printed | metrology#0, added#41 | high | S | — |
| T09.2 Helical contact in the base tangent plane (balls and span) | added#55, metrology#1, lens-docs-accuracy-2#0 | high | M | — |
| T09.3 An eccentric space bounded by both its teeth; one all-positions reducer | metrology#7, added#50, added2#114, metrology#18(d) | medium | M | — |
| T09.4 Span k: published closed form, user override, admissible interval | metrology#2 | medium | M | T09.2 |
| T09.5 Transcription guard and scope wording | lens-tests-geometry#6, added2#85, metrology#3, metrology#5, metrology#18(b) | low | S | T09.1 |
| T09.6 Pin range: one monotone solve per condition, binding condition reported | metrology#11, lens-continuity#7 | low | M | T09.2 |
| T09.7 Suggested pin served from Rust | metrology#14 | low | S | T09.4, T09.6 |
| T09.8 Face width a helical span needs, reported | added2#115, metrology#15 | low | S | T09.2, T09.4 |
| T09.9 Cutter tip width from the settled tool; small API fixes | added#13, metrology#18(a,c) | low | S | — |
| T09.10 Delete the unread `limits` fields | metrology#13 | low | S | — |
| T09.11 Labels that say what is measured | metrology#16 | low | S | — |
| T09.12 ISO 1328-2 as a user-selected second standard | metrology#4 | low | M | T09.1 |

### T09.1 JGMA bands stored as printed

**Change.** In `crates/gear-core/data/jgma_116_02.csv`, store the edges exactly as printed: diameters 1.5, 3, 6, 12, 25, 50, 100, 200, 400; fine modules 0.2, 0.6, 1, 1.6; standard modules 1, 3.5, 6.3, 10; standard diameters 0, 125, 400, …. Remove the +0.01 offsets and the header comment that explains them. Give each band an explicit `lo_inclusive` column as printed. Only m0.2 and d1.5 in the m0.2–0.6 block are closed; the m0.6–1 and m1–1.6 blocks open with `をこえ`, so the flag cannot be derived from band order. In `jgma.rs`, `in_band` becomes `(v > lo || lo_inclusive && v == lo) && v <= hi`. In the CSV header and `jgma.rs:90`, name the source actually transcribed (edition and language), quote the Japanese terms (両歯面1ピッチかみあい誤差, 両歯面全かみあい誤差), record the band convention (以上/をこえ/以下), and remove the dangling `DESIGN.md 4.6.1` pointer [added#41]. Amend the corrections.md 4.6.1 row, which records the +0.01 patch as the resolution. For the standard scale, state that the (lo, hi] reading follows the ISO 1328:1975 / JIS convention and has not been checked against the main body.

**Proof.** Replace `module_bands_are_half_open` with a table-driven test over every printed edge, asserting which row each edge reads: Fine 0 at m = 1.0, d = 9 gives 6.0/18 (6.3/18 today); m = 0.6, d = 20 gives 6.3/18; m = 1.6 is `Some`; d = 1.5 is `Some`; d = 3.005 gives 5.3/16 (5.0/13 today); d = 400.005 is `None`; standard m = 10 is `Some`, and `default_class(10, 400)` is `Some`. Add a law: for every (scale, grade), a lookup is `Some` on the closed hull of the printed ranges and `None` just outside it. `tools/check_wasm.sh --write`: the default tab's `tooth_to_tooth` moves from 6.2999… to 6.0.

### T09.2 Helical contact in the base tangent plane (balls and span)

**Change.** Add one helper in `metrology.rs` that projects a length along the helicoid normal onto the transverse roll (a factor of cos β_b), and use it at both sites:
- In `pin_seat`, set `u_contact = tan φ − σ·D·cos β_b/(2 r_b)` (metrology.rs:504). The seat relation at :481 is already correct and stays.
- For the span, the contact rolls along a common normal sum to W cos β_b. In `span_over_teeth`, set `half = W·cos β_b/2` (:260); in `span_over_teeth_at`, set `total = sweep·cos² β_b` (:364). Then `d_M = √(d_b² + (W cos β_b)²)`.

Correct the doc comments at metrology.rs:257-259, :459 and :496-497, and reference.md:1413-1414. Write the span contact formula and its derivation into `reference.md#span-over-teeth`. Spur results are unchanged bit for bit, since cos 0 = 1. W itself was already right, and the independent anvil simulation reproduces it to 6 decimals.

**Proof (written first, fails today).** Add a test-only oracle in `tests/common`: the nearest point of the analytic involute helicoid to a ball centre, and the symmetric common-normal contact of two anvil planes. Restrict it to the u ≥ 0 sheet or seed it near the contact, because an unseeded search finds the other branch. Laws over the shared grid, with β ∈ {0, 15, 30, 40}°, external gears and rings:
- The ball satisfies |dist − D/2| < 1e-9, and its contact radius matches the oracle to 1e-9. Today z20 β30 D1.8 gives 11.593929 against 11.636143.
- A z20 β30 gear with D = 3.1487 is refused; it is accepted today.
- For z60 β35, the span contact is 34.915095 at k = 8 and 36.643345 at k = 12; today these read 36.286 and 39.527, and k = 12 is refused.

The pin-range hair law must keep holding. Add a helical gear to the metrology golden row (T20.11 owns the command [gear-cli#15]); helical pin ranges move by design. Also remove the `continue` that skips helical cases in the textbook span test (T16.24 [lens-tests-geometry#8]). Do that in the same change, because it is the other half of the missing helical cover.

### T09.3 An eccentric space bounded by both its teeth; one all-positions reducer

**Change.**
- `space_at(gear, i)` (metrology.rs:146) takes `tip = min(ra_i, ra_{i+1})` and `form = max(r_j,i, r_j,i+1)`. The root is not `max(rf)`, which is 0.152 mm too strict on z12 amp0.8. Take it from the drawn envelope at the middle of the space, through a new `Gear::space_root(i)` built on `root_at`.
- Add `metrology::over_pins_around(gear, d, count) -> Result<(OverPins, [f64; 2]), MeasurementError>`. It requires every start to measure and returns the error of the first start that fails, by the same rule as `best_span_around`. Delete the fold at gear-wasm lib.rs:357-374 that skips failures with `if let Ok`, and read this function instead. Publish the pin range as a `gear_core::auto::Bound` with its exclusive ends (`ranges.pin_diameter`), and delete the TypeScript-built bound at `GearPanel.svelte:209`. This is also T14.3 [wasm-boundary#3].
- In `pin_diameter_range_around` and `over_pins_around`, evaluate each distinct `Space` value once, deduplicated by value. `Gear::distinct` counts teeth, and a pin measures a space. A concentric gear then costs one space, with a bit-identical result. Leave `best_span_around`'s k scan as it is: about 5z calls, under 1 ms at z = 3000.

**Proof.**
- A law over eccentric gears: `over_pins_at(start)` is `Ok` if and only if d lies within `pin_diameter_range` of every space that start uses; and the summary's pins value is unavailable exactly when d lies outside `pin_diameter_range_around`. It fails today:
  - z20 amp1 λ1: the range runs from 1.94057 in the code, from 1.94528 with both teeth.
  - z17 amp0.4, d = 1.5236: refused at 2 of 17 starts, yet shown as [18.4361, 18.6438].
- `pin_diameter_range_around(Gear::new(p)) == pin_diameter_range(&Space::of(..))` bit for bit on concentric gears.
- Rotating `index_offset` by one tooth leaves the verdict and the range unchanged. Today at z = 23, x = 0.2, angular shift 0.5, d = 0.999·lo, `around` is [24.126, 24.420], below the published range, and `around[0]` moves between 24.091 and 24.126 over `index_offset` 0..3.
- Timing at z = 1000: 17.5 ms native today, about 0.04 ms deduplicated.

### T09.4 Span k: published closed form, user override, admissible interval

**Change.** Both `best_span` and `best_span_around` pick the admissible k whose contact lies nearest r, whatever x is. In the one Gear-based API that metrology#12 / ablate-features#7 keep, replace this with the published rule:

k = z/π·(tan α_Mt / cos² β_b − 2x tan α_n / z − inv α_t) + 0.5, with cos α_Mt = d_b / (d + 2x m_n),

rounded, then clamped to the admissible interval [k_min, k_max]. Contact radius is monotone in k, so that interval exists. Check the helical form above against ISO 21771 / DIN 3960 before coding it; it has not been checked against the standard. Add `teeth_spanned: Option<u32>` to `GearRequest`, honoured when it lies within the interval, and report [k_min, k_max] in `SpanOut`, as the pin box reports its range. Write the rule once in reference.md:1403. A max-margin (mid-flank) rule is a third convention and would not reproduce drawings; add it only as a named option if one is wanted.

**Proof.**
- z60 β35 x0 gives k = 12, since the continuous k is 11.96. This needs T09.2's contact, otherwise k = 12 is refused.
- z17 x0.8 gives k = 3 (k = 2 today, contacting 0.08 mm above the form circle).
- A supplied k in [k_min, k_max] is honoured. The a-mec drawing (m0.5, z18, x0), asked with k = 3, gives W = 3.8162; that case is a tie under the rule, so test it through the input.
- Every reported k lies in the reported interval.

### T09.5 Transcription guard and scope wording

**Change.**
- Add module-band and diameter-band monotonicity laws to `jgma.rs`. Fine g1, m0.6–1.0, d12–25, total 28 (against 26 at m1–1.6) is the one listed exception; it cites page 1 of the source and states that it matches the source.
- Add a double-entry check: a second, independently typed copy or a digest of the 618 values, compared cell by cell. Do not add a Renard-ratio law: real fine-scale steps span 1.18–1.67, so any band would have to be fitted to the data.
- Factor out `parse_row(&str) -> Option<Row>`, and add a test that it is `Some` for every non-comment line, naming the line. Do not panic in production (Cargo.toml:26).
- Wording:
  - rationale.md:1553 and the CSV header should list the five structural checks that exist, not "three".
  - In jgma.rs:20-26, rationale.md:1540 and reference.md:1493, write "fine g ≈ standard g+4 within about one grade in the overlap; still two tables" in place of "no rule … avoids" [metrology#3].
  - `ui.gear_tolerance_not_covered` (×5), the CSV header, jgma.rs:53 and reference.md:1497 should say "no transcribed entry", not "does not cover" [metrology#5].

**Proof.** Mutation M5 (line 95, 15 → 14) passes all 645 tests today; it must fail the double-entry check. The module law fires on the one cell until its exception is listed. A malformed row makes the parse test name its line.

### T09.6 Pin range: one monotone solve per condition, binding condition reported

**Change.** Replace the doubling walk and the 64 + 64 halvings in `pin_diameter_range` (metrology.rs:184-213) with one monotone relation in φ, used at every helix with no β_b = 0 branch. With c = cos β_b and T09.2 applied:

u = (1 − c²) tan φ + c² (φ − h),  D(u) = 2 r_b c (inv φ + h)

Solve it with `solve::newton_bracketed`; at c = 1 it is linear and exact in one step. Root bottoming is one more monotone root. Return `PinRange { lo, hi, lo_binds: Form|Root|Base, hi_binds: Tip }`. The thresholds must use exactly the relation `seat()` uses. Remove the "Sixty halvings" comment. T18.18 corrects the solver inventory that this changes [lens-docs-accuracy-2#1].

**Proof.** The existing hair law (tests/metrology.rs:258, 1e-9 each side of each end) still holds. The spur ends equal D = 2 r_b[tan(u + h) − u] to 1e-12; for z17, D_max = 3.7643497586696886. For each end, `lo_binds`/`hi_binds` names the check `seat()` fires just outside it. z20 x0.5 reports `Root`.

### T09.7 Suggested pin served from Rust

**Change.** Add `suggested_pin` to `GearSummary`/`RingSummary`: the diameter whose contact lands at the same target radius as T09.4, d + 2x m_n. It is closed form in the spur limit and one monotone root through T09.6's relation. Seed the tab's default from it in place of the literal `1.75` (gear-wasm lib.rs:1159), and show it in the hint beside the range as a one-click value. Rounding to a stock size, if offered, is an explicit option.

**Proof.** The suggested pin's contact radius equals the target to 1e-12 and lies inside `pin_diameter_range`. The default no longer fails: today m = 3, z = 9 and m = 2, z = 17 give `PinTooSmall`, and m = 0.3, z = 17 gives `PinTooLarge`.

### T09.8 Face width a helical span needs, reported

**Change.** Report `min_face_for_span = W_k·sin β_b` on `SpanOut`, taking the largest W around an eccentric gear. It is 0 on a spur gear by value, with no branch. State the condition in `reference.md#span-over-teeth`. No face width reaches the lone-gear summary and no train member reports a span, so there is nothing to refuse against. KHK's +3 mm is a rule of thumb, and belongs only as a named option.

**Proof.** 0 bit-exact on spur. For z60 β35: 12.7299 at k = 8 and 19.094 at k = 12, matching the axial offset from the T09.2 anvil oracle.

### T09.9 Cutter tip width from the settled tool; small API fixes

**Change.** Lands as T03.2, which carries all three bullets: `cutter_tip_width` from the settled tool through `Rack::settle`, `base_helix_angle` on `Tooth`, and `span_over_teeth(g, 0)`.

**Proof.** T03.2's.

### T09.10 Delete the unread `limits` fields

**Change.** Remove `limits` from `Span`, `OverPins` and `BetweenPins`, together with the five `limits: None` writes, the test at tests/metrology.rs:123 that pins the dead state, and the reservation at docs/state.md:630. If tolerance limits are wanted later, they come from a user-entered normal thickness allowance A_sn: shift ψ_b by A_sn/(m_n z) inside the existing closed forms, keep the nominal gear's validity verdict, and carry the result across `SpanOut`/`PinsOut`. Build that with lens-feature-gaps#3 (thickness tolerance in backlash).

**Proof.** `cargo clippy --all-targets -- --deny warnings`, and a grep finding no reader in `crates/` or `web/src`.

### T09.11 Labels that say what is measured

**Change.** Use one label for every gear, "Measurement over pins or balls", with a note that a helical gear is measured over balls. Choose no label by β in TypeScript. Change "Composite error, JGMA 116-02" to "Double-flank composite tolerance, JGMA 116-02" (×5; in German, Zweiflanken-Wälzabweichung and Kugelmaß). Rename `CompositeError` to `RadialCompositeTolerance`. Do this together with T19.10, which moves the scale names into the catalogue [metrology#6].

**Proof.** `tools/check_strings.py`, and `tools/check_bindings.sh --write` followed by `npm run check`.

### T09.12 ISO 1328-2 as a user-selected second standard

**Change.** Add `Standard::{Jgma116_02, Iso1328_2}`, chosen by the user, with JGMA staying the default. Implement ISO as `base(m_n, d) · 2^((Q−Q₀)/2)`, unrounded, with the standard's rounding as an explicit option. Target ISO 1328-2:2020, which is current and covers d ≤ 600 mm; if the 1997 edition is used instead, label it withdrawn. Its grade-5 forms are F_i'' = 3.2 m_n + 1.01 √d + 6.4 and f_i'' = 2.96 m_n + 0.01 √d + 0.8. Put the validity bounds in `Ranges`, and say when a gear is outside them. The summary names the standard and grade in force. Grades cannot be mapped between the two standards: a JGMA fine grade's total corresponds to about ISO g+5.6, but its tooth-to-tooth to about g+7.5.

**Proof.** A handful of printed Annex cells, recomputed at the geometric-mean m and d with the rounding, are equal. Continuity in m and d. Exactly √2 per grade before rounding, for ISO only and never applied to JGMA data. The 2020 formula must first be read from the standard itself.
