use anyhow::Result;
use image::RgbaImage;

use crate::assets::RuntimeAssets;
use crate::export;
use crate::model::{LayerContent, Project};

pub fn render_frame_at_size(
    project: &Project,
    assets: &RuntimeAssets,
    global_progress: f32,
    out_w: u32,
    out_h: u32,
) -> Result<RgbaImage> {
    if project.canvas.width == out_w && project.canvas.height == out_h {
        return export::render_frame(project, assets, global_progress);
    }
    let scaled = project_for_output(project, out_w, out_h);
    export::render_frame(&scaled, assets, global_progress)
}

pub fn project_for_output(project: &Project, out_w: u32, out_h: u32) -> Project {
    let source_w = project.canvas.width.max(1) as f32;
    let source_h = project.canvas.height.max(1) as f32;
    let sx = out_w.max(1) as f32 / source_w;
    let sy = out_h.max(1) as f32 / source_h;
    let stroke_scale = (sx * sy).sqrt();

    let mut scaled = project.clone();
    scaled.canvas.width = out_w.max(1);
    scaled.canvas.height = out_h.max(1);

    for layer in &mut scaled.layers {
        layer.transform.x *= sx;
        layer.transform.y *= sy;
        layer.transform.w *= sx;
        layer.transform.h *= sy;
        layer.animation.outline_thickness *= stroke_scale;

        if let LayerContent::Text(text) = &mut layer.content {
            // Font size is a vertical pixel measurement. Letter spacing is horizontal.
            text.size *= sy;
            text.letter_spacing *= sx;
        }
    }

    scaled
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Layer, LayerContent};

    #[test]
    fn output_project_scales_geometry_and_pixel_sized_text_settings() {
        let mut project = Project::default();
        project.canvas.width = 1280;
        project.canvas.height = 720;
        let mut layer = Layer::new_text(1280, 720);
        layer.transform.x = 100.0;
        layer.transform.y = 50.0;
        layer.transform.w = 400.0;
        layer.transform.h = 120.0;
        layer.animation.outline_thickness = 2.0;
        if let LayerContent::Text(text) = &mut layer.content {
            text.size = 40.0;
            text.letter_spacing = 3.0;
        }
        project.layers.push(layer);

        let scaled = project_for_output(&project, 1920, 1080);
        assert_eq!(scaled.canvas.width, 1920);
        assert_eq!(scaled.canvas.height, 1080);
        let layer = &scaled.layers[0];
        assert!((layer.transform.x - 150.0).abs() < 0.001);
        assert!((layer.transform.y - 75.0).abs() < 0.001);
        assert!((layer.transform.w - 600.0).abs() < 0.001);
        assert!((layer.transform.h - 180.0).abs() < 0.001);
        assert!((layer.animation.outline_thickness - 3.0).abs() < 0.001);
        if let LayerContent::Text(text) = &layer.content {
            assert!((text.size - 60.0).abs() < 0.001);
            assert!((text.letter_spacing - 4.5).abs() < 0.001);
        } else {
            panic!("expected text layer");
        }
    }
}
