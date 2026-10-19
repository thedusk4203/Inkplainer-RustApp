use eframe::egui::{self, Align2, Color32, FontFamily, FontId, Pos2, Rect, Stroke, StrokeKind, Vec2};

use crate::animation::{layer_progress, reveal_regions};
use crate::assets::{RuntimeAssets, RuntimeImage};
use crate::model::{AnimationStyle, Background, Layer, LayerContent, Project, TextAlign};

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
    if size.x / size.y > aspect {
        size.x = size.y * aspect;
    } else {
        size.y = size.x / aspect;
    }
    size.x = size.x.max(64.0);
    size.y = size.y.max(64.0);

    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    paint_background(&painter, rect, &project.canvas.background, project.canvas.width, project.canvas.height);

    for (idx, layer) in project.layers.iter().enumerate() {
        if !layer.visible {
            continue;
        }
        let p = layer_progress(project, idx, playhead);
        paint_layer(&painter, rect, project, layer, assets, p, idx == selected_layer.unwrap_or(usize::MAX));
    }

    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::from_gray(110)), StrokeKind::Inside);

    let selected_drag_delta_canvas = if response.dragged() {
        selected_layer.map(|_| {
            let pointer_delta = ui.input(|i| i.pointer.delta());
            let sx = project.canvas.width as f32 / rect.width().max(1.0);
            let sy = project.canvas.height as f32 / rect.height().max(1.0);
            Vec2::new(pointer_delta.x * sx, pointer_delta.y * sy)
        })
    } else {
        None
    };

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
                    AnimationStyle::OutlineFill | AnimationStyle::IllustFill => {
                        paint_outline_then_reveal(painter, layer_rect, runtime, layer, progress, tint, true)
                    }
                    AnimationStyle::OutlineOnly => {
                        paint_outline_then_reveal(painter, layer_rect, runtime, layer, progress, tint, false)
                    }
                    _ => paint_reveal(painter, layer_rect, runtime, layer, progress, tint),
                }
            } else {
                painter.rect_filled(layer_rect, 0.0, Color32::from_rgb(80, 30, 30));
                painter.text(
                    layer_rect.center(),
                    Align2::CENTER_CENTER,
                    "Missing image",
                    FontId::proportional(16.0),
                    Color32::WHITE,
                );
            }
        }
        LayerContent::Text(text) => {
            let alpha = if matches!(layer.animation.style, AnimationStyle::TextDraw) { progress } else { progress.min(1.0) };
            if alpha > 0.0 {
                let color = Color32::from_rgba_unmultiplied(
                    text.color[0],
                    text.color[1],
                    text.color[2],
                    ((text.color[3] as f32) * layer.opacity * alpha).clamp(0.0, 255.0) as u8,
                );
                let align = match text.align {
                    TextAlign::Left => Align2::LEFT_CENTER,
                    TextAlign::Center => Align2::CENTER_CENTER,
                    TextAlign::Right => Align2::RIGHT_CENTER,
                };
                let anchor = match text.align {
                    TextAlign::Left => Pos2::new(layer_rect.left(), layer_rect.center().y),
                    TextAlign::Center => layer_rect.center(),
                    TextAlign::Right => Pos2::new(layer_rect.right(), layer_rect.center().y),
                };
                let scale = canvas_rect.width() / project.canvas.width.max(1) as f32;
                painter.text(
                    anchor,
                    align,
                    &text.text,
                    FontId::new((text.size * scale).max(8.0), FontFamily::Proportional),
                    color,
                );
            }
        }
    }

    if selected {
        painter.rect_stroke(
            layer_rect,
            0.0,
            Stroke::new(1.5, Color32::from_rgb(56, 132, 255)),
            StrokeKind::Outside,
        );
    }
}

fn paint_reveal(
    painter: &egui::Painter,
    rect: Rect,
    runtime: &RuntimeImage,
    layer: &Layer,
    progress: f32,
    tint: Color32,
) {
    for region in reveal_regions(layer, progress) {
        let dst = Rect::from_min_max(
            Pos2::new(rect.left() + rect.width() * region.u0, rect.top() + rect.height() * region.v0),
            Pos2::new(rect.left() + rect.width() * region.u1, rect.top() + rect.height() * region.v1),
        );
        let uv = Rect::from_min_max(Pos2::new(region.u0, region.v0), Pos2::new(region.u1, region.v1));
        painter.image(runtime.texture.id(), dst, uv, tint);
    }
}

fn paint_contour(
    painter: &egui::Painter,
    rect: Rect,
    runtime: &RuntimeImage,
    layer: &Layer,
    progress: f32,
    tint: Color32,
) {
    if progress < 0.35 {
        paint_edge_path(painter, rect, runtime, layer, progress / 0.35);
    } else {
        paint_edge_path(painter, rect, runtime, layer, 1.0);
        let mut reveal_layer = layer.clone();
        reveal_layer.animation.style = AnimationStyle::Scanner;
        paint_reveal(painter, rect, runtime, &reveal_layer, (progress - 0.35) / 0.65, tint);
    }
}

fn paint_outline_then_reveal(
    painter: &egui::Painter,
    rect: Rect,
    runtime: &RuntimeImage,
    layer: &Layer,
    progress: f32,
    tint: Color32,
    reveal_raw: bool,
) {
    let outline_phase = 0.55;
    if progress <= outline_phase {
        paint_edge_path(painter, rect, runtime, layer, progress / outline_phase);
        return;
    }
    paint_edge_path(painter, rect, runtime, layer, 1.0);
    if reveal_raw && layer.animation.reveal_original {
        let fade = ((progress - outline_phase) / (1.0 - outline_phase)).clamp(0.0, 1.0);
        let alpha = ((tint.a() as f32) * fade) as u8;
        painter.image(
            runtime.texture.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::from_white_alpha(alpha),
        );
    }
}

fn paint_edge_path(
    painter: &egui::Painter,
    rect: Rect,
    runtime: &RuntimeImage,
    layer: &Layer,
    progress: f32,
) {
    let pts = &runtime.edge_path;
    if pts.len() < 2 {
        return;
    }
    let count = ((pts.len() as f32 * progress.clamp(0.0, 1.0)).ceil() as usize).clamp(2, pts.len());
    let color = Color32::from_rgba_unmultiplied(
        layer.animation.outline_color[0],
        layer.animation.outline_color[1],
        layer.animation.outline_color[2],
        ((layer.animation.outline_color[3] as f32) * layer.opacity).clamp(0.0, 255.0) as u8,
    );
    let stroke = Stroke::new(layer.animation.outline_thickness.max(0.5), color);
    for pair in pts[..count].windows(2) {
        let a = Pos2::new(rect.left() + pair[0][0] * rect.width(), rect.top() + pair[0][1] * rect.height());
        let b = Pos2::new(rect.left() + pair[1][0] * rect.width(), rect.top() + pair[1][1] * rect.height());
        if a.distance(b) < rect.width().max(rect.height()) * 0.12 {
            painter.line_segment([a, b], stroke);
        }
    }
}

fn to_screen_rect(canvas_rect: Rect, project: &Project, layer: &Layer) -> Rect {
    let sx = canvas_rect.width() / project.canvas.width.max(1) as f32;
    let sy = canvas_rect.height() / project.canvas.height.max(1) as f32;
    Rect::from_min_size(
        Pos2::new(
            canvas_rect.left() + layer.transform.x * sx,
            canvas_rect.top() + layer.transform.y * sy,
        ),
        Vec2::new(layer.transform.w * sx, layer.transform.h * sy),
    )
}

fn paint_background(
    painter: &egui::Painter,
    rect: Rect,
    bg: &Background,
    canvas_w: u32,
    canvas_h: u32,
) {
    match bg {
        Background::Solid(c) => { painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])); }
        Background::Transparent => { painter.rect_filled(rect, 0.0, Color32::from_gray(210)); }
        Background::Notebook => {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(252, 251, 246));
            let step = (rect.height() / 36.0).max(12.0);
            let mut y = rect.top() + step;
            while y < rect.bottom() {
                painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], Stroke::new(0.6, Color32::from_rgb(205, 218, 235)));
                y += step;
            }
        }
        Background::Graph => {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(248, 250, 252));
            let step = (rect.width() / 32.0).max(12.0);
            let mut x = rect.left();
            while x < rect.right() {
                painter.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())], Stroke::new(0.5, Color32::from_gray(220)));
                x += step;
            }
            let mut y = rect.top();
            while y < rect.bottom() {
                painter.line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], Stroke::new(0.5, Color32::from_gray(220)));
                y += step;
            }
        }
        Background::Blueprint => { painter.rect_filled(rect, 0.0, Color32::from_rgb(20, 55, 92)); }
        Background::Kraft => { painter.rect_filled(rect, 0.0, Color32::from_rgb(198, 164, 113)); }
        Background::Dark => { painter.rect_filled(rect, 0.0, Color32::from_rgb(28, 30, 34)); }
    }

    let _ = (canvas_w, canvas_h);
}
