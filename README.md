# pbrtrs

pbrtrs is a path tracer written in Rust. It is based on the book
[Physically Based Rendering: From Theory To Implementation](https://www.pbr-book.org)
by Matt Pharr, Wenzel Jakob, and Greg Humphreys

## Examples

![Example rendered output](./out.png)

## Building

Built using cargo

### Building without image denoise:
```bash
cargo build --release
```

### Building with image denoise:
```bash
export OIDN_DIR="[path to oidn]"
cargo build --release --features enable_oidn
```

### Optional Features
 - `enable_oidn` - Enable image denoise using [Intel's Open Image Denoise library](https://www.openimagedenoise.org)
 - `enable_axis` - Draw coordinate axis in the top left tile of the image (for debugging)
 - `enable_debugger` - Enable the debugger which outputs debug information about a pixel to `debug_out.txt`.  Set `DEBUG_PIXEL` in `main.rs` to the pixel coordinates you want to debug.

## Running

```bash
export TEV_PATH="[path to tev executable]"
pbrtrs [path to scene.toml]
```