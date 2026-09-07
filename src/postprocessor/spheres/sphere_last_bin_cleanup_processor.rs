use std::collections::HashMap;

use crate::common::bin::Bin;
use crate::common::point3f::Point3f;
use crate::common::sphere_spec::Sphere;
use crate::postprocessor::postprocessor_interface::Postprocessor;
use crate::solver::spheres::geometry::{self, Axis, EPS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAxis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, PartialEq)]
enum Constraint {
    Plane { axis: Axis, value: f32 },
    Sphere { index: usize },
}

type CandidateKey = [u64; 3];

fn constraint_key(c: &Constraint) -> u64 {
    match c {
        Constraint::Plane { axis, value } => {
            (0 << 62) | ((*axis as u64) << 32) | (value.to_bits() as u64)
        }
        Constraint::Sphere { index } => (1 << 62) | (*index as u64),
    }
}

fn make_key(constraints: &[Constraint; 3]) -> CandidateKey {
    let mut keys = [
        constraint_key(&constraints[0]),
        constraint_key(&constraints[1]),
        constraint_key(&constraints[2]),
    ];
    keys.sort_unstable();
    keys
}

#[derive(Debug, Clone)]
struct Candidate {
    position: Point3f,
    constraints: [Constraint; 3],
    radius: f32,
}

struct CandidateList {
    candidates: Vec<Candidate>,
    seen: HashMap<CandidateKey, usize>,
}

impl CandidateList {
    fn new() -> Self {
        Self {
            candidates: Vec::new(),
            seen: HashMap::new(),
        }
    }

    fn initialize(bin: &Bin, r: f32, vertical_axis: VerticalAxis) -> Self {
        let mut list = Self::new();
        let mut walls = Vec::new();

        walls.push(Constraint::Plane {
            axis: Axis::X,
            value: 0.0,
        });
        if vertical_axis != VerticalAxis::X {
            walls.push(Constraint::Plane {
                axis: Axis::X,
                value: bin.w,
            });
        }

        walls.push(Constraint::Plane {
            axis: Axis::Y,
            value: 0.0,
        });
        if vertical_axis != VerticalAxis::Y {
            walls.push(Constraint::Plane {
                axis: Axis::Y,
                value: bin.h,
            });
        }

        walls.push(Constraint::Plane {
            axis: Axis::Z,
            value: 0.0,
        });
        if vertical_axis != VerticalAxis::Z {
            walls.push(Constraint::Plane {
                axis: Axis::Z,
                value: bin.d,
            });
        }

        let n = walls.len();
        for i in 0..n {
            for j in (i + 1)..n {
                for k in (j + 1)..n {
                    let triple = [walls[i].clone(), walls[j].clone(), walls[k].clone()];
                    if !is_valid_triple(&triple) {
                        continue;
                    }
                    if let Some(pos) = solve_candidate(&triple, r, &[], bin) {
                        list.add(Candidate {
                            position: pos,
                            constraints: triple,
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
        enable_gap_fill: bool,
        vertical_axis: VerticalAxis,
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

        let mut walls = Vec::new();
        walls.push(Constraint::Plane {
            axis: Axis::X,
            value: 0.0,
        });
        if vertical_axis != VerticalAxis::X {
            walls.push(Constraint::Plane {
                axis: Axis::X,
                value: bin.w,
            });
        }
        walls.push(Constraint::Plane {
            axis: Axis::Y,
            value: 0.0,
        });
        if vertical_axis != VerticalAxis::Y {
            walls.push(Constraint::Plane {
                axis: Axis::Y,
                value: bin.h,
            });
        }
        walls.push(Constraint::Plane {
            axis: Axis::Z,
            value: 0.0,
        });
        if vertical_axis != VerticalAxis::Z {
            walls.push(Constraint::Plane {
                axis: Axis::Z,
                value: bin.d,
            });
        }

        if enable_gap_fill {
            let r_search = 3.0 * r;

            for s_a_idx in 0..placements.len() {
                if s_a_idx == new_idx {
                    continue;
                }
                let sa = &placements[s_a_idx];

                let dist_sa_new = geometry::len(&geometry::sub(&sa.position, &new_sphere.position));
                if dist_sa_new > r_search {
                    continue;
                }

                for s_b_idx in (s_a_idx + 1)..placements.len() {
                    if s_b_idx == new_idx {
                        continue;
                    }
                    let sb = &placements[s_b_idx];

                    let dist_sb_new =
                        geometry::len(&geometry::sub(&sb.position, &new_sphere.position));
                    if dist_sb_new > r_search {
                        continue;
                    }

                    let dist_ab = geometry::len(&geometry::sub(&sa.position, &sb.position));
                    if dist_ab < r_search {
                        pairs_to_check.push([
                            Constraint::Sphere { index: s_a_idx },
                            Constraint::Sphere { index: s_b_idx },
                        ]);
                    }
                }

                for w in &walls {
                    let dist_to_wall = match w {
                        Constraint::Plane { axis: Axis::X, value } => {
                            (new_sphere.position.x - value).abs()
                        }
                        Constraint::Plane { axis: Axis::Y, value } => {
                            (new_sphere.position.y - value).abs()
                        }
                        Constraint::Plane { axis: Axis::Z, value } => {
                            (new_sphere.position.z - value).abs()
                        }
                        _ => unreachable!(),
                    };
                    if dist_to_wall < r_search {
                        pairs_to_check.push([w.clone(), Constraint::Sphere { index: s_a_idx }]);
                    }
                }
            }

            for i in 0..walls.len() {
                let w1 = &walls[i];
                let dist_to_w1 = match w1 {
                    Constraint::Plane { axis: Axis::X, value } => {
                        (new_sphere.position.x - value).abs()
                    }
                    Constraint::Plane { axis: Axis::Y, value } => {
                        (new_sphere.position.y - value).abs()
                    }
                    Constraint::Plane { axis: Axis::Z, value } => {
                        (new_sphere.position.z - value).abs()
                    }
                    _ => unreachable!(),
                };
                if dist_to_w1 > r_search {
                    continue;
                }

                for j in (i + 1)..walls.len() {
                    let w2 = &walls[j];
                    let dist_to_w2 = match w2 {
                        Constraint::Plane { axis: Axis::X, value } => {
                            (new_sphere.position.x - value).abs()
                        }
                        Constraint::Plane { axis: Axis::Y, value } => {
                            (new_sphere.position.y - value).abs()
                        }
                        Constraint::Plane { axis: Axis::Z, value } => {
                            (new_sphere.position.z - value).abs()
                        }
                        _ => unreachable!(),
                    };
                    if dist_to_w2 < r_search {
                        pairs_to_check.push([w1.clone(), w2.clone()]);
                    }
                }
            }
        }

        for pair in pairs_to_check {
            let triple = [pair[0].clone(), pair[1].clone(), new_c.clone()];
            if !is_valid_triple(&triple) {
                continue;
            }
            if let Some(pos) = solve_candidate(&triple, r, placements, bin) {
                self.add(Candidate {
                    position: pos,
                    constraints: triple,
                    radius: r,
                });
            }
        }

        let max_candidates = 2000;
        if self.candidates.len() > max_candidates {
            let removed = self.candidates.split_off(max_candidates);
            for c in removed {
                let key = make_key(&c.constraints);
                self.seen.remove(&key);
            }
        }
    }

    fn add(&mut self, c: Candidate) {
        let key = make_key(&c.constraints);
        if !self.seen.contains_key(&key) {
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

fn is_valid(pos: &Point3f, r: f32, placements: &[Sphere], bin: &Bin) -> bool {
    if pos.x - r < -EPS || pos.y - r < -EPS || pos.z - r < -EPS {
        return false;
    }
    if pos.x + r > bin.w + EPS || pos.y + r > bin.h + EPS || pos.z + r > bin.d + EPS {
        return false;
    }

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

pub struct SphereLastBinCleanupProcessor {
    pub vertical_axis: VerticalAxis,
    pub vertical_weight_factor: f32,
    pub fixed_vertical_weight: Option<f32>,
    pub enable_gap_fill: bool,
}

impl SphereLastBinCleanupProcessor {
    pub fn new() -> Self {
        Self {
            vertical_axis: VerticalAxis::Y,
            vertical_weight_factor: 10.0,
            fixed_vertical_weight: None,
            enable_gap_fill: true,
        }
    }

    pub fn with_vertical_axis(mut self, axis: VerticalAxis) -> Self {
        self.vertical_axis = axis;
        self
    }

    pub fn with_vertical_weight_factor(mut self, factor: f32) -> Self {
        self.vertical_weight_factor = factor;
        self
    }

    pub fn with_fixed_weight(mut self, weight: f32) -> Self {
        self.fixed_vertical_weight = Some(weight);
        self
    }

    pub fn with_gap_fill(mut self, enable: bool) -> Self {
        self.enable_gap_fill = enable;
        self
    }

    pub fn calculate_score(&self, pos: &Point3f, vertical_weight: f32) -> f32 {
        match self.vertical_axis {
            VerticalAxis::Y => pos.x + pos.z + pos.y * vertical_weight,
            VerticalAxis::Z => pos.x + pos.y + pos.z * vertical_weight,
            VerticalAxis::X => pos.y + pos.z + pos.x * vertical_weight,
        }
    }

    fn try_place_in_bin(
        &self,
        sphere: &Sphere,
        placements: &mut Vec<Sphere>,
        candidates: &mut CandidateList,
        bin: &Bin,
        weight_limit: f32,
        vertical_weight: f32,
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

        let mut best_score = f32::MAX;
        let mut best_candidate_idx: Option<usize> = None;
        let mut best_position: Option<Point3f> = None;

        for i in 0..candidates.candidates.len() {
            let candidate = &candidates.candidates[i];
            let mut candidate_pos = candidate.position;

            if (candidate.radius - r).abs() > EPS {
                if let Some(pos) = solve_candidate(&candidate.constraints, r, placements, bin) {
                    candidate_pos = pos;
                } else {
                    continue;
                }
            }

            if is_valid(&candidate_pos, r, placements, bin) {
                let score = self.calculate_score(&candidate_pos, vertical_weight);
                if score < best_score {
                    best_score = score;
                    best_candidate_idx = Some(i);
                    best_position = Some(candidate_pos);
                }
            }
        }

        if let (Some(idx), Some(pos)) = (best_candidate_idx, best_position) {
            let mut consumed = candidates.candidates[idx].clone();
            consumed.position = pos;
            consumed.radius = r;

            let mut placed = sphere.clone();
            placed.position = pos;

            let new_idx = placements.len();
            placements.push(placed);

            candidates.update(
                &consumed,
                new_idx,
                placements,
                bin,
                r,
                self.enable_gap_fill,
                self.vertical_axis,
            );

            return true;
        }

        false
    }
}

impl Default for SphereLastBinCleanupProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Postprocessor<Sphere, Bin> for SphereLastBinCleanupProcessor {
    fn process(&self, solution: &mut [Vec<Sphere>], bin: &Bin) {
        if solution.is_empty() {
            return;
        }

        let last_idx = solution.len() - 1;
        let last_spheres = &solution[last_idx];
        if last_spheres.is_empty() {
            return;
        }

        let bin_size = bin.w.max(bin.h).max(bin.d).max(1.0);
        let vertical_weight = self
            .fixed_vertical_weight
            .unwrap_or(self.vertical_weight_factor * bin_size);

        let mut repacked_spheres = Vec::with_capacity(last_spheres.len());
        let mut candidates = CandidateList::initialize(bin, last_spheres[0].radius, self.vertical_axis);

        let weight_limit = bin.max_weight;

        for sphere in last_spheres {
            if !self.try_place_in_bin(
                sphere,
                &mut repacked_spheres,
                &mut candidates,
                bin,
                weight_limit,
                vertical_weight,
            ) {
                // If any sphere fails to fit, preserve original last bin
                return;
            }
        }

        // Only splice if all spheres were placed into a single bin!
        if repacked_spheres.len() == last_spheres.len() {
            solution[last_idx] = repacked_spheres;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sphere_last_bin_cleanup_empty() {
        let processor = SphereLastBinCleanupProcessor::new();
        let bin = Bin::new(0, 100.0, 100.0, 100.0);

        let mut empty_solution: Vec<Vec<Sphere>> = vec![];
        processor.process(&mut empty_solution, &bin);
        assert!(empty_solution.is_empty());

        let mut empty_last_bin: Vec<Vec<Sphere>> = vec![vec![]];
        processor.process(&mut empty_last_bin, &bin);
        assert_eq!(empty_last_bin.len(), 1);
        assert!(empty_last_bin[0].is_empty());
    }

    #[test]
    fn test_sphere_last_bin_cleanup_excludes_top_corners() {
        let bin = Bin::new(0, 100.0, 100.0, 100.0);
        let candidates = CandidateList::initialize(&bin, 5.0, VerticalAxis::Y);

        assert_eq!(candidates.candidates.len(), 4);
        for c in &candidates.candidates {
            // Y position of all initial candidates must be equal to radius (5.0) on the floor plane
            assert_eq!(c.position.y, 5.0);
        }
    }

    #[test]
    fn test_sphere_last_bin_cleanup_prioritizes_floor() {
        let processor = SphereLastBinCleanupProcessor::new();
        let bin = Bin::new(0, 50.0, 50.0, 50.0);

        // Two spheres initially placed in an elevated stack
        let s1 = Sphere::new_without_weight(1, Point3f::new(5.0, 5.0, 5.0), 5.0);
        let s2 = Sphere::new_without_weight(2, Point3f::new(5.0, 20.0, 5.0), 5.0);

        let mut solution = vec![vec![s1, s2]];
        processor.process(&mut solution, &bin);

        assert_eq!(solution.len(), 1);
        assert_eq!(solution[0].len(), 2);

        // Both spheres must be placed on the floor Y = r = 5.0
        for s in &solution[0] {
            assert_eq!(s.position.y, 5.0);
        }
    }

    #[test]
    fn test_sphere_last_bin_cleanup_rejects_multi_bin() {
        let processor = SphereLastBinCleanupProcessor::new();
        let bin = Bin::new(0, 10.0, 10.0, 10.0);

        // Two spheres of radius 6.0 cannot fit together in a (10, 10, 10) bin
        let s1 = Sphere::new_without_weight(1, Point3f::new(6.0, 6.0, 6.0), 6.0);
        let s2 = Sphere::new_without_weight(2, Point3f::new(6.0, 6.0, 6.0), 6.0);

        let original_spheres = vec![s1, s2];
        let mut solution = vec![original_spheres.clone()];
        processor.process(&mut solution, &bin);

        // Original solution preserved
        assert_eq!(solution, vec![original_spheres]);
    }

    #[test]
    fn test_sphere_last_bin_cleanup_multi_bin_solution() {
        let processor = SphereLastBinCleanupProcessor::new();
        let bin = Bin::new(0, 50.0, 50.0, 50.0);

        let bin0_s = Sphere::new_without_weight(1, Point3f::new(5.0, 5.0, 5.0), 5.0);
        let bin1_s1 = Sphere::new_without_weight(2, Point3f::new(5.0, 5.0, 5.0), 5.0);
        let bin1_s2 = Sphere::new_without_weight(3, Point3f::new(5.0, 20.0, 5.0), 5.0);

        let mut solution = vec![vec![bin0_s.clone()], vec![bin1_s1, bin1_s2]];
        processor.process(&mut solution, &bin);

        assert_eq!(solution.len(), 2);
        assert_eq!(solution[0], vec![bin0_s]);
        assert_eq!(solution[1].len(), 2);
        for s in &solution[1] {
            assert_eq!(s.position.y, 5.0);
        }
    }
}
