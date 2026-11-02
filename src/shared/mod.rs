//! The UI's side of the studio: handles to everything running on the audio
//! thread, and the methods that change it while audio runs. Each kind of
//! thing (channels, tracks, instruments, inputs) has its own file with its
//! handle and its part of `impl AppParams`.

use std::sync::Arc;

use rtrb::{Consumer, Producer};

use crate::audio::{
    mixer::MixerParams,
    studio::studio::{Garbage, StudioCommand},
    transport::{TransportParams, metronome::MetronomeParams},
};

mod channels;
mod inputs;
mod instruments;
mod tracks;

pub use channels::ChannelHandle;
pub use inputs::{InputDevice, InputHandle, input_devices};
pub use instruments::{InstrumentHandle, InstrumentKind};
pub use tracks::Track;

pub struct AppParams {
    /// Sample rate of the audio device, needed to build instruments.
    pub sample_rate: f32,
    pub mixer: Arc<MixerParams>,
    pub transport: Arc<TransportParams>,
    pub metronome: Arc<MetronomeParams>,
    /// The mixer channels, in mixer order.
    pub channels: Vec<ChannelHandle>,
    /// The tracks, top to bottom. Each owns one instrument.
    pub tracks: Vec<Track>,
    pub inputs: Vec<InputHandle>,
    /// Sends changes to the studio on the audio thread.
    pub commands: Producer<StudioCommand>,
    /// What the studio sends back to be dropped on this thread.
    pub garbage: Consumer<Garbage>,
}

impl AppParams {
    /// Drops what the studio is done with. Call once per frame.
    pub fn collect_garbage(&mut self) {
        while self.garbage.pop().is_ok() {}
    }
}

/// `base`, or `base 2`, `base 3`, ... when `taken` says it's in use.
fn unique_name(base: &str, taken: impl Fn(&str) -> bool) -> String {
    if !taken(base) {
        return base.to_owned();
    }

    (2..)
        .map(|n| format!("{base} {n}"))
        .find(|name| !taken(name))
        .expect("some number is free")
}
