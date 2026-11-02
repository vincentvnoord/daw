use std::{error::Error, path::Path};

use crate::{
    audio::{
        instrument::Instrument,
        sampler::{Sampler, VOICES as SAMPLER_VOICES, sample::Sample},
        studio::{MAX_INSTRUMENTS, StudioCommand, instrument::InstrumentSlot},
        synth::Synth,
    },
    project::{InstrumentHandle, InstrumentKind, Project, unique_name},
};

/// A row on the timeline: one instrument of its own, playing into a mixer
/// channel through the instrument's route. Several tracks can share a channel.
pub struct Track {
    pub name: String,
    pub instrument: InstrumentHandle,
}

impl Track {
    /// The type to show next to the name, unless the name already says it
    /// (a track called "Synth" doesn't need "SYNTH" next to it).
    pub fn kind_label(&self) -> Option<&'static str> {
        let label = self.instrument.kind.label();
        (!self.name.eq_ignore_ascii_case(label)).then_some(label)
    }
}

impl Project {
    /// Adds a track with a synth while the audio is running. Returns its
    /// index in `tracks`, or `None` when the studio is full.
    pub fn add_synth_track(&mut self) -> Option<usize> {
        let synth = Synth::default(self.sample_rate);
        let kind = InstrumentKind::Synth(synth.params());
        let name = self.unique_track_name("Synth");
        self.add_track(name, kind, synth)
    }

    /// Loads a sample and adds a track with a sampler that plays it. Returns
    /// its index in `tracks`, or `None` when the studio is full.
    pub fn add_sample_track(&mut self, path: &Path) -> Result<Option<usize>, Box<dyn Error>> {
        let base = path
            .file_stem()
            .map_or("Sample".into(), |s| s.to_string_lossy().into_owned());
        self.add_named_sample_track(path, &base)
    }

    /// Like `add_sample_track`, with the track named `base` instead of after
    /// the file.
    pub fn add_named_sample_track(
        &mut self,
        path: &Path,
        base: &str,
    ) -> Result<Option<usize>, Box<dyn Error>> {
        // loading and resampling happen here, never on the audio thread
        let sample = Sample::load(path, self.sample_rate)?;
        let name = self.unique_track_name(base);
        let kind = InstrumentKind::Sampler {
            sample: path.display().to_string(),
        };

        Ok(self.add_track(name, kind, Sampler::new(sample.data, SAMPLER_VOICES)))
    }

    /// Sends the track's instrument to the studio, playing into a new channel
    /// named after the track so it makes sound straight away.
    fn add_track(
        &mut self,
        name: String,
        kind: InstrumentKind,
        instrument: impl Instrument,
    ) -> Option<usize> {
        // room for both the channel and the instrument, so we never end up
        // with one without the other
        if self.tracks.len() >= MAX_INSTRUMENTS || self.commands.slots() < 2 {
            return None;
        }

        let channel = self.add_channel(name.clone());
        let (slot, notes, params) = InstrumentSlot::create(instrument, channel);
        if self
            .commands
            .push(StudioCommand::AddInstrument(slot))
            .is_err()
        {
            return None;
        }

        self.tracks.push(Track {
            name,
            instrument: InstrumentHandle {
                kind,
                notes,
                params,
            },
        });
        Some(self.tracks.len() - 1)
    }

    fn unique_track_name(&self, base: &str) -> String {
        unique_name(base, |name| self.tracks.iter().any(|t| t.name == name))
    }
}
