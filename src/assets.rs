use anyhow::{Context, Result};
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use image::{imageops::FilterType, RgbaImage};
use std::collections::HashMap;
use std::path::Path;

use crate::effects::{build_edge_paths, EdgePaths};
use crate::hand_assets::{self, HandResolution};
use crate::model::{HandStyle, LayerContent, Project};

#[derive(Clone)]
pub struct RuntimeImage {
    pub texture: TextureHandle,
    pub rgba: RgbaImage,
    pub edges: EdgePaths,
}

#[derive(Clone)]
pub struct RuntimeHand {
    pub style: HandStyle,
    pub resolution: HandResolution,
    pub texture: TextureHandle,
    pub rgba: RgbaImage,
    pub width: u32,
    pub height: u32,
    pub tip_x: u32,
    pub tip_y: u32,
}

#[derive(Default, Clone)]
pub struct RuntimeAssets {
    pub images: HashMap<String, RuntimeImage>,
    pub hands: Vec<RuntimeHand>,
}

impl RuntimeAssets {
    pub fn clear(&mut self) {
        self.images.clear();
    }

    pub fn load_builtin_hands(&mut self, ctx: &egui::Context) -> Vec<String> {
        self.hands.clear();
        let mut errors = Vec::new();
        for def in hand_assets::definitions() {
            match image::load_from_memory(def.bytes) {
                Ok(image) => {
                    let decoded = image.to_rgba8();
                    let rgba = if decoded.width() == def.width && decoded.height() == def.height {
                        decoded
                    } else {
                        image::imageops::resize(&decoded, def.width, def.height, FilterType::Lanczos3)
                    };
                    let size = [rgba.width() as usize, rgba.height() as usize];
                    let color = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                    let texture = ctx.load_texture(
                        format!("builtin-hand-{:?}-{:?}", def.style, def.resolution),
                        color,
                        TextureOptions::LINEAR,
                    );
                    self.hands.push(RuntimeHand {
                        style: def.style,
                        resolution: def.resolution,
                        texture,
                        rgba,
                        width: def.width,
                        height: def.height,
                        tip_x: def.tip_x,
                        tip_y: def.tip_y,
                    });
                }
                Err(err) => errors.push(format!(
                    "Failed to decode {:?}/{:?} hand sprite: {err}",
                    def.style, def.resolution
                )),
            }
        }
        errors
    }

    pub fn hand(&self, style: HandStyle, canvas_w: u32, canvas_h: u32) -> Option<&RuntimeHand> {
        let resolution = hand_assets::select_resolution(canvas_w, canvas_h);
        self.hands
            .iter()
            .find(|h| h.style == style && h.resolution == resolution)
    }

    pub fn load_project(&mut self, ctx: &egui::Context, project: &Project) -> Vec<String> {
        self.clear();
        let mut errors = Vec::new();
        if self.hands.is_empty() {
            errors.extend(self.load_builtin_hands(ctx));
        }
        for layer in &project.layers {
            if let LayerContent::Image { path } = &layer.content {
                if let Err(err) = self.load_image(ctx, &layer.id, path) {
                    errors.push(format!("{}: {err:#}", layer.name));
                }
            }
        }
        errors
    }

    pub fn load_image(&mut self, ctx: &egui::Context, layer_id: &str, path: &Path) -> Result<(u32, u32)> {
        if self.hands.is_empty() {
            let _ = self.load_builtin_hands(ctx);
        }
        let decoded = image::ImageReader::open(path)
            .with_context(|| format!("Cannot open {}", path.display()))?
            .decode()
            .with_context(|| format!("Cannot decode {}", path.display()))?
            .to_rgba8();
        let (w, h) = decoded.dimensions();
        let size = [w as usize, h as usize];
        let color = ColorImage::from_rgba_unmultiplied(size, decoded.as_raw());
        let texture = ctx.load_texture(
            format!("layer-{layer_id}"),
            color,
            TextureOptions::LINEAR,
        );
        let edges = build_edge_paths(&decoded);
        self.images.insert(
            layer_id.to_owned(),
            RuntimeImage {
                texture,
                rgba: decoded,
                edges,
            },
        );
        Ok((w, h))
    }
}
