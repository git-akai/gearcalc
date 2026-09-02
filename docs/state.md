# State

Where the project stands, what to run, and what is left.

**This is the only document allowed to talk about the present**, and saying so is
what lets the others stop hedging. [`reference.md`](reference.md) states what the
tool computes, [`rationale.md`](rationale.md) why each model is the one chosen,
and [`corrections.md`](corrections.md) what was once wrong. None of those should
contain the words "now", "still" or "currently"; this file is where that belongs,
and it is the one file expected to be rewritten rather than amended.

---

## Running it

```bash
nix develop                       # or `direnv allow` once
cargo nextest run                 # the suite
nix flake check                   # what CI runs: build, clippy --deny warnings, fmt, tests
cd web && npm run dev             # the application
```

And the checks that live outside the Rust suite:

```bash
tools/check_bindings.sh           # the generated TypeScript matches the Rust
tools/check_bindings.sh --write   # ...or regenerate it
tools/check_doc_links.py          # every pointer from code into the documents resolves
tools/check_strings.py            # every ui message is used, and every use has a message
cd web && npm run check           # typecheck the front end
```

## Two copies in one browser

Nothing crosses between them but the **language**. Every input lives in its tab
and every output is recomputed from it, so there is no cookie, no service
worker, no IndexedDB, no shared worker and no cache — one `localStorage` key,
`gearcalc.language`, and a `storage` listener so the copy that did not make the
change follows the one that did rather than disagreeing until it is reloaded.
A value it does not recognise resolves to English, so an older copy cannot be
broken by a newer one writing something it has never heard of.

## Where it is published

<https://git-akai.github.io/gearcalc/>, from `main`, by `.github/workflows/ci.yml`.

The deploy is `needs: tests`, so the site cannot move unless every check passed
on that commit. The site is **packaged in the tests job** rather than rebuilt in
the deploy one, so what is published is byte for byte what those checks ran
against; `deploy` is a separate job only because publishing needs permissions
nothing running project code should hold.

Nothing about the build is Pages-specific. `base: "./"` makes every asset
relative and the wasm resolves through `new URL(…, import.meta.url)`, so the
site works at any subpath — and it fetches nothing external, so a static host is
all it ever needs.

```bash
nix build .#web                   # the deployable site, in ./result
cp -rL result public              # what CI hands to the upload; `result` is a
chmod -R u+w public               # symlink into the read-only store
```

## Driving the mathematics without a browser

```bash
cargo run --bin gear-cli -- show 17 0.2            # one gear's derived geometry
cargo run --bin gear-cli -- materials              # the library, with each value's basis
cargo run --bin gear-cli -- strength 17 43 2.0     # a worked mesh, end to end
cargo run --bin gear-cli -- train                  # a two-stage train
cargo run --bin gear-cli -- train mixed            # ...with a worm stage in it
cargo run --bin gear-cli -- trainfile [path]       # a train to TOML and back, answers compared
cargo run --bin gear-cli -- worm 1 40 7 90         # a worm pair, both directions
cargo run --bin gear-cli -- wormstage 1 40 7 2     # a worm stage, end to end
cargo run --bin gear-cli -- crossed 17 23 90       # a crossed pair, swept over the split
cargo run --bin gear-cli -- planetary 17 17 3      # every ring count that can work
cargo run --bin gear-cli -- planetstage 24 18 60 3 # a planetary stage, six modes
cargo run --bin gear-cli -- bending                 # the bending construction, drawn
cargo run --release --bin gear-cli -- verify 100   # the two-sided cutter check
python3 tools/worm_flank_curvature.py              # ZI vs ZN vs ZA, from the surface
python3 tools/crossed_path.py                      # the crossed path, from the surfaces
```

[`bending-check.html`](bending-check.html) is that last command's figures with
the prose that reads them, kept because the bending construction is far easier
to judge by looking than by reading an assertion. It is a **document with
generated figures in it**, not a stored answer: re-run the command and paste the
body back in if the construction ever moves.

The last two share no code with the crate — that is their whole purpose.
`crossed_path.py` builds both flanks as parametric surfaces and reaches the line
of action through differential geometry; the crate reaches it through a
construction in lines and angles. On a 17/23 pair at 45°/45° with shafts at 90°,
they give ε = 1.777921670 and 1.777921669562.

---

## The canaries

Two figures have survived every refactor unchanged, and between them they have
caught more in their areas than the suite has.

| | |
|---|---|
| `gear-cli strength 17 43 2.0` | `σ_F` 69.2 / 63.4 MPa · `σ_H` 692.7 MPa · ρ 1.723 mm · η 98.741 % |
| `gear-cli wormstage 1 40 7 2` | η 61.805 % forward, 0.000 % backward (self-locking) · backlash 0.15512° at the wheel (min 0.11342, max 0.19683), 6.20497° at the worm |

**The worm canary has moved four times, all deliberately**, and the reasons are
worth keeping because each was a model change rather than a fix:

1. *Efficiency*, when the friction balance replaced the pitch-point formula:
   68.691 → 68.430 % forward. The old figures were the same balance sampled at
   the one point on the path where the added term is zero.
2. *Backlash*, when the centre-distance term stopped counting one flank of two.
   The **minimum is unchanged at 0.11342°**, which is the arithmetic confirming
   itself: at `clearance − tol₋ = 0` the centre-distance term vanishes and only
   the axial float is left.
3. *Efficiency again*, when the rating moved to the centre distance the pair runs
   at: 68.430 → 68.369 %. **Backlash did not move**, which is the check that the
   two centre distances stayed in their own lanes.
4. *Backward efficiency to zero*, when static friction arrived. **Forward is
   unchanged**, which is the check that the two coefficients stayed in theirs.
5. *Efficiency again*, when the default sliding coefficient moved from 0.06 to
   0.08: 68.369 → 61.805 % forward. Not a model change — the same arithmetic at
   a different input — and recorded here only because the canary is quoted as a
   figure. **Backlash did not move**, which is again the two staying in their
   lanes: a coefficient of friction is not a geometry.

Load cases moved neither canary. Both are single-load reports, and a stage asked
for one torque answers with the figure it always did — which is the check that
the second case was added rather than substituted for the first.

---

## Layout

| Path | Role |
|---|---|
| `crates/gear-core` | All mathematics. No I/O, no UI, no wasm. `serde` and `ts-rs`, both optional and both about the shape a type takes when it leaves. |
| `gear-core/src/tooth.rs` | `Tooth` — one tooth's form, at one shift, cut by one `Rack`. Not a gear. |
| `gear-core/src/gear.rs` | `Gear` — the assembly, and the only place a gear is drawn. An ordinary gear is `Δx = 0`. |
| `gear-core/src/plane.rs` | The normal and transverse planes, and the two identities that carry an angle between them. One home, because there were nineteen. |
| `crates/gear-io` | File formats: DXF export, the TOML material library and geartrain documents, and the string catalogue. |
| `crates/gear-wasm` | The WebAssembly boundary. JSON in, JSON out. |
| `crates/gear-cli` | Development harness — drive the mathematics without a browser. |
| `web/` | Svelte 5 + TypeScript + Vite front end. |
| `web/src/wire/` | **Generated.** The Rust types that cross the boundary, written down by `ts-rs`. |
| `crates/gear-io/data/strings_en.toml` | **Every word the application shows**, one file per language. |
| `handoff_inbound/` | Prior Python work. **Reference only** — do not build on it. |
| `docs/history/` | The superseded design record, kept for provenance. Nothing points at it. |

---

## What is built

**Parallel-axis gearing.** Involute + trochoid profile, undercut, severed teeth,
validated against a rack simulation from both sides over 1080 cases · primitives
(safeguarded `inv⁻¹`, Brent, bracketed Newton) · mesh, centre distance, exact
backlash, contact path · metrology (span, over-pins, JGMA 116-02 tables) ·
strength (critical section, form factor, bending stress, Hertz, face width,
helical throughout) · efficiency · automatic profile shift and altered addendum.

**Crossed axes.** One model rather than a family: the lead angle exact, the path
of contact from two properties of an involute helicoid, elliptical contact,
sliding as a vector, and one friction balance containing both older efficiency
formulas. A crossed gear pair is a spur stage with an axis angle — three stage
kinds, not four. A crossed pair's face width is automatic from `ε ≥ 1`, a
*geometric* minimum; a worm keeps its published proportions. Both are labelled
with which kind of minimum they are, because they differ by 2.4× and answer
different questions.

**Internal gears.** The ring's flank, its profile shift, a shaper-cut fillet at
the centre distance the shift puts the tool at, the flank/fillet tangency, the
generation limit, two mesh interference conditions, and a bending rating.
Verified by simulating the cut — 2.5–2.7 µm across shifts −0.4 … +0.5.

**Planetary sets.** The planet shift that makes the two centre distances agree,
the ring search, layout checks, Willis kinematics, Pennestrì–Freudenstein
efficiency in all six arrangements, and backlash referred to the output shaft.

**Eccentric gears — not offered until they are asked for.** A gear whose profile
shift varies with angular position, at a genuinely constant transmission ratio.
The mathematics is derived and gated like everything else here; what keeps it
out of the picker is that no part cut from it has been measured, and the residual
it reports is against an ideal rather than against a mechanism anyone has built
([rationale](rationale.md), "An eccentric gear is an ordinary gear with Δx = 0").
The type appears in a gear tab's list only in the developer mode, which is
knocked for on the application's title in the sidebar — ten clicks inside four
seconds, no visible answer — and left by reloading, like every setting here that
is not the language. It is still labelled experimental in its own name once it is
there, so it cannot be read without the word
([rationale](rationale.md#unfinished-work-is-knocked-for-not-switched-on)).

One hob, one setting; the root belongs
to the gear; the commanded centre distance and what a simple crank leaves; and
inspection data — span and over-pins — as the range it takes around the
revolution, verified against a caliper reading off the drawn teeth.

**Trains.** Spur/helical, worm and planetary stages in one train; torque,
backlash and cycle accumulation; efficiency and backlash in **both** drive
directions. Contact is `max(elliptical, line)` on **both** mesh kinds now — a
crossed pair's ellipse lengthens as its shafts come parallel, so the line its
teeth actually provide is what carries the load there, and rating on the ellipse
alone under-stated a near-parallel pair eightfold. Load sharing is a stage input,
off by default, reaching bending alone. Two load cases throughout — a peak against the ultimate allowable
and a cyclic one against fatigue — with the automatic face width sized from any
of the four ratings a gear chooses. **Neither contact rating is enabled by
default**: both are computed and shown, but a fresh stage is sized from bending
alone, so its face width will not satisfy contact until a designer says which
rating should decide it — the figures are on screen, and the minimum face width
each rating asks for is beside them. The two gears of a mesh are rated at different points on the path
— each where its own dedendum is loaded alone — so they carry different contact
stresses; the shared pitch-point figure is reported at the mesh. An automatic
width answers to the mesh, not to one gear. A back-driving load applied at the output
finds the stage that reacts it, or reports that nothing does. A reversing
intermittent drive rounds its cycles within one actuation and splits contact
between the flanks. **Reversed bending is a train-wide switch, off by default**:
a planet's root is loaded both ways whatever the drive does, a reversing drive
loads every root both ways, and each gear that one reaches says so beside its own
numbers — corrected against the reduced allowable only where the switch asks for
it. A notch parameter outside the band the `Y_S` fit is stated for says so on
the gear too.

**Languages.** English, German, Portuguese, Simplified Chinese and Traditional
Chinese, all compiled in, picked from under the title in the sidebar. **The four
translations are machine-produced and have not been reviewed by a native
speaker** — the terminology follows the standard gear vocabulary of each
language (DIN 3960, GB/T 3374, CNS, and the usual Portuguese usage with Acordo
Ortográfico spelling) and the placeholders are gated, but the prose deserves a
proof-read before anyone leans on it. Correcting one changes no calculation:
`crates/gear-io/data/strings_<code>.toml`, one message per line, keys untouched.

**Materials, export, UI.** An eight-material library with per-value provenance ·
DXF with exact arcs for external *and* internal gears, written to the published
R2000 minimum so a reader that repairs nothing still opens it — confirmed
importing into SOLIDWORKS · geartrains exported and
imported as TOML, inputs only · gear tabs with external and internal kinds — and
eccentric, in the developer mode — geartrain tabs with spur, worm and planetary
stages.

---

## Decided, not pending

These will not be built, and the reason is on screen where the number would have
been. They are not a backlog.

| Item | Why |
|---|---|
| **Crossed-axis bending** | The beam formula has no honest reading of a point load on a wide tooth, and choosing an effective width is a convention that multiplies a stress. [rationale](rationale.md#a-worm-stage-reports-no-bending-stress) |
| **ISO/AGMA correction factors** | Narrow validated bands, balanced only as a complete set, against `σ_Flim` values this project does not have. [rationale](rationale.md#no-isoagma-correction-factors) |
| **Equal planet load sharing** | The remedy is a mesh-load factor of the kind above. Said in every planetary result's notes. |
| **An S-N curve per material** | The two points it needs do not exist for six of the eight materials. [rationale](rationale.md#material-data-ships-estimates-deliberately) |
| **Radial assembly** | Attempted, diagnosed and shelved with its findings; it blocks nothing, and planets are commonly installed axially. |

---

## Not built

| Item | Note |
|---|---|
| An **eccentric ring** as a tab kind | The core supports it — `centre_profile` takes which member the eccentric gear is — and the tab does not. A UI decision rather than a limit |
| The **enveloping** (throated) wheel's zone of action | The cylindrical one is derived and a worm reports it as a floor, with its assumed tooth height named |
| Tooth thickness tolerance (JGMA 1103-01) | Unavailable. Min/max on span and over-pins only; the result types carry the space |
| Span over teeth for a ring | Takeable in principle, rare in practice, not derived. Between-pins is done and the tab says which is which |
| A crossed pair's tooth form reaching its mesh figures | Would need the crossed mesh derived at a shifted centre distance. The form is still specified, and the panel says what it does and does not reach |
| Worm profile drawing and DXF | A crossed pair draws as its two helical gears already |
| A planetary **set's** drawing | The viewport draws single gears; a set needs the carrier and N planets placed. **Not planned** — nothing depends on it, and the set's numbers are all reported without it |
| A ring's own bounds for a stage member | The gear card shows a rack's buildable range, which is not a ring's, so it shows nothing there and says so |
| `Driven By` as a train direction on a worm stage | Back-driving is reported, not modelled as a train direction |
| A coupled glass POM grade | Can be added if one is wanted; it must be *coupled*, not filled |

---

## Known-approximate, documented at the call site

- **`Y_β` omitted** — helical bending is conservative against a published ISO
  rating by up to ~25 %.
- **The axial compression term is omitted** from bending, following ISO rather
  than AGMA. It relieves stress by order 10 %, so leaving it out is conservative;
  do not compare to an AGMA `J` without saying so.
- **A ZN worm's contact stress is 1–15 % below the reported ZI figure.**
- **A ring's flank below its generation limit is not a generated involute** —
  about 0.08 mm on ordinary designs. Flagged per part.
- **The cut simulation cannot see below the generation limit**: its simulated
  cutter has no fillet of its own, so what it reports there is not evidence
  either way.
- **The `Y_S` notch band (`1 ≤ q_s < 8`) is a secondary source**, not a reading
  of ISO 6336-3. It is the only second-hand constant in the geometry path, it is
  confined to the empirical correction, and whether it was applied is reported.
- **Load sharing above a virtual contact ratio of 2** is the ramp extrapolating:
  no single-pair zone exists, and it relieves the tooth by about a third. The
  stage says so where the figure is shown.
- **Hardened 4340's fatigue allowable is the weakest number in the library.**
- **A note slot is as tall as the tallest note that field can show.** Every real
  message fits; a validation message longer than its field's bound note would
  still move the controls when it appeared.

---

## Two notes nothing can fire

Both are live code with live messages, so neither is deleted on suspicion. They
are named in `strings.rs`'s `UNFIRED` with their evidence.

- `clamp.ring_fully_filleted` — searched for over ~11 000 ring/cutter
  combinations and never fired. `ShaperCut` already refuses a tool whose rounds
  overlap, which may shadow it entirely.
- `stage.ring_addendum_clamped` — needs a planetary ring whose tip clamps, and
  the set solves its own ring addendum.

---

## Worth doing next

Not a queue with a head; this is what a next session would pick from.

- **Further UI work**, as it is asked for.
- **Read ISO 6336-3** and settle the `Y_S` notch band from the standard rather
  than from a citation of it.
- **A calibrated mesh-stiffness model**, which would replace the load-sharing
  ramp rather than the control now exposing it.
