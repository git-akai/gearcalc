//! **The edits a train's graph takes**, each a rule about what else has to
//! change, made whole or refused whole ([`Edit`], made by
//! [`super::Train::edit`]).
//!
//! A designer permutes an arrangement by adding and removing: a gear at any
//! gear — a sun or a ring on a planet gear, a gear on a new axis at the end
//! of a chain — a step on a planet body, another ratio across a distance;
//! and by moving a gear to another body on its axis, joining two bodies,
//! holding one. Nothing is *flipped*: a member's kind is decided by the
//! entry that adds it and a ring stays a ring, since a sun and a ring
//! differ in more than a flag (a cutter, a shift rule) and a swap is a
//! remove and an add, the new member sized by the core to what it meets.
//!
//! Every add appends — a new body is numbered after every body the train
//! has, a new member is the last — so nothing a case or a hold names
//! moves. A remove takes a piece out with what goes with it; the train then
//! drops any body nothing names and renumbers the rest, repointing its
//! cases and holds ([`super::Train::edit`]). The invariants the edits keep:
//! every member is in a mesh, every planet gear meets a central member,
//! every distance carries a mesh, and a carrier's body is never removed.

use super::shape::{Member, Shape};
use super::structure::{CarrierTree, Hang};
use crate::kinematics::GROUND;
use crate::params::Auto;

/// **What a designer does to a train's graph** — the one set of edits,
/// every index the graph's own: a member, a mesh, a distance, an axis or a
/// coupling by its place in the [`Shape`]'s lists, a body by the train's
/// number for it. Each is a rule about what else changes, made whole or
/// refused whole ([`EditRefused`]) by [`super::Train::edit`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum Edit {
    /// **A gear meshing `mate`**, on `on` — a ring where `ring` — sized to
    /// what it meets: to the distance between the two axes where they have
    /// one (a sun or a ring on a planet gear to the radius its axis runs
    /// at), and on a new axis the mate's count, a ring twice it. It is the
    /// crate's default gear at that count — a ring cut by the default
    /// cutter — its module, pressure angle and helix automatic, so its mesh
    /// gives it the group's and the hand it needs, its shift and thickness
    /// automatic.
    AddGear { mate: usize, on: Place, ring: bool },
    /// **Another ratio across a distance**: a gear on `shared` — a body on
    /// one of the distance's two axes — meshing a gear on a new body of the
    /// other, the two copying the distance's first mesh. Which body the
    /// ratios share is asked: it is what makes the pairs a layshaft.
    AddRatio { distance: usize, shared: usize },
    /// **A step**: one more gear on the planet body of carried `axis`, with
    /// a ring meshing it at the radius the axis runs at.
    AddStep { axis: usize },
    /// **An offset coupling** from `body`, on a carried axis, to a new body
    /// on its carrier's axis: the pins that take a cycloidal disc's turn
    /// off to the centre line.
    Couple { body: usize },
    /// **A piece taken out, with what goes with it** — a gear left meshing
    /// nothing, a body left with nothing on it, a distance left with no
    /// mesh, an axis left with nothing on it — and refused where a planet
    /// gear would be left meeting nothing on its carrier's axis, whose
    /// radius it runs at.
    Remove(Piece),
    /// **A gear moved** to another body on its axis — `None` a new one —
    /// the body it leaves staying while anything names it.
    Move { member: usize, to: Option<usize> },
    /// **Two bodies made one** ([`super::Train::join`]).
    Join { a: usize, b: usize },
    /// A body held to ground.
    Hold(usize),
    /// A body's hold taken out.
    Release(usize),
    /// **A shape laid into the train** — a preset's, from the add menu —
    /// its bodies numbered after the train's: its conventional input made
    /// one with `at` where given, and otherwise with the last part's
    /// remaining open output, the case entries there carried to its own
    /// ([`super::Train::chain_on`]).
    Insert { shape: Shape, at: Option<usize> },
}

/// **Where a gear an edit adds goes.**
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum Place {
    /// On a body the train has.
    Body(usize),
    /// On a new body of an axis the graph has.
    NewBody(usize),
    /// On a new axis fixed in ground, at an automatic distance from the
    /// mate's.
    NewAxis,
}

/// **A piece of the graph**, by the graph's index — a body by its number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum Piece {
    Member(usize),
    Mesh(usize),
    Axis(usize),
    Body(usize),
    Coupling(usize),
}

/// Why an edit is refused: the invariant it would break.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditRefused {
    /// No such member, mesh, axis, distance or body.
    NoSuchIndex,
    /// A planet gear would be left meeting nothing on its carrier's axis,
    /// whose radius it runs at: the last member meeting it, taken.
    LastOnItsStep,
    /// **An edit the shape does not make**: a join, a hold, a release or
    /// an insert asked of the shape, which are the train's
    /// ([`super::Train::edit`]).
    WrongFamily,
    /// **A step or a coupling off a carried axis**: a step goes on the
    /// planets of an axis a carrier turns, and a coupling takes an
    /// orbiting body's turn to its carrier's axis.
    NotCarried,
    /// **The axis a carrier turns about**, taken away: the planets it
    /// carries would hang from nothing.
    CentralAxis,
    /// **Two rings in mesh**: an internal gear meshes an external one.
    RingToRing,
    /// **A new fixed axis at a gear that meshes riding a carrier** — a
    /// planet, or a sun or a ring meshing planets: its meshes stand in the
    /// carrier's frame, which no axis fixed in ground shares.
    OrbitingMate,
    /// **A ring across crossed shafts**: the screw model has no internal
    /// kind.
    RingCrossed,
    /// **An edit that locks the train**: after it, a body stands still
    /// under the train's meshes, couplings and holds that turned before it
    /// or that it added, and that the train does not hold — read from the
    /// motion before and after, each body followed through the edit's
    /// renumbering. A ring on a pair's second gear's body, a gear on a
    /// planet's body meshing a central member the planet already turns
    /// against, a join of two shafts one chain turns at different speeds,
    /// a move that leaves a gear meshing two gears on one body, a preset
    /// laid in on a held shaft. Decided without a solve; a twin at the same
    /// ratio forces nothing still, and is made.
    Locks,
    /// A body not on the member's axis.
    NotOnTheAxis,
    /// No member of that kind fits at the radius the axis runs at: a sun
    /// inside a planocentric, whose planet all but fills its ring.
    NoRoom,
    /// The body carries an axis: a gear on the carrier of the planets it
    /// meshes locks the set.
    CarriesAnAxis,
    /// The body is coupled already.
    Coupled,
    /// No axis distance joins the two axes a mesh would cross.
    NoDistance,
    /// Two bodies geared to each other — of one part — made one: a mesh
    /// or a carrier would turn against itself.
    Geared,
    /// Two bodies an axis distance apart made one: a shaft is straight.
    Apart,
    /// A body a case loads, reacts or measures its sweep at held, left in
    /// no part, or joined to a held body or to another the same case
    /// names: the figure would be grounded, cut off or dropped, and a
    /// stated figure is never moved to a guessed body or dropped.
    Loaded,
    /// **A gear meshing in two frames** — a gear on a fixed axis meshing a
    /// sun or a ring whose other mates ride a carrier — which the wiring
    /// cannot hold: it gives each member one frame. A frame per mesh would
    /// let it stand.
    TwoFrames,
}

impl EditRefused {
    /// The catalogue key of the sentence the panel shows — a `ui.` key,
    /// since a refusal is the interface's word and not a note the solve
    /// emits; the [`std::fmt::Display`] below is the harness's English.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::NoSuchIndex => "ui.train_edit_refused_no_such",
            Self::LastOnItsStep => "ui.train_edit_refused_last_on_step",
            Self::WrongFamily => "ui.train_edit_refused_family",
            Self::NotCarried => "ui.train_edit_refused_not_carried",
            Self::CentralAxis => "ui.train_edit_refused_central_axis",
            Self::RingToRing => "ui.train_edit_refused_ring_to_ring",
            Self::OrbitingMate => "ui.train_edit_refused_orbiting_mate",
            Self::RingCrossed => "ui.train_edit_refused_ring_crossed",
            Self::Locks => "ui.train_edit_refused_locks",
            Self::NotOnTheAxis => "ui.train_edit_refused_axis",
            Self::NoRoom => "ui.train_edit_refused_no_room",
            Self::CarriesAnAxis => "ui.train_edit_refused_carrier",
            Self::Coupled => "ui.train_edit_refused_coupled",
            Self::NoDistance => "ui.train_edit_refused_no_distance",
            Self::Geared => "ui.train_edit_refused_geared",
            Self::Apart => "ui.train_edit_refused_apart",
            Self::Loaded => "ui.train_edit_refused_loaded",
            Self::TwoFrames => "ui.train_edit_refused_two_frames",
        }
    }
}

impl std::fmt::Display for EditRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NoSuchIndex => "no such member, mesh, axis or body",
            Self::LastOnItsStep => "the last on its step",
            Self::WrongFamily => "not an edit of this family",
            Self::NotCarried => "that axis is not carried",
            Self::CentralAxis => "a carrier turns about that axis",
            Self::RingToRing => "two rings do not mesh",
            Self::OrbitingMate => "that gear meshes riding a carrier",
            Self::RingCrossed => "a ring does not mesh across crossed shafts",
            Self::Locks => "that would force a body that turns to stand still",
            Self::NotOnTheAxis => "not a body on the member's axis",
            Self::NoRoom => "nothing of that kind fits at this radius",
            Self::CarriesAnAxis => "that body carries an axis",
            Self::Coupled => "that body is coupled already",
            Self::NoDistance => "no axis distance joins those axes",
            Self::Geared => "those bodies are geared to each other",
            Self::Apart => "those bodies are an axis distance apart",
            Self::Loaded => "a case loads, reacts or measures its sweep at a body this would hold, cut off or join to another it names",
            Self::TwoFrames => "that gear would mesh in two frames",
        })
    }
}

impl Shape {
    /// **One of the graph's edits on the shape** — every one but a join, a
    /// hold and an insert, which are the train's ([`super::Train::edit`]) —
    /// numbering any body it adds from `next`.
    ///
    /// # Errors
    ///
    /// [`EditRefused`], the shape untouched.
    pub(crate) fn apply(&mut self, edit: &Edit, next: usize) -> Result<(), EditRefused> {
        self.transact(|s| match *edit {
            Edit::AddGear { mate, on, ring } => s.add_gear(mate, on, ring, next),
            Edit::AddRatio { distance, shared } => s.add_ratio(distance, shared, next),
            Edit::AddStep { axis } => s.add_step(axis, next),
            Edit::Couple { body } => s.couple(body, next),
            Edit::Remove(piece) => s.remove(piece),
            Edit::Move { member, to } => s.move_body(member, to, next),
            Edit::Join { .. } | Edit::Hold(_) | Edit::Release(_) | Edit::Insert { .. } => {
                Err(EditRefused::WrongFamily)
            }
        })
    }

    /// **An edit made whole or not at all**: on a copy, kept where it
    /// refuses nothing and leaves every planet that ran at a radius still
    /// running at one — a carried axis a gear is on meeting a gear on its
    /// carrier's axis, the mesh its radius is read from — and every mesh
    /// that had a kind and a frame still having them ([`Self::meshes_whole`]).
    fn transact(
        &mut self,
        edit: impl FnOnce(&mut Self) -> Result<(), EditRefused>,
    ) -> Result<(), EditRefused> {
        let mut s = self.clone();
        edit(&mut s)?;
        if self.planets_at_a_radius() && !s.planets_at_a_radius() {
            return Err(EditRefused::LastOnItsStep);
        }
        if self.meshes_whole().is_ok() {
            s.meshes_whole()?;
        }
        *self = s;
        Ok(())
    }

    /// **Every mesh has a kind and a frame**: a ring on crossed shafts has
    /// no kind the screw model holds
    /// ([`kind_of`](super::incidence::Indexed::kind_of)), and a member
    /// meshing in two frames none the wiring holds ([`super::Wiring::frame`]).
    fn meshes_whole(&self) -> Result<(), EditRefused> {
        let s = self.indexed();
        let wiring = s.wiring();
        for k in 0..self.meshes.len() {
            if s.kind_of(k).is_none() {
                return Err(EditRefused::RingCrossed);
            }
            if wiring.frame(k).is_err() {
                return Err(EditRefused::TwoFrames);
            }
        }
        Ok(())
    }

    /// Whether every carried axis with a gear on it meets a gear on its
    /// carrier's axis.
    fn planets_at_a_radius(&self) -> bool {
        let axis_of = |i: usize| self.axis_of_slot(self.slot_of_member(i));
        (0..self.axes.len()).filter(|&a| self.carried(a)).all(|a| {
            let central = self.central_axis_of(a);
            let meets = |x: usize, y: usize| axis_of(x) == Some(a) && axis_of(y) == central;
            !(0..self.members.len()).any(|i| axis_of(i) == Some(a))
                || self
                    .meshes
                    .iter()
                    .any(|m| meets(m.a, m.b) || meets(m.b, m.a))
        })
    }

    // ----------------------------------------------------------- reading ---

    fn carried(&self, axis: usize) -> bool {
        self.axes[axis].carried_by != GROUND
    }

    /// The axis a carried axis goes round: its carrier's.
    fn central_axis_of(&self, axis: usize) -> Option<usize> {
        self.axis_of_slot(self.slot(self.axes[axis].carried_by))
    }

    pub(crate) fn members_on_body(&self, body: usize) -> Vec<usize> {
        self.indexed().members_on(body).to_vec()
    }

    /// The other member of a mesh.
    fn mate(&self, mesh: usize, member: usize) -> usize {
        let m = self.meshes[mesh];
        if m.a == member {
            m.b
        } else {
            m.a
        }
    }

    /// A member on a carried axis: a planet, whatever it meshes with.
    pub(crate) fn is_planet_gear(&self, member: usize) -> bool {
        self.axis_of_slot(self.slot_of_member(member))
            .is_some_and(|a| self.carried(a))
    }

    pub(crate) fn carries_an_axis(&self, body: usize) -> bool {
        self.axes.iter().any(|a| a.carried_by == body)
    }

    /// The axis a body of this shape sits on.
    fn axis_of_body(&self, body: usize) -> Option<usize> {
        self.axis_of_slot(self.slot(body))
    }

    /// **The carrier radius a carried axis runs at**, read off any mesh a
    /// central member has with a gear on it: `(z_c ± z_p) m / 2`, a ring
    /// minus. `None` where no central member meets the axis yet.
    fn carrier_radius(&self, axis: usize) -> Option<f64> {
        self.meshes.iter().find_map(|m| {
            let (p, c) = [(m.a, m.b), (m.b, m.a)].into_iter().find(|&(p, c)| {
                self.axis_of_slot(self.slot_of_member(p)) == Some(axis) && !self.is_planet_gear(c)
            })?;
            let (zp, zc) = (
                f64::from(self.members[p].gear.teeth),
                f64::from(self.members[c].gear.teeth),
            );
            let module = self.members[c].normal_module();
            Some(if self.members[c].ring.is_some() {
                (zc - zp) * module / 2.0
            } else {
                (zc + zp) * module / 2.0
            })
        })
    }

    // ------------------------------------------------------------ adding ---

    /// **A gear meshing `mate`, on `on`** ([`Edit::AddGear`]).
    fn add_gear(
        &mut self,
        mate: usize,
        on: Place,
        ring: bool,
        next: usize,
    ) -> Result<(), EditRefused> {
        if mate >= self.members.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        let from = self
            .axis_of_slot(self.slot_of_member(mate))
            .ok_or(EditRefused::NoSuchIndex)?;
        // Two rings in mesh are no mesh.
        if ring && self.members[mate].ring.is_some() {
            return Err(EditRefused::RingToRing);
        }
        let (axis, body) = match on {
            Place::NewAxis => return self.add_on_new_axis(mate, from, ring, next),
            Place::NewBody(axis) => (axis, None),
            Place::Body(b) => (
                self.axis_of_body(b).ok_or(EditRefused::NoSuchIndex)?,
                Some(b),
            ),
        };
        // A gear fixed to the carrier of the planet it meshes locks it.
        if body.is_some_and(|b| self.axes[from].carried_by == b) {
            return Err(EditRefused::CarriesAnAxis);
        }
        if axis >= self.axes.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        let across = |d: &super::shape::Distance| d.axes == [axis, from] || d.axes == [from, axis];
        let Some(distance) = self
            .distances
            .iter()
            .position(across)
            .filter(|_| axis != from)
        else {
            return Err(EditRefused::NoDistance);
        };
        // A sun or a ring on a planet gear, at the radius the planet runs
        // at; anything else, at the distance between the two axes.
        let teeth = if self.carried(from) && self.central_axis_of(from) == Some(axis) {
            self.central_teeth(mate, ring, body.is_none())?
        } else {
            self.fitted_teeth(mate, axis, distance, ring)?
        };
        let body = body.unwrap_or_else(|| self.push_body(axis, next));
        self.push_follower(mate, body, teeth, ring);
        Ok(())
    }

    /// **A gear on a new axis fixed in ground**, at an automatic distance
    /// from its mate's: the mate's count — which at a chain's end is an
    /// idler behind the last — or a ring twice its count round it. Refused
    /// for a mate that does not mesh in ground — a planet, or a sun or a
    /// ring meshing planets — whose frame the new axis cannot share.
    fn add_on_new_axis(
        &mut self,
        mate: usize,
        from: usize,
        ring: bool,
        next: usize,
    ) -> Result<(), EditRefused> {
        if self.indexed().frame_of_member(mate) != GROUND {
            return Err(EditRefused::OrbitingMate);
        }
        let axis = self.push_axis(GROUND, 1);
        let body = self.push_body(axis, next);
        let z = self.members[mate].gear.teeth;
        self.push_follower(mate, body, if ring { 2 * z } else { z }, ring);
        self.push_distance([from, axis], 0.0);
        Ok(())
    }

    /// **The count a sun or a ring on planet gear `mate` takes**: sized to
    /// the radius its axis runs at, a few teeth of difference where nothing
    /// sets it yet — and on a body of its own (`fresh`), moved off a count
    /// that would turn as one with another. Refused as [`EditRefused::NoRoom`]
    /// where the count it comes to cannot close at the carrier radius: its
    /// mesh's operating angle, `cos α_w = a₀ cos α / a` with `a₀` its
    /// reference span and `a` the radius, the solve's own test
    /// ([`crate::mesh::Mesh::pressure_angle_at`]), outside `(0, 1)`.
    fn central_teeth(&self, mate: usize, ring: bool, fresh: bool) -> Result<u32, EditRefused> {
        let planet_axis = self
            .axis_of_slot(self.slot_of_member(mate))
            .ok_or(EditRefused::NoSuchIndex)?;
        let (zp, module) = (
            self.members[mate].gear.teeth,
            self.members[mate].normal_module(),
        );
        let radius = self.carrier_radius(planet_axis);
        let fit = radius.map(|r| (2.0 * r / module).round());
        // A ring one tooth larger than its planet is the least internal
        // mesh there is — a planocentric's; a sun of fewer than four teeth
        // is refused as ever (the floors are T15.15's to derive).
        let floor = if ring { f64::from(zp) + 1.0 } else { 4.0 };
        let mut teeth = match (ring, fit) {
            (true, Some(f)) => (f + f64::from(zp)).max(floor),
            (true, None) => f64::from(zp) + 2.0,
            (false, Some(f)) if f - f64::from(zp) < 4.0 => return Err(EditRefused::NoRoom),
            (false, Some(f)) => f - f64::from(zp),
            (false, None) => f64::from(zp.max(6)),
        };
        // Two equal centrals of one kind on one planet gear lock it, and
        // on two equal gears of one planet body turn as one — a ring added
        // to a step at the first step's count carries nothing — so the
        // count moves by a tooth: **down**, where it can, since a shift can
        // open a mesh past its reference distance by `1/cos α` at most,
        // some six per cent, and a planocentric's carrier radius is a few
        // teeth, so a ring a tooth *larger* at that radius has nowhere to
        // close; a tooth smaller always has.
        let at = self.indexed();
        let taken = |z: f64| {
            (0..self.members.len())
                .filter(|&p| self.axis_of_slot(self.slot_of_member(p)) == Some(planet_axis))
                .filter(|&p| self.members[p].gear.teeth == zp)
                .flat_map(|p| at.meshes_of(p).iter().map(move |&k| (p, k)))
                .any(|(p, k)| {
                    let c = self.mate(k, p);
                    self.members[c].ring.is_some() == ring
                        && f64::from(self.members[c].gear.teeth) == z
                })
        };
        while fresh && taken(teeth) && teeth > floor {
            teeth -= 1.0;
        }
        while fresh && taken(teeth) {
            teeth += 1.0;
        }
        if let Some(a) = radius {
            let span = if ring {
                teeth - f64::from(zp)
            } else {
                teeth + f64::from(zp)
            };
            let alpha = self.members[mate].normal_pressure_angle().to_radians();
            let cos_w = span * module / 2.0 * alpha.cos() / a;
            if !(cos_w > 0.0 && cos_w < 1.0) {
                return Err(EditRefused::NoRoom);
            }
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(teeth as u32)
    }

    /// **The count a gear on `axis` meshing `mate` takes to close
    /// `distance`**, read off the distance's first mesh: on parallel axes
    /// the reference span it runs at, `|z_a ± z_b| m_t / 2` — a ring's
    /// count negative, at that mesh's transverse module — in the mate's
    /// transverse module, less the mate's count; at an angle the count
    /// across from the mate's side, since there a helix sets the size.
    fn fitted_teeth(
        &self,
        mate: usize,
        axis: usize,
        distance: usize,
        ring: bool,
    ) -> Result<u32, EditRefused> {
        let s = self.indexed();
        let first = *s
            .meshes_on(distance)
            .first()
            .ok_or(EditRefused::NoDistance)?;
        let m = self.meshes[first];
        let on_axis = |i: usize| self.axis_of_slot(self.slot_of_member(i)) == Some(axis);
        let across = if on_axis(m.a) { m.a } else { m.b };
        if s.is_crossed(first) {
            return Ok(self.members[across].gear.teeth);
        }
        let signed = |i: usize| {
            let z = f64::from(self.members[i].gear.teeth);
            if self.members[i].ring.is_some() {
                -z
            } else {
                z
            }
        };
        let (shared, helix) = (self.shared(), s.helix_angles());
        let transverse = |i: usize| shared.members[i].normal_module() / helix[i].to_radians().cos();
        let span = ((signed(m.a) + signed(m.b)).abs() * transverse(m.a) / transverse(mate)).round();
        let zm = f64::from(self.members[mate].gear.teeth);
        let z = match (ring, self.members[mate].ring.is_some()) {
            (true, _) => zm + span,
            (false, true) => zm - span,
            (false, false) => span - zm,
        };
        if z < 4.0 || (ring && z < zm + 2.0) {
            return Err(EditRefused::NoRoom);
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Ok(z as u32)
    }

    /// **A gear of `teeth` on `body` meshing `mate`**: a gear of the
    /// crate's defaults — a ring cut by the default pinion cutter — its
    /// count the one thing the edit decides; its module, pressure angle and
    /// helix automatic, so the mesh it joins gives it the group's and the
    /// hand its mesh needs; its shift and thickness automatic, so a second
    /// central at one carrier radius is closed by its shift; and its face
    /// width automatic, as every gear the panel lays in is, seeded at the
    /// width every gear is born with ([`super::DEFAULT_FACE_WIDTH`]). It
    /// reads nothing of its mate but the seed of its automatic module box: a
    /// gear never copies another's width, given helix, form or material.
    fn push_follower(&mut self, mate: usize, body: usize, teeth: u32, ring: bool) {
        let module = self.members[mate].normal_module();
        let new = self.push_member(body, teeth, module, ring.then(Default::default));
        self.members[new].gear.face_width = Auto::automatic(super::DEFAULT_FACE_WIDTH);
        self.push_mesh(new, mate);
    }

    /// **Another ratio across `distance`**, sharing `shared`
    /// ([`Edit::AddRatio`]): the first mesh there copied, its gear on the
    /// shared body's axis onto the shared body and the other onto a new
    /// body of its own axis.
    fn add_ratio(
        &mut self,
        distance: usize,
        shared: usize,
        next: usize,
    ) -> Result<(), EditRefused> {
        if distance >= self.distances.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        let first = self
            .indexed()
            .meshes_on(distance)
            .first()
            .copied()
            .ok_or(EditRefused::NoSuchIndex)?;
        let m = self.meshes[first];
        let shared_axis = self.axis_of_body(shared).ok_or(EditRefused::NoSuchIndex)?;
        let axis_of = |i: usize| self.axis_of_slot(self.slot_of_member(i));
        let (on_shared, alone) = if axis_of(m.a) == Some(shared_axis) {
            (m.a, m.b)
        } else if axis_of(m.b) == Some(shared_axis) {
            (m.b, m.a)
        } else {
            return Err(EditRefused::NotOnTheAxis);
        };
        let axis = axis_of(alone).ok_or(EditRefused::NoSuchIndex)?;
        self.members.push(Member {
            body: shared,
            ..self.members[on_shared].clone()
        });
        let body = self.push_body(axis, next);
        self.members.push(Member {
            body,
            ..self.members[alone].clone()
        });
        let n = self.members.len();
        self.push_mesh(n - 2, n - 1);
        Ok(())
    }

    /// **A step on carried `axis`** ([`Edit::AddStep`]): the last gear on
    /// its planet body copied beside it, and a ring meshing the copy.
    fn add_step(&mut self, axis: usize, next: usize) -> Result<(), EditRefused> {
        if axis >= self.axes.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        if !self.carried(axis) {
            return Err(EditRefused::NotCarried);
        }
        let body = self
            .bodies
            .iter()
            .find(|b| b.axis == axis)
            .map(|b| b.body)
            .ok_or(EditRefused::NoSuchIndex)?;
        let last = self
            .members_on_body(body)
            .last()
            .copied()
            .ok_or(EditRefused::NoSuchIndex)?;
        let gear = Member {
            ring: None,
            ..self.members[last].clone()
        };
        self.members.push(gear);
        let new = self.members.len() - 1;
        let central = self.central_axis_of(axis).ok_or(EditRefused::NoSuchIndex)?;
        self.add_gear(new, Place::NewBody(central), true, next)
    }

    // ---------------------------------------------------------- removing ---

    /// **A piece taken out, with what goes with it** ([`Edit::Remove`]).
    fn remove(&mut self, piece: Piece) -> Result<(), EditRefused> {
        match piece {
            Piece::Member(i) => {
                if i >= self.members.len() {
                    return Err(EditRefused::NoSuchIndex);
                }
                self.cascade(vec![i]);
            }
            Piece::Mesh(k) => {
                if k >= self.meshes.len() {
                    return Err(EditRefused::NoSuchIndex);
                }
                let m = self.meshes.remove(k);
                let at = self.indexed();
                let loose = [m.a, m.b]
                    .into_iter()
                    .filter(|&i| at.meshes_of(i).is_empty())
                    .collect();
                self.cascade(loose);
            }
            Piece::Axis(axis) => {
                if axis >= self.axes.len() {
                    return Err(EditRefused::NoSuchIndex);
                }
                if self.turns_a_carrier(axis) {
                    return Err(EditRefused::CentralAxis);
                }
                let gears = (0..self.members.len())
                    .filter(|&i| self.axis_of_slot(self.slot_of_member(i)) == Some(axis))
                    .collect();
                self.clear_axis(axis, gears);
                return Ok(());
            }
            Piece::Body(body) => {
                if !self.bodies.iter().any(|b| b.body == body) {
                    return Err(EditRefused::NoSuchIndex);
                }
                if self.carries_an_axis(body) {
                    return Err(EditRefused::CarriesAnAxis);
                }
                self.cascade(self.members_on_body(body));
                self.drop_bodies(&[body]);
            }
            Piece::Coupling(c) => return self.uncouple(c),
        }
        self.tidy();
        Ok(())
    }

    /// **Members taken out with their meshes, and every gear that leaves
    /// meshing nothing after them**, until none does: a gear in no mesh is
    /// no mechanism, and the idler of a chain goes with the chain's end.
    fn cascade(&mut self, mut going: Vec<usize>) {
        while !going.is_empty() {
            going.sort_unstable();
            going.dedup();
            for &i in going.iter().rev() {
                self.drop_member(i);
            }
            let at = self.indexed();
            going = (0..self.members.len())
                .filter(|&i| at.meshes_of(i).is_empty())
                .collect();
        }
    }

    /// **`gears` taken off `axis`, and the axis with them**: every body on
    /// it — a shaft a coupling turns too, with its coupling, since the axis
    /// it stands on is what was asked to go — every distance left with no
    /// mesh, and the axis, the axes after it numbered down. A body a case
    /// loads, reacts or sweeps at keeps the removal from being made
    /// ([`super::Train::edit`]).
    fn clear_axis(&mut self, axis: usize, gears: Vec<usize>) {
        let on: Vec<usize> = self
            .bodies
            .iter()
            .filter(|b| b.axis == axis)
            .map(|b| b.body)
            .collect();
        self.cascade(gears);
        self.drop_bodies(&on);
        self.tidy();
    }

    /// **Whether a body is bare** — the one rule every place that gives a
    /// body up asks: no gear on it, it carries no axis, and no coupling
    /// holds it on a fixed axis. A shaft on a fixed axis a coupling turns
    /// is a port the train may load, and stays; an orbiting body with
    /// nothing on it turns nothing its coupling could carry, and goes with
    /// its couplings ([`Self::drop_bodies`]). The train gives a bare body up
    /// where nothing else names it ([`super::Train::drop_bare`]).
    pub(crate) fn is_bare(&self, body: usize) -> bool {
        let coupled = self.couplings.iter().any(|c| c.contains(&body));
        self.members_on_body(body).is_empty()
            && !self.carries_an_axis(body)
            && (self.orbits(body) || !coupled)
    }

    /// Whether a body is on an axis a carrier turns.
    pub(crate) fn orbits(&self, body: usize) -> bool {
        self.axis_of_body(body).is_some_and(|a| self.carried(a))
    }

    /// Bodies taken out by number, with every coupling they are in.
    pub(crate) fn drop_bodies(&mut self, bodies: &[usize]) {
        self.bodies.retain(|b| !bodies.contains(&b.body));
        self.couplings
            .retain(|c| !c.iter().any(|b| bodies.contains(b)));
    }

    /// Every distance left with no mesh on it, and every axis left with
    /// nothing on it, taken out.
    fn tidy(&mut self) {
        let s = self.indexed();
        let meshless: Vec<usize> = (0..self.distances.len())
            .filter(|&d| s.meshes_on(d).is_empty())
            .collect();
        for &d in meshless.iter().rev() {
            self.distances.remove(d);
        }
        self.drop_empty_axes();
    }

    /// Whether a body on `axis` carries an axis: `axis` is the one a
    /// carrier turns about.
    fn turns_a_carrier(&self, axis: usize) -> bool {
        self.axes
            .iter()
            .any(|a| a.carried_by != GROUND && self.axis_of_body(a.carried_by) == Some(axis))
    }

    // ------------------------------------------------------------ moving ---

    /// **A member moved** ([`Edit::Move`]): onto another body of its axis,
    /// or a new one — where it shares its body; alone on it, it is on one
    /// of its own already, and nothing changes.
    fn move_body(
        &mut self,
        member: usize,
        body: Option<usize>,
        next: usize,
    ) -> Result<(), EditRefused> {
        if member >= self.members.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        let from = self.members[member].body;
        let axis = self.axis_of_body(from).ok_or(EditRefused::NoSuchIndex)?;
        let to = match body {
            Some(b) => {
                if b == GROUND || self.slot(b) == GROUND {
                    return Err(EditRefused::NoSuchIndex);
                }
                if self.axis_of_body(b) != Some(axis) {
                    return Err(EditRefused::NotOnTheAxis);
                }
                if self.carries_an_axis(b) {
                    return Err(EditRefused::CarriesAnAxis);
                }
                b
            }
            // Alone on its body, it is already on one of its own.
            None if self.members_on_body(from).len() == 1 => return Ok(()),
            None => self.push_body(axis, next),
        };
        self.members[member].body = to;
        // **The body it leaves stays.** A body the graph lists is a port the
        // train may hold, share or load, and dropping it because its gear
        // moved would take the coupling with it — which is how engaging a
        // layshaft's other ratio used to lose the output. A body with
        // nothing on it is a shaft with nothing driving it, which is what
        // *neutral* is, and the motion says so by being a family one
        // condition short. What no longer has a reason to exist is given
        // up a level up ([`super::Train::edit`]), where what else names a
        // body can be seen.
        Ok(())
    }

    // --------------------------------------------------------- couplings ---

    fn couple(&mut self, body: usize, next: usize) -> Result<(), EditRefused> {
        let axis = self.axis_of_body(body).ok_or(EditRefused::NoSuchIndex)?;
        if !self.carried(axis) {
            return Err(EditRefused::NotCarried);
        }
        if self.couplings.iter().any(|c| c.contains(&body)) {
            return Err(EditRefused::Coupled);
        }
        let central = self.central_axis_of(axis).ok_or(EditRefused::NoSuchIndex)?;
        self.bodies.push(super::shape::BodyOn {
            body: next,
            axis: central,
        });
        self.couplings.push([next, body]);
        Ok(())
    }

    fn uncouple(&mut self, coupling: usize) -> Result<(), EditRefused> {
        if coupling >= self.couplings.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        let joined = self.couplings.remove(coupling);
        for body in joined {
            self.drop_if_bare(body);
        }
        Ok(())
    }

    // ---------------------------------------------------------- the drops ---

    /// A body off the graph, with its couplings, where it is bare
    /// ([`Self::is_bare`]).
    fn drop_if_bare(&mut self, body: usize) {
        if self.is_bare(body) {
            self.drop_bodies(&[body]);
        }
    }

    /// A member gone, with its meshes, and its body off the graph where the
    /// member leaves it bare ([`Self::drop_if_bare`]) — a shaft a planet's
    /// turn is taken off to stays with its coupling, while a planet body
    /// left with nothing on it goes with the coupling it turned.
    fn drop_member(&mut self, member: usize) {
        let body = self.members[member].body;
        self.meshes.retain(|m| m.a != member && m.b != member);
        for m in &mut self.meshes {
            if m.a > member {
                m.a -= 1;
            }
            if m.b > member {
                m.b -= 1;
            }
        }
        self.members.remove(member);
        self.drop_if_bare(body);
    }
}

/// **An invariant of a well-formed train, broken**, naming what breaks
/// it — what [`super::Train::check`] finds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Invariant {
    /// A body listed on an axis the graph has not.
    BodyOnNoAxis(usize),
    /// A body listed twice.
    BodyListedTwice(usize),
    /// A member on a body the graph does not list.
    MemberOnNoBody(usize),
    /// A member in no mesh.
    MemberInNoMesh(usize),
    /// A mesh whose two members are on one axis.
    MeshOnOneAxis(usize),
    /// A mesh across no axis distance.
    MeshAcrossNoDistance(usize),
    /// An axis distance with no mesh across it.
    DistanceWithNoMesh(usize),
    /// An axis distance stated twice, either way round.
    DistanceTwice(usize),
    /// A mesh whose first member is a ring: a ring is a mesh's second.
    RingFirst(usize),
    /// A mesh between two rings, which is no mesh.
    TwoRings(usize),
    /// A ring meshing across crossed axes: the screw model has no internal
    /// kind.
    RingCrossed(usize),
    /// An axis distance between two axes that stand still in no one
    /// frame — neither carried by one body nor one turning about the other.
    DistanceOffFrame(usize),
    /// An axis with no body and no distance.
    AxisWithNothing(usize),
    /// A carried axis whose carrier is on no other axis.
    CarriedByNothing(usize),
    /// An axis carried round, through its carriers, by itself.
    CarriedInACycle(usize),
    /// A planet gear meeting nothing on its carrier's axis.
    PlanetMeetsNothing,
    /// A graph with no member that still lists a body: an empty train
    /// lists nothing, and its cases wait by number.
    ListedWhileEmpty(usize),
    /// A hold at a body the graph does not list.
    HoldUnlisted(usize),
    /// A case entry at a body that is not an open port of a train with
    /// members.
    EntryNotOpen { case: usize, body: usize },
    /// Two entries of one case at one body.
    EntryTwice { case: usize, body: usize },
    /// A case with entries whose sweep is measured at a body that is not
    /// an open port.
    SweepNotOpen { case: usize, body: usize },
    /// A number below the largest that nothing names.
    NumberGap(usize),
}

impl Shape {
    /// **The turning pairs as a tree** ([`CarrierTree`]): ground and each
    /// listed body, the body hung from the carrier of the axis it turns
    /// about — ground for a fixed axis, nothing for a carrier the shape
    /// does not list. Vertex 0 is ground and vertex `s` the body in slot `s`.
    pub(crate) fn carrier_tree(&self) -> CarrierTree {
        CarrierTree::new(
            std::iter::once(None)
                .chain(self.bodies.iter().map(|b| {
                    let carrier = self.axes.get(b.axis)?.carried_by;
                    if carrier == GROUND {
                        Some(GROUND)
                    } else {
                        self.slot_if_any(carrier)
                    }
                }))
                .collect(),
        )
    }

    /// **Whether every carried axis hangs from ground**: each axis carried
    /// by ground or by a body on another axis, and ground reached down the
    /// carriers ([`Self::carrier_tree`]) — the turning pairs a tree rooted
    /// at ground. The one statement of the rule, read by
    /// [`super::Train::check`] and by [`super::Train::validate`] where
    /// input enters.
    ///
    /// # Errors
    ///
    /// [`Invariant::CarriedByNothing`] or [`Invariant::CarriedInACycle`],
    /// naming the first axis that breaks it; an axis whose carriers stop at
    /// one that hangs from nothing is left to that one's own turn.
    pub fn carriers(&self) -> Result<(), Invariant> {
        let tree = self.carrier_tree();
        for (a, x) in self.axes.iter().enumerate() {
            if x.carried_by == GROUND {
                continue;
            }
            let Some(c) = self
                .slot_if_any(x.carried_by)
                .filter(|&c| self.bodies[c - 1].axis != a)
            else {
                return Err(Invariant::CarriedByNothing(a));
            };
            if tree.hang(c) == Hang::Cycle {
                return Err(Invariant::CarriedInACycle(a));
            }
        }
        Ok(())
    }

    /// **One axis distance per pair of axes**, either way round.
    fn distances_once(&self) -> Result<(), Invariant> {
        for (d, x) in self.distances.iter().enumerate() {
            let same =
                |y: &super::shape::Distance| y.axes == x.axes || y.axes == [x.axes[1], x.axes[0]];
            if self.distances.iter().filter(|y| same(y)).count() > 1 {
                return Err(Invariant::DistanceTwice(d));
            }
        }
        Ok(())
    }

    /// **Every mesh has a kind**, read off its members
    /// ([`kind_of`](super::incidence::Indexed::kind_of)): two rings make no
    /// mesh, a ring is its mesh's second member — the side a mesh's kind is
    /// read on, which [`Self::order_meshes`] puts it on where a file comes
    /// in — and a ring does not mesh across crossed axes.
    fn meshes_have_a_kind(&self) -> Result<(), Invariant> {
        let ring = |i: usize| self.members.get(i).is_some_and(|x| x.ring.is_some());
        let axis = |i: usize| {
            let body = self.members.get(i)?.body;
            self.bodies.iter().find(|b| b.body == body).map(|b| b.axis)
        };
        let crossed = |a: usize, b: usize| match (axis(a), axis(b)) {
            (Some(p), Some(q)) => self
                .distances
                .iter()
                // A shaft angle that is no number is the figure's to refuse,
                // by its field, once the graph is read.
                .any(|d| {
                    (d.axes == [p, q] || d.axes == [q, p]) && d.angle.is_finite() && d.angle != 0.0
                }),
            _ => false,
        };
        for (k, m) in self.meshes.iter().enumerate() {
            match (ring(m.a), ring(m.b)) {
                (true, true) => return Err(Invariant::TwoRings(k)),
                (true, false) => return Err(Invariant::RingFirst(k)),
                (false, true) if crossed(m.a, m.b) => return Err(Invariant::RingCrossed(k)),
                _ => {}
            }
        }
        Ok(())
    }

    /// **Every internal mesh written (gear, ring)**, the order its kind is
    /// read in: a mesh listing its ring first is turned round, which changes
    /// nothing a mesh owns (its inputs are the pair's, in either order).
    /// Whether any was turned. Asked where a file comes in; two rings stay as
    /// they are, for [`Self::validate`] to refuse.
    pub fn order_meshes(&mut self) -> bool {
        let ring = |members: &[super::shape::Member], i: usize| {
            members.get(i).is_some_and(|x| x.ring.is_some())
        };
        let mut turned = false;
        for k in 0..self.meshes.len() {
            let m = self.meshes[k];
            if ring(&self.members, m.a) && !ring(&self.members, m.b) {
                self.meshes[k].a = m.b;
                self.meshes[k].b = m.a;
                turned = true;
            }
        }
        turned
    }

    /// **Every axis distance holds in one frame** — one axis per line in a
    /// frame: its two axes carried by one body, or one of them the axis the
    /// other's carrier turns about. A distance from a carried axis to any
    /// other axis changes as the carrier turns; where it was meant not to,
    /// the other axis is the carrier's own line entered twice
    /// (`work/design-graph.md`, *Where this may be wrong*).
    fn distances_in_a_frame(&self) -> Result<(), Invariant> {
        let turns_about =
            |body: usize, axis: usize| body != GROUND && self.axis_of_body(body) == Some(axis);
        for (d, x) in self.distances.iter().enumerate() {
            let [p, q] = x.axes;
            let (Some(fp), Some(fq)) = (self.axes.get(p), self.axes.get(q)) else {
                continue;
            };
            let (fp, fq) = (fp.carried_by, fq.carried_by);
            if fp != fq && !turns_about(fp, q) && !turns_about(fq, p) {
                return Err(Invariant::DistanceOffFrame(d));
            }
        }
        Ok(())
    }

    /// **What a graph must satisfy before anything reads it** — the one
    /// set of the graph's invariants input can break, read where input
    /// enters ([`Self::validate`]) and by every edit's check
    /// ([`super::Train::check`]): the carriers a tree rooted at ground
    /// ([`Self::carriers`]), one distance per pair of axes, every mesh of a
    /// kind (no two rings, a ring its mesh's second member and on parallel
    /// axes), and every distance held in one frame. Whether
    /// a loop of distances on a carrier closes is a question of their
    /// values, asked once they are known (`Indexed::frames_close`).
    ///
    /// # Errors
    ///
    /// The first [`Invariant`] broken, in that order.
    pub fn invariants(&self) -> Result<(), Invariant> {
        self.carriers()?;
        self.distances_once()?;
        self.meshes_have_a_kind()?;
        self.distances_in_a_frame()
    }

    /// **Every number in the graph read against its row
    /// ([`crate::input::shape`]), then [`Self::invariants`], as the solve
    /// refuses** — before anything reads the graph, so an index past its
    /// list cannot be followed, a carrier cycle cannot send a walk down it
    /// round for ever, and a graph that describes no train is refused by
    /// name, not by the symptom a later reading meets.
    ///
    /// # Errors
    ///
    /// [`super::TrainError::Input`], naming the field, or
    /// [`super::TrainError::Malformed`], naming the invariant and its piece.
    pub fn validate(&self) -> Result<(), super::TrainError> {
        crate::input::shape(self).map_err(super::TrainError::Input)?;
        self.invariants().map_err(super::TrainError::Malformed)
    }
}

impl super::Train {
    /// **What input must satisfy before anything reads it** — every number
    /// in it read against its row ([`crate::input::train`]: its graph's,
    /// its holds' and its cases'), then its graph's invariants
    /// ([`Shape::invariants`]) and its numbering. Called where a train
    /// enters: the solve, a file read, and every wasm entry point.
    ///
    /// # Errors
    ///
    /// [`super::TrainError::Input`], naming the field, or
    /// [`super::TrainError::Malformed`], naming the invariant and its piece.
    pub fn validate(&self) -> Result<(), super::TrainError> {
        self.validate_graph()?;
        crate::input::figures(self).map_err(super::TrainError::Input)
    }

    /// **What reading the train's graph needs**: every index in it naming
    /// something ([`crate::input::graph`]), its graph's invariants
    /// ([`Shape::invariants`]) and its numbering — what its parts, ports
    /// and names are read off, whatever its figures hold.
    ///
    /// # Errors
    ///
    /// [`super::TrainError::Input`], naming the field, or
    /// [`super::TrainError::Malformed`], naming the invariant and its piece.
    pub fn validate_graph(&self) -> Result<(), super::TrainError> {
        crate::input::graph(self).map_err(super::TrainError::Input)?;
        self.shape
            .invariants()
            .map_err(super::TrainError::Malformed)?;
        // **The graph's bodies numbered from 1 without a gap**, the number
        // every part, case and hold names a body by. Every number the graph
        // lists is one of its `n` (the rows read it), so a number of those
        // `n` that nothing names is one the list holds twice: no body at all.
        let s = &self.shape;
        match (1..=s.bodies.len()).find(|&b| s.slot_if_any(b).is_none()) {
            Some(b) => Err(super::TrainError::Malformed(Invariant::NumberGap(b))),
            None => Ok(()),
        }
    }

    /// **Whether the train is well formed** — what every edit keeps.
    ///
    /// The graph has nothing hanging: every gear on a listed body on an
    /// axis and in a mesh, every mesh across a distance between two axes,
    /// one distance per pair of axes and each carrying a mesh, no axis with
    /// nothing on it, every carried axis carried by a body on another and
    /// no carrier carried by what it carries, every planet meeting its
    /// carrier's axis; and a graph with no member lists nothing.
    ///
    /// What hangs off it names it rightly: every hold at a listed body;
    /// on a train with members, every case entry and every sweep of a case
    /// with entries at an open port, and no case with two entries at one
    /// body; and the body numbers dense, every one from 1 to the largest
    /// named by the graph, a hold or a case.
    ///
    /// # Errors
    ///
    /// The first [`Invariant`] broken.
    pub fn check(&self) -> Result<(), Invariant> {
        let s = &self.shape;
        let axis_of = |body: usize| s.bodies.iter().find(|b| b.body == body).map(|b| b.axis);
        for (i, b) in s.bodies.iter().enumerate() {
            if b.axis >= s.axes.len() {
                return Err(Invariant::BodyOnNoAxis(b.body));
            }
            if s.bodies[..i].iter().any(|x| x.body == b.body) {
                return Err(Invariant::BodyListedTwice(b.body));
            }
        }
        let at = s.indexed();
        for (i, m) in s.members.iter().enumerate() {
            if axis_of(m.body).is_none() {
                return Err(Invariant::MemberOnNoBody(i));
            }
            if at.meshes_of(i).is_empty() {
                return Err(Invariant::MemberInNoMesh(i));
            }
        }
        for (k, m) in s.meshes.iter().enumerate() {
            if axis_of(s.members[m.a].body) == axis_of(s.members[m.b].body) {
                return Err(Invariant::MeshOnOneAxis(k));
            }
            if at.distance_of(k).is_none() {
                return Err(Invariant::MeshAcrossNoDistance(k));
            }
        }
        for d in 0..s.distances.len() {
            if at.meshes_on(d).is_empty() {
                return Err(Invariant::DistanceWithNoMesh(d));
            }
        }
        for a in 0..s.axes.len() {
            if !s.bodies.iter().any(|b| b.axis == a)
                && !s.distances.iter().any(|d| d.axes.contains(&a))
            {
                return Err(Invariant::AxisWithNothing(a));
            }
        }
        s.invariants()?;
        if !s.planets_at_a_radius() {
            return Err(Invariant::PlanetMeetsNothing);
        }
        if s.members.is_empty() {
            if let Some(b) = s.bodies.first() {
                return Err(Invariant::ListedWhileEmpty(b.body));
            }
        }
        for &h in &self.held {
            if axis_of(h).is_none() {
                return Err(Invariant::HoldUnlisted(h));
            }
        }
        if !s.members.is_empty() {
            let open: Vec<usize> = self.open_ports().iter().map(|p| p.body).collect();
            for (c, case) in self.load_cases.iter().enumerate() {
                for (i, l) in case.loads.iter().enumerate() {
                    let body = l.at;
                    if !open.contains(&body) {
                        return Err(Invariant::EntryNotOpen { case: c, body });
                    }
                    if case.loads[..i].iter().any(|x| x.at == body) {
                        return Err(Invariant::EntryTwice { case: c, body });
                    }
                }
                if let super::Duty::Intermittent { at: Some(at), .. } = case.duty {
                    if !case.loads.is_empty() && !open.contains(&at) {
                        return Err(Invariant::SweepNotOpen { case: c, body: at });
                    }
                }
            }
        }
        let named = |b: usize| {
            axis_of(b).is_some() || self.held.contains(&b) || self.load_cases.iter().any(|c| {
                c.loads.iter().any(|l| l.at == b)
                    || matches!(c.duty, super::Duty::Intermittent { at: Some(at), .. } if at == b)
            })
        };
        if let Some(b) = (1..=self.max_body()).find(|&b| !named(b)) {
            return Err(Invariant::NumberGap(b));
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    //! **The laws an edit obeys**: it leaves a stage that solves, it undoes,
    //! it refuses whole, and what it renumbers the train follows.

    use super::super::arrangements::{self as arr, Preset};
    use super::super::testing::{alone, grid};
    use super::super::{solve_train, test_library as library, LoadCase, LoadRole, Shape, Train};
    use super::*;

    /// The shape asked with `held` held, loaded at `input` and reacted at
    /// `output` — its slots, as a train of one numbers them.
    fn asked(shape: &Shape, held: &[usize], input: usize, output: usize) -> crate::train::Alone {
        crate::train::solve_alone(
            &Train::alone(shape, 2.0, 3000.0).arranged(held, input, output),
            &library(),
        )
        .unwrap_or_else(|e| panic!("{e}: {shape:?}"))
    }

    fn same(a: &Shape, b: &Shape) -> bool {
        format!("{a:?}") == format!("{b:?}")
    }

    /// **A graph that describes no train is refused where it enters, by
    /// the key that names why** — each fixture refused by
    /// [`solve_train`]'s [`Train::validate`] with its own catalogue key, and
    /// beside each the near miss that stands: a carrier cycle of three
    /// axes; an axis distance stated twice, the same way round and the
    /// other; a mesh listing its ring first; a sun on an axis entry of its
    /// own that the carrier does not turn about — two entries for one line
    /// — beside the set as built and meshed planets; and a body number
    /// skipped. A loop of distances on a carrier is no fault of the graph's
    /// shape: whether it closes is its values' (`frame_closure` in
    /// `shape.rs`).
    /// **A gear is born at one width, wherever it is born** (Q5 to Q6): a gear
    /// an edit adds is seeded at [`super::super::DEFAULT_FACE_WIDTH`], as the
    /// core's default and the panel's presets are — not at its mate's box,
    /// which it once copied (a gear never reads its mate). A mate at 7 mm
    /// given, and one at 7 mm automatic: the new gear is automatic at the
    /// default either way.
    #[test]
    fn an_added_gear_is_born_at_the_default_width() {
        for mate_width in [Auto::fixed(7.0), Auto::automatic(7.0)] {
            let mut t = Train::alone(&arr::pair([17, 43]), 2.0, 3000.0);
            t.shape.members[1].gear.face_width = mate_width;
            t.edit(Edit::AddGear {
                mate: 1,
                on: Place::NewAxis,
                ring: false,
            })
            .unwrap();
            let new = &t.shape.members.last().unwrap().gear;
            assert_eq!(
                new.face_width,
                Auto::automatic(super::super::DEFAULT_FACE_WIDTH),
                "against a mate at {mate_width:?}"
            );
            assert_eq!(
                super::super::MemberGear::default().face_width.manual,
                new.face_width.manual
            );
        }
    }

    #[test]
    fn malformed_graphs_are_refused_where_they_enter_by_name() {
        use crate::note::Explain;
        let lib = library();
        let train = |shape: Shape, load_cases: Vec<LoadCase>| Train {
            load_cases,
            reversed_bending: false,
            shape,
            held: Vec::new(),
        };
        let refused = |name: &str, t: &Train, key: &str| match solve_train(t, &lib) {
            Err(e) => assert_eq!(e.note().key, key, "{name}: {e:?}"),
            Ok(_) => panic!("{name}: solved, where it should be refused by {key}"),
        };
        let stands = |name: &str, t: &Train| {
            assert!(t.validate().is_ok(), "{name}: {:?}", t.validate());
        };
        let body_on =
            |s: &Shape, axis: usize| s.bodies.iter().find(|b| b.axis == axis).unwrap().body;
        let mut checked = 0;

        // A carrier cycle of three axes: each fixed axis of an idler's
        // line carried by the body on the next.
        let mut cycle = Preset::Idler.build();
        for a in 0..3 {
            cycle.axes[a].carried_by = body_on(&cycle, (a + 1) % 3);
        }
        refused(
            "a carrier cycle",
            &train(cycle, Vec::new()),
            "error.train_malformed_carried_by_cycle",
        );
        checked += 1;

        // An axis distance twice, the same way round and the other.
        for turned in [false, true] {
            let mut twice = Preset::Spur.build();
            let mut again = twice.distances[0];
            if turned {
                again.axes = [again.axes[1], again.axes[0]];
            }
            twice.distances.push(again);
            refused(
                "a distance twice",
                &train(twice, Vec::new()),
                "error.train_malformed_distance_twice",
            );
            checked += 1;
        }
        stands("a pair", &train(Preset::Spur.build(), Vec::new()));

        // A mesh that lists its ring first.
        let mut ring_first = Preset::Planetary.build();
        let k = (0..ring_first.meshes.len())
            .find(|&k| ring_first.members[ring_first.meshes[k].b].ring.is_some())
            .unwrap();
        let m = &mut ring_first.meshes[k];
        (m.a, m.b) = (m.b, m.a);
        refused(
            "a ring first",
            &train(ring_first, Vec::new()),
            "error.train_malformed_ring_first",
        );
        checked += 1;
        stands("a set", &train(Preset::Planetary.build(), Vec::new()));

        stands(
            "meshed planets",
            &train(Preset::MeshedPlanets.build(), Vec::new()),
        );

        // A sun on an axis entry of its own, the same line as the carrier's
        // in fact and another axis in the graph: its distance to the planet
        // axis holds in no frame.
        let mut own_line = Preset::Planetary.build();
        let sun = own_line.members[0].body;
        let planet_axis = own_line.members[1].body;
        let planet_axis = own_line
            .bodies
            .iter()
            .find(|b| b.body == planet_axis)
            .unwrap()
            .axis;
        own_line.axes.push(own_line.axes[0]);
        let entry = own_line.axes.len() - 1;
        own_line
            .bodies
            .iter_mut()
            .find(|b| b.body == sun)
            .unwrap()
            .axis = entry;
        let d = (0..own_line.distances.len())
            .find(|&d| own_line.distances[d].axes.contains(&planet_axis))
            .unwrap();
        let mut sun_distance = own_line.distances[d];
        sun_distance.axes = [entry, planet_axis];
        own_line.distances.push(sun_distance);
        refused(
            "two entries for one line",
            &train(own_line, Vec::new()),
            "error.train_malformed_distance_off_frame",
        );
        checked += 1;

        // A body number skipped: a pair's second body numbered 3 — past the
        // two the graph lists, so its own field names it (`crate::input`).
        let mut gap = Preset::Spur.build();
        gap.renumber_bodies(|b| if b == 2 { 3 } else { b });
        let cases = vec![LoadCase::ultimate(1, 3, 2.0, 3000.0)];
        refused(
            "a body number skipped",
            &train(gap, cases),
            "error.input_out_of_range",
        );
        checked += 1;
        // ...and one listed twice, which leaves a number inside the list
        // that nothing names: the gap.
        let mut twice = Preset::Spur.build();
        twice.renumber_bodies(|_| 1);
        refused(
            "a body number listed twice",
            &train(twice, Vec::new()),
            "error.train_malformed_number_gap",
        );
        checked += 1;
        stands(
            "a pair, cased",
            &train(
                Preset::Spur.build(),
                vec![LoadCase::ultimate(1, 2, 2.0, 3000.0)],
            ),
        );

        assert_eq!(checked, 7);
    }

    /// **Every train an edit makes validates, and so does every part of
    /// it** — the rules input is held to refuse nothing the walk of offered
    /// edits reaches, and read no train's numbering into a part's: a part
    /// lists the train's bodies, not a train's own.
    #[test]
    fn every_walked_train_and_its_parts_validate() {
        let mut parts = 0;
        let visited = super::super::sweep::visited();
        for (walk, steps, t) in &visited {
            t.validate()
                .unwrap_or_else(|e| panic!("{walk}: {steps:?}: {e:?}"));
            for part in t.parts() {
                part.shape
                    .invariants()
                    .unwrap_or_else(|e| panic!("{walk}: {steps:?}: a part: {e:?}"));
                parts += 1;
            }
        }
        // Each walk's start and, on average, more than three of its steps.
        let least = super::super::sweep::WALKS * 4;
        assert!(
            visited.len() > least && parts > least,
            "{} trains, {parts} parts",
            visited.len()
        );
    }

    /// A shape of bare axes and bodies: `carried_by` per axis, the bodies
    /// as `(body, axis)`, and `pairs` as its axis distances — no gear, for
    /// the laws about the frames alone.
    fn frames(carried_by: &[usize], bodies: &[(usize, usize)], pairs: &[[usize; 2]]) -> Shape {
        let template = Preset::Planetary.build();
        Shape {
            axes: carried_by
                .iter()
                .map(|&c| super::super::shape::Axis {
                    carried_by: c,
                    ..template.axes[0]
                })
                .collect(),
            bodies: bodies
                .iter()
                .map(|&(body, axis)| super::super::shape::BodyOn { body, axis })
                .collect(),
            distances: pairs
                .iter()
                .map(|&axes| super::super::shape::Distance {
                    axes,
                    ..template.distances[0]
                })
                .collect(),
            ..Shape::default()
        }
    }

    /// **A distance holds in one frame exactly where it cannot change as
    /// the carriers turn** — the rule against the geometry it stands for.
    /// Every chain of carriers over five axes, each carried by ground or by
    /// a body on an earlier axis (the first axis has a second body, so two
    /// carriers turn about one line), every pair of axes as the one
    /// distance: each axis placed in the plane — a fixed one at a point of
    /// its own, a carried one at an offset from its carrier's axis turned
    /// by its carrier's angle — at three sets of angles. The rule refuses
    /// exactly the distances that move. Positions are seeded and generic,
    /// so a distance that moves moves by the order of its offsets, far
    /// from the rounding a constant one shows.
    #[test]
    fn a_distance_holds_in_a_frame_where_the_carriers_cannot_move_it() {
        const AXES: usize = 5;
        // Bodies: one per axis, `axis + 1`, and a second on axis 0.
        let second = AXES + 1;
        let mut bodies: Vec<(usize, usize)> = (0..AXES).map(|a| (a + 1, a)).collect();
        bodies.push((second, 0));
        let body_axis = |b: usize| bodies.iter().find(|x| x.0 == b).map(|x| x.1);
        let mut rng = super::super::sweep::Lcg(0x5eed);
        let mut unit = || rng.pick(1 << 20) as f64 / f64::from(1 << 20) - 0.5;
        let points: Vec<[f64; 2]> = (0..AXES).map(|_| [unit(), unit()]).collect();
        let angles: Vec<Vec<f64>> = (0..3)
            .map(|_| (0..=second).map(|_| 6.0 * unit()).collect())
            .collect();
        let (mut held, mut moved, mut widest, mut narrowest) = (0, 0, 0.0_f64, f64::INFINITY); // absence: no distance measured yet
                                                                                               // Each axis's carrier: ground, or a body on an earlier axis.
        let choices: Vec<Vec<usize>> = (0..AXES)
            .map(|a| {
                std::iter::once(GROUND)
                    .chain(bodies.iter().filter(|b| b.1 < a).map(|b| b.0))
                    .collect()
            })
            .collect();
        let total: usize = choices.iter().map(Vec::len).product();
        for code in 0..total {
            let mut rest = code;
            let carried_by: Vec<usize> = choices
                .iter()
                .map(|c| {
                    let pick = c[rest % c.len()];
                    rest /= c.len();
                    pick
                })
                .collect();
            let place = |axis: usize, turn: &[f64]| -> [f64; 2] {
                let mut at = axis;
                let mut p = [0.0, 0.0];
                loop {
                    let c = carried_by[at];
                    if c == GROUND {
                        return [p[0] + points[at][0], p[1] + points[at][1]];
                    }
                    let (sin, cos) = turn[c].sin_cos();
                    let [x, y] = points[at];
                    // The offset turned by the carrier, the carrier's axis
                    // the rest of the way.
                    p = [
                        cos * (p[0] + x) - sin * (p[1] + y),
                        sin * (p[0] + x) + cos * (p[1] + y),
                    ];
                    at = body_axis(c).unwrap();
                }
            };
            for p in 0..AXES {
                for q in p + 1..AXES {
                    let s = frames(&carried_by, &bodies, &[[p, q]]);
                    let spans: Vec<f64> = angles
                        .iter()
                        .map(|turn| {
                            let (a, b) = (place(p, turn), place(q, turn));
                            (a[0] - b[0]).hypot(a[1] - b[1])
                        })
                        .collect();
                    let change = spans
                        .iter()
                        .fold(0.0_f64, |m, x| m.max((x - spans[0]).abs()));
                    let refused = s.invariants() == Err(Invariant::DistanceOffFrame(0));
                    if refused {
                        narrowest = narrowest.min(change);
                        moved += 1;
                    } else {
                        widest = widest.max(change);
                        held += 1;
                    }
                }
            }
        }
        // Rounding moves a constant distance by a few ulp of its size; the
        // least a moving one moves is far above it.
        assert!(
            widest < 1e-12,
            "a distance the rule holds moved by {widest}"
        );
        assert!(
            narrowest > 1e-3,
            "a distance the rule refuses moved only {narrowest}"
        );
        assert_eq!(held + moved, total * AXES * (AXES - 1) / 2);
        assert!(held > 100 && moved > 100, "{held} held, {moved} moved");
    }

    /// An edit on a shape outside any train: what it adds is numbered after
    /// what the shape has, as a train of this one stage would number it.
    fn edit(shape: &mut Shape, edit: Edit) -> Result<(), EditRefused> {
        let next = shape.max_body() + 1;
        shape.apply(&edit, next)
    }

    /// A new body on the axis carried `axis` goes round: where a sun or a
    /// ring meshing its planet goes.
    fn central(shape: &Shape, axis: usize) -> Place {
        Place::NewBody(shape.central_axis_of(axis).unwrap())
    }

    /// The body another ratio across `distance` shares: its first mesh's
    /// second gear's.
    fn shared(shape: &Shape, distance: usize) -> usize {
        shape.members[shape.meshes[shape.indexed().meshes_on(distance)[0]].b].body
    }

    /// **What a designer adds to a preset first**, each on its first
    /// candidate: its first gear moved to a body of its own; another ratio
    /// across its first distance; on a set, a step, a ring and a sun on its
    /// first planet gear and a coupling from that gear's body; on a chain, a
    /// gear on a new axis at its last gear.
    fn applicable(shape: &Shape) -> Vec<Edit> {
        let carried = (0..shape.axes.len()).find(|&a| shape.axes[a].carried_by != GROUND);
        let mut out = vec![
            Edit::Move {
                member: 0,
                to: None,
            },
            Edit::AddRatio {
                distance: 0,
                shared: shared(shape, 0),
            },
        ];
        match carried {
            Some(axis) => {
                let gear = (0..shape.members.len())
                    .find(|&i| shape.axis_of_slot(shape.slot_of_member(i)) == Some(axis))
                    .unwrap();
                let on = central(shape, axis);
                out.push(Edit::AddStep { axis });
                out.push(Edit::AddGear {
                    mate: gear,
                    on,
                    ring: true,
                });
                let body = shape.members[gear].body;
                if !shape.couplings.iter().any(|c| c.contains(&body)) {
                    out.push(Edit::Couple { body });
                }
                // No sun fits inside a planocentric, and it says so.
                if shape.members.len() > 2 {
                    out.push(Edit::AddGear {
                        mate: gear,
                        on,
                        ring: false,
                    });
                }
            }
            None => out.push(Edit::AddGear {
                mate: shape.members.len() - 1,
                on: Place::NewAxis,
                ring: false,
            }),
        }
        out
    }

    /// **Every add on every preset leaves a stage that solves**, and the
    /// figure it leaves is a finite ratio: an added ring is not the count
    /// that locks a Wolfrom, an added sun fits inside the planets, an
    /// added axis meshes.
    #[test]
    fn every_add_on_every_preset_solves() {
        for preset in Preset::ALL {
            let base = preset.build();
            for edit in applicable(&base) {
                // A ratio on a crossed distance is a second point contact at
                // the same angle, which the worm's proportions do not size;
                // parallel shafts are what another ratio is for.
                if matches!(edit, Edit::AddRatio { .. })
                    && base.family() != arr::PresetFamily::Parallel
                {
                    continue;
                }
                let mut shape = base.clone();
                self::edit(&mut shape, edit.clone())
                    .unwrap_or_else(|e| panic!("{preset:?} {edit:?}: {e}"));
                let r = alone(&shape);
                assert!(
                    r.ratio.is_some_and(f64::is_finite),
                    "{preset:?} after {edit:?}: {:?}",
                    r.ratio
                );
                for m in &shape.members {
                    assert!(
                        shape
                            .meshes
                            .iter()
                            .any(|x| shape.members[x.a].body == m.body
                                || shape.members[x.b].body == m.body),
                        "{preset:?} after {edit:?}: a member in no mesh"
                    );
                }
            }
        }
    }

    /// **Every gear a preset offers is a gear of its own** (audit T13.7):
    /// on every preset, cased, and on each with its first gear's helix
    /// given (20°), every `AddGear` offered unrefused adds the crate's
    /// default gear at the count the edit sizes — its helix, module and
    /// pressure angle automatic, so the mesh gives it the group's and the
    /// hand it needs, its face width automatic at its mate's seed, and
    /// nothing of its mate's given helix, pitch diameter, form or material
    /// copied. The train it leaves solves, or its cases
    /// say the load divides by stiffness, or a part says by name why its
    /// geometry does not close; never a gear that cannot mesh its mate
    /// (`Mesh(Incompatible)`, which a copied given helix gave), nor a lock
    /// (refused as `Locks`), nor a wiring that is no mechanism. A preset
    /// that does not solve before the edit is skipped: its failure is not
    /// the gear's.
    #[test]
    fn every_gear_a_preset_offers_is_its_own() {
        use super::super::testing::cased;
        use super::super::{MemberGear, TrainError};
        let lib = library();
        let (mut solved, mut said, mut skipped) = (0, 0, 0);
        let mut failures: Vec<String> = Vec::new();
        for p in Preset::ALL {
            for helix in [None, Some(20.0)] {
                let shape = helix.map_or_else(|| p.build(), |b| p.build().with_first_helix(b));
                let t = cased(vec![shape]);
                if solve_train(&t, &lib).is_err() {
                    skipped += 1;
                    continue;
                }
                let mut seen: Vec<String> = Vec::new();
                for at in super::super::sweep::targets(&t) {
                    for o in t.offers(at) {
                        let Edit::AddGear { mate, .. } = o.edit else {
                            continue;
                        };
                        let named = format!("{:?}", o.edit);
                        if o.refused.is_some() || seen.contains(&named) {
                            continue;
                        }
                        seen.push(named.clone());
                        let context = format!("{p:?} helix {helix:?}, {named}");
                        let mut u = t.clone();
                        u.edit(o.edit.clone()).unwrap();
                        let new = u.shape.members.last().unwrap();
                        let own = MemberGear {
                            teeth: new.gear.teeth,
                            face_width: crate::params::Auto::automatic(
                                super::super::DEFAULT_FACE_WIDTH,
                            ),
                            ..MemberGear::default()
                        };
                        if format!("{:?}", new.gear) != format!("{own:?}")
                            || !(new.module.auto
                                && new.pressure_angle.auto
                                && new.pitch_diameter.auto)
                        {
                            failures.push(format!("{context}: not its own: {new:?}"));
                        }
                        match solve_train(&u, &lib) {
                            Ok(r) if r.cases.iter().all(|c| c.solved) => solved += 1,
                            Ok(r)
                                if r.cases.iter().any(|c| {
                                    c.notes
                                        .iter()
                                        .any(|n| n.is(crate::note::key::TRAIN_LOAD_SHARED))
                                }) =>
                            {
                                said += 1;
                            }
                            Err(TrainError::InPart { cause, .. })
                                if matches!(
                                    *cause,
                                    TrainError::NoCommonDistance
                                        | TrainError::TipsUnclearable { .. }
                                        | TrainError::FlankInterference
                                        | TrainError::Mesh(
                                            crate::mesh::MeshError::OutsideInvoluteDomain
                                        )
                                ) =>
                            {
                                said += 1;
                            }
                            other => failures.push(format!("{context}: {other:?}")),
                        }
                    }
                }
            }
        }
        assert!(
            failures.is_empty(),
            "{} failures:\n{}",
            failures.len(),
            failures.join("\n")
        );
        assert!(
            solved > 50 && said > 0 && skipped < Preset::ALL.len(),
            "solved {solved}, said {said}, skipped {skipped}"
        );
    }

    /// **No gear a train offers is a certain dead end** (audit T13.9): on
    /// every preset, and on planocentrics at one and two teeth of
    /// difference (18/19, 20/21, 30/31, 30/32, 18/20), cased, every
    /// `AddGear` offered unrefused solves, or ends with its load divided by
    /// stiffness (`load_shared`: an equal-ratio twin the model cannot
    /// share). A gear whose mesh contradicts the motion its bodies already
    /// have — a lock — is refused as `Locks` without a solve, and a ring
    /// round a planet that no count can close at the carrier radius as
    /// `NoRoom`: at one tooth of difference the only count that closes is
    /// the ring's own. At two, the count a tooth below the ring's closes,
    /// is offered and solves — which a floor of two teeth over the planet
    /// refused. The twins stay offered: a rule refusing every second mesh
    /// between two geared bodies fails here.
    #[test]
    fn no_gear_a_train_offers_is_a_dead_end() {
        use super::super::testing::cased;
        use crate::note::key;
        let lib = library();
        let shapes = Preset::ALL
            .into_iter()
            .map(|p| (format!("{p:?}"), p.build()))
            .map(|(name, shape)| (name, shape, None))
            .chain(
                [(18, 19), (20, 21), (30, 31), (30, 32), (18, 20)].map(|(p, r)| {
                    let name = format!("planocentric {p}/{r}");
                    (name, arr::planocentric(p, r), Some(r - p))
                }),
            );
        let (mut solved, mut twins, mut locks, mut rings) = (0, 0, 0, 0);
        let mut failures: Vec<String> = Vec::new();
        for (name, shape, difference) in shapes {
            let t = cased(vec![shape]);
            assert!(solve_train(&t, &lib).is_ok(), "{name} solves as laid in");
            let mut seen: Vec<String> = Vec::new();
            for at in super::super::sweep::targets(&t) {
                for o in t.offers(at) {
                    let Edit::AddGear { .. } = o.edit else {
                        continue;
                    };
                    let named = format!("{:?}", o.edit);
                    if seen.contains(&named) {
                        continue;
                    }
                    seen.push(named.clone());
                    if o.refused
                        .as_ref()
                        .is_some_and(|n| n.key == "ui.train_edit_refused_locks")
                    {
                        locks += 1;
                    }
                    // A second ring round a planocentric's planet.
                    if let (
                        Some(d),
                        Edit::AddGear {
                            mate: 0,
                            on: Place::NewBody(_),
                            ring: true,
                        },
                    ) = (difference, &o.edit)
                    {
                        let offered = o.refused.is_none();
                        assert_eq!(offered, d == 2, "{name}: {named} offered {offered}");
                        rings += usize::from(offered);
                    }
                    if o.refused.is_some() {
                        continue;
                    }
                    let mut u = t.clone();
                    u.edit(o.edit.clone()).unwrap();
                    match solve_train(&u, &lib) {
                        Ok(r) if r.cases.iter().all(|c| c.solved) => solved += 1,
                        Ok(r)
                            if r.cases
                                .iter()
                                .all(|c| c.notes.iter().any(|n| n.is(key::TRAIN_LOAD_SHARED))) =>
                        {
                            twins += 1;
                        }
                        other => failures.push(format!("{name}, {named}: {other:?}")),
                    }
                }
            }
        }
        assert!(
            failures.is_empty(),
            "{} dead ends offered:\n{}",
            failures.len(),
            failures.join("\n")
        );
        assert!(
            solved > 50 && twins > 0 && locks > 0 && rings == 2,
            "solved {solved}, twins {twins}, locks {locks}, rings {rings}"
        );
    }

    /// **Every add undoes**: the gear it added taken off again — a step's
    /// planet gear, whose ring goes with it meshing nothing — the axis, the
    /// ratio or the coupling removed, is the shape it was, field for field:
    /// the bodies it listed included, an added body being the last and its
    /// removal moving no other.
    #[test]
    fn every_add_undoes() {
        let wolfrom = Preset::Wolfrom.build();
        // Members: planet, ring 1, ring 2; the planet's axis is 1.
        let (planet, added) = (0, wolfrom.members.len());
        let on = central(&wolfrom, 1);
        for add in [
            Edit::AddGear {
                mate: planet,
                on,
                ring: true,
            },
            Edit::AddGear {
                mate: planet,
                on,
                ring: false,
            },
            Edit::AddStep { axis: 1 },
        ] {
            let mut shape = wolfrom.clone();
            edit(&mut shape, add.clone()).unwrap();
            assert!(!same(&shape, &wolfrom), "{add:?} changed nothing");
            edit(&mut shape, Edit::Remove(Piece::Member(added))).unwrap();
            assert!(same(&shape, &wolfrom), "{add:?} undone");
        }
        let idler = Preset::Idler.build();
        let mut shape = idler.clone();
        let at_end = Edit::AddGear {
            mate: 2,
            on: Place::NewAxis,
            ring: false,
        };
        edit(&mut shape, at_end).unwrap();
        edit(&mut shape, Edit::Remove(Piece::Axis(3))).unwrap();
        assert!(same(&shape, &idler), "an axis added and removed");
        let layshaft = Preset::Layshaft.build();
        let mut shape = layshaft.clone();
        let ratio = Edit::AddRatio {
            distance: 0,
            shared: shared(&layshaft, 0),
        };
        edit(&mut shape, ratio).unwrap();
        let last = shape.meshes.len() - 1;
        edit(&mut shape, Edit::Remove(Piece::Mesh(last))).unwrap();
        assert!(same(&shape, &layshaft), "a ratio added and removed");
        // A coupling added to a hula's wobble body and taken away, the
        // shaft it turned with it.
        let hula = arr::hula([19, 18, 17, 18], [1.0, 1.0]);
        let mut shape = hula.clone();
        edit(&mut shape, Edit::Couple { body: 4 }).unwrap();
        assert!(!same(&shape, &hula), "a coupling changed nothing");
        edit(&mut shape, Edit::Remove(Piece::Coupling(0))).unwrap();
        assert!(same(&shape, &hula), "a coupling added and removed");
    }

    /// **A refused edit changes nothing**, and refuses for the reason named:
    /// the last central on a step, the last gear on a planet axis, a
    /// chain's two axes, a distance's one mesh, an edit of the other family.
    #[test]
    fn a_refused_edit_changes_nothing() {
        // Planocentric: planet 0 and ring 1; bodies carrier 1, ring 2, the
        // coupled shaft 3, the planet's 4. Planetary: sun 0, planet 1,
        // ring 2; bodies sun 1, carrier 2, ring 3, planet 4. Meshed
        // planets: sun 0, planets A 1 and B 2, ring 3.
        let plano = Preset::Planocentric.build();
        let on_centre = central(&plano, 1);
        let cases: Vec<(Shape, Edit, EditRefused)> = vec![
            (
                plano.clone(),
                Edit::Couple { body: 4 },
                EditRefused::Coupled,
            ),
            (
                Preset::Spur.build(),
                Edit::Couple { body: 1 },
                EditRefused::NotCarried,
            ),
            (
                plano.clone(),
                Edit::Remove(Piece::Coupling(1)),
                EditRefused::NoSuchIndex,
            ),
            // The ring is the last central member planet B meets, and planet
            // A would hold it in mesh at no radius. (A planocentric's ring
            // taken takes its planet with it, meshing nothing, and leaves
            // nothing hanging.)
            (
                Preset::MeshedPlanets.build(),
                Edit::Remove(Piece::Member(3)),
                EditRefused::LastOnItsStep,
            ),
            (
                Preset::Spur.build(),
                Edit::AddStep { axis: 0 },
                EditRefused::NotCarried,
            ),
            // The axis a carrier turns about is no axis to take away.
            (
                Preset::Planetary.build(),
                Edit::Remove(Piece::Axis(0)),
                EditRefused::CentralAxis,
            ),
            (
                Preset::Planetary.build(),
                Edit::Move {
                    member: 0,
                    to: Some(4),
                },
                EditRefused::NotOnTheAxis,
            ),
            (
                Preset::Planetary.build(),
                Edit::Remove(Piece::Member(9)),
                EditRefused::NoSuchIndex,
            ),
            (
                plano,
                Edit::AddGear {
                    mate: 0,
                    on: on_centre,
                    ring: false,
                },
                EditRefused::NoRoom,
            ),
            // Two rings in mesh are no mesh.
            (
                Preset::Planetary.build(),
                Edit::AddGear {
                    mate: 2,
                    on: Place::NewAxis,
                    ring: true,
                },
                EditRefused::RingToRing,
            ),
            // A gear fixed to the carrier of the planet it meshes locks it.
            (
                Preset::Planetary.build(),
                Edit::AddGear {
                    mate: 1,
                    on: Place::Body(2),
                    ring: false,
                },
                EditRefused::CarriesAnAxis,
            ),
            // A gear meshes across an axis distance, and an idler's first
            // and last axes have none.
            (
                Preset::Idler.build(),
                Edit::AddGear {
                    mate: 0,
                    on: Place::NewBody(2),
                    ring: false,
                },
                EditRefused::NoDistance,
            ),
        ];
        for (before, what, why) in cases {
            let mut shape = before.clone();
            assert_eq!(edit(&mut shape, what.clone()), Err(why), "{what:?}");
            assert!(same(&shape, &before), "{what:?} touched the shape");
        }
    }

    /// **A refusal says why** (audit T13.8): each edit of another family is
    /// refused by the key that names its cause, the shape unchanged — a
    /// coupling or a step off a carried axis, the axis a carrier turns about
    /// taken away, two rings in mesh, a gear on a new fixed axis at a mate
    /// that meshes in a carrier's frame (a planet, and a sun meshing
    /// planets), and a ring across crossed shafts.
    #[test]
    fn a_refusal_says_why() {
        let cases: Vec<(Preset, Edit, &str)> = vec![
            (
                Preset::Spur,
                Edit::Couple { body: 1 },
                "ui.train_edit_refused_not_carried",
            ),
            (
                Preset::Spur,
                Edit::AddStep { axis: 0 },
                "ui.train_edit_refused_not_carried",
            ),
            (
                Preset::Planetary,
                Edit::Remove(Piece::Axis(0)),
                "ui.train_edit_refused_central_axis",
            ),
            (
                Preset::Planetary,
                Edit::AddGear {
                    mate: 2,
                    on: Place::NewAxis,
                    ring: true,
                },
                "ui.train_edit_refused_ring_to_ring",
            ),
            (
                Preset::Planetary,
                Edit::AddGear {
                    mate: 1,
                    on: Place::NewAxis,
                    ring: false,
                },
                "ui.train_edit_refused_orbiting_mate",
            ),
            (
                Preset::Planetary,
                Edit::AddGear {
                    mate: 0,
                    on: Place::NewAxis,
                    ring: false,
                },
                "ui.train_edit_refused_orbiting_mate",
            ),
            (
                Preset::Worm,
                Edit::AddGear {
                    mate: 0,
                    on: Place::Body(2),
                    ring: true,
                },
                "ui.train_edit_refused_ring_crossed",
            ),
        ];
        for (p, what, key) in cases {
            let before = p.build();
            let mut shape = before.clone();
            let got = edit(&mut shape, what.clone()).map_err(EditRefused::key);
            assert_eq!(got, Err(key), "{p:?} {what:?}");
            assert!(same(&shape, &before), "{what:?} touched the shape");
        }
    }

    /// **A hold and a release are one rule both ways** (audit T13.8): each
    /// refuses ground and a body the train does not list by `NoSuchIndex`,
    /// the train unchanged; and each asked of a body already so — a hold
    /// of a held body, a release of a free one — changes nothing and is no
    /// refusal.
    #[test]
    fn a_hold_and_a_release_are_one_rule_both_ways() {
        let t = super::super::testing::cased(vec![Preset::Planetary.build()]);
        // Sun 1, carrier 2, ring 3 held by convention, planet 4.
        assert_eq!(t.held, vec![3]);
        for body in [GROUND, 99] {
            for what in [Edit::Hold(body), Edit::Release(body)] {
                let mut u = t.clone();
                assert_eq!(
                    u.edit(what.clone()),
                    Err(EditRefused::NoSuchIndex),
                    "{what:?}"
                );
                assert_eq!(debug(&u), debug(&t), "{what:?} refused whole");
            }
        }
        for what in [Edit::Hold(3), Edit::Release(4)] {
            let mut u = t.clone();
            assert_eq!(u.edit(what.clone()), Ok(()), "{what:?}");
            assert_eq!(debug(&u), debug(&t), "{what:?} changed nothing");
        }
    }

    /// **A duty is switched on a case the train has** (audit T13.8): a case
    /// past the list is refused by `NoSuchIndex`, the train unchanged,
    /// where it did nothing and said so to no one.
    #[test]
    fn a_duty_is_switched_on_a_case_the_train_has() {
        let t = super::super::testing::cased(vec![Preset::Spur.build()]);
        let mut u = t.clone();
        let past = t.load_cases.len();
        assert_eq!(u.set_duty(past, true), Err(EditRefused::NoSuchIndex));
        assert_eq!(debug(&u), debug(&t), "refused whole");
        assert_eq!(u.set_duty(past - 1, false), Ok(()));
        assert_ne!(debug(&u), debug(&t), "a case it has is switched");
    }

    /// **A release gives up a body only its hold named** (audit T13.12), as
    /// every other edit gives up a bare body nothing names: a set's held
    /// ring whose gear moved off it stays while held, and released it goes,
    /// the numbers closing up — where it stayed listed, named by nothing.
    #[test]
    fn a_release_gives_up_a_body_only_its_hold_named() {
        let mut t =
            super::super::testing::cased(vec![Preset::Spur.build(), Preset::MeshedPlanets.build()]);
        let ring = t.port(1, 3);
        assert!(t.held.contains(&ring), "the set's ring is held");
        let gear = t.member(1, 3).expect("the set's ring gear");
        assert_eq!(t.shape.members[gear].body, ring);
        let sun = t.port(1, 1);
        t.edit(Edit::Move {
            member: gear,
            to: Some(sun),
        })
        .unwrap();
        assert!(
            t.shape.is_bare(ring) && t.held.contains(&ring),
            "bare, held"
        );
        let before = t.shape.bodies.len();
        t.edit(Edit::Release(ring)).unwrap();
        t.check().unwrap();
        assert_eq!(t.shape.bodies.len(), before - 1, "{:?}", t.shape.bodies);
    }

    /// **A move keeps what a body was told**: a gear alone on its body
    /// moved to a new one changes nothing; one that shares its body — the
    /// pair's second gear on the sun's shaft — takes a body of its own, the
    /// shaft staying the set's with the case at it; and a gear cannot be
    /// moved onto the body that carries the planets it meshes.
    #[test]
    fn a_move_to_a_body_of_its_own_is_no_move_and_the_carrier_takes_no_gear() {
        let mut t = Train::chained(vec![Preset::Spur.build(), Preset::Planetary.build()], |t| {
            vec![LoadCase::ultimate(t.port(0, 1), t.port(1, 2), 1.0, 1000.0)]
        });
        let before = t.clone();
        t.edit(Edit::Move {
            member: 0,
            to: None,
        })
        .unwrap();
        assert_eq!(debug(&t), debug(&before), "alone on its body: no move");
        let shared = t.port(0, 2);
        assert_eq!(t.ends_of(shared).len(), 2, "the chain joined gear 2 onward");
        t.edit(Edit::Move {
            member: 1,
            to: None,
        })
        .unwrap();
        assert_eq!(t.ends_of(shared).len(), 1, "the shaft is the set's");
        assert_eq!(t.load_cases, before.load_cases);
        // The set's carrier; its sun may not go there.
        let (carrier, sun) = (before.port(1, 2), before.parts()[1].members[0]);
        assert_eq!(
            t.edit(Edit::Move {
                member: sun,
                to: Some(carrier),
            }),
            Err(EditRefused::CarriesAnAxis)
        );
    }

    /// **A gear moved off a shaft does not take the shaft with it.** A body
    /// is a port the train may hold, share or load, and it stays while
    /// anything names it — another part's gear on it, a hold, a case — even
    /// with nothing of this part's on it, which is what a gearbox in neutral
    /// is: its part has no end of the shaft until a gear is engaged on it
    /// again. Dropping it took the shaft and the case with it: a layshaft's
    /// output moved to an idler left the next stage joined to nothing.
    ///
    /// Engaging another ratio is the two moves it is on the machine, in the
    /// order that locks nothing: this gear off to a body of its own —
    /// neutral — then the other onto the output. The other order passes
    /// through two ratios on one pair of shafts, which locks every shaft of
    /// the layshaft, and is refused (`Locks`), as is the move of the
    /// engaged gear onto the idler's body. It is given up where nothing
    /// names it, so a stage asked about alone keeps no numbers it has no
    /// use for.
    #[test]
    fn a_gear_moved_off_a_shaft_does_not_take_the_shaft_with_it() {
        let lay = || arr::layshaft((17, 43), &[(41, 19), (29, 31)], 1);
        let mut t = Train::chained(vec![lay(), Preset::Spur.build()], |t| {
            vec![LoadCase::ultimate(t.port(0, 1), t.port(1, 2), 1.0, 1000.0)]
        });
        // The layshaft's output (slot 2) runs on to the spur; the engaged
        // pair's gear is the one sitting on it, and the other pair's idles
        // on a body of its own.
        let output = t.port(0, 2);
        let input = t.port(0, 1);
        assert_eq!(t.ends_of(output).len(), 2, "the output runs on");
        let shape = t.part_shapes()[0].clone();
        let axis = shape.axis_of_slot(shape.slot(output));
        let engaged = shape
            .members_on_body(output)
            .first()
            .copied()
            .expect("a gear is engaged");
        let (idle, idler) = (0..shape.members.len())
            .filter_map(|i| {
                let on = shape.members[i].body;
                (on != output && on != input && shape.axis_of_slot(shape.slot(on)) == axis)
                    .then_some((i, on))
            })
            .next()
            .expect("the other pair's gear idles on a body of its own");

        // **Two ratios on one pair of shafts lock them**: the engaged gear
        // onto the idler's body, and the idle gear onto the output while
        // the engaged one is still there, each refused whole.
        for locking in [
            Edit::Move {
                member: engaged,
                to: Some(idler),
            },
            Edit::Move {
                member: idle,
                to: Some(output),
            },
        ] {
            let before = t.clone();
            assert_eq!(
                t.edit(locking.clone()),
                Err(EditRefused::Locks),
                "{locking:?}"
            );
            assert_eq!(debug(&t), debug(&before), "{locking:?} refused whole");
        }

        // **Neutral**: the engaged gear off to a body of its own. The
        // output keeps its number and the spur's gear on it, and nothing
        // of the layshaft's is on it.
        t.edit(Edit::Move {
            member: engaged,
            to: None,
        })
        .unwrap();
        assert!(
            t.shape.bodies.iter().any(|b| b.body == output),
            "the output stays"
        );
        assert_eq!(
            t.ends_of(output)
                .iter()
                .map(|&(k, _)| k)
                .collect::<Vec<_>>(),
            vec![1],
            "the spur's, and nothing is engaged: neutral"
        );

        // ...and the other ratio engaged: the idle gear onto the output,
        // the body it idled on given up with nothing to name it.
        t.edit(Edit::Move {
            member: idle,
            to: Some(output),
        })
        .unwrap();
        assert_eq!(t.part_shapes()[0].members_on_body(output), vec![idle]);
        assert_eq!(t.ends_of(output).len(), 2, "the output is the output");
        assert_eq!(t.port(0, 2), output, "...where it was");

        // **A bare body nothing names is given up**: the same first move on
        // a stage of its own, with no train to mean the shaft to be there —
        // one body given up for the one the gear moves to.
        let mut alone = Train::chained(vec![lay()], |_| Vec::new());
        let was = alone.shape.bodies.len();
        let on = alone.shape.members_on_body(alone.port(0, 2))[0];
        alone
            .edit(Edit::Move {
                member: on,
                to: None,
            })
            .unwrap();
        alone.check().unwrap();
        assert_eq!(alone.shape.bodies.len(), was, "one given up, one added");
        assert!(
            alone
                .shape
                .bodies
                .iter()
                .all(|b| !alone.shape.is_bare(b.body)),
            "the shaft nothing named is given up: {:?}",
            alone.shape.bodies
        );
    }

    /// **What a remove takes off a stage, the train follows**: a Wolfrom's
    /// first ring removed takes its body (2) out of the train, the hold
    /// written there with it, and the bodies after it close up — the second
    /// ring from 3 to 2, still running on to the spur, the case entry at
    /// the crank still at 1.
    #[test]
    fn a_remove_repoints_the_train_and_drops_what_named_the_body() {
        let mut t = Train::chained(vec![Preset::Wolfrom.build(), Preset::Spur.build()], |t| {
            vec![LoadCase::ultimate(t.port(0, 1), t.port(1, 2), 1.0, 1000.0)]
        });
        // The chain joins ring 2 (slot 3) onward; hold ring 1 (slot 2,
        // member 1 after the planet) explicitly too.
        assert_eq!(t.port(0, 3), 3);
        assert_eq!(t.ends_of(3).len(), 2);
        t.hold(t.port(0, 2));
        t.edit(Edit::Remove(Piece::Member(1))).unwrap();
        assert_eq!(t.part_shapes()[0].members.len(), 2);
        assert_eq!(t.port(0, 2), 2, "ring 2 closed up to body 2");
        assert_eq!(
            t.ends_of(2).len(),
            2,
            "ring 2 still runs on to the spur: {:?}",
            t.part_shapes()[1].bodies
        );
        assert!(
            t.held.is_empty(),
            "the hold on the removed body is gone: {:?}",
            t.held
        );
        assert_eq!(t.load_cases[0].loads[0].at, 1, "the crank stayed");
        assert_eq!(
            t.max_body(),
            4,
            "the numbers are dense again: three of the Wolfrom's, one more of the spur's"
        );
        // ...and an add repoints nothing.
        let before = t.clone();
        t.edit(Edit::AddGear {
            mate: 0,
            on: central(&t.shape, 1),
            ring: false,
        })
        .unwrap();
        assert_eq!(t.load_cases[0].loads, before.load_cases[0].loads);
        assert_eq!(t.held, before.held);
        for b in 1..=before.max_body() {
            assert_eq!(t.ends_of(b), before.ends_of(b), "body {b} moved");
        }
    }

    /// **An axis removed goes, with everything on it** (audit T13.12,
    /// orchestrator's decision (d)): a shaft a coupling turns and the
    /// coupling with it — the axis it stands on is what was asked to go, and
    /// a removal that took the gears and left the axis made a second removal
    /// of it an Ok that changed nothing. Where a case reacts at that shaft,
    /// the removal is refused whole under `Loaded` (plan decision 6). An
    /// uncoupled planocentric with a pair after it, joined to the planet by
    /// a coupling: the pair's input axis removed.
    #[test]
    fn removing_an_axis_takes_the_shaft_a_coupling_turns() {
        let uncoupled = arr::epicyclic(
            1,
            &[&[arr::external(30)]],
            &[
                arr::Central::Carrier,
                arr::Central::Ring { on: 0, teeth: 33 },
            ],
            &[],
        );
        let t = Train::chained(vec![uncoupled, Preset::Spur.build()], |_| Vec::new());
        let (planet, end) = (3, t.port(1, 1));
        assert_eq!(t.shape.couplings, vec![[planet, end]]);
        let axis = t.shape.bodies.iter().find(|b| b.body == end).unwrap().axis;
        let removal = Edit::Remove(Piece::Axis(axis));
        let mut u = t.clone();
        u.edit(removal.clone()).unwrap();
        u.check().unwrap();
        assert!(u.shape.couplings.is_empty(), "{:?}", u.shape.couplings);
        // The axis goes, with the pair's other axis, whose gear the removal
        // left meshing nothing.
        assert_eq!(
            (u.shape.axes.len(), u.shape.bodies.len()),
            (t.shape.axes.len() - 2, t.shape.bodies.len() - 2),
            "the axes and the shafts go: {:?}",
            u.shape.bodies
        );
        // Asked again, it names another axis or none: never an Ok that
        // changes nothing.
        let mut v = u.clone();
        let again = v.edit(removal.clone());
        assert!(
            again.is_err() || debug(&v) != debug(&u),
            "a second removal: {again:?}"
        );
        // Cased at the shaft, refused whole.
        let mut cased = t.clone();
        cased.load_cases = vec![LoadCase::ultimate(1, end, 1.0, 1000.0)];
        let mut w = cased.clone();
        assert_eq!(w.edit(removal), Err(EditRefused::Loaded));
        assert_eq!(debug(&w), debug(&cased), "refused whole");
    }

    /// **An axis removed goes, held or not, or the removal is refused by
    /// name**: on every train the walk visits and the grid's, every axis
    /// removed, with every coupled body held and without: made, the train
    /// has fewer axes than it had, so the axis asked is gone — never an Ok
    /// that leaves it standing because a shaft on it was coupled or held.
    #[test]
    fn an_axis_removed_goes_held_or_not() {
        let trains = super::super::sweep::visited()
            .into_iter()
            .map(|(name, steps, t)| (format!("{name}: {steps:?}"), t))
            .chain(super::super::sweep::trains());
        let (mut made, mut held_made) = (0, 0);
        for (name, t) in trains {
            let coupled: Vec<usize> = t.shape.couplings.iter().flatten().copied().collect();
            for axis in 0..t.shape.axes.len() {
                for held in [false, true] {
                    let mut u = t.clone();
                    if held {
                        if coupled.is_empty() {
                            continue;
                        }
                        for &b in &coupled {
                            if !u.held.contains(&b) {
                                u.held.push(b);
                            }
                        }
                    }
                    let before = u.shape.axes.len();
                    if u.edit(Edit::Remove(Piece::Axis(axis))).is_ok() {
                        assert!(
                            u.shape.axes.len() < before,
                            "{name}: held {held}, axis {axis} left standing"
                        );
                        made += 1;
                        held_made += usize::from(held);
                    }
                }
            }
        }
        assert!(
            made > 100 && held_made > 10,
            "{made} made, {held_made} with holds"
        );
    }

    /// **One rule for a bare body** (audit T13.12), where the train's own
    /// sweep asked otherwise: a planet body whose gears have all moved off
    /// it stayed, coupled — a body no mesh turns, holding the shaft it
    /// coupled to. An orbiting body with nothing on it is bare whatever it
    /// is coupled to ([`Shape::is_bare`]): it goes with its coupling, and
    /// the shaft with it where nothing names it; where a case reacts at the
    /// shaft, the shaft stays in neutral with the case at it. A
    /// planocentric with a step on its planet, both planet gears moved onto
    /// a new body of the planet's axis.
    #[test]
    fn a_planet_body_left_bare_goes_with_its_coupling() {
        let mut t = Train::chained(vec![Preset::Planocentric.build()], |_| Vec::new());
        t.edit(Edit::AddStep { axis: 1 }).unwrap();
        let (shaft, planet_body) = (3, 4);
        assert_eq!(t.shape.couplings, vec![[shaft, planet_body]]);
        let on_planet = t.shape.members_on_body(planet_body);
        assert_eq!(on_planet.len(), 2, "a stepped planet");
        t.edit(Edit::Move {
            member: on_planet[0],
            to: None,
        })
        .unwrap();
        let fresh = t.shape.members[on_planet[0]].body;
        let last = Edit::Move {
            member: on_planet[1],
            to: Some(fresh),
        };
        let mut u = t.clone();
        u.edit(last.clone()).unwrap();
        u.check().unwrap();
        assert!(u.shape.couplings.is_empty(), "{:?}", u.shape.couplings);
        assert_eq!(
            u.shape.bodies.len(),
            t.shape.bodies.len() - 2,
            "the planet body and the shaft it turned, named by nothing, go: {:?}",
            u.shape.bodies
        );
        // Cased, the shaft is the output a case reacts at: it stays, in
        // neutral — a port nothing drives, as a layshaft's output is between
        // its two ratios — and the planet body goes with its coupling.
        let mut v = t.clone();
        v.load_cases = vec![LoadCase::ultimate(1, shaft, 1.0, 1000.0)];
        let before = v.shape.bodies.len();
        v.edit(last).unwrap();
        v.check().unwrap();
        assert!(v.shape.couplings.is_empty(), "{:?}", v.shape.couplings);
        assert_eq!(v.shape.bodies.len(), before - 1, "{:?}", v.shape.bodies);
        assert!(
            v.shape.bodies.iter().any(|b| b.body == shaft),
            "the output stays"
        );
        assert_eq!(v.load_cases[0].loads[1].at, shaft, "and the case at it");
    }

    /// **The hula is reached from the Wolfrom preset by edits**:
    /// a step added, the first planet's second ring removed, the count set
    /// to one and the teeth written — the same motion `arrangements::hula`
    /// lists, ratio for ratio, on the counts the tables print.
    #[test]
    fn the_hula_is_reached_from_the_wolfrom_by_edits() {
        let mut shape = Preset::Wolfrom.build();
        // Members: planet, ring 1, ring 2. A step: planet 2 and a ring on it.
        edit(&mut shape, Edit::AddStep { axis: 1 }).unwrap();
        assert_eq!(shape.members.len(), 5);
        // Ring 2 off the first planet: the first step keeps ring 1.
        edit(&mut shape, Edit::Remove(Piece::Member(2))).unwrap();
        // Now: planet 1, ring 1 (grounded), planet 2, ring on planet 2 —
        // the hula's 18, 19, 17, 18.
        shape.axes[1].count = 1;
        for (m, z) in shape.members.iter_mut().zip([18, 19, 17, 18]) {
            m.gear.teeth = z;
        }
        // Under the hula's arrangement on each: crank driven, grounded ring
        // held, the output ring out — slots 1, 2 and 4 here, where the ring
        // removed gave its place up, and 1, 2 and 3 on the list.
        let edited = asked(&shape, &[2], 1, 4);
        let listed = asked(&arr::hula([19, 18, 17, 18], [1.0, 1.0]), &[2], 1, 3);
        assert!(
            (edited.ratio.unwrap() - listed.ratio.unwrap()).abs() < 1e-9,
            "{:?} against the list's {:?}",
            edited.ratio,
            listed.ratio
        );
    }

    /// **The hula is reached from the planocentric by edits**,
    /// and the planocentric from the hula: the coupling is the stage's to
    /// lose. A step on the planet — a gear and a ring on it — and the
    /// coupling taken away is a hula whose second ring is the output; a
    /// coupling on a hula's wobble body and its second step taken away is
    /// a planocentric whose coupled shaft is. Each is the list it reaches,
    /// ratio for ratio, on the counts the tables print.
    #[test]
    fn the_planocentric_and_the_hula_are_one_edit_apart() {
        let mut shape = Preset::Planocentric.build();
        // Bodies: carrier, ring, the coupled shaft, the planet's body.
        edit(&mut shape, Edit::AddStep { axis: 1 }).unwrap();
        edit(&mut shape, Edit::Remove(Piece::Coupling(0))).unwrap();
        assert!(shape.couplings.is_empty());
        assert_eq!(
            shape.bodies.iter().map(|b| b.body).collect::<Vec<_>>(),
            [1, 2, 4, 5],
            "the coupled shaft goes with its coupling"
        );
        // Members: planet 1, ring 1 (grounded), planet 2, the ring on it.
        for (m, z) in shape.members.iter_mut().zip([18, 19, 17, 18]) {
            m.gear.teeth = z;
        }
        let edited = asked(&shape, &[2], 1, 4);
        let listed = asked(&arr::hula([19, 18, 17, 18], [1.0, 1.0]), &[2], 1, 3);
        assert!(
            (edited.ratio.unwrap() - listed.ratio.unwrap()).abs() < 1e-9,
            "{:?} against the list's {:?}",
            edited.ratio,
            listed.ratio
        );

        // ...and back: the hula's wobble body coupled to a shaft on the
        // centre line, its second step and the output ring on it taken away.
        let mut shape = arr::hula([19, 18, 17, 18], [1.0, 1.0]);
        edit(&mut shape, Edit::Couple { body: 4 }).unwrap();
        edit(&mut shape, Edit::Remove(Piece::Member(1))).unwrap();
        assert_eq!(shape.couplings, vec![[5, 4]]);
        // Bodies: carrier, the grounded ring, the wobble body, the shaft.
        let edited = asked(&shape, &[2], 1, 4);
        let listed = asked(&arr::planocentric(18, 19), &[2], 1, 3);
        assert!(
            (edited.ratio.unwrap() - listed.ratio.unwrap()).abs() < 1e-9,
            "{:?} against the list's {:?}",
            edited.ratio,
            listed.ratio
        );
        assert!(
            (listed.ratio.unwrap() + 18.0).abs() < 1e-9,
            "−z_p / (z_r − z_p)"
        );
    }

    /// **A second sun at one carrier radius closes by its shift as a
    /// second ring does**: two suns on one planet, no ring — the Wolfrom
    /// with its sign flipped — reached from the Wolfrom preset by two suns
    /// added and both rings removed, and Willis gives `z_s2 / (z_s2 − z_s1)`
    /// with the first sun held and the carrier driving, every distance
    /// closed.
    #[test]
    fn a_planet_between_two_suns_is_a_wolfrom_with_the_sign_flipped() {
        let mut shape = Preset::Wolfrom.build();
        let (planet, on) = (0, central(&shape, 1));
        for _ in 0..2 {
            let sun = Edit::AddGear {
                mate: planet,
                on,
                ring: false,
            };
            edit(&mut shape, sun).unwrap();
        }
        // Rings 1 and 2 are members 1 and 2; what is left is the planet
        // and the two suns, 24 and 23 teeth at the Wolfrom's radius.
        edit(&mut shape, Edit::Remove(Piece::Member(2))).unwrap();
        edit(&mut shape, Edit::Remove(Piece::Member(1))).unwrap();
        let (s1, s2) = (
            f64::from(shape.members[1].gear.teeth),
            f64::from(shape.members[2].gear.teeth),
        );
        assert!(s1 != s2, "two suns of one count would turn as one");
        // Slots: carrier 1, planet 2, sun 1 at 3, sun 2 at 4.
        let r = asked(&shape, &[3], 1, 4);
        let want = s2 / (s2 - s1);
        assert!(
            (r.ratio.unwrap() - want).abs() < 1e-9,
            "{} against Willis's {want}",
            r.ratio.unwrap()
        );
        for d in &r.distances {
            for nominal in &d.nominal {
                assert!(
                    ((d.running - nominal).abs() - d.clearance.abs()).abs() < 1e-9,
                    "a sun's mesh not closed: {nominal} at {}",
                    d.running
                );
            }
        }
    }

    /// **A join that cannot be coaxial undoes, and one across an axis
    /// distance is refused.** The pair after an uncoupled planocentric is
    /// joined to its planet by a coupling; splitting the pair's end off
    /// takes the coupling away, and joining it again brings it back. Two ends on shafts an axis distance apart are no one body —
    /// a shaft is straight — and the train is left as it was.
    #[test]
    fn a_coupled_join_undoes_and_one_across_a_distance_is_refused() {
        let uncoupled = arr::epicyclic(
            1,
            &[&[arr::external(30)]],
            &[
                arr::Central::Carrier,
                arr::Central::Ring { on: 0, teeth: 33 },
            ],
            &[],
        );
        let mut t = Train::chained(vec![uncoupled, Preset::Spur.build()], |_| Vec::new());
        let (planet, end) = (3, t.port(1, 1));
        assert_eq!(t.shape.couplings, vec![[planet, end]]);
        let end = t.split(1, end);
        assert!(t.shape.couplings.is_empty(), "the end is its own");
        assert_eq!(t.parts().len(), 2);
        t.join(end, planet).unwrap();
        assert_eq!(t.shape.couplings, vec![[end, planet]], "joined again");
        // ...and power crosses it: the crank drives the pair's output.
        t.load_cases = vec![LoadCase::ultimate(1, t.port(1, 2), 1.0, 1000.0)];
        let r = solve_train(&t, &library()).unwrap();
        assert!(r.cases[0].solved, "{:?}", r.cases[0].notes);
        let e = r.paths[0].efficiency.forward;
        assert!(e > 0.5 && e < 1.0, "through the coupling: {e}");

        // Three pairs in a chain, each shared shaft split: the first pair's
        // output and the third's input are ends on the two shafts the
        // middle pair meshes across.
        let pair = || Preset::Spur.build();
        let mut t = Train::chained(vec![pair(), pair(), pair()], |_| Vec::new());
        let a = t.port(0, 2);
        t.split(1, a);
        let b = t.split(2, t.port(1, 2));
        assert_eq!((t.ends_of(a).len(), t.ends_of(b).len()), (1, 1));
        let before = t.clone();
        assert_eq!(t.edit(Edit::Join { a, b }), Err(EditRefused::Apart));
        assert_eq!(format!("{t:?}"), format!("{before:?}"), "refused");
    }

    // ------------------------------------------------ the graph's edits ---

    /// Every train of the grid, uncased: these laws are about the graph.
    fn trains() -> Vec<(String, Train)> {
        grid()
            .into_iter()
            .map(|e| (e.name.clone(), e.uncased()))
            .collect()
    }

    fn debug(t: &Train) -> String {
        format!("{t:?}")
    }

    /// **Every gear the graph admits is refused whole, or undoes.** On
    /// every train of the grid, a gear meshing every
    /// member, on a new axis, a new body of every axis and every body, as
    /// a ring and not: refused, the train is as it was; made, the graph
    /// has nothing hanging, the train solves or says why, and the gear
    /// taken off again leaves the train it was — where the body it went on
    /// keeps something without it, since a gear's body goes with it
    /// where nothing else is on it.
    #[test]
    fn every_gear_the_graph_admits_is_refused_whole_or_undoes() {
        let lib = library();
        let mut made = 0;
        let mut misfits: Vec<String> = Vec::new();
        for (name, t) in trains() {
            let s = &t.shape;
            let places: Vec<Place> = std::iter::once(Place::NewAxis)
                .chain((0..s.axes.len()).map(Place::NewBody))
                .chain(s.bodies.iter().map(|b| Place::Body(b.body)))
                .collect();
            for mate in 0..s.members.len() {
                for &on in &places {
                    for ring in [false, true] {
                        let edit = Edit::AddGear { mate, on, ring };
                        let mut u = t.clone();
                        let axis = s.axis_of_slot(s.slot_of_member(mate)).unwrap();
                        if on == Place::Body(s.axes[axis].carried_by) {
                            assert_eq!(
                                u.edit(edit.clone()),
                                Err(EditRefused::CarriesAnAxis),
                                "{name}: a gear on the carrier of the planet it meshes"
                            );
                        }
                        if u.edit(edit.clone()).is_err() {
                            assert_eq!(debug(&u), debug(&t), "{name}: {edit:?} refused");
                            continue;
                        }
                        made += 1;
                        u.check()
                            .unwrap_or_else(|e| panic!("{name}: {edit:?}: {e:?}"));
                        // It solves, or says why — a lock by construction, a
                        // distance two groups cannot share — as a train does.
                        let _ = solve_train(&u, &lib);
                        if let Some(error) = misfit(&u, mate) {
                            misfits.push(format!("{name}: {edit:?}: {error}"));
                        }
                        let keeps = match on {
                            Place::Body(b) => {
                                !s.members_on_body(b).is_empty()
                                    || s.carries_an_axis(b)
                                    || s.couplings.iter().any(|c| c.contains(&b))
                            }
                            Place::NewAxis | Place::NewBody(_) => true,
                        };
                        let new = u.shape.members.len() - 1;
                        u.edit(Edit::Remove(Piece::Member(new)))
                            .unwrap_or_else(|e| panic!("{name}: {edit:?} then its removal: {e}"));
                        if keeps {
                            assert_eq!(debug(&u), debug(&t), "{name}: {edit:?} then its removal");
                        }
                    }
                }
            }
        }
        assert!(made > 200, "only {made} gears made");
        assert!(
            misfits.is_empty(),
            "{} of {made}:\n{}",
            misfits.len(),
            misfits.join("\n")
        );
    }

    /// **Whether the last gear added is sized to the distance it meshes
    /// across** — on parallel axes, its mesh's reference span within half
    /// a transverse module of the distance's first mesh's, which is the
    /// rounding a count takes; `None` where it is, where it went on a new
    /// axis, crossed, or round a planet (sized to the carrier radius, and
    /// moved a tooth off a count that would turn as one).
    fn misfit(u: &Train, mate: usize) -> Option<String> {
        let s = &u.shape.indexed();
        let k = s.meshes.len() - 1;
        let d = s.distance_of(k)?;
        let first = s.meshes_on(d)[0];
        let axis = |i: usize| s.axis_of_slot(s.slot_of_member(i));
        let planet = axis(mate).is_some_and(|a| s.axes[a].carried_by != GROUND);
        if first == k || s.is_crossed(k) || planet {
            return None;
        }
        let (shared, helix) = (s.shared(), s.helix_angles());
        let transverse = |i: usize| shared.members[i].normal_module() / helix[i].to_radians().cos();
        let span = |k: usize| {
            let m = s.meshes[k];
            let z = |i: usize| {
                let z = f64::from(s.members[i].gear.teeth);
                if s.members[i].ring.is_some() {
                    -z
                } else {
                    z
                }
            };
            (z(m.a) + z(m.b)).abs() * transverse(m.a) / 2.0
        };
        let (want, got) = (span(first), span(k));
        ((got - want).abs() > transverse(mate) / 2.0 + 1e-9)
            .then(|| format!("spans {got} where the distance's first mesh spans {want}"))
    }

    /// **A removal takes what goes with it and leaves nothing hanging**:
    /// every member, mesh, axis, body and coupling of every train of the
    /// grid, taken out — refused, the train as it was;
    /// made, something gone and nothing left hanging, the train solving
    /// or saying why.
    #[test]
    fn a_removal_takes_what_goes_with_it_and_leaves_nothing_hanging() {
        let lib = library();
        let mut made = 0;
        for (name, t) in trains() {
            let s = &t.shape;
            let pieces: Vec<Piece> = (0..s.members.len())
                .map(Piece::Member)
                .chain((0..s.meshes.len()).map(Piece::Mesh))
                .chain((0..s.axes.len()).map(Piece::Axis))
                .chain(s.bodies.iter().map(|b| Piece::Body(b.body)))
                .chain((0..s.couplings.len()).map(Piece::Coupling))
                .collect();
            for piece in pieces {
                let mut u = t.clone();
                if u.edit(Edit::Remove(piece)).is_err() {
                    assert_eq!(debug(&u), debug(&t), "{name}: {piece:?} refused");
                    continue;
                }
                made += 1;
                assert_ne!(debug(&u), debug(&t), "{name}: {piece:?} took nothing");
                u.check()
                    .unwrap_or_else(|e| panic!("{name}: {piece:?}: {e:?}"));
                let _ = solve_train(&u, &lib);
            }
        }
        assert!(made > 100, "only {made} removals made");
    }

    /// **A join is one body on one axis, or says why.** Two pairs apart —
    /// four axes, two parts — the first's output joined to the second's
    /// input: one body, three axes, a chain of two at the product of their
    /// ratios. Two bodies of one part are refused (`Geared`), as are ground
    /// and a body the train has not; each refusal changes nothing.
    #[test]
    fn a_join_is_one_body_on_one_axis_or_says_why() {
        let pair = Preset::Spur.build();
        let mut second = pair.clone();
        second.renumber_bodies(|b| b + 2);
        let t = Train {
            load_cases: Vec::new(),
            reversed_bending: false,
            shape: super::super::graph::graph_of(&[pair, second], 1).shape,
            held: Vec::new(),
        };
        assert_eq!((t.parts().len(), t.shape.axes.len()), (2, 4));
        let mut u = t.clone();
        u.edit(Edit::Join { a: 2, b: 3 }).unwrap();
        u.check().unwrap();
        assert_eq!(
            (u.parts().len(), u.shape.axes.len(), u.shape.bodies.len()),
            (2, 3, 3)
        );
        assert_eq!(u.ends_of(2).len(), 2, "the shaft is both pairs'");
        let mut c = vec![crate::kinematics::Condition::Free; 4];
        c[0] = crate::kinematics::Condition::Ground;
        c[1] = crate::kinematics::Condition::Drive(crate::ratio::Ratio::ONE);
        let motion = u.system().unwrap().motion(&c).unwrap();
        assert_eq!(
            motion.values[1].checked_div(motion.values[3]),
            Some(crate::ratio::Ratio::new(43 * 43, 17 * 17).unwrap()),
            "a chain of two at the product"
        );
        for (a, b, why) in [
            (1, 2, EditRefused::Geared),
            (0, 1, EditRefused::NoSuchIndex),
            (1, 99, EditRefused::NoSuchIndex),
        ] {
            let mut v = t.clone();
            assert_eq!(v.edit(Edit::Join { a, b }), Err(why), "join {a} {b}");
            assert_eq!(debug(&v), debug(&t));
        }
    }

    /// **An insert at a body runs on its shaft**: a set laid in at a
    /// pair's input shares that shaft — its sun on the pair's first body
    /// — and at a body the train has not is refused whole.
    #[test]
    fn an_insert_at_a_body_runs_on_its_shaft() {
        let mut t = Train::chained(vec![Preset::Spur.build()], |_| Vec::new());
        t.edit(Edit::Insert {
            shape: Preset::Planetary.build(),
            at: Some(1),
        })
        .unwrap();
        t.check().unwrap();
        assert_eq!(t.parts().len(), 2);
        assert_eq!(t.ends_of(1).len(), 2, "the input shaft carries both");
        solve_train(&t, &library()).unwrap();
        let before = t.clone();
        assert_eq!(
            t.edit(Edit::Insert {
                shape: Preset::Spur.build(),
                at: Some(99),
            }),
            Err(EditRefused::NoSuchIndex)
        );
        assert_eq!(debug(&t), debug(&before));
    }

    /// **A reaction stays a reaction whatever it is joined to.** A pair
    /// loaded at its input and reacted at its output, a set laid in at that
    /// output: the shaft is both parts' now, and the case still reacts it —
    /// the path across the pair is still asked and still the pair's, and
    /// the set, its output free, carries nothing. Which parts a body lies
    /// between is the graph's to derive and never a case's to be told: a
    /// second reaction on the same line, at the set's output, divides the
    /// load by stiffness, and the case says so by name rather than the join
    /// taking a role away (a join once turned the reaction into a derived
    /// load, which dropped the pair's path).
    #[test]
    fn a_reaction_stays_a_reaction_whatever_it_is_joined_to() {
        let lib = library();
        let mut t = Train::chained(vec![Preset::Spur.build()], |t| {
            vec![LoadCase::ultimate(t.port(0, 1), t.port(0, 2), 1.0, 1000.0)]
        });
        let alone = solve_train(&t, &lib).unwrap();
        let (input, output) = (t.port(0, 1), t.port(0, 2));
        t.edit(Edit::Insert {
            shape: Preset::Planetary.build(),
            at: Some(output),
        })
        .unwrap();
        assert_eq!(t.ends_of(output).len(), 2, "the output is both parts'");
        assert!(
            t.load_cases[0]
                .loads
                .iter()
                .any(|l| l.at == output && l.role == LoadRole::Reacted),
            "the join keeps the reaction"
        );
        let r = solve_train(&t, &lib).unwrap();
        assert!(r.cases[0].solved);
        let (was, is) = (&alone.paths[0], &r.paths[0]);
        assert_eq!((is.from, is.to), (input, output));
        assert!((is.ratio - was.ratio).abs() < 1e-12);
        assert!((is.efficiency.forward - was.efficiency.forward).abs() < 1e-12);
        let far = t
            .open_ports()
            .iter()
            .map(|p| p.body)
            .find(|&b| b != input && b != output)
            .expect("the set's output");
        t.load_cases[0]
            .loads
            .push(super::super::Load::declared(far, LoadRole::Reacted));
        let r = solve_train(&t, &lib).unwrap();
        assert!(!r.cases[0].solved);
        assert!(r.cases[0]
            .notes
            .iter()
            .any(|n| n.is(crate::note::key::TRAIN_LOAD_SHARED)));
    }

    /// **A ratio goes on the body asked.** A layshaft's next ratio shares
    /// the body the edit names — the layshaft, or the input shaft — and a
    /// body on neither of the distance's axes, the output of a pair after
    /// it, is refused.
    #[test]
    fn a_ratio_goes_on_the_body_asked() {
        let t = Train::chained(vec![Preset::Layshaft.build(), Preset::Spur.build()], |_| {
            Vec::new()
        });
        let s = &t.shape;
        let across: Vec<usize> = s
            .bodies
            .iter()
            .filter(|b| s.distances[0].axes.contains(&b.axis))
            .map(|b| b.body)
            .collect();
        assert!(across.len() > 2, "a layshaft's shafts: {across:?}");
        for shared in across {
            let mut u = t.clone();
            u.edit(Edit::AddRatio {
                distance: 0,
                shared,
            })
            .unwrap();
            u.check()
                .unwrap_or_else(|e| panic!("sharing {shared}: {e:?}"));
            let n = u.shape.members.len();
            assert_eq!(u.shape.members[n - 2].body, shared, "on the body asked");
        }
        let elsewhere = s.bodies.iter().map(|b| b.body).find(|&b| {
            let axis = s.bodies.iter().find(|x| x.body == b).unwrap().axis;
            !s.distances[0].axes.contains(&axis)
        });
        let b = elsewhere.expect("the pair's output is off the layshaft's axes");
        let mut u = t.clone();
        assert_eq!(
            u.edit(Edit::AddRatio {
                distance: 0,
                shared: b
            }),
            Err(EditRefused::NotOnTheAxis)
        );
    }
}
