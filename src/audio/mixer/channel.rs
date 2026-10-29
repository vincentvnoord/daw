use std::sync::Arc;

use crate::{
    audio::mixer::meter::Meter,
    helpers::AtomicF32,
};

pub struct ChannelParams {
    pub gain: AtomicF32,
    pub meter: Meter,
}

pub struct Channel {
    input: f32,
    params: Arc<ChannelParams>,
    gain: f32,
    peak: f32,
}

impl Channel {
    pub fn new(params: Arc<ChannelParams>) -> Self {
        let gain = params.gain.get();
        Channel {
            input: 0.0,
            params,
            gain,
            peak: 0.0,
        }
    }

    pub fn add_input(&mut self, sample: f32) {
        self.input += sample;
    }

    pub fn begin_buffer(&mut self) {
        self.params.meter.record(self.peak);
        self.peak = 0.0;

        self.gain = self.params.gain.get();
    }

    pub fn process(&mut self) -> f32 {
        let out = self.input * self.gain;
        self.input = 0.0;
        self.peak = self.peak.max(out.abs());

        out
    }
}
