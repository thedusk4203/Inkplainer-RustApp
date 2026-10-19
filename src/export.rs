use ab_glyph::{FontArc, PxScale};
use anyhow::{bail, Context, Result};
use image::{imageops::FilterType, Rgba, RgbaImage};
use imageproc::drawing::{draw_line_segment_mut, draw_text_mut, text_size};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::animation::{layer_progress, reveal_regions, RevealRegion};
use crate::assets::{RuntimeAssets, RuntimeImage};
use crate::model::{AnimationStyle, Background, Layer, LayerContent, Project, TextAlign};

pub fn export_png(project: &Project, assets: &RuntimeAssets, path: &Path) -> Result<()> {
    let frame = render_frame(project, assets, 1.0)?;
    frame.save(path).with_context(|| format!("Save {}", path.display()))?;
    Ok(())
}

pub fn export_mp4(project: &Project, assets: &RuntimeAssets, path: &Path, fps: u32) -> Result<()> {
    if !ffmpeg_available() {
        bail!("ffmpeg was not found in PATH. Install ffmpeg or use PNG export.");
    }
    let fps = fps.clamp(12, 60);
    let frame_count = (project.duration_seconds.max(0.5) * fps as f32).ceil() as u32;
    let temp = std::env::temp_dir().join(format!("inkplainer-export-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp).context("Create export temp directory")?;

    let result = (|| -> Result<()> {
        for i in 0..frame_count {
            let p = if frame_count <= 1 { 1.0 } else { i as f32 / (frame_count - 1) as f32 };
            let frame = render_frame(project, assets, p)?;
            frame
                .save(temp.join(format!("frame-{i:06}.png")))
                .with_context(|| format!("Render frame {i}"))?;
        }

        let input = temp.join("frame-%06d.png");
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-framerate",
                &fps.to_string(),
                "-i",
            ])
            .arg(&input)
            .args([
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-movflags",
                "+faststart",
            ])
            .arg(path)
            .status()
            .context("Run ffmpeg")?;
        if !status.success() {
            bail!("ffmpeg exited with status {status}");
        }
        Ok(())
    })();

    let _ = fs::remove_dir_all(&temp);
    result
}

pub fn render_frame(project: &Project, assets: &RuntimeAssets, global_progress: f32) -> Result<RgbaImage> {
    let mut canvas = background_image(project);
    let font = load_windows_font();

    for (idx, layer) in project.layers.iter().enumerate() {
        if !layer.visible {
            continue;
        }
        let progress = layer_progress(project, idx, global_progress);
        if progress <= 0.0 {
            continue;
        }
        match &layer.content {
            LayerContent::Image { .. } => {
                if let Some(runtime) = assets.images.get(&layer.id) {
                    render_image_layer(&mut canvas, runtime, layer, progress);
                }
            }
            LayerContent::Text(text) => {
                if let Some(font) = font.as_ref() {
                    let alpha_progress = if matches!(layer.animation.style, AnimationStyle::TextDraw) {
                        progress
                    } else {
                        progress.min(1.0)
                    };
                    let color = Rgba([
                        text.color[0],
                        text.color[1],
                        text.color[2],
                        ((text.color[3] as f32) * layer.opacity * alpha_progress).clamp(0.0, 255.0) as u8,
                    ]);
                    let scale = PxScale::from(text.size.max(8.0));
                    let (tw, th) = text_size(scale, font, &text.text);
                    let x = match text.align {
                        TextAlign::Left => layer.transform.x,
                        TextAlign::Center => layer.transform.x + (layer.transform.w - tw as f32) * 0.5,
                        TextAlign::Right => layer.transform.x + layer.transform.w - tw as f32,
                    };
                    let y = layer.transform.y + (layer.transform.h - th as f32) * 0.5;
                    draw_text_mut(&mut canvas, color, x.round() as i32, y.round() as i32, scale, font, &text.text);
                }
            }
        }
    }
    Ok(canvas)
}

fn render_image_layer(canvas: &mut RgbaImage, runtime: &RuntimeImage, layer: &Layer, progress: f32) {
    let w = layer.transform.w.round().max(1.0) as u32;
    let h = layer.transform.h.round().max(1.0) as u32;
    let resized = image::imageops::resize(&runtime.rgba, w, h, FilterType::Lanczos3);
    let x = layer.transform.x.round() as i32;
    let y = layer.transform.y.round() as i32;

    match layer.animation.style {
        AnimationStyle::Contour => {
            if progress < 0.35 {
                draw_edge(canvas, runtime, layer, progress / 0.35);
            } else {
                draw_edge(canvas, runtime, layer, 1.0);
                let mut scanner = layer.clone();
                scanner.animation.style = AnimationStyle::Scanner;
                let regions = reveal_regions(&scanner, (progress - 0.35) / 0.65);
                blend_regions(canvas, &resized, x, y, &regions, layer.opacity);
            }
        }
        AnimationStyle::OutlineFill | AnimationStyle::IllustFill => {
            let outline_phase = 0.55;
            if progress <= outline_phase {
                draw_edge(canvas, runtime, layer, progress / outline_phase);
            } else {
                draw_edge(canvas, runtime, layer, 1.0);
                if layer.animation.reveal_original {
                    let fade = ((progress - outline_phase) / (1.0 - outline_phase)).clamp(0.0, 1.0);
                    blend_regions(canvas, &resized, x, y, &[RevealRegion::full()], layer.opacity * fade);
                }
            }
        }
        AnimationStyle::OutlineOnly => draw_edge(canvas, runtime, layer, progress),
        _ => {
            let regions = reveal_regions(layer, progress);
            blend_regions(canvas, &resized, x, y, &regions, layer.opacity);
        }
    }
}

fn draw_edge(canvas: &mut RgbaImage, runtime: &RuntimeImage, layer: &Layer, progress: f32) {
    let pts = &runtime.edge_path;
    if pts.len() < 2 {
        return;
    }
    let count = ((pts.len() as f32 * progress.clamp(0.0, 1.0)).ceil() as usize).clamp(2, pts.len());
    let color = Rgba([
        layer.animation.outline_color[0],
        layer.animation.outline_color[1],
        layer.animation.outline_color[2],
        ((layer.animation.outline_color[3] as f32) * layer.opacity).clamp(0.0, 255.0) as u8,
    ]);
    let max_jump = layer.transform.w.max(layer.transform.h) * 0.12;
    for pair in pts[..count].windows(2) {
        let a = (
            layer.transform.x + pair[0][0] * layer.transform.w,
            layer.transform.y + pair[0][1] * layer.transform.h,
        );
        let b = (
            layer.transform.x + pair[1][0] * layer.transform.w,
            layer.transform.y + pair[1][1] * layer.transform.h,
        );
        let dx = a.0 - b.0;
        let dy = a.1 - b.1;
        if (dx * dx + dy * dy).sqrt() <= max_jump {
            draw_line_segment_mut(canvas, a, b, color);
        }
    }
}

fn blend_regions(
    canvas: &mut RgbaImage,
    source: &RgbaImage,
    offset_x: i32,
    offset_y: i32,
    regions: &[RevealRegion],
    opacity: f32,
) {
    let sw = source.width();
    let sh = source.height();
    for r in regions {
        let x0 = (r.u0 * sw as f32).floor().clamp(0.0, sw as f32) as u32;
        let y0 = (r.v0 * sh as f32).floor().clamp(0.0, sh as f32) as u32;
        let x1 = (r.u1 * sw as f32).ceil().clamp(0.0, sw as f32) as u32;
        let y1 = (r.v1 * sh as f32).ceil().clamp(0.0, sh as f32) as u32;
        for sy in y0..y1 {
            for sx in x0..x1 {
                let dx = offset_x + sx as i32;
                let dy = offset_y + sy as i32;
                if dx < 0 || dy < 0 || dx >= canvas.width() as i32 || dy >= canvas.height() as i32 {
                    continue;
                }
                let mut src = *source.get_pixel(sx, sy);
                src.0[3] = ((src.0[3] as f32) * opacity.clamp(0.0, 1.0)) as u8;
                let dst = canvas.get_pixel_mut(dx as u32, dy as u32);
                alpha_over(dst, src);
            }
        }
    }
}

fn alpha_over(dst: &mut Rgba<u8>, src: Rgba<u8>) {
    let sa = src.0[3] as f32 / 255.0;
    if sa <= 0.0 {
        return;
    }
    let da = dst.0[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.0 {
        *dst = Rgba([0, 0, 0, 0]);
        return;
    }
    for c in 0..3 {
        let s = src.0[c] as f32 / 255.0;
        let d = dst.0[c] as f32 / 255.0;
        dst.0[c] = (((s * sa + d * da * (1.0 - sa)) / out_a) * 255.0).round() as u8;
    }
    dst.0[3] = (out_a * 255.0).round() as u8;
}

fn background_image(project: &Project) -> RgbaImage {
    let (w, h) = (project.canvas.width.max(1), project.canvas.height.max(1));
    let base = match project.canvas.background {
        Background::Solid(c) => Rgba(c),
        Background::Transparent => Rgba([0, 0, 0, 0]),
        Background::Notebook | Background::Graph => Rgba([250, 250, 247, 255]),
        Background::Blueprint => Rgba([20, 55, 92, 255]),
        Background::Kraft => Rgba([198, 164, 113, 255]),
        Background::Dark => Rgba([28, 30, 34, 255]),
    };
    let mut img = RgbaImage::from_pixel(w, h, base);

    match project.canvas.background {
        Background::Notebook => {
            let step = (h / 36).max(12);
            let line = Rgba([205, 218, 235, 255]);
            let mut y = step;
            while y < h {
                for x in 0..w {
                    img.put_pixel(x, y, line);
                }
                y += step;
            }
        }
        Background::Graph => {
            let step = (w / 32).max(12);
            let line = Rgba([220, 222, 226, 255]);
            let mut x = 0;
            while x < w {
                for y in 0..h {
                    img.put_pixel(x, y, line);
                }
                x += step;
            }
            let mut y = 0;
            while y < h {
                for x in 0..w {
                    img.put_pixel(x, y, line);
                }
                y += step;
            }
        }
        _ => {}
    }
    img
}

fn load_windows_font() -> Option<FontArc> {
    let windir = std::env::var_os("WINDIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let candidates = [
        windir.join("Fonts").join("segoeui.ttf"),
        windir.join("Fonts").join("arial.ttf"),
    ];
    for path in candidates {
        if let Ok(bytes) = fs::read(path) {
            if let Ok(font) = FontArc::try_from_vec(bytes) {
                return Some(font);
            }
        }
    }
    None
}

fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
