use image::{ImageBuffer, Rgba};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::config::{TileConfig, TransitionOverrides};
use crate::render::grass::{default_blade_max, grass_palette};
use crate::render::util::{draw_isometric_ground, edge_weight_for_angles};

pub fn render_grass_transition_tile(
    size: u32,
    bg: Rgba<u8>,
    seed: u64,
    config: &TileConfig,
    angles_override: Option<&Vec<f32>>,
    overrides: Option<&TransitionOverrides>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "grass_transition" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    let mut rng = StdRng::seed_from_u64(seed);
    let grass_palette = grass_palette(config)?;
    let mut img = ImageBuffer::from_pixel(size, size, bg);
    let mut base = ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(&mut base, size, Rgba([0, 0, 0, 255]));

    let blade_min = config.blade_min.unwrap_or(1);
    let blade_max = config.blade_max.unwrap_or_else(|| default_blade_max(size));
    let mut density = config.transition_density.unwrap_or(0.25).clamp(0.0, 1.0);
    let mut bias = config.transition_bias.unwrap_or(0.85).clamp(0.0, 1.0);
    let mut falloff = config.transition_falloff.unwrap_or(2.2);
    if let Some(overrides) = overrides {
        if let Some(override_density) = overrides.density {
            density = override_density.clamp(0.0, 1.0);
        }
        if let Some(override_bias) = overrides.bias {
            bias = override_bias.clamp(0.0, 1.0);
        }
        if let Some(override_falloff) = overrides.falloff {
            falloff = override_falloff;
        }
    }
    let angles = angles_override
        .cloned()
        .or_else(|| config.transition_angles.clone())
        .or_else(|| config.transition_angle.map(|angle| vec![angle]))
        .unwrap_or_else(|| vec![333.435]);

    add_grass_blades_weighted(
        &mut img,
        &base,
        &mut rng,
        &grass_palette,
        blade_min,
        blade_max,
        density,
        bias,
        &angles,
        falloff,
    );

    Ok(img)
}

pub fn add_grass_blades_weighted(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    base: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    rng: &mut StdRng,
    palette: &[Rgba<u8>; 4],
    blade_min: i32,
    blade_max: i32,
    density: f32,
    bias: f32,
    angles_deg: &[f32],
    falloff: f32,
) {
    let min_blade = blade_min.max(1);
    let max_blade = blade_max.max(min_blade);
    let w = base.width().max(1) as f32;
    let h = base.height().max(1) as f32;
    let shades = [palette[1], palette[2], palette[3]];

    for (x, y, pixel) in base.enumerate_pixels() {
        if pixel.0[3] == 0 {
            continue;
        }
        let xf = x as f32 / w;
        let yf = y as f32 / h;
        let edge_weight = edge_weight_for_angles(angles_deg, xf, yf);
        let weighted = edge_weight.powf(falloff.max(0.1));
        let prob = density * ((1.0 - bias) + bias * weighted);
        if rng.gen_range(0.0..1.0) > prob {
            continue;
        }
        let length = rng.gen_range(min_blade..=max_blade);
        let shade = shades[rng.gen_range(0..shades.len())];
        for dy in 0..length {
            put_pixel_safe(img, x as i32, y as i32 - dy, shade);
        }
    }
}

fn put_pixel_safe(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, x: i32, y: i32, color: Rgba<u8>) {
    if x >= 0 && y >= 0 {
        let (x, y) = (x as u32, y as u32);
        if x < img.width() && y < img.height() {
            img.put_pixel(x, y, color);
        }
    }
}
