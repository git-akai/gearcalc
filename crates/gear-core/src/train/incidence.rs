//! **What meets what in a shape, read once**: each member's meshes, each
//! mesh's axis distance and each distance's meshes, each body's members,
//! and the distance between two axes — the lookups every reading of the
//! graph makes, which were each a scan of the shape's lists at every call,
//! a distance's meshes a scan of the meshes each scanning the distances
//! (`work/design-graph.md`, *The stored representation*).
//!
//! Built once for a shape that does not change while it is read
//! ([`Shape::indexed`]): a solve, a part's, an edit's reading of what it
//! edits. Derived, never stored — a shape's lists are its only state, and
//! an index kept beside them would outlive the first edit.
//!
//! Every answer is the one the scan it replaces gave, on any input whose
//! indices are in range, well formed or not: the first of two distances
//! stated for one pair of axes, a body listed twice at its first listing,
//! a member on a body the shape does not list at no axis. The law holding
//! it to the scans is below.

use super::shape::Shape;
use crate::kinematics::Body;
use std::collections::HashMap;

/// **A shape's incidence** — see [the module](self).
#[derive(Clone, Debug)]
pub(crate) struct Incidence {
    /// Each body number's members, in member order — listed or not.
    body_members: HashMap<usize, Vec<usize>>,
    /// Each member's meshes, in mesh order.
    member_meshes: Vec<Vec<usize>>,
    /// Each mesh's axis distance.
    mesh_distance: Vec<Option<usize>>,
    /// Each distance's meshes, in mesh order.
    distance_meshes: Vec<Vec<usize>>,
    /// The first distance stated between two axes, by the pair both ways
    /// round.
    between: HashMap<(usize, usize), usize>,
}

impl Incidence {
    /// **The incidence of `shape`**, in one pass over each of its lists.
    /// Keyed by what a list holds rather than sized by it, so a number out
    /// of range reads as the scans read it — nothing there — rather than
    /// as a table that large.
    pub(crate) fn of(shape: &Shape) -> Self {
        #[cfg(test)]
        crate::testing::work::incidence();
        let mut slot_of_body: HashMap<usize, Body> = HashMap::new();
        for (i, b) in shape.bodies.iter().enumerate() {
            slot_of_body.entry(b.body).or_insert(i + 1);
        }
        let member_axis: Vec<Option<usize>> = shape
            .members
            .iter()
            .map(|m| slot_of_body.get(&m.body).map(|&s| shape.bodies[s - 1].axis))
            .collect();
        let mut body_members: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, m) in shape.members.iter().enumerate() {
            body_members.entry(m.body).or_default().push(i);
        }
        // A mesh is each of its members' once, a mesh of a member with
        // itself too.
        let mut member_meshes: Vec<Vec<usize>> = vec![Vec::new(); shape.members.len()];
        for (k, m) in shape.meshes.iter().enumerate() {
            let ends = if m.a == m.b {
                vec![m.a]
            } else {
                vec![m.a, m.b]
            };
            for end in ends {
                if let Some(list) = member_meshes.get_mut(end) {
                    list.push(k);
                }
            }
        }
        let mut between: HashMap<(usize, usize), usize> = HashMap::new();
        for (d, x) in shape.distances.iter().enumerate() {
            let [a, b] = x.axes;
            between.entry((a, b)).or_insert(d);
            between.entry((b, a)).or_insert(d);
        }
        let axis = |i: usize| member_axis.get(i).copied().flatten();
        let mesh_distance: Vec<Option<usize>> = shape
            .meshes
            .iter()
            .map(|m| between.get(&(axis(m.a)?, axis(m.b)?)).copied())
            .collect();
        let mut distance_meshes: Vec<Vec<usize>> = vec![Vec::new(); shape.distances.len()];
        for (k, d) in mesh_distance.iter().enumerate() {
            if let Some(d) = *d {
                distance_meshes[d].push(k);
            }
        }
        Self {
            body_members,
            member_meshes,
            mesh_distance,
            distance_meshes,
            between,
        }
    }

    /// A member's meshes, in mesh order.
    pub(crate) fn meshes_of(&self, member: usize) -> &[usize] {
        &self.member_meshes[member]
    }

    /// The members on a body, in member order.
    pub(crate) fn members_on(&self, body: usize) -> &[usize] {
        self.body_members.get(&body).map_or(&[], Vec::as_slice) // absence: a body nothing is on has no members
    }

    /// The distance a mesh runs at — the first stated for its two axes,
    /// either way round.
    pub(crate) fn distance_of(&self, mesh: usize) -> Option<usize> {
        self.mesh_distance[mesh]
    }

    /// The meshes on one distance, in mesh order.
    pub(crate) fn meshes_on(&self, distance: usize) -> &[usize] {
        &self.distance_meshes[distance]
    }

    /// The distance between two axes, the first stated for the pair.
    pub(crate) fn between(&self, a: usize, b: usize) -> Option<usize> {
        self.between.get(&(a, b)).copied()
    }
}

/// **A shape read with its incidence** — what a solve, a part and an edit's
/// reading of what it edits read the shape through: the shape's own lists
/// and methods as they are ([`std::ops::Deref`]), and every question of
/// what meets what answered by the one [`Incidence`] built with it.
pub(crate) struct Indexed<'a> {
    shape: &'a Shape,
    pub(crate) at: std::borrow::Cow<'a, Incidence>,
}

impl std::ops::Deref for Indexed<'_> {
    type Target = Shape;
    fn deref(&self) -> &Shape {
        self.shape
    }
}

impl Shape {
    /// **This shape with its incidence built** ([`Indexed`]), once for as
    /// long as it is read unchanged.
    pub(crate) fn indexed(&self) -> Indexed<'_> {
        Indexed {
            shape: self,
            at: std::borrow::Cow::Owned(Incidence::of(self)),
        }
    }
}

impl<'a> Indexed<'a> {
    /// `shape` read through an incidence already built of it — a cut's,
    /// which its rating reads again.
    pub(crate) fn over(shape: &'a Shape, at: &'a Incidence) -> Self {
        Self {
            shape,
            at: std::borrow::Cow::Borrowed(at),
        }
    }

    /// The shape itself, for what reads it as it is.
    pub(crate) fn shape(&self) -> &Shape {
        self.shape
    }

    /// The distance a mesh runs at — the first stated for its two axes,
    /// either way round.
    pub(crate) fn distance_of(&self, mesh: usize) -> Option<usize> {
        self.at.distance_of(mesh)
    }

    /// The meshes on one distance, in order.
    pub(crate) fn meshes_on(&self, distance: usize) -> &[usize] {
        self.at.meshes_on(distance)
    }

    /// A member's meshes, in order.
    pub(crate) fn meshes_of(&self, member: usize) -> &[usize] {
        self.at.meshes_of(member)
    }

    /// The members on a body, in order.
    pub(crate) fn members_on(&self, body: usize) -> &[usize] {
        self.at.members_on(body)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    //! **The incidence is the scans it replaces**, on every train the seeded
    //! walk visits and every part of each, and on shapes no edit makes: two
    //! distances for one pair of axes, a body listed twice, a member on a
    //! body the shape does not list, a mesh of a member with itself.

    use super::super::arrangements::Preset;
    use super::super::shape::BodyOn;
    use super::super::sweep;
    use super::*;

    /// The scans, as they were: each a walk over the shape's lists.
    mod scan {
        use super::super::super::shape::Shape;
        use crate::kinematics::{Body, GROUND};

        pub fn slot_if_any(s: &Shape, body: usize) -> Option<Body> {
            s.bodies.iter().position(|b| b.body == body).map(|i| i + 1)
        }

        pub fn axis_of_member(s: &Shape, member: usize) -> Option<usize> {
            let slot = slot_if_any(s, s.members[member].body).unwrap_or(GROUND);
            (slot != GROUND).then(|| s.bodies[slot - 1].axis)
        }

        pub fn distance_of(s: &Shape, mesh: usize) -> Option<usize> {
            let m = s.meshes[mesh];
            let (a, b) = (axis_of_member(s, m.a)?, axis_of_member(s, m.b)?);
            s.distances
                .iter()
                .position(|d| d.axes == [a, b] || d.axes == [b, a])
        }

        pub fn meshes_on(s: &Shape, distance: usize) -> Vec<usize> {
            (0..s.meshes.len())
                .filter(|&m| distance_of(s, m) == Some(distance))
                .collect()
        }

        pub fn meshes_of(s: &Shape, member: usize) -> Vec<usize> {
            (0..s.meshes.len())
                .filter(|&k| s.meshes[k].a == member || s.meshes[k].b == member)
                .collect()
        }

        pub fn members_on(s: &Shape, body: usize) -> Vec<usize> {
            (0..s.members.len())
                .filter(|&i| s.members[i].body == body)
                .collect()
        }

        pub fn between(s: &Shape, a: usize, b: usize) -> Option<usize> {
            s.distances
                .iter()
                .position(|d| d.axes == [a, b] || d.axes == [b, a])
        }
    }

    /// Every answer the incidence gives against its scan; how many compared.
    fn agrees(name: &str, s: &Shape) -> usize {
        let at = Incidence::of(s);
        let mut compared = 0;
        for i in 0..s.members.len() {
            assert_eq!(at.meshes_of(i), scan::meshes_of(s, i), "{name}: member {i}");
            compared += 1;
        }
        let most = s
            .bodies
            .iter()
            .map(|b| b.body)
            .chain(s.members.iter().map(|m| m.body));
        for body in 0..=most.max().unwrap_or_default() + 1 {
            assert_eq!(
                at.members_on(body),
                scan::members_on(s, body),
                "{name}: body {body}"
            );
            compared += 1;
        }
        for k in 0..s.meshes.len() {
            assert_eq!(
                at.distance_of(k),
                scan::distance_of(s, k),
                "{name}: mesh {k}"
            );
            compared += 1;
        }
        for d in 0..s.distances.len() {
            assert_eq!(
                at.meshes_on(d),
                scan::meshes_on(s, d),
                "{name}: distance {d}"
            );
            compared += 1;
        }
        for a in 0..s.axes.len() {
            for b in 0..s.axes.len() {
                assert_eq!(at.between(a, b), scan::between(s, a, b), "{name}: {a}-{b}");
                compared += 1;
            }
        }
        compared
    }

    #[test]
    fn the_incidence_is_the_scans_on_every_walked_train() {
        let (mut shapes, mut compared) = (0, 0);
        for (walk, steps, t) in sweep::visited() {
            let name = format!("{walk}: {steps:?}");
            compared += agrees(&name, &t.shape);
            for (p, part) in t.parts().iter().enumerate() {
                compared += agrees(&format!("{name}, part {p}"), &part.shape);
                shapes += 1;
            }
            shapes += 1;
        }
        assert!(shapes > 1500, "only {shapes} shapes");
        assert!(compared > 50_000, "only {compared} answers compared");
    }

    /// **One incidence per part solved**: a cut builds one, its rating
    /// builds none — it reads the cut's — and every candidate its search
    /// scores reads the cut's too: a search on builds no more than the same
    /// solve with it off, however many candidates it scores.
    #[test]
    fn a_part_is_solved_through_one_incidence() {
        use crate::testing::work;
        let lib = super::super::test_library();
        let mut searched = 0;
        for (name, shape) in sweep::arrangements() {
            let (built, cut) = work::of(|| super::super::shape::cut(&shape, &lib));
            assert_eq!(
                built.incidences, 1,
                "{name}: a cut built {}",
                built.incidences
            );
            let cut = cut.unwrap_or_else(|e| panic!("{name}: {e:?}"));
            let loads = [super::super::CaseLoad::nothing(
                0,
                super::super::CaseKind::Ultimate,
                &shape.shared().wiring(),
            )];
            let reversal = super::super::Reversal { correct: false };
            let (rated, _) = work::of(|| super::super::shape::rate(&cut, &loads, reversal));
            assert_eq!(
                rated.incidences, 0,
                "{name}: a rating built {}",
                rated.incidences
            );
            let solve = |search: bool| {
                let mut s = shape.clone();
                s.set_search(search);
                let t = super::super::Train::alone(&s, 2.0, 3000.0);
                work::of(|| super::super::solve_train(&t, &lib).is_ok())
            };
            let ((off, solved_off), (on, solved_on)) = (solve(false), solve(true));
            assert!(solved_off && solved_on, "{name}");
            assert_eq!(
                on.incidences, off.incidences,
                "{name}: the search built its own"
            );
            searched += usize::from(on.evaluations > 0);
        }
        assert!(searched > 5, "only {searched} searched");
    }

    /// **And on what no edit makes**, where the first of two answers is the
    /// scan's: each fixture is one that a near miss — the last distance
    /// stated rather than the first, a body's last listing, a member off
    /// the list read as on axis 0, a mesh with itself listed twice — gets
    /// wrong.
    #[test]
    fn the_incidence_is_the_scans_on_shapes_no_edit_makes() {
        let mut twice = Preset::Layshaft.build();
        let again = twice.distances[0];
        twice.distances.push(again);
        let mut reversed = Preset::Planetary.build();
        let turned = super::super::shape::Distance {
            axes: [reversed.distances[0].axes[1], reversed.distances[0].axes[0]],
            ..reversed.distances[0]
        };
        reversed.distances.insert(0, turned);
        let mut listed_twice = Preset::Spur.build();
        let first = listed_twice.bodies[0];
        listed_twice.bodies.push(BodyOn {
            axis: listed_twice.bodies[1].axis,
            ..first
        });
        let mut unlisted = Preset::Idler.build();
        unlisted.members[1].body = unlisted.max_body() + 3;
        let mut itself = Preset::Spur.build();
        itself.meshes.push(super::super::shape::MeshInput {
            b: 0,
            ..itself.meshes[0]
        });
        for (name, s) in [
            ("a distance twice", twice),
            ("a distance twice, the other way round", reversed),
            ("a body listed twice", listed_twice),
            ("a member on no listed body", unlisted),
            ("a mesh of a member with itself", itself),
        ] {
            assert!(agrees(name, &s) > 0);
        }
    }
}
