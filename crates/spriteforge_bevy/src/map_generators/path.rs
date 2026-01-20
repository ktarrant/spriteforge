use rand::rngs::StdRng;
use rand::Rng;
use std::collections::HashSet;

use crate::BaseTile;

const PATH_RADIUS: i32 = 1;
const BRANCH_RADIUS: i32 = 0;
const PRE_FORK_BRANCH_MIN: usize = 2;
const PRE_FORK_BRANCH_MAX: usize = 4;
const POST_FORK_BRANCH_MAX: usize = 2;
const BRANCH_LENGTH_MIN: i32 = 8;
const BRANCH_LENGTH_MAX: i32 = 12;
const BRANCH_ATTEMPTS: usize = 12;

#[derive(Clone, Copy)]
struct PathPoint {
    x: i32,
    y: i32,
    radius: i32,
}

pub fn generate_path_map(width: u32, height: u32, rng: &mut StdRng) -> Vec<BaseTile> {
    let mut cells = vec![BaseTile::Grass; (width * height) as usize];
    if width == 0 || height == 0 {
        return cells;
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

    let pre_fork_branches = rng.gen_range(PRE_FORK_BRANCH_MIN..=PRE_FORK_BRANCH_MAX);
    add_branches(
        &mut path,
        &mut occupied,
        &main_segment,
        pre_fork_branches,
        width,
        height,
        rng,
    );
    add_branches(
        &mut path,
        &mut occupied,
        &right_segment,
        rng.gen_range(0..=POST_FORK_BRANCH_MAX),
        width,
        height,
        rng,
    );
    add_branches(
        &mut path,
        &mut occupied,
        &left_segment,
        rng.gen_range(0..=POST_FORK_BRANCH_MAX),
        width,
        height,
        rng,
    );
    add_branches(
        &mut path,
        &mut occupied,
        &dead_segment,
        rng.gen_range(0..=POST_FORK_BRANCH_MAX),
        width,
        height,
        rng,
    );

    for point in path {
        let radius = point.radius;
        for ny in (point.y - radius)..=(point.y + radius) {
            for nx in (point.x - radius)..=(point.x + radius) {
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

fn carve_path_segment_checked(
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    width: u32,
    height: u32,
    occupied: &HashSet<(i32, i32)>,
    rng: &mut StdRng,
) -> Option<Vec<(i32, i32)>> {
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
        if is_edge(x, y, width, height) || occupied.contains(&(x, y)) {
            return None;
        }
        segment.push((x, y));
    }

    while x != end_x_i && steps < max_steps {
        steps += 1;
        x += if end_x_i > x { 1 } else { -1 };
        x = x.clamp(0, width.saturating_sub(1) as i32);
        if is_edge(x, y, width, height) || occupied.contains(&(x, y)) {
            return None;
        }
        segment.push((x, y));
    }

    if steps >= max_steps {
        return None;
    }
    Some(segment)
}

fn add_segment(
    path: &mut Vec<PathPoint>,
    occupied: &mut HashSet<(i32, i32)>,
    segment: &[(i32, i32)],
    radius: i32,
) {
    for &(x, y) in segment {
        path.push(PathPoint { x, y, radius });
        occupied.insert((x, y));
    }
}

fn add_branches(
    path: &mut Vec<PathPoint>,
    occupied: &mut HashSet<(i32, i32)>,
    trunk: &[(i32, i32)],
    count: usize,
    width: u32,
    height: u32,
    rng: &mut StdRng,
) {
    if trunk.len() < 2 || count == 0 || width <= 2 || height <= 2 {
        return;
    }
    let min_x = 1;
    let min_y = 1;
    let max_x = width.saturating_sub(2) as i32;
    let max_y = height.saturating_sub(2) as i32;
    let max_start_index = trunk.len().saturating_sub(2);
    for _ in 0..count {
        let mut carved = None;
        for _ in 0..BRANCH_ATTEMPTS {
            let start_idx = rng.gen_range(0..=max_start_index);
            let (sx, sy) = trunk[start_idx];
            if is_edge(sx, sy, width, height) {
                continue;
            }
            let (tx, ty) = if start_idx + 1 < trunk.len() {
                trunk[start_idx + 1]
            } else {
                trunk[start_idx.saturating_sub(1)]
            };
            let dir_x = tx - sx;
            let dir_y = ty - sy;
            let length = rng.gen_range(BRANCH_LENGTH_MIN..=BRANCH_LENGTH_MAX);
            let (dx, dy) = if dir_x.abs() >= dir_y.abs() {
                let sign = if rng.gen_bool(0.5) { 1 } else { -1 };
                (0, sign * length)
            } else {
                let sign = if rng.gen_bool(0.5) { 1 } else { -1 };
                (sign * length, 0)
            };
            if dx == 0 && dy == 0 {
                continue;
            }
            let mut ex = (sx + dx).clamp(min_x, max_x);
            let mut ey = (sy + dy).clamp(min_y, max_y);
            if ex == sx && ey == sy {
                continue;
            }
            if occupied.contains(&(ex, ey)) {
                continue;
            }
            if let Some(segment) = carve_path_segment_checked(
                sx,
                sy,
                ex,
                ey,
                width,
                height,
                occupied,
                rng,
            ) {
                carved = Some(segment);
                break;
            }
        }
        if let Some(segment) = carved {
            add_segment(path, occupied, &segment, BRANCH_RADIUS);
        }
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
