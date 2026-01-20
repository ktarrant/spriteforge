use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::reflect::TypePath;
use bevy_ecs_tilemap::helpers::geometry::get_tilemap_center_transform;
use bevy_ecs_tilemap::prelude::*;
use rand::{RngCore, SeedableRng};
use rand::rngs::StdRng;
use spriteforge_bevy::{
    build_render_layers,
    load_tilesheet_metadata,
    map_generators::{path, terrain},
    TilesheetMetadata,
};
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
const WATER_MASK_IMAGE: &str = "out/tilesheet/water_mask.png";
const WATER_TRANSITION_MASK_IMAGE: &str = "out/tilesheet/water_transition_mask.png";
const MAP_WIDTH: u32 = 24;
const MAP_HEIGHT: u32 = 24;
const CLUMP_PASSES: usize = 3;
const WATER_PASS_PASSES: usize = 2;
const CAMERA_MOVE_SPEED: f32 = 900.0;
const CAMERA_ZOOM: f32 = 1.6;
const DEFAULT_MAP_KIND: MapKind = MapKind::Path;

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
    water_mask_image: PathBuf,
    water_transition_mask_image: PathBuf,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
struct WaterFoamMaterial {
    #[texture(0)]
    #[sampler(1)]
    mask_texture: Handle<Image>,
    #[uniform(2)]
    params: WaterFoamParams,
}

#[derive(Clone, Copy, Debug, Default, ShaderType)]
#[allow(dead_code)]
struct WaterFoamParams {
    foam_color: Vec4,
    foam_settings: Vec4,
}

impl MaterialTilemap for WaterFoamMaterial {
    fn fragment_shader() -> ShaderRef {
        "assets/shaders/water_foam.wgsl".into()
    }
}

#[derive(Resource)]
struct MapAssets {
    grass_meta: TilesheetMetadata,
    dirt_meta: TilesheetMetadata,
    transition_meta: TilesheetMetadata,
    water_meta: TilesheetMetadata,
    water_transition_meta: TilesheetMetadata,
    grass_texture: Handle<Image>,
    dirt_texture: Handle<Image>,
    transition_texture: Handle<Image>,
    water_texture: Handle<Image>,
    water_transition_texture: Handle<Image>,
    water_material: Handle<WaterFoamMaterial>,
    water_transition_material: Handle<WaterFoamMaterial>,
    map_size: TilemapSize,
    tile_size: TilemapTileSize,
    grid_size: TilemapGridSize,
}

#[derive(Resource, Default)]
struct MapEntities {
    tilemaps: Vec<Entity>,
    tiles: Vec<Entity>,
}

#[derive(Resource)]
struct MapSeed(u64);

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
enum MapKind {
    Terrain,
    Path,
}

fn main() {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root");
    let map_kind = parse_map_kind(std::env::args());
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
        .add_plugins(MaterialTilemapPlugin::<WaterFoamMaterial>::default())
        .insert_resource(map_kind)
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
            water_mask_image: PathBuf::from(WATER_MASK_IMAGE),
            water_transition_mask_image: PathBuf::from(WATER_TRANSITION_MASK_IMAGE),
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (camera_pan, regenerate_map_on_space))
        .run();
}

fn parse_map_kind<I>(args: I) -> MapKind
where
    I: IntoIterator<Item = String>,
{
    for arg in args {
        if let Some(value) = arg.strip_prefix("--map=") {
            return match value {
                "terrain" => MapKind::Terrain,
                "path" => MapKind::Path,
                _ => DEFAULT_MAP_KIND,
            };
        }
    }
    DEFAULT_MAP_KIND
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<WaterFoamMaterial>>,
    map_kind: Res<MapKind>,
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
    let water_mask_texture: Handle<Image> =
        asset_server.load(paths.water_mask_image.to_string_lossy().to_string());
    let water_transition_mask_texture: Handle<Image> =
        asset_server.load(paths.water_transition_mask_image.to_string_lossy().to_string());

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
    let water_material = materials.add(WaterFoamMaterial {
        mask_texture: water_transition_mask_texture.clone(),
        params: WaterFoamParams {
            foam_color: Vec4::new(0.18, 0.0, 0.0, 0.0),
            foam_settings: Vec4::new(0.012, 3.0, 0.2, 0.0),
        },
    });
    let assets = MapAssets {
        grass_meta,
        dirt_meta,
        transition_meta,
        water_meta,
        water_transition_meta,
        grass_texture,
        dirt_texture,
        transition_texture,
        water_texture,
        water_transition_texture,
        water_material: water_material.clone(),
        water_transition_material: water_material,
        map_size,
        tile_size,
        grid_size,
    };
    let seed = 1337;
    let entities = spawn_map(&mut commands, &assets, seed, *map_kind);
    commands.insert_resource(assets);
    commands.insert_resource(MapSeed(seed));
    commands.insert_resource(entities);
}

fn spawn_map(
    commands: &mut Commands,
    assets: &MapAssets,
    seed: u64,
    map_kind: MapKind,
) -> MapEntities {
    let mut rng = StdRng::seed_from_u64(seed);
    let base_tiles = match map_kind {
        MapKind::Terrain => {
            let mut tiles = terrain::generate_terrain_map(MAP_WIDTH, MAP_HEIGHT, &mut rng);
            terrain::smooth_terrain(&mut tiles, MAP_WIDTH, MAP_HEIGHT, CLUMP_PASSES);
            terrain::reduce_water_islands(&mut tiles, MAP_WIDTH, MAP_HEIGHT, WATER_PASS_PASSES);
            tiles
        }
        MapKind::Path => path::generate_path_map(MAP_WIDTH, MAP_HEIGHT, &mut rng),
    };
    let layers = build_render_layers(
        &base_tiles,
        MAP_WIDTH,
        MAP_HEIGHT,
        &assets.grass_meta,
        &assets.dirt_meta,
        &assets.water_meta,
        &assets.water_transition_meta,
        &assets.transition_meta,
        &mut rng,
    );
    let mut grass_storage = TileStorage::empty(assets.map_size);
    let grass_entity = commands.spawn_empty().id();
    let mut dirt_storage = TileStorage::empty(assets.map_size);
    let dirt_entity = commands.spawn_empty().id();
    let mut transition_storage = TileStorage::empty(assets.map_size);
    let transition_entity = commands.spawn_empty().id();
    let mut water_storage = TileStorage::empty(assets.map_size);
    let water_entity = commands.spawn_empty().id();
    let mut water_transition_storage = TileStorage::empty(assets.map_size);
    let water_transition_entity = commands.spawn_empty().id();

    let mut tiles = Vec::new();
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
                tiles.push(tile_entity);
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
                tiles.push(tile_entity);
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
                tiles.push(tile_entity);
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
                tiles.push(tile_entity);
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
                tiles.push(tile_entity);
            }
        }
    }

    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    let mut grass_transform =
        get_tilemap_center_transform(&assets.map_size, &assets.grid_size, &map_type, 0.0);
    grass_transform.translation.z = 1.0;
    commands.entity(grass_entity).insert(TilemapBundle {
        grid_size: assets.grid_size,
        size: assets.map_size,
        storage: grass_storage,
        texture: TilemapTexture::Single(assets.grass_texture.clone()),
        tile_size: assets.tile_size,
        map_type,
        transform: grass_transform,
        ..Default::default()
    });
    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    let dirt_transform =
        get_tilemap_center_transform(&assets.map_size, &assets.grid_size, &map_type, 0.0);
    commands.entity(dirt_entity).insert(TilemapBundle {
        grid_size: assets.grid_size,
        size: assets.map_size,
        storage: dirt_storage,
        texture: TilemapTexture::Single(assets.dirt_texture.clone()),
        tile_size: assets.tile_size,
        map_type,
        transform: dirt_transform,
        ..Default::default()
    });
    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    let mut transition_transform =
        get_tilemap_center_transform(&assets.map_size, &assets.grid_size, &map_type, 0.0);
    transition_transform.translation.z = 0.5;
    commands.entity(transition_entity).insert(TilemapBundle {
        grid_size: assets.grid_size,
        size: assets.map_size,
        storage: transition_storage,
        texture: TilemapTexture::Single(assets.transition_texture.clone()),
        tile_size: assets.tile_size,
        map_type,
        transform: transition_transform,
        ..Default::default()
    });
    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    let mut water_transform =
        get_tilemap_center_transform(&assets.map_size, &assets.grid_size, &map_type, 0.0);
    water_transform.translation.z = 0.2;
    commands.entity(water_entity).insert(MaterialTilemapBundle {
        grid_size: assets.grid_size,
        size: assets.map_size,
        storage: water_storage,
        texture: TilemapTexture::Single(assets.water_texture.clone()),
        tile_size: assets.tile_size,
        map_type,
        transform: water_transform,
        material: assets.water_material.clone(),
        ..Default::default()
    });
    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    let mut water_transition_transform =
        get_tilemap_center_transform(&assets.map_size, &assets.grid_size, &map_type, 0.0);
    water_transition_transform.translation.z = 0.3;
    commands
        .entity(water_transition_entity)
        .insert(MaterialTilemapBundle {
        grid_size: assets.grid_size,
        size: assets.map_size,
        storage: water_transition_storage,
        texture: TilemapTexture::Single(assets.water_transition_texture.clone()),
        tile_size: assets.tile_size,
        map_type,
        transform: water_transition_transform,
        material: assets.water_transition_material.clone(),
        ..Default::default()
    });

    MapEntities {
        tilemaps: vec![
            grass_entity,
            dirt_entity,
            transition_entity,
            water_entity,
            water_transition_entity,
        ],
        tiles,
    }
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

fn regenerate_map_on_space(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    assets: Res<MapAssets>,
    mut seed: ResMut<MapSeed>,
    mut entities: ResMut<MapEntities>,
    map_kind: Res<MapKind>,
) {
    if !keys.just_pressed(KeyCode::Space) {
        return;
    }

    for entity in entities.tiles.drain(..) {
        commands.entity(entity).despawn();
    }
    for entity in entities.tilemaps.drain(..) {
        commands.entity(entity).despawn();
    }

    let mut seed_rng = StdRng::seed_from_u64(seed.0);
    seed.0 = seed_rng.next_u64();
    *entities = spawn_map(&mut commands, &assets, seed.0, *map_kind);
}
