use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

#[derive(Resource)]
pub struct CursorWorldPos(pub Vec2);

impl Default for CursorWorldPos {
    fn default() -> Self {
        Self(Vec2::new(-10000.0, -10000.0))
    }
}

#[derive(Resource, Default, Clone)]
pub struct TileSelectionState {
    pub hovered: Option<TilePos>,
    pub selected: Option<TilePos>,
}

#[derive(Resource, Clone)]
pub struct TileSelectionSettings {
    pub target_map: Option<Entity>,
    pub diamond_y_offset: f32,
}

impl Default for TileSelectionSettings {
    fn default() -> Self {
        Self {
            target_map: None,
            diamond_y_offset: 0.5,
        }
    }
}

impl TileSelectionSettings {
    pub fn new(target_map: Entity) -> Self {
        Self {
            target_map: Some(target_map),
            diamond_y_offset: 0.5,
        }
    }
}

#[derive(Event, Debug, Clone)]
pub struct TileSelectedEvent {
    pub map: Entity,
    pub tile_pos: TilePos,
    pub tile_entity: Option<Entity>,
    pub world_pos: Vec2,
}

pub struct TileSelectionPlugin;

impl Plugin for TileSelectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorWorldPos>()
            .init_resource::<TileSelectionSettings>()
            .init_resource::<TileSelectionState>()
            .add_event::<TileSelectedEvent>()
            .add_systems(First, update_cursor_pos)
            .add_systems(Update, (update_hovered_tile, update_selected_tile));
    }
}

fn update_cursor_pos(
    camera_q: Query<(&GlobalTransform, &Camera)>,
    mut cursor_moved_events: EventReader<CursorMoved>,
    mut cursor_pos: ResMut<CursorWorldPos>,
) {
    for cursor_moved in cursor_moved_events.read() {
        for (cam_t, cam) in camera_q.iter() {
            if let Some(pos) = cam.viewport_to_world_2d(cam_t, cursor_moved.position) {
                cursor_pos.0 = pos;
            }
        }
    }
}

fn update_hovered_tile(
    settings: Res<TileSelectionSettings>,
    cursor_pos: Res<CursorWorldPos>,
    tilemap_q: Query<(&TilemapSize, &TilemapGridSize, &TilemapType, &Transform)>,
    mut state: ResMut<TileSelectionState>,
) {
    let Some(map_entity) = settings.target_map else {
        return;
    };
    let Ok((map_size, grid_size, map_type, map_transform)) = tilemap_q.get(map_entity) else {
        return;
    };
    state.hovered = cursor_to_tile_pos(
        cursor_pos.0,
        map_size,
        grid_size,
        map_type,
        map_transform,
        settings.diamond_y_offset,
    );
}

fn update_selected_tile(
    mut events: EventWriter<TileSelectedEvent>,
    buttons: Res<ButtonInput<MouseButton>>,
    settings: Res<TileSelectionSettings>,
    cursor_pos: Res<CursorWorldPos>,
    tilemap_q: Query<(
        &TilemapSize,
        &TilemapGridSize,
        &TilemapType,
        &Transform,
        Option<&TileStorage>,
    )>,
    mut state: ResMut<TileSelectionState>,
) {
    if !buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(map_entity) = settings.target_map else {
        return;
    };
    let Ok((map_size, grid_size, map_type, map_transform, storage)) =
        tilemap_q.get(map_entity)
    else {
        return;
    };
    let Some(tile_pos) = cursor_to_tile_pos(
        cursor_pos.0,
        map_size,
        grid_size,
        map_type,
        map_transform,
        settings.diamond_y_offset,
    ) else {
        return;
    };
    if state.selected == Some(tile_pos) {
        return;
    }
    state.selected = Some(tile_pos);
    let tile_entity = storage.and_then(|storage| storage.get(&tile_pos));
    events.send(TileSelectedEvent {
        map: map_entity,
        tile_pos,
        tile_entity,
        world_pos: cursor_pos.0,
    });
}

fn cursor_to_tile_pos(
    cursor_pos: Vec2,
    map_size: &TilemapSize,
    grid_size: &TilemapGridSize,
    map_type: &TilemapType,
    map_transform: &Transform,
    diamond_y_offset: f32,
) -> Option<TilePos> {
    let cursor_pos = Vec4::from((cursor_pos, 0.0, 1.0));
    let cursor_in_map_pos = map_transform.compute_matrix().inverse() * cursor_pos;
    let mut cursor_in_map_pos = cursor_in_map_pos.xy();
    if matches!(map_type, TilemapType::Isometric(IsoCoordSystem::Diamond)) {
        cursor_in_map_pos.y += grid_size.y * diamond_y_offset;
    }
    TilePos::from_world_pos(&cursor_in_map_pos, map_size, grid_size, map_type)
}
