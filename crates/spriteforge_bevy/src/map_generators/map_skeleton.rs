use rand::rngs::StdRng;
use rand::Rng;
use serde::Deserialize;
use std::path::Path;

const DEFAULT_CONFIG_PATH: &str = "assets/map_skeleton.json";
const PATH_RADIUS: i32 = 1;
const CONNECTOR_RADIUS: i32 = 0;
const DOCK_CHANCE: f64 = 0.25;

#[derive(Clone, Copy, Debug)]
pub struct PathSegment {
    pub start_x: i32,
    pub start_y: i32,
    pub end_x: i32,
    pub end_y: i32,
    pub radius: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaType {
    Dock,
}

#[derive(Clone, Copy, Debug)]
pub struct MapArea {
    pub center_x: i32,
    pub center_y: i32,
    pub radius: i32,
    pub area_type: Option<AreaType>,
}

#[derive(Clone, Debug)]
pub struct MapSkeleton {
    pub paths: Vec<PathSegment>,
    pub areas: Vec<MapArea>,
    pub water_paths: Vec<PathSegment>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct MapPointConfig {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct MapAreaConfig {
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub major: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MapSkeletonConfig {
    pub entry: MapPointConfig,
    pub fork: MapPointConfig,
    pub exits: Vec<MapPointConfig>,
    pub areas: Vec<MapAreaConfig>,
}

pub fn generate_map_skeleton(width: u32, height: u32, rng: &mut StdRng) -> MapSkeleton {
    if width == 0 || height == 0 {
        return MapSkeleton {
            paths: Vec::new(),
            areas: Vec::new(),
            water_paths: Vec::new(),
        };
    }

    let config = load_map_skeleton_config(Path::new(DEFAULT_CONFIG_PATH))
        .unwrap_or_else(|_| default_map_skeleton_config());
    generate_map_skeleton_with_config(width, height, rng, &config)
}

pub fn generate_map_skeleton_with_config(
    width: u32,
    height: u32,
    rng: &mut StdRng,
    config: &MapSkeletonConfig,
) -> MapSkeleton {
    let width_i = width as i32;
    let height_i = height as i32;
    let (entry_x, entry_y) = resolve_point(config.entry, width_i, height_i);
    let (fork_x, fork_y) = resolve_point(config.fork, width_i, height_i);
    let exit_points: Vec<(i32, i32)> = config
        .exits
        .iter()
        .copied()
        .map(|point| resolve_point(point, width_i, height_i))
        .collect();
    let exit_points = if exit_points.is_empty() {
        vec![(0, height_i / 2)]
    } else {
        exit_points
    };

    let mut areas = build_areas(width_i, height_i, rng, &config.areas);
    let mut main_segment = Vec::new();
    let mut fork_segments: Vec<Vec<(i32, i32)>> = Vec::new();

    for _ in 0..6 {
        let area_occupied = build_area_occupancy(width_i, height_i, &areas);
        main_segment = carve_path_segment_points_avoiding(
            entry_x,
            entry_y,
            fork_x,
            fork_y,
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
        let (fork_px, fork_py) = *main_segment.last().unwrap_or(&(entry_x, entry_y));
        fork_segments = build_fork_segments(
            fork_px,
            fork_py,
            &exit_points,
            width,
            height,
            rng,
            &area_occupied,
        );
        if fork_segments.iter().all(|segment| !segment.is_empty()) {
            break;
        }
        shrink_areas(&mut areas);
    }

    if main_segment.is_empty() || fork_segments.iter().any(|segment| segment.is_empty()) {
        areas.clear();
        let area_occupied = build_area_occupancy(width_i, height_i, &areas);
        main_segment = carve_path_segment_points_avoiding(
            entry_x,
            entry_y,
            fork_x,
            fork_y,
            width,
            height,
            rng,
            &area_occupied,
            (0, 0),
        );
        let (fork_px, fork_py) = *main_segment.last().unwrap_or(&(entry_x, entry_y));
        fork_segments = build_fork_segments(
            fork_px,
            fork_py,
            &exit_points,
            width,
            height,
            rng,
            &area_occupied,
        );
    }

    let mut paths = Vec::new();
    paths.extend(points_to_segments(&main_segment, PATH_RADIUS));
    for segment in &fork_segments {
        paths.extend(points_to_segments(segment, PATH_RADIUS));
    }

    let water_paths = build_dock_paths(width_i, height_i, &areas, rng);

    if !areas.is_empty() {
        let fork_point = main_segment
            .last()
            .copied()
            .unwrap_or((fork_x, fork_y));
        let connector_targets = connector_targets_from_config(config, width_i, height_i);
        let mut used_areas = Vec::new();
        for (target_point, target) in connector_targets {
            let area_index = find_nearest_area_index(&areas, target_point, &used_areas)
                .or_else(|| find_nearest_area_index(&areas, target_point, &[]));
            let Some(area_index) = area_index else {
                continue;
            };
            if !used_areas.contains(&area_index) {
                used_areas.push(area_index);
            }
            let area = areas[area_index];
            let start = (area.center_x, area.center_y);
            let end = match target {
                ConnectorTarget::LeftFork | ConnectorTarget::RightFork => {
                    find_nearest_point_on_segments(&fork_segments, start)
                }
                ConnectorTarget::MainPath => find_nearest_point(&main_segment, start),
                ConnectorTarget::ForkPoint => Some(fork_point),
            };
            let Some(end) = end else {
                continue;
            };
            let connector_points = carve_connector_points(
                start,
                end,
                width_i,
                height_i,
                rng,
                &areas,
                area_index,
            );
            paths.extend(points_to_segments(&connector_points, CONNECTOR_RADIUS));
        }
    }

    MapSkeleton {
        paths,
        areas,
        water_paths,
    }
}

pub fn load_map_skeleton_config(path: &Path) -> Result<MapSkeletonConfig, String> {
    let data = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

fn default_map_skeleton_config() -> MapSkeletonConfig {
    MapSkeletonConfig {
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
            },
            MapAreaConfig {
                x: 1.0 / 2.0,
                y: 1.0 / 5.0,
                major: false,
            },
            MapAreaConfig {
                x: 3.0 / 4.0,
                y: 5.0 / 6.0,
                major: false,
            },
            MapAreaConfig {
                x: 3.0 / 4.0,
                y: 1.0 / 2.0,
                major: false,
            },
            MapAreaConfig {
                x: 1.0 / 4.0,
                y: 3.0 / 4.0,
                major: true,
            },
        ],
    }
}

fn resolve_point(point: MapPointConfig, width: i32, height: i32) -> (i32, i32) {
    let width_f = (width.saturating_sub(1) as f32).max(0.0);
    let height_f = (height.saturating_sub(1) as f32).max(0.0);
    let x = (point.x.clamp(0.0, 1.0) * width_f).round() as i32;
    let y = (point.y.clamp(0.0, 1.0) * height_f).round() as i32;
    (x.clamp(0, width.saturating_sub(1)), y.clamp(0, height.saturating_sub(1)))
}

fn build_areas(
    width: i32,
    height: i32,
    rng: &mut StdRng,
    area_configs: &[MapAreaConfig],
) -> Vec<MapArea> {
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

    let mut areas = Vec::new();
    for config in area_configs {
        let (target_x, target_y) = resolve_point(
            MapPointConfig {
                x: config.x,
                y: config.y,
            },
            width,
            height,
        );
        let base_radius = if config.major { major_radius } else { minor_radius };
        let min_radius = if config.major {
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
                    let area_type = if config.major {
                        None
                    } else if rng.gen_bool(DOCK_CHANCE) {
                        Some(AreaType::Dock)
                    } else {
                        None
                    };
                    placed = Some(MapArea {
                        center_x: cx,
                        center_y: cy,
                        radius,
                        area_type,
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

fn connector_targets_from_config(
    config: &MapSkeletonConfig,
    width: i32,
    height: i32,
) -> Vec<((i32, i32), ConnectorTarget)> {
    let mut targets = Vec::new();
    let connector_roles = [
        ConnectorTarget::LeftFork,
        ConnectorTarget::MainPath,
        ConnectorTarget::RightFork,
        ConnectorTarget::MainPath,
        ConnectorTarget::ForkPoint,
    ];
    for (index, role) in connector_roles.iter().enumerate() {
        if let Some(area) = config.areas.get(index) {
            let point = resolve_point(
                MapPointConfig { x: area.x, y: area.y },
                width,
                height,
            );
            targets.push((point, *role));
        }
    }
    targets
}

fn build_fork_segments(
    fork_x: i32,
    fork_y: i32,
    exit_points: &[(i32, i32)],
    width: u32,
    height: u32,
    rng: &mut StdRng,
    area_occupied: &[bool],
) -> Vec<Vec<(i32, i32)>> {
    exit_points
        .iter()
        .map(|&(exit_x, exit_y)| {
            let bias_dir = ((exit_x - fork_x).signum(), (exit_y - fork_y).signum());
            carve_path_segment_points_avoiding(
                fork_x,
                fork_y,
                exit_x,
                exit_y,
                width,
                height,
                rng,
                area_occupied,
                bias_dir,
            )
        })
        .collect()
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

fn build_dock_paths(
    width: i32,
    height: i32,
    areas: &[MapArea],
    rng: &mut StdRng,
) -> Vec<PathSegment> {
    let mut segments = Vec::new();
    for (idx, area) in areas.iter().enumerate() {
        if area.area_type != Some(AreaType::Dock) {
            continue;
        }
        let edge_point = nearest_edge_point(area.center_x, area.center_y, width, height);
        let points = carve_connector_points(
            (area.center_x, area.center_y),
            edge_point,
            width,
            height,
            rng,
            areas,
            idx,
        );
        segments.extend(points_to_segments(&points, CONNECTOR_RADIUS));
    }
    segments
}

fn nearest_edge_point(x: i32, y: i32, width: i32, height: i32) -> (i32, i32) {
    let left = x;
    let right = (width - 1) - x;
    let top = y;
    let bottom = (height - 1) - y;
    let min_dist = left.min(right).min(top).min(bottom);
    if min_dist == left {
        (0, y)
    } else if min_dist == right {
        (width - 1, y)
    } else if min_dist == top {
        (x, 0)
    } else {
        (x, height - 1)
    }
}

fn find_nearest_area_index(
    areas: &[MapArea],
    target: (i32, i32),
    used: &[usize],
) -> Option<usize> {
    let mut best = None;
    let mut best_dist = i32::MAX;
    for (idx, area) in areas.iter().enumerate() {
        if used.contains(&idx) {
            continue;
        }
        let dx = area.center_x - target.0;
        let dy = area.center_y - target.1;
        let dist = dx * dx + dy * dy;
        if dist < best_dist {
            best_dist = dist;
            best = Some(idx);
        }
    }
    best
}

fn find_nearest_point(points: &[(i32, i32)], target: (i32, i32)) -> Option<(i32, i32)> {
    let mut best = None;
    let mut best_dist = i32::MAX;
    for &(x, y) in points {
        let dx = x - target.0;
        let dy = y - target.1;
        let dist = dx * dx + dy * dy;
        if dist < best_dist {
            best_dist = dist;
            best = Some((x, y));
        }
    }
    best
}

fn find_nearest_point_on_segments(
    segments: &[Vec<(i32, i32)>],
    target: (i32, i32),
) -> Option<(i32, i32)> {
    let mut best = None;
    let mut best_dist = i32::MAX;
    for segment in segments {
        if let Some(point) = find_nearest_point(segment, target) {
            let dx = point.0 - target.0;
            let dy = point.1 - target.1;
            let dist = dx * dx + dy * dy;
            if dist < best_dist {
                best_dist = dist;
                best = Some(point);
            }
        }
    }
    best
}

fn carve_connector_points(
    start: (i32, i32),
    end: (i32, i32),
    width: i32,
    height: i32,
    rng: &mut StdRng,
    areas: &[MapArea],
    allowed_area: usize,
) -> Vec<(i32, i32)> {
    let mut segment = Vec::new();
    let mut x = start.0;
    let mut y = start.1;
    let mut last_dir = (0, 0);
    let max_steps = (width * height * 4).max(1) as usize;
    let mut steps = 0usize;

    segment.push((x, y));
    while (x, y) != end && steps < max_steps {
        steps += 1;
        let dx = (end.0 - x).signum();
        let dy = (end.1 - y).signum();
        let mut moves = Vec::with_capacity(4);
        moves.push((dx, 0));
        moves.push((0, dy));
        if rng.gen_bool(0.45) {
            moves.swap(0, 1);
        }
        if moves.len() > 1 && rng.gen_bool(0.35) {
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
            if nx < 0 || ny < 0 || nx >= width || ny >= height {
                continue;
            }
            if is_blocked(nx, ny, areas, allowed_area) {
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
            if try_detour_connector(
                &mut x,
                &mut y,
                &mut last_dir,
                width,
                height,
                areas,
                allowed_area,
                &mut segment,
            ) {
                continue;
            }
            break;
        }
    }

    segment
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

fn is_blocked(x: i32, y: i32, areas: &[MapArea], allowed_area: usize) -> bool {
    for (idx, area) in areas.iter().enumerate() {
        if idx == allowed_area {
            continue;
        }
        let dx = x - area.center_x;
        let dy = y - area.center_y;
        if dx * dx + dy * dy <= area.radius * area.radius {
            return true;
        }
    }
    false
}

fn try_detour_connector(
    x: &mut i32,
    y: &mut i32,
    last_dir: &mut (i32, i32),
    width: i32,
    height: i32,
    areas: &[MapArea],
    allowed_area: usize,
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
        if is_blocked(nx, ny, areas, allowed_area) {
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

#[derive(Clone, Copy)]
enum ConnectorTarget {
    LeftFork,
    MainPath,
    RightFork,
    ForkPoint,
}
