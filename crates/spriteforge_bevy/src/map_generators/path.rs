use rand::rngs::StdRng;
use rand::Rng;

use crate::BaseTile;

const PATH_RADIUS: i32 = 1;

#[derive(Clone, Copy, Debug)]
pub struct PathSegment {
    pub start_x: i32,
    pub start_y: i32,
    pub end_x: i32,
    pub end_y: i32,
    pub radius: i32,
}

#[derive(Clone, Copy, Debug)]
pub struct MapArea {
    pub center_x: i32,
    pub center_y: i32,
    pub radius: i32,
}

#[derive(Clone, Debug)]
pub struct MapSkeleton {
    pub paths: Vec<PathSegment>,
    pub areas: Vec<MapArea>,
}

pub fn generate_path_map(width: u32, height: u32, rng: &mut StdRng) -> Vec<BaseTile> {
    let skeleton = generate_map_skeleton(width, height, rng);
    rasterize_paths(width, height, &skeleton.paths)
}

pub fn generate_map_skeleton(width: u32, height: u32, rng: &mut StdRng) -> MapSkeleton {
    if width == 0 || height == 0 {
        return MapSkeleton {
            paths: Vec::new(),
            areas: Vec::new(),
        };
    }

    let start_x = width.saturating_sub(1);
    let start_y = 0;
    let fork_x = width / 2;
    let fork_y = height / 2;
    let exit_left_x = 0;
    let exit_left_y = height / 2;
    let exit_right_x = width / 2;
    let exit_right_y = height.saturating_sub(1);

    let mut areas = build_areas(width as i32, height as i32);
    let mut main_segment = Vec::new();
    let mut left_segment = Vec::new();
    let mut right_segment = Vec::new();

    for _ in 0..6 {
        let area_occupied = build_area_occupancy(width as i32, height as i32, &areas);
        main_segment = carve_path_segment_points_avoiding(
            start_x as i32,
            start_y as i32,
            fork_x as i32,
            fork_y as i32,
            width,
            height,
            rng,
            &area_occupied,
            (0, 0),
        );
        if main_segment.is_empty() {
            shrink_areas(&mut areas);
            continue;
        }
        let (fork_px, fork_py) = *main_segment
            .last()
            .unwrap_or(&(start_x as i32, start_y as i32));
        left_segment = carve_path_segment_points_avoiding(
            fork_px,
            fork_py,
            exit_left_x as i32,
            exit_left_y as i32,
            width,
            height,
            rng,
            &area_occupied,
            (-1, 0),
        );
        right_segment = carve_path_segment_points_avoiding(
            fork_px,
            fork_py,
            exit_right_x as i32,
            exit_right_y as i32,
            width,
            height,
            rng,
            &area_occupied,
            (0, 1),
        );
        if !left_segment.is_empty() && !right_segment.is_empty() {
            break;
        }
        shrink_areas(&mut areas);
    }
    if main_segment.is_empty() || left_segment.is_empty() || right_segment.is_empty() {
        areas.clear();
        let area_occupied = build_area_occupancy(width as i32, height as i32, &areas);
        main_segment = carve_path_segment_points_avoiding(
            start_x as i32,
            start_y as i32,
            fork_x as i32,
            fork_y as i32,
            width,
            height,
            rng,
            &area_occupied,
            (0, 0),
        );
        let (fork_px, fork_py) = *main_segment
            .last()
            .unwrap_or(&(start_x as i32, start_y as i32));
        left_segment = carve_path_segment_points_avoiding(
            fork_px,
            fork_py,
            exit_left_x as i32,
            exit_left_y as i32,
            width,
            height,
            rng,
            &area_occupied,
            (-1, 0),
        );
        right_segment = carve_path_segment_points_avoiding(
            fork_px,
            fork_py,
            exit_right_x as i32,
            exit_right_y as i32,
            width,
            height,
            rng,
            &area_occupied,
            (0, 1),
        );
    }

    let mut paths = Vec::new();
    paths.extend(points_to_segments(&main_segment, PATH_RADIUS));
    paths.extend(points_to_segments(&left_segment, PATH_RADIUS));
    paths.extend(points_to_segments(&right_segment, PATH_RADIUS));

    MapSkeleton { paths, areas }
}

pub fn rasterize_paths(width: u32, height: u32, paths: &[PathSegment]) -> Vec<BaseTile> {
    let mut cells = vec![BaseTile::Grass; (width * height) as usize];
    for segment in paths {
        rasterize_segment(width, height, segment, &mut cells);
    }
    cells
}

fn carve_path_segment_points_avoiding(
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    width: u32,
    height: u32,
    rng: &mut StdRng,
    area_occupied: &[bool],
    bias_dir: (i32, i32),
) -> Vec<(i32, i32)> {
    let mut segment = Vec::new();
    let mut x = start_x;
    let mut y = start_y;
    let mut last_dir = (0, 0);
    let max_steps = (width * height * 4) as usize;
    let mut steps = 0usize;

    segment.push((x, y));
    while (x, y) != (end_x, end_y) && steps < max_steps {
        steps += 1;
        let dx = (end_x - x).signum();
        let dy = (end_y - y).signum();
        let mut moves = Vec::with_capacity(5);
        moves.push((dx, 0));
        moves.push((0, dy));
        if bias_dir != (0, 0) {
            moves.push(bias_dir);
        }
        if dx == 0 {
            let wiggle_x = if rng.gen_bool(0.5) { 1 } else { -1 };
            moves.push((wiggle_x, 0));
        }
        if dy == 0 {
            let wiggle_y = if rng.gen_bool(0.5) { 1 } else { -1 };
            moves.push((0, wiggle_y));
        }
        if moves.len() > 1 && rng.gen_bool(0.45) {
            let last = moves.len() - 1;
            moves.swap(0, last);
        }

        let mut moved = false;
        for (mx, my) in moves {
            if mx == 0 && my == 0 {
                continue;
            }
            let nx = x + mx;
            let ny = y + my;
            if nx < 0
                || ny < 0
                || nx >= width as i32
                || ny >= height as i32
            {
                continue;
            }
            let idx = (ny * width as i32 + nx) as usize;
            if area_occupied[idx] {
                continue;
            }
            x = nx;
            y = ny;
            last_dir = (mx, my);
            segment.push((x, y));
            moved = true;
            break;
        }

        if !moved {
            if try_detour(
                &mut x,
                &mut y,
                &mut last_dir,
                width as i32,
                height as i32,
                area_occupied,
                &mut segment,
            ) {
                continue;
            }
            break;
        }
    }

    segment
}

fn points_to_segments(points: &[(i32, i32)], radius: i32) -> Vec<PathSegment> {
    if points.len() < 2 {
        return Vec::new();
    }
    let mut segments = Vec::new();
    let mut start = points[0];
    let mut prev = points[0];
    let mut dir = (points[1].0 - points[0].0, points[1].1 - points[0].1);
    for &point in points.iter().skip(1) {
        let next_dir = (point.0 - prev.0, point.1 - prev.1);
        if next_dir != dir {
            segments.push(PathSegment {
                start_x: start.0,
                start_y: start.1,
                end_x: prev.0,
                end_y: prev.1,
                radius,
            });
            start = prev;
            dir = next_dir;
        }
        prev = point;
    }
    segments.push(PathSegment {
        start_x: start.0,
        start_y: start.1,
        end_x: prev.0,
        end_y: prev.1,
        radius,
    });
    segments
}

fn rasterize_segment(width: u32, height: u32, segment: &PathSegment, cells: &mut [BaseTile]) {
    let dx = (segment.end_x - segment.start_x).signum();
    let dy = (segment.end_y - segment.start_y).signum();
    let steps = (segment.end_x - segment.start_x).abs() + (segment.end_y - segment.start_y).abs();
    for step in 0..=steps {
        let x = segment.start_x + dx * step;
        let y = segment.start_y + dy * step;
        for ny in (y - segment.radius)..=(y + segment.radius) {
            for nx in (x - segment.radius)..=(x + segment.radius) {
                if nx < 0 || ny < 0 {
                    continue;
                }
                let nx = nx as u32;
                let ny = ny as u32;
                if nx >= width || ny >= height {
                    continue;
                }
                let idx = (ny * width + nx) as usize;
                cells[idx] = BaseTile::Dirt;
            }
        }
    }
}

fn build_areas(width: i32, height: i32) -> Vec<MapArea> {
    if width < 5 || height < 5 {
        return Vec::new();
    }
    let mut area_occupied = vec![false; (width * height) as usize];
    let min_dim = width.min(height);
    let minor_radius = (min_dim / 10).clamp(3, 8);
    let min_minor_radius = 2;
    let major_radius = (min_dim / 6)
        .max(minor_radius + 1)
        .min((min_dim / 3).max(2));
    let min_major_radius = (min_minor_radius + 1).min(major_radius);
    let max_offset = (min_dim / 5).max(6).min(16);
    let offsets = build_search_offsets(max_offset);

    let targets = [
        (width / 6, height / 4, false),
        (width / 2, height / 5, false),
        (3 * width / 4, 5 * height / 6, false),
        (3 * width / 4, height / 2, false),
        (width / 4, 3 * height / 4, true),
    ];

    let mut areas = Vec::new();
    for (target_x, target_y, major) in targets {
        let base_radius = if major { major_radius } else { minor_radius };
        let min_radius = if major {
            min_major_radius
        } else {
            min_minor_radius
        };
        let mut placed = None;
        for radius in (min_radius..=base_radius).rev() {
            for (ox, oy) in offsets.iter().copied() {
                let cx = target_x + ox;
                let cy = target_y + oy;
                if circle_fits(
                    cx,
                    cy,
                    radius,
                    width,
                    height,
                    &area_occupied,
                ) {
                    placed = Some(MapArea {
                        center_x: cx,
                        center_y: cy,
                        radius,
                    });
                    mark_circle_occupancy(cx, cy, radius, width, height, &mut area_occupied);
                    break;
                }
            }
            if placed.is_some() {
                break;
            }
        }
        if let Some(area) = placed {
            areas.push(area);
        }
    }
    areas
}

fn build_search_offsets(max_offset: i32) -> Vec<(i32, i32)> {
    let mut offsets = Vec::new();
    for dy in -max_offset..=max_offset {
        for dx in -max_offset..=max_offset {
            offsets.push((dx, dy));
        }
    }
    offsets.sort_by_key(|(dx, dy)| dx.abs() + dy.abs());
    offsets
}

fn circle_fits(
    center_x: i32,
    center_y: i32,
    radius: i32,
    width: i32,
    height: i32,
    area_occupied: &[bool],
) -> bool {
    if center_x - radius < 0
        || center_y - radius < 0
        || center_x + radius >= width
        || center_y + radius >= height
    {
        return false;
    }
    let radius_sq = radius * radius;
    for y in (center_y - radius)..=(center_y + radius) {
        for x in (center_x - radius)..=(center_x + radius) {
            let dx = x - center_x;
            let dy = y - center_y;
            if dx * dx + dy * dy > radius_sq {
                continue;
            }
            let idx = (y * width + x) as usize;
            if area_occupied[idx] {
                return false;
            }
        }
    }
    true
}

fn mark_circle_occupancy(
    center_x: i32,
    center_y: i32,
    radius: i32,
    width: i32,
    height: i32,
    area_occupied: &mut [bool],
) {
    let radius_sq = radius * radius;
    for y in (center_y - radius)..=(center_y + radius) {
        for x in (center_x - radius)..=(center_x + radius) {
            if x < 0 || y < 0 || x >= width || y >= height {
                continue;
            }
            let dx = x - center_x;
            let dy = y - center_y;
            if dx * dx + dy * dy > radius_sq {
                continue;
            }
            let idx = (y * width + x) as usize;
            area_occupied[idx] = true;
        }
    }
}

fn build_area_occupancy(width: i32, height: i32, areas: &[MapArea]) -> Vec<bool> {
    let mut occupied = vec![false; (width * height) as usize];
    for area in areas {
        mark_circle_occupancy(
            area.center_x,
            area.center_y,
            area.radius,
            width,
            height,
            &mut occupied,
        );
    }
    occupied
}

fn shrink_areas(areas: &mut [MapArea]) {
    for area in areas {
        if area.radius > 1 {
            area.radius -= 1;
        }
    }
}

fn try_detour(
    x: &mut i32,
    y: &mut i32,
    last_dir: &mut (i32, i32),
    width: i32,
    height: i32,
    area_occupied: &[bool],
    segment: &mut Vec<(i32, i32)>,
) -> bool {
    let (dx, dy) = *last_dir;
    if dx == 0 && dy == 0 {
        return false;
    }
    let detours = if dx != 0 {
        [(0, 1), (0, -1)]
    } else {
        [(1, 0), (-1, 0)]
    };
    for (mx, my) in detours {
        let nx = *x + mx;
        let ny = *y + my;
        if nx < 0 || ny < 0 || nx >= width || ny >= height {
            continue;
        }
        let idx = (ny * width + nx) as usize;
        if area_occupied[idx] {
            continue;
        }
        *x = nx;
        *y = ny;
        *last_dir = (mx, my);
        segment.push((*x, *y));
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn dirt_metrics(tiles: &[BaseTile]) -> (usize, f32) {
        let dirt_count = tiles.iter().filter(|tile| matches!(tile, BaseTile::Dirt)).count();
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
        let tiles = generate_path_map(width, height, &mut rng);
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
        let skeleton = generate_map_skeleton(width, height, &mut rng);
        let total_length: i32 = skeleton
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
