//! Internal (ring) gear geometry.
//!
//! A ring's tooth points **inward**: its tip circle is smaller than its pitch
//! circle and its root circle larger, which is the reverse of everything in
//! [`crate::tooth`]. The flank is still an involute of the ring's own base
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
//! `inv_from_roll(u)` where [`crate::tooth::Tooth::involute_at`] subtracts it.
//! It is checked here as the complement it is, rather than asserted — tooth plus
//! space must come to the circular pitch at *every* radius, not only at the
//! pitch circle where it was set.
//!
//! # What is not here
//!
//! **Radial assembly** — whether a pinion can be brought in sideways past the
//! ring's teeth. It is a swept-motion question rather than a comparison of tip
//! circles, and docs/reference.md#internal-gears records what happened to the attempt that treated it as
//! one. It belongs with the planetary set that actually asks.
//!
//! **A bending rating.** A ring's tooth widens outward and its fillet is
//! shaper-cut, so neither the critical section nor the notch input is the one
//! [`crate::strength`] measures on an external tooth. NASA TM-107012's inscribed
//! parabola is the model, and it needs the cutter placed exactly — which is what
//! the shifted cut above now provides.

use crate::involute::{inv, inv_from_roll};
use crate::mesh::{operating_geometry, MeshKind};
use crate::note::{key, Note};
use crate::params::{guard, GearParams};
use crate::shaper::{CutParams, ShaperCut};
use crate::solve::{brent, Tol};
use crate::tooth::Section;
use crate::tooth::Tooth;

/// The pinion cutter a ring is shaped with.
///
/// A ring has no meaningful geometry without one: unlike a rack-cut external
/// gear, where the tool is implied by the basic rack, the fillet a ring gets
/// depends on how many teeth its cutter had. Two rings with identical teeth,
/// module and depth are *different parts* if they were shaped differently.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
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

/// The root fillet a shaper cut, as the span of the cutter corner's normal
/// angle that traced it (radians, see [`ShaperCut::trochoid_at`]).
///
/// The two angles are what the trochoid is read at; the radii and angles
/// round the ring come from [`Ring::trochoid_at`] rather than being stored, so
/// there is one place the curve is defined. The corner's normal angle rather
/// than the cutter's travel, because the travel stands still where the corner
/// rides on the pitch point and the whole round is cut at one travel.
#[derive(Clone, Copy, Debug)]
pub struct Fillet {
    /// Normal angle at the flank/fillet junction, radians: negative.
    pub phi_j: f64,
    /// Normal angle at which the fillet ends, radians.
    ///
    /// Zero when a root arc follows — the deepest cut, at mid-space. Non-zero
    /// when the fillets from the two flanks meet before they get there, which
    /// leaves a **fully filleted root** with no flat at all. Common, and not a
    /// fault: it simply means the cutter's tip is wide enough that its corner
    /// rounds overlap.
    pub phi_root: f64,
}

/// A ring gear's cross-section, so far as the involute goes.
#[derive(Clone, Debug)]
pub struct Ring {
    /// The inputs this was built from, kept as [`crate::Tooth`] keeps its own —
    /// so a ring can produce its virtual spur section without being handed back
    /// what it was made of.
    pub params: GearParams,
    /// ...and the tool, because the tool is part of the part (docs/reference.md#internal-gears).
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
    /// reason `Mesh` sums them (docs/reference.md#tooth-thickness-and-its-equivalent-shift). Positive widens the space and thins the
    /// tooth; see [`Ring::cut_by`].
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
    /// Roll parameter where the flank hands over to the fillet — or, when no
    /// fillet was cut, where the flank reaches the root circle.
    pub u_j: f64,
    /// The trochoid the cutter's tip corner left, when it left one.
    ///
    /// `None` in the two cases where nothing is generated: the corner rounds
    /// overlap, so the tool is not a tool, or the cutter never reaches this
    /// ring's flank. Both are reported in [`Self::clamps`].
    ///
    /// An `Option` rather than a pair of zeros, because **a zero-length fillet
    /// is not a fillet**. Sampled as though it were one, it made every section
    /// of the profile fall back to its minimum point count: a 600-point outline
    /// came out with seven, the involute became two straight chords, and a ring
    /// drew as a sharp-rooted polygon that looked deliberate. See docs/corrections.md.
    pub fillet: Option<Fillet>,
    /// The cut that made this ring.
    pub cut: ShaperCut,
    /// Guards that altered the geometry.
    pub clamps: Vec<Note>,
}

impl Ring {
    /// Where this ring's usable flank begins and ends — see
    /// [`crate::mesh::FlankEnds`].
    ///
    /// **The tip is the lower radius here**, which is the whole of why this is a
    /// seam rather than a field read off either kind: a ring's flank runs
    /// *outwards* from its tip to the junction with its fillet, and a rack-cut
    /// tooth's runs the other way.
    #[must_use]
    pub fn flank_ends(&self) -> crate::mesh::FlankEnds {
        crate::mesh::FlankEnds {
            tip: self.ra,
            junction: self.involute_at(self.u_j).0,
        }
    }

    /// Build a ring from the same parameters an external gear takes.
    ///
    /// `addendum` is measured **inward** and `dedendum` outward, which is what
    /// makes it a ring; the numbers themselves mean the same as they do for an
    /// external gear.
    #[must_use]
    pub fn cut_by(params: &GearParams, cutter: &Cutter) -> Self {
        Self::cut_by_at_virtual_z(
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
    /// [`crate::Tooth`] builds its virtual gear from a non-integer `z`.
    #[must_use]
    pub fn cut_by_at_virtual_z(
        params: &GearParams,
        cutter: &Cutter,
        z: f64,
        cutter_teeth: f64,
    ) -> Self {
        #[cfg(test)]
        crate::testing::work::ring();
        let mut clamps = Vec::new();
        let beta = params.helix_angle.to_radians();
        let (alpha_n, raised) = params.normal_pressure_angle_rad();
        if raised {
            clamps.push(Note::new(key::CLAMP_PRESSURE_ANGLE_RAISED).number(
                "degrees",
                alpha_n.to_degrees(),
                1,
            ));
        }
        let m = params.module;
        let mt = m / beta.cos();
        let alpha_t = crate::plane::transverse_pressure_angle(alpha_n, beta);

        let r = z * mt / 2.0;
        let rb = r * alpha_t.cos();
        let half_pitch = std::f64::consts::PI / z;

        // ---- thickness. It is the SPACE that takes the external formula.
        //
        // A ring's space is where the mating pinion's tooth goes, and it is
        // generated the way a pinion's tooth is; the ring's tooth is whatever
        // the pitch leaves over. So `thickness_mod` and the profile shift are
        // applied to the space, using `Tooth`'s expression unchanged — and the
        // consequence is that a *larger* k or x makes a ring's tooth **thinner**,
        // the opposite of an external gear.
        //
        // This is not a convention free to choose. `Mesh::new`'s internal
        // relation flips gear 2's `x` and `x_s` together, and that is consistent
        // only with this reading: measured against tooth thicknesses at the
        // operating circles, the space reading gives exactly zero backlash at
        // every k while the tooth reading is out by 0.63 mm at k = 1.2 (docs/corrections.md).
        // The pair invariant is therefore `k₁ = k₂` for an internal mesh, where
        // an external one needs `k₁ + k₂ = 2`.
        let x = params.profile_shift;
        let x_thick = x + params.thickness_shift();
        let pitch = std::f64::consts::PI * mt;
        let mut space = mt * (std::f64::consts::PI / 2.0 + 2.0 * x_thick * alpha_n.tan());
        // Both ends are real limits: a space wider than the pitch leaves no
        // tooth, and one of zero width leaves no space for the pinion.
        // The same two limits `Tooth` puts on a tooth, applied to the space —
        // because on a ring the space is the thing generated like a tooth.
        let space_max = guard::MAX_TOOTH_THICKNESS_FRACTION_OF_PITCH * pitch;
        let space_min = guard::MIN_TOOTH_THICKNESS_MODULES * m;
        // The space expression read backwards: which thickness shift leaves a
        // space of this width. Needed only where a clamp has moved the space,
        // because the **tool** has to be the one that leaves the space the ring
        // actually has — a capped space cut by the tool the raw figure asked
        // for is a cut and a profile describing two different rings.
        let shift_for = |s: f64| (s / mt - std::f64::consts::PI / 2.0) / (2.0 * alpha_n.tan());
        let mut x_space = x_thick;
        if space > space_max {
            space = space_max;
            x_space = shift_for(space);
            clamps.push(Note::new(key::CLAMP_RING_SPACE_CAPPED));
        } else if space < space_min {
            space = space_min;
            x_space = shift_for(space);
            clamps.push(Note::new(key::CLAMP_RING_SPACE_RAISED));
        }
        let tooth = pitch - space;
        let psi_p = tooth / (2.0 * r);
        // ...and the sign that makes it a ring: outward from the base circle the
        // tooth *gains* angle rather than losing it.
        let psi_b = psi_p - inv(alpha_t);

        // ---- radii. The whole form shifts **outward** by `x m`, which shortens
        // a ring's tooth (it points inward) and deepens its space. Written as
        // `Tooth`'s two expressions with inward and outward exchanged.
        let mut ra = r - m * (params.addendum - x);

        // The tip cannot dip below the base circle: there is no involute there
        // to cut it from.
        //
        // ...and on a large ring the tooth runs out of *thickness* first.
        //
        // A ring's tooth narrows **inward**, so its tip is its thinnest section.
        // Thickness at roll `u` is `2 r (ψ_b + inv_from_roll(u))`, which reaches
        // zero where `inv α = −ψ_b` — possible only when `ψ_b < 0`, and that
        // happens once `π/2z < inv α_t`, about 105 teeth at 20°. Closed form
        // through the same `inv⁻¹` everything else uses, and it mirrors `Tooth`'s
        // pointed-tooth clamp rather than being a new kind of guard.
        //
        // Unclamped this is not a thin tooth but a **crossed** one: a 150-tooth
        // ring at a 3-module addendum came out at −0.211 mm of thickness, and its
        // outline is a self-intersecting polygon that would go into a DXF.
        //
        // Whichever floor binds, the tip is clamped onto it and the note names
        // that floor (decision 1): the base circle exactly, with no margin.
        let pointed = (psi_b < 0.0)
            .then(|| crate::involute::inv_inverse(-psi_b))
            .flatten()
            .map(|alpha_point| rb / alpha_point.cos());
        match pointed {
            Some(ra_point) if ra < ra_point => {
                clamps.push(Note::new(key::CLAMP_RING_TIP_RAISED).number("radius", ra_point, 4));
                ra = ra_point;
            }
            _ if ra < rb => {
                clamps.push(Note::new(key::CLAMP_RING_TIP_AT_BASE).number("radius", rb, 4));
                ra = rb;
            }
            _ => {}
        }

        let roll_at = |radius: f64| crate::involute::roll_at_radius(radius, rb);

        // ---- where the cutter sits.
        //
        // A shifted ring is cut by the *same* tool placed further out, and that
        // displacement is what its shift means for a shaper. The distance is the
        // internal relation between tool and workpiece, read through the shared
        // `operating_geometry`: the cutter is member 1 (external, unshifted) and
        // the ring is member 2, so the signed sums are `z_c − z_r` and `−x_thick`.
        // An internal pair needs the ring to have more teeth than its cutter, and
        // `Mesh::new` and `mesh_with` both refuse the other way round. Here it
        // arrives as a *centre distance*, `r − r_c`, which simply goes negative:
        // a 43-tooth ring "cut" by a 50-tooth shaper reported a root radius of
        // 29.75 mm against a pitch radius of 21.5, and the only complaint was
        // about the tip corner. Clamped rather than refused, because `Ring::cut_by`
        // reports rather than fails and which of the two counts to give up is the
        // designer's call.
        let cutter_teeth = if cutter_teeth >= z {
            let capped = (z - 1.0).max(1.0);
            clamps.push(Note::new(key::CLAMP_CUTTER_TEETH_REDUCED).number("teeth", capped, 0));
            capped
        } else {
            cutter_teeth
        };
        let cutter_radius = cutter_teeth * mt / 2.0;
        // **The thickness modification is the tool's tooth, not its plunge.**
        //
        // `k` is thickness-only by definition (docs/reference.md#tooth-thickness-and-its-equivalent-shift):
        // radial quantities take plain `x` and thickness ones take `x + x_s`,
        // and a ring's root radius is radial — it is where the cutter's tip
        // reaches. An external gear gets this from its rack, whose tooth is
        // narrowed or widened while its depth stays where it was. A shaper is
        // the same statement on a pinion: the **cutter's tooth** carries the
        // modification, and it comes out as the standard tooth scaled by `k`,
        // which is the basic rack's own definition read round a circle.
        //
        // Taken into the centre distance instead — as this did — `k` plunges the
        // tool, so the root diameter grows and shrinks with it and the ring
        // *scales* where its teeth should have thinned. That is a module change
        // wearing a thickness control's name, and it had a second face: on a
        // 43-tooth ring `k = 0.7` drove the pair clean out of the involute
        // domain, where the fallback below quietly cut at reference centres and
        // the thickness modification reached the cut not at all.
        //
        // The cutter's own thickness shift is what is left once the ring's is
        // accounted for, so the signed sum `x_c − x_ring` collapses to `−x` and
        // the plunge answers to the radial shift alone. Written as `−x` rather
        // than as the difference because it *is* `−x`, exactly, and a cancelled
        // pair of large terms is not the same floating point as the small one
        // they cancel to.
        let cutter_thickness_shift = x_space - x;
        let cutter_tooth =
            mt * (std::f64::consts::PI / 2.0 + 2.0 * cutter_thickness_shift * alpha_n.tan());
        let sum_z = cutter_teeth - z;
        // Falls back to reference centres when the **radial** shift alone takes
        // the pair out of the involute domain — a far smaller region than the
        // thickness sum reached, and one only `x` can enter.
        let a_cut = operating_geometry(mt, alpha_t, alpha_n, sum_z, -x)
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
        let mut rf = a_cut + cutter_tip_radius;

        // ---- ...and it cannot reach past where the space closes.
        //
        // **A ring's space narrows outward**, because the space is where the
        // mating pinion's tooth goes and a pinion's tooth narrows toward its own
        // tip. So the two flanks bounding one space converge as they go out, and
        // where they meet the space *ends*: the tooth half-angle
        // `ψ_b + inv α` has reached half the angular pitch and there is no
        // material left between them to cut.
        //
        // This is the same guard as the one on `ra` above, at the other end of
        // the same tooth, and the same one `Tooth` puts on an external gear's
        // pointed tip — `inv α = π/z − ψ_b` where that one solves `inv α = −ψ_b`,
        // through the same `inv⁻¹`. The space clamp above keeps `ψ_b < π/z`, so
        // the argument is positive and a radius always exists.
        //
        // Left unclamped this is not a shallow space but a **crossed** one: the
        // flank runs past the space's own centreline and its mirror image comes
        // back through it, which draws as the inverted spur of geometry at the
        // bottom of every tooth space. It bit whenever no fillet was cut to
        // truncate the flank first — a thickness modification of 0.6 on a
        // 43-tooth ring put the root 0.14 mm beyond the crossing, and 0.4 put it
        // 0.48 mm beyond. An external gear's space closes the same way when the
        // rack's tooth comes to a point before its depth, and `Rack::wanted_by`
        // stops the depth there under the same key.
        if let Some(alpha_close) = crate::involute::inv_inverse(half_pitch - psi_b) {
            let rf_max = rb / alpha_close.cos();
            if rf > rf_max {
                rf = rf_max;
                clamps.push(Note::new(key::CLAMP_SPACE_CLOSED).number("radius", rf_max, 4));
            }
        }
        let rf = rf;

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

        // The tool caps its own round where the tip cannot hold what was asked
        // for, exactly as `Tooth::new` does on an external gear. Say so: a
        // substituted tool that goes unmentioned is the same fault as a
        // clamped input that goes unmentioned, and this one changes the fillet
        // the part is rated on.
        let asked = m * cutter.tip_round;
        if let Some(c) = &cut {
            if c.tip_round < asked {
                clamps.push(Note::new(key::CLAMP_FILLET_CAPPED).number("radius", c.tip_round, 4));
            }
        }

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
            u_j: roll_at(rf),
            fillet: None,
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
            ring.clamps.push(Note::new(key::CLAMP_CUTTER_NO_TIP_CORNER));
            return ring;
        }
        match ring.solve_junction() {
            Some((u_j, phi_j)) => {
                ring.u_j = u_j;
                ring.fillet = Some(Fillet {
                    phi_j,
                    phi_root: ring.solve_root_end(phi_j),
                });
                if !ring.fully_generated() {
                    let limit = ring.generation_limit();
                    ring.clamps.push(
                        Note::new(key::CLAMP_RING_FLANK_UNGENERATED)
                            .number("limit", limit, 4)
                            .number("tip", ring.ra, 4),
                    );
                }
                if ring.fillet.is_some_and(|f| f.phi_root != 0.0) {
                    ring.clamps.push(Note::new(key::CLAMP_RING_FULLY_FILLETED));
                }
            }
            None => ring
                .clamps
                .push(Note::new(key::CLAMP_RING_FLANK_FILLET_GAP)),
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
    /// `a sin α_w` rather than summing to it:
    ///
    /// ```text
    /// √(r_j² − r_b²) = a sin α_w + √(r_tan² − r_bc²)
    /// ```
    ///
    /// and `r_tan` — where the cutter's round meets its flank — comes from the
    /// same offset-involute fact the phase used: the round's centre sits at roll
    /// `t_g`, so the tangency is at roll `t_g + ρ/r_bc` on the flank itself.
    ///
    /// The fillet's end of it is closed form too: there the round's normal is
    /// the flank's, tangent to the cutter's base circle, so its angle from the
    /// corner centre's radial line is `−atan t_g`
    /// ([`ShaperCut::junction_normal`]) — on a prolate path and a curtate one
    /// alike. Nothing here iterates.
    fn solve_junction(&self) -> Option<(f64, f64)> {
        let r_bc = self.cut.cutter_radius * self.alpha_t.cos();
        let t_g = crate::involute::roll_at_radius(self.cut.corner_radius, r_bc);
        let t_tan = t_g + self.cut.tip_round / r_bc;

        let along = self.cut.centre_distance * self.cut.alpha_w.sin() + r_bc * t_tan;
        let r_j = f64::hypot(self.rb, along);
        if !(r_j.is_finite() && r_j > self.ra && r_j < self.rf) {
            return None;
        }
        Some((self.roll_at(r_j), self.cut.junction_normal(self.alpha_t)))
    }

    /// Where the fillet ends: the deepest cut, or mid-space if it gets there
    /// first.
    ///
    /// The fillet is symmetric about mid-space, so two of them meeting there is
    /// the same statement as one of them reaching it. Monotone in the normal
    /// angle, so one bracketed step again.
    ///
    /// Takes the junction's normal angle rather than reading it back off
    /// `self`, because it is called while the fillet is being built and there
    /// is nothing to read yet.
    fn solve_root_end(&self, phi_j: f64) -> f64 {
        if self.trochoid_at(0.0).1 <= self.half_pitch {
            return 0.0;
        }
        let over = |phi: f64| self.trochoid_at(phi).1 - self.half_pitch;
        brent(over, phi_j, 0.0, Tol::default()).unwrap_or(0.0)
    }

    /// A circle to draw the rim at, mm — `r + 2 m_t`, so the annulus is two
    /// modules of material thick.
    ///
    /// **A drawing convention, not a design output.** A real ring's outside
    /// diameter is the designer's: it carries the bolt circle, the press fit
    /// and whatever the housing needs, none of which the tooth geometry knows
    /// about. What this is for is that a ring drawn as an outline alone is
    /// indistinguishable from an external gear — the teeth simply point the
    /// other way — so the viewport shades the material *outside* the bore, and
    /// needs somewhere to stop. Two modules is enough to read as a rim at any
    /// tooth count, because it scales with the teeth it surrounds.
    ///
    /// It lives here rather than in the viewport because the DXF wants the same
    /// circle on its construction layer, and two of them would be two.
    #[must_use]
    pub fn rim_radius(&self) -> f64 {
        self.r + 2.0 * self.mt
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
        crate::involute::roll_at_radius(radius, self.rb)
    }

    /// The fillet at the cutter corner's normal angle `phi` (radians), as
    /// `(radius, angle)`.
    #[must_use]
    pub fn trochoid_at(&self, phi: f64) -> (f64, f64) {
        self.cut.trochoid_at(phi)
    }

    /// The **virtual spur ring**: this ring's normal section, as a spur ring.
    ///
    /// Bending is rated on the normal section, so a helical ring has to be rated
    /// on this rather than on its transverse form — measuring `Y_F` transversely
    /// and dividing by `m_n` mixes planes and under-predicts by about `cos β`,
    /// the error `docs/corrections.md` records for external gears.
    ///
    /// The construction is ISO's, the same one [`crate::Tooth::virtual_spur`]
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
        Self::cut_by_at_virtual_z(
            &params,
            &self.cutter,
            f64::from(self.params.teeth.max(1)) / scale,
            f64::from(self.cutter.teeth.max(1)) / scale,
        )
    }

    /// Base helix angle, radians — `sin β_b = sin β cos α_n`.
    ///
    /// The same relation [`crate::tooth::Tooth::base_helix_angle`] gives an external
    /// gear; it is a property of the reference rack, not of which side the
    /// material is on.
    #[must_use]
    pub fn base_helix_angle(&self) -> f64 {
        let beta = self.params.helix_angle.to_radians();
        crate::plane::base_helix_angle(beta, self.alpha_n)
    }

    /// The fillet point and its tangent, Cartesian, tooth centred on `+y`.
    ///
    /// Straight through to [`ShaperCut::trochoid_point_and_tangent`], which
    /// differentiates the construction analytically. Named here so a ring's
    /// bending model reads its own geometry off the ring rather than reaching
    /// into the cut.
    #[must_use]
    pub fn fillet_point_and_tangent(&self, phi: f64) -> ([f64; 2], [f64; 2]) {
        self.cut.trochoid_point_and_tangent(phi)
    }

    /// The flank point and its tangent, Cartesian, tooth centred on `+y`.
    ///
    /// The same involute derivative an external gear's flank has, with the one
    /// sign that makes it a ring: `dθ/du` is **positive** here, because a ring's
    /// tooth gains angle outward where an external gear's loses it.
    #[must_use]
    pub fn flank_point_and_tangent(&self, u: f64) -> ([f64; 2], [f64; 2]) {
        let r = self.rb * f64::hypot(1.0, u);
        let th = self.psi_b + inv_from_roll(u);
        let (st, ct) = th.sin_cos();
        // `d/du (r sin θ, r cos θ)` is `(r_b u / √(1+u²)) (sin θ + u cos θ,
        // cos θ − u sin θ)` — positive `dθ/du`, where `Tooth`'s is negative:
        // the flipped `inv` term of the module documentation, differentiated.
        // Only the direction is used, so the vanishing factor is left off: at
        // the base circle the involute's speed is zero but its direction is
        // radial, and a zero vector there read as a tangency everywhere.
        ([r * st, r * ct], [st + u * ct, ct - u * st])
    }

    /// The flank point and the direction of the load there.
    ///
    /// The load acts along the involute normal, which is the line from the
    /// contact point to the base-circle tangency point. For a ring that tangency
    /// sits `roll` radians **forward** around the base circle rather than back,
    /// which is the same sign as above. That line is `(r_b u / √(1+u²)) (u sin θ
    /// − cos θ, u cos θ + sin θ)`, so its unit direction is written without the
    /// factor, and it is the base circle's tangent where `u = 0`.
    #[must_use]
    pub fn flank_point_and_load_direction(&self, roll: f64) -> ([f64; 2], [f64; 2]) {
        let (r, th) = self.involute_at(roll);
        let (st, ct) = th.sin_cos();
        let root = f64::hypot(1.0, roll);
        (
            [r * st, r * ct],
            [(roll * st - ct) / root, (roll * ct + st) / root],
        )
    }

    /// Radius of curvature of the fillet at normal angle `phi` (radians), mm.
    ///
    /// The cutter's, since the cutter is what leaves it —
    /// [`ShaperCut::trochoid_curvature_radius`](crate::shaper::ShaperCut::trochoid_curvature_radius),
    /// closed form and shared with the rack-cut case.
    #[must_use]
    pub fn fillet_curvature_radius(&self, phi: f64) -> f64 {
        self.cut.trochoid_curvature_radius(phi)
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
        let u = crate::involute::roll_at_radius(radius, self.rb);
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
    /// inward. A fully filleted root drops the last one, and a cut that
    /// generated no fillet drops the third: the flank then runs to the root
    /// circle and the space is flat from there.
    #[must_use]
    pub fn sections(&self) -> Vec<Section> {
        let mut out = vec![Section::TipArc, Section::Involute];
        if self.fillet.is_some() {
            out.push(Section::Trochoid);
        }
        if self.space_starts_at() < self.half_pitch {
            out.push(Section::RootArc);
        }
        out
    }

    /// The angle at which the flat of the tooth space begins: where the fillet
    /// ends, or where the flank does when no fillet was cut.
    ///
    /// One place to ask, so the root arc cannot start somewhere the section
    /// before it did not finish.
    pub(crate) fn space_starts_at(&self) -> f64 {
        self.fillet.map_or_else(
            || self.involute_at(self.u_j).1,
            |f| self.trochoid_at(f.phi_root).1,
        )
    }

    fn sample_section(&self, section: Section, n: usize) -> Vec<(f64, f64)> {
        let n = n.max(2);
        let lerp = |a: f64, b: f64, i: usize| a + (b - a) * (i as f64 / (n - 1) as f64);
        (0..n)
            .map(|i| match section {
                Section::TipArc => (self.ra, lerp(0.0, self.involute_at(self.u_tip).1, i)),
                Section::Involute => self.involute_at(lerp(self.u_tip, self.u_j, i)),
                // `sections()` asks for a fillet only where there is one. If it
                // ever asked otherwise, the flank's end is where the space
                // begins, which is the curve the fillet would have joined.
                Section::Trochoid => self.fillet.map_or_else(
                    || self.involute_at(self.u_j),
                    |f| self.trochoid_at(lerp(f.phi_j, f.phi_root, i)),
                ),
                Section::RootArc => (self.rf, lerp(self.space_starts_at(), self.half_pitch, i)),
            })
            .collect()
    }

    /// `(radius, angle)` from the tooth tip centre to mid tooth-space, spaced by
    /// arc length so no section is starved of points.
    #[must_use]
    pub fn half_profile(&self, n: usize) -> Vec<(f64, f64)> {
        crate::tooth::allocate_by_arc_length(&self.sections(), n, |s, k| self.sample_section(s, k))
    }

    /// The closed cross-section, counter-clockwise, `per_tooth` points a tooth.
    ///
    /// The outline of the *material's inner boundary*: a ring's teeth point
    /// inward, so this traces the bore, and whatever rim sits outside it is the
    /// designer's business rather than the tooth geometry's.
    ///
    /// # Errors
    ///
    /// [`crate::input::Refused::past_budget`], naming `teeth`, where the
    /// drawing is more than `budget` — refused from one tooth's points times
    /// the count before the teeth are drawn.
    pub fn profile(
        &self,
        per_tooth: usize,
        budget: crate::input::Budget,
    ) -> Result<Vec<[f64; 2]>, crate::input::Refused> {
        let half = self.half_profile((per_tooth / 2).max(8));

        let mut full: Vec<(f64, f64)> = half.iter().rev().map(|&(r, t)| (r, -t)).collect();
        // The tip centre is shared by the two halves; a half with no point
        // (a ring refused at the boundary, built anyway) draws nothing.
        full.extend(half.iter().skip(1));

        let z = self.teeth;
        // `u32` into `usize` on every target this builds for.
        let points = full.len().saturating_mul(z as usize).saturating_add(1);
        let mut out = Vec::new();
        budget.room(&mut out, points, "teeth", f64::from(z))?;
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
        Ok(out)
    }
}

/// **The fewest teeth this ring could have and keep its tip off its base
/// circle**: the count from which the cut no longer sets the tip on it
/// ([`key::CLAMP_RING_TIP_AT_BASE`]). The tip sits at `r − m(h_a − x)` and
/// the base circle at `r cos α_t`, with `r = z m / (2 cos β)`, so the tip
/// clears it from
///
/// ```text
/// z ≥ 2 (h_a − x) cos β / (1 − cos α_t)
/// ```
///
/// The module cancels and the shift does not: a shift moves the tool out,
/// which raises the tip, so fewer teeth clear. One where the addendum is no
/// taller than the shift, since every count clears then.
///
/// The base circle's bound only. A large ring's tooth can come to a point
/// first ([`key::CLAMP_RING_TIP_RAISED`]), and at these counts the flank
/// need not be generated to the tip ([`key::CLAMP_RING_FLANK_UNGENERATED`]).
/// `None` where no count a `u32` holds clears it, or the inputs are not
/// numbers.
#[must_use]
pub fn smallest_tooth_count(params: &GearParams) -> Option<u32> {
    let beta = params.helix_angle.to_radians();
    let alpha_t =
        crate::plane::transverse_pressure_angle(params.normal_pressure_angle_rad().0, beta);
    let fewest = (2.0 * (params.addendum - params.profile_shift) * beta.cos()
        / (1.0 - alpha_t.cos()))
    .ceil();
    if fewest <= 1.0 {
        return Some(1);
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    (fewest <= f64::from(u32::MAX)).then_some(fewest as u32)
}

/// The largest addendum, in modules, at which a ring's tooth is at least
/// `min_tip_width` wide at its tip: [`crate::auto::addendum_for_tip_width`]
/// read on a ring, whose tooth narrows inward, so that its tip is where it is
/// thinnest.
///
/// `None` where the width bounds nothing: the tooth is thinner than that even
/// at its root, so no tip is wide enough, or it is that wide at the lowest tip
/// the ring can have — the base circle, or where the tooth comes to a point —
/// so every tip is. Either way the addendum asked stands, and the ring's own
/// clamps say what else it meets.
#[must_use]
pub fn addendum_for_tip_width(ring: &Ring, min_tip_width: f64) -> Option<f64> {
    let width = |u: f64| 2.0 * ring.rb * f64::hypot(1.0, u) * (ring.psi_b + inv_from_roll(u));
    let u_root = crate::involute::roll_at_radius(ring.rf, ring.rb);
    let u_low = if ring.psi_b < 0.0 {
        crate::involute::inv_inverse(-ring.psi_b)?.tan()
    } else {
        0.0
    };
    if width(u_root) < min_tip_width || width(u_low) >= min_tip_width {
        return None;
    }
    let u = brent(|u| width(u) - min_tip_width, u_low, u_root, Tol::default())?;
    let ra = ring.rb * f64::hypot(1.0, u);
    Some((ring.r - ra) / ring.params.module + ring.params.profile_shift)
}

/// The least profile shift at which a ring's flank is generated all the way
/// to its tip ([`Ring::fully_generated`]): a ring's edge of undercut. Below it
/// the cutter's involute runs out before the ring's tip and the tip end of the
/// flank is not an involute, as a rack's tip cuts into an external tooth's
/// flank below its edge.
///
/// Searched between the tip on the base circle, where the flank is never
/// generated to the tip, and the tip on the pitch circle. `None` where the
/// flank is short of its tip at both ends alike, as
/// [`crate::auto::MinimumShift::with_cutter_radius`] is where no shift is on
/// the edge.
#[must_use]
pub fn minimum_profile_shift(params: &GearParams, cutter: &Cutter) -> Option<f64> {
    let short = |x: f64| {
        let g = Ring::cut_by(
            &GearParams {
                profile_shift: x,
                ..*params
            },
            cutter,
        );
        g.ra - g.generation_limit()
    };
    let g = Ring::cut_by(params, cutter);
    let m = params.module;
    let (lo, hi) = (params.addendum - (g.r - g.rb) / m, params.addendum);
    if short(hi) < 0.0 {
        return None;
    }
    brent(short, lo, hi, Tol::default())
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
/// nothing. It needs its own derivation and belongs with the planetary set
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
    /// The centre distance the pair was asked at, mm: its zero-backlash one —
    /// `r_ring − r_pinion` for a standard pair, larger when the ring is
    /// shifted further than its pinion — or, through [`mesh_at`], the one it
    /// runs at, a clearance *inside* that.
    pub centre_distance: f64,
    /// Operating pressure angle there, radians.
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
    /// **The tips foul away from the line of action.**
    ///
    /// A different question from the two above, and one they cannot see: those
    /// ask whether a tip reaches past a flank *where the teeth mesh*, and this
    /// asks whether two teeth try to occupy the same place somewhere else
    /// entirely. It is the condition that decides small tooth differences, where
    /// the tip circles cross far from the line of centres — 136° from it on a
    /// one-tooth pair — and the mesh itself is perfectly conjugate.
    ///
    /// Set where the tips overlap where their circles cross
    /// ([`Self::tip_margin`] negative) or the pinion's tip stands outside the
    /// ring's on the far side ([`Self::far_gap`] negative).
    pub tip_interference: bool,
    /// How much room the tips have where their circles cross, as an angle of
    /// **pinion** rotation, radians. Negative is the overlap.
    ///
    /// `None` when the ring's tip circle encloses the pinion's, which is the
    /// ordinary case: there is then no place for the tips to meet.
    pub tip_margin: Option<f64>,
    /// The room between the pinion's tip and the ring's on the side away from
    /// the mesh, mm: `R_a − r_a + a`.
    pub far_gap: f64,
}

/// Where two tip circles cross, and whether a tooth from each is there.
///
/// ```text
/// cos θ_pinion = (R_a² − r_a² − a²) / (2 a r_a)      from the pinion's centre
/// cos θ_ring   = (a² + R_a² − r_a²) / (2 a R_a)      from the ring's
/// ```
///
/// both measured from the line of centres on the **mesh** side. A tooth is
/// present at its own tip circle over its angular half-thickness there: the
/// pinion's tip land as its tooth was cut, and the ring's from the
/// half-thickness at its base circle along its involute.
///
/// # Why the two windows can be compared at all
///
/// They are windows on *different* wheels, at different angles, about different
/// centres — but the rolling locks them together. Take the instant a pinion
/// tooth is symmetric about the line of centres: the ring space it fills is
/// symmetric about it too, and that fixes both phases at once. Then as the
/// pinion turns by δ the pinion window slides by δ and the ring window by
/// `δ z_p/z_r`, so writing both as intervals of δ makes them comparable.
///
/// # That tooth, not the nearest one
///
/// The tooth that reaches the crossing is the one that was in that space at
/// the mesh, and it has to be between the same two ring teeth when it gets
/// there. Its offset from the ring tooth behind it, in pinion rotation, is
///
/// ```text
/// D = θ_p − (θ_r − π/z_r) z_r/z_p,      margin = min(D − H, 2π/z_p − D − H)
/// ```
///
/// with `H` the two half-thicknesses. Folded into one pitch instead, a tooth
/// that had overtaken a ring tooth inside the lens read as sitting in the next
/// space.
///
/// `None` when the ring's tip circle encloses the pinion's, or the two lie
/// apart: the circles do not cross. Where the pinion's encloses the ring's the
/// crossing has run round to the far side, and the clamp holds it at `π`,
/// continuous with the tangency it grew from.
fn tip_clearance(ring: &Ring, pinion: &Tooth, a: f64) -> Option<f64> {
    use std::f64::consts::{PI, TAU};
    let (r_a, big_r_a) = (pinion.ra, ring.ra);
    if a <= big_r_a - r_a || a >= big_r_a + r_a {
        return None;
    }
    let theta_p = ((big_r_a * big_r_a - r_a * r_a - a * a) / (2.0 * a * r_a))
        .clamp(-1.0, 1.0)
        .acos();
    let theta_r = ((a * a + big_r_a * big_r_a - r_a * r_a) / (2.0 * a * big_r_a))
        .clamp(-1.0, 1.0)
        .acos();

    // Half the angular thickness of each tooth at its own tip, never below
    // nought: a pointed tip is a point. The pinion's is its own tip land,
    // which is nought by construction where the flanks meet and the
    // fillet's where its tip is on the fillet; the involute continued to the
    // tip radius read that last one wide. A ring's tip is on its involute,
    // whose angle it gains outward from its half-thickness at the base
    // circle.
    let half_p = pinion.theta_a.max(0.0);
    let half_r = (ring.psi_b + inv((ring.rb / big_r_a).clamp(-1.0, 1.0).acos())).max(0.0);

    let (z_p, z_r) = (f64::from(pinion.params.teeth), f64::from(ring.teeth));
    let ratio = z_r / z_p;
    let d = theta_p - (theta_r - PI / z_r) * ratio;
    let h = half_p + half_r * ratio;
    Some((d - h).min(TAU / z_p - d - h))
}

/// Mesh a ring with an external pinion at their zero-backlash centre distance.
///
/// Shifts on either member are carried, through the same
/// [`crate::mesh::operating_geometry`] the external mesh
/// uses: the pinion is member 1 and the ring member 2, so the sums are
/// `z_p − z_r` and `x_p − x_r`. A standard pair is the value of that at zero,
/// where `α_w = α_t` and `a = r_ring − r_pinion` — not a separate case.
///
/// **The shifts enter through the space, not the tooth.** A ring's `x_thick`
/// widens its space, so a ring shifted further than its pinion opens the mesh
/// out and the pinion sits further from the ring's axis. That is why the sum is
/// a difference and why it is this way round; see [`Ring::cut_by`].
///
/// # Errors
///
/// `None` if the pair cannot mesh — different modules or pressure angles, a
/// pinion no smaller than the ring, a geometry that never reaches contact, or
/// shifts that drive the operating pressure angle out of the involute domain.
#[must_use]
pub fn mesh_with(ring: &Ring, pinion: &Tooth) -> Option<RingMesh> {
    let (a_ref, zero_backlash) = reference_geometry(ring, pinion)?;
    described_at(ring, pinion, a_ref, zero_backlash)
}

/// [`mesh_with`], asked at the centre distance the pair **runs** at.
///
/// Every verdict here moves with the distance — the operating angle, the two
/// contact radii, both interference conditions and the room the tips have —
/// and a pair that assembles with a clearance is owed them where it
/// assembled, which for an internal pair is a clearance *inside* its
/// zero-backlash distance ([`crate::mesh::MeshKind::run_at`]). Read at zero
/// backlash instead, the tip room came out looser than the pair has: the
/// shipped hula stage opened its crank until that margin was exactly nought,
/// and then ran a clearance inside it.
#[must_use]
pub fn mesh_at(ring: &Ring, pinion: &Tooth, running: f64) -> Option<RingMesh> {
    let (a_ref, _) = reference_geometry(ring, pinion)?;
    described_at(ring, pinion, a_ref, running)
}

/// Whether the two can mesh at all, and if so their reference and
/// zero-backlash distances, mm.
fn reference_geometry(ring: &Ring, pinion: &Tooth) -> Option<(f64, f64)> {
    // The *transverse* rack, since a ring carries its own `mt` and `alpha_t`
    // rather than a `GearParams`. Same tolerance as `GearParams::same_rack_as`,
    // from the same place, so the two cannot drift apart again.
    let tol = crate::params::compat::SAME_RACK;
    if (ring.mt - pinion.mt).abs() > tol || (ring.alpha_t - pinion.alpha_t).abs() > tol {
        return None;
    }
    if pinion.params.teeth >= ring.teeth {
        return None;
    }
    let sum_z = f64::from(pinion.params.teeth) - f64::from(ring.teeth);
    let sum_x = (pinion.params.profile_shift + pinion.params.thickness_shift()) - ring.x_thick;
    let (_, a_ref, zero_backlash) =
        crate::mesh::operating_geometry(ring.mt, ring.alpha_t, ring.alpha_n, sum_z, sum_x)?;
    (zero_backlash.is_finite() && zero_backlash > 0.0).then_some((a_ref, zero_backlash))
}

/// The pair at a centre distance: the base cylinders are the gears, so the
/// line of action turns to keep touching them — `cos α' = a_ref cos α_t / a'`,
/// the relation [`crate::mesh::Mesh::at`] reads — and every verdict follows.
fn described_at(ring: &Ring, pinion: &Tooth, a_ref: f64, centre_distance: f64) -> Option<RingMesh> {
    let cos_alpha_w = a_ref * ring.alpha_t.cos() / centre_distance;
    if !(-1.0..=1.0).contains(&cos_alpha_w) {
        return None;
    }
    let alpha_w = cos_alpha_w.acos();
    let along = centre_distance * alpha_w.sin();

    // **The relation, both ways round — and it is the general one.** A ring's
    // base radius enters signed, which is the only thing that distinguishes this
    // arrangement from an external pair, and
    // [`crate::mesh::conjugate_radius`] does the rest. Reading it backwards can
    // fail, and the failure is the answer rather than an error: it means the
    // ring's tip would have to touch the pinion *inside its base circle*, where
    // no involute exists. That is involute interference in its strongest form,
    // and it is reported as a finding rather than as "this pair cannot be
    // described".
    let rb = [pinion.rb, -ring.rb];
    let ring_contact_at_pinion_tip =
        crate::mesh::conjugate_radius(rb, alpha_w, crate::mesh::MeshSide::Second, pinion.ra)?;
    let reachable =
        crate::mesh::conjugate_radius(rb, alpha_w, crate::mesh::MeshSide::First, ring.ra);
    let pinion_contact_at_ring_tip = reachable.unwrap_or(pinion.rb);

    // Contact ratio, from docs/reference.md#path-of-contact-and-contact-ratio's internal form. The path runs from where
    // the ring's tip engages to where the pinion's does.
    let base_pitch = crate::plane::base_pitch(ring.mt, ring.alpha_t);
    let path = ((pinion.ra * pinion.ra - pinion.rb * pinion.rb)
        .max(0.0)
        .sqrt()
        - (ring.ra * ring.ra - ring.rb * ring.rb).max(0.0).sqrt()
        + along)
        .max(0.0);
    let contact_ratio = path / base_pitch;

    // **The classical pair, read off the general condition.** A ring's flank
    // ends where its fillet begins and a pinion's ends where its own does, and a
    // tip reaching past either is the foul — which is
    // [`crate::mesh::Mesh::flank_interference`] asked of each member in turn.
    // The names are the literature's: interference *of the ring's flank* is what
    // it calls trochoid, and *of the pinion's* involute.
    //
    // Written through the general form rather than beside it so the two cannot
    // come apart; a test holds them to the same numbers.
    let ring_form = ring.involute_at(ring.u_j).0;
    let trochoid_interference = ring_contact_at_pinion_tip > ring_form;
    let involute_interference = reachable.is_none() || pinion_contact_at_ring_tip < pinion.r_j;
    let tip_margin = tip_clearance(ring, pinion, centre_distance);
    let far_gap = ring.ra - pinion.ra + centre_distance;

    Some(RingMesh {
        centre_distance,
        alpha_w,
        contact_ratio,
        ring_contact_at_pinion_tip,
        pinion_contact_at_ring_tip,
        trochoid_interference,
        involute_interference,
        tip_interference: tip_margin.is_some_and(|m| m < 0.0) || far_gap < 0.0,
        tip_margin,
        far_gap,
    })
}

/// **An independent roll of an internal pair**, for the tests: nothing here
/// shares code with the tip margin it checks.
#[cfg(test)]
pub(crate) mod roll {
    use super::Ring;
    use crate::tooth::Tooth;
    use std::f64::consts::{PI, TAU};

    /// Points along the pinion's half outline, tip centre to mid-space: the
    /// tip land and flank, which are what can reach ring material, take
    /// about half of them by arc length.
    const OUTLINE_POINTS: usize = 160;

    /// The deepest a pinion point sits inside ring material over one pinion
    /// pitch, negated, mm: zero where the teeth only touch, negative where
    /// they foul. The pinion sits in the middle of its play at the mesh.
    ///
    /// The pinion is its own outline, every tooth — [`Tooth::half_profile`]
    /// sampled densely, so a tip on the fillet, a pointed tip and a severed
    /// one are the shape they are; the ring is involute teeth and a tip
    /// circle. A point is in ring material outside the ring's tip circle and
    /// within its tooth's half-angle there, and how deep is the lesser of its
    /// height above the tip circle and its distance from the flank — `r_b`
    /// times the angle between the two involutes, since involutes of one base
    /// circle are parallel. Past the ring's junction its fillet only adds
    /// material, so this reads a foul there as no deeper than it is.
    pub(crate) fn rolled(ring: &Ring, pinion: &Tooth, a: f64) -> f64 {
        let (zp, zr) = (f64::from(pinion.params.teeth), f64::from(ring.teeth));
        let inv_at = |rb: f64, rho: f64| {
            let al = (rb / rho).min(1.0).acos();
            al.tan() - al
        };
        let (radii, angles) = pinion.half_profile(OUTLINE_POINTS);
        let outline: Vec<(f64, f64)> = radii
            .iter()
            .zip(&angles)
            .flat_map(|(&r, &h)| [(r, h), (r, -h)])
            .collect();
        // Ring teeth centred half a ring pitch off `+y`, where its space is.
        let depth = |x: f64, y: f64| -> f64 {
            let rho = x.hypot(y);
            if rho <= ring.ra {
                return 0.0;
            }
            let pitch = TAU / zr;
            let th = x.atan2(y) - PI / zr;
            let off = (th - pitch * (th / pitch).round()).abs();
            let half = ring.psi_b + inv_at(ring.rb, rho);
            (rho - ring.ra).min(ring.rb * (half - off)).max(0.0)
        };
        // Pinion centre at `(0, a)`, its tooth on `+y` at the mesh; both turn
        // clockwise, the ring `z_p/z_r` as far, and `phase` more.
        let deepest = |phase: f64, turn: f64| -> f64 {
            let ring_turn = turn * zp / zr + phase;
            let mut worst = 0.0_f64;
            for k in 0..pinion.params.teeth {
                let t = turn + TAU * f64::from(k) / zp;
                let tip = (pinion.ra * t.sin(), a + pinion.ra * t.cos());
                if tip.0.hypot(tip.1) < ring.ra - 2.0 * ring.mt {
                    continue;
                }
                for &(rho, h) in &outline {
                    let (x, y) = (rho * (h + t).sin(), a + rho * (h + t).cos());
                    let (r, th) = (x.hypot(y), x.atan2(y) - ring_turn);
                    worst = worst.max(depth(r * th.sin(), r * th.cos()));
                }
            }
            worst
        };
        // The assembly phase: the middle of the play at the mesh, or where
        // the teeth sit least deep in each other there if they have none.
        let phases: Vec<f64> = (-40..=40).map(|j| PI / zr * f64::from(j) / 80.0).collect();
        let at_mesh: Vec<f64> = phases.iter().map(|&p| deepest(p, 0.0)).collect();
        let least = at_mesh.iter().copied().fold(f64::INFINITY, f64::min);
        let free: Vec<f64> = phases
            .iter()
            .zip(&at_mesh)
            .filter(|(_, &d)| d <= least + 1e-12)
            .map(|(&p, _)| p)
            .collect();
        let phase = 0.5 * (free[0] + free[free.len() - 1]);
        let step = TAU / zp / 120.0;
        (0..120)
            .map(|i| -deepest(phase, step * f64::from(i)))
            .fold(0.0, f64::min)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// **The general reading and the ring's own reading are the same number.**
    ///
    /// `mesh_with` computes where a tip touches from the internal relation
    /// written out here; `Mesh::contact_radius_at` computes it from the signed
    /// one that serves both arrangements. They are the same relation, so they
    /// have to agree to the bit — and if they do, the general form can replace
    /// the special case rather than sit beside it.
    ///
    /// Asserted on **both** readings, because the two directions exercise
    /// different halves of the sign: the pinion's tip against the ring uses
    /// gear 2's negative `ρ`, and the ring's tip against the pinion is the one
    /// that can fail to reach an involute at all.
    #[test]
    fn the_general_contact_reading_reproduces_the_internal_one() {
        let mut checked = 0u32;
        for z_ring in [40u32, 52, 60, 84] {
            for z_pinion in [17u32, 20, 24, 31] {
                for x in [-0.3_f64, 0.0, 0.4] {
                    let ring = Ring::cut_by(
                        &GearParams {
                            teeth: z_ring,
                            profile_shift: x,
                            ..Default::default()
                        },
                        &Cutter::default(),
                    );
                    let pinion = crate::Tooth::new(GearParams {
                        teeth: z_pinion,
                        ..Default::default()
                    });
                    let Some(special) = mesh_with(&ring, &pinion) else {
                        continue;
                    };
                    // The same pair as a `Mesh`, which is how every stage builds
                    // its internal mesh.
                    let ring_as_gear = crate::Tooth::new(GearParams {
                        teeth: z_ring,
                        profile_shift: x,
                        ..Default::default()
                    });
                    let Ok(general) =
                        crate::mesh::Mesh::new(&pinion, &ring_as_gear, MeshKind::Internal)
                    else {
                        continue;
                    };
                    assert!(
                        (general.a_w - special.centre_distance).abs() < 1e-9,
                        "the two do not even describe the same pair"
                    );
                    checked += 1;

                    // The ring radius the pinion's tip touches.
                    let on_ring = general
                        .contact_radius_at(crate::mesh::MeshSide::Second, pinion.ra)
                        .expect("a ring's flank always reaches outwards");
                    assert!(
                        (on_ring - special.ring_contact_at_pinion_tip).abs() < 1e-9,
                        "{z_ring}/{z_pinion} x={x}: general {on_ring} vs internal {}",
                        special.ring_contact_at_pinion_tip
                    );

                    // ...and the pinion radius the ring's tip touches, which is
                    // the reading that can have no answer.
                    let on_pinion =
                        general.contact_radius_at(crate::mesh::MeshSide::First, ring.ra);
                    match on_pinion {
                        Some(r) => assert!(
                            (r - special.pinion_contact_at_ring_tip).abs() < 1e-9,
                            "{z_ring}/{z_pinion} x={x}: general {r} vs internal {}",
                            special.pinion_contact_at_ring_tip
                        ),
                        None => assert!(
                            special.involute_interference,
                            "{z_ring}/{z_pinion} x={x}: the general form found no involute \
                             where the internal one did"
                        ),
                    }
                }
            }
        }
        assert!(checked >= 30, "only {checked} pairs were comparable");
    }
    use crate::tooth::Tooth;

    fn ring(teeth: u32) -> Ring {
        Ring::cut_by(
            &GearParams {
                teeth,
                ..Default::default()
            },
            &Cutter::default(),
        )
    }

    /// The fillet of a ring that is supposed to have one.
    ///
    /// Tests that walk the fillet are asserting something about a curve; if it
    /// is missing entirely they should say so loudly rather than quietly assert
    /// nothing, which is what reading two zeros off the ring used to do.
    fn fillet_of(g: &Ring) -> Fillet {
        g.fillet
            .expect("this ring's cutter is supposed to generate a fillet")
    }

    /// **The smallest ring is a function of the design, not a number.**
    ///
    /// A ring's tip sits at `r − m(h_a − x)` and its base circle at
    /// `r cos α_t`, so the tip clears the base circle only while
    ///
    /// ```text
    /// z ≥ 2 (h_a − x) cos β / (1 − cos α_t)
    /// ```
    ///
    /// Four things move it and one does not. A **shallower tooth** allows far
    /// fewer teeth; a **positive shift** allows fewer, since it moves the tool
    /// and so the tip outward; a **larger pressure angle** allows fewer,
    /// because the base circle drops away from the pitch circle; a **helix**
    /// allows fewer, since the transverse module grows with it. The **module
    /// cancels**, which is right — this is a statement about tooth counts.
    ///
    /// The familiar "internal gears need at least about 34 teeth" is the
    /// single row of this table at a full addendum, no shift and 20°, and
    /// quoting it as a rule would have been wrong for every other row.
    #[test]
    fn the_smallest_ring_follows_the_design_rather_than_a_rule_of_thumb() {
        let cases = [
            // addendum, shift, α_n°, β°, the count the geometry gives
            (1.0, 0.0, 20.0, 0.0, 34u32),
            (0.8, 0.0, 20.0, 0.0, 27),
            (0.6, 0.0, 20.0, 0.0, 20),
            (1.0, 0.0, 25.0, 0.0, 22),
            (1.0, 0.0, 14.5, 0.0, 63),
            (1.0, 0.0, 20.0, 30.0, 23),
            (1.0, 0.5, 20.0, 0.0, 17),
            (1.0, -0.3, 20.0, 0.0, 44),
            (1.0, 0.8, 20.0, 0.0, 7),
            (0.8, 0.8, 20.0, 0.0, 1),
        ];
        for (addendum, profile_shift, pressure_angle, helix_angle, expected) in cases {
            let p = GearParams {
                addendum,
                profile_shift,
                pressure_angle,
                helix_angle,
                ..Default::default()
            };
            assert_eq!(
                smallest_tooth_count(&p),
                Some(expected),
                "h_a={addendum} x={profile_shift} α={pressure_angle} β={helix_angle}"
            );
        }
    }

    /// **The fewest ring teeth is where the cut stops setting the tip on the
    /// base circle**: at the count, no base-circle clamp; one fewer, the
    /// clamp. Over shift, addendum, pressure angle and helix, at counts
    /// below the thin-tooth end, whose clamp would stand in for this one.
    #[test]
    fn the_fewest_ring_teeth_is_where_the_cut_stops_clamping_the_tip() {
        let at_base = |p: GearParams| {
            Ring::cut_by(&p, &Cutter::default())
                .clamps
                .iter()
                .any(|c| c.is(key::CLAMP_RING_TIP_AT_BASE))
        };
        let mut below = 0;
        for profile_shift in [-0.5, -0.3, 0.0, 0.3, 0.5, 0.8] {
            for addendum in [0.6, 0.8, 1.0] {
                for pressure_angle in [14.5, 20.0, 25.0] {
                    for helix_angle in [0.0, 30.0] {
                        let p = GearParams {
                            profile_shift,
                            addendum,
                            pressure_angle,
                            helix_angle,
                            ..Default::default()
                        };
                        let n = smallest_tooth_count(&p).expect("a count");
                        let tag = format!(
                            "x={profile_shift} h_a={addendum} α={pressure_angle} β={helix_angle}: n={n}"
                        );
                        assert!(!at_base(GearParams { teeth: n, ..p }), "{tag}");
                        if n > 1 {
                            assert!(at_base(GearParams { teeth: n - 1, ..p }), "{tag}");
                            below += 1;
                        }
                    }
                }
            }
        }
        // All 108 but the 12 at x = 0.8 with h_a ≤ 0.8, whose every count clears.
        assert_eq!(below, 96);
        // The floor on the pressure angle is the cut's: at nought the count is
        // the one the floor gives, not a division by nought.
        let flat = GearParams {
            pressure_angle: 0.0,
            ..Default::default()
        };
        let floor = GearParams {
            pressure_angle: guard::MIN_PRESSURE_ANGLE_DEG,
            ..Default::default()
        };
        assert_eq!(smallest_tooth_count(&flat), smallest_tooth_count(&floor));
        assert!(smallest_tooth_count(&flat).is_some());
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
                g.clamps.iter().all(|c| c.is(key::CLAMP_RING_FULLY_FILLETED)
                    || c.is(key::CLAMP_RING_FLANK_UNGENERATED)),
                "z={teeth} should generate cleanly: {:?}",
                g.clamps
            );
            let f = g.fillet.expect("z={teeth} is cut with a fillet");
            let (r_flank, a_flank) = g.involute_at(g.u_j);
            let (r_fillet, a_fillet) = g.trochoid_at(f.phi_j);
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
        let f = fillet_of(&g);
        let mut radius = g.ra;
        let mut angle = 0.0_f64;
        for i in 0..=40 {
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
            let t = i as f64 / 40.0;
            let (r, a) = g.trochoid_at(f.phi_j + (f.phi_root - f.phi_j) * t);
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
        // This ring keeps a root arc: a fully filleted root is a case no
        // fixture here reaches (ring#5, T05.2's), so it is said rather than
        // tested in a branch that never runs.
        assert_eq!(f.phi_root, 0.0, "the fillet ends on the root circle");
        assert!(
            (radius - g.rf).abs() < 1e-9,
            "fillet reached {radius}, root {}",
            g.rf
        );
    }

    /// A bigger cutter takes more out: its tip corner is flatter, so the fillet
    /// it leaves reaches the flank higher up.
    #[test]
    fn a_larger_cutter_moves_the_junction() {
        let small = Ring::cut_by(
            &GearParams {
                teeth: 60,
                ..Default::default()
            },
            &Cutter {
                teeth: 15,
                ..Cutter::default()
            },
        );
        let large = Ring::cut_by(
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
                g.clamps.iter().all(|c| c.is(key::CLAMP_RING_FULLY_FILLETED)
                    || c.is(key::CLAMP_RING_FLANK_UNGENERATED)),
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
            let outline = g.profile(120, crate::input::Budget::DEFAULT).unwrap();
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
        let external = Tooth::new(GearParams {
            teeth: 60,
            ..Default::default()
        });
        let ring_max = g
            .profile(120, crate::input::Budget::DEFAULT)
            .unwrap()
            .iter()
            .map(|[x, y]| f64::hypot(*x, *y))
            .fold(0.0_f64, f64::max);
        let ext_max = crate::gear::Gear::new(external.params)
            .profile(120, crate::input::Budget::DEFAULT)
            .unwrap()
            .iter()
            .map(|[x, y]| f64::hypot(*x, *y))
            .fold(0.0_f64, f64::max);
        // The furthest point is where the fillet stops: on the root circle,
        // since this ring keeps a root arc (a fully filleted root is no
        // fixture's here; ring#5, T05.2).
        assert_eq!(fillet_of(&g).phi_root, 0.0, "a root arc");
        let deepest = g.trochoid_at(fillet_of(&g).phi_root).0;
        assert!(
            (ring_max - deepest).abs() < 1e-9,
            "a ring's furthest point is where its fillet ends: {ring_max} against {deepest}"
        );
        assert!(
            deepest <= g.rf + 1e-12,
            "and that cannot be beyond the root circle"
        );
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
            let g = Ring::cut_by(
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
    ///
    /// **The thickness modification is swept here too, and that is the point.**
    /// It was not, and a ring whose `k` left 1 was cut by a tool placed for a
    /// ring it was not making: `k` had been taken into the *centre distance*
    /// rather than into the cutter's tooth, so the root diameter grew and shrank
    /// with a control that is thickness-only by definition. Every gate on the
    /// cut swept the shift and left `k` at its default — *an axis nobody turns
    /// is an axis nobody tests* (docs/corrections.md), met in the module that
    /// records the sentence.
    #[test]
    fn a_shifted_ring_is_the_shape_its_cutter_leaves() {
        for teeth in [43u32, 60] {
            for x in [-0.4, -0.25, -0.1, 0.0, 0.1, 0.25, 0.5] {
                for k in [0.8, 1.0, 1.2] {
                    let g = Ring::cut_by(
                        &GearParams {
                            teeth,
                            profile_shift: x,
                            thickness_mod: k,
                            ..Default::default()
                        },
                        &Cutter::default(),
                    );
                    let report = crate::verify::check_ring_cut(&g, 400, 4_000);
                    assert!(
                        report.worst_distance < 5e-3,
                        "z={teeth} x={x} k={k}: cut and profile differ by {} mm",
                        report.worst_distance
                    );
                }
            }
        }
    }

    /// **A ring's drawn profile never crosses its own space centreline.**
    ///
    /// The half-profile is mirrored to make a tooth, so a point past the
    /// centreline comes back through its own reflection: the outline stops being
    /// a simple closed curve and draws as an inverted spur at the bottom of
    /// every space — geometry that would go into a DXF and that no tool can
    /// leave. The flank ran to the root circle whether or not the space had
    /// already closed. An external gear did the same when its rack's tooth came
    /// to a point before the depth (`geometry_laws::the_outline_is_a_simple_closed_curve`),
    /// and both now stop the root where the space closes, as `clamp.space_closed`.
    ///
    /// The case that reaches it is a **thick** ring tooth — a low thickness
    /// modification — where the cutter's own tooth comes to a point, no fillet
    /// is generated, and nothing truncates the flank. Swept here across `k`, `x`
    /// and the tooth count together, because the three move the same crossing.
    ///
    /// Run against the code this replaced it fails at `k = 0.6` with 14 of 398
    /// points past the centreline, and at `k = 0.4` with 45.
    #[test]
    fn a_rings_profile_stays_inside_its_own_space() {
        for teeth in [20u32, 43, 90, 150] {
            for x in [-0.3, 0.0, 0.4] {
                for k in [0.3, 0.5, 0.7, 1.0, 1.3, 1.6] {
                    let g = Ring::cut_by(
                        &GearParams {
                            teeth,
                            profile_shift: x,
                            thickness_mod: k,
                            ..Default::default()
                        },
                        &Cutter::default(),
                    );
                    let points = g.half_profile(400);
                    assert!(!points.is_empty(), "z={teeth} x={x} k={k}: no profile");
                    for &(radius, theta) in &points {
                        assert!(
                            theta <= g.half_pitch + 1e-12,
                            "z={teeth} x={x} k={k}: a point at r={radius} sits at {theta}, \
                             past the space centreline at {}",
                            g.half_pitch
                        );
                        assert!(
                            theta >= -1e-12 && radius.is_finite(),
                            "z={teeth} x={x} k={k}: r={radius} theta={theta}"
                        );
                    }
                    // ...and the truncation is reported wherever it happened,
                    // rather than the part quietly coming back shorter.
                    let closed = g.clamps.iter().any(|n| n.is(key::CLAMP_SPACE_CLOSED));
                    let reaches = g.involute_at(g.u_j).1;
                    assert_eq!(
                        closed,
                        (reaches - g.half_pitch).abs() < 1e-9 && g.fillet.is_none(),
                        "z={teeth} x={x} k={k}: the space closed at {reaches} against a \
                         half pitch of {}, and the note {}",
                        g.half_pitch,
                        if closed { "fired" } else { "did not" }
                    );
                }
            }
        }
    }

    /// **A thickness modification changes a thickness, and nothing radial.**
    ///
    /// The rule the whole crate is built on: *radial* quantities take plain `x`
    /// and *thickness* quantities take `x + x_s`
    /// (docs/reference.md#tooth-thickness-and-its-equivalent-shift). A ring's
    /// root radius is radial — it is where the cutter's tip reaches — so `k` may
    /// not move it, any more than it moves an external gear's.
    ///
    /// Asserted **exactly**, because it is an invariant rather than a trend: the
    /// plunge does not depend on `k`, so neither does anything the plunge sets.
    /// Run against the code this replaced it fails at once — `rf` swept
    /// 22.373 → 23.309 mm across `k = 0.85 … 1.3` on a 43-tooth ring, and at
    /// `k = 0.7` the pair left the involute domain and was quietly cut at
    /// reference centres instead.
    #[test]
    fn a_thickness_modification_moves_no_radius_on_a_ring() {
        for teeth in [43u32, 60, 90] {
            for x in [-0.25, 0.0, 0.3] {
                let at = |k: f64| {
                    Ring::cut_by(
                        &GearParams {
                            teeth,
                            profile_shift: x,
                            thickness_mod: k,
                            ..Default::default()
                        },
                        &Cutter::default(),
                    )
                };
                let plain = at(1.0);
                let closed = |g: &Ring| g.clamps.iter().any(|n| n.is(key::CLAMP_SPACE_CLOSED));
                for k in [0.7, 0.85, 1.15, 1.3] {
                    let g = at(k);
                    // **Where the space itself closes, the root is truncated by
                    // the profile rather than placed by the tool**, and that
                    // limit *is* a function of `k` — a thicker tooth closes its
                    // space sooner. It is a different guard, with its own note
                    // and its own test; what it may never do is let the root out
                    // *past* where the tool put it.
                    if closed(&g) || closed(&plain) {
                        assert!(
                            g.rf < plain.rf.max(g.rf) + 1e-12 && g.rf > 0.0,
                            "z={teeth} x={x} k={k}: a closed space grew the root to {}",
                            g.rf
                        );
                        continue;
                    }
                    for (name, got, want) in [
                        ("root radius", g.rf, plain.rf),
                        ("tip radius", g.ra, plain.ra),
                    ] {
                        assert_eq!(
                            got.to_bits(),
                            want.to_bits(),
                            "z={teeth} x={x} k={k}: {name} moved from {want} to {got}"
                        );
                    }
                    // **A `k` far enough from 1 asks for a tool that does not
                    // exist**, and that is a real answer rather than a hole in
                    // this one: thickening a ring's tooth thins the cutter's,
                    // and a cutter thin enough comes to a point before it
                    // reaches its own tip. The part then has no fillet and says
                    // so, which is where `cut` holds a placeholder rather than a
                    // placement — so the plunge is compared only where there is
                    // a tool to place. The radii above are compared regardless,
                    // because they are settled before the tool is built and are
                    // what the rule is actually about.
                    let no_tool = g
                        .clamps
                        .iter()
                        .any(|n| n.is(key::CLAMP_CUTTER_NO_TIP_CORNER));
                    if !no_tool {
                        assert_eq!(
                            g.cut.centre_distance.to_bits(),
                            plain.cut.centre_distance.to_bits(),
                            "z={teeth} x={x} k={k}: the cutter was plunged to {} rather \
                             than {}",
                            g.cut.centre_distance,
                            plain.cut.centre_distance
                        );
                    }
                    // ...while the thing it *is* about did change, or the test
                    // above would be satisfied by a control that does nothing.
                    assert!(
                        (g.psi_b - plain.psi_b).abs() > 1e-6,
                        "z={teeth} x={x} k={k}: the tooth did not change thickness"
                    );
                    // **The tool carries the thickness, the plunge carries the
                    // shift, and this is that sentence as arithmetic.** The
                    // cutter's tooth fills the ring's space but for the radial
                    // shift, which is delivered by moving the tool rather than
                    // by shaping it:
                    //
                    // ```text
                    // e_ring − s_cutter = 2 m_t x tan α_n
                    // ```
                    //
                    // — a difference in `x` alone, with no `k` in it at any `k`.
                    // At `x = 0` the two are simply equal, which is what
                    // generation means when the tool is not displaced.
                    let tooth = 2.0 * g.r * (g.psi_b + crate::involute::inv(g.alpha_t));
                    let space = std::f64::consts::PI * g.mt - tooth;
                    let from_shift = 2.0 * g.mt * x * g.alpha_n.tan();
                    assert!(
                        (space - g.cut.cutter_tooth - from_shift).abs() < 1e-12 * g.mt,
                        "z={teeth} x={x} k={k}: the space {space} less the cutter's tooth \
                         {} is {}, where only the shift's {from_shift} should separate them",
                        g.cut.cutter_tooth,
                        space - g.cut.cutter_tooth
                    );
                }
            }
        }
    }

    /// **...and the check can tell when it is not.**
    ///
    /// The point of this test is not the ring, it is the gate. The previous cut
    /// simulation derived the cutter's tooth from the ring's — the same inference
    /// the model made — so it agreed to 2.7 µm on a ring whose cutter was 0.44 mm
    /// out of place, and reported nothing. That is the docs/corrections.md trap exactly: a check
    /// that cannot distinguish two cases is not evidence for either.
    ///
    /// So place the cutter where the old model put it, at reference centres, and
    /// require the gate to *fail*. It comes out 13–66× the noise floor.
    #[test]
    fn a_cutter_at_the_wrong_centre_distance_is_visible_to_the_gate() {
        for x in [0.1, 0.25, 0.5, -0.25] {
            let good = Ring::cut_by(
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
            Ring::cut_by(
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
            small
                .clamps
                .iter()
                .any(|c| c.is(key::CLAMP_RING_FLANK_UNGENERATED)),
            "{:?}",
            small.clamps
        );
    }

    fn pinion(teeth: u32) -> Tooth {
        Tooth::new(GearParams {
            teeth,
            ..Default::default()
        })
    }

    /// **A tip fouling is not a mesh fault, and neither sees the other.**
    ///
    /// A well separated pair's tip circles do not cross at all, so there is
    /// nowhere for its tips to meet however its flanks behave; a close-count
    /// pair's do, far from the line of centres, where the mesh is perfectly
    /// conjugate and both of the other conditions are content.
    ///
    /// The figures are the ones a roll of the outlines gives
    /// (`gear-cli meshsweep`): the pairs called clear here measure exactly
    /// touching, and the ones called fouled measure 0.10 and 0.21 mm of overlap
    /// at 64° and 77° from the line of centres — which is where their tip
    /// circles cross and nowhere near their meshes.
    #[test]
    fn tips_foul_where_the_tip_circles_cross() {
        for (r, p) in [(60, 20), (40, 20), (100, 30)] {
            let m = mesh_with(&ring(r), &pinion(p)).unwrap();
            assert!(
                !m.tip_interference,
                "z{r}/z{p} should have room at its tips, margin {:?}",
                m.tip_margin
            );
        }
        for (r, p) in [(40, 34), (30, 26)] {
            let m = mesh_with(&ring(r), &pinion(p)).unwrap();
            assert!(
                m.tip_interference,
                "z{r}/z{p} tips should foul, margin {:?}",
                m.tip_margin
            );
        }
    }

    /// **The room at the tips grows as the pair is opened out.**
    ///
    /// Shifting the ring outward moves its tip away from the pinion's, so the
    /// margin rises monotonically — which is what lets a solve target it.
    #[test]
    fn the_tip_margin_rises_as_the_ring_is_shifted_out() {
        let mut last = f64::NEG_INFINITY;
        for step in 0..12 {
            let x = f64::from(step) * 0.05;
            let r = Ring::cut_by(
                &GearParams {
                    teeth: 30,
                    profile_shift: x,
                    addendum: 0.8,
                    ..Default::default()
                },
                &Cutter {
                    teeth: 25,
                    ..Cutter::default()
                },
            );
            let margin = mesh_with(&r, &pinion(26)).unwrap().tip_margin.unwrap();
            assert!(
                margin > last,
                "the margin fell from {last} to {margin} at x {x}"
            );
            last = margin;
        }
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
            let g = Ring::cut_by(&p(zr, xr), &Cutter::default());
            let pin = Tooth::new(p(zp, xp));
            let mesh =
                crate::mesh::Mesh::new(&pin, &Tooth::new(p(zr, xr)), MeshKind::Internal).unwrap();
            let path = crate::contact::ContactPath::new(&pin, g.flank_ends(), &mesh).unwrap();
            let own = mesh_with(&g, &pin).unwrap();

            assert!(
                (path.alpha_w - own.alpha_w).abs() < 1e-12,
                "z={zr}/{zp} x={xr}/{xp}: alpha_w {} vs {}",
                path.alpha_w,
                own.alpha_w
            );
            // `mesh_with`'s ratio is the tips' path; the general one is cut
            // where a tip reaches past a usable flank, which a standard ring
            // does (`mesh.flank_interference`).
            let [tip_approach, tip_recess] = path.tip_limited;
            let tips = (tip_approach + tip_recess) / path.base_pitch;
            assert!(
                (tips - own.contact_ratio).abs() < 1e-12,
                "z={zr}/{zp} x={xr}/{xp}: contact ratio {tips} vs {}",
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
            let pin = Tooth::new(p(zp));
            let wheel = Tooth::new(p(zr));
            let g = Ring::cut_by(&p(zr), &Cutter::default());
            let load = Load::new(2.0, 10.0);
            let e_star = 113_000.0;

            let internal = {
                let m = Mesh::new(&pin, &wheel, MeshKind::Internal).unwrap();
                let path = ContactPath::new(&pin, g.flank_ends(), &m).unwrap();
                contact_stress(&path, &m, &pin, PARALLEL_AXES, &load, e_star).unwrap()
            };
            let external = {
                let m = Mesh::new(&pin, &wheel, MeshKind::External).unwrap();
                let path = ContactPath::new(&pin, wheel.flank_ends(), &m).unwrap();
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
            let g = Ring::cut_by(
                &GearParams {
                    teeth: zr,
                    profile_shift: xr,
                    ..Default::default()
                },
                &Cutter::default(),
            );
            let p = Tooth::new(GearParams {
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

            let u = crate::involute::roll_at_radius(rp, p.rb);
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
    /// known — the docs/corrections.md rule about not predicting a threshold the computation can
    /// find.
    #[test]
    fn shifting_the_ring_moves_the_pinion_outward() {
        let mut last = f64::NEG_INFINITY;
        for xr in [-0.3, -0.15, 0.0, 0.15, 0.3, 0.45] {
            let g = Ring::cut_by(
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
        let both = Ring::cut_by(
            &GearParams {
                teeth: 60,
                profile_shift: 0.3,
                ..Default::default()
            },
            &Cutter::default(),
        );
        let shifted_pinion = Tooth::new(GearParams {
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
            let external = crate::contact::ContactPath::new(&a, b.flank_ends(), &m)
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

        let shortened = Ring::cut_by(
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
            let p = Tooth::new(GearParams {
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
                &Tooth::new(GearParams {
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
            // z = 31 asks for a tip inside the base circle and is set on it.
            assert!(
                g.rb <= g.ra,
                "z={teeth}: the tip must not be inside the base circle"
            );
            assert!(g.u_tip >= 0.0 && g.u_j > g.u_tip);
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
        let external = Tooth::new(GearParams {
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
            let u = crate::involute::roll_at_radius(radius, external.rb);
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
            let external = Tooth::new(GearParams {
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
            Ring::cut_by(
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
            Ring::cut_by(
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

    /// **A large ring's tooth runs out of thickness before it runs out of
    /// involute**, and that has to be caught — unclamped it is not a thin tooth
    /// but a *crossed* one, whose outline is a self-intersecting polygon.
    ///
    /// A ring's tooth narrows inward, so its tip is its thinnest section. The
    /// limit exists only when `ψ_b < 0`, which needs `π/2z < inv α_t` — about 105
    /// teeth at 20°, so it is a large-ring phenomenon and invisible on the sizes
    /// one would test first.
    #[test]
    fn a_large_rings_tooth_is_never_allowed_to_cross_itself() {
        // Below the threshold `ψ_b` is positive and the base circle binds first.
        let small = ring(43);
        assert!(small.psi_b > 0.0);
        assert!(small.tooth_thickness_at(small.ra) > 0.0);

        // Above it, pushing the addendum used to give negative thickness.
        for addendum in [1.0, 2.0, 2.5, 3.0, 4.0, 8.0] {
            let g = Ring::cut_by(
                &GearParams {
                    teeth: 150,
                    addendum,
                    ..Default::default()
                },
                &Cutter::default(),
            );
            assert!(g.psi_b < 0.0, "150 teeth should be past the threshold");
            let thickness = g.tooth_thickness_at(g.ra);
            assert!(
                thickness >= -1e-9,
                "addendum={addendum}: tooth thickness {thickness} at the tip"
            );
            // The clamp is reported, not silent, once it bites.
            if addendum > 2.5 {
                assert!(
                    g.clamps.iter().any(|c| c.is(key::CLAMP_RING_TIP_RAISED)),
                    "addendum={addendum}: the clamp must say so"
                );
            }
        }
    }

    /// **A shaper at least as large as the ring is not a shaper.** It arrives as a
    /// negative centre distance rather than an obvious error, and used to produce
    /// a root radius half again the ring's own pitch radius while complaining only
    /// about the tip corner.
    #[test]
    fn a_cutter_no_smaller_than_the_ring_is_clamped_and_reported() {
        for cutter_teeth in [43u32, 50, 100] {
            let g = Ring::cut_by(
                &GearParams {
                    teeth: 43,
                    ..Default::default()
                },
                &Cutter {
                    teeth: cutter_teeth,
                    ..Cutter::default()
                },
            );
            assert!(
                g.cut.centre_distance > 0.0,
                "z_c={cutter_teeth}: centre distance {} is not a distance",
                g.cut.centre_distance
            );
            // The root circle stays where a root circle could be: outside the
            // pitch circle, and not by more than the tool could reach.
            assert!(
                g.rf > g.r && g.rf < g.r + 3.0,
                "z_c={cutter_teeth}: root radius {} against pitch {}",
                g.rf,
                g.r
            );
            assert!(
                g.clamps
                    .iter()
                    .any(|c| c.is(key::CLAMP_CUTTER_TEETH_REDUCED)),
                "z_c={cutter_teeth}: the clamp must name the real problem"
            );
        }
        // One fewer tooth than the ring is extreme but legal, and not clamped.
        let ok = Ring::cut_by(
            &GearParams {
                teeth: 43,
                ..Default::default()
            },
            &Cutter {
                teeth: 42,
                ..Cutter::default()
            },
        );
        assert!(!ok
            .clamps
            .iter()
            .any(|c| c.is(key::CLAMP_CUTTER_TEETH_REDUCED)));
    }

    /// A ring whose addendum would reach inside its own base circle is clamped
    /// and says so, rather than producing an involute that does not exist.
    #[test]
    fn an_addendum_reaching_past_the_base_circle_is_clamped_and_reported() {
        let g = Ring::cut_by(
            &GearParams {
                teeth: 20,
                addendum: 3.0,
                ..Default::default()
            },
            &Cutter::default(),
        );
        assert!(g.ra >= g.rb);
        assert!(
            g.clamps.iter().any(|c| c.is(key::CLAMP_RING_TIP_AT_BASE)),
            "clamps: {:?}",
            g.clamps
        );
    }

    /// **A ring's fillet does not vanish as the tool's round grows past its
    /// tip — it stops growing.**
    ///
    /// The same guard on an external gear caps the round and says so
    /// (`clamp.fillet_capped`). On a ring it used to *refuse* the tool, so
    /// `Ring` fell back to `fillet: None` and the part had no fillet at all.
    /// One input, two answers, and the jump was in kind rather than in degree —
    /// at a point where nothing physical happens.
    ///
    /// Gated as a **law rather than a threshold**: the realised round is
    /// non-decreasing in the round asked for, and every ring in the sweep has a
    /// fillet. A refusal breaks both at once, and neither needs a tolerance.
    #[test]
    fn a_tool_round_too_large_is_capped_rather_than_refused() {
        for (teeth, cutter_teeth, addendum) in
            [(43_u32, 20_u32, 1.25_f64), (60, 24, 1.0), (120, 30, 1.25)]
        {
            let mut realised = f64::MIN;
            let mut capped_somewhere = false;
            for step in 0..=40 {
                let asked = f64::from(step) * 0.02; // 0 … 0.80 modules
                let p = GearParams {
                    teeth,
                    ..Default::default()
                };
                let ring = Ring::cut_by(
                    &p,
                    &Cutter {
                        teeth: cutter_teeth,
                        addendum,
                        tip_round: asked,
                    },
                );
                let f = ring.fillet.as_ref().unwrap_or_else(|| {
                    panic!("z={teeth} cutter={cutter_teeth}: no fillet at all at ρ={asked}")
                });
                let _ = f;
                let got = ring.cut.tip_round;
                assert!(
                    got >= realised - 1e-12,
                    "z={teeth}: the realised round fell from {realised} to {got} as the \
                     round asked for rose to {asked}"
                );
                realised = got;
                if got < asked * p.module - 1e-12 {
                    capped_somewhere = true;
                    assert!(
                        ring.clamps.iter().any(|n| n.is(key::CLAMP_FILLET_CAPPED)),
                        "z={teeth}: the tool was capped to {got} from {asked} and said nothing"
                    );
                }
            }
            assert!(
                capped_somewhere,
                "z={teeth} cutter={cutter_teeth}: the sweep never reached the cap, so it \
                 checks nothing"
            );
        }
    }

    /// **A ring's pressure angle is guarded as a tooth's is**: the same floor,
    /// the same note, and the same angle carried, so a ring and the pinion
    /// asked with it cannot disagree about the rack they share.
    #[test]
    fn a_rings_pressure_angle_is_guarded_as_a_tooths_is() {
        for degrees in [0.0_f64, -5.0, 0.25, guard::MIN_PRESSURE_ANGLE_DEG] {
            let p = GearParams {
                teeth: 43,
                pressure_angle: degrees,
                ..Default::default()
            };
            let g = Ring::cut_by(&p, &Cutter::default());
            let t = Tooth::new(p);
            assert_eq!(g.alpha_n.to_bits(), t.alpha_n.to_bits(), "α={degrees}");
            assert_eq!(g.alpha_t.to_bits(), t.alpha_t.to_bits(), "α={degrees}");
            let noted = |c: &[Note]| c.iter().any(|n| n.is(key::CLAMP_PRESSURE_ANGLE_RAISED));
            assert_eq!(
                noted(&g.clamps),
                t.clamps.fired(key::CLAMP_PRESSURE_ANGLE_RAISED),
                "α={degrees}: the ring and the tooth disagree on the note"
            );
            for v in [g.ra, g.rf, g.rb, g.psi_b, g.u_tip, g.u_j, g.x_thick] {
                assert!(v.is_finite(), "α={degrees}: {g:?}");
            }
        }
    }

    /// **A tip asked inside the base circle is set on it**, exactly, and says
    /// why: the base circle, not a hair outside it, is where the involute
    /// begins. The radius is continuous in the addendum through the clamp.
    #[test]
    fn a_tip_inside_the_base_circle_is_set_on_it_and_says_so() {
        let at = |addendum: f64| {
            Ring::cut_by(
                &GearParams {
                    teeth: 20,
                    addendum,
                    ..Default::default()
                },
                &Cutter::default(),
            )
        };
        let g = at(3.0);
        assert_eq!(
            g.ra.to_bits(),
            g.rb.to_bits(),
            "tip {} against base {}",
            g.ra,
            g.rb
        );
        assert!(
            g.clamps.iter().any(|c| c.is(key::CLAMP_RING_TIP_AT_BASE)),
            "clamps: {:?}",
            g.clamps
        );
        assert!(!g.clamps.iter().any(|c| c.is(key::CLAMP_RING_TIP_RAISED)));
        // The addendum at which the tip reaches the base circle, and either
        // side of it.
        let edge = (g.r - g.rb) / g.params.module;
        let (below, above) = (at(edge * (1.0 - 1e-9)), at(edge * (1.0 + 1e-9)));
        assert!(
            (below.ra - above.ra).abs() < 1e-8,
            "{} against {}",
            below.ra,
            above.ra
        );
        assert!(!below
            .clamps
            .iter()
            .any(|c| c.is(key::CLAMP_RING_TIP_AT_BASE)));
        assert!(above
            .clamps
            .iter()
            .any(|c| c.is(key::CLAMP_RING_TIP_AT_BASE)));
    }

    /// **A flank that starts on the base circle has a direction there.** The
    /// involute's speed vanishes at the base circle and its direction does
    /// not: radial, and the load along the base circle's tangent. Read as a
    /// zero vector, a tip set on the base circle was a tangency for any
    /// parabola, and the rating was 0/0.
    #[test]
    fn a_flank_on_the_base_circle_has_a_direction_there() {
        let g = Ring::cut_by(
            &GearParams {
                teeth: 40,
                profile_shift: -0.4,
                ..Default::default()
            },
            &Cutter::default(),
        );
        assert_eq!(g.u_tip, 0.0, "the tip is set on the base circle");
        let (p, t) = g.flank_point_and_tangent(0.0);
        let (_, load) = g.flank_point_and_load_direction(0.0);
        let radial = [p[0] / g.rb, p[1] / g.rb];
        let n = f64::hypot(t[0], t[1]);
        assert!(
            (t[0] * radial[1] - t[1] * radial[0]).abs() / n < 1e-15,
            "{t:?}"
        );
        assert!(
            (load[0] * radial[0] + load[1] * radial[1]).abs() < 1e-15,
            "{load:?}"
        );
        // ...and away from it, the direction is the one a difference gives.
        for u in [0.05, 0.3, 0.6] {
            let h = 1e-6;
            let (a, _) = g.flank_point_and_tangent(u - h);
            let (b, _) = g.flank_point_and_tangent(u + h);
            let (_, t) = g.flank_point_and_tangent(u);
            let d = [b[0] - a[0], b[1] - a[1]];
            let cross =
                (d[0] * t[1] - d[1] * t[0]) / (f64::hypot(d[0], d[1]) * f64::hypot(t[0], t[1]));
            assert!(cross.abs() < 1e-8, "u={u}: {cross}");
        }
        let rated = crate::strength::root_section_with(
            &g,
            g.u_tip,
            crate::strength::CriticalSection::LewisParabola,
        )
        .expect("a section");
        assert!(
            rated.form_factor > 0.0 && rated.form_factor.is_finite(),
            "{rated:?}"
        );
    }

    /// Rings cut with the corner's path on both sides of the cutter's
    /// operating pitch circle: prolate (the corner centre outside it, a
    /// looped path) and curtate (inside it), reached by large shifts on small
    /// tooth differences, blunt or short cutters and thin cutter teeth.
    fn prolate_and_curtate() -> Vec<Ring> {
        let mut out = Vec::new();
        for teeth in [24_u32, 25, 30, 43, 60, 90] {
            for cutter_teeth in [12_u32, 14, 20, 80] {
                if cutter_teeth >= teeth {
                    continue;
                }
                for x in [-0.3, 0.0, 0.5, 0.8, 1.0] {
                    for addendum in [0.8, 1.0, 1.25] {
                        for k in [0.8, 1.0] {
                            for tip_round in [0.0, 0.2] {
                                out.push(Ring::cut_by(
                                    &GearParams {
                                        teeth,
                                        profile_shift: x,
                                        thickness_mod: k,
                                        ..Default::default()
                                    },
                                    &Cutter {
                                        teeth: cutter_teeth,
                                        addendum,
                                        tip_round,
                                    },
                                ));
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// Whether the corner centre runs inside the cutter's operating pitch
    /// circle.
    fn curtate(g: &Ring) -> bool {
        g.cut.corner_radius < g.cut.cutter_operating_radius
    }

    /// Whether the cut sits where the ring's shift puts it: inside the
    /// involute domain of the cutter–ring pair. Outside it the cut falls back
    /// to reference centres, a fault of its own (T05.4), and profile and cut
    /// describe two different rings.
    fn cut_where_asked(g: &Ring) -> bool {
        let z_c = 2.0 * g.cut.cutter_radius / g.mt;
        crate::mesh::operating_geometry(
            g.mt,
            g.alpha_t,
            g.alpha_n,
            z_c - f64::from(g.teeth),
            -g.params.profile_shift,
        )
        .is_some()
    }

    /// **The fillet meets the flank on every path, prolate or curtate**: the
    /// trochoid at the junction is the involute at the junction, to 1e-9 mm.
    ///
    /// Before, the fillet point was taken on the far side of the corner
    /// whenever the path was curtate, and 841 fillets broke this (T05.2).
    #[test]
    fn the_fillet_meets_the_flank_on_a_curtate_path_too() {
        use crate::strength::ToothOutline;
        let rings = prolate_and_curtate();
        let (mut checked, mut curtate_checked) = (0, 0);
        for g in &rings {
            if g.fillet.is_none() || !cut_where_asked(g) {
                continue;
            }
            let (r_flank, a_flank) = g.involute_at(g.u_j);
            let (r_fillet, a_fillet) = g.trochoid_at(ToothOutline::fillet_junction(g));
            let off = f64::hypot(r_flank - r_fillet, r_flank * (a_flank - a_fillet));
            assert!(
                off < 1e-9,
                "z={}/{} x={} k={} h_a0={} ρ={} ({}): the fillet misses the flank by {off} mm",
                g.teeth,
                g.cutter.teeth,
                g.params.profile_shift,
                g.params.thickness_mod,
                g.cutter.addendum,
                g.cutter.tip_round,
                if curtate(g) { "curtate" } else { "prolate" },
            );
            checked += 1;
            curtate_checked += usize::from(curtate(g));
        }
        assert!(
            curtate_checked > 100,
            "{curtate_checked} curtate of {checked}"
        );
        assert!(checked > curtate_checked + 100, "{checked} checked");
    }

    /// **The corner cuts to the root it was set to**: the deepest point of
    /// the fillet is `a_cut + r_tip`, the root radius, whichever side of the
    /// operating pitch circle the corner runs. A curtate path read the near
    /// side of the round and stopped `2ρ` short.
    #[test]
    fn the_deepest_cut_is_the_root_on_every_path() {
        let mut curtate_checked = 0;
        for g in prolate_and_curtate() {
            if g.fillet.is_none() || g.clamps.iter().any(|c| c.is(key::CLAMP_SPACE_CLOSED)) {
                continue;
            }
            let deepest = g.trochoid_at(0.0).0;
            assert!(
                (deepest - g.rf).abs() < 1e-9,
                "z={}/{} x={} ρ={}: the deepest cut is {deepest}, the root {}",
                g.teeth,
                g.cutter.teeth,
                g.params.profile_shift,
                g.cut.tip_round,
                g.rf
            );
            curtate_checked += usize::from(curtate(&g));
        }
        assert!(curtate_checked > 100, "{curtate_checked}");
    }

    /// **Nothing jumps where the path turns from prolate to curtate.** On
    /// 30/20 that is at x ≈ 0.683; before, the junction jumped 0.094 mm there
    /// and the profile was not a number at the crossing itself.
    #[test]
    fn a_ring_is_continuous_in_its_shift_through_the_curtate_turn() {
        let at = |x: f64| {
            Ring::cut_by(
                &GearParams {
                    teeth: 30,
                    profile_shift: x,
                    ..Default::default()
                },
                &Cutter::default(),
            )
        };
        let step = 5e-4;
        let mut previous: Option<(f64, f64)> = None;
        let (mut seen_prolate, mut seen_curtate) = (false, false);
        for i in 0..=100 {
            let x = 0.66 + step * f64::from(i);
            let g = at(x);
            assert!(g.fillet.is_some(), "x={x}: {:?}", g.clamps);
            seen_prolate |= !curtate(&g);
            seen_curtate |= curtate(&g);
            let now = (g.rf, g.involute_at(g.u_j).0);
            assert!(now.0.is_finite() && now.1.is_finite(), "x={x}: {now:?}");
            if let Some(before) = previous {
                // A shift moves the form by about a module per unit: twenty
                // times that over one step is a jump, not a slope.
                let limit = 20.0 * step * g.params.module;
                assert!(
                    (now.0 - before.0).abs() < limit && (now.1 - before.1).abs() < limit,
                    "x={x}: root {} → {}, junction {} → {}",
                    before.0,
                    now.0,
                    before.1,
                    now.1
                );
            }
            previous = Some(now);
        }
        assert!(
            seen_prolate && seen_curtate,
            "the sweep does not cross the turn"
        );
        // At the turn itself the corner stands on the pitch point and cuts
        // its own arc: still a fillet, still a number.
        let turn = at(0.683_258_95);
        let f = turn.fillet.expect("a fillet at the turn");
        let _ = f;
        for v in [turn.rf, turn.u_j, turn.involute_at(turn.u_j).0] {
            assert!(v.is_finite(), "{turn:?}");
        }
    }

    /// **A curtate cut is the shape its cutter leaves**, by the simulation
    /// that shares no code with the profile: large shifts on small tooth
    /// differences, a thinner cutter tooth, the hula's proportions and the
    /// cutter's addendum turned. Before, 30/20 at x 0.8 read 0.065 mm.
    ///
    /// Not here: 24/20 at x 0.5, which misses by 0.335 mm at the ring's tip,
    /// where the cutter's tip trims it — T05.10's case, not the fillet's.
    #[test]
    fn a_curtate_ring_is_the_shape_its_cutter_leaves() {
        let cases: &[(u32, u32, f64, f64, f64, f64)] = &[
            // (z, z_c, x, k, h_a, h_a0)
            (30, 20, 0.8, 1.0, 1.0, 1.25),
            (30, 20, 0.9, 1.0, 1.0, 1.25),
            (30, 20, 1.0, 1.0, 1.0, 1.25),
            (30, 20, 0.8, 1.0, 1.0, 0.8),
            (30, 20, 0.8, 1.0, 1.0, 1.0),
            // At k 0.6 this cutter has no tip corner at all (T05.9's case),
            // and at h_a 1 its tip trims the ring's by 5 µm (T05.10's).
            (25, 20, 1.0, 0.8, 0.8, 1.25),
            (19, 14, 0.607, 1.0, 0.7, 1.0),
        ];
        let mut curtate_seen = 0;
        for &(teeth, cutter_teeth, x, k, addendum, cutter_addendum) in cases {
            let g = Ring::cut_by(
                &GearParams {
                    teeth,
                    profile_shift: x,
                    thickness_mod: k,
                    addendum,
                    ..Default::default()
                },
                &Cutter {
                    teeth: cutter_teeth,
                    addendum: cutter_addendum,
                    ..Cutter::default()
                },
            );
            curtate_seen += usize::from(curtate(&g));
            let report = crate::verify::check_ring_cut(&g, 400, 4_000);
            assert!(
                report.worst_distance < 5e-3,
                "z={teeth}/{cutter_teeth} x={x} k={k} h_a={addendum} h_a0={cutter_addendum}: \
                 cut and profile differ by {} mm",
                report.worst_distance
            );
        }
        assert!(curtate_seen >= 4, "{curtate_seen} of the cases are curtate");
    }

    /// A ring and its pinion, as the tip-room cases state them.
    fn internal_pair(
        teeth: [u32; 2],
        shift: [f64; 2],
        addendum: [f64; 2],
        cutter_teeth: u32,
    ) -> (Ring, Tooth) {
        (
            Ring::cut_by(
                &GearParams {
                    teeth: teeth[0],
                    profile_shift: shift[0],
                    addendum: addendum[0],
                    ..Default::default()
                },
                &Cutter {
                    teeth: cutter_teeth,
                    ..Cutter::default()
                },
            ),
            Tooth::new(GearParams {
                teeth: teeth[1],
                profile_shift: shift[1],
                addendum: addendum[1],
                ..Default::default()
            }),
        )
    }

    /// **The roll is the harness's**: the independent roll reads the three
    /// fouls the audit measured with `gear-cli`'s outline roll to 0.01 mm.
    #[test]
    fn the_independent_roll_reads_what_the_harness_rolled() {
        for (teeth, shift, addendum, cutter, a, harness) in [
            ([48, 46], [0.9, 0.9], [1.0, 1.0], 20, Some(1.0), -0.512),
            ([24, 23], [0.8, 0.2], [1.0, 1.0], 12, None, -0.187),
            ([43, 42], [0.0, 0.0], [0.7, 0.8], 20, Some(0.5), -0.587),
        ] {
            let (ring, pinion) = internal_pair(teeth, shift, addendum, cutter);
            let a = a.unwrap_or_else(|| mesh_with(&ring, &pinion).unwrap().centre_distance);
            let rolled = roll::rolled(&ring, &pinion, a);
            assert!(
                (rolled - harness).abs() < 0.01,
                "{teeth:?}: rolled {rolled}, the harness {harness}"
            );
        }
    }

    /// **A pair the roll finds fouled is called fouled** — each a case the
    /// verdict passed: a tooth that overtook a ring tooth inside the lens
    /// (48/46), a pinion tip circle enclosing the ring's (24/23), the ring's
    /// enclosing nothing at a stated distance (43/42, "+∞"), and a far side
    /// that overlaps (32/30 at 0.96).
    #[test]
    fn a_pair_the_roll_finds_fouled_is_called_fouled() {
        for (teeth, shift, addendum, cutter, a) in [
            ([48, 46], [0.9, 0.9], [1.0, 1.0], 20, Some(1.0)),
            ([24, 23], [0.8, 0.2], [1.0, 1.0], 12, None),
            ([43, 42], [0.0, 0.0], [0.7, 0.8], 20, Some(0.5)),
            ([32, 30], [0.0, 0.0], [1.0, 1.0], 20, Some(0.96)),
        ] {
            let (ring, pinion) = internal_pair(teeth, shift, addendum, cutter);
            let a = a.unwrap_or_else(|| mesh_with(&ring, &pinion).unwrap().centre_distance);
            let rolled = roll::rolled(&ring, &pinion, a);
            assert!(rolled < -0.1, "{teeth:?} at {a}: the roll says {rolled}");
            let m = mesh_at(&ring, &pinion, a).unwrap();
            assert!(
                m.tip_interference,
                "{teeth:?} at {a}: rolled {rolled}, called clear"
            );
        }
    }

    /// **The tip window reads each tooth's own tip land** (T03.5): a pointed
    /// tip is a point, and a tip on its fillet is as wide as the fillet is
    /// there. A 5-tooth pinion at x −0.4 has its tip below the form circle,
    /// on the fillet: at a 0.2-module addendum its land is 0.189 rad where
    /// the involute continued to that radius would be 0.271. Read at the
    /// involute's width it was called tip-fouled against rings of 6 to 14
    /// teeth that its own outline rolls clear of; at 0.4 modules (0.238
    /// against 0.256) the two readings agree that it is clear, the control.
    #[test]
    fn a_tip_on_its_fillet_is_read_at_its_own_land() {
        let mut checked = 0;
        for addendum in [0.2, 0.4] {
            for ring_teeth in 6..=14_u32 {
                for ring_shift in [0.0, 0.5, 1.0] {
                    let (ring, pinion) = internal_pair(
                        [ring_teeth, 5],
                        [ring_shift, -0.4],
                        [1.0, addendum],
                        (ring_teeth / 2).max(3),
                    );
                    assert!(pinion
                        .clamps
                        .notes
                        .iter()
                        .any(|n| n.is(key::CLAMP_TIP_BELOW_FORM)));
                    let m = mesh_with(&ring, &pinion).unwrap();
                    let Some(margin) = m.tip_margin else {
                        continue;
                    };
                    checked += 1;
                    let rolled = roll::rolled(&ring, &pinion, m.centre_distance);
                    assert!(
                        rolled > -1e-9 && margin >= 0.0,
                        "{ring_teeth} x {ring_shift} / 5 h_a {addendum}: rolled {rolled}, \
                         margin {margin}"
                    );
                }
            }
        }
        assert_eq!(checked, 54, "every pair's tip circles cross");
    }

    /// **A pointed tip is a point, and the margin passes through it
    /// continuously** (T03.5). The audit's pair — a 24-tooth ring at x 1.1
    /// cut by a 12-tooth shaper, a 19-tooth pinion at x 1.2 at 2.3613 mm —
    /// has a pointed pinion whose tip half-width came out −1.1e-16, which an
    /// early return once read as +∞ room; it fouls by 0.0251 rad. Swept
    /// through the shift at which the pinion's tip comes to a point, no step
    /// of the margin is more than twice the larger of its neighbours.
    #[test]
    fn a_pointed_tip_is_a_point_in_the_tip_window() {
        let at = |x: f64| {
            let (ring, pinion) = internal_pair([24, 19], [1.1, x], [1.0, 1.0], 12);
            let pointed = pinion
                .clamps
                .notes
                .iter()
                .any(|n| n.is(key::CLAMP_TIP_CAPPED_POINTED));
            let m = mesh_at(&ring, &pinion, 2.3613).unwrap();
            (m.tip_margin.unwrap(), m.tip_interference, pointed)
        };
        let (margin, fouled, pointed) = at(1.2);
        assert!(pointed && fouled, "{margin}");
        assert!((margin + 0.0251).abs() < 5e-5, "{margin}");

        let shifts: Vec<f64> = (0..=60).map(|i| 0.9 + 0.005 * f64::from(i)).collect();
        let read: Vec<_> = shifts.iter().map(|&x| at(x)).collect();
        let first = read
            .iter()
            .position(|r| r.2)
            .expect("the sweep reaches a point");
        assert!(
            first > 1 && first + 1 < read.len(),
            "the point is inside the sweep"
        );
        let step = |i: usize| (read[i].0 - read[i - 1].0).abs();
        let across = step(first);
        let beside = step(first - 1).max(step(first + 1));
        assert!(
            across <= 2.0 * beside,
            "{across} across the point, {beside} beside it"
        );
    }

    /// **Clear means clear**: over tooth differences 1–10, shifts and addenda,
    /// a pair every flag calls clear rolls clear to 0.02 modules.
    #[test]
    fn a_pair_called_clear_rolls_clear() {
        let (mut clear, mut fouled) = (0, 0);
        for ring_teeth in [30_u32, 48] {
            for difference in 1..=10 {
                for shift in [[0.0, 0.0], [0.5, 0.2], [0.9, 0.9]] {
                    for addendum in [0.7, 1.0] {
                        let (ring, pinion) = internal_pair(
                            [ring_teeth, ring_teeth - difference],
                            shift,
                            [addendum; 2],
                            20,
                        );
                        let Some(m) = mesh_with(&ring, &pinion) else {
                            continue;
                        };
                        let rolled = roll::rolled(&ring, &pinion, m.centre_distance);
                        let called_clear = !(m.tip_interference
                            || m.trochoid_interference
                            || m.involute_interference);
                        if called_clear {
                            clear += 1;
                            assert!(
                                rolled >= -0.02 * ring.params.module,
                                "{ring_teeth}/{} x {shift:?} h_a {addendum}: called clear, \
                                 rolled {rolled}",
                                ring_teeth - difference
                            );
                        } else {
                            fouled += 1;
                        }
                    }
                }
            }
        }
        assert!(clear > 20 && fouled > 20, "{clear} clear, {fouled} fouled");
    }
}
