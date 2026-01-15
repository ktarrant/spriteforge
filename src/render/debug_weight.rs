use image::{ImageBuffer, Rgba};

use crate::config::TileConfig;
use crate::render::util::{draw_isometric_ground, edge_weight_for_angles};

pub fn render_weight_debug_tile(
    size: u32,
    bg: Rgba<u8>,
    config: &TileConfig,
    angles_override: Option<&Vec<f32>>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "debug_weight" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    let angles = angles_override
        .cloned()
        .or_else(|| config.transition_angles.clone())
        .or_else(|| config.transition_angle.map(|angle| vec![angle]))
        .unwrap_or_else(|| vec![333.435]);

    let mut img = ImageBuffer::from_pixel(size, size, bg);
    let mut base = ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(&mut base, size, Rgba([0, 0, 0, 255]));

    let w = base.width().max(1) as f32;
    let h = base.height().max(1) as f32;
    for (x, y, pixel) in base.enumerate_pixels() {
        if pixel.0[3] == 0 {
            continue;
        }
        let xf = x as f32 / w;
        let yf = y as f32 / h;
        let weight = edge_weight_for_angles(&angles, xf, yf);
        img.put_pixel(x, y, weight_color(weight));
    }

    Ok(img)
}

fn weight_color(weight: f32) -> Rgba<u8> {
    let t = weight.clamp(0.0, 1.0);
    let v = (255.0 * t) as u8;
    Rgba([v, v, v, 255])
}
