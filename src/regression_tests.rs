#![cfg(test)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use eframe::egui::{ColorImage, TextureOptions};
use image::{Rgba, RgbaImage};

use crate::animation::{layer_progress, reveal_regions};
use crate::assets::{RuntimeAssets, RuntimeImage};
use crate::effects::build_edge_paths;
use crate::model::{AnimationStyle, Background, ExportQuality, HandStyle, Layer, Project};
use crate::{export, export_scale, native_mp4, native_webm, project_io};

#[test]
fn ordered_layers_share_groups_and_unordered_layers_follow() {
    let mut project = Project::default();
    let mut a = Layer::new_text(project.canvas.width, project.canvas.height);
    let mut b = Layer::new_text(project.canvas.width, project.canvas.height);
    let mut c = Layer::new_text(project.canvas.width, project.canvas.height);
    a.anim_order = Some(1);
    b.anim_order = Some(1);
    c.anim_order = None;
    project.layers = vec![a, b, c];

    let pa = layer_progress(&project, 0, 0.25);
    let pb = layer_progress(&project, 1, 0.25);
    let pc = layer_progress(&project, 2, 0.25);
    assert!((pa - pb).abs() < f32::EPSILON);
    assert!(pa > 0.0);
    assert_eq!(pc, 0.0);

    assert_eq!(layer_progress(&project, 0, 1.0), 1.0);
    assert_eq!(layer_progress(&project, 1, 1.0), 1.0);
    assert_eq!(layer_progress(&project, 2, 1.0), 1.0);
}

#[test]
fn every_reveal_style_stays_inside_normalized_canvas() {
    let project = Project::default();
    let mut layer = Layer::new_text(project.canvas.width, project.canvas.height);
    layer.id = "regression-seed".to_owned();

    let styles = [
        AnimationStyle::Scanner,
        AnimationStyle::ChunkJump,
        AnimationStyle::OutlineChunks,
        AnimationStyle::Scribble,
        AnimationStyle::Nervous,
        AnimationStyle::TopAnchor,
        AnimationStyle::Human,
        AnimationStyle::Animal,
        AnimationStyle::Portrait,
        AnimationStyle::Vehicle,
        AnimationStyle::Building,
        AnimationStyle::Landscape,
        AnimationStyle::Spiral,
        AnimationStyle::Nature,
    ];

    for style in styles {
        layer.animation.style = style;
        for progress in [0.01_f32, 0.1, 0.33, 0.5, 0.9, 0.999] {
            for region in reveal_regions(&layer, progress) {
                assert!(region.u0 >= 0.0 && region.u0 <= 1.0, "{style:?}: u0");
                assert!(region.u1 >= 0.0 && region.u1 <= 1.0, "{style:?}: u1");
                assert!(region.v0 >= 0.0 && region.v0 <= 1.0, "{style:?}: v0");
                assert!(region.v1 >= 0.0 && region.v1 <= 1.0, "{style:?}: v1");
                assert!(region.u1 >= region.u0, "{style:?}: inverted u");
                assert!(region.v1 >= region.v0, "{style:?}: inverted v");
            }
        }
    }
}

#[test]
fn deterministic_styles_produce_stable_reveal_geometry() {
    let project = Project::default();
    let mut layer = Layer::new_text(project.canvas.width, project.canvas.height);
    layer.id = "stable-seed".to_owned();
    layer.animation.chunks = 37;

    for style in [
        AnimationStyle::ChunkJump,
        AnimationStyle::Scribble,
        AnimationStyle::Nervous,
        AnimationStyle::TopAnchor,
        AnimationStyle::Human,
        AnimationStyle::Portrait,
        AnimationStyle::Spiral,
        AnimationStyle::Nature,
    ] {
        layer.animation.style = style;
        let a = reveal_regions(&layer, 0.47);
        let b = reveal_regions(&layer, 0.47);
        assert_eq!(a.len(), b.len(), "{style:?}");
        for (ra, rb) in a.iter().zip(b.iter()) {
            assert!((ra.u0 - rb.u0).abs() < f32::EPSILON);
            assert!((ra.v0 - rb.v0).abs() < f32::EPSILON);
            assert!((ra.u1 - rb.u1).abs() < f32::EPSILON);
            assert!((ra.v1 - rb.v1).abs() < f32::EPSILON);
        }
    }
}

#[test]
fn representative_scanner_frame_matches_upstream_pixel_fixture() {
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
    let layer_id = layer.id.clone();
    project.layers = vec![layer];

    let rgba = RgbaImage::from_pixel(16, 16, Rgba([200, 10, 20, 255]));
    let ctx = eframe::egui::Context::default();
    let color = ColorImage::from_rgba_unmultiplied([16, 16], rgba.as_raw());
    let texture = ctx.load_texture("scanner-upstream-fixture", color, TextureOptions::LINEAR);
    let edges = build_edge_paths(&rgba);
    let mut assets = RuntimeAssets::default();
    assets.images.insert(layer_id, RuntimeImage { texture, rgba, edges });

    let frame = export::render_frame(&project, &assets, 0.5).expect("render scanner fixture");
    assert_eq!(frame.dimensions(), (64, 64));

    // Upstream Scanner uses 12 horizontal bands. At 0.5 progress exactly six
    // bands are complete, therefore a 32x32 layer reveals exactly its top 16 rows.
    for y in 0..64 {
        for x in 0..64 {
            let expected = if (16..48).contains(&x) && (16..32).contains(&y) {
                Rgba([200, 10, 20, 255])
            } else {
                Rgba([255, 255, 255, 255])
            };
            assert_eq!(*frame.get_pixel(x, y), expected, "pixel mismatch at ({x}, {y})");
        }
    }
}

#[test]
fn project_json_roundtrip_preserves_editor_state_and_group_metadata() {
    let mut project = Project::default();
    project.name = "Parity regression".to_owned();
    project.duration_seconds = 13.25;
    project.hand_glide_enabled = false;
    let mut layer = Layer::new_text(project.canvas.width, project.canvas.height);
    layer.name = "Title".to_owned();
    layer.anim_order = Some(7);
    layer.group = Some("Intro".to_owned());
    layer.animation.style = AnimationStyle::TextDraw;
    layer.transform.x = 123.0;
    layer.transform.y = 77.0;
    project.layers.push(layer);

    let json = serde_json::to_string(&project).expect("serialize");
    let restored: Project = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.name, project.name);
    assert_eq!(restored.duration_seconds, project.duration_seconds);
    assert_eq!(restored.hand_glide_enabled, project.hand_glide_enabled);
    assert_eq!(restored.layers.len(), 1);
    assert_eq!(restored.layers[0].name, "Title");
    assert_eq!(restored.layers[0].anim_order, Some(7));
    assert_eq!(restored.layers[0].group.as_deref(), Some("Intro"));
    assert_eq!(restored.layers[0].animation.style, AnimationStyle::TextDraw);
    assert_eq!(restored.layers[0].transform.x, 123.0);
    assert_eq!(restored.layers[0].transform.y, 77.0);
}

#[test]
fn project_file_roundtrip_works_without_assets() {
    let mut project = Project::default();
    project.name = "Persistence regression".to_owned();
    project.layers.push(Layer::new_text(project.canvas.width, project.canvas.height));

    let root = std::env::temp_dir().join(format!("inkplainer-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).expect("create temp test dir");
    let path = root.join("roundtrip.inkplainer.json");
    project_io::save_project(&project, &path).expect("save project");
    let restored = project_io::load_project(&path).expect("load project");
    assert_eq!(restored.name, project.name);
    assert_eq!(restored.layers.len(), 1);
    let _ = fs::remove_dir_all(root);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Mp4Box {
    kind: [u8; 4],
    size: usize,
}

fn mp4_top_level_boxes(bytes: &[u8]) -> Result<Vec<Mp4Box>, String> {
    let mut boxes = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        if bytes.len() - offset < 8 {
            return Err(format!("truncated MP4 box header at offset {offset}"));
        }

        let size32 = u32::from_be_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .map_err(|_| "invalid MP4 size field")?,
        );
        let kind: [u8; 4] = bytes[offset + 4..offset + 8]
            .try_into()
            .map_err(|_| "invalid MP4 box type")?;

        let (size, header_size) = match size32 {
            0 => (bytes.len() - offset, 8usize),
            1 => {
                if bytes.len() - offset < 16 {
                    return Err(format!("truncated extended MP4 box header at offset {offset}"));
                }
                let extended = u64::from_be_bytes(
                    bytes[offset + 8..offset + 16]
                        .try_into()
                        .map_err(|_| "invalid extended MP4 size field")?,
                );
                let size = usize::try_from(extended)
                    .map_err(|_| "extended MP4 box size exceeds usize")?;
                (size, 16usize)
            }
            value => (value as usize, 8usize),
        };

        if size < header_size {
            return Err(format!("invalid MP4 box size {size} at offset {offset}"));
        }
        if size > bytes.len() - offset {
            return Err(format!("MP4 box overruns file at offset {offset}"));
        }

        boxes.push(Mp4Box { kind, size });
        offset += size;
        if size32 == 0 {
            break;
        }
    }
    Ok(boxes)
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|window| window == needle)
}

#[test]
fn native_video_encoders_emit_structurally_valid_mp4_and_webm_containers() {
    let mut project = Project::default();
    project.canvas.width = 320;
    project.canvas.height = 240;
    project.duration_seconds = 0.5;
    project.export.width = 320;
    project.export.height = 240;
    project.export.fps = 12;
    project.export.quality = ExportQuality::Low;
    project.layers.clear();
    assert_eq!(export_scale::resolved_export_dimensions(&project), (426, 320));

    let assets = RuntimeAssets::default();
    let root = std::env::temp_dir().join(format!("inkplainer-codec-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).expect("create codec test dir");
    let mp4_path = root.join("smoke.mp4");
    let webm_path = root.join("smoke.webm");

    let mp4_progress = AtomicU32::new(0);
    let mp4_cancel = AtomicBool::new(false);
    native_mp4::export_mp4(&project, &assets, &mp4_path, &mp4_progress, &mp4_cancel)
        .expect("pure Rust MP4 export");
    let mp4 = fs::read(&mp4_path).expect("read MP4");
    assert!(mp4.len() > 64, "MP4 output unexpectedly small");
    let boxes = mp4_top_level_boxes(&mp4).expect("parse top-level MP4 boxes");
    assert_eq!(boxes.first().map(|entry| entry.kind), Some(*b"ftyp"));
    assert!(boxes.iter().any(|entry| entry.kind == *b"moov" && entry.size > 8));
    assert!(boxes.iter().any(|entry| entry.kind == *b"mdat" && entry.size > 8));
    assert!(contains_bytes(&mp4, b"avc1"), "MP4 must declare an AVC/H.264 sample entry");
    assert!(contains_bytes(&mp4, b"avcC"), "MP4 must contain AVC decoder configuration");
    assert_eq!(mp4_progress.load(Ordering::Relaxed), 1000);

    let webm_progress = AtomicU32::new(0);
    let webm_cancel = AtomicBool::new(false);
    native_webm::export_webm(&project, &assets, &webm_path, &webm_progress, &webm_cancel)
        .expect("pure Rust WebM export");
    let webm = fs::read(&webm_path).expect("read WebM");
    assert!(webm.len() > 64, "WebM output unexpectedly small");
    assert_eq!(&webm[..4], &[0x1a, 0x45, 0xdf, 0xa3], "WebM must start with the EBML header");
    assert!(contains_bytes(&webm, &[0x18, 0x53, 0x80, 0x67]), "WebM must contain a Segment element");
    assert!(contains_bytes(&webm, &[0x16, 0x54, 0xae, 0x6b]), "WebM must contain Tracks");
    assert!(contains_bytes(&webm, &[0x1f, 0x43, 0xb6, 0x75]), "WebM must contain a Cluster");
    assert!(contains_bytes(&webm, b"V_AV1"), "WebM must declare an AV1 video track");
    assert_eq!(webm_progress.load(Ordering::Relaxed), 1000);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn cancelled_video_exports_do_not_leave_partial_files() {
    let mut project = Project::default();
    project.canvas.width = 320;
    project.canvas.height = 240;
    project.duration_seconds = 0.5;
    project.export.width = 320;
    project.export.height = 240;
    project.export.fps = 12;
    project.export.quality = ExportQuality::Low;
    project.layers.clear();

    let assets = RuntimeAssets::default();
    let root = std::env::temp_dir().join(format!("inkplainer-cancel-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).expect("create cancel test dir");

    let mp4_path = root.join("cancelled.mp4");
    fs::write(&mp4_path, b"stale").expect("seed stale MP4 file");
    let progress = AtomicU32::new(0);
    let cancel = AtomicBool::new(true);
    let result = native_mp4::export_mp4(&project, &assets, &mp4_path, &progress, &cancel);
    assert_eq!(result.as_deref().err(), Some("Export cancelled"));
    assert!(!mp4_path.exists(), "cancelled MP4 must remove stale/partial output");

    let webm_path = root.join("cancelled.webm");
    fs::write(&webm_path, b"stale").expect("seed stale WebM file");
    let progress = AtomicU32::new(0);
    let cancel = AtomicBool::new(true);
    let result = native_webm::export_webm(&project, &assets, &webm_path, &progress, &cancel);
    assert_eq!(result.as_deref().err(), Some("Export cancelled"));
    assert!(!webm_path.exists(), "cancelled WebM must remove stale/partial output");

    let _ = fs::remove_dir_all(root);
}
