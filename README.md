# 3D Binpacker (Rust)

A high-performance 3D bin-packing library ported to Rust, supporting both native and WebAssembly targets optional GPU acceleration.

## Related Projects
* **Web Demo**: A full interactive web application utilizing this library can be found at [https://github.com/MilanFIN/3d-binpacker-webdemo](https://github.com/MilanFIN/3d-binpacker-webdemo).
* **Reference Implementation**: The reference Java implementation of this optimizer is available at [https://github.com/MilanFIN/gpu-binpacker](https://github.com/MilanFIN/gpu-binpacker).

## Compute Modes

The library is designed to evaluate large populations of packing permutations via multiple backends:

### Native Support
* **CPU Computing**: Uses native multithreading to evaluate populations in parallel across available CPU cores.
* **GPU Computing**: Uses **OpenCL** to offload algorithm evaluation to native graphics hardware.

### Web Support
* **CPU Computing**: Uses **WebAssembly (Wasm)** to run the genetic algorithm on the browser's processor.
* **GPU Computing**: Uses **WebGPU** to bring parallel GPU execution into the browser

## Building

### Regular Target (Native)
To build the library for your local machine:
```bash
cargo build --release
```

### WebAssembly
To build for web browsers or JS bundlers:
```bash
wasm-pack build --target web
```
*(Alternatively: `cargo build --target wasm32-unknown-unknown`)*

This generates a `pkg/` directory containing the Wasm binary and JS glue code.

## Usage & Data Formats

The JavaScript interface is defined in `src/wasm_api.rs`. It provides a stateful `WasmOptimizer` for genetic-algorithm based bin packing and a one-shot `pack` function.

### Input Format (CSV)
When importing box data (as often used in the demo or testing), each line should represent a box in the following format:
`width, height, depth, [weight]`

*The weight parameter is optional. Lines starting with `#` are ignored.*

### Output Format (CSV)
Exported solutions follow this format:
`Bin, Box, x, y, z, w, h, d`

*Representing each packed box's bin assignment, ID, position (x, y, z), and dimensions (w, h, d).*
