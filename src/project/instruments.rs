use std::sync::Arc;

use rtrb::Producer;

use crate::audio::{note::NoteEvent, studio::instrument::InstrumentParams, synth::SynthParams};

/// The UI's side of an instrument in the studio. Every instrument belongs to
/// exactly one track.
pub struct InstrumentHandle {
    /// Index of the instrument's slot in the studio, which plays its notes.
    pub slot: usize,
    pub kind: InstrumentKind,
    /// Where the keyboard sends notes when this instrument's track is selected.
    pub notes: Producer<NoteEvent>,
    pub params: Arc<InstrumentParams>,
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
