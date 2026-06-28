use rand::Rng;
use crate::common::bin::Bin;
use crate::common::box_spec::BinBox;
use crate::optimizer::solution::Solution;

use crate::solver::solver_interface::Solver;

pub type ModifierFn = fn(
    &mut rand::rngs::ThreadRng,
    &Solution,
    &Solution,
    &Bin,
    &[BinBox],
    &mut dyn Solver,
) -> Vec<usize>;

// A trait version if we prefer struct-based modifiers (similar to Java functional interface):
pub trait Modifier: Sync + Send {
    fn modify(
        &self,
        rng: &mut rand::rngs::ThreadRng,
        current: &Solution,
        second: &Solution,
        bin: &Bin,
        original_boxes: &[BinBox],
    ) -> Vec<usize>;
}
