use std::sync::Arc;

pub mod channel;
pub mod meter;

use crate::{
    audio::{
        mixer::{channel::Channel, meter::Meter},
        source::AudioSource,
    },
    helpers::AtomicF32,
};

pub struct MixerParams {
    pub gain: AtomicF32,
    pub meter: Meter,
}

pub struct Mixer {
    channels: Vec<Channel>,
    params: Arc<MixerParams>,
    gain: f32,
    peak: f32,
}

impl Mixer {
    pub fn new(params: Arc<MixerParams>) -> Self {
        let gain = params.gain.get();
        Self {
            channels: Vec::new(),
            params,
            gain,
            peak: 0.0,
        }
    }

    /// Adds a channel and returns its index.
    pub fn add(&mut self, channel: Channel) -> usize {
        self.channels.push(channel);
        self.channels.len() - 1
    }

    pub fn add_input(&mut self, channel_idx: usize, input: f32) {
        if let Some(ch) = self.channels.get_mut(channel_idx) {
            ch.add_input(input);
        }
    }
}

impl AudioSource for Mixer {
    fn begin_buffer(&mut self) {
        self.params.meter.record(self.peak);
        self.peak = 0.0;

        self.gain = self.params.gain.get();
        for channel in &mut self.channels {
            channel.begin_buffer();
        }
    }

    fn next_sample(&mut self) -> f32 {
        let sum: f32 = self.channels.iter_mut().map(|c| c.process()).sum();
        let out = sum * self.gain;
        self.peak = self.peak.max(out.abs());

        out
    }
}
