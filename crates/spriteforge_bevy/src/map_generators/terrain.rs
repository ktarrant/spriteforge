use rand::Rng;
use rand::rngs::StdRng;

use crate::BaseTile;

pub fn generate_terrain_map(width: u32, height: u32, rng: &mut StdRng) -> Vec<BaseTile> {
    let mut cells = vec![BaseTile::Dirt; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let roll = rng.gen_range(0.0..1.0);
            cells[idx] = if roll < 0.2 {
                BaseTile::Water
            } else if roll < 0.6 {
                BaseTile::Grass
            } else {
                BaseTile::Dirt
            };
        }
    }
    cells
}

pub fn smooth_terrain(cells: &mut [BaseTile], width: u32, height: u32, passes: usize) {
    let mut temp = cells.to_vec();
    for _ in 0..passes {
        for y in 0..height {
            for x in 0..width {
                let mut grass_count = 0;
                let mut dirt_count = 0;
                let mut water_count = 0;
                for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                    for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                        let idx = (ny * width + nx) as usize;
                        match cells[idx] {
                            BaseTile::Grass => grass_count += 1,
                            BaseTile::Dirt => dirt_count += 1,
                            BaseTile::Water => water_count += 1,
                        }
                    }
                }
                let idx = (y * width + x) as usize;
                let max = grass_count.max(dirt_count).max(water_count);
                temp[idx] = if max == water_count {
                    BaseTile::Water
                } else if max == grass_count {
                    BaseTile::Grass
                } else {
                    BaseTile::Dirt
                };
            }
        }
        cells.copy_from_slice(&temp);
    }
}

pub fn reduce_water_islands(cells: &mut [BaseTile], width: u32, height: u32, passes: usize) {
    let mut temp = cells.to_vec();
    for _ in 0..passes {
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                if cells[idx] != BaseTile::Water {
                    temp[idx] = cells[idx];
                    continue;
                }
                let mut water_neighbors = 0;
                for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                    for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                        if nx == x && ny == y {
                            continue;
                        }
                        let nidx = (ny * width + nx) as usize;
                        if cells[nidx] == BaseTile::Water {
                            water_neighbors += 1;
                        }
                    }
                }
                if water_neighbors < 3 {
                    temp[idx] = BaseTile::Dirt;
                } else {
                    temp[idx] = BaseTile::Water;
                }
            }
        }
        cells.copy_from_slice(&temp);
    }
}
