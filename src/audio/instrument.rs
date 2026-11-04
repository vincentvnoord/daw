use rtrb::Consumer;

use crate::audio::{note::NoteEvent, source::AudioSource};

pub trait Instrument: AudioSource {
    fn note_on(&mut self, note: u8);
    fn note_off(&mut self, note: u8);
}

/// An instrument as the studio holds it: it makes sound and takes notes,
/// whichever instrument is inside.
pub trait NoteSource: AudioSource {
    /// Plays or releases a note, starting from the next sample.
    fn note(&mut self, event: NoteEvent);
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

impl<I: Instrument> NoteSource for InstrumentSource<I> {
    fn note(&mut self, event: NoteEvent) {
        match event {
            NoteEvent::On(note) => self.instrument.note_on(note),
            NoteEvent::Off(note) => self.instrument.note_off(note),
        }
    }
}

impl<I: Instrument> AudioSource for InstrumentSource<I> {
    fn begin_buffer(&mut self) {
        // live notes from the keyboard land on the buffer's first sample;
        // scheduled notes come in through `note` on their exact sample
        while let Ok(event) = self.notes.pop() {
            self.note(event);
        }

        self.instrument.begin_buffer();
    }

    fn process(&mut self, out: &mut [f32]) {
        self.instrument.process(out);
    }
}
