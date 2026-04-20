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
