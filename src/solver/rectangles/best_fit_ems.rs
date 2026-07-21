use crate::common::bin::Bin;
use crate::common::bin_box::BinBox;
use crate::solver::rectangles::placement_utils::PlacementUtils;
use crate::solver::solver_interface::Solver;
use crate::solver::common::solver_properties::SolverProperties;
use crate::common::pack_result::PackResult;

pub struct BestFitEMS {
    bin_template: Option<Bin>,
    growing_bin: bool,
    grow_axis: String,
    rotation_axes: Vec<i32>,
    weight_limit: f32,
}

impl Default for BestFitEMS {
    fn default() -> Self {
        Self {
            bin_template: None,
            growing_bin: false,
            grow_axis: String::new(),
            rotation_axes: Vec::new(),
            weight_limit: 0.0,
        }
    }
}

impl Solver<BinBox, Bin> for BestFitEMS {
    fn init(&mut self, properties: &SolverProperties<Bin>) {
        self.bin_template = Some(properties.bin.clone());
        self.growing_bin = properties.growing_bin;
        self.grow_axis = properties.grow_axis.clone();
        self.rotation_axes = properties.rotation_axes.clone();
        self.weight_limit = properties.weight;
    }

    fn solve(&mut self, boxes: &[BinBox]) -> PackResult<BinBox> {
        let mut active_bins: Vec<Bin> = Vec::new();
        let mut result = Vec::new();

        let mut bin_template = self.bin_template.clone().unwrap();

        if self.growing_bin {
            match self.grow_axis.as_str() {
                "x" => bin_template.w = f32::MAX,
                "y" => bin_template.h = f32::MAX,
                "z" => bin_template.d = f32::MAX,
                _ => bin_template.h = f32::MAX,
            }
        }

        active_bins.push(Bin::new(0, bin_template.w, bin_template.h, bin_template.d));

        for box_item in boxes {
            let mut best_score = f32::MAX;
            let mut best_bin_idx: Option<usize> = None;
            let mut best_space_index = 0usize;
            let mut best_fitted_box: Option<BinBox> = None;

            for (bin_idx, bin) in active_bins.iter().enumerate() {
                if self.weight_limit > 0.0 && bin.weight + box_item.weight > self.weight_limit {
                    continue;
                }
                for i in 0..bin.free_spaces.len() {
                    let space = &bin.free_spaces[i];
                    if let Some(fitted) = PlacementUtils::find_fit(box_item, space, Some(&self.rotation_axes)) {
                        let score = PlacementUtils::calculate_score_ems(&fitted, space);
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
                let placed = PlacementUtils::place_box_ems(&fitted, &mut active_bins[bin_idx], best_space_index);
                PlacementUtils::prune_colliding_spaces_ems(&placed, &mut active_bins[bin_idx]);

                active_bins[bin_idx].util_counter += 1;
                if active_bins[bin_idx].util_counter > 10 {
                    PlacementUtils::prune_wrapped_spaces_bin_ems(&mut active_bins[bin_idx]);
                    active_bins[bin_idx].util_counter = 0;
                }
            } else {
                let mut new_bin = Bin::new(
                    active_bins.len() as i32,
                    bin_template.w,
                    bin_template.h,
                    bin_template.d,
                );
                let space = new_bin.free_spaces[0].clone();
                if let Some(fitted) = PlacementUtils::find_fit(box_item, &space, Some(&self.rotation_axes)) {
                    let placed = PlacementUtils::place_box_ems(&fitted, &mut new_bin, 0);
                    PlacementUtils::prune_colliding_spaces_ems(&placed, &mut new_bin);
                } else {
                    eprintln!("Box too big for bin: {:?}", box_item);
                }
                active_bins.push(new_bin);
            }
        }

        if self.growing_bin {
            let first_bin = &mut active_bins[0];
            match self.grow_axis.as_str() {
                "x" => {
                    let max_x = first_bin.boxes.iter().map(|b| b.position.x + b.size.x).fold(0.0_f32, f32::max);
                    first_bin.w = max_x;
                }
                "y" => {
                    let max_y = first_bin.boxes.iter().map(|b| b.position.y + b.size.y).fold(0.0_f32, f32::max);
                    first_bin.h = max_y;
                }
                "z" => {
                    let max_z = first_bin.boxes.iter().map(|b| b.position.z + b.size.z).fold(0.0_f32, f32::max);
                    first_bin.d = max_z;
                }
                _ => {}
            }
        }

        for bin in active_bins {
            result.push(bin.boxes);
        }

        PackResult::new(Vec::new(), 0.0, result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::point3f::Point3f;

    #[test]
    fn test_best_fit_ems_simple() {
        let mut solver = BestFitEMS::default();
        let props = SolverProperties {
            bin: Bin::new(0, 10.0, 10.0, 10.0),
            growing_bin: false,
            grow_axis: "".to_string(),
            rotation_axes: vec![0, 1, 2],
            weight: 0.0,
        };
        solver.init(&props);

        let boxes = vec![
            BinBox::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), Point3f::new(5.0, 5.0, 5.0)),
            BinBox::new_without_weight(2, Point3f::new(0.0, 0.0, 0.0), Point3f::new(5.0, 5.0, 5.0)),
        ];

        let result = solver.solve(&boxes);
        assert_eq!(result.bins.len(), 1); 
        assert_eq!(result.bins[0].len(), 2);
    }
}
