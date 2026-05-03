use std::cmp::Ordering;
use crate::common::bin::Bin;
use crate::common::box_spec::BinBox;
use crate::optimizer::solution::Solution;
use crate::solver::parallelsolvers::ParallelSolver;
// use crate::solver::solver_properties::SolverProperties;
use rand::Rng;

pub struct GpuOptimizer {
    boxes: Vec<BinBox>,
    bin: Bin,
    growing_bin: bool,
    _grow_axis: String,
    _rotation_axes: Vec<i32>,
    population_size: usize,
    elite_count: usize,
    box_orders: Vec<Vec<usize>>,
    
    // The parallel solver (OpenCL for native, WebGPU for wasm)
    solver: Box<dyn ParallelSolver + Send + Sync>,
}

impl GpuOptimizer {
    pub fn new(
        boxes: Vec<BinBox>,
        bin: Bin,
        growing_bin: bool,
        grow_axis: String,
        rotation_axes: Vec<i32>,
        population_size: usize,
        elite_count: usize,
        solver: Box<dyn ParallelSolver + Send + Sync>,
    ) -> Self {
        let mut opt = Self {
            boxes,
            bin,
            growing_bin,
            _grow_axis: grow_axis,
            _rotation_axes: rotation_axes,
            population_size,
            elite_count,
            box_orders: Vec::new(),
            solver,
        };
        opt.generate_initial_population();
        opt
    }

    pub fn generate_initial_population(&mut self) {
        let base_order: Vec<usize> = (0..self.boxes.len()).collect();

        let mut growing_order = base_order.clone();
        growing_order.sort_by(|&a, &b| {
            self.boxes[a].volume().partial_cmp(&self.boxes[b].volume()).unwrap_or(Ordering::Equal)
        });
        self.box_orders.push(growing_order);

        let mut shrinking_order = base_order.clone();
        shrinking_order.sort_by(|&a, &b| {
            self.boxes[b].volume().partial_cmp(&self.boxes[a].volume()).unwrap_or(Ordering::Equal)
        });
        self.box_orders.push(shrinking_order);

        let mut shrinking_longest_order = base_order.clone();
        shrinking_longest_order.sort_by(|&a, &b| {
            self.boxes[b].longest_side().partial_cmp(&self.boxes[a].longest_side()).unwrap_or(Ordering::Equal)
        });
        self.box_orders.push(shrinking_longest_order);

        let mut rng = rand::thread_rng();
        use rand::seq::SliceRandom;
        while self.box_orders.len() < self.population_size {
            let mut order = base_order.clone();
            order.shuffle(&mut rng);
            self.box_orders.push(order);
        }

        self.population_size = self.box_orders.len();
    }

    fn evaluate_population(&mut self) -> Vec<Solution> {
        let max_bins = 64; // Default fallback; can be smarter with reference solver
        let max_spaces = 512;
        
        if self.solver.is_template() && !self.solver.is_compiled() {
            // Can use reference_solver to estimate limits (as Java did), for now hardcoded
            self.solver.compile_kernel(max_bins, max_spaces);
        }

        let scores = self.solver.solve(&self.boxes, &self.box_orders);

        let mut scored = Vec::with_capacity(self.population_size);
        for i in 0..self.population_size {
            scored.push(Solution::new(self.box_orders[i].clone(), scores[i], vec![]));
        }

        scored
    }

    pub fn execute_next_generation(&mut self) -> Vec<Vec<BinBox>> {
        let mut scored = self.evaluate_population();

        if !self.growing_bin {
            scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        } else {
            scored.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal));
        }

        let best_order = scored[0].order.clone();

        let best_solution_pack = if let Some(ref_solver) = self.solver.get_reference_solver() {
            let ordered_boxes: Vec<BinBox> = best_order.iter().map(|&idx| self.boxes[idx].clone()).collect();
            ref_solver.solve(&ordered_boxes)
        } else {
            vec![]
        };

        let mut next_gen = Vec::new();
        for i in 0..self.elite_count.min(scored.len()) {
            next_gen.push(scored[i].order.clone());
        }

        let mut rng = rand::thread_rng();
        let max_elite = 1.max(self.elite_count.min(scored.len()));

        use crate::optimizer::mutators::{
            crossover, insert_mutation, scramble_mutation, space_mutation, swap_mutation,
            bin_preservation_crossover,
        };

        let modifiers = vec![
            crossover::modify as crate::optimizer::mutators::modifier::ModifierFn,
            swap_mutation::modify,
            space_mutation::modify,
            insert_mutation::modify,
            bin_preservation_crossover::modify,
            scramble_mutation::modify,
        ];

        while next_gen.len() < self.population_size {
            let modifier = modifiers[rng.gen_range(0..modifiers.len())];
            
            let current_sequence = &scored[rng.gen_range(0..max_elite)];
            let second_sequence = &scored[rng.gen_range(0..max_elite)];

            let child = modifier(&mut rng, current_sequence, second_sequence, &self.bin, &self.boxes);
            next_gen.push(child);
        }

        self.box_orders = next_gen;
        best_solution_pack
    }

    pub fn rate(&self, solution: &[Vec<BinBox>]) -> f64 {
        if self.growing_bin {
            let mut max_extent = 0.0_f64;
            for packed_bin in solution {
                for box_spec in packed_bin {
                    max_extent = max_extent.max((box_spec.position.x + box_spec.size.x) as f64);
                    max_extent = max_extent.max((box_spec.position.y + box_spec.size.y) as f64);
                    max_extent = max_extent.max((box_spec.position.z + box_spec.size.z) as f64);
                }
            }
            max_extent
        } else {
            let mut total_used_volume = 0.0_f64;
            let bins_to_consider = solution.len().saturating_sub(1);
            if bins_to_consider == 0 {
                return 1.0;
            }
            for i in 0..bins_to_consider {
                let mut current_bin_used_volume = 0.0_f64;
                for box_spec in &solution[i] {
                    current_bin_used_volume += box_spec.volume();
                }
                total_used_volume += current_bin_used_volume;
            }
            total_used_volume / (bins_to_consider as f64 * self.bin.volume())
        }
    }
}
