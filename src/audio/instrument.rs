use rtrb::Consumer;

use crate::audio::source::AudioSource;
use crate::events::NoteEvent;

pub trait Instrument: AudioSource {
    fn note_on(&mut self, note: u8);
    fn note_off(&mut self, note: u8);
}

pub struct InstrumentSource<I: Instrument> {
    notes: Consumer<NoteEvent>,
    instrument: I,
}

impl<I: Instrument> InstrumentSource<I> {
    pub fn new(notes: Consumer<NoteEvent>, instrument: I) -> Self {
        Self { notes, instrument }
    }
}

impl<I: Instrument> AudioSource for InstrumentSource<I> {
    fn begin_buffer(&mut self) {
        while let Ok(ev) = self.notes.pop() {
            match ev {
                NoteEvent::On(note) => self.instrument.note_on(note),
                NoteEvent::Off(note) => self.instrument.note_off(note),
            }
        }

        self.instrument.begin_buffer();
    }

    fn next_sample(&mut self) -> f32 {
        self.instrument.next_sample()
    }
}
