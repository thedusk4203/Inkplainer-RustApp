use crate::model::{AnimationStyle, Layer, Project};

#[derive(Debug, Clone, Copy)]
pub struct RevealRegion {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

impl RevealRegion {
    pub fn full() -> Self {
        Self { u0: 0.0, v0: 0.0, u1: 1.0, v1: 1.0 }
    }
}

pub fn layer_progress(project: &Project, layer_index: usize, global_progress: f32) -> f32 {
    if layer_index >= project.layers.len() || project.layers.is_empty() {
        return 0.0;
    }

    let mut group_keys: Vec<GroupKey> = Vec::new();
    for (idx, layer) in project.layers.iter().enumerate().filter(|(_, l)| l.visible) {
        let key = layer.anim_order.map(GroupKey::Ordered).unwrap_or(GroupKey::Unordered(idx));
        if !group_keys.contains(&key) {
            group_keys.push(key);
        }
    }
    group_keys.sort_by(|a, b| a.sort_key().partial_cmp(&b.sort_key()).unwrap_or(std::cmp::Ordering::Equal));

    let layer = &project.layers[layer_index];
    let target = layer.anim_order.map(GroupKey::Ordered).unwrap_or(GroupKey::Unordered(layer_index));
    let group_index = group_keys.iter().position(|k| *k == target).unwrap_or(0);
    let count = group_keys.len().max(1) as f32;
    (global_progress.clamp(0.0, 1.0) * count - group_index as f32).clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupKey {
    Ordered(u32),
    Unordered(usize),
}

impl GroupKey {
    fn sort_key(self) -> f64 {
        match self {
            Self::Ordered(v) => v as f64,
            Self::Unordered(i) => 1_000_000.0 + i as f64,
        }
    }
}

pub fn reveal_regions(layer: &Layer, progress: f32) -> Vec<RevealRegion> {
    let p = progress.clamp(0.0, 1.0);
    if p <= 0.0 {
        return Vec::new();
    }
    if p >= 1.0 {
        return vec![RevealRegion::full()];
    }

    match layer.animation.style {
        AnimationStyle::Scanner => scanner_regions(p, layer.animation.zigzag),
        AnimationStyle::ChunkJump | AnimationStyle::OutlineChunks => {
            chunk_regions(layer, p, Priority::Random)
        }
        AnimationStyle::Human => chunk_regions(layer, p, Priority::Human),
        AnimationStyle::Animal | AnimationStyle::Vehicle => chunk_regions(layer, p, Priority::LeftToRight),
        AnimationStyle::Portrait => chunk_regions(layer, p, Priority::CenterOut),
        AnimationStyle::Building => chunk_regions(layer, p, Priority::BottomUp),
        AnimationStyle::Landscape => chunk_regions(layer, p, Priority::TopDown),
        AnimationStyle::Spiral => chunk_regions(layer, p, Priority::Spiral),
        AnimationStyle::Contour
        | AnimationStyle::OutlineFill
        | AnimationStyle::IllustFill
        | AnimationStyle::OutlineOnly
        | AnimationStyle::TextDraw => vec![RevealRegion::full()],
    }
}

fn scanner_regions(progress: f32, zigzag: bool) -> Vec<RevealRegion> {
    const BANDS: usize = 12;
    let scaled = progress * BANDS as f32;
    let full_bands = scaled.floor() as usize;
    let partial = scaled.fract();
    let mut out = Vec::with_capacity(full_bands + 1);

    for band in 0..full_bands.min(BANDS) {
        let v0 = band as f32 / BANDS as f32;
        let v1 = (band + 1) as f32 / BANDS as f32;
        out.push(RevealRegion { u0: 0.0, v0, u1: 1.0, v1 });
    }

    if full_bands < BANDS && partial > 0.0 {
        let v0 = full_bands as f32 / BANDS as f32;
        let v1 = (full_bands + 1) as f32 / BANDS as f32;
        if zigzag && full_bands % 2 == 1 {
            out.push(RevealRegion { u0: 1.0 - partial, v0, u1: 1.0, v1 });
        } else {
            out.push(RevealRegion { u0: 0.0, v0, u1: partial, v1 });
        }
    }
    out
}

#[derive(Debug, Clone, Copy)]
enum Priority {
    Random,
    Human,
    LeftToRight,
    CenterOut,
    BottomUp,
    TopDown,
    Spiral,
}

#[derive(Debug, Clone)]
struct Tile {
    region: RevealRegion,
    score: f32,
    tie: u64,
}

fn chunk_regions(layer: &Layer, progress: f32, priority: Priority) -> Vec<RevealRegion> {
    let n = layer.animation.chunks.clamp(6, 80) as usize;
    let cols = (n as f32).sqrt().ceil().max(2.0) as usize;
    let rows = ((n as f32 / cols as f32).ceil() as usize).max(2);
    let seed = stable_hash(&layer.id);

    let mut tiles = Vec::with_capacity(cols * rows);
    for row in 0..rows {
        for col in 0..cols {
            let cx = (col as f32 + 0.5) / cols as f32;
            let cy = (row as f32 + 0.5) / rows as f32;
            let score = match priority {
                Priority::Random => pseudo(seed, row as u64 * 131 + col as u64),
                Priority::Human => {
                    let center_bias = (cx - 0.5).abs() * 0.18;
                    cy + center_bias
                }
                Priority::LeftToRight => cx + cy * 0.08,
                Priority::CenterOut => ((cx - 0.5).powi(2) + (cy - 0.45).powi(2)).sqrt(),
                Priority::BottomUp => (1.0 - cy) + (cx - 0.5).abs() * 0.04,
                Priority::TopDown => cy + (cx - 0.5).abs() * 0.04,
                Priority::Spiral => {
                    let dx = cx - 0.5;
                    let dy = cy - 0.5;
                    let radius = (dx * dx + dy * dy).sqrt();
                    let angle = dy.atan2(dx) / std::f32::consts::TAU + 0.5;
                    radius + angle * 0.20
                }
            };
            tiles.push(Tile {
                region: RevealRegion {
                    u0: col as f32 / cols as f32,
                    v0: row as f32 / rows as f32,
                    u1: (col + 1) as f32 / cols as f32,
                    v1: (row + 1) as f32 / rows as f32,
                },
                score,
                tie: mix64(seed ^ ((row as u64) << 32) ^ col as u64),
            });
        }
    }

    tiles.sort_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.tie.cmp(&b.tie))
    });

    let visible = ((tiles.len() as f32 * progress).ceil() as usize).min(tiles.len());
    tiles.into_iter().take(visible).map(|t| t.region).collect()
}

fn stable_hash(text: &str) -> u64 {
    let mut h = 0xcbf29ce484222325u64;
    for byte in text.as_bytes() {
        h ^= *byte as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn mix64(mut x: u64) -> u64 {
    x ^= x >> 30;
    x = x.wrapping_mul(0xbf58476d1ce4e5b9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

fn pseudo(seed: u64, index: u64) -> f32 {
    let v = mix64(seed ^ index.wrapping_mul(0x9e3779b97f4a7c15));
    (v as f64 / u64::MAX as f64) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Layer, Project};

    #[test]
    fn equal_orders_share_timeline_group() {
        let mut p = Project::default();
        let mut a = Layer::new_text(p.canvas.width, p.canvas.height);
        let mut b = Layer::new_text(p.canvas.width, p.canvas.height);
        a.anim_order = Some(1);
        b.anim_order = Some(1);
        p.layers = vec![a, b];
        assert!((layer_progress(&p, 0, 0.5) - layer_progress(&p, 1, 0.5)).abs() < f32::EPSILON);
    }

    #[test]
    fn ordered_layer_runs_before_unordered_layer() {
        let mut p = Project::default();
        let mut a = Layer::new_text(p.canvas.width, p.canvas.height);
        let b = Layer::new_text(p.canvas.width, p.canvas.height);
        a.anim_order = Some(1);
        p.layers = vec![a, b];
        assert_eq!(layer_progress(&p, 0, 0.25), 0.5);
        assert_eq!(layer_progress(&p, 1, 0.25), 0.0);
    }

    #[test]
    fn final_progress_reveals_full_region() {
        let layer = Layer::new_text(1280, 720);
        let regions = reveal_regions(&layer, 1.0);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].u0, 0.0);
        assert_eq!(regions[0].v0, 0.0);
        assert_eq!(regions[0].u1, 1.0);
        assert_eq!(regions[0].v1, 1.0);
    }
}
