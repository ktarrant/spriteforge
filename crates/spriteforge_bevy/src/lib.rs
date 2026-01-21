use serde::Deserialize;
use std::path::Path;

pub mod minimap;
pub mod selection;
pub use map_generators::map_skeleton::{
    AreaType, MapArea, MapSkeleton, MapSkeletonConfig, PathSegment,
};
pub use minimap::{MiniMapPlugin, MiniMapSettings, MiniMapSource, MiniMapState};
pub use selection::{TileSelectedEvent, TileSelectionPlugin, TileSelectionSettings, TileSelectionState};

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
    pub transition_mask: Option<u8>,
}

const EDGE_N: u8 = 1 << 0;
const EDGE_E: u8 = 1 << 1;
const EDGE_S: u8 = 1 << 2;
const EDGE_W: u8 = 1 << 3;
const CORNER_NE: u8 = 1 << 4;
const CORNER_SE: u8 = 1 << 5;
const CORNER_SW: u8 = 1 << 6;
const CORNER_NW: u8 = 1 << 7;

pub fn load_tilesheet_metadata(path: &Path) -> Result<TilesheetMetadata, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseTile {
    Grass,
    Dirt,
    Water,
}

#[derive(Debug, Clone)]
pub struct RenderTileLayers {
    pub width: u32,
    pub height: u32,
    pub grass: Vec<Option<u32>>,
    pub dirt: Vec<Option<u32>>,
    pub water: Vec<Option<u32>>,
    pub water_transition: Vec<Option<u32>>,
    pub transition: Vec<Option<u32>>,
}

pub mod map_generators;

pub fn build_render_layers<R: rand::Rng>(
    base_tiles: &[BaseTile],
    width: u32,
    height: u32,
    grass_meta: &TilesheetMetadata,
    dirt_meta: &TilesheetMetadata,
    water_meta: &TilesheetMetadata,
    water_transition_meta: &TilesheetMetadata,
    transition_meta: &TilesheetMetadata,
    rng: &mut R,
) -> RenderTileLayers {
    let mut grass = vec![None; base_tiles.len()];
    let mut dirt = vec![None; base_tiles.len()];
    let mut water = vec![None; base_tiles.len()];
    let mut water_transition = vec![None; base_tiles.len()];
    let mut transition = vec![None; base_tiles.len()];

    let transition_lookup = build_transition_lookup(transition_meta);
    let water_transition_lookup = build_transition_lookup(water_transition_meta);

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            match base_tiles[idx] {
                BaseTile::Grass => {
                    let mask = adjacent_non_grass_mask(x, y, width, height, base_tiles);
                    if mask != 0 {
                        let index = pick_transition_index(mask, &transition_lookup, rng)
                            .unwrap_or_else(|| rng.gen_range(0..dirt_meta.tile_count) as u32);
                        transition[idx] = Some(index);
                        let dirt_index = rng.gen_range(0..dirt_meta.tile_count) as u32;
                        dirt[idx] = Some(dirt_index);
                    } else {
                        let index = rng.gen_range(0..grass_meta.tile_count) as u32;
                        grass[idx] = Some(index);
                    }
                }
                BaseTile::Water => {
                    let mask = adjacent_non_water_mask(x, y, width, height, base_tiles);
                    if mask != 0 {
                        let index =
                            pick_transition_index(mask, &water_transition_lookup, rng)
                                .unwrap_or_else(|| {
                                    rng.gen_range(0..water_transition_meta.tile_count) as u32
                                });
                        water_transition[idx] = Some(index);
                    } else {
                        let index = rng.gen_range(0..water_meta.tile_count) as u32;
                        water[idx] = Some(index);
                    }
                }
                BaseTile::Dirt => {
                    let dirt_index = rng.gen_range(0..dirt_meta.tile_count) as u32;
                    dirt[idx] = Some(dirt_index);
                }
            }
            if water_transition[idx].is_some() && dirt[idx].is_none() {
                let dirt_index = rng.gen_range(0..dirt_meta.tile_count) as u32;
                dirt[idx] = Some(dirt_index);
            }
        }
    }

    RenderTileLayers {
        width,
        height,
        grass,
        dirt,
        water,
        water_transition,
        transition,
    }
}

fn adjacent_non_water_mask(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    tiles: &[BaseTile],
) -> u8 {
    adjacent_mask(x, y, width, height, tiles, |tile| tile != BaseTile::Water)
}

fn adjacent_non_grass_mask(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    tiles: &[BaseTile],
) -> u8 {
    adjacent_mask(x, y, width, height, tiles, |tile| tile != BaseTile::Grass)
}

fn adjacent_mask<F>(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    tiles: &[BaseTile],
    mut is_match: F,
) -> u8
where
    F: FnMut(BaseTile) -> bool,
{
    let mut mask = 0u8;

    // Edge-adjacent (diamond edges).
    // Mapping keeps the original angle lookup behavior.
    if y > 0 && is_match(tiles[((y - 1) * width + x) as usize]) {
        mask |= EDGE_W;
    }
    if x > 0 && is_match(tiles[(y * width + (x - 1)) as usize]) {
        mask |= EDGE_S;
    }
    if y + 1 < height && is_match(tiles[((y + 1) * width + x) as usize]) {
        mask |= EDGE_E;
    }
    if x + 1 < width && is_match(tiles[(y * width + (x + 1)) as usize]) {
        mask |= EDGE_N;
    }

    // Point-adjacent (diamond corners). These are diagonal neighbors in grid space.
    // East point (0) -> (x+1, y-1), North point (90) -> (x-1, y-1),
    // West point (180) -> (x-1, y+1), South point (270) -> (x+1, y+1).
    if x + 1 < width && y > 0 && is_match(tiles[((y - 1) * width + (x + 1)) as usize])
    {
        mask |= CORNER_NW;
    }
    if x > 0 && y > 0 && is_match(tiles[((y - 1) * width + (x - 1)) as usize]) {
        mask |= CORNER_SW;
    }
    if x > 0 && y + 1 < height
        && is_match(tiles[((y + 1) * width + (x - 1)) as usize])
    {
        mask |= CORNER_SE;
    }
    if x + 1 < width && y + 1 < height
        && is_match(tiles[((y + 1) * width + (x + 1)) as usize])
    {
        mask |= CORNER_NE;
    }
    normalize_47(mask)
}

fn build_transition_lookup(meta: &TilesheetMetadata) -> std::collections::HashMap<u8, Vec<u32>> {
    let mut map = std::collections::HashMap::new();
    for tile in &meta.tiles {
        let Some(mask) = tile.transition_mask else {
            continue;
        };
        let key = normalize_47(mask);
        map.entry(key).or_insert_with(Vec::new).push(tile.index as u32);
    }
    map
}

fn pick_transition_index<R: rand::Rng>(
    mask: u8,
    lookup: &std::collections::HashMap<u8, Vec<u32>>,
    rng: &mut R,
) -> Option<u32> {
    if lookup.is_empty() {
        return None;
    }
    let mask = normalize_47(mask);
    if let Some(choices) = lookup.get(&mask) {
        if choices.is_empty() {
            return None;
        }
        return Some(choices[rng.gen_range(0..choices.len())]);
    }

    let mut best_matches = 0usize;
    let mut best_choices: Vec<u32> = Vec::new();

    for (key, choices) in lookup {
        if choices.is_empty() {
            continue;
        }
        if (key & mask) != *key {
            continue;
        }
        let match_count = (*key & mask).count_ones() as usize;
        if match_count > best_matches {
            best_matches = match_count;
            best_choices.clear();
            best_choices.extend_from_slice(choices);
        } else if match_count == best_matches {
            best_choices.extend_from_slice(choices);
        }
    }

    if best_choices.is_empty() || best_matches == 0 {
        return None;
    }
    Some(best_choices[rng.gen_range(0..best_choices.len())])
}

fn normalize_47(mask: u8) -> u8 {
    let mut normalized = mask;
    if (mask & EDGE_N == 0) || (mask & EDGE_E == 0) {
        normalized &= !CORNER_NE;
    }
    if (mask & EDGE_S == 0) || (mask & EDGE_E == 0) {
        normalized &= !CORNER_SE;
    }
    if (mask & EDGE_S == 0) || (mask & EDGE_W == 0) {
        normalized &= !CORNER_SW;
    }
    if (mask & EDGE_N == 0) || (mask & EDGE_W == 0) {
        normalized &= !CORNER_NW;
    }
    normalized
}
