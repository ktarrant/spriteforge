use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const DEFAULT_OUT_DIR: &str = "out/tilesheet";
pub const TILESET_CONFIG_DIR: &str = "configs/tilesheet";

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ConfigFile {
    Tile(TileConfig),
    Tilesheet(TilesheetConfig),
}

#[derive(Debug, Deserialize, Default)]
pub struct TileConfig {
    pub name: String,
    pub size: Option<u32>,
    pub bg: Option<String>,
    pub seed: Option<u64>,
    pub blade_min: Option<i32>,
    pub blade_max: Option<i32>,
    pub grass_base: Option<String>,
    pub grass_shades: Option<[String; 3]>,
    pub water_base: Option<String>,
    pub water_edge_cutoff: Option<f32>,
    pub dirt_base: Option<String>,
    pub dirt_splotches: Option<[String; 2]>,
    pub dirt_stones: Option<[String; 2]>,
    pub dirt_splotch_count: Option<u32>,
    pub dirt_stone_count: Option<u32>,
    pub transition_angle: Option<f32>,
    pub transition_angles: Option<Vec<f32>>,
    pub transition_density: Option<f32>,
    pub transition_bias: Option<f32>,
    pub transition_falloff: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct TilesheetConfig {
    pub tile_config: PathBuf,
    #[serde(default)]
    pub seeds: Vec<u64>,
    pub variants: Option<Vec<TilesheetVariant>>,
    pub columns: Option<u32>,
    pub padding: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct TilesheetVariant {
    pub seed: u64,
    pub angle: Option<f32>,
    pub angles: Option<Vec<f32>>,
    pub density: Option<f32>,
    pub bias: Option<f32>,
    pub falloff: Option<f32>,
    pub water_edge_cutoff: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct TilesheetEntry {
    pub seed: u64,
    pub angles: Option<Vec<f32>>,
    pub overrides: TransitionOverrides,
}

#[derive(Debug, Clone, Default)]
pub struct TransitionOverrides {
    pub density: Option<f32>,
    pub bias: Option<f32>,
    pub falloff: Option<f32>,
    pub water_edge_cutoff: Option<f32>,
}

pub fn load_config(path: &Path) -> Result<ConfigFile, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

pub fn load_tile_config(path: &Path) -> Result<TileConfig, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let config: ConfigFile = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    match config {
        ConfigFile::Tile(tile) => Ok(tile),
        ConfigFile::Tilesheet(_) => Err("Tile config cannot be a tilesheet".to_string()),
    }
}

pub fn tilesheet_entries(sheet: &TilesheetConfig) -> Result<Vec<TilesheetEntry>, String> {
    if let Some(variants) = &sheet.variants {
        return Ok(variants
            .iter()
            .map(|variant| TilesheetEntry {
                seed: variant.seed,
                angles: variant
                    .angles
                    .clone()
                    .or_else(|| variant.angle.map(|angle| vec![angle])),
                overrides: TransitionOverrides {
                    density: variant.density,
                    bias: variant.bias,
                    falloff: variant.falloff,
                    water_edge_cutoff: variant.water_edge_cutoff,
                },
            })
            .collect());
    }
    Ok(sheet
        .seeds
        .iter()
        .map(|seed| TilesheetEntry {
            seed: *seed,
            angles: None,
            overrides: TransitionOverrides::default(),
        })
        .collect())
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

pub fn resolve_path(base: &Path, rel: &Path) -> PathBuf {
    if rel.is_absolute() {
        rel.to_path_buf()
    } else {
        base.parent().unwrap_or_else(|| Path::new(".")).join(rel)
    }
}
