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
mod regression_tests;

use app::InkplainerApp;
use eframe::egui;

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
}

impl eframe::App for NativeUiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        app::native_ui::update(
            &mut self.inner,
            ctx,
            &mut self.left_tab,
            &mut self.input_mode,
        );
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
