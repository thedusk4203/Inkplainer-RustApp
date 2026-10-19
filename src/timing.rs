const DEFAULT_HAND_SPEED: f32 = 6.0;

/// Maps the layer's reveal progress to an independent hand-path progress.
///
/// The default hand speed (6) preserves the previous 1:1 mapping. Lower
/// values lag behind the reveal path and higher values move ahead. The curve
/// remains monotonic and preserves exact 0/1 endpoints so preview and offline
/// rendering remain deterministic and the hand never indexes outside a path.
pub fn hand_motion_progress(progress: f32, hand_speed: f32) -> f32 {
    if !progress.is_finite() {
        return 0.0;
    }

    let progress = progress.clamp(0.0, 1.0);
    if progress <= 0.0 || progress >= 1.0 {
        return progress;
    }

    let hand_speed = if hand_speed.is_finite() {
        hand_speed.clamp(1.0, 30.0)
    } else {
        DEFAULT_HAND_SPEED
    };
    let exponent = (DEFAULT_HAND_SPEED / hand_speed).sqrt();
    progress.powf(exponent).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hand_speed_preserves_progress() {
        for progress in [0.0_f32, 0.1, 0.25, 0.5, 0.9, 1.0] {
            assert!((hand_motion_progress(progress, DEFAULT_HAND_SPEED) - progress).abs() < 1e-6);
        }
    }

    #[test]
    fn hand_speed_is_monotonic_across_supported_range() {
        let speeds = [1.0_f32, 3.0, 6.0, 10.0, 20.0, 30.0];
        let values: Vec<f32> = speeds
            .into_iter()
            .map(|speed| hand_motion_progress(0.5, speed))
            .collect();
        for pair in values.windows(2) {
            assert!(pair[0] < pair[1]);
        }
    }

    #[test]
    fn hand_timing_preserves_endpoints_and_sanitizes_invalid_values() {
        assert_eq!(hand_motion_progress(0.0, 30.0), 0.0);
        assert_eq!(hand_motion_progress(1.0, 1.0), 1.0);
        assert_eq!(hand_motion_progress(f32::NAN, 6.0), 0.0);
        assert!((hand_motion_progress(0.5, f32::NAN) - 0.5).abs() < 1e-6);
    }
}
