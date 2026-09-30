//! A gear as the field reads it: its circles and its lead, in closed form for either kind.

use super::form::{FormRefused, TipCorner};

/// The plain inputs a [`FieldGear`] is built from until redesign G's generator gives the tooth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GearSpec {
    /// Tooth count, signed: negative for a ring.
    pub teeth: i32,
    /// Normal module, mm.
    pub module: f64,
    /// Normal pressure angle, degrees.
    pub pressure_angle: f64,
    /// Helix angle at the reference circle, degrees. Its sign is the hand.
    pub helix_angle: f64,
    /// Profile shift `x`, in modules.
    pub profile_shift: f64,
    /// Addendum `h_a`, in modules.
    pub addendum: f64,
}

/// A gear as the field reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldGear {
    /// `r_a`, mm.
    tip_radius: f64,
    /// `r_b`, mm.
    base_radius: f64,
    /// `τ = tan β / |r|`, the lead's twist: radians per mm along the axis.
    twist: f64,
}

impl FieldGear {
    /// The circles and the lead from `spec`, in closed form for either kind: the reference radius
    /// `r = z m_n / (2 cos β)` (negative for a ring), `r_b = |r| cos α_t` with
    /// `tan α_t = tan α_n / cos β`, `r_a = |r + m_n (h_a + x)|` and `τ = tan β / |r|`.
    ///
    /// `None` where these are not finite positive lengths and a finite twist: no gear (no teeth,
    /// or inputs that are not numbers).
    pub fn from_spec(spec: &GearSpec) -> Option<Self> {
        let normal = spec.pressure_angle.to_radians();
        let helix = spec.helix_angle.to_radians();
        let transverse = (normal.tan() / helix.cos()).atan();
        let reference = f64::from(spec.teeth) * spec.module / (2.0 * helix.cos());
        let gear = Self {
            tip_radius: (reference + spec.module * (spec.addendum + spec.profile_shift)).abs(),
            base_radius: reference.abs() * transverse.cos(),
            twist: helix.tan() / reference.abs(),
        };
        let lengths = [gear.tip_radius, gear.base_radius];
        (lengths.iter().all(|r| r.is_finite() && *r > 0.0) && gear.twist.is_finite())
            .then_some(gear)
    }

    /// `r_a`, mm.
    pub fn tip_radius(&self) -> f64 {
        self.tip_radius
    }

    /// `r_b`, mm.
    pub fn base_radius(&self) -> f64 {
        self.base_radius
    }

    /// `τ = tan β / |r|`, radians per mm along the axis.
    pub fn twist(&self) -> f64 {
        self.twist
    }

    /// The corner between the flank and the tip land ([`TipCorner::involute`]).
    pub fn tip_corner(&self) -> Result<TipCorner, FormRefused> {
        TipCorner::involute(self.tip_radius, self.base_radius, self.twist)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(teeth: i32) -> GearSpec {
        GearSpec {
            teeth,
            module: 1.0,
            pressure_angle: 20.0,
            helix_angle: 20.0,
            profile_shift: 0.0,
            addendum: 1.0,
        }
    }

    #[test]
    fn no_teeth_is_no_gear() {
        assert_eq!(FieldGear::from_spec(&spec(0)), None);
        assert_eq!(
            FieldGear::from_spec(&GearSpec {
                module: f64::NAN,
                ..spec(17)
            }),
            None
        );
        assert!(FieldGear::from_spec(&spec(17)).is_some());
        assert!(FieldGear::from_spec(&spec(-43)).is_some());
    }

    /// Each circle is held to a finite positive length on its own: an infinite module (every
    /// radius infinite, the twist a finite 0) and a ring whose tip circle is its axis (z −2 at
    /// β 0: `r + m h_a = 0`, every other input finite) are no gear.
    #[test]
    fn a_circle_that_is_no_length_is_no_gear() {
        let infinite = GearSpec {
            module: f64::INFINITY,
            ..spec(17)
        };
        let on_axis = GearSpec {
            helix_angle: 0.0,
            ..spec(-2)
        };
        for s in [infinite, on_axis] {
            assert_eq!(FieldGear::from_spec(&s), None, "{s:?}");
        }
        let near = GearSpec {
            helix_angle: 0.0,
            ..spec(-3)
        };
        assert!(FieldGear::from_spec(&near).is_some());
    }
}
