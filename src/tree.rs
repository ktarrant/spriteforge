use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalized(self) -> Self {
        let len = self.length();
        if len <= f32::EPSILON {
            return Self::default();
        }
        Self::new(self.x / len, self.y / len, self.z / len)
    }
}

impl std::ops::Add for Vec3 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

#[derive(Debug, Clone)]
pub struct TreeSegment {
    pub start: Vec3,
    pub end: Vec3,
    pub radius: f32,
    pub normal: Vec3,
}

#[derive(Debug, Clone)]
pub struct TreeLeaf {
    pub position: Vec3,
    pub size: f32,
    pub normal: Vec3,
}

#[derive(Debug, Clone, Default)]
pub struct TreeModel {
    pub segments: Vec<TreeSegment>,
    pub leaves: Vec<TreeLeaf>,
}

#[derive(Debug, Clone)]
pub struct TreeSettings {
    pub trunk_height: f32,
    pub crown_radius: f32,
    pub crown_height: f32,
    pub attraction_points: u32,
    pub segment_length: f32,
    pub influence_distance: f32,
    pub kill_distance: f32,
    pub max_iterations: u32,
    pub base_radius: f32,
    pub leaf_size: f32,
    pub max_leaves: u32,
}

impl Default for TreeSettings {
    fn default() -> Self {
        Self {
            trunk_height: 4.0,
            crown_radius: 3.5,
            crown_height: 5.0,
            attraction_points: 280,
            segment_length: 0.5,
            influence_distance: 2.4,
            kill_distance: 0.7,
            max_iterations: 220,
            base_radius: 0.35,
            leaf_size: 0.55,
            max_leaves: 120,
        }
    }
}

#[derive(Debug, Clone)]
struct Node {
    position: Vec3,
    children: u32,
}

pub fn generate_tree(seed: u64, settings: &TreeSettings) -> TreeModel {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut attraction_points = Vec::with_capacity(settings.attraction_points as usize);

    for _ in 0..settings.attraction_points {
        let mut point = Vec3::default();
        for _ in 0..30 {
            let x = rng.gen_range(-1.0..1.0) * settings.crown_radius;
            let y = rng.gen_range(-1.0..1.0) * settings.crown_radius;
            if x * x + y * y > settings.crown_radius * settings.crown_radius {
                continue;
            }
            let z = rng.gen_range(0.0..settings.crown_height);
            point = Vec3::new(x, y, z + settings.trunk_height);
            break;
        }
        attraction_points.push(point);
    }
    let initial_attraction_points = attraction_points.clone();

    let mut nodes = Vec::new();
    nodes.push(Node {
        position: Vec3::new(0.0, 0.0, 0.0),
        children: 1,
    });
    nodes.push(Node {
        position: Vec3::new(0.0, 0.0, settings.trunk_height),
        children: 0,
    });

    let mut segments = Vec::new();
    segments.push(TreeSegment {
        start: nodes[0].position,
        end: nodes[1].position,
        radius: settings.base_radius,
        normal: Vec3::default(),
    });

    let mut iter = 0;
    while !attraction_points.is_empty() && iter < settings.max_iterations {
        iter += 1;

        let mut direction_sums = vec![Vec3::default(); nodes.len()];
        let mut direction_counts = vec![0u32; nodes.len()];
        let mut remaining_points = Vec::with_capacity(attraction_points.len());

        for point in attraction_points.into_iter() {
            let mut closest = None;
            let mut closest_dist = f32::MAX;
            for (idx, node) in nodes.iter().enumerate() {
                let delta = point - node.position;
                let dist = delta.length();
                if dist < closest_dist {
                    closest_dist = dist;
                    closest = Some((idx, delta));
                }
            }

            if closest_dist <= settings.kill_distance {
                continue;
            }
            if let Some((idx, delta)) = closest {
                if closest_dist <= settings.influence_distance {
                    direction_sums[idx] = direction_sums[idx] + delta.normalized();
                    direction_counts[idx] += 1;
                }
            }
            remaining_points.push(point);
        }

        attraction_points = remaining_points;

        let mut new_nodes = Vec::new();
        for (idx, count) in direction_counts.iter().enumerate() {
            if *count == 0 {
                continue;
            }
            let direction = direction_sums[idx] * (1.0 / (*count as f32));
            let new_pos = nodes[idx].position + direction.normalized() * settings.segment_length;
            new_nodes.push((idx, new_pos));
        }

        if new_nodes.is_empty() {
            break;
        }

        for (parent_idx, position) in new_nodes {
            nodes[parent_idx].children += 1;
            nodes.push(Node {
                position,
                children: 0,
            });
            segments.push(TreeSegment {
                start: nodes[parent_idx].position,
                end: position,
                radius: 0.0,
                normal: Vec3::default(),
            });
        }
    }

    let max_height = (settings.trunk_height + settings.crown_height).max(0.001);
    for segment in segments.iter_mut() {
        let t = (segment.end.z / max_height).clamp(0.0, 1.0);
        segment.radius = settings.base_radius * (1.0 - t).powf(0.7).max(0.15);
    }

    let tree_center = compute_tree_center(&segments, &nodes);
    for segment in segments.iter_mut() {
        let mid = Vec3::new(
            (segment.start.x + segment.end.x) * 0.5,
            (segment.start.y + segment.end.y) * 0.5,
            (segment.start.z + segment.end.z) * 0.5,
        );
        segment.normal = (mid - tree_center).normalized();
    }

    let mut leaves = Vec::with_capacity(initial_attraction_points.len());
    for point in initial_attraction_points {
        leaves.push(TreeLeaf {
            position: point,
            size: settings.leaf_size,
            normal: (point - tree_center).normalized(),
        });
    }

    TreeModel { segments, leaves }
}

fn compute_tree_center(segments: &[TreeSegment], nodes: &[Node]) -> Vec3 {
    let mut min = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
    let mut max = Vec3::new(f32::MIN, f32::MIN, f32::MIN);

    for segment in segments {
        expand_bounds(segment.start, segment.radius, &mut min, &mut max);
        expand_bounds(segment.end, segment.radius, &mut min, &mut max);
    }

    for node in nodes {
        expand_bounds(node.position, 0.0, &mut min, &mut max);
    }

    if !min.x.is_finite() {
        return Vec3::default();
    }

    Vec3::new(
        (min.x + max.x) * 0.5,
        (min.y + max.y) * 0.5,
        (min.z + max.z) * 0.5,
    )
}

fn expand_bounds(point: Vec3, radius: f32, min: &mut Vec3, max: &mut Vec3) {
    let r = radius.max(0.0);
    min.x = min.x.min(point.x - r);
    min.y = min.y.min(point.y - r);
    min.z = min.z.min(point.z - r);
    max.x = max.x.max(point.x + r);
    max.y = max.y.max(point.y + r);
    max.z = max.z.max(point.z + r);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_generation_is_deterministic() {
        let settings = TreeSettings::default();
        let a = generate_tree(42, &settings);
        let b = generate_tree(42, &settings);
        assert_eq!(a.segments.len(), b.segments.len());
        assert_eq!(a.leaves.len(), b.leaves.len());
        assert_eq!(a.segments[0].start, b.segments[0].start);
        assert_eq!(a.segments[0].end, b.segments[0].end);
    }

    #[test]
    fn tree_has_segments_and_leaves() {
        let settings = TreeSettings::default();
        let model = generate_tree(7, &settings);
        assert!(!model.segments.is_empty());
        assert!(!model.leaves.is_empty());
    }
}
