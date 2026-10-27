use std::sync::Arc;

use crate::audio::{
    mixer::{MixerParams, channel::ChannelParams},
    synth::SynthParams,
};

pub struct AppParams {
    pub mixer: Arc<MixerParams>,
    /// Name and shared params of each mixer channel, in mixer order.
    pub channels: Vec<(String, Arc<ChannelParams>)>,
    pub synth: Arc<SynthParams>,
}
