use std::sync::Arc;

pub mod channel;
pub mod meter;

use crate::{
    audio::{mixer::channel::Channel, source::AudioSource},
    helpers::AtomicF32,
};

pub struct MixerParams {
    pub gain: AtomicF32,
}

pub struct Mixer {
    channels: Vec<Channel>,
    params: Arc<MixerParams>,
    gain: f32,
}

impl Mixer {
    pub fn new(params: Arc<MixerParams>) -> Self {
        let gain = params.gain.get();
        Self {
            channels: Vec::new(),
            params,
            gain,
        }
    }

    pub fn add(&mut self, channel: Channel) {
        self.channels.push(channel);
    }
}

impl AudioSource for Mixer {
    fn begin_buffer(&mut self) {
        self.gain = self.params.gain.get();
        for channel in &mut self.channels {
            channel.begin_buffer();
        }
    }

    fn next_sample(&mut self) -> f32 {
        let sum: f32 = self.channels.iter_mut().map(|s| s.next_sample()).sum();
        sum * self.gain
    }
}
