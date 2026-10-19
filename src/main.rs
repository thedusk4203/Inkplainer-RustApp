mod animation;
mod app;
mod assets;
mod effects;
mod export;
mod export_worker;
mod font_catalog;
mod model;
mod native_mp4;
mod native_webm;
mod project_io;
mod render;
mod slicer;

use app::InkplainerApp;
use eframe::egui;

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
        Box::new(|cc| Ok(Box::new(InkplainerApp::new(cc)))),
    )
}
