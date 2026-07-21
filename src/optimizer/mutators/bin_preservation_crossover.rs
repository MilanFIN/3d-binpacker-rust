use std::collections::HashSet;
use crate::common::item::Item;
use crate::common::container::Container;
use crate::optimizer::solution::Solution;
use crate::solver::solver_interface::Solver;

pub fn modify<I: Item, C: Container>(
    _rng: &mut rand::rngs::ThreadRng,
    current_sequence: &Solution<I>,
    second: &Solution<I>,
    bin: &C,
    original_items: &[I],
    solver: &mut dyn Solver<I, C>,
) -> Vec<usize> {
    let mut max_utilization = -1.0_f64;
    let mut best_bin_items_owned: Option<Vec<I>> = None;

    if current_sequence.bins.is_empty() && !current_sequence.order.is_empty() {
        let mut ordered_items = Vec::with_capacity(current_sequence.order.len());
        for &idx in &current_sequence.order {
            ordered_items.push(original_items[idx].clone());
        }

        let solved = solver.solve(&ordered_items);

        for bin_items in solved.bins {
            if bin_items.is_empty() {
                continue;
            }
            let mut volume = 0.0_f64;
            for item in &bin_items {
                volume += item.volume();
            }
            let utilization = volume / bin.volume();
            if utilization > max_utilization {
                max_utilization = utilization;
                best_bin_items_owned = Some(bin_items);
            }
        }
    } else {
        for bin_items in &current_sequence.bins {
            if bin_items.is_empty() {
                continue;
            }

            let mut volume = 0.0_f64;
            for item in bin_items {
                volume += item.volume();
            }

            let utilization = volume / bin.volume();
            if utilization > max_utilization {
                max_utilization = utilization;
                best_bin_items_owned = Some(bin_items.clone());
            }
        }
    }

    let mut child = Vec::new();
    let mut packed_ids = HashSet::new();

    if let Some(best) = best_bin_items_owned {
        for item in best {
            let mut target_index = usize::MAX;
            for (i, orig_item) in original_items.iter().enumerate() {
                if orig_item.id() == item.id() {
                    target_index = i;
                    break;
                }
            }
            if target_index == usize::MAX {
                target_index = item.id() as usize;
            }

            child.push(target_index);
            packed_ids.insert(target_index);
        }
    }

    for &id in &second.order {
        if !packed_ids.contains(&id) {
            child.push(id);
            packed_ids.insert(id);
        }
    }

    child
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::point3f::Point3f;
    use crate::solver::rectangles::first_fit_3d::FirstFit3D;
    use crate::common::bin_box::BinBox;
    use crate::common::bin::Bin;

    #[test]
    fn test_bin_preservation_crossover() {
        let mut rng = rand::thread_rng();
        
        let b1 = BinBox::new_without_weight(0, Point3f::new(0.0, 0.0, 0.0), Point3f::new(5.0, 5.0, 5.0));
        let b2 = BinBox::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), Point3f::new(5.0, 5.0, 5.0));
        let current_sequence: Solution<BinBox> = Solution::new(vec![0, 1, 2], 0.0, vec![vec![b1, b2]]);
        let second: Solution<BinBox> = Solution::new(vec![2, 1, 0], 0.0, vec![]);
        
        let original_boxes = vec![
            BinBox::new_without_weight(0, Point3f::new(0.0, 0.0, 0.0), Point3f::new(5.0, 5.0, 5.0)),
            BinBox::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), Point3f::new(5.0, 5.0, 5.0)),
            BinBox::new_without_weight(2, Point3f::new(0.0, 0.0, 0.0), Point3f::new(5.0, 5.0, 5.0)),
        ];
        
        let bin = Bin::new(0, 10.0, 10.0, 10.0);
        let mut solver = FirstFit3D::default();
        
        let child = modify(&mut rng, &current_sequence, &second, &bin, &original_boxes, &mut solver);
        
        assert_eq!(child.len(), 3);
        let mut sorted = child.clone();
        sorted.sort();
        assert_eq!(sorted, vec![0, 1, 2]);
        
        assert_eq!(child[0], 0);
        assert_eq!(child[1], 1);
        assert_eq!(child[2], 2);
    }
}
