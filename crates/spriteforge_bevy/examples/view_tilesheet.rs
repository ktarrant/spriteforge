use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy_ecs_tilemap::helpers::geometry::get_tilemap_center_transform;
use bevy_ecs_tilemap::prelude::*;
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use spriteforge_bevy::load_tilesheet_metadata;
use std::path::PathBuf;

const GRASS_IMAGE: &str = "out/tilesheet/grass.png";
const GRASS_META: &str = "out/tilesheet/grass.json";
const DIRT_IMAGE: &str = "out/tilesheet/dirt.png";
const DIRT_META: &str = "out/tilesheet/dirt.json";
const MAP_WIDTH: u32 = 24;
const MAP_HEIGHT: u32 = 24;
const CLUMP_PASSES: usize = 3;
const CAMERA_MOVE_SPEED: f32 = 900.0;
const CAMERA_ZOOM: f32 = 1.6;

#[derive(Resource)]
struct TilesheetPaths {
    grass_image: PathBuf,
    grass_meta: PathBuf,
    dirt_image: PathBuf,
    dirt_meta: PathBuf,
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

    let grass_texture: Handle<Image> =
        asset_server.load(paths.grass_image.to_string_lossy().to_string());
    let dirt_texture: Handle<Image> =
        asset_server.load(paths.dirt_image.to_string_lossy().to_string());

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

    let mut grass_storage = TileStorage::empty(map_size);
    let grass_entity = commands.spawn_empty().id();
    let mut dirt_storage = TileStorage::empty(map_size);
    let dirt_entity = commands.spawn_empty().id();

    for y in 0..MAP_HEIGHT {
        for x in 0..MAP_WIDTH {
            let is_grass = terrain[(y * MAP_WIDTH + x) as usize];
            let tile_pos = TilePos { x, y };
            if is_grass {
                let index = rng.gen_range(0..grass_meta.tile_count) as u32;
                let tile_entity = commands
                    .spawn(TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(grass_entity),
                        texture_index: TileTextureIndex(index),
                        ..Default::default()
                    })
                    .id();
                grass_storage.set(&tile_pos, tile_entity);
            } else {
                let index = rng.gen_range(0..dirt_meta.tile_count) as u32;
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

fn generate_terrain_map(width: u32, height: u32, rng: &mut StdRng) -> Vec<bool> {
    let mut cells = vec![false; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            cells[idx] = rng.gen_range(0.0..1.0) < 0.55;
        }
    }
    cells
}

fn smooth_terrain(cells: &mut [bool], width: u32, height: u32, passes: usize) {
    let mut temp = cells.to_vec();
    for _ in 0..passes {
        for y in 0..height {
            for x in 0..width {
                let mut count = 0;
                let mut total = 0;
                for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                    for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                        let idx = (ny * width + nx) as usize;
                        total += 1;
                        if cells[idx] {
                            count += 1;
                        }
                    }
                }
                let idx = (y * width + x) as usize;
                temp[idx] = count * 2 >= total;
            }
        }
        cells.copy_from_slice(&temp);
    }
}
