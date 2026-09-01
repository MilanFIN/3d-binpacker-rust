# Advancing-Front Sphere Packing Algorithm

## Overview

The current `FirstFitSpheres` solver in `src/solver/spheres/first_fit.rs` uses a brute-force
grid scan to find a valid placement for each sphere. For every candidate position on a 3D
lattice it checks collision against every placed sphere — O(grid³ × N) per sphere. This
document describes a replacement based on an **advancing-front** approach where a candidate
list is maintained and updated incrementally.

The key insight is to flip the question:

| Current (grid scan)                       | Advancing-front                               |
|-------------------------------------------|-----------------------------------------------|
| "Where in the bin could this sphere go?"  | "Here are all the known gaps right now."      |
| Scans all positions for each sphere       | Pops the best gap, places sphere, adds 3 new gaps |
| O(grid³ × N) per sphere                  | O(log N) pop + O(1) geometry per placement   |

---

## Core Data Structures

### `Constraint`

A constraint describes one object that bounds a candidate gap.  It is either a wall plane or
a previously placed sphere.

```rust
// src/solver/spheres/advancing_front.rs

pub enum Constraint {
    /// An axis-aligned wall.  Only one spatial dimension is fixed.
    Plane {
        axis:  Axis,   // X | Y | Z
        value: f32,    // coordinate of the inner face (0.0 for min-walls)
    },
    /// A placed sphere, identified by its index in the placement list.
    Sphere {
        index: usize,
    },
}

pub enum Axis { X, Y, Z }
```

### `Candidate`

A candidate represents a gap that is fully determined by exactly **three constraints**.
Three constraints are the minimum needed to pin a point in 3-D space (analogously to how
three tangent spheres/planes determine a unique fourth sphere position).

```rust
pub struct Candidate {
    /// Pre-computed sphere centre for a sphere of radius r placed here.
    /// Recomputed when the radius of the next sphere to place is known,
    /// or cached and invalidated when the radius changes significantly.
    pub position: Point3f,

    /// The three bounding objects.
    pub constraints: [Constraint; 3],

    /// Sorting key — lower is better (placed deep, low, left).
    pub score: f32,
}
```

**Canonical key** (for deduplication): sort the three constraint IDs and join them — e.g.
`"P(Y=0),P(X=0),S(4)"`.  Two candidates with the same sorted key are the same gap.

### `CandidateList`

```rust
pub struct CandidateList {
    candidates: BinaryHeap<Candidate>,         // min-heap on score
    seen:       HashMap<CandidateKey, f32>,    // key → best score seen
}
```

---

## Geometry: Computing a Candidate Position

Given constraints `[C0, C1, C2]` and the radius `r` of the sphere to place, the position is
solved as follows.

### Plane + Plane + Plane (corner)

```
pos = (r + plane_x.value, r + plane_y.value, r + plane_z.value)
```

### Sphere + Plane + Plane

Let sphere A have centre `cA` and radius `rA`.  The new sphere must be tangent to A
(`|pos - cA| = rA + r`) and touch two walls.  Two wall constraints fix two coordinates;
the remaining coordinate is solved from the tangency equation — choose the *smaller*
positive root so the sphere sits as close to the origin as possible.

### Sphere + Sphere + Plane

Two spheres A, B define a circle of candidate centres (the intersection locus at distance
`rA + r` from A and `rB + r` from B).  The wall constraint selects one or two points on
that circle; pick the one with the lower score.

The circle calculation:

1. Midpoint `M` between A and B along the line `A→B`.
2. Distance from A to M: `d = (|AB|² + (rA+r)² − (rB+r)²) / (2|AB|)`.
3. Radius of circle: `ρ = sqrt((rA+r)² − d²)`.
4. Normal to the circle = unit vector `AB`.
5. Choose a perpendicular axis (e.g. the cross product of `AB` with `world-up`).
6. The wall constraint substitutes the fixed coordinate and solves for the angle.

### Sphere + Sphere + Sphere

Classic Apollonius / three-sphere intersection:

1. Set up a local coordinate frame with origin at A, x-axis toward B, y-axis in the
   plane of ABC.
2. Solve the 2×2 linear system for the in-plane position.
3. Lift back to 3-D — two solutions (above/below the plane); pick the one with `y ≥ r`
   (inside the bin, lower first).

---

## Algorithm

### Initialisation

```
fn initialize_candidates(bin: &Bin) -> CandidateList {
    // Seed with the single triple-wall corner.
    // For a bin with walls at X=0, Y=0, Z=0 the first candidate is:
    //   constraints: [Plane(X=0), Plane(Y=0), Plane(Z=0)]
    //   position:    (r, r, r)   -- computed on demand
    // Additional edge candidates (two walls) are also seeded so that the
    // algorithm can fill strips along the walls before moving inward.
}
```

The six faces of the bin contribute `C(6,2) = 15` edge pairs and `C(6,3) = 20` corner
triples, but only the **inner-facing** combinations matter, giving a small fixed seed set.

### Main Loop

```
fn pack(bin: &Bin, spheres: &[Sphere]) -> PackResult<Sphere> {
    let mut placements: Vec<Sphere> = Vec::new();
    let mut candidates = initialize_candidates(bin);

    for sphere in spheres {
        // 1. Find the best valid candidate for this sphere's radius.
        let candidate = candidates
            .iter_scored(sphere.radius)
            .find(|c| is_valid(c, sphere, &placements, bin))?;

        // 2. Record the placement.
        let mut placed = sphere.clone();
        placed.position = candidate.position;
        placements.push(placed.clone());

        // 3. Update the candidate list.
        update_candidates(&mut candidates, &candidate, placements.len() - 1, &placements, bin);
    }

    PackResult::new(vec![], 0.0, vec![placements])
}
```

### Validity Check

A candidate position is valid for sphere of radius `r` if:

1. `pos.x - r ≥ 0`, `pos.y - r ≥ 0`, `pos.z - r ≥ 0`  (inside min walls)
2. `pos.x + r ≤ bin.w`, `pos.y + r ≤ bin.h`, `pos.z + r ≤ bin.d` (inside max walls)
3. For every placed sphere P: `|pos - P.position| ≥ r + P.radius` (no collision)

Checks 1 and 2 are O(1). Check 3 is O(placed) but in practice the number of nearby
spheres is small and can be bounded with a spatial index (e.g. a grid).

### Candidate Update

```
fn update_candidates(
    candidates:   &mut CandidateList,
    consumed:     &Candidate,
    new_idx:      usize,
    placements:   &[Sphere],
    bin:          &Bin,
) {
    candidates.remove(consumed);

    // For each pair from the consumed candidate's 3 constraints,
    // form a new candidate with the newly placed sphere as the third.
    let [c0, c1, c2] = &consumed.constraints;
    for pair in [[c0, c1], [c0, c2], [c1, c2]] {
        let new_candidate = make_candidate(
            pair[0].clone(),
            pair[1].clone(),
            Constraint::Sphere { index: new_idx },
        );
        if let Some(c) = new_candidate {
            candidates.add(c);   // deduplicates via canonical key
        }
    }
}
```

Each placement removes 1 candidate and adds at most 3, keeping the list size bounded.

---

## Scoring

```
score(candidate) = alpha * (pos.y)          // prefer low (gravity)
                 + beta  * (pos.x + pos.z)  // prefer front-left
                 - gamma * touching_count   // reward dense packing
```

`touching_count` is the number of constraints that are placed spheres (0–3).  A candidate
touching three spheres is maximally dense and should be strongly preferred.

---

## Integration with the Existing Codebase

| Aspect | Detail |
|--------|--------|
| **Trait** | Implement `Solver<Sphere, Bin>` from `solver_interface.rs`. |
| **New file** | `src/solver/spheres/advancing_front.rs` |
| **Export** | Add `pub mod advancing_front;` in `src/solver/spheres/mod.rs`. |
| **No new deps** | All geometry is plain `f32` arithmetic; no extra crates needed. |
| **`PackResult`** | Reuse existing `PackResult<Sphere>` with a single bin `vec![placements]`. |
| **`Point3f`** | Use directly; add helper methods (`sub`, `dot`, `cross`, `len`) inline or in `point3f.rs`. |

---

## File Layout (Proposed)

```
src/solver/spheres/
├── mod.rs                 (add: pub mod advancing_front)
├── first_fit.rs           (keep as fallback / reference)
└── advancing_front.rs     (NEW)
    ├── Axis
    ├── Constraint
    ├── Candidate
    ├── CandidateKey
    ├── CandidateList
    ├── geometry::solve_position(constraints, radius) -> Option<Point3f>
    ├── AdvancingFrontSpheres (implements Solver<Sphere, Bin>)
    │   ├── initialize_candidates
    │   ├── is_valid
    │   └── update_candidates
    └── #[cfg(test)] mod tests
```

---

## Open Questions / Future Work

- **Multi-radius reuse** – When all spheres have the same radius, candidate positions can
  be computed once and reused.  With mixed radii a candidate's position must be re-solved
  per sphere; caching the last-used radius avoids redundant work.
- **Spatial index** – For large N, replace the O(N) collision scan with an octree or
  uniform-grid acceleration structure.
- **Multiple bins** – The current `PackResult` structure already supports `bins: Vec<Vec<Sphere>>`.
  When a sphere cannot be placed in the current bin, open a new one with a fresh candidate list.
- **Genetic optimizer** – The advancing-front solver is deterministic, but the *order* in
  which spheres are presented drives quality.  The existing genetic optimizer in
  `src/optimizer/` can continue to permute sphere order and score via this solver.
