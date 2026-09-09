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
**Phase 3 — unify what is written twice.** Done. F2, F3, F4, F32, F33, F34,
F36, F37, F38 and F40 closed, and four of them were bugs.
**Phase 4 — the optimiser.** Next, and it carries F19 and F1's second half.

| Phase | What it does | State |
|---|---|---|
| 0 | Golden corpus, figure provenance, `CLAUDE.md` | **done** — gate proven |
| 1 | Truth-up the documents against the code | **done** — gate proven |
| 2 | The number ledger | **done** — gates proven |
| 3 | Unify what is written twice | **done** |
| 4 | The optimiser | **next** |
| 5 | Consolidate the tests | not started |
| 6 | Front end and payload | not started |

**Baseline, measured at `e5e4939`:** 531 tests green in 26.1 s · 13,690 lines of
production code · 10,346 lines of comment in that code · 9,348 lines of
standalone document · 4 `expect` in production, no `unwrap` · 1.49 MB wasm.

Phases 0 and 1 changed no answer: the golden corpus is byte-identical across
both except where `gear-cli matrix` gained a printed spread, which was the point.
Phases 2 onward are gated on that corpus, which is what makes "this refactor
moved no number" a diff rather than a claim.

**Suite: 547 tests** (was 531).

---

## The four decisions

Settled at the outset, recorded here so they are not re-litigated.

| | Question | Answer |
|---|---|---|
| **Q1** | Is there a compatibility contract on the wire types, the geartrain TOML, the DXF or the CLI's output? | **None.** Any such change is permitted. |
| **Q2** | Bring `WormMemberResult` inside `GearResult`, or record why a crossed member cannot be one? | Answered **bring it inside**; the experiment showed the premise was two propositions with opposite answers, and it was re-answered **B** — `gear: Option<GearResult>`, `Some` for a crossed pair, `None` for a worm. Done. |
| **Q3** | Assert the optimiser's convergence claim, or attempt the closed form? | **Attempt the closed form.** The learnings are worth the effort on their own; assert convergence first regardless, since that step stands alone. |
| **Q4** | Loosen the guard conventions toward true degeneracy limits, or document them as conventions? | **Loosen — cautiously.** With the caveat below, which is a constraint on the work and not a preference. |

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
| F2 | The worm stage is outside the shared member vocabulary | gap | 3 | **closed** — option B; a crossed member is a `GearResult`, a worm's is not and says why |
| F3 | `GearResult` assembled three times, one field by two formulas | gap | 3 | **closed** — one `GearResult::of`, and the shared rule is `StageTorques::referred_like` |
| F4 | `StageGear` — a shared input type — lives in `train/spur.rs` | drift | 3 | **closed** — moved, with its `Default`, `AddendumAsked` and serde helpers; `spur.rs` 1017 → 730 lines |
| F5 | No ledger of the numbers that are not model constants | gap | 2 | **closed** |
| F6 | The face-width invariance test runs a model the tool does not ship | gap | 5 | open |
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
| F19 | Five documented tables have no command that reproduces them | gap | 4 | open — moved to Phase 4, see below |
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
| F31 | No CLI train sets a back-driving load, so the corpus never exercises one | gap | 3 | open |
| F32 | `StageResult` has no kind-independent `members()` | gap | 3 | **closed** |
| F34 | `Widths::contact` was not optional, so "no rating sizes this face" had no way to be said | gap | 3 | **closed** |
| F35 | A set reported back-driving torques from the forward distribution — the ring 6 % low | gap | 3 | **closed** — and logged in `corrections.md` |
| F36 | `SpurResult` re-declared `MeshReport`'s seven fields, and the panel re-drew them | gap | 3 | **closed** |
| F37 | A given crank offset was not the offset the stage ran at | gap | 3 | **closed** — and logged in `corrections.md` |
| F38 | The reported clearance was the input echoed, not the gap run at | gap | 3 | **closed** — and logged in `corrections.md` |
| F39 | The clearance paradigm: `Auto` clearance, mode 3 without the optimiser, a planetary distance | gap | 6 | open — scheduled, see above |
| F40 | The tolerance band was built four times and its direction asserted nowhere | gap | 3 | **closed** |
| F33 | A crossed pair's members said nothing about their own teeth | gap | 3 | **closed** — and logged in `corrections.md` |

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

**`tools/check_golden.sh`** · 22 recorded outputs, 9,213 lines, 144 KB. Every
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

## The protocol

The seven passes, so a later audit runs the same thing rather than reinventing
it. `docs/corrections.md` sorts its faults by recurrence, and that ordering is
this list.

| # | Pass | The question |
|---|---|---|
| 8 | Direction | Which reverse cases are **written** rather than asserted? A mechanism has no forward; the reverse is the same construction with the roles swapped, and where the answers differ that difference is an output. Sweep for a second expression rather than a second evaluation. |
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
written: *a geartrain has no forward.* It found F35 on its first sweep — an
epicyclic set that solved its reverse power flow, used it for the efficiency and
discarded its torques. The rule is now
`docs/rationale.md#direction-is-the-readers-not-the-mechanisms`, and the sweep
is not finished: `back_driving_torques`' upstream walk, the worm's two
per-member expressions and the hula stage's power flow have not been read
against it.
