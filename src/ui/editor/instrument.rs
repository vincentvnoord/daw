//! The controls for an instrument track: the synth's knobs, or the sample a
//! sampler plays.

use eframe::egui::{self, RichText};

use crate::{
    audio::synth::SynthParams,
    project::{InstrumentHandle, InstrumentKind},
    ui::{
        theme,
        widgets::{
            card::card, envelope_view::EnvelopeView, knob::Knob, value::Unit,
            waveform_picker::WaveformPicker,
        },
    },
};

pub(super) fn instrument_controls(ui: &mut egui::Ui, instrument: &InstrumentHandle) {
    match &instrument.kind {
        InstrumentKind::Synth(params) => synth_controls(ui, params),
        InstrumentKind::Sampler { sample } => {
            card(ui, "Sample", |ui| {
                ui.label(RichText::new(sample).font(theme::value_font()));
            });
        }
    }
}

fn synth_controls(ui: &mut egui::Ui, params: &SynthParams) {
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
