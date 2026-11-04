use std::sync::Arc;

use rtrb::{Producer, RingBuffer};

use crate::audio::{
    instrument::{Instrument, InstrumentSource, NoteSource},
    mixer::channel::{ChannelId, ChannelRoute},
    note::NoteEvent,
};

/// How many note events can wait for the audio thread per instrument.
const NOTE_QUEUE_SIZE: usize = 256;

pub struct InstrumentParams {
    pub route: ChannelRoute,
}

impl InstrumentParams {
    pub fn new(channel: Option<ChannelId>) -> Self {
        Self {
            route: ChannelRoute::new(channel),
        }
    }
}

pub struct InstrumentSlot {
    pub source: Box<dyn NoteSource>,
    /// Index in the mixer of the channel this instrument plays into, looked
    /// up from the params once per buffer.
    pub output: Option<usize>,
    pub params: Arc<InstrumentParams>,
}

impl InstrumentSlot {
    /// Wraps an instrument for the studio, routed to `channel`. Returns the
    /// slot, the end the keyboard sends notes into, and the params for the UI.
    pub fn create(
        instrument: impl Instrument,
        channel: Option<ChannelId>,
    ) -> (Self, Producer<NoteEvent>, Arc<InstrumentParams>) {
        let (notes, consumer) = RingBuffer::new(NOTE_QUEUE_SIZE);
        let params = Arc::new(InstrumentParams::new(channel));
        let source = InstrumentSource::new(consumer, instrument);
        let slot = Self::new(Arc::clone(&params), Box::new(source));

        (slot, notes, params)
    }

    pub fn new(params: Arc<InstrumentParams>, source: Box<dyn NoteSource>) -> Self {
        Self {
            source,
            output: None,
            params,
        }
    }
}

#[cfg(test)]
pub mod recorder {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::audio::source::AudioSource;

    /// What a `Recorder` wrote down: each note it got, with how many frames
    /// it had rendered when the note came in.
    pub type Notes = Arc<Mutex<Vec<(usize, NoteEvent)>>>;

    /// An instrument for tests that makes no sound and writes down its notes.
    pub struct Recorder {
        rendered: usize,
        notes: Notes,
    }

    impl Recorder {
        /// A slot holding a recorder, routed nowhere, and what it writes down.
        pub fn slot() -> (InstrumentSlot, Notes) {
            let notes = Notes::default();
            let recorder = Self {
                rendered: 0,
                notes: Arc::clone(&notes),
            };
            let params = Arc::new(InstrumentParams::new(None));

            (InstrumentSlot::new(params, Box::new(recorder)), notes)
        }
    }

    impl AudioSource for Recorder {
        fn process(&mut self, out: &mut [f32]) {
            out.fill(0.0);
            self.rendered += out.len();
        }
    }

    impl NoteSource for Recorder {
        fn note(&mut self, event: NoteEvent) {
            self.notes.lock().unwrap().push((self.rendered, event));
        }
    }
}
