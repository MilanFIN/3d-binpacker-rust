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
    use crate::common::bin::Bin;
    use crate::common::bin_box::BinBox;
    use crate::solver::rectangles::first_fit_3d::FirstFit3D;

    #[test]
    fn test_insert_mutation() {
        let mut rng = rand::thread_rng();
        let current_sequence: Solution<BinBox> = Solution::new(vec![0, 1, 2, 3, 4], 0.0, vec![]);
        let bin = Bin::new(0, 10.0, 10.0, 10.0);
        let mut solver = FirstFit3D::default();
        
        let mutated = modify(&mut rng, &current_sequence, &current_sequence, &bin, &[], &mut solver);
        
        assert_eq!(mutated.len(), 5);
        
        let mut sorted = mutated.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2, 3, 4]);
    }
}
