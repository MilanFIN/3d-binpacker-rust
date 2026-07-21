use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

use crate::common::bin::Bin;
use crate::common::bin_box::BinBox;
use crate::common::item::Item;
use crate::common::container::Container;
use crate::common::point3f::Point3f;
use crate::optimizer::base::CpuOptimizer;
use crate::solver::rectangles::best_fit_ems::BestFitEMS;
use crate::solver::rectangles::best_fit_3d::BestFit3D;
use crate::solver::rectangles::first_fit_ems::FirstFitEMS;
use crate::solver::rectangles::first_fit_3d::FirstFit3D;
use crate::solver::solver_interface::Solver;
use crate::solver::common::solver_properties::SolverProperties;
use crate::solver::parallelsolvers::ParallelSolver;
use web_sys::console;
use wasm_bindgen::JsCast;

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
                x: b.position().x,
                y: b.position().y,
                z: b.position().z,
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

fn make_solver_properties(cfg: &JsConfig) -> SolverProperties<Bin> {
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
    BestFitEMS(CpuOptimizer<BinBox, BestFitEMS, Bin>),
    BestFit3D(CpuOptimizer<BinBox, BestFit3D, Bin>),
    FirstFitEMS(CpuOptimizer<BinBox, FirstFitEMS, Bin>),
    FirstFit3D(CpuOptimizer<BinBox, FirstFit3D, Bin>),
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
                AnyOptimizer::FirstFitEMS(CpuOptimizer::<BinBox, FirstFitEMS, Bin>::new(
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
                AnyOptimizer::FirstFit3D(CpuOptimizer::<BinBox, FirstFit3D, Bin>::new(
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
                AnyOptimizer::BestFit3D(CpuOptimizer::<BinBox, BestFit3D, Bin>::new(
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
                AnyOptimizer::BestFitEMS(CpuOptimizer::<BinBox, BestFitEMS, Bin>::new(
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
            solver.solve(&boxes).bins
        }
        "best_fit_3d" => {
            let mut solver = BestFit3D::default();
            solver.init(&props);
            solver.solve(&boxes).bins
        }
        "first_fit_3d" => {
            let mut solver = FirstFit3D::default();
            solver.init(&props);
            solver.solve(&boxes).bins
        }
        _ => {
            // default: best_fit_ems
            let mut solver = BestFitEMS::default();
            solver.init(&props);
            solver.solve(&boxes).bins
        }
    };

    let bin_count = packed_bins.len();
    let packed = pack_result(packed_bins);
    let result = JsResult { packed, bin_count, score: 0.0 };
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// GpuGenerationState & WasmGeneticPool for WebGPU
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub struct GpuGenerationState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    params_buf: wgpu::Buffer,
    params_data: [u32; 8],
    orders_buf: wgpu::Buffer,
    scores_buf: wgpu::Buffer,
    scores_staging: wgpu::Buffer,
    pop_size: u32,
    batch_size: u32,
}

#[wasm_bindgen]
pub async fn init_gpu_generation_state(
    boxes_flat: &[f32],
    orders_flat: &[i32],
    bin_w: f32, bin_h: f32, bin_d: f32, bin_weight: f32, rotation_mask: u32,
    max_bins: u32, max_spaces_per_bin: u32, batch_size: u32
) -> Result<GpuGenerationState, JsValue> {
    let max_spaces = max_spaces_per_bin;

    let instance = wgpu::Instance::default();
    let adapter_fut = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        force_fallback_adapter: false,
        compatible_surface: None,
    });
    let adapter: wgpu::Adapter = match adapter_fut.await {
        Ok(a) => a,
        Err(e) => return Err(JsValue::from_str(&format!("Failed to find adapter: {:?}", e))),
    };

    let device_fut = adapter.request_device(&wgpu::DeviceDescriptor::default());
    let (device, queue): (wgpu::Device, wgpu::Queue) = match device_fut.await {
        Ok(dq) => dq,
        Err(e) => return Err(JsValue::from_str(&format!("Failed to request device: {:?}", e))),
    };

    let shader_src = include_str!("kernels/bestfit_ems.wgsl");
    let cs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("bestfit_ems"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_src)),
    });

    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: true }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 5, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 6, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            wgpu::BindGroupLayoutEntry { binding: 7, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None, bind_group_layouts: &[Some(&bgl)], immediate_size: 0,
    });

    let constants = vec![
        ("MAX_SPACES_PER_BIN", max_spaces as f64),
    ];

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Compute Pipeline"),
        layout: Some(&pipeline_layout),
        module: &cs_module,
        entry_point: Some("best_fit_ems"),
        compilation_options: wgpu::PipelineCompilationOptions {
            constants: &constants,
            ..Default::default()
        },
        cache: None,
    });
    
    let num_boxes = (boxes_flat.len() / 4) as u32;
    let pop_size = (orders_flat.len() as u32) / num_boxes;

    use wgpu::util::DeviceExt;
    let boxes_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("boxes"),
        contents: bytemuck::cast_slice(boxes_flat),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let orders_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("orders"),
        contents: bytemuck::cast_slice(orders_flat),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });

    let scores_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scores"),
        size: (pop_size * 4) as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let mut params_data = [0u32; 8];
    params_data[0] = num_boxes;
    params_data[1] = bin_w.to_bits();
    params_data[2] = bin_h.to_bits();
    params_data[3] = bin_d.to_bits();
    params_data[4] = bin_weight.to_bits();
    params_data[5] = rotation_mask;

    let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("params"),
        contents: bytemuck::cast_slice(&params_data),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let spaces_size = (pop_size as u64) * (max_bins as u64) * (max_spaces as u64) * 24;
    let spaces_buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("spaces_store"),
        size: spaces_size,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });

    let counts_size = (pop_size as u64) * (max_bins as u64) * 4;
    let sc_buf = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: counts_size, usage: wgpu::BufferUsages::STORAGE, mapped_at_creation: false });
    let vol_buf = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: counts_size, usage: wgpu::BufferUsages::STORAGE, mapped_at_creation: false });
    let wt_buf = device.create_buffer(&wgpu::BufferDescriptor { label: None, size: counts_size, usage: wgpu::BufferUsages::STORAGE, mapped_at_creation: false });

    let scores_staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scores_staging"),
        size: (pop_size * 4) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: boxes_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: orders_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 2, resource: scores_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 3, resource: params_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 4, resource: spaces_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 5, resource: sc_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 6, resource: vol_buf.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 7, resource: wt_buf.as_entire_binding() },
        ],
    });

    Ok(GpuGenerationState {
        device,
        queue,
        pipeline,
        bind_group,
        params_buf,
        params_data,
        orders_buf,
        scores_buf,
        scores_staging,
        pop_size,
        batch_size,
    })
}
#[wasm_bindgen]
impl GpuGenerationState {
    pub async fn evaluate(&mut self, orders_flat: &[i32]) -> Result<js_sys::Float32Array, JsValue> {
        // ------------------------------------------------------------
        // 1. Upload input ONCE
        // ------------------------------------------------------------
        self.queue.write_buffer(
            &self.orders_buf,
            0,
            bytemuck::cast_slice(orders_flat),
        );

        // ------------------------------------------------------------
        // 2. Build ONE command buffer with multiple dispatches
        // ------------------------------------------------------------
        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: None }
        );

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: None,
                timestamp_writes: None,
            });

            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            pass.dispatch_workgroups(self.pop_size, 1, 1);
        }

        // ------------------------------------------------------------
        // COPY MUST HAPPEN BEFORE finish()
        // ------------------------------------------------------------
        encoder.copy_buffer_to_buffer(
            &self.scores_buf,
            0,
            &self.scores_staging,
            0,
            (self.pop_size * 4) as u64,
        );

        // ------------------------------------------------------------
        // NOW finish and submit
        // ------------------------------------------------------------
        self.queue.submit(Some(encoder.finish()));
        // ------------------------------------------------------------
        // 4. ONLY NOW read back results — with 500ms watchdog
        // ------------------------------------------------------------
        let scores_slice = self.scores_staging.slice(..);
        let (tx, rx) = futures::channel::oneshot::channel();

        scores_slice.map_async(wgpu::MapMode::Read, move |v| {
            let _ = tx.send(v);
        });

        // --- original: bare await (no timeout) ---
        rx.await.map_err(|_| JsValue::from_str("Map async failed"))?
            .map_err(|_| JsValue::from_str("Map async error"))?;

        // A JS-based timeout future (500 ms).
        // We use a raw JS Promise wrapping setTimeout so we don't need gloo_timers.
        // This is unusable as it lets the gpu work continue until it finishes...
        // let timeout_promise = js_sys::Promise::new(&mut |resolve, _reject| {
        //     let global = js_sys::global();
        //     let scope: &web_sys::WorkerGlobalScope = global.unchecked_ref();
        //     scope
        //         .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, 500)
        //         .expect("set_timeout failed");
        // });
        // let timeout_fut = wasm_bindgen_futures::JsFuture::from(timeout_promise);

        // // Race the GPU readback against the timeout.
        // use futures::future::{self, Either};
        // match future::select(Box::pin(rx), Box::pin(timeout_fut)).await {
        //     Either::Left((map_result, _timeout)) => {
        //         // GPU finished within the deadline.
        //         map_result
        //             .map_err(|_| JsValue::from_str("Map async failed"))?
        //             .map_err(|_| JsValue::from_str("Map async error"))?;
        //     }
        //     Either::Right((_timeout_val, rx)) => {
        //         // Timeout fired, but the GPU will still complete map_async eventually.
        //         // We MUST await rx and call unmap() before returning, otherwise
        //         // scores_staging is left in a mapped state and the next evaluate()
        //         // call panics with "Buffer is already mapped".
        //         let _ = rx.await; // wait for the in-flight map_async to land
        //         self.scores_staging.unmap();
        //         return Err(JsValue::from_str(
        //             "GPU evaluate timeout: dispatch exceeded 500 ms",
        //         ));
        //     }
        // }

        let data = scores_slice.get_mapped_range();
        let result: &[f32] = bytemuck::cast_slice(&data);

        let mut out = js_sys::Float32Array::new_with_length(result.len() as u32);
        for (i, v) in result.iter().enumerate() {
            out.set_index(i as u32, *v);
        }

        drop(data);
        self.scores_staging.unmap();

        let mut v = vec![0.0; out.length() as usize];
        out.copy_to(&mut v);

        console::log_1(&format!("array: {:?}", v).into());

        Ok(out)
    }
}

/// Genetic pool state mapped in WASM, avoiding JS object allocation overhead
#[wasm_bindgen]
pub struct WasmGeneticPool {
    boxes: Vec<BinBox>,
    bin: Bin,
    growing_bin: bool,
    elite_count: usize,
    population_size: usize,
    box_orders: Vec<Vec<usize>>,
    properties: SolverProperties<Bin>,
    solver_type: String,
}

#[wasm_bindgen]
impl WasmGeneticPool {
    #[wasm_bindgen(constructor)]
    pub fn new(config: JsValue) -> Result<WasmGeneticPool, JsValue> {
        let cfg: JsConfig = serde_wasm_bindgen::from_value(config)
            .map_err(|e| JsValue::from_str(&format!("Invalid Config: {e}")))?;

        if cfg.boxes.is_empty() {
            return Err(JsValue::from_str("boxes array empty"));
        }

        let properties = make_solver_properties(&cfg);
        let solver_type = cfg.solver.clone();

        let boxes: Vec<BinBox> = cfg.boxes.iter().map(js_box_to_bin_box).collect();
        let bin = js_bin_to_bin(&cfg.bin);

        let mut pool = WasmGeneticPool {
            boxes,
            bin,
            growing_bin: cfg.growing_bin,
            elite_count: cfg.elite_count,
            population_size: cfg.population_size,
            box_orders: Vec::new(),
            properties,
            solver_type,
        };

        pool.generate_initial_population();
        Ok(pool)
    }

    fn generate_initial_population(&mut self) {
        use std::cmp::Ordering;
        let base_order: Vec<usize> = (0..self.boxes.len()).collect();

        let mut growing_order = base_order.clone();
        growing_order.sort_by(|&a, &b| {
            self.boxes[a].volume().partial_cmp(&self.boxes[b].volume()).unwrap_or(Ordering::Equal)
        });
        self.box_orders.push(growing_order);

        let mut shrinking_order = base_order.clone();
        shrinking_order.sort_by(|&a, &b| {
            self.boxes[b].volume().partial_cmp(&self.boxes[a].volume()).unwrap_or(Ordering::Equal)
        });
        self.box_orders.push(shrinking_order);

        let mut shrinking_longest_order = base_order.clone();
        shrinking_longest_order.sort_by(|&a, &b| {
            self.boxes[b].longest_side().partial_cmp(&self.boxes[a].longest_side()).unwrap_or(Ordering::Equal)
        });
        self.box_orders.push(shrinking_longest_order);

        let mut rng = rand::thread_rng();
        use rand::seq::SliceRandom;
        while self.box_orders.len() < self.population_size {
            let mut order = base_order.clone();
            order.shuffle(&mut rng);
            self.box_orders.push(order);
        }
    }

    pub fn get_current_orders_flat(&self) -> js_sys::Int32Array {
        let num_boxes = self.boxes.len();
        let arr = js_sys::Int32Array::new_with_length((self.population_size * num_boxes) as u32);
        for p in 0..self.population_size {
            for b in 0..num_boxes {
                arr.set_index((p * num_boxes + b) as u32, self.box_orders[p][b] as i32);
            }
        }
        arr
    }

    pub fn advance_generation(&mut self, scores_flat: &[f32]) {
        use crate::optimizer::solution::Solution;
        use std::cmp::Ordering;

        let mut scored = Vec::with_capacity(self.population_size);
        for i in 0..self.population_size {
            scored.push(Solution::new(self.box_orders[i].clone(), scores_flat[i] as f64, vec![]));
        }

        if !self.growing_bin {
            scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        } else {
            scored.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal));
        }

        let mut next_gen = Vec::new();
        for i in 0..self.elite_count.min(scored.len()) {
            next_gen.push(scored[i].order.clone());
            console::log_1(&format!("elite: {:?}", scored[i].score).into());

        }

        let mut rng = rand::thread_rng();
        let max_elite = 1.max(self.elite_count.min(scored.len()));

        use crate::optimizer::mutators::{
            crossover, insert_mutation, scramble_mutation, swap_mutation,
            bin_preservation_crossover, space_mutation
        };

        let modifiers = vec![
            crossover::modify, 
            swap_mutation::modify,
            insert_mutation::modify,
            scramble_mutation::modify,
            bin_preservation_crossover::modify,
            space_mutation::modify,
        ];

        let mut mutation_solver: Box<dyn crate::solver::solver_interface::Solver<BinBox, Bin>> = match self.solver_type.as_str() {
            "first_fit_ems" => {
                let mut s = crate::solver::rectangles::first_fit_ems::FirstFitEMS::default();
                s.init(&self.properties);
                Box::new(s)
            }
            "best_fit_3d" => {
                let mut s = crate::solver::rectangles::best_fit_3d::BestFit3D::default();
                s.init(&self.properties);
                Box::new(s)
            }
            "first_fit_3d" => {
                let mut s = crate::solver::rectangles::first_fit_3d::FirstFit3D::default();
                s.init(&self.properties);
                Box::new(s)
            }
            _ => {
                let mut s = crate::solver::rectangles::best_fit_ems::BestFitEMS::default();
                s.init(&self.properties);
                Box::new(s)
            }
        };

        while next_gen.len() < self.population_size {
            let modifier = modifiers[rand::Rng::gen_range(&mut rng, 0..modifiers.len())];
            let current_sequence = &scored[rand::Rng::gen_range(&mut rng, 0..max_elite)];
            let second_sequence = &scored[rand::Rng::gen_range(&mut rng, 0..max_elite)];

            let child = modifier(&mut rng, current_sequence, second_sequence, &self.bin, &self.boxes, &mut *mutation_solver);
            next_gen.push(child);
        }

        self.box_orders = next_gen;
    }

    pub fn get_best_order(&self) -> js_sys::Int32Array {
        let arr = js_sys::Int32Array::new_with_length(self.boxes.len() as u32);
        for i in 0..self.boxes.len() {
            arr.set_index(i as u32, self.box_orders[0][i] as i32);
        }
        arr
    }
}

#[wasm_bindgen]
pub fn evaluate_single_placement(config: JsValue, best_order: &[i32]) -> Result<JsValue, JsValue> {
    let cfg: JsConfig = serde_wasm_bindgen::from_value(config)
        .map_err(|e| JsValue::from_str(&format!("Invalid config: {e}")))?;

    let props = make_solver_properties(&cfg);
    let boxes: Vec<BinBox> = cfg.boxes.iter().map(js_box_to_bin_box).collect();
    
    let mut ordered_boxes = Vec::with_capacity(boxes.len());
    for &idx in best_order {
        ordered_boxes.push(boxes[idx as usize].clone());
    }

    let mut solver = BestFitEMS::default();
    solver.init(&props);
    let packed_bins = solver.solve(&ordered_boxes).bins;

    let bin_count = packed_bins.len();
    
    let mut total_used_volume = 0.0_f64;
    let bins_to_consider = packed_bins.len().saturating_sub(1);
    for i in 0..bins_to_consider {
        let mut current_bin_used_volume = 0.0_f64;
        for box_spec in &packed_bins[i] {
            current_bin_used_volume += box_spec.volume();
        }
        total_used_volume += current_bin_used_volume;
    }
    let score = if bins_to_consider > 0 { total_used_volume / (bins_to_consider as f64 * props.bin.volume()) } else { 1.0 };

    let packed = pack_result(packed_bins);
    let result = JsResult { packed, bin_count, score };
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}


use crate::common::sphere_spec::Sphere;
use crate::solver::spheres::first_fit::FirstFitSpheres;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsSphere {
    pub id: i32,
    pub radius: f32,
    #[serde(default)]
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsPackedSphere {
    pub id: i32,
    pub bin_index: usize,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub radius: f32,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsResultSpheres {
    pub packed: Vec<JsPackedSphere>,
    pub bin_count: usize,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsConfigSpheres {
    pub bin: JsBin,
    pub spheres: Vec<JsSphere>,
}

#[wasm_bindgen]
pub fn pack_spheres(config: JsValue) -> Result<JsValue, JsValue> {
    let cfg: JsConfigSpheres = serde_wasm_bindgen::from_value(config)
        .map_err(|e| JsValue::from_str(&format!("Invalid config: {e}")))?;

    if cfg.spheres.is_empty() {
        return Err(JsValue::from_str("spheres array must not be empty"));
    }

    let bin = js_bin_to_bin(&cfg.bin);
    let mut properties = crate::solver::common::solver_properties::SolverProperties::<Bin>::new(
        bin, false, "y".to_string(), vec![], 0.0
    );

    let spheres: Vec<Sphere> = cfg.spheres.iter().map(|s| {
        Sphere::new(s.id, Point3f::new(0.0, 0.0, 0.0), s.radius, s.weight)
    }).collect();

    let mut solver = FirstFitSpheres::default();
    solver.init(&properties);
    let packed_bins = solver.solve(&spheres).bins;

    let mut out = Vec::new();
    for (bin_index, bin_spheres) in packed_bins.iter().enumerate() {
        for s in bin_spheres {
            out.push(JsPackedSphere {
                id: s.id,
                bin_index,
                x: s.position.x,
                y: s.position.y,
                z: s.position.z,
                radius: s.radius,
                weight: s.weight,
            });
        }
    }

    let bin_count = packed_bins.len();
    let result = JsResultSpheres { packed: out, bin_count, score: 0.0 };
    serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
}
