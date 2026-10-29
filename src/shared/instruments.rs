use std::{error::Error, path::Path, sync::Arc};

use rtrb::Producer;

use crate::{
    audio::{
        instrument::Instrument,
        sampler::{Sampler, VOICES as SAMPLER_VOICES, sample::Sample},
        studio::{
            instrument::{InstrumentParams, InstrumentSlot},
            studio::{MAX_INSTRUMENTS, StudioCommand},
        },
        synth::{Synth, SynthParams},
    },
    events::NoteEvent,
    shared::{AppParams, unique_name},
};

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

impl AppParams {
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

    fn unique_instrument_name(&self, base: &str) -> String {
        unique_name(base, |name| self.instruments.iter().any(|i| i.name == name))
    }
}
