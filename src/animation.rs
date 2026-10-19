use crate::model::{AnimationStyle, Layer, Project};

const DEFAULT_REVEAL_SPEED: f32 = 40.0;

#[derive(Debug, Clone, Copy)]
pub struct RevealRegion {
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

impl RevealRegion {
    pub fn full() -> Self {
        Self {
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        }
    }
}

pub fn layer_progress(project: &Project, layer_index: usize, global_progress: f32) -> f32 {
    if layer_index >= project.layers.len() || project.layers.is_empty() {
        return 0.0;
    }

    let mut group_keys: Vec<GroupKey> = Vec::new();
    for (idx, layer) in project
        .layers
        .iter()
        .enumerate()
        .filter(|(_, layer)| layer.visible)
    {
        let key = layer
            .anim_order
            .map(GroupKey::Ordered)
            .unwrap_or(GroupKey::Unordered(idx));
        if !group_keys.contains(&key) {
            group_keys.push(key);
        }
    }
    group_keys.sort_by(|a, b| {
        a.sort_key()
            .partial_cmp(&b.sort_key())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let layer = &project.layers[layer_index];
    let target = layer
        .anim_order
        .map(GroupKey::Ordered)
        .unwrap_or(GroupKey::Unordered(layer_index));
    let group_index = group_keys.iter().position(|key| *key == target).unwrap_or(0);
    let count = group_keys.len().max(1) as f32;
    let group_progress =
        (global_progress.clamp(0.0, 1.0) * count - group_index as f32).clamp(0.0, 1.0);

    shape_progress_by_speed(group_progress, layer.animation.speed)
}

fn shape_progress_by_speed(progress: f32, speed: f32) -> f32 {
    if !progress.is_finite() {
        return 0.0;
    }

    let progress = progress.clamp(0.0, 1.0);
    if progress <= 0.0 || progress >= 1.0 {
        return progress;
    }

    let speed = if speed.is_finite() {
        speed.clamp(1.0, 100.0)
    } else {
        DEFAULT_REVEAL_SPEED
    };

    // Preserve the existing 40-speed timing exactly while making every value
    // in the supported 1..=100 range distinct. The square-root ratio avoids
    // collapsing low speeds into one bucket and prevents extreme values from
    // becoming effectively frozen until the final frame. Endpoints remain
    // exact so every layer is complete when its timeline slot reaches 100%.
    let exponent = (DEFAULT_REVEAL_SPEED / speed).sqrt();
    progress.powf(exponent).clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GroupKey {
    Ordered(u32),
    Unordered(usize),
}

impl GroupKey {
    fn sort_key(self) -> f64 {
        match self {
            Self::Ordered(value) => value as f64,
            Self::Unordered(index) => 1_000_000.0 + index as f64,
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
        AnimationStyle::Scribble => chunk_regions(layer, p, Priority::Scribble),
        AnimationStyle::Nervous => nervous_regions(layer, p),
        AnimationStyle::TopAnchor => top_anchor_regions(layer, p),
        AnimationStyle::Human => chunk_regions(layer, p, Priority::Human),
        AnimationStyle::Animal | AnimationStyle::Vehicle => {
            chunk_regions(layer, p, Priority::LeftToRight)
        }
        AnimationStyle::Portrait => chunk_regions(layer, p, Priority::CenterOut),
        AnimationStyle::Building => chunk_regions(layer, p, Priority::BottomUp),
        AnimationStyle::Landscape => chunk_regions(layer, p, Priority::TopDown),
        AnimationStyle::Spiral => chunk_regions(layer, p, Priority::Spiral),
        AnimationStyle::Nature => chunk_regions(layer, p, Priority::Nature),
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
        out.push(RevealRegion {
            u0: 0.0,
            v0,
            u1: 1.0,
            v1,
        });
    }
    if full_bands < BANDS && partial > 0.0 {
        let v0 = full_bands as f32 / BANDS as f32;
        let v1 = (full_bands + 1) as f32 / BANDS as f32;
        if zigzag && full_bands % 2 == 1 {
            out.push(RevealRegion {
                u0: 1.0 - partial,
                v0,
                u1: 1.0,
                v1,
            });
        } else {
            out.push(RevealRegion {
                u0: 0.0,
                v0,
                u1: partial,
                v1,
            });
        }
    }
    out
}

fn nervous_regions(layer: &Layer, progress: f32) -> Vec<RevealRegion> {
    let rows = 64usize;
    let seed = stable_hash(&layer.id);
    let mut order: Vec<usize> = (0..rows).collect();
    order.sort_by_key(|row| mix64(seed ^ (*row as u64).wrapping_mul(0x9e3779b97f4a7c15)));
    // Preserve a generally top-to-bottom build but create small hesitant excursions.
    order.sort_by(|a, b| {
        let ja = (*a as f32 + (pseudo(seed, *a as u64) - 0.5) * 7.0).max(0.0);
        let jb = (*b as f32 + (pseudo(seed, *b as u64) - 0.5) * 7.0).max(0.0);
        ja.partial_cmp(&jb).unwrap_or(std::cmp::Ordering::Equal)
    });
    let count = ((rows as f32 * progress).ceil() as usize).min(rows);
    order
        .into_iter()
        .take(count)
        .map(|row| RevealRegion {
            u0: 0.0,
            v0: row as f32 / rows as f32,
            u1: 1.0,
            v1: (row + 1) as f32 / rows as f32,
        })
        .collect()
}

fn top_anchor_regions(layer: &Layer, progress: f32) -> Vec<RevealRegion> {
    let cols = layer.animation.chunks.clamp(12, 80) as usize;
    let full = progress * cols as f32;
    let full_cols = full.floor() as usize;
    let partial = full.fract();
    let seed = stable_hash(&layer.id);
    let mut order: Vec<usize> = (0..cols).collect();
    order.sort_by_key(|col| mix64(seed ^ *col as u64));
    let mut out = Vec::new();
    for col in order.iter().take(full_cols.min(cols)) {
        let u0 = *col as f32 / cols as f32;
        out.push(RevealRegion {
            u0,
            v0: 0.0,
            u1: (*col + 1) as f32 / cols as f32,
            v1: 1.0,
        });
    }
    if full_cols < cols && partial > 0.0 {
        let col = order[full_cols];
        let u0 = col as f32 / cols as f32;
        out.push(RevealRegion {
            u0,
            v0: 0.0,
            u1: (col + 1) as f32 / cols as f32,
            v1: partial,
        });
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
    Scribble,
    Nature,
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
            let noise = pseudo(seed, row as u64 * 131 + col as u64);
            let score = match priority {
                Priority::Random => noise,
                Priority::Human => cy + (cx - 0.5).abs() * 0.18,
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
                Priority::Scribble => {
                    let dx = cx - 0.5;
                    let dy = cy - 0.5;
                    (dx * dx + dy * dy).sqrt() + noise * 0.45
                }
                Priority::Nature => cy * 0.50 + (cx - 0.5).abs() * 0.15 + noise * 0.35,
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
    tiles
        .into_iter()
        .take(visible)
        .map(|tile| tile.region)
        .collect()
}

fn stable_hash(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58476d1ce4e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

fn pseudo(seed: u64, index: u64) -> f32 {
    let value = mix64(seed ^ index.wrapping_mul(0x9e3779b97f4a7c15));
    (value as f64 / u64::MAX as f64) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Layer, Project};

    #[test]
    fn equal_orders_share_timeline_group() {
        let mut project = Project::default();
        let mut a = Layer::new_text(project.canvas.width, project.canvas.height);
        let mut b = Layer::new_text(project.canvas.width, project.canvas.height);
        a.anim_order = Some(1);
        b.anim_order = Some(1);
        project.layers = vec![a, b];
        assert!(
            (layer_progress(&project, 0, 0.5) - layer_progress(&project, 1, 0.5)).abs()
                < f32::EPSILON
        );
    }

    #[test]
    fn ordered_layer_runs_before_unordered_layer() {
        let mut project = Project::default();
        let mut a = Layer::new_text(project.canvas.width, project.canvas.height);
        let b = Layer::new_text(project.canvas.width, project.canvas.height);
        a.anim_order = Some(1);
        project.layers = vec![a, b];
        assert_eq!(layer_progress(&project, 0, 0.25), 0.5);
        assert_eq!(layer_progress(&project, 1, 0.25), 0.0);
    }

    #[test]
    fn reveal_speed_uses_the_full_supported_range_monotonically() {
        let mut project = Project::default();
        let speeds = [1.0_f32, 10.0, 20.0, 40.0, 60.0, 80.0, 100.0];
        project.layers = speeds
            .iter()
            .map(|speed| {
                let mut layer = Layer::new_text(project.canvas.width, project.canvas.height);
                layer.anim_order = Some(1);
                layer.animation.speed = *speed;
                layer
            })
            .collect();

        let progresses: Vec<f32> = (0..project.layers.len())
            .map(|index| layer_progress(&project, index, 0.5))
            .collect();
        for pair in progresses.windows(2) {
            assert!(pair[0] < pair[1]);
        }
        assert!((progresses[3] - 0.5).abs() < f32::EPSILON);

        for index in 0..project.layers.len() {
            assert_eq!(layer_progress(&project, index, 0.0), 0.0);
            assert_eq!(layer_progress(&project, index, 1.0), 1.0);
        }
    }

    #[test]
    fn invalid_reveal_speed_falls_back_to_default() {
        let mut project = Project::default();
        let mut layer = Layer::new_text(project.canvas.width, project.canvas.height);
        layer.animation.speed = f32::NAN;
        project.layers = vec![layer];
        assert_eq!(layer_progress(&project, 0, 0.5), 0.5);
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
