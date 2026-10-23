use std::f32::consts::PI;

pub struct LowPassFilter {
    smoothing: f32,
    last_output: f32,
    sample_rate: f32,
}

impl LowPassFilter {
    pub fn new(cutoff_hz: f32, sample_rate: f32) -> Self {
        let mut filter = Self {
            smoothing: 0.0,
            last_output: 0.0,
            sample_rate,
        };

        filter.set_cutoff(cutoff_hz);
        filter
    }

    pub fn set_cutoff(&mut self, cutoff_hz: f32) {
        self.smoothing = 1.0 - (-2.0 * PI * cutoff_hz / self.sample_rate).exp();
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let distance = input - self.last_output;
        self.last_output += self.smoothing * distance;
        self.last_output
    }
}
