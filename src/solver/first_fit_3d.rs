use crate::common::bin::Bin;
use crate::common::box_spec::BinBox;
use crate::solver::placement_utils::PlacementUtils;
use crate::solver::solver_interface::Solver;
use crate::solver::solver_properties::SolverProperties;

pub struct FirstFit3D {
    bin_template: Option<Bin>,
    growing_bin: bool,
    grow_axis: String,
    rotation_axes: Vec<i32>,
    weight_limit: f32,
}

impl Default for FirstFit3D {
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

impl Solver for FirstFit3D {
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
            let mut placed = false;
            for bin in &mut active_bins {
                if self.weight_limit > 0.0 && bin.weight + box_item.weight > self.weight_limit {
                    continue;
                }
                
                let num_spaces = bin.free_spaces.len();
                for i in 0..num_spaces {
                    let space = bin.free_spaces[i].clone();
                    if let Some(fitted_box) = PlacementUtils::find_fit(box_item, &space, Some(&self.rotation_axes)) {
                        PlacementUtils::place_box_bsp(&fitted_box, bin, i);
                        placed = true;
                        break;
                    }
                }
                if placed {
                    break;
                }
            }

            if !self.growing_bin && !placed {
                let mut new_bin = Bin::new(
                    active_bins.len() as i32,
                    bin_template.w,
                    bin_template.h,
                    bin_template.d,
                );
                
                let space = new_bin.free_spaces[0].clone();
                if let Some(fitted_box) = PlacementUtils::find_fit(box_item, &space, Some(&self.rotation_axes)) {
                    PlacementUtils::place_box_bsp(&fitted_box, &mut new_bin, 0);
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
