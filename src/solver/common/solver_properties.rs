use crate::common::container::Container;

#[derive(Debug, Clone)]
pub struct SolverProperties<C: Container> {
    pub bin: C,
    pub growing_bin: bool,
    pub grow_axis: String,
    pub rotation_axes: Vec<i32>,
    pub weight: f32,
}

impl<C: Container> SolverProperties<C> {
    pub fn new(bin: C, growing_bin: bool, grow_axis: String, rotation_axes: Vec<i32>, weight: f32) -> Self {
        Self {
            bin,
            growing_bin,
            grow_axis,
            rotation_axes,
            weight,
        }
    }
}
