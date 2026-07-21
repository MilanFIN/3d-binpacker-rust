use crate::common::bin::Bin;
use crate::common::bin_box::BinBox;
use crate::solver::common::solver_properties::SolverProperties;

pub trait ReferenceSolver {
    fn solve(
        &mut self,
        boxes: &[BinBox],
        order: &[usize],
        properties: &SolverProperties<Bin>,
    ) -> Vec<Bin>;
}
