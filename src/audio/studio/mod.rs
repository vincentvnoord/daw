pub mod input;
pub mod instrument;

use std::sync::Arc;

use rtrb::{Consumer, Producer};

use crate::audio::{
    frame::Frame,
    mixer::{
        Mixer, MixerParams,
        channel::{Channel, ChannelId},
    },
    source::{AudioSource, StereoSource},
    studio::{input::InputSlot, instrument::InstrumentSlot},
    transport::{
        Transport, TransportParams,
        metronome::{Metronome, MetronomeParams},
    },
};

/// Most instruments and inputs the studio can hold. Space for all of them is
/// reserved up front, so adding one while audio runs never allocates on the
/// audio thread.
pub const MAX_INSTRUMENTS: usize = 64;
pub const MAX_INPUTS: usize = 32;

/// Changes the UI asks the studio to make while the audio is running.
pub enum StudioCommand {
    AddChannel(Channel),
    RemoveChannel(ChannelId),
    AddInstrument(InstrumentSlot),
    AddInput(InputSlot),
}

/// Things the studio no longer uses. They go back to the UI thread to be
/// dropped there, because freeing memory on the audio thread can glitch.
#[allow(dead_code)] // only held so it gets dropped on the UI thread
pub enum Garbage {
    Channel(Channel),
    Instrument(InstrumentSlot),
    Input(InputSlot),
}

pub struct Studio {
    instruments: Vec<InstrumentSlot>,
    inputs: Vec<InputSlot>,
    mixer: Mixer,
    commands: Consumer<StudioCommand>,
    garbage: Producer<Garbage>,
    transport: Transport,
    metronome: Metronome,
}

impl Studio {
    pub fn new(
        mixer_params: Arc<MixerParams>,
        transport_params: Arc<TransportParams>,
        metronome_params: Arc<MetronomeParams>,
        commands: Consumer<StudioCommand>,
        garbage: Producer<Garbage>,
        sample_rate: f32,
    ) -> Self {
        Self {
            instruments: Vec::with_capacity(MAX_INSTRUMENTS),
            inputs: Vec::with_capacity(MAX_INPUTS),
            mixer: Mixer::new(mixer_params),
            commands,
            garbage,
            transport: Transport::new(transport_params, sample_rate as f64),
            metronome: Metronome::new(metronome_params, sample_rate),
        }
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
                StudioCommand::AddInstrument(slot) => {
                    if self.instruments.len() < MAX_INSTRUMENTS {
                        self.instruments.push(slot);
                    } else {
                        self.discard(Garbage::Instrument(slot));
                    }
                }
                StudioCommand::AddInput(slot) => {
                    if self.inputs.len() < MAX_INPUTS {
                        self.inputs.push(slot);
                    } else {
                        self.discard(Garbage::Input(slot));
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

impl StereoSource for Studio {
    fn begin_buffer(&mut self) {
        self.handle_commands();

        for slot in &mut self.instruments {
            slot.output = self.mixer.resolve(&slot.params.route);
            slot.source.begin_buffer();
        }

        for slot in &mut self.inputs {
            slot.output = self.mixer.resolve(&slot.params.route);
            slot.begin_buffer();
        }

        self.mixer.begin_buffer();
        self.transport.begin_buffer();
        self.metronome.begin_buffer();
    }

    fn next_frame(&mut self) -> Frame {
        for slot in &mut self.instruments {
            let sample = slot.source.next_sample();
            if let Some(ch) = slot.output {
                self.mixer.add_input(ch, sample);
            }
        }

        for slot in &mut self.inputs {
            let sample = slot.next_sample();
            if let Some(ch) = slot.output {
                self.mixer.add_input(ch, sample);
            }
        }

        let click = self.metronome.next_sample(&self.transport);
        self.transport.advance(1);

        // the click skips the mixer, so it never ends up in the mix
        self.mixer.next_frame()
            + Frame {
                left: click,
                right: click,
            }
    }
}
