use std::collections::HashMap;

use crate::common::bin::Bin;
use crate::common::pack_result::PackResult;
use crate::common::point3f::Point3f;
use crate::common::sphere_spec::Sphere;
use crate::solver::common::solver_properties::SolverProperties;
use crate::solver::solver_interface::Solver;

use super::geometry::{self, Axis, EPS};

// ---------------------------------------------------------------------------
// Constraint
// ---------------------------------------------------------------------------

/// A constraint bounding a candidate gap: either an axis-aligned bin wall or
/// a previously placed sphere.
#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    Plane { axis: Axis, value: f32 },
    Sphere { index: usize },
}

impl Constraint {
    /// Canonical sort key — planes sort before spheres, then by axis / index.
    fn sort_key(&self) -> (u8, u32, u32) {
        match self {
            Constraint::Plane { axis, .. } => (0, *axis as u32, 0),
            Constraint::Sphere { index } => (1, *index as u32, 0),
        }
    }
}

// ---------------------------------------------------------------------------
// Candidate
// ---------------------------------------------------------------------------

/// A canonical key for deduplication.
type CandidateKey = String;

fn make_key(constraints: &[Constraint; 3]) -> CandidateKey {
    let mut keys: Vec<String> = constraints
        .iter()
        .map(|c| match c {
            Constraint::Plane { axis, value } => format!("P({:?},{})", axis, *value as i32),
            Constraint::Sphere { index } => format!("S({})", index),
        })
        .collect();
    keys.sort();
    keys.join(",")
}

/// A gap defined by exactly three constraints where a sphere could be placed.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub position: Point3f,
    pub constraints: [Constraint; 3],
    pub score: f32,
    pub radius: f32,
}

// ---------------------------------------------------------------------------
// CandidateList
// ---------------------------------------------------------------------------

pub struct CandidateList {
    candidates: Vec<Candidate>,
    seen: HashMap<CandidateKey, usize>, // key -> index in candidates
}

impl CandidateList {
    fn new() -> Self {
        Self {
            candidates: Vec::new(),
            seen: HashMap::new(),
        }
    }

    fn initialize(bin: &Bin, r: f32) -> Self {
        let mut list = Self::new();
        let walls = vec![
            Constraint::Plane { axis: Axis::X, value: 0.0 },
            Constraint::Plane { axis: Axis::X, value: bin.w },
            Constraint::Plane { axis: Axis::Y, value: 0.0 },
            Constraint::Plane { axis: Axis::Y, value: bin.h },
            Constraint::Plane { axis: Axis::Z, value: 0.0 },
            Constraint::Plane { axis: Axis::Z, value: bin.d },
        ];

        let n = walls.len();
        for i in 0..n {
            for j in (i + 1)..n {
                for k in (j + 1)..n {
                    let triple = [
                        walls[i].clone(),
                        walls[j].clone(),
                        walls[k].clone(),
                    ];
                    if !AdvancingFrontSpheres::is_valid_triple(&triple) {
                        continue;
                    }
                    if let Some(pos) = AdvancingFrontSpheres::solve_candidate(&triple, r, &[], bin) {
                        let score = AdvancingFrontSpheres::compute_score(&pos, 0);
                        list.add(Candidate {
                            position: pos,
                            constraints: triple,
                            score,
                            radius: r,
                        });
                    }
                }
            }
        }
        list
    }

    fn update(
        &mut self,
        consumed: &Candidate,
        new_idx: usize,
        placements: &[Sphere],
        bin: &Bin,
        r: f32,
    ) {
        let key = make_key(&consumed.constraints);
        if let Some(&idx) = self.seen.get(&key) {
            self.remove_index(idx);
        }

        let new_c = Constraint::Sphere { index: new_idx };
        let [c0, c1, c2] = &consumed.constraints;

        let pairs = [
            [c0.clone(), c1.clone()],
            [c0.clone(), c2.clone()],
            [c1.clone(), c2.clone()],
        ];

        for pair in pairs {
            let triple = [pair[0].clone(), pair[1].clone(), new_c.clone()];
            if !AdvancingFrontSpheres::is_valid_triple(&triple) {
                continue;
            }
            if let Some(pos) = AdvancingFrontSpheres::solve_candidate(&triple, r, placements, bin) {
                let touching = triple
                    .iter()
                    .filter(|c| matches!(c, Constraint::Sphere { .. }))
                    .count();
                let score = AdvancingFrontSpheres::compute_score(&pos, touching);
                self.add(Candidate {
                    position: pos,
                    constraints: triple,
                    score,
                    radius: r,
                });
            }
        }
    }

    fn add(&mut self, c: Candidate) {
        let key = make_key(&c.constraints);
        if let Some(&existing_idx) = self.seen.get(&key) {
            if c.score < self.candidates[existing_idx].score {
                self.candidates[existing_idx] = c;
            }
        } else {
            let idx = self.candidates.len();
            self.seen.insert(key, idx);
            self.candidates.push(c);
        }
    }

    fn remove_index(&mut self, idx: usize) {
        let key = make_key(&self.candidates[idx].constraints);
        self.seen.remove(&key);
        self.candidates.swap_remove(idx);
        if idx < self.candidates.len() {
            let swapped_key = make_key(&self.candidates[idx].constraints);
            self.seen.insert(swapped_key, idx);
        }
    }

    fn is_empty(&self) -> bool {
        self.candidates.is_empty()
    }

    fn sort(&mut self) {
        self.candidates.sort_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.seen.clear();
        for (i, c) in self.candidates.iter().enumerate() {
            self.seen.insert(make_key(&c.constraints), i);
        }
    }
}

// ---------------------------------------------------------------------------
// AdvancingFrontSpheres
// ---------------------------------------------------------------------------

pub struct AdvancingFrontSpheres {
    bin_template: Option<Bin>,
    weight_limit: f32,
    _growing_bin: bool,
}

impl Default for AdvancingFrontSpheres {
    fn default() -> Self {
        Self {
            bin_template: None,
            weight_limit: 0.0,
            _growing_bin: false,
        }
    }
}

impl Solver<Sphere, Bin> for AdvancingFrontSpheres {
    fn init(&mut self, properties: &SolverProperties<Bin>) {
        self.bin_template = Some(properties.bin.clone());
        self.weight_limit = properties.weight;
        self._growing_bin = properties.growing_bin;
    }

    fn solve(&mut self, spheres: &[Sphere]) -> PackResult<Sphere> {
        let bin = self.bin_template.clone().unwrap();
        let mut result_bins: Vec<Vec<Sphere>> = Vec::new();
        let mut bin_candidates: Vec<CandidateList> = Vec::new();

        'outer: for sphere in spheres {
            // Try existing bins
            for bin_idx in 0..result_bins.len() {
                let candidates = &mut bin_candidates[bin_idx];
                let placements = &mut result_bins[bin_idx];
                
                if Self::try_place_in_bin(sphere, placements, candidates, &bin, self.weight_limit) {
                    continue 'outer;
                }
            }
            
            // Open new bin
            let mut new_bin_placements = Vec::new();
            let mut new_candidates = CandidateList::initialize(&bin, sphere.radius);
            
            if Self::try_place_in_bin(sphere, &mut new_bin_placements, &mut new_candidates, &bin, self.weight_limit) {
                // placed successfully
            } else {
                eprintln!("Sphere too big for bin: {:?}", sphere);
            }
            
            result_bins.push(new_bin_placements);
            bin_candidates.push(new_candidates);
        }

        PackResult::new(Vec::new(), 0.0, result_bins)
    }
}

impl AdvancingFrontSpheres {
    /// Try to place a single sphere into an existing set of placements.
    /// Updates candidates in-place and pushes the placement.
    fn try_place_in_bin(
        sphere: &Sphere,
        placements: &mut Vec<Sphere>,
        candidates: &mut CandidateList,
        bin: &Bin,
        weight_limit: f32,
    ) -> bool {
        if weight_limit > 0.0 {
            let current: f32 = placements.iter().map(|s| s.weight).sum();
            if current + sphere.weight > weight_limit {
                return false;
            }
        }

        let r = sphere.radius;

        // Check if sphere even fits
        if r * 2.0 > bin.w || r * 2.0 > bin.h || r * 2.0 > bin.d {
            return false;
        }

        if candidates.is_empty() {
            return false;
        }

        // Sort by score ascending and rebuild hashmap
        candidates.sort();

        let mut chosen_idx = None;
        for i in 0..candidates.candidates.len() {
            let mut candidate = candidates.candidates[i].clone();
            
            if (candidate.radius - r).abs() > EPS {
                if let Some(pos) = Self::solve_candidate(&candidate.constraints, r, placements, bin) {
                    let touching = candidate.constraints.iter().filter(|c| matches!(c, Constraint::Sphere { .. })).count();
                    candidate.position = pos;
                    candidate.score = Self::compute_score(&pos, touching);
                    candidate.radius = r;
                    // Update in list just to cache
                    candidates.candidates[i] = candidate.clone();
                } else {
                    continue;
                }
            }
            
            if Self::is_valid(&candidate.position, r, placements, bin) {
                chosen_idx = Some(i);
                break;
            }
        }

        if let Some(idx) = chosen_idx {
            let consumed = candidates.candidates[idx].clone();
            let mut placed = sphere.clone();
            placed.position = consumed.position;
            
            let new_idx = placements.len();
            placements.push(placed);
            candidates.update(&consumed, new_idx, placements, bin, r);
            
            return true;
        }

        false
    }

    /// Check that a triple of constraints can define a valid candidate.
    fn is_valid_triple(triple: &[Constraint; 3]) -> bool {
        let mut plane_axes = Vec::new();
        for c in triple {
            if let Constraint::Plane { axis, .. } = c {
                plane_axes.push(*axis);
            }
        }

        // Can't have two planes on the same axis (parallel walls don't pin a point)
        let mut sorted = plane_axes.clone();
        sorted.sort();
        for i in 1..sorted.len() {
            if sorted[i] == sorted[i - 1] {
                return false;
            }
        }

        true
    }

    /// Solve for the sphere center position given three constraints.
    fn solve_candidate(
        triple: &[Constraint; 3],
        r: f32,
        placements: &[Sphere],
        bin: &Bin,
    ) -> Option<Point3f> {
        let mut planes = Vec::new();
        let mut spheres = Vec::new();

        for c in triple {
            match c {
                Constraint::Plane { axis, value } => planes.push((*axis, *value)),
                Constraint::Sphere { index } => spheres.push(*index),
            }
        }

        match (planes.len(), spheres.len()) {
            (3, 0) => geometry::solve_three_planes(
                [planes[0], planes[1], planes[2]],
                r,
                bin.w,
                bin.h,
                bin.d,
            ),
            (2, 1) => {
                let s = &placements[spheres[0]];
                geometry::solve_sphere_two_planes(
                    &s.position,
                    s.radius,
                    planes[0],
                    planes[1],
                    r,
                    bin.w,
                    bin.h,
                    bin.d,
                )
            }
            (1, 2) => {
                let s1 = &placements[spheres[0]];
                let s2 = &placements[spheres[1]];
                geometry::solve_two_spheres_one_plane(
                    &s1.position,
                    s1.radius,
                    &s2.position,
                    s2.radius,
                    planes[0],
                    r,
                    bin.w,
                    bin.h,
                    bin.d,
                )
            }
            (0, 3) => {
                let s1 = &placements[spheres[0]];
                let s2 = &placements[spheres[1]];
                let s3 = &placements[spheres[2]];
                geometry::solve_three_spheres(
                    &s1.position,
                    s1.radius,
                    &s2.position,
                    s2.radius,
                    &s3.position,
                    s3.radius,
                    r,
                    bin.w,
                    bin.h,
                    bin.d,
                )
            }
            _ => None,
        }
    }

    /// Check that a position is valid: inside the bin and not colliding.
    fn is_valid(pos: &Point3f, r: f32, placements: &[Sphere], bin: &Bin) -> bool {
        // Bin bounds
        if pos.x - r < -EPS || pos.y - r < -EPS || pos.z - r < -EPS {
            return false;
        }
        if pos.x + r > bin.w + EPS || pos.y + r > bin.h + EPS || pos.z + r > bin.d + EPS {
            return false;
        }

        // Collision with placed spheres
        for placed in placements {
            let dx = pos.x - placed.position.x;
            let dy = pos.y - placed.position.y;
            let dz = pos.z - placed.position.z;
            let dist_sq = dx * dx + dy * dy + dz * dz;
            let min_dist = r + placed.radius - EPS; // subtract epsilon for tangent spheres
            if dist_sq < min_dist * min_dist {
                return false;
            }
        }

        true
    }

    fn compute_score(pos: &Point3f, touching_spheres: usize) -> f32 {
        let distance = pos.x + pos.y + pos.z;
        let touch_bonus = touching_spheres as f32 * 10.0;
        distance - touch_bonus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bin(w: f32, h: f32, d: f32) -> Bin {
        Bin::new(0, w, h, d)
    }

    #[test]
    fn test_single_sphere() {
        let mut solver = AdvancingFrontSpheres::default();
        solver.init(&SolverProperties::new(
            make_bin(100.0, 100.0, 100.0),
            false,
            String::new(),
            vec![],
            0.0,
        ));

        let spheres = vec![Sphere::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), 5.0)];
        let result = solver.solve(&spheres);

        assert_eq!(result.bins.len(), 1);
        assert_eq!(result.bins[0].len(), 1);

        let placed = &result.bins[0][0];
        assert!((placed.position.x - 5.0).abs() < 0.1);
        assert!((placed.position.y - 5.0).abs() < 0.1);
        assert!((placed.position.z - 5.0).abs() < 0.1);
    }

    #[test]
    fn test_two_equal_spheres() {
        let mut solver = AdvancingFrontSpheres::default();
        solver.init(&SolverProperties::new(
            make_bin(100.0, 100.0, 100.0),
            false,
            String::new(),
            vec![],
            0.0,
        ));

        let spheres = vec![
            Sphere::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), 5.0),
            Sphere::new_without_weight(2, Point3f::new(0.0, 0.0, 0.0), 5.0),
        ];
        let result = solver.solve(&spheres);

        assert_eq!(result.bins.len(), 1);
        assert_eq!(result.bins[0].len(), 2);

        // Verify no collision
        let s1 = &result.bins[0][0];
        let s2 = &result.bins[0][1];
        let dx = s1.position.x - s2.position.x;
        let dy = s1.position.y - s2.position.y;
        let dz = s1.position.z - s2.position.z;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        assert!(dist >= 9.9, "Spheres overlap! dist={}", dist);
    }

    #[test]
    fn test_three_spheres_pack() {
        let mut solver = AdvancingFrontSpheres::default();
        solver.init(&SolverProperties::new(
            make_bin(100.0, 100.0, 100.0),
            false,
            String::new(),
            vec![],
            0.0,
        ));

        let spheres: Vec<Sphere> = (0..3)
            .map(|i| Sphere::new_without_weight(i, Point3f::new(0.0, 0.0, 0.0), 5.0))
            .collect();
        let result = solver.solve(&spheres);

        assert_eq!(result.bins.len(), 1);
        assert_eq!(result.bins[0].len(), 3);

        // Verify no collisions between any pair
        let placed = &result.bins[0];
        for i in 0..placed.len() {
            for j in (i + 1)..placed.len() {
                let dx = placed[i].position.x - placed[j].position.x;
                let dy = placed[i].position.y - placed[j].position.y;
                let dz = placed[i].position.z - placed[j].position.z;
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                assert!(
                    dist >= 9.9,
                    "Spheres {} and {} overlap! dist={}",
                    i, j, dist
                );
            }
        }
    }

    #[test]
    fn test_sphere_too_big() {
        let mut solver = AdvancingFrontSpheres::default();
        solver.init(&SolverProperties::new(
            make_bin(5.0, 5.0, 5.0),
            false,
            String::new(),
            vec![],
            0.0,
        ));

        // Sphere with radius 3 — diameter 6 > bin size 5
        let spheres = vec![Sphere::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), 3.0)];
        let result = solver.solve(&spheres);

        // Should end up in a bin but fail to place
        assert_eq!(result.bins[0].len(), 0);
    }

    #[test]
    fn test_weight_limit() {
        let mut solver = AdvancingFrontSpheres::default();
        solver.init(&SolverProperties::new(
            make_bin(100.0, 100.0, 100.0),
            false,
            String::new(),
            vec![],
            10.0,
        ));

        let spheres = vec![
            Sphere::new(1, Point3f::new(0.0, 0.0, 0.0), 5.0, 6.0),
            Sphere::new(2, Point3f::new(0.0, 0.0, 0.0), 5.0, 6.0),
        ];
        let result = solver.solve(&spheres);

        // Weight limit 10, each sphere weighs 6 — second should go to a new bin
        assert_eq!(result.bins.len(), 2);
    }

    #[test]
    fn test_many_small_spheres() {
        let mut solver = AdvancingFrontSpheres::default();
        solver.init(&SolverProperties::new(
            make_bin(30.0, 30.0, 30.0),
            false,
            String::new(),
            vec![],
            0.0,
        ));

        let spheres: Vec<Sphere> = (0..8)
            .map(|i| Sphere::new_without_weight(i, Point3f::new(0.0, 0.0, 0.0), 5.0))
            .collect();
        let result = solver.solve(&spheres);

        // All should be placed without panicking
        let total_placed: usize = result.bins.iter().map(|b| b.len()).sum();
        assert!(total_placed > 0);

        // Verify no collisions within each bin
        for bin_spheres in &result.bins {
            for i in 0..bin_spheres.len() {
                for j in (i + 1)..bin_spheres.len() {
                    let dx = bin_spheres[i].position.x - bin_spheres[j].position.x;
                    let dy = bin_spheres[i].position.y - bin_spheres[j].position.y;
                    let dz = bin_spheres[i].position.z - bin_spheres[j].position.z;
                    let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                    let min = bin_spheres[i].radius + bin_spheres[j].radius - 0.1;
                    assert!(
                        dist >= min,
                        "Collision in bin: spheres {} and {} dist={} min={}",
                        i, j, dist, min
                    );
                }
            }
        }
    }
}
