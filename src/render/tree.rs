use std::cmp::Ordering;

use image::{ImageBuffer, Rgba};

use crate::config::{require_field, TileConfig};
use crate::render::parse_hex_color;
use crate::tree::{generate_tree, TreeSettings, Vec3};

pub fn render_tree_tile(
    sprite_width: u32,
    sprite_height: u32,
    bg: Rgba<u8>,
    seed: u64,
    config: &TileConfig,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "tree" {
        return Err(format!("Unknown tile name: {}", config.name));
    }

    let settings = tree_settings_from_config(config)?;
    let model = generate_tree(seed, &settings);
    let trunk_color = parse_hex_color(&require_field(
        config.tree_trunk_color.clone(),
        "tree_trunk_color",
    )?)?;
    let leaf_color = parse_hex_color(&require_field(
        config.tree_leaf_color.clone(),
        "tree_leaf_color",
    )?)?;

    let mut tile = ImageBuffer::from_pixel(sprite_width, sprite_height, bg);
    let iso_scale = (sprite_width as f32 * 0.35) / settings.crown_radius.max(1.0);
    let height_scale = iso_scale;
    let center_x = sprite_width as f32 * 0.5;
    let base_y =
        (sprite_height.saturating_sub(1) as f32) - (sprite_width.saturating_sub(1) as f32) * 0.25;

    let project = |point: Vec3| -> (i32, i32) {
        let screen_x = (point.x - point.y) * iso_scale + center_x;
        let screen_y = (point.x + point.y) * iso_scale * 0.5 - point.z * height_scale + base_y;
        (screen_x.round() as i32, screen_y.round() as i32)
    };

    let mut segments = model.segments;
    segments.sort_by(|a, b| {
        let da = (a.start.x + a.start.y + a.start.z + a.end.x + a.end.y + a.end.z) * 0.5;
        let db = (b.start.x + b.start.y + b.start.z + b.end.x + b.end.y + b.end.z) * 0.5;
        da.partial_cmp(&db).unwrap_or(Ordering::Equal)
    });

    for segment in &segments {
        let (x0, y0) = project(segment.start);
        let (x1, y1) = project(segment.end);
        let radius = (segment.radius * iso_scale).round().max(1.0) as i32;
        draw_thick_line(&mut tile, x0, y0, x1, y1, radius, trunk_color);
    }

    let mut leaves = model.leaves;
    leaves.sort_by(|a, b| {
        let da = a.position.x + a.position.y + a.position.z;
        let db = b.position.x + b.position.y + b.position.z;
        da.partial_cmp(&db).unwrap_or(Ordering::Equal)
    });

    for leaf in &leaves {
        let (x, y) = project(leaf.position);
        let radius = (leaf.size * iso_scale).round().max(1.0) as i32;
        draw_filled_circle(&mut tile, x, y, radius, leaf_color);
    }

    Ok(tile)
}

fn tree_settings_from_config(config: &TileConfig) -> Result<TreeSettings, String> {
    Ok(TreeSettings {
        trunk_height: require_field(config.tree_trunk_height, "tree_trunk_height")?,
        crown_radius: require_field(config.tree_crown_radius, "tree_crown_radius")?,
        crown_height: require_field(config.tree_crown_height, "tree_crown_height")?,
        attraction_points: require_field(config.tree_attraction_points, "tree_attraction_points")?,
        segment_length: require_field(config.tree_segment_length, "tree_segment_length")?,
        influence_distance: require_field(
            config.tree_influence_distance,
            "tree_influence_distance",
        )?,
        kill_distance: require_field(config.tree_kill_distance, "tree_kill_distance")?,
        max_iterations: require_field(config.tree_max_iterations, "tree_max_iterations")?,
        base_radius: require_field(config.tree_base_radius, "tree_base_radius")?,
        leaf_size: require_field(config.tree_leaf_size, "tree_leaf_size")?,
        max_leaves: require_field(config.tree_leaf_count, "tree_leaf_count")?,
    })
}

fn draw_thick_line(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    radius: i32,
    color: Rgba<u8>,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        draw_filled_circle(img, x0, y0, radius, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

fn draw_filled_circle(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    cx: i32,
    cy: i32,
    radius: i32,
    color: Rgba<u8>,
) {
    let r = radius.max(1);
    let r2 = r * r;
    for y in -r..=r {
        for x in -r..=r {
            if x * x + y * y <= r2 {
                put_pixel_safe(img, cx + x, cy + y, color);
            }
        }
    }
}

fn put_pixel_safe(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    x: i32,
    y: i32,
    color: Rgba<u8>,
) {
    if x < 0 || y < 0 {
        return;
    }
    let (x, y) = (x as u32, y as u32);
    if x < img.width() && y < img.height() {
        img.put_pixel(x, y, color);
    }
}
