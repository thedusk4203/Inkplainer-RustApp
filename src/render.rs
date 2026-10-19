use eframe::egui::{self, Align2, Color32, FontFamily, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

use crate::animation::{layer_progress, reveal_regions, RevealRegion};
use crate::assets::{RuntimeAssets, RuntimeImage};
use crate::effects::select_edge_path;
use crate::model::{
    AnimationStyle, Background, ColoringStyle, HandStyle, Layer, LayerContent, Project,
    RevealStyle, StrokeStyle, TextAlign, TextDirection, TextDrawStyle,
};

pub struct CanvasResult {
    pub rect: Rect,
    pub selected_drag_delta_canvas: Option<Vec2>,
}

pub fn paint_project(
    ui: &mut egui::Ui,
    project: &Project,
    assets: &RuntimeAssets,
    selected_layer: Option<usize>,
    playhead: f32,
) -> CanvasResult {
    let avail = ui.available_size();
    let aspect = project.canvas.width.max(1) as f32 / project.canvas.height.max(1) as f32;
    let mut size = avail;
    if size.y <= 1.0 { size.y = 64.0; }
    if size.x / size.y > aspect { size.x = size.y * aspect; } else { size.y = size.x / aspect; }
    size.x = size.x.max(64.0);
    size.y = size.y.max(64.0);

    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    paint_background(&painter, rect, &project.canvas.background);

    for (idx, layer) in project.layers.iter().enumerate() {
        if !layer.visible { continue; }
        let p = layer_progress(project, idx, playhead);
        paint_layer(&painter, rect, project, layer, assets, p, idx == selected_layer.unwrap_or(usize::MAX));
    }
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, Color32::from_gray(110)), StrokeKind::Inside);

    let selected_drag_delta_canvas = if response.dragged() {
        selected_layer.map(|_| {
            let pointer_delta = ui.input(|i| i.pointer.delta());
            let sx = project.canvas.width as f32 / rect.width().max(1.0);
            let sy = project.canvas.height as f32 / rect.height().max(1.0);
            Vec2::new(pointer_delta.x * sx, pointer_delta.y * sy)
        })
    } else { None };

    CanvasResult { rect, selected_drag_delta_canvas }
}

fn paint_layer(
    painter: &egui::Painter,
    canvas_rect: Rect,
    project: &Project,
    layer: &Layer,
    assets: &RuntimeAssets,
    progress: f32,
    selected: bool,
) {
    let layer_rect = to_screen_rect(canvas_rect, project, layer);
    let tint = Color32::from_white_alpha((layer.opacity.clamp(0.0, 1.0) * 255.0) as u8);
    match &layer.content {
        LayerContent::Image { .. } => {
            if let Some(runtime) = assets.images.get(&layer.id) {
                match layer.animation.style {
                    AnimationStyle::Contour => paint_contour(painter, layer_rect, runtime, layer, progress, tint),
                    AnimationStyle::OutlineFill => paint_outline_then_reveal(painter, layer_rect, runtime, layer, progress, tint, false),
                    AnimationStyle::IllustFill => paint_outline_then_reveal(painter, layer_rect, runtime, layer, progress, tint, true),
                    AnimationStyle::OutlineOnly => paint_outline_only(painter, layer_rect, runtime, layer, progress),
                    _ => paint_reveal(painter, layer_rect, runtime, layer, progress, tint),
                }
                paint_hand(painter, layer_rect, runtime, layer, progress);
            } else {
                painter.rect_filled(layer_rect, 0.0, Color32::from_rgb(80, 30, 30));
                painter.text(layer_rect.center(), Align2::CENTER_CENTER, "Missing image", FontId::proportional(16.0), Color32::WHITE);
            }
        }
        LayerContent::Text(text) => paint_text_layer(painter, canvas_rect, layer_rect, project, layer, text, progress),
    }
    if selected {
        painter.rect_stroke(layer_rect, 0.0, Stroke::new(1.5_f32, Color32::from_rgb(56, 132, 255)), StrokeKind::Outside);
    }
}

fn paint_text_layer(
    painter: &egui::Painter,
    canvas_rect: Rect,
    layer_rect: Rect,
    project: &Project,
    layer: &Layer,
    text: &crate::model::TextLayer,
    progress: f32,
) {
    if progress <= 0.0 { return; }
    let color = Color32::from_rgba_unmultiplied(
        text.color[0], text.color[1], text.color[2],
        ((text.color[3] as f32) * layer.opacity).clamp(0.0, 255.0) as u8,
    );
    let align = match text.align { TextAlign::Left => Align2::LEFT_CENTER, TextAlign::Center => Align2::CENTER_CENTER, TextAlign::Right => Align2::RIGHT_CENTER };
    let anchor = match text.align {
        TextAlign::Left => Pos2::new(layer_rect.left(), layer_rect.center().y),
        TextAlign::Center => layer_rect.center(),
        TextAlign::Right => Pos2::new(layer_rect.right(), layer_rect.center().y),
    };
    let scale = canvas_rect.width() / project.canvas.width.max(1) as f32;
    let font = FontId::new((text.size * scale).max(8.0), FontFamily::Proportional);

    if layer.animation.style == AnimationStyle::TextDraw {
        let clip = directional_clip(layer_rect, progress, layer.animation.text_direction);
        let clipped = painter.with_clip_rect(clip);
        match layer.animation.text_draw_style {
            TextDrawStyle::Reveal | TextDrawStyle::OutlineFill => {
                clipped.text(anchor, align, &text.text, font.clone(), color);
            }
            TextDrawStyle::Outline => {
                let outline = Color32::from_rgba_unmultiplied(
                    layer.animation.outline_color[0], layer.animation.outline_color[1],
                    layer.animation.outline_color[2], layer.animation.outline_color[3],
                );
                for d in [Vec2::new(-1.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(0.0, -1.0), Vec2::new(0.0, 1.0)] {
                    clipped.text(anchor + d, align, &text.text, font.clone(), outline);
                }
            }
        }
    } else {
        painter.text(anchor, align, &text.text, font, color);
    }
}

fn directional_clip(rect: Rect, progress: f32, dir: TextDirection) -> Rect {
    let p = progress.clamp(0.0, 1.0);
    match dir {
        TextDirection::LeftToRight => Rect::from_min_max(rect.min, Pos2::new(rect.left() + rect.width() * p, rect.bottom())),
        TextDirection::RightToLeft => Rect::from_min_max(Pos2::new(rect.right() - rect.width() * p, rect.top()), rect.max),
        TextDirection::TopToBottom => Rect::from_min_max(rect.min, Pos2::new(rect.right(), rect.top() + rect.height() * p)),
        TextDirection::BottomToTop => Rect::from_min_max(Pos2::new(rect.left(), rect.bottom() - rect.height() * p), rect.max),
    }
}

fn paint_reveal(painter: &egui::Painter, rect: Rect, runtime: &RuntimeImage, layer: &Layer, progress: f32, tint: Color32) {
    for region in reveal_regions(layer, progress) { paint_image_region(painter, rect, runtime, region, tint); }
}

fn paint_image_region(painter: &egui::Painter, rect: Rect, runtime: &RuntimeImage, region: RevealRegion, tint: Color32) {
    let dst = Rect::from_min_max(
        Pos2::new(rect.left() + rect.width() * region.u0, rect.top() + rect.height() * region.v0),
        Pos2::new(rect.left() + rect.width() * region.u1, rect.top() + rect.height() * region.v1),
    );
    let uv = Rect::from_min_max(Pos2::new(region.u0, region.v0), Pos2::new(region.u1, region.v1));
    painter.image(runtime.texture.id(), dst, uv, tint);
}

fn paint_contour(painter: &egui::Painter, rect: Rect, runtime: &RuntimeImage, layer: &Layer, progress: f32, tint: Color32) {
    if progress < 0.35 {
        paint_edge_path(painter, rect, runtime, layer, progress / 0.35);
    } else {
        paint_edge_path(painter, rect, runtime, layer, 1.0);
        let mut reveal_layer = layer.clone();
        reveal_layer.animation.style = AnimationStyle::Scanner;
        paint_reveal(painter, rect, runtime, &reveal_layer, (progress - 0.35) / 0.65, tint);
    }
}

fn paint_outline_only(painter: &egui::Painter, rect: Rect, runtime: &RuntimeImage, layer: &Layer, progress: f32) {
    if layer.animation.outline_visible { paint_edge_path(painter, rect, runtime, layer, progress); }
}

fn paint_outline_then_reveal(
    painter: &egui::Painter,
    rect: Rect,
    runtime: &RuntimeImage,
    layer: &Layer,
    progress: f32,
    tint: Color32,
    illustrated: bool,
) {
    let outline_phase = if illustrated { 0.45 } else { 0.55 };
    if progress <= outline_phase {
        if layer.animation.outline_visible { paint_edge_path(painter, rect, runtime, layer, progress / outline_phase); }
        return;
    }
    if layer.animation.outline_visible { paint_edge_path(painter, rect, runtime, layer, 1.0); }
    let fill_p = ((progress - outline_phase) / (1.0 - outline_phase)).clamp(0.0, 1.0);
    if illustrated {
        match layer.animation.coloring_style {
            ColoringStyle::Sparse => {
                let mut sparse = layer.clone();
                sparse.animation.style = AnimationStyle::ChunkJump;
                sparse.animation.chunks = 80;
                for r in reveal_regions(&sparse, fill_p * 0.82) {
                    paint_image_region(painter, rect, runtime, r, Color32::from_white_alpha((tint.a() as f32 * 0.62) as u8));
                }
            }
            ColoringStyle::Filled => paint_reveal_style(painter, rect, runtime, layer, fill_p, tint),
            ColoringStyle::Watercolor => {
                for pass in 0..3 {
                    let pp = (fill_p + pass as f32 * 0.06).clamp(0.0, 1.0);
                    paint_reveal_style(painter, rect, runtime, layer, pp, Color32::from_white_alpha((tint.a() as f32 * 0.24) as u8));
                }
            }
        }
    } else if layer.animation.reveal_original {
        paint_reveal_style(painter, rect, runtime, layer, fill_p, tint);
    }
}

fn paint_reveal_style(painter: &egui::Painter, rect: Rect, runtime: &RuntimeImage, layer: &Layer, progress: f32, tint: Color32) {
    let p = progress.clamp(0.0, 1.0);
    match layer.animation.reveal_style {
        RevealStyle::Instant => {
            if p > 0.0 { painter.image(runtime.texture.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), tint); }
        }
        RevealStyle::Fade => {
            let alpha = (tint.a() as f32 * smoothstep(p)).round() as u8;
            painter.image(runtime.texture.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::from_white_alpha(alpha));
        }
        RevealStyle::WipeRight => paint_image_region(painter, rect, runtime, RevealRegion { u0: 0.0, v0: 0.0, u1: p, v1: 1.0 }, tint),
        RevealStyle::Iris => {
            let r = p * 0.72;
            paint_image_region(painter, rect, runtime, RevealRegion { u0: (0.5-r).max(0.0), v0: (0.5-r).max(0.0), u1: (0.5+r).min(1.0), v1: (0.5+r).min(1.0) }, tint);
        }
        RevealStyle::ScanLines => {
            for s in 0..18 {
                let offset = s as f32 / 18.0 * 0.35;
                let local = ((p - offset) / 0.65).clamp(0.0, 1.0);
                if local <= 0.0 { continue; }
                let v0 = s as f32 / 18.0;
                let v1 = (s + 1) as f32 / 18.0;
                let (u0, u1) = if s % 2 == 0 { (0.0, local) } else { (1.0-local, 1.0) };
                paint_image_region(painter, rect, runtime, RevealRegion { u0, v0, u1, v1 }, tint);
            }
        }
        RevealStyle::Dissolve => {
            let mut dissolve = layer.clone();
            dissolve.animation.style = AnimationStyle::ChunkJump;
            dissolve.animation.chunks = 80;
            for r in reveal_regions(&dissolve, p) { paint_image_region(painter, rect, runtime, r, tint); }
        }
    }
}

fn smoothstep(t: f32) -> f32 { t * t * (3.0 - 2.0 * t) }

fn paint_edge_path(painter: &egui::Painter, rect: Rect, runtime: &RuntimeImage, layer: &Layer, progress: f32) {
    let pts = select_edge_path(&runtime.edges, &layer.animation);
    if pts.len() < 2 { return; }
    let sensitivity = layer.animation.outline_detection.clamp(1, 100) as f32 / 100.0;
    let effective_len = (((pts.len() as f32) * (0.30 + sensitivity * 0.70)).round() as usize).clamp(2, pts.len());
    let count = ((effective_len as f32 * progress.clamp(0.0, 1.0)).ceil() as usize).clamp(2, effective_len);
    let base = Color32::from_rgba_unmultiplied(
        layer.animation.outline_color[0], layer.animation.outline_color[1], layer.animation.outline_color[2],
        ((layer.animation.outline_color[3] as f32) * layer.opacity * layer.animation.outline_opacity).clamp(0.0, 255.0) as u8,
    );
    let scale = rect.width().max(rect.height()) / 1000.0;
    let thickness = (layer.animation.outline_thickness * scale.max(0.55)).max(0.5);
    let max_jump = rect.width().max(rect.height()) * 0.09;
    for (i, pair) in pts[..count].windows(2).enumerate() {
        let a = Pos2::new(rect.left() + pair[0][0] * rect.width(), rect.top() + pair[0][1] * rect.height());
        let b = Pos2::new(rect.left() + pair[1][0] * rect.width(), rect.top() + pair[1][1] * rect.height());
        if a.distance(b) > max_jump { continue; }
        match layer.animation.stroke_style {
            StrokeStyle::Default => { painter.line_segment([a, b], Stroke::new(thickness, base)); }
            StrokeStyle::Charcoal => {
                painter.line_segment([a, b], Stroke::new(thickness * 2.1, Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), base.a() / 3)));
                painter.line_segment([a, b], Stroke::new(thickness * 0.9, base));
            }
            StrokeStyle::Sketch => {
                for k in 0..4 {
                    let off = (k as f32 - 1.5) * 0.55;
                    painter.line_segment([a + Vec2::new(off, -off * 0.4), b + Vec2::new(off, -off * 0.4)], Stroke::new(thickness * 0.65, Color32::from_rgba_unmultiplied(base.r(), base.g(), base.b(), base.a() / 2)));
                }
            }
            StrokeStyle::Fountain => {
                let wave = 0.65 + ((i as f32 * 0.17).sin().abs() * 1.25);
                painter.line_segment([a, b], Stroke::new(thickness * wave, base));
            }
            StrokeStyle::Blueprint => {
                let c = Color32::from_rgba_unmultiplied(118, 210, 255, base.a());
                painter.line_segment([a, b], Stroke::new(thickness * 0.75, c));
            }
        }
    }
}

fn paint_hand(painter: &egui::Painter, rect: Rect, runtime: &RuntimeImage, layer: &Layer, progress: f32) {
    if layer.animation.hand == HandStyle::Ghost || progress <= 0.0 || progress >= 1.0 { return; }
    let tip = hand_tip(rect, runtime, layer, progress);
    let s = (rect.width().min(rect.height()) * 0.035).clamp(8.0, 34.0);
    match layer.animation.hand {
        HandStyle::Pen => {
            painter.line_segment([tip + Vec2::new(s * 1.8, s * 1.6), tip], Stroke::new((s * 0.28).max(2.0), Color32::from_rgb(45, 45, 50)));
            painter.circle_filled(tip + Vec2::new(s * 1.8, s * 1.6), s * 0.24, Color32::from_rgb(210, 210, 215));
        }
        HandStyle::PixelArt => {
            painter.rect_filled(Rect::from_min_size(tip + Vec2::new(-s * 0.2, s * 0.4), Vec2::new(s * 1.6, s * 1.25)), 0.0, Color32::from_rgb(228, 183, 145));
            painter.line_segment([tip, tip + Vec2::new(s * 0.9, s * 0.95)], Stroke::new(s * 0.28, Color32::from_rgb(48, 48, 52)));
        }
        _ => {
            painter.circle_filled(tip + Vec2::new(s * 0.85, s * 1.0), s * 0.68, Color32::from_rgb(231, 190, 158));
            painter.line_segment([tip, tip + Vec2::new(s * 1.45, s * 1.65)], Stroke::new(s * 0.24, Color32::from_rgb(46, 46, 50)));
            painter.circle_filled(tip + Vec2::new(s * 0.55, s * 0.65), s * 0.25, Color32::from_rgb(238, 201, 171));
        }
    }
}

fn hand_tip(rect: Rect, runtime: &RuntimeImage, layer: &Layer, progress: f32) -> Pos2 {
    if matches!(layer.animation.style, AnimationStyle::Contour | AnimationStyle::OutlineFill | AnimationStyle::IllustFill | AnimationStyle::OutlineOnly) {
        let pts = select_edge_path(&runtime.edges, &layer.animation);
        if !pts.is_empty() {
            let i = ((pts.len() - 1) as f32 * progress.clamp(0.0, 1.0)).round() as usize;
            return Pos2::new(rect.left() + pts[i][0] * rect.width(), rect.top() + pts[i][1] * rect.height());
        }
    }
    if let Some(r) = reveal_regions(layer, progress).last() {
        return Pos2::new(rect.left() + (r.u0 + r.u1) * 0.5 * rect.width(), rect.top() + (r.v0 + r.v1) * 0.5 * rect.height());
    }
    rect.center()
}

fn to_screen_rect(canvas_rect: Rect, project: &Project, layer: &Layer) -> Rect {
    let sx = canvas_rect.width() / project.canvas.width.max(1) as f32;
    let sy = canvas_rect.height() / project.canvas.height.max(1) as f32;
    Rect::from_min_size(
        Pos2::new(canvas_rect.left() + layer.transform.x * sx, canvas_rect.top() + layer.transform.y * sy),
        Vec2::new(layer.transform.w * sx, layer.transform.h * sy),
    )
}

fn paint_background(painter: &egui::Painter, rect: Rect, bg: &Background) {
    match bg {
        Background::Solid(c) => { painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])); }
        Background::Transparent => checker(painter, rect),
        Background::Notebook => { painter.rect_filled(rect, 0.0, Color32::from_rgb(252, 251, 246)); horizontal_lines(painter, rect, Color32::from_rgb(205, 218, 235), 36.0); }
        Background::Graph => { painter.rect_filled(rect, 0.0, Color32::from_rgb(248, 250, 252)); grid(painter, rect, Color32::from_gray(220), 32.0); }
        Background::Cream => { painter.rect_filled(rect, 0.0, Color32::from_rgb(249, 242, 218)); }
        Background::Chalkboard => { painter.rect_filled(rect, 0.0, Color32::from_rgb(37, 53, 48)); grid(painter, rect, Color32::from_rgba_unmultiplied(210, 230, 220, 28), 22.0); }
        Background::SoftGradient => {
            for i in 0..40 {
                let t = i as f32 / 40.0;
                let r = Rect::from_min_max(Pos2::new(rect.left(), rect.top() + rect.height() * t), Pos2::new(rect.right(), rect.top() + rect.height() * (i + 1) as f32 / 40.0));
                painter.rect_filled(r, 0.0, Color32::from_rgb((250.0 - 14.0 * t) as u8, (252.0 - 9.0 * t) as u8, 255));
            }
        }
        Background::WarmWhite => { painter.rect_filled(rect, 0.0, Color32::from_rgb(252, 249, 243)); }
        Background::Blueprint => { painter.rect_filled(rect, 0.0, Color32::from_rgb(20, 55, 92)); grid(painter, rect, Color32::from_rgba_unmultiplied(140, 205, 242, 50), 28.0); }
        Background::Kraft => { painter.rect_filled(rect, 0.0, Color32::from_rgb(198, 164, 113)); }
        Background::Dark => { painter.rect_filled(rect, 0.0, Color32::from_rgb(28, 30, 34)); }
        Background::Linen => { painter.rect_filled(rect, 0.0, Color32::from_rgb(236, 231, 219)); grid(painter, rect, Color32::from_rgba_unmultiplied(135, 125, 110, 20), 12.0); }
    }
}

fn horizontal_lines(painter: &egui::Painter, rect: Rect, color: Color32, divisions: f32) {
    let step = (rect.height() / divisions).max(10.0);
    let mut y = rect.top() + step;
    while y < rect.bottom() {
        painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], Stroke::new(0.6_f32, color));
        y += step;
    }
}

fn grid(painter: &egui::Painter, rect: Rect, color: Color32, divisions: f32) {
    let step = (rect.width() / divisions).max(10.0);
    let mut x = rect.left();
    while x < rect.right() {
        painter.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())], Stroke::new(0.5_f32, color));
        x += step;
    }
    let mut y = rect.top();
    while y < rect.bottom() {
        painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], Stroke::new(0.5_f32, color));
        y += step;
    }
}

fn checker(painter: &egui::Painter, rect: Rect) {
    let step = 14.0;
    let mut row = 0usize;
    let mut y = rect.top();
    while y < rect.bottom() {
        let mut col = 0usize;
        let mut x = rect.left();
        while x < rect.right() {
            let r = Rect::from_min_max(Pos2::new(x, y), Pos2::new((x + step).min(rect.right()), (y + step).min(rect.bottom())));
            painter.rect_filled(r, 0.0, if (row + col) % 2 == 0 { Color32::from_gray(225) } else { Color32::from_gray(245) });
            x += step;
            col += 1;
        }
        y += step;
        row += 1;
    }
}
