pub use spriteforge_assets::{
    load_tilesheet_metadata, normalize_mask, TileMetadata, TilesheetMetadata, CORNER_MASK,
    CORNER_NE, CORNER_NW, CORNER_SE, CORNER_SW, EDGE_E, EDGE_MASK, EDGE_N, EDGE_S, EDGE_W,
};

pub mod map_skeleton;
pub mod minimap;
pub mod selection;
pub use map_skeleton::{AreaType, MapArea, MapSkeleton, MapSkeletonConfig, PathSegment};
pub use minimap::{MiniMapPlugin, MiniMapSettings, MiniMapSource, MiniMapState};
pub use selection::{TileSelectedEvent, TileSelectionPlugin, TileSelectionSettings, TileSelectionState};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseTile {
    Grass,
    Dirt,
    Path,
    Water,
}

#[derive(Debug, Clone)]
pub struct RenderTileLayers {
    pub width: u32,
    pub height: u32,
    pub grass: Vec<Option<u32>>,
    pub dirt: Vec<Option<u32>>,
    pub path: Vec<Option<u32>>,
    pub path_transition: Vec<Option<u32>>,
    pub water: Vec<Option<u32>>,
    pub water_transition: Vec<Option<u32>>,
    pub transition: Vec<Option<u32>>,
    pub trees: Vec<Option<u32>>,
}

pub mod map_generators;

pub fn build_render_layers<R: rand::Rng>(
    base_tiles: &[BaseTile],
    width: u32,
    height: u32,
    grass_meta: &TilesheetMetadata,
    dirt_meta: &TilesheetMetadata,
    path_meta: &TilesheetMetadata,
    path_transition_meta: &TilesheetMetadata,
    water_meta: &TilesheetMetadata,
    water_transition_meta: &TilesheetMetadata,
    transition_meta: &TilesheetMetadata,
    tree_meta: &TilesheetMetadata,
    rng: &mut R,
) -> RenderTileLayers {
    let mut grass = vec![None; base_tiles.len()];
    let mut dirt = vec![None; base_tiles.len()];
    let mut path = vec![None; base_tiles.len()];
    let mut path_transition = vec![None; base_tiles.len()];
    let mut water = vec![None; base_tiles.len()];
    let mut water_transition = vec![None; base_tiles.len()];
    let mut transition = vec![None; base_tiles.len()];
    let mut trees = vec![None; base_tiles.len()];

    let transition_lookup = build_transition_lookup(transition_meta);
    let path_transition_lookup = build_transition_lookup(path_transition_meta);
    let water_transition_lookup = build_transition_lookup(water_transition_meta);
    let tree_density = 0.08;

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
                        if rng.gen_bool(tree_density) {
                            let tree_index = rng.gen_range(0..tree_meta.tile_count) as u32;
                            trees[idx] = Some(tree_index);
                        }
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

    RenderTileLayers {
        width,
        height,
        grass,
        dirt,
        path,
        path_transition,
        water,
        water_transition,
        transition,
        trees,
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
