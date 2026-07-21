use rand::Rng;
use crate::common::item::Item;
use crate::common::container::Container;
use crate::optimizer::solution::Solution;
use crate::solver::solver_interface::Solver;

pub fn modify<I: Item, C: Container>(
    rng: &mut rand::rngs::ThreadRng,
    current_sequence: &Solution<I>,
    _second: &Solution<I>,
    _bin: &C,
    _original_items: &[I],
    _solver: &mut dyn Solver<I, C>,
) -> Vec<usize> {
    let mut mutated_order = current_sequence.order.clone();
    let size = mutated_order.len();
    if size < 2 {
        return mutated_order;
    }

    let index1 = rng.gen_range(0..size);
    let mut index2 = rng.gen_range(0..size);
    while index1 == index2 {
        index2 = rng.gen_range(0..size);
    }

    mutated_order.swap(index1, index2);
    mutated_order
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::bin::Bin;
    use crate::common::bin_box::BinBox;
    use crate::solver::rectangles::first_fit_3d::FirstFit3D;

    #[test]
    fn test_swap_mutation() {
        let mut rng = rand::thread_rng();
        let current_sequence: Solution<BinBox> = Solution::new(vec![0, 1, 2, 3, 4], 0.0, vec![]);
        let bin = Bin::new(0, 10.0, 10.0, 10.0);
        let mut solver = FirstFit3D::default();
        
        let mutated = modify(&mut rng, &current_sequence, &current_sequence, &bin, &[], &mut solver);
        
        assert_eq!(mutated.len(), 5);
        assert_ne!(mutated, vec![0, 1, 2, 3, 4]); 
        
        let mut sorted = mutated.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3, 4]);
    }
}
