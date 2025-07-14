use config::Config;
use once_cell::sync::Lazy;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct RawConfig {
    pub rows: usize,
    pub cols: usize,
    pub gap_ratio: f32,
    pub window_size: Option<(usize, usize)>,
}

pub struct AppConfig {
    pub rows: usize,
    pub cols: usize,
    pub gap_ratio: f32,
    pub cell_size: f32,
    pub window_size: Option<(usize, usize)>,
}

impl From<RawConfig> for AppConfig {
    fn from(value: RawConfig) -> Self {
        // calculate the cell_size
        let num_to_fit = value.rows.max(value.cols) as f32;
        // 2 = (cell_size * num_to_fit) + ((num_to_fit + 1) * cell_size * gap_ratio)
        // cell_size  = (num_to_fit + (num_to_fit+1)*gap_ratio) / 2

        let cell_size = (num_to_fit + (num_to_fit + 1.0) * value.gap_ratio) / 2.0;
        Self {
            rows: value.rows,
            cols: value.cols,
            gap_ratio: value.gap_ratio,
            cell_size,
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
    raw_config.into()
}
pub static CONFIG: Lazy<AppConfig> = Lazy::new(|| load_config());
