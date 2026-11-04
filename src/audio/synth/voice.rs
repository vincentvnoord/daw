use crate::audio::{
    dsp::filter::LowPassFilter,
    note::midi_to_freq,
    synth::{
        envelope::{Envelope, EnvelopeSettings, State},
        osc::{Oscillator, WaveForm},
    },
};

pub struct Voice {
    osc: Oscillator,
    env: Envelope,
    filter: LowPassFilter,
    note: u8,
}

impl Voice {
    pub fn new(osc: Oscillator, env: Envelope, filter: LowPassFilter) -> Voice {
        Voice {
            osc,
            env,
            note: 0,
            filter,
        }
    }

    pub fn note_on(&mut self, note: u8) {
        self.note = note;

        let frequency = midi_to_freq(note);
        self.osc.set_frequency(frequency);
        self.env.trigger();
    }

    pub fn note_off(&mut self) {
        self.env.release();
    }

    pub fn add_to(&mut self, out: &mut [f32]) {
        for sample in out {
            let raw = self.osc.next_sample();
            let filtered = self.filter.process(raw);

            *sample += filtered * self.env.next_value();
        }
    }

    pub fn is_idle(&self) -> bool {
        self.env.is_idle()
    }

    pub fn note(&self) -> u8 {
        self.note
    }

    pub fn is_held(&self) -> bool {
        matches!(
            self.env.state(),
            State::Attack | State::Decay | State::Sustain
        )
    }

    pub fn set_waveform(&mut self, waveform: WaveForm) {
        self.osc.set_waveform(waveform);
    }

    pub fn set_cutoff(&mut self, cutoff: f32) {
        self.filter.set_cutoff(cutoff);
    }

    pub fn set_adsr(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) {
        let settings = EnvelopeSettings {
            attack,
            decay,
            sustain,
            release,
        };

        self.env.set_settings(settings);
    }
}
