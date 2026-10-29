use std::sync::Arc;

use rtrb::{Consumer, Producer, RingBuffer};

use crate::{
    audio::{
        instrument::{Instrument, InstrumentSource},
        mixer::{
            Mixer, MixerParams,
            channel::{Channel, ChannelId, ChannelParams},
        },
        source::AudioSource,
        studio::instrument::{InstrumentParams, InstrumentSlot},
    },
    events::NoteEvent,
};

/// How many note events can wait for the audio thread per instrument.
const NOTE_QUEUE_SIZE: usize = 256;

/// Changes the UI asks the studio to make while the audio is running.
pub enum StudioCommand {
    AddChannel(Channel),
    RemoveChannel(ChannelId),
}

/// Things the studio no longer uses. They go back to the UI thread to be
/// dropped there, because freeing memory on the audio thread can glitch.
#[allow(dead_code)] // only held so it gets dropped on the UI thread
pub enum Garbage {
    Channel(Channel),
}

pub struct Studio {
    instruments: Vec<InstrumentSlot>,
    mixer: Mixer,
    commands: Consumer<StudioCommand>,
    garbage: Producer<Garbage>,
}

impl Studio {
    pub fn new(
        mixer_params: Arc<MixerParams>,
        commands: Consumer<StudioCommand>,
        garbage: Producer<Garbage>,
    ) -> Self {
        Self {
            instruments: Vec::new(),
            mixer: Mixer::new(mixer_params),
            commands,
            garbage,
        }
    }

    /// Adds a mixer channel. Returns the params for the UI, which hold its id.
    pub fn add_channel(&mut self, gain: f32) -> Arc<ChannelParams> {
        let (channel, params) = Channel::create(gain);
        if self.mixer.add(channel).is_err() {
            panic!("mixer is full");
        }

        params
    }

    /// Adds an instrument routed to `channel`. Returns the end the keyboard
    /// sends notes into, and the params for the UI.
    pub fn add_instrument(
        &mut self,
        instrument: impl Instrument,
        channel: Option<ChannelId>,
    ) -> (Producer<NoteEvent>, Arc<InstrumentParams>) {
        let (notes, consumer) = RingBuffer::new(NOTE_QUEUE_SIZE);
        let params = Arc::new(InstrumentParams::new(channel));
        let source = InstrumentSource::new(consumer, instrument);
        self.instruments
            .push(InstrumentSlot::new(Arc::clone(&params), Box::new(source)));

        (notes, params)
    }

    fn handle_commands(&mut self) {
        while let Ok(command) = self.commands.pop() {
            match command {
                StudioCommand::AddChannel(channel) => {
                    if let Err(channel) = self.mixer.add(channel) {
                        self.discard(Garbage::Channel(channel));
                    }
                }
                StudioCommand::RemoveChannel(id) => {
                    if let Some(channel) = self.mixer.remove(id) {
                        self.discard(Garbage::Channel(channel));
                    }
                }
            }
        }
    }

    fn discard(&mut self, garbage: Garbage) {
        // if the UI hasn't emptied the queue it gets dropped here after all,
        // which is better than keeping it forever
        let _ = self.garbage.push(garbage);
    }
}

impl AudioSource for Studio {
    fn begin_buffer(&mut self) {
        self.handle_commands();

        for slot in &mut self.instruments {
            slot.output = slot
                .params
                .get_channel()
                .and_then(|id| self.mixer.index_of(id));
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
