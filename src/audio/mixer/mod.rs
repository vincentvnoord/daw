use std::sync::Arc;

pub mod channel;
pub mod meter;

use crate::{
    audio::{
        mixer::{
            channel::{Channel, ChannelId},
            meter::Meter,
        },
        source::AudioSource,
    },
    helpers::AtomicF32,
};

/// Most channels the mixer can hold. Space for all of them is reserved up
/// front, so adding a channel while audio runs never allocates on the audio
/// thread.
pub const MAX_CHANNELS: usize = 64;

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
            channels: Vec::with_capacity(MAX_CHANNELS),
            params,
            gain,
            peak: 0.0,
        }
    }

    /// Adds a channel. Gives it back when the mixer is full.
    pub fn add(&mut self, channel: Channel) -> Result<(), Channel> {
        if self.channels.len() >= MAX_CHANNELS {
            return Err(channel);
        }

        self.channels.push(channel);
        Ok(())
    }

    /// Takes a channel out of the mixer. The channels after it move up one
    /// place; nothing is reallocated.
    pub fn remove(&mut self, id: ChannelId) -> Option<Channel> {
        let index = self.index_of(id)?;
        Some(self.channels.remove(index))
    }

    /// Where the channel with this id currently sits.
    pub fn index_of(&self, id: ChannelId) -> Option<usize> {
        self.channels.iter().position(|c| c.id() == id)
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
