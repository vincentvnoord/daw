use std::sync::Arc;

use rtrb::{Producer, RingBuffer};

use crate::{
    audio::{
        instrument::{Instrument, InstrumentSource},
        mixer::{
            Mixer, MixerParams,
            channel::{Channel, ChannelParams},
            meter::Meter,
        },
        source::AudioSource,
        studio::instrument::{InstrumentParams, InstrumentSlot},
    },
    events::NoteEvent,
    helpers::AtomicF32,
};

/// How many note events can wait for the audio thread per instrument.
const NOTE_QUEUE_SIZE: usize = 256;

pub struct Studio {
    instruments: Vec<InstrumentSlot>,
    mixer: Mixer,
}

impl Studio {
    pub fn new(mixer_params: Arc<MixerParams>) -> Self {
        Self {
            instruments: Vec::new(),
            mixer: Mixer::new(mixer_params),
        }
    }

    /// Adds a mixer channel. Returns its index and the params for the UI.
    pub fn add_channel(&mut self, gain: f32) -> (usize, Arc<ChannelParams>) {
        let params = Arc::new(ChannelParams {
            gain: AtomicF32::new(gain),
            meter: Meter::new(),
        });
        let index = self.mixer.add(Channel::new(Arc::clone(&params)));

        (index, params)
    }

    /// Adds an instrument routed to `channel`. Returns the end the keyboard
    /// sends notes into, and the params for the UI.
    pub fn add_instrument(
        &mut self,
        instrument: impl Instrument,
        channel: Option<usize>,
    ) -> (Producer<NoteEvent>, Arc<InstrumentParams>) {
        let (notes, consumer) = RingBuffer::new(NOTE_QUEUE_SIZE);
        let params = Arc::new(InstrumentParams::new(channel));
        let source = InstrumentSource::new(consumer, instrument);
        self.instruments
            .push(InstrumentSlot::new(Arc::clone(&params), Box::new(source)));

        (notes, params)
    }
}

impl AudioSource for Studio {
    fn begin_buffer(&mut self) {
        for slot in &mut self.instruments {
            slot.output = slot.params.get_channel();
            slot.source.begin_buffer();
        }

        self.mixer.begin_buffer();
    }

    fn next_sample(&mut self) -> f32 {
        for slot in &mut self.instruments {
            let sample = slot.source.next_sample();
            if let Some(ch) = slot.output {
                self.mixer.add_input(ch, sample);
            }
        }

        self.mixer.next_sample()
    }
}
