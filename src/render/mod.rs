use image::{ImageBuffer, Rgba};

use crate::config::{TileConfig, TilesheetEntry, TransitionOverrides};

mod debug_weight;
mod dirt;
mod grass;
mod path;
pub mod transition;
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
    if config.name == "grass_transition" {
        return transition::render_transition_tilesheet(
            size,
            bg,
            entries,
            columns,
            padding,
            |mask, _seed, overrides| {
                grass::render_grass_transition_tile(size, bg, _seed, config, mask, overrides)
            },
        );
    }
    if config.name == "water_transition" {
        return transition::render_transition_tilesheet(
            size,
            bg,
            entries,
            columns,
            padding,
            |mask, _seed, overrides| {
                water::render_water_transition_tile(size, bg, config, mask, overrides)
            },
        );
    }
    if config.name == "path_transition" {
        return transition::render_transition_tilesheet(
            size,
            bg,
            entries,
            columns,
            padding,
            |mask, _seed, _overrides| path::render_path_transition_tile(size, bg, config, mask),
        );
    }
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
            entry.transition_mask,
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
    if config.name == "water_transition" {
        return transition::render_transition_mask_tilesheet(
            size,
            entries,
            columns,
            padding,
            |mask, overrides| {
                water::render_water_transition_mask_tile(size, config, mask, overrides)
            },
        );
    }
    let cols = columns.max(1);
    let rows = ((entries.len() as u32) + cols - 1) / cols;
    let sheet_w = cols * size + padding * (cols.saturating_sub(1));
    let sheet_h = rows * size + padding * (rows.saturating_sub(1));
    let mut sheet = ImageBuffer::from_pixel(sheet_w, sheet_h, Rgba([0, 0, 0, 0]));

    for (i, entry) in entries.iter().enumerate() {
        let mask_tile =
            render_tile_mask(size, config, entry.transition_mask, Some(&entry.overrides))?;
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
    transition_mask: Option<u8>,
    overrides: Option<&TransitionOverrides>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    match config.name.as_str() {
        "water" => Ok(water::render_water_mask_tile(size)),
        "water_transition" => water::render_water_transition_mask_tile(
            size,
            config,
            transition_mask.unwrap_or(transition::EDGE_N),
            overrides,
        ),
        other => Err(format!("No mask renderer for tile name: {other}")),
    }
}

pub fn render_tile(
    size: u32,
    bg: Rgba<u8>,
    seed: u64,
    config: &TileConfig,
    transition_mask: Option<u8>,
    overrides: Option<&TransitionOverrides>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    match config.name.as_str() {
        "grass" => grass::render_grass_tile(size, bg, seed, config),
        "dirt" => dirt::render_dirt_tile(size, bg, seed, config),
        "grass_transition" => grass::render_grass_transition_tile(
            size,
            bg,
            seed,
            config,
            transition_mask.unwrap_or(transition::EDGE_N),
            overrides,
        ),
        "water" => water::render_water_tile(size, bg, config),
        "water_transition" => {
            water::render_water_transition_tile(
                size,
                bg,
                config,
                transition_mask.unwrap_or(transition::EDGE_N),
                overrides,
            )
        }
        "path" => path::render_path_tile(size, bg, config),
        "path_transition" => path::render_path_transition_tile(
            size,
            bg,
            config,
            transition_mask.unwrap_or(transition::EDGE_N),
        ),
        "debug_weight" => debug_weight::render_weight_debug_tile(
            size,
            bg,
            config,
            transition_mask,
        ),
        other => Err(format!("Unknown tile name: {other}")),
    }
}
