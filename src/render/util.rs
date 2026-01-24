use image::{ImageBuffer, Rgba};
use rand::Rng;
use rand::rngs::StdRng;

pub fn parse_hex_color(hex: &str) -> Result<Rgba<u8>, String> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.eq_ignore_ascii_case("transparent") {
        return Ok(Rgba([0, 0, 0, 0]));
    }
    if hex.len() != 6 {
        return Err("Color must be in #RRGGBB format or 'transparent'".to_string());
    }
    let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid red".to_string())?;
    let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid green".to_string())?;
    let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid blue".to_string())?;
    Ok(Rgba([r, g, b, 255]))
}

pub fn draw_isometric_ground(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    tile_width: u32,
    tile_height: u32,
    color: Rgba<u8>,
) {
    let width_f = tile_width.saturating_sub(1) as f32;
    if width_f <= 0.0 {
        return;
    }
    let left_x = 0.0;
    let right_x = width_f;
    let bottom_y = tile_height.saturating_sub(1) as f32;
    let height = width_f / 2.0;
    let top_y = bottom_y - height;
    let cx = width_f / 2.0;
    let mid_y = bottom_y - height / 2.0;

    let y_start = top_y.ceil() as i32;
    let y_end = bottom_y.floor() as i32;

    for y in y_start..=y_end {
        let yf = y as f32;
        let (lx, rx) = if yf <= mid_y {
            let t = (yf - top_y) / (mid_y - top_y);
            (lerp(cx, left_x, t), lerp(cx, right_x, t))
        } else {
            let t = (yf - mid_y) / (bottom_y - mid_y);
            (lerp(left_x, cx, t), lerp(right_x, cx, t))
        };
        let start = lx.floor().max(0.0) as i32;
        let end = rx.ceil().min(width_f) as i32;
        for x in start..=end {
            put_pixel_safe(img, x, y, color);
        }
    }
}


pub fn random_tile_point(base: &ImageBuffer<Rgba<u8>, Vec<u8>>, rng: &mut StdRng) -> (i32, i32) {
    let w = base.width() as i32;
    let h = base.height() as i32;
    for _ in 0..500 {
        let x = rng.gen_range(0..w);
        let y = rng.gen_range(0..h);
        if base.get_pixel(x as u32, y as u32).0[3] > 0 {
            return (x, y);
        }
    }
    (w / 2, h / 2)
}

pub fn blit_offset(
    target: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    src: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    offset_x: i32,
    offset_y: i32,
) {
    for (x, y, pixel) in src.enumerate_pixels() {
        if pixel.0[3] > 0 {
            let tx = x as i32 + offset_x;
            let ty = y as i32 + offset_y;
            if tx >= 0 && ty >= 0 {
                let (tx, ty) = (tx as u32, ty as u32);
                if tx < target.width() && ty < target.height() {
                    target.put_pixel(tx, ty, *pixel);
                }
            }
        }
    }
}

pub fn blit(target: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, src: &ImageBuffer<Rgba<u8>, Vec<u8>>) {
    for (x, y, pixel) in src.enumerate_pixels() {
        if pixel.0[3] > 0 {
            target.put_pixel(x, y, *pixel);
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

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
