pub mod base;
pub mod solution;
pub mod mutators;

#[cfg(all(not(target_arch = "wasm32"), feature = "opencl"))]
pub mod gpu_optimizer;
