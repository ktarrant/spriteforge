use rand::rngs::StdRng;
use rand::Rng;
use std::collections::HashSet;

use crate::BaseTile;

const PATH_RADIUS: i32 = 1;
const BRANCH_RADIUS: i32 = 0;
const BRANCH_LENGTH_MIN: i32 = 8;
const BRANCH_LENGTH_MAX: i32 = 12;
const BRANCHES_PER_TRUNK: usize = 2;
const BRANCH_CLEARANCE: i32 = 3;
const CAPILLARY_LENGTH_MIN: i32 = 4;
const CAPILLARY_LENGTH_STEP: i32 = 2;
const BRANCH_SET_ATTEMPTS: usize = 12;
const BRANCH_START_ATTEMPTS: usize = 24;

#[derive(Clone, Copy)]
struct PathPoint {
    x: i32,
    y: i32,
    radius: i32,
}

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

#[derive(Clone, Copy)]
struct BranchSpec {
    start_x: i32,
    start_y: i32,
    dir_x: i32,
    dir_y: i32,
    length: i32,
}

pub fn generate_path_map(width: u32, height: u32, rng: &mut StdRng) -> Vec<BaseTile> {
    let skeleton = generate_path_skeleton(width, height, rng);
    rasterize_skeleton(width, height, &skeleton)
}

fn generate_path_skeleton(width: u32, height: u32, rng: &mut StdRng) -> PathSkeleton {
    let mut cells = vec![BaseTile::Grass; (width * height) as usize];
    if width == 0 || height == 0 {
        return PathSkeleton { segments: Vec::new() };
    }

    let start_x = width.saturating_sub(1);
    let start_y = 0;
    let end_left = 0;
    let end_right = width / 2;
    let fork_x = width / 2;
    let fork_y = height / 2;
    let exit_y = height / 2;
    let dead_end_x = (width as f32 * 0.1).round() as u32;
    let dead_end_y = (height as f32 * 0.9).round() as u32;

    let mut path = Vec::new();
    let mut occupied = HashSet::new();
    let mut segments = Vec::new();

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
    add_segment(&mut path, &mut occupied, &main_segment, PATH_RADIUS);
    segments.extend(points_to_segments(&main_segment, PATH_RADIUS));

    let right_segment = carve_path_segment_points(
        fork_px,
        fork_py,
        end_right as i32,
        height.saturating_sub(1) as i32,
        width,
        height,
        rng,
    );
    add_segment(&mut path, &mut occupied, &right_segment, PATH_RADIUS);
    segments.extend(points_to_segments(&right_segment, PATH_RADIUS));

    let left_segment = carve_path_segment_points(
        fork_px,
        fork_py,
        end_left as i32,
        exit_y as i32,
        width,
        height,
        rng,
    );
    add_segment(&mut path, &mut occupied, &left_segment, PATH_RADIUS);
    segments.extend(points_to_segments(&left_segment, PATH_RADIUS));

    let dead_segment = carve_path_segment_points(
        fork_px,
        fork_py,
        dead_end_x as i32,
        dead_end_y as i32,
        width,
        height,
        rng,
    );
    add_segment(&mut path, &mut occupied, &dead_segment, PATH_RADIUS);
    segments.extend(points_to_segments(&dead_segment, PATH_RADIUS));

    if let Some(branches) = select_branch_set(
        &main_segment,
        &left_segment,
        &right_segment,
        &occupied,
        width,
        height,
        rng,
    ) {
        for branch in branches {
            apply_branch(
                &mut path,
                &mut occupied,
                &branch,
                BRANCH_CLEARANCE,
                width,
                height,
            );
            segments.push(PathSegment {
                start_x: branch.start_x,
                start_y: branch.start_y,
                end_x: branch.start_x + branch.dir_x * branch.length,
                end_y: branch.start_y + branch.dir_y * branch.length,
                radius: BRANCH_RADIUS,
            });
        }
    }

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

fn add_segment(
    path: &mut Vec<PathPoint>,
    occupied: &mut HashSet<(i32, i32)>,
    segment: &[(i32, i32)],
    radius: i32,
) {
    for &(x, y) in segment {
        path.push(PathPoint { x, y, radius });
        occupy_cell(occupied, x, y);
    }
}

fn is_edge(x: i32, y: i32, width: u32, height: u32) -> bool {
    if width == 0 || height == 0 {
        return false;
    }
    let max_x = width.saturating_sub(1) as i32;
    let max_y = height.saturating_sub(1) as i32;
    x <= 0 || y <= 0 || x >= max_x || y >= max_y
}

fn select_branch_set(
    main_segment: &[(i32, i32)],
    left_segment: &[(i32, i32)],
    right_segment: &[(i32, i32)],
    occupied: &HashSet<(i32, i32)>,
    width: u32,
    height: u32,
    rng: &mut StdRng,
) -> Option<Vec<BranchSpec>> {
    if width <= 2 || height <= 2 {
        return None;
    }
    let trunks = [main_segment, left_segment, right_segment];
    let sides = [-1, 1];
    for _ in 0..BRANCH_SET_ATTEMPTS {
        let mut specs = Vec::with_capacity(BRANCHES_PER_TRUNK * trunks.len());
        let mut valid = true;
        for trunk in trunks {
            for &side in &sides {
                if let Some(spec) = pick_branch_start(trunk, side, rng, width, height) {
                    specs.push(spec);
                } else {
                    valid = false;
                    break;
                }
            }
            if !valid {
                break;
            }
        }
        if !valid {
            continue;
        }
        let mut temp_occupied = occupied.clone();
        for spec in &specs {
            if !branch_fits(spec, &temp_occupied, width, height) {
                valid = false;
                break;
            }
            mark_branch_occupied(&mut temp_occupied, spec, BRANCH_CLEARANCE);
        }
        if valid {
            return Some(specs);
        }
    }
    None
}

fn pick_branch_start(
    trunk: &[(i32, i32)],
    side: i32,
    rng: &mut StdRng,
    width: u32,
    height: u32,
) -> Option<BranchSpec> {
    if trunk.len() < 2 {
        return None;
    }
    let max_start_index = trunk.len().saturating_sub(2);
    for _ in 0..BRANCH_START_ATTEMPTS {
        let start_idx = rng.gen_range(1..=max_start_index);
        let (sx, sy) = trunk[start_idx];
        if is_edge(sx, sy, width, height) {
            continue;
        }
        let (tx, ty) = trunk[start_idx + 1];
        let dir_x = tx - sx;
        let dir_y = ty - sy;
        if dir_x == 0 && dir_y == 0 {
            continue;
        }
        let (branch_dx, branch_dy) = if dir_x.abs() >= dir_y.abs() {
            (0, side)
        } else {
            (side, 0)
        };
        let max_length = max_length_in_direction(sx, sy, branch_dx, branch_dy, width, height);
        if max_length < BRANCH_LENGTH_MIN {
            continue;
        }
        let length = rng.gen_range(BRANCH_LENGTH_MIN..=BRANCH_LENGTH_MAX.min(max_length));
        return Some(BranchSpec {
            start_x: sx,
            start_y: sy,
            dir_x: branch_dx,
            dir_y: branch_dy,
            length,
        });
    }
    None
}

fn branch_fits(
    branch: &BranchSpec,
    occupied: &HashSet<(i32, i32)>,
    width: u32,
    height: u32,
) -> bool {
    let max_length = max_length_in_direction(
        branch.start_x,
        branch.start_y,
        branch.dir_x,
        branch.dir_y,
        width,
        height,
    );
    if branch.length > max_length {
        return false;
    }
    let max_x = width.saturating_sub(1) as i32;
    let max_y = height.saturating_sub(1) as i32;
    for step in 1..=branch.length {
        let x = branch.start_x + branch.dir_x * step;
        let y = branch.start_y + branch.dir_y * step;
        if x < 0 || y < 0 || x > max_x || y > max_y {
            return false;
        }
        if is_edge(x, y, width, height) || occupied.contains(&(x, y)) {
            return false;
        }
    }
    true
}

fn apply_branch(
    path: &mut Vec<PathPoint>,
    occupied: &mut HashSet<(i32, i32)>,
    branch: &BranchSpec,
    clearance: i32,
    width: u32,
    height: u32,
) {
    let mut end_x = branch.start_x;
    let mut end_y = branch.start_y;
    for step in 1..=branch.length {
        let x = branch.start_x + branch.dir_x * step;
        let y = branch.start_y + branch.dir_y * step;
        path.push(PathPoint {
            x,
            y,
            radius: BRANCH_RADIUS,
        });
        mark_with_clearance(occupied, x, y, clearance);
        end_x = x;
        end_y = y;
    }
    grow_capillaries(
        path,
        occupied,
        end_x,
        end_y,
        branch.dir_x,
        branch.dir_y,
        branch.length,
        clearance,
        width,
        height,
    );
}

fn mark_branch_occupied(
    occupied: &mut HashSet<(i32, i32)>,
    branch: &BranchSpec,
    clearance: i32,
) {
    for step in 1..=branch.length {
        let x = branch.start_x + branch.dir_x * step;
        let y = branch.start_y + branch.dir_y * step;
        mark_with_clearance(occupied, x, y, clearance);
    }
}

fn occupy_cell(occupied: &mut HashSet<(i32, i32)>, x: i32, y: i32) {
    occupied.insert((x, y));
}

fn mark_with_clearance(occupied: &mut HashSet<(i32, i32)>, x: i32, y: i32, clearance: i32) {
    for ny in (y - clearance)..=(y + clearance) {
        for nx in (x - clearance)..=(x + clearance) {
            occupy_cell(occupied, nx, ny);
        }
    }
}

fn max_length_in_direction(
    start_x: i32,
    start_y: i32,
    dir_x: i32,
    dir_y: i32,
    width: u32,
    height: u32,
) -> i32 {
    if dir_x == 0 && dir_y == 0 {
        return 0;
    }
    let max_x = width.saturating_sub(1) as i32;
    let max_y = height.saturating_sub(1) as i32;
    let mut length = 0;
    loop {
        let next_x = start_x + dir_x * (length + 1);
        let next_y = start_y + dir_y * (length + 1);
        if next_x < 0 || next_y < 0 || next_x > max_x || next_y > max_y {
            break;
        }
        if is_edge(next_x, next_y, width, height) {
            break;
        }
        length += 1;
    }
    length
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

fn grow_capillaries(
    path: &mut Vec<PathPoint>,
    occupied: &mut HashSet<(i32, i32)>,
    start_x: i32,
    start_y: i32,
    dir_x: i32,
    dir_y: i32,
    length: i32,
    clearance: i32,
    width: u32,
    height: u32,
) {
    let next_length = length.saturating_sub(CAPILLARY_LENGTH_STEP);
    if next_length < CAPILLARY_LENGTH_MIN {
        return;
    }
    let (fork_a, fork_b) = if dir_x.abs() >= dir_y.abs() {
        ((0, 1), (0, -1))
    } else {
        ((1, 0), (-1, 0))
    };
    for (fx, fy) in [fork_a, fork_b] {
        let branch = BranchSpec {
            start_x,
            start_y,
            dir_x: fx,
            dir_y: fy,
            length: next_length,
        };
        if branch_fits(&branch, occupied, width, height) {
            apply_branch(path, occupied, &branch, clearance, width, height);
        }
    }
}
