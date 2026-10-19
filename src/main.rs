use rtrb::RingBuffer;

use crate::{
    audio::{
        engine::{AudioDevice, AudioEngine},
        envelope::{self, Envelope, EnvelopeSettings},
        instrument::{self, Instrument},
        mixer::Mixer,
        osc::{Oscillator, WaveForm},
        sequence::Note,
        synth::Synth,
        voice::Voice,
    },
    events::NoteEvent,
    ui::app::App,
};

mod audio;
mod events;
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
    let voices: Vec<Voice> = (0..8)
        .map(|_| {
            Voice::new(
                Oscillator::new(WaveForm::Sine, 440.0, sr),
                Envelope::new(env_settings, sr),
            )
        })
        .collect();

    let synth = Synth::new(voices);

    let instrument = Instrument::new(cons, synth);

    let mut mixer = Mixer::new(0.2);
    mixer.add(instrument);

    let _engine = AudioEngine::new(device, mixer);

    eframe::run_native(
        "DAW",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(App::new(prod)))),
    )
}
