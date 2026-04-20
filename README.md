# Rustport

A bin-packing library ported to Rust with WebAssembly support.

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
This generates a `pkg/` directory containing the Wasm binary and JS glue code.

## Usage
The JavaScript interface is defined in `src/wasm_api.rs`. It provides a stateful `WasmOptimizer` for genetic-algorithm based bin packing and a one-shot `pack` function.
