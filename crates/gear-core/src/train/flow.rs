//! **Where the power goes, mesh by mesh, with loss** — one model for every
//! shape a stage can take.
//!
//! The ideal answer is the rowspace of the kinematic matrix
//! ([`crate::kinematics::System::torques`]): every mesh transmits its torque
//! whole, and the torques on the shafts follow from which of them are known.
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
//! A mesh between member `a` on shaft `A` and member `b` on shaft `B`, in
//! frame `F`, is the kinematic row `z_a(ω_A − ω_F) + z_b(ω_B − ω_F) = 0` with
//! the tooth counts signed. Virtual work over that row puts the torques on the
//! three shafts in the ratio `z_a : z_b : −(z_a + z_b)` — one tangential force
//! `F` at the two reference cylinders and its reaction at the carrier radius.
//! With loss, the driven member's torque is `η` of what the row says
//! (`τ_b = η · c z_b` where `a` drives `b`, and `c z_b / η` where `b` drives
//! `a`), and the frame's is the negative sum, which is the moment balance of
//! the three bodies. That is the whole of the model, and it is written once.
//!
//! # No gear here
//!
//! This module takes shafts, signed counts, speeds and efficiencies, and
//! returns torques. The stage that calls it knows which member is a ring and
//! how many planets there are; this knows only that a count is negative and a
//! mesh is one of `paths` alike.

use crate::contact::{Directional, Drive};
use crate::kinematics::{Shaft, GROUND};

/// One mesh as the power-flow solve sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshFlow {
    /// The shaft member `a` spins with, and `b`'s.
    pub a: Shaft,
    pub b: Shaft,
    /// The shaft both axes stand still in.
    pub frame: Shaft,
    /// Signed tooth counts, a ring's negative.
    pub za: f64,
    pub zb: f64,
    /// The mesh's own efficiency: `forward` where `a` drives `b`, `backward`
    /// where `b` drives `a`. Zero or below in a direction is a mesh that
    /// cannot be driven that way — a self-locking screw.
    pub efficiency: Directional<f64>,
    /// How many identical instances of this mesh act in parallel — one per
    /// planet — so a torque on a central member is `paths` times what one
    /// instance carries.
    pub paths: f64,
}

/// What is asked of the flow: which shaft drives, at what torque, which one
/// takes the power out, and which carry a reaction nobody names.
#[derive(Clone, Debug, PartialEq)]
pub struct Asked {
    pub input: Shaft,
    pub torque: f64,
    pub output: Shaft,
    /// Shafts whose torque is a reaction to be found rather than a zero to
    /// be imposed: the held ones. Every other shaft that is neither the input
    /// nor the output carries no external torque.
    pub reactions: Vec<Shaft>,
}

/// The flow as it came out.
#[derive(Clone, Debug, PartialEq)]
pub struct Flow {
    /// Per mesh, **the torque on member `a` summed over its paths** — what one
    /// instance carries is this over `paths`. Signed: positive drives `a` the
    /// way its speed goes.
    pub mesh_torques: Vec<f64>,
    /// Per mesh, which member drives: `Forward` is `a`.
    pub directions: Vec<Drive>,
    /// Per shaft, the external torque it carries: the input's as given, the
    /// output's and each reaction as found, and zero elsewhere. Ground's is
    /// the sum of what meshes against it.
    pub shaft_torques: Vec<f64>,
    /// `|P_out| / P_in`.
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
    /// The power crossing every mesh, over the power in.
    #[must_use]
    pub fn circulation(&self) -> f64 {
        self.mesh_powers.iter().sum()
    }
}

/// A tolerance for "this power is zero", relative to the powers in play.
const ZERO: f64 = 1e-12;

/// **The power flow**, given every shaft's speed per turn of the input.
///
/// `None` where no assignment of directions is self-consistent — a stage that
/// locks in this direction, or an input that is not the one driving — or
/// where the known torques do not determine the mesh torques, which is a
/// stage with more free shafts than a rating can be taken under.
#[must_use]
pub fn solve(shafts: usize, meshes: &[MeshFlow], speed: &[f64], asked: &Asked) -> Option<Flow> {
    let input_power = asked.torque * speed[asked.input];
    if input_power <= 0.0 || !input_power.is_finite() {
        return None;
    }
    let known = |s: Shaft| -> Option<f64> {
        if s == GROUND || s == asked.output || asked.reactions.contains(&s) {
            None
        } else if s == asked.input {
            Some(asked.torque)
        } else {
            Some(0.0)
        }
    };
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
        // The driven side's factor under this assignment, or nothing where the
        // mesh cannot be driven this way at all.
        let factor: Option<Vec<f64>> = meshes
            .iter()
            .zip(&directions)
            .map(|(mesh, d)| {
                let eta = *mesh.efficiency.get(*d);
                (eta > 0.0 && eta.is_finite()).then_some(match d {
                    Drive::Forward => eta,
                    Drive::Backward => 1.0 / eta,
                })
            })
            .collect();
        let Some(factor) = factor else { continue };
        // The torque each mesh puts on each shaft per unit of its `c`.
        let per_unit = |k: usize, s: Shaft| -> f64 {
            let mesh = &meshes[k];
            let mut t = 0.0;
            if mesh.a == s {
                t += mesh.za;
            }
            if mesh.b == s {
                t += factor[k] * mesh.zb;
            }
            if mesh.frame == s {
                t -= mesh.za + factor[k] * mesh.zb;
            }
            t
        };
        // One equation per shaft whose torque is known.
        let rows: Vec<Vec<f64>> = (1..shafts)
            .filter_map(|s| {
                known(s).map(|t| {
                    let mut row: Vec<f64> = (0..m).map(|k| per_unit(k, s)).collect();
                    row.push(t);
                    row
                })
            })
            .collect();
        let Some(c) = least_squares_exact(rows, m) else {
            continue;
        };
        let shaft_torques: Vec<f64> = (0..shafts)
            .map(|s| (0..m).map(|k| per_unit(k, s) * c[k]).sum())
            .collect();
        // Each mesh's assumed direction must be the one the solution puts
        // power across it: `a` drives where its torque works with its speed
        // relative to the frame.
        let consistent = meshes.iter().enumerate().all(|(k, mesh)| {
            let p_a = c[k] * mesh.za * (speed[mesh.a] - speed[mesh.frame]);
            p_a.abs() <= ZERO * input_power
                || (p_a > 0.0) == matches!(directions[k], Drive::Forward)
        });
        if !consistent {
            continue;
        }
        // ...and the output must absorb what the input delivers.
        let p_out = shaft_torques[asked.output] * speed[asked.output];
        if p_out > ZERO * input_power {
            continue;
        }
        let efficiency = p_out.abs() / input_power;
        // The power on the **driving** side of each mesh: `a`'s where `a`
        // drives, and `a`'s over the mesh's efficiency where it is driven —
        // so a mesh's loss is `(1 − η)` of this, exactly.
        let mesh_powers = meshes
            .iter()
            .enumerate()
            .map(|(k, mesh)| {
                let at_a = (c[k] * mesh.za * (speed[mesh.a] - speed[mesh.frame])).abs();
                let driving = match directions[k] {
                    Drive::Forward => at_a,
                    Drive::Backward => at_a / *mesh.efficiency.get(Drive::Backward),
                };
                driving / input_power
            })
            .collect();
        let flow = Flow {
            mesh_torques: c.iter().zip(meshes).map(|(c, m)| c * m.za).collect(),
            directions,
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
    best
}

/// Gaussian elimination on an over- or exactly-determined system whose rows
/// are consistent: `None` where the unknowns are not all determined or the
/// rows contradict each other.
fn least_squares_exact(mut rows: Vec<Vec<f64>>, unknowns: usize) -> Option<Vec<f64>> {
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
        return None;
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
        return None;
    }
    Some((0..unknowns).map(|k| rows[k][unknowns]).collect())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::planetary::{self, Arrangement, PlanetaryShaft, Teeth};

    /// Shafts `[ground, sun, carrier, ring, planet]`, as the wiring numbers a
    /// set; the planet is the fourth.
    const SUN: Shaft = 1;
    const CARRIER: Shaft = 2;
    const RING: Shaft = 3;
    const PLANET: Shaft = 4;

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

    /// Willis at the held shaft, per turn of the input.
    fn speeds(t: Teeth, arrangement: Arrangement) -> [f64; 5] {
        let p = planetary::power(planetary::basic_ratio(t), arrangement, 1.0, 1.0, 1.0).unwrap();
        let s = p.speeds;
        // The planet, relative to the carrier: `z_s(ω_s − ω_c) = −z_p(ω_p − ω_c)`.
        let planet = s[1] - f64::from(t.sun) * (s[0] - s[1]) / f64::from(t.planet);
        [0.0, s[0], s[1], s[2], planet]
    }

    fn shaft(p: PlanetaryShaft) -> Shaft {
        match p {
            PlanetaryShaft::Sun => SUN,
            PlanetaryShaft::Carrier => CARRIER,
            PlanetaryShaft::Ring => RING,
        }
    }

    /// **The per-mesh model reproduces Pennestrì on every arrangement**: the
    /// efficiency to a part in `10¹²`, and every shaft's torque — with the
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
                            &Asked {
                                input: shaft(input),
                                torque: 1.0,
                                output: shaft(output),
                                reactions: vec![shaft(fixed)],
                            },
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
        let forward = solve(
            3,
            &[mesh(0.9)],
            &speed,
            &Asked {
                input: 1,
                torque: 2.0,
                output: 2,
                reactions: vec![],
            },
        )
        .unwrap();
        assert!((forward.efficiency - 0.98).abs() < 1e-12);
        assert!((forward.shaft_torques[2] - 2.0 * 43.0 / 17.0 * 0.98).abs() < 1e-12);
        assert_eq!(forward.directions, vec![Drive::Forward]);
        let back = solve(
            3,
            &[mesh(0.9)],
            &[0.0, -43.0 / 17.0, 1.0],
            &Asked {
                input: 2,
                torque: 1.0,
                output: 1,
                reactions: vec![],
            },
        )
        .unwrap();
        assert!((back.efficiency - 0.9).abs() < 1e-12);
        assert_eq!(back.directions, vec![Drive::Backward]);
        assert!(solve(
            3,
            &[mesh(0.0)],
            &[0.0, -43.0 / 17.0, 1.0],
            &Asked {
                input: 2,
                torque: 1.0,
                output: 1,
                reactions: vec![],
            },
        )
        .is_none());
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
        assert!(solve(
            5,
            &set(t, 0.98, 0.99, 3.0),
            &speeds(t, arrangement),
            &Asked {
                input: SUN,
                torque: 1.0,
                output: CARRIER,
                reactions: vec![RING, PLANET],
            },
        )
        .is_none());
    }
}
