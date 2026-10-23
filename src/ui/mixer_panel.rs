use eframe::egui;

use crate::{
    audio::mixer::MixerParams,
    ui::widgets::{card::card, fader::Fader, value::Unit},
};

pub fn show(ui: &mut egui::Ui, params: &MixerParams) {
    card(ui, "Master", |ui| {
        ui.add(Fader::new(&params.gain, 0.0..=1.0, "Gain", Unit::Decibels).height(150.0));
    });
}
