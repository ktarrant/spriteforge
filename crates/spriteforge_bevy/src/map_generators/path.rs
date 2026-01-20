use rand::rngs::StdRng;
use rand::Rng;

use crate::BaseTile;

const PATH_RADIUS: i32 = 1;

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

    let mut path = Vec::new();
    let (fork_px, fork_py) = carve_path_segment(
        &mut path,
        start_x as i32,
        start_y as i32,
        fork_x as i32,
        fork_y as i32,
        width,
        height,
        rng,
    );
    carve_path_segment(
        &mut path,
        fork_px,
        fork_py,
        end_right as i32,
        height.saturating_sub(1) as i32,
        width,
        height,
        rng,
    );
    carve_path_segment(
        &mut path,
        fork_px,
        fork_py,
        end_left as i32,
        exit_y as i32,
        width,
        height,
        rng,
    );

    for (px, py) in path {
        for ny in (py - PATH_RADIUS)..=(py + PATH_RADIUS) {
            for nx in (px - PATH_RADIUS)..=(px + PATH_RADIUS) {
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

fn carve_path_segment(
    path: &mut Vec<(i32, i32)>,
    start_x: i32,
    start_y: i32,
    end_x: i32,
    end_y: i32,
    width: u32,
    height: u32,
    rng: &mut StdRng,
) -> (i32, i32) {
    let mut x = start_x;
    let mut y = start_y;
    let end_x_i = end_x;

    path.push((x, y));
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
        path.push((x, y));
    }

    while x != end_x_i && steps < max_steps {
        steps += 1;
        x += if end_x_i > x { 1 } else { -1 };
        x = x.clamp(0, width.saturating_sub(1) as i32);
        path.push((x, y));
    }
    (x, y)
}
