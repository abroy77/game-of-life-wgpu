# Conway's Game of Life - Rust + WebGPU

Conway's Game of Life is a high-performance implementation using Rust and WebGPU. The simulation and rendering runs entirely on your GPU, with support for both native desktop applications and web browsers via WebAssembly.

**Always reference these instructions first and fallback to search or bash commands only when you encounter unexpected information that does not match the info here.**

## Working Effectively

### Essential Setup and Build Process

**CRITICAL BUILD TIMING**: All build operations take significant time. NEVER CANCEL builds or long-running commands.

#### Prerequisites and Initial Setup
- Rust toolchain (latest stable) - already installed
- wasm-pack for WebAssembly builds 
- wasm-bindgen-cli (version 0.2.100) - requires special installation (see below)

#### Core Build Commands

**Native Build:**
```bash
cargo build --release
```
- **Takes 2-3 minutes - NEVER CANCEL. Set timeout to 300+ seconds.**
- Use for native desktop development and testing

**WASM Build (Method 1 - Manual, Recommended):**
```bash
# Step 1: Build WASM binary (45-60 seconds)
RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo build --target wasm32-unknown-unknown --release --lib

# Step 2: Download and install wasm-bindgen-cli (one-time setup)
cd /tmp
wget https://github.com/rustwasm/wasm-bindgen/releases/download/0.2.100/wasm-bindgen-0.2.100-x86_64-unknown-linux-musl.tar.gz
tar -xzf wasm-bindgen-0.2.100-x86_64-unknown-linux-musl.tar.gz
sudo cp wasm-bindgen-0.2.100-x86_64-unknown-linux-musl/wasm-bindgen /usr/local/bin/

# Step 3: Generate JavaScript bindings
cd /path/to/project
/usr/local/bin/wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/game_of_life.wasm
```

**WASM Build (Method 2 - Script, Has Known Issues):**
```bash
./wasm_build.sh
```
- **Note**: This script may fail due to wasm-bindgen-cli installation issues with getrandom backend
- **Takes 1-2 minutes when it works - NEVER CANCEL. Set timeout to 300+ seconds.**
- If it fails, use Method 1 above

#### Code Quality Checks
```bash
# Format check - fast (5-10 seconds)
cargo fmt --check

# Linting - takes 30-45 seconds, NEVER CANCEL
cargo clippy -- -D warnings
```

#### Running Tests
```bash
cargo test
```
- **Takes 60+ seconds - NEVER CANCEL. Set timeout to 180+ seconds.**
- Currently no unit tests defined, but framework validates successfully

### Running the Application

#### Web Application (Recommended for Testing)
```bash
# Serve the application (requires WASM build first)
python3 -m http.server 8080

# Access at: http://localhost:8080
# WebGPU must be enabled in browser
```

#### Native Application
```bash
cargo run --release
```
- **Takes 2-3 minutes to compile - NEVER CANCEL. Set timeout to 300+ seconds.**
- Requires GUI environment to run (not available in headless environments)

## Validation

### Manual Testing Scenarios
After making any changes, ALWAYS validate with these scenarios:

**Web Application Testing:**
1. Build WASM version using Method 1 above
2. Start HTTP server: `python3 -m http.server 8080`
3. Open browser to `http://localhost:8080`
4. Verify application loads without console errors
5. Test controls: Play/Pause, Step Forward, Shuffle, Reset
6. Test FPS slider functionality
7. Test mouse painting on the grid
8. Verify simulation runs smoothly

**Code Quality Validation:**
```bash
# ALWAYS run these before committing changes
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

### Known Working Validation Commands
These commands have been tested and work correctly:

```bash
# Build validation (timing measured)
time cargo build --release  # ~2 minutes
time cargo test              # ~60 seconds
time cargo clippy -- -D warnings  # ~30-45 seconds

# WASM validation
RUSTFLAGS='--cfg getrandom_backend="wasm_js"' cargo build --target wasm32-unknown-unknown --release --lib

# Server validation
python3 -m http.server 8080 &
curl -I http://localhost:8080
curl -I http://localhost:8080/pkg/game_of_life.js
kill %1  # Stop background server
```

## Common Tasks and Locations

### Key Files and Directories
```
├── src/                    # Rust source code
│   ├── main.rs            # Native application entry point
│   ├── lib.rs             # Library root
│   ├── app.rs             # Main application logic
│   ├── config.rs          # Configuration management
│   ├── game_data.rs       # Game state and compute logic
│   ├── graphics.rs        # WebGPU graphics setup
│   ├── render_data.rs     # Rendering pipeline
│   ├── paint.rs           # Mouse painting functionality
│   ├── vertex.rs          # Vertex data structures
│   ├── web_controls.rs    # Web interface bindings (WASM only)
│   └── shaders/           # WGSL compute and render shaders
├── index.html             # Web application interface
├── index.js              # JavaScript WASM loader
├── appconfig.toml        # Game configuration (grid size, colors, FPS)
├── wasm_build.sh         # WASM build script (has known issues)
└── pkg/                  # Generated WASM output (after build)
```

### Configuration Changes
Edit `appconfig.toml` for game parameters:
```toml
rows = 100                    # Grid height
cols = 100                   # Grid width
gap_ratio = 0.15             # Spacing between cells
fps = 20                     # Simulation speed
paint_fps = 120              # UI update rate
init_rand_threshold = 0.5    # Initial random fill percentage
background_color = [0,0,0,0] # RGBA background
cursor_color = [255,0,0,255] # RGBA cursor color
```

### Debugging and Development

**Common Issues:**
- **wasm-bindgen version mismatch**: Use Method 1 for WASM builds
- **WebGPU not available**: Ensure modern browser with WebGPU enabled
- **CORS issues**: Always serve via HTTP server, not file:// protocol

**Important Code Areas:**
- Game logic: `src/game_data.rs` and `src/shaders/compute.wgsl`
- Rendering: `src/render_data.rs` and GPU pipeline setup
- Web interface: `src/web_controls.rs` for WASM bindings
- Configuration: `src/config.rs` loads from `appconfig.toml`

### CI/CD Integration
The repository uses GitHub Actions (`.github/workflows/deploy.yml`):
- Runs `cargo fmt --check` and `cargo clippy -- -D warnings`
- Builds WASM version and deploys to GitHub Pages
- **CRITICAL**: Always run linting locally before pushing to avoid CI failures

## Emergency Workarounds

### If wasm-pack fails completely:
Use the manual WASM build process (Method 1) which bypasses wasm-pack entirely.

### If builds are taking too long:
- Check available system resources
- Ensure adequate timeout values (300+ seconds for builds)
- Do NOT cancel builds - they will complete given sufficient time

### If web application doesn't load:
1. Check browser console for WebGPU support
2. Verify all files in pkg/ directory exist
3. Ensure HTTP server is serving from project root
4. Check CORS headers if accessing from different origin

**Remember: This application requires WebGPU support. Test in Chrome, Safari, or Firefox with WebGPU enabled.**