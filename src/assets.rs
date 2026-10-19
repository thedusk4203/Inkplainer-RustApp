use anyhow::{Context, Result};
use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use image::RgbaImage;
use std::collections::HashMap;
use std::path::Path;

use crate::model::{LayerContent, Project};

pub struct RuntimeImage {
    pub texture: TextureHandle,
    pub rgba: RgbaImage,
    pub edge_path: Vec<[f32; 2]>,
}

#[derive(Default)]
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
        let edge_path = extract_edge_path(&decoded);
        self.images.insert(
            layer_id.to_owned(),
            RuntimeImage {
                texture,
                rgba: decoded,
                edge_path,
            },
        );
        Ok((w, h))
    }
}

fn extract_edge_path(img: &RgbaImage) -> Vec<[f32; 2]> {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return Vec::new();
    }
    let present = |x: u32, y: u32| {
        let p = img.get_pixel(x, y).0;
        p[3] > 20 && (p[3] < 250 || p[0] < 240 || p[1] < 240 || p[2] < 240)
    };

    let mut pts = Vec::new();
    for x in 0..w {
        for y in 0..h {
            if present(x, y) {
                pts.push([x as f32 / w as f32, y as f32 / h as f32]);
                break;
            }
        }
    }
    for x in (0..w).rev() {
        for y in (0..h).rev() {
            if present(x, y) {
                pts.push([x as f32 / w as f32, y as f32 / h as f32]);
                break;
            }
        }
    }
    for y in (0..h).rev() {
        for x in 0..w {
            if present(x, y) {
                pts.push([x as f32 / w as f32, y as f32 / h as f32]);
                break;
            }
        }
    }

    if pts.len() > 2500 {
        let stride = (pts.len() as f32 / 2500.0).ceil() as usize;
        pts.into_iter().step_by(stride).collect()
    } else {
        pts
    }
}
