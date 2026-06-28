use rand::Rng;
use crate::common::bin::Bin;
use crate::common::box_spec::BinBox;
use crate::optimizer::solution::Solution;

pub fn modify(
    rng: &mut rand::rngs::ThreadRng,
    current_sequence: &Solution,
    _second: &Solution,
    bin: &Bin,
    original_boxes: &[BinBox],
    _solver: &mut dyn crate::solver::solver_interface::Solver,
) -> Vec<usize> {
    let mut mutated_order = current_sequence.order.clone();

    let mut max_empty_space = -1.0_f32;
    let mut target_box_id: Option<i32> = None;

    let mut num_bins_to_check = current_sequence.solved.len();
    if num_bins_to_check > 1 {
        num_bins_to_check -= 1;
    }

    for b in 0..num_bins_to_check {
        let bin_boxes = &current_sequence.solved[b];
        for box_spec in bin_boxes {
            let cx = box_spec.position.x + box_spec.size.x / 2.0;
            let cy = box_spec.position.y + box_spec.size.y / 2.0;
            let cz = box_spec.position.z + box_spec.size.z / 2.0;

            let x_right = box_spec.position.x + box_spec.size.x;
            let y_top = box_spec.position.y + box_spec.size.y;
            let z_top = box_spec.position.z + box_spec.size.z;

            let mut next_x = bin.w;
            let mut next_y = bin.h;
            let mut next_z = bin.d;

            for other in bin_boxes {
                if other.id == box_spec.id {
                    continue; 
                }

                if other.position.x >= x_right {
                    if cy >= other.position.y && cy <= other.position.y + other.size.y &&
                       cz >= other.position.z && cz <= other.position.z + other.size.z {
                        if other.position.x < next_x {
                            next_x = other.position.x;
                        }
                    }
                }

                if other.position.y >= y_top {
                    if cx >= other.position.x && cx <= other.position.x + other.size.x &&
                       cz >= other.position.z && cz <= other.position.z + other.size.z {
                        if other.position.y < next_y {
                            next_y = other.position.y;
                        }
                    }
                }

                if other.position.z >= z_top {
                    if cx >= other.position.x && cx <= other.position.x + other.size.x &&
                       cy >= other.position.y && cy <= other.position.y + other.size.y {
                        if other.position.z < next_z {
                            next_z = other.position.z;
                        }
                    }
                }
            }

            let empty_x = next_x - x_right;
            let empty_y = next_y - y_top;
            let empty_z = next_z - z_top;

            let total_empty = empty_x + empty_y + empty_z;
            if total_empty > max_empty_space {
                max_empty_space = total_empty;
                target_box_id = Some(box_spec.id);
            }
        }
    }

    if let Some(target_id) = target_box_id {
        if mutated_order.len() > 1 {
            let mut target_index = usize::MAX;
            for (i, orig_box) in original_boxes.iter().enumerate() {
                if orig_box.id == target_id {
                    target_index = i;
                    break;
                }
            }
            if target_index == usize::MAX {
                target_index = target_id as usize;
            }

            if let Some(order_index1) = mutated_order.iter().position(|&x| x == target_index) {
                let mut order_index2 = rng.gen_range(0..mutated_order.len());
                while order_index1 == order_index2 {
                    order_index2 = rng.gen_range(0..mutated_order.len());
                }
                mutated_order.swap(order_index1, order_index2);
            }
        }
    }

    mutated_order
}
