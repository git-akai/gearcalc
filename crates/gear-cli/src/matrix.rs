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

use gear_core::strength::{root_section_with, CriticalSection, StressConcentration};
use gear_core::{GearParams, Tooth};

/// One cell of the matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    pub section: CriticalSection,
    pub concentration: StressConcentration,
}

impl Model {
    pub fn name(self) -> &'static str {
        match (self.section, self.concentration) {
            (CriticalSection::TangentAngle, StressConcentration::None) => "30° · Y_F only",
            (CriticalSection::TangentAngle, StressConcentration::Iso6336) => "30° · Y_F·Y_S",
            (CriticalSection::LewisParabola, StressConcentration::None) => "parabola · Y_F only",
            (CriticalSection::LewisParabola, StressConcentration::Iso6336) => "parabola · Y_F·Y_S",
        }
    }

    /// The bending factor this model predicts, or `None` where the geometry has
    /// no root section at all.
    pub fn evaluate(self, p: GearParams) -> Option<f64> {
        let g = Tooth::new(p);
        root_section_with(&g, g.u_tip, self.section)
            .and_then(|s| s.bending_factor(self.concentration))
    }
}

pub const MATRIX: [Model; 4] = [
    Model {
        section: CriticalSection::TangentAngle,
        concentration: StressConcentration::None,
    },
    Model {
        section: CriticalSection::TangentAngle,
        concentration: StressConcentration::Iso6336,
    },
    Model {
        section: CriticalSection::LewisParabola,
        concentration: StressConcentration::None,
    },
    Model {
        section: CriticalSection::LewisParabola,
        concentration: StressConcentration::Iso6336,
    },
];

/// A population of designs spanning the space the tool is meant to cover.
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
pub fn continuity_in_tooth_count(model: Model) -> (f64, u32) {
    let mut worst = 0.0_f64;
    let mut at = 0;
    let mut previous: Option<(u32, f64)> = None;
    for teeth in 9..=400u32 {
        let Some(v) = model.evaluate(GearParams {
            teeth,
            ..Default::default()
        }) else {
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
pub fn continuity_in_profile_shift(model: Model, teeth: u32) -> f64 {
    let mut worst = 0.0_f64;
    let mut previous: Option<f64> = None;
    let steps = 4000;
    for i in 0..=steps {
        #[allow(clippy::cast_precision_loss)]
        let x = -0.4 + 1.2 * (i as f64) / f64::from(steps);
        let Some(v) = model.evaluate(GearParams {
            teeth,
            profile_shift: x,
            ..Default::default()
        }) else {
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
pub fn rank_correlation(a: Model, b: Model, pop: &[GearParams]) -> (f64, usize) {
    let pairs: Vec<(f64, f64)> = pop
        .iter()
        .filter_map(|p| Some((a.evaluate(*p)?, b.evaluate(*p)?)))
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
pub fn gradient_agreement(a: Model, b: Model, pop: &[GearParams], min_effect: f64) -> [f64; 3] {
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
            let (Some(a1), Some(a0)) = (a.evaluate(up), a.evaluate(dn)) else {
                continue;
            };
            let (Some(b1), Some(b0)) = (b.evaluate(up), b.evaluate(dn)) else {
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
pub fn divergence(p: GearParams) -> Option<f64> {
    let vals: Vec<f64> = MATRIX.iter().filter_map(|m| m.evaluate(p)).collect();
    if vals.len() < MATRIX.len() {
        return None;
    }
    let lo = vals.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Some((hi - lo) / lo)
}
