use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, ShaderRef, ShaderType, TextureDimension, TextureFormat,
};
use bevy::reflect::TypePath;
use bevy_ecs_tilemap::helpers::geometry::get_tilemap_center_transform;
use bevy_ecs_tilemap::prelude::*;
use rand::{RngCore, SeedableRng};
use rand::rngs::StdRng;
use spriteforge_bevy::{
    build_render_layers,
    load_tilesheet_metadata,
    map_generators::{path, terrain},
    BaseTile, MapSkeleton, MiniMapPlugin, MiniMapSource, TileSelectedEvent,
    TileSelectionPlugin, TileSelectionSettings, TileSelectionState, TilesheetMetadata,
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
const PATH_MAP_WIDTH: u32 = 64;
const PATH_MAP_HEIGHT: u32 = 64;
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
    hover_outline_texture: Handle<Image>,
    selected_outline_texture: Handle<Image>,
    map_size: TilemapSize,
    tile_size: TilemapTileSize,
    grid_size: TilemapGridSize,
}

#[derive(Resource)]
struct MapEntities {
    tilemaps: Vec<Entity>,
    tiles: Vec<Entity>,
    primary_map: Entity,
    hover_map: Entity,
    selected_map: Entity,
}

struct MapSpawn {
    entities: MapEntities,
    base_tiles: Vec<BaseTile>,
    skeleton: Option<MapSkeleton>,
}

#[derive(Resource)]
struct MapTileData {
    tiles: Vec<BaseTile>,
    map_size: TilemapSize,
    skeleton: Option<MapSkeleton>,
}

#[derive(Resource)]
struct SelectedTileUi {
    text_entity: Entity,
    last_selected: Option<TilePos>,
}

#[derive(Resource)]
struct MapSeed(u64);

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
enum MapKind {
    Terrain,
    Path,
}

#[derive(Resource, Default)]
struct OverlayState {
    hovered: Option<TilePos>,
    selected: Option<TilePos>,
    hover_entity: Option<Entity>,
    selected_entity: Option<Entity>,
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
        .add_plugins(TileSelectionPlugin)
        .add_plugins(MiniMapPlugin)
        .init_resource::<OverlayState>()
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
        .add_systems(
            Update,
            (
                regenerate_map_on_space,
                update_tile_overlays,
                camera_pan,
            ),
        )
        .add_systems(Update, update_selected_tile_ui.after(regenerate_map_on_space))
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
    mut images: ResMut<Assets<Image>>,
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
    let _water_mask_texture: Handle<Image> =
        asset_server.load(paths.water_mask_image.to_string_lossy().to_string());
    let water_transition_mask_texture: Handle<Image> =
        asset_server.load(paths.water_transition_mask_image.to_string_lossy().to_string());

    let (map_width, map_height) = match *map_kind {
        MapKind::Path => (PATH_MAP_WIDTH, PATH_MAP_HEIGHT),
        MapKind::Terrain => (MAP_WIDTH, MAP_HEIGHT),
    };
    let map_size = TilemapSize {
        x: map_width,
        y: map_height,
    };
    let map_size_copy = map_size;
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
    let hover_outline_texture = images.add(create_outline_image(
        grass_meta.tile_size,
        [255, 255, 255, 255],
        2,
    ));
    let selected_outline_texture = images.add(create_outline_image(
        grass_meta.tile_size,
        [255, 215, 0, 255],
        2,
    ));
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
        hover_outline_texture,
        selected_outline_texture,
        map_size,
        tile_size,
        grid_size,
    };
    let minimap_grid_size = assets.grid_size;
    let seed = 1337;
    let spawn = spawn_map(&mut commands, &assets, seed, *map_kind);
    commands.insert_resource(assets);
    commands.insert_resource(MapSeed(seed));
    let primary_map = spawn.entities.primary_map;
    commands.insert_resource(spawn.entities);
    commands.insert_resource(MapTileData {
        tiles: spawn.base_tiles.clone(),
        map_size: map_size_copy,
        skeleton: spawn.skeleton.clone(),
    });
    commands.insert_resource(MiniMapSource {
        tiles: spawn.base_tiles,
        map_size: map_size_copy,
        grid_size: minimap_grid_size,
        map_type: TilemapType::Isometric(IsoCoordSystem::Diamond),
        map_entity: Some(primary_map),
        skeleton: spawn.skeleton,
    });
    commands.insert_resource(TileSelectionSettings::new(primary_map));
    spawn_selected_tile_ui(&mut commands, &asset_server);
}

fn spawn_map(
    commands: &mut Commands,
    assets: &MapAssets,
    seed: u64,
    map_kind: MapKind,
) -> MapSpawn {
    let mut rng = StdRng::seed_from_u64(seed);
    let (width, height) = match map_kind {
        MapKind::Path => (PATH_MAP_WIDTH, PATH_MAP_HEIGHT),
        MapKind::Terrain => (MAP_WIDTH, MAP_HEIGHT),
    };
    let (base_tiles, skeleton) = match map_kind {
        MapKind::Terrain => {
            let mut tiles = terrain::generate_terrain_map(width, height, &mut rng);
            terrain::smooth_terrain(&mut tiles, width, height, CLUMP_PASSES);
            terrain::reduce_water_islands(&mut tiles, width, height, WATER_PASS_PASSES);
            (tiles, None)
        }
        MapKind::Path => {
            let skeleton = path::generate_map_skeleton(width, height, &mut rng);
            let tiles = path::rasterize_skeleton(width, height, &skeleton);
            (tiles, Some(skeleton))
        }
    };
    let layers = build_render_layers(
        &base_tiles,
        width,
        height,
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
    let hover_storage = TileStorage::empty(assets.map_size);
    let hover_entity = commands.spawn_empty().id();
    let selected_storage = TileStorage::empty(assets.map_size);
    let selected_entity = commands.spawn_empty().id();

    let mut tiles = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let tile_pos = TilePos { x, y };
            let idx = (y * width + x) as usize;
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
    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    let mut hover_transform =
        get_tilemap_center_transform(&assets.map_size, &assets.grid_size, &map_type, 0.0);
    hover_transform.translation.z = 2.0;
    commands.entity(hover_entity).insert(TilemapBundle {
        grid_size: assets.grid_size,
        size: assets.map_size,
        storage: hover_storage,
        texture: TilemapTexture::Single(assets.hover_outline_texture.clone()),
        tile_size: assets.tile_size,
        map_type,
        transform: hover_transform,
        ..Default::default()
    });
    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    let mut selected_transform =
        get_tilemap_center_transform(&assets.map_size, &assets.grid_size, &map_type, 0.0);
    selected_transform.translation.z = 2.1;
    commands.entity(selected_entity).insert(TilemapBundle {
        grid_size: assets.grid_size,
        size: assets.map_size,
        storage: selected_storage,
        texture: TilemapTexture::Single(assets.selected_outline_texture.clone()),
        tile_size: assets.tile_size,
        map_type,
        transform: selected_transform,
        ..Default::default()
    });

    MapSpawn {
        entities: MapEntities {
        tilemaps: vec![
            grass_entity,
            dirt_entity,
            transition_entity,
            water_entity,
            water_transition_entity,
            hover_entity,
            selected_entity,
        ],
        tiles,
        primary_map: grass_entity,
        hover_map: hover_entity,
        selected_map: selected_entity,
        },
        base_tiles,
        skeleton,
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

fn update_tile_overlays(
    mut commands: Commands,
    selection: Res<TileSelectionState>,
    entities: Res<MapEntities>,
    mut overlay: ResMut<OverlayState>,
    mut storage_q: Query<&mut TileStorage>,
) {
    if selection.selected != overlay.selected {
        if let Some(prev_pos) = overlay.selected.take() {
            if let Ok(mut storage) = storage_q.get_mut(entities.selected_map) {
                if let Some(entity) = storage.get(&prev_pos) {
                    commands.entity(entity).despawn();
                }
                storage.remove(&prev_pos);
            }
        }
        overlay.selected_entity = None;
        overlay.selected = selection.selected;
        if let Some(tile_pos) = selection.selected {
            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(entities.selected_map),
                    texture_index: TileTextureIndex(0),
                    ..Default::default()
                })
                .id();
            if let Ok(mut storage) = storage_q.get_mut(entities.selected_map) {
                storage.set(&tile_pos, tile_entity);
            }
            overlay.selected_entity = Some(tile_entity);
        }
    }

    let hover_pos = selection
        .hovered
        .filter(|pos| Some(*pos) != selection.selected);
    if hover_pos != overlay.hovered {
        if let Some(prev_pos) = overlay.hovered.take() {
            if let Ok(mut storage) = storage_q.get_mut(entities.hover_map) {
                if let Some(entity) = storage.get(&prev_pos) {
                    commands.entity(entity).despawn();
                }
                storage.remove(&prev_pos);
            }
        }
        overlay.hover_entity = None;
        overlay.hovered = hover_pos;
        if let Some(tile_pos) = hover_pos {
            let tile_entity = commands
                .spawn(TileBundle {
                    position: tile_pos,
                    tilemap_id: TilemapId(entities.hover_map),
                    texture_index: TileTextureIndex(0),
                    ..Default::default()
                })
                .id();
            if let Ok(mut storage) = storage_q.get_mut(entities.hover_map) {
                storage.set(&tile_pos, tile_entity);
            }
            overlay.hover_entity = Some(tile_entity);
        }
    }
}

fn spawn_selected_tile_ui(commands: &mut Commands, _asset_server: &Res<AssetServer>) {
    let mut text_entity = Entity::PLACEHOLDER;
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                bottom: Val::Px(20.0),
                padding: UiRect::all(Val::Px(12.0)),
                min_width: Val::Px(220.0),
                ..Default::default()
            },
            background_color: Color::srgba(0.05, 0.05, 0.05, 0.85).into(),
            ..Default::default()
        })
        .with_children(|parent| {
            text_entity = parent
                .spawn(TextBundle::from_section(
                    "No tile selected",
                    TextStyle {
                        font_size: 18.0,
                        color: Color::WHITE,
                        ..Default::default()
                    },
                ))
                .id();
        });
    commands.insert_resource(SelectedTileUi {
        text_entity,
        last_selected: None,
    });
}

fn update_selected_tile_ui(
    mut events: EventReader<TileSelectedEvent>,
    mut ui: ResMut<SelectedTileUi>,
    tile_data: Res<MapTileData>,
    mut text_q: Query<&mut Text>,
) {
    let mut latest = None;
    for event in events.read() {
        latest = Some(event.tile_pos);
    }
    let Some(tile_pos) = latest else {
        return;
    };
    if ui.last_selected == Some(tile_pos) {
        return;
    }
    ui.last_selected = Some(tile_pos);
    let Ok(mut text) = text_q.get_mut(ui.text_entity) else {
        return;
    };
    let idx = (tile_pos.y * tile_data.map_size.x + tile_pos.x) as usize;
    let tile_type = match tile_data.tiles.get(idx) {
        Some(BaseTile::Grass) => "Grass",
        Some(BaseTile::Dirt) => "Dirt",
        Some(BaseTile::Water) => "Water",
        None => "Unknown",
    };
    text.sections[0].value = format!(
        "Selected Tile\nPos: {}, {}\nType: {}",
        tile_pos.x, tile_pos.y, tile_type
    );
}

fn regenerate_map_on_space(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    assets: Res<MapAssets>,
    mut seed: ResMut<MapSeed>,
    mut overlay: ResMut<OverlayState>,
    mut selection_state: ResMut<TileSelectionState>,
    mut selection_settings: ResMut<TileSelectionSettings>,
    mut entities: ResMut<MapEntities>,
    map_kind: Res<MapKind>,
    mut tile_data: ResMut<MapTileData>,
    mut minimap: ResMut<MiniMapSource>,
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
    if let Some(entity) = overlay.hover_entity.take() {
        commands.entity(entity).despawn();
    }
    if let Some(entity) = overlay.selected_entity.take() {
        commands.entity(entity).despawn();
    }
    overlay.hovered = None;
    overlay.selected = None;
    selection_state.hovered = None;
    selection_state.selected = None;

    let mut seed_rng = StdRng::seed_from_u64(seed.0);
    seed.0 = seed_rng.next_u64();
    let spawn = spawn_map(&mut commands, &assets, seed.0, *map_kind);
    *entities = spawn.entities;
    tile_data.tiles = spawn.base_tiles.clone();
    tile_data.map_size = assets.map_size;
    tile_data.skeleton = spawn.skeleton.clone();
    minimap.tiles = spawn.base_tiles;
    minimap.map_size = assets.map_size;
    minimap.grid_size = assets.grid_size;
    minimap.map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    minimap.map_entity = Some(entities.primary_map);
    minimap.skeleton = spawn.skeleton;
    selection_settings.target_map = Some(entities.primary_map);
}

fn create_outline_image(size: u32, color: [u8; 4], thickness: u32) -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    if size == 0 || thickness == 0 {
        return image;
    }
    let size_f = size.saturating_sub(1) as f32;
    let left_x = 0.0;
    let right_x = size_f;
    let bottom_y = size_f;
    let height = size_f / 2.0;
    let top_y = bottom_y - height;
    let cx = size_f / 2.0;
    let mid_y = bottom_y - height / 2.0;
    let y_start = top_y.ceil() as i32;
    let y_end = bottom_y.floor() as i32;

    for y in y_start..=y_end {
        let yf = y as f32;
        let (lx, rx) = if yf <= mid_y {
            let t = (yf - top_y) / (mid_y - top_y);
            (lerp(cx, left_x, t), lerp(cx, right_x, t))
        } else {
            let t = (yf - mid_y) / (bottom_y - mid_y);
            (lerp(left_x, cx, t), lerp(right_x, cx, t))
        };
        let lx = lx.round() as i32;
        let rx = rx.round() as i32;
        for offset in 0..thickness as i32 {
            set_pixel(&mut image, lx + offset, y, color);
            set_pixel(&mut image, rx - offset, y, color);
        }
    }
    image
}

fn set_pixel(image: &mut Image, x: i32, y: i32, color: [u8; 4]) {
    if x < 0 || y < 0 {
        return;
    }
    let width = image.width() as i32;
    let height = image.height() as i32;
    if x >= width || y >= height {
        return;
    }
    let idx = ((y * width + x) * 4) as usize;
    image.data[idx..idx + 4].copy_from_slice(&color);
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
