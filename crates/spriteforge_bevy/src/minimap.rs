use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_ecs_tilemap::prelude::TilemapSize;

use crate::map_generators::path::{MapArea, MapSkeleton, PathSegment};
use crate::BaseTile;

#[derive(Resource, Clone)]
pub struct MiniMapSource {
    pub tiles: Vec<BaseTile>,
    pub map_size: TilemapSize,
    pub skeleton: Option<MapSkeleton>,
}

#[derive(Resource, Clone)]
pub struct MiniMapSettings {
    pub tile_px: u32,
    pub padding: u32,
    pub grass_color: Color,
    pub dirt_color: Color,
    pub water_color: Color,
    pub path_color: Color,
    pub area_color: Color,
    pub background_color: Color,
    pub toggle_paths_key: KeyCode,
    pub toggle_areas_key: KeyCode,
    pub toggle_visible_key: KeyCode,
}

impl Default for MiniMapSettings {
    fn default() -> Self {
        Self {
            tile_px: 4,
            padding: 8,
            grass_color: Color::srgba(0.15, 0.45, 0.22, 1.0),
            dirt_color: Color::srgba(0.45, 0.31, 0.2, 1.0),
            water_color: Color::srgba(0.2, 0.35, 0.7, 1.0),
            path_color: Color::srgba(0.95, 0.95, 0.95, 0.95),
            area_color: Color::srgba(0.95, 0.75, 0.2, 0.9),
            background_color: Color::srgba(0.05, 0.05, 0.05, 0.85),
            toggle_paths_key: KeyCode::Digit1,
            toggle_areas_key: KeyCode::Digit2,
            toggle_visible_key: KeyCode::Digit3,
        }
    }
}

#[derive(Resource)]
pub struct MiniMapState {
    pub show_paths: bool,
    pub show_areas: bool,
    pub visible: bool,
}

impl Default for MiniMapState {
    fn default() -> Self {
        Self {
            show_paths: true,
            show_areas: true,
            visible: true,
        }
    }
}

#[derive(Resource)]
struct MiniMapImage {
    handle: Handle<Image>,
    size: UVec2,
    root: Entity,
}

pub struct MiniMapPlugin;

impl Plugin for MiniMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MiniMapSettings>()
            .init_resource::<MiniMapState>()
            .add_systems(Update, (init_minimap, toggle_minimap_overlays, update_minimap));
    }
}

fn init_minimap(
    mut commands: Commands,
    source: Option<Res<MiniMapSource>>,
    settings: Res<MiniMapSettings>,
    mut images: ResMut<Assets<Image>>,
    existing: Option<Res<MiniMapImage>>,
) {
    if existing.is_some() || source.is_none() {
        return;
    }
    let source = source.unwrap();
    let (size, offset) =
        minimap_image_size(&source.map_size, settings.tile_px, settings.padding);
    let mut image = Image::new_fill(
        Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.data.fill(0);
    let handle = images.add(image);

    let root = commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                right: Val::Px(16.0),
                bottom: Val::Px(16.0),
                width: Val::Px(size.x as f32),
                height: Val::Px(size.y as f32),
                ..Default::default()
            },
            background_color: settings.background_color.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn(ImageBundle {
                style: Style {
                    width: Val::Px(size.x as f32),
                    height: Val::Px(size.y as f32),
                    ..Default::default()
                },
                image: UiImage::new(handle.clone()),
                ..Default::default()
            });
        })
        .id();

    commands.insert_resource(MiniMapImage {
        handle,
        size,
        root,
    });

    if offset != Vec2::ZERO {
        let _ = offset;
    }
}

fn toggle_minimap_overlays(
    keys: Res<ButtonInput<KeyCode>>,
    settings: Res<MiniMapSettings>,
    mut state: ResMut<MiniMapState>,
    minimap: Option<Res<MiniMapImage>>,
    mut visibility_q: Query<&mut Visibility>,
) {
    if keys.just_pressed(settings.toggle_paths_key) {
        state.show_paths = !state.show_paths;
    }
    if keys.just_pressed(settings.toggle_areas_key) {
        state.show_areas = !state.show_areas;
    }
    if keys.just_pressed(settings.toggle_visible_key) {
        state.visible = !state.visible;
        if let Some(minimap) = minimap {
            if let Ok(mut visibility) = visibility_q.get_mut(minimap.root) {
                *visibility = if state.visible {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                };
            }
        }
    }
}

fn update_minimap(
    source: Option<Res<MiniMapSource>>,
    settings: Res<MiniMapSettings>,
    state: Res<MiniMapState>,
    minimap: Option<Res<MiniMapImage>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(source), Some(minimap)) = (source, minimap) else {
        return;
    };
    if !source.is_changed() && !settings.is_changed() && !state.is_changed() {
        return;
    }
    let image = images.get_mut(&minimap.handle);
    let Some(image) = image else {
        return;
    };
    image.data.fill(0);
    let (size, offset) =
        minimap_image_size(&source.map_size, settings.tile_px, settings.padding);
    if minimap.size != size {
        return;
    }

    draw_base_tiles(
        &mut image.data,
        &source.tiles,
        source.map_size,
        &settings,
        offset,
        size,
    );
    if state.show_paths {
        if let Some(skeleton) = &source.skeleton {
            draw_paths(
                &mut image.data,
                &skeleton.paths,
                source.map_size,
                &settings,
                offset,
                size,
            );
        }
    }
    if state.show_areas {
        if let Some(skeleton) = &source.skeleton {
            draw_areas(
                &mut image.data,
                &skeleton.areas,
                source.map_size,
                &settings,
                offset,
                size,
            );
        }
    }
}

fn minimap_image_size(map_size: &TilemapSize, tile_px: u32, padding: u32) -> (UVec2, Vec2) {
    let tile_w = tile_px as f32;
    let tile_h = tile_w * 0.5;
    let map_w = map_size.x as f32;
    let map_h = map_size.y as f32;
    let corners = [
        (0.0, 0.0),
        (map_w - 1.0, 0.0),
        (0.0, map_h - 1.0),
        (map_w - 1.0, map_h - 1.0),
    ];
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    for (x, y) in corners {
        let px = (x - y) * (tile_w * 0.5);
        let py = (x + y) * (tile_h * 0.5);
        min_x = min_x.min(px);
        min_y = min_y.min(py);
        max_x = max_x.max(px);
        max_y = max_y.max(py);
    }
    let width = (max_x - min_x + tile_w).ceil() as u32 + padding * 2;
    let height = (max_y - min_y + tile_h).ceil() as u32 + padding * 2;
    let offset = Vec2::new(
        -min_x + padding as f32 + tile_w * 0.5,
        -min_y + padding as f32 + tile_h * 0.5,
    );
    (UVec2::new(width, height), offset)
}

fn draw_base_tiles(
    data: &mut [u8],
    tiles: &[BaseTile],
    map_size: TilemapSize,
    settings: &MiniMapSettings,
    offset: Vec2,
    size: UVec2,
) {
    for y in 0..map_size.y {
        for x in 0..map_size.x {
            let idx = (y * map_size.x + x) as usize;
            let color = match tiles.get(idx) {
                Some(BaseTile::Grass) => settings.grass_color,
                Some(BaseTile::Dirt) => settings.dirt_color,
                Some(BaseTile::Water) => settings.water_color,
                None => settings.grass_color,
            };
            let (rx, ry) = rotate_coord(x as i32, y as i32, map_size);
            let center = minimap_center(rx, ry, settings.tile_px, offset);
            draw_diamond(data, size, center, settings.tile_px, color);
        }
    }
}

fn draw_paths(
    data: &mut [u8],
    paths: &[PathSegment],
    map_size: TilemapSize,
    settings: &MiniMapSettings,
    offset: Vec2,
    size: UVec2,
) {
    for segment in paths {
        let dx = (segment.end_x - segment.start_x).signum();
        let dy = (segment.end_y - segment.start_y).signum();
        let steps =
            (segment.end_x - segment.start_x).abs() + (segment.end_y - segment.start_y).abs();
        for step in 0..=steps {
            let x = segment.start_x + dx * step;
            let y = segment.start_y + dy * step;
            let (rx, ry) = rotate_coord(x, y, map_size);
            let center = minimap_center(rx, ry, settings.tile_px, offset);
            draw_diamond(data, size, center, settings.tile_px, settings.path_color);
        }
    }
}

fn draw_areas(
    data: &mut [u8],
    areas: &[MapArea],
    map_size: TilemapSize,
    settings: &MiniMapSettings,
    offset: Vec2,
    size: UVec2,
) {
    for area in areas {
        for x in area.min_x..=area.max_x {
            for y in [area.min_y, area.max_y] {
                let (rx, ry) = rotate_coord(x, y, map_size);
                let center = minimap_center(rx, ry, settings.tile_px, offset);
                draw_diamond(data, size, center, settings.tile_px, settings.area_color);
            }
        }
        for y in area.min_y..=area.max_y {
            for x in [area.min_x, area.max_x] {
                let (rx, ry) = rotate_coord(x, y, map_size);
                let center = minimap_center(rx, ry, settings.tile_px, offset);
                draw_diamond(data, size, center, settings.tile_px, settings.area_color);
            }
        }
    }
}

fn minimap_center(x: i32, y: i32, tile_px: u32, offset: Vec2) -> Vec2 {
    let tile_w = tile_px as f32;
    let tile_h = tile_w * 0.5;
    let px = (x as f32 - y as f32) * (tile_w * 0.5) + offset.x;
    let py = (x as f32 + y as f32) * (tile_h * 0.5) + offset.y;
    Vec2::new(px, py)
}

fn rotate_coord(x: i32, y: i32, map_size: TilemapSize) -> (i32, i32) {
    let max_y = map_size.y.saturating_sub(1) as i32;
    let rx: i32 = x;
    let ry: i32 = max_y - y;
    (rx, ry)
}

fn draw_diamond(data: &mut [u8], size: UVec2, center: Vec2, tile_px: u32, color: Color) {
    let tile_w = tile_px as f32;
    let tile_h = tile_w * 0.5;
    let half_w = tile_w * 0.5;
    let half_h = tile_h * 0.5;
    let rgba = color_to_rgba8(color);
    let min_y = (center.y - half_h).floor() as i32;
    let max_y = (center.y + half_h).ceil() as i32;
    for y in min_y..=max_y {
        let dy = (y as f32 - center.y).abs();
        let t = if half_h <= 0.0 { 0.0 } else { 1.0 - (dy / half_h) };
        let span = (half_w * t).ceil() as i32;
        let min_x = (center.x as i32) - span;
        let max_x = (center.x as i32) + span;
        for x in min_x..=max_x {
            set_pixel(data, size, x, y, rgba);
        }
    }
}

fn set_pixel(data: &mut [u8], size: UVec2, x: i32, y: i32, rgba: [u8; 4]) {
    if x < 0 || y < 0 {
        return;
    }
    let x = x as u32;
    let y = y as u32;
    if x >= size.x || y >= size.y {
        return;
    }
    let idx = ((y * size.x + x) * 4) as usize;
    data[idx..idx + 4].copy_from_slice(&rgba);
}

fn color_to_rgba8(color: Color) -> [u8; 4] {
    let [r, g, b, a] = color.to_srgba().to_f32_array();
    [
        (r * 255.0).round() as u8,
        (g * 255.0).round() as u8,
        (b * 255.0).round() as u8,
        (a * 255.0).round() as u8,
    ]
}
