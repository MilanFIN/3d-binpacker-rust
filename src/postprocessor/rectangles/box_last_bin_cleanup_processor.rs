use crate::common::bin::Bin;
use crate::common::bin_box::BinBox;
use crate::postprocessor::postprocessor_interface::Postprocessor;
use crate::solver::rectangles::placement_utils::PlacementUtils;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAxis {
    X,
    Y,
    Z,
}

pub struct BoxLastBinCleanupProcessor {
    pub rotation_axes: Vec<i32>,
    pub vertical_axis: VerticalAxis,
    pub vertical_weight_factor: f32,
    pub fixed_vertical_weight: Option<f32>,
}

impl BoxLastBinCleanupProcessor {
    pub fn new() -> Self {
        Self {
            rotation_axes: vec![0, 1, 2],
            vertical_axis: VerticalAxis::Y,
            vertical_weight_factor: 10.0,
            fixed_vertical_weight: None,
        }
    }

    pub fn with_rotation_axes(mut self, rotation_axes: Vec<i32>) -> Self {
        self.rotation_axes = rotation_axes;
        self
    }

    pub fn with_vertical_axis(mut self, axis: VerticalAxis) -> Self {
        self.vertical_axis = axis;
        self
    }

    pub fn with_vertical_weight_factor(mut self, factor: f32) -> Self {
        self.vertical_weight_factor = factor;
        self
    }

    pub fn with_fixed_weight(mut self, weight: f32) -> Self {
        self.fixed_vertical_weight = Some(weight);
        self
    }

    fn calculate_score(&self, space: &crate::common::space::Space, vertical_weight: f32) -> f32 {
        match self.vertical_axis {
            VerticalAxis::Y => space.x + space.z + space.y * vertical_weight,
            VerticalAxis::Z => space.x + space.y + space.z * vertical_weight,
            VerticalAxis::X => space.y + space.z + space.x * vertical_weight,
        }
    }
}

impl Default for BoxLastBinCleanupProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Postprocessor<BinBox, Bin> for BoxLastBinCleanupProcessor {
    fn process(&self, solution: &mut [Vec<BinBox>], bin: &Bin) {
        if solution.is_empty() {
            return;
        }

        let last_idx = solution.len() - 1;
        let last_boxes = &solution[last_idx];
        if last_boxes.is_empty() {
            return;
        }

        let bin_size = bin.w.max(bin.h).max(bin.d).max(1.0);
        let vertical_weight = self
            .fixed_vertical_weight
            .unwrap_or(self.vertical_weight_factor * bin_size);

        let mut active_bins: Vec<Bin> = vec![Bin::new(0, bin.w, bin.h, bin.d)];
        active_bins[0].max_weight = bin.max_weight;
        let weight_limit = bin.max_weight;

        for box_item in last_boxes {
            let mut best_score = f32::MAX;
            let mut best_bin_idx: Option<usize> = None;
            let mut best_space_index = 0usize;
            let mut best_fitted_box: Option<BinBox> = None;

            for (bin_idx, active_bin) in active_bins.iter().enumerate() {
                if weight_limit > 0.0 && active_bin.weight + box_item.weight > weight_limit {
                    continue;
                }

                for i in 0..active_bin.free_spaces.len() {
                    let space = &active_bin.free_spaces[i];
                    if let Some(fitted) =
                        PlacementUtils::find_fit(box_item, space, Some(&self.rotation_axes))
                    {
                        let score = self.calculate_score(space, vertical_weight);
                        if score < best_score {
                            best_score = score;
                            best_bin_idx = Some(bin_idx);
                            best_space_index = i;
                            best_fitted_box = Some(fitted);
                        }
                    }
                }
            }

            if let (Some(bin_idx), Some(fitted)) = (best_bin_idx, best_fitted_box) {
                let placed = PlacementUtils::place_box_ems(
                    &fitted,
                    &mut active_bins[bin_idx],
                    best_space_index,
                );
                PlacementUtils::prune_colliding_spaces_ems(&placed, &mut active_bins[bin_idx]);
                active_bins[bin_idx].util_counter += 1;
                if active_bins[bin_idx].util_counter > 10 {
                    PlacementUtils::prune_wrapped_spaces_bin_ems(&mut active_bins[bin_idx]);
                    active_bins[bin_idx].util_counter = 0;
                }
            } else {
                // If it doesn't fit in the current bin, create a new bin
                let mut new_bin = Bin::new(
                    active_bins.len() as i32,
                    bin.w,
                    bin.h,
                    bin.d,
                );
                new_bin.max_weight = bin.max_weight;
                let space = new_bin.free_spaces[0].clone();
                if let Some(fitted) =
                    PlacementUtils::find_fit(box_item, &space, Some(&self.rotation_axes))
                {
                    let placed = PlacementUtils::place_box_ems(&fitted, &mut new_bin, 0);
                    PlacementUtils::prune_colliding_spaces_ems(&placed, &mut new_bin);
                }
                active_bins.push(new_bin);
            }
        }

        // Only splice if all items fit into a single bin!
        if active_bins.len() == 1 && active_bins[0].boxes.len() == last_boxes.len() {
            solution[last_idx] = active_bins.pop().unwrap().boxes;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::point3f::Point3f;

    #[test]
    fn test_box_last_bin_cleanup_empty() {
        let processor = BoxLastBinCleanupProcessor::new();
        let bin = Bin::new(0, 10.0, 10.0, 10.0);

        // Empty solution
        let mut empty_solution: Vec<Vec<BinBox>> = vec![];
        processor.process(&mut empty_solution, &bin);
        assert!(empty_solution.is_empty());

        // Solution with empty last bin
        let mut empty_last_bin: Vec<Vec<BinBox>> = vec![vec![]];
        processor.process(&mut empty_last_bin, &bin);
        assert_eq!(empty_last_bin.len(), 1);
        assert!(empty_last_bin[0].is_empty());
    }

    #[test]
    fn test_box_last_bin_cleanup_prioritizes_low() {
        let processor = BoxLastBinCleanupProcessor::new();
        let bin = Bin::new(0, 10.0, 10.0, 10.0);

        // Two boxes initially stacked vertically
        let box1 = BinBox::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), Point3f::new(4.0, 4.0, 4.0));
        let box2 = BinBox::new_without_weight(2, Point3f::new(0.0, 4.0, 0.0), Point3f::new(4.0, 4.0, 4.0));

        let mut solution = vec![vec![box1, box2]];
        processor.process(&mut solution, &bin);

        assert_eq!(solution.len(), 1);
        assert_eq!(solution[0].len(), 2);
        for b in &solution[0] {
            assert_eq!(b.position.y, 0.0);
        }
    }

    #[test]
    fn test_box_last_bin_cleanup_rejects_multi_bin() {
        let processor = BoxLastBinCleanupProcessor::new();
        let bin = Bin::new(0, 5.0, 5.0, 5.0);

        // Two boxes size (4, 4, 4) in a (5, 5, 5) bin will not both fit in a single bin
        let box1 = BinBox::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), Point3f::new(4.0, 4.0, 4.0));
        let box2 = BinBox::new_without_weight(2, Point3f::new(0.0, 0.0, 0.0), Point3f::new(4.0, 4.0, 4.0));

        let original_boxes = vec![box1.clone(), box2.clone()];
        let mut solution = vec![original_boxes.clone()];
        processor.process(&mut solution, &bin);

        // Solution must remain unchanged
        assert_eq!(solution, vec![original_boxes]);
    }

    #[test]
    fn test_box_last_bin_cleanup_multi_bin_solution() {
        let processor = BoxLastBinCleanupProcessor::new();
        let bin = Bin::new(0, 10.0, 10.0, 10.0);

        let bin0_box = BinBox::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), Point3f::new(4.0, 4.0, 4.0));
        let bin1_box1 = BinBox::new_without_weight(2, Point3f::new(0.0, 0.0, 0.0), Point3f::new(4.0, 4.0, 4.0));
        let bin1_box2 = BinBox::new_without_weight(3, Point3f::new(0.0, 4.0, 0.0), Point3f::new(4.0, 4.0, 4.0));

        let mut solution = vec![vec![bin0_box.clone()], vec![bin1_box1, bin1_box2]];
        processor.process(&mut solution, &bin);

        assert_eq!(solution.len(), 2);
        // Bin 0 untouched
        assert_eq!(solution[0], vec![bin0_box]);
        // Bin 1 repacked: both boxes at Y=0.0
        assert_eq!(solution[1].len(), 2);
        for b in &solution[1] {
            assert_eq!(b.position.y, 0.0);
        }
    }
}
