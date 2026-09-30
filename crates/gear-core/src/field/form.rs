//! A gear's tip form: its flank, modified toward the tip edge by one function `F(σ)`, the gear's
//! own input.
//!
//! `F` is measured along the flank normal into the material, as every profile modification is
//! (ISO/TR 21771), and depends on one coordinate, `σ`: the distance across the tip edge along the
//! flank, in the section normal to the edge, from the sharp edge; positive down the flank, negative
//! beyond the edge. A point at radial depth `u` below the tip circle has `σ = u / sin θ_a`, where
//! `θ_a` is the exterior corner between the flank and the tip land ([`TipCorner`]).
//!
//! `F` is the sum of two pieces:
//! - the tip relief `C (1 − u/L)²` where `u < L`, continued beyond the edge;
//! - the edge: a round of radius `r_e` tangent to the flank at `σ_t = r_e tan(θ_a/2)`, where
//!   `F = r_e − √(r_e² − w²)` with `w = σ_t − σ`, up to `w_e = r_e sin θ_a`; then the tip land,
//!   `F = r_e (1 − cos θ_a) + tan θ_a (w − w_e)`.
//!
//! Each piece meets the next with equal value and slope, so the form is C¹. Its curvature across
//! the edge is that of `F`'s graph, `κ = F''/(1 + F'²)^{3/2}`: `1/r_e` on the round without a
//! relief, and with one the summed form's (relief plus round), not the round's own; `0` on the
//! flank and the land; and it steps at each joint ([`TipForm::joints`]).
//!
//! A sharp edge is a kink (C⁰) and is refused: the field's peak pressure diverges as `r_e → 0`.
//! The prototype read a sharp edge with the flank's slope and curvature carried past the kink; that
//! is a convention, not a limit of this family, and it is not ported.
//!
//! Ports the prototype's `form.Form`; its records are `tests/data/field_oracle/form.json`.

/// A relief: `depth · (1 − d/length)²` where `d < length`, `d` the distance from its edge, and
/// continued beyond the edge (`d < 0`). Its value and slope vanish at `d = length`, where it joins
/// the unmodified surface C¹.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Relief {
    /// `C`, the relief at the edge, along the surface normal, mm.
    pub depth: f64,
    /// `L`, the distance from the edge over which it runs out, mm; radial, for a tip relief.
    pub length: f64,
}

/// A gear's tip form, the gear's own input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ToothForm {
    /// A relief of the profile toward the tip edge.
    pub tip_relief: Option<Relief>,
    /// `r_e`, the round on the tip edge, mm. `None` is a sharp edge, which is refused.
    pub edge_radius: Option<f64>,
}

/// Why a form cannot be laid on a tooth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormRefused {
    /// The tip edge is a kink (C⁰): no edge radius, a zero one, or one too small for floating
    /// point to hold its round, whose arithmetic squares `r_e cos θ_a` (below `√f64::MIN_POSITIVE`
    /// that square is subnormal and the round's slope loses its bits).
    SharpEdge,
    /// An edge radius that is negative, or whose square is not a finite number.
    NotARadius,
    /// A relief whose depth is not a finite length `≥ 0`, whose length is not a finite length
    /// `> 0`, or whose curvature `2 C sin²θ_a / L²` is not finite (a step, in floating point).
    NotARelief,
    /// No corner: the tip circle is on or inside the base circle, so it meets no involute, or the
    /// inputs are not numbers.
    NoCorner,
}

/// The exterior corner between the flank and the tip land, in the section normal to the tip edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TipCorner {
    /// `sin θ_a`.
    sin: f64,
    /// `cos θ_a`, in `(0, 1)`.
    cos: f64,
}

impl TipCorner {
    /// An involute helicoid's corner, for either kind, in closed form:
    /// `cos θ_a = tan α_a / √(tan²α_a + 1 + tan²β_a)` and `sin θ_a = √(1 + tan²β_a) / √(…)`, where
    /// `tan α_a = √(r_a² − r_b²) / r_b` is the transverse pressure angle at the tip and
    /// `tan β_a = r_a τ` the helix at the tip circle, `τ` the lead's twist in radians per mm.
    pub fn involute(tip_radius: f64, base_radius: f64, twist: f64) -> Result<Self, FormRefused> {
        let tan_pressure =
            (tip_radius * tip_radius - base_radius * base_radius).sqrt() / base_radius;
        let tan_helix_sq = (tip_radius * twist).powi(2);
        let norm = (tan_pressure * tan_pressure + 1.0 + tan_helix_sq).sqrt();
        let corner = Self {
            sin: (1.0 + tan_helix_sq).sqrt() / norm,
            cos: tan_pressure / norm,
        };
        // A tip on the base circle gives cos 0, inside it NaN; either way there is no corner.
        if corner.cos > 0.0 {
            Ok(corner)
        } else {
            Err(FormRefused::NoCorner)
        }
    }

    /// `sin θ_a`.
    pub fn sin(self) -> f64 {
        self.sin
    }

    /// `cos θ_a`.
    pub fn cos(self) -> f64 {
        self.cos
    }

    /// `tan θ_a`: the tip land's slope in `σ`.
    pub fn tan(self) -> f64 {
        self.sin / self.cos
    }
}

/// The form at one `σ`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FormPoint {
    /// `F`, along the flank normal into the material, mm.
    pub offset: f64,
    /// `F' = dF/dσ`.
    pub slope: f64,
    /// `κ = F''/(1 + F'²)^{3/2}`, the curvature of `F`'s graph across the edge, 1/mm.
    pub curvature: f64,
}

/// A tooth form laid on its gear's tip corner: `F(σ)` as the field reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TipForm {
    corner: TipCorner,
    /// The tip relief and `σ_r = L / sin θ_a`, where it ends.
    relief: Option<(Relief, f64)>,
    /// `r_e`.
    radius: f64,
    /// `σ_t = r_e tan(θ_a/2)`, where the round meets the flank.
    tangency: f64,
    /// `w_e = r_e sin θ_a`, the round's extent in `w = σ_t − σ`.
    span: f64,
    /// `σ_t − w_e`, where the round meets the tip land.
    land: f64,
    /// `r_e (1 − cos θ_a)`, the offset where the land begins.
    rise: f64,
}

/// The piece of the edge a `σ` lies on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Edge {
    Flank,
    Round,
    Land,
}

/// `F` and its first two derivatives in `σ`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Jet {
    f: f64,
    d1: f64,
    d2: f64,
}

impl Jet {
    const ZERO: Self = Self {
        f: 0.0,
        d1: 0.0,
        d2: 0.0,
    };

    fn plus(self, o: Self) -> Self {
        Self {
            f: self.f + o.f,
            d1: self.d1 + o.d1,
            d2: self.d2 + o.d2,
        }
    }

    fn point(self) -> FormPoint {
        let g = 1.0 + self.d1 * self.d1;
        FormPoint {
            offset: self.f,
            slope: self.d1,
            curvature: self.d2 / (g * g.sqrt()),
        }
    }
}

/// Where a tip relief ends in `σ`, `L / sin θ_a`, once the relief is one.
fn relief_end(relief: Relief, corner: TipCorner) -> Result<f64, FormRefused> {
    let Relief { depth, length } = relief;
    let curvature = 2.0 * depth * (corner.sin / length).powi(2);
    if depth >= 0.0 && length > 0.0 && length.is_finite() && curvature.is_finite() {
        Ok(length / corner.sin)
    } else {
        Err(FormRefused::NotARelief)
    }
}

/// A tip relief's jet at `σ`, where it applies: `u = σ sin θ_a`, `v = 1 − u/L`.
fn relief_jet(relief: Relief, corner: TipCorner, across: f64) -> Jet {
    let Relief { depth, length } = relief;
    let v = 1.0 - across * corner.sin / length;
    Jet {
        f: depth * v * v,
        d1: -2.0 * depth * v * corner.sin / length,
        d2: 2.0 * depth * (corner.sin / length).powi(2),
    }
}

impl TipForm {
    /// `form` laid on `corner`, or why it cannot be.
    pub fn new(form: &ToothForm, corner: TipCorner) -> Result<Self, FormRefused> {
        let radius = form.edge_radius.ok_or(FormRefused::SharpEdge)?;
        if radius < 0.0 || !(radius * radius).is_finite() {
            return Err(FormRefused::NotARadius);
        }
        if (radius * corner.cos).powi(2) < f64::MIN_POSITIVE {
            return Err(FormRefused::SharpEdge);
        }
        let relief = match form.tip_relief {
            Some(r) => Some((r, relief_end(r, corner)?)),
            None => None,
        };
        let span = radius * corner.sin;
        // r_e tan(θ_a/2), as r_e sin θ_a / (1 + cos θ_a).
        let tangency = span / (1.0 + corner.cos);
        Ok(Self {
            corner,
            relief,
            radius,
            tangency,
            span,
            land: tangency - span,
            rise: radius * (1.0 - corner.cos),
        })
    }

    /// The corner the form is laid on.
    pub fn corner(&self) -> TipCorner {
        self.corner
    }

    /// Where the pieces meet, in `σ`: the round's tangency with the flank, its end on the tip
    /// land, and the relief's end where there is one. The curvature steps at these and nowhere
    /// else.
    pub fn joints(&self) -> impl Iterator<Item = f64> {
        [
            Some(self.tangency),
            Some(self.land),
            self.relief.map(|(_, end)| end),
        ]
        .into_iter()
        .flatten()
    }

    /// The form at `σ`. At a joint, where the curvature steps, the flank's side is read at the
    /// round's tangency and at the relief's end, the land's at the round's end.
    pub fn at(&self, across: f64) -> FormPoint {
        self.jet(across).point()
    }

    /// Whether `σ` is on the edge: on the round or the tip land beyond it.
    pub fn on_edge(&self, across: f64) -> bool {
        across < self.tangency
    }

    fn jet(&self, across: f64) -> Jet {
        let relieved = self.relief.is_some_and(|(_, end)| across < end);
        self.piece(across, self.edge_at(across), relieved)
    }

    /// The piece of the edge `σ` is on: the flank's side at the round's tangency, the land's at
    /// its end.
    fn edge_at(&self, across: f64) -> Edge {
        if across >= self.tangency {
            Edge::Flank
        } else if across <= self.land {
            Edge::Land
        } else {
            Edge::Round
        }
    }

    /// The jet of the given pieces at `σ`, wherever `σ` is: a joint's two sides are two calls.
    fn piece(&self, across: f64, edge: Edge, relieved: bool) -> Jet {
        let relief = match self.relief {
            Some((r, _)) if relieved => relief_jet(r, self.corner, across),
            _ => Jet::ZERO,
        };
        let w = self.tangency - across;
        let edge = match edge {
            Edge::Flank => Jet::ZERO,
            Edge::Round => {
                let r = (self.radius * self.radius - w * w).sqrt();
                // r_e²/r³, as (r_e/r)²/r: r³ underflows long before r² does.
                let k = self.radius / r;
                Jet {
                    f: self.radius - r,
                    d1: -w / r,
                    d2: k * k / r,
                }
            }
            Edge::Land => {
                let tan = self.corner.tan();
                Jet {
                    f: self.rise + tan * (w - self.span),
                    d1: -tan,
                    d2: 0.0,
                }
            }
        };
        Jet::ZERO.plus(relief).plus(edge)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::gap::{FieldGear, GearSpec};
    use crate::field::oracle::{self, closed_bound, miss, num, opt, Worst};
    use serde_json::Value;

    const EPS: f64 = f64::EPSILON;

    /// The records' tolerance, `tol.rel`: a closed form's, 1e-13 (the README's rule); every record
    /// states it ([`reproduce`]).
    const REL: f64 = 1e-13;

    fn oracle_file() -> Value {
        serde_json::from_str(include_str!("../../tests/data/field_oracle/form.json"))
            .expect("form.json parses")
    }

    fn spec_of(m: &Value) -> GearSpec {
        GearSpec {
            teeth: i32::try_from(m["z"].as_i64().expect("z is an integer")).expect("z fits"),
            module: num(&m["mn"]),
            pressure_angle: num(&m["an_deg"]),
            helix_angle: num(&m["beta_deg"]),
            profile_shift: num(&m["x"]),
            addendum: num(&m["ha"]),
        }
    }

    /// The prototype's form inputs: `C_a = 0` is no relief, `r_e = None` a sharp edge.
    fn form_of(inputs: &Value) -> ToothForm {
        let (depth, length) = (num(&inputs["Ca"]), num(&inputs["La"]));
        ToothForm {
            tip_relief: (depth > 0.0).then_some(Relief { depth, length }),
            edge_radius: opt(&inputs["re"]),
        }
    }

    /// The gears the gates run over: the oracle's five (both kinds, spur, helical, the worm) and
    /// the forms laid on them, the oracle's and a relief whose end lies on the round.
    fn corners() -> Vec<TipCorner> {
        let file = oracle_file();
        let mut out: Vec<TipCorner> = Vec::new();
        for r in oracle::records(&file) {
            let c = FieldGear::from_spec(&spec_of(&r.inputs["member"]))
                .expect("a gear")
                .tip_corner()
                .expect("a corner");
            if !out.contains(&c) {
                out.push(c);
            }
        }
        assert_eq!(out.len(), 5);
        out
    }

    fn forms() -> Vec<TipForm> {
        let mut out: Vec<TipForm> = Vec::new();
        for c in corners() {
            for r_e in [1e-3, 0.1, 0.45, 2.0] {
                for relief in [
                    None,
                    Some(Relief {
                        depth: 0.005,
                        length: 0.4,
                    }),
                    Some(Relief {
                        depth: 0.02,
                        length: 0.5 * r_e * c.sin,
                    }),
                ] {
                    let form = ToothForm {
                        tip_relief: relief,
                        edge_radius: Some(r_e),
                    };
                    out.push(TipForm::new(&form, c).expect("a C¹ form"));
                }
            }
        }
        assert_eq!(out.len(), 5 * 4 * 3);
        // Where the reliefs end: on the flank and on the round, so the relief's joint is read on
        // both; never on the land, which lies beyond the edge (σ < 0) where no relief ends.
        const ENDS: (usize, usize, usize) = (15, 25, 0);
        let ends = |e: Edge| {
            out.iter()
                .filter(|f| f.relief.is_some_and(|(_, end)| f.edge_at(end) == e))
                .count()
        };
        assert_eq!(
            (ends(Edge::Flank), ends(Edge::Round), ends(Edge::Land)),
            ENDS,
            "where the reliefs end"
        );
        out
    }

    // ---------------------------------------------------------------- the oracle

    /// Every record of `form.json`: the gear's circles, the corner, the round's constants, and
    /// `(F, F', κ)` and `on_edge` at each σ, within the record's own tolerance. A sharp edge is
    /// refused, so of its records the corner and the flank side (σ ≥ 0, the relief alone) are
    /// compared and the points beyond the edge counted as refused.
    fn reproduce(perturb: impl Fn(f64) -> f64) -> (Worst, usize, usize, usize) {
        let file = oracle_file();
        let mut worst = Worst::default();
        let (mut whole, mut refused_points, mut flags) = (0, 0, 0);
        let records = oracle::records(&file);
        assert_eq!(records.len(), 20);
        for rec in records {
            let (i, o) = (rec.inputs, rec.outputs);
            let rel = num(&rec.tol["rel"]);
            assert_eq!(rel, REL, "{}", rec.id);
            let mut check = |name: &str, port: f64, oracle: f64, passes_zero: bool| {
                let bound = closed_bound(rel, oracle, passes_zero);
                worst.see(miss(perturb(port), oracle, bound), || {
                    format!("{} {name}: {port:e} vs {oracle:e}", rec.id)
                });
            };
            let gear = FieldGear::from_spec(&spec_of(&i["member"])).expect("a gear");
            check("ra", gear.tip_radius(), num(&o["ra"]), false);
            check("rb", gear.base_radius(), num(&o["rb"]), false);
            check("tw", gear.twist(), num(&o["tw"]), true);
            let corner = gear.tip_corner().expect("a corner");
            check("sth", corner.sin(), num(&o["sth"]), false);
            check("cth", corner.cos(), num(&o["cth"]), false);
            let form = form_of(i);
            let sigma: Vec<f64> = i["sigma"].as_array().expect("σ").iter().map(num).collect();
            let values = o["values"].as_array().expect("values");
            let on_edge = o["on_edge"].as_array().expect("on_edge");
            assert_eq!((sigma.len(), values.len(), on_edge.len()), (25, 25, 25));
            let point = |s: f64, port: FormPoint, v: &Value| {
                [
                    (format!("F({s})"), port.offset, num(&v[0])),
                    (format!("F'({s})"), port.slope, num(&v[1])),
                    (format!("κ({s})"), port.curvature, num(&v[2])),
                ]
            };
            match TipForm::new(&form, corner) {
                Ok(f) => {
                    whole += 1;
                    check("sig_t", f.tangency, num(&o["sig_t"]), false);
                    check("w_end", f.span, num(&o["w_end"]), false);
                    check("F_end", f.rise, num(&o["F_end"]), false);
                    for ((&s, v), e) in sigma.iter().zip(values).zip(on_edge) {
                        for (name, port, oracle) in point(s, f.at(s), v) {
                            check(&name, port, oracle, true);
                        }
                        assert_eq!(f.on_edge(s), e.as_bool().expect("a flag"), "{}", rec.id);
                        flags += 1;
                    }
                }
                Err(why) => {
                    assert_eq!(why, FormRefused::SharpEdge, "{}", rec.id);
                    assert!(opt(&o["sig_t"]).is_none() && opt(&o["w_end"]).is_none());
                    let relief = form.tip_relief.expect("every sharp record is relieved");
                    let end = relief_end(relief, corner).expect("a relief");
                    for ((&s, v), e) in sigma.iter().zip(values).zip(on_edge) {
                        assert_eq!(e.as_bool(), Some(s < 0.0), "{}", rec.id);
                        flags += 1;
                        if s >= 0.0 {
                            let jet = if s < end {
                                relief_jet(relief, corner, s)
                            } else {
                                Jet::ZERO
                            };
                            for (name, port, oracle) in point(s, Jet::ZERO.plus(jet).point(), v) {
                                check(&name, port, oracle, true);
                            }
                        } else {
                            refused_points += 1;
                        }
                    }
                }
            }
        }
        (worst, whole, refused_points, flags)
    }

    #[test]
    fn the_oracle_records_reproduce() {
        let (worst, whole, refused, flags) = reproduce(|x| x);
        eprintln!(
            "form.json: {whole}/20 records whole, {} values within tolerance, worst {:.3} of it \
             ({}); {refused} points beyond a sharp edge refused",
            worst.count, worst.ratio, worst.at
        );
        // 15 laid forms × (5 circles and corner + 3 round constants + 25 × 3) + 5 sharp-edge
        // records × 5 + 79 flank-side points × 3.
        assert_eq!((whole, refused, flags), (15, 46, 20 * 25));
        assert_eq!(worst.count, 15 * (5 + 3 + 75) + 5 * 5 + 79 * 3);
        assert!(worst.ratio <= 1.0, "worst {} at {}", worst.ratio, worst.at);
    }

    /// The tolerance's two laws: an output a few roundings off still reproduces every record, and
    /// one ten times the tolerance off reproduces none of the values it touches.
    #[test]
    fn the_oracle_tolerance_passes_rounding_and_fails_ten_times_itself() {
        let (worst, ..) = reproduce(|x| x * (1.0 + 4.0 * EPS));
        assert!(
            worst.ratio <= 1.0,
            "rounding failed: {} at {}",
            worst.ratio,
            worst.at
        );
        let file = oracle_file();
        let mut seen = 0;
        for rec in oracle::records(&file) {
            let rel = num(&rec.tol["rel"]);
            for v in rec.outputs["values"].as_array().expect("values") {
                for x in v.as_array().expect("a triple").iter().map(num) {
                    let bound = closed_bound(rel, x, true);
                    assert!(miss(x + 10.0 * bound, x, bound) > 1.0);
                    seen += 1;
                }
            }
        }
        assert_eq!(seen, 20 * 25 * 3);
    }

    // ---------------------------------------------------------------- C¹ at every joint

    /// Each joint's one-sided limits, the pieces on either side evaluated at the joint itself:
    /// `(value jump, slope jump)` over their rounding bounds.
    ///
    /// The bounds are derived. At the round's tangency `w = 0` exactly and both sides are 0. At
    /// its end, `w` carries the rounding of `σ_t − w_e`, which moves both pieces along a common
    /// tangent, and `r = √(r_e² − w²) ≈ r_e cos θ_a` cancels `r_e²`'s rounding by `1/cos²θ_a`:
    /// value `ε r_e (2/cos θ_a + 2)`, slope `ε tan θ_a (3/cos²θ_a + 3/2)`. At the relief's end
    /// `|v| ≤ 3ε/2` (three roundings in `σ_r sin θ_a / L`), so the slope jumps at most
    /// `2 C |v| sin θ_a / L ≤ 3ε C sin θ_a / L`. Adding the relief to the edge rounds once per
    /// side: `2ε |F|` and `2ε |F'|`.
    fn joint_jumps(f: &TipForm) -> Vec<(f64, f64)> {
        let (sin, cos, tan) = (f.corner.sin, f.corner.cos, f.corner.tan());
        let (c, l) = f.relief.map_or((0.0, 1.0), |(r, _)| (r.depth, r.length));
        let relieved = |s: f64| f.relief.is_some_and(|(_, end)| s < end);
        let mut out = Vec::new();
        let mut meet = |a: Jet, b: Jet| {
            let value = EPS * (f.radius * (2.0 / cos + 2.0) + 2.0 * a.f.abs());
            let slope =
                EPS * (tan * (3.0 / (cos * cos) + 1.5) + 3.0 * c * sin / l + 2.0 * a.d1.abs());
            out.push(((a.f - b.f).abs() / value, (a.d1 - b.d1).abs() / slope));
        };
        let s = f.tangency;
        meet(
            f.piece(s, Edge::Flank, relieved(s)),
            f.piece(s, Edge::Round, relieved(s)),
        );
        let s = f.land;
        meet(
            f.piece(s, Edge::Round, relieved(s)),
            f.piece(s, Edge::Land, relieved(s)),
        );
        if let Some((_, end)) = f.relief {
            let edge = f.edge_at(end);
            meet(f.piece(end, edge, false), f.piece(end, edge, true));
        }
        assert_eq!(out.len(), f.joints().count(), "every joint is met");
        out
    }

    fn worst_jump(f: &TipForm) -> (f64, f64) {
        joint_jumps(f)
            .into_iter()
            .fold((0.0, 0.0), |(v, s), (a, b)| (v.max(a), s.max(b)))
    }

    #[test]
    fn the_form_is_c1_at_every_joint() {
        let mut joints = 0;
        let mut worst = (0.0_f64, 0.0_f64);
        for f in forms() {
            let (v, s) = worst_jump(&f);
            worst = (worst.0.max(v), worst.1.max(s));
            joints += f.joints().count();
        }
        eprintln!(
            "C¹: {joints} joints, worst value {:.3}, slope {:.3} of the bound",
            worst.0, worst.1
        );
        assert_eq!(joints, 5 * 4 * (2 + 3 + 3));
        assert!(worst.0 <= 1.0 && worst.1 <= 1.0, "{worst:?}");
    }

    /// The prototype's own probe: slopes 1e-9 either side of the joint, read by eye; here held to
    /// 1e-6.
    fn naive_c1(f: &TipForm) -> bool {
        f.joints()
            .all(|s| (f.at(s - 1e-9).slope - f.at(s + 1e-9).slope).abs() < 1e-6)
    }

    /// The oracle's first gear (17 teeth, β 20°) with r_e 0.1 and no relief.
    fn plain() -> TipForm {
        let c = corners()[0];
        let f = TipForm::new(&with(Some(0.1), None), c).expect("a round");
        assert!(forms().contains(&f));
        f
    }

    #[test]
    fn the_c1_gate_fails_its_plants() {
        let f = plain();
        assert!(worst_jump(&f).0 <= 1.0 && worst_jump(&f).1 <= 1.0 && naive_c1(&f));

        // Near miss: the round ends 1e-9 r_e short of its tangency with the land. Its value
        // joins (a C⁰ gate passes it) and the prototype's probe passes it; its slope does not.
        let mut p = f;
        p.land += 1e-9 * p.radius;
        let (v, s) = worst_jump(&p);
        assert!(
            v <= 1.0 && naive_c1(&p),
            "a C⁰ gate and the naive probe pass it"
        );
        assert!(s > 1e3, "the slope jump is caught: {s}");

        // A rounding passes: the round's end moved by one unit in the last place.
        let mut p = f;
        p.land = p.land.next_up();
        let (v, s) = worst_jump(&p);
        assert!(v <= 1.0 && s <= 1.0, "one ulp: {v} {s}");

        // Ten times the bound, in the slope alone (the joint moved by δ: slope jump
        // δ/(r_e cos³θ_a)) and in the value alone (the land raised).
        let (cos, tan) = (f.corner.cos, f.corner.tan());
        let slope_bound = EPS * tan * (3.0 / (cos * cos) + 1.5);
        let mut p = f;
        p.land += 10.0 * slope_bound * p.radius * cos.powi(3);
        let (v, s) = worst_jump(&p);
        assert!(v <= 1.0 && s > 5.0, "slope at 10×: {s}");
        let value_bound = EPS * f.radius * (2.0 / cos + 2.0);
        let mut p = f;
        p.rise += 10.0 * value_bound;
        let (v, s) = worst_jump(&p);
        assert!(v > 5.0 && s <= 1.0, "value at 10×: {v}");
    }

    /// At a joint, where the curvature steps, `at` reads the side its doc names, to the bit: the
    /// flank's at the round's tangency, the land's at its end, the unrelieved side at the
    /// relief's end; one unit in the last place into the other piece, that piece. And `on_edge`
    /// is exactly "not on the flank" at each joint and one unit either side of it.
    #[test]
    fn each_joint_reads_its_stated_side() {
        let mut read = 0;
        for f in forms() {
            let relieved = |s: f64| f.relief.is_some_and(|(_, end)| s < end);
            let (t, l) = (f.tangency, f.land);
            // (joint, its side's piece and relief, the other side's point, piece and relief)
            let mut sides = vec![
                (
                    t,
                    Edge::Flank,
                    relieved(t),
                    t.next_down(),
                    Edge::Round,
                    relieved(t),
                ),
                (
                    l,
                    Edge::Land,
                    relieved(l),
                    l.next_up(),
                    Edge::Round,
                    relieved(l),
                ),
            ];
            if let Some((_, end)) = f.relief {
                let e = f.edge_at(end);
                sides.push((end, e, false, end.next_down(), e, true));
            }
            for (joint, edge, relief, other, other_edge, other_relief) in sides {
                assert_eq!(f.edge_at(joint), edge);
                assert_eq!(f.at(joint), f.piece(joint, edge, relief).point());
                assert_eq!(f.edge_at(other), other_edge);
                assert_eq!(
                    f.at(other),
                    f.piece(other, other_edge, other_relief).point()
                );
                for s in [joint.next_down(), joint, joint.next_up()] {
                    assert_eq!(f.on_edge(s), f.edge_at(s) != Edge::Flank, "{s}");
                }
                read += 1;
            }
        }
        assert_eq!(read, 5 * 4 * (2 + 3 + 3));
    }

    // ---------------------------------------------------------------- 1/r_e on the round

    /// `|κ r_e − 1|` over its bound at `n` points inside the round of an unrelieved form, reading
    /// the curvature with `kappa`.
    ///
    /// The bound, `10ε`, is derived: `κ = r_e² / (r² + w²)^{3/2}` whatever `w` is, so `w`'s
    /// rounding moves nothing; `r = √(r_e² − w²)`'s rounding `δ` moves `κ` by `3δ (r/r_e)²`, at
    /// most `3ε`; `F''`, `F'` and the `3/2` power take thirteen roundings of `ε/2`, and the product
    /// `κ r_e` one more.
    fn round_misses(f: &TipForm, n: usize, kappa: impl Fn(Jet) -> f64) -> (usize, f64) {
        assert_eq!(f.relief, None);
        let mut worst = 0.0_f64;
        let mut seen = 0;
        for i in 0..n {
            let t = (i as f64 + 0.5) / n as f64;
            let s = f.land + (f.tangency - f.land) * t;
            assert!(f.land < s && s < f.tangency);
            worst = worst.max((kappa(f.jet(s)) * f.radius - 1.0).abs() / (10.0 * EPS));
            seen += 1;
        }
        (seen, worst)
    }

    fn production(j: Jet) -> f64 {
        j.point().curvature
    }

    #[test]
    fn the_curvature_is_one_over_r_e_on_the_round() {
        let mut seen = 0;
        let mut worst = 0.0_f64;
        for f in forms().into_iter().filter(|f| f.relief.is_none()) {
            let (n, w) = round_misses(&f, 1001, production);
            seen += n;
            worst = worst.max(w);
        }
        eprintln!("1/r_e: {seen} points on 20 rounds, worst {worst:.3} of the bound");
        assert_eq!(seen, 20 * 1001);
        assert!(worst <= 1.0, "{worst}");
    }

    #[test]
    fn the_round_curvature_gate_fails_its_plants() {
        let f = plain();
        // The prototype's probe: the round's midpoint, printed to four decimals.
        let naive = |kappa: &dyn Fn(Jet) -> f64| {
            let mid = f.tangency - 0.5 * f.span;
            ((kappa(f.jet(mid)) * f.radius - 1.0).abs()) < 5e-5
        };
        assert!(round_misses(&f, 1001, production).1 <= 1.0 && naive(&production));

        // A rounding passes: κ one ulp high.
        let ulp = |j: Jet| production(j) * (1.0 + EPS);
        assert!(round_misses(&f, 1001, ulp).1 <= 1.0);

        // Near miss: κ read 1e-9 high everywhere. The prototype's probe passes it.
        let high = |j: Jet| production(j) * (1.0 + 1e-9);
        assert!(naive(&high) && round_misses(&f, 1001, high).1 > 1e3);

        // F'' alone (the small-slope curvature): exact at the tangency, 1/cos³θ_a at the end.
        let small_slope = |j: Jet| j.d2;
        assert!(round_misses(&f, 1001, small_slope).1 > 1e3);

        // Ten times the bound.
        let ten = |j: Jet| production(j) * (1.0 + 100.0 * EPS);
        assert!(round_misses(&f, 1001, ten).1 > 5.0);
    }

    // ---------------------------------------------------------------- the relieved round

    /// A way of reading the curvature on a relieved round, given the form and `σ`.
    type Reading = fn(&TipForm, f64) -> f64;

    /// Every point of the relieved records that lies on the round, read with `kappa`: how many,
    /// the worst miss against the record's κ over the closed-form bound, and the least
    /// `|κ r_e − 1|` over the 1/r_e gate's bound `10ε` (above 1: nowhere 1/r_e).
    fn relieved_round(kappa: Reading) -> (usize, f64, f64, (f64, f64)) {
        let file = oracle_file();
        let (mut seen, mut worst, mut least) = (0, 0.0_f64, f64::MAX);
        let mut range = (f64::MAX, f64::MIN);
        for rec in oracle::records(&file) {
            let (i, o) = (rec.inputs, rec.outputs);
            let form = form_of(i);
            let (Some(_), Some(r_e)) = (form.tip_relief, form.edge_radius) else {
                continue;
            };
            let gear = FieldGear::from_spec(&spec_of(&i["member"])).expect("a gear");
            let f = TipForm::new(&form, gear.tip_corner().expect("a corner")).expect("a round");
            let rel = num(&rec.tol["rel"]);
            let values = o["values"].as_array().expect("values");
            for (s, v) in i["sigma"]
                .as_array()
                .expect("σ")
                .iter()
                .map(num)
                .zip(values)
            {
                if f.edge_at(s) != Edge::Round {
                    continue;
                }
                let (port, oracle) = (kappa(&f, s), num(&v[2]));
                worst = worst.max(miss(port, oracle, closed_bound(rel, oracle, true)));
                least = least.min((port * r_e - 1.0).abs() / (10.0 * EPS));
                range = (range.0.min(oracle * r_e), range.1.max(oracle * r_e));
                seen += 1;
            }
        }
        (seen, worst, least, range)
    }

    /// The five relieved records' points on the round (3, 4, 3, 3, 4).
    const ON_ROUND: usize = 17;

    fn read_production(f: &TipForm, s: f64) -> f64 {
        f.at(s).curvature
    }

    /// With a tip relief, the round's curvature is the summed form's, `F''/(1 + F'²)^{3/2}` of
    /// relief plus round: the record's κ at each of the relieved records' points on the round,
    /// and at none of them the round's own 1/r_e (reading the round alone misses every one).
    #[test]
    fn a_relieved_round_reads_the_summed_forms_curvature() {
        let (seen, worst, least, range) = relieved_round(read_production);
        eprintln!(
            "relieved round: {seen} points, worst {worst:.3} of the tolerance, |κ r_e − 1| at \
             least {least:.3e} × 10ε, κ r_e {:.4} … {:.4}",
            range.0, range.1
        );
        assert_eq!(seen, ON_ROUND);
        assert!(worst <= 1.0, "{worst}");
        assert!(least > 1.0, "{least}");
    }

    #[test]
    fn the_relieved_round_gate_fails_its_plants() {
        // The naive gate: κ r_e near 1 (the records span 0.968 … 1.003), which every plant passes.
        let naive = |kappa: Reading| {
            let (seen, _, _, _) = relieved_round(kappa);
            let file = oracle_file();
            let mut near = 0;
            for rec in oracle::records(&file) {
                let form = form_of(rec.inputs);
                let (Some(_), Some(r_e)) = (form.tip_relief, form.edge_radius) else {
                    continue;
                };
                let gear = FieldGear::from_spec(&spec_of(&rec.inputs["member"])).expect("a gear");
                let f = TipForm::new(&form, gear.tip_corner().expect("a corner")).expect("a round");
                for s in rec.inputs["sigma"].as_array().expect("σ").iter().map(num) {
                    if f.edge_at(s) == Edge::Round && (kappa(&f, s) * r_e - 1.0).abs() < 0.04 {
                        near += 1;
                    }
                }
            }
            near == seen && seen == ON_ROUND
        };
        let gate = |kappa: Reading| {
            let (seen, worst, least, _) = relieved_round(kappa);
            seen == ON_ROUND && worst <= 1.0 && least > 1.0
        };
        assert!(gate(read_production) && naive(read_production));

        // The round's own curvature, 1/r_e, as though the relief were not there.
        let own: Reading = |f, s| f.piece(s, Edge::Round, false).point().curvature;
        // Near miss: the pieces' curvatures summed, each of its own graph: 0.4 … 0.5 % above
        // 1/r_e, so a check against 1/r_e alone passes it, and up to 4 % off the summed form's.
        let summed: Reading = |f, s| {
            let (r, _) = f.relief.expect("relieved");
            f.piece(s, Edge::Round, false).point().curvature
                + relief_jet(r, f.corner, s).point().curvature
        };
        let (_, _, least, _) = relieved_round(summed);
        assert!(least > 1.0, "the pieces' sum is nowhere 1/r_e: {least}");
        // Ten times the tolerance.
        let ten: Reading = |f, s| {
            let k = f.at(s).curvature;
            k + 10.0 * closed_bound(REL, k, true)
        };
        for (name, plant) in [("own", own), ("summed", summed), ("10×", ten)] {
            assert!(naive(plant), "{name}: the naive gate passes it");
            assert!(!gate(plant), "{name}: the gate fails it");
        }
        // The small-slope curvature F'', which the naive gate catches too.
        let small: Reading = |f, s| f.jet(s).d2;
        assert!(!naive(small) && !gate(small));
        // A rounding passes.
        let ulp: Reading = |f, s| f.at(s).curvature * (1.0 + EPS);
        assert!(gate(ulp));
    }

    // ---------------------------------------------------------------- a C⁰ form refused

    fn with(radius: Option<f64>, relief: Option<Relief>) -> ToothForm {
        ToothForm {
            tip_relief: relief,
            edge_radius: radius,
        }
    }

    #[test]
    fn a_c0_form_is_refused() {
        let mut refused = 0;
        for c in corners() {
            // The smallest round floating point holds, and half of it.
            let floor = f64::MIN_POSITIVE.sqrt() / c.cos;
            for r in [None, Some(0.0), Some(-0.0), Some(1e-160), Some(0.5 * floor)] {
                assert_eq!(
                    TipForm::new(&with(r, None), c),
                    Err(FormRefused::SharpEdge),
                    "{r:?}"
                );
                refused += 1;
            }
            let least = TipForm::new(&with(Some(2.0 * floor), None), c).expect("a round");
            assert!(worst_jump(&least).0 <= 1.0 && worst_jump(&least).1 <= 1.0);
            assert!(round_misses(&least, 101, production).1 <= 1.0);

            for r in [-1e-3, f64::NAN, f64::INFINITY, 1e155] {
                assert_eq!(
                    TipForm::new(&with(Some(r), None), c),
                    Err(FormRefused::NotARadius)
                );
                refused += 1;
            }
            for (depth, length) in [
                (0.005, 0.0),
                (0.005, -0.4),
                (0.005, f64::NAN),
                (0.005, f64::INFINITY),
                (-1e-3, 0.4),
                (f64::NAN, 0.4),
                (f64::INFINITY, 0.4),
                (0.005, 1e-170),
                // The curvature 2 C sin²θ_a / L² overflowing where C and sin²θ_a / L² (1e10)
                // do not: by the factor C alone, and by the factor 2 alone (C sin²θ_a / L² 1e308).
                (1e300, 1e-5 * c.sin),
                (1e298, 1e-5 * c.sin),
            ] {
                let form = with(Some(0.1), Some(Relief { depth, length }));
                assert_eq!(TipForm::new(&form, c), Err(FormRefused::NotARelief));
                refused += 1;
            }
            // Laid: no depth, and the largest curvature above held to a finite 5e307.
            for (depth, length) in [(0.0, 0.4), (0.25e298, 1e-5 * c.sin)] {
                let form = with(Some(0.1), Some(Relief { depth, length }));
                assert!(TipForm::new(&form, c).is_ok(), "{depth} {length}");
            }
        }
        assert_eq!(refused, 5 * (5 + 4 + 10));
    }

    /// The near miss the refusal exists for: `r_e = 1e-160` is positive and finite, so a check of
    /// `r_e > 0` lays it, and its round's end squares to a subnormal: laid anyway, it is not C¹.
    #[test]
    fn a_round_floating_point_cannot_hold_is_not_c1() {
        let c = corners()[0];
        let good = TipForm::new(&with(Some(0.1), None), c).expect("a round");
        let tiny = 1e-160;
        let laid = TipForm {
            radius: tiny,
            span: tiny * c.sin,
            tangency: tiny * c.sin / (1.0 + c.cos),
            land: tiny * c.sin / (1.0 + c.cos) - tiny * c.sin,
            rise: tiny * (1.0 - c.cos),
            ..good
        };
        assert!(worst_jump(&laid).1 > 1e3, "{:?}", worst_jump(&laid));
    }

    #[test]
    fn a_tip_on_the_base_circle_has_no_corner() {
        assert_eq!(
            TipCorner::involute(10.0, 10.0, 0.0),
            Err(FormRefused::NoCorner)
        );
        assert_eq!(
            TipCorner::involute(9.0, 10.0, 0.0),
            Err(FormRefused::NoCorner)
        );
        assert_eq!(
            TipCorner::involute(10.0, 9.0, f64::NAN),
            Err(FormRefused::NoCorner)
        );
        let c = TipCorner::involute(10.0 * (1.0 + EPS), 10.0, 0.0).expect("just above");
        assert!(c.cos > 0.0 && c.cos < 1e-7 && c.sin <= 1.0);
    }
}
