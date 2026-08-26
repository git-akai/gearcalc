//! A gear whose profile shift varies with angular position — and every ordinary
//! gear, which is the same construction with the variation set to zero.
//!
//! # What the feature is
//!
//! `x(θ) = x̄ + Δx cos θ`, maximum at 0° and minimum at 180°: what a hob moving
//! radially in and out once per revolution produces. The tip and root envelopes
//! come out eccentric by `e = m Δx` while the **pitch and base circles stay on
//! the axis**, so the body moves eccentrically at a genuinely constant ratio.
//! That is the whole point, and §4.10 derives it; this module builds it.
//!
//! # Why it needs no new profile mathematics
//!
//! Constant ratio requires every driving flank to be a pure involute of one
//! base circle at a single seat — which is exactly what the generator produces
//! for one scalar `x`. So a per-tooth constant `x` is not an approximation of
//! the specification, it **is** the specification, and [`crate::profile::Gear`]
//! is used unchanged. What this module adds is *assembly*: which shift each
//! tooth gets, and where each tooth is seated.
//!
//! # The unavoidable error, and λ
//!
//! Profile shift moves a tooth's two flanks in **opposite** directions, so:
//!
//! > if both flank sets were uniformly spaced, every tooth's angular thickness
//! > would be the difference of two constants — the same for every tooth.
//!
//! Uniform spacing on both flanks forces uniform thickness. A gear with varying
//! thickness therefore cannot be exactly conjugate both ways, and the error can
//! only be *distributed*. Seating tooth `k` at `2πk/z + λ(ψ̄ − ψ_k)` puts the
//! drive flank's deviation at `(1 − λ)(ψ_k − ψ̄)` and the coast flank's at
//! `−(1 + λ)(ψ_k − ψ̄)` — one algebraic step, and the reason the two errors scale
//! as `|1 − λ|` and `|1 + λ|`.
//!
//! # An extension, not a branch
//!
//! There is one outline path in this crate and it is [`Eccentric::outline`].
//! `Δx = 0` gives every tooth the same shift, so the distinct-tooth list has one
//! entry, every seat is `2πk/z + 0.0`, and the result is bit-identical to the
//! z-fold replication it replaces — gated, not hoped for. `λ` has no effect
//! there either, since there is nothing for it to correct towards.

use crate::params::GearParams;
use crate::profile::Gear;

/// A gear assembled tooth by tooth.
///
/// Ordinary gears included: they are the ones whose teeth all came out the same.
#[derive(Clone, Debug)]
pub struct Eccentric {
    /// The distinct teeth. One entry for a concentric gear, `⌈z/2⌉ + 1` for an
    /// eccentric one — teeth `k` and `z − k` take the same shift, since `cos` is
    /// even about the axis of the variation.
    teeth: Vec<Gear>,
    /// Which distinct tooth each of the `z` positions uses.
    which: Vec<usize>,
    /// Where each tooth's centreline sits, radians.
    seat: Vec<f64>,
    /// The gear at the mean shift — what every scalar output is quoted from, and
    /// what the whole gear is when the variation is zero.
    mean: Gear,
}

impl Eccentric {
    /// Build from parameters. The angular shift and the indexing offset are read
    /// from them; everything else is the ordinary single-gear construction.
    ///
    /// # Panics
    ///
    /// Never: a zero tooth count is guarded to one, as elsewhere.
    #[must_use]
    pub fn new(params: GearParams) -> Self {
        let z = params.teeth.max(1);
        let mean = Gear::new(params);

        // The shift each tooth is cut at. Written so that `Δx = 0` gives
        // `x + 0.0`, which is `x` exactly — the equality below then collapses
        // every tooth onto one, which is what makes an ordinary gear cost what
        // it always did.
        let shift_at = |k: u32| {
            // Folded to the near half of the revolution, so teeth `k` and
            // `z − k` are given the *same* angle rather than two that agree
            // mathematically and not in floating point. `cos(τ − t)` is not
            // bit-identical to `cos t`, and the mirror pairs would then each
            // generate their own tooth — correct, but ⌈z/2⌉+1 turns into z.
            let theta = std::f64::consts::TAU * f64::from(k.min(z - k)) / f64::from(z);
            params.profile_shift + params.angular_shift * theta.cos()
        };

        // Distinct shifts, by exact equality. Nothing is quantised: two teeth
        // share a gear only if their shift is the *same number*, which for a
        // concentric gear is all of them and for an eccentric one is the mirror
        // pairs about the variation's axis.
        let mut shifts: Vec<f64> = Vec::new();
        let mut which = Vec::with_capacity(z as usize);
        for k in 0..z {
            let x = shift_at(k);
            let at = shifts.iter().position(|&s| s == x).unwrap_or_else(|| {
                shifts.push(x);
                shifts.len() - 1
            });
            which.push(at);
        }
        let teeth: Vec<Gear> = shifts
            .iter()
            .map(|&x| {
                Gear::new(GearParams {
                    profile_shift: x,
                    ..params
                })
            })
            .collect();

        // Seats. `ψ_b` is the angular half-thickness at the base circle — the
        // seat of the flank — and the correction is towards the mean tooth's.
        let seat = (0..z)
            .map(|k| {
                let base = std::f64::consts::TAU * f64::from(k) / f64::from(z);
                base + params.index_offset * (mean.psi_b - teeth[which[k as usize]].psi_b)
            })
            .collect();

        Self {
            teeth,
            which,
            seat,
            mean,
        }
    }

    /// The gear at the mean shift.
    #[must_use]
    pub fn mean(&self) -> &Gear {
        &self.mean
    }

    /// How many distinct teeth had to be generated.
    ///
    /// One for a concentric gear. Reported because it is the cost of the
    /// feature, and because it going above one is exactly what "this gear is
    /// eccentric" means.
    #[must_use]
    pub fn distinct_teeth(&self) -> usize {
        self.teeth.len()
    }

    /// The tooth at position `k`, and where it is seated.
    #[must_use]
    pub fn tooth(&self, k: usize) -> (&Gear, f64) {
        let i = k % self.which.len();
        (&self.teeth[self.which[i]], self.seat[i])
    }

    /// The whole outline, closed, as `[x, y]` in the gear's own frame.
    ///
    /// `per_tooth` is the point budget for one tooth, as for the single-gear
    /// generator it replaces.
    #[must_use]
    pub fn outline(&self, per_tooth: usize) -> Vec<[f64; 2]> {
        // A virtual spur gear has a fractional tooth count and exists only to be
        // measured; replicating it would draw a shape whose teeth do not close.
        // Caught in development rather than emitted as a plausible wrong outline.
        debug_assert!(
            (self.mean.z - f64::from(self.mean.params.teeth)).abs() < 1e-12,
            "outline() called on a virtual gear (z = {}); it exists to be measured, not drawn",
            self.mean.z
        );

        // Each distinct tooth's half-profile, mirrored into a full one. Computed
        // once per distinct tooth rather than once per position, which is what
        // keeps a concentric gear at exactly one generation.
        let halves: Vec<(Vec<f64>, Vec<f64>)> = self
            .teeth
            .iter()
            .map(|g| {
                let (r, th) = g.half_profile((per_tooth / 2).max(8));
                let mut r_full: Vec<f64> = r.iter().rev().copied().collect();
                let mut th_full: Vec<f64> = th.iter().rev().map(|t| -t).collect();
                r_full.extend_from_slice(&r[1..]);
                th_full.extend_from_slice(&th[1..]);
                (r_full, th_full)
            })
            .collect();

        let mut out = Vec::with_capacity(halves[0].0.len() * self.which.len() + 1);
        for (k, &i) in self.which.iter().enumerate() {
            let (r_full, th_full) = &halves[i];
            let base = self.seat[k];
            for (rr, tt) in r_full.iter().zip(th_full) {
                let a = base + tt;
                out.push([rr * a.cos(), rr * a.sin()]);
            }
        }
        if let Some(&first) = out.first() {
            out.push(first);
        }
        out
    }
}

impl Eccentric {
    /// The export outline: adaptively subdivided to a chord tolerance rather
    /// than to a point budget.
    ///
    /// The DXF's own generator, and the second place a gear gets drawn. It had
    /// been the one path an eccentric gear could slip through unnoticed — it
    /// replicated a single tooth `z` times, so the export would have been a
    /// **concentric** gear with no complaint. Routed here for the same reason
    /// [`Self::outline`] is: one assembly, and the ordinary gear is its `Δx = 0`.
    #[must_use]
    pub fn outline_adaptive(&self, chord_tolerance: f64) -> Vec<crate::outline::Vertex> {
        let mut out = Vec::new();
        for (k, &i) in self.which.iter().enumerate() {
            self.teeth[i].tooth_outline(chord_tolerance, self.seat[k], &mut out);
        }
        out
    }
}

/// What an eccentric gear costs, and what it varies over.
///
/// Every figure here is zero or degenerate for a concentric gear, which is what
/// makes it safe to report unconditionally.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Variation {
    /// `e = m Δx`, mm — how far the tip and root envelopes are displaced.
    pub eccentricity: f64,
    /// How far the tip envelope departs from a true displaced circle, mm.
    ///
    /// The envelope is a limaçon, not a circle, and the departure has the closed
    /// form `e²/2ρ`. It is what makes "nearly a shifted circle" a measurement
    /// rather than a hope: 0.5 µm at `e` = 0.1 mm, 13 µm at 0.5 mm.
    pub circle_departure: f64,
    /// Tip radius, smallest and largest around the revolution, mm.
    pub tip_radius: [f64; 2],
    /// Root radius, smallest and largest, mm.
    pub root_radius: [f64; 2],
    /// Transverse tooth thickness at the reference circle, smallest and
    /// largest, mm.
    pub tooth_thickness: [f64; 2],
    /// Tooth thickness at the **base** circle, smallest and largest, mm.
    ///
    /// Where the eccentricity actually lives: it is the flanks that carry it,
    /// and their seat is a base-circle quantity. 339 µm of spread at
    /// `e` = 0.25 mm, z = 17.
    pub base_thickness: [f64; 2],
    /// Single-flank pitch error on the **drive** flank, mm at the base circle.
    ///
    /// The largest departure of adjacent flank spacing from uniform. Scales as
    /// `|1 − λ|`, so `λ = 1` drives it to zero — at the cost of the next one.
    pub drive_pitch_error: f64,
    /// The same on the **coast** flank. Scales as `|1 + λ|`.
    ///
    /// Reversing the drive makes these the driving flanks, which is why `λ = 1`
    /// buys a one-way component: it is exactly conjugate forward and twice
    /// E2's error in reverse.
    pub coast_pitch_error: f64,
    /// Accumulated (index) error on each flank, mm at the base circle — the
    /// full swing of the departure from uniform spacing, not its tooth-to-tooth
    /// difference.
    pub drive_index_error: f64,
    /// The same on the coast flank.
    pub coast_index_error: f64,
}

impl Eccentric {
    /// The spread of everything that varies around the revolution.
    #[must_use]
    pub fn variation(&self) -> Variation {
        let p = self.mean.params;
        let z = p.teeth.max(1) as usize;

        // The flank seats, as they actually end up. `ψ_b` is the tooth's angular
        // half-thickness at the base circle, so the right flank of tooth k sits
        // at `seat_k + ψ_k` and the left at `seat_k − ψ_k`.
        // The seat's departure from ideal, **grouped so the cancellation happens
        // first**. `(seat + ψ) − ideal` and `(seat − ideal) + ψ` are the same
        // number and not the same arithmetic: the first adds ψ to something near
        // 2π and then subtracts it back, which costs a few ulps that differ from
        // tooth to tooth. That is enough to make a concentric gear report 8e-15
        // mm of pitch error — a rounding residual dressed as a measurement.
        let departure = |k: usize, side: f64| {
            let (g, seat) = self.tooth(k);
            #[allow(clippy::cast_precision_loss)]
            let ideal = std::f64::consts::TAU * k as f64 / z as f64;
            (seat - ideal) + side * g.psi_b
        };
        // Departure from uniform spacing: the flank's seat less the ideal
        // `2πk/z`. **Not** centred on its own mean — both outputs below are
        // ranges, and a range does not care about a constant offset. Subtracting
        // one anyway left a concentric gear reporting 8e-15 mm of pitch error,
        // which is a rounding residual wearing the clothes of a measurement.
        let departures = |side: f64| (0..z).map(|k| departure(k, side)).collect::<Vec<f64>>();
        // Adjacent-flank spacing error, and the accumulated swing.
        let errors = |side: f64| {
            let d = departures(side);
            // Both as **ranges** — the full swing from the tightest pitch to
            // the widest — rather than as amplitudes about the mean. That is
            // what a gear chart shows and what §4.10's tabulated figures are.
            let range = |v: &[f64]| {
                let (lo, hi) = v
                    .iter()
                    .fold((f64::MAX, f64::MIN), |(l, h), &x| (l.min(x), h.max(x)));
                hi - lo
            };
            let steps: Vec<f64> = (0..z).map(|k| d[(k + 1) % z] - d[k]).collect();
            (range(&steps) * self.mean.rb, range(&d) * self.mean.rb)
        };
        let (drive_pitch, drive_index) = errors(1.0);
        let (coast_pitch, coast_index) = errors(-1.0);

        let span = |f: fn(&Gear) -> f64| {
            self.teeth
                .iter()
                .fold((f64::MAX, f64::MIN), |(l, h), g| (l.min(f(g)), h.max(f(g))))
        };
        let (ra_lo, ra_hi) = span(|g| g.ra);
        let (rf_lo, rf_hi) = span(|g| g.rf);
        let (st_lo, st_hi) = span(|g| 2.0 * g.r * g.psi_p);
        let (sb_lo, sb_hi) = span(|g| 2.0 * g.rb * g.psi_b);

        let e = p.module * p.angular_shift.abs();
        Variation {
            eccentricity: e,
            // `e²/2ρ`, with the tip circle as the curvature it departs from.
            circle_departure: if self.mean.ra > 0.0 {
                e * e / (2.0 * self.mean.ra)
            } else {
                0.0
            },
            tip_radius: [ra_lo, ra_hi],
            root_radius: [rf_lo, rf_hi],
            tooth_thickness: [st_lo, st_hi],
            base_thickness: [sb_lo, sb_hi],
            drive_pitch_error: drive_pitch,
            coast_pitch_error: coast_pitch,
            drive_index_error: drive_index,
            coast_index_error: coast_index,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// **An ordinary gear comes out bit-identical to the construction this
    /// replaced.**
    ///
    /// The standing bar for the project: a new case is added by finding the
    /// parameter whose degenerate value reproduces the old one, and the reuse is
    /// gated on the old case being *identical* rather than close. The old
    /// algorithm is written out here rather than referred to, because it no
    /// longer exists to be called — generate one half-tooth, mirror it, and
    /// replicate it `z` times at `2πk/z`.
    ///
    /// Bit-for-bit, not to a tolerance. Anything less would let a reordering of
    /// the arithmetic through, and the whole claim is that the arithmetic is the
    /// same arithmetic.
    #[test]
    fn a_concentric_gear_is_drawn_exactly_as_it_always_was() {
        for teeth in [9_u32, 17, 43, 200] {
            for shift in [-0.4_f64, 0.0, 0.7] {
                for helix in [0.0_f64, 22.5] {
                    for lambda in [0.0_f64, 1.0, -3.0] {
                        let params = GearParams {
                            teeth,
                            profile_shift: shift,
                            helix_angle: helix,
                            // λ must not matter when there is no variation for
                            // it to correct, so it is swept here rather than
                            // left at its default.
                            index_offset: lambda,
                            ..Default::default()
                        };
                        let g = Gear::new(params);
                        let per_tooth = 120;

                        // ---- the construction as it was, in full.
                        let (r, th) = g.half_profile((per_tooth / 2).max(8));
                        let mut r_full: Vec<f64> = r.iter().rev().copied().collect();
                        let mut th_full: Vec<f64> = th.iter().rev().map(|t| -t).collect();
                        r_full.extend_from_slice(&r[1..]);
                        th_full.extend_from_slice(&th[1..]);
                        let z = params.teeth;
                        let mut was = Vec::with_capacity(r_full.len() * z as usize + 1);
                        for k in 0..z {
                            let base = 2.0 * std::f64::consts::PI * f64::from(k) / f64::from(z);
                            for (rr, tt) in r_full.iter().zip(&th_full) {
                                let a = base + tt;
                                was.push([rr * a.cos(), rr * a.sin()]);
                            }
                        }
                        if let Some(&first) = was.first() {
                            was.push(first);
                        }

                        let now = Eccentric::new(params).outline(per_tooth);
                        assert_eq!(now.len(), was.len(), "z={teeth} x={shift}");
                        for (i, (a, b)) in now.iter().zip(&was).enumerate() {
                            assert_eq!(
                                (a[0].to_bits(), a[1].to_bits()),
                                (b[0].to_bits(), b[1].to_bits()),
                                "z={teeth} x={shift} β={helix} λ={lambda}: point {i} moved, \
                                 {a:?} against {b:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    /// **The figures §4.10 tabulates, reproduced to every quoted digit.**
    ///
    /// That section's numbers were derived independently of this code and
    /// marked `[verified]` before any of it existed, which makes them the
    /// closest thing this feature has to an outside reference. Reproducing them
    /// is what says the assembly implements the design rather than something
    /// adjacent to it.
    ///
    /// z = 17, α = 20°, `e` = 0.25 mm, base-tangent pitch error:
    ///
    /// | λ | drive | coast |
    /// |---|---|---|
    /// | 0 | 62.6 µm | 62.6 µm |
    /// | 0.5 | 31.3 µm | 93.9 µm |
    /// | 1 | 0.000 µm | 125.2 µm |
    #[test]
    fn the_tabulated_errors_come_out_as_designed() {
        let at = |lambda: f64| {
            Eccentric::new(GearParams {
                teeth: 17,
                angular_shift: 0.25,
                index_offset: lambda,
                ..Default::default()
            })
            .variation()
        };
        for (lambda, drive, coast) in [
            (0.0_f64, 62.6_f64, 62.6_f64),
            (0.5, 31.3, 93.9),
            (1.0, 0.0, 125.2),
        ] {
            let v = at(lambda);
            for (name, got, want) in [
                ("drive", v.drive_pitch_error, drive),
                ("coast", v.coast_pitch_error, coast),
            ] {
                assert!(
                    (1e3 * got - want).abs() < 0.05,
                    "λ={lambda}: {name} pitch error {} µm against the tabulated {want}",
                    1e3 * got
                );
            }
        }

        // ...the base-thickness spread, which is where the eccentricity lives.
        let v = at(0.0);
        assert!(
            (1e3 * (v.base_thickness[1] - v.base_thickness[0]) - 339.0).abs() < 0.5,
            "base thickness spread {} µm against 339",
            1e3 * (v.base_thickness[1] - v.base_thickness[0])
        );

        // ...and the tip envelope's departure from a true displaced circle,
        // which is what makes "nearly a shifted circle" a measurement.
        for (shift, want) in [(0.10_f64, 0.0005_f64), (0.25, 0.0033), (0.50, 0.0132)] {
            let v = Eccentric::new(GearParams {
                teeth: 17,
                angular_shift: shift,
                ..Default::default()
            })
            .variation();
            assert!(
                (v.circle_departure - want).abs() < 5e-5,
                "e={shift}: departure {} against the tabulated {want}",
                v.circle_departure
            );
        }
    }

    /// **The two errors scale as `|1 − λ|` and `|1 + λ|`, and λ = 0 is the
    /// minimax.**
    ///
    /// The law behind the table, checked as a law rather than at three points:
    /// any compensation that improves one direction degrades the other by more
    /// than it gains, so `min_λ max(|1−λ|, |1+λ|)` is at λ = 0. That is the
    /// whole content of "E2 is provably the best symmetric choice", and it is
    /// the reason the tool exposes λ instead of choosing for the designer.
    #[test]
    fn the_indexing_trades_one_flank_against_the_other() {
        let at = |lambda: f64| {
            Eccentric::new(GearParams {
                teeth: 23,
                angular_shift: 0.3,
                index_offset: lambda,
                ..Default::default()
            })
            .variation()
        };
        let reference = at(0.0);
        for lambda in [-1.5_f64, -0.4, 0.0, 0.4, 1.0, 2.5] {
            let v = at(lambda);
            for (name, got, scale) in [
                ("drive", v.drive_pitch_error, (1.0 - lambda).abs()),
                ("coast", v.coast_pitch_error, (1.0 + lambda).abs()),
            ] {
                let want = scale * reference.drive_pitch_error;
                assert!(
                    (got - want).abs() < 1e-9 * reference.drive_pitch_error.max(1e-12),
                    "λ={lambda}: {name} error {got} against {want}"
                );
            }
            // Minimax: nothing beats λ = 0 on the worse of the two.
            assert!(
                v.drive_pitch_error.max(v.coast_pitch_error) >= reference.drive_pitch_error - 1e-12,
                "λ={lambda} beat the minimax, which is two lines of algebra away \
                 from impossible"
            );
        }
    }

    /// **The pitch and base circles do not move.** §4.10 rests on this: it is
    /// what makes the body eccentric while the *action* stays uniform, and it is
    /// why the feature is possible at all. Profile shift moves the tool, not the
    /// rolling.
    #[test]
    fn the_eccentricity_is_in_the_flanks_and_not_in_the_pitch_circle() {
        let base = Gear::new(GearParams {
            teeth: 31,
            ..Default::default()
        });
        let ecc = Eccentric::new(GearParams {
            teeth: 31,
            angular_shift: 0.4,
            ..Default::default()
        });
        for k in 0..31 {
            let (g, _) = ecc.tooth(k);
            assert_eq!(
                g.r.to_bits(),
                base.r.to_bits(),
                "tooth {k} moved its pitch circle"
            );
            assert_eq!(
                g.rb.to_bits(),
                base.rb.to_bits(),
                "tooth {k} moved its base circle"
            );
            assert_eq!(g.alpha_t.to_bits(), base.alpha_t.to_bits());
        }
        // What *does* move is the envelope, by `e` — and the flanks with it.
        let v = ecc.variation();
        assert!((v.eccentricity - 0.4).abs() < 1e-12);
        assert!(v.base_thickness[1] > v.base_thickness[0]);
    }

    /// **The teeth sample the envelope; they are not the envelope.**
    ///
    /// The tip radii span `2e` only when a tooth actually sits at 180°, which
    /// needs an **even** tooth count. With an odd one the nearest teeth straddle
    /// it by half a pitch and the realised spread falls short by
    /// `1 − cos(2π⌊z/2⌋/z)` — 0.7979 mm against 0.8 at z = 31. Small, and worth
    /// being a fact rather than a surprise: `eccentricity` is the envelope's and
    /// the tip range is what gets cut, so a reader can see both.
    #[test]
    fn the_realised_tip_spread_is_what_the_teeth_reach() {
        for teeth in [30_u32, 31, 32, 17] {
            let shift = 0.4;
            let v = Eccentric::new(GearParams {
                teeth,
                angular_shift: shift,
                ..Default::default()
            })
            .variation();
            #[allow(clippy::cast_precision_loss)]
            let reached =
                1.0 - (std::f64::consts::TAU * (teeth / 2) as f64 / f64::from(teeth)).cos();
            let want = shift * reached;
            for (name, span) in [("tip", v.tip_radius), ("root", v.root_radius)] {
                assert!(
                    (span[1] - span[0] - want).abs() < 1e-12,
                    "z={teeth}: {name} spread {} against {want}",
                    span[1] - span[0]
                );
            }
            // Even counts reach the whole envelope, odd ones fall just short.
            if teeth % 2 == 0 {
                assert!(
                    (span_of(v) - 2.0 * v.eccentricity).abs() < 1e-12,
                    "z={teeth}"
                );
            } else {
                assert!(span_of(v) < 2.0 * v.eccentricity, "z={teeth}");
            }
        }
    }

    fn span_of(v: Variation) -> f64 {
        v.tip_radius[1] - v.tip_radius[0]
    }

    /// A concentric gear varies over nothing, and says so in every field.
    #[test]
    fn an_ordinary_gear_reports_no_variation_at_all() {
        for lambda in [0.0_f64, 1.0, -2.0] {
            let v = Eccentric::new(GearParams {
                teeth: 19,
                profile_shift: 0.3,
                index_offset: lambda,
                ..Default::default()
            })
            .variation();
            assert_eq!(v.eccentricity, 0.0);
            assert_eq!(v.circle_departure, 0.0);
            assert_eq!(v.drive_pitch_error, 0.0, "λ={lambda}");
            assert_eq!(v.coast_pitch_error, 0.0, "λ={lambda}");
            assert_eq!(v.drive_index_error, 0.0);
            assert_eq!(v.coast_index_error, 0.0);
            for span in [
                v.tip_radius,
                v.root_radius,
                v.tooth_thickness,
                v.base_thickness,
            ] {
                assert_eq!(span[0].to_bits(), span[1].to_bits());
            }
        }
    }

    /// **Both ways of drawing a gear see the eccentricity.**
    ///
    /// There are two: a point budget for the screen and a chord tolerance for
    /// the DXF. The second replicated a single tooth `z` times, so an eccentric
    /// gear would have *exported as a concentric one* — the right shape on
    /// screen and the wrong one in the file, which is the worst way for this to
    /// be wrong. Both go through this module now, and this holds them to it.
    ///
    /// Asserted on the radius spread rather than on vertices, since the two
    /// generators sample differently by design and only the shape has to agree.
    #[test]
    fn an_eccentric_gear_is_eccentric_in_the_export_too() {
        let params = GearParams {
            teeth: 24,
            angular_shift: 0.25,
            ..Default::default()
        };
        let want = 2.0 * params.module * params.angular_shift;

        let spread = |radii: Vec<f64>| {
            let (lo, hi) = radii
                .iter()
                .fold((f64::MAX, f64::MIN), |(l, h), &r| (l.min(r), h.max(r)));
            hi - lo
        };
        let ecc = Eccentric::new(params);
        let flat = Eccentric::new(GearParams {
            angular_shift: 0.0,
            ..params
        });

        // An outline runs root to tip, so its radius spread is the tooth depth
        // *plus* the eccentricity. Taking the concentric gear's spread away
        // leaves the eccentricity alone — which is what has to appear in both,
        // and what a check on the raw spread would have confused with a tooth.
        let screen =
            |g: &Eccentric| spread(g.outline(600).iter().map(|p| p[0].hypot(p[1])).collect());
        let export = |g: &Eccentric| {
            spread(
                g.outline_adaptive(1e-3)
                    .iter()
                    .map(|v| v.x.hypot(v.y))
                    .collect(),
            )
        };
        for (name, got) in [
            ("screen", screen(&ecc) - screen(&flat)),
            ("export", export(&ecc) - export(&flat)),
        ] {
            assert!(
                (got - want).abs() < 0.02,
                "the {name} outline carries {got} mm of eccentricity against {want}"
            );
        }

        // ...and the concentric gear's own spread is its tooth depth and nothing
        // else, in both, which is what says the subtraction above is subtracting
        // the right thing.
        let depth = flat.mean().ra - flat.mean().rf;
        for (name, got) in [("screen", screen(&flat)), ("export", export(&flat))] {
            assert!(
                (got - depth).abs() < 1e-9,
                "concentric {name} spread {got} against a tooth depth of {depth}"
            );
        }
    }

    /// ...and it costs what it always did: one tooth generated, not `z`.
    ///
    /// The other half of "an extension, not a branch" — collapsing onto the old
    /// case has to be free as well as exact, or the general path would be a tax
    /// every ordinary gear pays.
    #[test]
    fn an_ordinary_gear_still_generates_one_tooth() {
        for teeth in [9_u32, 17, 200] {
            let concentric = Eccentric::new(GearParams {
                teeth,
                ..Default::default()
            });
            assert_eq!(concentric.distinct_teeth(), 1, "z={teeth}");

            // An eccentric one generates the mirror pairs and no more: teeth `k`
            // and `z − k` sit at the same place in the variation.
            let eccentric = Eccentric::new(GearParams {
                teeth,
                angular_shift: 0.2,
                ..Default::default()
            });
            assert_eq!(
                eccentric.distinct_teeth(),
                teeth as usize / 2 + 1,
                "z={teeth}: should be ⌈z/2⌉+1 distinct teeth"
            );
        }
    }
}
