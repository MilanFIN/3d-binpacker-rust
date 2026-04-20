use crate::common::box_spec::BinBox;
use crate::common::space::Space;
use crate::common::point3f::Point3f;

#[derive(Debug, Clone)]
pub struct Bin {
    pub boxes: Vec<BinBox>,
    pub free_spaces: Vec<Space>,
    pub index: i32,
    pub util_counter: i32,
    pub w: f32,
    pub h: f32,
    pub d: f32,
    pub weight: f32,
    pub max_weight: f32,
}

impl Bin {
    pub fn new(index: i32, w: f32, h: f32, d: f32) -> Self {
        Self {
            boxes: Vec::new(),
            free_spaces: vec![Space::new(0.0, 0.0, 0.0, w, h, d)],
            index,
            util_counter: 0,
            w,
            h,
            d,
            weight: 0.0,
            max_weight: 0.0,
        }
    }

    pub fn new_2d(index: i32, w: f32, h: f32) -> Self {
        Self::new(index, w, h, 0.0)
    }

    pub fn new_with_weight(index: i32, w: f32, h: f32, d: f32, max_weight: f32) -> Self {
        let mut bin = Self::new(index, w, h, d);
        bin.max_weight = max_weight;
        bin
    }

    pub fn from_template(index: i32, template: &BinBox) -> Self {
        Self::new(index, template.size.x, template.size.y, template.size.z)
    }

    pub fn volume(&self) -> f64 {
        (self.w * self.h * self.d) as f64
    }
}
