//! **Bodies, meshes and what relates them** — one matrix, read four ways.
//!
//! Nothing here knows what a tooth looks like. A gear reaches this as a signed
//! tooth count and nothing else, in the discipline of [`crate::solve`] and
//! [`crate::hertz`]: no geometry, no stress, no stage.
//!
//! # The one relation
//!
//! Write a mesh in the frame of the member carrying its axes and the epicyclic
//! character disappears, leaving an ordinary ratio constraint:
//!
//! ```text
//! z_a (ω_a − ω_f) + z_b (ω_b − ω_f) = 0
//! ```
//!
//! with `f` the **frame** — the body in whose rotating frame both gear axes
//! stand still — and the tooth counts **signed**, a ring's being negative.
//! That sign is not a new convention: it is [`crate::mesh::MeshKind::sign`],
//! which this crate already documents as "the whole of the difference between
//! the two kinds", and carrying it a second time as a field of its own would be
//! one idea written down twice.
//!
//! Every arrangement in this crate is that row, repeated:
//!
//! | arrangement | rows |
//! |---|---|
//! | a fixed-axis pair | one, framed on ground |
//! | a planetary set | two, framed on the carrier |
//! | a hula stage | two, framed on the crank |
//! | a compound or meshed-planet set | one per mesh, framed on its carrier |
//!
//! Eliminating the planet from a set's two rows gives
//! `z_s(ω_s − ω_c) + z_r(ω_r − ω_c) = 0` with `z_r` positive — Willis, and
//! exactly the relation `docs/reference.md#planetary-sets` states the set's
//! **backlash** in. That is not a coincidence; it is the third reading below.
//!
//! # Four readings
//!
//! Assemble one row per mesh and one per rigid coupling over the vector of
//! body speeds, and call it `A`:
//!
//! | reading | gives |
//! |---|---|
//! | `A ω = 0`, with the boundary conditions | speeds, ratios, and what is left free ([`System::motion`]) |
//! | `dim null(A)` | **mobility** — how many conditions the train needs ([`System::mobility`]) |
//! | `τ ∈ rowspace(A)` | torques and reactions ([`System::torques`]) |
//! | `A θ = Δ` | the play at each body per unit play in one mesh ([`System::play`]) |
//!
//! # The invariant that costs nothing, and exactly what it is worth
//!
//! A train locked solid turns as one body, so the all-ones vector satisfies
//! every row — which is to say **every row's coefficients sum to zero**
//! ([`System::lock_up_is_free`]).
//!
//! **It does not catch a wrong tooth-count sign, and the first draft of this
//! paragraph said it did.** Measured, by flipping a ring's sign and running the
//! suite: the lock-up test passed and only the cross-check against
//! `planetary::power` failed. The reason is plain once seen — [`System::mesh`]
//! derives the frame's coefficient from the very counts it was handed, so the
//! sum telescopes to zero whatever they are. A check built from the thing under
//! test measures nothing (`docs/corrections.md`), met in the first check this
//! module offered.
//!
//! What it *is* worth is stated rather than assumed:
//!
//! | fault | caught by lock-up? |
//! |---|---|
//! | a row assembled anywhere but [`System::mesh`] or [`System::couple`] | **yes** |
//! | a frame coefficient written by hand — the classic transposition | **yes** |
//! | a tooth count with the wrong sign | no — the cross-check and the geometry do |
//! | a frame on the wrong body | no — the cross-check does |
//!
//! So it is a guard on rows that arrive some other way, and the constructors
//! satisfy it by construction — which is itself asserted, so that "by
//! construction" is a checked statement and not a claim.
//!
//! # Why the arithmetic is exact
//!
//! [`crate::ratio`] says. In short: a rank taken with a pivot tolerance is a
//! rank somebody chose, and a ratio whose denominator is zero is a refusal
//! rather than a large number.

use crate::ratio::Ratio;

/// A body with one angular velocity. Ground is one of these — pinned by a
/// condition, not by being a different kind of thing.
pub type Body = usize;

/// **Ground — a frame like any other, which happens to be held.**
///
/// It is not a different kind of thing and nothing here treats it as one: it is
/// a [`Body`], it appears in mesh rows as a frame, it carries torque, and what
/// makes it ground is [`Condition::Ground`] on it and nothing else. Turn that
/// condition into a [`Condition::Drive`] and the whole train is being measured
/// from a turning frame, which is a coherent question with a coherent answer.
///
/// **Why it is body zero** is the only thing special about it, and it is a
/// bookkeeping fact rather than a physical one: every part needs the *same*
/// one, so that a fixed-axis mesh in one part and a grounded ring in another
/// are held against one frame rather than two (a part lists the train bodies
/// on its axes and numbers its own slots off that list, and every part's
/// slot 0 is this one).
///
/// **A train here has no housing.** An element is fixed to ground, carries a
/// load, or is attached to another element. Calling the reference a housing
/// invites two wrong readings, and both bite in the arrangements this module
/// exists for: that the reference is a component with an interface to size, and
/// that a *frame* must stand still. A frame is whatever body carries a mesh's
/// axes, and in a compound set that is a **carrier** — turning, and shared by
/// several meshes at once.
///
/// *(This crate does say "housing" elsewhere, for the real part whose bore
/// centres set a centre distance — a worm's wheel absorbing a housing distance
/// by its shift, `docs/reference.md`. That is a different thing with a
/// different job, and the two are named apart on purpose: a noun meaning two
/// things in two modules is the fault `tools/check_units.py` exists for in the
/// angle domain.)*
pub const GROUND: Body = 0;

/// What is asked of a body. Three things, and no fourth.
///
/// This is the whole of the boundary layer: which bodies are held, which are
/// driven and at what speed, and which are left to whatever the rest decides.
/// **"Fixed" and "free" are not structural** — nothing above changes when one
/// of these does, which is what lets a designer turn a body loose and re-solve
/// without editing the train.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Condition {
    /// `ω = 0`.
    Ground,
    /// `ω` is this.
    Drive(Ratio),
    /// Nothing is asked of it. It takes whatever the others leave, and carries
    /// no torque — which is why a free body leaves a residual degree of
    /// freedom rather than being solved for.
    Free,
}

/// Why the system could not answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// A condition contradicts what the structure and the earlier conditions
    /// already decided, at the **body** it was asked of — named, because
    /// "over-determined" is not something a designer can act on.
    Conflicts(Body),
    /// The structure itself admits no such displacement: a loop of meshes whose
    /// plays do not add up, which is teeth binding rather than a condition
    /// anyone chose. It cannot arise from [`System::motion`], whose right-hand
    /// side is zero throughout.
    NoMotion,
    /// An exact answer too large to represent. See [`crate::ratio`]: this is a
    /// refusal rather than a wrap into a number that looks like an answer.
    Overflow,
}

/// How much of the train is still free, and how much of that is real.
///
/// A body that no mesh and no coupling touches adds a degree of freedom that
/// belongs to nothing — the handoff this work answers is right that counting it
/// silently is a trap — so it is **named** rather than folded into the number.
/// The commonest one is ground itself, in a train made only of epicyclic
/// meshes — nothing in one meshes against it — and it stops being untouched
/// the moment a fixed-axis mesh joins the train.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mobility {
    /// `bodies − rank`: how many independent conditions the train needs.
    pub degrees: usize,
    /// The bodies no row touches, ascending.
    pub untouched: Vec<Body>,
}

/// One residual degree of freedom, as a direction in body space.
///
/// Normalised at a body — `direction[at] == 1` — so that a reader can say
/// *"and this much per turn of that one"*, which is what a family of answers
/// has to be presented as to be any use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Residual {
    pub at: Body,
    pub direction: Vec<Ratio>,
}

/// **An answer, and what is still free about it.**
///
/// One shape for every reading, because they are one object: a particular
/// solution per body, plus a basis for whatever the conditions did not pin.
/// A unique answer is this with **no** residual — the degenerate value rather
/// than a second variant, so nothing downstream branches on which it got.
///
/// ```text
/// value_i = values[i] + Σ_k p_k · residual[k].direction[i]
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Solution {
    /// One value per body: the particular answer, taken with every free
    /// parameter at zero.
    pub values: Vec<Ratio>,
    /// What the conditions left open. Empty is a unique answer.
    pub residual: Vec<Residual>,
    /// The **bodies** whose condition, or whose given torque, said nothing the
    /// structure had not already said. Not an error — a ring grounded and also
    /// coupled to ground is a designer being explicit — but worth
    /// reporting, since it is also how an over-determined train looks when it
    /// happens to be consistent.
    pub redundant: Vec<Body>,
}

impl Solution {
    /// Whether the answer is one answer.
    #[must_use]
    pub fn is_unique(&self) -> bool {
        self.residual.is_empty()
    }

    /// **The same family, parameterised by the bodies named** — so a
    /// differential reads *per turn of the ring* rather than per turn of a
    /// planet nobody drives.
    ///
    /// A family is a particular answer plus a basis for what is free, and any
    /// body a direction moves can be the parameter of that direction: the
    /// basis is re-normalised at the first of `preferred` each direction
    /// moves and not yet taken, and the particular answer moved to where that
    /// body stands still. Directions that move none of the preferred bodies
    /// keep the parameter they had. The set of answers is unchanged — every
    /// answer the old family contained the new one contains, and no other.
    ///
    /// `None` on overflow, which is a refusal rather than a wrap.
    #[must_use]
    pub fn rebased(&self, preferred: &[Body]) -> Option<Self> {
        let mut values = self.values.clone();
        let mut residual = self.residual.clone();
        let mut taken: Vec<Body> = Vec::new();
        for k in 0..residual.len() {
            let Some(&at) = preferred
                .iter()
                .find(|&&j| !taken.contains(&j) && !residual[k].direction[j].is_zero())
            else {
                continue;
            };
            taken.push(at);
            // Normalise direction `k` at `at`...
            let unit = residual[k].direction[at];
            for d in &mut residual[k].direction {
                *d = d.checked_div(unit)?;
            }
            residual[k].at = at;
            // ...and take `at` out of the particular answer and every other
            // direction, so the parameter is that body's own speed.
            let dk = residual[k].direction.clone();
            let v = values[at];
            for (x, d) in values.iter_mut().zip(&dk) {
                *x = x.checked_sub(v.checked_mul(*d)?)?;
            }
            for (other, r) in residual.iter_mut().enumerate() {
                if other == k {
                    continue;
                }
                let c = r.direction[at];
                if c.is_zero() {
                    continue;
                }
                for (x, d) in r.direction.iter_mut().zip(&dk) {
                    *x = x.checked_sub(c.checked_mul(*d)?)?;
                }
            }
        }
        Some(Self {
            values,
            residual,
            redundant: self.redundant.clone(),
        })
    }

    /// The ratio of one body's value to another's, exactly — `None` where the
    /// divisor is zero, or where the answer is a family and the quotient is not
    /// a number at all.
    #[must_use]
    pub fn ratio(&self, of: Body, per: Body) -> Option<Ratio> {
        self.is_unique()
            .then(|| self.values[of].checked_div(self.values[per]))
            .flatten()
    }
}

/// One mesh, as a constraint.
///
/// **The tooth counts are signed**, a ring's negative
/// ([`crate::mesh::MeshKind::sign`]). The frame is stored rather than inferred:
/// a nested carrier makes the inference ambiguous, and the arrangement that
/// built the mesh is the thing that knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshRow {
    pub a: Body,
    pub b: Body,
    pub za: i64,
    pub zb: i64,
    pub frame: Body,
}

/// A system of bodies and what relates them.
#[derive(Clone, Debug)]
pub struct System {
    bodies: usize,
    /// One row per constraint, over the bodies. Mesh rows first is *not*
    /// assumed anywhere; [`Self::mesh_rows`] indexes them.
    rows: Vec<Vec<Ratio>>,
    /// Which rows came from meshes, in the order they were added — what
    /// [`Self::play`] indexes.
    mesh_rows: Vec<usize>,
}

impl System {
    /// A system of `bodies` bodies, of which [`GROUND`] is one.
    ///
    /// # Panics
    ///
    /// If asked for no bodies at all, which is not a mechanism: ground always
    /// exists.
    #[must_use]
    pub fn new(bodies: usize) -> Self {
        assert!(bodies > 0, "ground is always a body");
        Self {
            bodies,
            rows: Vec::new(),
            mesh_rows: Vec::new(),
        }
    }

    #[must_use]
    pub const fn bodies(&self) -> usize {
        self.bodies
    }

    /// How many meshes have been added.
    #[must_use]
    pub fn meshes(&self) -> usize {
        self.mesh_rows.len()
    }

    /// Add a mesh. `None` where a body index is out of range, a tooth count is
    /// zero, or the row would not represent: all three are a caller's mistake
    /// rather than a design's, and none of them is a number to carry on with.
    pub fn mesh(&mut self, m: MeshRow) -> Option<()> {
        if m.a >= self.bodies || m.b >= self.bodies || m.frame >= self.bodies {
            return None;
        }
        if m.za == 0 || m.zb == 0 || m.a == m.b {
            return None;
        }
        let mut row = vec![Ratio::ZERO; self.bodies];
        let (za, zb) = (Ratio::whole(m.za), Ratio::whole(m.zb));
        row[m.a] = row[m.a].checked_add(za)?;
        row[m.b] = row[m.b].checked_add(zb)?;
        // **The frame's coefficient is what makes lock-up free.** Both members
        // are seen from it, so it carries the negative of their sum — and a
        // mesh whose frame is one of its own members (a gear rolling on a
        // carrier that is its own body) simply cancels there, which is the
        // degenerate case rather than a refusal.
        let sum = za.checked_add(zb)?;
        row[m.frame] = row[m.frame].checked_sub(sum)?;
        self.mesh_rows.push(self.rows.len());
        self.rows.push(row);
        Some(())
    }

    /// Rigidly connect two bodies: `ω_a − ω_b = 0`.
    ///
    /// A coaxial output coupling, a locked clutch and a train's shaft line are
    /// one mechanism, which is why there is one method. **Grounding is not one
    /// of them** — that is [`Condition::Ground`], a boundary rather than a
    /// structural edit, so a body can be held and released without the train
    /// changing shape.
    pub fn couple(&mut self, a: Body, b: Body) -> Option<()> {
        if a >= self.bodies || b >= self.bodies || a == b {
            return None;
        }
        let mut row = vec![Ratio::ZERO; self.bodies];
        row[a] = Ratio::ONE;
        row[b] = Ratio::ONE.checked_neg()?;
        self.rows.push(row);
        Some(())
    }

    /// **Whether a train locked solid turns as one body**, which it must.
    ///
    /// The all-ones vector satisfies a row exactly when the row's coefficients
    /// sum to zero, so this is a sum rather than a solve, and it has no
    /// tolerance in it — an identity that holds "to within 1e-12" would hide
    /// the fault it is for.
    ///
    /// **[`Self::mesh`] and [`Self::couple`] satisfy it by construction**, and
    /// the module header says what that does and does not buy: it is a guard on
    /// rows assembled any other way, not a check on the counts or the frame
    /// those constructors were handed. Neither a flipped sign nor a
    /// misattributed frame can break it, and it was written down here as though
    /// both could until the gate was run against the broken code.
    #[must_use]
    pub fn lock_up_is_free(&self) -> bool {
        self.rows.iter().all(|row| {
            row.iter()
                .try_fold(Ratio::ZERO, |acc, c| acc.checked_add(*c))
                .is_some_and(Ratio::is_zero)
        })
    }

    /// How many independent conditions this train needs, and which of its
    /// bodies nothing constrains.
    #[must_use]
    pub fn mobility(&self) -> Option<Mobility> {
        let rank = Reduced::of(&self.rows, self.bodies)?.pivots.len();
        Some(Mobility {
            degrees: self.bodies - rank,
            untouched: (0..self.bodies)
                .filter(|&j| self.rows.iter().all(|row| row[j].is_zero()))
                .collect(),
        })
    }

    /// **The speeds**, given what is asked of each body.
    ///
    /// `conditions` is one per body. Fewer conditions than the mobility is not
    /// an error: the answer is the family, and for a differential that family
    /// *is* the useful output — `ω_out = a ω_1 + b ω_2` in exact coefficients.
    ///
    /// # Errors
    ///
    /// [`Refusal::Conflicts`] naming the condition that contradicts what is
    /// already decided, or [`Refusal::Overflow`].
    pub fn motion(&self, conditions: &[Condition]) -> Result<Solution, Refusal> {
        self.solve(&vec![Ratio::ZERO; self.rows.len()], conditions, &[])
    }

    /// [`Self::motion`], with the conditions absorbed **in the order given**
    /// — `first` names bodies whose conditions go in before the rest, which
    /// are then taken in body order.
    ///
    /// The order changes no answer. What it changes is *which* condition a
    /// conflict is reported at: a condition is refused against what is
    /// already in, so the last one in is the one named, and a caller that
    /// puts the conventions in first has the designer's own statement named
    /// rather than the convention it contradicts.
    ///
    /// # Errors
    ///
    /// As [`Self::motion`].
    pub fn motion_in(&self, conditions: &[Condition], first: &[Body]) -> Result<Solution, Refusal> {
        self.solve(&vec![Ratio::ZERO; self.rows.len()], conditions, first)
    }

    /// **Where each body stands per unit of play in one mesh** — the same
    /// system with a load on it.
    ///
    /// `A θ = Δ` with `Δ` a single unit at mesh `which`, so the answer is the
    /// **coefficient** each body's angular play takes; the millimetres are the
    /// mesh's own and multiply it afterwards. That split is deliberate: the
    /// coefficients are quotients of tooth counts and exact, the play is a
    /// length and is not.
    ///
    /// # Errors
    ///
    /// As [`Self::motion`]; and `None` for a mesh index that does not exist.
    pub fn play(
        &self,
        which: usize,
        conditions: &[Condition],
    ) -> Option<Result<Solution, Refusal>> {
        let row = *self.mesh_rows.get(which)?;
        let mut rhs = vec![Ratio::ZERO; self.rows.len()];
        rhs[row] = Ratio::ONE;
        Some(self.solve(&rhs, conditions, &[]))
    }

    /// **The torque on every body**, from the ones that are known.
    ///
    /// For a lossless train the external torques do no net work over any motion
    /// the structure allows, so `Σ τ_i ω_i = 0` for every `ω` in the nullspace
    /// — which is to say `τ` lies in the **rowspace** of the same matrix. This
    /// is that, and nothing else: one transpose solve on a matrix already
    /// assembled, no second model.
    ///
    /// `applied` is one entry per body: `Some` where the torque is known — an
    /// input, a load, or the exact zero a free body carries — and `None` where
    /// it is the reaction being solved for. A held body is the ordinary
    /// `None`, and what comes back on it is **that body's own reaction**.
    ///
    /// **What is taken to ground is the sum over the held bodies**, not the
    /// entry at [`GROUND`]. A holding constraint is external, so its reaction
    /// appears on the body it holds; `GROUND` itself carries only what
    /// something *meshes* against it — everything, in a fixed-axis train, and
    /// exactly nothing in a pure epicyclic, where the held ring carries the lot.
    /// Both readings are right and they are different questions.
    ///
    /// The conditions are deliberately absent from this solve. Virtual work is
    /// taken over the motions the **structure** allows, and a holding condition
    /// is not structure — it is the thing whose reaction is being solved for.
    /// Adding its row here would put the answer on both sides of the equation.
    ///
    /// **Loss is not here**, and that is a statement rather than an omission: a
    /// mesh's efficiency depends on which way power crosses it, which makes the
    /// lossy problem non-linear in exactly the way this one is not. The ideal
    /// answer is the one that is exact, and it is what a lossy solve iterates
    /// from.
    ///
    /// # Errors
    ///
    /// [`Refusal::Conflicts`] naming the body whose given torque no
    /// equilibrium can carry — which is what *nothing reacts this load* looks
    /// like from here — or [`Refusal::Overflow`].
    pub fn torques(&self, applied: &[Option<Ratio>]) -> Result<Solution, Refusal> {
        // Unknowns are the row multipliers `c`, since `τ = Aᵀ c` is what
        // "lies in the rowspace" means. One equation per body whose torque is
        // given.
        let n = self.rows.len();
        let mut reduced = Reduced::new(n);
        let mut redundant = Vec::new();
        for (i, want) in applied.iter().enumerate() {
            let Some(want) = *want else { continue };
            let mut row: Vec<Ratio> = self.rows.iter().map(|r| r[i]).collect();
            row.push(want);
            match reduced.absorb(row)? {
                Insert::Added => {}
                Insert::Redundant => redundant.push(i),
                Insert::Conflicts => return Err(Refusal::Conflicts(i)),
            }
        }
        // `c`, then `τ = Aᵀ c` — for the particular solution and for each
        // direction the multipliers are still free in. A free direction that
        // maps to zero torque everywhere is not a freedom in the answer, and is
        // dropped: two different `c` giving one `τ` is the rowspace being
        // reached twice, not an ambiguity a reader should be shown.
        let c = reduced.solution(n);
        let project = |multipliers: &[Ratio]| -> Option<Vec<Ratio>> {
            (0..self.bodies)
                .map(|i| {
                    self.rows
                        .iter()
                        .zip(multipliers)
                        .try_fold(Ratio::ZERO, |acc, (r, m)| {
                            acc.checked_add(r[i].checked_mul(*m)?)
                        })
                })
                .collect()
        };
        let values = project(&c.values).ok_or(Refusal::Overflow)?;
        let mut residual = Vec::new();
        for r in &c.residual {
            let direction = project(&r.direction).ok_or(Refusal::Overflow)?;
            if let Some(at) = direction.iter().position(|v| !v.is_zero()) {
                let unit = direction[at];
                let direction: Option<Vec<Ratio>> =
                    direction.iter().map(|v| v.checked_div(unit)).collect();
                residual.push(Residual {
                    at,
                    direction: direction.ok_or(Refusal::Overflow)?,
                });
            }
        }
        Ok(Solution {
            values,
            residual,
            redundant,
        })
    }

    /// The structural rows with this right-hand side, then the conditions, one
    /// at a time so that the one that conflicts can be named.
    fn solve(
        &self,
        rhs: &[Ratio],
        conditions: &[Condition],
        first: &[Body],
    ) -> Result<Solution, Refusal> {
        let mut reduced = Reduced::new(self.bodies);
        for (row, b) in self.rows.iter().zip(rhs) {
            let mut augmented = row.clone();
            augmented.push(*b);
            // A structural row that is redundant with the others is an ordinary
            // fact about the topology — an idler loop, a doubled coupling — and
            // is not reported; one that *conflicts* is a system with no motion
            // at all, which only a right-hand side can produce.
            if reduced.absorb(augmented)? == Insert::Conflicts {
                return Err(Refusal::NoMotion);
            }
        }
        let mut redundant = Vec::new();
        let order = first
            .iter()
            .copied()
            .chain((0..conditions.len()).filter(|i| !first.contains(i)));
        for i in order {
            let Some(c) = conditions.get(i) else { continue };
            let value = match c {
                Condition::Free => continue,
                Condition::Ground => Ratio::ZERO,
                Condition::Drive(v) => *v,
            };
            let mut row = vec![Ratio::ZERO; self.bodies];
            row[i] = Ratio::ONE;
            row.push(value);
            match reduced.absorb(row)? {
                Insert::Added => {}
                Insert::Redundant => redundant.push(i),
                Insert::Conflicts => return Err(Refusal::Conflicts(i)),
            }
        }
        let mut out = reduced.solution(self.bodies);
        out.redundant = redundant;
        Ok(out)
    }
}

/// What adding a row to an already-reduced system did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Insert {
    /// It said something new, and is now a pivot.
    Added,
    /// It said nothing the system had not already said.
    Redundant,
    /// It said the opposite of what the system already says.
    Conflicts,
}

/// A system in reduced row echelon form, built one row at a time.
///
/// **One row at a time is the whole reason this is a type.** Reducing
/// everything at once says only *that* a system is over-determined; absorbing
/// each condition against what is already there says **which** condition
/// added nothing and which contradicted something, which is the difference
/// between a number a designer can act on and one they cannot.
#[derive(Clone, Debug)]
struct Reduced {
    /// Augmented rows: `width` coefficients and a right-hand side.
    rows: Vec<Vec<Ratio>>,
    /// The pivot column of each row, in the same order.
    pivots: Vec<usize>,
    width: usize,
}

impl Reduced {
    fn new(width: usize) -> Self {
        Self {
            rows: Vec::new(),
            pivots: Vec::new(),
            width,
        }
    }

    /// Reduce a whole set at once, for a caller that wants only the rank.
    fn of(rows: &[Vec<Ratio>], width: usize) -> Option<Self> {
        let mut out = Self::new(width);
        for row in rows {
            let mut augmented = row.clone();
            augmented.push(Ratio::ZERO);
            out.absorb(augmented).ok()?;
        }
        Some(out)
    }

    /// Take one augmented row into the system.
    fn absorb(&mut self, mut row: Vec<Ratio>) -> Result<Insert, Refusal> {
        let ov = || Refusal::Overflow;
        for (i, &p) in self.pivots.iter().enumerate() {
            let f = row[p];
            if f.is_zero() {
                continue;
            }
            for (cell, &above) in row.iter_mut().zip(&self.rows[i]) {
                *cell = cell
                    .checked_sub(f.checked_mul(above).ok_or_else(ov)?)
                    .ok_or_else(ov)?;
            }
        }
        let Some(pivot) = (0..self.width).find(|&c| !row[c].is_zero()) else {
            return Ok(if row[self.width].is_zero() {
                Insert::Redundant
            } else {
                Insert::Conflicts
            });
        };
        let unit = row[pivot];
        for cell in &mut row {
            *cell = cell.checked_div(unit).ok_or_else(ov)?;
        }
        // ...and back-substitute, so the system stays fully reduced and the
        // answer reads straight off the pivots.
        for existing in &mut self.rows {
            let f = existing[pivot];
            if f.is_zero() {
                continue;
            }
            for (cell, &new) in existing.iter_mut().zip(&row) {
                *cell = cell
                    .checked_sub(f.checked_mul(new).ok_or_else(ov)?)
                    .ok_or_else(ov)?;
            }
        }
        self.rows.push(row);
        self.pivots.push(pivot);
        Ok(Insert::Added)
    }

    /// The particular solution, with every free column at zero, and a basis for
    /// the free columns.
    fn solution(&self, width: usize) -> Solution {
        let mut values = vec![Ratio::ZERO; width];
        for (i, &p) in self.pivots.iter().enumerate() {
            values[p] = self.rows[i][width];
        }
        let residual = (0..width)
            .filter(|c| !self.pivots.contains(c))
            .map(|c| {
                let mut direction = vec![Ratio::ZERO; width];
                direction[c] = Ratio::ONE;
                for (i, &p) in self.pivots.iter().enumerate() {
                    // `checked_neg` cannot fail on a normalised value whose
                    // magnitude came out of this same elimination.
                    direction[p] = self.rows[i][c].checked_neg().unwrap_or(Ratio::ZERO);
                }
                Residual { at: c, direction }
            })
            .collect();
        Solution {
            values,
            residual,
            redundant: Vec::new(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::planetary::{self, Arrangement, PlanetaryShaft, Teeth};

    fn w(n: i64) -> Ratio {
        Ratio::whole(n)
    }

    /// An ordinary fixed-axis pair: two gears whose axes stand still in ground.
    ///
    /// Bodies `[ground, a, b]`.
    fn pair(za: i64, zb: i64) -> System {
        let mut s = System::new(3);
        s.mesh(MeshRow {
            a: 1,
            b: 2,
            za,
            zb,
            frame: GROUND,
        })
        .unwrap();
        s
    }

    /// A planetary set, **with no body held**: that is a condition, not a
    /// structure. Bodies `[ground, sun, carrier, ring, planet]`, and the
    /// ring's tooth count enters negative because a ring is a gear with a
    /// negative tooth count.
    fn set(t: Teeth) -> System {
        let mut s = System::new(5);
        s.mesh(MeshRow {
            a: 1,
            b: 4,
            za: i64::from(t.sun),
            zb: i64::from(t.planet),
            frame: 2,
        })
        .unwrap();
        s.mesh(MeshRow {
            a: 4,
            b: 3,
            za: i64::from(t.planet),
            zb: -i64::from(t.ring),
            frame: 2,
        })
        .unwrap();
        s
    }

    const SUN: Body = 1;
    const CARRIER: Body = 2;
    const RING: Body = 3;
    const PLANET: Body = 4;

    fn teeth() -> Teeth {
        Teeth {
            sun: 24,
            planet: 18,
            ring: 60,
        }
    }

    /// Conditions for a set: everything free, then ground held.
    fn base(n: usize) -> Vec<Condition> {
        let mut c = vec![Condition::Free; n];
        c[GROUND] = Condition::Ground;
        c
    }

    /// **Re-basing a family changes which body it is read per turn of, and
    /// nothing else.** A set with only its sun driven is a one-parameter
    /// family; the solver parameterises it at whichever column fell last, and
    /// re-based at the ring every answer the first family contained the
    /// second contains — evaluated at the ring speed the first gave — and its
    /// direction is a unit at the ring. Two-parameter too: nothing driven at
    /// all, re-based at the carrier and the ring.
    #[test]
    fn a_family_rebased_at_a_port_is_the_same_family() {
        let sys = set(teeth());
        let evaluate = |f: &Solution, p: &[Ratio]| -> Vec<Ratio> {
            (0..sys.bodies())
                .map(|i| {
                    f.residual.iter().zip(p).fold(f.values[i], |acc, (r, q)| {
                        acc.checked_add(r.direction[i].checked_mul(*q).unwrap())
                            .unwrap()
                    })
                })
                .collect()
        };
        for (conditions, preferred) in [
            (
                {
                    let mut c = base(5);
                    c[SUN] = Condition::Drive(Ratio::ONE);
                    c
                },
                vec![RING],
            ),
            (base(5), vec![CARRIER, RING]),
        ] {
            let first = sys.motion(&conditions).unwrap();
            assert_eq!(first.residual.len(), preferred.len());
            let second = first.rebased(&preferred).unwrap();
            for (k, &at) in preferred.iter().enumerate() {
                assert_eq!(second.residual[k].at, at);
                assert_eq!(second.residual[k].direction[at], Ratio::ONE);
                assert!(second.values[at].is_zero());
                for (other, r) in second.residual.iter().enumerate() {
                    assert!(
                        other == k || r.direction[at].is_zero(),
                        "{at} appears twice"
                    );
                }
            }
            for p in [[w(0), w(0)], [w(1), w(-3)], [w(5), w(2)]] {
                let answer = evaluate(&first, &p[..first.residual.len()]);
                let q: Vec<Ratio> = preferred.iter().map(|&at| answer[at]).collect();
                assert_eq!(evaluate(&second, &q), answer, "at {p:?}");
            }
        }
    }

    /// **A conflict is named at the last condition in**, and the order of
    /// absorption is the caller's: with the ring's hold put in first, holding
    /// the carrier beside it is what is refused, and the other way round the
    /// ring is.
    #[test]
    fn the_order_conditions_go_in_decides_which_one_a_conflict_names() {
        let sys = set(teeth());
        let mut c = base(5);
        c[SUN] = Condition::Drive(Ratio::ONE);
        c[CARRIER] = Condition::Ground;
        c[RING] = Condition::Ground;
        assert_eq!(
            sys.motion_in(&c, &[GROUND, RING, SUN]),
            Err(Refusal::Conflicts(CARRIER))
        );
        assert_eq!(
            sys.motion_in(&c, &[GROUND, CARRIER, SUN]),
            Err(Refusal::Conflicts(RING))
        );
        // ...and the order changes no answer where there is one.
        c[CARRIER] = Condition::Free;
        assert_eq!(sys.motion_in(&c, &[GROUND, RING, SUN]), sys.motion(&c));
    }

    /// **`mesh` and `couple` cannot build a row that locks up wrong**, at any
    /// signs and any frame — including a frame that is one of the mesh's own
    /// members.
    ///
    /// This is the *constructors'* guarantee, asserted rather than claimed. It
    /// is deliberately **not** described as catching a wrong sign or a
    /// misattributed frame, because it does not: the frame's coefficient is
    /// derived from the counts handed in, so the sum telescopes to zero
    /// whatever they are. That was found by flipping a ring's sign and watching
    /// this pass — see the module header, and
    /// `the_cross_check_is_what_catches_a_wrong_sign` below, which is the gate
    /// that does fire.
    #[test]
    fn no_constructor_here_can_build_a_row_that_locks_up_wrong() {
        let mut checked = 0;
        for (za, zb) in [(17, 43), (17, -43), (-17, 43), (1, 1), (-1, -1)] {
            assert!(pair(za, zb).lock_up_is_free(), "pair {za}/{zb}");
            checked += 1;
        }
        for t in [
            teeth(),
            Teeth {
                sun: 12,
                planet: 30,
                ring: 72,
            },
        ] {
            let mut s = set(t);
            assert!(s.lock_up_is_free(), "{t:?}");
            s.couple(RING, GROUND).unwrap();
            assert!(s.lock_up_is_free(), "{t:?} with the ring coupled");
            checked += 2;
        }
        // A mesh framed on one of its own members — the degenerate case, which
        // locks the two together and still sums to nothing.
        let mut odd = System::new(3);
        odd.mesh(MeshRow {
            a: 1,
            b: 2,
            za: 17,
            zb: 43,
            frame: 1,
        })
        .unwrap();
        assert!(odd.lock_up_is_free());
        checked += 1;
        assert_eq!(checked, 10);
    }

    /// **What actually catches a wrong sign**, run against the broken code
    /// rather than asserted.
    ///
    /// `CLAUDE.md`: before trusting a new gate, run it against the fault it is
    /// for. Doing that to the lock-up check found it silent — so the gate for a
    /// sign is the cross-check against the model being replaced, and this holds
    /// that it fires. A ring entered as a positive count turns an internal mesh
    /// into an external one, which is a different machine: the set's ratio with
    /// the ring held stops being `1 + z_r/z_s`.
    #[test]
    fn the_cross_check_is_what_catches_a_wrong_sign() {
        let t = teeth();
        let mut wrong = System::new(5);
        wrong
            .mesh(MeshRow {
                a: SUN,
                b: PLANET,
                za: i64::from(t.sun),
                zb: i64::from(t.planet),
                frame: CARRIER,
            })
            .unwrap();
        wrong
            .mesh(MeshRow {
                a: PLANET,
                b: RING,
                za: i64::from(t.planet),
                // The fault: a ring is a gear with a **negative** tooth count.
                zb: i64::from(t.ring),
                frame: CARRIER,
            })
            .unwrap();
        // Silent, and that is the finding.
        assert!(wrong.lock_up_is_free());

        let mut c = base(5);
        c[RING] = Condition::Ground;
        c[SUN] = Condition::Drive(Ratio::ONE);
        let got = wrong.motion(&c).unwrap().values[CARRIER];
        let right = set(t).motion(&c).unwrap().values[CARRIER];
        assert_eq!(
            right,
            Ratio::new(i128::from(t.sun), i128::from(t.sun + t.ring)).unwrap()
        );
        assert_ne!(got, right, "the cross-check has to be the thing that fires");
    }

    /// ...and the check **can** fail, which is the only thing that makes it a
    /// check at all — this project has three recorded that could not.
    ///
    /// What it fails on is the one fault it is actually for: a row that did not
    /// come from [`System::mesh`]. Reached here by writing into `rows`
    /// directly, because the constructor cannot produce one — which is the
    /// whole of what the invariant is worth, stated as a test rather than as a
    /// claim in a comment.
    #[test]
    fn the_lock_up_check_fails_on_a_row_with_the_frame_wrong() {
        let mut broken = System::new(3);
        // `z_a ω_a + z_b ω_b − z_a ω_f = 0` — the frame carrying one member's
        // count instead of both, which is the transposition a hand-written
        // mapping makes.
        broken
            .rows
            .push(vec![w(0).checked_sub(w(17)).unwrap(), w(17), w(43)]);
        assert!(!broken.lock_up_is_free());
    }

    /// A fixed-axis pair reverses, by exactly its tooth counts — and an
    /// internal one does not, from the same row with the sign the mesh kind
    /// already carries.
    #[test]
    fn a_fixed_axis_pair_is_minus_the_tooth_count_ratio() {
        let c = |s: &System| {
            let mut c = base(s.bodies());
            c[1] = Condition::Drive(Ratio::ONE);
            c
        };
        let ext = pair(17, 43);
        let m = ext.motion(&c(&ext)).unwrap();
        assert!(m.is_unique());
        assert_eq!(m.values[2], Ratio::new(-17, 43).unwrap());
        let int = pair(17, -43);
        let m = int.motion(&c(&int)).unwrap();
        assert_eq!(m.values[2], Ratio::new(17, 43).unwrap());
    }

    /// **The six arrangements, against the model they replace.**
    ///
    /// `planetary::power` solves Willis for three named roles with one body
    /// held; this solves the same set as a graph of four bodies and two meshes,
    /// with the holding as a condition. Agreeing on all six is what says the
    /// graph reproduces the kinematics — and the planet's own speed, which
    /// `power` has to be asked for separately, falls out of the same solve.
    #[test]
    fn the_graph_reproduces_every_planetary_arrangement() {
        let t = teeth();
        let s = set(t);
        let i0 = planetary::basic_ratio(t);
        let index = |m: PlanetaryShaft| match m {
            PlanetaryShaft::Sun => SUN,
            PlanetaryShaft::Carrier => CARRIER,
            PlanetaryShaft::Ring => RING,
        };
        let all = [
            PlanetaryShaft::Sun,
            PlanetaryShaft::Carrier,
            PlanetaryShaft::Ring,
        ];
        let mut checked = 0;
        for &input in &all {
            for &fixed in &all {
                if input == fixed {
                    continue;
                }
                let arrangement = Arrangement { input, fixed };
                let p = planetary::power(i0, arrangement, 1.0, 1.0, 1.0).unwrap();
                let mut c = base(s.bodies());
                c[index(fixed)] = Condition::Ground;
                c[index(input)] = Condition::Drive(Ratio::ONE);
                let m = s.motion(&c).unwrap();
                assert!(m.is_unique(), "{arrangement:?} should be determined");
                for &shaft in &all {
                    let want = p.speeds[shaft.index_pub()];
                    let got = m.values[index(shaft)].to_f64();
                    assert!(
                        (got - want).abs() < 1e-12,
                        "{arrangement:?} {shaft:?}: {got} vs {want}"
                    );
                }
                // ...and the planet, which `power` answers only when asked.
                let (absolute, relative) = p.planet_speed(t);
                let got = m.values[PLANET].to_f64();
                assert!(
                    (got - absolute).abs() < 1e-12,
                    "{arrangement:?} planet: {got} vs {absolute}"
                );
                let against = m.values[PLANET]
                    .checked_sub(m.values[CARRIER])
                    .unwrap()
                    .to_f64();
                assert!((against - relative).abs() < 1e-12);
                checked += 1;
            }
        }
        assert_eq!(checked, 6);
    }

    /// **A set with nothing held is a differential, and that is an answer.**
    ///
    /// Two degrees of freedom once ground is pinned, so one drive leaves
    /// one parameter, and the family that comes back *is* the differential's
    /// equation.
    ///
    /// # What is asserted, and what deliberately is not
    ///
    /// **Not which body the parameter is taken at.** A first draft assumed it
    /// would be the ring and read the particular solution as "the ring held" —
    /// the elimination left the *planet* free instead, so the particular
    /// solution is the planet standing still, the carrier came back at 4/7
    /// rather than 2/7, and the test failed against perfectly correct
    /// arithmetic. Which column an elimination leaves free is an implementation
    /// detail, and a test that reads it as a physical statement is testing the
    /// pivot order.
    ///
    /// So what is asserted is what is true of the family however it is
    /// presented: Willis holds at every point of it, and the member of it with
    /// the ring at rest is the classical set.
    #[test]
    fn an_unheld_set_answers_with_the_family_rather_than_refusing() {
        let t = teeth();
        let s = set(t);
        let m = s.mobility().unwrap();
        // Four bodies, two meshes — plus ground, which nothing in a pure
        // epicyclic touches and which is named rather than counted silently.
        assert_eq!(m.degrees, 3);
        assert_eq!(m.untouched, vec![GROUND]);

        let mut c = base(s.bodies());
        c[SUN] = Condition::Drive(Ratio::ONE);
        let f = s.motion(&c).unwrap();
        assert!(
            !f.is_unique(),
            "one drive on a two-degree set leaves one free"
        );
        assert_eq!(f.residual.len(), 1);
        let r = &f.residual[0];
        assert_eq!(r.direction[r.at], Ratio::ONE, "a residual is normalised");

        // `ω = particular + p · direction`, at three points of the family.
        let at = |p: Ratio, i: Body| {
            f.values[i]
                .checked_add(p.checked_mul(r.direction[i]).unwrap())
                .unwrap()
        };
        let (zs, zr) = (w(i64::from(t.sun)), w(i64::from(t.ring)));
        for p in [Ratio::ZERO, Ratio::ONE, Ratio::new(-5, 3).unwrap()] {
            // Willis, with the planet eliminated:
            // `z_s(ω_s − ω_c) + z_r(ω_r − ω_c) = 0`, the ring's count positive
            // — which is the relation `docs/reference.md#planetary-sets` states
            // the set's backlash in, holding here on its speeds.
            let carrier = at(p, CARRIER);
            let willis = zs
                .checked_mul(at(p, SUN).checked_sub(carrier).unwrap())
                .unwrap()
                .checked_add(
                    zr.checked_mul(at(p, RING).checked_sub(carrier).unwrap())
                        .unwrap(),
                )
                .unwrap();
            assert_eq!(willis, Ratio::ZERO, "Willis at p = {p}");
            assert_eq!(at(p, SUN), Ratio::ONE, "the drive is not a parameter");
        }

        // **And the classical set is a member of the family.** Choose the
        // parameter that brings the ring to rest; the carrier is then at
        // `z_s/(z_s + z_r)` of the sun, which is the ordinary reduction.
        let held = f.values[RING]
            .checked_neg()
            .unwrap()
            .checked_div(r.direction[RING])
            .expect("the ring moves with the parameter, so it can be brought to rest");
        assert_eq!(at(held, RING), Ratio::ZERO);
        assert_eq!(
            at(held, CARRIER),
            zs.checked_div(zs.checked_add(zr).unwrap()).unwrap()
        );
    }

    /// **Conditions that argue are named**, and ones that merely repeat
    /// themselves are reported without being an error.
    #[test]
    fn a_condition_that_contradicts_the_structure_names_itself() {
        let s = pair(17, 43);
        let mut c = base(s.bodies());
        c[1] = Condition::Drive(Ratio::ONE);
        // The second gear's speed is already decided; asking for a different
        // one is a contradiction, and asking for the right one is a repetition.
        let mut conflicting = c.clone();
        conflicting[2] = Condition::Drive(Ratio::ONE);
        assert_eq!(s.motion(&conflicting), Err(Refusal::Conflicts(2)));

        let mut agreeing = c.clone();
        agreeing[2] = Condition::Drive(Ratio::new(-17, 43).unwrap());
        let m = s.motion(&agreeing).unwrap();
        assert_eq!(m.redundant, vec![2]);
        assert!(m.is_unique());
    }

    /// **Torque is the transpose of the same matrix**, and the law it obeys is
    /// power balance rather than a formula anyone wrote down.
    ///
    /// Checked on a pair, where the answer is known — the wheel carries the
    /// tooth-count ratio and ground the sum — and asserted as the law
    /// `Σ τ ω = 0` on both.
    #[test]
    fn torque_balances_the_power_it_is_solved_from() {
        let s = pair(17, 43);
        let mut c = base(s.bodies());
        c[1] = Condition::Drive(Ratio::ONE);
        let speeds = s.motion(&c).unwrap();

        // Gear 1 is driven at 2; gear 2's and ground's are the reactions.
        let applied = vec![None, Some(w(2)), None];
        let t = s.torques(&applied).unwrap();
        assert!(t.is_unique());
        assert_eq!(t.values[2], Ratio::new(2 * 43, 17).unwrap());
        assert_eq!(t.values[GROUND], Ratio::new(-2 * 60, 17).unwrap());
        // The three sum to zero — the pair is in equilibrium — and the power
        // does too.
        let sum = t
            .values
            .iter()
            .try_fold(Ratio::ZERO, |a, v| a.checked_add(*v))
            .unwrap();
        assert_eq!(sum, Ratio::ZERO);
        let power = t
            .values
            .iter()
            .zip(&speeds.values)
            .try_fold(Ratio::ZERO, |a, (t, w)| a.checked_add(t.checked_mul(*w)?))
            .unwrap();
        assert_eq!(power, Ratio::ZERO);
    }

    /// The same law on an epicyclic set, in every arrangement — which is the
    /// case a pair cannot reach, because a pair has no third body for the
    /// reaction to be shared with.
    #[test]
    fn an_epicyclic_sets_torques_balance_in_every_arrangement() {
        let t = teeth();
        let s = set(t);
        let mut checked = 0;
        for fixed in [SUN, CARRIER, RING] {
            for input in [SUN, CARRIER, RING] {
                if input == fixed {
                    continue;
                }
                let mut c = base(s.bodies());
                c[fixed] = Condition::Ground;
                c[input] = Condition::Drive(Ratio::ONE);
                let speeds = s.motion(&c).unwrap();
                // The planet carries no external torque, and neither does the
                // ground: nothing outside the set touches either.
                let mut applied = vec![None; s.bodies()];
                applied[PLANET] = Some(Ratio::ZERO);
                applied[GROUND] = Some(Ratio::ZERO);
                applied[input] = Some(Ratio::ONE);
                let tq = s.torques(&applied).unwrap();
                assert!(tq.is_unique(), "input {input}, fixed {fixed}");
                let power = tq
                    .values
                    .iter()
                    .zip(&speeds.values)
                    .try_fold(Ratio::ZERO, |a, (t, w)| a.checked_add(t.checked_mul(*w)?))
                    .unwrap();
                assert_eq!(power, Ratio::ZERO, "input {input}, fixed {fixed}");
                // The three bodies' torques sum to zero: the set is in
                // equilibrium and the held body's is the reaction it carries.
                let sum = [SUN, CARRIER, RING]
                    .iter()
                    .try_fold(Ratio::ZERO, |a, &i| a.checked_add(tq.values[i]))
                    .unwrap();
                assert_eq!(sum, Ratio::ZERO, "input {input}, fixed {fixed}");
                checked += 1;
            }
        }
        assert_eq!(checked, 6);
    }

    /// **Backlash is the same matrix with a load on it.**
    ///
    /// A chain of two pairs: play in the first mesh reaches the output divided
    /// by everything downstream of it, which is `docs/reference.md#trains`'s
    /// accumulation — derived there for a chain, and here a consequence of the
    /// same rows the speeds came from.
    #[test]
    fn play_at_a_shaft_is_the_chain_referral_the_reference_derives() {
        // Bodies: ground, a, b, c — two meshes, `b` shared.
        let mut s = System::new(4);
        s.mesh(MeshRow {
            a: 1,
            b: 2,
            za: 17,
            zb: 43,
            frame: GROUND,
        })
        .unwrap();
        s.mesh(MeshRow {
            a: 2,
            b: 3,
            za: 13,
            zb: 31,
            frame: GROUND,
        })
        .unwrap();
        let mut c = base(4);
        // Hold the input; the play then appears at the output.
        c[1] = Condition::Ground;
        let first = s.play(0, &c).unwrap().unwrap();
        let second = s.play(1, &c).unwrap().unwrap();

        // **The first mesh's play reaches the output divided by everything
        // downstream of it** — `θ_out = j / Π_{j>k} i_j`, which is the
        // accumulation `docs/reference.md#trains` derives for a chain and which
        // here is a consequence of the same rows the speeds came from. The
        // second stage's ratio is 31/13, so the play at `c` is 13/31 of the
        // play at `b`, and negative because the mesh reverses.
        assert_eq!(
            first.values[3].checked_div(first.values[2]).unwrap(),
            Ratio::new(-13, 31).unwrap()
        );
        // ...and a mesh's play does not reach the body the other side of a
        // held input: the second mesh moves `c` and leaves `b` where it is.
        assert!(second.values[2].is_zero());
        assert_eq!(second.values[3], Ratio::new(1, 31).unwrap());

        // **The two are not comparable to each other**, and that is a property
        // of the reading rather than a shortcoming. A unit of `Δ` is one unit
        // of `z θ` in *that mesh's* counts, so `play` returns the coefficient a
        // mesh's own play is multiplied by; the millimetres that scale it are
        // the mesh's and are not exact. Comparing raw unit answers across two
        // meshes reads a ratio of 13/43 that means nothing — which a first
        // draft of this test asserted, and which is why this paragraph exists.
    }

    /// **A hula stage is two meshes on a crank**, and its reduction is the two
    /// integer products the reference states it in — exactly, and with `D = 0`
    /// a refusal rather than a very large number.
    #[test]
    fn a_hula_arrangement_reduces_by_its_two_products() {
        // Bodies: ground, gear 1, crank, wobble, gear 4. Both meshes are
        // internal, so the larger member of each carries the negative count.
        let build = |z: [i64; 4]| {
            let mut s = System::new(5);
            let (g1, crank, wobble, g4) = (1, 2, 3, 4);
            s.mesh(MeshRow {
                a: g1,
                b: wobble,
                za: -z[0],
                zb: z[1],
                frame: crank,
            })
            .unwrap();
            s.mesh(MeshRow {
                a: wobble,
                b: g4,
                za: z[2],
                zb: -z[3],
                frame: crank,
            })
            .unwrap();
            s
        };
        let shipped = build([65, 61, 57, 61]);
        assert!(shipped.lock_up_is_free());
        let mut c = base(5);
        c[1] = Condition::Ground;
        c[2] = Condition::Drive(Ratio::ONE);
        let m = shipped.motion(&c).unwrap();
        // Crank turns per turn of gear 4 — the reduction, `z₂z₄ / D`.
        let reduction = m.ratio(2, 4).unwrap();
        assert_eq!(reduction, Ratio::new(3721, 16).unwrap());

        // An arrangement whose two meshes cancel: the output cannot turn, so
        // the reduction is not a number. The graph says so by leaving gear 4
        // at zero while the crank turns, which no division rescues.
        let dead = build([60, 60, 60, 60]);
        let m = dead.motion(&c).unwrap();
        assert!(m.values[4].is_zero(), "a cancelling pair cannot turn");
        assert!(m.ratio(2, 4).is_none());
    }

    /// A body nothing touches is named rather than counted, on the case that
    /// produces it: a pure epicyclic, in which nothing meshes against ground until
    /// something is grounded to it.
    #[test]
    fn a_shaft_no_constraint_touches_is_named() {
        let mut s = set(teeth());
        assert_eq!(s.mobility().unwrap().untouched, vec![GROUND]);
        s.couple(RING, GROUND).unwrap();
        let m = s.mobility().unwrap();
        assert!(m.untouched.is_empty());
        assert_eq!(m.degrees, 2);
    }

    /// A mesh nobody could build is refused rather than assembled into a row
    /// that means something else.
    #[test]
    fn a_mesh_that_is_not_one_is_refused() {
        let mut s = System::new(3);
        assert!(s
            .mesh(MeshRow {
                a: 1,
                b: 9,
                za: 17,
                zb: 43,
                frame: GROUND
            })
            .is_none());
        assert!(s
            .mesh(MeshRow {
                a: 1,
                b: 2,
                za: 0,
                zb: 43,
                frame: GROUND
            })
            .is_none());
        assert!(s
            .mesh(MeshRow {
                a: 1,
                b: 1,
                za: 17,
                zb: 43,
                frame: GROUND
            })
            .is_none());
        assert!(s.couple(1, 1).is_none());
        assert_eq!(s.meshes(), 0);
    }
}
