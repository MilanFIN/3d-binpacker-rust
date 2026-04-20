use crate::common::bin::Bin;
use crate::common::box_spec::BinBox;
use crate::solver::solver_properties::SolverProperties;

pub trait ReferenceSolver {
    fn solve(
        &mut self,
        boxes: &[BinBox],
        order: &[usize],
        properties: &SolverProperties,
    ) -> Vec<Bin>;
}
