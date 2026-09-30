//! **The train graph's standard structures, each written once** with its
//! reference (`work/design-graph.md`, *The algorithms, each once*): the
//! graph is a known object, and what is asked of it — which pieces hang
//! together, which hangs from which — has a textbook answer that is used
//! here rather than re-derived at each site that needs it.
//!
//! Integers and indices only: nothing here compares a float, so nothing
//! here has a tolerance. Each structure is held by its tests to a
//! brute-force oracle that shares no code with it.

/// **Disjoint sets** over `0..n` (union–find): union by rank and full path
/// compression, after Tarjan, "Efficiency of a good but not linear set
/// union algorithm", *J. ACM* 22 (1975) 215–225.
///
/// What every part of the train graph that groups by connection reads —
/// the parts, the mesh groups, what a search moves together — so a
/// grouping is stated once. Which element stands for a set is the
/// structure's own business: callers read a set through [`Self::labels`]
/// or [`Self::components`], which number the sets **in the order of each
/// set's first element**. That order is load-bearing: a part, a mesh group
/// and a search component are numbered by it, and a train's pieces are
/// listed in it.
#[derive(Clone, Debug)]
pub struct DisjointSets {
    parent: Vec<usize>,
    /// How many sets there are: `n`, less one per union that merged two.
    sets: usize,
    /// An upper bound on each root's height. A root of rank `r` holds at
    /// least `2^r` elements, so a rank never exceeds `log₂ n < 64`.
    rank: Vec<u8>,
}

impl DisjointSets {
    /// `n` elements, each a set of its own.
    #[must_use]
    pub fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            sets: n,
            rank: vec![0; n],
        }
    }

    /// How many elements there are.
    fn len(&self) -> usize {
        self.parent.len()
    }

    /// How many sets there are.
    #[must_use]
    pub fn count(&self) -> usize {
        self.sets
    }

    /// **The element that stands for `i`'s set**, every element on the way
    /// to it pointed straight at it (full path compression).
    pub fn find(&mut self, i: usize) -> usize {
        let mut root = i;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut at = i;
        while self.parent[at] != root {
            at = std::mem::replace(&mut self.parent[at], root);
        }
        root
    }

    /// **`a`'s set and `b`'s made one**, the shorter tree hung under the
    /// taller (union by rank). Whether they were two.
    pub fn union(&mut self, a: usize, b: usize) -> bool {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra == rb {
            return false;
        }
        let (low, high) = if self.rank[ra] < self.rank[rb] {
            (ra, rb)
        } else {
            (rb, ra)
        };
        self.parent[low] = high;
        if self.rank[low] == self.rank[high] {
            self.rank[high] += 1;
        }
        self.sets -= 1;
        true
    }

    /// **Each element's set, numbered from 0 in the order of each set's
    /// first element** — element 0's set is 0, the next element not in it
    /// starts set 1, and so on.
    pub fn labels(&mut self) -> Vec<usize> {
        let mut of_root: Vec<Option<usize>> = vec![None; self.len()];
        let mut numbered = 0;
        let labels = (0..self.len())
            .map(|i| {
                let root = self.find(i);
                *of_root[root].get_or_insert_with(|| {
                    numbered += 1;
                    numbered - 1
                })
            })
            .collect();
        debug_assert_eq!(numbered, self.sets, "every set numbered once");
        labels
    }

    /// **The sets**, each its elements in ascending order, in the order of
    /// each set's first element ([`Self::labels`]).
    pub fn components(&mut self) -> Vec<Vec<usize>> {
        let mut out: Vec<Vec<usize>> = Vec::new();
        for (i, label) in self.labels().into_iter().enumerate() {
            if label == out.len() {
                out.push(Vec::new());
            }
            out[label].push(i);
        }
        out
    }
}

/// **Where a vertex hangs** in a [`CarrierTree`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hang {
    /// From the root, `depth` parents up — the root at 0.
    Root { depth: usize },
    /// From a vertex with no parent that is not the root.
    Nothing,
    /// From a cycle of parents, or on one.
    Cycle,
}

/// **A rooted tree given by parent pointers** — the turning pairs of a
/// gear train, each link hung from the carrier of the axis it turns about
/// and ground the root, which is Buchsbaum and Freudenstein's tree rule
/// ("Synthesis of kinematic structure of geared kinematic chains and other
/// mechanisms", *J. Mechanisms* 5 (1970) 357–392): every link reaches
/// ground by exactly one chain of carriers.
///
/// Built from any parent list, so it names what breaks the rule rather
/// than assuming it: a vertex whose parents stop short of the root hangs
/// from nothing, and one whose parents go round hangs from a cycle
/// ([`Hang`]). Vertex 0 is the root.
#[derive(Clone, Debug)]
pub struct CarrierTree {
    parent: Vec<Option<usize>>,
    hang: Vec<Hang>,
}

impl CarrierTree {
    /// The tree whose vertex `v` hangs from `parent[v]` — `None` for no
    /// parent; the root's own entry is not read. A parent out of range is
    /// no parent. Each vertex is walked up once: `O(n)`.
    #[must_use]
    pub fn new(parent: Vec<Option<usize>>) -> Self {
        let n = parent.len();
        let parent: Vec<Option<usize>> = parent
            .into_iter()
            .enumerate()
            .map(|(v, p)| if v == 0 { None } else { p.filter(|&p| p < n) })
            .collect();
        let mut hang: Vec<Option<Hang>> = vec![None; n];
        if n > 0 {
            hang[0] = Some(Hang::Root { depth: 0 });
        }
        // Each walk goes up until it meets a vertex already decided, the
        // end of the parents, or itself; every vertex it passed hangs as
        // that end does. `on_walk` marks the vertices of the walk under way.
        let mut on_walk = vec![false; n];
        for start in 0..n {
            let mut chain: Vec<usize> = Vec::new();
            let mut at = start;
            let end = loop {
                if let Some(h) = hang[at] {
                    break h;
                }
                if on_walk[at] {
                    break Hang::Cycle;
                }
                on_walk[at] = true;
                chain.push(at);
                match parent[at] {
                    Some(p) => at = p,
                    None => break Hang::Nothing,
                }
            };
            for (up, &v) in chain.iter().rev().enumerate() {
                hang[v] = Some(match end {
                    Hang::Root { depth } => Hang::Root {
                        depth: depth + up + 1,
                    },
                    other => other,
                });
                on_walk[v] = false;
            }
        }
        // Every vertex is decided by the walk from it, if not before.
        debug_assert!(hang.iter().all(Option::is_some), "an undecided vertex");
        Self {
            parent,
            hang: hang.into_iter().map(|h| h.unwrap_or(Hang::Cycle)).collect(), // absence: none undecided (asserted)
        }
    }

    /// Where `v` hangs.
    #[must_use]
    pub fn hang(&self, v: usize) -> Hang {
        self.hang[v]
    }

    /// **`v` and the vertices above it**, up to the root (not included),
    /// or to where the parents stop or come round again — each once.
    pub fn lineage(&self, v: usize) -> impl Iterator<Item = usize> + '_ {
        let mut seen: Vec<usize> = Vec::new();
        std::iter::successors(Some(v), move |&at| self.parent[at]).take_while(move |&at| {
            let fresh = at != 0 && !seen.contains(&at);
            seen.push(at);
            fresh
        })
    }
}

#[cfg(test)]
mod tests {
    //! Each structure against a brute-force oracle sharing no code with it.

    // The oracles index as the textbook states them.
    #![allow(clippy::needless_range_loop)]

    use super::*;

    /// **Connectivity by transitive closure** (Warshall): `reach[a][b]`
    /// where a chain of the given pairs joins `a` to `b`, every element
    /// reaching itself.
    fn closure(n: usize, pairs: &[(usize, usize)]) -> Vec<Vec<bool>> {
        let mut reach = vec![vec![false; n]; n];
        for (i, row) in reach.iter_mut().enumerate() {
            row[i] = true;
        }
        for &(a, b) in pairs {
            reach[a][b] = true;
            reach[b][a] = true;
        }
        for k in 0..n {
            for i in 0..n {
                for j in 0..n {
                    if reach[i][k] && reach[k][j] {
                        reach[i][j] = true;
                    }
                }
            }
        }
        reach
    }

    /// The oracle's labels: an element takes the label of the first element
    /// it reaches, renumbered in order of first appearance.
    fn oracle_labels(reach: &[Vec<bool>]) -> Vec<usize> {
        let first: Vec<usize> = (0..reach.len())
            .map(|i| (0..reach.len()).find(|&j| reach[i][j]).unwrap_or(i))
            .collect();
        let mut seen: Vec<usize> = Vec::new();
        first
            .iter()
            .map(|&f| match seen.iter().position(|&s| s == f) {
                Some(p) => p,
                None => {
                    seen.push(f);
                    seen.len() - 1
                }
            })
            .collect()
    }

    /// Every pair of `0..n`.
    fn all_pairs(n: usize) -> Vec<(usize, usize)> {
        (0..n)
            .flat_map(|a| (a + 1..n).map(move |b| (a, b)))
            .collect()
    }

    /// Checks one sequence of unions against the oracle, and the structure's
    /// own invariants after it: every element points at its root once found
    /// (full compression), and no root is taller than its set allows (union
    /// by rank). Returns how many checks ran.
    fn holds(n: usize, pairs: &[(usize, usize)]) -> usize {
        let mut sets = DisjointSets::new(n);
        let mut merged = 0;
        for &(a, b) in pairs {
            merged += usize::from(sets.union(a, b));
        }
        let reach = closure(n, pairs);
        let labels = oracle_labels(&reach);
        // A set per element that reaches no element before it.
        let count = (0..n).filter(|&i| (0..i).all(|j| !reach[i][j])).count();
        let context = format!("n {n}, unions {pairs:?}");
        assert_eq!(merged, n - count, "{context}: a union merged what was one");
        assert_eq!(sets.count(), count, "{context}: count");
        let mut checks = 1;
        for a in 0..n {
            for b in 0..n {
                assert_eq!(
                    sets.find(a) == sets.find(b),
                    reach[a][b],
                    "{context}: {a} ~ {b}"
                );
                checks += 1;
            }
        }
        assert_eq!(sets.labels(), labels, "{context}: labels");
        let components = sets.components();
        assert_eq!(components.len(), count, "{context}: components");
        for (c, members) in components.iter().enumerate() {
            let expected: Vec<usize> = (0..n).filter(|&i| labels[i] == c).collect();
            assert_eq!(*members, expected, "{context}: component {c}");
        }
        for i in 0..n {
            let root = sets.find(i);
            assert_eq!(sets.parent[i], root, "{context}: {i} not compressed");
            let size = (0..n).filter(|&j| reach[i][j]).count();
            assert!(
                1usize << sets.rank[root] <= size,
                "{context}: rank {} over {size} elements",
                sets.rank[root]
            );
        }
        checks
    }

    /// **Disjoint sets are connectivity**, exhaustively: every set of pairs
    /// over up to five elements, each in its own order and reversed —
    /// which ends in different trees — against the transitive closure.
    #[test]
    fn disjoint_sets_are_connectivity_on_every_small_graph() {
        let mut graphs = 0;
        for n in 0..=5 {
            let pairs = all_pairs(n);
            for mask in 0u32..1 << pairs.len() {
                let chosen: Vec<(usize, usize)> = pairs
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| (mask >> k) & 1 == 1)
                    .map(|(_, &p)| p)
                    .collect();
                holds(n, &chosen);
                let reversed: Vec<(usize, usize)> =
                    chosen.iter().rev().map(|&(a, b)| (b, a)).collect();
                holds(n, &reversed);
                graphs += 1;
            }
        }
        // 1 + 1 + 2 + 8 + 64 + 1024 edge sets over 0..=5 elements.
        assert_eq!(graphs, 1 + 1 + 2 + 8 + 64 + 1024);
    }

    /// **And on larger ones**: seeded random unions over up to twelve
    /// elements, a chain that grows one tall tree, and every element
    /// unioned with itself.
    #[test]
    fn disjoint_sets_are_connectivity_on_random_graphs() {
        let mut rng = super::super::sweep::Lcg(0x9e37_79b9_7f4a_7c15);
        let mut next = |bound: usize| rng.pick(bound);
        let mut runs = 0;
        for n in 6..=12 {
            for _ in 0..200 {
                let k = next(2 * n);
                let pairs: Vec<(usize, usize)> = (0..k).map(|_| (next(n), next(n))).collect();
                holds(n, &pairs);
                runs += 1;
            }
            let chain: Vec<(usize, usize)> = (1..n).map(|i| (i - 1, i)).collect();
            holds(n, &chain);
            let itself: Vec<(usize, usize)> = (0..n).map(|i| (i, i)).collect();
            holds(n, &itself);
        }
        assert_eq!(runs, 7 * 200);
    }

    /// **Where a vertex hangs, by walking its parents** — at most `n` steps,
    /// sharing nothing with the tree's memo: the root reached (at the steps
    /// taken), a vertex with no parent, or `n` steps without either, which
    /// is a cycle.
    fn walked(parent: &[Option<usize>], v: usize) -> Hang {
        let n = parent.len();
        let mut at = v;
        for depth in 0..=n {
            if at == 0 {
                return Hang::Root { depth };
            }
            match parent[at].filter(|&p| p < n) {
                Some(p) => at = p,
                None => return Hang::Nothing,
            }
        }
        Hang::Cycle
    }

    /// **The tree hangs every vertex where walking its parents does**, and
    /// its lineage is the walk: every parent list over up to five vertices,
    /// each parent any vertex, none, or one out of range — the rooted
    /// trees, the forests hanging from nothing, the self-parents and every
    /// cycle among them.
    #[test]
    fn a_carrier_tree_hangs_every_vertex_where_its_parents_lead() {
        let mut lists = 0;
        for n in 1..=5usize {
            // Each vertex but the root: a parent in 0..n, none, or n.
            let choices = n + 2;
            let total = choices.pow(u32::try_from(n - 1).unwrap());
            for code in 0..total {
                let mut rest = code;
                let parent: Vec<Option<usize>> = (0..n)
                    .map(|v| {
                        if v == 0 {
                            return None;
                        }
                        let c = rest % choices;
                        rest /= choices;
                        match c {
                            c if c < n => Some(c),
                            c if c == n => None,
                            _ => Some(n),
                        }
                    })
                    .collect();
                let tree = CarrierTree::new(parent.clone());
                for v in 0..n {
                    assert_eq!(tree.hang(v), walked(&parent, v), "{parent:?}: vertex {v}");
                    let lineage: Vec<usize> = tree.lineage(v).collect();
                    let mut expected = Vec::new();
                    let mut at = Some(v);
                    while let Some(x) = at.filter(|&x| x != 0 && x < n && !expected.contains(&x)) {
                        expected.push(x);
                        at = parent[x];
                    }
                    assert_eq!(lineage, expected, "{parent:?}: lineage of {v}");
                }
                lists += 1;
            }
        }
        // (n + 2)^(n − 1) lists for n = 1..=5.
        assert_eq!(lists, 1 + 4 + 25 + 216 + 2401);
    }

    /// **The sets are numbered by their first element, not by the element
    /// that stands for them** — the near miss a numbering by root would
    /// make: the union that hangs 0's set under 3 still leaves 0's set
    /// first.
    #[test]
    fn sets_are_numbered_by_their_first_element() {
        let mut sets = DisjointSets::new(5);
        sets.union(3, 4);
        sets.union(3, 0);
        sets.union(1, 2);
        assert_ne!(sets.find(0), 0, "the fixture hangs 0 under another root");
        assert_eq!(sets.labels(), vec![0, 1, 1, 0, 0]);
        assert_eq!(sets.components(), vec![vec![0, 3, 4], vec![1, 2]]);
    }
}
