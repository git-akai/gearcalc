//! Internal (ring) gear geometry.
//!
//! A ring's tooth points **inward**: its tip circle is smaller than its pitch
//! circle and its root circle larger, which is the reverse of everything in
//! [`crate::profile`]. The flank is still an involute of the ring's own base
//! circle — the involute is self-conjugate, so it does not care which side the
//! material is on — but it is used the other way round, and that shows up as a
//! single sign.
//!
//! # The sign, and where it comes from
//!
//! An involute tooth's thickness follows
//!
//! ```text
//! s(r′) = r′ [ s/r + 2(inv α − inv α′) ]
//! ```
//!
//! which **narrows** outward, because `inv` grows with radius. A ring's tooth is
//! the complement of that: what narrows outward is the *space*, since the space
//! is where the mating pinion's tooth goes and a pinion tooth narrows toward its
//! own tip. So a ring's tooth **widens** outward and the sign flips:
//!
//! ```text
//! s_ring(r′) = r′ [ s/r + 2(inv α′ − inv α) ]
//! ```
//!
//! In the profile that is one character: [`Ring::involute_at`] adds
//! `inv_from_roll(u)` where [`crate::profile::Gear::involute_at`] subtracts it.
//! It is checked here as the complement it is, rather than asserted — tooth plus
//! space must come to the circular pitch at *every* radius, not only at the
//! pitch circle where it was set.
//!
//! # What is not here yet
//!
//! The fillet. It is swept by a pinion cutter and [`crate::shaper`] has the
//! curve, but placing it needs the cutter's own tooth geometry to say where its
//! tip corner sits — and that phase is what ties the fillet to the flank. Until
//! it is derived rather than guessed, this module offers the flank and the radii
//! and stops there.

use crate::involute::{inv, inv_from_roll};
use crate::params::GearParams;

/// A ring gear's cross-section, so far as the involute goes.
#[derive(Clone, Debug)]
pub struct Ring {
    pub teeth: u32,
    /// Transverse module, mm.
    pub mt: f64,
    /// Transverse pressure angle, radians.
    pub alpha_t: f64,
    /// Pitch radius, mm.
    pub r: f64,
    /// Base radius, mm. Smaller than the pitch radius, as ever.
    pub rb: f64,
    /// Tip radius, mm — **smaller** than the pitch radius.
    pub ra: f64,
    /// Root radius, mm — **larger** than the pitch radius.
    pub rf: f64,
    /// Half the tooth's angular thickness at the base circle, radians.
    pub psi_b: f64,
    /// Half the angular pitch, radians.
    pub half_pitch: f64,
    /// Roll parameter at the tip — the *lower* end of the flank here.
    pub u_tip: f64,
    /// Roll parameter at the root radius, where the flank would run out.
    pub u_root: f64,
    /// Guards that altered the geometry.
    pub clamps: Vec<String>,
}

impl Ring {
    /// Build a ring from the same parameters an external gear takes.
    ///
    /// `addendum` is measured **inward** and `dedendum` outward, which is what
    /// makes it a ring; the numbers themselves mean the same as they do for an
    /// external gear.
    #[must_use]
    pub fn new(params: &GearParams) -> Self {
        let mut clamps = Vec::new();
        let z = params.teeth.max(1);
        let beta = params.helix_angle.to_radians();
        let alpha_n = params.pressure_angle.to_radians();
        let m = params.module;
        let mt = m / beta.cos();
        let alpha_t = (alpha_n.tan() / beta.cos()).atan();

        let r = f64::from(z) * mt / 2.0;
        let rb = r * alpha_t.cos();
        let half_pitch = std::f64::consts::PI / f64::from(z);

        // Tooth thickness at the pitch circle. `thickness_mod` moves it exactly
        // as it does on an external gear — k = 1 is half the circular pitch —
        // and the space takes the remainder, because a ring's tooth and its
        // space are complements on the same circle.
        let tooth = std::f64::consts::PI * mt * params.thickness_mod / 2.0;
        let psi_p = tooth / (2.0 * r);
        // ...and the sign that makes it a ring: outward from the base circle the
        // tooth *gains* angle rather than losing it.
        let psi_b = psi_p - inv(alpha_t);

        let mut ra = r - m * params.addendum;
        let rf = r + m * params.dedendum;

        // The tip cannot dip below the base circle: there is no involute there
        // to cut it from.
        let ra_min = rb * (1.0 + 1e-9);
        if ra < ra_min {
            clamps.push(format!(
                "tip radius raised to the base circle at {ra_min:.4} mm: a ring's \
                 addendum cannot reach inside its own base circle"
            ));
            ra = ra_min;
        }

        let roll_at = |radius: f64| (((radius / rb).powi(2) - 1.0).max(0.0)).sqrt();
        Self {
            teeth: z,
            mt,
            alpha_t,
            r,
            rb,
            ra,
            rf,
            psi_b,
            half_pitch,
            u_tip: roll_at(ra),
            u_root: roll_at(rf),
            clamps,
        }
    }

    /// The involute flank at roll parameter `u`, as `(radius, angle from the
    /// tooth centreline)`.
    ///
    /// The **plus** is the whole difference from an external gear: a ring's
    /// tooth widens as it goes outward.
    #[must_use]
    pub fn involute_at(&self, u: f64) -> (f64, f64) {
        (self.rb * f64::hypot(1.0, u), self.psi_b + inv_from_roll(u))
    }

    /// Tooth thickness, as an arc length, at a radius on the flank.
    #[must_use]
    pub fn tooth_thickness_at(&self, radius: f64) -> f64 {
        let u = (((radius / self.rb).powi(2) - 1.0).max(0.0)).sqrt();
        2.0 * radius * (self.psi_b + inv_from_roll(u))
    }

    /// Space width, as an arc length, at a radius on the flank — what a mating
    /// pinion's tooth has to fit into.
    #[must_use]
    pub fn space_width_at(&self, radius: f64) -> f64 {
        2.0 * radius * self.half_pitch - self.tooth_thickness_at(radius)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::profile::Gear;

    fn ring(teeth: u32) -> Ring {
        Ring::new(&GearParams {
            teeth,
            ..Default::default()
        })
    }

    /// A ring's radii run the other way, and that is the whole of what makes it
    /// a ring.
    #[test]
    fn a_rings_tip_is_inside_its_pitch_circle_and_its_root_outside() {
        for teeth in [31u32, 43, 60, 120] {
            let g = ring(teeth);
            assert!(g.ra < g.r, "z={teeth}: tip {} against pitch {}", g.ra, g.r);
            assert!(g.rf > g.r, "z={teeth}: root {} against pitch {}", g.rf, g.r);
            assert!(g.rb < g.ra, "z={teeth}: the tip must clear the base circle");
            assert!(g.u_tip > 0.0 && g.u_root > g.u_tip);
        }
    }

    /// **The tooth and the space are complements at every radius, not just at
    /// the one where the thickness was set.** That is what the flipped sign
    /// buys, and getting it backwards would still look right at the pitch circle
    /// — which is exactly why the check sweeps the flank.
    #[test]
    fn tooth_and_space_come_to_the_circular_pitch_at_every_radius() {
        for teeth in [31u32, 43, 60] {
            let g = ring(teeth);
            for i in 0..=10 {
                #[allow(clippy::cast_precision_loss)]
                let t = i as f64 / 10.0;
                let radius = g.ra + (g.rf - g.ra) * t;
                let pitch = 2.0 * radius * g.half_pitch;
                let sum = g.tooth_thickness_at(radius) + g.space_width_at(radius);
                assert!(
                    (sum - pitch).abs() < 1e-12 * pitch,
                    "z={teeth} r={radius}: {sum} against a pitch of {pitch}"
                );
            }
        }
    }

    /// The direction the sign controls: a ring's tooth **widens** outward while
    /// its space narrows — the mirror of an external gear, whose tooth narrows.
    /// An external gear of the same size is measured alongside, so the claim is
    /// a comparison rather than an assertion about one curve.
    #[test]
    fn a_rings_tooth_widens_outward_where_an_external_gears_narrows() {
        let g = ring(43);
        let external = Gear::new(GearParams {
            teeth: 43,
            ..Default::default()
        });

        let (inner, outer) = (g.r * 0.99, g.r * 1.01);
        assert!(
            g.tooth_thickness_at(outer) > g.tooth_thickness_at(inner),
            "a ring's tooth must widen outward"
        );
        assert!(
            g.space_width_at(outer) < g.space_width_at(inner),
            "...and its space must narrow"
        );

        // The external gear, measured the same way, goes the other way.
        let ext_thickness = |radius: f64| {
            let u = (((radius / external.rb).powi(2) - 1.0).max(0.0)).sqrt();
            2.0 * radius * (external.psi_b - inv_from_roll(u))
        };
        assert!(
            ext_thickness(outer) < ext_thickness(inner),
            "an external gear's tooth narrows outward"
        );
    }

    /// A ring and an external gear of the same size share a base circle and a
    /// pitch circle: the involute is self-conjugate, so nothing about the curve
    /// itself changes.
    #[test]
    fn the_involute_itself_is_the_same_curve_as_an_external_gears() {
        for teeth in [31u32, 43] {
            let g = ring(teeth);
            let external = Gear::new(GearParams {
                teeth,
                ..Default::default()
            });
            assert!((g.rb - external.rb).abs() < 1e-12);
            assert!((g.r - external.r).abs() < 1e-12);
            assert!((g.alpha_t - external.alpha_t).abs() < 1e-15);
            // and a point at a given roll sits at the same radius
            for u in [0.1, 0.3, 0.5] {
                assert!((g.involute_at(u).0 - external.involute_at(u).0).abs() < 1e-12);
            }
        }
    }

    /// Thickness modification moves a ring's tooth the same way it moves an
    /// external gear's, and the space takes the remainder.
    #[test]
    fn thickness_modification_moves_the_tooth_and_the_space_together() {
        let thin = Ring::new(&GearParams {
            teeth: 43,
            thickness_mod: 0.8,
            ..Default::default()
        });
        let thick = Ring::new(&GearParams {
            teeth: 43,
            thickness_mod: 1.2,
            ..Default::default()
        });
        assert!(thick.tooth_thickness_at(thick.r) > thin.tooth_thickness_at(thin.r));
        assert!(thick.space_width_at(thick.r) < thin.space_width_at(thin.r));
        // k = 1 is exactly half the circular pitch, as on an external gear.
        let even = ring(43);
        let pitch = 2.0 * even.r * even.half_pitch;
        assert!((even.tooth_thickness_at(even.r) - pitch / 2.0).abs() < 1e-12);
    }

    /// A ring whose addendum would reach inside its own base circle is clamped
    /// and says so, rather than producing an involute that does not exist.
    #[test]
    fn an_addendum_reaching_past_the_base_circle_is_clamped_and_reported() {
        let g = Ring::new(&GearParams {
            teeth: 20,
            addendum: 3.0,
            ..Default::default()
        });
        assert!(g.ra >= g.rb);
        assert!(
            g.clamps.iter().any(|c| c.contains("base circle")),
            "clamps: {:?}",
            g.clamps
        );
    }
}
