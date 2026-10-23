use eframe::egui;

use crate::{
    audio::synth::SynthParams,
    ui::widgets::{
        card::card, envelope_view::EnvelopeView, knob::Knob, value::Unit,
        waveform_picker::WaveformPicker,
    },
};

pub fn show(ui: &mut egui::Ui, params: &SynthParams) {
    card(ui, "Oscillator", |ui| {
        ui.add(WaveformPicker::new(&params.waveform));
    });

    card(ui, "Filter", |ui| {
        ui.add(Knob::new(&params.cutoff, 20.0..=20000.0, "Cutoff", Unit::Hz).logarithmic(true));
    });

    card(ui, "Amp envelope", |ui| {
        ui.set_width(300.0);
        ui.add(EnvelopeView {
            attack: params.attack.get(),
            decay: params.decay.get(),
            sustain: params.sustain.get(),
            release: params.release.get(),
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add(Knob::new(&params.attack, 0.0..=5.0, "Attack", Unit::Seconds).logarithmic(true));
            ui.add(Knob::new(&params.decay, 0.0..=5.0, "Decay", Unit::Seconds).logarithmic(true));
            ui.add(Knob::new(
                &params.sustain,
                0.0..=1.0,
                "Sustain",
                Unit::Percent,
            ));
            ui.add(
                Knob::new(&params.release, 0.0..=10.0, "Release", Unit::Seconds).logarithmic(true),
            );
        });
    });
}
