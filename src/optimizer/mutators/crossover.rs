use rand::Rng;
use crate::common::bin::Bin;
use crate::common::box_spec::BinBox;
use crate::optimizer::solution::Solution;

pub fn modify(
    rng: &mut rand::rngs::ThreadRng,
    current_sequence: &Solution,
    second: &Solution,
    _bin: &Bin,
    _original_boxes: &[BinBox],
) -> Vec<usize> {
    let parent1 = &current_sequence.order;
    let parent2 = &second.order;
    let size = parent1.len();

    if size == 0 {
        return vec![];
    }

    let mut cut1 = rng.gen_range(0..size);
    let mut cut2 = rng.gen_range(0..size);

    if cut1 > cut2 {
        std::mem::swap(&mut cut1, &mut cut2);
    }

    let mut child = vec![usize::MAX; size];
    let mut contained = std::collections::HashSet::new();

    // 1. Copy the slice from parent2
    for i in cut1..=cut2 {
        let val = parent2[i];
        child[i] = val;
        contained.insert(val);
    }

    // 2. Fill remaining positions from parent1 in order
    let mut fill_pos = (cut2 + 1) % size;

    for i in 0..size {
        let gene = parent1[(cut2 + 1 + i) % size];

        if !contained.contains(&gene) {
            child[fill_pos] = gene;
            contained.insert(gene);
            fill_pos = (fill_pos + 1) % size;
        }
    }

    child
}
