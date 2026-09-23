//! **The edits a stage takes on the card**, each a rule about what else has
//! to change — the mirror, one level down, of what `conditions.rs` does
//! for a train.
//!
//! A designer permutes an arrangement by adding and removing: a step on the
//! planet body, a sun or a ring on a planet gear, an axis at the end of a
//! parallel chain, one more mesh on a distance; and by moving a member to another
//! body on its axis. Nothing is *flipped*: a member's kind is decided by
//! the button that adds it and a ring stays a ring, since a sun and a ring
//! differ in more than a flag (a cutter, a shift rule) and a swap is a
//! remove and an add, the new member sized by the core to what it meets.
//!
//! Every add appends — a new body is numbered after every body the train
//! has, a new member is the last — so nothing a case or a hold names
//! moves. A remove takes a body off the stage; the train then drops any
//! body nothing names and renumbers the rest, repointing its cases and
//! holds ([`super::Train::edit_stage`]). The invariants the edits keep:
//! every member is in a mesh, every planet gear meets a central member,
//! every distance carries a mesh, and a carrier's body is never removed.

use super::shape::{Member, Shape};
use super::StageGear;
use crate::kinematics::GROUND;
use crate::params::Auto;

/// What a card asks of a stage. Indices are the shape's own: a member, a
/// mesh, a distance by position; a body as the wiring numbers it, ground
/// being 0 and the first listed body 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub enum StageEdit {
    /// **A step**: one more gear on the planet body of `axis`, a ring
    /// meshing it, both appended. The gear copies the last gear on that
    /// body; the ring is sized to the carrier radius the axis already runs
    /// at.
    AddStep { axis: usize },
    /// **A step removed**: the planet gear `gear` and every central member
    /// meshing it. Refused for the last gear on its axis.
    RemoveStep { gear: usize },
    /// **A sun or a ring on a planet gear**, on a new body of the central
    /// axis, sized to the carrier radius; a count that would repeat one
    /// already on that gear is moved by a tooth, since equal rings on one
    /// planet lock the stage.
    AddCentral { gear: usize, ring: bool },
    /// **A central member removed**, with its mesh and, where nothing else
    /// is on it, its body. Refused where it is the last member meeting its
    /// planet gear — remove the step instead — and for a planet gear, which
    /// is a step.
    RemoveMember { member: usize },
    /// **An axis at a gear**: a body on a new axis fixed in ground, a gear
    /// on it copying `mate`, a mesh with it, a distance between the two
    /// axes. Refused for a mate that does not mesh in ground — a planet, or
    /// a sun or a ring meshing planets — whose frame the new axis cannot
    /// share.
    AddAxis { mate: usize },
    /// **An axis removed**, with every gear on it, their meshes, its bodies
    /// and its distances. Refused for an axis a carrier turns or one that
    /// is carried, and where a gear left behind would mesh nothing.
    RemoveAxis { axis: usize },
    /// **One more mesh on a distance**, copying the first mesh there and
    /// bringing the two gears it needs: one on the body the distance's
    /// meshes share — a layshaft — or on the first mesh's second gear's
    /// body where none is shared yet, the other on a body of its own, an
    /// idler until it is the one moved onto the output.
    ///
    /// *A mesh, not a pair.* Every mesh is a pair of gears, so the word
    /// said nothing about what this adds: another **ratio across the same
    /// centres**, which is what makes a layshaft a gearbox rather than one
    /// reduction.
    AddMesh { distance: usize },
    /// **A mesh removed**: both its members, and their bodies where nothing
    /// else is on them. Refused for a distance's last mesh, and for a mesh
    /// a carried axis is on.
    RemoveMesh { mesh: usize },
    /// **A member moved to another body on its axis** — `None` a new one —
    /// and the body it leaves taken off the stage where it is emptied and
    /// carries no axis. A member alone on its body moved to a new one is
    /// already there, and nothing changes: the body, and what a case or a
    /// hold wrote on it, stays. Refused onto a body that carries an axis —
    /// a gear fixed to the carrier of the planets it meshes locks the
    /// stage.
    MoveBody { member: usize, body: Option<usize> },
    /// **An offset coupling** from `body`, on a carried axis, to a new body
    /// on its carrier's axis: the pins that take a cycloidal disc's turn off
    /// to the centre line, or an Oldham coupling. Refused for a body on an
    /// axis nothing carries, and for one coupled already.
    Couple { body: usize },
    /// **An offset coupling removed**, with each body it joined that is
    /// left with nothing on it — the shaft it turned, as a mesh removed
    /// takes its gears' emptied bodies.
    Uncouple { coupling: usize },
}

/// Why an edit is refused: the invariant it would break.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditRefused {
    /// No such member, mesh, axis, distance or body.
    NoSuchIndex,
    /// The last gear on a planet axis, or the last member meeting a planet
    /// gear: a step keeps at least one of each.
    LastOnItsStep,
    /// A parallel chain keeps two axes; a distance keeps one mesh.
    LastOfItsKind,
    /// The edit belongs to another family: a step on a stage with no
    /// carried axis, an axis on one with.
    WrongFamily,
    /// A body not on the member's axis.
    NotOnTheAxis,
    /// No member of that kind fits at the radius the axis runs at: a sun
    /// inside a planocentric, whose planet all but fills its ring.
    NoRoom,
    /// The body carries an axis: a gear on the carrier of the planets it
    /// meshes locks the stage.
    CarriesAnAxis,
    /// The body is coupled already.
    Coupled,
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
            Self::LastOfItsKind => "ui.train_edit_refused_last_of_kind",
            Self::WrongFamily => "ui.train_edit_refused_family",
            Self::NotOnTheAxis => "ui.train_edit_refused_axis",
            Self::NoRoom => "ui.train_edit_refused_no_room",
            Self::CarriesAnAxis => "ui.train_edit_refused_carrier",
            Self::Coupled => "ui.train_edit_refused_coupled",
        }
    }
}

impl std::fmt::Display for EditRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NoSuchIndex => "no such member, mesh, axis or body",
            Self::LastOnItsStep => "the last on its step",
            Self::LastOfItsKind => "the last of its kind",
            Self::WrongFamily => "not an edit of this family",
            Self::NotOnTheAxis => "not a body on the member's axis",
            Self::NoRoom => "nothing of that kind fits at this radius",
            Self::CarriesAnAxis => "that body carries an axis",
            Self::Coupled => "that body is coupled already",
        })
    }
}

impl Shape {
    /// Apply one edit, numbering any body it adds from `next` — the first
    /// number the train has free — upward.
    ///
    /// # Errors
    ///
    /// [`EditRefused`] where the edit would break an invariant, with the
    /// shape untouched.
    pub fn edit(&mut self, edit: StageEdit, next: usize) -> Result<(), EditRefused> {
        match edit {
            StageEdit::AddStep { axis } => self.add_step(axis, next),
            StageEdit::RemoveStep { gear } => self.remove_step(gear),
            StageEdit::AddCentral { gear, ring } => self.add_central(gear, ring, next),
            StageEdit::RemoveMember { member } => self.remove_member(member),
            StageEdit::AddAxis { mate } => self.add_axis(mate, next),
            StageEdit::RemoveAxis { axis } => self.remove_axis(axis),
            StageEdit::AddMesh { distance } => self.add_mesh_on(distance, next),
            StageEdit::RemoveMesh { mesh } => self.remove_mesh(mesh),
            StageEdit::MoveBody { member, body } => self.move_body(member, body, next),
            StageEdit::Couple { body } => self.couple(body, next),
            StageEdit::Uncouple { coupling } => self.uncouple(coupling),
        }
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
        (0..self.members.len())
            .filter(|&i| self.members[i].body == body)
            .collect()
    }

    fn meshes_of_member(&self, member: usize) -> Vec<usize> {
        (0..self.meshes.len())
            .filter(|&k| self.meshes[k].a == member || self.meshes[k].b == member)
            .collect()
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

    /// The axis a body of this stage sits on.
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

    // ---------------------------------------------------------- epicyclic ---

    fn add_step(&mut self, axis: usize, next: usize) -> Result<(), EditRefused> {
        if axis >= self.axes.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        if !self.carried(axis) {
            return Err(EditRefused::WrongFamily);
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
        self.add_central(new, true, next)
    }

    fn remove_step(&mut self, gear: usize) -> Result<(), EditRefused> {
        if gear >= self.members.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        if !self.is_planet_gear(gear) {
            return Err(EditRefused::WrongFamily);
        }
        if self.members_on_body(self.members[gear].body).len() < 2 {
            return Err(EditRefused::LastOnItsStep);
        }
        // Its central members go with it — a central that also meets
        // another gear only loses this mesh — then the gear. Dropped from
        // the highest index down, so each index is still the one read.
        let mut going: Vec<usize> = self
            .meshes_of_member(gear)
            .into_iter()
            .map(|k| self.mate(k, gear))
            .filter(|&c| !self.is_planet_gear(c) && self.meshes_of_member(c).len() == 1)
            .collect();
        going.push(gear);
        going.sort_unstable();
        going.dedup();
        for i in going.into_iter().rev() {
            self.drop_member(i);
        }
        Ok(())
    }

    fn add_central(&mut self, gear: usize, ring: bool, next: usize) -> Result<(), EditRefused> {
        if gear >= self.members.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        let planet_axis = self
            .axis_of_slot(self.slot_of_member(gear))
            .ok_or(EditRefused::NoSuchIndex)?;
        if !self.carried(planet_axis) {
            return Err(EditRefused::WrongFamily);
        }
        let central_axis = self
            .central_axis_of(planet_axis)
            .ok_or(EditRefused::NoSuchIndex)?;
        let (zp, module) = (
            self.members[gear].gear.teeth,
            self.members[gear].normal_module(),
        );
        // Sized to the radius the axis runs at; a few teeth of difference
        // where nothing sets it yet.
        let radius = self.carrier_radius(planet_axis);
        let fit = radius.map(|r| (2.0 * r / module).round());
        let mut teeth = match (ring, fit) {
            (true, Some(f)) => (f + f64::from(zp)).max(f64::from(zp) + 2.0),
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
        let floor = if ring { f64::from(zp) + 2.0 } else { 4.0 };
        let taken = |z: f64| {
            (0..self.members.len())
                .filter(|&p| self.axis_of_slot(self.slot_of_member(p)) == Some(planet_axis))
                .filter(|&p| self.members[p].gear.teeth == zp)
                .flat_map(|p| self.meshes_of_member(p).into_iter().map(move |k| (p, k)))
                .any(|(p, k)| {
                    let c = self.mate(k, p);
                    self.members[c].ring.is_some() == ring
                        && f64::from(self.members[c].gear.teeth) == z
                })
        };
        while taken(teeth) && teeth > floor {
            teeth -= 1.0;
        }
        while taken(teeth) {
            teeth += 1.0;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let teeth = teeth as u32;
        let body = self.push_body(central_axis, next);
        let cutter = ring.then(|| self.members.iter().find_map(|m| m.ring).unwrap_or_default());
        self.members.push(Member {
            body,
            // Its shift automatic: a second central at one carrier radius
            // is closed by its shift, which one given at zero could not do.
            gear: StageGear {
                teeth,
                profile_shift: Auto::automatic(0.0),
                ..self.members[gear].gear.clone()
            },
            // The gear it meets sets its module and pressure angle — they
            // mesh — so it follows them rather than stating its own.
            module: Auto::automatic(module),
            pressure_angle: Auto::automatic(self.members[gear].normal_pressure_angle()),
            thickness_mod: Auto::automatic(1.0),
            ring: cutter,
            pitch_diameter: Auto::automatic(0.0),
        });
        let new = self.members.len() - 1;
        self.push_mesh(new, gear);
        Ok(())
    }

    fn remove_member(&mut self, member: usize) -> Result<(), EditRefused> {
        if member >= self.members.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        if self.is_planet_gear(member) {
            return Err(EditRefused::WrongFamily);
        }
        // The planet gear it meets keeps at least one central member.
        for k in self.meshes_of_member(member) {
            let mate = self.mate(k, member);
            if self.is_planet_gear(mate)
                && self.meshes_of_member(mate).into_iter().all(|j| {
                    self.mate(j, mate) == member || self.is_planet_gear(self.mate(j, mate))
                })
            {
                return Err(EditRefused::LastOnItsStep);
            }
        }
        // On a parallel chain the same rule holds for a distance's last mesh.
        if !self.axes.iter().any(|a| a.carried_by != GROUND) {
            for k in self.meshes_of_member(member) {
                if let Some(d) = self.distance_of(k) {
                    if self.meshes_on(d).len() < 2 {
                        return Err(EditRefused::LastOfItsKind);
                    }
                }
            }
        }
        self.drop_member(member);
        Ok(())
    }

    // ----------------------------------------------------------- parallel ---

    fn add_axis(&mut self, mate: usize, next: usize) -> Result<(), EditRefused> {
        if mate >= self.members.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        let from = self
            .axis_of_slot(self.slot_of_member(mate))
            .ok_or(EditRefused::NoSuchIndex)?;
        if self.frame_of_member(mate) != GROUND {
            return Err(EditRefused::WrongFamily);
        }
        let axis = self.push_axis(GROUND, 1);
        let body = self.push_body(axis, next);
        self.members.push(Member {
            body,
            ring: None,
            ..self.members[mate].clone()
        });
        let new = self.members.len() - 1;
        self.push_mesh(mate, new);
        self.push_distance([from, axis], 0.0);
        Ok(())
    }

    fn remove_axis(&mut self, axis: usize) -> Result<(), EditRefused> {
        if axis >= self.axes.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        let turns_a_carrier = self
            .axes
            .iter()
            .any(|a| a.carried_by != GROUND && self.axis_of_body(a.carried_by) == Some(axis));
        if self.carried(axis) || turns_a_carrier {
            return Err(EditRefused::WrongFamily);
        }
        let on_axis: Vec<usize> = (0..self.members.len())
            .filter(|&i| self.axis_of_slot(self.slot_of_member(i)) == Some(axis))
            .collect();
        // **A gear left behind meshing nothing** is no mechanism: the
        // idler of a chain goes with the chain's end, not before it.
        let stranded = (0..self.members.len())
            .filter(|i| !on_axis.contains(i))
            .any(|i| {
                let meshes = self.meshes_of_member(i);
                !meshes.is_empty() && meshes.iter().all(|&k| on_axis.contains(&self.mate(k, i)))
            });
        if stranded {
            return Err(EditRefused::LastOfItsKind);
        }
        for &i in on_axis.iter().rev() {
            self.drop_member(i);
        }
        // Any body left on it, and its distances, and the axis — the axes
        // after it renumbered down.
        let gone: Vec<usize> = self
            .bodies
            .iter()
            .filter(|b| b.axis == axis)
            .map(|b| b.body)
            .collect();
        self.couplings
            .retain(|c| !c.iter().any(|b| gone.contains(b)));
        self.bodies.retain(|b| b.axis != axis);
        self.distances.retain(|d| !d.axes.contains(&axis));
        self.axes.remove(axis);
        for b in &mut self.bodies {
            if b.axis > axis {
                b.axis -= 1;
            }
        }
        for d in &mut self.distances {
            for a in &mut d.axes {
                if *a > axis {
                    *a -= 1;
                }
            }
        }
        Ok(())
    }

    fn add_mesh_on(&mut self, distance: usize, next: usize) -> Result<(), EditRefused> {
        if distance >= self.distances.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        let first = self
            .meshes_on(distance)
            .first()
            .copied()
            .ok_or(EditRefused::NoSuchIndex)?;
        let m = self.meshes[first];
        // The body the distance's pairs share: the one with the most
        // members among them, where that is more than one; else the first
        // pair's second gear's.
        let on_distance: Vec<usize> = self
            .meshes_on(distance)
            .into_iter()
            .flat_map(|k| [self.meshes[k].a, self.meshes[k].b])
            .collect();
        let count = |body: usize| {
            on_distance
                .iter()
                .filter(|&&i| self.members[i].body == body)
                .count()
        };
        let shared = on_distance
            .iter()
            .map(|&i| self.members[i].body)
            .max_by_key(|&b| count(b))
            .filter(|&b| count(b) > 1)
            .unwrap_or(self.members[m.b].body);
        let (on_shared, alone) = if self.members[m.a].body == shared {
            (m.a, m.b)
        } else {
            (m.b, m.a)
        };
        let axis = self
            .axis_of_body(self.members[alone].body)
            .ok_or(EditRefused::NoSuchIndex)?;
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

    fn remove_mesh(&mut self, mesh: usize) -> Result<(), EditRefused> {
        if mesh >= self.meshes.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        let d = self.distance_of(mesh).ok_or(EditRefused::NoSuchIndex)?;
        if self.meshes_on(d).len() < 2 {
            return Err(EditRefused::LastOfItsKind);
        }
        let m = self.meshes[mesh];
        if self.is_planet_gear(m.a) || self.is_planet_gear(m.b) {
            return Err(EditRefused::WrongFamily);
        }
        let (hi, lo) = (m.a.max(m.b), m.a.min(m.b));
        // A member in another mesh too stays: only the mesh goes from it.
        for i in [hi, lo] {
            if self.meshes_of_member(i).len() < 2 {
                self.drop_member(i);
            } else {
                self.meshes.retain(|x| !(x.a == m.a && x.b == m.b));
            }
        }
        Ok(())
    }

    // --------------------------------------------------------------- both ---

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
        // **The body it leaves stays.** A body a stage lists is a port the
        // train may hold, share or load, and dropping it because its gear
        // moved would take the coupling with it — which is how engaging a
        // layshaft's other ratio used to lose the output. A body with
        // nothing on it is a shaft with nothing driving it, which is what
        // *neutral* is, and the motion says so by being a family one
        // condition short. What no longer has a reason to exist is given
        // up a level up ([`super::Train::edit_stage`]), where what else
        // names a body can be seen.
        Ok(())
    }

    // --------------------------------------------------------- couplings ---

    fn couple(&mut self, body: usize, next: usize) -> Result<(), EditRefused> {
        let axis = self.axis_of_body(body).ok_or(EditRefused::NoSuchIndex)?;
        if !self.carried(axis) {
            return Err(EditRefused::WrongFamily);
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

    /// A body off the stage where nothing is on it, it carries no axis and
    /// no coupling turns it — with nothing left to say what it is.
    fn drop_if_bare(&mut self, body: usize) {
        if self.members_on_body(body).is_empty()
            && !self.carries_an_axis(body)
            && !self.couplings.iter().any(|c| c.contains(&body))
        {
            self.bodies.retain(|b| b.body != body);
        }
    }

    /// A member gone, with its meshes, and its body off the stage where it
    /// was alone on it and the body carries no axis.
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
        if self.members_on_body(body).is_empty() && !self.carries_an_axis(body) {
            self.bodies.retain(|b| b.body != body);
            self.couplings.retain(|c| !c.contains(&body));
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    //! **The laws an edit obeys**: it leaves a stage that solves, it undoes,
    //! it refuses whole, and what it renumbers the train follows.

    use super::super::arrangements::{self as arr, StagePreset};
    use super::super::{test_library as library, LoadCase, Shape, StageBoundary, Train};
    use super::*;

    fn conventionally(shape: &Shape) -> crate::train::Alone {
        under(
            shape,
            StageBoundary::conventional(&shape.wiring(), &shape.ports()),
        )
    }

    fn under(shape: &Shape, boundary: StageBoundary) -> crate::train::Alone {
        crate::train::solve_alone(
            &crate::train::Train::alone(shape, 2.0, 3000.0).under(&boundary),
            &library(),
        )
        .unwrap_or_else(|e| panic!("{e}: {shape:?}"))
    }

    fn same(a: &Shape, b: &Shape) -> bool {
        format!("{a:?}") == format!("{b:?}")
    }

    /// An edit on a shape outside any train: what it adds is numbered after
    /// what the shape has, as a train of this one stage would number it.
    fn edit(shape: &mut Shape, edit: StageEdit) -> Result<(), EditRefused> {
        let next = shape.max_body() + 1;
        shape.edit(edit, next)
    }

    /// The edits that apply to a shape, each on its first candidate.
    fn applicable(shape: &Shape) -> Vec<StageEdit> {
        let carried = (0..shape.axes.len()).find(|&a| shape.axes[a].carried_by != GROUND);
        let mut out = vec![
            StageEdit::MoveBody {
                member: 0,
                body: None,
            },
            StageEdit::AddMesh { distance: 0 },
        ];
        match carried {
            Some(axis) => {
                let gear = (0..shape.members.len())
                    .find(|&i| shape.axis_of_slot(shape.slot_of_member(i)) == Some(axis))
                    .unwrap();
                out.push(StageEdit::AddStep { axis });
                out.push(StageEdit::AddCentral { gear, ring: true });
                let body = shape.members[gear].body;
                if !shape.couplings.iter().any(|c| c.contains(&body)) {
                    out.push(StageEdit::Couple { body });
                }
                // No sun fits inside a planocentric, and it says so.
                if shape.members.len() > 2 {
                    out.push(StageEdit::AddCentral { gear, ring: false });
                }
            }
            None => out.push(StageEdit::AddAxis {
                mate: shape.members.len() - 1,
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
        for preset in StagePreset::ALL {
            let base = preset.build();
            for edit in applicable(&base) {
                // A pair on a crossed distance is a second point contact at
                // the same angle, which the worm's proportions do not size;
                // a parallel pair is what "add pair" is for.
                if matches!(edit, StageEdit::AddMesh { .. })
                    && base.family() != arr::StageFamily::Parallel
                {
                    continue;
                }
                let mut shape = base.clone();
                self::edit(&mut shape, edit).unwrap_or_else(|e| panic!("{preset:?} {edit:?}: {e}"));
                let r = conventionally(&shape);
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

    /// **Every add undoes**: the member, the step, the axis or the pair
    /// removed again is the shape it was, field for field — the bodies it
    /// listed included, an added body being the last and its removal
    /// moving no other.
    #[test]
    fn every_add_undoes() {
        let wolfrom = StagePreset::Wolfrom.build();
        // Members: planet, ring 1, ring 2.
        let planet = 0;
        for (add, remove) in [
            (
                StageEdit::AddCentral {
                    gear: planet,
                    ring: true,
                },
                StageEdit::RemoveMember {
                    member: wolfrom.members.len(),
                },
            ),
            (
                StageEdit::AddCentral {
                    gear: planet,
                    ring: false,
                },
                StageEdit::RemoveMember {
                    member: wolfrom.members.len(),
                },
            ),
            (
                StageEdit::AddStep { axis: 1 },
                StageEdit::RemoveStep {
                    gear: wolfrom.members.len(),
                },
            ),
        ] {
            let mut shape = wolfrom.clone();
            edit(&mut shape, add).unwrap();
            assert!(!same(&shape, &wolfrom), "{add:?} changed nothing");
            edit(&mut shape, remove).unwrap();
            assert!(same(&shape, &wolfrom), "{add:?} then {remove:?}");
        }
        let idler = StagePreset::Idler.build();
        let mut shape = idler.clone();
        edit(&mut shape, StageEdit::AddAxis { mate: 2 }).unwrap();
        edit(&mut shape, StageEdit::RemoveAxis { axis: 3 }).unwrap();
        assert!(same(&shape, &idler), "an axis added and removed");
        let layshaft = StagePreset::Layshaft.build();
        let mut shape = layshaft.clone();
        edit(&mut shape, StageEdit::AddMesh { distance: 0 }).unwrap();
        let last = shape.meshes.len() - 1;
        edit(&mut shape, StageEdit::RemoveMesh { mesh: last }).unwrap();
        assert!(same(&shape, &layshaft), "a mesh added and removed");
        // A coupling added to a hula's wobble body and taken away, the
        // shaft it turned with it.
        let hula = arr::hula([19, 18, 17, 18], [1.0, 1.0]);
        let mut shape = hula.clone();
        edit(&mut shape, StageEdit::Couple { body: 4 }).unwrap();
        assert!(!same(&shape, &hula), "a coupling changed nothing");
        edit(&mut shape, StageEdit::Uncouple { coupling: 0 }).unwrap();
        assert!(same(&shape, &hula), "a coupling added and removed");
    }

    /// **A refused edit changes nothing**, and refuses for the reason named:
    /// the last central on a step, the last gear on a planet axis, a
    /// chain's two axes, a distance's one mesh, an edit of the other family.
    #[test]
    fn a_refused_edit_changes_nothing() {
        let cases: Vec<(Shape, StageEdit, EditRefused)> = vec![
            (
                StagePreset::Planocentric.build(),
                StageEdit::Couple { body: 4 },
                EditRefused::Coupled,
            ),
            (
                StagePreset::Spur.build(),
                StageEdit::Couple { body: 1 },
                EditRefused::WrongFamily,
            ),
            (
                StagePreset::Planocentric.build(),
                StageEdit::Uncouple { coupling: 1 },
                EditRefused::NoSuchIndex,
            ),
            (
                StagePreset::Planocentric.build(),
                StageEdit::RemoveMember { member: 1 },
                EditRefused::LastOnItsStep,
            ),
            (
                StagePreset::Planocentric.build(),
                StageEdit::RemoveStep { gear: 0 },
                EditRefused::LastOnItsStep,
            ),
            (
                StagePreset::Spur.build(),
                StageEdit::RemoveAxis { axis: 1 },
                EditRefused::LastOfItsKind,
            ),
            // An idler goes with its chain's end, not before it: its two
            // mates would mesh nothing.
            (
                StagePreset::Idler.build(),
                StageEdit::RemoveAxis { axis: 1 },
                EditRefused::LastOfItsKind,
            ),
            (
                StagePreset::Planetary.build(),
                StageEdit::RemoveAxis { axis: 0 },
                EditRefused::WrongFamily,
            ),
            (
                StagePreset::Spur.build(),
                StageEdit::RemoveMesh { mesh: 0 },
                EditRefused::LastOfItsKind,
            ),
            (
                StagePreset::Spur.build(),
                StageEdit::AddStep { axis: 0 },
                EditRefused::WrongFamily,
            ),
            // A sun meshes its planets in the carrier, and a gear on an axis
            // fixed in ground cannot share that frame.
            (
                StagePreset::Planetary.build(),
                StageEdit::AddAxis { mate: 0 },
                EditRefused::WrongFamily,
            ),
            (
                StagePreset::Planetary.build(),
                StageEdit::MoveBody {
                    member: 0,
                    body: Some(4),
                },
                EditRefused::NotOnTheAxis,
            ),
            (
                StagePreset::Planetary.build(),
                StageEdit::RemoveMember { member: 9 },
                EditRefused::NoSuchIndex,
            ),
            (
                StagePreset::Planocentric.build(),
                StageEdit::AddCentral {
                    gear: 0,
                    ring: false,
                },
                EditRefused::NoRoom,
            ),
        ];
        for (before, what, why) in cases {
            let mut shape = before.clone();
            assert_eq!(edit(&mut shape, what), Err(why), "{what:?}");
            assert!(same(&shape, &before), "{what:?} touched the shape");
        }
    }

    /// **A move keeps what a body was told**: a member alone on its body
    /// moved to a new one changes nothing — the case entry at the body and
    /// the stage the body runs on to stay — and a gear cannot be moved onto
    /// the body that carries the planets it meshes.
    #[test]
    fn a_move_to_a_body_of_its_own_is_no_move_and_the_carrier_takes_no_gear() {
        let mut t = Train::chained(
            vec![StagePreset::Spur.build(), StagePreset::Planetary.build()],
            |t| vec![LoadCase::ultimate(t.port(0, 1), t.port(1, 2), 1.0, 1000.0)],
        );
        let before = t.clone();
        let shared = t.port(0, 2);
        assert_eq!(t.ends_of(shared).len(), 2, "the chain joined gear 2 onward");
        t.edit_stage(
            0,
            StageEdit::MoveBody {
                member: 1,
                body: None,
            },
        )
        .unwrap();
        assert_eq!(t.ends_of(shared).len(), 2, "gear 2 still runs on");
        assert_eq!(t.load_cases, before.load_cases);
        assert!(same(&t.stages[0], &before.stages[0]));
        // The set's carrier is its slot 2; its sun may not go there.
        let carrier = t.port(1, 2);
        assert_eq!(
            t.edit_stage(
                1,
                StageEdit::MoveBody {
                    member: 0,
                    body: Some(carrier),
                },
            ),
            Err(EditRefused::CarriesAnAxis)
        );
    }

    /// **A gear moved off a shaft does not take the shaft with it.** A body
    /// a stage lists is a port the train may hold, share or load, and it
    /// stays where anything still names it — with nothing on it, which is
    /// what a gearbox in neutral is. Dropping it took the coupling and the
    /// case with it: a layshaft's output moved to an idler left the next
    /// stage joined to nothing.
    ///
    /// It is given up where nothing names it, so a stage asked about alone
    /// keeps no numbers it has no use for. And engaging another ratio is
    /// the two moves it is on the machine: the other gear onto the output,
    /// this one off to a body of its own.
    #[test]
    fn a_gear_moved_off_a_shaft_does_not_take_the_shaft_with_it() {
        let lay = || arr::layshaft((17, 43), &[(41, 19), (29, 31)], 1);
        let mut t = Train::chained(vec![lay(), StagePreset::Spur.build()], |t| {
            vec![LoadCase::ultimate(t.port(0, 1), t.port(1, 2), 1.0, 1000.0)]
        });
        // The layshaft's output (slot 2) runs on to the spur; the engaged
        // pair's gear is the one sitting on it, and the other pair's idles
        // on a body of its own.
        let output = t.port(0, 2);
        let input = t.port(0, 1);
        assert_eq!(t.ends_of(output).len(), 2, "the output runs on");
        let shape = t.stages[0].clone();
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

        // **The destructive move, which is not destructive now**: the
        // engaged gear onto the idler's body. The output keeps its number,
        // its end on the spur and the case at it, with nothing on it.
        t.edit_stage(
            0,
            StageEdit::MoveBody {
                member: engaged,
                body: Some(idler),
            },
        )
        .unwrap();
        assert_eq!(t.port(0, 2), output, "the output is where it was");
        assert_eq!(t.ends_of(output).len(), 2, "...and still runs on");
        assert!(
            t.stages[0].members_on_body(output).is_empty(),
            "nothing is engaged: neutral"
        );

        // ...and the other ratio engaged: the idle gear on, this one off —
        // which it may be now, since it shares.
        t.edit_stage(
            0,
            StageEdit::MoveBody {
                member: idle,
                body: Some(output),
            },
        )
        .unwrap();
        t.edit_stage(
            0,
            StageEdit::MoveBody {
                member: engaged,
                body: None,
            },
        )
        .unwrap();
        assert_eq!(t.stages[0].members_on_body(output), vec![idle]);
        assert_eq!(t.ends_of(output).len(), 2, "the output is the output");

        // **A bare body nothing names is given up**: the same first move on
        // a stage of its own, with no train to mean the shaft to be there.
        let mut alone = Train::chained(vec![lay()], |_| Vec::new());
        let was = alone.stages[0].bodies.len();
        let (on, to) = {
            let s = &alone.stages[0];
            (s.members_on_body(alone.port(0, 2))[0], s.members[idle].body)
        };
        alone
            .edit_stage(
                0,
                StageEdit::MoveBody {
                    member: on,
                    body: Some(to),
                },
            )
            .unwrap();
        assert_eq!(
            alone.stages[0].bodies.len(),
            was - 1,
            "the shaft nothing named is given up"
        );
    }

    /// **What a remove takes off a stage, the train follows**: a Wolfrom's
    /// first ring removed takes its body (2) out of the train, the hold
    /// written there with it, and the bodies after it close up — the second
    /// ring from 3 to 2, still running on to the spur, the case entry at
    /// the crank still at 1.
    #[test]
    fn a_remove_repoints_the_train_and_drops_what_named_the_body() {
        let mut t = Train::chained(
            vec![StagePreset::Wolfrom.build(), StagePreset::Spur.build()],
            |t| vec![LoadCase::ultimate(t.port(0, 1), t.port(1, 2), 1.0, 1000.0)],
        );
        // The chain joins ring 2 (slot 3) onward; hold ring 1 (slot 2,
        // member 1 after the planet) explicitly too.
        assert_eq!(t.port(0, 3), 3);
        assert_eq!(t.ends_of(3).len(), 2);
        t.hold(t.port(0, 2));
        t.edit_stage(0, StageEdit::RemoveMember { member: 1 })
            .unwrap();
        assert_eq!(t.stages[0].members.len(), 2);
        assert_eq!(t.port(0, 2), 2, "ring 2 closed up to body 2");
        assert_eq!(
            t.ends_of(2).len(),
            2,
            "ring 2 still runs on to the spur: {:?}",
            t.stages[1].bodies
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
        t.edit_stage(
            0,
            StageEdit::AddCentral {
                gear: 0,
                ring: false,
            },
        )
        .unwrap();
        assert_eq!(t.load_cases[0].loads, before.load_cases[0].loads);
        assert_eq!(t.held, before.held);
        for b in 1..=before.max_body() {
            assert_eq!(t.ends_of(b), before.ends_of(b), "body {b} moved");
        }
    }

    /// **The hula is reached from the Wolfrom preset by the card's edits**:
    /// a step added, the first planet's second ring removed, the count set
    /// to one and the teeth written — the same motion `arrangements::hula`
    /// lists, ratio for ratio, on the counts the tables print.
    #[test]
    fn the_hula_is_reached_from_the_wolfrom_by_edits() {
        let mut shape = StagePreset::Wolfrom.build();
        // Members: planet, ring 1, ring 2. A step: planet 2 and a ring on it.
        edit(&mut shape, StageEdit::AddStep { axis: 1 }).unwrap();
        assert_eq!(shape.members.len(), 5);
        // Ring 2 off the first planet: the first step keeps ring 1.
        edit(&mut shape, StageEdit::RemoveMember { member: 2 }).unwrap();
        // Now: planet 1, ring 1 (grounded), planet 2, ring on planet 2 —
        // the hula's 18, 19, 17, 18.
        shape.axes[1].count = 1;
        for (m, z) in shape.members.iter_mut().zip([18, 19, 17, 18]) {
            m.gear.teeth = z;
        }
        // Under the hula's arrangement on each: crank driven, grounded ring
        // held, the output ring out — slots 1, 2 and 4 here, where the ring
        // removed gave its place up, and 1, 2 and 3 on the list.
        let edited = under(&shape, StageBoundary::holding(5, &[2], 1, 4));
        let listed = under(
            &arr::hula([19, 18, 17, 18], [1.0, 1.0]),
            StageBoundary::holding(5, &[2], 1, 3),
        );
        assert!(
            (edited.ratio.unwrap() - listed.ratio.unwrap()).abs() < 1e-9,
            "{:?} against the list's {:?}",
            edited.ratio,
            listed.ratio
        );
    }

    /// **The hula is reached from the planocentric by the card's edits**,
    /// and the planocentric from the hula: the coupling is the stage's to
    /// lose. A step on the planet — a gear and a ring on it — and the
    /// coupling taken away is a hula whose second ring is the output; a
    /// coupling on a hula's wobble body and its second step taken away is
    /// a planocentric whose coupled shaft is. Each is the list it reaches,
    /// ratio for ratio, on the counts the tables print.
    #[test]
    fn the_planocentric_and_the_hula_are_one_edit_apart() {
        let mut shape = StagePreset::Planocentric.build();
        // Bodies: carrier, ring, the coupled shaft, the planet's body.
        edit(&mut shape, StageEdit::AddStep { axis: 1 }).unwrap();
        edit(&mut shape, StageEdit::Uncouple { coupling: 0 }).unwrap();
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
        let edited = under(&shape, StageBoundary::holding(5, &[2], 1, 4));
        let listed = under(
            &arr::hula([19, 18, 17, 18], [1.0, 1.0]),
            StageBoundary::holding(5, &[2], 1, 3),
        );
        assert!(
            (edited.ratio.unwrap() - listed.ratio.unwrap()).abs() < 1e-9,
            "{:?} against the list's {:?}",
            edited.ratio,
            listed.ratio
        );

        // ...and back: the hula's wobble body coupled to a shaft on the
        // centre line, its second step and the output ring on it taken away.
        let mut shape = arr::hula([19, 18, 17, 18], [1.0, 1.0]);
        edit(&mut shape, StageEdit::Couple { body: 4 }).unwrap();
        edit(&mut shape, StageEdit::RemoveStep { gear: 1 }).unwrap();
        assert_eq!(shape.couplings, vec![[5, 4]]);
        // Bodies: carrier, the grounded ring, the wobble body, the shaft.
        let edited = under(&shape, StageBoundary::holding(5, &[2], 1, 4));
        let listed = under(
            &arr::planocentric(18, 19),
            StageBoundary::holding(5, &[2], 1, 3),
        );
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
        let mut shape = StagePreset::Wolfrom.build();
        let planet = 0;
        for _ in 0..2 {
            edit(
                &mut shape,
                StageEdit::AddCentral {
                    gear: planet,
                    ring: false,
                },
            )
            .unwrap();
        }
        // Rings 1 and 2 are members 1 and 2; what is left is the planet
        // and the two suns, 24 and 23 teeth at the Wolfrom's radius.
        edit(&mut shape, StageEdit::RemoveMember { member: 2 }).unwrap();
        edit(&mut shape, StageEdit::RemoveMember { member: 1 }).unwrap();
        let (s1, s2) = (
            f64::from(shape.members[1].gear.teeth),
            f64::from(shape.members[2].gear.teeth),
        );
        assert!(s1 != s2, "two suns of one count would turn as one");
        // Slots: carrier 1, planet 2, sun 1 at 3, sun 2 at 4.
        let r = under(&shape, StageBoundary::holding(5, &[3], 1, 4));
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
}
