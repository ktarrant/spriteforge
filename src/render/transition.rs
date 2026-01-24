use image::{ImageBuffer, Rgba};

use crate::config::{TilesheetEntry, TransitionOverrides};
use crate::render::util;
use spriteforge_assets::all_transition_masks;
#[allow(unused_imports)]
pub use spriteforge_assets::{
    normalize_mask, CORNER_MASK, CORNER_NE, CORNER_NW, CORNER_SE, CORNER_SW, EDGE_E, EDGE_MASK,
    EDGE_N, EDGE_S, EDGE_W,
};

pub fn render_transition_tilesheet<F>(
    tile_width: u32,
    tile_height: u32,
    _bg: Rgba<u8>,
    entries: &[TilesheetEntry],
    columns: u32,
    padding: u32,
    mut render_tile: F,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String>
where
    F: FnMut(u8, u64, Option<&TransitionOverrides>) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String>,
{
    let masks = all_transition_masks();
    let cols = columns.max(1);
    let rows = ((masks.len() as u32) + cols - 1) / cols;
    let sheet_w = cols * tile_width + padding * (cols.saturating_sub(1));
    let sheet_h = rows * tile_height + padding * (rows.saturating_sub(1));
    let mut sheet = ImageBuffer::from_pixel(sheet_w, sheet_h, Rgba([0, 0, 0, 0]));

    for (i, mask) in masks.iter().enumerate() {
        let (seed, overrides) = if entries.is_empty() {
            (1000 + i as u64, None)
        } else {
            let entry = &entries[i % entries.len()];
            (entry.seed, Some(&entry.overrides))
        };
        let tile = render_tile(*mask, seed, overrides)?;
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let x = (col * tile_width + padding * col) as i32;
        let y = (row * tile_height + padding * row) as i32;
        util::blit_offset(&mut sheet, &tile, x, y);
    }

    Ok(sheet)
}

pub fn render_transition_mask_tilesheet<F>(
    tile_width: u32,
    tile_height: u32,
    entries: &[TilesheetEntry],
    columns: u32,
    padding: u32,
    mut render_tile: F,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String>
where
    F: FnMut(u8, Option<&TransitionOverrides>) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String>,
{
    let masks = all_transition_masks();
    let cols = columns.max(1);
    let rows = ((masks.len() as u32) + cols - 1) / cols;
    let sheet_w = cols * tile_width + padding * (cols.saturating_sub(1));
    let sheet_h = rows * tile_height + padding * (rows.saturating_sub(1));
    let mut sheet = ImageBuffer::from_pixel(sheet_w, sheet_h, Rgba([0, 0, 0, 0]));

    for (i, mask) in masks.iter().enumerate() {
        let overrides = if entries.is_empty() {
            None
        } else {
            Some(&entries[i % entries.len()].overrides)
        };
        let tile = render_tile(*mask, overrides)?;
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let x = (col * tile_width + padding * col) as i32;
        let y = (row * tile_height + padding * row) as i32;
        util::blit_offset(&mut sheet, &tile, x, y);
    }

    Ok(sheet)
}
