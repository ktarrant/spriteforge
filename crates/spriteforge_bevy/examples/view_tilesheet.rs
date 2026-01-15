use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy_ecs_tilemap::helpers::geometry::get_tilemap_center_transform;
use bevy_ecs_tilemap::prelude::*;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use spriteforge_bevy::{build_render_layers, load_tilesheet_metadata, BaseTile};
use std::path::PathBuf;

const GRASS_IMAGE: &str = "out/tilesheet/grass.png";
const GRASS_META: &str = "out/tilesheet/grass.json";
const DIRT_IMAGE: &str = "out/tilesheet/dirt.png";
const DIRT_META: &str = "out/tilesheet/dirt.json";
const GRASS_TRANSITION_IMAGE: &str = "out/tilesheet/grass_transition.png";
const GRASS_TRANSITION_META: &str = "out/tilesheet/grass_transition.json";
const WATER_IMAGE: &str = "out/tilesheet/water.png";
const WATER_META: &str = "out/tilesheet/water.json";
const WATER_TRANSITION_IMAGE: &str = "out/tilesheet/water_transition.png";
const WATER_TRANSITION_META: &str = "out/tilesheet/water_transition.json";
const MAP_WIDTH: u32 = 24;
const MAP_HEIGHT: u32 = 24;
const CLUMP_PASSES: usize = 3;
const WATER_PASS_PASSES: usize = 2;
const WATER_MIN_NEIGHBORS: i32 = 3;
const CAMERA_MOVE_SPEED: f32 = 900.0;
const CAMERA_ZOOM: f32 = 1.6;

#[derive(Resource)]
struct TilesheetPaths {
    grass_image: PathBuf,
    grass_meta: PathBuf,
    dirt_image: PathBuf,
    dirt_meta: PathBuf,
    grass_transition_image: PathBuf,
    grass_transition_meta: PathBuf,
    water_image: PathBuf,
    water_meta: PathBuf,
    water_transition_image: PathBuf,
    water_transition_meta: PathBuf,
}

fn main() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root");
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(AssetPlugin {
                    file_path: workspace_root.to_string_lossy().to_string(),
                    ..Default::default()
                }),
        )
        .add_plugins(TilemapPlugin)
        .insert_resource(TilesheetPaths {
            grass_image: PathBuf::from(GRASS_IMAGE),
            grass_meta: workspace_root.join(GRASS_META),
            dirt_image: PathBuf::from(DIRT_IMAGE),
            dirt_meta: workspace_root.join(DIRT_META),
            grass_transition_image: PathBuf::from(GRASS_TRANSITION_IMAGE),
            grass_transition_meta: workspace_root.join(GRASS_TRANSITION_META),
            water_image: PathBuf::from(WATER_IMAGE),
            water_meta: workspace_root.join(WATER_META),
            water_transition_image: PathBuf::from(WATER_TRANSITION_IMAGE),
            water_transition_meta: workspace_root.join(WATER_TRANSITION_META),
        })
        .add_systems(Startup, setup)
        .add_systems(Update, camera_pan)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    paths: Res<TilesheetPaths>,
) {
    let mut camera = Camera2dBundle::default();
    camera.transform.scale = Vec3::splat(CAMERA_ZOOM);
    commands.spawn(camera);

    let grass_meta = match load_tilesheet_metadata(&paths.grass_meta) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to load grass metadata: {err}");
            return;
        }
    };

    let dirt_meta = match load_tilesheet_metadata(&paths.dirt_meta) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to load dirt metadata: {err}");
            return;
        }
    };

    let transition_meta = match load_tilesheet_metadata(&paths.grass_transition_meta) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to load transition metadata: {err}");
            return;
        }
    };

    let water_meta = match load_tilesheet_metadata(&paths.water_meta) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to load water metadata: {err}");
            return;
        }
    };
    let water_transition_meta = match load_tilesheet_metadata(&paths.water_transition_meta) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to load water transition metadata: {err}");
            return;
        }
    };

    let grass_texture: Handle<Image> =
        asset_server.load(paths.grass_image.to_string_lossy().to_string());
    let dirt_texture: Handle<Image> =
        asset_server.load(paths.dirt_image.to_string_lossy().to_string());
    let transition_texture: Handle<Image> =
        asset_server.load(paths.grass_transition_image.to_string_lossy().to_string());
    let water_texture: Handle<Image> =
        asset_server.load(paths.water_image.to_string_lossy().to_string());
    let water_transition_texture: Handle<Image> =
        asset_server.load(paths.water_transition_image.to_string_lossy().to_string());

    let map_size = TilemapSize {
        x: MAP_WIDTH,
        y: MAP_HEIGHT,
    };
    let tile_size = TilemapTileSize {
        x: grass_meta.tile_size as f32,
        y: grass_meta.tile_size as f32,
    };
    let grid_size = TilemapGridSize {
        x: grass_meta.tile_size as f32,
        y: (grass_meta.tile_size as f32) * 0.5,
    };

    let mut rng = StdRng::seed_from_u64(1337);
    let mut terrain = generate_terrain_map(MAP_WIDTH, MAP_HEIGHT, &mut rng);
    smooth_terrain(&mut terrain, MAP_WIDTH, MAP_HEIGHT, CLUMP_PASSES);
    reduce_water_islands(&mut terrain, MAP_WIDTH, MAP_HEIGHT, WATER_PASS_PASSES);
    let base_tiles = terrain.clone();
    let layers = build_render_layers(
        &base_tiles,
        MAP_WIDTH,
        MAP_HEIGHT,
        &grass_meta,
        &dirt_meta,
        &water_meta,
        &water_transition_meta,
        &transition_meta,
        &mut rng,
    );

    let mut grass_storage = TileStorage::empty(map_size);
    let grass_entity = commands.spawn_empty().id();
    let mut dirt_storage = TileStorage::empty(map_size);
    let dirt_entity = commands.spawn_empty().id();
    let mut transition_storage = TileStorage::empty(map_size);
    let transition_entity = commands.spawn_empty().id();
    let mut water_storage = TileStorage::empty(map_size);
    let water_entity = commands.spawn_empty().id();
    let mut water_transition_storage = TileStorage::empty(map_size);
    let water_transition_entity = commands.spawn_empty().id();

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let tile_pos = TilePos { x, y };
            let idx = (y * MAP_WIDTH + x) as usize;
            if let Some(index) = layers.grass[idx] {
                let tile_entity = commands
                    .spawn(TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(grass_entity),
                        texture_index: TileTextureIndex(index),
                        ..Default::default()
                    })
                    .id();
                grass_storage.set(&tile_pos, tile_entity);
            }
            if let Some(index) = layers.dirt[idx] {
                let tile_entity = commands
                    .spawn(TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(dirt_entity),
                        texture_index: TileTextureIndex(index),
                        ..Default::default()
                    })
                    .id();
                dirt_storage.set(&tile_pos, tile_entity);
            }
            if let Some(index) = layers.transition[idx] {
                let tile_entity = commands
                    .spawn(TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(transition_entity),
                        texture_index: TileTextureIndex(index),
                        ..Default::default()
                    })
                    .id();
                transition_storage.set(&tile_pos, tile_entity);
            }
            if let Some(index) = layers.water[idx] {
                let tile_entity = commands
                    .spawn(TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(water_entity),
                        texture_index: TileTextureIndex(index),
                        ..Default::default()
                    })
                    .id();
                water_storage.set(&tile_pos, tile_entity);
            }
            if let Some(index) = layers.water_transition[idx] {
                let tile_entity = commands
                    .spawn(TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(water_transition_entity),
                        texture_index: TileTextureIndex(index),
                        ..Default::default()
                    })
                    .id();
                water_transition_storage.set(&tile_pos, tile_entity);
            }
        }
    }

    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    let mut grass_transform = get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0);
    grass_transform.translation.z = 1.0;
    commands.entity(grass_entity).insert(TilemapBundle {
        grid_size,
        size: map_size,
        storage: grass_storage,
        texture: TilemapTexture::Single(grass_texture),
        tile_size,
        map_type,
        transform: grass_transform,
        ..Default::default()
    });
    let dirt_transform = get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0);
    commands.entity(dirt_entity).insert(TilemapBundle {
        grid_size,
        size: map_size,
        storage: dirt_storage,
        texture: TilemapTexture::Single(dirt_texture),
        tile_size,
        map_type,
        transform: dirt_transform,
        ..Default::default()
    });
    let mut transition_transform =
        get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0);
    transition_transform.translation.z = 0.5;
    commands.entity(transition_entity).insert(TilemapBundle {
        grid_size,
        size: map_size,
        storage: transition_storage,
        texture: TilemapTexture::Single(transition_texture),
        tile_size,
        map_type,
        transform: transition_transform,
        ..Default::default()
    });
    let mut water_transform =
        get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0);
    water_transform.translation.z = 0.2;
    commands.entity(water_entity).insert(TilemapBundle {
        grid_size,
        size: map_size,
        storage: water_storage,
        texture: TilemapTexture::Single(water_texture),
        tile_size,
        map_type,
        transform: water_transform,
        ..Default::default()
    });
    let mut water_transition_transform =
        get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0);
    water_transition_transform.translation.z = 0.3;
    commands.entity(water_transition_entity).insert(TilemapBundle {
        grid_size,
        size: map_size,
        storage: water_transition_storage,
        texture: TilemapTexture::Single(water_transition_texture),
        tile_size,
        map_type,
        transform: water_transition_transform,
        ..Default::default()
    });
}

fn camera_pan(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Camera2d>>,
) {
    let mut direction = Vec2::ZERO;
    if keys.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if keys.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }
    if direction == Vec2::ZERO {
        return;
    }
    let delta = direction.normalize() * CAMERA_MOVE_SPEED * time.delta_seconds();
    for mut transform in &mut query {
        transform.translation.x += delta.x;
        transform.translation.y += delta.y;
    }
}

fn generate_terrain_map(width: u32, height: u32, rng: &mut StdRng) -> Vec<BaseTile> {
    let mut cells = vec![BaseTile::Dirt; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let roll = rng.gen_range(0.0..1.0);
            cells[idx] = if roll < 0.2 {
                BaseTile::Water
            } else if roll < 0.6 {
                BaseTile::Grass
            } else {
                BaseTile::Dirt
            };
        }
    }
    cells
}

fn smooth_terrain(cells: &mut [BaseTile], width: u32, height: u32, passes: usize) {
    let mut temp = cells.to_vec();
    for _ in 0..passes {
        for y in 0..height {
            for x in 0..width {
                let mut grass_count = 0;
                let mut dirt_count = 0;
                let mut water_count = 0;
                for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                    for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                        let idx = (ny * width + nx) as usize;
                        match cells[idx] {
                            BaseTile::Grass => grass_count += 1,
                            BaseTile::Dirt => dirt_count += 1,
                            BaseTile::Water => water_count += 1,
                        }
                    }
                }
                let idx = (y * width + x) as usize;
                let max = grass_count.max(dirt_count).max(water_count);
                temp[idx] = if max == water_count {
                    BaseTile::Water
                } else if max == grass_count {
                    BaseTile::Grass
                } else {
                    BaseTile::Dirt
                };
            }
        }
        cells.copy_from_slice(&temp);
    }
}

fn reduce_water_islands(cells: &mut [BaseTile], width: u32, height: u32, passes: usize) {
    let mut temp = cells.to_vec();
    for _ in 0..passes {
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                if cells[idx] != BaseTile::Water {
                    temp[idx] = cells[idx];
                    continue;
                }
                let mut water_neighbors = 0;
                for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                    for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                        if nx == x && ny == y {
                            continue;
                        }
                        let nidx = (ny * width + nx) as usize;
                        if cells[nidx] == BaseTile::Water {
                            water_neighbors += 1;
                        }
                    }
                }
                if water_neighbors < WATER_MIN_NEIGHBORS {
                    temp[idx] = BaseTile::Dirt;
                } else {
                    temp[idx] = BaseTile::Water;
                }
            }
        }
        cells.copy_from_slice(&temp);
    }
}
