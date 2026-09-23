//! **Where the power goes, mesh by mesh, with loss** — one model for every
//! shape a stage can take.
//!
//! The ideal answer is the rowspace of the kinematic matrix
//! ([`crate::kinematics::System::torques`]): every mesh transmits its torque
//! whole, and the torques on the bodies follow from which of them are known.
//! Loss makes that non-linear in exactly one way — a mesh charges its
//! efficiency in the direction power crosses it, and which direction that is
//! depends on the answer. So the lossy solve is the ideal one with each mesh's
//! driven side scaled by `η` or `1/η` **under an assumed direction per mesh**,
//! and the assumption checked against what it produces. `2^M` assumptions for
//! `M` meshes, filtered by the two conditions `crate::planetary::power` already
//! applies to its one sign: each mesh's assumed direction is the direction the
//! solution puts power across it, and the output absorbs what the input
//! delivers. With `M = 2` and the planet carrying no torque of its own, the
//! two meshes carry power the same way and this *is* `η₀^w`
//! (`docs/reference.md#planetary-sets`) — which the tests hold on every
//! arrangement, and which is the reason a general model can be believed on
//! the arrangements that had no model before.
//!
//! # What a mesh row says about torque
//!
//! A mesh between member `a` on body `A` and member `b` on body `B`, in
//! frame `F`, is the kinematic row `z_a(ω_A − ω_F) + z_b(ω_B − ω_F) = 0` with
//! the tooth counts signed. Virtual work over that row puts the torques on the
//! three bodies in the ratio `z_a : z_b : −(z_a + z_b)` — one tangential force
//! `F` at the two reference cylinders and its reaction at the carrier radius.
//! With loss, the driven member's torque is `η` of what the row says: the
//! mesh is parametrised by its **driver's** torque `t` at its own count, so
//! `τ_a = t z_a, τ_b = η t z_b` where `a` drives `b` and `τ_b = t z_b, τ_a =
//! η t z_a` where `b` drives `a`, and the frame's is the negative sum, which
//! is the moment balance of the three bodies. Written from the driver's side
//! so that `η = 0` is a mesh that **holds** — the driver presses the flanks
//! and the driven member gets nothing, which is what a self-locking screw
//! under a load from the wheel is — rather than a division by nought. That
//! is the whole of the model, and it is written once.
//!
//! # No gear here
//!
//! This module takes bodies, signed counts, speeds and efficiencies, and
//! returns torques. The stage that calls it knows which member is a ring and
//! how many planets there are; this knows only that a count is negative and a
//! mesh is one of `paths` alike.

use crate::contact::{Directional, Drive};
use crate::kinematics::{Body, GROUND};

/// One mesh as the power-flow solve sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshFlow {
    /// The body member `a` spins with, and `b`'s.
    pub a: Body,
    pub b: Body,
    /// The body both axes stand still in.
    pub frame: Body,
    /// Signed tooth counts, a ring's negative.
    pub za: f64,
    pub zb: f64,
    /// The mesh's own efficiency: `forward` where `a` drives `b`, `backward`
    /// where `b` drives `a`. Zero or below in a direction is a mesh that
    /// cannot be driven that way — a self-locking screw — and *holds* under a
    /// load from that side: the driver's torque presses the flanks and
    /// nothing comes out the other.
    pub efficiency: Directional<f64>,
    /// How many identical instances of this mesh act in parallel — one per
    /// planet — so a torque on a central member is `paths` times what one
    /// instance carries.
    pub paths: f64,
}

/// What is asked of the flow: **which bodies' external torques are known**,
/// which are to be found, and — by omission — which carry none.
///
/// A load whose torque a designer gave is known; a load whose torque is
/// derived, a body held to ground, and ground itself are unknown; every
/// other body — an idler, a planet, a coupling inside the train — carries
/// no external torque. One input and one output is the case of one known
/// and one unknown ([`Asked::through`]); a differential with two inputs is
/// two known, and the flow is the same solve.
#[derive(Clone, Debug, PartialEq)]
pub struct Asked {
    /// Per body: `Some(torque)` where it is known, `None` where it is to be
    /// found. Bodies past the end of the list carry none.
    pub known: Vec<Option<f64>>,
    /// Bodies whose torque is to be found: the loads whose torque is
    /// derived, the held bodies, and ground.
    pub unknown: Vec<Body>,
}

impl Asked {
    /// One body driving at `torque`, one taking the power out, the rest
    /// held or carrying nothing.
    #[must_use]
    pub fn through(
        bodies: usize,
        input: Body,
        torque: f64,
        output: Body,
        reactions: &[Body],
    ) -> Self {
        let mut known: Vec<Option<f64>> = (0..bodies)
            .map(|s| (s != GROUND && s != output && !reactions.contains(&s)).then_some(0.0))
            .collect();
        known[input] = Some(torque);
        let unknown = (0..bodies).filter(|&s| known[s].is_none()).collect();
        Self { known, unknown }
    }

    fn known_at(&self, s: Body) -> Option<f64> {
        if self.unknown.contains(&s) {
            None
        } else {
            Some(self.known.get(s).copied().flatten().unwrap_or(0.0))
        }
    }
}

/// The flow as it came out.
#[derive(Clone, Debug, PartialEq)]
pub struct Flow {
    /// Per mesh, **the driver's torque read across to member `a`**, summed
    /// over its paths — what presses the flanks, at `a`'s reference cylinder,
    /// whichever member is driving; what one instance carries is this over
    /// `paths`. Signed: positive drives `a` the way its speed goes. The driven
    /// member's own torque is `η` of what this says at its count.
    pub mesh_torques: Vec<f64>,
    /// Per mesh, which member drives: `Forward` is `a`.
    pub directions: Vec<Drive>,
    /// Per mesh, the factor the driven member's torque stands under — its
    /// efficiency that way, nought where it holds.
    pub factors: Vec<f64>,
    /// Per body, the external torque it carries: the input's as given, the
    /// output's and each reaction as found, and zero elsewhere. Ground's is
    /// the sum of what meshes against it.
    pub shaft_torques: Vec<f64>,
    /// `|P_out| / P_in` — the power the bodies delivering it take out over
    /// the power the bodies driving it put in.
    pub efficiency: f64,
    /// **The power crossing each mesh, over the power in** — the driving
    /// side's `|τ (ω − ω_frame)|` per unit of what the input delivers, so
    /// that each mesh's loss is `(1 − η)` of it exactly. One on a
    /// pair's mesh, where all of it crosses; under one on a set's, where the
    /// carrier carries part of it bodily; and many times one where power
    /// circulates — a hula stage at hundreds to one, whose two meshes each
    /// pass a large multiple of the input to cancel to the output, which is
    /// where its efficiency goes.
    pub mesh_powers: Vec<f64>,
}

impl Flow {
    /// **What mesh `k` puts on its three bodies** — `a`'s, `b`'s and the
    /// frame's — the driver's whole, the driven member's under `η`, and the
    /// frame's the negative sum: the moment balance of the three bodies. Summed
    /// over a stage's meshes, a body's is the torque that stage delivers on
    /// it — the external load at a port, what it passes on at a coupling.
    #[must_use]
    pub fn on_shafts(&self, k: usize, mesh: &MeshFlow) -> [f64; 3] {
        let t = if mesh.za == 0.0 {
            0.0
        } else {
            self.mesh_torques[k] / mesh.za
        };
        let (on_a, on_b) = match self.directions[k] {
            Drive::Forward => (t * mesh.za, self.factors[k] * t * mesh.zb),
            Drive::Backward => (self.factors[k] * t * mesh.za, t * mesh.zb),
        };
        [on_a, on_b, -(on_a + on_b)]
    }
}

/// A tolerance for "this power is zero", relative to the powers in play.
const ZERO: f64 = 1e-12;

/// Why there is no flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// No known torque does any work — every one is nought or on a still
    /// body — so there is no power to follow.
    NothingDrives,
    /// The known torques do not determine the mesh torques — more unknown
    /// reactions than the meshes can tell apart, a load between two bodies
    /// both of which hold it, which is a division by stiffness this model
    /// does not make.
    Undetermined,
    /// The known torques contradict the rows in every assignment of
    /// directions: a load nothing holds, which turns the train.
    Inconsistent,
}

/// **The power flow**, given every body's speed.
///
/// `None` where no known torque does any work, where no assignment of
/// directions is self-consistent, or
/// where the known torques do not determine the mesh torques, which is a
/// stage with more free bodies than a rating can be taken under. A stage
/// that locks in this direction is **not** `None`: its locked mesh holds,
/// the flow through it stops there, and the efficiency is nought.
///
/// # Errors
///
/// [`Refused`] says which of the three.
pub fn solve(
    bodies: usize,
    meshes: &[MeshFlow],
    speed: &[f64],
    asked: &Asked,
) -> Result<Flow, Refused> {
    // The work the known torques do, either way: the scale a power is
    // nought against. A known torque that works *against* its body is a
    // load absorbing, and who drives it is one of the unknowns — a derived
    // load, or a reacted port — so a known driver is not required; a case
    // with no work known at all has nothing to follow.
    let input_power: f64 = (0..bodies)
        .filter_map(|s| asked.known_at(s).map(|t| (t * speed[s]).abs()))
        .sum();
    if input_power <= 0.0 || !input_power.is_finite() {
        return Err(Refused::NothingDrives);
    }
    let mut why = Refused::Inconsistent;
    let known = |s: Body| -> Option<f64> { asked.known_at(s) };
    let m = meshes.len();
    let mut best: Option<Flow> = None;
    for assignment in 0..(1u32 << m) {
        let directions: Vec<Drive> = (0..m)
            .map(|k| {
                if assignment & (1 << k) == 0 {
                    Drive::Forward
                } else {
                    Drive::Backward
                }
            })
            .collect();
        // The driven side's factor under this assignment: `η`, and nought
        // where the mesh cannot be driven this way at all — it then holds.
        let factor: Option<Vec<f64>> = meshes
            .iter()
            .zip(&directions)
            .map(|(mesh, d)| {
                let eta = *mesh.efficiency.get(*d);
                eta.is_finite().then_some(eta.max(0.0))
            })
            .collect();
        let Some(factor) = factor else { continue };
        // The torque each mesh puts on each body per unit of its driver's
        // `t`: the driver's count whole, the driven member's under `η`.
        let per_unit = |k: usize, s: Body| -> f64 {
            let mesh = &meshes[k];
            let (on_a, on_b) = match directions[k] {
                Drive::Forward => (mesh.za, factor[k] * mesh.zb),
                Drive::Backward => (factor[k] * mesh.za, mesh.zb),
            };
            let mut t = 0.0;
            if mesh.a == s {
                t += on_a;
            }
            if mesh.b == s {
                t += on_b;
            }
            if mesh.frame == s {
                t -= on_a + on_b;
            }
            t
        };
        // One equation per body whose torque is known.
        let rows: Vec<Vec<f64>> = (1..bodies)
            .filter_map(|s| {
                known(s).map(|t| {
                    let mut row: Vec<f64> = (0..m).map(|k| per_unit(k, s)).collect();
                    row.push(t);
                    row
                })
            })
            .collect();
        let c = match least_squares_exact(rows, m) {
            Ok(c) => c,
            Err(undetermined) => {
                if undetermined {
                    why = Refused::Undetermined;
                }
                continue;
            }
        };
        let shaft_torques: Vec<f64> = (0..bodies)
            .map(|s| (0..m).map(|k| per_unit(k, s) * c[k]).sum())
            .collect();
        // The power the assumed driver puts into each mesh: its torque
        // times its speed relative to the frame.
        let driving_power = |k: usize| -> f64 {
            let mesh = &meshes[k];
            match directions[k] {
                Drive::Forward => c[k] * mesh.za * (speed[mesh.a] - speed[mesh.frame]),
                Drive::Backward => c[k] * mesh.zb * (speed[mesh.b] - speed[mesh.frame]),
            }
        };
        // Each mesh's assumed direction must be the one the solution puts
        // power across it: the driver's torque works with its speed relative
        // to the frame.
        let consistent = (0..m).all(|k| {
            let p = driving_power(k);
            p.abs() <= ZERO * input_power || p > 0.0
        });
        if !consistent {
            continue;
        }
        // ...and the train as a whole must lose power, not make it: what
        // every body puts in, less what every body takes out, is the loss,
        // and a branch that has it negative is the spurious one — an output
        // delivering power while the input delivers too, with friction
        // making up the difference. One input and one output reads as "the
        // output absorbs what the input delivers".
        let powers: Vec<f64> = (0..bodies).map(|s| shaft_torques[s] * speed[s]).collect();
        let loss: f64 = powers.iter().sum();
        if loss < -ZERO * input_power {
            continue;
        }
        // **A power within the solve's zero is nought**, on either side. A
        // body the case leaves free carries a torque of rounding's size,
        // and counted as power out it made a drive that cannot move
        // deliver a few parts in 10¹⁵ — which is positive, and so broke
        // away at rest ([`crate::contact::Directional::once_moving`]) by
        // the width of a rounding: a compound back-driven at its static
        // friction did, alone and after a spur, and did not after an idler.
        let zero = ZERO * input_power;
        let p_in: f64 = powers.iter().filter(|p| **p > zero).sum();
        let p_out: f64 = powers.iter().filter(|p| **p < -zero).sum();
        if p_in <= zero {
            continue;
        }
        let efficiency = p_out.abs() / p_in;
        // The power on the **driving** side of each mesh, as a fraction of
        // what the train is given, so a mesh's loss is `(1 − η)` of this,
        // exactly.
        let mesh_powers = (0..m).map(|k| driving_power(k).abs() / p_in).collect();
        let flow = Flow {
            mesh_torques: c.iter().zip(meshes).map(|(c, m)| c * m.za).collect(),
            directions,
            factors: factor,
            shaft_torques,
            efficiency,
            mesh_powers,
        };
        // Two assignments can both be consistent where a mesh carries no
        // power at all and either direction is vacuously right; they give one
        // answer. Otherwise the first consistent one is the answer, and a
        // second genuinely different one would be a finding worth a test.
        if best.as_ref().is_none_or(|b| efficiency > b.efficiency) {
            best = Some(flow);
        }
    }
    best.ok_or(why)
}

/// Gaussian elimination on an over- or exactly-determined system whose rows
/// are consistent: `Err(true)` where the unknowns are not all determined,
/// `Err(false)` where the rows contradict each other.
fn least_squares_exact(mut rows: Vec<Vec<f64>>, unknowns: usize) -> Result<Vec<f64>, bool> {
    let mut pivots: Vec<usize> = Vec::new();
    let mut r = 0;
    for col in 0..unknowns {
        // The largest remaining entry in this column.
        let Some((p, _)) = rows
            .iter()
            .enumerate()
            .skip(r)
            .map(|(i, row)| (i, row[col].abs()))
            .filter(|(_, v)| *v > 1e-12)
            .max_by(|x, y| x.1.total_cmp(&y.1))
        else {
            continue;
        };
        rows.swap(r, p);
        let pivot = rows[r][col];
        rows[r].iter_mut().for_each(|x| *x /= pivot);
        let lead = rows[r].clone();
        for (i, row) in rows.iter_mut().enumerate() {
            let f = row[col];
            if i != r && f != 0.0 {
                row.iter_mut().zip(&lead).for_each(|(x, l)| *x -= f * l);
            }
        }
        pivots.push(col);
        r += 1;
    }
    if pivots.len() < unknowns {
        return Err(true);
    }
    // Every row past the rank must have come out as `0 = 0`.
    let scale = rows
        .iter()
        .map(|row| row[unknowns].abs())
        .fold(1.0_f64, f64::max);
    if rows
        .iter()
        .skip(r)
        .any(|row| row[unknowns].abs() > 1e-9 * scale)
    {
        return Err(false);
    }
    Ok((0..unknowns).map(|k| rows[k][unknowns]).collect())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::planetary::{self, Arrangement, PlanetaryShaft, Teeth};

    /// Bodies `[ground, sun, carrier, ring, planet]`, as the wiring numbers a
    /// set; the planet is the fourth.
    const SUN: Body = 1;
    const CARRIER: Body = 2;
    const RING: Body = 3;
    const PLANET: Body = 4;

    fn set(t: Teeth, eta_sp: f64, eta_pr: f64, planets: f64) -> Vec<MeshFlow> {
        let same = |e: f64| Directional {
            forward: e,
            backward: e,
        };
        vec![
            MeshFlow {
                a: SUN,
                b: PLANET,
                frame: CARRIER,
                za: f64::from(t.sun),
                zb: f64::from(t.planet),
                efficiency: same(eta_sp),
                paths: planets,
            },
            MeshFlow {
                a: PLANET,
                b: RING,
                frame: CARRIER,
                za: f64::from(t.planet),
                zb: -f64::from(t.ring),
                efficiency: same(eta_pr),
                paths: planets,
            },
        ]
    }

    /// Willis at the held body, per turn of the input.
    fn speeds(t: Teeth, arrangement: Arrangement) -> [f64; 5] {
        let p = planetary::power(planetary::basic_ratio(t), arrangement, 1.0, 1.0, 1.0).unwrap();
        let s = p.speeds;
        // The planet, relative to the carrier: `z_s(ω_s − ω_c) = −z_p(ω_p − ω_c)`.
        let planet = s[1] - f64::from(t.sun) * (s[0] - s[1]) / f64::from(t.planet);
        [0.0, s[0], s[1], s[2], planet]
    }

    fn shaft(p: PlanetaryShaft) -> Body {
        match p {
            PlanetaryShaft::Sun => SUN,
            PlanetaryShaft::Carrier => CARRIER,
            PlanetaryShaft::Ring => RING,
        }
    }

    /// **The per-mesh model reproduces Pennestrì on every arrangement**: the
    /// efficiency to a part in `10¹²`, and every body's torque — with the
    /// planet, which Pennestrì has no torque for, carrying none.
    #[test]
    fn the_per_mesh_flow_is_pennestri_on_a_simple_set() {
        let mut checked = 0;
        for t in [
            Teeth {
                sun: 24,
                planet: 18,
                ring: 60,
            },
            Teeth {
                sun: 12,
                planet: 30,
                ring: 72,
            },
            Teeth {
                sun: 41,
                planet: 20,
                ring: 81,
            },
        ] {
            for (eta_sp, eta_pr) in [(0.98, 0.99), (0.9, 0.95), (0.999, 0.999)] {
                for input in PlanetaryShaft::ALL {
                    for fixed in PlanetaryShaft::ALL {
                        let Some(output) = PlanetaryShaft::other(input, fixed) else {
                            continue;
                        };
                        let arrangement = Arrangement { input, fixed };
                        let eta0 = eta_sp * eta_pr;
                        let want = planetary::power(
                            planetary::basic_ratio(t),
                            arrangement,
                            1.0,
                            1.0,
                            eta0,
                        )
                        .expect("every arrangement of a simple set has a flow");
                        let got = solve(
                            5,
                            &set(t, eta_sp, eta_pr, 3.0),
                            &speeds(t, arrangement),
                            &Asked::through(5, shaft(input), 1.0, shaft(output), &[shaft(fixed)]),
                        )
                        .expect("...and so does the per-mesh model");
                        assert!(
                            (got.efficiency - want.efficiency).abs() < 1e-12,
                            "{arrangement:?} {t:?}: {} vs {}",
                            got.efficiency,
                            want.efficiency
                        );
                        for (k, p) in PlanetaryShaft::ALL.iter().enumerate() {
                            assert!(
                                (got.shaft_torques[shaft(*p)] - want.torques[k]).abs() < 1e-12,
                                "{arrangement:?}: {p:?} {} vs {}",
                                got.shaft_torques[shaft(*p)],
                                want.torques[k]
                            );
                        }
                        assert!(got.shaft_torques[PLANET].abs() < 1e-12);
                        checked += 1;
                    }
                }
            }
        }
        assert_eq!(checked, 3 * 3 * 6);
    }

    /// A pair: one mesh, the output's torque is the input's times the ratio
    /// and the efficiency, whichever way it is driven; and a mesh that cannot
    /// be driven backward is a stage that locks.
    #[test]
    fn a_pair_is_one_mesh_and_a_locked_mesh_is_a_locked_stage() {
        let mesh = |backward: f64| MeshFlow {
            a: 1,
            b: 2,
            frame: GROUND,
            za: 17.0,
            zb: 43.0,
            efficiency: Directional {
                forward: 0.98,
                backward,
            },
            paths: 1.0,
        };
        let speed = [0.0, 1.0, -17.0 / 43.0];
        let forward = solve(3, &[mesh(0.9)], &speed, &Asked::through(3, 1, 2.0, 2, &[])).unwrap();
        assert!((forward.efficiency - 0.98).abs() < 1e-12);
        assert!((forward.shaft_torques[2] - 2.0 * 43.0 / 17.0 * 0.98).abs() < 1e-12);
        assert_eq!(forward.directions, vec![Drive::Forward]);
        let back = solve(
            3,
            &[mesh(0.9)],
            &[0.0, -43.0 / 17.0, 1.0],
            &Asked::through(3, 2, 1.0, 1, &[]),
        )
        .unwrap();
        assert!((back.efficiency - 0.9).abs() < 1e-12);
        assert_eq!(back.directions, vec![Drive::Backward]);
        // **A locked mesh holds.** Driven from the wheel it transmits
        // nothing: the wheel's torque is the load, the worm's is nought, and
        // the efficiency is nought — a flow rather than a refusal, so the
        // flanks the load is held on can be rated.
        let held = solve(
            3,
            &[mesh(0.0)],
            &[0.0, -43.0 / 17.0, 1.0],
            &Asked::through(3, 2, 1.0, 1, &[]),
        )
        .unwrap();
        assert_eq!(held.efficiency, 0.0);
        assert_eq!(held.shaft_torques[1], 0.0);
        assert_eq!(held.directions, vec![Drive::Backward]);
        // The driver's torque read across to `a`: the wheel's, at the worm's
        // count.
        assert!((held.mesh_torques[0].abs() - 17.0 / 43.0).abs() < 1e-12);
        assert!((held.mesh_powers[0] - 1.0).abs() < 1e-12);
    }

    /// Two inputs on a set leave the mesh torques undetermined, and the
    /// solve says so rather than choosing.
    #[test]
    fn a_second_input_is_not_a_reaction_and_the_flow_is_undetermined() {
        let t = Teeth {
            sun: 24,
            planet: 18,
            ring: 60,
        };
        let arrangement = Arrangement {
            input: PlanetaryShaft::Sun,
            fixed: PlanetaryShaft::Ring,
        };
        assert_eq!(
            solve(
                5,
                &set(t, 0.98, 0.99, 3.0),
                &speeds(t, arrangement),
                &Asked::through(5, SUN, 1.0, CARRIER, &[RING, PLANET]),
            )
            .err(),
            Some(Refused::Undetermined)
        );
    }
}
