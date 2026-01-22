use image::{ImageBuffer, Rgba};

use crate::config::TileConfig;
use crate::render::util::{draw_isometric_ground, parse_hex_color};
use spriteforge_assets::{edge_weight_for_mask, uv_from_xy};

pub fn render_path_tile(
    size: u32,
    bg: Rgba<u8>,
    config: &TileConfig,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "path" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    render_path_tile_with_mask(size, bg, config, 0, None)
}

pub fn render_path_transition_tile(
    size: u32,
    bg: Rgba<u8>,
    config: &TileConfig,
    transition_mask: u8,
    overrides: Option<&crate::config::TransitionOverrides>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "path_transition" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    render_path_tile_with_mask(size, bg, config, transition_mask, overrides)
}

fn render_path_tile_with_mask(
    size: u32,
    bg: Rgba<u8>,
    config: &TileConfig,
    transition_mask: u8,
    overrides: Option<&crate::config::TransitionOverrides>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    let path = parse_hex_color(
        &config
            .path_base
            .clone()
            .unwrap_or_else(|| "#6b6b6b".to_string()),
    )?;
    let mask = transition_mask;
    let mut cutoff = config.path_edge_cutoff.unwrap_or(0.2).clamp(0.0, 1.0);
    if let Some(overrides) = overrides {
        if let Some(override_cutoff) = overrides.path_edge_cutoff {
            cutoff = override_cutoff.clamp(0.0, 1.0);
        }
    }

    let mut img = ImageBuffer::from_pixel(size, size, bg);
    draw_isometric_ground(&mut img, size, path);

    // Apply path edge transitions
    let w = img.width().max(1) as f32;
    let h = img.height().max(1) as f32;
    let brick_rows = 8;
    let brick_cols = 8; // Should be a multiple of 8
    let brick_row_width: f32 = 1.0 / brick_rows as f32;
    let brick_col_width: f32 = 1.0 / brick_cols as f32;
    let brick_crack = 0.05;
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        if pixel.0[3] == 0 {
            continue;
        }
        let xf = x as f32 / w;
        let yf = y as f32 / h;
        let (u, v) = uv_from_xy(xf, yf);
        if u < 0.0 || v < 0.0 {
            continue;
        }
        let brick_u = u / brick_row_width as f32;
        let brick_v = v / brick_col_width as f32;
        let brick_col: u8 = brick_u as u8;
        let brick_row: u8 = (brick_v as u8 + brick_col) % 4;
        let [r, g, b, _] = pixel.0;
        let alpha_u8: u8 = (edge_weight_for_mask(mask, xf, yf, cutoff, 0.0) * 255.0).round() as u8;
        if brick_u.fract() < brick_crack && brick_row != 2 {
            *pixel = Rgba([0, 0, 0, alpha_u8]);
        } else if brick_v.fract() < brick_crack && brick_row != 0 {
            *pixel = Rgba([0, 0, 0, alpha_u8]);
        } else {
            *pixel = Rgba([r, g, b, alpha_u8]);
        }
    }
    Ok(img)
}
