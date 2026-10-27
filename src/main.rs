use std::{path::Path, sync::Arc};

use eframe::egui;
use rtrb::RingBuffer;

use crate::{
    audio::{
        dsp::filter::LowPassFilter,
        engine::{AudioDevice, AudioEngine},
        instrument::{Instrument, InstrumentSource},
        mixer::{
            Mixer, MixerParams,
            channel::{Channel, ChannelParams},
            meter::Meter,
        },
        sampler::{Sampler, sample::Sample},
        synth::{
            Synth, SynthParams,
            envelope::{Envelope, EnvelopeSettings},
            osc::{AtomicWaveForm, Oscillator, WaveForm},
            voice::Voice,
        },
    },
    events::NoteEvent,
    helpers::AtomicF32,
    shared::AppParams,
    ui::app::App,
};

mod audio;
mod events;
mod helpers;
mod shared;
mod ui;

fn main() -> eframe::Result {
    let (prod, cons) = RingBuffer::<NoteEvent>::new(256);

    let device = AudioDevice::default();
    let sr = device.sample_rate();

    let env_settings = EnvelopeSettings {
        attack: 0.005,
        decay: 0.5,
        sustain: 1.0,
        release: 0.8,
    };
    let waveform = WaveForm::Saw;
    let voices: Vec<Voice> = (0..8)
        .map(|_| {
            Voice::new(
                Oscillator::new(waveform, 440.0, sr),
                Envelope::new(env_settings, sr),
                LowPassFilter::new(200.0, sr),
            )
        })
        .collect();

    let synth_params = Arc::new(SynthParams {
        waveform: AtomicWaveForm::new(waveform),
        cutoff: AtomicF32::new(500.0),
        attack: AtomicF32::new(env_settings.attack),
        decay: AtomicF32::new(env_settings.decay),
        sustain: AtomicF32::new(env_settings.sustain),
        release: AtomicF32::new(env_settings.release),
    });

    let _ = Synth::new(voices, Arc::clone(&synth_params));

    let sample = Sample::load(Path::new("testsamples/kick_16bit_stereo_44k.wav"), sr)
        .expect("failed to load sample");
    let sampler = Sampler::new(sample.data, 8);

    let instrument_source = InstrumentSource::new(cons, sampler);

    let channel_params = Arc::new(ChannelParams {
        gain: AtomicF32::new(0.2),
        meter: Meter::new(),
    });

    let mut channel0 = Channel::new(channel_params);
    channel0.add(instrument_source);

    let mixer_params = Arc::new(MixerParams {
        gain: AtomicF32::new(0.2),
    });
    let mut mixer = Mixer::new(Arc::clone(&mixer_params));
    mixer.add(channel0);

    let _engine = AudioEngine::new(device, mixer);

    let params = AppParams {
        synth: Arc::clone(&synth_params),
        mixer: Arc::clone(&mixer_params),
    };

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
            Ok(Box::new(App::new(prod, params)))
        }),
    )
}
