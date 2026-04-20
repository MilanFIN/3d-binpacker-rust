use std::collections::HashSet;
use crate::common::bin::Bin;
use crate::common::box_spec::BinBox;
use crate::optimizer::solution::Solution;

pub fn modify(
    _rng: &mut rand::rngs::ThreadRng,
    current_sequence: &Solution,
    second: &Solution,
    bin: &Bin,
    original_boxes: &[BinBox],
) -> Vec<usize> {
    let mut max_utilization = -1.0_f64;
    let mut best_bin_boxes: Option<&Vec<BinBox>> = None;

    for bin_boxes in &current_sequence.solved {
        if bin_boxes.is_empty() {
            continue;
        }

        let mut volume = 0.0_f64;
        for box_spec in bin_boxes {
            volume += box_spec.volume();
        }

        let utilization = volume / bin.volume();
        if utilization > max_utilization {
            max_utilization = utilization;
            best_bin_boxes = Some(bin_boxes);
        }
    }

    let mut child = Vec::new();
    let mut packed_ids = HashSet::new();

    // 1. Pack the best bin from parent 1
    if let Some(best) = best_bin_boxes {
        for box_spec in best {
            let mut target_index = usize::MAX;
            for (i, orig_box) in original_boxes.iter().enumerate() {
                if orig_box.id == box_spec.id {
                    target_index = i;
                    break;
                }
            }
            if target_index == usize::MAX {
                target_index = box_spec.id as usize;
            }

            child.push(target_index);
            packed_ids.insert(target_index);
        }
    }

    // 2. Fill the rest of the sequence from parent 2
    for &id in &second.order {
        if !packed_ids.contains(&id) {
            child.push(id);
            packed_ids.insert(id);
        }
    }

    child
}
