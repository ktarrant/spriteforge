use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy_ecs_tilemap::helpers::geometry::get_tilemap_center_transform;
use bevy_ecs_tilemap::prelude::*;
use spriteforge_bevy::load_tilesheet_metadata;
use std::path::PathBuf;

const DEFAULT_IMAGE: &str = "out/tilesheet/dirt_to_grass.png";
const DEFAULT_META: &str = "out/tilesheet/dirt_to_grass.json";

#[derive(Resource)]
struct TilesheetPaths {
    image: PathBuf,
    meta: PathBuf,
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
            image: PathBuf::from(DEFAULT_IMAGE),
            meta: workspace_root.join(DEFAULT_META),
        })
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    paths: Res<TilesheetPaths>,
) {
    commands.spawn(Camera2dBundle::default());

    let metadata = match load_tilesheet_metadata(&paths.meta) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Failed to load metadata: {err}");
            return;
        }
    };

    let texture: Handle<Image> = asset_server.load(paths.image.to_string_lossy().to_string());

    let map_size = TilemapSize {
        x: metadata.columns,
        y: metadata.rows,
    };
    let tile_size = TilemapTileSize {
        x: metadata.tile_size as f32,
        y: metadata.tile_size as f32,
    };
    let grid_size = TilemapGridSize {
        x: metadata.tile_size as f32,
        y: (metadata.tile_size as f32) * 0.5,
    };

    let mut storage = TileStorage::empty(map_size);
    let map_entity = commands.spawn_empty().id();

    for tile in &metadata.tiles {
        let tile_pos = TilePos {
            x: tile.col,
            y: tile.row,
        };
        let tile_entity = commands
            .spawn(TileBundle {
                position: tile_pos,
                tilemap_id: TilemapId(map_entity),
                texture_index: TileTextureIndex(tile.index as u32),
                ..Default::default()
            })
            .id();
        storage.set(&tile_pos, tile_entity);
    }

    let map_type = TilemapType::Isometric(IsoCoordSystem::Diamond);
    commands.entity(map_entity).insert(TilemapBundle {
        grid_size,
        size: map_size,
        storage,
        texture: TilemapTexture::Single(texture),
        tile_size,
        map_type,
        transform: get_tilemap_center_transform(&map_size, &grid_size, &map_type, 0.0),
        ..Default::default()
    });
}
