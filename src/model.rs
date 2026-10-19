use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    pub name: String,
    pub canvas: CanvasSettings,
    pub layers: Vec<Layer>,
    pub duration_seconds: f32,
    pub hand_glide_enabled: bool,
    pub autosave_enabled: bool,
    pub export: ExportSettings,
    pub custom_presets: Vec<Preset>,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            name: "Untitled Inkplainer Project".to_owned(),
            canvas: CanvasSettings::default(),
            layers: Vec::new(),
            duration_seconds: 8.0,
            hand_glide_enabled: true,
            autosave_enabled: true,
            export: ExportSettings::default(),
            custom_presets: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportSettings {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub quality: ExportQuality,
    pub format: ExportFormat,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 30,
            quality: ExportQuality::High,
            format: ExportFormat::Mp4,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ExportQuality {
    #[default]
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ExportFormat {
    #[default]
    Mp4,
    WebM,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Background {
    Solid([u8; 4]),
    Transparent,
    Notebook,
    Graph,
    Cream,
    Chalkboard,
    SoftGradient,
    WarmWhite,
    Blueprint,
    Kraft,
    Dark,
    Linen,
}

impl Default for Background {
    fn default() -> Self {
        Self::Solid([255, 255, 255, 255])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Layer {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub transform: Transform,
    pub anim_order: Option<u32>,
    pub group: Option<String>,
    pub animation: AnimationSettings,
    pub content: LayerContent,
}

impl Default for Layer {
    fn default() -> Self {
        Self::new_text(1280, 720)
    }
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
            group: None,
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
            group: None,
            animation: AnimationSettings::default(),
            content: LayerContent::Text(TextLayer::default()),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0, w: 100.0, h: 100.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LayerContent {
    Image { path: PathBuf },
    Text(TextLayer),
}

impl Default for LayerContent {
    fn default() -> Self {
        Self::Text(TextLayer::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TextLayer {
    pub text: String,
    pub font_family: String,
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
            font_family: "Segoe UI".to_owned(),
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnimationSettings {
    pub style: AnimationStyle,
    pub speed: f32,
    pub hand_speed: f32,
    pub chunks: u32,
    pub zigzag: bool,
    pub hand: HandStyle,
    pub reveal_original: bool,
    pub reveal_style: RevealStyle,
    pub outline_thickness: f32,
    pub outline_color: [u8; 4],
    pub outline_opacity: f32,
    pub outline_visible: bool,
    pub outline_detection: u8,
    pub detection_algorithm: DetectionAlgorithm,
    pub stroke_style: StrokeStyle,
    pub coloring_style: ColoringStyle,
    pub color_region_outlines: bool,
    pub real_image_edges: bool,
    pub text_direction: TextDirection,
    pub text_draw_style: TextDrawStyle,
}

impl Default for AnimationSettings {
    fn default() -> Self {
        Self {
            style: AnimationStyle::ChunkJump,
            speed: 40.0,
            hand_speed: 6.0,
            chunks: 30,
            zigzag: true,
            hand: HandStyle::Hand1,
            reveal_original: true,
            reveal_style: RevealStyle::Instant,
            outline_thickness: 2.0,
            outline_color: [36, 36, 36, 255],
            outline_opacity: 1.0,
            outline_visible: true,
            outline_detection: 50,
            detection_algorithm: DetectionAlgorithm::Classic,
            stroke_style: StrokeStyle::Default,
            coloring_style: ColoringStyle::Filled,
            color_region_outlines: false,
            real_image_edges: false,
            text_direction: TextDirection::LeftToRight,
            text_draw_style: TextDrawStyle::Reveal,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum HandStyle {
    Ghost,
    #[default]
    Hand1,
    Hand2,
    Hand3,
    Pen,
    Cartoon,
    PixelArt,
}

impl HandStyle {
    pub const ALL: [Self; 7] = [Self::Ghost, Self::Hand1, Self::Hand2, Self::Hand3, Self::Pen, Self::Cartoon, Self::PixelArt];
    pub fn label(self) -> &'static str {
        match self {
            Self::Ghost => "Ghost",
            Self::Hand1 => "Hand 1",
            Self::Hand2 => "Hand 2",
            Self::Hand3 => "Hand 3",
            Self::Pen => "Pen",
            Self::Cartoon => "Cartoon",
            Self::PixelArt => "Pixel Art",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum StrokeStyle {
    #[default]
    Default,
    Charcoal,
    Sketch,
    Fountain,
    Blueprint,
}

impl StrokeStyle {
    pub const ALL: [Self; 5] = [Self::Default, Self::Charcoal, Self::Sketch, Self::Fountain, Self::Blueprint];
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Charcoal => "Charcoal",
            Self::Sketch => "Sketch",
            Self::Fountain => "Fountain",
            Self::Blueprint => "Blueprint",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum DetectionAlgorithm {
    #[default]
    Classic,
    Adaptive,
    MorphShell,
    CannyPlus,
}

impl DetectionAlgorithm {
    pub const ALL: [Self; 4] = [Self::Classic, Self::Adaptive, Self::MorphShell, Self::CannyPlus];
    pub fn label(self) -> &'static str {
        match self {
            Self::Classic => "Classic",
            Self::Adaptive => "Adaptive",
            Self::MorphShell => "Morph Shell",
            Self::CannyPlus => "Canny+",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ColoringStyle {
    Sparse,
    #[default]
    Filled,
    Watercolor,
}

impl ColoringStyle {
    pub const ALL: [Self; 3] = [Self::Sparse, Self::Filled, Self::Watercolor];
    pub fn label(self) -> &'static str {
        match self {
            Self::Sparse => "Sparse",
            Self::Filled => "Filled",
            Self::Watercolor => "Watercolor",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum RevealStyle {
    #[default]
    Instant,
    Fade,
    Dissolve,
    WipeRight,
    Iris,
    ScanLines,
}

impl RevealStyle {
    pub const ALL: [Self; 6] = [Self::Instant, Self::Fade, Self::Dissolve, Self::WipeRight, Self::Iris, Self::ScanLines];
    pub fn label(self) -> &'static str {
        match self {
            Self::Instant => "Instant",
            Self::Fade => "Fade",
            Self::Dissolve => "Dissolve",
            Self::WipeRight => "Wipe",
            Self::Iris => "Iris",
            Self::ScanLines => "Scanlines",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TextDirection {
    #[default]
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}

impl TextDirection {
    pub const ALL: [Self; 4] = [Self::LeftToRight, Self::RightToLeft, Self::TopToBottom, Self::BottomToTop];
    pub fn label(self) -> &'static str {
        match self {
            Self::LeftToRight => "Left → Right",
            Self::RightToLeft => "Right → Left",
            Self::TopToBottom => "Top → Bottom",
            Self::BottomToTop => "Bottom → Top",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum TextDrawStyle {
    #[default]
    Reveal,
    Outline,
    OutlineFill,
}

impl TextDrawStyle {
    pub const ALL: [Self; 3] = [Self::Reveal, Self::Outline, Self::OutlineFill];
    pub fn label(self) -> &'static str {
        match self {
            Self::Reveal => "Reveal",
            Self::Outline => "Outline",
            Self::OutlineFill => "Outline + Fill",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AnimationStyle {
    #[default]
    ChunkJump,
    Scanner,
    Contour,
    OutlineChunks,
    OutlineFill,
    IllustFill,
    OutlineOnly,
    TextDraw,
    Scribble,
    Nervous,
    TopAnchor,
    Human,
    Animal,
    Portrait,
    Vehicle,
    Building,
    Landscape,
    Spiral,
    Nature,
}

impl AnimationStyle {
    pub const ALL: [Self; 19] = [
        Self::ChunkJump,
        Self::Scanner,
        Self::Contour,
        Self::OutlineChunks,
        Self::OutlineFill,
        Self::IllustFill,
        Self::OutlineOnly,
        Self::TextDraw,
        Self::Scribble,
        Self::Nervous,
        Self::TopAnchor,
        Self::Human,
        Self::Animal,
        Self::Portrait,
        Self::Vehicle,
        Self::Building,
        Self::Landscape,
        Self::Spiral,
        Self::Nature,
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
            Self::Scribble => "Scribble",
            Self::Nervous => "Nervous",
            Self::TopAnchor => "Top Anchor",
            Self::Human => "Human",
            Self::Animal => "Animal",
            Self::Portrait => "Portrait",
            Self::Vehicle => "Vehicle",
            Self::Building => "Building",
            Self::Landscape => "Landscape",
            Self::Spiral => "Spiral",
            Self::Nature => "Nature",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preset {
    pub name: String,
    pub animation: AnimationSettings,
}

impl Default for Preset {
    fn default() -> Self {
        Self { name: "Custom preset".to_owned(), animation: AnimationSettings::default() }
    }
}
