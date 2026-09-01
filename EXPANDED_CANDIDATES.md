# Expanding Candidate Generation for Imperfect Gaps

## The Problem
Currently, the Advancing-Front algorithm generates new candidate positions strictly from the topology of the placement graph. When a sphere is placed at a candidate position defined by three constraints (e.g., three spheres), the algorithm generates three new candidates by taking pairs of the original constraints and combining them with the newly placed sphere. 

While this guarantees that the generated candidates form a continuous "front", it misses potential valid placements when gaps are left behind. If three spheres end up relatively close to each other—but were not generated from the same parent candidate—they will never be combined to form a candidate gap. This prevents the algorithm from filling in "potholes" or irregular gaps that naturally form during imperfect packing.

## The Mathematical Foundation
The current geometry solver (`solve_three_spheres` and `solve_two_spheres_one_plane` in `geometry.rs`) solves the Apollonius problem by finding a point `P` such that:
- `|P - C1| = r + r1`
- `|P - C2| = r + r2`
- `|P - C3| = r + r3`

Crucially, **the math does not require C1, C2, and C3 to be mutually touching.** As long as the three spheres are close enough that a fourth sphere of radius `r` can physically bridge the distance and touch all three, the mathematical intersection of the three distance equations remains valid and will yield a real position.

## Proposed Expansion: Spatial Proximity Candidates

To support filling gaps, we need to decouple candidate generation from the strict parent-child graph topology and incorporate **spatial proximity**. 

### 1. Spatial Acceleration Structure
Implement a lightweight spatial index (e.g., a uniform grid or octree) to quickly query placed constraints (spheres and walls) within a given bounding box. 

### 2. Proximity-Based Candidate Generation
When a new sphere `S_new` is placed, we do not *only* look at the constraints of the candidate that spawned it. Instead:
1. Query the spatial index for all placed spheres within a search radius `R_search`. A good heuristic for `R_search` is `2.5 * MAX_RADIUS`, ensuring we catch spheres across a typical gap.
2. Iterate through pairs of these nearby spheres `(S_A, S_B)`.
3. If `S_A`, `S_B`, and `S_new` form a triangle of mutually "close" spheres (their pairwise distances are within bounds that could plausibly fit another sphere), form a new candidate from `[S_A, S_B, S_new]`.
4. Run this candidate through the standard `solve_three_spheres` logic. If it produces a valid coordinate (discriminant `> 0` and within bin bounds), score it and push it to the `CandidateList`.

### 3. Wall Proximity
Similarly, if the newly placed sphere `S_new` is close to one or two bin walls, we can form new candidates by combining:
- `S_new`, a nearby wall, and a nearby sphere (`solve_two_spheres_one_plane`).
- `S_new` and two nearby walls (`solve_sphere_two_planes`).

### 4. Deduplication
Because this approach will generate overlapping combinations (e.g., `S_new` finds `S_A` and `S_B`, but later `S_A` might find `S_new` and `S_B`), the existing canonical candidate key deduplication in `CandidateList` (e.g. sorting constraint IDs and using a `HashMap` of seen keys) will effectively filter out redundant gap discoveries.

## Summary of Changes Needed
1. **Add Spatial Index**: Add a grid structure to `AdvancingFrontSpheres` to track `[Sphere]` by 3D cell.
2. **Modify `update_candidates`**: Change the logic to query the spatial index around the new placement rather than just inheriting constraints.
3. **Distance Heuristic**: Introduce a quick distance check before calling `solve_three_spheres` to prune combinations of spheres that are too far apart to mutually touch a new sphere.
