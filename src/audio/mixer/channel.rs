use std::sync::Arc;

use crate::{
    audio::{mixer::meter::Meter, source::AudioSource},
    helpers::AtomicF32,
};

pub struct ChannelParams {
    pub gain: AtomicF32,
    pub meter: Meter,
}

pub struct Channel {
    sources: Vec<Box<dyn AudioSource>>,
    params: Arc<ChannelParams>,
    gain: f32,
    peak: f32,
}

impl Channel {
    pub fn new(params: Arc<ChannelParams>) -> Self {
        let gain = params.gain.get();
        Channel {
            sources: Vec::new(),
            params,
            gain,
            peak: 0.0,
        }
    }

    pub fn add(&mut self, source: impl AudioSource) {
        self.sources.push(Box::new(source));
    }
}

impl AudioSource for Channel {
    fn begin_buffer(&mut self) {
        self.params.meter.record(self.peak);
        self.peak = 0.0;

        self.gain = self.params.gain.get();

        for source in &mut self.sources {
            source.begin_buffer();
        }
    }

    fn next_sample(&mut self) -> f32 {
        let sum: f32 = self.sources.iter_mut().map(|s| s.next_sample()).sum();
        let out = sum * self.gain;
        self.peak = self.peak.max(out.abs());

        out
    }
}
