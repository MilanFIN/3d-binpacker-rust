# Unit Testing Plan for `rustport`

This document outlines the proposed unit testing strategy for the `rustport` crate, broken down by module. The goal is to ensure the core bin-packing logic, genetic optimizers, and WebAssembly interfaces function correctly and are resilient against regressions.

## 1. `common` Module (Data Models)
The `common` module contains the fundamental data structures. Tests here should verify logic for dimensions, intersections, and volume calculations.

*   **`bin.rs` & `box_spec.rs`**
    *   Test instantiation and dimension validation (e.g., preventing negative dimensions).
    *   Test volume and surface area calculations.
    *   Verify rotation logic (checking if a box can be rotated to specific orientations).
*   **`space.rs`**
    *   Test 3D space definitions and whether specific points or smaller spaces fit within a given space.
    *   Test overlap detection between two spaces.
*   **`utils.rs`**
    *   Test mathematical or utility functions, ensuring they handle edge cases correctly.

## 2. `solver` Module (Packing Algorithms)
This module contains the core packing algorithms (e.g., First Fit, Best Fit). Tests should focus on the correctness of placement and edge cases.

*   **`placement_utils.rs`**
    *   Test intersection logic between placed boxes.
    *   Test calculation of residual spaces (Empty Maximal Spaces - EMS) after a placement.
*   **`first_fit_3d.rs` & `best_fit_3d.rs`**
    *   **Simple Placement:** Test packing a few identical boxes that easily fit.
    *   **Overfill:** Test that the solver correctly rejects boxes when the bin is full.
    *   **Rotation:** Ensure boxes are rotated appropriately to fit when required.
    *   **Score Calculation:** Verify that the calculated fitness score matches expected values for a given packing result.
*   **`first_fit_ems.rs` & `best_fit_ems.rs`**
    *   Test similar scenarios as above, but specifically verifying that the Empty Maximal Spaces algorithm correctly tracks and utilizes available fragmented space.

## 3. `optimizer` Module (Genetic Algorithm)
The optimizer uses genetic algorithms to find optimal packing orders and rotations.

*   **`base.rs` & `mutators/`**
    *   **Mutation:** Test that mutation operations (swap, rotate, invert) correctly alter the sequence/rotations without introducing invalid states.
    *   **Crossover:** Test that crossover operations produce valid child sequences from two parent sequences.
    *   **Selection:** Verify the selection logic picks better-scoring individuals more frequently.
*   **`gpu_optimizer.rs`**
    *   Test fallback logic or mocked GPU interactions (if possible) to ensure the state machine behaves correctly during stateful iterations.
*   **`solution.rs`**
    *   Verify solution parsing and serialization.

## 4. Integration Tests (`tests/` directory)
In addition to the genetic algorithm logic, we must ensure that the orchestration of these algorithms operates correctly across different computing backends (CPU and GPU). 

*   **`CpuOptimizer` (`base.rs`)**:
    *   **Generation Loop:** Test that invoking `execute_generation` correctly processes a population, mutates/crosses them over, and produces a scored generation.
    *   **Convergence:** Run a simplified test case over multiple generations to verify that the best score strictly improves or plateaus, but never degrades (elitism check).
*   **`GpuOptimizer` (`gpu_optimizer.rs` / OpenCL Solver)**:
    *   **State Machine:** Test the stateful initialization, buffer creation, and generation loops to ensure that VRAM is managed correctly between generations.
    **`Leaving last since it's complex/inaccurate description` `Equivalence Testing`**: Run the same exact initial population and seed through both the `CpuOptimizer` and the `GpuOptimizer` (or a mock of it). They should theoretically converge on the same or highly similar results. This can be left last, since it's quite complex

Integration tests should validate the full execution stack of the library, particularly the public interfaces.

*   **`wasm_api.rs` (WebAssembly Interface)**:
    *   These tests should be run using a headless browser environment (e.g., `wasm-pack test --headless --chrome`) rather than standard `cargo test` because they rely on `wasm-bindgen` and browser-specific state.
    *   **Serialization/Deserialization:** Test that passing JSON strings or JS Objects for boxes/bins from the "frontend" correctly parses into Rust structs.
    *   **Full Pipeline:** Simulate a full frontend interaction: initialize a `WasmGeneticPool` with a JSON payload, run a few `step()` iterations, and request the `get_best_solution()` to ensure the full pipeline (JSON -> Rust -> WASM -> Rust -> JSON) works seamlessly.
    *   **Error Handling:** Ensure that invalid inputs trigger appropriate JavaScript-catchable errors.

## Execution
*   **Unit & Native Tests:** Run using Rust's built-in testing framework via `cargo test` for internal functions and the native CPU/GPU optimizers.
*   **WASM Integration Tests:** Run using `wasm-pack test --headless` to validate the frontend-facing API boundary.
