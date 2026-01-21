use image::{ImageBuffer, Rgba};

use crate::config::TileConfig;
use crate::render::util::{edge_weight_for_mask, draw_isometric_ground, parse_hex_color};

pub fn render_path_tile(
    size: u32,
    bg: Rgba<u8>,
    config: &TileConfig,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "path" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    let mut img = ImageBuffer::from_pixel(size, size, bg);
    let path = parse_hex_color(
        &config
            .path_base
            .clone()
            .unwrap_or_else(|| "#6b6b6b".to_string()),
    )?;
    draw_isometric_ground(&mut img, size, path);
    Ok(img)
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
    let gradient = 0.2;
    let w = img.width().max(1) as f32;
    let h = img.height().max(1) as f32;
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        if pixel.0[3] == 0 {
            continue;
        }
        let xf = x as f32 / w;
        let yf = y as f32 / h;
        let [r, g, b, _] = pixel.0;
        let alpha_u8 = (edge_weight_for_mask(mask, xf, yf, cutoff, gradient) * 255.0).round() as u8;
        *pixel = Rgba([r, g, b, alpha_u8]);
    }
    Ok(img)
}
