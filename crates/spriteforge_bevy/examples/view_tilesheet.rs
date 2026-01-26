#![allow(dead_code)]

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
    map_raster,
    map_layout,
    BaseTile, LayerKind, MapLayout, MiniMapPlugin, MiniMapSource, TileSelectedEvent,
    TileSelectionPlugin, TileSelectionSettings, TileSelectionState, TilesheetMetadata,
};
use std::collections::HashMap;
use std::path::PathBuf;

const GRASS_IMAGE: &str = "out/tilesheet/grass.png";
const GRASS_META: &str = "out/tilesheet/grass.json";
const DIRT_IMAGE: &str = "out/tilesheet/dirt.png";
const DIRT_META: &str = "out/tilesheet/dirt.json";
const PATH_IMAGE: &str = "out/tilesheet/path.png";
const PATH_META: &str = "out/tilesheet/path.json";
const PATH_TRANSITION_IMAGE: &str = "out/tilesheet/path_transition.png";
const PATH_TRANSITION_META: &str = "out/tilesheet/path_transition.json";
const GRASS_TRANSITION_IMAGE: &str = "out/tilesheet/grass_transition.png";
const GRASS_TRANSITION_META: &str = "out/tilesheet/grass_transition.json";
const WATER_IMAGE: &str = "out/tilesheet/water.png";
const WATER_META: &str = "out/tilesheet/water.json";
const WATER_TRANSITION_IMAGE: &str = "out/tilesheet/water_transition.png";
const WATER_TRANSITION_META: &str = "out/tilesheet/water_transition.json";
const WATER_MASK_IMAGE: &str = "out/tilesheet/water_mask.png";
const WATER_TRANSITION_MASK_IMAGE: &str = "out/tilesheet/water_transition_mask.png";
const TREE_IMAGE: &str = "out/tilesheet/tree.png";
const TREE_META: &str = "out/tilesheet/tree.json";
const TREE_MASK_IMAGE: &str = "out/tilesheet/tree_mask.png";
const BUSH_IMAGE: &str = "out/tilesheet/bush.png";
const BUSH_META: &str = "out/tilesheet/bush.json";
const BUSH_MASK_IMAGE: &str = "out/tilesheet/bush_mask.png";
const MAP_WIDTH: u32 = 64;
const MAP_HEIGHT: u32 = 64;
const MAP_LAYOUT_CONFIG: &str = "assets/map_layouts/rural_fork.json";
const CAMERA_MOVE_SPEED: f32 = 900.0;
const CAMERA_ZOOM: f32 = 1.6;

#[derive(Resource)]
struct TilesheetPaths {
    grass_image: PathBuf,
    grass_meta: PathBuf,
    dirt_image: PathBuf,
    dirt_meta: PathBuf,
    path_image: PathBuf,
    path_meta: PathBuf,
    path_transition_image: PathBuf,
    path_transition_meta: PathBuf,
    grass_transition_image: PathBuf,
    grass_transition_meta: PathBuf,
    water_image: PathBuf,
    water_meta: PathBuf,
    water_transition_image: PathBuf,
    water_transition_meta: PathBuf,
    water_mask_image: PathBuf,
    water_transition_mask_image: PathBuf,
    tree_image: PathBuf,
    tree_meta: PathBuf,
    tree_mask_image: PathBuf,
    bush_image: PathBuf,
    bush_meta: PathBuf,
    bush_mask_image: PathBuf,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
struct WaterFoamMaterial {
    #[texture(0)]
    #[sampler(1)]
    mask_texture: Handle<Image>,
    #[uniform(2)]
    params: WaterFoamParams,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone, Default)]
struct TreeLightMaterial {
    #[texture(0)]
    #[sampler(1)]
    normal_texture: Handle<Image>,
    #[uniform(2)]
    params: TreeLightParams,
}

#[derive(Clone, Copy, Debug, Default, ShaderType)]
struct TreeLightParams {
    light_dir: Vec4,
    ambient_strength: f32,
    diffuse_strength: f32,
    _pad0: Vec2,
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

impl MaterialTilemap for TreeLightMaterial {
    fn fragment_shader() -> ShaderRef {
        "assets/shaders/tree_light.wgsl".into()
    }
}

#[derive(Clone)]
enum LayerMaterial {
    Water(Handle<WaterFoamMaterial>),
    Tree(Handle<TreeLightMaterial>),
}

#[derive(Clone)]
struct LayerAssets {
    meta: TilesheetMetadata,
    texture: Handle<Image>,
    tile_size: TilemapTileSize,
    z: f32,
    material: Option<LayerMaterial>,
}

struct LayerCatalog {
    layers: HashMap<LayerKind, LayerAssets>,
    order: Vec<LayerKind>,
}

impl LayerCatalog {
    fn layer(&self, kind: LayerKind) -> &LayerAssets {
        self.layers
            .get(&kind)
            .unwrap_or_else(|| panic!("Missing layer assets for {kind:?}"))
    }
}

#[derive(Resource)]
struct MapAssets {
    layout_config: map_layout::MapLayoutConfig,
    layers: LayerCatalog,
    tree_materials: Vec<Handle<TreeLightMaterial>>,
    hover_outline_texture: Handle<Image>,
    selected_outline_texture: Handle<Image>,
    map_size: TilemapSize,
    grid_size: TilemapGridSize,
    base_tile_size: TilemapTileSize,
}

impl MapAssets {
    fn layer(&self, kind: LayerKind) -> &LayerAssets {
        self.layers.layer(kind)
    }

    fn layer_meta(&self, kind: LayerKind) -> &TilesheetMetadata {
        &self.layer(kind).meta
    }
}

#[derive(Resource)]
struct MapEntities {
    tilemaps: Vec<Entity>,
    tiles: Vec<Entity>,
    primary_map: Entity,
    layer_maps: HashMap<LayerKind, Entity>,
    hover_map: Entity,
    selected_map: Entity,
}

impl MapEntities {
    fn layer_map(&self, kind: LayerKind) -> Entity {
        *self
            .layer_maps
            .get(&kind)
            .unwrap_or_else(|| panic!("Missing layer map for {kind:?}"))
    }
}

struct MapSpawn {
    entities: MapEntities,
    base_tiles: Vec<BaseTile>,
    skeleton: Option<MapLayout>,
    environment: Vec<map_raster::EnvironmentObject>,
}

#[derive(Resource)]
struct MapTileData {
    tiles: Vec<BaseTile>,
    map_size: TilemapSize,
    skeleton: Option<MapLayout>,
    environment: Vec<map_raster::EnvironmentObject>,
}

#[derive(Resource)]
struct SelectedTileUi {
    text_entity: Entity,
    last_selected: Option<TilePos>,
}

#[derive(Resource)]
struct TimeOfDayUi {
    text_entity: Entity,
}

#[derive(Resource)]
struct MapSeed(u64);

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
enum TimeOfDay {
    Dawn,
    Noon,
    Dusk,
    Night,
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
        .add_plugins(MaterialTilemapPlugin::<TreeLightMaterial>::default())
        .add_plugins(TileSelectionPlugin)
        .add_plugins(MiniMapPlugin)
        .init_resource::<OverlayState>()
        .insert_resource(TimeOfDay::Dawn)
        .insert_resource(TilesheetPaths {
            grass_image: PathBuf::from(GRASS_IMAGE),
            grass_meta: workspace_root.join(GRASS_META),
            dirt_image: PathBuf::from(DIRT_IMAGE),
            dirt_meta: workspace_root.join(DIRT_META),
            path_image: PathBuf::from(PATH_IMAGE),
            path_meta: workspace_root.join(PATH_META),
            path_transition_image: PathBuf::from(PATH_TRANSITION_IMAGE),
            path_transition_meta: workspace_root.join(PATH_TRANSITION_META),
            grass_transition_image: PathBuf::from(GRASS_TRANSITION_IMAGE),
            grass_transition_meta: workspace_root.join(GRASS_TRANSITION_META),
            water_image: PathBuf::from(WATER_IMAGE),
            water_meta: workspace_root.join(WATER_META),
            water_transition_image: PathBuf::from(WATER_TRANSITION_IMAGE),
            water_transition_meta: workspace_root.join(WATER_TRANSITION_META),
            water_mask_image: PathBuf::from(WATER_MASK_IMAGE),
            water_transition_mask_image: PathBuf::from(WATER_TRANSITION_MASK_IMAGE),
            tree_image: PathBuf::from(TREE_IMAGE),
            tree_meta: workspace_root.join(TREE_META),
            tree_mask_image: PathBuf::from(TREE_MASK_IMAGE),
            bush_image: PathBuf::from(BUSH_IMAGE),
            bush_meta: workspace_root.join(BUSH_META),
            bush_mask_image: PathBuf::from(BUSH_MASK_IMAGE),
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                regenerate_map_on_space,
                update_tile_overlays,
                update_time_of_day,
                camera_pan,
            ),
        )
        .add_systems(Update, update_selected_tile_ui.after(regenerate_map_on_space))
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<WaterFoamMaterial>>,
    mut tree_materials: ResMut<Assets<TreeLightMaterial>>,
    paths: Res<TilesheetPaths>,
) {
    let mut camera = Camera2dBundle::default();
    camera.transform.scale = Vec3::splat(CAMERA_ZOOM);
    commands.spawn(camera);

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root");
    let layout_config_path = workspace_root.join(MAP_LAYOUT_CONFIG);
    let layout_config = match map_layout::load_map_layout_config(&layout_config_path) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Failed to load map layout config: {err}");
            return;
        }
    };

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

    let path_meta = match load_tilesheet_metadata(&paths.path_meta) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to load path metadata: {err}");
            return;
        }
    };

    let path_transition_meta = match load_tilesheet_metadata(&paths.path_transition_meta) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to load path transition metadata: {err}");
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
    let tree_meta = match load_tilesheet_metadata(&paths.tree_meta) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to load tree metadata: {err}");
            return;
        }
    };
    let bush_meta = match load_tilesheet_metadata(&paths.bush_meta) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to load bush metadata: {err}");
            return;
        }
    };

    let grass_texture: Handle<Image> =
        asset_server.load(paths.grass_image.to_string_lossy().to_string());
    let dirt_texture: Handle<Image> =
        asset_server.load(paths.dirt_image.to_string_lossy().to_string());
    let path_texture: Handle<Image> =
        asset_server.load(paths.path_image.to_string_lossy().to_string());
    let path_transition_texture: Handle<Image> =
        asset_server.load(paths.path_transition_image.to_string_lossy().to_string());
    let transition_texture: Handle<Image> =
        asset_server.load(paths.grass_transition_image.to_string_lossy().to_string());
    let water_texture: Handle<Image> =
        asset_server.load(paths.water_image.to_string_lossy().to_string());
    let water_transition_texture: Handle<Image> =
        asset_server.load(paths.water_transition_image.to_string_lossy().to_string());
    let tree_texture: Handle<Image> =
        asset_server.load(paths.tree_image.to_string_lossy().to_string());
    let bush_texture: Handle<Image> =
        asset_server.load(paths.bush_image.to_string_lossy().to_string());
    let water_mask_texture: Handle<Image> =
        asset_server.load(paths.water_mask_image.to_string_lossy().to_string());
    let water_transition_mask_texture: Handle<Image> =
        asset_server.load(paths.water_transition_mask_image.to_string_lossy().to_string());
    let tree_mask_texture: Handle<Image> =
        asset_server.load(paths.tree_mask_image.to_string_lossy().to_string());
    let bush_mask_texture: Handle<Image> =
        asset_server.load(paths.bush_mask_image.to_string_lossy().to_string());

    let (map_width, map_height) = (MAP_WIDTH, MAP_HEIGHT);
    let map_size = TilemapSize {
        x: map_width,
        y: map_height,
    };
    let map_size_copy = map_size;
    let sprite_width = grass_meta.sprite_width.unwrap_or(256) as f32;
    let sprite_height = grass_meta.sprite_height.unwrap_or(256) as f32;
    let tile_size = TilemapTileSize {
        x: sprite_width,
        y: sprite_height,
    };
    let tree_sprite_width = tree_meta.sprite_width.unwrap_or(256) as f32;
    let tree_sprite_height = tree_meta.sprite_height.unwrap_or(256) as f32;
    let tree_tile_size = TilemapTileSize {
        x: tree_sprite_width,
        y: tree_sprite_height,
    };
    let bush_sprite_width = bush_meta.sprite_width.unwrap_or(256) as f32;
    let bush_sprite_height = bush_meta.sprite_height.unwrap_or(256) as f32;
    let bush_tile_size = TilemapTileSize {
        x: bush_sprite_width,
        y: bush_sprite_height,
    };
    let grid_size = TilemapGridSize {
        x: sprite_width,
        y: sprite_width * 0.5,
    };
    let water_material = materials.add(WaterFoamMaterial {
        mask_texture: water_mask_texture,
        params: WaterFoamParams {
            foam_color: Vec4::new(0.10, 0.18, 0.22, 0.0),
            foam_settings: Vec4::new(0.018, 2.2, 0.18, 0.0),
        },
    });
    let water_transition_material = materials.add(WaterFoamMaterial {
        mask_texture: water_transition_mask_texture,
        params: WaterFoamParams {
            foam_color: Vec4::new(0.10, 0.18, 0.22, 0.0),
            foam_settings: Vec4::new(0.018, 2.2, 0.18, 0.0),
        },
    });
    let tree_material = tree_materials.add(TreeLightMaterial {
        normal_texture: tree_mask_texture,
        params: tree_light_params(TimeOfDay::Dawn),
    });
    let bush_material = tree_materials.add(TreeLightMaterial {
        normal_texture: bush_mask_texture,
        params: tree_light_params(TimeOfDay::Dawn),
    });
    let hover_outline_texture =
        images.add(create_outline_image(sprite_width as u32, [255, 255, 255, 255], 2));
    let selected_outline_texture =
        images.add(create_outline_image(sprite_width as u32, [255, 215, 0, 255], 2));
    let mut layers = HashMap::new();
    let mut order = Vec::new();
    let mut push_layer = |kind: LayerKind,
                          meta: TilesheetMetadata,
                          texture: Handle<Image>,
                          tile_size: TilemapTileSize,
                          z: f32,
                          material: Option<LayerMaterial>| {
        layers.insert(
            kind,
            LayerAssets {
                meta,
                texture,
                tile_size,
                z,
                material,
            },
        );
        order.push(kind);
    };
    push_layer(LayerKind::Grass, grass_meta, grass_texture, tile_size, 1.0, None);
    push_layer(LayerKind::Dirt, dirt_meta, dirt_texture, tile_size, 0.0, None);
    push_layer(LayerKind::Path, path_meta, path_texture, tile_size, 0.8, None);
    push_layer(
        LayerKind::PathTransition,
        path_transition_meta,
        path_transition_texture,
        tile_size,
        0.9,
        None,
    );
    push_layer(
        LayerKind::Transition,
        transition_meta,
        transition_texture,
        tile_size,
        0.5,
        None,
    );
    push_layer(
        LayerKind::Water,
        water_meta,
        water_texture,
        tile_size,
        0.2,
        Some(LayerMaterial::Water(water_material.clone())),
    );
    push_layer(
        LayerKind::WaterTransition,
        water_transition_meta,
        water_transition_texture,
        tile_size,
        0.3,
        Some(LayerMaterial::Water(water_transition_material.clone())),
    );
    push_layer(
        LayerKind::Trees,
        tree_meta,
        tree_texture,
        tree_tile_size,
        1.6,
        Some(LayerMaterial::Tree(tree_material.clone())),
    );
    push_layer(
        LayerKind::Bushes,
        bush_meta,
        bush_texture,
        bush_tile_size,
        1.5,
        Some(LayerMaterial::Tree(bush_material.clone())),
    );
    let assets = MapAssets {
        layout_config,
        layers: LayerCatalog { layers, order },
        tree_materials: vec![tree_material, bush_material],
        hover_outline_texture,
        selected_outline_texture,
        map_size,
        grid_size,
        base_tile_size: tile_size,
    };
    let minimap_grid_size = assets.grid_size;
    let seed = 1337;
    let spawn = spawn_map(&mut commands, &assets, seed);
    commands.insert_resource(assets);
    commands.insert_resource(MapSeed(seed));
    let primary_map = spawn.entities.primary_map;
    commands.insert_resource(spawn.entities);
    commands.insert_resource(MapTileData {
        tiles: spawn.base_tiles.clone(),
        map_size: map_size_copy,
        skeleton: spawn.skeleton.clone(),
        environment: spawn.environment.clone(),
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
    spawn_time_of_day_ui(&mut commands);
    spawn_selected_tile_ui(&mut commands, &asset_server);
}

fn spawn_map(
    commands: &mut Commands,
    assets: &MapAssets,
    seed: u64,
) -> MapSpawn {
    let mut rng = StdRng::seed_from_u64(seed);
    let (width, height) = (MAP_WIDTH, MAP_HEIGHT);
    let layout = map_layout::generate_map_layout(width, height, &mut rng, &assets.layout_config);
    let raster = map_raster::rasterize_layout(width, height, &layout, &mut rng);
    let skeleton = Some(layout);
    let layers = build_render_layers(
        &raster.base_tiles,
        &raster.environment,
        width,
        height,
        |kind| assets.layer_meta(kind),
        &mut rng,
    );
    let mut layer_storages = HashMap::new();
    let mut layer_entities = HashMap::new();
    for kind in &assets.layers.order {
        let entity = commands.spawn_empty().id();
        layer_entities.insert(*kind, entity);
        layer_storages.insert(*kind, TileStorage::empty(assets.map_size));
    }
    let hover_storage = TileStorage::empty(assets.map_size);
    let hover_entity = commands.spawn_empty().id();
    let selected_storage = TileStorage::empty(assets.map_size);
    let selected_entity = commands.spawn_empty().id();

    let mut tiles = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let tile_pos = TilePos { x, y };
            let idx = (y * width + x) as usize;
            for kind in &assets.layers.order {
                let Some(layer_tiles) = layers.layers.get(kind) else {
                    continue;
                };
                let Some(index) = layer_tiles[idx] else {
                    continue;
                };
                let layer_entity = *layer_entities
                    .get(kind)
                    .unwrap_or_else(|| panic!("Missing layer entity for {kind:?}"));
                let tile_entity = commands
                    .spawn(TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(layer_entity),
                        texture_index: TileTextureIndex(index),
                        ..Default::default()
                    })
                    .id();
                if let Some(storage) = layer_storages.get_mut(kind) {
                    storage.set(&tile_pos, tile_entity);
                }
                tiles.push(tile_entity);
            }
        }
    }

    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    for kind in &assets.layers.order {
        let layer_assets = assets.layer(*kind);
        let mut transform =
            get_tilemap_center_transform(&assets.map_size, &assets.grid_size, &map_type, 0.0);
        transform.translation.z = layer_assets.z;
        let storage = layer_storages
            .remove(kind)
            .unwrap_or_else(|| panic!("Missing layer storage for {kind:?}"));
        let entity = *layer_entities
            .get(kind)
            .unwrap_or_else(|| panic!("Missing layer entity for {kind:?}"));
        match layer_assets.material.as_ref() {
            Some(LayerMaterial::Water(material)) => {
                commands.entity(entity).insert(MaterialTilemapBundle {
                    grid_size: assets.grid_size,
                    size: assets.map_size,
                    storage,
                    texture: TilemapTexture::Single(layer_assets.texture.clone()),
                    tile_size: layer_assets.tile_size,
                    map_type,
                    transform,
                    material: material.clone(),
                    ..Default::default()
                });
            }
            Some(LayerMaterial::Tree(material)) => {
                commands.entity(entity).insert(MaterialTilemapBundle {
                    grid_size: assets.grid_size,
                    size: assets.map_size,
                    storage,
                    texture: TilemapTexture::Single(layer_assets.texture.clone()),
                    tile_size: layer_assets.tile_size,
                    map_type,
                    transform,
                    material: material.clone(),
                    ..Default::default()
                });
            }
            None => {
                commands.entity(entity).insert(TilemapBundle {
                    grid_size: assets.grid_size,
                    size: assets.map_size,
                    storage,
                    texture: TilemapTexture::Single(layer_assets.texture.clone()),
                    tile_size: layer_assets.tile_size,
                    map_type,
                    transform,
                    ..Default::default()
                });
            }
        }
    }
    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    let mut hover_transform =
        get_tilemap_center_transform(&assets.map_size, &assets.grid_size, &map_type, 0.0);
    hover_transform.translation.z = 2.0;
    commands.entity(hover_entity).insert(TilemapBundle {
        grid_size: assets.grid_size,
        size: assets.map_size,
        storage: hover_storage,
        texture: TilemapTexture::Single(assets.hover_outline_texture.clone()),
        tile_size: assets.base_tile_size,
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
        tile_size: assets.base_tile_size,
        map_type,
        transform: selected_transform,
        ..Default::default()
    });

    let mut tilemaps = Vec::new();
    for kind in &assets.layers.order {
        if let Some(entity) = layer_entities.get(kind) {
            tilemaps.push(*entity);
        }
    }
    tilemaps.push(hover_entity);
    tilemaps.push(selected_entity);

    MapSpawn {
        entities: MapEntities {
            tilemaps,
            tiles,
            primary_map: *layer_entities
                .get(&LayerKind::Grass)
                .unwrap_or_else(|| panic!("Missing grass layer")),
            layer_maps: layer_entities,
            hover_map: hover_entity,
            selected_map: selected_entity,
        },
        base_tiles: raster.base_tiles,
        skeleton,
        environment: raster.environment,
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
                bottom: Val::Px(64.0),
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

fn spawn_time_of_day_ui(commands: &mut Commands) {
    let mut text_entity = Entity::PLACEHOLDER;
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                bottom: Val::Px(20.0),
                padding: UiRect::all(Val::Px(10.0)),
                min_width: Val::Px(220.0),
                ..Default::default()
            },
            background_color: Color::srgba(0.05, 0.05, 0.05, 0.85).into(),
            ..Default::default()
        })
        .with_children(|parent| {
            text_entity = parent
                .spawn(TextBundle::from_section(
                    "Time: Dawn",
                    TextStyle {
                        font_size: 16.0,
                        color: Color::WHITE,
                        ..Default::default()
                    },
                ))
                .id();
        });
    commands.insert_resource(TimeOfDayUi { text_entity });
}

fn update_selected_tile_ui(
    mut events: EventReader<TileSelectedEvent>,
    mut ui: ResMut<SelectedTileUi>,
    assets: Res<MapAssets>,
    entities: Res<MapEntities>,
    tile_data: Res<MapTileData>,
    storage_q: Query<&TileStorage>,
    tile_q: Query<&TileTextureIndex>,
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
        Some(BaseTile::Path) => "Path",
        Some(BaseTile::Water) => "Water",
        None => "Unknown",
    };
    let mut lines = vec![
        "Selected Tile".to_string(),
        format!("Pos: {}, {}", tile_pos.x, tile_pos.y),
        format!("Type: {}", tile_type),
    ];
    let environment = environment_for_tile(tile_pos, &tile_data.environment);
    if environment.is_empty() {
        lines.push("Environment: None".to_string());
    } else {
        let labels = environment
            .iter()
            .map(|kind| environment_kind_label(*kind))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("Environment: {}", labels));
    }
    if let Some(mask) = transition_mask_for_tile(
        entities.layer_map(LayerKind::Transition),
        tile_pos,
        &storage_q,
        &tile_q,
        assets.layer_meta(LayerKind::Transition),
    ) {
        lines.push(format!("Grass Transition: {:08b}", mask));
    }
    if let Some(mask) = transition_mask_for_tile(
        entities.layer_map(LayerKind::WaterTransition),
        tile_pos,
        &storage_q,
        &tile_q,
        assets.layer_meta(LayerKind::WaterTransition),
    ) {
        lines.push(format!("Water Transition: {:08b}", mask));
    }
    if let Some(mask) = transition_mask_for_tile(
        entities.layer_map(LayerKind::PathTransition),
        tile_pos,
        &storage_q,
        &tile_q,
        assets.layer_meta(LayerKind::PathTransition),
    ) {
        lines.push(format!("Path Transition: {:08b}", mask));
    }
    text.sections[0].value = lines.join("\n");
}

fn transition_mask_for_tile(
    map_entity: Entity,
    tile_pos: TilePos,
    storage_q: &Query<&TileStorage>,
    tile_q: &Query<&TileTextureIndex>,
    meta: &TilesheetMetadata,
) -> Option<u8> {
    let storage = storage_q.get(map_entity).ok()?;
    let tile_entity = storage.get(&tile_pos)?;
    let texture_index = tile_q.get(tile_entity).ok()?;
    let tile = meta.tiles.get(texture_index.0 as usize)?;
    tile.transition_mask
}

fn environment_for_tile(
    tile_pos: TilePos,
    environment: &[map_raster::EnvironmentObject],
) -> Vec<map_raster::EnvironmentKind> {
    let mut results = Vec::new();
    for object in environment {
        if object.x == tile_pos.x && object.y == tile_pos.y {
            results.push(object.kind);
        }
    }
    results
}

fn environment_kind_label(kind: map_raster::EnvironmentKind) -> &'static str {
    match kind {
        map_raster::EnvironmentKind::Tree => "Tree",
        map_raster::EnvironmentKind::Bush => "Bush",
    }
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
    let spawn = spawn_map(&mut commands, &assets, seed.0);
    *entities = spawn.entities;
    tile_data.tiles = spawn.base_tiles.clone();
    tile_data.map_size = assets.map_size;
    tile_data.skeleton = spawn.skeleton.clone();
    tile_data.environment = spawn.environment.clone();
    minimap.tiles = spawn.base_tiles;
    minimap.map_size = assets.map_size;
    minimap.grid_size = assets.grid_size;
    minimap.map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    minimap.map_entity = Some(entities.primary_map);
    minimap.skeleton = spawn.skeleton;
    selection_settings.target_map = Some(entities.primary_map);
}

fn update_time_of_day(
    keys: Res<ButtonInput<KeyCode>>,
    mut time_of_day: ResMut<TimeOfDay>,
    assets: Res<MapAssets>,
    mut materials: ResMut<Assets<TreeLightMaterial>>,
    ui: Res<TimeOfDayUi>,
    mut text_q: Query<&mut Text>,
) {
    if !keys.just_pressed(KeyCode::KeyT) {
        return;
    }

    let next = match *time_of_day {
        TimeOfDay::Dawn => TimeOfDay::Noon,
        TimeOfDay::Noon => TimeOfDay::Dusk,
        TimeOfDay::Dusk => TimeOfDay::Night,
        TimeOfDay::Night => TimeOfDay::Dawn,
    };

    if *time_of_day != next {
        *time_of_day = next;
        for handle in &assets.tree_materials {
            if let Some(material) = materials.get_mut(handle) {
                material.params = tree_light_params(next);
            }
        }
        if let Ok(mut text) = text_q.get_mut(ui.text_entity) {
            text.sections[0].value = format!("Time: {}", time_of_day_label(next));
        }
    }
}

fn time_of_day_label(time_of_day: TimeOfDay) -> &'static str {
    match time_of_day {
        TimeOfDay::Dawn => "Dawn",
        TimeOfDay::Noon => "Noon",
        TimeOfDay::Dusk => "Dusk",
        TimeOfDay::Night => "Night",
    }
}

fn tree_light_params(time_of_day: TimeOfDay) -> TreeLightParams {
    match time_of_day {
        TimeOfDay::Dawn => TreeLightParams {
            light_dir: Vec4::new(-0.707, 0.707, 0.0, 0.0),
            ambient_strength: 0.35,
            diffuse_strength: 0.65,
            _pad0: Vec2::ZERO,
        },
        TimeOfDay::Noon => TreeLightParams {
            light_dir: Vec4::new(0.0, 0.0, 1.0, 0.0),
            ambient_strength: 0.4,
            diffuse_strength: 0.55,
            _pad0: Vec2::ZERO,
        },
        TimeOfDay::Dusk => TreeLightParams {
            light_dir: Vec4::new(0.707, -0.707, 0.0, 0.0),
            ambient_strength: 0.35,
            diffuse_strength: 0.65,
            _pad0: Vec2::ZERO,
        },
        TimeOfDay::Night => TreeLightParams {
            light_dir: Vec4::new(0.0, 0.0, 1.0, 0.0),
            ambient_strength: 0.2,
            diffuse_strength: 0.0,
            _pad0: Vec2::ZERO,
        },
    }
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
