//! **The planetary set's vocabulary, and the closed form its power flow is
//! checked against.**
//!
//! The set's own solvers are gone: its closure — the planet's shift that
//! brings two centre distances to one — is what the shape does for any
//! distance with more than one mesh on it ([`crate::train::shape`]), and
//! its ring search is `gear-cli planetary` walking the counts through the
//! same closure. What stays is what a set is *called* — [`PlanetaryShaft`],
//! [`Arrangement`], which a preset's boundary and the harness's fixtures
//! name a set by — and [`power`], Pennestrì's closed form for a three-shaft
//! set with a basic ratio and a fixed-carrier efficiency, kept as the
//! independent statement the per-mesh flow is held to on every arrangement
//! ([`crate::train::flow`]'s tests, and the shape's). Nothing in production
//! reads it.
//!
//! # What is not here
//!
//! Radial assembly — whether a planet can be brought in sideways past the
//! ring's teeth. It is a swept-motion question, not a comparison of tip
//! circles, and `docs/reference.md#internal-gears` records what happened to
//! the attempt that treated it as one.

/// The three tooth counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Teeth {
    pub sun: u32,
    pub planet: u32,
    pub ring: u32,
}

// ------------------------------------------------------------- kinematics ---

/// One of the three shafts a planetary set presents.
///
/// The planets themselves are not on this list: they have no shaft of their own,
/// and their speed is a consequence of the other three.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanetaryShaft {
    Sun,
    Carrier,
    Ring,
}

impl PlanetaryShaft {
    /// The three shafts, in the order the arrays below index them.
    ///
    /// Named once because it was written out three times — in `other`, in the
    /// tests' arrangement sweep and in the harness — and a list of three
    /// written four times is a list that can come to be three.
    pub const ALL: [Self; 3] = [Self::Sun, Self::Carrier, Self::Ring];

    /// Index into the `[sun, carrier, ring]` arrays below.
    const fn index(self) -> usize {
        match self {
            Self::Sun => 0,
            Self::Carrier => 1,
            Self::Ring => 2,
        }
    }

    /// Index into the `[sun, carrier, ring]` arrays, for callers outside this
    /// module that hold those arrays.
    #[must_use]
    pub const fn index_pub(self) -> usize {
        self.index()
    }

    /// The member that is neither of these two.
    ///
    /// A planetary set has three shafts and exactly two are chosen — one driven,
    /// one held — so the third is not a choice at all.
    pub(crate) fn other(a: Self, b: Self) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|&m| m != a && m != b)
            .filter(|_| a != b)
    }
}

/// Which shaft drives and which is held.
///
/// **This is an addition to the specification's field list** (docs/rationale.md#additions-to-the-specifications-field-list). The
/// specification names only "Driven By", which picks one shaft of three and
/// leaves the arrangement undetermined: a sun-driven set behaves quite
/// differently with the ring held than with the carrier held. Naming the held
/// shaft as well is what makes the six modes of docs/reference.md#planetary-sets reachable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arrangement {
    pub input: PlanetaryShaft,
    pub fixed: PlanetaryShaft,
}

/// What the three shafts do, and what it costs to make them do it.
#[derive(Clone, Copy, Debug)]
pub struct Power {
    /// Angular speeds `[sun, carrier, ring]`, in whatever unit the input was
    /// given. The held shaft is exactly zero.
    pub speeds: [f64; 3],
    /// Torques `[sun, carrier, ring]`, in whatever unit the input was given.
    /// They sum to zero — the set is in equilibrium, and the held shaft's torque
    /// is the reaction it carries.
    pub torques: [f64; 3],
    /// Speed reduction, input over output. Negative when the output turns the
    /// other way.
    pub ratio: f64,
    /// Mechanical efficiency, `|T_out ω_out| / |T_in ω_in|`.
    pub efficiency: f64,
    /// The member the other two leave over.
    pub output: PlanetaryShaft,
    /// Sign of the rolling power — which way power crosses the meshes in the
    /// carrier's frame. `+1` when the sun leads the carrier under a driving
    /// torque, `−1` when it trails.
    pub rolling_power_sign: f64,
}

impl Power {
    /// **The planet's own rotation**, absolute and relative to the carrier.
    ///
    /// **An oracle, not the model.** Nothing shipped reads this any more: a
    /// planet's speed comes off the shaft-line graph with every other member's
    /// (`train::Wiring::unit_motion`). It is kept, test-only, because it is a
    /// *second derivation* — Willis on the sun mesh in the carrier's frame —
    /// and the graph is held against it in `kinematics::tests`. That is the
    /// plan's sequencing done as it said: the old model becomes the fixture
    /// rather than being deleted with nothing to stand in its place.
    ///
    /// The planet is not one of the three shafts — [`Power`] is about a set with
    /// a basic ratio, and a basic ratio does not say how many teeth the thing
    /// between the two central members has — so its speed is a question for the
    /// tooth counts, asked here where the rest of the kinematics live.
    ///
    /// In the carrier's frame the sun mesh is an ordinary external pair:
    ///
    /// ```text
    /// ω_p − ω_c = −(z_s / z_p) (ω_s − ω_c)
    /// ```
    ///
    /// The ring mesh gives the same number by the other road,
    /// `+(z_r / z_p)(ω_r − ω_c)`, which is Willis again and is what
    /// `the_planets_own_speed_is_what_both_of_its_meshes_say` checks.
    ///
    /// **The relative figure is the one its teeth see**, and it is not the sun's
    /// relative speed: the two differ by `z_s/z_p`, which is a third on an
    /// ordinary set. Nor is its absolute speed the carrier's — the planet spins
    /// on the arm as well as riding it, and on a set with the ring held the two
    /// have opposite signs.
    ///
    /// Returns `(absolute, relative to the carrier)`, in the unit the speeds
    /// were given in.
    #[cfg(test)]
    #[must_use]
    pub fn planet_speed(&self, teeth: Teeth) -> (f64, f64) {
        let carrier = self.speeds[PlanetaryShaft::Carrier.index_pub()];
        let sun = self.speeds[PlanetaryShaft::Sun.index_pub()];
        let relative = -(f64::from(teeth.sun) / f64::from(teeth.planet)) * (sun - carrier);
        (carrier + relative, relative)
    }
}

/// The basic, carrier-fixed ratio `i₀ = −z_ring / z_sun`.
///
/// Negative because with the carrier held the sun and ring turn opposite ways —
/// the planet reverses the sense once and the internal mesh does not reverse it
/// again. Everything below is written in terms of this one number, which is what
/// makes the six modes one piece of algebra rather than six.
#[must_use]
pub fn basic_ratio(teeth: Teeth) -> f64 {
    -(f64::from(teeth.ring) / f64::from(teeth.sun))
}

/// Solve the whole set: speeds, torques, ratio and efficiency, in one go.
///
/// # What it is about
///
/// **Three shafts, a basic ratio and a fixed-carrier efficiency** — and nothing
/// else. Sun, carrier and ring name the three *roles*: the two central members
/// on the common axis, and the arm that carries whatever runs between them.
/// Which tooth form each central member has, and what the planet is, reach this
/// solve only through the basic ratio it is handed. So any three-shaft
/// epicyclic can be put through it by naming its own members in those roles —
/// which is why the ratio arrives as a number rather than as a set of planetary
/// tooth counts to derive one from.
///
/// # The method
///
/// Pennestrì–Freudenstein, as docs/reference.md#planetary-sets sets it out. Two linear relations carry
/// everything:
///
/// ```text
/// ω_s + (i₀ − 1) ω_c − i₀ ω_r = 0             Willis — kinematics
/// T_s : T_c : T_r = 1 : −(1 − k) : −k         k = i₀ η₀^w — equilibrium with loss
/// ```
///
/// The first is Willis's equation rearranged so all three speeds appear
/// symmetrically; the second is torque equilibrium with the mesh loss folded in
/// through `η₀^w`. Both are written once with the member's *index* selecting a
/// coefficient, so no mode is a special case of any other.
///
/// **Efficiency must not be taken mesh by mesh in the fixed frame.** The meshes
/// slide at their speeds relative to the *carrier*, not to ground, which is why
/// `η₀` — the fixed-carrier efficiency — is the quantity that enters. A set whose
/// two meshes are each 99 % efficient can be far worse than 98 % overall, and can
/// self-lock; that is a real property of the arrangement, not an error.
///
/// # The sign of the rolling power
///
/// `w = sgn(T_s (ω_s − ω_c))` decides whether `η₀` multiplies or divides, and it
/// depends on a torque that is itself being solved for. Rather than assume it,
/// both values are tried and the self-consistent one kept — there are only two,
/// and consistency picks between them.
///
/// # Errors
///
/// `None` when the same member is both driven and held, when the named input is
/// not actually driving (`T ω ≤ 0` — see above), when the tooth counts make a
/// relation degenerate, or when neither sign of the rolling power is
/// self-consistent.
#[must_use]
pub fn power(
    basic_ratio: f64,
    arrangement: Arrangement,
    input_speed: f64,
    input_torque: f64,
    fixed_carrier_efficiency: f64,
) -> Option<Power> {
    let output = PlanetaryShaft::other(arrangement.input, arrangement.fixed)?;
    let (i, f, o) = (
        arrangement.input.index(),
        arrangement.fixed.index(),
        output.index(),
    );
    let i0 = basic_ratio;
    if !i0.is_finite() || !fixed_carrier_efficiency.is_finite() {
        return None;
    }
    // **The named input has to be driving.** A shaft with `T ω < 0` is absorbing
    // power, so calling it the input is a contradiction rather than a design —
    // and the arithmetic says so plainly, returning `1/η₀ > 1`. Refused, because
    // the fix is to name the shaft that is actually driving.
    let input_power = input_torque * input_speed;
    if !input_power.is_finite() || input_power <= 0.0 {
        return None;
    }

    // Willis, with the held shaft at zero: one equation, one unknown.
    let willis = [1.0, i0 - 1.0, -i0];
    if willis[o].abs() < f64::MIN_POSITIVE {
        return None;
    }
    let mut speeds = [0.0; 3];
    speeds[i] = input_speed;
    speeds[f] = 0.0;
    speeds[o] = -willis[i] * input_speed / willis[o];

    // ...then equilibrium, for each candidate sign of the rolling power.
    for w in [1.0, -1.0] {
        let k = i0 * fixed_carrier_efficiency.powf(w);
        let shares = [1.0, -(1.0 - k), -k];
        if shares[i].abs() < f64::MIN_POSITIVE {
            continue;
        }
        let sun_torque = input_torque / shares[i];
        let torques = [
            sun_torque * shares[0],
            sun_torque * shares[1],
            sun_torque * shares[2],
        ];
        // The sign this branch assumed has to be the sign it produces.
        let rolling = torques[0] * (speeds[0] - speeds[1]);
        if rolling != 0.0 && rolling.signum() != w {
            continue;
        }
        // **...and the output has to absorb what the input delivers.**
        //
        // Self-consistency in the rolling sign is necessary and not sufficient.
        // `k = i₀ η₀^w` sits either side of 1 as `w` flips, and where `i₀` is
        // itself close to 1 — which is exactly what a set reducing by the square
        // of a tooth count is — the two candidates straddle it. Then `1 − k`
        // changes sign between the branches, so the sun's torque does, so the
        // rolling power does, and **both branches confirm their own assumption**.
        //
        // What separates them is where the power goes. One puts the output's
        // torque along its own rotation, which is a shaft *delivering* power
        // while the input delivers too and friction makes up the difference —
        // energy from nowhere, and it shows up as an efficiency above 1. The
        // other has the output absorbing, the loss positive, and an efficiency
        // below it. On a well conditioned set only one branch was ever
        // self-consistent and this changes nothing; near `i₀ = 1` it is the
        // whole answer.
        if torques[o] * speeds[o] > 0.0 {
            continue;
        }
        let efficiency = (torques[o] * speeds[o]).abs() / input_power;
        return Some(Power {
            speeds,
            torques,
            ratio: input_speed / speeds[o],
            efficiency,
            output,
            rolling_power_sign: w,
        });
    }
    None
}

/// **What a carrier-driven reduction can reach**, given meshes that keep
/// `mesh` of what passes through them:
///
/// ```text
/// η = 1 / [ R(1 − η₀) + η₀ ]
/// ```
///
/// The whole power flow collapses to this for the 3K family with the
/// carrier driving, one central member held and the other the output — a
/// Wolfrom, a hula stage, a planocentric — and it answers the design
/// question before anything is drawn: *what would the teeth have to be
/// worth?* The loss term carries `R`, so a reduction multiplies the mesh
/// loss before it reaches the output: at `R = 324` a mesh pair losing
/// 0.85 % keeps 27 % of the input, and it would have to lose under 0.04 %
/// to keep 90 %. Halve the reduction and the same teeth do far better.
/// This is why a gearbox of this family is built at a few tens to one and
/// not a few hundreds, and why the ones that reach both are a different
/// mechanism. The per-mesh flow is held to it on every count and friction
/// tried (`train::arrangements`' tests).
#[must_use]
pub fn carrier_driven_efficiency(ratio: f64, mesh: f64) -> f64 {
    1.0 / (ratio.abs() * (1.0 - mesh) + mesh)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn teeth() -> Teeth {
        Teeth {
            sun: 24,
            planet: 18,
            ring: 60,
        }
    }

    /// Every arrangement of driven and held shaft, six in all.
    fn arrangements() -> Vec<Arrangement> {
        let all = [
            PlanetaryShaft::Sun,
            PlanetaryShaft::Carrier,
            PlanetaryShaft::Ring,
        ];
        let mut out = Vec::new();
        for &input in &all {
            for &fixed in &all {
                if input != fixed {
                    out.push(Arrangement { input, fixed });
                }
            }
        }
        assert_eq!(out.len(), 6);
        out
    }

    /// **A genuinely driving input always has a flow**, and that is why the
    /// refusal a set can meet is the *back-driven* one.
    ///
    /// [`power`] returns `None` three ways: a degenerate Willis coefficient, an
    /// input that is not driving (`T ω ≤ 0`), and neither sign of the rolling
    /// power confirming itself. A stage asks it twice — forward at unit speed
    /// and unit torque, and backward with the output's own reaction — and only
    /// the second can refuse, which is self-locking and is an *answer*.
    ///
    /// This is the standing evidence for that, and for
    /// `gear_io::strings::UNFIRED`'s exemption of `error.train_no_power_flow`:
    /// **an absence has a date**, and a note claiming one is worth less than a
    /// sweep that fails if it stops being true. Sun against ring across the
    /// whole plausible range, `η₀` from near-lossless down to 0.3 — far below
    /// anything two involute meshes produce — and every arrangement.
    #[test]
    fn a_driving_input_always_has_a_flow() {
        let mut checked = 0u32;
        for zs in (1..=119).step_by(2) {
            for zr in (1..=249).step_by(3) {
                let i0 = -f64::from(zr) / f64::from(zs);
                for eta0 in [0.999, 0.97, 0.9, 0.7, 0.5, 0.3] {
                    for arrangement in arrangements() {
                        assert!(
                            power(i0, arrangement, 1.0, 1.0, eta0).is_some(),
                            "z {zs}/{zr}, eta0 {eta0}, {arrangement:?} refused a driving input"
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 100_000, "only {checked} combinations swept");

        // ...and the two refusals that *are* reachable, so this is a statement
        // about driving inputs rather than about `power` never saying no.
        let ordinary = Arrangement {
            input: PlanetaryShaft::Sun,
            fixed: PlanetaryShaft::Carrier,
        };
        assert!(
            power(basic_ratio(teeth()), ordinary, 1.0, -1.0, 0.97).is_none(),
            "a shaft with T omega < 0 is not an input"
        );
        assert!(
            power(0.0, ordinary, 1.0, 1.0, 0.97).is_none(),
            "a zero basic ratio leaves the ring with no Willis coefficient"
        );
    }

    /// **The three classical ratios, arrived at rather than written down.**
    ///
    /// Each falls out of the one Willis relation with a different shaft held, so
    /// agreeing with the textbook forms says the relation is right — and none of
    /// the three is a special case in the code.
    #[test]
    fn the_classical_ratios_come_out_of_one_relation() {
        let t = teeth();
        let (zs, zr) = (f64::from(t.sun), f64::from(t.ring));

        // Ring held, sun driving: the reduction is 1 + z_r/z_s.
        let p = power(
            basic_ratio(t),
            Arrangement {
                input: PlanetaryShaft::Sun,
                fixed: PlanetaryShaft::Ring,
            },
            1000.0,
            1.0,
            1.0,
        )
        .unwrap();
        assert_eq!(p.output, PlanetaryShaft::Carrier);
        assert!((p.ratio - (1.0 + zr / zs)).abs() < 1e-12, "{}", p.ratio);

        // Sun held, ring driving: 1 + z_s/z_r.
        let p = power(
            basic_ratio(t),
            Arrangement {
                input: PlanetaryShaft::Ring,
                fixed: PlanetaryShaft::Sun,
            },
            1000.0,
            1.0,
            1.0,
        )
        .unwrap();
        assert_eq!(p.output, PlanetaryShaft::Carrier);
        assert!((p.ratio - (1.0 + zs / zr)).abs() < 1e-12, "{}", p.ratio);

        // Carrier held: the sun and ring turn opposite ways, ratio −z_r/z_s.
        let p = power(
            basic_ratio(t),
            Arrangement {
                input: PlanetaryShaft::Sun,
                fixed: PlanetaryShaft::Carrier,
            },
            1000.0,
            1.0,
            1.0,
        )
        .unwrap();
        assert_eq!(p.output, PlanetaryShaft::Ring);
        assert!((p.ratio - (-zr / zs)).abs() < 1e-12, "{}", p.ratio);
        assert!(p.ratio < 0.0, "a fixed carrier reverses the output");
    }

    /// A set built from lossless meshes is lossless, in **all six** arrangements
    /// and exactly — so the loss term enters only through `η₀` and nothing else
    /// leaks.
    /// **Self-consistency in the rolling sign does not pick the branch on its
    /// own**, and where it cannot, energy is what decides.
    ///
    /// `k = i₀ η₀^w` sits either side of 1 as `w` flips, so where `i₀` is itself
    /// close to 1 the two candidates straddle it, `1 − k` changes sign between
    /// them, and both confirm their own assumption. One of the two has the
    /// output's torque along its own rotation — a shaft delivering power while
    /// the input delivers too — which is energy from nowhere and shows up as an
    /// efficiency above 1. Taking the first self-consistent branch returned
    /// exactly that: 1.22 at `i₀ = 144/143`.
    ///
    /// This is the regime of a set that reduces by the square of a tooth count,
    /// so it is ordinary rather than pathological — and nothing about the
    /// arrangement warns of it, which is why the guard is on the physics.
    #[test]
    fn a_basic_ratio_near_one_still_loses_power() {
        for (num, den) in [(144_i64, 143_i64), (324, 323), (900, 899), (2500, 2499)] {
            let i0 = num as f64 / den as f64;
            for eta0 in [0.98, 0.99, 0.995] {
                let p = power(
                    i0,
                    Arrangement {
                        input: PlanetaryShaft::Carrier,
                        fixed: PlanetaryShaft::Sun,
                    },
                    1000.0,
                    2.0,
                    eta0,
                )
                .unwrap_or_else(|| panic!("i0 {i0} eta0 {eta0} should solve"));
                assert!(
                    p.efficiency > 0.0 && p.efficiency < 1.0,
                    "i0 {i0} eta0 {eta0}: efficiency {} is not one",
                    p.efficiency
                );
                let out = PlanetaryShaft::Ring.index_pub();
                assert!(
                    p.torques[out] * p.speeds[out] <= 0.0,
                    "i0 {i0}: the output delivers power as well as the input"
                );
            }
        }
    }

    /// **The planet's own speed is what both of its meshes say.**
    ///
    /// [`Power::planet_speed`] reads it off the sun mesh in the carrier's frame;
    /// this rebuilds it from the *ring* mesh, `+(z_r/z_p)(ω_r − ω_c)`. Different
    /// pair, different tooth counts, and the two agree only if the relative
    /// speed is genuinely the planet's rather than the sun's — which is the
    /// mistake this gates, and the one the stage made: it reported the sun's
    /// relative speed as the planet's, a third out on an ordinary set, and the
    /// carrier's absolute speed as the planet's, which on a set with the ring
    /// held is not even the same sign.
    #[test]
    fn the_planets_own_speed_is_what_both_of_its_meshes_say() {
        for teeth in [
            Teeth {
                sun: 24,
                planet: 18,
                ring: 60,
            },
            Teeth {
                sun: 17,
                planet: 17,
                ring: 51,
            },
            Teeth {
                sun: 40,
                planet: 10,
                ring: 60,
            },
        ] {
            for fixed in [
                PlanetaryShaft::Ring,
                PlanetaryShaft::Sun,
                PlanetaryShaft::Carrier,
            ] {
                let input = match fixed {
                    PlanetaryShaft::Ring | PlanetaryShaft::Carrier => PlanetaryShaft::Sun,
                    PlanetaryShaft::Sun => PlanetaryShaft::Carrier,
                };
                let p = power(
                    basic_ratio(teeth),
                    Arrangement { input, fixed },
                    3000.0,
                    2.0,
                    1.0,
                )
                .unwrap_or_else(|| panic!("{teeth:?} held at {fixed:?} should solve"));
                let carrier = p.speeds[PlanetaryShaft::Carrier.index_pub()];
                let ring = p.speeds[PlanetaryShaft::Ring.index_pub()];
                let want = (f64::from(teeth.ring) / f64::from(teeth.planet)) * (ring - carrier);
                let (absolute, relative) = p.planet_speed(teeth);
                assert!(
                    (relative - want).abs() < 1e-9 * want.abs().max(1.0),
                    "{teeth:?} held at {fixed:?}: the sun mesh gives {relative}, the ring {want}"
                );
                assert!(
                    (absolute - (carrier + relative)).abs() < 1e-9,
                    "the absolute speed is the carrier's plus its own spin"
                );
            }
        }
    }

    /// A set that loses more than half of what it is given cannot be driven
    /// backwards: the reversed flow has no branch where the output absorbs, and
    /// the classical `2 − 1/η` for such a set is negative.
    #[test]
    fn a_set_below_half_efficiency_does_not_back_drive() {
        let i0 = 324.0 / 323.0;
        let fixed = PlanetaryShaft::Sun;
        let forward = power(
            i0,
            Arrangement {
                input: PlanetaryShaft::Carrier,
                fixed,
            },
            1000.0,
            2.0,
            0.99,
        )
        .unwrap();
        assert!(forward.efficiency < 0.5, "{}", forward.efficiency);
        let out = PlanetaryShaft::Ring.index_pub();
        let back = power(
            i0,
            Arrangement {
                input: PlanetaryShaft::Ring,
                fixed,
            },
            forward.speeds[out],
            forward.torques[out].abs(),
            0.99,
        );
        assert!(
            back.is_none(),
            "back-driving should have no state at all, not {:?}",
            back.map(|p| p.efficiency)
        );
        assert!(
            2.0 - 1.0 / forward.efficiency < 0.0,
            "and the relation agrees"
        );
    }

    #[test]
    fn a_lossless_set_is_lossless_in_every_arrangement() {
        for a in arrangements() {
            let p = power(basic_ratio(teeth()), a, 1500.0, 3.0, 1.0).unwrap();
            assert!(
                (p.efficiency - 1.0).abs() < 1e-12,
                "{a:?}: efficiency {}",
                p.efficiency
            );
            // ...and the power balance closes exactly.
            let out = p.torques[p.output.index()] * p.speeds[p.output.index()];
            assert!((out.abs() - (3.0 * 1500.0f64).abs()).abs() < 1e-9, "{a:?}");
        }
    }

    /// The three torques sum to zero in every arrangement — the set is in
    /// equilibrium, and the held shaft's torque is the reaction it carries.
    /// Getting this wrong is how a loss term ends up creating power.
    #[test]
    fn the_torques_are_in_equilibrium() {
        for a in arrangements() {
            for eta in [1.0, 0.98, 0.9] {
                let p = power(basic_ratio(teeth()), a, 1500.0, 3.0, eta).unwrap();
                let sum: f64 = p.torques.iter().sum();
                assert!(
                    sum.abs() < 1e-9 * p.torques[0].abs().max(1.0),
                    "{a:?} eta={eta}: torques sum to {sum}"
                );
            }
        }
    }

    /// **Loss costs power, never makes it** — in every arrangement. The sign
    /// convention that is easy to get backwards, and the one a planetary punishes
    /// hardest, because `η₀` appears as a *power*.
    #[test]
    fn friction_never_pays() {
        for a in arrangements() {
            let ideal = power(basic_ratio(teeth()), a, 1500.0, 3.0, 1.0).unwrap();
            let mut last = ideal.efficiency;
            for eta in [0.99, 0.97, 0.94, 0.9] {
                let p = power(basic_ratio(teeth()), a, 1500.0, 3.0, eta).unwrap();
                assert!(
                    p.efficiency <= 1.0,
                    "{a:?} eta={eta}: efficiency {} exceeds one",
                    p.efficiency
                );
                assert!(
                    p.efficiency < last,
                    "{a:?} eta={eta}: {} did not fall below {last}",
                    p.efficiency
                );
                // The output speed is kinematic and loss cannot touch it.
                assert!((p.ratio - ideal.ratio).abs() < 1e-12);
                last = p.efficiency;
            }
        }
    }

    /// **With the carrier held, the answer must be exactly `η₀`.**
    ///
    /// A fixed-carrier set *is* two ordinary meshes in series — that is what
    /// `η₀` means — so this is the one arrangement whose efficiency is known in
    /// advance, and it comes out of the general algebra rather than being
    /// short-circuited. `|k/i₀| = η₀^w` exactly, with nothing left over.
    #[test]
    fn a_held_carrier_gives_exactly_the_fixed_carrier_efficiency() {
        for eta0 in [1.0, 0.99, 0.98, 0.9, 0.75] {
            for input in [PlanetaryShaft::Sun, PlanetaryShaft::Ring] {
                let p = power(
                    basic_ratio(teeth()),
                    Arrangement {
                        input,
                        fixed: PlanetaryShaft::Carrier,
                    },
                    1000.0,
                    4.0,
                    eta0,
                )
                .unwrap();
                assert!(
                    (p.efficiency - eta0).abs() < 1e-12,
                    "{input:?} eta0={eta0}: got {}",
                    p.efficiency
                );
            }
        }
    }

    /// **And with the ring held, the published closed form.**
    ///
    /// `η = (1 − i₀ η₀) / (1 − i₀)` for a sun-in, carrier-out set — derived
    /// independently of the code, which reaches it through the torque shares. Note
    /// what it says: the answer is **above** `η₀`, because only part of the power
    /// passes through the meshes at all. That is the result a mesh-by-mesh
    /// calculation gets wrong in the optimistic direction for some arrangements
    /// and the pessimistic direction for others.
    #[test]
    fn a_held_ring_matches_the_published_closed_form() {
        let t = teeth();
        let i0 = basic_ratio(t);
        for eta0 in [1.0, 0.99, 0.98, 0.95] {
            let p = power(
                basic_ratio(t),
                Arrangement {
                    input: PlanetaryShaft::Sun,
                    fixed: PlanetaryShaft::Ring,
                },
                1000.0,
                4.0,
                eta0,
            )
            .unwrap();
            let want = (1.0 - i0 * eta0) / (1.0 - i0);
            assert!(
                (p.efficiency - want).abs() < 1e-12,
                "eta0={eta0}: {} vs {want}",
                p.efficiency
            );
            assert!(
                p.efficiency >= eta0,
                "eta0={eta0}: a carrier-output set should beat its meshes"
            );
        }
    }

    /// **Efficiency never exceeds one in either drive sense** — the test that was
    /// missing, and that a 101.571 % figure in the running application found.
    ///
    /// Reversing a set means the shaft that was the output now drives, so its
    /// torque must have the same sign as its speed. Hand it the *reaction* torque
    /// the forward solution left there and the rolling power comes out the wrong
    /// way round, `η₀^w` takes the wrong branch, and the answer is above unity.
    /// Every earlier test drove forward with a positive torque and a positive
    /// speed, so none of them could see it.
    #[test]
    fn no_arrangement_is_efficient_above_one_in_either_direction() {
        for a in arrangements() {
            for eta0 in [1.0, 0.99, 0.97, 0.9] {
                // Both senses of a genuinely driving input.
                for (speed, torque) in [(1500.0, 3.0), (-1500.0, -3.0)] {
                    let p = power(basic_ratio(teeth()), a, speed, torque, eta0).unwrap();
                    assert!(
                        p.efficiency <= 1.0 + 1e-12,
                        "{a:?} eta0={eta0} n={speed} T={torque}: efficiency {}",
                        p.efficiency
                    );
                    if eta0 < 1.0 {
                        assert!(p.efficiency < 1.0, "{a:?} eta0={eta0}: lossless at a loss");
                    }
                }
                // ...and a shaft that absorbs power is not an input. Asking
                // anyway used to return `1/η₀`, above one, which is the
                // arithmetic saying the question was put the wrong way round.
                assert!(
                    power(basic_ratio(teeth()), a, 1500.0, -3.0, eta0).is_none(),
                    "{a:?}: a non-driving input must be refused"
                );
            }
        }
    }

    /// **A coupled planetary is not as efficient as its meshes.**
    ///
    /// The result worth surfacing, and the reason docs/reference.md#planetary-sets refuses a mesh-by-mesh
    /// calculation: the meshes slide at their speeds relative to the *carrier*, so
    /// as the ratio grows the recirculating power grows with it and the overall
    /// efficiency falls well below `η₀` — from meshes that never change.
    #[test]
    fn a_high_ratio_set_loses_more_than_its_meshes_do() {
        let eta0 = 0.98;
        let mut previous = f64::INFINITY;
        // Growing the sun against a fixed ring raises the carrier-output ratio.
        for sun in [60u32, 40, 30, 24, 20, 18] {
            let t = Teeth {
                sun,
                planet: 18,
                ring: 60,
            };
            let p = power(
                basic_ratio(t),
                Arrangement {
                    input: PlanetaryShaft::Carrier,
                    fixed: PlanetaryShaft::Ring,
                },
                1000.0,
                5.0,
                eta0,
            )
            .unwrap();
            assert!(p.efficiency < 1.0);
            assert!(
                p.efficiency < previous,
                "z_sun={sun}: efficiency {} did not fall below {previous}",
                p.efficiency
            );
            previous = p.efficiency;
        }
        // Even so, a carrier-output set stays *above* its mesh product: it is the
        // sun-or-ring-output arrangements that fall below. Stated as a comparison
        // because the direction is the whole point.
        let t = teeth();
        let carrier_out = power(
            basic_ratio(t),
            Arrangement {
                input: PlanetaryShaft::Sun,
                fixed: PlanetaryShaft::Ring,
            },
            1000.0,
            5.0,
            eta0,
        )
        .unwrap();
        let ring_out = power(
            basic_ratio(t),
            Arrangement {
                input: PlanetaryShaft::Sun,
                fixed: PlanetaryShaft::Carrier,
            },
            1000.0,
            5.0,
            eta0,
        )
        .unwrap();
        assert!(
            carrier_out.efficiency > ring_out.efficiency,
            "{} vs {}",
            carrier_out.efficiency,
            ring_out.efficiency
        );
    }

    /// The held shaft never turns, and the shaft that is neither driven nor held
    /// is the one reported as the output.
    #[test]
    fn the_held_shaft_is_still_and_the_third_is_the_output() {
        for a in arrangements() {
            let p = power(basic_ratio(teeth()), a, 1234.0, 7.0, 0.98).unwrap();
            assert_eq!(p.speeds[a.fixed.index()], 0.0);
            assert_eq!(p.speeds[a.input.index()], 1234.0);
            assert_ne!(p.output, a.input);
            assert_ne!(p.output, a.fixed);
        }
    }

    /// Driving and holding the same shaft is not an arrangement.
    #[test]
    fn a_shaft_cannot_be_both_driven_and_held() {
        for m in [
            PlanetaryShaft::Sun,
            PlanetaryShaft::Carrier,
            PlanetaryShaft::Ring,
        ] {
            assert!(power(
                basic_ratio(teeth()),
                Arrangement { input: m, fixed: m },
                1000.0,
                1.0,
                0.98
            )
            .is_none());
        }
    }
}
