use crate::common::box_spec::BinBox;
use crate::solver::solver_properties::SolverProperties;
use crate::solver::solver_interface::Solver;

pub trait ParallelSolver: Send + Sync {
    fn is_template(&self) -> bool;
    fn is_compiled(&self) -> bool;
    fn compile_kernel(&mut self, max_bins: usize, max_spaces: usize);
    fn init(&mut self, properties: &SolverProperties);
    fn get_reference_solver(&mut self) -> Option<&mut dyn Solver>;
    fn solve(&mut self, boxes: &[BinBox], orders: &[Vec<usize>]) -> Vec<f64>;
    fn release(&mut self) {}
}

#[cfg(not(target_arch = "wasm32"))]
pub mod opencl_solver;

#[cfg(target_arch = "wasm32")]
pub mod webgpu_solver;
