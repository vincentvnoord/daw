use crate::{envelope::Envelope, note::midi_to_freq, osc::Oscillator};

pub struct Voice {
    osc: Oscillator,
    env: Envelope,
}

impl Voice {
    pub fn new(osc: Oscillator, env: Envelope) -> Voice {
        Voice { osc, env }
    }

    pub fn note_on(&mut self, note: u8) {
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
}
