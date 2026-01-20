use rand::rngs::StdRng;
use rand::Rng;

use crate::BaseTile;

const PATH_RADIUS: i32 = 1;

#[derive(Clone, Copy, Debug)]
struct PathSegment {
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    radius: i32,
}

#[derive(Clone, Debug)]
struct PathSkeleton {
    segments: Vec<PathSegment>,
}

pub fn generate_path_map(width: u32, height: u32, rng: &mut StdRng) -> Vec<BaseTile> {
    let skeleton = generate_path_skeleton(width, height, rng);
    rasterize_skeleton(width, height, &skeleton)
}

fn generate_path_skeleton(width: u32, height: u32, rng: &mut StdRng) -> PathSkeleton {
    if width == 0 || height == 0 {
        return PathSkeleton { segments: Vec::new() };
    }

    let start_x = width.saturating_sub(1);
    let start_y = 0;
    let fork_x = width / 2;
    let fork_y = height / 2;
    let exit_left_x = 0;
    let exit_left_y = height / 2;
    let exit_right_x = width / 2;
    let exit_right_y = height.saturating_sub(1);

    let main_segment = carve_path_segment_points(
        start_x as i32,
        start_y as i32,
        fork_x as i32,
        fork_y as i32,
        width,
        height,
        rng,
    );
    let (fork_px, fork_py) = *main_segment
        .last()
        .unwrap_or(&(start_x as i32, start_y as i32));
    let left_segment = carve_path_segment_points(
        fork_px,
        fork_py,
        exit_left_x as i32,
        exit_left_y as i32,
        width,
        height,
        rng,
    );
    let right_segment = carve_path_segment_points(
        fork_px,
        fork_py,
        exit_right_x as i32,
        exit_right_y as i32,
        width,
        height,
        rng,
    );

    let mut segments = Vec::new();
    segments.extend(points_to_segments(&main_segment, PATH_RADIUS));
    segments.extend(points_to_segments(&left_segment, PATH_RADIUS));
    segments.extend(points_to_segments(&right_segment, PATH_RADIUS));

    PathSkeleton { segments }
}

fn rasterize_skeleton(width: u32, height: u32, skeleton: &PathSkeleton) -> Vec<BaseTile> {
    let mut cells = vec![BaseTile::Grass; (width * height) as usize];
    for segment in &skeleton.segments {
        rasterize_segment(width, height, segment, &mut cells);
    }
    cells
}

fn carve_path_segment_points(
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    width: u32,
    height: u32,
    rng: &mut StdRng,
) -> Vec<(i32, i32)> {
    let mut segment = Vec::new();
    let mut x = start_x;
    let mut y = start_y;
    let end_x_i = end_x;

    segment.push((x, y));
    let max_steps = (width * height * 4) as usize;
    let mut steps = 0usize;

    while y != end_y && steps < max_steps {
        steps += 1;
        if x != end_x_i && rng.gen_bool(0.45) {
            x += if end_x_i > x { 1 } else { -1 };
        } else {
            y += if end_y > y { 1 } else { -1 };
        }
        x = x.clamp(0, width.saturating_sub(1) as i32);
        y = y.clamp(0, height.saturating_sub(1) as i32);
        segment.push((x, y));
    }

    while x != end_x_i && steps < max_steps {
        steps += 1;
        x += if end_x_i > x { 1 } else { -1 };
        x = x.clamp(0, width.saturating_sub(1) as i32);
        segment.push((x, y));
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
        let skeleton = generate_path_skeleton(width, height, &mut rng);
        let total_length: i32 = skeleton
            .segments
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
