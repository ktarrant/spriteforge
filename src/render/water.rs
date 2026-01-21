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
    transition_mask: u8,
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
    let mask = transition_mask;
    let mut cutoff = config.water_edge_cutoff.unwrap_or(0.2).clamp(0.0, 1.0);
    if let Some(overrides) = overrides {
        if let Some(override_cutoff) = overrides.water_edge_cutoff {
            cutoff = override_cutoff.clamp(0.0, 1.0);
        }
    }

    let mut img = ImageBuffer::from_pixel(size, size, bg);
    draw_isometric_ground(&mut img, size, water);
    let gradient = 0.0;
    apply_edge_cutout(&mut img, mask, cutoff, gradient);
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
    transition_mask: u8,
    overrides: Option<&crate::config::TransitionOverrides>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "water_transition" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    let mask = transition_mask;
    let mut cutoff = config.water_edge_cutoff.unwrap_or(0.2).clamp(0.0, 1.0);
    if let Some(overrides) = overrides {
        if let Some(override_cutoff) = overrides.water_edge_cutoff {
            cutoff = override_cutoff.clamp(0.0, 1.0);
        }
    }
    let mut tile = ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(&mut tile, size, Rgba([255, 255, 255, 255]));
    let gradient = 0.2;
    apply_edge_cutout(&mut tile, mask, cutoff, gradient);
    Ok(tile)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn apply_edge_cutout(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    mask: u8,
    cutoff: f32,
    gradient: f32,
) {
    let w = img.width().max(1) as f32;
    let h = img.height().max(1) as f32;
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        if pixel.0[3] == 0 {
            continue;
        }
        let xf = x as f32 / w;
        let yf = y as f32 / h;
        
        let mut alpha: f32 = 1.0;
        if mask & crate::render::transition::EDGE_N != 0 {
            // Line is written as y = 0.75 - (1.0 - x - cutoff) * 0.5
            let border: f32 = 0.75 - (1.0 - xf - cutoff) * 0.5;
            let m: f32 = 0.5;
            let denom: f32 = (m*m + 1.0).sqrt();      // sqrt(1.25) ~= 1.1180
            let d: f32 = (border - yf) / denom;       // >0 above line, <0 below line
            if gradient > 0.0 {
                alpha *= smoothstep(0.0, -gradient, d);
            }
            if d > 0.0 {
                alpha = 0.0;
            }
        }

        if mask & crate::render::transition::EDGE_W != 0 {
            // Line is written as y = 0.75 - (x - cutoff) * 0.5
            let border: f32 = 0.75 - (xf - cutoff) * 0.5;
            let m: f32 = 0.5;
            let denom: f32 = (m*m + 1.0).sqrt();      // sqrt(1.25) ~= 1.1180
            let d: f32 = (border - yf) / denom;       // >0 above line, <0 below line
            if gradient > 0.0 {
                alpha *= smoothstep(0.0, -gradient, d);
            }
            if d > 0.0 {
                alpha = 0.0;
            }
        }

        if mask & crate::render::transition::EDGE_S != 0 {
            // Line is written as y = 0.75 + (x - cutoff) * 0.5
            let border: f32 = 0.75 + (xf - cutoff) * 0.5;
            let m: f32 = 0.5;
            let denom: f32 = (m*m + 1.0).sqrt();      // sqrt(1.25) ~= 1.1180
            let d: f32 = (border - yf) / denom;       // >0 above line, <0 below line
            if gradient > 0.0 {
                alpha *= smoothstep(0.0, gradient, d);
            }
            if d < 0.0 {
                alpha = 0.0;
            }
        }

        if mask & crate::render::transition::EDGE_E != 0 {
            // Line is written as y = 0.75 + (1.0 - x - cutoff) * 0.5
            let border: f32 = 0.75 + (1.0 - xf - cutoff) * 0.5;
            let m: f32 = 0.5;
            let denom: f32 = (m*m + 1.0).sqrt();      // sqrt(1.25) ~= 1.1180
            let d: f32 = (border - yf) / denom;       // >0 above line, <0 below line
            if gradient > 0.0 {
                alpha *= smoothstep(0.0, gradient, d);
            }
            if d < 0.0 {
                alpha = 0.0;
            }
        }

        if mask & crate::render::transition::CORNER_NE != 0 {
            let cx = 1.0 - cutoff * 0.25;
            let cy = 0.75;
            let dx = xf - cx;
            let dy = yf - cy;
            let radius = cutoff * 0.4;
            let d = (dx * dx + dy * dy).sqrt();
            if xf > cx {
                alpha = 0.0;
            } else if gradient > 0.0 {
                alpha *= smoothstep(radius, radius + gradient, d);
            }
            if d < radius {
                alpha = 0.0;
            }
        }

        if mask & crate::render::transition::CORNER_NW != 0 {
            let cx = 0.5;
            let cy = 0.5 - cutoff * 0.6;
            let dx = xf - cx;
            let dy = yf - cy;
            let radius = cutoff;
            let d = (dx * dx + dy * dy).sqrt();
            if yf < cy {
                alpha = 0.0;
            } else if gradient > 0.0 {
                alpha *= smoothstep(radius, radius + gradient, d);
            }
            if d < radius {
                alpha = 0.0;
            }
        }

        if mask & crate::render::transition::CORNER_SW != 0 {
            let cx = cutoff * 0.25;
            let cy = 0.75;
            let dx = xf - cx;
            let dy = yf - cy;
            let radius = cutoff * 0.4;
            let d = (dx * dx + dy * dy).sqrt();
            if xf < cx {
                alpha = 0.0;
            } else if gradient > 0.0 {
                alpha *= smoothstep(radius, radius + gradient, d);
            }
            if d < radius {
                alpha = 0.0;
            }
        }

        if mask & crate::render::transition::CORNER_SE != 0 {
            let cx = 0.5;
            let cy = 1.0 + cutoff * 0.6;
            let dx = xf - cx;
            let dy = yf - cy;
            let radius = cutoff;
            let d = (dx * dx + dy * dy).sqrt();
            if yf > cy {
                alpha = 0.0;
            } else if gradient > 0.0 {
                alpha *= smoothstep(radius, radius + gradient, d);
            }
            if d < radius {
                alpha = 0.0;
            }
        }

        if (mask & crate::render::transition::EDGE_E != 0)
            && (mask & crate::render::transition::EDGE_N != 0)
        {
            let cx = 1.0 - cutoff * 2.0;
            let cy = 0.75;
            let dx = xf - cx;
            let dy = yf - cy;
            let radius = cutoff * 0.5;
            if (dx * dx + dy * dy >= radius * radius) && (xf > cx) {
                alpha = 0.0;
            }
        }
        
        if (mask & crate::render::transition::EDGE_S != 0)
            && (mask & crate::render::transition::EDGE_W != 0)
        {
            let cx = cutoff * 2.0;
            let cy = 0.75;
            let dx = xf - cx;
            let dy = yf - cy;
            let radius = cutoff * 0.5;
            if (dx * dx + dy * dy >= radius * radius) && (xf < cx) {
                alpha = 0.0;
            }
        }

        if (mask & crate::render::transition::EDGE_W != 0)
            && (mask & crate::render::transition::EDGE_N != 0)
        {
            let cx = 0.5;
            let cy = 0.5 + cutoff * 4.8;
            let dx = xf - cx;
            let dy = yf - cy;
            let radius = cutoff * 4.0;
            if dx * dx + dy * dy >= radius * radius {
                alpha = 0.0;
            }
        }

        if (mask & crate::render::transition::EDGE_S != 0)
            && (mask & crate::render::transition::EDGE_E != 0)
        {
            let cx = 0.5;
            let cy = 1.0 - cutoff * 4.8;
            let dx = xf - cx;
            let dy = yf - cy;
            let radius = cutoff * 4.0;
            if dx * dx + dy * dy >= radius * radius {
                alpha = 0.0;
            }
        }

        let [r, g, b, _] = pixel.0;
        let alpha_u8 = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
        *pixel = Rgba([r, g, b, alpha_u8]);
    }
}
