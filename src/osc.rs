use std::f32::consts::TAU;

pub enum WaveForm {
    Sine,
    Saw,
    Square,
}

pub struct Oscillator {
    waveform: WaveForm,
    frequency: f32,
    sample_rate: f32,
    phase: f32,
}

impl Oscillator {
    pub fn new(waveform: WaveForm, frequency: f32, sample_rate: f32) -> Oscillator {
        Oscillator {
            waveform,
            frequency,
            sample_rate,
            phase: 0.0,
        }
    }

    pub fn next_sample(&mut self) -> f32 {
        let value = match self.waveform {
            WaveForm::Sine => (self.phase * TAU).sin(),
            WaveForm::Saw => 2.0 * self.phase - 1.0,
            WaveForm::Square => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
        };

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
