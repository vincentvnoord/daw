use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use rtrb::{Producer, RingBuffer};

use crate::{
    audio::{
        instrument::{Instrument, InstrumentSource},
        mixer::channel::ChannelId,
        source::AudioSource,
    },
    events::NoteEvent,
};

const NO_CHANNEL: u32 = u32::MAX;

/// How many note events can wait for the audio thread per instrument.
const NOTE_QUEUE_SIZE: usize = 256;

pub struct InstrumentParams {
    output_channel: AtomicU32,
}

impl InstrumentParams {
    pub fn new(channel: Option<ChannelId>) -> Self {
        Self {
            output_channel: AtomicU32::new(channel.map_or(NO_CHANNEL, ChannelId::to_raw)),
        }
    }

    pub fn set_channel(&self, channel: Option<ChannelId>) {
        self.output_channel
            .store(channel.map_or(NO_CHANNEL, ChannelId::to_raw), Ordering::Relaxed);
    }

    pub fn get_channel(&self) -> Option<ChannelId> {
        match self.output_channel.load(Ordering::Relaxed) {
            NO_CHANNEL => None,
            raw => Some(ChannelId::from_raw(raw)),
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
