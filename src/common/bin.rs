use crate::common::bin_box::BinBox;
use crate::common::container::Container;
use crate::common::space::Space;

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

impl Container for Bin {
    fn volume(&self) -> f64 {
        (self.w * self.h * self.d) as f64
    }

    fn max_weight(&self) -> f32 {
        self.max_weight
    }

    fn current_weight(&self) -> f32 {
        self.weight
    }

    fn w(&self) -> f32 {
        self.w
    }

    fn h(&self) -> f32 {
        self.h
    }

    fn d(&self) -> f32 {
        self.d
    }
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
}

#[cfg(test)]
mod tests {
    use crate::common::point3f::Point3f;

use super::*;

    #[test]
    fn test_bin_new() {
        let bin = Bin::new(1, 10.0, 20.0, 30.0);
        assert_eq!(bin.index, 1);
        assert_eq!(bin.volume(), 6000.0);
        assert_eq!(bin.free_spaces.len(), 1);
        assert_eq!(bin.free_spaces[0].w, 10.0);
    }
    
    #[test]
    fn test_bin_from_template() {
        let template = BinBox::new_without_weight(0, Point3f::new(0.0, 0.0, 0.0), Point3f::new(5.0, 5.0, 5.0));
        let bin = Bin::from_template(2, &template);
        assert_eq!(bin.index, 2);
        assert_eq!(bin.volume(), 125.0);
    }
}
