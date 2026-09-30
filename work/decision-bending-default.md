# Decision: the default bending rating (Q10, for the owner)

Measured by `python3 tools/fillet_bem.py` on 228 of the crate's own teeth, recorded in `docs/state.md`
(Known-approximate). Figures are rated / exact elastic peak − 1, median (min … max); **negative is
unconservative**. The instrument agrees with a second solver (`tools/shoulder_trefftz.py`) to 0.06 % and
passes three exact canaries; the checker's own FEM put 8 teeth within 0.10 % of the record. Peterson's chart,
the pre-registered canary, is 0.5–2 % off both solvers and is reported, not gated. "Net" credits ISO's notch
support and a hobbed root (Rz 10) against the polished coupon the library's endurance is measured on, on
**4340 Hardened** (annealed 4340 in brackets).

| Option | External, ordinary fillets (85) | Tight fillets (29) · rings (48) | Net | New inputs | Cost |
|---|---|---|---|---|---|
| **A. Keep Dolan–Broghamer** (today) | −15.0 % (−30.1 … +7.4), 89 % under; −7.8 % at 14.5°, **−16.9 % at 20°, every tooth under**; −24.5 % at 25°, past DB's calibration | −52.9 % · −26.8 %, every one under | −20.6 %, every tooth under (−5.6 %) | none | none: the bias is recorded (rule 6) |
| **B. DB without its axial term** | −2.3 % (−13.7 … +15.4) | −46.4 % · −10.8 % | −9.0 % (+8.4 %) | none | one term; AGMA 908 subtracts it, so B is a model neither source states unless DB's 1942 nominal was bending-only (the bulletin is unread) |
| **C. ISO `Y_F·Y_S` on its tangent section** | **+6.4 %** (−7.1 … +22.8): conservative by that at the median, 7 % of teeth under | −29.9 % · +26.3 %; with `Y_S` continued past its `q_s = 8` clamp, tight −3.7 % (−23.9 … +30.6) | −0.8 % (+16.9 %) | none | the set exists (`RootStressModel::Iso6336`, `CriticalSection::TangentAngle`): flip the default, re-record the corpus and documents; continuing `Y_S` changes the fit's stated range |
| **D. A computed `K_t`** (BEM, IGA or Trefftz on the exact outline) | model error ≈ 1 %: mesh ≤ 0.12 %, body ≤ 0.5 %, and the load patch and plane strain besides | the same at every radius | support and surface only | none | a solver in Rust (dense LU on 1–2 k unknowns); 0.1–1 s per member against a train solve of microseconds (rule 3): an on-demand analysis, or cached per geometry |
| **E. Add a surface factor** (to any of A–D) | every row lower: ISO `Y_RrelT` at Rz 10, 1/1.118 hardened, 1/1.069 annealed; Marin (recalled), ×0.849 ground and ×0.649 machined on hardened, ×0.906 and ×0.798 on annealed | the same | already in the column | per gear: root Rz or a finish class; per material: `σ_u` (a note today) and a heat-treatment class | a rule of thumb with class constants; ISO's comes paired with its notch support (+4.1 % hardened, +18.5 % annealed at the median), which runs the other way |

What the measurement says, without choosing:
- A is unconservative almost everywhere; at 20°, inside DB's calibration, every ordinary tooth is under. Most
  of it is the axial term (12.6 % of `Y_F` at the median); it grows with tooth count and on tight fillets.
- C is conservative by 6.4 % at the median on ordinary external fillets and by 26 % on rings, and
  unconservative where its clamp binds; continuing `Y_S` past the clamp centres tight fillets (−3.7 %), with a
  +30.6 % tail. B centres too, but is not a published model.
- E is owed whatever is chosen: every endurance in the library is a polished coupon.
- Hardness ÷ 3 is read nowhere as a yield (it estimates σ_u; as σ_y it would be 10–60 % high).
