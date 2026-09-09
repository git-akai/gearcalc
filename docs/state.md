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
tools/check_doc_links.py          # every pointer into the documents resolves, from code and from each other
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

The list below is the interesting ones. **The exhaustive one is the harness's
own module comment** — `crates/gear-cli/src/main.rs`, next to the code it
describes, where it cannot fall out of step with the commands it lists. This one
has (`dump`, `dxf`, `loadcase`, `matrix` and `sweep` are not here), and a
curated list that reads as exhaustive is the worse of the two failures.

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
cargo run --bin gear-cli -- hula 18 0.2            # a hula stage, offset to teeth to ratings
cargo run --release --bin gear-cli -- meshsweep 60 20 0.8   # roll an internal pair, the control
cargo run --release --bin gear-cli -- hulasweep 18 0.25     # ...and a hula pair, where the tips cross
cargo run --release --bin gear-cli -- hulaband 18           # one reduction at every tooth difference
cargo run --bin gear-cli -- bending                 # the bending construction, drawn
cargo run --release --bin gear-cli -- matrix        # the bending matrix, on external teeth and on rings
cargo run --release --bin gear-cli -- verify 100   # the two-sided cutter check
python3 tools/worm_flank_curvature.py              # ZI vs ZN vs ZA, from the surface
python3 tools/crossed_path.py                      # the crossed path, from the surfaces
python3 tools/hula_kinematics.py                   # the hula ratio, from the rolling circles
python3 tools/iso_6336_3_stack.py                  # where this tool stands against ISO 6336-3, factor by factor
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
| `gear-core/src/plane.rs` | The normal and transverse planes, the identities that carry a quantity between them, and the basic rack they act on. One home, because the two angles had nineteen and the base pitch six. |
| `gear-core/src/hula.rs` | The hula arrangement: the integer ratio, the one crank offset, and the shifts that let both meshes run at it. |
| `gear-core/src/train/hula.rs` | ...and the stage that builds the parts it describes, and rates them. |
| `gear-core/src/train/mod.rs` | What every stage kind shares: the load cases, `MemberRating` — every mesh a member is in, and the worst of them — `Bending`, `MeshReport`, the engagement rule, and the train that strings the stages together. |
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

**ISO 6336-3's factors, and which of them are here.** `σ_F0` is
`F_t/(b·m_n) · Y_F · Y_S · Y_β · Y_B · Y_DT`. `Y_F` is measured off the profile
this crate generates rather than taken from the standard's closed form. **`Y_S`
and `Y_B` are applied and everything else is declined**, and the test that
separates them is not the direction any one factor points: it is whether the
factor is half of a balanced pair. `Y_S` and `Y_B` are not — nothing else in
this model pushes back against a notch factor or a thin rim. `Y_β` is: the 2019
edition revised it and `Y_F` *together*, `f_ε` lives inside that revised `Y_F`,
and their product is **below 1** for any gear with full axial overlap. Measured
against a published ISO 2019 rating this tool sits 1.26–1.36× at full overlap
and 0.78–0.94× at low overlap and high helix — conservative where gears are
designed, below the standard only in the regime `stage.overlap_below_one`
already flags. `tools/iso_6336_3_stack.py` multiplies the set out
([rationale](rationale.md#the-helix-factors-are-a-pair-and-this-tool-can-take-neither)).

**Crossed axes.** One model rather than a family: the lead angle exact, the path
of contact from two properties of an involute helicoid, elliptical contact,
sliding as a vector, and one friction balance containing both older efficiency
formulas. A crossed gear pair is a spur stage with an axis angle — one stage
kind fewer than a family of them would need. A crossed pair's face width is automatic from `ε ≥ 1`, a
*geometric* minimum; a worm keeps its published proportions. Both are labelled
with which kind of minimum they are, because they differ by 2.4× and answer
different questions.

**Internal gears.** The ring's flank, its profile shift, a shaper-cut fillet at
the centre distance the shift puts the tool at, the flank/fillet tangency, the
generation limit, **three** mesh interference conditions — two about a tip
reaching past a flank where the teeth mesh, and one about the tips fouling where
their circles cross, which is what decides a small tooth difference — and a
bending rating.
Verified by simulating the cut — 2.5–2.7 µm across shifts −0.4 … +0.5.

**Planetary sets.** The shift that makes the two centre distances agree — the
planet's by default, and the sun's or the ring's where the planet is pinned,
which is one relation among three shifts and so two of them a design; the sun
and ring cases are closed form where the planet's needs a solve,
the ring search, layout checks, Willis kinematics, Pennestrì–Freudenstein
efficiency in all six arrangements, and backlash referred to the output shaft.

**A tooth costs what it measures.** Building one ran a two-thousand-point scan of
its trochoid to ask whether undercut had removed the flank entirely — on every
tooth, including the great majority that are not undercut at all and so have
nothing for it to find. Severing is undercut taken to its limit, gated by
sweeping thirty thousand teeth that are not undercut across seven rack
proportions, so the scan is skipped for them. A tooth went from 62 µs to 219 ns,
and with it every search that builds hundreds of teeth to read a radius and a
flag off each: the pair's from 34 ms to 0.7, the hula stage's from 35 to
1.7, the epicyclic set's from 68 to 10.

**Two controls on a profile shift, and they are not the same kind of thing.**
`auto` says who decides the number; `no undercut` says what the answer has to
satisfy however it is decided — so they combine, and the automatic shift this
tool has always offered is the pair of them on at once. A shift given by hand is
held to the true undercut minimum, which is negative on a comfortable tooth
count, so a deliberate negative shift survives and only a genuinely undercut one
is raised — and it says so when it is. A search is floored at the automatic
value instead, for a reason that is measured rather than tidy
([reference](reference.md#efficiency-parallel-axes)). Where the bound cannot
reach at all — a shift a *relation* leaves over, which nobody chose and nothing
can move — the tooth is reported undercut instead, on every rack-cut member of
every stage kind. A ring has only the first control: its flank is its shaper's, and undercut is not a question that can be
asked of it. The hula stage's "shift given on" select is gone with it — a
mesh has one shift to give, and which member gives it is what the toggles say.

The addendum carries the same pair, on the other end of the tooth: `no sharp
tip` holds it to the tallest that keeps a tip `min_tip_width` wide, where an
`auto` toggle used to *be* that tooth and left a typed addendum unbounded. The
hula stage reports that bound rather than acting on it — its crank offset
is a closed-form solve on the tips, and an addendum moving with the shift would
put an iteration inside it.

**Every parallel-axis mesh reports the same things**, from one type rather than a
copy per stage kind: the operating pressure angle, all three contact ratios,
whether the pair hunts, the efficiency both ways, the contact stress the two
members share, and the one gap seen from each of its ends. A crossed pair has
none of it — its line of action slides rather than turning, so there is no such
angle and no contact ratio to report.

**And every member is rated over every mesh it is in**, likewise once rather
than once per stage: two stresses against two load cases, the face width each of
those would need, and the worst mesh answering figure by figure. Most members are
in one mesh, a planet is in two, and adding a third is adding a list entry rather
than an arm to an expression. The loadings are held **per load case** rather than
as one list and a factor, because "the second case is the first times a number"
is a claim about a stage's *power flow* and not about gearing — every kind here
can make it, and one that could not would build each case for itself with nothing
added.

**A stage that cannot be built still shows what built it.** A geartrain
mid-edit is regularly one that will not solve, so every input, note and label
stands and only the figures go blank — a readout that vanishes takes its label
with it and moves the page at the moment it most needs to hold still. The
refusal itself crosses as a `Note` and the stage it happened in, so it reads in
the catalogue's words like every other message and names where to look
([rationale](rationale.md#an-input-does-not-wait-on-an-answer)).

**Profile shifts chosen for efficiency.** A stage-level toggle, off by default,
that chooses the automatic shifts to lose least instead of taking the least that
clears undercut — which becomes a floor rather than the answer, and is worth up
to 1.6 points of mesh efficiency on an ordinary pair. Whatever is given
constrains the search instead of being overruled by it: a shift is that gear's, a
centre distance fixes the two shifts' sum, and pinning all three is relieved
visibly rather than silently ignored. The spur pair, the planetary set and the
hula stage each have their own free variables and their own objective over
one shared search; the worm stage has no profile shift to choose. Two bounds that
never bit near zero shift do here — a contact-ratio floor, which is a stage input
because the answer sits against it, and bottom clearance, which reads the
dedendum the designer already specified ([reference](reference.md#efficiency-parallel-axes)).

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
to the gear; the commanded centre distance and what a simple crank leaves —
either the shift amplitude or the centre-distance offset is the input and the
other is solved, an inversion that reads the shift law directly rather than
building a gear per trial; and
inspection data — span and over-pins — as the range it takes around the
revolution, verified against a caliper reading off the drawn teeth.

**Trains.** Spur/helical, worm, planetary and hula stages in one
train — the last behind the developer knock, as the eccentric gear is; torque,
backlash and cycle accumulation; efficiency and backlash in **both** drive
directions. Contact is `max(elliptical, line)` on **both** mesh kinds now — a
crossed pair's ellipse lengthens as its shafts come parallel, so the line its
teeth actually provide is what carries the load there, and rating on the ellipse
alone under-stated a near-parallel pair eightfold. Load sharing is a stage input
on **every kind that reports a bending stress**, off by default, reaching
bending alone — a ring included, which had no shared section of its own. Below a
virtual contact ratio of 2 the model finds the point the unshared rating already
took and reports the same tooth, which is the model rather than a fault; a hula
stage cannot reach that band at any proportion it can be built at. Two load cases throughout — a peak against the ultimate allowable
and a cyclic one against fatigue — with the automatic face width sized from any
of the four ratings a gear chooses. **Neither contact rating is enabled by
default**: both are computed and shown, but a fresh stage is sized from bending
alone, so its face width will not satisfy contact until a designer says which
rating should decide it — the figures are on screen, and the minimum face width
each rating asks for is beside them. The two gears of a mesh are rated at different points on the path
— each where its own dedendum is loaded alone — so they carry different contact
stresses; the shared pitch-point figure is reported at the mesh. An automatic
width answers to the mesh, not to one gear, and so does the width a member is
*rated* at — the narrower face carries the pair, so that is the width the load
is spread over. A back-driving load applied at the output
finds the stage that reacts it, or reports that nothing does. A reversing
intermittent drive rounds its cycles within one actuation and splits contact
between the flanks. **Reversed bending is a train-wide switch, off by default**:
a planet's root is loaded both ways whatever the drive does, a reversing drive
loads every root both ways, and each gear that one reaches says so beside its own
numbers — corrected against the reduced allowable only where the switch asks for
it. No member of a hula stage is reversed structurally: its wobble body
carries two gears rather than one, and each of them meshes once. A notch
parameter outside the band the `Y_S` fit is stated for says so on the gear too.

**Tooth cycles follow one rule for every arrangement**: a member is engaged once
per revolution *relative to the carrier of its mesh*, once for each parallel
mesh path. A pair has no carrier and one path, so this is its own revolutions; an
epicyclic set has both, and the consequence a per-member reading cannot state is
that a shaft which does not turn is still loaded — a held ring meets a planet
once per carrier revolution.

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
importing into SOLIDWORKS · geartrain stages: spur/helical, crossed, worm,
planetary and **hula**, the last behind the developer knock · geartrains
exported and imported as TOML, inputs only · gear tabs with external and internal
kinds, and eccentric in the same developer mode.

**One word for a stage kind, and it is "stage"** — a worm stage, a hula stage,
never a drive ([rationale](rationale.md#a-stage-kind-is-a-stage)). "Drive" names
which way power flows and how the train is actuated, and nothing else.

---

## Decided, not pending

These will not be built, and the reason is on screen where the number would have
been. They are not a backlog.

| Item | Why |
|---|---|
| **Crossed-axis bending** | The beam formula has no honest reading of a point load on a wide tooth, and choosing an effective width is a convention that multiplies a stress. [rationale](rationale.md#a-worm-stage-reports-no-bending-stress) |
| **ISO/AGMA correction factors** | Narrow validated bands, balanced only as a complete set, against `σ_Flim` values this project does not have. [rationale](rationale.md#no-isoagma-correction-factors). `Y_S` and `Y_B` are the exceptions and are applied, neither being half of a pair; `Y_β`, `f_ε` and `Y_DT` are declined and recorded in full below |
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

- **Helical bending is conservative against ISO 6336-3:2019 by 26–36 %** at
  full axial overlap, and **below it by up to 22 %** at an overlap ratio under
  0.3 with a helix over 20° — the one regime where this model runs under the
  standard, and one `stage.overlap_below_one` already flags. Measured with
  `tools/iso_6336_3_stack.py`, not asserted; the figure the documentation used
  to quote was right by accident.
- **`Y_S` is stated for external spur gears at `α_n = 20°`**, and gives
  "approximate values" for internal gears and other pressure angles by the
  standard's own words. This tool applies it to both, and to a section located
  by the inscribed parabola rather than the tangent it was calibrated against.
  None of the three has an edge to report, so they are stated in
  [`reference.md`](reference.md#bending) rather than raised per gear.
- **The axial compression term is omitted** from bending, following ISO rather
  than AGMA. It relieves stress by order 10 %, so leaving it out is conservative;
  do not compare to an AGMA `J` without saying so.
- **A ZN worm's contact stress is 1–15 % below the reported ZI figure.**
- **A ring's flank below its generation limit is not a generated involute** —
  about 0.08 mm on ordinary designs. Flagged per part.
- **The cut simulation cannot see below the generation limit**: its simulated
  cutter has no fillet of its own, so what it reports there is not evidence
  either way.
- **The `Y_S` notch band is `1 ≤ q_s < 8`**, read from ISO 6336-3:2019, 7.2.
  It was carried as a citation of a citation for a year and the two agree.
  Whether a gear falls outside it is reported.
- **Load sharing above a virtual contact ratio of 2** is the ramp extrapolating:
  no single-pair zone exists, and it relieves the tooth by about a third. Each
  mesh says so where its figure is shown, so a set with one mesh in the band and
  one out names which. **Below the band it changes nothing** — the single-pair
  boundary is in the sweep at a share of exactly 1, so the maximum is the point
  the unshared rating already took — and a hula stage cannot reach the band at
  any proportion it can be built at.
- **Hardened 4340's fatigue allowable is the weakest number in the library.**
- **A face width typed as zero describes a gear with no face**, and every
  rating taken at one is infinite. Those cross the boundary as `null` and the
  panel draws them blank, so the browser reads correctly; the CLI prints `inf`.
  A degenerate input rather than a reachable state of the controls: an automatic
  width with no rating to size it stands at the number it was given.
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

## An open finding: the parabola on a ring

**Not acted on.** The default critical section is the inscribed Lewis parabola
for every member, external and internal, and it still is. This records what
measuring it found, because the measurement was not possible until the 60°
internal tangent existed — before that a ring's tangent section was taken at
30°, so there was no baseline to compare against.

`gear-cli matrix` now runs its four studies on both kinds of member, plus a
fifth that puts the two constructions against each other:

| | external (1508) | ring (160) |
|---|---|---|
| parabola tangency on the **flank** | 12.9 % | **100 %** |
| `Y_F` parabola/tangent | 1.006–1.312, mean 1.055 | 1.273–1.690, mean **1.425** |
| `Y_F·Y_S` parabola/tangent | 0.740–1.351, mean 0.943 | 0.789–1.226, mean **0.877** |
| mean `q_s`, parabola vs tangent | 1.95 vs 3.17 | **1.00** vs 4.94 |
| outside the `Y_S` band | 19.2 % vs 5.8 % | **66.9 %** vs 1.2 % |
| Spearman ρ, `Y_F` | **0.993** | **0.537** |
| Spearman ρ, `Y_F·Y_S` | 0.891 | **0.289** |

**What it says about tooth strengths.** A ring's reported bending factor is on
average **12 % below** what the 60° tangent gives, spanning 21 % below to 23 %
above — so the choice of construction is worth about ±20 % on a ring's root
stress, against about ±6 % on the mean for an external tooth. Lower is the
unconservative direction.

**Three things worth separating.**

1. **The parabola lands on the flank for every ring.** Already known for
   `z ≥ 40` (see [corrections](corrections.md)); now measured across shift and
   pressure angle too, and it is universal. `s_Fn` is then read across a point
   on the involute while `ρ_F` falls back to the fillet junction — a documented,
   continuous fallback, but two different places, and on a ring it is not the
   exception it is externally.
2. **`q_s` collapses to the bottom of the `Y_S` band.** Mean 1.00 against the
   tangent construction's 4.94, with **two rings in three clamped**. Clamping
   below the band is the conservative direction, but it means most rings are
   rated with a fitted factor held at its boundary rather than evaluated.
3. **The ranking argument does not transfer.** The stated reason the parabola's
   divergence from the standard is "principled rather than consequential" is
   ρ = 0.993 — an **external** measurement. On rings it is 0.537 on `Y_F` and
   0.289 on the product, and one pair of models in the matrix orders ring
   designs in *opposite* directions (ρ = −0.399).

**And one correction that falls out of it regardless.** "The parabola is the
more conservative construction" is a claim about `Y_F` and does not survive
`Y_S`: a narrower section at the same notch radius is a smaller `q_s`, and `Y_S`
rises with `q_s`. On the product — the number a stress is proportional to — the
parabola is *below* the tangent construction on both populations, 0.943 external
and 0.877 internal. The doc comments said "more conservative, everywhere"; they
say `Y_F` now.

**What would settle it.** A ring's bending model cites Savage, Rubadeux & Coe
(NASA TM-107012), which chooses the inscribed parabola deliberately for internal
teeth — so the construction is not an unprincipled transplant, and the paper is
the place to check whether its parabola also lands on the flank or whether
something in the inscribing differs. Failing that, the honest options are to
make `TangentAngle` the default **for rings only** — ISO's construction, ISO's
angle, `Y_S` used where it was calibrated, 1.2 % out of band instead of 66.9 % —
or to keep one construction for both and say plainly that a ring's is running
outside the notch fit's domain most of the time. It is a model decision and is
being left to one.


---

## Recorded but not applied: ISO 6336-3's remaining bending factors

Three factors of `σ_F0` are declined, and this section is why plus everything
needed to change that decision **without the standard in hand**. Recording them
is not a plan to add them — it is so that the next person to ask does not have
to buy the document to find out what was turned down.

**`Y_β` and `f_ε` are declined as a pair, and that is the whole reason.** The
2019 Foreword lists its first two changes as a modification of `Y_β` (Clause 8)
and of `Y_F` (6.2); `f_ε` is inside that revised `Y_F`. Taking one without the
other was tried and reverted — it moved this tool *away* from the standard —
and the arithmetic is in `tools/iso_6336_3_stack.py`
([rationale](rationale.md#the-helix-factors-are-a-pair-and-this-tool-can-take-neither)).
Adopting them would mean adopting **both**, and then facing the two mixings that
remain: this crate's `Y_F` is measured at an inscribed-parabola section where
`Y_β` is fitted to a 30°-tangent closed form, and its virtual gear uses
`z_n = z/cos³β` where ISO 2019 uses `z_n = z/(cos²β_b · cos β)`.

### `Y_β`, the helix angle factor (Clause 8)

```text
Y_β = (1 − ε_β · β/120°) / cos³β
```

with 1 substituted for `ε_β` above 1 and 30° for `β` above 30°. Above `β = 25°`
the standard asks for the factor to "be confirmed by experience". Its Figure 8
plateaus at 1,50 for `ε_β = 0,1` and 1,155 for `ε_β = 1`, both at `β = 30°`,
which the formula reproduces to 1,5011 and 1,1547 — the check that pins the
`1/cos³β` term, since the 2006 form would give 0,975 and 0,75 and the figure's
ordinate starts at 0,9.

### `Y_DT`, the deep tooth factor (Clause 10)

Adjusts the nominal root stress where the load is taken at the inner point of
*triple* pair contact rather than the outer point of single pair contact.

```text
Y_DT = 1.0                          ε_αn ≤ 2.05,  or ISO tolerance class > 4
Y_DT = −0.666 · ε_αn + 2.366        2.05 < ε_αn ≤ 2.5  and class ≤ 4
Y_DT = 0.7                          ε_αn > 2.5         and class ≤ 4
```

`ε_αn = ε_α / cos²β_b` is the virtual contact ratio this crate already computes
for the bending section. The two inputs it needs and this project does not have
are **an ISO tolerance class** (the worse of the pair's, if they differ) and an
assertion that **actual profile modification for a trapezoidal load distribution
along the path of contact** has been applied. Both are manufacturing claims
rather than geometry, and neither can be derived from anything on screen.

**Why declined.** It reaches at most 0.7, so it lowers a root stress by up to
30 % on the strength of two things a designer asserts rather than draws. Its
band is 0.45 of contact ratio wide. And the graph it fits (Figure 10) is two
straight lines, one of which is the constant 1 that a design failing either
input already gets — so for everything this tool can verify, `Y_DT` **is** 1.

### `f_ε`, the load distribution factor inside `Y_F` (6.2, Formulae 10–14)

`Y_β`'s other half. A multiplier on the form factor rather than a factor beside
it, said to give "more accurate results for gears with contact ratios
`ε_αn ≥ 2,0`".

```text
f_ε = 1                                    ε_β = 0      and ε_αn < 2
f_ε = 0.7                                  ε_β = 0      and ε_αn ≥ 2
f_ε = √(1 − ε_β + ε_β/ε_αn)                0 < ε_β < 1  and ε_αn < 2
f_ε = √((1 − ε_β)/2 + ε_β/ε_αn)            0 < ε_β < 1  and ε_αn ≥ 2
f_ε = ε_αn^(−0.5)                          ε_β ≥ 1
```

Both inputs — the overlap ratio and the virtual contact ratio — are already
computed here, so unlike `Y_DT` this one is reachable today. It is declined
anyway.

**Why declined.** Primarily because `Y_β` is, and the two only mean anything
together. On its own merits it also answers the same question as the
`LoadSharing` model this project already ships as a designer-facing option: how
the mesh load divides while more than one pair is engaged. Adopting `f_ε` would
apply *a* sharing model unconditionally, inside the form factor, where the
existing one is off by default and says when it is extrapolating — the opposite
of this project's stated posture on estimates. It is also a step function: 1 and
0.7 either side of `ε_αn = 2` at zero helix, with nothing physical happening at
exactly 2.

**One thing it is worth for, and it is not a small one.** `f_ε = 0.7` for a spur
mesh at `ε_αn ≥ 2` is an independent corroboration of this project's own
measurement — that above a virtual contact ratio of 2 the linear ramp "relieves
the tooth by about a third", found by sweeping designs and reported in every
mesh that reaches the band. Two models built on different reasoning landing on
the same 30 % is the best evidence either of them has.

### And two clauses read but not needed

- **`Y_Sg`, grinding notches in the fillet** (7.3): `Y_Sg = 1.3 Y_S / (1.3 − 0.6
  √(t_g/ρ_g))`, valid for `√(t_g/ρ_g) < 2.0`, where `t_g` is the notch depth
  measured perpendicular to the tangent and `ρ_g` its root radius. Nothing here
  models a grinding notch, and the two inputs are process rather than geometry.
- **`Y_ST` = 2.0** (7.4), the stress correction factor of the reference test
  gears the `σ_Flim` values are quoted against. It belongs to the *permissible*
  stress rather than the applied one, which is
  [material data](rationale.md#material-data-ships-estimates-deliberately)'s
  territory and not this crate's.


---

## Worth doing next

Not a queue with a head; this is what a next session would pick from.

- **Further UI work**, as it is asked for.
- **Multiply the set out before adopting anything else from a standard.**
  `tools/iso_6336_3_stack.py` is the pattern: a factor's direction is a property
  of the set it was calibrated in, not of the factor, and this project has been
  caught by that once.
- **Wire `rim_thickness` into the boundary and the UI.** The model is complete
  and gated in `gear-core`, the input is a field on `StageGear` so it already
  crosses as a wire type, and `gear-cli strength … <rim>` exercises it. What is
  left is a form field on the gear card, a serde-defaulted `null` in the tab
  state, and one label plus one unit in five string catalogues. Nothing about
  it needs the standard in hand.
- **A calibrated mesh-stiffness model**, which would replace the load-sharing
  ramp rather than the control exposing it.
- **A planet's root under the ring mesh is rated; its flank's sliding is not**
  — the fixed-carrier efficiency takes each mesh once, which is right, but no
  member reports what *it* loses. Nothing depends on it and nothing is wrong;
  it is the next thing a member-over-meshes model makes askable.
- **The stage kinds that need more than three shafts.** `MemberRating` and
  `MeshReport` are per member and per mesh rather than per named role, and
  `planetary::power` takes a basic ratio rather than a set of tooth counts — so
  a fourth kind should be new *kinematics* and no new rating machinery. That
  claim is untested until something tests it.
