use crate::audio::source::AudioSource;

pub struct Mixer {
    inputs: Vec<Box<dyn AudioSource>>,
    gain: f32,
}

impl Mixer {
    pub fn new(gain: f32) -> Self {
        Self {
            inputs: Vec::new(),
            gain,
        }
    }

    pub fn add(&mut self, source: impl AudioSource) {
        self.inputs.push(Box::new(source));
    }
}

impl AudioSource for Mixer {
    fn next_sample(&mut self) -> f32 {
        let sum: f32 = self.inputs.iter_mut().map(|s| s.next_sample()).sum();
        sum * self.gain
    }
}
