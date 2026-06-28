use rand::Rng;
use crate::common::bin::Bin;
use crate::common::box_spec::BinBox;
use crate::optimizer::solution::Solution;

pub fn modify(
    rng: &mut rand::rngs::ThreadRng,
    current_sequence: &Solution,
    _second: &Solution,
    _bin: &Bin,
    _original_boxes: &[BinBox],
    _solver: &mut dyn crate::solver::solver_interface::Solver,
) -> Vec<usize> {
    let mut mutated_order = current_sequence.order.clone();
    let size = mutated_order.len();
    if size < 2 {
        return mutated_order;
    }

    let mut index1 = rng.gen_range(0..size);
    let mut index2 = rng.gen_range(0..size);
    while index1 == index2 {
        index2 = rng.gen_range(0..size);
    }

    mutated_order.swap(index1, index2);
    mutated_order
}
