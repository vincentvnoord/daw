use crate::{
    audio::{envelope::State, note::midi_to_freq, osc::Oscillator},
    envelope::Envelope,
};

pub struct Voice {
    osc: Oscillator,
    env: Envelope,
    note: u8,
}

impl Voice {
    pub fn new(osc: Oscillator, env: Envelope) -> Voice {
        Voice { osc, env, note: 0 }
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

    pub fn next_sample(&mut self) -> f32 {
        self.osc.next_sample() * self.env.next_value()
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
}
