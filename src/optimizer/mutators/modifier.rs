use crate::common::item::Item;
use crate::common::container::Container;
use crate::optimizer::solution::Solution;
use crate::solver::solver_interface::Solver;

pub type ModifierFn<I, C> = fn(
    &mut rand::rngs::ThreadRng,
    &Solution<I>,
    &Solution<I>,
    &C,
    &[I],
    &mut dyn Solver<I, C>,
) -> Vec<usize>;

// A trait version if we prefer struct-based modifiers (similar to Java functional interface):
pub trait Modifier<I: Item, C: Container>: Sync + Send {
    fn modify(
        &self,
        rng: &mut rand::rngs::ThreadRng,
        current: &Solution<I>,
        second: &Solution<I>,
        bin: &C,
        original_items: &[I],
    ) -> Vec<usize>;
}
