> **Closed.** The audit this file carried ran from Phase 0 to Phase 8 and every
> finding on its ledger is closed, holds, or is declined with its reason. Its
> faults are logged in [`corrections.md`](../corrections.md), its decisions
> argued in [`rationale.md`](../rationale.md), and its residual biases and
> measurements stated in [`state.md`](../state.md). What survives here, and
> nowhere else, is the **evidence**: the sweeps, the gate runs against broken
> trees, the line counts, the tables that were regenerated and the ones that
> held. Code cites it for that — `docs/history/audit.md`, a finding number —
> and for nothing that governs the tool. It is not maintained; where it
> disagrees with the four documents, they are right.

# Audit

A working document that spanned several sessions, kept so the audit could be
put down and picked up by someone who was not there for the earlier ones.

**Scope.** Audit, ablate and refactor the tool in its entirety: the mathematics
for robustness and closed form, the code for duplication and stray branches, the
tests for what they actually discriminate, and the documents for whether they are
true of the code. Plus a second objective, kept separate throughout: make the
project cheaper to work on without spending the prose that makes it auditable.

**Two rules arrived after the plan was written**, and both are in
`docs/rationale.md` rather than only here, because they govern the tool and not
just this audit:

- **A geartrain has no forward** — the reverse is the same construction with the
  roles swapped, asserted rather than written. It is protocol pass 8, and it
  found two bugs in four sites.
- **A centre distance is the true distance and a clearance is what portion of it
  is clearance** — two relations in three unknowns, so any two of {distance,
  clearance, shifts} are given and the third follows.

Every phase carries a **gate**, and a phase is not done until its gate has been
run *against the fault it is meant to catch*. That is this project's own rule
(`docs/corrections.md`, "A check built from the thing under test measures
nothing") applied to the audit itself.

---

## Status

**Phase 0 — build the instrument.** Done, gate run and passed.
**Phase 1 — truth-up the documents.** Done, gate run and passed.
**Phase 2 — the number ledger.** Done. Four findings, three of them bugs.
**Phase 3 — unify what is written twice.** Done, with pass 8 folded in. F2, F3,
F4, F31, F32, F33, F34, F35, F36, F37, F38, F40 and F41 closed — **six of them
bugs** (F30, F33, F35, F37, F38, F41).
**Phase 3b — the direction sweep, second half.** Done. Opened by the answer to
Q5 below, which settled the one question pass 8 had raised and not resolved.
F42, F43, F44, F45, F46 and F49 closed — **all six bugs**, and F47, F48 recorded
open. The half pass 8 missed is that it swept the *reported* torques and not the
*ratings* built from them.
**Phase 4 — the optimiser.** Done. Six findings, **five of them bugs** (F50, F52,
F53, F62, and the two instruments F57 and F49), plus F63 and F64 on the documents
and the derivation. Q3 answered: **the closed form is not taken**, and the
measurement that says so is below.

*Step 1, done.* The six tuned numbers are `auto::Search`, a value a gate can
raise, and raising it showed the claim beside them **half false**: a pair's
search is converged to 4.0e-7, an epicyclic set's is not converged at all and is
not even monotone in its own effort (F50).

*Step 2, measured.* The loss is **not** monotone in the shift sum, and the
objective is not unimodal in the division either — the trough is where the
automatic addendum cap engages. That names the repair rather than blocking it.

*Step 2, first two items done.* The search sweeps the box its shifts can take
rather than a guess at one (**F52, F53, F56, F57**, and half of F19), and a ring
is asked of its **cutter** rather than of a rack (**F54**) — which turned out to
be the whole of what a set needed, the range having existed all along. 26 of 30
sets were returning a ring the cutter had altered; none are now. The last guessed
interval has left the crate.

*Step 2, third item.* **What is asked of a mesh is asked once** (`MeshTrial`), so
a constraint reaches every arrangement that builds one rather than the one that
happened to write it. It closed **F59** — three kinds, three different answers to
the same question — and moved no number anywhere, which is what makes it an
extraction rather than a change.

*F62, closed.* The walk took **no step at all** whenever the box was narrower
than about eight thousandths of a module — its first step fell below its own
stopping distance, so the loop never ran and the answer was the best of thirteen
grid points. That is the given-centre-distance path, and it cost up to 3.6e-5.
Gated against a scan of the same interval, which shares no step, no direction
and no budget with the walk.

*F50, closed.* A set's search was running **one start of six** — the budget was a
pool and the first walk spent it. Per walk, quadrupling the budget now moves no
answer by a bit, and the set's spread at fourteen times the effort is **1.5e-6**
where it was 3.2e-4. It cost a pair eight times the time, which was recorded as
F61 and is now measured: a quarter of that is genuinely redundant, and Phase 5
declines it for want of an exact repair.

**Phase 7 — F82, an optimiser with nothing to choose.** Done. Turning the
optimiser on and seeing no shift move meant one of two **opposite** things — the
search agreed with the floor, or it found nothing admissible at all — and the
shifts are identical in both. Every kind fell back in the same silence.
`Searched` names the three states and all three choosers report it.

**Phase 7 — F51 and F58, the hula stage's search.** Done. It was the one search
that could not be asked for an effort; it can now, and it **converges** — four of
six fixtures bit-identical at nine times the work, worst 6.5e-7. The 798-line
refactor the finding assumed turned out not to be needed. With that instrument
**F58 is diagnosed and closes as `holds`**: the optimiser moves nothing at
`d ≥ 6` because the optimum *is* the undercut floor, and nothing at `d = 1`
because nothing in the interval is admissible. Neither is a broken search — but
the two are indistinguishable to a reader, which is **F82** and is left open with
its measurement.

**Phase 7 — F55, what a stage says about a distance.** Done. Two silent faults,
not one: a distance no admissible shifts reach was answered with the shifts
unchanged, and — worse — a distance short enough to put the running centres
**inside** the pair's own zero-backlash distance was answered too. That is teeth
overlapping at rest, and the tool reported an efficiency for it. The harness's
own `shifts 9 37` table had been printing three such rows.

**Phase 7 — F79, the interference check.** Done. An internal pair had it twice
over under two classical names and an **external pair had it not at all** — so
the shift optimiser walked into it, and was **recommending gears that foul on 2
of 14 fixture pairs**, both nine-tooth pinions. One signed relation now answers
it for either arrangement and the ring's own two closures are gone. It costs
0.028 of a point on 9/37, and it costs the search something too, which is pinned
rather than hidden. **F81** came out of it and is the more useful finding:
narrowing `TipRoom::clear` left the harness's own filter asking a third of the
question, the corpus caught it, and the diff read like an improvement.

**Phase 8 — F83, the worm is a pair.** Done. The worm stage was a stage
*type* of its own — members that were not gears, no shift, no addendum, a
result unlike a spur pair's — and the model underneath never needed it to be:
both flanks are involute helicoids, a worm is a helical gear with a few starts
at a steep helix. One `PairStage` under two kinds now, one `PairResult` with
the mesh as the one place it branches, one five-input relation for every pair.
The mathematics it took is one line — a shift enters a crossed mesh as a
rack's does, `a₀ = a_ref + (x₁ + x₂) m_n`, exact — and one correction: the
crossed zone was read on both sides of each tangency point, where a member has
no involute, so a fouling tip could count as contact. **Every recorded number
is unchanged**; what the corpus gained is the wheel's shift absorbing a
distance, which is DIN 3975's convention and was not available at all.

**Phase 7 — F7, F19, F21, the figures.** Done. Every table the documents print
is gated — the four hula studies by one test, and **two of the four had
drifted** through Phase 4's repairs. The prose is not tagged and is not a
backlog: it was read for the one class that is live output, *a sentence
attributing a figure to a harness command*, and that class turned up a claim
that has been **false since before the audit began** — `README.md` said the
harness prints ε = 1.777921670 against the surfaces derivation, and it prints
1.758113579, because a stage runs at its operating centre and the script builds
the nominal one. The agreement itself holds, to 4.4e-10, and nothing had ever
asserted it; a test does now. The other 57 paragraphs are classified below.

**Phase 6 — the front end and the payload.** Done. Every finding it carried is
closed, and **three of the five premises did not survive being counted** — F17's
simulator was never in the payload, F15's four forms share sixteen labels of
thirty-five to fifty-three, and F16's eleven files are four hand edits and five
recordings. What the phase actually found is elsewhere: nothing had ever executed
the payload (F72), the over-constraint rule lived untested in the panel (F76),
and four angular names meant degrees in one module and radians in the next (F80).

**Angular units are
unambiguous** (F80): four names meant degrees in one module and radians in the
next, and eight angles stated no unit at all. One of the four had already cost a
live bug — in a call site that read perfectly well and was wrong, against a field
that was *correctly documented one line above*. The radian one carries `_rad`
now, `tools/check_units.py` enforces it, and a second bug turned up in a fixture
that had been building 0.0087° pairs while calling them 0.5°.

**F39 is closed on all four items.** The last is the worm, which has no profile shift, so its *size*
absorbs a given distance — and a screw pair's centre distance turns out not to be
monotone in that size: it has a minimum at `tan γ = (z₁/z₂)^⅓`, closed form at a
right angle, so a target above it is reached by **two** different worms and the
branch is chosen by continuity. Verified against a scan sharing none of its
arithmetic. What is *not* done is the answer's wider half — a worm wheel with the
helical inputs it is denied — which wants an interference check no mesh kind has
(F79).

**A planetary set has a centre distance** (F39 item 3) — the one kind whose geometry a housing most
constrains was the one kind that could not be told about one. A target makes the
layout *easier*: two closed-form sums instead of one Newton iteration, and one
shift left free. It also found **F78**: a set's shifts carry a relation of their
own that a pair's do not, so its two constraints nest, and writing them flat left
the stage over-determined with two groups fighting.

**The clearance is an `Auto` on all four kinds** (F39 item 1, F77) — a plain number could not say
"derived", so the box was read in some states and silently disregarded in others.
It needed a second bound on a freedom group, `automatic_at_most`: a distance and
a clearance cannot *both* be derived, since each is defined from the other. The
planetary declares `0` there because it has no distance input yet, which is the
same statement counted rather than special-cased.

**The toggle model is built** (F76): which of a stage's inputs argue with each other, how many may
stand and which gives way first are `Stage::freedoms` and `Stage::relieved` in
`gear-core`, where they can be tested — they were three untested functions in the
panel, one per stage kind, restating a relation the core already enforces. A worm
declares no freedoms, which is the model working rather than a hole.

**F39's item 2 done**
— mode 3 of the clearance paradigm now holds with the optimiser *off*, which is
the plainest thing a designer does and was returning the undercut floor
regardless of the distance typed. The division rule is the even split projected
onto what each member can be cut at, and it found **F75** on the way: the
optimiser's fallback was discarding the centre distance along with the
optimisation. The toggle model that answers item 4 is specified below and is
wider than F39.

**F47 closed**, and
the half of it that was not in the finding is the *threshold*: a pair's locking
friction was quoted for one direction only, so a forward-locked pair was told a
number about the direction it could drive in. Both are now closed form and the
same expression with the members swapped. No engineering number moved — five
golden files changed and every one is words — and F74 came out of it: an
assertion that passed because one expression rounded to `-0.0`.

**F17 closed and its
premise found false — the simulator was never in the payload, and gating it out
moves **ten bytes**. The payload is 1.51 MB → **1.21 MB** (−19.8 %, −14.5 %
gzipped) with no answer moved, from a TOML dependency that carried a
format-preserving parser, an `opt-level` measured rather than assumed, and a
`wasm-opt` pass. Two findings out of it: **nothing had ever executed the
payload** (F72) and the build was written three times (F73).

**Phase 5 — the tests.** Done. Eight passes of mutation, thirteen findings, and
**two live wrong numbers** among them: an epicyclic set that never asked its own
internal mesh whether the teeth foul (F60), and a screw pair that reported no
flank load when it could not transmit (F48). F12 and F61 were measured and their
premises did not hold; both are recorded as declines with the counting beside
them.

| Phase | What it does | State |
|---|---|---|
| 0 | Golden corpus, figure provenance, `CLAUDE.md` | **done** — gate proven |
| 1 | Truth-up the documents against the code | **done** — gate proven |
| 2 | The number ledger | **done** — gates proven |
| 3 | Unify what is written twice | **done** |
| 3b | The direction sweep, second half — the ratings | **done** — gate proven |
| 4 | The optimiser | **done** — gates proven; the closed form weighed and declined, with its derivation kept |
| 5 | Consolidate the tests | **done** — eight passes, gates proven; two live wrong numbers |
| 6 | Front end and payload | **done** — F15, F16, F17, F39, F47 closed, plus F72–F78 and F80 out of them; F79 opened and scoped |
| 7 | One interference check for every mesh | **done** — F79 closed, gates proven; **a live wrong answer**, and F81 caught by the corpus |
| 7 | What a stage says about a distance it could not reach | **done** — F55 closed, gates proven; two silent faults, the worse being a pair that cannot be assembled |
| 7 | The hula search asked for an effort, and what it found | **done** — F51 closed, F58 diagnosed and closed as `holds`, F82 opened and closed |
| 7 | The figures | **done** — F7, F19 closed; every table gated, two had drifted; one attribution false since before the audit, now held by a test |
| 8 | The worm is a pair | **done** — F83 closed, gates proven; one primitive under two kinds, one result, one relation; every recorded number unchanged |
| 8 | One mesh report for a line and a point | **done** — F84 closed; the two contacts measured at the limit, two seams recorded; the mini-audit of the interrupted run below |
| 8 | The optimiser on a crossed pair | **done** — F85 closed; a pinned member is a constraint in both searches |

**Baseline, measured at `e5e4939`:** 531 tests green in 26.1 s · 13,690 lines of
production code · 10,346 lines of comment in that code · 9,348 lines of
standalone document · 4 `expect` in production, no `unwrap` · 1.49 MB wasm.

Phases 0 and 1 changed no answer: the golden corpus is byte-identical across
both except where `gear-cli matrix` gained a printed spread, which was the point.
Phases 2 onward are gated on that corpus, which is what makes "this refactor
moved no number" a diff rather than a claim.

**Suite: 564 tests** (was 531). **Golden corpus: 27 cases** (was 22).

### Phase 5, in full

Eight passes, and the instrument for all of them is **mutation**: perturb one
production quantity, run the whole suite *and* the corpus *and* the figure
check, record what fires. The scripts are throwaway by design — three lines of
`sed`, a build, a `nextest` run, restore — and the only part worth carrying
forward is the shape:

```
for each mutation:
    back up the file, apply the edit
    cargo build --tests   || record DOES-NOT-COMPILE and restore   # not optional
    cargo nextest run     -> grep 'FAIL ['
    tools/check_golden.sh -> caught / silent
    tools/check_figures.py-> caught / silent
    restore
```

| pass | what it asked | findings |
|---|---|---|
| 1 | the rating **constants** — what fails if each moves? | F65, F66, F67 |
| 2 | the stated **laws** — the proportionalities a stage scales by | F68 |
| 3 | the **boundary** — the wasm crate's own numbers | F24, F70 |
| 4 | the tests that never met their case | F21, F14, F6 |
| 5 | the five modules with no inline tests | F13, F71 |
| 6 | F12, by counting rather than by reading | F12 |
| 7 | F60 — the *reporting* side of "a constraint belongs to the mesh" | F60, and a live wrong answer |
| 8 | F48 and F61 — the two findings carried in from earlier phases | F48, a second live wrong answer; F61 measured and declined |

**Phase 5's remaining work:**

Nothing. The three that stood open at the sixth pass are closed: F60 by the
`TipRoom` extraction, F48 by rating a screw mesh at the load it was given, and
F61 by measurement — see the eighth pass below, which is a decline rather than a
change.

**The question that produced most of this phase**, asked six times and answered
yes six times: *is there an opt-in path the harness never switches on?* F31, F56,
F67, F24, F71 and the sixth in `docs/corrections.md`'s last row. It is protocol
pass 6 and it is worth asking first, not last.

### What is open when Phase 6 closes

Phase 6's own five are all closed; what remains is what earlier phases deferred,
plus the one Phase 6 opened.

| | Finding | Phase |
|---|---|---|
| F79 | No mesh kind has a tip-to-flank interference check — the wider half of F39's item 4, and it would serve every kind | 6, opened |
| F51 | The hula stage's shift search cannot be asked for an effort | 4, deferred |
| F55 | A centre distance no admissible shifts can reach is answered rather than refused | 4, deferred |
| F58 | The hula shift optimiser moves no answer over a band of tooth differences | 4, deferred |

**Phase 7 was what was left**, and it is four deferred findings and one new one
rather than a sweep of its own. F79 is the largest and the only one that adds
mathematics; F55 and F58 are both about a search saying *no answer* clearly; F51
is a refactor the hula stage wants anyway; F7/F19/F21 were tagging, and turned
up one more false sentence on the way. **All of it is closed.** What remains
open is nothing this audit proposes to act on: the prose figures it classifies
rather than gates, and the worm wheel's helical inputs, which F39 records as the
ideal and F79 as the shape it would take.

---

## The four decisions

Settled at the outset, recorded here so they are not re-litigated.

| | Question | Answer |
|---|---|---|
| **Q1** | Is there a compatibility contract on the wire types, the geartrain TOML, the DXF or the CLI's output? | **None.** Any such change is permitted. |
| **Q2** | Bring `WormMemberResult` inside `GearResult`, or record why a crossed member cannot be one? | Answered **bring it inside**; the experiment showed the premise was two propositions with opposite answers, and it was re-answered **B** — `gear: Option<GearResult>`, `Some` for a crossed pair, `None` for a worm. Done. |
| **Q3** | Assert the optimiser's convergence claim, or attempt the closed form? | **Attempt the closed form.** The learnings are worth the effort on their own; assert convergence first regardless, since that step stands alone. **Both done. The convergence claim was half false and is now a gate; the closed form is derived, verified, and declined** — see Phase 4. The learnings were indeed the return: four bugs came out of attempting it, none of them in the mathematics. |
| **Q4** | Loosen the guard conventions toward true degeneracy limits, or document them as conventions? | **Loosen — cautiously.** With the caveat below, which is a constraint on the work and not a preference. |
| **Q5** | Should a member's back-driving torque carry its *own* stage's backward efficiency? | **Yes — and the question was too narrow.** See below. |

### Q5, and why it reopened Phase 3

Pass 8 raised this and did not settle it; the convention in the code said *no*,
on the reading that the load is referred kinematically and the loss applied on
the way to the next stage. The answer given:

> The geartrain should have no concept of forward and backward, and all methods
> should be bi-directional. Zero is a valid torque, as it is a valid speed. A
> geartrain could, theoretically, be non-forward drivable and only
> back-drivable. Forward and reverse are only semantics to aid the end user, and
> while duplicating computation costs more compute, the benefits for future
> geartrains with more complex power flows are usually worth the cost.

So the test is **role-swap symmetry**, not a convention to be chosen: driving
forward, a stage's output member carries its input's torque referred by the ratio
and cut by the forward loss; being driven, the *same construction with the roles
swapped* puts the backward loss on the member the load leaves by. Read that way
the answer is yes, and the number a stage reports becomes the number it hands to
the stage before it rather than that number before its own loss.

Asking it of every site rather than of the one that raised it found **five more
faults**, of which four were in ratings rather than reports — the half pass 8 had
not swept. They are F42 to F46, and two further findings the sweep turned up are
F47 and F48. Three consequences of the answer are now standing rules and are
recorded in `docs/rationale.md` rather than only here:

1. **A load case is a torque *and a direction*.** The peak is the worse of the
   two, taken **after** each direction's own distribution and never before it.
2. **Zero is a load.** A stage carrying nothing rates at nothing and still
   solves; a degenerate limit with a closed form is a value, not a refusal.
3. **A mesh's peak is the mesh's.** Two meshes of one stage need not agree about
   which direction loads them hardest, so the scale is per mesh.

### Q4's caveat, in full

> Watch for limits that trade space. Losing solutions that are more probable for
> ones that are less probable is not necessarily an improvement — though the
> reverse could be true. The relative complexity could multiply for little gain
> if not carefully managed.

So a limit is only loosened where **all three** hold, and each is recorded per
limit in the Phase 2 ledger:

1. **It admits shapes and removes none.** A guard that is currently a clamp
   against one wall may, loosened, allow a search to walk into a region it now
   cannot reach — and if the reachable region shrinks anywhere, that is a trade
   and not a loosening.
2. **The complexity does not multiply.** If honouring the wider domain adds a
   branch, a fallback, or a second construction downstream, the limit stays and
   is documented as a convention instead. A wider domain bought with a new branch
   is the opposite of what this audit is for.
3. **Something measures the difference.** The golden corpus is unchanged (a
   loosened guard cannot move a shape that was already admissible), and a new
   case exercises the newly admitted region.

Where any of the three fails, the limit is **documented as a convention** in
`docs/reference.md` and left alone. That outcome is a success of the rule, not a
failure to act on it.

---

## Findings ledger

Eighteen findings from the first pass, and three the instrument found once it
existed. `F` numbers are stable; nothing is renumbered.

| | Finding | Kind | Phase | State |
|---|---|---|---|---|
| F1 | The crate has one optimiser and the solve inventory omits it | gap | 1, 4 | **half closed** — inventory names it; the closed form is Phase 4 |
| F51 | The hula stage's shift search cannot be asked for an effort, so it is the one search with no convergence gate | gap | 4, 7 | **closed** — `solve_hula_stage_at`; converged, worst 6.5e-7. The refactor the finding assumed was not needed |
| F52 | A given centre distance drops the optimiser onto the undercut floor — the tool's own recommended distance, typed back, costs 0.42 points | gap | 4 | **closed** — and logged in `corrections.md` |
| F53 | The division objective is bimodal at the addendum cap and the search takes the lower peak | gap | 4 | **closed** — " |
| F54 | A ring was asked nothing, so the search chose rings its cutter had to alter — 26 of 30 sets | gap | 4 | **closed** — and logged in `corrections.md`; the premise was wrong, see below |
| F55 | A centre distance no admissible shifts can reach is answered rather than refused | gap | 4, 7 | **closed** — and it was two faults; a *negative* clearance was answered silently too, which is a pair that cannot be assembled |
| F58 | The hula stage's shift optimiser moves no answer over a band of tooth differences, on or off | **holds** | 4, 7 | **closed** — two causes, neither a fault: at `d ≥ 6` the optimum is the floor, at `d = 1` nothing is admissible |
| F82 | An optimiser that finds nothing admissible is indistinguishable from one that agrees with the floor — every kind falls back in silence | **gap** | 7 | **closed** — `Searched`, reported by all three choosers, gates proven |
| F83 | The worm was a stage type rather than a kind: no shift, no addendum, no interference check, no mode 3 but by resizing, a crossed gear pair whose typed shifts reached the tooth report and not the mesh — and the model needed none of it | **gap** | 8 | **closed** — `PairStage` under `Stage::Spur`/`Stage::Worm`, one `PairResult`, the rack law for a crossed shift, the zone read one way from each tangency point; gates proven, corpus unchanged |
| F84 | A line contact and a point contact reported in two types with an enum choosing, though the physics is one model with the shaft angle as a parameter — and nothing had measured where the two contacts' reported figures meet | **gap** | 8 | **closed** — one `MeshReport` for every mesh; the limit measured field by field, two seams named and sized; a patch a quarter as wide as its own rating found on the way |
| F85 | The shift optimiser did not reach a crossed pair, and both searches judged a pinned member's clamps as if its shift were a candidate — a worm's thread never passes, so its wheel was never searched | **gap** | 8 | **closed** — one search with the mesh's own objective; a pinned member is a constraint, not a candidate; gates proven, the harness prints the crossed answer |
| F59 | Three kinds each wrote out what to ask of a mesh, and answered it three ways | gap | 4 | **closed** — `auto::MeshTrial`, and logged in `corrections.md` |
| F60 | A hula pair's tip margin and an internal mesh's interference flags are asked by one kind each | **gap** | 5 | **closed** — `train::TipRoom` on `MeshReport`; and it found the shipped set interfering, see below |
| F56 | No CLI command drove the optimiser, so its answers were outside the corpus | gap | 4 | **closed** — `gear-cli shifts`, which also closes F19's first row |
| F57 | `check_figures.py` had `check_golden.sh`'s stale-binary fault | drift | 4 | **closed** — and logged in `corrections.md` |
| F2 | The worm stage is outside the shared member vocabulary | gap | 3 | **closed** — option B; a crossed member is a `GearResult`, a worm's is not and says why |
| F3 | `GearResult` assembled three times, one field by two formulas | gap | 3 | **closed** — one `GearResult::of`, and the shared rule is `StageTorques::referred_like` |
| F4 | `StageGear` — a shared input type — lives in `train/spur.rs` | drift | 3 | **closed** — moved, with its `Default`, `AddendumAsked` and serde helpers; `spur.rs` 1017 → 730 lines |
| F5 | No ledger of the numbers that are not model constants | gap | 2 | **closed** |
| F6 | The face-width invariance test ran the one model no stage rates with | gap | 5 | **closed** — every model and a rim, six cases |
| F7 | ~212 documented figures, one gate | gap | 0 | **closed** — every table gated by a command or a named test; 57 paragraphs read by class rather than tagged, and the one live class is empty |
| F8 | The CLI list chosen to be exhaustive is not | drift | 1 | **closed** — the table *is* the dispatch |
| F9 | The Layout table names 7 of 27 modules | drift | 1 | **closed** — the map is `CLAUDE.md`; `state.md` keeps the decisions |
| F10 | `bending-check.html` regenerates by hand | drift | 0 | **closed** — `figures-verbatim`, checked exactly |
| F11 | An orphaned sentence fragment in `reference.md` | drift | 1 | **closed** |
| F12 | The inline tests never had the integration tests' consolidation | holds | 5 | **closed** — the premise was measured and does not hold; one law asserted at two standards, levelled |
| F13 | Five production modules carry no inline tests, invisibly | holds | 5 | **closed** — four are covered elsewhere, measured; the fifth hid a dead branch |
| F14 | The two unfired notes need their evidence re-dated | **gap** | 5 | **closed** — one was never fired *at*; the other re-searched at 7× the breadth |
| F15 | `TrainPanel.svelte` is 2,848 lines, four hand-written stage forms | gap | 6 | **closed — the premise is softer than it reads**; the forms share sixteen labels of thirty-five to fifty-three and the file was already factored. The real duplication was one missing row snippet, written out 34 times |
| F16 | One stage input touches eleven files | holds | 6 | **closed — measured**; four hand edits, three documents and five recordings, and no strings unless the label is new |
| F17 | 1.49 MB wasm carrying a simulator no browser path reaches | drift | 6 | **closed — the premise was false**; the payload is 1.21 MB and the simulator was never in it |
| F72 | Nothing ever executed the payload: the boundary's shape was checked, `gear-core`'s values were checked, and the `.wasm` the browser downloads was run by nothing | **gap** | 6 | **closed** — `tools/check_wasm.sh`, gates proven |
| F73 | The wasm build was written out three times — flake, npm script, payload check — so a check could measure a module nobody downloads | gap | 6 | **closed** — `tools/build_wasm.sh` is the one recipe |
| F18 | `CLAUDE.md` is empty | gap | 0 | **closed** |
| F19 | Five documented tables have no command that reproduces them | gap | 4 | **closed** — `:360` from `gear-cli shifts`; the four hula studies by one test, and two had drifted |
| F20 | `state.md` derived a figure by hand from rounded output, and it was wrong | drift | 0 | **closed** |
| F21 | The figure checker cannot see figures in prose, only in tables | gap | 5 | **closed** — it counts both now, and the count was an undercount by an order |
| F22 | The tense rule as written forbade 127 sentences it was not aimed at | drift | 1 | **closed** — the rule was narrowed, not the prose |
| F23 | The gear tab and a stage member bounded the same gear differently | gap | 2 | **closed** — and logged in `corrections.md` |
| F24 | The golden corpus covers the CLI, not the wasm boundary | gap | 5 | **closed** — measured; the values and shape are covered, the boundary's *own* defaults were not |
| F25 | `load_share`'s two ramps do not meet above ε = 2 | gap | 2 | **closed** — and logged in `corrections.md` |
| F26 | No sampling constant had a convergence gate | gap | 2 | **closed** |
| F27 | `SEVER_SCAN_SAMPLES` could not resolve what it looked for | gap | 2 | **closed** — the scan became a solve |
| F28 | A search bound acted as a design limit on the eccentric throw | gap | 2 | **closed** — the bound is now the buildable one |
| F29 | The golden corpus was written from a stale binary and the check caught it | holds | — | **closed** — see Phase 2 notes |
| F30 | A self-locking worm's wheel reported 2.2e307 N·m | gap | 3 | **closed** — and logged in `corrections.md` |
| F31 | No CLI train sets a back-driving load, so the corpus never exercises one | gap | 3 | **closed** — `train mixed` reacts one |
| F32 | `StageResult` has no kind-independent `members()` | gap | 3 | **closed** |
| F34 | `Widths::contact` was not optional, so "no rating sizes this face" had no way to be said | gap | 3 | **closed** |
| F35 | A set reported back-driving torques from the forward distribution — the ring 6 % low | gap | 3 | **closed** — and logged in `corrections.md` |
| F41 | ...and the hula stage had the same fault | gap | 3 | **closed** — found by finishing the sweep |
| F36 | `SpurResult` re-declared `MeshReport`'s seven fields, and the panel re-drew them | gap | 3 | **closed** |
| F37 | A given crank offset was not the offset the stage ran at | gap | 3 | **closed** — and logged in `corrections.md` |
| F38 | The reported clearance was the input echoed, not the gap run at | gap | 3 | **closed** — and logged in `corrections.md` |
| F39 | The clearance paradigm: `Auto` clearance, mode 3 without the optimiser, a planetary distance | gap | 6 | **closed for all four items** — item 4's *minimum* (the worm's size absorbs a distance) is done; its *ideal*, a wheel with helical inputs and an interference check, is recorded as F79 |
| F79 | No mesh kind has a tip-to-flank interference check, which a worm wheel's absorbing shift would need — and which would serve every kind | **gap** | 7 | **closed** — one relation for both kinds; the optimiser had been recommending interfering gears on 2 of 14 fixture pairs |
| F81 | Narrowing `TipRoom::clear` left every caller asking a third of the question, including the harness's own admissibility filter | **gap** | 7 | **closed** — renamed, and `MeshReport::teeth_clear` is the whole question |
| F80 | Four angular names meant degrees in one module and radians in the next, and eight angles stated no unit at all — one of them cost a live bug | **gap** | 6 | **closed** — `_rad` where a name would mean both, and `tools/check_units.py` keeps it true; a second bug found in a fixture |
| F78 | A planetary set's freedoms written as one flat group left it over-determined — its shifts carry a relation of their own that a pair's do not | gap | 6 | **closed** — the shift limit is read from the distance's toggle; the relief test checks every group now |
| F75 | The optimiser's fallback discarded the *centre distance* along with the optimisation whenever its own constraints admitted nothing | **gap** | 6 | **closed** — the fallback is what the constraints imply, not what the stage would build unasked |
| F76 | The over-constraint rule lived in the panel as three per-kind functions, untested in either language | gap | 6 | **closed** — `Stage::freedoms` and `Stage::relieved`, gates proven |
| F77 | A plain `f64` clearance could not say "derived", so the box was read in some states and silently disregarded in others | gap | 6 | **closed** — `Auto<f64>` on all four kinds, and `FreedomGroup` bounds automatics as well as givens |
| F40 | The tolerance band was built four times and its direction asserted nowhere | gap | 3 | **closed** |
| F33 | A crossed pair's members said nothing about their own teeth | gap | 3 | **closed** — and logged in `corrections.md` |
| F42 | A locked mesh reported a torque on the shaft it delivers nothing to | gap | 3b | **closed** — and logged in `corrections.md` |
| F43 | A worm was rated at `η_forward` of the load it was holding — 17 % low | gap | 3b | **closed** — " |
| F44 | A back-driven set rated its ring 6.0 % low in bending, 3.0 % in contact | gap | 3b | **closed** — " |
| F45 | ...and the hula stage 41 % and 23 % low, the same fault | gap | 3b | **closed** — " |
| F46 | A zero force was refused, so a stage at rest could not be solved | gap | 3b | **closed** — " |
| F47 | `Directional::self_locking` asks a directional question one way only | gap | 6 | **closed** — `locked()` and `locking_friction()` are both directional; the forward threshold is closed form and verified |
| F74 | A test asserted that the efficiency at the locking threshold is *signed* locked — true only because one expression happened to round to `-0.0` | holds | 6 | **closed** — found by asking it of the other direction; the claim is now that it vanishes |
| F48 | A screw mesh that transmits nothing reports no flank load | **gap** | 5 | **closed** — rated at the load it was given, on the member given; and the back-driving flank with it |
| F49 | `check_golden.sh` recorded the corpus from whatever binary was on disk | drift | 3b | **closed** — and logged in `corrections.md` |
| F50 | The optimiser's convergence claim was half true: a set's search ran one start of six | gap | 4 | **closed** — and logged in `corrections.md` |
| F61 | A pair pays eight times over for starts that all land on the same point | holds | 5 | **closed — measured and declined**; the premise is out by an order and no exact repair exists |
| F62 | The walk took no step at all on a narrow box, so the answer was the sweep's grid | gap | 4 | **closed** — and logged in `corrections.md` |
| F63 | Three documents said the division is solved; it is searched, and the solver has no production caller | drift | 4 | **closed** — and logged in `corrections.md` |
| F64 | `split_residual` is derived at fixed tip radii, which the default tip cap breaks | gap | 4 | **closed** — stated where it lives; the corrected rate is below |
| F65 | Five rating constants could be perturbed with the whole suite silent | gap | 5 | **closed** — measured; four are the corpus's and it is now in the pre-push list |
| F66 | The load-sharing ramp's constants were guarded by a test written in terms of them | gap | 5 | **closed** — pinned as figures |
| F67 | The harness turned three of a gear's eleven controls | gap | 5 | **closed** — `sweep` turns every gear axis, `train toggles` every stage one |
| F68 | `Loading::at_width`'s exponent was hidden by `PROBE` equalling the default face width | gap | 5 | **closed** — and logged in `corrections.md` |
| F69 | The outline's own promise was untested; its `worst_deviation` measured chord length | gap | 5 | **closed** — " |
| F70 | The panel seeded a face width automatically for two stage kinds of four | gap | 5 | **closed** — and logged in `corrections.md` |
| F71 | `SpurStage::clearance_taken` was a conditional whose every caller made its condition true | gap | 5 | **closed** — and logged in `corrections.md` |

**Kinds.** `gap` — the code and its own stated intent disagree. `drift` — a
document has fallen behind the code. `holds` — checked and sound, recorded so
the next pass does not re-derive it.

### What the first pass found *sound*, and did not change

Recorded so no later session spends itself re-checking. Each was verified
against the code, not taken from the documents: the signed-mesh-kind discipline
(`MeshKind` appears in two `match` expressions in the whole workspace, neither
arithmetic); the absence of panics; both solvers' bracketing, their `None` on
non-convergence and their relative tolerance; the generated boundary and the
three CI consistency checks; `tests/common/mod.rs` as a real fix for the
three-grids fault; the two independent Python verifications still running and
still agreeing; and `strength::ToothOutline` as the seam that makes one bending
model serve both kinds of member.

### What this audit does not propose

- **Adopting any further ISO or AGMA factor.** The reasoning in `rationale.md`
  is sound and `tools/iso_6336_3_stack.py` is the right instrument for reopening
  it. Nothing found here does.
- **Touching the bending model.** The Savage / Dolan–Broghamer pairing is
  coherent and the ISO comparison set is retained for the right reason.
- **Reducing the prose.** It is why the corrections log exists.
- **A fifth document in `docs/`.** The four-document rule is working.

---

## Phase 0 — what was built, and the evidence it works

**`tools/check_golden.sh`** · 23 recorded outputs. Every
`gear-cli` subcommand, one invocation each plus a second where the first left a
regime uncovered. `dump` is a digest — 10.9 MB of raw profile points is a file
nobody reads a diff of. `bending` is deliberately absent: its output *is*
`docs/bending-check.html`, so the useful check is against the document.

> **Gate, run.** Perturbed `K_f`'s `H` coefficient by 0.15 % (0.331 → 0.3315),
> rebuilt, ran the check: **5 files and 38 lines moved and the check failed**.
> Worth noting what it caught that the documented canary would not — `σ_F` still
> rounded to 66.8 MPa, while `K_f` moved 1.8369 → 1.8374. Reverted; clean.

**`tools/check_figures.py`** · provenance markers in the documents, four verbs:
`figures:` (a command regenerates this block), `figures-bold:` (only the bolded
figures are claimed — for a table with a `before` column that is history),
`figures-verbatim:` (this file *is* the output), `figures-by-test:` (a named test
gates it, and the name is checked to exist), and `figures-exempt:` with a reason,
which is section-scoped because history comes in sections.

Coverage when this was written: **6 blocks gated by a command, 2 by a test, 2
sections exempt, 4 tables and 56 paragraphs still ungated** (F19, F21). At the
close of Phase 7 it is 8, 8, 2, **no tables**, and 57 paragraphs classified
rather than tagged — see *F7, F19 and F21* under Phase 7.

> **Gate, run.** Built a worktree at `30679fb` — the commit before
> `9e18527` "the tables had drifted again" — tagged study 5's table as it stood
> there, and ran the checker. It named **1.026, 1.194, 1.081**: exactly the three
> figures a human found by hand and fixed in that commit.

**`CLAUDE.md`** · the map. All 27 core modules with what each *must not know*;
the "to change X, touch these" table, traced rather than guessed; and what each
of the eleven checks catches.

### Two things Phase 0 found on its own

- **F20, closed.** The exemption written for study 5's table was wrong — it
  exempted the very block whose drift motivated the gate. Replacing it with
  `figures-bold` immediately caught that `state.md`'s spread of **0.262** was
  derived by subtracting two *rounded* printed values; the figure is **0.261**.
  `gear-cli matrix` prints the spread now, so nothing derives it by hand.
- **F21, closed in Phase 5.** The coverage report scanned Markdown tables only,
  so a figure quoted in prose was invisible to it — and there were others: **56
  paragraphs** carrying about 150 figures at two decimals or better, against the
  four tables the line reported. An order of magnitude, and the line read as
  though it were the whole story.

  It counts both now and reports them **separately**, which is the point rather
  than a formatting choice: a table of figures is output and something should
  regenerate it, while a figure in a sentence is as often history ("the ring
  came out 6 % low"), an illustration, or a bound quoted from a standard.
  Reading the fifty-six confirms it — `state.md`'s densest two are a change log
  and the known-bias register, the latter measured with a Python tool that is
  not a `gear-cli` command and so cannot be tagged at all. **Tagging them is not
  the work; knowing the number is.** Spot-checked the one case that could have
  gone stale — the last figure of a change chain is the live one, and 61.805 %
  is what `train mixed` prints today.

---

## Phase 1 — the documents against the code

**F8, closed, and the fix is structural.** `gear-cli`'s subcommands are a
`COMMANDS` table that *is* the dispatch: name, argument shape, summary, handler,
and how the golden corpus records it. `gear-cli help` prints it, `state.md`
points at that, and the module comment no longer restates anything. Two tests
hold it — every command says how its output is kept, and names are unique.

The same move closed a hole the corpus had: its case list lived in the shell
script, where a command added in Rust would have been invisible. It is a field on
`Command` now, and `check_golden.sh` asks the binary (`--golden-cases`). A
command recorded *elsewhere* is on that list too, with its reason, and the check
prints it — a coverage claim that omits its own exceptions is how a partial check
comes to read as a complete one.

> **Gate, run.** Pointed `sweep`'s golden case at `show`, ran the suite:
> `every_command_says_how_its_output_is_kept` failed and named it. Restored.

Also collapsed on the way: `args.get(n).and_then(|s| s.parse().ok()).unwrap_or(d)`
was written out forty times and is `arg(a, n, d)`.

**F22, and it is the one place this phase declined to act.** `state.md` said the
other three documents "should not contain the words now, still or currently".
Read literally that forbids 127 sentences, and reading them showed nearly all are
either ordinary English (`Y_β` above 25° "must still be confirmed by experience")
or a model's history stated where the model is argued ("the shaper caps now, by
the same rule"). Neither *dates*. **The word was never the fault**; hedging about
a present another document owns is. So the rule was narrowed to say that, and no
prose was rewritten — a 127-line sweep with a real chance of damage and no effect
on the tool is the shape of change this audit is supposed to refuse.

**F19, deferred to Phase 4 with its reasons.** Five tables in
`reference.md` still have nothing that regenerates them:

| Table | What it would need |
|---|---|
| `:360` least loss vs least shift, 9/37 and 17/43 | a command that sweeps a pair's shift sum for least loss — the optimiser's own question |
| `:1435` the four hula arrangements | `hula` cannot be given four arbitrary tooth counts |
| `:1455` addendum against involute interference | `hula` takes no addendum argument |
| `:1550` each pair optimised alone at `z = 36` | a per-pair optimum, which `hulaband` does not report |
| `:1567` least loss against least shift, per tooth difference | as above |

Each is an answer of `auto::maximise`, which **Phase 4 is about to change**. Any
command written for them now would be written twice. Regenerating these is
therefore part of Phase 4's acceptance rather than a task of its own.

---

## Phase 2 — the number ledger

Every numeric constant in production, sorted by **what kind of claim it makes**.
The sort is the work: a model constant needs a citation, a convergence bound
needs a measurement, a search parameter needs a derivation or an admission, and
a guard needs a reading of *could this gear exist?*

### Model constants — cited, and not in question

`K_f`'s `0.331 / 0.436 / 0.324 / 0.492 / 0.261 / 0.545` (Dolan and Broghamer) ·
`TANGENT_ANGLE_DEG` 30 and `TANGENT_ANGLE_INTERNAL_DEG` 60 (ISO 6336-3:2019,
6.1) · `REVERSED_BENDING_FRACTION` 0.7 (ISO 6336-5, and a Goodman statement) ·
`RAMP_MIN`/`RAMP_MAX` 1/3, 2/3 (the uncalibrated ramp, disclosed as such) ·
`elliptic::TOL` 2.4e-3 and `MAX_STEPS` 200 (Carlson's own stated bound) ·
`PARALLEL_AXES` 0.0 (a named zero, not a tolerance).

### Convergence bounds — each needs a measurement

| Constant | State |
|---|---|
| `SHARING_SAMPLES` 200 | **gated.** `the_sharing_sweep_has_converged` quadruples it; < 1e-4. Finding it failed is what produced F25 |
| `SEVER_SCAN_SAMPLES` 2000 | **gone.** It decided a boolean and was measured ten times too coarse for the case that matters, and no count fixes that — the window closes to zero at the threshold. Replaced by a bracketed solve on `dθ/ds` (F27) |
| `PATH_SAMPLES` 2048 (worm) | **holds, and it was already right**: a second-order convergence *law* is asserted, which fixes what any count is worth. It only ever turned the crossed-pair axis, though, so a worm fixture was added — the same rule, the other end of the family |
| `mesh POINTS` 2700, `FLOOR` 2e-4 | **holds, with the law now asserted.** The floor is still a record of this sweep; what makes it honest is the claim beneath it, that refining the drawing shrinks the residual — which is what separates a discretisation from a disagreement |
| `outline` `MAX_SUBDIVISION_DEPTH` 14, `DEFAULT_CHORD_TOLERANCE` 1e-3 | **holds.** The tolerance is an *input* with a stated meaning (a sagitta in mm), and the depth is a safety stop on it |
| `verify` FLANK 600 / ROUND 300 / TIP 120 / DENSE 3000 / SCAN 400, `MAX_PHASES` 4000, `MAX_ROTATION_STEP` 1e-3 | **open**, and lower priority: `verify` is the instrument rather than the model, and `phase_resolution_has_converged` covers the one that matters most |
| `tooth` `LENGTH_SAMPLES` 60, `MIN_SECTION_SHARE` 0.004, `MIN_SECTION_POINTS` 3 | **holds.** Point *allocation* between sections, which moves no answer — the outline's accuracy is the chord tolerance's job |

### Search parameters — a derivation or an admission

| Constant | State |
|---|---|
| `auto` SPAN 3.0 / SCAN 6 / RESOLUTION 1e-3 / BUDGET 220 / STARTS 2 | **admitted** in `rationale.md` (Phase 1). Phase 4 attempts to retire them |
| `tooth` `BASE_CROSS_GROWTH` 1.6, `CROSSING_GROWTH` 1.4, `MAX_STEPS` 200 | **holds**, and it is already well said: bracket-expansion heuristics before a *guaranteed bracketed* solve, so any values that find a bracket give the same root |
| `POINTED_TOOTH_MAX_ROLL` 50.0 | **holds.** A bracket end at α ≈ 88.9°, stated as such |
| `gear` `MAX_SEARCH_AMPLITUDE` 2.0 | **gone.** It could be reached: at z = 60 the mesh is feasible to 2.1, so the constant bound rather than the geometry and a reachable throw came back unreachable. Replaced by `admissible_ranges`' own bound — one constant fewer, and the search now agrees with the field the gear card draws (F28) |
| `train/hula` ROUNDS 3, SETTLED 1e-3 | **holds, measured.** Quadrupling the cap to 12 changes neither `gear-cli hula 18 0.2` nor `hulaband 18` — a band of tooth differences — by a digit, so the loop settles on its own and the cap never binds |
| `CROSSING_NUDGE_MODULES` 1e-6, `MIN_FILLET_MODULES` 1e-9, `TIP_ABOVE_BASE_FRACTION` 1e-9, `SAME_RACK` 1e-9 | **holds.** Degeneracy epsilons, each at the scale of the quantity it separates |

### Guard conventions — Q4's caveat decided the whole class

Read against *could this gear exist?*, all five could be widened: each sits
inside the limit where the shape stops existing (the axis; zero and the whole
pitch). **None was**, and it is one shared reason rather than five.

**This tool searches, and the optimum sits on these bounds.** The loss falls
monotonically with the length of the path, so the least-loss pair is always the
one whose teeth barely reach — `auto::Freedoms`' own words are that constraints
"say which parts of the plane the answer is not allowed to come from", and
`reference.md#the-hula-stage` reports that "the loss is still falling when the
geometry runs out". Widening a guard therefore does not merely admit a shape a
designer might type. It **moves the answer the tool returns** onto a thinner
tooth, a deeper cut, a root nearer the axis — silently, on a control nobody
touched.

That is Q4's caveat exactly: *losing solutions that are more probable for ones
that are less probable is not an improvement.* Condition 1 fails — not because
the reachable set shrinks, but because the *returned* set moves, which is the
thing the caveat is about.

| Constant | Verdict |
|---|---|
| `MIN_TOOTH_THICKNESS_MODULES` 0.02 | convention, kept — the shift search presses on it |
| `MAX_TOOTH_THICKNESS_FRACTION_OF_PITCH` 0.95 | " |
| `MAX_CUTTER_DEPTH_FRACTION_OF_R` 0.9 | " — and a root at a twentieth of the pitch radius is not a part |
| `MIN_CUTTER_DEPTH_MODULES` 0.05 | convention, kept; the gain from widening is nil |
| `MIN_PRESSURE_ANGLE_DEG` 0.5 | convention, kept. Its stated reason was also wrong — the degeneracy at small `α` is `x_s = π(k−1)/(4 tan α)` running away, not the involute failing |
| `FILLET_FRACTION_OF_MAX` 0.95 | untouched by design: the five per cent *is* the margin argument |

Written into `docs/reference.md#input-ranges` and `params::guard`'s preamble,
which had called the whole group degeneracy tolerances when five of the eight
are conventions. **The rule this produced is narrower than the one an input
field follows**: a *given* number is held to what can exist, a *chosen* one to
what can be made, and where those differ a guard is the second.

Recorded as a success of Q4's rule rather than a failure to act on it.

**Phase 2 is done.** Four findings out of it — F23, F25, F27, F28 — of which
three were bugs and one a constant that could not be justified. Two constants
are gone entirely (`SEVER_SCAN_SAMPLES`, `MAX_SEARCH_AMPLITUDE`), two gained
gates, three were measured and left alone, and five were reclassified from
degeneracy tolerances to conventions with their reason written down.

---

## Q2 revisited — what the experiment found

Q2 was answered **bring `WormMemberResult` inside `GearResult`**, on principle
and as a stress test of the rationale's claim that a stage kind should be new
kinematics and no new rating machinery. The stress test ran. **The premise it
was answered on turned out to be wrong, so the question needs answering again
with what is now known.**

### What was not known when Q2 was answered

**`WormResult` is fed by two different input types.** A *worm stage* is a
`WormStage`, whose members are `WormMember` — a face width and a material,
nothing else. A *crossed gear pair* is a `SpurStage`, whose members are
`StageGear` — shift, addendum, dedendum, root radius, working depth, face
sources, rim thickness, both toggles. `solve_crossed_stage` translates the
second into the first and throws the difference away.

So "bring it inside" is two different propositions:

- For a **worm stage**, a `GearResult` would be mostly absences. A worm is a
  thread and its wheel is the envelope of one; neither is rack-cut, so a profile
  shift, a dedendum, an admissible range and an undercut flag are not values
  those members are missing — they are questions that cannot be put to them.
  Forcing the type would be inventing them, which is what a ring's "no dedendum
  input; it has a cutter" already refuses.
- For a **crossed pair**, a `GearResult` is fully available and was being
  discarded. That half was a real loss and is fixed (F33): a crossed pinion said
  nothing when a cutter had eaten into its flank, where the same pinion with its
  shafts parallel said so.

### What has been done regardless of the answer

F33 is closed — a crossed member reports its clamps and its undercut note — and
`rationale.md`'s claim is corrected from "a member of any kind is a
`GearResult`" to "a member of any kind **that is a gear**", with the
qualification argued rather than asserted.

### Answered: B, and what it took

`WormMemberResult::gear: Option<GearResult>` — `Some` for a crossed pair, `None`
for a worm stage — and `StageResult::members()` on the back of it. Two things
fell out that were not foreseen:

**`Widths::contact` had to become optional (F34).** A crossed member's face is
sized by neither rating: bending is not taken at all, and inverting a contact
stress for a width assumes the stress depends on the width, which a *point*
contact's does not. The alternatives were a zero — the exact fault
`corrections.md` records under "said rather than divided by" — or the continuity
minimum, which is a *geometric* answer and would be the mixing this project
refuses. One construction site, one reader, and `width_for` already handled the
optional `bending` the same way.

**The null-walk gate fired, and was then made narrower.** Two new absences had
to be named with their reasons, which is the gate working. But `contact` is also
a tooth-cycle count and a face-width toggle, so allowing the bare name would
have stopped the gate noticing if either of those went absent. Allowances may be
dotted paths now, and this one is `min_face_width.peak.contact`.

**And a claim did not survive the walk.** The first draft of
`every_member_of_a_reacting_stage_reports_its_share` asserted that a stage's
members agree on the ratio of backward to forward torque. They do not, and the
disagreement is exactly `1/η_forward` on a screw pair — by construction and
correctly, since its output torque carries a forward efficiency the backward
load does not share. The quantitative law holds for parallel-axis kinds and is
asserted there; across kinds only the weak claim holds. **F35 is the question
that leaves**: a planetary's member torques also carry a forward efficiency, and
whether a backward load should be scaled by it is the same question the worm
answered "no". Nothing measures it.

### The question as it was put

| | Option | What it costs, what it buys |
|---|---|---|
| **A** | **Leave it.** `WormMemberResult` stays its own shape; the qualification is documented. | Nothing more to do. F32 (`StageResult::members()`) stays impossible, so the "walk every member of every stage" sweep has to special-case one kind — which is the sweep `docs/corrections.md` recommends and which found F30. |
| **B** | **`WormMemberResult { gear: Option<GearResult>, … }`** — `Some` for a crossed pair, `None` for a worm. | Matches `PlanetResult`/`HulaGear`. Makes `members()` writable and the crossed pair's ranges, notes and per-gear ratings reachable. Costs an `Option` the front end must read, and a wire-type change. |
| **C** | **Split the result types**: a crossed pair returns something with `GearResult` members, a worm stage returns what it has. | The cleanest statement — two arrangements, two shapes — and `members()` is total on four kinds of five. Costs a new result type and a new branch in the front end, against a rationale entry that says a crossed pair is *not* its own kind. |

**Recommendation: B.** It is the only one that makes a crossed pair's members
first-class without claiming a worm's are gears, and the `Option` is the same
"a question that cannot be put to this member" the crate already expresses with
`bending_stress: LoadCase<Option<f64>>`.

---

## Protocol pass 3 — scoping by the output

Run mechanically rather than by reading: every result type's fields extracted,
then grouped by name to find the ones more than one type declares. Two clusters
came out.

**`SpurResult` re-declared all seven of `MeshReport`'s fields (F36).** The type
named for what a parallel-axis mesh reports was used by the planetary, hula and
screw kinds and **not** by the parallel-axis stage. The front end had the same
split, and worse: a `meshRows` snippet for the kinds that carry a `MeshReport`,
and the same rows hand-written again for this one — which had drifted, since the
snippet warns on a transverse contact ratio below one and the copy did not.

Closed by giving `SpurResult` a `mesh: MeshReport` and the panel a call to
`meshRows`. Two things fell out:

- **A spur pair's efficiency and backlash *are* its mesh's** — one mesh, no
  carrier — so they are read through it rather than stored twice, and
  `StageResult::efficiency` is where the kinds are made to agree about which
  level is being asked for.
- **The two backlash shapes met.** A mesh reports the gap per *member*
  (`[Backlash; 2]`); a stage reports it per *drive direction*. Two indexings of
  two numbers, and the mapping was written out in the spur solver as a `match`
  from `Drive` to `MeshSide`. It is `MeshReport::backlash_by_drive` now, in one
  place.

**`WormMemberResult` duplicates six of `GearResult`'s fields** — torque,
back-driving torque, speed, cycles, face width, material. That one is **left**,
and deliberately: those six are what a worm's members have, and a worm's `gear`
is `None`, so they cannot move inside it. For a crossed pair they exist twice and
agree by construction, the inner being built from the outer. It is the price of
option B and is recorded rather than fixed.

Everything else the grouping found — `ratio`, `efficiency`, `backlash`,
`centre_distance`, `clearance`, `notes` on each stage result — is each kind
computing its own, already unified at the *reading* end by `StageResult`'s
accessors. That is the intended shape, not duplication.

**The stage half, traced rather than grouped.** Asking where each kind's
*clearance* rule lives found three methods called `clearance_taken` — and two of
them answer one question while the third answers another. A spur stage's and a
worm stage's is the assembly clearance added to a centre distance; the hula
stage's is the **far-side gap** that sets its crank, which is a geometric
requirement and not running play. The name collides; the rule does not.

What the trace then found is that the hula stage's *running* clearance had no
such rule at all — applied unconditionally, so a given crank offset became the
nominal one and the stage ran 20 µm wider than the number typed (F37). The
field's own documentation described the spur behaviour, not its own.

**The mesh half.** Every field of `MeshReport`, traced across the three kinds
that build one.

- **`efficiency`** — all four kinds apply `Directional::once_moving`, the
  static-versus-sliding rule. *Recorded because the first read said otherwise:*
  a `grep | head` truncated at ten lines and hid the planetary's call, and the
  finding was half-written before the untruncated grep contradicted it. A
  measurement taken through a pipe that can silently drop the answer is not a
  measurement.
- **`backlash`** — one construction written **four times**, once per kind, each
  closing over its own way of turning a distance into an angle. That closure is
  what genuinely differs; the three lines around it were not.
  `Backlash::banded` now, and the direction is gated on every kind (F40): less
  centre distance is less room and so less play, which nothing anywhere had
  asserted.
- **`contact_stress_at_pitch_point`** — two routes. The spur stage solves the
  contact twice, once per load case; the planetary and hula stages solve once
  and scale by `√(torque ratio)`. Both are right, since `σ_H ∝ √T` exactly, and
  the second is what `LoadCase`'s own contract licenses. **Left as it is** and
  recorded: collapsing it would mean either an extra solve on two kinds or a
  scaling on the kind that already has both figures in hand.
- **`operating_pressure_angle`, `coprime`, `relative_radius`, `contact_ratios`**
  — one expression each, differing only in which mesh they are asked of.

**Phase 3 is done.** Nine findings out of it, of which four were bugs
(F30, F33, F37, F38) and one a claim in the documents that was false of the code
(F2). The golden corpus is unchanged across all of it except where a change was
the point.

---

## Phase 3b — the direction sweep, second half

Pass 8 swept the sites that *report* a torque. It did not sweep the sites that
*rate* one, and every kind whose distribution depends on direction had the fault
in both halves — the reports were corrected in Phase 3 and the ratings were left
behind, in the same functions, with nothing comparing them.

**The rule the fix is written from.** A load case is a torque and a direction.
The peak is the worse of the two, taken **after** each direction's own
distribution. Collapsing them to one magnitude at the input shaft first is the
same answer only where the distribution is direction-independent, which is a
parallel-axis mesh and nothing else here — so the spur stage is unchanged to the
bit and is the control.

| Site | Was | Is | Cost of the fault |
|---|---|---|---|
| `StageTorques::at` | `max` of two shaft torques | `on_mesh(forward, backward)`, asked of the input shaft — the general rule, of which `at` is one case | — |
| `Loading::both_cases` | one scale for the stage | one per mesh, since two meshes need not agree about which direction loads them hardest | — |
| planetary | forward distribution at a backward magnitude | each mesh probed at its own peak, cyclic scaled from it | ring **6.0 %** low bending, 3.0 % contact |
| hula | " | " | **41 %** low bending, **23 %** contact |
| worm, rating | `max(T_in, T_back) · i · η_forward` | the worse of `T_in · i · η_forward` and `T_back · i` | **17 %** low — 5803.9 MPa where 6813.6 |
| worm, member | the load referred, before its own loss | ...and after it, which is the number handed to the stage before | 0.0150 N·m on a shaft delivering nothing |
| `hertz::elliptical_contact` | zero load refused | the limit, taken | a worm stage at rest would not solve |

> **Gate, run.** Both new gates against the pre-fix tree in a detached worktree.
> `a_member_is_rated_at_the_load_it_carries` named stage 3 member 2 and printed
> the ratio it expected against the one it got;
> `a_worm_is_rated_at_the_torque_on_its_wheel_from_either_end` printed
> **5803.9 against 6813.6 MPa**. Both pass on the fixed tree. The zero-load gate
> was run against the fault by construction — the test that reaches it was
> written first and failed with `NoContact`.

**The corpus could not have shown any of it, and now can.** `train mixed` carried
a back-driving load the drive still outweighed, so the *peak* case was never the
backward one anywhere in the corpus. `gear-cli train held` is the third regime:
the same train holding 400 N·m at its output, which is what a self-locking worm
is for. Two golden files moved and both are the fix — the worm's back-driving
torque to zero, and a self-locking crossed helix split from "the teeth never come
into contact" to its actual geometry at `0.000 %`.

### What this sweep found and did not fix

- **F47 — `Directional::self_locking` is `backward <= 0.0`.** A stage that cannot
  be driven *forward* has no such flag and no note of its own; it is described
  only by `STAGE_LOW_MESH_EFFICIENCY` reading `0.0 %`. That is the exact case the
  answer to Q5 names — *a geartrain could be non-forward drivable and only
  back-drivable* — and the vocabulary cannot say it. Reachable today:
  `gear-cli crossed 17 23 90` at a 9°/81° split. Deferred because the flag
  crosses the boundary and is read by the panel, so it is a Phase 6 change.
- **F48 — a screw mesh that transmits nothing reports no flank load.** A worm
  stage rates contact from the wheel's torque, `T_in · i · η_forward`, and the
  flanks of a forward-locked pair are pressed by whatever holds it while that
  product is zero. **Measured:** `normal_force(T_in, First, μ)` and
  `normal_force(T_in · i · η, Second, μ)` agree to the bit at four geometries
  spanning μ = 0.03 to 0.30 and η = 0.11 to 0.78 — the two routes are one
  balance, and the *clamp* in `Directional::once_moving` is what breaks the
  identity by zeroing the numerator while the denominator stays finite. So the
  robust form is to rate from the torque the stage is **given**, on the member it
  is given on, which is never degenerate and is identical everywhere else. Not
  done here because the backward direction arrives on the other member and the
  driving flank swaps with it, which is a decision rather than a transcription.

---

## Phase 4 — the brief, written to be picked up cold

**Q3 was answered "attempt the closed form".** What that means concretely:

### What is there now

`auto::maximise` (`crates/gear-core/src/auto.rs`) is the crate's only optimiser:
a bounded box sweep followed by a multi-start pattern walk, carrying six tuned
numbers — `SPAN` 3.0, `SCAN` 6, `RESOLUTION` 1e-3, `BUDGET` 220, `STARTS` 2, and
a first step of `spacing/8`. Every stage that chooses profile shifts for
efficiency calls it. `docs/rationale.md#where-closed-form-is-impossible` names it
as the tenth item beside the nine bracketed solves, which Phase 1 added.

### The two steps, in order

1. **Assert the convergence claim. Done — and it is half false (F50).**
2. **Attempt the closed form.** The question was: *is the loss monotone in the
   shift sum up to whichever constraint binds first?* **Measured, and the answer
   is no** — but the reason is not the physics, and it names the repair. See
   "Step 2, measured" below.

### Step 1, run — and what it found

The six numbers were constants inside `maximise`'s loop, so the claim beside
them could not be raised and could not be checked. They are `auto::Search` now,
a value with `Search::SHIPPED` for every caller and `Search::refined(k)` for the
gate; `SpurStage::shifts_at` and `PlanetaryStage::shifts_at` take one. The
golden corpus is unchanged across the whole refactor, which is what makes "this
moved no number" a diff rather than a claim.

**A pair's search is converged. An epicyclic set's is not.**

| | fixtures | worst move in the objective | worst move in a shift |
|---|---|---|---|
| pair, `shifts_for_efficiency` | 14 tooth pairs | **4.1e-7** (9/20) | under one step of `resolution` |
| set, `PlanetaryStage::shifts` | 30 sun/planet combinations | **2.4e-4** (11/18) | **0.30 modules** (11/14, the sun) |

Three sets — 11/14, 11/17, 11/18 — leave 6e-5 to 2.4e-4 of `η₀` on the table,
and 13/25 another 4.7e-6 at a sun shift 0.19 modules away. Worse, the movement
is **not one-signed**: at 11/21, 11/25 and 17/17 the *refined* search returns a
worse answer than the shipped one, by up to 1.1e-5. A search whose answer is not
monotone in its own effort cannot be fixed by raising the effort.

**The diagnosis is coordinates, not budget**, and the pair is the control that
says so. `shifts_for_efficiency` searches the pair's *own* two directions — the
shift sum, which sets the operating pressure angle and so the length of the
path, and the division, which only moves the path's two ends against each other
— so the flat direction is an axis and the walk climbs it. `PlanetaryStage`
hands `Freedoms` two of its three raw shifts, which are nobody's natural
coordinate: the admissible region is bounded by a curve, the optimum lies
against it, and the walk slides along it on a budget that is a ceiling on
exactly that sliding.

So the set needs the same treatment the pair already has, which is the *same
question* step 2 asks — find the coordinate the constraint is flat in, and
search along the bound rather than across it. **F50 is therefore not separate
work; it is step 2 asked of the set instead of the pair.**

The gate is `the_search_is_converged_not_budgeted`: a law for the pair, and for
the set a **canary pinned at the measured spread** so the fault can only get
smaller, with an assertion that fails once it is fixed and tells the reader to
replace the canary with the law.

**F51, and it is why the hula stage is absent from the table.** Its shift search
lives inside `solve_hula_stage_with` rather than in a chooser of its own, so it
cannot be asked for an effort and has no gate. The other two kinds each have a
`shifts()` the tests already call directly; giving the hula one means lifting
`built`, `pair_of`, `carrier`, `set_with` and `tip_room` out of that function
onto `HulaStage`, which is a refactor worth doing on its own account — that
function is the longest in the crate.

### Step 2, measured — and the two faults it found first

Swept a 9/37 pair: the shift sum from 0.9 to 1.8 in steps of 0.005, and at each
sum the division searched as the tool searches it, scored by solving the stage at
what it chose. Everything below is from that sweep and from a brute 1-mm-per-
thousand scan of the same admissible interval.

**The loss is not monotone in the sum, and the objective is not even unimodal in
the division.** At a sum of 1.21 the efficiency against the pinion's shift has
*two* peaks — 0.9764578 at `x₁ = 0.478` and **0.9766037 at `x₁ = 0.674`** — with
a trough between them at `x₁ ≈ 0.545`.

**The trough is exactly where the automatic addendum cap engages.** Below it the
addendum coefficient is 1.0000; at 0.550 it is 0.9959 and falling, because the
tooth would come to a point and `addendum_asked` caps it. Capping shortens the
tip, which shortens the path of contact, which cuts the sliding loss faster than
the shift alone does — so past the cap the efficiency *rises again*. The two
regimes are the two peaks.

This is not a discontinuity in gearing. It is the tool's own design rule folded
into the objective: `params_at` re-derives the addendum at every trial shift, on
the stated principle that "the geometry that is rated is the geometry that is
built". That principle is right and is not in question. What follows from it is
that **the objective is piecewise, with a slope reversal at a boundary the crate
can compute in closed form** — which is the shape of the repair rather than an
obstacle to it.

#### F52 — a given centre distance drops the optimiser onto the undercut floor

`maximise` opens with a grid at multiples of `span/scan` = **0.5** across
`[−3, 3]`. Pin the sum and the admissible interval in `x₁` is about **0.19 wide**
and contains no multiple of 0.5 — so the sweep collects nothing, `maximise`
returns `None`, and `SpurStage::shifts` falls back to `asked.settled`, the
undercut floor. **The opening scan's step is acting as a feasibility filter.**

Measured on the shipped path, a 9/37 pair with the shift optimiser **on**:

| centre distance | shifts returned | η forward |
|---|---|---|
| automatic | 0.8301 / 0.8398 | **0.977062** |
| 24.1 mm given | 0.6855 / 0.5607 | 0.976649 |
| 24.2 mm given | 0.4736 / 0.0000 | 0.976271 |
| **24.420 mm given** — the distance the tool itself chose | 0.4736 / 0.0000 | **0.972895** |

The last row is the finding in one line: **type in the centre distance the tool
just recommended and it returns a worse gear**, by 0.42 points of efficiency, at
shifts that do not reach the distance asked for. Not idempotent under its own
output, which is F37's fault in a different organ.

#### F53 — the division search returns whichever peak it met first

Where both peaks exist the search takes the lower one. At a given distance of
24.0 mm it returns `x₁ = 0.4746` for 0.9764666 where the brute scan of the same
admissible interval gives `x₁ = 0.672` for 0.9765912 — **1.2e-4 of efficiency**,
and a pinion shift 0.20 modules away. Above 24.1 mm it finds the higher peak, so
the fault is confined to the band where both exist.

#### Done: one place for what a mesh is asked

The direction that produced it: *optimisation is something that can happen at any
mesh, with different constraints and results depending on the mechanics — aiming
at solutions not specific to a mesh type or a stage type allows extension without
duplication.* Run as pass 2 over the three search sites, it found them answering
one question three ways (F59, and `docs/rationale.md` now carries the rule).

| asked of a mesh | pair | set | hula |
|---|---|---|---|
| each member cut as asked | rack only | rack only | rack only |
| a ring asked of its cutter | — | ✗ | ✗ |
| the teeth clear the bottom of the space | ✓ | ✗ | ✗ |
| contact stays continuous | ✓ | ✓ | ✓ |

All four are `auto::MeshTrial`'s now, and every kind hands it the meshes its own
kinematics assembled. `Mesh::bottom_clearance` is one expression for both mesh
kinds — `s·a_w − r_a − s·r_f`, the sign doing the work — gated against the
quarter-module a standard tooth leaves externally and against the two circles'
own separation internally. **No number moved**, on any kind, which is what says
it is an extraction.

#### F50, closed — and both diagnoses before it were wrong

Worth recording as a sequence, because the first two answers were reasoned and
the third was measured.

1. *"The set's coordinates are wrong; its walk slides along a curve."* Plausible,
   and it is even true — but it was not what cost the answer.
2. *"It is the budget, and raising it is not the repair because a pair pays 24×."*
   Half right: it was the budget. The reason was not.
3. **Logged what each walk actually spent**, and the answer was flat: *a walk
   terminates on its own after 80 to 608 evaluations, so the first walk of six
   spent the whole pool of 220 and the other five never ran.* The third start is
   the one that finds the better ridge.

So it was never coordinates, and never a walk that needed more room. It was a
**multi-start search running one start** — which is also why "starts ×6" changed
nothing, the measurement that should have said so two rounds earlier.

The budget is a guard on **one walk** now. Quadrupled, it moves no answer by a
bit — the claim a guard can make and a truncation cannot — and
`the_search_is_converged_not_budgeted` asserts exactly that, alongside the
weaker "everything ×3 moves less than 2e-6" for the grid and the stopping
distance, which move both ways.

| | before | after |
|---|---|---|
| a set's spread at ×14 effort | **3.2e-4** | **1.5e-6**, and both-signed |
| four times the budget | recovered 3.2e-4 | moves **nothing** |
| `shifts epicyclic` 11/18 | 97.0464 % | **97.0775 %** |
| a pair's answer | — | unchanged, at a fifth of the guard too |

**And it costs.** Every start now runs, so an optimised spur stage goes from
0.7 ms to **8 ms** and an epicyclic set from 10 ms to **39 ms**; a three-stage
optimised train is **46 ms**. The optimiser is off by default, so nothing pays
this unless it was asked for, and the timing gate's ceilings were re-measured
with the reason written beside them — 40 / 200 / 20 ms, the multiplier coming
*down* from ten to five so the gate did not go slack while the measurement grew.

**F61 is what is left of the cost.** For a pair, five of its six walks land on
the same point; for the set, the sixth is the one that matters, and no cheap rule
separates the two — the set's winning walk starts from a point whose sweep value
is *below* a result another walk had already reached, so nothing can be pruned by
value. Recorded rather than guessed at.

#### The measurements, kept because two of them mislead

Four knobs, on the four sets that move most — and read at the time as *the
budget is the answer and cannot be paid for*, which was wrong in its second
half:

| | 11/18 | 13/18 | 13/17 | 24/14 |
|---|---|---|---|---|
| shipped | 0.970464 | 0.972258 | 0.971529 | 0.972330 |
| **starts ×6** | *no change* | *no change* | *no change* | *no change* |
| **budget ×10** | 0.970775 | 0.972382 | 0.971587 | 0.972345 |
| everything ×3 | 0.970776 | 0.972382 | 0.971587 | 0.972345 |

**"More starting points buy nothing"** was read as *it is not a basin*. It was
in fact the loudest possible statement of the real fault — there were no more
starting points, because the first walk had spent the pool — and it took a third
round to hear it that way.

**"It is the budget"** was right. **"And raising it is not the repair"** was
wrong: raising the *pool* is not, because a pool sized for the kind that needs
most is spent by the kinds that do not — 24× on a pair, failing
`every_search_is_quick_enough_to_type_over`. Making it a guard *per walk* is,
and it costs each kind only what its own walks cost.

The lesson is the one this project already writes down and I did not follow for
two rounds: **the cheapest instrument is the one that prints what actually
happened.** Logging each walk's spend took one edit and settled in a single run
what two rounds of reasoning had got backwards.

### The repair these three findings agree on

F50 (a set's search is not converged), F52 and F53 have one cause: **the search
is given a box it has no business choosing and a resolution that then decides
feasibility.** `span` = 3.0 and `scan` = 6 are a guess at where shifts live; the
crate already computes where they actually live, in closed form, in
`admissible_ranges` — the same bound the gear card draws and the same one F28
replaced a constant with.

#### Done: the box, and what it reached

**A pair's search now sweeps the interval its shifts can take.**
`auto::searchable_shift` states that interval once — the admissible range, the
caller's floor, and the shift past which the root round asked for no longer
fits, the last **bisected off `admissible_ranges` rather than re-derived beside
it**, since one idea written twice is a place two answers can differ. `Search`
lost its `span` and its `scan` became steps across the caller's box.

| | before | after |
|---|---|---|
| 9/37 at the distance it chose (24.42 mm) | 97.289 %, shifts that do not reach it | **97.706 %**, the free answer to 1e-6 |
| 9/37 at 24.2 mm | 97.627 % | **97.680 %** |
| 9/37 free, bimodal division at 24.0 mm | 97.647 % | **97.648 %**, the higher peak |
| pair convergence, worst of 14 | 4.1e-7 | **4.0e-7** |
| the epicyclic set, the hula stage | — | **unchanged to the bit** |

The one regression is 1.4e-5 on 9/37 at 23.5 mm, a grid artifact, against gains
three orders larger; and a handful of pairs move by 1e-9 to 2e-9, which is the
last step of a different grid. A 9/9 pair *looks* worse by 8.0e-5 and is not:
the old answer was the fallback and its contact ratio of **1.19718** is below the
1.2 the stage asked for, so it was never an admissible answer at all.

#### F54, and the premise it was written on was wrong

It was recorded as *there is no admissible range for a ring's shift*, and the
question put back was the right one: **can a ring ask of its cutter rather than
of a new rack?** It can, and asking it turned the finding inside out.

**Two of the four questions carry over unchanged, and one has no answer.** A
ring's *space* is where the mating pinion's tooth goes and is generated the way a
tooth is, so it takes the identical expression and the same two guards —
`Ring::cut_by` says so where it applies them. `admissible_profile_shift`
therefore **already bounds a ring**, `[−2.130, 1.942]` on the shipped set, and
there was never a second range to write. What has no answer is the *round*: a
ring specifies none, its fillet being its cutter's tip.

So the earlier reading — "give the ring a range" — meant reading a **rack's**
bound onto it, which caps it near 1.2 modules and costs every set 1e-5 to 4.5e-4.
That was the wrong repair, and measuring it is what said so.

**The right one is one line.** `auto::ring_is_cut_as_asked` asks the ring what
its cutter did: [`Ring`] records every guard that altered its geometry, so a
candidate that clamped anything is a shift the tool had to be talked out of —
which is exactly what `member_is_buildable` refuses on a rack-cut member. It is
free, because every caller has already cut the ring to get the mesh it is
scoring.

| | before | after |
|---|---|---|
| sets returning a ring the cutter altered | **26 of 30** | **0 of 30** |
| the shipped 13/25's ring | x = 2.35, space capped at 1.94 | x = 1.94, pressed against the cap |
| `η₀`, all 30 | — | falls 4e-7 to 6.2e-4 — the part is now one that exists |
| more effort finding a *worse* answer | 3 of 30 | **none**, and it is asserted |

The efficiencies fall, and that is the answer being right rather than large: the
higher figures were the efficiency of a part nobody makes. The hula stage is
asked the same question in the same place, for the reason
`docs/corrections.md` records of the last rule that reached one epicyclic kind
and not the other.

**`Search::fallback_box` is gone with it** — every caller states its own box now,
so the last of the guessed intervals has left the crate.

**F50 is not closed by this.** The set's spread is 3.2e-4 and one fixture (13/18)
loses 1.2e-4 to under-search, but the *character* has changed: every move is now
one-signed, so more effort only ever helps. That is asserted as a law beside the
canary. What is left is the coordinate problem alone.

#### Still to do

So step 2's work is:

1. ~~Give `maximise` its box.~~ **Done for a pair; F54 blocks the set.**
2. ~~Give a ring an admissible shift range.~~ **Done, and the premise was
   wrong** — the range already existed; what was missing was the cutter's
   question. See F54 above.
3. ~~Stop the walk sliding.~~ **Done, and it was not the sliding** — it was a
   budget shared across the starts, so five of six never ran. See F50 above.
4. **Split the box at the regime boundaries.** The addendum cap's onset is the
   shift at which the tip reaches its minimum width, and
   `addendum_for_tip_width` is already the closed-form solve for it. Cut the
   division's interval there and each piece is smooth.
5. ~~Then the closed form applies, piecewise.~~ **Weighed and not taken** — see
   below. The derivation is finished and verified; what it buys is not worth what
   it costs.
6. **Then the closed form applies, piecewise.** On a smooth piece the division's
   stationary condition is `contact::split_residual`, already derived and already
   bracketed by `efficient_split`. Solve each piece and take the best. **The
   division stops being searched at all.**
7. **Ask the sum the same question.** With the division closed-form at every sum,
   the sum is one dimension against the active bound — which is what step 2 set
   out to establish, now with the reason the first attempt would have failed.

That sequence also answers F50: a set's admissible region is a curve because its
box is wrong, not because its geometry is hostile.

**This moves answers**, which the earlier steps deliberately did not. Every moved
figure in the golden corpus and in `reference.md`'s five tables has to be
explained as a *better* optimum rather than a different one — and the test for
that is F52's row above: the answer at a given distance must equal the answer the
free search finds at that distance.

#### The closed form: finished, verified, and not taken

**The premise was false.** The brief opened with *"half of the problem already
has one: at a fixed shift sum, the stationary condition for the division is
derived and solved directly"*, and three documents said so. It is derived and
checked; **nothing in production calls it** (F63). A grep for the function's
callers settles that in a second, and it survived because each document was
checked against the other documents. It surfaced the way a false claim about
code usually does — by trying to build on it.

**And the derivation was incomplete for the shipped default** (F64). `1/sin α_a`
is `dξ/dr_a`, how far a path end moves per millimetre of tip; turning it into a
rate per unit *shift* wants `dr_a/dx`, which is `m` only while the addendum is a
number somebody gave. `no_sharp_tip` is **on by default** and holds a capped tip
at the radius where the tooth is `min_tip_width` wide. Differentiating that
condition:

```text
dr_a/dx = 2 r_a c / (2 tan α_a − w_min/r_a),      c = 2 tan α_n / z
```

> **Verified.** Against finite differences of the tooth the stage actually
> builds, at z = 9, 17 and 37 across the shift range: **exact to six figures in
> both regimes**, `1.000000` uncapped and `1.000000` capped. On a nine-tooth
> pinion the capped rate is about **0.47 m**, and it differs between the two
> members — so the factor that used to cancel does not.

Measured against a brute scan of the same interval, the existing solver behaves
exactly as that predicts: it lands within 1e-4 of the best division wherever the
cap is idle, and returns `None` or a third of the interval away wherever it is
not.

**Why it is not wired in.** The corrected condition supplies the *interior*
candidates only, and the measurement says the optimum is at an **end of the
admissible interval** about as often as inside it — 9/37 sits on its undercut
floor from a shift sum of 0.5 to 1.1, and 17/43 on the same floor above 1.4. So
using it means bracketing the interval's two ends, the cap's onset, and a
stationary point per smooth piece: **five bracketed solves where a bounded
one-dimensional search is one**, each costing a path build, for an answer already
right to a thousandth of a module — below what the tool prints.

That is Q4's caveat, met in the last place it was expected: *the relative
complexity could multiply for little gain.* The derivation is kept, stated where
it lives, and the documents now say what the code does.

### What the acceptance has to be

- **Identical answers on every documented table**, to the digits the document
  prints. The tables are in `docs/reference.md#the-hula-stage` and
  `#efficiency-parallel-axes`; two of them are gated by
  `the_documented_tables_are_the_ones_this_code_prints` and one by
  `tools/check_figures.py`.
- **The golden corpus unchanged**, or every moved figure explained.
- **F19 closed on the way.** Five tables in `reference.md` still have no command
  that reproduces them (`:360`, `:1435`, `:1455`, `:1550`, `:1567`), and all five
  are `maximise`'s answers. They were deferred to this phase precisely because
  any command written for them before the search changes would be written twice.

### The trap to expect

The bounds are what the answer sits on, so **a change to the search is a change
to which bound binds**. `docs/rationale.md#an-input-limit-means-could-this-gear-exist`
and Q4's caveat both bear on this: widening a guard moves the returned answer
even though it admits no new shape a designer would type. If the closed form
lands the answer somewhere new, check whether it is a better optimum or a
different constraint before believing it.

---

## The clearance paradigm, and what each kind is short of it

**The model, as stated:** a centre distance is the **true** distance, and a
clearance says what portion of it is clearance. So

```text
centre distance = zero-backlash distance + clearance
zero-backlash distance = f(the shifts)
```

— two relations in three unknowns, so **any two of the three are given and the
third follows**, and there are three working modes:

| | Given | Derived |
|---|---|---|
| **1** | clearance, shifts | the distance: nominal + clearance |
| **2** | the distance, shifts | the clearance: distance − nominal |
| **3** | the distance, clearance | the shifts, solved to reach `distance − clearance` |

### Where each kind stands

| kind | distance input | clearance input | 1 | 2 | 3 |
|---|---|---|---|---|---|
| spur | `Auto<f64>` | `f64` | ✅ | geometry right, **was mis-reported** | only with the shift optimiser **on** |
| worm | `Auto<f64>` | `f64` | ✅ | geometry right, **was mis-reported** | ✗ — nothing inside the stage is free unless the worm's diameter is |
| planetary | **none** | `f64` | ✅ | ✗ | ✗ |
| hula | `offset: Auto<f64>` | `running_clearance: f64` | ✅ | ✗ | ✗ |

**Done (F38).** Mode 2's reporting, on the two kinds that report a clearance at
all: it is `centre_distance − centre_distance_nominal` now, derived rather than
echoed. It was the input read back where something was free to absorb it and
**zero otherwise** — so a pair told to run at 30.3 mm whose shifts put it at
30.0057 reported 0.02 or 0.000 against an actual gap of 0.294.

### What the paradigm still needs — scheduled, not done

1. **`clearance` becomes `Auto<f64>`, as the distance already is.** Mode 2 is
   "the clearance is *derived*", and a plain `f64` has no way to say that. With
   both `Auto`, the mode falls out of which two are given, exactly as the pair's
   `{a, x₁, x₂}` already does — and the over-determined corner gets the same
   visible relief the front end already gives that one.
2. **Mode 3 without the optimiser (spur).** A given distance and a given
   clearance should solve the shifts to reach `a − clearance` whether or not the
   stage is optimising for efficiency. `mesh::shift_sum_for` is already the
   function, and the optimiser path already calls it; what is missing is the
   plain path calling it too. **This is the largest of the four and the only one
   that moves an answer.**
3. **A planetary needs a centre-distance input.** It has none: the common
   distance falls out of the shift relation. The machinery is there — the set
   already solves *one* shift to make two distances agree, and already picks
   which member absorbs — so a target distance is one more constraint on the
   same solve rather than new kinematics.
4. **A worm's mode 3 needs a free variable named.** A worm has no profile shift,
   so nothing inside the stage can move to meet a given distance at a given
   clearance. `FirstMemberSizing` is the candidate — the worm's diameter — and
   whether that is the intended absorber is a design question rather than a
   defect.

Item 4 is a question; 1–3 are work. None of them changes the mathematics — they
change which of three related numbers a designer states and which the tool
derives.

---

## Phase 5 — the tests, and what they discriminate

Not "are there enough" but "what would have to break for one to fail". Run as
**mutation**: perturb one production quantity, run the whole suite, count.

| perturbed | tests failing | golden | figures |
|---|---|---|---|
| `inv α`, by 2 parts in 10⁴ | **50** | — | — |
| the loss integral, by 0.1 % | 6 | — | — |
| `K_f`'s `H`, 0.331 → 0.3315 | **0** | caught | caught |
| `K_f`'s `L`, 0.324 → 0.325 | **0** | caught | caught |
| ISO `Y_S`, 1.2 → 1.21 | **0** | caught | caught |
| `TANGENT_ANGLE_DEG`, 30 → 30.1 | **0** | caught | caught |
| `REVERSED_BENDING_FRACTION`, 0.7 → 0.71 | **0** | **silent** | **silent** |
| `RAMP_MIN`, ⅓ → 0.34 | **0** | **silent** | **silent** |

**The geometry is gated to the bit and the strength model was not gated at all**
— which is the shape of a suite grown one geometric learning at a time, and is
exactly what the audit was asked to look for.

Three findings, and they are different from one another.

**F65 — four of them are the corpus's job, and the advice omitted it.** A change
detector is the right instrument for a *cited* constant: nothing derives `K_f`'s
coefficients, so there is no law to assert, only a figure that must not move by
accident. `tools/check_golden.sh` catches all four. What was wrong is that
`CLAUDE.md`'s "before pushing, run four" did not list it, so a developer
following this repository's own advice would not see a mutated strength model
until CI. It runs five now, with the measurement as the reason.

**F66 — the ramp was guarded by a test written in terms of itself.**
`the_load_share_is_continuous_and_unchanged_below_two` is thorough about *shape*
and names `RAMP_MIN`/`RAMP_MAX` on both sides of every comparison, so moving
either moves the comparison with it. That is `docs/corrections.md`'s "a gate on
a ratio cannot see a scale error", recurring. And nothing else could see them,
for a reason worth keeping: below `ε = 2` the governing point *is* the
single-pair boundary, where the share is exactly one — so the ramp reaches no
answer the tool reports, and a corpus case cannot be contrived to catch it
without leaving the band the model is for. Pinned as figures instead, which is
the right instrument for a number disclosed as uncalibrated.

**F67 — the harness turned three of a gear's eleven controls.** `gear-cli sweep`
*is* the parameter grid and it swept four axes; `reversed_bending`,
`load_sharing`, `no_undercut`, `no_sharp_tip`, `rim_thickness` and
`material_overrides` were never switched anywhere in the harness. So their
constants were outside the detector by construction — the same fault as the
back-driving load (F31) and the optimiser (F56), for the third and fourth time.
**The pattern is now named**: *an opt-in the harness never switches on is a path
the detector cannot see.* `sweep` turns every gear axis (5,916 cases → 94,656,
in 59 ms) and `gear-cli train toggles` turns every stage one.

> **Gate, run.** Each of the three constants perturbed again afterwards:
> `RAMP_MIN` and `RAMP_MAX` now fail **two tests each** where they failed none,
> and `REVERSED_BENDING_FRACTION` is caught by the corpus where it was silent
> everywhere. Measured against the *recorded* corpus, not against a case list
> with a missing file — the first run of this check reported a false catch for
> exactly that reason.

### Phase 5, second pass — the laws, not the constants

The same question asked of the crate's stated *proportionalities*, which are
load-bearing: a stage rates once and scales, so a wrong exponent is a wrong
answer everywhere the widths or the torques differ.

| stated law | tests failing when broken |
|---|---|
| `Loading::under` — contact goes as √torque | 2 |
| `Loading::under` — bending is linear in torque | 1, and the corpus |
| `min_face_width_contact` — the square | 3, and the corpus |
| **`Loading::at_width` — contact goes as 1/√width** | **0** |
| Willis's basic ratio | 7, and the corpus |
| the span's nominal | 3 |
| **the outline's chord tolerance** | **0** |
| **the subdivision's stop** | **0** |

**F68 — a probe width and a default face width happened to be the same number.**
`PROBE` is 10.0 and `StageGear`'s default face width is 10.0, so every shipped
case scales by exactly one and the exponent could have been anything. *Two
unrelated numbers that happen to be equal will hide whatever lies between them.*

**F69 — the outline's own promise was untested, and its test was named for it.**
`worst_deviation` returned the longest **chord**, with the real computation
abandoned mid-line: `let _ = mid_r;` sits in the committed source where the
distance to the profile was going to be worked out. Everything built on it was
relative — tighter tolerance, shorter chords, more vertices — and a relative test
cannot see a scale move.

Measured properly, against the gear's own dense sampler, the outline is inside
**3×** its tolerance and converges on it: 2.7× at 1e-2, 1.8× at 1e-3, 0.99× at
1e-4, worst on an undercut tooth. Subdivision stops on a span's *midpoint*
sagitta and a re-entrant flank's worst deviation exceeds it, which is the
mechanism and is worth a designer knowing.

> **And the first version of that measurement measured the reference.** It
> reported three times the tolerance, and the shape was the giveaway — the
> deviation *rose* as the tolerance tightened, which is impossible for a
> convergent scheme. The polyline had overtaken the curve it was compared
> against. The reference is derived from the outline's own vertex count now, ten
> times denser than whatever it judges. *A reference is only a reference while it
> is finer than its subject.*

> **Gate, run.** Loosening the subdivision test by 3× fails the new law and
> nothing else; doubling `DEFAULT_CHORD_TOLERANCE` fails the fallback test;
> cutting `MAX_SUBDIVISION_DEPTH` from 14 to 10 fails the bounded-count test.
> Each of the three was silent across all 558 tests and all 27 golden files
> before.

### Phase 5, third pass — the boundary

**F24 said the corpus covers the CLI and not the wasm boundary. Measured, most
of it is covered** — and by the right things rather than by luck. The boundary's
*shape* is `tools/check_bindings.sh`'s, since `ts-rs` generates the front end's
types from the Rust ones; its *values* are `gear-core`'s, and the corpus sees
them because `gear-cli` builds its stages from the same `Default`. Two defaults
probed at random — `min_tip_width`, `min_contact_ratio` — were both caught.

**What is not covered is the handful of numbers the boundary invents.** Rule 1
is that if a number appears in the UI, Rust computed it, *and a default is one of
those numbers*. These exist only in `defaults_impl`:

| | seen by |
|---|---|
| the gear tab's tooth count, 9 | **nothing** |
| the pin diameter, 1.75 mm | **nothing** |
| the eccentric throw, 0.1 | **nothing** |
| the face width a fresh panel seeds | **nothing** |
| a fresh train's 30,000 rpm and 0.1 N·m | **nothing** |

Not a test, not the corpus, not the binding check. Pinned as figures now.

**F70, which writing that canary found.** The rule beside the code is that a
designer opening a stage should see *the width the rating asks for*, seeded at
5 mm. It had reached a parallel pair and an epicyclic set:

| kind the panel opens | the width it started at |
|---|---|
| spur, planetary | automatic, 5 mm seed |
| worm | automatic, 10 mm seed |
| **hula** | **fixed, 10 mm** |

Three answers to one question, decided by which stage a designer happened to
pick — and a fresh hula panel showing a width nobody chose and no rating sized.
Found by writing the test to walk **every member of every kind the panel
offers** rather than the two the rule had reached.

> **Gate, run.** Each of the four boundary defaults perturbed again: all four now
> fail `the_defaults_this_boundary_invents_are_the_ones_it_shipped`, and each was
> silent across the whole suite, the whole corpus and the binding check before.

### Phase 5, fourth pass — the tests that never met their case

**F21, and the number was out by an order.** The figure checker scanned Markdown
tables only, so its coverage line — "5 tables still ungated" — was true and read
as though it were the whole story. Widened to prose: **4 tables and 56
paragraphs**, about 150 figures at two decimals or better. They are reported
*separately* rather than failed, and that is the finding rather than a formatting
choice: a table of figures is output and something should regenerate it, while a
figure in a sentence is as often history, an illustration, or a bound quoted from
a standard. Reading the fifty-six confirms it — `state.md`'s two densest are a
change log and the known-bias register, the latter measured with a Python tool
that is not a `gear-cli` command and so cannot be tagged at all. **Tagging them
is not the work; knowing the number is.**

**F14, and it was not a re-dating.** Two notes sat in `UNFIRED` as "looked for
and not found".

- `clamp.ring_fully_filleted` — re-searched over **71,750** combinations against
  the original 11,000, and it still never fires. Evidence re-dated, which is what
  the finding asked for.
- `stage.ring_addendum_clamped` — **fires on 441 of the 1,482 sets** a sweep can
  solve. It was never fired *at*: the sweep carried five cases aimed at it and
  every one failed with *the teeth never come into contact*, inside an
  `if let Ok(…)` that said nothing.

Two faults in those five, and both are the kind that hide: they **raised** the
ring's addendum, where a ring's tip is `r − m(h_a − x)` and reaches its base
circle on a *short* one against a negative shift; and they left the ring's tooth
count at `StageGear`'s own 17 while setting the sun's and the planet's — a ring
that does not close the set it is in. *A case that cannot solve is not a case*,
and an `if let Ok` around one is how it stays that way quietly.

### Phase 5, fifth pass — the five modules with no inline tests

**F13 asked whether five `gear-core` modules carrying no `#[cfg(test)]` are
untested or merely tested elsewhere.** Answered by mutation rather than by
reading:

| module | perturbed | caught by |
|---|---|---|
| `metrology.rs` | the span's nominal | 3 integration tests |
| `params.rs` | `MAX_CUTTER_DEPTH_FRACTION_OF_R` | 2 tests, and the corpus |
| `params.rs` | `MIN_TOOTH_THICKNESS_MODULES` | the corpus |
| `tooth.rs` | `CROSSING_NUDGE_MODULES`, `LENGTH_SAMPLES` | the corpus |
| `verify.rs` | `MAX_ROTATION_STEP` | 2 integration tests |
| **`train/spur.rs`** | `clearance_taken`'s `else` arm | **nothing** |

So four of the five are covered, and the finding is a *labelling* one for them —
which the map now says. The fifth was not covered, and mutating it explains why.

**F71 — a conditional that decided nothing, explaining a rule it did not
enforce.** `SpurStage::clearance_taken` returned the clearance "wherever
anything is free to absorb it" and zero otherwise, with four paragraphs on which
case is which. **Every caller already ran where its condition held**: the centre
distance reads it only when automatic, and the shift search only when the
optimiser is on. The zero arm was unreachable in effect, and replacing the whole
method with the field moved no test and no recorded figure.

The rule it describes is real, and is enforced by *where the clearance is read*.
It is stated there now, and a method that suggested there were two answers is
gone — rule 4, met somewhere worse than a `match`: a branch that decides nothing
still tells a reader that something is decided.

**And the reason nothing could see it is the sixth opt-in path.** A spur stage
with a centre distance **given** and the optimiser **off** — the plainest thing a
designer does — was in no recorded case: every one either left the distance
automatic or turned the optimiser on. `gear-cli shifts` prints that combination
now, beside the optimised one.

**F6, closed on the way.** The face-width invariance ran on `FormFactorOnly`
alone, the one notch model no stage rates with — it ships through
`gear-cli matrix`, and the stages all use Dolan–Broghamer. Nothing was wrong with
the answer, since none of the three reads a face width, which is *why* the
invariant holds; but a property asserted of one arm of a `match` is asserted of
one arm of a `match`. Six cases now: three models against a rim silent and
biting.

### Phase 5, sixth pass — F12, and what counting said about it

**The finding as recorded:** `tests/` was pulled onto one shared grid and the
`#[cfg(test)]` blocks in `src/` were left as the one-off fixtures they grew from,
so the inline tests want the same consolidation. Plausible, and read rather than
measured — so it was measured.

| asked | answer |
|---|---|
| mutations run | 32 |
| test firings | 122 |
| distinct tests that fired | 101 |
| tests firing on more than one mutation | 19 |
| pairs where one test's firings are a subset of another's | none that are about the same thing |
| helper functions duplicated across modules | 2 (`fn pair(z1, z2)`, `fn library()`) |

**So the premise is largely false.** There is no measurable redundancy: the 19
tests that fire more than once are broad canaries — the regression fixture, the
string sweep, the boundary walk — and the things they share firing on are
unrelated. The nested-loop sweeps in `src/` are not the three-grids fault
either; each turns the axes its own law needs, which is what
`docs/corrections.md`'s "an axis nobody turns" asks for and not what a shared
grid would give it.

**What counting did find** came from clustering the tests by **subject** rather
than by file: *nine* thickness-modification tests across five files, two of which
state the same law for the two kinds at very different standards.
`ring.rs::a_thickness_modification_moves_no_radius_on_a_ring` sweeps three tooth
counts × three shifts × four values of `k`, checks the cutter's plunge as well as
the radii, and carves out the one case the rule bends — a space that closes on
itself. `tests/geometry_laws.rs::thickness_modification_leaves_radial_dimensions_alone`
asserted the same rule on **one default gear**.

**Not merged, and deliberately.** A ring needs a cutter to be cut by and has a
limit an external gear has not; merging them would put a `match kind` inside a
law. What they share now is the *standard*: the external half runs the shared
grid × five values of `k`, 540 gears, and the law splits where the wider sweep
says it does — `r` and `rb` exact always, `ra` and `rf` exact unless the profile
truncated them, and `st` obliged to have moved or the test asserts that a control
does nothing.

The widened sweep found its own carve-out immediately: `z=3, x=−0.5, α=14.5°,
k=0.6` moves the tip from 0.2082 to 0.1726 mm. The tooth is **severed** — the tip
is set by what survives rather than by the addendum asked for — which is the
exact mirror of the ring's space-closed case. One law, two kinds, one place each
where it bends, and the same reason both times.

> **Gate, run.** Giving `rb` a `k`-and-helix dependence is caught by this test
> and by `metrology::consecutive_spans_differ_by_one_base_pitch`, and **could not
> have been caught before**: the factor is one at β = 0 and the old fixture never
> turned the helix axis.

**And the harness had a blind spot, found on the way.** It edited a constant,
rebuilt, and grepped for `FAIL [`. A mutation naming a binding out of scope fails
to *build*, so there is no `FAIL` line and the harness recorded "no test caught
it" — its most interesting possible finding — for what was a typo. It builds
first and records `DOES-NOT-COMPILE` separately now. *An instrument whose failure
mode is indistinguishable from its most interesting finding will hand you that
finding.*


### Phase 5, seventh pass — F60, and the finding under it

**The finding as recorded:** a hula pair's tip margin and an internal mesh's
interference flags are asked by one kind each, and they are the next candidates
for the mesh level — the continuation of `auto::MeshTrial`, which closed F59 by
putting *what is asked of a mesh* in one place.

**The half that was left** is what the asking **found**. `MeshTrial` had taken
the constraints; the four numbers a reader is given were still four fields on
`HulaMesh`, computed in the hula stage's file and drawn in the hula stage's form.
An epicyclic set has the same internal mesh in it and reported none of them. *A
designer was told whether their teeth foul according to which stage kind they
had picked.*

One `train::TipRoom` on `MeshReport` now, `None` on an external mesh — which is
not three answers of `false` but a question that does not arise, since an
external pair's tip circles cross on the line of centres or not at all. The
search reads the same value as its fourth refusal, so the reader and the
constraint cannot come to different conclusions, and the hula's own pre-check —
one of the three, asked immediately before building the trial — is gone.

**And it found a live wrong answer.** `ring::mesh_with` has known since it
existed that a full-depth internal pair interferes: the ring's tip can only touch
the pinion's involute while `√(r_a2² − r_b2²) ≥ a sin α_w`, and `ring.rs` carries
a test saying a 60-tooth ring misses it against every pinion from 20 to 40 teeth.
**Nothing ever put the question to a set.** The shipped 24/18/60 — full-depth
ring at zero shift — has involute interference on its planet-ring mesh and had
never said so. It says so now, and `gear-cli planetstage` prints it, which is
what puts it in the change detector.

| measured | before wiring it in |
|---|---|
| epicyclic sets swept, ring shift pinned | 30 |
| answers the new refusal moves | **2**, both under 5e-5 of `η₀` |
| numbers moved in the whole corpus | **1** — 17/17's shift division, by 0.002 modules, at the same efficiency to four decimals |
| searches that returned nothing where the ring is full depth | all of them, which is the refusal being right |

The last row is the one to read twice: with a full-depth ring **every** candidate
fouls, so the optimiser now finds nothing admissible and falls back to the plain
shifts. That is correct and it is not silent — the mesh row says which condition
bit, and the remedy is the ring's addendum, which is an input. Shortening it to
0.75 clears the same set, and that is the second half of the gate: *a canary that
only watches the shipped set interfere would pass if everything interfered for a
new reason.*

`StageResult::meshes()` arrives beside `members()` for the reason `members()`
exists — a walk that names the kinds is a walk that forgets one — and the panel
draws the row from the shared `meshRows` snippet, whose own comment already
records this fault happening once before, to the axial-overlap warning.

> **Gate, run.** A set reporting no tip room fails two tests; the refusal
> relieved fails a third and moves two corpus files.


### Phase 5, eighth pass — the two carried findings

**F48, closed, and it was a live wrong number.** A screw pair's contact was rated
from the torque on its **wheel**, `T_in · i · η_forward` — the conservative
reading of the friction balance, and everywhere the pair transmits the same
number as the input torque read on the worm. `Directional::once_moving` clamps a
locked pair's efficiency to zero, so that product is zero and a forward-locked
pair reported **no flank load at all**.

It was in the corpus the whole time. `gear-cli crossed 17 23 90` prints a 9°/81°
split that cannot drive forward at µ = 0.06, and it printed `0.0` between
neighbours reading 1273. It prints 1176.1 now.

| what moved | by |
|---|---|
| the locked split | 0.0 → **1176.1 MPa** |
| the splits that do transmit | under 0.03 % |
| a near-parallel crossed pair's pitch-point rating | 1 % — the wheel torque carried a **path** efficiency into a **pitch-point** balance |
| the back-driven worm's peak | −0.8 %, onto its own flank |

Three faults, one change. The first is the degeneracy. The second is that the
wheel torque mixes two models. The third is that rating *along* the path from the
output torque holds the output fixed while the moment arm varies — the constant
is the input torque, which is what the shaft delivers at every instant.

And the half of F43 that was left: `Screw::normal_force` took `Flank::Driving`
unconditionally, so a back-driving load was balanced on the driving flank. The
direction is a parameter of the balance now, exactly as it already was of the
efficiency three functions away.

> **Gate, run.** Two laws: a pair that transmits nothing still has its flanks
> pressed, and a back-driving load reaches the wheel undiminished — the second
> asserting both that the flank swap moves the rating by half a percent *and*
> that this is at least five times smaller than the fault it is being told apart
> from, since a bound without the second half is a tolerance rather than a test.

**F61, measured and declined.** The finding said a pair pays eight times over for
starts that all land on the same point. Counted, on the shipped search:

| | walks | evaluations | spent re-finding an answer already found |
|---|---|---|---|
| a pair, both shifts free | 5.0 | 1,480 | **28 %** |
| a pair, shift sum pinned | 3.4 | 113 | 23 % |
| an epicyclic set | 4.8 | 2,718 | **2.4 %** |

So the premise is out by an order — a quarter of the cheap case, and almost none
of the dear one, because a set's walks genuinely land in different places, which
is why it has the multi-start. And the walks that duplicate on a pair are the
ones that **confirm** the summit: the sweep's own best point lands on the lower
ridge and it is an outermost start that reaches the top, so the starts are all
earning their keep even when four of five agree.

**No exact repair exists.** The trajectories converge to within 10–300 ULP of one
another and not to the same bits, so a cache of visited states or a memoised
objective never fires; anything coarser trades a guaranteed answer for a guess
about which walk was going to matter. Declined on the same footing as Q3's closed
form — with the measurement written down so it is not re-derived.


---

## Phase 6 — the front end and the payload

### F17, closed — and the premise was false

**The finding said the payload carries "a simulator no browser path reaches".**
`verify.rs` is 735 lines of rack simulation, it is in the library rather than in
`tests/` so the CLI can sweep it, and no wasm entry point calls it. The
inference was reasonable and it is wrong.

> **Measured.** Gated `verify` out of the wasm build entirely —
> `#[cfg(not(target_arch = "wasm32"))]` — and rebuilt: **1,593,391 →
> 1,593,381 bytes. Ten bytes.** LTO had been stripping it all along. A `pub mod`
> in a `cdylib` is not a root; only the `#[wasm_bindgen]` exports are.

So the finding named a real property of the source and drew a size conclusion
that nothing had measured. *A module being unreachable is a claim about the
linker, and the linker is cheap to ask.*

**Where the payload actually goes.** Attributed by parsing the name section of
an unstripped build and bucketing every function by crate:

| | share of the code section |
|---|---|
| serde derive and traits | **32.2 %** |
| `gear_core` — the mathematics | 20.2 % |
| Rust runtime / std | 18.1 % |
| `toml_edit` — a **format-preserving** TOML parser | **15.7 %** |
| `serde_json` | 5.7 % |
| `gear_wasm` — the boundary | 4.0 % |
| `gear_io` | 3.5 % |

Serialisation is **53.7 %** of the code and the gearing is a fifth of it. The
two largest functions in the whole binary are two monomorphisations of the
derived `Deserialize for train::Stage`, 46.8 KB and 35.6 KB — 7 % of the code
section for one derive instantiated twice, once for JSON and once for TOML.

`toml_edit` is there because `toml` 0.8 is built on it. Nothing in this project
edits TOML in place; it reads documents and writes them. **`toml` 1.x parses and
writes without it.**

| step | payload | gzipped |
|---|---|---|
| shipped before this phase | 1,510,018 | 508,169 |
| `toml` 0.8 → 1 | 1,291,580 | — |
| `opt-level = "z"` for the wasm only | — | — |
| `wasm-opt -Oz` | **1,209,487** | **434,630** |

**−19.8 % on the wire, −14.5 % gzipped, and no answer moved.** The TOML bump is
not merely test-clean: the exported document is **byte-identical**, checked by
diffing `gear-cli trainfile`'s output across both versions, and the corpus pins
that file's length and byte count independently.

**The `opt-level` trade was measured rather than assumed, and the first
measurement was wrong.** A `sed` written to retune a probe profile matched
`^opt-level = 3$` — which `[profile.release]` also carries — so the row recorded
as "opt-level 3" had been built at `"z"`. The correction inverted the
conclusion: `"z"` is not worthless, it is **10.8 %** of the payload. What it
costs is in `Cargo.toml` beside the profile:

| | opt 3 | opt "z" |
|---|---|---|
| payload after `wasm-opt` | 1,355,263 | **1,209,487** |
| `solve_train`, the per-keystroke path | 0.170 ms | 0.273 ms |
| an optimised train | 18.1 ms | 20.5 ms |

The percentages read alarmingly (+61 %) and the absolute numbers decide it:
a quarter of a millisecond against a 16.7 ms frame. Taken — and the two builds
give **bit-identical answers across all 17 entry points**, which is checked
rather than assumed.

### F47, closed — the question asked both ways, and the threshold with it

**What it was.** `Directional::self_locking()` read `backward <= 0.0`. A stage
that cannot be driven **forward** had no flag, no note and no vocabulary: it was
described only by a mesh efficiency reading `0.0 %`, which reads as an
arithmetic accident rather than as *this end cannot turn that one*. The case is
reachable and is already in the corpus — `gear-cli crossed 17 23 90` at its
9°/81° split, at µ = 0.06.

**The threshold was one-sided too**, and that half was not in the finding.
`Screw::self_locking_friction()` returned `cos α_n tan γ` — the backward
threshold — and the panel drew it under a label reading *Self-locks at µ*. So a
forward-locked pair was quoted a number about the direction it was not locked in.

**Both are one construction.** A pair locks in a direction when the tangential
force reaching the member the power **leaves by** falls to zero: forwards that
member is the wheel, backwards it is the worm. Setting the relevant component of
the flank balance to zero gives each in closed form — no bracketing anywhere —
and they are the same expression with the members swapped, which is the standing
rule (`a mechanism has no forward`) showing up as arithmetic.

> **Verified.** Against the friction at which each direction's efficiency
> actually crosses zero, found by bisection, over worm diameters 3–120 mm and
> shaft angles 60°–110°: **exact in both directions at every one**, and negative
> in precisely the cases where no crossing exists. A negative threshold is a
> value — *no friction locks it this way* — and not an absence.

**What moved, and what did not.** Five golden files changed and **every one is
words**; the only new number is the forward threshold itself:

| | was | is |
|---|---|---|
| `crossed 17 23 90`, the 9°/81° row | `0.000 %` | `locked`, with the note under it |
| `worm 1 40 7 90` | `self-locks at mu >= 0.1356` | both: forward `6.5104`, backward `0.1356` |
| `wormstage`, `train mixed`, `train held` | `(self-locking)` | `(cannot be back-driven)` |

**The panel had written the predicate three times in TypeScript** — the crossed
readout, the hula stage and the train total each testing
`efficiency.backward <= 0` inline, each able to say only *self-locking*. The
worm's is gone entirely: `stage.self_locking` and the new
`stage.forward_locking` are Rust's, carry the coefficient and the threshold, and
are drawn beside the efficiency by the same `FIELD_NOTES` convention a shift
raised for undercut already follows. The two that have no note behind them share
one `lockedWays` helper. `ui.train_self_locking` is gone, being what
`ui.train_cannot_be_back_driven` already said.

**The note had to be fired at, and the crate's own gate insisted.**
`the_sweep_fires_most_of_the_catalogue` failed the moment the two keys existed —
*"the sweep does not fire [...], so the placeholder check is vacuous for them"* —
which is F14's lesson enforced rather than remembered. A 3° first-member helix
(an 87° lead angle) locks forwards at µ ≈ 0.049 and back-drives at 45 %
efficiency; four coefficients either side of that fire both new notes.

**And the corpus needed the same treatment**, for the sixth-recorded time: a note
fired only by the string sweep is a note the change detector cannot see. The
`crossed` table prints the locking notes under a locked row, so the case that
motivated the finding is the case that records it. Nothing was contrived.

#### F74 — an assertion that was a rounding coincidence

`self_locking_begins_exactly_where_the_closed_form_says` claimed *the threshold
itself counts as locked*, and passed. Asked of the forward direction it fails:
the backward expression happens to round to `-0.0` at every fixture, while the
forward one lands on `+3e-19` at two of four.

So the old assertion was about **one expression's rounding** and not about
gearing. What is true, and is now what is asserted, is that the efficiency
*vanishes* at the threshold — to within 1e-14 — and that the pair is locked just
above it. Found only by asking the same question the other way round, which is
the whole of what this finding was about.

> **Gates, run.** Reverting `locked()` to answer the backward question for both
> directions fails two tests; making `locking_friction` return the backward
> value for both fails the same two. Each was written against the fault and run
> against it.

### F39, item 2 — mode 3 without the optimiser

**The paradigm's own words:** a centre distance is the true distance and a
clearance says what portion of it is clearance, so any two of {distance,
clearance, shifts} are given and the third follows. Mode 3 is *the distance and
the clearance are given, so the shifts follow*, and the audit called it "the
largest of the four and the only one that moves an answer".

**It held only with the optimiser on.** `SpurStage::shifts_at` returned
`asked.settled` — the undercut floor — before ever looking at the centre
distance. So the plainest thing a designer does, type a housing distance with
nothing asked to move, ran the pair at whatever distance the floor happened to
make and reported the shortfall as clearance.

**The division rule, which is the part the constraint leaves open.** The sum is
fixed in closed form; the division is not, and with no objective to search
against it needs a stated rule. The rule, given by the tool's designer and
implemented as `auto::divide_shift_sum`:

> the even split, projected onto what each member can be cut at.

Written as one projection rather than as three cases — gear 1's admissible
contribution is `[max(lo₁, sum−hi₂), min(hi₁, sum−lo₂)]` and the answer is
`sum/2` clamped into it. An **empty** interval is the honest "no admissible pair
reaches this distance". A ring is the same expression with `sign` doing the work.

What that *does* is the behaviour a designer describes, and it falls out rather
than being coded:

| 9/37, distance opened from nominal | x₁ | x₂ |
|---|---|---|
| at the floor sum | 0.4736 | 0.0000 |
| opening — **the pinion cannot move**, the wheel absorbs | 0.4736 | 0.0265 |
| level, and from here they rise together | 0.5298 | 0.5298 |
| further out | 0.8348 | 0.8348 |

**The two paths agree about the sum and differ only in the division**, which is
the strongest available check that the constraint is being honoured identically
by a search and by a rule that share no code:

| a mm | optimiser off | optimiser on |
|---|---|---|
| 23.4866 | 97.568 % | 97.568 % |
| 23.9532 | 97.634 % | 97.645 % |
| 24.4199 | 97.706 % | 97.706 % |

The optimiser is better or equal everywhere, as it must be, by at most **0.011
points** — so the even division is a *good* answer and not merely a defined one.

**And the old answer was worse than it looked.** At 24.4199 mm the pair ran at a
transverse contact ratio of **0.5711** — below one, so not in continuous contact
at all — because the shifts were the floor's and the distance was 0.43 mm wider
than they reached. It is 1.2505 now.

#### The fallback was discarding a constraint

Found by the new test failing on the path it was not written for. With a
clearance of 0.20 the optimiser's own conditions admit nothing — its minimum
contact ratio bites at the wider operating distance — and it returned `None`,
whereupon `shifts_at` fell back to `asked.settled` and **threw away the centre
distance along with the optimisation**. A pair told to run at 23.6866 mm ran at
23.6433 and said so only through a clearance readout of 0.2433.

The objective and the constraints are not the same kind of thing. Failing to
optimise gives up the objective; it must not give up a constraint. The fallback
is `constrained()` — the same mode-3 placement the non-optimising path uses — and
only where *that* has no answer does the stage fall back to what it would have
built unasked. One statement of "what the constraints alone imply", used twice.

> **Gates, run — four, each against its own fault.** Reverting the
> optimiser-off path to `asked.settled` fails the mode-3 law; removing
> `.or_else(constrained)` fails it too; replacing the clamped even split with the
> interval's low end fails both division tests; and dropping the clamp so the
> even split ignores the floors fails both as well.
>
> The third of those is why the division has tests at all: the first version of
> the mode-3 law asserted only the **sum**, which is division-independent by
> construction, and the "not the even split" mutation passed against it silently.
> *A law that cannot see the rule it is about is not a gate on that rule.*

The division is checked against a **scan** of the same interval that shares no
bound, no midpoint and no clamp with the projection, plus the 9/37 narrative
above stated as three claims that never mention a formula.

#### And it made a comment in the panel false

`relieveSpur` returned early with the optimiser off, on the reading that *"with
the optimiser off the shifts are not being solved for anything, so pinning every
one of them is the fully specified design it always was and nothing is
relieved."* That was true when it was written and is not any more: a given
distance and a given clearance decide the shifts now, so giving both shifts as
well is exactly the contradiction the relief exists for. The relation belongs to
the geometry and never belonged to the optimiser.

Worth recording as a shape rather than as a line: **a guard whose justification
is another module's behaviour goes stale when that module is fixed, and nothing
type-checks a justification.** It was found by reading what the change implied,
not by any check.

**Still open in F39:** the `Auto<f64>` clearance (item 1), a planetary centre
distance (item 3), and the worm's absorber (item 4) — which now has an answer,
recorded below.

Item 1 has a shape now that mode 3 is live. The pinnable set is
`{a, clearance, x₁, x₂}` against one geometric relation, so **three** of the four
may be given and the fourth follows — which is exactly the three modes, counted
rather than enumerated:

| given | what follows |
|---|---|
| clearance, x₁, x₂ | the distance — mode 1 |
| a, x₁, x₂ | the clearance — mode 2 |
| a, clearance, and one shift *or the division rule* | the shifts — mode 3 |

`relieve`'s limit becomes that count. Today `clearance` is a plain `f64` and so
cannot say "derived", which is why it is absent from the relief list and why the
limit reads as `gears.length` rather than as the freedom count it is.

### The toggle model, as specified

Answering F39's item 4 produced a general rule that is wider than the item, and
it is recorded here because it governs the remaining work:

> The under/over-constrained state should be considered a **failure** —
> the toggles automatically resolve it. Toggles should all behave in a similar,
> consistent priority or order regardless of mesh type, when being automatically
> flipped, without keeping track of the order they were touched in.

Concretely, for a parallel pair: both shifts given plus a given distance is
over-constrained, and the *distance* toggle returns to automatic; one shift given
means that one stands and the other absorbs the whole sum; neither given is the
projection above. The `no undercut` toggle is what puts a floor under a member,
and with it off the member keeps absorbing until a geometric limit — a severed
tooth — stops it.

**This was three functions in TypeScript**, one per stage kind (`relieveSpur`,
`relievePlanetary`, `relieveHula` in `TrainPanel.svelte`) — both a rule-1
violation and the per-kind duplication rule 4 exists to catch. **Done**, see
below.

For the **worm**, the specification is: the worm's pitch diameter gains an
automatic toggle that participates in the same relief; and ideally the wheel
gains the helical member's inputs it is currently denied (addendum, dedendum,
profile shift) with an automatic shift that absorbs the distance *in preference
to* the diameter. That wants an interference check — worm tip to flank, flank to
undercut junction — which if closed form is worth having for every mesh kind and
not only this one.

### F39, item 1 — the clearance is an `Auto`, on all four kinds

**Why it was a fault and not a nicety.** A plain `f64` clearance cannot say
"derive this", so the box was read in some states and silently disregarded in
others: with a distance given *and* the shifts given, the reported clearance is
`distance − nominal` and the number typed in the box reaches nothing. That is
exactly the *accept an input and quietly disregard it* the relief exists to
prevent, met on the one input the relief could not see.

**What it changed in the solve**, which is one thing and worth stating plainly:
mode 3 now requires the clearance to be **given**. A distance with an automatic
clearance is a designer asking what gap their shifts leave — mode 2 — and
solving the shifts from the distance would answer a question they did not ask.
Every shipped default is still `Auto::fixed(0.02)`, so **the golden corpus is
unchanged** apart from the exported TOML growing by nine lines, which is the
`Auto` table `centre_distance` already writes.

**The state that could not exist before.** Distance automatic *and* clearance
automatic: a distance is nominal + clearance and a derived clearance is
distance − nominal, so neither has anything to derive from. Answered by giving
`FreedomGroup` a second bound, `automatic_at_most`, and having relief walk the
same order in both directions — too many given turns one automatic, too many
automatic pins one.

| kind | the group that bounds automatics | `automatic_at_most` |
|---|---|---|
| spur, worm, hula | `[clearance, centre distance]` | 1 |
| planetary | `[clearance]` — it has no distance input | **0** |

The planetary's row is the one to read twice. It has no centre-distance input
yet (item 3), so its clearance can never be derived, and **the count says so
without a special case** — when the input arrives the declaration stops saying it
on its own. The hula's crank offset answers to `Freedom::CentreDistance`, which
its own documentation had already argued: *"the same shape every stage's centre
distance has, because it is the same decision."*

**And it turned up a rule the model needed.** A planetary's clearance is the only
input in its group, so if the designer sets it automatic there is nothing else to
pin — and sparing the toggle they just touched leaves the group unsatisfiable.
Relief takes two passes now: the first spares `just`, the second is reached only
when sparing it has no answer. **`just` is a preference; the relation is a law.**
The toggle snapping back is the tool saying *this cannot be derived*, which is
true, and it is the only case that reaches the second pass today.

> **Gates, run.** Running only the given direction fails the new law; declaring
> one more automatic than a kind can afford fails it too. Both mutations were
> silent across the whole suite before it existed.

**The panel lost a rule with it.** The clearance field was a bare box greyed when
the answer was *exactly zero* — "nothing absorbed this", said by a coincidence of
value. It is an ordinary `autoNumber` now, the same control as the distance above
it, on all four kinds.

### F39, item 3 — a planetary centre distance

**It had none.** The common distance was whatever the shifts left, so the one
kind whose geometry is *most* constrained by a housing was the one kind that
could not be told about one.

**And a target makes the layout easier, not harder.** Without one the set has a
single equation — the two distances must agree — solved by Newton on the planet's
shift. With one there are **two**, each of them `mesh::shift_sum_for` in closed
form:

```text
x_s + x_p = shift_sum_for(z_s + z_p,  a)
x_p − x_r = shift_sum_for(z_p − z_r,  a)
```

so one freedom remains, taken as the planet's shift, and the other two are read
off it. **The iteration disappears.** A shift a designer gave fixes that freedom
through whichever mesh it is in; where none is given it follows the same rule a
pair's division does.

| asked | ran at | residual | x_sun / x_planet / x_ring |
|---|---|---|---|
| 20.8200 | 20.8200 | 0.0e0 | −0.3851 / 0.1926 / 0.0000 |
| 21.0200 | 21.0200 | 0.0e0 | 0.0000 / 0.0000 / 0.0000 |
| 21.2200 | 21.2200 | 0.0e0 | 0.4139 / −0.2070 / 0.0000 |

The ring holds at zero because the shipped set **gives** its shift, so it is the
freedom and the other two absorb — the rule working, not a coincidence. Outside
the reachable band the answer is a refusal (`NoContact`, `NoRootSection`), not a
number.

**`PlanetaryResult` gained a `clearance`** on the way, derived like every other
kind's. It was the one result type without one, for the good reason that it had
no distance to derive a gap from; that reason has gone.

#### The declaration had to learn a shape the pair does not have

Writing the freedoms as one flat group of five — `{a, clearance, x_s, x_p, x_r}`
against two relations, three given — **left the stage over-determined**, and the
relief test caught it: two groups were fighting, one turning a toggle automatic
and the next pinning it straight back.

The reason is real geometry. A pair's distance and its two shifts are bound by
one relation and that is all. A set has a relation **among its shifts alone**, so
the two constraints nest: with the distance free, two shifts are a design; with
it given, one. The limit is therefore read from the toggles, which is the same
value-dependence the model already had for a screw stage's sizing.

> **Gates, run.** Ignoring the target fails the distance law. Hard-wiring the
> shift limit to 2 regardless of the distance fails the freedom law — **but not
> the first version of it**, which compared the declaration against itself and
> passed against the wrong number. That is `docs/corrections.md`'s *a check built
> from the thing under test measures nothing*, met while writing a check for a
> declaration. It is asserted against what the set **does** now: give it a
> distance and two shifts, and one of the three cannot survive.

**And the relief test was strengthened by the same finding.** It checked the
group it had just relieved; it checks *every* group now, which is what makes
"these declarations do not fight" a property rather than a hope.

`gear-cli planetstage` prints the given-distance table, so the path is in the
change detector — the seventh time this audit has had to add a case for *an
opt-in the harness never switches on*.

### F80 — angular units, made unambiguous

Raised by the units bug the worm sizing caught, and audited rather than patched:
the bug was a symptom.

**The convention was already right and already there.** Degrees where a designer
states a number — a stage input, a gear input, anything crossing the boundary —
and radians from `plane.rs` inwards, converted once at the edge.
`BasicRack::new(module, pressure_angle_deg, helix_angle_deg)` had been naming the
unit in its *parameters* all along.

**What was wrong was four names that meant both**, and eight angles that stated
no unit at all:

| name | degrees in | radians in |
|---|---|---|
| `shaft_angle` | `train/spur.rs`, `train/worm.rs` | `screw.rs` ×2 |
| `lead_angle`, `wheel_lead_angle` | `WormResult` | `Screw` |
| `wheel_helix_angle` | `WormResult` | `Screw` |
| `pressure_angle` | `params.rs`, `hula.rs`, 4 × `train/` | `strength.rs` |

**The bug's shape is the argument for fixing the names rather than the prose.**
`Screw::least_distance_lead_angle` was written to take radians;
`WormStage::shaft_angle` holds degrees; the call site read perfectly well to its
author and was wrong. The field was **documented** — correctly, one line above
the declaration. Documentation is not what a reader checks; the name is.

So the radian one carries `_rad` wherever a name would otherwise mean both, and
`tools/check_units.py` enforces both halves: every angular field states its unit,
and no name is used in two. **47 fields, all stating one unit.**

> **Gates, run.** Removing `radians` from one doc comment is named and fails;
> stripping the `_rad` from `shaft_angle` reproduces the original collision
> exactly and fails, printing all four sites.

**And it found a second live one, in a fixture.**
`a_near_parallel_crossed_pair_is_rated_on_the_line_its_teeth_have` built its
stages with `shaft_angle: sigma_deg.to_radians()` into a **degrees** field, and
the same mistake again on the helix. So "crossed 0.5°" was a pair at Σ =
**0.0087°** and "crossed 1°" one at 0.0175° — the same point twice, against a
comment saying the two cases exist *because the answer has to be different at the
two ends*. Corrected, and it passes at the angles it names, so the test is now
the one it always claimed to be.

**A newtype was weighed and declined**, on Q3's footing: `Deg(f64)`/`Rad(f64)`
would make the confusion impossible rather than visible, and it touches 47 fields
— most crossing the boundary through `ts-rs` — to remove a fault a suffix and a
check already remove. Recorded in `rationale.md` with what would change the
arithmetic.

### F15 and F16 — measured, and both premises are softer than they read

Both findings describe a *cost*, and both were recorded by reading rather than by
counting. Counted, neither says what it appears to.

#### F15 — "four hand-written stage forms" is not four copies of one

The four forms are 1,042 of the file's 2,900 lines. What they **share** is not
much, and that is the answer rather than an obstacle:

| | labels and fields | unique to it |
|---|---|---|
| spur | 35 | 9 |
| worm | 46 | 21 |
| planetary | 53 | 32 |
| hula | 37 | 13 |

**Sixteen things are common to all four**, and the largest identical run between
any two forms is 21 lines. The "shared header" turns out to be *two* fields —
the module and the pressure angle — after which every kind diverges: spur has a
shaft angle and an additional helix, worm a shaft angle, planetary and hula a
helix angle; the frictions are one pair, one pair, **two** pairs and an array.

The forms are hand-written because the stages genuinely differ, and the file is
**already factored**: about 700 lines of shared snippets (`gearCard` 388,
`screwReadout` 106, `autoNumber` 94, `meshRows` 72, `property` 51,
`boundedNumber` 50) serve the 1,042 lines of per-kind form.

**So a declaration language is declined**, and the counting is why. To replace
forms that share sixteen labels out of thirty-five to fifty-three, the
declaration would have to express sections, per-member cards, conditional
fields, notes and readouts — a configuration dialect harder to read than the
markup it replaced, to remove duplication that is not there. This is Q4's
caveat one level up: *the relative complexity could multiply for little gain.*

**What was actually duplicated**, found by looking for repeated runs rather than
assuming them: three blocks, each written four times — the pressure-angle field,
the static-friction note, and the tolerance pair. All three are the same missing
thing: the row family had `autoNumber` (a value the tool may supply) and
`boundedNumber` (a value with a range), and **not the plain one**. So it was
written out by hand thirty-four times, and had already drifted — some rows
carried a `FieldNote` and some did not, for no reason but which form they were
in.

`numberField` is that third member. **2,900 → 2,765 lines**, and the built
bundle 177.4 → 169.4 kB. Three narrowings had to be carried into `{@const}`
first, which the type-checker named exactly.

> **And a check caught one of eight.** The first pass mapped units through a
> table that knew only millimetres and degrees, so eight rows silently lost
> theirs. `tools/check_strings.py` reported **one** — `ui.train_hours`, the only
> unit that appeared nowhere else — and the other seven were invisible to it
> because their keys are used elsewhere too. A coverage check on a *catalogue*
> cannot see a message that is still used somewhere. Redone from the original
> markup with no table at all: whatever unit cell was there becomes the argument,
> and an unrecognised one stops the script rather than defaulting to blank.

#### F16 — "eleven files" is four hand edits and five recordings

Measured on a real one: `PlanetaryStage::centre_distance`, added for F39's third
item, touched thirteen files. Sorted by what they cost:

| | files | what they are |
|---|---|---|
| substance | 4 | the stage's file, the layout, `train/mod.rs`'s freedom and toggle, the panel's control |
| harness and documents | 3 | a `gear-cli` case so the corpus sees it, `reference.md`, this record |
| **regenerated or recorded** | 5 | two wire types, two golden files, the boundary record — all a `--write` |
| one line | 1 | a JSON fixture |

And **no string files at all**, because it reused a label. The five-catalogue
cost lands only when the *word* is new, which is what five languages costs and
is not reducible.

So the reducible part is the four, and three of those are irreducible too: the
field, its use, and the control have to be written. The remaining one is
`train/mod.rs`'s toggle accessor, which a macro could derive from the freedom
declaration — a saving of two lines per input against a macro nobody can grep.
**Declined, with the counting.**

### F39, item 4 — a worm sized by its housing

**The question the audit left open** was what absorbs a given centre distance in
a stage that has no profile shift. The answer given was the worm's own size, and
that is what is built: `FirstMemberSizing` is an `Auto` now, so the pitch
diameter or the helix angle — two readings of one number — can be the thing the
distance decides.

**It is the one kind whose mode 3 changes the teeth**, not where they sit, which
is why the size *leads* this stage's relief order where every other kind's
distance does. A designer stating a housing and a clearance is asking what worm
fits, so the worm is the answer rather than the input that should give way.

#### The interesting part: there are two answers, or none

A screw pair's centre distance is not monotone in the worm's size. Steepening the
thread shrinks the worm and grows the wheel, so `2a/m_n = z₁/sin γ + z₂/cos β₂`
has a **minimum** — and above it two quite different worms reach the same
centres. Differentiating and solving:

```text
z₂ sin β₂ / cos²β₂ = z₁ cos γ / sin²γ        β₂ = Σ − 90° + γ
tan γ = (z₁/z₂)^⅓                            at a right angle, closed form
```

> **Verified against a scan that shares none of its arithmetic** — one that
> builds pairs and reads their distances rather than differentiating anything —
> at six geometries including two off the right angle, to within two steps of a
> 40,000-point grid. The scan also confirms it is a *minimum* rather than a
> stationary point, which is what makes the two branches a fact rather than a
> claim.

On the shipped 1/40 worm the turn is at **γ = 16.2990°, d₁ = 3.5632 mm**, and
`gear-cli worm` prints it beside the sizing table.

**The branch is chosen by continuity** — the side the designer's own number is
on. It is the only choice under which nudging the target moves the answer instead
of jumping between a thin fast worm and a fat slow one, and it makes the control
behave like every other automatic value: it starts where you left it.

| a mm asked | d₁ solved | lead angle | ran at |
|---|---|---|---|
| 23.7073 | 7.0000 | 8.2132° | 23.7073 |
| 24.7073 | 9.1748 | 6.2573° | 24.7073 |
| 25.7073 | 11.2557 | 5.0971° | 25.7073 |

> **Gates, run — three.** Ignoring the automatic size fails the distance law;
> taking the wrong branch fails it; and `(z₁/z₂)^⅓` written as a square root
> fails the closed-form check against the scan.

**And a units bug was caught by its own test on the first run.**
`WormStage::shaft_angle` is the designer's number in degrees and `Screw`'s is the
mathematics' in radians; the new call passed one where the other was wanted, and
the turning point came back `None` on a right-angle pair. Written down because it
is the kind of thing that survives when a test asserts a shape rather than a
value.

#### What of item 4 is *not* done

The answer given went further than this: *ideally the wheel absorbs the missing
helical inputs — addendum, dedendum, profile shift — and gains an automatic shift
that absorbs the distance **in preference to** the diameter.* That is a larger
change: a `WormMember` would have to become something much closer to a
`StageGear`, and it wants an interference check — worm tip to flank, flank to
undercut junction — which **no mesh kind has today**. If that check is closed
form it belongs beside `Mesh::bottom_clearance` and `train::TipRoom`, serving
every kind, rather than being written for this one. Recorded as the shape of the
work rather than attempted.

### The toggle model, built

The specification above, implemented. `Stage::freedoms` declares which of a
stage's inputs argue with each other and `Stage::relieved` resolves it, both in
`gear-core`; `relieve_stage` carries it across the boundary; the panel says which
toggle was pinned and nothing else.

**What it replaced.** Three functions in `TrainPanel.svelte` — `relieveSpur`,
`relievePlanetary`, `relieveHula` — each restating a relation the core already
enforces. Two faults at once: an engineering rule written outside Rust (rule 1)
and one idea written once per kind (rule 4). It was also **untested**, in either
language, and one of the three carried a justification that had gone stale.

**What the kinds actually declare**, which is the part that had to be right:

| kind | groups | given at most |
|---|---|---|
| spur | `[centre_distance, shift 0, shift 1]` | 2 |
| planetary | `[shift sun, shift planet, shift ring]` | 2 |
| hula | `[shift 0, shift 1]` and `[shift 2, shift 3]` | 1 each |
| worm | none | — |

A worm declaring **nothing** is the model working rather than a hole: it has no
profile shift, so nothing inside it is free to absorb a distance, and that is
exactly why its mode 3 is an open question.

**The limit is `order.len() − 1`, stated once.** Every group here is a single
relation, so exactly one of its inputs is the one the others decide. The first
draft wrote the count per kind and got it wrong by one on the kind with the most
tests — which is the argument for writing it once, made by writing it twice.

> **Gates, run.** Removing the "never relieve what was just pinned" skip fails
> the relief test; relaxing the limit by one so relief fires on a stage already
> inside it fails the same test. Both were written before either fault existed
> and run against them afterwards.

**And the boundary check caught its own first case, unprompted.** Adding the
entry point made `tools/check_wasm.sh` fail with *"entry points the probe never
calls: stage_freedoms"* — written an hour earlier, against exactly this. The
probe over-determines every kind on purpose, so what is recorded is the relief
rather than a stage that needed none.

**Why it went the whole way into Rust.** The first version had the core declare
the groups and the panel do the flipping, which would have left the mechanics
untested. `Freedom` turned out to be the missing piece: it names *which toggle
the designer just pinned*, so the whole rule crosses as data and the panel is
reduced to saying `{ shift: j }`. The call sites read better for it — they now
say which freedom they are about instead of passing a reference to it.

### F72 — nothing had ever executed the payload

Found by asking what would catch it if `wasm-opt` were wrong. The answer was
nothing, and the shape of the gap is worth stating because every individual
check was doing its job:

- `tools/check_bindings.sh` checks the boundary's **shape** — the generated
  TypeScript against the Rust it comes from.
- `cargo nextest run` and the golden corpus check **`gear-core`'s values**,
  natively, through the CLI.
- `nix build .#web` checks that the site **packages**.

The `.wasm` a browser downloads was run by nothing at all. That was tolerable
while the build was a straight compile; it stops being tolerable the moment a
step *rewrites* the module, which is exactly what this phase added.

**`tools/check_wasm.sh`** makes two claims of different kinds, plus a coverage
claim that holds itself up:

1. **A law.** Optimising the payload changes no answer — asserted
   *differentially*, by running every entry point against the module before and
   after `wasm-opt` and requiring identical output. It needs no recorded file
   and cannot go stale.
2. **A change detector**, in the corpus idiom: what the boundary answers is
   recorded, and a diff is a question.
3. **Coverage.** `tools/wasm_probe.mjs` reports which entry points it called,
   and the check greps the crate for `#[wasm_bindgen] pub fn` and fails on any
   the probe does not reach — the rule `gear-cli`'s `COMMANDS` table already
   follows, where the table *is* the dispatch.

Floats are printed to seventeen digits rather than left to `JSON.stringify`,
whose shortest-round-tripping form can render two doubles that differ in the
last bit as the same string.

> **Gates, run — all three against the fault.**
> The law: a module built from perturbed source, stood in as the optimised one,
> fails it and prints the moved stresses. The detector: `K_f`'s `H` at
> 0.331 → 0.3315 moves the record and exits 1. The coverage claim: an added
> `#[wasm_bindgen] pub fn` with no probe call is named and exits 1.

**And no `wasm-opt` flag could break the law.** `--fast-math`,
`--traps-never-happen`, `--ignore-implicit-traps` and `--closed-world` each
leave every answer identical, which is why the gate had to be run by
substituting a genuinely different module. Reassuring about the pass; it says
nothing about the next toolchain, which is what the check is for.

**The corpus caught the first attempt at storing the record.** It was written to
`tools/golden/wasm_boundary.txt`, and `check_golden.sh` — which compares that
directory whole — reported the extra file immediately. Worth more than the
tidiness: `check_golden.sh --write` opens with `rm -f golden/*.txt`, so a record
kept there that the CLI does not produce is one a routine `--write` deletes
silently. It lives at `tools/wasm_boundary.json`.

### F73 — the build was written three times

Adding the `wasm-opt` pass meant adding it to `flake.nix`, to
`web/package.json`'s `build:wasm`, and to the check. Three copies of a build
recipe are three answers to *which module did you measure?*, and the check is
worthless the moment its answer differs from what ships.

`tools/build_wasm.sh` is the one recipe; all three call it. The Nix derivation
needed one line in `srcFor`'s filter to see it, since `filterCargoSources` keeps
only what Cargo needs — the same accommodation the JGMA CSV already has.

The two `--enable-` features are not tuning. They are what rustc emits for this
target and what `wasm-opt` cannot infer, nothing having written a
`target_features` section; **without them the pass fails**, which is the failure
mode to want. A toolchain bump that adds a third makes the build say so rather
than shipping a module optimised under the wrong assumptions.

---

## Phase 7 — F79, one interference check for every mesh

**The finding as scoped:** no mesh kind has a tip-to-flank interference check,
and one would serve every kind. The measurement made it sharper than that: an
**internal** pair had it twice over, under two classical names, and an
**external** pair had it not at all.

### The two internal checks were always one question

`ring::mesh_with` reported *trochoid interference* — the pinion's tip reaching
into the ring's fillet — and *involute interference* — the ring's tip reaching
below where the pinion's flank ends. Both are: **does the mate's tip contact me
outside my usable flank?**, asked of each member in turn. And `mesh.rs` already
held the relation that answers it for either arrangement, signed:

```text
ρ₁ = r_b1 tan α_w + ξ            ρ₂ = r_b2 tan α_w − ξ
```

> **Checked algebraically first, then asserted.** The signed form reproduces both
> of `ring::mesh_with`'s readings exactly — `ρ_ring = a_w sin α_w + ρ_pinion` and
> its inverse — and a test holds them to the bit over ring and pinion counts and
> shifts. The ring's own two closures are **gone**: it calls the general one.

`mesh::conjugate_radius` is that relation, `Mesh::flank_interference` is the
condition, and `FlankEnds` is the seam that lets a rack-cut tooth and a
shaper-cut ring each say where their own flank runs — the `ToothOutline` idiom,
and necessary because **a ring's tip is the lower end of its flank**. The
comparison flips with `MeshKind::sign` rather than with a branch.

### What it found: the optimiser was recommending gears that foul

An external mesh had never been asked, and the shift search is exactly what walks
into it — loss falls with the length of the path, and the longest admissible path
ends where the flank does.

| | fixture pairs whose chosen shifts interfere |
|---|---|
| before | **2 of 14** — 9/37 and 9/20, both nine-tooth pinions |
| after | 0 of 14 |

On 9/37 the recommendation was `Σx = 1.6697` for **97.706 %**, and at those
shifts the wheel's tip contacts the pinion at 4.3053 mm where its flank does not
begin until 4.3554. The honest answer is `Σx = 1.4078` for **97.678 %** —
**0.028 of a point** is what it costs. The 11/18 epicyclic set moved likewise, by
0.023.

The condition also reproduces the classical results without being told them: a
17/43 pair clears by 0.008 mm and a 14/14 pair fouls by 0.0005 mm, so the
seventeen-tooth limit for 20° full depth falls out rather than being encoded.

### And it costs the search something, which is recorded rather than hidden

A refused region is a **wall**, and a constrained optimum sits on it. A pattern
walk in the raw shifts resolves a wall to its own step:

| | before | after |
|---|---|---|
| pairs, worst of 14 | 4.1e-7 | **13 of 14 at 1e-9 or better**; 9/20 at 3.3e-6 |
| sets, worst of 30 | 1.5e-6 | 28 at 1e-6 or better; **11/18 at 1.9e-4** |

Thirteen pairs got an order *tighter* — eliminating an infeasible region takes a
ridge out of several surfaces — and the outliers are exactly the members small
enough for interference to be the binding constraint. It is F50's diagnosis met
on a new constraint, the canary is pinned so it can only shrink, and the remedy
is the one F50 names: search the coordinate the constraint is flat in.

### The fault this nearly shipped, and what caught it

`TipRoom::clear()` meant *all three* conditions. Moving two of them onto
`MeshReport` left it meaning one — and every caller that had been asking the
whole question silently began asking a third of it. One of those callers was
`gear-cli hulaband`'s own admissibility filter.

**The golden corpus caught it, and it read like an improvement.** Four rows moved
and every one moved *up*: `d = 9` went from 79.49 % to 95.02 % of stage
efficiency. It was about to be written into `reference.md` as a better answer.
It was a filter that had stopped filtering.

Two lessons, and the second is the one worth keeping:

1. A predicate that is narrowed must be **renamed**. It is `TipRoom::tips_clear`
   now, which cannot be read as the whole question, and `MeshReport::teeth_clear`
   is the whole question.
2. **A change detector's diff is a question even when the answer looks good.** A
   number moving the way you hoped is the easiest kind to explain away, and this
   one had a ready explanation — *the constraint changed which ridge the search
   lands on* — that was entirely wrong. With the filter restored the table is
   byte-identical to the original.

> **Gates, run — three, and the third took two attempts.** Dropping the sign from
> the general relation fails; not flipping the comparison for a ring fails; and
> removing the search's refusal **failed nothing** until a test was written that
> asserts the optimiser's *recommendation* does not interfere. The tests that had
> broken when the refusal was added were the convergence and idempotency ones,
> and loosening them to accept the wall had left them unable to discriminate it.

### F55 — a distance no shifts reach, and the pair that cannot be assembled

**The finding as recorded:** a centre distance no admissible shifts can reach is
answered rather than refused. Probed, it is two silent faults rather than one,
and the second is the worse.

| 9/37 asked to run at | ran at | clearance | said |
|---|---|---|---|
| 20.00 mm | — | — | refused, `CentreDistanceTooSmall` |
| 22.00 mm | — | — | refused, `NoRootSection` |
| **23.00 mm** | 23.00 | **−0.4433** | **nothing** |
| 25.00 mm | 25.00 | +1.5567 | only that the contact ratio is below one |
| 26.00 mm | — | — | refused, `NoContact` |

The middle row is the one to read twice. A **negative clearance** is a pair whose
teeth overlap at rest: it cannot be assembled, every figure taken at that distance
describes nothing, and the tool reported an efficiency for it without comment.

**Both are notes, not refusals**, on rule 5's reading: the gears are cuttable and
it is the *assembly* that is impossible, so the designer is owed the number.
`train::distance_notes` is the one place, and it is a free function in
`train/mod.rs` rather than in any kind's file for the reason `TipRoom` is —
**every kind with a centre distance can reach these**, and a rule one kind asks
is a rule the others forget. All four raise them, the hula stage through its
crank offset, which is what that kind calls a centre distance.

Neither is reported by the caller. `distance_notes` takes the *target* the shifts
were given — the distance less the clearance, `None` where the stage was not in
mode 3 — and derives whether it was reached, so the finding cannot disagree with
the numbers beside it.

**And the harness had been printing three of them.** `gear-cli shifts 9 37` sweeps
given distances from the reference distance upwards, and its first three rows are
below what the shifted pair can reach — so they were **unassemblable pairs
reported with efficiencies** of 96.979 %, 97.277 % and 97.518 %. They have said
so since, in the corpus, which is where a fault like that is caught by something
other than reading. 17/43 has two such rows as well, missing by 0.0057 mm on the
pinion's much smaller floor.

> **Gates, run — three.** Silencing the not-reached note fails; silencing the
> negative-clearance one fails; and **firing the not-reached note
> unconditionally** fails, which is the half that matters — a note that fires on
> every stage with a distance would pass the first gate and be worthless.

### F51 and F58 — the hula stage's search, asked and diagnosed

**F51.** This was the one search in the crate that could not be asked for an
effort, so *"the shifts it chooses are converged"* was a claim nothing could
raise. `solve_hula_stage_at` takes a `Search` now, following the `_with` pattern
already beside it — **no lifting onto `HulaStage` was needed**, which the finding
had assumed and which would have been a 798-line refactor for an argument.

The effort reaches both halves of what this stage does. `Search` governs each
mesh's own one-dimensional search; `Search::effort` — the `k` a caller passed
`refined`, recovered from `starts` — scales the **outer** loop that solves the
crank, chooses the splits at it and goes round again. `SETTLED` is gone with it:
it was `1e-3` written out, which is what `Search::SHIPPED.resolution` is, so
refining the search now refines the stopping distance as it always should have.

**Measured, it converges.** Four of six fixtures are bit-identical at nine times
the work and the worst is **6.5e-7** — the same order the parallel pair reaches.

> **Gate, run, and its scope stated.** Holding the outer loop while refining the
> inner one fails the gate, so the inner effort is live. Doing the reverse
> changes **no answer at all**, which is not a hole: the number ledger already
> records that this loop settles well inside its cap. An outer loop that never
> binds cannot be gated by refining it, and the test says so rather than implying
> otherwise.

**F58, and it is `holds`.** The observation was that the optimiser moves no
answer over a band of tooth differences. Reproduced at a fixed 324:1 reduction,
turning it on moves the answer for `d = 2..5` and by **nothing at all** at
`d = 1` and `d ≥ 6` — and the two ends are not the same thing:

| | why nothing moves |
|---|---|
| `d ≥ 6` | **the optimum *is* the floor.** The searchable interval begins at the pinion's undercut shift and the efficiency falls monotonically across it — 0.99990 at the floor to 0.99979 at the top — so the best split is the least one, which is where the stage sits unasked. The search runs and agrees |
| `d = 1` | **nothing in the interval is admissible.** Every split is refused by the mesh, so there is no candidate at all. At a one-tooth difference the pair opens to about 45° to clear itself and sits at `ε ≈ 1.02`; there is no room in it |

Neither is a broken search, so F58 closes as `holds` — and the diagnosis took the
instrument F51 built, which is why they were done together.

**What it left is F82**, and that was the part worth acting on: the two ends were
**indistinguishable to a reader**. Closed below.

### F82 — an optimiser that found nothing, and one that agreed

Raised by F58's diagnosis and closed in the same phase. Turning *optimise for
efficiency* on and seeing no shift move means one of **two opposite things**:

- the search ran and **agreed** — the optimum is on the floor the stage already
  sits at, which is the ordinary answer wherever loss falls toward the shortest
  admissible path; or
- the search ran and found **nothing admissible at all**, so there was no answer
  to choose and the stage kept what it had.

The first is the tool working. The second is a design with no room in it. **The
shifts are identical in both**, and every kind fell back in the same silence.

`Searched` names the three states — `NotAsked`, `Chose`, `FoundNothing` — and
`Searched::note` is the only place the wording lives, so no kind has to remember
which of the three is worth saying. All three choosers report it:

| kind | how it can find nothing |
|---|---|
| spur | `shifts_for_efficiency` returns nothing; the constraints still decide, the objective does not |
| planetary | no member has an interval, or the search comes back empty |
| hula | **per mesh**, and the stage says so only when *no* mesh chose — some choosing means the stage optimised, whatever the others did |

The last row is a correction made while writing it: the first version set the
state on any mesh failing, so `d = 3` — where the optimiser moves the answer by
7.1e-4 — reported *nothing admissible*. Caught by putting the three rows in the
harness and reading them.

> **Gates, run — three.** Silencing the note fails; firing it whenever the
> optimiser *ran* fails, which is the half that matters — a note that called
> agreement a failure would pass the first gate and be worse than silence; and
> not recording a refused mesh fails.

`gear-cli shifts epicyclic` prints the three hula rows beside the epicyclic sets,
which is what puts the path in the change detector. It is the **eighth** time
this audit has had to add a case for *an opt-in the harness never switches on*.

### F7, F19 and F21 — the tables gated, and the prose read by class

The four hula tables in `reference.md` — the sixteen-arrangement study, the
addendum sweep, and the two `z = 36` studies at `d = 2..5` — had no command
because no command prints them: each is a study built from several solves. One
test now prints all four, `train::hula`'s
`the_four_hula_studies_are_the_ones_this_code_prints`, and the tables carry a
`figures-by-test` pointer to it. The checker resolves the name, so a renamed
test is a failed check rather than a dangling comment.

**Two of the four had drifted.** Tables 3 and 4 were written against the search
as it stood before F50, F52, F53 and F54 fixed it — a search that quoted a module
ratio the code never looked for. The sixteen-arrangement and addendum tables
held. Regenerating the two moved every efficiency by a few tenths and the shift
divisions by more; the reading did not change, and the paragraph under them that
says the sum is pinned and the division is worth thirteen points at `d = 3`
still says it, against the new figures rather than the old.

> **Gate, run.** A misspelt test name in the tag fails the checker; a table
> figure moved by one unit in its last place fails the test.

**The prose, and why it is not tagged.** With the tables gated, 57 paragraphs
across the four documents still carry a figure at two or more decimals that
nothing generates. `check_figures.py`'s own docstring declines to tag them, and
the reason stands: a figure in a sentence is as often a residual, a citation, or
arithmetic on a gated table as it is live output. Read by class:

| class | count | what holds it |
|---|---|---|
| verification residuals — `4e-16`, `3e-16`, `7.5e-14`, `1e-12` | 7 | the test that produced the residual, which would fail before the sentence became false |
| a measurement recorded once — a sweep, a threshold, a bias with its sign, a canary's history | 27 | nothing; these are history, and each says when it was taken |
| constants cited from a standard or a paper — ISO 6336-3's factors, Table B.1, Hamrock–Dowson | 5 | the citation |
| arithmetic on a block gated above — "27 %", "thirteen points", "within 0.03", "+0.47 at 9 teeth" | 7 | the block's gate, read through by hand; every one checked here against the block, all consistent after the two corrections below |
| illustration — a rule stated with a number in it, or a study's inputs | 11 | nothing, and nothing needs to |

Fifty-seven, counted one by one rather than estimated — this file has already
recorded what a ratio quoted from a glob is worth.

The one class that *is* live output — **a sentence attributing a figure to a
harness command** — was swept by grep rather than by reading, and after this
phase it is empty. It was not empty before:

**`README.md` and `state.md` said the harness prints ε = 1.777921670.** It
prints 1.758113579, and has since `187dc75`, four commits before this audit
began. That commit moved the crossed stage's path to the operating centre — the
zero-backlash distance plus the default clearance of 0.02 mm — which is right,
and on this pair costs 0.02 of contact ratio. `tools/crossed_path.py` builds the
nominal pair, so the two figures the sentence set beside each other were no
longer the same question. The agreement itself is intact: at the nominal centre
with tips at `r + m_n` the construction gives 1.777921670 against the surfaces'
1.777921669562, and **nothing had ever asserted it** — the README's "4.4e-10"
was a figure held in nobody's test. `screw::tests::the_construction_reproduces_the_surfaces_derivation`
holds it now, both documents point at it, and the harness's figure sits beside
it under its own `figures:` tag with the reason the two differ.

> **Gate, run.** Asking the path at the operating centre instead of the nominal
> one fails the test with the harness's figure in the message.

Two smaller corrections on the way, both prose reading a table that had moved:
`reference.md:608` quoted the shift optimiser's gains on 17/43 and 13/61 from
before F79's interference wall — six to fourteen hundredths, now tagged to the
command that prints them; and the paragraph under `hulaband 18` said the rows
through `d = 5` sit at `ε ≈ 1.01`, where `d = 5` sits at 1.09 and the table it
reads does not have a `d = 5` row. Both were arithmetic on a gated table, done
once and not redone when the table was.

**What this says about the count.** Fourteen paragraphs were read closely
before the class sweep; three were wrong. That rate is not an argument for
tagging the other 43 — two of the three were arithmetic on a gated block and the
third was the one live-output sentence, and both classes are now covered by a
means other than a tag: the seven arithmetic paragraphs are checked above, and
the live-output class is swept by a grep that finds nothing. It is an argument for the class sweep being run again by whoever
next changes a table: the checker names the block, and the paragraph under it
is the next thing to read.

---

## Phase 8 — F83, the worm is a pair

The direction was given in one sentence — *always aiming for the ideal where
all stages are fundamentally the same* — and the answer to the one question it
raised was that a **kind is a layer, not a model**: the minimum construct to
pre-assemble and constrain the primitives, carrying the words a designer uses
and the choice of which inputs to put in front of them, over one primitive
underneath. The other answer was that anything nearly the same thing should
become the same thing, results included.

### What was actually two things, and what was not

`WormStage` differed from a crossed `SpurStage` in one input — the first
member sized by pitch diameter rather than helix angle — and an axial float.
Every other field was the same field under a second name. What it *lacked* was
everything `StageGear` has: profile shift, addendum, dedendum, root round, the
undercut bound. And the screw model had no profile shift at all, so a crossed
gear pair's typed shifts reached its tooth-form report and never its mesh —
its centre distance, its zone, its efficiency were those of the unshifted pair.
Its mode 3 did not exist either.

The model never asked for the division. Both flanks are involute helicoids on
cylinders, which is what a helical gear is; a worm is one with a few starts at
a steep helix. `Tooth::new` at one start and 82° of helix builds a sensible
thread — no undercut, a fillet cap clamped and said — which was the first
thing checked and the reason the rest could proceed.

### The mathematics it took

**A shift enters a crossed mesh as a rack's does.** The line of action's
direction is fixed by the base helices and the shaft angle and cannot turn —
the fact the backlash projection already rested on — so a flank thickened by
`x` is the same helicoid rotated about its axis, displaced along that fixed
normal by `x m_n sin α_n` uniformly, and separating the axes by `Δa` displaces
the flanks by `Δa sin α_n` along the same normal. Hence
`a₀ = a_ref + (x₁ + x₂) m_n`, exact, with no involute function and no
operating pressure angle. The parallel pair is the degeneracy and the two laws
part company at second order, the same step at `Σ = 0` the backlash has.

> **Verified off the tooth generator**, which shares nothing with `screw.rs`:
> the transverse half-thickness angle `ψ_p` moves with the shift, `r_b Δψ cos β_b`
> is the helicoid's displacement along its normal, and the product is
> `x m_n sin α_n` to 1e-12 at five helix angles including a worm's 82°.

A consequence worth writing down: a shifted pair's contact is **off the common
perpendicular** even at its own zero-backlash distance. With `n̂` fixed, contact
on the perpendicular at operating radii `r_wi` would need `cos β_bi sin α_wt,i`
equal for both members, which forces `α_wt,i = α_t,i` and `r_wi = r_i` — the
reference radii and nowhere else. So the branch of the line of action is
settled at the *reference* distance (where the pitch point is on it), and the
face widths are sized for where the contact actually is, which the code already
did for a centre-distance error and now does for a shift by the same road.

**The zone was read on both sides of each tangency point.** Past its tangency
point a member has no involute — that is inside its base cylinder — so a mate
whose reach crosses it is fouling, not in a longer zone. The zone runs one way
from each tangency point now, and `CrossedPath::flank_interference` is F79's
relation asked along the line: `mesh::conjugate_radius` with the tangency span
in place of `a_w sin α_w`.

> **Gates, run — three.** The old symmetric reading fails the law that the
> zone stays between the tangency points; ignoring the junction radius fails
> the parallel-limit check; and that check meets `Mesh::flank_interference` a
> hundredth of a degree off parallel on a grid with a tall addendum against a
> small pinion, which has fouling cases in it — asserted, since a grid that
> never fouls would be a gate that cannot fail.

### The primitive, and the kind over it

`PairStage` is `SpurStage`'s fields plus `sizing: Auto<FirstMemberSizing>` —
three readings of one number now, `AdditionalHelix`, `HelixAngle`,
`PitchDiameter` — and `axial_clearance`, which every helical gear has and a
spur kind leaves at zero unseen. `Stage::Spur` and `Stage::Worm` both carry it;
the tag is the kind. In the core the kind decides exactly one thing, the face
width a worm and its wheel take where no rating sizes one (the DIN and BS
proportions, which describe a worm carrying an enveloping wheel and are offered
nowhere else). Everything else a kind is — *starts*, *worm*, *wheel*, which
fields a panel shows — is the front end's to read off the tag.

`PairResult` replaces both result types. Its `mesh` is `PairMesh::Line`
(`MeshReport`, as every parallel mesh) or `PairMesh::Point` (`CrossedMesh`:
two efficiencies, the locking thresholds, the sliding, the patch, the zone, the
lead angles), and the accessors every mesh has — efficiency, backlash, flank
interference, coprime — are read off it without asking which. Both members are
`GearResult`s; the two fields a worm's readout wanted and no gear had —
pitch diameter and helix angle, solved outputs where the sizing is automatic —
are on every gear now, with the proportion's recommendation beside the face
width where one applies.

**One relation for every pair**: `{a, clearance, x₁, x₂, size}`, four may be
given. The order is the spur's with the size last, because a shift moves the
teeth where a size changes them — which is also the solve's preference when
both are free to absorb: the shifts do wherever one is automatic, and the size
only when both are pinned. The worm preset sets DIN's convention as *inputs*
rather than building it in: the worm's shift pinned at zero (it is the tool),
the wheel's free, so a given distance moves the wheel's shift; pin the wheel's
too and the worm's diameter absorbs it, on the branch the designer's number is
on as before. A helical pair cut to fit a standard centre distance is the same
request on parallel shafts, one branch, and it works there now too.

> **Gate, run.** `the_declared_limit_is_the_freedom_the_stage_actually_has`
> gives four and requires every one honoured — with both shifts pinned the
> helix reaches the distance — then gives five and requires that it cannot be.

**The optimiser stays a parallel-axis search.** Its trial mesh, its objective
and its five refusals are the line-contact model's; a crossed pair asked to
optimise takes what the constraints imply and says so
(`stage.optimiser_not_for_crossed`, fired by the string sweep and offered by no
panel where it does not reach). It is not offered on a crossed pair rather than
silently ignored there — the choice the front end makes by kind.

### What the corpus said

Every recorded number is byte-identical: the wormstage canary, the crossed
table, both trains, the mixed train. Two files changed, both additions — the
train file is larger because a worm stage now carries the full member inputs,
and `gear-cli worm` gained the table it could not print before, the wheel's
shift absorbing a distance:

```text
  a mm given, the wheel's shift absorbs it
    23.7073  x2  +0.0000   d1   7.0000 mm   ran at   23.7073
    24.2073  x2  +0.5000   d1   7.0000 mm   ran at   24.2073
    24.7073  x2  +1.0000   d1   7.0000 mm   ran at   24.7073
  a mm given, both shifts pinned, worm sized to reach it
    23.7073  d1   7.0000 mm   lead angle  8.2132 deg   ran at   23.7073
```

The wasm boundary's answers are likewise identical in every number and differ
only in shape, checked field by field against the old record before it was
rewritten. That the model moved nothing is the point of the change: a stage
type was deleted and no answer noticed.

### The front end, and one bug it was carrying

One form for both kinds, the kind deciding the words and the exposure: a worm
shows *starts*, a *length* for its worm, the axial float, and the proportions
beside its faces; a spur shows load sharing and the optimiser only with its
shafts parallel. The sizing is a select over the three readings, seeded from
the geometry on a switch so the pair does not jump. Every member card carries
every member input now — a worm wheel's shift and addendum included, which is
what the change was for — and every card has the shared readout, so the
`extra` block that existed because a crossed member had no `GearResult` is
gone.

**The relief list was short.** `autosOf`, which lines a stage up against its
relieved copy to copy the toggles back, listed the distance and the shifts and
not the clearance or the sizing — so a relief the core decided on either never
reached the panel. Every toggle relief can turn is in it now, by the flag
rather than the value's type.

### Mini-audit of the run above, which was interrupted

Read back cold, against the checks and by grep:

- **Stale words, four.** Two doc comments and the units checker's preamble
  still named `solve_worm_stage`, `solve_crossed_stage` and `WormStage`; a
  hula doc pointer named `PairStage::clearance_taken`, a method deleted in
  Phase 5. Fixed.
- **Dead API, three.** `Stage::preset`, `StageKind` and `as_pair_mut` were
  written for a front end that builds its presets through `defaults()` and
  never called. Deleted, with the generated `StageKind.ts`.
- **The parallel counterpart ran the optimiser.** The crossed solve builds the
  same teeth at `Σ = 0` for comparison, cloning the stage — optimiser toggle
  included, so a crossed pair asked to optimise would have run the search on
  its counterpart. The counterpart is a comparison and takes
  `Optimisation::default()` now.
- **Two accessors nobody read** — `PairMesh::coprime` and `::contact_ratio` —
  which the second half of this phase removed with the enum they were on.
- Everything the corpus, the boundary record and the suite hold, held.

### One mesh report — F84

The first half kept `MeshReport` for a line contact and `CrossedMesh` for a
point, with `PairMesh` choosing, on the reading that the physics differs. The
reading was too strong: the physics is **one model with the shaft angle as a
parameter** — one Hertz answer of which the line is the degenerate value, one
friction balance the parallel integral is the limit of, one backlash
projection, one interference relation — and each of those had already been
held at the limit by a test. A designer turning a shaft angle from zero should
see the same rows with the numbers moving, not a readout changing shape.

So every mesh of every kind reports in one `MeshReport` now: the shared fields
— coprime, the count of pairs in contact, efficiency, the locking thresholds,
the sliding at the pitch point, one `ContactPatch`, backlash, interference,
tips — and what only one contact has in `LineContact` (the transverse
decomposition and the operating angle) or `PointContact` (the zone as the
faces leave it, the parallel counterpart). The three kinds that build a line
contact fill it through one `line_mesh_report`, so the degenerate values —
sliding at zero, locking at *never*, the line's patch — are written once. The
front end's two readouts are one `meshRows`; the CLI's two printers read one
type; `StageResult::meshes()` walks a worm's mesh as it walks every other.

**Where the two contacts meet was then measured**, which nothing had done —
the earlier gates each held one *field* at the limit and none had compared the
reports. On the 17/43 pair a hundredth of a degree off parallel, the contact
centred and the face wide enough that the line governs, the pitch-point
pressure meets to a part in 10⁵ at no friction; the seams are two:

- **1.5 % at the pitch point with friction**, and it is the flank load
  convention: the crossed balance presses with `μ F_n` along a sliding
  direction that stays finite as the sliding speed vanishes, the line rating
  presses with the transverse projection alone, as ISO does. Two conventions,
  each standard where it lives; recorded, not closed.
- **5 % at the worst point**, because *one pair carries everything* is a
  different point on each: a transverse base pitch in from the path's ends on
  a line, a normal base pitch in along the line of action on a point — the
  zones agree to a micron and the pitches differ by `cos² β_b`. A different
  measure, like the contact ratio, and not compared as if it were one.

The measurement found a fault on the way. `hertz::peak_pressure` is the larger
of the ellipse and the line the teeth have, and the crossed patch reported the
*ellipse's* minor axis under whichever pressure won — so a near-parallel pair
rated as a line contact reported a patch a quarter as wide as the line it was
rated on. The patch is the governing model's now, with the line's half-width
`2 ρ p / E*` closed form from what the rating has in hand, and the same closed
form gives every line contact its width. The corpus did not move: the worm
canary's ellipse governs, and no recorded case printed a near-parallel patch.

**And one number was declined.** A line contact's loss is exactly linear in
`μ`, so its locking threshold has a closed form, `μ / (1 − η)` — about 5 on
the shipped pair — and the first draft reported it. The friction balance a
hundredth of a degree off parallel puts the threshold at half that and
asymmetric; a first-order model extrapolated to `μ ≈ 5` describes nothing.
`locking_friction` is *never* on every line contact, and the field says why.

> **Gates, run.** The limit test asserts the seams are *there* as well as
> bounded — a version that closed the single-pair gap would be asserting an
> agreement the models do not have — and the corpus was what caught a
> curvature printed where a radius belonged, on the first run.

### The line count, before and after

Counted from the tree at each commit — Rust production code and its comments
with the test modules split off, the front end without the generated wire,
and the documents:

| | `4daa05a` before | `1d49a9d` one primitive | now, one report |
|---|---|---|---|
| Rust, production code | 15,993 | 16,048 | 16,020 |
| Rust, comment in that code | 12,832 | 12,850 | 12,869 |
| Rust, tests (code and comment) | 25,936 | 26,146 | 26,283 |
| front end, code | 4,745 | 4,588 | 4,545 |
| documents | 7,124 | 7,343 | 7,374 |

**The core did not shrink, and it should be said plainly.** Deleting the worm
type removed 1,105 lines of stage code and the pair and crossed solves are
966; but `mod.rs` grew by the report types and their builder, and the CLI by
the table it could not print before. Rust production is 27 lines longer than
before the phase, for a worm that has a shift, an addendum, an interference
check and a mode 3, a crossed pair whose shifts reach its mesh, a helical pair
that can be sized to a housing, and one report where there were two. The
front end is 200 lines shorter with two forms and two readouts become one
each. That is the honest shape of the change: not less code, the same code
doing more, once.

### The crossed optimiser — F85

Recorded below as not done and then done, because the reason it was not was
weak: the search is not the parallel mesh's, it is `auto::maximise` over a
box the members' own intervals give, with the mesh asked what a candidate is
worth. `crossed_shifts_for_efficiency` is `shifts_for_efficiency` with the
objective swapped — the friction balance along the line of action, on the
zone the teeth leave — and the five refusals asked of a point contact. The
note that said the optimiser did not reach a crossed pair is gone with its
five strings, and the toggle is offered on crossed shafts.

**It found a fault in the parallel search on the way.** Both searches asked
every member to be *as asked* — no clamp raised — before scoring a candidate,
including a member whose shift the designer had pinned. A pinned member is a
constraint on the search and not a candidate of it, which the undercut floor
had already established for the undercut floor (`ShiftAsked::search_floor`, Phase 6)
and the clamps had not: a worm's thread has its round capped at every shift,
so the pinned worm was never as asked and the search refused every wheel,
silently, as `FoundNothing`. `Cut::Pinned` is judged by nothing now, and its
interval is the number given rather than one it will never sweep. The
parallel search carried the same latent fault and never met it, because an
ordinary gear's round always fits.

On the shipped worm the search **agrees with the floor** — `Searched::Chose`
at the same shifts — since a worm's loss is its lead angle's and a wheel
shift only lengthens the path it slides along; on 17/43 at 5° it gains a
third of a point, which `gear-cli crossed 17 43 5` prints and is the ninth
opt-in this audit has had to switch on in the harness. At the parallel limit
it lands within 0.15 of shift of the parallel search and not on it, for the
seam F84 names: the contact-ratio floor is a normal-line count on a point
contact and a transverse one on a line, so the crossed search may shorten the
path further before the same floor stops it.

> **Gates, run.** Restoring the as-asked judgement on a pinned member fails
> the worm's `Chose`; the crossed answer is held admissible under the crossed
> model, converged to the parallel search's own ceiling, and near the parallel
> answer at the limit.

**Recorded, not fixed: the transverse round at a steep helix.** The tooth
generator's rack round is the coefficient times the *transverse* module; at a
worm's 82° that is seven normal modules, it never fits, and the fillet the
thread gets is the cap's (0.95 of the depth). No reported number reads it — the
crossed rating never touches the fillet, and bending is rated on the virtual
spur with the normal round — but the worm's `clamp.fillet_capped` fires at
every shift for that reason, and `r_j` sits higher than a real hob's round
would put it, which makes the crossed interference verdict conservative by
that much. At an ordinary helix it is a 6 % larger round at 20°. A normal
round's transverse section is an ellipse, and neither circle is it; this
belongs with the helical conventions in `docs/state.md`'s ledger.

### What is recorded as not done

- **The transverse round at a steep helix** — above.
- ~~**A crossed pair's shift optimiser.**~~ Done, F85. The search is the parallel mesh's; a
  crossed objective would be the path-averaged friction balance over the rack
  law's distance, and the interference wall is already there to bound it. Not
  attempted; said, not silent.
- **The document format changed shape**, as its own rule allows: a worm file
  written before this is refused loudly, and `gear-io/src/train.rs` records the
  edit that carries it across.

---

## The protocol

The seven passes, so a later audit runs the same thing rather than reinventing
it. `docs/corrections.md` sorts its faults by recurrence, and that ordering is
this list.

| # | Pass | The question |
|---|---|---|
| 8 | Direction | Which reverse cases are **written** rather than asserted? A mechanism has no forward; the reverse is the same construction with the roles swapped, and where the answers differ that difference is an output. Sweep for a second expression rather than a second evaluation — **and sweep the ratings as well as the reports**, which are two readings of one quantity through different functions. Phase 3b is what the first run of this pass missed. |
| 1 | Baseline | What is true right now, mechanically? |
| 2 | Written twice | Where can two answers differ? |
| 3 | Scope by output | Where does each reported number come from? Scope by the **output**, never by a construction — an audit scoped by a construction reports silence as agreement. |
| 4 | Number ledger | What class is each constant, and what asserts it? |
| 5 | Figure provenance | Which command produces each documented figure? |
| 6 | Turn every axis | Is each control turned in each context it reaches? |
| 7 | Break it | Would the gate fail against the fault? |

**Pass 7 is not optional.** This project has recorded three separate gates that
passed against the exact fault they were written to condemn.

**Pass 8 was added mid-audit**, from a standing rule stated after the plan was
written: *a geartrain has no forward.* The rule is
`docs/rationale.md#direction-is-the-readers-not-the-mechanisms`. **The sweep is
complete**; four sites read, two of them faults:

| Site | Verdict |
|---|---|
| An epicyclic set's member torques | **F35.** Solved the reverse for its efficiency, discarded its torques, reported the forward distribution scaled. Ring 6 % low |
| A hula stage's member torques | **F41.** The same, found by finishing the sweep rather than by a second symptom — a hula stage *is* an epicyclic power flow |
| `back_driving_torques`' upstream walk | **Sound.** It looked asymmetric and is not: the forward walk stores the torque at stage `k`'s input shaft counting every *upstream* loss but not stage `k`'s own, and the backward walk stores it counting every *downstream* loss but not stage `k`'s own. The same convention, read the other way |
| The worm's two per-member expressions | **Sound**, and checked by the same degenerate test the fixes are gated on: at zero friction its two members' torque ratio is the tooth ratio in both directions. Its forward output torque carries the stage's own loss and its backward one does not, which is that convention again |

One question the sweep raised and did not settle — whether a member's
back-driving torque should carry its *own* stage's backward efficiency — is
**Q5**, and it is answered above: yes, because the test is role-swap symmetry
rather than a convention to be chosen. Answering it reopened the sweep as
Phase 3b, and the four rows below are only half of what the pass had to cover.

**The row that read `Sound` and was not.** "The worm's two per-member
expressions" was checked against a degenerate test — at zero friction the two
members' torque ratio is the tooth ratio in both directions — and at zero
friction `η_forward = η_backward = 1`, so the very factor in question is
invisible. *A degenerate case cannot discriminate a factor that is one there.*
Both sides of the self-locking threshold now.

**And the sweep was scoped by the output, but by the wrong output.** It asked
where each *reported* torque comes from and never asked where each *rating* does,
which is a second reading of the same numbers through a different function. Four
of the five faults Phase 3b found are in that second half.
