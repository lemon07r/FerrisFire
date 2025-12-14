mod config;
mod device;
mod gui;
mod humanize;
mod proxy;

use eframe::egui;
use gui::FerrisFireApp;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("FerrisFire starting...");

    let icon = load_icon();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 740.0])
            .with_min_inner_size([380.0, 620.0])
            .with_title("FerrisFire")
            .with_icon(icon),
        ..Default::default()
    };

    eframe::run_native(
        "FerrisFire",
        options,
        Box::new(|cc| Ok(Box::new(FerrisFireApp::new(cc)))),
    )
}

fn load_icon() -> egui::IconData {
    let icon_bytes = include_bytes!("../assets/ferrisfire.ico");
    
    match image::load_from_memory(icon_bytes) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let (width, height) = rgba.dimensions();
            egui::IconData {
                rgba: rgba.into_raw(),
                width,
                height,
            }
        }
        Err(e) => {
            log::warn!("Failed to load icon: {}", e);
            egui::IconData::default()
        }
    }
}
