use anyhow::{bail, Context, Result};
use image::{GenericImageView, Rgba, RgbaImage};
use std::path::PathBuf;

use crate::assets::RuntimeImage;
use crate::model::{Layer, LayerContent};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalizedRect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl NormalizedRect {
    pub fn clamped(self) -> Self {
        let x = self.x.clamp(0.0, 1.0);
        let y = self.y.clamp(0.0, 1.0);
        let w = self.w.clamp(0.0, 1.0 - x);
        let h = self.h.clamp(0.0, 1.0 - y);
        Self { x, y, w, h }
    }
}

pub fn grid_slice(layer: &Layer, runtime: &RuntimeImage, cols: u32, rows: u32) -> Result<Vec<Layer>> {
    let cols = cols.clamp(1, 8);
    let rows = rows.clamp(1, 8);
    let mut rects = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            rects.push(NormalizedRect {
                x: col as f32 / cols as f32,
                y: row as f32 / rows as f32,
                w: 1.0 / cols as f32,
                h: 1.0 / rows as f32,
            });
        }
    }
    rectangle_slice(layer, runtime, &rects)
}

pub fn rectangle_slice(layer: &Layer, runtime: &RuntimeImage, rects: &[NormalizedRect]) -> Result<Vec<Layer>> {
    if rects.is_empty() { bail!("No slice regions supplied"); }
    let root = slice_root(layer)?;
    let mut out = Vec::new();
    for (idx, rect) in rects.iter().copied().map(NormalizedRect::clamped).enumerate() {
        if rect.w <= 0.001 || rect.h <= 0.001 { continue; }
        let sw = runtime.rgba.width().max(1);
        let sh = runtime.rgba.height().max(1);
        let x = (rect.x * sw as f32).floor().clamp(0.0, sw.saturating_sub(1) as f32) as u32;
        let y = (rect.y * sh as f32).floor().clamp(0.0, sh.saturating_sub(1) as f32) as u32;
        let w = (rect.w * sw as f32).ceil().max(1.0) as u32;
        let h = (rect.h * sh as f32).ceil().max(1.0) as u32;
        let w = w.min(sw - x);
        let h = h.min(sh - y);
        let cropped = runtime.rgba.view(x, y, w, h).to_image();
        let path = root.join(format!("slice-{idx:03}.png"));
        cropped.save(&path).with_context(|| format!("Save slice {}", path.display()))?;
        out.push(make_slice_layer(layer, &path, rect, idx));
    }
    if out.is_empty() { bail!("All slice regions were empty"); }
    Ok(out)
}

pub fn freehand_slice(layer: &Layer, runtime: &RuntimeImage, polygon: &[[f32; 2]]) -> Result<Vec<Layer>> {
    if polygon.len() < 3 { bail!("Freehand slice needs at least three points"); }
    let min_x = polygon.iter().map(|p| p[0]).fold(1.0f32, f32::min).clamp(0.0, 1.0);
    let min_y = polygon.iter().map(|p| p[1]).fold(1.0f32, f32::min).clamp(0.0, 1.0);
    let max_x = polygon.iter().map(|p| p[0]).fold(0.0f32, f32::max).clamp(0.0, 1.0);
    let max_y = polygon.iter().map(|p| p[1]).fold(0.0f32, f32::max).clamp(0.0, 1.0);
    if max_x <= min_x || max_y <= min_y { bail!("Freehand polygon has no area"); }

    let sw = runtime.rgba.width().max(1);
    let sh = runtime.rgba.height().max(1);
    let x0 = (min_x * sw as f32).floor() as u32;
    let y0 = (min_y * sh as f32).floor() as u32;
    let x1 = (max_x * sw as f32).ceil().min(sw as f32) as u32;
    let y1 = (max_y * sh as f32).ceil().min(sh as f32) as u32;
    let mut cropped = RgbaImage::from_pixel((x1 - x0).max(1), (y1 - y0).max(1), Rgba([0, 0, 0, 0]));
    for y in y0..y1 {
        for x in x0..x1 {
            let nx = (x as f32 + 0.5) / sw as f32;
            let ny = (y as f32 + 0.5) / sh as f32;
            if point_in_polygon([nx, ny], polygon) {
                cropped.put_pixel(x - x0, y - y0, *runtime.rgba.get_pixel(x, y));
            }
        }
    }
    let root = slice_root(layer)?;
    let path = root.join("freehand-000.png");
    cropped.save(&path).with_context(|| format!("Save freehand slice {}", path.display()))?;
    let rect = NormalizedRect { x: min_x, y: min_y, w: max_x - min_x, h: max_y - min_y };
    Ok(vec![make_slice_layer(layer, &path, rect, 0)])
}

fn slice_root(layer: &Layer) -> Result<PathBuf> {
    let root = std::env::temp_dir().join("inkplainer-rustapp-slices").join(&layer.id);
    std::fs::create_dir_all(&root).with_context(|| format!("Create slicer directory {}", root.display()))?;
    Ok(root)
}

fn make_slice_layer(source: &Layer, path: &std::path::Path, rect: NormalizedRect, index: usize) -> Layer {
    let mut layer = source.clone();
    layer.id = uuid::Uuid::new_v4().to_string();
    layer.name = format!("{} slice {}", source.name, index + 1);
    layer.transform.x = source.transform.x + source.transform.w * rect.x;
    layer.transform.y = source.transform.y + source.transform.h * rect.y;
    layer.transform.w = source.transform.w * rect.w;
    layer.transform.h = source.transform.h * rect.h;
    layer.content = LayerContent::Image { path: path.to_path_buf() };
    layer
}

fn point_in_polygon(p: [f32; 2], polygon: &[[f32; 2]]) -> bool {
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let a = polygon[i];
        let b = polygon[j];
        if (a[1] > p[1]) != (b[1] > p[1]) {
            let denom = b[1] - a[1];
            if denom.abs() > f32::EPSILON {
                let crossing_x = (b[0] - a[0]) * (p[1] - a[1]) / denom + a[0];
                if p[0] < crossing_x { inside = !inside; }
            }
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polygon_hit_test() {
        let clockwise = [[0.0,0.0],[1.0,0.0],[1.0,1.0],[0.0,1.0]];
        let counter_clockwise = [[0.0,0.0],[0.0,1.0],[1.0,1.0],[1.0,0.0]];
        assert!(point_in_polygon([0.5,0.5], &clockwise));
        assert!(point_in_polygon([0.5,0.5], &counter_clockwise));
        assert!(!point_in_polygon([1.5,0.5], &clockwise));
        assert!(!point_in_polygon([-0.1,0.5], &counter_clockwise));
    }
}
