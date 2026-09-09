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
**Phase 1 — truth-up the documents.** Next.

| Phase | What it does | State |
|---|---|---|
| 0 | Golden corpus, figure provenance, `CLAUDE.md` | **done** — gate proven |
| 1 | Truth-up the documents against the code | **next** |
| 2 | The number ledger | not started |
| 3 | Unify what is written twice | not started |
| 4 | The optimiser | not started |
| 5 | Consolidate the tests | not started |
| 6 | Front end and payload | not started |

**Baseline, measured at `e5e4939`:** 531 tests green in 26.1 s · 13,690 lines of
production code · 10,346 lines of comment in that code · 9,348 lines of
standalone document · 4 `expect` in production, no `unwrap` · 1.49 MB wasm.

Phases 0 and 1 change no behaviour. Phases 2 onward are gated on the golden
corpus, which is what makes "this refactor moved no number" a diff rather than a
claim.

---

## The four decisions

Settled at the outset, recorded here so they are not re-litigated.

| | Question | Answer |
|---|---|---|
| **Q1** | Is there a compatibility contract on the wire types, the geartrain TOML, the DXF or the CLI's output? | **None.** Any such change is permitted. |
| **Q2** | Bring `WormMemberResult` inside `GearResult`, or record why a crossed member cannot be one? | **Bring it inside** — on principle, and as a stress test of the claim that a stage kind should be new kinematics and no new rating machinery. |
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
| F1 | The crate has one optimiser and the solve inventory omits it | gap | 1, 4 | open |
| F2 | The worm stage is outside the shared member vocabulary | gap | 3 | open |
| F3 | `GearResult` assembled three times, one field by two formulas | gap | 3 | open |
| F4 | `StageGear` — a shared input type — lives in `train/spur.rs` | drift | 3 | open |
| F5 | No ledger of the numbers that are not model constants | gap | 2 | open |
| F6 | The face-width invariance test runs a model the tool does not ship | gap | 5 | open |
| F7 | ~212 documented figures, one gate | gap | 0 | **part closed** — mechanism built; 5 tables still ungated (F19) |
| F8 | The CLI list chosen to be exhaustive is not | drift | 1 | open |
| F9 | The Layout table names 7 of 27 modules | drift | 1 | open |
| F10 | `bending-check.html` regenerates by hand | drift | 0 | **closed** — `figures-verbatim`, checked exactly |
| F11 | An orphaned sentence fragment in `reference.md` | drift | 1 | open |
| F12 | The inline tests never had the integration tests' consolidation | gap | 5 | open |
| F13 | Five production modules carry no inline tests, invisibly | holds | 5 | open |
| F14 | The two unfired notes need their evidence re-dated | holds | 5 | open |
| F15 | `TrainPanel.svelte` is 2,848 lines, four hand-written stage forms | gap | 6 | open |
| F16 | One stage input touches eleven files | holds | 6 | open |
| F17 | 1.49 MB wasm carrying a simulator no browser path reaches | drift | 6 | open |
| F18 | `CLAUDE.md` is empty | gap | 0 | **closed** |
| F19 | Five documented tables have no command that reproduces them | gap | 1 | open |
| F20 | `state.md` derived a figure by hand from rounded output, and it was wrong | drift | 0 | **closed** |
| F21 | The figure checker cannot see figures in prose, only in tables | gap | 5 | open |

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

## The protocol

The seven passes, so a later audit runs the same thing rather than reinventing
it. `docs/corrections.md` sorts its faults by recurrence, and that ordering is
this list.

| # | Pass | The question |
|---|---|---|
| 1 | Baseline | What is true right now, mechanically? |
| 2 | Written twice | Where can two answers differ? |
| 3 | Scope by output | Where does each reported number come from? Scope by the **output**, never by a construction — an audit scoped by a construction reports silence as agreement. |
| 4 | Number ledger | What class is each constant, and what asserts it? |
| 5 | Figure provenance | Which command produces each documented figure? |
| 6 | Turn every axis | Is each control turned in each context it reaches? |
| 7 | Break it | Would the gate fail against the fault? |

**Pass 7 is not optional.** This project has recorded three separate gates that
passed against the exact fault they were written to condemn.
