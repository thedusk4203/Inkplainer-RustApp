use eframe::egui::{
    self, Align2, Color32, FontId, Mesh, Pos2, Rect, Stroke, StrokeKind, Vec2,
};
use eframe::epaint::Vertex;

use crate::animation::{layer_progress, reveal_regions, RevealRegion};
use crate::assets::{RuntimeAssets, RuntimeImage};
use crate::effects::select_edge_path;
use crate::font_catalog;
use crate::model::{
    AnimationStyle, Background, ColoringStyle, HandStyle, Layer, LayerContent, Project,
    RevealStyle, StrokeStyle, TextAlign, TextDirection, TextDrawStyle,
};
use crate::timing::hand_motion_progress;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy)]
pub struct ResizeDelta {
    pub handle: ResizeHandle,
    pub delta_canvas: Vec2,
}

pub struct CanvasResult {
    pub selected_drag_delta_canvas: Option<Vec2>,
    pub selected_resize_delta: Option<ResizeDelta>,
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
    if size.y <= 1.0 {
        size.y = 64.0;
    }
    if size.x / size.y > aspect {
        size.x = size.y * aspect;
    } else {
        size.y = size.x / aspect;
    }
    size.x = size.x.max(64.0);
    size.y = size.y.max(64.0);

    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    paint_background(&painter, rect, project);

    for (idx, layer) in project.layers.iter().enumerate() {
        if !layer.visible {
            continue;
        }
        let progress = layer_progress(project, idx, playhead);
        paint_layer(
            &painter,
            rect,
            project,
            layer,
            assets,
            progress,
            idx == selected_layer.unwrap_or(usize::MAX),
        );
    }
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0_f32, Color32::from_gray(110)),
        StrokeKind::Inside,
    );

    let sx = project.canvas.width as f32 / rect.width().max(1.0);
    let sy = project.canvas.height as f32 / rect.height().max(1.0);
    let mut selected_resize_delta = None;

    if let Some(idx) = selected_layer {
        if let Some(layer) = project.layers.get(idx) {
            let layer_rect = to_screen_rect(rect, project, layer);
            let handles = [
                (ResizeHandle::TopLeft, layer_rect.left_top()),
                (ResizeHandle::TopRight, layer_rect.right_top()),
                (ResizeHandle::BottomLeft, layer_rect.left_bottom()),
                (ResizeHandle::BottomRight, layer_rect.right_bottom()),
            ];
            for (slot, (handle, center)) in handles.into_iter().enumerate() {
                let handle_rect = Rect::from_center_size(center, Vec2::splat(12.0));
                let id = ui.id().with(("layer_resize", idx, slot));
                let interaction = ui.interact(handle_rect, id, egui::Sense::drag());
                let fill = if interaction.hovered() || interaction.dragged() {
                    Color32::WHITE
                } else {
                    Color32::from_rgb(56, 132, 255)
                };
                painter.rect_filled(handle_rect, 2.0, fill);
                painter.rect_stroke(
                    handle_rect,
                    2.0,
                    Stroke::new(1.5_f32, Color32::from_rgb(25, 82, 180)),
                    StrokeKind::Inside,
                );
                if interaction.dragged() {
                    let delta = ui.input(|input| input.pointer.delta());
                    selected_resize_delta = Some(ResizeDelta {
                        handle,
                        delta_canvas: Vec2::new(delta.x * sx, delta.y * sy),
                    });
                }
            }
        }
    }

    let selected_drag_delta_canvas = if selected_resize_delta.is_none() && response.dragged() {
        selected_layer.map(|_| {
            let pointer_delta = ui.input(|input| input.pointer.delta());
            Vec2::new(pointer_delta.x * sx, pointer_delta.y * sy)
        })
    } else {
        None
    };

    CanvasResult {
        selected_drag_delta_canvas,
        selected_resize_delta,
    }
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
                    AnimationStyle::Contour => {
                        paint_contour(painter, layer_rect, runtime, layer, progress, tint)
                    }
                    AnimationStyle::OutlineFill => paint_outline_then_reveal(
                        painter,
                        layer_rect,
                        runtime,
                        layer,
                        progress,
                        tint,
                        false,
                    ),
                    AnimationStyle::IllustFill => paint_outline_then_reveal(
                        painter,
                        layer_rect,
                        runtime,
                        layer,
                        progress,
                        tint,
                        true,
                    ),
                    AnimationStyle::OutlineOnly => {
                        paint_outline_only(painter, layer_rect, runtime, layer, progress)
                    }
                    _ => paint_reveal(painter, layer_rect, runtime, layer, progress, tint),
                }
                paint_hand(
                    painter,
                    canvas_rect,
                    layer_rect,
                    project,
                    assets,
                    runtime,
                    layer,
                    progress,
                    project.hand_glide_enabled,
                );
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
        LayerContent::Text(text) => paint_text_layer(
            painter,
            canvas_rect,
            layer_rect,
            project,
            layer,
            text,
            progress,
        ),
    }
    if selected {
        painter.rect_stroke(
            layer_rect,
            0.0,
            Stroke::new(1.5_f32, Color32::from_rgb(56, 132, 255)),
            StrokeKind::Outside,
        );
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
    if progress <= 0.0 {
        return;
    }

    let color = Color32::from_rgba_unmultiplied(
        text.color[0],
        text.color[1],
        text.color[2],
        ((text.color[3] as f32) * layer.opacity).clamp(0.0, 255.0) as u8,
    );
    let scale = canvas_rect.width() / project.canvas.width.max(1) as f32;
    let font = FontId::new(
        (text.size.max(8.0) * scale).max(1.0),
        font_catalog::egui_family(&text.font_family, text.bold, text.italic),
    );

    if layer.animation.style == AnimationStyle::TextDraw {
        let clipped = painter.with_clip_rect(directional_clip(
            layer_rect,
            progress,
            layer.animation.text_direction,
        ));
        match layer.animation.text_draw_style {
            TextDrawStyle::Reveal | TextDrawStyle::OutlineFill => {
                paint_text_layout(&clipped, layer_rect, text, &font, color, scale, Vec2::ZERO);
            }
            TextDrawStyle::Outline => {
                let outline = Color32::from_rgba_unmultiplied(
                    layer.animation.outline_color[0],
                    layer.animation.outline_color[1],
                    layer.animation.outline_color[2],
                    layer.animation.outline_color[3],
                );
                let outline_step = scale.max(0.1);
                for offset in [
                    Vec2::new(-outline_step, 0.0),
                    Vec2::new(outline_step, 0.0),
                    Vec2::new(0.0, -outline_step),
                    Vec2::new(0.0, outline_step),
                ] {
                    paint_text_layout(
                        &clipped,
                        layer_rect,
                        text,
                        &font,
                        outline,
                        scale,
                        offset,
                    );
                }
            }
        }
    } else {
        paint_text_layout(painter, layer_rect, text, &font, color, scale, Vec2::ZERO);
    }
}

fn paint_text_layout(
    painter: &egui::Painter,
    layer_rect: Rect,
    text: &crate::model::TextLayer,
    font: &FontId,
    color: Color32,
    scale: f32,
    offset: Vec2,
) {
    let lines: Vec<&str> = text.text.lines().collect();
    let line_height = ((text.size * text.line_height.max(0.8)).round() * scale).max(1.0);
    let total_height = line_height * lines.len().max(1) as f32;
    let mut y = layer_rect.top() + ((layer_rect.height() - total_height) * 0.5).max(0.0);
    let letter_spacing = text.letter_spacing * scale;

    for line in lines {
        let line_galley = painter.layout_no_wrap(line.to_owned(), font.clone(), color);
        let base_width = line_galley.size().x;
        let x = match text.align {
            TextAlign::Left => layer_rect.left(),
            TextAlign::Center => {
                layer_rect.left() + ((layer_rect.width() - base_width) * 0.5).max(0.0)
            }
            TextAlign::Right => (layer_rect.right() - base_width).max(layer_rect.left()),
        };

        if letter_spacing.abs() < 0.01 {
            painter.galley(Pos2::new(x, y) + offset, line_galley, color);
        } else {
            let mut cursor_x = x;
            for ch in line.chars() {
                let galley = painter.layout_no_wrap(ch.to_string(), font.clone(), color);
                let advance = galley.size().x + letter_spacing;
                painter.galley(Pos2::new(cursor_x, y) + offset, galley, color);
                cursor_x += advance;
            }
        }
        y += line_height;
    }
}

fn directional_clip(rect: Rect, progress: f32, dir: TextDirection) -> Rect {
    let progress = progress.clamp(0.0, 1.0);
    match dir {
        TextDirection::LeftToRight => Rect::from_min_max(
            rect.min,
            Pos2::new(rect.left() + rect.width() * progress, rect.bottom()),
        ),
        TextDirection::RightToLeft => Rect::from_min_max(
            Pos2::new(rect.right() - rect.width() * progress, rect.top()),
            rect.max,
        ),
        TextDirection::TopToBottom => Rect::from_min_max(
            rect.min,
            Pos2::new(rect.right(), rect.top() + rect.height() * progress),
        ),
        TextDirection::BottomToTop => Rect::from_min_max(
            Pos2::new(rect.left(), rect.bottom() - rect.height() * progress),
            rect.max,
        ),
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
        paint_image_region(painter, rect, runtime, region, tint);
    }
}

fn paint_image_region(
    painter: &egui::Painter,
    rect: Rect,
    runtime: &RuntimeImage,
    region: RevealRegion,
    tint: Color32,
) {
    let dst = Rect::from_min_max(
        Pos2::new(
            rect.left() + rect.width() * region.u0,
            rect.top() + rect.height() * region.v0,
        ),
        Pos2::new(
            rect.left() + rect.width() * region.u1,
            rect.top() + rect.height() * region.v1,
        ),
    );
    let uv = Rect::from_min_max(
        Pos2::new(region.u0, region.v0),
        Pos2::new(region.u1, region.v1),
    );
    painter.image(runtime.texture.id(), dst, uv, tint);
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
        paint_reveal(
            painter,
            rect,
            runtime,
            &reveal_layer,
            (progress - 0.35) / 0.65,
            tint,
        );
    }
}

fn paint_outline_only(
    painter: &egui::Painter,
    rect: Rect,
    runtime: &RuntimeImage,
    layer: &Layer,
    progress: f32,
) {
    if layer.animation.outline_visible {
        paint_edge_path(painter, rect, runtime, layer, progress);
    }
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
        if layer.animation.outline_visible {
            paint_edge_path(painter, rect, runtime, layer, progress / outline_phase);
        }
        return;
    }
    if layer.animation.outline_visible {
        paint_edge_path(painter, rect, runtime, layer, 1.0);
    }
    let fill_progress = ((progress - outline_phase) / (1.0 - outline_phase)).clamp(0.0, 1.0);
    if illustrated {
        match layer.animation.coloring_style {
            ColoringStyle::Sparse => {
                let mut sparse = layer.clone();
                sparse.animation.style = AnimationStyle::ChunkJump;
                sparse.animation.chunks = 80;
                for region in reveal_regions(&sparse, fill_progress * 0.82) {
                    paint_image_region(
                        painter,
                        rect,
                        runtime,
                        region,
                        Color32::from_white_alpha((tint.a() as f32 * 0.62) as u8),
                    );
                }
            }
            ColoringStyle::Filled => {
                paint_reveal_style(painter, rect, runtime, layer, fill_progress, tint)
            }
            ColoringStyle::Watercolor => {
                for pass in 0..3 {
                    let progress = (fill_progress + pass as f32 * 0.06).clamp(0.0, 1.0);
                    paint_reveal_style(
                        painter,
                        rect,
                        runtime,
                        layer,
                        progress,
                        Color32::from_white_alpha((tint.a() as f32 * 0.28) as u8),
                    );
                }
            }
        }
    } else if layer.animation.reveal_original {
        paint_reveal_style(painter, rect, runtime, layer, fill_progress, tint);
    }
}

fn paint_reveal_style(
    painter: &egui::Painter,
    rect: Rect,
    runtime: &RuntimeImage,
    layer: &Layer,
    progress: f32,
    tint: Color32,
) {
    let progress = progress.clamp(0.0, 1.0);
    match layer.animation.reveal_style {
        RevealStyle::Instant => {
            if progress > 0.0 {
                painter.image(
                    runtime.texture.id(),
                    rect,
                    Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    tint,
                );
            }
        }
        RevealStyle::Fade => {
            let alpha = (tint.a() as f32 * smoothstep(progress)).round() as u8;
            painter.image(
                runtime.texture.id(),
                rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::from_white_alpha(alpha),
            );
        }
        RevealStyle::WipeRight => paint_image_region(
            painter,
            rect,
            runtime,
            RevealRegion {
                u0: 0.0,
                v0: 0.0,
                u1: progress,
                v1: 1.0,
            },
            tint,
        ),
        RevealStyle::Iris => paint_iris(painter, rect, runtime, progress, tint),
        RevealStyle::ScanLines => {
            for scanline in 0..18 {
                let offset = scanline as f32 / 18.0 * 0.35;
                let local = ((progress - offset) / 0.65).clamp(0.0, 1.0);
                if local <= 0.0 {
                    continue;
                }
                let v0 = scanline as f32 / 18.0;
                let v1 = (scanline + 1) as f32 / 18.0;
                let (u0, u1) = if scanline % 2 == 0 {
                    (0.0, local)
                } else {
                    (1.0 - local, 1.0)
                };
                paint_image_region(
                    painter,
                    rect,
                    runtime,
                    RevealRegion { u0, v0, u1, v1 },
                    tint,
                );
            }
        }
        RevealStyle::Dissolve => {
            let mut dissolve = layer.clone();
            dissolve.animation.style = AnimationStyle::ChunkJump;
            dissolve.animation.chunks = 80;
            for region in reveal_regions(&dissolve, progress) {
                paint_image_region(painter, rect, runtime, region, tint);
            }
        }
    }
}

fn paint_iris(
    painter: &egui::Painter,
    rect: Rect,
    runtime: &RuntimeImage,
    progress: f32,
    tint: Color32,
) {
    let progress = progress.clamp(0.0, 1.0);
    if progress <= 0.0 {
        return;
    }
    if progress >= 1.0 {
        painter.image(
            runtime.texture.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            tint,
        );
        return;
    }

    const SEGMENTS: usize = 64;
    let center = rect.center();
    let half_w = rect.width() * 0.5;
    let half_h = rect.height() * 0.5;
    let radius = (half_w * half_w + half_h * half_h).sqrt() * progress;
    let mut mesh = Mesh::with_texture(runtime.texture.id());
    mesh.vertices.push(Vertex {
        pos: center,
        uv: Pos2::new(0.5, 0.5),
        color: tint,
    });
    for segment in 0..=SEGMENTS {
        let angle = std::f32::consts::TAU * segment as f32 / SEGMENTS as f32;
        let pos = center + Vec2::new(angle.cos() * radius, angle.sin() * radius);
        mesh.vertices.push(Vertex {
            pos,
            uv: Pos2::new(
                (pos.x - rect.left()) / rect.width().max(1.0),
                (pos.y - rect.top()) / rect.height().max(1.0),
            ),
            color: tint,
        });
        if segment > 0 {
            mesh.add_triangle(0, segment as u32, segment as u32 + 1);
        }
    }
    painter.with_clip_rect(rect).add(egui::Shape::mesh(mesh));
}

fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

fn paint_edge_path(
    painter: &egui::Painter,
    rect: Rect,
    runtime: &RuntimeImage,
    layer: &Layer,
    progress: f32,
) {
    let points = select_edge_path(&runtime.edges, &layer.animation);
    if points.len() < 2 {
        return;
    }
    let sensitivity = layer.animation.outline_detection.clamp(1, 100) as f32 / 100.0;
    let effective_len = (((points.len() as f32) * (0.30 + sensitivity * 0.70)).round()
        as usize)
        .clamp(2, points.len());
    let count = ((effective_len as f32 * progress.clamp(0.0, 1.0)).ceil() as usize)
        .clamp(2, effective_len);
    let base = Color32::from_rgba_unmultiplied(
        layer.animation.outline_color[0],
        layer.animation.outline_color[1],
        layer.animation.outline_color[2],
        ((layer.animation.outline_color[3] as f32)
            * layer.opacity
            * layer.animation.outline_opacity)
            .clamp(0.0, 255.0) as u8,
    );
    let sx = rect.width() / layer.transform.w.max(1.0);
    let sy = rect.height() / layer.transform.h.max(1.0);
    let preview_scale = (sx * sy).sqrt().max(0.001);
    let thickness = (layer.animation.outline_thickness * preview_scale).max(0.5);
    let max_jump = rect.width().max(rect.height()) * 0.09;

    for (index, pair) in points[..count].windows(2).enumerate() {
        let a = Pos2::new(
            rect.left() + pair[0][0] * rect.width(),
            rect.top() + pair[0][1] * rect.height(),
        );
        let b = Pos2::new(
            rect.left() + pair[1][0] * rect.width(),
            rect.top() + pair[1][1] * rect.height(),
        );
        if a.distance(b) > max_jump {
            continue;
        }
        match layer.animation.stroke_style {
            StrokeStyle::Default => {
                painter.line_segment([a, b], Stroke::new(thickness, base));
            }
            StrokeStyle::Charcoal => {
                painter.line_segment(
                    [a, b],
                    Stroke::new(
                        thickness * 2.0,
                        Color32::from_rgba_unmultiplied(
                            base.r(),
                            base.g(),
                            base.b(),
                            base.a() / 2,
                        ),
                    ),
                );
            }
            StrokeStyle::Sketch => {
                for canvas_offset in [-1.5_f32, -0.5, 0.5, 1.5] {
                    let offset = canvas_offset * preview_scale;
                    painter.line_segment(
                        [
                            a + Vec2::new(offset, -offset * 0.4),
                            b + Vec2::new(offset, -offset * 0.4),
                        ],
                        Stroke::new(
                            thickness,
                            Color32::from_rgba_unmultiplied(
                                base.r(),
                                base.g(),
                                base.b(),
                                base.a() / 2,
                            ),
                        ),
                    );
                }
            }
            StrokeStyle::Fountain => {
                let wave = 0.7 + (index as f32 * 0.17).sin().abs();
                painter.line_segment([a, b], Stroke::new(thickness * wave, base));
            }
            StrokeStyle::Blueprint => {
                painter.line_segment(
                    [a, b],
                    Stroke::new(
                        thickness,
                        Color32::from_rgba_unmultiplied(118, 210, 255, base.a()),
                    ),
                );
            }
        }
    }
}

fn paint_hand(
    painter: &egui::Painter,
    canvas_rect: Rect,
    rect: Rect,
    project: &Project,
    assets: &RuntimeAssets,
    runtime: &RuntimeImage,
    layer: &Layer,
    progress: f32,
    smooth_hand: bool,
) {
    if layer.animation.hand == HandStyle::Ghost || progress <= 0.0 || progress >= 1.0 {
        return;
    }
    let tip = if smooth_hand {
        smoothed_hand_tip(rect, runtime, layer, progress)
    } else {
        hand_tip(rect, runtime, layer, progress)
    };

    if let Some(hand) = assets.hand(
        layer.animation.hand,
        project.canvas.width,
        project.canvas.height,
    ) {
        if hand.rgba.width() > 1 || hand.rgba.height() > 1 {
            let sx = canvas_rect.width() / project.canvas.width.max(1) as f32;
            let sy = canvas_rect.height() / project.canvas.height.max(1) as f32;
            let dst = Rect::from_min_size(
                Pos2::new(
                    tip.x - hand.tip_x as f32 * sx,
                    tip.y - hand.tip_y as f32 * sy,
                ),
                Vec2::new(hand.width as f32 * sx, hand.height as f32 * sy),
            );
            painter.image(
                hand.texture.id(),
                dst,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
            return;
        }
    }

    let size = (rect.width().min(rect.height()) * 0.035).clamp(8.0, 34.0);
    match layer.animation.hand {
        HandStyle::Ghost => {}
        HandStyle::Hand1 => {
            painter.circle_filled(
                tip + Vec2::new(size * 0.82, size * 0.96),
                size * 0.70,
                Color32::from_rgb(231, 190, 158),
            );
            painter.circle_filled(
                tip + Vec2::new(size * 0.43, size * 0.54),
                size * 0.24,
                Color32::from_rgb(242, 205, 175),
            );
            painter.line_segment(
                [tip, tip + Vec2::new(size * 1.48, size * 1.68)],
                Stroke::new(size * 0.24, Color32::from_rgb(46, 46, 50)),
            );
        }
        HandStyle::Hand2 => {
            painter.circle_filled(
                tip + Vec2::new(size * 0.72, size * 1.12),
                size * 0.76,
                Color32::from_rgb(176, 118, 84),
            );
            painter.rect_filled(
                Rect::from_center_size(
                    tip + Vec2::new(size * 1.18, size * 1.62),
                    Vec2::new(size * 0.95, size),
                ),
                size * 0.18,
                Color32::from_rgb(72, 91, 131),
            );
            painter.line_segment(
                [tip, tip + Vec2::new(size * 1.35, size * 1.78)],
                Stroke::new(size * 0.25, Color32::from_rgb(30, 31, 35)),
            );
        }
        HandStyle::Hand3 => {
            painter.circle_filled(
                tip + Vec2::new(size * 0.92, size * 0.92),
                size * 0.72,
                Color32::from_rgb(246, 211, 184),
            );
            painter.rect_filled(
                Rect::from_center_size(
                    tip + Vec2::new(size * 1.35, size * 1.54),
                    Vec2::new(size * 1.12, size * 0.82),
                ),
                size * 0.32,
                Color32::from_rgb(205, 218, 232),
            );
            painter.line_segment(
                [tip, tip + Vec2::new(size * 1.62, size * 1.55)],
                Stroke::new(size * 0.22, Color32::from_rgb(62, 50, 44)),
            );
        }
        HandStyle::Pen => {
            painter.line_segment(
                [tip + Vec2::new(size * 1.8, size * 1.6), tip],
                Stroke::new(
                    (size * 0.28).max(2.0),
                    Color32::from_rgb(45, 45, 50),
                ),
            );
            painter.circle_filled(
                tip + Vec2::new(size * 1.8, size * 1.6),
                size * 0.24,
                Color32::from_rgb(210, 210, 215),
            );
        }
        HandStyle::Cartoon => {
            painter.circle_filled(
                tip + Vec2::new(size * 0.9, size * 0.95),
                size * 0.78,
                Color32::from_rgb(255, 220, 157),
            );
            painter.circle_stroke(
                tip + Vec2::new(size * 0.9, size * 0.95),
                size * 0.78,
                Stroke::new((size * 0.12).max(1.5), Color32::from_rgb(55, 49, 45)),
            );
            painter.line_segment(
                [tip, tip + Vec2::new(size * 1.55, size * 1.7)],
                Stroke::new(size * 0.27, Color32::from_rgb(54, 64, 83)),
            );
        }
        HandStyle::PixelArt => {
            painter.rect_filled(
                Rect::from_min_size(
                    tip + Vec2::new(-size * 0.2, size * 0.4),
                    Vec2::new(size * 1.6, size * 1.25),
                ),
                0.0,
                Color32::from_rgb(228, 183, 145),
            );
            painter.line_segment(
                [tip, tip + Vec2::new(size * 0.9, size * 0.95)],
                Stroke::new(size * 0.28, Color32::from_rgb(48, 48, 52)),
            );
        }
    }
}

fn smoothed_hand_tip(
    rect: Rect,
    runtime: &RuntimeImage,
    layer: &Layer,
    progress: f32,
) -> Pos2 {
    const SMOOTHING_SPAN: f32 = 0.024;
    let mut sum = Vec2::ZERO;
    let mut weight_sum: f32 = 0.0;
    for sample in 0..6 {
        let t = sample as f32 / 5.0;
        let sample_progress = (progress - SMOOTHING_SPAN * t).clamp(0.0, 1.0);
        let point = hand_tip(rect, runtime, layer, sample_progress);
        let weight = 1.0 - t * 0.65;
        sum += point.to_vec2() * weight;
        weight_sum += weight;
    }
    Pos2::new(
        sum.x / weight_sum.max(0.001),
        sum.y / weight_sum.max(0.001),
    )
}

fn hand_tip(rect: Rect, runtime: &RuntimeImage, layer: &Layer, progress: f32) -> Pos2 {
    let progress = hand_motion_progress(progress, layer.animation.hand_speed);
    if matches!(
        layer.animation.style,
        AnimationStyle::Contour
            | AnimationStyle::OutlineFill
            | AnimationStyle::IllustFill
            | AnimationStyle::OutlineOnly
    ) {
        let points = select_edge_path(&runtime.edges, &layer.animation);
        if !points.is_empty() {
            let index = ((points.len() - 1) as f32 * progress.clamp(0.0, 1.0)).round()
                as usize;
            return Pos2::new(
                rect.left() + points[index][0] * rect.width(),
                rect.top() + points[index][1] * rect.height(),
            );
        }
    }
    if let Some(region) = reveal_regions(layer, progress).last() {
        return Pos2::new(
            rect.left() + (region.u0 + region.u1) * 0.5 * rect.width(),
            rect.top() + (region.v0 + region.v1) * 0.5 * rect.height(),
        );
    }
    rect.center()
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

fn paint_background(painter: &egui::Painter, rect: Rect, project: &Project) {
    match &project.canvas.background {
        Background::Solid(color) => {
            painter.rect_filled(
                rect,
                0.0,
                Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]),
            );
        }
        Background::Transparent => checker(painter, rect),
        Background::Notebook => {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(252, 249, 243));
            let step = (project.canvas.height / 36).max(12) as f32
                * rect.height()
                / project.canvas.height.max(1) as f32;
            horizontal_lines(painter, rect, Color32::from_rgb(205, 218, 235), step);
        }
        Background::Graph => {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(252, 249, 243));
            let step = (project.canvas.width / 32).max(12) as f32
                * rect.width()
                / project.canvas.width.max(1) as f32;
            grid(painter, rect, Color32::from_rgb(220, 222, 226), step);
        }
        Background::Cream => {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(249, 242, 218));
        }
        Background::Chalkboard => {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(37, 53, 48));
            let step = (project.canvas.width / 22).max(14) as f32
                * rect.width()
                / project.canvas.width.max(1) as f32;
            grid(painter, rect, Color32::from_rgb(60, 78, 70), step);
        }
        Background::SoftGradient => paint_soft_gradient(painter, rect),
        Background::WarmWhite => {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(252, 249, 243));
        }
        Background::Blueprint => {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(20, 55, 92));
            let step = (project.canvas.width / 28).max(12) as f32
                * rect.width()
                / project.canvas.width.max(1) as f32;
            grid(painter, rect, Color32::from_rgb(70, 115, 155), step);
        }
        Background::Kraft => {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(198, 164, 113));
        }
        Background::Dark => {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(28, 30, 34));
        }
        Background::Linen => {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(236, 231, 219));
            let step = (project.canvas.width / 70).max(8) as f32
                * rect.width()
                / project.canvas.width.max(1) as f32;
            grid(painter, rect, Color32::from_rgb(220, 214, 202), step);
        }
    }
}

fn paint_soft_gradient(painter: &egui::Painter, rect: Rect) {
    let mut mesh = Mesh::default();
    let top = Color32::from_rgb(250, 252, 255);
    let bottom = Color32::from_rgb(236, 243, 255);
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    painter.add(egui::Shape::mesh(mesh));
}

fn horizontal_lines(painter: &egui::Painter, rect: Rect, color: Color32, step: f32) {
    let step = step.max(1.0);
    let mut y = rect.top() + step;
    while y < rect.bottom() {
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(0.6_f32, color),
        );
        y += step;
    }
}

fn grid(painter: &egui::Painter, rect: Rect, color: Color32, step: f32) {
    let step = step.max(1.0);
    let mut x = rect.left();
    while x < rect.right() {
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            Stroke::new(0.5_f32, color),
        );
        x += step;
    }
    let mut y = rect.top();
    while y < rect.bottom() {
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(0.5_f32, color),
        );
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
            let checker = Rect::from_min_max(
                Pos2::new(x, y),
                Pos2::new((x + step).min(rect.right()), (y + step).min(rect.bottom())),
            );
            painter.rect_filled(
                checker,
                0.0,
                if (row + col) % 2 == 0 {
                    Color32::from_gray(225)
                } else {
                    Color32::from_gray(245)
                },
            );
            x += step;
            col += 1;
        }
        y += step;
        row += 1;
    }
}
