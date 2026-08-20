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
//! # What is not here
//!
//! **Radial assembly** — whether a pinion can be brought in sideways past the
//! ring's teeth. It is a swept-motion question rather than a comparison of tip
//! circles, and §4.11 records what happened to the attempt that treated it as
//! one. It belongs with the planetary set that actually asks.
//!
//! **A bending rating.** A ring's tooth widens outward and its fillet is
//! shaper-cut, so neither the critical section nor the notch input is the one
//! [`crate::strength`] measures on an external tooth. NASA TM-107012's inscribed
//! parabola is the model, and it needs the cutter placed exactly — which is what
//! the shifted cut above now provides.

use crate::involute::{inv, inv_from_roll};
use crate::mesh::{operating_geometry, MeshKind};
use crate::params::{guard, GearParams};
use crate::profile::Gear;
use crate::profile::Section;
use crate::shaper::{CutParams, ShaperCut};
use crate::solve::{brent, Tol};

/// The pinion cutter a ring is shaped with.
///
/// A ring has no meaningful geometry without one: unlike a rack-cut external
/// gear, where the tool is implied by the basic rack, the fillet a ring gets
/// depends on how many teeth its cutter had. Two rings with identical teeth,
/// module and depth are *different parts* if they were shaped differently.
#[derive(Clone, Copy, Debug)]
pub struct Cutter {
    pub teeth: u32,
    /// Addendum, in modules — how far past its pitch circle the tool reaches.
    pub addendum: f64,
    /// Tip corner round, in modules.
    pub tip_round: f64,
}

impl Default for Cutter {
    fn default() -> Self {
        Self {
            teeth: 20,
            addendum: 1.25,
            // Small, because a shaper cutter's tip is narrow: at 20 teeth and a
            // 1.25 addendum the tip is 0.38 modules wide, so two 0.38 rounds
            // cannot both live on it. 0.38 is the *rack's* figure and does not
            // carry over.
            tip_round: 0.2,
        }
    }
}

/// A ring gear's cross-section, so far as the involute goes.
#[derive(Clone, Debug)]
pub struct Ring {
    /// The inputs this was built from, kept as [`crate::Gear`] keeps its own —
    /// so a ring can produce its virtual spur section without being handed back
    /// what it was made of.
    pub params: GearParams,
    /// ...and the tool, because the tool is part of the part (§4.11).
    pub cutter: Cutter,
    /// Tooth count, **rounded**. A virtual spur ring has a fractional one; the
    /// geometry carries the exact value through `r` and `half_pitch`, and this
    /// field is for replication and display.
    pub teeth: u32,
    /// Transverse module, mm.
    pub mt: f64,
    /// Transverse pressure angle, radians.
    pub alpha_t: f64,
    /// Normal pressure angle, radians. Equal to `alpha_t` for a spur ring.
    pub alpha_n: f64,
    /// The thickness shift `x + x_s`, acting on the **space**.
    ///
    /// One number rather than the shift and the thickness modification
    /// separately, because only their sum ever reaches an answer — the same
    /// reason `Mesh` sums them (§4.1). Positive widens the space and thins the
    /// tooth; see [`Ring::new`].
    pub x_thick: f64,
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
    /// Roll parameter where the flank hands over to the fillet.
    pub u_j: f64,
    /// Cutter travel at that same junction.
    pub s_j: f64,
    /// Travel at which the fillet ends.
    ///
    /// Zero when a root arc follows — the deepest cut, at mid-space. Non-zero
    /// when the fillets from the two flanks meet before they get there, which
    /// leaves a **fully filleted root** with no flat at all. Common, and not a
    /// fault: it simply means the cutter's tip is wide enough that its corner
    /// rounds overlap.
    pub s_root: f64,
    /// The cut that made this ring.
    pub cut: ShaperCut,
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
    pub fn new(params: &GearParams, cutter: &Cutter) -> Self {
        Self::new_with_z(
            params,
            cutter,
            f64::from(params.teeth.max(1)),
            f64::from(cutter.teeth.max(1)),
        )
    }

    /// The same, at an arbitrary — possibly fractional — tooth count for the ring
    /// and its cutter.
    ///
    /// Exists for the **virtual spur ring**: rating a helical ring's bending means
    /// working on its normal section, where both members carry `z / cos³β` teeth
    /// and neither is a whole number. The same reason
    /// [`crate::Gear`] builds its virtual gear from a non-integer `z`.
    #[must_use]
    pub fn new_with_z(params: &GearParams, cutter: &Cutter, z: f64, cutter_teeth: f64) -> Self {
        let mut clamps = Vec::new();
        let beta = params.helix_angle.to_radians();
        let alpha_n = params.pressure_angle.to_radians();
        let m = params.module;
        let mt = m / beta.cos();
        let alpha_t = (alpha_n.tan() / beta.cos()).atan();

        let r = z * mt / 2.0;
        let rb = r * alpha_t.cos();
        let half_pitch = std::f64::consts::PI / z;

        // ---- thickness. It is the SPACE that takes the external formula.
        //
        // A ring's space is where the mating pinion's tooth goes, and it is
        // generated the way a pinion's tooth is; the ring's tooth is whatever
        // the pitch leaves over. So `thickness_mod` and the profile shift are
        // applied to the space, using `Gear`'s expression unchanged — and the
        // consequence is that a *larger* k or x makes a ring's tooth **thinner**,
        // the opposite of an external gear.
        //
        // This is not a convention free to choose. `Mesh::new`'s internal
        // relation flips gear 2's `x` and `x_s` together, and that is consistent
        // only with this reading: measured against tooth thicknesses at the
        // operating circles, the space reading gives exactly zero backlash at
        // every k while the tooth reading is out by 0.63 mm at k = 1.2 (§12).
        // The pair invariant is therefore `k₁ = k₂` for an internal mesh, where
        // an external one needs `k₁ + k₂ = 2`.
        let x = params.profile_shift;
        let x_thick = x + params.thickness_shift();
        let pitch = std::f64::consts::PI * mt;
        let mut space = mt * (std::f64::consts::PI / 2.0 + 2.0 * x_thick * alpha_n.tan());
        // Both ends are real limits: a space wider than the pitch leaves no
        // tooth, and one of zero width leaves no space for the pinion.
        // The same two limits `Gear` puts on a tooth, applied to the space —
        // because on a ring the space is the thing generated like a tooth.
        let space_max = guard::MAX_TOOTH_THICKNESS_FRACTION_OF_PITCH * pitch;
        let space_min = guard::MIN_TOOTH_THICKNESS_MODULES * m;
        if space > space_max {
            space = space_max;
            clamps.push(
                "space width capped: profile shift or thickness modification too \
                 positive, and a ring needs a tooth left over"
                    .to_string(),
            );
        } else if space < space_min {
            space = space_min;
            clamps.push(
                "space width raised: profile shift or thickness modification too \
                 negative, and the mating pinion's tooth has to fit"
                    .to_string(),
            );
        }
        let tooth = pitch - space;
        let psi_p = tooth / (2.0 * r);
        // ...and the sign that makes it a ring: outward from the base circle the
        // tooth *gains* angle rather than losing it.
        let psi_b = psi_p - inv(alpha_t);

        // ---- radii. The whole form shifts **outward** by `x m`, which shortens
        // a ring's tooth (it points inward) and deepens its space. Written as
        // `Gear`'s two expressions with inward and outward exchanged.
        let mut ra = r - m * (params.addendum - x);

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

        // ---- where the cutter sits.
        //
        // A shifted ring is cut by the *same* tool placed further out, and that
        // displacement is what its shift means for a shaper. The distance is the
        // internal relation between tool and workpiece, read through the shared
        // `operating_geometry`: the cutter is member 1 (external, unshifted) and
        // the ring is member 2, so the signed sums are `z_c − z_r` and `−x_thick`.
        let cutter_radius = cutter_teeth * mt / 2.0;
        // A standard cutter: `k = 1`, no shift of its own. A resharpened tool
        // carries one, and it would enter the sums below exactly as the ring's
        // does; nothing here assumes it is zero beyond this line.
        let cutter_tooth = std::f64::consts::PI * mt / 2.0;
        let sum_z = cutter_teeth - z;
        // Falls back to reference centres only when the shift takes the pair out
        // of the involute domain, which `ShaperCut::new` then refuses anyway.
        let a_cut = operating_geometry(mt, alpha_t, alpha_n, sum_z, -x_thick)
            .map_or(r - cutter_radius, |(_, _, a)| a);
        // ---- the root circle is where the cutter's tip reaches, not an input.
        //
        // `a_cut + r_tip` exactly, rather than `r + m(dedendum + x)`, which is
        // that expression linearised: the two differ by 17 µm at x = 0.25 and
        // 57 µm at x = 0.5, both well above the 3.6 µm the cut simulation
        // resolves. A ring's dedendum is therefore **not** an input — it is the
        // cutter's addendum seen from the other side, and having both invites
        // them to disagree.
        let cutter_tip_radius = cutter_radius + m * cutter.addendum;
        let rf = a_cut + cutter_tip_radius;

        let cut = ShaperCut::new(&CutParams {
            module_t: mt,
            alpha_t,
            workpiece_radius: r,
            workpiece_tooth: tooth,
            cutter_tooth,
            centre_distance: a_cut,
            cutter_radius,
            cutter_tip_radius,
            tip_round: m * cutter.tip_round,
            kind: MeshKind::Internal,
        });

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let teeth = z.round().max(1.0) as u32;
        let mut ring = Self {
            params: *params,
            cutter: *cutter,
            teeth,
            mt,
            alpha_t,
            alpha_n,
            x_thick,
            r,
            rb,
            ra,
            rf,
            psi_b,
            half_pitch,
            u_tip: roll_at(ra),
            u_root: roll_at(rf),
            u_j: roll_at(rf),
            s_j: 0.0,
            s_root: 0.0,
            cut: cut.unwrap_or(ShaperCut {
                workpiece_radius: r,
                cutter_radius,
                cutter_tooth,
                alpha_w: alpha_t,
                centre_distance: r - cutter_radius,
                workpiece_operating_radius: r,
                cutter_operating_radius: cutter_radius,
                corner_radius: cutter_radius,
                tip_round: 0.0,
                phase: 0.0,
                kind: MeshKind::Internal,
            }),
            clamps,
        };
        if cut.is_none() {
            ring.clamps
                .push("the cutter has no usable tip corner; no fillet generated".into());
            return ring;
        }
        match ring.solve_junction() {
            Some((u_j, s_j)) => {
                ring.u_j = u_j;
                ring.s_j = s_j;
                ring.s_root = ring.solve_root_end();
                if !ring.fully_generated() {
                    let limit = ring.generation_limit();
                    ring.clamps.push(format!(
                        "the flank below {limit:.4} mm is not generated by this cutter — \
                         its own involute runs out there, so the tip {:.4} mm is inside \
                         the generated region. A cutter with more teeth reaches further",
                        ring.ra
                    ));
                }
                if ring.s_root != 0.0 {
                    ring.clamps.push(
                        "fully filleted root: the corner rounds meet before mid-space, so \
                         there is no root arc"
                            .into(),
                    );
                }
            }
            None => ring.clamps.push(
                "flank and fillet do not meet: the cutter does not reach this ring's \
                 flank, so the profile is not generated"
                    .into(),
            ),
        }
        ring
    }

    /// Where the involute flank hands over to the shaper's trochoid.
    ///
    /// # The two curves *touch*, they do not cross
    ///
    /// The first attempt looked for a sign change and found none, because there
    /// is none: the cutter's flank ends exactly where its tip round begins, so
    /// the flank it generates ends exactly where the fillet begins and the two
    /// meet **tangentially**. Measured on a 43-tooth ring the residual bottoms
    /// out at 1e-6 rad and never changes sign. A bracketed solver was the wrong
    /// tool, and its failing was the useful signal.
    ///
    /// # So it is solved in closed form, from the line of action
    ///
    /// The last workpiece flank point the cutter's *flank* can generate is the
    /// one conjugate to where that flank ends. Conjugate points share a position
    /// on the line of action, and each member's distance from the pitch point
    /// along it is `√(r² − r_b²)`. For an internal pair the ring's tangency
    /// point lies beyond the cutter's, so the two distances differ by
    /// `a sin α_t` rather than summing to it:
    ///
    /// ```text
    /// √(r_j² − r_bw²) = a sin α_t + √(r_tan² − r_bc²)
    /// ```
    ///
    /// and `r_tan` — where the cutter's round meets its flank — comes from the
    /// same offset-involute fact the phase used: the round's centre sits at roll
    /// `t_g`, so the tangency is at roll `t_g + ρ/r_bc` on the flank itself.
    /// Nothing here iterates.
    ///
    /// The one solve left is turning that radius back into a cutter travel, and
    /// the trochoid's radius is monotone either side of the deepest cut.
    fn solve_junction(&self) -> Option<(f64, f64)> {
        let r_bc = self.cut.cutter_radius * self.alpha_t.cos();
        let t_g = (((self.cut.corner_radius / r_bc).powi(2) - 1.0).max(0.0)).sqrt();
        let t_tan = t_g + self.cut.tip_round / r_bc;

        let along = self.cut.centre_distance * self.cut.alpha_w.sin() + r_bc * t_tan;
        let r_j = f64::hypot(self.rb, along);
        if !(r_j.is_finite() && r_j > self.ra && r_j < self.rf) {
            return None;
        }

        // ...and back to a travel. Monotone in `s` away from the deepest cut,
        // which is what makes one bracketed step enough.
        let radius_at = |s: f64| self.cut.trochoid_at(s).0 - r_j;
        let mut far = -self.mt;
        for _ in 0..64 {
            if radius_at(far) < 0.0 {
                let s_j = brent(radius_at, far, 0.0, Tol::default())?;
                return Some((self.roll_at(r_j), s_j));
            }
            far *= 1.4;
        }
        None
    }

    /// Where the fillet ends: the deepest cut, or mid-space if it gets there
    /// first.
    ///
    /// The fillet is symmetric about mid-space, so two of them meeting there is
    /// the same statement as one of them reaching it. Monotone in `s`, so one
    /// bracketed step again.
    fn solve_root_end(&self) -> f64 {
        if self.trochoid_at(0.0).1 <= self.half_pitch {
            return 0.0;
        }
        let over = |s: f64| self.trochoid_at(s).1 - self.half_pitch;
        brent(over, self.s_j, 0.0, Tol::default()).unwrap_or(0.0)
    }

    /// The smallest radius at which this ring's flank is a **generated**
    /// involute, mm.
    ///
    /// A cutter's flank stops at its own base circle, and conjugate points share
    /// a position on the line of action, so the deepest the cutter's involute
    /// can reach on the ring is where its own contribution runs out:
    ///
    /// ```text
    /// √(r_limit² − r_b²) = a sin α_t          (the cutter's term is zero)
    /// ```
    ///
    /// Below this the ring's flank is not cut by an involute at all — the
    /// cutter's fillet region passes there instead. It is the internal gear's
    /// analogue of undercut, and like undercut it is a property of the *pair*:
    /// the same ring cut by a bigger cutter has a smaller limit, because the
    /// centre distance shrinks.
    ///
    /// Reported rather than silently accepted. [`Ring::fully_generated`] says
    /// whether it bites.
    #[must_use]
    pub fn generation_limit(&self) -> f64 {
        f64::hypot(self.rb, self.cut.centre_distance * self.cut.alpha_w.sin())
    }

    /// Whether the tip reaches down only as far as the cutter can generate.
    #[must_use]
    pub fn fully_generated(&self) -> bool {
        self.ra >= self.generation_limit()
    }

    /// The involute's roll parameter at a radius. Closed form.
    fn roll_at(&self, radius: f64) -> f64 {
        (((radius / self.rb).powi(2) - 1.0).max(0.0)).sqrt()
    }

    /// The fillet at cutter travel `s`, as `(radius, angle)`.
    #[must_use]
    pub fn trochoid_at(&self, s: f64) -> (f64, f64) {
        self.cut.trochoid_at(s)
    }

    /// The **virtual spur ring**: this ring's normal section, as a spur ring.
    ///
    /// Bending is rated on the normal section, so a helical ring has to be rated
    /// on this rather than on its transverse form — measuring `Y_F` transversely
    /// and dividing by `m_n` mixes planes and under-predicts by about `cos β`,
    /// the error `docs/DESIGN.md` §12 records for external gears.
    ///
    /// The construction is ISO's, the same one [`crate::Gear::virtual_spur`]
    /// uses: `z_n = z / cos³β` at the normal module and normal pressure angle.
    /// **The cutter is virtualised the same way**, because a ring's form is its
    /// tool's — and scaling both by the same factor leaves `z_c/z_r` unchanged, so
    /// the virtual pair still rolls together and the cut stays conjugate.
    ///
    /// At `β = 0` this rebuilds the ring it was called on, by construction rather
    /// than by a branch.
    #[must_use]
    pub fn virtual_spur(&self) -> Self {
        let beta = self.params.helix_angle.to_radians();
        let scale = beta.cos().powi(3);
        let params = GearParams {
            helix_angle: 0.0,
            ..self.params
        };
        Self::new_with_z(
            &params,
            &self.cutter,
            f64::from(self.params.teeth.max(1)) / scale,
            f64::from(self.cutter.teeth.max(1)) / scale,
        )
    }

    /// Base helix angle, radians — `sin β_b = sin β cos α_n`.
    ///
    /// The same relation [`crate::metrology::base_helix_angle`] gives an external
    /// gear; it is a property of the reference rack, not of which side the
    /// material is on.
    #[must_use]
    pub fn base_helix_angle(&self) -> f64 {
        let beta = self.params.helix_angle.to_radians();
        (beta.sin() * self.alpha_n.cos()).asin()
    }

    /// The fillet point and its tangent, Cartesian, tooth centred on `+y`.
    ///
    /// Straight through to [`ShaperCut::trochoid_point_and_tangent`], which
    /// differentiates the construction analytically. Named here so a ring's
    /// bending model reads its own geometry off the ring rather than reaching
    /// into the cut.
    #[must_use]
    pub fn fillet_point_and_tangent(&self, s: f64) -> ([f64; 2], [f64; 2]) {
        self.cut.trochoid_point_and_tangent(s)
    }

    /// The flank point and its tangent, Cartesian, tooth centred on `+y`.
    ///
    /// The same involute derivative an external gear's flank has, with the one
    /// sign that makes it a ring: `dθ/du` is **positive** here, because a ring's
    /// tooth gains angle outward where an external gear's loses it.
    #[must_use]
    pub fn flank_point_and_tangent(&self, u: f64) -> ([f64; 2], [f64; 2]) {
        let root = f64::hypot(1.0, u);
        let r = self.rb * root;
        let th = self.psi_b + inv_from_roll(u);
        let (st, ct) = th.sin_cos();

        let dr = self.rb * u / root;
        // Positive, where `Gear`'s is negative — the flipped `inv` term of the
        // module documentation, differentiated.
        let dth = (u * u) / (1.0 + u * u);
        (
            [r * st, r * ct],
            [dr * st + r * ct * dth, dr * ct - r * st * dth],
        )
    }

    /// The flank point and the direction of the load there.
    ///
    /// The load acts along the involute normal, which is the line from the
    /// contact point to the base-circle tangency point. For a ring that tangency
    /// sits `roll` radians **forward** around the base circle rather than back,
    /// which is the same sign as above.
    #[must_use]
    pub fn flank_point_and_load_direction(&self, roll: f64) -> ([f64; 2], [f64; 2]) {
        let (r, th) = self.involute_at(roll);
        let p = [r * th.sin(), r * th.cos()];
        let tangent_angle = self.psi_b + roll;
        let t = [self.rb * tangent_angle.sin(), self.rb * tangent_angle.cos()];
        let (dx, dy) = (p[0] - t[0], p[1] - t[1]);
        let len = f64::hypot(dx, dy);
        if len < f64::MIN_POSITIVE {
            return (p, [1.0, 0.0]);
        }
        (p, [dx / len, dy / len])
    }

    /// Radius of curvature of the fillet at travel `s`, mm.
    ///
    /// A central difference on the **analytic** first derivative, exactly as the
    /// rack-cut case does it and for the same reason: this feeds the empirical
    /// notch factor rather than locating the section, so a difference is
    /// proportionate here where it would not be for the tangent.
    #[must_use]
    pub fn fillet_curvature_radius(&self, s: f64) -> f64 {
        let h = 1e-6 * self.mt.max(1e-9);
        let (_, t0) = self.fillet_point_and_tangent(s - h);
        let (_, t1) = self.fillet_point_and_tangent(s + h);
        let (_, t) = self.fillet_point_and_tangent(s);
        let ddx = (t1[0] - t0[0]) / (2.0 * h);
        let ddy = (t1[1] - t0[1]) / (2.0 * h);
        let speed = f64::hypot(t[0], t[1]);
        let cross = (t[0] * ddy - t[1] * ddx).abs();
        if cross < f64::MIN_POSITIVE {
            f64::INFINITY
        } else {
            speed.powi(3) / cross
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
    // ---------------------------------------------------------------- //
    //  assembly
    // ---------------------------------------------------------------- //

    /// The half-profile sections, ordered tip → mid tooth-space.
    ///
    /// The same four an external gear has, in the same order — but the radius
    /// *climbs* through them rather than falling, because a ring's tooth points
    /// inward. A fully filleted root drops the last one.
    #[must_use]
    pub fn sections(&self) -> Vec<Section> {
        let mut out = vec![Section::TipArc, Section::Involute, Section::Trochoid];
        if self.trochoid_at(self.s_root).1 < self.half_pitch {
            out.push(Section::RootArc);
        }
        out
    }

    fn sample_section(&self, section: Section, n: usize) -> Vec<(f64, f64)> {
        let n = n.max(2);
        #[allow(clippy::cast_precision_loss)]
        let lerp = |a: f64, b: f64, i: usize| a + (b - a) * (i as f64 / (n - 1) as f64);
        (0..n)
            .map(|i| match section {
                Section::TipArc => (self.ra, lerp(0.0, self.involute_at(self.u_tip).1, i)),
                Section::Involute => self.involute_at(lerp(self.u_tip, self.u_j, i)),
                Section::Trochoid => self.trochoid_at(lerp(self.s_j, self.s_root, i)),
                Section::RootArc => (
                    self.rf,
                    lerp(self.trochoid_at(self.s_root).1, self.half_pitch, i),
                ),
            })
            .collect()
    }

    /// `(radius, angle)` from the tooth tip centre to mid tooth-space, spaced by
    /// arc length so no section is starved of points.
    #[must_use]
    pub fn half_profile(&self, n: usize) -> Vec<(f64, f64)> {
        const LENGTH_SAMPLES: usize = 60;
        const MIN_SHARE: f64 = 0.004;
        const MIN_POINTS: usize = 3;

        let sections = self.sections();
        let lengths: Vec<f64> = sections
            .iter()
            .map(|&s| {
                let pts = self.sample_section(s, LENGTH_SAMPLES);
                (1..pts.len())
                    .map(|i| {
                        let dr = pts[i].0 - pts[i - 1].0;
                        let dt = (pts[i].0 + pts[i - 1].0) / 2.0 * (pts[i].1 - pts[i - 1].1);
                        f64::hypot(dr, dt)
                    })
                    .sum()
            })
            .collect();
        let total: f64 = lengths.iter().sum();
        let shares: Vec<f64> = lengths.iter().map(|w| w.max(total * MIN_SHARE)).collect();
        let share_total: f64 = shares.iter().sum();

        let mut out: Vec<(f64, f64)> = Vec::new();
        for (&section, share) in sections.iter().zip(shares) {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let count = ((share / share_total) * n as f64) as usize;
            let pts = self.sample_section(section, count.max(MIN_POINTS));
            let skip = usize::from(!out.is_empty());
            out.extend_from_slice(&pts[skip..]);
        }
        out
    }

    /// The closed cross-section, counter-clockwise, `per_tooth` points a tooth.
    ///
    /// The outline of the *material's inner boundary*: a ring's teeth point
    /// inward, so this traces the bore, and whatever rim sits outside it is the
    /// designer's business rather than the tooth geometry's.
    #[must_use]
    pub fn profile(&self, per_tooth: usize) -> Vec<[f64; 2]> {
        let half = self.half_profile((per_tooth / 2).max(8));

        let mut full: Vec<(f64, f64)> = half.iter().rev().map(|&(r, t)| (r, -t)).collect();
        full.extend_from_slice(&half[1..]);

        let z = self.teeth;
        let mut out = Vec::with_capacity(full.len() * z as usize + 1);
        for k in 0..z {
            let base = 2.0 * std::f64::consts::PI * f64::from(k) / f64::from(z);
            for &(r, t) in &full {
                let a = base + t;
                out.push([r * a.cos(), r * a.sin()]);
            }
        }
        if let Some(&first) = out.first() {
            out.push(first);
        }
        out
    }
}

// -------------------------------------------------------------------------- //
//  meshing a ring with a pinion
// -------------------------------------------------------------------------- //

/// What an internal mesh does, and two of the ways it can foul.
///
/// **Radial assembly is not here**, deliberately. Whether the pinion can be
/// brought in sideways rather than axially is a *swept-motion* question — the
/// teeth have to pass each other on the way in — not a comparison of tip
/// circles, and a first attempt at it as one produced a figure that was negative
/// for every meshing pair, which is the signature of a formula that means
/// nothing. It needs its own derivation and belongs with the planetary stage
/// that will actually ask.
///
/// # The one relation everything here comes from
///
/// Conjugate points share a place on the line of action, and each member's
/// distance from the pitch point along it is `√(r² − r_b²)`. For an **internal**
/// pair the ring's tangency point lies beyond the pinion's, so the two distances
/// differ by `a sin α_w` rather than summing to it:
///
/// ```text
/// √(r_ring² − r_b2²) = a sin α_w + √(r_pinion² − r_b1²)
/// ```
///
/// Read it forwards and it maps a pinion radius to the ring radius it touches;
/// read it backwards and it does the reverse. Every check below is one of those
/// two readings, asked at a tip.
#[derive(Clone, Copy, Debug)]
pub struct RingMesh {
    /// Zero-backlash centre distance, mm — `r_ring − r_pinion` for a standard
    /// pair, larger when the ring is shifted further than its pinion.
    pub centre_distance: f64,
    /// Operating pressure angle, radians.
    pub alpha_w: f64,
    /// Transverse contact ratio.
    pub contact_ratio: f64,
    /// The ring radius the pinion's **tip** touches, mm. It is the deepest point
    /// of the mesh, and it must stay clear of the ring's fillet.
    pub ring_contact_at_pinion_tip: f64,
    /// The pinion radius the ring's **tip** touches, mm — the shallowest point,
    /// which must stay above whatever the pinion's own flank runs out at.
    ///
    /// Reported as the pinion's base radius when the tip cannot reach the
    /// involute at all; [`Self::involute_interference`] is then set.
    pub pinion_contact_at_ring_tip: f64,
    /// The pinion's tip reaches past where the ring's flank ends and into its
    /// fillet.
    pub trochoid_interference: bool,
    /// The ring's tip reaches below where the pinion's flank ends.
    pub involute_interference: bool,
}

/// Mesh a ring with an external pinion at their zero-backlash centre distance.
///
/// Shifts on either member are carried, through the same
/// [`operating_geometry`](crate::mesh::operating_geometry) the external mesh
/// uses: the pinion is member 1 and the ring member 2, so the sums are
/// `z_p − z_r` and `x_p − x_r`. A standard pair is the value of that at zero,
/// where `α_w = α_t` and `a = r_ring − r_pinion` — not a separate case.
///
/// **The shifts enter through the space, not the tooth.** A ring's `x_thick`
/// widens its space, so a ring shifted further than its pinion opens the mesh
/// out and the pinion sits further from the ring's axis. That is why the sum is
/// a difference and why it is this way round; see [`Ring::new`].
///
/// # Errors
///
/// `None` if the pair cannot mesh — different modules or pressure angles, a
/// pinion no smaller than the ring, a geometry that never reaches contact, or
/// shifts that drive the operating pressure angle out of the involute domain.
#[must_use]
pub fn mesh_with(ring: &Ring, pinion: &Gear) -> Option<RingMesh> {
    if (ring.mt - pinion.mt).abs() > 1e-9 || (ring.alpha_t - pinion.alpha_t).abs() > 1e-9 {
        return None;
    }
    if pinion.params.teeth >= ring.teeth {
        return None;
    }
    let sum_z = f64::from(pinion.params.teeth) - f64::from(ring.teeth);
    let sum_x = (pinion.params.profile_shift + pinion.params.thickness_shift()) - ring.x_thick;
    let (alpha_w, _, centre_distance) =
        crate::mesh::operating_geometry(ring.mt, ring.alpha_t, ring.alpha_n, sum_z, sum_x)?;
    if centre_distance.is_nan() || centre_distance <= 0.0 {
        return None;
    }
    let along = centre_distance * alpha_w.sin();

    // The relation, both ways round.
    let ring_at = |r_pinion: f64| {
        let t = (r_pinion * r_pinion - pinion.rb * pinion.rb)
            .max(0.0)
            .sqrt();
        f64::hypot(ring.rb, along + t)
    };
    // Read backwards this can fail, and the failure is the answer rather than an
    // error: a negative distance means the ring's tip would have to touch the
    // pinion *inside its base circle*, where no involute exists. That is
    // involute interference in its strongest form, and it is reported as a
    // finding, not as "this pair cannot be described".
    let pinion_at = |r_ring: f64| {
        let t = (r_ring * r_ring - ring.rb * ring.rb).max(0.0).sqrt() - along;
        (t >= 0.0).then(|| f64::hypot(pinion.rb, t))
    };

    let ring_contact_at_pinion_tip = ring_at(pinion.ra);
    let reachable = pinion_at(ring.ra);
    let pinion_contact_at_ring_tip = reachable.unwrap_or(pinion.rb);

    // Contact ratio, from DESIGN §4.5's internal form. The path runs from where
    // the ring's tip engages to where the pinion's does.
    let base_pitch = std::f64::consts::PI * ring.mt * ring.alpha_t.cos();
    let path = ((pinion.ra * pinion.ra - pinion.rb * pinion.rb)
        .max(0.0)
        .sqrt()
        - (ring.ra * ring.ra - ring.rb * ring.rb).max(0.0).sqrt()
        + along)
        .max(0.0);
    let contact_ratio = path / base_pitch;

    // The ring's flank ends where its fillet begins; the pinion's ends where
    // its own does. A tip reaching past either is the foul.
    let ring_form = ring.involute_at(ring.u_j).0;
    let trochoid_interference = ring_contact_at_pinion_tip > ring_form;
    let involute_interference = reachable.is_none() || pinion_contact_at_ring_tip < pinion.r_j;

    Some(RingMesh {
        centre_distance,
        alpha_w,
        contact_ratio,
        ring_contact_at_pinion_tip,
        pinion_contact_at_ring_tip,
        trochoid_interference,
        involute_interference,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::profile::Gear;

    fn ring(teeth: u32) -> Ring {
        Ring::new(
            &GearParams {
                teeth,
                ..Default::default()
            },
            &Cutter::default(),
        )
    }

    /// **The smallest ring is a function of the design, not a number.**
    ///
    /// A ring's tip sits at `r − h_a` and its base circle at `r cos α_t`, so the
    /// tip clears the base circle only while
    ///
    /// ```text
    /// z > 2 h_a cos β / (1 − cos α_t)
    /// ```
    ///
    /// Three things move it and one does not. A **shallower tooth** allows far
    /// fewer teeth; a **larger pressure angle** allows fewer, because the base
    /// circle drops away from the pitch circle; a **helix** allows fewer, since
    /// the transverse module grows with it. The **module cancels**, which is
    /// right — this is a statement about tooth counts.
    ///
    /// The familiar "internal gears need at least about 34 teeth" is the single
    /// row of this table at a full addendum and 20°, and quoting it as a rule
    /// would have been wrong for every other row.
    #[test]
    fn the_smallest_ring_follows_the_design_rather_than_a_rule_of_thumb() {
        let cases = [
            // addendum, α_n°, β°, the count the geometry gives
            (1.0, 20.0, 0.0, 34u32),
            (0.8, 20.0, 0.0, 27),
            (0.6, 20.0, 0.0, 20),
            (1.0, 25.0, 0.0, 22),
            (1.0, 14.5, 0.0, 63),
            (1.0, 20.0, 30.0, 23),
        ];
        for (addendum, alpha_deg, beta_deg, expected) in cases {
            let beta = f64::to_radians(beta_deg);
            let alpha_t = (f64::to_radians(alpha_deg).tan() / beta.cos()).atan();
            let threshold = 2.0 * addendum * beta.cos() / (1.0 - alpha_t.cos());
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let smallest = threshold.ceil() as u32;
            assert_eq!(
                smallest, expected,
                "a={addendum} α={alpha_deg} β={beta_deg}: the formula gives {threshold}"
            );

            let build = |teeth: u32| {
                Ring::new(
                    &GearParams {
                        teeth,
                        addendum,
                        pressure_angle: alpha_deg,
                        helix_angle: beta_deg,
                        ..Default::default()
                    },
                    &Cutter::default(),
                )
            };
            let clamped = |r: &Ring| r.clamps.iter().any(|c| c.contains("base circle"));
            assert!(
                !clamped(&build(smallest)),
                "a={addendum} α={alpha_deg} β={beta_deg}: z={smallest} should fit"
            );
            assert!(
                clamped(&build(smallest - 1)),
                "a={addendum} α={alpha_deg} β={beta_deg}: z={} should not",
                smallest - 1
            );
        }
    }

    /// **The flank and the fillet actually meet.** That is what the phase buys:
    /// with it wrong the two curves are the right shapes in the wrong places,
    /// and the profile has a step in it. Continuity in both radius and angle,
    /// to the solver's own tolerance.
    #[test]
    fn the_flank_and_the_fillet_meet_at_the_junction() {
        for teeth in [43u32, 60, 90, 120] {
            let g = ring(teeth);
            assert!(
                g.clamps
                    .iter()
                    .all(|c| c.contains("fully filleted") || c.contains("not generated")),
                "z={teeth} should generate cleanly: {:?}",
                g.clamps
            );
            let (r_flank, a_flank) = g.involute_at(g.u_j);
            let (r_fillet, a_fillet) = g.trochoid_at(g.s_j);
            assert!(
                (r_flank - r_fillet).abs() < 1e-9,
                "z={teeth}: radius {r_flank} against {r_fillet}"
            );
            assert!(
                (a_flank - a_fillet).abs() < 1e-9,
                "z={teeth}: angle {a_flank} against {a_fillet}"
            );
            // ...and the junction sits between the tip and the root, which is
            // what makes it a junction rather than a coincidence off the part.
            assert!(
                r_flank > g.ra && r_flank < g.rf,
                "z={teeth}: junction at {r_flank}, outside ({}, {})",
                g.ra,
                g.rf
            );
        }
    }

    /// The half-profile runs outward the whole way: tip, flank, fillet, root.
    /// A ring that doubled back on itself would still pass the junction test.
    #[test]
    fn the_profile_climbs_from_tip_to_root_without_turning_back() {
        let g = ring(43);
        let mut radius = g.ra;
        let mut angle = 0.0_f64;
        for i in 0..=40 {
            #[allow(clippy::cast_precision_loss)]
            let t = i as f64 / 40.0;
            let (r, a) = g.involute_at(g.u_tip + (g.u_j - g.u_tip) * t);
            assert!(
                r >= radius - 1e-12,
                "flank turned back at {r} from {radius}"
            );
            assert!(a >= angle - 1e-12, "flank angle turned back");
            radius = r;
            angle = a;
        }
        for i in 0..=40 {
            #[allow(clippy::cast_precision_loss)]
            let t = i as f64 / 40.0;
            let (r, a) = g.trochoid_at(g.s_j + (g.s_root - g.s_j) * t);
            assert!(
                r >= radius - 1e-9,
                "fillet turned back at {r} from {radius}"
            );
            assert!(a >= angle - 1e-9, "fillet angle turned back");
            radius = r;
            angle = a;
        }
        // The fillet either stops short of mid-space, leaving a root arc, or
        // reaches it exactly and leaves none. Both are real; which one you get
        // is the cutter's tip width against the ring's space.
        assert!(
            angle <= g.half_pitch + 1e-9,
            "the fillet ran past mid-space to {angle}, beyond {}",
            g.half_pitch
        );
        if g.s_root == 0.0 {
            assert!(
                (radius - g.rf).abs() < 1e-9,
                "fillet reached {radius}, root {}",
                g.rf
            );
        } else {
            assert!(
                (angle - g.half_pitch).abs() < 1e-9,
                "a fully filleted root must stop exactly at mid-space, not {angle}"
            );
        }
    }

    /// A bigger cutter takes more out: its tip corner is flatter, so the fillet
    /// it leaves reaches the flank higher up.
    #[test]
    fn a_larger_cutter_moves_the_junction() {
        let small = Ring::new(
            &GearParams {
                teeth: 60,
                ..Default::default()
            },
            &Cutter {
                teeth: 15,
                ..Cutter::default()
            },
        );
        let large = Ring::new(
            &GearParams {
                teeth: 60,
                ..Default::default()
            },
            &Cutter {
                teeth: 40,
                ..Cutter::default()
            },
        );
        for g in [&small, &large] {
            assert!(
                g.clamps
                    .iter()
                    .all(|c| c.contains("fully filleted") || c.contains("not generated")),
                "{:?}",
                g.clamps
            );
        }
        let (r_small, _) = small.involute_at(small.u_j);
        let (r_large, _) = large.involute_at(large.u_j);
        assert!(
            (r_small - r_large).abs() > 1e-6,
            "the cutter should change the junction: {r_small} against {r_large}"
        );
    }

    /// The outline closes, stays between the tip and the root, and has the
    /// tooth count it claims.
    #[test]
    fn the_profile_closes_and_stays_between_the_tip_and_the_root() {
        for teeth in [43u32, 60, 90] {
            let g = ring(teeth);
            let outline = g.profile(120);
            assert!(
                outline.len() > 100,
                "z={teeth}: only {} points",
                outline.len()
            );
            assert_eq!(
                outline.first(),
                outline.last(),
                "z={teeth}: the outline must close"
            );
            for [x, y] in &outline {
                let r = f64::hypot(*x, *y);
                assert!(
                    r >= g.ra - 1e-9 && r <= g.rf + 1e-9,
                    "z={teeth}: a point at {r}, outside ({}, {})",
                    g.ra,
                    g.rf
                );
            }
            // The tip is reached once per tooth and the root likewise.
            let at_tip = outline
                .iter()
                .filter(|[x, y]| (f64::hypot(*x, *y) - g.ra).abs() < 1e-9)
                .count();
            assert!(
                at_tip >= teeth as usize,
                "z={teeth}: the tip is touched {at_tip} times"
            );
        }
    }

    /// A ring's outline is traced the way its material lies: every point of it
    /// is *outside* the tip circle, so the shape is a bore rather than a disc.
    /// An external gear of the same teeth is the mirror statement.
    #[test]
    fn the_outline_is_a_bore_where_an_external_gears_is_a_disc() {
        let g = ring(60);
        let external = Gear::new(GearParams {
            teeth: 60,
            ..Default::default()
        });
        let ring_max = g
            .profile(120)
            .iter()
            .map(|[x, y]| f64::hypot(*x, *y))
            .fold(0.0_f64, f64::max);
        let ext_max = external
            .profile(120)
            .iter()
            .map(|[x, y]| f64::hypot(*x, *y))
            .fold(0.0_f64, f64::max);
        // The furthest point is where the fillet stops. With a fully filleted
        // root that is where the two fillets meet at mid-space, a hair *inside*
        // the root circle — the root circle is the cutter's reach, not the
        // part's boundary, and the two only coincide when a root arc exists.
        let deepest = g.trochoid_at(g.s_root).0;
        assert!(
            (ring_max - deepest).abs() < 1e-9,
            "a ring's furthest point is where its fillet ends: {ring_max} against {deepest}"
        );
        assert!(
            deepest <= g.rf + 1e-12,
            "and that cannot be beyond the root circle"
        );
        if g.s_root != 0.0 {
            assert!(
                deepest < g.rf,
                "a fully filleted root never reaches the root circle"
            );
        }
        assert!(
            (ext_max - external.ra).abs() < 1e-9,
            "an external gear's furthest point is its tip"
        );
        assert!(
            ring_max > ext_max,
            "the ring encloses the gear of the same z"
        );
    }

    /// **Milestone 8's gate: is this the shape that tool would leave?**
    ///
    /// The cut is simulated — the cutter swept through the rolling motion, its
    /// boundary transformed into the ring's frame, the envelope taken — and
    /// compared with the analytic profile. It consults none of the ring's flank,
    /// fillet or junction, which is the point: every other test here checks a
    /// piece, and a construction can be right in every part and wrong in how the
    /// parts are placed.
    ///
    /// # What it caught
    ///
    /// One fault in the geometry and two in itself, and the order matters.
    ///
    /// `Cutter::default()` **was not a tool**: a 0.38-module tip round on a
    /// 20-tooth cutter with a 1.25 addendum leaves a tip 0.377 mm wide, so the
    /// two corner rounds overlap. 0.38 is the *rack's* figure and does not carry
    /// over. `ShaperCut` now refuses such a cutter.
    ///
    /// Then a 0.1 mm disagreement that took some finding, because the obvious
    /// check exonerated the wrong thing. The simulation's corner-centre
    /// trajectory matched [`crate::shaper::ShaperCut::corner_centre_at`]
    /// exactly — but that match is **invariant under mirroring the cutter's
    /// tooth**, so it confirmed nothing about where the flank was. The corner
    /// sits at `−θ_g` from the cutter's tooth centreline, on the flank *facing*
    /// the ring's tooth, and the simulation had put it at `+θ_g`.
    ///
    /// What pointed at the simulation rather than the profile was that the
    /// envelope was **not an involute of the ring's base circle**. Conjugate
    /// action says it has to be, so `θ − inv α` must be constant along the
    /// flank; it drifted. Once the flank moved to the right side it stopped
    /// drifting, and now sits on `ψ_b` to 6e-5 rad.
    ///
    /// The other fault was the sweep spanning one circular pitch instead of two:
    /// an engagement outlasts a pitch of travel, so the flank near the ring's
    /// tip was never generated.
    #[test]
    fn the_generated_profile_is_the_shape_the_cutter_would_leave() {
        for (teeth, cutter_teeth) in [(43u32, 20u32), (60, 20), (60, 30), (90, 25)] {
            let g = Ring::new(
                &GearParams {
                    teeth,
                    ..Default::default()
                },
                &Cutter {
                    teeth: cutter_teeth,
                    ..Cutter::default()
                },
            );
            let report = crate::verify::check_ring_cut(&g, 300, 12_000);
            assert!(
                report.samples > 250,
                "z={teeth}/{cutter_teeth}: only {} radii were reached",
                report.samples
            );
            // The floor is the simulation's own discretisation — a radius bin is
            // 7 µm wide and the envelope is a minimum over samples — so a couple
            // of microns is as close as this can come.
            assert!(
                report.worst_distance < 5e-3,
                "z={teeth}/{cutter_teeth}: the cut and the profile differ by {} mm",
                report.worst_distance
            );
        }
    }

    /// **A shifted ring is still the shape its cutter leaves** — because the
    /// shift is *where the cutter sits*.
    ///
    /// A shaper cannot be displaced the way a rack can. A rack's pitch line is a
    /// machine setting, so shifting it leaves the rolling alone; two pinions have
    /// their ratio fixed by their tooth counts, so the pitch point is wherever
    /// the centre distance puts it and the rolling circles move with it. One
    /// factor `a / a_ref` carries all of that, and at zero shift it is exactly 1.
    #[test]
    fn a_shifted_ring_is_the_shape_its_cutter_leaves() {
        for teeth in [43u32, 60] {
            for x in [-0.4, -0.25, -0.1, 0.0, 0.1, 0.25, 0.5] {
                let g = Ring::new(
                    &GearParams {
                        teeth,
                        profile_shift: x,
                        ..Default::default()
                    },
                    &Cutter::default(),
                );
                let report = crate::verify::check_ring_cut(&g, 400, 4_000);
                assert!(
                    report.worst_distance < 5e-3,
                    "z={teeth} x={x}: cut and profile differ by {} mm",
                    report.worst_distance
                );
            }
        }
    }

    /// **...and the check can tell when it is not.**
    ///
    /// The point of this test is not the ring, it is the gate. The previous cut
    /// simulation derived the cutter's tooth from the ring's — the same inference
    /// the model made — so it agreed to 2.7 µm on a ring whose cutter was 0.44 mm
    /// out of place, and reported nothing. That is the §12 trap exactly: a check
    /// that cannot distinguish two cases is not evidence for either.
    ///
    /// So place the cutter where the old model put it, at reference centres, and
    /// require the gate to *fail*. It comes out 13–66× the noise floor.
    #[test]
    fn a_cutter_at_the_wrong_centre_distance_is_visible_to_the_gate() {
        for x in [0.1, 0.25, 0.5, -0.25] {
            let good = Ring::new(
                &GearParams {
                    teeth: 43,
                    profile_shift: x,
                    ..Default::default()
                },
                &Cutter::default(),
            );
            let mut bad = good.clone();
            let a_ref = bad.cut.reference_centre_distance();
            bad.cut.phase *= a_ref / good.cut.centre_distance;
            bad.cut.centre_distance = a_ref;
            bad.cut.workpiece_operating_radius = bad.r;
            bad.cut.cutter_operating_radius = bad.cut.cutter_radius;

            let right = crate::verify::check_ring_cut(&good, 400, 4_000).worst_distance;
            let wrong = crate::verify::check_ring_cut(&bad, 400, 4_000).worst_distance;
            assert!(
                wrong > 10.0 * right,
                "x={x}: a cutter {:.4} mm out of place must be visible, but the \
                 gate said {wrong} against {right}",
                a_ref - good.cut.centre_distance
            );
        }
    }

    /// **How far down a cutter can generate, and what moves it.**
    ///
    /// The limit is where the cutter's own involute runs out: its flank stops at
    /// its base circle, and conjugate points share a place on the line of
    /// action, so the deepest it reaches on the ring is `√(r_b² + (a sin α_t)²)`.
    /// A bigger cutter sits closer — the centre distance is `r − r_c` — so it
    /// reaches further down. That is the lever a designer has, and it is the
    /// reason the same ring is a different part cut two ways.
    #[test]
    fn a_bigger_cutter_generates_further_down_the_flank() {
        let build = |cutter_teeth: u32| {
            Ring::new(
                &GearParams {
                    teeth: 60,
                    ..Default::default()
                },
                &Cutter {
                    teeth: cutter_teeth,
                    ..Cutter::default()
                },
            )
        };
        let mut previous = f64::INFINITY;
        for cutter_teeth in [15u32, 20, 30, 40, 50] {
            let g = build(cutter_teeth);
            let limit = g.generation_limit();
            // the closed form, spelled out again from the other direction
            let expected = f64::hypot(
                g.rb,
                (g.r - g.mt * f64::from(cutter_teeth) / 2.0) * g.alpha_t.sin(),
            );
            assert!((limit - expected).abs() < 1e-12);
            assert!(
                limit < previous,
                "z_c={cutter_teeth}: limit {limit} did not improve on {previous}"
            );
            previous = limit;
        }
        // A cutter close in size to the ring reaches past the tip; a small one
        // does not, and says so.
        assert!(
            build(50).fully_generated(),
            "a 50-tooth cutter should reach"
        );
        let small = build(15);
        assert!(!small.fully_generated());
        assert!(
            small.clamps.iter().any(|c| c.contains("not generated")),
            "{:?}",
            small.clamps
        );
    }

    fn pinion(teeth: u32) -> Gear {
        Gear::new(GearParams {
            teeth,
            ..Default::default()
        })
    }

    /// The relation the whole mesh section rests on, checked at the one place
    /// its answer is known independently: **the pitch point**. There the pinion
    /// touches at its own pitch radius and the ring at its own, and the two
    /// distances along the line of action differ by `a sin α_w`.
    #[test]
    fn the_conjugate_relation_holds_at_the_pitch_point() {
        for (ring_teeth, pinion_teeth) in [(60u32, 20u32), (43, 17), (90, 40)] {
            let g = ring(ring_teeth);
            let p = pinion(pinion_teeth);
            let m = mesh_with(&g, &p).unwrap();

            let ring_side = (g.r * g.r - g.rb * g.rb).sqrt();
            let pinion_side = (p.r * p.r - p.rb * p.rb).sqrt();
            let along = m.centre_distance * m.alpha_w.sin();
            assert!(
                (ring_side - pinion_side - along).abs() < 1e-9,
                "z={ring_teeth}/{pinion_teeth}: {ring_side} − {pinion_side} vs {along}"
            );
        }
    }

    /// **The general contact path and the ring's own agree** — two routes to one
    /// number, written independently.
    ///
    /// `mesh_with` computes the path length directly as `T₁ − T₂ + a sin α_w`.
    /// [`crate::contact::ContactPath`] reaches it as `approach + recess` from
    /// **signed** radii, using the same expressions it uses for an external pair.
    /// Neither knows about the other, so agreement says the sign convention
    /// reproduces the internal geometry rather than merely being self-consistent.
    #[test]
    fn the_general_contact_path_agrees_with_the_rings_own() {
        for (zr, zp, xr, xp) in [
            (60u32, 20u32, 0.0, 0.0),
            (43, 17, 0.0, 0.0),
            (90, 40, 0.0, 0.0),
            (60, 20, 0.3, 0.0),
            (60, 20, 0.0, 0.25),
            (43, 17, -0.2, 0.15),
        ] {
            let p = |teeth: u32, x: f64| GearParams {
                teeth,
                profile_shift: x,
                ..Default::default()
            };
            let g = Ring::new(&p(zr, xr), &Cutter::default());
            let pin = Gear::new(p(zp, xp));
            let mesh =
                crate::mesh::Mesh::new(&pin, &Gear::new(p(zr, xr)), MeshKind::Internal).unwrap();
            let path = crate::contact::ContactPath::new(&pin, g.ra, &mesh).unwrap();
            let own = mesh_with(&g, &pin).unwrap();

            assert!(
                (path.alpha_w - own.alpha_w).abs() < 1e-12,
                "z={zr}/{zp} x={xr}/{xp}: alpha_w {} vs {}",
                path.alpha_w,
                own.alpha_w
            );
            assert!(
                (path.contact_ratio - own.contact_ratio).abs() < 1e-12,
                "z={zr}/{zp} x={xr}/{xp}: contact ratio {} vs {}",
                path.contact_ratio,
                own.contact_ratio
            );
            // Both ends of the path are real, and the ring's tip is the shallow
            // end — its approach comes from the pitch point *inward*.
            assert!(path.approach > 0.0 && path.recess > 0.0);
        }
    }

    /// **An internal mesh presses more gently than the external pair of the same
    /// teeth**, and this is the first thing to ask `contact_stress` for an
    /// internal path at all.
    ///
    /// Convex against concave gives a larger relative radius of curvature, so
    /// less Hertzian pressure at the same load — one of the reasons a planetary
    /// stage carries what it does. A law rather than a number, and the cheapest
    /// check that the whole signed route reaches a stress instead of a NaN.
    #[test]
    fn an_internal_mesh_presses_more_gently_than_its_external_twin() {
        use crate::contact::ContactPath;
        use crate::mesh::Mesh;
        use crate::strength::{contact_stress, Load, PARALLEL_AXES};

        for (zr, zp) in [(60u32, 20u32), (43, 17), (90, 40)] {
            let p = |teeth: u32| GearParams {
                teeth,
                ..Default::default()
            };
            let pin = Gear::new(p(zp));
            let wheel = Gear::new(p(zr));
            let g = Ring::new(&p(zr), &Cutter::default());
            let load = Load::new(2.0, 10.0);
            let e_star = 113_000.0;

            let internal = {
                let m = Mesh::new(&pin, &wheel, MeshKind::Internal).unwrap();
                let path = ContactPath::new(&pin, g.ra, &m).unwrap();
                contact_stress(&path, &m, &pin, PARALLEL_AXES, &load, e_star).unwrap()
            };
            let external = {
                let m = Mesh::new(&pin, &wheel, MeshKind::External).unwrap();
                let path = ContactPath::new(&pin, wheel.ra, &m).unwrap();
                contact_stress(&path, &m, &pin, PARALLEL_AXES, &load, e_star).unwrap()
            };

            assert!(
                internal.worst > 0.0 && internal.worst.is_finite(),
                "z={zr}/{zp}: internal stress is {}",
                internal.worst
            );
            assert!(
                internal.worst < external.worst,
                "z={zr}/{zp}: internal {} should be below external {}",
                internal.worst,
                external.worst
            );
            // ...and the relative radius is the reason, not the load.
            assert!(internal.relative_radius > external.relative_radius);
        }
    }

    /// **A shifted internal pair has zero backlash at the centre distance
    /// `mesh_with` returns**, measured from the two profiles rather than from
    /// the relation that produced it.
    ///
    /// The same law `geometry_laws.rs` checks through `Mesh`, asked here through
    /// the ring's own mesh — because these are two routes to one relation and a
    /// disagreement between them would be invisible to either alone.
    #[test]
    fn a_shifted_internal_mesh_has_zero_backlash_at_its_own_centre_distance() {
        for (zr, zp, xr, xp) in [
            (60u32, 20u32, 0.0, 0.0),
            (60, 20, 0.3, 0.0),
            (60, 20, 0.0, 0.3),
            (60, 20, 0.4, 0.4),
            (43, 17, -0.2, 0.25),
            (90, 40, 0.15, -0.1),
        ] {
            let g = Ring::new(
                &GearParams {
                    teeth: zr,
                    profile_shift: xr,
                    ..Default::default()
                },
                &Cutter::default(),
            );
            let p = Gear::new(GearParams {
                teeth: zp,
                profile_shift: xp,
                ..Default::default()
            });
            let m = mesh_with(&g, &p).unwrap();

            // Operating circles: the ring's is one centre distance beyond the
            // pinion's, which is what makes the pair internal.
            let sz = f64::from(zr) - f64::from(zp);
            let rp = m.centre_distance * f64::from(zp) / sz;
            let rr = m.centre_distance * f64::from(zr) / sz;
            assert!((rr - rp - m.centre_distance).abs() < 1e-12);

            let u = (((rp / p.rb).powi(2) - 1.0).max(0.0)).sqrt();
            let tooth = 2.0 * rp * (p.psi_b - inv_from_roll(u));
            let space = g.space_width_at(rr);
            assert!(
                (space - tooth).abs() < 1e-10,
                "z={zr}/{zp} x={xr}/{xp}: backlash {} mm",
                space - tooth
            );
        }
    }

    /// **A standard pair is the value of the shifted formula at zero, not a
    /// separate case.**
    ///
    /// `mesh_with` used to assert `α_w = α_t` and `a = r_ring − r_pinion`
    /// outright. Now it reaches both through the involute inversion, so they are
    /// arrived at — and this is what says the general route did not move them.
    #[test]
    fn an_unshifted_internal_pair_still_meshes_at_the_difference_of_its_radii() {
        for (zr, zp) in [(60u32, 20u32), (43, 17), (90, 40), (51, 17)] {
            let (g, p) = (ring(zr), pinion(zp));
            let m = mesh_with(&g, &p).unwrap();
            assert!(
                (m.centre_distance - (g.r - p.r)).abs() < 1e-12,
                "z={zr}/{zp}: {} vs {}",
                m.centre_distance,
                g.r - p.r
            );
            assert!((m.alpha_w - g.alpha_t).abs() < 1e-12);
        }
    }

    /// Shifting the **ring** further than its pinion opens the mesh out: its
    /// space is wider, so the pinion sits further from the ring's axis.
    ///
    /// A direction rather than a number, because the number is not independently
    /// known — the §12 rule about not predicting a threshold the computation can
    /// find.
    #[test]
    fn shifting_the_ring_moves_the_pinion_outward() {
        let mut last = f64::NEG_INFINITY;
        for xr in [-0.3, -0.15, 0.0, 0.15, 0.3, 0.45] {
            let g = Ring::new(
                &GearParams {
                    teeth: 60,
                    profile_shift: xr,
                    ..Default::default()
                },
                &Cutter::default(),
            );
            let a = mesh_with(&g, &pinion(20)).unwrap().centre_distance;
            assert!(
                a > last,
                "x_ring={xr}: centre distance must rise, {a} <= {last}"
            );
            last = a;
        }
        // ...and shifting the pinion by the same amount as the ring puts it
        // back: only the difference of the two shifts reaches the mesh.
        let both = Ring::new(
            &GearParams {
                teeth: 60,
                profile_shift: 0.3,
                ..Default::default()
            },
            &Cutter::default(),
        );
        let shifted_pinion = Gear::new(GearParams {
            teeth: 20,
            profile_shift: 0.3,
            ..Default::default()
        });
        let m = mesh_with(&both, &shifted_pinion).unwrap();
        assert!(
            (m.centre_distance - (both.r - shifted_pinion.r)).abs() < 1e-12,
            "equal shifts must not move the centre distance"
        );
    }

    /// An internal mesh has a **higher** contact ratio than the external pair of
    /// the same teeth — one of the reasons planetary stages use them — and it
    /// must still be above one or the mesh loses contact between teeth.
    #[test]
    fn an_internal_mesh_has_more_contact_than_the_external_pair_of_the_same_teeth() {
        for (ring_teeth, pinion_teeth) in [(60u32, 20u32), (43, 17), (90, 40)] {
            let internal = mesh_with(&ring(ring_teeth), &pinion(pinion_teeth))
                .unwrap()
                .contact_ratio;

            let a = pinion(pinion_teeth);
            let b = pinion(ring_teeth);
            let m = crate::mesh::Mesh::new(&a, &b, crate::mesh::MeshKind::External).unwrap();
            let external = crate::contact::ContactPath::new(&a, b.ra, &m)
                .unwrap()
                .contact_ratio;

            assert!(
                internal > external,
                "z={ring_teeth}/{pinion_teeth}: internal {internal} against external {external}"
            );
            assert!(internal > 1.0 && internal < 3.0, "implausible {internal}");
        }
    }

    /// **A standard full-depth internal pair interferes, and the fix is the one
    /// the handbooks give: shorten the ring's tooth.**
    ///
    /// The condition is exact. The ring's tip can only touch the pinion's
    /// involute while `√(r_a2² − r_b2²) ≥ a sin α_w`; below that the contact
    /// would have to happen inside the pinion's base circle, where there is no
    /// involute to touch. A 60-tooth ring on a 20-tooth pinion misses it by
    /// 0.009 mm at a full addendum — which is why internal pairs are not built
    /// full-depth, and part of where the rule of thumb about tooth differences
    /// comes from.
    #[test]
    fn a_full_depth_internal_pair_interferes_and_a_shorter_ring_tooth_fixes_it() {
        let full = mesh_with(&ring(60), &pinion(20)).unwrap();
        assert!(
            full.involute_interference,
            "a standard full-depth 60/20 pair should interfere"
        );

        let shortened = Ring::new(
            &GearParams {
                teeth: 60,
                addendum: 0.8,
                ..Default::default()
            },
            &Cutter::default(),
        );
        assert!(
            !mesh_with(&shortened, &pinion(20))
                .unwrap()
                .involute_interference,
            "shortening the ring's tooth should clear it"
        );

        // And a full-depth ring interferes across the whole useful range of
        // pinions, not at one tooth count — which is why the remedy is the
        // ring's addendum rather than a rule about tooth differences.
        assert!(
            (20u32..=40).all(|z| {
                mesh_with(&ring(60), &pinion(z))
                    .unwrap()
                    .involute_interference
            }),
            "a full-depth 60-tooth ring should interfere with every pinion here"
        );
        // Shortening the ring's tooth widens the set of pinions that mesh
        // cleanly. Stated as a comparison rather than a threshold, because the
        // threshold is what the computation is *for* — predicting it by hand is
        // how the wrong expectations in this file's history got written.
        let clear = |g: &Ring| {
            (16u32..=40)
                .filter(|&z| !mesh_with(g, &pinion(z)).unwrap().involute_interference)
                .count()
        };
        assert!(
            clear(&shortened) > clear(&ring(60)),
            "a shorter ring tooth should clear more pinions: {} against {}",
            clear(&shortened),
            clear(&ring(60))
        );
    }

    /// The two interference checks are about **tips reaching past flanks**, so
    /// growing a tip must be what triggers them — not a coincidence of tooth
    /// counts.
    #[test]
    fn a_taller_pinion_tooth_is_what_drives_it_into_the_rings_fillet() {
        let g = ring(60);
        let mut fouled = false;
        for addendum in [1.0_f64, 1.4, 1.8, 2.2, 2.6] {
            let p = Gear::new(GearParams {
                teeth: 20,
                addendum,
                ..Default::default()
            });
            let Some(m) = mesh_with(&g, &p) else { continue };
            if m.trochoid_interference {
                fouled = true;
            }
            // The contact always moves deeper into the ring as the pinion grows.
            assert!(
                m.ring_contact_at_pinion_tip > g.r,
                "contact should be outside the pitch circle"
            );
        }
        assert!(
            fouled,
            "a tall enough pinion tooth must reach the ring's fillet"
        );
    }

    /// A pair that cannot mesh says so rather than returning numbers.
    #[test]
    fn a_pair_that_cannot_mesh_is_refused() {
        let g = ring(60);
        assert!(
            mesh_with(
                &g,
                &Gear::new(GearParams {
                    teeth: 20,
                    module: 2.0,
                    ..Default::default()
                })
            )
            .is_none(),
            "different modules cannot mesh"
        );
        assert!(
            mesh_with(&g, &pinion(60)).is_none(),
            "a pinion the size of the ring has no centre distance"
        );
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

    /// **On a ring it is the space that thickness modification and profile shift
    /// describe, so a larger k or x makes the tooth thinner.**
    ///
    /// The opposite of an external gear, and not a free choice: the space is
    /// where the mating pinion's tooth goes, so the space is what is generated
    /// like a tooth. `Mesh::new`'s internal relation flips gear 2's `x` and `x_s`
    /// together and is consistent only with this reading — see
    /// `an_internal_pair_has_zero_backlash_at_the_centre_distance_the_mesh_gives`,
    /// which is the check that decides it.
    #[test]
    fn thickness_modification_and_shift_act_on_the_space_not_the_tooth() {
        let of = |k: f64, x: f64| {
            Ring::new(
                &GearParams {
                    teeth: 43,
                    thickness_mod: k,
                    profile_shift: x,
                    ..Default::default()
                },
                &Cutter::default(),
            )
        };
        let base = of(1.0, 0.0);
        let pitch = 2.0 * base.r * base.half_pitch;
        // k = 1, x = 0 is exactly half the circular pitch either way round, which
        // is why the two readings agree there and nowhere else.
        assert!((base.tooth_thickness_at(base.r) - pitch / 2.0).abs() < 1e-12);
        assert!((base.space_width_at(base.r) - pitch / 2.0).abs() < 1e-12);

        for (k, x) in [(1.2, 0.0), (1.0, 0.3), (1.1, 0.15)] {
            let more = of(k, x);
            assert!(
                more.space_width_at(more.r) > base.space_width_at(base.r),
                "k={k} x={x}: the space must widen"
            );
            assert!(
                more.tooth_thickness_at(more.r) < base.tooth_thickness_at(base.r),
                "k={k} x={x}: ...so the tooth must thin"
            );
            // ...and they still come to the circular pitch, as complements must.
            assert!(
                (more.tooth_thickness_at(more.r) + more.space_width_at(more.r) - pitch).abs()
                    < 1e-12
            );
        }
    }

    /// A profile shift moves a ring's whole form **outward**: its tooth gets
    /// shorter, because it points inward, and its space deeper.
    #[test]
    fn a_positive_shift_moves_a_rings_radii_outward() {
        let of = |x: f64| {
            Ring::new(
                &GearParams {
                    teeth: 43,
                    profile_shift: x,
                    ..Default::default()
                },
                &Cutter::default(),
            )
        };
        let (lo, mid, hi) = (of(-0.25), of(0.0), of(0.25));
        assert!(lo.ra < mid.ra && mid.ra < hi.ra, "tip radius rises with x");
        assert!(lo.rf < mid.rf && mid.rf < hi.rf, "root radius too");
        // The pitch and base circles are properties of the tooth count and the
        // rack, and a shift leaves both exactly where they were.
        assert_eq!(lo.r, hi.r);
        assert_eq!(lo.rb, hi.rb);

        // The tip is a bore, so it moves by exactly `x m`.
        let m = 1.0;
        assert!((hi.ra - mid.ra - 0.25 * m).abs() < 1e-12);

        // The root is not: it is wherever the cutter's tip reaches, so what is
        // constant is `r_f − a_cut`, the tool's own tip radius.
        let tip_reach = |g: &Ring| g.rf - g.cut.centre_distance;
        for g in [&lo, &mid, &hi] {
            assert!(
                (tip_reach(g) - tip_reach(&mid)).abs() < 1e-12,
                "the root circle is the cutter's reach, at every shift"
            );
        }
        // ...and that is *not* the linearised `r + m(dedendum + x)`. The gap is
        // small but sits well above the 3.6 µm the cut simulation resolves, which
        // is why the exact form is used.
        let linearised = mid.r + m * (1.25 + 0.25);
        let gap = (hi.rf - linearised).abs();
        assert!(
            (1e-5..5e-2).contains(&gap),
            "expected a gap of order tens of µm between exact and linearised, got {gap}"
        );
    }

    /// A ring whose addendum would reach inside its own base circle is clamped
    /// and says so, rather than producing an involute that does not exist.
    #[test]
    fn an_addendum_reaching_past_the_base_circle_is_clamped_and_reported() {
        let g = Ring::new(
            &GearParams {
                teeth: 20,
                addendum: 3.0,
                ..Default::default()
            },
            &Cutter::default(),
        );
        assert!(g.ra >= g.rb);
        assert!(
            g.clamps.iter().any(|c| c.contains("base circle")),
            "clamps: {:?}",
            g.clamps
        );
    }
}
