//! App settings: loaded from `appconfig.toml`. Additional values are derived from config values
//!
//! [`RawConfig`] is exactly what's in the TOML file, and [`AppConfig`] is computed
//! from it.

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
// `std::time` doesn't work in the browser, `web_time` is a drop-in replacement.
#[cfg(target_arch = "wasm32")]
use web_time::Duration;

use config::Config;
use serde::Deserialize;
use std::sync::Arc;
use winit::window::Window;

/// How many threads are in one compute workgroup, in x and y.
///
/// The GPU runs compute shaders in groups of threads called **workgroups**.
/// This must match `@workgroup_size(16, 16, 1)` in `compute.wgsl` and `paint.wgsl`:
/// each workgroup handles a 16x16 tile of cells, one thread per cell.
const COMPUTE_WORKGROUP_SIZE: [usize; 2] = [16, 16];

/// Span in **Normalized Device Coordinates (NDC)** of the canvas.
const CLIP_WIDTH: f32 = 2.0;

/// Settings exactly as written in `appconfig.toml`.
#[derive(Deserialize, Debug)]
pub struct RawConfig {
    /// Grid rows. On startup this is recalculated from the window shape,
    /// see [`AppConfig::update_cell_configuration`].
    pub rows: usize,
    /// Grid columns. Recalculated from the window shape like `rows`.
    pub cols: usize,
    /// shorter of rows and columns
    pub min_cell_array_len: usize,
    /// Gap between cells as a fraction of a cell's size (0.15 = gap is 15% of a cell).
    pub gap_ratio: f32,
    /// Simulation steps per second.
    pub fps: usize,
    /// How many times per second painted cells are pushed to the GPU.
    pub paint_fps: usize,
    /// Probability (0..1) that a cell starts alive when shuffling.
    pub init_rand_threshold: f64,
    /// Background RGBA colour, 0-255 per channel.
    pub background_color: [u8; 4],
    /// Cursor RGBA colour on desktop, 0-255 per channel.
    pub cursor_color: [u8; 4],
    /// desktop window default size
    pub desktop_size: [u32; 2],
}

/// The settings the app actually uses: [`RawConfig`] values plus numbers derived from them.
///
/// Sizes here are in **clip space** / **Normalized Device Coordinates (NDC)**:
/// The visible area always goes from -1 to 1 on the x and y axes.
/// x increases from  left to right and y increases upward.
/// x and y are 2.0 units wide each.
#[derive(Debug)]
pub struct AppConfig {
    pub rows: usize,
    pub cols: usize,
    pub min_cell_array_len: usize,
    /// Width and height of one cell in clip space.
    pub cell_size: (f32, f32),
    pub gap_ratio: f32,
    /// Horizontal and vertical gap between cells in clip space.
    pub gap_size: (f32, f32),
    pub fps: usize,
    pub paint_fps: usize,
    pub init_rand_threshold: f64,
    /// Time between simulation steps (1 / `fps`).
    pub frame_duration: Duration,
    /// Time between paint uploads (1 / `paint_fps`).
    pub paint_frame_duration: Duration,
    /// How many workgroups to launch in x and y so that every cell gets a GPU thread.
    pub compute_dispatches: [usize; 2],
    /// Is the simulation paused?.
    pub is_paused: bool,
    /// Background colour: f64 channels in 0..1.
    pub background_color: wgpu::Color,
    pub cursor_color: [u8; 4],
    pub desktop_size: [u32; 2],
}

impl From<RawConfig> for AppConfig {
    fn from(value: RawConfig) -> Self {
        // Cell size: fit the bigger dimension into the 2.0-wide clip space.
        // n cells and (n + 1) gaps  must sum to CLIP_WIDTH:
        // n * cell + (n + 1) * cell * gap_ratio = 2  =>  cell = 2 / (n + (n + 1) * gap_ratio)
        let num_cells_to_fit = value.rows.max(value.cols) as f32;
        let num_gaps_to_fit = num_cells_to_fit + 1.0;
        let cell_size = CLIP_WIDTH / (num_cells_to_fit + (num_gaps_to_fit) * value.gap_ratio);
        // Convert fps into frame duration.
        let frame_duration = Duration::from_nanos(1_000_000_000 / value.fps as u64);
        let paint_frame_duration = Duration::from_nanos(1_000_000_000 / value.paint_fps as u64);
        // Workgroups needed to cover the grid. Integer division rounds down, so we add 1 to
        // make sure the edge is covered. Any extra threads past the grid edge exit early
        // (see `pos_in_grid` in the shaders).
        let compute_dispatches = [
            (value.cols / COMPUTE_WORKGROUP_SIZE[0]) + 1,
            (value.rows / COMPUTE_WORKGROUP_SIZE[1]) + 1,
        ];
        // wgpu colours are 0.0..=1.0 floats, the config uses 0..=255 bytes.
        let background_color = wgpu::Color {
            r: value.background_color[0] as f64 / 255.0,
            g: value.background_color[1] as f64 / 255.0,
            b: value.background_color[2] as f64 / 255.0,
            a: value.background_color[3] as f64 / 255.0,
        };
        let gap_size = (cell_size * value.gap_ratio, cell_size * value.gap_ratio);
        // start as true so that the user can paint before starting the simulation
        let is_paused = true;
        Self {
            rows: value.rows,
            cols: value.cols,
            cell_size: (cell_size, cell_size),
            fps: value.fps,
            paint_fps: value.paint_fps,
            init_rand_threshold: value.init_rand_threshold,
            frame_duration,
            paint_frame_duration,
            gap_size,
            gap_ratio: value.gap_ratio,
            compute_dispatches,
            is_paused,
            background_color,
            cursor_color: value.cursor_color,
            min_cell_array_len: value.min_cell_array_len,
            desktop_size: value.desktop_size,
        }
    }
}
impl AppConfig {
    /// Total number of cells, i.e. the length of every grid buffer.
    pub fn num_elements(&self) -> usize {
        self.rows * self.cols
    }
}

/// Load the config from `appconfig.toml` in the current directory (desktop).
#[cfg(not(target_arch = "wasm32"))]
pub fn load_config() -> AppConfig {
    let raw_config: RawConfig = Config::builder()
        .add_source(config::File::with_name("appconfig"))
        .build()
        .expect("Config object should build successfully")
        .try_deserialize()
        .expect("RawConfig object should build successfully");
    log::info!("Raw Config:\n{:?}", raw_config);
    let app_config: AppConfig = raw_config.into();
    log::info!("App Config:\n{:?}", app_config);
    app_config
}

/// Load the config on the web.
///
/// A browser page can't read files from disk, so `include_str!` copies `appconfig.toml`
/// into the wasm binary at compile time.
#[cfg(target_arch = "wasm32")]
pub fn load_config() -> AppConfig {
    use config::FileFormat;
    let config_str: &str = include_str!("../appconfig.toml");

    let raw_config: RawConfig = Config::builder()
        .add_source(config::File::from_str(config_str, FileFormat::Toml))
        .build()
        .expect("Config object should build successfully")
        .try_deserialize()
        .expect("RawConfig object should build successfully");
    log::info!("Raw Config:\n{:?}", &raw_config);
    let app_config: AppConfig = raw_config.into();
    log::info!("App Config:\n{:?}", &app_config);
    app_config
}

impl AppConfig {
    /// Pick rows/cols to fit the window's shape, then recompute the sizes that depend on them.
    ///
    /// The shorter side gets `min_cell_array_len` cells, and the longer side gets
    /// proportionally more, so cells stay roughly square.
    /// Called once the window exists, before any GPU buffers are created.
    pub fn update_cell_configuration(&mut self, window: &Arc<Window>) {
        // Logical size = physical pixels / display scale (e.g. 2x on Retina screens).
        let window_size = window.inner_size().to_logical::<f64>(window.scale_factor());

        // Shorter side gets `min_cell_array_len` cells, longer side scales up to match.
        match (window_size.width, window_size.height) {
            (w, h) if w < h => {
                self.cols = self.min_cell_array_len;
                self.rows = (self.min_cell_array_len as f64 * h / w).floor() as usize;
            }
            (w, h) if w > h => {
                self.rows = self.min_cell_array_len;
                self.cols = (self.min_cell_array_len as f64 * w / h).floor() as usize;
            }
            (_, _) => {}
        };
        // Cell size per axis. Done separately for x and y because the
        // clip-width is 2.0 in both axes. Same formula as `From<RawConfig>`
        self.cell_size = (
            CLIP_WIDTH / (self.cols as f32 + (self.cols as f32 + 1.0) * self.gap_ratio),
            CLIP_WIDTH / (self.rows as f32 + (self.rows as f32 + 1.0) * self.gap_ratio),
        );
        self.gap_size = (
            self.cell_size.0 * self.gap_ratio,
            self.cell_size.1 * self.gap_ratio,
        );
        self.compute_dispatches = [
            (self.cols / COMPUTE_WORKGROUP_SIZE[0]) + 1,
            (self.rows / COMPUTE_WORKGROUP_SIZE[1]) + 1,
        ];
        log::info!(
            "Window size at row col update time: {}x{}",
            window_size.width,
            window_size.height
        );
        log::info!("Set Rows {}, Cols: {}", self.rows, self.cols);
        log::info!("Set cell size: {}x{}", self.cell_size.0, self.cell_size.1);
    }
}
