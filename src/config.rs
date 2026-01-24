use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const DEFAULT_OUT_DIR: &str = "out/tilesheet";
pub const TILESET_CONFIG_DIR: &str = "configs/tile";

#[derive(Debug, Deserialize, Default)]
pub struct TileConfig {
    pub name: String,
    pub size: Option<u32>,
    pub bg: Option<String>,
    pub seed: Option<u64>,
    pub tilesheet_seed_start: Option<u64>,
    pub tilesheet_count: Option<u32>,
    pub tilesheet_columns: Option<u32>,
    pub tilesheet_padding: Option<u32>,
    pub blade_min: Option<i32>,
    pub blade_max: Option<i32>,
    pub grass_base: Option<String>,
    pub grass_shades: Option<[String; 3]>,
    pub grass_edge_cutoff: Option<f32>,
    pub grass_edge_gradient: Option<f32>,
    pub water_base: Option<String>,
    pub water_edge_cutoff: Option<f32>,
    pub path_base: Option<String>,
    pub path_brick_count: Option<u32>,
    pub path_brick_crack: Option<f32>,
    pub dirt_base: Option<String>,
    pub dirt_splotches: Option<[String; 2]>,
    pub dirt_stones: Option<[String; 2]>,
    pub dirt_splotch_count: Option<u32>,
    pub dirt_stone_count: Option<u32>,
    pub transition_density: Option<f32>,
    pub transition_bias: Option<f32>,
    pub transition_falloff: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct TilesheetEntry {
    pub seed: u64,
    pub overrides: TransitionOverrides,
    pub transition_mask: Option<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct TransitionOverrides {
    pub density: Option<f32>,
    pub bias: Option<f32>,
    pub falloff: Option<f32>,
    pub water_edge_cutoff: Option<f32>,
    pub grass_edge_cutoff: Option<f32>,
    pub grass_edge_gradient: Option<f32>,
}

pub fn load_tile_config(path: &Path) -> Result<TileConfig, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let config: TileConfig = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    Ok(config)
}

pub fn output_path_for_config(
    config_path: &Path,
    out_override: Option<&PathBuf>,
    default_out_dir: &str,
) -> PathBuf {
    if let Some(out) = out_override {
        return out.clone();
    }
    let stem = config_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    Path::new(default_out_dir).join(format!("{stem}.png"))
}
