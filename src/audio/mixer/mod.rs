use std::sync::Arc;

use crate::{audio::source::AudioSource, helpers::AtomicF32};

pub struct MixerParams {
    pub gain: AtomicF32,
}

pub struct Mixer {
    inputs: Vec<Box<dyn AudioSource>>,
    params: Arc<MixerParams>,
    gain: f32,
}

impl Mixer {
    pub fn new(params: Arc<MixerParams>) -> Self {
        let gain = params.gain.get();
        Self {
            inputs: Vec::new(),
            params,
            gain,
        }
    }

    pub fn add(&mut self, source: impl AudioSource) {
        self.inputs.push(Box::new(source));
    }
}

impl AudioSource for Mixer {
    fn begin_buffer(&mut self) {
        self.gain = self.params.gain.get();
        for input in &mut self.inputs {
            input.begin_buffer();
        }
    }

    fn next_sample(&mut self) -> f32 {
        let sum: f32 = self.inputs.iter_mut().map(|s| s.next_sample()).sum();
        sum * self.gain
    }
}
