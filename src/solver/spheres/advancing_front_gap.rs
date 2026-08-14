use std::collections::HashMap;

use crate::common::bin::Bin;
use crate::common::pack_result::PackResult;
use crate::common::point3f::Point3f;
use crate::common::sphere_spec::Sphere;
use crate::solver::common::solver_properties::SolverProperties;
use crate::solver::solver_interface::Solver;

use super::geometry::{self, Axis, EPS};

// ---------------------------------------------------------------------------
// SpatialGrid
// ---------------------------------------------------------------------------

pub struct SpatialGrid {
    cell_size: f32,
    cells: HashMap<(i32, i32, i32), Vec<usize>>,
}

impl SpatialGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    pub fn insert(&mut self, pos: &Point3f, index: usize) {
        let key = self.get_key(pos);
        self.cells.entry(key).or_default().push(index);
    }

    fn get_key(&self, pos: &Point3f) -> (i32, i32, i32) {
        (
            (pos.x / self.cell_size).floor() as i32,
            (pos.y / self.cell_size).floor() as i32,
            (pos.z / self.cell_size).floor() as i32,
        )
    }

    pub fn query(&self, center: &Point3f, radius: f32) -> Vec<usize> {
        let mut result = Vec::new();
        let min_key = self.get_key(&Point3f::new(
            center.x - radius,
            center.y - radius,
            center.z - radius,
        ));
        let max_key = self.get_key(&Point3f::new(
            center.x + radius,
            center.y + radius,
            center.z + radius,
        ));

        for x in min_key.0..=max_key.0 {
            for y in min_key.1..=max_key.1 {
                for z in min_key.2..=max_key.2 {
                    if let Some(indices) = self.cells.get(&(x, y, z)) {
                        result.extend(indices);
                    }
                }
            }
        }
        result
    }
}

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
                    if !AdvancingFrontGapSpheres::is_valid_triple(&triple) {
                        continue;
                    }
                    if let Some(pos) = AdvancingFrontGapSpheres::solve_candidate(&triple, r, &[], bin) {
                        let score = AdvancingFrontGapSpheres::compute_score(&pos, 0);
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
        grid: &SpatialGrid,
    ) {
        let key = make_key(&consumed.constraints);
        if let Some(&idx) = self.seen.get(&key) {
            self.remove_index(idx);
        }

        let new_c = Constraint::Sphere { index: new_idx };
        let new_sphere = &placements[new_idx];
        
        let [c0, c1, c2] = &consumed.constraints;
        let mut pairs_to_check = vec![
            [c0.clone(), c1.clone()],
            [c0.clone(), c2.clone()],
            [c1.clone(), c2.clone()],
        ];

        // Gap filling heuristic: find nearby spheres using the spatial grid
        let r_search = 3.0 * r; // Search radius based on the new sphere's radius
        let nearby = grid.query(&new_sphere.position, r_search);

        let walls = vec![
            Constraint::Plane { axis: Axis::X, value: 0.0 },
            Constraint::Plane { axis: Axis::X, value: bin.w },
            Constraint::Plane { axis: Axis::Y, value: 0.0 },
            Constraint::Plane { axis: Axis::Y, value: bin.h },
            Constraint::Plane { axis: Axis::Z, value: 0.0 },
            Constraint::Plane { axis: Axis::Z, value: bin.d },
        ];

        for &s_a_idx in &nearby {
            if s_a_idx == new_idx { continue; }
            let sa = &placements[s_a_idx];
            
            // Sphere-Sphere gap candidates
            for &s_b_idx in &nearby {
                if s_b_idx <= s_a_idx || s_b_idx == new_idx { continue; }
                let sb = &placements[s_b_idx];
                
                let dist_ab = geometry::len(&geometry::sub(&sa.position, &sb.position));
                if dist_ab < r_search {
                    pairs_to_check.push([
                        Constraint::Sphere { index: s_a_idx },
                        Constraint::Sphere { index: s_b_idx },
                    ]);
                }
            }
            
            // Sphere-Wall gap candidates
            for w in &walls {
                let dist_to_wall = match w {
                    Constraint::Plane { axis: Axis::X, value } => (new_sphere.position.x - value).abs(),
                    Constraint::Plane { axis: Axis::Y, value } => (new_sphere.position.y - value).abs(),
                    Constraint::Plane { axis: Axis::Z, value } => (new_sphere.position.z - value).abs(),
                    _ => unreachable!(),
                };
                if dist_to_wall < r_search {
                    pairs_to_check.push([w.clone(), Constraint::Sphere { index: s_a_idx }]);
                }
            }
        }

        // Wall-Wall gap candidates
        for i in 0..walls.len() {
            let w1 = &walls[i];
            let dist_to_w1 = match w1 {
                Constraint::Plane { axis: Axis::X, value } => (new_sphere.position.x - value).abs(),
                Constraint::Plane { axis: Axis::Y, value } => (new_sphere.position.y - value).abs(),
                Constraint::Plane { axis: Axis::Z, value } => (new_sphere.position.z - value).abs(),
                _ => unreachable!(),
            };
            if dist_to_w1 > r_search { continue; }
            
            for j in (i + 1)..walls.len() {
                let w2 = &walls[j];
                let dist_to_w2 = match w2 {
                    Constraint::Plane { axis: Axis::X, value } => (new_sphere.position.x - value).abs(),
                    Constraint::Plane { axis: Axis::Y, value } => (new_sphere.position.y - value).abs(),
                    Constraint::Plane { axis: Axis::Z, value } => (new_sphere.position.z - value).abs(),
                    _ => unreachable!(),
                };
                if dist_to_w2 < r_search {
                    pairs_to_check.push([w1.clone(), w2.clone()]);
                }
            }
        }

        for pair in pairs_to_check {
            let triple = [pair[0].clone(), pair[1].clone(), new_c.clone()];
            if !AdvancingFrontGapSpheres::is_valid_triple(&triple) {
                continue;
            }
            if let Some(pos) = AdvancingFrontGapSpheres::solve_candidate(&triple, r, placements, bin) {
                let touching = triple
                    .iter()
                    .filter(|c| matches!(c, Constraint::Sphere { .. }))
                    .count();
                let score = AdvancingFrontGapSpheres::compute_score(&pos, touching);
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
// AdvancingFrontGapSpheres
// ---------------------------------------------------------------------------

pub struct AdvancingFrontGapSpheres {
    bin_template: Option<Bin>,
    weight_limit: f32,
    _growing_bin: bool,
}

impl Default for AdvancingFrontGapSpheres {
    fn default() -> Self {
        Self {
            bin_template: None,
            weight_limit: 0.0,
            _growing_bin: false,
        }
    }
}

impl Solver<Sphere, Bin> for AdvancingFrontGapSpheres {
    fn init(&mut self, properties: &SolverProperties<Bin>) {
        self.bin_template = Some(properties.bin.clone());
        self.weight_limit = properties.weight;
        self._growing_bin = properties.growing_bin;
    }

    fn solve(&mut self, spheres: &[Sphere]) -> PackResult<Sphere> {
        let bin = self.bin_template.clone().unwrap();
        let mut result_bins: Vec<Vec<Sphere>> = Vec::new();
        let mut bin_candidates: Vec<CandidateList> = Vec::new();
        let mut bin_grids: Vec<SpatialGrid> = Vec::new();

        'outer: for sphere in spheres {
            // Try existing bins
            for bin_idx in 0..result_bins.len() {
                let candidates = &mut bin_candidates[bin_idx];
                let placements = &mut result_bins[bin_idx];
                let grid = &mut bin_grids[bin_idx];
                
                if Self::try_place_in_bin(sphere, placements, candidates, grid, &bin, self.weight_limit) {
                    continue 'outer;
                }
            }
            
            // Open new bin
            let mut new_bin_placements = Vec::new();
            let mut new_candidates = CandidateList::initialize(&bin, sphere.radius);
            let mut new_grid = SpatialGrid::new(10.0); // 10.0 is a reasonable cell size
            
            if Self::try_place_in_bin(sphere, &mut new_bin_placements, &mut new_candidates, &mut new_grid, &bin, self.weight_limit) {
                // placed successfully
            } else {
                eprintln!("Sphere too big for bin: {:?}", sphere);
            }
            
            result_bins.push(new_bin_placements);
            bin_candidates.push(new_candidates);
            bin_grids.push(new_grid);
        }

        PackResult::new(Vec::new(), 0.0, result_bins)
    }
}

impl AdvancingFrontGapSpheres {
    fn try_place_in_bin(
        sphere: &Sphere,
        placements: &mut Vec<Sphere>,
        candidates: &mut CandidateList,
        grid: &mut SpatialGrid,
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

        if r * 2.0 > bin.w || r * 2.0 > bin.h || r * 2.0 > bin.d {
            return false;
        }

        if candidates.is_empty() {
            return false;
        }

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
                    candidates.candidates[i] = candidate.clone();
                } else {
                    continue;
                }
            }
            
            if Self::is_valid_with_grid(&candidate.position, r, placements, grid, bin) {
                chosen_idx = Some(i);
                break;
            }
        }

        if let Some(idx) = chosen_idx {
            let consumed = candidates.candidates[idx].clone();
            let mut placed = sphere.clone();
            placed.position = consumed.position;
            
            let new_idx = placements.len();
            placements.push(placed.clone());
            grid.insert(&placed.position, new_idx);
            
            candidates.update(&consumed, new_idx, placements, bin, r, grid);
            
            return true;
        }

        false
    }

    fn is_valid_triple(triple: &[Constraint; 3]) -> bool {
        let mut plane_axes = Vec::new();
        for c in triple {
            if let Constraint::Plane { axis, .. } = c {
                plane_axes.push(*axis);
            }
        }

        let mut sorted = plane_axes.clone();
        sorted.sort();
        for i in 1..sorted.len() {
            if sorted[i] == sorted[i - 1] {
                return false;
            }
        }

        true
    }

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

    fn is_valid_with_grid(pos: &Point3f, r: f32, placements: &[Sphere], _grid: &SpatialGrid, bin: &Bin) -> bool {
        if pos.x - r < -EPS || pos.y - r < -EPS || pos.z - r < -EPS {
            return false;
        }
        if pos.x + r > bin.w + EPS || pos.y + r > bin.h + EPS || pos.z + r > bin.d + EPS {
            return false;
        }

        // Just check all placements for safety and simplicity
        for placed in placements {
            let dx = pos.x - placed.position.x;
            let dy = pos.y - placed.position.y;
            let dz = pos.z - placed.position.z;
            let dist_sq = dx * dx + dy * dy + dz * dz;
            let min_dist = r + placed.radius - EPS; 
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
        let mut solver = AdvancingFrontGapSpheres::default();
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
    }

    #[test]
    fn test_two_equal_spheres() {
        let mut solver = AdvancingFrontGapSpheres::default();
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
    }

    #[test]
    fn test_three_spheres_pack() {
        let mut solver = AdvancingFrontGapSpheres::default();
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
    }

    #[test]
    fn test_sphere_too_big() {
        let mut solver = AdvancingFrontGapSpheres::default();
        solver.init(&SolverProperties::new(
            make_bin(5.0, 5.0, 5.0),
            false,
            String::new(),
            vec![],
            0.0,
        ));

        let spheres = vec![Sphere::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), 3.0)];
        let result = solver.solve(&spheres);

        assert_eq!(result.bins[0].len(), 0);
    }
}
