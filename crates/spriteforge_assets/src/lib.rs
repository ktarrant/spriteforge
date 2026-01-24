use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

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


pub fn uv_from_xy(xf: f32, yf: f32) -> (f32, f32) {
    // Left vertex of the diamond in normalized image coords
    let lx = 0.0;
    let ly = 0.75;

    let dx = xf - lx;
    let dy = yf - ly;

    let u: f32 = dx + 2.0 * dy;
    let v: f32 = u - dy * 4.0;

    (u, v)
}

pub fn xy_from_uv(u: f32, v: f32) -> (f32, f32) {
    let x = (u + v) * 0.5;
    let y = (u - v) * 0.25 + 0.75;

    (x, y)
}

pub fn edge_weight_for_mask(mask: u8, xf: f32, yf: f32, cutoff: f32, gradient: f32) -> f32 {
    let mut alpha: f32 = 1.0;
    let (u, v): (f32, f32) = uv_from_xy(xf, yf);

    if mask & EDGE_N != 0 {
        let border = 1.0 - cutoff;
        if v > border {
            alpha = 0.0;
        } else if gradient > 0.0 {
            alpha = alpha.min(smoothstep(border, border - gradient, v));
        }
    }

    if mask & EDGE_W != 0 {
        let border = cutoff;
        if u < border {
            alpha = 0.0;
        } else if gradient > 0.0 {
            alpha = alpha.min(smoothstep(border, border + gradient, u));
        }
    }

    if mask & EDGE_S != 0 {
        let border = cutoff;
        if v < border {
            alpha = 0.0;
        } else if gradient > 0.0 {
            alpha = alpha.min(smoothstep(border, border + gradient, v));
        }
    }

    if mask & EDGE_E != 0 {
        let border = 1.0 - cutoff;
        if u > border {
            alpha = 0.0;
        } else if gradient > 0.0 {
            alpha = alpha.min(smoothstep(border, border - gradient, u));
        }
    }

    if mask & CORNER_NE != 0 {
        let du = u - 1.0;
        let dv = v - 1.0;
        let d = (du * du + dv * dv).sqrt();
        if gradient > 0.0 {
            alpha = alpha.min(smoothstep(cutoff, cutoff + gradient, d));
        }
        if d < cutoff {
            alpha = 0.0;
        }
    }

    if mask & CORNER_NW != 0 {
        let du = u;
        let dv = v - 1.0;
        let d = (du * du + dv * dv).sqrt();
        if gradient > 0.0 {
            alpha = alpha.min(smoothstep(cutoff, cutoff + gradient, d));
        }
        if d < cutoff {
            alpha = 0.0;
        }
    }

    if mask & CORNER_SW != 0 {
        let du = u;
        let dv = v;
        let d = (du * du + dv * dv).sqrt();
        if gradient > 0.0 {
            alpha = alpha.min(smoothstep(cutoff, cutoff + gradient, d));
        }
        if d < cutoff {
            alpha = 0.0;
        }
    }

    if mask & CORNER_SE != 0 {
        let du = u - 1.0;
        let dv = v;
        let d = (du * du + dv * dv).sqrt();
        if gradient > 0.0 {
            alpha = alpha.min(smoothstep(cutoff, cutoff + gradient, d));
        }
        if d < cutoff {
            alpha = 0.0;
        }
    }

    if cutoff > 0.0 && (mask & EDGE_E != 0) && (mask & EDGE_N != 0) {
        let cx = 1.0 - cutoff * 2.0;
        let cy = 0.75;
        let dx = xf - cx;
        let dy = yf - cy;
        let radius = cutoff * 0.5;
        if (dx * dx + dy * dy >= radius * radius) && (xf > cx) {
            alpha = 0.0;
        }
    }

    if cutoff > 0.0 && (mask & EDGE_S != 0) && (mask & EDGE_W != 0) {
        let cx = cutoff * 2.0;
        let cy = 0.75;
        let dx = xf - cx;
        let dy = yf - cy;
        let radius = cutoff * 0.5;
        if (dx * dx + dy * dy >= radius * radius) && (xf < cx) {
            alpha = 0.0;
        }
    }

    if cutoff > 0.0 && (mask & EDGE_W != 0) && (mask & EDGE_N != 0) {
        let cx = 0.5;
        let cy = 0.5 + cutoff * 4.8;
        let dx = xf - cx;
        let dy = yf - cy;
        let radius = cutoff * 4.0;
        if dx * dx + dy * dy >= radius * radius && yf < cy {
            alpha = 0.0;
        }
    }

    if cutoff > 0.0 && (mask & EDGE_S != 0) && (mask & EDGE_E != 0) {
        let cx = 0.5;
        let cy = 1.0 - cutoff * 4.8;
        let dx = xf - cx;
        let dy = yf - cy;
        let radius = cutoff * 4.0;
        if dx * dx + dy * dy >= radius * radius && yf > cy {
            alpha = 0.0;
        }
    }

    alpha.clamp(0.0, 1.0)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TilesheetMetadata {
    pub image: String,
    pub config: String,
    pub sprite_width: Option<u32>,
    pub sprite_height: Option<u32>,
    pub columns: u32,
    pub rows: u32,
    pub padding: u32,
    pub tile_count: usize,
    pub tiles: Vec<TileMetadata>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TileMetadata {
    pub index: usize,
    pub row: u32,
    pub col: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub seed: u64,
    pub transition_mask: Option<u8>,
}

pub fn load_tilesheet_metadata(path: &Path) -> Result<TilesheetMetadata, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

pub fn normalize_mask(mask: u8) -> u8 {
    let mut mask = !mask;

    if (mask & (EDGE_N | EDGE_E)) != (EDGE_N | EDGE_E) { mask &= !CORNER_NE; }
    if (mask & (EDGE_S | EDGE_E)) != (EDGE_S | EDGE_E) { mask &= !CORNER_SE; }
    if (mask & (EDGE_S | EDGE_W)) != (EDGE_S | EDGE_W) { mask &= !CORNER_SW; }
    if (mask & (EDGE_N | EDGE_W)) != (EDGE_N | EDGE_W) { mask &= !CORNER_NW; }

    !mask
}

pub fn all_transition_masks() -> Vec<u8> {
    let mut masks = BTreeSet::new();
    for raw in 0u8..=u8::MAX {
        masks.insert(normalize_mask(raw));
    }
    masks.into_iter().filter(|mask| *mask != 0).collect()
}

pub fn angles_for_mask(mask: u8) -> Vec<f32> {
    let mask = normalize_mask(mask);
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

pub fn mask_index(mask: u8) -> Option<usize> {
    let normalized = normalize_mask(mask);
    all_transition_masks()
        .iter()
        .position(|&candidate| candidate == normalized)
}

pub fn mask_edges(mask: u8) -> u8 {
    mask & EDGE_MASK
}

pub fn mask_corners(mask: u8) -> u8 {
    mask & CORNER_MASK
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    const EPS: f32 = 1e-6;

    #[test]
    fn transition_mask_count() {
        let masks = all_transition_masks();
        assert_eq!(masks.len(), 46);
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= EPS,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn uv_from_xy_matches_expected_points() {
        let (u, v) = uv_from_xy(0.0, 0.75);
        assert_close(u, 0.0);
        assert_close(v, 0.0);

        let (u, v) = uv_from_xy(0.5, 1.0);
        assert_close(u, 1.0);
        assert_close(v, 0.0);

        let (u, v) = uv_from_xy(0.5, 0.5);
        assert_close(u, 0.0);
        assert_close(v, 1.0);

        let (u, v) = uv_from_xy(1.0, 0.75);
        assert_close(u, 1.0);
        assert_close(v, 1.0);
    }

    #[test]
    fn uv_xy_roundtrip() {
        let samples = [
            (0.0, 0.75),
            (0.5, 0.5),
            (0.5, 1.0),
            (1.0, 0.75),
            (0.25, 0.875),
        ];
        for (xf, yf) in samples {
            let (u, v) = uv_from_xy(xf, yf);
            let (x2, y2) = xy_from_uv(u, v);
            assert_close(x2, xf);
            assert_close(y2, yf);
        }

        let uv_samples = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0), (0.2, 0.8)];
        for (u, v) in uv_samples {
            let (xf, yf) = xy_from_uv(u, v);
            let (u2, v2) = uv_from_xy(xf, yf);
            assert_close(u2, u);
            assert_close(v2, v);
        }
    }
}
