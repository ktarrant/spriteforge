use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

pub const EDGE_N: u8 = 1 << 0;
pub const EDGE_E: u8 = 1 << 1;
pub const EDGE_S: u8 = 1 << 2;
pub const EDGE_W: u8 = 1 << 3;
pub const CORNER_NE: u8 = 1 << 4;
pub const CORNER_SE: u8 = 1 << 5;
pub const CORNER_SW: u8 = 1 << 6;
pub const CORNER_NW: u8 = 1 << 7;

pub const EDGE_MASK: u8 = EDGE_N | EDGE_E | EDGE_S | EDGE_W;
pub const CORNER_MASK: u8 = CORNER_NE | CORNER_SE | CORNER_SW | CORNER_NW;

const EDGE_N_MASK: u8 = 0b10010001;
const EDGE_W_MASK: u8 = 0b11001000;
const EDGE_S_MASK: u8 = 0b01100100;
const EDGE_E_MASK: u8 = 0b00110010;

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TileMetadata {
    pub index: usize,
    pub row: u32,
    pub col: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub seed: u64,
    pub transition_mask: Option<u8>,
}

pub fn load_tilesheet_metadata(path: &Path) -> Result<TilesheetMetadata, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

pub fn normalize_mask(mask: u8) -> u8 {
    let edges = mask & EDGE_MASK;
    if edges.count_ones() >= 2 {
        return edges;
    }

    let mut mask = mask;
    if mask & EDGE_N_MASK == EDGE_N_MASK {
        mask = (mask & !EDGE_N_MASK) | EDGE_N;
    }
    if mask & EDGE_W_MASK == EDGE_W_MASK {
        mask = (mask & !EDGE_W_MASK) | EDGE_W;
    }
    if mask & EDGE_S_MASK == EDGE_S_MASK {
        mask = (mask & !EDGE_S_MASK) | EDGE_S;
    }
    if mask & EDGE_E_MASK == EDGE_E_MASK {
        mask = (mask & !EDGE_E_MASK) | EDGE_E;
    }
    mask
}

pub fn all_transition_masks() -> Vec<u8> {
    let mut masks = BTreeSet::new();
    for raw in 0u8..=u8::MAX {
        masks.insert(normalize_mask(raw));
    }
    masks.into_iter().filter(|mask| *mask != 0).collect()
}

pub fn angles_for_mask(mask: u8) -> Vec<f32> {
    let mask = normalize_mask(mask);
    let mut angles = Vec::new();
    if mask & EDGE_N != 0 {
        angles.push(333.435);
    }
    if mask & EDGE_E != 0 {
        angles.push(26.565);
    }
    if mask & EDGE_S != 0 {
        angles.push(153.435);
    }
    if mask & EDGE_W != 0 {
        angles.push(206.565);
    }
    if mask & CORNER_NE != 0 {
        angles.push(0.0);
    }
    if mask & CORNER_NW != 0 {
        angles.push(270.0);
    }
    if mask & CORNER_SW != 0 {
        angles.push(180.0);
    }
    if mask & CORNER_SE != 0 {
        angles.push(90.0);
    }
    angles
}

pub fn mask_index(mask: u8) -> Option<usize> {
    let normalized = normalize_mask(mask);
    all_transition_masks()
        .iter()
        .position(|&candidate| candidate == normalized)
}

pub fn mask_edges(mask: u8) -> u8 {
    mask & EDGE_MASK
}

pub fn mask_corners(mask: u8) -> u8 {
    mask & CORNER_MASK
}
