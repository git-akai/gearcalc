# Audit

A working document, and the only one in this repository that is expected to be
**deleted** rather than maintained. It exists so that an audit spanning several
sessions can be put down and picked up by someone who was not there for the
earlier ones.

It is not one of the four documents. [`README.md`](README.md) describes the
tool; `docs/` describes what it computes, why, what was once wrong, and what is
built. This describes a piece of *work in progress* on all five, and when that
work is finished this file goes and its findings live wherever they belonged:
a correction in `corrections.md`, a decision in `rationale.md`, a state in
`state.md`.

**Scope.** Audit, ablate and refactor the tool in its entirety: the mathematics
for robustness and closed form, the code for duplication and stray branches, the
tests for what they actually discriminate, and the documents for whether they are
true of the code. Plus a second objective, kept separate throughout: make the
project cheaper to work on without spending the prose that makes it auditable.

---

## How to resume

1. Read **Status** below. It names the phase in progress and the next action.
2. Read the **Findings ledger**. Anything marked `open` still stands; anything
   `closed` names what closed it.
3. Run the checks in [`CLAUDE.md`](CLAUDE.md) to confirm the tree is where this
   file says it is. If they disagree, this file is wrong — fix it first.

**Two rules arrived after the plan was written**, and both are now in
`docs/rationale.md` rather than only here, because they govern the tool and not
just this audit:

- **A geartrain has no forward** — the reverse is the same construction with the
  roles swapped, asserted rather than written. It is protocol pass 8, and it
  found two bugs in four sites.
- **A centre distance is the true distance and a clearance is what portion of it
  is clearance** — two relations in three unknowns, so any two of {distance,
  clearance, shifts} are given and the third follows. Fully stated below; one
  part done, the rest scheduled as F39.

Every phase carries a **gate**, and a phase is not done until its gate has been
run *against the fault it is meant to catch*. That is this project's own rule
(`docs/corrections.md`, "A check built from the thing under test measures
nothing") applied to the audit itself. A gate that has been written but not run
against a broken tree is recorded here as `written, not proven`.

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
where it was 3.2e-4. It cost eight times the time on a pair, which is F61.

| Phase | What it does | State |
|---|---|---|
| 0 | Golden corpus, figure provenance, `CLAUDE.md` | **done** — gate proven |
| 1 | Truth-up the documents against the code | **done** — gate proven |
| 2 | The number ledger | **done** — gates proven |
| 3 | Unify what is written twice | **done** |
| 3b | The direction sweep, second half — the ratings | **done** — gate proven |
| 4 | The optimiser | **done** — gates proven; the closed form weighed and declined, with its derivation kept |
| 5 | Consolidate the tests | **in progress** |
| 6 | Front end and payload | not started |

**Baseline, measured at `e5e4939`:** 531 tests green in 26.1 s · 13,690 lines of
production code · 10,346 lines of comment in that code · 9,348 lines of
standalone document · 4 `expect` in production, no `unwrap` · 1.49 MB wasm.

Phases 0 and 1 changed no answer: the golden corpus is byte-identical across
both except where `gear-cli matrix` gained a printed spread, which was the point.
Phases 2 onward are gated on that corpus, which is what makes "this refactor
moved no number" a diff rather than a claim.

**Suite: 559 tests** (was 531). **Golden corpus: 27 cases** (was 22).

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
| F51 | The hula stage's shift search cannot be asked for an effort, so it is the one search with no convergence gate | gap | 4 | open — see Phase 4 |
| F52 | A given centre distance drops the optimiser onto the undercut floor — the tool's own recommended distance, typed back, costs 0.42 points | gap | 4 | **closed** — and logged in `corrections.md` |
| F53 | The division objective is bimodal at the addendum cap and the search takes the lower peak | gap | 4 | **closed** — " |
| F54 | A ring was asked nothing, so the search chose rings its cutter had to alter — 26 of 30 sets | gap | 4 | **closed** — and logged in `corrections.md`; the premise was wrong, see below |
| F55 | A centre distance no admissible shifts can reach is answered rather than refused | gap | 4 | open |
| F58 | The hula stage's shift optimiser moves no answer over a band of tooth differences, on or off | holds? | 4 | open — observed, not yet diagnosed |
| F59 | Three kinds each wrote out what to ask of a mesh, and answered it three ways | gap | 4 | **closed** — `auto::MeshTrial`, and logged in `corrections.md` |
| F60 | A hula pair's tip margin and an internal mesh's interference flags are asked by one kind each | gap | 5 | open — the next candidates for the mesh level |
| F56 | No CLI command drove the optimiser, so its answers were outside the corpus | gap | 4 | **closed** — `gear-cli shifts`, which also closes F19's first row |
| F57 | `check_figures.py` had `check_golden.sh`'s stale-binary fault | drift | 4 | **closed** — and logged in `corrections.md` |
| F2 | The worm stage is outside the shared member vocabulary | gap | 3 | **closed** — option B; a crossed member is a `GearResult`, a worm's is not and says why |
| F3 | `GearResult` assembled three times, one field by two formulas | gap | 3 | **closed** — one `GearResult::of`, and the shared rule is `StageTorques::referred_like` |
| F4 | `StageGear` — a shared input type — lives in `train/spur.rs` | drift | 3 | **closed** — moved, with its `Default`, `AddendumAsked` and serde helpers; `spur.rs` 1017 → 730 lines |
| F5 | No ledger of the numbers that are not model constants | gap | 2 | **closed** |
| F6 | The face-width invariance test ran the one model no stage rates with | gap | 5 | **closed** — every model and a rim, six cases |
| F7 | ~212 documented figures, one gate | gap | 0 | **part closed** — mechanism built; 5 tables still ungated (F19) |
| F8 | The CLI list chosen to be exhaustive is not | drift | 1 | **closed** — the table *is* the dispatch |
| F9 | The Layout table names 7 of 27 modules | drift | 1 | **closed** — the map is `CLAUDE.md`; `state.md` keeps the decisions |
| F10 | `bending-check.html` regenerates by hand | drift | 0 | **closed** — `figures-verbatim`, checked exactly |
| F11 | An orphaned sentence fragment in `reference.md` | drift | 1 | **closed** |
| F12 | The inline tests never had the integration tests' consolidation | gap | 5 | open |
| F13 | Five production modules carry no inline tests, invisibly | holds | 5 | open |
| F14 | The two unfired notes need their evidence re-dated | holds | 5 | open |
| F15 | `TrainPanel.svelte` is 2,848 lines, four hand-written stage forms | gap | 6 | open |
| F16 | One stage input touches eleven files | holds | 6 | open |
| F17 | 1.49 MB wasm carrying a simulator no browser path reaches | drift | 6 | open |
| F18 | `CLAUDE.md` is empty | gap | 0 | **closed** |
| F19 | Five documented tables have no command that reproduces them | gap | 4 | **part closed** — `:360` regenerates from `gear-cli shifts`; four remain |
| F20 | `state.md` derived a figure by hand from rounded output, and it was wrong | drift | 0 | **closed** |
| F21 | The figure checker cannot see figures in prose, only in tables | gap | 5 | open |
| F22 | The tense rule as written forbade 127 sentences it was not aimed at | drift | 1 | **closed** — the rule was narrowed, not the prose |
| F23 | The gear tab and a stage member bounded the same gear differently | gap | 2 | **closed** — and logged in `corrections.md` |
| F24 | The golden corpus covers the CLI, not the wasm boundary | gap | 5 | open |
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
| F39 | The clearance paradigm: `Auto` clearance, mode 3 without the optimiser, a planetary distance | gap | 6 | open — scheduled, see above |
| F40 | The tolerance band was built four times and its direction asserted nowhere | gap | 3 | **closed** |
| F33 | A crossed pair's members said nothing about their own teeth | gap | 3 | **closed** — and logged in `corrections.md` |
| F42 | A locked mesh reported a torque on the shaft it delivers nothing to | gap | 3b | **closed** — and logged in `corrections.md` |
| F43 | A worm was rated at `η_forward` of the load it was holding — 17 % low | gap | 3b | **closed** — " |
| F44 | A back-driven set rated its ring 6.0 % low in bending, 3.0 % in contact | gap | 3b | **closed** — " |
| F45 | ...and the hula stage 41 % and 23 % low, the same fault | gap | 3b | **closed** — " |
| F46 | A zero force was refused, so a stage at rest could not be solved | gap | 3b | **closed** — " |
| F47 | `Directional::self_locking` asks a directional question one way only | gap | 6 | open — see Phase 3b |
| F48 | A screw mesh that transmits nothing reports no flank load | gap | 5 | open — see Phase 3b |
| F49 | `check_golden.sh` recorded the corpus from whatever binary was on disk | drift | 3b | **closed** — and logged in `corrections.md` |
| F50 | The optimiser's convergence claim was half true: a set's search ran one start of six | gap | 4 | **closed** — and logged in `corrections.md` |
| F61 | A pair pays eight times over for starts that all land on the same point | gap | 5 | open — measured, see Phase 4 |
| F62 | The walk took no step at all on a narrow box, so the answer was the sweep's grid | gap | 4 | **closed** — and logged in `corrections.md` |
| F63 | Three documents said the division is solved; it is searched, and the solver has no production caller | drift | 4 | **closed** — and logged in `corrections.md` |
| F64 | `split_residual` is derived at fixed tip radii, which the default tip cap breaks | gap | 4 | **closed** — stated where it lives; the corrected rate is below |
| F65 | Five rating constants could be perturbed with the whole suite silent | gap | 5 | **closed** — measured; four are the corpus's and it is now in the pre-push list |
| F66 | The load-sharing ramp's constants were guarded by a test written in terms of them | gap | 5 | **closed** — pinned as figures |
| F67 | The harness turned three of a gear's eleven controls | gap | 5 | **closed** — `sweep` turns every gear axis, `train toggles` every stage one |
| F68 | `Loading::at_width`'s exponent was hidden by `PROBE` equalling the default face width | gap | 5 | **closed** — and logged in `corrections.md` |
| F69 | The outline's own promise was untested; its `worst_deviation` measured chord length | gap | 5 | **closed** — " |

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

Coverage today: **5 blocks gated by a command, 2 by a test, 2 sections exempt,
5 tables still ungated** (F19).

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
- **F21, open.** The coverage report scans Markdown tables only, so a figure
  quoted in prose is invisible to it. One such block was found and tagged by
  hand; there will be others. Phase 5 should widen it or the "5 ungated" number
  is an undercount.

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

**F6, closed on the way.** The face-width invariance ran on `FormFactorOnly`
alone, the one notch model no stage rates with — it ships through
`gear-cli matrix`, and the stages all use Dolan–Broghamer. Nothing was wrong with
the answer, since none of the three reads a face width, which is *why* the
invariant holds; but a property asserted of one arm of a `match` is asserted of
one arm of a `match`. Six cases now: three models against a rim silent and
biting.

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
