use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use crate::audio::{mixer::channel::ChannelId, source::AudioSource};

const NO_CHANNEL: u32 = u32::MAX;

pub struct InstrumentParams {
    output_channel: AtomicU32,
}

impl InstrumentParams {
    pub fn new(channel: Option<ChannelId>) -> Self {
        Self {
            output_channel: AtomicU32::new(channel.map_or(NO_CHANNEL, ChannelId::to_raw)),
        }
    }

    pub fn set_channel(&self, channel: Option<ChannelId>) {
        self.output_channel
            .store(channel.map_or(NO_CHANNEL, ChannelId::to_raw), Ordering::Relaxed);
    }

    pub fn get_channel(&self) -> Option<ChannelId> {
        match self.output_channel.load(Ordering::Relaxed) {
            NO_CHANNEL => None,
            raw => Some(ChannelId::from_raw(raw)),
        }
    }
}

pub struct InstrumentSlot {
    pub source: Box<dyn AudioSource>,
    /// Index in the mixer of the channel this instrument plays into, looked
    /// up from the params once per buffer.
    pub output: Option<usize>,
    pub params: Arc<InstrumentParams>,
}

impl InstrumentSlot {
    pub fn new(params: Arc<InstrumentParams>, source: Box<dyn AudioSource>) -> Self {
        Self {
            source,
            output: None,
            params,
        }
    }
}
