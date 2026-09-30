# Surface factor on the root's endurance: ISO, geometry only, or none (for the owner)

**The question.** Every root endurance in the library is a coupon's, and the rating applies no surface term.
Should it adopt ISO 6336-3's `Y_RrelT`, a roughness term with no material in it, or no term with a note?

**The owner's rules** (plan.md, 2026-10-03): (1) no new material properties; (2) ISO's factor only if its
error stays bounded as the basis moves to non-metals and other routes, else "no term, with a note";
(3) measured factors are validation, not inputs.

**Conventions.** **Deviation** = predicted − measured, in points of the reference specimen's allowable; **+ is unconservative**
(the model rates the root stronger than it is), **− conservative**. Tags: **[R]** read in full (primary text
or a faithful reproduction), **[A]** abstract only, **[C]** recalled and not checked, **[D]** derived or
computed in this workflow. Built from three research angles and two skeptic passes, corrections applied.
The third angle's text and its skeptic reached this synthesis cut off; its Concli, Ferri, Ho and Zorko 2021
figures were re-checked against the cached texts and are tagged [R] only where checked.

**ISO `Y_RrelT`, as read** (IACS UR M56.3.12 and IRS 3.12, which reproduce ISO 6336-3:2019; GEARpie's
DIN3990.py codes the same constants). The Rz range is 1–40 µm, and below Rz 1 each line takes a constant.

| Line | Covers | Formula, 1 ≤ Rz ≤ 40 | Below Rz 1 |
|---|---|---|---|
| L1 | case-hardened; through-hardened with σ_B ≥ 800 | 1.674 − 0.529(Rz+1)^0.1 | 1.120 |
| L2 | normalised, σ_B < 800 | 5.306 − 4.203(Rz+1)^0.01 | 1.070 |
| L3 | nitrided | 4.299 − 3.259(Rz+1)^0.0058 | 1.025 |

Void if a scratch or defect is deeper than 2·Rz. Relative to a test gear with a hobbed root, so against a
polished coupon it is the ratio `Y_R(Rz)/Y_R(<1)`: from polished to Rz 10 the allowable falls 10.6 % (L1),
6.4 % (L2) and 3.0 % (L3). Only iron and steel have lines.

**The geometry-only model** is the only roughness term rule 1 leaves: the elastic `K_t` of the root's own
profile, applied as `1/K_t`, with no material length [D]. It is computed by the first-order half-plane
solution (Gao), checked against `tools/fillet_bem.py`'s BEM: within 0.3 % on a sinusoid at a/λ ≤ 0.03, and
within 1–6 % on one random self-affine surface (Ra 1.6 µm) at filter cut-offs λ_s 25, 10 and 5 µm (BEM `K_t`
1.52, 1.92, 2.31). Scripts in scratchpad `surfkt/`, re-run in `synth/`. Arola–Ramulu's published formula
reads 1.08 to 4.68 on the same surface.

## 1. Material classes and routes: measured, ISO, geometry only

| # | Route (root against the endurance's specimen) | Measured effect on the endurance, % | ISO `Y_RrelT` predicts, % | ISO deviation | Geometry only predicts, % | Geometry-only deviation |
|---|---|---|---|---|---|---|
| 1 | Wrought steel, σ_u ≥ 800 (4340 Hardened): hobbed root Rz 10 against a polished coupon | −10.6, ISO's own fit (its data unread); FKM K_R diluted at K_f 2 gives −7.1 (σ_u 800) to −10.7 (1500) [R] | −10.6 | 0 by construction; 0 to −3.5 against FKM | −34 (λ_s 25 µm) to −65 (λ_s 2.5 µm) | **−23 to −54** |
| 2 | Wrought steel, σ_u < 800 (4340 annealed): the same | −6.4 (ISO fit); FKM −3.4 (400) to −6.3 (690) [R] | −6.4 | 0; 0 to −3 against FKM | −34 to −65 | **−28 to −59** |
| 3 | Two steels either side of σ_u 800, same root | about equal: FKM gives −7.0 at 790 and −7.1 at 800 | a 4.1-point step | **4.1 points between near-identical steels** | no material term, so no step | 0 |
| 4 | Carburised root ground (Rz 3), not peened after, against hobbed (Rz 10) | −10 to −25 [C]: the compressive layer is removed, and grinding burn | +6.5 | **+17 to +32, sign flipped** | a gain, sized by λ_s | sign flipped |
| 5 | Case-hardened root shot-peened against unpeened. Sz 20.9 → 16.6–28.2 µm (Ho 2021 [R]) | +10 to +40 on carburised roots [C]; +26 to +29 on MIM Ti (≈ +100 MPa, Ferri's ref. 5 [R, second-hand]) | +1.6 to −2.2 | **−8 to −42; wrong sign where peening roughens** | follows Sz, like ISO | wrong sign |
| 6 | L-PBF metal, as-built root against machined or polished | −40 to −75 [C]. "Significantly inferior"; "dominated by surface roughness" [A] | −19.0 at its Rz 40 clamp (−30.8 extrapolated to Rz 200) | **+21 to +56** | −34 to −75, and the same at Ra 16 as at 1.6 (scale-free) | −35 to +41: brackets it by coincidence |
| 7 | L-PBF 17-4PH gear, root machined to Rz 2.98 (Concli 2021 [R]) | Gear limit 453–480 MPa at R 0.1; cracks from porosity, with a 50 µm surface defect. AM is ≈30 % below wrought in *uniaxial* tests (their ref. 30); no wrought gear was tested | −4.8 | outside its validity (50 µm > 2Rz = 6 µm). The loss belongs to the basis: ≈0 on an AM coupon, ≈ +25 on a wrought one | as row 1 | as row 1 |
| 8 | MIM Ti-6Al-4V as moulded, two feedstocks at Ra 1.95 and 2.08 (Ferri 2010 [R]) | 350 against 400 MPa, 12.5 % apart, from 100 against 50 µm moulding notches the stylus missed | equal (−11.6 each) | **blind to the 12.5 %** | equal: the stylus profiles match, and a notch at twice the size has the same `K_t` | blind |
| 9 | Press-and-sinter PM steel, root rolled against as-sintered | +30 to +60 [C], from density and residual stress; porosity acts as notches [A] | +6.5 to +10.5 (Rz 10 → 3 to 1) | **−20 to −54** | right sign, sized by λ_s; blind to density | unbounded |
| 10 | Ductile iron, as-cast skin against machined | −20 to −40 [C]. FKM rates cast, forged and rolled skins at Rz 200 "even if measured values are available" [R] | −5.5 (L3, ferritic) to −19.0 (L1, pearlitic) at the Rz 40 clamp; class assignment [C] | **+1 to +35** | as row 6 | unknown |
| 11 | Moulded POM gear against the library's own moulded ASTM D671 bar (Delrin) | No direct test found. Governing instead: a root at 70 °C instead of 30 °C costs > 14 % in stress, bracketed by Kalin 2017's data [R] (every 30 °C gear ran out at 1.4 N m; at 70 °C 1.0 N m ran out and 1.2 N m failed), ≈ 22–30 % on an assumed S-N slope [D]; and moulding voids (Hasl 2018, Tavčar 2018 [R]) | ≈ 0 relative; −0.5 to −1.9 against a polished coupon, on an arbitrary line | ≈ 0, invisible beside temperature | ≈ 0 relative; neither profile is known | ≈ 0 |
| 12 | POM or PA gear hobbed from bar, against a moulded bar | No gear data found. A direct notch test exists, result unread: Crawford, Klewpatinond, Benham 1979, moulded against machined notches in acetal copolymer (title only). Machining loses the skin (−); bar has no voids (+) | −3.0, −6.4 or −10.6 at Rz 10, depending on which steel line is picked | sign unknown | −34 to −65 at L 0; −1.5 with a polymer's L ≈ 100 µm, which is forbidden | −35 to −65 if the root is as roughness-blind as row 13 |
| 13 | SLS PA12 as built (Ra 15–18.5) against injection-moulded (Ra 0.7), axial at R −1 (Van Hooreweder 2013, 2014 [R]) | ≈ 0: "equal" (VH 2013), S-N lines within 4 MPa over 10⁴–10⁶ (VH 2014), endurance ≈ 18 MPa; cracks start at internal pores, failure thermo-mechanical at 3 Hz. Unnotched and axial: the authors call the roughness negligible *because* the load is tension–compression, which a root in bending does not share. Blattmeier 2012 [A], laser-sintered with and without a surface finish: the surface's contribution "can be neglected" | −8.1 to −14.9 at the Rz 40 clamp; −10 to −24 extrapolated to Rz 60–130 (Ra × 4–7) [D] | **−8 to −15** (clamped) | the as-built profile's `K_t` over the moulded bar's: ≈ −30 to −65 | ≈ −30 to −65 |
| 14 | SLS PA12, as-built notch against machined notch (VH 2014 [R]) | The as-built notch lived "significantly" longer. Three specimens each, at one amplitude (35 MPa, a finite life, not the endurance); the authors say "probably" | the as-built notch weaker by ≥ 8–15 | **wrong sign** (confounded with the skin) | as-built weaker | wrong sign |
| 15 | Woven CFRP gear milled from plate (Zorko 2021 [R]) | Fails by ply delamination in the flank; "no fatigue damage was observed in the root region" | no class | undefined: a different failure | not a root failure | undefined |
| 16 | Brass C360, machined; the endurance is a 0.30 × UTS estimate | none found | no line; its σ_u of 386 routes it to L2, −6.4 | unknown | −34 to −65 | unknown |

**ISO's term is half of a pair** [R for the steels' ρ′; C for St and GG]. The notch support `Y_δrelT` runs
the other way and reads ρ′, a tabulated material length. Hobbed at Rz 10, with the support measured from a
coupon's gradient of 0.2 /mm, the pair nets **−8.4 %** on 4340 Hardened (−8.6 at 0.262 /mm, the 7.62 mm
R. R. Moore bar), **+3.1 %** on annealed 4340 (+2.1), +14.5 % on nitrided steel and +25 % on grey iron. The
surface term alone therefore has the wrong sign for every class below about σ_u 800, by 8–28 points. Where
the sign changes depends on the line a Re 500 steel is put on and on the coupon's gradient (at zero
gradient, Re 600 already nets +3.1).

## 2. The owner's test

**ISO: the deviation does not stay bounded, and it changes sign.**
- **At home it is bounded:** on wrought steel, hobbed or ground, with no surface-layer change, 0 by
  construction and 3–4 points against FKM, including its own 4.1-point step at σ_u 800 (row 3), a class
  boundary FKM does not have.
- **Formed, treated or porous roots (rows 4–10): 8–56 points, both ways.** Unconservative on as-built AM
  and cast skins (+21 to +56, +1 to +35); conservative on peened and rolled roots (−8 to −54); sign
  flipped on ground carburised roots. The effect there is carried by residual stress, density, the skin,
  and defects deeper than 2·Rz, which ISO's own validity clause excludes.
- **Non-metals (rows 11–16): no class exists.** Routed by σ_u, a polymer lands on L2 at −6.4, a figure
  owing nothing to the material; the choice of line alone spans 3.0–10.6 at Rz 10. Where polymer routes
  were measured, ISO is −8 to −15 (at its Rz 40 clamp) against about nil (row 13, axial and unnotched, so weak for a root in
  bending), wrong-signed against a skin (row 14), and misses the failure on CFRP (row 15). A strength-scaled
  law flips when extrapolated: FKM's K_R on POM (σ_u 71) gives +9.9 % at Rz 10 with steel's constants and
  −0.6 % with aluminium's [D].
- **The class is a process attribute.** Nitrided and case-hardened roots have similar hardness yet differ
  3.5× at a fixed Rz. One micro-notch attenuated by Neuber's q with ISO's own ρ′ fits all three lines [D],
  so choosing a line amounts to choosing a ρ′.
- **Against no term, which is what rule (2) weighs** (the risk the factor *adds*). Most of the 8–56 is
  the route's own effect, which no term misses as much or more. ISO's increment over no term: it removes
  3–11 points on wrought steel, and in the right direction 5–31 on as-built and as-cast metal skins and
  6.5–10.5 on rolled PM (rows 6, 9, 10; magnitudes [C]); it is within ±2 on peened roots (row 5); it adds 4.1
  at the σ_u 800 step (row 3), about 6.5 on ground carburised roots (row 4, [C]), and on a non-metal
  whatever line is chosen for it (8–15 against the one measured nil, row 13). On metals off wrought steel
  ISO is mostly the lesser error, not the greater.
- **Verdict: fails, on the test the owner named.** Off iron and steel it has no basis: a line has to be
  invented, the choice alone moves the answer 7.6 points, a strength-scaled extension flips sign, and the
  one measured polymer comparison shows nil where it predicts 8–15. Applied to the steels alone, it puts
  the two sides of a steel-against-non-steel comparison on different bases (below). The growth from 0–4 to
  8–56 points on metallic routes is not the reason, since no term grows as much there.

**Geometry only: fails before the basis moves, and worse.**
- **At home:** −23 to −59 on wrought steel (rows 1 and 2).
- **Ill-posed:** on one surface `K_t` goes from 1.5 to 2.9 (3.9 at λ_s 1 µm) while Rz moves only from 5.1
  to 6.8 µm. The instrument's filter sets the answer, not anything on a drawing [D].
- **Scale-free:** elasticity has no length, so the same profile at ten times the size has the same `K_t`.
  It cannot see size, which is what separates the MIM batches (row 8) and makes a polymer roughness-blind.
- **Brought into line only by a critical distance L.** At one surface the factor on the endurance is 0.35,
  0.66, 0.85, 0.985 and 0.999 at L = 0, 3.5, 16, 100 and 490 µm [D]. The material length decides
  everything, and rule 1 forbids it.
- **Verdict: fails.** The deviation spans −65 to +41 across routes, near 0 only on as-built metal, by
  coincidence.

**No term:** the deviation is the effect itself, and nothing is added that depends on the route. On steels
its size and sign are known from ISO's own lines (§3); elsewhere the sign is unknown, with the measured
evidence about nil between polymer routes (axially) and small beside temperature. No class boundary is
introduced. On as-built and as-cast metal skins its error is the larger of the two (rows 6, 10); there the
remedy is an endurance measured on that root (§4), not either term.

**Comparing dissimilar designs, the tool's key use.** In the rating's utilisation (stress over allowable)
at the median tooth: without a term, by ISO's pair, the tool is unconservative by 8.4 % on hardened steel
and conservative by 3.1 % on annealed; on polymers the sign is unknown. ISO's surface term on the steels
alone leaves only the support, conservative by 2.4 % and 10.2 %, narrowing the spread between the two
steels from 11.5 to 7.8 points, which is ISO's whole case. But it leaves annealed steel 10.2 %
conservative against non-steels left untouched, adds the σ_u 800 step, and is right only where ISO's lines
hold. Applied to everything, it shifts polymer routes against each other by 8–15 points that measurement
(axial) does not show. Neither version reduces the error of a steel-to-polymer comparison; both add route-dependent error.

## 3. Recommendation

**No surface term, with a note**, under rule (2). Both terms fail the test: ISO's has no basis on the
non-metals the owner named and adds error wherever one is given a line, and the geometry-only term is wrong
at home and realistic only with a forbidden material length.

**Bias record for `docs/state.md` (Known-approximate).** Exact wording:

> - **A coupon's endurance is taken as the root's: no surface term and no notch support.** A
>   `fatigue_allowable` measured on a coupon is read as the root's own, whatever the root's finish or route.
>   On the steels, ISO 6336-3's lines (IACS UR M56.3.11–12) give the size. At a hobbed root (Rz 10), the
>   surface alone would take **10.6 %** off 4340 Hardened's endurance (σ_u ≥ 800 MPa) and **6.4 %** off
>   annealed 4340's: the allowable used is 1.118 and 1.069 times the root's, so the rating's utilisation is
>   **low** by 10.6 % and 6.4 %, which is unconservative. At a ground root (Rz 3) the figures are 4.8 % and
>   2.4 %; at Rz 40, 19.0 % and 11.8 %. Taken with the notch support ISO pairs it with, measured from the
>   coupon's own gradient (0.2 /mm) at the median tooth (`q_s` 2.5), the omission leaves the utilisation
>   **low by 8.4 %** on 4340 Hardened (unconservative) and **high by 3.1 %** on annealed 4340
>   (conservative): on the softer steel the omitted support outweighs the omitted surface. The support grows
>   with `q_s`: across 1.5–8 these run from 9.1 to 6.0 % low, and from 0.1 to 13.9 % high.
>   On brass, POM and the polyamides no figure exists, and **the sign is unknown**. Not all are polished
>   coupons: brass's is a 0.30 × ultimate estimate of a rotating-beam figure, POM's an ASTM D671 moulded
>   bar, and the polyamides' are 0.30 × ultimate conventions with no specimen. Where polymer routes were
>   measured, in tension–compression, the surface's share was about nil (as-built against moulded PA12,
>   cracks starting at pores, which the authors tie to the axial load; a second study found a sintered
>   surface's contribution negligible against a finished one), or ran against the roughness (an as-built
>   notch outlived a machined one, at a finite life). No measurement on a polymer root in bending was found.
>   Both sit beside a root-temperature effect of more than 14 % in stress between 30 and 70 °C (about
>   22–30 % on an assumed S-N slope; every 30 °C gear ran out), which the tool does not model. A formed, treated or porous root (as built by additive manufacture, as cast, sintered, peened,
>   rolled, or ground after hardening) moves its endurance by tens of percent either way (10–75 %, recalled
>   and not checked against a source), through residual stress, density, skin and defects that no roughness
>   figure carries. The endurance for such a root should be one measured on it
>   (`fatigue_specimen = "gear_root"`), reduced from load to stress by this tool's own model: ISO's `σ_FE`,
>   reduced by ISO's, sits on a stress about 25 % above the default's at the median ordinary fillet
>   (+6.4 % against −15.0 % of the elastic peak). ISO's
>   factor is declined because it has no line off iron and steel. A line chosen for a non-metal sets the
>   answer by the choice (3.0–10.6 % at Rz 10) where the one measured polymer comparison shows nil, and
>   applied to steels alone it puts a steel design and a non-steel one on different bases. A roughness term
>   with no material length (the profile's elastic `K_t`) is declined because it takes 34–65 % off a hobbed
>   steel root that loses about 6–11 %.

**If accepted:** the argument (§2) belongs in `rationale.md`, cited from the entry. The figures need a
generating tag for `check_figures.py`: `fillet_bem.py` already prints the ratios 1.118 and 1.069, and the
pair's −8.4 and +3.1, and their range across `q_s`, would need a line added (`isobasis/pair.py` has it).

**Owner's option, a runtime note**, on the pattern of `train.stresses_nominal`: a **suggested** key,
`train.root_surface_uncredited`, raised on every rated case with a coupon's endurance. Suggested English:
"Root endurances are their coupons': no surface or notch-support term is applied. By ISO's factors, at a
hobbed root that is about 8 % unconservative on hardened steel and conservative on softer steel; on other
materials the sign is unknown." Cost: the key, its site, and five strings.

## 4. Inputs

**The recommendation adds none.** `fatigue_specimen = "gear_root"` already lets a route-specific endurance
be entered. The field changes only how the figure's load ratio is read
(`Material::reversed_bending_fraction`), so the figure must be reduced from load to stress by this tool's
own model: ISO 6336-5's `σ_FE` stands on ISO's `σ_F`, about 25 % above the default's at the median ordinary
fillet (`docs/state.md`, +6.4 % against −15.0 % of the elastic peak), and entered as it is it carries that
gap.

**What the declined options would have needed.**
- **ISO:** per gear, a root Rz (a manufacturing, drawing input: allowed); per material, σ_u (a common
  property, allowed, but today only a note in the toml); a class (case-hardened, through-hardened,
  normalised, nitrided, cast iron), which for a steel is its heat-treatment callout on the drawing
  (allowed), but for any other material does not exist and would have to be assigned per material, in
  effect ISO's ρ′ (**forbidden in substance**); per material, the roughness its endurance was measured at,
  since the ratio needs the specimen's side too and the library's are not all polished (§5), which a
  datasheet rarely states (a new attribute); and, used correctly, its pair's ρ′, a tabulated material
  length that exists only for iron and steel (**forbidden**).
- **Geometry only:** per gear, a root-profile descriptor (Rdq or valley radius; Rz does not fix `K_t`) plus
  the measuring filter λ_s (metrology inputs, allowed but not on drawings); to be realistic, a critical
  distance L (**forbidden**).

## 5. Corrections to existing work files, and one separate decision

- **`work/decision-bending-default.md`** says "E is owed whatever is chosen: every endurance in the library
  is a polished coupon". Wrong: the toml never says "polished", POM's endurance is a D671 flexural bar, the
  polyamides' are 0.30 × ultimate estimates with no specimen, and only annealed 4340's is a datasheet value.
- **Its "Net" column** credits the support against a zero gradient, double-counting the coupon's own
  support (about +7.5 % on annealed 4340, +1.7 % on hardened [D]); the column is optimistic by that much.
  The same credit is in `tools/fillet_bem.py`'s `notch_support` (the numerator of `Y_δrelT` only), so
  `docs/state.md`'s material table and its "ISO's notch support" rows (18.5 % and 4.1 % at the median)
  carry it too.
- **`docs/state.md`, two sentences the entry contradicts**, to amend when it is adopted: the bending
  entry's "with the support (conservative) and the surface (unconservative) unmeasured" for brass, POM and
  the polyamides, and the load-ratio entry's "the coupon's missing notch, surface, size and reliability
  reductions, which run the other way". Off the steels the surface's sign is not known.
- **`tools/fillet_bem.py`**, the comment above `RZ_POLISHED`, makes the same claim: false for POM and the
  polyamides. It also omits ISO's nitrided line.
- **Concli 2021's "≈30 %"** is uniaxial AM against wrought; the gear was not compared with a wrought gear.
- **ISO 6336-2's `Z_R` (flank) is a different case**, not decided here. It is consistent with the library's
  σ_Hlim basis, needs no new material property (C_ZR reads σ_Hlim), and applies only to the steel flanks the
  tool rates. Omitting it overrates a hobbed flank (Rz10 ≈ 10) by 9–17 % in permissible stress. It belongs
  with `Z_L` and `Z_v` and should go to the owner separately. *Since decided by the owner (plan.md,
  2026-10-03): no factor, with a note; `docs/state.md` carries it, sized by `tools/iso_6336_3_stack.py`.
  This file's evidence is the root's and was not extended to flanks.*

## Sources

**Read.** IACS UR M56 Rev.4 2021/Corr.2 2023, M56.2.10, 3.8, 3.11, 3.12
(https://iacs.org.uk/resolutions/unified-requirements/; cached `~/.cache/gearcalc-work/notch-research/iacs_m56.txt`) ·
IRS Classification Notes, Marine Gears, Dec 2021 · GEARpie DIN3990.py (https://github.com/cfernandesFEUP/GEARpie) ·
McKelvey, Lee, Barkey 2012, FKM (https://doi.org/10.1007/s11668-012-9599-4) ·
Van Hooreweder et al. 2013 (https://doi.org/10.1016/j.polymertesting.2013.04.014; accepted text https://lirias.kuleuven.be/retrieve/ad7ddd58-6111-48eb-b76f-8497910cb040) ·
Van Hooreweder & Kruth 2014 (https://doi.org/10.1016/j.cirp.2014.03.060) ·
Kalin & Kupec 2017 (https://doi.org/10.1016/j.wear.2017.02.003) · Hasl et al. 2018 (https://doi.org/10.1299/jamdsm.2018jamdsm0016) ·
Tavčar et al. 2018 (https://doi.org/10.1299/jamdsm.2018jamdsm0006) · Zorko et al. 2019 (https://doi.org/10.1016/j.mechmachtheory.2019.07.001) ·
Zorko et al. 2021, CFRP gears (https://doi.org/10.1016/j.polymertesting.2021.107339) ·
Concli et al. 2021 (https://doi.org/10.3390/app11073019) · Ferri, Ebel, Bormann 2010 (https://doi.org/10.1016/j.msea.2009.11.007) ·
Ho et al. 2021 (https://doi.org/10.5755/j02.ms.24475) · KISSsoft, ISO 6336:2019 slides · VDI 2736-2/-4 contents · KHK plastic gears.

**Abstract only.** Spierings 2013; Mower & Long 2016; Greitemeier 2016; Holmes & Queeney 1985; Hadrboletz &
Weiss 1997; Damon 2021; Baragetti 2011; Mao 2015; Zhou & Mallick 2006; Mortazavian & Fatemi 2014;
Blattmeier et al. 2012 (https://doi.org/10.1108/13552541211212140, via Crossref).

**Title only.** Crawford, Klewpatinond, Benham 1979, Polymer 20:649, moulded and machined notches in acetal
(https://doi.org/10.1016/0032-3861(79)90181-2): found via Crossref, result not read.

**Recalled, not checked.** The data behind ISO's lines (DIN 3990-3; Siebel–Gaier); ISO's cast-iron classes,
and ρ′ for St and GG; Marin's constants; the size of the effect of as-built AM, rolled PM, ground carburised,
peened carburised and as-cast roots; Kahlin 2017's Kt ≈ 3; Berer 2013, known only through Zorko 2019's summary.

**Derived.** Scratchpad `isobasis/`, `skeptic_iso/`, `surfkt/`, `skeptic_surf/`, `synth/`. No web search here; ISO's text returned 403.

**Adversarial review 2** re-read IACS M56.3.11–12, Van Hooreweder 2013 §5.3 and 2014's abstract from the cached
texts (`polysurf/`); re-ran `isobasis/pair.py` across `q_s`; no web search (the session's budget was spent).
It corrected the note's sizes to the utilisation, §2's reason, the dissimilar-designs figures, §4 and §5.

**Adversarial review 1** re-read Van Hooreweder 2013 in full (open text, above), Kalin 2017's results, Ferri 2010,
Hasl 2018, Tavčar 2018 and Zorko 2021 from the cached texts, and recomputed rows 3–5, 9, 10, 13 and the pair
independently (`rev1/recompute.py`): the ISO arithmetic stands. It corrected Kalin's figure (> 14 % bracketed,
22–30 % only on an assumed slope), row 12's "no data" (Crawford 1979 exists, unread), row 13's clamp and
evidence, row 14's finite life, and the pair's "5–28" to 8–28. No web search (budget spent); Crossref, OpenAlex fetched.
