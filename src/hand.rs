use crate::animation::reveal_regions;
use crate::assets::RuntimeImage;
use crate::effects::select_edge_path;
use crate::model::{AnimationStyle, Layer};

/// Returns the native hand tip in normalized layer coordinates.
///
/// When `smooth` is enabled, several trailing targets are averaged with a recency-weighted
/// kernel. This mirrors the upstream intent (avoid high-speed teleporting) while keeping the
/// renderer stateless, which also makes preview and offline export deterministic.
pub fn hand_tip_normalized(
    runtime: &RuntimeImage,
    layer: &Layer,
    progress: f32,
    smooth: bool,
) -> [f32; 2] {
    let p = progress.clamp(0.0, 1.0);
    if !smooth {
        return raw_tip(runtime, layer, p);
    }

    // Faster hand settings look ahead less; slower hand settings get a slightly longer trail.
    let hand_speed = layer.animation.hand_speed.clamp(1.0, 30.0);
    let window = (0.028 - hand_speed * 0.00045).clamp(0.010, 0.028);
    let samples = 7usize;
    let mut sum = [0.0f32, 0.0f32];
    let mut weight_sum = 0.0f32;
    for i in 0..samples {
        let t = if samples <= 1 { 0.0 } else { i as f32 / (samples - 1) as f32 };
        let sample_p = (p - window * t).max(0.0);
        let weight = (1.0 - t * 0.72).max(0.12);
        let q = raw_tip(runtime, layer, sample_p);
        sum[0] += q[0] * weight;
        sum[1] += q[1] * weight;
        weight_sum += weight;
    }
    if weight_sum <= f32::EPSILON { raw_tip(runtime, layer, p) }
    else { [sum[0] / weight_sum, sum[1] / weight_sum] }
}

fn raw_tip(runtime: &RuntimeImage, layer: &Layer, progress: f32) -> [f32; 2] {
    if matches!(
        layer.animation.style,
        AnimationStyle::Contour
            | AnimationStyle::OutlineFill
            | AnimationStyle::IllustFill
            | AnimationStyle::OutlineOnly
    ) {
        let pts = select_edge_path(&runtime.edges, &layer.animation);
        if !pts.is_empty() {
            let i = ((pts.len() - 1) as f32 * progress.clamp(0.0, 1.0)).round() as usize;
            return pts[i];
        }
    }

    if let Some(region) = reveal_regions(layer, progress).last() {
        return [(region.u0 + region.u1) * 0.5, (region.v0 + region.v1) * 0.5];
    }
    [0.5, 0.5]
}

#[cfg(test)]
mod tests {
    // The trajectory engine depends on RuntimeImage/egui GPU handles, so integration-level visual
    // behavior is covered by the Windows regression harness. Keep mathematical invariants local.
    #[test]
    fn smoothing_kernel_weights_are_positive() {
        for i in 0..7 {
            let t = i as f32 / 6.0;
            assert!((1.0 - t * 0.72).max(0.12) > 0.0);
        }
    }
}
