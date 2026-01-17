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
    apply_edge_cutout(&mut img, &angles, cutoff);
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
    apply_edge_cutout(&mut tile, &angles, cutoff);
    Ok(tile)
}

fn apply_edge_cutout(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    angles: &[f32],
    cutoff: f32,
) {
    let w = img.width().max(1) as f32;
    let h = img.height().max(1) as f32;
    let cutoff_a = |xf: f32| 0.75 - (1.0 - xf - cutoff) * 0.5;
    let cutoff_b = |xf: f32| 0.75 - (xf - cutoff) * 0.5;
    let cutoff_c = |xf: f32| 0.75 + (xf - cutoff) * 0.5;
    let cutoff_d = |xf: f32| 0.75 + (1.0 - xf - cutoff) * 0.5;
    let has_angle = |target: f32| angles.iter().any(|angle| (*angle - target).abs() < 0.01);
    let angles_lookup = [
        has_angle(0.0),
        has_angle(26.5),
        has_angle(90.0),
        has_angle(153.435),
        has_angle(180.0),
        has_angle(206.565),
        has_angle(270.0),
        has_angle(333.435),
    ];
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        if pixel.0[3] == 0 {
            continue;
        }
        let xf = x as f32 / w;
        let yf = y as f32 / h;

        let should_cutout = (angles_lookup[1] && yf <= cutoff_a(xf))
            || (angles_lookup[3] && yf <= cutoff_b(xf))
            || (angles_lookup[5] && yf >= cutoff_c(xf))
            || (angles_lookup[7] && yf >= cutoff_d(xf))
            || (angles_lookup[0] && {
                    let cx = 1.0 - cutoff * 0.75;
                    let cy = 0.75;
                    let dx = xf - cx;
                    let dy = yf - cy;
                    let radius = cutoff * 0.5;
                    (dx * dx + dy * dy <= radius * radius) || (xf > cx)
                })
            || (angles_lookup[2] && {
                    let cx = 0.5;
                    let cy = 0.5 - cutoff * 2.0;
                    let dx = xf - cx;
                    let dy = yf - cy;
                    let radius = cutoff * 2.7;
                    (dx * dx + dy * dy <= radius * radius) || (yf < cy)
                })
            || (angles_lookup[4] && {
                    let cx = cutoff * 0.75;
                    let cy = 0.75;
                    let dx = xf - cx;
                    let dy = yf - cy;
                    let radius = cutoff * 0.5;
                    (dx * dx + dy * dy <= radius * radius) || (xf < cx)
                })
            || (angles_lookup[6] && {
                    let cx = 0.5;
                    let cy = 1.0 + cutoff * 2.0;
                    let dx = xf - cx;
                    let dy = yf - cy;
                    let radius = cutoff * 2.7;
                    (dx * dx + dy * dy <= radius * radius) || (yf > cy)
                })
            || (angles_lookup[1]
                && angles_lookup[7]
                && {
                    let cx = 1.0 - cutoff * 2.0;
                    let cy = 0.75;
                    let dx = xf - cx;
                    let dy = yf - cy;
                    let radius = cutoff * 0.5;
                    (dx * dx + dy * dy >= radius * radius) && (xf > cx)
                })
            || (angles_lookup[3]
                && angles_lookup[5]
                && {
                    let cx = cutoff * 2.0;
                    let cy = 0.75;
                    let dx = xf - cx;
                    let dy = yf - cy;
                    let radius = cutoff * 0.5;
                    (dx * dx + dy * dy >= radius * radius) && (xf < cx)
                })
            || (angles_lookup[1]
                && angles_lookup[3]
                && {
                    let cx = 0.5;
                    let cy = 0.5 + cutoff * 4.8;
                    let dx = xf - cx;
                    let dy = yf - cy;
                    let radius = cutoff * 4.0;
                    dx * dx + dy * dy >= radius * radius
                })
            || (angles_lookup[5]
                && angles_lookup[7]
                && {
                    let cx = 0.5;
                    let cy = 1.0 - cutoff * 4.8;
                    let dx = xf - cx;
                    let dy = yf - cy;
                    let radius = cutoff * 4.0;
                    dx * dx + dy * dy >= radius * radius
                });

        if should_cutout {
            *pixel = Rgba([0, 0, 0, 0]);
        }
    }
}
