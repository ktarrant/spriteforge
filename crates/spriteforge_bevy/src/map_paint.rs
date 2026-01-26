pub use spriteforge_assets::{
    load_tilesheet_metadata, normalize_mask, TileMetadata, TilesheetMetadata, CORNER_MASK,
    CORNER_NE, CORNER_NW, CORNER_SE, CORNER_SW, EDGE_E, EDGE_MASK, EDGE_N, EDGE_S, EDGE_W,
};

pub use crate::map_layout::{AreaType, MapArea, MapLayout, MapLayoutConfig, PathSegment};
use crate::map_raster::{EnvironmentKind, EnvironmentObject};
pub use crate::minimap::{MiniMapPlugin, MiniMapSettings, MiniMapSource, MiniMapState};
pub use crate::selection::{
    TileSelectedEvent, TileSelectionPlugin, TileSelectionSettings, TileSelectionState,
};

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseTile {
    Grass,
    Dirt,
    Path,
    Water,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayerKind {
    Grass,
    Dirt,
    Path,
    PathTransition,
    Transition,
    Water,
    WaterTransition,
    Trees,
    Bushes,
}

#[derive(Debug, Clone)]
pub struct RenderTileLayers {
    pub width: u32,
    pub height: u32,
    pub layers: HashMap<LayerKind, Vec<Option<u32>>>,
}


pub fn build_render_layers<'a, R, F>(
    base_tiles: &[BaseTile],
    environment: &[EnvironmentObject],
    width: u32,
    height: u32,
    meta_for: F,
    rng: &mut R,
) -> RenderTileLayers
where
    R: rand::Rng,
    F: Fn(LayerKind) -> &'a TilesheetMetadata,
{
    let grass_meta = meta_for(LayerKind::Grass);
    let dirt_meta = meta_for(LayerKind::Dirt);
    let path_meta = meta_for(LayerKind::Path);
    let path_transition_meta = meta_for(LayerKind::PathTransition);
    let water_meta = meta_for(LayerKind::Water);
    let water_transition_meta = meta_for(LayerKind::WaterTransition);
    let transition_meta = meta_for(LayerKind::Transition);
    let tree_meta = meta_for(LayerKind::Trees);
    let bush_meta = meta_for(LayerKind::Bushes);

    let mut grass = vec![None; base_tiles.len()];
    let mut dirt = vec![None; base_tiles.len()];
    let mut path = vec![None; base_tiles.len()];
    let mut path_transition = vec![None; base_tiles.len()];
    let mut water = vec![None; base_tiles.len()];
    let mut water_transition = vec![None; base_tiles.len()];
    let mut transition = vec![None; base_tiles.len()];
    let mut trees = vec![None; base_tiles.len()];
    let mut bushes = vec![None; base_tiles.len()];

    let transition_lookup = build_transition_lookup(transition_meta);
    let path_transition_lookup = build_transition_lookup(path_transition_meta);
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
                BaseTile::Path => {
                    let mask = adjacent_non_path_mask(x, y, width, height, base_tiles);
                    if mask != 0 {
                        let index = pick_transition_index(mask, &path_transition_lookup, rng)
                            .unwrap_or_else(|| {
                                rng.gen_range(0..path_transition_meta.tile_count) as u32
                            });
                        path_transition[idx] = Some(index);
                    } else {
                        let path_index = rng.gen_range(0..path_meta.tile_count) as u32;
                        path[idx] = Some(path_index);
                    }
                }
            }
            if water_transition[idx].is_some() && dirt[idx].is_none() {
                let dirt_index = rng.gen_range(0..dirt_meta.tile_count) as u32;
                dirt[idx] = Some(dirt_index);
            }
            if path_transition[idx].is_some() && dirt[idx].is_none() {
                let dirt_index = rng.gen_range(0..dirt_meta.tile_count) as u32;
                dirt[idx] = Some(dirt_index);
            }
        }
    }

    for object in environment {
        if object.x >= width || object.y >= height {
            continue;
        }
        let idx = (object.y * width + object.x) as usize;
        match object.kind {
            EnvironmentKind::Tree => {
                let tree_index = rng.gen_range(0..tree_meta.tile_count) as u32;
                trees[idx] = Some(tree_index);
            }
            EnvironmentKind::Bush => {
                let bush_index = rng.gen_range(0..bush_meta.tile_count) as u32;
                bushes[idx] = Some(bush_index);
            }
        }
    }

    let mut layers = HashMap::new();
    layers.insert(LayerKind::Grass, grass);
    layers.insert(LayerKind::Dirt, dirt);
    layers.insert(LayerKind::Path, path);
    layers.insert(LayerKind::PathTransition, path_transition);
    layers.insert(LayerKind::Water, water);
    layers.insert(LayerKind::WaterTransition, water_transition);
    layers.insert(LayerKind::Transition, transition);
    layers.insert(LayerKind::Trees, trees);
    layers.insert(LayerKind::Bushes, bushes);

    RenderTileLayers {
        width,
        height,
        layers,
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

fn adjacent_non_path_mask(
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    tiles: &[BaseTile],
) -> u8 {
    adjacent_mask(x, y, width, height, tiles, |tile| tile != BaseTile::Path)
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
        mask |= EDGE_S;
    }
    if x > 0 && is_match(tiles[(y * width + (x - 1)) as usize]) {
        mask |= EDGE_W;
    }
    if y + 1 < height && is_match(tiles[((y + 1) * width + x) as usize]) {
        mask |= EDGE_N;
    }
    if x + 1 < width && is_match(tiles[(y * width + (x + 1)) as usize]) {
        mask |= EDGE_E;
    }

    // Point-adjacent (diamond corners). These are diagonal neighbors in grid space.
    // NE point (0) -> (x+1, y+1)
    // SE point (90) -> (x+1, y-1)
    // SW point (180) -> (x-1, y-1)
    // NW point (270) -> (x-1, y+1)
    if x + 1 < width && y + 1 < height
        && is_match(tiles[((y + 1) * width + (x + 1)) as usize])
    {
        mask |= CORNER_NE;
    }
    if x + 1 < width && y > 0 && is_match(tiles[((y - 1) * width + (x + 1)) as usize])
    {
        mask |= CORNER_SE;
    }
    if x > 0 && y > 0 && is_match(tiles[((y - 1) * width + (x - 1)) as usize]) {
        mask |= CORNER_SW;
    }
    if x > 0 && y + 1 < height
        && is_match(tiles[((y + 1) * width + (x - 1)) as usize])
    {
        mask |= CORNER_NW;
    }
    normalize_mask(mask)
}

fn build_transition_lookup(meta: &TilesheetMetadata) -> std::collections::HashMap<u8, Vec<u32>> {
    let mut map = std::collections::HashMap::new();
    for tile in &meta.tiles {
        let Some(mask) = tile.transition_mask else {
            continue;
        };
        map.entry(mask).or_insert_with(Vec::new).push(tile.index as u32);
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
    let mask = normalize_mask(mask);
    let choices = lookup.get(&mask)?;
    if choices.is_empty() {
        return None;
    }
    Some(choices[rng.gen_range(0..choices.len())])
}
