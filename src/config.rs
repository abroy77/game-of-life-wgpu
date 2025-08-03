use std::time::Duration;

use config::Config;
use once_cell::sync::Lazy;
use serde::Deserialize;

const COMPUTE_WORKGROUP_SIZE: [usize; 2] = [16, 16];

#[derive(Deserialize, Debug)]
pub struct RawConfig {
    pub rows: usize,
    pub cols: usize,
    pub gap_ratio: f32,
    pub fps: usize,
    pub init_rand_threshold: f64,
    pub window_size: Option<(usize, usize)>,
}

#[derive(Debug)]
pub struct AppConfig {
    pub rows: usize,
    pub cols: usize,
    pub num_elements: usize,
    pub cell_size: f32,
    pub gap_size: f32,
    pub fps: usize,
    pub init_rand_threshold: f64,
    pub frame_duration: Duration,
    pub window_size: Option<(usize, usize)>,
    pub compute_dispatches: [usize; 2],
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
        let compute_dispatches = [
            (value.cols / COMPUTE_WORKGROUP_SIZE[0]) + 1,
            (value.rows / COMPUTE_WORKGROUP_SIZE[1]) + 1,
        ];
        dbg!(&compute_dispatches);
        Self {
            rows: value.rows,
            cols: value.cols,
            num_elements: value.rows * value.cols,
            cell_size,
            fps: value.fps,
            init_rand_threshold: value.init_rand_threshold,
            frame_duration,
            gap_size,
            compute_dispatches,
            window_size: value.window_size,
        }
    }
}

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
pub static CONFIG: Lazy<AppConfig> = Lazy::new(load_config);
