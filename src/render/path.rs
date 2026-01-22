use image::{ImageBuffer, Rgba};

use crate::config::TileConfig;
use crate::render::util::{draw_isometric_ground, parse_hex_color};
use spriteforge_assets::{EDGE_N, EDGE_E, EDGE_W, EDGE_S, uv_from_xy};

// const BR_IND_MASK: u8 = 0x03;
// const BR_ROW_SHIFT: u8 = 0;
// const BR_COL_SHIFT: u8 = 2;
// const BR_EDGE_MASK: u8 = 0x0F;
// const BR_EDGE_ROW_SHIFT: u8 = 4;
// const BR_EDGE_COL_SHIFT: u8 = 8;

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

    let mut cutoff_rows: u32 = config.path_edge_cutoff.unwrap_or(0);
    if let Some(overrides) = overrides {
        if let Some(override_cutoff) = overrides.path_edge_cutoff {
            cutoff_rows = override_cutoff;
        }
    }

    let mut img = ImageBuffer::from_pixel(size, size, bg);
    draw_isometric_ground(&mut img, size, path);

    // Apply path edge transitions
    let w = img.width().max(1) as f32;
    let h = img.height().max(1) as f32;
    let brick_count: u8 = config.path_brick_count.unwrap_or(8).max(1) as u8;
    let brick_row_width: f32 = 1.0 / brick_count as f32;
    let brick_col_width: f32 = 1.0 / brick_count as f32;
    let brick_crack = config.path_brick_crack.unwrap_or(0.10).clamp(0.0, 0.5);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        if pixel.0[3] == 0 {
            continue;
        }
        let xf = x as f32 / w;
        let yf = y as f32 / h;
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
        // let edge_rows = 0.max(brick_row + 1 - brick_count.saturating_sub(cutoff_rows))
        //                         .max(cutoff_rows - brick_row);
        // let edge_cols = 0.max(brick_row + 1 - brick_count.saturating_sub(cutoff_rows))
        //                         .max(cutoff_rows - brick_row);
        // // Encode the brick's ID using rowi + coli + edge flags
        // let brick_id: u8 = (brick_rowi & BR_IND_MASK) << BR_ROW_SHIFT |
        //                     (brick_coli & BR_IND_MASK) << BR_COL_SHIFT |
        //                     (edge_rows & BR_EDGE_MASK) << BR_EDGE_ROW_SHIFT |
        //                     (edge_cols & BR_EDGE_MASK) << BR_EDGE_COL_SHIFT;
        // let allowed_ns: [u8; 1] = [
        //     (0x01 << BR_EDGE_ROW_SHIFT) | (0x01 << BR_ROW_SHIFT),
        // ];

        let alpha_u8: u8 = 255;
        if (transition_mask & EDGE_N != 0) && (brick_row >= brick_count - 1) && (brick_coli != 1) {
            *pixel = Rgba([0, 0, 0, 0]);
        } else if (transition_mask & EDGE_S != 0) && (brick_row == 0) && (brick_coli != 3) {
            *pixel = Rgba([0, 0, 0, 0]);
        } else if (transition_mask & EDGE_E != 0) && (brick_col >= brick_count - 1) && (brick_rowi != 2) {
            *pixel = Rgba([0, 0, 0, 0]);
        } else if (transition_mask & EDGE_W != 0) && (brick_col == 0) && (brick_rowi != 1) {
            *pixel = Rgba([0, 0, 0, 0]);
        } else if brick_u.fract() < brick_crack && (brick_rowi != 2) {
            *pixel = Rgba([0, 0, 0, alpha_u8]);
        } else if brick_v.fract() < brick_crack && (brick_rowi != 0) {
            *pixel = Rgba([0, 0, 0, alpha_u8]);
        }
    }
    Ok(img)
}
