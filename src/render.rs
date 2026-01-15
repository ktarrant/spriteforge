use image::{ImageBuffer, Rgba};
use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

use crate::config::{TileConfig, TilesheetEntry, TransitionOverrides};

pub fn render_tilesheet(
    size: u32,
    bg: Rgba<u8>,
    config: &TileConfig,
    entries: &[TilesheetEntry],
    columns: u32,
    padding: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    let cols = columns.max(1);
    let rows = ((entries.len() as u32) + cols - 1) / cols;
    let sheet_w = cols * size + padding * (cols.saturating_sub(1));
    let sheet_h = rows * size + padding * (rows.saturating_sub(1));
    let mut sheet = ImageBuffer::from_pixel(sheet_w, sheet_h, Rgba([0, 0, 0, 0]));

    for (i, entry) in entries.iter().enumerate() {
        let tile = render_tile(
            size,
            bg,
            entry.seed,
            config,
            entry.angles.as_ref(),
            Some(&entry.overrides),
        )?;
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let x = (col * size + padding * col) as i32;
        let y = (row * size + padding * row) as i32;
        blit_offset(&mut sheet, &tile, x, y);
    }

    Ok(sheet)
}

pub fn render_tilesheet_mask(
    size: u32,
    entries: &[TilesheetEntry],
    columns: u32,
    padding: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    let cols = columns.max(1);
    let rows = ((entries.len() as u32) + cols - 1) / cols;
    let sheet_w = cols * size + padding * (cols.saturating_sub(1));
    let sheet_h = rows * size + padding * (rows.saturating_sub(1));
    let mut sheet = ImageBuffer::from_pixel(sheet_w, sheet_h, Rgba([0, 0, 0, 0]));
    let mask_tile = render_water_mask_tile(size);

    for (i, _entry) in entries.iter().enumerate() {
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let x = (col * size + padding * col) as i32;
        let y = (row * size + padding * row) as i32;
        blit_offset(&mut sheet, &mask_tile, x, y);
    }

    Ok(sheet)
}

pub fn render_tile(
    size: u32,
    bg: Rgba<u8>,
    seed: u64,
    config: &TileConfig,
    angles_override: Option<&Vec<f32>>,
    overrides: Option<&TransitionOverrides>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    match config.name.as_str() {
        "grass" => render_grass_tile(size, bg, seed, config),
        "dirt" => render_dirt_tile(size, bg, seed, config),
        "transition" => render_transition_tile(size, bg, seed, config, angles_override, overrides),
        "grass_transition" => {
            render_grass_transition_tile(size, bg, seed, config, angles_override, overrides)
        }
        "water" => render_water_tile(size, bg, config),
        "debug_weight" => render_weight_debug_tile(size, bg, config, angles_override),
        other => Err(format!("Unknown tile name: {other}")),
    }
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

fn render_transition_tile(
    size: u32,
    bg: Rgba<u8>,
    seed: u64,
    config: &TileConfig,
    angles_override: Option<&Vec<f32>>,
    overrides: Option<&TransitionOverrides>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name != "transition" {
        return Err(format!("Unknown tile name: {}", config.name));
    }
    let mut rng = StdRng::seed_from_u64(seed);
    let dirt_palette = dirt_palette(config)?;
    let grass_palette = grass_palette(config)?;
    let mut img = ImageBuffer::from_pixel(size, size, bg);
    let mut base = ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(&mut base, size, dirt_palette[0]);
    blit(&mut img, &base);

    let splotches = config
        .dirt_splotch_count
        .unwrap_or((size / 3).max(24));
    for _ in 0..splotches {
        let (cx, cy) = random_tile_point(&base, &mut rng);
        let radius = rng.gen_range(3..=8);
        let shade = if rng.gen_bool(0.5) {
            dirt_palette[1]
        } else {
            dirt_palette[2]
        };
        draw_oval(&mut img, &base, cx, cy, radius * 2, radius, shade);
    }

    let stones = config
        .dirt_stone_count
        .unwrap_or((size / 10).max(6));
    for _ in 0..stones {
        let (cx, cy) = random_tile_point(&base, &mut rng);
        let radius = rng.gen_range(1..=3);
        let shade = if rng.gen_bool(0.5) {
            dirt_palette[3]
        } else {
            dirt_palette[4]
        };
        if rng.gen_bool(0.5) {
            draw_blob(&mut img, &base, cx, cy, radius, shade);
        } else {
            draw_triangle(&mut img, &base, cx, cy, radius, shade);
        }
    }

    let blade_min = config.blade_min.unwrap_or(1);
    let blade_max = config.blade_max.unwrap_or_else(|| default_blade_max(size));
    let mut density = config.transition_density.unwrap_or(0.25).clamp(0.0, 1.0);
    let mut bias = config.transition_bias.unwrap_or(0.85).clamp(0.0, 1.0);
    let mut falloff = config.transition_falloff.unwrap_or(2.2);
    let angles = angles_override
        .cloned()
        .or_else(|| config.transition_angles.clone())
        .or_else(|| config.transition_angle.map(|angle| vec![angle]))
        .unwrap_or_else(|| vec![333.435]);
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

fn render_grass_transition_tile(
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

fn render_water_tile(
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

fn render_weight_debug_tile(
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

fn render_water_mask_tile(size: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut tile = ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(&mut tile, size, Rgba([255, 255, 255, 255]));
    tile
}

fn make_grass_tile(size: u32, palette: &[Rgba<u8>; 4]) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
    let mut tile = ImageBuffer::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    draw_isometric_ground(&mut tile, size, palette[0]);
    tile
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

fn add_grass_blades_weighted(
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
        let weighted = edge_weight.powf(falloff);
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

fn edge_weight_for_angles(angles_deg: &[f32], xf: f32, yf: f32) -> f32 {
    let mut best: f32 = 0.0;
    for &angle in angles_deg {
        best = best.max(edge_weight_for_angle(angle, xf, yf));
    }
    best
}

fn edge_weight_for_angle(angle_deg: f32, xf: f32, yf: f32) -> f32 {
    // Center of the tile in image space (adjust if your tile isn't centered in the image)
    let cx = 0.5;
    let cy = 0.75;

    // Centered coordinates; flip Y so "up" is positive (optional but usually nicer)
    let dx = xf - cx;
    let dy = (cy - yf) / 0.5;

    // Direction unit vector for the gradient
    let a = angle_deg.to_radians();
    let nx = a.cos();
    let ny = a.sin();

    // Signed coordinate along gradient direction
    let s = dx * nx + dy * ny * 2.0;

    // Normalize: in normalized space, s is typically within about [-1,1]
    // Clamp makes it safe even if corners exceed slightly due to aspect.
    (0.25 + 0.5 * s).clamp(0.0, 1.0)
}

fn weight_color(weight: f32) -> Rgba<u8> {
    let t = weight.clamp(0.0, 1.0);
    let v = (255.0 * t) as u8;
    Rgba([v, v, v, 255])
}

fn random_tile_point(base: &ImageBuffer<Rgba<u8>, Vec<u8>>, rng: &mut StdRng) -> (i32, i32) {
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

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
