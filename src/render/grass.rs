use image::{ImageBuffer, Rgba};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::config::{require_field, TileConfig, TransitionOverrides};
use crate::render::util::{blit, draw_isometric_ground, parse_hex_color};
use spriteforge_assets::edge_weight_for_mask;

pub fn render_grass_tile(
    sprite_width: u32,
    sprite_height: u32,
    bg: Rgba<u8>,
    seed: u64,
    config: &TileConfig,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "grass" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    let mut rng = StdRng::seed_from_u64(seed);
    let palette = grass_palette(config)?;
    let mut img = ImageBuffer::from_pixel(sprite_width, sprite_height, bg);
    let mut base = ImageBuffer::from_pixel(sprite_width, sprite_height, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(&mut base, sprite_width, sprite_height, palette[0]);
    blit(&mut img, &base);

    let blade_min = require_field(config.blade_min, "blade_min")?;
    let blade_max = require_field(config.blade_max, "blade_max")?;
    add_grass_blades(&mut img, &base, &mut rng, &palette, blade_min, blade_max);
    Ok(img)
}

pub fn render_grass_transition_tile(
    sprite_width: u32,
    sprite_height: u32,
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
    let mut img = ImageBuffer::from_pixel(sprite_width, sprite_height, bg);
    let mut base = ImageBuffer::from_pixel(sprite_width, sprite_height, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(&mut base, sprite_width, sprite_height, Rgba([0, 0, 0, 255]));

    let blade_min = require_field(config.blade_min, "blade_min")?;
    let blade_max = require_field(config.blade_max, "blade_max")?;
    let mut density = require_field(config.transition_density, "transition_density")?
        .clamp(0.0, 1.0);
    let mut bias =
        require_field(config.transition_bias, "transition_bias")?.clamp(0.0, 1.0);
    let mut falloff = require_field(config.transition_falloff, "transition_falloff")?;
    let mut edge_cutoff =
        require_field(config.grass_edge_cutoff, "grass_edge_cutoff")?.clamp(0.0, 1.0);
    let mut edge_gradient =
        require_field(config.grass_edge_gradient, "grass_edge_gradient")?.max(0.0);
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
    let width = base.width().max(1) as f32;
    let shades = [palette[1], palette[2], palette[3]];

    for (x, y, pixel) in base.enumerate_pixels() {
        if pixel.0[3] == 0 {
            continue;
        }
        let xf = x as f32 / width;
        let yf = y as f32 / width;
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

pub fn grass_palette(config: &TileConfig) -> Result<[Rgba<u8>; 4], String> {
    let base_hex = require_field(config.grass_base.clone(), "grass_base")?;
    let shades = require_field(config.grass_shades.clone(), "grass_shades")?;
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
