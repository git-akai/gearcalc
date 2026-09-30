# Decision: the default bending rating (Q10, for the owner)

Measured by `python3 tools/fillet_bem.py` on 228 of the crate's own teeth, recorded in `docs/state.md`
(Known-approximate). Figures are rated / exact elastic peak − 1, median (min … max); **negative is
unconservative**. "Hardened, net" adds ISO's notch support (+4.1 % at the median) and a hobbed root (Rz 10)
against the polished coupon the library's endurance is measured on (the coupon is 1.118× stronger). The
instrument agrees with an independent Trefftz solver to 0.06 % and passes three exact canaries; Peterson's
chart, the pre-registered canary, is itself 0.5–2 % off both solvers and is reported, not gated.

| Option | External, ordinary fillets (85) | Tight fillets (29) · rings (48) | 4340 Hardened, net | New inputs | Cost |
|---|---|---|---|---|---|
| **A. Keep Dolan–Broghamer** (today) | −14.8 % (−30.1 … +7.6), 89 % under; −7.5 % at 14.5°, −23.6 % at 25° | −52.9 % · −26.8 %, every one under | −20.6 %, every tooth under | none | none: the bias is recorded (rule 6) |
| **B. DB without its axial term** | −1.9 % (−13.7 … +15.7) | −46.4 % · −10.8 % | −8.8 % | none | one term; AGMA 908 subtracts it, so B is a model neither source states unless DB's 1942 nominal was bending-only (the bulletin is unread) |
| **C. ISO `Y_F·Y_S` on its tangent section** | +6.9 % (−7.1 … +22.8), 7 % under | −29.9 % · +26.3 % | −0.5 % | none | the set exists (`RootStressModel::Iso6336`, `CriticalSection::TangentAngle`): flip the default, re-record the corpus and documents. Past its `q_s = 8` clamp −25.3 %; continuing `Y_S` there gives +2.9 %, a change to the fit's stated range |
| **D. A computed `K_t`** (BEM, IGA or Trefftz on the exact outline) | ≈ 0 by construction: mesh ≤ 0.12 %, model (rim, teeth) ≤ 0.5 % | ≈ 0 at every radius | support and surface only | none | a solver in Rust (dense LU on ~1000 unknowns); 0.1–1 s per member against a train solve of microseconds (rule 3): an on-demand analysis, or cached per geometry |
| **E. Add a surface factor** (to any of A–D) | every row lower by 1/1.069–1/1.118 (ISO `Y_RrelT`, Rz 10) or ×0.849–×0.649 (Marin, ground/machined, recalled) | same | already in the column | per gear: root Rz or a finish class; per material: `σ_u` (a note today) and a heat-treatment class | a rule of thumb with class constants; ISO's is paired with its notch support (+4 % hardened, +18 % annealed), which runs the other way |

What the measurement says, without choosing:
- A is unconservative almost everywhere, more so at higher pressure angle, larger tooth count, tighter fillets
  and on every ring; most of it is the axial term (12.6 % of `Y_F` at the median).
- C is centred within about ±10 % on ordinary external fillets, conservative on rings (+26 %), and
  unconservative wherever its clamp binds; B is centred too, but is not a published model.
- No closed-form fit rates tight fillets (ρ_f/s_Fn < 0.02) within 29 %; only D does.
- E is owed whatever is chosen: every endurance in the library is a polished coupon.
- Hardness ÷ 3 is read nowhere as a yield (it estimates σ_u; as σ_y it would be 10–60 % high).
