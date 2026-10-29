use std::{error::Error, path::Path, sync::Arc};

use rtrb::{Consumer, Producer};

use crate::{
    audio::{
        instrument::Instrument,
        mixer::{
            MAX_CHANNELS, MixerParams,
            channel::{Channel, ChannelId, ChannelParams, DEFAULT_GAIN},
        },
        sampler::{Sampler, VOICES as SAMPLER_VOICES, sample::Sample},
        studio::{
            instrument::{InstrumentParams, InstrumentSlot},
            studio::{Garbage, MAX_INSTRUMENTS, StudioCommand},
        },
        synth::{Synth, SynthParams},
    },
    events::NoteEvent,
};

pub struct AppParams {
    /// Sample rate of the audio device, needed to build instruments.
    pub sample_rate: f32,
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
    /// Adds a mixer channel while the audio is running. Returns its id, or
    /// `None` when the mixer is full or the command queue is.
    pub fn add_channel(&mut self, name: String) -> Option<ChannelId> {
        if self.channels.len() >= MAX_CHANNELS {
            return None;
        }

        // built here so the audio thread only has to push it into the mixer
        let (channel, params) = Channel::create(DEFAULT_GAIN);
        if self.commands.push(StudioCommand::AddChannel(channel)).is_err() {
            return None;
        }

        let id = params.id;
        self.channels.push(ChannelHandle { name, params });
        Some(id)
    }

    /// A name for a new channel that nobody named.
    pub fn next_channel_name(&self) -> String {
        format!("Channel {}", self.channels.len() + 1)
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

    /// Adds a synth while the audio is running. Returns its index in
    /// `instruments`, or `None` when the studio is full.
    pub fn add_synth(&mut self) -> Option<usize> {
        let synth = Synth::default(self.sample_rate);
        let kind = InstrumentKind::Synth(synth.params());
        let name = self.unique_instrument_name("Synth");
        self.add_instrument(name, kind, synth)
    }

    /// Loads a sample and adds a sampler that plays it. Returns its index in
    /// `instruments`, or `None` when the studio is full.
    pub fn add_sample(&mut self, path: &Path) -> Result<Option<usize>, Box<dyn Error>> {
        // loading and resampling happen here, never on the audio thread
        let sample = Sample::load(path, self.sample_rate)?;
        let base = path
            .file_stem()
            .map_or("Sample".into(), |s| s.to_string_lossy().into_owned());
        let name = self.unique_instrument_name(&base);
        let kind = InstrumentKind::Sampler {
            sample: path.display().to_string(),
        };

        Ok(self.add_instrument(name, kind, Sampler::new(sample.data, SAMPLER_VOICES)))
    }

    /// Sends a new instrument to the studio, playing into a new channel named
    /// after it so it makes sound straight away.
    fn add_instrument(
        &mut self,
        name: String,
        kind: InstrumentKind,
        instrument: impl Instrument,
    ) -> Option<usize> {
        // room for both the channel and the instrument, so we never end up
        // with one without the other
        if self.instruments.len() >= MAX_INSTRUMENTS || self.commands.slots() < 2 {
            return None;
        }

        let channel = self.add_channel(name.clone());
        let (slot, notes, params) = InstrumentSlot::create(instrument, channel);
        if self.commands.push(StudioCommand::AddInstrument(slot)).is_err() {
            return None;
        }

        self.instruments.push(InstrumentHandle {
            name,
            kind,
            notes,
            params,
        });
        Some(self.instruments.len() - 1)
    }

    /// `base`, or `base 2`, `base 3`, ... when an instrument already has it.
    fn unique_instrument_name(&self, base: &str) -> String {
        let taken = |name: &str| self.instruments.iter().any(|i| i.name == name);
        if !taken(base) {
            return base.to_owned();
        }

        (2..)
            .map(|n| format!("{base} {n}"))
            .find(|name| !taken(name))
            .expect("some number is free")
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

impl InstrumentHandle {
    /// The type to show next to the name, unless the name already says it
    /// (an instrument called "Synth" doesn't need "SYNTH" next to it).
    pub fn kind_label(&self) -> Option<&'static str> {
        let label = self.kind.label();
        (!self.name.eq_ignore_ascii_case(label)).then_some(label)
    }
}

/// What kind of instrument it is, so the UI knows which editor to show.
pub enum InstrumentKind {
    Synth(Arc<SynthParams>),
    Sampler { sample: String },
}

impl InstrumentKind {
    pub fn label(&self) -> &'static str {
        match self {
            InstrumentKind::Synth(_) => "Synth",
            InstrumentKind::Sampler { .. } => "Sampler",
        }
    }
}
