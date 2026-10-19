use rtrb::Consumer;

use crate::audio::{source::AudioSource, synth::Synth};
use crate::events::NoteEvent;

pub struct Instrument {
    notes: Consumer<NoteEvent>,
    synth: Synth,
}

impl Instrument {
    pub fn new(notes: Consumer<NoteEvent>, synth: Synth) -> Self {
        Self { notes, synth }
    }
}

impl AudioSource for Instrument {
    fn next_sample(&mut self) -> f32 {
        while let Ok(ev) = self.notes.pop() {
            self.synth.handle(ev);
        }

        self.synth.next_sample()
    }
}
