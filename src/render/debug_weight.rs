use image::{ImageBuffer, Rgba};

use crate::config::TileConfig;
use crate::render::transition;
use crate::render::util::draw_isometric_ground;
use spriteforge_assets::edge_weight_for_mask;

pub fn render_weight_debug_tile(
    tile_width: u32,
    tile_height: u32,
    bg: Rgba<u8>,
    config: &TileConfig,
    transition_mask: Option<u8>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "debug_weight" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    let mask = transition_mask.unwrap_or(transition::EDGE_N);

    let mut img = ImageBuffer::from_pixel(tile_width, tile_height, bg);
    let mut base = ImageBuffer::from_pixel(tile_width, tile_height, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(&mut base, tile_width, tile_height, Rgba([0, 0, 0, 255]));

    let width = base.width().max(1) as f32;
    for (x, y, pixel) in base.enumerate_pixels() {
        if pixel.0[3] == 0 {
            continue;
        }
        let xf = x as f32 / width;
        let yf = y as f32 / width;
        let weight = edge_weight_for_mask(mask, xf, yf, 0.0, 1.0);
        img.put_pixel(x, y, weight_color(weight));
    }

    Ok(img)
}

fn weight_color(weight: f32) -> Rgba<u8> {
    let t = weight.clamp(0.0, 1.0);
    let v = (255.0 * t) as u8;
    Rgba([v, v, v, 255])
}
