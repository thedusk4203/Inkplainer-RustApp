#![cfg(test)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32};

use eframe::egui::{ColorImage, TextureOptions};
use image::{Rgba, RgbaImage};

use crate::assets::{RuntimeAssets, RuntimeImage};
use crate::effects::build_edge_paths;
use crate::model::{AnimationStyle, Background, ExportQuality, HandStyle, Layer, Project};
use crate::{export, export_scale, native_mp4};

fn scanner_fixture(speed: f32) -> (Project, RuntimeAssets) {
    let mut project = Project::default();
    project.canvas.width = 64;
    project.canvas.height = 64;
    project.canvas.background = Background::Solid([255, 255, 255, 255]);

    let mut layer = Layer::new_image(PathBuf::from("fixture.png"), 64, 64, 16, 16);
    layer.transform.x = 16.0;
    layer.transform.y = 16.0;
    layer.transform.w = 32.0;
    layer.transform.h = 32.0;
    layer.animation.style = AnimationStyle::Scanner;
    layer.animation.zigzag = false;
    layer.animation.hand = HandStyle::Ghost;
    layer.animation.speed = speed;
    let layer_id = layer.id.clone();
    project.layers = vec![layer];

    let rgba = RgbaImage::from_pixel(16, 16, Rgba([200, 10, 20, 255]));
    let ctx = eframe::egui::Context::default();
    let color = ColorImage::from_rgba_unmultiplied([16, 16], rgba.as_raw());
    let texture = ctx.load_texture(
        format!("scanner-speed-fixture-{speed}"),
        color,
        TextureOptions::LINEAR,
    );
    let edges = build_edge_paths(&rgba);
    let mut assets = RuntimeAssets::default();
    assets.images.insert(
        layer_id,
        RuntimeImage {
            texture,
            rgba,
            edges,
        },
    );
    (project, assets)
}

fn colored_pixel_count(frame: &RgbaImage) -> usize {
    frame
        .pixels()
        .filter(|pixel| **pixel != Rgba([255, 255, 255, 255]))
        .count()
}

#[test]
fn reveal_speed_changes_actual_rendered_progress() {
    let (slow_project, slow_assets) = scanner_fixture(10.0);
    let (fast_project, fast_assets) = scanner_fixture(80.0);

    let slow = export::render_frame(&slow_project, &slow_assets, 0.5)
        .expect("render slow scanner fixture");
    let fast = export::render_frame(&fast_project, &fast_assets, 0.5)
        .expect("render fast scanner fixture");

    let slow_pixels = colored_pixel_count(&slow);
    let fast_pixels = colored_pixel_count(&fast);
    assert!(slow_pixels > 0, "slow reveal must make measurable progress");
    assert!(fast_pixels > slow_pixels, "higher reveal speed must reveal more content at the same global timestamp");
}

fn avc1_dimensions(bytes: &[u8]) -> Option<(u16, u16)> {
    let type_pos = bytes.windows(4).position(|window| window == b"avc1")?;
    let entry_start = type_pos.checked_sub(4)?;
    let width_start = entry_start.checked_add(32)?;
    let height_start = entry_start.checked_add(34)?;
    let width = u16::from_be_bytes(bytes.get(width_start..width_start + 2)?.try_into().ok()?);
    let height = u16::from_be_bytes(bytes.get(height_start..height_start + 2)?.try_into().ok()?);
    Some((width, height))
}

#[test]
fn mp4_sample_entry_uses_resolved_canvas_aspect_dimensions() {
    let mut project = Project::default();
    project.canvas.width = 320;
    project.canvas.height = 240;
    project.duration_seconds = 0.5;
    project.export.width = 320;
    project.export.height = 240;
    project.export.fps = 12;
    project.export.quality = ExportQuality::Low;
    project.layers.clear();

    let expected = export_scale::resolved_export_dimensions(&project);
    assert_eq!(expected, (426, 320));

    let root = std::env::temp_dir().join(format!(
        "inkplainer-mp4-dimension-test-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root).expect("create MP4 dimension test directory");
    let path = root.join("dimensions.mp4");
    let progress = AtomicU32::new(0);
    let cancel = AtomicBool::new(false);

    native_mp4::export_mp4(
        &project,
        &RuntimeAssets::default(),
        &path,
        &progress,
        &cancel,
    )
    .expect("export MP4 for dimension audit");

    let bytes = fs::read(&path).expect("read MP4 dimension fixture");
    let dimensions = avc1_dimensions(&bytes).expect("find avc1 visual sample entry");
    assert_eq!(dimensions, (expected.0 as u16, expected.1 as u16));

    let _ = fs::remove_dir_all(root);
}
