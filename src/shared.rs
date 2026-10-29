use std::sync::Arc;

use rtrb::{Consumer, Producer};

use crate::{
    audio::{
        mixer::{
            MAX_CHANNELS, MixerParams,
            channel::{Channel, ChannelId, ChannelParams, DEFAULT_GAIN},
        },
        studio::{
            instrument::InstrumentParams,
            studio::{Garbage, StudioCommand},
        },
        synth::SynthParams,
    },
    events::NoteEvent,
};

pub struct AppParams {
    pub mixer: Arc<MixerParams>,
    /// The mixer channels, in mixer order.
    pub channels: Vec<ChannelHandle>,
    pub instruments: Vec<InstrumentHandle>,
    /// Sends changes to the studio on the audio thread.
    pub commands: Producer<StudioCommand>,
    /// What the studio sends back to be dropped on this thread.
    pub garbage: Consumer<Garbage>,
}

impl AppParams {
    /// Adds a mixer channel while the audio is running. Returns `false` when
    /// the mixer is full or the command queue is.
    pub fn add_channel(&mut self) -> bool {
        if self.channels.len() >= MAX_CHANNELS {
            return false;
        }

        // built here so the audio thread only has to push it into the mixer
        let (channel, params) = Channel::create(DEFAULT_GAIN);
        if self.commands.push(StudioCommand::AddChannel(channel)).is_err() {
            return false;
        }

        let name = format!("Channel {}", self.channels.len() + 1);
        self.channels.push(ChannelHandle { name, params });
        true
    }

    /// Removes a mixer channel while the audio is running. Instruments that
    /// played into it get no output.
    pub fn remove_channel(&mut self, id: ChannelId) {
        if self.commands.push(StudioCommand::RemoveChannel(id)).is_err() {
            return;
        }

        self.channels.retain(|c| c.id() != id);
        for instrument in &self.instruments {
            if instrument.params.get_channel() == Some(id) {
                instrument.params.set_channel(None);
            }
        }
    }

    pub fn channel(&self, id: ChannelId) -> Option<&ChannelHandle> {
        self.channels.iter().find(|c| c.id() == id)
    }

    /// Drops what the studio is done with. Call once per frame.
    pub fn collect_garbage(&mut self) {
        while self.garbage.pop().is_ok() {}
    }
}

/// The UI's side of a mixer channel.
pub struct ChannelHandle {
    pub name: String,
    pub params: Arc<ChannelParams>,
}

impl ChannelHandle {
    pub fn id(&self) -> ChannelId {
        self.params.id
    }
}

/// The UI's side of an instrument in the studio.
pub struct InstrumentHandle {
    pub name: String,
    pub kind: InstrumentKind,
    /// Where the keyboard sends notes when this instrument is active.
    pub notes: Producer<NoteEvent>,
    pub params: Arc<InstrumentParams>,
}

/// What kind of instrument it is, so the UI knows which editor to show.
pub enum InstrumentKind {
    Synth(Arc<SynthParams>),
    Sampler { sample: String },
}
