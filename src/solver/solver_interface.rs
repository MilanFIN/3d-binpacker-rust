use crate::common::item::Item;
use crate::common::container::Container;
use crate::common::pack_result::PackResult;
use crate::solver::common::solver_properties::SolverProperties;

pub trait Solver<I: Item, C: Container> {
    fn init(&mut self, properties: &SolverProperties<C>);
    
    fn solve(&mut self, items: &[I]) -> PackResult<I>;
    
    // release() is generally handled by the `Drop` trait automatically in Rust,
    // but if explicit cleanup via a method is preferred, we can keep it here.
    fn release(&mut self) {}
}
