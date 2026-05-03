use crate::common::box_spec::BinBox;
use crate::solver::solver_interface::Solver;
use crate::solver::solver_properties::SolverProperties;
use crate::solver::parallelsolvers::ParallelSolver;

/// A dummy WebGPU based solver implementation for WebAssembly
pub struct WebGPUSolver {
    reference_solver: Box<dyn Solver + Send + Sync>,
}

impl WebGPUSolver {
    pub fn new(
        _kernel_file_name: &str,
        _kernel_function_name: &str,
        _display_name: &str,
        reference_solver: Box<dyn Solver + Send + Sync>,
    ) -> Self {
        Self {
            reference_solver,
        }
    }
}

impl ParallelSolver for WebGPUSolver {
    fn is_template(&self) -> bool {
        false
    }

    fn is_compiled(&self) -> bool {
        true
    }

    fn compile_kernel(&mut self, _max_bins: usize, _max_spaces: usize) {
        // dummy: no-op since no WebGPU backend is implemented yet
    }

    fn init(&mut self, _properties: &SolverProperties) {
        // dummy: no-op
    }

    fn get_reference_solver(&mut self) -> Option<&mut dyn Solver> {
        Some(&mut *self.reference_solver)
    }

    fn solve(&mut self, _boxes: &[BinBox], orders: &[Vec<usize>]) -> Vec<f64> {
        // dummy: return 0 for all orders since we don't have a real WebGPU implementation
        vec![0.0; orders.len()]
    }
}
