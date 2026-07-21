use crate::common::bin_box::BinBox;
use crate::solver::common::solver_properties::SolverProperties;
use crate::solver::solver_interface::Solver;

use crate::common::bin::Bin;

pub trait ParallelSolver: Send + Sync {
    fn is_template(&self) -> bool;
    fn is_compiled(&self) -> bool;
    fn compile_kernel(&mut self, max_bins: usize, max_spaces: usize);
    fn init(&mut self, properties: &SolverProperties<Bin>);
    fn get_reference_solver(&mut self) -> Option<&mut dyn Solver<BinBox, Bin>>;
    fn solve(&mut self, boxes: &[BinBox], orders: &[Vec<usize>]) -> Vec<f64>;
    fn release(&mut self) {}
}

#[cfg(all(not(target_arch = "wasm32"), feature = "opencl"))]
pub mod opencl_solver;

