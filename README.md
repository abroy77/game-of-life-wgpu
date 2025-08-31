# Conway's Game of Life - Rust + WebGPU

A high-performance implementation of Conway's Game of Life. The simulation and rendering runs entirely on your GPU.
No graphics libraries were used in this app. It's all running in WASM locally on your browser in the static web-app.

## [Live Demo: Click Here!](https://abroy77.github.io/game-of-life-wgpu/)

The Game of Life follows simple rules:

1. **Survival**: Live cell with 2-3 neighbors survives
2. **Birth**: Dead cell with exactly 3 neighbors becomes alive  
3. **Death**: All other cells die or stay dead

## Features

- **GPU-Accelerated**: Uses WebGPU compute shaders for cellular automaton updates
- **Cross-Platform**: Runs natively on desktop and in web browsers via WebAssembly
- **High Performance**: Ping-pong buffer system for efficient GPU memory usage
- **Instanced Rendering**: Efficient GPU rendering of thousands of cells
## Inspiration
There are hundreds of GoL projects on the web and most of them look way better than mine.
These two are notable examples which I took inspiration from:
1. [Jovianmoon's implementation](https://life.jovianmoon.io/)
2. [Psychedelicio](https://lifelike.psychedelicio.us/)

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) (for WASM builds)

### Native Build

```bash
cargo run --release
```

### WASM Build

```bash
./wasm_build.sh
```

Then serve the directory with any HTTP server:

```bash
# Python
python -m http.server 8000

# Node.js
npx serve .

# Or any other static file server
```

### Conway's Rules


### Implementation Details

- **Compute Shaders**: Game logic runs on GPU using WGSL compute shaders
- **Ping-Pong Buffers**: Two storage buffers alternate between read/write for each frame
- **Instanced Rendering**: Each cell is rendered as an instance with position and state
- **WebGPU Pipeline**: Separate compute and render passes for optimal performance


## Deployment

This project includes GitHub Actions for automatic deployment to GitHub Pages:

1. **Push to main branch** triggers the build
2. **Rust + wasm-pack** builds the WASM module
3. **GitHub Pages** serves the static files
