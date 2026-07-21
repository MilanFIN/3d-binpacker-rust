use crate::common::item::Item;

#[derive(Debug, Clone)]
pub struct PackResult<I: Item> {
    pub order: Vec<usize>,
    pub score: f64,
    pub bins: Vec<Vec<I>>,
}

impl<I: Item> PackResult<I> {
    pub fn new(order: Vec<usize>, score: f64, bins: Vec<Vec<I>>) -> Self {
        Self { order, score, bins }
    }
}
