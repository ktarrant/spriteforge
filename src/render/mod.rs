use image::{ImageBuffer, Rgba};

use crate::config::{TileConfig, TilesheetEntry, TransitionOverrides};

mod debug_weight;
mod dirt;
mod grass;
mod grass_transition;
mod util;
mod water;

pub use util::parse_hex_color;

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
        util::blit_offset(&mut sheet, &tile, x, y);
    }

    Ok(sheet)
}

pub fn render_tilesheet_mask(
    size: u32,
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
        let mask_tile = render_tile_mask(size, config, entry.angles.as_ref())?;
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let x = (col * size + padding * col) as i32;
        let y = (row * size + padding * row) as i32;
        util::blit_offset(&mut sheet, &mask_tile, x, y);
    }

    Ok(sheet)
}

fn render_tile_mask(
    size: u32,
    config: &TileConfig,
    angles_override: Option<&Vec<f32>>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    match config.name.as_str() {
        "water" => Ok(water::render_water_mask_tile(size)),
        "water_transition" => water::render_water_transition_mask_tile(size, config, angles_override),
        other => Err(format!("No mask renderer for tile name: {other}")),
    }
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
        "grass" => grass::render_grass_tile(size, bg, seed, config),
        "dirt" => dirt::render_dirt_tile(size, bg, seed, config),
        "grass_transition" => grass_transition::render_grass_transition_tile(
            size,
            bg,
            seed,
            config,
            angles_override,
            overrides,
        ),
        "water" => water::render_water_tile(size, bg, config),
        "water_transition" => water::render_water_transition_tile(size, bg, config, angles_override),
        "debug_weight" => debug_weight::render_weight_debug_tile(size, bg, config, angles_override),
        other => Err(format!("Unknown tile name: {other}")),
    }
}
