use crate::common::box_spec::BinBox;
use crate::solver::solver_properties::SolverProperties;

pub trait Solver {
    fn init(&mut self, properties: &SolverProperties);
    
    fn solve(&mut self, boxes: &[BinBox]) -> Vec<Vec<BinBox>>;
    
    // release() is generally handled by the `Drop` trait automatically in Rust,
    // but if explicit cleanup via a method is preferred, we can keep it here.
    fn release(&mut self) {}
}
