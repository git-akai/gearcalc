## T13 — Graph edits, offers and load cases

**Why.** The train's edits keep the graph whole but not what hangs off it. Case entries, a fatigue duty's sweep body and holds all name bodies by number, and each edit fixes them up with its own local rule: `hold` drops entries but leaves the duty, `try_join` keeps entries on a body it makes held, `drop_orphans` drops a reaction and grounds the duty, and an emptied graph keeps listing a body that `chain_on` then indexes by a list position the first merge has already moved. No law checks the train as a whole. The offer and edit laws build every train with **no load case**, go one edit deep, and compare `Shape`s. So one offered edit (Wolfrom, Remove(Axis(1))) followed by the panel's `offers(Train)` panics with "index out of bounds: the len is 2 but the index is 2" and traps the wasm instance [edit-ops#0]. A random walk hit "case entry at held body" 237 times [graph-ops#6]. A hold that leaves the duty behind turns 1250/167/209 counted cycles into 0/0/0 with no note [train-mod-b#3]. And not one of the 94 add-a-gear-onto-an-existing-body offers on the ten presets gives a train that solves [edit-ops#7]. The fix is one train-level invariant, one rule per kind of rewrite, and laws that walk cased trains through sequences of offered edits.

| Task | Findings | Sev | Effort | Needs |
|---|---|---|---|---|
| T13.1 Train-level invariant and a seeded walk over cased trains | graph-ops#8, edit-ops#6 | medium | M | — |
| T13.2 An empty graph lists nothing; `chain_on` reads body numbers | edit-ops#0, graph-ops#2 | high | S | T13.1 |
| T13.3 One rule for a held body: entries and the duty sweep | edit-ops#5, graph-ops#6, graph-ops#5, train-mod-b#3 (hold part) | medium | S | T13.1 |
| T13.4 A removal re-seats case entries rather than dropping them | graph-ops#4, graph-ops#5 (drop_orphans part) | medium | M | T13.3 |
| T13.5 The solve validates a case: duty port, still sweep, duplicate entries, switched-off cases | train-mod-b#3, graph-ops#5, train-mod-b#6, graph-ops#3 (b) | medium | S | — |
| T13.6 `fresh_case` lands on open ports; fresh figures stated once in the core | graph-ops#3 (a), lens-errors-policy#15 | medium | S | T13.5 |
| T13.7 An added gear does not copy its mate's given helix | edit-ops#1 | medium | S | T13.1 |
| T13.8 One validated way to hold, release, join and set a duty; refusal keys that say why | graph-ops#7, added2#82, edit-ops#12, lens-errors-policy#15 | low | S | T13.3 |
| T13.9 Refuse the adds that are certain dead ends | edit-ops#7, added2#38 | low | S | T13.1 |
| T13.10 Pin the preview's comparison | edit-ops#4 | low | S | — |
| T13.11 `Train::edit` reports what it made and how it renumbered | lens-architecture#4, edit-ops#4 (path identity) | low | M | T13.2, T13.10 |
| T13.12 One rule for a bare body | edit-ops#10 | low | S | — |
| T13.13 An Insert names its preset | edit-ops#13 | low | S | — |
| T13.14 `LoadCase` API: `arranged` keeps its promise, first-entry accessors go | train-mod-b#12, train-mod-b#13 | low | S | — |

### T13.1 Train-level invariant and a seeded walk over cased trains
**Change.** Promote `well_formed` (`train/edits.rs:834`, test-only today) to a `Train::check() -> Result<(), Invariant>`, called in tests and under `debug_assert!` at the end of `Train::edit`. It checks the graph as it does today, and also:
- every case entry and every `Duty::Intermittent::at` names a listed, unheld, open port. On a train with no members nothing is listed, and the entries are parked numbers;
- no two entries of one case at the same body;
- `held` is a subset of the listed bodies;
- body numbers are dense (`1..=max`);
- `carried_by` is acyclic.

Give `offers.rs`'s `trains()` (`:247-262`) and the sweeps at `edits.rs:1566/1570` each preset's conventional cases, an ultimate case and a fatigue case at `chain_ends()`, plus `with_first_helix(20.0)` variants. This one-line change has the highest yield. Then add a seeded walk law with a fixed in-test LCG (gear-core has no `rand`), about 200 walks of depth 4 over unrefused offers at random targets. It interleaves `fresh_case`, `set_duty`, Hold/Release and Join. After each step it asserts `check()`, that `offers(Target::*)` and `solve_train` do not panic, and a JSON round trip. On failure it prints the edit sequence. The probe ran 200×8 in about 1 s.
**Proof.** Run the walk in a worktree at HEAD first. It must fail on the `chain_on` panic (reached within 2 edits: Wolfrom, AddGear, AddGear ring, Remove(Axis(1))) and on "entry at held body". The walk goes red on HEAD, so land it in the same change as T13.2 and T13.3. The cased `trains()` alone already catches [edit-ops#5] over every preset. Mutation M13 (hold keeps its entries), which survives 678/678 today, must now fail. `set_duty` (`conditions.rs:1148-1165`) and `chain_on`'s duty move (`1222-1223`) are executed for the first time.
**Notes.** The production import check that shares `check()` belongs to [wasm-boundary#0] and [lens-tests-train#5]. CLAUDE.md's "sweep the presets" wording is [lens-tests-train#10].

### T13.2 An empty graph lists nothing; `chain_on` reads body numbers
**Change.** In `Train::edit`, once a removal leaves `shape.members` empty, set `self.shape = Shape::default()` (no bodies, axes, distances or couplings). The cases stay parked by number, which is what `drop_orphans` (returns early at `conditions.rs:341`) and `parked()` already assume. This restores `lay`'s stated precondition. Then remove the fragility itself: `lay` returns the laid bodies' numbers instead of list positions (`conditions.rs:1236`), `merge` returns the renumbering it made, and `chain_on`'s k==0 branch (`1197-1210`) looks up the output by its renumbered body number. Correct the comment "so the second is read afresh".
**Proof.** Law: for every preset P and every order in which offered removals empty it, `offers(Target::Train)` does not panic, and `Insert{Q, at: None}` for every preset Q gives the same cases and holds as `chain(&[Q])`, with a headline and every case solved. Extend `the_cases_survive_an_emptied_train_and_take_the_next_preset_conventionally` (`mod.rs:7325`) with the `chain_on` of a Spur. On HEAD this fails in three ways:
- Wolfrom panics.
- Idler inserted into an emptied Wolfrom reads 1→3 at −1.4706 instead of 1→2 at +2.5294 (= 43/17). Layshaft reads 1→3 at −2.5294 instead of 2.3662.
- Planocentric emptied then given a Spur puts both entries on one shaft, and the paths are None.

### T13.3 One rule for a held body: entries and the duty sweep
**Change.** Write one private `Train::settle_held(body)` and make it the only thing that applies a hold. It drops every case entry at the body and re-seats a duty sweep measured there with the rule `set_duty` uses: the first Reacted entry, else the first entry. With neither, the sweep is left unset for the solve to report (T13.5), never GROUND. `hold`, the end of `merge`/`try_join` (`conditions.rs:870-918`) and `lay` (`1236-1253`) call it for every body they leave held, instead of pushing to `held` directly. A Join that makes a loaded or reacted body held should be refused: it turns a reacted shaft into ground, and today it only yields `Overdetermined` later. The preview then lists "case entries" as changed, as it does for a plain Hold.
**Proof.** The `check()` clause "no entry and no sweep at a held body", run by T13.1's walk. Cases that fail on HEAD:
- Spur + Spur at 1, Hold(3), Join(2,3) gives a reaction at held 2 and `Overdetermined{at:2}`.
- Two Planetaries with held [3,6] and Join{3,5} give a reaction at held 3.
- A Planetary with the duty at the ring, then Hold(ring), reports cycles 0/0 on every member with no note.
- A 12/30/72 planetary after Hold(2) and Release(3) reports 0/0 where it gave 1250/167/209.

### T13.4 A removal re-seats case entries rather than dropping them
**Change.** In `drop_orphans` (`conditions.rs:341-355`), when a removal takes the body an entry names and members remain, move the entry and not drop it. It goes to the removed part's body that exactly one remaining part still lists (the join body). If that is ambiguous, it goes to `chain_ends().1`. If there is still nowhere to put it, it stays parked with a note. Role and figures travel with it, and a duty sweep at that body follows it by T13.3's rule. Silent dropping, and silently grounding a duty, both go.
**Proof.** A train-level undo law beside `every_add_undoes`: for every preset p, take a cased Spur, Insert p at None, then remove p's pieces through offers. `load_cases` must equal the original field for field, and every case must solve. On HEAD the shape comes back equal but the cases become [(1,Load)] with the duty at 0, and every case reports `train.load_not_reacted` (Spur, Idler, Layshaft, Worm, Crossed, Wolfrom, Compound).

### T13.5 The solve validates a case
**Change.** In `solve_parts`, next to the `LoadPort` check (`mod.rs:4346`):
- Refuse a duty port that is not an open port, with a new `TrainError::DutyPort{case, body}`. Today `speeds[port]` (`:4527`) panics on duty at 3 or 99, reached through `from_toml` too.
- Refuse two entries of one case at one body, with a new `TrainError::DuplicateEntry{case, body}`. Today two given 3000 rpm speeds sum to 6000 (body 2 at −2372.09), and for torques the last entry silently wins.
- When the sweep body turns in the case motion at zero speed, or the sweep is unset, report cycles `None` with a note `train.duty_at_still` naming the body. Delete the `unit` fallback (`:4413-4438`) for cycle counting. It mixes rpm with a ±1 weight: the sun reads 77,118 cycles at 3000 rpm and 7,689 at 300 for one geometric sweep, which is exactly 30·(6/7)(n−1).
- A switched-off case with a bad port becomes `solved=false` plus a note in its own `TrainCase`, and no longer fails the whole train. An enabled case at a non-port keeps its refusal.

Add the new keys to all five catalogues.
**Proof.** Law over every preset and every body b, including b = 0 and b = max+1: a fatigue case with its duty at b either refuses, or carries the note, or counts non-zero cycles on every loaded member, and it never panics. Unit test: `[given(1,3000), given(1,3000), Reacted(2)]` is refused. A negative fixture in gear-io's reader for a duty at 99. Run `check_strings.py`.
**Notes.** The cycle count feeds no rating today, since identical min face widths come out at 2.5e8 and 0 cycles. So the harm is to a reported figure, which is why the severity is medium.

### T13.6 `fresh_case` lands on open ports; fresh figures stated once
**Change.** In `fresh_case` (`conditions.rs:1094-1101`), on a non-empty train with no headline and no chain ends, fall back to the first and last open ports in body order. `(max_body+1, max_body+2)` is kept for an empty train only. Move the fresh-case figures, (0.1, 30 000) ultimate and (0.02, 30 000) fatigue, into gear-core as `LoadCase::fresh(kind)`. `defaults_impl` and `apply_edit`'s AddCase (`gear-wasm lib.rs:1164-1168`, `1580-1583`) both read it.
**Proof.** The walk law gains a clause: the case `fresh_case` returns names only open ports, or parked numbers on an empty train, and pushing it switched off leaves `solve_train`'s Ok/Err unchanged. On HEAD, cased [Worm, Planetary, Compound], Remove(Mesh(0)) twice, then Insert Layshaft at 1 gives a case at [11,12] and `Err(LoadPort{case:2})`. A gear-wasm test: `AddCase` of each kind equals the shipped train's case of that kind, torque and speed included; today nothing reaches `AddCase` (the only test, at lib.rs:2853, checks the shipped train).
**Notes.** The figures half is also T14.13 and T15.1's first bullet [lens-architecture#1, wasm-boundary#7, graph-ops#14, lens-magic-numbers#3, added2#37]. The continuous duty's 1000 h seed [added2#24, added2#36] is T15.1's `Duty::continuous()`.

### T13.7 An added gear does not copy its mate's given helix
**Change.** In `push_follower` (`edits.rs:503-512`) and `add_on_new_axis` (`388-393`), build the new gear with `helix_angle` and `pitch_diameter` at `Auto::automatic`, so the per-part helix propagation gives it the hand its mesh needs. At a mate whose own mesh is crossed (a worm or a crossed-pair gear), refuse the add on a new parallel axis, or build a spur/helical gear at the mate's normal module, not a copy of the thread. `AddRatio` keeps its whole-pair copy.
**Proof.** Replace the hand list `applicable()` (`edits.rs:941-985`) in `every_add_on_every_preset_solves` with every unrefused AddGear the offers list, on every preset and on its `with_first_helix(20.0)` variant. Each must solve, or fail only with an error from an explicit allowed set (see T13.9), never `Mesh(Incompatible)`. On HEAD, Spur with a 20° given helix plus AddGear{mate 0, NewAxis} copies the 20° as given and fails `Mesh(Incompatible)`. The same add at the Worm's mate 0 copies d = 7 mm and 81.79° and fails the same way.

### T13.8 One validated way to hold, release, join and set a duty
**Change.**
- Make `Train::hold`, `join` and `split` `pub(crate)`, and route `gear-cli/src/kinematics.rs:289-328` and the test fixtures through `Train::edit`. Today `join` discards `try_join`'s refusal (`let _ =`, `conditions.rs:865`), and `hold(99)` records a hold that fails only later as `NoSuchBody`.
- Make Hold and Release symmetric. Both refuse `NoSuchIndex` for ground or an unlisted body, and both treat the no-op (Hold of a held body, Release of a free one) the same way: both Ok, or one shared "already so" key.
- `set_duty` returns `Result<(), EditRefused>` with `NoSuchIndex` for an out-of-range case.
- Split `WrongFamily`'s five causes into keys that each say why: `CentralAxis` ("a carrier turns about this axis"), `NotCarried` (a step or a coupling off a carried axis), `RingToRing`, and `OrbitingMate` (a fixed new axis cannot reach a gear whose mates orbit). The last is the refusal an epicyclic train shows most, on every member's "on a new axis" offer. Each key needs five catalogue lines. Correct the enum's doc.
**Proof.** Extend `a_refused_edit_changes_nothing` to Release(99), Release of an unheld body, and `set_duty` at a bad index. The test at `edits.rs:1553`, which relies on the silent join refusal, asserts `Err(Apart)` instead. Also `an_offer_is_its_edit`, `check_strings.py` and `check_wasm.sh --write`.

### T13.9 Refuse the adds that are certain dead ends
**Change.**
- `AddGear{on: Body(b)}`: refuse with the existing `EditRefused::Geared` when b and the mate's body are already joined by a mesh of a *different* ratio. That is a lock, decidable without a solve. An equal-ratio twin is a real split-path mechanism the model cannot load-share, so it stays offered and the dry run says `load_shared`.
- In `central_teeth` (`edits.rs:414, 429`), lower the ring floor from zp+2 to zp+1. When the fitted count is taken, refuse `NoRoom` unless some other count ≥ zp+1 can close at the carrier radius, checked with the solve's own test cos α_w = a₀ cos α / a ∈ (0,1). Today planocentrics 18/19, 20/21, 30/31, 30/32 and 18/20 all offer the ring add unrefused, and each fails `NoCommonDistance`.
- Look into the 8 `Overdetermined` offers on MeshedPlanets and 2 on Planocentric at bodies not yet geared, and give them a rule of their own. The check above does not reach them.
**Proof.** Replay the 94 AddGear-at-a-body offers on the ten presets. Today 0 solve: 35 end in `load_shared`, 46 in `Overdetermined`, 5 in `NoCommonDistance`, and 8 in `Overdetermined` at ungeared bodies. Afterwards every unrefused one solves or ends in `load_shared` only. Add planocentric(18,19) and (30,32) to the add sweep.
**Notes.** The zp+2 and "fewer than 4 teeth" constants are named and pinned under [lens-magic-numbers#13] and [edit-ops#11].

### T13.10 Pin the preview's comparison
**Change.** Tests only, in `preview.rs`:
- An insert at the output pins ratio_before 2.5294 and ratio_after 6.3979 over 1→3 (Spur + Spur).
- A Move that changes no count does not yield `PREVIEW_NOTHING`.
- A (Some, Some) pair with a different headline yields gone plus new.
**Proof.** Mutations M4 (before and after swapped), M5 (identity check ignored) and M12 (NOTHING whenever the counts are equal) survive the whole workspace today, and each must fail.

### T13.11 `Train::edit` reports what it made and how it renumbered
**Change.** `Train::edit` returns `EditOutcome { made: Vec<PieceRef>, renumbered: Renumbering }`, built from the maps that `prune`/`merge` already compute (T13.2 exposes `merge`'s). `edit_train` passes it through. `TrainPanel.svelte:249-254`'s `made()` becomes a lookup instead of restating "the lower body survives a join" and "the first new mesh is at the old count" in TypeScript. The preview then matches the headline path by its mapped (from, to) ends, not by (case index, entry count). Newtype indices (`BodyId`, `SlotId`, …) that serialize as plain numbers can follow as a separate, optional step.
**Proof.** Law: for every offered edit on every preset, each piece named in `made` exists afterwards, and `renumbered` maps every surviving old index to the same piece, compared by content. `check_bindings.sh --write` and `check_wasm.sh --write`.
**Notes.** Stable never-renumbered ids, which would bump the file format, are not justified: the TypeScript rules agree with the core today, and nothing shows a wrong renumbering.

### T13.12 One rule for a bare body
**Change.** Add one `Shape::is_bare(body)`: no member, carries no axis, and not held by a coupling from a fixed axis (the orbiting exception stated once). `clear_axis`, `drop_if_bare` and `drop_member` (`edits.rs:667-680, 790-826`) call it, and `Train::drop_bare` (`conditions.rs:1120-1143`) adds only "named by a hold or case". Unified this way, the gear-core suite passed 604/604. That shows the differences between the four rules are unpinned. It does not show they don't matter.
**Proof.** First decide which rule is intended at each difference: `clear_axis` on a coupled bare body, and `drop_if_bare` on an orbiting coupled body. Write one law per difference (for example Remove(Axis) on a coupled carrier shaft), then unify. The existing remove laws pass unchanged.

### T13.13 An Insert names its preset
**Change.** Change `Edit::Insert` from `{shape, at}` to `{preset: Preset, at}`, with the core building the shape. Keep `Train::insert(shape, at)` public for code and `convert`. `Offer.preset` then duplicates the edit and can go. Update `gear-io/src/strings.rs:1548`, the tests at `edits.rs:1770-1808` and `wasm_probe.mjs:137-199`.
**Proof.** Today the offers at one Spur body are 34,804 bytes, 33,958 of them the ten inserted shapes, against a 2,873-byte train. That round trip happens on every selection and hover. `check_bindings.sh --write`, `check_wasm.sh --write`, and `every_edit_the_train_makes_is_offered` unchanged.

### T13.14 `LoadCase` API: `arranged` keeps its promise, first-entry accessors go
**Change.** `Train::arranged` (`mod.rs:3139`) rebuilds each case as `[first is_load entry moved to input, Load::declared(output, Reacted)]`, keeping kind, duty and figures, which is what its doc promises. Delete `LoadCase::torque/speed/set_torque/set_speed/set_port` (`mod.rs:2986-3015`). Every caller is test code, which indexes `loads[0]` directly, including gear-io `train.rs:591`.
**Proof.** Unit test: `arranged` on [(2,Load),(1,Free)] gives one Load at the input and one Reacted at the output. `cargo build --all-targets` shows no production caller of the removed accessors.

### Declined
None. The one part set aside is [lens-architecture#4]'s step C (stable ids and a format bump), for the reason given under T13.11.
