# State

Where the project stands, what to run, and what is left. **Version 0.3.0** — the
minor bump is the load cases: a train carries any number of them, each a
torque at a port judged against one allowable, in place of the five load
fields it held; every rating crosses the boundary per case, and the geartrain
document changed shape with no shim. 0.2.0 was the bending model — the notch
factor, the fillet radius it reads and the parabola's selection rule moved to
one source, which moved the strength canary.

**This is the only document allowed to talk about the present**, and saying so is
what lets the others stop hedging. [`reference.md`](reference.md) states what the
tool computes, [`rationale.md`](rationale.md) why each model is the one chosen,
and [`corrections.md`](corrections.md) what was once wrong. What belongs here and
nowhere else is a claim that **dates**: what is built, what is not, what is
planned, what version this is, and what to run. This is the one file expected to
be rewritten rather than amended.

That rule was once written as a ban on the words "now", "still" and "currently",
and read literally it forbids 127 sentences across the other three — nearly all
of them either ordinary English ("`Y_β` above 25° must still be confirmed by
experience") or a model's history stated where the model is argued ("the shaper
caps now, by the same rule"). Neither dates. **The word was never the fault**:
the fault is a document hedging about a present it is not the one to describe,
and that is what this says instead.

---

## Running it

```bash
nix develop                       # or `direnv allow` once
cargo nextest run                 # the suite
nix flake check                   # build, clippy --deny warnings, fmt, tests
nix build .#web                   # ...and the site, which flake check does NOT cover
cd web && npm run dev             # the application
```

**`nix flake check` is not all of what CI runs**, and reading it as though it
were has cost one red build. The workflow also runs `nix build .#web` and the
front end's own `npm run check`, and the site build carries a **fixed-output
hash over `web/package-lock.json`** (`npmDepsHash` in `flake.nix`) that nothing
else consults — so any change to that lockfile, a version bump included, breaks
the site build alone and passes everything a developer usually runs. Before
pushing, run all four.

And the checks that live outside the Rust suite:

```bash
tools/check_bindings.sh           # the generated TypeScript matches the Rust
tools/check_bindings.sh --write   # ...or regenerate it
tools/check_doc_links.py          # every pointer into the documents resolves, from code and from each other
tools/check_strings.py            # every ui message is used, and every use has a message
tools/check_golden.sh             # every number the harness prints, against what it printed before
tools/check_golden.sh --write     # ...or accept what it prints now
tools/check_figures.py            # every figure these documents quote is one the code still prints
tools/check_figures.py --list     # ...and which tables nothing yet generates
cd web && npm run check           # typecheck the front end
```

`check_golden.sh` is a **change detector, not a correctness gate**: a diff is a
question — did you mean to move that? — and the answer is often yes. It exists
because the suite can only notice a number some test names, and three documented
tables have gone stale between two commits that no test was looking at.
`check_figures.py` is the other half: a documented figure carries a marker naming
the command that regenerates it, so provenance is in the document rather than in
anyone's head.

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

The list below is the interesting ones. **The exhaustive one is
`gear-cli help`**, which prints the harness's own dispatch table — so the list
*is* the code and there is nothing to keep in step.

That was not always true. This paragraph used to name the harness's module
comment instead, "next to the code it describes, where it cannot fall out of
step with the commands it lists". It had fallen out of step by eight of
twenty-one. **Proximity is not a mechanism**, and a curated list that reads as
exhaustive is the worse of the two failures — which is why the fix was to make
one list rather than to correct the second one.

```bash
cargo run --bin gear-cli -- show 17 0.2            # one gear's derived geometry
cargo run --bin gear-cli -- materials              # the library, with each value's basis
cargo run --bin gear-cli -- strength 17 43 2.0     # a worked mesh, end to end
cargo run --bin gear-cli -- shifts 9 37            # the shifts a pair loses least at
cargo run --bin gear-cli -- train                  # a two-stage train
cargo run --bin gear-cli -- train mixed            # ...with a worm stage in it
cargo run --bin gear-cli -- train held             # ...that worm holding more than it drives
cargo run --bin gear-cli -- trainfile [path]       # a train to TOML and back, answers compared
cargo run --bin gear-cli -- worm 1 40 7 90         # a worm pair, both directions
cargo run --bin gear-cli -- wormstage 1 40 7 2     # a worm stage, end to end
cargo run --bin gear-cli -- crossed 17 23 90       # a crossed pair, swept over the split
cargo run --bin gear-cli -- planetary 17 17 3      # every ring count that can work
cargo run --bin gear-cli -- planetstage 12 30 72 3 # a planetary stage, six modes
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

[`bending-check.html`](bending-check.html) is `gear-cli bending`'s figures with
the prose that reads them, kept because the bending construction is far easier
to judge by looking than by reading an assertion. It is a **document with
generated figures in it**, not a stored answer — and it says so in its own first
line, a `figures-verbatim` marker that `tools/check_figures.py` reads. If the
construction moves, that check fails and names the file; re-run the command and
paste the body back in.

The last two share no code with the crate — that is their whole purpose.
`crossed_path.py` builds both flanks as parametric surfaces and reaches the line
of action through differential geometry; the crate reaches it through a
construction in lines and angles.

<!-- figures-by-test: the_construction_reproduces_the_surfaces_derivation -->
On a 17/23 pair at 45°/45° with shafts at 90°, tips at `r + m_n`, they give
ε = 1.777921670 and 1.777921669562 — at the nominal centre, which is what the
script builds; the harness's `crossed` command runs the pair 0.02 mm open and
prints 1.758113579.

---

## The canaries

Two figures have survived every refactor unchanged, and between them they have
caught more in their areas than the suite has.

<!-- figures: gear-cli strength 17 43 2.0 -->
<!-- figures: gear-cli wormstage 1 40 7 2 -->
| | |
|---|---|
| `gear-cli strength 17 43 2.0` | `σ_F` 66.8 / 56.0 MPa · `σ_H` 692.7 MPa · ρ 1.723 mm · η 98.741 % |
| `gear-cli wormstage 1 40 7 2` | η 61.805 % forward, 0.000 % backward (self-locking) · backlash 0.15512° at the wheel (min 0.11342, max 0.19683), 6.20497° at the worm |

**The strength canary has moved twice, both deliberately, and both are the same
model arriving in two commits.**

1. `σ_F` 69.2 / 63.4 → 74.3 / 63.8, when the notch factor became Dolan and
   Broghamer's `K_f` read at the fillet's own minimum radius, in place of ISO's
   `Y_S` read at the critical section.
2. → **66.8 / 56.0**, when the **axial compression term** arrived — the second
   half of the same `J`, relieving by 10.1 % and 12.2 %.

Taken together the pair moves the canary from 69.2 / 63.4 to 66.8 / 56.0: one
factor up, one down, and the tool is no longer stacking the conservative halves
of two models ([rationale](rationale.md#a-conservative-answer-is-not-a-free-one)).
**`σ_H`, ρ and η did not move at either step**, which is the check that a
bending model stayed in bending, and the whole worm canary did not move either,
a worm stage reporting no bending stress at all.

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

Load cases moved neither canary, twice: when the second case arrived, and when
the pair of them became a list. Both are single-load reports, and a stage asked
for one torque answers with the figure it always did — which is the check that
a case was added rather than substituted for the first. The corpus's train
reports moved only in layout when the list arrived: every member's figures in
the three cases a train used to hold as fields are what they were.

---

## Layout

**The complete map is [`CLAUDE.md`](../CLAUDE.md)** — all 27 modules of
`gear-core`, each with what it owns and, more usefully, what it must *not* know.
This table named seven of them and read as though it named all of them, which is
the same failure the harness's command list had.

What is kept here is the entries that carry a **decision** rather than a
location: where a boundary is drawn, and what a directory is not for.

| Path | Role |
|---|---|
| `crates/gear-core` | All mathematics. No I/O, no UI, no wasm. `serde` and `ts-rs`, both optional and both about the shape a type takes when it leaves. |
| `gear-core/src/gear.rs` | `Gear` — the assembly, and the only place a gear is drawn. An ordinary gear is `Δx = 0`. |
| `gear-core/src/strength.rs` | The bending model: the critical section both kinds of member share, the notch factors and which fillet radius each reads, and the Hertz contact beside it. |
| `gear-core/src/train/mod.rs` | What every stage kind shares: the load cases and the walk that carries each toward the far port, `MemberRating` — every mesh a member is in, in every case, and the worst mesh — `Bending`, `MeshReport`, the engagement rule, and the train that strings the stages together. |
| `crates/gear-io` | File formats: DXF export, the TOML material library and geartrain documents, and the string catalogue. |
| `crates/gear-wasm` | The WebAssembly boundary. JSON in, JSON out. |
| `crates/gear-cli` | Development harness — drive the mathematics without a browser. |
| `web/` | Svelte 5 + TypeScript + Vite front end. |
| `web/src/wire/` | **Generated.** The Rust types that cross the boundary, written down by `ts-rs`. |
| `crates/gear-io/data/strings_en.toml` | **Every word the application shows**, one file per language. |
| `handoff_inbound/` | Prior Python work. **Reference only** — do not build on it. |
| `docs/history/` | The superseded design record, kept for provenance and pointed at by nothing — and the closed audit's record, `audit.md`, which code cites by finding number for its measurements and which governs nothing |

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
designed, below the standard only in the regime `mesh.overlap_below_one`
already flags. `tools/iso_6336_3_stack.py` multiplies the set out
([rationale](rationale.md#the-helix-factors-are-a-pair-and-this-tool-can-take-neither)).

**Crossed axes.** One model rather than a family: the lead angle exact, the path
of contact from two properties of an involute helicoid, elliptical contact,
sliding as a vector, and one friction balance containing both older efficiency
formulas. A crossed gear pair is a spur stage with an axis angle, and a worm is
the same stage with its first member sized by pitch diameter — one `PairStage`
under the spur and worm kinds, where the kind is a preset, a vocabulary and a
choice of which inputs to show ([rationale](rationale.md#each-stage-kind-keeps-its-own-result-type)).
A profile shift enters the crossed mesh as a rack's does, exactly, so a worm's
wheel absorbs a housing distance by its shift and the worm's diameter absorbs it
when both shifts are pinned; the interference verdict is the parallel relation
asked along the line of action; the optimiser reaches it with the friction
balance as its objective. A crossed pair's face width is automatic from
`ε ≥ 1`, a *geometric* minimum; a worm keeps its published proportions. Both are
labelled with which kind of minimum they are, because they differ by 2.4× and
answer different questions. Every mesh, on parallel shafts or crossed, reports
in one `MeshReport`, and where the two contacts' figures meet is measured and
recorded with its seams ([reference](reference.md#contact-stress)).

**Internal gears.** The ring's flank, its profile shift, a shaper-cut fillet at
the centre distance the shift puts the tool at, the flank/fillet tangency, the
generation limit, **three** mesh interference conditions — two about a tip
reaching past a flank where the teeth mesh, and one about the tips fouling where
their circles cross, which is what decides a small tooth difference — and a
bending rating.
Verified by simulating the cut — 2.5–2.7 µm across shifts −0.4 … +0.5.

**Planetary sets.** The shift that puts the two meshes at one physical distance
with the running clearance in both — their zero-backlash distances differing by
twice it, the internal mesh opening as its centres close — the planet's by
default, and the sun's or the ring's where the planet is pinned, which is one
relation among three shifts and so two of them a design; the sun and ring cases
are closed form where the planet's needs a solve. On an ideal ring with nothing
else asked that is the planet thinned by the clearance; the shipped 12/30/72
has a sun small enough to need shift, so it opens with +0.298 on the sun and
−0.170 on the planet, and its planet–ring mesh fouls at a full-depth ring as
the reference records. The
ring search, layout checks, Willis kinematics, Pennestrì–Freudenstein
efficiency in all six arrangements, and backlash referred to the output shaft —
which on the ideal ring a centre tolerance cannot move, the sun mesh gaining
what the ring mesh loses. The set's clearance is always given: it is what the
two nominal distances differ by, and no one distance could hand it back.

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
than once per stage: two stresses in every load case, the face width each of
those would need, and the worst mesh answering figure by figure. Most members are
in one mesh, a planet is in two, and adding a third is adding a list entry rather
than an arm to an expression. The loadings are held **per load case** rather than
as one list and a factor, because "the next case is this one times a number"
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
one shared search; a crossed pair's shifts are placed by the constraints alone,
since the search is the parallel-axis mesh's. Two bounds that
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
stage cannot reach that band at any proportion it can be built at. **Load cases
are a list**, as the stages are: any number, each an ultimate or a fatigue load
— the allowable it is judged against — entering at the start or the end, held
by the far end or by nothing but a stage that locks, at its own torque and
speed, a fatigue case with its own duty; every stress, cycle count and width
is reported per case, and the automatic face width is the largest ask of every
enabled case of a kind a gear's four toggles switch on. A fresh train carries
three — the peak, the load from the end and the operating duty it used to hold
as fields, at the same defaults. **Neither contact rating is enabled by
default**: both are computed and shown, but a fresh stage is sized from bending
alone, so its face width will not satisfy contact until a designer says which
rating should decide it — the figures are on screen, and the minimum face width
each rating asks for is beside them. The two gears of a mesh are rated at different points on the path
— each where its own dedendum is loaded alone — so they carry different contact
stresses; the shared pitch-point figure is reported at the mesh. An automatic
width answers to the mesh, not to one gear, and so does the width a member is
*rated* at — the narrower face carries the pair, so that is the width the load
is spread over. A load from either port is carried toward the other by one
walk, attenuated by each stage's efficiency in the direction it travels, and
finds the stage that holds it — a self-locking worm from the end, a
forward-locked crossed pair from the start — or the far end, or reports that
nothing does. A reversing intermittent duty rounds its cycles within one
actuation and splits contact between the flanks. **Reversed bending is a
train-wide switch, off by default**: a planet's root is loaded both ways
whatever the load does, a reversing duty loads every root both ways in its
case, and each gear that one reaches says so beside its own numbers —
corrected against the reduced allowable only where the switch asks for it. No member of a hula stage is reversed structurally: its wobble body
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
| The **enveloping** (throated) wheel's zone of action | The cylindrical one is derived from the members' own teeth, and a worm reports it as a floor |
| Tooth thickness tolerance (JGMA 1103-01) | Unavailable. Min/max on span and over-pins only; the result types carry the space |
| Span over teeth for a ring | Takeable in principle, rare in practice, not derived. Between-pins is done and the tab says which is which |
| Worm profile drawing and DXF | A crossed pair draws as its two helical gears already |
| A planetary **set's** drawing | The viewport draws single gears; a set needs the carrier and N planets placed. **Not planned** — nothing depends on it, and the set's numbers are all reported without it |
| A ring's own bounds for a stage member | The gear card shows a rack's buildable range, which is not a ring's, so it shows nothing there and says so |
| A third port | A train has a start and an end; a load case names one. A stage kind with a third shaft a load could enter by would add a value to `Port`, and nothing else knows a direction |
| A coupled glass POM grade | Can be added if one is wanted; it must be *coupled*, not filled |

---

## Known-approximate, documented at the call site

Each entry carries a **size and a sign**, not just a direction. "Conservative"
is a description of an error, never an excuse for one
([rationale](rationale.md#a-conservative-answer-is-not-a-free-one)) — an entry
whose size is unmeasured is a debt still owed, and is marked as one.

- **Helical bending is conservative against ISO 6336-3:2019 by 26–36 %** at
  full axial overlap, and **below it by up to 22 %** at an overlap ratio under
  0.3 with a helix over 20° — the one regime where this model runs under the
  standard, and one `mesh.overlap_below_one` already flags. Measured with
  `tools/iso_6336_3_stack.py`, not asserted; the figure the documentation used
  to quote was right by accident.
- **`K_f`'s calibration contained no undercut teeth.** Dolan and Broghamer's
  photoelastic specimens "contained various standard gear teeth but did not
  include any undercut gears", and this tool rates undercut teeth. Size
  unmeasured, sign unknown — **a debt**. It is not raised per gear because an
  undercut tooth already says so on its own account.
- **The axial compression term is applied**, being the second term of the `J`
  whose first term is `Y_F`. It relieved the canary by 10.1 % and 12.2 %, which
  is the size the debt had been carrying. **ISO omits it**, so the ISO
  comparison set omits it too and a number from that set is not an AGMA `J`.
- **The load point is on the flank, and used not to be.** `d = ε_n − 1` base
  pitches back from the tip goes **negative below a contact ratio of 1** — a
  load point past the end of the tooth, where `root_section` answered by
  extrapolating the involute beyond the tip and reporting a longer moment arm
  than the tooth has. Held at the tip now, which is
  `ContactPath::highest_single_pair`'s own `.min(recess)` in the coordinate the
  sweep counts in. Measured over-prediction: 0.06 % at `ε_n` = 0.999, 2.8 % at
  0.95, 11.4 % at 0.8, 29.4 % at 0.5. The shipped hula stage's meshes sit at
  0.998 and 0.996, so its figures move −0.12 % and −0.25 %.
- **A ZN worm's contact stress is 1–15 % below the reported ZI figure.**
- **The rack round is the coefficient times the *transverse* module**, so a
  helical tooth's transverse fillet is `1/cos β` larger than the normal round
  a hob has — 6 % at 20°, and seven normal modules at a worm's 82°, where it
  never fits and the thread's fillet is the cap's (0.95 of the depth). No
  rating reads it: bending is taken on the virtual spur with the normal round,
  and a crossed rating never touches the fillet. What it does move is the
  junction radius the interference verdict reads, upward, which makes that
  verdict **conservative** by the difference, and it is why a worm's
  `clamp.fillet_capped` fires at every shift. A normal round's transverse
  section is an ellipse and neither circle is it; sign stated, size stated,
  unrepaired.
- **The two contacts do not quite meet where the shafts straighten**, and
  the reported figures carry the seams
  ([reference](reference.md#contact-stress)): the pitch-point pressure of a
  crossed pair sits **1.5 % below** a parallel one's at `μ = 0.08` because its
  balance loads the flank with `μ F_n` along a sliding direction that stays
  finite as the speed vanishes, where the line rating uses the transverse
  projection alone; and its peak pressure sits **5 % below** because *one pair
  carries everything* is a normal base pitch in along the line rather than a
  transverse one. Each is the standard convention of its own model; the
  parallel figure is the higher on both. At `μ = 0` the pitch point meets to a
  part in 10⁵.
- **A crossed pair's centre-distance error slides its contact along the shafts
  by `Δa / sin Σ`**, which is the model's own degeneracy toward parallel: at a
  hundredth of a degree the default 0.02 mm of clearance moves the contact
  95 mm, off any face. The reported zone then says *face* and a contact ratio
  under one; the rating falls back to the tip-limited zone rather than to an
  empty one. Neither is a number a designer should read at that angle, and the
  parallel solve is one shaft-angle keystroke away.
- **A ring's flank below its generation limit is not a generated involute** —
  about 0.08 mm on ordinary designs. Flagged per part.
- **The cut simulation cannot see below the generation limit**: its simulated
  cutter has no fillet of its own, so what it reports there is not evidence
  either way.
- **`Y_S`'s notch band, `1 ≤ q_s < 8`**, read from ISO 6336-3:2019, 7.2 — it
  was a citation of a citation for a year and the two agree. It bounds the **ISO
  comparison set only**; no stage applies `Y_S`, so no stage reports the band.
  `K_f` states none. And ISO's own 7.1: `Y_S` is derived from external spur
  gears at `α_n = 20°` and gives "approximate values" elsewhere — which is one
  more reason the comparison set is a comparison rather than the default, since
  `K_f`'s constants are functions of `α_n`.
- **Load sharing above a virtual contact ratio of 2** is the ramp extrapolating:
  no single-pair zone exists, and what it does to the figure has **no fixed
  direction** — measured from a 24 % relief to a 15 % increase across
  high-contact-ratio spur designs. Each
  mesh says so where its figure is shown, so a set with one mesh in the band and
  one out names which. **Below the band it changes nothing** — the single-pair
  boundary is in the sweep at a share of exactly 1, so the maximum is the point
  the unshared rating already took — and a hula stage cannot reach the band at
  any proportion it can be built at.
- **The unshared convention — full load at the highest point of single-pair
  contact — is not always the conservative reading.** A member whose form factor
  rises steeply toward its tip can be governed there instead, at a partial share
  but a longer moment arm: a planetary ring comes out **2.4 % higher** with
  sharing on than off. Measured, not assumed, and it is why the sharing sweep
  asserts reach rather than relief.
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

## One note nothing can fire

Live code with a live message, so it is not deleted on suspicion. It is named
in `strings.rs`'s `UNFIRED` with its evidence.

- `clamp.ring_fully_filleted` — searched for over 71 750 ring/cutter
  combinations and never fired. `ShaperCut` already refuses a tool whose rounds
  overlap, which may shadow it entirely.

A second used to be listed here — `ring_addendum_clamped`, on the reading that
a set solves its own ring addendum — and it fires on 441 of the 1 482 sets a
sweep can solve ([corrections](corrections.md)). The sweep that was cited had
five cases aimed at it, every one of which failed to solve inside an `if let
Ok` that said nothing.

---

## One bending model, and it is the NASA one

**Settled.** The critical section is the inscribed Lewis parabola and the notch
factor is Dolan and Broghamer's `K_f`, on **both** kinds of member. They belong
together: Savage, Rubadeux & Coe (NASA TM-107012), whose internal model is
explicitly "an extension of the model for an external gear tooth", carry exactly
that pair. The tool had been carrying half of it.

**What was wrong, and it was one thing wearing three faces.** The section came
from Savage; the notch factor came from ISO; and the fillet radius fed to the
notch factor came from neither. ISO's `ρ_F` is the radius *at the critical
section*; Dolan and Broghamer's `ρ_f` is *"the minimum radius of the fillets"*
(confirmed independently of the paper, in the Virginia Tech survey of the same
literature). This tool read it at the fillet **junction**, which is the flattest
point the fillet has — **up to 16.6× the minimum** on an external tooth and
11.3× on a ring over the matrix population (study 7 prints nine designs apiece,
whose middle of the range is 1.4–4.1 and 2.1–6.3). `q_s` is inverse in it, which is the whole of why a ring's sat on the floor
of `Y_S`'s band with two in three clamped.

### What changed

| | before | after |
|---|---|---|
| notch factor | ISO `Y_S`, fitted to the tangent section | Dolan–Broghamer `K_f`, fitted to this one |
| load resolution | bending term only | **both** terms of Savage's `J`: `Y_F − axial compression` |
| fillet radius it reads | at the junction (neither definition) | the fillet's **minimum**, `ρ_f` |
| when both curves have a tangency | fillet wins | the **weaker** wins, per the paper |
| pressure angles covered | 20° ("approximate" elsewhere, ISO 7.1) | `α_n` is an argument of the fit |
| a stated band to fall outside | `1 ≤ q_s < 8` | none; `K_f` is a product of powers |

`ρ_f` is read at the fillet's deepest point on the physical argument that a
trochoid is closest to the tool's own corner radius where the corner cut deepest
and flattens toward the flank. That is an argument, so
`the_fillet_is_tightest_at_its_root` sweeps the whole fillet on both kinds of
member and checks nothing anywhere is smaller.

### What it did to the numbers

The strength canary moved once, deliberately: `σ_F` 69.2 / 63.4 → **74.3 / 63.8**
— conservative, and `σ_H`, ρ and η did not move, which is the check that a notch
factor stayed in bending. **The ring is the change worth having.** Against the
coherent ISO set (60° tangent + `Y_S`), `gear-cli matrix` study 5:

<!-- figures-bold: gear-cli matrix -->
| ring, the parabola set over the ISO set | before | after |
|---|---|---|
| range | 0.789 – 1.226 | **0.901 – 1.163** |
| mean | 0.877 | **0.970** |
| spread | 0.437 | **0.261** |

Closer to agreement on both counts: the mean has moved from 12 % out to 3 %, and
the spread is down by two fifths. The remaining 3 % is **expected and has a
name** — the ISO set does not take the axial compression term, so it reports the
higher number, and a ratio a little under 1 is that difference showing up where
it should.

<!-- figures: gear-cli matrix -->
The external population reads 0.510 – 1.128, mean 0.827, and the wider spread
there is the same thing seen on a population that includes small and undercut
teeth, where the two constructions genuinely disagree about where the section
is.

### And one thing the sweep found on the way

A planetary ring's bending comes out **2.4 % higher** with load sharing on than
off. Sharing was assumed to relieve every tooth it reaches, and that was an
observation rather than an invariant: the swept maximum is a product of a form
factor rising toward the tip and a share falling away there, and a ring's `Y_F`
runs 2.49 to 0.14 across its flank, so it is governed near its tip at a partial
share — above what the unshared convention (full load at the single-pair
boundary) assumes. The sweep is doing its job; the convention is the
approximation. `the_sharing_model_reaches_every_member_that_bends` asserts reach
now, not direction, and says why.

### On mechanics-derived alternatives, which were looked for

`K_f` stays a 1942 photoelastic curve fit, and the search for something derived
instead came up empty for a reason worth recording. **Neuber's notch theory** is
genuinely elasticity-derived — it interpolates between closed-form deep-hyperbolic
and shallow-elliptical solutions — but every form of it needs a **notch depth**
and a net section, and a gear fillet is a transition from tooth to rim rather
than a notch cut into a prismatic bar. Choosing that depth is a convention, which
is the thing the exercise was meant to remove; the interpolation is also reported
to underestimate by about 9 % in bending. **Heywood** is semi-empirical rather
than derived. **Critical-distance methods** are material-dependent, which
`rationale.md` already refuses. No elasticity-derived stress concentration factor
for a gear fillet appears in the literature surveyed, and the surveys of that
literature report photoelastic and finite-element work throughout.

What `K_f` has instead is corroboration across methods and decades: Jacobson's
photoelastic work (1955) agreed with it, Chabert, Dang Tran and Mathis (1972)
were "substantially in agreement", and Wilcox and Coleman's finite-element study
(1973) found results "only a few percent different".

**Its limit, recorded rather than discovered later.** Dolan and Broghamer's
specimens "contained various standard gear teeth but did not include any undercut
gears", and this tool rates undercut teeth. That is the same class of limit as
`Y_S`'s 20°-only origin. It is not raised per gear because an undercut tooth
already says so on its own account.

### Is the tangent construction still earning its place?

Asked directly, because nothing in a *rating* reaches it: no stage, no wasm
entry point and no part of the application selects `CriticalSection` or
`RootStressModel`, so from the product's side both ISO pieces are unreachable.

**They are kept, and the reason is not sentiment.** `gear-cli matrix` is a
shipped, documented command that runs both coherent sets over both kinds of
member, and it is the only way this tool can be checked against a published
standard. That check has been worth having three times in short order: it is
what showed `Y_β` was half a pair, what showed the parabola set was landing
*below* the tangent set on rings, and what measured the junction-versus-minimum
fillet radius. A tool with no second opinion has no way to find those.

So the ISO pieces are not dead code — they are the instrument. What would make
them dead is the matrix command going away, and the two should then go together.

The two constants (30° and 60°) and `ToothOutline::tangent_angle_deg` exist only
for that construction and are named rather than buried so this is visible.

### The ISO set is still there, whole

`CriticalSection::TangentAngle` + `RootStressModel::Iso6336` is the other
coherent pair — ISO's section at ISO's angle, `ρ_F` at that section, `Y_S` fitted
on exactly that — and is what to switch to for a number comparable with a
published ISO rating. `gear-cli matrix` runs both, on both kinds of member.
**What is not offered as a default is a mixture**, and the two notch models read
different fillet radii off the same `RootSection` precisely so that neither can
be quietly fed the other's.

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
the tooth by about a third" as it then measured — the two agreeing on a number
this project has since re-measured at 12–24 % under a different notch model,
which is a coincidence worth less than it looked but still not nothing. Found by
sweeping designs and reported in every
mesh that reaches the band. Two models built on different reasoning landing on
the same 30 % is the best evidence either of them has.

### The permissible-stress side, none of which is built

`σ_F < σ_FP` is ISO's check, and this tool computes the left side its own way
and compares it against a material allowable from its own library. **The whole
right-hand side of 6336-3 is unused**, and it is recorded here as a structure
rather than a set of formulae because what makes it unusable is its inputs, not
its arithmetic. Clause 5.4.3:

```text
σ_FP = σ_Flim · Y_ST · Y_NT / S_Fmin · Y_δrelT · Y_RrelT · Y_X
```

| | | why it is not here |
|---|---|---|
| `σ_Flim` | nominal bending stress number, from reference test gears (ISO 6336-5) | The values this tool would need do not exist for six of its eight materials; the library ships datasheet figures with provenance instead |
| `Y_ST` | 2.0, the reference test gears' own stress correction (7.4) | Meaningful only against a `σ_Flim` quoted with it |
| `Y_NT` | life factor, S-N interpolation (Clause 12) | Needs the two S-N points `σ_Flim` carries |
| `Y_δrelT` | relative notch sensitivity (Clause 13) | The ratio of this gear's notch sensitivity to the test gear's — a *relative* factor with no meaning away from that test gear |
| `Y_RrelT` | relative surface factor (Clause 14) | Likewise, and needs a fillet roughness this tool does not model |
| `Y_X` | size factor (Clause 15) | Likewise relative |
| `S_Fmin` | minimum safety factor | A contract between manufacturer and customer (Clause 4), not a property of a gear |

Four of the seven are **relative** factors, defined as ratios against a standard
reference test gear. They are not portable: adopting one without `σ_Flim` and
`Y_ST` is taking a ratio and dropping its denominator. That is the same error as
[the helix factors](rationale.md#the-helix-factors-are-a-pair-and-this-tool-can-take-neither),
and it is why the permissible side is all-or-nothing.

For limited life, 5.4.4.3 gives
`σ_FP = σ_FP,ref · (3·10⁶/N_L)^exp` with
`exp = 0.4037·log(σ_FP,stat/σ_FP,ref)` for through-hardened steels and
`0.2876·log(...)` for surface-hardened, nitrided and cast irons — recorded
because it is the one piece of that clause that needs only two stress numbers
and a cycle count, all of which this tool has.

### Annex B, `Y_M`, and the reversed-bending fraction this tool does use

This tool derates a fully reversed root to **0.7** of its one-directional
allowable, a Goodman/Haigh statement rather than a rating factor. ISO 6336-5
uses the same 0.7. Annex B offers a finer method and is recorded in case it is
ever wanted:

```text
Y_M = 1 / (1 − R · (1−M)/(1+M))        R = −1.2 for equal loads both flanks
                                       R = −1.2 · F_Rlow/F_Rhigh otherwise
```

`M` is the mean-stress sensitivity, from Annex B's Table B.1: case hardened
`0.8 − 0.15·Y_S` (endurance) / 0.7 (static); case hardened and shot peened
0.4 / 0.6; nitrided 0.3 / 0.3; induction or flame hardened 0.4 / 0.6; not
surface hardened 0.3 / 0.5; cast steel 0.4 / 0.6. It applies to the
**permissible** stress, not the applied one — so it belongs to the section
above, and inherits its problem. Note also that its own opening line says the
annex "does not conform with ISO 6336-5", which covers reverse loading with the
flat 0.7 this tool already uses.

### Clause 6.2's closed-form `Y_F`, which this tool replaces rather than declines

Method B computes the form factor from a closed form — Formulae (26)–(32) for a
hob-cut external gear, and (33)–(61) for a shaper-cut external or internal one,
including a transcendental solve for `θ` said to converge in about five
iterations and a Newton iteration for the auxiliary angle `ψ`. None of it is
here, and not because it was judged: this tool **generates the profile** and
measures `s_Fn`, `h_Fe` and `ρ` off it, which is what lets undercut, profile
shift and thickness modification flow through without special cases. The closed
form is what one writes when one cannot do that. It is named here only so a
future reader knows it was read and set aside rather than missed.

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
  claim was tested once, the other way: the worm kind was deleted as a type
  and became a preset over the pair, and no answer moved.
- **The transverse rack round at a steep helix**, in the ledger above. The
  honest transverse tool is a normal round's elliptical section, which neither
  circle is; until then a worm's fillet is the cap's and its interference
  verdict is conservative by the difference.
- **The two contacts' seams**, also above — the flank-load convention and the
  single-pair point — are each standard in their own model. Closing either
  would mean choosing one convention for both, which is a decision to make on
  purpose rather than on the way to something else.
