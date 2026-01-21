use std::collections::BTreeSet;

use image::{ImageBuffer, Rgba};

use crate::config::{TilesheetEntry, TransitionOverrides};
use crate::render::util;

pub const EDGE_N: u8 = 1 << 0;
pub const EDGE_E: u8 = 1 << 1;
pub const EDGE_S: u8 = 1 << 2;
pub const EDGE_W: u8 = 1 << 3;
pub const CORNER_NE: u8 = 1 << 4;
pub const CORNER_SE: u8 = 1 << 5;
pub const CORNER_SW: u8 = 1 << 6;
pub const CORNER_NW: u8 = 1 << 7;

pub const EDGE_MASK: u8 = EDGE_N | EDGE_E | EDGE_S | EDGE_W;
pub const CORNER_MASK: u8 = CORNER_NE | CORNER_SE | CORNER_SW | CORNER_NW;

pub fn normalize_47(mask: u8) -> u8 {
    let mut normalized = mask;
    if (mask & EDGE_N != 0) && (mask & EDGE_E != 0) {
        normalized &= !CORNER_NE;
    }
    if (mask & EDGE_S != 0) && (mask & EDGE_E != 0) {
        normalized &= !CORNER_SE;
    }
    if (mask & EDGE_S != 0) && (mask & EDGE_W != 0) {
        normalized &= !CORNER_SW;
    }
    if (mask & EDGE_N != 0) && (mask & EDGE_W != 0) {
        normalized &= !CORNER_NW;
    }
    normalized
}

pub fn all_47_masks() -> Vec<u8> {
    let mut masks = BTreeSet::new();
    for raw in 0u8..=u8::MAX {
        masks.insert(normalize_47(raw));
    }
    masks.into_iter().filter(|mask| *mask != 0).collect()
}

pub fn angles_for_mask(mask: u8) -> Vec<f32> {
    let mask = normalize_47(mask);
    let mut angles = Vec::new();
    if mask & EDGE_N != 0 {
        angles.push(333.435);
    }
    if mask & EDGE_E != 0 {
        angles.push(26.565);
    }
    if mask & EDGE_S != 0 {
        angles.push(153.435);
    }
    if mask & EDGE_W != 0 {
        angles.push(206.565);
    }
    if mask & CORNER_NE != 0 {
        angles.push(0.0);
    }
    if mask & CORNER_NW != 0 {
        angles.push(270.0);
    }
    if mask & CORNER_SW != 0 {
        angles.push(180.0);
    }
    if mask & CORNER_SE != 0 {
        angles.push(90.0);
    }
    angles
}

pub fn mask_index_47(mask: u8) -> Option<usize> {
    let normalized = normalize_47(mask);
    all_47_masks()
        .iter()
        .position(|&candidate| candidate == normalized)
}

pub fn mask_edges(mask: u8) -> u8 {
    mask & EDGE_MASK
}

pub fn mask_corners(mask: u8) -> u8 {
    mask & CORNER_MASK
}

pub fn render_transition_tilesheet<F>(
    size: u32,
    bg: Rgba<u8>,
    entries: &[TilesheetEntry],
    columns: u32,
    padding: u32,
    mut render_tile: F,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String>
where
    F: FnMut(u8, u64, Option<&TransitionOverrides>) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String>,
{
    let masks = all_47_masks();
    let cols = columns.max(1);
    let rows = ((masks.len() as u32) + cols - 1) / cols;
    let sheet_w = cols * size + padding * (cols.saturating_sub(1));
    let sheet_h = rows * size + padding * (rows.saturating_sub(1));
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
        let x = (col * size + padding * col) as i32;
        let y = (row * size + padding * row) as i32;
        util::blit_offset(&mut sheet, &tile, x, y);
    }

    Ok(sheet)
}

pub fn render_transition_mask_tilesheet<F>(
    size: u32,
    entries: &[TilesheetEntry],
    columns: u32,
    padding: u32,
    mut render_tile: F,
) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String>
where
    F: FnMut(u8, Option<&TransitionOverrides>) -> Result<ImageBuffer<Rgba<u8>, Vec<u8>>, String>,
{
    let masks = all_47_masks();
    let cols = columns.max(1);
    let rows = ((masks.len() as u32) + cols - 1) / cols;
    let sheet_w = cols * size + padding * (cols.saturating_sub(1));
    let sheet_h = rows * size + padding * (rows.saturating_sub(1));
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
        let x = (col * size + padding * col) as i32;
        let y = (row * size + padding * row) as i32;
        util::blit_offset(&mut sheet, &tile, x, y);
    }

    Ok(sheet)
}
