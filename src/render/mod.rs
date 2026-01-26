use image::{ImageBuffer, Rgba};

use crate::config::{TileConfig, TilesheetEntry, TransitionOverrides};

mod debug_weight;
mod dirt;
mod grass;
mod path;
mod tree;
pub mod transition;
mod util;
mod water;

pub use util::parse_hex_color;

pub fn render_tilesheet(
    sprite_width: u32,
    sprite_height: u32,
    bg: Rgba<u8>,
    config: &TileConfig,
    entries: &[TilesheetEntry],
    columns: u32,
    padding: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name == "grass_transition" {
        return transition::render_transition_tilesheet(
            sprite_width,
            sprite_height,
            bg,
            entries,
            columns,
            padding,
            |mask, _seed, overrides| {
                grass::render_grass_transition_tile(
                    sprite_width,
                    sprite_height,
                    bg,
                    _seed,
                    config,
                    mask,
                    overrides,
                )
            },
        );
    }
    if config.name == "water_transition" {
        return transition::render_transition_tilesheet(
            sprite_width,
            sprite_height,
            bg,
            entries,
            columns,
            padding,
            |mask, _seed, overrides| {
                water::render_water_transition_tile(
                    sprite_width,
                    sprite_height,
                    bg,
                    config,
                    mask,
                    overrides,
                )
            },
        );
    }
    if config.name == "path_transition" {
        return transition::render_transition_tilesheet(
            sprite_width,
            sprite_height,
            bg,
            entries,
            columns,
            padding,
            |mask, _seed, _overrides| {
                path::render_path_transition_tile(sprite_width, sprite_height, bg, config, mask)
            },
        );
    }
    let cols = columns.max(1);
    let rows = ((entries.len() as u32) + cols - 1) / cols;
    let sheet_w = cols * sprite_width + padding * (cols.saturating_sub(1));
    let sheet_h = rows * sprite_height + padding * (rows.saturating_sub(1));
    let mut sheet = ImageBuffer::from_pixel(sheet_w, sheet_h, Rgba([0, 0, 0, 0]));

    for (i, entry) in entries.iter().enumerate() {
        let tile = render_tile(
            sprite_width,
            sprite_height,
            bg,
            entry.seed,
            config,
            entry.transition_mask,
            Some(&entry.overrides),
        )?;
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let x = (col * sprite_width + padding * col) as i32;
        let y = (row * sprite_height + padding * row) as i32;
        util::blit_offset(&mut sheet, &tile, x, y);
    }

    Ok(sheet)
}

pub fn render_tilesheet_mask(
    sprite_width: u32,
    sprite_height: u32,
    config: &TileConfig,
    entries: &[TilesheetEntry],
    columns: u32,
    padding: u32,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    if config.name == "water_transition" {
        return transition::render_transition_mask_tilesheet(
            sprite_width,
            sprite_height,
            entries,
            columns,
            padding,
            |mask, overrides| {
                water::render_water_transition_mask_tile(
                    sprite_width,
                    sprite_height,
                    config,
                    mask,
                    overrides,
                )
            },
        );
    }
    let cols = columns.max(1);
    let rows = ((entries.len() as u32) + cols - 1) / cols;
    let sheet_w = cols * sprite_width + padding * (cols.saturating_sub(1));
    let sheet_h = rows * sprite_height + padding * (rows.saturating_sub(1));
    let mut sheet = ImageBuffer::from_pixel(sheet_w, sheet_h, Rgba([0, 0, 0, 0]));

    for (i, entry) in entries.iter().enumerate() {
        let mask_tile = render_tile_mask(
            sprite_width,
            sprite_height,
            entry.seed,
            config,
            entry.transition_mask,
            Some(&entry.overrides),
        )?;
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let x = (col * sprite_width + padding * col) as i32;
        let y = (row * sprite_height + padding * row) as i32;
        util::blit_offset(&mut sheet, &mask_tile, x, y);
    }

    Ok(sheet)
}

fn render_tile_mask(
    sprite_width: u32,
    sprite_height: u32,
    seed: u64,
    config: &TileConfig,
    transition_mask: Option<u8>,
    overrides: Option<&TransitionOverrides>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    match config.name.as_str() {
        "water" => Ok(water::render_water_mask_tile(sprite_width, sprite_height)),
        "water_transition" => water::render_water_transition_mask_tile(
            sprite_width,
            sprite_height,
            config,
            transition_mask.unwrap_or(transition::EDGE_N),
            overrides,
        ),
        "tree" | "bush" => tree::render_tree_mask_tile(sprite_width, sprite_height, seed, config),
        other => Err(format!("No mask renderer for tile name: {other}")),
    }
}

pub fn render_tile(
    sprite_width: u32,
    sprite_height: u32,
    bg: Rgba<u8>,
    seed: u64,
    config: &TileConfig,
    transition_mask: Option<u8>,
    overrides: Option<&TransitionOverrides>,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String> {
    match config.name.as_str() {
        "grass" => grass::render_grass_tile(sprite_width, sprite_height, bg, seed, config),
        "dirt" => dirt::render_dirt_tile(sprite_width, sprite_height, bg, seed, config),
        "grass_transition" => grass::render_grass_transition_tile(
            sprite_width,
            sprite_height,
            bg,
            seed,
            config,
            transition_mask.unwrap_or(transition::EDGE_N),
            overrides,
        ),
        "water" => water::render_water_tile(sprite_width, sprite_height, bg, config),
        "water_transition" => {
            water::render_water_transition_tile(
                sprite_width,
                sprite_height,
                bg,
                config,
                transition_mask.unwrap_or(transition::EDGE_N),
                overrides,
            )
        }
        "path" => path::render_path_tile(sprite_width, sprite_height, bg, config),
        "path_transition" => path::render_path_transition_tile(
            sprite_width,
            sprite_height,
            bg,
            config,
            transition_mask.unwrap_or(transition::EDGE_N),
        ),
        "tree" | "bush" => tree::render_tree_tile(sprite_width, sprite_height, bg, seed, config),
        "debug_weight" => debug_weight::render_weight_debug_tile(
            sprite_width,
            sprite_height,
            bg,
            config,
            transition_mask,
        ),
        other => Err(format!("Unknown tile name: {other}")),
    }
}
