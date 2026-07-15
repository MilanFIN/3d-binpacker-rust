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

    if mutated_order.len() > 1 {
        let remove_index = rng.gen_range(0..mutated_order.len());
        let temp = mutated_order.remove(remove_index);

        let insert_index = rng.gen_range(0..=mutated_order.len());
        mutated_order.insert(insert_index, temp);
    }

    mutated_order
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::first_fit_3d::FirstFit3D;

    #[test]
    fn test_insert_mutation() {
        let mut rng = rand::thread_rng();
        let current_sequence = Solution::new(vec![0, 1, 2, 3, 4], 0.0, vec![]);
        let bin = Bin::new(0, 10.0, 10.0, 10.0);
        let mut solver = FirstFit3D::default();
        
        let mutated = modify(&mut rng, &current_sequence, &current_sequence, &bin, &[], &mut solver);
        
        assert_eq!(mutated.len(), 5);
        
        let mut sorted = mutated.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3, 4]);
    }
}
