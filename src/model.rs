use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub canvas: CanvasSettings,
    pub layers: Vec<Layer>,
    pub duration_seconds: f32,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            name: "Untitled Inkplainer Project".to_owned(),
            canvas: CanvasSettings::default(),
            layers: Vec::new(),
            duration_seconds: 8.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasSettings {
    pub width: u32,
    pub height: u32,
    pub background: Background,
}

impl Default for CanvasSettings {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            background: Background::Solid([255, 255, 255, 255]),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Background {
    Solid([u8; 4]),
    Transparent,
    Notebook,
    Graph,
    Blueprint,
    Kraft,
    Dark,
}

impl Default for Background {
    fn default() -> Self {
        Self::Solid([255, 255, 255, 255])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub transform: Transform,
    pub anim_order: Option<u32>,
    pub animation: AnimationSettings,
    pub content: LayerContent,
}

impl Layer {
    pub fn new_image(path: PathBuf, canvas_w: u32, canvas_h: u32, image_w: u32, image_h: u32) -> Self {
        let fit = (canvas_w as f32 / image_w.max(1) as f32)
            .min(canvas_h as f32 / image_h.max(1) as f32)
            .min(1.0);
        let w = image_w as f32 * fit;
        let h = image_h as f32 * fit;
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: path.file_name().and_then(|s| s.to_str()).unwrap_or("Image").to_owned(),
            visible: true,
            opacity: 1.0,
            transform: Transform {
                x: (canvas_w as f32 - w) * 0.5,
                y: (canvas_h as f32 - h) * 0.5,
                w,
                h,
            },
            anim_order: None,
            animation: AnimationSettings::default(),
            content: LayerContent::Image { path },
        }
    }

    pub fn new_text(canvas_w: u32, canvas_h: u32) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Text".to_owned(),
            visible: true,
            opacity: 1.0,
            transform: Transform {
                x: canvas_w as f32 * 0.25,
                y: canvas_h as f32 * 0.42,
                w: canvas_w as f32 * 0.5,
                h: 100.0,
            },
            anim_order: None,
            animation: AnimationSettings::default(),
            content: LayerContent::Text(TextLayer::default()),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LayerContent {
    Image { path: PathBuf },
    Text(TextLayer),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextLayer {
    pub text: String,
    pub size: f32,
    pub color: [u8; 4],
    pub bold: bool,
    pub italic: bool,
    pub align: TextAlign,
    pub line_height: f32,
    pub letter_spacing: f32,
}

impl Default for TextLayer {
    fn default() -> Self {
        Self {
            text: "Your text".to_owned(),
            size: 64.0,
            color: [24, 24, 24, 255],
            bold: false,
            italic: false,
            align: TextAlign::Center,
            line_height: 1.15,
            letter_spacing: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationSettings {
    pub style: AnimationStyle,
    pub speed: f32,
    pub hand_speed: f32,
    pub chunks: u32,
    pub zigzag: bool,
    pub reveal_original: bool,
    pub outline_thickness: f32,
    pub outline_color: [u8; 4],
}

impl Default for AnimationSettings {
    fn default() -> Self {
        Self {
            style: AnimationStyle::ChunkJump,
            speed: 40.0,
            hand_speed: 6.0,
            chunks: 30,
            zigzag: true,
            reveal_original: true,
            outline_thickness: 2.0,
            outline_color: [36, 36, 36, 255],
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnimationStyle {
    ChunkJump,
    Scanner,
    Contour,
    OutlineChunks,
    OutlineFill,
    IllustFill,
    OutlineOnly,
    TextDraw,
    Human,
    Animal,
    Portrait,
    Vehicle,
    Building,
    Landscape,
    Spiral,
}

impl AnimationStyle {
    pub const ALL: [Self; 15] = [
        Self::ChunkJump,
        Self::Scanner,
        Self::Contour,
        Self::OutlineChunks,
        Self::OutlineFill,
        Self::IllustFill,
        Self::OutlineOnly,
        Self::TextDraw,
        Self::Human,
        Self::Animal,
        Self::Portrait,
        Self::Vehicle,
        Self::Building,
        Self::Landscape,
        Self::Spiral,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::ChunkJump => "Chunk Jump",
            Self::Scanner => "Scanner",
            Self::Contour => "Contour",
            Self::OutlineChunks => "Outline Chunks",
            Self::OutlineFill => "Outline Fill",
            Self::IllustFill => "Illust Fill",
            Self::OutlineOnly => "Outline Only",
            Self::TextDraw => "Text Draw",
            Self::Human => "Human",
            Self::Animal => "Animal",
            Self::Portrait => "Portrait",
            Self::Vehicle => "Vehicle",
            Self::Building => "Building",
            Self::Landscape => "Landscape",
            Self::Spiral => "Spiral",
        }
    }
}
