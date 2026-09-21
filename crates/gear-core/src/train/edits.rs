//! **The edits a stage takes on the card**, each a rule about what else has
//! to change — the mirror, one level down, of what `conditions.rs` does
//! for a train.
//!
//! A designer permutes an arrangement by adding and removing: a step on the
//! planet shaft, a sun or a ring on a planet gear, an axis at the end of a
//! parallel chain, a pair on a distance; and by moving a member to another
//! shaft on its axis. Nothing is *flipped*: a member's kind is decided by
//! the button that adds it and a ring stays a ring, since a sun and a ring
//! differ in more than a flag (a cutter, a shift rule) and a swap is a
//! remove and an add, the new member sized by the core to what it meets.
//!
//! Every add appends — a new shaft is the last, a new member the last, so
//! nothing a case or a coupling names moves. Every remove renumbers, and
//! hands back what moved where ([`Renumbered`]) for the train to repoint
//! its cases, couplings and constraints by ([`super::Train::edit_stage`]).
//! The invariants the edits keep: every member is in a mesh, every planet
//! gear meets a central member, every distance carries a mesh, and a
//! carrier's shaft is never removed.

use super::shape::{Axis, Distance, Member, MeshInput, ShaftOn, Shape};
use super::StageGear;
use crate::kinematics::{Shaft, GROUND};
use crate::params::Auto;

/// What a card asks of a stage. Indices are the shape's own: a member, a
/// mesh, a distance by position; a shaft as the wiring numbers it, ground
/// being 0 and the first listed shaft 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub enum StageEdit {
    /// **A step**: one more gear on the planet shaft of `axis`, a ring
    /// meshing it, both appended. The gear copies the last gear on that
    /// shaft; the ring is sized to the carrier radius the axis already runs
    /// at.
    AddStep { axis: usize },
    /// **A step removed**: the planet gear `gear` and every central member
    /// meshing it. Refused for the last gear on its axis.
    RemoveStep { gear: usize },
    /// **A sun or a ring on a planet gear**, on a new shaft of the central
    /// axis, sized to the carrier radius; a count that would repeat one
    /// already on that gear is moved by a tooth, since equal rings on one
    /// planet lock the stage.
    AddCentral { gear: usize, ring: bool },
    /// **A central member removed**, with its mesh and, where nothing else
    /// is on it, its shaft. Refused where it is the last member meeting its
    /// planet gear — remove the step instead — and for a planet gear, which
    /// is a step.
    RemoveMember { member: usize },
    /// **An axis at the end of a parallel chain**: a shaft, a gear copying
    /// the last member, a mesh with it, a distance to its axis.
    AddAxis,
    /// **The last axis of a parallel chain removed**, with everything on it.
    /// Refused where two would be left short.
    RemoveAxis,
    /// **A pair on a distance**, copying its first: one gear on the shaft
    /// the distance's pairs share — a layshaft — or the first pair's second
    /// gear's shaft where none is shared yet, the other on a shaft of its
    /// own, an idler until it is moved onto the output.
    AddPair { distance: usize },
    /// **A pair removed**: both members of `mesh`, their shafts where
    /// nothing else is on them. Refused for a distance's last mesh, and for
    /// a mesh a carried axis is on.
    RemovePair { mesh: usize },
    /// **A member moved to another shaft on its axis** — `None` a new one —
    /// and the shaft it leaves removed where it is emptied and carries no
    /// axis.
    MoveShaft { member: usize, shaft: Option<Shaft> },
}

/// Why an edit is refused: the invariant it would break.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditRefused {
    /// No such member, mesh, axis, distance or shaft.
    NoSuchIndex,
    /// The last gear on a planet axis, or the last member meeting a planet
    /// gear: a step keeps at least one of each.
    LastOnItsStep,
    /// A parallel chain keeps two axes; a distance keeps one mesh.
    LastOfItsKind,
    /// The edit belongs to another family: a step on a stage with no
    /// carried axis, an axis on one with.
    WrongFamily,
    /// A shaft not on the member's axis.
    NotOnTheAxis,
    /// No member of that kind fits at the radius the axis runs at: a sun
    /// inside a planocentric, whose planet all but fills its ring.
    NoRoom,
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
        }
    }
}

impl std::fmt::Display for EditRefused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NoSuchIndex => "no such member, mesh, axis or shaft",
            Self::LastOnItsStep => "the last on its step",
            Self::LastOfItsKind => "the last of its kind",
            Self::WrongFamily => "not an edit of this family",
            Self::NotOnTheAxis => "not a shaft on the member's axis",
            Self::NoRoom => "nothing of that kind fits at this radius",
        })
    }
}

/// **Where each shaft went**: indexed by the old wiring number, `None` for
/// a shaft removed. Ground stays ground; an add moves nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Renumbered(pub Vec<Option<Shaft>>);

impl Renumbered {
    fn identity(shafts: usize) -> Self {
        Self((0..=shafts).map(Some).collect())
    }

    /// The new number of an old shaft.
    #[must_use]
    pub fn of(&self, shaft: Shaft) -> Option<Shaft> {
        self.0.get(shaft).copied().flatten()
    }
}

impl Shape {
    /// Apply one edit; what moved where.
    ///
    /// # Errors
    ///
    /// [`EditRefused`] where the edit would break an invariant, with the
    /// shape untouched.
    pub fn edit(&mut self, edit: StageEdit) -> Result<Renumbered, EditRefused> {
        match edit {
            StageEdit::AddStep { axis } => self.add_step(axis),
            StageEdit::RemoveStep { gear } => self.remove_step(gear),
            StageEdit::AddCentral { gear, ring } => self.add_central(gear, ring),
            StageEdit::RemoveMember { member } => self.remove_member(member),
            StageEdit::AddAxis => self.add_axis(),
            StageEdit::RemoveAxis => self.remove_axis(),
            StageEdit::AddPair { distance } => self.add_pair(distance),
            StageEdit::RemovePair { mesh } => self.remove_pair(mesh),
            StageEdit::MoveShaft { member, shaft } => self.move_shaft(member, shaft),
        }
    }

    // ----------------------------------------------------------- reading ---

    fn carried(&self, axis: usize) -> bool {
        self.axes[axis].carried_by != GROUND
    }

    /// The axis a carried axis goes round: its carrier's.
    fn central_axis_of(&self, axis: usize) -> Option<usize> {
        self.axis_of_shaft(self.axes[axis].carried_by)
    }

    fn members_on_shaft(&self, shaft: Shaft) -> Vec<usize> {
        (0..self.members.len())
            .filter(|&i| self.members[i].shaft == shaft)
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

    fn is_planet_gear(&self, member: usize) -> bool {
        self.axis_of_shaft(self.shaft_of(member))
            .is_some_and(|a| self.carried(a))
    }

    fn carries_an_axis(&self, shaft: Shaft) -> bool {
        self.axes.iter().any(|a| a.carried_by == shaft)
    }

    /// **The carrier radius a carried axis runs at**, read off any mesh a
    /// central member has with a gear on it: `(z_c ± z_p) m / 2`, a ring
    /// minus. `None` where no central member meets the axis yet.
    fn carrier_radius(&self, axis: usize) -> Option<f64> {
        self.meshes.iter().find_map(|m| {
            let (p, c) = [(m.a, m.b), (m.b, m.a)].into_iter().find(|&(p, c)| {
                self.axis_of_shaft(self.shaft_of(p)) == Some(axis) && !self.is_planet_gear(c)
            })?;
            let (zp, zc) = (
                f64::from(self.members[p].gear.teeth),
                f64::from(self.members[c].gear.teeth),
            );
            let module = self.members[c].module;
            Some(if self.members[c].ring.is_some() {
                (zc - zp) * module / 2.0
            } else {
                (zc + zp) * module / 2.0
            })
        })
    }

    /// The friction the stage's meshes run with — the first mesh's, or the
    /// set's where there is none.
    fn friction(&self) -> (f64, f64) {
        self.meshes
            .first()
            .map_or((0.08, 0.16), |m| (m.sliding_friction, m.static_friction))
    }

    fn push_shaft(&mut self, axis: usize) -> Shaft {
        self.shafts.push(ShaftOn { axis });
        self.shafts.len()
    }

    fn push_mesh(&mut self, a: usize, b: usize) {
        // A ring goes second, as the mesh's kind is read.
        let (a, b) = if self.members[a].ring.is_some() {
            (b, a)
        } else {
            (a, b)
        };
        let (sliding_friction, static_friction) = self.friction();
        self.meshes.push(MeshInput {
            a,
            b,
            sliding_friction,
            static_friction,
        });
    }

    /// A distance between two axes, automatic, at the first distance's
    /// clearances and tolerances where there is one.
    fn push_distance(&mut self, axes: [usize; 2]) {
        let like = self.distances.first().copied();
        self.distances.push(Distance {
            axes,
            angle: 0.0,
            worm: false,
            distance: Auto::automatic(0.0),
            clearance: like.map_or(Auto::fixed(0.02), |d| d.clearance),
            tip_clearance: like.map_or(0.0, |d| d.tip_clearance),
            tolerance_plus: like.map_or(0.02, |d| d.tolerance_plus),
            tolerance_minus: like.map_or(0.02, |d| d.tolerance_minus),
            axial_clearance: 0.0,
        });
    }

    // ---------------------------------------------------------- epicyclic ---

    fn add_step(&mut self, axis: usize) -> Result<Renumbered, EditRefused> {
        if axis >= self.axes.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        if !self.carried(axis) {
            return Err(EditRefused::WrongFamily);
        }
        let shaft = (1..=self.shafts.len())
            .find(|&s| self.shafts[s - 1].axis == axis)
            .ok_or(EditRefused::NoSuchIndex)?;
        let last = self
            .members_on_shaft(shaft)
            .last()
            .copied()
            .ok_or(EditRefused::NoSuchIndex)?;
        let gear = Member {
            ring: None,
            ..self.members[last].clone()
        };
        self.members.push(gear);
        let new = self.members.len() - 1;
        self.add_central(new, true)
    }

    fn remove_step(&mut self, gear: usize) -> Result<Renumbered, EditRefused> {
        if gear >= self.members.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        if !self.is_planet_gear(gear) {
            return Err(EditRefused::WrongFamily);
        }
        if self.members_on_shaft(self.shaft_of(gear)).len() < 2 {
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
        let mut moved = Renumbered::identity(self.shafts.len());
        for i in going.into_iter().rev() {
            let step = self.drop_member(i);
            moved = compose(&moved, &step);
        }
        Ok(moved)
    }

    fn add_central(&mut self, gear: usize, ring: bool) -> Result<Renumbered, EditRefused> {
        if gear >= self.members.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        let planet_axis = self
            .axis_of_shaft(self.shaft_of(gear))
            .ok_or(EditRefused::NoSuchIndex)?;
        if !self.carried(planet_axis) {
            return Err(EditRefused::WrongFamily);
        }
        let central_axis = self
            .central_axis_of(planet_axis)
            .ok_or(EditRefused::NoSuchIndex)?;
        let (zp, module) = (self.members[gear].gear.teeth, self.members[gear].module);
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
        // on two equal gears of one planet shaft turn as one — a ring added
        // to a step at the first step's count carries nothing — so the
        // count moves by a tooth: **down**, where it can, since a shift can
        // open a mesh past its reference distance by `1/cos α` at most,
        // some six per cent, and a planocentric's carrier radius is a few
        // teeth, so a ring a tooth *larger* at that radius has nowhere to
        // close; a tooth smaller always has.
        let floor = if ring { f64::from(zp) + 2.0 } else { 4.0 };
        let taken = |z: f64| {
            (0..self.members.len())
                .filter(|&p| self.axis_of_shaft(self.shaft_of(p)) == Some(planet_axis))
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
        let shaft = self.push_shaft(central_axis);
        let cutter = ring.then(|| self.members.iter().find_map(|m| m.ring).unwrap_or_default());
        self.members.push(Member {
            shaft,
            // Its shift automatic: a second central at one carrier radius
            // is closed by its shift, which one given at zero could not do.
            gear: StageGear {
                teeth,
                profile_shift: Auto::automatic(0.0),
                ..self.members[gear].gear.clone()
            },
            module,
            thickness_mod: Auto::automatic(1.0),
            ring: cutter,
            pitch_diameter: Auto::automatic(0.0),
        });
        let new = self.members.len() - 1;
        self.push_mesh(new, gear);
        Ok(Renumbered::identity(self.shafts.len()))
    }

    fn remove_member(&mut self, member: usize) -> Result<Renumbered, EditRefused> {
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
        Ok(self.drop_member(member))
    }

    // ----------------------------------------------------------- parallel ---

    fn add_axis(&mut self) -> Result<Renumbered, EditRefused> {
        if self.axes.iter().any(|a| a.carried_by != GROUND) {
            return Err(EditRefused::WrongFamily);
        }
        let last_axis = self.axes.len() - 1;
        let last = (0..self.members.len())
            .rev()
            .find(|&i| self.axis_of_shaft(self.shaft_of(i)) == Some(last_axis))
            .ok_or(EditRefused::NoSuchIndex)?;
        self.axes.push(Axis {
            carried_by: GROUND,
            count: 1,
        });
        let axis = self.axes.len() - 1;
        let shaft = self.push_shaft(axis);
        self.members.push(Member {
            shaft,
            ring: None,
            ..self.members[last].clone()
        });
        let new = self.members.len() - 1;
        self.push_mesh(last, new);
        self.push_distance([last_axis, axis]);
        Ok(Renumbered::identity(self.shafts.len()))
    }

    fn remove_axis(&mut self) -> Result<Renumbered, EditRefused> {
        if self.axes.iter().any(|a| a.carried_by != GROUND) {
            return Err(EditRefused::WrongFamily);
        }
        if self.axes.len() < 3 {
            return Err(EditRefused::LastOfItsKind);
        }
        let axis = self.axes.len() - 1;
        let mut moved = Renumbered::identity(self.shafts.len());
        let mut on_axis: Vec<usize> = (0..self.members.len())
            .filter(|&i| self.axis_of_shaft(self.shaft_of(i)) == Some(axis))
            .collect();
        on_axis.sort_unstable();
        for i in on_axis.into_iter().rev() {
            let step = self.drop_member(i);
            moved = compose(&moved, &step);
        }
        // Any shaft left on it, and its distances, and the axis.
        for s in (1..=self.shafts.len()).rev() {
            if self.shafts[s - 1].axis == axis {
                let step = self.drop_shaft(s);
                moved = compose(&moved, &step);
            }
        }
        self.distances.retain(|d| !d.axes.contains(&axis));
        self.axes.pop();
        Ok(moved)
    }

    fn add_pair(&mut self, distance: usize) -> Result<Renumbered, EditRefused> {
        if distance >= self.distances.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        let first = self
            .meshes_on(distance)
            .first()
            .copied()
            .ok_or(EditRefused::NoSuchIndex)?;
        let m = self.meshes[first];
        // The shaft the distance's pairs share: the one with the most
        // members among them, where that is more than one; else the first
        // pair's second gear's.
        let on_distance: Vec<usize> = self
            .meshes_on(distance)
            .into_iter()
            .flat_map(|k| [self.meshes[k].a, self.meshes[k].b])
            .collect();
        let shared = on_distance
            .iter()
            .map(|&i| self.shaft_of(i))
            .max_by_key(|&s| {
                on_distance
                    .iter()
                    .filter(|&&i| self.shaft_of(i) == s)
                    .count()
            })
            .filter(|&s| {
                on_distance
                    .iter()
                    .filter(|&&i| self.shaft_of(i) == s)
                    .count()
                    > 1
            })
            .unwrap_or(self.shaft_of(m.b));
        let (on_shared, alone) = if self.shaft_of(m.a) == shared {
            (m.a, m.b)
        } else {
            (m.b, m.a)
        };
        let axis = self
            .axis_of_shaft(self.shaft_of(alone))
            .ok_or(EditRefused::NoSuchIndex)?;
        self.members.push(Member {
            shaft: shared,
            ..self.members[on_shared].clone()
        });
        let shaft = self.push_shaft(axis);
        self.members.push(Member {
            shaft,
            ..self.members[alone].clone()
        });
        let n = self.members.len();
        self.push_mesh(n - 2, n - 1);
        Ok(Renumbered::identity(self.shafts.len()))
    }

    fn remove_pair(&mut self, mesh: usize) -> Result<Renumbered, EditRefused> {
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
        let mut moved = Renumbered::identity(self.shafts.len());
        for i in [hi, lo] {
            if self.meshes_of_member(i).len() < 2 {
                let step = self.drop_member(i);
                moved = compose(&moved, &step);
            } else {
                self.meshes.retain(|x| !(x.a == m.a && x.b == m.b));
            }
        }
        Ok(moved)
    }

    // --------------------------------------------------------------- both ---

    fn move_shaft(
        &mut self,
        member: usize,
        shaft: Option<Shaft>,
    ) -> Result<Renumbered, EditRefused> {
        if member >= self.members.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        let from = self.shaft_of(member);
        let axis = self.axis_of_shaft(from).ok_or(EditRefused::NoSuchIndex)?;
        let to = match shaft {
            Some(s) => {
                if s == GROUND || s > self.shafts.len() {
                    return Err(EditRefused::NoSuchIndex);
                }
                if self.shafts[s - 1].axis != axis {
                    return Err(EditRefused::NotOnTheAxis);
                }
                s
            }
            None => self.push_shaft(axis),
        };
        self.members[member].shaft = to;
        if self.members_on_shaft(from).is_empty() && !self.carries_an_axis(from) {
            return Ok(self.drop_shaft(from));
        }
        Ok(Renumbered::identity(self.shafts.len()))
    }

    // ---------------------------------------------------------- the drops ---

    /// A member gone, with its meshes, and its shaft where it was alone on
    /// it and the shaft carries no axis.
    fn drop_member(&mut self, member: usize) -> Renumbered {
        let shaft = self.shaft_of(member);
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
        if self.members_on_shaft(shaft).is_empty() && !self.carries_an_axis(shaft) {
            return self.drop_shaft(shaft);
        }
        Renumbered::identity(self.shafts.len())
    }

    /// A shaft gone and every number above it moved down: the members'
    /// shafts, the carriers.
    fn drop_shaft(&mut self, shaft: Shaft) -> Renumbered {
        let old = self.shafts.len();
        self.shafts.remove(shaft - 1);
        for m in &mut self.members {
            if m.shaft > shaft {
                m.shaft -= 1;
            }
        }
        for a in &mut self.axes {
            if a.carried_by > shaft {
                a.carried_by -= 1;
            }
        }
        Renumbered(
            (0..=old)
                .map(|s| match s.cmp(&shaft) {
                    std::cmp::Ordering::Less => Some(s),
                    std::cmp::Ordering::Equal => None,
                    std::cmp::Ordering::Greater => Some(s - 1),
                })
                .collect(),
        )
    }
}

/// `first` then `second`, as one map from the original numbering.
fn compose(first: &Renumbered, second: &Renumbered) -> Renumbered {
    Renumbered(
        first
            .0
            .iter()
            .map(|s| s.and_then(|s| second.of(s)))
            .collect(),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    //! **The laws an edit obeys**: it leaves a stage that solves, it undoes,
    //! it refuses whole, and what it renumbers the train follows.

    use super::super::arrangements::{self as arr, StagePreset};
    use super::super::shape::{solve_loads, ShapeResult};
    use super::super::{
        test_library as library, Constrained, LoadCase, Reversal, ShaftRef, Stage, StageBoundary,
        StageLoads, Train,
    };
    use super::*;

    fn conventionally(shape: &Shape) -> ShapeResult {
        under(
            shape,
            StageBoundary::conventional(&shape.wiring(), &shape.ports()),
        )
    }

    fn under(shape: &Shape, boundary: StageBoundary) -> ShapeResult {
        solve_loads(
            shape,
            &StageLoads::at(2.0, 3000.0).under(boundary),
            &library(),
            Reversal::default(),
        )
        .unwrap_or_else(|e| panic!("{e}: {shape:?}"))
    }

    fn same(a: &Shape, b: &Shape) -> bool {
        format!("{a:?}") == format!("{b:?}")
    }

    /// The edits that apply to a shape, each on its first candidate.
    fn applicable(shape: &Shape) -> Vec<StageEdit> {
        let carried = (0..shape.axes.len()).find(|&a| shape.axes[a].carried_by != GROUND);
        let mut out = vec![
            StageEdit::MoveShaft {
                member: 0,
                shaft: None,
            },
            StageEdit::AddPair { distance: 0 },
        ];
        match carried {
            Some(axis) => {
                let gear = (0..shape.members.len())
                    .find(|&i| shape.axis_of_shaft(shape.shaft_of(i)) == Some(axis))
                    .unwrap();
                out.push(StageEdit::AddStep { axis });
                out.push(StageEdit::AddCentral { gear, ring: true });
                // No sun fits inside a planocentric, and it says so.
                if shape.members.len() > 2 {
                    out.push(StageEdit::AddCentral { gear, ring: false });
                }
            }
            None => out.push(StageEdit::AddAxis),
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
                if matches!(edit, StageEdit::AddPair { .. })
                    && base.family() != arr::StageFamily::Parallel
                {
                    continue;
                }
                let mut shape = base.clone();
                shape
                    .edit(edit)
                    .unwrap_or_else(|e| panic!("{preset:?} {edit:?}: {e}"));
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
                            .any(|x| shape.members[x.a].shaft == m.shaft
                                || shape.members[x.b].shaft == m.shaft),
                        "{preset:?} after {edit:?}: a member in no mesh"
                    );
                }
            }
        }
    }

    /// **Every add undoes**: the member, the step, the axis or the pair
    /// removed again is the shape it was, field for field.
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
            shape.edit(add).unwrap();
            assert!(!same(&shape, &wolfrom), "{add:?} changed nothing");
            let moved = shape.edit(remove).unwrap();
            assert!(same(&shape, &wolfrom), "{add:?} then {remove:?}");
            // The shafts added were the last, so nothing before them moved.
            for s in 0..=wolfrom.shafts.len() {
                assert_eq!(moved.of(s), Some(s));
            }
        }
        let idler = StagePreset::Idler.build();
        let mut shape = idler.clone();
        shape.edit(StageEdit::AddAxis).unwrap();
        shape.edit(StageEdit::RemoveAxis).unwrap();
        assert!(same(&shape, &idler), "an axis added and removed");
        let layshaft = StagePreset::Layshaft.build();
        let mut shape = layshaft.clone();
        shape.edit(StageEdit::AddPair { distance: 0 }).unwrap();
        let last = shape.meshes.len() - 1;
        shape.edit(StageEdit::RemovePair { mesh: last }).unwrap();
        assert!(same(&shape, &layshaft), "a pair added and removed");
    }

    /// **A refused edit changes nothing**, and refuses for the reason named:
    /// the last central on a step, the last gear on a planet axis, a pair's
    /// two axes, a distance's one mesh, an edit of the other family.
    #[test]
    fn a_refused_edit_changes_nothing() {
        let cases: Vec<(Shape, StageEdit, EditRefused)> = vec![
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
                StageEdit::RemoveAxis,
                EditRefused::LastOfItsKind,
            ),
            (
                StagePreset::Spur.build(),
                StageEdit::RemovePair { mesh: 0 },
                EditRefused::LastOfItsKind,
            ),
            (
                StagePreset::Spur.build(),
                StageEdit::AddStep { axis: 0 },
                EditRefused::WrongFamily,
            ),
            (
                StagePreset::Planetary.build(),
                StageEdit::AddAxis,
                EditRefused::WrongFamily,
            ),
            (
                StagePreset::Planetary.build(),
                StageEdit::MoveShaft {
                    member: 0,
                    shaft: Some(4),
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
        for (before, edit, why) in cases {
            let mut shape = before.clone();
            assert_eq!(shape.edit(edit), Err(why), "{edit:?}");
            assert!(same(&shape, &before), "{edit:?} touched the shape");
        }
    }

    /// **What a remove renumbers, the train follows**: a Wolfrom's first
    /// ring removed (shaft 2) moves the second ring from shaft 3 to 2 — the
    /// case entry, the coupling and the hold written there move with it —
    /// and the hold that was on the removed shaft is gone.
    #[test]
    fn a_remove_repoints_the_train_and_drops_what_named_the_shaft() {
        let at = |stage, shaft| ShaftRef::Of { stage, shaft };
        let mut t = Train::chained(
            vec![
                Stage::Shape(Box::new(StagePreset::Wolfrom.build())),
                Stage::Shape(Box::new(StagePreset::Spur.build())),
            ],
            vec![LoadCase::ultimate(at(0, 1), at(1, 2), 1.0, 1000.0)],
        );
        // The chain couples ring 2 (shaft 3) onward; hold ring 1 (shaft 2,
        // member 1 after the planet) explicitly too.
        t.hold(at(0, 2));
        assert!(t.couplings.iter().any(|c| c.a == at(0, 3)));
        t.edit_stage(0, StageEdit::RemoveMember { member: 1 })
            .unwrap();
        assert_eq!(t.stages[0].as_shape().unwrap().members.len(), 2);
        assert!(
            t.couplings.iter().any(|c| c.a == at(0, 2)),
            "the coupling followed ring 2 to shaft 2: {:?}",
            t.couplings
        );
        assert!(
            t.constraints.is_empty(),
            "the hold on the removed shaft is gone: {:?}",
            t.constraints
        );
        assert_eq!(t.load_cases[0].loads[0].at, at(0, 1), "the crank stayed");
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
        assert_eq!(t.couplings, before.couplings);
        assert_eq!(t.load_cases[0].loads, before.load_cases[0].loads);
    }

    /// **The hula is reached from the Wolfrom preset by the card's edits**:
    /// a step added, the first planet's second ring removed, the count set
    /// to one and the teeth written — the same motion `arrangements::hula`
    /// lists, ratio for ratio, on the counts the tables print.
    #[test]
    fn the_hula_is_reached_from_the_wolfrom_by_edits() {
        let mut shape = StagePreset::Wolfrom.build();
        // Members: planet, ring 1, ring 2. A step: planet 2 and a ring on it.
        shape.edit(StageEdit::AddStep { axis: 1 }).unwrap();
        assert_eq!(shape.members.len(), 5);
        // Ring 2 off the first planet: the first step keeps ring 1.
        shape.edit(StageEdit::RemoveMember { member: 2 }).unwrap();
        // Now: planet 1, ring 1 (grounded), planet 2, ring on planet 2 —
        // the hula's 18, 19, 17, 18.
        shape.axes[1].count = 1;
        for (m, z) in shape.members.iter_mut().zip([18, 19, 17, 18]) {
            m.gear.teeth = z;
        }
        // Under the hula's arrangement on each: crank driven, grounded ring
        // held, the output ring out — shafts 1, 2 and 4 here, where the
        // ring removed gave its number up, and 1, 2 and 3 on the list.
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
        shape
            .edit(StageEdit::AddCentral {
                gear: planet,
                ring: false,
            })
            .unwrap();
        shape
            .edit(StageEdit::AddCentral {
                gear: planet,
                ring: false,
            })
            .unwrap();
        // Rings 1 and 2 are members 1 and 2; what is left is the planet
        // and the two suns, 24 and 23 teeth at the Wolfrom's radius.
        shape.edit(StageEdit::RemoveMember { member: 2 }).unwrap();
        shape.edit(StageEdit::RemoveMember { member: 1 }).unwrap();
        let (s1, s2) = (
            f64::from(shape.members[1].gear.teeth),
            f64::from(shape.members[2].gear.teeth),
        );
        assert!(s1 != s2, "two suns of one count would turn as one");
        // Shafts: carrier 1, planet 2, sun 1 at 3, sun 2 at 4.
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
