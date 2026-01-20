use serde::Deserialize;
use std::path::Path;

pub mod minimap;
pub mod selection;
pub use map_generators::path::{MapArea, MapSkeleton, PathSegment};
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
    pub angles: Vec<f32>,
}

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
                    let angles = adjacent_non_grass_angles(x, y, width, height, base_tiles);
                    if !angles.is_empty() {
                        let index = pick_transition_index(&angles, &transition_lookup, rng)
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
                    let angles = adjacent_non_water_angles(x, y, width, height, base_tiles);
                    if !angles.is_empty() {
                        let index =
                            pick_transition_index(&angles, &water_transition_lookup, rng)
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

fn adjacent_non_water_angles(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    tiles: &[BaseTile],
) -> Vec<f32> {
    adjacent_angles(x, y, width, height, tiles, |tile| tile != BaseTile::Water)
}

fn adjacent_non_grass_angles(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    tiles: &[BaseTile],
) -> Vec<f32> {
    adjacent_angles(x, y, width, height, tiles, |tile| tile != BaseTile::Grass)
}

fn adjacent_angles<F>(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    tiles: &[BaseTile],
    mut is_match: F,
) -> Vec<f32>
where
    F: FnMut(BaseTile) -> bool,
{
    let mut angles = Vec::new();
    let north = y > 0 && is_match(tiles[((y - 1) * width + x) as usize]);
    let west = x > 0 && is_match(tiles[(y * width + (x - 1)) as usize]);
    let south = y + 1 < height && is_match(tiles[((y + 1) * width + x) as usize]);
    let east = x + 1 < width && is_match(tiles[(y * width + (x + 1)) as usize]);

    // Edge-adjacent (diamond edges).
    // North -> NE (26.5), West -> NW (153.435), South -> SW (206.565), East -> SE (333.435).
    if north {
        angles.push(206.565);
    }
    if west {
        angles.push(153.435);
    }
    if south {
        angles.push(26.5);
    }
    if east {
        angles.push(333.435);
    }

    if angles.len() > 2 {
        return angles;
    }

    // Point-adjacent (diamond corners). These are diagonal neighbors in grid space.
    // East point (0) -> (x+1, y-1), North point (90) -> (x-1, y-1),
    // West point (180) -> (x-1, y+1), South point (270) -> (x+1, y+1).
    if x + 1 < width && y > 0 && is_match(tiles[((y - 1) * width + (x + 1)) as usize])
    {
        if !north && !east {
            angles.push(270.0);
        }
    }
    if x > 0 && y > 0 && is_match(tiles[((y - 1) * width + (x - 1)) as usize]) {
        if !north && !west {
            angles.push(180.0);
        }
    }
    if x > 0 && y + 1 < height
        && is_match(tiles[((y + 1) * width + (x - 1)) as usize])
    {
        if !west && !south {
            angles.push(90.0);
        }
    }
    if x + 1 < width && y + 1 < height
        && is_match(tiles[((y + 1) * width + (x + 1)) as usize])
    {
        if !south && !east {
            angles.push(0.0);
        }
    }
    angles
}

fn build_transition_lookup(meta: &TilesheetMetadata) -> std::collections::HashMap<String, Vec<u32>> {
    let mut map = std::collections::HashMap::new();
    for tile in &meta.tiles {
        let key = angles_key(&tile.angles);
        map.entry(key).or_insert_with(Vec::new).push(tile.index as u32);
    }
    map
}

fn pick_transition_index<R: rand::Rng>(
    angles: &[f32],
    lookup: &std::collections::HashMap<String, Vec<u32>>,
    rng: &mut R,
) -> Option<u32> {
    if lookup.is_empty() {
        return None;
    }

    let target_keys: std::collections::HashSet<String> = angles_key(angles)
        .split(',')
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.to_string())
        .collect();

    let mut best_matches = 0usize;
    let mut best_choices: Vec<u32> = Vec::new();

    for (key, choices) in lookup {
        if choices.is_empty() {
            continue;
        }
        let match_count = key
            .split(',')
            .filter(|entry| target_keys.contains(*entry))
            .count();
        if target_keys.len() < key.split(',').filter(|entry| !entry.is_empty()).count() {
            continue;
        }
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

fn angles_key(angles: &[f32]) -> String {
    let mut sorted = angles.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    sorted
        .iter()
        .map(|angle| format!("{angle:.3}"))
        .collect::<Vec<_>>()
        .join(",")
}
