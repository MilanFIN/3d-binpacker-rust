use crate::common::box_spec::BinBox;

#[derive(Debug, Clone)]
pub struct Solution {
    pub order: Vec<usize>,
    pub score: f64,
    pub solved: Vec<Vec<BinBox>>,
}

impl Solution {
    pub fn new(order: Vec<usize>, score: f64, solved: Vec<Vec<BinBox>>) -> Self {
        Self {
            order,
            score,
            solved,
        }
    }
}
