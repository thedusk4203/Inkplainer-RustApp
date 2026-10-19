use ab_glyph::PxScale;
use anyhow::{Context, Result};
use image::{imageops::FilterType, Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_circle_mut, draw_line_segment_mut, draw_text_mut, text_size};
use std::path::Path;

use crate::animation::{layer_progress, reveal_regions, RevealRegion};
use crate::assets::{RuntimeAssets, RuntimeImage};
use crate::effects::select_edge_path;
use crate::font_catalog;
use crate::model::{
    AnimationStyle, Background, ColoringStyle, HandStyle, Layer, LayerContent, Project,
    RevealStyle, StrokeStyle, TextAlign, TextDirection, TextDrawStyle,
};
use crate::timing::hand_motion_progress;

pub fn export_png(project: &Project, assets: &RuntimeAssets, path: &Path) -> Result<()> {
    let frame = render_frame(project, assets, 1.0)?;
    frame.save(path).with_context(|| format!("Save {}", path.display()))?;
    Ok(())
}

pub fn render_frame(project: &Project, assets: &RuntimeAssets, global_progress: f32) -> Result<RgbaImage> {
    let mut canvas = background_image(project);
    for (idx, layer) in project.layers.iter().enumerate() {
        if !layer.visible { continue; }
        let progress = layer_progress(project, idx, global_progress);
        if progress <= 0.0 { continue; }
        match &layer.content {
            LayerContent::Image { .. } => {
                if let Some(runtime) = assets.images.get(&layer.id) {
                    render_image_layer(
                        &mut canvas,
                        runtime,
                        layer,
                        progress,
                        project.hand_glide_enabled,
                        assets,
                        project,
                    );
                }
            }
            LayerContent::Text(text) => render_text_layer(&mut canvas, layer, text, progress),
        }
    }
    Ok(canvas)
}

fn render_text_layer(canvas: &mut RgbaImage, layer: &Layer, text: &crate::model::TextLayer, progress: f32) {
    let Some(font) = font_catalog::load_font_arc(&text.font_family, text.bold, text.italic) else { return; };
    let w = layer.transform.w.round().max(1.0) as u32;
    let h = layer.transform.h.round().max(1.0) as u32;
    let mut local = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0]));
    let color = Rgba([
        text.color[0], text.color[1], text.color[2],
        ((text.color[3] as f32) * layer.opacity).clamp(0.0, 255.0) as u8,
    ]);
    let scale = PxScale::from(text.size.max(8.0));
    let lines: Vec<&str> = text.text.lines().collect();
    let line_h = (text.size * text.line_height.max(0.8)).round() as i32;
    let total_h = line_h * lines.len().max(1) as i32;
    let mut y = ((h as i32 - total_h) / 2).max(0);
    for line in lines {
        let (tw, _) = text_size(scale, &font, line);
        let x = match text.align {
            TextAlign::Left => 0,
            TextAlign::Center => ((w as i32 - tw as i32) / 2).max(0),
            TextAlign::Right => (w as i32 - tw as i32).max(0),
        };
        if text.letter_spacing.abs() < 0.01 {
            draw_text_mut(&mut local, color, x, y, scale, &font, line);
        } else {
            let mut cx = x as f32;
            for ch in line.chars() {
                let s = ch.to_string();
                draw_text_mut(&mut local, color, cx.round() as i32, y, scale, &font, &s);
                let (cw, _) = text_size(scale, &font, &s);
                cx += cw as f32 + text.letter_spacing;
            }
        }
        y += line_h;
    }

    if layer.animation.style == AnimationStyle::TextDraw {
        match layer.animation.text_draw_style {
            TextDrawStyle::Reveal | TextDrawStyle::OutlineFill => {
                mask_text_direction(&mut local, progress, layer.animation.text_direction)
            }
            TextDrawStyle::Outline => {
                let outlined = text_outline(&local, layer.animation.outline_color);
                local = outlined;
                mask_text_direction(&mut local, progress, layer.animation.text_direction);
            }
        }
    }
    blend_regions(
        canvas,
        &local,
        layer.transform.x.round() as i32,
        layer.transform.y.round() as i32,
        &[RevealRegion::full()],
        1.0,
    );
}

fn text_outline(src: &RgbaImage, color: [u8; 4]) -> RgbaImage {
    let (w, h) = src.dimensions();
    let mut out = RgbaImage::from_pixel(w, h, Rgba([0, 0, 0, 0]));
    if w < 3 || h < 3 { return out; }
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            if src.get_pixel(x, y).0[3] == 0 { continue; }
            let boundary = src.get_pixel(x - 1, y).0[3] == 0
                || src.get_pixel(x + 1, y).0[3] == 0
                || src.get_pixel(x, y - 1).0[3] == 0
                || src.get_pixel(x, y + 1).0[3] == 0;
            if boundary { out.put_pixel(x, y, Rgba(color)); }
        }
    }
    out
}

fn mask_text_direction(img: &mut RgbaImage, progress: f32, direction: TextDirection) {
    let p = progress.clamp(0.0, 1.0);
    let (w, h) = img.dimensions();
    for y in 0..h {
        for x in 0..w {
            let keep = match direction {
                TextDirection::LeftToRight => x as f32 <= w as f32 * p,
                TextDirection::RightToLeft => x as f32 >= w as f32 * (1.0 - p),
                TextDirection::TopToBottom => y as f32 <= h as f32 * p,
                TextDirection::BottomToTop => y as f32 >= h as f32 * (1.0 - p),
            };
            if !keep { img.get_pixel_mut(x, y).0[3] = 0; }
        }
    }
}

fn render_image_layer(
    canvas: &mut RgbaImage,
    runtime: &RuntimeImage,
    layer: &Layer,
    progress: f32,
    smooth_hand: bool,
    assets: &RuntimeAssets,
    project: &Project,
) {
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
                blend_regions(
                    canvas,
                    &resized,
                    x,
                    y,
                    &reveal_regions(&scanner, (progress - 0.35) / 0.65),
                    layer.opacity,
                );
            }
        }
        AnimationStyle::OutlineFill => {
            render_outline_fill(canvas, &resized, runtime, layer, progress, false);
        }
        AnimationStyle::IllustFill => {
            render_outline_fill(canvas, &resized, runtime, layer, progress, true);
        }
        AnimationStyle::OutlineOnly => {
            if layer.animation.outline_visible { draw_edge(canvas, runtime, layer, progress); }
        }
        _ => blend_regions(canvas, &resized, x, y, &reveal_regions(layer, progress), layer.opacity),
    }
    draw_hand(canvas, runtime, layer, progress, smooth_hand, assets, project);
}

fn render_outline_fill(
    canvas: &mut RgbaImage,
    resized: &RgbaImage,
    runtime: &RuntimeImage,
    layer: &Layer,
    progress: f32,
    illustrated: bool,
) {
    let outline_phase = if illustrated { 0.45 } else { 0.55 };
    if progress <= outline_phase {
        if layer.animation.outline_visible {
            draw_edge(canvas, runtime, layer, progress / outline_phase);
        }
        return;
    }
    if layer.animation.outline_visible { draw_edge(canvas, runtime, layer, 1.0); }
    let fill = ((progress - outline_phase) / (1.0 - outline_phase)).clamp(0.0, 1.0);
    if illustrated {
        match layer.animation.coloring_style {
            ColoringStyle::Sparse => {
                let mut l = layer.clone();
                l.animation.style = AnimationStyle::ChunkJump;
                l.animation.chunks = 80;
                blend_regions(
                    canvas,
                    resized,
                    layer.transform.x.round() as i32,
                    layer.transform.y.round() as i32,
                    &reveal_regions(&l, fill * 0.82),
                    layer.opacity * 0.62,
                );
            }
            ColoringStyle::Filled => blend_reveal_style(canvas, resized, layer, fill, layer.opacity),
            ColoringStyle::Watercolor => {
                for i in 0..3 {
                    blend_reveal_style(
                        canvas,
                        resized,
                        layer,
                        (fill + i as f32 * 0.06).clamp(0.0, 1.0),
                        layer.opacity * 0.28,
                    );
                }
            }
        }
    } else if layer.animation.reveal_original {
        blend_reveal_style(canvas, resized, layer, fill, layer.opacity);
    }
}

fn blend_reveal_style(
    canvas: &mut RgbaImage,
    source: &RgbaImage,
    layer: &Layer,
    progress: f32,
    opacity: f32,
) {
    let p = progress.clamp(0.0, 1.0);
    let x = layer.transform.x.round() as i32;
    let y = layer.transform.y.round() as i32;
    match layer.animation.reveal_style {
        RevealStyle::Instant => {
            if p > 0.0 { blend_regions(canvas, source, x, y, &[RevealRegion::full()], opacity); }
        }
        RevealStyle::Fade => blend_regions(
            canvas,
            source,
            x,
            y,
            &[RevealRegion::full()],
            opacity * (p * p * (3.0 - 2.0 * p)),
        ),
        RevealStyle::WipeRight => blend_regions(
            canvas,
            source,
            x,
            y,
            &[RevealRegion { u0: 0.0, v0: 0.0, u1: p, v1: 1.0 }],
            opacity,
        ),
        RevealStyle::Dissolve => {
            let mut l = layer.clone();
            l.animation.style = AnimationStyle::ChunkJump;
            l.animation.chunks = 80;
            blend_regions(canvas, source, x, y, &reveal_regions(&l, p), opacity);
        }
        RevealStyle::ScanLines => {
            let mut regions = Vec::new();
            for s in 0..18 {
                let offset = s as f32 / 18.0 * 0.35;
                let local = ((p - offset) / 0.65).clamp(0.0, 1.0);
                if local <= 0.0 { continue; }
                let v0 = s as f32 / 18.0;
                let v1 = (s + 1) as f32 / 18.0;
                let (u0, u1) = if s % 2 == 0 { (0.0, local) } else { (1.0 - local, 1.0) };
                regions.push(RevealRegion { u0, v0, u1, v1 });
            }
            blend_regions(canvas, source, x, y, &regions, opacity);
        }
        RevealStyle::Iris => blend_iris(canvas, source, x, y, p, opacity),
    }
}

fn blend_iris(
    canvas: &mut RgbaImage,
    source: &RgbaImage,
    ox: i32,
    oy: i32,
    progress: f32,
    opacity: f32,
) {
    let cx = source.width() as f32 * 0.5;
    let cy = source.height() as f32 * 0.5;
    let max_r = (cx * cx + cy * cy).sqrt();
    let r2 = (max_r * progress).powi(2);
    for sy in 0..source.height() {
        for sx in 0..source.width() {
            let dx0 = sx as f32 - cx;
            let dy0 = sy as f32 - cy;
            if dx0 * dx0 + dy0 * dy0 > r2 { continue; }
            blend_one(canvas, *source.get_pixel(sx, sy), ox + sx as i32, oy + sy as i32, opacity);
        }
    }
}

fn draw_edge(canvas: &mut RgbaImage, runtime: &RuntimeImage, layer: &Layer, progress: f32) {
    let pts = select_edge_path(&runtime.edges, &layer.animation);
    if pts.len() < 2 { return; }
    let sensitivity = layer.animation.outline_detection.clamp(1, 100) as f32 / 100.0;
    let effective_len = ((pts.len() as f32) * (0.30 + sensitivity * 0.70)).round() as usize;
    let effective_len = effective_len.clamp(2, pts.len());
    let count = ((effective_len as f32 * progress.clamp(0.0, 1.0)).ceil() as usize)
        .clamp(2, effective_len);
    let mut c = layer.animation.outline_color;
    c[3] = ((c[3] as f32) * layer.opacity * layer.animation.outline_opacity)
        .clamp(0.0, 255.0) as u8;
    if layer.animation.stroke_style == StrokeStyle::Blueprint { c = [118, 210, 255, c[3]]; }
    let color = Rgba(c);
    let max_jump = layer.transform.w.max(layer.transform.h) * 0.09;
    let thickness = layer.animation.outline_thickness.max(0.5).round() as i32;
    for (i, pair) in pts[..count].windows(2).enumerate() {
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
        if (dx * dx + dy * dy).sqrt() > max_jump { continue; }
        match layer.animation.stroke_style {
            StrokeStyle::Sketch => {
                for off in [-1.5f32, -0.5, 0.5, 1.5] {
                    draw_line_segment_mut(
                        canvas,
                        (a.0 + off, a.1 - off * 0.4),
                        (b.0 + off, b.1 - off * 0.4),
                        Rgba([c[0], c[1], c[2], c[3] / 2]),
                    );
                }
            }
            StrokeStyle::Charcoal => draw_thick_line(
                canvas,
                a,
                b,
                Rgba([c[0], c[1], c[2], c[3] / 2]),
                thickness * 2,
            ),
            StrokeStyle::Fountain => draw_thick_line(
                canvas,
                a,
                b,
                color,
                ((thickness as f32) * (0.7 + (i as f32 * 0.17).sin().abs())).round() as i32,
            ),
            _ => draw_thick_line(canvas, a, b, color, thickness),
        }
    }
}

fn draw_thick_line(
    img: &mut RgbaImage,
    a: (f32, f32),
    b: (f32, f32),
    color: Rgba<u8>,
    thickness: i32,
) {
    let t = thickness.max(1);
    for o in -(t / 2)..=(t / 2) {
        draw_line_segment_mut(img, (a.0 + o as f32, a.1), (b.0 + o as f32, b.1), color);
        if t > 2 {
            draw_line_segment_mut(img, (a.0, a.1 + o as f32), (b.0, b.1 + o as f32), color);
        }
    }
}

fn draw_hand(
    canvas: &mut RgbaImage,
    runtime: &RuntimeImage,
    layer: &Layer,
    progress: f32,
    smooth_hand: bool,
    assets: &RuntimeAssets,
    project: &Project,
) {
    if layer.animation.hand == HandStyle::Ghost || progress <= 0.0 || progress >= 1.0 { return; }
    let (tx, ty) = if smooth_hand {
        smoothed_hand_tip(runtime, layer, progress)
    } else {
        hand_tip(runtime, layer, progress)
    };

    if let Some(hand) = assets.hand(layer.animation.hand, project.canvas.width, project.canvas.height) {
        if hand.rgba.width() > 1 || hand.rgba.height() > 1 {
            let ox = tx.round() as i32 - hand.tip_x as i32;
            let oy = ty.round() as i32 - hand.tip_y as i32;
            for sy in 0..hand.rgba.height() {
                for sx in 0..hand.rgba.width() {
                    let px = *hand.rgba.get_pixel(sx, sy);
                    if px.0[3] == 0 { continue; }
                    blend_one(canvas, px, ox + sx as i32, oy + sy as i32, 1.0);
                }
            }
            return;
        }
    }

    // Procedural fallback is retained only for local/offline builds where the
    // exact pinned upstream sprite library could not be staged by build.rs.
    let s = (layer.transform.w.min(layer.transform.h) * 0.045).clamp(8.0, 42.0);
    let dark = Rgba([45, 45, 50, 250]);
    match layer.animation.hand {
        HandStyle::Ghost => {}
        HandStyle::Hand1 => {
            draw_filled_circle_mut(canvas, ((tx + s * 0.82) as i32, (ty + s * 0.96) as i32), (s * 0.70) as i32, Rgba([231, 190, 158, 235]));
            draw_filled_circle_mut(canvas, ((tx + s * 0.43) as i32, (ty + s * 0.54) as i32), (s * 0.24) as i32, Rgba([242, 205, 175, 245]));
            draw_thick_line(canvas, (tx, ty), (tx + s * 1.48, ty + s * 1.68), dark, (s * 0.24) as i32);
        }
        HandStyle::Hand2 => {
            draw_filled_circle_mut(canvas, ((tx + s * 0.72) as i32, (ty + s * 1.12) as i32), (s * 0.76) as i32, Rgba([176, 118, 84, 240]));
            fill_rect(canvas, tx + s * 0.70, ty + s * 1.12, s * 0.95, s, Rgba([72, 91, 131, 245]));
            draw_thick_line(canvas, (tx, ty), (tx + s * 1.35, ty + s * 1.78), Rgba([30, 31, 35, 250]), (s * 0.25) as i32);
        }
        HandStyle::Hand3 => {
            draw_filled_circle_mut(canvas, ((tx + s * 0.92) as i32, (ty + s * 0.92) as i32), (s * 0.72) as i32, Rgba([246, 211, 184, 242]));
            fill_rect(canvas, tx + s * 0.78, ty + s * 1.13, s * 1.12, s * 0.82, Rgba([205, 218, 232, 245]));
            draw_thick_line(canvas, (tx, ty), (tx + s * 1.62, ty + s * 1.55), Rgba([62, 50, 44, 250]), (s * 0.22) as i32);
        }
        HandStyle::Pen => {
            draw_thick_line(canvas, (tx + s * 1.8, ty + s * 1.6), (tx, ty), dark, (s * 0.28) as i32);
            draw_filled_circle_mut(canvas, ((tx + s * 1.8) as i32, (ty + s * 1.6) as i32), (s * 0.24) as i32, Rgba([210, 210, 215, 255]));
        }
        HandStyle::Cartoon => {
            draw_filled_circle_mut(canvas, ((tx + s * 0.9) as i32, (ty + s * 0.95) as i32), (s * 0.78) as i32, Rgba([255, 220, 157, 250]));
            draw_thick_line(canvas, (tx, ty), (tx + s * 1.55, ty + s * 1.70), Rgba([54, 64, 83, 255]), (s * 0.27) as i32);
        }
        HandStyle::PixelArt => {
            fill_rect(canvas, tx - s * 0.2, ty + s * 0.4, s * 1.6, s * 1.25, Rgba([228, 183, 145, 245]));
            draw_thick_line(canvas, (tx, ty), (tx + s * 0.9, ty + s * 0.95), Rgba([48, 48, 52, 255]), (s * 0.28) as i32);
        }
    }
}

fn fill_rect(img: &mut RgbaImage, x: f32, y: f32, w: f32, h: f32, color: Rgba<u8>) {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = (x + w).ceil() as i32;
    let y1 = (y + h).ceil() as i32;
    for py in y0..y1 {
        for px in x0..x1 {
            if px >= 0 && py >= 0 && px < img.width() as i32 && py < img.height() as i32 {
                blend_one(img, color, px, py, 1.0);
            }
        }
    }
}

fn smoothed_hand_tip(runtime: &RuntimeImage, layer: &Layer, progress: f32) -> (f32, f32) {
    const SMOOTHING_SPAN: f32 = 0.024;
    let mut sx = 0.0;
    let mut sy = 0.0;
    let mut sw = 0.0;
    for sample in 0..6 {
        let t = sample as f32 / 5.0;
        let p = (progress - SMOOTHING_SPAN * t).clamp(0.0, 1.0);
        let (x, y) = hand_tip(runtime, layer, p);
        let weight = 1.0 - t * 0.65;
        sx += x * weight;
        sy += y * weight;
        sw += weight;
    }
    (sx / sw.max(0.001), sy / sw.max(0.001))
}

fn hand_tip(runtime: &RuntimeImage, layer: &Layer, progress: f32) -> (f32, f32) {
    let progress = hand_motion_progress(progress, layer.animation.hand_speed);
    if matches!(
        layer.animation.style,
        AnimationStyle::Contour
            | AnimationStyle::OutlineFill
            | AnimationStyle::IllustFill
            | AnimationStyle::OutlineOnly
    ) {
        let pts = select_edge_path(&runtime.edges, &layer.animation);
        if !pts.is_empty() {
            let i = ((pts.len() - 1) as f32 * progress.clamp(0.0, 1.0)).round() as usize;
            return (
                layer.transform.x + pts[i][0] * layer.transform.w,
                layer.transform.y + pts[i][1] * layer.transform.h,
            );
        }
    }
    let regions = reveal_regions(layer, progress);
    if let Some(r) = regions.last() {
        return (
            layer.transform.x + (r.u0 + r.u1) * 0.5 * layer.transform.w,
            layer.transform.y + (r.v0 + r.v1) * 0.5 * layer.transform.h,
        );
    }
    (
        layer.transform.x + layer.transform.w * 0.5,
        layer.transform.y + layer.transform.h * 0.5,
    )
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
                blend_one(
                    canvas,
                    *source.get_pixel(sx, sy),
                    offset_x + sx as i32,
                    offset_y + sy as i32,
                    opacity,
                );
            }
        }
    }
}

fn blend_one(canvas: &mut RgbaImage, mut src: Rgba<u8>, dx: i32, dy: i32, opacity: f32) {
    if dx < 0 || dy < 0 || dx >= canvas.width() as i32 || dy >= canvas.height() as i32 { return; }
    src.0[3] = ((src.0[3] as f32) * opacity.clamp(0.0, 1.0)) as u8;
    alpha_over(canvas.get_pixel_mut(dx as u32, dy as u32), src);
}

fn alpha_over(dst: &mut Rgba<u8>, src: Rgba<u8>) {
    let sa = src.0[3] as f32 / 255.0;
    if sa <= 0.0 { return; }
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
        Background::Notebook | Background::Graph | Background::WarmWhite => Rgba([252, 249, 243, 255]),
        Background::Cream => Rgba([249, 242, 218, 255]),
        Background::Chalkboard => Rgba([37, 53, 48, 255]),
        Background::SoftGradient => Rgba([246, 249, 255, 255]),
        Background::Blueprint => Rgba([20, 55, 92, 255]),
        Background::Kraft => Rgba([198, 164, 113, 255]),
        Background::Dark => Rgba([28, 30, 34, 255]),
        Background::Linen => Rgba([236, 231, 219, 255]),
    };
    let mut img = RgbaImage::from_pixel(w, h, base);
    match project.canvas.background {
        Background::Notebook => horizontal_raster_lines(&mut img, Rgba([205, 218, 235, 255]), (h / 36).max(12)),
        Background::Graph => raster_grid(&mut img, Rgba([220, 222, 226, 255]), (w / 32).max(12)),
        Background::Blueprint => raster_grid(&mut img, Rgba([70, 115, 155, 255]), (w / 28).max(12)),
        Background::Chalkboard => raster_grid(&mut img, Rgba([60, 78, 70, 255]), (w / 22).max(14)),
        Background::Linen => raster_grid(&mut img, Rgba([220, 214, 202, 255]), (w / 70).max(8)),
        Background::SoftGradient => {
            for y in 0..h {
                let t = y as f32 / h.max(1) as f32;
                let c = Rgba([(250.0 - 14.0 * t) as u8, (252.0 - 9.0 * t) as u8, 255, 255]);
                for x in 0..w { img.put_pixel(x, y, c); }
            }
        }
        _ => {}
    }
    img
}

fn horizontal_raster_lines(img: &mut RgbaImage, color: Rgba<u8>, step: u32) {
    let mut y = step;
    while y < img.height() {
        for x in 0..img.width() { img.put_pixel(x, y, color); }
        y += step;
    }
}

fn raster_grid(img: &mut RgbaImage, color: Rgba<u8>, step: u32) {
    let mut x = 0;
    while x < img.width() {
        for y in 0..img.height() { img.put_pixel(x, y, color); }
        x += step;
    }
    let mut y = 0;
    while y < img.height() {
        for x in 0..img.width() { img.put_pixel(x, y, color); }
        y += step;
    }
}
