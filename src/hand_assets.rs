use image::RgbaImage;
use std::sync::OnceLock;

use crate::model::HandStyle;

// Inkplainer-OS Apache-2.0 hand sprites. The 720p variants are embedded because they scale
// cleanly for preview/export while keeping the executable substantially smaller than embedding
// all 12 resolution/orientation variants. The original higher-resolution files remain in
// assets/hands for provenance and future packaging.
const HAND1_720: &[u8] = include_bytes!("../assets/hands/hand1-720p.png");
const HAND2_720: &[u8] = include_bytes!("../assets/hands/hand2-720p.png");

static HAND1_IMAGE: OnceLock<RgbaImage> = OnceLock::new();
static HAND2_IMAGE: OnceLock<RgbaImage> = OnceLock::new();

#[derive(Clone, Copy)]
pub struct HandSprite {
    pub image: &'static RgbaImage,
    /// Pen-tip coordinate in source sprite pixels, taken from Inkplainer-OS CUSTOM_HANDS.
    pub tip_x: f32,
    pub tip_y: f32,
    /// Upstream sprite was authored for this canvas-height class.
    pub reference_height: f32,
}

pub fn sprite(style: HandStyle) -> Option<HandSprite> {
    match style {
        HandStyle::Hand1 => Some(HandSprite {
            image: HAND1_IMAGE.get_or_init(|| decode(HAND1_720)),
            tip_x: 229.0,
            tip_y: 0.0,
            reference_height: 720.0,
        }),
        HandStyle::Hand2 => Some(HandSprite {
            image: HAND2_IMAGE.get_or_init(|| decode(HAND2_720)),
            tip_x: 166.0,
            tip_y: 0.0,
            reference_height: 720.0,
        }),
        _ => None,
    }
}

fn decode(bytes: &'static [u8]) -> RgbaImage {
    image::load_from_memory(bytes)
        .expect("embedded upstream hand PNG must decode")
        .to_rgba8()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_upstream_hands_decode() {
        let h1 = sprite(HandStyle::Hand1).unwrap();
        let h2 = sprite(HandStyle::Hand2).unwrap();
        assert!(h1.image.width() > 100 && h1.image.height() > 100);
        assert!(h2.image.width() > 100 && h2.image.height() > 100);
        assert!(h1.tip_x < h1.image.width() as f32);
        assert!(h2.tip_x < h2.image.width() as f32);
    }
}
