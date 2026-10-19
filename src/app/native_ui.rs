use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};
use std::time::Instant;

use crate::model::{
    AnimationStyle, Background, ColoringStyle, DetectionAlgorithm, ExportFormat, HandStyle,
    LayerContent, RevealStyle, StrokeStyle, TextAlign, TextDirection, TextDrawStyle,
};
use crate::render;

use super::{
    background_choices, background_label, bg_choice, fingerprint, uses_chunks, InkplainerApp,
    SlicerState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum LeftPanelTab {
    #[default]
    Animation,
    Drawing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum InputMode {
    #[default]
    Image,
    Text,
}

pub(crate) fn configure(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = Color32::WHITE;
    visuals.window_fill = Color32::WHITE;
    visuals.extreme_bg_color = Color32::from_rgb(245, 245, 245);
    visuals.faint_bg_color = Color32::from_rgb(250, 250, 250);
    visuals.selection.bg_fill = Color32::from_rgb(26, 26, 26);
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(7.0, 6.0);
    style.spacing.button_padding = egui::vec2(9.0, 5.0);
    style.spacing.interact_size.y = 28.0;
    ctx.set_style(style);
}

pub(crate) fn update(
    app: &mut InkplainerApp,
    ctx: &egui::Context,
    left_tab: &mut LeftPanelTab,
    input_mode: &mut InputMode,
) {
    let before = app.project.clone();

    app.update_playback(ctx);
    app.process_shortcuts(ctx);
    app.poll_export(ctx);

    top_bar(app, ctx);
    bottom_bar(app, ctx);
    right_panel(app, ctx, input_mode);
    left_panel(app, ctx, left_tab);
    center_canvas(app, ctx);

    app.project_browser_window(ctx);
    app.slicer_window(ctx);
    app.sync_fonts(ctx);
    app.maybe_autosave();

    if !app.suppress_history_once && fingerprint(&before) != fingerprint(&app.project) {
        let duplicate = app
            .undo_stack
            .last()
            .map(|project| fingerprint(project) == fingerprint(&before))
            .unwrap_or(false);
        if !duplicate {
            app.undo_stack.push(before);
            if app.undo_stack.len() > 100 {
                app.undo_stack.remove(0);
            }
            app.redo_stack.clear();
        }
    }
    app.suppress_history_once = false;
}

fn panel_frame(fill: Color32) -> egui::Frame {
    egui::Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, Color32::from_black_alpha(20)))
}

fn section_title(ui: &mut egui::Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(9.0)
            .strong()
            .color(Color32::from_gray(112)),
    );
}

fn top_bar(app: &mut InkplainerApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("native_top_bar")
        .exact_height(52.0)
        .frame(panel_frame(Color32::WHITE))
        .show(ctx, |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.add_sized(
                    [30.0, 30.0],
                    egui::Button::new(RichText::new("✎").size(18.0))
                        .fill(Color32::from_rgb(232, 232, 232)),
                );
                ui.label(RichText::new("Inkplainer").size(20.0).strong());
                ui.separator();

                if ui.button("New").clicked() {
                    app.new_project();
                }
                if ui.button("Projects").clicked() {
                    app.open_project_browser();
                }
                if ui.button("Open").clicked() {
                    app.open_project(ctx);
                }
                if ui.button("Save").clicked() {
                    app.save_project();
                }
                ui.menu_button("More", |ui| {
                    if ui.button("Save As…").clicked() {
                        app.save_project_as();
                        ui.close();
                    }
                    if ui
                        .add_enabled(app.autosave_available, egui::Button::new("Restore Autosave"))
                        .clicked()
                    {
                        app.restore_autosave(ctx);
                        ui.close();
                    }
                });
                ui.add_sized(
                    [160.0, 28.0],
                    egui::TextEdit::singleline(&mut app.project.name).hint_text("Untitled Project"),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let exporting = app.export_job.is_some();
                    let export_label = if exporting { "Exporting…" } else { "Export Video" };
                    if ui
                        .add_enabled(
                            !exporting,
                            egui::Button::new(
                                RichText::new(export_label)
                                    .strong()
                                    .color(Color32::from_rgb(184, 32, 32)),
                            ),
                        )
                        .clicked()
                    {
                        app.start_video_export(matches!(app.project.export.format, ExportFormat::WebM));
                    }
                    ui.menu_button("Export settings", |ui| {
                        app.export_controls(ui);
                        ui.separator();
                        if ui
                            .add_enabled(app.export_job.is_none(), egui::Button::new("Export PNG"))
                            .clicked()
                        {
                            app.export_png();
                            ui.close();
                        }
                    });
                    ui.hyperlink_to(
                        "Open Source",
                        "https://github.com/thedusk4203/Inkplainer-RustApp",
                    );
                    if ui
                        .add_enabled(!app.redo_stack.is_empty(), egui::Button::new("↷"))
                        .on_hover_text("Redo")
                        .clicked()
                    {
                        app.redo(ctx);
                    }
                    if ui
                        .add_enabled(!app.undo_stack.is_empty(), egui::Button::new("↶"))
                        .on_hover_text("Undo")
                        .clicked()
                    {
                        app.undo(ctx);
                    }
                });
            });
        });
}

fn left_panel(app: &mut InkplainerApp, ctx: &egui::Context, tab: &mut LeftPanelTab) {
    egui::SidePanel::left("native_left_panel")
        .exact_width(262.0)
        .resizable(false)
        .frame(panel_frame(Color32::WHITE))
        .show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Presets").size(11.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("Settings").size(10.0).color(Color32::from_gray(145)));
                });
            });
            ui.horizontal(|ui| {
                if ui.small_button("Quick Reveal").clicked() {
                    app.apply_builtin_preset(0);
                }
                if ui.small_button("Sketch").clicked() {
                    app.apply_builtin_preset(1);
                }
                if ui.small_button("Blueprint").clicked() {
                    app.apply_builtin_preset(2);
                }
            });
            ui.horizontal(|ui| {
                if ui.small_button("Illustrated").clicked() {
                    app.apply_builtin_preset(3);
                }
                app.preset_controls(ui);
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.selectable_value(tab, LeftPanelTab::Animation, "Animation");
                ui.selectable_value(tab, LeftPanelTab::Drawing, "Drawing");
            });
            ui.separator();

            if let Some(index) = app.selected_layer {
                if let Some(layer) = app.project.layers.get(index) {
                    ui.group(|ui| {
                        section_title(ui, "SELECTED LAYER");
                        ui.label(RichText::new(&layer.name).size(11.0).strong());
                    });
                }
            } else {
                ui.group(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("Select a layer on the right →")
                                .size(10.0)
                                .color(Color32::from_gray(140)),
                        );
                        ui.label(
                            RichText::new("to edit its animation settings.")
                                .size(10.0)
                                .color(Color32::from_gray(140)),
                        );
                    });
                });
            }

            let available_h = (ui.available_height() - 58.0).max(100.0);
            egui::ScrollArea::vertical()
                .max_height(available_h)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let Some(index) = app.selected_layer else {
                        return;
                    };
                    let Some(layer) = app.project.layers.get_mut(index) else {
                        return;
                    };

                    match *tab {
                        LeftPanelTab::Animation => {
                            ui.add_space(4.0);
                            section_title(ui, "ANIMATION");
                            ui.group(|ui| {
                                ui.label(RichText::new("Animation Style").size(10.0).strong());
                                egui::ComboBox::from_id_salt("native_animation_style")
                                    .selected_text(layer.animation.style.label())
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        for style in AnimationStyle::ALL {
                                            ui.selectable_value(
                                                &mut layer.animation.style,
                                                style,
                                                style.label(),
                                            );
                                        }
                                    });
                                ui.add(
                                    egui::Slider::new(&mut layer.animation.speed, 1.0..=100.0)
                                        .text("Reveal speed"),
                                );
                                ui.add(
                                    egui::Slider::new(&mut layer.animation.hand_speed, 1.0..=20.0)
                                        .text("Hand speed"),
                                );
                                egui::ComboBox::from_id_salt("native_hand_style")
                                    .selected_text(layer.animation.hand.label())
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        for hand in HandStyle::ALL {
                                            ui.selectable_value(
                                                &mut layer.animation.hand,
                                                hand,
                                                hand.label(),
                                            );
                                        }
                                    });
                                if layer.animation.style == AnimationStyle::Scanner {
                                    ui.checkbox(&mut layer.animation.zigzag, "Zigzag scan");
                                }
                                if uses_chunks(layer.animation.style) {
                                    ui.add(
                                        egui::Slider::new(&mut layer.animation.chunks, 6..=80)
                                            .text("Chunks"),
                                    );
                                }
                            });

                            ui.add_space(6.0);
                            section_title(ui, "SPECIALIZED");
                            ui.label(
                                RichText::new("Subject-aware drawing order")
                                    .size(9.0)
                                    .color(Color32::from_gray(145)),
                            );
                            for row in [
                                [AnimationStyle::Human, AnimationStyle::Animal],
                                [AnimationStyle::Portrait, AnimationStyle::Vehicle],
                                [AnimationStyle::Building, AnimationStyle::Landscape],
                                [AnimationStyle::Spiral, AnimationStyle::Nature],
                            ] {
                                ui.horizontal(|ui| {
                                    for style in row {
                                        if ui
                                            .selectable_label(
                                                layer.animation.style == style,
                                                style.label(),
                                            )
                                            .clicked()
                                        {
                                            layer.animation.style = style;
                                        }
                                    }
                                });
                            }
                        }
                        LeftPanelTab::Drawing => {
                            ui.add_space(4.0);
                            section_title(ui, "DRAWING STYLE");
                            ui.group(|ui| {
                                egui::ComboBox::from_id_salt("native_drawing_style")
                                    .selected_text(layer.animation.style.label())
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        for style in [
                                            AnimationStyle::Contour,
                                            AnimationStyle::OutlineChunks,
                                            AnimationStyle::OutlineFill,
                                            AnimationStyle::IllustFill,
                                            AnimationStyle::OutlineOnly,
                                            AnimationStyle::TextDraw,
                                        ] {
                                            ui.selectable_value(
                                                &mut layer.animation.style,
                                                style,
                                                style.label(),
                                            );
                                        }
                                    });

                                egui::ComboBox::from_id_salt("native_detection_algorithm")
                                    .selected_text(layer.animation.detection_algorithm.label())
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        for algorithm in DetectionAlgorithm::ALL {
                                            ui.selectable_value(
                                                &mut layer.animation.detection_algorithm,
                                                algorithm,
                                                algorithm.label(),
                                            );
                                        }
                                    });
                                ui.add(
                                    egui::Slider::new(
                                        &mut layer.animation.outline_detection,
                                        0..=100,
                                    )
                                    .text("Outline detection"),
                                );
                                egui::ComboBox::from_id_salt("native_stroke_style")
                                    .selected_text(layer.animation.stroke_style.label())
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        for style in StrokeStyle::ALL {
                                            ui.selectable_value(
                                                &mut layer.animation.stroke_style,
                                                style,
                                                style.label(),
                                            );
                                        }
                                    });
                                ui.add(
                                    egui::Slider::new(
                                        &mut layer.animation.outline_thickness,
                                        0.5..=8.0,
                                    )
                                    .text("Outline thickness"),
                                );
                                ui.add(
                                    egui::Slider::new(
                                        &mut layer.animation.outline_opacity,
                                        0.0..=1.0,
                                    )
                                    .text("Opacity"),
                                );
                                let mut outline_color = Color32::from_rgba_unmultiplied(
                                    layer.animation.outline_color[0],
                                    layer.animation.outline_color[1],
                                    layer.animation.outline_color[2],
                                    layer.animation.outline_color[3],
                                );
                                ui.horizontal(|ui| {
                                    ui.label("Outline color");
                                    if ui.color_edit_button_srgba(&mut outline_color).changed() {
                                        layer.animation.outline_color = [
                                            outline_color.r(),
                                            outline_color.g(),
                                            outline_color.b(),
                                            outline_color.a(),
                                        ];
                                    }
                                });
                                ui.checkbox(
                                    &mut layer.animation.outline_visible,
                                    "Show outlines on export",
                                );
                                if ui
                                    .checkbox(
                                        &mut layer.animation.color_region_outlines,
                                        "Color region outlines",
                                    )
                                    .changed()
                                    && layer.animation.color_region_outlines
                                {
                                    layer.animation.real_image_edges = false;
                                }
                                if ui
                                    .checkbox(
                                        &mut layer.animation.real_image_edges,
                                        "Real image edges",
                                    )
                                    .changed()
                                    && layer.animation.real_image_edges
                                {
                                    layer.animation.color_region_outlines = false;
                                }
                            });

                            if matches!(
                                layer.animation.style,
                                AnimationStyle::OutlineFill | AnimationStyle::IllustFill
                            ) {
                                ui.add_space(6.0);
                                ui.group(|ui| {
                                    ui.checkbox(
                                        &mut layer.animation.reveal_original,
                                        "Reveal original image at end",
                                    );
                                    egui::ComboBox::from_id_salt("native_reveal_style")
                                        .selected_text(layer.animation.reveal_style.label())
                                        .width(ui.available_width())
                                        .show_ui(ui, |ui| {
                                            for style in RevealStyle::ALL {
                                                ui.selectable_value(
                                                    &mut layer.animation.reveal_style,
                                                    style,
                                                    style.label(),
                                                );
                                            }
                                        });
                                });
                            }

                            if layer.animation.style == AnimationStyle::IllustFill {
                                egui::ComboBox::from_id_salt("native_coloring_style")
                                    .selected_text(layer.animation.coloring_style.label())
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        for style in ColoringStyle::ALL {
                                            ui.selectable_value(
                                                &mut layer.animation.coloring_style,
                                                style,
                                                style.label(),
                                            );
                                        }
                                    });
                            }

                            if layer.animation.style == AnimationStyle::TextDraw {
                                egui::ComboBox::from_id_salt("native_text_direction")
                                    .selected_text(layer.animation.text_direction.label())
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        for direction in TextDirection::ALL {
                                            ui.selectable_value(
                                                &mut layer.animation.text_direction,
                                                direction,
                                                direction.label(),
                                            );
                                        }
                                    });
                                egui::ComboBox::from_id_salt("native_text_draw_style")
                                    .selected_text(layer.animation.text_draw_style.label())
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        for style in TextDrawStyle::ALL {
                                            ui.selectable_value(
                                                &mut layer.animation.text_draw_style,
                                                style,
                                                style.label(),
                                            );
                                        }
                                    });
                            }
                        }
                    }
                });

            ui.separator();
            let generate = egui::Button::new(
                RichText::new("▶ Generate Animation")
                    .color(Color32::WHITE)
                    .strong(),
            )
            .fill(Color32::from_rgb(26, 26, 26));
            if ui
                .add_sized([ui.available_width(), 38.0], generate)
                .clicked()
            {
                app.playhead = 0.0;
                app.playing = true;
                app.last_tick = Instant::now();
            }
        });
}

fn right_panel(app: &mut InkplainerApp, ctx: &egui::Context, mode: &mut InputMode) {
    egui::SidePanel::right("native_right_panel")
        .exact_width(296.0)
        .resizable(false)
        .frame(panel_frame(Color32::WHITE))
        .show(ctx, |ui| {
            ui.add_space(9.0);
            ui.heading("Layers");
            ui.label(
                RichText::new("Upload & arrange your content")
                    .size(10.0)
                    .color(Color32::from_gray(144)),
            );
            ui.separator();

            section_title(ui, "INPUT MODE");
            ui.horizontal(|ui| {
                ui.selectable_value(mode, InputMode::Image, "Image");
                ui.selectable_value(mode, InputMode::Text, "Text");
            });

            match *mode {
                InputMode::Image => {
                    ui.add_space(4.0);
                    let import = egui::Button::new("＋  Drop images or browse")
                        .fill(Color32::from_rgb(250, 250, 250));
                    if ui
                        .add_sized([ui.available_width(), 46.0], import)
                        .clicked()
                    {
                        app.import_images(ctx);
                    }
                    ui.label(
                        RichText::new("PNG, JPG, GIF, WebP · Multiple files OK")
                            .size(9.0)
                            .color(Color32::from_gray(150)),
                    );
                    ui.add_space(5.0);
                    section_title(ui, "CANVAS BACKGROUND");
                    egui::ComboBox::from_id_salt("native_background")
                        .selected_text(background_label(&app.project.canvas.background))
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            for (value, label) in background_choices() {
                                bg_choice(ui, &mut app.project.canvas.background, value, label);
                            }
                        });
                    if let Background::Solid(color) = &mut app.project.canvas.background {
                        let mut value = Color32::from_rgba_unmultiplied(
                            color[0], color[1], color[2], color[3],
                        );
                        if ui.color_edit_button_srgba(&mut value).changed() {
                            *color = [value.r(), value.g(), value.b(), value.a()];
                        }
                    }
                }
                InputMode::Text => {
                    ui.add_space(4.0);
                    if ui
                        .add_sized(
                            [ui.available_width(), 38.0],
                            egui::Button::new("＋  Add text layer")
                                .fill(Color32::from_rgb(250, 250, 250)),
                        )
                        .clicked()
                    {
                        app.add_text();
                    }

                    if let Some(index) = app.selected_layer {
                        if let Some(layer) = app.project.layers.get_mut(index) {
                            if let LayerContent::Text(text) = &mut layer.content {
                                ui.add_space(6.0);
                                ui.text_edit_multiline(&mut text.text);
                                egui::ComboBox::from_id_salt("native_font_family")
                                    .selected_text(if text.font_family.trim().is_empty() {
                                        "Segoe UI"
                                    } else {
                                        &text.font_family
                                    })
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        egui::ScrollArea::vertical()
                                            .max_height(180.0)
                                            .show(ui, |ui| {
                                                for family in &app.font_families {
                                                    ui.selectable_value(
                                                        &mut text.font_family,
                                                        family.clone(),
                                                        family,
                                                    );
                                                }
                                            });
                                    });
                                ui.horizontal(|ui| {
                                    ui.label("Custom family");
                                    ui.text_edit_singleline(&mut text.font_family);
                                });
                                ui.add(
                                    egui::Slider::new(&mut text.size, 10.0..=400.0).text("px"),
                                );
                                ui.horizontal(|ui| {
                                    ui.checkbox(&mut text.bold, "Bold");
                                    ui.checkbox(&mut text.italic, "Italic");
                                });
                                egui::ComboBox::from_id_salt("native_text_align")
                                    .selected_text(match text.align {
                                        TextAlign::Left => "Left",
                                        TextAlign::Center => "Center",
                                        TextAlign::Right => "Right",
                                    })
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut text.align, TextAlign::Left, "Left");
                                        ui.selectable_value(
                                            &mut text.align,
                                            TextAlign::Center,
                                            "Center",
                                        );
                                        ui.selectable_value(
                                            &mut text.align,
                                            TextAlign::Right,
                                            "Right",
                                        );
                                    });
                                ui.add(
                                    egui::Slider::new(&mut text.line_height, 0.8..=2.5)
                                        .text("Line height"),
                                );
                                ui.add(
                                    egui::Slider::new(&mut text.letter_spacing, -5.0..=30.0)
                                        .text("Spacing"),
                                );
                                let mut color = Color32::from_rgba_unmultiplied(
                                    text.color[0],
                                    text.color[1],
                                    text.color[2],
                                    text.color[3],
                                );
                                ui.horizontal(|ui| {
                                    ui.label("Text color");
                                    if ui.color_edit_button_srgba(&mut color).changed() {
                                        text.color = [color.r(), color.g(), color.b(), color.a()];
                                    }
                                });
                            }
                        }
                    }
                }
            }

            ui.separator();
            let can_slice = app.selected_image().is_some();
            ui.horizontal(|ui| {
                ui.label(RichText::new("Layers").size(11.0).strong());
                if ui.add_enabled(can_slice, egui::Button::new("Slice")).clicked() {
                    app.slicer = SlicerState {
                        open: true,
                        ..Default::default()
                    };
                }
                ui.label(
                    RichText::new(format!("{} layers", app.project.layers.len()))
                        .size(9.0)
                        .color(Color32::from_gray(145)),
                );
            });

            let mut select_index = None;
            let mut delete_index = None;
            let mut move_up = None;
            let mut move_down = None;

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if app.project.layers.is_empty() {
                        ui.add_space(16.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                RichText::new("↑ Drop images above to begin")
                                    .size(10.0)
                                    .color(Color32::from_gray(150)),
                            );
                        });
                    }

                    for index in (0..app.project.layers.len()).rev() {
                        let selected = app.selected_layer == Some(index);
                        let layer = &mut app.project.layers[index];
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut layer.visible, "");
                                if ui.selectable_label(selected, &layer.name).clicked() {
                                    select_index = Some(index);
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("×").clicked() {
                                            delete_index = Some(index);
                                        }
                                    },
                                );
                            });

                            if selected {
                                ui.text_edit_singleline(&mut layer.name);
                                ui.horizontal(|ui| {
                                    if ui.small_button("↑").clicked() {
                                        move_up = Some(index);
                                    }
                                    if ui.small_button("↓").clicked() {
                                        move_down = Some(index);
                                    }
                                    let mut enabled = layer.anim_order.is_some();
                                    let mut order = layer.anim_order.unwrap_or(1);
                                    ui.label("Order");
                                    if ui.checkbox(&mut enabled, "").changed() {
                                        layer.anim_order = enabled.then_some(order.max(1));
                                    }
                                    if enabled
                                        && ui
                                            .add(egui::DragValue::new(&mut order).range(1..=999))
                                            .changed()
                                    {
                                        layer.anim_order = Some(order.max(1));
                                    }
                                });
                                let mut group = layer.group.clone().unwrap_or_default();
                                ui.horizontal(|ui| {
                                    ui.label("Group");
                                    if ui.text_edit_singleline(&mut group).changed() {
                                        layer.group = if group.trim().is_empty() {
                                            None
                                        } else {
                                            Some(group.clone())
                                        };
                                    }
                                });
                                ui.add(
                                    egui::Slider::new(&mut layer.opacity, 0.0..=1.0)
                                        .text("Opacity"),
                                );
                                ui.horizontal(|ui| {
                                    ui.label("X");
                                    ui.add(
                                        egui::DragValue::new(&mut layer.transform.x).speed(1.0),
                                    );
                                    ui.label("Y");
                                    ui.add(
                                        egui::DragValue::new(&mut layer.transform.y).speed(1.0),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("W");
                                    ui.add(
                                        egui::DragValue::new(&mut layer.transform.w)
                                            .speed(1.0)
                                            .range(1.0..=10000.0),
                                    );
                                    ui.label("H");
                                    ui.add(
                                        egui::DragValue::new(&mut layer.transform.h)
                                            .speed(1.0)
                                            .range(1.0..=10000.0),
                                    );
                                });
                            }
                        });
                    }
                });

            if let Some(index) = select_index {
                app.selected_layer = Some(index);
            }
            if let Some(index) = move_up {
                if index + 1 < app.project.layers.len() {
                    app.project.layers.swap(index, index + 1);
                    app.selected_layer = Some(index + 1);
                }
            }
            if let Some(index) = move_down {
                if index > 0 {
                    app.project.layers.swap(index, index - 1);
                    app.selected_layer = Some(index - 1);
                }
            }
            if let Some(index) = delete_index {
                app.selected_layer = Some(index);
                app.delete_selected();
            }
        });
}

fn center_canvas(app: &mut InkplainerApp, ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(Color32::from_rgb(216, 216, 216)))
        .show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} × {}",
                        app.project.canvas.width, app.project.canvas.height
                    ))
                    .size(10.0)
                    .color(Color32::from_gray(82)),
                );
                let style = app
                    .selected_layer
                    .and_then(|index| app.project.layers.get(index))
                    .map(|layer| layer.animation.style.label())
                    .unwrap_or("Scanner");
                ui.label(
                    RichText::new(style)
                        .size(10.0)
                        .color(Color32::from_gray(82)),
                );
                if app.playhead >= 0.999 {
                    ui.label(
                        RichText::new("Done")
                            .size(9.0)
                            .strong()
                            .color(Color32::from_rgb(22, 130, 65)),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(app.selected_image().is_some(), egui::Button::new("Slicer"))
                        .clicked()
                    {
                        app.slicer = SlicerState {
                            open: true,
                            ..Default::default()
                        };
                    }
                });
            });
            ui.add_space(5.0);

            let canvas_height = (ui.available_height() - 48.0).max(80.0);
            let result = ui
                .allocate_ui(Vec2::new(ui.available_width(), canvas_height), |ui| {
                    ui.vertical_centered(|ui| {
                        render::paint_project(
                            ui,
                            &app.project,
                            &app.assets,
                            app.selected_layer,
                            app.playhead,
                        )
                    })
                    .inner
                })
                .inner;

            if let (Some(index), Some(delta)) =
                (app.selected_layer, result.selected_drag_delta_canvas)
            {
                if let Some(layer) = app.project.layers.get_mut(index) {
                    layer.transform.x += delta.x;
                    layer.transform.y += delta.y;
                }
            }

            if let (Some(index), Some(resize)) =
                (app.selected_layer, result.selected_resize_delta)
            {
                if let Some(layer) = app.project.layers.get_mut(index) {
                    const MIN_SIZE: f32 = 12.0;
                    let transform = &mut layer.transform;
                    match resize.handle {
                        render::ResizeHandle::TopLeft => {
                            let new_width =
                                (transform.w - resize.delta_canvas.x).max(MIN_SIZE);
                            let new_height =
                                (transform.h - resize.delta_canvas.y).max(MIN_SIZE);
                            transform.x += transform.w - new_width;
                            transform.y += transform.h - new_height;
                            transform.w = new_width;
                            transform.h = new_height;
                        }
                        render::ResizeHandle::TopRight => {
                            let new_height =
                                (transform.h - resize.delta_canvas.y).max(MIN_SIZE);
                            transform.y += transform.h - new_height;
                            transform.w =
                                (transform.w + resize.delta_canvas.x).max(MIN_SIZE);
                            transform.h = new_height;
                        }
                        render::ResizeHandle::BottomLeft => {
                            let new_width =
                                (transform.w - resize.delta_canvas.x).max(MIN_SIZE);
                            transform.x += transform.w - new_width;
                            transform.w = new_width;
                            transform.h =
                                (transform.h + resize.delta_canvas.y).max(MIN_SIZE);
                        }
                        render::ResizeHandle::BottomRight => {
                            transform.w =
                                (transform.w + resize.delta_canvas.x).max(MIN_SIZE);
                            transform.h =
                                (transform.h + resize.delta_canvas.y).max(MIN_SIZE);
                        }
                    }
                }
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.small_button("⟳ Restart").clicked() {
                    app.playhead = 0.0;
                    app.playing = true;
                    app.last_tick = Instant::now();
                }
                let label = if app.playing { "⏸ Pause" } else { "▶ Play" };
                if ui.small_button(label).clicked() {
                    if app.playhead >= 1.0 {
                        app.playhead = 0.0;
                    }
                    app.playing = !app.playing;
                    app.last_tick = Instant::now();
                }
                ui.add(
                    egui::Slider::new(&mut app.playhead, 0.0..=1.0)
                        .show_value(false)
                        .text("Timeline"),
                );
                ui.label(
                    RichText::new(format!("{}%", (app.playhead * 100.0).round() as u32))
                        .size(10.0),
                );
            });
        });
}

fn bottom_bar(app: &mut InkplainerApp, ctx: &egui::Context) {
    egui::TopBottomPanel::bottom("native_bottom_bar")
        .exact_height(112.0)
        .frame(panel_frame(Color32::WHITE))
        .show(ctx, |ui| {
            ui.add_space(5.0);
            ui.columns(3, |columns| {
                columns[0].label(RichText::new("Hand Style").size(10.0).strong());
                if let Some(index) = app.selected_layer {
                    if let Some(layer) = app.project.layers.get_mut(index) {
                        for row in [
                            [HandStyle::Ghost, HandStyle::Hand1, HandStyle::Hand2],
                            [HandStyle::Hand3, HandStyle::Pen, HandStyle::Cartoon],
                        ] {
                            columns[0].horizontal(|ui| {
                                for hand in row {
                                    if ui
                                        .selectable_label(layer.animation.hand == hand, hand.label())
                                        .clicked()
                                    {
                                        layer.animation.hand = hand;
                                    }
                                }
                            });
                        }
                        if ui_select_hand_pixel(&mut columns[0], layer.animation.hand) {
                            layer.animation.hand = HandStyle::PixelArt;
                        }
                    }
                } else {
                    columns[0].label(
                        RichText::new("Select a layer")
                            .size(10.0)
                            .color(Color32::from_gray(145)),
                    );
                }

                columns[1].label(RichText::new("Speed").size(10.0).strong());
                if let Some(index) = app.selected_layer {
                    if let Some(layer) = app.project.layers.get_mut(index) {
                        columns[1].add(
                            egui::Slider::new(&mut layer.animation.speed, 1.0..=100.0)
                                .text("Reveal"),
                        );
                        columns[1].add(
                            egui::Slider::new(&mut layer.animation.hand_speed, 1.0..=20.0)
                                .text("Hand"),
                        );
                    }
                } else {
                    columns[1].label(
                        RichText::new("Reveal —   Hand —")
                            .size(10.0)
                            .color(Color32::from_gray(145)),
                    );
                }

                columns[2].label(RichText::new("Canvas Size").size(10.0).strong());
                columns[2].horizontal(|ui| {
                    if ui.small_button("16:9").clicked() {
                        app.project.canvas.width = 1280;
                        app.project.canvas.height = 720;
                    }
                    if ui.small_button("9:16").clicked() {
                        app.project.canvas.width = 720;
                        app.project.canvas.height = 1280;
                    }
                    if ui.small_button("1:1").clicked() {
                        app.project.canvas.width = 1080;
                        app.project.canvas.height = 1080;
                    }
                });
                columns[2].horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut app.project.canvas.width)
                            .range(320..=3840)
                            .speed(1.0),
                    );
                    ui.label("×");
                    ui.add(
                        egui::DragValue::new(&mut app.project.canvas.height)
                            .range(240..=3840)
                            .speed(1.0),
                    );
                    ui.checkbox(&mut app.project.hand_glide_enabled, "Smooth hand");
                });
                columns[2].horizontal(|ui| {
                    ui.label("Duration");
                    ui.add(
                        egui::DragValue::new(&mut app.project.duration_seconds)
                            .speed(0.25)
                            .range(0.5..=120.0),
                    );
                    ui.label("s");
                    ui.checkbox(&mut app.project.autosave_enabled, "Autosave");
                });
            });

            if let Some(job) = &app.export_job {
                ui.separator();
                ui.horizontal(|ui| {
                    let progress = job.progress();
                    ui.add(
                        egui::ProgressBar::new(progress)
                            .desired_width(260.0)
                            .text(format!(
                                "Export {:>3}%",
                                (progress * 100.0).round() as u32
                            )),
                    );
                    if ui.button("Cancel export").clicked() {
                        job.cancel();
                    }
                    ui.label(
                        RichText::new(job.path.display().to_string())
                            .size(9.0)
                            .color(Color32::from_gray(130)),
                    );
                });
            } else {
                ui.label(
                    RichText::new(&app.status)
                        .size(9.0)
                        .color(Color32::from_gray(120)),
                );
            }
        });
}

fn ui_select_hand_pixel(ui: &mut egui::Ui, selected: HandStyle) -> bool {
    ui.selectable_label(selected == HandStyle::PixelArt, HandStyle::PixelArt.label())
        .clicked()
}
