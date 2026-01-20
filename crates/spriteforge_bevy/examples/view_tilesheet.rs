use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, ShaderRef, ShaderType, TextureDimension, TextureFormat,
};
use bevy::reflect::TypePath;
use bevy_ecs_tilemap::helpers::geometry::get_tilemap_center_transform;
use bevy_ecs_tilemap::prelude::*;
use rand::{Rng, RngCore, SeedableRng};
use rand::rngs::StdRng;
use spriteforge_bevy::{build_render_layers, load_tilesheet_metadata, BaseTile, TilesheetMetadata};
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

#[derive(Resource)]
struct MapTileData {
    tiles: Vec<BaseTile>,
    map_size: TilemapSize,
}

#[derive(Resource)]
struct SelectedTileUi {
    text_entity: Entity,
    last_selected: Option<TilePos>,
}

#[derive(Resource)]
struct MapSeed(u64);

#[derive(Resource)]
struct CursorPos(Vec2);

impl Default for CursorPos {
    fn default() -> Self {
        Self(Vec2::new(-10000.0, -10000.0))
    }
}

#[derive(Resource, Default)]
struct HighlightState {
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
        .init_resource::<CursorPos>()
        .init_resource::<HighlightState>()
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
        .add_systems(First, update_cursor_pos)
        .add_systems(
            Update,
            (
                regenerate_map_on_space,
                update_tile_selection,
                update_tile_hover,
                camera_pan,
            ),
        )
        .add_systems(
            Update,
            update_selected_tile_ui
                .after(update_tile_selection)
                .after(regenerate_map_on_space),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<WaterFoamMaterial>>,
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

    let map_size = TilemapSize {
        x: MAP_WIDTH,
        y: MAP_HEIGHT,
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
    let seed = 1337;
    let entities = spawn_map(&mut commands, &assets, seed);
    commands.insert_resource(assets);
    commands.insert_resource(MapSeed(seed));
    commands.insert_resource(entities.entities);
    commands.insert_resource(MapTileData {
        tiles: entities.base_tiles,
        map_size: map_size_copy,
    });
    spawn_selected_tile_ui(&mut commands, &asset_server);
}

struct MapSpawn {
    entities: MapEntities,
    base_tiles: Vec<BaseTile>,
}

fn spawn_map(commands: &mut Commands, assets: &MapAssets, seed: u64) -> MapSpawn {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut terrain = generate_terrain_map(MAP_WIDTH, MAP_HEIGHT, &mut rng);
    smooth_terrain(&mut terrain, MAP_WIDTH, MAP_HEIGHT, CLUMP_PASSES);
    reduce_water_islands(&mut terrain, MAP_WIDTH, MAP_HEIGHT, WATER_PASS_PASSES);
    let base_tiles = terrain.clone();
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
    let hover_storage = TileStorage::empty(assets.map_size);
    let hover_entity = commands.spawn_empty().id();
    let selected_storage = TileStorage::empty(assets.map_size);
    let selected_entity = commands.spawn_empty().id();

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

fn update_cursor_pos(
    camera_q: Query<(&GlobalTransform, &Camera)>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut cursor_pos: ResMut<CursorPos>,
) {
    for cursor_moved in cursor_moved_events.read() {
        for (cam_t, cam) in camera_q.iter() {
            if let Some(pos) = cam.viewport_to_world_2d(cam_t, cursor_moved.position) {
                cursor_pos.0 = pos;
            }
        }
    }
}

fn update_tile_selection(
    mut commands: Commands,
    buttons: Res<ButtonInput<MouseButton>>,
    cursor_pos: Res<CursorPos>,
    entities: Res<MapEntities>,
    mut highlight: ResMut<HighlightState>,
    base_map_q: Query<(&TilemapSize, &TilemapGridSize, &TilemapType, &Transform)>,
    mut storage_q: Query<&mut TileStorage>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Ok((map_size, grid_size, map_type, map_transform)) =
        base_map_q.get(entities.primary_map)
    else {
        return;
    };
    let tile_pos = cursor_to_tile_pos(cursor_pos.0, map_size, grid_size, map_type, map_transform);
    if tile_pos == highlight.selected {
        return;
    }
    if let Some(prev_pos) = highlight.selected.take() {
        if let Ok(mut storage) = storage_q.get_mut(entities.selected_map) {
            if let Some(entity) = storage.get(&prev_pos) {
                commands.entity(entity).despawn();
            }
            storage.remove(&prev_pos);
        }
    }
    highlight.selected_entity = None;
    highlight.selected = tile_pos;
    let Some(tile_pos) = tile_pos else {
        return;
    };
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
    highlight.selected_entity = Some(tile_entity);
}

fn update_tile_hover(
    mut commands: Commands,
    cursor_pos: Res<CursorPos>,
    entities: Res<MapEntities>,
    mut highlight: ResMut<HighlightState>,
    base_map_q: Query<(&TilemapSize, &TilemapGridSize, &TilemapType, &Transform)>,
    mut storage_q: Query<&mut TileStorage>,
) {
    let Ok((map_size, grid_size, map_type, map_transform)) =
        base_map_q.get(entities.primary_map)
    else {
        return;
    };
    let mut tile_pos =
        cursor_to_tile_pos(cursor_pos.0, map_size, grid_size, map_type, map_transform);
    if tile_pos == highlight.selected {
        tile_pos = None;
    }
    if tile_pos == highlight.hovered {
        return;
    }
    if let Some(prev_pos) = highlight.hovered.take() {
        if let Ok(mut storage) = storage_q.get_mut(entities.hover_map) {
            if let Some(entity) = storage.get(&prev_pos) {
                commands.entity(entity).despawn();
            }
            storage.remove(&prev_pos);
        }
    }
    highlight.hover_entity = None;
    highlight.hovered = tile_pos;
    let Some(tile_pos) = tile_pos else {
        return;
    };
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
    highlight.hover_entity = Some(tile_entity);
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
    mut ui: ResMut<SelectedTileUi>,
    highlight: Res<HighlightState>,
    tile_data: Res<MapTileData>,
    mut text_q: Query<&mut Text>,
) {
    if highlight.selected == ui.last_selected {
        return;
    }
    ui.last_selected = highlight.selected;
    let Ok(mut text) = text_q.get_mut(ui.text_entity) else {
        return;
    };
    let message = if let Some(tile_pos) = highlight.selected {
        let idx = (tile_pos.y * tile_data.map_size.x + tile_pos.x) as usize;
        let tile_type = match tile_data.tiles.get(idx) {
            Some(BaseTile::Grass) => "Grass",
            Some(BaseTile::Dirt) => "Dirt",
            Some(BaseTile::Water) => "Water",
            None => "Unknown",
        };
        format!(
            "Selected Tile\nPos: {}, {}\nType: {}",
            tile_pos.x, tile_pos.y, tile_type
        )
    } else {
        "No tile selected".to_string()
    };
    text.sections[0].value = message;
}

fn regenerate_map_on_space(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    assets: Res<MapAssets>,
    mut seed: ResMut<MapSeed>,
    mut highlight: ResMut<HighlightState>,
    mut entities: ResMut<MapEntities>,
    mut tile_data: ResMut<MapTileData>,
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
    if let Some(entity) = highlight.hover_entity.take() {
        commands.entity(entity).despawn();
    }
    if let Some(entity) = highlight.selected_entity.take() {
        commands.entity(entity).despawn();
    }
    highlight.hovered = None;
    highlight.selected = None;

    let mut seed_rng = StdRng::seed_from_u64(seed.0);
    seed.0 = seed_rng.next_u64();
    let spawn = spawn_map(&mut commands, &assets, seed.0);
    *entities = spawn.entities;
    tile_data.tiles = spawn.base_tiles;
}

fn cursor_to_tile_pos(
    cursor_pos: Vec2,
    map_size: &TilemapSize,
    grid_size: &TilemapGridSize,
    map_type: &TilemapType,
    map_transform: &Transform,
) -> Option<TilePos> {
    let cursor_pos = Vec4::from((cursor_pos, 0.0, 1.0));
    let cursor_in_map_pos = map_transform.compute_matrix().inverse() * cursor_pos;
    let mut cursor_in_map_pos = cursor_in_map_pos.xy();
    if matches!(map_type, TilemapType::Isometric(IsoCoordSystem::Diamond)) {
        cursor_in_map_pos.y += grid_size.y * 0.5;
    }
    TilePos::from_world_pos(&cursor_in_map_pos, map_size, grid_size, map_type)
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
