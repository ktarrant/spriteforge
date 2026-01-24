use image::{ImageBuffer, Rgba};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::config::{require_field, TileConfig};
use crate::render::util::{blit, draw_isometric_ground, parse_hex_color, random_tile_point};

pub fn render_dirt_tile(
    sprite_width: u32,
    sprite_height: u32,
    bg: Rgba<u8>,
    seed: u64,
    config: &TileConfig,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "dirt" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    let mut rng = StdRng::seed_from_u64(seed);
    let palette = dirt_palette(config)?;
    let mut img = ImageBuffer::from_pixel(sprite_width, sprite_height, bg);
    let mut base = ImageBuffer::from_pixel(sprite_width, sprite_height, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(&mut base, sprite_width, sprite_height, palette[0]);
    blit(&mut img, &base);

    let splotches = require_field(config.dirt_splotch_count, "dirt_splotch_count")?;
    for _ in 0..splotches {
        let (cx, cy) = random_tile_point(&base, &mut rng);
        let radius = rng.gen_range(3..=8);
        let shade = if rng.gen_bool(0.5) { palette[1] } else { palette[2] };
        draw_oval(&mut img, &base, cx, cy, radius * 2, radius, shade);
    }

    let stones = require_field(config.dirt_stone_count, "dirt_stone_count")?;
    for _ in 0..stones {
        let (cx, cy) = random_tile_point(&base, &mut rng);
        let radius = rng.gen_range(1..=3);
        let shade = if rng.gen_bool(0.5) { palette[3] } else { palette[4] };
        if rng.gen_bool(0.5) {
            draw_blob(&mut img, &base, cx, cy, radius, shade);
        } else {
            draw_triangle(&mut img, &base, cx, cy, radius, shade);
        }
    }

    Ok(img)
}

fn dirt_palette(config: &TileConfig) -> Result<[Rgba<u8>; 5], String> {
    let base_hex = require_field(config.dirt_base.clone(), "dirt_base")?;
    let splotch_hexes = require_field(config.dirt_splotches.clone(), "dirt_splotches")?;
    let stone_hexes = require_field(config.dirt_stones.clone(), "dirt_stones")?;
    Ok([
        parse_hex_color(&base_hex)?,
        parse_hex_color(&splotch_hexes[0])?,
        parse_hex_color(&splotch_hexes[1])?,
        parse_hex_color(&stone_hexes[0])?,
        parse_hex_color(&stone_hexes[1])?,
    ])
}

fn draw_blob(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    mask: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    cx: i32,
    cy: i32,
    radius: i32,
    color: Rgba<u8>,
) {
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let x = cx + dx;
            let y = cy + dy;
            if x < 0 || y < 0 {
                continue;
            }
            let (ux, uy) = (x as u32, y as u32);
            if ux >= mask.width() || uy >= mask.height() {
                continue;
            }
            if mask.get_pixel(ux, uy).0[3] > 0 {
                img.put_pixel(ux, uy, color);
            }
        }
    }
}

fn draw_triangle(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    mask: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    cx: i32,
    cy: i32,
    size: i32,
    color: Rgba<u8>,
) {
    if size <= 0 {
        return;
    }
    let p1 = (cx, cy - size);
    let p2 = (cx - size, cy + size);
    let p3 = (cx + size, cy + size);
    let min_x = p2.0.min(p3.0).min(p1.0);
    let max_x = p2.0.max(p3.0).max(p1.0);
    let min_y = p1.1.min(p2.1).min(p3.1);
    let max_y = p1.1.max(p2.1).max(p3.1);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if !point_in_triangle((x, y), p1, p2, p3) {
                continue;
            }
            if x < 0 || y < 0 {
                continue;
            }
            let (ux, uy) = (x as u32, y as u32);
            if ux >= mask.width() || uy >= mask.height() {
                continue;
            }
            if mask.get_pixel(ux, uy).0[3] > 0 {
                img.put_pixel(ux, uy, color);
            }
        }
    }
}

fn point_in_triangle(p: (i32, i32), a: (i32, i32), b: (i32, i32), c: (i32, i32)) -> bool {
    let d1 = sign(p, a, b);
    let d2 = sign(p, b, c);
    let d3 = sign(p, c, a);
    let has_neg = d1 < 0 || d2 < 0 || d3 < 0;
    let has_pos = d1 > 0 || d2 > 0 || d3 > 0;
    !(has_neg && has_pos)
}

fn sign(p1: (i32, i32), p2: (i32, i32), p3: (i32, i32)) -> i64 {
    (p1.0 - p3.0) as i64 * (p2.1 - p3.1) as i64
        - (p2.0 - p3.0) as i64 * (p1.1 - p3.1) as i64
}

fn draw_oval(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    mask: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    cx: i32,
    cy: i32,
    rx: i32,
    ry: i32,
    color: Rgba<u8>,
) {
    if rx <= 0 || ry <= 0 {
        return;
    }
    let rx2 = rx * rx;
    let ry2 = ry * ry;
    for dy in -ry..=ry {
        for dx in -rx..=rx {
            let lhs = dx * dx * ry2 + dy * dy * rx2;
            let rhs = rx2 * ry2;
            if lhs > rhs {
                continue;
            }
            let x = cx + dx;
            let y = cy + dy;
            if x < 0 || y < 0 {
                continue;
            }
            let (ux, uy) = (x as u32, y as u32);
            if ux >= mask.width() || uy >= mask.height() {
                continue;
            }
            if mask.get_pixel(ux, uy).0[3] > 0 {
                let existing = *img.get_pixel(ux, uy);
                if existing == color {
                    img.put_pixel(ux, uy, darken_color(color, 24));
                } else {
                    img.put_pixel(ux, uy, color);
                }
            }
        }
    }
}

fn darken_color(color: Rgba<u8>, amount: u8) -> Rgba<u8> {
    let [r, g, b, a] = color.0;
    Rgba([
        r.saturating_sub(amount),
        g.saturating_sub(amount),
        b.saturating_sub(amount),
        a,
    ])
}
