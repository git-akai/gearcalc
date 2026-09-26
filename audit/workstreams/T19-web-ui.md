## T19 — The web front end

**Why.** Rule 1 keeps engineering out of TypeScript by having defaults, bounds and strings cross from Rust. That stops duplication, but nothing stops a new literal, and the front end has no test runner (`npm run check` only type-checks) [web#20]. The panels parse numbers four different ways. A draft-plus-validate path sits in the gear tab; `bind`+`finite()`, raw `valueAsNumber` and `Number()` sit in the train tab. Three of the four let an empty or half-typed box write NaN, `null`, `0` or a fraction into the inputs. serde then refuses the whole train ("bad train request: data did not match any variant…"), and the lists, ports and names blank out [web#1, web#5]. Worst, the gear tab keeps a second copy of each box's text. After an external → internal → external round trip, the Dedendum box shows 1.4 while the drawing and the DXF use 1.25, which moves the root diameter by 0.3 mm at m = 1 and by 1.5 mm at m = 5 [web#0, reproduced]. Around this core sit TS copies of core verdicts, fixed-decimal formatting (16 `toFixed` calls) in mixed locales, English outside the catalogue, a zh-Hant catalogue with five Simplified characters, and no persistence or undo. `TrainPanel.svelte` is 3,079 lines.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T19.1 A front-end test runner | web#20 (part 1) | medium | M | — |
| T19.2 One `NumberBox` for every number input | web#0, web#1, web#2, web#4, web#5, lens-errors-policy#16, web#9 (seeding), web#17 (step 1), web#10 (badInput) | high | L | T19.1 |
| T19.3 The ring tab shows no external gear's bounds | web#3, added3#4 (unverified) | medium | S | — |
| T19.4 Case edits go through the core | web#15 | low | M | — |
| T19.5 Undo, and a destructive edit that can be seen first | lens-feature-gaps#15 | medium | M | T19.2, T19.4 |
| T19.6 Tabs survive a reload | lens-feature-gaps#14 | medium | M | T19.5 |
| T19.7 One formatter, in the app's language | web#9 (display), web#10, primitives#14, ablate-features#14 | medium | M | T19.2 |
| T19.8 No boundary failure is silent | web#13, added3#12 (unverified), added3#17 (unverified), added2#21, web#14 | low | S | — |
| T19.9 Rule 1 enforced in TS; the core sends what the panel computes | web#20 (part 2), web#6, lens-magic-numbers#2, web#7, web#8, added2#25 | medium | M | T19.7 |
| T19.10 Every word through the catalogue | web#11, metrology#6, web#12, ablate-features#5, added2#77 | low | S | — |
| T19.11 zh-Hant: Traditional script, one term per concept | gear-io#9, added2#28, added2#27, added3#6 (unverified) | low | S | — |
| T19.12 The script subtag decides the Chinese catalogue | gear-io#13 | low | S | — |
| T19.13 Viewport: multi-pointer and accessibility | added2#22, web#19 | low | S | — |
| T19.14 Small state fixes | web#22 | info | S | — |
| T19.15 Split `TrainPanel.svelte` | web#17 (steps 2–3) | medium | M | T19.2 |
| T19.16 Show what one more tooth does | lens-feature-gaps#10 | info | S | T19.15 |
| T19.17 A mesh drawing in the train tab | lens-feature-gaps#13 | low | L | T19.15 |

Other tasks that bear on the panel: the refusal protocol, including `edit_train` returning `{train, refused}` and a note for every refusal a designer can reach, belongs to T02.5 [added2#80, wasm-boundary#5, lens-architecture#3, lens-errors-policy#10]. Generated request types belong to T14.1 [added2#42, wasm-boundary#6, tools-ci#10]. Fresh-case figures written once belong to T13.6 [wasm-boundary#7, graph-ops#14]. The ring's own shift range belongs to T05.13. The chord-tolerance floor sent to the panel belongs to T04.7 [wasm-boundary#12]. Solving off the main thread belongs to T11.13 [lens-performance#4]. Terminology across all five catalogues is T18.24 [gear-io#10], and a check of `ui.*` placeholders against their call sites is T16.20 [gear-io#15].

### T19.1 A front-end test runner
**Change.** Add `vitest` and `jsdom` as devDependencies and a `test` script. Components mount through Svelte's own `mount()`: `@testing-library/svelte`'s `.svelte.js` is not compiled under `node_modules`. Stub `ResizeObserver` and `getContext`, load the wasm with `initSync`, then run `loadCore()` and `workspace.initialise()`. The lock file changes, so update the fixed-output hash in `nix build .#web`, add `npm test` to CI, and add the check to CLAUDE.md's table. Seed the suite with the four-case repro of web#0 (`test/stale.test.ts`, already written against the real panel).
**Proof.** The seeded suite fails 4 of 4 on the current code. `nix build .#web` is green with the new hash.

### T19.2 One `NumberBox` for every number input
**Change.** Add `NumberBox.svelte`. It is the only place in `web/src` with `type="number"`.
- Its props are `value`, `set`, `integer?`, `bound?` (a core `Bound`) and `auto?`.
- It holds draft text only while the text does not parse, or differs from `value` while focused. Otherwise it shows `value`. A change from outside, such as `setKind` or adopt, clears the draft.
- Its error is `$derived` from (draft or value) against the *current* bound, so an error follows a bound that moves in either direction.
- It commits only a finite value that passes the integer check and the bound. An empty box, a lone `-`, a non-integer or an out-of-bound value keeps the last committed number and shows why: `ui.validation_required`; `ui.validation_not_a_number` (new, ×5) when `validity.badInput` is set; `ui.validation_not_a_whole_number`; or the bound's text.
- Turning auto off seeds `manual` with the full solved value, not the 4-decimal display. Fix the comment at TrainPanel.svelte:1847-1848, which says the gear tab does the same.

Migrate in three steps, each green on `npm run check` and T19.1:
1. Port GearPanel's `FIELDS` loop and its plain `bind:value` boxes (mate, cutter, tip round, pin, chord, throw). The eccentric sizing mode moves into its own boolean, `throwIsInput`, so clearing the throw box no longer flips the mode [web#4].
2. Port `property()`. An empty box keeps the override, × restores the library value, and the box shows `BLANK` rather than a made-up greyed `0` when nothing is known [web#2]. Then port `boundedNumber` [web#1] and `autoNumber` [web#9].
3. Port `numberField` and the integer boxes (teeth, cutter teeth, axis count, actuations) [web#5, lens-errors-policy#16]. Delete every `min=` attribute: bounds come from the core. `GearResult.ranges.teeth` exists; add a range for `axis.count` beside it. Until T02.5 makes a malformed request a note, `core.ts` serialises with a replacer that throws `ui.train_boundary_failed` on a non-finite number, so nothing sends one.

**Proof** (vitest, written first):
- The web#0 repro cases pass: each box's text equals `String(params[key])` unless a draft exists; clearing a draft on a kind change; an error that clears when the bound widens; an error that appears when the bound tightens past an accepted value (dedendum 1 → 1.25 with root radius 0.5, max 0.572 → 0.448).
- For every number box, typing `""`, `-`, `1,5` and `17.5` (in an integer box) never changes the train. A solve after each returns ports and names, where today it gives the serde failure.
- A grep gate in the test: `type="number"` appears only in `NumberBox.svelte`.

### T19.3 The ring tab shows no external gear's bounds
**Change.** In `GearPanel.svelte`, compute the external `result` only when the tab is not internal. An internal tab validates against no shift bound and shows the sentence "the ring's limits are reported as clamps" (new key ×5), in place of undercut, sharp-rack undercut and pointed figures that describe no ring. At z = 30 those read −0.755 and 1.622, while the ring is clamp-free only on [0.10, 0.68]. Once `RingSummary.ranges` exists (T05.13), `bounds` becomes `$derived` from the ring's ranges.
**Proof.** A vitest test: on an internal z = 30 tab, no external-range text renders, and `solve` (external) is not called. At z = 60, x = −1.5, which the external bound admits and the ring clamps, is no longer shown as within bounds.

### T19.4 Case edits go through the core
**Change.** Add `TrainEdit::RemoveCase(i)`, which calls `drop_bare` afterwards, so that a body only the removed case named leaves as the core's rule says. Add `TrainEdit::SetRole {case, body, role}`, where Rust shapes the fresh `Load` rather than TS pushing `{auto: true, manual: 0}`. Add both to `train/conditions.rs`, with a `wasm_probe.mjs` step each. `removeCase` and `setRole` in TrainPanel become `editTrain` calls. `enabled` stays a plain field.
**Proof.** A core law: removing the only case that names a parked body leaves no bare body. Today the TS splice leaves one. `check_wasm.sh --write` records both steps.

### T19.5 Undo, and a destructive edit that can be seen first
**Change.** In `state.svelte.ts`, add one `write(tab, f)` through which every mutation of `tab.train` passes: `editTrain`, relief, and case and duty edits. After T19.2 and T19.4 that is every writer. It pushes a `structuredClone` of the whole `TrainTab.train` onto a bounded per-tab stack. Snapshot the train, not the shape, because a removal also prunes holds and cases. Bind Ctrl/Cmd-Z and Ctrl-Shift-Z, and put an Undo button beside the offers strip. On `(pointer: coarse)`, the first tap on an offer shows its dry run and the second applies it.
**Proof.** vitest: for each preset, apply every offered edit and undo, and the train is deep-equal to before, holds and cases included. A synthetic coarse-pointer tap shows the preview and leaves the train unchanged.

### T19.6 Tabs survive a reload
**Change.** Write the tabs' inputs to `sessionStorage` on change, debounced and wrapped in try/catch, and restore them on load. sessionStorage is per tab, so two copies still never cross. Gear tabs have no other save route: DXF cannot be read back. The developer flag stays unstored, so a restored developer-only kind (eccentric) must re-enable developer mode or be converted; say which in the code. Add a `beforeunload` prompt while any tab differs from its default. Rewrite `rationale.md` 2150-2160 and the comments at `state.svelte.ts:36` and 439-449, which argue that a reload returns everything to its default.
**Proof.** vitest: serialise, reload the workspace, and restore gives deep-equal tabs for every preset and for a gear tab of each kind. A throwing storage accessor still renders defaults.

### T19.7 One formatter, in the app's language
**Change.** Add `fmt.ts`: `fmt(v, kind)` = `Intl.NumberFormat(language(), …)`, with significant digits for readouts so small magnitudes keep their digits (a 3e-5 N·m torque today prints 0.0000). `kind` also carries the display unit change (mm → µm, fraction → %), stated once there. Route `num`, `n`, `pct`, `count` (today `toLocaleString()` in the *browser's* locale) and GearPanel's formatters through it. Set `document.documentElement.lang` in `setLanguage`.
Notes: `Note::number` crosses `{value, decimals}` instead of a finished `'.'` string, and both the TS renderer and gear-io's `fill` format it with the language's separator. A value that rounds to zero is written as +0; a signed gap at `shape.rs:3670` is the reachable `-0.000`. This changes the Note wire type, the ts-rs binding and the golden outputs. Reword `note.rs:19-21`, `rationale.md:46-50` and `strings.svelte.ts:80` to say it once: notes carry the core's decimals, and readouts carry significant digits. Pin the shared placeholder grammar with one Rust test in `gear-io/src/strings.rs`: every `{…}` matches `^\{[A-Za-z0-9_]+\}$`, and `{{` never occurs [ablate-features#14].
**Proof.** vitest: in `de`, one table prints the same decimal separator in the cycles column, the torques and a note value. Today it prints `1.000.000` beside `0.1234`. `fmt(3e-5, torque)` is non-zero, and a note gap of −0.0004 prints `0.000`. `check_golden.sh --write` is reviewed as a format-only diff.

### T19.8 No boundary failure is silent
**Change.** Apply the interim fix in `core.ts` until T02.5 lands. `editTrain` returns `ui.train_boundary_failed` with the detail in place of `null`, so `Offers.make` shows it and does not call `made()`. `relieveTrain`, `relieveCase`, `previewEdit` and `offersAt` log `console.error` with the entry point's name before their fallback. `addCaseOfKind` checks `editTrain`'s return before moving the selection [added3#17 (unverified)].
Member names: the workspace keeps one `$derived` train outcome per train tab, which TrainPanel and `memberRefs` both read, so opening a gear tab no longer runs a full `solveTrain` per train tab (2.7–7.4 ms each). No new entry point is needed. A missing name is not adoptable: `names[i] !== undefined && names[i].role !== "worm"`.
**Proof.** vitest with `solve_train` and `edit_train` stubbed to throw a non-`ui.` error: the offers menu stays open and shows the key, `addCaseOfKind` leaves the selection alone, and a worm is not adoptable. With N train tabs open, mounting a gear tab adds no `solve_train` calls.

### T19.9 Rule 1 enforced in TS; the core sends what the panel computes
**Change.** Add `tools/check_ts_numbers.py` to CI. It flags numeric literals other than 0, 1 and −1 in `web/src` script and markup, skipping comments, CSS, `step=` and `fmt.ts`, with a short commented allowlist. Fix, rather than allowlist, what it finds:
- `GearTabDefaults` gains `mate: MateRef`, pinned in the boundary-defaults canary, and `freshTab` reads `d.mate` [web#6, lens-magic-numbers#2]. The 600 profile points may stay a display constant, or come from the chord tolerance (T11 owns that argument).
- `Variation` gains `tip_diameter` and `root_diameter` ranges, as `GearSummary` has, and the `2 *` at GearPanel.svelte:723/728 go [web#7].
- `MeshCase` gains `idle: bool` from `groupings.rs`'s one rule (`< 1e-9`), in place of TS `=== 0`. `internalOn` is replaced by `tip_room !== null`. `Directional` crosses `locked: Directional<bool>` [web#8]. Basis weak/strong, `angle !== 0` and the contact-ratio colour are reads of documented encodings and stay.
- Crank backlash [added2#25]: `gear.rs:855` computes j from (ideal − fit) without cancellation. At angular shift 1e-12 the magnitude reads −5.77e-15 against a true ≈ 1e-24. The panel then warns only on a displayed non-zero value.

**Proof.** The scan fails on the current tree (43, `2 *`) and passes after. The canary pins `mate`. A core test: `idle` matches the flow's idle set on every preset. The crank-backlash test at `gear.rs:2022` gains |lower| ≤ 10·amplitude² at shift 1e-12.

### T19.10 Every word through the catalogue
**Change.**
- Split `ui.gear_tolerance_no_entry` into `_fine` and `_standard`, each with `{grade}`, and delete `Display for Class` [metrology#6]. Add `ui.tolerance_scale_fine` and `ui.tolerance_scale_standard`, written as literal keys in both branches (not concatenated, so check_strings sees them).
- The Sidebar uses `ui.gear_unnamed` and `ui.train_unnamed`. The export file names come from keys.
- Units follow one convention: literal SI symbols everywhere. Delete `ui.gear_mm`, `ui.train_mm` and `numberField`'s `'°'` special case.
- The basis badge: `ui.basis_mark_<basis>` and `ui.basis_<basis>` for the four that render (overridden never does), with `used.note` in the title. The weak class reads a `measured` flag Rust sends from `Basis::is_measured` [web#12, ablate-features#5].
- Show the selected material's `condition` under the picker, as data like the names. Say in `materials_default.toml` that it is untranslated [added2#77].
- Extend `check_strings.py` to flag capitalised string literals in markup. Do not flag unit symbols.

**Proof.** The extended check fails today on `"Fine"`, `"Standard"` and `"Unnamed"`. A vitest render shows four distinct badge marks. The fire-every-note law covers the split keys.

### T19.11 zh-Hant: Traditional script, one term per concept
**Change.** Fix 际→際 (lines 43, 252), 属→屬 (105), 点→點 (186) and 机→機 (253). Fix 加載→載入 and 內核→核心 in `app_core_failed`, `app_loading_core` and line 255. In `train_load_port`, 端口→軸口 and 地→機架. Use one word for load (負荷, 載荷 or 負載 today), per the CNS vocabulary the header cites. Add a Rust test in `gear-io/src/strings.rs`:
- no Simplified-only character in zh-Hant, with the set generated once from Unihan `kTraditionalVariant` into a checked-in data file (a hand list would have missed three of the four);
- a per-concept glossary for zh-Hant (port→軸口, ground→機架, core→核心, load→the chosen term). gear-io#10 extends it to the other languages.

**Proof.** Both tests fail on the current catalogue: five Simplified hits, plus 端口 and 內核.

### T19.12 The script subtag decides the Chinese catalogue
**Change.** In `Language::resolve` (`strings.rs:69-88`), find a `hans` or `hant` subtag first. Use the region (tw, hk, mo) only when there is none, as the doc at :58 says.
**Proof.** Add `zh-Hans-HK` → Hans, `zh-Hans-TW` → Hans and `zh-Hant-CN` → Hant to the table. The first two fail today.

### T19.13 Viewport: multi-pointer and accessibility
**Change.** In `Viewport.svelte`, keep a `Map<pointerId, {x, y}>`. One pointer pans. Two pointers pan by the midpoint and zoom through `zoomAbout(midpoint, span ratio)`. Delete the id on up, cancel and `lostpointercapture`, and re-seed the remaining pointer when the count drops to one. Give the canvas `role="img"`, an `aria-label` and +/−/arrow keys. Add `aria-pressed` to the segmented buttons (duty, case role, grouping), matching `Switch`, and `aria-current` to selected rows.
**Proof.** vitest with synthetic pointer events: two fingers moving together pan by their shared delta, with no jump when one lifts; spreading them zooms.

### T19.14 Small state fixes
**Change.** The popup width lives in one CSS variable (`--pop-width`), which `Offers` reads with `getComputedStyle` in place of `POP = 320`. `importError` and `adoptError` clear on tab switch, and each gets a dismiss control. Add a non-cloning `presetInfo()` in `core.ts` for preset and family labels; `defaults()` costs 0.27–0.55 ms per clone, three times per offer.
**Proof.** vitest: rendering the add menu calls `structuredClone` of the defaults zero times, and switching tabs clears a failed import's message.

### T19.15 Split `TrainPanel.svelte`
**Change.** After T19.2, add `FieldRow.svelte` (label, `NumberBox`, switches, unit, `FieldNote`) with one grid template. Today `1fr auto var(--field-box) var(--unit-cell)` is repeated at TrainPanel 2242/2245/2248/2615 and GearPanel 947 under rising specificity. Then move the snippets out one component at a time: `GearCard`, `MeshReadout`, `CaseEditor`, `TrainLists`, `PathBox`.
**Proof.** Each step passes `npm run check`, T19.1 and a visual check. There is one `grid-template-columns` for field rows.

### T19.16 Show what one more tooth does
**Change.** Send `PathReport.per_tooth` (drop its `serde(skip)`) and show "ratio at z+1" on each gear card of the headline path. Reword its doc at `train/mod.rs:3914-3920` and `corrections.md:681`. Do not make it lazy: it costs tens of µs.
**Proof.** `check_bindings.sh --write`. A vitest render: the card's figure equals the harness's golden "one more tooth" line for the same preset.

### T19.17 A mesh drawing in the train tab
**Change.** Add a `mesh_profile(train, mesh)` entry point. It returns both outlines placed: b at the running distance, rotated by a closed-form phase that centres a tooth of a in a space of b, and planets at 2πk/N with phase φ_k = 2πk·z_c/(N·z_p) (the textbook simple set). An eccentric set also needs the crank angle. `Shape::assembly` computes no angle, so derive the phase fresh and write it into `reference.md`. The existing `Viewport` draws it. Add a probe step. Rewrite `state.md:628` ("Not planned").
**Proof.** An independent check that shares no code: sampled outlines of a and b at the placed phase do not intersect (clear by the reported backlash to 1e-6), and do intersect at the phase shifted by half a pitch.

### Declined
None. No finding here was refuted. Where a claim was narrowed, the tasks implement the narrower one.
