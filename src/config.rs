#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
#[cfg(target_arch = "wasm32")]
use web_time::Duration;

use config::Config;
use serde::Deserialize;

const COMPUTE_WORKGROUP_SIZE: [usize; 2] = [16, 16];

#[derive(Deserialize, Debug)]
pub struct RawConfig {
    pub rows: usize,
    pub cols: usize,
    pub gap_ratio: f32,
    pub fps: usize,
    pub paint_fps: usize,
    pub init_rand_threshold: f64,
    pub window_size: Option<(usize, usize)>,
    pub background_color: [u8; 4],
    pub cursor_color: [u8; 4],
}

#[derive(Debug)]
pub struct AppConfig {
    pub rows: usize,
    pub cols: usize,
    pub cell_size: f32,
    pub gap_size: f32,
    pub fps: usize,
    pub paint_fps: usize,
    pub init_rand_threshold: f64,
    pub frame_duration: Duration,
    pub paint_frame_duration: Duration,
    pub window_size: Option<(usize, usize)>,
    pub compute_dispatches: [usize; 2],
    pub is_paused: bool,
    pub background_color: [u8; 4],
    pub cursor_color: [u8; 4],
}

impl From<RawConfig> for AppConfig {
    fn from(value: RawConfig) -> Self {
        // calculate the cell_size
        let num_to_fit = value.rows.max(value.cols) as f32;
        // 2 = (cell_size * num_to_fit) + ((num_to_fit + 1) * cell_size * gap_ratio)
        // cell_size  = (num_to_fit + (num_to_fit+1)*gap_ratio) / 2

        let cell_size = 2.0 / (num_to_fit + (num_to_fit + 1.0) * value.gap_ratio);
        let gap_size = value.gap_ratio * cell_size;
        let frame_duration = Duration::from_nanos(1_000_000_000 / value.fps as u64);
        let paint_frame_duration = Duration::from_nanos(1_000_000_000 / value.paint_fps as u64);
        let compute_dispatches = [
            (value.cols / COMPUTE_WORKGROUP_SIZE[0]) + 1,
            (value.rows / COMPUTE_WORKGROUP_SIZE[1]) + 1,
        ];
        dbg!(&compute_dispatches);
        Self {
            rows: value.rows,
            cols: value.cols,
            cell_size,
            fps: value.fps,
            paint_fps: value.paint_fps,
            init_rand_threshold: value.init_rand_threshold,
            frame_duration,
            paint_frame_duration,
            gap_size,
            compute_dispatches,
            window_size: value.window_size,
            is_paused: false,
            background_color: value.background_color,
            cursor_color: value.cursor_color,
        }
    }
}
impl AppConfig {
    pub fn num_elements(&self) -> usize {
        self.rows * self.cols
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_config() -> AppConfig {
    let raw_config: RawConfig = Config::builder()
        .add_source(config::File::with_name("appconfig"))
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();
    println!("Raw Config:\n{:?}", &raw_config);
    let app_config = raw_config.into();
    println!("App Config:\n{:?}", &app_config);
    app_config
}

/// build at compile time using include_str!
#[cfg(target_arch = "wasm32")]
pub fn load_config() -> AppConfig {
    use config::FileFormat;
    let config_str: &str = include_str!("../appconfig.toml");

    let raw_config: RawConfig = Config::builder()
        .add_source(config::File::from_str(config_str, FileFormat::Toml))
        .build()
        .unwrap()
        .try_deserialize()
        .unwrap();
    println!("Raw Config:\n{:?}", &raw_config);
    let app_config = raw_config.into();
    println!("App Config:\n{:?}", &app_config);
    app_config
}
