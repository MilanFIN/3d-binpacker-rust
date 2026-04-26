use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

use crate::common::bin::Bin;
use crate::common::box_spec::BinBox;
use crate::common::point3f::Point3f;
use crate::optimizer::base::CpuOptimizer;
use crate::solver::best_fit_ems::BestFitEMS;
use crate::solver::best_fit_3d::BestFit3D;
use crate::solver::first_fit_ems::FirstFitEMS;
use crate::solver::first_fit_3d::FirstFit3D;
use crate::solver::solver_interface::Solver;
use crate::solver::solver_properties::SolverProperties;

// ---------------------------------------------------------------------------
// JS-facing input/output types (plain serde structs)
// ---------------------------------------------------------------------------

/// A box item supplied from JavaScript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsBox {
    pub id: i32,
    pub w: f32,
    pub h: f32,
    pub d: f32,
    #[serde(default)]
    pub weight: f32,
}

/// Bin dimensions supplied from JavaScript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsBin {
    pub w: f32,
    pub h: f32,
    pub d: f32,
    #[serde(default)]
    pub max_weight: f32,
}

/// Configuration for the optimizer supplied from JavaScript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsConfig {
    pub bin: JsBin,
    pub boxes: Vec<JsBox>,
    /// "best_fit_ems" (default) or "first_fit_ems"
    #[serde(default = "default_solver")]
    pub solver: String,
    #[serde(default = "default_population")]
    pub population_size: usize,
    #[serde(default = "default_elite")]
    pub elite_count: usize,
    #[serde(default)]
    pub growing_bin: bool,
    #[serde(default = "default_grow_axis")]
    pub grow_axis: String,
    /// rotation axes to enable (0=x,1=y,2=z). Empty = all rotations off.
    #[serde(default)]
    pub rotation_axes: Vec<i32>,
    /// Number of threads (0 for max, 1 for single-threaded). Default is 0.
    #[serde(default = "default_threads")]
    pub threads: usize,
}

fn default_solver() -> String { "best_fit_ems".to_string() }
fn default_population() -> usize { 32 }
fn default_elite() -> usize { 4 }
fn default_grow_axis() -> String { "y".to_string() }
fn default_threads() -> usize { 0 }

// ---------------------------------------------------------------------------
// Output type
// ---------------------------------------------------------------------------

/// A packed box as returned to JavaScript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsPackedBox {
    pub id: i32,
    pub bin_index: usize,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    pub h: f32,
    pub d: f32,
    pub weight: f32,
}

/// Result of a generation – the best packing found so far.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsResult {
    pub packed: Vec<JsPackedBox>,
    /// Number of bins used.
    pub bin_count: usize,
    /// Optimiser score for this generation.
    pub score: f64,
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------

fn js_box_to_bin_box(b: &JsBox) -> BinBox {
    BinBox::new(
        b.id,
        Point3f::new(0.0, 0.0, 0.0),
        Point3f::new(b.w, b.h, b.d),
        b.weight,
    )
}

fn js_bin_to_bin(b: &JsBin) -> Bin {
    if b.max_weight > 0.0 {
        Bin::new_with_weight(0, b.w, b.h, b.d, b.max_weight)
    } else {
        Bin::new(0, b.w, b.h, b.d)
    }
}

fn pack_result(packed_bins: Vec<Vec<BinBox>>) -> Vec<JsPackedBox> {
    let mut out = Vec::new();
    for (bin_index, bin_boxes) in packed_bins.iter().enumerate() {
        for b in bin_boxes {
            out.push(JsPackedBox {
                id: b.id,
                bin_index,
                x: b.position.x,
                y: b.position.y,
                z: b.position.z,
                w: b.size.x,
                h: b.size.y,
                d: b.size.z,
                weight: b.weight,
            });
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Convenience function: build solver properties from config
// ---------------------------------------------------------------------------

fn make_solver_properties(cfg: &JsConfig) -> SolverProperties {
    SolverProperties::new(
        js_bin_to_bin(&cfg.bin),
        cfg.growing_bin,
        cfg.grow_axis.clone(),
        cfg.rotation_axes.clone(),
        cfg.bin.max_weight,
    )
}

// ---------------------------------------------------------------------------
// WasmOptimizer – stateful, runs generation by generation
// ---------------------------------------------------------------------------

/// Stateful genetic-algorithm optimizer exposed to JavaScript.
///
/// ```js
/// import init, { WasmOptimizer } from './rustport/pkg/rustport.js';
/// await init();
///
/// const opt = new WasmOptimizer({
///   bin:   { w: 100, h: 100, d: 100 },
///   boxes: [{ id: 1, w: 30, h: 30, d: 30 }, ...],
///   solver: "best_fit_ems",
///   population_size: 32,
///   elite_count: 4,
/// });
///
/// for (let i = 0; i < 50; i++) {
///   const result = opt.run_generation();
///   console.log(result);
/// }
/// ```
#[wasm_bindgen]
pub struct WasmOptimizer {
    // Boxed trait object so we don't need to expose the generic CpuOptimizer<F,S> to JS.
    inner: Box<dyn FnMut() -> JsResult>,
}

// We need to hold the concrete optimizer inside. Using an enum covers both solver variants
// without dynamic dispatch on the hot path.
enum AnyOptimizer {
    BestFitEMS(CpuOptimizer<Box<dyn Fn() -> BestFitEMS + Sync + Send>, BestFitEMS>),
    BestFit3D(CpuOptimizer<Box<dyn Fn() -> BestFit3D + Sync + Send>, BestFit3D>),
    FirstFitEMS(CpuOptimizer<Box<dyn Fn() -> FirstFitEMS + Sync + Send>, FirstFitEMS>),
    FirstFit3D(CpuOptimizer<Box<dyn Fn() -> FirstFit3D + Sync + Send>, FirstFit3D>),
}

impl AnyOptimizer {
    fn run_generation(&mut self) -> Vec<Vec<BinBox>> {
        match self {
            AnyOptimizer::BestFitEMS(o) => o.execute_next_generation(),
            AnyOptimizer::BestFit3D(o) => o.execute_next_generation(),
            AnyOptimizer::FirstFitEMS(o) => o.execute_next_generation(),
            AnyOptimizer::FirstFit3D(o) => o.execute_next_generation(),
        }
    }

    fn rate(&self, solution: &[Vec<BinBox>]) -> f64 {
        match self {
            AnyOptimizer::BestFitEMS(o) => o.rate(solution),
            AnyOptimizer::BestFit3D(o) => o.rate(solution),
            AnyOptimizer::FirstFitEMS(o) => o.rate(solution),
            AnyOptimizer::FirstFit3D(o) => o.rate(solution),
        }
    }
}

#[wasm_bindgen]
impl WasmOptimizer {
    /// Construct a new optimizer from a JS config object.
    ///
    /// Throws a JS error if config is invalid.
    #[wasm_bindgen(constructor)]
    pub fn new(config: JsValue) -> Result<WasmOptimizer, JsValue> {
        // Parse config
        let cfg: JsConfig = serde_wasm_bindgen::from_value(config)
            .map_err(|e| JsValue::from_str(&format!("Invalid config: {e}")))?;

        if cfg.boxes.is_empty() {
            return Err(JsValue::from_str("boxes array must not be empty"));
        }

        let props = make_solver_properties(&cfg);
        let boxes: Vec<BinBox> = cfg.boxes.iter().map(js_box_to_bin_box).collect();
        let bin = js_bin_to_bin(&cfg.bin);

        let mut opt = match cfg.solver.as_str() {
            "first_fit_ems" => {
                let p = props.clone();
                let factory: Box<dyn Fn() -> FirstFitEMS + Sync + Send> = Box::new(move || {
                    let mut s = FirstFitEMS::default();
                    s.init(&p);
                    s
                });
                AnyOptimizer::FirstFitEMS(CpuOptimizer::new(
                    factory, boxes, bin,
                    cfg.growing_bin, cfg.grow_axis.clone(), cfg.rotation_axes.clone(),
                    cfg.population_size, cfg.elite_count,
                    cfg.threads,
                ))
            }
            "first_fit_3d" => {
                let p = props.clone();
                let factory: Box<dyn Fn() -> FirstFit3D + Sync + Send> = Box::new(move || {
                    let mut s = FirstFit3D::default();
                    s.init(&p);
                    s
                });
                AnyOptimizer::FirstFit3D(CpuOptimizer::new(
                    factory, boxes, bin,
                    cfg.growing_bin, cfg.grow_axis.clone(), cfg.rotation_axes.clone(),
                    cfg.population_size, cfg.elite_count,
                    cfg.threads,
                ))
            }
            "best_fit_3d" => {
                let p = props.clone();
                let factory: Box<dyn Fn() -> BestFit3D + Sync + Send> = Box::new(move || {
                    let mut s = BestFit3D::default();
                    s.init(&p);
                    s
                });
                AnyOptimizer::BestFit3D(CpuOptimizer::new(
                    factory, boxes, bin,
                    cfg.growing_bin, cfg.grow_axis.clone(), cfg.rotation_axes.clone(),
                    cfg.population_size, cfg.elite_count,
                    cfg.threads,
                ))
            }
            _ => {
                // default: best_fit_ems
                let p = props.clone();
                let factory: Box<dyn Fn() -> BestFitEMS + Sync + Send> = Box::new(move || {
                    let mut s = BestFitEMS::default();
                    s.init(&p);
                    s
                });
                AnyOptimizer::BestFitEMS(CpuOptimizer::new(
                    factory, boxes, bin,
                    cfg.growing_bin, cfg.grow_axis.clone(), cfg.rotation_axes.clone(),
                    cfg.population_size, cfg.elite_count,
                    cfg.threads,
                ))
            }
        };

        // Wrap in a closure captured inside WasmOptimizer so JS only sees the opaque handle.
        let inner: Box<dyn FnMut() -> JsResult> = Box::new(move || {
            let packed_bins = opt.run_generation();
            let score = opt.rate(&packed_bins);
            let bin_count = packed_bins.len();
            let packed = pack_result(packed_bins);
            JsResult { packed, bin_count, score }
        });

        Ok(WasmOptimizer { inner })
    }

    /// Run one generation of the genetic algorithm.
    ///
    /// Returns a `JsResult` object: `{ packed: [...], bin_count, score }`.
    pub fn run_generation(&mut self) -> JsValue {
        let result = (self.inner)();
        serde_wasm_bindgen::to_value(&result).unwrap_or(JsValue::NULL)
    }
}

// ---------------------------------------------------------------------------
// One-shot convenience function (no optimizer state)
// ---------------------------------------------------------------------------

/// Pack boxes into bins in a single call (no optimisation loop).
///
/// ```js
/// import init, { pack } from './rustport/pkg/rustport.js';
/// await init();
///
/// const result = pack({
///   bin:   { w: 100, h: 100, d: 100 },
///   boxes: [{ id: 1, w: 40, h: 40, d: 40 }, { id: 2, w: 20, h: 20, d: 20 }],
/// });
/// console.log(result.packed);
/// ```
#[wasm_bindgen]
pub fn pack(config: JsValue) -> Result<JsValue, JsValue> {
    let cfg: JsConfig = serde_wasm_bindgen::from_value(config)
        .map_err(|e| JsValue::from_str(&format!("Invalid config: {e}")))?;

    if cfg.boxes.is_empty() {
        return Err(JsValue::from_str("boxes array must not be empty"));
    }

    let props = make_solver_properties(&cfg);
    let boxes: Vec<BinBox> = cfg.boxes.iter().map(js_box_to_bin_box).collect();

    let packed_bins = match cfg.solver.as_str() {
        "first_fit_ems" => {
            let mut solver = FirstFitEMS::default();
            solver.init(&props);
            solver.solve(&boxes)
        }
        "best_fit_3d" => {
            let mut solver = BestFit3D::default();
            solver.init(&props);
            solver.solve(&boxes)
        }
        "first_fit_3d" => {
            let mut solver = FirstFit3D::default();
            solver.init(&props);
            solver.solve(&boxes)
        }
        _ => {
            // default: best_fit_ems
            let mut solver = BestFitEMS::default();
            solver.init(&props);
            solver.solve(&boxes)
        }
    };

    let bin_count = packed_bins.len();
    let packed = pack_result(packed_bins);
    let result = JsResult { packed, bin_count, score: 0.0 };
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}
