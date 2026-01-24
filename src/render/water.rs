use image::{ImageBuffer, Rgba};

use crate::config::{require_field, TileConfig};
use crate::render::util::{draw_isometric_ground, parse_hex_color};
use spriteforge_assets::edge_weight_for_mask;

pub fn render_water_tile(
    sprite_width: u32,
    sprite_height: u32,
    bg: Rgba<u8>,
    config: &TileConfig,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "water" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    let mut img = ImageBuffer::from_pixel(sprite_width, sprite_height, bg);
    let water_base = require_field(config.water_base.clone(), "water_base")?;
    let water = parse_hex_color(&water_base)?;
    draw_isometric_ground(&mut img, sprite_width, sprite_height, water);
    Ok(img)
}

pub fn render_water_transition_tile(
    sprite_width: u32,
    sprite_height: u32,
    bg: Rgba<u8>,
    config: &TileConfig,
    transition_mask: u8,
    overrides: Option<&crate::config::TransitionOverrides>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "water_transition" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    let water_base = require_field(config.water_base.clone(), "water_base")?;
    let water = parse_hex_color(&water_base)?;
    let mask = transition_mask;
    let mut cutoff =
        require_field(config.water_edge_cutoff, "water_edge_cutoff")?.clamp(0.0, 1.0);
    if let Some(overrides) = overrides {
        if let Some(override_cutoff) = overrides.water_edge_cutoff {
            cutoff = override_cutoff.clamp(0.0, 1.0);
        }
    }

    let mut img = ImageBuffer::from_pixel(sprite_width, sprite_height, bg);
    draw_isometric_ground(&mut img, sprite_width, sprite_height, water);
    let gradient = 0.0;
    let width = img.width().max(1) as f32;
    let height = img.height().max(1) as f32;
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        if pixel.0[3] == 0 {
            continue;
        }
        let xf = x as f32 / width;
        let yf = y as f32 / width;
        let [r, g, b, _] = pixel.0;
        let alpha_u8 =
            (edge_weight_for_mask(mask, xf, yf, cutoff, gradient) * 255.0).round() as u8;
        *pixel = Rgba([r, g, b, alpha_u8]);
    }
    Ok(img)
}

pub fn render_water_mask_tile(
    sprite_width: u32,
    sprite_height: u32,
) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut tile = ImageBuffer::from_pixel(sprite_width, sprite_height, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(
        &mut tile,
        sprite_width,
        sprite_height,
        Rgba([255, 255, 255, 255]),
    );
    tile
}

pub fn render_water_transition_mask_tile(
    sprite_width: u32,
    sprite_height: u32,
    config: &TileConfig,
    transition_mask: u8,
    overrides: Option<&crate::config::TransitionOverrides>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "water_transition" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    let mask = transition_mask;
    let mut cutoff =
        require_field(config.water_edge_cutoff, "water_edge_cutoff")?.clamp(0.0, 1.0);
    if let Some(overrides) = overrides {
        if let Some(override_cutoff) = overrides.water_edge_cutoff {
            cutoff = override_cutoff.clamp(0.0, 1.0);
        }
    }
    let mut tile = ImageBuffer::from_pixel(sprite_width, sprite_height, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(
        &mut tile,
        sprite_width,
        sprite_height,
        Rgba([255, 255, 255, 255]),
    );

    // Apply water edge transitions
    let mut gradient =
        require_field(config.water_edge_gradient, "water_edge_gradient")?.max(0.0);
    if let Some(overrides) = overrides {
        if let Some(override_gradient) = overrides.water_edge_gradient {
            gradient = override_gradient.max(0.0);
        }
    }
    let width = tile.width().max(1) as f32;
    let height = tile.height().max(1) as f32;
    for (x, y, pixel) in tile.enumerate_pixels_mut() {
        if pixel.0[3] == 0 {
            continue;
        }
        let xf = x as f32 / width;
        let yf = y as f32 / width;
        let [r, g, b, _] = pixel.0;
        let alpha_u8 = (edge_weight_for_mask(mask, xf, yf, cutoff, gradient) * 255.0).round() as u8;
        *pixel = Rgba([r, g, b, alpha_u8]);
    }
    Ok(tile)
}
