mod animation;
mod app {
    include!("app.rs");
    #[allow(unused_imports)]
    pub(crate) mod native_ui;
}
mod assets;
mod effects;
mod export;
mod export_scale;
mod export_worker;
mod font_catalog;
mod hand_assets;
mod model;
mod native_mp4;
mod native_webm;
mod project_io;
mod render;
mod slicer;
mod timing;

#[cfg(test)]
mod audit_tests;
#[cfg(test)]
mod regression_tests;

use std::collections::HashSet;

use app::InkplainerApp;
use eframe::egui;
use model::LayerContent;

struct NativeUiApp {
    inner: InkplainerApp,
    left_tab: app::native_ui::LeftPanelTab,
    input_mode: app::native_ui::InputMode,
}

impl NativeUiApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        app::native_ui::configure(&cc.egui_ctx);
        Self {
            inner: InkplainerApp::new(cc),
            left_tab: app::native_ui::LeftPanelTab::default(),
            input_mode: app::native_ui::InputMode::default(),
        }
    }

    fn normalize_export_dimensions(&mut self) {
        let resolved = export_scale::resolved_export_dimensions(&self.inner.project);
        let current = (
            self.inner.project.export.width,
            self.inner.project.export.height,
        );
        if current != resolved {
            self.inner.project.export.width = resolved.0;
            self.inner.project.export.height = resolved.1;
        }
    }

    fn prune_failed_new_image_layers(&mut self, before_ids: &HashSet<String>) -> usize {
        if self.inner.status.starts_with("Opened ")
            || self.inner.status.starts_with("Autosave restored")
        {
            return 0;
        }

        let images = &self.inner.assets.images;
        let before_len = self.inner.project.layers.len();
        self.inner.project.layers.retain(|layer| {
            if before_ids.contains(&layer.id) {
                return true;
            }
            match &layer.content {
                LayerContent::Text(_) => true,
                LayerContent::Image { .. } => images.contains_key(&layer.id),
            }
        });
        let removed = before_len.saturating_sub(self.inner.project.layers.len());
        if removed == 0 {
            return 0;
        }

        if self
            .inner
            .selected_layer
            .is_some_and(|index| index >= self.inner.project.layers.len())
        {
            self.inner.selected_layer = self.inner.project.layers.len().checked_sub(1);
        }
        self.inner.status = format!(
            "{} | {removed} failed image layer(s) were not added",
            self.inner.status
        );
        removed
    }

    fn repair_autosave_after_failed_import(&mut self, removed: usize) {
        if removed == 0 || !self.inner.project.autosave_enabled {
            return;
        }

        if let Err(err) = project_io::save_autosave(&self.inner.project) {
            self.inner.status = format!(
                "{} | clean autosave after rollback failed: {err:#}",
                self.inner.status
            );
        }
    }
}

impl eframe::App for NativeUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let before_ids: HashSet<String> = self
            .inner
            .project
            .layers
            .iter()
            .map(|layer| layer.id.clone())
            .collect();
        app::native_ui::update(
            &mut self.inner,
            ctx,
            &mut self.left_tab,
            &mut self.input_mode,
        );
        let removed = self.prune_failed_new_image_layers(&before_ids);
        self.repair_autosave_after_failed_import(removed);
        self.normalize_export_dimensions();
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Inkplainer RustApp")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1000.0, 650.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Inkplainer RustApp",
        options,
        Box::new(|cc| Ok(Box::new(NativeUiApp::new(cc)))),
    )
}
