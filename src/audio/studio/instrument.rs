use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crate::audio::source::AudioSource;

const NO_CHANNEL: usize = usize::MAX;

pub struct InstrumentParams {
    output_channel: AtomicUsize,
}

impl InstrumentParams {
    pub fn new(channel: Option<usize>) -> Self {
        Self {
            output_channel: AtomicUsize::new(channel.unwrap_or(NO_CHANNEL)),
        }
    }

    pub fn set_channel(&self, channel: Option<usize>) {
        self.output_channel
            .store(channel.unwrap_or(NO_CHANNEL), Ordering::Relaxed);
    }

    pub fn get_channel(&self) -> Option<usize> {
        match self.output_channel.load(Ordering::Relaxed) {
            NO_CHANNEL => None,
            c => Some(c),
        }
    }
}

pub struct InstrumentSlot {
    pub source: Box<dyn AudioSource>,
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
