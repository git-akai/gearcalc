//! The crossed-axis mesh: point contact, along a line of action that cannot
//! turn.
//!
//! **Not a stage kind.** A worm stage and a crossed gear pair are one
//! [`super::PairStage`] whose shafts are not parallel, and this is the mesh
//! they share; the parallel mesh is [`super::pair`]'s. It fills the same
//! [`super::MeshReport`] a parallel mesh does — two efficiencies where the
//! other has one number twice, a sliding at the pitch point where the other
//! has zero, a patch that is an ellipse where the other's is a line, and the
//! zone as the faces leave it in [`super::PointContact`] — and what it lacks
//! is a bending rating, for the reason docs/reference.md#crossed-axes gives.
//! Both members are
//! ordinary [`super::GearResult`]s: a worm is a helical gear with a few starts
//! at a steep helix, and everything a gear can be asked it can be asked.
//!
//! The mesh mathematics is all in [`crate::screw`]. What this module adds is the
//! *stage*: materials, face widths, torque, backlash and the notes a designer
//! needs to see.
//!
//! # Backlash comes out of one projection — and two kinds of displacement
//!
//! Two things open a gap between the flanks: axial slack in the first member,
//! and a centre distance larger than nominal. Both are displacements, and both
//! matter only through their component along the common flank normal `n̂` —
//! the very same `n̂` the path of contact is built on
//! ([`crate::screw::Screw::contact_normal`]), projected rather than re-derived:
//!
//! ```text
//! j_n = j_axial · (â₁·n̂)   +   2 Δa · (x̂·n̂)
//!     = j_axial · sin β_b1  +   2 Δa · sin α_n
//! ```
//!
//! **The two are not the same kind of displacement, and the factor of two is
//! where that shows.** A centre distance above nominal *separates the axes*, and
//! a separation opens **both** flanks by the same amount; lost motion is the sum
//! of the gaps a member can travel across, so it counts twice. The worm's axial
//! float is a *rigid-body slide* along its own axis: it opens one flank exactly
//! as far as it closes the other, so running from one end of the float to the
//! other is lost once. Writing both as "a gap along `n̂`" and giving them one
//! coefficient is what left a factor of two in this model for a milestone
//! (docs/corrections.md).
//!
//! `sin α_n` there is an identity, not a small-angle reading: the contact
//! normal's component along the line of centres is `sin α_n` at **every** shaft
//! angle, which `screw.rs` gates and `tools/crossed_path.py` measures off the
//! surfaces.
//!
//! Closing the gap takes a rotation, and that conversion is
//! [`crate::mesh::angular_play`] — the law every mesh in this crate shares. It
//! is the same number as the `j_n / (r cos α_n sin γ)` this module used to
//! write for itself, since `r sin γ = z m_n / 2` exactly; the point of calling
//! the shared one is that a law with two homes is a law with two answers, and
//! the home nothing checks is the one that is wrong.
//!
//! At a right-angle drive the axial half reduces to the two relations the
//! handbooks give separately — the wheel turns by `j_axial/r₂`, and the worm by
//! `2π j_axial/lead` — which is the check the tests make.
//!
//! ## Where this law stops, and where the parallel one takes over
//!
//! A crossed pair's backlash is **exactly linear** in the centre-distance error,
//! because its line of action cannot rotate when the centres move. A parallel
//! pair's is not: at `Σ = 0` the constraint on `n̂` degenerates, the freed
//! rotation *is* the operating pressure angle, and `j_t = 2a′(inv α′ − inv α_w)`
//! carries the difference. The two agree to first order — gated both ways in the
//! tests below — and part company by 0.21 % at the default clearance. That step
//! at `Σ = 0` is the degeneracy, not a seam in the code.

use super::{
    Backlash, ContactPatch, GearResult, LoadCase, MeshReport, PairKind, PairResult, PairStage,
    PointContact, StageTorques, TrainError,
};

/// Quadrature points for the path-averaged friction balance.
///
/// Not a tuning parameter: the trapezium rule is second order and a test
/// asserts that the error quarters as the step halves, which fixes what any
/// count is worth. At this one the residual is below 1e-9 in efficiency —
/// four orders under the last digit anything reports.
const PATH_SAMPLES: usize = 2048;
use crate::contact::{Directional, Drive};
use crate::material::{contact_modulus, Material, MaterialLibrary};
use crate::mesh::MeshSide;
use crate::note::{key, Note};
use crate::params::{Auto, GearParams};
use crate::screw::{Screw, ZoneLimit};
use crate::tooth::Tooth;

/// The proportions a worm stage is conventionally given.
///
/// **These are conventions, not derivations, and they are shipped deliberately.**
/// docs/reference.md#contact-stress's standing policy refuses published *rating* factors, and the reason is
/// specific: a correction factor multiplies a stress, so shipping one outside
/// its validated band silently moves a number a designer will size a part
/// against. These are a different kind of thing. A worm's length and a wheel's
/// face width **enter no stress in this crate** — the contact is a point, and
/// the appendix records the measurement: the same mesh at 4 mm and at 40 mm of
/// face width gives a bit-identical peak pressure. So a recommendation here
/// informs a choice and cannot distort an answer, which is what makes it
/// admissible where a `K_v` is not.
///
/// They are reported *as* recommendations, with the source named on screen, and
/// the input stays editable. See docs/reference.md#crossed-axes.
///
/// # What they are for
///
/// A real worm stage has an enveloping wheel that wraps the worm, and both
/// dimensions are about **covering the zone of action**: the worm must be long
/// enough for the wheel to run off neither end, and the wheel wide enough to
/// take the thread but not so wide that its outer corners hang past where the
/// worm can touch. This crate models the pair as crossed-axis screw gearing
/// with point contact and does not derive that zone (docs/reference.md#crossed-axes, open), so the
/// proportions come from published practice rather than from our own geometry —
/// which is exactly why each carries its source.
pub mod proportions {
    /// Recommended minimum worm length (thread length), mm.
    ///
    /// ```text
    /// b₁ = (11 + c z₂) m_x,     c = 0.06 for z₁ < 4, 0.09 for z₁ ≥ 4
    /// ```
    ///
    /// DIN/ČSN practice, as tabulated by MITcalc's worm-gear geometry
    /// documentation. It is a **function of the wheel's tooth count**, which is
    /// what the specification asks for: a bigger wheel wraps further round the
    /// worm, so the worm must be longer to carry the contact. More starts steepen
    /// the lead and stretch the same wrap over more axial length, which is the
    /// step in `c`.
    ///
    /// Takes the **axial** module, because that is the module these proportions
    /// are written in and the one the worm's own pitch is measured in.
    #[must_use]
    pub fn worm_length(axial_module: f64, wheel_teeth: u32, starts: u32) -> f64 {
        let c = if starts < 4 { 0.06 } else { 0.09 };
        (11.0 + c * f64::from(wheel_teeth)) * axial_module
    }

    /// Recommended wheel face width, mm — BS 721.
    ///
    /// ```text
    /// b₂ = 2 m_x √(q + 1),   capped at 0.67 d₁,   q = d₁ / m_x
    /// ```
    ///
    /// Two statements from the same source, and the cap is the operative one on
    /// a slender worm: past about two thirds of the worm's reference diameter
    /// the wheel's outer corners are beyond the thread they were widened to
    /// catch, so the extra face carries nothing. `q`, the diameter quotient, is
    /// how worm practice expresses the worm's slenderness.
    #[must_use]
    pub fn wheel_face_width(axial_module: f64, worm_pitch_diameter: f64) -> f64 {
        let q = worm_pitch_diameter / axial_module;
        let recommended = 2.0 * axial_module * (q + 1.0).sqrt();
        recommended.min(0.67 * worm_pitch_diameter)
    }
}

/// Solve a pair whose shafts cross — a worm stage, or a crossed gear pair.
///
/// One solve for both, because they are one thing: crossed-axis screw gearing
/// between two involute helicoids. The kind decides one thing here, the
/// automatic face width where no rating sizes one ([`PairKind`]):
///
/// - a **worm** and its wheel take the conventional proportions of a worm
///   drive ([`proportions`]), which describe a worm carrying an enveloping
///   wheel and are offered nowhere else;
/// - a crossed **gear** pair takes the width at which one tooth pair hands
///   over to the next exactly — `ε = 1` — a geometric minimum that says
///   nothing about stress, because a point contact's peak pressure does not
///   depend on the face width and there is no bending model to invert.
///
/// # Errors
///
/// [`TrainError`] if the pair cannot exist or names a material the library does
/// not have.
pub fn solve_crossed_pair(
    stage: &PairStage,
    kind: PairKind,
    torques: StageTorques,
    lib: &MaterialLibrary,
) -> Result<PairResult, TrainError> {
    let input_torque = torques.peak_forward;

    // The shifts, decided once — a given distance is reached through them by
    // the rack law, or through the size where both are pinned
    // (`PairStage::first_pitch_diameter`).
    let chosen = stage.chosen_at(&crate::auto::Search::SHIPPED);
    let x = chosen.shifts;
    let p: [GearParams; 2] = [stage.params_at(0, x[0]), stage.params_at(1, x[1])];
    let g = [Tooth::new(p[0]), Tooth::new(p[1])];
    let s = stage.screw_at(x)?;

    let materials: Vec<Material> = stage
        .gears
        .iter()
        .map(|m| {
            lib.get(&m.material)
                .ok_or_else(|| TrainError::UnknownMaterial(m.material.clone()))
                .map(|found| found.overridden(&m.material_overrides))
        })
        .collect::<Result<_, _>>()?;

    let centre = operating_centre(&s, stage);
    // The tips are the teeth's own: this is the one place a crossed pair's
    // tooth form reaches an answer, which is why it is specified at all
    // (docs/reference.md#crossed-axes).
    let tips = [g[0].ra, g[1].ra];
    let ends = [g[0].flank_ends(), g[1].flank_ends()];

    // --- the face widths, decided before anything that reads them — the path
    // is one of those things, since a face narrow enough to cut the zone
    // changes what the mesh does as well as how long it does it for.
    //
    // At the centre distance the pair will run at, not the zero-backlash one:
    // an automatic face is sized for the machine that gets built. On a crossed
    // pair that is not a refinement — the contact slides bodily along the
    // shafts with a centre-distance error, so a face centred on the nominal
    // contact can miss it entirely (docs/reference.md#centre-distance-and-backlash).
    let full_path = s.path_of_contact_at(tips[0], tips[1], centre);
    let recommended: [Option<f64>; 2] = match kind {
        PairKind::Worm => [
            Some(proportions::worm_length(
                s.axial_module,
                stage.gears[1].teeth,
                stage.gears[0].teeth,
            )),
            Some(proportions::wheel_face_width(
                s.axial_module,
                s.worm_pitch_diameter,
            )),
        ],
        PairKind::Spur => [None, None],
    };
    let continuity = full_path
        .as_ref()
        .and_then(|path| path.face_widths_for(&s, 1.0));
    let mut notes = Vec::new();
    let widths: [f64; 2] = [0, 1].map(|i| {
        let width = &stage.gears[i].face_width;
        match (recommended[i], continuity) {
            (Some(r), _) => width.resolve(r),
            (None, Some(c)) => width.resolve(c[i]),
            (None, None) => {
                if width.auto {
                    notes.push(
                        Note::new(key::STAGE_CROSSED_FACE_WIDTH_AS_ENTERED)
                            .text("member", (i + 1).to_string()),
                    );
                }
                width.manual
            }
        }
    });

    // **Efficiency from the friction balance along the path, not at the pitch
    // point.** The pitch point is the one place with no sliding up the profile,
    // so a figure taken only there counts the sliding along the trace and
    // nothing else — which is why it tended to 100 % as the shafts came parallel
    // while the real pair keeps its profile loss (docs/reference.md#crossed-axes). The balance contains
    // the classical formula exactly: at the pitch point the two agree to 1e-12,
    // including the friction at which back-driving stops.
    //
    // Every figure this moves, it moves **down**: more sliding is counted, and
    // sliding cannot repay. See the canary note in docs/reference.md#crossed-axes.
    let path = full_path
        .as_ref()
        .map(|path| path.limited_by_face(&s, widths).map_or(*path, |(z, _)| z));
    // **Two coefficients, and only one of them is an efficiency.** The static
    // one decides whether the drive breaks away at all; the sliding one decides
    // how well it runs once it has. This is the stage kind the rule is *for* —
    // a worm is the only mesh here that sits near its own threshold — but the
    // rule lives in `Directional::once_moving` and every stage applies it.
    let with = |mu: f64| {
        Directional::of(|d| {
            path.and_then(|p| p.efficiency(&s, mu, d, PATH_SAMPLES))
                .unwrap_or_else(|| s.efficiency(mu, d))
        })
    };
    let at_rest = with(stage.static_friction);
    let efficiency = with(stage.sliding_friction).once_moving(&at_rest);
    // Quoted beside that efficiency, so it has to be the friction at which *it*
    // reaches zero rather than the pitch point's — and in **both** directions,
    // since a pair that cannot be driven forward has a threshold too and the
    // reader is owed it. The pitch-point form is the fallback per direction,
    // which is why this is not one `unwrap_or_else` over the pair.
    let pitch_point = s.locking_friction();
    let along_path = path.map(|p| p.locking_friction(&s, PATH_SAMPLES));
    let threshold = Directional::of(|d| {
        along_path
            .and_then(|t| *t.get(d))
            .unwrap_or_else(|| *pitch_point.get(d))
    });
    let output_torque = input_torque * s.ratio * efficiency.forward;

    // **Contact is rated on the torque the stage was given, on the member it was
    // given on**, and in the direction that gives it. See `Screw::normal_force`:
    // the efficiency *is* this balance, so reading the input torque on the worm
    // and the output torque on the wheel are the same number wherever the pair
    // transmits. Where it does not — a **forward-locked** pair, whose efficiency
    // is clamped to zero and whose output torque goes with it — only the first
    // reading survives, and the flanks are pressed by whatever is holding them
    // either way. It used to be the second, so a locked pair reported no flank
    // load at all.
    let e_star = contact_modulus(&materials[0], &materials[1]);
    let (curvature_along, curvature_across) =
        s.contact_curvatures().ok_or(TrainError::NoContact)?;

    // **How much contact line the teeth actually have.** The patch lengthens
    // along the rulings, and each member's flank runs along its own ruling for
    // `b / cos β_b` — its face seen in the plane the contact is a point in. The
    // shorter of the two is what bounds the patch, exactly as the narrower face
    // carries a parallel pair.
    //
    // Without this the crossed rating was the elliptical solution alone, which
    // assumes half-spaces of unlimited extent. At a worm's 90° the patch is a
    // small fraction of the face and the assumption costs nothing; as the shafts
    // come parallel the ellipse lengthens without bound and the pressure it
    // reports falls toward zero, while the real pair is carrying its load on a
    // line that has not grown at all (`docs/corrections.md`).
    let line_length = [0, 1]
        .map(|i| widths[i] / s.member_base(i).1.cos())
        .into_iter()
        .fold(f64::MAX, f64::min);

    // One rating, and what changes about it is the load: a torque, the member it
    // is quoted on, and the direction that presses that flank. The closure knows
    // nothing about which case or which direction asked.
    let rate = |torque: f64, on: MeshSide, drive: Drive| -> Result<ContactPatch, TrainError> {
        // **The patch is the governing model's.** `hertz::peak_pressure` is
        // the larger of the ellipse and the line the teeth have, and the
        // patch reported has to be the one that gave the number: a line of
        // the face's length and the line's half-width where the line governs,
        // the ellipse's two axes where it does. Reporting the ellipse's minor
        // axis under the line's pressure gave a near-parallel pair a patch a
        // quarter as wide as the line contact it was rated as.
        let patch_at = |along: f64, across: f64, force: f64| -> Option<(f64, [f64; 2])> {
            let line = crate::hertz::line_pressure(across, force, line_length, e_star);
            let ellipse = crate::hertz::elliptical_contact(along, across, force, e_star);
            let pressure = crate::hertz::peak_pressure(along, across, force, line_length, e_star)?;
            Some(match ellipse {
                Some(e) if e.max_pressure > line => (
                    pressure,
                    [
                        (2.0 * e.semi_major()).min(line_length),
                        2.0 * e.semi_minor(),
                    ],
                ),
                _ => (
                    pressure,
                    [
                        line_length,
                        2.0 * crate::hertz::line_half_width(across, pressure, e_star),
                    ],
                ),
            })
        };
        let force = s.normal_force(torque, on, stage.sliding_friction, drive);
        let (pitch_pressure, pitch_patch) =
            patch_at(curvature_along, curvature_across, force).ok_or(TrainError::NoContact)?;
        // **Rated along the path, not at the pitch point.** The relative radius
        // peaks where the two roll lengths are equal and falls toward both ends of
        // the zone; the pitch point sits near that peak, so rating there alone took
        // the mesh at close to its gentlest and flattered it.
        // The points that matter are the two boundaries of single-pair contact,
        // where one tooth carries everything; beyond them the next pair has taken
        // up. Where `ε ≤ 1` those boundaries are the ends of the zone, so a face
        // too narrow raises this figure as well as costing continuity.
        let mut max_pressure = pitch_pressure;
        let mut patch = pitch_patch;
        let mut worst_position = 0.0;
        // The curvatures travel with the worst point, as a line contact's do.
        let mut curvatures = (curvature_along, curvature_across);
        if let Some(path) = path {
            for position in path.single_pair_bounds(&s) {
                let contact = path.contact_at(&s, position);
                let Some((along, across)) = path.curvatures_at(&s, position) else {
                    continue;
                };
                // The force at *this* contact, not at the pitch point: a stress
                // evaluated in one place with a load computed in another is two
                // answers wearing one number.
                let Some(force) = contact.normal_force(torque, on, stage.sliding_friction, drive)
                else {
                    continue;
                };
                let Some((pressure, here)) = patch_at(along, across, force) else {
                    continue;
                };
                if pressure > max_pressure {
                    max_pressure = pressure;
                    patch = here;
                    worst_position = position;
                    curvatures = (along, across);
                }
            }
        }

        Ok(ContactPatch {
            max_pressure,
            at_pitch_point: pitch_pressure,
            worst_position,
            patch_length: patch[0],
            patch_width: patch[1],
            curvature_along: curvatures.0,
            curvature_across: curvatures.1,
        })
    };

    // **The peak is the worse of two directions, and each is rated in its own.**
    //
    // Driving forward the stage is handed a torque on its worm; being driven it
    // carries the applied load, which `back_driving_torques` has referred to
    // this stage's input shaft and which this brings back to the wheel by the
    // ratio alone. Two constructions, on two members, pressing two different
    // flanks — so the comparison is between the two **ratings** rather than
    // between two shaft torques, which is one number's worth of the same rule
    // that already governs a stage's load cases: *the peak is taken after each
    // direction's own distribution, never before it.*
    //
    // Taking the larger shaft torque first rated a back-driven pair at
    // `η_forward` of its load — 62 % of it on the shipped worm, so 79 % of the
    // contact stress (`docs/corrections.md`). Taking it on the wheel alone left
    // a forward-locked pair reporting no load at all.
    let on_worm = torques.on_mesh(input_torque, None);
    let forward_peak = rate(on_worm.peak, MeshSide::First, Drive::Forward)?;
    let backward_peak = torques
        .peak_backward
        .map(|t| rate(t * s.ratio, MeshSide::Second, Drive::Backward))
        .transpose()?;
    let contact = LoadCase {
        peak: match backward_peak {
            Some(back) if back.max_pressure > forward_peak.max_pressure => back,
            _ => forward_peak,
        },
        cyclic: rate(on_worm.cyclic, MeshSide::First, Drive::Forward)?,
    };

    // The screw law takes the *separation* from the geometric distance rather
    // than the distance itself, which is the only thing that differs from a
    // parallel stage's band. Per member, as every mesh reports it.
    let backlash = [MeshSide::First, MeshSide::Second].map(|at| {
        Backlash::banded(centre, stage.tolerance_minus, stage.tolerance_plus, |d| {
            angular_backlash(&s, stage, d - s.centre_distance, at).to_degrees()
        })
    });

    // **Against the static coefficient, because that is what decides it.**
    // Whether one member can turn the other at all is a question about breaking
    // away; the sliding coefficient answers a different one, and quoting it here
    // named the wrong number for the reader to go and change.
    //
    // **Asked in both directions.** A worm that cannot be back-driven is the
    // familiar case and the one the word "self-locking" is for; a crossed pair
    // at a steep helix split cannot be driven *forward*, and used to be
    // described only by a mesh efficiency reading `0.0 %`. Same construction,
    // roles swapped — the keys differ because the sentence differs, which is a
    // catalogue matter and not a computation.
    let locked = at_rest.locked();
    for drive in Drive::BOTH {
        let threshold = *threshold.get(drive);
        let (is_locked, is_near) = match drive {
            Drive::Backward => (key::STAGE_SELF_LOCKING, key::STAGE_NEAR_SELF_LOCKING),
            Drive::Forward => (key::STAGE_FORWARD_LOCKING, key::STAGE_NEAR_FORWARD_LOCKING),
        };
        let said = |k: &'static str| {
            Note::new(k)
                .number("friction", stage.static_friction, 3)
                .number("threshold", threshold, 4)
        };
        if *locked.get(drive) {
            notes.push(said(is_locked));
        } else if threshold > 0.0 && stage.static_friction > 0.8 * threshold {
            // **`threshold > 0.0` is load-bearing, not defensive.** A direction
            // no friction can lock has a negative threshold, and `µ > 0.8 × a
            // negative number` is true of every µ — so without this the "close
            // to locking" note fires on precisely the pairs that are furthest
            // from it. Forwards that is the ordinary case.
            notes.push(said(is_near));
        }
    }
    if efficiency.forward < 0.5 {
        notes.push(Note::new(key::STAGE_LOW_MESH_EFFICIENCY).number(
            "percent",
            efficiency.forward * 100.0,
            1,
        ));
    }
    // What the distance has to say: whether the shifts — or the size, where
    // both shifts are pinned — reached the one that was given, and whether the
    // pair can be assembled at all (`train::distance_notes`).
    notes.extend(super::distance_notes(
        stage.nominal_distance(),
        centre,
        s.centre_distance,
    ));

    // The zone as the widths in use actually leave it — one construction, asked
    // twice for different things, rather than two that can disagree about
    // where the teeth touch.
    // No zone at all — the teeth never meet — is a contact ratio of zero,
    // which the note below says as plainly as a short one.
    let (contact_ratio, zone) = full_path.as_ref().map_or((0.0, None), |path| {
        let (zone, limited_by) = path
            .limited_by_face(&s, widths)
            .unwrap_or((*path, ZoneLimit::Face));
        (
            zone.contact_ratio,
            Some((
                limited_by,
                path.face_widths_for(&s, 1.0),
                zone.axial_travel(&s),
            )),
        )
    });
    if contact_ratio < 1.0 {
        notes.push(
            Note::new(key::STAGE_CROSSED_CONTACT_RATIO_BELOW_ONE).number("ratio", contact_ratio, 3),
        );
    }
    // Interference is asked of the flanks whether or not there is a zone: a
    // pair with no zone and a tip inside a base cylinder is fouling, not idle.
    let flank_interference = full_path
        .as_ref()
        .map_or([true, true], |path| path.flank_interference(&s, ends));

    // The same teeth with their shafts brought parallel — which the stage can
    // describe exactly, since a crossed pair *is* this stage at `Σ = 0`. Here
    // to be compared against, not to correct with (`CrossedMesh::parallel_axis_efficiency`).
    let parallel = super::pair::solve_pair_stage(
        &PairStage {
            shaft_angle: 0.0,
            // The same helix on the first member, read as the split: at Σ = 0
            // the additional helix *is* the first member's angle.
            sizing: Auto::fixed(super::FirstMemberSizing::AdditionalHelix(
                stage.helix_angles()[0],
            )),
            // A comparison, not a design: the optimiser is not run on the
            // counterpart, whatever the stage asked of its own mesh.
            optimisation: super::Optimisation::default(),
            ..stage.clone()
        },
        kind,
        torques,
        lib,
    )
    .ok()
    .map(|r| r.mesh.efficiency.forward);

    let mut gears = Vec::with_capacity(2);
    let member_torque = [input_torque, output_torque];
    // **The reverse is the same construction with the roles swapped.** Driving
    // forward the first member is the input and the wheel carries its torque
    // stepped up and cut by the mesh's loss; being driven the wheel is the
    // input and the first member carries the applied load stepped *down* and
    // cut by the mesh's loss the other way. So the first member's figure is
    // the load referred to its shaft and then attenuated by this stage's own
    // backward efficiency — which is what the walk multiplies by on its way to
    // the stage before this one, so the number a worm reports is the number
    // it hands on. A self-locking worm's backward efficiency is zero and this
    // reads **zero**, which is the answer: nothing reaches the worm's shaft.
    //
    // The wheel's is the ratio and nothing else: `back_driving_torques` has
    // already referred the load to this stage's input shaft, so bringing it
    // back to the output member is that referral inverted. It used to *divide*
    // by the backward efficiency, and the wheel of the shipped worm stage
    // reported **2.2e307 N·m** (`docs/corrections.md`).
    // `StageTorques::referred_like` is deliberately not used on either member:
    // it scales a member's *forward* torque, and a worm's carries a forward
    // efficiency a backward load does not share.
    let back_driving = [
        torques
            .peak_backward
            .map(|t| t * efficiency.backward.max(0.0)),
        torques.peak_backward.map(|t| t * s.ratio),
    ];
    for i in 0..2 {
        let input = &stage.gears[i];
        gears.push(GearResult::of(super::MemberFacts {
            profile_shift: p[i].profile_shift,
            params: &p[i],
            input,
            rated: super::Rated {
                // **No bending, and that is a decision rather than a gap.** The
                // tooth a beam formula would measure is not the tooth this mesh
                // loads: a crossed pair's contact is a *point* tracking
                // diagonally across the flank, and a cantilever loaded across
                // its whole face has no honest reading of it
                // (docs/rationale.md#a-worm-stage-reports-no-bending-stress).
                bending_stress: LoadCase::of(|_| None),
                // The mesh's figure, which is the members' figure: two flanks
                // share one patch, one normal force and one `E*`
                // (docs/rationale.md#one-pressure-two-ratings--and-the-curvature-is-not-what-separates-them).
                // Both members are rated at the same point here because a point
                // contact has only the one.
                contact_stress: LoadCase::of(|c| contact.get(c).max_pressure),
                // **Neither rating sizes this face.** Bending is not taken, and
                // inverting a contact stress for a width assumes the stress
                // depends on the width — a point contact's does not. What sizes
                // a crossed pair's face is continuity, or a worm's proportions,
                // and both are reported as their own figure rather than
                // smuggled into this one.
                min_face_width: LoadCase::of(|_| super::Widths {
                    bending: None,
                    contact: None,
                }),
            },
            face_width: widths[i],
            recommended_face_width: recommended[i],
            torque: member_torque[i],
            back_driving_torque: back_driving[i],
            speed: 0.0,
            material: materials[i].clone(),
            clamps: g[i].clamps.notes.clone(),
            notes: {
                let mut out: Vec<Note> = super::undercut_note(&g[i]).into_iter().collect();
                out.extend(input.shift_asked(&stage.base_params(i)).note(input.teeth));
                out.extend(
                    input
                        .addendum_asked(&GearParams {
                            profile_shift: x[i],
                            ..stage.base_params(i)
                        })
                        .note(input.teeth),
                );
                out
            },
        }));
    }
    notes.extend(chosen.how.note());

    Ok(PairResult {
        ratio: s.ratio,
        centre_distance_nominal: s.centre_distance,
        centre_distance: centre,
        // As on a parallel stage: the running distance less the geometric
        // one, rather than the input read back.
        clearance: centre - s.centre_distance,
        mesh: MeshReport {
            coprime: super::gcd(stage.gears[0].teeth, stage.gears[1].teeth) == 1,
            contact_ratio,
            efficiency,
            locking_friction: threshold,
            sliding_ratio: s.sliding_ratio,
            sliding_velocity: 0.0,
            contact,
            backlash,
            flank_interference,
            // Both members external: the tips meet on the line of action or
            // not at all.
            tips: None,
            line: None,
            point: Some(PointContact {
                limited_by: zone.map_or(ZoneLimit::Face, |z| z.0),
                face_width_for_continuity: zone.and_then(|z| z.1),
                axial_travel: zone.map_or([0.0; 2], |z| z.2),
                parallel_axis_efficiency: parallel,
            }),
        },
        gears: [gears[0].clone(), gears[1].clone()],
        notes,
    })
}

/// The centre distance the pair actually runs at.
///
/// Nominal plus the assembly clearance when the centre distance is automatic,
/// and the number as entered when it is not — the specification's rule, and the
/// one both the rating and the backlash have to agree on. One home for it
/// because a stage asks twice: once inside the solve, and once before it, to
/// size an automatic face for the pair **as built** rather than for the
/// zero-backlash one nobody assembles.
fn operating_centre(s: &Screw, stage: &PairStage) -> f64 {
    if stage.centre_distance.auto {
        s.centre_distance + stage.clearance.manual
    } else {
        stage.centre_distance.manual
    }
}

/// Angular backlash at one member, radians, for a centre distance `delta` above
/// nominal.
///
/// The module documentation derives this; the short of it is that the axial
/// slack and the centre-distance change are two displacements, and only their
/// components along the common flank normal open a gap.
fn angular_backlash(s: &Screw, stage: &PairStage, delta: f64, at: MeshSide) -> f64 {
    // One normal, and both displacements projected onto it — the same vector the
    // path of contact is built on, rather than its components written out again.
    let Some(n) = s.contact_normal() else {
        return 0.0;
    };

    // **Two displacements, and they are not the same kind of thing.** Reading
    // them as one gap is what kept a factor of two wrong here (docs/corrections.md).
    //
    // A centre distance above nominal *separates the axes*, and a separation
    // opens **both** flanks by the same amount. Lost motion is the sum of the
    // gaps a member can travel across, so the factor is two — the same "a tooth
    // has two flanks" the path construction turns on.
    //
    // A worm's axial float is a **rigid-body slide** along its own axis, not a
    // separation: it opens one flank exactly as far as it closes the other, so
    // running from one end of the float to the other is lost once, not twice.
    //
    // `n[0]` is the centre line's projection and comes out `sin α_n` at every
    // shaft angle; `n[2]` is the worm axis's and is `sin β_b1`. Neither is a
    // small-angle reading — see `Screw::contact_normal`.
    let separation = 2.0 * delta.max(0.0) * n[0].abs();
    let slide = stage.axial_clearance * n[2].abs();

    let teeth = stage.gears[at.index()].teeth;
    // The gap becomes an angle by the law every mesh here shares. `r cos α_n
    // sin γ` is the same number — `r sin γ = z m_n / 2` exactly — but writing it
    // that way made this the second home of a law the parallel mesh already had,
    // and the second home is the one that got the wrong numerator.
    crate::mesh::angular_play(separation + slide, teeth, s.normal_base_pitch())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::{solve_pair_stage, FirstMemberSizing, StageGear};
    use super::*;

    fn library() -> MaterialLibrary {
        super::super::test_library()
    }

    /// The old worm entry point, as the tests were written against it.
    fn solve_worm(
        stage: &PairStage,
        torques: StageTorques,
        lib: &MaterialLibrary,
    ) -> Result<PairResult, TrainError> {
        solve_pair_stage(stage, PairKind::Worm, torques, lib)
    }

    /// The old crossed-gear entry point, likewise.
    fn solve_crossed(
        stage: &PairStage,
        torques: StageTorques,
        lib: &MaterialLibrary,
    ) -> Result<PairResult, TrainError> {
        solve_pair_stage(stage, PairKind::Spur, torques, lib)
    }

    /// Two members of a worm stage with these counts, otherwise at the preset.
    fn teeth(starts: u32, wheel: u32) -> [StageGear; 2] {
        let [worm, wheel_gear] = PairStage::worm().gears;
        [
            StageGear {
                teeth: starts,
                ..worm
            },
            StageGear {
                teeth: wheel,
                ..wheel_gear
            },
        ]
    }

    /// A stage with one member's inputs edited.
    fn member(mut stage: PairStage, i: usize, edit: impl FnOnce(&mut StageGear)) -> PairStage {
        edit(&mut stage.gears[i]);
        stage
    }

    /// The mesh a solved pair reports, checked to be the point contact a
    /// crossed pair has.
    fn point(r: &PairResult) -> &MeshReport {
        assert!(
            r.mesh.point.is_some() && r.mesh.line.is_none(),
            "a crossed pair reports a point contact"
        );
        &r.mesh
    }

    fn solved(stage: &PairStage) -> PairResult {
        solve_pair_stage(stage, PairKind::Worm, StageTorques::just(2.0), &library()).unwrap()
    }

    /// **A worm stage runs at the centre distance it was given** — by its
    /// wheel's shift while that is free, and by sizing the worm once both
    /// shifts are pinned.
    ///
    /// The second is the one kind of mode 3 that changes the *teeth* rather
    /// than where they sit, which is why it is what a pinned pair falls to and
    /// not the preference.
    ///
    /// Two claims, and the second is the interesting one. The distance asked for
    /// is the distance run at — and the answer stays on **the branch the
    /// designer's own number is on**, because a screw pair's centre distance has
    /// a minimum in the worm's diameter and a target above it is reached by two
    /// different worms.
    #[test]
    fn a_worm_sized_automatically_reaches_the_distance_it_was_given() {
        let free = PairStage::worm();
        let a0 = free
            .geometry()
            .expect("the shipped worm exists")
            .centre_distance;
        let turning = Screw::least_distance_lead_angle(
            free.gears[0].teeth,
            free.gears[1].teeth,
            free.shaft_angle.to_radians(),
        )
        .expect("a right-angle pair has one");
        let d_turning = f64::from(free.gears[0].teeth) * free.module / turning.sin();
        assert!(
            free.first_pitch_diameter() > d_turning,
            "the shipped worm should sit on the fat branch: {} against {d_turning}",
            free.first_pitch_diameter()
        );

        // **The wheel's shift absorbs a distance before the size does.** With
        // the wheel free — the preset — a given distance moves the wheel's
        // shift by the rack law and leaves the worm the size it was.
        {
            let mut stage = free.clone();
            stage.sizing.auto = true;
            stage.centre_distance = Auto::fixed(a0 + 0.5 + stage.clearance.manual);
            let x = stage.chosen_at(&crate::auto::Search::SHIPPED).shifts;
            assert!(
                (x[1] - 0.5 / stage.module).abs() < 1e-9 && x[0] == 0.0,
                "the wheel should take the half module: {x:?}"
            );
            assert!(
                (stage.first_pitch_diameter() - 7.0).abs() < 1e-12,
                "and the worm stays 7 mm while a shift is free: {}",
                stage.first_pitch_diameter()
            );
        }
        // Pin the wheel's shift as well, and the size is what is left.
        let free = member(free, 1, |g| g.profile_shift = Auto::fixed(0.0));

        let mut checked = 0u32;
        for step in 0..6 {
            let target = a0 + 0.5 * f64::from(step);
            let mut stage = free.clone();
            stage.sizing.auto = true;
            stage.centre_distance = Auto::fixed(target + stage.clearance.manual);
            let Ok(s) = stage.geometry() else { continue };
            checked += 1;

            assert!(
                (s.centre_distance - target).abs() < 1e-6,
                "asked {target}, sized to {} which runs at {}",
                stage.first_pitch_diameter(),
                s.centre_distance
            );
            // The fat branch, which is where the shipped worm is. The thin one
            // reaches the same distances with a wildly different worm, and
            // jumping between the two on a nudge is what continuity forbids.
            assert!(
                stage.first_pitch_diameter() >= d_turning,
                "the answer left the branch it started on: {} against {d_turning}",
                stage.first_pitch_diameter()
            );
        }
        assert!(checked >= 5, "only {checked} targets were reachable");

        // ...and from the thin branch, the thin answer.
        let mut thin = free.clone();
        thin.sizing = Auto::fixed(FirstMemberSizing::PitchDiameter(d_turning * 0.6));
        let from_thin = thin.geometry().expect("a thin worm exists").centre_distance;
        thin.sizing.auto = true;
        thin.centre_distance = Auto::fixed(from_thin + 0.4 + thin.clearance.manual);
        if let Ok(s) = thin.geometry() {
            assert!(
                (s.centre_distance - (from_thin + 0.4)).abs() < 1e-6,
                "the thin branch should reach its own target: {}",
                s.centre_distance
            );
            assert!(
                thin.first_pitch_diameter() <= d_turning,
                "a thin worm should stay thin: {}",
                thin.first_pitch_diameter()
            );
        }
    }

    /// **A crossed pair is rated on the contact line its teeth actually have.**
    ///
    /// The elliptical solution assumes half-spaces of unlimited extent, so as
    /// the shafts come parallel its patch lengthens without bound and the
    /// pressure it reports falls toward **zero** — while the real pair carries
    /// the same load on a contact line the face has not lengthened at all. The
    /// two models cross once, and the physical answer is the larger on each
    /// side of the crossing.
    ///
    /// This gates both sides of that, which is what makes it a gate rather than
    /// a demonstration:
    ///
    /// - near parallel the **line** term governs, and the elliptical figure the
    ///   rating used to report is far below it — 30–40 % at half a degree;
    /// - at a worm's right angle the **elliptical** term governs and the line
    ///   term is nowhere near, so nothing about a worm moves. Both canaries are
    ///   evidence of the same thing.
    ///
    /// Run against the elliptical-only code this fails at `Σ = 0.5°` with the
    /// rating 40 % under the line its own teeth provide.
    #[test]
    fn a_near_parallel_crossed_pair_is_rated_on_the_line_its_teeth_have() {
        use crate::hertz::elliptical_contact;

        let lib = library();
        // A near-parallel crossed pair, and the worm the canary is taken on.
        // Which term governs is the assertion; the two cases are here because
        // the answer has to be different at the two ends.
        // **Degrees, because that is what these two fields are.** Both used to
        // be handed `sigma_deg.to_radians()`, so "crossed 0.5°" built a pair at
        // Σ = 0.0087° and "crossed 1°" one at 0.0175° — the same point twice,
        // against a comment saying the two cases exist because the answer
        // differs at the two ends.
        let crossed = |sigma_deg: f64| PairStage {
            gears: teeth(17, 23),
            shaft_angle: sigma_deg,
            sizing: Auto::fixed(FirstMemberSizing::HelixAngle(sigma_deg / 2.0)),
            ..PairStage::worm()
        };
        let worm = PairStage {
            gears: teeth(1, 40),
            sizing: Auto::fixed(FirstMemberSizing::PitchDiameter(7.0)),
            ..PairStage::worm()
        };
        for (name, stage, line_should_govern) in [
            ("crossed 0.5°", crossed(0.5), true),
            ("crossed 1°", crossed(1.0), true),
            ("the worm canary", worm, false),
        ] {
            let s = stage.geometry().unwrap();
            let r = solve_worm(&stage, StageTorques::just(2.0), &lib).unwrap();

            // The same questions the stage asked, from the widths it actually
            // resolved: the load on the wheel, the line the teeth provide, and
            // the two models evaluated separately at the pitch point.
            let materials: Vec<Material> = [&stage.gears[0], &stage.gears[1]]
                .iter()
                .map(|m| lib.get(&m.material).unwrap().clone())
                .collect();
            let e_star = contact_modulus(&materials[0], &materials[1]);
            // **The load as the stage was given it**, on the member it was given
            // on. Reading it back off the wheel means multiplying by an
            // efficiency taken along the *path* and then using it in a balance
            // written at the *pitch point*, which is two models in one number:
            // it differs by 1 % on the near-parallel pair here, where the sweep
            // is longest.
            let force =
                s.normal_force(2.0, MeshSide::First, stage.sliding_friction, Drive::Forward);
            let (along, across) = s.contact_curvatures().unwrap();
            let betas = [
                s.worm_helix_angle_rad,
                s.shaft_angle_rad - s.worm_helix_angle_rad,
            ];
            let line_length = (0..2)
                .map(|i| {
                    r.gears[i].face_width
                        / crate::plane::base_helix_angle(betas[i], s.normal_pressure_angle_rad)
                            .cos()
                })
                .fold(f64::MAX, f64::min);

            let ellipse = elliptical_contact(along, across, force, e_star)
                .unwrap()
                .max_pressure;
            let line = (force / line_length * across * e_star / std::f64::consts::PI).sqrt();

            assert_eq!(
                line > ellipse,
                line_should_govern,
                "{name}: line {line:.1} against ellipse {ellipse:.1} — the sweep is \
                 not exercising the regime it claims to"
            );
            // ...and the rating is the larger of the two, whichever that is.
            let want = line.max(ellipse);
            assert!(
                (point(&r).contact.peak.at_pitch_point - want).abs() < 1e-9 * want,
                "{name}: rated at {} where the governing model gives {want}",
                point(&r).contact.peak.at_pitch_point
            );
            // A patch cannot be longer than the teeth it sits on.
            assert!(
                point(&r).contact.peak.patch_length <= line_length * (1.0 + 1e-12),
                "{name}: a {} mm patch on a {line_length} mm line",
                point(&r).contact.peak.patch_length
            );
        }
    }

    /// **Backlash, derived once and checked against the two rules the handbooks
    /// give separately.** Axial slack in the worm turns the wheel by
    /// `j/r₂`, and turns the worm itself by `2π j/lead`. Both come out of one
    /// projection onto the flank normal, which is the claim worth testing.
    #[test]
    fn axial_slack_reproduces_the_two_handbook_relations() {
        for (starts, wheel, d1) in [(1u32, 40u32, 7.0), (2, 31, 9.0), (4, 60, 14.0)] {
            let stage = PairStage {
                gears: teeth(starts, wheel),
                sizing: Auto::fixed(FirstMemberSizing::PitchDiameter(d1)),
                axial_clearance: 0.04,
                // isolate the axial term: no clearance on the centre distance
                clearance: Auto::fixed(0.0),
                tolerance_plus: 0.0,
                tolerance_minus: 0.0,
                ..PairStage::worm()
            };
            let r = solved(&stage);
            let s = stage.geometry().unwrap();

            let wheel = 0.04 / (s.wheel_pitch_diameter / 2.0);
            assert!(
                (r.mesh.backlash_by_drive().forward.nominal.to_radians() - wheel).abs()
                    < 1e-12 * wheel,
                "z₁={starts}: wheel backlash {} vs j/r₂ {}",
                r.mesh.backlash_by_drive().forward.nominal.to_radians(),
                wheel
            );

            let worm = std::f64::consts::TAU * 0.04 / s.lead;
            assert!(
                (r.mesh.backlash_by_drive().backward.nominal.to_radians() - worm).abs()
                    < 1e-12 * worm,
                "z₁={starts}: worm backlash {} vs 2π j/lead {}",
                r.mesh.backlash_by_drive().backward.nominal.to_radians(),
                worm
            );
        }
    }

    /// Opening the centre distance opens the flanks too, by its component along
    /// the normal — and closing it below nominal cannot make the backlash
    /// negative, because the flanks would simply touch.
    #[test]
    fn the_centre_distance_tolerance_moves_the_backlash_the_right_way() {
        let stage = PairStage {
            clearance: Auto::fixed(0.0),
            axial_clearance: 0.0,
            tolerance_plus: 0.05,
            tolerance_minus: 0.05,
            ..PairStage::worm()
        };
        let r = solved(&stage);
        assert_eq!(
            r.mesh.backlash_by_drive().forward.nominal,
            0.0,
            "nominal, with no slack at all"
        );
        assert!(
            r.mesh.backlash_by_drive().forward.maximum > 0.0,
            "opening the centres opens the mesh"
        );
        assert_eq!(
            r.mesh.backlash_by_drive().forward.minimum,
            0.0,
            "tighter than nominal is contact"
        );
    }

    /// The stage's headline numbers, and the one it deliberately does not have.
    #[test]
    fn a_worm_stage_reports_contact_and_two_efficiencies_and_no_bending() {
        let r = solved(&PairStage::worm());
        assert!((r.ratio - 40.0).abs() < 1e-12);
        assert!(r.mesh.efficiency.forward > 0.0 && r.mesh.efficiency.forward < 1.0);
        assert!(
            r.mesh.efficiency.backward < r.mesh.efficiency.forward,
            "back-driving is the worse direction"
        );
        assert!(
            point(&r).contact.peak.max_pressure > 0.0
                && point(&r).contact.peak.max_pressure.is_finite()
        );
        assert!(
            point(&r).contact.peak.curvature_along > 0.0,
            "crossed shafts make a point contact, not a line"
        );
        assert!(
            point(&r).contact.peak.patch_length > point(&r).contact.peak.patch_width,
            "an ellipse: {} by {}",
            point(&r).contact.peak.patch_length,
            point(&r).contact.peak.patch_width
        );
        // Torque follows the ratio and the operative efficiency.
        let expected = 2.0 * r.ratio * r.mesh.efficiency.forward;
        assert!((r.gears[1].torque - expected).abs() < 1e-12 * expected);
    }

    /// **Why the automatic face width here is a proportion and not a rating.**
    /// A point contact's peak pressure does not depend on the face width, so
    /// `σ_H ∝ 1/√b` — the relation a spur stage inverts to size a gear — has
    /// nothing to invert.
    ///
    /// That holds *while the face is not what bounds the zone of action*, which
    /// is the case here and on any sanely proportioned pair. Squeeze the face
    /// until it cuts the zone and the pressure does move — **upward**, because
    /// the pair loses the load sharing that kept the rating off the ends of the
    /// path. Which is the other reason nothing sizes a face from contact here:
    /// the relation runs the wrong way, and inverting it would ask for a
    /// *narrower* gear. The next test is that one.
    #[test]
    fn contact_pressure_does_not_depend_on_the_face_width() {
        let narrow = solved(&member(PairStage::worm(), 1, |g| {
            g.face_width = Auto::fixed(4.0);
        }));
        let wide = solved(&member(PairStage::worm(), 1, |g| {
            g.face_width = Auto::fixed(40.0);
        }));
        assert_eq!(
            point(&narrow).contact.peak.max_pressure,
            point(&wide).contact.peak.max_pressure,
            "a ten-fold face width must change nothing about a point contact"
        );
    }

    /// **The recommended proportions behave like the things they describe.**
    ///
    /// They are conventions, so there is no independent derivation to check
    /// them against — which makes it the more important to assert what they
    /// must *do* rather than the arithmetic they are. Each of these would catch
    /// a transcription error without repeating the transcription.
    #[test]
    fn the_recommended_proportions_behave_as_lengths_of_a_worm_drive() {
        use proportions::{wheel_face_width, worm_length};

        // A bigger wheel wraps further round the worm, so it needs more worm to
        // wrap onto. Strictly monotone in the wheel's tooth count.
        let lengths: Vec<f64> = (10..80).map(|z| worm_length(2.0, z, 1)).collect();
        assert!(
            lengths.windows(2).all(|w| w[1] > w[0]),
            "worm length must grow with the wheel it carries"
        );

        // More starts steepen the lead, so the same wrap is spread over more
        // axial length.
        assert!(worm_length(2.0, 40, 4) > worm_length(2.0, 40, 1));

        // Both scale with the module: they are proportions, so doubling the
        // whole drive doubles them exactly.
        for (a, b) in [
            (worm_length(1.0, 40, 2) * 2.0, worm_length(2.0, 40, 2)),
            (
                wheel_face_width(1.0, 7.0) * 2.0,
                wheel_face_width(2.0, 14.0),
            ),
        ] {
            assert!((a - b).abs() < 1e-12, "not homogeneous in the module");
        }

        // The BS 721 cap is the operative statement on a slender worm and not on
        // a stout one, which is the whole reason it is written as a cap.
        let slender = wheel_face_width(1.0, 5.0);
        assert!(
            (slender - 0.67 * 5.0).abs() < 1e-12,
            "the cap must bind at q = 5: {slender}"
        );
        let stout = wheel_face_width(1.0, 30.0);
        assert!(
            stout < 0.67 * 30.0,
            "the cap must not bind at q = 30: {stout}"
        );

        // A wheel wider than the worm's own diameter would hang past the thread
        // at every proportion in the range worm practice uses.
        for q in [6.0_f64, 8.0, 10.0, 14.0, 18.0, 25.0] {
            let b2 = wheel_face_width(1.0, q);
            assert!(b2 > 0.0 && b2 < q, "q={q}: face width {b2} against d1 {q}");
        }
    }

    /// **The automatic width is the recommendation, and a hand-set one survives
    /// untouched** — with the recommendation still reported beside it, so the
    /// two can be read against each other.
    #[test]
    fn automatic_takes_the_recommendation_and_manual_is_left_alone() {
        let auto = solved(&PairStage::worm());
        assert_eq!(
            Some(auto.gears[0].face_width),
            auto.gears[0].recommended_face_width,
            "an automatic worm length is the recommendation"
        );
        assert_eq!(
            Some(auto.gears[1].face_width),
            auto.gears[1].recommended_face_width,
            "an automatic wheel face width is the recommendation"
        );

        let manual = solved(&member(PairStage::worm(), 0, |g| {
            g.face_width = Auto::fixed(3.5);
        }));
        assert!((manual.gears[0].face_width - 3.5).abs() < 1e-12);
        assert_eq!(
            manual.gears[0].recommended_face_width, auto.gears[0].recommended_face_width,
            "the recommendation is reported whether or not it is in use"
        );
    }

    /// **The kind decides the proportions, not the reading.**
    ///
    /// The proportions describe a worm carrying an enveloping wheel. A crossed
    /// gear pair is two helical gears touching at a point, so quoting them
    /// there would be shipping a convention outside the case it was written
    /// for — and quietly, since the number would look like any other. The same
    /// 17/23 pair at 45° is a worm drive if a designer says it is and a gear
    /// pair otherwise, and only the kind says which: as a gear pair its
    /// automatic face is the width that keeps contact continuous, with no
    /// recommendation beside it.
    #[test]
    fn a_crossed_gear_pair_is_not_given_a_worms_proportions() {
        let stage = PairStage {
            gears: teeth(17, 23),
            sizing: Auto::fixed(FirstMemberSizing::HelixAngle(45.0)),
            ..PairStage::worm()
        };
        let lib = library();
        let as_gears = solve_crossed(&stage, StageTorques::just(2.0), &lib).unwrap();
        let as_worm = solve_worm(&stage, StageTorques::just(2.0), &lib).unwrap();
        assert!(as_gears.gears[0].recommended_face_width.is_none());
        assert!(as_gears.gears[1].recommended_face_width.is_none());
        assert!(as_worm.gears[0].recommended_face_width.is_some());
        assert!(as_worm.gears[1].recommended_face_width.is_some());
        // The gear pair's automatic face is continuity's.
        let continuity = point(&as_gears)
            .point
            .unwrap()
            .face_width_for_continuity
            .unwrap();
        for (i, (gear, want)) in as_gears.gears.iter().zip(continuity).enumerate() {
            assert!(
                (gear.face_width - want).abs() < 1e-9,
                "member {i}: face {} against continuity's {want}",
                gear.face_width
            );
        }
    }

    /// **A crossed stage reports its contact ratio, and an automatic face width
    /// is the width that keeps it at 1.**
    ///
    /// The two have to agree: size the face automatically, and the ratio that
    /// comes back must be exactly 1. Everything else here is a comparison —
    /// a hand-set face narrower than that gives less, a wider one gives what the
    /// teeth allow and no more, and the number moves the way the geometry says.
    #[test]
    fn an_automatic_face_width_on_a_crossed_pair_buys_exactly_continuous_contact() {
        use crate::params::Auto;
        use crate::train::{PairStage, StageGear};

        let lib = super::super::test_library();
        let gear = |teeth: u32, face: Auto<f64>| StageGear {
            teeth,
            face_width: face,
            ..StageGear::default()
        };
        let stage = |face: Auto<f64>| PairStage {
            shaft_angle: 90.0,
            gears: [gear(17, face), gear(23, face)],
            ..PairStage::default()
        };

        // Automatic: the width for ε = 1, and the ratio comes back as 1.
        let auto =
            solve_crossed(&stage(Auto::automatic(0.0)), StageTorques::just(2.0), &lib).unwrap();
        let m = point(&auto);
        assert!(
            (m.contact_ratio - 1.0).abs() < 1e-9,
            "automatic should buy exactly continuous contact, got {}",
            m.contact_ratio
        );
        let zone = m.point.expect("a crossed pair has a path of contact");
        assert_eq!(zone.limited_by, ZoneLimit::Face);
        let sized = zone
            .face_width_for_continuity
            .expect("a width for continuity");
        for (i, (member, want)) in auto.gears.iter().zip(sized).enumerate() {
            assert!(
                (member.face_width - want).abs() < 1e-9,
                "member {i} should be sized to {want}"
            );
        }

        // Half that face, about half the contact — and slightly less than half,
        // which is the point. The face is centred on its own gear and the
        // contact is not quite centred on the face, because the pair runs at its
        // nominal centre distance *plus the clearance* and that slides the
        // contact along the shafts (docs/reference.md#centre-distance-and-backlash). Trimming a face symmetrically about
        // an asymmetric contact loses a little more than half.
        let narrow = solve_crossed(
            &stage(Auto::fixed(sized[0] / 2.0)),
            StageTorques::just(2.0),
            &lib,
        )
        .unwrap();
        let n = point(&narrow);
        assert!(
            n.contact_ratio < 0.5 && n.contact_ratio > 0.45,
            "half the face should leave a little under half the contact: {}",
            n.contact_ratio
        );

        // ...and it is *exactly* half once the clearance is taken away, which
        // pins the shortfall on the slide rather than on the arithmetic.
        let tight = |face: Auto<f64>| PairStage {
            clearance: Auto::fixed(0.0),
            ..stage(face)
        };
        let centred =
            solve_crossed(&tight(Auto::automatic(0.0)), StageTorques::just(2.0), &lib).unwrap();
        let width = point(&centred)
            .point
            .expect("a path")
            .face_width_for_continuity
            .expect("a width for continuity");
        let halved = solve_crossed(
            &tight(Auto::fixed(width[0] / 2.0)),
            StageTorques::just(2.0),
            &lib,
        )
        .unwrap();
        assert!(
            (point(&halved).contact_ratio - 0.5).abs() < 1e-9,
            "with the contact centred, half the face is exactly half the contact"
        );
        assert!(
            narrow
                .notes
                .iter()
                .any(|s| s.is(key::STAGE_CROSSED_CONTACT_RATIO_BELOW_ONE)),
            "a contact ratio below 1 must be said: {:?}",
            narrow.notes
        );

        // Generous, and the teeth are what end it — a wider face buys nothing.
        let wide = solve_crossed(&stage(Auto::fixed(60.0)), StageTorques::just(2.0), &lib).unwrap();
        let w = point(&wide);
        assert_eq!(w.point.unwrap().limited_by, ZoneLimit::Tips);
        assert!(w.contact_ratio > m.contact_ratio);
        let wider =
            solve_crossed(&stage(Auto::fixed(120.0)), StageTorques::just(2.0), &lib).unwrap();
        assert!((point(&wider).contact_ratio - w.contact_ratio).abs() < 1e-12);
    }

    /// **A worm stage reports the same path, from its own teeth, and keeps its
    /// proportions.**
    ///
    /// Every other figure a worm stage gives comes from a model in which both
    /// flanks are involute helicoids on cylinders, so withholding the one that
    /// comes from the same construction would be holding it to a standard
    /// nothing else here meets. The tips are the members' own now — a worm has
    /// an addendum like any gear — where a separate worm type once had to
    /// assume one and flag it.
    ///
    /// Its face width stays proportion-sized: BS 721 and DIN are worm-drive
    /// conventions and answer a different question from continuity of contact.
    /// Both numbers are reported, each labelled with what it is.
    #[test]
    fn a_worm_drive_reports_the_path_from_its_own_teeth_and_keeps_its_proportions() {
        let r = solved(&PairStage::worm());
        let m = point(&r);
        assert!(m.contact_ratio > 0.0);
        // The proportions still size the face; continuity is reported beside
        // them rather than instead of them.
        assert!(r.gears[0].recommended_face_width.is_some());
        assert!(m.point.unwrap().face_width_for_continuity.is_some());

        // The tips are the teeth's: a taller worm thread lengthens the zone.
        let taller = solved(&member(PairStage::worm(), 0, |g| g.addendum = 1.2));
        assert!(
            point(&taller).contact_ratio > m.contact_ratio,
            "a taller addendum must reach further: {} against {}",
            point(&taller).contact_ratio,
            m.contact_ratio
        );
    }

    /// **The rating walks the path, and the pitch point is near its gentlest
    /// place.**
    ///
    /// The relative radius peaks where the two roll lengths are equal, which the
    /// pitch point is near, so a figure taken there alone is close to the least
    /// severe the mesh offers.
    /// Checked as an ordering rather than as a number — the rated pressure can
    /// only be at or above what the pitch point says — and then as the
    /// consequence that matters: squeeze the face until `ε` falls below 1, and
    /// the same mesh rates **higher**, because with nothing else in contact the
    /// one pair carries the load out to the ends of the zone.
    #[test]
    fn rating_along_the_path_is_never_kinder_than_the_pitch_point() {
        use crate::params::Auto;
        use crate::train::{PairStage, StageGear};

        let lib = super::super::test_library();
        let crossed = |face: f64| PairStage {
            shaft_angle: 90.0,
            gears: [
                StageGear {
                    teeth: 17,
                    face_width: Auto::fixed(face),
                    ..StageGear::default()
                },
                StageGear {
                    teeth: 23,
                    face_width: Auto::fixed(face),
                    ..StageGear::default()
                },
            ],
            ..PairStage::default()
        };

        // Generous face: the teeth end the zone, the pair shares load, and the
        // rating sits just above the pitch-point figure.
        let wide = solve_crossed(&crossed(20.0), StageTorques::just(2.0), &lib).unwrap();
        assert!(
            point(&wide).contact.peak.max_pressure >= point(&wide).contact.peak.at_pitch_point,
            "the path cannot be kinder than its gentlest point"
        );
        assert!(point(&wide).contact_ratio > 1.0);

        // Squeeze it: ε falls below 1, load sharing goes, and the same mesh at
        // the same torque rates higher.
        let narrow = solve_crossed(&crossed(0.8), StageTorques::just(2.0), &lib).unwrap();
        assert!(point(&narrow).contact_ratio < 1.0);
        assert!(
            point(&narrow).contact.peak.max_pressure > point(&wide).contact.peak.max_pressure,
            "losing contact continuity should cost stress, not save it: {} against {}",
            point(&narrow).contact.peak.max_pressure,
            point(&wide).contact.peak.max_pressure
        );
        // ...and it is the *place* being rated that did it, not the load. The
        // load moved too — a narrower face engages only the middle of the zone,
        // which slides less, which delivers more torque — so the two figures
        // are compared as a ratio, which divides the load out: the same mesh
        // rated at its worst against rated at its pitch point.
        let severity = |r: &PairResult| {
            point(r).contact.peak.max_pressure / point(r).contact.peak.at_pitch_point
        };
        // The margin is small, and the reason is worth knowing: a narrow face
        // cuts the zone's *ends* off, and those ends are what made the path
        // severe. So losing the sharing pushes the rating outward while the
        // shortened zone pulls it back in, and the net is slight — 1.012
        // against 1.007. Asserted as the ordering, since the two effects fight
        // and a margin would be a guess about which wins by how much.
        assert!(
            severity(&narrow) > severity(&wide),
            "losing load sharing should move the rating further from the pitch \
             point: {} against {}",
            severity(&narrow),
            severity(&wide)
        );
    }

    /// **Counting more sliding can only cost efficiency**, and the stage's
    /// figure is now the path average rather than the pitch point.
    ///
    /// The pitch point is the one place on the path with no sliding up the
    /// profile, so averaging over the path can only find more of it. That the
    /// stage's number is at or below the classical one — in **both** directions,
    /// across worms and crossed pairs — is the law this integration has to obey,
    /// and it is asserted rather than the numbers it produced.
    ///
    /// The self-locking threshold moves with it, and must: quoted beside an
    /// efficiency, it has to be the friction at which *that* efficiency reaches
    /// zero. More sliding means locking at less friction, so the threshold falls
    /// too — and the pair reported as self-locking is the one whose reported
    /// backward efficiency is not positive.
    #[test]
    fn the_stage_counts_all_the_sliding_and_can_only_lose_by_it() {
        use crate::params::Auto;
        use crate::train::{PairStage, StageGear};

        let lib = super::super::test_library();

        for stage in [
            PairStage::worm(),
            PairStage {
                gears: teeth(2, 40),
                sizing: Auto::fixed(FirstMemberSizing::PitchDiameter(12.0)),
                ..PairStage::worm()
            },
        ] {
            let r = solve_worm(&stage, StageTorques::just(2.0), &lib).unwrap();
            let classical = point(&r).point.map(|_| ()).map(|()| {
                let s = stage.geometry().unwrap();
                Directional::of(|d| s.efficiency(stage.sliding_friction, d))
            });
            let classical = classical.expect("a screw geometry");
            for drive in Drive::BOTH {
                assert!(
                    r.mesh.efficiency.get(drive) <= classical.get(drive),
                    "{drive:?}: the path average {} should not beat the pitch point {}",
                    r.mesh.efficiency.get(drive),
                    classical.get(drive)
                );
            }
            // The threshold is the friction at which the *reported* backward
            // efficiency reaches zero, so it moves down with it.
            assert!(
                point(&r).locking_friction.backward
                    <= stage.geometry().unwrap().locking_friction().backward
            );
            assert_eq!(
                r.mesh.efficiency.locked().backward,
                r.mesh.efficiency.backward <= 0.0,
                "self-locking is what the reported figure says, not a second opinion"
            );
        }

        // ...and the same for a crossed gear pair, where the term being added is
        // the whole of a parallel mesh's loss.
        let crossed = PairStage {
            shaft_angle: 60.0,
            gears: [
                StageGear {
                    teeth: 17,
                    face_width: Auto::fixed(12.0),
                    ..StageGear::default()
                },
                StageGear {
                    teeth: 43,
                    face_width: Auto::fixed(12.0),
                    ..StageGear::default()
                },
            ],
            ..PairStage::default()
        };
        let r = solve_crossed(&crossed, StageTorques::just(2.0), &lib).unwrap();
        point(&r).point.expect("a path");
        assert!(
            r.mesh.efficiency.forward < point(&r).point.unwrap().parallel_axis_efficiency.unwrap(),
            "a crossed pair must still lose more than the same teeth parallel"
        );
    }

    /// **Crossing shafts adds sliding, and the figures show it** — with one
    /// exception that is the *other* formula's approximation, not this one's.
    ///
    /// A crossed pair loses more than the same teeth running parallel, and by
    /// more the further the shafts are turned. At a shaft angle small enough
    /// that the two are the same mesh, the balance comes out a hundredth of a
    /// point *above* the parallel closed form — because that closed form is
    /// first order in `μ` and the balance is exact in it. `screw.rs` measures
    /// that difference as O(μ²); here it is allowed for and bounded.
    #[test]
    fn a_crossed_pair_loses_more_the_further_its_shafts_are_turned() {
        use crate::params::Auto;
        use crate::train::{PairStage, StageGear};

        let lib = super::super::test_library();
        let stage = |sigma: f64| PairStage {
            shaft_angle: sigma,
            sizing: Auto::fixed(FirstMemberSizing::AdditionalHelix(20.0)),
            gears: [
                StageGear {
                    teeth: 17,
                    // Wide enough that the **tips** end the zone at every shaft
                    // angle below, which is what makes this a like-for-like
                    // comparison. A 12 mm face is not: near the parallel limit a
                    // centre-distance error slides the contact several
                    // millimetres along the shafts, the face cuts the zone
                    // short, and comparing a partly engaged pair against a fully
                    // engaged one measures the truncation rather than the
                    // sliding. That effect has its own test below.
                    face_width: Auto::fixed(60.0),
                    ..StageGear::default()
                },
                StageGear {
                    teeth: 43,
                    face_width: Auto::fixed(60.0),
                    ..StageGear::default()
                },
            ],
            ..PairStage::default()
        };

        let mut previous = 1.0;
        for sigma in [0.5f64, 2.0, 10.0, 45.0, 90.0] {
            let r = solve_crossed(&stage(sigma), StageTorques::just(2.0), &lib).unwrap();
            let mesh = point(&r).point.expect("a path");
            assert_eq!(
                mesh.limited_by,
                ZoneLimit::Tips,
                "Σ={sigma}°: the face is cutting the zone, so this is not a \
                 like-for-like comparison"
            );
            let parallel = point(&r)
                .point
                .unwrap()
                .parallel_axis_efficiency
                .expect("a parallel counterpart");
            assert!(
                r.mesh.efficiency.forward < previous,
                "Σ={sigma}°: turning the shafts further must cost more"
            );
            previous = r.mesh.efficiency.forward;

            // Below the parallel figure everywhere the difference is physical,
            // and above it by no more than that formula's own linearisation
            // where the two are the same mesh.
            assert!(
                r.mesh.efficiency.forward < parallel + 2e-4,
                "Σ={sigma}°: {} against the parallel {parallel}",
                r.mesh.efficiency.forward
            );
        }

        // A worm has one too now — it is a helical gear with one start — and
        // the same ordering holds against it: crossing the shafts to a right
        // angle costs a single-start worm most of what it had.
        let worm = solved(&PairStage::worm());
        let parallel = point(&worm)
            .point
            .unwrap()
            .parallel_axis_efficiency
            .expect("a one-start helical gear on parallel shafts is buildable");
        assert!(
            worm.mesh.efficiency.forward < parallel,
            "the worm keeps {} against {parallel} with its shafts parallel",
            worm.mesh.efficiency.forward
        );
    }

    /// A self-locking pair says so, rather than reporting a negative efficiency
    /// and leaving the reader to notice.
    #[test]
    fn self_locking_is_said_out_loud() {
        let r = solved(&PairStage {
            gears: teeth(1, 40),
            sizing: Auto::fixed(FirstMemberSizing::PitchDiameter(25.0)),
            sliding_friction: 0.06,
            ..PairStage::worm()
        });
        assert!(r.mesh.efficiency.locked().backward);
        assert!(
            r.notes
                .iter()
                .any(|n| n.is(key::STAGE_SELF_LOCKING) || n.is(key::STAGE_NEAR_SELF_LOCKING)),
            "notes: {:?}",
            r.notes
        );
    }

    #[test]
    fn a_pair_that_cannot_exist_says_which_way_it_failed() {
        let err = solve_worm(
            &PairStage {
                gears: teeth(9, 40),
                sizing: Auto::fixed(FirstMemberSizing::PitchDiameter(8.0)),
                ..PairStage::worm()
            },
            StageTorques::just(2.0),
            &library(),
        )
        .unwrap_err();
        assert!(format!("{err}").contains("too thin"), "{err}");

        let err = solve_worm(
            &member(PairStage::worm(), 1, |g| g.material = "Unobtainium".into()),
            StageTorques::just(2.0),
            &library(),
        )
        .unwrap_err();
        assert!(matches!(err, TrainError::UnknownMaterial(_)), "{err}");
    }
    /// **A crossed gear pair is this same stage, sized the other way.**
    ///
    /// Giving the first member a helix angle instead of a diameter must produce a
    /// lead angle of exactly `90° − β₁`, and a second member at `β₂ = Σ − β₁`.
    /// That is the whole of docs/reference.md#crossed-axes's claim that worm and crossed-helical gearing
    /// are one thing, and it is checkable without knowing any answers.
    #[test]
    fn a_helix_angle_and_a_pitch_diameter_describe_the_same_pair() {
        for (z1, z2) in [(17u32, 23u32), (20, 20), (1, 40)] {
            for sigma in [45.0f64, 60.0, 90.0] {
                for beta1 in [15.0f64, 30.0, 45.0] {
                    if sigma - beta1 < 0.0 || sigma - beta1 > 89.0 {
                        continue;
                    }
                    let by_angle = PairStage {
                        gears: teeth(z1, z2),
                        shaft_angle: sigma,
                        sizing: Auto::fixed(FirstMemberSizing::HelixAngle(beta1)),
                        ..PairStage::worm()
                    };
                    let Ok(a) = by_angle.geometry() else { continue };
                    assert!(
                        (a.lead_angle_rad.to_degrees() - (90.0 - beta1)).abs() < 1e-9,
                        "z={z1}/{z2} S={sigma} b={beta1}: lead angle {}",
                        a.lead_angle_rad.to_degrees()
                    );
                    assert!(
                        (a.wheel_helix_angle_rad.to_degrees() - (sigma - beta1)).abs() < 1e-9,
                        "z={z1}/{z2} S={sigma} b={beta1}: wheel helix {}",
                        a.wheel_helix_angle_rad.to_degrees()
                    );

                    // ...and handing the derived diameter back as a diameter is
                    // the same pair, which is what "sized the other way" means.
                    let by_diameter = PairStage {
                        sizing: Auto::fixed(FirstMemberSizing::PitchDiameter(
                            by_angle.first_pitch_diameter(),
                        )),
                        ..by_angle.clone()
                    };
                    let b = by_diameter.geometry().unwrap();
                    assert_eq!(a.lead_angle_rad, b.lead_angle_rad);
                    assert_eq!(a.wheel_pitch_diameter, b.wheel_pitch_diameter);
                    assert_eq!(a.centre_distance, b.centre_distance);
                }
            }
        }
    }

    /// **A crossed-axis *spur* pair works, with the spur member second.**
    ///
    /// Put the helical member first at `β₁ = Σ` and the mate comes out at
    /// `β₂ = 0` — an ordinary spur gear, crossed. Which member is "first" is a
    /// labelling choice, and this is the labelling that has a lead angle to
    /// speak of.
    ///
    /// The other order is genuinely not representable, and for a reason worth
    /// stating rather than working around: at `β₁ = 0` the first member's lead
    /// angle is exactly 90°, so its *lead* and *axial module* are infinite. Not a
    /// numerical artefact — a spur gear is not a screw, and it has no lead. The
    /// stage refuses that ordering and says so.
    #[test]
    fn a_crossed_axis_spur_pair_puts_the_spur_member_second() {
        for sigma in [20.0f64, 30.0, 45.0, 60.0] {
            let stage = PairStage {
                gears: teeth(17, 23),
                shaft_angle: sigma,
                // The helical member takes the whole shaft angle...
                sizing: Auto::fixed(FirstMemberSizing::HelixAngle(sigma)),
                ..PairStage::worm()
            };
            let s = stage
                .geometry()
                .unwrap_or_else(|e| panic!("Sigma={sigma}: {e}"));
            // ...leaving the mate a spur gear.
            assert!(
                s.wheel_helix_angle_rad.abs() < 1e-9,
                "Sigma={sigma}: wheel helix {}",
                s.wheel_helix_angle_rad.to_degrees()
            );
            // Its pitch diameter is then the plain `z m_n`, no helix correction.
            assert!((s.wheel_pitch_diameter - 23.0).abs() < 1e-12);
            // And everything reported stays finite — the singular quantities are
            // the *first* member's, and the first member is the helical one.
            assert!(s.lead.is_finite() && s.axial_module.is_finite());
            assert!(s.sliding_ratio.is_finite() && s.centre_distance.is_finite());
        }

        // The other way round is refused, not fudged.
        let backwards = PairStage {
            gears: teeth(17, 23),
            shaft_angle: 30.0,
            sizing: Auto::fixed(FirstMemberSizing::HelixAngle(0.0)),
            ..PairStage::worm()
        };
        assert!(
            backwards.geometry().is_err(),
            "a spur first member has no lead angle to report"
        );
    }

    /// **A ninety-degree helix is a disc, not a gear**, and it has to be refused
    /// where the helix angle is still known.
    ///
    /// `cos 90°` is 6e-17 rather than zero, so a derived pitch diameter comes out
    /// *enormous* rather than infinite — 2.8e17 mm on a 17-tooth member — and
    /// then `sin γ₁ = z m_n / d` is 6e-17, so the `WormTooThin` guard at the
    /// other end of the range sees nothing wrong. Found by sweeping the helix
    /// split in `gear-cli crossed` and reading the last row.
    #[test]
    fn a_ninety_degree_helix_is_refused_where_it_is_still_visible() {
        for beta in [90.0f64, 91.0, 120.0, -90.0] {
            let stage = PairStage {
                gears: teeth(17, 23),
                shaft_angle: 90.0,
                sizing: Auto::fixed(FirstMemberSizing::HelixAngle(beta)),
                ..PairStage::worm()
            };
            assert!(
                stage.geometry().is_err(),
                "beta={beta}: a disc is not a gear"
            );
        }
        // Just inside is silly but representable — the project's standing rule is
        // that a limit answers "could this exist", not "would anyone want it".
        let stage = PairStage {
            gears: teeth(17, 23),
            shaft_angle: 90.0,
            sizing: Auto::fixed(FirstMemberSizing::HelixAngle(89.0)),
            ..PairStage::worm()
        };
        let g = stage.geometry().unwrap();
        assert!(g.worm_pitch_diameter.is_finite() && g.worm_pitch_diameter > 0.0);
    }

    /// **The merged stage is the same machine, to the last bit.**
    ///
    /// A crossed pair used to be entered as a worm stage sized by helix angle;
    /// it is now a spur stage with a shaft angle, which is what the
    /// specification asks for. That is a change of *input*, so the answer must
    /// not move — and this compares the two routes rather than asserting
    /// remembered numbers, so it keeps meaning something as the model changes.
    #[test]
    fn a_crossed_spur_stage_is_the_screw_stage_it_used_to_be_entered_as() {
        use crate::params::Auto;
        use crate::train::{PairStage, StageGear};

        let lib = super::super::test_library();
        let gear = |teeth: u32| StageGear {
            teeth,
            face_width: Auto::fixed(8.0),
            ..StageGear::default()
        };
        let spur = PairStage {
            shaft_angle: 90.0,
            sizing: Auto::fixed(FirstMemberSizing::AdditionalHelix(0.0)),
            gears: [gear(17), gear(23)],
            ..PairStage::default()
        };
        // The same pair entered the worm way: by the first member's helix,
        // as a worm kind, with the worm preset's float taken off.
        let as_screw = PairStage {
            shaft_angle: 90.0,
            sizing: Auto::fixed(FirstMemberSizing::HelixAngle(45.0)),
            axial_clearance: 0.0,
            gears: [gear(17), gear(23)],
            ..PairStage::worm()
        };

        let a = solve_crossed(&spur, StageTorques::just(2.0), &lib).unwrap();
        let b = solve_worm(&as_screw, StageTorques::just(2.0), &lib).unwrap();
        for (name, x, y) in [
            ("ratio", a.ratio, b.ratio),
            ("centre distance", a.centre_distance, b.centre_distance),
            ("lead angle", a.gears[0].lead_angle, b.gears[0].lead_angle),
            (
                "efficiency",
                a.mesh.efficiency.forward,
                b.mesh.efficiency.forward,
            ),
            (
                "contact",
                point(&a).contact.peak.max_pressure,
                point(&b).contact.peak.max_pressure,
            ),
            (
                "backlash",
                a.mesh.backlash_by_drive().forward.nominal,
                b.mesh.backlash_by_drive().forward.nominal,
            ),
        ] {
            assert_eq!(x.to_bits(), y.to_bits(), "{name}: {x} against {y}");
        }

        // The helix angles are what the shaft angle says they are, and they sum
        // to it — the relation the screw model runs on.
        let [b1, b2] = spur.helix_angles();
        assert!((b1 - 45.0).abs() < 1e-12 && (b2 - 45.0).abs() < 1e-12);
        assert!((b1 + b2 - spur.shaft_angle).abs() < 1e-12);
    }

    /// **A parallel stage is the shaft angle's zero, not a separate thing.**
    ///
    /// At `Σ = 0` the additional helix is the whole of each gear's helix and the
    /// two hands are opposed, which is exactly what the stage did before it had
    /// a shaft angle at all. Asserted on the geometry the mesh is built from,
    /// so it holds whatever the solve does with it.
    #[test]
    fn a_parallel_stage_is_the_shaft_angles_zero() {
        use crate::train::PairStage;

        for additional in [0.0_f64, 12.5, -30.0] {
            let stage = PairStage {
                sizing: Auto::fixed(FirstMemberSizing::AdditionalHelix(additional)),
                ..PairStage::default()
            };
            let [b1, b2] = stage.helix_angles();
            assert!((b1 - additional).abs() < 1e-12);
            assert!((b2 + additional).abs() < 1e-12, "the hands must oppose");
            assert!(!stage.is_crossed());
        }
    }

    /// **The crossed backlash law and the parallel one meet where the shafts
    /// straighten — and the gate is the meeting, not a number.**
    ///
    /// This is the check that was missing while the crossed centre-distance term
    /// counted one flank where two open, and a factor of two sat in the model
    /// for a milestone (docs/corrections.md): every crossed backlash test held the crossed model
    /// against itself, and the parallel model — which has the exact involute law
    /// and its own tests — was never asked.
    ///
    /// They are **not the same law** and must not be forced to be. A parallel
    /// pair's line of action rotates as the centres separate, so its backlash is
    /// exactly `2a′(inv α′ − inv α_w)` and nonlinear in the error. A crossed
    /// pair's line cannot rotate (`Screw::contact_normal`), so its backlash is
    /// exactly linear. What must hold is that they agree to **first order**, and
    /// the way to test that without picking a tolerance out of the air is to
    /// watch the shortfall vanish *with the error*: halve the clearance and the
    /// relative gap must halve too.
    #[test]
    fn the_crossed_backlash_meets_the_parallel_law_at_its_limit() {
        use crate::params::Auto;
        use crate::train::pair::solve_pair_stage;
        use crate::train::{PairStage, StageGear};

        let lib = super::super::test_library();
        let stage = |sigma: f64, clearance: f64| PairStage {
            shaft_angle: sigma,
            sizing: Auto::fixed(FirstMemberSizing::AdditionalHelix(20.0)),
            clearance: Auto::fixed(clearance),
            gears: [
                StageGear {
                    teeth: 17,
                    face_width: Auto::fixed(8.0),
                    ..StageGear::default()
                },
                StageGear {
                    teeth: 43,
                    face_width: Auto::fixed(8.0),
                    ..StageGear::default()
                },
            ],
            ..PairStage::default()
        };

        let mut last = f64::INFINITY;
        for clearance in [0.08_f64, 0.04, 0.02, 0.01, 0.005] {
            let parallel = solve_pair_stage(
                &stage(0.0, clearance),
                PairKind::Spur,
                StageTorques::just(2.0),
                &lib,
            )
            .expect("a parallel pair")
            .mesh
            .backlash_by_drive()
            .forward
            .nominal;
            // As close to parallel as the screw model will go. The pair is still
            // crossed, so it is still the crossed law answering.
            let crossed = solve_crossed(&stage(0.001, clearance), StageTorques::just(2.0), &lib)
                .expect("a crossed pair")
                .mesh
                .backlash_by_drive()
                .forward
                .nominal;

            assert!(
                parallel > 0.0 && crossed > 0.0,
                "clearance {clearance}: {parallel} and {crossed}"
            );
            let gap = (parallel - crossed) / parallel;
            assert!(
                gap > 0.0,
                "clearance {clearance}: the crossed figure must sit just below the \
                 parallel one, since the parallel law's second-order term only adds. \
                 Got {crossed} against {parallel}"
            );
            assert!(
                gap < last,
                "clearance {clearance}: the shortfall is {gap} and did not fall \
                 from {last} — the two laws are not meeting"
            );
            last = gap;
        }
        // ...and the agreement is real rather than merely improving: at 5 µm of
        // clearance the two laws sit 0.052 % apart, which is the parallel law's
        // second-order term and nothing else. The bound is loose because the
        // *trend* above is the gate; this only catches a model that converges
        // on the wrong number.
        assert!(
            last < 1e-3,
            "the closest the two laws came was {last} apart, relative"
        );

        // **And the axial float is one term on both sides.** A helical gear's
        // slide along its axis opens the flanks by `j_axial sin β_b` whether
        // its shafts cross or not; with the centre-distance term taken away
        // the two laws must agree on it exactly, not merely to first order —
        // the projection has no second-order term to differ by.
        let float = |sigma: f64| PairStage {
            axial_clearance: 0.05,
            tolerance_plus: 0.0,
            tolerance_minus: 0.0,
            ..stage(sigma, 0.0)
        };
        let parallel = solve_pair_stage(&float(0.0), PairKind::Spur, StageTorques::just(2.0), &lib)
            .expect("a parallel pair")
            .mesh
            .backlash_by_drive();
        let crossed = solve_crossed(&float(0.001), StageTorques::just(2.0), &lib)
            .expect("a crossed pair")
            .mesh
            .backlash_by_drive();
        for (name, p, c) in [
            ("forward", parallel.forward.nominal, crossed.forward.nominal),
            (
                "backward",
                parallel.backward.nominal,
                crossed.backward.nominal,
            ),
        ] {
            assert!(
                p > 0.0,
                "{name}: an axial float on a helical pair opens the flanks"
            );
            // The crossed pair sits a thousandth of a degree off parallel, so
            // its first member's helix — and `sin β_b` with it — is off by
            // that much: the 2e-5 the two differ by is the shaft angle, not
            // a term.
            assert!(
                ((p - c) / p).abs() < 1e-4,
                "{name}: the float reads {p} parallel and {c} crossed"
            );
        }
        // ...and on a spur gear the term is nothing, since `β_b = 0`.
        let spur = PairStage {
            sizing: Auto::fixed(FirstMemberSizing::AdditionalHelix(0.0)),
            ..float(0.0)
        };
        let play = solve_pair_stage(&spur, PairKind::Spur, StageTorques::just(2.0), &lib)
            .expect("a spur pair")
            .mesh
            .backlash_by_drive()
            .forward
            .nominal;
        // Zero to rounding: the centre-distance term at zero clearance is a
        // difference of two involute functions and leaves 1e-14 behind.
        assert!(
            play.abs() < 1e-12,
            "a spur gear's axial float opens nothing: {play}"
        );
    }

    /// **The optimiser reaches a crossed pair, by the crossed mesh's own
    /// objective — and it is the same search.**
    ///
    /// Three claims, each on the friction balance along the line of action:
    ///
    /// - what it chooses is **admissible under the crossed model** — no flank
    ///   fouled, the contact ratio along the line at or above the floor — and
    ///   loses no more than the undercut floor does; on the 17/43 pair at 5°
    ///   it gains, and on the shipped worm it *agrees* with the floor, since a
    ///   worm's loss is its lead angle's and a wheel shift only lengthens the
    ///   path it slides along, which is `Searched::Chose` at the same shifts
    ///   rather than `FoundNothing`;
    /// - it is **converged, not budgeted**: fourteen times the work moves the
    ///   crossed efficiency by less than the parallel search's own ceiling;
    /// - at the parallel limit it lands **near** the parallel search — within
    ///   0.15 of shift and half a point of efficiency — and not on it, for the
    ///   reason `the_two_contacts_report_one_patch_at_the_limit` names: the
    ///   contact-ratio floor is a normal-line count on a point contact and a
    ///   transverse one on a line, `cos² β_b` apart, so the crossed search may
    ///   shorten the path further before the same floor stops it.
    #[test]
    fn the_optimiser_reaches_a_crossed_pair_by_its_own_mesh() {
        use super::super::{Optimisation, Searched};
        use crate::auto::Search;
        let lib = library();
        let optimised = |mut st: PairStage| {
            st.optimisation = Optimisation {
                enabled: true,
                ..Optimisation::default()
            };
            st
        };
        let shifts = |r: &PairResult| [r.gears[0].profile_shift, r.gears[1].profile_shift];

        // The shipped worm: the floor is the answer, and the search says so
        // by choosing rather than by finding nothing.
        let worm =
            solve_worm(&optimised(PairStage::worm()), StageTorques::just(2.0), &lib).unwrap();
        assert_eq!(
            shifts(&worm),
            [0.0, 0.0],
            "a wheel shift buys a worm nothing"
        );
        assert_eq!(
            optimised(PairStage::worm()).chosen_at(&Search::SHIPPED).how,
            Searched::Chose,
            "the search ran and agreed with the floor"
        );

        // A crossed gear pair, where there is something to choose.
        let crossed = |sigma: f64| PairStage {
            shaft_angle: sigma,
            sizing: Auto::fixed(FirstMemberSizing::AdditionalHelix(20.0)),
            gears: [17u32, 43].map(|z| StageGear {
                teeth: z,
                face_width: Auto::fixed(30.0),
                ..StageGear::default()
            }),
            ..PairStage::default()
        };
        let floor = solve_crossed(&crossed(5.0), StageTorques::just(2.0), &lib).unwrap();
        let best = solve_crossed(&optimised(crossed(5.0)), StageTorques::just(2.0), &lib).unwrap();
        assert_eq!(best.mesh.flank_interference, [false, false]);
        assert!(
            best.mesh.contact_ratio >= Optimisation::default().min_contact_ratio - 1e-9,
            "the floor is the crossed count: {}",
            best.mesh.contact_ratio
        );
        assert!(
            best.mesh.efficiency.forward > floor.mesh.efficiency.forward + 1e-3,
            "at 5° the shifts are worth something: {} against the floor's {}",
            best.mesh.efficiency.forward,
            floor.mesh.efficiency.forward
        );

        // Converged: the same ceiling the parallel search is held to.
        let at = |x: [f64; 2]| {
            let mut fixed = crossed(5.0);
            for (gear, shift) in fixed.gears.iter_mut().zip(x) {
                gear.profile_shift = Auto::fixed(shift);
            }
            solve_crossed(&fixed, StageTorques::just(2.0), &lib)
                .unwrap()
                .mesh
                .efficiency
                .forward
        };
        let stage = optimised(crossed(5.0));
        let shipped = at(stage.chosen_at(&Search::SHIPPED).shifts);
        let refined = at(stage.chosen_at(&Search::refined(3)).shifts);
        assert!(
            (refined - shipped).abs() < 1e-5,
            "fourteen times the work moves the crossed efficiency by {}",
            refined - shipped
        );

        // The parallel limit: near, and not on, for the reason above.
        let parallel = solve_pair_stage(
            &optimised(crossed(0.0)),
            PairKind::Spur,
            StageTorques::just(2.0),
            &lib,
        )
        .unwrap();
        let near = solve_crossed(&optimised(crossed(0.01)), StageTorques::just(2.0), &lib).unwrap();
        for i in 0..2 {
            assert!(
                (shifts(&near)[i] - shifts(&parallel)[i]).abs() < 0.15,
                "member {i}: the crossed search chose {} where the parallel chose {}",
                shifts(&near)[i],
                shifts(&parallel)[i]
            );
        }
        assert!(
            (near.mesh.efficiency.forward - parallel.mesh.efficiency.forward).abs() < 5e-3,
            "{} against {}",
            near.mesh.efficiency.forward,
            parallel.mesh.efficiency.forward
        );
    }

    /// **A crossed pair reports its interference, from its own teeth.** The
    /// worm preset clears; a wheel with a tall enough addendum reaches below
    /// where the worm's flank ends, and the result says so on the worm.
    #[test]
    fn a_crossed_pair_reports_the_member_whose_flank_is_fouled() {
        let clear = solved(&PairStage::worm());
        assert_eq!(
            point(&clear).flank_interference,
            [false, false],
            "the shipped worm clears"
        );
        let tall = solved(&member(PairStage::worm(), 1, |g| {
            g.addendum = 3.0;
            g.no_sharp_tip = false;
        }));
        assert!(
            point(&tall).flank_interference[0],
            "a wheel three modules tall reaches past the worm's flank: {:?}",
            point(&tall).flank_interference
        );
        assert!(
            !point(&tall).flank_interference[1],
            "and nothing about the worm's tip changed"
        );
    }

    /// The other half of the same story: the shortfall above is **second order**,
    /// so it must fall by half when the clearance does. A first-order error —
    /// the factor of two that was there — would not move at all.
    #[test]
    fn the_shortfall_at_the_limit_is_second_order_in_the_error() {
        use crate::params::Auto;
        use crate::train::pair::solve_pair_stage;
        use crate::train::{PairStage, StageGear};

        let lib = super::super::test_library();
        let stage = |sigma: f64, clearance: f64| PairStage {
            shaft_angle: sigma,
            sizing: Auto::fixed(FirstMemberSizing::AdditionalHelix(20.0)),
            clearance: Auto::fixed(clearance),
            gears: [
                StageGear {
                    teeth: 17,
                    face_width: Auto::fixed(8.0),
                    ..StageGear::default()
                },
                StageGear {
                    teeth: 43,
                    face_width: Auto::fixed(8.0),
                    ..StageGear::default()
                },
            ],
            ..PairStage::default()
        };
        let shortfall = |clearance: f64| {
            let parallel = solve_pair_stage(
                &stage(0.0, clearance),
                PairKind::Spur,
                StageTorques::just(2.0),
                &lib,
            )
            .expect("a parallel pair")
            .mesh
            .backlash_by_drive()
            .forward
            .nominal;
            let crossed = solve_crossed(&stage(0.001, clearance), StageTorques::just(2.0), &lib)
                .expect("a crossed pair")
                .mesh
                .backlash_by_drive()
                .forward
                .nominal;
            (parallel - crossed) / parallel
        };
        let (coarse, fine) = (shortfall(0.04), shortfall(0.02));
        let halving = coarse / fine;
        assert!(
            (halving - 2.0).abs() < 0.05,
            "halving the clearance moved the shortfall by {halving}×, not 2× — \
             the residual is not the second-order term it is documented as"
        );
    }

    /// **Breaking away is a different question from running, and the static
    /// coefficient answers it.**
    ///
    /// The whole content of the two-coefficient model. A drive that cannot break
    /// away delivers nothing; one that can then runs on its *sliding*
    /// coefficient, and the static figure is discarded — it never was the
    /// efficiency of anything that moved.
    ///
    /// Asserted as the three cases the rule has, and against the geometry's own
    /// threshold rather than against remembered numbers, so it keeps meaning
    /// something if the default worm changes.
    #[test]
    fn a_drive_that_cannot_break_away_delivers_nothing() {
        let stage = |sliding: f64, statik: f64| PairStage {
            sliding_friction: sliding,
            static_friction: statik,
            ..PairStage::worm()
        };
        let threshold = point(&solved(&stage(0.06, 0.06))).locking_friction.backward;
        assert!(threshold > 0.0 && threshold < 0.3, "{threshold}");

        // 1. Both coefficients below it: back-drives, and the figure is the
        //    sliding one — the static coefficient changes nothing but the sign
        //    it was consulted for.
        let free = solved(&stage(0.06, threshold * 0.5));
        assert!(free.mesh.efficiency.backward > 0.0);
        let alone = solved(&stage(0.06, 0.06));
        assert!(
            (free.mesh.efficiency.backward - alone.mesh.efficiency.backward).abs() < 1e-12,
            "the static coefficient must not leak into the number: {} against {}",
            free.mesh.efficiency.backward,
            alone.mesh.efficiency.backward
        );

        // 2. Static above it, sliding below: it never starts, so **zero** — not
        //    the sliding figure, which describes a motion that does not happen.
        let stuck = solved(&stage(0.06, threshold * 1.2));
        assert_eq!(stuck.mesh.efficiency.backward, 0.0);
        assert!(
            stuck.mesh.efficiency.forward > 0.0,
            "forward is unaffected: it is not the direction near the threshold"
        );
        assert!(
            (stuck.mesh.efficiency.forward - alone.mesh.efficiency.forward).abs() < 1e-12,
            "and forward runs on the sliding coefficient like anything else"
        );

        // 3. Both above: still zero, and by the same route.
        assert_eq!(
            solved(&stage(threshold * 1.2, threshold * 1.2))
                .mesh
                .efficiency
                .backward,
            0.0
        );

        // The note names the coefficient that decided it, so a reader knows
        // which input to go and change.
        let notes = &stuck.notes;
        let locked = notes
            .iter()
            .find(|n| n.is(key::STAGE_SELF_LOCKING))
            .expect("self-locking must be said out loud");
        assert_eq!(
            locked.values["friction"],
            format!("{:.3}", threshold * 1.2),
            "the note quotes the static coefficient, not the sliding one"
        );
    }

    /// **A parallel-axis stage obeys the same rule and is never touched by it.**
    ///
    /// The reason it is applied everywhere rather than to worms alone: the rule
    /// is general, and the geometry decides whether it bites. A spur mesh sits a
    /// couple of per cent from unity in both directions, so no plausible static
    /// coefficient reaches it — asserted over a range wide enough to include
    /// dry steel on steel.
    #[test]
    fn a_parallel_stage_passes_the_breakaway_rule_untouched() {
        use crate::train::pair::solve_pair_stage;
        use crate::train::PairStage;

        let lib = super::super::test_library();
        let reference = solve_pair_stage(
            &PairStage::default(),
            PairKind::Spur,
            StageTorques::just(2.0),
            &lib,
        )
        .expect("a stage");
        for statik in [0.0_f64, 0.06, 0.16, 0.5, 0.9] {
            let r = solve_pair_stage(
                &PairStage {
                    static_friction: statik,
                    ..PairStage::default()
                },
                PairKind::Spur,
                StageTorques::just(2.0),
                &lib,
            )
            .expect("a stage");
            assert_eq!(
                r.mesh.efficiency.forward.to_bits(),
                reference.mesh.efficiency.forward.to_bits(),
                "static μ {statik} moved a parallel stage"
            );
            assert_eq!(
                r.mesh.efficiency.backward.to_bits(),
                reference.mesh.efficiency.backward.to_bits()
            );
        }
    }

    /// **A worm stage is rated where it runs, too.**
    ///
    /// The crossed path above reached the operating centre distance through
    /// one call site and a plain worm stage through another, each with its own
    /// chance to be left on the nominal; they are one solve now, and this stays
    /// as the gate on the worm preset's own path.
    ///
    /// Asserted as a direction, not a figure: opening the centre distance
    /// separates the base cylinders along the line of action, which can only
    /// **shorten** the zone, so more clearance is less contact.
    ///
    /// The efficiency is asserted only to *move*, because its direction is not a
    /// law. A shorter zone is less sliding to pay for, but which end it loses
    /// decides whether that helps: a near-parallel pair loses the ends, where
    /// profile sliding is worst, and gets better (0.98786 → 0.98801 at
    /// `Σ = 0.5°`); a worm loses path where the balance was doing comparatively
    /// well and gets worse (68.430 → 67.525 % from zero to 0.3 mm). Asserting a
    /// direction here would be asserting one of those two cases.
    #[test]
    fn a_worm_stage_is_rated_at_the_centre_distance_it_runs_at() {
        let stage = |clearance: f64| PairStage {
            clearance: Auto::fixed(clearance),
            ..PairStage::worm()
        };
        let mut previous: Option<(f64, f64)> = None;
        for clearance in [0.0_f64, 0.02, 0.1, 0.3] {
            let r = solved(&stage(clearance));
            let eps = point(&r).contact_ratio;
            let eta = r.mesh.efficiency.forward;
            if let Some((was_eps, was_eta)) = previous {
                assert!(
                    eps < was_eps,
                    "clearance {clearance}: opening the centres can only shorten \
                     the zone — ε {eps} against {was_eps}"
                );
                assert!(
                    (eta - was_eta).abs() > 1e-9,
                    "clearance {clearance}: the rating did not move, so the centre \
                     distance is not reaching it"
                );
            }
            previous = Some((eps, eta));
        }
    }

    /// **A centre-distance error slides a crossed pair's contact along the
    /// shafts, and near the parallel limit it slides clean off the face.**
    ///
    /// The effect the model could not see while every path was built at the
    /// zero-backlash centre distance. A crossed pair's line of action cannot
    /// turn when the centres move (docs/reference.md#centre-distance-and-backlash), so it *translates* instead — and the
    /// translation grows roughly as `1/sin Σ`, without bound. A 20 µm clearance
    /// is nothing at 90° and several millimetres at half a degree.
    ///
    /// Asserted as the trend and its consequence rather than as millimetres:
    /// with the clearance the near-parallel pair is **face-limited and below
    /// ε = 1**, and taking the clearance away restores it. That is a designer's
    /// question — will these teeth touch as built — answered where it used to
    /// silently report the contact ratio of a pair nobody assembled.
    #[test]
    fn a_centre_distance_error_slides_a_crossed_pairs_contact_off_its_face() {
        use crate::params::Auto;
        use crate::train::{PairStage, StageGear};

        let lib = super::super::test_library();
        let stage = |sigma: f64, clearance: f64| PairStage {
            shaft_angle: sigma,
            sizing: Auto::fixed(FirstMemberSizing::AdditionalHelix(20.0)),
            clearance: Auto::fixed(clearance),
            gears: [
                StageGear {
                    teeth: 17,
                    face_width: Auto::fixed(12.0),
                    ..StageGear::default()
                },
                StageGear {
                    teeth: 43,
                    face_width: Auto::fixed(12.0),
                    ..StageGear::default()
                },
            ],
            ..PairStage::default()
        };
        let mesh = |sigma: f64, clearance: f64| {
            solve_crossed(&stage(sigma, clearance), StageTorques::just(2.0), &lib)
                .expect("a crossed pair")
                .mesh
        };

        // At a right angle the same clearance costs almost nothing...
        let square = mesh(90.0, 0.02);
        assert_eq!(square.point.unwrap().limited_by, ZoneLimit::Tips);
        assert!(square.contact_ratio > 1.5);

        // ...and near the parallel limit it takes the mesh apart.
        let near = mesh(0.5, 0.02);
        assert_eq!(
            near.point.unwrap().limited_by,
            ZoneLimit::Face,
            "the contact should have slid past the face"
        );
        assert!(
            near.contact_ratio < 1.0,
            "a pair whose contact has left the face cannot be continuous: {}",
            near.contact_ratio
        );

        // The clearance is the whole of it: take it away and the same teeth on
        // the same face are tip-limited again, with contact to spare.
        let centred = mesh(0.5, 0.0);
        assert_eq!(centred.point.unwrap().limited_by, ZoneLimit::Tips);
        assert!(
            centred.contact_ratio > 1.5 * near.contact_ratio,
            "{} against {}",
            centred.contact_ratio,
            near.contact_ratio
        );

        // And it is monotone in the shaft angle, which is the `1/sin Σ` showing
        // through without a number being written down.
        let mut previous = 0.0;
        for sigma in [0.5f64, 1.0, 5.0, 30.0, 90.0] {
            let eps = mesh(sigma, 0.02).contact_ratio;
            assert!(
                eps > previous,
                "Σ={sigma}°: straightening the shafts must cost more contact, \
                 not less — {eps} against {previous}"
            );
            previous = eps;
        }
    }

    /// A crossed gear pair solves end to end, and reports what a worm stage
    /// reports — the result shape is shared because the mathematics is.
    #[test]
    fn a_crossed_gear_pair_solves_end_to_end() {
        let stage = PairStage {
            gears: teeth(17, 23),
            shaft_angle: 90.0,
            sizing: Auto::fixed(FirstMemberSizing::HelixAngle(45.0)),
            ..PairStage::worm()
        };
        let r = solve_worm(
            &stage,
            StageTorques::just(2.0),
            &super::super::test_library(),
        )
        .unwrap();
        assert!((r.ratio - 23.0 / 17.0).abs() < 1e-12);
        assert!(r.mesh.efficiency.forward > 0.0 && r.mesh.efficiency.forward < 1.0);
        assert!(point(&r).contact.peak.max_pressure > 0.0);
        assert!(!r.mesh.efficiency.locked().backward);

        // **Where a crossed pair sits, stated as comparisons rather than a
        // threshold.** It slides hard at the pitch point — `1/cos γ₁`, which is
        // 1.41 × the pitch line speed at 45° — so it is far worse than a
        // parallel-axis mesh and far better than a worm of the same shaft angle.
        // Both bounds are computed here rather than remembered.
        let worm = solve_worm(
            &PairStage {
                gears: teeth(1, 40),
                shaft_angle: 90.0,
                sizing: Auto::fixed(FirstMemberSizing::PitchDiameter(7.0)),
                ..PairStage::worm()
            },
            StageTorques::just(2.0),
            &super::super::test_library(),
        )
        .unwrap();
        assert!(
            r.mesh.efficiency.forward > worm.mesh.efficiency.forward,
            "a crossed gear pair should beat a worm: {} vs {}",
            r.mesh.efficiency.forward,
            worm.mesh.efficiency.forward
        );
        assert!(
            r.mesh.efficiency.forward < 0.95,
            "...but it slides too much to approach a parallel-axis mesh: {}",
            r.mesh.efficiency.forward
        );

        // ...and more shaft angle means more sliding means less efficiency.
        let mut previous = f64::INFINITY;
        for sigma in [20.0f64, 40.0, 60.0, 90.0] {
            let e = solve_worm(
                &PairStage {
                    gears: teeth(17, 23),
                    shaft_angle: sigma,
                    sizing: Auto::fixed(FirstMemberSizing::HelixAngle(sigma / 2.0)),
                    ..PairStage::worm()
                },
                StageTorques::just(2.0),
                &super::super::test_library(),
            )
            .unwrap()
            .mesh
            .efficiency
            .forward;
            assert!(
                e < previous,
                "Sigma={sigma}: {e} should be below {previous}"
            );
            previous = e;
        }
    }
}
