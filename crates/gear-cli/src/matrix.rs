//! A precision study of the bending-model matrix.
//!
//! For a comparative design tool, absolute accuracy matters less than whether
//! the model **ranks designs correctly and responds correctly to a change**.
//! A model that is 15% high everywhere is usable; one that is right on average
//! but has a kink, a reversal, or a gradient of the wrong sign will send an
//! optimiser — or a designer — the wrong way.
//!
//! So this measures four things across the matrix of options, in order of how
//! badly each would hurt:
//!
//! 1. **Continuity.** Any jump in the output for a smooth change of input is
//!    disqualifying. The parabola construction is the suspect here: its tangency
//!    migrates from the fillet to the flank as teeth get larger, and if the two
//!    branches do not meet, the seam is a cliff in the middle of the design space.
//! 2. **Gradient agreement.** Whether the models agree on which way to move.
//! 3. **Rank agreement.** Whether they order a population of designs the same way.
//! 4. **Divergence map.** Where in the space the choice actually matters.

use gear_core::ring::{Cutter, Ring};
use gear_core::strength::{
    root_section_with, CriticalSection, RootSection, RootStressModel, ToothOutline,
};
use gear_core::{GearParams, Tooth};

/// **A member the study can be run on**, external or internal.
///
/// The four studies below ask nothing that is particular to a rack-cut tooth —
/// they want a bending factor for a set of parameters, and a way to nudge one
/// parameter — so they are written against this rather than against `Tooth`. A
/// ring is then the same four measurements rather than a second harness beside
/// them, which is the only way the two answers can be compared at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Member {
    /// Cut by a rack.
    External,
    /// Cut by a shaper. The tool is [`Cutter::default`] unless a study varies it.
    Internal,
}

impl Member {
    pub fn name(self) -> &'static str {
        match self {
            Self::External => "external",
            Self::Internal => "ring",
        }
    }

    /// The tangent angle this member's construction uses, degrees.
    pub fn tangent_angle(self) -> f64 {
        match self {
            Self::External => gear_core::strength::TANGENT_ANGLE_DEG,
            Self::Internal => gear_core::strength::TANGENT_ANGLE_INTERNAL_DEG,
        }
    }

    /// This member's critical section under one construction, loaded at its tip.
    ///
    /// Tip loading for both: without a mate there is no outer point of
    /// single-pair contact, so the tip is the one point both members define
    /// the same way, which is what makes the two populations comparable.
    pub fn section(self, p: GearParams, method: CriticalSection) -> Option<RootSection> {
        match self {
            Self::External => {
                let g = Tooth::new(p);
                root_section_with(&g, g.u_tip, method)
            }
            Self::Internal => {
                let r = Ring::cut_by(&p, &Cutter::default());
                root_section_with(&r, r.u_tip, method)
            }
        }
    }
}

/// One cell of the matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    pub section: CriticalSection,
    pub concentration: RootStressModel,
}

impl Model {
    /// The model's name. The tangent construction is named by the angle it
    /// actually takes, which differs by member — 30° external, 60° internal —
    /// so the caller says which population it is describing.
    pub fn name(self, on: Member) -> String {
        let section = match self.section {
            CriticalSection::TangentAngle => format!("{:.0}°", on.tangent_angle()),
            CriticalSection::LewisParabola => "parabola".to_string(),
        };
        let concentration = match self.concentration {
            RootStressModel::FormFactorOnly => "Y_F only",
            RootStressModel::Iso6336 => "Y_F·Y_S",
            RootStressModel::DolanBroghamer => "Y_F·K_f",
        };
        format!("{section} · {concentration}")
    }

    /// The bending factor this model predicts, or `None` where the geometry has
    /// no root section at all.
    pub fn evaluate(self, on: Member, p: GearParams) -> Option<f64> {
        on.section(p, self.section)
            .and_then(|s| s.bending_factor(self.concentration))
    }
}

/// The models compared, and note that two of them are **coherent sets** rather
/// than free combinations: the tangent section carries ISO's `Y_S`, which is
/// fitted to it, and the parabola carries Dolan and Broghamer's `K_f`, which is
/// what Savage's construction carries. The bare form factors are the controls.
pub const MATRIX: [Model; 4] = [
    Model {
        section: CriticalSection::TangentAngle,
        concentration: RootStressModel::FormFactorOnly,
    },
    Model {
        section: CriticalSection::TangentAngle,
        concentration: RootStressModel::Iso6336,
    },
    Model {
        section: CriticalSection::LewisParabola,
        concentration: RootStressModel::FormFactorOnly,
    },
    Model {
        section: CriticalSection::LewisParabola,
        concentration: RootStressModel::DolanBroghamer,
    },
];

/// A population of designs spanning the space the tool is meant to cover.
///
/// A ring's is a different space and not a subset: it starts where a shaper can
/// cut one at all, and it carries no 14.5° arm because the default cutter's tip
/// will not reach at that angle.
pub fn population_for(member: Member) -> Vec<GearParams> {
    match member {
        Member::External => population(),
        Member::Internal => {
            let mut v = Vec::new();
            for teeth in [40u32, 48, 60, 72, 90, 120, 160, 220] {
                for xi in -4i32..=5 {
                    for pressure_angle in [20.0_f64, 25.0] {
                        v.push(GearParams {
                            teeth,
                            profile_shift: f64::from(xi) * 0.1,
                            pressure_angle,
                            ..Default::default()
                        });
                    }
                }
            }
            v
        }
    }
}

/// The external population.
pub fn population() -> Vec<GearParams> {
    let mut v = Vec::new();
    for teeth in [9u32, 11, 13, 15, 17, 20, 24, 30, 40, 55, 75, 100, 140] {
        for xi in -4i32..=8 {
            for root_radius in [0.10_f64, 0.25, 0.38] {
                for pressure_angle in [14.5_f64, 20.0, 25.0] {
                    v.push(GearParams {
                        teeth,
                        profile_shift: f64::from(xi) * 0.1,
                        root_radius,
                        pressure_angle,
                        ..Default::default()
                    });
                }
            }
        }
    }
    v
}

/// Largest relative jump in the output for a one-step change in tooth count.
///
/// A smooth model should give a small, steadily shrinking step. A branch seam
/// shows up here as a spike that does not shrink when the step is refined.
pub fn continuity_in_tooth_count(model: Model, on: Member) -> (f64, u32) {
    let mut worst = 0.0_f64;
    let mut at = 0;
    let mut previous: Option<(u32, f64)> = None;
    let from = match on {
        Member::External => 9u32,
        Member::Internal => 32,
    };
    for teeth in from..=400u32 {
        let Some(v) = model.evaluate(
            on,
            GearParams {
                teeth,
                ..Default::default()
            },
        ) else {
            continue;
        };
        if let Some((pz, pv)) = previous {
            if teeth == pz + 1 {
                let step = ((v - pv) / pv).abs();
                if step > worst {
                    worst = step;
                    at = teeth;
                }
            }
        }
        previous = Some((teeth, v));
    }
    (worst, at)
}

/// Continuity across a continuous parameter, where a true discontinuity cannot
/// hide behind the integer step of a tooth count.
///
/// Sweeps profile shift finely at a tooth count near the fillet/flank seam.
pub fn continuity_in_profile_shift(model: Model, on: Member, teeth: u32) -> f64 {
    let mut worst = 0.0_f64;
    let mut previous: Option<f64> = None;
    let steps = 4000;
    for i in 0..=steps {
        #[allow(clippy::cast_precision_loss)]
        let x = -0.4 + 1.2 * (i as f64) / f64::from(steps);
        let Some(v) = model.evaluate(
            on,
            GearParams {
                teeth,
                profile_shift: x,
                ..Default::default()
            },
        ) else {
            continue;
        };
        if let Some(pv) = previous {
            worst = worst.max(((v - pv) / pv).abs());
        }
        previous = Some(v);
    }
    worst
}

/// Spearman rank correlation between two models over the population.
///
/// This is the question "would these two ever pick a different design?".
pub fn rank_correlation(a: Model, b: Model, on: Member, pop: &[GearParams]) -> (f64, usize) {
    let pairs: Vec<(f64, f64)> = pop
        .iter()
        .filter_map(|p| Some((a.evaluate(on, *p)?, b.evaluate(on, *p)?)))
        .collect();
    let n = pairs.len();
    if n < 3 {
        return (f64::NAN, n);
    }
    let rank = |get: &dyn Fn(&(f64, f64)) -> f64| {
        let mut idx: Vec<usize> = (0..n).collect();
        idx.sort_by(|&i, &j| {
            get(&pairs[i])
                .partial_cmp(&get(&pairs[j]))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut r = vec![0.0; n];
        for (place, &i) in idx.iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            {
                r[i] = place as f64;
            }
        }
        r
    };
    let ra = rank(&|p| p.0);
    let rb = rank(&|p| p.1);
    #[allow(clippy::cast_precision_loss)]
    let nf = n as f64;
    let mean = (nf - 1.0) / 2.0;
    let (mut num, mut da, mut db) = (0.0, 0.0, 0.0);
    for i in 0..n {
        let (u, v) = (ra[i] - mean, rb[i] - mean);
        num += u * v;
        da += u * u;
        db += v * v;
    }
    (num / (da.sqrt() * db.sqrt()), n)
}

/// Do two models agree on which way a change moves the answer?
///
/// Returns the fraction of the population where the sign of the local gradient
/// agrees, for each of the three levers a designer actually has.
///
/// `min_effect` screens out cases where the lever barely moves the answer at
/// all: near a turning point the sign is noise, and counting it as disagreement
/// overstates the problem. Expressed as a relative change in the bending factor
/// across the step, so it means "the lever visibly did something".
pub fn gradient_agreement(
    a: Model,
    b: Model,
    on: Member,
    pop: &[GearParams],
    min_effect: f64,
) -> [f64; 3] {
    let levers: [fn(GearParams, f64) -> GearParams; 3] = [
        |mut p, d| {
            p.profile_shift += d;
            p
        },
        |mut p, d| {
            p.root_radius = (p.root_radius + d * 0.5).max(0.01);
            p
        },
        |mut p, d| {
            p.dedendum += d;
            p
        },
    ];
    let mut out = [0.0; 3];
    for (k, lever) in levers.iter().enumerate() {
        let (mut agree, mut total) = (0usize, 0usize);
        for p in pop {
            let h = 0.02;
            let up = lever(*p, h);
            let dn = lever(*p, -h);
            let (Some(a1), Some(a0)) = (a.evaluate(on, up), a.evaluate(on, dn)) else {
                continue;
            };
            let (Some(b1), Some(b0)) = (b.evaluate(on, up), b.evaluate(on, dn)) else {
                continue;
            };
            let (ga, gb) = (a1 - a0, b1 - b0);
            // both models must agree the lever does something material
            if (ga / a0).abs() < min_effect || (gb / b0).abs() < min_effect {
                continue;
            }
            total += 1;
            if (ga > 0.0) == (gb > 0.0) {
                agree += 1;
            }
        }
        #[allow(clippy::cast_precision_loss)]
        {
            out[k] = if total == 0 {
                f64::NAN
            } else {
                agree as f64 / total as f64
            };
        }
    }
    out
}

/// Spread between the matrix's extremes, as a fraction of the lowest.
pub fn divergence(on: Member, p: GearParams) -> Option<f64> {
    let vals: Vec<f64> = MATRIX.iter().filter_map(|m| m.evaluate(on, p)).collect();
    if vals.len() < MATRIX.len() {
        return None;
    }
    let lo = vals.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Some((hi - lo) / lo)
}

/// **Where the two constructions part on one kind of member**, and what that
/// does to the factor the stress is proportional to.
#[derive(Clone, Copy, Debug, Default)]
pub struct Parting {
    /// Designs where both constructions returned a section.
    pub n: usize,
    /// ...of which the parabola's tangency landed on the involute **flank**
    /// rather than the fillet.
    ///
    /// The number worth watching. On a flank tangency the chord `s_Fn` is read
    /// across a point on the flank, and `ρ_F` falls back to the fillet junction
    /// — a documented and continuous fallback, but a different measurement from
    /// the one the tangent construction makes, and the fraction says how much of
    /// the space is in it.
    pub on_flank: usize,
    /// `Y_F(parabola) / Y_F(tangent)`: least, greatest, mean.
    pub form: [f64; 3],
    /// The same for the two **coherent sets** — `Y_F·K_f` over `Y_F·Y_S`, each
    /// section carrying the notch factor fitted to it. This, not `Y_F`, is what
    /// a bending stress is proportional to, so it is the number a designer
    /// feels.
    pub factor: [f64; 3],
    /// ISO's notch parameter `q_s` under each section, mean: parabola, tangent.
    ///
    /// Reported for the tangent section because that is where `Y_S` is used and
    /// where the band applies; reported for the parabola because it is the
    /// evidence that `Y_S` does **not** belong there.
    pub notch: [f64; 2],
    /// ...and how many designs leave `Y_S`'s `1 ≤ q_s < 8` band, each way.
    /// Only the tangent column is a live concern; `K_f` states no band.
    pub notch_out: [usize; 2],
}

/// Measure it over a population.
pub fn parting(on: Member, pop: &[GearParams]) -> Parting {
    let mut out = Parting {
        form: [f64::INFINITY, f64::NEG_INFINITY, 0.0],
        factor: [f64::INFINITY, f64::NEG_INFINITY, 0.0],
        ..Parting::default()
    };
    for p in pop {
        let (Some(para), Some(tan)) = (
            on.section(*p, CriticalSection::LewisParabola),
            on.section(*p, CriticalSection::TangentAngle),
        ) else {
            continue;
        };
        // Each section with the notch factor fitted to it: the parabola with
        // Dolan and Broghamer's, the tangent with ISO's. Comparing the two
        // *sets* is the question; comparing one notch model across two sections
        // is what mixing looks like.
        let (Some(fa), Some(ft)) = (
            para.bending_factor(RootStressModel::DolanBroghamer),
            tan.bending_factor(RootStressModel::Iso6336),
        ) else {
            continue;
        };
        out.n += 1;
        out.on_flank += usize::from(para.tangency_on_flank);
        let r = para.form_factor / tan.form_factor;
        let f = fa / ft;
        out.form[0] = out.form[0].min(r);
        out.form[1] = out.form[1].max(r);
        out.form[2] += r;
        out.factor[0] = out.factor[0].min(f);
        out.factor[1] = out.factor[1].max(f);
        out.factor[2] += f;
        out.notch[0] += para.notch_parameter;
        out.notch[1] += tan.notch_parameter;
        out.notch_out[0] += usize::from(!para.notch_parameter_in_range());
        out.notch_out[1] += usize::from(!tan.notch_parameter_in_range());
    }
    if out.n > 0 {
        #[allow(clippy::cast_precision_loss)]
        let n = out.n as f64;
        out.form[2] /= n;
        out.factor[2] /= n;
        out.notch[0] /= n;
        out.notch[1] /= n;
    }
    out
}

/// **Which end of a ring's tooth is the thick one?**
///
/// The cantilever the whole bending model rests on assumes a tooth that narrows
/// toward the load, and it is worth measuring rather than asserting which way a
/// ring's runs. Read through [`ToothOutline`] — the crate's own flank, in the
/// crate's own frame — so no sign convention is re-derived here to be got wrong.
///
/// Returns `(radius, chordal thickness)` sampled along the generated flank from
/// the tip outward, mm.
pub fn ring_flank_thickness(p: GearParams, samples: usize) -> Vec<(f64, f64)> {
    let r = Ring::cut_by(&p, &Cutter::default());
    let (lo, hi) = ToothOutline::flank_bracket(&r);
    (0..=samples)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let u = lo + (hi - lo) * (i as f64) / (samples as f64);
            let (q, _) = ToothOutline::flank_at(&r, u);
            (f64::hypot(q[0], q[1]), 2.0 * q[0].abs())
        })
        .collect()
}

/// **The fillet radius the notch factor is fed**, two ways of reading it.
///
/// Savage, Rubadeux & Coe define `ρ_f` in their stress concentration factor as
/// "the **minimum** radius of curvature of the fillet curve" — a property of the
/// whole fillet, well defined wherever the parabola's tangency happened to land.
/// This crate reads the curvature at the fillet **junction** when the tangency is
/// on the flank, which is a different quantity, and `q_s = s_Fn/(2ρ_F)` is
/// linear in it.
///
/// Returns `(at the junction, minimum over the fillet, where the minimum is as a
/// fraction of the bracket)`, mm.
pub fn fillet_radius_readings(
    on: Member,
    p: GearParams,
    samples: usize,
) -> Option<(f64, f64, f64)> {
    let read = |g: &dyn ToothOutline| {
        let (lo, hi) = g.fillet_bracket();
        let at_junction = g.fillet_curvature(g.fillet_junction());
        let mut best = (f64::INFINITY, 0.0);
        for i in 0..=samples {
            #[allow(clippy::cast_precision_loss)]
            let t = i as f64 / samples as f64;
            let s = lo + (hi - lo) * t;
            let r = g.fillet_curvature(s);
            if r.is_finite() && r < best.0 {
                best = (r, t);
            }
        }
        (at_junction.is_finite() && best.0.is_finite()).then_some((at_junction, best.0, best.1))
    };
    match on {
        Member::External => read(&Tooth::new(p)),
        Member::Internal => {
            let r = Ring::cut_by(&p, &Cutter::default());
            r.fillet.is_some().then(|| read(&r)).flatten()
        }
    }
}
