use crate::common::bin::Bin;
use crate::common::box_spec::BinBox;
use crate::solver::placement_utils::PlacementUtils;
use crate::solver::solver_interface::Solver;
use crate::solver::solver_properties::SolverProperties;

pub struct BestFit3D {
    bin_template: Option<Bin>,
    growing_bin: bool,
    grow_axis: String,
    rotation_axes: Vec<i32>,
    weight_limit: f32,
}

impl Default for BestFit3D {
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

impl Solver for BestFit3D {
    fn init(&mut self, properties: &SolverProperties) {
        self.bin_template = Some(properties.bin.clone());
        self.growing_bin = properties.growing_bin;
        self.grow_axis = properties.grow_axis.clone();
        self.rotation_axes = properties.rotation_axes.clone();
        self.weight_limit = properties.weight;
    }

    fn solve(&mut self, boxes: &[BinBox]) -> Vec<Vec<BinBox>> {
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
                        let score = Self::calculate_score(&fitted, space);
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
                PlacementUtils::place_box_bsp(&fitted, &mut active_bins[bin_idx], best_space_index);
            } else {
                let mut new_bin = Bin::new(
                    active_bins.len() as i32,
                    bin_template.w,
                    bin_template.h,
                    bin_template.d,
                );
                let space = new_bin.free_spaces[0].clone();
                if let Some(fitted) = PlacementUtils::find_fit(box_item, &space, Some(&self.rotation_axes)) {
                    PlacementUtils::place_box_bsp(&fitted, &mut new_bin, 0);
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

        result
    }
}

impl BestFit3D {
    fn calculate_score(box_item: &BinBox, space: &crate::common::space::Space) -> f32 {
        let space_vol = space.w * space.h * space.d;
        let box_vol = box_item.size.x * box_item.size.y * box_item.size.z;
        let wasted_space_score = space_vol - box_vol;
        let distance_score = space.x + space.y + space.z;
        wasted_space_score + distance_score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::point3f::Point3f;
    use crate::common::space::Space;

    #[test]
    fn test_best_fit_3d_simple() {
        let mut solver = BestFit3D::default();
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
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].len(), 2);
    }
    
    #[test]
    fn test_calculate_score() {
        let box_item = BinBox::new_without_weight(1, Point3f::new(0.0, 0.0, 0.0), Point3f::new(2.0, 2.0, 2.0));
        let space = Space::new(1.0, 1.0, 1.0, 3.0, 3.0, 3.0);
        let score = BestFit3D::calculate_score(&box_item, &space);
        // space vol = 27, box vol = 8, wasted = 19
        // dist = 1+1+1 = 3
        // total = 22
        assert_eq!(score, 22.0);
    }
}
