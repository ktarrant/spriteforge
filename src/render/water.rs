use image::{ImageBuffer, Rgba};

use crate::config::TileConfig;
use crate::render::util::{draw_isometric_ground, parse_hex_color};

pub fn render_water_tile(
    size: u32,
    bg: Rgba<u8>,
    config: &TileConfig,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "water" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    let mut img = ImageBuffer::from_pixel(size, size, bg);
    let water = parse_hex_color(
        &config
            .water_base
            .clone()
            .unwrap_or_else(|| "#2a4f7a".to_string()),
    )?;
    draw_isometric_ground(&mut img, size, water);
    Ok(img)
}

pub fn render_water_transition_tile(
    size: u32,
    bg: Rgba<u8>,
    config: &TileConfig,
    angles_override: Option<&Vec<f32>>,
    overrides: Option<&crate::config::TransitionOverrides>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "water_transition" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    let water = parse_hex_color(
        &config
            .water_base
            .clone()
            .unwrap_or_else(|| "#2a4f7a".to_string()),
    )?;
    let angles = angles_override
        .cloned()
        .or_else(|| config.transition_angles.clone())
        .or_else(|| config.transition_angle.map(|angle| vec![angle]))
        .unwrap_or_else(|| vec![333.435]);
    let mut cutoff = config.water_edge_cutoff.unwrap_or(0.78).clamp(0.0, 1.0);
    if let Some(overrides) = overrides {
        if let Some(override_cutoff) = overrides.water_edge_cutoff {
            cutoff = override_cutoff.clamp(0.0, 1.0);
        }
    }

    let mut img = ImageBuffer::from_pixel(size, size, bg);
    draw_isometric_ground(&mut img, size, water);
    for angle in &angles {
        apply_edge_cutout(&mut img, *angle, cutoff);
    }
    Ok(img)
}

pub fn render_water_mask_tile(size: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut tile = ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(&mut tile, size, Rgba([255, 255, 255, 255]));
    tile
}

pub fn render_water_transition_mask_tile(
    size: u32,
    config: &TileConfig,
    angles_override: Option<&Vec<f32>>,
    overrides: Option<&crate::config::TransitionOverrides>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "water_transition" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    let angles = angles_override
        .cloned()
        .or_else(|| config.transition_angles.clone())
        .or_else(|| config.transition_angle.map(|angle| vec![angle]))
        .unwrap_or_else(|| vec![333.435]);
    let mut cutoff = config.water_edge_cutoff.unwrap_or(0.78).clamp(0.0, 1.0);
    if let Some(overrides) = overrides {
        if let Some(override_cutoff) = overrides.water_edge_cutoff {
            cutoff = override_cutoff.clamp(0.0, 1.0);
        }
    }
    let mut tile = ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(&mut tile, size, Rgba([255, 255, 255, 255]));
    for angle in &angles {
        apply_edge_cutout(&mut tile, *angle, cutoff);
    }
    Ok(tile)
}

fn apply_edge_cutout(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    angle: f32,
    cutoff: f32,
) {
    let w = img.width().max(1) as f32;
    let h = img.height().max(1) as f32;
    let cutoff_a = |xf: f32| 0.75 - (1.0 - xf - cutoff) * 0.5;
    let cutoff_b = |xf: f32| 0.75 - (xf - cutoff) * 0.5;
    let cutoff_c = |xf: f32| 0.75 + (xf - cutoff) * 0.5;
    let cutoff_d = |xf: f32| 0.75 + (1.0 - xf - cutoff) * 0.5;
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        if pixel.0[3] == 0 {
            continue;
        }
        let xf = x as f32 / w;
        let yf = y as f32 / h;

        let should_cutout = if (angle - 26.5).abs() < 0.01 {
            yf <= cutoff_a(xf)
        } else if (angle - 153.435).abs() < 0.01 {
            yf <= cutoff_b(xf)
        } else if (angle - 206.565).abs() < 0.01 {
            yf >= cutoff_c(xf)
        } else if (angle - 333.435).abs() < 0.01 {
            yf >= cutoff_d(xf)
        } else if angle.abs() < 0.01 {
            xf >= 1.0 - cutoff
        } else if (angle - 90.0).abs() < 0.01 {
            yf <= 0.5 + cutoff / 2.0
        } else if (angle - 180.0).abs() < 0.01 {
            xf <= cutoff
        } else if (angle - 270.0).abs() < 0.01 {
            yf >= 1.0 - cutoff / 2.0
        } else {
            false
        };

        if should_cutout {
            *pixel = Rgba([0, 0, 0, 0]);
        }
    }
}
