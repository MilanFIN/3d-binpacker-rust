use crate::common::container::Container;
use crate::common::item::Item;

pub trait Postprocessor<I: Item, C: Container>: Send + Sync {
    /// Postprocesses a full packing solution (sequence of packed bins).
    /// Typically modifies only the last bin (`solution.last_mut()`),
    /// but receives the entire slice to allow context-aware adjustments.
    fn process(&self, solution: &mut [Vec<I>], bin: &C);
}
