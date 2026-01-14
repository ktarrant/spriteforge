use clap::Parser;
use image::{ImageBuffer, Rgba};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;
use serde::Deserialize;
use std::path::{Path, PathBuf};

const DEFAULT_OUT_DIR: &str = "out/tilesheet";
const TILESET_CONFIG_DIR: &str = "configs/tilesheet";

#[derive(Parser, Debug)]
#[command(name = "spriteforge", about = "Procedural sprite generator")]
struct Args {
    /// Output PNG path
    #[arg(long)]
    out: Option<PathBuf>,

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
    if args.config.is_none()
        && args.out.is_none()
        && args.size.is_none()
        && args.bg.is_none()
        && args.seed.is_none()
    {
        build_all_tilesheets()?;
        return Ok(());
    }

    let config_path = args
        .config
        .as_ref()
        .ok_or("Config file is required unless running with no arguments")?;
    build_from_config_path(config_path, &args)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ConfigFile {
    Tile(TileConfig),
    Tilesheet(TilesheetConfig),
}

#[derive(Debug, Deserialize, Default)]
struct TileConfig {
    name: String,
    size: Option<u32>,
    bg: Option<String>,
    seed: Option<u64>,
    blade_min: Option<i32>,
    blade_max: Option<i32>,
    grass_base: Option<String>,
    grass_shades: Option<[String; 3]>,
    dirt_base: Option<String>,
    dirt_splotches: Option<[String; 2]>,
    dirt_stones: Option<[String; 2]>,
    dirt_splotch_count: Option<u32>,
    dirt_stone_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct TilesheetConfig {
    tile_config: PathBuf,
    seeds: Vec<u64>,
    columns: Option<u32>,
    padding: Option<u32>,
}

fn load_config(path: &Path) -> Result<ConfigFile, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

fn load_tile_config(path: &Path) -> Result<TileConfig, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let config: ConfigFile = serde_json::from_str(&data).map_err(|e| e.to_string())?;
    match config {
        ConfigFile::Tile(tile) => Ok(tile),
        ConfigFile::Tilesheet(_) => Err("Tile config cannot be a tilesheet".to_string()),
    }
}

fn build_from_config_path(config_path: &Path, args: &Args) -> Result<(), String> {
    let config = load_config(config_path)?;
    let out_path = output_path_for_config(config_path, args.out.as_ref());

    let image = match config {
        ConfigFile::Tile(tile) => {
            let size = args.size.or(tile.size).unwrap_or(256);
            let bg_hex = args
                .bg
                .clone()
                .or_else(|| tile.bg.clone())
                .unwrap_or_else(|| "#2b2f3a".to_string());
            let bg = parse_hex_color(&bg_hex)?;
            let seed = args
                .seed
                .or(tile.seed)
                .unwrap_or_else(rand::random::<u64>);
            render_tile(size, bg, seed, &tile)?
        }
        ConfigFile::Tilesheet(sheet) => {
            if sheet.seeds.is_empty() {
                return Err("Tilesheet seeds list cannot be empty".to_string());
            }
            let tile_path = resolve_path(config_path, &sheet.tile_config);
            let tile_config = load_tile_config(&tile_path)?;
            let size = args.size.or(tile_config.size).unwrap_or(256);
            let bg_hex = args
                .bg
                .clone()
                .or_else(|| tile_config.bg.clone())
                .unwrap_or_else(|| "#2b2f3a".to_string());
            let bg = parse_hex_color(&bg_hex)?;
            let columns = sheet.columns.unwrap_or(4).max(1);
            let padding = sheet.padding.unwrap_or(0);
            render_tilesheet(
                size,
                bg,
                &tile_config,
                &sheet.seeds,
                columns,
                padding,
            )?
        }
    };

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    image.save(&out_path).map_err(|e| e.to_string())?;
    println!("Saved sprite to {}", out_path.display());
    Ok(())
}

fn build_all_tilesheets() -> Result<(), String> {
    let dir = Path::new(TILESET_CONFIG_DIR);
    if !dir.exists() {
        return Err(format!("Tilesheet config directory not found: {TILESET_CONFIG_DIR}"));
    }
    let mut found = false;
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("config") {
            continue;
        }
        found = true;
        let args = Args {
            out: None,
            size: None,
            bg: None,
            seed: None,
            config: None,
        };
        build_from_config_path(&path, &args)?;
    }
    if !found {
        return Err("No tilesheet configs found".to_string());
    }
    Ok(())
}

fn output_path_for_config(config_path: &Path, out_override: Option<&PathBuf>) -> PathBuf {
    if let Some(out) = out_override {
        return out.clone();
    }
    let stem = config_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    Path::new(DEFAULT_OUT_DIR).join(format!("{stem}.png"))
}

fn grass_palette(config: &TileConfig) -> Result<[Rgba<u8>; 4], String> {
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

fn dirt_palette(config: &TileConfig) -> Result<[Rgba<u8>; 5], String> {
    let base_hex = config
        .dirt_base
        .clone()
        .unwrap_or_else(|| "#6b4a2b".to_string());
    let splotch_hexes = config.dirt_splotches.clone().unwrap_or([
        "#6a4a2f".to_string(),
        "#5c3f27".to_string(),
    ]);
    let stone_hexes = config.dirt_stones.clone().unwrap_or([
        "#4b5057".to_string(),
        "#3e4349".to_string(),
    ]);
    Ok([
        parse_hex_color(&base_hex)?,
        parse_hex_color(&splotch_hexes[0])?,
        parse_hex_color(&splotch_hexes[1])?,
        parse_hex_color(&stone_hexes[0])?,
        parse_hex_color(&stone_hexes[1])?,
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

fn default_blade_max(size: u32) -> i32 {
    ((size / 32).max(2)).min(8) as i32
}

fn render_grass_tile(
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
    let ground = make_grass_tile(size, &palette);
    blit(&mut img, &ground);
    let blade_min = config.blade_min.unwrap_or(1);
    let blade_max = config.blade_max.unwrap_or_else(|| default_blade_max(size));
    add_grass_blades(&mut img, &ground, &mut rng, &palette, blade_min, blade_max);
    Ok(img)
}

fn render_tilesheet(
    size: u32,
    bg: Rgba<u8>,
    config: &TileConfig,
    seeds: &[u64],
    columns: u32,
    padding: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    let cols = columns.max(1);
    let rows = ((seeds.len() as u32) + cols - 1) / cols;
    let sheet_w = cols * size + padding * (cols.saturating_sub(1));
    let sheet_h = rows * size + padding * (rows.saturating_sub(1));
    let mut sheet = ImageBuffer::from_pixel(sheet_w, sheet_h, Rgba([0, 0, 0, 0]));

    for (i, seed) in seeds.iter().enumerate() {
        let tile = render_tile(size, bg, *seed, config)?;
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let x = (col * size + padding * col) as i32;
        let y = (row * size + padding * row) as i32;
        blit_offset(&mut sheet, &tile, x, y);
    }

    Ok(sheet)
}

fn render_tile(
    size: u32,
    bg: Rgba<u8>,
    seed: u64,
    config: &TileConfig,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    match config.name.as_str() {
        "grass" => render_grass_tile(size, bg, seed, config),
        "dirt" => render_dirt_tile(size, bg, seed, config),
        other => Err(format!("Unknown tile name: {other}")),
    }
}

fn render_dirt_tile(
    size: u32,
    bg: Rgba<u8>,
    seed: u64,
    config: &TileConfig,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "dirt" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    let mut rng = StdRng::seed_from_u64(seed);
    let palette = dirt_palette(config)?;
    let mut img = ImageBuffer::from_pixel(size, size, bg);
    let mut base = ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(&mut base, size, palette[0]);
    blit(&mut img, &base);

    let splotches = config
        .dirt_splotch_count
        .unwrap_or((size / 3).max(24));
    for _ in 0..splotches {
        let (cx, cy) = random_tile_point(&base, &mut rng);
        let radius = rng.gen_range(3..=8);
        let shade = if rng.gen_bool(0.5) { palette[1] } else { palette[2] };
        draw_oval(&mut img, &base, cx, cy, radius * 2, radius, shade);
    }

    let stones = config
        .dirt_stone_count
        .unwrap_or((size / 10).max(6));
    for _ in 0..stones {
        let (cx, cy) = random_tile_point(&base, &mut rng);
        let radius = rng.gen_range(1..=3);
        let shade = if rng.gen_bool(0.5) { palette[3] } else { palette[4] };
        if rng.gen_bool(0.5) {
            draw_blob(&mut img, &base, cx, cy, radius, shade);
        } else {
            draw_triangle(&mut img, &base, cx, cy, radius, shade);
        }
    }

    Ok(img)
}

fn random_tile_point(
    base: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    rng: &mut StdRng,
) -> (i32, i32) {
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

fn draw_blob(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    mask: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    cx: i32,
    cy: i32,
    radius: i32,
    color: Rgba<u8>,
) {
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
            let x = cx + dx;
            let y = cy + dy;
            if x < 0 || y < 0 {
                continue;
            }
            let (ux, uy) = (x as u32, y as u32);
            if ux >= mask.width() || uy >= mask.height() {
                continue;
            }
            if mask.get_pixel(ux, uy).0[3] > 0 {
                img.put_pixel(ux, uy, color);
            }
        }
    }
}

fn draw_triangle(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    mask: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    cx: i32,
    cy: i32,
    size: i32,
    color: Rgba<u8>,
) {
    if size <= 0 {
        return;
    }
    let p1 = (cx, cy - size);
    let p2 = (cx - size, cy + size);
    let p3 = (cx + size, cy + size);
    let min_x = p2.0.min(p3.0).min(p1.0);
    let max_x = p2.0.max(p3.0).max(p1.0);
    let min_y = p1.1.min(p2.1).min(p3.1);
    let max_y = p1.1.max(p2.1).max(p3.1);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if !point_in_triangle((x, y), p1, p2, p3) {
                continue;
            }
            if x < 0 || y < 0 {
                continue;
            }
            let (ux, uy) = (x as u32, y as u32);
            if ux >= mask.width() || uy >= mask.height() {
                continue;
            }
            if mask.get_pixel(ux, uy).0[3] > 0 {
                img.put_pixel(ux, uy, color);
            }
        }
    }
}

fn point_in_triangle(p: (i32, i32), a: (i32, i32), b: (i32, i32), c: (i32, i32)) -> bool {
    let d1 = sign(p, a, b);
    let d2 = sign(p, b, c);
    let d3 = sign(p, c, a);
    let has_neg = d1 < 0 || d2 < 0 || d3 < 0;
    let has_pos = d1 > 0 || d2 > 0 || d3 > 0;
    !(has_neg && has_pos)
}

fn sign(p1: (i32, i32), p2: (i32, i32), p3: (i32, i32)) -> i64 {
    (p1.0 - p3.0) as i64 * (p2.1 - p3.1) as i64
        - (p2.0 - p3.0) as i64 * (p1.1 - p3.1) as i64
}

fn draw_oval(
    img: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    mask: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    cx: i32,
    cy: i32,
    rx: i32,
    ry: i32,
    color: Rgba<u8>,
) {
    if rx <= 0 || ry <= 0 {
        return;
    }
    let rx2 = rx * rx;
    let ry2 = ry * ry;
    for dy in -ry..=ry {
        for dx in -rx..=rx {
            let lhs = dx * dx * ry2 + dy * dy * rx2;
            let rhs = rx2 * ry2;
            if lhs > rhs {
                continue;
            }
            let x = cx + dx;
            let y = cy + dy;
            if x < 0 || y < 0 {
                continue;
            }
            let (ux, uy) = (x as u32, y as u32);
            if ux >= mask.width() || uy >= mask.height() {
                continue;
            }
            if mask.get_pixel(ux, uy).0[3] > 0 {
                let existing = *img.get_pixel(ux, uy);
                if existing == color {
                    img.put_pixel(ux, uy, darken_color(color, 24));
                } else {
                    img.put_pixel(ux, uy, color);
                }
            }
        }
    }
}

fn darken_color(color: Rgba<u8>, amount: u8) -> Rgba<u8> {
    let [r, g, b, a] = color.0;
    Rgba([
        r.saturating_sub(amount),
        g.saturating_sub(amount),
        b.saturating_sub(amount),
        a,
    ])
}

fn blit_offset(
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

fn resolve_path(base: &Path, rel: &Path) -> PathBuf {
    if rel.is_absolute() {
        rel.to_path_buf()
    } else {
        base.parent()
            .unwrap_or_else(|| Path::new("."))
            .join(rel)
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

fn blit(target: &mut ImageBuffer<Rgba<u8>, Vec<u8>>, src: &ImageBuffer<Rgba<u8>, Vec<u8>>) {
    for (x, y, pixel) in src.enumerate_pixels() {
        if pixel.0[3] > 0 {
            target.put_pixel(x, y, *pixel);
        }
    }
}
