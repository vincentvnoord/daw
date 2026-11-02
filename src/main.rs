use std::error::Error;

use eframe::egui;

use crate::{
    audio::engine::{AudioDevice, AudioEngine},
    project::default::default_project,
    ui::app::App,
};

mod atomic;
mod audio;
mod project;
mod ui;

fn main() -> Result<(), Box<dyn Error>> {
    let device = AudioDevice::open()?;
    let (project, studio) = default_project(device.sample_rate());
    let _engine = AudioEngine::new(device, studio)?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 720.0])
            .with_min_inner_size([720.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Vincent Studio",
        options,
        Box::new(|cc| {
            ui::theme::apply(&cc.egui_ctx);
            Ok(Box::new(App::new(project)))
        }),
    )?;

    Ok(())
}
