use std::sync::Arc;

use rtrb::{Producer, RingBuffer};

use crate::audio::{
    instrument::{Instrument, InstrumentSource},
    mixer::channel::{ChannelId, ChannelRoute},
    note::NoteEvent,
    source::AudioSource,
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
    pub source: Box<dyn AudioSource>,
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

    pub fn new(params: Arc<InstrumentParams>, source: Box<dyn AudioSource>) -> Self {
        Self {
            source,
            output: None,
            params,
        }
    }
}
