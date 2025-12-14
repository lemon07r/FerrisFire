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

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 550.0])
            .with_min_inner_size([350.0, 450.0])
            .with_title("FerrisFire"),
        ..Default::default()
    };

    eframe::run_native(
        "FerrisFire",
        options,
        Box::new(|cc| Ok(Box::new(FerrisFireApp::new(cc)))),
    )
}
