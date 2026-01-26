use crate::map_layout::{AreaType, MapLayout, PathSegment};
use crate::BaseTile;

pub fn rasterize_paths(width: u32, height: u32, paths: &[PathSegment]) -> Vec<BaseTile> {
    let mut cells = vec![BaseTile::Grass; (width * height) as usize];
    for segment in paths {
        rasterize_segment(width, height, segment, &mut cells);
    }
    cells
}

pub fn rasterize_layout(width: u32, height: u32, skeleton: &MapLayout) -> Vec<BaseTile> {
    let mut cells = vec![BaseTile::Grass; (width * height) as usize];
    for segment in &skeleton.paths {
        rasterize_segment(width, height, segment, &mut cells);
    }
    for area in &skeleton.areas {
        if area.area_type != Some(AreaType::Dock) {
            continue;
        }
        fill_water_circle(width, height, area.center_x, area.center_y, area.radius, &mut cells);
    }
    for segment in &skeleton.water_paths {
        rasterize_water_segment(width, height, segment, &mut cells);
    }
    cells
}

fn rasterize_segment(width: u32, height: u32, segment: &PathSegment, cells: &mut [BaseTile]) {
    let dx = (segment.end_x - segment.start_x).signum();
    let dy = (segment.end_y - segment.start_y).signum();
    let steps = (segment.end_x - segment.start_x).abs() + (segment.end_y - segment.start_y).abs();
    let path_width = if segment.radius >= 1 { 2 } else { 1 };
    for step in 0..=steps {
        let x = segment.start_x + dx * step;
        let y = segment.start_y + dy * step;
        if dx != 0 {
            for offset in 0..path_width {
                set_tile(width, height, x, y + offset, BaseTile::Path, cells, true);
            }
            set_tile(width, height, x, y - 1, BaseTile::Dirt, cells, false);
            set_tile(width, height, x, y + path_width, BaseTile::Dirt, cells, false);
        } else {
            for offset in 0..path_width {
                set_tile(width, height, x + offset, y, BaseTile::Path, cells, true);
            }
            set_tile(width, height, x - 1, y, BaseTile::Dirt, cells, false);
            set_tile(width, height, x + path_width, y, BaseTile::Dirt, cells, false);
        }
    }
}

fn rasterize_water_segment(width: u32, height: u32, segment: &PathSegment, cells: &mut [BaseTile]) {
    let water_radius = segment.radius.max(2);
    let dx = (segment.end_x - segment.start_x).signum();
    let dy = (segment.end_y - segment.start_y).signum();
    let steps = (segment.end_x - segment.start_x).abs() + (segment.end_y - segment.start_y).abs();
    for step in 0..=steps {
        let x = segment.start_x + dx * step;
        let y = segment.start_y + dy * step;
        for ny in (y - water_radius)..=(y + water_radius) {
            for nx in (x - water_radius)..=(x + water_radius) {
                if nx < 0 || ny < 0 {
                    continue;
                }
                let nx = nx as u32;
                let ny = ny as u32;
                if nx >= width || ny >= height {
                    continue;
                }
                let idx = (ny * width + nx) as usize;
                if matches!(cells[idx], BaseTile::Dirt | BaseTile::Path) {
                    continue;
                }
                cells[idx] = BaseTile::Water;
            }
        }
    }
}

fn fill_water_circle(
    width: u32,
    height: u32,
    center_x: i32,
    center_y: i32,
    radius: i32,
    cells: &mut [BaseTile],
) {
    let radius = radius.max(1);
    let radius_sq = radius * radius;
    for y in (center_y - radius)..=(center_y + radius) {
        for x in (center_x - radius)..=(center_x + radius) {
            if x < 0 || y < 0 {
                continue;
            }
            let x_u = x as u32;
            let y_u = y as u32;
            if x_u >= width || y_u >= height {
                continue;
            }
            let dx = x - center_x;
            let dy = y - center_y;
            if dx * dx + dy * dy > radius_sq {
                continue;
            }
            let idx = (y_u * width + x_u) as usize;
            if matches!(cells[idx], BaseTile::Dirt | BaseTile::Path) {
                continue;
            }
            cells[idx] = BaseTile::Water;
        }
    }
}

fn set_tile(
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    tile: BaseTile,
    cells: &mut [BaseTile],
    overwrite: bool,
) {
    if x < 0 || y < 0 {
        return;
    }
    let x = x as u32;
    let y = y as u32;
    if x >= width || y >= height {
        return;
    }
    let idx = (y * width + x) as usize;
    if overwrite || matches!(cells[idx], BaseTile::Grass) {
        cells[idx] = tile;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_layout::{MapAreaConfig, MapLayoutConfig, MapPointConfig};
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    fn sample_layout_config() -> MapLayoutConfig {
        MapLayoutConfig {
            entry: MapPointConfig { x: 1.0, y: 0.0 },
            fork: MapPointConfig { x: 0.5, y: 0.5 },
            exits: vec![
                MapPointConfig { x: 0.0, y: 0.5 },
                MapPointConfig { x: 0.5, y: 1.0 },
            ],
            areas: vec![
                MapAreaConfig {
                    x: 1.0 / 6.0,
                    y: 1.0 / 4.0,
                    major: false,
                    connect_to: None,
                },
                MapAreaConfig {
                    x: 3.0 / 4.0,
                    y: 3.0 / 4.0,
                    major: true,
                    connect_to: None,
                },
            ],
        }
    }

    fn dirt_metrics(tiles: &[BaseTile]) -> (usize, f32) {
        let dirt_count = tiles
            .iter()
            .filter(|tile| matches!(tile, BaseTile::Dirt | BaseTile::Path))
            .count();
        let dirt_pct = if tiles.is_empty() {
            0.0
        } else {
            dirt_count as f32 / tiles.len() as f32
        };
        (dirt_count, dirt_pct)
    }

    #[test]
    fn path_map_basic_metrics() {
        let width = 64;
        let height = 64;
        let mut rng = StdRng::seed_from_u64(1337);
        let layout = crate::map_layout::generate_map_layout(
            width,
            height,
            &mut rng,
            &sample_layout_config(),
        );
        let tiles = rasterize_layout(width, height, &layout);
        assert_eq!(tiles.len(), (width * height) as usize);

        let (dirt_count, dirt_pct) = dirt_metrics(&tiles);
        let min_dirt = (width * height) as usize / 20;
        let max_dirt = (width * height) as usize * 3 / 4;
        assert!(
            dirt_count >= min_dirt,
            "dirt tiles too few: {dirt_count} ({dirt_pct:.2}%)"
        );
        assert!(
            dirt_count <= max_dirt,
            "dirt tiles too many: {dirt_count} ({dirt_pct:.2}%)"
        );
    }

    #[test]
    fn skeleton_total_length_reasonable() {
        let width = 64;
        let height = 64;
        let mut rng = StdRng::seed_from_u64(1337);
        let layout = crate::map_layout::generate_map_layout(
            width,
            height,
            &mut rng,
            &sample_layout_config(),
        );
        let total_length: i32 = layout
            .paths
            .iter()
            .map(|segment| (segment.end_x - segment.start_x).abs()
                + (segment.end_y - segment.start_y).abs())
            .sum();
        assert!(total_length > 0, "skeleton has no length");
        assert!(
            total_length < (width * height) as i32,
            "skeleton length too large: {total_length}"
        );
    }
}

