use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct TilesheetMetadata {
    pub image: String,
    pub config: String,
    pub tile_size: u32,
    pub columns: u32,
    pub rows: u32,
    pub padding: u32,
    pub tile_count: usize,
    pub tiles: Vec<TileMetadata>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TileMetadata {
    pub index: usize,
    pub row: u32,
    pub col: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub seed: u64,
    pub angles: Vec<f32>,
}

pub fn load_tilesheet_metadata(path: &Path) -> Result<TilesheetMetadata, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}
