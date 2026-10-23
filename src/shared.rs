use std::sync::Arc;

use crate::{
    audio::{mixer::MixerParams, synth::SynthParams},
    helpers::AtomicF32,
};

pub struct AppParams {
    pub mixer: Arc<MixerParams>,
    pub synth: Arc<SynthParams>,
}
