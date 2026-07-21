use std::cmp::Ordering;
use rand::Rng;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
use crate::common::item::Item;
use crate::common::container::Container;
use crate::solver::solver_interface::Solver;
use crate::optimizer::solution::Solution;
use crate::optimizer::mutators::modifier::ModifierFn;
use crate::optimizer::mutators::{
    crossover, insert_mutation, scramble_mutation, space_mutation, swap_mutation,
    bin_preservation_crossover,
};

pub struct CpuOptimizer<I, S, C>
where
    I: Item,
    S: Solver<I, C>,
    C: Container,
{
    solver_factory: Box<dyn Fn() -> S + Sync + Send>,
    items: Vec<I>,
    bin: C,
    population_size: usize,
    elite_count: usize,
    growing_bin: bool,
    _grow_axis: String,
    _rotation_axes: Vec<i32>,
    threads: usize,

    box_orders: Vec<Vec<usize>>,
    modifiers: Vec<ModifierFn<I, C>>,
}

impl<I, S, C> CpuOptimizer<I, S, C>
where
    I: Item,
    S: Solver<I, C>,
    C: Container,
{
    pub fn new(
        solver_factory: Box<dyn Fn() -> S + Sync + Send>,
        items: Vec<I>,
        bin: C,
        growing_bin: bool,
        grow_axis: String,
        rotation_axes: Vec<i32>,
        population_size: usize,
        elite_count: usize,
        threads: usize,
    ) -> Self {
        let mut opt = Self {
            solver_factory,
            items,
            bin,
            population_size,
            elite_count,
            growing_bin,
            _grow_axis: grow_axis,
            _rotation_axes: rotation_axes,
            threads,
            box_orders: Vec::new(),
            modifiers: vec![
                crossover::modify,
                swap_mutation::modify,
                space_mutation::modify,
                insert_mutation::modify,
                bin_preservation_crossover::modify,
                scramble_mutation::modify,
            ],
        };
        opt.generate_initial_population();
        opt
    }

    pub fn generate_initial_population(&mut self) {
        let base_order: Vec<usize> = (0..self.items.len()).collect();

        // 1. Growing by volume
        let mut growing_order = base_order.clone();
        growing_order.sort_by(|&a, &b| {
            self.items[a].volume().partial_cmp(&self.items[b].volume()).unwrap_or(Ordering::Equal)
        });
        self.box_orders.push(growing_order);

        // 2. Shrinking by volume
        let mut shrinking_order = base_order.clone();
        shrinking_order.sort_by(|&a, &b| {
            self.items[b].volume().partial_cmp(&self.items[a].volume()).unwrap_or(Ordering::Equal)
        });
        self.box_orders.push(shrinking_order);

        // 3. Shrinking by longest side
        let mut shrinking_longest_order = base_order.clone();
        shrinking_longest_order.sort_by(|&a, &b| {
            self.items[b].longest_side().partial_cmp(&self.items[a].longest_side()).unwrap_or(Ordering::Equal)
        });
        self.box_orders.push(shrinking_longest_order);

        // Remaining: random
        let mut rng = rand::thread_rng();
        use rand::seq::SliceRandom;
        while self.box_orders.len() < self.population_size {
            let mut order = base_order.clone();
            order.shuffle(&mut rng);
            self.box_orders.push(order);
        }

        self.population_size = self.box_orders.len();
    }

    fn apply_order(&self, order: &[usize]) -> Vec<I> {
        order.iter().map(|&idx| self.items[idx].clone()).collect()
    }

    pub fn rate(&self, solution: &[Vec<I>]) -> f64 {
        if self.growing_bin {
            let mut max_extent = 0.0_f64;
            for packed_bin in solution {
                for item in packed_bin {
                    let pos = item.position();
                    let longest = item.longest_side() as f32; // basic approx for extent
                    max_extent = max_extent.max((pos.x + longest) as f64);
                    max_extent = max_extent.max((pos.y + longest) as f64);
                    max_extent = max_extent.max((pos.z + longest) as f64);
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
                for item in &solution[i] {
                    current_bin_used_volume += item.volume();
                }
                total_used_volume += current_bin_used_volume;
            }
            total_used_volume / (bins_to_consider as f64 * self.bin.volume())
        }
    }

    fn evaluate_population(&self) -> Vec<Solution<I>> {
        #[cfg(feature = "parallel")]
        if self.threads != 1 {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(if self.threads == 0 { 0 } else { self.threads })
                .build()
                .unwrap();
            return pool.install(|| {
                self.box_orders.par_iter().map(|order| {
                    let mut solver = (self.solver_factory)();
                    let ordered_items = self.apply_order(order);
                    let mut solved = solver.solve(&ordered_items);
                    let score = self.rate(&solved.bins);
                    solved.order = order.clone();
                    solved.score = score;
                    solved
                }).collect()
            });
        }

        let mut solver = (self.solver_factory)();
        self.box_orders.iter().map(|order| {
            let ordered_items = self.apply_order(order);
            let mut solved = solver.solve(&ordered_items);
            let score = self.rate(&solved.bins);
            solved.order = order.clone();
            solved.score = score;
            solved
        }).collect()
    }

    pub fn execute_next_generation(&mut self) -> Vec<Vec<I>> {
        let mut scored = self.evaluate_population();

        if !self.growing_bin {
            scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        } else {
            scored.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal));
        }

        let best_solution_pack = scored[0].bins.clone();
        let mut next_gen = Vec::new();

        // Elitism
        for i in 0..self.elite_count.min(scored.len()) {
            next_gen.push(scored[i].order.clone());
        }

        let mut rng = rand::thread_rng();
        let max_elite = 1.max(self.elite_count.min(scored.len()));

        let mut mutation_solver = (self.solver_factory)();

        while next_gen.len() < self.population_size {
            let modifier = self.modifiers[rng.gen_range(0..self.modifiers.len())];
            
            let current_sequence = &scored[rng.gen_range(0..max_elite)];
            let second_sequence = &scored[rng.gen_range(0..max_elite)];

            let child = modifier(&mut rng, current_sequence, second_sequence, &self.bin, &self.items, &mut mutation_solver);
            next_gen.push(child);
        }

        self.box_orders = next_gen;
        best_solution_pack
    }
}
