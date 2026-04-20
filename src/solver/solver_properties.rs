use crate::common::bin::Bin;

#[derive(Debug, Clone)]
pub struct SolverProperties {
    pub bin: Bin,
    pub growing_bin: bool,
    pub grow_axis: String,
    pub rotation_axes: Vec<i32>,
    pub weight: f32,
}

impl SolverProperties {
    pub fn new(bin: Bin, growing_bin: bool, grow_axis: String, rotation_axes: Vec<i32>, weight: f32) -> Self {
        Self {
            bin,
            growing_bin,
            grow_axis,
            rotation_axes,
            weight,
        }
    }
}
