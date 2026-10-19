use eframe::egui::{self, Color32, Pos2, RichText, Stroke, Vec2};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::assets::RuntimeAssets;
use crate::export;
use crate::model::{
    AnimationSettings, AnimationStyle, Background, ColoringStyle, DetectionAlgorithm,
    ExportFormat, ExportQuality, HandStyle, Layer, LayerContent, Preset, Project, RevealStyle,
    StrokeStyle, TextAlign, TextDirection, TextDrawStyle,
};
use crate::project_io;
use crate::render;
use crate::slicer::{self, NormalizedRect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SliceMode { Grid, Rectangles, Freehand }

struct SlicerState {
    open: bool,
    mode: SliceMode,
    cols: u32,
    rows: u32,
    rects: Vec<NormalizedRect>,
    rect_editor: [f32; 4],
    rect_drag_start: Option<[f32; 2]>,
    freehand: Vec<[f32; 2]>,
}

impl Default for SlicerState {
    fn default() -> Self {
        Self {
            open: false,
            mode: SliceMode::Grid,
            cols: 2,
            rows: 2,
            rects: Vec::new(),
            rect_editor: [0.1, 0.1, 0.4, 0.4],
            rect_drag_start: None,
            freehand: Vec::new(),
        }
    }
}

pub struct InkplainerApp {
    pub project: Project,
    pub assets: RuntimeAssets,
    pub selected_layer: Option<usize>,
    pub project_path: Option<PathBuf>,
    pub playhead: f32,
    pub playing: bool,
    pub status: String,
    last_tick: Instant,
    last_autosave: Instant,
    last_autosave_fingerprint: String,
    undo_stack: Vec<Project>,
    redo_stack: Vec<Project>,
    suppress_history_once: bool,
    slicer: SlicerState,
    autosave_available: bool,
}

impl InkplainerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            project: Project::default(),
            assets: RuntimeAssets::default(),
            selected_layer: None,
            project_path: None,
            playhead: 1.0,
            playing: false,
            status: "Ready".to_owned(),
            last_tick: Instant::now(),
            last_autosave: Instant::now(),
            last_autosave_fingerprint: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            suppress_history_once: false,
            slicer: SlicerState::default(),
            autosave_available: project_io::autosave_path().exists(),
        }
    }

    fn new_project(&mut self) {
        self.project = Project::default();
        self.assets.clear();
        self.selected_layer = None;
        self.project_path = None;
        self.playhead = 1.0;
        self.playing = false;
        self.status = "New project".to_owned();
    }

    fn import_images(&mut self, ctx: &egui::Context) {
        let Some(paths) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp"])
            .pick_files()
        else { return; };

        for path in paths {
            match image::image_dimensions(&path) {
                Ok((w, h)) => {
                    let layer = Layer::new_image(path.clone(), self.project.canvas.width, self.project.canvas.height, w, h);
                    let id = layer.id.clone();
                    self.project.layers.push(layer);
                    match self.assets.load_image(ctx, &id, &path) {
                        Ok(_) => {
                            self.selected_layer = Some(self.project.layers.len() - 1);
                            self.playhead = 1.0;
                            self.status = format!("Imported {}", path.display());
                        }
                        Err(err) => self.status = format!("Import failed: {err:#}"),
                    }
                }
                Err(err) => self.status = format!("Cannot inspect {}: {err}", path.display()),
            }
        }
    }

    fn add_text(&mut self) {
        self.project.layers.push(Layer::new_text(self.project.canvas.width, self.project.canvas.height));
        self.selected_layer = Some(self.project.layers.len() - 1);
        self.playhead = 1.0;
        self.status = "Text layer added".to_owned();
    }

    fn save_project_as(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Inkplainer Project", &["json"])
            .set_file_name("project.inkplainer.json")
            .save_file()
        else { return; };
        match project_io::save_project(&self.project, &path) {
            Ok(()) => {
                self.project_path = Some(path.clone());
                self.status = format!("Saved {}", path.display());
            }
            Err(err) => self.status = format!("Save failed: {err:#}"),
        }
    }

    fn save_project(&mut self) {
        if let Some(path) = self.project_path.clone() {
            match project_io::save_project(&self.project, &path) {
                Ok(()) => self.status = format!("Saved {}", path.display()),
                Err(err) => self.status = format!("Save failed: {err:#}"),
            }
        } else { self.save_project_as(); }
    }

    fn open_project(&mut self, ctx: &egui::Context) {
        let Some(path) = rfd::FileDialog::new().add_filter("Inkplainer Project", &["json"]).pick_file() else { return; };
        match project_io::load_project(&path) {
            Ok(project) => {
                self.project = project;
                let errors = self.assets.load_project(ctx, &self.project);
                self.project_path = Some(path.clone());
                self.selected_layer = None;
                self.playhead = 1.0;
                self.playing = false;
                self.status = if errors.is_empty() { format!("Opened {}", path.display()) } else { format!("Opened with missing assets: {}", errors.join(" | ")) };
            }
            Err(err) => self.status = format!("Open failed: {err:#}"),
        }
    }

    fn restore_autosave(&mut self, ctx: &egui::Context) {
        match project_io::load_autosave() {
            Ok(Some(project)) => {
                self.project = project;
                let errors = self.assets.load_project(ctx, &self.project);
                self.selected_layer = None;
                self.project_path = None;
                self.playhead = 1.0;
                self.status = if errors.is_empty() { "Autosave restored".to_owned() } else { format!("Autosave restored with missing assets: {}", errors.join(" | ")) };
            }
            Ok(None) => self.status = "No autosave found".to_owned(),
            Err(err) => self.status = format!("Autosave restore failed: {err:#}"),
        }
    }

    fn export_png(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("PNG image", &["png"]).set_file_name("inkplainer-frame.png").save_file() else { return; };
        match export::export_png(&self.project, &self.assets, &path) {
            Ok(()) => self.status = format!("PNG exported: {}", path.display()),
            Err(err) => self.status = format!("PNG export failed: {err:#}"),
        }
    }

    fn export_video(&mut self, webm: bool) {
        let (ext, label) = if webm { ("webm", "WebM video") } else { ("mp4", "MP4 video") };
        let Some(path) = rfd::FileDialog::new().add_filter(label, &[ext]).set_file_name(format!("inkplainer-animation.{ext}")).save_file() else { return; };
        self.status = format!("Rendering {}…", ext.to_uppercase());
        let fps = self.project.export.fps;
        let result = if webm { export::export_webm(&self.project, &self.assets, &path, fps) } else { export::export_mp4(&self.project, &self.assets, &path, fps) };
        match result {
            Ok(()) => self.status = format!("{} exported: {}", ext.to_uppercase(), path.display()),
            Err(err) => self.status = format!("{} export failed: {err:#}", ext.to_uppercase()),
        }
    }

    fn update_playback(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        let dt = (now - self.last_tick).as_secs_f32();
        self.last_tick = now;
        if self.playing {
            self.playhead += dt / self.project.duration_seconds.max(0.5);
            if self.playhead >= 1.0 { self.playhead = 1.0; self.playing = false; }
            ctx.request_repaint();
        }
    }

    fn maybe_autosave(&mut self) {
        if !self.project.autosave_enabled || self.last_autosave.elapsed() < Duration::from_secs(3) { return; }
        self.last_autosave = Instant::now();
        let fp = fingerprint(&self.project);
        if fp == self.last_autosave_fingerprint { return; }
        if project_io::save_autosave(&self.project).is_ok() {
            self.last_autosave_fingerprint = fp;
            self.autosave_available = true;
        }
    }

    fn delete_selected(&mut self) {
        let Some(idx) = self.selected_layer else { return; };
        if idx >= self.project.layers.len() { return; }
        let id = self.project.layers[idx].id.clone();
        self.project.layers.remove(idx);
        self.assets.images.remove(&id);
        self.selected_layer = None;
        self.status = "Layer deleted".to_owned();
    }

    fn undo(&mut self, ctx: &egui::Context) {
        let Some(previous) = self.undo_stack.pop() else { return; };
        self.redo_stack.push(self.project.clone());
        self.project = previous;
        self.assets.load_project(ctx, &self.project);
        self.selected_layer = None;
        self.suppress_history_once = true;
        self.status = "Undo".to_owned();
    }

    fn redo(&mut self, ctx: &egui::Context) {
        let Some(next) = self.redo_stack.pop() else { return; };
        self.undo_stack.push(self.project.clone());
        self.project = next;
        self.assets.load_project(ctx, &self.project);
        self.selected_layer = None;
        self.suppress_history_once = true;
        self.status = "Redo".to_owned();
    }

    fn process_shortcuts(&mut self, ctx: &egui::Context) {
        let (space, delete, backspace, undo, redo, save, open, escape) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Space),
                i.key_pressed(egui::Key::Delete),
                i.key_pressed(egui::Key::Backspace),
                i.key_pressed(egui::Key::Z) && i.modifiers.ctrl && !i.modifiers.shift,
                (i.key_pressed(egui::Key::Y) && i.modifiers.ctrl) || (i.key_pressed(egui::Key::Z) && i.modifiers.ctrl && i.modifiers.shift),
                i.key_pressed(egui::Key::S) && i.modifiers.ctrl,
                i.key_pressed(egui::Key::O) && i.modifiers.ctrl,
                i.key_pressed(egui::Key::Escape),
            )
        });
        if undo { self.undo(ctx); return; }
        if redo { self.redo(ctx); return; }
        if save { self.save_project(); }
        if open { self.open_project(ctx); }
        if escape { self.slicer.open = false; }
        if delete || backspace { self.delete_selected(); }
        if space && !ctx.wants_keyboard_input() {
            if self.playhead >= 1.0 { self.playhead = 0.0; }
            self.playing = !self.playing;
            self.last_tick = Instant::now();
        }
    }

    fn top_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Inkplainer RustApp");
                ui.separator();
                if ui.button("New").clicked() { self.new_project(); }
                if ui.button("Open").clicked() { self.open_project(ctx); }
                if ui.button("Save").clicked() { self.save_project(); }
                if ui.button("Save As").clicked() { self.save_project_as(); }
                if self.autosave_available && ui.button("Restore Autosave").clicked() { self.restore_autosave(ctx); }
                ui.separator();
                if ui.add_enabled(!self.undo_stack.is_empty(), egui::Button::new("Undo")).clicked() { self.undo(ctx); }
                if ui.add_enabled(!self.redo_stack.is_empty(), egui::Button::new("Redo")).clicked() { self.redo(ctx); }
                ui.separator();
                if ui.button("Import image").clicked() { self.import_images(ctx); }
                if ui.button("Add text").clicked() { self.add_text(); }
                if ui.add_enabled(self.selected_image().is_some(), egui::Button::new("Slice")).clicked() {
                    self.slicer = SlicerState { open: true, ..Default::default() };
                }
                ui.separator();
                if ui.button("Export PNG").clicked() { self.export_png(); }
                if ui.button("Export MP4").clicked() { self.export_video(false); }
                if ui.button("Export WebM").clicked() { self.export_video(true); }
            });
        });
    }

    fn selected_image(&self) -> Option<usize> {
        let idx = self.selected_layer?;
        matches!(self.project.layers.get(idx)?.content, LayerContent::Image { .. }).then_some(idx)
    }

    fn layers_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("layers_panel").resizable(true).default_width(240.0).show(ctx, |ui| {
            ui.heading("Layers");
            ui.small("Top row is visually on top");
            ui.separator();
            let mut delete_idx = None;
            let mut move_up = None;
            let mut move_down = None;
            for idx in (0..self.project.layers.len()).rev() {
                let selected = self.selected_layer == Some(idx);
                let layer = &mut self.project.layers[idx];
                ui.horizontal(|ui| {
                    ui.checkbox(&mut layer.visible, "");
                    if ui.selectable_label(selected, &layer.name).clicked() { self.selected_layer = Some(idx); }
                    if let Some(order) = layer.anim_order { ui.small(format!("#{order}")); }
                });
                if selected {
                    ui.horizontal(|ui| {
                        if ui.small_button("↑").clicked() { move_up = Some(idx); }
                        if ui.small_button("↓").clicked() { move_down = Some(idx); }
                        if ui.small_button("Delete").clicked() { delete_idx = Some(idx); }
                    });
                }
                ui.separator();
            }
            if let Some(idx) = move_up {
                if idx + 1 < self.project.layers.len() { self.project.layers.swap(idx, idx + 1); self.selected_layer = Some(idx + 1); }
            }
            if let Some(idx) = move_down {
                if idx > 0 { self.project.layers.swap(idx, idx - 1); self.selected_layer = Some(idx - 1); }
            }
            if let Some(idx) = delete_idx { self.selected_layer = Some(idx); self.delete_selected(); }
        });
    }

    fn settings_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("settings_panel").resizable(true).default_width(330.0).show(ctx, |ui| {
            ui.heading("Properties");
            ui.separator();
            self.canvas_controls(ui);
            ui.separator();
            self.export_controls(ui);
            ui.separator();
            self.preset_controls(ui);
            ui.separator();

            let Some(idx) = self.selected_layer else { ui.label("Select a layer to edit it."); return; };
            if idx >= self.project.layers.len() { self.selected_layer = None; return; }
            let layer = &mut self.project.layers[idx];
            ui.heading("Layer");
            ui.text_edit_singleline(&mut layer.name);
            ui.checkbox(&mut layer.visible, "Visible");
            ui.add(egui::Slider::new(&mut layer.opacity, 0.0..=1.0).text("Opacity"));
            ui.horizontal(|ui| {
                ui.label("Animation order");
                let mut order = layer.anim_order.unwrap_or(1);
                let mut enabled = layer.anim_order.is_some();
                if ui.checkbox(&mut enabled, "").changed() { layer.anim_order = enabled.then_some(order.max(1)); }
                if enabled && ui.add(egui::DragValue::new(&mut order).range(1..=999)).changed() { layer.anim_order = Some(order.max(1)); }
            });
            let mut group = layer.group.clone().unwrap_or_default();
            if ui.text_edit_singleline(&mut group).changed() { layer.group = if group.trim().is_empty() { None } else { Some(group) }; }

            ui.collapsing("Position & size", |ui| {
                ui.horizontal(|ui| { ui.label("X"); ui.add(egui::DragValue::new(&mut layer.transform.x)); ui.label("Y"); ui.add(egui::DragValue::new(&mut layer.transform.y)); });
                ui.horizontal(|ui| { ui.label("W"); ui.add(egui::DragValue::new(&mut layer.transform.w).range(1.0..=10000.0)); ui.label("H"); ui.add(egui::DragValue::new(&mut layer.transform.h).range(1.0..=10000.0)); });
            });

            ui.separator();
            ui.heading("Animation");
            egui::ComboBox::from_label("Style").selected_text(layer.animation.style.label()).show_ui(ui, |ui| {
                for style in AnimationStyle::ALL { ui.selectable_value(&mut layer.animation.style, style, style.label()); }
            });
            ui.add(egui::Slider::new(&mut layer.animation.speed, 1.0..=100.0).text("Reveal speed"));
            ui.add(egui::Slider::new(&mut layer.animation.hand_speed, 1.0..=30.0).text("Hand speed"));
            egui::ComboBox::from_label("Hand").selected_text(layer.animation.hand.label()).show_ui(ui, |ui| {
                for hand in HandStyle::ALL { ui.selectable_value(&mut layer.animation.hand, hand, hand.label()); }
            });
            if uses_chunks(layer.animation.style) { ui.add(egui::Slider::new(&mut layer.animation.chunks, 6..=80).text("Chunks")); }
            if layer.animation.style == AnimationStyle::Scanner { ui.checkbox(&mut layer.animation.zigzag, "Zigzag"); }

            if uses_drawing(layer.animation.style) {
                ui.separator();
                ui.strong("Drawing");
                ui.add(egui::Slider::new(&mut layer.animation.outline_detection, 0..=100).text("Outline detection"));
                egui::ComboBox::from_label("Detection algorithm").selected_text(layer.animation.detection_algorithm.label()).show_ui(ui, |ui| {
                    for a in DetectionAlgorithm::ALL { ui.selectable_value(&mut layer.animation.detection_algorithm, a, a.label()); }
                });
                egui::ComboBox::from_label("Stroke style").selected_text(layer.animation.stroke_style.label()).show_ui(ui, |ui| {
                    for s in StrokeStyle::ALL { ui.selectable_value(&mut layer.animation.stroke_style, s, s.label()); }
                });
                ui.add(egui::Slider::new(&mut layer.animation.outline_thickness, 0.5..=8.0).text("Outline thickness"));
                ui.add(egui::Slider::new(&mut layer.animation.outline_opacity, 0.0..=1.0).text("Outline opacity"));
                ui.checkbox(&mut layer.animation.outline_visible, "Outline visible");
                let mut color = Color32::from_rgba_unmultiplied(layer.animation.outline_color[0], layer.animation.outline_color[1], layer.animation.outline_color[2], layer.animation.outline_color[3]);
                if ui.color_edit_button_srgba(&mut color).changed() { layer.animation.outline_color = [color.r(), color.g(), color.b(), color.a()]; }
                if ui.checkbox(&mut layer.animation.color_region_outlines, "Color region outlines").changed() && layer.animation.color_region_outlines { layer.animation.real_image_edges = false; }
                if ui.checkbox(&mut layer.animation.real_image_edges, "Real image edges").changed() && layer.animation.real_image_edges { layer.animation.color_region_outlines = false; }
            }

            if matches!(layer.animation.style, AnimationStyle::OutlineFill | AnimationStyle::IllustFill) {
                ui.checkbox(&mut layer.animation.reveal_original, "Reveal original image");
                egui::ComboBox::from_label("Reveal style").selected_text(layer.animation.reveal_style.label()).show_ui(ui, |ui| {
                    for r in RevealStyle::ALL { ui.selectable_value(&mut layer.animation.reveal_style, r, r.label()); }
                });
            }
            if layer.animation.style == AnimationStyle::IllustFill {
                egui::ComboBox::from_label("Coloring").selected_text(layer.animation.coloring_style.label()).show_ui(ui, |ui| {
                    for c in ColoringStyle::ALL { ui.selectable_value(&mut layer.animation.coloring_style, c, c.label()); }
                });
            }
            if layer.animation.style == AnimationStyle::TextDraw {
                egui::ComboBox::from_label("Text direction").selected_text(layer.animation.text_direction.label()).show_ui(ui, |ui| {
                    for d in TextDirection::ALL { ui.selectable_value(&mut layer.animation.text_direction, d, d.label()); }
                });
                egui::ComboBox::from_label("Text draw style").selected_text(layer.animation.text_draw_style.label()).show_ui(ui, |ui| {
                    for s in TextDrawStyle::ALL { ui.selectable_value(&mut layer.animation.text_draw_style, s, s.label()); }
                });
            }

            if let LayerContent::Text(text) = &mut layer.content {
                ui.separator(); ui.heading("Text");
                ui.text_edit_multiline(&mut text.text);
                ui.horizontal(|ui| { ui.label("Font"); ui.text_edit_singleline(&mut text.font_family); });
                ui.add(egui::Slider::new(&mut text.size, 10.0..=400.0).text("Font size"));
                ui.horizontal(|ui| { ui.checkbox(&mut text.bold, "Bold"); ui.checkbox(&mut text.italic, "Italic"); });
                egui::ComboBox::from_label("Align").selected_text(match text.align { TextAlign::Left => "Left", TextAlign::Center => "Center", TextAlign::Right => "Right" }).show_ui(ui, |ui| {
                    ui.selectable_value(&mut text.align, TextAlign::Left, "Left"); ui.selectable_value(&mut text.align, TextAlign::Center, "Center"); ui.selectable_value(&mut text.align, TextAlign::Right, "Right");
                });
                ui.add(egui::Slider::new(&mut text.line_height, 0.8..=2.5).text("Line height"));
                ui.add(egui::Slider::new(&mut text.letter_spacing, -5.0..=30.0).text("Letter spacing"));
                let mut c = Color32::from_rgba_unmultiplied(text.color[0], text.color[1], text.color[2], text.color[3]);
                if ui.color_edit_button_srgba(&mut c).changed() { text.color = [c.r(), c.g(), c.b(), c.a()]; }
            }
        });
    }

    fn canvas_controls(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Canvas", |ui| {
            ui.horizontal(|ui| {
                ui.label("Size");
                ui.add(egui::DragValue::new(&mut self.project.canvas.width).range(320..=3840));
                ui.label("×");
                ui.add(egui::DragValue::new(&mut self.project.canvas.height).range(240..=3840));
            });
            ui.horizontal(|ui| {
                if ui.small_button("16:9").clicked() { self.project.canvas.width = 1280; self.project.canvas.height = 720; }
                if ui.small_button("1:1").clicked() { self.project.canvas.width = 1080; self.project.canvas.height = 1080; }
                if ui.small_button("9:16").clicked() { self.project.canvas.width = 720; self.project.canvas.height = 1280; }
            });
            egui::ComboBox::from_label("Background").selected_text(background_label(&self.project.canvas.background)).show_ui(ui, |ui| {
                for (value, label) in background_choices() { bg_choice(ui, &mut self.project.canvas.background, value, label); }
            });
            if let Background::Solid(c) = &mut self.project.canvas.background {
                let mut color = Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]);
                if ui.color_edit_button_srgba(&mut color).changed() { *c = [color.r(), color.g(), color.b(), color.a()]; }
            }
            ui.add(egui::Slider::new(&mut self.project.duration_seconds, 0.5..=120.0).text("Duration (s)"));
            ui.checkbox(&mut self.project.hand_glide_enabled, "Smooth hand movement");
            ui.checkbox(&mut self.project.autosave_enabled, "Auto-save");
        });
    }

    fn export_controls(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Export", |ui| {
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
            egui::ComboBox::from_label("Default format").selected_text(match self.project.export.format { ExportFormat::Mp4 => "MP4", ExportFormat::WebM => "WebM" }).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.project.export.format, ExportFormat::Mp4, "MP4");
                ui.selectable_value(&mut self.project.export.format, ExportFormat::WebM, "WebM");
            });
        });
    }

    fn preset_controls(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("Presets", |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.small_button("Quick Reveal").clicked() { self.apply_builtin_preset(0); }
                if ui.small_button("Sketch Artist").clicked() { self.apply_builtin_preset(1); }
                if ui.small_button("Blueprint").clicked() { self.apply_builtin_preset(2); }
                if ui.small_button("Illustrated").clicked() { self.apply_builtin_preset(3); }
            });
            let mut apply = None;
            for (i, preset) in self.project.custom_presets.iter().enumerate() {
                if ui.small_button(&preset.name).clicked() { apply = Some(i); }
            }
            if let Some(i) = apply {
                if let (Some(idx), Some(preset)) = (self.selected_layer, self.project.custom_presets.get(i).cloned()) {
                    if let Some(layer) = self.project.layers.get_mut(idx) { layer.animation = preset.animation; }
                }
            }
            if self.project.custom_presets.len() < 6 && ui.small_button("+ Save current settings").clicked() {
                if let Some(idx) = self.selected_layer {
                    if let Some(layer) = self.project.layers.get(idx) {
                        self.project.custom_presets.push(Preset { name: format!("Custom {}", self.project.custom_presets.len() + 1), animation: layer.animation.clone() });
                    }
                }
            }
        });
    }

    fn apply_builtin_preset(&mut self, which: usize) {
        let Some(idx) = self.selected_layer else { return; };
        let Some(layer) = self.project.layers.get_mut(idx) else { return; };
        let mut a = AnimationSettings::default();
        match which {
            0 => { a.style = AnimationStyle::Scanner; a.hand = HandStyle::Ghost; a.speed = 75.0; }
            1 => { a.style = AnimationStyle::Contour; a.hand = HandStyle::Hand1; a.speed = 25.0; a.stroke_style = StrokeStyle::Charcoal; }
            2 => { a.style = AnimationStyle::OutlineOnly; a.hand = HandStyle::Ghost; a.stroke_style = StrokeStyle::Blueprint; a.outline_color = [118, 210, 255, 255]; }
            3 => { a.style = AnimationStyle::IllustFill; a.hand = HandStyle::Hand1; a.coloring_style = ColoringStyle::Filled; }
            _ => return,
        }
        layer.animation = a;
    }

    fn center_canvas(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let result = render::paint_project(ui, &self.project, &self.assets, self.selected_layer, self.playhead);
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
                                t.x += t.w - new_w;
                                t.y += t.h - new_h;
                                t.w = new_w;
                                t.h = new_h;
                            }
                            render::ResizeHandle::TopRight => {
                                let new_h = (t.h - resize.delta_canvas.y).max(MIN_SIZE);
                                t.y += t.h - new_h;
                                t.w = (t.w + resize.delta_canvas.x).max(MIN_SIZE);
                                t.h = new_h;
                            }
                            render::ResizeHandle::BottomLeft => {
                                let new_w = (t.w - resize.delta_canvas.x).max(MIN_SIZE);
                                t.x += t.w - new_w;
                                t.w = new_w;
                                t.h = (t.h + resize.delta_canvas.y).max(MIN_SIZE);
                            }
                            render::ResizeHandle::BottomRight => {
                                t.w = (t.w + resize.delta_canvas.x).max(MIN_SIZE);
                                t.h = (t.h + resize.delta_canvas.y).max(MIN_SIZE);
                            }
                        }
                    }
                }
            });
        });
    }

    fn bottom_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let label = if self.playing { "Pause" } else { "Play" };
                if ui.button(label).clicked() { if self.playhead >= 1.0 { self.playhead = 0.0; } self.playing = !self.playing; self.last_tick = Instant::now(); }
                if ui.button("Restart").clicked() { self.playhead = 0.0; self.playing = true; self.last_tick = Instant::now(); }
                ui.add(egui::Slider::new(&mut self.playhead, 0.0..=1.0).show_value(false).text("Timeline"));
                ui.label(format!("{:>3}%", (self.playhead * 100.0).round() as u32));
                ui.separator();
                ui.label(RichText::new(&self.status).small());
            });
        });
    }

    fn slicer_window(&mut self, ctx: &egui::Context) {
        if !self.slicer.open { return; }
        let Some(idx) = self.selected_image() else { self.slicer.open = false; return; };
        let id = self.project.layers[idx].id.clone();
        let Some(runtime) = self.assets.images.get(&id) else { self.slicer.open = false; return; };
        let texture = runtime.texture.id();
        let image_aspect = runtime.rgba.width().max(1) as f32 / runtime.rgba.height().max(1) as f32;
        let mut apply = false;
        let mut cancel = false;
        egui::Window::new("Slicer").open(&mut self.slicer.open).resizable(true).default_size([700.0, 650.0]).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.slicer.mode, SliceMode::Grid, "Grid");
                ui.selectable_value(&mut self.slicer.mode, SliceMode::Rectangles, "Rectangles");
                ui.selectable_value(&mut self.slicer.mode, SliceMode::Freehand, "Freehand");
            });
            match self.slicer.mode {
                SliceMode::Grid => {
                    ui.horizontal(|ui| { ui.add(egui::Slider::new(&mut self.slicer.cols, 1..=8).text("Columns")); ui.add(egui::Slider::new(&mut self.slicer.rows, 1..=8).text("Rows")); });
                }
                SliceMode::Rectangles => {
                    ui.label("Drag directly on the preview to add regions, or enter normalized X/Y/W/H values (0–1).");
                    ui.horizontal(|ui| {
                        for (i, label) in ["X", "Y", "W", "H"].iter().enumerate() { ui.label(*label); ui.add(egui::DragValue::new(&mut self.slicer.rect_editor[i]).speed(0.01).range(0.0..=1.0)); }
                    });
                    if ui.button("Add numeric region").clicked() {
                        self.slicer.rects.push(NormalizedRect { x: self.slicer.rect_editor[0], y: self.slicer.rect_editor[1], w: self.slicer.rect_editor[2], h: self.slicer.rect_editor[3] }.clamped());
                    }
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("{} region(s)", self.slicer.rects.len()));
                        if ui.small_button("Clear").clicked() { self.slicer.rects.clear(); self.slicer.rect_drag_start = None; }
                    });
                }
                SliceMode::Freehand => {
                    ui.label("Drag around the subject to draw a freehand closed polygon.");
                    if ui.small_button("Clear path").clicked() { self.slicer.freehand.clear(); }
                }
            }

            let avail_w = ui.available_width().min(620.0).max(180.0);
            let mut preview = Vec2::new(avail_w, avail_w / image_aspect.max(0.1));
            if preview.y > 430.0 { preview.y = 430.0; preview.x = preview.y * image_aspect; }
            let (rect, response) = ui.allocate_exact_size(preview, egui::Sense::click_and_drag());
            ui.painter().image(texture, rect, egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);

            if self.slicer.mode == SliceMode::Grid {
                for c in 1..self.slicer.cols { let x = rect.left() + rect.width() * c as f32 / self.slicer.cols as f32; ui.painter().line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())], Stroke::new(1.0, Color32::LIGHT_BLUE)); }
                for r in 1..self.slicer.rows { let y = rect.top() + rect.height() * r as f32 / self.slicer.rows as f32; ui.painter().line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)], Stroke::new(1.0, Color32::LIGHT_BLUE)); }
            } else if self.slicer.mode == SliceMode::Rectangles {
                for r in &self.slicer.rects {
                    let rr = egui::Rect::from_min_size(Pos2::new(rect.left() + r.x * rect.width(), rect.top() + r.y * rect.height()), Vec2::new(r.w * rect.width(), r.h * rect.height()));
                    ui.painter().rect_stroke(rr, 0.0, Stroke::new(2.0, Color32::YELLOW), egui::StrokeKind::Inside);
                }
                if response.drag_started() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        self.slicer.rect_drag_start = Some([((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0), ((pos.y - rect.top()) / rect.height()).clamp(0.0, 1.0)]);
                    }
                }
                if let (Some(start), Some(pos)) = (self.slicer.rect_drag_start, response.interact_pointer_pos()) {
                    let end = [((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0), ((pos.y - rect.top()) / rect.height()).clamp(0.0, 1.0)];
                    let x0 = start[0].min(end[0]);
                    let y0 = start[1].min(end[1]);
                    let x1 = start[0].max(end[0]);
                    let y1 = start[1].max(end[1]);
                    let live = egui::Rect::from_min_max(
                        Pos2::new(rect.left() + x0 * rect.width(), rect.top() + y0 * rect.height()),
                        Pos2::new(rect.left() + x1 * rect.width(), rect.top() + y1 * rect.height()),
                    );
                    ui.painter().rect_stroke(live, 0.0, Stroke::new(2.0, Color32::LIGHT_GREEN), egui::StrokeKind::Inside);
                    if response.drag_stopped() {
                        let w = x1 - x0;
                        let h = y1 - y0;
                        if w > 0.005 && h > 0.005 {
                            self.slicer.rects.push(NormalizedRect { x: x0, y: y0, w, h }.clamped());
                        }
                        self.slicer.rect_drag_start = None;
                    }
                }
            } else {
                if response.dragged() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let p = [((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0), ((pos.y - rect.top()) / rect.height()).clamp(0.0, 1.0)];
                        let push = self.slicer.freehand.last().map(|last| { let dx = last[0] - p[0]; let dy = last[1] - p[1]; dx * dx + dy * dy > 0.00008 }).unwrap_or(true);
                        if push { self.slicer.freehand.push(p); }
                    }
                }
                for pair in self.slicer.freehand.windows(2) {
                    let a = Pos2::new(rect.left() + pair[0][0] * rect.width(), rect.top() + pair[0][1] * rect.height());
                    let b = Pos2::new(rect.left() + pair[1][0] * rect.width(), rect.top() + pair[1][1] * rect.height());
                    ui.painter().line_segment([a,b], Stroke::new(2.0, Color32::YELLOW));
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Apply slice").clicked() { apply = true; }
                if ui.button("Cancel").clicked() { cancel = true; }
            });
        });
        if cancel { self.slicer.open = false; }
        if apply { self.apply_slice(ctx, idx); }
    }

    fn apply_slice(&mut self, ctx: &egui::Context, idx: usize) {
        if idx >= self.project.layers.len() { return; }
        let layer = self.project.layers[idx].clone();
        let Some(runtime) = self.assets.images.get(&layer.id) else { return; };
        let result = match self.slicer.mode {
            SliceMode::Grid => slicer::grid_slice(&layer, runtime, self.slicer.cols, self.slicer.rows),
            SliceMode::Rectangles => slicer::rectangle_slice(&layer, runtime, &self.slicer.rects),
            SliceMode::Freehand => slicer::freehand_slice(&layer, runtime, &self.slicer.freehand),
        };
        match result {
            Ok(new_layers) => {
                let old_id = layer.id;
                self.project.layers.remove(idx);
                self.assets.images.remove(&old_id);
                let count = new_layers.len();
                for (offset, new_layer) in new_layers.into_iter().enumerate() {
                    let id = new_layer.id.clone();
                    let path = match &new_layer.content { LayerContent::Image { path } => path.clone(), _ => continue };
                    self.project.layers.insert(idx + offset, new_layer);
                    if let Err(err) = self.assets.load_image(ctx, &id, &path) { self.status = format!("Slice asset load failed: {err:#}"); }
                }
                self.selected_layer = Some(idx);
                self.slicer.open = false;
                self.status = format!("Created {count} slice layer(s)");
            }
            Err(err) => self.status = format!("Slice failed: {err:#}"),
        }
    }
}

impl eframe::App for InkplainerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let before = self.project.clone();
        self.update_playback(ctx);
        self.process_shortcuts(ctx);
        self.top_bar(ctx);
        self.bottom_bar(ctx);
        self.layers_panel(ctx);
        self.settings_panel(ctx);
        self.center_canvas(ctx);
        self.slicer_window(ctx);
        self.maybe_autosave();

        if !self.suppress_history_once && fingerprint(&before) != fingerprint(&self.project) {
            let duplicate = self.undo_stack.last().map(|p| fingerprint(p) == fingerprint(&before)).unwrap_or(false);
            if !duplicate {
                self.undo_stack.push(before);
                if self.undo_stack.len() > 100 { self.undo_stack.remove(0); }
                self.redo_stack.clear();
            }
        }
        self.suppress_history_once = false;
    }
}

fn uses_chunks(style: AnimationStyle) -> bool {
    matches!(style, AnimationStyle::ChunkJump | AnimationStyle::OutlineChunks | AnimationStyle::Scribble | AnimationStyle::TopAnchor | AnimationStyle::Human | AnimationStyle::Animal | AnimationStyle::Portrait | AnimationStyle::Vehicle | AnimationStyle::Building | AnimationStyle::Landscape | AnimationStyle::Spiral | AnimationStyle::Nature)
}

fn uses_drawing(style: AnimationStyle) -> bool {
    matches!(style, AnimationStyle::Contour | AnimationStyle::OutlineChunks | AnimationStyle::OutlineFill | AnimationStyle::IllustFill | AnimationStyle::OutlineOnly | AnimationStyle::TextDraw)
}

fn background_label(bg: &Background) -> &'static str {
    match bg {
        Background::Solid(_) => "Solid / Custom",
        Background::Transparent => "None / Transparent",
        Background::Notebook => "Notebook",
        Background::Graph => "Graph",
        Background::Cream => "Cream",
        Background::Chalkboard => "Chalkboard",
        Background::SoftGradient => "Soft Gradient",
        Background::WarmWhite => "Warm White",
        Background::Blueprint => "Blueprint",
        Background::Kraft => "Kraft",
        Background::Dark => "Dark",
        Background::Linen => "Linen",
    }
}

fn background_choices() -> Vec<(Background, &'static str)> {
    vec![
        (Background::Solid([255,255,255,255]), "White / Custom"),
        (Background::Solid([0,0,0,255]), "Black"),
        (Background::Transparent, "None"),
        (Background::Notebook, "Notebook"),
        (Background::Graph, "Graph"),
        (Background::Cream, "Cream"),
        (Background::Chalkboard, "Chalkboard"),
        (Background::SoftGradient, "Soft Gradient"),
        (Background::WarmWhite, "Warm White"),
        (Background::Blueprint, "Blueprint"),
        (Background::Kraft, "Kraft"),
        (Background::Dark, "Dark"),
        (Background::Linen, "Linen"),
    ]
}

fn bg_choice(ui: &mut egui::Ui, current: &mut Background, value: Background, label: &str) {
    let selected = match (&*current, &value) {
        (Background::Solid(a), Background::Solid(b)) => a == b,
        _ => std::mem::discriminant(&*current) == std::mem::discriminant(&value),
    };
    if ui.selectable_label(selected, label).clicked() { *current = value; }
}

fn set_export_resolution(project: &mut Project, base: u32) {
    let cw = project.canvas.width.max(1) as f32;
    let ch = project.canvas.height.max(1) as f32;
    if (cw - ch).abs() / cw.max(ch) < 0.05 {
        project.export.width = base;
        project.export.height = base;
    } else if ch > cw {
        project.export.width = base;
        project.export.height = (base as f32 * ch / cw).round() as u32;
    } else {
        project.export.height = base;
        project.export.width = (base as f32 * cw / ch).round() as u32;
    }
}

fn fingerprint(project: &Project) -> String {
    serde_json::to_string(project).unwrap_or_default()
}