use eframe::egui;

use crate::{
    audio::{
        engine::{AudioDevice, AudioEngine},
        studio::default::default_studio,
    },
    ui::app::App,
};

mod audio;
mod events;
mod helpers;
mod shared;
mod ui;

fn main() -> eframe::Result {
    let device = AudioDevice::default();
    let (studio, params, notes) = default_studio(device.sample_rate());
    let _engine = AudioEngine::new(device, studio);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([760.0, 440.0])
            .with_min_inner_size([560.0, 380.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Vincent Studio",
        options,
        Box::new(|cc| {
            ui::theme::apply(&cc.egui_ctx);
            Ok(Box::new(App::new(notes, params)))
        }),
    )
}
