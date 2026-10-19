from pathlib import Path

p = Path('src/app.rs')
s = p.read_text(encoding='utf-8')


def replace_fn(src: str, name: str, new_code: str) -> str:
    needle = f'    fn {name}('
    start = src.find(needle)
    if start < 0:
        raise RuntimeError(f'function {name} not found')
    brace = src.find('{', start)
    if brace < 0:
        raise RuntimeError(f'opening brace for {name} not found')
    depth = 0
    end = None
    in_str = False
    escape = False
    for i in range(brace, len(src)):
        ch = src[i]
        if in_str:
            if escape:
                escape = False
            elif ch == '\\':
                escape = True
            elif ch == '"':
                in_str = False
            continue
        if ch == '"':
            in_str = True
            continue
        if ch == '{':
            depth += 1
        elif ch == '}':
            depth -= 1
            if depth == 0:
                end = i + 1
                break
    if end is None:
        raise RuntimeError(f'closing brace for {name} not found')
    return src[:start] + new_code.rstrip() + src[end:]


if 'enum LeftPanelTab' not in s:
    s = s.replace(
        '#[derive(Debug, Clone, Copy, PartialEq, Eq)]\nenum SliceMode { Grid, Rectangles, Freehand }',
        '#[derive(Debug, Clone, Copy, PartialEq, Eq)]\nenum SliceMode { Grid, Rectangles, Freehand }\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\nenum LeftPanelTab { Animation, Drawing }\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq)]\nenum InputMode { Image, Text }'
    )
    s = s.replace(
        '    font_signature: String,\n}',
        '    font_signature: String,\n    left_panel_tab: LeftPanelTab,\n    input_mode: InputMode,\n}'
    )
    s = s.replace(
        '            font_signature: String::new(),\n        }',
        '            font_signature: String::new(),\n            left_panel_tab: LeftPanelTab::Animation,\n            input_mode: InputMode::Image,\n        }'
    )

if 'configure_inkplainer_style(&cc.egui_ctx);' not in s:
    s = s.replace(
        '        font_catalog::install_project_fonts(&cc.egui_ctx, &project);',
        '        font_catalog::install_project_fonts(&cc.egui_ctx, &project);\n        configure_inkplainer_style(&cc.egui_ctx);'
    )

s = replace_fn(s, 'top_bar', r'''    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_bar")
            .exact_height(52.0)
            .frame(egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0_f32, Color32::from_black_alpha(20))))
            .show(ctx, |ui| {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    let logo = egui::Button::new(RichText::new("✎").size(18.0)).fill(Color32::from_rgb(228, 228, 228));
                    ui.add_sized([30.0, 30.0], logo);
                    ui.label(RichText::new("Inkplainer").size(20.0).strong());
                    ui.separator();

                    if ui.button("Projects").clicked() { self.open_project_browser(); }
                    ui.add_sized([190.0, 28.0], egui::TextEdit::singleline(&mut self.project.name).hint_text("Untitled Project"));
                    ui.label(RichText::new("●").size(9.0).color(Color32::from_rgb(144, 144, 144)));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let exporting = self.export_job.is_some();
                        let export_label = if exporting { "Exporting…" } else { "Export Video" };
                        if ui.add_enabled(!exporting, egui::Button::new(RichText::new(export_label).color(Color32::from_rgb(180, 35, 35)))).clicked() {
                            self.start_video_export(matches!(self.project.export.format, ExportFormat::WebM));
                        }
                        ui.menu_button("Export settings", |ui| self.export_controls(ui));
                        ui.hyperlink_to("Open Source", "https://github.com/thedusk4203/Inkplainer-RustApp");
                        if ui.small_button("↶").on_hover_text("Undo").clicked() { self.undo(ctx); }
                        if ui.small_button("↷").on_hover_text("Redo").clicked() { self.redo(ctx); }
                    });
                });
            });
    }''')

s = replace_fn(s, 'settings_panel', r'''    fn settings_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("settings_panel")
            .exact_width(262.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0_f32, Color32::from_black_alpha(18))))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Presets").size(11.0).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("Settings").size(10.0).color(Color32::from_gray(144)));
                    });
                });
                self.preset_controls(ui);
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    let anim = self.left_panel_tab == LeftPanelTab::Animation;
                    if ui.add_sized([115.0, 30.0], egui::Button::new("Animation").selected(anim)).clicked() {
                        self.left_panel_tab = LeftPanelTab::Animation;
                    }
                    let drawing = self.left_panel_tab == LeftPanelTab::Drawing;
                    if ui.add_sized([115.0, 30.0], egui::Button::new("Drawing").selected(drawing)).clicked() {
                        self.left_panel_tab = LeftPanelTab::Drawing;
                    }
                });
                ui.separator();

                if let Some(idx) = self.selected_layer {
                    if let Some(layer) = self.project.layers.get(idx) {
                        ui.group(|ui| {
                            ui.label(RichText::new("SELECTED LAYER").size(9.0).color(Color32::from_gray(100)).strong());
                            ui.label(RichText::new(&layer.name).size(11.0).strong());
                        });
                    }
                } else {
                    ui.group(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("Select a layer on the right →").size(10.0).color(Color32::from_gray(140)));
                            ui.label(RichText::new("to edit its animation settings.").size(10.0).color(Color32::from_gray(140)));
                        });
                    });
                }

                let available_h = (ui.available_height() - 54.0).max(80.0);
                egui::ScrollArea::vertical().max_height(available_h).auto_shrink([false, false]).show(ui, |ui| {
                    let Some(idx) = self.selected_layer else { return; };
                    if idx >= self.project.layers.len() { return; }
                    let layer = &mut self.project.layers[idx];

                    match self.left_panel_tab {
                        LeftPanelTab::Animation => {
                            ui.add_space(4.0);
                            ui.label(RichText::new("ANIMATION").size(9.0).color(Color32::from_gray(120)).strong());
                            ui.group(|ui| {
                                ui.label(RichText::new("Animation Style").size(11.0).strong());
                                egui::ComboBox::from_id_salt("animation_style_clone")
                                    .selected_text(layer.animation.style.label())
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        for style in AnimationStyle::ALL {
                                            ui.selectable_value(&mut layer.animation.style, style, style.label());
                                        }
                                    });
                                if layer.animation.style == AnimationStyle::Scanner {
                                    ui.checkbox(&mut layer.animation.zigzag, "Zigzag scan");
                                }
                                if uses_chunks(layer.animation.style) {
                                    ui.add(egui::Slider::new(&mut layer.animation.chunks, 6..=80).text("Chunks"));
                                }
                            });
                            ui.add_space(6.0);
                            ui.label(RichText::new("SPECIALIZED").size(9.0).color(Color32::from_gray(120)).strong());
                            ui.small("Subject-aware drawing order.");
                            ui.horizontal_wrapped(|ui| {
                                for style in [AnimationStyle::Human, AnimationStyle::Animal, AnimationStyle::Portrait, AnimationStyle::Vehicle, AnimationStyle::Building, AnimationStyle::Landscape, AnimationStyle::Spiral, AnimationStyle::Nature] {
                                    if ui.selectable_label(layer.animation.style == style, style.label()).clicked() {
                                        layer.animation.style = style;
                                    }
                                }
                            });
                        }
                        LeftPanelTab::Drawing => {
                            ui.add_space(4.0);
                            ui.label(RichText::new("DRAWING STYLE").size(9.0).color(Color32::from_gray(120)).strong());
                            ui.group(|ui| {
                                egui::ComboBox::from_id_salt("drawing_style_clone")
                                    .selected_text(layer.animation.style.label())
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        for style in [AnimationStyle::OutlineFill, AnimationStyle::IllustFill, AnimationStyle::OutlineOnly, AnimationStyle::TextDraw] {
                                            ui.selectable_value(&mut layer.animation.style, style, style.label());
                                        }
                                    });

                                ui.label(RichText::new("Stroke Style").size(10.0).strong());
                                egui::ComboBox::from_id_salt("stroke_style_clone")
                                    .selected_text(layer.animation.stroke_style.label())
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        for style in StrokeStyle::ALL { ui.selectable_value(&mut layer.animation.stroke_style, style, style.label()); }
                                    });

                                if layer.animation.style == AnimationStyle::IllustFill {
                                    ui.label(RichText::new("Coloring Style").size(10.0).strong());
                                    egui::ComboBox::from_id_salt("color_style_clone")
                                        .selected_text(layer.animation.coloring_style.label())
                                        .width(ui.available_width())
                                        .show_ui(ui, |ui| {
                                            for style in ColoringStyle::ALL { ui.selectable_value(&mut layer.animation.coloring_style, style, style.label()); }
                                        });
                                }
                            });

                            ui.add_space(6.0);
                            ui.group(|ui| {
                                ui.label(RichText::new("Outline Detection").size(10.0).strong());
                                ui.add(egui::Slider::new(&mut layer.animation.outline_detection, 0..=100).text("Strict ↔ Fine"));
                                egui::ComboBox::from_id_salt("detect_algo_clone")
                                    .selected_text(layer.animation.detection_algorithm.label())
                                    .width(ui.available_width())
                                    .show_ui(ui, |ui| {
                                        for a in DetectionAlgorithm::ALL { ui.selectable_value(&mut layer.animation.detection_algorithm, a, a.label()); }
                                    });
                                ui.add(egui::Slider::new(&mut layer.animation.outline_thickness, 0.5..=8.0).text("Outline thickness"));
                                ui.add(egui::Slider::new(&mut layer.animation.outline_opacity, 0.0..=1.0).text("Opacity"));
                                ui.checkbox(&mut layer.animation.outline_visible, "Show outlines on export");
                                ui.checkbox(&mut layer.animation.color_region_outlines, "Color Region Outlines");
                                ui.checkbox(&mut layer.animation.real_image_edges, "Real Image Edges");
                            });

                            if matches!(layer.animation.style, AnimationStyle::OutlineFill | AnimationStyle::IllustFill) {
                                ui.add_space(6.0);
                                ui.group(|ui| {
                                    ui.label(RichText::new("Image Reveal").size(10.0).strong());
                                    ui.checkbox(&mut layer.animation.reveal_original, "Reveal original image at end");
                                    egui::ComboBox::from_id_salt("reveal_style_clone")
                                        .selected_text(layer.animation.reveal_style.label())
                                        .width(ui.available_width())
                                        .show_ui(ui, |ui| {
                                            for r in RevealStyle::ALL { ui.selectable_value(&mut layer.animation.reveal_style, r, r.label()); }
                                        });
                                });
                            }

                            if layer.animation.style == AnimationStyle::TextDraw {
                                ui.add_space(6.0);
                                ui.group(|ui| {
                                    ui.label(RichText::new("Text Draw Options").size(10.0).strong());
                                    egui::ComboBox::from_label("Start Direction").selected_text(layer.animation.text_direction.label()).show_ui(ui, |ui| {
                                        for d in TextDirection::ALL { ui.selectable_value(&mut layer.animation.text_direction, d, d.label()); }
                                    });
                                    egui::ComboBox::from_label("Fill Style").selected_text(layer.animation.text_draw_style.label()).show_ui(ui, |ui| {
                                        for d in TextDrawStyle::ALL { ui.selectable_value(&mut layer.animation.text_draw_style, d, d.label()); }
                                    });
                                });
                            }
                        }
                    }
                });

                ui.separator();
                let generate = egui::Button::new(RichText::new("▶ Generate Animation").color(Color32::WHITE).strong())
                    .fill(Color32::from_rgb(26, 26, 26));
                if ui.add_sized([ui.available_width(), 38.0], generate).clicked() {
                    self.playhead = 0.0;
                    self.playing = true;
                    self.last_tick = Instant::now();
                }
            });
    }''')

s = replace_fn(s, 'layers_panel', r'''    fn layers_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("layers_panel")
            .exact_width(296.0)
            .resizable(false)
            .frame(egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0_f32, Color32::from_black_alpha(18))))
            .show(ctx, |ui| {
                ui.add_space(9.0);
                ui.heading("Layers");
                ui.label(RichText::new("Upload & arrange your content").size(10.0).color(Color32::from_gray(144)));
                ui.separator();

                ui.label(RichText::new("Input Mode").size(9.0).color(Color32::from_gray(120)).strong());
                ui.horizontal(|ui| {
                    if ui.add_sized([132.0, 29.0], egui::Button::new("Image").selected(self.input_mode == InputMode::Image)).clicked() {
                        self.input_mode = InputMode::Image;
                    }
                    if ui.add_sized([132.0, 29.0], egui::Button::new("Text").selected(self.input_mode == InputMode::Text)).clicked() {
                        self.input_mode = InputMode::Text;
                    }
                });

                match self.input_mode {
                    InputMode::Image => {
                        ui.add_space(4.0);
                        let import_btn = egui::Button::new("＋  Drop images or browse").fill(Color32::from_rgb(250, 250, 250));
                        if ui.add_sized([ui.available_width(), 50.0], import_btn).clicked() { self.import_images(ctx); }
                        ui.vertical_centered(|ui| ui.label(RichText::new("PNG, JPG, GIF, WebP · Multiple files OK").size(9.0).color(Color32::from_gray(150))));
                        ui.add_space(4.0);
                        ui.label(RichText::new("Canvas Background").size(9.0).color(Color32::from_gray(120)).strong());
                        egui::ComboBox::from_id_salt("background_clone")
                            .selected_text(background_label(&self.project.canvas.background))
                            .width(ui.available_width())
                            .show_ui(ui, |ui| {
                                for (value, label) in background_choices() { bg_choice(ui, &mut self.project.canvas.background, value, label); }
                            });
                        if let Background::Solid(c) = &mut self.project.canvas.background {
                            let mut color = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
                            if ui.color_edit_button_srgba(&mut color).changed() { *c = [color.r(), color.g(), color.b(), color.a()]; }
                        }
                    }
                    InputMode::Text => {
                        ui.add_space(4.0);
                        let add = egui::Button::new("＋  Click canvas to place text").fill(Color32::from_rgb(250, 250, 250));
                        if ui.add_sized([ui.available_width(), 38.0], add).clicked() { self.add_text(); }
                        if let Some(idx) = self.selected_layer {
                            if let Some(layer) = self.project.layers.get_mut(idx) {
                                if let LayerContent::Text(text) = &mut layer.content {
                                    ui.add_space(6.0);
                                    ui.text_edit_multiline(&mut text.text);
                                    egui::ComboBox::from_label("Font").selected_text(&text.font_family).width(ui.available_width()).show_ui(ui, |ui| {
                                        egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
                                            for family in &self.font_families { ui.selectable_value(&mut text.font_family, family.clone(), family); }
                                        });
                                    });
                                    ui.add(egui::Slider::new(&mut text.size, 10.0..=400.0).text("px"));
                                    ui.horizontal(|ui| { ui.checkbox(&mut text.bold, "B"); ui.checkbox(&mut text.italic, "I"); });
                                    ui.add(egui::Slider::new(&mut text.line_height, 0.8..=2.5).text("Line height"));
                                    ui.add(egui::Slider::new(&mut text.letter_spacing, -5.0..=30.0).text("Spacing"));
                                }
                            }
                        }
                    }
                }

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Layers").size(11.0).strong());
                    if ui.add_enabled(self.selected_image().is_some(), egui::Button::new("Slice")).clicked() {
                        self.slicer = SlicerState { open: true, ..Default::default() };
                    }
                    ui.label(RichText::new(format!("{} layers", self.project.layers.len())).size(9.0).color(Color32::from_gray(145)));
                });

                let mut delete_idx = None;
                let mut move_up = None;
                let mut move_down = None;
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    if self.project.layers.is_empty() {
                        ui.add_space(16.0);
                        ui.vertical_centered(|ui| ui.label(RichText::new("↑ Drop images above to begin").size(10.0).color(Color32::from_gray(150))));
                    }
                    for idx in (0..self.project.layers.len()).rev() {
                        let selected = self.selected_layer == Some(idx);
                        let layer = &mut self.project.layers[idx];
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut layer.visible, "");
                                if ui.selectable_label(selected, &layer.name).clicked() { self.selected_layer = Some(idx); }
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("×").clicked() { delete_idx = Some(idx); }
                                });
                            });
                            if selected {
                                ui.horizontal(|ui| {
                                    if ui.small_button("↑").clicked() { move_up = Some(idx); }
                                    if ui.small_button("↓").clicked() { move_down = Some(idx); }
                                    let mut order = layer.anim_order.unwrap_or(1);
                                    ui.label("Order");
                                    if ui.add(egui::DragValue::new(&mut order).range(1..=999)).changed() { layer.anim_order = Some(order.max(1)); }
                                });
                                ui.add(egui::Slider::new(&mut layer.opacity, 0.0..=1.0).text("Opacity"));
                                ui.horizontal(|ui| {
                                    ui.label("X"); ui.add(egui::DragValue::new(&mut layer.transform.x).speed(1.0));
                                    ui.label("Y"); ui.add(egui::DragValue::new(&mut layer.transform.y).speed(1.0));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("W"); ui.add(egui::DragValue::new(&mut layer.transform.w).speed(1.0));
                                    ui.label("H"); ui.add(egui::DragValue::new(&mut layer.transform.h).speed(1.0));
                                });
                            }
                        });
                    }
                });

                if let Some(idx) = move_up {
                    if idx + 1 < self.project.layers.len() { self.project.layers.swap(idx, idx + 1); self.selected_layer = Some(idx + 1); }
                }
                if let Some(idx) = move_down {
                    if idx > 0 { self.project.layers.swap(idx, idx - 1); self.selected_layer = Some(idx - 1); }
                }
                if let Some(idx) = delete_idx { self.selected_layer = Some(idx); self.delete_selected(); }
            });
    }''')

s = replace_fn(s, 'center_canvas', r'''    fn center_canvas(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::from_rgb(216, 216, 216)))
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{} × {}", self.project.canvas.width, self.project.canvas.height)).size(10.0).color(Color32::from_gray(82)));
                    let style = self.selected_layer.and_then(|i| self.project.layers.get(i)).map(|l| l.animation.style.label()).unwrap_or("Scanner");
                    ui.label(RichText::new(style).size(10.0).color(Color32::from_gray(82)));
                    if self.playhead >= 0.999 { ui.label(RichText::new("Done").size(9.0).color(Color32::from_rgb(22, 130, 65)).strong()); }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(self.selected_image().is_some(), egui::Button::new("Slicer")).clicked() {
                            self.slicer = SlicerState { open: true, ..Default::default() };
                        }
                    });
                });
                ui.add_space(5.0);

                let canvas_h = (ui.available_height() - 48.0).max(80.0);
                let result = ui.allocate_ui(Vec2::new(ui.available_width(), canvas_h), |ui| {
                    ui.vertical_centered(|ui| render::paint_project(ui, &self.project, &self.assets, self.selected_layer, self.playhead))
                        .inner
                }).inner;

                if let (Some(idx), Some(delta)) = (self.selected_layer, result.selected_drag_delta_canvas) {
                    if let Some(layer) = self.project.layers.get_mut(idx) { layer.transform.x += delta.x; layer.transform.y += delta.y; }
                }
                if let (Some(idx), Some(resize)) = (self.selected_layer, result.selected_resize_delta) {
                    if let Some(layer) = self.project.layers.get_mut(idx) {
                        const MIN_SIZE: f32 = 12.0;
                        let t = &mut layer.transform;
                        match resize.handle {
                            render::ResizeHandle::TopLeft => {
                                let new_w = (t.w - resize.delta_canvas.x).max(MIN_SIZE);
                                let new_h = (t.h - resize.delta_canvas.y).max(MIN_SIZE);
                                t.x += t.w - new_w; t.y += t.h - new_h; t.w = new_w; t.h = new_h;
                            }
                            render::ResizeHandle::TopRight => {
                                let new_h = (t.h - resize.delta_canvas.y).max(MIN_SIZE);
                                t.y += t.h - new_h; t.w = (t.w + resize.delta_canvas.x).max(MIN_SIZE); t.h = new_h;
                            }
                            render::ResizeHandle::BottomLeft => {
                                let new_w = (t.w - resize.delta_canvas.x).max(MIN_SIZE);
                                t.x += t.w - new_w; t.w = new_w; t.h = (t.h + resize.delta_canvas.y).max(MIN_SIZE);
                            }
                            render::ResizeHandle::BottomRight => {
                                t.w = (t.w + resize.delta_canvas.x).max(MIN_SIZE); t.h = (t.h + resize.delta_canvas.y).max(MIN_SIZE);
                            }
                        }
                    }
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.small_button("⟳ Restart").clicked() { self.playhead = 0.0; self.playing = true; self.last_tick = Instant::now(); }
                    let label = if self.playing { "⏸ Pause" } else { "▶ Play" };
                    if ui.small_button(label).clicked() {
                        if self.playhead >= 1.0 { self.playhead = 0.0; }
                        self.playing = !self.playing;
                        self.last_tick = Instant::now();
                    }
                    ui.add(egui::Slider::new(&mut self.playhead, 0.0..=1.0).show_value(false));
                    ui.label(RichText::new(format!("{}%", (self.playhead * 100.0).round() as u32)).size(10.0));
                });
            });
    }''')

s = replace_fn(s, 'bottom_bar', r'''    fn bottom_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_bar")
            .exact_height(112.0)
            .frame(egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0_f32, Color32::from_black_alpha(20))))
            .show(ctx, |ui| {
                ui.add_space(5.0);
                ui.columns(3, |cols| {
                    cols[0].label(RichText::new("Hand Style").size(10.0).strong());
                    if let Some(idx) = self.selected_layer {
                        if let Some(layer) = self.project.layers.get_mut(idx) {
                            cols[0].horizontal_wrapped(|ui| {
                                for hand in HandStyle::ALL {
                                    if ui.selectable_label(layer.animation.hand == hand, hand.label()).clicked() { layer.animation.hand = hand; }
                                }
                            });
                        }
                    } else {
                        cols[0].label(RichText::new("Select a layer").size(10.0).color(Color32::from_gray(145)));
                    }

                    cols[1].label(RichText::new("Speed").size(10.0).strong());
                    if let Some(idx) = self.selected_layer {
                        if let Some(layer) = self.project.layers.get_mut(idx) {
                            cols[1].add(egui::Slider::new(&mut layer.animation.speed, 1.0..=100.0).text("Reveal"));
                            cols[1].add(egui::Slider::new(&mut layer.animation.hand_speed, 1.0..=30.0).text("Hand"));
                        }
                    } else {
                        cols[1].label(RichText::new("Reveal —   Hand —").size(10.0).color(Color32::from_gray(145)));
                    }

                    self.canvas_controls(&mut cols[2]);
                });

                if let Some(job) = &self.export_job {
                    ui.separator();
                    ui.horizontal(|ui| {
                        let p = job.progress();
                        ui.add(egui::ProgressBar::new(p).desired_width(260.0).text(format!("Export {:>3}%", (p * 100.0).round() as u32)));
                        if ui.button("Cancel export").clicked() { job.cancel(); }
                        ui.label(RichText::new(job.path.display().to_string()).size(9.0).color(Color32::from_gray(130)));
                    });
                }
            });
    }''')

s = replace_fn(s, 'canvas_controls', r'''    fn canvas_controls(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Canvas Size").size(10.0).strong());
        ui.horizontal(|ui| {
            ui.label(RichText::new("Aspect Ratio").size(9.0).color(Color32::from_gray(130)));
            if ui.small_button("16:9").clicked() { self.project.canvas.width = 1280; self.project.canvas.height = 720; }
            if ui.small_button("9:16").clicked() { self.project.canvas.width = 720; self.project.canvas.height = 1280; }
            if ui.small_button("1:1").clicked() { self.project.canvas.width = 720; self.project.canvas.height = 720; }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Resolution").size(9.0).color(Color32::from_gray(130)));
            if ui.small_button("720p").clicked() { set_canvas_resolution(&mut self.project, 720); }
            if ui.small_button("1080p").clicked() { set_canvas_resolution(&mut self.project, 1080); }
            if ui.small_button("1440p").clicked() { set_canvas_resolution(&mut self.project, 1440); }
        });
        ui.label(RichText::new(format!("{} × {}", self.project.canvas.width, self.project.canvas.height)).size(9.0).color(Color32::from_gray(100)));
        ui.checkbox(&mut self.project.hand_glide_enabled, "Smooth hand movement");
    }''')

s = replace_fn(s, 'export_controls', r'''    fn export_controls(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Export").strong());
        ui.horizontal(|ui| {
            if ui.small_button("720p").clicked() { set_export_resolution(&mut self.project, 720); }
            if ui.small_button("1080p").clicked() { set_export_resolution(&mut self.project, 1080); }
            if ui.small_button("1440p").clicked() { set_export_resolution(&mut self.project, 1440); }
        });
        ui.label(format!("{} × {}", self.project.export.width, self.project.export.height));
        ui.add(egui::Slider::new(&mut self.project.export.fps, 12..=60).text("FPS"));
        egui::ComboBox::from_label("Quality").selected_text(format!("{:?}", self.project.export.quality)).show_ui(ui, |ui| {
            ui.selectable_value(&mut self.project.export.quality, ExportQuality::High, "High");
            ui.selectable_value(&mut self.project.export.quality, ExportQuality::Medium, "Medium");
            ui.selectable_value(&mut self.project.export.quality, ExportQuality::Low, "Low");
        });
        egui::ComboBox::from_label("Format").selected_text(match self.project.export.format { ExportFormat::Mp4 => "MP4", ExportFormat::WebM => "WebM" }).show_ui(ui, |ui| {
            ui.selectable_value(&mut self.project.export.format, ExportFormat::Mp4, "MP4");
            ui.selectable_value(&mut self.project.export.format, ExportFormat::WebM, "WebM");
        });
    }''')

# Clean warnings reported by the user while touching the UI.
s = s.replace('Stroke::new(1.0, Color32::LIGHT_BLUE)', 'Stroke::new(1.0_f32, Color32::LIGHT_BLUE)')
s = s.replace('Stroke::new(2.0, Color32::YELLOW)', 'Stroke::new(2.0_f32, Color32::YELLOW)')
s = s.replace('Stroke::new(2.0, Color32::LIGHT_GREEN)', 'Stroke::new(2.0_f32, Color32::LIGHT_GREEN)')

if 'fn configure_inkplainer_style' not in s:
    marker = '\nfn uses_chunks(style: AnimationStyle) -> bool {'
    helpers = r'''
fn configure_inkplainer_style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = Color32::WHITE;
    visuals.window_fill = Color32::WHITE;
    visuals.extreme_bg_color = Color32::from_rgb(245, 245, 245);
    visuals.faint_bg_color = Color32::from_rgb(250, 250, 250);
    visuals.code_bg_color = Color32::from_rgb(245, 245, 245);
    visuals.selection.bg_fill = Color32::from_rgb(26, 26, 26);
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(250, 250, 250);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(250, 250, 250);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(240, 240, 240);
    visuals.widgets.active.bg_fill = Color32::from_rgb(228, 228, 228);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_black_alpha(18));
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(7.0, 6.0);
    style.spacing.button_padding = Vec2::new(9.0, 5.0);
    style.spacing.interact_size.y = 26.0;
    ctx.set_style(style);
}

fn set_canvas_resolution(project: &mut Project, base: u32) {
    let portrait = project.canvas.height > project.canvas.width;
    let square = ((project.canvas.width as i64 - project.canvas.height as i64).abs() as f32 / project.canvas.width.max(project.canvas.height).max(1) as f32) < 0.05;
    let (new_w, new_h) = if square {
        (base, base)
    } else if portrait {
        (base, (base as f32 * 16.0 / 9.0).round() as u32)
    } else {
        ((base as f32 * 16.0 / 9.0).round() as u32, base)
    };
    let sx = new_w as f32 / project.canvas.width.max(1) as f32;
    let sy = new_h as f32 / project.canvas.height.max(1) as f32;
    for layer in &mut project.layers {
        layer.transform.x *= sx;
        layer.transform.y *= sy;
        layer.transform.w *= sx;
        layer.transform.h *= sy;
        if let LayerContent::Text(text) = &mut layer.content {
            text.size *= (sx + sy) * 0.5;
            text.letter_spacing *= sx;
        }
        layer.animation.outline_thickness *= (sx + sy) * 0.5;
    }
    project.canvas.width = new_w;
    project.canvas.height = new_h;
    project.export.width = new_w;
    project.export.height = new_h;
}
'''
    if marker not in s:
        raise RuntimeError('helper insertion marker missing')
    s = s.replace(marker, '\n' + helpers + marker, 1)

p.write_text(s, encoding='utf-8')

# Other warning cleanups.
worker = Path('src/export_worker.rs')
w = worker.read_text(encoding='utf-8')
w = w.replace('\n    pub fn is_cancelled(&self) -> bool {\n        self.cancel.load(Ordering::Relaxed)\n    }\n', '\n')
worker.write_text(w, encoding='utf-8')

render = Path('src/render.rs')
r = render.read_text(encoding='utf-8')
r = r.replace('    pub rect: Rect,\n', '')
r = r.replace('CanvasResult { rect, selected_drag_delta_canvas, selected_resize_delta }', 'CanvasResult { selected_drag_delta_canvas, selected_resize_delta }')
render.write_text(r, encoding='utf-8')
