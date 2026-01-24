use std::cmp::Ordering;

use image::{ImageBuffer, Rgba};

use crate::config::{require_field, TileConfig};
use crate::render::parse_hex_color;
use crate::tree::{generate_tree, TreeModel, TreeSettings, Vec3};

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
    let projection = build_projection(&model, sprite_width, sprite_height);
    let project = |point: Vec3| -> (i32, i32) {
        let (screen_x, screen_y) = projection.project(point);
        (screen_x.round() as i32, screen_y.round() as i32)
    };

    let mut draw_items = Vec::with_capacity(model.segments.len() + model.leaves.len());
    for segment in &model.segments {
        let depth = (segment.start.x + segment.start.y + segment.end.x + segment.end.y) * 0.5;
        draw_items.push(DrawItem::Segment {
            depth,
            start: segment.start,
            end: segment.end,
            radius: segment.radius,
        });
    }
    for stem in &model.leaf_stems {
        let depth = (stem.start.x + stem.start.y + stem.end.x + stem.end.y) * 0.5;
        draw_items.push(DrawItem::LeafStem {
            depth,
            start: stem.start,
            end: stem.end,
            radius: stem.radius,
        });
    }
    for leaf in &model.leaves {
        let depth = leaf.position.x + leaf.position.y;
        draw_items.push(DrawItem::Leaf {
            depth,
            position: leaf.position,
            radius: leaf.size,
        });
    }

    draw_items.sort_by(|a, b| a.depth().partial_cmp(&b.depth()).unwrap_or(Ordering::Equal));
    for item in draw_items {
        match item {
            DrawItem::Segment {
                start,
                end,
                radius,
                ..
            } => {
                let (x0, y0) = project(start);
                let (x1, y1) = project(end);
                let radius = (radius * projection.scale).round().max(1.0) as i32;
                draw_thick_line(&mut tile, x0, y0, x1, y1, radius, trunk_color);
            }
            DrawItem::Leaf {
                position,
                radius,
                ..
            } => {
                let (x, y) = project(position);
                let rx = (radius * projection.scale).round().max(1.0) as i32;
                let ry = (radius * projection.scale * 0.7).round().max(1.0) as i32;
                draw_filled_oval(&mut tile, x, y, rx, ry, leaf_color);
            }
            DrawItem::LeafStem {
                start,
                end,
                radius,
                ..
            } => {
                let (x0, y0) = project(start);
                let (x1, y1) = project(end);
                let radius = (radius * projection.scale).round().max(1.0) as i32;
                draw_thick_line(&mut tile, x0, y0, x1, y1, radius, trunk_color);
            }
        }
    }

    Ok(tile)
}

pub fn render_tree_mask_tile(
    sprite_width: u32,
    sprite_height: u32,
    seed: u64,
    config: &TileConfig,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "tree" {
        return Err(format!("Unknown tile name: {}", config.name));
    }

    let settings = tree_settings_from_config(config)?;
    let model = generate_tree(seed, &settings);
    let projection = build_projection(&model, sprite_width, sprite_height);

    let mut mask = ImageBuffer::from_pixel(sprite_width, sprite_height, Rgba([0, 0, 0, 0]));
    let mut depth = vec![f32::NEG_INFINITY; (sprite_width * sprite_height) as usize];

    for segment in &model.segments {
        let dir = Vec3::new(
            segment.end.x - segment.start.x,
            segment.end.y - segment.start.y,
            segment.end.z - segment.start.z,
        );
        let length = dir.length().max(0.001);
        let step = (segment.radius * 0.75).max(0.05);
        let steps = (length / step).ceil().max(1.0) as i32;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let point = Vec3::new(
                segment.start.x + dir.x * t,
                segment.start.y + dir.y * t,
                segment.start.z + dir.z * t,
            );
            let depth_value = point.x + point.y;
            rasterize_normal_sphere(
                &projection,
                &mut mask,
                &mut depth,
                sprite_width,
                sprite_height,
                point,
                segment.radius,
                depth_value,
                segment.normal,
            );
        }
    }

    for stem in &model.leaf_stems {
        let dir = Vec3::new(
            stem.end.x - stem.start.x,
            stem.end.y - stem.start.y,
            stem.end.z - stem.start.z,
        );
        let length = dir.length().max(0.001);
        let step = (stem.radius * 0.75).max(0.05);
        let steps = (length / step).ceil().max(1.0) as i32;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let point = Vec3::new(
                stem.start.x + dir.x * t,
                stem.start.y + dir.y * t,
                stem.start.z + dir.z * t,
            );
            let depth_value = point.x + point.y;
            rasterize_normal_sphere(
                &projection,
                &mut mask,
                &mut depth,
                sprite_width,
                sprite_height,
                point,
                stem.radius,
                depth_value,
                stem.normal,
            );
        }
    }

    for leaf in &model.leaves {
        let depth_value = leaf.position.x + leaf.position.y;
        rasterize_normal_sphere(
            &projection,
            &mut mask,
            &mut depth,
            sprite_width,
            sprite_height,
            leaf.position,
            leaf.size,
            depth_value,
            leaf.normal,
        );
    }

    Ok(mask)
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

struct Projection {
    scale: f32,
    offset_x: f32,
    offset_y: f32,
}

enum DrawItem {
    Segment {
        depth: f32,
        start: Vec3,
        end: Vec3,
        radius: f32,
    },
    Leaf {
        depth: f32,
        position: Vec3,
        radius: f32,
    },
    LeafStem {
        depth: f32,
        start: Vec3,
        end: Vec3,
        radius: f32,
    },
}

impl DrawItem {
    fn depth(&self) -> f32 {
        match self {
            DrawItem::Segment { depth, .. } => *depth,
            DrawItem::Leaf { depth, .. } => *depth,
            DrawItem::LeafStem { depth, .. } => *depth,
        }
    }
}

impl Projection {
    fn project(&self, point: Vec3) -> (f32, f32) {
        let (raw_x, raw_y) = project_raw(point);
        (
            raw_x * self.scale + self.offset_x,
            raw_y * self.scale + self.offset_y,
        )
    }
}

fn build_projection(model: &TreeModel, sprite_width: u32, sprite_height: u32) -> Projection {
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;

    for segment in &model.segments {
        expand_bounds(
            segment.start,
            segment.radius,
            &mut min_x,
            &mut max_x,
            &mut min_y,
            &mut max_y,
        );
        expand_bounds(
            segment.end,
            segment.radius,
            &mut min_x,
            &mut max_x,
            &mut min_y,
            &mut max_y,
        );
    }

    for leaf in &model.leaves {
        expand_bounds(
            leaf.position,
            leaf.size,
            &mut min_x,
            &mut max_x,
            &mut min_y,
            &mut max_y,
        );
    }
    for stem in &model.leaf_stems {
        expand_bounds(
            stem.start,
            stem.radius,
            &mut min_x,
            &mut max_x,
            &mut min_y,
            &mut max_y,
        );
        expand_bounds(
            stem.end,
            stem.radius,
            &mut min_x,
            &mut max_x,
            &mut min_y,
            &mut max_y,
        );
    }

    if min_x == f32::MAX || min_y == f32::MAX {
        min_x = -1.0;
        max_x = 1.0;
        min_y = -1.0;
        max_y = 1.0;
    }

    let width = (max_x - min_x).max(0.001);
    let height = (max_y - min_y).max(0.001);
    let width_f = sprite_width.saturating_sub(1).max(1) as f32;
    let height_f = sprite_height.saturating_sub(1).max(1) as f32;
    let pad_x = width_f * 0.05;
    let pad_top = height_f * 0.05;
    let pad_bottom = width_f * 0.05;
    let available_w = (width_f - pad_x * 2.0).max(1.0);
    let available_h = (height_f - (pad_top + pad_bottom)).max(1.0);
    let scale = (available_w / width).min(available_h / height);

    let offset_x = width_f * 0.5 - (min_x + max_x) * 0.5 * scale;
    let offset_y = (height_f - pad_bottom) - max_y * scale;

    Projection {
        scale,
        offset_x,
        offset_y,
    }
}

fn expand_bounds(
    point: Vec3,
    radius: f32,
    min_x: &mut f32,
    max_x: &mut f32,
    min_y: &mut f32,
    max_y: &mut f32,
) {
    let (raw_x, raw_y) = project_raw(point);
    let r = radius.max(0.0);
    *min_x = min_x.min(raw_x - r);
    *max_x = max_x.max(raw_x + r);
    *min_y = min_y.min(raw_y - r);
    *max_y = max_y.max(raw_y + r);
}

fn project_raw(point: Vec3) -> (f32, f32) {
    let screen_x = point.x - point.y;
    let screen_y = (point.x + point.y) * 0.5 - point.z;
    (screen_x, screen_y)
}

fn rasterize_normal_sphere(
    projection: &Projection,
    mask: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    depth: &mut [f32],
    sprite_width: u32,
    sprite_height: u32,
    center: Vec3,
    radius: f32,
    depth_value: f32,
    normal: Vec3,
) {
    if radius <= 0.0 {
        return;
    }
    let (cx, cy) = projection.project(center);
    let screen_radius = radius * projection.scale;
    if screen_radius <= 0.0 {
        return;
    }

    let min_x = (cx - screen_radius).floor().max(0.0) as i32;
    let max_x = (cx + screen_radius).ceil().min(sprite_width.saturating_sub(1) as f32) as i32;
    let min_y = (cy - screen_radius).floor().max(0.0) as i32;
    let max_y = (cy + screen_radius).ceil().min(sprite_height.saturating_sub(1) as f32) as i32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx_screen = x as f32 - cx;
            let dy_screen = y as f32 - cy;
            if dx_screen * dx_screen + dy_screen * dy_screen > screen_radius * screen_radius {
                continue;
            }
            let idx = (y as u32 * sprite_width + x as u32) as usize;
            if depth_value <= depth[idx] {
                continue;
            }
            depth[idx] = depth_value;
            mask.put_pixel(x as u32, y as u32, encode_normal(normal));
        }
    }
}

fn encode_normal(normal: Vec3) -> Rgba<u8> {
    let nx = (normal.x * 0.5 + 0.5).clamp(0.0, 1.0);
    let ny = (normal.y * 0.5 + 0.5).clamp(0.0, 1.0);
    let nz = (normal.z * 0.5 + 0.5).clamp(0.0, 1.0);
    Rgba([
        (nx * 255.0).round() as u8,
        (ny * 255.0).round() as u8,
        (nz * 255.0).round() as u8,
        255,
    ])
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

fn draw_filled_oval(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    cx: i32,
    cy: i32,
    rx: i32,
    ry: i32,
    color: Rgba<u8>,
) {
    let rx = rx.max(1);
    let ry = ry.max(1);
    let rx2 = (rx * rx) as f32;
    let ry2 = (ry * ry) as f32;
    for y in -ry..=ry {
        for x in -rx..=rx {
            let dx = x as f32;
            let dy = y as f32;
            if (dx * dx) / rx2 + (dy * dy) / ry2 <= 1.0 {
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
