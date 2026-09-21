//! **The arrangements the one shape makes free**, each written as the list
//! of what sits where and nothing else — no solve, no kinematics, no model.
//!
//! A spur pair and a planetary set are the presets a panel offers
//! ([`super::PairStage`], [`super::PlanetaryStage`]); these are the ones the
//! shape reaches with no code of their own, kept here so the harness
//! (`gear-cli kinematics`) and the suite can name them, and so a future
//! preset is a function of tooth counts rather than a type of its own. Each is a
//! textbook arrangement, and the test beside each holds the ratio the
//! textbook gives.
//!
//! [`Builder`] is the whole vocabulary: an axis, carried or not and
//! replicated or not; a shaft on an axis; a gear or a ring on a shaft; a
//! mesh between two members; a distance between two axes. Shafts are
//! numbered as the wiring numbers them, ground being 0, so the numbers a
//! builder hands back are the ones a train's constraints address.

use super::shape::{Axis, Distance, Member, MeshInput, ShaftOn, Shape};
use super::StageGear;
use crate::kinematics::Shaft;
use crate::params::Auto;
use crate::ring::Cutter;

/// Sliding and static friction every mesh here runs with — the set's.
const FRICTION: (f64, f64) = (0.08, 0.16);

/// A shape assembled a piece at a time, at one module and one pressure
/// angle, every distance automatic and every clearance the shipped 0.02 mm.
#[derive(Clone, Debug)]
pub struct Builder {
    shape: Shape,
    module: f64,
}

impl Builder {
    /// An empty shape at this module, 20° pressure angle.
    #[must_use]
    pub fn new(module: f64) -> Self {
        let set = super::PlanetaryStage::default();
        Self {
            shape: Shape {
                pressure_angle: set.pressure_angle,
                overlap: set.overlap,
                optimisation: set.optimisation,
                load_sharing: set.load_sharing,
                min_planet_clearance: set.min_planet_clearance,
                axes: Vec::new(),
                shafts: Vec::new(),
                members: Vec::new(),
                meshes: Vec::new(),
                distances: Vec::new(),
            },
            module,
        }
    }

    /// An axis fixed in ground.
    pub fn axis(&mut self) -> usize {
        self.carried_axis(None, 1)
    }

    /// An axis carried round by `carrier`, `count` times.
    pub fn carried_axis(&mut self, carrier: Option<Shaft>, count: u32) -> usize {
        self.shape.axes.push(Axis {
            carried_by: carrier,
            count,
        });
        self.shape.axes.len() - 1
    }

    /// A shaft on an axis, numbered as the wiring numbers it.
    pub fn shaft(&mut self, axis: usize) -> Shaft {
        self.shape.shafts.push(ShaftOn { axis });
        self.shape.shafts.len()
    }

    /// An external gear on a shaft; the member's index.
    pub fn gear(&mut self, shaft: Shaft, teeth: u32) -> usize {
        self.member(shaft, teeth, None)
    }

    /// A ring on a shaft, cut by the shipped cutter, its shift automatic
    /// like every other member's here — a second ring at one carrier radius
    /// is closed by its shift, which a ring given at zero could not do; the
    /// member's index.
    pub fn ring(&mut self, shaft: Shaft, teeth: u32) -> usize {
        self.member(shaft, teeth, Some(Cutter::default()))
    }

    fn member(&mut self, shaft: Shaft, teeth: u32, ring: Option<Cutter>) -> usize {
        self.shape.members.push(Member {
            shaft,
            gear: StageGear {
                teeth,
                ..StageGear::default()
            },
            module: self.module,
            thickness_mod: Auto::automatic(1.0),
            ring,
            pitch_diameter: Auto::automatic(0.0),
        });
        self.shape.members.len() - 1
    }

    /// Two members in mesh. On an internal mesh the ring goes second, as
    /// [`MeshInput`] has it.
    pub fn mesh(&mut self, a: usize, b: usize) -> &mut Self {
        let (a, b) = if self.shape.members[a].ring.is_some() {
            (b, a)
        } else {
            (a, b)
        };
        self.shape.meshes.push(MeshInput {
            a,
            b,
            sliding_friction: FRICTION.0,
            static_friction: FRICTION.1,
        });
        self
    }

    /// An automatic distance between two axes, at the shipped clearance.
    pub fn distance(&mut self, axes: [usize; 2]) -> &mut Self {
        self.crossed(axes, 0.0)
    }

    /// An automatic distance between two axes at a shaft angle, degrees —
    /// the point contact of crossed shafts where the angle is not nought.
    pub fn crossed(&mut self, axes: [usize; 2], angle: f64) -> &mut Self {
        self.shape.distances.push(Distance {
            axes,
            angle,
            worm: false,
            distance: Auto::automatic(0.0),
            clearance: Auto::fixed(0.02),
            tip_clearance: 0.0,
            tolerance_plus: 0.02,
            tolerance_minus: 0.02,
            axial_clearance: 0.0,
        });
        self
    }

    #[must_use]
    pub fn build(self) -> Shape {
        self.shape
    }
}

/// **A layshaft transmission**: an input shaft and an output shaft on one
/// centreline, a layshaft beside them, and a pair of gears per ratio at the
/// one distance between the two axes. The input's gear drives the layshaft;
/// the engaged pair drives the output shaft; every other pair's output-side
/// gear idles on a shaft of its own, coaxial with the output. `pairs` are
/// `(on the layshaft, on the output side)` in order, `engaged` the index of
/// the one driving; the first pair is the input's constant mesh.
///
/// Shafts: input 1, output 2, layshaft 3, then one idler per disengaged
/// pair — the output before the layshaft so that the convention's "first
/// free port after the input" is the output and not the layshaft.
#[must_use]
pub fn layshaft(input: (u32, u32), pairs: &[(u32, u32)], engaged: usize) -> Shape {
    let mut b = Builder::new(1.0);
    let centre = b.axis();
    let lay = b.axis();
    let input_shaft = b.shaft(centre);
    let output = b.shaft(centre);
    let layshaft = b.shaft(lay);
    let driving = b.gear(input_shaft, input.0);
    let driven = b.gear(layshaft, input.1);
    b.mesh(driving, driven);
    for (i, &(on_lay, on_out)) in pairs.iter().enumerate() {
        let shaft = if i == engaged {
            output
        } else {
            b.shaft(centre)
        };
        let a = b.gear(layshaft, on_lay);
        let z = b.gear(shaft, on_out);
        b.mesh(a, z);
    }
    b.distance([centre, lay]);
    b.build()
}

/// **A member on the central axis of an epicyclic stage**, or the carrier —
/// what sits on the axis the planets go round, in the order it is listed.
///
/// The order is the shaft order, and the shaft order is what a stage's
/// conventions read (`Shape::ports`): the first ring listed is held, the
/// first shaft not held is the input and the next the output. So a list
/// is an arrangement *and* the way it is conventionally used, and every
/// textbook arrangement below is one list with nothing else stated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Central {
    /// The carrier, on its own shaft. One per stage.
    Carrier,
    /// An external gear meshing the planet gear `on` (an index into the
    /// planet gears as [`epicyclic`] flattens them, axis by axis).
    Sun { on: usize, teeth: u32 },
    /// A ring meshing the planet gear `on`.
    Ring { on: usize, teeth: u32 },
}

/// **The one epicyclic stage**: a carrier, `planets` carried axes each
/// replicated `count` times and each carrying the gears it lists (a step
/// each, on one shaft — a negative count a ring, the crate's own sign for
/// one), the central members and the carrier in [`Central`]'s order, and
/// `planet_meshes` between planet gears on different axes. A simple set, a
/// Wolfrom, a stepped planet, a planocentric, meshed planets and a
/// Ravigneaux are lists; so is a hula stage, at one planet with two steps
/// and a ring on each.
///
/// Shafts: the centrals in list order (the carrier among them), then one
/// per carried axis. Members: the centrals in list order, then the planet
/// gears axis by axis — so the first ring listed is `Ring 1` and the one
/// held by convention, which is how the textbook reads it.
///
/// # Panics
///
/// On a list with no carrier, or a central on a planet gear that is not
/// there — a mistake in a fixture, not an input a designer can write.
#[must_use]
pub fn epicyclic(
    count: u32,
    planets: &[&[i32]],
    centrals: &[Central],
    planet_meshes: &[(usize, usize)],
) -> Shape {
    let mut b = Builder::new(1.0);
    let centre = b.axis();
    // Every central shaft first, in the order listed; the carrier's number
    // is what the carried axes are hung from.
    let shafts: Vec<Shaft> = centrals.iter().map(|_| b.shaft(centre)).collect();
    let carrier = centrals
        .iter()
        .position(|c| *c == Central::Carrier)
        .map(|i| shafts[i])
        .expect("an epicyclic stage has a carrier");
    let axes: Vec<usize> = planets
        .iter()
        .map(|_| b.carried_axis(Some(carrier), count))
        .collect();
    let planet_shafts: Vec<Shaft> = axes.iter().map(|&a| b.shaft(a)).collect();
    // Centrals are members before the planets are, so the ordinals a name
    // carries follow the list.
    let central_members: Vec<Option<usize>> = centrals
        .iter()
        .zip(&shafts)
        .map(|(c, &shaft)| match *c {
            Central::Carrier => None,
            Central::Sun { teeth, .. } => Some(b.gear(shaft, teeth)),
            Central::Ring { teeth, .. } => Some(b.ring(shaft, teeth)),
        })
        .collect();
    let mut planet_members: Vec<usize> = Vec::new();
    let mut axis_of_planet: Vec<usize> = Vec::new();
    for (k, steps) in planets.iter().enumerate() {
        for &teeth in steps.iter() {
            let member = if teeth < 0 {
                b.ring(planet_shafts[k], teeth.unsigned_abs())
            } else {
                b.gear(planet_shafts[k], teeth.unsigned_abs())
            };
            planet_members.push(member);
            axis_of_planet.push(axes[k]);
        }
    }
    for (c, m) in centrals.iter().zip(&central_members) {
        if let (Central::Sun { on, .. } | Central::Ring { on, .. }, Some(m)) = (*c, *m) {
            b.mesh(m, planet_members[on]);
        }
    }
    for &(p, q) in planet_meshes {
        b.mesh(planet_members[p], planet_members[q]);
    }
    for &axis in &axes {
        b.distance([centre, axis]);
    }
    let mut between: Vec<[usize; 2]> = Vec::new();
    for &(p, q) in planet_meshes {
        let pair = [axis_of_planet[p], axis_of_planet[q]];
        if pair[0] != pair[1] && !between.contains(&pair) && !between.contains(&[pair[1], pair[0]])
        {
            between.push(pair);
            b.distance(pair);
        }
    }
    b.build()
}

/// **A line of gears on parallel axes**, each meshing the next: two are a
/// pair, three a pair with an idler, and the ratio is the ends' whatever
/// stands between. Shafts and members in the order given.
///
/// # Panics
///
/// On fewer than two gears.
#[must_use]
pub fn line(teeth: &[u32]) -> Shape {
    assert!(teeth.len() >= 2, "a line of gears is at least a pair");
    let mut b = Builder::new(1.0);
    let members: Vec<usize> = teeth
        .iter()
        .map(|&z| {
            let axis = b.axis();
            let shaft = b.shaft(axis);
            b.gear(shaft, z)
        })
        .collect();
    for k in 1..members.len() {
        b.mesh(members[k - 1], members[k]).distance([k - 1, k]);
    }
    b.build()
}

/// A planet gear's count as [`epicyclic`] takes it: external, so positive.
fn external(teeth: u32) -> i32 {
    i32::try_from(teeth).unwrap_or(i32::MAX)
}

/// **A hula stage**: a stepped Wolfrom at one planet, on a crank — the
/// grounded gear, the two that ride the wobble body, the output, in that
/// order, each mesh internal with the larger of its pair the ring — at the
/// proportions the family runs at: teeth cut to 0.7 of a module over a 1.0
/// dedendum (a taller tooth reaches past the interference limit at the
/// operating angles these differences run at, and costs efficiency on the
/// way — docs/reference.md#the-hula-stage), a twenty-tooth shaper cutting
/// to that depth, and a far-side gap of 0.3 mm the tips size the crank
/// offset by, since a fifth leaves the tip circles crossing where they
/// meet. Each mesh's module is its own; the pinion of each states the
/// thickness coefficient and its ring follows.
///
/// Carrier, grounded gear, output, then the wobble body: the grounded gear
/// held by convention, the crank in, the output out.
#[must_use]
pub fn hula(teeth: [u32; 4], module: [f64; 2]) -> Shape {
    let pair = |mesh: usize| {
        let (central, wobble) = (teeth[mesh * 3], teeth[mesh + 1]);
        let central_is_ring = central > wobble;
        let on = mesh;
        let z = central;
        let c = if central_is_ring {
            Central::Ring { on, teeth: z }
        } else {
            Central::Sun { on, teeth: z }
        };
        let w = external(wobble);
        (c, if central_is_ring { w } else { -w })
    };
    let (c0, w0) = pair(0);
    let (c1, w1) = pair(1);
    let mut shape = epicyclic(1, &[&[w0, w1]], &[Central::Carrier, c0, c1], &[]);
    shape.optimisation.min_contact_ratio = 1.0;
    shape.min_planet_clearance = 0.0;
    shape.distances[0].tip_clearance = 0.3;
    // Members: the two centrals, then the two wobble gears; each mesh is a
    // central and the wobble gear of the same index.
    for (i, m) in shape.members.iter_mut().enumerate() {
        let mesh = i % 2;
        m.module = module[mesh];
        m.gear.addendum = 0.7;
        m.gear.dedendum = 1.0;
        m.thickness_mod = if m.ring.is_some() {
            Auto::automatic(1.0)
        } else {
            Auto::fixed(1.0)
        };
        if let Some(cutter) = &mut m.ring {
            *cutter = Cutter {
                teeth: 20,
                addendum: 1.0,
                ..Cutter::default()
            };
        }
    }
    shape
}

/// **A Wolfrom (3K) set**: one planet meshing two rings at one carrier
/// radius, one ring held, the other the output, the carrier the input. No
/// sun; the ratio is `z_r2 / (z_r2 − z_r1)` at one planet, which is what
/// makes a difference of a few teeth a large reduction.
///
/// Carrier, first ring, second ring — the first ring held by convention,
/// the carrier in and the second ring out with nothing stated.
#[must_use]
pub fn wolfrom(planet: u32, rings: [u32; 2], count: u32) -> Shape {
    epicyclic(
        count,
        &[&[external(planet)]],
        &[
            Central::Carrier,
            Central::Ring {
                on: 0,
                teeth: rings[0],
            },
            Central::Ring {
                on: 0,
                teeth: rings[1],
            },
        ],
        &[],
    )
}

/// **A stepped-planet set**: two gears on each planet shaft, the first
/// meshing the sun and the first ring, the second the second ring — a
/// compound set at one carrier radius, which the second ring's shift
/// closes.
///
/// Sun, first ring, second ring, carrier: the first ring held by
/// convention, the sun in and the second ring out — the compound
/// reduction; a hold on the carrier or a drive elsewhere gives the others.
#[must_use]
pub fn stepped(sun: u32, planets: [u32; 2], rings: [u32; 2], count: u32) -> Shape {
    epicyclic(
        count,
        &[&planets.map(external)],
        &[
            Central::Sun { on: 0, teeth: sun },
            Central::Ring {
                on: 0,
                teeth: rings[0],
            },
            Central::Ring {
                on: 1,
                teeth: rings[1],
            },
            Central::Carrier,
        ],
        &[],
    )
}

/// **A planocentric (cycloid-style involute) reducer**: one planet on an
/// eccentric carrier meshing one ring a tooth or two larger; the ring held,
/// the carrier the input, the planet's own rotation the output. The ratio
/// is `−z_p / (z_r − z_p)`.
///
/// Carrier, ring, then the planet's own shaft — the output, an orbiting
/// port.
#[must_use]
pub fn planocentric(planet: u32, ring: u32) -> Shape {
    epicyclic(
        1,
        &[&[external(planet)]],
        &[Central::Carrier, Central::Ring { on: 0, teeth: ring }],
        &[],
    )
}

/// **A set with meshed planets** — the "gutter": a sun meshing planet A,
/// planet A meshing planet B, planet B meshing the ring, the two planets on
/// their own carried axes. It reverses the simple set's sense: with the
/// ring held the carrier turns against the sun, at `1 − z_r/z_s`.
///
/// Sun, carrier, ring, then planet A's shaft and planet B's.
#[must_use]
pub fn meshed_planets(sun: u32, planets: [u32; 2], ring: u32, count: u32) -> Shape {
    epicyclic(
        count,
        &[&[external(planets[0])], &[external(planets[1])]],
        &[
            Central::Sun { on: 0, teeth: sun },
            Central::Carrier,
            Central::Ring { on: 1, teeth: ring },
        ],
        &[(0, 1)],
    )
}

/// **A Ravigneaux set**: a small sun meshing the long planets, which mesh
/// the ring; a large sun meshing the short planets, which mesh the long
/// ones — two planet axes on one carrier, and the planet–planet mesh a
/// distance between two carried axes.
///
/// Small sun, carrier, large sun, ring, then the long planet's shaft and
/// the short's — the carrier the output by convention, whichever sun
/// drives and whichever is held.
#[must_use]
pub fn ravigneaux(suns: [u32; 2], planets: [u32; 2], ring: u32, count: u32) -> Shape {
    epicyclic(
        count,
        &[&[external(planets[0])], &[external(planets[1])]],
        &[
            Central::Sun {
                on: 0,
                teeth: suns[0],
            },
            Central::Carrier,
            Central::Sun {
                on: 1,
                teeth: suns[1],
            },
            Central::Ring { on: 0, teeth: ring },
        ],
        &[(1, 0)],
    )
}

/// **A worm feeding a spur pair in one stage**: the worm on its own axis at
/// a right angle to a wheel shaft that also carries a pinion, and the gear
/// the pinion drives on a third axis parallel to it. Two distances, one at
/// an angle; a point contact and a line contact in one shape, which is what
/// a crossed distance being a mesh like any other buys.
///
/// Shafts: worm 1, output 2, wheel 3; members: worm, wheel, pinion, gear.
#[must_use]
pub fn worm_and_pair(worm: (u32, u32), pair: (u32, u32)) -> Shape {
    let mut b = Builder::new(1.0);
    let worm_axis = b.axis();
    let wheel_axis = b.axis();
    let out_axis = b.axis();
    let worm_shaft = b.shaft(worm_axis);
    let output = b.shaft(out_axis);
    let wheel_shaft = b.shaft(wheel_axis);
    let w = b.gear(worm_shaft, worm.0);
    let wheel = b.gear(wheel_shaft, worm.1);
    let pinion = b.gear(wheel_shaft, pair.0);
    let gear = b.gear(output, pair.1);
    b.mesh(w, wheel)
        .mesh(pinion, gear)
        .crossed([worm_axis, wheel_axis], 90.0)
        .distance([wheel_axis, out_axis]);
    // Sized as a worm and its wheel — the preset's word, which gives the
    // two their conventional proportions and their names — and a worm's
    // size is its diameter, not a helix: seven millimetres at one start is
    // the worm preset's.
    b.shape.distances[0].worm = true;
    b.shape.members[0].pitch_diameter = Auto::fixed(7.0);
    b.build()
}

/// **The three families a shape reads as**, off its graph and never stored:
/// a carried axis makes it epicyclic; failing that, a distance at an angle
/// or marked as a worm drive makes it skew; anything else is parallel. A
/// spur pair is the epicyclic family with its carrier held and no ring —
/// which is why the family is a reading and not a kind, and why a crossed
/// pair turned to nought is simply a parallel one afterwards.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum StageFamily {
    Parallel,
    Skew,
    Epicyclic,
}

impl Shape {
    /// Which family this shape is, read off it — see [`StageFamily`].
    #[must_use]
    pub fn family(&self) -> StageFamily {
        if self.axes.iter().any(|a| a.carried_by.is_some()) {
            StageFamily::Epicyclic
        } else if self.distances.iter().any(|d| d.angle != 0.0 || d.worm) {
            StageFamily::Skew
        } else {
            StageFamily::Parallel
        }
    }
}

/// **A preset: a shape pre-assembled at sensible teeth**, offered by name
/// under its family. None is a kind — every one is a list of what sits
/// where, and a designer edits it into its neighbours afterwards — but a
/// worm is not an obvious construction from a pair and a Wolfrom is not one
/// from a planetary set, so each is a menu entry. The order here is the
/// menu's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum StagePreset {
    /// A spur or helical pair ([`super::PairStage`]).
    Spur,
    /// A pair with an idler between: the same ratio, the other sense.
    Idler,
    /// Two pairs on one distance with a layshaft between, one engaged.
    Layshaft,
    /// A worm and its wheel ([`super::PairStage::worm`]).
    Worm,
    /// A helical pair on shafts at a right angle: a point contact.
    Crossed,
    /// A simple set ([`super::PlanetaryStage`]).
    Planetary,
    /// A planet meshing two rings, no sun: the 3K reduction.
    Wolfrom,
    /// A stepped planet: sun and first ring on one gear, second ring on
    /// the other.
    Compound,
    /// One planet in one ring, its own turn the output.
    Planocentric,
    /// Two planets in mesh between the sun and the ring: the reversing
    /// set.
    MeshedPlanets,
}

impl StagePreset {
    /// Every preset, in the menu's order.
    pub const ALL: [Self; 10] = [
        Self::Spur,
        Self::Idler,
        Self::Layshaft,
        Self::Worm,
        Self::Crossed,
        Self::Planetary,
        Self::Wolfrom,
        Self::Compound,
        Self::Planocentric,
        Self::MeshedPlanets,
    ];

    /// The family the preset is listed under — the family its shape reads
    /// as, and the test below holds the two to each other.
    #[must_use]
    pub fn family(self) -> StageFamily {
        match self {
            Self::Spur | Self::Idler | Self::Layshaft => StageFamily::Parallel,
            Self::Worm | Self::Crossed => StageFamily::Skew,
            Self::Planetary
            | Self::Wolfrom
            | Self::Compound
            | Self::Planocentric
            | Self::MeshedPlanets => StageFamily::Epicyclic,
        }
    }

    /// The catalogue key of the preset's name — a `ui.` key, since the
    /// name is the interface's word and not a note the solve emits.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Spur => "ui.train_preset_spur",
            Self::Idler => "ui.train_preset_idler",
            Self::Layshaft => "ui.train_preset_layshaft",
            Self::Worm => "ui.train_preset_worm",
            Self::Crossed => "ui.train_preset_crossed",
            Self::Planetary => "ui.train_preset_planetary",
            Self::Wolfrom => "ui.train_preset_wolfrom",
            Self::Compound => "ui.train_preset_compound",
            Self::Planocentric => "ui.train_preset_planocentric",
            Self::MeshedPlanets => "ui.train_preset_meshed_planets",
        }
    }

    /// The shape the preset starts as. The two pairs and the simple set
    /// are their own vocabularies' defaults; the rest are the lists above
    /// at the counts the suite's textbook checks use.
    #[must_use]
    pub fn build(self) -> Shape {
        match self {
            Self::Spur => Shape::from(&super::PairStage::default()),
            Self::Idler => line(&[17, 25, 43]),
            Self::Layshaft => layshaft((17, 43), &[(19, 41), (31, 29)], 1),
            Self::Worm => Shape::from_pair(&super::PairStage::worm(), super::PairKind::Worm),
            Self::Crossed => Shape::from(&super::PairStage {
                shaft_angle: 90.0,
                ..super::PairStage::default()
            }),
            Self::Planetary => Shape::from(&super::PlanetaryStage::default()),
            Self::Wolfrom => wolfrom(18, [60, 61], 3),
            Self::Compound => stepped(24, [18, 17], [60, 59], 3),
            Self::Planocentric => planocentric(30, 33),
            Self::MeshedPlanets => meshed_planets(24, [18, 18], 96, 3),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    //! Each arrangement solves through the shape under its textbook
    //! boundary and gives the ratio the textbook gives, with every distance
    //! closed — the laws the set's tests hold, asked of the arrangements
    //! the set's kind could not name.

    use super::super::shape::{solve_loads, ShapeResult};
    use super::super::{test_library as library, Constrained, Reversal, StageBoundary, StageLoads};
    use super::*;

    fn solve(shape: &Shape, held: &[Shaft], input: Shaft, output: Shaft) -> ShapeResult {
        let boundary = StageBoundary::holding(shape.shafts.len() + 1, held, input, output);
        under(shape, boundary)
    }

    /// The arrangement as its list reads: the first ring held, the first
    /// shaft not held driven, the next the output — what a stage does with
    /// nothing stated, which is the claim each list's doc makes.
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
        .unwrap()
    }

    /// Every mesh on every distance runs at that distance, opened by the
    /// clearance its own way.
    fn every_distance_closes(r: &ShapeResult) {
        for d in &r.distances {
            for nominal in &d.nominal {
                assert!(
                    ((d.running - nominal).abs() - d.clearance.abs()).abs() < 1e-9,
                    "nominal {nominal} at running {} with clearance {}",
                    d.running,
                    d.clearance
                );
            }
        }
    }

    /// **A planet's teeth carry one instance's share, on every mesh it is
    /// in** — a planet–planet mesh included. A planet is a free idler: the
    /// tooth load its second mesh carries is its first mesh's less that
    /// mesh's loss, so a planet's reported torque — the larger of its two —
    /// is its sun's tooth load, itself one instance's share, read across the
    /// sun mesh, to within the mesh's efficiency. The mesh between two
    /// planets was divided by what its members see — one path each — rather
    /// than by its own count, and the meshed planets' and the Ravigneaux's
    /// planets carried `N` times their share on that mesh.
    #[test]
    fn a_planet_carries_one_instances_share_on_every_mesh() {
        let cases = [
            (meshed_planets(24, [18, 18], 96, 3), 1, 3),
            (ravigneaux([18, 30], [22, 18], 62, 3), 1, 3),
        ];
        for (shape, input, held) in cases {
            let r = solve(&shape, &[held], input, 2);
            let sun = 0;
            for (i, m) in shape.members.iter().enumerate() {
                let axis = shape.shafts[m.shaft - 1].axis;
                if shape.axes[axis].carried_by.is_none() {
                    continue;
                }
                // The sun's tooth load — one instance's — read across to
                // this planet.
                let share = r.members[sun].cases[0].torque * f64::from(m.gear.teeth)
                    / f64::from(shape.members[sun].gear.teeth);
                let got = r.members[i].cases[0].torque;
                assert!(
                    got <= share * 1.0001 && got >= share * 0.9,
                    "member {i}: {got} against one instance's share {share}"
                );
            }
        }
    }

    /// **A point contact and a line contact in one stage** multiply as any
    /// two meshes do: the ratio is the worm's times the pair's, the
    /// efficiency the product of the two meshes' own, the wheel is rated by
    /// contact alone and the pinion on its shaft by bending as well, and
    /// the stage locks backward exactly where the worm does.
    #[test]
    fn a_worm_and_a_spur_pair_share_one_stage() {
        let shape = worm_and_pair((1, 40), (17, 43));
        let r = solve(&shape, &[], 1, 2);
        assert!(
            (r.ratio.unwrap().abs() - 40.0 * 43.0 / 17.0).abs() < 1e-9,
            "{}",
            r.ratio.unwrap()
        );
        every_distance_closes(&r);
        assert!(r.meshes[0].point.is_some() && r.meshes[1].line.is_some());
        let product = r.meshes[0].efficiency.forward * r.meshes[1].efficiency.forward;
        assert!(
            (r.efficiency.unwrap().forward - product).abs() < 1e-9,
            "{} vs {product}",
            r.efficiency.unwrap().forward
        );
        assert_eq!(
            r.efficiency.unwrap().backward <= 0.0,
            r.meshes[0].efficiency.backward <= 0.0,
            "the stage locks where its worm does"
        );
        assert!(r.members[1].cases[0].bending_stress.is_none());
        assert!(r.members[2].cases[0].bending_stress.is_some());
        assert!(r.members[1].cases[0].contact_stress > 0.0);
        // The wheel and the pinion turn as one: the same shaft.
        assert_eq!(r.members[1].cases[0].speed, r.members[2].cases[0].speed);
    }

    #[test]
    fn a_layshaft_transmission_is_the_engaged_pair_times_the_constant_mesh() {
        // 17/43 into the layshaft; three ratios, the second engaged.
        let pairs = [(19, 41), (31, 29), (43, 17)];
        for engaged in 0..pairs.len() {
            let shape = layshaft((17, 43), &pairs, engaged);
            let r = solve(&shape, &[], 1, 2);
            let (on_lay, on_out) = pairs[engaged];
            let want = (43.0 / 17.0) * (f64::from(on_out) / f64::from(on_lay));
            assert!(
                (r.ratio.unwrap() - want).abs() < 1e-12,
                "pair {engaged}: {} vs {want}",
                r.ratio.unwrap()
            );
            every_distance_closes(&r);
            // Four meshes at one distance: the input's, and one per ratio.
            assert_eq!(r.distances.len(), 1);
            assert_eq!(r.distances[0].nominal.len(), 4);
            // The idlers turn and carry nothing.
            let idlers: Vec<Shaft> = (4..=shape.shafts.len()).collect();
            for c in &r.cases {
                for &s in &idlers {
                    assert!(c.speeds[s] != 0.0 || c.case == 2, "idler {s} stands still");
                    assert!(c.torques[s].abs() < 1e-12, "idler {s} carries torque");
                }
            }
        }
    }

    #[test]
    fn a_wolfrom_set_reduces_by_the_ring_difference() {
        // Carrier in, first ring held, second ring out: i = z_r2 / (z_r2 − z_r1).
        let shape = wolfrom(18, [60, 61], 3);
        let r = conventionally(&shape);
        assert!(
            (r.ratio.unwrap() - 61.0).abs() < 1e-12,
            "{}",
            r.ratio.unwrap()
        );
        every_distance_closes(&r);
        assert_eq!(r.layouts[0].count, 3);
    }

    #[test]
    fn a_stepped_planet_set_carries_the_second_ring_at_the_first_radius() {
        // Sun in, first ring held, second ring out. Willis on the compound
        // planet: ω_r2 − ω_c = (ω_s − ω_c) · (−z_s/z_p1) · (z_p2/z_r2).
        let (zs, zp1, zr1, zp2, zr2) = (24, 18, 60, 17, 59);
        let shape = stepped(zs, [zp1, zp2], [zr1, zr2], 3);
        let r = conventionally(&shape);
        let e1 = -f64::from(zs) / f64::from(zp1) * f64::from(zp1) / f64::from(zr1);
        let e2 = -f64::from(zs) / f64::from(zp1) * f64::from(zp2) / f64::from(zr2);
        // With ring 1 held, ω_c/ω_s = e1/(e1 − 1); then ω_r2 = ω_c + e2(ω_s − ω_c).
        let wc = e1 / (e1 - 1.0);
        let wr2 = wc + e2 * (1.0 - wc);
        assert!(
            (r.ratio.unwrap() - 1.0 / wr2).abs() < 1e-9,
            "{} vs {}",
            r.ratio.unwrap(),
            1.0 / wr2
        );
        every_distance_closes(&r);
    }

    #[test]
    fn a_planocentric_reducer_is_minus_the_planet_count_over_the_difference() {
        // Carrier in, ring held, the planet's own turn out.
        for (zp, zr) in [(30, 33), (40, 42)] {
            let shape = planocentric(zp, zr);
            let r = conventionally(&shape);
            let want = -f64::from(zr - zp) / f64::from(zp);
            assert!(
                (1.0 / r.ratio.unwrap() - want).abs() < 1e-12,
                "{zp}/{zr}: {} vs {}",
                1.0 / r.ratio.unwrap(),
                want
            );
            every_distance_closes(&r);
            // One planet: no replicated axis, so nothing to lay out.
            assert!(r.layouts.is_empty());
        }
    }

    /// **A distance the tips size**: a planocentric at one tooth of
    /// difference cannot run at the half-module the shifts leave — the
    /// pinion's tip would stand inside the ring's on the far side — so the
    /// distance opens out to where the tips clear by what was asked, the
    /// shifts reach it, and the report says which mesh held it open.
    #[test]
    fn a_four_tooth_planocentric_is_sized_by_its_tips() {
        // The hula stage's second mesh, on its own: 57 in 61, teeth cut to
        // 0.7 of a module, three tenths of far-side gap asked.
        let mut shape = planocentric(57, 61);
        for m in &mut shape.members {
            m.gear.addendum = 0.7;
            m.gear.dedendum = 1.0;
        }
        shape.members[0].ring = Some(Cutter {
            teeth: 20,
            addendum: 1.0,
            ..Cutter::default()
        });
        shape.distances[0].tip_clearance = 0.3;
        let r = solve(&shape, &[2], 1, 3);
        let d = &r.distances[0];
        assert_eq!(d.sized_by, Some(0), "the one mesh sized it");
        assert!(
            d.running > 2.0,
            "opened past the two modules the counts leave: {}",
            d.running
        );
        every_distance_closes(&r);
        // The far-side gap is what was asked, to the solver's tolerance, and
        // the tips do not cross.
        let (ring, pinion) = (&r.members[0], &r.members[1]);
        let ring_tip = crate::ring::Ring::cut_by(&ring.params, &shape.members[0].ring.unwrap()).ra;
        let far = ring_tip - crate::tooth::Tooth::new(pinion.params).ra + d.running;
        assert!((far - 0.3).abs() < 1e-6 || far > 0.3, "far-side gap {far}");
        assert_eq!(r.meshes[0].tips.map(|t| t.tip_interference), Some(false));
        assert!((1.0 / r.ratio.unwrap() + 4.0 / 57.0).abs() < 1e-12);
        // ...and a distance the designer states is not sized: it leaves what
        // it leaves, and says so through the mesh's own room.
        shape.distances[0].distance = Auto::fixed(2.0);
        let r = solve(&shape, &[2], 1, 3);
        assert_eq!(r.distances[0].sized_by, None);
    }

    /// **The shape's sizing is the hula stage's**, on the mesh the hula's
    /// harness reports held open by its tips: `gear-cli hula 18 0.2` runs
    /// 19/18 at a crank offset of 0.726026 mm (0.746026 at zero backlash),
    /// its tip margin at nought and its far-side gap 0.2779 with 0.2 asked.
    /// A planocentric of that pair, cut to the hula's proportions, is sized
    /// to the same offset by the shape — two solvers, one bound.
    #[test]
    fn the_shape_sizes_a_distance_where_the_hula_stage_does() {
        // The hula's proportions: the first mesh of `hula([19, 18, ..])`,
        // as a planocentric of the same two members.
        let proportions = hula([19, 18, 17, 18], [1.0, 1.0]);
        let mut shape = planocentric(18, 19);
        for (m, g) in shape
            .members
            .iter_mut()
            .zip([&proportions.members[0], &proportions.members[2]])
        {
            m.gear = StageGear {
                teeth: m.gear.teeth,
                ..g.gear.clone()
            };
        }
        shape.members[0].ring = Some(Cutter {
            teeth: 14,
            ..proportions.members[0].ring.unwrap()
        });
        shape.distances[0].tip_clearance = 0.2;
        let r = solve(&shape, &[2], 1, 3);
        let d = &r.distances[0];
        assert_eq!(d.sized_by, Some(0));
        assert!(
            (d.running - 0.726_026).abs() < 2e-5,
            "the hula's crank offset: {}",
            d.running
        );
        assert!(
            (r.members[0].profile_shift - 0.4519).abs() < 2e-4,
            "the ring's shift"
        );
    }

    #[test]
    fn meshed_planets_reverse_the_simple_set() {
        // Sun in, ring held, carrier out: 1 − z_r/z_s, negative.
        let (zs, zr) = (24, 96);
        let shape = meshed_planets(zs, [18, 18], zr, 3);
        let r = conventionally(&shape);
        assert!(
            (r.ratio.unwrap() - (1.0 - f64::from(zr) / f64::from(zs))).abs() < 1e-12,
            "{}",
            r.ratio.unwrap()
        );
        every_distance_closes(&r);
        assert_eq!(r.layouts.len(), 2);
    }

    #[test]
    fn a_ravigneaux_set_gives_both_textbook_reductions() {
        // The ring the long planets' sun and count make ideal, `z_s1 + 2 z_pl`;
        // the same counts `tools/train_kinematics.py` lays out.
        let (zs1, zs2, zpl, zps, zr) = (18, 30, 22, 18, 62);
        let shape = ravigneaux([zs1, zs2], [zpl, zps], zr, 3);
        // Small sun in, carrier held, ring out: a simple reversing pair through
        // the long planet, −z_r/z_s1.
        let r = solve(&shape, &[2], 1, 4);
        assert!(
            (r.ratio.unwrap() + f64::from(zr) / f64::from(zs1)).abs() < 1e-12,
            "{}",
            r.ratio.unwrap()
        );
        every_distance_closes(&r);
        // Large sun in, carrier held, ring out: through both planets, +z_r/z_s2.
        let r = solve(&shape, &[2], 3, 4);
        assert!(
            (r.ratio.unwrap() - f64::from(zr) / f64::from(zs2)).abs() < 1e-12,
            "{}",
            r.ratio.unwrap()
        );
        every_distance_closes(&r);
        // Small sun in, ring held, carrier out: 1 + z_r/z_s1.
        let r = solve(&shape, &[4], 1, 2);
        assert!(
            (r.ratio.unwrap() - (1.0 + f64::from(zr) / f64::from(zs1))).abs() < 1e-12,
            "{}",
            r.ratio.unwrap()
        );
        every_distance_closes(&r);
        assert_eq!(r.distances.len(), 3);
        // Two replicated axes, each laid out: the short planets stand nearer
        // the centre than the long ones and have a clearance of their own.
        assert_eq!(r.layouts.len(), 2);
        assert_eq!((r.layouts[0].axis, r.layouts[1].axis), (1, 2));
        assert!(r.layouts.iter().all(|l| l.count == 3 && l.clearance > 0.0));
    }

    /// **Every preset reads as the family it is listed under, and solves
    /// with nothing stated** — the menu's grouping is the shape's own
    /// reading, and a preset a designer adds is a stage that answers.
    #[test]
    fn every_preset_is_in_its_own_family_and_solves_conventionally() {
        for preset in StagePreset::ALL {
            let shape = preset.build();
            assert_eq!(shape.family(), preset.family(), "{preset:?}");
            let r = conventionally(&shape);
            assert!(
                r.ratio.is_some_and(|x| x.is_finite() && x != 0.0),
                "{preset:?}"
            );
            every_distance_closes(&r);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod hula {
    //! **The hula tables `docs/reference.md#the-hula-stage` prints are the
    //! ones this code prints**, through the shape the `hula` list lays out.
    //! Prose is the copy no test reads, and these sections had drifted
    //! before the tests existed. A canary rather than an invariant: the
    //! digits are free to move, and not free to move *quietly*. Asserted to
    //! half of the last digit each table prints, since that is the claim it
    //! makes. Written against the hula's own preset once; the preset is
    //! gone and the shape is the list, and every figure held.

    use super::super::shape::{solve_loads, Shape, ShapeResult};
    use super::super::{test_library, Reversal, StageBoundary, StageLoads, TrainError};
    use super::*;
    use crate::planetary::carrier_driven_efficiency;

    /// The inputs the hula's preset once took, over the list: the counts
    /// (grounded, wobble, wobble, output), each mesh's module, every tooth's
    /// addendum, the far-side gap the tips size the offset by, the meshes'
    /// friction, the search, and the shaper's teeth.
    struct Fixture {
        teeth: [u32; 4],
        module: [f64; 2],
        addendum: f64,
        gap: f64,
        friction: (f64, f64),
        optimise: bool,
        cutter_teeth: u32,
    }

    impl Fixture {
        /// The shipped stage: N ± 4 about 61.
        fn shipped() -> Self {
            Self {
                teeth: [65, 61, 57, 61],
                module: [1.0, 1.0],
                addendum: 0.7,
                gap: 0.3,
                friction: (0.08, 0.16),
                optimise: false,
                cutter_teeth: 20,
            }
        }

        /// One tooth of difference about 18, at 0.8 of a module, with a
        /// shaper that fits the rings this fixture builds — the fixture
        /// the tables were generated from.
        fn tables() -> Self {
            Self {
                teeth: [19, 18, 17, 18],
                addendum: 0.8,
                cutter_teeth: 12,
                ..Self::shipped()
            }
        }

        fn teeth(self, teeth: [u32; 4]) -> Self {
            Self { teeth, ..self }
        }

        fn shape(&self) -> Shape {
            let mut shape = hula(self.teeth, self.module);
            shape.optimisation.enabled = self.optimise;
            shape.distances[0].tip_clearance = self.gap;
            for m in &mut shape.meshes {
                m.sliding_friction = self.friction.0;
                m.static_friction = self.friction.1;
            }
            for m in &mut shape.members {
                m.gear.addendum = self.addendum;
                if let Some(c) = &mut m.ring {
                    c.teeth = self.cutter_teeth;
                }
            }
            shape
        }

        /// The hula's own arrangement: crank driven, grounded gear held,
        /// output out — the shape's shafts 1, 2 and 3.
        fn solve(&self, speed: f64) -> Result<ShapeResult, TrainError> {
            solve_loads(
                &self.shape(),
                &StageLoads::at(2.0, speed).under(StageBoundary::holding(5, &[2], 1, 3)),
                &test_library(),
                Reversal::default(),
            )
        }
    }

    /// The two meshes alone, crank held: `η₀`.
    fn eta0(r: &ShapeResult) -> f64 {
        r.meshes.iter().map(|m| m.efficiency.forward).product()
    }

    fn alpha_w(r: &ShapeResult, mesh: usize) -> f64 {
        r.meshes[mesh].line.unwrap().operating_pressure_angle
    }

    #[test]
    fn the_documented_tables_are_the_ones_this_code_prints() {
        // | reduction | meshes, crank held | the stage |
        for (n, reduction, meshes, keeps) in [
            (12_u32, 144.0, 98.85, 37.9),
            (18, 324.0, 99.18, 27.4),
            (30, 900.0, 99.50, 18.1),
            (50, 2500.0, 99.69, 11.5),
        ] {
            let r = Fixture::tables()
                .teeth([n + 1, n, n - 1, n])
                .solve(1000.0)
                .unwrap();
            assert!(
                (r.ratio.unwrap().abs() - reduction).abs() < 1e-9,
                "z{n}: {}",
                r.ratio.unwrap()
            );
            let got = (eta0(&r) * 100.0, r.efficiency.unwrap().forward * 100.0);
            assert!(
                (got.0 - meshes).abs() < 0.005 && (got.1 - keeps).abs() < 0.05,
                "z{n}: the table says {meshes} % / {keeps} %, this gives {:.2} / {:.1}",
                got.0,
                got.1
            );
        }

        // | m₁/m₂ | offset | α_w mesh 1 | α_w mesh 2 | — the angles the
        // **running** meshes turn at, 0.02 mm inside the offset quoted.
        for (ratio, offset, first, second) in [
            (0.8_f64, 0.813, 61.7, 53.6),
            (0.9, 0.813, 57.8, 53.6),
            (1.0, 0.813, 53.6, 53.6),
            (1.1, 0.884, 53.3, 57.1),
            (1.3, 1.028, 52.7, 62.2),
        ] {
            let f = Fixture {
                module: [ratio, 1.0],
                gap: 0.30,
                ..Fixture::tables()
            };
            let r = f
                .solve(1000.0)
                .unwrap_or_else(|e| panic!("m {ratio}: the table's row must solve: {e}"));
            let angles = [alpha_w(&r, 0), alpha_w(&r, 1)];
            let nominal = r.distances[0].nominal[0];
            assert!(
                (nominal - offset).abs() < 0.0005
                    && (angles[0] - first).abs() < 0.05
                    && (angles[1] - second).abs() < 0.05,
                "m {ratio}: the table says {offset} mm and {first}°/{second}°, \
                 this gives {nominal:.3} mm and {:.1}°/{:.1}°",
                angles[0],
                angles[1]
            );
        }
    }

    #[test]
    fn the_four_hula_studies_are_the_ones_this_code_prints() {
        // | arrangement | `D` | ratio | meshes | the stage |    — at N = 18.
        // Each is solved under the hula's own arrangement whatever its
        // counts make the rings, which is what the explicit boundary is for.
        for (name, counts, ratio, meshes, keeps) in [
            ("N+1/N/N−1/N", [19u32, 18, 17, 18], 324.0_f64, 99.18, 27.4),
            ("N/N+1/N/N−1", [18, 19, 18, 17], -323.0, 99.18, 27.2),
            ("N+1/N/N/N−1", [19, 18, 18, 17], -8.5, 99.18, 92.7),
            ("N/N+1/N/N+1", [18, 19, 18, 19], 9.8, 99.20, 93.5),
        ] {
            let r = Fixture::tables()
                .teeth(counts)
                .solve(1000.0)
                .unwrap_or_else(|e| panic!("{name}: must solve: {e}"));
            assert!(
                (r.ratio.unwrap() - ratio).abs() < 0.05,
                "{name}: the table says a ratio of {ratio}, this gives {}",
                r.ratio.unwrap()
            );
            let got = (eta0(&r) * 100.0, r.efficiency.unwrap().forward * 100.0);
            assert!(
                (got.0 - meshes).abs() < 0.005 && (got.1 - keeps).abs() < 0.05,
                "{name}: the table says {meshes} % / {keeps} %, this gives {:.2} / {:.1}",
                got.0,
                got.1
            );
        }

        // | `h_a` | involute interference | ε_α | the stage |   — on the shipped
        // N ± 4 about 61
        for (addendum, fouls, eps, keeps) in [
            (0.60_f64, false, 1.17, 90.4),
            (0.65, false, 1.26, 86.2),
            (0.70, false, 1.35, 81.9),
            (0.75, true, 1.44, 78.2),
            (0.80, true, 1.52, 75.0),
        ] {
            let f = Fixture {
                addendum,
                ..Fixture::shipped()
            };
            let r = f
                .solve(1000.0)
                .unwrap_or_else(|e| panic!("h_a {addendum}: must solve: {e}"));
            // The involute interference the table names is the pinion's flank
            // reached past its end by the ring's tip, on either mesh.
            let got_fouls = r.meshes.iter().any(|m| m.flank_interference[0]);
            let got_eps = r.meshes[0].line.unwrap().contact_ratios.transverse;
            let got_keeps = r.efficiency.unwrap().forward * 100.0;
            assert!(
                got_fouls == fouls,
                "h_a {addendum}: the table says fouls={fouls}, this says {got_fouls}"
            );
            assert!(
                (got_eps - eps).abs() < 0.005 && (got_keeps - keeps).abs() < 0.05,
                "h_a {addendum}: the table says ε {eps} / {keeps} %, this gives {:.2} / {:.1}",
                got_eps,
                got_keeps
            );
        }

        // | d | reduction | α_w | the pair keeps | the stage keeps |  — z = 36,
        // h_a = 0.6, μ = 0.08, each pair optimised alone
        // | d | least loss (Σx, x_ring, x_pinion) | least shift | stage, best | stage, least |
        let at_d = |d: u32, optimise: bool| {
            Fixture {
                addendum: 0.6,
                optimise,
                ..Fixture::shipped().teeth([36 + d, 36, 36 - d, 36])
            }
            .solve(1000.0)
            .unwrap_or_else(|e| panic!("d {d}: must solve: {e}"))
        };
        for (d, reduction, alpha, pair_keeps, best, least) in [
            (2u32, 324.0_f64, 33.0, 99.893, 58.39, 54.81),
            (3, 144.0, 25.2, 99.966, 90.57, 79.52),
            (4, 81.0, 20.9, 99.959, 93.25, 91.72),
            (5, 51.8, 18.8, 99.948, 94.25, 94.25),
        ] {
            let (on, off) = (at_d(d, true), at_d(d, false));
            assert!(
                (on.ratio.unwrap().abs() - reduction).abs() < 0.05,
                "d {d}: the table says {reduction}, this gives {}",
                on.ratio.unwrap()
            );
            let aw = alpha_w(&on, 0);
            let pk = on.meshes[0].efficiency.forward * 100.0;
            assert!(
                (aw - alpha).abs() < 0.05 && (pk - pair_keeps).abs() < 0.0005,
                "d {d}: the table says α_w {alpha}° and the pair keeps {pair_keeps} %, \
                 this gives {aw:.1}° and {pk:.3} %"
            );
            let (got_best, got_least) = (
                on.efficiency.unwrap().forward * 100.0,
                off.efficiency.unwrap().forward * 100.0,
            );
            assert!(
                (got_best - best).abs() < 0.005 && (got_least - least).abs() < 0.005,
                "d {d}: the table says {best} % best and {least} % least, this gives \
                 {got_best:.2} % and {got_least:.2} %"
            );
            // **The two pairs land at the same operating angle** — the physical
            // claim the prose makes, and the one that makes equal modules cost
            // nothing.
            let twin = alpha_w(&on, 1);
            assert!(
                (aw - twin).abs() < 0.005,
                "d {d}: the two meshes should sit at one angle: {aw:.2}° and {twin:.2}°"
            );
        }

        // | d | least loss (Σx, x_ring, x_pinion) | least shift |  — the shift
        // columns of the same table, to the two decimals it prints. The
        // grounded ring is member 0, its pinion the first wobble gear, 2.
        for (d, on_shifts, off_shifts) in [
            (2u32, [-0.19_f64, 0.37, 0.18], [-0.20_f64, 0.20, 0.00]),
            (3, [-0.09, 0.51, 0.42], [-0.11, 0.11, 0.00]),
            (4, [-0.03, 0.37, 0.34], [-0.05, 0.05, 0.00]),
            (5, [0.00, 0.00, 0.00], [0.00, 0.00, 0.00]),
        ] {
            for (optimise, want) in [(true, on_shifts), (false, off_shifts)] {
                let r = at_d(d, optimise);
                let (ring, pin) = (r.members[0].profile_shift, r.members[2].profile_shift);
                let got = [pin - ring, ring, pin];
                for (g, w) in got.iter().zip(want) {
                    assert!(
                        (g - w).abs() < 0.005,
                        "d {d} optimise={optimise}: the table says {want:?}, this gives \
                         [{:.2}, {:.2}, {:.2}]",
                        got[0],
                        got[1],
                        got[2]
                    );
                }
            }
        }
    }

    /// **The power circulates, and the figure says by how much.** On a pair
    /// everything crosses the one mesh, once; on a hula each mesh passes
    /// about `η |R − 1|` times the input to cancel to the output — 88× at
    /// 324 : 1 keeping 27 %, 9× at 8.5 : 1 keeping 93 % — and the stage's loss is
    /// each mesh's loss on the power crossing it, exactly, which is what
    /// `η = 1/[R(1 − η₀) + η₀]` folds into one line.
    #[test]
    fn the_circulating_power_is_the_reductions_worth() {
        let r = Fixture::tables().solve(1000.0).unwrap();
        let through = r.circulation.unwrap().forward;
        assert!(through > 100.0, "324 : 1 circulates: {through}× the input");
        // The loss is each mesh's loss on the power crossing it: what the
        // teeth pass, less what comes out, over the input — to the
        // per-mesh accounting the flow keeps, exactly.
        let lost = 1.0 - r.efficiency.unwrap().forward;
        let by_mesh: f64 = r
            .meshes
            .iter()
            .map(|m| (1.0 - m.efficiency.forward) * m.power_through.forward)
            .sum();
        assert!(
            (lost - by_mesh).abs() < 1e-9,
            "loss {lost} against the meshes' {by_mesh}"
        );
        assert!(
            (r.meshes[0].power_through.forward + r.meshes[1].power_through.forward - through).abs()
                < 1e-12
        );
        // ...and each mesh passes about `R η` times the input, on the
        // stage that cancels and on one that does not.
        let plain = Fixture::tables()
            .teeth([19, 18, 18, 17])
            .solve(1000.0)
            .unwrap();
        // The output turns `R` times slower than the crank, so relative to
        // the crank the mesh sees the output's torque at `|1 − 1/R|` of the
        // crank's speed: `η |R − 1|` of the input, give or take the loss's
        // own share.
        for r in [&r, &plain] {
            let expect = (r.ratio.unwrap() - 1.0).abs() * r.efficiency.unwrap().forward;
            for m in &r.meshes {
                let got = m.power_through.forward;
                assert!(
                    (got - expect).abs() / expect < 0.05,
                    "{} : 1 keeping {}: a mesh passes {got}× against about {expect}×",
                    r.ratio.unwrap(),
                    r.efficiency.unwrap().forward
                );
            }
        }
    }

    /// **The power flow collapses to one relation**, and the shape's flow —
    /// mesh by mesh, with each mesh's loss in the direction it turns — agrees
    /// with it everywhere: `η = 1/[R(1 − η₀) + η₀]`, written from the torque
    /// shares by hand. Agreeing across three reductions and four friction
    /// coefficients says the closed form is the same statement, which is what
    /// makes it safe to design against.
    #[test]
    fn the_stage_efficiency_is_the_reduction_and_the_meshes() {
        for n in [7_u32, 12, 18] {
            for mu in [0.08, 0.04, 0.02, 0.01] {
                let r = Fixture {
                    friction: (mu, mu * 2.0),
                    ..Fixture::tables().teeth([n + 1, n, n - 1, n])
                }
                .solve(1000.0)
                .unwrap();
                let want = carrier_driven_efficiency(r.ratio.unwrap(), eta0(&r));
                assert!(
                    (r.efficiency.unwrap().forward - want).abs() < 1e-9,
                    "z {n} mu {mu}: solve {} against the relation {want}",
                    r.efficiency.unwrap().forward
                );
            }
        }
    }

    /// **Checked against a gearbox somebody built.** The bilateral drive
    /// gear is a 3K of this family reporting 89.0 % forward against 68.5 %
    /// uncorrected; read through the relation those are meshes at 99.73 %
    /// and 99.04 % at a reduction near fifty, and this stage at that
    /// reduction lands where the relation says it should for the teeth it
    /// actually has.
    #[test]
    fn the_relation_agrees_with_a_gearbox_somebody_built() {
        let implied = |eta: f64, ratio: f64| (1.0 / eta - 1.0) / (ratio - 1.0);
        let optimised = 1.0 - implied(0.890, 49.0);
        let uncorrected = 1.0 - implied(0.685, 49.0);
        assert!(
            (optimised - 0.9973).abs() < 5e-4,
            "89.0 % at 49:1 wants meshes at {optimised}"
        );
        assert!(
            (uncorrected - 0.9904).abs() < 5e-4,
            "68.5 % at 49:1 wants meshes at {uncorrected}"
        );
        assert!((carrier_driven_efficiency(49.0, optimised) - 0.890).abs() < 1e-3);
        let r = Fixture {
            friction: (0.010, 0.020),
            ..Fixture::tables().teeth([8, 7, 6, 7])
        }
        .solve(1000.0)
        .unwrap();
        assert!(
            (r.ratio.unwrap() - 49.0).abs() < 1e-9,
            "ratio {}",
            r.ratio.unwrap()
        );
        assert!(
            r.efficiency.unwrap().forward > 0.89 && r.efficiency.unwrap().forward < 0.93,
            "a stage of this reduction with meshes this good keeps {}",
            r.efficiency.unwrap().forward
        );
        let implied_here = 1.0 - implied(r.efficiency.unwrap().forward, 49.0);
        assert!(
            (carrier_driven_efficiency(49.0, implied_here) - r.efficiency.unwrap().forward).abs()
                < 1e-9
        );
    }

    /// **A reduction that does not come from cancellation is efficient**, and
    /// the same code says so: both families have the same two meshes losing
    /// the same 0.85 % between them and differ only in whether the wobble
    /// body carries two faces of the same kind. Where it does the meshes
    /// nearly cancel and the stage keeps a quarter; where it does not, ninety-
    /// odd percent — an ordinary gearbox.
    #[test]
    fn a_reduction_that_does_not_come_from_cancellation_is_efficient() {
        let solve_z = |z: [u32; 4]| Fixture::tables().teeth(z).solve(1000.0).unwrap();
        for z in [[19, 18, 17, 18], [17, 18, 19, 18]] {
            let r = solve_z(z);
            assert!(
                r.ratio.unwrap().abs() > 300.0,
                "{z:?} reduces by {}",
                r.ratio.unwrap()
            );
            assert!(
                r.efficiency.unwrap().forward < 0.35,
                "{z:?}: {} is too good",
                r.efficiency.unwrap().forward
            );
        }
        for z in [[19, 18, 18, 17], [17, 18, 18, 19], [18, 17, 19, 18]] {
            let r = solve_z(z);
            assert!(
                r.ratio.unwrap().abs() < 12.0,
                "{z:?} reduces by {}",
                r.ratio.unwrap()
            );
            assert!(
                r.efficiency.unwrap().forward > 0.9,
                "{z:?}: {} is too poor",
                r.efficiency.unwrap().forward
            );
        }
        let cancelling = eta0(&solve_z([19, 18, 17, 18]));
        let plain = eta0(&solve_z([19, 18, 18, 17]));
        assert!(
            (cancelling - plain).abs() < 0.005,
            "{cancelling} against {plain}"
        );
    }

    /// **A higher reduction costs efficiency**, monotonically, and none of
    /// these turns backwards.
    #[test]
    fn a_higher_reduction_costs_efficiency() {
        let mut last = 1.0;
        for n in [12_u32, 18, 30, 50] {
            let r = Fixture::tables()
                .teeth([n + 1, n, n - 1, n])
                .solve(100.0)
                .unwrap();
            assert!(
                r.efficiency.unwrap().forward < last,
                "z {n}: {} did not fall below {last}",
                r.efficiency.unwrap().forward
            );
            assert_eq!(
                r.efficiency.unwrap().backward,
                0.0,
                "z {n} should be self-locking"
            );
            last = r.efficiency.unwrap().forward;
        }
        assert!(last < 0.2, "2500:1 should be dear: {last}");
    }

    /// **The play at the two shafts differs by the reduction**, and the
    /// output's play is the two meshes' plays referred through the body —
    /// mesh A's at the wobble gear, carried through mesh B by `z₃/z₄`, plus
    /// mesh B's own at the output.
    #[test]
    fn the_outputs_play_is_the_two_meshes_referred_through_the_body() {
        for teeth in [[19_u32, 18, 17, 18], [17, 18, 19, 18], [20, 18, 17, 18]] {
            let f = Fixture::tables().teeth(teeth);
            let shape = f.shape();
            let r = f.solve(100.0).unwrap();
            let want = r.ratio.unwrap().abs();
            let got = r.backlash.unwrap().backward.nominal / r.backlash.unwrap().forward.nominal;
            assert!(
                (got - want).abs() < 1e-9 * want,
                "{teeth:?}: {got} where the reduction is {want}"
            );
            // `backlash[0]` is the pinion's, `[1]` the ring's — the mesh was
            // built with the pinion first. The first wobble gear is member 2,
            // the output member 1.
            let at = |mesh: usize, gear: usize| {
                r.meshes[mesh].backlash[usize::from(shape.members[gear].ring.is_some())].nominal
            };
            let want = at(0, 2) * f64::from(teeth[2]) / f64::from(teeth[3]) + at(1, 1);
            let got = r.backlash.unwrap().forward.nominal;
            assert!(
                (got - want).abs() < 1e-9 * want,
                "{teeth:?}: {got} referred, {want} from the members"
            );
        }
    }

    /// **The offset clears the tips as well as the far side**, and either can
    /// be what binds: at the fixture's gap the far side asks for more and the
    /// tips have room; ask for a gap the tips cannot live with and the offset
    /// opens past it, the tips at their limit and the far-side gap larger than
    /// asked — the tool answering with what can be built.
    #[test]
    fn the_offset_clears_the_tips_as_well_as_the_far_side() {
        let f = Fixture::tables();
        let shipped = f.solve(100.0).unwrap();
        for mesh in &shipped.meshes {
            let tips = mesh.tips.expect("every hula mesh is internal");
            assert!(
                !tips.tip_interference && tips.tip_margin > 0.1,
                "margin {}",
                tips.tip_margin
            );
        }
        let held = shipped.distances[0].sized_by.expect("the gap held it open");
        assert!(
            (shipped.meshes[held].tips.unwrap().far_gap - f.gap).abs() < 1e-6,
            "the binding mesh should sit at the gap asked for"
        );
        let tight = Fixture {
            gap: 0.22,
            ..Fixture::tables()
        };
        let opened = tight.solve(100.0).unwrap();
        for mesh in &opened.meshes {
            let tips = mesh.tips.expect("every hula mesh is internal");
            assert!(
                !tips.tip_interference,
                "the offset should have opened until the tips cleared"
            );
            assert!(
                tips.far_gap >= tight.gap - 1e-6,
                "opening for the tips gives the far side more"
            );
        }
        let binding = opened.distances[0]
            .sized_by
            .expect("something held it open");
        assert!(
            opened.meshes[binding]
                .tips
                .is_some_and(|t| t.tip_margin.abs() < 1e-4),
            "the tips are what held it, so they sit at their limit: {:?}",
            opened.meshes[binding].tips
        );
    }
}
