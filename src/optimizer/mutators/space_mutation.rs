use rand::Rng;
use crate::common::item::Item;
use crate::common::container::Container;
use crate::optimizer::solution::Solution;

pub fn modify<I: Item, C: Container>(
    rng: &mut rand::rngs::ThreadRng,
    current_sequence: &Solution<I>,
    _second: &Solution<I>,
    bin: &C,
    original_items: &[I],
    solver: &mut dyn crate::solver::solver_interface::Solver<I, C>,
) -> Vec<usize> {
    let mut mutated_order = current_sequence.order.clone();

    let mut max_empty_space = -1.0_f32;
    let mut target_box_id: Option<i32> = None;

    let mut sequence = current_sequence.clone();

    if current_sequence.bins.is_empty() && !current_sequence.order.is_empty() {
        let mut ordered_items = Vec::with_capacity(current_sequence.order.len());
        for &idx in &current_sequence.order {
            ordered_items.push(original_items[idx].clone());
        }

        let result = solver.solve(&ordered_items);

        sequence.bins = result.bins;
    }

    let mut num_bins_to_check = sequence.bins.len();
    if num_bins_to_check > 1 {
        num_bins_to_check -= 1;
    }

    for b in 0..num_bins_to_check {
        let bin_items = &sequence.bins[b];
        for item in bin_items {
            let ls = item.longest_side() as f32;
            let pos = item.position();
            let cx = pos.x + ls / 2.0;
            let cy = pos.y + ls / 2.0;
            let cz = pos.z + ls / 2.0;

            let x_right = pos.x + ls;
            let y_top = pos.y + ls;
            let z_top = pos.z + ls;

            let mut next_x = bin.w();
            let mut next_y = bin.h();
            let mut next_z = bin.d();

            for other in bin_items {
                if other.id() == item.id() {
                    continue; 
                }

                let o_pos = other.position();
                let o_ls = other.longest_side() as f32;

                if o_pos.x >= x_right {
                    if cy >= o_pos.y && cy <= o_pos.y + o_ls &&
                       cz >= o_pos.z && cz <= o_pos.z + o_ls {
                        if o_pos.x < next_x {
                            next_x = o_pos.x;
                        }
                    }
                }

                if o_pos.y >= y_top {
                    if cx >= o_pos.x && cx <= o_pos.x + o_ls &&
                       cz >= o_pos.z && cz <= o_pos.z + o_ls {
                        if o_pos.y < next_y {
                            next_y = o_pos.y;
                        }
                    }
                }

                if o_pos.z >= z_top {
                    if cx >= o_pos.x && cx <= o_pos.x + o_ls &&
                       cy >= o_pos.y && cy <= o_pos.y + o_ls {
                        if o_pos.z < next_z {
                            next_z = o_pos.z;
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
                target_box_id = Some(item.id());
            }
        }
    }

    if let Some(target_id) = target_box_id {
        if mutated_order.len() > 1 {
            let mut target_index = usize::MAX;
            for (i, orig_item) in original_items.iter().enumerate() {
                if orig_item.id() == target_id {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::point3f::Point3f;
    use crate::solver::rectangles::first_fit_3d::FirstFit3D;
    use crate::common::bin_box::BinBox;
    use crate::common::bin::Bin;

    #[test]
    fn test_space_mutation() {
        let mut rng = rand::thread_rng();
        
        let b1 = BinBox::new_without_weight(0, Point3f::new(0.0, 0.0, 0.0), Point3f::new(5.0, 5.0, 5.0));
        let current_sequence: Solution<BinBox> = Solution::new(vec![0, 1, 2], 0.0, vec![vec![b1]]);
        
        let original_boxes = vec![
            BinBox::new_without_weight(0, Point3f::new(0.0, 0.0, 0.0), Point3f::new(5.0, 5.0, 5.0)),
            BinBox::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), Point3f::new(5.0, 5.0, 5.0)),
            BinBox::new_without_weight(2, Point3f::new(0.0, 0.0, 0.0), Point3f::new(5.0, 5.0, 5.0)),
        ];
        
        let bin = Bin::new(0, 10.0, 10.0, 10.0);
        let mut solver = FirstFit3D::default();
        
        let mutated = modify(&mut rng, &current_sequence, &current_sequence, &bin, &original_boxes, &mut solver);
        
        assert_eq!(mutated.len(), 3);
        let mut sorted = mutated.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2]);
    }
}
