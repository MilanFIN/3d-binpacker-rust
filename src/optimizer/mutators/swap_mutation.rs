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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::first_fit_3d::FirstFit3D;

    #[test]
    fn test_swap_mutation() {
        let mut rng = rand::thread_rng();
        let current_sequence = Solution::new(vec![0, 1, 2, 3, 4], 0.0, vec![]);
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
