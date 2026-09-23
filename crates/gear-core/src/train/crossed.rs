//! The crossed-axis mesh: point contact, along a line of action that cannot
//! turn.
//!
//! **Not a stage of its own, and not a solve of its own.** A worm stage and a
//! crossed gear pair are one shape whose bodies are not
//! parallel, and a distance at an angle in any [`super::shape::Shape`] is
//! this mesh; the shape builds it as a point contact beside its line
//! contacts (`shape::BuiltContact`) and fills the same [`super::MeshReport`]
//! a parallel mesh gets — two efficiencies where the other has one number
//! twice, a sliding at the pitch point where the other has zero, a patch
//! that is an ellipse where the other's is a line, and the zone as the faces
//! leave it in [`super::PointContact`] — and what it lacks is a bending
//! rating, for the reason docs/reference.md#crossed-axes gives. Both members
//! are ordinary [`super::GearResult`]s: a worm is a helical gear with a few
//! starts at a steep helix, and everything a gear can be asked it can be
//! asked.
//!
//! The mesh mathematics is all in [`crate::screw`]. What this module keeps is
//! the worm's conventional [`proportions`], the derivation below of how play
//! reaches a crossed mesh, and the tests that hold the shape to the model on
//! every crossed pair.
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
//! normal's component along the line of centres is `sin α_n` at **every** body
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::arrangements as arr;
    use super::super::shape::{Shape, ShapeResult};
    use super::super::{MeshReport, StageGear, TrainError};
    use super::*;
    use crate::contact::{Directional, Drive};
    use crate::material::{contact_modulus, Material, MaterialLibrary};
    use crate::mesh::MeshSide;
    use crate::note::key;
    use crate::params::Auto;
    use crate::screw::{Screw, ZoneLimit};

    fn library() -> MaterialLibrary {
        super::super::test_library()
    }

    /// A stage solved alone, under its own conventions.
    fn solve_pair_stage(
        stage: &Shape,
        torque: f64,
        speed: f64,
        lib: &MaterialLibrary,
    ) -> Result<crate::train::Alone, TrainError> {
        crate::train::solve_alone(&crate::train::Train::alone(stage, torque, speed), lib)
    }

    /// The old worm entry point, as the tests were written against it: the
    /// pair as a worm drive, whatever its own distance says.
    fn solve_worm(
        stage: &Shape,
        torque: f64,
        speed: f64,
        lib: &MaterialLibrary,
    ) -> Result<crate::train::Alone, TrainError> {
        let mut s = stage.clone();
        s.distances[0].worm = true;
        solve_pair_stage(&s, torque, speed, lib)
    }

    /// The old crossed-gear entry point, likewise: the pair as two gears.
    fn solve_crossed(
        stage: &Shape,
        torque: f64,
        speed: f64,
        lib: &MaterialLibrary,
    ) -> Result<crate::train::Alone, TrainError> {
        let mut s = stage.clone();
        s.distances[0].worm = false;
        solve_pair_stage(&s, torque, speed, lib)
    }

    /// A stage with one member's inputs edited.
    fn member(mut stage: Shape, i: usize, edit: impl FnOnce(&mut StageGear)) -> Shape {
        edit(&mut stage.members[i].gear);
        stage
    }

    /// The mesh a solved pair reports, checked to be the point contact a
    /// crossed pair has.
    fn point(r: &ShapeResult) -> &MeshReport {
        assert!(
            r.meshes[0].point.is_some() && r.meshes[0].line.is_none(),
            "a crossed pair reports a point contact"
        );
        &r.meshes[0]
    }

    fn solved(stage: &Shape) -> crate::train::Alone {
        solve_pair_stage(stage, 2.0, 0.0, &library()).unwrap()
    }

    /// **The same teeth with their bodies brought parallel**, as an efficiency
    /// — the best the pair can be, which a crossed figure is measured against
    /// here and nowhere else. The stage can describe it exactly, since a
    /// crossed pair *is* this stage at `Σ = 0`: the same helix on the first
    /// member, read as the split, and no optimiser on the counterpart. It was
    /// a field of every point contact once, solved for every crossed stage a
    /// designer built and read by nothing but this comparison.
    fn parallel_counterpart(stage: &Shape) -> f64 {
        let flat = {
            let mut s = stage.clone();
            s.distances[0].angle = 0.0;
            s.set_search(false);
            s.with_additional_helix(stage.helix_angles()[0])
        };
        solve_pair_stage(&flat, 2.0, 0.0, &library())
            .expect("the parallel counterpart is buildable")
            .meshes[0]
            .efficiency
            .forward
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
        let free = arr::worm(1, 40);
        let a0 = free
            .screw(0)
            .expect("the shipped worm exists")
            .centre_distance;
        let turning = Screw::least_distance_lead_angle(
            free.members[0].gear.teeth,
            free.members[1].gear.teeth,
            free.distances[0].angle.to_radians(),
        )
        .expect("a right-angle pair has one");
        let d_turning =
            f64::from(free.members[0].gear.teeth) * free.members[0].normal_module() / turning.sin();
        assert!(
            free.first_pitch_diameter() > d_turning,
            "the shipped worm should sit on the fat branch: {} against {d_turning}",
            free.first_pitch_diameter()
        );

        // **The wheel's shift absorbs a distance before the size does.** With
        // the wheel free — the preset — a given distance moves the wheel's
        // shift by the rack law and leaves the worm the size it was stated at.
        {
            let mut stage = free.clone();
            stage.distances[0].distance =
                Auto::fixed(a0 + 0.5 + stage.distances[0].clearance.manual);
            let x = stage.shifts();
            assert!(
                (x[1] - 0.5 / stage.members[0].normal_module()).abs() < 1e-9 && x[0] == 0.0,
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
            let mut stage = free.clone().size_free();
            stage.distances[0].distance = Auto::fixed(target + stage.distances[0].clearance.manual);
            let Ok(s) = stage.screw(0) else { continue };
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
        let mut thin = free.clone().with_first_diameter(d_turning * 0.6);
        let from_thin = thin.screw(0).expect("a thin worm exists").centre_distance;
        thin = thin.size_free();
        thin.distances[0].distance =
            Auto::fixed(from_thin + 0.4 + thin.distances[0].clearance.manual);
        if let Ok(s) = thin.screw(0) {
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
    /// the bodies come parallel its patch lengthens without bound and the
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
        let crossed = |sigma_deg: f64| {
            let mut s = arr::worm(17, 23);
            s.distances[0].angle = sigma_deg;
            s.with_first_helix(sigma_deg / 2.0)
        };
        let worm = arr::worm(1, 40).with_first_diameter(7.0);
        for (name, stage, line_should_govern) in [
            ("crossed 0.5°", crossed(0.5), true),
            ("crossed 1°", crossed(1.0), true),
            ("the worm canary", worm, false),
        ] {
            let s = stage.screw(0).unwrap();
            let r = solve_worm(&stage, 2.0, 0.0, &lib).unwrap();

            // The same questions the stage asked, from the widths it actually
            // resolved: the load on the wheel, the line the teeth provide, and
            // the two models evaluated separately at the pitch point.
            let materials: Vec<Material> = [&stage.members[0].gear, &stage.members[1].gear]
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
            let force = s.normal_force(
                2.0,
                MeshSide::First,
                stage.meshes[0].sliding_friction,
                Drive::Forward,
            );
            let (along, across) = s.contact_curvatures().unwrap();
            let betas = [
                s.worm_helix_angle_rad,
                s.shaft_angle_rad - s.worm_helix_angle_rad,
            ];
            let line_length = (0..2)
                .map(|i| {
                    r.members[i].face_width
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
                (point(&r).cases[0].contact.at_pitch_point - want).abs() < 1e-9 * want,
                "{name}: rated at {} where the governing model gives {want}",
                point(&r).cases[0].contact.at_pitch_point
            );
            // A patch cannot be longer than the teeth it sits on.
            assert!(
                point(&r).cases[0].contact.patch_length <= line_length * (1.0 + 1e-12),
                "{name}: a {} mm patch on a {line_length} mm line",
                point(&r).cases[0].contact.patch_length
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
            let mut s = arr::worm(starts, wheel);
            s.distances[0].axial_clearance = 0.04;
            s.distances[0].clearance = Auto::fixed(0.0);
            s.distances[0].tolerance_plus = 0.0;
            s.distances[0].tolerance_minus = 0.0;
            let stage = s.with_first_diameter(d1);
            let r = solved(&stage);
            let s = stage.screw(0).unwrap();

            let wheel = 0.04 / (s.wheel_pitch_diameter / 2.0);
            assert!(
                (r.meshes[0].backlash_by_drive().forward.nominal.to_radians() - wheel).abs()
                    < 1e-12 * wheel,
                "z₁={starts}: wheel backlash {} vs j/r₂ {}",
                r.meshes[0].backlash_by_drive().forward.nominal.to_radians(),
                wheel
            );

            let worm = std::f64::consts::TAU * 0.04 / s.lead;
            assert!(
                (r.meshes[0]
                    .backlash_by_drive()
                    .backward
                    .nominal
                    .to_radians()
                    - worm)
                    .abs()
                    < 1e-12 * worm,
                "z₁={starts}: worm backlash {} vs 2π j/lead {}",
                r.meshes[0]
                    .backlash_by_drive()
                    .backward
                    .nominal
                    .to_radians(),
                worm
            );
        }
    }

    /// Opening the centre distance opens the flanks too, by its component along
    /// the normal — and closing it below nominal cannot make the backlash
    /// negative, because the flanks would simply touch.
    #[test]
    fn the_centre_distance_tolerance_moves_the_backlash_the_right_way() {
        let stage = {
            let mut s = arr::worm(1, 40);
            s.distances[0].clearance = Auto::fixed(0.0);
            s.distances[0].axial_clearance = 0.0;
            s.distances[0].tolerance_plus = 0.05;
            s.distances[0].tolerance_minus = 0.05;
            s
        };
        let r = solved(&stage);
        assert_eq!(
            r.meshes[0].backlash_by_drive().forward.nominal,
            0.0,
            "nominal, with no slack at all"
        );
        assert!(
            r.meshes[0].backlash_by_drive().forward.maximum > 0.0,
            "opening the centres opens the mesh"
        );
        assert_eq!(
            r.meshes[0].backlash_by_drive().forward.minimum,
            0.0,
            "tighter than nominal is contact"
        );
    }

    /// The stage's headline numbers, and the one it deliberately does not have.
    #[test]
    fn a_worm_stage_reports_contact_and_two_efficiencies_and_no_bending() {
        let r = solved(&arr::worm(1, 40));
        // Negative: a worm is an external mesh, its wheel turns the other way,
        // and a ratio is signed now — it is the graph's rather than `z₂/z₁`.
        assert!((r.ratio.unwrap() + 40.0).abs() < 1e-12);
        assert!(r.meshes[0].efficiency.forward > 0.0 && r.meshes[0].efficiency.forward < 1.0);
        assert!(
            r.meshes[0].efficiency.backward < r.meshes[0].efficiency.forward,
            "back-driving is the worse direction"
        );
        assert!(
            point(&r).cases[0].contact.max_pressure > 0.0
                && point(&r).cases[0].contact.max_pressure.is_finite()
        );
        assert!(
            point(&r).cases[0].contact.curvature_along > 0.0,
            "crossed shafts make a point contact, not a line"
        );
        assert!(
            point(&r).cases[0].contact.patch_length > point(&r).cases[0].contact.patch_width,
            "an ellipse: {} by {}",
            point(&r).cases[0].contact.patch_length,
            point(&r).cases[0].contact.patch_width
        );
        // **A member's torque is the torque its teeth carry** — the driver's
        // read across the mesh, which is what every stress on the member is
        // proportional to — and what its *body* delivers is `η` less, on the
        // body's own row. `|ratio|`: the reduction is signed and a worm's is
        // negative. The wheel's tooth load used to be quoted delivered, and
        // the shape's rule for every member is the tooth load
        // (`docs/corrections.md`).
        let tooth_load = 2.0 * r.ratio.unwrap().abs();
        assert!((r.members[1].cases[0].torque - tooth_load).abs() < 1e-12 * tooth_load);
        let delivered = tooth_load * r.meshes[0].efficiency.forward;
        let at_wheel_shaft = r.cases[0].torques[2].abs();
        assert!(
            (at_wheel_shaft - delivered).abs() < 1e-9 * delivered,
            "the wheel's shaft delivers {at_wheel_shaft}, {delivered} expected"
        );
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
        let narrow = solved(&member(arr::worm(1, 40), 1, |g| {
            g.face_width = Auto::fixed(4.0);
        }));
        let wide = solved(&member(arr::worm(1, 40), 1, |g| {
            g.face_width = Auto::fixed(40.0);
        }));
        assert_eq!(
            point(&narrow).cases[0].contact.max_pressure,
            point(&wide).cases[0].contact.max_pressure,
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
        let auto = solved(&arr::worm(1, 40));
        assert_eq!(
            Some(auto.members[0].face_width),
            auto.members[0].recommended_face_width,
            "an automatic worm length is the recommendation"
        );
        assert_eq!(
            Some(auto.members[1].face_width),
            auto.members[1].recommended_face_width,
            "an automatic wheel face width is the recommendation"
        );

        let manual = solved(&member(arr::worm(1, 40), 0, |g| {
            g.face_width = Auto::fixed(3.5);
        }));
        assert!((manual.members[0].face_width - 3.5).abs() < 1e-12);
        assert_eq!(
            manual.members[0].recommended_face_width, auto.members[0].recommended_face_width,
            "the recommendation is reported whether or not it is in use"
        );
    }

    /// **The pair's kind decides the proportions, not the reading.**
    ///
    /// The proportions describe a worm carrying an enveloping wheel. A crossed
    /// gear pair is two helical gears touching at a point, so quoting them
    /// there would be shipping a convention outside the case it was written
    /// for — and quietly, since the number would look like any other. The same
    /// 17/23 pair at 45° is a worm drive if a designer says it is and a gear
    /// pair otherwise, and only the distance's `worm` bit says which: as a gear pair its
    /// automatic face is the width that keeps contact continuous, with no
    /// recommendation beside it.
    #[test]
    fn a_crossed_gear_pair_is_not_given_a_worms_proportions() {
        let s = arr::worm(17, 23);
        let stage = s.with_first_helix(45.0);
        let lib = library();
        let as_gears = solve_crossed(&stage, 2.0, 0.0, &lib).unwrap();
        let as_worm = solve_worm(&stage, 2.0, 0.0, &lib).unwrap();
        assert!(as_gears.members[0].recommended_face_width.is_none());
        assert!(as_gears.members[1].recommended_face_width.is_none());
        assert!(as_worm.members[0].recommended_face_width.is_some());
        assert!(as_worm.members[1].recommended_face_width.is_some());
        // **Nothing sizes the gear pair's automatic face**: a point contact's
        // pressure does not depend on it, so it stands at its box and says so
        // — while the width at which contact stays continuous is reported
        // beside it as a figure.
        for (i, gear) in as_gears.members.iter().enumerate() {
            assert_eq!(
                gear.face_width, stage.members[i].gear.face_width.manual,
                "member {i}"
            );
            assert!(gear
                .notes
                .iter()
                .any(|n| n.is(key::GEAR_FACE_WIDTH_AS_ENTERED)));
        }
        assert!(point(&as_gears)
            .point
            .unwrap()
            .face_width_for_continuity
            .is_some());
    }

    /// **A crossed stage reports its contact ratio, and the width it says
    /// keeps that at 1 does.**
    ///
    /// The two have to agree: give the face the width the stage reports for
    /// continuity, and the ratio that comes back must be exactly 1. Everything
    /// else here is a comparison — a hand-set face narrower than that gives
    /// less, a wider one gives what the teeth allow and no more, and the number
    /// moves the way the geometry says. The width used to be what an automatic
    /// face resolved to; it is a figure now, since no rating sizes a point
    /// contact's face and a designer types the width they want.
    #[test]
    fn the_width_a_crossed_pair_reports_for_continuity_buys_exactly_continuous_contact() {
        use crate::params::Auto;
        use crate::train::StageGear;

        let lib = super::super::test_library();
        let gear = |teeth: u32, face: Auto<f64>| StageGear {
            teeth,
            face_width: face,
            ..StageGear::default()
        };
        let stage = |face: Auto<f64>| {
            let mut s = arr::pair([17, 43]);
            s.distances[0].angle = 90.0;
            s.members[0].gear = gear(17, face);
            s.members[1].gear = gear(23, face);
            s
        };

        // The width reported for ε = 1, given back: the ratio comes back as 1.
        let wide = solve_crossed(&stage(Auto::fixed(60.0)), 2.0, 0.0, &lib).unwrap();
        let sized = point(&wide)
            .point
            .expect("a crossed pair has a path of contact")
            .face_width_for_continuity
            .expect("a width for continuity");
        let stage_at = |faces: [f64; 2]| {
            let mut s = arr::pair([17, 43]);
            s.distances[0].angle = 90.0;
            s.members[0].gear = gear(17, Auto::fixed(faces[0]));
            s.members[1].gear = gear(23, Auto::fixed(faces[1]));
            s
        };
        let auto = solve_crossed(&stage_at(sized), 2.0, 0.0, &lib).unwrap();
        let m = point(&auto);
        assert!(
            (m.contact_ratio - 1.0).abs() < 1e-9,
            "the reported width should buy exactly continuous contact, got {}",
            m.contact_ratio
        );
        assert_eq!(m.point.expect("a path").limited_by, ZoneLimit::Face);

        // Half that face, about half the contact — and slightly less than half,
        // which is the point. The face is centred on its own gear and the
        // contact is not quite centred on the face, because the pair runs at its
        // nominal centre distance *plus the clearance* and that slides the
        // contact along the bodies (docs/reference.md#centre-distance-and-backlash). Trimming a face symmetrically about
        // an asymmetric contact loses a little more than half.
        let narrow = solve_crossed(&stage(Auto::fixed(sized[0] / 2.0)), 2.0, 0.0, &lib).unwrap();
        let n = point(&narrow);
        assert!(
            n.contact_ratio < 0.5 && n.contact_ratio > 0.45,
            "half the face should leave a little under half the contact: {}",
            n.contact_ratio
        );

        // ...and it is *exactly* half once the clearance is taken away, which
        // pins the shortfall on the slide rather than on the arithmetic.
        let tight = |face: Auto<f64>| {
            let mut s = stage(face);
            s.distances[0].clearance = Auto::fixed(0.0);
            s
        };
        let centred = solve_crossed(&tight(Auto::fixed(60.0)), 2.0, 0.0, &lib).unwrap();
        let width = point(&centred)
            .point
            .expect("a path")
            .face_width_for_continuity
            .expect("a width for continuity");
        let halved = solve_crossed(&tight(Auto::fixed(width[0] / 2.0)), 2.0, 0.0, &lib).unwrap();
        assert!(
            (point(&halved).contact_ratio - 0.5).abs() < 1e-9,
            "with the contact centred, half the face is exactly half the contact"
        );
        assert!(
            narrow.meshes[0]
                .notes
                .iter()
                .any(|s| s.is(key::MESH_CONTACT_RATIO_BELOW_ONE)),
            "a contact ratio below 1 must be said, on the mesh: {:?}",
            narrow.meshes[0].notes
        );

        // Generous, and the teeth are what end it — a wider face buys nothing.
        let wide = solve_crossed(&stage(Auto::fixed(60.0)), 2.0, 0.0, &lib).unwrap();
        let w = point(&wide);
        assert_eq!(w.point.unwrap().limited_by, ZoneLimit::Tips);
        assert!(w.contact_ratio > m.contact_ratio);
        let wider = solve_crossed(&stage(Auto::fixed(120.0)), 2.0, 0.0, &lib).unwrap();
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
        let r = solved(&arr::worm(1, 40));
        let m = point(&r);
        assert!(m.contact_ratio > 0.0);
        // The proportions still size the face; continuity is reported beside
        // them rather than instead of them.
        assert!(r.members[0].recommended_face_width.is_some());
        assert!(m.point.unwrap().face_width_for_continuity.is_some());

        // The tips are the teeth's: a taller worm thread lengthens the zone.
        let taller = solved(&member(arr::worm(1, 40), 0, |g| g.addendum = 1.2));
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
        use crate::train::StageGear;

        let lib = super::super::test_library();
        let crossed = |face: f64| {
            let mut s = arr::pair([17, 43]);
            s.distances[0].angle = 90.0;
            s.members[0].gear = StageGear {
                teeth: 17,
                face_width: Auto::fixed(face),
                ..StageGear::default()
            };
            s.members[1].gear = StageGear {
                teeth: 23,
                face_width: Auto::fixed(face),
                ..StageGear::default()
            };
            s
        };

        // Generous face: the teeth end the zone, the pair shares load, and the
        // rating sits just above the pitch-point figure.
        let wide = solve_crossed(&crossed(20.0), 2.0, 0.0, &lib).unwrap();
        assert!(
            point(&wide).cases[0].contact.max_pressure
                >= point(&wide).cases[0].contact.at_pitch_point,
            "the path cannot be kinder than its gentlest point"
        );
        assert!(point(&wide).contact_ratio > 1.0);

        // Squeeze it: ε falls below 1, load sharing goes, and the same mesh at
        // the same torque rates higher.
        let narrow = solve_crossed(&crossed(0.8), 2.0, 0.0, &lib).unwrap();
        assert!(point(&narrow).contact_ratio < 1.0);
        assert!(
            point(&narrow).cases[0].contact.max_pressure
                > point(&wide).cases[0].contact.max_pressure,
            "losing contact continuity should cost stress, not save it: {} against {}",
            point(&narrow).cases[0].contact.max_pressure,
            point(&wide).cases[0].contact.max_pressure
        );
        // ...and it is the *place* being rated that did it, not the load. The
        // load moved too — a narrower face engages only the middle of the zone,
        // which slides less, which delivers more torque — so the two figures
        // are compared as a ratio, which divides the load out: the same mesh
        // rated at its worst against rated at its pitch point.
        let severity = |r: &ShapeResult| {
            point(r).cases[0].contact.max_pressure / point(r).cases[0].contact.at_pitch_point
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
        use crate::train::StageGear;

        let lib = super::super::test_library();

        for stage in [
            arr::worm(1, 40),
            ({ arr::worm(2, 40) }).with_first_diameter(12.0),
        ] {
            let r = solve_worm(&stage, 2.0, 0.0, &lib).unwrap();
            let classical = point(&r).point.map(|_| ()).map(|()| {
                let s = stage.screw(0).unwrap();
                Directional::of(|d| s.efficiency(stage.meshes[0].sliding_friction, d))
            });
            let classical = classical.expect("a screw geometry");
            for drive in Drive::BOTH {
                assert!(
                    r.meshes[0].efficiency.get(drive) <= classical.get(drive),
                    "{drive:?}: the path average {} should not beat the pitch point {}",
                    r.meshes[0].efficiency.get(drive),
                    classical.get(drive)
                );
            }
            // The threshold is the friction at which the *reported* backward
            // efficiency reaches zero, so it moves down with it.
            assert!(
                point(&r).locking_friction.backward
                    <= stage.screw(0).unwrap().locking_friction().backward
            );
            assert_eq!(
                r.meshes[0].efficiency.locked().backward,
                r.meshes[0].efficiency.backward <= 0.0,
                "self-locking is what the reported figure says, not a second opinion"
            );
        }

        // ...and the same for a crossed gear pair, where the term being added is
        // the whole of a parallel mesh's loss.
        let crossed = {
            let mut s = arr::pair([17, 43]);
            s.distances[0].angle = 60.0;
            s.members[0].gear = StageGear {
                teeth: 17,
                face_width: Auto::fixed(12.0),
                ..StageGear::default()
            };
            s.members[1].gear = StageGear {
                teeth: 43,
                face_width: Auto::fixed(12.0),
                ..StageGear::default()
            };
            s
        };
        let r = solve_crossed(&crossed, 2.0, 0.0, &lib).unwrap();
        point(&r).point.expect("a path");
        assert!(
            r.meshes[0].efficiency.forward < parallel_counterpart(&crossed),
            "a crossed pair must still lose more than the same teeth parallel"
        );
    }

    /// **Crossing bodies adds sliding, and the figures show it** — with one
    /// exception that is the *other* formula's approximation, not this one's.
    ///
    /// A crossed pair loses more than the same teeth running parallel, and by
    /// more the further the bodies are turned. At a shaft angle small enough
    /// that the two are the same mesh, the balance comes out a hundredth of a
    /// point *above* the parallel closed form — because that closed form is
    /// first order in `μ` and the balance is exact in it. `screw.rs` measures
    /// that difference as O(μ²); here it is allowed for and bounded.
    #[test]
    fn a_crossed_pair_loses_more_the_further_its_shafts_are_turned() {
        use crate::params::Auto;
        use crate::train::StageGear;

        let lib = super::super::test_library();
        let stage = |sigma: f64| {
            ({
                let mut s = arr::pair([17, 43]);
                s.distances[0].angle = sigma;
                s.members[0].gear = StageGear {
                    teeth: 17,
                    face_width: Auto::fixed(60.0),
                    ..StageGear::default()
                };
                s.members[1].gear = StageGear {
                    teeth: 43,
                    face_width: Auto::fixed(60.0),
                    ..StageGear::default()
                };
                s
            })
            .with_additional_helix(20.0)
        };

        let mut previous = 1.0;
        for sigma in [0.5f64, 2.0, 10.0, 45.0, 90.0] {
            let r = solve_crossed(&stage(sigma), 2.0, 0.0, &lib).unwrap();
            let mesh = point(&r).point.expect("a path");
            assert_eq!(
                mesh.limited_by,
                ZoneLimit::Tips,
                "Σ={sigma}°: the face is cutting the zone, so this is not a \
                 like-for-like comparison"
            );
            let parallel = parallel_counterpart(&stage(sigma));
            assert!(
                r.meshes[0].efficiency.forward < previous,
                "Σ={sigma}°: turning the shafts further must cost more"
            );
            previous = r.meshes[0].efficiency.forward;

            // Below the parallel figure everywhere the difference is physical,
            // and above it by no more than that formula's own linearisation
            // where the two are the same mesh.
            assert!(
                r.meshes[0].efficiency.forward < parallel + 2e-4,
                "Σ={sigma}°: {} against the parallel {parallel}",
                r.meshes[0].efficiency.forward
            );
        }

        // A worm has one too now — it is a helical gear with one start — and
        // the same ordering holds against it: crossing the bodies to a right
        // angle costs a single-start worm most of what it had.
        let worm = solved(&arr::worm(1, 40));
        let parallel = parallel_counterpart(&arr::worm(1, 40));
        assert!(
            worm.meshes[0].efficiency.forward < parallel,
            "the worm keeps {} against {parallel} with its shafts parallel",
            worm.meshes[0].efficiency.forward
        );
    }

    /// A self-locking pair says so, rather than reporting a negative efficiency
    /// and leaving the reader to notice.
    #[test]
    fn self_locking_is_said_out_loud() {
        let r = solved(
            &({
                let mut s = arr::worm(1, 40);
                s.meshes[0].sliding_friction = 0.06;
                s
            })
            .with_first_diameter(25.0),
        );
        assert!(r.meshes[0].efficiency.locked().backward);
        assert!(
            r.meshes[0]
                .notes
                .iter()
                .any(|n| n.is(key::MESH_SELF_LOCKING) || n.is(key::MESH_NEAR_SELF_LOCKING)),
            "notes: {:?}",
            r.meshes[0].notes
        );
    }

    #[test]
    fn a_pair_that_cannot_exist_says_which_way_it_failed() {
        let err = solve_worm(
            &({ arr::worm(9, 40) }).with_first_diameter(8.0),
            2.0,
            0.0,
            &library(),
        )
        .unwrap_err();
        assert!(format!("{err}").contains("too thin"), "{err}");

        let err = solve_worm(
            &member(arr::worm(1, 40), 1, |g| g.material = "Unobtainium".into()),
            2.0,
            0.0,
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
                    let mut s = arr::worm(z1, z2);
                    s.distances[0].angle = sigma;
                    let by_angle = s.with_first_helix(beta1);
                    let Ok(a) = by_angle.screw(0) else { continue };
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
                    // The pitch geometry to the bit, since it is built from the
                    // one diameter either way; the distance to rounding, since
                    // the worm's shift is held up from undercut at a helix that
                    // came back through an arccosine.
                    let by_diameter = by_angle
                        .clone()
                        .with_first_diameter(by_angle.first_pitch_diameter());
                    let b = by_diameter.screw(0).unwrap();
                    assert_eq!(a.lead_angle_rad, b.lead_angle_rad);
                    assert_eq!(a.wheel_pitch_diameter, b.wheel_pitch_diameter);
                    assert!(
                        (a.centre_distance - b.centre_distance).abs() < 1e-12 * a.centre_distance
                    );
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
            let mut stage = arr::worm(17, 23);
            stage.distances[0].angle = sigma;
            let stage = stage.with_first_helix(sigma);
            let s = stage
                .screw(0)
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
        let mut s = arr::worm(17, 23);
        s.distances[0].angle = 30.0;
        let backwards = s.with_first_helix(0.0);
        assert!(
            backwards.screw(0).is_err(),
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
            let mut s = arr::worm(17, 23);
            s.distances[0].angle = 90.0;
            let stage = s.with_first_helix(beta);
            assert!(stage.screw(0).is_err(), "beta={beta}: a disc is not a gear");
        }
        // Just inside is silly but representable — the project's standing rule is
        // that a limit answers "could this exist", not "would anyone want it".
        let mut s = arr::worm(17, 23);
        s.distances[0].angle = 90.0;
        let stage = s.with_first_helix(89.0);
        let g = stage.screw(0).unwrap();
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
        use crate::train::StageGear;

        let lib = super::super::test_library();
        let gear = |teeth: u32| StageGear {
            teeth,
            face_width: Auto::fixed(8.0),
            ..StageGear::default()
        };
        let mut s = arr::pair([17, 43]);
        s.distances[0].angle = 90.0;
        s.members[0].gear = gear(17);
        s.members[1].gear = gear(23);
        let spur = s.with_additional_helix(0.0);
        // The same pair entered the worm way: by the first member's helix,
        // as a worm kind, with the worm preset's float taken off.
        let mut s = arr::worm(1, 40);
        s.distances[0].angle = 90.0;
        s.distances[0].axial_clearance = 0.0;
        s.members[0].gear = gear(17);
        s.members[1].gear = gear(23);
        let as_screw = s.with_first_helix(45.0);

        let a = solve_crossed(&spur, 2.0, 0.0, &lib).unwrap();
        let b = solve_worm(&as_screw, 2.0, 0.0, &lib).unwrap();
        for (name, x, y) in [
            ("ratio", a.ratio.unwrap(), b.ratio.unwrap()),
            (
                "centre distance",
                a.distances[0].running,
                b.distances[0].running,
            ),
            (
                "lead angle",
                a.members[0].lead_angle,
                b.members[0].lead_angle,
            ),
            (
                "efficiency",
                a.meshes[0].efficiency.forward,
                b.meshes[0].efficiency.forward,
            ),
            (
                "contact",
                point(&a).cases[0].contact.max_pressure,
                point(&b).cases[0].contact.max_pressure,
            ),
            (
                "backlash",
                a.meshes[0].backlash_by_drive().forward.nominal,
                b.meshes[0].backlash_by_drive().forward.nominal,
            ),
        ] {
            assert_eq!(x.to_bits(), y.to_bits(), "{name}: {x} against {y}");
        }

        // The helix angles are what the shaft angle says they are, and they sum
        // to it — the relation the screw model runs on.
        let (b1, b2) = {
            let h = spur.helix_angles();
            (h[0], h[1])
        };
        assert!((b1 - 45.0).abs() < 1e-12 && (b2 - 45.0).abs() < 1e-12);
        assert!((b1 + b2 - spur.distances[0].angle).abs() < 1e-12);
    }

    /// **A parallel stage is the shaft angle's zero, not a separate thing.**
    ///
    /// At `Σ = 0` the additional helix is the whole of each gear's helix and the
    /// two hands are opposed, which is exactly what the stage did before it had
    /// a shaft angle at all. Asserted on the geometry the mesh is built from,
    /// so it holds whatever the solve does with it.
    #[test]
    fn a_parallel_stage_is_the_shaft_angles_zero() {
        for additional in [0.0_f64, 12.5, -30.0] {
            let stage = arr::pair([17, 43]).with_additional_helix(additional);
            let (b1, b2) = {
                let h = stage.helix_angles();
                (h[0], h[1])
            };
            assert!((b1 - additional).abs() < 1e-12);
            assert!((b2 + additional).abs() < 1e-12, "the hands must oppose");
            assert!(stage.distances[0].angle == 0.0);
        }
    }

    /// **The crossed backlash law and the parallel one meet where the bodies
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

        use crate::train::StageGear;

        let lib = super::super::test_library();
        let stage = |sigma: f64, clearance: f64| {
            ({
                let mut s = arr::pair([17, 43]);
                s.distances[0].angle = sigma;
                s.distances[0].clearance = Auto::fixed(clearance);
                s.members[0].gear = StageGear {
                    teeth: 17,
                    face_width: Auto::fixed(8.0),
                    ..StageGear::default()
                };
                s.members[1].gear = StageGear {
                    teeth: 43,
                    face_width: Auto::fixed(8.0),
                    ..StageGear::default()
                };
                s
            })
            .with_additional_helix(20.0)
        };

        let mut last = f64::INFINITY;
        for clearance in [0.08_f64, 0.04, 0.02, 0.01, 0.005] {
            let parallel = solve_pair_stage(&stage(0.0, clearance), 2.0, 0.0, &lib)
                .expect("a parallel pair")
                .meshes[0]
                .backlash_by_drive()
                .forward
                .nominal;
            // As close to parallel as the screw model will go. The pair is still
            // crossed, so it is still the crossed law answering.
            let crossed = solve_crossed(&stage(0.001, clearance), 2.0, 0.0, &lib)
                .expect("a crossed pair")
                .meshes[0]
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
        let float = |sigma: f64| {
            let mut s = stage(sigma, 0.0);
            s.distances[0].axial_clearance = 0.05;
            s.distances[0].tolerance_plus = 0.0;
            s.distances[0].tolerance_minus = 0.0;
            s
        };
        let parallel = solve_pair_stage(&float(0.0), 2.0, 0.0, &lib)
            .expect("a parallel pair")
            .meshes[0]
            .backlash_by_drive();
        let crossed = solve_crossed(&float(0.001), 2.0, 0.0, &lib)
            .expect("a crossed pair")
            .meshes[0]
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
        let spur = float(0.0).with_additional_helix(0.0);
        let play = solve_pair_stage(&spur, 2.0, 0.0, &lib)
            .expect("a spur pair")
            .meshes[0]
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
        use super::super::Searched;
        use crate::auto::Search;
        let lib = library();
        let optimised = |mut st: Shape| {
            st.set_search(true);
            st
        };
        let shifts = |r: &ShapeResult| [r.members[0].profile_shift, r.members[1].profile_shift];

        // The shipped worm: the floor is the answer, and the search says so
        // by choosing rather than by finding nothing.
        let worm = solve_worm(&optimised(arr::worm(1, 40)), 2.0, 0.0, &lib).unwrap();
        assert_eq!(
            shifts(&worm),
            [0.0, 0.0],
            "a wheel shift buys a worm nothing"
        );
        assert_eq!(
            optimised(arr::worm(1, 40)).searched(&Search::SHIPPED),
            Searched::Chose,
            "the search ran and agreed with the floor"
        );

        // A crossed gear pair, where there is something to choose.
        let crossed = |sigma: f64| {
            ({
                let mut s = arr::pair([17, 43]);
                s.distances[0].angle = sigma;
                for (m, g) in s.members.iter_mut().zip([17u32, 43].map(|z| StageGear {
                    teeth: z,
                    face_width: Auto::fixed(30.0),
                    ..StageGear::default()
                })) {
                    m.gear = g;
                }
                s
            })
            .with_additional_helix(20.0)
        };
        let floor = solve_crossed(&crossed(5.0), 2.0, 0.0, &lib).unwrap();
        let best = solve_crossed(&optimised(crossed(5.0)), 2.0, 0.0, &lib).unwrap();
        assert_eq!(best.meshes[0].flank_interference, [false, false]);
        assert!(
            best.meshes[0].contact_ratio >= super::super::DEFAULT_MIN_CONTACT_RATIO - 1e-9,
            "the floor is the crossed count: {}",
            best.meshes[0].contact_ratio
        );
        assert!(
            best.meshes[0].efficiency.forward > floor.meshes[0].efficiency.forward + 1e-3,
            "at 5° the shifts are worth something: {} against the floor's {}",
            best.meshes[0].efficiency.forward,
            floor.meshes[0].efficiency.forward
        );

        // Converged: the same ceiling the parallel search is held to.
        let at = |x: &[f64]| {
            let mut fixed = crossed(5.0);
            for (m, shift) in fixed.members.iter_mut().zip(x) {
                m.gear.profile_shift = Auto::fixed(*shift);
            }
            solve_crossed(&fixed, 2.0, 0.0, &lib).unwrap().meshes[0]
                .efficiency
                .forward
        };
        let stage = optimised(crossed(5.0));
        let shipped = at(&stage.shifts_at(&Search::SHIPPED));
        let refined = at(&stage.shifts_at(&Search::refined(3)));
        assert!(
            (refined - shipped).abs() < 1e-5,
            "fourteen times the work moves the crossed efficiency by {}",
            refined - shipped
        );

        // The parallel limit: near, and not on, for the reason above.
        let parallel = solve_pair_stage(&optimised(crossed(0.0)), 2.0, 0.0, &lib).unwrap();
        let near = solve_crossed(&optimised(crossed(0.01)), 2.0, 0.0, &lib).unwrap();
        for i in 0..2 {
            assert!(
                (shifts(&near)[i] - shifts(&parallel)[i]).abs() < 0.15,
                "member {i}: the crossed search chose {} where the parallel chose {}",
                shifts(&near)[i],
                shifts(&parallel)[i]
            );
        }
        assert!(
            (near.meshes[0].efficiency.forward - parallel.meshes[0].efficiency.forward).abs()
                < 5e-3,
            "{} against {}",
            near.meshes[0].efficiency.forward,
            parallel.meshes[0].efficiency.forward
        );
    }

    /// **A crossed pair reports its interference, from its own teeth.** The
    /// worm preset clears; a wheel with a tall enough addendum reaches below
    /// where the worm's flank ends, and the result says so on the worm.
    #[test]
    fn a_crossed_pair_reports_the_member_whose_flank_is_fouled() {
        let clear = solved(&arr::worm(1, 40));
        assert_eq!(
            point(&clear).flank_interference,
            [false, false],
            "the shipped worm clears"
        );
        let tall = solved(&member(arr::worm(1, 40), 1, |g| {
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

        use crate::train::StageGear;

        let lib = super::super::test_library();
        let stage = |sigma: f64, clearance: f64| {
            ({
                let mut s = arr::pair([17, 43]);
                s.distances[0].angle = sigma;
                s.distances[0].clearance = Auto::fixed(clearance);
                s.members[0].gear = StageGear {
                    teeth: 17,
                    face_width: Auto::fixed(8.0),
                    ..StageGear::default()
                };
                s.members[1].gear = StageGear {
                    teeth: 43,
                    face_width: Auto::fixed(8.0),
                    ..StageGear::default()
                };
                s
            })
            .with_additional_helix(20.0)
        };
        let shortfall = |clearance: f64| {
            let parallel = solve_pair_stage(&stage(0.0, clearance), 2.0, 0.0, &lib)
                .expect("a parallel pair")
                .meshes[0]
                .backlash_by_drive()
                .forward
                .nominal;
            let crossed = solve_crossed(&stage(0.001, clearance), 2.0, 0.0, &lib)
                .expect("a crossed pair")
                .meshes[0]
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
        let stage = |sliding: f64, statik: f64| {
            let mut s = arr::worm(1, 40);
            s.meshes[0].sliding_friction = sliding;
            s.meshes[0].static_friction = statik;
            s
        };
        let threshold = point(&solved(&stage(0.06, 0.06))).locking_friction.backward;
        assert!(threshold > 0.0 && threshold < 0.3, "{threshold}");

        // 1. Both coefficients below it: back-drives, and the figure is the
        //    sliding one — the static coefficient changes nothing but the sign
        //    it was consulted for.
        let free = solved(&stage(0.06, threshold * 0.5));
        assert!(free.meshes[0].efficiency.backward > 0.0);
        let alone = solved(&stage(0.06, 0.06));
        assert!(
            (free.meshes[0].efficiency.backward - alone.meshes[0].efficiency.backward).abs()
                < 1e-12,
            "the static coefficient must not leak into the number: {} against {}",
            free.meshes[0].efficiency.backward,
            alone.meshes[0].efficiency.backward
        );

        // 2. Static above it, sliding below: it never starts, so **zero** — not
        //    the sliding figure, which describes a motion that does not happen.
        let stuck = solved(&stage(0.06, threshold * 1.2));
        assert_eq!(stuck.meshes[0].efficiency.backward, 0.0);
        assert!(
            stuck.meshes[0].efficiency.forward > 0.0,
            "forward is unaffected: it is not the direction near the threshold"
        );
        assert!(
            (stuck.meshes[0].efficiency.forward - alone.meshes[0].efficiency.forward).abs() < 1e-12,
            "and forward runs on the sliding coefficient like anything else"
        );

        // 3. Both above: still zero, and by the same route.
        assert_eq!(
            solved(&stage(threshold * 1.2, threshold * 1.2)).meshes[0]
                .efficiency
                .backward,
            0.0
        );

        // The note names the coefficient that decided it, so a reader knows
        // which input to go and change.
        let notes = &stuck.meshes[0].notes;
        let locked = notes
            .iter()
            .find(|n| n.is(key::MESH_SELF_LOCKING))
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
        let lib = super::super::test_library();
        let reference = solve_pair_stage(&arr::pair([17, 43]), 2.0, 0.0, &lib).expect("a stage");
        for statik in [0.0_f64, 0.06, 0.16, 0.5, 0.9] {
            let r = solve_pair_stage(
                &({
                    let mut s = arr::pair([17, 43]);
                    s.meshes[0].static_friction = statik;
                    s
                }),
                2.0,
                0.0,
                &lib,
            )
            .expect("a stage");
            assert_eq!(
                r.meshes[0].efficiency.forward.to_bits(),
                reference.meshes[0].efficiency.forward.to_bits(),
                "static μ {statik} moved a parallel stage"
            );
            assert_eq!(
                r.meshes[0].efficiency.backward.to_bits(),
                reference.meshes[0].efficiency.backward.to_bits()
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
        let stage = |clearance: f64| {
            let mut s = arr::worm(1, 40);
            s.distances[0].clearance = Auto::fixed(clearance);
            s
        };
        let mut previous: Option<(f64, f64)> = None;
        for clearance in [0.0_f64, 0.02, 0.1, 0.3] {
            let r = solved(&stage(clearance));
            let eps = point(&r).contact_ratio;
            let eta = r.meshes[0].efficiency.forward;
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
    /// bodies, and near the parallel limit it slides clean off the face.**
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
        use crate::train::StageGear;

        let lib = super::super::test_library();
        let stage = |sigma: f64, clearance: f64| {
            ({
                let mut s = arr::pair([17, 43]);
                s.distances[0].angle = sigma;
                s.distances[0].clearance = Auto::fixed(clearance);
                s.members[0].gear = StageGear {
                    teeth: 17,
                    face_width: Auto::fixed(12.0),
                    ..StageGear::default()
                };
                s.members[1].gear = StageGear {
                    teeth: 43,
                    face_width: Auto::fixed(12.0),
                    ..StageGear::default()
                };
                s
            })
            .with_additional_helix(20.0)
        };
        let mesh = |sigma: f64, clearance: f64| {
            solve_crossed(&stage(sigma, clearance), 2.0, 0.0, &lib)
                .expect("a crossed pair")
                .meshes[0]
                .clone()
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
        let stage = arr::worm(17, 23).with_first_helix(45.0);
        let r = solve_worm(&stage, 2.0, 0.0, &super::super::test_library()).unwrap();
        // Negative for the same reason a worm's is: an external mesh reverses.
        assert!((r.ratio.unwrap() + 23.0 / 17.0).abs() < 1e-12);
        assert!(r.meshes[0].efficiency.forward > 0.0 && r.meshes[0].efficiency.forward < 1.0);
        assert!(point(&r).cases[0].contact.max_pressure > 0.0);
        assert!(!r.meshes[0].efficiency.locked().backward);

        // **Where a crossed pair sits, stated as comparisons rather than a
        // threshold.** It slides hard at the pitch point — `1/cos γ₁`, which is
        // 1.41 × the pitch line speed at 45° — so it is far worse than a
        // parallel-axis mesh and far better than a worm of the same shaft angle.
        // Both bounds are computed here rather than remembered.
        let worm = solve_worm(
            &({
                let mut s = arr::worm(1, 40);
                s.distances[0].angle = 90.0;
                s
            })
            .with_first_diameter(7.0),
            2.0,
            0.0,
            &super::super::test_library(),
        )
        .unwrap();
        assert!(
            r.meshes[0].efficiency.forward > worm.meshes[0].efficiency.forward,
            "a crossed gear pair should beat a worm: {} vs {}",
            r.meshes[0].efficiency.forward,
            worm.meshes[0].efficiency.forward
        );
        assert!(
            r.meshes[0].efficiency.forward < 0.95,
            "...but it slides too much to approach a parallel-axis mesh: {}",
            r.meshes[0].efficiency.forward
        );

        // ...and more shaft angle means more sliding means less efficiency.
        let mut previous = f64::INFINITY;
        for sigma in [20.0f64, 40.0, 60.0, 90.0] {
            let e = solve_worm(
                &({
                    let mut s = arr::worm(17, 23);
                    s.distances[0].angle = sigma;
                    s
                })
                .with_first_helix(sigma / 2.0),
                2.0,
                0.0,
                &super::super::test_library(),
            )
            .unwrap()
            .meshes[0]
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
