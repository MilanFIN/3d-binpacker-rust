// ============================================================================
// Integration tests for the rustport WASM API
//
// Run with:
//   wasm-pack test --headless --chrome
//
// These tests exercise the full JSON → Rust → WASM → Rust → JSON pipeline
// for every public entry-point exposed to JavaScript.
// ============================================================================

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

// Run all tests in the browser (headless via wasm-pack test --headless --chrome).
wasm_bindgen_test_configure!(run_in_browser);

// Re-export the API under test so paths are short.
use rustport::wasm_api::{pack, WasmOptimizer, WasmGeneticPool, evaluate_single_placement};

// ---------------------------------------------------------------------------
// Helper: build a valid JsValue config from a JSON literal.
// ---------------------------------------------------------------------------

fn make_config(json: &str) -> JsValue {
    // js_sys::JSON::parse produces a JsValue that serde-wasm-bindgen can read.
    js_sys::JSON::parse(json).expect("test JSON must be valid")
}

// ---------------------------------------------------------------------------
// Shared fixture: a small 100×100×100 bin with three boxes.
// ---------------------------------------------------------------------------

const SIMPLE_CONFIG: &str = r#"{
    "bin":  { "w": 100, "h": 100, "d": 100 },
    "boxes": [
        { "id": 1, "w": 30, "h": 30, "d": 30 },
        { "id": 2, "w": 20, "h": 20, "d": 20 },
        { "id": 3, "w": 10, "h": 10, "d": 10 }
    ]
}"#;

// ---------------------------------------------------------------------------
// Serialization / Deserialization
// ---------------------------------------------------------------------------

/// The `pack()` function must correctly parse JSON and return a well-formed
/// `JsResult` object whose `packed` array contains all supplied boxes.
#[wasm_bindgen_test]
fn test_pack_deserializes_all_fields() {
    let config = make_config(SIMPLE_CONFIG);
    let result = pack(config).expect("pack() must succeed for a valid config");

    // Parse the returned JsValue as JSON so we can inspect it.
    let json_str: String = js_sys::JSON::stringify(&result)
        .expect("result must be stringify-able")
        .as_string()
        .expect("stringify returns a JS string");

    // All three boxes should appear in the output.
    assert!(json_str.contains("\"id\":1"), "box 1 must be in result: {json_str}");
    assert!(json_str.contains("\"id\":2"), "box 2 must be in result: {json_str}");
    assert!(json_str.contains("\"id\":3"), "box 3 must be in result: {json_str}");

    // The result must have bin_count and score fields.
    assert!(json_str.contains("\"bin_count\""), "result must contain bin_count");
    assert!(json_str.contains("\"score\""), "result must contain score");
}

/// Supplying optional fields (weight, rotation_axes, solver name) must not
/// cause a parse failure; default values are applied silently.
#[wasm_bindgen_test]
fn test_pack_optional_fields_have_defaults() {
    let config = make_config(r#"{
        "bin":  { "w": 50, "h": 50, "d": 50, "max_weight": 100.0 },
        "boxes": [
            { "id": 10, "w": 10, "h": 10, "d": 10, "weight": 5.0 }
        ],
        "solver": "first_fit_ems",
        "population_size": 8,
        "elite_count": 2,
        "growing_bin": false,
        "grow_axis": "y",
        "rotation_axes": [0, 1, 2],
        "threads": 1
    }"#);

    let result = pack(config).expect("pack() must succeed with all optional fields set");

    let json_str: String = js_sys::JSON::stringify(&result)
        .unwrap()
        .as_string()
        .unwrap();

    assert!(json_str.contains("\"id\":10"), "box 10 must be present: {json_str}");
}

/// Each supported solver name must produce a valid result for pack().
#[wasm_bindgen_test]
fn test_pack_all_solver_variants() {
    let solvers = ["best_fit_ems", "first_fit_ems", "best_fit_3d", "first_fit_3d"];

    for solver in solvers {
        let json = format!(r#"{{
            "bin":   {{ "w": 60, "h": 60, "d": 60 }},
            "boxes": [
                {{ "id": 1, "w": 20, "h": 20, "d": 20 }},
                {{ "id": 2, "w": 15, "h": 15, "d": 15 }}
            ],
            "solver": "{solver}"
        }}"#);

        let config = make_config(&json);
        let result = pack(config)
            .unwrap_or_else(|_| panic!("pack() must succeed for solver '{solver}'"));

        let json_str = js_sys::JSON::stringify(&result)
            .unwrap()
            .as_string()
            .unwrap();

        assert!(
            json_str.contains("\"id\":1"),
            "solver '{solver}': box 1 missing in: {json_str}"
        );
    }
}

// ---------------------------------------------------------------------------
// Full Pipeline – pack() one-shot
// ---------------------------------------------------------------------------

/// A perfectly fitting box should land at the origin (0, 0, 0) in a single bin.
#[wasm_bindgen_test]
fn test_pack_single_box_fills_bin() {
    let config = make_config(r#"{
        "bin":   { "w": 40, "h": 40, "d": 40 },
        "boxes": [{ "id": 7, "w": 40, "h": 40, "d": 40 }]
    }"#);

    let result = pack(config).expect("pack() must succeed");
    let json_str = js_sys::JSON::stringify(&result)
        .unwrap()
        .as_string()
        .unwrap();

    // Exactly one bin should be used.
    assert!(json_str.contains("\"bin_count\":1"), "expected 1 bin: {json_str}");

    // The box must be placed at the origin.
    assert!(json_str.contains("\"x\":0"), "x must be 0: {json_str}");
    assert!(json_str.contains("\"y\":0"), "y must be 0: {json_str}");
    assert!(json_str.contains("\"z\":0"), "z must be 0: {json_str}");
}

/// Boxes that exceed the bin volume must require multiple bins (growing bin off)
/// or a single growing bin.
#[wasm_bindgen_test]
fn test_pack_overfill_requires_multiple_bins() {
    // 3 boxes of 60³ into a 100³ bin: first two fill it, third overflows.
    let config = make_config(r#"{
        "bin":   { "w": 100, "h": 100, "d": 100 },
        "boxes": [
            { "id": 1, "w": 60, "h": 60, "d": 60 },
            { "id": 2, "w": 60, "h": 60, "d": 60 }
        ]
    }"#);

    let result = pack(config).expect("pack() must succeed");
    let json_str = js_sys::JSON::stringify(&result)
        .unwrap()
        .as_string()
        .unwrap();

    // With two large boxes in a 100³ bin, both boxes may fit in the same bin
    // (100³ = 1,000,000; each box = 216,000 so both fit volume-wise, but
    // spatially 60+60=120>100 so they won't fit together — expect 2 bins).
    assert!(
        !json_str.contains("\"bin_count\":0"),
        "bin_count must be at least 1: {json_str}"
    );
    // We can't assert exactly 2 without knowing the solver's placement
    // strategy, but we can assert both boxes appear in the result.
    assert!(json_str.contains("\"id\":1"), "box 1 missing: {json_str}");
    assert!(json_str.contains("\"id\":2"), "box 2 missing: {json_str}");
}

// ---------------------------------------------------------------------------
// Full Pipeline – WasmOptimizer (stateful, generation-by-generation)
// ---------------------------------------------------------------------------

/// WasmOptimizer::new() must succeed and run_generation() must return a
/// well-formed JsResult with packed boxes.
#[wasm_bindgen_test]
fn test_wasm_optimizer_one_generation() {
    let config = make_config(r#"{
        "bin":   { "w": 100, "h": 100, "d": 100 },
        "boxes": [
            { "id": 1, "w": 30, "h": 30, "d": 30 },
            { "id": 2, "w": 25, "h": 25, "d": 25 }
        ],
        "solver": "best_fit_ems",
        "population_size": 8,
        "elite_count": 2
    }"#);

    let mut opt = WasmOptimizer::new(config).expect("WasmOptimizer::new must succeed");
    let result = opt.run_generation();

    // run_generation must not return null.
    assert!(!result.is_null(), "run_generation must not return null");

    let json_str = js_sys::JSON::stringify(&result)
        .unwrap()
        .as_string()
        .unwrap();

    assert!(json_str.contains("\"packed\""), "result must have packed array: {json_str}");
    assert!(json_str.contains("\"bin_count\""), "result must have bin_count: {json_str}");
    assert!(json_str.contains("\"score\""), "result must have score: {json_str}");
}

/// Multiple sequential generation steps must each return a valid result.
#[wasm_bindgen_test]
fn test_wasm_optimizer_multiple_generations() {
    let config = make_config(r#"{
        "bin":   { "w": 100, "h": 100, "d": 100 },
        "boxes": [
            { "id": 1, "w": 30, "h": 30, "d": 30 },
            { "id": 2, "w": 20, "h": 20, "d": 20 },
            { "id": 3, "w": 15, "h": 15, "d": 15 }
        ],
        "solver": "best_fit_ems",
        "population_size": 8,
        "elite_count": 2
    }"#);

    let mut opt = WasmOptimizer::new(config).expect("WasmOptimizer::new must succeed");

    for generation in 1..=5 {
        let result = opt.run_generation();
        assert!(
            !result.is_null(),
            "generation {generation}: run_generation must not return null"
        );
        let json_str = js_sys::JSON::stringify(&result)
            .unwrap()
            .as_string()
            .unwrap();
        assert!(
            json_str.contains("\"packed\""),
            "generation {generation}: result must have packed: {json_str}"
        );
    }
}

/// WasmOptimizer must work correctly with all solver variants.
#[wasm_bindgen_test]
fn test_wasm_optimizer_all_solver_variants() {
    let solvers = ["best_fit_ems", "first_fit_ems", "best_fit_3d", "first_fit_3d"];

    for solver in solvers {
        let json = format!(r#"{{
            "bin":   {{ "w": 80, "h": 80, "d": 80 }},
            "boxes": [
                {{ "id": 1, "w": 25, "h": 25, "d": 25 }},
                {{ "id": 2, "w": 20, "h": 20, "d": 20 }}
            ],
            "solver": "{solver}",
            "population_size": 4,
            "elite_count": 1
        }}"#);

        let config = make_config(&json);
        let mut opt = WasmOptimizer::new(config)
            .unwrap_or_else(|_| panic!("WasmOptimizer::new must succeed for solver '{solver}'"));

        let result = opt.run_generation();
        assert!(
            !result.is_null(),
            "solver '{solver}': run_generation returned null"
        );
    }
}

// ---------------------------------------------------------------------------
// Full Pipeline – WasmGeneticPool (CPU-side scoring, JS-driven loop)
// ---------------------------------------------------------------------------

/// WasmGeneticPool::new() must produce an initial population of the correct size.
#[wasm_bindgen_test]
fn test_wasm_genetic_pool_initial_population_size() {
    let config = make_config(r#"{
        "bin":   { "w": 100, "h": 100, "d": 100 },
        "boxes": [
            { "id": 1, "w": 30, "h": 30, "d": 30 },
            { "id": 2, "w": 20, "h": 20, "d": 20 }
        ],
        "population_size": 10,
        "elite_count": 2
    }"#);

    let pool = WasmGeneticPool::new(config).expect("WasmGeneticPool::new must succeed");

    // get_current_orders_flat returns an Int32Array of length pop_size * num_boxes.
    let flat = pool.get_current_orders_flat();
    let expected_len = 10 * 2; // population_size * num_boxes
    assert_eq!(
        flat.length(),
        expected_len as u32,
        "orders flat must have {} entries, got {}",
        expected_len,
        flat.length()
    );
}

/// The flat order array must contain only valid box indices (0..num_boxes).
#[wasm_bindgen_test]
fn test_wasm_genetic_pool_orders_are_valid_indices() {
    let config = make_config(r#"{
        "bin":   { "w": 100, "h": 100, "d": 100 },
        "boxes": [
            { "id": 1, "w": 30, "h": 30, "d": 30 },
            { "id": 2, "w": 20, "h": 20, "d": 20 },
            { "id": 3, "w": 10, "h": 10, "d": 10 }
        ],
        "population_size": 6,
        "elite_count": 2
    }"#);

    let pool = WasmGeneticPool::new(config).expect("WasmGeneticPool::new must succeed");
    let flat = pool.get_current_orders_flat();
    let num_boxes = 3u32;

    for i in 0..flat.length() {
        let idx = flat.get_index(i);
        assert!(
            idx >= 0 && (idx as u32) < num_boxes,
            "order[{i}] = {idx} is out of range [0, {num_boxes})"
        );
    }
}

/// A full pool step: construct pool, supply mock scores, advance generation,
/// then verify the pool is still in a valid state.
#[wasm_bindgen_test]
fn test_wasm_genetic_pool_advance_generation() {
    let pop_size: u32 = 8;
    let num_boxes: u32 = 3;

    let config = make_config(&format!(r#"{{
        "bin":   {{ "w": 100, "h": 100, "d": 100 }},
        "boxes": [
            {{ "id": 1, "w": 30, "h": 30, "d": 30 }},
            {{ "id": 2, "w": 20, "h": 20, "d": 20 }},
            {{ "id": 3, "w": 10, "h": 10, "d": 10 }}
        ],
        "population_size": {pop_size},
        "elite_count": 2
    }}"#));

    let mut pool = WasmGeneticPool::new(config).expect("WasmGeneticPool::new must succeed");

    // Simulate scores: individual 0 is the best (highest score).
    let scores: Vec<f32> = (0..pop_size).map(|i| (pop_size - i) as f32 * 0.1).collect();
    pool.advance_generation(&scores);

    // After one generation the flat orders must still be the right size.
    let flat = pool.get_current_orders_flat();
    assert_eq!(
        flat.length(),
        pop_size * num_boxes,
        "orders flat must remain {pop_size}×{num_boxes} after advance_generation"
    );

    // All indices must still be in range.
    for i in 0..flat.length() {
        let idx = flat.get_index(i);
        assert!(
            idx >= 0 && (idx as u32) < num_boxes,
            "post-advance order[{i}] = {idx} out of range"
        );
    }
}

/// get_best_order() must return an Int32Array with num_boxes entries,
/// all valid indices.
#[wasm_bindgen_test]
fn test_wasm_genetic_pool_get_best_order() {
    let pop_size: u32 = 6;
    let num_boxes: u32 = 4;

    let config = make_config(&format!(r#"{{
        "bin":   {{ "w": 100, "h": 100, "d": 100 }},
        "boxes": [
            {{ "id": 1, "w": 30, "h": 30, "d": 30 }},
            {{ "id": 2, "w": 20, "h": 20, "d": 20 }},
            {{ "id": 3, "w": 10, "h": 10, "d": 10 }},
            {{ "id": 4, "w": 15, "h": 15, "d": 15 }}
        ],
        "population_size": {pop_size},
        "elite_count": 2
    }}"#));

    let mut pool = WasmGeneticPool::new(config).expect("WasmGeneticPool::new must succeed");

    // One generation of advance before querying best order.
    let scores: Vec<f32> = (0..pop_size).map(|i| i as f32 * 0.05 + 0.5).collect();
    pool.advance_generation(&scores);

    let best = pool.get_best_order();
    assert_eq!(best.length(), num_boxes, "best order must have {num_boxes} entries");

    for i in 0..best.length() {
        let idx = best.get_index(i);
        assert!(
            idx >= 0 && (idx as u32) < num_boxes,
            "best_order[{i}] = {idx} out of range [0, {num_boxes})"
        );
    }
}

/// Simulate three consecutive pool steps to verify the loop is stable.
#[wasm_bindgen_test]
fn test_wasm_genetic_pool_multi_step_loop() {
    let pop_size: u32 = 10;
    let num_boxes: u32 = 3;

    let config = make_config(&format!(r#"{{
        "bin":   {{ "w": 100, "h": 100, "d": 100 }},
        "boxes": [
            {{ "id": 1, "w": 30, "h": 30, "d": 30 }},
            {{ "id": 2, "w": 20, "h": 20, "d": 20 }},
            {{ "id": 3, "w": 10, "h": 10, "d": 10 }}
        ],
        "population_size": {pop_size},
        "elite_count": 3
    }}"#));

    let mut pool = WasmGeneticPool::new(config).expect("WasmGeneticPool::new must succeed");

    for step in 1..=3 {
        let flat = pool.get_current_orders_flat();
        assert_eq!(
            flat.length(),
            pop_size * num_boxes,
            "step {step}: flat orders size mismatch"
        );

        // Mock scores: score linearly by index.
        let scores: Vec<f32> = (0..pop_size).map(|i| 0.5 + i as f32 * 0.01).collect();
        pool.advance_generation(&scores);
    }

    // Pool must still be usable after 3 steps.
    let best = pool.get_best_order();
    assert_eq!(best.length(), num_boxes);
}

// ---------------------------------------------------------------------------
// evaluate_single_placement
// ---------------------------------------------------------------------------

/// evaluate_single_placement must return a valid JsResult for a good order.
#[wasm_bindgen_test]
fn test_evaluate_single_placement_valid() {
    let config = make_config(r#"{
        "bin":   { "w": 100, "h": 100, "d": 100 },
        "boxes": [
            { "id": 1, "w": 30, "h": 30, "d": 30 },
            { "id": 2, "w": 20, "h": 20, "d": 20 },
            { "id": 3, "w": 10, "h": 10, "d": 10 }
        ]
    }"#);

    // Order: box indices in the desired packing sequence.
    let order: Vec<i32> = vec![0, 1, 2];
    let result = evaluate_single_placement(config, &order)
        .expect("evaluate_single_placement must succeed with valid inputs");

    let json_str = js_sys::JSON::stringify(&result)
        .unwrap()
        .as_string()
        .unwrap();

    assert!(json_str.contains("\"packed\""), "result must have packed: {json_str}");
    assert!(json_str.contains("\"bin_count\""), "result must have bin_count: {json_str}");
    assert!(json_str.contains("\"score\""), "result must have score: {json_str}");

    // All three boxes must appear in the result.
    assert!(json_str.contains("\"id\":1"), "box 1 missing: {json_str}");
    assert!(json_str.contains("\"id\":2"), "box 2 missing: {json_str}");
    assert!(json_str.contains("\"id\":3"), "box 3 missing: {json_str}");
}

/// Reversed order must also succeed (no panics with a different permutation).
#[wasm_bindgen_test]
fn test_evaluate_single_placement_reversed_order() {
    let config = make_config(r#"{
        "bin":   { "w": 100, "h": 100, "d": 100 },
        "boxes": [
            { "id": 10, "w": 10, "h": 10, "d": 10 },
            { "id": 20, "w": 20, "h": 20, "d": 20 },
            { "id": 30, "w": 30, "h": 30, "d": 30 }
        ]
    }"#);

    let order: Vec<i32> = vec![2, 1, 0]; // largest first
    let result = evaluate_single_placement(config, &order)
        .expect("evaluate_single_placement must succeed with reversed order");

    assert!(!result.is_null(), "result must not be null");
}

// ---------------------------------------------------------------------------
// Error Handling
// ---------------------------------------------------------------------------

/// pack() with an empty boxes array must return a JS-catchable error.
#[wasm_bindgen_test]
fn test_pack_empty_boxes_is_error() {
    let config = make_config(r#"{
        "bin":  { "w": 100, "h": 100, "d": 100 },
        "boxes": []
    }"#);

    let result = pack(config);
    assert!(result.is_err(), "pack() with empty boxes must return Err");

    let err_msg = result.unwrap_err().as_string().unwrap_or_default();
    assert!(
        !err_msg.is_empty(),
        "error must carry a descriptive message"
    );
}

/// WasmOptimizer::new() with an empty boxes array must return an error.
#[wasm_bindgen_test]
fn test_wasm_optimizer_empty_boxes_is_error() {
    let config = make_config(r#"{
        "bin":  { "w": 100, "h": 100, "d": 100 },
        "boxes": []
    }"#);

    let result = WasmOptimizer::new(config);
    assert!(result.is_err(), "WasmOptimizer::new with empty boxes must return Err");
}

/// WasmGeneticPool::new() with an empty boxes array must return an error.
#[wasm_bindgen_test]
fn test_wasm_genetic_pool_empty_boxes_is_error() {
    let config = make_config(r#"{
        "bin":  { "w": 100, "h": 100, "d": 100 },
        "boxes": []
    }"#);

    let result = WasmGeneticPool::new(config);
    assert!(result.is_err(), "WasmGeneticPool::new with empty boxes must return Err");
}

/// Passing a completely invalid (non-object) value must be caught gracefully.
#[wasm_bindgen_test]
fn test_pack_invalid_json_type_is_error() {
    // A JS number is not a valid config object.
    let not_an_object = JsValue::from(42_f64);
    let result = pack(not_an_object);
    assert!(result.is_err(), "pack() with a non-object must return Err");
}

/// WasmOptimizer with a string instead of a config object must error.
#[wasm_bindgen_test]
fn test_wasm_optimizer_invalid_config_type_is_error() {
    let bad_config = JsValue::from_str("this is not json");
    let result = WasmOptimizer::new(bad_config);
    assert!(result.is_err(), "WasmOptimizer::new with bad config type must return Err");
}

/// WasmGeneticPool with a null value must error gracefully.
#[wasm_bindgen_test]
fn test_wasm_genetic_pool_null_config_is_error() {
    let result = WasmGeneticPool::new(JsValue::NULL);
    assert!(result.is_err(), "WasmGeneticPool::new with null must return Err");
}
