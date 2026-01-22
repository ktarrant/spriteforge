use image::{ImageBuffer, Rgba};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::config::{TileConfig, TransitionOverrides};
use crate::render::transition;
use crate::render::util::{blit, draw_isometric_ground, edge_weight_for_mask, parse_hex_color};

pub fn render_grass_tile(
    size: u32,
    bg: Rgba<u8>,
    seed: u64,
    config: &TileConfig,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "grass" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    let mut rng = StdRng::seed_from_u64(seed);
    let palette = grass_palette(config)?;
    let mut img = ImageBuffer::from_pixel(size, size, bg);
    let mut base = ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(&mut base, size, palette[0]);
    blit(&mut img, &base);

    let blade_min = config.blade_min.unwrap_or(1);
    let blade_max = config.blade_max.unwrap_or_else(|| default_blade_max(size));
    add_grass_blades(&mut img, &base, &mut rng, &palette, blade_min, blade_max);
    Ok(img)
}

pub fn render_grass_transition_tile(
    size: u32,
    bg: Rgba<u8>,
    seed: u64,
    config: &TileConfig,
    transition_mask: u8,
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
    let mut edge_cutoff = config.grass_edge_cutoff.unwrap_or(0.0).clamp(0.0, 1.0);
    let mut edge_gradient = config.grass_edge_gradient.unwrap_or(1.3).max(0.0);
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
        if let Some(override_cutoff) = overrides.grass_edge_cutoff {
            edge_cutoff = override_cutoff.clamp(0.0, 1.0);
        }
        if let Some(override_gradient) = overrides.grass_edge_gradient {
            edge_gradient = override_gradient.max(0.0);
        }
    }
    add_grass_blades_weighted(
        &mut img,
        &base,
        &mut rng,
        &grass_palette,
        blade_min,
        blade_max,
        density,
        bias,
        transition_mask,
        edge_cutoff,
        edge_gradient,
        falloff,
    );

    Ok(img)
}

pub fn add_grass_blades(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    base: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    rng: &mut StdRng,
    palette: &[Rgba<u8>; 4],
    blade_min: i32,
    blade_max: i32,
) {
    let min_blade = blade_min.max(1);
    let max_blade = blade_max.max(min_blade);
    let shades = [palette[1], palette[2], palette[3]];

    for (x, y, pixel) in base.enumerate_pixels() {
        if pixel.0[3] == 0 {
            continue;
        }
        let length = rng.gen_range(min_blade..=max_blade);
        let shade = shades[rng.gen_range(0..shades.len())];
        for dy in 0..length {
            put_pixel_safe(img, x as i32, y as i32 - dy, shade);
        }
    }
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
    transition_mask: u8,
    edge_cutoff: f32,
    edge_gradient: f32,
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
        let edge_weight =
            edge_weight_for_mask(transition_mask, xf, yf, edge_cutoff, edge_gradient)
                .powf(falloff);
        let prob = density * ((1.0 - bias) + bias * edge_weight);
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

pub fn default_blade_max(size: u32) -> i32 {
    ((size / 32).max(2)).min(8) as i32
}

pub fn grass_palette(config: &TileConfig) -> Result<[Rgba<u8>; 4], String> {
    let base_hex = config
        .grass_base
        .clone()
        .unwrap_or_else(|| "#205c3e".to_string());
    let shades = config.grass_shades.clone().unwrap_or([
        "#2f6f4a".to_string(),
        "#3f8f5e".to_string(),
        "#58b174".to_string(),
    ]);
    Ok([
        parse_hex_color(&base_hex)?,
        parse_hex_color(&shades[0])?,
        parse_hex_color(&shades[1])?,
        parse_hex_color(&shades[2])?,
    ])
}

fn put_pixel_safe(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, x: i32, y: i32, color: Rgba<u8>) {
    if x >= 0 && y >= 0 {
        let (x, y) = (x as u32, y as u32);
        if x < img.width() && y < img.height() {
            img.put_pixel(x, y, color);
        }
    }
}
