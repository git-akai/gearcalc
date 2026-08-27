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

use crate::involute::inv;
use crate::mesh::{operating_geometry, Member, MeshError, MeshKind};
use crate::note::{key, Note};
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
        let build = |x: f64, root_radius: f64, dedendum: f64| {
            Gear::new(GearParams {
                profile_shift: x,
                root_radius,
                dedendum,
                ..params
            })
        };
        let mut teeth: Vec<Gear> = shifts
            .iter()
            .map(|&x| build(x, params.root_radius, params.dedendum))
            .collect();

        // **One hob, one setting.** This is the whole of what makes an eccentric
        // gear an eccentric gear rather than a ring of unrelated ones, and it is
        // where every guard rail in `Gear::new` becomes a trap: those guards are
        // *gear-level* decisions, and applying them tooth by tooth lets
        // neighbours be built to different rules. Every such difference is a
        // step around the gear.
        //
        // Two of the settings are the tool's and must be shared:
        //
        // - **The tip round.** `Gear::new` caps it to what the tooth space will
        //   hold, and a space narrows as the shift rises — so per-tooth building
        //   gave 0.2375 modules on the high side against 0.3800 on the low.
        // - **The depth.** `Gear::new` raises the cutter depth when it would go
        //   non-positive, which happens as the shift approaches the dedendum. On
        //   the high side that pinned four teeth to the same root radius while
        //   their neighbours followed the envelope — a flat spot and then a
        //   corner, exactly where the shift is most extreme.
        //
        // The tool that can cut every tooth is the one the *most demanding*
        // tooth needs: the smallest round and the greatest depth. Set once and
        // used by all, which is what a hob does. Teeth that set a limit keep
        // their clamps, so the reason stays on the record.
        //
        // What is **not** shared is what is genuinely a fact about a tooth
        // rather than about the tool: a tooth with too much shift comes to a
        // point, and one with too little is undercut. Those are reported per
        // tooth (see `clamped_teeth`) rather than smoothed away.
        //
        // The rebuild happens only when the teeth **disagree**, which a
        // concentric gear's never do — it has one tooth. So this costs an
        // ordinary gear nothing and cannot move it.
        let round = teeth.iter().fold(f64::MAX, |m, g| m.min(g.rho)) / params.module;
        let depth = teeth.iter().zip(&shifts).fold(f64::MIN, |m: f64, (g, &x)| {
            m.max((g.r - g.rf) / params.module + x)
        });
        let settled = |g: &Gear, x: f64| {
            g.rho / params.module == round && (g.r - g.rf) / params.module + x == depth
        };
        if !teeth.iter().zip(&shifts).all(|(g, &x)| settled(g, x)) {
            for (t, &x) in teeth.iter_mut().zip(&shifts) {
                *t = build(x, round, depth);
            }
        }

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

    /// Every distinct tooth, once each.
    ///
    /// The unit of work for anything that varies around the revolution: ask a
    /// per-tooth question once per *answer* rather than once per position, since
    /// teeth `k` and `z − k` are the same gear. A concentric gear yields one.
    ///
    /// This is what an output becoming a range is built on — [`Self::span`] for
    /// the scalar case, and this directly for anything that is not a scalar.
    /// The measurements over teeth and pins are the obvious next customers: they
    /// return `Option`s rather than numbers, so they want the iterator and their
    /// own reduction rather than `span`.
    pub fn distinct(&self) -> impl Iterator<Item = &Gear> + '_ {
        self.teeth.iter()
    }

    /// The smallest and largest a per-tooth quantity takes around the
    /// revolution.
    ///
    /// `[v, v]` for a concentric gear, with both ends the *same bits* — which is
    /// what lets a caller report a range unconditionally and have an ordinary
    /// gear read as a single number.
    #[must_use]
    pub fn span(&self, of: impl Fn(&Gear) -> f64) -> [f64; 2] {
        let (lo, hi) = self.teeth.iter().fold((f64::MAX, f64::MIN), |(l, h), g| {
            let v = of(g);
            (l.min(v), h.max(v))
        });
        [lo, hi]
    }

    /// The root radius the tool leaves at an absolute angle, mm.
    ///
    /// # Why the root is not the tooth's
    ///
    /// A tooth's own root radius is `r − m(h_f − x_k)`, and neighbouring teeth
    /// have different `x` — so drawing each tooth's root at its own radius puts
    /// a **radial step at every mid-space**, up to 0.13 mm on a module-1 gear.
    /// No hob can leave that: it is one edge sweeping past, and what it leaves
    /// is the envelope `r − m(h_f − x(θ))`, smooth all the way round.
    ///
    /// The flanks are a different matter and stay with their teeth — constant
    /// ratio *requires* each to be one involute at one shift (§4.10), and that
    /// is what makes the feature possible. Nothing constrains the root, which is
    /// why it is free to be continuous.
    ///
    /// Written as an offset from the **mean** tooth rather than from each
    /// tooth's own, so two neighbours compute the same bits at the angle they
    /// share, and so a concentric gear's offset is exactly zero.
    fn root_at(&self, angle: f64) -> f64 {
        let p = self.mean.params;
        self.mean.rf + p.module * (p.angular_shift * angle.cos())
    }

    /// How far a point of tooth `k` is displaced radially by the tool's motion,
    /// mm — `tt` is its angle from the tooth's own centreline.
    ///
    /// # What is being corrected
    ///
    /// A tooth's own root radius is `r − m(h_f − x_k)`, and neighbours have
    /// different `x`, so leaving each tooth's root at its own radius put a
    /// radial **step at every mid-space**, up to 0.13 mm. No hob can leave that:
    /// it is one edge sweeping past, and what it leaves is the envelope
    /// `r − m(h_f − x(θ))`.
    ///
    /// # Why the fillet moves and the flank does not
    ///
    /// The **flank** is untouchable: constant ratio requires it to be one
    /// involute of one base circle at one shift (§4.10), and that is the whole
    /// feature. So the displacement is zero at the flank/fillet junction and
    /// stays zero over the flank and the tip.
    ///
    /// The **fillet** is not constrained by anything, and it is cut by the same
    /// tip corner that cuts the root — at a radial position that is already
    /// moving. Correcting only the flat root was the first attempt and it left a
    /// visible notch: the root arc spans about 0.005 rad, so taking up 0.05 mm
    /// inside it means a dive of 9 mm/rad where the envelope's own slope is 0.4.
    /// Spread over the fillet as well the span is ten times longer and the
    /// correction is gentle.
    ///
    /// `w = t²(3 − 2t)` rather than `t`, so the displacement is **stationary at
    /// both ends**: it leaves the flank junction without kinking the involute,
    /// and meets its neighbour's half at mid-space with the same slope.
    ///
    /// Still an **interpolation, not a derivation** — the true surface is the
    /// envelope of a tool corner under rolling *and* radial motion, which §4.10
    /// scoped out. What it has to be is zero at the flank, tangent there, and
    /// through the envelope at mid-space; it is all three.
    ///
    /// Exactly `0.0` for a concentric gear, where `root_at` returns that tooth's
    /// own `rf` to the bit — so every point is `r + 0.0` and nothing moves.
    /// How far tooth `k`'s root has to reach on one side to meet its
    /// neighbour, as an angle from the tooth's own centreline.
    ///
    /// # Why this is not half a pitch
    ///
    /// It is half a pitch only when the teeth are **evenly seated**. The
    /// indexing offset λ moves each tooth by `λ(ψ̄ − ψ_k)`, and neighbours move
    /// by different amounts — so the space between two teeth is a pitch *plus*
    /// the difference of their offsets. Drawing every tooth exactly one ideal
    /// pitch wide then leaves an angular **gap** wherever the seats spread and
    /// an overlap wherever they close: at λ = 1 on a Δx = 1 gear that is 0.009
    /// rad, and the outline simply jumps across it.
    ///
    /// A tooth owns its flanks and its fillet; the space between two teeth is
    /// whatever is left over, and the root has to fill exactly that. So the
    /// reach is to the **midpoint between the two seats**, which is half a pitch
    /// plus half the offset difference.
    ///
    /// Written as that difference rather than from the seats themselves, so a
    /// concentric gear — where every `ψ_k` is the same — gets exactly `0.0` and
    /// the root is left where it was, to the bit.
    pub fn reach(&self, k: usize, side: f64) -> f64 {
        let n = self.which.len();
        let psi = |j: usize| self.tooth(j % n).0.psi_b;
        let (a, b) = if side < 0.0 {
            ((k + n - 1) % n, k)
        } else {
            (k, (k + 1) % n)
        };
        let spread = self.mean.params.index_offset * (psi(a) - psi(b)) / 2.0;
        self.tooth(k).0.half_pitch + spread
    }

    /// A point of tooth `k`, corrected for the tool's motion and for where its
    /// neighbours actually sit: `(radius, angle from the tooth's centreline)`.
    ///
    /// Both outlines go through this, and a concentric gear comes out of it
    /// unchanged to the bit.
    fn corrected(&self, k: usize, r: f64, tt: f64) -> (f64, f64) {
        let (g, seat) = self.tooth(k);
        // The root arc is emitted at exactly the tooth's own `rf` — every other
        // point is on a generated curve — and it is the only part that stretches
        // to meet a neighbour. The tooth's own flanks and fillet do not move.
        let tt = if r == g.rf {
            let reach = self.reach(k, tt);
            let span = g.half_pitch - g.theta0;
            let along = if span > 0.0 {
                (tt.abs() - g.theta0) / span
            } else {
                1.0
            };
            tt.signum() * (g.theta0 + along * (reach - g.theta0))
        } else {
            tt
        };
        (r + self.displacement(k, r, seat + tt), tt)
    }

    fn displacement(&self, k: usize, r: f64, angle: f64) -> f64 {
        let (g, _) = self.tooth(k);
        // **Parametrised on radius, not on angle.** `θ` is not monotone along a
        // profile — the flank continues *below the base circle* to its true
        // intersection with the trochoid (§4.2) and is re-entrant down there, so
        // a flank point can sit at a larger angle than the junction does and
        // would take a displacement it must never have. Radius is the invariant
        // that does stay monotone, which is why it is the one that measures how
        // far down the fillet a point is.
        let junction_r = g.trochoid_at(g.s_j).0;
        let span = junction_r - g.rf;
        let t = if span > 0.0 {
            ((junction_r - r) / span).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let w = t * t * (3.0 - 2.0 * t);
        w * (self.root_at(angle) - g.rf)
    }

    /// The tooth at position `k`, and where it is seated.    /// The tooth at position `k`, and where it is seated.
    #[must_use]
    pub fn tooth(&self, k: usize) -> (&Gear, f64) {
        let i = k % self.which.len();
        (&self.teeth[self.which[i]], self.seat[i])
    }

    /// Which teeth came out other than as drawn, and why.
    ///
    /// Empty for an ordinary gear whose inputs are buildable, and empty for an
    /// eccentric one too — the tool settings are shared, so a guard that trips
    /// on a *setting* trips for the whole gear or not at all. What lands here is
    /// only what is true of one tooth and not its neighbour.
    #[must_use]
    pub fn troubled_teeth(&self) -> Trouble {
        let mut teeth = Vec::new();
        let mut notes: Vec<Note> = Vec::new();
        for k in 0..self.which.len() {
            let (g, _) = self.tooth(k);
            let mut its: Vec<Note> = g.clamps.notes.clone();
            if g.severed {
                its.push(Note::new(key::CLAMP_TOOTH_SEVERED));
            } else if g.undercut {
                its.push(Note::new(key::CLAMP_TOOTH_UNDERCUT));
            }
            if its.is_empty() {
                continue;
            }
            #[allow(clippy::cast_possible_truncation)]
            teeth.push(k as u32);
            for n in its {
                if !notes.iter().any(|m| m.key == n.key) {
                    notes.push(n);
                }
            }
        }
        Trouble { teeth, notes }
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
                // Corrected for the tool's motion and for where the neighbours
                // actually sit. No section needs identifying: the rules are
                // functions of where the point is.
                let (rr, tt) = self.corrected(k, *rr, *tt);
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
        // A constant root is a circle and stays an exact arc in the export; a
        // varying one is not a circle, so it is subdivided like a flank. The
        // rule for *where* it runs is `root_radius`, shared with the screen
        // outline — one root, two consumers.
        let varying = self.mean.params.angular_shift != 0.0;
        let mut out = Vec::new();
        for (k, &i) in self.which.iter().enumerate() {
            let displace = |r: f64, tt: f64| self.corrected(k, r, tt);
            let displace: Option<&dyn Fn(f64, f64) -> (f64, f64)> =
                if varying { Some(&displace) } else { None };
            self.teeth[i].tooth_outline(chord_tolerance, self.seat[k], displace, &mut out);
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

        let tip_radius = self.span(|g| g.ra);
        let root_radius = self.span(|g| g.rf);
        let tooth_thickness = self.span(|g| 2.0 * g.r * g.psi_p);
        let base_thickness = self.span(|g| 2.0 * g.rb * g.psi_b);

        let e = p.module * p.angular_shift.abs();
        Variation {
            eccentricity: e,
            // `e²/2ρ`, with the tip circle as the curvature it departs from.
            circle_departure: if self.mean.ra > 0.0 {
                e * e / (2.0 * self.mean.ra)
            } else {
                0.0
            },
            tip_radius,
            root_radius,
            tooth_thickness,
            base_thickness,
            drive_pitch_error: drive_pitch,
            coast_pitch_error: coast_pitch,
            drive_index_error: drive_index,
            coast_index_error: coast_index,
        }
    }
}

/// Teeth that are not the gear that was asked for, and why.
///
/// A guard rail that is genuinely about **one tooth** — a tooth with too much
/// shift comes to a point, one with too little is undercut — cannot be shared
/// away like a tool setting, and smoothing it over would be inventing geometry.
/// It is reported instead, with the positions, because "some of your teeth are
/// not what you drew" is only useful if it says which.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Trouble {
    /// Positions round the gear, counting from `θ = 0`.
    pub teeth: Vec<u32>,
    /// What happened, each reason once.
    pub notes: Vec<Note>,
}

/// The centre distance an eccentric mesh wants, around one revolution.
///
/// Sampled at the **tooth positions**, one per tooth: that is where contact
/// actually is, and it is where the discrete shifts the gear was cut at land
/// exactly on the continuous `x(θ)` the mechanism is tracking. No sample count
/// to choose, and nothing interpolated.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CentreProfile {
    /// The zero-backlash centre distance at each tooth position, mm, starting at
    /// `θ = 0` and going round.
    pub commanded: Vec<f64>,
    /// Smallest and largest of the above, mm.
    pub range: [f64; 2],
    /// The best-fit pure sinusoid: mean, amplitude, phase in radians.
    ///
    /// What a **simple eccentric or crank** can actually deliver. `a_w(θ)` is
    /// not sinusoidal even though `x(θ)` is — it passes through `inv⁻¹` and a
    /// cosine on the way — so a mechanism built that way tracks this instead,
    /// and the difference is the next two fields.
    ///
    /// Exact rather than fitted: the samples are equally spaced, so the first
    /// Fourier coefficient *is* the least-squares sinusoid, with no optimiser
    /// and no starting guess.
    pub sinusoid: [f64; 3],
    /// The largest departure of the ideal from that sinusoid, mm of centre
    /// distance.
    pub sinusoid_error: f64,
    /// What that departure costs, as backlash at the pitch circle, mm —
    /// smallest and largest around the revolution.
    ///
    /// **The number an engineer building the mechanism actually needs**, and the
    /// reason the centre-distance error is not the end of the story. Negative is
    /// not slack but *interference*: the mechanism has brought the axes closer
    /// than the teeth at that position allow.
    pub sinusoid_backlash: [f64; 2],
}

impl Eccentric {
    /// The centre distance this gear's mesh wants at each tooth position.
    ///
    /// `at` says which member the eccentric gear is. That matters only for an
    /// **internal** mesh, where the ring is member 2 by the same convention
    /// [`Mesh::new`](crate::mesh::Mesh::new) uses — so an eccentric ring is
    /// `Member::Second` and an eccentric pinion running in a fixed ring is
    /// `Member::First`. For an external pair the two are the same answer.
    ///
    /// # One relation, both kinds
    ///
    /// `mesh::operating_geometry` takes a **signed** tooth sum and shift sum,
    /// with member 2 carrying the kind's sign, and that one expression is the
    /// whole of the difference between an external and an internal mesh (§4.11).
    /// Nothing here re-derives it: the arrangement decides which slot carries
    /// the sign, and the arithmetic below never asks which kind it is.
    ///
    /// # Errors
    ///
    /// [`MeshError::Incompatible`] if the two cannot mesh at all, and
    /// [`MeshError::CentreDistanceTooSmall`] if some position drives the base
    /// circles into each other — which for this feature means the eccentricity
    /// is larger than the pair can absorb.
    pub fn centre_profile(
        &self,
        mate: &Gear,
        kind: MeshKind,
        at: Member,
    ) -> Result<CentreProfile, MeshError> {
        let g = &self.mean;
        if (g.params.module - mate.params.module).abs() > 1e-12
            || (g.params.pressure_angle - mate.params.pressure_angle).abs() > 1e-12
        {
            return Err(MeshError::Incompatible);
        }

        // Which slot carries the kind's sign. The *only* place the arrangement
        // enters; everything below is one expression for both kinds.
        let sigma = kind.sign();
        let (sign_e, sign_m) = match at {
            Member::First => (1.0, sigma),
            Member::Second => (sigma, 1.0),
        };
        let z_sum = sign_e * g.z + sign_m * mate.z;
        let x_mate = mate.params.profile_shift + mate.params.thickness_shift();

        let commanded = (0..self.which.len())
            .map(|k| {
                let (tooth, _) = self.tooth(k);
                let x_e = tooth.params.profile_shift + tooth.params.thickness_shift();
                let x_sum = sign_e * x_e + sign_m * x_mate;
                operating_geometry(g.mt, g.alpha_t, g.alpha_n, z_sum, x_sum)
                    .map(|(_, _, a_w)| a_w)
                    .ok_or(MeshError::CentreDistanceTooSmall)
            })
            .collect::<Result<Vec<f64>, MeshError>>()?;

        let n = commanded.len();
        #[allow(clippy::cast_precision_loss)]
        let count = n as f64;
        let angle = |k: usize| {
            #[allow(clippy::cast_precision_loss)]
            let kf = k as f64;
            std::f64::consts::TAU * kf / count
        };

        // The least-squares sinusoid over equally spaced samples is the first
        // Fourier coefficient, exactly.
        // Anchored on the first sample rather than summed outright: for a flat
        // profile every `a_k − a_0` is exactly zero, so the mean comes out
        // exactly `a_0` where `Σa/n` would land an ulp away. Mathematically the
        // same number, and it is the third time in this module that grouping the
        // cancellation first is what separates an answer from a residual.
        let anchor = commanded[0];
        let mean = anchor + commanded.iter().map(|a| a - anchor).sum::<f64>() / count;
        // Projected onto the **deviation from the mean**, not the raw values.
        // `Σ mean·cos θ_k` is zero mathematically and about 1e-15 of the mean in
        // practice, which is enough to give a *concentric* gear a 4e-15 mm
        // sinusoid amplitude — a rounding residual dressed as a measurement, for
        // the second time in this module. Subtracting first makes every term
        // exactly zero when the samples are.
        let (mut c, mut s) = (0.0, 0.0);
        for (k, a) in commanded.iter().enumerate() {
            let (sin, cos) = angle(k).sin_cos();
            c += (a - mean) * cos;
            s += (a - mean) * sin;
        }
        let (c, s) = (2.0 * c / count, 2.0 * s / count);
        let amplitude = c.hypot(s);
        let phase = s.atan2(c);

        let mut error = 0.0_f64;
        let mut play = [f64::MAX, f64::MIN];
        for (k, &ideal) in commanded.iter().enumerate() {
            let fit = mean + amplitude * (angle(k) - phase).cos();
            error = error.max((ideal - fit).abs());
            // What the mechanism's departure costs, by the §4.4 law: the ideal
            // is the zero-backlash distance at this position, the fit is where
            // the machine actually puts the axes.
            let a_ref = g.mt * z_sum.abs() / 2.0;
            let cos_actual = a_ref * g.alpha_t.cos() / fit;
            let alpha_actual = if (-1.0..=1.0).contains(&cos_actual) {
                cos_actual.acos()
            } else {
                return Err(MeshError::CentreDistanceTooSmall);
            };
            let cos_ideal = a_ref * g.alpha_t.cos() / ideal;
            let alpha_ideal = if (-1.0..=1.0).contains(&cos_ideal) {
                cos_ideal.acos()
            } else {
                return Err(MeshError::CentreDistanceTooSmall);
            };
            let j = 2.0 * fit * (inv(alpha_actual) - inv(alpha_ideal));
            play[0] = play[0].min(j);
            play[1] = play[1].max(j);
        }

        let (lo, hi) = commanded
            .iter()
            .fold((f64::MAX, f64::MIN), |(l, h), &a| (l.min(a), h.max(a)));
        Ok(CentreProfile {
            commanded,
            range: [lo, hi],
            sinusoid: [mean, amplitude, phase],
            sinusoid_error: error,
            sinusoid_backlash: play,
        })
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

    /// **The profile is `Mesh`'s own answer, tooth by tooth — bit for bit.**
    ///
    /// The centre-distance profile is not a second derivation of the operating
    /// geometry; it is the existing one asked once per tooth. This holds it to
    /// that by building the mesh the ordinary way at each tooth's shift and
    /// demanding the *same number*, for all three arrangements: an external
    /// pair, an eccentric pinion inside a fixed ring, and an eccentric ring
    /// around a fixed pinion.
    ///
    /// Bit-identical rather than close, because "one relation, both kinds" is
    /// the claim and a tolerance would let a second one through.
    #[test]
    fn every_sample_is_the_mesh_the_ordinary_way() {
        use crate::mesh::Mesh;

        let ecc = |z: u32| {
            Eccentric::new(GearParams {
                teeth: z,
                angular_shift: 0.25,
                ..Default::default()
            })
        };
        let plain = |z: u32| {
            Gear::new(GearParams {
                teeth: z,
                ..Default::default()
            })
        };

        for (name, e, mate, kind, at) in [
            (
                "external",
                ecc(24),
                plain(43),
                MeshKind::External,
                Member::First,
            ),
            (
                "eccentric pinion in a ring",
                ecc(24),
                plain(60),
                MeshKind::Internal,
                Member::First,
            ),
            (
                "eccentric ring round a pinion",
                ecc(60),
                plain(24),
                MeshKind::Internal,
                Member::Second,
            ),
        ] {
            let profile = e.centre_profile(&mate, kind, at).expect("a meshable pair");
            for k in 0..profile.commanded.len() {
                let (tooth, _) = e.tooth(k);
                // The ring is member 2, whichever gear that is — the same
                // convention `Mesh::new` is written to.
                let mesh = match at {
                    Member::First => Mesh::new(tooth, &mate, kind),
                    Member::Second => Mesh::new(&mate, tooth, kind),
                }
                .expect("a meshable pair");
                assert_eq!(
                    profile.commanded[k].to_bits(),
                    mesh.a_w.to_bits(),
                    "{name}, tooth {k}: {} against Mesh's {}",
                    profile.commanded[k],
                    mesh.a_w
                );
            }
        }
    }

    /// **The range framework holds an ordinary gear to one number.**
    ///
    /// `span` is what every output that varies around the revolution is built
    /// on, and the property that makes it safe to use unconditionally is that a
    /// concentric gear's two ends are the *same bits*. A caller can then report
    /// a range for every gear and have an ordinary one read as a single value,
    /// with no flag to check. Gated here rather than left to each customer, so
    /// the measurements over teeth and pins can be added without re-proving it.
    #[test]
    fn a_range_over_an_ordinary_gear_is_one_number_twice() {
        let flat = Eccentric::new(GearParams {
            teeth: 31,
            profile_shift: 0.3,
            ..Default::default()
        });
        assert_eq!(flat.distinct().count(), 1);
        for of in [
            (|g: &Gear| g.ra) as fn(&Gear) -> f64,
            |g: &Gear| g.rf,
            |g: &Gear| g.psi_b,
            |g: &Gear| 2.0 * g.r * g.psi_p,
        ] {
            let [lo, hi] = flat.span(of);
            assert_eq!(lo.to_bits(), hi.to_bits());
        }

        // ...and an eccentric one visits every distinct tooth, so a range built
        // this way cannot miss an extreme.
        let ecc = Eccentric::new(GearParams {
            teeth: 31,
            angular_shift: 0.3,
            ..Default::default()
        });
        assert_eq!(ecc.distinct().count(), 16);
        let [lo, hi] = ecc.span(|g| g.params.profile_shift);
        let seen = ecc
            .distinct()
            .map(|g| g.params.profile_shift)
            .fold((f64::MAX, f64::MIN), |(l, h), v| (l.min(v), h.max(v)));
        assert_eq!(lo.to_bits(), seen.0.to_bits());
        assert_eq!(hi.to_bits(), seen.1.to_bits());
    }

    /// A concentric gear commands one centre distance and never moves off it —
    /// so the whole feature costs an ordinary pair nothing, here as everywhere.
    #[test]
    fn an_ordinary_gear_commands_a_constant_centre_distance() {
        use crate::mesh::Mesh;

        let flat = Eccentric::new(GearParams {
            teeth: 24,
            profile_shift: 0.2,
            ..Default::default()
        });
        let mate = Gear::new(GearParams {
            teeth: 43,
            profile_shift: -0.1,
            ..Default::default()
        });
        let p = flat
            .centre_profile(&mate, MeshKind::External, Member::First)
            .unwrap();
        let mesh = Mesh::new(flat.mean(), &mate, MeshKind::External).unwrap();
        for a in &p.commanded {
            assert_eq!(a.to_bits(), mesh.a_w.to_bits());
        }
        assert_eq!(p.range[0].to_bits(), p.range[1].to_bits());
        assert_eq!(p.sinusoid[1], 0.0, "a flat profile has no amplitude");
        assert_eq!(p.sinusoid_error, 0.0);
        assert_eq!(p.sinusoid_backlash, [0.0, 0.0]);
    }

    /// **`a_w(θ)` is not sinusoidal, even though `x(θ)` is** — and that is the
    /// whole reason the residual is worth reporting.
    ///
    /// It passes through `inv⁻¹` and a cosine on the way, so a mechanism built
    /// as a simple crank cannot track it. The departure grows faster than the
    /// eccentricity that causes it, which is what makes it a design limit rather
    /// than a rounding note: doubling `Δx` more than doubles the error.
    #[test]
    fn a_simple_crank_cannot_follow_the_ideal_profile() {
        let mate = Gear::new(GearParams {
            teeth: 43,
            ..Default::default()
        });
        let at = |shift: f64| {
            Eccentric::new(GearParams {
                teeth: 24,
                angular_shift: shift,
                ..Default::default()
            })
            .centre_profile(&mate, MeshKind::External, Member::First)
            .expect("a meshable pair")
        };

        let mut previous: Option<(f64, f64)> = None;
        for shift in [0.05_f64, 0.10, 0.20, 0.40] {
            let p = at(shift);
            assert!(
                p.sinusoid_error > 0.0,
                "Δx={shift}: the ideal is not a sinusoid, so a fit must leave something"
            );
            // The backlash it costs straddles zero: a crank runs wide over part
            // of the revolution and tight over the rest, and the tight half is
            // interference rather than slack.
            assert!(p.sinusoid_backlash[0] < 0.0 && p.sinusoid_backlash[1] > 0.0);
            if let Some((was_shift, was_error)) = previous {
                let growth = p.sinusoid_error / was_error;
                let doubling = shift / was_shift;
                assert!(
                    growth > doubling,
                    "Δx {was_shift}→{shift}: the error grew {growth}×, which is not \
                     faster than the {doubling}× eccentricity that caused it"
                );
            }
            previous = Some((shift, p.sinusoid_error));
        }
    }

    /// **The envelopes are envelopes: no tooth's tip or root leaves them.**
    ///
    /// The audit's own gate, and the one that caught the third root defect. The
    /// tip and root radii are `r ± m(h + x(θ))` — smooth in `θ` — so every
    /// tooth's must sit on that curve exactly. A tooth whose *tool setting* was
    /// clamped on its own does not: four teeth had their cutter depth raised to
    /// the same floor while their neighbours followed the envelope, which is a
    /// flat spot and then a corner, on the high side at positive shift and the
    /// low side at negative. Sharing the setting is what closes it, and this is
    /// what says so.
    ///
    /// **The tip is exempt where a tooth is genuinely pointed**, which is a fact
    /// about that tooth and not about the tool — its tip really is lower than
    /// the envelope, and inventing the envelope's value there would be drawing a
    /// gear that cannot be made. Those teeth are reported instead.
    #[test]
    fn no_tooth_leaves_the_envelope_it_belongs_to() {
        for (shift, amplitude) in [
            (0.0_f64, 0.3_f64),
            (0.6, 0.3),
            (1.0, 0.3),
            (-0.6, 0.3),
            (0.5, 0.5),
        ] {
            let p = GearParams {
                teeth: 24,
                profile_shift: shift,
                angular_shift: amplitude,
                ..Default::default()
            };
            let e = Eccentric::new(p);
            let (first, seat0) = e.tooth(0);
            let pointed = |g: &Gear| g.clamps.fired(key::CLAMP_TIP_CAPPED_POINTED);

            for k in 0..24_usize {
                let (g, seat) = e.tooth(k);
                let step = p.module * p.angular_shift * (seat.cos() - seat0.cos());
                assert!(
                    (g.rf - (first.rf + step)).abs() < 1e-12,
                    "x={shift} Δx={amplitude}: tooth {k}'s root is {} where the \
                     envelope is {} — its cutter depth is not the gear's",
                    g.rf,
                    first.rf + step
                );
                if !pointed(g) && !pointed(first) {
                    assert!(
                        (g.ra - (first.ra + step)).abs() < 1e-12,
                        "x={shift} Δx={amplitude}: tooth {k}'s tip is {} where the \
                         envelope is {}",
                        g.ra,
                        first.ra + step
                    );
                }
            }
        }
    }

    /// **A tooth that is not the gear that was asked for says which tooth it
    /// is.**
    ///
    /// The other half of the audit. A guard on a *tool setting* is shared, so it
    /// trips for the whole gear or not at all; a guard on **one tooth** — too
    /// much shift and it comes to a point, too little and it is undercut —
    /// cannot be shared away, and smoothing it over would be drawing geometry
    /// that cannot be cut. It is reported with its position instead.
    #[test]
    fn the_teeth_that_are_not_as_drawn_are_named() {
        let at = |shift: f64| {
            Eccentric::new(GearParams {
                teeth: 24,
                profile_shift: shift,
                angular_shift: 0.3,
                ..Default::default()
            })
            .troubled_teeth()
        };

        // A buildable gear has nothing to report, eccentric or not. Not the
        // default one: z = 17 at zero shift is the textbook marginal-undercut
        // case, and reporting it is the feature working.
        assert!(at(0.2).teeth.is_empty());
        assert!(Eccentric::new(GearParams {
            teeth: 30,
            ..Default::default()
        })
        .troubled_teeth()
        .teeth
        .is_empty());
        assert!(!Eccentric::new(GearParams::default())
            .troubled_teeth()
            .teeth
            .is_empty());

        // Too much shift: the high teeth come to a point, and they are the ones
        // named — positions near θ = 0, not a bare count.
        let high = at(1.4);
        assert!(!high.teeth.is_empty());
        assert!(
            high.notes
                .iter()
                .any(|n| n.is(key::CLAMP_TIP_CAPPED_POINTED)),
            "{:?}",
            high.notes
        );
        assert!(
            high.teeth.iter().all(|&k| k <= 6 || k >= 18),
            "the pointed teeth are the tall half, within a quarter turn of θ = 0: {:?}",
            high.teeth
        );

        // Too little: the low teeth are undercut, and those sit near θ = 180°.
        let low = at(-1.0);
        assert!(low.notes.iter().any(|n| n.is(key::CLAMP_TOOTH_UNDERCUT)));
        assert!(
            low.teeth.iter().any(|&k| (6..=18).contains(&k)),
            "the undercut teeth are the short ones, near θ = 180°: {:?}",
            low.teeth
        );
    }

    /// **One hob has one tip radius.**
    ///
    /// `Gear::new` caps the cutter's tip round to what the tooth space will
    /// hold, and a space narrows as the shift rises — so building each tooth on
    /// its own gives the high side a *different tool* from the low side, 0.2375
    /// modules against 0.3800 on a gear that asks for 0.38. That is not a tool.
    /// It showed as a fillet that collapsed on the high teeth, and as the
    /// trochoid's own extent jumping sixfold between two neighbours.
    #[test]
    fn every_tooth_is_cut_by_the_same_tool() {
        for (shift, amplitude) in [(0.5_f64, 0.5_f64), (0.0, 0.25), (0.6, 0.5), (-0.3, 0.4)] {
            let e = Eccentric::new(GearParams {
                teeth: 24,
                profile_shift: shift,
                angular_shift: amplitude,
                ..Default::default()
            });
            let first = e.mean().rho;
            for (k, g) in e.distinct().enumerate() {
                assert_eq!(
                    g.rho.to_bits(),
                    e.tooth(0).0.rho.to_bits(),
                    "x={shift} Δx={amplitude}: tooth {k} was cut by a {} tool where \
                     tooth 0 had {}",
                    g.rho,
                    e.tooth(0).0.rho
                );
            }
            let _ = first;

            // ...and the trochoid's extent then varies smoothly rather than in
            // steps: it can only grow as the tooth gets deeper.
            let mut previous = 0.0_f64;
            for k in 0..=(24 / 2) {
                let (g, _) = e.tooth(k);
                let extent = g.s_j.abs();
                assert!(
                    extent >= previous,
                    "tooth {k}: the fillet shrank as the tooth deepened, {extent} \
                     against {previous}"
                );
                previous = extent;
            }
        }
    }

    /// **The root runs continuously round the gear, on screen and in the
    /// export.**
    ///
    /// Each tooth's own root radius is `r − m(h_f − x_k)`, and neighbours have
    /// different `x` — so drawing each tooth's root at its own radius left a
    /// radial **step** at every mid-space, up to 0.13 mm. No hob can leave that.
    ///
    /// The property that separates a continuous curve from a stepped one is not
    /// a tolerance but a *trend*: refine the sampling and a curve's largest jump
    /// falls with it, while a step does not move at all. That is what is
    /// asserted, so no threshold is chosen and the check cannot be satisfied by
    /// a step small enough to sneak under one.
    #[test]
    fn the_root_runs_continuously_round_an_eccentric_gear() {
        let radial_jump = |points: &[[f64; 2]]| {
            points
                .windows(2)
                .map(|w| (w[0][0].hypot(w[0][1]) - w[1][0].hypot(w[1][1])).abs())
                .fold(0.0_f64, f64::max)
        };

        // The last case carries a **non-zero λ**, without which none of these
        // sees the indexing at all: λ moves each tooth by a different amount, so
        // the space between two of them is no longer a pitch, and a tooth drawn
        // one ideal pitch wide leaves a gap. Every earlier case here had λ = 0,
        // which is exactly why the gap survived them.
        for (shift, amplitude, teeth, lambda) in [
            (0.5_f64, 0.5_f64, 24_u32, 0.0_f64),
            (0.0, 0.25, 24, 0.0),
            (0.0, 0.6, 40, 0.0),
            (0.2, 1.0, 23, 1.0),
        ] {
            let e = Eccentric::new(GearParams {
                teeth,
                pressure_angle: 25.0,
                profile_shift: shift,
                addendum: 0.8,
                dedendum: 1.0,
                angular_shift: amplitude,
                index_offset: lambda,
                ..Default::default()
            });

            // Screen: doubling the points must nearly halve the largest jump.
            let mut previous = f64::MAX;
            for n in [600_usize, 1200, 2400, 4800] {
                let jump = radial_jump(&e.outline(n));
                assert!(
                    jump < 0.6 * previous,
                    "z={teeth} Δx={amplitude} λ={lambda}: {n} points a tooth jump {jump} mm \
                     against {previous} at half that — the root is stepping, not curving"
                );
                previous = jump;
            }

            // Export: the same, against its chord tolerance — but only where
            // the teeth are evenly seated. An adaptive polyline puts a long
            // chord along any nearly-straight run, and a steep one covers a lot
            // of radius inside its tolerance, so this measure reads a working
            // subdivider as a step. The seam that λ breaks is checked exactly
            // instead, in `the_root_of_one_tooth_meets_the_next`.
            if lambda != 0.0 {
                continue;
            }
            let mut previous = f64::MAX;
            for tol in [1e-2_f64, 1e-3, 1e-4] {
                let jump = radial_jump(
                    &e.outline_adaptive(tol)
                        .iter()
                        .map(|v| [v.x, v.y])
                        .collect::<Vec<_>>(),
                );
                assert!(
                    jump < 0.6 * previous,
                    "z={teeth} Δx={amplitude} λ={lambda}: at {tol} mm the export jumps {jump} \
                     against {previous} at ten times that"
                );
                previous = jump;
            }
        }
    }

    /// **One tooth's root meets the next where the two actually sit.**
    ///
    /// A tooth is drawn one *ideal* pitch wide, and the indexing offset λ moves
    /// each tooth by `λ(ψ̄ − ψ_k)` — a different amount for each. So the space
    /// between two teeth is a pitch **plus the difference of their offsets**, and
    /// drawing both to the ideal width leaves an angular gap wherever the seats
    /// spread and an overlap wherever they close. At λ = 1 on a Δx = 1 gear that
    /// is 0.009 rad, and the outline jumps across it.
    ///
    /// Exact rather than sampled, because it can be: the two reaches are angles,
    /// and they either meet or they do not. Every earlier continuity check here
    /// ran at λ = 0, which is the one value that hides this.
    #[test]
    fn the_root_of_one_tooth_meets_the_next() {
        for lambda in [0.0_f64, 0.5, 1.0, -1.0, 2.0] {
            let e = Eccentric::new(GearParams {
                pressure_angle: 25.0,
                teeth: 23,
                profile_shift: 0.2,
                addendum: 0.8,
                dedendum: 1.0,
                angular_shift: 1.0,
                index_offset: lambda,
                ..Default::default()
            });
            for k in 0..23_usize {
                let (_, seat) = e.tooth(k);
                let (_, next) = e.tooth((k + 1) % 23);
                // Both reaches are measured from their own tooth, so they meet
                // only if each is half the *actual* space between the two.
                let from_here = seat + e.reach(k, 1.0);
                let from_there = next - e.reach((k + 1) % 23, -1.0);
                // The wrap at k = z − 1 crosses zero, so compare modulo a turn.
                let gap = (from_here - from_there + std::f64::consts::PI)
                    .rem_euclid(std::f64::consts::TAU)
                    - std::f64::consts::PI;
                assert!(
                    gap.abs() < 1e-12,
                    "λ={lambda}: tooth {k}'s root ends at {from_here} and tooth {}'s \
                     begins at {from_there} — {gap} rad of nothing between them",
                    (k + 1) % 23
                );
            }
        }
    }

    /// **The root has no kink in it either.**
    ///
    /// Continuity in *value* was the first defect and is gated above. This is
    /// continuity in **slope**, which was the second: correcting only the flat
    /// root left it diving out of the fillet at 9 mm/rad where the envelope's
    /// own slope is 0.4, because the root arc spans about 0.005 rad and had to
    /// absorb 0.05 mm inside it. On screen that is a notch at the bottom of
    /// every tooth space.
    ///
    /// Gated the same way and for the same reason: a **kink keeps its angle**
    /// however finely the curve is sampled, while a smooth curve's turning angle
    /// falls with the spacing. Doubling the points must nearly halve it. No
    /// threshold is chosen, and a kink cannot hide under one.
    ///
    /// Measured over the region the correction touches — the fillet, the root,
    /// and a little flank either side of the junction between them. Not the tip,
    /// where the outline has a genuine corner that is the tooth rather than a
    /// defect; and **not merely below the base circle**, which was the first
    /// window and let a linear ramp through: its kink is at the *flank* junction,
    /// which sits above it.
    #[test]
    fn the_root_leaves_the_fillet_without_a_kink() {
        // A zero-length segment has no direction: consecutive teeth both emit
        // the mid-space point, so the seam carries a duplicate vertex whose
        // "turn" is whatever the last bits say.
        let turning = |o: &[[f64; 2]], below: f64| {
            let mut worst = 0.0_f64;
            for w in o.windows(3) {
                if w[1][0].hypot(w[1][1]) >= below {
                    continue;
                }
                let (ax, ay) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
                let (bx, by) = (w[2][0] - w[1][0], w[2][1] - w[1][1]);
                if ax.hypot(ay) < 1e-9 || bx.hypot(by) < 1e-9 {
                    continue;
                }
                worst = worst.max((ax * by - ay * bx).atan2(ax * bx + ay * by).abs());
            }
            worst
        };

        for (teeth, shift, amplitude) in
            [(24_u32, 0.0_f64, 0.4_f64), (24, 0.6, 0.4), (40, -0.3, 0.5)]
        {
            let e = Eccentric::new(GearParams {
                teeth,
                profile_shift: shift,
                angular_shift: amplitude,
                ..Default::default()
            });
            // **No tooth may be undercut here.** An undercut profile is
            // genuinely re-entrant and has a real corner where the flank turns
            // back — physics, not a defect, and it would hold this test's
            // turning angle open for ever. Asserted rather than assumed, since
            // a case that drifted across the undercut boundary would look like
            // the fix had broken.
            assert!(
                e.distinct().all(|g| !g.undercut),
                "z={teeth} x={shift} Δx={amplitude}: an undercut tooth has a corner of its own"
            );
            // Just past the flank/fillet junction, so the junction itself is
            // inside the window.
            let junction = e.mean().trochoid_at(e.mean().s_j).0;
            let below = junction * 1.02;
            let mut previous = f64::MAX;
            for n in [600_usize, 1200, 2400, 4800] {
                let turn = turning(&e.outline(n), below);
                assert!(
                    turn < 0.6 * previous,
                    "z={teeth} x={shift} Δx={amplitude}: at {n} points a tooth the root \
                     still turns {turn} rad against {previous} at half that — there is \
                     a kink in it, not a curve"
                );
                previous = turn;
            }
        }
    }

    /// A concentric gear's root is a genuine circle, so the export keeps it as
    /// an **exact arc** rather than subdividing it. The other half of the root
    /// change: it must not cost an ordinary gear its exactness.
    #[test]
    fn an_ordinary_gear_keeps_its_root_as_an_arc() {
        let flat = Eccentric::new(GearParams {
            teeth: 24,
            ..Default::default()
        })
        .outline_adaptive(1e-3);
        let arcs = flat.iter().filter(|v| v.bulge != 0.0).count();
        // Three arcs a tooth: the tip, and the root either side of it — the
        // root is emitted as two halves, one leading into the tooth and one
        // trailing out, meeting the neighbour's at mid-space.
        assert_eq!(arcs, 72, "an ordinary gear's tip and root are still arcs");

        // An eccentric one keeps its tip arcs and gives up its root ones, since
        // a varying root is not a circle at all.
        let ecc = Eccentric::new(GearParams {
            teeth: 24,
            angular_shift: 0.25,
            ..Default::default()
        })
        .outline_adaptive(1e-3);
        // One left: the tip. Both root halves gave up their arcs, since a
        // varying root is not a circle at all.
        assert_eq!(ecc.iter().filter(|v| v.bulge != 0.0).count(), 24);
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
