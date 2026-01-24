use image::{ImageBuffer, Rgba};

use crate::config::{require_field, TileConfig};
use crate::render::util::{draw_isometric_ground, parse_hex_color};
use spriteforge_assets::{EDGE_N, EDGE_E, EDGE_W, EDGE_S, uv_from_xy};

// const BR_IND_MASK: u8 = 0x03;
// const BR_ROW_SHIFT: u8 = 0;
// const BR_COL_SHIFT: u8 = 2;
// const BR_EDGE_MASK: u8 = 0x0F;
// const BR_EDGE_ROW_SHIFT: u8 = 4;
// const BR_EDGE_COL_SHIFT: u8 = 8;

pub fn render_path_tile(
    sprite_width: u32,
    sprite_height: u32,
    bg: Rgba<u8>,
    config: &TileConfig,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "path" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    render_path_tile_with_mask(sprite_width, sprite_height, bg, config, 0)
}

pub fn render_path_transition_tile(
    sprite_width: u32,
    sprite_height: u32,
    bg: Rgba<u8>,
    config: &TileConfig,
    transition_mask: u8,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "path_transition" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    render_path_tile_with_mask(sprite_width, sprite_height, bg, config, transition_mask)
}

fn render_path_tile_with_mask(
    sprite_width: u32,
    sprite_height: u32,
    bg: Rgba<u8>,
    config: &TileConfig,
    transition_mask: u8,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    let path_base = require_field(config.path_base.clone(), "path_base")?;
    let path = parse_hex_color(&path_base)?;

    let mut img = ImageBuffer::from_pixel(sprite_width, sprite_height, bg);
    draw_isometric_ground(&mut img, sprite_width, sprite_height, path);

    // Apply path edge transitions
    let width = img.width().max(1) as f32;
    let brick_count: u8 = require_field(config.path_brick_count, "path_brick_count")?
        .max(1) as u8;
    let brick_row_width: f32 = 1.0 / brick_count as f32;
    let brick_col_width: f32 = 1.0 / brick_count as f32;
    let brick_crack =
        require_field(config.path_brick_crack, "path_brick_crack")?.clamp(0.0, 0.5);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        if pixel.0[3] == 0 {
            continue;
        }
        let xf = x as f32 / width;
        let yf = y as f32 / width;
        let (u, v) = uv_from_xy(xf, yf);
        if u < 0.0 || v < 0.0 {
            pixel.0[3] = 0;
            continue;
        }
        let brick_u = u / brick_row_width as f32;
        let brick_v = v / brick_col_width as f32;
        let brick_col: u8 = brick_u as u8;
        let brick_row: u8 = brick_v as u8;
        let brick_coli: u8 = brick_col % 4;
        let brick_rowi: u8 = (brick_row + brick_col) % 4;
        const BRICK_A: u8 = 75;
        const BRICK_B: u8 = 90;
        const BRICK_C: u8 = 115;
        const BRICK_D: u8 = 130;

        let alpha_u8: u8 = 255;
        if (transition_mask & EDGE_N != 0) && (brick_row >= brick_count - 1) && (brick_coli != 1) {
            *pixel = Rgba([0, 0, 0, 0]);
        } else if (transition_mask & EDGE_S != 0) && (brick_row == 0) && (brick_coli != 3) {
            *pixel = Rgba([0, 0, 0, 0]);
        } else if (transition_mask & EDGE_E != 0) && (brick_col >= brick_count - 1) && (brick_rowi != 2) {
            *pixel = Rgba([0, 0, 0, 0]);
        } else if (transition_mask & EDGE_W != 0) && (brick_col == 0) && (brick_rowi != 1) {
            *pixel = Rgba([0, 0, 0, 0]);
        } else if (transition_mask & EDGE_N != 0) && (brick_row == brick_count - 2) && (brick_coli == 0 || brick_coli == 3) {
            *pixel = Rgba([0, 0, 0, 0]);
        } else if (transition_mask & EDGE_S != 0) && (brick_row == 1) && (brick_coli == 0 || brick_coli == 1) {
            *pixel = Rgba([0, 0, 0, 0]);
        } else if (transition_mask & EDGE_E != 0) && (brick_col >= brick_count - 2) && (brick_rowi == 0 || brick_rowi == 3) {
            *pixel = Rgba([0, 0, 0, 0]);
        } else if (transition_mask & EDGE_W != 0) && (brick_col == 1) && (brick_rowi == 0 || brick_rowi == 3) {
            *pixel = Rgba([0, 0, 0, 0]);
        } else if brick_u.fract() < brick_crack && (brick_rowi != 2) {
            *pixel = Rgba([0, 0, 0, alpha_u8]);
        } else if brick_v.fract() < brick_crack && (brick_rowi != 0) {
            *pixel = Rgba([0, 0, 0, alpha_u8]);
        } else if (brick_rowi == 0 || brick_rowi == 3) && brick_coli == 2 {
            *pixel = Rgba([BRICK_A, BRICK_A, BRICK_A, 255]);
        } else if (brick_rowi == 0 || brick_rowi == 3) && brick_coli == 1 {
            *pixel = Rgba([BRICK_B, BRICK_B, BRICK_B, 255])
        } else if brick_rowi == 1 && brick_coli == 0 {
            *pixel = Rgba([BRICK_C, BRICK_C, BRICK_C, 255])
        } else if brick_rowi == 2 && brick_coli == 1 {
            *pixel = Rgba([BRICK_C, BRICK_C, BRICK_C, 255])
        } else if brick_rowi == 2 && brick_coli == 0 {
            *pixel = Rgba([BRICK_D, BRICK_D, BRICK_D, 255])
        } else if brick_rowi == 1 && brick_coli == 3 {
            *pixel = Rgba([BRICK_D, BRICK_D, BRICK_D, 255])
        }
    }
    Ok(img)
}
