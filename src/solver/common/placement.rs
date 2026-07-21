use crate::common::bin_box::BinBox;

#[derive(Debug, Clone)]
pub struct Placement {
    pub box_item: BinBox,
    pub space_index: usize,
}

impl Placement {
    pub fn new(box_item: BinBox, space_index: usize) -> Self {
        Self {
            box_item,
            space_index,
        }
    }
}
