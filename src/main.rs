use clap::Parser;
use image::{ImageBuffer, Rgba};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "spriteforge", about = "Procedural sprite generator")]
struct Args {
    /// Output PNG path
    #[arg(long, default_value = "out/sprite.png")]
    out: PathBuf,

    /// Image size in pixels (square)
    #[arg(long)]
    size: Option<u32>,

    /// Solid background color (hex)
    #[arg(long)]
    bg: Option<String>,

    /// Random seed for reproducibility
    #[arg(long)]
    seed: Option<u64>,

    /// Path to JSON config file
    #[arg(long)]
    config: Option<PathBuf>,
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    let config = load_config(args.config.as_ref())?;
    let size = args.size.or(config.size).unwrap_or(256);
    let bg_hex = args
        .bg
        .clone()
        .or_else(|| config.bg.clone())
        .unwrap_or_else(|| "#2b2f3a".to_string());
    let bg = parse_hex_color(&bg_hex)?;
    let seed = args.seed.or(config.seed).unwrap_or(0xC0FFEE);
    let mut rng = StdRng::seed_from_u64(seed);

    let mut img = ImageBuffer::from_pixel(size, size, bg);
    let palette = palette(&config)?;
    let name = config.name.clone().unwrap_or_else(|| "grass".to_string());
    if name != "grass" {
        return Err(format!("Unknown config name: {name}"));
    }
    let ground = make_grass_tile(size, &palette);
    blit(&mut img, &ground);
    let blade_min = config.blade_min.unwrap_or(1);
    let blade_max = config.blade_max.unwrap_or_else(|| default_blade_max(size));
    add_grass_blades(&mut img, &ground, &mut rng, &palette, blade_min, blade_max);

    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    img.save(&args.out).map_err(|e| e.to_string())?;
    println!("Saved sprite to {}", args.out.display());
    Ok(())
}

#[derive(Debug, Deserialize, Default)]
struct Config {
    name: Option<String>,
    size: Option<u32>,
    bg: Option<String>,
    seed: Option<u64>,
    blade_min: Option<i32>,
    blade_max: Option<i32>,
    grass_base: Option<String>,
    grass_shades: Option<[String; 3]>,
}

fn load_config(path: Option<&PathBuf>) -> Result<Config, String> {
    let Some(path) = path else {
        return Ok(Config::default());
    };
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

fn palette(config: &Config) -> Result<Vec<Rgba<u8>>, String> {
    let base_hex = config
        .grass_base
        .clone()
        .unwrap_or_else(|| "#205c3e".to_string());
    let shades = config.grass_shades.clone().unwrap_or([
        "#205c3e".to_string(),
        "#32784e".to_string(),
        "#4a9864".to_string(),
    ]);
    Ok(vec![
        parse_hex_color(&base_hex)?,
        parse_hex_color(&shades[0])?,
        parse_hex_color(&shades[1])?,
        parse_hex_color(&shades[2])?,
    ])
}

fn parse_hex_color(hex: &str) -> Result<Rgba<u8>, String> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Err("Color must be in #RRGGBB format".to_string());
    }
    let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "Invalid red".to_string())?;
    let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "Invalid green".to_string())?;
    let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "Invalid blue".to_string())?;
    Ok(Rgba([r, g, b, 255]))
}

fn draw_isometric_ground(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, size: u32, color: Rgba<u8>) {
    let size_f = size.saturating_sub(1) as f32;
    if size_f <= 0.0 {
        return;
    }
    let left_x = 0.0;
    let right_x = size_f;
    let bottom_y = size_f;
    let height = size_f / 2.0;
    let top_y = bottom_y - height;
    let cx = size_f / 2.0;
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
        let end = rx.ceil().min(size_f) as i32;
        for x in start..=end {
            put_pixel_safe(img, x, y, color);
        }
    }
}

fn make_grass_tile(size: u32, palette: &[Rgba<u8>]) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut tile = ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(&mut tile, size, palette[0]);

    tile
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn add_grass_blades(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    base: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    rng: &mut StdRng,
    palette: &[Rgba<u8>],
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

fn default_blade_max(size: u32) -> i32 {
    ((size / 32).max(2)).min(8) as i32
}

fn put_pixel_safe(img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, x: i32, y: i32, color: Rgba<u8>) {
    if x >= 0 && y >= 0 {
        let (x, y) = (x as u32, y as u32);
        if x < img.width() && y < img.height() {
            img.put_pixel(x, y, color);
        }
    }
}

fn blit(target: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, src: &ImageBuffer<Rgba<u8>, Vec<u8>>) {
    for (x, y, pixel) in src.enumerate_pixels() {
        if pixel.0[3] > 0 {
            target.put_pixel(x, y, *pixel);
        }
    }
}
