use std::f32::consts::TAU;

pub struct SineOsc {
    frequency: f32,
    sample_rate: f32,
    phase: f32,
}

impl SineOsc {
    pub fn new(frequency: f32, sample_rate: f32) -> SineOsc {
        SineOsc {
            frequency,
            sample_rate,
            phase: 0.0,
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        let value = (self.phase * TAU).sin();

        self.phase += self.frequency / self.sample_rate;

        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        value
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        self.frequency = frequency;
    }
}
