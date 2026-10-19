use crate::model::HandStyle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HandResolution {
    P720,
    P1080,
    P1440,
    V720,
    V1080,
    V1440,
}

#[derive(Clone, Copy)]
pub struct HandDefinition {
    pub style: HandStyle,
    pub resolution: HandResolution,
    pub bytes: &'static [u8],
    pub width: u32,
    pub height: u32,
    pub tip_x: u32,
    pub tip_y: u32,
}

pub fn select_resolution(canvas_w: u32, canvas_h: u32) -> HandResolution {
    let max_dim = canvas_w.max(canvas_h);
    let portrait = canvas_h > canvas_w;
    match (portrait, max_dim) {
        (false, 0..=1280) => HandResolution::P720,
        (false, 1281..=1920) => HandResolution::P1080,
        (false, _) => HandResolution::P1440,
        (true, 0..=1280) => HandResolution::V720,
        (true, 1281..=1920) => HandResolution::V1080,
        (true, _) => HandResolution::V1440,
    }
}

macro_rules! hand {
    ($style:ident, $res:ident, $file:literal, $w:expr, $h:expr, $tx:expr, $ty:expr) => {
        HandDefinition {
            style: HandStyle::$style,
            resolution: HandResolution::$res,
            bytes: include_bytes!(concat!(env!("OUT_DIR"), "/inkplainer-hands/", $file)),
            width: $w,
            height: $h,
            tip_x: $tx,
            tip_y: $ty,
        }
    };
}

pub fn definitions() -> &'static [HandDefinition] {
    static DEFINITIONS: &[HandDefinition] = &[
        hand!(Hand1, P720,  "hand1-720p.png", 432, 768, 229, 0),
        hand!(Hand1, P1080, "hand1-1080p.png", 648, 1152, 325, 1),
        hand!(Hand1, P1440, "hand1-1440p.png", 864, 1536, 448, 1),
        hand!(Hand1, V720,  "hand1vertical-720p.png", 720, 1280, 0, 0),
        hand!(Hand1, V1080, "hand1vertical-1080p.png", 1080, 1920, 0, 0),
        hand!(Hand1, V1440, "hand1vertical-1440p.png", 1440, 2560, 0, 0),

        hand!(Hand2, P720,  "hand2-720p.png", 432, 768, 166, 0),
        hand!(Hand2, P1080, "hand2-1080p.png", 648, 1152, 249, 0),
        hand!(Hand2, P1440, "hand2-1440p.png", 864, 1536, 345, 0),
        hand!(Hand2, V720,  "hand2vertical-720p.png", 720, 1280, 0, 0),
        hand!(Hand2, V1080, "hand2vertical-1080p.png", 1080, 1920, 0, 0),
        hand!(Hand2, V1440, "hand2vertical-1440p.png", 1440, 2560, 0, 0),

        hand!(Hand3, P720,  "hand3-720p.png", 432, 768, 117, 0),
        hand!(Hand3, P1080, "hand3-1080p.png", 648, 1152, 196, 0),
        hand!(Hand3, P1440, "hand3-1440p.png", 864, 1536, 266, 1),
        hand!(Hand3, V720,  "hand3vertical-720p.png", 720, 1280, 1, 0),
        hand!(Hand3, V1080, "hand3vertical-1080p.png", 1080, 1920, 0, 0),
        hand!(Hand3, V1440, "hand3vertical-1440p.png", 1440, 2560, 0, 0),

        hand!(Pen, P720,  "hand4-720p.png", 432, 768, 81, 628),
        hand!(Pen, P1080, "hand4-1080p.png", 648, 1152, 140, 904),
        hand!(Pen, P1440, "hand4-1440p.png", 864, 1536, 175, 1214),
        hand!(Pen, V720,  "hand4vertical-720p.png", 432, 768, 81, 628),
        hand!(Pen, V1080, "hand4vertical-1080p.png", 450, 750, 225, 150),
        hand!(Pen, V1440, "hand4vertical-1440p.png", 864, 1536, 175, 1213),

        hand!(Cartoon, P720,  "hand5-720p.png", 180, 320, 0, 49),
        hand!(Cartoon, P1080, "hand5-1080p.png", 270, 480, 1, 75),
        hand!(Cartoon, P1440, "hand5-1440p.png", 360, 640, 1, 101),
        hand!(Cartoon, V720,  "hand5vertical-720p.png", 180, 320, 0, 49),
        hand!(Cartoon, V1080, "hand5vertical-1080p.png", 270, 480, 1, 75),
        hand!(Cartoon, V1440, "hand5vertical-1440p.png", 360, 640, 1, 101),

        hand!(PixelArt, P720,  "hand6-720p.png", 180, 320, 19, 94),
        hand!(PixelArt, P1080, "hand6-1080p.png", 270, 480, 29, 141),
        hand!(PixelArt, P1440, "hand6-1440p.png", 360, 640, 34, 176),
        hand!(PixelArt, V720,  "hand6vertical-720p.png", 180, 320, 19, 94),
        hand!(PixelArt, V1080, "hand6vertical-1080p.png", 270, 480, 29, 141),
        hand!(PixelArt, V1440, "hand6vertical-1440p.png", 360, 640, 34, 176),
    ];
    DEFINITIONS
}
