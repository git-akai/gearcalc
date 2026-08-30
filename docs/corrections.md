# Corrections

Every entry here was a claim this project believed and acted on, and every one
turned out to be wrong. They are recorded rather than quietly edited away,
because **the pattern is more useful than any single entry**:

> The errors that survived longest were the ones that looked reasonable and were
> never checked against something independent.

Two entries are corrections *of corrections* — the second diagnosis was right
about the symptom and wrong about the cause, and both times the wrong cause
looked like caution.

What the tool computes is [`reference.md`](reference.md); why each model was
chosen is [`rationale.md`](rationale.md); what is built is
[`state.md`](state.md).

---

## The patterns, and what now enforces them

Sorted by how often they recur. Each is followed by what makes it
unrepresentable rather than merely known — an entry earns its full length below
only while the fault it records can still be written.

### One idea written down twice

**A duplicated formula is a place where two answers can differ, and the copy
nothing exercises is the one that is wrong.** The internal relative curvature was
wrong two ways at once and unreachable; the gear tab's cutter default drifted
between Rust and TypeScript; the fillet's curvature was differentiated
numerically in two files; `MeshKind::sign` was written out again three times.

*Now:* the mesh kind is a signed number rather than a branch; defaults, bounds
and strings cross from Rust; the TypeScript wire types are generated and CI
regenerates them and requires no diff; the trochoid curvature is one closed-form
function shared by both cutters; and the tool an external gear is cut by is a
`Rack` of two **lengths**, because a coefficient converted on the way in and out
picked up a stray `1/cos β` every time it made the round trip.

### A check built from the thing under test measures nothing

The ring cut simulation derived the cutter's tooth the same way the model did, so
it agreed to 2.7 µm on a ring whose cutter was 0.44 mm out of place. The
mesh-phase coefficient's acceptance gate was "reproduce the backlash law", and
the answer turned out to *be* the backlash law halved.

*Ask what the check would say in the failing case — and if you cannot make it
fail, it is not a check.* Before trusting a new gate, run it against the broken
code.

### An axis nobody turns is an axis nobody tests

Every continuity check on the eccentric gear ran at λ = 0, which is the one value
at which the teeth *are* evenly seated. Then λ was swept on a lone gear — and
every `centre_profile` test, the place λ meets a *mate*, still left it at its
default. Three test grids between them left six of eleven inputs at their
defaults, and the module was 1.0 in every profile law the crate had.

*Ask of a control not "is it tested?" but "is it turned in each context it
reaches?"*

*Now:* one shared grid with every axis nameable, and module homogeneity asserted
as a standing law.

### A gate on a ratio cannot see a scale error

`moment_per_force` returned a power where a torque was wanted, putting a factor
`z₂/z₁` — 40× on a worm — into every flank load. All four efficiency tests stayed
green, because efficiency is a ratio and the factor cancelled.

*If every gate on a quantity divides it by something, none of them constrains its
size.*

### Refusing to answer can be the discontinuity

`Y_S` was declined on a flank tangency to avoid a 17 % jump. But a number
becoming no number is also a jump, and nothing physical happens at 151 teeth. The
admissible shift ceiling stepped by 62 % as the eccentricity left zero, because
two arms of one branch had two different tool models.

*A value becoming a different value across a boundary the physics does not have
is the defect, whichever direction it goes.*

### A sentence cannot do a symbol's job

Four places matched on a message's *text* to decide something. Every one would
have gone quietly false on a rewording, and all of them on a translation.

*Now:* `Note::is(key)`, checked by the compiler, and errors carry a `Note` too.

### An absent thing is not a zero-length thing

The missing ring fillet was stored as `s_j = s_root = 0`, so everything
downstream asked for a curve that was not there — and the failure was silent and
*plausible*: a `NaN` arc length collapsed a 600-point outline to seven, which
draws as a polygon that looks deliberate.

*When something can be absent, say so in the type, or every consumer has to
remember.*

### Assembling a thing out of things gives each piece its own everything

An eccentric gear is built tooth by tooth from `Gear`s, and `Tooth::new` takes
gear-level decisions. Sorted by *whose property is this?*, the guards fall out
cleanly — but the sort was applied to two of the three, and the third came back a
week later wearing a different symptom. **The summary is one of the pieces** too:
`Gear::mean` kept the raw dedendum while the teeth were rebuilt deeper.

*When a construction is per-piece, list what is a property of the **whole** and
check each is actually shared.*

### An audit scoped by a construction reports silence as agreement

The crossed-axis audit asked "where is the contact path missing?" and found five
branches. Backlash does not use a contact path, so nothing in the sweep pointed
at it — and it was the branch actually carrying a bug.

*Scope a sweep by the **output** instead: every number a stage reports, and where
each one comes from.*

### Deriving a law to explain a refusal is how you get two bugs

A close tooth-count internal pair genuinely cannot absorb much eccentricity.
Inventing a plausible reading of λ to make the refusal go away kept the refusal
*and* broke the working case.

*When a model refuses, first check whether it is right to.*

### Group the cancellation first

`(seat + ψ) − ideal` against `(seat − ideal) + ψ`; a Fourier coefficient
projected onto raw samples rather than onto their deviation from the mean; a mean
summed outright rather than anchored on the first sample. Each cost a
*concentric* gear a nonzero answer where the true value is exactly zero.

*When a quantity must vanish in a degenerate case, arrange the arithmetic so the
terms cancel before they meet anything large, and assert the exact zero — the
tolerance is what hides it.*

### A disclosure can expire, and a stale one is a false statement

"This efficiency omits profile sliding" became wrong of the new model and then
fired on the *more* exact number. "The wheel is throated" was true of worm drives
and false of this crate, and was used to withhold a number the cylindrical model
produces perfectly well.

*Inherited justifications need re-reading against the code each time they are
leaned on.*

### Units are a diagnosis

A radial-assembly threshold came out scaling with ring *size* when a
tooth-passing condition must depend mainly on the tooth *difference*. A result
whose units are wrong is wrong however plausible.

---

## The log

| Where | What was wrong | How it surfaced |
|---|---|---|
| [4.3](reference.md#automatic-values) | Working depth modelled as a constraint on the form radius | Would not reproduce the classical z=17 result; the correct reading substitutes `h_w` for `h_f` in the cutter-depth term |
| [4.5](reference.md#path-of-contact-and-contact-ratio) | Approach and recess lengths each subtracted `a_w sin α_w` | Both came out **negative**; each must subtract its own gear's `r′ sin α_w`, and only their sum uses `a_w` |
| [4.6.1](reference.md#tolerance-tables) | "The standard scale's grade 4 is tighter than the fine scale's grade 0" | False — 7 against 6.3. The argument it supported (non-monotonic merged ladder) survives and is now stated from the transcribed data |
| [4.6.1](reference.md#tolerance-tables) | JGMA diameter bands stored half-open as printed | Left a **gap**: a gear of exactly 12.00 mm got no tolerance at all |
| [4.7](reference.md#contact-stress) | 30° tangent treated as the model rather than an approximation | Its tangents miss the load point by 11.8% at z=9; the Lewis parabola is the construction the cantilever model implies |
| [4.7](reference.md#contact-stress) | Parabola tangency searched on the fillet only | No solution at all above z≈150 — on large teeth it touches the **flank** |
| [4.7](reference.md#contact-stress) | ISO `Y_S` applied to a flank tangency, using the **involute's** curvature as `ρ_F` | **17% discontinuity** at z=150→151 while `Y_F` moved 0.03%. `ρ_F` is a notch radius and an involute is not a notch: the number jumps 0.61 → 22.9 mm across one tooth |
| [4.7](reference.md#contact-stress) | ...then refusing the correction there instead | **Also a discontinuity** — a number becoming no number — and the diagnosis had stopped one step short. Nothing physical happens at 151 teeth, so nothing may step. The notch is the *fillet*, and when the critical section climbs above it the nearest fillet point is the junction; read there, `ρ_F` runs smoothly through the seam (0.6095, 0.6081, 0.6067) and `q_s` with it (1.805, 1.808, 1.808). **This mattered far more than the external case suggested**: a ring's tooth is short and wide, so its section sits on the flank for *every* count from 40 up — the old rule withheld ring bending across most of the design space, appearing and disappearing as the contact ratio moved the load point |
| [4.7](reference.md#contact-stress) | "Rack-generated fillets keep `q_s` in range" | False at large z — 10.3 at z=300 with a sharp cutter |
| [4.5, 4.7](reference.md#contact-stress) | "A helical mesh slides along its contact line, and the efficiency formula under-states that loss" | **No such component exists for parallel axes.** Building the sliding as a vector and resolving it on the contact line returned zero at every helix angle to 1e-14 of the pitch line velocity. Both surface velocities are `ω ẑ × r`, so the sliding is transverse, while the contact line is not — and the two are orthogonal by construction. The closed form was already exact; two documents described it as conservative |
| [4.5](reference.md#path-of-contact-and-contact-ratio) | Mesh efficiency without the `/ε_α` | Implicitly let every engaged pair carry the full load, so the mesh transmitted `ε_α F_n` and the loss came out too large by exactly the contact ratio. Caught by a numerical average of the instantaneous loss |
| [4.7](reference.md#contact-stress) | Hertz checked at "the inner point of single-pair contact (usually the pinion's worst case)" | Label-dependent: the relative-radius peak moves to the other side when gear 1 is the wheel, so one physical mesh gave two answers. Both boundaries are now checked |
| [4.7](reference.md#contact-stress) | `Load` stored a force in a field named `normal_force` | The value was the **transverse** `F_bt = T/r_b` while the name asserted the normal plane. Numerically right for spur, but the name would have survived a refactor its meaning did not. Now stores torque, with each projection named at its point of use |
| [4.7](reference.md#contact-stress) | Helical contact used the transverse force, face width and curvature throughout | Three separate `cos β_b` factors were missing. They nearly cancel — the net is `√(cos β_b)` — so the error was small but the model was wrong in three places at once rather than right |
| [4.7](reference.md#contact-stress) | Helical bending measured `Y_F` on the transverse section and divided by `m_n` | Mixes planes; under-predicts by about `cos β` (6 % at 20°, 13 % at 30°). Now measured on the ISO virtual spur gear `z_n = z/cos³β` |
| [4.5.1](reference.md#crossed-axes) | "Four lines are tangent to both base cylinders; **only one** passes through the pitch point" | Eight, and **two** pass — the tooth's two flanks, drive and coast. The count came from enumerating one normal direction and forgetting the other, and it was hidden because the two are mirror images, so every *length* the path yields is the same on either. It surfaced only when the scratch script was promoted to `tools/crossed_path.py` and the tangency check was added: floating-point noise was deciding which of the two flanks was returned, and the flank normals then came out 83° from it |
| [4.5.1](reference.md#crossed-axes) | The flank tangency check compared the crate's normal against one flank of each member | A tooth has two, and in mesh the driving flank of one member meets the **facing** flank of the other. Pairing them wrongly puts the answer out by exactly `2α_n` — 40° here — which looks like a broken construction and is a broken comparison. Both faults were in the *checks*, not in the crate, and neither could have been found while the scripts lived in a temporary directory |
| [4.4](reference.md#centre-distance-and-backlash) | A crossed pair's centre-distance backlash as `Δa · sin α_n` | **Half the answer.** A separation opens *both* flanks and lost motion is the sum of the gaps a member can travel across, so the term is `2 Δa sin α_n`. It reached the UI as a **50 % step** in backlash the moment the shaft angle left zero — the parallel stage had the exact involute law, the crossed stage had half of its first-order form, and moving `Σ` from 0 to anything crossed between them |
| [4.4](reference.md#centre-distance-and-backlash) | ...and the reason it survived | The module derivation wrote both displacements as one gap along `n̂` — `j_n = j_axial sin β_b1 + Δa sin α_n` — which reads as a single tidy projection and is two different physical quantities. A worm's axial float is a **rigid-body slide** (opens one flank as far as it closes the other: counted once); a centre-distance error is a **separation** (opens both: counted twice). One coefficient for both hid the factor. The test that existed, `axial_slack_reproduces_the_two_handbook_relations`, deliberately zeroed the clearance to isolate the axial term, so it constrained the half that was right |
| [4.5.1](reference.md#crossed-axes) | "Unifying backlash is not obviously a gain... expressing the parallel case in the crossed form would trade exactness for uniformity" | True of the **numerators** and false of everything else, and acting on it as though it settled the question is what left the error unguarded for a milestone. The *conversion* from gap to angle was one law with two homes, and the second home had no check against the first. Every crossed backlash test held the crossed model against itself; the parallel model — which has the exact law and its own tests — was never asked |
| [4.4](reference.md#centre-distance-and-backlash) | Every contact quantity rated at the **zero-backlash** centre distance | A pair runs at that plus its assembly clearance, and the path, the operating pressure angle, the curvature and every stress belong to where it runs. `ε` was over by 1.2 % and bending under by the same, on every parallel stage ever solved. The clearance was being treated as a tolerance to ignore when it is the reason the stage reports backlash at all |
| [4.5.1](reference.md#crossed-axes) | `path_of_contact` returning `None` at any centre distance but the nominal | Not a refusal, a **missing branch rule**: it identified the mesh by asking which tangent line passes through the pitch point, and none does once the centres move. Which flanks face each other is not a function of centre distance, so the branch is settled at the nominal distance and carried |
| [4.5.1](reference.md#crossed-axes) | A crossed pair's face width clipped symmetrically about the path's own origin | The face is centred on its **gear**. Those coincide only at the zero-backlash distance; away from it the contact has slid along the shafts, and at `Σ = 0.5°` with 20 µm of clearance a 12 mm face has `ε = 0.860` where the model reported 1.664 |
| [11.5](rationale.md#no-english-in-gear-core-and-no-engineering-in-the-catalogue) | Notes as `Vec<String>`, and code matching on their text | Four places branched on a *sentence* — the planetary solve on `"tip radius raised"`, the range test on `"cutter depth"`, two ring tests on `"base circle"` — every one of which would go quietly false on a rewording and all of them on a translation. A note that anything acts on needs an identity, not prose |
| [11.5](rationale.md#no-english-in-gear-core-and-no-engineering-in-the-catalogue) | The string catalogue read from a plain module variable in the front end | Not reactive, so the **sidebar** — which draws before the core finishes loading — kept the fallback and rendered `ui.sidebar_gears` where "Gears" belonged. Every panel looked right because every panel renders later, which is exactly how a bug survives a look at the screen |
| [4.3](reference.md#automatic-values) | `working_depth` defaulting to a fixed 1 module | The control exists to expose the classical rule's hidden assumption, and it shipped defaulting *to* that assumption. It follows the gear's own dedendum now, which is the depth the `undercut` flag answers about — the two agree by construction instead of by coincidence. Every automatic shift previously clamped at zero moved, and nothing in the suite noticed |
| [4.5.1](reference.md#crossed-axes) | "What a crossed stage lacks is the *backlash from thinning* one alone, which would need the normal-plane play derived for crossed axes" | Recorded as a gap; it is an **answer**. The play is derived ([centre distance](reference.md#centre-distance-and-backlash)), and `k₁ + k₂ = 2` means thickness is only ever moved between two teeth and never removed from the pair, so what it contributes is exactly zero. A missing derivation and a derivation that returns zero read the same in a document and are not the same thing |
| [11.4](rationale.md#two-friction-coefficients-because-there-are-two-questions) | One friction coefficient per mesh | Breaking away and running are different events with different coefficients. Every efficiency in the model was the sliding one, so a worm that cannot be started was reported as back-driving at 54 % — the efficiency of a motion that does not happen |
| [11.4](rationale.md#two-friction-coefficients-because-there-are-two-questions) | The self-locking note quoting the sliding coefficient | It named the input a reader would go and change to no effect. Whether the drive locks is decided by the **static** coefficient, and the note now says which number it means |
| [4.10](reference.md#angularly-varying-profile-shift) | Each tooth of an eccentric gear built as its own gear, tool and all | `Tooth::new` caps the cutter tip round per tooth, so the high side got a 0.2375-module tool and the low side 0.3800 — a different hob for each tooth. The fillet collapsed on the high teeth and the trochoid's extent jumped sixfold between neighbours. One gear, one tool: the tightest tooth's |
| [4.10](reference.md#angularly-varying-profile-shift) | ...and each tooth's root drawn at its own radius | A radial **step at every mid-space**, up to 0.13 mm, which no hob can leave. The root is the gear's, not the tooth's: it runs from the fillet junction to the envelope `r − m(h_f − x(θ))`, continuous at both ends |
| [4.10](reference.md#angularly-varying-profile-shift) | The root correction applied to the flat root alone | It has 0.005 rad to absorb 0.05 mm in, a dive of 9 mm/rad against the envelope's own 0.4 — a notch at the bottom of every tooth space. Spread across the fillet, which the same tool corner cuts at the same moving radius and which nothing constrains |
| [4.10](reference.md#angularly-varying-profile-shift) | ...and parametrised on **angle** | The flank is re-entrant below the base circle, so a flank point can sit at a larger angle than the fillet junction and take a displacement it must never have. Radius is the monotone invariant ([the generated profile](reference.md#the-generated-profile)) and is what measures position along the profile |
| [8.0](rationale.md#notes-must-not-move-the-controls) | A gear tab's type-specific inputs left set when the type changed | Switching an eccentric gear back to external left its shift amplitude in place, so the gear stayed eccentric with no control on screen to say so — and the eccentricity outputs keyed on that *value* rather than on the type, so they stayed too. Changing type now returns every field the new type does not use to its default, read from `FIELDS` rather than from a second list |
| [4.10](reference.md#angularly-varying-profile-shift) | Only the cutter *tip round* shared across the teeth | The **depth** is a tool setting too. `Tooth::new` raises the cutter depth when it would go non-positive, which pinned four teeth to one root radius while their neighbours followed the envelope — a flat spot and a corner on the high side at positive shift, and the low side at negative. Both settings are the tool's and are settled once; what is a fact about *one tooth* is reported instead |
| [4.10](reference.md#angularly-varying-profile-shift) | Each tooth drawn one **ideal** pitch wide | λ seats the teeth unevenly by construction — that is what it is for — so the space between two of them is a pitch plus the difference of their offsets. Every tooth drawn to the ideal width left a gap of 0.009 rad at λ = 1, and every continuity check written before it ran at λ = 0, the one value that hides it |
| [4.10](reference.md#angularly-varying-profile-shift) | The shared cutter depth applied to the teeth but not to `Gear::mean` | When a high-shift tooth forces the cutter deeper than the dedendum asked for, the teeth are rebuilt to it — but `mean`, which every scalar is quoted from and which `root_at` builds the root envelope on, kept the raw dedendum. The drawn root then stood `m·(depth − dedendum)` proud of the teeth and protruded past the fillets: **0.25 mm** on `α=25° z=23 x=0.2 Δx=1 addendum=0.8 dedendum=1.0`, plainly visible on the shallow high side. Every step/kink trend test passed — a uniform radial offset of the whole root is smooth. `mean` is rebuilt with the same tool now, in lockstep with the teeth so a concentric gear is still `Tooth::new` verbatim; the gate is `root_at(seat_k) == teeth[k].rf` at λ = 0, exact. *The object that summarises the pieces is a piece.* |
| [4.10, 8](reference.md#angularly-varying-profile-shift) | The wasm gear summary built from `Gear::new(params)`, ignoring the eccentricity and the shared-tool rebuild | Same root cause on a second surface: `fillet_radius` reported the 0.38-module rack where the shared tool is 0.05, `cutter_tip_width` used the raw dedendum, and span/over-pins quoted the mean tooth as if it were *the* measurement. `solve_gear` now builds `Gear::new(params)` and quotes `mean()` — one construction for both kinds, a concentric gear's `mean` being `Tooth::new` bit for bit. Span and over-pins are withheld for an eccentric gear (`Unavailable`, "varies around the revolution"); `undercut`/`severed` became "any tooth" (`per_tooth_clamps` names which); the panel hides the tip/root/thickness scalars for an eccentric gear and shows the `variation` ranges only |
| [4.10, 8](reference.md#angularly-varying-profile-shift) | `admissible_ranges` bounded the nominal tooth, not the swept interval | An eccentric gear's teeth are cut across `x̄ + Δx cos θ`, so `x̄ = 0.2, Δx = 1` puts a tooth at 1.2 — past the buildable 0.95 — while the hint said the shift was fine and the root-radius hint offered the 0.38 rack the high teeth cannot hold. `admissible_ranges` now closes the shift-dependent bounds onto the interval: `profile_shift` pulls in by each extreme tooth's own offset (its cutter-depth ceiling drops — the shared tool follows the shift up), and `addendum`/`dedendum`/`root_radius` are the tighter of their two extremes against the shared cutter depth. Gated against the generator as the concentric bounds are — just inside the window every tooth builds, just outside one clamps. A concentric gear is `Δx = 0` and unchanged to the bit |
| [4.10](reference.md#angularly-varying-profile-shift) | `centre_profile` reported `CentreDistanceTooSmall` where the cause is `inv α_w < 0` | Two different failures wore one name. `operating_geometry` returning `None` means no operating pressure angle exists — the tooth is too thick to sit in the mate's space at *any* centre distance — which is `OutsideInvoluteDomain`; `CentreDistanceTooSmall` is the base-circle limit and belongs to the sinusoid-fit step below it. On a close tooth-count **internal** mate the first fires readily, because the shift term carries `1/Σz` and `Σz` is the tooth-count *difference* there, so a modest amplitude is amplified into it. The message now names the eccentricity and says what relieves it |
| [4.10](reference.md#angularly-varying-profile-shift) | ...and the "fix" for it invented a law, which broke the feature outright | Diagnosing the above, `x_eff = x̄ + (1 − λ)(x_k − x̄)` was introduced on the reasoning that a corrected drive flank sits where the mean tooth's would, so a λ ≈ 1 drive operates against a smaller shift interval. **λ moves a tooth rigidly** — both flanks by one angle — so it decides when a tooth arrives, not how thick it is, and zero backlash is set by the thickness. The consequence was not subtle: at λ = 1 every tooth collapsed onto the mean and the commanded centre distance reported **zero throw at every amplitude**, which is the whole output of the feature. Reverted; λ reaches nothing here and that is now gated *exactly*, over both mesh kinds and four λ. The gap that let it through: every test that turned λ did so on a lone gear, and every test with a mate attached left λ at its default — **a control needs a case that turns it in each context it reaches**, not just somewhere |
| [4.10](reference.md#angularly-varying-profile-shift) | `centre_profile` answered an internal pair whose "ring" had fewer teeth than the pinion | `Mesh::new` refuses it as `RingTooSmall`; this arrived only as a signed sum of the wrong sign and the arithmetic answered it. Refused by name now |
| [6](rationale.md#material-data-ships-estimates) | Two-point Basquin S-N law per material | The data does not exist — no polyamide grade publishes any fatigue figure, and POM's is a printed graph. Replaced by peak and cyclic allowables, [material data](rationale.md#material-data-ships-estimates-deliberately) |
| [6](rationale.md#material-data-ships-estimates) | `yield_strength` as the single strength field | Glass-filled grades have **no yield point**; their datasheets report stress at break. Renamed to an allowable, with `ultimate_measure` recording which quantity it is |
| [6](rationale.md#material-data-ships-estimates) | "1215 Hardened Steel" assumed a valid entry | 1215 is ~0.09 %C and cannot be through-harden; only carburised, giving a hard case over a soft core that one scalar cannot represent. Both 1215 entries dropped |
| [6](rationale.md#material-data-ships-estimates) | Delrin 570 assumed a reasonable "POM GF20" | It is glass *filled*, not *reinforced* — **25 % weaker** than unfilled Delrin. Entry dropped; only a glass *coupled* grade would belong |
| [6](rationale.md#material-data-ships-estimates) | PA6/PA66 stiffness "roughly halves" when conditioned | Understated: unfilled PA6 modulus falls 3000 → 1000 MPa, a factor of **three** |
| [4.10](reference.md#angularly-varying-profile-shift) | Read as an axial taper (beveloid) | It is an *angular* variation; the beveloid treatment was withdrawn entirely |
| [4.10](reference.md#angularly-varying-profile-shift) | "No changes to the generator" (beveloid reading) | Did not survive the correction above |
| — | Involute inversion by series seed + Newton | **Diverges above ~60°**, inside the allowed pressure-angle range; needs safeguarding |
| [4.7, 4.11](reference.md#internal-gears) | Internal relative curvature written as its own branch beside the external one | **Wrong in two independent ways at once**, neither reachable. `r_b2` was scaled by `z₁ + z₂` where an internal pair needs `z₂ − z₁` (exactly 0.5× on a 17/51 pair), and `ρ₂ = r_b2 tan α_w − ξ` should be `+ ξ`. Together: −50 % at the pitch point of a 17/51, and a **negative** relative curvature on a 25/41 — which `contact_stress` would have reported as "no contact" for an ordinary internal mesh. Both were dead code only because `ContactPath::new` admitted no internal mesh, and both would have gone live with milestone 9. Fixed by making the sign a *value* rather than a branch: gear 2's tooth count, shift and radii are negative for a ring, and the external expressions then serve both kinds unchanged |
| [4.7](reference.md#contact-stress) | The two members' base radii reached by different routes | Gear 1's came from its own reference geometry, gear 2's through `a_w` and `α_w` — so `r_b1 + r_b2 = a_w cos α_w` held only to an ulp, and gear 2's carried the involute inversion's residual for no reason. A base radius is `m_t z / 2 · cos α_t` and owes nothing to the centre distance; both now take that route, and the identity is exact |
| [4.11](reference.md#internal-gears) | `thickness_mod` applied to a ring's **tooth**, as it is to an external gear's | Backwards. The ring's *space* is what is generated like a tooth — it is where the pinion's tooth goes — and `Mesh::new`'s internal relation flips gear 2's `x` and `x_s` together, which is consistent only with the space reading. Measured against tooth thicknesses at the operating circles, the space reading gives exactly zero backlash at every k while the tooth reading is **0.63 mm** out at k = 1.2. Consequence: a larger k or x makes a ring's tooth *thinner*, and the internal pair invariant is `k₁ = k₂` where an external one needs `k₁ + k₂ = 2` |
| [4.11](reference.md#internal-gears) | `Ring::new` ignored `profile_shift` entirely | Not a wrong formula but a missing one, and it had a visible face: the gear tab sends `profile_shift` for an internal gear, so the box was there and moved nothing. Every layer was individually happy, which is why only an end-to-end check found it |
| [4.11](reference.md#internal-gears) | A shifted ring cut by a cutter at **reference** centres | A shaper cannot be displaced the way a rack can. A rack's pitch line is a machine setting, so shifting it leaves the rolling alone; two pinions have their ratio fixed by their tooth counts, so the pitch point is wherever the centre distance puts it and the rolling circles move with it. The cutter was up to **0.44 mm** out of place at x = 0.5. One factor `a / a_ref` carries all of it, and is exactly 1 at zero shift |
| [4.11](reference.md#internal-gears) | **The cut simulation derived the cutter's tooth from the ring's** | So it shared the model's assumption and could not see the fault above: it reported 2.7 µm on a ring whose cutter was 0.44 mm out of place, unchanged from the unshifted case. The this log trap in its purest form — *a check built from the thing under test measures nothing.* The cutter's tooth now comes from the cutter, and placing it at reference centres makes the gate fail by 13–66× its noise floor |
| [4.11](reference.md#internal-gears) | A ring's root radius from `r + m(dedendum + x)` | That is the exact relation linearised. A ring's root is wherever its cutter's tip *reaches*, `a_cut + r_tip`; the two differ by 17 µm at x = 0.25 and 57 µm at x = 0.5, both well above the 3.6 µm the cut simulation resolves. So a ring has **no dedendum input** — it is the cutter's addendum seen from the other side, and having both invites them to disagree. Its root-radius coefficient goes the same way: the fillet round is the cutter's own |
| [4.11](reference.md#internal-gears) | `Ring::new` accepted a shaper **at least as large as the ring** | It arrives as a negative centre distance rather than an obvious error, so nothing objected: a 43-tooth ring "cut" by a 50-tooth shaper reported a root radius of 29.75 mm against a pitch radius of 21.5, and the only complaint was about the tip corner — a true statement of the wrong problem. `Mesh::new` and `mesh_with` both refuse that pairing; the constructor did not. Now clamped and named |
| [4.11](reference.md#internal-gears) | A ring's tooth was never checked for **running out of thickness** | A ring's tooth narrows *inward*, so its tip is its thinnest section — and where `ψ_b < 0`, which needs `π/2z < inv α_t` (about 105 teeth at 20°), it reaches zero *above* the base circle. Unclamped that is not a thin tooth but a **crossed** one: a 150-tooth ring at a 3-module addendum came out at −0.211 mm of tip thickness, and its outline is a self-intersecting polygon bound for a DXF. The limit is closed form through `inv⁻¹` — `inv α = −ψ_b` — and mirrors `Gear`'s pointed-tooth clamp rather than being a new kind of guard. Invisible on the tooth counts one would try first |
| [4.8](reference.md#planetary-sets) | `mesh_with` asserted standard centres for an internal pair | Fine as far as it went, but a planetary set needs the shifted case and the two would then have been separate constructions. Now both come from `mesh::operating_geometry`, the same relation the external mesh uses, and a standard pair is its value at zero shift — reached rather than asserted, and equal to `r_ring − r_pinion` to 1e-12 |
| [4.8](reference.md#planetary-sets) | The planet-shift bracket evaluated **on** the involute domain's boundary | The endpoints are where `inv α_w = 0` exactly, and whether that arithmetic lands on zero or on −1e-17 depends on the tooth counts. On a 24/16 set with four planets the upper endpoint fell a hair outside a domain it was meant to sit on, so `z_ring = 57` was refused although its root is at +0.2485, comfortably inside — **a hole in a run that monotonicity says is contiguous**. Fixed by halving each endpoint toward a point known to be interior, which finds the last representable point in the domain and needs no tolerance: it stops when the answer becomes a number |
| [4.8](reference.md#planetary-sets) | Contiguity checked on one configuration | The test swept 17/17 only, where the rounding above happens to fall the other way, so it passed while a neighbouring set had a gap. The property is about *all* sets; it is now asserted over eight, and as a **gap is a bug by construction** it needs no knowledge of the answers |
| [4.5, 4.7](reference.md#contact-stress) | `ContactPath` external-only, with the internal case listed as "contact needs no new code" | Half right. `hertz::relative_curvatures` and `contact_stress`'s curvature genuinely needed nothing, but the **path** did: it is the input to the contact ratio, the efficiency, both single-pair stress boundaries and the bending load point, and it refused an internal mesh outright. It is now one pair of expressions for both kinds, with the tangent length carrying the sign of its own base radius — so a ring's tip sits at a *smaller* tangent length than the pitch point and its approach comes out `\ |
| [4.5](reference.md#path-of-contact-and-contact-ratio) | `ContactPath::new` took gear 2 as a whole `Gear` | It only ever read `r_a2`, and a ring's tip radius cannot come from a `Gear` — it is inside the pitch circle and set by the cutter. Now takes the tip radius, which is both less than it asked for before and enough for the internal case |
| [4.5](reference.md#path-of-contact-and-contact-ratio) | `sliding_at` placed gear 2 at `+a_w`, unreachable *because* no internal path existed | The comment saying so went stale the moment the path was lifted, which is the hazard of documenting an assumption as safe on the strength of something elsewhere refusing. A ring encloses its pinion rather than sitting beside it, so its axis is at `−a_w`; from the signed operating radii `r′₁ + r′₂` gives both, and the pitch point stays put |
| [4.7](reference.md#contact-stress) | "A ring's bending would need its own critical-section construction" | It needs its own *curves*, not its own construction. The inscribed parabola's tangency condition `X·Y′ + 2X′(y_v − Y) = 0` is **odd in y**, so negating `y`, `Y′` and `y_v` together leaves its zero set untouched — and the moment arm is a difference, so it flips with the frame and comes out positive either way. One flip is the whole of what an inward-pointing tooth needs, and external stays a value of the same search rather than a sibling of it |
| [4.7](reference.md#contact-stress) | Figure 8's "the two factors approach each other" taken as a check | It is a property of the paper's setup, not of ours: TM-107012 cuts its external gear with the same 20-tooth shaper as its ring, while this crate's external gear is rack-cut. With the shaper held at 20 teeth the ring's fillet never tends to the rack's, so the gap closes only to z ≈ 150 and then widens. The real convergence is with the shaper growing too, where both teeth become the same rack tooth |
| [4.7](reference.md#contact-stress) | Helical rings left unrated, on the grounds that a virtual spur *ring* did not exist | A gap, not a decision — the stated intent is feature parity with spur gears. It exists now: `z_n = z/cos³β` at the normal module, **with the cutter virtualised the same way**, which leaves `z_c/z_r` unchanged so the virtual pair still rolls together and the cut stays conjugate. Needed a `Ring` buildable at a fractional tooth count, mirroring `Gear::build_with_z`, and a `CutParams` that takes the cutter's *radius* rather than a whole-number tooth count |
| [4.9](reference.md#trains) | A planetary stage assumed to fit the train's two-member shape | It has three shafts, and its speeds come from its own kinematics rather than from a ratio applied to the previous stage. `solve_any` therefore needs a **speed** as well as a torque: which shaft is held is a kinematic question, and the efficiency depends on it. The train now carries the speed it has reached alongside the torque it has reached |
| [4.11, 3](rationale.md#no-engineering-calculation-in-typescript) | **The gear tab's cutter default was written down in TypeScript as well as in Rust, and the two drifted.** | `defaultCutter()` carried `tip_round = 0.38` — the *rack's* figure — where `Cutter::default()` has held 0.2 all along, with a comment saying that a 20-tooth shaper's tip is 0.377 modules wide and cannot hold two 0.38 rounds. So every ring the UI built was cut by a tool that generates no fillet, and the viewport drew a sharp-rooted polygon with straight flanks. Nothing in Rust was wrong; nothing in Rust could see it either, because the wrong number lived only on the side that has no tests. **The fix is not the number.** Every default a fresh tab starts at is now served across the boundary by `gear_wasm::defaults`, so there is one home for them and the class of drift is closed rather than this instance of it |
| [4.11](reference.md#internal-gears) | A ring with no fillet was represented as one whose fillet has **zero length** | `s_j = s_root = 0` and a `Trochoid` section still in the list. Sampling it measured an arc length of `NaN`, which made the total `NaN`, every share `NaN`, and `(NaN) as usize` **zero** — so every section fell back to its minimum point count and a 600-point outline came back with **seven**: an involute drawn as two straight chords. The absent fillet is now `Option<Fillet>` and cannot be asked for as a curve, the root arc starts wherever the section before it finished, and the allocator drops a section of zero or non-finite length instead of letting one poison the rest |
| [4.11](reference.md#internal-gears) | The boundary test that was supposed to prove "an outline the viewport can draw" | Was built on the same 0.38 cutter, and asserted `outline.len() > 200` — which sixty teeth of seven-point rubbish clears comfortably. *A check built from the thing under test measures nothing*, in its second form: the fixture was the defect. It now asserts points **per tooth** against the number requested, and the standing gate in `geometry_laws.rs` states the two properties the drawing must have — that the points arrive at the density asked for, and that each flank chord stands at `√(r² − r_b²)` from the centre, which is what makes the curve an involute of its own base circle. Run against the pre-fix code, that gate fails with "asked for 600 points a tooth and got 13.0" |
| [4.5.1](reference.md#crossed-axes) | "A worm stage reports no contact ratio — **the zone of action for a throated wheel** is not derived" | The phrase was true of worm *drives* and false of this crate: [crossed axes](reference.md#crossed-axes) takes both flanks as involute helicoids on **cylinders**, and nothing here throats anything. It had been written when no zone of action was derived for any crossed geometry — true then for a simpler reason — and was later leaned on to withhold the one number the cylindrical construction produces, while every other number in the same result came from that model unquestioned. A worm reports it now, as a floor, with its assumed tooth height named |
| [4.5.1](reference.md#crossed-axes) | "The contact path unblocks crossed-axis bending" — the audit's own prediction | It gives the load's position along the *profile*. `σ_F = F_t/(b·m)·Y_F·Y_S` is a cantilever loaded across its whole *face*, and a crossed pair's load is a point. Two missing ingredients looked like one until the formula was read; choosing an effective width is a convention that multiplies a stress, so [contact stress](reference.md#contact-stress) refuses it and bending stays out with a reason rather than a gap |
| [4.5.1](reference.md#crossed-axes) | A **disclosure that outlived its premise**, and then accused the better number | While the crossed efficiency omitted the profile sliding, a law disclosed it: crossing shafts can only add sliding, so beating the same teeth run parallel meant sliding had gone missing. Once the friction balance counted that sliding, the comparison inverted — the *parallel closed form* is the approximate one, first order in `μ` where the balance is exact — and at Σ ≤ 0.5° the check fired on a figure that was a hundredth of a point **better**. Removed. A warning is a claim, and it expires with the thing it warned about |
| [4.5.1](reference.md#crossed-axes) | The friction balance's flank load, out by `z₂/z₁` — **forty on a worm** | `moment_per_force` returned a *power* where a *torque* was wanted. All four gates on the balance were on the **efficiency**, which is a ratio, so the factor cancelled and every one stayed green; on a stress it does not, and a cube root turned forty into a plausible-looking 3.4×. Caught by the canary, not by the suite. A gate on the load itself now compares it with the classical closed form, and re-breaking the code deliberately confirms it is the only test that fails |
| [4.5.1](reference.md#crossed-axes) | "A narrow face pushes the rating 10–27 % past the pitch point" | Measured at 1.2 %, not 27 %. Losing load sharing does push the rating outward, but a face narrow enough to lose it has also cut the zone's ends off — and those ends were what made the path severe — so the two effects nearly cancel. The 10–27 % figure describes the zone's extremes, which are never rated because the load is shared there |
| [4.5.1](reference.md#crossed-axes) | "Frictionless is lossless **to the last bit**" | To a few ulps. The answer is a ratio of two moments reached by different cancellations, so bit-exactness was never on offer; the assertion said more than the arithmetic could and failed on the first geometry that exercised it |
| [4.5.2](reference.md#planetary-sets) | Reversing a planetary by handing its output shaft's torque back as the input | That torque is a **reaction**, opposite in sign to the shaft's speed, and a shaft that is now driving must have them agree. The wrong sign flips the rolling power, `η₀^w` takes the wrong branch, and the set reports an efficiency **above one** — 101.571 % in the running application. Every test drove forward with a positive torque and a positive speed, so none could see it; found by looking at the UI. Related: a shaft with `T ω ≤ 0` is absorbing power and is not an input at all, so naming it one is now refused rather than answered with `1/η₀` |
| [reference.md#angularly-varying-profile-shift](reference.md#angularly-varying-profile-shift) | The shared tip round settled as a **coefficient** | Divided by the normal module and handed back as a `root_radius`, which is re-multiplied by `m/cos β` — so every rebuild inflated a helical gear's round by `1/cos β`, 1.22× at 35°, and the teeth came out cut by different tools. Invisible because every eccentric test used a spur gear. The tool is two lengths now, so there is no coefficient to round-trip |
| [reference.md#internal-gears](reference.md#internal-gears) | A tip round too large for a shaper's tip **refused** the tool | The same guard caps it on an external gear. So one input gave a fillet outside and *no fillet at all* inside — a jump in kind at a point where nothing physical happens. Capped now, by the same rule and with the same note |
| [reference.md#internal-gears](reference.md#internal-gears) | ...and the first cap was not monotone | Backing off by the 5 % margin only once the ask crossed the boundary dropped the realised round *at* that instant. `min(asked, 0.95 × max)` rather than `if asked > max { 0.95 × max }` — a clamp has to be continuous in what it clamps |

| [reference.md#metrology](reference.md#metrology) | A span and a pin measurement taken as a difference of two **accumulated** seats | `t(k+1)/z - tk/z` is an ulp or two off `t/z`, which gave an evenly cut gear a measurement that varied around a revolution it is constant on — and reached the screen as "11.258 to 11.258 around the revolution". Grouped on the pitch and the `psi` differences, which vanish exactly when the teeth agree. The third time this module has met *group the cancellation first* |
| [reference.md#metrology](reference.md#metrology) | ...and the pin index wrapped **before** the difference was taken | An offset of one space became one of `z - 1` — the same angle, and not the same `cos`. `Gear` wraps for its own lookups, so the caller must not |

| [reference.md#trains](reference.md#trains) | **Every rating judged against the fatigue allowable, including the peak one** | The crate has held `ultimate_allowable` and `fatigue_allowable` since the material library existed, and rated everything against the second. So a load the train has to survive *once* was being asked to survive it forever — a wrong question, asked with a confident number attached, failing gears that are fine. The two are separate cases now, and what distinguishes them is exactly the torque and the allowable |
| [reference.md#trains](reference.md#trains) | A planet's reversed-bending derate written down **twice** | `width_for` sized the planet's face against the reversed allowable while `gear_result` reported its minimum against the plain fatigue figure, and a separate `min_face_width_reversed` field carried the other answer beside it. Two numbers for one question, and the width the stage actually used matched neither label. `gear_result` takes the allowable it rates against, so the width a member is given and the minimum it reports come from one function; the extra field is gone |
| [rationale.md#a-load-exists-only-where-it-is-reacted](rationale.md#a-load-exists-only-where-it-is-reacted) | A back-driving load first modelled as a **sign on the input torque** | It enters at the far end and is attenuated by *backward* efficiencies, which are not the forward ones and can be zero. Worse, the interesting case is the one where the number is absent: a train nothing can hold does not carry the load at all. Reported per stage as an `Option`, with the train saying which stage reacted it or that none did |

| [rationale.md#contact-stress-belongs-to-the-mesh-not-to-either-gear](rationale.md#contact-stress-belongs-to-the-mesh-not-to-either-gear) | Contact stress reported **per gear**, where it is a property of the pair | Not an arithmetic fault — the pair shares a patch, a normal force and an `E*`, so there is one pressure and there never was a second number. But printed twice it reads as a calculation skipped for the second gear, and reporting invited exactly that reading. Moved to the mesh, where the worm and planetary stages already had it; what stays per gear is the allowable, and therefore the width each asks for. Gated both ways: softening *either* material moves the one figure, and halving *one* gear's allowable quadruples that gear's `b_min` and touches nothing else |
| [reference.md#trains](reference.md#trains) | A gear's reported `torque` was the **peak load case**, not the forward torque | `StageTorques::at(Peak)` is `max(|forward|, |backward|)`, which is right for a *rating* and wrong for a label. Where a back-driving load was the larger, the "Torque" row and the "Back-driving torque" row beside it showed the same number — the only condition under which the second row is displayed at all, so it was the only thing it ever showed. The label is the forward torque now, and the peak rating still uses the worse direction |

---

## Process notes

- **A green local test run does not imply a green build.** Twice, the working
  tree held something a fresh checkout does not: a data file the source filter
  stripped, and generated bindings that are gitignored. `git add` before
  `nix build`; flakes only see tracked files.
- **Typechecking is not running unless you run it.** `cd web && npm run check`.
- **Run the app.** Two real defects in one session were found on screen and by
  nothing else. `--dump-dom` and `--screenshot` both run the JS, so they prove
  the app mounted and computed.
- **When a refactor must not change what a user sees, diff what a user sees.**
  The gate for pulling 185 strings into a catalogue was the rendered DOM, byte
  for byte. It caught a reactivity bug immediately, which no typecheck would
  have.
- **A scratch verification is not a verification once the directory is gone.**
  The crossed-axis derivation was checked in numpy before a line of Rust was
  written, and those scripts lived in a temporary directory while the design
  document cited their results as settled. Promoting them to `tools/` found two
  faults in the checks themselves.
