use image::{ImageBuffer, Rgba};

use crate::config::TileConfig;
use crate::render::util::{apply_edge_cutout, draw_isometric_ground, parse_hex_color};

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
    let gradient = 0.0;
    apply_edge_cutout(&mut img, mask, cutoff, gradient);
    Ok(img)
}
