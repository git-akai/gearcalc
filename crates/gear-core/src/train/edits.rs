//! **The edits a train's graph takes**, each a rule about what else has to
//! change, made whole or refused whole ([`Edit`], made by
//! [`super::Train::edit`]) — and the card's vocabulary for the same
//! ([`StageEdit`]), read through its part into the graph's until the
//! interface asks the graph's edits by name.
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

use super::graph::Part;
use super::shape::{Member, Shape};
use super::StageGear;
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
pub enum Edit {
    /// **A gear meshing `mate`**, on `on` — a ring where `ring` — sized to
    /// what it meets: to the distance between the two axes where they have
    /// one (a sun or a ring on a planet gear to the radius its axis runs
    /// at), and on a new axis the mate's count, a ring twice it. It follows
    /// the mate's module and pressure angle, its shift and thickness
    /// automatic. Everything a card's *add* guessed at is one of these.
    AddGear { mate: usize, on: Place, ring: bool },
    /// **Another ratio across a distance**: a gear on `shared` — a body on
    /// one of the distance's two axes — meshing a gear on a new body of the
    /// other, the two copying the distance's first mesh. Which body the
    /// ratios share is asked: it is what makes the stage a layshaft.
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
    /// **A stage laid into the train**, its bodies numbered after the
    /// train's: its conventional input made one with `at` where given, and
    /// otherwise with the last part's remaining open output, the case
    /// entries there carried to its own ([`super::Train::push_stage`]).
    Insert { stage: Shape, at: Option<usize> },
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
pub enum Piece {
    Member(usize),
    Mesh(usize),
    Axis(usize),
    Body(usize),
    Coupling(usize),
}

/// **What a card asks of its stage** — the card's vocabulary for the graph's
/// edits ([`Edit`]), each read through its part into one of them with the
/// card's own refusals. Indices are the card's own — a member, a mesh, a
/// distance, an axis or a coupling by its position in the part
/// ([`super::graph::Part`]), which on a shape asked alone is the shape's —
/// and a body is the train's number for it.
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
    /// No axis distance joins the two axes a mesh would cross.
    NoDistance,
    /// Two bodies of one part made one: a mesh or a carrier would turn
    /// against itself.
    OneCard,
    /// Two bodies an axis distance apart made one: a shaft is straight.
    Apart,
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
            Self::NoDistance => "ui.train_edit_refused_no_distance",
            Self::OneCard => "ui.train_edit_refused_one_card",
            Self::Apart => "ui.train_edit_refused_apart",
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
            Self::NoDistance => "no axis distance joins those axes",
            Self::OneCard => "those bodies are on one card",
            Self::Apart => "those bodies are an axis distance apart",
        })
    }
}

impl Shape {
    /// **A card's edit on a shape asked alone** — the whole shape one card
    /// ([`Part::whole`]) — numbering any body it adds from `next`, the first
    /// number the train has free, upward.
    ///
    /// # Errors
    ///
    /// [`EditRefused`] where the edit would break an invariant, with the
    /// shape untouched.
    pub fn edit(&mut self, edit: StageEdit, next: usize) -> Result<(), EditRefused> {
        let whole = Part::whole(self);
        self.edit_part(&whole, edit, next)
    }

    /// **An edit a card asks of its part**, on the graph the part is of:
    /// the card's indices read through the part's maps into the graph's,
    /// the card's own refusals asked, and the graph's edit made
    /// ([`Self::apply`]). What it reads of its stage it reads among the
    /// card's gears — an axis removed from them alone, a shaft another
    /// part's gears are on staying with them; a gear alone among them
    /// already on a body of its own.
    ///
    /// # Errors
    ///
    /// As [`Self::edit`], and [`EditRefused::NoSuchIndex`] for an index
    /// the part does not have.
    pub fn edit_part(
        &mut self,
        part: &Part,
        edit: StageEdit,
        next: usize,
    ) -> Result<(), EditRefused> {
        let at = |v: &[usize], i: usize| v.get(i).copied().ok_or(EditRefused::NoSuchIndex);
        let member = |i: usize| at(&part.members, i);
        let graph = match edit {
            StageEdit::AddStep { axis } => Edit::AddStep {
                axis: at(&part.axes, axis)?,
            },
            StageEdit::RemoveStep { gear } => {
                let gear = member(gear)?;
                if !self.is_planet_gear(gear) {
                    return Err(EditRefused::WrongFamily);
                }
                if self.members_on_body(self.members[gear].body).len() < 2 {
                    return Err(EditRefused::LastOnItsStep);
                }
                Edit::Remove(Piece::Member(gear))
            }
            StageEdit::AddCentral { gear, ring } => {
                let gear = member(gear)?;
                let axis = self
                    .axis_of_slot(self.slot_of_member(gear))
                    .ok_or(EditRefused::NoSuchIndex)?;
                if !self.carried(axis) {
                    return Err(EditRefused::WrongFamily);
                }
                let central = self.central_axis_of(axis).ok_or(EditRefused::NoSuchIndex)?;
                Edit::AddGear {
                    mate: gear,
                    on: Place::NewBody(central),
                    ring,
                }
            }
            StageEdit::RemoveMember { member: i } => {
                let i = member(i)?;
                self.may_remove_member(i)?;
                Edit::Remove(Piece::Member(i))
            }
            StageEdit::AddAxis { mate } => Edit::AddGear {
                mate: member(mate)?,
                on: Place::NewAxis,
                ring: false,
            },
            StageEdit::RemoveAxis { axis } => {
                let axis = at(&part.axes, axis)?;
                let gears = self.may_clear_axis(axis, &part.members)?;
                return self.transact(|s| {
                    s.clear_axis(axis, gears);
                    Ok(())
                });
            }
            StageEdit::AddMesh { distance } => {
                let distance = at(&part.distances, distance)?;
                Edit::AddRatio {
                    distance,
                    shared: self.shared_body(distance)?,
                }
            }
            StageEdit::RemoveMesh { mesh } => {
                let mesh = at(&part.meshes, mesh)?;
                let d = self.distance_of(mesh).ok_or(EditRefused::NoSuchIndex)?;
                if self.meshes_on(d).len() < 2 {
                    return Err(EditRefused::LastOfItsKind);
                }
                let m = self.meshes[mesh];
                if self.is_planet_gear(m.a) || self.is_planet_gear(m.b) {
                    return Err(EditRefused::WrongFamily);
                }
                Edit::Remove(Piece::Mesh(mesh))
            }
            StageEdit::MoveBody { member: i, body } => {
                let i = member(i)?;
                // Alone among the card's gears on its body, it is on one of
                // its own already, whatever another part has there.
                let alone = self
                    .members_on_body(self.members[i].body)
                    .iter()
                    .filter(|j| part.members.contains(j))
                    .count()
                    == 1;
                if body.is_none() && alone {
                    return Ok(());
                }
                Edit::Move {
                    member: i,
                    to: body,
                }
            }
            StageEdit::Couple { body } => Edit::Couple { body },
            StageEdit::Uncouple { coupling } => {
                Edit::Remove(Piece::Coupling(at(&part.couplings, coupling)?))
            }
        };
        self.apply(&graph, next)
    }

    /// **One of the graph's edits on the shape** — every one but a join, a
    /// hold and an insert, which are the train's ([`super::Train::edit`]) —
    /// numbering any body it adds from `next`.
    ///
    /// # Errors
    ///
    /// [`EditRefused`], the shape untouched.
    pub(crate) fn apply(&mut self, edit: &Edit, next: usize) -> Result<(), EditRefused> {
        let all: Vec<usize> = (0..self.members.len()).collect();
        self.transact(|s| match *edit {
            Edit::AddGear { mate, on, ring } => s.add_gear(mate, on, ring, next),
            Edit::AddRatio { distance, shared } => s.add_ratio(distance, shared, next),
            Edit::AddStep { axis } => s.add_step(axis, next),
            Edit::Couple { body } => s.couple(body, next),
            Edit::Remove(piece) => s.remove(piece),
            Edit::Move { member, to } => s.move_body(member, to, &all, next),
            Edit::Join { .. } | Edit::Hold(_) | Edit::Release(_) | Edit::Insert { .. } => {
                Err(EditRefused::WrongFamily)
            }
        })
    }

    /// **An edit made whole or not at all**: on a copy, kept where it
    /// refuses nothing and leaves every planet that ran at a radius still
    /// running at one — a carried axis a gear is on meeting a gear on its
    /// carrier's axis, the mesh its radius is read from.
    fn transact(
        &mut self,
        edit: impl FnOnce(&mut Self) -> Result<(), EditRefused>,
    ) -> Result<(), EditRefused> {
        let mut s = self.clone();
        edit(&mut s)?;
        if self.planets_at_a_radius() && !s.planets_at_a_radius() {
            return Err(EditRefused::LastOnItsStep);
        }
        *self = s;
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
            return Err(EditRefused::WrongFamily);
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
    /// from its mate's: a copy of the mate — which at a chain's end is an
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
        if self.frame_of_member(mate) != GROUND {
            return Err(EditRefused::WrongFamily);
        }
        let axis = self.push_axis(GROUND, 1);
        let body = self.push_body(axis, next);
        if ring {
            let teeth = 2 * self.members[mate].gear.teeth;
            self.push_follower(mate, body, teeth, true);
        } else {
            self.members.push(Member {
                body,
                ring: None,
                ..self.members[mate].clone()
            });
            let new = self.members.len() - 1;
            self.push_mesh(mate, new);
        }
        self.push_distance([from, axis], 0.0);
        Ok(())
    }

    /// **The count a sun or a ring on planet gear `mate` takes**: sized to
    /// the radius its axis runs at, a few teeth of difference where nothing
    /// sets it yet — and on a body of its own (`fresh`), moved off a count
    /// that would turn as one with another.
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
        while fresh && taken(teeth) && teeth > floor {
            teeth -= 1.0;
        }
        while fresh && taken(teeth) {
            teeth += 1.0;
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
        let first = *self
            .meshes_on(distance)
            .first()
            .ok_or(EditRefused::NoDistance)?;
        let m = self.meshes[first];
        let on_axis = |i: usize| self.axis_of_slot(self.slot_of_member(i)) == Some(axis);
        let across = if on_axis(m.a) { m.a } else { m.b };
        if self.is_crossed(first) {
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
        let (shared, helix) = (self.shared(), self.helix_angles());
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

    /// A gear of `teeth` on `body` meshing `mate` and following it: its
    /// module, pressure angle and form the mate's, its shift and thickness
    /// automatic — a second central at one carrier radius is closed by its
    /// shift, which one given at zero could not do — a ring cut by the
    /// shape's first cutter.
    fn push_follower(&mut self, mate: usize, body: usize, teeth: u32, ring: bool) {
        let cutter = ring.then(|| self.members.iter().find_map(|m| m.ring).unwrap_or_default());
        let module = self.members[mate].normal_module();
        self.members.push(Member {
            body,
            gear: StageGear {
                teeth,
                profile_shift: Auto::automatic(0.0),
                ..self.members[mate].gear.clone()
            },
            module: Auto::automatic(module),
            pressure_angle: Auto::automatic(self.members[mate].normal_pressure_angle()),
            thickness_mod: Auto::automatic(1.0),
            ring: cutter,
            pitch_diameter: Auto::automatic(0.0),
        });
        let new = self.members.len() - 1;
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

    /// **The body a card's *add mesh* shares** — its guess, where the
    /// graph's edit asks: the one with the most members among the
    /// distance's meshes, where that is more than one — a layshaft — and
    /// else the first mesh's second gear's.
    fn shared_body(&self, distance: usize) -> Result<usize, EditRefused> {
        if distance >= self.distances.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        let first = *self
            .meshes_on(distance)
            .first()
            .ok_or(EditRefused::NoSuchIndex)?;
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
        Ok(on_distance
            .iter()
            .map(|&i| self.members[i].body)
            .max_by_key(|&b| count(b))
            .filter(|&b| count(b) > 1)
            .unwrap_or(self.members[self.meshes[first].b].body))
    }

    /// **A step on carried `axis`** ([`Edit::AddStep`]): the last gear on
    /// its planet body copied beside it, and a ring meshing the copy.
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
                let loose = [m.a, m.b]
                    .into_iter()
                    .filter(|&i| self.meshes_of_member(i).is_empty())
                    .collect();
                self.cascade(loose);
            }
            Piece::Axis(axis) => {
                if axis >= self.axes.len() {
                    return Err(EditRefused::NoSuchIndex);
                }
                if self.turns_a_carrier(axis, None) {
                    return Err(EditRefused::WrongFamily);
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
            going = (0..self.members.len())
                .filter(|&i| self.meshes_of_member(i).is_empty())
                .collect();
        }
    }

    /// **`gears` taken off `axis`**, with what goes with them: every body
    /// left on it with nothing on it, every distance left with no mesh, and
    /// the axis where nothing is left on it — the axes after it numbered
    /// down.
    fn clear_axis(&mut self, axis: usize, gears: Vec<usize>) {
        let before: Vec<usize> = self
            .bodies
            .iter()
            .filter(|b| b.axis == axis)
            .map(|b| b.body)
            .collect();
        self.cascade(gears);
        let bare: Vec<usize> = before
            .into_iter()
            .filter(|&b| self.members_on_body(b).is_empty() && !self.carries_an_axis(b))
            .collect();
        self.drop_bodies(&bare);
        self.tidy();
    }

    /// Bodies taken out by number, with every coupling they are in.
    fn drop_bodies(&mut self, bodies: &[usize]) {
        self.bodies.retain(|b| !bodies.contains(&b.body));
        self.couplings
            .retain(|c| !c.iter().any(|b| bodies.contains(b)));
    }

    /// Every distance left with no mesh on it, and every axis left with
    /// nothing on it, taken out.
    fn tidy(&mut self) {
        let meshless: Vec<usize> = (0..self.distances.len())
            .filter(|&d| self.meshes_on(d).is_empty())
            .collect();
        for &d in meshless.iter().rev() {
            self.distances.remove(d);
        }
        self.drop_empty_axes();
    }

    /// Whether a body on `axis` carries an axis — one of `among`'s gears
    /// is on, where given.
    fn turns_a_carrier(&self, axis: usize, among: Option<&[usize]>) -> bool {
        let axis_of = |i: usize| self.axis_of_slot(self.slot_of_member(i));
        self.axes.iter().enumerate().any(|(c, a)| {
            a.carried_by != GROUND
                && self.axis_of_body(a.carried_by) == Some(axis)
                && among.is_none_or(|g| g.iter().any(|&i| axis_of(i) == Some(c)))
        })
    }

    /// **What a card's *remove* asks of a central member**: not a planet
    /// gear, which is a step; not the last member meeting its planet gear —
    /// remove the step instead; and on a parallel chain not a distance's
    /// last mesh.
    fn may_remove_member(&self, member: usize) -> Result<(), EditRefused> {
        if member >= self.members.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        if self.is_planet_gear(member) {
            return Err(EditRefused::WrongFamily);
        }
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
        if !self.axes.iter().any(|a| a.carried_by != GROUND) {
            for k in self.meshes_of_member(member) {
                if let Some(d) = self.distance_of(k) {
                    if self.meshes_on(d).len() < 2 {
                        return Err(EditRefused::LastOfItsKind);
                    }
                }
            }
        }
        Ok(())
    }

    /// **What a card's *remove axis* asks**, and the card's gears on it:
    /// not a carried axis, nor one turning a carrier of the card's, and
    /// no gear left behind meshing nothing — the idler of a chain goes with
    /// the chain's end, not before it.
    fn may_clear_axis(&self, axis: usize, among: &[usize]) -> Result<Vec<usize>, EditRefused> {
        if axis >= self.axes.len() {
            return Err(EditRefused::NoSuchIndex);
        }
        if self.carried(axis) || self.turns_a_carrier(axis, Some(among)) {
            return Err(EditRefused::WrongFamily);
        }
        let on_axis: Vec<usize> = among
            .iter()
            .copied()
            .filter(|&i| self.axis_of_slot(self.slot_of_member(i)) == Some(axis))
            .collect();
        let stranded = (0..self.members.len())
            .filter(|i| !on_axis.contains(i))
            .any(|i| {
                let meshes = self.meshes_of_member(i);
                !meshes.is_empty() && meshes.iter().all(|&k| on_axis.contains(&self.mate(k, i)))
            });
        if stranded {
            return Err(EditRefused::LastOfItsKind);
        }
        Ok(on_axis)
    }

    // --------------------------------------------------------------- both ---

    /// A member moved, `among` the gears of the asking card: a gear alone
    /// among them on its body is on a body of its own already, whatever
    /// another part has on the same shaft.
    fn move_body(
        &mut self,
        member: usize,
        body: Option<usize>,
        among: &[usize],
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
            None if self
                .members_on_body(from)
                .iter()
                .filter(|i| among.contains(i))
                .count()
                == 1 =>
            {
                return Ok(())
            }
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
    /// was alone on it, the body carries no axis and no coupling turns it
    /// from a fixed axis — a shaft a planet's turn is taken off to stays
    /// with its coupling, while a planet body left with nothing on it goes
    /// with the coupling it turned.
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
        let coupled = self.couplings.iter().any(|c| c.contains(&body));
        let orbiting = self.axis_of_body(body).is_some_and(|a| self.carried(a));
        if self.members_on_body(body).is_empty()
            && !self.carries_an_axis(body)
            && (!coupled || orbiting)
        {
            self.bodies.retain(|b| b.body != body);
            self.couplings.retain(|c| !c.contains(&body));
        }
    }
}

/// **What every edit leaves**: nothing hanging — every gear on a body
/// on an axis and in a mesh, every mesh across a distance between two
/// axes, one distance per pair of axes and each carrying a mesh, no
/// axis with nothing on it, every carried axis carried by a body on
/// another, and every hold at a body the graph has.
#[cfg(test)]
pub(super) fn well_formed(t: &super::Train) -> Result<(), String> {
    let s = &t.shape;
    let axis_of = |body: usize| s.bodies.iter().find(|b| b.body == body).map(|b| b.axis);
    for b in &s.bodies {
        if b.axis >= s.axes.len() {
            return Err(format!("body {} on no axis", b.body));
        }
    }
    for (i, m) in s.members.iter().enumerate() {
        if axis_of(m.body).is_none() {
            return Err(format!("member {i} on no body"));
        }
        if !s.meshes.iter().any(|x| x.a == i || x.b == i) {
            return Err(format!("member {i} in no mesh"));
        }
    }
    for (k, m) in s.meshes.iter().enumerate() {
        if axis_of(s.members[m.a].body) == axis_of(s.members[m.b].body) {
            return Err(format!("mesh {k} on one axis"));
        }
        if s.distance_of(k).is_none() {
            return Err(format!("mesh {k} across no distance"));
        }
    }
    for (d, x) in s.distances.iter().enumerate() {
        if s.meshes_on(d).is_empty() {
            return Err(format!("distance {d} with no mesh"));
        }
        let same =
            |y: &super::shape::Distance| y.axes == x.axes || y.axes == [x.axes[1], x.axes[0]];
        if s.distances.iter().filter(|y| same(y)).count() > 1 {
            return Err(format!("distance {d} stated twice"));
        }
    }
    for (a, x) in s.axes.iter().enumerate() {
        if !s.bodies.iter().any(|b| b.axis == a) && !s.distances.iter().any(|d| d.axes.contains(&a))
        {
            return Err(format!("axis {a} with nothing on it"));
        }
        if x.carried_by != GROUND && axis_of(x.carried_by).is_none_or(|c| c == a) {
            return Err(format!("axis {a} carried by no body on another axis"));
        }
    }
    for &h in &t.held {
        if axis_of(h).is_none() {
            return Err(format!("a hold at {h}, which the graph has not"));
        }
    }
    if !s.planets_at_a_radius() {
        return Err("a planet meeting nothing on its carrier's axis".into());
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    //! **The laws an edit obeys**: it leaves a stage that solves, it undoes,
    //! it refuses whole, and what it renumbers the train follows.

    use super::super::arrangements::{self as arr, StagePreset};
    use super::super::{
        solve_train, test_library as library, LoadCase, Shape, StageBoundary, Train,
    };
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
        assert!(same(&t.stages()[0], &before.stages()[0]));
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
    /// is a port the train may hold, share or load, and it stays while
    /// anything names it — another part's gear on it, a hold, a case — even
    /// with nothing of this card's on it, which is what a gearbox in neutral
    /// is: its card has no end of the shaft until a gear is engaged on it
    /// again. Dropping it took the shaft and the case with it: a layshaft's
    /// output moved to an idler left the next stage joined to nothing.
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
        let shape = t.stages()[0].clone();
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
        // engaged gear onto the idler's body. The output keeps its number
        // and the spur's gear on it, and nothing of the layshaft's is on
        // it: neutral.
        t.edit_stage(
            0,
            StageEdit::MoveBody {
                member: engaged,
                body: Some(idler),
            },
        )
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
        assert_eq!(t.stages()[0].members_on_body(output), vec![idle]);
        assert_eq!(t.ends_of(output).len(), 2, "the output is the output");
        assert_eq!(t.port(0, 2), output, "...where it was");

        // **A bare body nothing names is given up**: the same first move on
        // a stage of its own, with no train to mean the shaft to be there.
        let mut alone = Train::chained(vec![lay()], |_| Vec::new());
        let was = alone.stages()[0].bodies.len();
        let (on, to) = {
            let stages = alone.stages();
            let s = &stages[0];
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
            alone.stages()[0].bodies.len(),
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
        assert_eq!(t.stages()[0].members.len(), 2);
        assert_eq!(t.port(0, 2), 2, "ring 2 closed up to body 2");
        assert_eq!(
            t.ends_of(2).len(),
            2,
            "ring 2 still runs on to the spur: {:?}",
            t.stages()[1].bodies
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

    /// A shape with its bodies numbered by its slots — how two shapes that
    /// number the same bodies differently are compared.
    fn by_slot(shape: &Shape) -> Shape {
        let listed = shape.clone();
        let mut out = shape.clone();
        out.renumber_bodies(|b| listed.slot(b));
        out
    }

    /// **An edit on a card is the edit on its stage.** Every add that
    /// applies to a preset, asked of the preset as the second card of a
    /// train — whose members, meshes, distances, axes and couplings are
    /// not the graph's by the same index — leaves that card the stage the
    /// same add leaves alone, and the first card as it was.
    #[test]
    fn an_edit_on_a_card_is_the_edit_on_its_stage() {
        for preset in StagePreset::ALL {
            let base = preset.build();
            for edit in applicable(&base) {
                let mut alone = base.clone();
                if self::edit(&mut alone, edit).is_err() {
                    continue;
                }
                let mut t = Train::chained(vec![StagePreset::Spur.build(), base.clone()], |_| {
                    Vec::new()
                });
                let first = by_slot(&t.stages()[0]);
                // A body is the train's number for it.
                let body = |b: usize| t.port(1, base.slot(b));
                let asked = match edit {
                    StageEdit::Couple { body: b } => StageEdit::Couple { body: body(b) },
                    StageEdit::MoveBody {
                        member,
                        body: Some(b),
                    } => StageEdit::MoveBody {
                        member,
                        body: Some(body(b)),
                    },
                    other => other,
                };
                t.edit_stage(1, asked)
                    .unwrap_or_else(|e| panic!("{preset:?} {edit:?}: {e}"));
                assert!(
                    same(&by_slot(&t.stages()[1]), &by_slot(&alone)),
                    "{preset:?} {edit:?}: {:?}\nvs {:?}",
                    by_slot(&t.stages()[1]),
                    by_slot(&alone)
                );
                assert!(
                    same(&by_slot(&t.stages()[0]), &first),
                    "{preset:?} {edit:?} touched the first card"
                );
            }
        }
    }

    /// **An axis removed from one card leaves another's gears on it.** Two
    /// pairs in a chain share a shaft; the second, grown to three axes,
    /// gives up the shared one — its gear there, that gear's mesh and the
    /// distance nothing is left on — and the first pair keeps its gear on
    /// the shaft, which stays. Removing every gear on the axis would strand
    /// the first pair's other gear, and be refused.
    #[test]
    fn an_axis_removed_from_one_card_leaves_anothers_gears_on_it() {
        let mut t = Train::chained(
            vec![StagePreset::Spur.build(), StagePreset::Spur.build()],
            |_| Vec::new(),
        );
        t.edit_stage(1, StageEdit::AddAxis { mate: 1 }).unwrap();
        let before = t.stages()[0].clone();
        let shared = t.port(0, 2);
        assert_eq!(t.ends_of(shared).len(), 2, "the pairs share a shaft");
        // The second card's axes, in the graph's order: the shared shaft first.
        let part = t.parts()[1].clone();
        let axis = t
            .shape
            .bodies
            .iter()
            .find(|b| b.body == shared)
            .unwrap()
            .axis;
        assert_eq!(part.axes[0], axis);
        t.edit_stage(1, StageEdit::RemoveAxis { axis: 0 }).unwrap();
        assert!(same(&t.stages()[0], &before), "the first pair is as it was");
        assert_eq!(t.ends_of(shared).len(), 1, "the shaft is the first pair's");
        assert_eq!(t.parts().len(), 2);
        assert_eq!(
            t.stages()[1].members.len(),
            2,
            "the second card's other two"
        );
        assert_eq!(t.stages()[1].distances.len(), 1);
        solve_train(&t, &library()).unwrap();
    }

    /// **A join that cannot be coaxial undoes, and one across an axis
    /// distance is refused.** The pair after an uncoupled planocentric is
    /// joined to its planet by a coupling; moving the pair's end to a body
    /// of its own takes the coupling away, and joining it again brings it
    /// back. Two ends on shafts an axis distance apart are no one body —
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
        let mut t = Train::chained(vec![uncoupled, StagePreset::Spur.build()], |_| Vec::new());
        let (planet, end) = (3, t.port(1, 1));
        assert_eq!(t.shape.couplings, vec![[planet, end]]);
        t.move_end(1, end, None);
        assert!(t.shape.couplings.is_empty(), "the end is its own");
        assert_eq!(t.parts().len(), 2);
        t.move_end(1, end, Some(planet));
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
        let pair = || StagePreset::Spur.build();
        let mut t = Train::chained(vec![pair(), pair(), pair()], |_| Vec::new());
        let a = t.port(0, 2);
        t.split(1, a);
        let b = t.split(2, t.port(1, 2));
        assert_eq!((t.ends_of(a).len(), t.ends_of(b).len()), (1, 1));
        let before = t.clone();
        t.join(a, b);
        assert_eq!(format!("{t:?}"), format!("{before:?}"), "refused");
    }

    // ------------------------------------------------ the graph's edits ---

    /// Every preset alone, and as a train's second card — whose members,
    /// meshes, distances and axes are not the graph's by the same index.
    fn trains() -> Vec<(String, Train)> {
        let mut out = Vec::new();
        for p in StagePreset::ALL {
            out.push((
                format!("{p:?}"),
                Train::chained(vec![p.build()], |_| Vec::new()),
            ));
            out.push((
                format!("spur then {p:?}"),
                Train::chained(vec![StagePreset::Spur.build(), p.build()], |_| Vec::new()),
            ));
        }
        out
    }

    fn debug(t: &Train) -> String {
        format!("{t:?}")
    }

    /// **Every gear the graph admits is refused whole, or undoes.** On
    /// every preset alone and as a second card, a gear meshing every
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
                        well_formed(&u).unwrap_or_else(|e| panic!("{name}: {edit:?}: {e}"));
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
        let s = &u.shape;
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
    /// every member, mesh, axis, body and coupling of every preset alone
    /// and as a second card, taken out — refused, the train as it was;
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
                well_formed(&u).unwrap_or_else(|e| panic!("{name}: {piece:?}: {e}"));
                let _ = solve_train(&u, &lib);
            }
        }
        assert!(made > 100, "only {made} removals made");
    }

    /// **A join is one body on one axis, or says why.** Two pairs apart —
    /// four axes, two parts — the first's output joined to the second's
    /// input: one body, three axes, a chain of two at the product of their
    /// ratios. A card's two bodies are refused (`OneCard`), as are ground
    /// and a body the train has not; each refusal changes nothing.
    #[test]
    fn a_join_is_one_body_on_one_axis_or_says_why() {
        let pair = StagePreset::Spur.build();
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
        well_formed(&u).unwrap();
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
            (1, 2, EditRefused::OneCard),
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
        let mut t = Train::chained(vec![StagePreset::Spur.build()], |_| Vec::new());
        t.edit(Edit::Insert {
            stage: StagePreset::Planetary.build(),
            at: Some(1),
        })
        .unwrap();
        well_formed(&t).unwrap();
        assert_eq!(t.parts().len(), 2);
        assert_eq!(t.ends_of(1).len(), 2, "the input shaft carries both");
        solve_train(&t, &library()).unwrap();
        let before = t.clone();
        assert_eq!(
            t.edit(Edit::Insert {
                stage: StagePreset::Spur.build(),
                at: Some(99),
            }),
            Err(EditRefused::NoSuchIndex)
        );
        assert_eq!(debug(&t), debug(&before));
    }

    /// **A ratio goes on the body asked.** A layshaft's next ratio shares
    /// the body the edit names — the layshaft, or the input shaft — which
    /// a card's *add mesh* could only guess; a body on neither of the
    /// distance's axes is refused.
    #[test]
    fn a_ratio_goes_on_the_body_asked() {
        let t = Train::chained(vec![StagePreset::Layshaft.build()], |_| Vec::new());
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
            well_formed(&u).unwrap_or_else(|e| panic!("sharing {shared}: {e}"));
            let n = u.shape.members.len();
            assert_eq!(u.shape.members[n - 2].body, shared, "on the body asked");
        }
        let elsewhere = s.bodies.iter().map(|b| b.body).find(|&b| {
            let axis = s.bodies.iter().find(|x| x.body == b).unwrap().axis;
            !s.distances[0].axes.contains(&axis)
        });
        if let Some(b) = elsewhere {
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
}
