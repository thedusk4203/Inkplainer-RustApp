use anyhow::{Context, Result};
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use image::RgbaImage;
use std::collections::HashMap;
use std::path::Path;

use crate::effects::{build_edge_paths, EdgePaths};
use crate::model::{LayerContent, Project};

#[derive(Clone)]
pub struct RuntimeImage {
    pub texture: TextureHandle,
    pub rgba: RgbaImage,
    pub edges: EdgePaths,
}

#[derive(Default, Clone)]
pub struct RuntimeAssets {
    pub images: HashMap<String, RuntimeImage>,
}

impl RuntimeAssets {
    pub fn clear(&mut self) {
        self.images.clear();
    }

    pub fn load_project(&mut self, ctx: &egui::Context, project: &Project) -> Vec<String> {
        self.clear();
        let mut errors = Vec::new();
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
