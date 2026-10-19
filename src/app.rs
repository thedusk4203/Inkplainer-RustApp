use eframe::egui::{self, Color32, RichText};
use std::path::PathBuf;
use std::time::Instant;

use crate::assets::RuntimeAssets;
use crate::export;
use crate::model::{AnimationStyle, Background, Layer, LayerContent, Project, TextAlign};
use crate::project_io;
use crate::render;

pub struct InkplainerApp {
    pub project: Project,
    pub assets: RuntimeAssets,
    pub selected_layer: Option<usize>,
    pub project_path: Option<PathBuf>,
    pub playhead: f32,
    pub playing: bool,
    pub status: String,
    last_tick: Instant,
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
        else {
            return;
        };

        for path in paths {
            match image::image_dimensions(&path) {
                Ok((w, h)) => {
                    let layer = Layer::new_image(
                        path.clone(),
                        self.project.canvas.width,
                        self.project.canvas.height,
                        w,
                        h,
                    );
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
        let layer = Layer::new_text(self.project.canvas.width, self.project.canvas.height);
        self.project.layers.push(layer);
        self.selected_layer = Some(self.project.layers.len() - 1);
        self.playhead = 1.0;
        self.status = "Text layer added".to_owned();
    }

    fn save_project_as(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Inkplainer Project", &["json"])
            .set_file_name("project.inkplainer.json")
            .save_file()
        else {
            return;
        };
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
        } else {
            self.save_project_as();
        }
    }

    fn open_project(&mut self, ctx: &egui::Context) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Inkplainer Project", &["json"])
            .pick_file()
        else {
            return;
        };
        match project_io::load_project(&path) {
            Ok(project) => {
                self.project = project;
                let errors = self.assets.load_project(ctx, &self.project);
                self.project_path = Some(path.clone());
                self.selected_layer = None;
                self.playhead = 1.0;
                self.playing = false;
                self.status = if errors.is_empty() {
                    format!("Opened {}", path.display())
                } else {
                    format!("Opened with missing assets: {}", errors.join(" | "))
                };
            }
            Err(err) => self.status = format!("Open failed: {err:#}"),
        }
    }

    fn export_png(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG image", &["png"])
            .set_file_name("inkplainer-frame.png")
            .save_file()
        else {
            return;
        };
        match export::export_png(&self.project, &self.assets, &path) {
            Ok(()) => self.status = format!("PNG exported: {}", path.display()),
            Err(err) => self.status = format!("PNG export failed: {err:#}"),
        }
    }

    fn export_mp4(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("MP4 video", &["mp4"])
            .set_file_name("inkplainer-animation.mp4")
            .save_file()
        else {
            return;
        };
        self.status = "Rendering MP4…".to_owned();
        match export::export_mp4(&self.project, &self.assets, &path, 30) {
            Ok(()) => self.status = format!("MP4 exported: {}", path.display()),
            Err(err) => self.status = format!("MP4 export failed: {err:#}"),
        }
    }

    fn update_playback(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        let dt = (now - self.last_tick).as_secs_f32();
        self.last_tick = now;
        if self.playing {
            self.playhead += dt / self.project.duration_seconds.max(0.5);
            if self.playhead >= 1.0 {
                self.playhead = 1.0;
                self.playing = false;
            }
            ctx.request_repaint();
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
                ui.separator();
                if ui.button("Import image").clicked() { self.import_images(ctx); }
                if ui.button("Add text").clicked() { self.add_text(); }
                ui.separator();
                if ui.button("Export PNG").clicked() { self.export_png(); }
                if ui.button("Export MP4").clicked() { self.export_mp4(); }
            });
        });
    }

    fn layers_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("layers_panel")
            .resizable(true)
            .default_width(230.0)
            .show(ctx, |ui| {
                ui.heading("Layers");
                ui.small("Bottom → top visual stacking order");
                ui.separator();

                let mut delete_idx = None;
                let mut move_up = None;
                let mut move_down = None;
                for idx in (0..self.project.layers.len()).rev() {
                    let selected = self.selected_layer == Some(idx);
                    let mut clicked = false;
                    {
                        let layer = &mut self.project.layers[idx];
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut layer.visible, "");
                            clicked = ui.selectable_label(selected, &layer.name).clicked();
                        });
                    }
                    if clicked {
                        self.selected_layer = Some(idx);
                    }
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
                    if idx + 1 < self.project.layers.len() {
                        self.project.layers.swap(idx, idx + 1);
                        self.selected_layer = Some(idx + 1);
                    }
                }
                if let Some(idx) = move_down {
                    if idx > 0 {
                        self.project.layers.swap(idx, idx - 1);
                        self.selected_layer = Some(idx - 1);
                    }
                }
                if let Some(idx) = delete_idx {
                    let id = self.project.layers[idx].id.clone();
                    self.project.layers.remove(idx);
                    self.assets.images.remove(&id);
                    self.selected_layer = None;
                }
            });
    }

    fn settings_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("settings_panel")
            .resizable(true)
            .default_width(310.0)
            .show(ctx, |ui| {
                ui.heading("Properties");
                ui.separator();

                ui.collapsing("Canvas", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Size");
                        ui.add(egui::DragValue::new(&mut self.project.canvas.width).range(320..=3840));
                        ui.label("×");
                        ui.add(egui::DragValue::new(&mut self.project.canvas.height).range(240..=2160));
                    });
                    egui::ComboBox::from_label("Background")
                        .selected_text(background_label(&self.project.canvas.background))
                        .show_ui(ui, |ui| {
                            bg_choice(ui, &mut self.project.canvas.background, Background::Solid([255,255,255,255]), "White");
                            bg_choice(ui, &mut self.project.canvas.background, Background::Transparent, "Transparent");
                            bg_choice(ui, &mut self.project.canvas.background, Background::Notebook, "Notebook");
                            bg_choice(ui, &mut self.project.canvas.background, Background::Graph, "Graph");
                            bg_choice(ui, &mut self.project.canvas.background, Background::Blueprint, "Blueprint");
                            bg_choice(ui, &mut self.project.canvas.background, Background::Kraft, "Kraft");
                            bg_choice(ui, &mut self.project.canvas.background, Background::Dark, "Dark");
                        });
                    ui.add(egui::Slider::new(&mut self.project.duration_seconds, 0.5..=120.0).text("Duration (s)"));
                });

                ui.separator();
                let Some(idx) = self.selected_layer else {
                    ui.label("Select a layer to edit it.");
                    return;
                };
                if idx >= self.project.layers.len() {
                    self.selected_layer = None;
                    return;
                }

                let layer = &mut self.project.layers[idx];
                ui.text_edit_singleline(&mut layer.name);
                ui.checkbox(&mut layer.visible, "Visible");
                ui.add(egui::Slider::new(&mut layer.opacity, 0.0..=1.0).text("Opacity"));
                ui.horizontal(|ui| {
                    ui.label("Animation order");
                    let mut order = layer.anim_order.unwrap_or(0);
                    let enabled = layer.anim_order.is_some();
                    let mut use_order = enabled;
                    if ui.checkbox(&mut use_order, "").changed() {
                        layer.anim_order = if use_order { Some(order.max(1)) } else { None };
                    }
                    if use_order && ui.add(egui::DragValue::new(&mut order).range(1..=999)).changed() {
                        layer.anim_order = Some(order.max(1));
                    }
                });

                ui.collapsing("Position & size", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("X"); ui.add(egui::DragValue::new(&mut layer.transform.x).speed(1.0));
                        ui.label("Y"); ui.add(egui::DragValue::new(&mut layer.transform.y).speed(1.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("W"); ui.add(egui::DragValue::new(&mut layer.transform.w).speed(1.0).range(1.0..=10000.0));
                        ui.label("H"); ui.add(egui::DragValue::new(&mut layer.transform.h).speed(1.0).range(1.0..=10000.0));
                    });
                });

                ui.separator();
                ui.heading("Animation");
                egui::ComboBox::from_label("Style")
                    .selected_text(layer.animation.style.label())
                    .show_ui(ui, |ui| {
                        for style in AnimationStyle::ALL {
                            ui.selectable_value(&mut layer.animation.style, style, style.label());
                        }
                    });
                ui.add(egui::Slider::new(&mut layer.animation.speed, 1.0..=100.0).text("Reveal speed"));
                ui.add(egui::Slider::new(&mut layer.animation.hand_speed, 1.0..=30.0).text("Hand speed"));
                if matches!(
                    layer.animation.style,
                    AnimationStyle::ChunkJump
                        | AnimationStyle::OutlineChunks
                        | AnimationStyle::Human
                        | AnimationStyle::Animal
                        | AnimationStyle::Portrait
                        | AnimationStyle::Vehicle
                        | AnimationStyle::Building
                        | AnimationStyle::Landscape
                        | AnimationStyle::Spiral
                ) {
                    ui.add(egui::Slider::new(&mut layer.animation.chunks, 6..=80).text("Chunks"));
                }
                if matches!(layer.animation.style, AnimationStyle::Scanner) {
                    ui.checkbox(&mut layer.animation.zigzag, "Zigzag");
                }
                if matches!(layer.animation.style, AnimationStyle::OutlineFill | AnimationStyle::IllustFill) {
                    ui.checkbox(&mut layer.animation.reveal_original, "Reveal original image");
                }
                if matches!(
                    layer.animation.style,
                    AnimationStyle::Contour
                        | AnimationStyle::OutlineFill
                        | AnimationStyle::IllustFill
                        | AnimationStyle::OutlineOnly
                        | AnimationStyle::OutlineChunks
                ) {
                    ui.add(egui::Slider::new(&mut layer.animation.outline_thickness, 0.5..=8.0).text("Outline thickness"));
                    let mut color = Color32::from_rgba_unmultiplied(
                        layer.animation.outline_color[0],
                        layer.animation.outline_color[1],
                        layer.animation.outline_color[2],
                        layer.animation.outline_color[3],
                    );
                    if ui.color_edit_button_srgba(&mut color).changed() {
                        layer.animation.outline_color = [color.r(), color.g(), color.b(), color.a()];
                    }
                }

                if let LayerContent::Text(text) = &mut layer.content {
                    ui.separator();
                    ui.heading("Text");
                    ui.text_edit_multiline(&mut text.text);
                    ui.add(egui::Slider::new(&mut text.size, 10.0..=400.0).text("Font size"));
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut text.bold, "Bold");
                        ui.checkbox(&mut text.italic, "Italic");
                    });
                    egui::ComboBox::from_label("Align")
                        .selected_text(match text.align { TextAlign::Left => "Left", TextAlign::Center => "Center", TextAlign::Right => "Right" })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut text.align, TextAlign::Left, "Left");
                            ui.selectable_value(&mut text.align, TextAlign::Center, "Center");
                            ui.selectable_value(&mut text.align, TextAlign::Right, "Right");
                        });
                }
            });
    }

    fn center_canvas(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                let result = render::paint_project(
                    ui,
                    &self.project,
                    &self.assets,
                    self.selected_layer,
                    self.playhead,
                );
                let _ = result.rect;
                if let (Some(idx), Some(delta)) = (self.selected_layer, result.selected_drag_delta_canvas) {
                    if let Some(layer) = self.project.layers.get_mut(idx) {
                        layer.transform.x += delta.x;
                        layer.transform.y += delta.y;
                    }
                }
            });
        });
    }

    fn bottom_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let label = if self.playing { "Pause" } else { "Play" };
                if ui.button(label).clicked() {
                    if self.playhead >= 1.0 { self.playhead = 0.0; }
                    self.playing = !self.playing;
                    self.last_tick = Instant::now();
                }
                if ui.button("Restart").clicked() {
                    self.playhead = 0.0;
                    self.playing = true;
                    self.last_tick = Instant::now();
                }
                ui.add(egui::Slider::new(&mut self.playhead, 0.0..=1.0).show_value(false).text("Timeline"));
                ui.label(format!("{:>3}%", (self.playhead * 100.0).round() as u32));
                ui.separator();
                ui.label(RichText::new(&self.status).small());
            });
        });
    }
}

impl eframe::App for InkplainerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_playback(ctx);
        self.top_bar(ctx);
        self.bottom_bar(ctx);
        self.layers_panel(ctx);
        self.settings_panel(ctx);
        self.center_canvas(ctx);
    }
}

fn background_label(bg: &Background) -> &'static str {
    match bg {
        Background::Solid(_) => "Solid",
        Background::Transparent => "Transparent",
        Background::Notebook => "Notebook",
        Background::Graph => "Graph",
        Background::Blueprint => "Blueprint",
        Background::Kraft => "Kraft",
        Background::Dark => "Dark",
    }
}

fn bg_choice(ui: &mut egui::Ui, current: &mut Background, value: Background, label: &str) {
    let selected = std::mem::discriminant(current) == std::mem::discriminant(&value);
    if ui.selectable_label(selected, label).clicked() {
        *current = value;
    }
}
